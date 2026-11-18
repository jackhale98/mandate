//! Settlement sites, rank-size populations, and municipal territories
//! (worldgen steps 6–7).

use crate::grid::{Grid, HeapItem, cell_dist};
use crate::profile::CellProfile;
use mandate_core::SimRng;
use mandate_core::math::{pow, sqrt};
use mandate_sim::world::{Biome, Cover, SettlementClass};
use std::collections::{BTreeSet, BinaryHeap};

pub struct Site {
    pub cell: usize,
    pub score: f64,
    pub population: u64,
    pub class: SettlementClass,
}

pub fn site_score(
    i: usize,
    cover: &Grid<Cover>,
    biome: &Grid<Biome>,
    river: &Grid<u8>,
    elev: &Grid<f64>,
    prof: &CellProfile,
) -> Option<f64> {
    if cover.data[i] != Cover::Country {
        return None;
    }
    if matches!(biome.data[i], Biome::Alpine | Biome::Rock | Biome::Wetland) {
        return None;
    }
    let mut water: f64 = 0.0;
    let mut lake = false;
    for j in cover.neighbors8(i) {
        water = water.max(river.data[j] as f64 * 0.22);
        lake |= cover.data[j] == Cover::Lake;
    }
    water = water.max(river.data[i] as f64 * 0.22);
    let coast = if prof.coastal.data[i] { 0.45 } else { 0.0 };
    let harbor = if prof.coastal.data[i] && prof.slope.data[i] < 0.2 {
        0.25
    } else {
        0.0
    };
    let resources = prof.minerals.data[i]
        .max(prof.timber.data[i] * 0.6)
        .max(prof.energy.data[i] * 0.7)
        * 0.3;
    Some(
        1.1 * prof.fertility.data[i]
            + water
            + coast
            + harbor
            + if lake { 0.25 } else { 0.0 }
            + resources
            + 0.3 * (1.0 - prof.slope.data[i])
            - (elev.data[i] / 2_000.0).min(0.6)
            - 0.25 * prof.flood.data[i],
    )
}

/// Choose `count` sites greedily by score with minimum spacing.
pub fn choose_sites(
    rng: &mut SimRng,
    scores: &[(usize, f64)],
    grid_w: usize,
    country_cells: usize,
    count: usize,
) -> Vec<usize> {
    let mut cands: Vec<(usize, f64)> = scores
        .iter()
        .map(|&(i, s)| (i, s + rng.range_f64(0.0, 0.25)))
        .collect();
    cands.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut spacing = sqrt(country_cells as f64 / count.max(1) as f64) * 0.72;
    let dummy: Grid<u8> = Grid {
        w: grid_w,
        h: 1,
        data: vec![],
    };
    loop {
        let mut chosen: Vec<usize> = Vec::new();
        for &(i, _) in &cands {
            if chosen.iter().all(|&c| cell_dist(&dummy, c, i) >= spacing) {
                chosen.push(i);
                if chosen.len() == count {
                    return chosen;
                }
            }
        }
        if spacing < 2.0 {
            return chosen;
        }
        spacing *= 0.85;
    }
}

/// Rank-size populations (Zipf-like) normalised to `total`.
pub fn populations(rng: &mut SimRng, n: usize, total: u64) -> Vec<u64> {
    let raw: Vec<f64> = (0..n)
        .map(|r| pow((r + 1) as f64, -1.1) * rng.range_f64(0.8, 1.2))
        .collect();
    let sum: f64 = raw.iter().sum();
    raw.iter()
        .map(|v| ((v / sum) * total as f64).max(1_500.0) as u64)
        .collect()
}

pub fn class_for(pop: u64, rank: usize) -> SettlementClass {
    if rank == 0 || pop >= 700_000 {
        SettlementClass::Metro
    } else if pop >= 150_000 {
        SettlementClass::City
    } else if pop >= 40_000 {
        SettlementClass::Town
    } else {
        SettlementClass::Rural
    }
}

/// Multi-source Dijkstra over country cells with terrain costs. Returns the
/// owning site index per cell (`u32::MAX` if unowned).
pub fn territories(
    sites: &[usize],
    cover: &Grid<Cover>,
    river: &Grid<u8>,
    prof: &CellProfile,
) -> Grid<u32> {
    let mut owner = Grid::new(cover.w, cover.h, u32::MAX);
    let mut best = vec![f64::MAX; cover.len()];
    let mut heap = BinaryHeap::new();
    for (k, &s) in sites.iter().enumerate() {
        owner.set(s, k as u32);
        best[s] = 0.0;
        heap.push(HeapItem { cost: 0.0, idx: s });
    }
    while let Some(HeapItem { cost, idx }) = heap.pop() {
        if cost > best[idx] {
            continue;
        }
        let o = owner.data[idx];
        for d in crate::grid::N8 {
            let Some(n) = cover.offset(idx, d) else {
                continue;
            };
            if !matches!(cover.data[n], Cover::Country | Cover::Lake) {
                continue;
            }
            let step = if d.0 != 0 && d.1 != 0 { 1.414 } else { 1.0 };
            let water = if cover.data[n] == Cover::Lake {
                6.0
            } else if river.data[n] >= 2 {
                1.5
            } else {
                0.0
            };
            let c = cost + step * (1.0 + 3.0 * prof.slope.data[n] + water);
            if c < best[n] || (c == best[n] && o < owner.data[n]) {
                best[n] = c;
                owner.set(n, o);
                heap.push(HeapItem { cost: c, idx: n });
            }
        }
    }
    owner
}

/// Sorted neighbour lists between territories (4-adjacency).
pub fn adjacency(owner: &Grid<u32>, n: usize) -> Vec<Vec<u32>> {
    let mut sets = vec![BTreeSet::new(); n];
    for i in 0..owner.len() {
        let a = owner.data[i];
        if a == u32::MAX {
            continue;
        }
        for j in owner.neighbors4(i) {
            let b = owner.data[j];
            if b != u32::MAX && b != a {
                sets[a as usize].insert(b);
            }
        }
    }
    sets.into_iter().map(|s| s.into_iter().collect()).collect()
}
