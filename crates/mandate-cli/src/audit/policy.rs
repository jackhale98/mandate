//! Topic 1, policy responsiveness: every policy template, enacted at its
//! defaults in the player's home town at month 1, against its twin without
//! it; what moves at 1, 3 and 5 years, compared with what the template says
//! it does (its mechanisms and side effects).

use super::*;
use crate::lab::metrics::Town;
use crate::lab::runner::{Driver, Game, Intervention, Setup, par_map, threads};
use mandate_content::{MechanismDef, PolicyTemplate, SideEffectDef};
use std::collections::BTreeMap;

/// Home-town deltas (treated − base) in readable units.
pub fn town_delta(b: &Town, t: &Town) -> BTreeMap<String, f64> {
    let mut m = BTreeMap::new();
    let mut put = |k: &str, v: f64| {
        m.insert(k.to_string(), v);
    };
    put(
        "unemployment_pts",
        (t.unemployment - b.unemployment) * 100.0,
    );
    put("wage_pct", pct(b.mean_wage, t.mean_wage));
    put("income_low_pct", pct(b.income[0], t.income[0]));
    put("income_middle_pct", pct(b.income[1], t.income[1]));
    put("income_high_pct", pct(b.income[2], t.income[2]));
    put("real_wage_pct", pct(b.real_wage, t.real_wage));
    put("rent_pct", pct(b.rent, t.rent));
    put("rent_burden_pts", (t.rent_burden - b.rent_burden) * 100.0);
    put("vacancy_pts", (t.vacancy - b.vacancy) * 100.0);
    put("starts_index", t.starts_index - b.starts_index);
    put("housing_stock_pct", pct(b.housing_stock, t.housing_stock));
    put(
        "population_pct",
        pct(b.population as f64, t.population as f64),
    );
    put("labour_force_pct", pct(b.labour_force, t.labour_force));
    put("output_pct", pct(b.output, t.output));
    let jb: f64 = b.jobs.values().sum();
    let jt: f64 = t.jobs.values().sum();
    put("jobs_pct", pct(jb, jt));
    for (k, v) in &b.jobs {
        put(
            &format!("jobs_{k}_pct"),
            pct(*v, t.jobs.get(k).copied().unwrap_or(0.0)),
        );
    }
    put(
        "gov_approval_pts",
        (t.gov_approval - b.gov_approval) * 100.0,
    );
    put("attractiveness", t.attractiveness - b.attractiveness);
    for (k, v) in &b.concerns {
        put(
            &format!("worry_{k}_pts"),
            (t.concerns.get(k).copied().unwrap_or(0.0) - v) * 100.0,
        );
    }
    let pop = (b.population as f64).max(1.0);
    put("cash_per_head", (t.cash - b.cash) / pop);
    put("debt_per_head", (t.debt - b.debt) / pop);
    m
}

