#![allow(clippy::unwrap_used, clippy::expect_used)]
//! DEC-075: communities are a content registry and a cohort dimension. An
//! expansion adds a community (here a First Nation concentrated in rural
//! neighbourhoods, with its own views and lower turnout) purely in content.

use mandate_content::{ContentBundle, Source, bundle::read_dir_sources, load};
use mandate_sim::Command;
use mandate_sim::population::{
    AGE_BANDS, CLASSES, GROUPS, PopulationState, age_of, class_of, cohort_index, community_of,
    group_index,
};
use mandate_sim::world::NeighborhoodKind;
use mandate_worldgen::{WorldConfig, WorldSize, generate, generate_world};
use std::path::Path;
use std::sync::Arc;

fn base_sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn base() -> Arc<ContentBundle> {
    Arc::new(load(&base_sources()).unwrap().0)
}

fn content_with_first_nation() -> Arc<ContentBundle> {
    let mut src = base_sources();
    src.push(Source {
        path: "nation/communities.jsonc".into(),
        text: r#"{
          // An Indigenous nation: mostly rural, attached to land and place,
          // and less likely to vote in settler elections.
          "kind": "communities",
          "items": [
            { "id": "community.first_nation", "name_key": "community-first_nation",
              "share": { "base": 0.0, "by_neighborhood": { "rural": 0.35 }, "regional_sd": 0.03 },
              "attitude_offsets": { "localism": 0.3, "environment": 0.25 },
              "turnout_offset": -0.1 },
          ],
        }"#
        .into(),
    });
    src.push(Source {
        path: "nation/locale/en/nation.ftl".into(),
        text: "community-first_nation = First Nation\n".into(),
    });
    Arc::new(load(&src).unwrap().0)
}

const SEED: u64 = 71;

fn adults_where(pop: &PopulationState, hoods: &[usize], community: usize) -> u64 {
    hoods
        .iter()
        .map(|&h| pop.hoods[h].community_adults(community))
        .sum()
}

#[test]
fn cohort_helpers_invert_the_index() {
    for k in 0..3 {
        for a in 0..AGE_BANDS {
            for c in 0..CLASSES {
                let ci = cohort_index(a, c, k);
                assert_eq!((age_of(ci), class_of(ci), community_of(ci)), (a, c, k));
                assert_eq!(ci % GROUPS, group_index(a, c));
            }
        }
    }
    // One community is the plain age × class grid.
    assert_eq!(cohort_index(2, 1, 0), group_index(2, 1));
}

#[test]
fn a_first_nation_splits_cohorts_exactly_and_concentrates_in_rural_areas() {
    let c = content_with_first_nation();
    assert_eq!(c.communities.len(), 2);
    assert_eq!(
        c.remainder_community(),
        c.community_index("community.general").unwrap()
    );
    let nation = c.community_index("community.first_nation").unwrap();
    let general = c.remainder_community();

    // Starting population straight from the same geography, with and
    // without the expansion: every neighbourhood and every age × class group
    // holds exactly the same people, now split across two communities.
    let cfg = WorldConfig::new(SEED, WorldSize::Small);
    let g = generate_world(&cfg, &c, &mut |_, _| {}).unwrap();
    let b = base();
    let one = mandate_sim::population::init(
        &g.world,
        &g.hood_population,
        SEED,
        &Default::default(),
        &b.attitudes,
        &b.communities,
    );
    let two = mandate_sim::population::init(
        &g.world,
        &g.hood_population,
        SEED,
        &Default::default(),
        &c.attitudes,
        &c.communities,
    );
    assert_eq!(two.community_keys.len(), 2);
    for (h, (p1, p2)) in one.hoods.iter().zip(&two.hoods).enumerate() {
        assert_eq!(p1.cohorts.len(), GROUPS);
        assert_eq!(p2.cohorts.len(), 2 * GROUPS);
        assert_eq!(p2.total(), g.hood_population[h], "hood {h}");
        assert_eq!(p1.total(), p2.total(), "hood {h}");
        assert_eq!(p1.children, p2.children);
        assert_eq!(p2.children, p2.children_by_community.iter().sum::<u32>());
        for gi in 0..GROUPS {
            let split = p2.cohorts[gi].count + p2.cohorts[GROUPS + gi].count;
            assert_eq!(p1.cohorts[gi].count, split, "hood {h} group {gi}");
        }
    }

    let kinds = |pred: &dyn Fn(NeighborhoodKind) -> bool| -> Vec<usize> {
        g.world
            .neighborhoods
            .iter()
            .enumerate()
            .filter(|(_, n)| pred(n.kind))
            .map(|(i, _)| i)
            .collect()
    };
    let rural = kinds(&|k| k == NeighborhoodKind::Rural);
    let other = kinds(&|k| k != NeighborhoodKind::Rural);
    assert!(!rural.is_empty() && !other.is_empty());
    let share = |hoods: &[usize]| {
        let all: u64 = hoods.iter().map(|&h| two.hoods[h].adults()).sum();
        adults_where(&two, hoods, nation) as f64 / all.max(1) as f64
    };
    let (sr, so) = (share(&rural), share(&other));
    assert!(sr > 0.25 && sr < 0.45, "rural share {sr}");
    assert!(so < sr / 3.0, "elsewhere {so} vs rural {sr}");

    // The nation's cohorts differ from their general neighbours by the
    // offsets (same place, age, and class; only noise differs).
    let diff = |key: &str| {
        let k = c.attitude_index(key).unwrap();
        let (mut acc, mut w) = (0.0, 0.0);
        for &h in &rural {
            for gi in 0..GROUPS {
                let a = &two.hoods[h].cohorts[nation * GROUPS + gi];
                let b = &two.hoods[h].cohorts[general * GROUPS + gi];
                acc += (a.attitudes[k] - b.attitudes[k]) as f64;
                w += 1.0;
            }
        }
        acc / w
    };
    let (loc, env, red) = (
        diff("localism"),
        diff("environment"),
        diff("redistribution"),
    );
    assert!((loc - 0.3).abs() < 0.08, "localism offset {loc}");
    assert!((env - 0.25).abs() < 0.08, "environment offset {env}");
    assert!(red.abs() < 0.05, "no offset on redistribution: {red}");
    let turnout = |k: usize| {
        let (mut acc, mut w) = (0.0, 0.0);
        for &h in &rural {
            for c in &two.hoods[h].cohorts[k * GROUPS..(k + 1) * GROUPS] {
                acc += c.turnout as f64 * c.count as f64;
                w += c.count as f64;
            }
        }
        acc / w.max(1.0)
    };
    assert!(
        turnout(nation) < turnout(general) - 0.05,
        "lower turnout: {} vs {}",
        turnout(nation),
        turnout(general)
    );

    // Base content is unchanged by the expansion's existence.
    assert_eq!(b.communities.len(), 1);
    assert!(one.hoods.iter().all(|h| h.children_by_community.is_empty()));
}

