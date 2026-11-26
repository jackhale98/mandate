//! The audition and the opening draft on the wire (DEC-101 §5; STAGED_ARCS
//! §6; task A7):
//!
//! - the same seed and parameters give the same draft, to the last score;
//!   the native view is pinned in a fixture the WASM build is checked
//!   against (`web/src/engine/draft.parity.test.ts`);
//! - an audition leaves the real state as it was: its hash, its command
//!   log, the next turn;
//! - nothing from a fork appears in any view: every view reads the same
//!   after a draft, and each opening offered is a candidate as the scan
//!   read it from the unforked world, before any fork was played;
//! - every opening drafted is a matter that built in its passive fork.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_protocol::*;
use mandate_sim::arcs::audition::{self, Audition, Params};
use mandate_wasm::session::Session;
use std::path::{Path, PathBuf};

/// The world the fixture pins (small; its draft holds openings).
const SEED: u64 = 3;

fn req(id: u32, body: RequestBody) -> String {
    serde_json::to_string(&Request {
        protocol_version: PROTOCOL_VERSION,
        request_id: id,
        body,
    })
    .unwrap()
}

/// A response's JSON, ok or not (a view that is refused must be refused
/// the same way after an audition too).
fn raw(s: &mut Session, id: u32, body: RequestBody) -> String {
    s.handle_json(&req(id, body))
}

fn send(s: &mut Session, id: u32, body: RequestBody) -> ResponseBody {
    let r: Response = serde_json::from_str(&raw(s, id, body)).unwrap();
    match r.result {
        ResponseResult::Ok { body } => body,
        ResponseResult::Err { error } => panic!("request {id} failed: {}", error.message),
    }
}

fn session(seed: u64) -> Session {
    let mut s = Session::new().unwrap();
    send(
        &mut s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
        }),
    );
    s
}

fn draft(s: &mut Session) -> DraftView {
    match send(
        s,
        2,
        RequestBody::Query(QueryDto::Draft { recent: Vec::new() }),
    ) {
        ResponseBody::Draft(d) => d,
        b => panic!("draft: {b:?}"),
    }
}

/// Every view a player can ask for without naming something.
fn views() -> Vec<QueryDto> {
    vec![
        QueryDto::Overview,
        QueryDto::Office,
        QueryDto::Campaign,
        QueryDto::Map,
        QueryDto::MapLayers,
        QueryDto::Pocket,
        QueryDto::Town,
        QueryDto::TownMapLayers,
        QueryDto::Story,
        QueryDto::Newspaper,
        QueryDto::Notebook,
        QueryDto::TermReview,
        QueryDto::SkipSummary,
        QueryDto::Legacy,
        QueryDto::Asks,
        QueryDto::People,
        QueryDto::PromiseMenu,
    ]
}

fn read_all(s: &mut Session) -> Vec<String> {
    views()
        .into_iter()
        .enumerate()
        .map(|(i, q)| raw(s, 10 + i as u32, RequestBody::Query(q)))
        .collect()
}

#[test]
fn an_audition_leaves_the_game_and_every_view_as_they_were() {
    let mut s = session(SEED);
    let mut twin = session(SEED);
    let before = read_all(&mut s);
    let hash = s.engine().unwrap().state_hash();
    let log = s.engine().unwrap().command_log().len();
    let d = draft(&mut s);
    assert!(!d.openings.is_empty(), "seed {SEED} drafts nothing");
    assert_eq!(s.engine().unwrap().state_hash(), hash);
    assert_eq!(s.engine().unwrap().command_log().len(), log);
    let after = read_all(&mut s);
    for (q, (b, a)) in views().iter().zip(before.iter().zip(&after)) {
        assert_eq!(b, a, "{q:?} changed after an audition");
    }
    // The next turn is the one the game would have played: no stream was
    // drawn from, nothing waits to be applied.
    send(&mut s, 50, RequestBody::Command(CommandDto::EndTurn));
    send(&mut twin, 50, RequestBody::Command(CommandDto::EndTurn));
    assert_eq!(
        s.engine().unwrap().state_hash(),
        twin.engine().unwrap().state_hash()
    );
}

