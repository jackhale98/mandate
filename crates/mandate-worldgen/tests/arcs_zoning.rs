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
use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::actions::PlayerCommand;
use mandate_sim::arcs::beats::{self, BeatAction};
use mandate_sim::arcs::exchange::{self, ExchangeAct};
use mandate_sim::arcs::{ArcActor, ArcRun};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::events::EventKind;
use mandate_sim::spine::{Cause, Link, Process};
use mandate_sim::standin::{self, Kind, Seat};
use mandate_sim::world::{Jurisdiction, MunicipalityId};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

const ZONING: &str = "arc.zoning";
/// A small world whose home town has housing as its strongest worry from
/// the start (measured on 2026-11-25 with the base pack; the grader's seed
/// 2): the arc is raised in the first turns.
const ZONING_SEED: u64 = 2;

fn pack() -> ContentBundle {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    load(&src).unwrap().0
}

/// The base pack with one arc.
fn only(id: &str) -> Arc<ContentBundle> {
    let mut b = pack();
    b.arcs.retain(|a| a.id == id);
    Arc::new(b)
}

fn engine(c: &Arc<ContentBundle>, seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

fn home(e: &Engine) -> MunicipalityId {
    mandate_sim::player::home(e.state()).unwrap()
}

fn me(e: &Engine) -> CharacterId {
    characters::player_character(e.state()).unwrap()
}

fn run_of<'a>(e: &'a Engine, template: &str) -> Option<&'a ArcRun> {
    e.state().arcs.runs.iter().find(|r| r.template == template)
}

fn exit_of(e: &Engine, template: &str) -> Option<String> {
    e.state()
        .arcs
        .past
        .iter()
        .rev()
        .find(|p| p.template == template)
        .map(|p| p.exit.clone())
}

fn end_turn(e: &mut Engine) {
    e.apply(Command::EndTurn).unwrap();
}

fn seat(e: &Engine) -> Seat {
    Seat::of_player(e.state(), e.content()).unwrap_or(Seat {
        at: Jurisdiction::town(home(e)),
        post: None,
    })
}

fn occupation(e: &Engine, who: CharacterId) -> String {
    characters::profile(&e.state().politics, &e.state().world, who)
        .and_then(|p| p.occupation)
        .unwrap_or_default()
}

/// End turns until the run is raised.
fn raised(e: &mut Engine) -> ArcRun {
    for _ in 0..40 {
        if let Some(r) = run_of(e, ZONING) {
            return r.clone();
        }
        end_turn(e);
    }
    panic!("housing as the strongest worry raised no zoning fight");
}

/// How a scripted player meets the meeting: never (an empty chair), or
/// turning up and saying, each round, the first line of these kinds on
/// offer (else the safe line).
#[derive(Clone, Copy)]
enum Meeting {
    Away,
    Lines(&'static [MoveKind]),
    /// The line whose shown change, weighed by the groups and the chance
    /// shown, is largest (the middle of a range): what the screen shows.
    Best,
}

/// The shown change of a line across the room.
fn shown(v: &exchange::view::SceneView, o: &exchange::view::OptionView) -> f64 {
    let mid = |s: &mandate_sim::arcs::exchange::landing::Span| (s.lo + s.hi) / 2.0;
    v.groups
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let up = o.stands.get(i).map_or(0.0, mid);
            let down = o.fails.get(i).map_or(0.0, mid);
            f64::from(g.weight) * (o.chance * up + (1.0 - o.chance) * down)
        })
        .sum()
}

