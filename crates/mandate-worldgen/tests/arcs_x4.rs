#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]
//! The arcs rewritten to the standard (EXCHANGE_SCENES X4, inside
//! STAGED_ARCS A6): the plant on its review, the favour's bind and its
//! ways out, the rival's weeks before the debate.
//!
//! - a plant put on short time raises the plant arc with a named plant, its
//!   steward and its manager from live state; the notice stage carries the
//!   plant's own date; the steward's concession and the manager's case are
//!   relief on that plant's margin, applied by the economy; a passive and an
//!   engaged player meet the same review and can leave it differently;
//! - a favour the player owes raises the favour arc; a debt paid in any
//!   coin ends it at *repaid*;
//! - the rival's lead-up comes before the showdown, with preparation on
//!   offer there, and a turn that is one decision is gone from all four.

use mandate_sim::Command;
use mandate_sim::arcs::ArcActor;
use mandate_sim::arcs::beats::{self, BeatAction};
use mandate_sim::economy::plants::PlantStatus;
use mandate_sim::events::EventKind;
use mandate_testkit::{
    self as kit, Kind, Policy, Scene, end_turn, exit_of, home, only, run_of, shock_plant, world,
};

const PLANT: &str = "arc.plant_closure";
/// A small world with a plant that can close at home, whose middling
/// review (a planted fall to 0.72 of demand) a rescue can lift.
const PLANT_SEED: u64 = 8;

/// Play until the run of `template` is at `stage` or has ended, its levers
/// taken by a stand-in of `kind` (scenes left alone, as before).
fn until_stage(e: &mut mandate_sim::Engine, template: &str, stage: &str, kind: Kind, turns: u32) {
    kit::until_stage(e, template, stage, Policy::new(kind, Scene::Away), turns);
}

#[test]
fn a_plant_on_short_time_raises_the_arc_with_its_own_people() {
    let c = only(&[PLANT]);
    let mut e = world(&c, PLANT_SEED);
    let plant = shock_plant(&mut e, 0.72, 30);
    for _ in 0..20 {
        if run_of(&e, PLANT).is_some() {
            break;
        }
        end_turn(&mut e);
    }
    let st = e.state();
    let r = run_of(&e, PLANT)
        .expect("short time raised the plant arc")
        .clone();
    assert_eq!(r.stage, "short_time");
    // The plant is the one on short time, at its own place.
    let p = &st.economy.plants[plant.index()];
    assert!(
        matches!(p.status, PlantStatus::ShortTime { .. }),
        "{:?}",
        p.status
    );
    assert_eq!(r.actor("site"), Some(ArcActor::Place { id: p.place }));
    // Its steward works there; its manager does too where it has one.
    let works_there = |role: &str| {
        let id = r.person(role).unwrap();
        st.politics
            .characters
            .get(id)
            .and_then(|c| c.named.as_deref())
            .and_then(|n| n.place)
            == Some(p.place)
    };
    assert!(works_there("steward"), "the steward is the plant's");
    let has_manager = st.politics.characters.people.iter().any(|c| {
        c.named
            .as_deref()
            .is_some_and(|n| n.place == Some(p.place) && n.occupation == "occupation.plant_manager")
    });
    assert_eq!(works_there("owner"), has_manager);
    // Its district is the plant's.
    let hood = st.world.places[p.place.index()].neighborhood;
    match r.actor("hood") {
        Some(ArcActor::Area { area }) => assert!(area.hoods(&st.world).contains(&hood.index())),
        other => panic!("the hood is an area: {other:?}"),
    }
    // The root is the short time on the spine; its text has the plant.
    let root = st.events.iter().find(|ev| Some(ev.id) == r.root).unwrap();
    assert!(
        matches!(root.kind, EventKind::PlantShortTime { .. }),
        "{:?}",
        root.kind
    );
    let facts = st
        .events
        .iter()
        .find_map(|ev| match &ev.kind {
            EventKind::ArcStage { arc, facts, .. } if *arc == r.id.0 => Some(facts.clone()),
            _ => None,
        })
        .unwrap();
    for k in ["plant_key", "stem", "works_key", "jobs"] {
        assert!(facts.iter().any(|f| f.0 == k), "{k}: {facts:?}");
    }
}

