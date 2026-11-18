#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{bundle::read_dir_sources, load};
use mandate_save::*;
use mandate_sim::Command;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn content() -> Arc<mandate_content::ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn req() -> SaveRequest {
    SaveRequest {
        run_id: "00112233445566778899aabbccddeeff".into(),
        revision: 1,
        parent_revision: None,
        device_id: "test-device".into(),
        saved_at: "2026-01-01T00:00:00Z".into(),
        protocol_version: 1,
        run_mode: RunMode::Ironman,
    }
}

#[test]
fn round_trip_preserves_state() {
    let c = content();
    let mut e = generate(
        &WorldConfig {
            seed: 11,
            size: WorldSize::Small,
            preset: None,
        },
        c.clone(),
    )
    .unwrap();
    for _ in 0..5 {
        e.apply(Command::EndTurn).unwrap();
    }
    let (bytes, meta) = encode(e.state(), &req()).unwrap();
    assert_eq!(read_meta(&bytes).unwrap(), meta);
    let (state, _) = decode(&bytes).unwrap();
    assert_eq!(state.hash(), e.state_hash());
}

#[test]
fn save_load_continue_matches_uninterrupted() {
    let c = content();
    let cfg = WorldConfig {
        seed: 12345,
        size: WorldSize::Medium,
        preset: None,
    };
    let mut a = generate(&cfg, c.clone()).unwrap();
    let mut b = generate(&cfg, c.clone()).unwrap();
    for _ in 0..10 {
        a.apply(Command::EndTurn).unwrap();
        b.apply(Command::EndTurn).unwrap();
    }
    let (bytes, _) = encode(b.state(), &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    let mut b2 = mandate_sim::Engine::from_state(state, c).unwrap();
    for _ in 0..40 {
        a.apply(Command::EndTurn).unwrap();
        b2.apply(Command::EndTurn).unwrap();
    }
    assert_eq!(a.state_hash(), b2.state_hash());
}

#[test]
fn rejects_corruption() {
    let c = content();
    let e = generate(
        &WorldConfig {
            seed: 1,
            size: WorldSize::Small,
            preset: None,
        },
        c,
    )
    .unwrap();
    let (mut bytes, _) = encode(e.state(), &req()).unwrap();
    assert!(matches!(
        decode(b"nope"),
        Err(SaveError::Malformed | SaveError::BadMagic)
    ));
    let n = bytes.len();
    bytes[n - 3] ^= 0xff;
    assert!(decode(&bytes).is_err());
}

/// If this fails, the serialized state shape may have changed: if any
/// serialized struct or enum changed, bump SAVE_FORMAT_VERSION, add a
/// migration in `migrate`, add a fixture, then update EXPECTED. The
/// fingerprint is taken from a sample game, so a content or balance change can
/// alter it with no struct change; then (and only then, after checking the
/// diff) update the hash without bumping the version.
#[test]
fn state_shape_matches_format_version() {
    const EXPECTED: (u16, &str) = (6, "060579902307894c");
    let c = content();
    let mut e = generate(
        &WorldConfig {
            seed: 3,
            size: WorldSize::Small,
            preset: None,
        },
        c,
    )
    .unwrap();
    e.apply(mandate_sim::Command::AdvanceDays { days: 800 })
        .unwrap();
    let v = serde_json::to_value(e.state()).unwrap();
    let fp = shape_fingerprint(&v);
    assert_eq!(
        (SAVE_FORMAT_VERSION, fp.as_str()),
        EXPECTED,
        "state shape changed"
    );
}

/// Base content plus an expansion that adds a fishing industry. The pack path
/// sorts before `base/`, so the new sector comes first in the registry and
/// every existing sector's index shifts (DEC-075).
fn content_with_fishing() -> Arc<mandate_content::ContentBundle> {
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    src.push(mandate_content::Source {
        path: "aa_fishing/sectors.jsonc".into(),
        text: r#"{"kind":"sectors","items":[{"id":"sector.fishing","name_key":"sector-fishing","class":"primary","driver":"land_based","base_weight":0.03,"base_wage":34000,"cyclicality":0.5,"productivity_growth":0.01}]}"#.into(),
    });
    src.push(mandate_content::Source {
        path: "aa_fishing/locale/en/fishing.ftl".into(),
        text: "sector-fishing = Fishing\n".into(),
    });
    Arc::new(load(&src).unwrap().0)
}

#[test]
fn save_loads_into_content_that_adds_a_sector() {
    let base = content();
    let mut e = generate(
        &WorldConfig {
            seed: 41,
            size: WorldSize::Small,
            preset: None,
        },
        base.clone(),
    )
    .unwrap();
    for _ in 0..5 {
        e.apply(Command::EndTurn).unwrap();
    }
    let (bytes, _) = encode(e.state(), &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    let before = state.economy.clone();

    let fishing = content_with_fishing();
    assert_eq!(fishing.sector_index("sector.fishing"), Some(0));
    let mut e2 =
        mandate_sim::Engine::from_state(state, fishing.clone()).expect("extra sector loads");
    let econ = &e2.state().economy;
    let keys: Vec<String> = fishing.sector_keys().cloned().collect();
    assert_eq!(econ.sector_keys, keys);
    for (m, mu) in econ.munis.iter().enumerate() {
        assert_eq!(mu.sectors.len(), keys.len());
        // Existing sectors keep their data under their key.
        for (old_i, k) in before.sector_keys.iter().enumerate() {
            let new_i = fishing.sector_index(k).unwrap();
            assert_eq!(mu.sectors[new_i], before.munis[m].sectors[old_i], "{k}");
        }
        let fish = &mu.sectors[0];
        assert_eq!(fish.jobs, 0.0);
        assert!(fish.capacity > 0.0);
    }
    assert!(
        e2.state()
            .events
            .iter()
            .any(|ev| matches!(ev.kind, mandate_sim::events::EventKind::RulesUpdated { .. }))
    );

    e2.apply(Command::AdvanceDays { days: 365 }).unwrap();
    let s = e2.state();
    let fish_jobs: f64 = s.economy.munis.iter().map(|m| m.sectors[0].jobs).sum();
    assert!(fish_jobs > 0.0, "the new industry grows in");
    for m in &s.economy.munis {
        assert!(m.unemployment.is_finite());
        assert!(
            m.sectors
                .iter()
                .all(|x| x.jobs.is_finite() && x.wage.is_finite())
        );
    }
    // Saves of the remapped run round-trip.
    let (bytes, _) = encode(s, &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    assert_eq!(state.hash(), e2.state_hash());
}

#[test]
fn save_refuses_content_that_removes_a_sector() {
    let e = generate(
        &WorldConfig {
            seed: 42,
            size: WorldSize::Small,
            preset: None,
        },
        content_with_fishing(),
    )
    .unwrap();
    let (bytes, _) = encode(e.state(), &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    let err = mandate_sim::Engine::from_state(state, content())
        .err()
        .expect("a removed sector refuses to load");
    assert!(err.to_string().contains("sector.fishing"), "{err}");
}
