//! View models: read-only, per-screen projections of simulation state
//! (OVERVIEW §4.5). Text is never prose: events carry Fluent keys and args.

use mandate_sim::Engine;
use mandate_sim::events::{DomainEvent, EventKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DateView {
    pub day_number: u32,
    pub year: u32,
    pub month: u32,
    pub day: u32,
}

impl DateView {
    pub fn of(d: mandate_core::SimDate) -> Self {
        let c = d.calendar();
        DateView {
            day_number: d.0,
            year: c.year,
            month: c.month,
            day: c.day,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RegionView {
    pub id: u32,
    pub name: String,
    pub population: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EventView {
    pub id: u32,
    pub date: DateView,
    /// Fluent message key, e.g. `event-year-began`.
    pub text_key: String,
    pub args: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OverviewView {
    pub date: DateView,
    pub turn: u32,
    pub turn_days: u32,
    pub seed: String,
    pub content_hash: String,
    pub state_hash: String,
    pub total_population: f64,
    pub regions: Vec<RegionView>,
    pub recent_events: Vec<EventView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CommandAppliedView {
    pub days_advanced: u32,
    pub overview: OverviewView,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LocaleView {
    pub lang: String,
    pub ftl: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DiagnosticsView {
    pub engine_version: String,
    pub protocol_version: u32,
    pub systems: Vec<String>,
    pub event_count: u32,
    pub state_hash: String,
}

pub fn event_view(e: &DomainEvent, engine: &Engine) -> EventView {
    let mut args = BTreeMap::new();
    let key = match &e.kind {
        EventKind::GameStarted { seed } => {
            args.insert("seed".into(), seed.to_string());
            "event-game-started"
        }
        EventKind::YearBegan { year } => {
            args.insert("year".into(), year.to_string());
            "event-year-began"
        }
        EventKind::TurnEnded { turn } => {
            args.insert("turn".into(), turn.to_string());
            "event-turn-ended"
        }
        EventKind::RegionalShock {
            region,
            population_delta,
        } => {
            let name = engine
                .state()
                .world
                .regions
                .get(*region as usize)
                .map(|r| r.name.clone())
                .unwrap_or_default();
            args.insert("region".into(), name);
            args.insert("delta".into(), population_delta.to_string());
            "event-regional-shock"
        }
    };
    EventView {
        id: e.id.0,
        date: DateView::of(e.date),
        text_key: key.into(),
        args,
    }
}

pub fn overview(engine: &Engine) -> OverviewView {
    let s = engine.state();
    let recent_events = s
        .events
        .iter()
        .rev()
        .filter(|e| !matches!(e.kind, EventKind::TurnEnded { .. }))
        .take(12)
        .map(|e| event_view(e, engine))
        .collect();
    let hash = engine.state_hash();
    OverviewView {
        date: DateView::of(s.calendar.date),
        turn: s.calendar.turn,
        turn_days: engine.turn_days(),
        seed: s.meta.seed.to_string(),
        content_hash: s.meta.content_hash.chars().take(12).collect(),
        state_hash: hash.chars().take(16).collect(),
        total_population: s.world.total_population() as f64,
        regions: s
            .world
            .regions
            .iter()
            .map(|r| RegionView {
                id: r.id.0,
                name: r.name.clone(),
                population: s.world.region_population(r.id) as f64,
            })
            .collect(),
        recent_events,
    }
}

pub fn diagnostics(engine: &Engine) -> DiagnosticsView {
    DiagnosticsView {
        engine_version: mandate_core::ENGINE_VERSION.to_string(),
        protocol_version: crate::PROTOCOL_VERSION,
        systems: engine
            .system_names()
            .iter()
            .map(|s| s.to_string())
            .collect(),
        event_count: engine.state().events.len() as u32,
        state_hash: engine.state_hash(),
    }
}
