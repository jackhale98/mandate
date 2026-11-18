// Simulation code indexes several parallel arrays by entity; range loops are clearer here.
#![allow(clippy::needless_range_loop)]

//! Mandate simulation: authoritative world state, domain systems, scheduler,
//! commands, and event log (DEC-042). Domains are modules; each system writes
//! only its own domain and emits effect requests for others (OVERVIEW §4.3).

pub mod actions;
pub mod commands;
pub mod economy;
pub mod engine;
pub mod environment;
pub mod events;
pub mod finance;
pub mod npc;
pub mod places;
pub mod player;
pub mod policy;
pub mod politics;
pub mod population;
pub mod scheduler;
pub mod state;
pub mod systems;
pub mod world;

pub use commands::{Command, CommandOutcome, EngineError};
pub use engine::Engine;
pub use state::State;
