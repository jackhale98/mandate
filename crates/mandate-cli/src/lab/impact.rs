//! `mandate-cli lab impact calibrate`: fit the impact table (DEC-111,
//! `docs/design/IMPACT_ESTIMATE.md` §3) on twins and write
//! `content/base/impact.jsonc`.
//!
//! For every policy template, at each parameter level, in each world: fork
//! the game at the month of enactment, take the bill's direct quantities
//! exactly as the estimator computes them there (`policy::impact::project`),
//! enact it in one branch, and measure what moved against the other (where
//! the bill was tabled and defeated) at 12, 36 and 60 months. A coefficient is (the outcome's change, less what the
//! bill's already-fitted bases explain) ÷ the basis; a row keeps the 10th,
//! 50th and 90th percentile across twins.
//!
//! **Which twins fit which basis.** A basis is fitted on the twins in which
//! it is the only basis not yet fitted, in the order of `ImpactBasis::ALL`,
//! repeated until nothing more can be fitted. So `spend` comes from laws
//! whose mechanisms do nothing without their event (flood defences), then a
//! housing programme's rows are what is left after its spending, and a rent
//! cap's what is left after the relief paid beside it. No template is named:
//! the order follows from what each bill is made of.
//!
//! **Two kinds of spread.** Two draws of one world differ whatever the law
//! is (it takes a place on the statute book, and a different bill passes or
//! fails after it): that scatter does not grow with the bill, so it is the
//! `law` row's range, the intercept of a line through the first basis's
//! twins. A basis's own range is how its pass-through differs between
//! worlds and sizes: the 10th and 90th percentile over groups of twins (a
//! template at a level in a scenario), each group's seeds averaged. Taking
//! every twin's ratio instead would divide the scatter by a small basis and
//! multiply it back by a large one.
//!
//! The same twins serve `mandate-cli audit estimate` ([`world_twins`]), on
//! other seeds and with the shipped table.
//!
//! Deterministic: the same arguments give the same file, byte for byte
//! (runs come back in input order; quantiles are order statistics).

use super::runner::{Driver, Game, Intervention, Setup, par_map, threads};
use mandate_content::{
    ContentBundle, IMPACT_HORIZONS, ImpactBasis, ImpactFit, ImpactOutcome, ImpactRow, TierKey,
};
use mandate_sim::policy::impact::{self, Estimate, Projection};
use mandate_sim::world::{Jurisdiction, MunicipalityId};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

/// What a calibration or an audit runs.
#[derive(Clone, Debug)]
pub struct Design {
    /// Months (from the run's start) at which laws are enacted.
    pub at: Vec<u32>,
    /// `home`: the player's home town; `classes`: also one town of each
    /// other settlement class (the lowest id).
    pub towns: String,
    /// `default`, `max`.
    pub levels: Vec<String>,
    pub templates: Vec<String>,
    /// How many of the horizons to run (3: to 60 months).
    pub horizons: usize,
}

/// A town at one moment: what the outcomes are read from.
#[derive(Clone, Debug, Default)]
struct Measure {
    stock: f64,
    jobs: f64,
    sector: BTreeMap<String, f64>,
    rent: f64,
    income_low: f64,
    debt: f64,
    services: f64,
    approval: f64,
    class: [f64; 3],
}