#[test]
fn the_same_seed_gives_the_same_draft_and_the_fixture_is_the_engines() {
    let mut s = session(SEED);
    let a = draft(&mut s);
    let b = draft(&mut s);
    assert_eq!(a, b);
    // To the last score, from another session of the same seed.
    let e = s.engine().unwrap();
    let p = Params::for_state(e.state());
    let one = audition::audition(e.state(), &e.content_arc(), p.clone()).unwrap();
    let t = session(SEED);
    let e2 = t.engine().unwrap();
    let two = audition::audition(e2.state(), &e2.content_arc(), p).unwrap();
    assert_eq!(audition::digest(&one), audition::digest(&two));
    assert_eq!(one, two);
    fixture(&format!("draft_{SEED}"), &a);
}

#[test]
fn each_opening_offered_is_a_day_0_fact_and_a_matter_that_built() {
    let mut offered = 0;
    for seed in [1, 2, SEED] {
        let mut s = session(seed);
        let e = s.engine().unwrap();
        let content = e.content_arc();
        let p = Params::for_state(e.state());
        // What the scan read before any fork was played.
        let scanned: Vec<_> = Audition::begin(e.state(), &content, p.clone())
            .unwrap()
            .candidates()
            .cloned()
            .collect();
        let d = audition::audition(e.state(), &content, p).unwrap();
        for a in d.offered() {
            offered += 1;
            assert!(
                scanned.contains(&a.candidate),
                "seed {seed}: {} at town {} is not as the scan read it",
                a.candidate.template,
                a.candidate.town.0
            );
            let pa = a
                .forks
                .iter()
                .find(|f| f.kind == mandate_sim::standin::Kind::Passive)
                .unwrap();
            assert!(
                audition::escalates(content.arc(&a.candidate.template), &pa.trace),
                "seed {seed}: {} at town {} was drafted and never built",
                a.candidate.template,
                a.candidate.town.0
            );
            assert!(a.score.as_ref().is_some_and(|x| x.escalates));
        }
        // The view is the draft's candidates, and names no score.
        let v = draft(&mut s);
        let views: Vec<(u32, String)> = v
            .openings
            .iter()
            .map(|o| (o.town, o.template.clone()))
            .collect();
        let chosen: Vec<(u32, String)> = d
            .offered()
            .map(|a| (a.candidate.town.0, a.candidate.template.clone()))
            .collect();
        assert_eq!(views, chosen);
        let json = serde_json::to_string(&v).unwrap();
        for word in [
            "score",
            "agency",
            "escalation",
            "visibility",
            "pacing",
            "total",
        ] {
            assert!(!json.contains(word), "the draft view names `{word}`");
        }
    }
    assert!(offered > 0, "no world drafted anything");
}

/// A draft found by advancing the world opens on the game's own future:
/// the same advance on the real game shows the same candidates, so the
/// frame (task A8) can start play there.
#[test]
fn the_world_a_draft_opens_on_is_the_games_own_future() {
    let mut s = session(SEED);
    let e = s.engine().unwrap();
    let content = e.content_arc();
    let d = audition::audition(e.state(), &content, Params::for_state(e.state())).unwrap();
    assert!(d.months_ahead > 0, "seed {SEED} drafts as play begins");
    for i in 0..d.months_ahead {
        send(
            &mut s,
            60 + i,
            RequestBody::Command(CommandDto::AdvanceDays {
                days: audition::SCAN_STEP_DAYS,
            }),
        );
    }
    let st = s.engine().unwrap().state();
    assert_eq!(st.calendar.date, d.opens);
    let found = audition::scan(st, &content, &[]);
    for a in d.offered() {
        assert!(found.contains(&a.candidate), "{:?}", a.candidate);
    }
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/src/components/fixtures/draft")
}

fn fixture(name: &str, v: &DraftView) {
    let path = fixture_dir().join(format!("{name}.json"));
    let json = serde_json::to_string_pretty(v).unwrap() + "\n";
    if std::env::var("MANDATE_WRITE_FIXTURES").is_ok() {
        std::fs::create_dir_all(fixture_dir()).unwrap();
        std::fs::write(&path, json).unwrap();
    } else {
        let on_disk = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{} missing: write the fixtures", path.display()));
        let back: DraftView = serde_json::from_str(&on_disk)
            .unwrap_or_else(|e| panic!("{} no longer reads as today's view: {e}", path.display()));
        assert_eq!(&back, v, "{} is stale: write the fixtures", path.display());
    }
}
