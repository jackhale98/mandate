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
    // 9193f13a5022eb59 -> dd1fe79093dba355: NPC votes follow the platform party
    // line (W1 note 9). No serialized struct changed (checked in the diff); the
    // sample game's contents did, so the version stays 25.
    // dd1fe79093dba355 -> c8db676e2177925f: rents bounded by incomes and movers
    // shared without regard to list position (audit
    // fix 2); values only, no struct changed.
    //
    // c8db676e2177925f (v25) -> 740f115b1c8d9bf3 (v26): the causal spine (S1) and the
    // last post held. `events` gained typed causes, thread heads and
    // `pruned_on`; a hidden fact its `cause`; a Legacy entry its
    // `source_event`; a held hazard its `hold_event`; a politician
    // `last_post`. Migration: `causal_spine`.
    // 740f115b1c8d9bf3 -> e90c9a8a50934436: the record decides votes (audit
    // fix 1). No serialized struct changed; the sample game's history did.
    //
    // e90c9a8a50934436 (v26) -> the same (v27): staged arcs (A1, DEC-109). The
    // state gained `arcs` (not written while no arc has ever run), a
    // decision's and an ask's `arc`, and the environment's `repairs`: each
    // absent until an arc runs, and the sample game has no flood in the
    // player's town, so its shape is the one it had. The version moves
    // because a v27 save may hold them and a v26 build cannot read them.
    // Migration: `storylines_to_arcs`.
    // (Audit fixes 3, 5 and 8 moved the fingerprint without a version: no
    // serialized struct changed; a law's ledger, a map, has new keys
    // (`homes_started`, `homes_building`, `homes_added`, `jobs_added_*`,
    // ...) in place of `homes_built`, which an older build reads as any
    // other key, and the sample game's contents moved. Recorded with them,
    // on v28: 336f9ea0b265bcff.)
    //
    // v28 -> v29: industries and named plants (I1, DEC-110). The economy
    // gained `industry_keys`, `market_keys`, `markets`, `good_keys`,
    // `plants` and `plant_ids`; a town its `industries` (and `prospect`,
    // `last_notice` when set); a sector `trend_jobs`; a shock its
    // `industry` and `demand` when set; the plant events and pressures on
    // the spine. Migration: `industries` (the engine shares each split
    // sector among its industries as it loads, as a new world does).
    //
    // v29 (04a127111f358b05) -> v30: the impact estimate a law passed with
    // (DEC-111). A law gained `estimate`, absent on a bill not yet passed,
    // on a law from an older save, and on a law ended and enacted over
    // twelve years ago. Migration: `frozen_estimates`.
    //
    // v30 -> v31: exchange scenes (X1, DEC-112). An arc run gained
    // `exchange` (a scene under way) and `played` (its finished scenes),
    // an approval request its `communities`, a story the source
    // `exchange`, the log the events `arc_exchange` and
    // `arc_exchange_ended`: each absent until a scene is held, and the
    // sample game holds none, so its shape is the one it had. The version
    // moves because a v31 save may hold them and a v30 build cannot read
    // them. Migration: `exchange_scenes` (a challenge waiting as a
    // decision lapses).
    const EXPECTED: (u16, &str) = (31, "04a127111f358b05");
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
    // `scripts/repin.sh` re-records the fingerprint: it prints it here and
    // leaves the assertion to the confirming run.
    if std::env::var_os("MANDATE_RECORD_PINS").is_some() {
        println!("REPIN shape {fp}");
        return;
    }
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

/// Whether an item names a role list holding only `role`.
fn only_for(x: &serde_json::Value, role: &serde_json::Value) -> bool {
    match x {
        serde_json::Value::Array(a) => {
            (a.len() == 1 && &a[0] == role) || a.iter().any(|y| only_for(y, role))
        }
        serde_json::Value::Object(o) => o.values().any(|y| only_for(y, role)),
        _ => false,
    }
}

