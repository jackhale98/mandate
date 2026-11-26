//! The career card and stepping away on the wire (M6 note 23):
//!
//! - stepping away works at any time: on the first turn, and in a campaign
//!   in the middle of a scene; the run closes with its ending, the Legacy
//!   record names the step away and the race walked away from, time no
//!   longer passes for the player, and the card says unfinished;
//! - a race is rated as it opens with the player in it, and the opening on
//!   the world as play began; the card's figure is wins less the wins the
//!   ratings expected;
//! - the card is a function of the run file: the same through a save, and
//!   the same when the opening is rated in another session from the file;
//! - a world code round-trips: the code read back gives the same setup,
//!   and a game started from it the same first turn, drafted opening
//!   included;
//! - every key the card names is in the English locale;
//! - the fixtures the web builds against are cards the engine made.
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

fn try_send(s: &mut Session, id: u32, body: RequestBody) -> Result<ResponseBody, ProtocolError> {
    let r: Response = serde_json::from_str(&s.handle_json(&req(id, body))).unwrap();
    match r.result {
        ResponseResult::Ok { body } => Ok(body),
        ResponseResult::Err { error } => Err(error),
    }
}

fn send(s: &mut Session, id: u32, body: RequestBody) -> ResponseBody {
    try_send(s, id, body).unwrap_or_else(|e| panic!("request {id} failed: {}", e.message))
}

/// A new game as the title screen starts one: the storyteller chosen is the
/// default (the browser test reads this game's card from the fixture).
fn new_game(seed: u64) -> Session {
    let mut s = Session::new().unwrap();
    let teller = s.content().director.default_profile.clone();
    send(
        &mut s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: Some(teller),
            local_knowledge: false,
            recent_openings: Vec::new(),
        }),
    );
    s
}

fn command(s: &mut Session, c: CommandDto) {
    send(s, 4, RequestBody::Command(c));
}

fn card(s: &mut Session) -> Option<CareerCardView> {
    match send(s, 5, RequestBody::Query(QueryDto::CareerCard)) {
        ResponseBody::CareerCard(c) => c,
        b => panic!("card: {b:?}"),
    }
}

fn pocket(s: &mut Session) -> PocketView {
    match send(s, 2, RequestBody::Query(QueryDto::Pocket)) {
        ResponseBody::Pocket(p) => p,
        b => panic!("pocket: {b:?}"),
    }
}

fn legacy(s: &mut Session) -> LegacyView {
    match send(s, 6, RequestBody::Query(QueryDto::Legacy)) {
        ResponseBody::Legacy(Some(l)) => l,
        b => panic!("legacy: {b:?}"),
    }
}

fn save_of(s: &Session) -> Vec<u8> {
    s.export_save(&SaveRequestDto {
        run_id: "cd".repeat(16),
        revision: 1,
        parent_revision: None,
        device_id: "d".into(),
        saved_at: "2026-11-26T00:00:00Z".into(),
        run_mode: RunModeDto::Ironman,
    })
    .unwrap()
}

fn log_of(s: &Session) -> serde_json::Value {
    let core: serde_json::Value = serde_json::from_str(&s.report_core().unwrap()).unwrap();
    core["log"].clone()
}

/// Every Fluent key the card names, with those its arguments name.
fn keys(c: &CareerCardView) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut text = |t: &TextView| {
        out.insert(t.key.clone());
        for (k, v) in &t.args {
            if k.ends_with("_key") {
                out.insert(v.clone());
            }
        }
    };
    if let Some(a) = &c.opening.arc {
        text(a);
    }
    for a in &c.arcs {
        text(&a.title);
        text(&a.text);
    }
    for d in &c.decisions {
        text(&TextView::of(d.text_key.clone(), d.args.clone()));
    }
    if let Some(e) = &c.ending {
        out.insert(e.title_key.clone());
        out.insert(e.text_key.clone());
        out.extend(e.facts.iter().map(|f| f.label_key.clone()));
    }
    for r in &c.races {
        out.insert(r.post.label_key.clone());
        out.insert(format!("ui-card-race-{}", r.outcome));
        for f in r.rating.iter().flat_map(|x| &x.facts) {
            out.insert(f.key.clone());
        }
    }
    for f in c.opening.rating.iter().flat_map(|x| &x.facts) {
        out.insert(f.key.clone());
    }
    if let Some(k) = &c.world.scenario_key {
        out.insert(k.clone());
    }
    out.insert(c.world.constitution_key.clone());
    out.insert(format!("ui-card-size-{}", size_word(c.world.size)));
    out
}

