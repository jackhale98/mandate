//! Authoritative simulation state. Everything here is serialized into saves
//! (via `mandate-save` DTOs) and into the state hash.

use crate::economy::EconomyState;
use crate::environment::EnvironmentState;
use crate::events::EventLog;
use crate::finance::FinanceState;
use crate::player::PlayerState;
use crate::policy::PolicyState;
use crate::politics::PoliticsState;
use crate::population::PopulationState;
use crate::world::{MunicipalityId, NeighborhoodId, World};
use mandate_core::{RngStreams, SimDate};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldSize {
    Small,
    Medium,
    Large,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldParams {
    pub size: WorldSize,
    /// Content id of the world's scenario (DEC-078 decision 4); `None` is a
    /// random country. Saves record it; content that removes it is refused.
    #[serde(default)]
    pub scenario: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldMeta {
    pub seed: u64,
    pub params: WorldParams,
    /// Content key of the constitutional archetype in force (DEC-007).
    pub constitution: String,
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
    pub politics: PoliticsState,
    pub policy: PolicyState,
    pub player: Option<PlayerState>,
    /// The director, decisions, storylines, and the facts log (DEC-033).
    #[serde(default)]
    pub narrative: crate::narrative::NarrativeState,
    /// Outlets, knowledge, coverage, scandals, rumours (DEC-034).
    #[serde(default)]
    pub media: crate::media::MediaState,
    /// Investigations, charges, trials, sentences (DEC-006).
    #[serde(default)]
    pub justice: crate::justice::JusticeState,
}

/// A town as play began (TOWN_GENERATION §4.1, §15 E1): what the layout's
/// static plan is sized from. Read-only presentation data assembled from the
/// baselines each domain stores (`HoodPop::baseline_population`,
/// `SectorState::baseline_jobs`, `Housing::baseline_stock`); whole numbers,
/// so floating-point drift never moves a town.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TownBaseline {
    pub municipality: MunicipalityId,
    /// Residents of all its neighbourhoods.
    pub population: u64,
    /// In the municipality's neighbourhood order.
    pub neighborhoods: Vec<HoodBaseline>,
    /// Jobs per sector, in `EconomyState::sector_keys` order (the sector's
    /// driver is in the content's sector registry).
    pub sectors: Vec<SectorBaseline>,
    /// Dwellings.
    pub housing_stock: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoodBaseline {
    pub id: NeighborhoodId,
    pub population: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectorBaseline {
    /// Sector content key.
    pub sector: String,
    pub jobs: u32,
}

impl State {
    /// Record the town baselines (population per neighbourhood, jobs per
    /// sector, housing stock) from the current state. Worldgen calls this
    /// once, as play begins after the history burn-in; saves from before
    /// baselines existed get them from their values when migrated. No system
    /// calls it, so the baselines never change during a run.
    pub fn record_baselines(&mut self) {
        crate::population::record_baselines(&mut self.population);
        crate::economy::record_baselines(&mut self.economy);
    }

    /// The baselines of one town, or `None` for an unknown municipality.
    pub fn town_baseline(&self, m: MunicipalityId) -> Option<TownBaseline> {
        let mu = self.world.municipalities.get(m.index())?;
        let neighborhoods: Vec<HoodBaseline> = mu
            .neighborhoods
            .iter()
            .map(|&id| HoodBaseline {
                id,
                population: self
                    .population
                    .hoods
                    .get(id.index())
                    .map_or(0, |h| h.baseline_population),
            })
            .collect();
        let econ = self.economy.munis.get(m.index());
        let sectors = self
            .economy
            .sector_keys
            .iter()
            .enumerate()
            .map(|(k, key)| SectorBaseline {
                sector: key.clone(),
                jobs: econ
                    .and_then(|e| e.sectors.get(k))
                    .map_or(0, |s| s.baseline_jobs),
            })
            .collect();
        Some(TownBaseline {
            municipality: m,
            population: neighborhoods.iter().map(|h| u64::from(h.population)).sum(),
            neighborhoods,
            sectors,
            housing_stock: econ.map_or(0, |e| e.housing.baseline_stock),
        })
    }

    /// Canonical state hash (DEC-041 rule 7).
    pub fn hash(&self) -> String {
        match mandate_core::stable_hash_json(self) {
            Ok(h) => h.hex(),
            Err(e) => {
                // Unserialisable state would also break saves: fail loudly in tests.
                debug_assert!(false, "state is not serialisable: {e}");
                String::new()
            }
        }
    }
}
