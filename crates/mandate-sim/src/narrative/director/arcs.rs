//! The director on arcs (DEC-109, STAGED_ARCS §3). DEC-033 B holds: it
//! times, and never decides whether. Here are its powers over arcs, each
//! logged as a `DirectorIntervention`:
//!
//! 1. which running arc **leads** the chapter ([`lead`]): its turn (the
//!    first stage marked `climax` it reaches) is the chapter's climax;
//! 2. where a **director-owned clock** falls inside its content range
//!    ([`place_clock`]): in the chapter's climax window when the two
//!    overlap. A date the simulation owns is never moved;
//! 3. **when a decision beat is presented** (`narrative::offered`), within
//!    the chapter's budgets.
//!
//! It never raises a run, blocks or forces an edge, picks an exit or
//! changes an effect: arcs run the same with the spotlight off. (The
//! arrival of an opportunity arc waits for the first such arc.)

use super::*;
use crate::arcs::{ArcId, ArcRun, ArcsState};
use crate::spine::{Cause, Process};
use mandate_content::arcs::{ArcDef, Standing};

/// Draw salt of the lead's pick (`KEY_PICK`).
const SALT_LEAD: u64 = 7;

/// A session's climax window as dates, [first day, last day]: where its
/// tension curve wants the turn. `None` in a campaign (its climax is the
/// election).
pub fn climax_window(v: &View<'_>, ch: &Chapter) -> Option<[SimDate; 2]> {
    if ch.kind != ChapterKind::Session {
        return None;
    }
    let total = f64::from(ch.start.days_until(climax_horizon(v, ch)));
    let w = &v.content().director.tension.climax_window;
    Some([
        ch.start.add_days((total * w[0]).ceil() as u32),
        ch.start.add_days((total * w[1]).floor() as u32),
    ])
}

/// Place a director-owned clock (STAGED_ARCS §3.2) between `lo` and `hi`
/// (the content's range, as dates) with the run's own keyed draw `u`. The
/// turn comes `lead` days before the clock. Under a storyteller with a
/// tension curve, in a session: inside the chapter's climax window where
/// it overlaps the range, else at the end of the range nearest the
/// window. Otherwise anywhere in the range. Returns the date, and whether
/// the director placed it.
///
/// It reads the chapter's dates and the profile, which are the same with
/// the spotlight on or off: the spotlight never moves a clock.
pub fn place_clock(v: &View<'_>, lo: SimDate, hi: SimDate, lead: u32, u: f64) -> (SimDate, bool) {
    let hi = hi.max(lo);
    let within = |a: SimDate, b: SimDate| {
        let span = a.days_until(b);
        a.add_days(((u.clamp(0.0, 1.0) * f64::from(span + 1)).floor() as u32).min(span))
    };
    let n = v.r.narrative();
    let shaped = n.profile(v.content()).is_some_and(|p| p.tension_curve);
    let window = n
        .chapter
        .as_ref()
        .filter(|_| shaped)
        .and_then(|ch| climax_window(v, ch));
    let Some([a, b]) = window else {
        return (within(lo, hi), false);
    };
    let (a, b) = (a.add_days(lead), b.add_days(lead));
    let (x, y) = (lo.max(a), hi.min(b));
    if x <= y {
        (within(x, y), true)
    } else if hi < a {
        (hi, true)
    } else {
        (lo, true)
    }
}

/// The theme of a material arc: one about something physical in the
/// town (a flood, a plant), which outranks a standing arc for the lead.
pub const THEME_MATERIAL: &str = "crisis";

/// How a run ranks for the lead: a material arc, then the standing arc of
/// this kind of chapter, then the rest (a debt being called in, the
/// other kind's standing arc).
fn lead_tier(def: &ArcDef, kind: ChapterKind) -> Tier {
    match (def.standing, kind) {
        (None, _) if def.themes.iter().any(|t| t == THEME_MATERIAL) => Tier::A,
        (Some(Standing::Session), ChapterKind::Session)
        | (Some(Standing::Campaign), ChapterKind::Campaign) => Tier::B,
        _ => Tier::C,
    }
}

