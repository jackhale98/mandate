//! Mandate core primitives: typed IDs, calendar, money, deterministic math,
//! RNG streams, and stable hashing. No browser or platform dependencies.

pub mod hash;
pub mod ids;
pub mod math;
pub mod money;
pub mod rng;
pub mod serde_util;
pub mod time;

pub use hash::{StableHash, stable_hash_json};
pub use ids::IdAllocator;
pub use money::Money;
pub use rng::{RngStreams, SimRng, Stream};
pub use time::{Cadence, SimDate};

/// Engine version string, recorded in saves and determinism reports.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
