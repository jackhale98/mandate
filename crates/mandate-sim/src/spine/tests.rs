//! Tests of the causal spine: threads, the graph's queries, two worked
//! sifting patterns, and the bound. Logs are recorded by hand with the
//! causes the engine's sites give, so the tests do not move with balance.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::graph::{self, Step};
use super::prune::{self, Holds};
use super::*;
use crate::asks::Deed;
use crate::events::{DomainEvent, EventLog};
use crate::politics::Vote;

fn day(n: u32) -> SimDate {
    SimDate(100_000 + n)
}

fn person(n: u32) -> CharacterId {
    CharacterId(n)
}

/// A recession closes a plant; someone asks the player for help; the
/// player promises, votes, and the ask is kept.
fn favour() -> (EventLog, [EventId; 6]) {
    let mut log = EventLog::default();
    let recession = log.record(
        day(0),
        EventKind::GlobalRecessionBegan { depth_pct: 6 },
        Cause::Exogenous(Shock::WorldEconomy),
    );
    let closure = log.record(
        day(40),
        EventKind::PlantClosure {
            municipality: 3,
            sector: "sector.manufacturing".into(),
            jobs: 400,
        },
        Causes::from(Cause::Pressure(Pressure::WorldDemand))
            .and_event(Some(recession), Link::Caused),
    );
    let offered = log.record(
        day(60),
        EventKind::AskOffered {
            ask: 1,
            template: "ask.vote".into(),
            big: true,
        },
        Causes::from(Cause::Process(Process::Asks))
            .and(Cause::Actor(person(7)))
            .and_event(Some(closure), Link::Prompted),
    );
    let accepted = log.record(
        day(61),
        EventKind::AskAccepted {
            ask: 1,
            template: "ask.vote".into(),
            big: true,
        },
        Cause::Player(PlayerAct::RespondAsk),
    );
    let voted = log.record(
        day(80),
        EventKind::PlayerDeed {
            deed: Deed::Vote {
                policy: 9,
                vote: Vote::Yes,
            },
        },
        Cause::Player(PlayerAct::Vote),
    );
    let kept = log.record(
        day(81),
        EventKind::AskResolved {
            ask: 1,
            template: "ask.vote".into(),
            outcome: "kept".into(),
            public: true,
        },
        Causes::from(Cause::Process(Process::Asks)).and_event(Some(voted), Link::Caused),
    );
    (log, [recession, closure, offered, accepted, voted, kept])
}

#[test]
fn a_thread_links_each_event_to_the_one_before() {
    let mut log = EventLog::default();
    let intro = log.record(
        day(0),
        EventKind::PolicyIntroduced {
            policy: 4,
            municipality: 1,
            sponsor: 2,
        },
        Cause::Actor(person(2)),
    );
    assert_eq!(log.head(Thread::Bill(4)), Some(intro));
    let enacted = log.record(
        day(30),
        EventKind::PolicyEnacted {
            policy: 4,
            municipality: 1,
            yes: 5,
            no: 2,
            overrode_veto: false,
        },
        Cause::Process(Process::Lawmaking),
    );
    assert_eq!(
        log.get(enacted).unwrap().causes,
        vec![
            Cause::Process(Process::Lawmaking),
            Cause::Event(intro, Link::Continues)
        ]
    );
    // A site that names the link itself is not given it twice.
    let expired = log.record(
        day(400),
        EventKind::PolicyExpired {
            policy: 4,
            municipality: 1,
        },
        Causes::from(Cause::Process(Process::Lawmaking)).and_event(Some(enacted), Link::Ended),
    );
    assert_eq!(log.get(expired).unwrap().links().count(), 1);
    assert_eq!(log.head(Thread::Bill(4)), Some(expired), "how it ended");
    // Another bill's thread is its own.
    let other = log.record(
        day(401),
        EventKind::PolicyFailed {
            policy: 5,
            municipality: 1,
            vetoed: false,
            yes: 1,
            no: 6,
        },
        Cause::Process(Process::Lawmaking),
    );
    assert_eq!(log.get(other).unwrap().links().count(), 0);
    assert!(log.heads().windows(2).all(|w| w[0].0 < w[1].0));
}