#[test]
fn the_notice_carries_the_plants_date_and_the_levers_reach_the_plant() {
    let c = only(&[PLANT]);
    let mut e = world(&c, PLANT_SEED);
    let plant = shock_plant(&mut e, 0.72, 30);
    until_stage(&mut e, PLANT, "notice", Kind::Passive, 60);
    let r = run_of(&e, PLANT).expect("a notice").clone();
    assert_eq!(r.stage, "notice");
    let st = e.state();
    let PlantStatus::Threatened { closes_on, .. } = st.economy.plants[plant.index()].status else {
        panic!("under notice");
    };
    // The stage's text has the plant's own date.
    let facts = st
        .events
        .iter()
        .rev()
        .find_map(|ev| match &ev.kind {
            EventKind::ArcStage {
                arc, stage, facts, ..
            } if *arc == r.id.0 && stage == "notice" => Some(facts.clone()),
            _ => None,
        })
        .unwrap();
    let day = closes_on.calendar().day.to_string();
    assert!(
        facts.contains(&("closes_day".to_string(), day)),
        "{facts:?}"
    );
    // The levers are on offer: the gate, the steward, the manager.
    let offered: Vec<String> = beats::offers(st, &c)
        .iter()
        .filter(|o| o.run == r.id)
        .map(|o| o.beat.beat.clone())
        .collect();
    for b in ["gate", "short", "case"] {
        assert!(offered.contains(&b.to_string()), "{b}: {offered:?}");
    }
    // An engaged turn: at the gate, short time carried, the case made, and
    // the relief is on the plant's margin once the economy's month turns.
    let before = st.economy.plants[plant.index()].relief;
    let s = kit::seat(&e);
    mandate_sim::standin::play_turn(&mut e, Kind::Engaged, s);
    let r = run_of(&e, PLANT).unwrap();
    for m in ["at_gate", "conceded", "offered"] {
        assert!(r.has_mark(m), "{m}: {:?}", r.marks);
    }
    // Held by the arcs until the month turns (the economy applies monthly).
    assert!(e.state().arcs.deferred.iter().any(|q| matches!(
        q.effect,
        mandate_sim::scheduler::Effect::PlantRelief { plant: p, .. } if p == plant.0
    )));
    for _ in 0..8 {
        if e.state().arcs.deferred.is_empty() {
            break;
        }
        end_turn(&mut e);
    }
    let after = e.state().economy.plants[plant.index()].relief;
    assert!(
        f64::from(after) >= f64::from(before) + 0.15,
        "relief {before} -> {after}"
    );
}

#[test]
fn a_rescue_can_lift_a_notice_that_a_passive_player_meets_closed() {
    let c = only(&[PLANT]);
    let mut passive = world(&c, PLANT_SEED);
    let mut engaged = world(&c, PLANT_SEED);
    shock_plant(&mut passive, 0.72, 30);
    shock_plant(&mut engaged, 0.72, 30);
    until_stage(&mut passive, PLANT, "__end", Kind::Passive, 120);
    until_stage(&mut engaged, PLANT, "__end", Kind::Engaged, 120);
    let (p, g) = (
        exit_of(&passive, PLANT).unwrap(),
        exit_of(&engaged, PLANT).unwrap(),
    );
    assert_eq!(g, "reprieved");
    assert_ne!(p, "reprieved");
}

