//! The scene's view and command on the wire (DEC-112; `EXCHANGE_SCENES.md`
//! §5.7, §7, task X2):
//!
//! - the protocol's view carries the engine's with the same redaction: a
//!   group the player does not know has no lean and no number for how it
//!   moved, and an option's change for it is a range;
//! - shown = rolled, in the screen's units: for every line on offer the
//!   change the command applies is inside the points previewed;
//! - a debate on offer is on Today ("Needs you") and on the arc's sheet,
//!   and the Go-then-begin request starts it; three lines end it, and the
//!   result's "Why?" reads the three rounds back;
//! - every key the view names is in the English locale;
//! - a query writes nothing;
//! - the fixtures the web task builds against are views the engine made.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_protocol::*;
use mandate_wasm::session::Session;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn req(id: u32, body: RequestBody) -> String {
    serde_json::to_string(&Request {
        protocol_version: PROTOCOL_VERSION,
        request_id: id,
        body,
    })
    .unwrap()
}

fn send(s: &mut Session, id: u32, body: RequestBody) -> ResponseBody {
    let r: Response = serde_json::from_str(&s.handle_json(&req(id, body))).unwrap();
    match r.result {
        ResponseResult::Ok { body } => body,
        ResponseResult::Err { error } => panic!("request {id} failed: {}", error.message),
    }
}

fn session(seed: u64, local_knowledge: bool) -> Session {
    sized(seed, local_knowledge, WorldSizeDto::Small)
}

fn sized(seed: u64, local_knowledge: bool, size: WorldSizeDto) -> Session {
    let mut s = Session::new().unwrap();
    send(
        &mut s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed,
            size,
            scenario: None,
            storyteller: None,
            local_knowledge,
        }),
    );
    s
}

fn pocket(s: &mut Session) -> PocketView {
    match send(s, 2, RequestBody::Query(QueryDto::Pocket)) {
        ResponseBody::Pocket(p) => p,
        b => panic!("pocket: {b:?}"),
    }
}

fn scene(s: &mut Session, arc: u32, beat: &str) -> ArcSceneView {
    match send(
        s,
        3,
        RequestBody::Query(QueryDto::ArcExchange {
            arc,
            beat: beat.into(),
        }),
    ) {
        ResponseBody::ArcExchange(v) => v,
        b => panic!("scene: {b:?}"),
    }
}

fn command(s: &mut Session, c: CommandDto) {
    send(s, 4, RequestBody::Command(c));
}

/// Turns until the debate needs the player: (arc, beat, turn). Another
/// arc's scene may come first (the zoning fight's meeting, 2026-11-25);
/// these tests are the debate's.
fn to_scene(s: &mut Session) -> (u32, String, u32) {
    for _ in 0..80 {
        let p = pocket(s);
        if let Some(n) = p.needs_you.iter().find(|n| {
            n.kind == NeedKind::Scene
                && matches!(&n.target, NeedTargetDto::Scene { beat, .. } if beat == "showdown.debate")
        }) {
            let NeedTargetDto::Scene { arc, beat } = &n.target else {
                panic!("a scene's need targets the scene");
            };
            return (*arc, beat.clone(), p.turn);
        }
        command(s, CommandDto::EndTurn);
    }
    panic!("no scene in eighty turns");
}

fn begin(s: &mut Session, v: &ArcSceneView) {
    let to = v.route.as_ref().and_then(|r| r.to.clone()).unwrap();
    command(
        s,
        CommandDto::Go {
            location: to.location,
            then: Some(Box::new(CommandDto::ArcExchange {
                arc: v.arc,
                beat: v.beat.clone(),
                act: SceneActDto::Begin,
            })),
        },
    );
}

fn say(s: &mut Session, v: &ArcSceneView, option: &str) {
    command(
        s,
        CommandDto::ArcExchange {
            arc: v.arc,
            beat: v.beat.clone(),
            act: SceneActDto::Say {
                option: option.into(),
            },
        },
    );
}

/// The seed the browser test plays (`web/e2e/exchange.spec.ts`).
const SEED: u64 = 4242;

