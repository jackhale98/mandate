//! Topic 2, believable behaviour: the textbook relationships, measured on
//! baseline histories (idle player, NPC turns) and on twins for the shocks.

use super::*;
use crate::lab::runner::{Driver, Game, Intervention, Setup, par_map, threads};

/// Tercile edges of a sample.
fn terciles(v: &[f64]) -> (f64, f64) {
    (quantile(v, 1.0 / 3.0), quantile(v, 2.0 / 3.0))
}

/// Unemployment against incumbents: each local race an incumbent ran in,
/// by the town's 12-month unemployment change relative to the nation's.
fn retrospective(hs: &[History]) -> Value {
    let mut xs = Vec::new();
    let mut won = Vec::new();
    let mut office = Vec::new();
    let mut gov = Vec::new();
    let mut gov_du = (Vec::new(), Vec::new());
    for h in hs {
        for e in &h.tally.elections {
            let (Some(t), true) = (e.town, e.incumbent_ran) else {
                continue;
            };
            let m = e.month as usize;
            if m < 12 || m >= h.months.len() {
                continue;
            }
            let du = h.months[m][t as usize].u - h.months[m - 12][t as usize].u;
            let dn = h.nat_u[m] - h.nat_u[m - 12];
            xs.push((du - dn) * 100.0);
            won.push(if e.incumbent_won { 1.0 } else { 0.0 });
            office.push(e.office);
            gov.push(h.months[m][t as usize].gov);
        }
        // Town-years: approval of the town government against relative
        // unemployment change.
        for m in (12..h.months.len()).step_by(12) {
            for t in 0..h.months[m].len() {
                let du = h.months[m][t].u - h.months[m - 12][t].u - (h.nat_u[m] - h.nat_u[m - 12]);
                gov_du.0.push(vec![du * 100.0]);
                gov_du
                    .1
                    .push((h.months[m][t].gov - h.months[m - 12][t].gov) * 100.0);
            }
        }
    }
    let (a, b) = terciles(&xs);
    let bin = |lo: f64, hi: f64, filt: &dyn Fn(usize) -> bool| -> Value {
        let idx: Vec<usize> = (0..xs.len())
            .filter(|i| xs[*i] >= lo && xs[*i] <= hi && filt(*i))
            .collect();
        let w: Vec<f64> = idx.iter().map(|i| won[*i]).collect();
        json!({ "races": w.len(), "reelected": r3(mean(&w)), "mean_rel_du_pts": r3(mean(&idx.iter().map(|i| xs[*i]).collect::<Vec<_>>())) })
    };
    let all = |_: usize| true;
    let execs = |i: usize| office[i];
    let seats = |i: usize| !office[i];
    let rows = json!({
        "all": [bin(f64::MIN, a, &all), bin(a, b, &all), bin(b, f64::MAX, &all)],
        "executive": [bin(f64::MIN, a, &execs), bin(a, b, &execs), bin(b, f64::MAX, &execs)],
        "council": [bin(f64::MIN, a, &seats), bin(a, b, &seats), bin(b, f64::MAX, &seats)],
    });
    let low = rows["all"][0]["reelected"].as_f64().unwrap_or(0.0);
    let high = rows["all"][2]["reelected"].as_f64().unwrap_or(0.0);
    let (beta, t, r2) = ols(&xs.iter().map(|x| vec![*x]).collect::<Vec<_>>(), &won);
    let (gb, gt, gr2) = ols(&gov_du.0, &gov_du.1);
    let (gwb, gwt, _) = ols(
        &gov.iter().map(|x| vec![x * 100.0]).collect::<Vec<_>>(),
        &won,
    );
    let gap = low - high;
    json!({
        "question": "does a rise in local unemployment (relative to the nation's) lower re-election?",
        "terciles_rel_du_pts": [r3(a), r3(b)],
        "by_tercile": rows,
        "linear_probability_per_pt": { "beta": r3(beta[1]), "t": r1(t[1]), "r2": r3(r2) },
        "approval_change_per_pt": { "beta": r3(gb[1]), "t": r1(gt[1]), "r2": r3(gr2), "n": gov_du.1.len() },
        "reelection_per_approval_pt": { "beta": r3(gwb[1]), "t": r1(gwt[1]) },
        "gap_low_minus_high": r3(gap),
        "verdict": if gap >= 0.10 { "works" } else if gap >= 0.03 { "weak" } else { "broken" },
    })
}

