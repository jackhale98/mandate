//! Offices from the constitution (DEC-098, R1): the save migration on real
//! saves from the previous formats, identical continuation, and a
//! constitution whose bodies and offices have other names playing an
//! election and a bill with a veto and an override.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{Source, bundle::read_dir_sources, load};
use mandate_save::*;
use mandate_sim::Command;
use mandate_sim::politics::{Post, Role};
use mandate_sim::world::Jurisdiction;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

fn sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn content() -> Arc<mandate_content::ContentBundle> {
    Arc::new(load(&sources()).unwrap().0)
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
        run_mode: RunMode::Ironman,
    }
}

/// Every role and seat in a migrated payload has the post shape, and the
/// old storage is gone.
fn assert_post_shapes(v: &Value) {
    let pol = &v["state"]["politics"];
    assert!(pol.get("councils").is_none());
    for p in pol["politicians"].as_array().unwrap() {
        let t = p["role"]["type"].as_str().unwrap();
        assert!(["seat", "office", "private"].contains(&t), "{p}");
    }
    for r in pol["elections"]["races"]
        .as_array()
        .unwrap()
        .iter()
        .chain(pol["elections"]["results"].as_array().unwrap())
    {
        assert!(r["seat"]["type"].is_string(), "{}", r["seat"]);
    }
    let text = serde_json::to_string(v).unwrap();
    for old in ["\"Councillor\"", "\"Mayor\"", "\"Council\"", "\"councils\""] {
        assert!(!text.contains(old), "{old} left in the migrated payload");
    }
}

/// The R1 step (v19 → v20) from a v18 save, path by path (audit §7.7): roles, the councils,
/// races' and results' seats, memories' seats and the filing.
#[test]
fn v18_paths_migrate_to_posts() {
    let (version, mut v) = raw(&fixture("v18_campaign.mndt"));
    assert_eq!(version, 18);
    let old = v.clone();
    // A filing of each old kind: the player's ward, the mayoralty, none.
    let pl = &old["state"]["player"];
    let (m, ward) = (pl["municipality"].clone(), pl["ward"].clone());
    let town = serde_json::json!({ "type": "municipality", "id": m });
    let cases = [
        (
            Value::from("Council"),
            serde_json::json!({ "type": "stand", "post": { "type": "seat", "body": "council", "at": town, "constituency": ward } }),
        ),
        (
            Value::from("Mayor"),
            serde_json::json!({ "type": "stand", "post": { "type": "office", "office": "mayor", "at": town } }),
        ),
        (
            Value::from("Retire"),
            serde_json::json!({ "type": "retire" }),
        ),
        (Value::Null, Value::Null),
    ];
    for (filing, want) in cases {
        v["state"]["player"]["filing"] = filing;
        let migrated = migrate(18, v.clone()).unwrap();
        assert_eq!(migrated["state"]["player"]["filing"], want);
        assert_post_shapes(&migrated);
        let st: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
        assert!(st.player.is_some());
    }
    let migrated = migrate(18, v.clone()).unwrap();
    let pol = &migrated["state"]["politics"];
    let councils = old["state"]["politics"]["councils"].as_array().unwrap();
    // One council and one mayoralty per town, members and holders kept.
    let bodies = pol["bodies"].as_array().unwrap();
    let offices = pol["offices"].as_array().unwrap();
    assert_eq!(bodies.len(), councils.len());
    assert_eq!(offices.len(), councils.len());
    for ((c, b), o) in councils.iter().zip(bodies).zip(offices) {
        let at = serde_json::json!({ "type": "municipality", "id": c["municipality"] });
        assert_eq!(b["body"], "council");
        assert_eq!(b["at"], at);
        assert_eq!(b["members"], c["members"]);
        assert_eq!(o["office"], "mayor");
        assert_eq!(o["at"], at);
        assert_eq!(o["holder"], c["mayor"]);
    }
    // Roles: the same politicians, the same posts.
    let before = old["state"]["politics"]["politicians"].as_array().unwrap();
    let after = pol["politicians"].as_array().unwrap();
    for (a, b) in before.iter().zip(after) {
        let (k, inner) = a["role"].as_object().unwrap().iter().next().unwrap();
        let at = serde_json::json!({ "type": "municipality", "id": inner["municipality"] });
        let want = match k.as_str() {
            "Councillor" => {
                serde_json::json!({ "type": "seat", "body": "council", "at": at, "constituency": inner["ward"] })
            }
            "Mayor" => serde_json::json!({ "type": "office", "office": "mayor", "at": at }),
            _ => serde_json::json!({ "type": "private", "home": inner["municipality"] }),
        };
        assert_eq!(b["role"], want);
    }
    // Races in the campaign keep their order and their posts.
    let races = pol["elections"]["races"].as_array().unwrap();
    assert!(!races.is_empty(), "the fixture is mid-campaign");
    assert_eq!(
        races.len(),
        old["state"]["politics"]["elections"]["races"]
            .as_array()
            .unwrap()
            .len()
    );
    // The step is deterministic and the payload decodes.
    assert_eq!(migrated, migrate(18, v).unwrap());
}