#[test]
fn the_players_vote_is_a_step_in_the_bills_life() {
    let mut log = EventLog::default();
    let intro = log.record(
        day(0),
        EventKind::PolicyIntroduced {
            policy: 4,
            municipality: 1,
            sponsor: 2,
        },
        Cause::Actor(person(2)),
    );
    let vote = log.record(
        day(10),
        EventKind::PlayerDeed {
            deed: Deed::Vote {
                policy: 4,
                vote: Vote::Yes,
            },
        },
        Cause::Player(PlayerAct::Vote),
    );
    let enacted = log.record(
        day(30),
        EventKind::PolicyEnacted {
            policy: 4,
            municipality: 1,
            yes: 5,
            no: 2,
            overrode_veto: false,
        },
        Causes::from(Cause::Process(Process::Lawmaking)).and_event(Some(intro), Link::Continues),
    );
    let behind = graph::nearest_player_act(&log, enacted, 4).unwrap();
    assert_eq!(
        (behind.act, behind.at, behind.depth),
        (PlayerAct::Vote, vote, 1)
    );
    // Someone else's bill the player never touched has no player act behind it.
    let theirs = log.record(
        day(40),
        EventKind::PolicyIntroduced {
            policy: 6,
            municipality: 1,
            sponsor: 3,
        },
        Cause::Actor(person(3)),
    );
    assert_eq!(graph::nearest_player_act(&log, theirs, 8), None);
}

#[test]
fn the_chain_goes_back_to_its_roots() {
    let (log, [recession, closure, offered, accepted, voted, kept]) = favour();
    let chain = graph::chain(&log, kept, 8);
    let order: Vec<EventId> = chain.events.iter().map(|r| r.event).collect();
    // Nearest first; ties in the order the causes are named.
    assert_eq!(
        order,
        vec![kept, voted, accepted, offered, closure, recession]
    );
    let depth = |id| chain.events.iter().find(|r| r.event == id).unwrap().depth;
    assert_eq!((depth(voted), depth(accepted), depth(recession)), (1, 1, 4));
    assert!(!chain.truncated);
    // Every root on the way, with the event that names it.
    assert!(chain.roots.contains(&graph::Root {
        at: recession,
        depth: 4,
        cause: Cause::Exogenous(Shock::WorldEconomy),
    }));
    assert!(chain.roots.contains(&graph::Root {
        at: offered,
        depth: 2,
        cause: Cause::Actor(person(7)),
    }));
    // The link each event was reached by.
    let from = |id| chain.events.iter().find(|r| r.event == id).unwrap().from;
    assert_eq!(from(accepted), Some((kept, Link::Ended)));
    assert_eq!(from(closure), Some((offered, Link::Prompted)));
    // Depth-limited: one link back, and it says more lies behind.
    let near = graph::chain(&log, kept, 1);
    assert_eq!(near.events.len(), 3);
    assert!(near.truncated);
    // An event the log does not hold has no chain.
    assert!(graph::chain(&log, EventId(999), 8).events.is_empty());
}

#[test]
fn consequences_within_n_links() {
    let (log, [recession, closure, offered, accepted, _voted, kept]) = favour();
    assert_eq!(
        graph::effects_of(&log, recession),
        vec![(closure, Link::Caused)]
    );
    let two: Vec<(EventId, u8)> = graph::consequences(&log, recession, 2)
        .iter()
        .map(|r| (r.event, r.depth))
        .collect();
    assert_eq!(two, vec![(closure, 1), (offered, 2)]);
    let all: Vec<(EventId, u8)> = graph::consequences(&log, recession, 8)
        .iter()
        .map(|r| (r.event, r.depth))
        .collect();
    assert_eq!(
        all,
        vec![(closure, 1), (offered, 2), (accepted, 3), (kept, 4)]
    );
    // A player decision's consequences: the promise led to the ask kept.
    let promise = graph::consequences(&log, accepted, 3);
    assert_eq!(promise.len(), 1);
    assert_eq!(promise[0].from, Some((accepted, Link::Ended)));
    assert!(graph::consequences(&log, kept, 8).is_empty());
    assert!(graph::consequences(&log, recession, 0).is_empty());
}