fn size_word(s: WorldSizeDto) -> &'static str {
    match s {
        WorldSizeDto::Small => "small",
        WorldSizeDto::Medium => "medium",
        WorldSizeDto::Large => "large",
    }
}

fn assert_in_locale(s: &Session, c: &CareerCardView) {
    let ftl = s.content().locale_text("en");
    for k in keys(c) {
        assert!(
            ftl.lines().any(|l| l.starts_with(&format!("{k} ="))),
            "`{k}` has no English text"
        );
    }
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/src/components/fixtures/card")
}

fn fixture<T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug>(
    name: &str,
    v: &T,
) {
    let path = fixture_dir().join(format!("{name}.json"));
    let json = serde_json::to_string_pretty(v).unwrap() + "\n";
    if std::env::var("MANDATE_WRITE_FIXTURES").is_ok() {
        std::fs::create_dir_all(fixture_dir()).unwrap();
        std::fs::write(&path, json).unwrap();
    } else {
        let on_disk = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{} missing: write the fixtures", path.display()));
        let back: T = serde_json::from_str(&on_disk)
            .unwrap_or_else(|e| panic!("{} no longer reads as today's view: {e}", path.display()));
        assert_eq!(&back, v, "{} is stale: write the fixtures", path.display());
    }
}

/// The seed the browser test plays the step away on (`web/e2e/career-card.spec.ts`).
const SEED: u64 = 3;

/// Stepped away on the first turn: the card says unfinished, the record and
/// the epilogue say how; the same card through a save; the opening rated
/// alike in a session that reads the file before it was rated.
#[test]
fn stepping_away_on_the_first_turn_gives_the_same_card_from_the_run_file() {
    let mut a = new_game(SEED);
    // While the act runs there is no card, and nothing is rated yet.
    assert_eq!(card(&mut a), None);
    command(&mut a, CommandDto::StepAway);
    let o = match send(&mut a, 7, RequestBody::Query(QueryDto::Overview)) {
        ResponseBody::Overview(o) => o,
        b => panic!("{b:?}"),
    };
    assert_eq!(
        o.act.as_ref().and_then(|x| x.ended_key.as_deref()),
        Some("ending-stepped_away")
    );
    let l = legacy(&mut a);
    assert_eq!(l.entries[0].text_key, "legacy-stepped_away");
    // Time no longer passes for the player; a second step away is refused.
    assert!(try_send(&mut a, 8, RequestBody::Command(CommandDto::EndTurn)).is_err());
    assert!(try_send(&mut a, 9, RequestBody::Command(CommandDto::StepAway)).is_err());

    // The file before the card was asked for: the opening is not rated yet.
    let before = save_of(&a);
    assert!(log_of(&a)["opening_rating"].is_null());

    let c = card(&mut a).expect("a card once the act is over");
    assert!(c.unfinished);
    assert_eq!(
        c.ending.as_ref().map(|e| e.id.as_str()),
        Some("ending.stepped_away")
    );
    assert!(!c.name.is_empty() && !c.town.is_empty());
    let r = c.opening.rating.as_ref().expect("the opening is rated");
    assert!(r.plays >= 1 && r.wins <= r.plays);
    assert!(r.facts.len() <= 3);
    assert!(c.opening.post.is_some());
    assert!(c.highest_post.is_some());
    assert!(c.races.is_empty(), "{:?}", c.races);
    assert_eq!(c.figure.rated, 0);
    let code = c.code.clone().expect("a new game's card has a code");
    assert!(code.starts_with("M1."));
    assert_in_locale(&a, &c);
    // Kept in the run file.
    assert!(log_of(&a)["opening_rating"].is_object());

    // Through a save: the same card, rated or not when it was saved.
    let mut b = Session::new().unwrap();
    b.import_save(&save_of(&a)).unwrap();
    assert_eq!(card(&mut b).as_ref(), Some(&c));
    let mut d = Session::new().unwrap();
    d.import_save(&before).unwrap();
    assert_eq!(card(&mut d).as_ref(), Some(&c));
    // And asked again.
    assert_eq!(card(&mut a).as_ref(), Some(&c));
    fixture("first_turn", &c);

    // The code read back.
    let w = match send(
        &mut a,
        10,
        RequestBody::Query(QueryDto::WorldCode { code: code.clone() }),
    ) {
        ResponseBody::WorldCode(w) => w,
        b => panic!("{b:?}"),
    };
    let world = w.world.as_ref().expect("the card's own code reads");
    assert!(world.same_build);
    assert_eq!(world.seed, SEED);
    assert_eq!(world.size, WorldSizeDto::Small);
    assert!(world.opening.is_none());
    assert_eq!(world.code, code);
    fixture("code", &w);
    let bad = match send(
        &mut a,
        11,
        RequestBody::Query(QueryDto::WorldCode {
            code: "M1.not.a.code".into(),
        }),
    ) {
        ResponseBody::WorldCode(w) => w,
        b => panic!("{b:?}"),
    };
    assert_eq!(bad.error.as_deref(), Some("malformed"));
}