fn measure(g: &Game, town: u32) -> Measure {
    let st = g.e.state();
    let m = town as usize;
    let Some(e) = st.economy.munis.get(m) else {
        return Measure::default();
    };
    let r = mandate_sim::scheduler::Reads::of_state(st, &g.content);
    let mut acc = [0.0; 3];
    let mut n = [0.0; 3];
    if let Some(mu) = st.world.municipalities.get(m) {
        for h in &mu.neighborhoods {
            for (ci, c) in st.population.hoods[h.index()].cohorts.iter().enumerate() {
                let k = mandate_sim::population::class_of(ci);
                acc[k] += c.gov_approval as f64 * c.count as f64;
                n[k] += c.count as f64;
            }
        }
    }
    let all: f64 = n.iter().sum();
    Measure {
        stock: e.housing.stock,
        jobs: e.total_jobs(),
        sector: st
            .economy
            .sector_keys
            .iter()
            .zip(&e.sectors)
            .map(|(k, s)| (k.to_string(), s.jobs))
            .collect(),
        rent: e.housing.rent,
        income_low: e.class_income[0],
        debt: st
            .finance
            .local
            .get(m)
            .map(|t| t.debt.units() as f64)
            .unwrap_or(0.0),
        services: 1.0 - mandate_sim::concerns::service_shortfall(&r, Some(&st.population), m),
        approval: if all > 0.0 {
            acc.iter().sum::<f64>() / all
        } else {
            0.0
        },
        class: std::array::from_fn(|k| if n[k] > 0.0 { acc[k] / n[k] } else { 0.0 }),
    }
}

/// One law against its twin without it.
#[derive(Clone, Debug)]
pub struct Twin {
    pub template: String,
    pub level: String,
    pub world: String,
    pub at: u32,
    /// The bill's months by the simulation's arithmetic, at enactment.
    pub projection: Projection,
    /// The estimate at enactment, with the content's table.
    pub estimate: Estimate,
    /// The town at enactment: what shares are shares of.
    pub stock: f64,
    pub jobs: f64,
    pub budget: f64,
    pub sector_jobs: BTreeMap<String, f64>,
    /// Treated − base at each horizon, by the estimate's outcome key
    /// (`homes`, `sector_jobs.retail`, …), in its unit (homes, jobs,
    /// currency, per cent, points).
    pub twin: BTreeMap<String, [f64; 3]>,
    /// The law's ledger, funded share and yearly cost as they turned out.
    pub ledger: [BTreeMap<String, f64>; 3],
    pub funded: [f64; 3],
    pub annual_cost: [f64; 3],
}

fn level_params(t: &mandate_content::PolicyTemplate, level: &str) -> BTreeMap<String, f64> {
    match level {
        "max" => t
            .params
            .iter()
            .filter(|p| !p.hidden)
            .map(|p| (p.id.clone(), p.max))
            .collect(),
        _ => BTreeMap::new(),
    }
}

fn towns_of(g: &Game, mode: &str) -> Vec<u32> {
    let home = g.home();
    let mut v = vec![home];
    if mode == "classes" {
        let st = g.e.state();
        let class_of = |m: u32| {
            st.world
                .municipalities
                .get(m as usize)
                .map(|x| format!("{:?}", x.class))
        };
        let mut seen = vec![class_of(home)];
        for m in 0..st.world.municipalities.len() as u32 {
            let c = class_of(m);
            if !seen.contains(&c) {
                seen.push(c);
                v.push(m);
            }
        }
    }
    v
}

pub fn short_world(s: &Option<String>) -> String {
    s.as_deref()
        .map(|x| x.trim_start_matches("scenario.").to_string())
        .unwrap_or_else(|| "none".into())
}

