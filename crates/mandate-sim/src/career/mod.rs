//! The career card (M6 note 23): the figures a finished or abandoned run is
//! compared by. Facts only (DEC-054): the protocol's card view names them
//! with Fluent keys, and the client words them and computes nothing.
//!
//! - **Ratings** ([`RaceRating`], DEC-102 clause 6: a start's rating is a
//!   win rate measured by the stand-ins). A race is rated by playing it on
//!   forks of the state as it stands, one per luck, by the engaged
//!   stand-in (`standin::rate_standin`), and the rating names the two or
//!   three factors of the vote model's decomposition that drove it. The
//!   host (the browser session) rates each race as it opens with the
//!   player in it ([`races_open`]), and the opening as play began (its
//!   next election, from the world the run file rebuilds). A rating is a
//!   pure function of the state, the content and the number of plays, so
//!   the same run file gives the same ratings.
//! - **What became of each race** ([`races`]): the ratings joined with the
//!   Legacy record's elections and the race walked away from.
//! - **One comparable figure** ([`tally`]): wins less the wins the ratings
//!   expected, over the races that were rated and decided.
//! - **The highest post and the years served** ([`highest_post`],
//!   [`days_served`]), read from the Legacy record through the
//!   constitution (DEC-098: no office is named).
//! - **The world code** ([`code`]): seed, size, scenario, constitution,
//!   storyteller, starting knowledge and opening, and the build, so that
//!   someone else can play the same opening.

pub mod code;

use crate::legacy::{LegacyKind, act_of_post};
use crate::politics::Post;
use crate::standin::{self, Kind};
use crate::state::State;
use mandate_content::ContentBundle;
use mandate_core::SimDate;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A race as the stand-ins rated it (DEC-102 clause 6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaceRating {
    pub post: Post,
    /// The race's election day.
    pub election_day: SimDate,
    /// The day it was rated (the forks were taken from that day's world).
    pub rated_on: SimDate,
    pub plays: u32,
    pub wins: u32,
    /// Plays in which the player was on the ballot (the party may pass
    /// them over on a fork).
    pub on_ballot: u32,
    /// The factors that drove it, largest first: a factor's mean share of
    /// the margin between the player and the other of the first two, in
    /// thousandths (above zero it worked for the player).
    pub facts: Vec<RatingFact>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RatingFact {
    /// The vote model's factor (`issues`, `party`, `valence`, `record`,
    /// `campaign`, `shock`).
    pub factor: String,
    pub margin_pm: i32,
}

impl RaceRating {
    /// The win rate in thousandths.
    pub fn win_pm(&self) -> u32 {
        if self.plays == 0 {
            return 0;
        }
        (self.wins * 1000 + self.plays / 2) / self.plays
    }

    /// The same race (its post and its day).
    pub fn is(&self, post: Post, day: SimDate) -> bool {
        self.post == post && self.election_day == day
    }
}

/// The open races the player stands in: (the post, its election day).
pub fn races_open(state: &State) -> Vec<(Post, SimDate)> {
    let Some(me) = state.player.as_ref().map(|p| p.politician) else {
        return Vec::new();
    };
    state
        .politics
        .elections
        .races
        .iter()
        .filter(|r| r.candidates.iter().any(|c| c.politician == me))
        .map(|r| (r.seat, r.election_day))
        .collect()
}

/// The race under way the player stands in, if any (the first).
pub fn race_under_way(state: &State) -> Option<Post> {
    races_open(state).first().map(|r| r.0)
}

/// Rate a race (`None`: the player's next election, the opening's): the
/// content's `legacy.rating_plays` plays on forks of `state`, lucks
/// `0..plays`, by the engaged stand-in. `state` is only read.
pub fn rate_race(
    state: &State,
    content: &Arc<ContentBundle>,
    post: Option<Post>,
) -> Result<RaceRating, String> {
    let plays = u64::from(content.act_flow.legacy.rating_plays.max(1));
    let lucks: Vec<u64> = (0..plays).collect();
    let r = standin::rate_standin(state, content, post, &lucks, Kind::Engaged)?;
    let day = election_day(state, content, r.post).ok_or("the post is not elected")?;
    Ok(RaceRating {
        post: r.post,
        election_day: day,
        rated_on: state.calendar.date,
        plays: r.runs,
        wins: r.wins,
        on_ballot: r.on_ballot,
        facts: r
            .facts
            .iter()
            .map(|f| RatingFact {
                factor: f.factor.clone(),
                margin_pm: (f.margin * 1000.0).round() as i32,
            })
            .collect(),
    })
}

/// The election day `standin::rate` plays a race to: the open race's, else
/// the post's next.
fn election_day(state: &State, content: &ContentBundle, post: Post) -> Option<SimDate> {
    if let Some(r) = state
        .politics
        .elections
        .races
        .iter()
        .find(|r| r.seat == post)
    {
        return Some(r.election_day);
    }
    let c = content.constitutions.get(&state.meta.constitution)?;
    let (cal, term) = crate::politics::elections::calendar_for(c, post)?;
    Some(crate::politics::elections::next_election_day(
        cal,
        term,
        state.calendar.date,
    ))
}

/// What became of a race the player entered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RaceOutcome {
    Won,
    Lost,
    /// The player stepped away from politics while it ran.
    WalkedAway,
    /// Not decided when the run ended.
    Undecided,
}

impl RaceOutcome {
    pub fn key(self) -> &'static str {
        match self {
            RaceOutcome::Won => "won",
            RaceOutcome::Lost => "lost",
            RaceOutcome::WalkedAway => "walked_away",
            RaceOutcome::Undecided => "undecided",
        }
    }
}