#[test]
fn the_nearest_player_act_behind_an_event() {
    let (log, [recession, closure, _offered, accepted, voted, kept]) = favour();
    // The vote that kept the promise, before the promise itself (both one
    // link back; the deed is named first).
    let behind = graph::nearest_player_act(&log, kept, 8).unwrap();
    assert_eq!(
        (behind.act, behind.at, behind.depth),
        (PlayerAct::Vote, voted, 1)
    );
    // The act's own event is at depth 0.
    let own = graph::nearest_player_act(&log, accepted, 8).unwrap();
    assert_eq!((own.act, own.depth), (PlayerAct::RespondAsk, 0));
    // No player act lies behind a closure in a recession.
    assert_eq!(graph::nearest_player_act(&log, closure, 8), None);
    assert_eq!(graph::nearest_player_act(&log, recession, 8), None);
}

/// Worked sifting pattern 1, "a favour repaid": someone asked after a
/// closure, the player promised, and kept it.
#[test]
fn pattern_a_favour_repaid() {
    let (mut log, [_, closure, offered, accepted, _, kept]) = favour();
    // Noise the pattern must not match: an ask that was broken, and one
    // that no closure prompted.
    log.record(
        day(90),
        EventKind::AskOffered {
            ask: 2,
            template: "ask.town_hall".into(),
            big: false,
        },
        Cause::Process(Process::Asks),
    );
    log.record(
        day(91),
        EventKind::AskAccepted {
            ask: 2,
            template: "ask.town_hall".into(),
            big: false,
        },
        Cause::Player(PlayerAct::RespondAsk),
    );
    log.record(
        day(120),
        EventKind::AskResolved {
            ask: 2,
            template: "ask.town_hall".into(),
            outcome: "broken".into(),
            public: false,
        },
        Cause::Process(Process::Asks),
    );
    let is_closure = |e: &DomainEvent| matches!(e.kind, EventKind::PlantClosure { .. });
    let asked = |e: &DomainEvent| matches!(e.kind, EventKind::AskOffered { .. });
    let promised = |e: &DomainEvent| {
        matches!(e.kind, EventKind::AskAccepted { .. })
            && e.causes.iter().any(|c| c.player_act().is_some())
    };
    let kept_it = |e: &DomainEvent| matches!(&e.kind, EventKind::AskResolved { outcome, .. } if outcome == "kept");
    let pattern = [
        Step::next(&is_closure),
        Step::next(&asked).via(Link::Prompted),
        Step::next(&promised),
        Step::next(&kept_it).via(Link::Ended),
    ];
    assert_eq!(
        graph::matches(&log, &pattern, 10),
        vec![vec![closure, offered, accepted, kept]]
    );
    // Any promise and how it ended: both asks, the broken one too.
    let settled = |e: &DomainEvent| matches!(e.kind, EventKind::AskResolved { .. });
    let any_ask = [
        Step::next(&asked),
        Step::next(&promised),
        Step::next(&settled),
    ];
    assert_eq!(graph::matches(&log, &any_ask, 10).len(), 2);
    assert_eq!(graph::matches(&log, &any_ask, 1).len(), 1, "the limit");
    // An empty pattern matches nothing.
    assert!(graph::matches(&log, &[], 10).is_empty());
}