/// Every twin of a design in one world.
pub fn world_twins(
    content: &Arc<ContentBundle>,
    scenario: &Option<String>,
    seed: u64,
    d: &Design,
) -> Result<Vec<Twin>, String> {
    let mut g = Game::new(
        content,
        &Setup::small(seed, scenario.as_deref()),
        Driver::Idle,
    )?;
    let horizons = &IMPACT_HORIZONS[..d.horizons.clamp(1, 3)];
    let mut at_months = d.at.clone();
    at_months.sort_unstable();
    let mut out = Vec::new();
    for &at in &at_months {
        g.run_to(at, &[], &mut Vec::new());
        let towns = towns_of(&g, &d.towns);
        // One base run serves every town and template: the same game in
        // which the bill was tabled and defeated. The defeated bill does
        // nothing, and keeps the two arms' later bills under the same ids
        // (members' leanings on a bill are keyed by its id), so the twins
        // differ by the law and not by a renumbering.
        let mut b = g.fork()?;
        if let Some(t) = d
            .templates
            .first()
            .and_then(|k| content.policies.get(k.as_str()))
        {
            let home = b.home();
            b.e.setup(|st| {
                let mut p = mandate_sim::policy::probe(t, home, &BTreeMap::new());
                p.id = st.policy.ids.alloc();
                p.status = mandate_sim::policy::PolicyStatus::Ended;
                p.end_reason = Some(mandate_sim::policy::EndReason::Defeated);
                st.policy.instances.push(p);
            });
        }
        let mut base: Vec<Vec<Measure>> = Vec::new();
        for h in horizons {
            b.run_to(at + h, &[], &mut Vec::new());
            base.push(towns.iter().map(|t| measure(&b, *t)).collect());
        }
        for (ti, &town) in towns.iter().enumerate() {
            for tpl in &d.templates {
                let Some(t) = content.policies.get(tpl.as_str()) else {
                    continue;
                };
                for level in &d.levels {
                    let params = level_params(t, level);
                    let probe = mandate_sim::policy::probe(t, town, &params);
                    let resolved = mandate_sim::policy::Resolved::plain(t);
                    let (x, projection, estimate) = {
                        let st = g.e.state();
                        let r = mandate_sim::scheduler::Reads::of_state(st, content);
                        let at_town = Jurisdiction::town(MunicipalityId(town));
                        let Some(x) = impact::inputs(&r, &st.policy, at_town) else {
                            continue;
                        };
                        let pr = impact::project(&x, content, &resolved, &probe);
                        let es = impact::estimate(&x, content, &resolved, &probe);
                        (x, pr, es)
                    };
                    let mut tr = g.fork()?;
                    let id = tr.e.state().policy.instances.len();
                    tr.intervene(&Intervention::Policy {
                        template: tpl.clone(),
                        town,
                        params,
                    })?;
                    let mut twin: BTreeMap<String, [f64; 3]> = BTreeMap::new();
                    let mut ledger: [BTreeMap<String, f64>; 3] = Default::default();
                    let mut funded = [0.0; 3];
                    let mut annual_cost = [0.0; 3];
                    for (h, months) in horizons.iter().enumerate() {
                        tr.run_to(at + months, &[], &mut Vec::new());
                        let (a, z) = (&base[h][ti], measure(&tr, town));
                        let mut put = |k: &str, v: f64| {
                            twin.entry(k.to_string()).or_insert([0.0; 3])[h] = v;
                        };
                        let rel = |a: f64, z: f64| {
                            if a.abs() < 1e-12 {
                                0.0
                            } else {
                                (z - a) / a * 100.0
                            }
                        };
                        put("homes", z.stock - a.stock);
                        put("jobs", z.jobs - a.jobs);
                        put("rent", rel(a.rent, z.rent));
                        put("low_income", rel(a.income_low, z.income_low));
                        put("debt", z.debt - a.debt);
                        put("services", (z.services - a.services) * 100.0);
                        put("approval", (z.approval - a.approval) * 100.0);
                        put("approval_low", (z.class[0] - a.class[0]) * 100.0);
                        put("approval_middle", (z.class[1] - a.class[1]) * 100.0);
                        put("approval_high", (z.class[2] - a.class[2]) * 100.0);
                        for sector in projection.sector_added.keys() {
                            let key =
                                format!("sector_jobs.{}", mandate_sim::policy::sector_key(sector));
                            let of = |m: &Measure| m.sector.get(sector).copied().unwrap_or(0.0);
                            put(&key, of(&z) - of(a));
                        }
                        let st = tr.e.state();
                        if let Some(p) = st.policy.instances.get(id) {
                            ledger[h] = p.ledger.clone();
                            funded[h] = st.finance.funded_share(p.id.0);
                            annual_cost[h] = st
                                .finance
                                .appropriations
                                .get(&p.id.0)
                                .map(|a| a.annual_cost)
                                .unwrap_or(0.0);
                        }
                    }
                    out.push(Twin {
                        template: tpl.clone(),
                        level: level.clone(),
                        world: short_world(scenario),
                        at,
                        projection,
                        estimate,
                        stock: x.stock,
                        jobs: x.jobs,
                        budget: x.budget,
                        sector_jobs: x.sector_jobs.clone(),
                        twin,
                        ledger,
                        funded,
                        annual_cost,
                    });
                }
            }
        }
    }
    Ok(out)
}

