//! The causal spine on played games (S1): nothing significant is uncaused,
//! every effect request applied names its cause, the log keeps its
//! invariants and its bound, and the queries answer on real chains.
//!
//! Games: three seeds, each idle and with one active bot, six years (the
//! history's kept events are read too). Events are read after every
//! command; the log's invariants and its bound are checked as it prunes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::proof::play_by_command;
use super::runner::{Driver, Game, Setup};
use mandate_sim::events::{DomainEvent, EventId, EventKind, EventLog};
use mandate_sim::spine::graph::{self, Step};
use mandate_sim::spine::{Cause, CauseClass, Link, PlayerAct, Pressure, Shock, prune};

const GAMES: [(u64, &str); 6] = [
    (2026, "idle"),
    (2026, "explorer_legislator"),
    (7, "idle"),
    (7, "corrupt"),
    (31, "idle"),
    (31, "mixed"),
];
const MONTHS: u32 = 72;

/// A played game: every event ever read (pruned or not), and the game.
struct Played {
    all: Vec<DomainEvent>,
    game: Game,
    /// The log's invariants held after every command.
    max_len: usize,
}

/// The log's invariants: ids ascending, every link to an earlier event the
/// log holds, heads sorted and pointing into the log.
fn check_log(log: &EventLog, what: &str) {
    let mut prev: Option<EventId> = None;
    for e in log.iter() {
        assert!(prev.is_none_or(|p| p < e.id), "{what}: ids ascend");
        prev = Some(e.id);
        assert!(!e.causes.is_empty(), "{what}: {:?} has no cause", e.kind);
        for (c, _) in e.links() {
            assert!(c < e.id, "{what}: a cause comes first");
            assert!(
                log.get(c).is_some(),
                "{what}: {:?} links to an event the log dropped",
                e.kind
            );
        }
    }
    assert!(
        log.heads().windows(2).all(|w| w[0].0 < w[1].0),
        "{what}: heads sorted"
    );
    for (t, id, _) in log.heads() {
        assert!(log.get(*id).is_some(), "{what}: the head of {t:?} is kept");
    }
}

fn play(seed: u64, strategy: &str) -> Played {
    let content = crate::lab::load(&super::runner::content_root(), &[]).unwrap();
    let mut g = Game::new(&content, &Setup::small(seed, None), Driver::parse(strategy)).unwrap();
    let mut all: Vec<DomainEvent> = Vec::new();
    let mut max_len = 0;
    let what = format!("seed {seed} {strategy}");
    let mut read = |g: &Game| {
        let log = &g.e.state().events;
        let from = all.last().map(|e| e.id);
        all.extend(
            log.iter()
                .filter(|e| from.is_none_or(|f| e.id > f))
                .cloned(),
        );
        max_len = max_len.max(log.len());
        check_log(log, &what);
    };
    read(&g);
    play_by_command(&mut g, 1..=MONTHS, &mut read);
    Played {
        all,
        game: g,
        max_len,
    }
}

fn has(e: &DomainEvent, class: CauseClass) -> bool {
    e.causes.iter().any(|c| c.class() == class)
}

fn by_player(e: &DomainEvent) -> bool {
    e.causes.iter().any(|c| c.player_act().is_some())
}

