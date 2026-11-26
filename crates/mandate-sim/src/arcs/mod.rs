//! Staged arcs (DEC-109; `docs/design/STAGED_ARCS.md`): the `arcs` domain.
//!
//! An arc is one matter in one jurisdiction: a cast bound to real people
//! and places, a current stage, a clock, a heat and marks. A template
//! (`mandate_content::arcs::ArcDef`) says how it unfolds; a run
//! ([`ArcRun`]) is one telling.
//!
//! **Transitions read; beats write.** The step ([`step`]) reads simulated
//! state through `Reads` and moves a run along its stage's edges; what a
//! beat or an exit changes outside this domain goes out as effect requests
//! the owning systems apply (OVERVIEW §4.3), each with its cause on the
//! spine (`Thread::Arc`). Nothing a player picks names a stage or an exit:
//! a visit, a reply, an answer or a kept ask sets marks and posts
//! requests, and the edges read the result.
//!
//! **Where arcs run.** In the player's focus jurisdictions, keyed by
//! `Jurisdiction` (DEC-098). A run is raised as soon as its template's
//! `raise` holds and its cast can be bound: one run a template in a
//! jurisdiction, at most `director.arcs_active` there at once, standing
//! arcs (the budget round, the rival) aside. The director never raises,
//! blocks or ends a run (DEC-033): it chooses which leads the chapter,
//! places the clocks that are its own (`ClockDef::placed`) and presents
//! the decisions runs ask for (`ArcRun::open`), within its budgets.
//!
//! **Saved, bounded, deterministic.** [`ArcsState`] holds the live runs
//! (a few per focus jurisdiction), their cooldowns and the last
//! [`PAST_CAP`] summaries. A run's `done` and `marks` are bounded by its
//! template. Every draw is keyed (`keyed(seed, Misc, [tag, …])`: clocks
//! and moves); none depends on what the player knows. An empty state is
//! not written to a save, so a game no arc has touched hashes as it did
//! before arcs existed.
//!
//! - [`cast`]: roles bound to people, areas, places and jurisdictions.
//! - [`step`]: raising, heat, moves, press, offers, edges and exits.
//! - [`beats`]: the beats a player takes (a visit, a choice in a
//!   conversation), and what is offered where.
//! - [`effects`]: an arc's effects as effect requests.
//! - [`exchange`]: the eighth beat kind (DEC-112): a played scene of a few
//!   rounds, its state on the run.
//! - [`score`]: the five terms and coverage a played run is measured by
//!   (the grader, and the audition).
//! - [`audition`]: the openings worth offering (DEC-101 §5): candidates,
//!   forks played by the stand-ins, the score and the draft.

pub mod audition;
pub mod beats;
pub mod cast;
pub mod effects;
pub mod exchange;
pub mod score;
pub mod step;

use crate::characters::CharacterId;
use crate::events::EventId;
use crate::world::{Area, Jurisdiction, PlaceId};
use mandate_content::ContentKey;
use mandate_content::arcs::{ArcBeat, ArcDef, StageDef};
use mandate_core::{IdAllocator, SimDate, define_id};
use serde::{Deserialize, Serialize};

pub use step::{daily, weekly};

define_id!(
    /// An arc run; typed `u32`, never reused (DEC-041).
    ArcId
);

/// Keyed-draw tags (`Stream::Misc`; never reused): a stage's clock, a
/// cast person's move.
pub(crate) const KEY_CLOCK: u64 = 0xA2C5_0001;
pub(crate) const KEY_MOVE: u64 = 0xA2C5_0002;

/// Ended runs kept as summaries.
pub const PAST_CAP: usize = 40;

/// The exit recorded for a run whose template (or stage) an update removed.
pub const EXIT_REMOVED: &str = "removed";

/// What a role is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ArcActor {
    Person { id: CharacterId },
    Area { area: Area },
    Place { id: PlaceId },
    Jurisdiction { at: Jurisdiction },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcCastEntry {
    pub role: String,
    pub actor: ArcActor,
}

/// A beat of a run, by stage and beat id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeatRef {
    pub stage: String,
    pub beat: String,
}

impl BeatRef {
    pub fn new(stage: &str, beat: &str) -> Self {
        BeatRef {
            stage: stage.into(),
            beat: beat.into(),
        }
    }

    /// `<stage>.<beat>`.
    pub fn key(&self) -> String {
        format!("{}.{}", self.stage, self.beat)
    }
}

/// The run and beat a decision or an ask belongs to (`Decision::arc`,
/// `Ask::arc`): how the run finds what its beat opened.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcRef {
    pub run: ArcId,
    pub beat: BeatRef,
}

