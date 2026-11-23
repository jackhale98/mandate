//! Topic 5, player leverage: how much one seat-holder changes, with the bot
//! strategies as drivers against the idle player on the same seed (the
//! worlds are identical until the first action, so the difference is the
//! player's doing): their own elections, their ward's worries and
//! conditions, and the town's indicators.

use super::*;
use crate::lab::runner::{Driver, Game, Setup, par_map, threads};

pub const DRIVERS: [&str; 10] = [
    "idle",
    "canvass",
    "media",
    "asks_only",
    "mixed",
    "legislator",
    "explorer",
    "explorer_legislator",
    "explorer_campaigner",
    "corrupt",
];

#[derive(Clone, Debug, Default)]
struct Out {
    ran: u32,
    won: u32,
    first_won: Option<bool>,
    laws: u32,
    ward: std::collections::BTreeMap<String, f64>,
    town: std::collections::BTreeMap<String, f64>,
}

fn one(cx: &Ctx, seed: u64, driver: &str, months: u32) -> Result<Out, String> {
    let mut g = Game::new(
        &cx.content,
        &Setup::small(seed, None),
        Driver::parse(driver),
    )?;
    let home = g.home();
    g.detail = vec![home];
    let mut s = Vec::new();
    g.run_to(months, &[months], &mut s);
    let st = g.e.state();
    let me = st.player.as_ref().map(|p| p.politician);
    let ward: Vec<u32> = st
        .player
        .as_ref()
        .map(|p| {
            mandate_sim::policy::ward_hoods(&st.world, p.ward)
                .into_iter()
                .map(|h| h as u32)
                .collect()
        })
        .unwrap_or_default();
    let mine: Vec<_> = g.tally.elections.iter().filter(|e| e.player_ran).collect();
    let laws = st
        .policy
        .instances
        .iter()
        .filter(|p| p.sponsor.is_some() && p.sponsor == me && p.enacted_on.is_some())
        .count() as u32;
    let t = &s[0].towns[home as usize];
    let hs: Vec<_> = t.hoods.iter().filter(|h| ward.contains(&h.id)).collect();
    let wavg = |f: &dyn Fn(&crate::lab::metrics::Hood) -> f64| {
        let n: f64 = hs.iter().map(|h| h.population as f64).sum::<f64>().max(1.0);
        hs.iter().map(|h| f(h) * h.population as f64).sum::<f64>() / n
    };
    let mut w = std::collections::BTreeMap::new();
    w.insert(
        "player_approval_pts".into(),
        wavg(&|h| h.player_approval) * 100.0,
    );
    w.insert(
        "player_awareness_pts".into(),
        wavg(&|h| h.player_awareness) * 100.0,
    );
    w.insert("gov_approval_pts".into(), wavg(&|h| h.gov_approval) * 100.0);
    w.insert(
        "population".into(),
        wavg(&|h| h.population as f64) * hs.len() as f64,
    );
    for k in t.concerns.keys() {
        w.insert(
            format!("worry_{k}_pts"),
            wavg(&|h| h.concerns.get(k).copied().unwrap_or(0.0)) * 100.0,
        );
    }
    let mut town = std::collections::BTreeMap::new();
    town.insert("unemployment_pts".into(), t.unemployment * 100.0);
    town.insert("rent".into(), t.rent);
    town.insert("gov_approval_pts".into(), t.gov_approval * 100.0);
    town.insert("population".into(), t.population as f64);
    town.insert("housing_stock".into(), t.housing_stock);
    town.insert(
        "cash_per_head".into(),
        t.cash / (t.population as f64).max(1.0),
    );
    Ok(Out {
        ran: mine.len() as u32,
        won: mine.iter().filter(|e| e.player_won).count() as u32,
        first_won: mine.first().map(|e| e.player_won),
        laws,
        ward: w,
        town,
    })
}

