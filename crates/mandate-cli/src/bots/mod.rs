//! Bot players (M4, TEST_STRATEGY): scripted strategies play the player's
//! first local election through the same commands as the UI, across many
//! seeds. The gate ladder (ENGAGEMENT §7.2, DEC-097's balance principle)
//! compares them by class: single-verb routines < a blind mix < informed
//! play, with several informed styles that each win somewhere. Bots go where each
//! action is done before doing it, through the same auto-routing `Go`
//! command as the UI's agenda (DEC-079).
//!
//! Bots play as the player, so they know only what the player knows
//! (DEC-091): strategy code reads gated information (worries, support,
//! people, situational policies) only through [`Knowing`], never the true
//! state, and learns by the same commands. The blind-invariance test checks
//! it: perturbing truths a bot has not learned leaves its commands unchanged.

mod asks;
mod careers;
mod directed;
mod gates;
mod informed;
mod learn;
mod moves;
mod play;
mod report;
pub mod skips;
mod taste;
#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) use self::tests::perturb;
mod variety;

use mandate_content::{AiAction, ContentBundle};
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::characters::CharacterId;
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Knowing, Level, PolicyRef};
use mandate_sim::policy::{PolicyStatus, ward_hoods};
use mandate_sim::world::NeighborhoodId;
use mandate_sim::{Command, CommandOutcome, Engine, EngineError, npc};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The lab switch (`--omniscient`, DEC-091 §1.10): every bot's player
/// reads every subject as `detailed`, for baselines. Process-wide for a lab
/// run; worlds stay independent and deterministic.
static OMNISCIENT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_omniscient(on: bool) {
    OMNISCIENT.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn omniscient() -> bool {
    OMNISCIENT.load(std::sync::atomic::Ordering::Relaxed)
}

/// A bot's new game, with the lab switch applied.
fn new_game(content: &Arc<ContentBundle>, seed: u64) -> Option<Engine> {
    new_game_with(content, seed, omniscient())
}

/// A bot's new game, with the lab switch on or off for this game alone (the
/// `knowledge_matters` pass plays `mixed` omniscient beside the others).
fn new_game_with(content: &Arc<ContentBundle>, seed: u64, omniscient: bool) -> Option<Engine> {
    let mut e = generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        content.clone(),
    )
    .ok()?;
    e.set_lab_omniscient(omniscient);
    Some(e)
}

pub const STRATEGIES: [&str; 14] = [
    "idle",
    "canvass",
    "media",
    "asks_only",
    "legislator",
    "mixed",
    "proxy",
    "corrupt",
    "explorer",
    "explorer_coalition",
    "explorer_legislator",
    "explorer_campaigner",
    "explore_only",
    "directed",
];

/// The strategy classes the gate ladder compares (ENGAGEMENT §2.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// One verb, repeated: `canvass`, `media`, `asks_only`.
    SingleVerb,
    /// A mix played without reading the town: `mixed`, `legislator`, `proxy`.
    BlindMix,
    /// Learn the town first, then lean on one way of playing: `explorer`
    /// and its three variants.
    Informed,
    /// Compared only by their own gates: `idle`, `explore_only`, `corrupt`,
    /// `directed` (the time-skip proxy alone, by default directives).
    Probe,
}

impl Class {
    pub fn key(self) -> &'static str {
        match self {
            Class::SingleVerb => "single_verb",
            Class::BlindMix => "blind_mix",
            Class::Informed => "informed",
            Class::Probe => "probe",
        }
    }
}

pub fn class(s: &str) -> Class {
    match s {
        "canvass" | "media" | "asks_only" => Class::SingleVerb,
        "mixed" | "legislator" | "proxy" => Class::BlindMix,
        "explorer" | "explorer_coalition" | "explorer_legislator" | "explorer_campaigner" => {
            Class::Informed
        }
        _ => Class::Probe,
    }
}

/// Probes, not ways of playing: outside the class comparisons.
pub fn is_probe(s: &str) -> bool {
    class(s) == Class::Probe
}

/// Rote routines: single-verb routines and blind mixes together.
pub fn is_rote(s: &str) -> bool {
    matches!(class(s), Class::SingleVerb | Class::BlindMix)
}

/// The informed styles (each learns the town first, `EXPLORE_TURNS`).
pub fn is_informed(s: &str) -> bool {
    class(s) == Class::Informed
}

/// Turns `explorer` spends learning the town before it plays the mix.
pub const EXPLORE_TURNS: u32 = 10;

thread_local! {
    /// The commands bots send on this thread, while recording (the
    /// blind-invariance test).
    static RECORD: std::cell::RefCell<Option<Vec<String>>> = const { std::cell::RefCell::new(None) };
}

/// Start (or stop and return) recording the commands bots send on this
/// thread (the blind-invariance test; lab tools may record too).
#[cfg(test)]
pub fn record(on: bool) -> Vec<String> {
    RECORD.with(|r| {
        let mut r = r.borrow_mut();
        let out = r.take().unwrap_or_default();
        if on {
            *r = Some(Vec::new());
        }
        out
    })
}

thread_local! {
    /// The verbs bots used on this thread, counted while a game is played
    /// (the action mix the report prints).
    static MIX: std::cell::RefCell<BTreeMap<&'static str, u32>> =
        const { std::cell::RefCell::new(BTreeMap::new()) };
}

/// The verb a command counts as in the action mix (moves, meetings and
/// answers are not verbs).
fn verb(cmd: &PlayerCommand) -> Option<&'static str> {
    match cmd {
        PlayerCommand::Act { action, .. } => Some(action.key()),
        PlayerCommand::Go { then, .. } => then.as_deref().and_then(verb),
        PlayerCommand::Talk { topic, .. } => Some(match topic.as_str() {
            "topic.support" => "talk_support",
            "topic.pitch" | "topic.favour" => "talk_bill",
            _ => "talk",
        }),
        PlayerCommand::DraftPolicy { .. } => Some("draft"),
        PlayerCommand::IntroducePolicy { .. } => Some("introduce"),
        _ => None,
    }
}

/// Every command a bot sends goes through here.
fn apply(e: &mut Engine, cmd: Command) -> Result<CommandOutcome, EngineError> {
    RECORD.with(|r| {
        if let Some(v) = r.borrow_mut().as_mut() {
            v.push(serde_json::to_string(&cmd).unwrap_or_default());
        }
    });
    let v = match &cmd {
        Command::Player(p) => verb(p),
        _ => None,
    };
    let r = e.apply(cmd);
    if r.is_ok()
        && let Some(v) = v
    {
        MIX.with(|m| *m.borrow_mut().entry(v).or_default() += 1);
    }
    r
}

/// Take the action mix counted on this thread since the last take.
fn take_mix() -> BTreeMap<&'static str, u32> {
    MIX.with(|m| std::mem::take(&mut *m.borrow_mut()))
}

use self::asks::*;
pub use self::careers::*;
pub use self::directed::*;
pub use self::gates::*;
pub use self::informed::*;
use self::learn::*;
use self::moves::*;
pub use self::play::*;
pub use self::report::*;
pub use self::taste::*;
pub use self::variety::*;
