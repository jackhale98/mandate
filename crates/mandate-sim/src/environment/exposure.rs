//! Per-neighbourhood hazard exposure, damage and recovery (DEC-090 D8).
//!
//! The rolls stay per town (`environment::hazards`): how often a town floods
//! and how much of it a strike destroys are unchanged. Exposure decides where
//! the damage falls inside the town. Each neighbourhood has a fixed exposure
//! per hazard (per mille), derived as the world is generated: the terrain's
//! floodplains, coasts and forest edges in `mandate-worldgen`, or, for a save
//! from before exposure existed, the town's stored geographic profile and
//! the neighbourhood's kind (`from_profile`). A strike shares the town's
//! damage by exposure × residents, so the flood-prone district takes most of
//! a flood while the town total stays what it was. Protections (flood
//! defences and the like) scale each neighbourhood's share.
//!
//! The environment keeps the footprint (`HoodDamage`: homes damaged and not
//! yet repaired, by neighbourhood). The town's housing stock changes through
//! the economy (`Effect::HousingDamage`), residents through the population
//! (`Effect::HoodDisaster`: who leaves, who stays elsewhere for a while, and
//! how hard the neighbourhood was hit relative to its town).

use super::HazardKind;
use crate::world::{NeighborhoodId, NeighborhoodKind, World};
use mandate_content::HazardParams;
use mandate_core::SimDate;
use serde::{Deserialize, Serialize};

/// A neighbourhood's exposure to the hazards that strike unevenly within a
/// town, per mille (0 = none, 1000 = the most exposed land). Droughts and
/// earthquakes strike a town evenly and have no entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoodExposure {
    /// Floodplain: low land by a river, lake or the sea.
    pub flood: u16,
    /// Fuel and forest edge.
    pub wildfire: u16,
    /// Coastal wind and surge.
    pub storm: u16,
}

impl HoodExposure {
    /// The exposure to one hazard, or `None` for a hazard that strikes a
    /// town evenly (drought, earthquake).
    pub fn of(&self, kind: HazardKind) -> Option<u16> {
        match kind {
            HazardKind::Flood => Some(self.flood),
            HazardKind::Wildfire => Some(self.wildfire),
            HazardKind::Storm => Some(self.storm),
            HazardKind::Drought | HazardKind::Earthquake => None,
        }
    }
}

/// Homes in a neighbourhood damaged by a hazard and not yet repaired.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoodDamage {
    pub neighborhood: u32,
    /// The latest hazard that damaged it.
    pub hazard: HazardKind,
    /// When that hazard struck.
    pub struck: SimDate,
    /// Share of the neighbourhood's homes damaged and not yet repaired, per
    /// mille (repaired at `economy.hazards.repair_rate` a month).
    pub damaged_pm: u16,
}

/// A value in [0, 1] as a whole per mille.
pub fn per_mille(x: f64) -> u16 {
    (x.clamp(0.0, 1.0) * 1000.0).round() as u16
}

/// Exposure from what a save holds (no terrain): the town's hazard index
/// (its `GeoProfile`) × how exposed a neighbourhood of its kind tends to be.
/// Used by the save migration for runs that began before exposure existed;
/// new games derive it from the terrain (`mandate-worldgen`). Deterministic,
/// draws nothing.
pub fn from_profile(world: &World) -> Vec<HoodExposure> {
    world
        .neighborhoods
        .iter()
        .map(|n| {
            let Some(mu) = world.municipalities.get(n.municipality.index()) else {
                return HoodExposure::default();
            };
            let g = &mu.geo;
            use NeighborhoodKind as K;
            // By water, then low and central, then higher ground.
            let flood = match n.kind {
                K::Waterfront => 1.8,
                K::Industrial => 1.4,
                K::Core => 1.15,
                K::Inner => 0.9,
                K::Rural => 0.7,
                K::Suburb => 0.55,
            };
            // The edge of the woods and the open country burn.
            let wildfire = match n.kind {
                K::Rural => 1.6,
                K::Suburb => 1.3,
                K::Industrial => 0.8,
                K::Inner => 0.6,
                K::Waterfront => 0.5,
                K::Core => 0.45,
            };
            // On a coast the shore takes the surge; inland, wind is even.
            let storm = if mu.coastal {
                match n.kind {
                    K::Waterfront => 1.8,
                    K::Industrial => 1.1,
                    K::Core => 1.0,
                    K::Inner => 0.9,
                    K::Rural => 0.8,
                    K::Suburb => 0.7,
                }
            } else {
                1.0
            };
            HoodExposure {
                flood: per_mille(g.flood * flood),
                wildfire: per_mille(g.wildfire * wildfire),
                storm: per_mille(g.storm * storm),
            }
        })
        .collect()
}

/// One neighbourhood's share of a strike.
#[derive(Clone, Debug, PartialEq)]
pub struct HoodHit {
    pub neighborhood: u32,
    /// Its share of the town's residents.
    pub share: f64,
    /// Its hit relative to the town's average (1 = average).
    pub relative: f64,
    /// Share of its homes destroyed.
    pub damage: f64,
}