/// Worked sifting pattern 2, "the bribe that ended a career": the
/// player's own act left evidence, a story broke, prosecutors took it up,
/// and the sentence ended the career.
#[test]
fn pattern_the_bribe_that_ended_a_career() {
    let mut log = EventLog::default();
    // An unrelated scandal about someone else, with a case that was dropped.
    log.record(
        day(0),
        EventKind::FactDiscoverable { hidden: 1 },
        [Cause::Actor(person(9)), Cause::Process(Process::Director)],
    );
    let other = log.record(
        day(5),
        EventKind::ScandalBroke {
            scandal: 1,
            politician: 9,
            municipality: 0,
            severity: "ethics".into(),
            kind: "back_room_deal".into(),
        },
        Causes::from(Cause::Process(Process::Press))
            .and_event(log.head(Thread::Hidden(1)), Link::Revealed),
    );
    // The player's bribe.
    let fact = log.record(
        day(10),
        EventKind::FactDiscoverable { hidden: 2 },
        [
            Cause::Player(PlayerAct::TakeBribe),
            Cause::Process(Process::Director),
        ],
    );
    let broke = log.record(
        day(20),
        EventKind::ScandalBroke {
            scandal: 2,
            politician: 0,
            municipality: 0,
            severity: "crime".into(),
            kind: "bribe".into(),
        },
        Causes::from(Cause::Process(Process::Press))
            .and_event(log.head(Thread::Hidden(2)), Link::Revealed),
    );
    let opened = log.record(
        day(30),
        EventKind::CaseOpened {
            case: 1,
            politician: 0,
        },
        Causes::from(Cause::Process(Process::Justice))
            .and_event(log.head(Thread::Scandal(2)), Link::Prompted),
    );
    let charged = log.record(
        day(90),
        EventKind::CaseDeveloped {
            case: 1,
            politician: 0,
            stage: "charged".into(),
        },
        Cause::Process(Process::Justice),
    );
    let convicted = log.record(
        day(200),
        EventKind::CaseDeveloped {
            case: 1,
            politician: 0,
            stage: "convicted".into(),
        },
        Cause::Process(Process::Justice),
    );
    let ended = log.record(
        day(200),
        EventKind::CareerEnded {
            reason: "prison".into(),
        },
        Cause::caused_by(convicted),
    );
    let own_act = |e: &DomainEvent| {
        matches!(e.kind, EventKind::FactDiscoverable { .. })
            && e.causes.iter().any(|c| c.player_act().is_some())
    };
    let story = |e: &DomainEvent| matches!(e.kind, EventKind::ScandalBroke { .. });
    let case = |e: &DomainEvent| matches!(e.kind, EventKind::CaseOpened { .. });
    let guilty = |e: &DomainEvent| matches!(&e.kind, EventKind::CaseDeveloped { stage, .. } if stage == "convicted");
    let over = |e: &DomainEvent| matches!(e.kind, EventKind::CareerEnded { .. });
    let pattern = [
        Step::next(&own_act),
        Step::next(&story).via(Link::Revealed),
        Step::next(&case).via(Link::Prompted),
        // However many stages the case took.
        Step::later(&guilty, 4),
        Step::next(&over).via(Link::Caused),
    ];
    assert_eq!(
        graph::matches(&log, &pattern, 10),
        vec![vec![fact, broke, opened, convicted, ended]]
    );
    // The same answer from the other end: the act behind the ending.
    let behind = graph::nearest_player_act(&log, ended, 8).unwrap();
    assert_eq!((behind.act, behind.at), (PlayerAct::TakeBribe, fact));
    assert_eq!(behind.depth, 5);
    assert_eq!(
        graph::nearest_player_act(&log, ended, 4),
        None,
        "depth-limited"
    );
    // The other scandal led nowhere.
    assert!(graph::consequences(&log, other, 8).is_empty());
    assert_eq!(
        log.get(charged).unwrap().links().next(),
        Some((opened, Link::Continues))
    );
    assert_eq!(log.head(Thread::Case(1)), Some(convicted));
}

fn turn(log: &mut EventLog, on: SimDate, n: u32) -> EventId {
    log.record(
        on,
        EventKind::TurnEnded { turn: n },
        Cause::Process(Process::Calendar),
    )
}

fn linked(log: &EventLog) -> bool {
    log.iter()
        .all(|e| e.links().all(|(c, _)| c < e.id && log.get(c).is_some()))
}