/// Every local template of the pack that is still offered.
pub fn templates(content: &ContentBundle, only: &[String]) -> Vec<String> {
    content
        .policies
        .values()
        .filter(|p| p.tier == TierKey::Local && !p.retired)
        .map(|p| p.id.to_string())
        .filter(|k| only.is_empty() || only.iter().any(|o| k.contains(o.as_str())))
        .collect()
}

/// Twins of a design over worlds, in input order.
pub fn all_twins(
    content: &Arc<ContentBundle>,
    worlds: &[(Option<String>, u64)],
    d: &Design,
) -> Result<Vec<Twin>, String> {
    let res = par_map(worlds, threads(), |(s, seed)| {
        world_twins(content, s, *seed, d)
    });
    let mut all = Vec::new();
    for r in res {
        all.extend(r?);
    }
    Ok(all)
}

// --- Fitting ------------------------------------------------------------------------------

/// A basis below this share of the town is not what moved it: such twins
/// neither fit the basis nor wait for it.
fn floor_of(b: ImpactBasis) -> f64 {
    match b {
        // Half a per cent of the budget.
        ImpactBasis::Spend => 0.005,
        _ => 1e-6,
    }
}

/// Fewest twins a row is fitted on.
const MIN_TWINS: usize = 6;

fn quantile(v: &[f64], q: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    let i = ((s.len() - 1) as f64 * q).round() as usize;
    s[i.min(s.len() - 1)]
}

/// Four significant figures: enough for a ratio, and a file a person can read.
fn r4(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return 0.0;
    }
    format!("{x:.3e}").parse().unwrap_or(0.0)
}

/// The outcome's change as a share of the town, as the estimate scales it.
fn share(t: &Twin, o: ImpactOutcome, h: usize) -> f64 {
    let v = t.twin.get(o.key()).map(|x| x[h]).unwrap_or(0.0);
    match o {
        ImpactOutcome::Homes => v / t.stock.max(1.0),
        ImpactOutcome::Jobs => v / t.jobs.max(1.0),
        // Less the bill's own borrowing, which the estimate adds by arithmetic.
        ImpactOutcome::Debt => v / t.budget.max(1.0) - t.projection.borrowed[h],
        _ => v / 100.0,
    }
}

fn basis(t: &Twin, b: ImpactBasis, h: usize) -> f64 {
    t.projection.bases.get(&b).map(|x| x[h]).unwrap_or(0.0)
}

/// Twins of one bill in one kind of world: a template at a level, enacted
/// in a month, in a scenario. Their mean is the bill's effect there with
/// most of what differs from seed to seed averaged away.
fn group(t: &Twin) -> (&str, &str, &str, u32) {
    (&t.template, &t.level, &t.world, t.at)
}

/// A row from per-twin ratios (the median) and per-group ratios (the
/// spread: how the pass-through differs between worlds and sizes, not
/// between two draws of one world, which the `law` row carries).
fn row(
    b: ImpactBasis,
    o: ImpactOutcome,
    months: u32,
    ratios: &[f64],
    groups: &BTreeMap<(&str, &str, &str, u32), (f64, f64)>,
) -> ImpactRow {
    let p50 = quantile(ratios, 0.5);
    let by_group: Vec<f64> = groups
        .values()
        .filter(|(_, x)| x.abs() > 0.0)
        .map(|(y, x)| y / x)
        .collect();
    ImpactRow {
        basis: b,
        outcome: o,
        months,
        p10: r4(quantile(&by_group, 0.1).min(p50)),
        p50: r4(p50),
        p90: r4(quantile(&by_group, 0.9).max(p50)),
        n: ratios.len() as u32,
    }
}

