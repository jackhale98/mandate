//! The grounding audit (REQ-003, SIM-005): every item the director emits
//! references live actors and true conditions. Used by tests on the day an
//! item is emitted; read-only.

use super::cast::{self, View};
use super::{Decision, FactKind};
use crate::State;
use crate::events::EventKind;
use mandate_content::{ContentBundle, RoleDef};

/// Problems with a decision's grounding now (empty = grounded).
pub fn decision(state: &State, content: &ContentBundle, d: &Decision) -> Vec<String> {
    let mut out = Vec::new();
    let Some(def) = content.dilemma(&d.template) else {
        return vec![format!("{}: unknown template", d.template)];
    };
    let r = crate::scheduler::Reads::of_state(state, content);
    let v = View::new(&r, &state.politics);
    if !cast::actors_exist(&v, &d.cast) {
        out.push(format!("{}: a cast actor does not exist", d.template));
    }
    let holder = d.goal.as_ref().map(|g| g.0);
    // Roles an arc cast (DEC-109) are the run's own, bound by the arc's
    // rules: the run is live, the beat is a decision beat of its template
    // for this dilemma, and the roles the arc has are cast as the run's.
    // A role only the dilemma has (a bill) is checked like any other.
    let mut arc_roles: Vec<&str> = Vec::new();
    if let Some(a) = &d.arc {
        match state.arcs.run(a.run) {
            None => out.push(format!("{}: its arc run is not live", d.template)),
            Some(run) => {
                let beat = content
                    .arc(&run.template)
                    .and_then(|t| crate::arcs::beat_of(t, &a.beat))
                    .is_some_and(|(_, b)| {
                        matches!(&b.kind, mandate_content::arcs::BeatKind::Decision { dilemma, .. }
                            if dilemma == &d.template)
                    });
                if !beat {
                    out.push(format!(
                        "{}: not the decision of beat `{}` of its arc",
                        d.template,
                        a.beat.key()
                    ));
                }
                let cast = run.narrative_cast();
                if let Some(t) = content.arc(&run.template) {
                    arc_roles = t.roles.iter().map(|r| r.id.as_str()).collect();
                }
                for e in d
                    .cast
                    .iter()
                    .filter(|e| arc_roles.contains(&e.role.as_str()) && !cast.contains(e))
                {
                    out.push(format!(
                        "{}: role `{}` is not the run's {:?}",
                        d.template, e.role, e.actor
                    ));
                }
            }
        }
    }
    for e in d
        .cast
        .iter()
        .filter(|e| !arc_roles.contains(&e.role.as_str()))
    {
        let role: Option<&RoleDef> = def.roles.iter().find(|x| x.id == e.role);
        let Some(role) = role else {
            continue;
        };
        if !cast::still_cast(&v, role, e.actor, &d.cast, holder) {
            out.push(format!(
                "{}: role `{}` is not filled by {:?} now",
                d.template, e.role, e.actor
            ));
        }
    }
    for c in &def.triggers {
        if !cast::holds(&v, c, &d.cast) {
            out.push(format!("{}: trigger {:?} does not hold", d.template, c));
        }
    }
    if let Some((h, g)) = &d.goal
        && !state
            .politics
            .characters
            .goals
            .iter()
            .any(|x| x.holder == *h && &x.goal == g)
    {
        out.push(format!("{}: the goal `{g}` is not held", d.template));
    }
    out
}

/// Problems with a spotlit fact's grounding now (developments).
pub fn fact(state: &State, content: &ContentBundle, f: &super::Fact) -> Vec<String> {
    let r = crate::scheduler::Reads::of_state(state, content);
    let v = View::new(&r, &state.politics);
    let town = v.town_character();
    let bad = |m: String| vec![m];
    match &f.kind {
        FactKind::CrisisBegan { kind } | FactKind::CrisisWorse { kind } => {
            if town.is_some_and(|t| t.crises.iter().any(|c| &c.kind == kind)) {
                Vec::new()
            } else {
                bad(format!("crisis {kind} is not live"))
            }
        }
        FactKind::GoalTaken { person, goal } => {
            if state
                .politics
                .characters
                .goals
                .iter()
                .any(|g| g.holder == *person && &g.goal == goal)
            {
                Vec::new()
            } else {
                bad(format!("goal {goal} is not held"))
            }
        }
        FactKind::RivalNamed { person } => {
            if town.is_some_and(|t| t.rival == Some(*person)) {
                Vec::new()
            } else {
                bad("the rival is someone else".into())
            }
        }
        FactKind::TownEvent { event } => match state.events.get(*event).map(|e| &e.kind) {
            Some(EventKind::PlantClosure { municipality, .. })
            | Some(EventKind::Disaster { municipality, .. })
            | Some(EventKind::PolicyIntroduced { municipality, .. })
            | Some(EventKind::PolicyEnacted { municipality, .. })
            | Some(EventKind::PolicyFailed { municipality, .. })
            | Some(EventKind::RaceDecided { municipality, .. }) => {
                if Some(*municipality) == v.town() {
                    Vec::new()
                } else {
                    bad("the event is in another town".into())
                }
            }
            Some(EventKind::PollsPublished { .. }) | Some(EventKind::CampaignOpened { .. }) => {
                Vec::new()
            }
            _ => bad("no such event".into()),
        },
        _ => Vec::new(),
    }
}
