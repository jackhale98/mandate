#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Asks and promises for the client (E7; DEC-097 §2, ENGAGEMENT §3.9), and
//! the time skip's summary facts:
//!
//! - the pocket lists offers first, then accepted asks by due date, with the
//!   next need, its route, the casework odds and what keeping or breaking
//!   it does; the badge counts offers plus asks due within the turn;
//! - accepting, declining and keeping an ask show in the views (kept asks
//!   in the last outcomes); the outcome's toast has the ask's arguments;
//! - the promise menu while campaigning, and the promises on record;
//! - a skip in which a law lapses lists it ("Nothing of note" only when
//!   nothing happened);
//! - Rust ⇄ TS: the web fixture's views and commands (the shapes the
//!   generated TS types describe) deserialize and serialize back unchanged.

use mandate_protocol::*;
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::asks::{Ask, AskId, AskStatus, Asker, History, Need};
use mandate_sim::narrative::{Actor, CastEntry};
use mandate_sim::{Command, Engine};
use mandate_wasm::session::Session;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn content() -> Arc<mandate_content::ContentBundle> {
    let (c, _) = mandate_content::load(&mandate_content::embedded::sources()).unwrap();
    Arc::new(c)
}

fn engine(c: &Arc<mandate_content::ContentBundle>, seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

/// Put an ask before the player: a casework ask from a Named resident of
/// the home town, offered or accepted.
fn put(e: &mut Engine, needs: Vec<Need>, due_in: u32, offered: bool) -> AskId {
    let date = e.state().calendar.date;
    let home = mandate_sim::player::home(e.state()).map(|m| m.0);
    let st = e.state();
    let asker = st
        .politics
        .characters
        .people
        .iter()
        .filter(|c| c.named.is_some())
        .find(|c| mandate_sim::characters::municipality_of(&st.politics, &st.world, c.id) == home)
        .map(|c| c.id)
        .unwrap();
    let hood = st
        .world
        .neighborhoods
        .iter()
        .find(|h| Some(h.municipality.0) == home)
        .unwrap()
        .id;
    let mut id = AskId(0);
    e.setup(|st| {
        id = st.asks.ids.alloc();
        st.asks.asks.push(Ask {
            id,
            template: "ask.casework".into(),
            promise: false,
            asker: Asker::Person { id: asker },
            cast: vec![
                CastEntry {
                    role: "asker".into(),
                    actor: Actor::Person { id: asker },
                },
                CastEntry {
                    role: "asker_hood".into(),
                    actor: Actor::Neighborhood { id: hood.0 },
                },
            ],
            needs,
            step: 0,
            done: Vec::new(),
            public: false,
            big: false,
            opened: date,
            respond_by: date.add_days(7),
            due: date.add_days(due_in),
            status: if offered {
                AskStatus::Offered
            } else {
                AskStatus::Accepted { on: date }
            },
            history: History::None,
            race: None,
            answer: None,
            tries: 0,
        });
    });
    id
}

fn pocket(e: &Engine) -> PocketView {
    pocket_view(e).unwrap()
}

fn find(p: &PocketView, id: AskId) -> AskView {
    p.asks
        .iter()
        .chain(&p.asks_recent)
        .find(|a| a.id == id.0)
        .cloned()
        .unwrap_or_else(|| panic!("ask {} not in the pocket", id.0))
}

#[test]
fn the_pocket_lists_offers_then_accepted_asks_with_their_next_step() {
    let c = content();
    let mut e = engine(&c, 6);
    // Clear what the start offered, to count exactly.
    e.setup(|st| st.asks.asks.clear());
    let later = put(&mut e, vec![Need::Casework], 60, false);
    let soon = put(&mut e, vec![Need::Casework], 2, false);
    let offer = put(&mut e, vec![Need::Casework], 30, true);
    let p = pocket(&e);
    let ids: Vec<u32> = p.asks.iter().map(|a| a.id).collect();
    assert_eq!(
        ids,
        vec![offer.0, soon.0, later.0],
        "offers first, then by due date"
    );
    // The badge: the offer, and the ask due within the turn.
    assert_eq!(p.asks_due_soon, 2);
    let o = find(&p, offer);
    assert_eq!(o.status, "offered");
    assert!(o.respond_by.is_some() && o.odds.is_none());
    assert_eq!(o.days_left, 7);
    assert_eq!(o.title_key, "ask-casework");
    assert!(o.args.contains_key("asker") && o.args.contains_key("asker_hood"));
    assert!(matches!(o.asker, AskerView::Person { .. }));
    assert!(!o.keep_lines.is_empty() && !o.break_lines.is_empty());
    let a = find(&p, soon);
    assert_eq!(a.status, "accepted");
    assert!(a.respond_by.is_none() && a.due_this_turn);
    // Casework: the engine's odds and where it is done.
    assert_eq!(
        a.odds,
        Some(mandate_sim::asks::casework_odds(e.state(), &c))
    );
    let how = a.how.clone().unwrap();
    assert_eq!(how.action, "casework");
    assert_eq!(how.to.unwrap().kind_key, "town_hall");
    assert_eq!(a.needs.len(), 1);
    assert_eq!(a.needs[0].state, "current");
    assert!(!find(&p, later).due_this_turn);
}

#[test]
fn accept_decline_and_keep_show_in_the_views() {
    let c = content();
    let mut e = engine(&c, 6);
    e.setup(|st| st.asks.asks.clear());
    let yes = put(&mut e, vec![Need::Casework], 30, true);
    let no = put(&mut e, vec![Need::Casework], 30, true);
    e.apply(Command::Player(PlayerCommand::RespondAsk {
        ask: yes.0,
        accept: true,
    }))
    .unwrap();
    e.apply(Command::Player(PlayerCommand::RespondAsk {
        ask: no.0,
        accept: false,
    }))
    .unwrap();
    let p = pocket(&e);
    assert_eq!(find(&p, yes).status, "accepted");
    let d = find(&p, no);
    assert_eq!((d.status.as_str(), d.unanswered), ("declined", false));
    assert!(
        p.asks_recent.iter().any(|a| a.id == no.0),
        "a declined ask is a recent outcome"
    );
    // Keep it: casework at the town hall until it is sorted.
    let mut tries = 0;
    while e.state().asks.get(yes).unwrap().open() && tries < 12 {
        if e.state().player.as_ref().unwrap().slots_left() == 0 {
            e.apply(Command::EndTurn).unwrap();
        }
        let cmd = e.route(PlayerCommand::Act {
            action: Action::Casework { ask: yes.0 },
            late_night: false,
        });
        e.apply(cmd).unwrap();
        e.apply(Command::AdvanceDays { days: 1 }).unwrap();
        tries += 1;
    }
    let p = pocket(&e);
    assert!(p.asks.iter().all(|a| a.id != yes.0), "no longer open");
    let k = find(&p, yes);
    assert_eq!(k.status, "kept");
    assert_eq!(k.outcome_key.as_deref(), Some("ask-casework-kept"));
    assert!(k.resolved_on.is_some() && k.needs.iter().all(|n| n.state == "done"));
    let v = asks_view(&e).unwrap();
    assert!(v.tally.kept >= 1 && v.tally.accepted >= 1 && v.tally.declined >= 1);
    assert!(v.recent.iter().any(|a| a.id == yes.0 && a.status == "kept"));
}

/// The asks' own commands through the protocol (the web client's path):
/// the toast's arguments name the ask's cast.
#[test]
fn ask_commands_through_the_protocol() {
    let mut s = Session::new().unwrap();
    let send = |s: &mut Session, body: RequestBody| -> Result<ResponseBody, ProtocolError> {
        let j = serde_json::to_string(&Request {
            protocol_version: PROTOCOL_VERSION,
            request_id: 1,
            body,
        })
        .unwrap();
        let r: Response = serde_json::from_str(&s.handle_json(&j)).unwrap();
        match r.result {
            ResponseResult::Ok { body } => Ok(body),
            ResponseResult::Err { error } => Err(error),
        }
    };
    send(
        &mut s,
        RequestBody::NewGame(NewGameParams {
            seed: 6,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
        }),
    )
    .unwrap();
    // Wait for an offer.
    let mut offer = None;
    for _ in 0..60 {
        let Ok(ResponseBody::Pocket(p)) = send(&mut s, RequestBody::Query(QueryDto::Pocket)) else {
            panic!("no pocket");
        };
        offer = p.asks.iter().find(|a| a.status == "offered").cloned();
        if offer.is_some() {
            assert!(p.asks_due_soon >= 1);
            break;
        }
        send(&mut s, RequestBody::Command(CommandDto::EndTurn)).unwrap();
    }
    let offer = offer.expect("an offer within 60 turns");
    let Ok(ResponseBody::CommandApplied(c)) = send(
        &mut s,
        RequestBody::Command(CommandDto::RespondAsk {
            ask: offer.id,
            accept: true,
        }),
    ) else {
        panic!("accepting failed");
    };
    let r = c.result.unwrap();
    assert_eq!(r.outcome_key, "outcome-ask-accepted");
    assert_eq!(r.args.get("ask_key"), Some(&offer.title_key));
    for k in offer.args.keys().filter(|k| *k != "days") {
        assert!(r.args.contains_key(k), "the toast lacks `{k}`");
    }
    // Answered once only.
    assert!(
        send(
            &mut s,
            RequestBody::Command(CommandDto::RespondAsk {
                ask: offer.id,
                accept: false,
            }),
        )
        .is_err()
    );
    let Ok(ResponseBody::Asks(Some(v))) = send(&mut s, RequestBody::Query(QueryDto::Asks)) else {
        panic!("no asks view");
    };
    assert!(
        v.open
            .iter()
            .any(|a| a.id == offer.id && a.status == "accepted")
    );
    // No promises outside a campaign.
    let Ok(ResponseBody::PromiseMenu(m)) = send(&mut s, RequestBody::Query(QueryDto::PromiseMenu))
    else {
        panic!("no menu");
    };
    assert!(m.options.is_empty());
    assert!(
        send(
            &mut s,
            RequestBody::Command(CommandDto::Promise {
                template: "promise.town_halls".into(),
                answer: None,
            }),
        )
        .is_err()
    );
}

#[test]
fn promises_are_picked_while_campaigning_and_kept_on_record() {
    let c = content();
    let mut e = engine(&c, 2);
    assert!(!pocket(&e).can_promise);
    while !mandate_sim::characters::is_candidate(
        &e.state().politics,
        e.state().player.as_ref().unwrap().politician,
    ) {
        e.apply(Command::EndTurn).unwrap();
    }
    let m = promise_menu_view(&e);
    assert!(pocket(&e).can_promise && !m.options.is_empty());
    assert_eq!((m.made, m.max), (0, 3));
    match m.voters.as_ref().unwrap() {
        AskerView::Constituency { name_key, .. } => {
            assert_eq!(name_key, "ask-constituency");
            assert!(m.voters_args.contains_key("number"));
        }
        AskerView::Electorate { name_key, .. } => assert_eq!(name_key, "ask-electorate"),
        other => panic!("promises are made to voters, not {other:?}"),
    }
    let o = m
        .options
        .iter()
        .find(|o| o.template == "promise.town_halls")
        .unwrap();
    assert_eq!(o.title_key, "promise-town-halls");
    assert!(o.args.contains_key("constituency_key"));
    assert!(!o.keep_lines.is_empty() && !o.break_lines.is_empty());
    e.apply(Command::Player(PlayerCommand::Promise {
        template: o.template.clone(),
        args: vec![],
    }))
    .unwrap();
    let m = promise_menu_view(&e);
    assert_eq!(m.made, 1);
    assert!(
        m.options.iter().all(|x| x.template != "promise.town_halls"),
        "made once a race"
    );
    let v = asks_view(&e).unwrap();
    assert_eq!(v.promises.len(), 1);
    let p = &v.promises[0];
    assert!(p.promise && p.public && p.status == "accepted");
    assert_eq!(v.tally.promises_made, 1);
    // A promise to hold a meeting in each neighbourhood shows how far along it is.
    let in_pocket = pocket(&e).asks.into_iter().find(|a| a.id == p.id).unwrap();
    let total = match &e.state().asks.get(AskId(p.id)).unwrap().needs[0] {
        Need::TownHall { hoods } => hoods.len() as u32,
        _ => 0,
    };
    if total > 1 {
        let pr = in_pocket.progress.unwrap();
        assert_eq!((pr.done, pr.total), (0, total));
        assert_eq!(in_pocket.how.unwrap().action, "town_hall");
    }
}

/// Whenever the laws in force fall over a skip, a line says which law ended
/// (the owner's report: "Laws in force 5 → 4" under "Nothing of note").
#[test]
fn laws_in_force_never_fall_unexplained() {
    let c = content();
    for seed in [14, 4, 20] {
        let mut e = engine(&c, seed);
        e.apply(Command::EndTurn).unwrap();
        e.apply(Command::Skip {
            target: mandate_content::act::SkipTargetKind::NextSession,
            limit: None,
        })
        .unwrap();
        let s = skip_summary_view(&e).unwrap();
        let laws = s.changes.iter().find(|x| x.figure == "laws").unwrap();
        let ended = s
            .items
            .iter()
            .filter(|i| {
                i.text_key == "skip-item-law-expired" || i.text_key == "skip-item-law-repealed"
            })
            .count() as i64;
        assert!(
            laws.before - laws.after <= ended,
            "seed {seed}: {laws:?} with {ended} ended"
        );
        // Seed 14 (the web test's): a law lapses, named in its line. It was
        // seed 2 until the party line changed which laws pass in the burn-in,
        // then 5 until rents were bounded by incomes, then 9 until arrivals
        // were shared fairly over neighbourhoods (audit fix 2); 14 is the
        // first seed after 9 with a lapsing law (0 is avoided: "0" is falsy
        // in web code).
        if seed == 14 {
            let line = s
                .items
                .iter()
                .find(|i| i.text_key == "skip-item-law-expired");
            assert!(
                line.is_some_and(|l| l.args.get("policy_key").is_some_and(|k| !k.is_empty())),
                "seed 14: a lapsed law is listed: {:?}",
                s.items
            );
        }
        assert!(!s.items.is_empty(), "seed {seed}: something happened");
    }
}

/// Every summary line has English text with the arguments it names.
#[test]
fn every_summary_item_key_has_english_text() {
    let c = content();
    let en = c.locale_text("en");
    for key in [
        "skip-item-law-expired",
        "skip-item-law-repealed",
        "skip-item-bill-withdrawn",
        "skip-item-ask-offered",
        "skip-item-ask-kept",
        "skip-item-ask-broken",
        "skip-item-ask-void",
        "skip-item-ask-declined",
        "skip-item-ask-lapsed",
        "skip-item-crisis-began",
        "skip-item-crisis-ended",
        "skip-item-deselected",
        "skip-item-case",
        "skip-item-imprisoned",
        "skip-item-released",
        "skip-item-rumour",
        "skip-item-recession-began",
        "skip-item-recession-ended",
        "skip-item-proposal-opened",
        "skip-item-proposal-lapsed",
    ] {
        assert!(
            en.contains(&format!("\n{key} =")),
            "no English text for {key}"
        );
    }
}

/// The web's fixture (`web/src/components/asks/asks.fixture.json`): the
/// views and commands the client's tests render and send, in the shape the
/// generated TS types describe. They deserialize into the Rust types and
/// serialize back unchanged. Regenerate it from a game with
/// `MANDATE_UPDATE_FIXTURES=1 cargo test -p mandate-wasm --test asks fixture`.
#[test]
fn the_web_fixture_round_trips() {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Fixture {
        pocket_asks: Vec<AskView>,
        asks_recent: Vec<AskView>,
        asks_due_soon: u32,
        menu: PromiseMenuView,
        asks: AsksView,
        commands: Vec<CommandDto>,
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../web/src/components/asks/asks.fixture.json");
    if std::env::var("MANDATE_UPDATE_FIXTURES").as_deref() == Ok("1") {
        let c = content();
        let mut e = engine(&c, 2);
        e.setup(|st| st.asks.asks.clear());
        while !mandate_sim::characters::is_candidate(
            &e.state().politics,
            e.state().player.as_ref().unwrap().politician,
        ) {
            e.apply(Command::EndTurn).unwrap();
        }
        e.setup(|st| st.asks.asks.retain(|a| a.promise));
        let kept = put(&mut e, vec![Need::Casework], 20, false);
        put(&mut e, vec![Need::Casework], 25, false);
        put(&mut e, vec![Need::Casework], 30, true);
        e.setup(|st| {
            let date = st.calendar.date;
            if let Some(a) = st.asks.asks.iter_mut().find(|a| a.id == kept) {
                a.status = AskStatus::Kept { on: date };
                a.step = 1;
            }
        });
        let menu = promise_menu_view(&e);
        let p = pocket(&e);
        let f = Fixture {
            pocket_asks: p.asks,
            asks_recent: p.asks_recent,
            asks_due_soon: p.asks_due_soon,
            asks: asks_view(&e).unwrap(),
            menu,
            commands: vec![
                CommandDto::RespondAsk {
                    ask: 3,
                    accept: true,
                },
                CommandDto::RespondAsk {
                    ask: 3,
                    accept: false,
                },
                CommandDto::Promise {
                    template: "promise.town_halls".into(),
                    answer: None,
                },
                CommandDto::Act {
                    action: ActionDto::Casework { ask: 2 },
                    late_night: false,
                },
            ],
        };
        let text = serde_json::to_string_pretty(&f).unwrap() + "\n";
        std::fs::write(&path, text).unwrap();
    }
    let text = std::fs::read_to_string(&path).expect("the web fixture");
    let raw: serde_json::Value = serde_json::from_str(&text).unwrap();
    let f: Fixture = serde_json::from_value(raw.clone()).expect("the fixture has the Rust shape");
    assert_eq!(
        serde_json::to_value(&f).unwrap(),
        raw,
        "a round trip changes nothing"
    );
    assert!(f.pocket_asks.iter().any(|a| a.status == "offered"));
    assert!(f.pocket_asks.iter().any(|a| a.odds.is_some()));
    assert!(!f.menu.options.is_empty());
}
