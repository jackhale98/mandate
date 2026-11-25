//! The stand-in players and the skip's limits, in played games
//! (STAGED_ARCS A4; DEC-102 clause 7). What is enforced here is
//! structural, never a balance figure:
//!
//! - a stand-in sends only commands (a game's command log replays to the
//!   same state);
//! - it knows only what a player would (truths it has not learned can be
//!   perturbed without changing a command);
//! - knowing more changes nothing but the choice of commands (the same
//!   commands under an omniscient ledger leave the same world);
//! - a skip learns nothing new, makes no promises and files for no post;
//!   an ask on offer and an arc's stage stop it;
//! - the grader gives the same output twice.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::runner::{Driver, Game, Setup};
use super::seed_reqs::flooded;
use mandate_content::ContentBundle;
use mandate_content::act::SkipTargetKind;
use mandate_content::overrides::Override;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::player::knowledge::Source;
use mandate_sim::skip::SkipStop;
use mandate_sim::standin::{self, Kind, Seat};
use mandate_sim::{Command, Engine};
use std::sync::Arc;

fn content() -> Arc<ContentBundle> {
    super::load(&super::runner::content_root(), &[]).unwrap()
}

fn content_with(path: &str, value: serde_json::Value) -> Arc<ContentBundle> {
    super::load(
        &super::runner::content_root(),
        &[Override {
            path: path.into(),
            value,
        }],
    )
    .unwrap()
}

/// A flooded game (the flood arc gives the stand-ins levers) played by a
/// stand-in.
fn game(seed: u64, kind: Kind) -> Game {
    let mut g = flooded(seed);
    g.set_driver(Driver::standin(kind)).unwrap();
    g
}

/// Replay a command log on a fresh copy of the same start.
fn replay(start: &Engine, log: &[mandate_sim::engine::LoggedCommand]) -> Engine {
    let mut e = Engine::from_state(start.state().clone(), start.content_arc()).unwrap();
    for c in log {
        // A logged command that failed (a `Go` whose move stood) fails
        // again, and its move stands again.
        let _ = e.apply(c.command.clone());
    }
    e
}

#[test]
fn a_stand_in_writes_nothing_but_its_commands() {
    for (seed, kind, turns) in [
        (3, Kind::Engaged, 40),
        (7, Kind::Contrarian, 40),
        (3, Kind::Attends, 12),
        (2, Kind::Passive, 20),
    ] {
        let mut g = game(seed, kind);
        let start = g.fork().unwrap();
        let logged = g.e.command_log().len();
        for _ in 0..turns {
            if !g.step_turn() {
                break;
            }
        }
        let log = &g.e.command_log()[logged..];
        let player = log
            .iter()
            .filter(|c| matches!(c.command, Command::Player(_)))
            .count();
        let skips = log
            .iter()
            .filter(|c| matches!(c.command, Command::Skip { .. }))
            .count();
        println!(
            "seed {seed} {}: {} commands ({player} the player's, {skips} skips)",
            kind.key(),
            log.len()
        );
        match kind {
            Kind::Passive => assert_eq!(player, 0, "the passive stand-in only ends turns"),
            Kind::Attends => assert!(skips > 0, "the attending stand-in skips"),
            _ => assert!(player > 20, "{}: it plays", kind.key()),
        }
        // Everything it did is in the log: the log alone rebuilds the game.
        let again = replay(&start.e, log);
        assert_eq!(
            again.state_hash(),
            g.e.state_hash(),
            "seed {seed} {}: the command log replays to the same state",
            kind.key()
        );
    }
}

/// The commands a stand-in sends in one turn from this game.
fn turn_commands(e: &Engine, kind: Kind) -> Vec<String> {
    let mut g = Engine::from_state(e.state().clone(), e.content_arc()).unwrap();
    let from = g.command_log().len();
    let seat = Seat::of_player(g.state(), g.content()).unwrap();
    standin::play_turn(&mut g, kind, seat);
    g.command_log()[from..]
        .iter()
        .map(|c| serde_json::to_string(&c.command).unwrap())
        .collect()
}

