//! Stand-ins play exchange scenes (task X3; DEC-112, EXCHANGE_SCENES
//! §8.2; DEC-102 clauses 6 and 7): the debate on `arc.rival`, played by
//! `mandate_sim::standin` through the scene's own commands.
//!
//! - a stand-in begins, says a line each round and the scene ends: every
//!   line it says is one the view listed, at the chance the view showed,
//!   and the change applied is inside what the view showed (shown =
//!   rolled);
//! - it reads the view the player has (DEC-091), and draws nothing of its
//!   own: the same commands sent by hand leave the same world;
//! - the styles part: absent never begins, cautious and bold say
//!   different lines;
//! - the engaged stand-in begins on the last turn it can; the one that
//!   attends the stops plays the scene where its skip stopped. (A skip
//!   with no stand-in still leaves the chair empty: `exchange.rs`,
//!   `a_skip_stops_at_a_debate_and_the_stand_in_never_plays_it`.)
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::exchange::Outcome;
use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::actions::PlayerCommand;
use mandate_sim::arcs::exchange::{self, ExchangeAct, Phase, SceneView};
use mandate_sim::events::EventKind;
use mandate_sim::player::knowledge::Subject;
use mandate_sim::standin::{self, Career, Kind, Play, Seat, Turn};
use mandate_sim::{Command, Engine, State};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

const SEEDS: [u64; 4] = [1, 3, 31, 99];
const ARC: &str = "arc.rival";

fn content() -> Arc<ContentBundle> {
    static C: OnceLock<Arc<ContentBundle>> = OnceLock::new();
    C.get_or_init(|| {
        let src =
            read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
        Arc::new(load(&src).unwrap().0)
    })
    .clone()
}

fn game(seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), content()).unwrap()
}

fn open(e: &Engine) -> Option<(u32, String)> {
    // The rival's debate only: other arcs have scenes of their own.
    exchange::view::open_scenes(e.state(), e.content())
        .into_iter()
        .find(|(run, _, _)| e.state().arcs.run(*run).is_some_and(|r| r.template == ARC))
        .map(|(run, beat, _)| (run.0, beat.key()))
}

/// Idle turns until the debate is on offer (kept by seed).
fn at_debate(seed: u64) -> Engine {
    static CACHE: OnceLock<Mutex<BTreeMap<u64, State>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    let kept = cache.lock().unwrap().get(&seed).cloned();
    let st = match kept {
        Some(st) => st,
        None => {
            let mut e = game(seed);
            for _ in 0..60 {
                if open(&e).is_some() {
                    break;
                }
                e.apply(Command::EndTurn).unwrap();
            }
            cache.lock().unwrap().insert(seed, e.state().clone());
            e.state().clone()
        }
    };
    let e = Engine::from_state(st, content()).unwrap();
    assert!(
        open(&e).is_some(),
        "seed {seed}: no debate in the first campaign"
    );
    e
}

fn seat(e: &Engine) -> Seat {
    Seat::of_player(e.state(), e.content()).unwrap()
}

/// The rival's run, live or past: its last scene's outcome.
fn outcome(e: &Engine) -> Option<Outcome> {
    let st = e.state();
    st.events.iter().rev().find_map(|ev| match &ev.kind {
        EventKind::ArcExchangeEnded {
            template, outcome, ..
        } if template == ARC => Outcome::from_key(outcome),
        _ => None,
    })
}

/// The lines said in the rival's scenes, in order.
fn lines(e: &Engine) -> Vec<String> {
    e.state()
        .events
        .iter()
        .filter_map(|ev| match &ev.kind {
            EventKind::ArcExchange {
                template, option, ..
            } if template == ARC => Some(option.clone()),
            _ => None,
        })
        .collect()
}

/// Forget what the player knows of worries and people: the ledger of a
/// player who prepared nothing.
fn blank(e: &mut Engine) {
    e.setup(|st| {
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        k.entries.retain(|s, _| {
            !matches!(
                s,
                Subject::HoodConcerns { .. }
                    | Subject::TownConcerns { .. }
                    | Subject::Person { .. }
                    | Subject::Secret { .. }
            )
        });
    });
}

