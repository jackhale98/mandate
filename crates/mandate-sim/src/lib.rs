// Simulation code indexes several parallel arrays by entity; range loops are clearer here.
#![allow(clippy::needless_range_loop)]

//! Mandate simulation: authoritative world state, domain systems, scheduler,
//! commands, and event log (DEC-042). Domains are modules; each system writes
//! only its own domain and emits effect requests for others (OVERVIEW §4.3).

pub mod act;
pub mod actions;
pub mod asks;
pub mod characters;
pub mod commands;
pub mod concerns;
pub mod condition;
pub mod economy;
pub mod endings;
pub mod engine;
pub mod environment;
pub mod events;
#[cfg(feature = "explain")]
pub mod explain;
pub mod finance;
pub mod justice;
pub mod legacy;
pub mod media;
pub mod narrative;
pub mod npc;
pub mod overheard;
pub mod places;
pub mod player;
pub mod policy;
pub mod politics;
pub mod population;
pub mod proxy;
pub mod scheduler;
pub mod skip;
pub mod state;
pub mod systems;
pub mod talk;
pub mod visits;
pub mod world;

pub use commands::{Command, CommandOutcome, EngineError};
pub use engine::Engine;
pub use state::State;
