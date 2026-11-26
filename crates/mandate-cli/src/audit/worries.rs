//! Worries (DEC-106 item 1): does any worry dominate? Over small random
//! countries under the idle driver, each month every town's and every
//! neighbourhood's strongest felt worry (`concerns::ranked`, the reading
//! the player is told) is counted, with each worry's mean felt level; and
//! for the readers of the measured ranking (conditions of asks, arcs and
//! dilemmas), how often each worry is a town's strongest and among its
//! three strongest measured, and in how many worlds it was the home town's
//! strongest measured worry within the first 14 months (the zoning arc's
//! raise reads that). Each signal's spread across town-months is printed.
//!
//! `cargo run --release -p mandate-cli -- audit worries --seeds 16 --years 5`

use super::*;
use crate::lab::runner::{Driver, Game, Setup, par_map, threads};
use mandate_content::Indicator;
use mandate_content::condition::Worry;
use mandate_sim::concerns::{Scope, indicator, ranked, top};
use mandate_sim::scheduler::Reads;
use mandate_sim::world::{MunicipalityId, NeighborhoodId};

/// Per worry (registry order): months a town had it strongest, months a
/// neighbourhood did, and the sum of its felt town levels.
#[derive(Default, Clone)]
struct Count {
    town: Vec<u64>,
    hood: Vec<u64>,
    level: Vec<f64>,
    town_months: u64,
    hood_months: u64,
    /// Months a town had it strongest measured, and among its three
    /// strongest measured; and felt among the three strongest.
    measured: Vec<u64>,
    measured_top3: Vec<u64>,
    felt_top3: Vec<u64>,
    /// Worlds in which it was the home town's strongest measured worry in
    /// some month of the first 14.
    home_first_14: Vec<u64>,
    /// Every indicator's town values (`Indicator::ALL` order), for the
    /// spread of each signal.
    signals: Vec<Vec<f64>>,
}

fn world(cx: &Ctx, scenario: &Option<String>, seed: u64, months: u32) -> Result<Count, String> {
    let setup = Setup::small(seed, scenario.as_deref());
    let mut g = Game::new(&cx.content, &setup, Driver::Idle)?;
    let n = cx.content.concerns.len();
    let mut c = Count {
        town: vec![0; n],
        hood: vec![0; n],
        level: vec![0.0; n],
        signals: vec![Vec::new(); Indicator::ALL.len()],
        measured: vec![0; n],
        measured_top3: vec![0; n],
        felt_top3: vec![0; n],
        home_first_14: vec![0; n],
        ..Count::default()
    };
    let home = g.home();
    let mut month = 0;
    for _ in 0..months {
        month += 1;
        g.step_month();
        let st = g.e.state();
        let (pop, world) = (&st.population, &st.world);
        let reads = Reads::of_state(st, &cx.content);
        for m in 0..world.municipalities.len() {
            for (k, i) in Indicator::ALL.iter().enumerate() {
                c.signals[k].push(indicator(&reads, m, *i));
            }
        }
        for m in 0..world.municipalities.len() {
            let r = ranked(
                pop,
                &cx.content,
                world,
                Scope::town(MunicipalityId(m as u32)),
                Worry::Felt,
            );
            if let Some((i, _)) = top(&r, 1).next() {
                c.town[i] += 1;
            }
            for (i, _) in top(&r, 3) {
                c.felt_top3[i] += 1;
            }
            for (i, w) in &r {
                c.level[*i] += *w;
            }
            let rm = ranked(
                pop,
                &cx.content,
                world,
                Scope::town(MunicipalityId(m as u32)),
                Worry::Measured,
            );
            if let Some((i, _)) = top(&rm, 1).next() {
                c.measured[i] += 1;
                if m as u32 == home && month <= 14 {
                    c.home_first_14[i] = 1;
                }
            }
            for (i, _) in top(&rm, 3) {
                c.measured_top3[i] += 1;
            }
            c.town_months += 1;
        }
        for h in 0..world.neighborhoods.len() {
            let r = ranked(
                pop,
                &cx.content,
                world,
                Scope::Neighbourhood(NeighborhoodId(h as u32)),
                Worry::Felt,
            );
            if let Some((i, _)) = top(&r, 1).next() {
                c.hood[i] += 1;
            }
            c.hood_months += 1;
        }
    }
    Ok(c)
}