#[test]
fn events_leave_by_age_and_salience() {
    let mut log = EventLog::default();
    let trace = turn(&mut log, day(0), 1);
    let routine = log.record(
        day(0),
        EventKind::CampaignOpened {
            election_day: day(60),
        },
        Cause::Process(Process::Elections),
    );
    let notable = log.record(
        day(0),
        EventKind::Disaster {
            municipality: 2,
            hazard: crate::environment::HazardKind::Flood,
            severity_pct: 40,
        },
        Cause::Exogenous(Shock::Hazard),
    );
    let landmark = log.record(
        day(0),
        EventKind::PlayerDeselected,
        [
            Cause::Pressure(Pressure::Track),
            Cause::Process(Process::Parties),
        ],
    );
    let pinned = turn(&mut log, day(0), 2);
    log.pin(pinned);
    let none = Holds::default();
    let kept = |log: &EventLog, id| log.get(id).is_some();
    let all = [trace, routine, notable, landmark, pinned];
    let after = |days: u32| {
        let mut l = log.clone();
        prune::prune(&mut l, day(days), &none);
        all.map(|id| kept(&l, id))
    };
    assert_eq!(after(prune::TRACE_DAYS), [true; 5]);
    assert_eq!(
        after(prune::TRACE_DAYS + 1),
        [false, true, true, true, true]
    );
    assert_eq!(
        after(prune::ROUTINE_DAYS + 1),
        [false, false, true, true, true]
    );
    assert_eq!(
        after(prune::NOTABLE_DAYS + 1),
        [false, false, false, true, true]
    );
    assert_eq!(after(100 * 365), [false, false, false, true, true]);
}

#[test]
fn what_is_kept_keeps_its_chain_and_live_state_holds() {
    let mut log = EventLog::default();
    // An old bill, enacted long ago, still in force.
    let intro = log.record(
        day(0),
        EventKind::PolicyIntroduced {
            policy: 1,
            municipality: 0,
            sponsor: 3,
        },
        Cause::Actor(person(3)),
    );
    let vote = log.record(
        day(5),
        EventKind::PlayerDeed {
            deed: Deed::Vote {
                policy: 1,
                vote: Vote::No,
            },
        },
        Cause::Player(PlayerAct::Vote),
    );
    let enacted = log.record(
        day(20),
        EventKind::PolicyEnacted {
            policy: 1,
            municipality: 0,
            yes: 4,
            no: 3,
            overrode_veto: false,
        },
        Causes::from(Cause::Process(Process::Lawmaking)).and_event(Some(intro), Link::Continues),
    );
    // Another old bill that failed: nothing holds it.
    let dead = log.record(
        day(0),
        EventKind::PolicyIntroduced {
            policy: 2,
            municipality: 0,
            sponsor: 3,
        },
        Cause::Actor(person(3)),
    );
    let failed = log.record(
        day(20),
        EventKind::PolicyFailed {
            policy: 2,
            municipality: 0,
            vetoed: false,
            yes: 1,
            no: 6,
        },
        Cause::Process(Process::Lawmaking),
    );
    let later = day(prune::NOTABLE_DAYS + 400);
    // Nothing live: all of it is past its age.
    let mut gone = log.clone();
    prune::prune(&mut gone, later, &Holds::default());
    assert!(gone.is_empty());
    assert!(gone.heads().is_empty(), "a thread ends with its events");
    // The law in force holds its thread's latest event, and with it the
    // whole chain: the introduction and the player's vote (a trace event,
    // long past a year).
    let live = Holds {
        threads: vec![Thread::Bill(1)],
        ..Holds::default()
    };
    let mut held = log.clone();
    let done = prune::prune(&mut held, later, &live);
    assert_eq!((done.before, done.after, done.capped), (5, 3, 0));
    for id in [intro, vote, enacted] {
        assert!(held.get(id).is_some());
    }
    assert!(held.get(dead).is_none() && held.get(failed).is_none());
    assert!(linked(&held));
    assert_eq!(held.head(Thread::Bill(1)), Some(enacted));
    // An event live state names holds its chain the same way.
    let named = Holds {
        events: vec![failed],
        ..Holds::default()
    };
    let mut by_name = log.clone();
    prune::prune(&mut by_name, later, &named);
    assert!(by_name.get(failed).is_some() && by_name.get(dead).is_some());
    assert!(by_name.get(enacted).is_none());
    // A skip in progress keeps everything it has yet to read.
    let floor = Holds {
        floor: Some(dead),
        ..Holds::default()
    };
    let mut skipping = log.clone();
    prune::prune(&mut skipping, later, &floor);
    assert!(skipping.get(dead).is_some() && skipping.get(failed).is_some());
    assert!(skipping.get(enacted).is_none());
}

