//! The audition and the opening draft on the wire (DEC-101 §3 and §5;
//! STAGED_ARCS §6; tasks A7 and A8):
//!
//! - the same seed and parameters give the same draft, to the last score;
//!   the native view is pinned in a fixture the WASM build is checked
//!   against (`web/src/engine/draft.parity.test.ts`); a second query reads
//!   the draft made;
//! - an audition leaves the real state as it was: its hash, its command
//!   log, the next turn;
//! - nothing from a fork appears in any view: every view reads the same
//!   after a draft, each opening offered is a candidate as the scan read
//!   it from the unforked world, and the view names no score;
//! - every opening drafted is a matter that built in its passive fork;
//! - **the first-beat guarantee**: in every drafted opening the player has
//!   a beat of its arc on offer on turn 1 (16 small worlds and 3 medium);
//! - the same seed and choice give the same game, through a save and a
//!   replay of the run file; the run log keeps the scores.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_protocol::*;
use mandate_sim::Engine;
use mandate_sim::arcs::audition::{self, Audition, Params};
use mandate_sim::arcs::opening::{self, OpeningChoice};
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

fn session_sized(seed: u64, size: WorldSizeDto) -> Session {
    let mut s = Session::new().unwrap();
    send(
        &mut s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed,
            size,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
            recent_openings: Vec::new(),
        }),
    );
    s
}

fn session(seed: u64) -> Session {
    session_sized(seed, WorldSizeDto::Small)
}

fn draft(s: &mut Session) -> DraftView {
    match send(s, 2, RequestBody::Query(QueryDto::Draft)) {
        ResponseBody::Draft(d) => d,
        b => panic!("draft: {b:?}"),
    }
}