/// A run can give this chapter its turn: it turned inside it, or it has
/// not turned and a turn is still ahead of its stage. A run in a hidden
/// stage (signs nobody has been told of: a plant's short weeks) leads
/// nothing until it breaks.
fn can_lead(run: &ArcRun, def: &ArcDef, ch: &Chapter) -> bool {
    if def.stage(&run.stage).is_some_and(|s| s.hidden) {
        return false;
    }
    match &run.turn {
        Some(t) => t.on >= ch.start,
        None => crate::arcs::turn_ahead(def, &run.stage),
    }
}

/// The turn a run (live or ended) reached, with its template.
fn turn_of(arcs: &ArcsState, id: u32) -> Option<(String, crate::arcs::ArcTurn)> {
    let id = ArcId(id);
    match arcs.run(id) {
        Some(r) => r.turn.clone().map(|t| (r.template.clone(), t)),
        None => arcs
            .past
            .iter()
            .find(|p| p.id == id)
            .and_then(|p| p.turn.clone().map(|t| (p.template.clone(), t))),
    }
}

/// The climax key of a turn: `<arc template>/<stage>` (climax memory
/// counts the variant).
pub fn turn_key(template: &str, stage: &str) -> String {
    format!("{template}/{stage}")
}

/// The lead arc (STAGED_ARCS §3.1), and the chapter's climax.
///
/// The lead is chosen among the runs in the focus jurisdictions that can
/// still give the chapter a turn, by the existing pick (tier, then
/// relevance × novelty, one keyed draw): a material arc outranks a
/// standing one. It is kept while it can still turn and nothing of a
/// better tier has been raised. When the lead reaches its turn inside a
/// session chapter, that turn is the chapter's climax (a campaign's is
/// the election).
pub(crate) fn lead(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>, b: Budget) {
    let Some(arcs) = v.r.arcs() else { return };
    let Some(ch) = n.chapter.clone() else { return };
    let content = v.content();
    let d = &content.director;
    let date = v.date();
    let session = ch.kind == ChapterKind::Session;
    let counted = !session || ch.climaxes >= b.climax;
    // The lead's turn, reached inside the chapter, is its climax.
    if !counted
        && let Some((template, t)) = ch.lead.and_then(|id| turn_of(arcs, id))
        && t.on >= ch.start
    {
        note_turn(n, &template, &t.stage);
        return;
    }
    if counted && ch.lead.is_some() {
        return;
    }
    let focus = v.player().map(|p| p.focus()).unwrap_or_default();
    let mut cands: Vec<(&ArcRun, Tier, f64)> = arcs
        .runs
        .iter()
        .filter(|r| focus.contains(&r.at))
        .filter_map(|r| {
            let def = content.arc(&r.template)?;
            can_lead(r, def, &ch).then(|| {
                let relevance = d.relevance.base + d.relevance.crisis * f64::from(r.heat);
                (r, lead_tier(def, ch.kind), relevance)
            })
        })
        .collect();
    let Some(best) = cands.iter().map(|c| c.1).min() else {
        return;
    };
    // Kept while it can still turn and nothing outranks it.
    if ch
        .lead
        .is_some_and(|id| cands.iter().any(|c| c.0.id.0 == id && c.1 == best))
    {
        return;
    }
    cands.sort_by_key(|c| c.0.id);
    let items: Vec<Pickable> = cands
        .iter()
        .map(|c| Pickable {
            tier: c.1,
            relevance: c.2,
            opportunity: false,
            novelty: novelty(n, &c.0.template),
        })
        .collect();
    let u = mandate_core::rng::keyed(
        v.r.meta.seed,
        Stream::Director,
        &[
            super::super::KEY_PICK,
            u64::from(date.0),
            u64::from(ch.index),
            SALT_LEAD,
        ],
    )
    .next_f64();
    let Some(i) = pick_index(&items, d.pick_band, false, u) else {
        return;
    };
    let run = cands[i].0;
    if let Some(c) = n.chapter.as_mut() {
        c.lead = Some(run.id.0);
    }
    let uses = n.uses.entry(run.template.clone()).or_insert(0);
    *uses = uses.saturating_add(1);
    out.events.record(
        date,
        EventKind::DirectorIntervention {
            lever: "lead_arc".into(),
            municipality: run.at.municipality().map(|m| m.0).or(v.town()).unwrap_or(0),
        },
        Cause::Process(Process::Director),
    );
    // A lead that had already turned inside the chapter gives it its climax.
    if !counted
        && let Some(t) = &run.turn
        && t.on >= ch.start
    {
        note_turn(n, &run.template, &t.stage);
    }
}

