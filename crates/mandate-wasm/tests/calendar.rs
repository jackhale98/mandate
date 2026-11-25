//! The town calendar and familiarity (task B1, DEC-114): one date per turn
//! (the bar, people's routines and the fortnight's events agree in every
//! turn), an event held with its people there and its place full, absence
//! costing only where the player is expected and shown a turn ahead,
//! familiarity built by presence and shown only in words, and all of it
//! the same through a save.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_content::ContentBundle;
use mandate_content::calendar::{CalendarAbsent, CalendarExpected};
use mandate_content::routines::DayPart;
use mandate_protocol::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::characters::calendar::{self, Span};
use mandate_sim::characters::routines::{self, Now};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::events::EventKind;
use mandate_sim::places::Location;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeMap;
use std::sync::Arc;

fn base() -> ContentBundle {
    mandate_content::load(&mandate_content::embedded::sources())
        .unwrap()
        .0
}

fn game_with(seed: u64, c: Arc<ContentBundle>) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c).unwrap()
}

fn game(seed: u64) -> Engine {
    game_with(seed, Arc::new(base()))
}

fn me(e: &Engine) -> CharacterId {
    characters::player_character(e.state()).unwrap()
}

fn go(e: &mut Engine, loc: Location) {
    e.apply(Command::Player(PlayerCommand::MoveTo { location: loc }))
        .unwrap();
}

/// Spend slots until the player's part of the day is `part`.
fn reach_part(e: &mut Engine, part: DayPart) {
    let cap = e.state().player.as_ref().unwrap().slot_cap();
    for used in 0..=cap {
        e.setup(|st| st.player.as_mut().unwrap().slots_used = used);
        if routines::day_part(e.state()) == part {
            return;
        }
    }
    panic!("no slot count gives {part:?}");
}

#[test]
fn one_date_per_turn_for_two_years_on_eight_seeds() {
    let c = Arc::new(base());
    let mut table: BTreeMap<u64, BTreeMap<String, (u32, u32)>> = BTreeMap::new();
    for seed in 1..=8u64 {
        let mut e = game_with(seed, c.clone());
        let until = e.state().calendar.date.add_days(2 * 365);
        let mut turns = 0;
        while e.state().calendar.date < until {
            let st = e.state();
            let ov = overview(&e);
            let now = Now::of(st, &c);
            let day = st.calendar.day;
            let span = Span::now(st, &c);
            assert!(span.contains(day), "seed {seed}: the day is in the turn");
            // The bar, people's routines and the stored day agree.
            assert_eq!(ov.day.day_number, day.0, "seed {seed}: the bar's day");
            assert_eq!(now.day, day, "seed {seed}: routines' day");
            assert_eq!(
                ov.weekday_key,
                format!("weekday-{}", mandate_content::routines::RoutineDay::of(day).key())
            );
            // Every event held this turn is held on that day; one reported
            // fell in the turn on its own date.
            let pocket = pocket_view(&e).unwrap();
            let mut parts = Vec::new();
            for ev in &pocket.fortnight {
                let t = table.entry(seed).or_default().entry(ev.id.clone()).or_default();
                if ev.today {
                    assert_eq!(ev.date.day_number, day.0, "seed {seed}: {} on the day", ev.id);
                    let p = ev.part.clone().unwrap();
                    parts.push(p);
                    t.0 += 1;
                } else {
                    assert!(span.contains(mandate_core::SimDate(ev.date.day_number)));
                    t.1 += 1;
                }
            }
            // One event a part, except a sitting, which keeps its part.
            let mut dedup = parts.clone();
            dedup.sort();
            dedup.dedup();
            let sittings = pocket
                .fortnight
                .iter()
                .filter(|x| x.id == "event.council_night" && x.today)
                .count();
            assert!(parts.len() - dedup.len() <= sittings, "seed {seed}: {parts:?}");
            for h in &now.held {
                assert_eq!(h.on, day);
            }
            // The day is the same however often it is asked, and after a
            // save round trip.
            let again = Engine::from_state(st.clone(), e.content_arc()).unwrap();
            assert_eq!(again.state().calendar.day, day);
            assert_eq!(calendar::this_turn(again.state(), &c), calendar::this_turn(st, &c));
            e.apply(Command::EndTurn).unwrap();
            turns += 1;
        }
        assert!(turns >= 40, "seed {seed}: two years of turns ({turns})");
    }
    // The events each test world got: held / reported, over two years.
    for (seed, evs) in &table {
        let row: Vec<String> = evs
            .iter()
            .map(|(k, (h, r))| format!("{}={h}/{r}", k.trim_start_matches("event.")))
            .collect();
        eprintln!("calendar seed {seed}: {}", row.join(" "));
    }
    // Every world holds market day or a sitting, and some occasion.
    for (seed, evs) in &table {
        assert!(
            evs.keys().any(|k| k == "event.summer_show" || k == "event.school_term"),
            "seed {seed}: an occasion in two years: {evs:?}"
        );
    }
}