/// What any law does (the intercept), from the twins of the first basis
/// that can be fitted alone: a resistant line through the medians of their
/// smaller and their larger halves (by the basis), read at a basis of
/// zero. `None` when the twins are all of a size, so there is no line.
fn intercept(xy: &[(f64, f64)]) -> Option<f64> {
    let mut s = xy.to_vec();
    s.sort_by(|a, b| a.0.abs().total_cmp(&b.0.abs()));
    let (lo, hi) = s.split_at(s.len() / 2);
    let med = |v: &[(f64, f64)], f: fn(&(f64, f64)) -> f64| {
        quantile(&v.iter().map(f).collect::<Vec<_>>(), 0.5)
    };
    let (x1, y1) = (med(lo, |p| p.0), med(lo, |p| p.1));
    let (x2, y2) = (med(hi, |p| p.0), med(hi, |p| p.1));
    if (x2 - x1).abs() < 0.25 * x2.abs() {
        return None;
    }
    let slope = (y2 - y1) / (x2 - x1);
    Some(y1 - slope * x1)
}

/// Fit every row the twins support.
pub fn fit(twins: &[Twin]) -> Vec<ImpactRow> {
    let mut rows: Vec<ImpactRow> = Vec::new();
    for (h, &months) in IMPACT_HORIZONS.iter().enumerate() {
        for o in ImpactOutcome::ALL {
            if o == ImpactOutcome::SectorJobs {
                // A sector's own jobs against the jobs its ledger counts.
                let mut ratios = Vec::new();
                let mut groups: BTreeMap<_, (f64, f64)> = BTreeMap::new();
                for t in twins {
                    for (sector, added) in &t.projection.sector_added {
                        let jobs = t.sector_jobs.get(sector).copied().unwrap_or(0.0);
                        let key =
                            format!("sector_jobs.{}", mandate_sim::policy::sector_key(sector));
                        let y = t.twin.get(&key).map(|x| x[h]).unwrap_or(0.0);
                        if jobs > 0.0 && added[h].abs() / jobs > 1e-6 {
                            ratios.push(y / added[h]);
                            let g = groups.entry(group(t)).or_default();
                            g.0 += y / jobs;
                            g.1 += added[h] / jobs;
                        }
                    }
                }
                if ratios.len() >= MIN_TWINS {
                    rows.push(row(ImpactBasis::JobsAdded, o, months, &ratios, &groups));
                }
                continue;
            }
            let mut fitted: BTreeMap<ImpactBasis, f64> = BTreeMap::new();
            // Whether the intercept (`law`) is fitted yet.
            let mut law = false;
            // What is left of a twin's outcome after the rows fitted so far.
            let left = |t: &Twin, fitted: &BTreeMap<ImpactBasis, f64>| -> f64 {
                share(t, o, h) - fitted.iter().map(|(c, k)| k * basis(t, *c, h)).sum::<f64>()
            };
            loop {
                let mut progress = false;
                for b in ImpactBasis::ALL {
                    if b == ImpactBasis::Law || fitted.contains_key(&b) {
                        continue;
                    }
                    let mine: Vec<&Twin> = twins
                        .iter()
                        .filter(|t| basis(t, b, h).abs() >= floor_of(b))
                        // Every other basis that matters in it is already fitted.
                        .filter(|t| {
                            ImpactBasis::ALL.into_iter().all(|c| {
                                c == b
                                    || c == ImpactBasis::Law
                                    || fitted.contains_key(&c)
                                    || basis(t, c, h).abs() < floor_of(c)
                            })
                        })
                        .collect();
                    if mine.len() < MIN_TWINS {
                        continue;
                    }
                    if !law {
                        law = true;
                        // The first basis fitted gives the intercept: what
                        // any law does, and how far single twins stray.
                        let xy: Vec<(f64, f64)> = mine
                            .iter()
                            .map(|t| (basis(t, b, h), share(t, o, h)))
                            .collect();
                        let a = intercept(&xy).unwrap_or(0.0);
                        let slope = quantile(
                            &xy.iter().map(|(x, y)| (y - a) / x).collect::<Vec<_>>(),
                            0.5,
                        );
                        let stray: Vec<f64> = xy.iter().map(|(x, y)| y - a - slope * x).collect();
                        rows.push(ImpactRow {
                            basis: ImpactBasis::Law,
                            outcome: o,
                            months,
                            p10: r4(a + quantile(&stray, 0.1).min(0.0)),
                            p50: r4(a),
                            p90: r4(a + quantile(&stray, 0.9).max(0.0)),
                            n: xy.len() as u32,
                        });
                        fitted.insert(ImpactBasis::Law, r4(a));
                    }
                    let ratios: Vec<f64> = mine
                        .iter()
                        .map(|t| left(t, &fitted) / basis(t, b, h))
                        .collect();
                    let mut groups: BTreeMap<_, (f64, f64)> = BTreeMap::new();
                    for t in &mine {
                        let g = groups.entry(group(t)).or_default();
                        g.0 += left(t, &fitted);
                        g.1 += basis(t, b, h);
                    }
                    let r = row(b, o, months, &ratios, &groups);
                    fitted.insert(b, r.p50);
                    rows.push(r);
                    progress = true;
                }
                if !progress {
                    break;
                }
            }
        }
    }
    rows.sort_by_key(|r| (r.basis, r.outcome, r.months));
    rows
}