/// Turn up at the scene of the run if it is open, and play its rounds.
/// Returns the lines' kinds offered in the first round.
fn meet(e: &mut Engine, how: Meeting) -> Option<Vec<MoveKind>> {
    let prefer: &[MoveKind] = match how {
        Meeting::Away => return None,
        Meeting::Lines(p) => p,
        Meeting::Best => &[],
    };
    let (run, beat, location) =
        beats::offers(e.state(), e.content())
            .into_iter()
            .find_map(|o| match o.action {
                BeatAction::Exchange {
                    run,
                    beat,
                    location,
                } => Some((run, beat, location)),
                _ => None,
            })?;
    e.apply(Command::Player(PlayerCommand::Go {
        location,
        then: Some(Box::new(PlayerCommand::Exchange {
            run,
            beat: beat.clone(),
            act: ExchangeAct::Begin,
        })),
    }))
    .ok()?;
    let mut first = None;
    for _ in 0..8 {
        let v = exchange::view(e.state(), e.content(), run, &beat)?;
        if v.phase != exchange::Phase::UnderWay {
            break;
        }
        let kinds: Vec<MoveKind> = v.options.iter().map(|o| o.kind).collect();
        first.get_or_insert(kinds);
        let best = || {
            v.options
                .iter()
                .max_by(|a, b| shown(&v, a).total_cmp(&shown(&v, b)))
        };
        let o = match how {
            Meeting::Best => best(),
            _ => prefer
                .iter()
                .find_map(|k| v.options.iter().find(|o| o.kind == *k))
                .or_else(|| v.options.iter().find(|o| o.kind == MoveKind::Pass)),
        }
        .unwrap();
        e.apply(Command::Player(PlayerCommand::Exchange {
            run,
            beat: beat.clone(),
            act: ExchangeAct::Say {
                option: o.id.clone(),
            },
        }))
        .unwrap();
    }
    first
}

/// Play until the run ends (at most `turns` turns): the stand-in of `kind`
/// takes the arc's levers each turn, and the meeting is met as `how` says.
/// Returns the exit and the lines offered in the meeting's first round.
fn play(e: &mut Engine, kind: Kind, how: Meeting, turns: u32) -> (String, Option<Vec<MoveKind>>) {
    let mut lines = None;
    for _ in 0..turns {
        if let Some(x) = exit_of(e, ZONING) {
            return (x, lines);
        }
        // The meeting first: it takes the evening the levers would.
        if let Some(l) = meet(e, how) {
            lines = Some(l);
        }
        if kind != Kind::Passive && run_of(e, ZONING).is_some() {
            let s = seat(e);
            standin::play_turn(e, kind, s);
        }
        end_turn(e);
    }
    (exit_of(e, ZONING).expect("the zoning fight ended"), lines)
}

#[test]
fn housing_as_the_towns_strongest_worry_raises_the_fight_with_its_own_people() {
    let c = only(ZONING);
    let mut e = engine(&c, ZONING_SEED);
    let r = raised(&mut e);
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
    // On the spine: a matter raised by its own conditions (no event made
    // it true: the town's rents), and the stage continues the thread.
    let log = &st.events;
    let up = log
        .iter()
        .find(|ev| matches!(&ev.kind, EventKind::ArcRaised { arc, .. } if *arc == r.id.0))
        .unwrap();
    assert_eq!(up.causes, vec![Cause::Process(Process::Arcs)]);
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
    let c = only(ZONING);
    let mut passive = engine(&c, ZONING_SEED);
    let mut engaged = engine(&c, ZONING_SEED);
    let r = raised(&mut passive);
    raised(&mut engaged);
    let (p, _) = play(&mut passive, Kind::Passive, Meeting::Away, 80);
    let (g, _) = play(&mut engaged, Kind::Engaged, Meeting::Best, 80);
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
    let c = only(ZONING);
    let lines: &[MoveKind] = &[MoveKind::Proof, MoveKind::Second];
    // Prepared: walked the site, saw the drawings, asked the organiser.
    let mut ready = engine(&c, ZONING_SEED);
    raised(&mut ready);
    let (_, said) = play(&mut ready, Kind::Engaged, Meeting::Lines(lines), 80);
    let said = said.expect("the meeting was held");
    assert!(said.contains(&MoveKind::Proof), "{said:?}");
    assert!(said.contains(&MoveKind::Second), "{said:?}");
    // Turned up cold: nothing to show, nobody to speak.
    let mut cold = engine(&c, ZONING_SEED);
    raised(&mut cold);
    let (_, said) = play(&mut cold, Kind::Passive, Meeting::Lines(lines), 80);
    let said = said.expect("the meeting was held");
    assert!(!said.contains(&MoveKind::Proof), "{said:?}");
    assert!(!said.contains(&MoveKind::Second), "{said:?}");
    assert!(said.contains(&MoveKind::Pass), "{said:?}");
}

#[test]
fn an_edge_that_reads_a_law_names_its_enactment() {
    let c = only(ZONING);
    let mut e = engine(&c, ZONING_SEED);
    let r = raised(&mut e);
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