/// Rent against supply and demand: yearly rent growth per town.
fn rent(hs: &[History]) -> Value {
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut inc = (Vec::new(), Vec::new());
    for h in hs {
        for m in (12..h.months.len()).step_by(12) {
            for t in 0..h.months[m].len() {
                let (a, b) = (&h.months[m - 12][t], &h.months[m][t]);
                if a.rent <= 0.0 || a.pop <= 0.0 || a.stock <= 0.0 {
                    continue;
                }
                let g = (b.rent / a.rent - 1.0) * 100.0;
                let hh = |l: &Lite| l.stock * (1.0 - l.vacancy);
                let demand = (hh(b) / hh(a).max(1.0) - 1.0) * 100.0;
                let supply = (b.stock / a.stock - 1.0) * 100.0;
                x.push(vec![a.vacancy * 100.0, demand - supply]);
                y.push(g);
                inc.0.push(vec![(b.wage / a.wage.max(1.0) - 1.0) * 100.0]);
                inc.1.push(g);
            }
        }
    }
    let (b, t, r2) = ols(&x, &y);
    let (ib, it, ir2) = ols(&inc.0, &inc.1);
    let ok = b[1] < 0.0 && t[1] < -2.0 && b[2] > 0.0 && t[2] > 2.0;
    json!({
        "question": "does rent rise when vacancy is low and households outgrow the stock?",
        "n": y.len(),
        "rent_growth_pct_on": {
            "vacancy_pts": { "beta": r3(b[1]), "t": r1(t[1]) },
            "household_minus_stock_growth_pts": { "beta": r3(b[2]), "t": r1(t[2]) },
            "r2": r3(r2),
        },
        "rent_growth_pct_on_wage_growth_pct": { "beta": r3(ib[1]), "t": r1(it[1]), "r2": r3(ir2) },
        "verdict": if ok { "works" } else { "weak" },
    })
}

/// Migration against jobs, real wages and rent: yearly population growth
/// per town (births and deaths included).
fn migration(hs: &[History]) -> Value {
    let mut x = Vec::new();
    let mut y = Vec::new();
    for h in hs {
        for m in (12..h.months.len()).step_by(12) {
            let n = h.months[m].len() as f64;
            let mean_rw: f64 =
                h.months[m - 12].iter().map(|l| l.real_wage).sum::<f64>() / n.max(1.0);
            for t in 0..h.months[m].len() {
                let (a, b) = (&h.months[m - 12][t], &h.months[m][t]);
                if a.pop <= 0.0 || a.real_wage <= 0.0 {
                    continue;
                }
                x.push(vec![
                    (a.u - h.nat_u[m - 12]) * 100.0,
                    mandate_core::math::ln(a.real_wage / mean_rw.max(1.0)) * 100.0,
                    a.rent_burden * 100.0,
                ]);
                y.push((b.pop / a.pop - 1.0) * 100.0);
            }
        }
    }
    let (b, t, r2) = ols(&x, &y);
    let signs = [
        b[1] < 0.0 && t[1] < -2.0,
        b[2] > 0.0 && t[2] > 2.0,
        b[3] < 0.0 && t[3] < -2.0,
    ];
    let good = signs.iter().filter(|s| **s).count();
    json!({
        "question": "do people move toward jobs and real wages, and away from high rents?",
        "n": y.len(),
        "pop_growth_pct_on": {
            "rel_unemployment_pts": { "beta": r3(b[1]), "t": r1(t[1]), "expected": "-" },
            "rel_real_wage_log_pct": { "beta": r3(b[2]), "t": r1(t[2]), "expected": "+" },
            "rent_burden_pts": { "beta": r3(b[3]), "t": r1(t[3]), "expected": "-" },
            "r2": r3(r2),
        },
        "verdict": match good { 3 => "works", 1 | 2 => "weak", _ => "broken" },
    })
}