#[test]
fn a_stand_in_is_blind_to_what_the_player_has_not_learned() {
    for (seed, kind) in [(3, Kind::Engaged), (5, Kind::Contrarian)] {
        let mut g = game(seed, kind);
        let (mut perturbed, mut sent) = (0, 0);
        for _ in 0..30 {
            let plain = turn_commands(&g.e, kind);
            let mut p = Engine::from_state(g.e.state().clone(), g.e.content_arc()).unwrap();
            perturbed += crate::bots::perturb(&mut p);
            let blind = turn_commands(&p, kind);
            assert_eq!(
                plain,
                blind,
                "{} (seed {seed}, turn {}) acted on something the player had not learned",
                kind.key(),
                g.e.state().calendar.turn
            );
            sent += plain.len();
            if !g.step_turn() {
                break;
            }
        }
        assert!(perturbed > 0, "nothing was perturbed");
        assert!(sent > 20, "{}: it sent commands", kind.key());
    }
}

#[test]
fn the_same_commands_under_an_omniscient_ledger_leave_the_same_world() {
    // A default stand-in plays; its commands are then given, one by one,
    // to a game whose player reads every subject as known in detail.
    let mut g = game(3, Kind::Engaged);
    let mut start = g.fork().unwrap();
    let logged = g.e.command_log().len();
    for _ in 0..36 {
        if !g.step_turn() {
            break;
        }
    }
    start.e.set_lab_omniscient(true);
    let mut omni = replay(&start.e, &g.e.command_log()[logged..]);
    // The switch itself is a field of the ledger: set alike, then compare.
    g.e.set_lab_omniscient(true);
    omni.set_lab_omniscient(true);
    assert_eq!(
        omni.state_hash(),
        g.e.state_hash(),
        "what the player knows steers the commands a stand-in sends, and nothing else"
    );
}

#[test]
fn the_three_arc_stand_ins_part_ways_in_a_flooded_town() {
    // Coverage of the levers: in one flooded town the engaged stand-in
    // takes beats the passive one never does, and the run records them.
    let st = |kind: Kind| {
        let mut g = game(3, kind);
        for _ in 0..12 {
            g.step_turn();
        }
        let st = g.e.state();
        let run = st
            .arcs
            .runs
            .iter()
            .find(|r| r.template == "arc.flood")
            .map(|r| (r.marks.clone(), r.done.len()));
        let past = st
            .arcs
            .past
            .iter()
            .find(|p| p.template == "arc.flood")
            .map(|p| (p.marks.clone(), 0));
        run.or(past).expect("the flood arc ran")
    };
    let (passive, engaged) = (st(Kind::Passive), st(Kind::Engaged));
    println!("passive {passive:?}; engaged {engaged:?}");
    assert!(passive.0.is_empty(), "nobody acted: no mark is set");
    assert!(
        engaged.0.iter().any(|m| m == "at_hall"),
        "the engaged stand-in carried boxes at the hall: {engaged:?}"
    );
}

fn skip(e: &mut Engine, limit: Option<u32>) -> Option<SkipStop> {
    e.apply(Command::Skip {
        target: SkipTargetKind::EndOfTerm,
        limit,
    })
    .ok()?;
    e.state().act.last_skip.as_ref().map(|s| s.stop.clone())
}

/// Ledger entries the player's own acts taught since `since`.
fn taught_by_acts(e: &Engine, since: mandate_core::SimDate) -> usize {
    e.state()
        .player
        .as_ref()
        .unwrap()
        .knowledge
        .entries
        .values()
        .flat_map(|x| x.learned.iter())
        .filter(|l| {
            l.on >= since && matches!(l.source, Source::Action { .. } | Source::Visit { .. })
        })
        .count()
}

#[test]
fn a_skip_learns_nothing_new() {
    let play = |c: &Arc<ContentBundle>| {
        let mut g = Game::new(c, &Setup::small(4, None), Driver::Idle).unwrap();
        let since = g.e.state().calendar.date;
        // Stops are skipped on from: twelve turns of skipping in all.
        for _ in 0..12 {
            skip(&mut g.e, Some(1));
        }
        assert!(g.e.state().calendar.turn >= 10, "the skip ran");
        taught_by_acts(&g.e, since)
    };
    let kept = play(&content_with("act_flow.proxy.learns", true.into()));
    let none = play(&content());
    println!("taught by the proxy's acts: {kept} when kept, {none} as shipped");
    assert!(kept > 0, "the proxy's canvasses would teach");
    assert_eq!(none, 0, "a skip learns nothing new (DEC-102 clause 7)");
}

