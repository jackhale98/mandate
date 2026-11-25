//! The skip rows of the bots report (DEC-102 clause 7): what a career
//! played only by skipping comes to, and what skipping the quiet weeks
//! costs. Measured and printed with the bars the decision intends beside
//! them as data; none is enforced, and `proxy_not_optimal` is untouched.
//!
//! - **Skip-only** (the `directed` probe: every turn a turn of a real
//!   skip): its first-election win rate beside the single-action
//!   routines', and its re-election rate (the election after the first,
//!   for those who won the first) beside the engaged newcomer's.
//! - **Attends the stops**: a player who skips the quiet weeks with the
//!   default directives and plays each stop a skip makes, beside the same
//!   player playing every turn. Twice: the best informed style of this
//!   report, and the engaged stand-in (`mandate_sim::standin`).

use super::*;
use mandate_sim::standin::{self, Career, Kind, Stepped};

/// The probe that only skips.
pub const SKIP_ONLY: &str = "directed";
/// The engaged newcomer of DEC-102 item 2: engaged, and new to the town
/// (a blind mix: it works every lever and has learned nothing first).
pub const ENGAGED_NEWCOMER: &str = "mixed";

/// The bars DEC-102 clause 7 intends, as data.
pub const SKIP_REELECTION_BAND: (f64, f64) = (0.40, 0.45);
pub const NEWCOMER_REELECTION_BAND: (f64, f64) = (0.60, 0.70);

/// Who plays a career measured here.
#[derive(Clone, Debug, PartialEq)]
pub enum Player {
    /// A bot strategy, every turn.
    Bot(String),
    /// A stand-in: `Engaged` plays every turn, `Attends` skips the quiet
    /// weeks and plays the stops as `Engaged`.
    StandIn(Kind),
    /// Skips the quiet weeks with the default directives and plays each
    /// stop (and the campaign) as this bot style; the style's learning
    /// turns are played first.
    Attends(String),
}

impl Player {
    pub fn name(&self) -> String {
        match self {
            Player::Bot(s) => s.clone(),
            Player::StandIn(k) => format!("standin:{}", k.key()),
            Player::Attends(s) => format!("attends:{s}"),
        }
    }
}

/// An election the player stood in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stood {
    pub won: bool,
    pub share: f64,
}

/// A career through its first elections.
#[derive(Clone, Debug, Default)]
pub struct Through {
    /// In order; fewer than asked for when the career ended, the player
    /// was not on a ballot, or time ran out.
    pub elections: Vec<Stood>,
    pub played: u32,
    pub skipped: u32,
    /// Each skip: the turns it ran and why it stopped.
    pub skips: Vec<(u32, String)>,
    /// Turns played and skipped, and skips made, up to the first election
    /// (the whole of it when there was none).
    pub to_first: Option<(u32, u32, usize)>,
}

/// Turns played, turns skipped, and the skips made (turns, why it
/// stopped).
type ToFirst<'a> = (u32, u32, &'a [(u32, String)]);

impl Through {
    /// Turns played, turns skipped and the skips made up to the first
    /// election.
    fn first(&self) -> ToFirst<'_> {
        match self.to_first {
            Some((p, s, n)) => (p, s, &self.skips[..n.min(self.skips.len())]),
            None => (self.played, self.skipped, &self.skips),
        }
    }
}

/// Turns a career is given per election (a four-year term of fortnights
/// and its campaign fit with room to spare).
const TURNS_PER_ELECTION: u32 = 160;

