//! Topic 7, readability: when something large changes (a town's
//! unemployment or rent in a year; an incumbent losing), do the engine's own
//! explanation surfaces name the cause the twins show?
//!
//! The surfaces: events and their causes (the causal spine, S1), town
//! crises, a law's attribution ledger (the bill's "Why?"), and the
//! election-night factor decomposition ("Why it went this way").

use super::*;
use crate::lab::metrics::Election;
use crate::lab::runner::{Driver, Game, Intervention, Setup, par_map, threads};
use std::collections::BTreeMap;

/// Large changes in baseline histories and what the surfaces show for them.
fn large_changes(hs: &[History]) -> Value {
    let (mut n_u, mut ex_u) = (0usize, BTreeMap::<&str, usize>::new());
    let (mut n_r, mut ex_r) = (0usize, BTreeMap::<&str, usize>::new());
    let (mut n_l, mut ex_l) = (0usize, BTreeMap::<String, usize>::new());
    for h in hs {
        for (y, s) in h.years.iter().enumerate().skip(1) {
            let m = y * 12;
            let in_window = |kind: &str, t: Option<u32>| {
                h.tally.events.iter().any(|e| {
                    e.kind == kind
                        && (e.month as usize) > m - 12
                        && (e.month as usize) <= m
                        && (t.is_none() || e.town == t)
                })
            };
            let recession = in_window("recession", None)
                || s.nation.in_recession
                || h.years[y - 1].nation.in_recession;
            for (t, town) in s.towns.iter().enumerate() {
                let (a, b) = (&h.months[m - 12][t], &h.months[m][t]);
                let tt = Some(t as u32);
                if b.u - a.u >= 0.03 {
                    n_u += 1;
                    let cause = if in_window("plant_closure", tt) {
                        "plant_closure_event"
                    } else if in_window("disaster", tt) {
                        "disaster_event"
                    } else if !town.crises.is_empty() {
                        "town_crisis"
                    } else if recession {
                        "national_recession_only"
                    } else {
                        "nothing_shown"
                    };
                    *ex_u.entry(cause).or_default() += 1;
                }
                let infl = s.nation.inflation;
                if a.rent > 0.0 && (b.rent / a.rent - 1.0 - infl) >= 0.10 {
                    n_r += 1;
                    let cause = if town.crises.iter().any(|c| c == "crisis.housing_squeeze") {
                        "housing_squeeze_crisis"
                    } else if in_window("disaster", tt) {
                        "disaster_event"
                    } else {
                        "nothing_shown"
                    };
                    *ex_r.entry(cause).or_default() += 1;
                }
            }
        }
        for e in h
            .tally
            .elections
            .iter()
            .filter(|e| e.town.is_some() && e.incumbent_ran && !e.incumbent_won)
        {
            n_l += 1;
            let top = e
                .decomposition
                .iter()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|x| x.0.clone())
                .unwrap_or_else(|| "no_decomposition".into());
            *ex_l.entry(top).or_default() += 1;
        }
    }
    let shown = |m: &BTreeMap<&str, usize>, n: usize| {
        let none = m.get("nothing_shown").copied().unwrap_or(0)
            + m.get("national_recession_only").copied().unwrap_or(0);
        r3(1.0 - none as f64 / n.max(1) as f64)
    };
    json!({
        "unemployment_up_3pts_in_a_year": { "cases": n_u, "by_surface": ex_u, "share_with_local_cause_shown": shown(&ex_u, n_u) },
        "real_rent_up_10pct_in_a_year": { "cases": n_r, "by_surface": ex_r, "share_with_cause_shown": shown(&ex_r, n_r) },
        "incumbent_lost": { "cases": n_l, "largest_factor_named": ex_l },
    })
}

