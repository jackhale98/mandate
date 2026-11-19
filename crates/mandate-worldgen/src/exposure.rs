//! Per-neighbourhood hazard exposure from the terrain (DEC-090 D8,
//! TOWN_GENERATION §10).
//!
//! Each neighbourhood is given a site, one terrain cell of its town, chosen
//! by its kind: the core at the town's centre, the waterfront on the lowest
//! land by the water, industry on low land near it, inner districts beside
//! the centre, suburbs further out and higher, rural neighbourhoods spread
//! over the outskirts. Its land's exposure per hazard is ¾ its site's and ¼
//! the town's built-up mean (a district is bigger than a point), and its
//! stored exposure is the town's hazard index (`GeoProfile`, which sets how
//! often the hazard strikes) × that land's exposure against the town's most
//! exposed district: the riverside district of a flood-prone town is near
//! the town's index, its hilltop suburb far below, and a town the hazard
//! never strikes has none anywhere.
//!
//! - **flood**: height above the nearest drainage (followed down the flow
//!   directions, `TerrainData::downstream`, or next to a river, lake or the
//!   sea), scaled by the size of the water: a major river's floodplain is the
//!   most exposed land;
//! - **wildfire**: the fuel of the cell's biome and the forest at its edge
//!   (`TerrainData::forest_pm` of the cell and its neighbours);
//! - **storm**: coastal wind and surge on low land by the sea; inland, a
//!   little wind everywhere.
//!
//! Pure: no draws, ties broken by cell index, so a seed always gives the
//! same exposure.

use mandate_sim::environment::HoodExposure;
use mandate_sim::environment::exposure::per_mille;
use mandate_sim::world::{Biome, COMPASS_NONE, Cover, NeighborhoodKind, TerrainData, World};

/// Metres above the drainage at which flood exposure reaches zero.
const FLOOD_HEAD_M: f64 = 30.0;
/// Most steps followed down the flow directions to find the drainage.
const FLOW_STEPS: usize = 12;

/// Exposure for every neighbourhood of the world, indexed by neighbourhood.
pub fn derive(world: &World, t: &TerrainData) -> Vec<HoodExposure> {
    let mut out = vec![HoodExposure::default(); world.neighborhoods.len()];
    let n = t.w * t.h;
    if n == 0 || t.elevation.len() != n || t.owner.len() != n {
        return out;
    }
    // Each municipality's land cells, in index order.
    let mut cells: Vec<Vec<usize>> = vec![Vec::new(); world.municipalities.len()];
    for (i, &o) in t.owner.iter().enumerate() {
        if t.cover[i] == Cover::Country
            && let Some(v) = cells.get_mut(o as usize)
        {
            v.push(i);
        }
    }
    for (m, owned) in cells.iter().enumerate() {
        for (h, e) in town(world, m, owned, t) {
            if let Some(x) = out.get_mut(h) {
                *x = e;
            }
        }
    }
    out
}

/// One cell's hazard indices in [0, 1].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CellHazard {
    pub flood: f64,
    pub wildfire: f64,
    pub storm: f64,
}

impl CellHazard {
    fn blend(self, town: CellHazard) -> CellHazard {
        let mix = |a: f64, b: f64| 0.75 * a + 0.25 * b;
        CellHazard {
            flood: mix(self.flood, town.flood),
            wildfire: mix(self.wildfire, town.wildfire),
            storm: mix(self.storm, town.storm),
        }
    }

    fn max(self, o: CellHazard) -> CellHazard {
        CellHazard {
            flood: self.flood.max(o.flood),
            wildfire: self.wildfire.max(o.wildfire),
            storm: self.storm.max(o.storm),
        }
    }
}

/// The town's index × land exposure / the town's most exposed land.
fn scaled(land: CellHazard, top: CellHazard, g: &mandate_sim::world::GeoProfile) -> HoodExposure {
    let s = |x: f64, top: f64, index: f64| {
        if top > 0.0 {
            per_mille(index * x / top)
        } else {
            0
        }
    };
    HoodExposure {
        flood: s(land.flood, top.flood, g.flood),
        wildfire: s(land.wildfire, top.wildfire, g.wildfire),
        storm: s(land.storm, top.storm, g.storm),
    }
}