pub fn run(cx: &Ctx) -> Result<Value, String> {
    let years = cx.years(5);
    let months = years * 12;
    let scenarios = if cx.args.scenario.is_empty() {
        vec![None]
    } else {
        cx.scenarios()
    };
    let worlds: Vec<(Option<String>, u64)> = scenarios
        .into_iter()
        .flat_map(|s| (1..=cx.args.seeds).map(move |k| (s.clone(), k)))
        .collect();
    let runs = par_map(&worlds, threads(), |(s, seed)| world(cx, s, *seed, months));
    let n = cx.content.concerns.len();
    let mut all = Count {
        town: vec![0; n],
        hood: vec![0; n],
        level: vec![0.0; n],
        signals: vec![Vec::new(); Indicator::ALL.len()],
        measured: vec![0; n],
        measured_top3: vec![0; n],
        felt_top3: vec![0; n],
        home_first_14: vec![0; n],
        ..Count::default()
    };
    let nworlds = runs.len();
    for r in runs {
        let r = r?;
        for i in 0..n {
            all.town[i] += r.town[i];
            all.hood[i] += r.hood[i];
            all.level[i] += r.level[i];
            all.measured[i] += r.measured[i];
            all.measured_top3[i] += r.measured_top3[i];
            all.felt_top3[i] += r.felt_top3[i];
            all.home_first_14[i] += r.home_first_14[i];
        }
        for (a, b) in all.signals.iter_mut().zip(r.signals) {
            a.extend(b);
        }
        all.town_months += r.town_months;
        all.hood_months += r.hood_months;
    }
    let share = |k: u64, of: u64| (k as f64 / of.max(1) as f64 * 1000.0).round() / 10.0;
    let rows: Vec<Value> = cx
        .content
        .concerns
        .iter()
        .enumerate()
        .map(|(i, c)| {
            json!({
                "concern": c.id.to_string(),
                "strongest_town_pct": share(all.town[i], all.town_months),
                "strongest_hood_pct": share(all.hood[i], all.hood_months),
                "felt_top3_town_pct": share(all.felt_top3[i], all.town_months),
                "measured_strongest_town_pct": share(all.measured[i], all.town_months),
                "measured_top3_town_pct": share(all.measured_top3[i], all.town_months),
                "home_measured_strongest_first_14m_worlds": all.home_first_14[i],
                "mean_felt": (all.level[i] / all.town_months.max(1) as f64 * 1000.0).round() / 1000.0,
            })
        })
        .collect();
    let r4 = |x: f64| (x * 10_000.0).round() / 10_000.0;
    let signals: Vec<Value> = Indicator::ALL
        .iter()
        .zip(&all.signals)
        .map(|(i, v)| {
            json!({
                "indicator": format!("{i:?}"),
                "p10": r4(quantile(v, 0.1)),
                "p50": r4(quantile(v, 0.5)),
                "p90": r4(quantile(v, 0.9)),
                "p99": r4(quantile(v, 0.99)),
            })
        })
        .collect();
    let top_town = (0..n).map(|i| all.town[i]).max().unwrap_or(0);
    let top_hood = (0..n).map(|i| all.hood[i]).max().unwrap_or(0);
    Ok(json!({
        "seeds": cx.args.seeds,
        "years": years,
        "driver": "idle",
        "worlds": nworlds,
        "town_months": all.town_months,
        "hood_months": all.hood_months,
        "worries": rows,
        "signals": signals,
        "largest_town_share_pct": share(top_town, all.town_months),
        "largest_hood_share_pct": share(top_hood, all.hood_months),
        "never_strongest": cx.content.concerns.iter().enumerate()
            .filter(|(i, _)| all.hood[*i] == 0)
            .map(|(_, c)| c.id.to_string())
            .collect::<Vec<_>>(),
    }))
}
