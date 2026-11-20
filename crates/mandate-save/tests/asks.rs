//! Asks and promises (E6, DEC-097 §2): the v23 → v24 step, older files
//! through the whole chain, and asks, promises and the deed log surviving a
//! save (save, load, continue: the same state as playing on).
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{bundle::read_dir_sources, load};
use mandate_save::*;
use mandate_sim::Command;
use mandate_sim::actions::{Action, PlayerCommand};
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

/// One turn of a player who accepts every offer and canvasses their ward.
fn turn(e: &mut mandate_sim::Engine) {
    let offers: Vec<u32> = e
        .state()
        .asks
        .asks
        .iter()
        .filter(|a| a.offered())
        .map(|a| a.id.0)
        .collect();
    for ask in offers {
        e.apply(Command::Player(PlayerCommand::RespondAsk { ask, accept: true }))
            .unwrap();
    }
    let hood = {
        let st = e.state();
        let pl = st.player.as_ref().unwrap();
        mandate_sim::policy::ward_hoods(&st.world, pl.ward)[0] as u32
    };
    let c = e.route(PlayerCommand::Act {
        action: Action::Canvass {
            neighborhood: hood,
            issue: None,
        },
        late_night: false,
    });
    let _ = e.apply(c);
    e.apply(Command::EndTurn).unwrap();
}

fn played(seed: u64, turns: usize) -> mandate_sim::Engine {
    let mut e = generate(&WorldConfig::new(seed, WorldSize::Small), content()).unwrap();
    for _ in 0..turns {
        turn(&mut e);
    }
    e
}

#[test]
fn asks_survive_a_save() {
    let mut e = played(5, 40);
    let a = e.state().asks.clone();
    assert!(a.tally.offered > 0 && !a.deeds.is_empty(), "{:?}", a.tally);
    let (bytes, meta) = encode(e.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e.state_hash());
    assert_eq!(back.asks, a);
    let mut loaded = mandate_sim::Engine::from_state(back, e.content_arc()).unwrap();
    for _ in 0..8 {
        turn(&mut e);
        turn(&mut loaded);
    }
    assert_eq!(loaded.state_hash(), e.state_hash());
}

/// A v23 save has no asks: the step leaves the default (nothing asked, an
/// empty deed log), offers begin at the sliding rate, and the file is the
/// current format.
#[test]
fn v23_saves_migrate_to_asks() {
    let e = played(9, 10);
    let mut v = serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap() });
    v["state"].as_object_mut().unwrap().remove("asks");
    let migrated = mandate_save::migrate(23, v.clone()).unwrap();
    assert_eq!(migrated, mandate_save::migrate(23, v).unwrap());
    let state: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    assert_eq!(state.asks, mandate_sim::asks::AskState::default());
    let mut e2 = mandate_sim::Engine::from_state(state, e.content_arc()).unwrap();
    for _ in 0..30 {
        turn(&mut e2);
    }
    assert!(e2.state().asks.tally.offered > 0, "offers began");
    let (bytes, meta) = encode(e2.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.hash(), e2.state_hash());
}

/// Older files (the v18 fixture) decode through the whole chain with no asks.
#[test]
fn older_files_decode_with_no_asks() {
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v18_campaign.mndt"),
    )
    .unwrap();
    let (state, _) = decode(&bytes).unwrap();
    assert_eq!(state.asks, mandate_sim::asks::AskState::default());
}