/// What one stand-in turn sent and saw: the views it chose from.
struct Played {
    said: Vec<(SceneView, String)>,
    began: bool,
    commands: Vec<PlayerCommand>,
}

/// One stand-in turn, command by command, checking each line it says
/// against the view it was shown and the world it lands in.
fn play(e: &mut Engine, kind: Kind, play: Play) -> Played {
    let mut t = Turn::styled(kind, play, seat(e), e.content());
    let mut out = Played {
        said: Vec::new(),
        began: false,
        commands: Vec::new(),
    };
    while let Some(cmd) = t.next(e.state(), e.content()) {
        let seen = match &cmd {
            PlayerCommand::Exchange {
                run,
                beat,
                act: ExchangeAct::Say { option },
            } => {
                let v = exchange::view(e.state(), e.content(), *run, beat).unwrap();
                let known = exchange::view_known(e.state(), e.content(), *run, beat).unwrap();
                let o = v
                    .options
                    .iter()
                    .find(|o| &o.id == option)
                    .unwrap_or_else(|| panic!("{option} is not on the list it was shown"));
                let k = known.options.iter().find(|o| &o.id == option).unwrap();
                // The chance shown is the chance rolled: always exact.
                assert_eq!(o.chance.to_bits(), k.chance.to_bits(), "{option}");
                assert_eq!(standin::line(&v, play).as_deref(), Some(option.as_str()));
                Some((v.clone(), option.clone(), *run, beat.clone(), v.round))
            }
            PlayerCommand::Go { then: Some(b), .. }
                if matches!(
                    **b,
                    PlayerCommand::Exchange {
                        act: ExchangeAct::Begin,
                        ..
                    }
                ) =>
            {
                out.began = true;
                None
            }
            _ => None,
        };
        out.commands.push(cmd.clone());
        let r = e.apply(Command::Player(cmd));
        let created = r
            .as_ref()
            .ok()
            .and_then(|o| o.result.as_ref())
            .and_then(|x| x.created);
        if let Some((v, option, run, _, round)) = seen {
            assert!(r.is_ok(), "{option}: {r:?}");
            // The change applied is inside what was shown, group by group.
            let st = e.state();
            let r = st.arcs.runs.iter().find(|x| x.id.0 == run).unwrap();
            let rounds = r
                .exchange
                .as_ref()
                .map(|x| &x.rounds)
                .or(r.played.last().map(|p| &p.rounds))
                .unwrap();
            let played = &rounds[usize::from(round) - 1];
            assert_eq!(played.option, option);
            let o = v.options.iter().find(|o| o.id == option).unwrap();
            for (g, d) in played.deltas.iter().enumerate() {
                let span = if played.stood {
                    o.stands[g]
                } else {
                    o.fails[g]
                };
                assert!(
                    f64::from(*d) >= span.lo - 1e-6 && f64::from(*d) <= span.hi + 1e-6,
                    "{option} group {g}: {d} outside {span:?}"
                );
            }
            out.said.push((v, option));
        }
        t.result(r.is_ok(), created);
    }
    out
}

fn phase(e: &Engine, run: u32, beat: &str) -> Phase {
    exchange::view(e.state(), e.content(), run, beat)
        .unwrap()
        .phase
}

#[test]
fn a_stand_in_plays_the_debate_by_its_commands_at_the_odds_it_was_shown() {
    for seed in SEEDS {
        for style in [Play::Cautious, Play::Bold] {
            let mut e = at_debate(seed);
            let (run, beat) = open(&e).unwrap();
            let p = play(&mut e, Kind::Attends, style);
            assert!(p.began, "seed {seed} {}: it turns up", style.key());
            let rounds = exchange::view(e.state(), e.content(), run, &beat)
                .unwrap()
                .rounds;
            assert_eq!(phase(&e, run, &beat), Phase::Done, "seed {seed}");
            assert_eq!(
                p.said.len(),
                usize::from(rounds),
                "seed {seed} {}: a line every round",
                style.key()
            );
            assert_ne!(outcome(&e), Some(Outcome::Absent));
            assert!(outcome(&e).is_some());
            assert!(lines(&e).iter().all(|l| !l.is_empty()));
        }
    }
}

