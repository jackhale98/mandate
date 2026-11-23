//! Topic 3, sensitivity: one-at-a-time changes (±20%) to the main content
//! parameters, and which of them move the outcomes that matter
//! (unemployment, incumbents' re-election, the idle player's win rate).

use super::*;
use crate::lab::runner::{Driver, Game, Setup, par_map, threads};
use mandate_content::ContentBundle;
use mandate_content::overrides::Override;
use std::sync::Arc;

/// The parameters varied (path as `--set` takes it).
pub const PARAMS: [&str; 36] = [
    "economy.natural_unemployment",
    "economy.migration_rate",
    "economy.target_vacancy",
    "economy.recession_monthly_chance",
    "economy.unemployment_replacement",
    "economy.tech_growth",
    "economy.manufacturing_trend",
    "economy.birth_rate",
    "economy.construction_lag_months",
    "politics.vote.issue_weight",
    "politics.vote.party_weight",
    "politics.vote.valence_weight",
    "politics.vote.retrospective_weight",
    "politics.vote.executive_multiplier",
    "politics.vote.incumbent_awareness",
    "politics.vote.challenger_awareness",
    "politics.vote.race_shock_sd",
    "politics.vote.national_swing_sd",
    "politics.vote.strategic_weight",
    "politics.vote.record_penalty",
    "politics.vote.record_bonus",
    "politics.vote.partisan_discount",
    "politics.vote.retro_economy_weight",
    "politics.vote.player_approval_weight",
    "politics.vote.media_base",
    "politics.vote.home_ward_bonus",
    "politics.turnout.standalone",
    "politics.turnout.habit",
    "politics.campaign.awareness_per_canvass",
    "politics.campaign.persuasion_per_canvass",
    "opinion.experience.unemployment",
    "opinion.experience.rent_growth",
    "opinion.experience.services",
    "opinion.policy_reaction",
    "opinion.law_reaction_weight",
    "opinion.gov_approval_rate",
];

/// The current value of a parameter path in loaded content.
pub fn current(content: &ContentBundle, path: &str) -> Option<serde_json::Value> {
    let (root, rest) = path.split_once('.')?;
    let v = match root {
        "economy" => serde_json::to_value(&content.economy).ok()?,
        "politics" => serde_json::to_value(&content.politics).ok()?,
        "opinion" => serde_json::to_value(&content.opinion).ok()?,
        "council" => serde_json::to_value(&content.council).ok()?,
        "career" => serde_json::to_value(&content.career).ok()?,
        _ => return None,
    };
    rest.split('.').try_fold(v, |v, k| v.get(k).cloned())
}

/// The value at `factor` × the current one (rounded for an integer).
pub fn scaled(content: &ContentBundle, path: &str, factor: f64) -> Option<serde_json::Value> {
    let v = current(content, path)?;
    if let Some(i) = v.as_i64().filter(|_| v.is_i64() || v.is_u64()) {
        return Some(json!(((i as f64) * factor).round() as i64));
    }
    v.as_f64().map(|x| json!(x * factor))
}

/// What a content variant does to the world.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    pub u_median: f64,
    pub reelection: f64,
    pub gov_mean: f64,
    pub pop_growth_pct: f64,
    pub debt_change: f64,
    pub idle_win: f64,
    pub mixed_win: f64,
    /// Per seed: every local race's winner, in order.
    pub winners: Vec<Vec<(String, u32)>>,
    /// Per seed: the final state hash.
    pub hashes: Vec<String>,
    /// Per-seed values, for noise estimates.
    pub per_seed: Vec<[f64; 5]>,
}

