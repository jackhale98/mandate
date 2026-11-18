//! World generation (DEC-021, DEC-022). M0 provides a stub world so the full
//! engine, save, protocol, and browser path can be exercised; M1 implements the
//! real terrain → settlement → region pipeline.

pub mod names;

use mandate_content::ContentBundle;
use mandate_core::{ENGINE_VERSION, RngStreams, SimDate, Stream};
use mandate_sim::events::EventLog;
use mandate_sim::state::{Calendar, GamePhase, State, WorldMeta};
use mandate_sim::world::{Region, RegionId, World};
use mandate_sim::{Engine, EngineError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldSize {
    Small,
    Medium,
    Large,
}

impl WorldSize {
    pub fn region_count(self) -> usize {
        match self {
            WorldSize::Small => 6,
            WorldSize::Medium => 10,
            WorldSize::Large => 14,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldConfig {
    pub seed: u64,
    pub size: WorldSize,
}

/// Generate a new world and return an engine ready for play.
pub fn generate(config: &WorldConfig, content: Arc<ContentBundle>) -> Result<Engine, EngineError> {
    let mut rng = RngStreams::new(config.seed);
    let start_date = SimDate::from_ymd(content.tuning.start_year, 1, 1);
    let pool = content
        .name_pools
        .get("names.region")
        .ok_or_else(|| EngineError::Content("missing name pool names.region".into()))?;

    let wg = rng.get(Stream::Worldgen);
    let mut used = std::collections::BTreeSet::new();
    let mut regions = Vec::new();
    for i in 0..config.size.region_count() {
        let name = names::unique(pool, wg, &mut used);
        let population = (wg.range_f64(0.2, 3.0) * 1_000_000.0) as u64;
        regions.push(Region {
            id: RegionId(i as u32),
            name,
            population,
        });
    }

    let state = State {
        meta: WorldMeta {
            seed: config.seed,
            content_hash: content.content_hash.clone(),
            engine_version: ENGINE_VERSION.to_string(),
            start_date,
        },
        calendar: Calendar {
            date: start_date,
            turn: 0,
            phase: GamePhase::Governing,
            act: 1,
            chapter: 1,
        },
        rng,
        world: World { regions },
        events: EventLog::default(),
    };
    let mut engine = Engine::from_state(state, content)?;
    engine.mark_started();
    Ok(engine)
}