/// The bill's "Why?" ledger against the twin-measured effect, at month 36.
fn ledgers(cx: &Ctx) -> Result<Value, String> {
    // (template, ledger key, twin metric, ledger-to-twin unit)
    let checks: [(&str, &str, &str); 6] = [
        ("policy.affordable_housing", "homes_built", "housing_stock"),
        ("policy.rebuild_fund", "homes_built", "housing_stock"),
        (
            "policy.small_business_grants",
            "jobs_supported_retail",
            "jobs.retail",
        ),
        (
            "policy.tech_incubator",
            "jobs_supported_knowledge",
            "jobs.knowledge",
        ),
        (
            "policy.closure_response",
            "jobs_supported_manufacturing",
            "jobs.manufacturing",
        ),
        (
            "policy.local_wage_floor",
            "wage_uplift_pct",
            "income_low_pct",
        ),
    ];
    let worlds = cx.worlds();
    let res = par_map(
        &worlds,
        threads(),
        |(s, seed)| -> Result<Vec<(usize, f64, f64)>, String> {
            let mut g = Game::new(
                &cx.content,
                &Setup::small(*seed, s.as_deref()),
                Driver::Idle,
            )?;
            g.run_to(1, &[], &mut Vec::new());
            let home = g.home() as usize;
            let mut b = g.fork()?;
            let mut sb = Vec::new();
            b.run_to(36, &[36], &mut sb);
            let mut out = Vec::new();
            for (k, (tpl, key, metric)) in checks.iter().enumerate() {
                let mut x = g.fork()?;
                let id = x.e.state().policy.instances.len();
                x.intervene(&Intervention::Policy {
                    template: tpl.to_string(),
                    town: home as u32,
                    params: Default::default(),
                })?;
                let mut sx = Vec::new();
                x.run_to(36, &[36], &mut sx);
                let ledger =
                    x.e.state()
                        .policy
                        .instances
                        .get(id)
                        .and_then(|p| p.ledger.get(*key))
                        .copied()
                        .unwrap_or(0.0);
                let (ta, tb) = (&sb[0].towns[home], &sx[0].towns[home]);
                let twin = match *metric {
                    "housing_stock" => tb.housing_stock - ta.housing_stock,
                    "income_low_pct" => pct(ta.income[0], tb.income[0]),
                    m => {
                        let k = m.trim_start_matches("jobs.");
                        tb.jobs.get(k).copied().unwrap_or(0.0)
                            - ta.jobs.get(k).copied().unwrap_or(0.0)
                    }
                };
                out.push((k, ledger, twin));
            }
            Ok(out)
        },
    );
    let mut by: Vec<(Vec<f64>, Vec<f64>)> = vec![(Vec::new(), Vec::new()); checks.len()];
    for r in res {
        for (k, l, t) in r? {
            by[k].0.push(l);
            by[k].1.push(t);
        }
    }
    let rows: Vec<Value> = checks
        .iter()
        .zip(&by)
        .map(|((tpl, key, metric), (l, t))| {
            let ratio = mean(t) / mean(l).abs().max(1e-9);
            json!({
                "template": tpl, "ledger_key": key, "twin_metric": metric,
                "ledger_mean": r3(mean(l)), "twin_mean": r3(mean(t)), "twin_over_ledger": r3(ratio),
                "readable": (0.5..=2.0).contains(&ratio),
            })
        })
        .collect();
    let ok = rows
        .iter()
        .filter(|r| r["readable"].as_bool().unwrap_or(false))
        .count();
    Ok(json!({ "rows": rows, "share_readable": r3(ok as f64 / rows.len().max(1) as f64) }))
}

