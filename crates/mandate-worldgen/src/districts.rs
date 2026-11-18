//! Contiguous, population-balanced partitioning of a unit graph into `k`
//! districts (worldgen step 10). Used for wards and legislative districts.

use std::collections::{BTreeSet, VecDeque};

pub struct PartitionInput<'a> {
    /// Units to partition (indices into `adjacency` / `population`).
    pub units: &'a [usize],
    pub adjacency: &'a [Vec<usize>],
    pub population: &'a [u64],
}

/// Returns a district index per entry of `units`. Districts are contiguous
/// within the induced subgraph whenever the subgraph itself is connected.
pub fn partition(inp: &PartitionInput<'_>, k: usize) -> Vec<usize> {
    let n = inp.units.len();
    if n == 0 {
        return Vec::new();
    }
    let k = k.clamp(1, n);
    let pos: std::collections::BTreeMap<usize, usize> =
        inp.units.iter().enumerate().map(|(i, u)| (*u, i)).collect();
    let adj: Vec<Vec<usize>> = inp
        .units
        .iter()
        .map(|u| {
            inp.adjacency[*u]
                .iter()
                .filter_map(|v| pos.get(v).copied())
                .collect()
        })
        .collect();
    let pop: Vec<u64> = inp
        .units
        .iter()
        .map(|u| inp.population[*u].max(1))
        .collect();

    // Seeds: farthest-point sampling by hop distance, starting from the largest unit.
    let mut seeds = vec![
        (0..n)
            .max_by_key(|&i| (pop[i], usize::MAX - i))
            .unwrap_or(0),
    ];
    while seeds.len() < k {
        let d = hops(&adj, &seeds);
        let next = (0..n)
            .filter(|i| !seeds.contains(i))
            .max_by_key(|&i| (d[i], pop[i], usize::MAX - i));
        match next {
            Some(i) => seeds.push(i),
            None => break,
        }
    }
    let k = seeds.len();
    let mut assign = vec![usize::MAX; n];
    let mut totals = vec![0u64; k];
    for (d, &s) in seeds.iter().enumerate() {
        assign[s] = d;
        totals[d] = pop[s];
    }

    // Greedy growth: the lightest district takes its lightest free neighbour.
    let mut remaining = n - k;
    while remaining > 0 {
        let mut order: Vec<usize> = (0..k).collect();
        order.sort_by_key(|&d| (totals[d], d));
        let mut progressed = false;
        for d in order {
            let frontier: BTreeSet<usize> = (0..n)
                .filter(|&i| assign[i] == d)
                .flat_map(|i| adj[i].iter().copied())
                .filter(|&j| assign[j] == usize::MAX)
                .collect();
            if let Some(&take) = frontier.iter().min_by_key(|&&j| (pop[j], j)) {
                assign[take] = d;
                totals[d] += pop[take];
                remaining -= 1;
                progressed = true;
                break;
            }
        }
        if !progressed {
            // Disconnected leftovers join the lightest district.
            for i in 0..n {
                if assign[i] == usize::MAX {
                    let d = (0..k).min_by_key(|&d| (totals[d], d)).unwrap_or(0);
                    assign[i] = d;
                    totals[d] += pop[i];
                    remaining -= 1;
                }
            }
        }
    }

    // Refinement: move boundary units from heavier to lighter neighbours when
    // it reduces imbalance and keeps the donor district connected.
    for _ in 0..60 {
        let mut moved = false;
        for i in 0..n {
            let a = assign[i];
            for &j in &adj[i] {
                let b = assign[j];
                if a == b || totals[a] <= totals[b] + pop[i] {
                    continue;
                }
                if !connected_without(&adj, &assign, a, i) {
                    continue;
                }
                assign[i] = b;
                totals[a] -= pop[i];
                totals[b] += pop[i];
                moved = true;
                break;
            }
        }
        if !moved {
            break;
        }
    }
    assign
}

fn hops(adj: &[Vec<usize>], sources: &[usize]) -> Vec<usize> {
    let mut d = vec![usize::MAX / 2; adj.len()];
    let mut q = VecDeque::new();
    for &s in sources {
        d[s] = 0;
        q.push_back(s);
    }
    while let Some(i) = q.pop_front() {
        for &j in &adj[i] {
            if d[j] > d[i] + 1 {
                d[j] = d[i] + 1;
                q.push_back(j);
            }
        }
    }
    d
}

fn connected_without(
    adj: &[Vec<usize>],
    assign: &[usize],
    district: usize,
    removed: usize,
) -> bool {
    let members: Vec<usize> = (0..assign.len())
        .filter(|&i| assign[i] == district && i != removed)
        .collect();
    let Some(&start) = members.first() else {
        return false;
    };
    let mut seen = BTreeSet::from([start]);
    let mut q = VecDeque::from([start]);
    while let Some(i) = q.pop_front() {
        for &j in &adj[i] {
            if j != removed && assign[j] == district && seen.insert(j) {
                q.push_back(j);
            }
        }
    }
    seen.len() == members.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn grid_graph(w: usize, h: usize) -> Vec<Vec<usize>> {
        (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                let mut v = Vec::new();
                if x > 0 {
                    v.push(i - 1);
                }
                if x + 1 < w {
                    v.push(i + 1);
                }
                if y > 0 {
                    v.push(i - w);
                }
                if y + 1 < h {
                    v.push(i + w);
                }
                v
            })
            .collect()
    }

    proptest! {
        #[test]
        fn districts_are_contiguous_and_cover_everything(w in 2usize..9, h in 2usize..9, k in 1usize..8, seed in any::<u64>()) {
            let adj = grid_graph(w, h);
            let mut rng = mandate_core::RngStreams::new(seed);
            let pop: Vec<u64> = (0..w * h).map(|_| 100 + rng.get(mandate_core::Stream::Misc).below(1000)).collect();
            let units: Vec<usize> = (0..w * h).collect();
            let a = partition(&PartitionInput { units: &units, adjacency: &adj, population: &pop }, k);
            prop_assert_eq!(a.len(), w * h);
            let kk = k.min(w * h);
            for d in 0..kk {
                let members: Vec<usize> = (0..a.len()).filter(|&i| a[i] == d).collect();
                prop_assert!(!members.is_empty());
                let mut seen = BTreeSet::from([members[0]]);
                let mut q = VecDeque::from([members[0]]);
                while let Some(i) = q.pop_front() {
                    for &j in &adj[i] { if a[j] == d && seen.insert(j) { q.push_back(j); } }
                }
                prop_assert_eq!(seen.len(), members.len());
            }
        }
    }
}