/// Outcomes of `seeds` worlds generated with the base content; with `sets`,
/// the rules change at month 0 (the same worlds, a balance patch on day
/// one), so race winners compare one to one. The bots play whole games
/// under the variant content.
pub fn outcomes(
    base: &Arc<ContentBundle>,
    sets: &[(String, serde_json::Value)],
    seeds: u64,
    years: u32,
    bot_seeds: u64,
) -> Result<Outcome, String> {
    let content = &if sets.is_empty() {
        base.clone()
    } else {
        variant(base, sets)?
    };
    let list: Vec<u64> = (1..=seeds).collect();
    let runs = par_map(&list, threads(), |seed| -> Result<_, String> {
        let mut g = Game::new(base, &Setup::small(*seed, None), Driver::Idle)?;
        if !sets.is_empty() {
            g.intervene(&crate::lab::runner::Intervention::Set {
                sets: sets.to_vec(),
            })?;
        }
        let mut s = Vec::new();
        g.run_to(years * 12, &[0, years * 12], &mut s);
        let us: Vec<f64> = {
            let h = &g.e.state().economy.national.history;
            let w = (years as usize * 12).min(h.len());
            h[h.len() - w..]
                .iter()
                .map(|x| x.unemployment as f64)
                .collect()
        };
        let es: Vec<_> = g
            .tally
            .elections
            .iter()
            .filter(|e| e.town.is_some())
            .collect();
        let ran = es.iter().filter(|e| e.incumbent_ran).count();
        let won = es
            .iter()
            .filter(|e| e.incumbent_ran && e.incumbent_won)
            .count();
        let gov = mean(
            &s[1]
                .towns
                .iter()
                .map(|t| t.gov_approval)
                .collect::<Vec<_>>(),
        );
        let pop = pct(s[0].nation.population as f64, s[1].nation.population as f64);
        let debt = s[1].nation.debt_pct_gdp - s[0].nation.debt_pct_gdp;
        let winners = es
            .iter()
            .map(|e| {
                (
                    format!("{}{}{:?}{:?}", e.date, e.post, e.town, e.constituency),
                    e.winner,
                )
            })
            .collect();
        Ok((
            [median(&us), won as f64 / ran.max(1) as f64, gov, pop, debt],
            winners,
            g.e.state_hash(),
        ))
    });
    let mut o = Outcome::default();
    for r in runs {
        let (v, w, h) = r?;
        o.per_seed.push(v);
        o.winners.push(w);
        o.hashes.push(h);
    }
    let col = |i: usize| o.per_seed.iter().map(|v| v[i]).collect::<Vec<f64>>();
    o.u_median = mean(&col(0));
    o.reelection = mean(&col(1));
    o.gov_mean = mean(&col(2));
    o.pop_growth_pct = mean(&col(3));
    o.debt_change = mean(&col(4));
    let bl: Vec<u64> = (1..=bot_seeds).collect();
    let win = |s: &str| {
        let w = par_map(&bl, threads(), |seed| {
            crate::bots::play(content, *seed, s)
                .map(|x| x.0)
                .unwrap_or(false)
        });
        w.iter().filter(|x| **x).count() as f64 / w.len().max(1) as f64
    };
    if bot_seeds > 0 {
        o.idle_win = win("idle");
        o.mixed_win = win("mixed");
    }
    Ok(o)
}

/// Share of races whose winner differs from the base's (matched by date and seat).
pub fn flipped(base: &Outcome, v: &Outcome) -> (usize, usize) {
    let mut n = 0;
    let mut f = 0;
    for (a, b) in base.winners.iter().zip(&v.winners) {
        for (k, w) in a {
            if let Some((_, w2)) = b.iter().find(|(k2, _)| k2 == k) {
                n += 1;
                if w2 != w {
                    f += 1;
                }
            }
        }
    }
    (f, n)
}

pub fn variant(
    root_content: &Arc<ContentBundle>,
    sets: &[(String, serde_json::Value)],
) -> Result<Arc<ContentBundle>, String> {
    let _ = root_content;
    let ovs: Vec<Override> = sets
        .iter()
        .map(|(p, v)| Override {
            path: p.clone(),
            value: v.clone(),
        })
        .collect();
    crate::lab::load(&crate::lab::runner::content_root(), &ovs)
}

pub fn outcome_json(o: &Outcome) -> Value {
    json!({
        "unemployment_median": r3(o.u_median),
        "reelection": r3(o.reelection),
        "gov_approval": r3(o.gov_mean),
        "pop_growth_pct": r3(o.pop_growth_pct),
        "debt_change": r3(o.debt_change),
        "idle_win": r3(o.idle_win),
        "mixed_win": r3(o.mixed_win),
    })
}

