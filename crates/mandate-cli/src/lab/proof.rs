//! The causal spine changes no outcome (S1). Two digests of a played game,
//! pinned at the commit before the spine and unchanged by it:
//!
//! - the **event stream**: every event's id, date and kind in the order
//!   recorded (never its causes), read after every command;
//! - the **state without the spine**: the whole state as the save writes
//!   it, less the event log and the fields that only carry causes.
//!
//! Each game is played twice. With the log's pruning on, as players have
//! it, the state digest must match: pruning changes nothing but the log.
//! With the lab switch that keeps every event, the stream must match (and
//! the state again): a command that spans years (a prison term) would
//! otherwise prune its early bookkeeping events before anyone could read
//! them.
//!
//! Games: three seeds, each idle and with one active bot, eight years; and
//! two whose player spends years in prison (a lab edit of the status: four
//! years, the longest a career survives, and six), where the director
//! reads nothing for the whole term and the whole term on release: the log
//! must have kept everything it had not read.
//!
//! **The pins are a golden**, like the v18 continuation: they were recorded
//! from the build without the spine (at e7916f9, and again at c72cbbd
//! after the rents fix changed outcomes) and the build with it matches
//! them. A later commit that changes what happens in a game on purpose (a
//! balance fix, a new system) moves them: re-record with
//! `SPINE_PROOF_PRINT=1 cargo test -p mandate-cli proof -- --nocapture`,
//! paste the six lines, and say why in the commit. The half that never
//! moves is the first: pruning on or off, the state is the same.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]

use super::runner::{Driver, Game, Setup};
use mandate_core::hash::StableHash;
use serde_json::Value;

/// Keys that only carry causes or the spine's own bookkeeping: left out of
/// the state digest on both sides of the change.
const SPINE_KEYS: [&str; 5] = ["cause", "causes", "source_event", "last_post", "hold_event"];

