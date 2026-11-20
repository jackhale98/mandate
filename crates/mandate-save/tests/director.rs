//! The director's variety and pacing cycle (E2, DEC-097 §3): the v21 → v22
//! step, older files through the whole chain, and the cycle surviving a
//! save.
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

/// A game a year and a half into play, with decisions answered by default.
fn played(seed: u64) -> mandate_sim::Engine {
    let mut e = generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        content(),
    )
    .unwrap();
    for _ in 0..40 {
        e.apply(Command::EndTurn).unwrap();
    }
    e
}

#[test]
fn the_pacing_cycle_survives_a_save() {
    let mut e = played(5);
    let n = &e.state().narrative;
    assert!(n.phase.is_some() && !n.phase_log.is_empty());
    assert!(!n.uses.is_empty() && !n.answers.is_empty());
    assert!(n.chapter.as_ref().unwrap().theme.is_some());
    let (bytes, meta) = encode(e.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e.state_hash());
    assert_eq!(back.narrative, *n);
    let mut loaded = mandate_sim::Engine::from_state(back, e.content_arc()).unwrap();
    for _ in 0..6 {
        e.apply(Command::EndTurn).unwrap();
        loaded.apply(Command::EndTurn).unwrap();
    }
    assert_eq!(loaded.state_hash(), e.state_hash());
}

/// A v21 save has none of the new state: the step leaves the defaults (no
/// uses, no answers, no phase), the cycle begins at the next weekly step,
/// and the file is the current format.
#[test]
fn v21_saves_migrate_to_the_pacing_cycle() {
    let e = played(9);
    let mut v = serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap() });
    {
        let n = v["state"]["narrative"].as_object_mut().unwrap();
        for k in [
            "uses",
            "climaxes",
            "answers",
            "phase",
            "setback",
            "phase_log",
        ] {
            n.remove(k);
        }
        n["seen"].as_object_mut().unwrap().remove("filed");
        for c in n["chapters"].as_array_mut().unwrap() {
            let c = c.as_object_mut().unwrap();
            c.remove("theme");
            c.remove("theme_options");
            c.remove("peaks");
        }
        let c = n["chapter"].as_object_mut().unwrap();
        c.remove("theme");
        c.remove("theme_options");
        c.remove("peaks");
    }
    let migrated = mandate_save::migrate(21, v.clone()).unwrap();
    assert_eq!(migrated, mandate_save::migrate(21, v).unwrap());
    let state: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    let n = &state.narrative;
    assert!(n.uses.is_empty() && n.answers.is_empty() && n.phase.is_none());
    assert!(n.phase_log.is_empty() && n.climaxes.is_empty() && n.setback.is_none());
    assert!(n.chapters.iter().all(|c| c.theme.is_none() && c.peaks == 0));
    let mut e2 = mandate_sim::Engine::from_state(state, e.content_arc()).unwrap();
    for _ in 0..4 {
        e2.apply(Command::EndTurn).unwrap();
    }
    assert!(e2.state().narrative.phase.is_some(), "the cycle began");
    let (bytes, meta) = encode(e2.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e2.state_hash());
}

/// Older files (the v18 fixture) decode through the whole chain with the
/// director's new state empty.
#[test]
fn older_files_decode_with_an_empty_cycle() {
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v18_campaign.mndt"),
    )
    .unwrap();
    let (st, meta) = decode(&bytes).unwrap();
    assert_eq!(meta.format_version, 18);
    assert!(st.narrative.phase.is_none() && st.narrative.uses.is_empty());
}
