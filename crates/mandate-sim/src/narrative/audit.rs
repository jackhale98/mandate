//! The grounding audit (REQ-003, SIM-005): every item the director emits
//! references live actors and true conditions. Used by tests on the day an
//! item is emitted; read-only.

use super::cast::{self, View};
use super::{Decision, FactKind, NarrativeState, StorylineRun};
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
    // Roles a storyline cast are checked against the storyline's own casts.
    let run: Option<&StorylineRun> = d
        .storyline
        .and_then(|id| state.narrative.storylines.iter().find(|r| r.id == id));
    let sdef = run.and_then(|r| content.storyline(&r.template));
    let holder = d.goal.as_ref().map(|g| g.0);
    for e in &d.cast {
        let role: Option<&RoleDef> = sdef
            .and_then(|s| s.roles.iter().find(|x| x.id == e.role))
            .or_else(|| def.roles.iter().find(|x| x.id == e.role));
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
    if let (Some(r), Some(s)) = (run, sdef)
        && let Some(stage) = s.stages.get(r.stage as usize)
    {
        for c in &stage.advance {
            if !cast::holds(&v, c, &r.cast) {
                out.push(format!("{}: stage condition {:?} does not hold", s.id, c));
            }
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

/// Problems with a spotlit fact's grounding now (developments and
/// storyline beats).
pub fn fact(state: &State, content: &ContentBundle, f: &super::Fact) -> Vec<String> {
    let n: &NarrativeState = &state.narrative;
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
        FactKind::StorylineStage { run, .. } => {
            let Some(r) = n.storylines.iter().find(|x| x.id == *run) else {
                return bad("no such storyline run".into());
            };
            if !cast::actors_exist(&v, &r.cast) {
                return bad(format!("{}: a cast actor does not exist", r.template));
            }
            Vec::new()
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
