//! Industries and named plants (I1, DEC-110): an older save on a real file.
//! `v27_town.mndt` is seed 2026, a small world, sixty idle turns, written
//! by the branch head 12bfe17 (the build before industries). The game is
//! not released, so the step is the cheap one: an older save gets its
//! industries, markets and named plants from the routine a new world uses
//! (the world's seed and each town's sector employment as it stands), its
//! sector totals stay what they were, and it plays on.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{bundle::read_dir_sources, load};
use mandate_save::*;
use mandate_sim::world::PlaceKind;
use mandate_sim::{Command, Engine};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

fn content() -> Arc<mandate_content::ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

/// A save file's format version and its payload as JSON, before migration.
fn raw(bytes: &[u8]) -> (u16, Value) {
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    let meta_len = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
    let payload = miniz_oxide::inflate::decompress_to_vec(&bytes[10 + meta_len..]).unwrap();
    (version, serde_json::from_slice(&payload).unwrap())
}

fn req() -> SaveRequest {
    SaveRequest {
        run_id: "00112233445566778899aabbccddeeff".into(),
        revision: 1,
        parent_revision: None,
        device_id: "test-device".into(),
        saved_at: "2026-01-01T00:00:00Z".into(),
        protocol_version: 1,
        run_mode: RunMode::Story,
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

#[test]
fn a_v27_save_gains_industries_and_plants_at_its_works_and_plays_on() {
    let c = content();
    let bytes = fixture("v27_town.mndt");
    let (version, old) = raw(&bytes);
    assert_eq!(version, 27);
    assert!(old["state"]["economy"].get("industry_keys").is_none());
    let (st, meta) = decode(&bytes).unwrap();
    assert_eq!(meta.format_version, 27);
    assert!(st.economy.industry_keys.is_empty() && st.economy.plants.is_empty());
    let places_before = st.world.places.len();
    let e = Engine::from_state(st.clone(), c.clone()).unwrap();
    let now = e.state();
    // The registries are the content's; the one market is open.
    assert_eq!(now.economy.industry_keys.len(), c.industries.len());
    assert_eq!(now.economy.market_keys, ["market.world"]);
    assert_eq!(now.economy.markets[0].index, vec![1.0; c.industries.len()]);
    // Every sector total is what the save held: a split sector's capacity,
    // jobs and wage are shared among its industries, nothing added or lost.
    let munis = old["state"]["economy"]["munis"].as_array().unwrap();
    for (m, mu) in now.economy.munis.iter().enumerate() {
        for (k, s) in mu.sectors.iter().enumerate() {
            let was = &munis[m]["sectors"][k];
            assert!(close(s.capacity, was["capacity"].as_f64().unwrap()));
            assert!(close(s.jobs, was["jobs"].as_f64().unwrap()));
            assert!(close(s.wage, was["wage"].as_f64().unwrap()), "wage {m} {k}");
            assert_eq!(
                s.baseline_jobs as u64,
                was["baseline_jobs"].as_u64().unwrap()
            );
            if c.is_split(k) && s.capacity > 0.0 {
                assert!(
                    mu.industries
                        .iter()
                        .any(|x| c.industries[x.k as usize].sector_index == k)
                );
            }
        }
    }
    // Plants only at works the towns already have: no place appeared, each
    // plant is at an open place of its industry's kind, one each.
    assert_eq!(now.world.places.len(), places_before);
    assert!(!now.economy.plants.is_empty());
    let mut seen = std::collections::BTreeSet::new();
    for p in &now.economy.plants {
        let place = &now.world.places[p.place.index()];
        let d = c.industry(&p.industry).unwrap();
        assert!(place.is_open());
        assert_eq!(place.municipality, p.municipality);
        assert_eq!(d.plants.place_kind.as_deref(), Some(place.kind.key()));
        assert!(seen.insert(p.place));
    }
    // A game with a factory gains a plant there.
    assert!(
        now.economy
            .plants
            .iter()
            .any(|p| now.world.places[p.place.index()].kind == PlaceKind::Factory),
        "a factory became a named plant"
    );
    // The same save loads the same way twice, plays on, and is written as
    // the new format and read back whole.
    let play = |turns: u32| {
        let mut e = Engine::from_state(st.clone(), c.clone()).unwrap();
        for _ in 0..turns {
            e.apply(Command::EndTurn).unwrap();
        }
        e
    };
    let a = play(12);
    assert_eq!(a.state_hash(), play(12).state_hash());
    assert!(a.state().calendar.turn >= st.calendar.turn + 12);
    for mu in &a.state().economy.munis {
        for (k, s) in mu.sectors.iter().enumerate() {
            if c.is_split(k) {
                let sum: f64 = mu
                    .industries
                    .iter()
                    .filter(|x| c.industries[x.k as usize].sector_index == k)
                    .map(|x| x.jobs)
                    .sum();
                assert_eq!(s.jobs.to_bits(), sum.to_bits());
            }
        }
    }
    let (bytes, meta) = encode(a.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.economy, a.state().economy);
    assert_eq!(back.hash(), a.state().hash());
}

#[test]
fn older_saves_load_through_the_new_version_and_play_on() {
    let c = content();
    for (name, version) in [("v25_town.mndt", 25), ("v26_town.mndt", 26)] {
        let bytes = fixture(name);
        assert_eq!(raw(&bytes).0, version);
        let (st, _) = decode(&bytes).unwrap();
        let mut e = Engine::from_state(st, c.clone()).unwrap();
        assert!(!e.state().economy.plants.is_empty(), "{name} gains plants");
        for _ in 0..8 {
            e.apply(Command::EndTurn).unwrap();
        }
        let (bytes, _) = encode(e.state(), &req()).unwrap();
        assert_eq!(decode(&bytes).unwrap().0.hash(), e.state().hash());
    }
}

#[test]
fn the_step_adds_empty_fields_and_leaves_what_is_there() {
    let (_, old) = raw(&fixture("v27_town.mndt"));
    let new = migrate(27, old.clone()).unwrap();
    let econ = &new["state"]["economy"];
    assert_eq!(econ["plants"], serde_json::json!([]));
    assert_eq!(econ["markets"], serde_json::json!([]));
    let s = &econ["munis"][0]["sectors"][0];
    assert_eq!(s["trend_jobs"], s["jobs"]);
    // A payload that already holds plants keeps them.
    let mut held = old;
    held["state"]["economy"]["plant_ids"] = serde_json::json!({ "next": 7 });
    let kept = migrate(27, held).unwrap();
    assert_eq!(kept["state"]["economy"]["plant_ids"]["next"], 7);
}
