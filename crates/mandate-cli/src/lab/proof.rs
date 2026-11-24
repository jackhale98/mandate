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
/// switch that turns the log's pruning off. `arcs`: with the pack's arc
/// content (staged arcs, DEC-109), or with none.
fn digests(
    seed: u64,
    driver: Driver,
    months: u32,
    prison: Option<Prison>,
    keep_all: bool,
    arcs: bool,
) -> (String, String, usize) {
    let mut content = crate::lab::load(&super::runner::content_root(), &[]).unwrap();
    if !arcs {
        let mut bare = (*content).clone();
        bare.arcs.clear();
        content = std::sync::Arc::new(bare);
    }
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
    // The content hash covers the pack's text as well as its rules, so a
    // reworded line would move the digest without changing any outcome.
    if let Some(meta) = state.get_mut("meta").and_then(|m| m.as_object_mut()) {
        meta.remove("content_hash");
    }
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
/// The state digests were re-recorded once more at 7a54822 with
/// `meta.content_hash` left out of the state (the event streams did not
/// move): the hash covers the pack's text, so a reworded line moved the
/// digest without changing any outcome. The state digests of the four games
/// whose bot talks were re-recorded with M6 play note 19 (the event streams
/// did not move, and neither did the idle games): the conversation record
/// (`player.talk`) changed on purpose, as the first reply of the day stores
/// no opener (the greeting took its place), the greeting is in the
/// used-lines guard, and spoken-line arguments carry the centre's label.
/// With `player.talk` left out, all eight state digests equal the branch
/// head's computed the same way.
/// Re-recorded for audit fix 1 (the record decides votes; canvassing alone
/// builds no record), which changes outcomes on purpose: event streams and
/// states both moved. Recorded on the tree with both play note 19 and fix
/// 1: the event streams equal fix 1's own; the states of the four games
/// whose bot talks differ from fix 1's own, as they did with note 19.
///
/// **Staged arcs (A1, DEC-109) moved the two seed-31 games, and only
/// those.** A flood strikes the player's town in seed 31 (October of the
/// second year), so the base pack's flood arc is raised there: its events
/// join the stream (raised, its stages, the story in the paper, the
/// organiser's petition, the exit, and the ask and the decision they
/// prompt: fourteen more events in each game when recorded on 4c2a2c1;
/// thirteen and seventeen on the tree with audit fix 1, where the seed-31
/// pins and `BEFORE_ARCS` were re-recorded and the no-arc digests of all
/// eight games equal fix 1's pins exactly), and the state changes
/// through what its beats and its exit request and nothing else: the ask
/// it has the asks offer and the decision it has the narrative open (and
/// so the ids, the sliding rate and the pending count of the asks and
/// decisions that follow, by those systems' own rules), the default of
/// its meeting, and its exit's effects. The other six games have no flood
/// in the player's town within their years and did not move. With no arc
/// content every game still gives the digests from before arcs
/// (`BEFORE_ARCS` and the test below; all eight checked with
/// `SPINE_PROOF_NO_ARCS=all`).
const PINS: [(u64, &str, u32, &str, &str); 8] = [
    (2026, "idle", 96, "1e3b972b3b5abb07", "da5f10a54ca8de33"), // 1254 events
    (2026, "mixed", 96, "923e5b3d25a4ef04", "385af4cc9b5ce86b"), // 2390 events
    (7, "idle", 96, "69ca0ae3748c2891", "132d29526700f9fc"),    // 1146 events
    (7, "corrupt", 96, "3b3fa70238acff2d", "4d095a3a1b9407cb"), // 1490 events
    (31, "idle", 96, "5649ad10c492f916", "28702694276c8ec5"),   // 1290 events
    (
        31,
        "explorer_legislator",
        96,
        "3a542490ea1ec3f6",
        "7276af5520373e41",
    ), // 2132 events
    (
        2026,
        "mixed+prison",
        96,
        "012b16a022d7a492",
        "08cf93baae8862c4",
    ), // 1773 events
    (
        2026,
        "idle+six_years",
        120,
        "10407e999451b859",
        "0f82072b0f75f933",
    ), // 1484 events
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
        let (stream, state, seen) =
            digests(seed, Driver::parse(strategy), months, prison, true, true);
        // As players have it (pruning on): the same state.
        let (_, pruned_state, _) =
            digests(seed, Driver::parse(strategy), months, prison, false, true);
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

/// Staged arcs (DEC-109) with no arc content change nothing. The games of
/// `PINS` that an arc of the base pack touches (see the note above `PINS`),
/// as they were pinned before arcs existed: played with no arc content,
/// each still gives these digests. A game not listed here is pinned above
/// with the pack's arcs loaded and did not move, so it needs no second
/// pin. `SPINE_PROOF_NO_ARCS=all` plays every game of `PINS` with no arc
/// content and checks each against this table, else against `PINS`;
/// `=print` prints them.
const BEFORE_ARCS: [(u64, &str, u32, &str, &str); 2] = [
    (31, "idle", 96, "397a0facb6fa3f58", "ee852502bc130b86"), // 1277 events
    (
        31,
        "explorer_legislator",
        96,
        "fb722d09a2714fef",
        "441520496d110d90",
    ), // 2115 events
];

#[test]
fn with_no_arc_content_every_pin_holds() {
    let mode = std::env::var("SPINE_PROOF_NO_ARCS").unwrap_or_default();
    let prison = |label: &str| match label {
        "mixed+prison" => Some(Prison {
            after: 24,
            years: 4,
        }),
        "idle+six_years" => Some(Prison {
            after: 36,
            years: 6,
        }),
        _ => None,
    };
    for pin in PINS {
        let (seed, label, months) = (pin.0, pin.1, pin.2);
        let before = BEFORE_ARCS
            .iter()
            .find(|p| p.0 == seed && p.1 == label && p.2 == months);
        if before.is_none() && mode.is_empty() {
            continue;
        }
        let strategy = label.split('+').next().unwrap();
        let (stream, state, seen) = digests(
            seed,
            Driver::parse(strategy),
            months,
            prison(label),
            true,
            false,
        );
        if mode == "print" {
            println!(
                "    ({seed}, \"{label}\", {months}, \"{stream}\", \"{state}\"), // {seen} events"
            );
            continue;
        }
        let want = before.unwrap_or(&pin);
        assert_eq!(
            (stream.as_str(), state.as_str()),
            (want.3, want.4),
            "seed {seed}, {label}: with no arc content the game changed"
        );
    }
}
