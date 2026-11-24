#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]
//! Staged arcs (DEC-109, task A1): the arc core, and the flood arc played
//! from the day the water comes.
//!
//! - a planted flood raises the arc, with its cast bound to real people and
//!   places, on the spine (`Thread::Arc`, the strike as its root);
//! - two scripted command lists in one seed, a passive one and an engaged
//!   one, reach different exits, and the difference is in simulated state
//!   (homes repaired, approval in the struck neighbourhood, a memory), not
//!   only in the run's own record;
//! - every beat of the flood a player takes is reached in quick mode and
//!   through Places, and the beats nobody takes happen;
//! - with no arc content, nothing changes, and the state is as it was;
//! - what the engine does not do yet is never raised;
//! - the state is bounded, and holds no town key.

use mandate_content::arcs::{ArcEffect, BeatKind, GatedEffect};
use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::arcs::beats::{self, BeatAction, BeatOffer};
use mandate_sim::arcs::{ArcActor, ArcRun, ArcSummary};
use mandate_sim::characters::{self, CharacterId, MemoryKind};
use mandate_sim::environment::{DeferredHazard, HazardKind};
use mandate_sim::events::EventKind;
use mandate_sim::places::Location;
use mandate_sim::spine::{Cause, Link, Thread};
use mandate_sim::talk::TalkTarget;
use mandate_sim::world::{Area, Jurisdiction, MunicipalityId, NeighborhoodId};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

fn base() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

/// The base pack with no arc content.
fn without_arcs(c: &ContentBundle) -> Arc<ContentBundle> {
    let mut b = c.clone();
    b.arcs.clear();
    Arc::new(b)
}

