//! The shared arc-test harness (dev-only; no engine crate depends on it).
//!
//! An arc's tests used to build their own world from the whole base pack,
//! so every new arc in the pack walked into every other arc's tests: two
//! places for arcs a jurisdiction, one scene a chapter, a material arc that
//! leads (the pilot arc met twelve tests in seven files). Here:
//!
//! - [`only`]: the base pack with the arcs a test names and no other, so a
//!   world holds the arc under test alone unless the test asks for company
//!   ([`only`] with several ids, or [`keep`] on a pack of its own);
//! - [`world`]: a small world of a seed; [`raised`]: idle until the arc is
//!   live at the player's home;
//! - [`play`] and [`meet`], with a [`Policy`]: the levers a stand-in takes
//!   (`mandate_sim::standin`, X3) and how the scene is met: *away* (the
//!   empty chair), *cold* (turned up unprepared, the best line shown), *best
//!   shown* (prepared, the best line shown), or a list of line kinds. The
//!   best line shown is the cautious stand-in's (`standin::line`), chosen
//!   from the view the player has.
//!
//! What stays in a test of its own is the director's real interaction:
//! places for arcs, one scene a chapter, which arc leads
//! (`mandate-worldgen/tests/director_arcs.rs`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use mandate_content::exchange::MoveKind;
use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::actions::PlayerCommand;
use mandate_sim::arcs::exchange::{self, ExchangeAct, Phase};
use mandate_sim::arcs::{ArcId, ArcRun, ArcSummary};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::economy::plants;
use mandate_sim::environment::{DeferredHazard, HazardKind};
use mandate_sim::standin::{self, Seat};
use mandate_sim::world::{Jurisdiction, MunicipalityId};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

pub use mandate_sim::standin::{Kind, Play};

/// Turns [`raised`] waits for an arc before it gives up.
pub const RAISE_TURNS: u32 = 40;

// --- Worlds ------------------------------------------------------------------------------

/// The base pack, read from `content/` on disk.
pub fn pack() -> ContentBundle {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let src = read_dir_sources(&root).unwrap();
    load(&src).unwrap().0
}

/// A pack with the arcs named and no other (every other registry kept).
pub fn keep(mut b: ContentBundle, arcs: &[&str]) -> ContentBundle {
    b.arcs.retain(|a| arcs.contains(&a.id.as_str()));
    for id in arcs {
        assert!(b.arc(id).is_some(), "no arc {id} in the pack");
    }
    b
}

/// The base pack with the arcs named and no other: a world of it holds the
/// arc under test alone (or the company a test asks for).
pub fn only(arcs: &[&str]) -> Arc<ContentBundle> {
    Arc::new(keep(pack(), arcs))
}

/// The base pack with no arc at all.
pub fn no_arcs() -> Arc<ContentBundle> {
    let mut b = pack();
    b.arcs.clear();
    Arc::new(b)
}

/// A small world of a seed.
pub fn world(c: &Arc<ContentBundle>, seed: u64) -> Engine {
    world_sized(c, seed, WorldSize::Small)
}

pub fn world_sized(c: &Arc<ContentBundle>, seed: u64, size: WorldSize) -> Engine {
    generate(&WorldConfig::new(seed, size), c.clone()).unwrap()
}

// --- Reading a world ---------------------------------------------------------------------

/// The player's home town.
pub fn home(e: &Engine) -> MunicipalityId {
    mandate_sim::player::home(e.state()).unwrap()
}

/// The player's home as a jurisdiction (where their arcs run).
pub fn town(e: &Engine) -> Jurisdiction {
    Jurisdiction::town(home(e))
}

/// The player's own person.
pub fn me(e: &Engine) -> CharacterId {
    characters::player_character(e.state()).unwrap()
}

/// What a stand-in plays for: the player's seat, else their home.
pub fn seat(e: &Engine) -> Seat {
    Seat::of_player(e.state(), e.content()).unwrap_or(Seat {
        at: town(e),
        post: None,
    })
}

pub fn end_turn(e: &mut Engine) {
    e.apply(Command::EndTurn).unwrap();
}

/// The live run of a template.
pub fn run_of<'a>(e: &'a Engine, template: &str) -> Option<&'a ArcRun> {
    e.state().arcs.runs.iter().find(|r| r.template == template)
}

