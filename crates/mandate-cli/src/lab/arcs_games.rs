//! Staged arcs in the lab (DEC-109, task A1's acceptance): a flood the lab
//! plants (`Intervention::Disaster`) raises the flood arc in eight seeds of
//! eight, and a fork of a game with a run in progress continues exactly as
//! the original does (a fork is a copy of the state; the run is in it).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use mandate_sim::events::EventKind;

/// The seeds of the acceptance run.
const SEEDS: [u64; 8] = [1, 2, 3, 7, 31, 99, 777, 2026];

use super::seed_reqs::flooded;

#[test]
fn a_planted_flood_raises_the_arc_in_eight_seeds_of_eight() {
    let mut raised = 0;
    for seed in SEEDS {
        let mut g = flooded(seed);
        // The strike falls on the next month's first day; the arc is
        // raised at the weekly step after it.
        g.step_month();
        g.step_month();
        let st = g.e.state();
        let struck = st.events.iter().any(|e| {
            matches!(&e.kind, EventKind::Disaster { municipality, .. } if *municipality == g.home())
        });
        let run = st
            .arcs
            .runs
            .iter()
            .chain(std::iter::empty())
            .find(|r| r.template == "arc.flood");
        let ended = st.arcs.past.iter().any(|p| p.template == "arc.flood");
        println!(
            "seed {seed}: struck {struck}, run {:?}",
            run.map(|r| (&r.stage, r.cast.len()))
        );
        assert!(struck, "seed {seed}: the planted flood struck");
        if run.is_some() || ended {
            raised += 1;
        }
    }
    assert_eq!(
        raised,
        SEEDS.len(),
        "the flood raises the arc in every seed"
    );
}

#[test]
fn a_fork_with_a_run_in_progress_continues_as_the_original() {
    // A world where the idle game lets the run drift to an exit by month 12
    // (tests/seeds.json, found again when the world changes).
    let mut a = flooded(mandate_seeds::seed_for(
        "flood_arc_exits_idle_by_month_12",
        super::seed_reqs::flood_arc_exits_idle_by_month_12,
    ));
    a.run_to(3, &[], &mut Vec::new());
    assert!(
        !a.e.state().arcs.runs.is_empty(),
        "the run is in progress at the fork"
    );
    let mut b = a.fork().unwrap();
    assert_eq!(a.e.state_hash(), b.e.state_hash());
    a.run_to(12, &[], &mut Vec::new());
    b.run_to(12, &[], &mut Vec::new());
    assert_eq!(a.e.state_hash(), b.e.state_hash());
    assert_eq!(a.e.state().arcs, b.e.state().arcs);
    // The idle game let it drift to an exit, and the original and the
    // fork agree on which.
    assert_eq!(a.e.state().arcs.past.len(), 1);
}
