//! Audit fix 8, part B: the mechanisms that now back three policy texts,
//! each shown in a twin on the home town (the same worlds as topic 1).
//!
//! - **Property tax → housing and rents** ("Homeowners feel it first;
//!   renters see it later in rents"): the property tax at its default
//!   (+0.1 points) against its twin; prices, starts, rents, the stock and
//!   the tax's revenue.
//! - **A rent freeze is a cap**: the emergency rent measure with its freeze
//!   (level 1) against the same measure with none (level 0), so the relief
//!   is in both arms and the difference is the cap; the law is lifted at
//!   month 36 in both, and rents catch up.
//! - **Borrowing costs rise with debt** ("the town's borrowing costs
//!   ease"): a fiscal hole (debt of 1.5 years of revenue) planted in both
//!   arms at month 1, the fiscal bridge enacted in one; the spread, interest
//!   and debt. Beside it, the share of towns paying their tier's spread in
//!   the untouched worlds.

use super::*;
use crate::lab::runner::{Driver, Game, Intervention, Setup, par_map, threads};
use mandate_sim::finance::Revenue;
use std::collections::BTreeMap;

/// What the twins read from the home town each month.
#[derive(Clone, Copy, Debug, Default)]
struct Probe {
    price: f64,
    rent: f64,
    starts: f64,
    stock: f64,
    vacancy: f64,
    households: f64,
    /// Property tax booked this month.
    property: f64,
    spread: f64,
    /// Interest booked this month.
    interest: f64,
    debt: f64,
    /// The year-to-date totals the monthly figures are differenced from.
    property_ytd: f64,
    interest_ytd: f64,
}

fn probe(g: &Game, town: u32, last: &Probe) -> Probe {
    let st = g.e.state();
    let mut p = Probe::default();
    if let Some(e) = st.economy.munis.get(town as usize) {
        let h = &e.housing;
        p.price = h.price;
        p.rent = h.rent;
        p.starts = mandate_sim::economy::housing::starts_index(h);
        p.stock = h.stock;
        p.vacancy = h.vacancy;
        p.households = h.households;
    }
    if let Some(t) = st.finance.local.get(town as usize) {
        // Year-to-date figures restart each January.
        let ytd = t.revenue_ytd[Revenue::PropertyTax as usize].units() as f64;
        let interest = t.interest_ytd.units() as f64;
        let since = |now: f64, before: f64| if now >= before { now - before } else { now };
        p.property = since(ytd, last.property_ytd);
        p.interest = since(interest, last.interest_ytd);
        p.property_ytd = ytd;
        p.interest_ytd = interest;
        p.spread = mandate_sim::finance::spread(&g.content.economy.borrowing, t);
        p.debt = t.debt.units() as f64;
    }
    p
}

/// Run a game month by month to `months`, probing the town each month
/// (index 0 is the month the run starts from). `at(month, game)` may edit
/// the game before the month is stepped.
fn monthly(g: &mut Game, town: u32, months: u32, mut at: impl FnMut(u32, &mut Game)) -> Vec<Probe> {
    let mut out = vec![probe(g, town, &Probe::default())];
    while g.month < months {
        at(g.month, g);
        g.step_month();
        let p = probe(g, town, out.last().unwrap_or(&Probe::default()));
        out.push(p);
    }
    out
}

fn policy(template: &str, town: u32, params: &[(&str, f64)]) -> Intervention {
    Intervention::Policy {
        template: template.into(),
        town,
        params: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
    }
}

/// End the law the lab enacted (its instance id) at once.
fn lift(g: &mut Game, id: usize) {
    g.e.setup(|st| {
        if let Some(p) = st.policy.instances.get_mut(id) {
            p.status = mandate_sim::policy::PolicyStatus::Ended;
            p.end_reason = Some(mandate_sim::policy::EndReason::Repealed);
        }
    });
}

/// Mean and t of a column over worlds.
fn stat(v: &[f64]) -> Value {
    json!({ "mean": r3(mean(v)), "t": r1(t_stat(v)) })
}

fn sum_months(p: &[Probe], from: usize, to: usize, f: impl Fn(&Probe) -> f64) -> f64 {
    p.get(from..=to.min(p.len().saturating_sub(1)))
        .map(|s| s.iter().map(f).sum())
        .unwrap_or(0.0)
}

const START: u32 = 1;