/// What a template says it does, as (metric, expected sign, why).
pub fn expectations(t: &PolicyTemplate) -> Vec<(String, f64, String)> {
    let mut v: Vec<(String, f64, String)> = Vec::new();
    let mut add = |k: &str, s: f64, why: &str| {
        if !v.iter().any(|(x, _, _)| x == k) {
            v.push((k.to_string(), s, why.to_string()));
        }
    };
    for m in &t.mechanisms {
        match m {
            MechanismDef::Service { .. } => {
                add(
                    "gov_approval_pts",
                    1.0,
                    "service: residents feel better services",
                );
                add("attractiveness", 1.0, "service: the town draws people");
            }
            MechanismDef::HousingSupply { .. } => {
                add("housing_stock_pct", 1.0, "housing_supply: homes built");
                add(
                    "gov_approval_low_pts",
                    1.0,
                    "housing_supply: those it houses feel the homes added",
                );
                add(
                    "rent_pct",
                    -1.0,
                    "housing_supply: rents ease as supply grows",
                );
                add(
                    "worry_housing_pts",
                    -1.0,
                    "housing_supply: the housing worry eases",
                );
            }
            MechanismDef::Zoning { per_unit, .. } => {
                let s = per_unit.signum();
                add("housing_stock_pct", s, "zoning: private building changes");
                add("rent_pct", -s, "zoning: rents follow supply");
            }
            MechanismDef::RentCap { .. } => {
                add(
                    "rent_pct",
                    -1.0,
                    "rent_cap: rents held down while it is in force",
                );
                add(
                    "housing_stock_pct",
                    -1.0,
                    "rent_cap: returns fall, so fewer homes are started",
                );
            }
            MechanismDef::SectorSubsidy { sector, .. } => {
                let k = sector.trim_start_matches("sector.");
                add(
                    &format!("jobs_{k}_pct"),
                    1.0,
                    "sector_subsidy: jobs in the sector",
                );
                add(
                    "unemployment_pts",
                    -1.0,
                    "sector_subsidy: fewer out of work",
                );
            }
            MechanismDef::SectorCapacity { sector, .. } => {
                let k = sector.trim_start_matches("sector.");
                add(&format!("jobs_{k}_pct"), 1.0, "sector_capacity: jobs added");
            }
            MechanismDef::Transfer { class, .. } => {
                let name = CLASS_NAMES[mandate_sim::policy::class_index(*class)];
                add(
                    &format!("income_{name}_pct"),
                    1.0,
                    "transfer: recipients' income rises",
                );
                // The recipients' own approval (audit fix 3): the claim is
                // about them, not the town (a levy with a rebate can cost
                // the town's approval and still leave the rebated better off).
                add(
                    &format!("gov_approval_{name}_pts"),
                    1.0,
                    "transfer: recipients better off",
                );
            }
            MechanismDef::WageFloor { .. } => {
                add("income_low_pct", 1.0, "wage_floor: low pay rises");
                add("jobs_pct", -1.0, "wage_floor: some low-paid jobs go");
                add(
                    "gov_approval_low_pts",
                    1.0,
                    "wage_floor: the low-paid feel the rise",
                );
            }
            MechanismDef::LocalTaxRate { .. } => {
                let s = t
                    .params
                    .iter()
                    .find(|p| p.id == "points")
                    .map(|p| p.default.signum())
                    .unwrap_or(1.0);
                add("cash_per_head", s, "local_tax_rate: revenue");
                add("worry_taxes_pts", s, "local_tax_rate: the tax worry");
            }
            MechanismDef::Borrow { .. } => {
                add("debt_per_head", 1.0, "borrow: debt grows");
            }
            MechanismDef::HazardProtection { .. } | MechanismDef::Revenue { .. } => {}
        }
    }
    for fx in &t.side_effects {
        match fx {
            SideEffectDef::LabourSupply { .. } => {
                add("labour_force_pct", -1.0, "labour_supply side effect")
            }
            SideEffectDef::Disruption { .. } | SideEffectDef::Grievance { .. } => {}
        }
    }
    v
}

#[derive(Clone, Debug, Default)]
struct Row {
    template: String,
    scenario: String,
    class: String,
    /// sample month → metric deltas
    at: BTreeMap<u32, BTreeMap<String, f64>>,
    funded: f64,
    progress: f64,
    races: u32,
    flipped: u32,
    /// With a flood at month 30 in both arms (hazard templates only).
    flood: Option<BTreeMap<String, f64>>,
}

const SAMPLES: [u32; 3] = [12, 36, 60];

/// The town government's approval by income class (low, middle, high),
/// count-weighted over the town's neighbourhoods.
fn class_approval(g: &Game, town: u32) -> [f64; 3] {
    let st = g.e.state();
    let mut acc = [0.0; 3];
    let mut n = [0.0; 3];
    if let Some(mu) = st.world.municipalities.get(town as usize) {
        for h in &mu.neighborhoods {
            for (ci, c) in st.population.hoods[h.index()].cohorts.iter().enumerate() {
                let k = mandate_sim::population::class_of(ci);
                acc[k] += c.gov_approval as f64 * c.count as f64;
                n[k] += c.count as f64;
            }
        }
    }
    std::array::from_fn(|k| if n[k] > 0.0 { acc[k] / n[k] } else { 0.0 })
}

/// Run to `months`, sampling at `samples`, with each sample's class
/// approvals in the home town.
fn run_sampled(
    g: &mut Game,
    months: u32,
    samples: &[u32],
    town: u32,
) -> (Vec<crate::lab::metrics::Snapshot>, Vec<[f64; 3]>) {
    let mut snaps = Vec::new();
    let mut classes = Vec::new();
    for &m in samples {
        g.run_to(m, samples, &mut snaps);
        classes.push(class_approval(g, town));
    }
    g.run_to(months, &[], &mut Vec::new());
    (snaps, classes)
}

const CLASS_NAMES: [&str; 3] = ["low", "middle", "high"];

fn put_classes(m: &mut BTreeMap<String, f64>, b: &[f64; 3], t: &[f64; 3]) {
    for (k, name) in CLASS_NAMES.iter().enumerate() {
        m.insert(format!("gov_approval_{name}_pts"), (t[k] - b[k]) * 100.0);
    }
}

fn home_town(g: &Game, s: &crate::lab::metrics::Snapshot) -> Town {
    s.towns.get(g.home() as usize).cloned().unwrap_or_default()
}

