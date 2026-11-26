//! Topic 6, stuck states: runaway or absorbing states across seeds and
//! years (debt spirals, towns that never recover from a closure, a party
//! that never loses, unemployment pinned at a bound, towns draining away).

use super::*;
use crate::lab::runner::Driver;

pub fn run(cx: &Ctx) -> Result<Value, String> {
    let years = cx.years(30);
    let hs = histories(cx, years, Driver::Idle)?;
    let end = (years * 12) as usize;
    let mut towns = 0usize;
    let mut debt_spiral = Vec::new();
    let mut pinned_high = Vec::new();
    let mut pinned_low = 0usize;
    let mut drained = Vec::new();
    let mut boomed = Vec::new();
    let mut gov_pinned = 0usize;
    let mut no_recovery = Vec::new();
    let mut closures = 0usize;
    let mut one_party_town = 0usize;
    let mut one_party_nation = Vec::new();
    let mut long_crises = Vec::new();
    let mut nat_debt = Vec::new();
    let mut u_max: f64 = 0.0;
    let (mut stalled, mut town_months) = (0usize, 0usize);
    let mut rents = Rents::default();
    for h in &hs {
        let nt = h.months[0].len();
        towns += nt;
        for t in 0..nt {
            let s = |m: usize| &h.months[m.min(end)][t];
            let pop0 = s(0).pop.max(1.0);
            // Debt per head at the end against 10 years in, rising throughout the last five.
            let dph = |m: usize| s(m).debt / s(m).pop.max(1.0);
            let rising = (end.saturating_sub(60)..end)
                .step_by(12)
                .all(|m| dph(m + 12) >= dph(m));
            if dph(end) > 3.0 * dph(120).max(100.0) && rising {
                debt_spiral.push(json!({ "scenario": h.scenario, "seed": h.seed, "town": t, "debt_per_head": r1(dph(end)), "at_10y": r1(dph(120)) }));
            }
            // Unemployment pinned: two years or more at 25%+ (or under 1%).
            let mut run = 0;
            let mut best = 0;
            let mut low = 0;
            for m in 0..=end {
                u_max = u_max.max(s(m).u);
                town_months += 1;
                if s(m).starts < 0.2 {
                    stalled += 1;
                }
                if s(m).u >= 0.25 {
                    run += 1;
                    best = best.max(run);
                } else {
                    run = 0;
                }
                if s(m).u < 0.01 {
                    low += 1;
                }
            }
            if best >= 24 {
                pinned_high.push(json!({ "scenario": h.scenario, "seed": h.seed, "town": t, "months_at_25pct_plus": best }));
            }
            if low >= 24 {
                pinned_low += 1;
            }
            let ratio = s(end).pop / pop0;
            if ratio < 0.6 {
                drained.push(json!({ "scenario": h.scenario, "seed": h.seed, "town": t, "pop_ratio": r3(ratio), "start": pop0 }));
            }
            if ratio > 2.0 {
                boomed.push(json!({ "scenario": h.scenario, "seed": h.seed, "town": t, "pop_ratio": r3(ratio), "start": pop0 }));
            }
            if (0..=end)
                .filter(|m| s(*m).gov < 0.1 || s(*m).gov > 0.9)
                .count()
                >= 24
            {
                gov_pinned += 1;
            }
            // The executive's party never changes in 30 years.
            let p0 = s(0).exec_party;
            if p0 >= 0 && (0..=end).all(|m| s(m).exec_party == p0) {
                one_party_town += 1;
            }
        }
        rents.add(h, end);
        // Closures: is the town's unemployment back within 2 points of its
        // pre-closure level (relative to the nation's) after 5 and 10 years?
        for e in h.tally.events.iter().filter(|e| e.kind == "plant_closure") {
            let (Some(t), m) = (e.town, e.month as usize) else {
                continue;
            };
            closures += 1;
            if m + 120 > end || m < 1 {
                continue;
            }
            let rel = |k: usize| h.months[k][t as usize].u - h.nat_u[k];
            let pre = rel(m - 1);
            let peak = (m..m + 24).map(rel).fold(f64::MIN, f64::max);
            let after10 = rel(m + 120);
            if peak - pre > 0.01 && after10 - pre > 0.02 {
                no_recovery.push(json!({ "scenario": h.scenario, "seed": h.seed, "town": t, "month": m, "pre_rel_u_pts": r1(pre * 100.0), "peak": r1(peak * 100.0), "after_10y": r1(after10 * 100.0), "jobs": e.detail }));
            }
        }
        // Nation: one party holding most towns' executives all along.
        let holder = |m: usize| {
            let mut c = std::collections::BTreeMap::<i64, usize>::new();
            for l in &h.months[m] {
                *c.entry(l.exec_party).or_default() += 1;
            }
            c.into_iter().max_by_key(|x| x.1).map(|x| x.0).unwrap_or(-1)
        };
        let first = holder(0);
        let kept = (0..=end)
            .step_by(12)
            .filter(|m| holder(*m) == first)
            .count() as f64
            / (years + 1) as f64;
        one_party_nation.push(kept);
        for c in &h.tally.crises {
            let len = c.ended.unwrap_or(end as u32) - c.begun;
            if len >= 120 {
                long_crises.push(json!({ "scenario": h.scenario, "seed": h.seed, "town": c.town, "kind": c.kind, "months": len, "ended": c.ended.is_some() }));
            }
        }
        let d0 = h
            .years
            .first()
            .map(|s| s.nation.debt_pct_gdp)
            .unwrap_or(0.0);
        let d1 = h.years.last().map(|s| s.nation.debt_pct_gdp).unwrap_or(0.0);
        nat_debt.push(d1 - d0);
    }
    let share = |n: usize| r3(n as f64 / towns.max(1) as f64);
    let industries = industries(&hs, end);
    let checks = json!({
        "industries": industries,
        "debt_spiral_towns": { "count": debt_spiral.len(), "share": share(debt_spiral.len()), "verdict": if debt_spiral.is_empty() { "works" } else { "weak" } },
        "unemployment_25pct_for_2y": { "count": pinned_high.len(), "share": share(pinned_high.len()), "max_town_unemployment": r3(u_max), "verdict": if pinned_high.len() * 50 < towns { "works" } else { "broken" } },
        "unemployment_under_1pct_for_2y": { "count": pinned_low, "share": share(pinned_low) },
        "towns_drained_below_60pct": { "count": drained.len(), "share": share(drained.len()), "verdict": if drained.len() * 50 < towns { "works" } else { "broken" } },
        "towns_more_than_doubled": { "count": boomed.len(), "share": share(boomed.len()) },
        "burn_in_growth": burn_in_growth(cx)?,
        "rent_runaway_4x_median_town": { "count": rents.runaway.len(), "share": share(rents.runaway.len()), "verdict": if rents.runaway.is_empty() { "works" } else { "broken" } },
        "gov_approval_pinned_2y": { "count": gov_pinned, "share": share(gov_pinned) },
        "closures_without_recovery_10y": { "closures": closures, "count": no_recovery.len(), "verdict": if no_recovery.len() * 4 < closures.max(1) { "works" } else { "weak" } },
        "executive_party_never_changed_30y": { "count": one_party_town, "share": share(one_party_town), "verdict": if one_party_town * 5 < towns { "works" } else { "broken" } },
        "national_top_party_kept_share_of_years": { "median": r3(median(&one_party_nation)), "max": r3(quantile(&one_party_nation, 1.0)) },
        "crises_lasting_10y": { "count": long_crises.len(), "by_kind": long_crises.iter().fold(std::collections::BTreeMap::<String, usize>::new(), |mut m, c| { *m.entry(c["kind"].as_str().unwrap_or("").to_string()).or_default() += 1; m }), "crisis_spans_total": hs.iter().map(|h| h.tally.crises.len()).sum::<usize>() },
        "real_rent_growth_town": rents.real_growth(),
        "rent_burden": rents.burden(),
        "private_building_stalled_town_months": { "share": r3(stalled as f64 / town_months.max(1) as f64), "threshold": "starts under 20% of baseline" },
        "national_debt_pct_gdp_change": { "median": r3(median(&nat_debt)), "max": r3(quantile(&nat_debt, 1.0)), "min": r3(quantile(&nat_debt, 0.0)) },
    });
    let first = |v: &Vec<Value>| v.iter().take(12).cloned().collect::<Vec<_>>();
    Ok(json!({
        "params": { "seeds_per_scenario": cx.args.seeds, "years": years, "driver": "idle", "towns": towns, "scenarios": cx.scenarios().iter().map(short_scenario).collect::<Vec<_>>() },
        "checks": checks,
        "examples": {
            "debt_spiral": first(&debt_spiral),
            "pinned_high": first(&pinned_high),
            "drained": first(&drained),
            "boomed": first(&boomed),
            "rent_runaway": first(&rents.runaway),
            "no_recovery": first(&no_recovery),
            "long_crises": first(&long_crises),
        },
    }))
}