/// The property tax twin: (month, metric) → per-world deltas.
fn property_tax(
    cx: &Ctx,
    s: &Option<String>,
    seed: u64,
) -> Result<Vec<(u32, String, f64)>, String> {
    let mut g = Game::new(&cx.content, &Setup::small(seed, s.as_deref()), Driver::Idle)?;
    g.run_to(START, &[], &mut Vec::new());
    let home = g.home();
    let mut b = g.fork()?;
    let pb = monthly(&mut b, home, 121, |_, _| {});
    let mut x = g.fork()?;
    x.intervene(&policy("policy.property_tax", home, &[]))?;
    let px = monthly(&mut x, home, 121, |_, _| {});
    let mut out = Vec::new();
    for m in [3u32, 6, 12, 24, 36, 60, 84, 120] {
        let i = (m - START) as usize;
        let (a, z) = (&pb[i], &px[i]);
        out.push((m, "price_pct".into(), pct(a.price, z.price)));
        out.push((m, "starts_index".into(), z.starts - a.starts));
        out.push((m, "rent_pct".into(), pct(a.rent, z.rent)));
        out.push((m, "stock_pct".into(), pct(a.stock, z.stock)));
        out.push((m, "vacancy_pts".into(), (z.vacancy - a.vacancy) * 100.0));
        // The tax's revenue over the year to this month.
        let from = i.saturating_sub(11).max(1);
        let rb = sum_months(&pb, from, i, |p| p.property);
        let rx = sum_months(&px, from, i, |p| p.property);
        out.push((m, "property_revenue_pct".into(), pct(rb, rx)));
    }
    Ok(out)
}

/// The rent cap twin: the measure with its freeze against the measure
/// without, lifted at month 36 in both.
fn rent_cap(cx: &Ctx, s: &Option<String>, seed: u64) -> Result<Vec<(u32, String, f64)>, String> {
    let mut g = Game::new(&cx.content, &Setup::small(seed, s.as_deref()), Driver::Idle)?;
    g.run_to(START, &[], &mut Vec::new());
    let home = g.home();
    let arm = |freeze: f64| -> Result<Vec<Probe>, String> {
        let mut x = g.fork()?;
        let id = x.e.state().policy.instances.len();
        x.intervene(&policy(
            "policy.emergency_rent",
            home,
            &[("freeze", freeze)],
        ))?;
        Ok(monthly(&mut x, home, 61, |m, g| {
            if m == LIFT {
                lift(g, id)
            }
        }))
    };
    let (pb, px) = (arm(0.0)?, arm(1.0)?);
    let mut out = Vec::new();
    for m in [6u32, 12, 24, 36, 37, 38, 40, 42, 48, 60] {
        let i = (m - START) as usize;
        let (a, z) = (&pb[i], &px[i]);
        out.push((m, "rent_pct".into(), pct(a.rent, z.rent)));
        out.push((m, "price_pct".into(), pct(a.price, z.price)));
        out.push((m, "starts_index".into(), z.starts - a.starts));
        out.push((m, "stock_pct".into(), pct(a.stock, z.stock)));
        out.push((m, "vacancy_pts".into(), (z.vacancy - a.vacancy) * 100.0));
        out.push((m, "households_pct".into(), pct(a.households, z.households)));
        // Rent growth over the month (the jump when the cap lifts).
        let g = |p: &[Probe]| pct(p[i - 1].rent, p[i].rent);
        out.push((m, "rent_growth_month_pts".into(), g(&px) - g(&pb)));
    }
    Ok(out)
}

const LIFT: u32 = 37;

