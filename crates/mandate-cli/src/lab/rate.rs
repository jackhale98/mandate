//! `mandate-cli lab rate-start` (DEC-102 clause 6): a start's rating.
//!
//! For a generated world and a race, the race is played several times on
//! forks under different luck (`mandate_sim::standin::rate`), once by the
//! engaged stand-in and once by an informed bot style, and the win rate is
//! printed with the two or three factors of the vote model's own
//! decomposition that drive it: facts and Fluent keys, never prose.
//!
//! The race is the opening (the player's next election for the post they
//! hold), or, with `--post`, the same character's race for another post
//! the constitution defines, as they stand at a later date: the game is
//! first played on by the engaged stand-in (`--after-months`, default:
//! until their first election is over), and nothing resets.
//!
//! A pure function of the seed, the content and the arguments: the same
//! inputs give the same rating. No start is labelled by hand (DEC-102).

use super::runner::{Driver, Game, Setup};
use mandate_content::ContentBundle;
use mandate_sim::standin::{self, Kind, Rating};
use serde_json::{Value, json};
use std::sync::Arc;

/// The informed style a start is rated with beside the stand-in. The
/// bots report names the best informed style of a run
/// (`skips.best_informed`); at 16 seeds it is this one. Re-set with the
/// start ratings at the tuning pass (`TUNING_QUEUE.md`, step 3).
pub const INFORMED: &str = "explorer_coalition";

pub struct RateArgs {
    pub seed: u64,
    /// A constitution post id (an office's or a body's); `None`: the
    /// opening.
    pub post: Option<String>,
    /// Plays per player (lucks 0..N; 0 is the game's own).
    pub plays: u32,
    /// Months the engaged stand-in plays before the race is rated.
    pub after_months: Option<u32>,
    pub style: String,
}

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

fn rating(r: &Rating) -> Value {
    json!({
        "runs": r.runs,
        "wins": r.wins,
        "on_ballot": r.on_ballot,
        "win_rate": r3(r.win_rate),
        "mean_share": r3(r.mean_share),
        "facts": r.facts.iter().map(|f| json!({ "factor": f.factor, "key": f.key, "margin": r3(f.margin) })).collect::<Vec<_>>(),
        "plays": r.plays.iter().map(|p| json!({ "luck": p.luck, "won": p.won, "share": r3(p.share), "on_ballot": p.on_ballot, "state_hash": p.state_hash })).collect::<Vec<_>>(),
    })
}

/// The two ratings pooled: every play of both players, and the factors
/// over all of them.
fn pooled(a: &Rating, b: &Rating) -> Value {
    let runs = a.runs + b.runs;
    let wins = a.wins + b.wins;
    let mut sums: std::collections::BTreeMap<&str, (f64, u32)> = Default::default();
    for p in a.plays.iter().chain(&b.plays) {
        for (f, m) in &p.factors {
            let e = sums.entry(f.as_str()).or_default();
            e.0 += m;
            e.1 += 1;
        }
    }
    let mut facts: Vec<(&str, f64)> = sums
        .into_iter()
        .map(|(f, (s, n))| (f, s / f64::from(n.max(1))))
        .collect();
    facts.sort_by(|x, y| y.1.abs().total_cmp(&x.1.abs()).then(x.0.cmp(y.0)));
    facts.truncate(standin::FACTS);
    json!({
        "runs": runs,
        "wins": wins,
        "win_rate": r3(f64::from(wins) / f64::from(runs.max(1))),
        "facts": facts.iter().map(|(f, m)| json!({ "factor": f, "key": format!("factor-{f}"), "margin": r3(*m) })).collect::<Vec<_>>(),
    })
}

pub fn rate_start(content: &Arc<ContentBundle>, a: &RateArgs) -> Result<Value, String> {
    let mut g = Game::new(
        content,
        &Setup::small(a.seed, None),
        Driver::standin(Kind::Engaged),
    )?;
    let start = g.e.state().calendar.date;
    // A later race: the same character, played on to the date.
    let mut months = 0;
    match (a.after_months, &a.post) {
        (Some(m), _) => {
            g.run_to(m, &[], &mut Vec::new());
            months = m;
        }
        (None, Some(_)) => {
            // Until their first election is over, and a month on.
            while g
                .e
                .state()
                .politics
                .elections
                .last_election
                .is_none_or(|d| d <= start)
                && months < 72
            {
                g.step_month();
                months += 1;
            }
            g.step_month();
            months += 1;
        }
        (None, None) => {}
    }
    let st = g.e.state();
    let post = match &a.post {
        Some(id) => Some(standin::post_named(st, content, id)?),
        None => None,
    };
    let held = mandate_sim::player::posts(st);
    let lucks: Vec<u64> = (0..u64::from(a.plays.max(1))).collect();
    let engaged = standin::rate_standin(st, content, post, &lucks, Kind::Engaged)?;
    let start_turn = g.start_turn;
    let style = a.style.clone();
    let informed = standin::rate(st, content, post, &lucks, &mut |e| {
        crate::bots::play_turn_from(e, content, &style, start_turn)
    })?;
    let rated = engaged.post;
    let pl = st.player.as_ref();
    let c = content.constitutions.get(&st.meta.constitution);
    Ok(json!({
        "seed": a.seed,
        "race": {
            "post": rated.id(),
            "role": c.and_then(|c| rated.word(c)),
            "at": rated.at(),
            "holds_it": held.contains(&rated),
            "holds": held.iter().map(|p| p.id().to_string()).collect::<Vec<_>>(),
            "rated_on": st.calendar.date.calendar(),
            "months_played_first": months,
            "standing": pl.map(|p| r3(f64::from(p.tracks.standing))),
            "mandate": pl.map(|p| r3(f64::from(p.mandate))),
        },
        "plays_per_player": lucks.len(),
        "rating": pooled(&engaged, &informed),
        "players": {
            "standin:engaged": rating(&engaged),
            style: rating(&informed),
        },
    }))
}