/// Industries and named plants over the run (DEC-110): no industry dead or
/// runaway, every closure explained, towns keeping their mixes, and total
/// employment and unemployment beside them.
fn industries(hs: &[History], end: usize) -> Value {
    use std::collections::BTreeMap;
    let mut ratios: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut sector_ratios: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut jobs_ratio = Vec::new();
    let mut u_end = Vec::new();
    let mut u_all = Vec::new();
    let (mut closures, mut named, mut explained) = (0usize, 0usize, 0usize);
    let mut by_factor: BTreeMap<String, usize> = BTreeMap::new();
    let mut steps: BTreeMap<String, usize> = BTreeMap::new();
    let mut unexplained_steps = 0usize;
    let (mut kept_top, mut with_mix) = (0usize, 0usize);
    let mut defining_kinds = Vec::new();
    let mut defining_max_share = Vec::new();
    let mut plants = [[0u32; 4]; 2];
    let mut jump = Vec::new();
    for h in hs {
        let (Some(a), Some(b)) = (h.years.first(), h.years.last()) else {
            continue;
        };
        let sum =
            |s: &crate::lab::metrics::Snapshot,
             f: &dyn Fn(&crate::lab::metrics::Town) -> &BTreeMap<String, f64>| {
                let mut m: BTreeMap<String, f64> = BTreeMap::new();
                for t in &s.towns {
                    for (k, v) in f(t) {
                        *m.entry(k.clone()).or_default() += v;
                    }
                }
                m
            };
        let (ia, ib) = (sum(a, &|t| &t.industries), sum(b, &|t| &t.industries));
        for (k, v) in &ia {
            if *v >= 1.0 {
                ratios
                    .entry(k.clone())
                    .or_default()
                    .push(ib.get(k).copied().unwrap_or(0.0) / v);
            }
        }
        let (sa, sb) = (sum(a, &|t| &t.jobs), sum(b, &|t| &t.jobs));
        for (k, v) in &sa {
            if *v >= 1.0 {
                sector_ratios
                    .entry(k.clone())
                    .or_default()
                    .push(sb.get(k).copied().unwrap_or(0.0) / v);
            }
        }
        jobs_ratio.push(sb.values().sum::<f64>() / sa.values().sum::<f64>().max(1.0));
        u_end.push(b.nation.unemployment);
        u_all.extend(h.nat_u.iter().copied());
        // Does a town's largest industry stay among its two largest?
        let top = |t: &crate::lab::metrics::Town, n: usize| -> Vec<String> {
            let mut v: Vec<(&String, &f64)> = t.industries.iter().collect();
            v.sort_by(|x, y| y.1.total_cmp(x.1).then(x.0.cmp(y.0)));
            v.into_iter().take(n).map(|x| x.0.clone()).collect()
        };
        let mut defining: BTreeMap<String, usize> = BTreeMap::new();
        for (ta, tb) in a.towns.iter().zip(&b.towns) {
            for (s, t) in [(0, ta), (1, tb)] {
                for i in 0..4 {
                    plants[s][i] += t.plants[i];
                }
            }
            if let Some(d) = top(ta, 1).first() {
                *defining.entry(d.clone()).or_default() += 1;
                if ta.industries.len() >= 2 {
                    with_mix += 1;
                    if top(tb, 2).contains(d) {
                        kept_top += 1;
                    }
                }
            }
        }
        defining_kinds.push(defining.len() as f64);
        defining_max_share.push(
            defining.values().copied().max().unwrap_or(0) as f64 / a.towns.len().max(1) as f64,
        );
        for e in &h.tally.events {
            let pressure = e.causes.iter().any(|c| c.starts_with("pressure"));
            match e.kind.as_str() {
                "plant_closure" => {
                    closures += 1;
                    let parts: Vec<&str> = e.detail.split(':').collect();
                    if parts.len() >= 4 {
                        named += 1;
                        *by_factor.entry(parts[3].to_string()).or_default() += 1;
                    }
                    if pressure {
                        explained += 1;
                    }
                    // The town's unemployment a year on against the month before.
                    if let Some(t) = e.town {
                        let m = e.month as usize;
                        if m >= 1 && m + 12 <= end {
                            let u = |k: usize| h.months[k][t as usize].u - h.nat_u[k];
                            jump.push((u(m + 12) - u(m - 1)) * 100.0);
                        }
                    }
                }
                k @ ("plant_short_time" | "plant_reviewed" | "plant_threatened"
                | "plant_reprieved" | "plant_opened") => {
                    *steps.entry(k.to_string()).or_default() += 1;
                    if !(pressure || e.cause_class == "process" || e.cause_class == "linked") {
                        unexplained_steps += 1;
                    }
                }
                _ => {}
            }
        }
    }
    let rows: BTreeMap<String, Value> = ratios
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                json!({ "worlds": v.len(), "median": r3(median(v)), "min": r3(quantile(v, 0.0)), "max": r3(quantile(v, 1.0)) }),
            )
        })
        .collect();
    let dead: Vec<&String> = ratios
        .iter()
        .filter(|(_, v)| !(0.35..=2.0).contains(&median(v)))
        .map(|(k, _)| k)
        .collect();
    let top_factor = by_factor.values().copied().max().unwrap_or(0);
    json!({
        "jobs_year_end_over_year_0_by_industry": rows,
        "dead_or_runaway": { "band": "median 0.35 to 2.0", "outside": dead, "verdict": if dead.is_empty() { "works" } else { "broken" } },
        "jobs_year_end_over_year_0_by_sector": sector_ratios.iter().map(|(k, v)| (k.clone(), json!(r3(median(v))))).collect::<BTreeMap<_, _>>(),
        "total_jobs_year_end_over_year_0": { "median": r3(median(&jobs_ratio)), "min": r3(quantile(&jobs_ratio, 0.0)), "max": r3(quantile(&jobs_ratio, 1.0)) },
        "national_unemployment": { "median_all_months": r3(median(&u_all)), "p95_all_months": r3(quantile(&u_all, 0.95)), "median_at_end": r3(median(&u_end)) },
        "closures": { "count": closures, "named_plant": named, "explained_by_a_pressure": explained, "explained_share": r3(explained as f64 / closures.max(1) as f64), "by_weakest_factor": by_factor, "top_factor_share": r3(top_factor as f64 / named.max(1) as f64), "verdict": if explained == closures { "works" } else { "broken" } },
        "review_steps": { "counts": steps, "without_a_cause": unexplained_steps },
        "town_unemployment_change_a_year_after_a_closure_pts": { "median": r3(median(&jump)), "p95": r3(quantile(&jump, 0.95)), "max": r3(quantile(&jump, 1.0)) },
        "plants_running_short_notice_closed": { "year_0": plants[0], "year_end": plants[1] },
        "towns_keep_their_mix": { "towns_with_two_or_more": with_mix, "largest_industry_still_in_top_two": kept_top, "share": r3(kept_top as f64 / with_mix.max(1) as f64), "verdict": if kept_top * 10 >= with_mix * 6 { "works" } else { "weak" } },
        "defining_industries_per_world": { "median": r3(median(&defining_kinds)), "min": r3(quantile(&defining_kinds, 0.0)), "largest_share_of_towns_median": r3(median(&defining_max_share)) },
    })
}

