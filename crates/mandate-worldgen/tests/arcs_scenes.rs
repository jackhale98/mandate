#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]
//! Played scenes matter (the arcs' infrastructure), on the harness:
//!
//! - a scene is on offer for at least one turn the player can act in,
//!   whatever its content's clock says (the engine holds it to a turn's
//!   length from the stage's entry);
//! - where the ending is decided the scene played counts (`arc_scene`): a
//!   cold player who turns up for the flood's hall ends elsewhere than one
//!   who leaves the chair empty, in some worlds, and only through the hall.

use mandate_content::ContentBundle;
use mandate_sim::arcs::exchange;
use mandate_testkit::{self as kit, Policy, plant_flood};
use std::sync::Arc;

const FLOOD: &str = "arc.flood";

/// The flood alone, its meeting's clock as short as content can make it.
fn short_meeting() -> Arc<ContentBundle> {
    let mut b = kit::keep(kit::pack(), &[FLOOD]);
    let a = b.arcs.iter_mut().find(|a| a.id == FLOOD).unwrap();
    let m = a.stages.iter_mut().find(|s| s.id == "meeting").unwrap();
    m.clock.as_mut().unwrap().days = Some([1, 1]);
    Arc::new(b)
}

#[test]
fn a_scene_is_on_offer_for_a_turn_whatever_its_clock() {
    let c = short_meeting();
    let mut checked = 0;
    for seed in 1..=8u64 {
        let mut e = kit::world(&c, seed);
        plant_flood(&mut e, 0.9);
        if kit::raised_within(&mut e, FLOOD, 6).is_none() {
            continue;
        }
        // Nobody acts: the run comes to its hall on its own clock.
        if !kit::until_stage(&mut e, FLOOD, "meeting", Policy::PASSIVE, 20) {
            continue;
        }
        let r = kit::run_of(&e, FLOOD).unwrap().clone();
        let turn = mandate_sim::characters::routines::turn_days(e.state(), e.content());
        let clock = r.clock.unwrap();
        assert!(
            clock >= r.since.add_days(turn),
            "seed {seed}: entered {:?}, clock {clock:?}, turn {turn}",
            r.since
        );
        // The player's next turn has it on offer, and a player who turns
        // up holds it (not an empty chair).
        let open = exchange::view::open_scenes(e.state(), e.content());
        assert!(open.iter().any(|(id, _, _)| *id == r.id), "seed {seed}");
        let met = kit::meet(&mut e, FLOOD, kit::Scene::Best).expect("the hall was held");
        assert_ne!(met.outcome, "absent");
        checked += 1;
        if checked >= 2 {
            break;
        }
    }
    assert!(checked > 0, "no flood came to its hall in eight worlds");
}

#[test]
fn where_the_flood_ends_reads_the_hall_it_played() {
    let c = kit::only(&[FLOOD]);
    let mut differ = Vec::new();
    for seed in 1..=16u64 {
        let p = kit::exits(
            &c,
            seed,
            FLOOD,
            &|e| plant_flood(e, 0.9),
            &[Policy::PASSIVE, Policy::COLD],
            60,
        );
        let (away, cold) = (&p[0], &p[1]);
        let held = cold.met.iter().any(|m| m.beat == "meeting.room");
        println!(
            "seed {seed}: away {:?} {:?}, cold {:?} {:?}",
            away.exit, away.outcomes, cold.exit, cold.outcomes
        );
        if held && away.exit != cold.exit {
            // The one difference between the two is the hall.
            assert_eq!(away.outcomes, vec!["absent".to_string()]);
            differ.push(seed);
        }
        if differ.len() >= 2 {
            break;
        }
    }
    assert!(
        !differ.is_empty(),
        "turning up for the hall changed no flood's ending in 16 worlds"
    );
}