fn inner(c: &PlayerCommand) -> &PlayerCommand {
    match c {
        PlayerCommand::Go {
            then: Some(then), ..
        } => inner(then),
        c => c,
    }
}

#[test]
fn the_proxy_makes_no_promises_and_files_for_nothing() {
    for seed in [2, 6] {
        let c = content();
        let mut g = Game::new(&c, &Setup::small(seed, None), Driver::Idle).unwrap();
        let (mut sent, mut campaign) = (0, false);
        for _ in 0..70 {
            campaign |= standin::campaigning(g.e.state());
            let mut t = mandate_sim::proxy::ProxyTurn::new(g.e.state(), &c);
            while let Some(cmd) = t.next(g.e.state(), &c) {
                assert!(
                    !matches!(
                        inner(&cmd),
                        PlayerCommand::File { .. }
                            | PlayerCommand::Promise { .. }
                            | PlayerCommand::RespondAsk { .. }
                            | PlayerCommand::Talk { .. }
                            | PlayerCommand::Meet { .. }
                    ),
                    "the skip's stand-in sent {cmd:?}"
                );
                sent += 1;
                let r = g.e.apply(Command::Player(cmd));
                let created = r
                    .as_ref()
                    .ok()
                    .and_then(|o| o.result.as_ref())
                    .and_then(|x| x.created);
                t.result(r.is_ok(), created);
            }
            if g.e.apply(Command::EndTurn).is_err() {
                break;
            }
        }
        assert!(
            sent > 50 && campaign,
            "seed {seed}: it played into a campaign"
        );
        let st = g.e.state();
        assert!(
            st.player.as_ref().unwrap().filing.is_none(),
            "it filed for nothing"
        );
        assert!(
            st.asks.asks.iter().all(|a| !a.promise && !a.accepted()),
            "it promised nothing and accepted no ask"
        );
    }
}

#[test]
fn an_ask_on_offer_waits_and_stops_the_skip_when_it_comes_due() {
    // A game with an ask on offer that falls due inside the interrupt's
    // window: idle turns until one is there.
    // As shipped, only a big ask on offer stops a skip (every offer made a
    // skipped year stop 11.6 times); the mechanism is tested with every
    // offer counted, against none.
    let shipped = content_with("act_flow.interrupts[5].when.offers", true.into());
    let without = content_with("act_flow.interrupts[5].when.big_offers", false.into());
    assert!(
        matches!(
            content().act_flow.interrupts[5].when,
            mandate_content::act::InterruptWhen::RequestDue {
                offers: false,
                big_offers: true,
                ..
            }
        ),
        "the overrides name the request_due interrupt"
    );
    let mut found = None;
    'seeds: for seed in 1..=8 {
        let mut g = Game::new(&shipped, &Setup::small(seed, None), Driver::Idle).unwrap();
        for _ in 0..60 {
            let st = g.e.state();
            let today = st.calendar.date;
            if let Some(a) = st
                .asks
                .due_within(today, 10)
                .into_iter()
                .find(|a| a.offered())
            {
                found = Some((seed, a.id.0, g));
                break 'seeds;
            }
            if !g.step_turn() {
                break;
            }
        }
    }
    let (seed, ask, g) = found.expect("some world offers the player an ask");
    let key = format!("interrupt.request_due:offer:{ask}");
    let keys = |c: &ContentBundle| -> Vec<String> {
        mandate_sim::skip::interrupt_keys(g.e.state(), c)
            .into_iter()
            .map(|k| k.1)
            .collect()
    };
    println!("seed {seed}: ask {ask} on offer and due");
    assert!(keys(&shipped).contains(&key), "the offer stops a skip");
    assert!(
        !keys(&without).contains(&key),
        "without `offers` an offer lapses unseen"
    );
    // As shipped: it stops the skip exactly when the ask is a big one.
    let big = g.e.state().asks.asks.iter().any(|a| a.id.0 == ask && a.big);
    assert_eq!(keys(&content()).contains(&key), big);
    // A skip started earlier stops on it: nothing answered it meanwhile.
    assert!(
        g.e.state()
            .asks
            .asks
            .iter()
            .any(|a| a.id.0 == ask && a.offered()),
        "the ask is still on offer: nobody answered it for the player"
    );
}