/// Town populations over the history burn-in: each audit world generated
/// with and without its burn-in (the same seed gives the same starting
/// towns). No town should grow more than 4× (a position-biased migration
/// once made the last town in the list grow tenfold).
fn burn_in_growth(cx: &Ctx) -> Result<Value, String> {
    use mandate_sim::world::MunicipalityId;
    use mandate_worldgen::{WorldConfig, WorldSize, generate};
    let mut bare = (*cx.content).clone();
    bare.tuning.burn_in_years = 0;
    let bare = std::sync::Arc::new(bare);
    let worlds = cx.worlds();
    let rows = crate::lab::runner::par_map(&worlds, crate::lab::runner::threads(), |(s, k)| {
        let pops = |c: &std::sync::Arc<mandate_content::ContentBundle>| {
            let cfg = WorldConfig {
                seed: *k,
                size: WorldSize::Small,
                scenario: s.clone(),
            };
            generate(&cfg, c.clone())
                .map_err(|e| e.to_string())
                .map(|e| {
                    let st = e.state();
                    (0..st.world.municipalities.len())
                        .map(|m| {
                            st.population
                                .municipality_total(&st.world, MunicipalityId(m as u32))
                                as f64
                        })
                        .collect::<Vec<_>>()
                })
        };
        let (a, b) = (pops(&bare)?, pops(&cx.content)?);
        Ok::<_, String>(
            a.iter()
                .zip(&b)
                .enumerate()
                .map(|(t, (a, b))| (b / a.max(1.0), short_scenario(s), *k, t))
                .collect::<Vec<_>>(),
        )
    });
    let mut all = Vec::new();
    for r in rows {
        all.extend(r?);
    }
    let over: Vec<Value> = all
        .iter()
        .filter(|x| x.0 > 4.0)
        .take(12)
        .map(|x| json!({ "ratio": r3(x.0), "scenario": x.1, "seed": x.2, "town": x.3 }))
        .collect();
    let count = all.iter().filter(|x| x.0 > 4.0).count();
    let ratios: Vec<f64> = all.iter().map(|x| x.0).collect();
    Ok(json!({
        "max": r3(quantile(&ratios, 1.0)),
        "median": r3(median(&ratios)),
        "over_4x": count,
        "examples": over,
        "verdict": if count == 0 { "works" } else { "broken" },
    }))
}

