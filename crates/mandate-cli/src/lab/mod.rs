//! The lab (S2, IMPLEMENTATION_PLAN §2): content overrides (`--set`,
//! `--grid`), and the experiment runner shared by the audit and later tools
//! (the audition, the engagement and difficulty gates, quality-diversity
//! search).
//!
//! The runner takes a game generated from a seed (optionally a canonical
//! scenario), runs it month by month under a named [`Driver`], samples a
//! metric set ([`metrics::Snapshot`]) at chosen months and keeps a tally of
//! elections, events and crises. [`runner::twin`] runs the same game with
//! and without one [`Intervention`] from a shared prefix (a cloned state);
//! the engine is deterministic, so the difference is exact. Runs are
//! independent and execute in parallel threads ([`runner::par_map`]); the
//! output order is the input order.
//!
//! The lab writes nothing into the engine's rules: interventions are lab
//! commands the engine already has (`LabSectorShock`), state edits through
//! `Engine::setup` of the kind a lab command makes (a law enacted at once, a
//! plant closed, a hazard released, a recession begun), or content changes
//! (`--set`) loaded through the save path as a balance patch would be.

pub mod cli;
pub mod impact;
pub mod metrics;
pub mod overrides;
pub mod rate;
pub mod repetition;
pub mod runner;
pub mod seed_reqs;

#[cfg(test)]
mod arcs_games;
#[cfg(test)]
mod proof;
#[cfg(test)]
mod spine_games;
#[cfg(test)]
mod standin_games;

pub use overrides::*;