#[test]
fn an_event_is_held_with_its_people_there_and_the_place_full() {
    let c = Arc::new(base());
    let mut checked = 0;
    for seed in [3u64, 11, 29] {
        let mut e = game_with(seed, c.clone());
        // Within a year the town holds an occasion with attendees.
        for _ in 0..30 {
            let held = calendar::this_turn(e.state(), &c);
            let Some(h) = held.iter().find(|h| {
                h.today() && h.def(&c).cadence.occasion() && !h.def(&c).attendees.is_empty()
            }) else {
                e.apply(Command::EndTurn).unwrap();
                continue;
            };
            let h = h.clone();
            let part = h.part.unwrap();
            let def = h.def(&c).clone();
            reach_part(&mut e, part);
            let st = e.state();
            let now = Now::of(st, &c);
            let there = routines::present(st, &c, &now, h.location());
            // The attendees are there: everyone at the venue whom a rule
            // covers, and more than at another part of the day.
            let other = DayPart::ALL
                .iter()
                .copied()
                .find(|p| *p != part && *p != DayPart::LateNight)
                .unwrap();
            let before = routines::present(st, &c, &Now::at(st, &c, other), h.location());
            assert!(
                there.len() > before.len(),
                "seed {seed}: {} is fuller in its part ({} vs {})",
                def.id,
                there.len(),
                before.len()
            );
            // Its crowd: more passers-by than the place has otherwise.
            let crowd = characters::ambient::ambient_at(st, &c, h.location()).len();
            assert!(crowd >= def.crowd as usize, "seed {seed}: a crowd at {}", def.id);
            // Today lists it with its place, part and a way there; going
            // there is presence.
            let v = pocket_view(&e)
                .unwrap()
                .fortnight
                .into_iter()
                .find(|x| x.id == def.id)
                .unwrap();
            assert!(v.today && v.now && !v.here);
            go(&mut e, h.location());
            let v = pocket_view(&e)
                .unwrap()
                .fortnight
                .into_iter()
                .find(|x| x.id == def.id)
                .unwrap();
            assert!(v.here && v.attended, "seed {seed}: being there is attending");
            // The turn's passage tells it, with its date.
            e.apply(Command::EndTurn).unwrap();
            let lines = overview(&e).passage;
            let line = lines
                .iter()
                .find(|l| l.args.get("event_key") == Some(&h.name_key))
                .expect("the passage tells the event");
            assert_eq!(line.text_key, "event-calendar-attended");
            assert_eq!(line.args["day"], h.on.calendar().day.to_string());
            checked += 1;
            break;
        }
    }
    assert!(checked >= 2, "occasions held in the test worlds: {checked}");
}

/// The base pack, with market day expected of the player's role word and
/// a small cost for missing it.
fn expected_market(word: &str) -> ContentBundle {
    let mut c = base();
    let m = c
        .calendar
        .iter_mut()
        .find(|x| x.id == "event.market_day")
        .unwrap();
    m.expected = vec![CalendarExpected::Role { word: word.into() }];
    m.absent = Some(CalendarAbsent { opinion: -3.0 });
    c
}

fn player_word(e: &Engine) -> String {
    let st = e.state();
    let con = e.content().constitutions.get(&st.meta.constitution).unwrap();
    mandate_sim::player::posts(st)[0]
        .word(con)
        .unwrap()
        .to_string()
}