/// Play a seed until the player has stood in `elections` elections.
pub fn through(
    content: &Arc<ContentBundle>,
    seed: u64,
    player: &Player,
    elections: usize,
) -> Option<Through> {
    let mut e = new_game(content, seed)?;
    let me = e.state().player.as_ref()?.politician;
    let start = e.state().calendar.date;
    let start_turn = e.state().calendar.turn;
    let mut out = Through::default();
    let mut career = match player {
        Player::StandIn(k) => Some(Career::new(*k)),
        _ => None,
    };
    let mut attend = false;
    let stood = |e: &Engine| -> Vec<Stood> {
        let mut v: Vec<(mandate_core::SimDate, Stood)> = e
            .state()
            .politics
            .elections
            .results
            .iter()
            .filter(|r| r.date > start)
            .filter_map(|r| {
                let c = r.candidates.iter().find(|c| c.politician == me)?;
                Some((
                    r.date,
                    Stood {
                        won: r.winner == me,
                        share: c.share as f64,
                    },
                ))
            })
            .collect();
        v.sort_by_key(|x| x.0);
        v.into_iter().map(|x| x.1).collect()
    };
    let limit = start_turn + TURNS_PER_ELECTION * elections as u32;
    // Results are kept for a few elections a seat, so they are read as
    // they come.
    let mut seen = 0usize;
    while out.elections.len() < elections && e.state().calendar.turn < limit {
        let before = e.state().calendar.turn;
        let alive = match player {
            Player::Bot(s) => turn(&mut e, content, s, start_turn).is_ok(),
            Player::StandIn(_) => match career.as_mut() {
                Some(c) => standin::step(&mut e, c) != Stepped::Over,
                None => false,
            },
            Player::Attends(style) => {
                let learned = e.state().calendar.turn >= start_turn + EXPLORE_TURNS;
                let target = (learned && !attend)
                    .then(|| standin::quiet_target(e.state(), e.content()))
                    .flatten();
                let skipped = target.is_some_and(|target| {
                    apply(
                        &mut e,
                        Command::Skip {
                            target,
                            limit: None,
                        },
                    )
                    .is_ok()
                        && e.state().calendar.turn > before
                });
                if skipped {
                    let ran = e.state().calendar.turn - before;
                    let stop = e
                        .state()
                        .act
                        .last_skip
                        .as_ref()
                        .map(|s| standin::stop_key(&s.stop))
                        .unwrap_or_else(|| "open".into());
                    out.skipped += ran;
                    out.skips.push((ran, stop));
                    attend = true;
                    true
                } else {
                    attend = false;
                    let ok = turn(&mut e, content, style, start_turn).is_ok();
                    out.played += u32::from(ok);
                    ok
                }
            }
        };
        if matches!(player, Player::Bot(_)) && alive {
            out.played += 1;
        }
        if let Some(c) = &career {
            out.played = c.played;
            out.skipped = c.skipped;
            out.skips.clone_from(&c.skips);
        }
        let now = stood(&e);
        if now.len() > seen {
            out.elections.extend(now[seen..].iter().copied());
            seen = now.len();
            if out.to_first.is_none() {
                out.to_first = Some((out.played, out.skipped, out.skips.len()));
            }
        }
        if !alive || e.state().calendar.turn == before {
            break;
        }
    }
    out.elections.truncate(elections);
    Some(out)
}

pub(super) fn workers() -> usize {
    match crate::lab::runner::threads() {
        0 => std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .clamp(1, 3),
        n => n,
    }
}

fn rate(n: usize, of: usize) -> Option<f64> {
    (of > 0).then(|| n as f64 / of as f64)
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    Some(if n.is_multiple_of(2) {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    } else {
        v[n / 2]
    })
}

/// A player's first elections over the seeds.
fn first_row(runs: &[Through]) -> serde_json::Value {
    let first: Vec<&Stood> = runs.iter().filter_map(|r| r.elections.first()).collect();
    let n = runs.len();
    let wins = first.iter().filter(|s| s.won).count();
    // Up to the first election.
    let to_first: Vec<ToFirst<'_>> = runs.iter().map(|r| r.first()).collect();
    let skips: Vec<f64> = to_first
        .iter()
        .flat_map(|r| r.2.iter().map(|s| s.0 as f64))
        .collect();
    let mut stops: BTreeMap<String, u32> = BTreeMap::new();
    for r in &to_first {
        for (_, why) in r.2 {
            *stops.entry(why.clone()).or_default() += 1;
        }
    }
    let per = |f: &dyn Fn(&ToFirst<'_>) -> f64| {
        (n > 0).then(|| to_first.iter().map(f).sum::<f64>() / n as f64)
    };
    json!({
        "careers": n,
        "win_rate": rate(wins, n),
        // A career with no first election (ended, or off the ballot) counts as share 0.
        "mean_share": (n > 0).then(|| first.iter().map(|s| s.share).sum::<f64>() / n as f64),
        "turns_played": per(&|r| f64::from(r.0)),
        "turns_skipped": per(&|r| f64::from(r.1)),
        "skips": per(&|r| r.2.len() as f64),
        "median_skip_turns": median(skips),
        "stops": stops,
    })
}

/// A player's re-election: the second election, for those who won the
/// first.
fn second_row(runs: &[Through]) -> serde_json::Value {
    let n = runs.len();
    let won_first: Vec<&Through> = runs
        .iter()
        .filter(|r| r.elections.first().is_some_and(|s| s.won))
        .collect();
    let stood_again = won_first.iter().filter(|r| r.elections.len() > 1).count();
    let re_elected = won_first
        .iter()
        .filter(|r| r.elections.get(1).is_some_and(|s| s.won))
        .count();
    json!({
        "careers": n,
        "won_first": won_first.len(),
        "stood_again": stood_again,
        "re_elected": re_elected,
        // Of those who won the first election; one who did not stand again
        // (deselected, in prison, the career over) was not re-elected.
        "re_election_rate": rate(re_elected, won_first.len()),
        "held_through_two": rate(re_elected, n),
    })
}