/// A beat done in a run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeatDone {
    pub beat: BeatRef,
    pub on: SimDate,
}

/// The turn a run reached: its first stage marked `climax`, and the day.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcTurn {
    pub stage: String,
    pub on: SimDate,
}

/// One telling of an arc.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArcRun {
    pub id: ArcId,
    pub template: ContentKey,
    pub at: Jurisdiction,
    pub cast: Vec<ArcCastEntry>,
    /// The current stage's id.
    pub stage: String,
    /// When the current stage was entered.
    pub since: SimDate,
    /// When the run was raised.
    pub started: SimDate,
    /// When the matter began: its root event's day (the strike), else the
    /// day it was raised.
    pub began: SimDate,
    /// The deadline in force.
    pub clock: Option<SimDate>,
    /// 0..1: escalation and relief. It changes no material outcome itself.
    pub heat: f32,
    /// Marks set by beats, sorted.
    pub marks: Vec<String>,
    /// Beats done (at most one entry a beat).
    pub done: Vec<BeatDone>,
    /// Decision and ask beats asked of their owners and not yet settled.
    pub open: Vec<BeatRef>,
    /// Readings taken as the run was raised (`<role>.<reading>`): what a
    /// later reading is measured against.
    pub base: Vec<(String, u64)>,
    /// The event that made its `raise` true (the root on the spine).
    pub root: Option<EventId>,
    /// The turn it has reached, if it has (the first `climax` stage).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<ArcTurn>,
    /// The turn the last run of this template here reached (`arc_last_turn`:
    /// the same turn does not come twice running).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    /// An exchange scene begun and not finished (DEC-112). It never spans
    /// a turn. Not written when there is none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange: Option<exchange::ArcExchangeState>,
    /// The scenes this run has finished, oldest first: the beat, the
    /// outcome, the groups before and after, the rounds. Not written when
    /// empty, so a run no scene has touched serializes as it did.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub played: Vec<exchange::ExchangePlayed>,
}

impl ArcRun {
    pub fn has_mark(&self, mark: &str) -> bool {
        self.marks.iter().any(|m| m == mark)
    }

    pub(crate) fn set_mark(&mut self, mark: &str) {
        if let Err(i) = self.marks.binary_search_by(|m| m.as_str().cmp(mark)) {
            self.marks.insert(i, mark.to_string());
        }
    }

    pub fn actor(&self, role: &str) -> Option<ArcActor> {
        self.cast.iter().find(|c| c.role == role).map(|c| c.actor)
    }

    pub fn person(&self, role: &str) -> Option<CharacterId> {
        match self.actor(role)? {
            ArcActor::Person { id } => Some(id),
            _ => None,
        }
    }

    pub fn area(&self, role: &str) -> Option<Area> {
        match self.actor(role)? {
            ArcActor::Area { area } => Some(area),
            _ => None,
        }
    }

    pub fn is_done(&self, b: &BeatRef) -> bool {
        self.done.iter().any(|d| &d.beat == b)
    }

    pub(crate) fn mark_done(&mut self, b: BeatRef, on: SimDate) {
        self.open.retain(|x| x != &b);
        match self.done.iter_mut().find(|d| d.beat == b) {
            Some(d) => d.on = on,
            None => self.done.push(BeatDone { beat: b, on }),
        }
    }

    pub fn base(&self, key: &str) -> Option<u64> {
        self.base.iter().find(|(k, _)| k == key).map(|(_, v)| *v)
    }

    /// Days in the current stage.
    pub fn days_in_stage(&self, today: SimDate) -> u32 {
        if today >= self.since {
            self.since.days_until(today)
        } else {
            0
        }
    }

    pub(crate) fn add_heat(&mut self, delta: f32) {
        self.heat = (self.heat + delta).clamp(0.0, 1.0);
    }

    /// The template's current stage.
    pub fn stage_def<'c>(&self, def: &'c ArcDef) -> Option<&'c StageDef> {
        def.stage(&self.stage)
    }

    /// The cast as the narrative and the asks know casts (people, and
    /// areas that are one neighbourhood): the slots of conditions, and the
    /// cast of a decision or an ask the run opens.
    pub fn narrative_cast(&self) -> Vec<crate::narrative::CastEntry> {
        use crate::narrative::{Actor, CastEntry};
        self.cast
            .iter()
            .filter_map(|c| {
                let actor = match c.actor {
                    ArcActor::Person { id } => Actor::Person { id },
                    ArcActor::Area {
                        area: Area::Hood { id },
                    } => Actor::Neighborhood { id: id.0 },
                    _ => return None,
                };
                Some(CastEntry {
                    role: c.role.clone(),
                    actor,
                })
            })
            .collect()
    }
}