/// Every key a scene view names, with the keys its arguments name.
fn keys(v: &ArcSceneView) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut text = |t: &TextView| {
        out.insert(t.key.clone());
        for (k, x) in &t.args {
            if k.ends_with("_key") && !x.is_empty() {
                out.insert(x.clone());
            }
        }
    };
    text(&v.title);
    text(&v.offer);
    text(&v.opening);
    for g in &v.groups {
        text(&g.label);
        text(&g.short);
        text(&TextView::new(g.lean_key.clone()));
    }
    if let Some(p) = &v.prompt {
        text(&p.text);
    }
    for o in &v.options {
        text(&o.label);
        for n in &o.notes {
            text(n);
        }
        if let Some(k) = &o.answer_key {
            text(&TextView::new(k.clone()));
        }
    }
    if let Some(l) = &v.locked {
        text(&l.text);
    }
    for r in &v.played {
        for k in [
            Some(&r.prompt_key),
            r.said_key.as_ref(),
            r.reply_key.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            text(&TextView::new(k.clone()));
        }
        text(&TextView::new(r.room_key.clone()));
    }
    if let Some(r) = &v.result {
        text(&r.text);
        text(&r.headline);
        text(&TextView::new(r.outcome_key.clone()));
        for g in &r.groups {
            text(&TextView::new(g.moved_key.clone()));
        }
        for p in &r.promises {
            text(&p.title);
        }
    }
    out
}

fn assert_in_locale(s: &Session, v: &ArcSceneView) {
    let known = mandate_content::validate::fluent_keys(&s.content().locale_text("en"));
    for k in keys(v) {
        assert!(known.contains(&k), "{k} is not in the English locale");
    }
}

/// A group the player cannot know shows no lean and no number for how it
/// moved; a known one shows both.
fn assert_redacted(v: &ArcSceneView, truth: &ArcSceneView) {
    for (g, t) in v.groups.iter().zip(&truth.groups) {
        assert_eq!(g.key, t.key);
        if g.lean.is_none() {
            assert_eq!(g.lean_key, "exchange-lean-unknown");
            assert!(g.moved_points.is_none(), "{}: a move in points", g.key);
        } else {
            assert_eq!(g.lean, t.lean);
            assert_eq!(g.moved_points, t.moved_points);
        }
        assert_eq!(g.moved_steps, t.moved_steps);
    }
    if let Some(r) = &v.result {
        for (g, sv) in r.groups.iter().zip(&v.groups) {
            assert_eq!(g.points.is_some(), sv.lean.is_some());
            assert_eq!(g.before.is_some(), sv.lean.is_some());
        }
    }
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/src/components/fixtures/exchange")
}

fn fixture(name: &str, v: &ArcSceneView) {
    let path = fixture_dir().join(format!("{name}.json"));
    let json = serde_json::to_string_pretty(v).unwrap() + "\n";
    if std::env::var("MANDATE_WRITE_FIXTURES").is_ok() {
        std::fs::create_dir_all(fixture_dir()).unwrap();
        std::fs::write(&path, json).unwrap();
    } else {
        let on_disk = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{} missing: write the fixtures", path.display()));
        let back: ArcSceneView = serde_json::from_str(&on_disk)
            .unwrap_or_else(|e| panic!("{} no longer reads as today's view: {e}", path.display()));
        assert_eq!(&back, v, "{} is stale: write the fixtures", path.display());
    }
}

