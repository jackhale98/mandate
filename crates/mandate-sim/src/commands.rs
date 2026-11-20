//! Commands mutate state and are logged for replay (OVERVIEW §4.5).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// End the current turn; the calendar advances by the phase's turn length.
    EndTurn,
    /// Lab/debug: advance an exact number of days.
    AdvanceDays { days: u32 },
    /// A player command (Commands phase).
    Player(crate::actions::PlayerCommand),
    /// The player has looked at their notebook (DEC-091, K5): what they
    /// learned is no longer new. Changes nothing else; no slot, no time.
    NotebookSeen,
    /// Set standing directives for time skips (DEC-012): (directive id,
    /// option id) pairs from `act_flow.jsonc`; others keep their choice.
    /// No slot, no time.
    SetDirectives { choices: Vec<(String, String)> },
    /// Skip ahead with the player-proxy (DEC-012) to a point the engine
    /// offers (`skip::targets`), at most `limit` turns now: a skip with
    /// turns left goes on with the next `Skip` to the same target (so a
    /// client can show progress and cancel between chunks). It stops at
    /// the target, an interrupt, or the act's end; the summary is
    /// `State::act.last_skip`.
    Skip {
        target: mandate_content::act::SkipTargetKind,
        #[serde(default)]
        limit: Option<u32>,
    },
    /// Cancel the skip in progress (a safe point: between turns).
    CancelSkip,
    /// Lab only (never exposed to players): inject a sector demand shock.
    LabSectorShock {
        municipality: u32,
        /// Content key of the sector, e.g. `sector.manufacturing`.
        sector: mandate_content::ContentKey,
        factor: f64,
        months: u16,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandOutcome {
    pub days_advanced: u32,
    pub turn: u32,
    pub result: Option<crate::actions::ActionResult>,
    /// A skip's progress (`Command::Skip`).
    #[serde(default)]
    pub skip: Option<crate::skip::SkipProgress>,
}

#[derive(Clone, Debug, PartialEq, thiserror::Error, Serialize, Deserialize)]
pub enum EngineError {
    #[error("invalid command: {0}")]
    InvalidCommand(String),
    #[error("content error: {0}")]
    Content(String),
}