pub fn run(cx: &Ctx) -> Result<Value, String> {
    let months = cx.years(6) * 12;
    let seeds: Vec<u64> = (1..=cx.args.seeds).collect();
    let drivers: Vec<&str> = DRIVERS
        .iter()
        .copied()
        .filter(|d| *d == "idle" || cx.args.only.is_empty() || cx.args.only.iter().any(|o| o == d))
        .collect();
    let jobs: Vec<(u64, &str)> = seeds
        .iter()
        .flat_map(|s| drivers.iter().map(move |d| (*s, *d)))
        .collect();
    let res = par_map(&jobs, threads(), |(s, d)| one(cx, *s, d, months));
    let mut by: std::collections::BTreeMap<(u64, &str), Out> = Default::default();
    for ((s, d), r) in jobs.iter().zip(res) {
        by.insert((*s, *d), r?);
    }
    let mut rows = Vec::new();
    for d in &drivers {
        let mut ward_d: std::collections::BTreeMap<String, Vec<f64>> = Default::default();
        let mut town_d: std::collections::BTreeMap<String, Vec<f64>> = Default::default();
        let (mut ran, mut won, mut first, mut firsts, mut laws) = (0, 0, 0, 0, 0);
        for s in &seeds {
            let (Some(x), Some(i)) = (by.get(&(*s, *d)), by.get(&(*s, "idle"))) else {
                continue;
            };
            ran += x.ran;
            won += x.won;
            laws += x.laws;
            if let Some(f) = x.first_won {
                firsts += 1;
                if f {
                    first += 1;
                }
            }
            for (k, v) in &x.ward {
                ward_d
                    .entry(k.clone())
                    .or_default()
                    .push(v - i.ward.get(k).copied().unwrap_or(0.0));
            }
            for (k, v) in &x.town {
                let b = i.town.get(k).copied().unwrap_or(0.0);
                let dv = if k == "rent" || k == "population" || k == "housing_stock" {
                    pct(b, *v)
                } else {
                    v - b
                };
                town_d.entry(k.clone()).or_default().push(dv);
            }
        }
        let summ =
            |m: &std::collections::BTreeMap<String, Vec<f64>>| -> serde_json::Map<String, Value> {
                m.iter()
                .map(|(k, v)| (k.clone(), json!({ "mean": r3(mean(v)), "max_abs": r3(v.iter().fold(0.0f64, |a, x| a.max(x.abs()))) })))
                .collect()
            };
        rows.push(json!({
            "driver": d,
            "class": crate::bots::class(d).key(),
            "first_election_win_rate": r3(first as f64 / firsts.max(1) as f64),
            "races_run": ran,
            "races_won_share": r3(won as f64 / ran.max(1) as f64),
            "own_laws_enacted": laws,
            "ward_vs_idle": summ(&ward_d),
            "town_vs_idle": summ(&town_d),
        }));
    }
    // Verdict: the best informed style should beat idle clearly on its own
    // elections, and move its ward's worries measurably but not wildly.
    let win = |d: &str| {
        rows.iter()
            .find(|r| r["driver"] == d)
            .and_then(|r| r["first_election_win_rate"].as_f64())
            .unwrap_or(0.0)
    };
    let best_informed = ["explorer", "explorer_legislator", "explorer_campaigner"]
        .iter()
        .map(|d| win(d))
        .fold(0.0, f64::max);
    let worry_move = rows
        .iter()
        .filter(|r| r["driver"] != "idle")
        .flat_map(|r| {
            r["ward_vs_idle"]
                .as_object()
                .map(|o| {
                    o.iter()
                        .filter(|(k, _)| k.starts_with("worry_"))
                        .map(|(_, v)| v["mean"].as_f64().unwrap_or(0.0).abs())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .fold(0.0, f64::max);
    let town_move = rows
        .iter()
        .filter(|r| r["driver"] != "idle")
        .map(|r| {
            r["town_vs_idle"]["unemployment_pts"]["mean"]
                .as_f64()
                .unwrap_or(0.0)
                .abs()
        })
        .fold(0.0, f64::max);
    Ok(json!({
        "params": { "seeds": cx.args.seeds, "months": months, "world": "random country, small", "baseline": "idle on the same seed" },
        "table": rows,
        "summary": {
            "idle_first_win": r3(win("idle")),
            "best_informed_first_win": r3(best_informed),
            "largest_mean_ward_worry_move_pts": r3(worry_move),
            "largest_mean_town_unemployment_move_pts": r3(town_move),
        },
        "verdict_elections": if best_informed - win("idle") >= 0.2 { "works" } else { "weak" },
        "verdict_conditions": if worry_move >= 3.0 && town_move < 3.0 { "works" } else if worry_move >= 1.0 { "weak" } else { "broken" },
    }))
}
