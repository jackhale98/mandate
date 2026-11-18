//! Depression filling, flow routing, rivers, and lakes (worldgen step 2).

use crate::grid::{Grid, HeapItem};
use mandate_sim::world::Cover;
use std::collections::BinaryHeap;

pub struct Hydrology {
    /// Downstream neighbour per cell (`usize::MAX` for outlets).
    pub downstream: Vec<usize>,
    /// Upstream runoff in "mean cell" units.
    pub accumulation: Grid<f64>,
    /// River class: 0 none, 1 stream, 2 river, 3 major river.
    pub river: Grid<u8>,
}

/// Priority-flood (Barnes et al.) with an epsilon gradient so every land cell
/// drains to the sea or a map edge. Returns the filled surface.
pub fn fill_depressions(elev: &Grid<f64>, cover: &Grid<Cover>) -> Grid<f64> {
    let mut filled = elev.clone();
    let mut done = vec![false; elev.len()];
    let mut heap = BinaryHeap::new();
    for i in 0..elev.len() {
        if cover.data[i] == Cover::Sea || elev.is_edge(i) {
            done[i] = true;
            heap.push(HeapItem {
                cost: filled.data[i],
                idx: i,
            });
        }
    }
    while let Some(HeapItem { idx, .. }) = heap.pop() {
        let base = filled.data[idx];
        for n in elev.neighbors8(idx) {
            if done[n] {
                continue;
            }
            done[n] = true;
            if filled.data[n] <= base {
                filled.set(n, base + 1e-3);
            }
            heap.push(HeapItem {
                cost: filled.data[n],
                idx: n,
            });
        }
    }
    filled
}

pub fn route(
    elev: &Grid<f64>,
    filled: &Grid<f64>,
    cover: &mut Grid<Cover>,
    precip: &Grid<f64>,
) -> Hydrology {
    let n = elev.len();
    let mut downstream = vec![usize::MAX; n];
    for (i, ds) in downstream.iter_mut().enumerate() {
        if cover.data[i] == Cover::Sea {
            continue;
        }
        let mut best = filled.data[i];
        for j in filled.neighbors8(i) {
            let v = filled.data[j];
            if v < best || (v == best && *ds != usize::MAX && j < *ds) {
                best = v;
                *ds = j;
            }
        }
    }

    // Lakes: deep, filled depressions of at least `MIN_LAKE` connected cells.
    const MIN_LAKE: usize = 10;
    let deep: Vec<bool> = (0..n)
        .map(|i| cover.data[i] == Cover::Country && filled.data[i] - elev.data[i] > 20.0)
        .collect();
    let mut seen = vec![false; n];
    for start in 0..n {
        if !deep[start] || seen[start] {
            continue;
        }
        let mut comp = vec![start];
        seen[start] = true;
        let mut k = 0;
        while k < comp.len() {
            let i = comp[k];
            k += 1;
            for j in elev.neighbors4(i) {
                if deep[j] && !seen[j] {
                    seen[j] = true;
                    comp.push(j);
                }
            }
        }
        if comp.len() >= MIN_LAKE {
            for i in comp {
                cover.set(i, Cover::Lake);
            }
        }
    }

    let land_cells: Vec<usize> = (0..n).filter(|&i| cover.data[i] != Cover::Sea).collect();
    let mean_p =
        land_cells.iter().map(|&i| precip.data[i]).sum::<f64>() / land_cells.len().max(1) as f64;
    let mut order = land_cells;
    order.sort_by(|&a, &b| filled.data[b].total_cmp(&filled.data[a]).then(a.cmp(&b)));
    let mut acc = Grid::new(elev.w, elev.h, 0.0);
    for &i in &order {
        let here = acc.data[i] + precip.data[i] / mean_p.max(1.0);
        acc.set(i, here);
        let d = downstream[i];
        if d != usize::MAX {
            acc.data[d] += here;
        }
    }

    let scale = (elev.len() as f64 / 38_400.0).max(0.25);
    let (t1, t2, t3) = (70.0 * scale, 260.0 * scale, 900.0 * scale);
    let mut river = Grid::new(elev.w, elev.h, 0u8);
    for i in 0..n {
        if matches!(cover.data[i], Cover::Sea | Cover::Lake) {
            continue;
        }
        let a = acc.data[i];
        let class = if a >= t3 {
            3
        } else if a >= t2 {
            2
        } else if a >= t1 {
            1
        } else {
            0
        };
        river.set(i, class);
    }
    Hydrology {
        downstream,
        accumulation: acc,
        river,
    }
}
