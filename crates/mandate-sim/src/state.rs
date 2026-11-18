//! Authoritative simulation state. Everything here is serialized into saves
//! (via `mandate-save` DTOs) and into the state hash.

use crate::economy::EconomyState;
use crate::environment::EnvironmentState;
use crate::events::EventLog;
use crate::finance::FinanceState;
use crate::population::PopulationState;
use crate::world::World;
use mandate_core::{RngStreams, SimDate};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldSize {
    Small,
    Medium,
    Large,
}

/// Canonical test-world presets (TEST_STRATEGY §3, DEC-058).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Preset {
    IndustrialDecline,
    WealthyCoast,
    TechGrowth,
    DroughtFarmland,
    ResourceFrontier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldParams {
    pub size: WorldSize,
    pub preset: Option<Preset>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldMeta {
    pub seed: u64,
    pub params: WorldParams,
    pub content_hash: String,
    pub engine_version: String,
    pub start_date: SimDate,
}

/// Broad game phase; selects turn length (DEC-012).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePhase {
    /// NPC-only history generation (DEC-022).
    BurnIn,
    Campaign,
    Governing,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Calendar {
    pub date: SimDate,
    pub turn: u32,
    pub phase: GamePhase,
    pub act: u8,
    pub chapter: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub meta: WorldMeta,
    pub calendar: Calendar,
    pub rng: RngStreams,
    pub world: World,
    pub events: EventLog,
    pub population: PopulationState,
    pub economy: EconomyState,
    pub finance: FinanceState,
    pub environment: EnvironmentState,
}

impl State {
    /// Canonical state hash (DEC-041 rule 7).
    pub fn hash(&self) -> String {
        mandate_core::stable_hash_json(self)
            .map(|h| h.hex())
            .unwrap_or_default()
    }
}