#[test]
fn a_town_in_crisis_holds_its_closures() {
    let mut log = EventLog::default();
    let closure = |log: &mut EventLog, town: u32, on: SimDate| {
        log.record(
            on,
            EventKind::PlantClosure {
                municipality: town,
                sector: "sector.manufacturing".into(),
                jobs: 100,
            },
            Cause::Pressure(Pressure::Competitiveness),
        )
    };
    let before = closure(&mut log, 1, day(0));
    let here = closure(&mut log, 1, day(500));
    let elsewhere = closure(&mut log, 2, day(500));
    let holds = Holds {
        towns: vec![(1, day(400))],
        ..Holds::default()
    };
    prune::prune(&mut log, day(500 + prune::NOTABLE_DAYS + 1), &holds);
    assert!(log.get(here).is_some());
    assert!(log.get(before).is_none() && log.get(elsewhere).is_none());
}

#[test]
fn the_hard_cap_drops_the_least_salient_first_and_never_a_live_chain() {
    let mut log = EventLog::default();
    let root = log.record(
        day(0),
        EventKind::GlobalRecessionBegan { depth_pct: 5 },
        Cause::Exogenous(Shock::WorldEconomy),
    );
    // A live chain through an old trace event.
    let deed = log.record(
        day(1),
        EventKind::PlayerDeed {
            deed: Deed::Vote {
                policy: 1,
                vote: Vote::Yes,
            },
        },
        Cause::Player(PlayerAct::Vote),
    );
    let mut closures = Vec::new();
    for i in 0..prune::MAX_EVENTS as u32 {
        turn(&mut log, day(2), i);
        if i % 100 == 0 {
            closures.push(
                log.record(
                    day(2),
                    EventKind::PlantClosure {
                        municipality: 0,
                        sector: "sector.manufacturing".into(),
                        jobs: 10,
                    },
                    Causes::from(Cause::Pressure(Pressure::WorldDemand))
                        .and_event(Some(root), Link::Caused),
                ),
            );
        }
    }
    let n = log.len();
    assert!(n > prune::MAX_EVENTS);
    let holds = Holds {
        threads: vec![Thread::Bill(1)],
        ..Holds::default()
    };
    let done = prune::prune(&mut log, day(3), &holds);
    assert_eq!(done.after, prune::MAX_EVENTS);
    assert_eq!(done.capped, n - prune::MAX_EVENTS);
    // Trace events went (oldest first); the notable ones and the live
    // chain's trace event stayed.
    assert!(closures.iter().all(|c| log.get(*c).is_some()));
    assert!(log.get(deed).is_some() && log.get(root).is_some());
    assert!(linked(&log));
    // Dropping a cause leaves a mark, not a dangling link.
    let mut small = EventLog::default();
    let a = small.record(
        day(0),
        EventKind::GlobalRecessionBegan { depth_pct: 5 },
        Cause::Exogenous(Shock::WorldEconomy),
    );
    let b = small.record(
        day(1),
        EventKind::GlobalRecoveryBegan,
        Cause::Exogenous(Shock::WorldEconomy),
    );
    assert_eq!(small.get(b).unwrap().links().next(), Some((a, Link::Ended)));
    small.events.remove(0);
    prune::mend(&mut small);
    assert_eq!(
        small.get(b).unwrap().causes,
        vec![Cause::Exogenous(Shock::WorldEconomy), Cause::Forgotten]
    );
    assert!(linked(&small));
}

