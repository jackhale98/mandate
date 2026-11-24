//! Decisions other domains ask for (effect requests, OVERVIEW §4.3). An
//! arc's decision beat (DEC-109, `Effect::OpenDecision`) is a dilemma put
//! to the player with the run's cast: the narrative domain opens it, the
//! player answers it (or lets it pass) like any decision, and the run
//! reads the answer from `Decision::arc`.
//!
//! It is the arc's beat, not the director's spotlight: it counts against
//! no chapter budget, cooldown or pacing phase, and it opens whether the
//! spotlight is on or off (the arc is simulation; only its presentation
//! is the director's, task A2). A request is dropped when it cannot be
//! met (no player to ask, a role missing, the template removed): the run
//! asks again while its stage lasts.

use super::cast::View;
use super::{
    CastEntry, Decision, DecisionId, DecisionStatus, FactKind, NarrativeState, Weight, actor_of,
};
use crate::arcs::ArcRef;
use crate::events::EventKind;
use crate::scheduler::{Effect, Outs, Reads};
use crate::spine::{Cause, Causes, Process};

/// Narrative domain, Director phase, daily.
pub fn requests(n: &mut NarrativeState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let reqs = out.take(|e| matches!(e, Effect::OpenDecision { .. }));
    if reqs.is_empty() {
        return;
    }
    let pol = r.politics();
    let v = View::new(r, pol);
    for q in reqs {
        if let Effect::OpenDecision {
            template,
            cast,
            weight,
            arc,
        } = q.effect
        {
            open(n, &v, out, &template, cast, weight, arc, q.cause);
        }
    }
}

/// Open a decision of `template` with a given cast for an arc's beat.
#[allow(clippy::too_many_arguments)]
fn open(
    n: &mut NarrativeState,
    v: &View<'_>,
    out: &mut Outs<'_>,
    template: &str,
    cast: Vec<CastEntry>,
    weight: Weight,
    arc: ArcRef,
    cause: Cause,
) -> Option<DecisionId> {
    let content = v.content();
    let def = content.dilemma(template)?;
    // Nothing is put to a player who cannot act; one decision a beat.
    if !v.player().is_some_and(|p| p.status.active())
        || n.decisions.iter().any(|d| d.arc.as_ref() == Some(&arc))
    {
        return None;
    }
    // The dilemma's roles are the run's (content validation says so; a
    // run whose cast lost one cannot put it).
    if !def.roles.iter().all(|r| actor_of(&cast, &r.id).is_some())
        || !super::cast::actors_exist(v, &cast)
    {
        return None;
    }
    let date = v.date();
    let id: DecisionId = n.decision_ids.alloc();
    let deadline = date.add_days(def.deadline_days.unwrap_or(content.director.deadline_days));
    let place = def
        .scene
        .as_deref()
        .and_then(|s| content.scene(s))
        .and_then(|s| super::director::scene_place(v, &s.place, &cast));
    // A decision names its cast (DEC-091): the player hears of them.
    for person in super::learn::cast_people(v, &cast) {
        out.effects.push(
            Effect::Learn {
                subject: crate::player::knowledge::Subject::Person { person },
                level: crate::player::knowledge::Level::Heard,
                source: crate::player::knowledge::Source::Decision { decision: id },
            },
            cause,
        );
    }
    let actors = cast.iter().map(|e| e.actor).collect();
    n.decisions.push(Decision {
        id,
        template: def.id.clone(),
        weight,
        cast,
        reasons: Vec::new(),
        opened: date,
        deadline,
        chapter: n.chapter.as_ref().map_or(0, |c| c.index),
        relevance: 1.0,
        storyline: None,
        goal: None,
        place,
        status: DecisionStatus::Pending,
        resolution: None,
        about: None,
        arc: Some(arc),
    });
    n.log(
        date,
        FactKind::DecisionOpened { decision: id },
        actors,
        true,
        0.5,
    );
    // The run's stage prompted it; the arcs' rule put it.
    out.events.record(
        date,
        EventKind::DecisionOpened {
            decision: id.0,
            template: def.id.clone(),
        },
        Causes::from(cause).and(Cause::Process(Process::Arcs)),
    );
    Some(id)
}
