//! The impact estimate kept with a law (DEC-111) and its save step, on a
//! real save from the build before it: `v27_town.mndt` (seed 2026, a small
//! world, sixty idle turns, written by 1ca677c with `mandate-cli run --seed
//! 2026 --size small --turns 60 --save`).
//!
//! An older law has no frozen estimate and none is invented; a law passed
//! after loading carries one, and it survives a save. The estimate itself
//! sees no future: it is unchanged when the seed, the streams, the held
//! hazards and the hidden facts change.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{bundle::read_dir_sources, load};
use mandate_save::*;
use mandate_sim::policy::impact;
use mandate_sim::policy::{PolicyStatus, Resolved};
use mandate_sim::world::Jurisdiction;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
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

/// The old save as today's build loads it, marked as made with this
/// content (so that loading records no rules change).
fn old_state(c: &mandate_content::ContentBundle) -> mandate_sim::State {
    let (mut st, _) = decode(&fixture("v27_town.mndt")).unwrap();
    st.meta.content_hash = c.content_hash.clone();
    st
}

#[test]
fn an_older_save_loads_with_no_estimate_and_new_laws_keep_theirs() {
    let c = content();
    let bytes = fixture("v27_town.mndt");
    let (version, old) = raw(&bytes);
    assert!(version < SAVE_FORMAT_VERSION);
    // The step rewrites nothing in a save whose ledgers already tell
    // started from added, and invents no estimate. (The steps between,
    // the fold of storylines and industries, write elsewhere: the laws
    // are as they were.)
    let migrated = migrate(version, old.clone()).unwrap();
    // A ledger from before started was told from added is reworded (the
    // next test); nothing else about a law changes, and none gains an
    // estimate.
    let laws = |v: &serde_json::Value| {
        let mut p = v["state"]["policy"].clone();
        for law in p["instances"].as_array_mut().unwrap() {
            law.as_object_mut().unwrap().remove("ledger");
        }
        p
    };
    assert_eq!(laws(&migrated), laws(&old));
    assert!(
        migrated["state"]["policy"]["instances"]
            .as_array()
            .unwrap()
            .iter()
            .all(|law| law.get("estimate").is_none())
    );
    let st = old_state(&c);
    let laws = st
        .policy
        .instances
        .iter()
        .filter(|p| p.status == PolicyStatus::InEffect)
        .count();
    assert!(laws > 20, "the old save has laws in effect: {laws}");
    assert!(st.policy.instances.iter().all(|p| p.estimate.is_none()));
    // It plays on, the same way twice; a law that passes now keeps what
    // its bill said, dated the day it passed.
    let play = |turns: u32| {
        let mut e = Engine::from_state(old_state(&c), c.clone()).unwrap();
        for _ in 0..turns {
            e.apply(Command::EndTurn).unwrap();
        }
        e
    };
    let known = st.policy.instances.len();
    let e = play(80);
    assert_eq!(e.state_hash(), play(80).state_hash());
    let passed: Vec<_> = e
        .state()
        .policy
        .instances
        .iter()
        .filter(|p| p.enacted_on.is_some_and(|d| d > st.calendar.date))
        .collect();
    assert!(!passed.is_empty(), "a bill passed within eighty turns");
    for p in &passed {
        let est = p
            .estimate
            .as_ref()
            .expect("a law passed now keeps its estimate");
        assert_eq!(Some(est.on), p.enacted_on);
        assert!(!est.outcomes.is_empty(), "{}", p.template);
        // Every direct count is filed under a ledger key.
        assert!(est.direct.keys().all(|k| !k.is_empty()));
    }
    // Laws from before the save still have none.
    assert!(
        e.state().policy.instances[..known]
            .iter()
            .filter(|p| p.enacted_on.is_none_or(|d| d <= st.calendar.date))
            .all(|p| p.estimate.is_none())
    );
    // Written in today's format and read back whole.
    let (bytes, meta) = encode(e.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&bytes).unwrap();
    assert_eq!(back.policy, e.state().policy);
    assert_eq!(back.hash(), e.state().hash());
}

#[test]
fn an_older_ledger_reads_in_todays_words() {
    let (version, mut old) = raw(&fixture("v27_town.mndt"));
    let laws = old["state"]["policy"]["instances"].as_array_mut().unwrap();
    // As saves from before the ledger told started from added had it.
    laws[0]["ledger"] = serde_json::json!({ "homes_built": 120.5 });
    laws[1]["ledger"] = serde_json::json!({ "homes_built": 20.0, "homes_started": 3.0 });
    let new = migrate(version, old).unwrap();
    let laws = &new["state"]["policy"]["instances"];
    assert_eq!(
        laws[0]["ledger"],
        serde_json::json!({ "homes_started": 120.5 })
    );
    assert_eq!(
        laws[1]["ledger"],
        serde_json::json!({ "homes_started": 23.0 })
    );
}

/// Every template's estimate in the player's home town, as frozen bytes.
fn estimates(e: &Engine) -> Vec<(String, impact::Estimate)> {
    let st = e.state();
    let c = e.content();
    let home = mandate_sim::player::home(st).unwrap();
    let r = mandate_sim::scheduler::Reads::of_state(st, c);
    let x = impact::inputs(&r, &st.policy, Jurisdiction::town(home)).expect("a local estimate");
    c.policies
        .values()
        .map(|t| {
            let probe = mandate_sim::policy::probe(t, home.0, &Default::default());
            (
                t.id.to_string(),
                impact::estimate(&x, c, &Resolved::plain(t), &probe),
            )
        })
        .collect()
}

#[test]
fn the_estimate_sees_no_future() {
    let c = content();
    let mut e = generate(
        &WorldConfig {
            seed: 3,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    e.apply(Command::AdvanceDays { days: 200 }).unwrap();
    let before = estimates(&e);
    assert_eq!(before.len(), c.policies.len());
    // Every template says something: a cost or a count, and outcomes.
    for (id, est) in &before {
        assert!(!est.outcomes.is_empty(), "{id}");
        assert!(est.cost.yearly > 0.0 || !est.direct.is_empty(), "{id}");
    }
    // Another seed, other streams, a flood held for next month, and every
    // secret gone or dated differently: none of it is read.
    let home = mandate_sim::player::home(e.state()).unwrap().0;
    e.setup(|st| {
        st.meta.seed ^= 0x5EED_5EED;
        st.rng = mandate_core::RngStreams::new(st.meta.seed);
        st.environment
            .deferred
            .push(mandate_sim::environment::DeferredHazard {
                municipality: home,
                kind: mandate_sim::environment::HazardKind::Flood,
                severity: 0.9,
                drawn: st.calendar.date,
                hold_event: None,
            });
        for h in st.narrative.hidden.iter_mut() {
            h.discoverable_on = Some(st.calendar.date);
            h.surfaced = !h.surfaced;
        }
        st.narrative.hidden.truncate(1);
    });
    let after = estimates(&e);
    assert_eq!(before, after);
    let frozen = |v: &[(String, impact::Estimate)]| {
        serde_json::to_vec(&v.iter().map(|(k, e)| (k, e.freeze())).collect::<Vec<_>>()).unwrap()
    };
    assert_eq!(frozen(&before), frozen(&after));
}