/// The rule's third clause: nothing from the oldest live reader's cursor on
/// goes by age, whatever its kind; what the reader has read still does.
#[test]
fn nothing_unread_goes_by_age() {
    let mut log = EventLog::default();
    let routine = |log: &mut EventLog, on: SimDate| {
        log.record(
            on,
            EventKind::CampaignOpened {
                election_day: on.add_days(60),
            },
            Cause::Process(Process::Elections),
        )
    };
    // Read: a turn and a campaign, then the reader stops (the director,
    // as the player goes to prison).
    let read_trace = turn(&mut log, day(0), 1);
    let read_routine = routine(&mut log, day(0));
    // Unread: the same kinds, a day later, and four years of turns.
    let unread_trace = turn(&mut log, day(1), 2);
    let unread_routine = routine(&mut log, day(1));
    let polls = log.record(
        day(30),
        EventKind::PollsPublished { count: 3 },
        Cause::Process(Process::Elections),
    );
    let waiting = Holds {
        floor: Some(unread_trace),
        ..Holds::default()
    };
    // Four years on (a sentence), and a day past the routine horizon.
    let release = day(prune::ROUTINE_DAYS + 2);
    let mut held = log.clone();
    let done = prune::prune(&mut held, release, &waiting);
    assert_eq!((done.before, done.after, done.capped), (5, 3, 0));
    assert!(held.get(read_trace).is_none() && held.get(read_routine).is_none());
    for id in [unread_trace, unread_routine, polls] {
        assert!(held.get(id).is_some(), "unread, so kept: {id:?}");
    }
    // Without a waiting reader all five are past their age.
    let mut gone = log.clone();
    prune::prune(&mut gone, release, &Holds::default());
    assert!(gone.is_empty());
    // Once the reader has caught up, the next prune lets them go.
    let caught_up = Holds {
        floor: Some(EventId(polls.0 + 1)),
        ..Holds::default()
    };
    prune::prune(&mut held, release, &caught_up);
    assert!(held.is_empty());
}

/// At the hard cap: what only its age keeps goes first; then the unread,
/// oldest first whatever their kind; a live chain never.
#[test]
fn at_the_cap_the_unread_go_last_and_oldest_first() {
    let over = 40;
    let build = |read: usize| {
        let mut log = EventLog::default();
        // A live law's chain, oldest of all.
        let intro = log.record(
            day(0),
            EventKind::PolicyIntroduced {
                policy: 1,
                municipality: 0,
                sponsor: 2,
            },
            Cause::Actor(person(2)),
        );
        // Read, young bookkeeping (kept by age).
        for i in 0..read {
            turn(&mut log, day(1), i as u32);
        }
        // Unread from here: a notable closure first, then turns.
        let closure = log.record(
            day(2),
            EventKind::PlantClosure {
                municipality: 0,
                sector: "sector.manufacturing".into(),
                jobs: 10,
            },
            Cause::Pressure(Pressure::WorldDemand),
        );
        while log.len() < prune::MAX_EVENTS + over {
            let n = log.len() as u32;
            turn(&mut log, day(2), n);
        }
        (log, intro, closure)
    };
    // Enough read events to absorb the excess: nothing unread is touched.
    let (mut log, intro, closure) = build(over + 10);
    let holds = Holds {
        floor: Some(closure),
        threads: vec![Thread::Bill(1)],
        ..Holds::default()
    };
    let done = prune::prune(&mut log, day(3), &holds);
    assert_eq!((done.after, done.capped), (prune::MAX_EVENTS, over));
    assert!(log.get(intro).is_some() && log.get(closure).is_some());
    assert_eq!(log.after(closure).len(), prune::MAX_EVENTS - 12);
    assert_eq!(log.iter().filter(|e| e.id < closure).count(), 11);
    // Too few read events: the read go, then the oldest unread, the
    // notable closure among them; the live chain stays.
    let (mut log, intro, closure) = build(5);
    let holds = Holds {
        floor: Some(closure),
        threads: vec![Thread::Bill(1)],
        ..Holds::default()
    };
    let first_unread_turn = EventId(closure.0 + 1);
    let done = prune::prune(&mut log, day(3), &holds);
    assert_eq!((done.after, done.capped), (prune::MAX_EVENTS, over));
    assert!(log.get(intro).is_some(), "live");
    assert!(
        log.get(closure).is_none(),
        "the oldest unread goes first, whatever its kind"
    );
    assert!(log.get(first_unread_turn).is_none());
    // Exactly the oldest `over - 5` unread went; everything newer is there.
    let oldest_kept = log.after(intro).first().unwrap().id;
    assert_eq!(oldest_kept.0, closure.0 + (over as u32 - 5));
    assert!(linked(&log));
}

