#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Neighbourhood concerns (N1): each neighbourhood feels its town's
//! indicators through its own conditions, so districts of one town rank their
//! worries differently; a town's felt levels are its neighbourhoods' weighted
//! by residents; its politics still read the town-wide (measured) levels.

use mandate_content::condition::Worry;
use mandate_content::{ContentBundle, Indicator, bundle::read_dir_sources, load};
use mandate_sim::concerns::{Scope, compute_all, levels_in, ranked, salience_from};
use mandate_sim::population::{AGE_BANDS, CLASSES, cohort_index};
use mandate_sim::scheduler::Reads;
use mandate_sim::world::{MunicipalityId, NeighborhoodKind, PlaceKind};
use mandate_sim::{Command, Engine, State};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn base() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn engine(seed: u64, size: WorldSize, turns: u32) -> Engine {
    let mut e = generate(
        &WorldConfig {
            seed,
            size,
            scenario: None,
        },
        base(),
    )
    .unwrap();
    for _ in 0..turns {
        e.apply(Command::EndTurn).unwrap();
    }
    e
}

/// Recompute every concern from the state as it stands (as the monthly
/// refresh does).
fn recompute(e: &mut Engine) {
    let c = e.content().clone();
    e.setup(|s| {
        let (towns, hoods) = compute_all(&Reads::of_state(s, &c), &s.population);
        s.population.town_concerns = towns;
        s.population.hood_concerns = hoods;
    });
}

fn concern(c: &ContentBundle, id: &str) -> usize {
    c.concern_index(id).unwrap()
}

/// A neighbourhood's worries as felt there, strongest first.
fn felt(st: &State, c: &ContentBundle, h: usize) -> Vec<(usize, f64)> {
    let hood = st.world.neighborhoods[h].id;
    ranked(
        &st.population,
        c,
        &st.world,
        Scope::Neighbourhood(hood),
        Worry::Felt,
    )
}

/// A neighbourhood's own level of a concern, and its rank among its felt
/// concerns (0 = its top worry).
fn level_and_rank(st: &State, c: &ContentBundle, h: usize, i: usize) -> (f64, usize) {
    let m = st.world.neighborhoods[h].municipality.index();
    let level = levels_in(&st.population, c, m, &[h])[i];
    let rank = felt(st, c, h).iter().position(|(j, _)| *j == i).unwrap();
    (level, rank)
}

fn polluting_places(st: &State, h: usize) -> usize {
    st.world
        .places
        .iter()
        .filter(|p| p.neighborhood.index() == h)
        .filter(|p| {
            matches!(
                p.kind,
                PlaceKind::Factory | PlaceKind::Mill | PlaceKind::Mine | PlaceKind::Port
            )
        })
        .count()
}

#[test]
fn an_exposed_district_worries_more_about_the_environment_than_a_sheltered_one() {
    let mut e = engine(4242, WorldSize::Medium, 0);
    let c = e.content().clone();
    let env = concern(&c, "concern.environment");
    // A town with a polluting workplace in one district and a suburb with
    // none in or next to it.
    let st = e.state();
    let (m, exposed, sheltered) = (0..st.world.municipalities.len())
        .find_map(|m| {
            let hoods: Vec<usize> = st.world.hoods_of(MunicipalityId(m as u32)).collect();
            let a = *hoods.iter().find(|&&h| polluting_places(st, h) > 0)?;
            let b = *hoods.iter().find(|&&h| {
                let n = &st.world.neighborhoods[h];
                n.kind == NeighborhoodKind::Suburb
                    && polluting_places(st, h) == 0
                    && n.neighbors
                        .iter()
                        .all(|x| polluting_places(st, x.index()) == 0)
            })?;
            Some((m, a, b))
        })
        .expect("a town with industry and a quiet suburb");
    // A middling regional pollution (no level at its cap); the industrial
    // district on the floodplain, the suburb on high ground.
    e.setup(|s| {
        let r = s.world.municipalities[m].region.index();
        s.environment.regions[r].pollution = 0.6;
        s.environment.exposure[exposed].flood = 900;
        s.environment.exposure[sheltered].flood = 0;
    });
    recompute(&mut e);
    let st = e.state();
    let (le, re) = level_and_rank(st, &c, exposed, env);
    let (ls, rs) = level_and_rank(st, &c, sheltered, env);
    assert!(le > ls + 0.1, "exposed {le} vs sheltered {ls}");
    assert!(re <= rs, "ranked {re} vs {rs}");
    // The town's own pollution is unchanged: the residents' mean of the
    // districts' pollution is the region's.
    let r = Reads::of_state(st, &c);
    assert!((mandate_sim::concerns::indicator(&r, m, Indicator::Pollution) - 0.6).abs() < 1e-9);
}