#[test]
fn a_debate_is_played_on_the_wire_and_shown_as_rolled() {
    let mut s = session(SEED, false);
    let (arc, beat, turn) = to_scene(&mut s);
    println!("seed {SEED}: a scene needs the player on turn {turn} ({arc} {beat})");
    let v = scene(&mut s, arc, &beat);
    assert_eq!(v.phase, "open");
    assert!(v.available, "{:?}", v.blocked_key);
    assert!(v.route.is_some());
    assert!(v.prompt.is_none() && v.options.is_empty());
    assert_in_locale(&s, &v);
    fixture("open", &v);

    // The arc's sheet lists it, as the command that begins it.
    let a = match send(&mut s, 5, RequestBody::Query(QueryDto::Arc { id: arc })) {
        ResponseBody::Arc(a) => a,
        b => panic!("arc: {b:?}"),
    };
    let b = a
        .beats
        .iter()
        .find(|b| b.kind == "exchange")
        .expect("the arc's sheet lists the scene");
    assert_eq!(format!("{}.{}", b.stage, b.beat), beat);
    assert_eq!(b.label, v.title);
    assert!(b.available);

    // A query writes nothing.
    let before = s.engine().unwrap().state_hash();
    let _ = scene(&mut s, arc, &beat);
    assert_eq!(s.engine().unwrap().state_hash(), before);

    begin(&mut s, &v);
    let mut unknown = 0;
    let mut ranges = 0;
    for round in 1..=3u8 {
        let v = scene(&mut s, arc, &beat);
        let e = s.engine().unwrap();
        let truth = scene_view_known(e, arc, &beat).unwrap();
        assert_eq!(v.phase, "under_way");
        assert_eq!(v.round, round);
        assert!(v.prompt.is_some());
        assert!(
            (4..=5).contains(&v.options.len()),
            "{} lines",
            v.options.len()
        );
        assert!(v.options.iter().any(|o| o.safe), "the safe line is there");
        assert!(v.options.iter().all(|o| o.tags.len() <= 3));
        assert_redacted(&v, &truth);
        assert_in_locale(&s, &v);
        unknown += v.groups.iter().filter(|g| g.lean.is_none()).count();
        if round == 1 {
            fixture("round", &v);
        }
        ranges += every_line_lands_inside(e, &v, &beat);
        let pick = v.options[usize::from(round) % v.options.len()].id.clone();
        say(&mut s, &v, &pick);
    }
    // In this world the player knows their seat's worries from the start:
    // the groups are known, and the ranges are the other side's answers
    // (the next test forgets the seat).
    let _ = unknown;
    assert!(ranges > 0, "no range was ever shown");

    let v = scene(&mut s, arc, &beat);
    assert_eq!(v.phase, "done");
    assert_eq!(v.played.len(), 3);
    let r = v.result.as_ref().unwrap();
    assert!(["won", "split", "lost"].contains(&r.outcome.as_str()));
    let truth = scene_view_known(s.engine().unwrap(), arc, &beat).unwrap();
    assert_redacted(&v, &truth);
    assert_in_locale(&s, &v);
    fixture("result", &v);
    // The scene no longer needs the player.
    assert!(
        !pocket(&mut s)
            .needs_you
            .iter()
            .any(|n| n.kind == NeedKind::Scene)
    );
    // "Why?" on the result reads the three rounds back.
    let why = match send(
        &mut s,
        6,
        RequestBody::Query(QueryDto::Why {
            event: r.event,
            depth: None,
        }),
    ) {
        ResponseBody::Why(w) => w,
        b => panic!("why: {b:?}"),
    };
    let rounds: BTreeSet<u32> = v.played.iter().map(|x| x.event).collect();
    let found: BTreeSet<u32> = why
        .chain
        .iter()
        .map(|x| x.id)
        .filter(|id| rounds.contains(id))
        .collect();
    assert_eq!(found, rounds, "the result's chain holds the three rounds");
    // The wire names.
    assert_eq!(
        serde_json::to_string(&QueryDto::ArcExchange {
            arc: 3,
            beat: "showdown.debate".into()
        })
        .unwrap(),
        r#"{"type":"arc_exchange","arc":3,"beat":"showdown.debate"}"#
    );
}

