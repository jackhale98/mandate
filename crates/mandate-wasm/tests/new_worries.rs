//! The first batch of new worries (DEC-106 item 1, DEC-095: every worry
//! driven by a simulated signal). One twin per worry: the same world, once
//! as it is and once with the worry's signal moved; the worry moves the
//! right way in the neighbourhoods (and income classes) the signal touches,
//! and a twin with the signal still leaves it where it was. Then the split
//! of schools and health care from services (a cut to one moves only its
//! own worry), and determinism through a save.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]
use mandate_content::{BudgetCategory, ContentBundle, Gravity};
use mandate_sim::media::Scandal;
use mandate_sim::politics::PoliticianId;
use mandate_sim::world::{MunicipalityId, NeighborhoodId};
use mandate_sim::{Command, Engine, State};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::Arc;

const SEEDS: [u64; 2] = [2026, 7];

fn base() -> Arc<ContentBundle> {
    Arc::new(
        mandate_content::load(&mandate_content::embedded::sources())
            .unwrap()
            .0,
    )
}

fn game(seed: u64, c: Arc<ContentBundle>) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c).unwrap()
}

fn copy(e: &Engine) -> Engine {
    Engine::from_state(e.state().clone(), e.content_arc()).unwrap()
}

fn home(e: &Engine) -> MunicipalityId {
    mandate_sim::player::home(e.state()).unwrap()
}

fn hoods(e: &Engine) -> Vec<NeighborhoodId> {
    e.state().world.municipalities[home(e).index()]
        .neighborhoods
        .clone()
}

fn line(c: BudgetCategory) -> usize {
    mandate_sim::policy::category_index(c)
}

