#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]
//! The zoning fight (`arcs/zoning.jsonc`; DEC-109, DEC-112): the pilot arc,
//! written with the machinery the four X4 arcs left.
//!
//! - housing as the town's strongest worry raises it, with a builder, an
//!   organiser, a neighbour and a hall from live state;
//! - a passive player sees the plans shelved; an engaged one who walks the
//!   site, sees the drawings, keeps the organiser's ask (a real bill) and
//!   plays the meeting ends elsewhere, with the difference in simulated
//!   state (a bill before the body, an opinion);
//! - what the player learned before the meeting is what they can say in
//!   it: the drawings to show, the organiser to speak; without them the
//!   lines are not there;
//! - an edge that reads a law names the law's enactment on the spine.

use mandate_content::exchange::MoveKind;
use mandate_sim::Engine;
use mandate_sim::arcs::ArcActor;
use mandate_sim::arcs::beats;
use mandate_sim::characters;
use mandate_sim::events::EventKind;
use mandate_sim::spine::{Cause, Link, Pressure, Process};
use mandate_testkit::{
    self as kit, Policy, Scene, end_turn, exit_of, home, me, occupation, only, world,
};

const ZONING: &str = "arc.zoning";
/// A small world whose home town has housing as its strongest worry from
/// the start (measured on 2026-11-25 with the base pack; the grader's seed
/// 2): the arc is raised in the first turns.
const ZONING_SEED: u64 = 2;

/// Turns enough for any path of the fight.
const TURNS: u32 = 80;

#[test]
fn housing_as_the_towns_strongest_worry_raises_the_fight_with_its_own_people() {
    let c = only(&[ZONING]);
    let mut e = world(&c, ZONING_SEED);
    let r = kit::raised(&mut e, ZONING);
    assert_eq!(r.stage, "plans");
    let st = e.state();
    // The cast is from live state: a neighbourhood of the town, three
    // different people of the roles the arc asks for, a hall.
    assert!(matches!(r.actor("hood"), Some(ArcActor::Area { .. })));
    assert!(matches!(r.actor("hall"), Some(ArcActor::Place { .. })));
    let (dev, org, nb) = (
        r.person("developer").unwrap(),
        r.person("organiser").unwrap(),
        r.person("neighbour").unwrap(),
    );
    assert!(dev != org && org != nb && dev != nb);
    assert!(
        ["occupation.business_leader", "occupation.hotel_manager"]
            .contains(&occupation(&e, dev).as_str())
    );
    assert!(
        [
            "occupation.retiree",
            "occupation.resident",
            "occupation.clergy",
            "occupation.shopkeeper"
        ]
        .contains(&occupation(&e, nb).as_str())
    );
    // On the spine: no event made it true, so its root is the worry its
    // `raise` read (housing, the town's strongest), with the arcs' rule;
    // and the stage continues the thread.
    let log = &st.events;
    let up = log
        .iter()
        .find(|ev| matches!(&ev.kind, EventKind::ArcRaised { arc, .. } if *arc == r.id.0))
        .unwrap();
    let housing = c
        .concerns
        .iter()
        .position(|x| x.id == "concern.housing")
        .unwrap();
    assert_eq!(
        up.causes,
        vec![
            Cause::Pressure(Pressure::Worry {
                concern: housing as u16
            }),
            Cause::Process(Process::Arcs)
        ]
    );
    assert!(log.iter().any(|ev| matches!(&ev.kind,
        EventKind::ArcStage { arc, stage, .. } if *arc == r.id.0 && stage == "plans")
        && ev.causes.contains(&Cause::Event(up.id, Link::Continues))));
    // What the plans stage offers: the site, the drawings, the objections.
    let offered: Vec<String> = beats::offers(st, &c)
        .iter()
        .filter(|o| o.run == r.id)
        .map(|o| o.beat.beat.clone())
        .collect();
    for b in ["site", "drawings", "objections"] {
        assert!(offered.contains(&b.to_string()), "{b}: {offered:?}");
    }
}