/// Incumbency: how often incumbents run and win, by kind of post.
fn incumbency(hs: &[History]) -> Value {
    let es: Vec<&crate::lab::metrics::Election> = hs
        .iter()
        .flat_map(|h| h.tally.elections.iter())
        .filter(|e| e.town.is_some())
        .collect();
    let rate = |f: &dyn Fn(&&crate::lab::metrics::Election) -> bool| {
        let ran: Vec<_> = es.iter().filter(|e| f(e) && e.incumbent_ran).collect();
        let all = es.iter().filter(|e| f(e) && e.incumbent.is_some()).count();
        let won = ran.iter().filter(|e| e.incumbent_won).count();
        json!({
            "races_with_incumbent": all,
            "incumbent_ran_share": r3(ran.len() as f64 / all.max(1) as f64),
            "reelection_rate": r3(won as f64 / ran.len().max(1) as f64),
        })
    };
    let all = rate(&|_| true);
    let r = all["reelection_rate"].as_f64().unwrap_or(0.0);
    // Party holding a town's executive at election: kept or lost.
    let party_kept = {
        let ex: Vec<_> = es
            .iter()
            .filter(|e| e.office && e.incumbent_party.is_some())
            .collect();
        let kept = ex
            .iter()
            .filter(|e| e.winner_party == e.incumbent_party)
            .count();
        r3(kept as f64 / ex.len().max(1) as f64)
    };
    let margins: Vec<f64> = es.iter().map(|e| e.margin).collect();
    let competitive =
        margins.iter().filter(|m| **m < 0.10).count() as f64 / margins.len().max(1) as f64;
    json!({
        "question": "is there an incumbency advantage, and is it beatable?",
        "all": all,
        "executive": rate(&|e| e.office),
        "council": rate(&|e| !e.office),
        "executive_party_kept_share": party_kept,
        "competitive_share_margin_lt_10pts": r3(competitive),
        "bands": { "TEST_STRATEGY_reelection": [0.55, 0.85], "SIMULATION_MODELS_reelection": [0.65, 0.80], "competitive": [0.15, 0.50] },
        "verdict": if (0.55..=0.85).contains(&r) { "works" } else { "weak" },
    })
}