#[test]
fn a_district_with_more_unemployment_worries_more_about_jobs() {
    let mut e = engine(2026, WorldSize::Medium, 12);
    let c = e.content().clone();
    let jobs = concern(&c, "concern.jobs");
    let st = e.state();
    let m = (0..st.world.municipalities.len())
        .find(|&m| {
            st.world.municipalities[m].neighborhoods.len() >= 3
                && st.economy.munis[m].unemployment > 0.05
        })
        .unwrap();
    let hoods: Vec<usize> = st.world.hoods_of(MunicipalityId(m as u32)).collect();
    let (poor, rich) = (hoods[1], hoods[2]);
    // Every adult of one district in the low income class, of the other in
    // the high one (the town's employment rate is lowest for the first).
    let move_to = |s: &mut State, h: usize, to: usize| {
        let hp = &mut s.population.hoods[h];
        for k in 0..hp.communities() {
            for a in 0..AGE_BANDS {
                let n: u32 = (0..CLASSES)
                    .map(|cl| std::mem::take(&mut hp.cohorts[cohort_index(a, cl, k)].count))
                    .sum();
                hp.cohorts[cohort_index(a, to, k)].count = n;
            }
        }
    };
    e.setup(|s| {
        move_to(s, poor, 0);
        move_to(s, rich, 2);
    });
    recompute(&mut e);
    let st = e.state();
    let rates = st.economy.munis[m].class_employment;
    assert!(rates[0] < rates[2], "{rates:?}");
    let (lp, rp) = level_and_rank(st, &c, poor, jobs);
    let (lr, rr) = level_and_rank(st, &c, rich, jobs);
    assert!(lp > lr + 0.05, "poor {lp} vs rich {lr}");
    assert!(rp <= rr, "ranked {rp} vs {rr}");

    // The estate by the plant: when the town's manufacturing loses a third
    // of its jobs, the district with the factory feels it most.
    let mut e = engine(4242, WorldSize::Medium, 0);
    let st = e.state();
    let manufacturing = c
        .sectors
        .iter()
        .position(|s| s.id == "sector.manufacturing")
        .unwrap();
    let (m, plant) = (0..st.world.municipalities.len())
        .find_map(|m| {
            let mu = &st.world.municipalities[m];
            let h = mu.places.iter().find_map(|p| {
                let p = &st.world.places[p.index()];
                (p.kind == PlaceKind::Factory).then_some(p.neighborhood.index())
            })?;
            (mu.neighborhoods.len() >= 3).then_some((m, h))
        })
        .unwrap();
    recompute(&mut e);
    let before: Vec<f64> = e
        .state()
        .world
        .hoods_of(MunicipalityId(m as u32))
        .map(|h| levels_in(&e.state().population, &c, m, &[h])[jobs])
        .collect();
    e.setup(|s| {
        let sec = &mut s.economy.munis[m].sectors[manufacturing];
        sec.jobs = sec.baseline_jobs as f64 * 0.66;
    });
    recompute(&mut e);
    let st = e.state();
    let rise: Vec<(usize, f64)> = st
        .world
        .hoods_of(MunicipalityId(m as u32))
        .zip(&before)
        .map(|(h, b)| (h, levels_in(&st.population, &c, m, &[h])[jobs] - b))
        .collect();
    let plant_rise = rise.iter().find(|x| x.0 == plant).unwrap().1;
    assert!(
        rise.iter().all(|x| x.0 == plant || x.1 < plant_rise),
        "plant district {plant}: {rise:?}"
    );
}

