//! Turn resolution order and cadence scheduling (OVERVIEW §4.2).

use crate::state::State;
use mandate_content::ContentBundle;
use mandate_core::{Cadence, SimDate};
use serde::{Deserialize, Serialize};

/// The 13 resolution phases, in order. Changing this order is a recorded
/// architecture change and requires a determinism test update.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Phase {
    Commands,
    NpcActions,
    Institutions,
    PolicyImplementation,
    Environment,
    Economy,
    Population,
    Information,
    Opinion,
    Elections,
    Characters,
    Director,
    Commit,
}

impl Phase {
    pub const ALL: [Phase; 13] = [
        Phase::Commands,
        Phase::NpcActions,
        Phase::Institutions,
        Phase::PolicyImplementation,
        Phase::Environment,
        Phase::Economy,
        Phase::Population,
        Phase::Information,
        Phase::Opinion,
        Phase::Elections,
        Phase::Characters,
        Phase::Director,
        Phase::Commit,
    ];
}

/// Cross-domain change request, applied by the owning system (OVERVIEW §4.3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// Population change for a region, applied by the population system.
    PopulationShock {
        region: u32,
        delta: i64,
        cause: Option<crate::events::EventId>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct EffectQueue {
    pending: Vec<Effect>,
}

impl EffectQueue {
    pub fn push(&mut self, e: Effect) {
        self.pending.push(e);
    }

    /// Remove and return effects matching a predicate, preserving order.
    pub fn take(&mut self, pred: impl Fn(&Effect) -> bool) -> Vec<Effect> {
        let (taken, keep): (Vec<_>, Vec<_>) = self.pending.drain(..).partition(|e| pred(e));
        self.pending = keep;
        taken
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// Context handed to a system for one run.
pub struct SystemCtx<'a> {
    pub state: &'a mut State,
    pub content: &'a ContentBundle,
    pub effects: &'a mut EffectQueue,
    pub date: SimDate,
}

/// A registered system: stateless function plus its phase and cadence.
#[derive(Clone, Copy)]
pub struct SystemDef {
    pub name: &'static str,
    pub phase: Phase,
    pub cadence: Cadence,
    pub run: fn(&mut SystemCtx<'_>),
}

/// Systems sorted by phase, keeping registration order within a phase.
pub struct Schedule {
    systems: Vec<SystemDef>,
}

impl Schedule {
    pub fn new(mut systems: Vec<SystemDef>) -> Self {
        systems.sort_by_key(|s| s.phase); // stable sort keeps registration order
        Schedule { systems }
    }

    /// Advance the calendar day by day, running each system whose cadence hits.
    pub fn advance(&self, state: &mut State, content: &ContentBundle, days: u32) {
        let mut effects = EffectQueue::default();
        for _ in 0..days {
            let date = state.calendar.date.add_days(1);
            state.calendar.date = date;
            for sys in &self.systems {
                if sys.cadence.hits(date) {
                    let mut ctx = SystemCtx {
                        state,
                        content,
                        effects: &mut effects,
                        date,
                    };
                    (sys.run)(&mut ctx);
                }
            }
        }
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.systems.iter().map(|s| s.name).collect()
    }
}
