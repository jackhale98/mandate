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
    let mut rent_runaway = Vec::new();
    let mut gov_pinned = 0usize;
    let mut no_recovery = Vec::new();
    let mut closures = 0usize;
    let mut one_party_town = 0usize;
    let mut one_party_nation = Vec::new();
    let mut long_crises = Vec::new();
    let mut nat_debt = Vec::new();
    let mut u_max: f64 = 0.0;
    let (mut stalled, mut town_months) = (0usize, 0usize);
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
        // Rent runaway: a town whose rent rose 4× more than the median town's.
        let growth: Vec<f64> = (0..nt)
            .map(|t| h.months[end][t].rent / h.months[0][t].rent.max(1e-9))
            .collect();
        let med = median(&growth);
        for (t, g) in growth.iter().enumerate() {
            if *g > 4.0 * med {
                rent_runaway.push(json!({ "scenario": h.scenario, "seed": h.seed, "town": t, "rent_ratio": r1(*g), "median_town_ratio": r3(med), "start_rent": r1(h.months[0][t].rent), "end_rent": r1(h.months[end][t].rent), "end_vacancy": r3(h.months[end][t].vacancy), "pop_ratio": r3(h.months[end][t].pop / h.months[0][t].pop.max(1.0)) }));
            }
        }
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
    let checks = json!({
        "debt_spiral_towns": { "count": debt_spiral.len(), "share": share(debt_spiral.len()), "verdict": if debt_spiral.is_empty() { "works" } else { "weak" } },
        "unemployment_25pct_for_2y": { "count": pinned_high.len(), "share": share(pinned_high.len()), "max_town_unemployment": r3(u_max), "verdict": if pinned_high.len() * 50 < towns { "works" } else { "broken" } },
        "unemployment_under_1pct_for_2y": { "count": pinned_low, "share": share(pinned_low) },
        "towns_drained_below_60pct": { "count": drained.len(), "share": share(drained.len()), "verdict": if drained.len() * 50 < towns { "works" } else { "broken" } },
        "towns_more_than_doubled": { "count": boomed.len(), "share": share(boomed.len()) },
        "rent_runaway_4x_median_town": { "count": rent_runaway.len(), "share": share(rent_runaway.len()), "verdict": if rent_runaway.is_empty() { "works" } else { "broken" } },
        "gov_approval_pinned_2y": { "count": gov_pinned, "share": share(gov_pinned) },
        "closures_without_recovery_10y": { "closures": closures, "count": no_recovery.len(), "verdict": if no_recovery.len() * 4 < closures.max(1) { "works" } else { "weak" } },
        "executive_party_never_changed_30y": { "count": one_party_town, "share": share(one_party_town), "verdict": if one_party_town * 5 < towns { "works" } else { "broken" } },
        "national_top_party_kept_share_of_years": { "median": r3(median(&one_party_nation)), "max": r3(quantile(&one_party_nation, 1.0)) },
        "crises_lasting_10y": { "count": long_crises.len(), "by_kind": long_crises.iter().fold(std::collections::BTreeMap::<String, usize>::new(), |mut m, c| { *m.entry(c["kind"].as_str().unwrap_or("").to_string()).or_default() += 1; m }), "crisis_spans_total": hs.iter().map(|h| h.tally.crises.len()).sum::<usize>() },
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
            "rent_runaway": first(&rent_runaway),
            "no_recovery": first(&no_recovery),
            "long_crises": first(&long_crises),
        },
    }))
}