#[test]
fn a_world_with_a_first_nation_simulates_deterministically() {
    let c = content_with_first_nation();
    let nation = c.community_index("community.first_nation").unwrap();
    let run = || {
        let mut e = generate(&WorldConfig::new(SEED, WorldSize::Small), c.clone()).unwrap();
        // Burn-in already held elections with the new cohorts voting.
        assert!(!e.state().politics.elections.results.is_empty());
        for _ in 0..2 {
            let before = e.state().population.total() as i64;
            e.apply(Command::AdvanceDays { days: 365 }).unwrap();
            let s = e.state();
            let y = s.population.history.last().unwrap();
            // People are conserved exactly across communities.
            assert_eq!(
                s.population.total() as i64 - before,
                y.births as i64 - y.deaths as i64 + y.immigration as i64
            );
            for hp in &s.population.hoods {
                assert_eq!(hp.cohorts.len(), 2 * GROUPS);
                assert_eq!(hp.children, hp.children_by_community.iter().sum::<u32>());
            }
        }
        e
    };
    let a = run();
    let b = run();
    assert_eq!(a.state_hash(), b.state_hash(), "same seed, same state");

    let s = a.state();
    let people: u64 = s
        .population
        .hoods
        .iter()
        .map(|h| h.community_adults(nation) + h.children_of(nation) as u64)
        .sum();
    assert!(people > 0, "the nation survives the burn-in");
    // Elections still produce results and every race has a winner.
    let res = &s.politics.elections.results;
    assert!(!res.is_empty());
    for r in res {
        assert!(r.candidates.iter().any(|c| c.politician == r.winner));
        assert!(r.turnout > 0.0 && r.turnout <= 1.0);
    }
    // Some towns hold both communities.
    assert!((0..s.world.municipalities.len()).any(|m| {
        s.population
            .municipality_communities(&s.world, mandate_sim::world::MunicipalityId(m as u32))
            .iter()
            .all(|n| *n > 0)
    }));
}

#[test]
fn bad_communities_are_errors() {
    let mut src = base_sources();
    src.push(Source {
        path: "bad/communities.jsonc".into(),
        text: r#"{"kind":"communities","items":[
            {"id":"community.second_majority","name_key":"community-general"},
            {"id":"community.lost","name_key":"community-lost","share":{"base":1.5},"attitude_offsets":{"whaling":0.5}}
        ]}"#
        .into(),
    });
    let errs = load(&src).unwrap_err();
    let msg = errs
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        msg.contains("exactly one community must omit `share`"),
        "{msg}"
    );
    assert!(msg.contains("share.base must be in [0, 1]"), "{msg}");
    assert!(msg.contains("unknown attitude `whaling`"), "{msg}");
    assert!(msg.contains("community-lost"), "{msg}");
}