/// The flood run's stage entries and its exit since the last look, with
/// their days.
fn flood_moves(e: &Engine, cursor: &mut u32) -> Vec<(mandate_core::SimDate, String)> {
    use mandate_sim::events::EventKind;
    let st = e.state();
    let moved = st
        .events
        .iter()
        .filter(|e| e.id.0 >= *cursor)
        .filter_map(|e| match &e.kind {
            EventKind::ArcStage {
                template, stage, ..
            } if template == "arc.flood" => Some((e.date, stage.clone())),
            EventKind::ArcEnded { template, exit, .. } if template == "arc.flood" => {
                Some((e.date, exit.clone()))
            }
            _ => None,
        })
        .collect();
    *cursor = st.events.iter().last().map_or(*cursor, |e| e.id.0 + 1);
    moved
}

/// Whether a skip stops at the end of the turn a planted flood's arc is
/// raised in (whichever interrupt names the stop: the ask the first stage
/// brings comes first in the list).
fn stops_at_the_raise(c: &Arc<ContentBundle>) -> bool {
    let mut g = Game::new(c, &Setup::small(3, None), Driver::Idle).unwrap();
    let town = g.home();
    g.intervene(&super::runner::Intervention::Disaster {
        town,
        hazard: "flood".into(),
        severity: 0.9,
    })
    .unwrap();
    let mut cursor = g.e.state().events.iter().last().map_or(0, |e| e.id.0 + 1);
    let turn_days = g.e.content().tuning.turn_days.governing_local as u32;
    for _ in 0..12 {
        let stop = skip(&mut g.e, None);
        let moved = flood_moves(&g.e, &mut cursor);
        if let Some(first) = moved.first() {
            return stop.is_some() && first.0.add_days(turn_days) >= g.e.state().calendar.date;
        }
    }
    false
}

/// Every moment the arc interrupt would stop a skip at, over an idle game
/// with a planted flood: `(arc, stage, raised)`.
fn arc_moments(c: &Arc<ContentBundle>) -> std::collections::BTreeSet<(String, String, bool)> {
    let mut g = flooded(3);
    let mut out = std::collections::BTreeSet::new();
    for _ in 0..70 {
        for (i, key, args) in mandate_sim::skip::interrupt_keys(g.e.state(), c) {
            if c.act_flow.interrupts[i].id != "interrupt.arc_stage" {
                continue;
            }
            let arg = |k: &str| {
                args.iter()
                    .find(|a| a.0 == k)
                    .map(|a| a.1.clone())
                    .unwrap_or_default()
            };
            assert!(!arg("run").is_empty(), "a stop names its run");
            out.insert((arg("arc"), arg("stage"), key.ends_with(":+raised")));
        }
        if !g.step_turn() {
            break;
        }
    }
    out
}

#[test]
fn an_arc_stops_a_skip_when_it_is_the_players_to_meet() {
    let shipped = content();
    let standing = |arc: &str| shipped.arc(arc).is_some_and(|d| d.standing.is_some());
    let stage = |arc: &str, id: &str| shipped.arc(arc).and_then(|d| d.stage(id)).cloned();
    assert!(
        stops_at_the_raise(&shipped),
        "a material arc stops a skip as it is raised"
    );
    let moments = arc_moments(&shipped);
    println!("as shipped: {moments:?}");
    assert!(moments.contains(&("arc.flood".into(), "struck".into(), true)));
    for (arc, id, raised) in &moments {
        let s = stage(arc, id).expect("a stage of the template");
        if *raised {
            assert!(
                !standing(arc),
                "a standing arc's raise stops no skip: {arc}"
            );
            continue;
        }
        assert!(s.exit.is_none(), "an exit stops no skip: {arc} {id}");
        assert!(
            s.beats.iter().any(|b| b.kind.player()),
            "a stage with nothing for the player to do stops no skip: {arc} {id}"
        );
        if standing(arc) {
            assert!(
                s.climax,
                "a standing arc stops a skip only at its turn: {arc} {id}"
            );
        }
    }
    assert!(
        moments.iter().any(|x| !x.2 && x.0 == "arc.flood"),
        "a material arc's stage with a beat stops a skip: {moments:?}"
    );
    assert!(
        moments.iter().any(|x| !x.2 && standing(&x.0)),
        "a standing arc's turn stops a skip: {moments:?}"
    );
    // The broad rule is the same mechanism with other content: every
    // stage and every exit of every arc.
    let at = "act_flow.interrupts[6].when";
    assert!(matches!(
        shipped.act_flow.interrupts[6].when,
        mandate_content::act::InterruptWhen::ArcStage { .. }
    ));
    let broad = super::load(
        &super::runner::content_root(),
        &[
            (format!("{at}.material.idle"), true.into()),
            (format!("{at}.standing.raised"), true.into()),
            (format!("{at}.standing.stages"), "every".into()),
            (format!("{at}.standing.lead_or_actor"), false.into()),
            (format!("{at}.standing.idle"), true.into()),
        ]
        .map(|(path, value): (String, serde_json::Value)| Override { path, value }),
    )
    .unwrap();
    let wide = arc_moments(&broad);
    println!("under the broad rule: {wide:?}");
    assert!(wide.is_superset(&moments) && wide.len() > moments.len());
    assert!(
        wide.iter()
            .any(|(arc, id, _)| stage(arc, id).is_some_and(|s| s.exit.is_some())),
        "under it an exit stops a skip: {wide:?}"
    );
    assert!(
        wide.iter().any(|(arc, id, raised)| !raised
            && standing(arc)
            && stage(arc, id).is_some_and(|s| !s.climax)),
        "under it a standing arc's early stage stops a skip: {wide:?}"
    );
}

