//! The variety gate: which informed styles win, and where.

use super::*;

/// Per world, the best of a set of strategies are those within
/// `VARIETY_TIE` of the highest vote share (a near-tie is shared).
pub const VARIETY_TIE: f64 = 0.002;

/// `good_styles_vary` (ENGAGEMENT §7.2; DEC-077 §5 as amended by DEC-097):
/// among the informed styles, (a) at least `STYLES_MIN_WINNERS` whose mean
/// share is within `STYLES_NEAR` of the best style's are each the sole best
/// in at least `STYLES_MIN_SOLE` of the worlds; (b) none is among the best
/// in more than `STYLES_MAX_SHARE` of the worlds; (c) the best style differs
/// by starting situation (crisis or industry).
pub const STYLES_MIN_WINNERS: usize = 2;
pub const STYLES_NEAR: f64 = 0.015;
pub const STYLES_MIN_SOLE: f64 = 0.2;
pub const STYLES_MAX_SHARE: f64 = 0.6;

/// The best of `among` by situation: the highest mean share in each group
/// of worlds (`key` picks the group: crisis or industry).
pub(super) fn best_by(
    shares: &BTreeMap<u64, BTreeMap<&'static str, f64>>,
    situations: &BTreeMap<u64, (String, String)>,
    among: &[&'static str],
    key: impl Fn(&(String, String)) -> String,
) -> Vec<serde_json::Value> {
    let mut groups: BTreeMap<String, BTreeMap<&'static str, (f64, u32)>> = BTreeMap::new();
    for (seed, row) in shares {
        let k = situations
            .get(seed)
            .map(&key)
            .unwrap_or_else(|| "none".into());
        let g = groups.entry(k).or_default();
        for s in among {
            if let Some(v) = row.get(s) {
                let x = g.entry(s).or_default();
                x.0 += v;
                x.1 += 1;
            }
        }
    }
    groups
        .iter()
        .map(|(k, m)| {
            let best = among
                .iter()
                .filter_map(|s| m.get(s).map(|(t, c)| (*s, t / (*c).max(1) as f64, *c)))
                .fold(None::<(&str, f64, u32)>, |b, x| match b {
                    Some(b) if b.1 >= x.1 => Some(b),
                    _ => Some(x),
                });
            json!({
                "situation": k,
                "worlds": best.map(|b| b.2).unwrap_or(0),
                "best": best.map(|b| b.0).unwrap_or(""),
                "mean_share": best.map(|b| b.1).unwrap_or(0.0),
            })
        })
        .collect()
}

/// The variety of the best way of playing across worlds, among `among`
/// (the informed styles for `good_styles_vary`).
pub fn variety(
    shares: &BTreeMap<u64, BTreeMap<&'static str, f64>>,
    situations: &BTreeMap<u64, (String, String)>,
    among: &[&'static str],
) -> serde_json::Value {
    let mut among_best: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut sole_best: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut totals: BTreeMap<&'static str, (f64, u32)> = BTreeMap::new();
    let mut worlds = Vec::new();
    let mut n = 0usize;
    for (seed, row) in shares {
        let active: Vec<(&'static str, f64)> = among
            .iter()
            .filter_map(|s| row.get(s).map(|v| (*s, *v)))
            .collect();
        let Some(top) = active.iter().map(|x| x.1).reduce(f64::max) else {
            continue;
        };
        n += 1;
        for (s, v) in &active {
            let t = totals.entry(s).or_default();
            t.0 += v;
            t.1 += 1;
        }
        let best: Vec<&'static str> = active
            .iter()
            .filter(|x| x.1 >= top - VARIETY_TIE)
            .map(|x| x.0)
            .collect();
        for s in &best {
            *among_best.entry(s).or_default() += 1;
        }
        if let [only] = best.as_slice() {
            *sole_best.entry(only).or_default() += 1;
        }
        let second = active
            .iter()
            .filter(|x| !best.contains(&x.0))
            .map(|x| x.1)
            .fold(f64::MIN, f64::max);
        let (crisis, industry) = situations.get(seed).cloned().unwrap_or_default();
        worlds.push(json!({
            "seed": seed, "crisis": crisis, "industry": industry,
            "best": best, "share": top,
            "margin": if second > f64::MIN { top - second } else { 0.0 },
        }));
    }
    let nf = n.max(1) as f64;
    let means: BTreeMap<&'static str, f64> = totals
        .iter()
        .map(|(s, (t, c))| (*s, t / (*c).max(1) as f64))
        .collect();
    let best_mean = means.values().copied().fold(f64::MIN, f64::max);
    // (a) Several near-best styles each the sole best somewhere.
    let winners: Vec<&'static str> = among
        .iter()
        .copied()
        .filter(|s| means.get(s).is_some_and(|m| *m >= best_mean - STYLES_NEAR))
        .filter(|s| sole_best.get(s).copied().unwrap_or(0) as f64 / nf >= STYLES_MIN_SOLE)
        .collect();
    let a = winners.len() >= STYLES_MIN_WINNERS;
    // (b) None among the best in most worlds.
    let top = among_best.values().copied().max().unwrap_or(0) as f64 / nf;
    let b = top <= STYLES_MAX_SHARE;
    // (c) The best style differs by starting situation.
    let by_crisis = best_by(shares, situations, among, |s| s.0.clone());
    let by_industry = best_by(shares, situations, among, |s| s.1.clone());
    let distinct = |rows: &[serde_json::Value]| {
        let mut v: Vec<&str> = rows.iter().filter_map(|r| r["best"].as_str()).collect();
        v.sort();
        v.dedup();
        v.len()
    };
    let c = distinct(&by_crisis) >= 2 || distinct(&by_industry) >= 2;
    json!({
        "among": among,
        "worlds": worlds,
        "mean_share": means,
        "best_counts": among_best,
        "sole_best_counts": sole_best,
        "distinct_best": sole_best.len(),
        "top_best_share": top,
        "tie": VARIETY_TIE,
        "near_best_winners": winners,
        "by_crisis": by_crisis,
        "by_industry": by_industry,
        "a": a,
        "b": b,
        "c": c,
        "pass": a && b && c,
    })
}