/// Every line on offer, said on a copy of the game: what the command does
/// is inside the points (and the steps) the view showed, and the copy's
/// view says no more than it may. The ranges seen.
fn every_line_lands_inside(e: &mandate_sim::Engine, v: &ArcSceneView, beat: &str) -> usize {
    let truth = scene_view_known(e, v.arc, beat).unwrap();
    let mut ranges = 0;
    for (o, t) in v.options.iter().zip(&truth.options) {
        assert_eq!(o.id, t.id);
        let mut f = mandate_sim::Engine::from_state(e.state().clone(), e.content_arc()).unwrap();
        // The command the wire's `Say` becomes.
        f.apply(mandate_sim::Command::Player(
            mandate_sim::actions::PlayerCommand::Exchange {
                run: v.arc,
                beat: beat.to_string(),
                act: SceneActDto::Say {
                    option: o.id.clone(),
                }
                .to_sim(),
            },
        ))
        .unwrap();
        let after = scene_view(&f, v.arc, beat).unwrap();
        let known = scene_view_known(&f, v.arc, beat).unwrap();
        let r = &known.played[usize::from(v.round) - 1];
        assert_eq!(r.option.as_deref(), Some(o.id.as_str()));
        let shown = if r.stood || o.fails.is_empty() {
            &o.stands
        } else {
            &o.fails
        };
        for (m, c) in r.moved.iter().zip(shown) {
            let p = m.points.unwrap();
            assert!(
                c.lo <= p && p <= c.hi,
                "{}: {} moved {p}, shown {c:?}",
                o.id,
                m.group
            );
            assert!(c.lo_steps <= m.steps && m.steps <= c.hi_steps);
            if !c.exact {
                ranges += 1;
            }
        }
        assert_redacted(&after, &known);
    }
    ranges
}

/// The unprepared player's ledger: what they know of the seat's worries
/// and of everyone, forgotten (test setup).
fn blank(e: &mut mandate_sim::Engine) {
    use mandate_sim::player::knowledge::Subject;
    e.setup(|st| {
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        k.entries.retain(|s, _| {
            !matches!(
                s,
                Subject::HoodConcerns { .. }
                    | Subject::TownConcerns { .. }
                    | Subject::Person { .. }
            )
        });
    });
}

#[test]
fn a_group_the_player_does_not_know_is_a_range_and_the_change_is_inside_it() {
    let mut s = session(SEED, false);
    let (arc, beat, _) = to_scene(&mut s);
    let mut e = mandate_sim::Engine::from_state(
        s.engine().unwrap().state().clone(),
        s.engine().unwrap().content_arc(),
    )
    .unwrap();
    blank(&mut e);
    let v = scene_view(&e, arc, &beat).unwrap();
    let to = v
        .route
        .as_ref()
        .and_then(|r| r.to.clone())
        .unwrap()
        .location;
    let act = |a: SceneActDto| mandate_sim::actions::PlayerCommand::Exchange {
        run: arc,
        beat: beat.clone(),
        act: a.to_sim(),
    };
    e.apply(mandate_sim::Command::Player(
        mandate_sim::actions::PlayerCommand::Go {
            location: to.to_sim(),
            then: Some(Box::new(act(SceneActDto::Begin))),
        },
    ))
    .unwrap();
    let mut ranges = 0;
    for round in 1..=3u8 {
        let v = scene_view(&e, arc, &beat).unwrap();
        let truth = scene_view_known(&e, arc, &beat).unwrap();
        if round == 2 {
            // A round with a move already made: the strip's arrows, no numbers.
            fixture("round-unknown", &v);
        }
        assert!(
            v.groups.iter().all(|g| g.lean.is_none()),
            "nothing is known of the seat"
        );
        assert!(
            v.groups
                .iter()
                .all(|g| g.lean_key == "exchange-lean-unknown")
        );
        assert_redacted(&v, &truth);
        ranges += every_line_lands_inside(&e, &v, &beat);
        let pick = v.options[0].id.clone();
        e.apply(mandate_sim::Command::Player(act(SceneActDto::Say {
            option: pick,
        })))
        .unwrap();
    }
    assert!(ranges > 0);
    let v = scene_view(&e, arc, &beat).unwrap();
    let r = v.result.as_ref().unwrap();
    assert!(
        r.groups
            .iter()
            .all(|g| g.before.is_none() && g.points.is_none())
    );
}

#[test]
fn today_drops_the_alerts_its_rows_already_say() {
    let mut s = session(SEED, false);
    for _ in 0..40 {
        let p = pocket(&mut s);
        let decisions = p
            .needs_you
            .iter()
            .filter(|n| n.kind == NeedKind::Decision)
            .count();
        if decisions > 0 {
            assert!(
                !p.alerts.iter().any(|a| a.key == "alert-decisions"),
                "{decisions} decisions are rows, and an alert says so again"
            );
            return;
        }
        command(&mut s, CommandDto::EndTurn);
    }
    panic!("no decision in forty turns");
}
