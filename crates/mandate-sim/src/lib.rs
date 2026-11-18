//! Mandate simulation: authoritative world state, domain systems, scheduler,
//! commands, and event log (DEC-042). Domains are modules; each system writes
//! only its own domain and emits effect requests for others (OVERVIEW §4.3).

pub mod commands;
pub mod engine;
pub mod events;
pub mod scheduler;
pub mod state;
pub mod systems;
pub mod world;

pub use commands::{Command, CommandOutcome, EngineError};
pub use engine::Engine;
pub use state::State;