/// A code round-trips: from a drafted opening's card, a game started from
/// the code is the same game, to its first turn.
#[test]
fn a_world_code_starts_the_same_opening() {
    let mut a = new_game(SEED);
    let d = match send(&mut a, 2, RequestBody::Query(QueryDto::Draft)) {
        ResponseBody::Draft(d) => d,
        b => panic!("{b:?}"),
    };
    assert!(!d.openings.is_empty(), "seed {SEED} drafts nothing");
    send(&mut a, 3, RequestBody::ChooseOpening { index: 0 });
    let start = a.engine().unwrap().state_hash();
    command(&mut a, CommandDto::EndTurn);
    let first = a.engine().unwrap().state_hash();
    command(&mut a, CommandDto::StepAway);
    let c = card(&mut a).unwrap();
    let code = c.code.clone().unwrap();
    let opening = d.openings[0].clone();
    assert_eq!(c.opening.town, opening.town_name);
    assert_eq!(
        c.opening.arc.as_ref().map(|t| t.key.as_str()),
        Some(opening.title_key.as_str())
    );
    assert_eq!(c.opening.months_ahead, d.months_ahead);
    assert_in_locale(&a, &c);

    let w = match send(
        &mut a,
        10,
        RequestBody::Query(QueryDto::WorldCode { code: code.clone() }),
    ) {
        ResponseBody::WorldCode(w) => w.world.unwrap(),
        b => panic!("{b:?}"),
    };
    let o = w.opening.as_ref().expect("the code carries the opening");
    assert_eq!(o.town, opening.town);
    assert_eq!(o.title_key, opening.title_key);
    assert_eq!(o.months_ahead, d.months_ahead);

    // Started from the code: the same world, the same first turn.
    let mut b = Session::new().unwrap();
    match send(&mut b, 1, RequestBody::StartFromCode { code: code.clone() }) {
        ResponseBody::GameStarted(_) => {}
        x => panic!("{x:?}"),
    }
    assert_eq!(b.engine().unwrap().state_hash(), start);
    command(&mut b, CommandDto::EndTurn);
    assert_eq!(b.engine().unwrap().state_hash(), first);
    // Its own run file carries the opening, and so its own code is the same.
    command(&mut b, CommandDto::StepAway);
    assert_eq!(card(&mut b).unwrap().code.as_deref(), Some(code.as_str()));
    // A mistyped code starts nothing.
    let mut typo = code.clone().into_bytes();
    let i = typo.len() - 1;
    typo[i] = if typo[i] == b'0' { b'1' } else { b'0' };
    let mut e = Session::new().unwrap();
    assert!(
        try_send(
            &mut e,
            1,
            RequestBody::StartFromCode {
                code: String::from_utf8(typo).unwrap()
            }
        )
        .is_err()
    );
}