fn world(
    cx: &Ctx,
    scenario: &Option<String>,
    seed: u64,
    templates: &[String],
    months: u32,
) -> Result<Vec<Row>, String> {
    let content = &cx.content;
    let mut g = Game::new(
        content,
        &Setup::small(seed, scenario.as_deref()),
        Driver::Idle,
    )?;
    g.run_to(1, &[], &mut Vec::new());
    let home = g.home();
    let samples: Vec<u32> = SAMPLES.iter().copied().filter(|m| *m <= months).collect();
    let mut b = g.fork()?;
    let (base, base_classes) = run_sampled(&mut b, months, &samples, home);
    let class = base
        .first()
        .and_then(|s| s.towns.get(home as usize))
        .map(|t| t.class.clone())
        .unwrap_or_default();
    let home_races = |g: &Game| -> Vec<(String, u32)> {
        g.tally
            .elections
            .iter()
            .filter(|e| e.town == Some(home))
            .map(|e| {
                (
                    e.date.clone() + &e.post + &format!("{:?}", e.constituency),
                    e.winner,
                )
            })
            .collect()
    };
    let base_races = home_races(&b);
    let mut flood_base: Option<Town> = None;
    let mut out = Vec::new();
    for tpl in templates {
        let Some(t) = content.policies.get(tpl.as_str()) else {
            continue;
        };
        let iv = Intervention::Policy {
            template: tpl.clone(),
            town: home,
            params: BTreeMap::new(),
        };
        let mut x = g.fork()?;
        let id = x.e.state().policy.instances.len();
        x.intervene(&iv)?;
        let (treated, classes) = run_sampled(&mut x, months, &samples, home);
        let st = x.e.state();
        let inst = st.policy.instances.get(id);
        let funded = inst.map(|p| st.finance.funded_share(p.id.0)).unwrap_or(0.0);
        let progress = inst.map(|p| p.progress as f64).unwrap_or(0.0);
        let tr = home_races(&x);
        let flipped = base_races
            .iter()
            .filter(|(k, w)| tr.iter().any(|(k2, w2)| k2 == k && w2 != w))
            .count() as u32;
        let mut at = BTreeMap::new();
        for (i, (sb, sx)) in base.iter().zip(&treated).enumerate() {
            let mut d = town_delta(&home_town(&b, sb), &home_town(&x, sx));
            if let (Some(cb), Some(cx)) = (base_classes.get(i), classes.get(i)) {
                put_classes(&mut d, cb, cx);
            }
            at.insert(sb.month, d);
        }
        // Protection only shows when the hazard strikes: a flood at month
        // 30 in both arms, read at 36.
        let protects = t
            .mechanisms
            .iter()
            .any(|m| matches!(m, MechanismDef::HazardProtection { .. }));
        let flood = if protects && months >= 36 {
            let flood = Intervention::Disaster {
                town: home,
                hazard: "flood".into(),
                severity: 0.8,
            };
            if flood_base.is_none() {
                let mut fb = g.fork()?;
                fb.run_to(30, &[], &mut Vec::new());
                fb.intervene(&flood)?;
                let mut s = Vec::new();
                fb.run_to(36, &[36], &mut s);
                flood_base = s.first().map(|s| home_town(&fb, s));
            }
            let mut fx = g.fork()?;
            fx.intervene(&iv)?;
            fx.run_to(30, &[], &mut Vec::new());
            fx.intervene(&flood)?;
            let mut s = Vec::new();
            fx.run_to(36, &[36], &mut s);
            match (&flood_base, s.first()) {
                (Some(fb), Some(s)) => Some(town_delta(fb, &home_town(&fx, s))),
                _ => None,
            }
        } else {
            None
        };
        out.push(Row {
            template: tpl.clone(),
            scenario: short_scenario(scenario),
            class: class.clone(),
            at,
            funded,
            progress,
            races: base_races.len() as u32,
            flipped,
            flood,
        });
    }
    Ok(out)
}

fn templates(cx: &Ctx) -> Vec<String> {
    cx.content
        .policies
        .keys()
        .map(|k| k.to_string())
        .filter(|k| cx.args.only.is_empty() || cx.args.only.iter().any(|o| k.contains(o.as_str())))
        .collect()
}

fn rows(cx: &Ctx, months: u32) -> Result<Vec<Row>, String> {
    let tpls = templates(cx);
    let worlds = cx.worlds();
    let res = par_map(&worlds, threads(), |(s, seed)| {
        world(cx, s, *seed, &tpls, months)
    });
    let mut all = Vec::new();
    for r in res {
        all.extend(r?);
    }
    Ok(all)
}