/// The borrowing twin: a fiscal hole in both arms, the fiscal bridge in one.
fn borrowing(cx: &Ctx, s: &Option<String>, seed: u64) -> Result<Vec<(u32, String, f64)>, String> {
    let mut g = Game::new(&cx.content, &Setup::small(seed, s.as_deref()), Driver::Idle)?;
    g.run_to(START, &[], &mut Vec::new());
    let home = g.home();
    let mut out = Vec::new();
    // Spread before the hole (a prudent town pays its tier's).
    let tier = cx.content.economy.borrowing.spreads[0];
    let before =
        g.e.state()
            .finance
            .local
            .get(home as usize)
            .map(|t| mandate_sim::finance::spread(&cx.content.economy.borrowing, t))
            .unwrap_or(0.0);
    out.push((
        0,
        "spread_above_tier_bp_before_hole".into(),
        (before - tier) * 1e4,
    ));
    g.e.setup(|st| {
        if let Some(t) = st.finance.local.get_mut(home as usize) {
            let revenue = if t.last_year.revenue.0 > 0 {
                t.last_year.revenue
            } else {
                t.budget_total()
            };
            t.debt += revenue.scale(1.5);
        }
    });
    let mut b = g.fork()?;
    let pb = monthly(&mut b, home, 61, |_, _| {});
    let mut x = g.fork()?;
    x.intervene(&policy("policy.fiscal_bridge", home, &[]))?;
    let px = monthly(&mut x, home, 61, |_, _| {});
    out.push((
        1,
        "spread_above_tier_bp_with_hole".into(),
        (pb[1].spread - tier) * 1e4,
    ));
    for m in [12u32, 24, 36, 60] {
        let i = (m - START) as usize;
        let (a, z) = (&pb[i], &px[i]);
        out.push((m, "spread_bp".into(), (z.spread - a.spread) * 1e4));
        out.push((
            m,
            "base_spread_above_tier_bp".into(),
            (a.spread - tier) * 1e4,
        ));
        out.push((m, "debt_pct".into(), pct(a.debt, z.debt)));
        let from = i.saturating_sub(11).max(1);
        let ib = sum_months(&pb, from, i, |p| p.interest);
        let ix = sum_months(&px, from, i, |p| p.interest);
        out.push((m, "interest_pct".into(), pct(ib, ix)));
    }
    Ok(out)
}

/// The share of towns paying exactly their tier's spread after `years` in
/// the untouched worlds, and the highest spread above it.
fn prudent(cx: &Ctx, s: &Option<String>, seed: u64, months: u32) -> Result<(f64, f64), String> {
    let mut g = Game::new(&cx.content, &Setup::small(seed, s.as_deref()), Driver::Idle)?;
    g.run_to(months, &[], &mut Vec::new());
    let b = &cx.content.economy.borrowing;
    let towns = &g.e.state().finance.local;
    let above: Vec<f64> = towns
        .iter()
        .map(|t| mandate_sim::finance::spread(b, t) - b.spreads[0])
        .collect();
    let at_tier = above.iter().filter(|x| **x <= 1e-12).count() as f64 / above.len().max(1) as f64;
    Ok((at_tier, above.iter().copied().fold(0.0, f64::max)))
}

fn table(rows: Vec<Vec<(u32, String, f64)>>) -> Value {
    let mut by: BTreeMap<(u32, String), Vec<f64>> = BTreeMap::new();
    for r in rows {
        for (m, k, v) in r {
            by.entry((m, k)).or_default().push(v);
        }
    }
    let mut o = serde_json::Map::new();
    for ((m, k), v) in by {
        let e = o.entry(format!("m{m:02}")).or_insert_with(|| json!({}));
        e[k] = stat(&v);
    }
    Value::Object(o)
}

pub fn run(cx: &Ctx) -> Result<Value, String> {
    let worlds = cx.worlds();
    type Twin = fn(&Ctx, &Option<String>, u64) -> Result<Vec<(u32, String, f64)>, String>;
    let mut out = serde_json::Map::new();
    let twins: [(&str, Twin); 3] = [
        ("property_tax", property_tax),
        ("rent_cap", rent_cap),
        ("borrowing", borrowing),
    ];
    for (name, f) in twins {
        if !cx.args.only.is_empty() && !cx.args.only.iter().any(|o| name.contains(o.as_str())) {
            continue;
        }
        let res = par_map(&worlds, threads(), |(s, seed)| f(cx, s, *seed));
        let rows: Result<Vec<_>, String> = res.into_iter().collect();
        out.insert(name.into(), table(rows?));
    }
    if cx.args.only.is_empty() || cx.args.only.iter().any(|o| "prudent".contains(o.as_str())) {
        let res = par_map(&worlds, threads(), |(s, seed)| prudent(cx, s, *seed, 120));
        let rows: Result<Vec<_>, String> = res.into_iter().collect();
        let rows = rows?;
        let shares: Vec<f64> = rows.iter().map(|r| r.0).collect();
        let worst = rows.iter().map(|r| r.1).fold(0.0, f64::max);
        out.insert(
            "prudent_towns_at_year_10".into(),
            json!({ "share_at_tier_spread": r3(mean(&shares)), "highest_above_tier_bp": r1(worst * 1e4) }),
        );
    }
    out.insert(
        "params".into(),
        json!({ "seeds_per_scenario": cx.args.seeds, "scenarios": cx.scenarios().iter().map(short_scenario).collect::<Vec<_>>(), "enacted_at_month": START, "cap_lifted_at_month": LIFT }),
    );
    Ok(Value::Object(out))
}