fn engine(c: &Arc<ContentBundle>, seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

fn home(e: &Engine) -> MunicipalityId {
    mandate_sim::player::home(e.state()).unwrap()
}

/// A flood strikes the player's town at the next monthly hazard step (as
/// the lab plants one: a held hazard past its longest deferral).
fn plant_flood(e: &mut Engine, severity: f64) {
    let town = home(e).0;
    let max = e.content().director.chance.hazard_max_defer_days;
    e.setup(|st| {
        let drawn = mandate_core::SimDate(st.calendar.date.0.saturating_sub(max + 1));
        st.environment.deferred.push(DeferredHazard {
            municipality: town,
            kind: HazardKind::Flood,
            severity,
            drawn,
            hold_event: None,
        });
    });
}

fn end_turn(e: &mut Engine) {
    e.apply(Command::EndTurn).unwrap();
}

fn run(e: &Engine) -> Option<&ArcRun> {
    e.state().arcs.runs.first()
}

fn last_exit(e: &Engine) -> Option<&ArcSummary> {
    e.state().arcs.past.last()
}

fn ended(e: &Engine) -> bool {
    run(e).is_none() && last_exit(e).is_some()
}

/// End turns until the flood arc is raised (at most ten).
fn until_raised(e: &mut Engine) {
    for _ in 0..10 {
        if run(e).is_some() {
            return;
        }
        end_turn(e);
    }
    panic!("the flood raised no arc in ten turns");
}

fn hood_of(cast: &[mandate_sim::arcs::ArcCastEntry]) -> NeighborhoodId {
    match cast.iter().find(|c| c.role == "hood").map(|c| c.actor) {
        Some(ArcActor::Area {
            area: Area::Hood { id },
        }) => id,
        other => panic!("the hood is a neighbourhood: {other:?}"),
    }
}

fn person_of(cast: &[mandate_sim::arcs::ArcCastEntry], role: &str) -> Option<CharacterId> {
    match cast.iter().find(|c| c.role == role)?.actor {
        ArcActor::Person { id } => Some(id),
        _ => None,
    }
}

/// Unrepaired damage in a neighbourhood, per mille of its homes.
fn damaged_pm(e: &Engine, h: NeighborhoodId) -> u16 {
    e.state().environment.damage(h).map_or(0, |d| d.damaged_pm)
}

/// The player's mean approval among a neighbourhood's residents.
fn approval(e: &Engine, h: NeighborhoodId) -> f64 {
    let cs = &e.state().population.hoods[h.index()].cohorts;
    let n: f64 = cs.iter().map(|c| f64::from(c.count)).sum();
    cs.iter()
        .map(|c| f64::from(c.player_approval) * f64::from(c.count))
        .sum::<f64>()
        / n.max(1.0)
}

fn me(e: &Engine) -> CharacterId {
    characters::player_character(e.state()).unwrap()
}

fn opinion_of_me(e: &Engine, who: CharacterId) -> f32 {
    characters::opinion(&e.state().politics, who, me(e))
}

fn remembers(e: &Engine, who: CharacterId, kind: MemoryKind) -> bool {
    let me = me(e);
    e.state()
        .politics
        .characters
        .get(who)
        .is_some_and(|c| c.memories.iter().any(|m| m.actor == me && m.kind == kind))
}

/// The beats that happened, as `stage.beat:kind:ok`, from the log.
fn beats_done(e: &Engine) -> BTreeSet<String> {
    e.state()
        .events
        .iter()
        .filter_map(|ev| match &ev.kind {
            EventKind::ArcBeat {
                stage,
                beat,
                kind,
                ok,
                ..
            } => Some(format!(
                "{stage}.{beat}:{kind}:{}",
                if *ok { "done" } else { "no" }
            )),
            _ => None,
        })
        .collect()
}

/// A scripted player: which beats it takes, and how it gets to them.
#[derive(Clone, Copy)]
struct Script {
    /// Take every beat on offer once the run has reached this stage
    /// (`None`: never, the passive script: every turn just ends).
    from: Option<&'static str>,
    /// Through Places: walk there (`MoveTo`), then do it. Otherwise quick
    /// mode: one request, routed.
    places: bool,
    /// The answers to the flood's two decisions.
    money: &'static str,
    meeting: &'static str,
}

const PASSIVE: Script = Script {
    from: None,
    places: false,
    money: "wait",
    meeting: "absent",
};

/// Everything, from the day the arc is raised: carry boxes, get a second
/// roof opened, press for the money, ask for a crew.
const ENGAGED: Script = Script {
    from: Some("struck"),
    places: false,
    money: "now",
    meeting: "date",
};

/// Nothing until the meeting is called; then give a date and walk the
/// repairs.
const LATE: Script = Script {
    from: Some("meeting"),
    places: false,
    money: "wait",
    meeting: "date",
};

impl Script {
    fn through_places(self) -> Script {
        Script {
            places: true,
            ..self
        }
    }

    /// Is the script acting yet?
    fn acting(&self, e: &Engine) -> bool {
        let (Some(from), Some(r)) = (self.from, run(e)) else {
            return false;
        };
        let def = e.content().arc(&r.template).unwrap();
        let at = |id: &str| def.stages.iter().position(|s| s.id == id);
        at(&r.stage) >= at(from)
    }

    /// The commands that take an offered beat now. `None`: not this turn
    /// (the person is nowhere to be found).
    fn commands(&self, e: &Engine, o: &BeatOffer) -> Option<Vec<Command>> {
        let st = e.state();
        // Go there and do it: one routed request, or a walk and the act.
        let located = |at: Location, then: PlayerCommand| -> Vec<Command> {
            if self.places {
                vec![
                    Command::Player(PlayerCommand::MoveTo { location: at }),
                    Command::Player(then),
                ]
            } else {
                vec![Command::Player(PlayerCommand::Go {
                    location: at,
                    then: Some(Box::new(then)),
                })]
            }
        };
        let talk = |person: CharacterId, topic: String| -> Option<Vec<Command>> {
            let at = characters::routines::whereabouts(st, e.content(), person)?;
            Some(located(
                at,
                PlayerCommand::Talk {
                    to: TalkTarget::Person { person },
                    topic,
                    about: None,
                    late_night: false,
                },
            ))
        };
        match &o.action {
            BeatAction::Visit { verb, location } => {
                let act = PlayerCommand::Act {
                    action: Action::Visit { verb: verb.clone() },
                    late_night: false,
                };
                if self.places {
                    Some(located(*location, act))
                } else {
                    // Quick mode: the engine finds the place.
                    Some(vec![e.route(act)])
                }
            }
            BeatAction::Talk { person, topic } => talk(*person, topic.clone()),
            BeatAction::Decision { decision } => {
                let d = st
                    .narrative
                    .decision(mandate_sim::narrative::DecisionId(*decision))?;
                let option = match d.template.as_str() {
                    "dilemma.flood_money" => self.money,
                    "dilemma.flood_meeting" => self.meeting,
                    other => panic!("no scripted answer for {other}"),
                };
                Some(vec![Command::Player(PlayerCommand::Answer {
                    decision: *decision,
                    option: option.to_string(),
                })])
            }
            BeatAction::Ask { ask } => {
                let a = st.asks.get(mandate_sim::asks::AskId(*ask))?;
                if a.offered() {
                    return Some(vec![Command::Player(PlayerCommand::RespondAsk {
                        ask: *ask,
                        accept: true,
                    })]);
                }
                // Accepted: its need is a word with someone (the ask's own
                // choice in the conversation with them).
                match a.current()? {
                    mandate_sim::asks::Need::Talk { person } => {
                        talk(*person, mandate_sim::asks::hook::choice_id(a.id))
                    }
                    _ => None,
                }
            }
        }
    }

    /// One turn: take what the script takes, then end the turn. Returns
    /// the commands given, in order.
    fn turn(&self, e: &mut Engine) -> Vec<Command> {
        let mut given = Vec::new();
        let mut tried: BTreeSet<String> = BTreeSet::new();
        for _ in 0..16 {
            if !self.acting(e) {
                break;
            }
            let offers = beats::offers(e.state(), e.content());
            let Some((key, cmds)) = offers.iter().find_map(|o| {
                let key = format!("{:?}", o.action);
                (!tried.contains(&key)).then(|| (key, self.commands(e, o)))
            }) else {
                break;
            };
            tried.insert(key);
            let Some(cmds) = cmds else { continue };
            let mut ok = true;
            for c in cmds {
                if ok && e.apply(c.clone()).is_ok() {
                    given.push(c);
                } else {
                    ok = false;
                }
            }
            if ok {
                // An ask accepted has a next step this turn.
                tried.retain(|k| !k.starts_with("Ask"));
            }
        }
        e.apply(Command::EndTurn).unwrap();
        given.push(Command::EndTurn);
        given
    }

    /// Play until the arc has ended (at most `turns` turns). Returns the
    /// script as the list of commands it gave.
    fn play(&self, e: &mut Engine, turns: u32) -> Vec<Command> {
        let mut list = Vec::new();
        for _ in 0..turns {
            if ended(e) {
                break;
            }
            list.extend(self.turn(e));
        }
        list
    }
}

const SEED: u64 = 7;
const SEVERITY: f64 = 0.9;
/// Turns enough for any path of the flood (fortnightly turns).
const TURNS: u32 = 60;

fn flooded(c: &Arc<ContentBundle>, seed: u64) -> Engine {
    let mut e = engine(c, seed);
    plant_flood(&mut e, SEVERITY);
    e
}

#[test]
fn a_planted_flood_raises_the_arc_on_the_spine() {
    let c = base();
    let mut e = flooded(&c, SEED);
    until_raised(&mut e);
    let st = e.state();
    let town = home(&e);
    let r = run(&e).unwrap().clone();
    assert_eq!(r.template, "arc.flood");
    // Keyed by the jurisdiction it is in (DEC-098), at its first stage.
    assert_eq!(r.at, Jurisdiction::town(town));
    assert_eq!(r.stage, "struck");
    assert!(r.clock.is_some_and(|d| d > st.calendar.date));
    // The cast is real: a struck neighbourhood of the town, someone who
    // lives in the town, a place that stands there, the tier above.
    let hood = hood_of(&r.cast);
    assert_eq!(st.world.neighborhoods[hood.index()].municipality, town);
    assert!(
        damaged_pm(&e, hood) > 0,
        "the hood is one the water reached"
    );
    let organiser = person_of(&r.cast, "organiser").unwrap();
    assert_eq!(
        characters::municipality_of(&st.politics, &st.world, organiser),
        Some(town.0)
    );
    assert_ne!(organiser, me(&e));
    match r.actor("hall") {
        Some(ArcActor::Place { id }) => {
            let p = &st.world.places[id.index()];
            assert_eq!(p.municipality, town);
            assert!(p.is_open());
        }
        other => panic!("the hall is a place: {other:?}"),
    }
    let region = st.world.municipalities[town.index()].region;
    assert_eq!(
        r.actor("above"),
        Some(ArcActor::Jurisdiction {
            at: Jurisdiction::Region { id: region }
        })
    );
    // On the spine: the strike is the arc's root; the raise opens the
    // thread, the stage continues it, the story notes it.
    let log = &st.events;
    let strike = log
        .iter()
        .find(|ev| {
            matches!(&ev.kind, EventKind::Disaster { municipality, hazard: HazardKind::Flood, .. }
                if *municipality == town.0)
        })
        .unwrap();
    assert_eq!(r.root, Some(strike.id));
    assert_eq!(r.began, strike.date);
    let raised = log
        .iter()
        .find(|ev| matches!(&ev.kind, EventKind::ArcRaised { arc, .. } if *arc == r.id.0))
        .unwrap();
    assert_eq!(raised.causes, vec![Cause::Event(strike.id, Link::Caused)]);
    let stage = log
        .iter()
        .find(|ev| matches!(&ev.kind, EventKind::ArcStage { arc, stage, .. } if *arc == r.id.0 && stage == "struck"))
        .unwrap();
    assert!(
        stage
            .causes
            .contains(&Cause::Event(raised.id, Link::Continues))
    );
    // The stage's facts are numbers as they were: homes damaged, people away.
    if let EventKind::ArcStage { facts, .. } = &stage.kind {
        let get = |k: &str| {
            facts
                .iter()
                .find(|f| f.0 == k)
                .map(|f| f.1.parse::<u64>().unwrap())
        };
        assert!(get("homes").unwrap() > 0, "{facts:?}");
        assert!(get("away").unwrap() > 0, "{facts:?}");
        assert_eq!(get("repaired"), Some(0));
    }
    let press = log
        .iter()
        .find(|ev| matches!(&ev.kind, EventKind::ArcBeat { arc, kind, .. } if *arc == r.id.0 && kind == "press"))
        .unwrap();
    assert!(
        press.causes.iter().all(|c| c.is_known())
            && press
                .causes
                .iter()
                .any(|c| matches!(c, Cause::Event(id, Link::Continues) if *id == stage.id))
    );
    assert!(log.open_head(Thread::Arc(r.id.0)).is_some());
    // What the stage offers is there to take: the hall, and (in office)
    // the money; the ask comes from the organiser.
    let offers = beats::offers(st, &c);
    assert!(
        offers
            .iter()
            .any(|o| matches!(o.action, BeatAction::Visit { .. }) && o.beat.beat == "box")
    );
    let ask = st
        .asks
        .asks
        .iter()
        .find(|a| a.arc.as_ref().is_some_and(|x| x.run == r.id));
    if let Some(a) = ask {
        assert_eq!(a.asker_person(), Some(organiser));
        assert_eq!(a.template, "ask.flood_shelter");
    }
    if let Some(d) = st.narrative.pending().find(|d| d.arc.is_some()) {
        assert_eq!(d.template, "dilemma.flood_money");
        assert_eq!(
            mandate_sim::narrative::person_of(&d.cast, "organiser"),
            Some(organiser)
        );
        // Its opening was prompted by the run's stage.
        let opened = log
            .iter()
            .find(|ev| matches!(&ev.kind, EventKind::DecisionOpened { decision, .. } if *decision == d.id.0))
            .unwrap();
        assert!(
            opened
                .causes
                .iter()
                .any(|c| matches!(c, Cause::Event(_, Link::Prompted)))
        );
    }
    // The line is heard in the struck neighbourhood, and nowhere else.
    let heard = |h: NeighborhoodId| {
        mandate_sim::overheard::available(st, &c, Location::Street { neighborhood: h })
            .iter()
            .any(|l| l.id == "overheard.flood.skips")
    };
    assert!(heard(hood));
    let other = st.world.municipalities[town.index()]
        .neighborhoods
        .iter()
        .copied()
        .find(|h| *h != hood)
        .unwrap();
    assert!(!heard(other));
    // The state names no town: a jurisdiction, and typed ids.
    let json = serde_json::to_string(&st.arcs).unwrap();
    assert!(json.contains("\"at\":{\"type\":\"municipality\""));
    assert!(!json.contains("\"municipality\":"), "{json}");
}

/// The acceptance pair: one seed, one flood, two scripts.
#[test]
fn passive_and_engaged_scripts_reach_different_exits() {
    let c = base();
    let mut passive = flooded(&c, SEED);
    let mut engaged = flooded(&c, SEED);
    until_raised(&mut passive);
    until_raised(&mut engaged);
    assert_eq!(
        passive.state_hash(),
        engaged.state_hash(),
        "one world until someone acts"
    );
    let cast = run(&engaged).unwrap().cast.clone();
    let hood = hood_of(&cast);
    let organiser = person_of(&cast, "organiser").unwrap();
    let before = (damaged_pm(&engaged, hood), approval(&engaged, hood));
    // Turn by turn, side by side, until the passive run has ended.
    let mut script = Vec::new();
    for _ in 0..TURNS {
        if ended(&passive) {
            break;
        }
        PASSIVE.turn(&mut passive);
        script.extend(ENGAGED.turn(&mut engaged));
    }
    assert!(ended(&passive), "the passive run ends on time");
    // On the same day: what the engaged player's acts did to the town.
    assert_eq!(passive.state().calendar.date, engaged.state().calendar.date);
    let (p_left, e_left) = (damaged_pm(&passive, hood), damaged_pm(&engaged, hood));
    assert!(
        e_left < p_left && p_left < before.0,
        "homes repaired: {e_left} per mille still damaged against {p_left} (of {})",
        before.0
    );
    let (p_app, e_app) = (approval(&passive, hood), approval(&engaged, hood));
    assert!(
        e_app > p_app + 0.01,
        "approval in the struck neighbourhood: {e_app:.3} against {p_app:.3}"
    );
    assert!(opinion_of_me(&engaged, organiser) > opinion_of_me(&passive, organiser) + 10.0);
    assert!(remembers(&engaged, organiser, MemoryKind::StoodWithMe));
    assert!(remembers(&passive, organiser, MemoryKind::IgnoredMe));
    assert!(!remembers(&engaged, organiser, MemoryKind::IgnoredMe));
    // And where each ends.
    script.extend(ENGAGED.play(&mut engaged, TURNS));
    let (p, g) = (last_exit(&passive).unwrap(), last_exit(&engaged).unwrap());
    assert_eq!(p.exit, "hollowed");
    assert_eq!(g.exit, "rebuilt");
    assert!(g.marks.contains(&"paid_first".to_string()), "{:?}", g.marks);
    assert!(g.marks.contains(&"at_hall".to_string()), "{:?}", g.marks);
    assert!(p.marks.is_empty(), "{:?}", p.marks);
    // The engaged script is a list of commands like any other: given again
    // to the same world, it ends in the same state.
    assert!(script.iter().any(|c| matches!(c, Command::Player(_))));
    let mut again = flooded(&c, SEED);
    until_raised(&mut again);
    for cmd in &script {
        again.apply(cmd.clone()).unwrap();
    }
    assert_eq!(again.state_hash(), engaged.state_hash());
    // Every request the run made named its cause.
    let audit = engaged.state().events.effect_audit();
    assert_eq!(audit.unnamed, 0);
    // The exit closed the thread.
    assert!(
        engaged
            .state()
            .events
            .open_head(Thread::Arc(g.id.0))
            .is_none()
    );
}

/// DEC-051: every beat a player takes is taken with commands that exist,
/// in quick mode (one routed request) and through Places (walk there, then
/// act). Two scripts cover the flood's two halves: everything from the
/// first day, and nothing until the meeting.
#[test]
fn every_beat_is_reached_in_quick_mode_and_through_places() {
    let c = base();
    let def = c.arc("arc.flood").unwrap();
    let mut seen: Vec<BTreeSet<String>> = Vec::new();
    for places in [false, true] {
        let mut all = BTreeSet::new();
        for script in [ENGAGED, LATE] {
            let script = if places {
                script.through_places()
            } else {
                script
            };
            let mut e = flooded(&c, SEED);
            until_raised(&mut e);
            let list = script.play(&mut e, TURNS);
            assert!(ended(&e));
            if places {
                assert!(
                    list.iter()
                        .any(|x| matches!(x, Command::Player(PlayerCommand::MoveTo { .. })))
                        && !list
                            .iter()
                            .any(|x| matches!(x, Command::Player(PlayerCommand::Go { .. })))
                );
            }
            all.extend(beats_done(&e));
        }
        // Every beat a player takes was taken; every story ran and the
        // organiser made the move (on the late script's path).
        for (stage, beat) in def.beats() {
            let key = format!("{}.{}", stage.id, beat.id);
            let done = all.iter().any(|x| {
                x.starts_with(&format!("{key}:")) && (x.ends_with(":done") || !beat.kind.player())
            });
            match &beat.kind {
                // Heard lines hold while the stage does (checked where the
                // arc is raised); the wall's story needs the wall's law.
                BeatKind::Heard { .. } => {}
                BeatKind::Press { .. } if beat.id == "wall" => {}
                _ => assert!(
                    done,
                    "{key} ({}) was not reached: {all:#?}",
                    beat.kind.key()
                ),
            }
        }
        seen.push(all);
    }
    assert_eq!(
        seen[0], seen[1],
        "quick mode and Places reach the same beats"
    );
}

#[test]
fn with_no_arc_content_nothing_changes() {
    let c = base();
    let bare = without_arcs(&c);
    // No flood: the arc content is there and does nothing.
    let mut a = engine(&c, SEED);
    let mut b = engine(&bare, SEED);
    for _ in 0..12 {
        end_turn(&mut a);
        end_turn(&mut b);
    }
    assert_eq!(a.state_hash(), b.state_hash());
    let json = serde_json::to_value(a.state()).unwrap();
    assert!(json.get("arcs").is_none(), "an empty domain is not written");
    // A flood and no arc content: the flood happens, and that is all.
    let mut b = flooded(&bare, SEED);
    let mut a = flooded(&c, SEED);
    for _ in 0..12 {
        end_turn(&mut a);
        end_turn(&mut b);
    }
    assert!(b.state().arcs.is_empty());
    assert!(
        serde_json::to_value(b.state())
            .unwrap()
            .get("arcs")
            .is_none()
    );
    assert!(
        !b.state()
            .events
            .iter()
            .any(|e| mandate_sim::spine::kind_key(&e.kind).starts_with("arc_"))
    );
    assert!(run(&a).is_some());
    assert_ne!(a.state_hash(), b.state_hash());
}

/// The parts of the design the engine does not do yet (STAGED_ARCS §9): a
/// template that uses one is parsed and validated, and never raised. Which
/// they are: a `subject` cast and a clock read from it (the fuses, A5),
/// `gov_approval` and `place_state` (their owners' requests, A5), and any
/// `requires`.
#[test]
fn what_the_engine_does_not_do_yet_is_never_raised() {
    let c = base();
    let flood = c.arc("arc.flood").unwrap();
    assert!(
        mandate_sim::arcs::not_yet(flood).is_empty(),
        "the flood uses only what is built"
    );
    let mut b = (*c).clone();
    let exit = b.arcs[0]
        .stages
        .iter_mut()
        .find_map(|s| s.exit.as_mut())
        .unwrap();
    exit.effects.push(GatedEffect {
        when: Vec::new(),
        unless: Vec::new(),
        effect: ArcEffect::GovApproval {
            area: "hood".into(),
            delta: 0.03,
        },
    });
    let why = mandate_sim::arcs::not_yet(&b.arcs[0]);
    assert_eq!(why.len(), 1);
    assert!(why[0].contains("`gov_approval`"), "{why:?}");
    let mut e = flooded(&Arc::new(b), SEED);
    for _ in 0..10 {
        end_turn(&mut e);
    }
    assert!(e.state().arcs.is_empty(), "not yet: never raised");
}

#[test]
fn the_state_is_bounded() {
    let c = base();
    let mut e = flooded(&c, SEED);
    until_raised(&mut e);
    let def = c.arc("arc.flood").unwrap();
    let beats = def.beats().count();
    let mut biggest = 0;
    for _ in 0..TURNS {
        if ended(&e) {
            break;
        }
        ENGAGED.turn(&mut e);
        let a = &e.state().arcs;
        // One run a focus jurisdiction; its lists are bounded by its template.
        assert!(a.runs.len() <= 1);
        for r in &a.runs {
            assert!(r.done.len() <= beats && r.open.len() <= beats);
            assert!(r.marks.len() <= 8 && r.base.len() <= 2 * def.roles.len());
            assert!((0.0..=1.0).contains(&r.heat));
        }
        biggest = biggest.max(serde_json::to_vec(a).unwrap().len());
    }
    assert!(ended(&e));
    let a = &e.state().arcs;
    assert_eq!((a.runs.len(), a.past.len()), (0, 1));
    assert!(a.cooldowns.is_empty(), "the flood has no cooldown");
    println!(
        "arcs state: at most {biggest} bytes of JSON with one run; {} after it ended",
        serde_json::to_vec(a).unwrap().len()
    );
    assert!(biggest < 2500, "{biggest}");
    // The same strike raises nothing twice.
    for _ in 0..6 {
        end_turn(&mut e);
    }
    assert_eq!(
        (e.state().arcs.runs.len(), e.state().arcs.past.len()),
        (0, 1)
    );
}

/// Calibration: what a planted flood does in eight towns, and where each
/// script ends (run with `--ignored --nocapture`).
#[test]
#[ignore]
fn probe() {
    let c = base();
    for seed in [1u64, 2, 3, 7, 31, 99, 777, 2026] {
        let mut e = flooded(&c, seed);
        for _ in 0..8 {
            end_turn(&mut e);
            if run(&e).is_some() {
                break;
            }
        }
        let st = e.state();
        let hoods: Vec<String> = st
            .world
            .hoods_of(home(&e))
            .map(|h| {
                let id = NeighborhoodId(h as u32);
                format!(
                    "{}:pm{} away{} of {}",
                    h,
                    damaged_pm(&e, id),
                    st.population.displaced_from(id),
                    st.population.hood_total(h)
                )
            })
            .collect();
        println!(
            "seed {seed}: run {:?} | {}",
            run(&e).map(|r| (&r.stage, &r.base, r.cast.len())),
            hoods.join(" | ")
        );
        for (name, script) in [("passive", PASSIVE), ("engaged", ENGAGED), ("late", LATE)] {
            let mut p = flooded(&c, seed);
            for _ in 0..8 {
                end_turn(&mut p);
                if run(&p).is_some() {
                    break;
                }
            }
            let list = script.play(&mut p, TURNS);
            println!(
                "  {name}: {:?} after {} turns, marks {:?}, beats {:?}",
                last_exit(&p).map(|x| &x.exit),
                list.iter()
                    .filter(|x| matches!(x, Command::EndTurn))
                    .count(),
                last_exit(&p).map(|x| &x.marks),
                beats_done(&p)
            );
        }
    }
}
