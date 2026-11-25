//! The arcs' decision beats, presented (DEC-109, STAGED_ARCS §3). A run
//! asks for a decision by listing its beat in `ArcRun::open`; the
//! director decides **when** it is put to the player, within the
//! chapter's budgets, and nothing else: the dilemma is the beat's, its
//! cast is the run's (a role the arc lacks, such as a bill, is cast as
//! any decision's is), and the run reads the answer from
//! `Decision::arc`.
//!
//! - The **turn** of the chapter's lead arc is the chapter's climax: its
//!   decision opens the day the stage is entered, with weight `climax`,
//!   and the pacing cycle peaks there.
//! - A decision of the lead arc, or one under a clock, opens in any phase
//!   and spends the significant budget (the minor budget when that is
//!   spent).
//! - A background arc's decision without a clock is a minor beat.
//! - A decision waits while the player has `max_pending` waiting, or no
//!   budget is left; its run goes on without it (the world does not
//!   wait).
//!
//! With the spotlight off nothing is presented, and every run takes the
//! path it takes when the player lets everything pass.

use super::cast::View;
use super::director::{self, Budget};
use super::{ChapterKind, NarrativeState, Reason, Weight};
use crate::arcs::{ArcRef, ArcRun};
use crate::scheduler::{Outs, Reads};
use mandate_content::arcs::{BeatKind, BeatWeight};

/// Narrative domain, Director phase, daily.
pub fn present(n: &mut NarrativeState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let Some(arcs) = r.arcs() else { return };
    if !n.spotlight || n.chapter.is_none() || !r.player.is_some_and(|p| p.status.active()) {
        return;
    }
    // The beats asked for and not yet presented.
    let asked: Vec<(&ArcRun, ArcRef)> = arcs
        .runs
        .iter()
        .flat_map(|run| {
            run.open.iter().map(move |b| {
                (
                    run,
                    ArcRef {
                        run: run.id,
                        beat: b.clone(),
                    },
                )
            })
        })
        .filter(|(_, a)| !n.decisions.iter().any(|d| d.arc.as_ref() == Some(a)))
        .collect();
    if asked.is_empty() {
        return;
    }
    let content = r.content;
    let d = &content.director;
    let pol = r.politics();
    let mut v = View::new(r, pol);
    v.recent = director::recent(n);
    v.answers = Some(n.answers.clone());
    let b = director::budget(d, n.profile(content));
    // A run raised today may be the lead (and its turn the climax).
    director::lead(n, &v, out, b);
    for (run, aref) in asked {
        if n.pending().count() >= d.max_pending as usize {
            break;
        }
        let Some(def) = content.arc(&run.template) else {
            continue;
        };
        let Some((stage, beat)) = crate::arcs::beat_of(def, &aref.beat) else {
            continue;
        };
        let BeatKind::Decision {
            dilemma, weight, ..
        } = &beat.kind
        else {
            continue;
        };
        let Some(ddef) = content.dilemma(dilemma) else {
            continue;
        };
        let Some(weight) = weigh(n, r, b, run, stage.climax, *weight) else {
            continue;
        };
        // The run's cast fills the roles the dilemma shares; the rest are
        // cast, and its triggers checked, as for any decision.
        let Some(mut c) = director::evaluate(&v, n, ddef, &run.narrative_cast(), None, &[]) else {
            continue;
        };
        c.reasons.push(Reason::Arc {
            arc: def.id.clone(),
        });
        c.relevance += d.relevance.arc;
        let id = director::open(n, &v, out, c, weight, Some(aref), None);
        if weight == Weight::Climax {
            director::turn_peak(n, &v, id);
        }
    }
}

/// The weight a decision beat opens with now, or `None` while it waits
/// for room in the chapter's budgets.
fn weigh(
    n: &NarrativeState,
    r: &Reads<'_>,
    b: Budget,
    run: &ArcRun,
    turn: bool,
    asked: BeatWeight,
) -> Option<Weight> {
    let ch = n.chapter.as_ref()?;
    let d = &r.content.director;
    let lead = ch.lead == Some(run.id.0);
    let clocked = run.clock.is_some_and(|c| c >= r.date);
    // The chapter's climax: the lead's turn, counted by `director::lead`.
    let climax = turn
        && lead
        && ch.kind == ChapterKind::Session
        && ch.climax.is_none()
        && run.turn.as_ref().is_some_and(|t| {
            ch.turn.as_deref() == Some(&director::turn_key(&run.template, &t.stage))
        });
    if climax && asked == BeatWeight::Climax {
        return Some(Weight::Climax);
    }
    let minor = || director::minor_room(n, d, b, r.date, lead || clocked).then_some(Weight::Minor);
    if asked == BeatWeight::Minor || !(lead || clocked) {
        return minor();
    }
    if ch.significant < b.sig_max {
        Some(Weight::Significant)
    } else {
        minor()
    }
}