/// A race the player entered: its rating at the time, if it was rated, and
/// what became of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RaceEntered {
    pub post: Post,
    pub election_day: SimDate,
    pub rating: Option<RaceRating>,
    pub outcome: RaceOutcome,
    /// The player's share of the vote, in thousandths, once decided.
    pub share_pm: Option<u32>,
}

/// Every race the player entered, by election day: those rated as they
/// opened, joined with the Legacy record's elections (an election with no
/// rating is listed without one) and the race walked away from.
pub fn races(state: &State, ratings: &[RaceRating]) -> Vec<RaceEntered> {
    let mut out: Vec<RaceEntered> = Vec::new();
    let mut walked: Option<Post> = None;
    for rec in &state.act.records {
        for e in &rec.entries {
            match &e.kind {
                LegacyKind::Election { post, won, share } => out.push(RaceEntered {
                    post: *post,
                    election_day: e.date,
                    rating: None,
                    outcome: if *won {
                        RaceOutcome::Won
                    } else {
                        RaceOutcome::Lost
                    },
                    share_pm: Some((f64::from(*share) * 1000.0).round().max(0.0) as u32),
                }),
                LegacyKind::SteppedAway { race } => walked = *race,
                _ => {}
            }
        }
    }
    let stepped = state.act.stepped_away;
    for r in ratings {
        if let Some(x) = out
            .iter_mut()
            .find(|x| x.rating.is_none() && x.post == r.post && x.election_day == r.election_day)
        {
            x.rating = Some(r.clone());
            continue;
        }
        // Not decided in the record: walked away from, or still to come.
        let outcome = match (walked, stepped) {
            (Some(p), Some(d)) if p == r.post && r.election_day >= d => RaceOutcome::WalkedAway,
            _ => RaceOutcome::Undecided,
        };
        out.push(RaceEntered {
            post: r.post,
            election_day: r.election_day,
            rating: Some(r.clone()),
            outcome,
            share_pm: None,
        });
    }
    out.sort_by_key(|x| x.election_day);
    out
}

/// Wins against the wins the ratings expected (M6 note 23's one comparable
/// figure), over the races rated and decided.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tally {
    /// Races rated and decided.
    pub rated: u32,
    /// Of those, won.
    pub wins: u32,
    /// The wins the ratings expected, in thousandths (the sum of their win
    /// rates).
    pub expected_pm: u32,
    /// Wins less expected wins, in thousandths.
    pub over_pm: i32,
}

