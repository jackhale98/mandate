//! The act flow (DEC-005, DEC-012, DEC-006): the Legacy record of each act,
//! the act's ending, and time skips with their summaries.
//!
//! This domain writes only `State::act`. Its turn hook (`after_turn`) runs
//! at the end of every turn, played or skipped, after the systems: it reads
//! what happened (the event log, posts, tracks, memories) and appends to the
//! Legacy record, then checks the endings registry. A skip (`crate::skip`)
//! plays turns through the same path, so a skipped turn and a played one
//! leave the same record.

use crate::legacy::LegacyRecord;
use crate::skip::{SkipRun, SkipSummary};
use mandate_content::ContentBundle;
use mandate_core::SimDate;
use serde::{Deserialize, Serialize};

/// How an act ended (the endings registry, DEC-006).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActEnding {
    /// The act (the constitution's `career.acts` id).
    pub act: String,
    /// The ending's content id.
    pub ending: String,
    pub date: SimDate,
}

/// The act domain.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ActState {
    /// The player's standing directives for time skips (DEC-012).
    pub directives: crate::skip::Directives,
    /// One record per act, the current one last. Closed records never change
    /// (append-only, DEC-100: a record only, no unlocks).
    pub records: Vec<LegacyRecord>,
    /// The ending of the current act, once one holds. In M6 there is no
    /// Act 2: the act's end is the run's; the world can still be simulated
    /// (lab tools), but the record is closed.
    pub ending: Option<ActEnding>,
    /// The event log entry looked at last (exclusive cursor).
    pub cursor: u32,
    /// The day the memories were read up to.
    pub scanned_to: Option<SimDate>,
    /// A skip in progress (between its chunks).
    pub skip: Option<SkipRun>,
    /// The last finished skip: the interrupt or homecoming summary (REQ-010).
    pub last_skip: Option<SkipSummary>,
}

impl ActState {
    /// The current act's record.
    pub fn current(&self) -> Option<&LegacyRecord> {
        self.records.last()
    }
    pub fn over(&self) -> bool {
        self.ending.is_some()
    }
}

/// End of turn (played or skipped): the Legacy record and the endings.
pub fn after_turn(state: &mut crate::State, content: &ContentBundle) {
    if state.player.is_none() || state.act.ending.is_some() {
        return;
    }
    if state.act.records.is_empty() {
        crate::legacy::begin(state, content);
    }
    crate::legacy::observe(state, content);
    if let Some(e) = crate::endings::check(state, content) {
        crate::legacy::close(state, content, e);
    }
}
