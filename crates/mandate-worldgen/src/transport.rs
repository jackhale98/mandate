//! Transport network: least-cost road paths between neighbouring
//! municipalities, MST plus shortcuts, and rail spines (worldgen step 8).

use crate::grid::{Grid, HeapItem, N8, cell_dist};
use crate::profile::CellProfile;
use mandate_sim::world::Cover;
use std::collections::BinaryHeap;

pub struct RoadEdge {
    pub a: usize,
    pub b: usize,
    pub cost: f64,
    pub length_cells: f64,
    pub path: Vec<u32>,
}

/// A* over land cells; lakes and sea are impassable, slopes and rivers cost more.
pub fn path(
    from: usize,
    to: usize,
    cover: &Grid<Cover>,
    river: &Grid<u8>,
    prof: &CellProfile,
) -> Option<(f64, f64, Vec<u32>)> {
    let n = cover.len();
    let mut g = vec![f64::MAX; n];
    let mut came = vec![usize::MAX; n];
    let mut heap = BinaryHeap::new();
    g[from] = 0.0;
    heap.push(HeapItem {
        cost: cell_dist(cover, from, to),
        idx: from,
    });
    while let Some(HeapItem { idx, .. }) = heap.pop() {
        if idx == to {
            break;
        }
        for d in N8 {
            let Some(nb) = cover.offset(idx, d) else {
                continue;
            };
            if !matches!(cover.data[nb], Cover::Country | Cover::Lake) {
                continue;
            }
            let step = if d.0 != 0 && d.1 != 0 { 1.414 } else { 1.0 };
            // Lakes are crossed by bridges and ferries at a high cost.
            let water = if cover.data[nb] == Cover::Lake {
                8.0
            } else if river.data[nb] >= 2 {
                2.0
            } else {
                0.0
            };
            let c = g[idx] + step * (1.0 + 5.0 * prof.slope.data[nb] + water);
            if c < g[nb] {
                g[nb] = c;
                came[nb] = idx;
                heap.push(HeapItem {
                    cost: c + cell_dist(cover, nb, to),
                    idx: nb,
                });
            }
        }
    }
    if g[to] == f64::MAX {
        return None;
    }
    let mut cells = vec![to as u32];
    let mut cur = to;
    let mut length = 0.0;
    while cur != from {
        let p = came[cur];
        length += cell_dist(cover, p, cur);
        cells.push(p as u32);
        cur = p;
    }
    cells.reverse();
    Some((g[to], length, cells))
}

struct Dsu(Vec<usize>);
impl Dsu {
    fn find(&mut self, x: usize) -> usize {
        if self.0[x] != x {
            let r = self.find(self.0[x]);
            self.0[x] = r;
        }
        self.0[x]
    }
    fn union(&mut self, a: usize, b: usize) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return false;
        }
        self.0[ra.max(rb)] = ra.min(rb);
        true
    }
}

/// Graph shortest path cost between two nodes using chosen edges.
pub fn graph_dist(n: usize, edges: &[&RoadEdge], from: usize, to: usize) -> (f64, Vec<usize>) {
    let mut adj: Vec<Vec<(usize, f64, usize)>> = vec![Vec::new(); n];
    for (k, e) in edges.iter().enumerate() {
        adj[e.a].push((e.b, e.cost, k));
        adj[e.b].push((e.a, e.cost, k));
    }
    let mut dist = vec![f64::MAX; n];
    let mut via = vec![usize::MAX; n];
    let mut prev = vec![usize::MAX; n];
    let mut heap = BinaryHeap::new();
    dist[from] = 0.0;
    heap.push(HeapItem {
        cost: 0.0,
        idx: from,
    });
    while let Some(HeapItem { cost, idx }) = heap.pop() {
        if cost > dist[idx] {
            continue;
        }
        for &(nb, w, k) in &adj[idx] {
            let c = cost + w;
            if c < dist[nb] {
                dist[nb] = c;
                via[nb] = k;
                prev[nb] = idx;
                heap.push(HeapItem { cost: c, idx: nb });
            }
        }
    }
    let mut route = Vec::new();
    let mut cur = to;
    while cur != from && prev[cur] != usize::MAX {
        route.push(via[cur]);
        cur = prev[cur];
    }
    route.reverse();
    (dist[to], route)
}

/// MST over candidate edges plus shortcuts that cut detours by > 35%.
pub fn select_network(n: usize, mut candidates: Vec<RoadEdge>) -> Vec<RoadEdge> {
    candidates.sort_by(|x, y| x.cost.total_cmp(&y.cost).then((x.a, x.b).cmp(&(y.a, y.b))));
    let mut dsu = Dsu((0..n).collect());
    let mut chosen = Vec::new();
    let mut rest = Vec::new();
    for e in candidates {
        if dsu.union(e.a, e.b) {
            chosen.push(e);
        } else {
            rest.push(e);
        }
    }
    for e in rest {
        let refs: Vec<&RoadEdge> = chosen.iter().collect();
        let (d, _) = graph_dist(n, &refs, e.a, e.b);
        if e.cost * 1.35 < d {
            chosen.push(e);
        }
    }
    chosen
}