/// The latest ended run of a template.
pub fn summary_of<'a>(e: &'a Engine, template: &str) -> Option<&'a ArcSummary> {
    e.state()
        .arcs
        .past
        .iter()
        .rev()
        .find(|p| p.template == template)
}

/// The exit the latest ended run of a template took.
pub fn exit_of(e: &Engine, template: &str) -> Option<String> {
    summary_of(e, template).map(|p| p.exit.clone())
}

/// The exit a run took, once it ended.
pub fn exit_of_run(e: &Engine, id: ArcId) -> Option<String> {
    e.state()
        .arcs
        .past
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.exit.clone())
}

/// An occupation's content key, or empty.
pub fn occupation(e: &Engine, who: CharacterId) -> String {
    characters::profile(&e.state().politics, &e.state().world, who)
        .and_then(|p| p.occupation)
        .unwrap_or_default()
}

// --- Causes a test plants ----------------------------------------------------------------

/// A flood strikes the player's town at the next monthly hazard step (as
/// the lab plants one: a held hazard past its longest deferral).
pub fn plant_flood(e: &mut Engine, severity: f64) {
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

/// A fall in demand for the industry of the town's largest plant that can
/// close, held for `months` (as the lab plants a plant's review).
pub fn shock_plant(e: &mut Engine, factor: f64, months: u16) -> plants::PlantId {
    let town = home(e);
    let c = e.content().clone();
    let (id, industry) = plants::of_town(&e.state().economy, town)
        .filter(|p| p.is_open())
        .filter(|p| c.industry(&p.industry).is_some_and(|d| d.plants.closes))
        .max_by(|a, b| a.capacity.total_cmp(&b.capacity).then(b.id.cmp(&a.id)))
        .map(|p| (p.id, p.industry.clone()))
        .expect("a plant that can close");
    let def = c.industry(&industry).unwrap().clone();
    e.setup(|st| {
        st.economy.munis[town.index()]
            .shocks
            .push(mandate_sim::economy::ActiveShock {
                sector: def.sector.clone(),
                factor,
                months_left: months,
                industry: Some(def.id.clone()),
                demand: true,
            });
    });
    id
}

// --- Raised ------------------------------------------------------------------------------

/// End turns (nobody playing) until a run of the template is live, at most
/// `turns`: the run as raised.
pub fn raised_within(e: &mut Engine, template: &str, turns: u32) -> Option<ArcRun> {
    for _ in 0..=turns {
        if let Some(r) = run_of(e, template) {
            return Some(r.clone());
        }
        if e.apply(Command::EndTurn).is_err() {
            break;
        }
    }
    None
}

/// Idle until the arc is live ([`RAISE_TURNS`] at most; panics if not).
pub fn raised(e: &mut Engine, template: &str) -> ArcRun {
    raised_within(e, template, RAISE_TURNS)
        .unwrap_or_else(|| panic!("{template} was not raised in {RAISE_TURNS} turns"))
}

// --- Policies ----------------------------------------------------------------------------

/// How a scene is met.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    /// Never turns up: the other side speaks to an empty chair.
    Away,
    /// Each round, the best line shown: the cautious stand-in's pick from
    /// the player's view (`standin::line`, `Play::Cautious`).
    Best,
    /// Each round, the first line of these kinds on offer, else the safe
    /// line (`pass`), else the first listed.
    Lines(&'static [MoveKind]),
}

/// A player of an arc: the levers a stand-in of `levers` takes each turn
/// (`Kind::Passive`: none), and how the scene is met.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy {
    pub levers: Kind,
    pub scene: Scene,
}

impl Policy {
    /// Does nothing, and leaves the chair empty.
    pub const PASSIVE: Policy = Policy {
        levers: Kind::Passive,
        scene: Scene::Away,
    };
    /// Takes every lever, and leaves the chair empty (what the turn alone
    /// is worth, against [`Policy::BEST`]).
    pub const AWAY: Policy = Policy {
        levers: Kind::Engaged,
        scene: Scene::Away,
    };
    /// Takes no lever, and turns up for the scene with the best line shown.
    pub const COLD: Policy = Policy {
        levers: Kind::Passive,
        scene: Scene::Best,
    };
    /// Takes every lever, and plays the scene with the best line shown.
    pub const BEST: Policy = Policy {
        levers: Kind::Engaged,
        scene: Scene::Best,
    };

