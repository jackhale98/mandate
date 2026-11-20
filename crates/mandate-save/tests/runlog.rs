#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The run log beside the state (format v18, bug reports): it round-trips
//! through a save, a replay from it reproduces the run's hashes, and saves
//! from before it migrate with their state untouched.

use mandate_content::{bundle::read_dir_sources, load};
use mandate_save::runlog::{Checkpoint, replay};
use mandate_save::*;
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::places::Location;
use mandate_sim::state::WorldSize;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, generate};
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

fn setup() -> NewGameSetup {
    NewGameSetup {
        seed: 9_007_199_254_740_993, // above 2^53: kept exact as a string
        size: WorldSize::Small,
        scenario: None,
        storyteller: None,
        local_knowledge: true,
    }
}

fn new_game(c: &Arc<mandate_content::ContentBundle>, s: &NewGameSetup) -> Engine {
    let mut e = generate(
        &WorldConfig {
            seed: s.seed,
            size: s.size,
            scenario: s.scenario.clone(),
        },
        c.clone(),
    )
    .unwrap();
    s.prepare(&mut e).unwrap();
    e
}

/// A few turns with player commands (some refused, which are not logged).
fn play(e: &mut Engine, log: &mut RunLog, turns: u32) {
    for t in 0..turns {
        let _ = e.apply(Command::Player(PlayerCommand::Act {
            action: if t % 2 == 0 {
                Action::Fundraise
            } else {
                Action::Rest
            },
            late_night: false,
        }));
        let _ = e.apply(Command::Player(PlayerCommand::MoveTo {
            location: Location::Home,
        }));
        e.apply(Command::EndTurn).unwrap();
        log.checkpoint(e);
    }
}

#[test]
fn the_log_round_trips_through_a_save_and_replays_to_its_hashes() {
    let c = content();
    let s = setup();
    let mut e = new_game(&c, &s);
    let mut log = RunLog::start(s.clone(), &e);
    play(&mut e, &mut log, 3);
    let log = log.with_commands(&e);
    assert!(
        log.commands
            .iter()
            .any(|c| matches!(c.command, Command::Player(_))),
        "player commands were logged"
    );
    assert_eq!(log.checkpoints.len(), 4);

    let (bytes, meta) = encode_with_log(e.state(), Some(&log), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (state, back, _) = decode_with_log(&bytes).unwrap();
    assert_eq!(state.hash(), e.state_hash());
    assert_eq!(back.as_ref(), Some(&log));
    // The seed survives JSON readers that lose integers above 2^53.
    let json = serde_json::to_value(&log).unwrap();
    assert_eq!(json["new_game"]["seed"], "9007199254740993");
    assert_eq!(json["commands"][0]["command"]["type"], "player");

    // A save made without a log decodes with none.
    let (plain, _) = encode(e.state(), &req()).unwrap();
    assert_eq!(decode_with_log(&plain).unwrap().1, None);

    // A replay from the start matches every checkpoint and the end.
    let mut r = new_game(&c, &s);
    let out = replay(&mut r, &log, None);
    assert_eq!(out.divergence, None);
    assert_eq!(out.checkpoints.len(), 4);
    assert!(out.checkpoints.iter().all(|c| c.matches()));
    assert_eq!(out.state_hash, e.state_hash());
    assert!(!out.gaps);

    // A save loaded mid-run carries its log on: the next command follows it.
    let mut loaded = Engine::from_state(state, c.clone()).unwrap();
    let n = log.commands.len() as u64;
    loaded.restore_log(log.commands.clone());
    loaded.apply(Command::EndTurn).unwrap();
    assert_eq!(loaded.command_log().last().map(|c| c.index), Some(n));
}

#[test]
fn a_replay_names_the_turn_where_a_changed_command_diverges() {
    let c = content();
    let s = setup();
    let mut e = new_game(&c, &s);
    let mut log = RunLog::start(s.clone(), &e);
    play(&mut e, &mut log, 3);
    let mut log = log.with_commands(&e);
    // The first command of turn 2 (between checkpoints 2 and 3).
    let Checkpoint { commands: from, .. } = log.checkpoints[2].clone();
    log.commands[from as usize].command = Command::AdvanceDays { days: 3 };
    let mut r = new_game(&c, &s);
    let out = replay(&mut r, &log, None);
    let d = out.divergence.expect("diverges");
    assert_eq!(d.at.turn, 3);
    assert_eq!(d.last_good_turn, Some(2));
    assert_eq!(d.commands_from, from);
    assert_eq!(d.commands_to, log.checkpoints[3].commands);
    // The last turn: nothing was left to replay after it.
    assert!(!out.stopped_early);

    // `until` stops before the first command of that turn.
    let mut r = new_game(&c, &s);
    let out = replay(&mut r, &log, Some(2));
    assert_eq!(out.divergence, None);
    assert_eq!(out.turn, 2);
    assert_eq!(out.commands_applied, from);
}

/// Format v17 (before the run log): the file as v17 wrote it, payload `{state}`.
fn v17_file(state: &mandate_sim::State) -> Vec<u8> {
    let (_, meta) = encode(state, &req()).unwrap();
    let payload = serde_json::to_vec(&serde_json::json!({ "state": state })).unwrap();
    let mut meta = serde_json::to_value(meta).unwrap();
    meta["format_version"] = 17.into();
    meta["payload_hash"] = mandate_core::hash::StableHash::of_bytes(&payload)
        .hex()
        .into();
    let meta = serde_json::to_vec(&meta).unwrap();
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&17u16.to_le_bytes());
    out.extend_from_slice(&(meta.len() as u32).to_le_bytes());
    out.extend_from_slice(&meta);
    out.extend_from_slice(&miniz_oxide::deflate::compress_to_vec(&payload, 6));
    out
}

#[test]
fn v17_saves_migrate_without_a_log_and_keep_their_hash() {
    let c = content();
    let s = setup();
    let mut e = new_game(&c, &s);
    let mut log = RunLog::start(s, &e);
    play(&mut e, &mut log, 2);
    let bytes = v17_file(e.state());
    assert_eq!(read_meta(&bytes).unwrap().format_version, 17);
    let (state, log, _) = decode_with_log(&bytes).unwrap();
    assert_eq!(log, None);
    assert_eq!(state.hash(), e.state_hash());
    // The step itself only adds the empty log.
    let v = migrate(17, serde_json::json!({ "state": {} })).unwrap();
    assert_eq!(v, serde_json::json!({ "state": {}, "log": null }));
}
