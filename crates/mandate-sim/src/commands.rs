//! Commands mutate state and are logged for replay (OVERVIEW §4.5).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// End the current turn; the calendar advances by the phase's turn length.
    EndTurn,
    /// Lab/debug: advance an exact number of days.
    AdvanceDays { days: u32 },
    /// Lab only (never exposed to players): inject a sector demand shock.
    LabSectorShock {
        municipality: u32,
        sector: mandate_content::SectorKind,
        factor: f64,
        months: u16,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandOutcome {
    pub days_advanced: u32,
    pub turn: u32,
}

#[derive(Clone, Debug, PartialEq, thiserror::Error, Serialize, Deserialize)]
pub enum EngineError {
    #[error("invalid command: {0}")]
    InvalidCommand(String),
    #[error("content error: {0}")]
    Content(String),
}
