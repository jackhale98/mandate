#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Town baselines (TOWN_GENERATION §15 E1) and the terrain layers of the town
//! geography view (§4.1): recorded as play begins, never moved by play,
//! identical across runs and rebuilds.

use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::Command;
use mandate_sim::world::{COMPASS_NONE, Cover, MunicipalityId};
use mandate_worldgen::{WorldConfig, WorldSize, generate, rebuild_terrain, terrain_hash};
use std::path::Path;
use std::sync::Arc;

fn content() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

#[test]
fn baselines_are_the_towns_as_play_begins() {
    let c = content();
    let mut e = generate(&WorldConfig::new(71, WorldSize::Small), c.clone()).unwrap();
    let st = e.state();
    for (h, hp) in st.population.hoods.iter().enumerate() {
        assert_eq!(
            u64::from(hp.baseline_population),
            st.population.hood_total(h)
        );
        assert!(hp.baseline_population > 0, "every neighbourhood has people");
    }
    for mu in &st.economy.munis {
        assert_eq!(mu.sectors.len(), st.economy.sector_keys.len());
        for s in &mu.sectors {
            assert_eq!(s.baseline_jobs, s.jobs.round() as u32);
        }
        assert_eq!(mu.housing.baseline_stock, mu.housing.stock.round() as u32);
        assert!(mu.housing.baseline_stock > 0);
    }
    let n = st.world.municipalities.len();
    let towns: Vec<_> = (0..n)
        .map(|m| st.town_baseline(MunicipalityId(m as u32)).unwrap())
        .collect();
    assert!(st.town_baseline(MunicipalityId(n as u32)).is_none());
    for (m, t) in towns.iter().enumerate() {
        let mu = &st.world.municipalities[m];
        assert_eq!(
            t.neighborhoods.iter().map(|h| h.id).collect::<Vec<_>>(),
            mu.neighborhoods
        );
        assert_eq!(
            t.population,
            st.population.municipality_total(&st.world, mu.id)
        );
        let keys: Vec<_> = t.sectors.iter().map(|s| s.sector.clone()).collect();
        assert_eq!(keys, st.economy.sector_keys);
        // Jobs match the labour market as play begins (within rounding).
        let jobs: u64 = t.sectors.iter().map(|s| u64::from(s.jobs)).sum();
        let live = st.economy.munis[m].total_jobs();
        assert!(
            (jobs as f64 - live).abs() <= t.sectors.len() as f64,
            "{jobs} vs {live}"
        );
    }
    // Play moves the towns, never their baselines.
    for _ in 0..24 {
        e.apply(Command::EndTurn).unwrap();
    }
    let st = e.state();
    assert_ne!(
        st.population.hood_total(0),
        u64::from(st.population.hoods[0].baseline_population),
        "the population moved over two years of play"
    );
    for (m, t) in towns.iter().enumerate() {
        assert_eq!(&st.town_baseline(MunicipalityId(m as u32)).unwrap(), t);
    }
}

#[test]
fn baselines_are_deterministic() {
    let c = content();
    let cfg = WorldConfig::new(72, WorldSize::Small);
    let a = generate(&cfg, c.clone()).unwrap();
    let b = generate(&cfg, c).unwrap();
    assert_eq!(a.state_hash(), b.state_hash());
    for m in 0..a.state().world.municipalities.len() {
        let m = MunicipalityId(m as u32);
        assert_eq!(a.state().town_baseline(m), b.state().town_baseline(m));
    }
}

#[test]
fn terrain_layers_for_the_town_view() {
    let c = content();
    let e = generate(&WorldConfig::new(73, WorldSize::Small), c.clone()).unwrap();
    let t = e.terrain().unwrap();
    let n = t.w * t.h;
    assert_eq!(t.downstream.len(), n);
    assert_eq!(t.forest_pm.len(), n);
    assert_eq!(t.minerals_pm.len(), n);
    for i in 0..n {
        let d = t.downstream[i];
        assert!(
            d == COMPASS_NONE || (d < 16 && d.is_multiple_of(2)),
            "cell {i}: {d}"
        );
        assert!(t.forest_pm[i] <= 1000 && t.minerals_pm[i] <= 1000);
        if t.cover[i] == Cover::Sea {
            assert_eq!(d, COMPASS_NONE, "the sea does not flow");
        }
    }
    // Rivers flow along themselves: a river cell drains into a cell that
    // carries at least as much water (a river as big or bigger), a lake, or
    // the sea. A wrong compass encoding would break this almost everywhere.
    let step = |d: u8| -> (i64, i64) {
        match d {
            0 => (0, -1),
            2 => (1, -1),
            4 => (1, 0),
            6 => (1, 1),
            8 => (0, 1),
            10 => (-1, 1),
            12 => (-1, 0),
            _ => (-1, -1),
        }
    };
    let mut river = 0;
    for i in 0..n {
        if t.river[i] == 0 || t.downstream[i] == COMPASS_NONE {
            continue;
        }
        let (dx, dy) = step(t.downstream[i]);
        let (x, y) = ((i % t.w) as i64 + dx, (i / t.w) as i64 + dy);
        assert!(x >= 0 && y >= 0 && (x as usize) < t.w && (y as usize) < t.h);
        let j = y as usize * t.w + x as usize;
        river += 1;
        assert!(
            t.river[j] >= t.river[i] || matches!(t.cover[j], Cover::Lake | Cover::Sea),
            "cell {i} (class {}) drains into {j} (class {}, {:?})",
            t.river[i],
            t.river[j],
            t.cover[j]
        );
    }
    assert!(river > 50, "{river} river cells");
    // Forest and minerals are real layers, not blanks.
    assert!(t.forest_pm.iter().any(|v| *v > 500));
    assert!(t.minerals_pm.iter().any(|v| *v > 0));
    // A rebuild after load reproduces the layers, under the same terrain hash.
    let (again, ok) = rebuild_terrain(e.state(), &c).unwrap();
    assert!(ok);
    assert_eq!(terrain_hash(&again), terrain_hash(t));
    assert_eq!(&again, t);
}