#[test]
fn a_towns_levels_are_its_districts_weighted_by_residents() {
    for (seed, size, turns) in [(777, WorldSize::Medium, 30), (61, WorldSize::Small, 0)] {
        let mut e = engine(seed, size, turns);
        recompute(&mut e);
        let st = e.state();
        let c = e.content();
        let r = Reads::of_state(st, c);
        let pop = &st.population;
        assert_eq!(pop.hood_concerns.len(), st.world.neighborhoods.len());
        for (m, t) in pop.town_concerns.iter().enumerate() {
            let hoods: Vec<usize> = st.world.hoods_of(MunicipalityId(m as u32)).collect();
            let w: f64 = hoods.iter().map(|&h| pop.hood_total(h) as f64).sum();
            for (i, level) in t.levels.iter().enumerate() {
                let agg: f64 = hoods
                    .iter()
                    .map(|&h| pop.hood_total(h) as f64 * pop.hood_concerns[h].levels[i] as f64)
                    .sum::<f64>()
                    / w;
                assert!(
                    (agg - *level as f64).abs() < 1e-5,
                    "town {m} concern {i}: {agg} vs {level}"
                );
            }
            // The town's view of all its districts is the town's level.
            let all = levels_in(pop, c, m, &hoods);
            for (a, b) in all.iter().zip(&t.levels) {
                assert!((a - *b as f64).abs() < 1e-5);
            }
            // Measured levels (the town's politics) come from the town-wide
            // indicators, and the salience from them.
            let measured: Vec<f64> = c
                .concerns
                .iter()
                .map(|d| {
                    d.level
                        .of(mandate_sim::concerns::indicator(&r, m, d.indicator))
                })
                .collect();
            for (a, b) in measured.iter().zip(&t.measured) {
                assert!((a - *b as f64).abs() < 1e-6);
            }
            assert_eq!(t.salience, salience_from(c, &measured));
            // Ranked by level, the town's stored levels exactly; all of its
            // districts are the town.
            let town = Scope::town(MunicipalityId(m as u32));
            let top = ranked(pop, c, &st.world, town, Worry::Measured);
            assert!(top.windows(2).all(|w| w[0].1 >= w[1].1));
            assert!(top.iter().all(|(i, l)| *l == t.levels[*i] as f64));
            for by in [Worry::Measured, Worry::Felt] {
                assert_eq!(
                    ranked(pop, c, &st.world, Scope::District(&hoods), by),
                    ranked(pop, c, &st.world, town, by)
                );
            }
        }
    }
}

#[test]
fn canvassing_a_districts_own_worry_matches_and_another_districts_does_not() {
    let e = engine(777, WorldSize::Medium, 24);
    let st = e.state();
    let c = e.content();
    let (mut pairs, mut no_bonus) = (0, 0);
    for mu in st.world.municipalities.iter() {
        let top: Vec<(usize, usize)> = mu
            .neighborhoods
            .iter()
            .map(|h| {
                let f = felt(st, c, h.index());
                (h.index(), f[0].0)
            })
            .collect();
        for &(a, ta) in &top {
            for &(_, tb) in top.iter().filter(|x| x.1 != ta) {
                pairs += 1;
                let hood = st.world.neighborhoods[a].id;
                let own =
                    mandate_sim::actions::canvass_match(st, c, hood, Some(&c.concerns[ta].id));
                let other =
                    mandate_sim::actions::canvass_match(st, c, hood, Some(&c.concerns[tb].id));
                assert!(own > 1.0, "own worry {own}");
                assert!(own >= other, "own {own} vs another district's {other}");
                // Another district's worry that is not a worry here (below
                // this district's average) earns no bonus.
                let felt = felt(st, c, a);
                let mean = felt.iter().map(|x| x.1).sum::<f64>() / felt.len() as f64;
                let here = felt.iter().find(|x| x.0 == tb).unwrap().1;
                if here <= mean {
                    assert!(other <= 1.0, "{other}");
                    no_bonus += 1;
                }
            }
        }
    }
    assert!(
        pairs > 20,
        "districts of a town differ in their top worry: {pairs}"
    );
    assert!(
        no_bonus > 0,
        "another district's worry often earns no bonus"
    );
}