/// A run that ended: what the spine's events of it are read with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArcSummary {
    pub id: ArcId,
    pub template: ContentKey,
    pub at: Jurisdiction,
    /// The exit stage's id (`removed` when an update removed its template).
    pub exit: String,
    pub started: SimDate,
    pub began: SimDate,
    pub ended: SimDate,
    pub cast: Vec<ArcCastEntry>,
    pub marks: Vec<String>,
    pub root: Option<EventId>,
    /// The turn it reached, if it did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<ArcTurn>,
}

/// A template may not be raised again at a jurisdiction before a day.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcCooldown {
    pub template: ContentKey,
    pub at: Jurisdiction,
    pub until: SimDate,
}

/// The arcs domain's state (`State.arcs`; save v27).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ArcsState {
    /// Live runs, in id order.
    pub runs: Vec<ArcRun>,
    pub ids: IdAllocator,
    /// Sorted by (template, jurisdiction); gone once they pass.
    pub cooldowns: Vec<ArcCooldown>,
    /// Ended runs, oldest first (the last [`PAST_CAP`]).
    pub past: Vec<ArcSummary>,
    /// Requests whose applier runs monthly (people leaving a town): kept
    /// here until the month's first day, so a save between loses nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deferred: Vec<crate::scheduler::Request>,
}

impl ArcsState {
    /// Nothing has ever run: not written to a save (a game no arc has
    /// touched hashes as it did before arcs existed).
    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
            && self.past.is_empty()
            && self.cooldowns.is_empty()
            && self.deferred.is_empty()
            && self.ids.peek_next() == 0
    }

    pub fn run(&self, id: ArcId) -> Option<&ArcRun> {
        self.runs.iter().find(|r| r.id == id)
    }

    pub fn run_mut(&mut self, id: ArcId) -> Option<&mut ArcRun> {
        self.runs.iter_mut().find(|r| r.id == id)
    }

    /// The live runs at a jurisdiction, in id order.
    pub fn at(&self, at: Jurisdiction) -> impl Iterator<Item = &ArcRun> {
        self.runs.iter().filter(move |r| r.at == at)
    }

    /// The turn the last ended run of a template at a jurisdiction reached
    /// (the latest that reached one).
    pub fn last_turn(&self, template: &str, at: Jurisdiction) -> Option<&str> {
        self.past
            .iter()
            .rev()
            .filter(|p| p.template == template && p.at == at)
            .find_map(|p| p.turn.as_ref().map(|t| t.stage.as_str()))
    }

    /// The cast of a run, live or ended (for the text of its events).
    pub fn cast_of(&self, id: ArcId) -> Option<&[ArcCastEntry]> {
        self.run(id).map(|r| r.cast.as_slice()).or_else(|| {
            self.past
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.cast.as_slice())
        })
    }

    pub(crate) fn retire(&mut self, run: ArcRun, exit: &str, on: SimDate) {
        self.past.push(ArcSummary {
            id: run.id,
            template: run.template,
            at: run.at,
            exit: exit.to_string(),
            started: run.started,
            began: run.began,
            ended: on,
            cast: run.cast,
            marks: run.marks,
            root: run.root,
            turn: run.turn,
        });
        if self.past.len() > PAST_CAP {
            let drop = self.past.len() - PAST_CAP;
            self.past.drain(..drop);
        }
    }
}

/// A template's beat by reference.
pub fn beat_of<'c>(def: &'c ArcDef, b: &BeatRef) -> Option<(&'c StageDef, &'c ArcBeat)> {
    let s = def.stage(&b.stage)?;
    Some((s, s.beat(&b.beat)?))
}

/// Whether a stage marked `climax` can still be reached from a stage by
/// the template's edges (the stage itself not counted): a run that has not
/// turned and can no longer turn gives its chapter none.
pub fn turn_ahead(def: &ArcDef, stage: &str) -> bool {
    let mut seen: Vec<&str> = vec![stage];
    let mut stack: Vec<&str> = vec![stage];
    while let Some(id) = stack.pop() {
        let Some(s) = def.stage(id) else { continue };
        for e in &s.next {
            if seen.contains(&e.to.as_str()) {
                continue;
            }
            if def.stage(&e.to).is_some_and(|t| t.climax) {
                return true;
            }
            seen.push(&e.to);
            stack.push(&e.to);
        }
    }
    false
}

/// What the engine does not do yet for a template, if anything: such a
/// template is parsed and validated, and never raised.
pub fn not_yet(def: &ArcDef) -> Vec<String> {
    mandate_content::arcs::not_yet(def)
}