/// A real v18 save (mid-campaign) loads and plays on deterministically to
/// the recorded continuation: eight turns, through an election.
///
/// **The golden.** `v18_campaign_continued.mndt` is the state today's build
/// reaches by loading `v18_campaign.mndt` and ending eight turns (written
/// with the current format). It was re-recorded for E2 (DEC-097 §3, the
/// director's variety and pacing cycle): the first golden was
/// `v18_after.mndt`, the old build's own continuation, which held while no
/// later task changed what those turns do (with carve-outs for E1's record
/// value and N2's ward vote figures). E2 changes which situations the
/// director spotlights, so no build after it can reach the old build's
/// state. The comparison is the whole state, with no carve-outs. When a
/// later task changes an outcome on purpose, re-record with
/// `MANDATE_RECORD_FIXTURES=1 cargo test -p mandate-save --test
/// constitution_posts v18_save_continues`, check the diff, and say why here.
///
/// Re-recorded for the act flow (save v23, DEC-012, DEC-006): the run now
/// carries `State::act`, its Legacy record opened on load and kept through
/// the eight turns (the lost election, the seat left, and the act's
/// `lost_seat` ending). Checked against the previous golden: only `act` (new)
/// and `meta.content_hash` (the new content files) changed. The comparison
/// stays the whole state, strict, with no carve-outs.
#[test]
fn v18_save_continues_identically() {
    let c = content();
    let load_campaign = || {
        let (mut campaign, meta) = decode(&fixture("v18_campaign.mndt")).unwrap();
        assert_eq!(meta.format_version, 18);
        // The save comes from the content before R1's fields; loading it
        // records a rules change in the event log. Mark it as made with this
        // content, as the golden was.
        campaign.meta.content_hash = c.content_hash.clone();
        campaign
    };
    let campaign = load_campaign();
    assert!(
        !campaign.politics.elections.races.is_empty(),
        "mid-campaign"
    );
    let play = || {
        let mut e = mandate_sim::Engine::from_state(load_campaign(), c.clone()).unwrap();
        for _ in 0..8 {
            e.apply(Command::EndTurn).unwrap();
        }
        e
    };
    let e = play();
    // Deterministic: the same save played on again reaches the same state.
    assert_eq!(play().state_hash(), e.state_hash());
    // The old build's continuation held the same election.
    let (after, _) = decode(&fixture("v18_after.mndt")).unwrap();
    assert_eq!(
        e.state().politics.elections.last_election,
        after.politics.elections.last_election
    );
    assert!(
        e.state()
            .politics
            .elections
            .results
            .iter()
            .any(
                |r| Some(r.date) == e.state().politics.elections.last_election
                    && r.seat.is_office()
            ),
        "an election between the fixtures"
    );
    let golden =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v18_campaign_continued.mndt");
    if std::env::var_os("MANDATE_RECORD_FIXTURES").is_some() {
        let (bytes, _) = encode(e.state(), &req()).unwrap();
        std::fs::write(&golden, bytes).unwrap();
    }
    let (reached, _) = decode(&std::fs::read(&golden).unwrap()).unwrap();
    assert_eq!(reached.hash(), e.state_hash(), "the golden round-trips");
    let a = serde_json::to_value(e.state()).unwrap();
    let b = serde_json::to_value(&reached).unwrap();
    if a != b {
        // Name the first part that differs.
        for k in a.as_object().unwrap().keys() {
            assert_eq!(a[k], b[k], "state.{k} differs");
        }
    }
    assert_eq!(a, b);
}