fn choose(s: &mut Session, index: u32) -> OverviewView {
    match send(s, 3, RequestBody::ChooseOpening { index }) {
        ResponseBody::GameStarted(o) => o,
        b => panic!("choose: {b:?}"),
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
fn the_same_seed_gives_the_same_draft_made_once_and_the_fixture_is_the_engines() {
    let mut s = session(SEED);
    // One fork a message: progress, then the draft.
    let mut steps = 0;
    let stepped = loop {
        steps += 1;
        match send(&mut s, 4, RequestBody::Query(QueryDto::DraftStep)) {
            ResponseBody::DraftProgress(p) => assert!(p.done <= p.total, "{p:?}"),
            ResponseBody::Draft(d) => break d,
            b => panic!("draft step: {b:?}"),
        }
        assert!(steps < 100, "the audition never ends");
    };
    assert!(steps > 3, "the audition took {steps} messages");
    // A second query reads the draft made (one message, no fork played).
    let a = draft(&mut s);
    assert_eq!(a, stepped);
    match send(&mut s, 5, RequestBody::Query(QueryDto::DraftStep)) {
        ResponseBody::Draft(d) => assert_eq!(d, a),
        b => panic!("a made draft answers its view: {b:?}"),
    }
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
            assert!(
                a.arrival.first_beat,
                "seed {seed}: drafted without a beat on turn 1"
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
        // The view is the draft's candidates, each with its line, and
        // names no score.
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
        for o in &v.openings {
            assert!(
                o.pitch_key
                    .as_deref()
                    .is_some_and(|k| k.starts_with("pitch-"))
            );
            assert!(o.post.is_some(), "seed {seed}: an opening without a seat");
            assert!(o.args.contains_key("place"));
        }
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
/// the same advance on the real game is the audition's world, to the
/// state's hash.
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

/// The first-beat guarantee (STAGED_ARCS A8's acceptance): every opening
/// of every draft, opened on the real game, has a beat of its arc on offer
/// on turn 1 that the player is shown (Today's row for the arc counts it,
/// and its sheet lists it), and the arrival is the one the audition read.
#[test]
fn a_beat_on_turn_1_in_every_drafted_opening() {
    let worlds: Vec<(u64, WorldSizeDto)> = (1..=16)
        .map(|s| (s, WorldSizeDto::Small))
        .chain([11, 12, 13].map(|s| (s, WorldSizeDto::Medium)))
        .collect();
    let mut openings = 0;
    let mut empty = Vec::new();
    for (seed, size) in worlds {
        let mut s = session_sized(seed, size);
        let d = {
            let e = s.engine().unwrap();
            audition::audition(e.state(), &e.content_arc(), Params::for_state(e.state())).unwrap()
        };
        if d.openings.is_empty() {
            empty.push(seed);
        }
        let base = s.engine().unwrap();
        for (i, a) in d.offered().enumerate() {
            openings += 1;
            let mut e = Engine::from_state(base.state().clone(), base.content_arc()).unwrap();
            let choice = OpeningChoice {
                months_ahead: d.months_ahead,
                town: a.candidate.town.0,
                template: a.candidate.template.clone(),
            };
            let got = opening::open(&mut e, &choice).unwrap();
            let what = format!("seed {seed} ({size:?}) opening {i}: {}", choice.template);
            assert_eq!(got, a.arrival, "{what}: not the arrival the audition read");
            assert!(
                e.command_log().is_empty(),
                "{what}: the opening logged commands"
            );
            assert_eq!(e.state().calendar.turn, 0, "{what}");
            assert_eq!(
                mandate_sim::player::home(e.state()).map(|h| h.0),
                Some(choice.town)
            );
            let run = got.run.unwrap();
            let pocket = pocket_view(&e).unwrap();
            let row = pocket
                .arcs
                .iter()
                .find(|r| r.id == run.0)
                .unwrap_or_else(|| panic!("{what}: no row for the arc on Today"));
            assert!(row.open_beats > 0, "{what}: nothing to do in it on turn 1");
            let sheet = arc_view(&e, run.0).unwrap();
            assert!(!sheet.beats.is_empty(), "{what}: its sheet lists no beat");
        }
        // And through the wire: the first opening chosen.
        if !d.openings.is_empty() {
            draft(&mut s);
            choose(&mut s, 0);
            let pocket = match send(&mut s, 6, RequestBody::Query(QueryDto::Pocket)) {
                ResponseBody::Pocket(p) => p,
                b => panic!("{b:?}"),
            };
            assert!(
                pocket.arcs.iter().any(|r| r.open_beats > 0),
                "seed {seed}: turn 1 shows no beat of the opening"
            );
        }
    }
    eprintln!("{openings} openings drafted; worlds with none: {empty:?}");
    assert!(openings >= 16, "only {openings} openings in 19 worlds");
}

fn save_of(s: &Session) -> Vec<u8> {
    s.export_save(&SaveRequestDto {
        run_id: "ab".repeat(16),
        revision: 1,
        parent_revision: None,
        device_id: "d".into(),
        saved_at: "2026-11-26T00:00:00Z".into(),
        run_mode: RunModeDto::Ironman,
    })
    .unwrap()
}

/// The same seed and choice give the same game: twice, through a save,
/// and replayed from the run file (which keeps the choice and the scores,
/// not the days the opening stepped).
#[test]
fn the_same_seed_and_choice_give_the_same_game_through_a_save() {
    let mut a = session(SEED);
    let mut b = session(SEED);
    let da = draft(&mut a);
    assert_eq!(da, draft(&mut b));
    let last = (da.openings.len() - 1) as u32;
    choose(&mut a, last);
    choose(&mut b, last);
    let want = &da.openings[last as usize];
    let st = a.engine().unwrap().state();
    assert_eq!(mandate_sim::player::home(st).map(|h| h.0), Some(want.town));
    assert_eq!(
        a.engine().unwrap().state_hash(),
        b.engine().unwrap().state_hash()
    );
    // A second choice, or a draft after the choice, is refused.
    let again: Response =
        serde_json::from_str(&raw(&mut a, 7, RequestBody::ChooseOpening { index: 0 })).unwrap();
    assert!(matches!(again.result, ResponseResult::Err { .. }));
    // Through a save: the same game, and the same next turn.
    let mut c = Session::new().unwrap();
    c.import_save(&save_of(&a)).unwrap();
    assert_eq!(
        c.engine().unwrap().state_hash(),
        a.engine().unwrap().state_hash()
    );
    for s in [&mut a, &mut b, &mut c] {
        send(s, 8, RequestBody::Command(CommandDto::EndTurn));
    }
    let h = a.engine().unwrap().state_hash();
    assert_eq!(b.engine().unwrap().state_hash(), h);
    assert_eq!(c.engine().unwrap().state_hash(), h);
    // The run file: the choice and the scores, and a replay from it.
    let core: serde_json::Value = serde_json::from_str(&a.report_core().unwrap()).unwrap();
    let log = &core["log"];
    assert_eq!(log["new_game"]["opening"]["town"], want.town);
    assert_eq!(
        log["new_game"]["opening"]["template"],
        want.template.as_str()
    );
    assert!(
        log["draft"]["candidates"]
            .as_array()
            .is_some_and(|c| !c.is_empty())
    );
    assert!(log["draft"]["candidates"][0]["total"].is_number());
    let out: serde_json::Value =
        serde_json::from_str(&c.replay_report(&a.report_core().unwrap(), None).unwrap()).unwrap();
    assert!(out["divergence"].is_null(), "the replay parted: {out}");
    assert_eq!(out["state_hash"], h.as_str());
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
