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
/// pins and the no-arc pins were re-recorded and the no-arc digests of all
/// eight games equal fix 1's pins exactly), and the state changes
/// through what its beats and its exit request and nothing else: the ask
/// it has the asks offer and the decision it has the narrative open (and
/// so the ids, the sliding rate and the pending count of the asks and
/// decisions that follow, by those systems' own rules), the default of
/// its meeting, and its exit's effects. The other six games have no flood
/// in the player's town within their years and did not move. With no arc
/// content every game still gave the digests from before arcs.
///
/// **The director on arcs (A2, DEC-109) moved all eight games, with the
/// pack's arcs and with none; both tables were re-recorded** (on the tree
/// merged with play note 20, which alone moved no digest: the branch head
/// 8edba26 still gave every no-arc digest of before).
///
/// - *With no arc content* (`BEFORE_ARCS` below) a game is no longer the game
///   from before arcs, because two things that ran without arc content are
///   gone. Storylines were the director's own; they are arcs now, so with
///   no arc content none runs (38 `storyline_stage` events fewer over the
///   eight games, with the decisions they opened and what those moved).
///   And the director no longer makes a climax of its own: a chapter's
///   climax is its lead arc's turn, so with no arc there is none, and the
///   dilemma the old build opened as a chapter's climax (any significant
///   one, at the climax peak; a borrowed one as the fallback) is not
///   opened then (the first difference in seven of the eight games is
///   that decision, day 737499, in the first session's climax window; in
///   the eighth a storyline's first stage). The rest follows by the systems' own
///   rules: the later decisions' dates and ids, the asks' sliding rate,
///   and for the bots what they answered and so did. Compared kind by
///   kind with the streams of 8edba26 (`SPINE_PROOF_DUMP`): 38 storyline
///   stages and 10 decisions fewer, 21 `story_development` more (a quiet
///   chapter's news of the town where a decision stood), and in the three
///   games whose bot acts on decisions, the deeds, bills and schemes that
///   follow from different answers.
/// - *With the pack's arcs* (`PINS`) every game also has the budget round
///   each year (raised, its stages, its press story, its talk and visit
///   beats, its ask, its decisions, its exit and what the exit requests,
///   a carried change voted into the next budget among them), the rival's
///   arc in each campaign, the favour debt and the plant closure where
///   their causes hold, and the director's two new interventions
///   (`lead_arc`, `arc_clock`): 89 to 185 events more than the no-arc game
///   when first recorded.
///
/// Both tables were recorded again (`scripts/repin.sh`) on the tree merged
/// with audit fixes 3, 5 and 8 (c55bd17: laws pay off in approval, honest
/// ledgers), which move every game on their own account, with arcs and
/// without. The kind-by-kind counts above are from the recording before
/// that merge.
const PINS: [(u64, &str, u32, &str, &str); 8] = [
    (2026, "idle", 96, "2fd25c51e3adf5b9", "840b300c978e982d"), // 1434 events
    (2026, "mixed", 96, "a2b28ec8838962da", "97c57826c62198ab"), // 2611 events
    (7, "idle", 96, "81202894527d4d0f", "405d44aa2b9b54bc"),    // 1277 events
    (7, "corrupt", 96, "7ba6daed6341e842", "d52c807ece04007b"), // 2513 events
    (31, "idle", 96, "49f053ffb36497b5", "70d6c85ab9e13a30"),   // 1512 events
    (
        31,
        "explorer_legislator",
        96,
        "04c886a8549160ec",
        "41b7bae4211afc1c",
    ), // 3356 events
    (
        2026,
        "mixed+prison",
        96,
        "d0c8e9077dad147c",
        "cc4dec6f07d295e8",
    ), // 1989 events
    (
        2026,
        "idle+six_years",
        120,
        "076c5c942e64fcca",
        "ad4e3c73175d281a",
    ), // 1690 events
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

/// The same eight games with no arc content (see the note above `PINS`
/// for what such a game is since A2, and what moved these digests). Until
/// A2 this table held the two seed-31 games as pinned before arcs existed,
/// and the other six equalled `PINS`; now every game has an arc in it (the
/// budget round runs in every town), so all eight are pinned twice. The
/// test below plays the seed-31 games on every run (a flood strikes there,
/// so they show that an arc's cause without its content changes nothing);
/// `SPINE_PROOF_NO_ARCS=all` plays and checks all eight, `=print` prints
/// them.
const BEFORE_ARCS: [(u64, &str, u32, &str, &str); 8] = [
    (2026, "idle", 96, "544bc8b2812ca226", "05fad90e5de70038"), // 1334 events
    (2026, "mixed", 96, "9a2ed9ee238fdf39", "e9c8896d28b195d2"), // 2514 events
    (7, "idle", 96, "17d3d695b978af15", "1824c730616cb9ac"),    // 1183 events
    (7, "corrupt", 96, "d5e443d3f5926d0b", "3b6bc2104a9c6095"), // 2333 events
    (31, "idle", 96, "5f13f30a19e1292e", "ef2c65c04c943fbd"),   // 1393 events
    (
        31,
        "explorer_legislator",
        96,
        "f2ed62b19a6d5297",
        "61dd5e59891fac53",
    ), // 3202 events
    (
        2026,
        "mixed+prison",
        96,
        "3d0cff9372ec3a3b",
        "e3578b4c629c45bd",
    ), // 1851 events
    (
        2026,
        "idle+six_years",
        120,
        "94ee0717e5cd07ca",
        "43da10a836c6a939",
    ), // 1580 events
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
        let want = BEFORE_ARCS
            .iter()
            .find(|p| p.0 == seed && p.1 == label && p.2 == months)
            .unwrap_or_else(|| panic!("no no-arc pin for seed {seed} {label}"));
        if seed != 31 && mode.is_empty() {
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
        assert_eq!(
            (stream.as_str(), state.as_str()),
            (want.3, want.4),
            "seed {seed}, {label}: with no arc content the game changed"
        );
    }
}