#[test]
fn a_debt_raises_the_favour_and_paying_it_in_any_coin_ends_it() {
    let c = only(&["arc.favour_debt"]);
    let mut e = world(&c, 5);
    let town = home(&e);
    let date = e.state().calendar.date;
    let me = e
        .state()
        .politics
        .get(e.state().player.as_ref().unwrap().politician)
        .unwrap()
        .character;
    let creditor = e
        .state()
        .politics
        .politicians
        .iter()
        .find(|p| !p.is_player && p.municipality() == town.0 && p.role.post().is_some())
        .map(|p| p.character)
        .unwrap();
    e.setup(|st| {
        mandate_sim::characters::owe(&mut st.politics, me, creditor, "favour-lab", None, date);
    });
    for _ in 0..4 {
        if run_of(&e, "arc.favour_debt").is_some() {
            break;
        }
        end_turn(&mut e);
    }
    let r = run_of(&e, "arc.favour_debt").expect("a debt raised the favour");
    assert_eq!(r.person("creditor"), Some(creditor));
    assert_eq!(r.stage, "debt");
    // Paid in another coin: the debt is gone, and the run ends at repaid.
    e.setup(|st| {
        mandate_sim::characters::repay(&mut st.politics, me, creditor);
    });
    for _ in 0..3 {
        end_turn(&mut e);
    }
    assert_eq!(exit_of(&e, "arc.favour_debt").as_deref(), Some("repaid"));
    assert_eq!(
        mandate_sim::characters::favours_owed(&e.state().politics, me, creditor),
        0
    );
}

#[test]
fn the_rival_prepares_in_the_lead_up_before_the_showdown() {
    let c = only(&["arc.rival"]);
    let mut found = false;
    for seed in [1u64, 7, 2026] {
        let mut e = world(&c, seed);
        for _ in 0..80 {
            if run_of(&e, "arc.rival").is_some_and(|r| r.stage == "lead_up") {
                break;
            }
            end_turn(&mut e);
        }
        let Some(r) = run_of(&e, "arc.rival")
            .filter(|r| r.stage == "lead_up")
            .cloned()
        else {
            continue;
        };
        found = true;
        let offered: Vec<String> = beats::offers(e.state(), &c)
            .iter()
            .filter(|o| o.run == r.id)
            .map(|o| o.beat.beat.clone())
            .collect();
        // The cuttings and a walk are always there to take; a word with an
        // ally or a backer where the cast has one.
        assert!(
            offered.contains(&"cuttings".to_string()) || r.actor("paper").is_none(),
            "{offered:?}"
        );
        assert!(
            offered.contains(&"walk".to_string()) || r.actor("ward").is_none(),
            "{offered:?}"
        );
        assert!(
            !offered.iter().any(|b| b == "debate"),
            "the debate is the showdown's"
        );
        // On to the showdown, where the debate is on offer; what was done
        // in the lead-up is not offered twice.
        let cuttings = beats::offers(e.state(), &c)
            .into_iter()
            .find(|o| o.beat.beat == "cuttings");
        if let Some(BeatAction::Visit { verb, location }) = cuttings.map(|o| o.action) {
            e.apply(Command::Player(mandate_sim::actions::PlayerCommand::Go {
                location,
                then: Some(Box::new(mandate_sim::actions::PlayerCommand::Act {
                    action: mandate_sim::actions::Action::Visit { verb },
                    late_night: false,
                })),
            }))
            .unwrap();
        }
        for _ in 0..20 {
            if run_of(&e, "arc.rival").is_some_and(|r| r.stage == "showdown") {
                break;
            }
            end_turn(&mut e);
        }
        if let Some(r) = run_of(&e, "arc.rival").filter(|r| r.stage == "showdown") {
            let offered: Vec<String> = beats::offers(e.state(), &c)
                .iter()
                .filter(|o| o.run == r.id)
                .map(|o| o.beat.beat.clone())
                .collect();
            assert!(offered.contains(&"debate".to_string()), "{offered:?}");
            if r.has_mark("read") {
                assert!(!offered.contains(&"cuttings".to_string()), "{offered:?}");
            }
        }
    }
    assert!(found, "no campaign reached the lead-up in three seeds");
}

#[test]
fn no_turn_of_the_four_is_one_decision() {
    let b = kit::pack();
    let flagged = mandate_content::arcs::single_decision_turns(&b);
    for arc in [
        "arc.rival",
        "arc.favour_debt",
        "arc.flood",
        "arc.plant_closure",
    ] {
        assert!(!flagged.iter().any(|x| x.0 == arc), "{arc}: {flagged:?}");
    }
    // The decisions the scenes and beats replaced are gone.
    for d in [
        "dilemma.debate_challenge",
        "dilemma.workers_campaign",
        "dilemma.favour_called_in",
        "dilemma.flood_meeting",
    ] {
        assert!(b.dilemma(d).is_none(), "{d} is retired");
    }
}
