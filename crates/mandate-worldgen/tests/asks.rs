#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Asks and promises (E6, DEC-097 §2; ENGAGEMENT §3):
//!
//! - each `Need` is kept by the deed it asks for, broken by its deadline (or
//!   a deed against it) and void when it becomes impossible;
//! - an ask resolves exactly once, and the deed log stays bounded and ordered;
//! - the sliding rate: no offer the day after one, none at `max_open`, twice
//!   the base chance with nothing open after 21 quiet days;
//! - casework, promises, the second-term deadlines;
//! - accepting a big ask ends the director's relax phase;
//! - the asks state holds no `MunicipalityId` key, and asks name no office.

use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::asks::{
    Ask, AskId, AskStatus, Asker, Deed, History, Need, StatementNeed, offers,
};
use mandate_sim::narrative::{Actor, CastEntry};
use mandate_sim::places::Location;
use mandate_sim::policy::PolicyStatus;
use mandate_sim::politics::Vote;
use mandate_sim::scheduler::Reads;
use mandate_sim::world::NeighborhoodId;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

fn base() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn engine(c: &Arc<ContentBundle>, seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

/// Put an accepted ask with these needs before the player (a casework
/// template: its effects name the asker, cast as the first Named resident).
fn put(e: &mut Engine, needs: Vec<Need>, due_in: u32) -> AskId {
    let date = e.state().calendar.date;
    let asker = e
        .state()
        .politics
        .characters
        .people
        .iter()
        .find(|c| {
            c.named.is_some()
                && mandate_sim::characters::municipality_of(
                    &e.state().politics,
                    &e.state().world,
                    c.id,
                ) == mandate_sim::player::home(e.state()).map(|m| m.0)
        })
        .map(|c| c.id)
        .unwrap();
    let mut id = AskId(0);
    e.setup(|st| {
        id = st.asks.ids.alloc();
        st.asks.asks.push(Ask {
            id,
            template: "ask.casework".into(),
            promise: false,
            asker: Asker::Person { id: asker },
            cast: vec![CastEntry {
                role: "asker".into(),
                actor: Actor::Person { id: asker },
            }],
            needs,
            step: 0,
            done: Vec::new(),
            public: false,
            big: false,
            opened: date,
            respond_by: date,
            due: date.add_days(due_in),
            status: AskStatus::Accepted { on: date },
            history: History::None,
            race: None,
            answer: None,
            tries: 0,
        });
    });
    id
}

fn status(e: &Engine, id: AskId) -> AskStatus {
    e.state().asks.get(id).unwrap().status.clone()
}

fn day(e: &mut Engine) {
    e.apply(Command::AdvanceDays { days: 1 }).unwrap();
}

fn cmd(e: &mut Engine, c: PlayerCommand) {
    let c = e.route(c);
    e.apply(c).unwrap();
}

fn act(e: &mut Engine, a: Action) {
    cmd(
        e,
        PlayerCommand::Act {
            action: a,
            late_night: false,
        },
    );
}

fn ward_hood(e: &Engine) -> NeighborhoodId {
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    NeighborhoodId(mandate_sim::policy::ward_hoods(&st.world, pl.ward)[0] as u32)
}

fn kept(s: &AskStatus) -> bool {
    matches!(s, AskStatus::Kept { .. })
}

fn broken(s: &AskStatus) -> bool {
    matches!(s, AskStatus::Broken { .. })
}

#[test]
fn deeds_keep_their_needs() {
    let c = base();
    let mut e = engine(&c, 3);
    let hood = ward_hood(&e);
    // A statement on the town's affairs, a canvass, a town hall.
    let s = put(
        &mut e,
        vec![Need::Statement {
            about: StatementNeed::Any,
        }],
        20,
    );
    let k = put(
        &mut e,
        vec![Need::Canvass {
            hood,
            or_town_hall: false,
        }],
        20,
    );
    let t = put(&mut e, vec![Need::TownHall { hoods: vec![hood] }], 20);
    act(&mut e, Action::Press { policy: None });
    day(&mut e);
    assert!(kept(&status(&e, s)), "{:?}", status(&e, s));
    assert!(e.state().asks.get(k).unwrap().accepted());
    e.apply(Command::EndTurn).unwrap();
    act(
        &mut e,
        Action::Canvass {
            neighborhood: hood.0,
            issue: None,
        },
    );
    act(
        &mut e,
        Action::TownHall {
            neighborhood: hood.0,
            policy: None,
        },
    );
    day(&mut e);
    assert!(kept(&status(&e, k)));
    assert!(kept(&status(&e, t)));
    // A visit: be at the place.
    let place = e
        .state()
        .world
        .places
        .iter()
        .find(|p| Some(p.municipality) == mandate_sim::player::home(e.state()) && p.is_open())
        .unwrap()
        .id;
    let v = put(&mut e, vec![Need::Visit { place }], 20);
    cmd(
        &mut e,
        PlayerCommand::MoveTo {
            location: Location::Place { place },
        },
    );
    day(&mut e);
    assert!(kept(&status(&e, v)));
    // The deed log has them, in order.
    let deeds = &e.state().asks.deeds;
    assert!(deeds.iter().any(|d| matches!(d.deed, Deed::Statement { .. })));
    assert!(deeds.iter().any(|d| matches!(d.deed, Deed::Canvass { .. })));
    assert!(deeds.iter().any(|d| matches!(d.deed, Deed::Arrive { .. })));
    assert!(deeds.windows(2).all(|w| w[0].on <= w[1].on));
}

#[test]
fn needs_are_met_in_order_and_break_at_the_deadline() {
    let c = base();
    let mut e = engine(&c, 4);
    let hood = ward_hood(&e);
    // A town hall, then a statement: a statement first does not count.
    let id = put(
        &mut e,
        vec![
            Need::TownHall { hoods: vec![hood] },
            Need::Statement {
                about: StatementNeed::Any,
            },
        ],
        10,
    );
    act(&mut e, Action::Press { policy: None });
    day(&mut e);
    assert_eq!(e.state().asks.get(id).unwrap().step, 0);
    act(
        &mut e,
        Action::TownHall {
            neighborhood: hood.0,
            policy: None,
        },
    );
    day(&mut e);
    assert_eq!(e.state().asks.get(id).unwrap().step, 1);
    assert!(e.state().asks.get(id).unwrap().accepted());
    // Nothing more: broken after the deadline, once.
    e.apply(Command::AdvanceDays { days: 12 }).unwrap();
    assert!(broken(&status(&e, id)));
    let resolved = e
        .state()
        .events
        .iter()
        .filter(|ev| {
            matches!(&ev.kind, mandate_sim::events::EventKind::AskResolved { ask, .. } if *ask == id.0)
        })
        .count();
    assert_eq!(resolved, 1);
    assert_eq!(e.state().asks.tally.broken, 1);
}

#[test]
fn watchers_are_kept_by_not_acting() {
    let c = base();
    let mut e = engine(&c, 5);
    let me = e.state().player.as_ref().unwrap().politician;
    let person = e.state().politics.get(me).unwrap().character;
    let a = put(&mut e, vec![Need::NoSurfacedHiddenFact], 5);
    let b = put(
        &mut e,
        vec![Need::VoteAgainstAll {
            tag: "tax_rise".into(),
        }],
        5,
    );
    let q = put(&mut e, vec![Need::Quiet { person }], 5);
    e.apply(Command::AdvanceDays { days: 3 }).unwrap();
    assert!(e.state().asks.get(a).unwrap().accepted());
    e.apply(Command::AdvanceDays { days: 4 }).unwrap();
    for id in [a, b, q] {
        assert!(kept(&status(&e, id)), "{:?}", status(&e, id));
    }
}

/// An NPC's bill before the council in the player's town that the player
/// can vote on, after a few turns of some seed.
fn with_bill(c: &Arc<ContentBundle>) -> (Engine, u32) {
    for seed in 1..=30 {
        let mut e = engine(c, seed);
        for _ in 0..60 {
            let st = e.state();
            let r = Reads::of_state(st, c);
            let v = mandate_sim::narrative::cast::View::new(&r, &st.politics);
            if let Some(p) = st.policy.instances.iter().find(|p| {
                v.can_vote(p.id.0)
                    && p.next_stage_on
                        .is_some_and(|d| d > st.calendar.date.add_days(3))
            }) {
                let id = p.id.0;
                return (e, id);
            }
            e.apply(Command::EndTurn).unwrap();
        }
    }
    panic!("no bill to vote on in 30 seeds");
}

#[test]
fn votes_keep_break_or_void() {
    let c = base();
    // Kept by the vote asked for.
    let (mut e, bill) = with_bill(&c);
    let id = put(
        &mut e,
        vec![Need::Vote {
            policy: bill,
            vote: Vote::Yes,
        }],
        60,
    );
    cmd(
        &mut e,
        PlayerCommand::CastVote {
            policy: bill,
            vote: Vote::Yes,
        },
    );
    day(&mut e);
    assert!(kept(&status(&e, id)));
    // Broken at once by the other vote.
    let (mut e, bill) = with_bill(&c);
    let id = put(
        &mut e,
        vec![Need::Vote {
            policy: bill,
            vote: Vote::Yes,
        }],
        60,
    );
    cmd(
        &mut e,
        PlayerCommand::CastVote {
            policy: bill,
            vote: Vote::No,
        },
    );
    day(&mut e);
    assert!(broken(&status(&e, id)));
    // Void when the bill is withdrawn (not the player's fault).
    let (mut e, bill) = with_bill(&c);
    let id = put(
        &mut e,
        vec![Need::Vote {
            policy: bill,
            vote: Vote::Yes,
        }],
        60,
    );
    e.setup(|st| {
        let p = &mut st.policy.instances[bill as usize];
        p.status = PolicyStatus::Ended;
        p.end_reason = Some(mandate_sim::policy::EndReason::Withdrawn);
    });
    day(&mut e);
    assert!(
        matches!(status(&e, id), AskStatus::Void { ref reason, .. } if reason == "ask-void-withdrawn")
    );
    // Broken when the vote came and went without the player's.
    let (mut e, bill) = with_bill(&c);
    let id = put(
        &mut e,
        vec![Need::Vote {
            policy: bill,
            vote: Vote::Yes,
        }],
        400,
    );
    while e.state().policy.instances[bill as usize].status == PolicyStatus::UnderConsideration {
        e.apply(Command::EndTurn).unwrap();
    }
    day(&mut e);
    assert!(
        !e.state().asks.get(id).unwrap().open(),
        "{:?}",
        status(&e, id)
    );
}

#[test]
fn casework_is_sorted_at_the_town_hall_with_the_shown_odds() {
    let c = base();
    let mut e = engine(&c, 6);
    let id = put(&mut e, vec![Need::Casework], 30);
    let odds = mandate_sim::asks::casework_odds(e.state(), &c);
    assert!(odds == 0.85 || odds == 0.5);
    let mut tries = 0;
    while e.state().asks.get(id).unwrap().open() && tries < 12 {
        if e.state().player.as_ref().unwrap().slots_left() == 0 {
            e.apply(Command::EndTurn).unwrap();
        }
        let before = e.state().player.as_ref().unwrap().slots_left();
        act(&mut e, Action::Casework { ask: id.0 });
        assert_eq!(e.state().player.as_ref().unwrap().slots_left(), before - 1);
        tries += 1;
        day(&mut e);
    }
    assert!(kept(&status(&e, id)));
    assert_eq!(e.state().asks.get(id).unwrap().tries as usize, tries);
    // Nothing to sort out: refused, no slot spent.
    let r = e.apply(e.route(PlayerCommand::Act {
        action: Action::Casework { ask: id.0 },
        late_night: false,
    }));
    assert!(r.is_err());
}

#[test]
fn the_sliding_rate_follows_its_curves() {
    let c = base();
    let mut e = engine(&c, 7);
    let p = c.ask_params.clone();
    let base_rate = p.new_per_month * 12.0 / 52.0;
    let chance = |e: &Engine| {
        let r = Reads::of_state(e.state(), &c);
        offers::chance(&e.state().asks, &r)
    };
    // Nothing open, 21 quiet days: twice the base chance.
    let date = e.state().calendar.date;
    e.setup(|st| st.asks.last_offer = Some(mandate_core::SimDate(date.0 - 21)));
    assert!((chance(&e) - 2.0 * base_rate).abs() < 1e-9, "{}", chance(&e));
    // An offer today: none until days have passed.
    e.setup(|st| st.asks.last_offer = Some(date));
    assert_eq!(chance(&e), 0.0);
    // At max_open: none.
    e.setup(|st| st.asks.last_offer = Some(mandate_core::SimDate(date.0 - 60)));
    for _ in 0..p.max_open {
        put(&mut e, vec![Need::Casework], 30);
    }
    assert_eq!(chance(&e), 0.0);
}

/// The time skip's `request_due` interrupt reads accepted asks and promises
/// through the asks' read API; offers do not stop a skip.
#[test]
fn asks_falling_due_interrupt_a_skip() {
    let c = base();
    let mut e = engine(&c, 8);
    let key = |e: &Engine| {
        mandate_sim::skip::interrupt_keys(e.state(), &c)
            .into_iter()
            .any(|(_, k, _)| k.starts_with("interrupt.request_due:ask:"))
    };
    assert!(!key(&e));
    let id = put(&mut e, vec![Need::Casework], 60);
    assert!(!key(&e), "not due within the interrupt's days");
    assert!(e.state().asks.due_within(e.state().calendar.date, 60).len() == 1);
    e.setup(|st| {
        let date = st.calendar.date;
        let a = st.asks.asks.iter_mut().find(|a| a.id == id).unwrap();
        a.due = date.add_days(3);
    });
    assert!(key(&e));
    e.setup(|st| {
        let a = st.asks.asks.iter_mut().find(|a| a.id == id).unwrap();
        a.status = AskStatus::Offered;
    });
    assert!(!key(&e), "an offer lapses rather than stopping a skip");
}

#[test]
fn second_term_deadlines_draw_from_the_lower_half() {
    assert_eq!(offers::deadline_range([21, 35], false), [21, 35]);
    assert_eq!(offers::deadline_range([21, 35], true), [21, 28]);
    assert_eq!(offers::deadline_range([14, 14], true), [14, 14]);
}

/// Answer offers by a seeded coin, keep what is easy, and play years: every
/// ask resolves exactly once, the tally adds up, the deed log stays bounded.
#[test]
fn asks_resolve_exactly_once_over_careers() {
    let c = base();
    for seed in [11u64, 12, 13] {
        let mut e = engine(&c, seed);
        let mut coin = seed;
        for _ in 0..90 {
            let offers: Vec<u32> = e
                .state()
                .asks
                .asks
                .iter()
                .filter(|a| a.offered())
                .map(|a| a.id.0)
                .collect();
            for ask in offers {
                coin = coin.wrapping_mul(6364136223846793005).wrapping_add(1);
                let accept = (coin >> 33) % 3 != 0;
                e.apply(Command::Player(PlayerCommand::RespondAsk { ask, accept }))
                    .unwrap();
            }
            let hood = ward_hood(&e);
            let c2 = e.route(PlayerCommand::Act {
                action: Action::Canvass {
                    neighborhood: hood.0,
                    issue: None,
                },
                late_night: false,
            });
            let _ = e.apply(c2);
            e.apply(Command::EndTurn).unwrap();
        }
        let st = e.state();
        let mut seen: BTreeMap<u32, u32> = BTreeMap::new();
        for ev in st.events.iter() {
            if let mandate_sim::events::EventKind::AskResolved { ask, .. } = ev.kind {
                *seen.entry(ask).or_default() += 1;
            }
        }
        assert!(seen.values().all(|n| *n == 1), "seed {seed}: {seen:?}");
        let t = &st.asks.tally;
        assert!(t.offered >= 8, "seed {seed}: {t:?}");
        let open = st.asks.asks.iter().filter(|a| a.open() && !a.promise).count() as u32;
        assert_eq!(
            t.offered,
            t.kept + t.broken + t.declined + t.void + open,
            "seed {seed}: {t:?}"
        );
        assert!(st.asks.deeds.len() <= c.ask_params.deeds_kept as usize);
        assert!(st.asks.deeds.windows(2).all(|w| w[0].on <= w[1].on));
        assert!(st.asks.resolved().count() <= c.ask_params.history as usize);
    }
}

#[test]
fn promises_are_made_while_campaigning_and_counted_per_race() {
    let c = base();
    let mut e = engine(&c, 2);
    assert!(mandate_sim::asks::promise_menu(e.state(), &c).is_empty());
    let r = e.apply(Command::Player(PlayerCommand::Promise {
        template: "promise.town_halls".into(),
        args: vec![],
    }));
    assert!(r.is_err(), "no promises outside a campaign");
    // To the campaign.
    while !mandate_sim::characters::is_candidate(
        &e.state().politics,
        e.state().player.as_ref().unwrap().politician,
    ) {
        e.apply(Command::EndTurn).unwrap();
    }
    let menu = mandate_sim::asks::promise_menu(e.state(), &c);
    assert!(menu.iter().any(|o| o.template == "promise.town_halls"));
    let approval = |e: &Engine| -> f64 {
        e.state()
            .population
            .hoods
            .iter()
            .flat_map(|h| h.cohorts.iter())
            .map(|c| c.player_approval as f64)
            .sum()
    };
    let before = approval(&e);
    let mut made = 0;
    for o in menu.iter().take(5) {
        let args = o
            .answer
            .clone()
            .map(|a| vec![("answer".to_string(), a)])
            .unwrap_or_default();
        if e
            .apply(Command::Player(PlayerCommand::Promise {
                template: o.template.clone(),
                args,
            }))
            .is_ok()
        {
            made += 1;
        }
    }
    assert_eq!(made, menu.len().min(3), "at most three a campaign");
    assert!(approval(&e) > before, "promises persuade");
    let st = e.state();
    assert_eq!(st.asks.promises_this_campaign as usize, made);
    let race = st.asks.promises_race.unwrap();
    for a in st.asks.asks.iter().filter(|a| a.promise) {
        assert_eq!(a.race, Some(race));
        assert!(a.public && a.accepted());
        assert!(matches!(
            a.asker,
            Asker::Constituency { .. } | Asker::Electorate { .. }
        ));
    }
    assert!(mandate_sim::asks::promise_menu(st, &c).is_empty());
    assert_eq!(st.asks.promise_outcomes().made as usize, made);
    assert_eq!(st.asks.promise_outcomes().open as usize, made);
}

#[test]
fn accepting_a_big_ask_ends_a_relax() {
    use mandate_sim::narrative::PhaseKind;
    let c = base();
    for seed in 1..=20u64 {
        let mut e = engine(&c, seed);
        // Find a relax with time left in it.
        let mut found = false;
        for _ in 0..80 {
            e.apply(Command::EndTurn).unwrap();
            let n = &e.state().narrative;
            if let Some(p) = &n.phase
                && p.kind == PhaseKind::Relax
                && p.until.is_some_and(|u| u > e.state().calendar.date.add_days(10))
            {
                found = true;
                break;
            }
        }
        if !found {
            continue;
        }
        let id = put(&mut e, vec![Need::Casework], 60);
        e.setup(|st| {
            let date = st.calendar.date;
            let a = st.asks.asks.iter_mut().find(|a| a.id == id).unwrap();
            a.status = AskStatus::Offered;
            a.big = true;
            a.respond_by = date.add_days(7);
        });
        e.apply(Command::Player(PlayerCommand::RespondAsk {
            ask: id.0,
            accept: true,
        }))
        .unwrap();
        let d = e.state().calendar.date.0;
        e.apply(Command::AdvanceDays { days: 7 - d % 7 }).unwrap();
        let p = e.state().narrative.phase.as_ref().unwrap();
        assert_ne!(p.kind, PhaseKind::Relax, "seed {seed}");
        return;
    }
    panic!("no relax phase in 20 seeds");
}

#[test]
fn asks_name_no_town_key_and_no_office() {
    let c = base();
    let mut e = engine(&c, 9);
    for _ in 0..30 {
        e.apply(Command::EndTurn).unwrap();
    }
    let v = serde_json::to_string(&e.state().asks).unwrap();
    assert!(!v.contains("municipality"), "the asks state is tier-generic");
    // No office or body word in the asks' content or text (DEC-098).
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/base");
    let mut text = String::new();
    for f in ["asks/asks.jsonc", "asks/promises.jsonc", "locale/en/asks.ftl"] {
        text.push_str(&std::fs::read_to_string(root.join(f)).unwrap().to_lowercase());
    }
    for k in c.constitutions.values() {
        for t in &k.tiers {
            for p in t.posts() {
                for w in [p.id(), p.word()] {
                    let w = w.to_lowercase();
                    assert!(
                        !text
                            .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                            .any(|x| x == w),
                        "asks name the office or body `{w}`"
                    );
                }
            }
        }
    }
}
