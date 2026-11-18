#![allow(clippy::unwrap_used, clippy::expect_used)]
//! M2 integration tests: accounting invariants, causal chains, determinism.

use mandate_content::{ContentBundle, SectorKind, bundle::read_dir_sources, load};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::{Arc, OnceLock};

fn content() -> Arc<ContentBundle> {
    static C: OnceLock<Arc<ContentBundle>> = OnceLock::new();
    C.get_or_init(|| {
        let src =
            read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
        Arc::new(load(&src).unwrap().0)
    })
    .clone()
}

fn world(seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), content()).unwrap()
}

fn all_finite(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Number(n) => n.as_f64().is_some_and(f64::is_finite),
        serde_json::Value::Array(a) => a.iter().all(all_finite),
        serde_json::Value::Object(o) => o.values().all(all_finite),
        // serde_json writes NaN/inf as null; nulls in numeric fields are a bug,
        // but Option fields are legitimately null, so nulls are allowed here
        // and NaN is caught by the explicit checks below.
        _ => true,
    }
}

#[test]
fn state_stays_finite_and_within_bounds() {
    let mut e = world(21);
    e.apply(Command::AdvanceDays { days: 365 * 10 }).unwrap();
    let s = e.state();
    assert!(all_finite(&serde_json::to_value(s).unwrap()));
    for m in &s.economy.munis {
        assert!(m.unemployment.is_finite() && (0.0..=0.6).contains(&m.unemployment));
        assert!(
            m.sectors
                .iter()
                .all(|x| x.jobs.is_finite() && x.jobs >= 0.0)
        );
        assert!(m.housing.rent.is_finite() && m.housing.rent > 0.0);
        assert!(m.housing.price.is_finite() && m.housing.price > 0.0);
        assert!(m.attractiveness.is_finite());
    }
    let n = &s.economy.national;
    assert!(n.inflation.is_finite() && n.policy_rate >= 0.0 && n.price_level > 0.0);
    for t in s
        .finance
        .local
        .iter()
        .chain(&s.finance.regional)
        .chain(s.finance.national.iter())
    {
        assert!(!t.cash.is_negative(), "treasury cash negative");
        assert!(!t.debt.is_negative(), "treasury debt negative");
    }
    assert!(
        s.environment
            .regions
            .iter()
            .all(|r| (0.0..=1.0).contains(&r.mineral_stock))
    );
}

#[test]
fn population_accounting_balances_each_year() {
    let mut e = world(22);
    for _ in 0..3 {
        let before = e.state().population.total() as i64;
        e.apply(Command::AdvanceDays { days: 365 }).unwrap();
        let s = e.state();
        let y = s.population.history.last().unwrap();
        let after = s.population.total() as i64;
        // Internal migration conserves people; displacement moves them too.
        assert_eq!(
            after - before,
            y.births as i64 - y.deaths as i64 + y.immigration as i64,
            "year {}",
            y.year
        );
    }
}

/// Brief §7.2: a factory closure raises local unemployment, cuts household
/// income and spending, and pushes people to leave.
#[test]
fn factory_closure_causal_chain() {
    let mut base = world(23);
    let mut shocked = world(23);
    // Pick the municipality with the most manufacturing jobs.
    let m = base
        .state()
        .economy
        .munis
        .iter()
        .enumerate()
        .max_by(|a, b| {
            a.1.sectors[SectorKind::Manufacturing.index()]
                .jobs
                .total_cmp(&b.1.sectors[SectorKind::Manufacturing.index()].jobs)
        })
        .map(|(i, _)| i)
        .unwrap();
    shocked
        .apply(Command::LabSectorShock {
            municipality: m as u32,
            sector: SectorKind::Manufacturing,
            factor: 0.3,
            months: 24,
        })
        .unwrap();
    base.apply(Command::AdvanceDays { days: 365 }).unwrap();
    shocked.apply(Command::AdvanceDays { days: 365 }).unwrap();
    let (a, b) = (
        &base.state().economy.munis[m],
        &shocked.state().economy.munis[m],
    );
    assert!(
        b.unemployment > a.unemployment + 0.01,
        "unemployment {} vs {}",
        b.unemployment,
        a.unemployment
    );
    assert!(
        b.class_income[0] < a.class_income[0],
        "low-class income should fall"
    );
    assert!(b.consumption < a.consumption, "spending should fall");
    assert!(
        b.sectors[SectorKind::Retail.index()].jobs < a.sectors[SectorKind::Retail.index()].jobs,
        "retail follows"
    );
    assert!(b.attractiveness < a.attractiveness);
    let pa = base.state().population.municipality_total(
        &base.state().world,
        mandate_sim::world::MunicipalityId(m as u32),
    );
    let pb = shocked.state().population.municipality_total(
        &shocked.state().world,
        mandate_sim::world::MunicipalityId(m as u32),
    );
    assert!(pb < pa, "people should leave: {pb} vs {pa}");
}

#[test]
fn burn_in_produces_history() {
    let e = world(24);
    let s = e.state();
    assert_eq!(s.calendar.date.calendar().year, 2020);
    assert!(s.economy.national.history.len() >= 12 * 19);
    assert!(s.population.history.len() >= 19);
    assert!(s.events.iter().any(|ev| matches!(
        ev.kind,
        mandate_sim::events::EventKind::NationalBudget { .. }
    )));
}
