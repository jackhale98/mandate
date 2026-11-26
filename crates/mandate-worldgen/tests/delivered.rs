#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Audit fixes 3, 5 and 8 (SIMULATION_AUDIT "After fix 3"): approval credits
//! what laws deliver, the services worry reads service levels, the housing
//! ledger splits started from added, and the pollution inspection does
//! something.

use mandate_content::{ContentBundle, Indicator, bundle::read_dir_sources, load};
use mandate_sim::policy::{PolicyStatus, probe};
use mandate_sim::scheduler::Reads;
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

fn home(e: &Engine) -> u32 {
    mandate_sim::player::home(e.state()).unwrap().0
}

/// Enact a template at its defaults in the player's town, as a passed law.
fn enact(e: &mut Engine, template: &str) -> usize {
    let c = content();
    let t = c.policies.get(template).unwrap().clone();
    let m = home(e);
    let id = e.state().policy.instances.len();
    e.setup(|st| {
        let mut p = probe(&t, m, &Default::default());
        p.id = st.policy.ids.alloc();
        p.status = PolicyStatus::Enacted;
        p.introduced_on = Some(st.calendar.date);
        p.enacted_on = Some(st.calendar.date);
        st.policy.instances.push(p);
    });
    id
}

fn months(e: &mut Engine, n: u32) {
    e.apply(Command::AdvanceDays { days: 30 * n }).unwrap();
}

fn shortfall(e: &Engine) -> f64 {
    let c = content();
    let r = Reads::of_state(e.state(), &c);
    mandate_sim::concerns::indicator(&r, home(e) as usize, Indicator::ServiceShortfall)
}

/// The services worry reads services delivered against need: a cut to the
/// town's budget raises it, a services law in effect lowers it.
#[test]
fn the_services_worry_reads_service_levels() {
    let mut e = world(11);
    let m = home(&e) as usize;
    let before = shortfall(&e);
    let saved = e.state().finance.local[m].budget.clone();
    e.setup(|st| {
        for b in st.finance.local[m].budget.iter_mut() {
            *b = b.scale(0.8);
        }
    });
    let cut = shortfall(&e);
    assert!(cut > before + 0.15, "{before} -> {cut}");
    e.setup(|st| st.finance.local[m].budget = saved);
    assert_eq!(shortfall(&e), before);
    let id = enact(&mut e, "policy.transit_frequency");
    months(&mut e, 14);
    let st = e.state();
    assert_eq!(st.policy.instances[id].status, PolicyStatus::InEffect);
    let c = content();
    let r = Reads::of_state(st, &c);
    let mods =
        mandate_sim::policy::mods_for(&st.policy, &c, m as u32, &|i| st.finance.funded_share(i));
    // Schools and health care read their own lines and laws (DEC-106).
    let care = [
        mandate_content::BudgetCategory::Education,
        mandate_content::BudgetCategory::Health,
    ]
    .map(mandate_sim::policy::category_index);
    let gains: f64 = mods
        .services
        .iter()
        .enumerate()
        .filter(|(i, _)| !care.contains(i))
        .map(|(_, g)| g)
        .sum();
    let budget = mandate_sim::concerns::services_index(&r, Some(&st.population), m);
    assert!(gains > 0.05, "gains {gains}");
    assert!((shortfall(&e) - (1.0 - budget - gains)).abs() < 1e-9);
}

/// Credit for delivered results is zero without laws and reaches the class
/// a transfer pays; the hardship fund's credit is felt by the low class only.
#[test]
fn delivered_credit_reaches_those_paid() {
    let mut e = world(12);
    let m = home(&e);
    // Strip the town's laws so the credit is this law's alone.
    e.setup(|st| {
        for p in st.policy.instances.iter_mut() {
            if p.jurisdiction == m {
                p.status = PolicyStatus::Ended;
            }
        }
    });
    let c = content();
    let credit = |e: &Engine| {
        let st = e.state();
        let r = Reads::of_state(st, &c);
        mandate_sim::population::delivered_credit(
            &r,
            &st.policy,
            &st.finance,
            &st.economy.munis[m as usize],
            m,
        )
        .0
    };
    assert_eq!(credit(&e), mandate_sim::population::Credit::default());
    enact(&mut e, "policy.hardship_fund");
    months(&mut e, 6);
    let k = credit(&e);
    assert!(k.by_class[0] > 0.01, "{k:?}");
    assert_eq!(k.by_class[1], 0.0);
    assert_eq!(k.by_class[2], 0.0);
    assert_eq!(k.jobs, 0.0);
    // A senior of the low class feels the payment; no one feels jobs.
    assert_eq!(k.felt(3, 0), k.by_class[0]);
}

/// The housing ledger counts homes started exactly and estimates what they
/// added: completions over the construction lag, net of displaced private
/// building and of demolition, so added stays below started.
#[test]
fn the_housing_ledger_splits_started_from_added() {
    let mut e = world(13);
    let id = enact(&mut e, "policy.affordable_housing");
    months(&mut e, 36);
    let l = &e.state().policy.instances[id].ledger;
    let started = l.get("homes_started").copied().unwrap_or(0.0);
    let building = l.get("homes_building").copied().unwrap_or(0.0);
    let added = l.get("homes_added").copied().unwrap_or(0.0);
    let net = content().economy.attribution.homes_net;
    assert!(started > 200.0, "{l:?}");
    assert!(
        added > 0.0 && added < (started - building) * net + 1e-6,
        "{l:?}"
    );
    assert!(!l.contains_key("homes_built"));
}

/// The inspection: the region's pollution falls at once by the content's
/// share × the town's share of the region's polluting jobs, and the town's
/// largest polluting sector loses output for the months asked, then not.
#[test]
fn an_inspection_cuts_pollution_and_output_for_a_while() {
    let c = content();
    for seed in 1..=20u64 {
        let mut e = world(seed);
        let m = home(&e) as usize;
        let region = e.state().world.municipalities[m].region.index();
        let before = e.state().environment.regions[region].pollution;
        let shocks = e.state().economy.munis[m].shocks.len();
        let mut sector = None;
        e.setup(|st| {
            sector = mandate_sim::narrative::resolve::inspect(
                st,
                &c,
                0.15,
                0.08,
                12,
                mandate_sim::spine::Cause::Lab,
            );
        });
        let Some(_) = sector else { continue };
        let st = e.state();
        let after = st.environment.regions[region].pollution;
        assert!(
            after < before && after >= before * 0.85,
            "{before} -> {after}"
        );
        let mu = &st.economy.munis[m];
        assert_eq!(mu.shocks.len(), shocks + 1);
        let s = mu.shocks.last().unwrap().clone();
        assert!((s.factor - 0.92).abs() < 1e-9 && s.months_left == 12);
        // Not a disaster: the town is not marked as struck.
        assert_ne!(mu.months_since_disaster, 0);
        months(&mut e, 14);
        assert!(
            e.state().economy.munis[m]
                .shocks
                .iter()
                .all(|x| x.sector != s.sector || x.factor != s.factor)
        );
        return;
    }
    panic!("no home town with polluting jobs in seeds 1–20");
}