/// The sites and exposures of one town's neighbourhoods.
fn town(world: &World, m: usize, owned: &[usize], t: &TerrainData) -> Vec<(usize, HoodExposure)> {
    let Some(mu) = world.municipalities.get(m) else {
        return Vec::new();
    };
    let (cx, cy) = (i64::from(mu.center.0), i64::from(mu.center.1));
    let dist = |i: usize| -> i64 {
        let (x, y) = ((i % t.w) as i64, (i / t.w) as i64);
        (x - cx).abs().max((y - cy).abs())
    };
    let hoods = &mu.neighborhoods;
    // The built-up area grows with the number of neighbourhoods.
    let r: i64 = match hoods.len() {
        0..=3 => 1,
        4..=9 => 2,
        _ => 3,
    };
    let mut sorted: Vec<usize> = owned.to_vec();
    sorted.sort_by_key(|&i| (dist(i), i));
    let within = |lo: i64, hi: i64| -> Vec<usize> {
        sorted
            .iter()
            .copied()
            .filter(|&i| (lo..=hi).contains(&dist(i)))
            .collect()
    };
    let centre = (cy.max(0) as usize) * t.w + cx.max(0) as usize;
    let fallback: Vec<usize> = if centre < t.w * t.h {
        vec![centre]
    } else {
        Vec::new()
    };
    let near = {
        let v = within(0, r);
        if v.is_empty() { fallback.clone() } else { v }
    };
    if near.is_empty() {
        return Vec::new();
    }
    let ring = {
        let v = within(0, r + 1);
        if v.is_empty() { near.clone() } else { v }
    };
    let outer = {
        let v = within(r + 1, r + 4);
        if v.is_empty() { ring.clone() } else { v }
    };
    let k = near.len() as f64;
    let mean = near.iter().fold(CellHazard::default(), |a, &i| {
        let c = cell(t, i);
        CellHazard {
            flood: a.flood + c.flood / k,
            wildfire: a.wildfire + c.wildfire / k,
            storm: a.storm + c.storm / k,
        }
    });
    let elev_dm = |i: usize| (f64::from(t.elevation[i]) * 10.0).round() as i64;
    let wet = |i: usize| i64::from(by_water(t, i));
    let mut used: Vec<usize> = Vec::new();
    let mut land: Vec<(usize, CellHazard)> = Vec::with_capacity(hoods.len());
    for id in hoods {
        let Some(hood) = world.neighborhoods.get(id.index()) else {
            continue;
        };
        // The candidates for this kind (unused first) and what it prefers:
        // the largest key wins; ties go to the lower cell index.
        let cands: &[usize] = match hood.kind {
            NeighborhoodKind::Core | NeighborhoodKind::Inner | NeighborhoodKind::Industrial => {
                &near
            }
            NeighborhoodKind::Waterfront | NeighborhoodKind::Suburb => &ring,
            NeighborhoodKind::Rural => &outer,
        };
        let key = |i: usize, used: &[usize]| -> (i64, i64, i64) {
            match hood.kind {
                NeighborhoodKind::Core | NeighborhoodKind::Inner => (-dist(i), 0, 0),
                // The lowest land by the water.
                NeighborhoodKind::Waterfront => (wet(i), -elev_dm(i), -dist(i)),
                // Low land (in 5 m steps), by the water if it can.
                NeighborhoodKind::Industrial => (-elev_dm(i) / 50, wet(i), -dist(i)),
                // Further out, on higher ground.
                NeighborhoodKind::Suburb => (dist(i), elev_dm(i), 0),
                // Spread out: as far as possible from every site so far.
                NeighborhoodKind::Rural => {
                    let (x, y) = ((i % t.w) as i64, (i / t.w) as i64);
                    let spread = used
                        .iter()
                        .map(|&u| {
                            let (ux, uy) = ((u % t.w) as i64, (u / t.w) as i64);
                            (x - ux).abs().max((y - uy).abs())
                        })
                        .min()
                        .unwrap_or(0);
                    (spread, 0, 0)
                }
            }
        };
        let free: Vec<usize> = cands
            .iter()
            .copied()
            .filter(|i| !used.contains(i))
            .collect();
        let pool: &[usize] = if free.is_empty() { cands } else { &free };
        let mut best: Option<(usize, (i64, i64, i64))> = None;
        for &i in pool {
            let kv = key(i, &used);
            if best.is_none_or(|(_, b)| kv > b) {
                best = Some((i, kv));
            }
        }
        let Some((site, _)) = best else {
            continue;
        };
        used.push(site);
        land.push((id.index(), cell(t, site).blend(mean)));
    }
    let top = land
        .iter()
        .fold(CellHazard::default(), |a, (_, c)| a.max(*c));
    land.into_iter()
        .map(|(h, c)| (h, scaled(c, top, &mu.geo)))
        .collect()
}

