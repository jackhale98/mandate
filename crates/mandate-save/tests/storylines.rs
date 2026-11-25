//! Storylines folded into arcs (A2, DEC-109): the v27 → v28 step on a real
//! save from the build before the fold (`v27_storylines.mndt`: seed 1, a
//! small world, twenty turns answering every decision but the storylines'
//! own, written by the branch head 2ac45b9). In it the favour-debt
//! storyline has reached its reminder, and the rival storyline's offer is
//! waiting on the player as decision 3.
//!
//! A running storyline loads at the stage of the same id of its arc and
//! plays on; the older fixtures (v25, v26) load through the new version.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{bundle::read_dir_sources, load};
use mandate_save::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::arcs::{ArcActor, BeatRef};
use mandate_sim::narrative::{DecisionId, DecisionStatus};
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

/// An old save as today's build loads it, marked as made with this content
/// (so that loading records no rules change).
fn old_state(name: &str, c: &mandate_content::ContentBundle) -> mandate_sim::State {
    let (mut st, _) = decode(&fixture(name)).unwrap();
    st.meta.content_hash = c.content_hash.clone();
    st
}

#[test]
fn the_step_folds_running_storylines_into_arc_runs() {
    let (version, old) = raw(&fixture("v27_storylines.mndt"));
    assert_eq!(version, 27);
    let n = &old["state"]["narrative"];
    assert_eq!(n["storylines"].as_array().unwrap().len(), 2);
    assert_eq!(n["decisions"][3]["storyline"], 1);
    assert!(old["state"].get("arcs").is_none());
    let new = migrate(27, old.clone()).unwrap();
    let n = &new["state"]["narrative"];
    for gone in ["storylines", "run_ids", "follow_ups"] {
        assert!(n.get(gone).is_none(), "{gone} is gone");
    }
    assert!(
        n["decisions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d.get("storyline").is_none())
    );
    assert!(
        n["facts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| { !f["kind"]["type"].as_str().unwrap().starts_with("storyline") })
    );
    assert!(n["phase"].get("climax_fallback").is_none());
    // Uses carry to the arcs; no storyline key is left anywhere in the
    // narrative but the name of a reason's arc.
    let uses = serde_json::to_string(&n["uses"]).unwrap();
    assert!(uses.contains("arc.rival") && uses.contains("arc.favour_debt"));
    assert!(!serde_json::to_string(n).unwrap().contains("storyline."));
    // The two runs, at the stages of the same ids.
    let runs = new["state"]["arcs"]["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(
        (runs[0]["template"].as_str(), runs[0]["stage"].as_str()),
        (Some("arc.favour_debt"), Some("reminder"))
    );
    assert_eq!(
        (runs[1]["template"].as_str(), runs[1]["stage"].as_str()),
        (Some("arc.rival"), Some("offer"))
    );
    assert_eq!(new["state"]["arcs"]["ids"]["next"], 2);
    // The offer still waits on the player, now as the run's beat.
    assert_eq!(
        n["decisions"][3]["arc"],
        serde_json::json!({ "run": 1, "beat": { "stage": "offer", "beat": "terms" } })
    );
    assert_eq!(
        runs[1]["open"],
        serde_json::json!([{ "stage": "offer", "beat": "terms" }])
    );
    // The log's storyline events are kept as they were.
    let count = |v: &Value| {
        v["state"]["events"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"]["type"] == "storyline_stage")
            .count()
    };
    assert_eq!((count(&old), count(&new)), (3, 3));
}

#[test]
fn a_save_with_running_storylines_loads_at_the_matching_stages_and_plays_on() {
    let c = content();
    let (st, meta) = decode(&fixture("v27_storylines.mndt")).unwrap();
    assert_eq!(meta.format_version, 27);
    let home = mandate_sim::world::Jurisdiction::town(mandate_sim::player::home(&st).unwrap());
    let favour = &st.arcs.runs[0];
    let rival = &st.arcs.runs[1];
    assert_eq!(
        (favour.template.as_str(), favour.stage.as_str(), favour.at),
        ("arc.favour_debt", "reminder", home)
    );
    assert_eq!(
        (rival.template.as_str(), rival.stage.as_str(), rival.at),
        ("arc.rival", "offer", home)
    );
    // The casts are the storylines': the same people, by the same roles.
    assert!(matches!(
        favour.actor("creditor"),
        Some(ArcActor::Person { id }) if id.0 == 1286
    ));
    assert!(matches!(
        rival.actor("rival"),
        Some(ArcActor::Person { id }) if id.0 == 1773
    ));
    assert_eq!(rival.open, vec![BeatRef::new("offer", "terms")]);
    let d = st.narrative.decision(DecisionId(3)).unwrap();
    assert_eq!(d.status, DecisionStatus::Pending);
    assert_eq!(d.arc.as_ref().map(|a| a.run), Some(rival.id));
    // Both templates and stages exist in today's pack: nothing is orphaned.
    for run in &st.arcs.runs {
        assert!(
            c.arc(&run.template)
                .and_then(|a| a.stage(&run.stage))
                .is_some(),
            "{run:?}"
        );
    }
    // The loaded game is grounded: the decision is its run's beat.
    assert!(
        mandate_sim::narrative::audit::decision(&st, &c, d).is_empty(),
        "{:?}",
        mandate_sim::narrative::audit::decision(&st, &c, d)
    );

    // It plays on. The player takes the truce: the run reads the answer,
    // sets its mark and moves to the stage the storyline's branch named.
    let mut e = Engine::from_state(old_state("v27_storylines.mndt", &c), c.clone()).unwrap();
    e.apply(Command::Player(PlayerCommand::Answer {
        decision: 3,
        option: "truce".into(),
    }))
    .unwrap();
    e.apply(Command::EndTurn).unwrap();
    e.apply(Command::EndTurn).unwrap();
    let rival = e
        .state()
        .arcs
        .runs
        .iter()
        .find(|r| r.template == "arc.rival")
        .expect("the rival's run");
    assert!(rival.has_mark("pact"), "{rival:?}");
    assert!(
        ["pact", "showdown"].contains(&rival.stage.as_str()),
        "{rival:?}"
    );
    // To the end of the campaign and past it: the rival's arc ends at an
    // exit the race decided, and the debt goes on under its own rules.
    for _ in 0..12 {
        e.apply(Command::EndTurn).unwrap();
    }
    let st = e.state();
    let ended = st
        .arcs
        .past
        .iter()
        .find(|p| p.template == "arc.rival")
        .expect("the rival's arc ended");
    assert!(["won", "lost"].contains(&ended.exit.as_str()), "{ended:?}");
    assert_eq!(
        ended.turn.as_ref().map(|t| t.stage.as_str()),
        Some("showdown")
    );
    assert!(
        st.arcs
            .runs
            .iter()
            .chain(std::iter::empty())
            .any(|r| r.template == "arc.favour_debt")
            || st.arcs.past.iter().any(|p| p.template == "arc.favour_debt"),
        "the debt is still told"
    );

    // The same way twice, and the game it is now is written as v28 and
    // read back whole.
    let play = |turns: u32| {
        let mut e = Engine::from_state(old_state("v27_storylines.mndt", &c), c.clone()).unwrap();
        for _ in 0..turns {
            e.apply(Command::EndTurn).unwrap();
        }
        e
    };
    let a = play(8);
    assert_eq!(a.state_hash(), play(8).state_hash());
    let (bytes, meta) = encode(a.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.arcs, a.state().arcs);
    assert_eq!(back.hash(), a.state().hash());
}

/// A passive player lets the offer pass: the decision takes its default,
/// and the run goes on to the showdown with no pact.
#[test]
fn a_folded_storylines_decision_can_be_let_pass() {
    let c = content();
    let mut e = Engine::from_state(old_state("v27_storylines.mndt", &c), c.clone()).unwrap();
    for _ in 0..5 {
        e.apply(Command::EndTurn).unwrap();
    }
    let st = e.state();
    let d = st.narrative.decision(DecisionId(3)).unwrap();
    assert_eq!(d.status, DecisionStatus::Defaulted);
    let rival = st
        .arcs
        .runs
        .iter()
        .find(|r| r.template == "arc.rival")
        .expect("the rival's run");
    assert!(!rival.has_mark("pact"), "{rival:?}");
    assert!(rival.is_done(&BeatRef::new("offer", "terms")), "{rival:?}");
}

/// The older fixtures load through the new version and play on.
#[test]
fn older_saves_load_through_the_fold() {
    let c = content();
    for (name, version) in [("v25_town.mndt", 25u16), ("v26_town.mndt", 26)] {
        let bytes = fixture(name);
        let (v, old) = raw(&bytes);
        assert_eq!(v, version, "{name}");
        // No storyline was running in either, so no run is made up; the
        // narrative still loses its storyline state.
        let running = old["state"]["narrative"]["storylines"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["ended"].is_null())
            .count();
        let new = migrate(version, old).unwrap();
        assert!(new["state"]["narrative"].get("storylines").is_none());
        let runs = new["state"]
            .get("arcs")
            .and_then(|a| a.get("runs"))
            .and_then(|r| r.as_array())
            .map_or(0, |r| r.len());
        assert_eq!(runs, running, "{name}");
        let mut e = Engine::from_state(old_state(name, &c), c.clone()).unwrap();
        let turn = e.state().calendar.turn;
        for _ in 0..6 {
            e.apply(Command::EndTurn).unwrap();
        }
        assert!(e.state().calendar.turn >= turn + 6, "{name}");
        let (bytes, meta) = encode(e.state(), &req()).unwrap();
        assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
        let (back, _) = decode(&bytes).unwrap();
        assert_eq!(back.hash(), e.state().hash(), "{name}");
    }
}