#[test]
fn the_styles_part_and_absent_leaves_the_chair_empty() {
    let mut differ = 0;
    for seed in SEEDS {
        let said = |style: Play| {
            let mut e = at_debate(seed);
            play(&mut e, Kind::Attends, style);
            lines(&e)
        };
        let (cautious, bold) = (said(Play::Cautious), said(Play::Bold));
        differ += usize::from(cautious != bold);
        // Absent: turn after turn, never a command of the scene's; the
        // other side speaks to an empty chair.
        let mut e = at_debate(seed);
        for _ in 0..8 {
            let p = play(&mut e, Kind::Engaged, Play::Absent);
            assert!(!p.began && p.said.is_empty());
            assert!(
                p.commands
                    .iter()
                    .all(|c| !matches!(c, PlayerCommand::Exchange { .. })),
                "seed {seed}"
            );
            e.apply(Command::EndTurn).unwrap();
            if outcome(&e).is_some() {
                break;
            }
        }
        assert_eq!(outcome(&e), Some(Outcome::Absent), "seed {seed}");
    }
    assert!(differ > 0, "cautious and bold said the same in every seed");
}

#[test]
fn a_stand_in_reads_the_view_the_player_has_and_draws_nothing() {
    let mut redacted = 0;
    for seed in SEEDS {
        let mut e = at_debate(seed);
        blank(&mut e);
        let start = Engine::from_state(e.state().clone(), content()).unwrap();
        let p = play(&mut e, Kind::Attends, Play::Cautious);
        // Each line was chosen from the player's view (`play` asserts it),
        // and that view hid something the true one shows.
        redacted += p
            .said
            .iter()
            .filter(|(v, _)| {
                v.groups.iter().any(|g| g.lean.is_none())
                    || v.options
                        .iter()
                        .any(|o| o.stands.iter().any(|s| !s.exact()))
            })
            .count();
        // The same commands sent by hand: the same world, to the hash. The
        // stand-in rolls nothing; the scene's draws are keyed on the run,
        // the beat, the round and the line.
        let mut by_hand = start;
        for c in p.commands {
            let _ = by_hand.apply(Command::Player(c));
        }
        assert_eq!(by_hand.state_hash(), e.state_hash(), "seed {seed}");
    }
    assert!(redacted > 0, "no view hid anything: the test has no teeth");
}

#[test]
fn the_engaged_stand_in_begins_on_the_last_turn_it_can() {
    for seed in SEEDS {
        let mut e = at_debate(seed);
        let (run, beat) = open(&e).unwrap();
        let last =
            exchange::last_day(e.state().arcs.run(mandate_sim::arcs::ArcId(run)).unwrap()).unwrap();
        let mut began_on = None;
        for _ in 0..8 {
            let today = e.state().calendar.date;
            let days = e.turn_days();
            let p = play(&mut e, Kind::Engaged, Play::Cautious);
            if p.began {
                began_on = Some((today, days));
                break;
            }
            e.apply(Command::EndTurn).unwrap();
        }
        let (on, days) = began_on.unwrap_or_else(|| panic!("seed {seed}: it never began"));
        assert!(on <= last, "seed {seed}");
        assert!(
            on.add_days(days) > last,
            "seed {seed}: begun on {on:?} with a turn to spare before {last:?}"
        );
        assert_eq!(phase(&e, run, &beat), Phase::Done);
        assert_ne!(outcome(&e), Some(Outcome::Absent));
    }
}

#[test]
fn the_stand_in_that_attends_the_stops_plays_the_scene_where_it_stopped() {
    let mut met = 0;
    for seed in SEEDS {
        let mut e = game(seed);
        let mut c = Career::new(Kind::Attends);
        for _ in 0..80 {
            if outcome(&e).is_some() {
                break;
            }
            if standin::step(&mut e, &mut c) == standin::Stepped::Over {
                break;
            }
        }
        let Some(o) = outcome(&e) else {
            continue;
        };
        met += 1;
        assert_ne!(
            o,
            Outcome::Absent,
            "seed {seed}: the attending stand-in left the chair empty (skips {:?})",
            c.skips
        );
        assert!(c.skipped > 0, "seed {seed}: it skipped the quiet weeks");
    }
    assert!(
        met >= 2,
        "the debate came in {met} of {} seeds",
        SEEDS.len()
    );
}