/// Remove `role` from every list in a value.
fn strip(x: &mut serde_json::Value, role: &serde_json::Value) {
    match x {
        serde_json::Value::Array(a) => {
            a.retain(|y| y != role);
            a.iter_mut().for_each(|y| strip(y, role));
        }
        serde_json::Value::Object(o) => o.values_mut().for_each(|y| strip(y, role)),
        _ => {}
    }
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
    // A used occupation removed: refused, naming it. (One only outlets,
    // conversations and routines name besides its own file; those lose it
    // too, so the content itself still validates.)
    let all =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    let named_elsewhere = |id: &str| {
        all.iter().any(|s| {
            !matches!(
                s.path.as_str(),
                "base/occupations.jsonc" | "base/outlets.jsonc" | "base/routines.jsonc"
            ) && !s.path.starts_with("base/conversations/")
                && s.text.contains(&format!("\"{id}\""))
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
        // Routines: its routine goes.
        let f = src
            .iter_mut()
            .find(|s| s.path == "base/routines.jsonc")
            .unwrap();
        let mut v: serde_json::Value =
            serde_json::from_str(&mandate_content::jsonc::to_strict_json(&f.text)).unwrap();
        v["items"]
            .as_array_mut()
            .unwrap()
            .retain(|r| r["occupation"] != serde_json::json!(used));
        f.text = v.to_string();
        // Conversations: the role leaves every list, and a line or topic
        // left for no role at all goes.
        for f in src
            .iter_mut()
            .filter(|s| s.path.starts_with("base/conversations/"))
        {
            let mut v: serde_json::Value =
                serde_json::from_str(&mandate_content::jsonc::to_strict_json(&f.text)).unwrap();
            let gone = serde_json::json!(used);
            if let Some(items) = v.get_mut("items").and_then(|i| i.as_array_mut()) {
                items.retain(|x| !only_for(x, &gone));
            }
            strip(&mut v, &gone);
            f.text = v.to_string();
        }
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
    // (An older save holds no arc run: the budget round is in every played
    // game since A2, and the step to v27 refuses a payload that carries one.)
    v["state"].as_object_mut().unwrap().remove("arcs");
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
/// that removes an arc ends its runs, and one that removes a goal drops
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
    // Remove the pending decision's template, every arc (with the
    // dilemmas, asks and scenes its file carries), and every goal.
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
            "base/goals.jsonc" => s.text = r#"{"kind":"goals","items":[]}"#.into(),
            p if p.starts_with("base/arcs/") => s.text = r#"{"kind":"arcs","items":[]}"#.into(),
            _ => {}
        }
    }
    let fewer = Arc::new(load(&src).unwrap().0);
    let (state, _) = decode(&bytes).unwrap();
    let mut e2 = mandate_sim::Engine::from_state(state, fewer).expect("loads");
    let d = e2.state().narrative.decision(pending.id).unwrap();
    assert_eq!(d.status, mandate_sim::narrative::DecisionStatus::Lapsed);
    assert!(e2.state().politics.characters.goals.is_empty());
    for _ in 0..20 {
        e2.apply(Command::EndTurn).unwrap();
    }
    // Every run of a removed arc ended at its next step.
    assert!(e2.state().arcs.runs.is_empty());
    assert!(
        e2.state()
            .arcs
            .past
            .iter()
            .all(|p| p.exit == mandate_sim::arcs::EXIT_REMOVED)
    );
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
    // (An older save holds no arc run; the step to v27 refuses one that does.)
    v["state"].as_object_mut().unwrap().remove("arcs");
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

/// Strip the town baselines (TOWN_GENERATION §15 E1) from a payload: the
/// shape of the format before them.
fn without_baselines(v: &mut serde_json::Value) {
    for h in v
        .pointer_mut("/state/population/hoods")
        .unwrap()
        .as_array_mut()
        .unwrap()
    {
        h.as_object_mut().unwrap().remove("baseline_population");
    }
    for mu in v
        .pointer_mut("/state/economy/munis")
        .unwrap()
        .as_array_mut()
        .unwrap()
    {
        let mu = mu.as_object_mut().unwrap();
        for s in mu["sectors"].as_array_mut().unwrap() {
            s.as_object_mut().unwrap().remove("baseline_jobs");
        }
        mu["housing"]
            .as_object_mut()
            .unwrap()
            .remove("baseline_stock");
    }
}

/// The format after v13 added town baselines (TOWN_GENERATION §15 E1):
/// residents per neighbourhood, jobs per sector, and housing stock, as play
/// began. A save from before takes them from its current values (whole
/// units), keeps them through play, and round-trips.
#[test]
fn v13_saves_migrate_to_town_baselines() {
    let c = content();
    let mut e = generate(
        &WorldConfig {
            seed: 65,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    e.apply(Command::AdvanceDays { days: 500 }).unwrap();
    let mut v = serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap() });
    without_baselines(&mut v);
    assert!(
        serde_json::from_value::<mandate_sim::State>(v["state"].clone()).is_err(),
        "the older payload lacks the baselines"
    );
    // (An older save holds no arc run; the step to v27 refuses one that does.)
    v["state"].as_object_mut().unwrap().remove("arcs");
    let migrated = mandate_save::migrate(13, v.clone()).unwrap();
    assert_eq!(migrated, mandate_save::migrate(13, v).unwrap());
    let state: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    // The baselines are the save's own values, in whole units.
    for (h, hp) in state.population.hoods.iter().enumerate() {
        assert_eq!(
            u64::from(hp.baseline_population),
            state.population.hood_total(h)
        );
    }
    for mu in &state.economy.munis {
        for s in &mu.sectors {
            assert_eq!(s.baseline_jobs, s.jobs.round() as u32);
        }
        assert_eq!(mu.housing.baseline_stock, mu.housing.stock.round() as u32);
    }
    let m = state.player.as_ref().unwrap().municipality;
    let before = state.town_baseline(m).unwrap();
    assert_eq!(
        before.population,
        state.population.municipality_total(&state.world, m)
    );
    assert!(before.housing_stock > 0 && before.sectors.iter().any(|s| s.jobs > 0));
    // Play on: the baselines never move.
    let mut e2 = mandate_sim::Engine::from_state(state, c).unwrap();
    for _ in 0..12 {
        e2.apply(Command::EndTurn).unwrap();
    }
    assert_eq!(e2.state().town_baseline(m).unwrap(), before);
    let (bytes, meta) = encode(e2.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e2.state_hash());
    assert_eq!(back.town_baseline(m).unwrap(), before);
}

/// A file in the format before the baselines (v13: its header, its payload
/// shape) decodes through the whole chain. For a game saved as play begins,
/// the migrated baselines equal the ones a new game records, so the state
/// is identical.
#[test]
fn v13_files_decode_with_town_baselines() {
    let c = content();
    let e = generate(
        &WorldConfig {
            seed: 66,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    let mut v = serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap() });
    without_baselines(&mut v);
    // v13 also predates hazard exposure and the knowledge ledger (later formats).
    without_hazards(&mut v);
    let bytes = old_file(e.state(), &v, 13);
    let (mut state, read) = decode(&bytes).unwrap();
    assert_eq!(read.format_version, 13);
    // Exposure comes from the save's profile (no terrain in a save), and the
    // history's damage, displacement and impact are not on record; the
    // knowledge ledger is seeded on load, not migrated; neighbourhood
    // concerns are derived on load (v18), and the town levels this
    // payload holds (felt, as written today) stand in for the measured ones
    // an older build wrote. The rest is identical.
    let st = e.state();
    assert!(state.population.hood_concerns.is_empty());
    state.population.hood_concerns = st.population.hood_concerns.clone();
    state.population.town_concerns = st.population.town_concerns.clone();
    state.environment.exposure = st.environment.exposure.clone();
    state.environment.hood_damage = st.environment.hood_damage.clone();
    state.population.hood_impact = st.population.hood_impact.clone();
    state.population.displaced = st.population.displaced.clone();
    // The act flow (v23) is new state: a migrated run starts it empty.
    assert!(state.act.records.is_empty());
    state.act = st.act.clone();
    let strip = |s: &mandate_sim::State| {
        let mut v = serde_json::to_value(s).unwrap();
        v["player"].as_object_mut().unwrap().remove("knowledge");
        v
    };
    assert_eq!(strip(&state), strip(st));
    mandate_sim::Engine::from_state(state, c).unwrap();
}

/// The format after v14 added the player's knowledge ledger (DEC-091). An
/// older save loads with an unseeded ledger and is seeded generously on load
/// (`Migrated`): the default profile, the local option, and everyone with a
/// relationship to the player known; then it plays on and round-trips. A v13
/// save (no baselines, no ledger) goes through both steps (13 → 14 → 15).
#[test]
fn older_saves_migrate_to_the_knowledge_ledger() {
    use mandate_sim::player::knowledge::{LedgerOrigin, Level, Source, Subject};
    let c = content();
    let mut e = generate(
        &WorldConfig {
            seed: 64,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    for _ in 0..6 {
        e.apply(Command::EndTurn).unwrap();
    }
    let fresh = e.state().player.as_ref().unwrap().knowledge.clone();
    assert!(matches!(fresh.origin, LedgerOrigin::Profile { .. }));
    // The v14 shape: no ledger on the player.
    let mut v14 = serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap() });
    v14.pointer_mut("/state/player")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("knowledge");
    // The v13 shape: no baselines either.
    let mut v13 = v14.clone();
    without_baselines(&mut v13);
    let from13 = mandate_save::migrate(13, v13).unwrap();
    let migrated = mandate_save::migrate(14, v14).unwrap();
    // Through both steps: baselines from the save's values, the ledger unseeded.
    let s13: mandate_sim::State = serde_json::from_value(from13["state"].clone()).unwrap();
    assert_eq!(
        s13.player.as_ref().unwrap().knowledge.origin,
        LedgerOrigin::Unseeded
    );
    assert_eq!(from13["state"]["player"], migrated["state"]["player"]);
    let e13 = mandate_sim::Engine::from_state(s13, c.clone()).unwrap();
    assert_eq!(
        e13.state().player.as_ref().unwrap().knowledge.origin,
        LedgerOrigin::Migrated
    );
    let state: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    assert_eq!(
        state.player.as_ref().unwrap().knowledge.origin,
        LedgerOrigin::Unseeded
    );
    let me = mandate_sim::characters::player_character(&state).unwrap();
    let related: Vec<_> = state
        .politics
        .characters
        .edges
        .keys()
        .filter(|(h, o)| *o == me.0 && *h != me.0)
        .map(|(h, _)| mandate_sim::characters::CharacterId(*h))
        .collect();
    assert!(!related.is_empty(), "the player has relationships to keep");
    let mut e2 = mandate_sim::Engine::from_state(state, c).unwrap();
    let k = &e2.state().player.as_ref().unwrap().knowledge;
    assert_eq!(k.origin, LedgerOrigin::Migrated);
    assert!(k.zone.is_some());
    // Generous: at least what a fresh run of today's start knows, and more.
    for (s, entry) in &fresh.entries {
        assert!(k.level(s) >= entry.level, "{s:?} was known before");
    }
    assert!(k.entries.len() > fresh.entries.len());
    for person in related {
        assert!(k.level(&Subject::Person { person }) >= Level::Known);
    }
    let town = e2.state().player.as_ref().unwrap().municipality;
    assert_eq!(k.level(&Subject::Broker { town }), Level::Known);
    assert!(
        k.entries
            .values()
            .flat_map(|x| &x.learned)
            .all(|l| l.source == Source::Migrated)
    );
    for _ in 0..10 {
        e2.apply(Command::EndTurn).unwrap();
    }
    let (bytes, meta) = encode(e2.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e2.state_hash());
    // A seeded ledger is kept as it is on the next load.
    let before = back.player.as_ref().unwrap().knowledge.clone();
    let e3 = mandate_sim::Engine::from_state(back, content()).unwrap();
    assert_eq!(e3.state().player.as_ref().unwrap().knowledge, before);
}

/// A save file in an older format: its header and the given payload.
fn old_file(st: &mandate_sim::State, v: &serde_json::Value, version: u16) -> Vec<u8> {
    let payload = serde_json::to_vec(v).unwrap();
    let (_, mut meta) = encode(st, &req()).unwrap();
    meta.format_version = version;
    meta.payload_hash = mandate_core::hash::StableHash::of_bytes(&payload).hex();
    let meta_json = serde_json::to_vec(&meta).unwrap();
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.extend_from_slice(&(meta_json.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&meta_json);
    bytes.extend_from_slice(&miniz_oxide::deflate::compress_to_vec(&payload, 6));
    bytes
}

/// Strip per-neighbourhood hazards (DEC-090 D8, G1) from a payload: the
/// shape of the format before them.
fn without_hazards(v: &mut serde_json::Value) {
    let env = v
        .pointer_mut("/state/environment")
        .unwrap()
        .as_object_mut()
        .unwrap();
    env.remove("exposure");
    env.remove("hood_damage");
    let pop = v
        .pointer_mut("/state/population")
        .unwrap()
        .as_object_mut()
        .unwrap();
    pop.remove("hood_impact");
    pop.remove("displaced");
}

/// The format after v14 added per-neighbourhood hazard exposure (DEC-090
/// D8, G1). A save from before derives exposure from its towns' profiles and
/// its neighbourhoods' kinds, the same every time, keeps it through play,
/// and round-trips; damage, displacement and impact start empty.
#[test]
fn v15_saves_migrate_to_hazard_exposure() {
    let c = content();
    let mut e = generate(
        &WorldConfig {
            seed: 67,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    e.apply(Command::AdvanceDays { days: 500 }).unwrap();
    let mut v = serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap() });
    without_hazards(&mut v);
    assert!(
        serde_json::from_value::<mandate_sim::State>(v["state"].clone()).is_err(),
        "the older payload lacks exposure"
    );
    // (An older save holds no arc run; the step to v27 refuses one that does.)
    v["state"].as_object_mut().unwrap().remove("arcs");
    let migrated = mandate_save::migrate(15, v.clone()).unwrap();
    assert_eq!(migrated, mandate_save::migrate(15, v.clone()).unwrap());
    let state: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    let exposure = mandate_sim::environment::exposure::from_profile(&state.world);
    assert_eq!(state.environment.exposure, exposure);
    assert_eq!(exposure.len(), state.world.neighborhoods.len());
    assert!(state.environment.hood_damage.is_empty());
    assert!(state.population.displaced.is_empty() && state.population.hood_impact.is_empty());
    // A flood-prone town's waterfront is more exposed than its suburbs.
    for mu in state
        .world
        .municipalities
        .iter()
        .filter(|m| m.geo.flood > 0.2)
    {
        let of = |k: mandate_sim::world::NeighborhoodKind| {
            mu.neighborhoods
                .iter()
                .filter(|h| state.world.neighborhoods[h.index()].kind == k)
                .map(|h| exposure[h.index()].flood)
                .max()
        };
        if let (Some(w), Some(s)) = (
            of(mandate_sim::world::NeighborhoodKind::Waterfront),
            of(mandate_sim::world::NeighborhoodKind::Suburb),
        ) {
            assert!(w > s);
        }
    }
    // Play on: exposure never moves; the file is the current format.
    let mut e2 = mandate_sim::Engine::from_state(state, c).unwrap();
    for _ in 0..12 {
        e2.apply(Command::EndTurn).unwrap();
    }
    assert_eq!(e2.state().environment.exposure, exposure);
    let (bytes, meta) = encode(e2.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e2.state_hash());
    // And an old file decodes through the chain.
    let (old, read) = decode(&old_file(e.state(), &v, 15)).unwrap();
    assert_eq!(read.format_version, 15);
    assert_eq!(old.environment.exposure, exposure);
}

/// The format after v17 added neighbourhood concerns (N1). A v17 save's town
/// levels were measured from the town-wide indicators: they become the
/// measured levels, beside the salience they produced, so the town's
/// politics are unchanged. Each neighbourhood's levels and the felt town
/// levels that aggregate them are derived on load, the same every time; the
/// run plays on and round-trips.
#[test]
fn v17_saves_migrate_to_neighbourhood_concerns() {
    let c = content();
    let mut e = generate(
        &WorldConfig {
            seed: 68,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    e.apply(Command::AdvanceDays { days: 400 }).unwrap();
    let st = e.state();
    // The shape of a v17 payload: a town's levels are its measured ones; no
    // neighbourhood levels.
    let mut v = serde_json::json!({ "state": serde_json::to_value(st).unwrap() });
    {
        let pop = v
            .pointer_mut("/state/population")
            .unwrap()
            .as_object_mut()
            .unwrap();
        pop.remove("hood_concerns");
        for t in pop["town_concerns"].as_array_mut().unwrap() {
            let t = t.as_object_mut().unwrap();
            let measured = t.remove("measured").unwrap();
            t.insert("levels".into(), measured);
        }
    }
    assert!(
        serde_json::from_value::<mandate_sim::State>(v["state"].clone()).is_err(),
        "the older payload lacks neighbourhood concerns"
    );
    // (An older save holds no arc run; the step to v27 refuses one that does.)
    v["state"].as_object_mut().unwrap().remove("arcs");
    let migrated = mandate_save::migrate(17, v.clone()).unwrap();
    assert_eq!(migrated, mandate_save::migrate(17, v.clone()).unwrap());
    let state: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    assert!(state.population.hood_concerns.is_empty());
    for (t, o) in state
        .population
        .town_concerns
        .iter()
        .zip(&st.population.town_concerns)
    {
        assert_eq!(t.measured, o.measured);
        assert_eq!(t.salience, o.salience);
    }
    // On load: neighbourhood levels and the felt town levels are derived as
    // the monthly refresh derives them; measured levels and salience stay.
    let loaded = mandate_sim::Engine::from_state(state.clone(), c.clone()).unwrap();
    let again = mandate_sim::Engine::from_state(state, c.clone()).unwrap();
    assert_eq!(loaded.state_hash(), again.state_hash());
    let pop = &loaded.state().population;
    let (towns, hoods) = mandate_sim::concerns::compute_all(
        &mandate_sim::scheduler::Reads::of_state(loaded.state(), &c),
        pop,
    );
    assert_eq!(pop.hood_concerns, hoods);
    assert_eq!(
        pop.hood_concerns.len(),
        loaded.state().world.neighborhoods.len()
    );
    for ((t, n), o) in pop
        .town_concerns
        .iter()
        .zip(&towns)
        .zip(&st.population.town_concerns)
    {
        assert_eq!(t.levels, n.levels);
        assert_eq!(t.measured, o.measured);
        assert_eq!(t.salience, o.salience);
    }
    // Play on; the file is the current format and round-trips.
    let mut e2 = loaded;
    for _ in 0..6 {
        e2.apply(Command::EndTurn).unwrap();
    }
    let (bytes, meta) = encode(e2.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e2.state_hash());
    // And an old file decodes through the chain.
    let (old, read) = decode(&old_file(e.state(), &v, 17)).unwrap();
    assert_eq!(read.format_version, 17);
    assert!(old.population.hood_concerns.is_empty());
    let old = mandate_sim::Engine::from_state(old, c).unwrap();
    assert_eq!(old.state().population.hood_concerns, hoods);
}
