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
    const EXPECTED: (u16, &str) = (13, "9d193ebd40fd1955");
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
            "conviction-pro-land_rights = Land back\nconviction-anti-land_rights = Settled title\nissue-land_rights = Land rights\n"
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
    let mut v = v10_payload(e.state());
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
    assert_eq!(
        state.world.municipalities.len(),
        e.state().world.municipalities.len()
    );
    let mut e2 = mandate_sim::Engine::from_state(state, content()).unwrap();
    e2.apply(Command::EndTurn).unwrap();
    // A random country and an unknown preset.
    let mut v = v10_payload(e.state());
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

/// Concerns are derived state (DEC-078 decision 3): a save loads into content
/// that adds or removes a concern, and the towns' concerns are recomputed.
#[test]
fn save_loads_into_content_that_adds_or_removes_a_concern() {
    let mut e = generate(
        &WorldConfig {
            seed: 49,
            size: WorldSize::Small,
            scenario: None,
        },
        content(),
    )
    .unwrap();
    for _ in 0..3 {
        e.apply(Command::EndTurn).unwrap();
    }
    let (bytes, _) = encode(e.state(), &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    assert_eq!(state.population.concern_keys.len(), 6);
    let base_src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();

    // Added: an expansion concern sorts first (its pack path sorts before base/).
    let mut src = base_src.clone();
    src.push(mandate_content::Source {
        path: "aa_heritage/concerns.jsonc".into(),
        text: r#"{"kind":"concerns","items":[{"id":"concern.heritage","name_key":"concern-heritage","indicator":"pollution","attitudes":{"tradition":1.0},"level":{"baseline":0.0,"slope":1.0,"min":0.0,"max":1.0}}]}"#.into(),
    });
    src.push(mandate_content::Source {
        path: "aa_heritage/locale/en/h.ftl".into(),
        text: "concern-heritage = Heritage\n".into(),
    });
    let more = Arc::new(load(&src).unwrap().0);
    let mut e2 = mandate_sim::Engine::from_state(state.clone(), more.clone()).unwrap();
    let pop = &e2.state().population;
    assert_eq!(pop.concern_keys[0], "concern.heritage");
    assert_eq!(pop.concern_keys.len(), 7);
    assert!(pop.town_concerns.iter().all(|t| t.levels.len() == 7));
    // The other domains are untouched.
    assert_eq!(e2.state().population.hoods, state.population.hoods);
    e2.apply(Command::EndTurn).unwrap();

    // Removed: not refused; the remaining concerns are recomputed.
    let mut src = base_src;
    let f = src
        .iter_mut()
        .find(|s| s.path == "base/concerns.jsonc")
        .unwrap();
    f.text = r#"{"kind":"concerns","items":[{"id":"concern.jobs","name_key":"concern-jobs","indicator":"unemployment","attitudes":{"redistribution":0.6},"level":{"baseline":0.0,"slope":9.0,"min":0.0,"max":1.0}}]}"#.into();
    let fewer = Arc::new(load(&src).unwrap().0);
    let e3 = mandate_sim::Engine::from_state(state, fewer).expect("a removed concern still loads");
    assert_eq!(
        e3.state().population.concern_keys,
        vec!["concern.jobs".to_string()]
    );
    assert!(
        e3.state()
            .population
            .town_concerns
            .iter()
            .all(|t| t.levels.len() == 1)
    );
}

/// A v10 save held each politician's opinion of the player and favours on the
/// politician; v11 keeps them on relationship edges and the favour ledger
/// (DEC-031). The migration moves them, and loading generates the character
/// records, the towns' Named characters, and town characters.
#[test]
fn v10_saves_migrate_to_characters() {
    let c = content();
    let mut e = generate(
        &WorldConfig {
            seed: 53,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    for _ in 0..8 {
        e.apply(Command::EndTurn).unwrap();
    }
    let st = e.state();
    let me = mandate_sim::characters::player_character(st).unwrap();
    // Give the player a debt to someone, so favours migrate too.
    let creditor = st
        .politics
        .politicians
        .iter()
        .find(|p| !p.is_player)
        .unwrap()
        .character;
    let mut e = e;
    e.setup(|s| {
        let d = s.calendar.date;
        mandate_sim::characters::owe(&mut s.politics, me, creditor, "favour-bill", None, d);
    });
    let st = e.state();
    let expected: Vec<(f32, i32)> = st
        .politics
        .politicians
        .iter()
        .map(|p| {
            if p.is_player {
                return (0.0, 0);
            }
            (
                mandate_sim::characters::opinion(&st.politics, p.character, me),
                mandate_sim::characters::favour_balance(&st.politics, p.character, me),
            )
        })
        .collect();
    // Rewrite the payload into the v10 shape.
    let v = v10_payload(st);
    let migrated = mandate_save::migrate(10, v).unwrap();
    let state: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    let mut e2 = mandate_sim::Engine::from_state(state, c.clone()).unwrap();
    let s2 = e2.state();
    let me2 = mandate_sim::characters::player_character(s2).unwrap();
    assert_eq!(
        s2.politics.characters.people.len(),
        s2.politics.politicians.len() + named_count(s2)
    );
    for (p, (op, fav)) in s2.politics.politicians.iter().zip(&expected) {
        assert_eq!(p.character.index(), p.id.index());
        if p.is_player {
            continue;
        }
        assert_eq!(
            mandate_sim::characters::opinion(&s2.politics, p.character, me2),
            *op
        );
        assert_eq!(
            mandate_sim::characters::favour_balance(&s2.politics, p.character, me2),
            *fav
        );
    }
    assert!(s2.politics.characters.named_seeded);
    assert_eq!(
        s2.politics.characters.towns.len(),
        s2.world.municipalities.len()
    );
    for _ in 0..5 {
        e2.apply(Command::EndTurn).unwrap();
    }
    let (bytes, _) = encode(e2.state(), &req()).unwrap();
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e2.state_hash());
}

/// A save payload in the v10 shape (before characters): each politician's
/// opinion of the player and favours on the politician.
fn v10_payload(st: &mandate_sim::State) -> serde_json::Value {
    let me = mandate_sim::characters::player_character(st);
    let mut v = serde_json::json!({ "state": serde_json::to_value(st).unwrap() });
    let pols = v
        .pointer_mut("/state/politics/politicians")
        .unwrap()
        .as_array_mut()
        .unwrap();
    for (p, sp) in pols.iter_mut().zip(&st.politics.politicians) {
        let (op, fav) = match me {
            Some(me) if !sp.is_player => (
                mandate_sim::characters::opinion(&st.politics, sp.character, me),
                mandate_sim::characters::favour_balance(&st.politics, sp.character, me),
            ),
            _ => (0.0, 0),
        };
        let o = p.as_object_mut().unwrap();
        o.remove("character");
        o.insert("opinion_of_player".into(), serde_json::json!(op));
        o.insert("favors".into(), serde_json::json!(fav));
    }
    v.pointer_mut("/state/politics")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("characters");
    v
}

fn named_count(s: &mandate_sim::State) -> usize {
    s.politics
        .characters
        .people
        .iter()
        .filter(|c| c.named.is_some())
        .count()
}

/// Characters keep their traits and occupations: content that removes one
/// the save uses is refused; content that adds or removes a crisis kind just
/// recomputes town characters.
#[test]
fn saves_and_the_character_registries() {
    let c = content();
    let e = generate(
        &WorldConfig {
            seed: 57,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    let (bytes, _) = encode(e.state(), &req()).unwrap();
    let load_with = |edit: &dyn Fn(&mut Vec<mandate_content::Source>)| {
        let mut src =
            read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
        edit(&mut src);
        let bundle = Arc::new(load(&src).unwrap().0);
        let (state, _) = decode(&bytes).unwrap();
        mandate_sim::Engine::from_state(state, bundle)
    };
    // A used occupation removed: refused, naming it. (One only outlets name
    // besides its own file; the outlets lose it too, so the content itself
    // still validates.)
    let all =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    let named_elsewhere = |id: &str| {
        all.iter().any(|s| {
            !matches!(
                s.path.as_str(),
                "base/occupations.jsonc" | "base/outlets.jsonc"
            ) && s.text.contains(&format!("\"{id}\""))
        })
    };
    let used = e
        .state()
        .politics
        .characters
        .people
        .iter()
        .filter_map(|p| p.named.as_deref().map(|n| n.occupation.clone()))
        .find(|o| !named_elsewhere(o))
        .unwrap();
    let r = load_with(&|src| {
        let f = src
            .iter_mut()
            .find(|s| s.path == "base/occupations.jsonc")
            .unwrap();
        let mut v: serde_json::Value =
            serde_json::from_str(&mandate_content::jsonc::to_strict_json(&f.text)).unwrap();
        v["items"]
            .as_array_mut()
            .unwrap()
            .retain(|o| o["id"] != serde_json::json!(used));
        f.text = v.to_string();
        let f = src
            .iter_mut()
            .find(|s| s.path == "base/outlets.jsonc")
            .unwrap();
        let mut v: serde_json::Value =
            serde_json::from_str(&mandate_content::jsonc::to_strict_json(&f.text)).unwrap();
        for o in v["items"].as_array_mut().unwrap() {
            if let Some(js) = o.get_mut("journalists").and_then(|j| j.as_array_mut()) {
                js.retain(|j| j != &serde_json::json!(used));
            }
        }
        f.text = v.to_string();
    });
    match r {
        Err(err) => assert!(err.to_string().contains(&used), "{err}"),
        Ok(_) => panic!("a removed occupation must be refused"),
    }
    // Crisis kinds are derived: removing them all loads and recomputes.
    let e2 = load_with(&|src| {
        let f = src
            .iter_mut()
            .find(|s| s.path == "base/crises.jsonc")
            .unwrap();
        f.text = r#"{"kind":"crises","items":[]}"#.into();
        // Occupations name crisis kinds in their stakes.
        let o = src
            .iter_mut()
            .find(|s| s.path == "base/occupations.jsonc")
            .unwrap();
        let mut v: serde_json::Value =
            serde_json::from_str(&mandate_content::jsonc::to_strict_json(&o.text)).unwrap();
        for item in v["items"].as_array_mut().unwrap() {
            if let Some(st) = item.get_mut("stake") {
                st["crises"] = serde_json::json!([]);
            }
        }
        o.text = v.to_string();
    })
    .unwrap();
    assert!(e2.state().politics.characters.crisis_keys.is_empty());
    assert!(
        e2.state()
            .politics
            .characters
            .towns
            .iter()
            .all(|t| t.crises.is_empty())
    );
}

/// v12 added the narrative domain (DEC-033), NPC goals, and the director's
/// held hazards and recessions. A v11 save loads with the director starting
/// fresh, and plays on.
#[test]
fn v11_saves_migrate_to_the_narrative() {
    let c = content();
    let mut e = generate(
        &WorldConfig {
            seed: 61,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    e.apply(Command::AdvanceDays { days: 400 }).unwrap();
    // The v11 shape: no narrative, goals, held hazards, or held recession.
    let mut v = serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap() });
    let st = v.pointer_mut("/state").unwrap().as_object_mut().unwrap();
    st.remove("narrative");
    for (path, key) in [
        ("/state/politics/characters", "goals"),
        ("/state/politics/characters", "ended_goals"),
        ("/state/environment", "deferred"),
        ("/state/economy/global", "held_recession"),
    ] {
        v.pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
    }
    let migrated = mandate_save::migrate(11, v).unwrap();
    let state: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    assert!(state.narrative.decisions.is_empty());
    assert!(state.narrative.chapter.is_none());
    assert!(state.politics.characters.goals.is_empty());
    let mut e2 = mandate_sim::Engine::from_state(state, c).unwrap();
    for _ in 0..30 {
        e2.apply(Command::EndTurn).unwrap();
    }
    // The director picked up where the save left off.
    assert!(e2.state().narrative.chapter.is_some());
    let (bytes, _) = encode(e2.state(), &req()).unwrap();
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e2.state_hash());
}

/// Narrative templates are rules like any other, but a save never needs
/// them: content that removes a dilemma lapses its pending decisions, one
/// that removes a storyline ends its runs, and one that removes a goal drops
/// it (DEC-074). Nothing is refused.
#[test]
fn a_removed_dilemma_template_does_not_break_loading() {
    let c = content();
    let mut e = generate(
        &WorldConfig {
            seed: 62,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    // Play until something is pending.
    for _ in 0..200 {
        if e.state().narrative.pending().next().is_some() {
            break;
        }
        e.apply(Command::EndTurn).unwrap();
    }
    let pending = e
        .state()
        .narrative
        .pending()
        .next()
        .cloned()
        .expect("a decision is pending");
    let (bytes, _) = encode(e.state(), &req()).unwrap();
    // Remove the pending decision's template, every storyline, and every goal.
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    for s in src.iter_mut() {
        match s.path.as_str() {
            "base/dilemmas.jsonc" => {
                let mut v: serde_json::Value =
                    serde_json::from_str(&mandate_content::jsonc::to_strict_json(&s.text)).unwrap();
                v["items"].as_array_mut().unwrap().retain(|d| {
                    d["id"] != serde_json::json!(pending.template)
                        && d["category"] != serde_json::json!("request")
                });
                s.text = v.to_string();
            }
            "base/storylines.jsonc" => s.text = r#"{"kind":"storylines","items":[]}"#.into(),
            "base/goals.jsonc" => s.text = r#"{"kind":"goals","items":[]}"#.into(),
            _ => {}
        }
    }
    let fewer = Arc::new(load(&src).unwrap().0);
    let (state, _) = decode(&bytes).unwrap();
    let mut e2 = mandate_sim::Engine::from_state(state, fewer).expect("loads");
    let d = e2.state().narrative.decision(pending.id).unwrap();
    assert_eq!(d.status, mandate_sim::narrative::DecisionStatus::Lapsed);
    assert!(e2.state().narrative.active_runs().next().is_none());
    assert!(e2.state().politics.characters.goals.is_empty());
    for _ in 0..20 {
        e2.apply(Command::EndTurn).unwrap();
    }
    assert!(
        e2.state()
            .narrative
            .decisions
            .iter()
            .all(|d| d.template != pending.template || d.id == pending.id)
    );
}

/// v13 added media and justice (DEC-034, DEC-006): outlets, knowledge,
/// stories, scandals, rumours, cases, a career's status, and hidden facts'
/// fates. A v12 save loads with media and justice empty, gets its outlets
/// on load, and plays on; the press then covers what happens from there.
#[test]
fn v12_saves_migrate_to_media_and_justice() {
    let c = content();
    let mut e = generate(
        &WorldConfig {
            seed: 63,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    e.apply(Command::AdvanceDays { days: 300 }).unwrap();
    // The v12 shape: no media, justice, or status; hidden facts and
    // decisions without their new fields; no media RNG stream.
    let mut v = serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap() });
    let st = v.pointer_mut("/state").unwrap().as_object_mut().unwrap();
    st.remove("media");
    st.remove("justice");
    st.get_mut("player")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("status");
    let n = st.get_mut("narrative").unwrap().as_object_mut().unwrap();
    n.remove("raised");
    n.remove("shape_discovery");
    for h in n.get_mut("hidden").unwrap().as_array_mut().unwrap() {
        let h = h.as_object_mut().unwrap();
        for k in ["subject", "severity", "evidence", "town", "fate"] {
            h.remove(k);
        }
    }
    for d in n.get_mut("decisions").unwrap().as_array_mut().unwrap() {
        d.as_object_mut().unwrap().remove("about");
    }
    st.get_mut("rng")
        .unwrap()
        .get_mut("states")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("Media");
    let migrated = mandate_save::migrate(12, v).unwrap();
    let state: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    assert!(!state.media.active && state.media.outlets.is_empty());
    assert!(state.justice.cases.is_empty());
    assert!(state.narrative.hidden.iter().all(|h| h.fate.is_none()));
    let mut e2 = mandate_sim::Engine::from_state(state, c).unwrap();
    assert!(e2.state().media.active, "outlets are generated on load");
    assert!(!e2.state().media.outlets.is_empty());
    for _ in 0..30 {
        e2.apply(Command::EndTurn).unwrap();
    }
    assert!(
        !e2.state().media.stories.is_empty(),
        "the press covers the town from here on"
    );
    let (bytes, _) = encode(e2.state(), &req()).unwrap();
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e2.state_hash());
}
