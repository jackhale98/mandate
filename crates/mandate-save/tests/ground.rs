//! The ground game's memory (E1, DEC-097 §1): the v20 → v21 step, older
//! files through the whole chain, and the memory surviving a save.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{bundle::read_dir_sources, load};
use mandate_save::*;
use mandate_sim::Command;
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::world::NeighborhoodId;
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

/// A game in which the player has canvassed two ward neighbourhoods.
fn canvassed(seed: u64) -> (mandate_sim::Engine, Vec<u32>) {
    let mut e = generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        content(),
    )
    .unwrap();
    let ward = {
        let st = e.state();
        mandate_sim::policy::ward_hoods(&st.world, st.player.as_ref().unwrap().ward)
    };
    let mut done = Vec::new();
    for h in ward.iter().take(2) {
        let cmd = e.route(PlayerCommand::Act {
            action: Action::Canvass {
                neighborhood: *h as u32,
                issue: None,
            },
            late_night: false,
        });
        e.apply(cmd).unwrap();
        done.push(*h as u32);
        e.apply(Command::EndTurn).unwrap();
    }
    (e, done)
}

#[test]
fn the_canvass_memory_survives_a_save() {
    let (mut e, done) = canvassed(5);
    let g = &e.state().player.as_ref().unwrap().ground;
    assert_eq!(
        g.hoods.iter().map(|h| h.hood.0).collect::<Vec<_>>(),
        {
            let mut d = done.clone();
            d.sort();
            d
        },
        "each canvassed neighbourhood is remembered"
    );
    let (bytes, meta) = encode(e.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e.state_hash());
    assert_eq!(back.player.as_ref().unwrap().ground, *g);
    // Continuing from the load matches continuing in place.
    let mut loaded = mandate_sim::Engine::from_state(back, e.content_arc()).unwrap();
    for _ in 0..4 {
        e.apply(Command::EndTurn).unwrap();
        loaded.apply(Command::EndTurn).unwrap();
    }
    assert_eq!(loaded.state_hash(), e.state_hash());
}

/// A v20 save has no canvass memory: the step leaves the default (every
/// neighbourhood fresh), the run plays on, and the file is the current
/// format.
#[test]
fn v20_saves_migrate_to_the_ground_game() {
    let (e, done) = canvassed(9);
    let mut v = serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap() });
    v["state"]["player"]
        .as_object_mut()
        .unwrap()
        .remove("ground");
    let migrated = mandate_save::migrate(20, v.clone()).unwrap();
    assert_eq!(migrated, mandate_save::migrate(20, v).unwrap());
    let state: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    let pl = state.player.as_ref().unwrap();
    assert!(pl.ground.hoods.is_empty() && pl.ground.last_turn.is_none());
    let fx = &e.content().actions.canvass;
    let c = mandate_sim::player::ground::canvass_curve(&state, fx, NeighborhoodId(done[0]));
    assert_eq!(c.fresh, 1.0, "a neighbourhood with no memory is fresh");
    let mut e2 = mandate_sim::Engine::from_state(state, e.content_arc()).unwrap();
    for _ in 0..4 {
        e2.apply(Command::EndTurn).unwrap();
    }
    let (bytes, meta) = encode(e2.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e2.state_hash());
}

/// Older files (the v18 fixture) decode through the whole chain with an
/// empty canvass memory.
#[test]
fn older_files_decode_with_an_empty_canvass_memory() {
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v18_campaign.mndt"),
    )
    .unwrap();
    let (st, meta) = decode(&bytes).unwrap();
    assert_eq!(meta.format_version, 18);
    let pl = st.player.as_ref().unwrap();
    assert!(pl.ground.hoods.is_empty());
}