/// The chapter's climax is this turn.
fn note_turn(n: &mut NarrativeState, template: &str, stage: &str) {
    let key = turn_key(template, stage);
    let Some(ch) = n.chapter.as_mut() else { return };
    ch.climaxes = ch.climaxes.saturating_add(1);
    ch.turn = Some(key.clone());
    let index = ch.index;
    remember(n, index, key);
}

fn remember(n: &mut NarrativeState, chapter: u32, key: String) {
    n.climaxes.push((chapter, key));
    if n.climaxes.len() > super::super::CLIMAXES_CAP {
        n.climaxes.remove(0);
    }
}

/// A session chapter closes without its lead having turned: if another
/// run in the focus jurisdictions reached its turn inside the chapter,
/// that turn was the chapter's (the earliest; the lead it should have
/// had). The director's choice of lead is a bet on which matter turns;
/// this is where a lost bet is settled.
pub(super) fn settle_turn(n: &mut NarrativeState, v: &View<'_>, ch: &mut Chapter, b: Budget) {
    if ch.kind != ChapterKind::Session || ch.climaxes >= b.climax {
        return;
    }
    let Some(arcs) = v.r.arcs() else { return };
    let focus = v.player().map(|p| p.focus()).unwrap_or_default();
    let live = arcs
        .runs
        .iter()
        .map(|r| (r.id, &r.template, r.at, r.turn.as_ref()));
    let past = arcs
        .past
        .iter()
        .map(|p| (p.id, &p.template, p.at, p.turn.as_ref()));
    let first = live
        .chain(past)
        .filter(|x| focus.contains(&x.2))
        .filter_map(|(id, template, _, turn)| {
            let t = turn.filter(|t| t.on >= ch.start)?;
            Some((t.on, id, template, &t.stage))
        })
        .min_by_key(|x| (x.0, x.1));
    if let Some((_, id, template, stage)) = first {
        let key = turn_key(template, stage);
        ch.climaxes = ch.climaxes.saturating_add(1);
        ch.lead = Some(id.0);
        ch.turn = Some(key.clone());
        remember(n, ch.index, key);
    }
}

/// The heat of the hottest run in the focus jurisdictions (0 when none
/// runs): what the tension curve reads of arcs.
pub(super) fn heat(v: &View<'_>) -> f64 {
    let Some(arcs) = v.r.arcs() else { return 0.0 };
    let focus = v.player().map(|p| p.focus()).unwrap_or_default();
    arcs.runs
        .iter()
        .filter(|r| focus.contains(&r.at))
        .map(|r| f64::from(r.heat))
        .fold(0.0, f64::max)
}

/// The dilemmas the live runs name as decision beats: the spotlight never
/// presents one of them on its own while its run lives.
pub(super) fn staged<'c>(v: &View<'c>) -> Vec<&'c str> {
    let content = v.content();
    let Some(arcs) = v.r.arcs() else {
        return Vec::new();
    };
    arcs.runs
        .iter()
        .filter_map(|r| content.arc(&r.template))
        .flat_map(|def| def.beats())
        .filter_map(|(_, beat)| match &beat.kind {
            mandate_content::arcs::BeatKind::Decision { dilemma, .. } => Some(dilemma.as_str()),
            _ => None,
        })
        .collect()
}

/// The last day a run entered a stage or ended (a beat for the quiet
/// rescue).
pub(super) fn last_stage(v: &View<'_>) -> Option<SimDate> {
    let arcs = v.r.arcs()?;
    arcs.runs
        .iter()
        .map(|r| r.since)
        .chain(arcs.past.iter().map(|p| p.ended))
        .max()
}