#[test]
fn absence_costs_only_where_expected_and_is_shown_a_turn_ahead() {
    for seed in [5u64, 17] {
        let probe = game(seed);
        let word = player_word(&probe);
        let c = Arc::new(expected_market(&word));
        let e = game_with(seed, c.clone());
        let held = calendar::this_turn(e.state(), &c);
        let Some(market) = held.iter().find(|h| h.def(&c).id == "event.market_day" && h.today())
        else {
            panic!("seed {seed}: no market held this turn: {held:?}");
        };
        // At most one expected event a turn, and this one is it.
        assert_eq!(held.iter().filter(|h| h.expected).count(), 1);
        assert!(market.expected);
        // Shown a turn ahead, with its cost.
        let next = pocket_view(&e).unwrap().next_expected.expect("shown a turn ahead");
        assert_eq!(next.id, "event.market_day");
        assert!(next.cost.is_some());
        // Missed: those who were there think a little less of the player.
        let part = market.part.unwrap();
        let st = e.state();
        let there: Vec<CharacterId> =
            routines::present(st, &c, &Now::at(st, &c, part), market.location())
                .into_iter()
                .filter(|x| *x != me(&e))
                .collect();
        assert!(!there.is_empty(), "seed {seed}: people at the market");
        let before: Vec<f32> = there
            .iter()
            .map(|x| characters::opinion(&st.politics, *x, me(&e)))
            .collect();
        let mut away = st.clone();
        calendar::end_turn(&mut away, &c);
        let after: Vec<f32> = there
            .iter()
            .map(|x| characters::opinion(&away.politics, *x, me(&e)))
            .collect();
        assert!(
            before.iter().zip(&after).all(|(b, a)| a < b),
            "seed {seed}: absence noticed: {before:?} -> {after:?}"
        );
        assert!(away.events.iter().any(|ev| matches!(
            &ev.kind,
            EventKind::CalendarEvent { missed: true, .. }
        )));
        // Being there costs nothing.
        let mut e2 = game_with(seed, c.clone());
        reach_part(&mut e2, part);
        go(&mut e2, market.location());
        let mut came = e2.state().clone();
        let mid: Vec<f32> = there
            .iter()
            .map(|x| characters::opinion(&came.politics, *x, me(&e)))
            .collect();
        calendar::end_turn(&mut came, &c);
        let after: Vec<f32> = there
            .iter()
            .map(|x| characters::opinion(&came.politics, *x, me(&e)))
            .collect();
        assert_eq!(mid, after, "seed {seed}: attending costs nothing");
        // Where nobody expects the player (the base pack), nothing is lost.
        let plain = Arc::new(base());
        let e3 = game_with(seed, plain.clone());
        assert!(calendar::this_turn(e3.state(), &plain).iter().all(|h| !h.expected
            || h.def(&plain).absent.is_some()));
        let mut free = e3.state().clone();
        let before3: Vec<f32> = there
            .iter()
            .map(|x| characters::opinion(&free.politics, *x, me(&e3)))
            .collect();
        let market_expected = calendar::this_turn(&free, &plain)
            .iter()
            .any(|h| h.expected);
        calendar::end_turn(&mut free, &plain);
        let after3: Vec<f32> = there
            .iter()
            .map(|x| characters::opinion(&free.politics, *x, me(&e3)))
            .collect();
        if !market_expected {
            assert_eq!(before3, after3, "seed {seed}: no cost where not expected");
        }
    }
}

#[test]
fn at_most_one_expected_event_a_turn() {
    let probe = game(9);
    let word = player_word(&probe);
    let mut c = expected_market(&word);
    for d in c.calendar.iter_mut() {
        d.expected = vec![CalendarExpected::Role { word: word.clone() }];
        d.absent = Some(CalendarAbsent { opinion: -1.0 });
    }
    let c = Arc::new(c);
    let mut e = game_with(9, c.clone());
    for _ in 0..12 {
        let held = calendar::this_turn(e.state(), &c);
        assert!(held.iter().filter(|h| h.expected).count() <= 1, "{held:?}");
        e.apply(Command::EndTurn).unwrap();
    }
}

