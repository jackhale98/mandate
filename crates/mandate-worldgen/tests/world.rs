#![allow(clippy::unwrap_used, clippy::expect_used)]
//! World invariants (M1-13) and canonical presets (DEC-058).

use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::world::*;
use mandate_worldgen::{Generated, WorldConfig, WorldSize, generate_world, terrain_hash};
use proptest::prelude::*;
use std::collections::{BTreeSet, VecDeque};
use std::path::Path;
use std::sync::OnceLock;

fn content() -> &'static ContentBundle {
    static C: OnceLock<ContentBundle> = OnceLock::new();
    C.get_or_init(|| {
        let src =
            read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
        load(&src).unwrap().0
    })
}

fn make_full(seed: u64, size: WorldSize, scenario: Option<&str>) -> Generated {
    generate_world(
        &WorldConfig {
            seed,
            size,
            scenario: scenario.map(str::to_string),
        },
        content(),
        &mut |_, _| {},
    )
    .unwrap()
}

fn make(seed: u64, size: WorldSize, scenario: Option<&str>) -> (World, TerrainData) {
    let g = make_full(seed, size, scenario);
    (g.world, g.terrain)
}

fn total_pop(seed: u64, size: WorldSize, scenario: Option<&str>) -> u64 {
    make_full(seed, size, scenario).hood_population.iter().sum()
}

fn connected(members: &BTreeSet<usize>, adj: impl Fn(usize) -> Vec<usize>) -> bool {
    let Some(&start) = members.iter().next() else {
        return false;
    };
    let mut seen = BTreeSet::from([start]);
    let mut q = VecDeque::from([start]);
    while let Some(i) = q.pop_front() {
        for j in adj(i) {
            if members.contains(&j) && seen.insert(j) {
                q.push_back(j);
            }
        }
    }
    seen.len() == members.len()
}

fn check_invariants(w: &World, t: &TerrainData) {
    // Every country cell is owned; lakes may be; nothing else is.
    for i in 0..t.cover.len() {
        let owned = t.owner[i] != u32::MAX;
        match t.cover[i] {
            Cover::Country => assert!(owned, "unowned country cell {i}"),
            Cover::Lake => {}
            _ => assert!(!owned, "owned non-country cell {i}"),
        }
    }
    // Municipalities: unique names, neighbourhoods, one region each.
    let names: BTreeSet<&str> = w.municipalities.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names.len(), w.municipalities.len());
    for m in &w.municipalities {
        assert!(
            !m.neighborhoods.is_empty(),
            "{} has no neighbourhoods",
            m.name
        );
        assert!(w.regions[m.region.index()].municipalities.contains(&m.id));
        assert!(m.cells > 0);
    }
    let total: usize = w.regions.iter().map(|r| r.municipalities.len()).sum();
    assert_eq!(total, w.municipalities.len());
    // Regions are contiguous over municipality adjacency.
    for r in &w.regions {
        let members: BTreeSet<usize> = r.municipalities.iter().map(|m| m.index()).collect();
        assert!(
            connected(&members, |i| w.municipalities[i]
                .neighbors
                .iter()
                .map(|n| n.index())
                .collect()),
            "region {} split",
            r.name
        );
        assert!(r.municipalities.contains(&r.capital));
    }
    // Every neighbourhood belongs to exactly one municipality.
    let listed: usize = w.municipalities.iter().map(|m| m.neighborhoods.len()).sum();
    assert_eq!(listed, w.neighborhoods.len());
    // Constituencies partition their units exactly once per tier/jurisdiction, contiguously.
    for tier in [Tier::Local, Tier::Regional, Tier::National] {
        let mut seen: BTreeSet<(u32, usize)> = BTreeSet::new();
        for c in w.constituencies.iter().filter(|c| c.tier == tier) {
            assert!(!c.neighborhoods.is_empty());
            let members: BTreeSet<usize> = c.neighborhoods.iter().map(|n| n.index()).collect();
            assert!(
                connected(&members, |i| w.neighborhoods[i]
                    .neighbors
                    .iter()
                    .map(|n| n.index())
                    .collect()),
                "{tier:?} district {} not contiguous",
                c.number
            );
            for h in &c.neighborhoods {
                assert!(
                    seen.insert((c.jurisdiction, h.index())),
                    "neighbourhood in two {tier:?} districts"
                );
            }
        }
        if tier == Tier::National {
            assert_eq!(seen.len(), w.neighborhoods.len());
        }
    }
    // Links and places reference valid entities.
    for l in &w.links {
        assert!(l.a.index() < w.municipalities.len() && l.b.index() < w.municipalities.len());
        assert!(l.length_km > 0.0 && !l.path.is_empty());
    }
    for p in &w.places {
        assert_eq!(
            w.neighborhoods[p.neighborhood.index()].municipality,
            p.municipality
        );
    }
    let capital = w.capital.unwrap();
    assert!(
        w.municipalities[capital.index()]
            .places
            .iter()
            .any(|p| w.places[p.index()].kind == PlaceKind::Parliament)
    );
    // The road network connects every municipality.
    let roads: Vec<&TransportLink> = w
        .links
        .iter()
        .filter(|l| l.mode != LinkMode::Rail)
        .collect();
    let all: BTreeSet<usize> = (0..w.municipalities.len()).collect();
    assert!(connected(&all, |i| roads
        .iter()
        .filter_map(|l| if l.a.index() == i {
            Some(l.b.index())
        } else if l.b.index() == i {
            Some(l.a.index())
        } else {
            None
        })
        .collect()));
}