// --- The command ---------------------------------------------------------------------------

/// `tiers` and `arithmetic` are the two things in the table a person
/// writes; a refit keeps what the file has, and a first fit starts here.
fn defaults() -> (Vec<TierKey>, Vec<String>) {
    (
        vec![TierKey::Local],
        [
            "hazard_protection",
            "borrow",
            "revenue",
            "disruption",
            "grievance",
            "labour_supply",
        ]
        .map(String::from)
        .to_vec(),
    )
}

fn tier_word(t: TierKey) -> &'static str {
    match t {
        TierKey::Local => "local",
        TierKey::Regional => "regional",
        TierKey::National => "national",
    }
}

/// The file as text: one row a line, so a refit's diff reads row by row.
pub fn render(
    tiers: &[TierKey],
    arithmetic: &[String],
    fitted: &ImpactFit,
    rows: &[ImpactRow],
) -> String {
    let list = |v: &[String]| {
        v.iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut s = String::new();
    s.push_str("// The impact table (DEC-111, docs/design/IMPACT_ESTIMATE.md): what reaches a town of what\n");
    s.push_str("// a law does directly. A bill's estimate is its direct quantities (the simulation's own\n");
    s.push_str("// arithmetic, as shares of the town) times these coefficients.\n");
    s.push_str("//\n");
    s.push_str("// GENERATED by `mandate-cli lab impact calibrate`. Do not edit `fitted` or `rows`: refit\n");
    s.push_str(
        "// with `mandate-cli lab impact calibrate --refit` (the same run again) or with new\n",
    );
    s.push_str(
        "// arguments, or `scripts/repin.sh --impact`. `tiers` and `arithmetic` are written by\n",
    );
    s.push_str("// hand and kept by a refit. `mandate-cli audit estimate` judges the table on other seeds.\n");
    s.push_str("//\n");
    s.push_str(
        "// A row: across the lab's twins, the 10th, 50th and 90th percentile of (the outcome's\n",
    );
    s.push_str(
        "// change against the same town without the law, less what the bill's other bases\n",
    );
    s.push_str("// explain) ÷ the basis, `months` after enactment; `n` twins. Where the ledger keeps a net\n");
    s.push_str("// figure of its own (homes added, jobs added, low incomes raised: `economy.attribution`),\n");
    s.push_str("// the basis is that figure, so the row is the lab's ratio to it and 1 means the ledger is\n");
    s.push_str("// exactly right: the table never repeats an attribution coefficient.\n");
    s.push_str("{\n  \"kind\": \"impact_table\",\n  \"value\": {\n");
    s.push_str(&format!(
        "    \"tiers\": [{}],\n",
        list(
            &tiers
                .iter()
                .map(|t| tier_word(*t).to_string())
                .collect::<Vec<_>>()
        )
    ));
    s.push_str(&format!("    \"arithmetic\": [{}],\n", list(arithmetic)));
    s.push_str(&format!(
        "    \"fitted\": {},\n",
        serde_json::to_string(fitted).unwrap_or_default()
    ));
    s.push_str("    \"rows\": [\n");
    for r in rows {
        s.push_str(&format!(
            "      {},\n",
            serde_json::to_string(r).unwrap_or_default()
        ));
    }
    s.push_str("    ],\n  },\n}\n");
    s
}

/// The lab's own content for a fit: the tiers to estimate (the file's, or
/// the defaults on a first fit) and no rows, so a twin never depends on the
/// table being replaced.
fn lab_content(content: &ContentBundle) -> (Arc<ContentBundle>, Vec<TierKey>, Vec<String>) {
    let (tiers, arithmetic) = if content.impact.tiers.is_empty() {
        defaults()
    } else {
        (
            content.impact.tiers.clone(),
            content.impact.arithmetic.clone(),
        )
    };
    let mut bare = content.clone();
    bare.impact.tiers = tiers.clone();
    bare.impact.arithmetic = arithmetic.clone();
    bare.impact.rows.clear();
    (Arc::new(bare), tiers, arithmetic)
}

/// `lab impact calibrate`.
pub fn run(content: Arc<ContentBundle>, c: &super::cli::LabCmd) -> Result<Value, String> {
    match c.sub.as_deref() {
        Some("calibrate") => {}
        other => {
            return Err(format!(
                "lab impact {}: use `lab impact calibrate`",
                other.unwrap_or("")
            ));
        }
    }
    let old = &content.impact;
    // `--refit`: the run the shipped table records, again.
    let fit_args = if c.refit {
        if old.fitted.seeds == 0 {
            return Err("--refit: the pack has no fitted impact table to refit".into());
        }
        old.fitted.clone()
    } else {
        let all: Vec<String> = std::iter::once("none".to_string())
            .chain(
                content
                    .scenarios
                    .iter()
                    .map(|s| s.id.trim_start_matches("scenario.").to_string()),
            )
            .collect();
        let worlds = if c.world.is_empty() {
            all
        } else {
            c.world.clone()
        };
        ImpactFit {
            seeds: c.seeds as u32,
            first_seed: c.first_seed,
            worlds,
            levels: c.levels.split(',').map(|s| s.trim().to_string()).collect(),
            enacted_at: c
                .enact_at
                .split(',')
                .map(|x| x.trim().parse().map_err(|_| format!("bad month `{x}`")))
                .collect::<Result<_, String>>()?,
            towns: c.towns.clone(),
            twins: 0,
        }
    };
    let (lab, tiers, arithmetic) = lab_content(&content);
    let design = Design {
        at: fit_args.enacted_at.clone(),
        towns: fit_args.towns.clone(),
        levels: fit_args.levels.clone(),
        templates: templates(&lab, &[]),
        horizons: 3,
    };
    let worlds: Vec<(Option<String>, u64)> = fit_args
        .worlds
        .iter()
        .map(|w| (w != "none").then(|| format!("scenario.{w}")))
        .flat_map(|s| (0..fit_args.seeds as u64).map(move |k| (s.clone(), fit_args.first_seed + k)))
        .collect();
    let twins = all_twins(&lab, &worlds, &design)?;
    let rows = fit(&twins);
    let fitted = ImpactFit {
        twins: twins.len() as u32,
        ..fit_args
    };
    let text = render(&tiers, &arithmetic, &fitted, &rows);
    let path = c
        .out
        .clone()
        .unwrap_or_else(|| super::runner::content_root().join("base/impact.jsonc"));
    let written = if path.as_os_str() == "-" {
        None
    } else {
        std::fs::write(&path, &text).map_err(|e| format!("{}: {e}", path.display()))?;
        Some(path.display().to_string())
    };
    // What the lab measured against the ledger's own net figures: the
    // ratios the end-of-M6 tuning pass would fold into `economy.attribution`.
    let ledger_ratio = |b: ImpactBasis, o: ImpactOutcome| -> Value {
        IMPACT_HORIZONS
            .iter()
            .map(|m| {
                rows.iter()
                    .find(|r| r.basis == b && r.outcome == o && r.months == *m)
                    .map(|r| json!({ "months": m, "p10": r.p10, "p50": r.p50, "p90": r.p90, "n": r.n }))
                    .unwrap_or(Value::Null)
            })
            .collect()
    };
    let mut by_basis: BTreeMap<&str, usize> = BTreeMap::new();
    for r in &rows {
        *by_basis.entry(r.basis.key()).or_default() += 1;
    }
    Ok(json!({
        "written": written,
        "fitted": fitted,
        "rows": rows.len(),
        "rows_by_basis": by_basis,
        "twin_over_ledger": {
            "homes_added (economy.attribution.homes_net)": ledger_ratio(ImpactBasis::HousingSupply, ImpactOutcome::Homes),
            "homes_added, zoning": ledger_ratio(ImpactBasis::Zoning, ImpactOutcome::Homes),
            "jobs_added (jobs_net)": ledger_ratio(ImpactBasis::JobsAdded, ImpactOutcome::SectorJobs),
            "low_income_added (wage_floor_net)": ledger_ratio(ImpactBasis::WageFloor, ImpactOutcome::LowIncome),
            "rent_held": ledger_ratio(ImpactBasis::RentCap, ImpactOutcome::Rent),
        },
        "text": if written.is_none() { Value::String(text) } else { Value::Null },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content() -> Arc<ContentBundle> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        super::super::load(&root, &[]).unwrap_or_else(|e| panic!("{e}"))
    }

    /// Calibration is a pure function of its arguments: the same twins
    /// twice give the same file, and the rows are fitted in an order that
    /// follows from what each bill is made of.
    #[test]
    fn the_same_calibration_twice_gives_the_same_file() {
        let (c, tiers, arithmetic) = lab_content(&content());
        let d = Design {
            at: vec![1],
            towns: "home".into(),
            levels: vec!["default".into(), "max".into()],
            templates: vec![
                "policy.flood_defenses".into(),
                "policy.affordable_housing".into(),
            ],
            horizons: 3,
        };
        let worlds: Vec<(Option<String>, u64)> = (201..=203).map(|s| (None, s)).collect();
        let go = || {
            let twins = all_twins(&c, &worlds, &d).unwrap_or_else(|e| panic!("{e}"));
            let rows = fit(&twins);
            (
                render(&tiers, &arithmetic, &ImpactFit::default(), &rows),
                rows,
            )
        };
        let (a, rows) = go();
        assert_eq!(a, go().0);
        // Spending is fitted on the law that does nothing else (with what
        // any law does), and the housing programme on what that leaves.
        let has = |b, o| {
            rows.iter()
                .any(|r: &ImpactRow| r.basis == b && r.outcome == o)
        };
        assert!(has(ImpactBasis::Law, ImpactOutcome::Jobs), "{a}");
        assert!(has(ImpactBasis::Spend, ImpactOutcome::Jobs), "{a}");
        assert!(has(ImpactBasis::HousingSupply, ImpactOutcome::Homes), "{a}");
        // And the text is a table the loader reads back as it was written.
        let strict = mandate_content::jsonc::to_strict_json(&a);
        let v: Value = serde_json::from_str(&strict).unwrap_or_else(|e| panic!("{e}"));
        let back: Vec<ImpactRow> =
            serde_json::from_value(v["value"]["rows"].clone()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(back, rows);
    }
}
