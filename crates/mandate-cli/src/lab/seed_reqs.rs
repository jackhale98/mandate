//! Seed requirements of the lab's own tests (see `mandate-seeds`): the
//! predicates live here because they play games with the lab's runner, which
//! the seeds crate cannot depend on. `mandate-cli seeds` evaluates them with
//! the seeds crate's own.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::runner::{Driver, Game, Intervention, Setup};
use mandate_seeds::requirements::Requirement;

pub const ALL: &[Requirement] = &[
    Requirement {
        name: "explorer_enacts_own_law",
        needs: "a small world where the explorer legislator's own bill is enacted within 72 months",
        holds: explorer_enacts_own_law,
    },
    Requirement {
        name: "flood_arc_exits_idle_by_month_12",
        needs: "a planted flood raises the arc, still running at month 3, and the idle game lets it exit by month 12",
        holds: flood_arc_exits_idle_by_month_12,
    },
];

const SEVERITY: f64 = 0.9;

/// A small idle game with a flood planted in the player's town.
pub fn flooded(seed: u64) -> Game {
    let content = crate::lab::load(&super::runner::content_root(), &[]).unwrap();
    flooded_in(&content, seed)
}

/// [`flooded`] with the given content (a test that wants the flood alone).
pub fn flooded_in(content: &std::sync::Arc<mandate_content::ContentBundle>, seed: u64) -> Game {
    let mut g = Game::new(content, &Setup::small(seed, None), Driver::Idle).unwrap();
    let town = g.home();
    g.intervene(&Intervention::Disaster {
        town,
        hazard: "flood".into(),
        severity: SEVERITY,
    })
    .unwrap();
    g
}

pub fn flood_arc_exits_idle_by_month_12(seed: u64) -> bool {
    // The flood's own run: other arcs run in every game (the budget
    // round, since A2).
    const FLOOD: &str = "arc.flood";
    let mut a = flooded(seed);
    a.run_to(3, &[], &mut Vec::new());
    if !a.e.state().arcs.runs.iter().any(|r| r.template == FLOOD) {
        return false;
    }
    a.run_to(12, &[], &mut Vec::new());
    let st = a.e.state();
    st.arcs.past.iter().filter(|p| p.template == FLOOD).count() == 1
        && !st.arcs.runs.iter().any(|r| r.template == FLOOD)
}

pub fn explorer_enacts_own_law(seed: u64) -> bool {
    let content = crate::lab::load(&super::runner::content_root(), &[]).unwrap();
    let mut g = Game::new(
        &content,
        &Setup::small(seed, None),
        Driver::parse("explorer_legislator"),
    )
    .unwrap();
    g.run_to(72, &[], &mut Vec::new());
    let st = g.e.state();
    let Some(me) = st.player.as_ref().map(|p| p.politician) else {
        return false;
    };
    st.policy
        .instances
        .iter()
        .any(|p| p.sponsor == Some(me) && p.enacted_on.is_some())
}