#[test]
fn generation_is_deterministic() {
    let (a, ta) = make(2026, WorldSize::Medium, None);
    let (b, tb) = make(2026, WorldSize::Medium, None);
    assert_eq!(terrain_hash(&ta), terrain_hash(&tb));
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
    let (c, _) = make(2027, WorldSize::Medium, None);
    assert_ne!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&c).unwrap()
    );
}

#[test]
fn medium_world_meets_scale_targets() {
    let (w, t) = make(7, WorldSize::Medium, None);
    check_invariants(&w, &t);
    assert!((8..=12).contains(&w.regions.len()));
    assert!(w.municipalities.len() >= 45);
    assert!(
        (300..=700).contains(&w.neighborhoods.len()),
        "{} neighbourhoods",
        w.neighborhoods.len()
    );
    assert!(
        w.municipalities
            .iter()
            .any(|m| m.class == SettlementClass::Rural)
    );
}

#[test]
fn canonical_presets_bias_geography() {
    let mean_precip = |w: &World| {
        w.municipalities
            .iter()
            .map(|m| m.geo.precipitation_mm)
            .sum::<f64>()
            / w.municipalities.len() as f64
    };
    let coastal = |w: &World| w.municipalities.iter().filter(|m| m.coastal).count();
    let minerals = |w: &World| w.municipalities.iter().map(|m| m.geo.minerals).sum::<f64>();
    let mut drier = 0;
    let mut more_coast = 0;
    let mut richer = 0;
    for seed in 1..=4u64 {
        let (base, _) = make(seed, WorldSize::Small, None);
        let (dry, t) = make(seed, WorldSize::Small, Some("scenario.drought_farmland"));
        check_invariants(&dry, &t);
        let (coast, _) = make(seed, WorldSize::Small, Some("scenario.wealthy_coast"));
        let (front, _) = make(seed, WorldSize::Small, Some("scenario.resource_frontier"));
        drier += (mean_precip(&dry) < mean_precip(&base)) as u32;
        more_coast += (coastal(&coast) >= coastal(&base)) as u32;
        richer += (minerals(&front) > minerals(&base)) as u32;
        assert!(
            total_pop(seed, WorldSize::Small, Some("scenario.resource_frontier"))
                < total_pop(seed, WorldSize::Small, None)
        );
        let _ = (&front, &base);
    }
    assert!(
        drier >= 3 && more_coast >= 3 && richer >= 3,
        "drier {drier} coast {more_coast} richer {richer}"
    );
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 6, ..ProptestConfig::default() })]
    #[test]
    fn invariants_hold_for_any_seed(seed in any::<u64>()) {
        let (w, t) = make(seed, WorldSize::Small, None);
        check_invariants(&w, &t);
    }
}