#[test]
fn the_grader_gives_the_same_output_twice() {
    let c = content();
    let args = crate::arcs_grade::GradeArgs {
        seeds: 2,
        arc: Some("arc.flood".into()),
        ..Default::default()
    };
    let go = || {
        let (report, failures, _) = crate::arcs_grade::run(&c, &args).unwrap();
        assert!(failures.is_empty(), "coverage is not enforced on two seeds");
        serde_json::to_string(&report).unwrap()
    };
    let a = go();
    assert_eq!(a, go());
    let v: serde_json::Value = serde_json::from_str(&a).unwrap();
    let arc = &v["arcs"][0];
    assert_eq!(arc["worlds_found"], 2);
    for term in ["agency", "escalation", "visibility", "pacing", "variety"] {
        assert!(
            arc["terms"][term]["median"].is_number(),
            "the grader prints {term}"
        );
    }
    assert!(arc["coverage"]["share"].is_number());
}

#[test]
fn a_start_is_rated_the_same_twice() {
    let c = content();
    let args = super::rate::RateArgs {
        seed: 5,
        post: None,
        plays: 2,
        after_months: None,
        style: super::rate::INFORMED.into(),
    };
    let a = super::rate::rate_start(&c, &args).unwrap();
    let b = super::rate::rate_start(&c, &args).unwrap();
    assert_eq!(a, b);
    assert_eq!(
        a["race"]["holds_it"], true,
        "the opening is the player's own seat"
    );
    assert_eq!(a["rating"]["runs"], 4);
    let facts = a["rating"]["facts"].as_array().unwrap();
    assert!(
        (2..=3).contains(&facts.len()),
        "two or three facts: {facts:?}"
    );
    assert!(
        facts
            .iter()
            .all(|f| f["key"].as_str().unwrap().starts_with("factor-")),
        "facts are keys, never prose"
    );
    // Another luck is another future: the plays do not all end alike.
    let plays = a["players"]["standin:engaged"]["plays"].as_array().unwrap();
    assert_ne!(plays[0]["state_hash"], plays[1]["state_hash"]);
}

#[test]
fn a_post_is_found_by_its_constitution_id() {
    let c = content();
    let g = Game::new(&c, &Setup::small(5, None), Driver::Idle).unwrap();
    let st = g.e.state();
    let con = c.constitutions.get(&st.meta.constitution).unwrap();
    // Every post of the tiers the engine instantiates resolves; the rest
    // say why not. No id is written here: they are read from the
    // constitution.
    let mut found = 0;
    for t in &con.tiers {
        for id in t
            .bodies
            .iter()
            .map(|b| &b.id)
            .chain(t.offices.iter().map(|o| &o.id))
        {
            match standin::post_named(st, &c, id) {
                Ok(p) => {
                    assert_eq!(p.id(), id.as_str());
                    assert_eq!(p.at().tier(), t.tier);
                    found += 1;
                }
                Err(e) => assert!(e.contains("not elected yet"), "{id}: {e}"),
            }
        }
    }
    assert!(found >= 2, "the home town's body and its office");
    assert!(standin::post_named(st, &c, "no_such_post").is_err());
}