#[test]
fn a_passive_player_sees_the_plans_shelved_and_an_engaged_one_does_not() {
    let c = only(&[ZONING]);
    let mut passive = world(&c, ZONING_SEED);
    let mut engaged = world(&c, ZONING_SEED);
    let r = kit::raised(&mut passive, ZONING);
    kit::raised(&mut engaged, ZONING);
    let p = kit::play(&mut passive, ZONING, Policy::PASSIVE, TURNS)
        .exit
        .unwrap();
    let g = kit::play(&mut engaged, ZONING, Policy::BEST, TURNS)
        .exit
        .unwrap();
    assert_eq!(p, "shelved");
    assert_ne!(g, "shelved");
    println!("seed {ZONING_SEED}: passive {p}, engaged {g}");
    // The engaged player played the meeting; the passive one left an
    // empty chair.
    let outcome = |e: &Engine| {
        e.state().events.iter().find_map(|ev| match &ev.kind {
            EventKind::ArcExchangeEnded { arc, outcome, .. } if *arc == r.id.0 => {
                Some(outcome.clone())
            }
            _ => None,
        })
    };
    assert_eq!(outcome(&passive).as_deref(), Some("absent"));
    let o = outcome(&engaged).expect("the meeting was held");
    assert_ne!(o, "absent");
    println!("the meeting: {o}");
    // The difference is in simulated state: a bill of the player's went
    // before the body, and the organiser thinks better of them.
    let mine = |e: &Engine| {
        let pol = e.state().player.as_ref().unwrap().politician;
        e.state().policy.instances.iter().any(|p| {
            p.sponsor == Some(pol)
                && ["policy.upzoning", "policy.affordable_housing"].contains(&p.template.as_str())
        })
    };
    assert!(!mine(&passive));
    assert!(mine(&engaged));
    let org = r.person("organiser").unwrap();
    let op = |e: &Engine| characters::opinion(&e.state().politics, org, me(e));
    assert!(
        op(&engaged) > op(&passive) + 5.0,
        "{} vs {}",
        op(&engaged),
        op(&passive)
    );
}

#[test]
fn what_was_learned_before_the_meeting_is_what_can_be_said_in_it() {
    let c = only(&[ZONING]);
    let lines: &[MoveKind] = &[MoveKind::Proof, MoveKind::Second];
    // Prepared: walked the site, saw the drawings, asked the organiser.
    let mut ready = world(&c, ZONING_SEED);
    kit::raised(&mut ready, ZONING);
    let played = kit::play(
        &mut ready,
        ZONING,
        Policy::new(kit::Kind::Engaged, Scene::Lines(lines)),
        TURNS,
    );
    let said = played
        .met
        .first()
        .expect("the meeting was held")
        .first()
        .to_vec();
    assert!(said.contains(&MoveKind::Proof), "{said:?}");
    assert!(said.contains(&MoveKind::Second), "{said:?}");
    // Turned up cold: nothing to show, nobody to speak.
    let mut cold = world(&c, ZONING_SEED);
    kit::raised(&mut cold, ZONING);
    let played = kit::play(
        &mut cold,
        ZONING,
        Policy::new(kit::Kind::Passive, Scene::Lines(lines)),
        TURNS,
    );
    let said = played
        .met
        .first()
        .expect("the meeting was held")
        .first()
        .to_vec();
    assert!(!said.contains(&MoveKind::Proof), "{said:?}");
    assert!(!said.contains(&MoveKind::Second), "{said:?}");
    assert!(said.contains(&MoveKind::Pass), "{said:?}");
}

#[test]
fn an_edge_that_reads_a_law_names_its_enactment() {
    let c = only(&[ZONING]);
    let mut e = world(&c, ZONING_SEED);
    let r = kit::raised(&mut e, ZONING);
    assert_eq!(r.stage, "plans");
    // The rules are loosened while the plans are talked about: a law
    // enacted at home (as lawmaking records one).
    let town = home(&e);
    let mut enacted = None;
    e.setup(|st| {
        use mandate_sim::policy::{PolicyInstance, PolicyInstanceId, PolicyStatus};
        let id: PolicyInstanceId = st.policy.ids.alloc();
        let date = st.calendar.date;
        let ev = st.events.record(
            date,
            EventKind::PolicyEnacted {
                policy: id.0,
                municipality: town.0,
                yes: 4,
                no: 3,
                overrode_veto: false,
            },
            Cause::Process(Process::Lawmaking),
        );
        st.policy.instances.push(PolicyInstance {
            id,
            template: "policy.upzoning".into(),
            tier: mandate_content::TierKey::Local,
            jurisdiction: town.0,
            sponsor: None,
            params: [("density".to_string(), 1.0)].into_iter().collect(),
            clauses: Vec::new(),
            status: PolicyStatus::Enacted,
            stage: 1,
            next_stage_on: None,
            votes: Vec::new(),
            introduced_on: Some(date),
            enacted_on: Some(date),
            progress: 0.0,
            awareness: 0.1,
            end_reason: None,
            cause: None,
            ledger: Default::default(),
            family: None,
            estimate: None,
        });
        enacted = Some(ev);
    });
    for _ in 0..3 {
        if exit_of(&e, ZONING).is_some() {
            break;
        }
        end_turn(&mut e);
    }
    assert_eq!(exit_of(&e, ZONING).as_deref(), Some("upzoned"));
    let ended = e
        .state()
        .events
        .iter()
        .find(|ev| matches!(&ev.kind, EventKind::ArcEnded { arc, .. } if *arc == r.id.0))
        .unwrap();
    assert!(
        ended
            .causes
            .contains(&Cause::Event(enacted.unwrap(), Link::Caused)),
        "{:?}",
        ended.causes
    );
}