/// Metrics shown in the summary table for every template.
const KEY: [&str; 27] = [
    "jobs_agriculture_pct",
    "jobs_construction_pct",
    "jobs_knowledge_pct",
    "jobs_manufacturing_pct",
    "jobs_public_pct",
    "jobs_resources_pct",
    "jobs_retail_pct",
    "jobs_tourism_pct",
    "labour_force_pct",
    "starts_index",
    "debt_per_head",
    "income_low_pct",
    "gov_approval_low_pts",
    "gov_approval_middle_pts",
    "gov_approval_high_pts",
    "unemployment_pts",
    "wage_pct",
    "rent_pct",
    "housing_stock_pct",
    "population_pct",
    "jobs_pct",
    "gov_approval_pts",
    "worry_jobs_pts",
    "worry_housing_pts",
    "worry_taxes_pts",
    "worry_services_pts",
    "cash_per_head",
];

/// Too large for one town law at its defaults.
fn too_large(k: &str, v: f64) -> bool {
    match k {
        "unemployment_pts" => v.abs() > 3.0,
        "rent_pct" | "wage_pct" => v.abs() > 25.0,
        "population_pct" => v.abs() > 10.0,
        "gov_approval_pts" => v.abs() > 15.0,
        _ => false,
    }
}

fn summarise(cx: &Ctx, rows: &[Row], months: u32) -> Value {
    let mut table = Vec::new();
    let mut flags = Vec::new();
    let mut verdicts = BTreeMap::new();
    for tpl in templates(cx) {
        let Some(t) = cx.content.policies.get(tpl.as_str()) else {
            continue;
        };
        let rs: Vec<&Row> = rows.iter().filter(|r| r.template == tpl).collect();
        if rs.is_empty() {
            continue;
        }
        let col = |m: u32, k: &str| -> Vec<f64> {
            rs.iter()
                .filter_map(|r| r.at.get(&m).and_then(|x| x.get(k)).copied())
                .collect()
        };
        let mut years = serde_json::Map::new();
        for &m in SAMPLES.iter().filter(|m| **m <= months) {
            let mut o = serde_json::Map::new();
            for k in KEY {
                let v = col(m, k);
                o.insert(
                    k.into(),
                    json!({ "mean": r3(mean(&v)), "t": r1(t_stat(&v)) }),
                );
            }
            years.insert(format!("y{}", m / 12), Value::Object(o));
        }
        // Expectations at the last sample.
        let last = *SAMPLES.iter().rfind(|m| **m <= months).unwrap_or(&12);
        let mut checks = Vec::new();
        let mut worst = "works";
        for (k, sign, why) in expectations(t) {
            let v = col(last, &k);
            let (m, ts) = (mean(&v), t_stat(&v));
            let status = if v.is_empty() {
                "missing"
            } else if ts.abs() < 2.0 || m.abs() < 1e-3 {
                "no_effect"
            } else if m.signum() == sign {
                "ok"
            } else {
                "wrong_sign"
            };
            match status {
                "wrong_sign" => worst = "broken",
                "no_effect" | "missing" if worst != "broken" => worst = "weak",
                _ => {}
            }
            if status != "ok" {
                flags.push(json!({ "template": tpl, "metric": k, "status": status, "mean": r3(m), "t": r1(ts), "claim": why }));
            }
            checks.push(json!({ "metric": k, "expected": sign, "mean": r3(m), "t": r1(ts), "status": status, "claim": why }));
        }
        for k in KEY {
            let v = col(last, k);
            let big = v.iter().filter(|x| too_large(k, **x)).count();
            if big * 4 > v.len().max(1) {
                flags.push(json!({ "template": tpl, "metric": k, "status": "too_large", "share_of_seeds": r3(big as f64 / v.len().max(1) as f64), "median": r3(median(&v)) }));
                if worst == "works" {
                    worst = "weak";
                }
            }
        }
        let gov = col(last, "gov_approval_pts");
        let races: u32 = rs.iter().map(|r| r.races).sum();
        let flipped: u32 = rs.iter().map(|r| r.flipped).sum();
        let reaches_opinion = t_stat(&gov).abs() >= 2.0;
        if !reaches_opinion {
            flags.push(json!({ "template": tpl, "metric": "gov_approval_pts", "status": "never_reaches_opinion", "mean": r3(mean(&gov)), "t": r1(t_stat(&gov)) }));
        }
        if flipped == 0 {
            flags.push(json!({ "template": tpl, "metric": "home_races_flipped", "status": "never_reaches_votes", "races": races }));
        }
        let funded: Vec<f64> = rs.iter().map(|r| r.funded).collect();
        let progress: Vec<f64> = rs.iter().map(|r| r.progress).collect();
        let flood = {
            let v: Vec<&BTreeMap<String, f64>> =
                rs.iter().filter_map(|r| r.flood.as_ref()).collect();
            if v.is_empty() {
                Value::Null
            } else {
                let c = |k: &str| {
                    v.iter()
                        .filter_map(|m| m.get(k).copied())
                        .collect::<Vec<f64>>()
                };
                json!({
                    "n": v.len(),
                    "housing_stock_pct": { "mean": r3(mean(&c("housing_stock_pct"))), "t": r1(t_stat(&c("housing_stock_pct"))) },
                    "population_pct": { "mean": r3(mean(&c("population_pct"))), "t": r1(t_stat(&c("population_pct"))) },
                    "worry_recovery_pts": { "mean": r3(mean(&c("worry_recovery_pts"))), "t": r1(t_stat(&c("worry_recovery_pts"))) },
                    "gov_approval_pts": { "mean": r3(mean(&c("gov_approval_pts"))), "t": r1(t_stat(&c("gov_approval_pts"))) },
                })
            }
        };
        // By scenario: the template's first expected metric.
        let by_scenario: serde_json::Map<String, Value> = {
            let first = expectations(t)
                .first()
                .map(|x| x.0.clone())
                .unwrap_or("gov_approval_pts".into());
            let mut o = serde_json::Map::new();
            for s in cx.scenarios() {
                let name = short_scenario(&s);
                let v: Vec<f64> = rs
                    .iter()
                    .filter(|r| r.scenario == name)
                    .filter_map(|r| r.at.get(&last).and_then(|x| x.get(&first)).copied())
                    .collect();
                o.insert(
                    name,
                    json!({ "metric": first, "mean": r3(mean(&v)), "t": r1(t_stat(&v)) }),
                );
            }
            o
        };
        // By settlement class of the home town: the same metric.
        let by_class: serde_json::Map<String, Value> = {
            let first = expectations(t)
                .first()
                .map(|x| x.0.clone())
                .unwrap_or("gov_approval_pts".into());
            let mut classes: Vec<String> = rs.iter().map(|r| r.class.clone()).collect();
            classes.sort();
            classes.dedup();
            classes
                .into_iter()
                .map(|c| {
                    let v: Vec<f64> = rs
                        .iter()
                        .filter(|r| r.class == c)
                        .filter_map(|r| r.at.get(&last).and_then(|x| x.get(&first)).copied())
                        .collect();
                    (
                        c,
                        json!({ "n": v.len(), "mean": r3(mean(&v)), "t": r1(t_stat(&v)) }),
                    )
                })
                .collect()
        };
        verdicts.insert(tpl.clone(), worst);
        table.push(json!({
            "template": tpl,
            "n": rs.len(),
            "funded_share": { "mean": r3(mean(&funded)), "min": r3(quantile(&funded, 0.0)) },
            "progress": r3(mean(&progress)),
            "home_races": races,
            "races_flipped": flipped,
            "years": years,
            "checks": checks,
            "with_flood_at_30": flood,
            "by_scenario": by_scenario,
            "by_home_class": by_class,
            "verdict": worst,
        }));
    }
    json!({ "table": table, "flags": flags, "verdicts": verdicts })
}

pub fn run(cx: &Ctx) -> Result<Value, String> {
    let months = cx.years(5) * 12;
    let rows = rows(cx, months)?;
    let mut v = summarise(cx, &rows, months);
    v["params"] = json!({
        "seeds_per_scenario": cx.args.seeds,
        "scenarios": cx.scenarios().iter().map(short_scenario).collect::<Vec<_>>(),
        "months": months,
        "driver": "idle",
        "enacted_at_month": 1,
        "town": "the player's home town",
    });
    Ok(v)
}

/// The nightly gate's policy check: no template with a wrong-sign effect,
/// at a small sample (3 years, a random country only).
pub fn gate(cx: &Ctx, seeds: u64) -> Result<Value, String> {
    let sub = Ctx {
        content: cx.content.clone(),
        args: AuditArgs {
            seeds,
            years: 3,
            scenario: vec!["none".into()],
            ..cx.args.clone()
        },
    };
    let rows = rows(&sub, 36)?;
    let s = summarise(&sub, &rows, 36);
    let wrong: Vec<Value> = s["flags"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|f| f["status"] == "wrong_sign")
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    Ok(json!({ "pass": wrong.is_empty(), "wrong_sign": wrong, "verdicts": s["verdicts"] }))
}