/// Whether a cell is on or beside water (a river, a lake or the sea).
fn by_water(t: &TerrainData, i: usize) -> bool {
    water(t, i).is_some() || neighbours(t, i).any(|j| water(t, j).is_some())
}

/// The drainage a cell is, if any: (how much water: a major river 1, the sea
/// 0.6, …, the water's surface level in metres).
fn water(t: &TerrainData, i: usize) -> Option<(f64, f64)> {
    let e = f64::from(t.elevation[i]);
    match (t.cover[i], t.river[i]) {
        (Cover::Sea, _) => Some((0.6, 0.0)),
        (Cover::Lake, _) => Some((0.55, e)),
        (_, 3..) => Some((1.0, e)),
        (_, 2) => Some((0.8, e)),
        (_, 1) => Some((0.45, e)),
        _ => None,
    }
}

/// The 8 neighbours of a cell inside the grid.
fn neighbours(t: &TerrainData, i: usize) -> impl Iterator<Item = usize> + '_ {
    let (x, y) = ((i % t.w) as i64, (i / t.w) as i64);
    (-1i64..=1)
        .flat_map(move |dy| (-1i64..=1).map(move |dx| (dx, dy)))
        .filter(|&(dx, dy)| dx != 0 || dy != 0)
        .filter_map(move |(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            (nx >= 0 && ny >= 0 && (nx as usize) < t.w && (ny as usize) < t.h)
                .then(|| ny as usize * t.w + nx as usize)
        })
}

/// The next cell down the flow direction.
fn downstream(t: &TerrainData, i: usize) -> Option<usize> {
    let c = *t.downstream.get(i)?;
    if c == COMPASS_NONE {
        return None;
    }
    let (dx, dy): (i64, i64) = match c {
        0 => (0, -1),
        2 => (1, -1),
        4 => (1, 0),
        6 => (1, 1),
        8 => (0, 1),
        10 => (-1, 1),
        12 => (-1, 0),
        14 => (-1, -1),
        _ => return None,
    };
    let (x, y) = ((i % t.w) as i64 + dx, (i / t.w) as i64 + dy);
    (x >= 0 && y >= 0 && (x as usize) < t.w && (y as usize) < t.h)
        .then(|| y as usize * t.w + x as usize)
}

/// A cell's flood, wildfire and storm indices, from the terrain.
pub fn cell(t: &TerrainData, i: usize) -> CellHazard {
    let e = f64::from(t.elevation[i]);
    let head = |f: f64, level: f64| f * (1.0 - (e - level).max(0.0) / FLOOD_HEAD_M).clamp(0.0, 1.0);
    // Flood: on the water, beside it, or draining to it nearby.
    let mut flood = water(t, i).map_or(0.0, |(f, _)| f);
    for j in neighbours(t, i) {
        if let Some((f, level)) = water(t, j) {
            flood = flood.max(head(f, level));
        }
    }
    let mut j = downstream(t, i);
    for _ in 0..FLOW_STEPS {
        let Some(c) = j else { break };
        if let Some((f, level)) = water(t, c) {
            flood = flood.max(head(f, level));
            break;
        }
        j = downstream(t, c);
    }
    // Wildfire: what burns here, and the forest at the edge.
    let fuel = match t.biome[i] {
        Biome::Shrubland | Biome::Steppe => 0.8,
        Biome::TemperateForest | Biome::Boreal => 0.6,
        Biome::Grassland => 0.45,
        _ => 0.1,
    };
    let forest = std::iter::once(i)
        .chain(neighbours(t, i))
        .filter_map(|j| t.forest_pm.get(j).copied())
        .max()
        .unwrap_or(0);
    let wildfire = 0.5 * fuel + 0.5 * f64::from(forest) / 1000.0;
    // Storm: wind and surge on low land by the sea.
    let coastal = neighbours(t, i).any(|j| t.cover[j] == Cover::Sea);
    let storm = if coastal {
        0.35 + 0.65 * (1.0 - e.max(0.0) / 20.0).clamp(0.0, 1.0)
    } else {
        0.15
    };
    CellHazard {
        flood: flood.clamp(0.0, 1.0),
        wildfire: wildfire.clamp(0.0, 1.0),
        storm,
    }
}
