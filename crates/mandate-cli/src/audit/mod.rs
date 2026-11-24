//! The simulation audit (`mandate-cli audit <topic>`): experiments on the
//! lab runner that ask whether the simulation is *effective* (believable,
//! responsive and readable), not only stable. Each topic prints JSON: its
//! parameters, result tables, and a verdict per check (`works` | `weak` |
//! `broken` | `missing`) computed from thresholds written beside it.
//! `docs/design/research/SIMULATION_AUDIT.md` is the report; every table
//! there is one of these commands' outputs.

// Statistics loops index several parallel arrays; range loops are clearer here.
#![allow(clippy::needless_range_loop)]

mod believe;
mod claims;
mod diverge;
mod leverage;
mod policy;
mod readability;
mod sensitivity;
mod stuck;

use mandate_content::ContentBundle;
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(clap::Args, Clone)]
pub struct AuditArgs {
    /// policy | believe | sensitivity | diverge | leverage | stuck |
    /// readability | claims | gate (the nightly gate's cheap checks)
    pub topic: String,
    /// Seeds per scenario (or per run set).
    #[arg(long, default_value_t = 8)]
    pub seeds: u64,
    /// Years to run (topics with a default of their own ignore 0).
    #[arg(long, default_value_t = 0)]
    pub years: u32,
    /// Only these scenario ids (`none` for a random country); default all.
    #[arg(long)]
    pub scenario: Vec<String>,
    /// Only these policy templates or parameters (topic-specific filter).
    #[arg(long)]
    pub only: Vec<String>,
}

pub struct Ctx {
    pub content: Arc<ContentBundle>,
    pub args: AuditArgs,
}

impl Ctx {
    /// The canonical worlds (DEC-078 decision 4): a random country and
    /// each scenario of the base pack, or those asked for.
    pub fn scenarios(&self) -> Vec<Option<String>> {
        let mut all: Vec<Option<String>> = vec![None];
        all.extend(
            self.content
                .scenarios
                .iter()
                .map(|s| Some(s.id.to_string())),
        );
        if self.args.scenario.is_empty() {
            return all;
        }
        all.into_iter()
            .filter(|s| {
                let short = s
                    .as_deref()
                    .map(|x| x.trim_start_matches("scenario."))
                    .unwrap_or("none");
                self.args
                    .scenario
                    .iter()
                    .any(|a| a == short || Some(a.as_str()) == s.as_deref())
            })
            .collect()
    }
    pub fn years(&self, default: u32) -> u32 {
        if self.args.years == 0 {
            default
        } else {
            self.args.years
        }
    }
    /// (scenario, seed) pairs: `seeds` per scenario.
    pub fn worlds(&self) -> Vec<(Option<String>, u64)> {
        self.scenarios()
            .into_iter()
            .flat_map(|s| (1..=self.args.seeds).map(move |k| (s.clone(), k)))
            .collect()
    }
}

pub fn short_scenario(s: &Option<String>) -> String {
    s.as_deref()
        .map(|x| x.trim_start_matches("scenario.").to_string())
        .unwrap_or_else(|| "none".into())
}

pub fn run(content: Arc<ContentBundle>, args: AuditArgs) -> Result<Value, String> {
    let topic = args.topic.clone();
    let cx = Ctx { content, args };
    let mut v = match topic.as_str() {
        "policy" => policy::run(&cx)?,
        "believe" => believe::run(&cx)?,
        "sensitivity" => sensitivity::run(&cx)?,
        "diverge" => diverge::run(&cx)?,
        "leverage" => leverage::run(&cx)?,
        "stuck" => stuck::run(&cx)?,
        "readability" => readability::run(&cx)?,
        "claims" => claims::run(&cx)?,
        "gate" => gate(&cx)?,
        t => return Err(format!("unknown audit topic `{t}`")),
    };
    v["topic"] = json!(topic);
    Ok(v)
}

/// The nightly gate's cheapest checks (report §"Nightly gate"): the
/// believable-behaviour core (unemployment hurts incumbents; rent follows
/// vacancy; migration follows jobs), policy responsiveness on the home
/// town for every template, and rent sanity (no runaway town, median real
/// rent growth within ±50% over 30 years), at the gate's sample sizes.
fn gate(cx: &Ctx) -> Result<Value, String> {
    let b = believe::core(cx, 4, 12)?;
    let p = policy::gate(cx, 3)?;
    let r = stuck::rent_gate(cx, 2, 30)?;
    let ok = |v: &Value| v["pass"].as_bool().unwrap_or(false);
    let pass = ok(&b) && ok(&p) && ok(&r);
    Ok(json!({ "pass": pass, "believe": b, "policy": p, "rent": r }))
}

// --- Small statistics ----------------------------------------------------------------

pub fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