/// Planted closures that flip a race: which election-night factor carries
/// the change (the twins say the cause is the closure, so `record`, where
/// unemployment and approval enter, should).
fn flipped_races(cx: &Ctx) -> Result<Value, String> {
    let worlds = cx.worlds();
    let res = par_map(
        &worlds,
        threads(),
        |(s, seed)| -> Result<Vec<String>, String> {
            let mut g = Game::new(
                &cx.content,
                &Setup::small(*seed, s.as_deref()),
                Driver::Idle,
            )?;
            g.run_to(24, &[], &mut Vec::new());
            let home = g.home();
            let mut a = g.fork()?;
            let mut b = g.fork()?;
            b.intervene(&Intervention::PlantClosure {
                town: home,
                sector: None,
                share: 0.5,
            })?;
            a.run_to(72, &[], &mut Vec::new());
            b.run_to(72, &[], &mut Vec::new());
            let key = |e: &Election| format!("{}{}{:?}", e.date, e.post, e.constituency);
            let mut out = Vec::new();
            for ea in a
                .tally
                .elections
                .iter()
                .filter(|e| e.town == Some(home) && e.month > 24)
            {
                let Some(eb) = b.tally.elections.iter().find(|e| key(e) == key(ea)) else {
                    continue;
                };
                if eb.winner == ea.winner {
                    continue;
                }
                // Base winner's lead per factor in each arm (only when the base
                // winner is the closure arm's runner-up).
                if eb.runner_up != Some(ea.winner) {
                    out.push("not_comparable".into());
                    continue;
                }
                let top = ea
                    .decomposition
                    .iter()
                    .map(|(f, x)| {
                        (
                            f.clone(),
                            x + eb.decomposition.get(f).copied().unwrap_or(0.0),
                        )
                    })
                    .max_by(|x, y| x.1.abs().total_cmp(&y.1.abs()))
                    .map(|x| x.0)
                    .unwrap_or_else(|| "none".into());
                out.push(top);
            }
            Ok(out)
        },
    );
    let mut count = BTreeMap::<String, usize>::new();
    let mut n = 0;
    for r in res {
        for k in r? {
            n += 1;
            *count.entry(k).or_default() += 1;
        }
    }
    let record = count.get("record").copied().unwrap_or(0);
    Ok(
        json!({ "flipped_races": n, "factor_that_moved_most": count, "share_record": r3(record as f64 / n.max(1) as f64) }),
    )
}

/// The causal spine: which kinds of event name a cause.
fn spine(hs: &[History]) -> Value {
    let mut by = BTreeMap::<String, (usize, usize)>::new();
    for e in hs.iter().flat_map(|h| h.tally.events.iter()) {
        let x = by.entry(e.kind.clone()).or_default();
        x.0 += 1;
        if !e.causes.is_empty() {
            x.1 += 1;
        }
    }
    let rows: serde_json::Map<String, Value> = by
        .into_iter()
        .map(|(k, (n, c))| {
            (
                k,
                json!({ "events": n, "with_cause": c, "share": r3(c as f64 / n.max(1) as f64) }),
            )
        })
        .collect();
    Value::Object(rows)
}

pub fn run(cx: &Ctx) -> Result<Value, String> {
    let years = cx.years(20);
    let hs = histories(cx, years, Driver::Idle)?;
    let lc = large_changes(&hs);
    let led = ledgers(cx)?;
    let fl = flipped_races(cx)?;
    let u = lc["unemployment_up_3pts_in_a_year"]["share_with_local_cause_shown"]
        .as_f64()
        .unwrap_or(0.0);
    let r = lc["real_rent_up_10pct_in_a_year"]["share_with_cause_shown"]
        .as_f64()
        .unwrap_or(0.0);
    Ok(json!({
        "params": { "seeds_per_scenario": cx.args.seeds, "years": years, "driver": "idle", "scenarios": cx.scenarios().iter().map(short_scenario).collect::<Vec<_>>() },
        "large_changes": lc,
        "ledger_vs_twin": led,
        "flipped_by_closure": fl,
        "causal_spine": spine(&hs),
        "verdict_unemployment": if u >= 0.7 { "works" } else if u >= 0.4 { "weak" } else { "broken" },
        "verdict_rent": if r >= 0.7 { "works" } else if r >= 0.4 { "weak" } else { "broken" },
    }))
}