fn strip(v: &mut Value) {
    match v {
        Value::Object(m) => {
            for k in SPINE_KEYS {
                m.remove(k);
            }
            for x in m.values_mut() {
                strip(x);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(strip),
        _ => {}
    }
}

/// Play months `months` (counted from the game's start, from 1) as the
/// runner does (`Game::step_month`), calling
/// `after` after every command that moves time. The log is pruned only
/// between commands, and never of what the command just recorded, so a
/// reader called this way misses nothing (a prison term passes in one
/// command).
pub(crate) fn play_by_command(
    g: &mut Game,
    months: std::ops::RangeInclusive<u32>,
    after: &mut dyn FnMut(&Game),
) {
    let strategy = g.driver.name();
    for month in months {
        let target = super::runner::month_start(g.start, month);
        while g.e.state().calendar.date < target {
            let active =
                g.e.state()
                    .player
                    .as_ref()
                    .is_some_and(|p| p.status.active());
            if active && strategy != "idle" {
                crate::bots::play_turn_from(&mut g.e, &g.content, &strategy, g.start_turn);
                after(g);
            }
            if g.e.apply(mandate_sim::Command::EndTurn).is_err() {
                // The career ended: the world goes on without turns.
                let days = g.e.state().calendar.date.days_until(target);
                if days > 0 {
                    let _ = g.e.apply(mandate_sim::Command::AdvanceDays { days });
                }
                after(g);
                break;
            }
            after(g);
        }
    }
}

/// A lab edit for the proof: after `after` months the player is put in
/// prison for `years` years (no case, no sentence: only the status), so
/// the director waits out a whole term and then reads the log from where
/// it stopped. Four years is the longest sentence a career survives.
#[derive(Clone, Copy)]
struct Prison {
    after: u32,
    years: u32,
}

/// (event stream digest, state digest, events seen). `keep_all`: the lab
/// switch that turns the log's pruning off.
fn digests(
    seed: u64,
    driver: Driver,
    months: u32,
    prison: Option<Prison>,
    keep_all: bool,
) -> (String, String, usize) {
    let content = crate::lab::load(&super::runner::content_root(), &[]).unwrap();
    let mut g = Game::new(&content, &Setup::small(seed, None), driver).unwrap();
    g.e.setup(|s| s.events.set_keep_all(keep_all));
    let mut stream = Vec::<u8>::new();
    // The history's last id (how many events the burn-in recorded), then
    // its last 300 days (kept under any pruning rule), then play.
    let start = g.e.state().calendar.date;
    let newest = g.e.state().events.iter().last().map(|e| e.id.0);
    stream.extend_from_slice(format!("{newest:?}\n").as_bytes());
    let mut last: Option<u32> =
        g.e.state()
            .events
            .iter()
            .take_while(|e| e.date.0 + 300 < start.0)
            .last()
            .map(|e| e.id.0);
    let mut seen = 0usize;
    let mut fold = |g: &Game, last: &mut Option<u32>| {
        let from = *last;
        for ev in
            g.e.state()
                .events
                .iter()
                .filter(|x| from.is_none_or(|l| x.id.0 > l))
        {
            let line = serde_json::to_string(&(ev.id.0, ev.date.0, &ev.kind)).unwrap();
            stream.extend_from_slice(line.as_bytes());
            stream.push(b'\n');
            *last = Some(ev.id.0);
            seen += 1;
        }
    };
    fold(&g, &mut last);
    let free = prison.map_or(months, |p| p.after.min(months));
    play_by_command(&mut g, 1..=free, &mut |g| fold(g, &mut last));
    if let Some(p) = prison {
        g.e.setup(|s| {
            let until = s.calendar.date.add_days(p.years * 365);
            if let Some(pl) = s.player.as_mut() {
                pl.status = mandate_sim::player::RunStatus::Imprisoned { until, case: 0 };
            }
        });
        play_by_command(&mut g, free + 1..=months, &mut |g| fold(g, &mut last));
        let released =
            g.e.state()
                .player
                .as_ref()
                .is_some_and(|pl| pl.status.active());
        assert!(released, "the term ended within the game");
    }
    // `SPINE_PROOF_DUMP=<dir>` writes the stream as text, to find what
    // differs between two commits.
    if let Ok(dir) = std::env::var("SPINE_PROOF_DUMP") {
        let name = format!(
            "{dir}/{seed}-{}{}.txt",
            g.driver.name(),
            if prison.is_some() { "+prison" } else { "" }
        );
        std::fs::write(name, &stream).unwrap();
    }
    let mut state = serde_json::to_value(g.e.state()).unwrap();
    state.as_object_mut().unwrap().remove("events");
    strip(&mut state);
    let state = serde_json::to_vec(&state).unwrap();
    (
        StableHash::of_bytes(&stream).short(),
        StableHash::of_bytes(&state).short(),
        seen,
    )
}

/// (seed, game, months, event stream, state without the spine), recorded
/// from the build **without** the spine and matched by the build with it.
/// First at e7916f9 (the six games, then the two prison games); then again
/// at c72cbbd, the branch head with the rents and migration fix (audit fix
/// 2, which changes outcomes on purpose), built as it is, without the
/// spine: these are that build's digests, and the merged tree matches them.
const PINS: [(u64, &str, u32, &str, &str); 8] = [
    (2026, "idle", 96, "87c7f491cc59eb96", "1b62fa0a7e227026"), // 1241 events
    (2026, "mixed", 96, "30513a0fd79f8345", "037e0eed3baa655d"), // 2014 events
    (7, "idle", 96, "0c7529b9245d03ca", "3933a142c80a2378"),    // 1179 events
    (7, "corrupt", 96, "c3167aacfc06bb48", "a01a320061529ea6"), // 1904 events
    (31, "idle", 96, "2e81ed540a34aee5", "306c1e59ac04695a"),   // 1227 events
    (
        31,
        "explorer_legislator",
        96,
        "604610467beb7465",
        "88c6e22e0c0b2907",
    ), // 3086 events
    (
        2026,
        "mixed+prison",
        96,
        "2baccbdaf9b90ad5",
        "c21042eca3d47ed9",
    ), // 1702 events
    (
        2026,
        "idle+six_years",
        120,
        "8fecd3f0c7ad80a8",
        "e86a42d58483a21e",
    ), // 1431 events
];

#[test]
fn the_spine_changes_no_outcome() {
    let print = std::env::var("SPINE_PROOF_PRINT").is_ok();
    // The last two games go to prison: the director reads nothing for the
    // whole term and everything on release, so what the log kept decides
    // what it finds. Four years is the longest sentence a career survives.
    // Under the engine's rules that term alone never loses an unread event
    // (the director reads in the turn of the release, before the routine
    // events' four years are up), so the second term is six years, which
    // no court gives: without the rule "nothing unread goes by age" that
    // game's state changed (at e7916f9, where it was added), and with it
    // both match the build without the spine.
    let term = Some(Prison {
        after: 24,
        years: 4,
    });
    let long = Some(Prison {
        after: 36,
        years: 6,
    });
    let games: [(u64, &str, u32, Option<Prison>); 8] = [
        (2026, "idle", 96, None),
        (2026, "mixed", 96, None),
        (7, "idle", 96, None),
        (7, "corrupt", 96, None),
        (31, "idle", 96, None),
        (31, "explorer_legislator", 96, None),
        (2026, "mixed+prison", 96, term),
        (2026, "idle+six_years", 120, long),
    ];
    for (seed, label, months, prison) in games {
        let strategy = label.split('+').next().unwrap();
        let (stream, state, seen) = digests(seed, Driver::parse(strategy), months, prison, true);
        // As players have it (pruning on): the same state.
        let (_, pruned_state, _) = digests(seed, Driver::parse(strategy), months, prison, false);
        assert_eq!(
            state, pruned_state,
            "seed {seed}, {label}: pruning the log changed the state"
        );
        if print {
            println!(
                "    ({seed}, \"{label}\", {months}, \"{stream}\", \"{state}\"), // {seen} events"
            );
            continue;
        }
        let pin = PINS
            .iter()
            .find(|p| p.0 == seed && p.1 == label && p.2 == months)
            .unwrap_or_else(|| panic!("no pin for seed {seed} {label}"));
        assert_eq!(
            (stream.as_str(), state.as_str()),
            (pin.3, pin.4),
            "seed {seed}, {label}: the event stream or the state changed. If this commit \
             changes outcomes on purpose, re-record the pins (see this file's header)"
        );
    }
}