    pub const fn new(levers: Kind, scene: Scene) -> Policy {
        Policy { levers, scene }
    }

    /// The policy's name in a test's output.
    pub fn key(&self) -> String {
        let scene = match self.scene {
            Scene::Away => "away",
            Scene::Best => "best",
            Scene::Lines(_) => "lines",
        };
        format!("{}/{scene}", self.levers.key())
    }
}

/// A scene met: what was on offer and said, and how it ended.
#[derive(Clone, Debug, PartialEq)]
pub struct Met {
    pub run: ArcId,
    /// `<stage>.<beat>`.
    pub beat: String,
    /// The kinds of line on offer each round.
    pub offered: Vec<Vec<MoveKind>>,
    /// The kind said each round.
    pub said: Vec<MoveKind>,
    /// `won`, `split`, `lost` (`absent` is never met: see [`Played`]).
    pub outcome: String,
    /// Σ weight × change in lean, per mille.
    pub result_pm: i32,
}

impl Met {
    /// The kinds on offer in the first round.
    pub fn first(&self) -> &[MoveKind] {
        self.offered.first().map_or(&[][..], |v| v.as_slice())
    }
}

/// An arc played to its end (or to the turns given).
#[derive(Clone, Debug, PartialEq)]
pub struct Played {
    pub run: Option<ArcId>,
    /// The exit, if it ended.
    pub exit: Option<String>,
    /// The scenes the player met.
    pub met: Vec<Met>,
    /// Every scene the run finished, by outcome (`absent` included).
    pub outcomes: Vec<String>,
    pub turns: u32,
}

/// The scene of a template's run on offer now, if any: (run, beat, where).
fn open_scene(e: &Engine, template: &str) -> Option<(ArcId, String, mandate_sim::places::Location)> {
    exchange::view::open_scenes(e.state(), e.content())
        .into_iter()
        .find(|(run, _, _)| {
            e.state()
                .arcs
                .run(*run)
                .is_some_and(|r| r.template == template)
        })
        .map(|(run, beat, at)| (run, beat.key(), at))
}

/// The line a scene policy says this round.
fn line(v: &exchange::SceneView, scene: Scene) -> Option<String> {
    match scene {
        Scene::Away => None,
        Scene::Best => standin::line(v, Play::Cautious),
        Scene::Lines(kinds) => kinds
            .iter()
            .find_map(|k| v.options.iter().find(|o| o.kind == *k))
            .or_else(|| v.options.iter().find(|o| o.kind == MoveKind::Pass))
            .or_else(|| v.options.first())
            .map(|o| o.id.clone()),
    }
}

/// Meet the scene of a template's run if one is on offer now: turn up and
/// say a line each round by `scene`. `None`: nothing on offer, or the
/// policy stays away.
pub fn meet(e: &mut Engine, template: &str, scene: Scene) -> Option<Met> {
    if scene == Scene::Away {
        return None;
    }
    let (run, beat, location) = open_scene(e, template)?;
    e.apply(Command::Player(PlayerCommand::Go {
        location,
        then: Some(Box::new(PlayerCommand::Exchange {
            run: run.0,
            beat: beat.clone(),
            act: ExchangeAct::Begin,
        })),
    }))
    .ok()?;
    let (mut offered, mut said) = (Vec::new(), Vec::new());
    for _ in 0..16 {
        let Some(v) = exchange::view(e.state(), e.content(), run.0, &beat) else {
            break;
        };
        if v.phase != Phase::UnderWay {
            break;
        }
        offered.push(v.options.iter().map(|o| o.kind).collect::<Vec<_>>());
        let act = match line(&v, scene) {
            Some(option) => {
                said.push(v.options.iter().find(|o| o.id == option).unwrap().kind);
                ExchangeAct::Say { option }
            }
            None => ExchangeAct::Leave,
        };
        e.apply(Command::Player(PlayerCommand::Exchange {
            run: run.0,
            beat: beat.clone(),
            act,
        }))
        .unwrap();
    }
    let r = e
        .state()
        .arcs
        .run(run)
        .expect("a scene's run outlives its scene");
    let p = r
        .played
        .iter()
        .rev()
        .find(|p| p.beat.key() == beat)
        .expect("the scene ended");
    Some(Met {
        run,
        beat,
        offered,
        said,
        outcome: p.outcome.key().to_string(),
        result_pm: (p.result * 1000.0).round() as i32,
    })
}

