//! Playing the bots across seeds and building the report (`bots` JSON).

use super::*;

/// Which games a tournament plays: every strategy on every seed, and
/// `mixed` with the lab switch on (`knowledge_matters`) unless the whole run
/// is omniscient already.
pub(super) fn jobs(seeds: u64) -> Vec<(&'static str, u64, bool)> {
    let mut v: Vec<(&'static str, u64, bool)> = STRATEGIES
        .iter()
        .flat_map(|s| (1..=seeds).map(move |seed| (*s, seed, false)))
        .collect();
    if !omniscient() {
        v.extend((1..=seeds).map(|seed| ("mixed", seed, true)));
    }
    v
}

/// The games of a tournament. Each world is its own single-threaded,
/// deterministic simulation; independent worlds run on up to three threads
/// (a lab convenience: results are keyed, so the report never depends on
/// scheduling). Keyed by (strategy, seed, omniscient).
pub(super) fn play_all(
    content: &Arc<ContentBundle>,
    seeds: u64,
) -> BTreeMap<(&'static str, u64, bool), Played> {
    let jobs = jobs(seeds);
    // Up to three, or what the lab is told (`MANDATE_LAB_THREADS`).
    let workers = super::skips::workers().min(jobs.len().max(1));
    let mut out = BTreeMap::new();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|w| {
                let jobs = &jobs;
                scope.spawn(move || {
                    jobs.iter()
                        .skip(w)
                        .step_by(workers)
                        .filter_map(|(s, seed, omni)| {
                            let omni = *omni || omniscient();
                            play_full_with(content, *seed, s, omni)
                                .map(|p| ((*s, *seed, omni && !omniscient()), p))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for h in handles {
            if let Ok(v) = h.join() {
                out.extend(v);
            }
        }
    });
    out
}

/// Which policy families (DEC-092) the bots drafted (DEC-077 §5, the
/// variety check, M6_ENGINE_CONTRACTS §2.14): per strategy over all worlds,
/// and for the best informed style of each world (within `VARIETY_TIE` of
/// the highest share, as `variety` counts them). Reported, not enforced.
pub(super) fn families_drafted(
    results: &BTreeMap<(&'static str, u64, bool), Played>,
    shares: &BTreeMap<u64, BTreeMap<&'static str, f64>>,
    informed: &[&'static str],
    seeds: u64,
) -> serde_json::Value {
    let mut by_strategy: BTreeMap<&'static str, BTreeMap<String, u32>> = BTreeMap::new();
    for s in STRATEGIES {
        for seed in 1..=seeds {
            if let Some(p) = results.get(&(s, seed, false)) {
                let row = by_strategy.entry(s).or_default();
                for (f, n) in &p.families {
                    *row.entry(f.clone()).or_default() += n;
                }
            }
        }
    }
    let mut by_winners: BTreeMap<String, u32> = BTreeMap::new();
    let mut worlds_with_family = 0u32;
    for (seed, row) in shares {
        let top = informed
            .iter()
            .filter_map(|s| row.get(s))
            .copied()
            .fold(f64::MIN, f64::max);
        let mut any = false;
        for s in informed {
            if row.get(s).is_some_and(|v| *v >= top - VARIETY_TIE)
                && let Some(p) = results.get(&(*s, *seed, false))
            {
                for (f, n) in &p.families {
                    *by_winners.entry(f.clone()).or_default() += n;
                    any = true;
                }
            }
        }
        worlds_with_family += any as u32;
    }
    json!({
        "by_strategy": by_strategy,
        "by_winners": by_winners,
        "distinct_by_winners": by_winners.len(),
        "worlds_where_winners_drafted_one": worlds_with_family,
    })
}

/// The share of each verb in a mix (verbs used / all verbs used).
pub(super) fn mix_shares(mix: &BTreeMap<&'static str, u32>) -> BTreeMap<&'static str, f64> {
    let total = mix.values().sum::<u32>().max(1) as f64;
    mix.iter().map(|(k, v)| (*k, *v as f64 / total)).collect()
}

/// How far apart two action mixes are: half the sum of the differences in
/// each verb's share (0: the same mix; 1: no verb in common).
pub(super) fn mix_distance(
    a: &BTreeMap<&'static str, f64>,
    b: &BTreeMap<&'static str, f64>,
) -> f64 {
    let keys: std::collections::BTreeSet<&&str> = a.keys().chain(b.keys()).collect();
    keys.into_iter()
        .map(|k| (a.get(*k).unwrap_or(&0.0) - b.get(*k).unwrap_or(&0.0)).abs())
        .sum::<f64>()
        / 2.0
}

pub fn run(content: &Arc<ContentBundle>, seeds: u64) -> serde_json::Value {
    let mut rows = BTreeMap::new();
    let mut idle_share: BTreeMap<u64, f64> = BTreeMap::new();
    let mut shares: BTreeMap<u64, BTreeMap<&'static str, f64>> = BTreeMap::new();
    let mut situations: BTreeMap<u64, (String, String)> = BTreeMap::new();
    let mut mixes: BTreeMap<&'static str, BTreeMap<&'static str, f64>> = BTreeMap::new();
    let results = play_all(content, seeds);
    for s in STRATEGIES {
        let mut wins = 0;
        let mut n = 0;
        let mut gain = 0.0;
        let mut total = 0.0;
        let mut mix: BTreeMap<&'static str, u32> = BTreeMap::new();
        for seed in 1..=seeds {
            if let Some(p) = results.get(&(s, seed, false)) {
                let (won, share) = (p.won, p.share);
                shares.entry(seed).or_default().insert(s, share);
                situations.insert(seed, (p.crisis.clone(), p.industry.clone()));
                n += 1;
                wins += won as u32;
                total += share;
                if s == "idle" {
                    idle_share.insert(seed, share);
                }
                gain += share - idle_share.get(&seed).copied().unwrap_or(share);
                for (k, v) in &p.mix {
                    *mix.entry(k).or_default() += v;
                }
            }
        }
        let per_career: BTreeMap<&'static str, f64> = mix
            .iter()
            .map(|(k, v)| (*k, *v as f64 / n.max(1) as f64))
            .collect();
        let shares_of = mix_shares(&mix);
        mixes.insert(s, shares_of.clone());
        rows.insert(
            s.to_string(),
            json!({
                "class": class(s).key(),
                "races": n,
                "win_rate": wins as f64 / n.max(1) as f64,
                "mean_share": total / n.max(1) as f64,
                "mean_share_gain_vs_idle": gain / n.max(1) as f64,
                "mix": shares_of,
                "verbs_per_career": per_career,
            }),
        );
    }
    let rate = |s: &str| rows.get(s).and_then(|v| v["win_rate"].as_f64());
    let mean = |s: &str| rows.get(s).and_then(|v| v["mean_share"].as_f64());
    let of = |f: fn(&str) -> bool| -> Vec<&'static str> {
        STRATEGIES.iter().copied().filter(|s| f(s)).collect()
    };
    let (single, rote, informed) = (
        of(|s| class(s) == Class::SingleVerb),
        of(is_rote),
        of(is_informed),
    );
    let honest = of(|s| !is_probe(s));
    let max_of = |set: &[&'static str], f: &dyn Fn(&str) -> Option<f64>| {
        set.iter().filter_map(|s| f(s)).fold(0.0, f64::max)
    };
    let best = max_of(&honest, &rate);
    let median_of = |set: &[&'static str]| {
        let mut r: Vec<f64> = set.iter().filter_map(|s| rate(s)).collect();
        r.sort_by(|a, b| a.total_cmp(b));
        r.get(r.len() / 2).copied().unwrap_or(0.0)
    };
    let idle = rate("idle").unwrap_or(1.0);
    // Rung 0: effort pays, and exploring alone never wins.
    let idle_cap = idle <= IDLE_CAP;
    let effort_matters = best > idle;
    let explore_only = rate("explore_only").unwrap_or(1.0);
    let rote_median = median_of(&rote);
    let explore_alone = explore_only <= idle + EXPLORE_ONLY_OVER_IDLE && explore_only < rote_median;
    // Rung 1: a blind mix beats every single verb.
    let (mixed_mean, mixed_rate) = (mean("mixed").unwrap_or(0.0), rate("mixed").unwrap_or(0.0));
    let ladder_rote = single.iter().all(|s| {
        mixed_mean >= mean(s).unwrap_or(1.0) + ROTE_SHARE_MARGIN
            && mixed_rate >= rate(s).unwrap_or(1.0) + ROTE_WIN_MARGIN
    });
    // Rung 2: learning the town pays, and knowing it is worth as much.
    let explorer = mean("explorer").unwrap_or(0.0);
    let ladder_learned =
        explorer >= mixed_mean + LEARNED_MARGIN && rate("explorer").unwrap_or(0.0) >= mixed_rate;
    let ladder_learned_e8 =
        explorer >= mixed_mean + LEARNED_MARGIN_E8 && rate("explorer").unwrap_or(0.0) >= mixed_rate;
    let omni: Vec<&Played> = (1..=seeds)
        .filter_map(|seed| results.get(&("mixed", seed, true)))
        .collect();
    let omni_mean =
        (!omni.is_empty()).then(|| omni.iter().map(|p| p.share).sum::<f64>() / omni.len() as f64);
    let knowledge_matters = omni_mean.map(|m| m >= mixed_mean + KNOWLEDGE_MARGIN);
    // Rung 3: informed play beats rote play by a clear margin, in several styles.
    let (best_informed_mean, best_rote_mean) = (max_of(&informed, &mean), max_of(&rote, &mean));
    let (best_informed_rate, best_rote_rate) = (max_of(&informed, &rate), max_of(&rote, &rate));
    let skill_gap = best_informed_mean >= best_rote_mean + SKILL_SHARE_MARGIN
        && best_informed_rate >= best_rote_rate + SKILL_WIN_MARGIN;
    let styles = variety(&shares, &situations, &informed);
    let families = families_drafted(&results, &shares, &informed, seeds);
    // Skipping is a convenience, never the best way to play (DEC-097,
    // DEC-012): the proxy alone stays at or below the median informed style.
    let directed = mean("directed").unwrap_or(1.0);
    let informed_median = {
        let mut m: Vec<f64> = informed.iter().filter_map(|s| mean(s)).collect();
        m.sort_by(|a, b| a.total_cmp(b));
        match m.len() {
            0 => 0.0,
            n if n.is_multiple_of(2) => (m[n / 2 - 1] + m[n / 2]) / 2.0,
            n => m[n / 2],
        }
    };
    let proxy_not_optimal = directed <= informed_median;
    // Corruption tempts but never dominates.
    let corrupt = rate("corrupt").unwrap_or(0.0);
    let corrupt_ok = corrupt <= best + CORRUPT_MARGIN;
    // The informed styles each play their own way (reported).
    let explorer_mix = mixes.get("explorer").cloned().unwrap_or_default();
    let mix_vs_explorer: BTreeMap<&'static str, f64> = informed
        .iter()
        .filter(|s| **s != "explorer")
        .map(|s| {
            (
                *s,
                mix_distance(mixes.get(s).unwrap_or(&BTreeMap::new()), &explorer_mix),
            )
        })
        .collect();
    // Is `canvass` still the best way of playing (the owner's worry)?
    let top_by = |f: &dyn Fn(&str) -> Option<f64>| {
        honest
            .iter()
            .copied()
            .max_by(|a, b| {
                f(a).unwrap_or(0.0)
                    .total_cmp(&f(b).unwrap_or(0.0))
                    .then(b.cmp(a))
            })
            .unwrap_or("")
    };
    // The skip rows (DEC-102 clause 7): measured and printed, never
    // enforced; `proxy_not_optimal` above is as it was.
    let best_informed = informed
        .iter()
        .copied()
        .max_by(|a, b| {
            rate(a)
                .unwrap_or(0.0)
                .total_cmp(&rate(b).unwrap_or(0.0))
                .then(mean(a).unwrap_or(0.0).total_cmp(&mean(b).unwrap_or(0.0)))
                .then(b.cmp(a))
        })
        .unwrap_or("explorer");
    let skips = super::skips::rows(content, seeds, &rows, best_informed);
    let enforced = |g: &str| ENFORCED.contains(&g);
    let rung = |r: u8, g: &str, pass: serde_json::Value, detail: serde_json::Value| json!({ "rung": r, "gate": g, "pass": pass, "enforced": enforced(g), "detail": detail });
    let ladder = vec![
        rung(
            0,
            "idle_cap",
            idle_cap.into(),
            json!({ "idle_win_rate": idle }),
        ),
        rung(
            0,
            "effort_matters",
            effort_matters.into(),
            json!({ "best_win_rate": best }),
        ),
        rung(
            0,
            "exploring_alone_never_wins",
            explore_alone.into(),
            json!({ "explore_only_win_rate": explore_only, "rote_median_win_rate": rote_median }),
        ),
        rung(
            1,
            "ladder_rote",
            ladder_rote.into(),
            json!({ "mixed_mean_share": mixed_mean, "mixed_win_rate": mixed_rate }),
        ),
        rung(
            2,
            "ladder_learned",
            ladder_learned.into(),
            json!({ "explorer_minus_mixed_share": explorer - mixed_mean, "margin": LEARNED_MARGIN }),
        ),
        rung(
            2,
            "knowledge_matters",
            knowledge_matters.into(),
            json!({ "omniscient_mixed_mean_share": omni_mean, "mixed_mean_share": mixed_mean }),
        ),
        rung(
            3,
            "skill_gap",
            skill_gap.into(),
            json!({
                "share_gap": best_informed_mean - best_rote_mean,
                "win_rate_gap": best_informed_rate - best_rote_rate,
            }),
        ),
        rung(
            3,
            "good_styles_vary",
            styles["pass"].clone(),
            json!({ "a": styles["a"], "b": styles["b"], "c": styles["c"] }),
        ),
        rung(
            9,
            "corrupt_not_dominant",
            corrupt_ok.into(),
            json!({ "corrupt_minus_best_honest": corrupt - best }),
        ),
        rung(
            9,
            "proxy_not_optimal",
            proxy_not_optimal.into(),
            json!({ "directed_mean_share": directed, "informed_median_mean_share": informed_median }),
        ),
    ];
    json!({
        "seeds": seeds,
        "omniscient": omniscient(),
        "strategies": rows,
        "ladder": ladder,
        "idle_cap": idle_cap,
        "idle_win_rate": idle,
        "effort_matters": effort_matters,
        "explore_only_win_rate": explore_only,
        "exploring_alone_never_wins": explore_alone,
        "ladder_rote": ladder_rote,
        "explorer_minus_mixed_share": explorer - mixed_mean,
        "ladder_learned": ladder_learned,
        "ladder_learned_e8": ladder_learned_e8,
        "omniscient_mixed_mean_share": omni_mean,
        "knowledge_matters": knowledge_matters,
        "skill_gap": skill_gap,
        "good_styles_vary": styles["pass"],
        "good_styles_vary_b": styles["b"],
        "styles": styles,
        "families": families,
        "corrupt_minus_best_honest": corrupt - best,
        "corrupt_not_dominant": corrupt_ok,
        "proxy_not_optimal": proxy_not_optimal,
        "directed_minus_informed_median_share": directed - informed_median,
        "best_minus_median": best - median_of(&honest),
        "mix_vs_explorer": mix_vs_explorer,
        "best_win_rate_strategy": top_by(&rate),
        "best_mean_share_strategy": top_by(&mean),
        "skips": skips,
    })
}
