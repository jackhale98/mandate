//! World and space (DATA_MODEL §1). M0 holds a minimal region list; M1
//! replaces it with full generated geography.

use mandate_core::define_id;
use serde::{Deserialize, Serialize};

define_id!(
    /// A region (regional tier of government).
    RegionId
);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Region {
    pub id: RegionId,
    pub name: String,
    pub population: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct World {
    pub regions: Vec<Region>,
}

impl World {
    pub fn total_population(&self) -> u64 {
        self.regions.iter().map(|r| r.population).sum()
    }
}