/// The skip rows. `tournament`: the report's strategy rows (first
/// elections of every strategy, already played); `best_informed`: its best
/// informed style.
pub fn rows(
    content: &Arc<ContentBundle>,
    seeds: u64,
    tournament: &BTreeMap<String, serde_json::Value>,
    best_informed: &str,
) -> serde_json::Value {
    let of = |s: &str, k: &str| tournament.get(s).and_then(|v| v[k].as_f64());
    // (a) Skip-only against the single-action routines.
    let singles: Vec<&str> = STRATEGIES
        .iter()
        .copied()
        .filter(|s| class(s) == Class::SingleVerb)
        .collect();
    let best_single = singles
        .iter()
        .copied()
        .max_by(|a, b| {
            of(a, "win_rate")
                .unwrap_or(0.0)
                .total_cmp(&of(b, "win_rate").unwrap_or(0.0))
                .then(b.cmp(a))
        })
        .unwrap_or("canvass");
    let skip_rate = of(SKIP_ONLY, "win_rate");
    let single_rate = of(best_single, "win_rate");
    // The careers played here: two elections for the re-election rows,
    // one for the players that attend the stops.
    let jobs: Vec<(Player, usize)> = vec![
        (Player::Bot(SKIP_ONLY.into()), 2),
        (Player::Bot(ENGAGED_NEWCOMER.into()), 2),
        (Player::Bot(best_informed.into()), 2),
        (Player::Attends(best_informed.into()), 1),
        (Player::StandIn(Kind::Engaged), 1),
        (Player::StandIn(Kind::Attends), 1),
    ];
    let work: Vec<(usize, u64)> = (0..jobs.len())
        .flat_map(|j| (1..=seeds).map(move |s| (j, s)))
        .collect();
    let played = crate::lab::runner::par_map(&work, workers(), |(j, seed)| {
        through(content, *seed, &jobs[*j].0, jobs[*j].1)
    });
    let runs_of = |j: usize| -> Vec<Through> {
        work.iter()
            .zip(&played)
            .filter(|(w, _)| w.0 == j)
            .filter_map(|(_, r)| r.clone())
            .collect()
    };
    let (skip2, newcomer2, informed2) = (runs_of(0), runs_of(1), runs_of(2));
    let (attends, engaged, standin_attends) = (runs_of(3), runs_of(4), runs_of(5));
    let re = |runs: &[Through]| second_row(runs)["re_election_rate"].as_f64();
    let (skip_re, newcomer_re) = (re(&skip2), re(&newcomer2));
    let every = first_row(&informed2);
    let att = first_row(&attends);
    let (eng, satt) = (first_row(&engaged), first_row(&standin_attends));
    let gap = |a: &serde_json::Value, b: &serde_json::Value, k: &str| {
        Some(a[k].as_f64()? - b[k].as_f64()?)
    };
    json!({
        "enforced": false,
        "source": "DEC-102 clause 7; docs/development/TUNING_QUEUE.md",
        "careers_played_here": jobs.iter().map(|j| json!({ "player": j.0.name(), "elections": j.1 })).collect::<Vec<_>>(),
        "skip_only": SKIP_ONLY,
        "engaged_newcomer": ENGAGED_NEWCOMER,
        "best_informed": best_informed,
        "first_election": {
            "skip_only": { "win_rate": skip_rate, "mean_share": of(SKIP_ONLY, "mean_share") },
            "canvass_only": { "win_rate": of("canvass", "win_rate"), "mean_share": of("canvass", "mean_share") },
            "best_single_action": { "strategy": best_single, "win_rate": single_rate, "mean_share": of(best_single, "mean_share") },
            "bar": {
                "target": "skip-only first-election win rate no higher than the best single-action routine's",
                "skip_minus_best_single": skip_rate.zip(single_rate).map(|(a, b)| a - b),
                "skip_minus_canvass": skip_rate.zip(of("canvass", "win_rate")).map(|(a, b)| a - b),
                "pass": skip_rate.zip(single_rate).map(|(a, b)| a <= b),
            },
        },
        "re_election": {
            "skip_only": second_row(&skip2),
            "engaged_newcomer": second_row(&newcomer2),
            "best_informed": second_row(&informed2),
            "bar": {
                "target": "skip-only re-election clearly below the engaged newcomer's",
                "skip_only_band": [SKIP_REELECTION_BAND.0, SKIP_REELECTION_BAND.1],
                "engaged_newcomer_band": [NEWCOMER_REELECTION_BAND.0, NEWCOMER_REELECTION_BAND.1],
                "newcomer_minus_skip": newcomer_re.zip(skip_re).map(|(a, b)| a - b),
                "skip_in_band": skip_re.map(|x| (SKIP_REELECTION_BAND.0..=SKIP_REELECTION_BAND.1).contains(&x)),
                "newcomer_in_band": newcomer_re.map(|x| (NEWCOMER_REELECTION_BAND.0..=NEWCOMER_REELECTION_BAND.1).contains(&x)),
            },
        },
        "attends_the_stops": {
            "informed": {
                "style": best_informed,
                "every_turn": every,
                "attends": att,
                "win_rate_gap": gap(&every, &att, "win_rate"),
                "share_gap": gap(&every, &att, "mean_share"),
            },
            "stand_in": {
                "every_turn": eng,
                "attends": satt,
                "win_rate_gap": gap(&eng, &satt, "win_rate"),
                "share_gap": gap(&eng, &satt, "mean_share"),
            },
            "bar": {
                "target": "skipping the quiet weeks and playing each stop lands within a few points of playing every turn",
                "few_points": serde_json::Value::Null,
                "note": "DEC-102 clause 7 gives no number; the gaps above are every turn minus attends (share: 0.01 is one point)",
            },
        },
    })
}
