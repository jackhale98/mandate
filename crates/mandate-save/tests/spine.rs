//! The causal spine's save migration (S1, v25 → v26) on a real save from
//! the build before the spine: `v25_town.mndt` (seed 2026, a small world,
//! sixty idle turns, written by the branch head e7916f9).
//!
//! An old save loads, plays on deterministically, and its old events gain
//! explicit `unknown` causes ("recorded before the spine"), never invented
//! ones; what was recorded with a cause keeps it as a typed link; what
//! happens next links into the old log.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{bundle::read_dir_sources, load};
use mandate_save::*;
use mandate_sim::events::{EventId, EventKind, Retention};
use mandate_sim::spine::{Cause, Link, Thread, graph};
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

/// The old save as today's build loads it, marked as made with this
/// content (so that loading records no rules change).
fn old_state(c: &mandate_content::ContentBundle) -> mandate_sim::State {
    let (mut st, meta) = decode(&fixture("v25_town.mndt")).unwrap();
    assert_eq!(meta.format_version, 25);
    st.meta.content_hash = c.content_hash.clone();
    st
}

#[test]
fn old_events_gain_unknown_causes_and_keep_the_ones_they_had() {
    let bytes = fixture("v25_town.mndt");
    let (version, old) = raw(&bytes);
    assert_eq!(version, 25);
    let migrated = migrate(25, old.clone()).unwrap();
    assert_eq!(migrated, migrate(25, old.clone()).unwrap(), "deterministic");
    let old_events = old["state"]["events"]["events"].as_array().unwrap();
    let (st, _) = decode(&bytes).unwrap();
    let log = &st.events;
    assert_eq!(log.len(), old_events.len(), "the migration drops nothing");
    let (mut unknown, mut linked) = (0, 0);
    for (was, e) in old_events.iter().zip(log.iter()) {
        assert_eq!(was["id"].as_u64(), Some(e.id.0 as u64));
        let had: Vec<u32> = was["causes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_u64().unwrap() as u32)
            .collect();
        if had.is_empty() {
            // Not known, and not invented: not even "exogenous" for a
            // disaster or a recession.
            assert_eq!(e.causes, vec![Cause::Unknown], "{:?}", e.kind);
            unknown += 1;
            continue;
        }
        linked += 1;
        // What it had, as typed links (or `forgotten` where the old log had
        // already dropped the event).
        for id in had {
            let id = EventId(id);
            if log.get(id).is_some() {
                let link = match e.kind {
                    EventKind::PolicyFailed { .. } | EventKind::PolicyExpired { .. } => Link::Ended,
                    EventKind::PolicyEnacted { .. } => Link::Continues,
                    _ => Link::Caused,
                };
                assert!(e.causes.contains(&Cause::Event(id, link)), "{:?}", e.kind);
            } else {
                assert!(e.causes.contains(&Cause::Forgotten));
            }
        }
        assert!(!e.causes.contains(&Cause::Unknown));
    }
    assert!(
        unknown > 100 && linked > 100,
        "{unknown} unknown, {linked} linked"
    );
    // No link dangles.
    for e in log.iter() {
        for (c, _) in e.links() {
            assert!(c < e.id && log.get(c).is_some());
        }
    }
    // The threads' latest events are the ones replaying the log gives: the
    // migration's frozen thread table agrees with the engine's.
    assert!(!log.heads().is_empty());
    assert_eq!(log.heads(), log.replayed_heads().as_slice());
    assert!(log.heads().windows(2).all(|w| w[0].0 < w[1].0));
    // Laws are kept by the rule now, not by a pin.
    assert!(log.iter().all(|e| {
        !matches!(
            e.kind,
            EventKind::PolicyEnacted { .. } | EventKind::PolicyFailed { .. }
        ) || e.retention == Retention::Recent
    }));
    // The game's start is still a landmark.
    assert!(log.iter().any(|e| {
        matches!(e.kind, EventKind::GameStarted { .. }) && e.retention == Retention::Pinned
    }));
}

#[test]
fn an_old_save_plays_on_deterministically_and_what_follows_is_caused() {
    let c = content();
    let play = |turns: u32| {
        let mut e = Engine::from_state(old_state(&c), c.clone()).unwrap();
        for _ in 0..turns {
            e.apply(Command::EndTurn).unwrap();
        }
        e
    };
    let e = play(26);
    // The same save played on again reaches the same state; so does a run
    // saved and loaded on the way (in today's format).
    assert_eq!(play(26).state_hash(), e.state_hash());
    let half = play(13);
    let (bytes, meta) = encode(half.state(), &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (st, _) = decode(&bytes).unwrap();
    let mut resumed = Engine::from_state(st, c.clone()).unwrap();
    for _ in 0..13 {
        resumed.apply(Command::EndTurn).unwrap();
    }
    assert_eq!(resumed.state_hash(), e.state_hash());

    let before = old_state(&c);
    let last_old = before.events.iter().last().unwrap().id;
    let log = &e.state().events;
    let new: Vec<_> = log.iter().filter(|x| x.id > last_old).collect();
    assert!(new.len() > 50, "a year of events: {}", new.len());
    // Everything recorded since the load names a known cause. The one
    // honest gap: a hidden fact from before the spine coming to light (the
    // act that left it was never recorded), which says so beside the
    // director's timing.
    let old_hidden = before.narrative.next_hidden;
    for x in &new {
        assert!(x.causes.iter().any(Cause::is_known), "{:?}", x.kind);
        if !x.causes.iter().all(Cause::is_known) {
            assert!(
                matches!(x.kind, EventKind::FactDiscoverable { hidden } if hidden < old_hidden),
                "{:?}: {:?}",
                x.kind,
                x.causes
            );
        }
    }
    // The old events the rule still keeps are still unknown: nothing was
    // filled in afterwards.
    let still_unknown = log
        .iter()
        .filter(|x| x.id <= last_old && x.causes == vec![Cause::Unknown])
        .count();
    assert!(still_unknown > 50);
    // What happens next to matters from before the spine links into the
    // old log (a law that expires, an ask that is settled, a case that
    // moves on), and the chain behind it ends at its honest root.
    let across: Vec<_> = new
        .iter()
        .filter(|x| x.links().any(|(c, _)| c <= last_old))
        .collect();
    assert!(
        across.len() > 5,
        "{} events link into the old log",
        across.len()
    );
    for x in &across {
        let chain = graph::chain(log, x.id, 8);
        assert!(
            chain.roots.iter().any(|r| r.cause == Cause::Unknown),
            "{:?}",
            x.kind
        );
    }
    // A law from before the spine that expired since: linked `ended`, and
    // its thread's latest event.
    let expired = new.iter().find_map(|x| match &x.kind {
        EventKind::PolicyExpired { policy, .. } => Some((x, *policy)),
        _ => None,
    });
    if let Some((x, policy)) = expired {
        assert!(x.links().any(|(c, l)| c <= last_old && l == Link::Ended));
        assert_eq!(log.head(Thread::Bill(policy)), Some(x.id));
    }
    // The log prunes by the new rule, and stays whole.
    for x in log.iter() {
        for (c, _) in x.links() {
            assert!(log.get(c).is_some());
        }
    }
    assert!(log.len() <= mandate_sim::spine::prune::MAX_EVENTS);
}

/// The last post held (M6 note 16) is filled where the save's own election
/// results show it, and left empty otherwise.
#[test]
fn the_last_post_is_filled_where_the_save_shows_it() {
    let (st, _) = decode(&fixture("v25_town.mndt")).unwrap();
    let results = &st.politics.elections.results;
    let mut filled = 0;
    for p in &st.politics.politicians {
        let lost = results
            .iter()
            .rev()
            .find(|r| r.winner == p.id || r.incumbent == Some(p.id))
            .filter(|r| r.winner != p.id);
        match (p.last_post, p.in_office()) {
            // In office: nothing is claimed.
            (last, true) => assert_eq!(last, None),
            (Some(last), false) => {
                let r = lost.expect("a result shows the post lost");
                assert_eq!((last.post, last.ended), (r.seat, r.date));
                filled += 1;
            }
            // Out of office with no result to show for it: left empty.
            (None, false) => assert!(lost.is_none()),
        }
    }
    assert!(filled > 0, "the fixture's last election unseated someone");
    // From here on the engine records it as people leave.
    let c = content();
    let mut e = Engine::from_state(old_state(&c), c.clone()).unwrap();
    for _ in 0..4 {
        e.apply(Command::EndTurn).unwrap();
    }
    assert!(
        e.state()
            .politics
            .politicians
            .iter()
            .filter(|p| p.last_post.is_some())
            .count()
            >= filled
    );
}

#[test]
fn the_older_fixtures_migrate_through_the_spine_too() {
    for name in ["v17_campaign.mndt", "v18_campaign.mndt"] {
        let (st, _) = decode(&fixture(name)).unwrap();
        assert!(!st.events.is_empty());
        for e in st.events.iter() {
            assert!(!e.causes.is_empty(), "{name}: {:?}", e.kind);
            for (c, _) in e.links() {
                assert!(st.events.get(c).is_some(), "{name}");
            }
        }
        assert_eq!(st.events.heads(), st.events.replayed_heads().as_slice());
    }
}