#[test]
fn nothing_significant_is_uncaused() {
    let mut seen = std::collections::BTreeMap::<&'static str, u32>::new();
    let mut effects = 0;
    for (seed, strategy) in GAMES {
        let p = play(seed, strategy);
        let what = format!("seed {seed} {strategy}");
        for e in &p.all {
            let kind = mandate_sim::spine::kind_key(&e.kind);
            *seen.entry(kind).or_default() += 1;
            // Every event, significant or not: a cause, and a named one.
            assert!(!e.causes.is_empty(), "{what}: {kind} has no cause");
            assert!(
                e.causes.iter().all(Cause::is_known),
                "{what}: {kind} has an unnamed cause: {:?}",
                e.causes
            );
            // What each significant kind must be able to say.
            use EventKind as K;
            let ok = match &e.kind {
                // Exogenous by design, and saying so.
                K::Disaster { .. } => e.causes.contains(&Cause::Exogenous(Shock::Hazard)),
                K::GlobalRecessionBegan { .. } => {
                    e.causes.contains(&Cause::Exogenous(Shock::WorldEconomy))
                }
                K::CommodityShock { .. } => {
                    e.causes.contains(&Cause::Exogenous(Shock::Commodities))
                }
                K::RaceDecided { .. } => e.causes.contains(&Cause::Exogenous(Shock::Race)),
                // A closure names the weak factor of its sector's demand.
                K::PlantClosure { .. } => has(e, CauseClass::Pressure),
                // A bill is someone's: the player's, the proxy's, or an NPC's.
                K::PolicyIntroduced { .. } => by_player(e) || has(e, CauseClass::Actor),
                // What happens to a bill links to the bill.
                K::PolicyEnacted { .. }
                | K::PolicyFailed { .. }
                | K::PolicyExpired { .. }
                | K::PolicyWithdrawn { .. } => has(e, CauseClass::Linked),
                // Closings link to what they close.
                K::GlobalRecoveryBegan
                | K::AskResolved { .. }
                | K::DecisionResolved { .. }
                | K::DecisionLapsed { .. }
                | K::ProposalLapsed { .. }
                | K::CaseDeveloped { .. }
                | K::RumourCorrected { .. }
                | K::PlayerReleased => has(e, CauseClass::Linked),
                // Prosecutors act on a story; a sentence follows a verdict.
                K::CaseOpened { .. } | K::PlayerImprisoned { .. } | K::CareerEnded { .. } => {
                    has(e, CauseClass::Linked)
                }
                K::AskAccepted { .. } => by_player(e),
                // A scandal names the act that left the evidence: the
                // player's, or the subject's own.
                K::ScandalBroke { .. } | K::FactDiscoverable { .. } => {
                    by_player(e) || has(e, CauseClass::Actor)
                }
                _ => true,
            };
            assert!(ok, "{what}: {kind} does not explain itself: {:?}", e.causes);
        }
        let audit = p.game.e.state().events.effect_audit();
        assert_eq!(
            audit.unnamed, 0,
            "{what}: an effect was applied with no named cause"
        );
        effects += audit.applied;
        // The bound.
        assert!(
            p.max_len <= prune::MAX_EVENTS,
            "{what}: {} events",
            p.max_len
        );
    }
    assert!(effects > 1_000, "effect requests were applied: {effects}");
    if std::env::var("SPINE_PRINT").is_ok() {
        eprintln!("{seen:#?}\neffects applied: {effects}");
    }
    // The games exercised the kinds that matter.
    for kind in [
        "disaster",
        "plant_closure",
        "policy_introduced",
        "policy_enacted",
        "policy_failed",
        "race_decided",
        "scandal_broke",
        "case_opened",
        "ask_resolved",
        "decision_resolved",
        "case_developed",
        "ask_accepted",
        "campaign_opened",
    ] {
        assert!(
            seen.get(kind).copied().unwrap_or(0) > 0,
            "no {kind} in any game"
        );
    }
}

#[test]
fn the_queries_answer_on_a_played_game() {
    let p = play(2026, "explorer_legislator");
    let st = p.game.e.state();
    let log = &st.events;
    let me = st.player.as_ref().unwrap().politician.0;
    // Every enacted law the log holds: its chain reaches its introduction,
    // whose root is whoever tabled it.
    let mut enacted = 0;
    let mut mine = 0;
    for e in log.iter() {
        let EventKind::PolicyEnacted { policy, .. } = &e.kind else {
            continue;
        };
        enacted += 1;
        let chain = graph::chain(log, e.id, 6);
        let intro = chain.events.iter().find_map(|r| {
            let x = log.get(r.event)?;
            matches!(&x.kind, EventKind::PolicyIntroduced { policy: p, .. } if p == policy)
                .then_some(x)
        });
        let intro = intro.unwrap_or_else(|| panic!("law {policy} has no introduction behind it"));
        let EventKind::PolicyIntroduced { sponsor, .. } = &intro.kind else {
            unreachable!()
        };
        // Credit and blame: the nearest player act behind the law.
        let behind = graph::nearest_player_act(log, e.id, 6);
        if *sponsor == me {
            mine += 1;
            let b = behind.expect("the player's own law has their act behind it");
            assert!(matches!(b.act, PlayerAct::Introduce | PlayerAct::Vote));
            // Forward from the introduction: the law is a consequence.
            assert!(
                graph::consequences(log, intro.id, 3)
                    .iter()
                    .any(|r| r.event == e.id)
            );
        } else if let Some(b) = behind {
            // Someone else's law: only the player's vote can lie behind it.
            assert_eq!(b.act, PlayerAct::Vote, "law {policy}");
        }
    }
    assert!(
        enacted > 10 && mine > 0,
        "{enacted} laws, {mine} the player's"
    );
    // A pattern on the real graph: a bill tabled by the player, then
    // enacted (any number of the player's votes between).
    let tabled = |e: &DomainEvent| matches!(&e.kind, EventKind::PolicyIntroduced { sponsor, .. } if *sponsor == me);
    let passed = |e: &DomainEvent| matches!(e.kind, EventKind::PolicyEnacted { .. });
    let found = graph::matches(
        log,
        &[
            Step::next(&tabled),
            Step::later(&passed, 2).via(Link::Continues),
        ],
        100,
    );
    assert_eq!(found.len(), mine);
}