pub fn run(cx: &Ctx) -> Result<Value, String> {
    let years = cx.years(10);
    let seeds = cx.args.seeds;
    let bot_seeds = seeds * 2;
    let base = outcomes(&cx.content, &[], seeds, years, bot_seeds)?;
    // Noise: the standard error of each outcome's mean across seeds.
    let se = |i: usize| {
        let v: Vec<f64> = base.per_seed.iter().map(|x| x[i]).collect();
        sd(&v) / (v.len() as f64).sqrt()
    };
    let noise = json!({
        "unemployment_median": r3(se(0)),
        "reelection": r3(se(1)),
        "gov_approval": r3(se(2)),
        "win_rate": r3((0.25f64 / bot_seeds.max(1) as f64).sqrt()),
    });
    let params: Vec<&str> = PARAMS
        .iter()
        .copied()
        .filter(|p| cx.args.only.is_empty() || cx.args.only.iter().any(|o| p.contains(o.as_str())))
        .collect();
    let mut rows = Vec::new();
    for p in params {
        let mut row = serde_json::Map::new();
        row.insert("param".into(), json!(p));
        row.insert(
            "base".into(),
            current(&cx.content, p).unwrap_or(Value::Null),
        );
        let mut effect = 0.0f64;
        let mut flips_total = 0;
        let mut races_total = 0;
        let mut identical = true;
        for (name, f) in [("minus20", 0.8), ("plus20", 1.2)] {
            let Some(v) = scaled(&cx.content, p, f) else {
                row.insert(name.into(), json!("not numeric"));
                continue;
            };

            let o = outcomes(
                &cx.content,
                &[(p.to_string(), v.clone())],
                seeds,
                years,
                bot_seeds,
            )?;
            let (fl, n) = flipped(&base, &o);
            flips_total += fl;
            races_total += n;
            identical &= o.per_seed == base.per_seed && fl == 0;
            let d = [
                (o.u_median - base.u_median) * 100.0,
                (o.reelection - base.reelection) * 100.0,
                (o.gov_mean - base.gov_mean) * 100.0,
                (o.idle_win - base.idle_win) * 100.0,
                (o.mixed_win - base.mixed_win) * 100.0,
            ];
            // Size: the largest change in points among the outcomes.
            effect = effect.max(d.iter().fold(0.0, |a: f64, x| a.max(x.abs())));
            row.insert(
                name.into(),
                json!({
                    "value": v,
                    "d_unemployment_pts": r3(d[0]),
                    "d_reelection_pts": r1(d[1]),
                    "d_gov_approval_pts": r1(d[2]),
                    "d_idle_win_pts": r1(d[3]),
                    "d_mixed_win_pts": r1(d[4]),
                    "races_flipped_share": r3(fl as f64 / n.max(1) as f64),
                }),
            );
        }
        row.insert("max_effect_pts".into(), json!(r1(effect)));
        row.insert(
            "races_flipped_share".into(),
            json!(r3(flips_total as f64 / races_total.max(1) as f64)),
        );
        row.insert("no_effect_at_all".into(), json!(identical));
        eprintln!("{}", serde_json::to_string(&row).unwrap_or_default());
        rows.push(Value::Object(row));
    }
    rows.sort_by(|a, b| {
        b["max_effect_pts"]
            .as_f64()
            .unwrap_or(0.0)
            .total_cmp(&a["max_effect_pts"].as_f64().unwrap_or(0.0))
    });
    let top = rows
        .first()
        .and_then(|r| r["max_effect_pts"].as_f64())
        .unwrap_or(0.0);
    let second = rows
        .get(1)
        .and_then(|r| r["max_effect_pts"].as_f64())
        .unwrap_or(0.0);
    let dead: Vec<Value> = rows
        .iter()
        .filter(|r| r["no_effect_at_all"].as_bool().unwrap_or(false))
        .map(|r| r["param"].clone())
        .collect();
    Ok(json!({
        "params": { "seeds": seeds, "years": years, "bot_seeds": bot_seeds, "driver": "idle", "world": "random country, small" },
        "base": outcome_json(&base),
        "noise_se": noise,
        "ranked": rows,
        "dominant": if top > 3.0 * second.max(0.1) { rows.first().map(|r| r["param"].clone()) } else { None },
        "no_effect_at_all": dead,
    }))
}