/// A real v17 save (two formats back, mid-campaign) migrates through both
/// steps, loads, holds its election under the constitution's posts, plays
/// on and round-trips.
#[test]
fn v17_save_loads_and_holds_its_election() {
    let c = content();
    let bytes = fixture("v17_campaign.mndt");
    let (version, v) = raw(&bytes);
    assert_eq!(version, 17);
    let migrated = migrate(17, v).unwrap();
    assert_post_shapes(&migrated);
    let (state, meta) = decode(&bytes).unwrap();
    assert_eq!(meta.format_version, 17);
    let towns = state.world.municipalities.len();
    assert_eq!(state.politics.bodies.len(), towns);
    assert_eq!(state.politics.offices.len(), towns);
    let mut e = mandate_sim::Engine::from_state(state, c.clone()).unwrap();
    for _ in 0..20 {
        e.apply(Command::EndTurn).unwrap();
    }
    let st = e.state();
    let held = st.politics.elections.last_election.unwrap();
    let results: Vec<_> = st
        .politics
        .elections
        .results
        .iter()
        .filter(|r| r.date == held)
        .collect();
    assert!(!results.is_empty());
    for r in &results {
        match r.seat {
            Post::Seat { body, .. } => assert_eq!(body.as_str(), "council"),
            Post::Office { office, .. } => assert_eq!(office.as_str(), "mayor"),
        }
    }
    // Every holder's role names the post they hold.
    for b in &st.politics.bodies {
        for id in &b.members {
            let role = st.politics.get(*id).unwrap().role;
            assert!(
                matches!(role, Role::Seat { body, at, .. } if body == b.body && at == b.at),
                "{role:?}"
            );
        }
    }
    let (out, meta) = encode(st, &req()).unwrap();
    assert_eq!(meta.format_version, SAVE_FORMAT_VERSION);
    let (back, _) = decode(&out).unwrap();
    assert_eq!(back.hash(), e.state_hash());
}

/// A save whose politicians hold a post the run's constitution lacks is
/// refused on load, not run without it.
#[test]
fn a_post_the_constitution_lacks_refuses_the_save() {
    let c = content();
    let (mut state, _) = decode(&fixture("v18_campaign.mndt")).unwrap();
    let id = state.politics.offices[0].holder.unwrap();
    let at = state.politics.offices[0].at;
    state.politics.get_mut(id).unwrap().role = Role::Office {
        office: mandate_content::PostKey::new("doge").unwrap(),
        at,
    };
    let err = mandate_sim::Engine::from_state(state, c)
        .err()
        .expect("refused");
    assert!(err.to_string().contains("doge"), "{err}");
}

