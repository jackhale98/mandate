//! Monte Carlo health sweep (TEST_STRATEGY §4, DEC-059 rev. 2): stability
//! bands on medians across seeds, plus drama floors. Runs are independent, so
//! they execute in parallel threads (allowed between runs, DEC-041 rule 1).

use mandate_content::ContentBundle;
use mandate_sim::Command;
use mandate_sim::events::EventKind;
use mandate_worldgen::{Preset, WorldConfig, WorldSize, generate};
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize)]
pub struct RunMetrics {
    pub seed: u64,
    pub population_change_per_decade: f64,
    pub unemployment_median: f64,
    pub unemployment_max: f64,
    pub inflation_median: f64,
    pub inflation_max: f64,
    pub debt_pct_gdp_start: f64,
    pub debt_pct_gdp_end: f64,
    pub recessions: u32,
    pub disasters: u32,
    pub major_disasters: u32,
    pub plant_closures: u32,
    pub state_hash: String,
}

fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

fn debt_ratio(e: &mandate_sim::Engine) -> f64 {
    let s = e.state();
    let gdp = s.economy.national.gdp.max(1.0);
    s.finance
        .national
        .as_ref()
        .map(|t| t.debt.units() as f64 / gdp)
        .unwrap_or(0.0)
}

pub fn run_one(
    content: &Arc<ContentBundle>,
    seed: u64,
    years: u32,
    size: WorldSize,
    preset: Option<Preset>,
) -> RunMetrics {
    let mut e = match generate(&WorldConfig { seed, size, preset }, content.clone()) {
        Ok(e) => e,
        Err(err) => panic!("generation failed for seed {seed}: {err}"),
    };
    let start_events = e
        .state()
        .events
        .iter()
        .last()
        .map(|x| x.id)
        .unwrap_or(mandate_sim::events::EventId(0));
    let pop0 = e.state().population.total() as f64;
    let debt0 = debt_ratio(&e);
    let _ = e.apply(Command::AdvanceDays { days: years * 365 });
    let s = e.state();
    let pop1 = s.population.total() as f64;
    let hist = &s.economy.national.history;
    let window = (years as usize * 12).min(hist.len());
    let recent = &hist[hist.len() - window..];
    let mut recessions = 0;
    let mut disasters = 0;
    let mut major = 0;
    let mut closures = 0;
    for ev in s.events.iter().filter(|x| x.id > start_events) {
        match &ev.kind {
            EventKind::GlobalRecessionBegan { .. } => recessions += 1,
            EventKind::Disaster { severity_pct, .. } => {
                disasters += 1;
                if *severity_pct >= 70 {
                    major += 1;
                }
            }
            EventKind::PlantClosure { .. } => closures += 1,
            _ => {}
        }
    }
    RunMetrics {
        seed,
        population_change_per_decade: mandate_core::math::pow(
            (pop1 / pop0.max(1.0)).max(1e-6),
            10.0 / years.max(1) as f64,
        ) - 1.0,
        unemployment_median: median(recent.iter().map(|h| h.unemployment as f64).collect()),
        unemployment_max: recent
            .iter()
            .map(|h| h.unemployment as f64)
            .fold(0.0, f64::max),
        inflation_median: median(recent.iter().map(|h| h.inflation as f64).collect()),
        inflation_max: recent
            .iter()
            .map(|h| h.inflation as f64)
            .fold(f64::MIN, f64::max),
        debt_pct_gdp_start: debt0,
        debt_pct_gdp_end: debt_ratio(&e),
        recessions,
        disasters,
        major_disasters: major,
        plant_closures: closures,
        state_hash: e.state_hash(),
    }
}

pub fn run(
    content: &Arc<ContentBundle>,
    seeds: u64,
    years: u32,
    size: WorldSize,
    preset: Option<Preset>,
) -> Vec<RunMetrics> {
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .min(seeds as usize)
        .max(1);
    let seed_list: Vec<u64> = (1..=seeds).collect();
    let mut rows: Vec<RunMetrics> = std::thread::scope(|sc| {
        let handles: Vec<_> = seed_list
            .chunks(seed_list.len().div_ceil(threads))
            .map(|chunk| {
                let chunk = chunk.to_vec();
                let c = content.clone();
                sc.spawn(move || {
                    chunk
                        .into_iter()
                        .map(|s| run_one(&c, s, years, size, preset))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });
    rows.sort_by_key(|r| r.seed);
    rows
}

pub fn report(rows: &[RunMetrics], years: u32) -> serde_json::Value {
    let med = |f: &dyn Fn(&RunMetrics) -> f64| median(rows.iter().map(f).collect());
    let scale30 = 30.0 / years.max(1) as f64;
    let pop = med(&|r| r.population_change_per_decade);
    let unemp = med(&|r| r.unemployment_median);
    let infl = med(&|r| r.inflation_median);
    let debt_growth = med(&|r| r.debt_pct_gdp_end - r.debt_pct_gdp_start);
    let recessions = med(&|r| r.recessions as f64) * scale30;
    let disasters = med(&|r| r.disasters as f64) * scale30;
    let bands = serde_json::json!({
        "population_change_per_decade": { "median": pop, "band": [-0.10, 0.20], "pass": (-0.10..=0.20).contains(&pop) },
        "unemployment": { "median": unemp, "band": [0.02, 0.20], "pass": (0.02..=0.20).contains(&unemp) },
        "inflation": { "median": infl, "band": [-0.02, 0.12], "pass": (-0.02..=0.12).contains(&infl) },
        "debt_pct_gdp_change": { "median": debt_growth, "band": [-0.5, 0.6], "pass": (-0.5..=0.6).contains(&debt_growth) },
    });
    let floors = serde_json::json!({
        "recessions_per_30y": { "median": recessions, "floor": 2, "pass": recessions >= 2.0 },
        "disasters_per_30y": { "median": disasters, "floor": 1, "pass": disasters >= 1.0 },
    });
    let all = |v: &serde_json::Value| {
        v.as_object()
            .map(|o| o.values().all(|x| x["pass"].as_bool().unwrap_or(false)))
            .unwrap_or(false)
    };
    let pass = all(&bands) && all(&floors);
    serde_json::json!({ "pass": pass, "seeds": rows.len(), "years": years, "bands": bands, "drama_floors": floors, "runs": rows })
}
