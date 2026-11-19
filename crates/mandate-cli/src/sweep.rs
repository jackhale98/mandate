//! Monte Carlo health sweep (TEST_STRATEGY §4, DEC-059 rev. 2): stability
//! bands on medians across seeds, plus drama floors. Runs are independent, so
//! they execute in parallel threads (allowed between runs, DEC-041 rule 1).

use mandate_content::ContentBundle;
use mandate_sim::Command;
use mandate_sim::events::EventKind;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
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
    /// Town character (DEC-077 §1) at the end of the run: shares of towns
    /// with a defining industry, a power broker, and a rival.
    pub towns_with_industry: f64,
    pub towns_with_broker: f64,
    pub towns_with_rival: f64,
    /// Crises seen beginning and ending (sampled yearly) across all towns.
    pub crises_begun: u32,
    pub crises_ended: u32,
    /// Story density (DEC-033): closed chapters, and per chapter the
    /// decisions (climax plus significant) and minor developments the
    /// director spotlighted, and the share of chapters with a climax. The
    /// player is idle in a sweep, so every decision takes its default.
    pub chapters: u32,
    pub decisions_per_chapter: f64,
    pub minor_per_chapter: f64,
    pub climax_share: f64,
    /// Chapters whose spotlight exceeded a budget (must be 0).
    pub over_budget: u32,
    pub storylines: u32,
    /// Scandals exposed and cases opened (DEC-034, DEC-006): the press and
    /// the courts at work on NPCs (the player is idle and clean in a sweep).
    pub scandals: u32,
    pub major_scandals: u32,
    pub cases: u32,
    pub state_hash: String,
}

/// (town, crisis kind) pairs in progress.
fn crises_now(e: &mandate_sim::Engine) -> Vec<(usize, String)> {
    e.state()
        .politics
        .characters
        .towns
        .iter()
        .enumerate()
        .flat_map(|(m, t)| t.crises.iter().map(move |c| (m, c.kind.clone())))
        .collect()
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
    scenario: Option<&str>,
) -> RunMetrics {
    let mut e = match generate(
        &WorldConfig {
            seed,
            size,
            scenario: scenario.map(str::to_string),
        },
        content.clone(),
    ) {
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
    // A year at a time (the same result as one long advance) to watch
    // town crises begin and end.
    let (mut begun, mut ended) = (0u32, 0u32);
    let mut before = crises_now(&e);
    for _ in 0..years {
        let _ = e.apply(Command::AdvanceDays { days: 365 });
        let now = crises_now(&e);
        begun += now.iter().filter(|c| !before.contains(c)).count() as u32;
        ended += before.iter().filter(|c| !now.contains(c)).count() as u32;
        before = now;
    }
    let s = e.state();
    let towns = &s.politics.characters.towns;
    let share = |f: &dyn Fn(&mandate_sim::characters::TownCharacter) -> bool| {
        towns.iter().filter(|t| f(t)).count() as f64 / towns.len().max(1) as f64
    };
    let pop1 = s.population.total() as f64;
    let hist = &s.economy.national.history;
    let window = (years as usize * 12).min(hist.len());
    let recent = &hist[hist.len() - window..];
    let mut recessions = 0;
    let mut disasters = 0;
    let mut major = 0;
    let mut closures = 0;
    let (mut scandals, mut major_scandals) = (0, 0);
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
            EventKind::ScandalBroke { severity, .. } => {
                scandals += 1;
                if severity != "ethics" {
                    major_scandals += 1;
                }
            }
            _ => {}
        }
    }
    let ch = &s.narrative.chapters;
    let nch = ch.len().max(1) as f64;
    let d = &content.director;
    let b = mandate_sim::narrative::director::budget(d, s.narrative.profile(content));
    let over = ch
        .iter()
        .filter(|c| c.climaxes > b.climax || c.significant > b.sig_max || c.minor > b.minor_max)
        .count() as u32;
    RunMetrics {
        chapters: ch.len() as u32,
        decisions_per_chapter: ch
            .iter()
            .map(|c| (c.climaxes + c.significant) as f64)
            .sum::<f64>()
            / nch,
        minor_per_chapter: ch.iter().map(|c| c.minor as f64).sum::<f64>() / nch,
        climax_share: ch.iter().filter(|c| c.climaxes > 0).count() as f64 / nch,
        over_budget: over,
        storylines: s.narrative.storylines.len() as u32,
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
        towns_with_industry: share(&|t| t.industry.is_some()),
        towns_with_broker: share(&|t| t.broker.is_some()),
        towns_with_rival: share(&|t| t.rival.is_some()),
        crises_begun: begun,
        crises_ended: ended,
        scandals,
        major_scandals,
        cases: s.justice.next_case,
        state_hash: e.state_hash(),
    }
}

pub fn run(
    content: &Arc<ContentBundle>,
    seeds: u64,
    years: u32,
    size: WorldSize,
    scenario: Option<&str>,
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
                        .map(|s| run_one(&c, s, years, size, scenario))
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
    let major = med(&|r| r.major_scandals as f64) * scale30;
    let floors = serde_json::json!({
        "recessions_per_30y": { "median": recessions, "floor": 2, "pass": recessions >= 2.0 },
        "disasters_per_30y": { "median": disasters, "floor": 1, "pass": disasters >= 1.0 },
        // TEST_STRATEGY §4: major scandals exposed (misconduct or crime).
        "major_scandals_per_30y": { "median": major, "floor": 3, "pass": major >= 3.0,
            "all_scandals": med(&|r| r.scandals as f64) * scale30,
            "cases": med(&|r| r.cases as f64) * scale30 },
    });
    let all = |v: &serde_json::Value| {
        v.as_object()
            .map(|o| o.values().all(|x| x["pass"].as_bool().unwrap_or(false)))
            .unwrap_or(false)
    };
    // Town character (DEC-077 §1): every town has a defining industry; most
    // have a power broker and a rival; crises come and go.
    let industry_min = rows
        .iter()
        .map(|r| r.towns_with_industry)
        .fold(1.0, f64::min);
    let town = serde_json::json!({
        "industry_min": industry_min,
        "broker_median": med(&|r| r.towns_with_broker),
        "rival_median": med(&|r| r.towns_with_rival),
        "crises_begun_median": med(&|r| r.crises_begun as f64) * scale30,
        "crises_ended_median": med(&|r| r.crises_ended as f64) * scale30,
        "pass": industry_min >= 1.0,
    });
    // Story density (DEC-033): decisions and developments per chapter; no
    // chapter may exceed the director's budgets.
    let over = rows.iter().map(|r| r.over_budget).sum::<u32>();
    let story = serde_json::json!({
        "chapters_median": med(&|r| r.chapters as f64),
        "decisions_per_chapter_median": med(&|r| r.decisions_per_chapter),
        "minor_per_chapter_median": med(&|r| r.minor_per_chapter),
        "climax_share_median": med(&|r| r.climax_share),
        "storylines_median": med(&|r| r.storylines as f64),
        "chapters_over_budget": over,
        "pass": over == 0,
    });
    let pass = all(&bands)
        && all(&floors)
        && town["pass"].as_bool().unwrap_or(false)
        && story["pass"].as_bool().unwrap_or(false);
    serde_json::json!({ "pass": pass, "seeds": rows.len(), "years": years, "bands": bands, "drama_floors": floors, "town_character": town, "story_density": story, "runs": rows })
}
