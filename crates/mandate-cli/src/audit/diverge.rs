//! Topic 4, towns diverging: do towns stay distinct over 20–30 years
//! (industry, wealth, politics, worries), or drift to one average?

use super::*;
use crate::lab::metrics::Snapshot;
use crate::lab::runner::Driver;

/// Spread measures across one world's towns at one sample.
fn spread(s: &Snapshot, atts: &[Vec<f64>], pop0: &[f64]) -> [f64; 8] {
    let ts = &s.towns;
    let cv = |f: &dyn Fn(&crate::lab::metrics::Town) -> f64| {
        let v: Vec<f64> = ts.iter().map(f).collect();
        sd(&v) / mean(&v).abs().max(1e-9)
    };
    // Industry: the mean dissimilarity index between a town's job shares
    // and the nation's (0 = every town has the national mix).
    let mut nat = std::collections::BTreeMap::<&str, f64>::new();
    for t in ts {
        for (k, v) in &t.jobs {
            *nat.entry(k).or_default() += v;
        }
    }
    let ntot: f64 = nat.values().sum::<f64>().max(1.0);
    let dis = mean(
        &ts.iter()
            .map(|t| {
                let tot: f64 = t.jobs.values().sum::<f64>().max(1.0);
                0.5 * t
                    .jobs
                    .iter()
                    .map(|(k, v)| {
                        (v / tot - nat.get(k.as_str()).copied().unwrap_or(0.0) / ntot).abs()
                    })
                    .sum::<f64>()
            })
            .collect::<Vec<_>>(),
    );
    // Politics: attitude spread across town means (mean over attitudes).
    let na = atts.first().map(|a| a.len()).unwrap_or(0);
    let att = mean(
        &(0..na)
            .map(|k| sd(&atts.iter().map(|a| a[k]).collect::<Vec<_>>()))
            .collect::<Vec<_>>(),
    );
    // Executives: share of towns not run by the most common party.
    let mut parties = std::collections::BTreeMap::<i64, usize>::new();
    for t in ts {
        *parties
            .entry(t.executive_party.map(|p| p as i64).unwrap_or(-1))
            .or_default() += 1;
    }
    let top = parties.values().copied().max().unwrap_or(0);
    let other_party = 1.0 - top as f64 / ts.len().max(1) as f64;
    // Worries: the spread of each worry's level across towns.
    let keys: Vec<String> = ts
        .first()
        .map(|t| t.concerns.keys().cloned().collect())
        .unwrap_or_default();
    let worry = mean(
        &keys
            .iter()
            .map(|k| {
                sd(&ts
                    .iter()
                    .map(|t| t.concerns.get(k).copied().unwrap_or(0.0))
                    .collect::<Vec<_>>())
            })
            .collect::<Vec<_>>(),
    );
    let growth: Vec<f64> = ts
        .iter()
        .zip(pop0)
        .map(|(t, p0)| pct(*p0, t.population as f64))
        .collect();
    [
        cv(&|t| t.mean_wage),
        cv(&|t| t.rent),
        sd(&ts
            .iter()
            .map(|t| t.unemployment * 100.0)
            .collect::<Vec<_>>()),
        dis,
        att,
        other_party,
        worry,
        sd(&growth),
    ]
}

const NAMES: [&str; 8] = [
    "wage_cv",
    "rent_cv",
    "unemployment_sd_pts",
    "industry_dissimilarity",
    "attitude_sd",
    "towns_not_run_by_top_party",
    "worry_sd",
    "pop_growth_sd_pct",
];

/// Spearman rank correlation.
fn spearman(a: &[f64], b: &[f64]) -> f64 {
    let rank = |v: &[f64]| {
        let mut idx: Vec<usize> = (0..v.len()).collect();
        idx.sort_by(|x, y| v[*x].total_cmp(&v[*y]));
        let mut r = vec![0.0; v.len()];
        for (k, i) in idx.into_iter().enumerate() {
            r[i] = k as f64;
        }
        r
    };
    corr(&rank(a), &rank(b))
}

pub fn run(cx: &Ctx) -> Result<Value, String> {
    let years = cx.years(30);
    let hs = histories(cx, years, Driver::Idle)?;
    let checkpoints: Vec<u32> = [0, 5, 10, 20, 30]
        .into_iter()
        .filter(|y| *y <= years)
        .collect();
    let mut table = Vec::new();
    let mut at0 = [0.0; 8];
    let mut at_end = [0.0; 8];
    for &y in &checkpoints {
        let mut acc: Vec<Vec<f64>> = vec![Vec::new(); 8];
        for h in &hs {
            let Some(s) = h.years.get(y as usize) else {
                continue;
            };
            let pop0: Vec<f64> = h.years[0]
                .towns
                .iter()
                .map(|t| t.population as f64)
                .collect();
            let v = spread(s, &h.attitudes[y as usize], &pop0);
            for k in 0..8 {
                acc[k].push(v[k]);
            }
        }
        let mut row = serde_json::Map::new();
        row.insert("year".into(), json!(y));
        for k in 0..8 {
            let m = mean(&acc[k]);
            if y == 0 {
                at0[k] = m;
            }
            at_end[k] = m;
            row.insert(NAMES[k].into(), json!(r3(m)));
        }
        table.push(Value::Object(row));
    }
    // Identity: do the rich towns stay rich, the industrial stay industrial?
    let last = *checkpoints.last().unwrap_or(&0) as usize;
    let keep = |f: &dyn Fn(&crate::lab::metrics::Town) -> f64| {
        mean(
            &hs.iter()
                .map(|h| {
                    let a: Vec<f64> = h.years[0].towns.iter().map(f).collect();
                    let b: Vec<f64> = h.years[last].towns.iter().map(f).collect();
                    spearman(&a, &b)
                })
                .collect::<Vec<_>>(),
        )
    };
    let share = |k: &str| {
        let k = k.to_string();
        move |t: &crate::lab::metrics::Town| {
            t.jobs.get(&k).copied().unwrap_or(0.0) / t.jobs.values().sum::<f64>().max(1.0)
        }
    };
    let persistence = json!({
        "wage_rank": r3(keep(&|t| t.mean_wage)),
        "unemployment_rank": r3(keep(&|t| t.unemployment)),
        "manufacturing_share_rank": r3(keep(&share("manufacturing"))),
        "knowledge_share_rank": r3(keep(&share("knowledge"))),
        "population_rank": r3(keep(&|t| t.population as f64)),
    });
    // Ratio end / start per measure: below 0.5 is convergence.
    let ratios: serde_json::Map<String, Value> = (0..8)
        .map(|k| {
            (
                NAMES[k].to_string(),
                json!(r3(at_end[k] / at0[k].abs().max(1e-9))),
            )
        })
        .collect();
    let converging: Vec<&str> = (0..8)
        .filter(|k| at_end[*k] < 0.5 * at0[*k] && *k != 5 && *k != 7)
        .map(|k| NAMES[k])
        .collect();
    Ok(json!({
        "params": { "seeds_per_scenario": cx.args.seeds, "years": years, "driver": "idle", "scenarios": cx.scenarios().iter().map(short_scenario).collect::<Vec<_>>() },
        "spread_by_year": table,
        "end_over_start": ratios,
        "rank_persistence_spearman": persistence,
        "converging": converging,
        "verdict": if converging.is_empty() { "works" } else if converging.len() <= 2 { "weak" } else { "broken" },
    }))
}