/// A worry's level in each of the home town's neighbourhoods, as felt.
fn levels(e: &Engine, id: &str) -> Vec<f64> {
    let st = e.state();
    let i = st
        .population
        .concern_keys
        .iter()
        .position(|k| k == id)
        .unwrap_or_else(|| panic!("no worry {id}"));
    hoods(e)
        .iter()
        .map(|h| f64::from(st.population.hood_concerns[h.index()].levels[i]))
        .collect()
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

/// Idle turns until `days` have passed.
fn idle(e: &mut Engine, days: u32) {
    let until = e.state().calendar.date.add_days(days);
    while e.state().calendar.date < until {
        e.apply(Command::EndTurn).unwrap();
    }
}

/// One twin per seed: `still` is the world as it is, `moved` has `edit`
/// applied; both run `days`. Returns, per seed, the worry's levels in the
/// home town's neighbourhoods (still, moved).
fn twin(
    id: &str,
    days: u32,
    edit: &dyn Fn(&mut State, MunicipalityId),
) -> Vec<(Vec<f64>, Vec<f64>)> {
    let c = base();
    SEEDS
        .iter()
        .map(|&seed| {
            let mut still = game(seed, c.clone());
            // A year on, so the year-on-year signals have a history.
            idle(&mut still, 365);
            let mut moved = copy(&still);
            let mut again = copy(&still);
            let m = home(&still);
            moved.setup(|st| edit(st, m));
            for e in [&mut still, &mut moved, &mut again] {
                idle(e, days);
            }
            let (a, b) = (levels(&still, id), levels(&moved, id));
            // The signal still: the same world gives the same worry.
            assert_eq!(levels(&again, id), a, "{id}: a still twin moved");
            println!(
                "{id} seed {seed}: still {:.3} moved {:.3} (by neighbourhood {:?} -> {:?})",
                mean(&a),
                mean(&b),
                a.iter()
                    .map(|x| (x * 1000.0).round() / 1000.0)
                    .collect::<Vec<_>>(),
                b.iter()
                    .map(|x| (x * 1000.0).round() / 1000.0)
                    .collect::<Vec<_>>(),
            );
            (a, b)
        })
        .collect()
}

/// The worry rose (on the town's mean, and nowhere fell) in every twin.
fn rose(id: &str, runs: &[(Vec<f64>, Vec<f64>)]) {
    for (a, b) in runs {
        assert!(mean(b) > mean(a) + 1e-3, "{id}: {a:?} -> {b:?}");
        for (x, y) in a.iter().zip(b) {
            assert!(y + 1e-6 >= *x, "{id}: a neighbourhood fell {x} -> {y}");
        }
    }
}

#[test]
fn the_batch_is_in_the_registry_with_names() {
    let c = base();
    for id in [
        "concern.cost_of_living",
        "concern.newcomers",
        "concern.young_leaving",
        "concern.schools_health",
        "concern.upkeep",
        "concern.trust",
        "concern.development",
    ] {
        let k = c.concern_index(id).unwrap_or_else(|| panic!("{id}"));
        assert!(c.concerns[k].talk_key.is_some(), "{id}");
    }
}

/// Cost of living: prices jump 3% (wages follow at their own pace). The
/// worry rises everywhere, and most where more residents are on low
/// incomes (compared where neither is at the top of the range).
#[test]
fn prices_against_wages_raise_the_cost_of_living_worry() {
    let id = "concern.cost_of_living";
    let runs = twin(id, 35, &|st, _| st.economy.national.price_level *= 1.03);
    rose(id, &runs);
    // Classes: the low-income-heaviest neighbourhood rises at least as much
    // as the high-income-heaviest (`signals.cost_of_living_class`).
    let c = base();
    for &seed in &SEEDS {
        let e = game(seed, c.clone());
        let st = e.state();
        let low_share = |h: &NeighborhoodId| {
            let hp = &st.population.hoods[h.index()];
            let (mut low, mut all) = (0.0, 0.0);
            for (ci, x) in hp.cohorts.iter().enumerate() {
                all += x.count as f64;
                if mandate_sim::population::class_of(ci) == 0 {
                    low += x.count as f64;
                }
            }
            low / all.max(1.0)
        };
        let hs = hoods(&e);
        let (lo, hi) = hs.iter().enumerate().fold((0, 0), |(lo, hi), (i, h)| {
            (
                if low_share(h) > low_share(&hs[lo]) {
                    i
                } else {
                    lo
                },
                if low_share(h) < low_share(&hs[hi]) {
                    i
                } else {
                    hi
                },
            )
        });
        let (a, b) = &runs[SEEDS.iter().position(|s| *s == seed).unwrap()];
        if lo != hi && b[lo] < 0.999 && b[hi] < 0.999 {
            assert!(
                b[lo] - a[lo] + 1e-6 >= b[hi] - a[hi],
                "seed {seed}: low-income {} vs high-income {}",
                b[lo] - a[lo],
                b[hi] - a[hi]
            );
        }
    }
}

/// Newcomers: the home town has a quarter more homes (more room and lower
/// rents draw movers). More people move in and the worry rises.
#[test]
fn more_arrivals_raise_the_newcomers_worry() {
    let id = "concern.newcomers";
    let runs = twin(id, 90, &|st, m| {
        st.economy.munis[m.index()].housing.stock *= 1.25;
    });
    rose(id, &runs);
}

/// Young people leaving: the home town loses most of its biggest sector's
/// demand for a year; jobs go, the young move away, and the worry rises.
#[test]
fn a_slump_sends_the_young_away_and_raises_the_worry() {
    let id = "concern.young_leaving";
    let runs = twin(id, 120, &|st, m| {
        let mu = &st.economy.munis[m.index()];
        let k = (0..mu.sectors.len())
            .max_by(|a, b| mu.sectors[*a].jobs.total_cmp(&mu.sectors[*b].jobs))
            .unwrap();
        let sector = st.economy.sector_keys[k].clone();
        mandate_sim::economy::add_shock(&mut st.economy, m.0, &sector, 0.3, 12);
    });
    rose(id, &runs);
}

/// Schools and health care: the town's Education line halved. The worry
/// rises, most where more residents are children or old.
#[test]
fn a_schools_cut_raises_the_schools_and_health_worry() {
    let id = "concern.schools_health";
    let runs = twin(id, 35, &|st, m| {
        let b = &mut st.finance.local[m.index()].budget[line(BudgetCategory::Education)];
        *b = b.scale(0.5);
    });
    rose(id, &runs);
}

/// Roads and upkeep: the Infrastructure line halved (B2's twin reads the
/// worry with faults over two years: `faults.rs`).
#[test]
fn an_infrastructure_cut_raises_the_upkeep_worry() {
    let id = "concern.upkeep";
    let runs = twin(id, 35, &|st, m| {
        let b = &mut st.finance.local[m.index()].budget[line(BudgetCategory::Infrastructure)];
        *b = b.scale(0.5);
    });
    rose(id, &runs);
}

/// Trust and corruption: a misconduct scandal about one of the town's
/// politicians breaks. The worry rises in every neighbourhood of the town.
#[test]
fn a_scandal_raises_the_trust_worry() {
    let id = "concern.trust";
    let runs = twin(id, 35, &|st, m| {
        let date = st.calendar.date;
        let id = st.media.scandals.iter().map(|s| s.id).max().unwrap_or(0) + 1000;
        st.media.scandals.push(Scandal {
            id,
            hidden: 0,
            subject: PoliticianId(0),
            kind: "kickback".into(),
            severity: Gravity::Misconduct,
            evidence: 0.8,
            town: m.0,
            broke: date,
            by: mandate_content::Discoverer::Journalist,
            who: None,
            outlet: None,
            heat: 0.0,
            stories: 1,
            response: None,
            responded: None,
            backfire_on: None,
            // Over at once: only the worry reads it.
            ended: Some(date),
            leaked: false,
        });
    });
    rose(id, &runs);
}

/// Development and character: three times the homes under construction.
/// The worry rises.
#[test]
fn a_building_boom_raises_the_development_worry() {
    let id = "concern.development";
    let runs = twin(id, 35, &|st, m| {
        for x in &mut st.economy.munis[m.index()].housing.pipeline {
            *x *= 3.0;
        }
    });
    rose(id, &runs);
}

/// The split (DEC-106): a cut to the schools moves schools and health
/// care, not services; a cut to the other lines moves services, not
/// schools and health care. Read at the next monthly refresh.
#[test]
fn schools_and_health_are_split_from_services() {
    let cut = |cats: &'static [BudgetCategory]| {
        move |st: &mut State, m: MunicipalityId| {
            for c in cats {
                let b = &mut st.finance.local[m.index()].budget[line(*c)];
                *b = b.scale(0.5);
            }
        }
    };
    let close = |a: &[f64], b: &[f64]| (mean(a) - mean(b)).abs() < 0.01;
    let schools = cut(&[BudgetCategory::Education]);
    let others = cut(&[
        BudgetCategory::Administration,
        BudgetCategory::Safety,
        BudgetCategory::Social,
    ]);
    for (a, b) in twin("concern.services", 35, &schools) {
        assert!(close(&a, &b), "a schools cut moved services: {a:?} {b:?}");
    }
    for (a, b) in twin("concern.schools_health", 35, &others) {
        assert!(close(&a, &b), "a services cut moved schools: {a:?} {b:?}");
    }
    rose("concern.services", &twin("concern.services", 35, &others));
}

/// Through a save: the moves kept for the worries (`flows`) and the real
/// wage kept by the economy come back, and the games go on the same.
#[test]
fn the_new_signals_survive_a_save() {
    let mut e = game(2026, base());
    idle(&mut e, 400);
    assert!(
        e.state().population.flows.iter().any(|f| f.arrived > 0.0),
        "moves are kept"
    );
    let req = mandate_save::SaveRequest {
        run_id: "00112233445566778899aabbccddeeff".into(),
        revision: 1,
        parent_revision: None,
        device_id: "test".into(),
        saved_at: "2026-01-01T00:00:00Z".into(),
        protocol_version: 1,
        run_mode: mandate_save::RunMode::Ironman,
    };
    let (bytes, _) = mandate_save::encode(e.state(), &req).unwrap();
    let (st, _) = mandate_save::decode(&bytes).unwrap();
    let mut back = Engine::from_state(st, e.content_arc()).unwrap();
    assert_eq!(back.state().population.flows, e.state().population.flows);
    assert_eq!(back.state_hash(), e.state_hash());
    idle(&mut e, 120);
    idle(&mut back, 120);
    assert_eq!(back.state_hash(), e.state_hash());
}