/// A thread's last event stays its head after it closes, for what is said
/// of it later; but a closed thread prompts and continues nothing.
#[test]
fn a_closed_thread_has_a_head_but_not_an_open_one() {
    let mut log = EventLog::default();
    let opened = log.record(
        day(0),
        EventKind::ProposalOpened {
            proposal: 5,
            family: "family.x".into(),
            at: crate::world::Jurisdiction::town(crate::world::MunicipalityId(0)),
        },
        Cause::Process(Process::Agenda),
    );
    assert_eq!(log.open_head(Thread::Proposal(5)), Some(opened));
    let lapsed = log.record(
        day(40),
        EventKind::ProposalLapsed {
            proposal: 5,
            family: "family.x".into(),
            at: crate::world::Jurisdiction::town(crate::world::MunicipalityId(0)),
        },
        Cause::Process(Process::Agenda),
    );
    assert_eq!(log.head(Thread::Proposal(5)), Some(lapsed));
    assert_eq!(log.open_head(Thread::Proposal(5)), None);
    assert_eq!(log.open_head(Thread::Proposal(6)), None);
    // A director's lever over a town is not a thread: holds link their own.
    let lever = EventKind::DirectorIntervention {
        lever: "hazard_held".into(),
        municipality: 3,
    };
    assert_eq!(thread(&lever, day(0)), None);
}

#[test]
fn pruning_is_about_monthly() {
    let mut log = EventLog::default();
    let old = turn(&mut log, day(0), 1);
    let today = day(prune::TRACE_DAYS + 10);
    // A reader has yet to read `old`: it stays.
    let mark = Holds {
        floor: Some(old),
        ..Holds::default()
    };
    assert!(log.prune_if_due(today, || mark.clone()).is_some());
    assert!(log.get(old).is_some());
    // Not due again for a month, whatever the holds.
    assert!(
        log.prune_if_due(today.add_days(27), Holds::default)
            .is_none()
    );
    assert!(log.get(old).is_some());
    assert!(
        log.prune_if_due(today.add_days(28), Holds::default)
            .is_some()
    );
    assert!(log.get(old).is_none());
}

#[test]
fn every_kind_has_a_key_a_salience_and_at_most_one_thread() {
    // The serde tag and `kind_key` agree (the protocol and the audit name
    // kinds by it).
    let samples = [
        EventKind::GameStarted { seed: 1 },
        EventKind::GlobalRecoveryBegan,
        EventKind::PlayerDeselected,
        EventKind::TurnEnded { turn: 1 },
        EventKind::FactDiscoverable { hidden: 1 },
        EventKind::ElectionHeld { races: 3 },
    ];
    for k in samples {
        let tag = serde_json::to_value(&k).unwrap()["type"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(tag, kind_key(&k));
        let _ = salience(&k);
    }
    // A cause round-trips through the save's JSON, compactly.
    let causes = vec![
        Cause::Event(EventId(7), Link::Prompted),
        Cause::Player(PlayerAct::Vote),
        Cause::Exogenous(Shock::Hazard),
        Cause::History,
        Cause::Unknown,
    ];
    let json = serde_json::to_string(&causes).unwrap();
    assert_eq!(
        json,
        r#"[{"event":[7,"prompted"]},{"player":"vote"},{"exogenous":"hazard"},"history","unknown"]"#
    );
    assert_eq!(serde_json::from_str::<Vec<Cause>>(&json).unwrap(), causes);
    assert_eq!(best_class(&causes), CauseClass::Linked);
    assert_eq!(best_class(&[Cause::Unknown]), CauseClass::Unknown);
    assert_eq!(best_class(&[]), CauseClass::Unknown);
}