#[test]
fn familiarity_rises_with_presence_and_shows_only_in_words() {
    let c = Arc::new(base());
    let mut e = game_with(7, c.clone());
    let me = me(&e);
    // Someone stored, where they are now.
    let town = e.state().player.as_ref().unwrap().home();
    let (who, at) = routines::placements(e.state(), &c, &Now::of(e.state(), &c), town)
        .into_iter()
        .find(|(id, l)| *id != me && matches!(l, Location::Place { .. }))
        .unwrap();
    let fam = |e: &Engine| characters::familiar::familiar(&e.state().politics, who, me);
    assert_eq!(fam(&e), 0);
    assert_eq!(familiar_key(&e, who), None, "a stranger has no words");
    go(&mut e, at);
    let one = fam(&e);
    assert!(one >= 1, "presence counts");
    // Once a turn a person.
    go(&mut e, Location::Home);
    go(&mut e, at);
    assert_eq!(fam(&e), one, "once a turn");
    // Turn by turn, where they are, until they know the player's face.
    let face = c.characters.familiarity.face;
    for _ in 0..8 {
        if fam(&e) >= face {
            break;
        }
        e.apply(Command::EndTurn).unwrap();
        e.setup(|st| st.player.as_mut().unwrap().slots_used = 0);
        if let Some(l) = routines::whereabouts(e.state(), &c, who) {
            go(&mut e, l);
        }
    }
    assert!(fam(&e) >= face, "familiar after a few turns: {}", fam(&e));
    let key = familiar_key(&e, who).expect("words once familiar");
    assert!(key.starts_with("familiar-"));
    // Words only: the views carry the key and never the number.
    let sheet = character_view(&e, who.0).unwrap();
    assert_eq!(sheet.familiar_key.as_deref(), Some(key.as_str()));
    let json = serde_json::to_string(&sheet).unwrap();
    assert!(!json.contains("\"familiar\""), "no count in the view: {json}");
    let people = serde_json::to_string(&people_view(&e).unwrap()).unwrap();
    assert!(!people.contains("\"familiar\""));
    // Kept through a save.
    let req = mandate_save::SaveRequest {
        run_id: "00112233445566778899aabbccddeeff".into(),
        revision: 1,
        parent_revision: None,
        device_id: "test".into(),
        saved_at: "2026-01-01T00:00:00Z".into(),
        protocol_version: 1,
        run_mode: mandate_save::RunMode::Ironman,
    };
    let (bytes, _) = mandate_save::encode(e.state(), &req).unwrap();
    let (st, _) = mandate_save::decode(&bytes).unwrap();
    let back = Engine::from_state(st, e.content_arc()).unwrap();
    assert_eq!(fam(&back), fam(&e));
    assert_eq!(back.state_hash(), e.state_hash());
}

#[test]
fn a_familiar_face_greets_the_player_by_name() {
    let c = Arc::new(base());
    let mut familiar_greetings = 0;
    let mut stranger_familiar = 0;
    for seed in [3u64, 17, 41, 58] {
        let mut e = game_with(seed, c.clone());
        let me = me(&e);
        let town = e.state().player.as_ref().unwrap().home();
        let folk = routines::townsfolk(e.state(), town);
        for who in folk.iter().copied().filter(|x| *x != me).take(12) {
            let g = mandate_sim::talk::greeting(e.state(), &c, who);
            if g.is_some_and(|g| g.line.starts_with("line.open.familiar")) {
                stranger_familiar += 1;
            }
            let nod = c.characters.familiarity.nod;
            e.setup(|st| {
                let edge = st.politics.characters.edges.entry((who.0, me.0)).or_insert(
                    characters::Relationship {
                        opinion: 10.0,
                        trust: 0.3,
                        bonds: Vec::new(),
                        familiar: 0,
                    },
                );
                edge.familiar = nod;
                edge.opinion = edge.opinion.max(0.0);
            });
            if mandate_sim::talk::greeting(e.state(), &c, who)
                .is_some_and(|g| g.line.starts_with("line.open.familiar"))
            {
                familiar_greetings += 1;
            }
        }
    }
    assert_eq!(stranger_familiar, 0, "strangers are not greeted as familiar");
    assert!(familiar_greetings >= 10, "familiar faces greet by name: {familiar_greetings}");
}