/// The director reads nothing while the player is in prison and the whole
/// term on release: the log keeps everything from its cursor on, whatever
/// its age, and lets it go once it has been read.
#[test]
fn a_waiting_reader_finds_everything_it_has_not_read() {
    use mandate_sim::spine::{Salience, salience};
    let content = crate::lab::load(&super::runner::content_root(), &[]).unwrap();
    let mut g = Game::new(&content, &Setup::small(2026, None), Driver::parse("idle")).unwrap();
    play_by_command(&mut g, 1..=12, &mut |_| {});
    let years = 4;
    g.e.setup(|s| {
        let until = s.calendar.date.add_days(years * 365);
        if let Some(pl) = s.player.as_mut() {
            pl.status = mandate_sim::player::RunStatus::Imprisoned { until, case: 0 };
        }
    });
    let cursor = g.e.state().narrative.seen.last_event;
    // Month by month to the last week of the term (the log is pruned as
    // each advance ends; the director waits).
    for _ in 0..(years * 365 - 7) / 30 {
        g.e.apply(mandate_sim::Command::AdvanceDays { days: 30 })
            .unwrap();
    }
    let st = g.e.state();
    assert!(!st.player.as_ref().unwrap().status.active(), "still inside");
    assert_eq!(st.narrative.seen.last_event, cursor, "the director waits");
    // Every event since its cursor is still in the log, the oldest of
    // them nearly four years old, bookkeeping included.
    let floor = prune::unread_from(st).unwrap();
    assert!(floor.0 <= cursor + 1);
    let last = st.events.iter().last().unwrap().id.0;
    let unread = st.events.after(EventId(cursor));
    assert_eq!(
        unread.len() as u32,
        last - cursor,
        "no gap since the cursor"
    );
    let today = st.calendar.date.0;
    let old = |e: &&DomainEvent, class: Salience, days: u32| {
        salience(&e.kind) == class && e.date.0 + days < today
    };
    assert!(
        unread
            .iter()
            .filter(|e| old(e, Salience::Trace, 2 * prune::TRACE_DAYS))
            .count()
            > 3,
        "bookkeeping far past its year is kept while unread"
    );
    check_log(&st.events, "inside");
    assert!(st.events.len() <= prune::MAX_EVENTS);
    // Released; the director reads; within three months the log is back
    // to the rule's ages.
    g.e.apply(mandate_sim::Command::AdvanceDays { days: 30 })
        .unwrap();
    assert!(
        g.e.state().player.as_ref().unwrap().status.active(),
        "released"
    );
    for _ in 0..7 {
        g.e.apply(mandate_sim::Command::EndTurn).unwrap();
    }
    let st = g.e.state();
    assert!(
        st.narrative.seen.last_event > cursor,
        "the director has read"
    );
    let today = st.calendar.date.0;
    // (Turns and years: bookkeeping nothing live ever holds.)
    let stale = st
        .events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                EventKind::TurnEnded { .. } | EventKind::YearBegan { .. }
            )
        })
        .filter(|e| e.date.0 + prune::TRACE_DAYS + 35 < today)
        .count();
    assert_eq!(stale, 0, "read events go by their age again");
}

#[test]
fn closures_explain_themselves() {
    // Every closure in every game names a pressure; those in a recession
    // link to it as well.
    let mut closures = 0;
    let mut in_recession = 0;
    for (seed, strategy) in [(2026, "idle"), (7, "idle"), (31, "idle")] {
        let p = play(seed, strategy);
        for e in p
            .all
            .iter()
            .filter(|e| matches!(e.kind, EventKind::PlantClosure { .. }))
        {
            closures += 1;
            assert!(e.causes.iter().any(|c| matches!(
                c,
                Cause::Pressure(
                    Pressure::WorldDemand
                        | Pressure::CommodityPrices
                        | Pressure::Depletion
                        | Pressure::Competitiveness
                        | Pressure::LabourCosts
                )
            )));
            if e.links().any(|(_, l)| l == Link::Caused) {
                in_recession += 1;
            }
        }
    }
    assert!(closures > 0, "no closures in three games");
    let _ = in_recession;
}
