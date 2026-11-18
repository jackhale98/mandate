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
            scenario: None,
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
        scenario: None,
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
            scenario: None,
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
/// diff) update the hash without bumping the version. A purely additive field
/// marked `#[serde(default)]` also keeps older saves loadable, so it may keep
/// the version too (old saves get the default).
#[test]
fn state_shape_matches_format_version() {
    const EXPECTED: (u16, &str) = (10, "ca0f914be85effb1");
    let c = content();
    let mut e = generate(
        &WorldConfig {
            seed: 3,
            size: WorldSize::Small,
            scenario: None,
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
            scenario: None,
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
            scenario: None,
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

/// Base content plus an expansion that adds a `land_rights` attitude. The pack
/// path sorts before `base/`, so the new attitude comes first in the registry
/// and every existing attitude's index shifts (DEC-075).
fn content_with_land_rights() -> Arc<mandate_content::ContentBundle> {
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    src.push(mandate_content::Source {
        path: "aa_land/attitudes.jsonc".into(),
        text: r#"{"kind":"attitudes","items":[{"id":"attitude.land_rights","key":"land_rights","seed":{"by_class":[0.2,0.0,-0.2],"by_neighborhood":{"rural":0.3}}}]}"#.into(),
    });
    src.push(mandate_content::Source {
        path: "aa_land/locale/en/land.ftl".into(),
        text:
            "conviction-pro-land_rights = Land back\nconviction-anti-land_rights = Settled title\n"
                .into(),
    });
    Arc::new(load(&src).unwrap().0)
}

#[test]
fn save_loads_into_content_that_adds_an_attitude() {
    let mut e = generate(
        &WorldConfig {
            seed: 43,
            size: WorldSize::Small,
            scenario: None,
        },
        content(),
    )
    .unwrap();
    for _ in 0..5 {
        e.apply(Command::EndTurn).unwrap();
    }
    let (bytes, _) = encode(e.state(), &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    let before_pop = state.population.clone();
    let before_pol = state.politics.clone();
    let old_keys = before_pop.attitude_keys.clone();
    assert_eq!(old_keys.len(), 8);

    let land = content_with_land_rights();
    assert_eq!(land.attitude_index("land_rights"), Some(0));
    let mut e2 =
        mandate_sim::Engine::from_state(state, land.clone()).expect("extra attitude loads");
    let s = e2.state();
    let keys: Vec<String> = land.attitude_keys().cloned().collect();
    assert_eq!(s.population.attitude_keys, keys);
    // Every value is kept under its key.
    let moved = |new: &[f32], old: &[f32]| {
        assert_eq!(new.len(), keys.len());
        for (oi, k) in old_keys.iter().enumerate() {
            assert_eq!(new[land.attitude_index(k).unwrap()], old[oi], "{k}");
        }
    };
    for (h, hp) in s.population.hoods.iter().enumerate() {
        let rural = s.world.neighborhoods[h].kind == mandate_sim::world::NeighborhoodKind::Rural;
        for (k, c) in hp.cohorts.iter().enumerate() {
            moved(&c.attitudes, &before_pop.hoods[h].cohorts[k].attitudes);
            // The new issue starts at its seeding model's mean (no randomness).
            let class = mandate_sim::population::class_of(k);
            let want = [0.2, 0.0, -0.2][class] + if rural { 0.3 } else { 0.0 };
            assert!((c.attitudes[0] as f64 - want).abs() < 1e-6);
        }
    }
    for (p, old) in s.politics.politicians.iter().zip(&before_pol.politicians) {
        moved(&p.attitudes, &old.attitudes);
        assert!(p.attitudes[0].is_finite());
    }
    for (p, old) in s.politics.parties.iter().zip(&before_pol.parties) {
        moved(&p.platform, &old.platform);
        assert_eq!(p.platform[0], 0.0, "parties start neutral on the new issue");
        for (f, of) in p.factions.iter().zip(&old.factions) {
            moved(&f.position, &of.position);
        }
    }
    assert!(
        s.events
            .iter()
            .any(|ev| matches!(ev.kind, mandate_sim::events::EventKind::RulesUpdated { .. }))
    );

    e2.apply(Command::AdvanceDays { days: 365 }).unwrap();
    let s = e2.state();
    for hp in &s.population.hoods {
        for c in &hp.cohorts {
            assert_eq!(c.attitudes.len(), keys.len());
            assert!(c.attitudes.iter().all(|a| a.is_finite() && a.abs() <= 1.0));
        }
    }
    for p in &s.politics.politicians {
        assert_eq!(p.attitudes.len(), keys.len());
    }
    // Saves of the remapped run round-trip.
    let (bytes, _) = encode(s, &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    assert_eq!(state.hash(), e2.state_hash());
}

#[test]
fn save_refuses_content_that_removes_an_attitude() {
    let e = generate(
        &WorldConfig {
            seed: 44,
            size: WorldSize::Small,
            scenario: None,
        },
        content_with_land_rights(),
    )
    .unwrap();
    let (bytes, _) = encode(e.state(), &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    let err = mandate_sim::Engine::from_state(state, content())
        .err()
        .expect("a removed attitude refuses to load");
    assert!(err.to_string().contains("land_rights"), "{err}");
}

/// Base content plus an expansion that adds a First Nation community. The pack
/// path sorts before `base/`, so the new community comes first in the registry
/// and the general community's cohorts move to the second block (DEC-075).
fn content_with_first_nation() -> Arc<mandate_content::ContentBundle> {
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    src.push(mandate_content::Source {
        path: "aa_nation/communities.jsonc".into(),
        text: r#"{"kind":"communities","items":[{"id":"community.first_nation","name_key":"community-first_nation","share":{"by_neighborhood":{"rural":0.3,"industrial":0.05},"regional_sd":0.05},"attitude_offsets":{"localism":0.3,"environment":0.2},"turnout_offset":-0.08,"immigration_share":0.1}]}"#.into(),
    });
    src.push(mandate_content::Source {
        path: "aa_nation/locale/en/nation.ftl".into(),
        text: "community-first_nation = First Nation\n".into(),
    });
    Arc::new(load(&src).unwrap().0)
}

#[test]
fn save_loads_into_content_that_adds_a_community() {
    let mut e = generate(
        &WorldConfig {
            seed: 45,
            size: WorldSize::Small,
            scenario: None,
        },
        content(),
    )
    .unwrap();
    for _ in 0..5 {
        e.apply(Command::EndTurn).unwrap();
    }
    let (bytes, _) = encode(e.state(), &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    let before = state.population.clone();
    assert_eq!(before.community_keys, vec!["community.general".to_string()]);
    assert!(
        before
            .hoods
            .iter()
            .all(|h| h.children_by_community.is_empty())
    );

    let nation = content_with_first_nation();
    assert_eq!(nation.community_index("community.first_nation"), Some(0));
    let general = nation.community_index("community.general").unwrap();
    assert_eq!(general, 1);
    let mut e2 =
        mandate_sim::Engine::from_state(state, nation.clone()).expect("extra community loads");
    let s = e2.state();
    let keys: Vec<String> = nation.community_keys().cloned().collect();
    assert_eq!(s.population.community_keys, keys);
    let g = mandate_sim::population::GROUPS;
    for (h, hp) in s.population.hoods.iter().enumerate() {
        let old = &before.hoods[h];
        assert_eq!(hp.cohorts.len(), 2 * g);
        // The existing people keep their cohorts under their community's id.
        assert_eq!(
            &hp.cohorts[general * g..(general + 1) * g],
            &old.cohorts[..]
        );
        // The new community starts empty, seeded without randomness.
        for (ci, c) in hp.cohorts[..g].iter().enumerate() {
            assert_eq!(c.count, 0);
            assert_eq!(mandate_sim::population::community_of(ci), 0);
            assert_eq!(c.attitudes.len(), nation.attitudes.len());
            assert_eq!(c.party_id.len(), s.politics.parties.len());
        }
        assert_eq!(hp.children, old.children);
        assert_eq!(hp.children_by_community, vec![0, old.children]);
    }
    assert_eq!(s.population.total(), before.total());
    assert!(
        s.events
            .iter()
            .any(|ev| matches!(ev.kind, mandate_sim::events::EventKind::RulesUpdated { .. }))
    );

    // The community grows through immigration; everything keeps running.
    e2.apply(Command::AdvanceDays { days: 365 }).unwrap();
    let s = e2.state();
    let newcomers: u64 = s
        .population
        .hoods
        .iter()
        .map(|h| h.community_adults(0))
        .sum();
    assert!(newcomers > 0, "the new community gains immigrants");
    for hp in &s.population.hoods {
        assert_eq!(hp.cohorts.len(), 2 * g);
        assert_eq!(hp.children, hp.children_by_community.iter().sum::<u32>());
    }
    let (bytes, _) = encode(s, &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    assert_eq!(state.hash(), e2.state_hash());
}

#[test]
fn save_refuses_content_that_removes_a_community() {
    let e = generate(
        &WorldConfig {
            seed: 46,
            size: WorldSize::Small,
            scenario: None,
        },
        content_with_first_nation(),
    )
    .unwrap();
    let (bytes, _) = encode(e.state(), &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    let err = mandate_sim::Engine::from_state(state, content())
        .err()
        .expect("a removed community refuses to load");
    assert!(err.to_string().contains("community.first_nation"), "{err}");
}

/// A v9 save named its world's preset with an enum; v10 names a scenario
/// content id (DEC-078 decision 4). The migration maps the five presets.
#[test]
fn v9_saves_migrate_their_preset_to_a_scenario_id() {
    let e = generate(
        &WorldConfig::with_scenario(47, WorldSize::Small, "scenario.wealthy_coast"),
        content(),
    )
    .unwrap();
    let mut v = serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap() });
    // Rewrite the payload into the v9 shape.
    let params = v.pointer_mut("/state/meta/params").unwrap();
    let obj = params.as_object_mut().unwrap();
    obj.remove("scenario");
    obj.insert("preset".into(), "WealthyCoast".into());
    let migrated = mandate_save::migrate(9, v).unwrap();
    let state: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    assert_eq!(
        state.meta.params.scenario.as_deref(),
        Some("scenario.wealthy_coast")
    );
    assert_eq!(state.hash(), e.state_hash());
    let mut e2 = mandate_sim::Engine::from_state(state, content()).unwrap();
    e2.apply(Command::EndTurn).unwrap();
    // A random country and an unknown preset.
    let mut v = serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap() });
    let obj = v
        .pointer_mut("/state/meta/params")
        .unwrap()
        .as_object_mut()
        .unwrap();
    obj.remove("scenario");
    obj.insert("preset".into(), serde_json::Value::Null);
    let m = mandate_save::migrate(9, v.clone()).unwrap();
    assert!(m.pointer("/state/meta/params/scenario").unwrap().is_null());
    if let Some(p) = v.pointer_mut("/state/meta/params/preset") {
        *p = "Atlantis".into();
    }
    assert!(mandate_save::migrate(9, v).is_err());
}

#[test]
fn save_refuses_content_that_removes_its_scenario() {
    let e = generate(
        &WorldConfig::with_scenario(48, WorldSize::Small, "scenario.drought_farmland"),
        content(),
    )
    .unwrap();
    let (bytes, _) = encode(e.state(), &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    let f = src
        .iter_mut()
        .find(|s| s.path == "base/scenarios.jsonc")
        .unwrap();
    f.text = r#"{"kind":"scenarios","items":[]}"#.into();
    let fewer = Arc::new(load(&src).unwrap().0);
    let err = mandate_sim::Engine::from_state(state.clone(), fewer)
        .err()
        .expect("a removed scenario refuses to load");
    assert!(
        err.to_string().contains("scenario.drought_farmland"),
        "{err}"
    );
    // With its scenario present it loads.
    assert!(mandate_sim::Engine::from_state(state, content()).is_ok());
}
