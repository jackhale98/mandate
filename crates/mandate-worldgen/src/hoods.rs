//! Neighbourhoods within municipalities, with adjacency (worldgen step 10).

use mandate_core::SimRng;
use mandate_core::math::{pow, sqrt};
use mandate_sim::world::{NeighborhoodKind, SettlementClass};

pub struct HoodPlan {
    pub kind: NeighborhoodKind,
    pub population: u64,
}

pub fn count_for(pop: u64) -> usize {
    ((sqrt(pop as f64) / 50.0).round() as usize).clamp(1, 22)
}

/// Kinds and populations for one municipality's neighbourhoods.
pub fn plan(
    rng: &mut SimRng,
    pop: u64,
    class: SettlementClass,
    coastal: bool,
    industrial: bool,
) -> Vec<HoodPlan> {
    let n = count_for(pop);
    let mut kinds = vec![NeighborhoodKind::Core];
    if n > 1 && coastal {
        kinds.push(NeighborhoodKind::Waterfront);
    }
    if n > 2 && industrial {
        kinds.push(NeighborhoodKind::Industrial);
    }
    while kinds.len() < n {
        let k = match class {
            SettlementClass::Rural => NeighborhoodKind::Rural,
            SettlementClass::Town => {
                if kinds.len() % 3 == 0 {
                    NeighborhoodKind::Rural
                } else {
                    NeighborhoodKind::Suburb
                }
            }
            _ => {
                if kinds.len() < n / 2 {
                    NeighborhoodKind::Inner
                } else {
                    NeighborhoodKind::Suburb
                }
            }
        };
        kinds.push(k);
    }
    let weights: Vec<f64> = kinds
        .iter()
        .map(|k| {
            let base = match k {
                NeighborhoodKind::Core => 1.2,
                NeighborhoodKind::Inner => 1.3,
                NeighborhoodKind::Suburb => 1.1,
                NeighborhoodKind::Waterfront => 0.8,
                NeighborhoodKind::Industrial => 0.6,
                NeighborhoodKind::Rural => 0.5,
            };
            base * pow(rng.range_f64(0.7, 1.3), 1.0)
        })
        .collect();
    let total_w: f64 = weights.iter().sum();
    let mut out: Vec<HoodPlan> = kinds
        .into_iter()
        .zip(&weights)
        .map(|(kind, w)| HoodPlan {
            kind,
            population: ((pop as f64) * w / total_w) as u64,
        })
        .collect();
    // Assign rounding remainder to the core so totals match exactly.
    let assigned: u64 = out.iter().map(|h| h.population).sum();
    if let Some(core) = out.first_mut() {
        core.population += pop.saturating_sub(assigned);
    }
    out
}

/// Intra-municipality adjacency for `n` neighbourhoods: the core touches the
/// first ring; the rest form a ring around it.
pub fn local_adjacency(n: usize) -> Vec<Vec<usize>> {
    let mut adj = vec![Vec::new(); n];
    if n <= 1 {
        return adj;
    }
    let ring: Vec<usize> = (1..n).collect();
    for (k, &a) in ring.iter().enumerate() {
        let b = ring[(k + 1) % ring.len()];
        if a != b {
            adj[a].push(b);
            adj[b].push(a);
        }
        if k < 6 {
            adj[0].push(a);
            adj[a].push(0);
        }
    }
    for v in &mut adj {
        v.sort_unstable();
        v.dedup();
    }
    adj
}