/// Turnout habits by age band and income class at the end of the runs.
fn turnout(hs: &[History]) -> Value {
    let mut t = [[0.0; 3]; 4];
    for h in hs {
        for a in 0..4 {
            for k in 0..3 {
                t[a][k] += h.turnout[a][k] / hs.len().max(1) as f64;
            }
        }
    }
    let age: Vec<f64> = (0..4)
        .map(|a| (t[a][0] + t[a][1] + t[a][2]) / 3.0)
        .collect();
    let class: Vec<f64> = (0..3)
        .map(|k| (0..4).map(|a| t[a][k]).sum::<f64>() / 4.0)
        .collect();
    let rises_with_age = age[3] > age[0] + 0.15;
    let rises_with_class = class[2] > class[0] + 0.08;
    json!({
        "question": "do turnout habits rise with age and class?",
        "by_age_band_18_29_30_44_45_64_65plus": age.iter().map(|x| r3(*x)).collect::<Vec<_>>(),
        "by_class_low_mid_high": class.iter().map(|x| r3(*x)).collect::<Vec<_>>(),
        "grid": t.iter().map(|r| r.iter().map(|x| r3(*x)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "verdict": match (rises_with_age, rises_with_class) { (true, true) => "works", (false, false) => "broken", _ => "weak" },
    })
}

/// Recessions by sector: twins with and without a recession at month 24,
/// national jobs per sector 12 months on.
fn recession(cx: &Ctx) -> Result<Value, String> {
    let worlds = cx.worlds();
    let res = par_map(
        &worlds,
        threads(),
        |(s, seed)| -> Result<Vec<(String, f64)>, String> {
            let mut g = Game::new(
                &cx.content,
                &Setup::small(*seed, s.as_deref()),
                Driver::Idle,
            )?;
            g.run_to(24, &[], &mut Vec::new());
            let mut a = g.fork()?;
            let mut b = g.fork()?;
            b.intervene(&Intervention::Recession {
                depth: 0.10,
                months: 18,
            })?;
            let (mut sa, mut sb) = (Vec::new(), Vec::new());
            a.run_to(36, &[36], &mut sa);
            b.run_to(36, &[36], &mut sb);
            let tot = |s: &crate::lab::metrics::Snapshot| {
                let mut m = std::collections::BTreeMap::<String, f64>::new();
                for t in &s.towns {
                    for (k, v) in &t.jobs {
                        *m.entry(k.clone()).or_default() += v;
                    }
                }
                m
            };
            let (ja, jb) = (tot(&sa[0]), tot(&sb[0]));
            let mut out: Vec<(String, f64)> = ja
                .iter()
                .map(|(k, v)| (k.clone(), pct(*v, jb.get(k).copied().unwrap_or(0.0))))
                .collect();
            out.push((
                "unemployment_pts".into(),
                (sb[0].nation.unemployment - sa[0].nation.unemployment) * 100.0,
            ));
            Ok(out)
        },
    );
    let mut by: std::collections::BTreeMap<String, Vec<f64>> = Default::default();
    for r in res {
        for (k, v) in r? {
            by.entry(k).or_default().push(v);
        }
    }
    let drivers: std::collections::BTreeMap<String, String> = cx
        .content
        .sectors
        .iter()
        .map(|s| {
            (
                s.id.trim_start_matches("sector.").to_string(),
                format!("{:?}", s.driver),
            )
        })
        .collect();
    let rows: Vec<Value> = by
        .iter()
        .map(|(k, v)| json!({ "sector": k, "driver": drivers.get(k), "jobs_pct_12m": r3(mean(v)), "t": r1(t_stat(v)) }))
        .collect();
    let sect: Vec<f64> = by
        .iter()
        .filter(|(k, _)| k.as_str() != "unemployment_pts")
        .map(|(_, v)| mean(v))
        .collect();
    let spread = quantile(&sect, 1.0) - quantile(&sect, 0.0);
    Ok(json!({
        "question": "does a recession (10% demand, 18 months, at month 24) hit sectors differently?",
        "by_sector": rows,
        "spread_pts": r3(spread),
        "verdict": if spread >= 3.0 { "works" } else if spread >= 1.0 { "weak" } else { "broken" },
    }))
}

/// A plant closure in the home town (half its largest industrial sector,
/// month 24): the neighbourhoods' jobs worry, approval and the next votes.
fn closure(cx: &Ctx) -> Result<Value, String> {
    let worlds = cx.worlds();
    let res = par_map(&worlds, threads(), |(s, seed)| -> Result<Value, String> {
        let mut g = Game::new(
            &cx.content,
            &Setup::small(*seed, s.as_deref()),
            Driver::Idle,
        )?;
        g.detail = vec![g.home()];
        g.run_to(24, &[], &mut Vec::new());
        let home = g.home() as usize;
        let mut a = g.fork()?;
        let mut b = g.fork()?;
        b.intervene(&Intervention::PlantClosure {
            town: home as u32,
            sector: None,
            share: 0.5,
        })?;
        let (mut sa, mut sb) = (Vec::new(), Vec::new());
        a.run_to(72, &[30, 36, 72], &mut sa);
        b.run_to(72, &[30, 36, 72], &mut sb);
        let at = |i: usize| -> Value {
            let (ta, tb) = (&sa[i].towns[home], &sb[i].towns[home]);
            let hood: Vec<f64> = ta
                .hoods
                .iter()
                .zip(&tb.hoods)
                .map(|(x, y)| {
                    (y.concerns.get("jobs").copied().unwrap_or(0.0)
                        - x.concerns.get("jobs").copied().unwrap_or(0.0))
                        * 100.0
                })
                .collect();
            let hood_gov: Vec<f64> = ta
                .hoods
                .iter()
                .zip(&tb.hoods)
                .map(|(x, y)| (y.gov_approval - x.gov_approval) * 100.0)
                .collect();
            json!({
                "u_pts": (tb.unemployment - ta.unemployment) * 100.0,
                "pop_pct": pct(ta.population as f64, tb.population as f64),
                "gov_pts": (tb.gov_approval - ta.gov_approval) * 100.0,
                "worry_jobs_town_pts": (tb.concerns.get("jobs").copied().unwrap_or(0.0) - ta.concerns.get("jobs").copied().unwrap_or(0.0)) * 100.0,
                "worry_jobs_hood_min": quantile(&hood, 0.0),
                "worry_jobs_hood_max": quantile(&hood, 1.0),
                "gov_hood_min": quantile(&hood_gov, 0.0),
                "gov_hood_max": quantile(&hood_gov, 1.0),
                "crisis_raised": tb.crises.iter().any(|c| c == "crisis.plant_closure") && !ta.crises.iter().any(|c| c == "crisis.plant_closure"),
            })
        };
        let races = |g: &Game| -> Vec<(String, u32, bool)> {
            g.tally
                .elections
                .iter()
                .filter(|e| e.town == Some(home as u32) && e.month > 24)
                .map(|e| {
                    (
                        format!("{}{}{:?}", e.date, e.post, e.constituency),
                        e.winner,
                        e.incumbent_won,
                    )
                })
                .collect()
        };
        let (ra, rb) = (races(&a), races(&b));
        let flipped = ra
            .iter()
            .filter(|(k, w, _)| rb.iter().any(|(k2, w2, _)| k2 == k && w2 != w))
            .count();
        let inc = |r: &[(String, u32, bool)]| r.iter().filter(|x| x.2).count();
        Ok(
            json!({ "m6": at(0), "m12": at(1), "m48": at(2), "races": ra.len(), "flipped": flipped, "incumbents_won_base": inc(&ra), "incumbents_won_closure": inc(&rb) }),
        )
    });
    let rs: Vec<Value> = res.into_iter().collect::<Result<_, _>>()?;
    let col = |when: &str, k: &str| -> Vec<f64> {
        rs.iter().filter_map(|r| r[when][k].as_f64()).collect()
    };
    let mut table = serde_json::Map::new();
    for w in ["m6", "m12", "m48"] {
        let mut o = serde_json::Map::new();
        for k in [
            "u_pts",
            "pop_pct",
            "gov_pts",
            "worry_jobs_town_pts",
            "worry_jobs_hood_min",
            "worry_jobs_hood_max",
            "gov_hood_min",
            "gov_hood_max",
        ] {
            let v = col(w, k);
            o.insert(
                k.into(),
                json!({ "mean": r3(mean(&v)), "t": r1(t_stat(&v)) }),
            );
        }
        let raised = rs
            .iter()
            .filter(|r| r[w]["crisis_raised"].as_bool().unwrap_or(false))
            .count();
        o.insert(
            "crisis_raised_share".into(),
            json!(r3(raised as f64 / rs.len().max(1) as f64)),
        );
        table.insert(w.into(), Value::Object(o));
    }
    let races: u64 = rs.iter().filter_map(|r| r["races"].as_u64()).sum();
    let flipped: u64 = rs.iter().filter_map(|r| r["flipped"].as_u64()).sum();
    let iw_a: u64 = rs
        .iter()
        .filter_map(|r| r["incumbents_won_base"].as_u64())
        .sum();
    let iw_b: u64 = rs
        .iter()
        .filter_map(|r| r["incumbents_won_closure"].as_u64())
        .sum();
    let worry = mean(&col("m12", "worry_jobs_town_pts"));
    let spread =
        mean(&col("m12", "worry_jobs_hood_max")) - mean(&col("m12", "worry_jobs_hood_min"));
    Ok(json!({
        "question": "does a plant closure show up in its neighbourhoods' worries and votes?",
        "after_closure": table,
        "home_races_after": races,
        "races_flipped": flipped,
        "incumbent_wins_base_vs_closure": [iw_a, iw_b],
        "hood_spread_jobs_worry_pts_m12": r3(spread),
        "verdict": if worry > 2.0 && flipped > 0 && iw_b < iw_a { "works" } else if worry > 2.0 { "weak" } else { "broken" },
    }))
}

/// The three core relationships, for the nightly gate.
pub fn core(cx: &Ctx, seeds: u64, years: u32) -> Result<Value, String> {
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
    let (a, b, c) = (retrospective(&hs), rent(&hs), migration(&hs));
    let ok = |v: &Value| v["verdict"] != "broken";
    // Retrospective voting must work, not merely show (audit fix 1): a gap
    // of 10 points between the best and worst terciles, and approval worth
    // at least half a point of re-election per point.
    let retro_ok = a["verdict"] == "works"
        && a["reelection_per_approval_pt"]["beta"]
            .as_f64()
            .is_some_and(|b| b >= 0.005);
    Ok(
        json!({ "pass": retro_ok && ok(&b) && ok(&c), "retrospective": a, "rent": b, "migration": c }),
    )
}

pub fn run(cx: &Ctx) -> Result<Value, String> {
    let years = cx.years(20);
    let hs = histories(cx, years, Driver::Idle)?;
    Ok(json!({
        "params": { "seeds_per_scenario": cx.args.seeds, "years": years, "driver": "idle", "scenarios": cx.scenarios().iter().map(short_scenario).collect::<Vec<_>>() },
        "retrospective": retrospective(&hs),
        "rent": rent(&hs),
        "migration": migration(&hs),
        "incumbency": incumbency(&hs),
        "turnout": turnout(&hs),
        "recession_by_sector": recession(cx)?,
        "plant_closure": closure(cx)?,
    }))
}