/// Share a strike's town-wide damage among a town's neighbourhoods by
/// exposure × residents. `damage` is the town's damage before protection;
/// `protection(hood)` is the share of it a protection removes there (today
/// a town's policies, the same in every neighbourhood). The residents'
/// weighted mean of `relative` is 1, so the town total matches an even
/// split; a neighbourhood's `relative` is capped at `max_relative` (the
/// excess goes to the others) and its damage at `max_hood_damage`. A town
/// whose neighbourhoods have no exposure to the hazard (or a hazard that
/// strikes evenly) splits evenly.
#[allow(clippy::too_many_arguments)]
pub fn split(
    world: &World,
    exposure: &[HoodExposure],
    residents: &dyn Fn(usize) -> u64,
    municipality: u32,
    kind: HazardKind,
    damage: f64,
    protection: &dyn Fn(u32) -> f64,
    params: &HazardParams,
) -> Vec<HoodHit> {
    let Some(mu) = world.municipalities.get(municipality as usize) else {
        return Vec::new();
    };
    let hoods = &mu.neighborhoods;
    let n = hoods.len().max(1) as f64;
    let total: u64 = hoods.iter().map(|h| residents(h.index())).sum();
    let share: Vec<f64> = hoods
        .iter()
        .map(|h| {
            if total > 0 {
                residents(h.index()) as f64 / total as f64
            } else {
                1.0 / n
            }
        })
        .collect();
    let exp: Option<Vec<f64>> = hoods
        .iter()
        .map(|h| {
            exposure
                .get(h.index())
                .and_then(|e| e.of(kind))
                .map(f64::from)
        })
        .collect();
    let rel = relative(&share, exp.as_deref(), params.max_relative.max(1.0));
    hoods
        .iter()
        .zip(share)
        .zip(rel)
        .map(|((h, share), relative)| {
            let p = protection(h.0).clamp(0.0, 1.0);
            HoodHit {
                neighborhood: h.0,
                share,
                relative,
                damage: (damage * relative * (1.0 - p))
                    .clamp(0.0, params.max_hood_damage.clamp(0.0, 1.0)),
            }
        })
        .collect()
}

/// Relative hits with a weighted mean of 1, each at most `cap`; the excess
/// of capped neighbourhoods is spread over the rest in proportion.
fn relative(share: &[f64], exposure: Option<&[f64]>, cap: f64) -> Vec<f64> {
    let even = vec![1.0; share.len()];
    let Some(exp) = exposure else {
        return even;
    };
    let mean: f64 = share.iter().zip(exp).map(|(s, e)| s * e).sum();
    if mean <= 0.0 {
        return even;
    }
    let mut rel: Vec<f64> = exp.iter().map(|e| (e / mean).min(cap)).collect();
    for _ in 0..4 {
        let sum: f64 = share.iter().zip(&rel).map(|(s, r)| s * r).sum();
        let free: f64 = share
            .iter()
            .zip(&rel)
            .filter(|(_, r)| **r < cap)
            .map(|(s, r)| s * r)
            .sum();
        if sum >= 1.0 - 1e-9 || free <= 0.0 {
            break;
        }
        let k = 1.0 + (1.0 - sum) / free;
        for r in rel.iter_mut() {
            if *r < cap {
                *r = (*r * k).min(cap);
            }
        }
    }
    rel
}

/// What a view shows of one neighbourhood's hazards (B2's town views read
/// this; it decides nothing).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoodHazard {
    pub neighborhood: NeighborhoodId,
    pub exposure: HoodExposure,
    /// Homes damaged and not yet repaired, if any.
    pub damage: Option<HoodDamage>,
    /// Its residents staying elsewhere until they can return.
    pub displaced_away: u64,
    /// Residents of other neighbourhoods staying here for now.
    pub sheltering: u64,
}

/// One neighbourhood's exposure, damage and displacement, or `None` for an
/// unknown neighbourhood.
pub fn hood_hazard(state: &crate::State, id: NeighborhoodId) -> Option<HoodHazard> {
    state.world.neighborhoods.get(id.index())?;
    let env = &state.environment;
    Some(HoodHazard {
        neighborhood: id,
        exposure: env.exposure(id).unwrap_or_default(),
        damage: env.damage(id).cloned(),
        displaced_away: state.population.displaced_from(id),
        sheltering: state.population.displaced_into(id),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_keeps_the_town_mean_and_the_cap() {
        let share = [0.1, 0.3, 0.6];
        let rel = relative(&share, Some(&[900.0, 100.0, 50.0]), 4.0);
        let mean: f64 = share.iter().zip(&rel).map(|(s, r)| s * r).sum();
        assert!((mean - 1.0).abs() < 1e-6, "mean {mean}");
        assert!(rel[0] <= 4.0 && rel[0] > rel[1] && rel[1] > rel[2]);
        assert_eq!(relative(&share, Some(&[0.0, 0.0, 0.0]), 4.0), vec![1.0; 3]);
        assert_eq!(relative(&share, None, 4.0), vec![1.0; 3]);
    }
}