/// Whether a player of `levers` begins a scene on offer this turn: the
/// stand-ins' rule (X3): one that prepares begins on the last turn that
/// can, so the weeks before are for preparing; one that does not, at once.
fn due(e: &Engine, template: &str, levers: Kind) -> bool {
    if matches!(levers, Kind::Passive | Kind::Attends) {
        return true;
    }
    let Some(last) = run_of(e, template).and_then(exchange::last_day) else {
        return true;
    };
    let days = characters::routines::turn_days(e.state(), e.content());
    e.state().calendar.date.add_days(days) > last
}

/// Play a template's run by a policy until it ends (at most `turns` turns
/// from now): each turn the scene first, when due (it takes the evening the
/// levers would), then the levers of the run's jurisdiction, then the turn
/// ends. A run not yet live is waited for; once it has ended, play stops.
pub fn play(e: &mut Engine, template: &str, policy: Policy, turns: u32) -> Played {
    let mut id = run_of(e, template).map(|r| r.id);
    let mut met = Vec::new();
    let mut n = 0;
    while n < turns {
        if let Some(r) = id
            && exit_of_run(e, r).is_some()
        {
            break;
        }
        if id.is_none() {
            id = run_of(e, template).map(|r| r.id);
        }
        if id.is_some() && due(e, template, policy.levers)
            && let Some(m) = meet(e, template, policy.scene)
        {
            met.push(m);
        }
        if policy.levers != Kind::Passive && run_of(e, template).is_some() {
            let s = Seat {
                at: run_of(e, template).map_or(town(e), |r| r.at),
                ..seat(e)
            };
            // Scenes are this harness's to meet: the stand-in plays none.
            standin::play_turn_styled(e, policy.levers, Play::Absent, s);
        }
        if e.apply(Command::EndTurn).is_err() {
            break;
        }
        n += 1;
        if id.is_none() {
            id = run_of(e, template).map(|r| r.id).or_else(|| {
                // Raised and ended inside one turn.
                summary_of(e, template).map(|p| p.id)
            });
        }
    }
    let outcomes = id
        .map(|r| {
            e.state()
                .events
                .iter()
                .filter_map(|ev| match &ev.kind {
                    mandate_sim::events::EventKind::ArcExchangeEnded { arc, outcome, .. }
                        if *arc == r.0 =>
                    {
                        Some(outcome.clone())
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    Played {
        run: id,
        exit: id.and_then(|r| exit_of_run(e, r)),
        met,
        outcomes,
        turns: n,
    }
}

/// Play until the run of a template is at `stage`, or has ended (at most
/// `turns` turns), by a policy. Returns whether it is at the stage.
pub fn until_stage(e: &mut Engine, template: &str, stage: &str, policy: Policy, turns: u32) -> bool {
    for _ in 0..turns {
        if run_of(e, template).is_some_and(|r| r.stage == stage) {
            return true;
        }
        if run_of(e, template).is_none() && exit_of(e, template).is_some() {
            return false;
        }
        if run_of(e, template).is_some() && due(e, template, policy.levers) {
            meet(e, template, policy.scene);
        }
        if policy.levers != Kind::Passive && run_of(e, template).is_some() {
            let s = seat(e);
            standin::play_turn_styled(e, policy.levers, Play::Absent, s);
        }
        if e.apply(Command::EndTurn).is_err() {
            return false;
        }
    }
    run_of(e, template).is_some_and(|r| r.stage == stage)
}

/// Each policy's exit in a fresh copy of a world (the same seed, the same
/// cause planted by `prepare`), the arc raised first: what a test of "the
/// turn changes the ending" compares.
pub fn exits(
    c: &Arc<ContentBundle>,
    seed: u64,
    template: &str,
    prepare: &dyn Fn(&mut Engine),
    policies: &[Policy],
    turns: u32,
) -> Vec<Played> {
    policies
        .iter()
        .map(|p| {
            let mut e = world(c, seed);
            prepare(&mut e);
            match raised_within(&mut e, template, RAISE_TURNS) {
                Some(_) => play(&mut e, template, *p, turns),
                None => Played {
                    run: None,
                    exit: None,
                    met: Vec::new(),
                    outcomes: Vec::new(),
                    turns: 0,
                },
            }
        })
        .collect()
}