/// Rent over a run, per town: runaways (rent rising 4× more than the median
/// town's), real rent growth (deflated by the national price level) and the
/// rent burden (a year's rent over a middle-class income) at both ends.
#[derive(Default)]
struct Rents {
    runaway: Vec<Value>,
    real: Vec<f64>,
    start_burden: Vec<f64>,
    end_burden: Vec<f64>,
}

impl Rents {
    fn add(&mut self, h: &History, end: usize) {
        let (first, last) = (&h.months[0], &h.months[end]);
        let growth: Vec<f64> = first
            .iter()
            .zip(last)
            .map(|(a, b)| b.rent / a.rent.max(1e-9))
            .collect();
        let med = median(&growth);
        let pl =
            |s: Option<&crate::lab::metrics::Snapshot>| s.map_or(1.0, |s| s.nation.price_level);
        let inflation = pl(h.years.last()) / pl(h.years.first()).max(1e-9);
        self.real.extend(growth.iter().map(|g| g / inflation));
        self.start_burden
            .extend(first.iter().map(|l| l.rent_burden));
        self.end_burden.extend(last.iter().map(|l| l.rent_burden));
        for (t, g) in growth.iter().enumerate() {
            if *g > 4.0 * med {
                self.runaway.push(json!({ "scenario": h.scenario, "seed": h.seed, "town": t, "rent_ratio": r1(*g), "median_town_ratio": r3(med), "start_rent": r1(first[t].rent), "end_rent": r1(last[t].rent), "end_vacancy": r3(last[t].vacancy), "pop_ratio": r3(last[t].pop / first[t].pop.max(1.0)) }));
            }
        }
    }
    /// The median town's real rent growth is within ±50% over the run.
    fn real_ok(&self) -> bool {
        (0.5..=1.5).contains(&median(&self.real))
    }
    fn real_growth(&self) -> Value {
        json!({ "median": r3(median(&self.real)), "p95": r3(quantile(&self.real, 0.95)), "max": r3(quantile(&self.real, 1.0)), "verdict": if self.real_ok() { "works" } else { "broken" } })
    }
    fn burden(&self) -> Value {
        json!({ "start_median": r3(median(&self.start_burden)), "start_max": r3(quantile(&self.start_burden, 1.0)), "end_median": r3(median(&self.end_burden)), "end_p95": r3(quantile(&self.end_burden, 0.95)), "end_max": r3(quantile(&self.end_burden, 1.0)) })
    }
}

/// The nightly gate's rent sanity (report §"Nightly gate"): over `years`
/// on the six worlds × `seeds`, no town's rent rises 4× more than the
/// median town's, and the median town's real rent growth is within ±50%.
pub fn rent_gate(cx: &Ctx, seeds: u64, years: u32) -> Result<Value, String> {
    let sub = Ctx {
        content: cx.content.clone(),
        args: AuditArgs {
            seeds,
            years,
            scenario: vec![],
            ..cx.args.clone()
        },
    };
    let hs = histories(&sub, years, Driver::Idle)?;
    let end = (years * 12) as usize;
    let mut rents = Rents::default();
    for h in &hs {
        rents.add(h, end);
    }
    let pass = rents.runaway.is_empty() && rents.real_ok();
    Ok(json!({
        "pass": pass,
        "seeds_per_scenario": seeds,
        "years": years,
        "rent_runaway_4x_median_town": { "count": rents.runaway.len(), "examples": rents.runaway.iter().take(12).cloned().collect::<Vec<_>>() },
        "real_rent_growth_town": rents.real_growth(),
        "rent_burden": rents.burden(),
    }))
}