/// The seed whose campaign debate the scene tests play (`scene_views.rs`).
const SCENE_SEED: u64 = 4242;

/// Mid-race and mid-scene: in the campaign, a round into the debate, the
/// player steps away. The race was rated as it opened; the card names it
/// walked away from, with that rating, and counts it in neither wins nor
/// expected wins.
#[test]
fn stepping_away_mid_race_and_mid_scene() {
    let mut s = new_game(SCENE_SEED);
    let mut found = None;
    for _ in 0..80 {
        let p = pocket(&mut s);
        if let Some(n) = p
            .needs_you
            .iter()
            // The campaign's debate (another arc's scene may come first).
            .find(|n| {
                n.kind == NeedKind::Scene
                    && matches!(&n.target, NeedTargetDto::Scene { beat, .. } if beat == "showdown.debate")
            })
            .map(|n| n.target.clone())
        {
            found = Some(n);
            break;
        }
        command(&mut s, CommandDto::EndTurn);
    }
    let Some(NeedTargetDto::Scene { arc, beat }) = found else {
        panic!("no scene in eighty turns");
    };
    let v = match send(
        &mut s,
        3,
        RequestBody::Query(QueryDto::ArcExchange {
            arc,
            beat: beat.clone(),
        }),
    ) {
        ResponseBody::ArcExchange(v) => v,
        b => panic!("{b:?}"),
    };
    let to = v.route.as_ref().and_then(|r| r.to.clone()).unwrap();
    command(
        &mut s,
        CommandDto::Go {
            location: to.location,
            then: Some(Box::new(CommandDto::ArcExchange {
                arc,
                beat: beat.clone(),
                act: SceneActDto::Begin,
            })),
        },
    );
    let v = match send(
        &mut s,
        3,
        RequestBody::Query(QueryDto::ArcExchange {
            arc,
            beat: beat.clone(),
        }),
    ) {
        ResponseBody::ArcExchange(v) => v,
        b => panic!("{b:?}"),
    };
    assert_eq!(v.phase, "under_way");
    command(
        &mut s,
        CommandDto::ArcExchange {
            arc,
            beat: beat.clone(),
            act: SceneActDto::Say {
                option: v.options[0].id.clone(),
            },
        },
    );
    // In the campaign: the race the player stands in was rated as it opened.
    let open = mandate_sim::career::races_open(s.engine().unwrap().state());
    assert!(!open.is_empty(), "the debate is a campaign's");
    let ratings = log_of(&s)["ratings"].clone();
    assert!(
        ratings.as_array().is_some_and(|r| !r.is_empty()),
        "{ratings}"
    );

    command(&mut s, CommandDto::StepAway);
    let l = legacy(&mut s);
    assert_eq!(l.entries[0].text_key, "legacy-stepped_away-race");
    let c = card(&mut s).unwrap();
    assert!(c.unfinished);
    let walked: Vec<&CardRaceView> = c
        .races
        .iter()
        .filter(|r| r.outcome == "walked_away")
        .collect();
    assert_eq!(walked.len(), 1, "{:?}", c.races);
    assert!(walked[0].rating.is_some());
    assert!(walked[0].share_pm.is_none());
    // Counted in neither: the figure is over the races decided.
    let decided = c
        .races
        .iter()
        .filter(|r| r.rating.is_some() && (r.outcome == "won" || r.outcome == "lost"))
        .count() as u32;
    assert_eq!(c.figure.rated, decided);
    assert_eq!(
        c.figure.over_pm,
        c.figure.wins as i32 * 1000 - c.figure.expected_pm as i32
    );
    // The matter under way is on the card, as under way.
    assert!(c.arcs.iter().any(|a| a.under_way), "{:?}", c.arcs);
    assert_in_locale(&s, &c);
    // The same card through a save.
    let mut b = Session::new().unwrap();
    b.import_save(&save_of(&s)).unwrap();
    assert_eq!(card(&mut b).as_ref(), Some(&c));
    fixture("mid_race", &c);
}