pub fn tally(races: &[RaceEntered]) -> Tally {
    let mut t = Tally::default();
    for r in races {
        let Some(rating) = &r.rating else { continue };
        if !matches!(r.outcome, RaceOutcome::Won | RaceOutcome::Lost) {
            continue;
        }
        t.rated += 1;
        t.expected_pm += rating.win_pm();
        if r.outcome == RaceOutcome::Won {
            t.wins += 1;
        }
    }
    t.over_pm = (t.wins * 1000) as i32 - t.expected_pm as i32;
    t
}

/// Every post held in the run, in the order first held.
pub fn posts_held(state: &State) -> Vec<Post> {
    let mut out: Vec<Post> = Vec::new();
    for rec in &state.act.records {
        for p in rec.posts_held() {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

/// The highest post held in the run: the latest act of the career ladder
/// it belongs to, then an office over a seat, then the first held.
pub fn highest_post(state: &State, content: &ContentBundle) -> Option<Post> {
    let c = content.constitutions.get(&state.meta.constitution)?;
    let held = posts_held(state);
    let rank = |p: &Post| (act_of_post(c, *p).map_or(0, |i| i + 1), p.is_office());
    let best = held.iter().map(rank).max()?;
    held.into_iter().find(|p| rank(p) == best)
}

/// Days the player held a post (any), over every act's record, to the
/// record's close (or today).
pub fn days_served(state: &State) -> u32 {
    let today = state.calendar.date;
    let mut total = 0;
    for rec in &state.act.records {
        let end = rec.closed.unwrap_or(today);
        let mut holding: Vec<Post> = rec.start_posts.clone();
        let mut since = rec.began;
        for e in &rec.entries {
            let before = !holding.is_empty();
            match &e.kind {
                LegacyKind::PostTaken { post } if !holding.contains(post) => holding.push(*post),
                LegacyKind::PostLeft { post } => holding.retain(|p| p != post),
                _ => continue,
            }
            let after = !holding.is_empty();
            if before && !after {
                total += since.days_until(e.date.min(end));
            }
            if !before && after {
                since = e.date;
            }
        }
        if !holding.is_empty() {
            total += since.days_until(end);
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Jurisdiction;
    use mandate_content::PostKey;

    fn post(n: u32) -> Post {
        Post::Office {
            office: PostKey::new(&format!("office.{n}")).unwrap(),
            at: Jurisdiction::town(crate::world::MunicipalityId(0)),
        }
    }

    fn rating(p: Post, day: u32, wins: u32, plays: u32) -> RaceRating {
        RaceRating {
            post: p,
            election_day: SimDate(day),
            rated_on: SimDate(day - 50),
            plays,
            wins,
            on_ballot: plays,
            facts: Vec::new(),
        }
    }

    #[test]
    fn the_tally_is_wins_less_the_wins_expected() {
        let races = vec![
            RaceEntered {
                post: post(1),
                election_day: SimDate(100),
                rating: Some(rating(post(1), 100, 1, 4)),
                outcome: RaceOutcome::Won,
                share_pm: Some(520),
            },
            RaceEntered {
                post: post(2),
                election_day: SimDate(200),
                rating: Some(rating(post(2), 200, 3, 4)),
                outcome: RaceOutcome::Lost,
                share_pm: Some(410),
            },
            // Walked away from and unrated races count in neither.
            RaceEntered {
                post: post(3),
                election_day: SimDate(300),
                rating: Some(rating(post(3), 300, 4, 4)),
                outcome: RaceOutcome::WalkedAway,
                share_pm: None,
            },
            RaceEntered {
                post: post(4),
                election_day: SimDate(400),
                rating: None,
                outcome: RaceOutcome::Won,
                share_pm: Some(600),
            },
        ];
        let t = tally(&races);
        assert_eq!(t.rated, 2);
        assert_eq!(t.wins, 1);
        assert_eq!(t.expected_pm, 250 + 750);
        assert_eq!(t.over_pm, 0);
    }

    #[test]
    fn a_win_rate_rounds_to_thousandths() {
        assert_eq!(rating(post(1), 100, 1, 3).win_pm(), 333);
        assert_eq!(rating(post(1), 100, 2, 3).win_pm(), 667);
        assert_eq!(rating(post(1), 100, 0, 0).win_pm(), 0);
    }
}