pub fn sd(v: &[f64]) -> f64 {
    if v.len() < 2 {
        return 0.0;
    }
    let m = mean(v);
    (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (v.len() - 1) as f64).sqrt()
}

pub fn quantile(v: &[f64], q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    let i = ((s.len() - 1) as f64 * q).round() as usize;
    s[i.min(s.len() - 1)]
}

pub fn median(v: &[f64]) -> f64 {
    quantile(v, 0.5)
}

/// Mean over standard error (0 when there is no spread and no mean).
pub fn t_stat(v: &[f64]) -> f64 {
    let s = sd(v);
    let m = mean(v);
    if s == 0.0 {
        if m == 0.0 { 0.0 } else { m.signum() * 99.0 }
    } else {
        m / (s / (v.len() as f64).sqrt())
    }
}

pub fn corr(x: &[f64], y: &[f64]) -> f64 {
    let (mx, my) = (mean(x), mean(y));
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for (a, b) in x.iter().zip(y) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx) * (a - mx);
        syy += (b - my) * (b - my);
    }
    if sxx <= 0.0 || syy <= 0.0 {
        0.0
    } else {
        sxy / (sxx * syy).sqrt()
    }
}

/// Ordinary least squares with an intercept: coefficients (intercept
/// first), their t statistics, and R².
pub fn ols(xs: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, Vec<f64>, f64) {
    let n = y.len();
    let k = xs.first().map(|r| r.len()).unwrap_or(0) + 1;
    if n <= k {
        return (vec![0.0; k], vec![0.0; k], 0.0);
    }
    let row = |i: usize| -> Vec<f64> {
        let mut r = vec![1.0];
        r.extend_from_slice(&xs[i]);
        r
    };
    let mut xtx = vec![vec![0.0; k]; k];
    let mut xty = vec![0.0; k];
    for i in 0..n {
        let r = row(i);
        for a in 0..k {
            xty[a] += r[a] * y[i];
            for b in 0..k {
                xtx[a][b] += r[a] * r[b];
            }
        }
    }
    let Some(inv) = invert(&xtx) else {
        return (vec![0.0; k], vec![0.0; k], 0.0);
    };
    let beta: Vec<f64> = (0..k)
        .map(|a| (0..k).map(|b| inv[a][b] * xty[b]).sum())
        .collect();
    let my = mean(y);
    let (mut ssr, mut sst) = (0.0, 0.0);
    for i in 0..n {
        let r = row(i);
        let fit: f64 = r.iter().zip(&beta).map(|(a, b)| a * b).sum();
        ssr += (y[i] - fit) * (y[i] - fit);
        sst += (y[i] - my) * (y[i] - my);
    }
    let s2 = ssr / (n - k) as f64;
    let t = (0..k)
        .map(|a| {
            let se = (s2 * inv[a][a]).max(0.0).sqrt();
            if se > 0.0 { beta[a] / se } else { 0.0 }
        })
        .collect();
    let r2 = if sst > 0.0 { 1.0 - ssr / sst } else { 0.0 };
    (beta, t, r2)
}

fn invert(m: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = m.len();
    let mut a: Vec<Vec<f64>> = m
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut x = r.clone();
            x.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
            x
        })
        .collect();
    for c in 0..n {
        let p = (c..n).max_by(|x, y| a[*x][c].abs().total_cmp(&a[*y][c].abs()))?;
        if a[p][c].abs() < 1e-12 {
            return None;
        }
        a.swap(c, p);
        let d = a[c][c];
        for x in a[c].iter_mut() {
            *x /= d;
        }
        for r in 0..n {
            if r != c {
                let f = a[r][c];
                if f != 0.0 {
                    for j in 0..2 * n {
                        a[r][j] -= f * a[c][j];
                    }
                }
            }
        }
    }
    Some(a.into_iter().map(|r| r[n..].to_vec()).collect())
}

/// Round for tables.
pub fn r3(x: f64) -> f64 {
    if x.is_finite() {
        (x * 1000.0).round() / 1000.0
    } else {
        0.0
    }
}

pub fn r1(x: f64) -> f64 {
    if x.is_finite() {
        (x * 10.0).round() / 10.0
    } else {
        0.0
    }
}

/// Relative change in percent (0 when the base is 0).
pub fn pct(base: f64, treated: f64) -> f64 {
    if base.abs() < 1e-12 {
        0.0
    } else {
        (treated - base) / base.abs() * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ols_recovers_a_known_line() {
        let xs: Vec<Vec<f64>> = (0..50)
            .map(|i| vec![i as f64, (i * i % 7) as f64])
            .collect();
        let y: Vec<f64> = xs.iter().map(|r| 2.0 + 3.0 * r[0] - 0.5 * r[1]).collect();
        let (b, _, r2) = ols(&xs, &y);
        assert!(
            (b[0] - 2.0).abs() < 1e-6 && (b[1] - 3.0).abs() < 1e-6 && (b[2] + 0.5).abs() < 1e-6
        );
        assert!((r2 - 1.0).abs() < 1e-9);
        assert!((corr(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]) - 1.0).abs() < 1e-12);
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
    }
}