/// The base pack with its local posts renamed: the council is a `moot` of
/// `alderman`s, the mayor a `reeve`. Role words in content follow (they
/// name posts through the constitution); grant subjects and Fluent keys
/// do not change.
fn renamed() -> Arc<mandate_content::ContentBundle> {
    let mut src = sources();
    for s in src.iter_mut() {
        let p = s.path.replace('\\', "/");
        // Interior spot roles ("councillor" desks) are presentation words of
        // the art pack, not role words.
        if p.contains("locale/") || p.ends_with("council.jsonc") || p.ends_with("interiors.jsonc") {
            continue;
        }
        // The executive cast in its new spelling, then the role words.
        let mut t = s
            .text
            .replace(r#"{ "type": "mayor" }"#, r#"{ "type": "office" }"#);
        t = t.replace("\"councillor\"", "\"alderman\"");
        t = t.replace("\"mayor\"", "\"reeve\"");
        if p.contains("constitutions/") {
            t = t.replace("\"council\"", "\"moot\"");
        }
        s.text = t;
    }
    let (b, warnings) = load(&src).unwrap_or_else(|e| {
        panic!(
            "renamed content loads: {}",
            e.iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    assert!(
        warnings.iter().all(|w| !w.to_string().contains("error")),
        "{warnings:?}"
    );
    Arc::new(b)
}

/// A constitution whose local body and office have other ids plays a local
/// election: races are for its posts, winners hold them, the player sits
/// in its body; then a bill passes its body, is vetoed by the renamed
/// office's holder, and the body overrides the veto.
#[test]
fn renamed_constitution_plays_an_election_and_overrides_a_veto() {
    use mandate_sim::actions::PlayerCommand;
    let c = renamed();
    let k = c.constitutions.values().next().unwrap();
    assert!(
        k.tier(mandate_content::TierKey::Local)
            .unwrap()
            .body("moot")
            .is_some()
    );
    let mut e = generate(
        &WorldConfig {
            seed: 11,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    let pl = e.state().player.clone().unwrap();
    let town = Jurisdiction::town(pl.home());
    let me = e.state().politics.get(pl.politician).unwrap().role;
    assert!(
        matches!(me, Role::Seat { body, at, .. } if body.as_str() == "moot" && at == town),
        "{me:?}"
    );
    // The election: every race is for a renamed post.
    for _ in 0..60 {
        e.apply(Command::EndTurn).unwrap();
    }
    let st = e.state();
    let held = st.politics.elections.last_election.expect("an election");
    let mine: Vec<_> = st
        .politics
        .elections
        .results
        .iter()
        .filter(|r| r.date == held && r.seat.at() == town)
        .collect();
    assert!(mine.iter().any(|r| r.seat.id() == "reeve"));
    assert!(mine.iter().any(|r| r.seat.id() == "moot"));
    for r in &mine {
        let holder = mandate_sim::politics::holder_of_post(&st.politics, &st.world, r.seat);
        assert_eq!(holder, Some(r.winner), "{:?}", r.seat);
    }
    // A bill: the player drafts and introduces one; every other member
    // declares for it and the reeve against.
    if !e.state().politics.get(pl.politician).unwrap().in_office() {
        // The player lost: seat them again for the bill (a lab setup).
        let seat = mine.iter().find(|r| r.seat.id() == "moot").unwrap().seat;
        let w = mine.iter().find(|r| r.seat == seat).unwrap().winner;
        e.setup(|s| {
            mandate_sim::politics::seat_holder(&mut s.politics, &s.world, seat, pl.politician);
            s.politics.get_mut(pl.politician).unwrap().role = seat.into();
            s.politics.get_mut(w).unwrap().role = Role::Private { home: pl.home() };
        });
    }
    let template = c
        .policies
        .values()
        .find(|t| {
            t.tier == mandate_content::TierKey::Local
                && t.unlock.is_none()
                && t.requires == mandate_content::Power::ProposeOrdinance
        })
        .unwrap()
        .id
        .clone();
    let drafted = e
        .apply(e.route(PlayerCommand::DraftPolicy {
            template,
            params: Default::default(),
        }))
        .unwrap();
    let bill = drafted.result.unwrap().created.unwrap();
    e.apply(e.route(PlayerCommand::IntroducePolicy { policy: bill }))
        .unwrap();
    let k = c.constitutions.values().next().unwrap().clone();
    e.setup(|s| {
        let members = mandate_sim::politics::legislature_of(&s.politics, &k, town).to_vec();
        let reeve = mandate_sim::politics::executive_of(&s.politics, &k, town).unwrap();
        for m in members {
            s.politics.declared_votes.insert((m.0, bill), 1);
        }
        s.politics.declared_votes.insert((reeve.0, bill), -1);
    });
    for _ in 0..12 {
        e.apply(Command::EndTurn).unwrap();
        let p = &e.state().policy.instances[bill as usize];
        if p.status != mandate_sim::policy::PolicyStatus::UnderConsideration {
            break;
        }
    }
    let p = &e.state().policy.instances[bill as usize];
    let veto = p.votes.iter().find(|v| v.veto).expect("the reeve vetoed");
    assert!(veto.passed, "the moot overrode the veto: {:?}", p.votes);
    assert!(matches!(
        p.status,
        mandate_sim::policy::PolicyStatus::Enacted | mandate_sim::policy::PolicyStatus::InEffect
    ));
}
