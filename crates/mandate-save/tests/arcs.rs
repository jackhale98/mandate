//! Staged arcs (A1, DEC-109): the v26 → v27 step on a real save from the
//! build before arcs (`v26_town.mndt`: seed 2026, a small world, sixty idle
//! turns, written by the branch head ee9aa67), and a run in progress
//! surviving a save (save, load, continue: the same state as playing on).
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{bundle::read_dir_sources, load};
use mandate_save::*;
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::arcs::beats::{self, BeatAction};
use mandate_sim::environment::{DeferredHazard, HazardKind};
use mandate_sim::talk::TalkTarget;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

/// The base pack with the flood arc alone: these are the flood's tests
/// (task A1). The folded storylines have their own (`storylines.rs`).
fn content() -> Arc<mandate_content::ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    let mut b = load(&src).unwrap().0;
    b.arcs.retain(|a| a.id == "arc.flood");
    Arc::new(b)
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

/// A flood strikes the player's town at the next monthly hazard step.
fn plant_flood(e: &mut Engine) {
    let town = mandate_sim::player::home(e.state()).unwrap().0;
    let max = e.content().director.chance.hazard_max_defer_days;
    e.setup(|st| {
        let drawn = mandate_core::SimDate(st.calendar.date.0.saturating_sub(max + 1));
        st.environment.deferred.push(DeferredHazard {
            municipality: town,
            kind: HazardKind::Flood,
            severity: 0.9,
            drawn,
            hold_event: None,
        });
    });
}

/// One turn of a player who takes every beat a run offers (the first
/// option of a decision), then ends the turn.
fn turn(e: &mut Engine) {
    for _ in 0..8 {
        let offers = beats::offers(e.state(), e.content());
        let mut acted = false;
        for o in offers {
            let cmd = match &o.action {
                BeatAction::Visit { verb, .. } => e.route(PlayerCommand::Act {
                    action: Action::Visit { verb: verb.clone() },
                    late_night: false,
                }),
                BeatAction::Decision { decision } => {
                    let st = e.state();
                    let d = st
                        .narrative
                        .decision(mandate_sim::narrative::DecisionId(*decision))
                        .unwrap();
                    let option = e.content().dilemma(&d.template).unwrap().options[0]
                        .id
                        .clone();
                    Command::Player(PlayerCommand::Answer {
                        decision: *decision,
                        option,
                    })
                }
                BeatAction::Ask { ask } => {
                    let st = e.state();
                    let a = st.asks.get(mandate_sim::asks::AskId(*ask)).unwrap();
                    if !a.offered() {
                        continue;
                    }
                    Command::Player(PlayerCommand::RespondAsk {
                        ask: *ask,
                        accept: true,
                    })
                }
                BeatAction::Talk { person, topic } => {
                    let Some(at) = mandate_sim::characters::routines::whereabouts(
                        e.state(),
                        e.content(),
                        *person,
                    ) else {
                        continue;
                    };
                    Command::Player(PlayerCommand::Go {
                        location: at,
                        then: Some(Box::new(PlayerCommand::Talk {
                            to: TalkTarget::Person { person: *person },
                            topic: topic.clone(),
                            about: None,
                            late_night: false,
                        })),
                    })
                }
            };
            if e.apply(cmd).is_ok() {
                acted = true;
                break;
            }
        }
        if !acted {
            break;
        }
    }
    e.apply(Command::EndTurn).unwrap();
}

#[test]
fn a_run_in_progress_survives_a_save() {
    let c = content();
    let mut e = generate(&WorldConfig::new(7, WorldSize::Small), c.clone()).unwrap();
    plant_flood(&mut e);
    for _ in 0..7 {
        turn(&mut e);
    }
    let st = e.state();
    let run = st.arcs.runs.first().expect("a run in progress");
    assert!(!run.done.is_empty() && !run.marks.is_empty(), "{run:?}");
    assert!(
        !st.environment.repairs.is_empty(),
        "the repairs the player pressed for are running"
    );
    // Save, load: the same state, arcs and all.
    let (bytes, meta) = encode(st, &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (loaded, _) = decode(&bytes).unwrap();
    assert_eq!(loaded.arcs, st.arcs);
    assert_eq!(loaded.environment.repairs, st.environment.repairs);
    assert_eq!(loaded.hash(), st.hash());
    // Continue both to the run's end: one state.
    let mut b = Engine::from_state(loaded, c.clone()).unwrap();
    for _ in 0..40 {
        turn(&mut e);
        turn(&mut b);
        assert_eq!(e.state_hash(), b.state_hash());
    }
    assert!(
        e.state().arcs.runs.is_empty() && e.state().arcs.past.len() == 1,
        "the run ended: {:?}",
        e.state().arcs
    );
    assert_eq!(e.state().arcs, b.state().arcs);
}

/// The old save as today's build loads it, marked as made with this
/// content (so that loading records no rules change).
fn old_state(c: &mandate_content::ContentBundle) -> mandate_sim::State {
    let (mut st, meta) = decode(&fixture("v26_town.mndt")).unwrap();
    assert_eq!(meta.format_version, 26);
    st.meta.content_hash = c.content_hash.clone();
    st
}

#[test]
fn a_v26_save_loads_and_continues() {
    let c = content();
    let bytes = fixture("v26_town.mndt");
    let (version, old) = raw(&bytes);
    assert_eq!(version, 26);
    // The steps make no run up: an older save holds no arc, and no
    // storyline was running in this one (the fold, v27 → v28, has its own
    // fixture: `tests/storylines.rs`).
    let migrated = migrate(26, old.clone()).unwrap();
    assert!(
        migrated["state"]
            .pointer("/arcs/runs")
            .and_then(|r| r.as_array())
            .is_none_or(|r| r.is_empty())
    );
    // (Industries adds to the economy alone; the last step renames a
    // ledger entry, `homes_built` to `homes_started`.)
    assert_eq!(migrated["state"]["events"], old["state"]["events"]);
    let st = old_state(&c);
    assert!(st.arcs.runs.is_empty() && st.arcs.past.is_empty());
    assert!(st.narrative.decisions.iter().all(|d| d.arc.is_none()));
    assert!(st.asks.asks.iter().all(|a| a.arc.is_none()));
    assert!(st.environment.repairs.is_empty());
    // It plays on, and the same way twice.
    let play = |turns: u32, flood: bool| {
        let mut e = Engine::from_state(old_state(&c), c.clone()).unwrap();
        if flood {
            plant_flood(&mut e);
        }
        for _ in 0..turns {
            e.apply(Command::EndTurn).unwrap();
        }
        e
    };
    let a = play(10, false);
    assert_eq!(a.state_hash(), play(10, false).state_hash());
    assert!(a.state().calendar.turn >= st.calendar.turn + 10);
    // A flood after loading raises its arc in the old world too, and the
    // game it is now is written in today's format and read back whole.
    let f = play(6, true);
    let run = f.state().arcs.runs.first().expect("the flood's arc");
    assert_eq!(run.template, "arc.flood");
    let (bytes, meta) = encode(f.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.arcs, f.state().arcs);
    assert_eq!(back.hash(), f.state().hash());
}

#[test]
fn an_older_payload_cannot_carry_arcs() {
    let (_, mut old) = raw(&fixture("v26_town.mndt"));
    old["state"]["arcs"] = serde_json::json!({ "runs": [] });
    assert!(migrate(26, old).is_err());
}