// --- Long baseline histories (topics 2, 4, 6) ----------------------------------------

/// A town's main numbers in one month (compact: histories keep every month).
#[derive(Clone, Copy, Debug, Default)]
pub struct Lite {
    pub u: f64,
    pub wage: f64,
    pub real_wage: f64,
    pub rent: f64,
    pub rent_burden: f64,
    pub vacancy: f64,
    pub starts: f64,
    pub stock: f64,
    pub pop: f64,
    pub gov: f64,
    pub debt: f64,
    pub exec_party: i64,
}

impl Lite {
    pub fn of(t: &crate::lab::metrics::Town) -> Lite {
        Lite {
            u: t.unemployment,
            wage: t.mean_wage,
            real_wage: t.real_wage,
            rent: t.rent,
            rent_burden: t.rent_burden,
            vacancy: t.vacancy,
            starts: t.starts_index,
            stock: t.housing_stock,
            pop: t.population as f64,
            gov: t.gov_approval,
            debt: t.debt,
            exec_party: t.executive_party.map(|p| p as i64).unwrap_or(-1),
        }
    }
}

/// One baseline world over the years.
pub struct History {
    pub scenario: String,
    pub seed: u64,
    /// `months[m][town]`, month 0 to the end.
    pub months: Vec<Vec<Lite>>,
    /// The national unemployment rate, monthly.
    pub nat_u: Vec<f64>,
    /// A full snapshot every 12 months (month 0, 12, …).
    pub years: Vec<crate::lab::metrics::Snapshot>,
    pub tally: crate::lab::metrics::Tally,
    /// Turnout habit (last turnout) by age band × class at the end.
    pub turnout: [[f64; 3]; 4],
    /// Mean attitudes per town (registry order) every 12 months.
    pub attitudes: Vec<Vec<Vec<f64>>>,
}

fn town_attitudes(e: &mandate_sim::Engine) -> Vec<Vec<f64>> {
    let st = e.state();
    let na = st.population.attitude_count();
    st.world
        .municipalities
        .iter()
        .map(|mu| {
            let mut acc = vec![0.0; na];
            let mut n = 0.0;
            for h in &mu.neighborhoods {
                for c in &st.population.hoods[h.index()].cohorts {
                    for (k, a) in c.attitudes.iter().enumerate().take(na) {
                        acc[k] += *a as f64 * c.count as f64;
                    }
                    n += c.count as f64;
                }
            }
            acc.iter().map(|x| x / f64::max(n, 1.0)).collect()
        })
        .collect()
}

pub fn history(
    cx: &Ctx,
    scenario: &Option<String>,
    seed: u64,
    years: u32,
    driver: crate::lab::runner::Driver,
) -> Result<History, String> {
    use crate::lab::runner::{Game, Setup};
    let mut g = Game::new(
        &cx.content,
        &Setup::small(seed, scenario.as_deref()),
        driver,
    )?;
    let mut months = Vec::new();
    let mut nat_u = Vec::new();
    let mut ys = Vec::new();
    let mut atts = Vec::new();
    for m in 0..=years * 12 {
        if m > 0 {
            g.step_month();
        }
        let s = g.snapshot();
        months.push(s.towns.iter().map(Lite::of).collect());
        nat_u.push(s.nation.unemployment);
        if m % 12 == 0 {
            ys.push(s);
            atts.push(town_attitudes(&g.e));
        }
    }
    let mut turnout = [[0.0; 3]; 4];
    let mut w = [[0.0; 3]; 4];
    for h in &g.e.state().population.hoods {
        for (ci, c) in h.cohorts.iter().enumerate() {
            let (a, k) = (
                mandate_sim::population::age_of(ci),
                mandate_sim::population::class_of(ci),
            );
            turnout[a][k] += c.last_turnout as f64 * c.count as f64;
            w[a][k] += c.count as f64;
        }
    }
    for a in 0..4 {
        for k in 0..3 {
            turnout[a][k] /= f64::max(w[a][k], 1.0);
        }
    }
    Ok(History {
        scenario: short_scenario(scenario),
        seed,
        months,
        nat_u,
        years: ys,
        tally: g.tally.clone(),
        turnout,
        attitudes: atts,
    })
}

pub fn histories(
    cx: &Ctx,
    years: u32,
    driver: crate::lab::runner::Driver,
) -> Result<Vec<History>, String> {
    let worlds = cx.worlds();
    crate::lab::runner::par_map(&worlds, crate::lab::runner::threads(), |(s, k)| {
        history(cx, s, *k, years, driver.clone())
    })
    .into_iter()
    .collect()
}
