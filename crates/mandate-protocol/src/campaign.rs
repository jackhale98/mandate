//! Campaign views (M4): the election calendar, the player's filing, races in
//! the player's town with **polls only** (never the model truth, DEC-029), and
//! the latest results with their explanation (DEC-028).

use crate::views::DateView;
use mandate_sim::Engine;
use mandate_sim::politics::PoliticianId;
use mandate_sim::politics::elections::{self, Filing, Seat};
use mandate_sim::world::Tier;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PartyView {
    pub id: u32,
    pub name: String,
    pub short: String,
    /// Stable categorical colour slot (0-based).
    pub colour: u32,
    /// The two attitudes the platform leans on hardest, e.g. `pro-environment`.
    pub stances: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CandidateView {
    pub politician: u32,
    pub name: String,
    pub party: Option<u32>,
    pub is_player: bool,
    pub is_incumbent: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PollView {
    pub date: DateView,
    /// Day number (for charting).
    pub day: u32,
    pub pollster: String,
    /// Share per candidate (race order), among decided voters.
    pub shares: Vec<f64>,
    pub undecided: f64,
    pub sample: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RaceView {
    /// `mayor` | `ward`
    pub seat: String,
    pub ward_number: Option<u32>,
    pub is_player_race: bool,
    pub candidates: Vec<CandidateView>,
    pub polls: Vec<PollView>,
    /// Average of each pollster's latest poll.
    pub poll_average: Option<Vec<f64>>,
    /// The player's name recognition in this race (0..1), if running.
    pub player_awareness: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResultCandidateView {
    pub name: String,
    pub party: Option<u32>,
    pub votes: u32,
    pub share: f64,
    pub final_poll: Option<f64>,
    pub is_player: bool,
    pub is_winner: bool,
    pub was_incumbent: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FactorView {
    /// Fluent key suffix: `issues`, `party`, `valence`, `record`, `campaign`, `shock`.
    pub factor: String,
    /// Contribution to the winner's margin over the runner-up (share, −1..1).
    pub margin: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ResultView {
    pub seat: String,
    pub ward_number: Option<u32>,
    pub date: DateView,
    pub candidates: Vec<ResultCandidateView>,
    pub turnout: f64,
    pub decomposition: Vec<FactorView>,
    pub player_ran: bool,
    pub player_won: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CampaignView {
    pub election_day: Option<DateView>,
    pub campaign_opens: Option<DateView>,
    pub days_to_election: Option<u32>,
    pub in_campaign: bool,
    /// `council` | `mayor` | `retire` | `default`
    pub filing: String,
    /// Whether the player's party would nominate them for mayor (Standing).
    pub mayor_nomination_open: bool,
    pub player_in_office: bool,
    pub player_party: Option<u32>,
    pub parties: Vec<PartyView>,
    /// The player's race first, then the mayoralty, then other wards.
    pub races: Vec<RaceView>,
    /// The latest election's results in the player's town (player's race first).
    pub results: Vec<ResultView>,
    pub last_election: Option<DateView>,
}

fn name_of(engine: &Engine, id: PoliticianId) -> String {
    engine
        .state()
        .politics
        .get(id)
        .map(|p| p.full_name())
        .unwrap_or_default()
}

fn ward_number(engine: &Engine, seat: Seat) -> Option<u32> {
    let ward = seat.constituency()?;
    engine
        .state()
        .world
        .constituencies
        .get(ward.index())
        .filter(|c| c.tier == Tier::Local)
        .map(|c| c.number)
}

/// `ward` for a seat in a body; an office's own id (DTO unchanged until R2).
fn seat_key(seat: Seat) -> String {
    if seat.is_office() {
        seat.id().to_string()
    } else {
        "ward".into()
    }
}

pub fn campaign_view(engine: &Engine) -> Option<CampaignView> {
    let st = engine.state();
    let pl = st.player.as_ref()?;
    let me = st.politics.get(pl.politician)?;
    let town = mandate_sim::world::Jurisdiction::town(pl.home());
    let constitution = engine.content().constitutions.get(&st.meta.constitution);
    let executive = constitution.and_then(|c| mandate_sim::politics::executive_post(c, town));
    let el = &st.politics.elections;
    let parties = st
        .politics
        .parties
        .iter()
        .map(|p| {
            // Stances iterate the attitude registry (DEC-075).
            let keys = &engine.content().attitudes;
            let mut dims: Vec<usize> = (0..p.platform.len().min(keys.len())).collect();
            dims.sort_by(|a, b| p.platform[*b].abs().total_cmp(&p.platform[*a].abs()));
            PartyView {
                id: p.id.0,
                name: p.name.clone(),
                short: p.short.clone(),
                colour: p.colour as u32,
                stances: dims
                    .iter()
                    .take(2)
                    .map(|d| {
                        format!(
                            "{}-{}",
                            if p.platform[*d] >= 0.0 { "pro" } else { "anti" },
                            keys[*d].key
                        )
                    })
                    .collect(),
            }
        })
        .collect();
    let pollster = |i: u8| {
        engine
            .content()
            .politics
            .polls
            .pollsters
            .get(i as usize)
            .cloned()
            .unwrap_or_default()
    };
    let mut races: Vec<(u8, RaceView)> = el
        .races_in(town)
        .map(|(ri, r)| {
            let mine = r
                .candidates
                .iter()
                .position(|c| c.politician == pl.politician);
            let polls = el
                .polls
                .iter()
                .filter(|p| p.race as usize == ri)
                .map(|p| PollView {
                    date: DateView::of(p.date),
                    day: p.date.0,
                    pollster: pollster(p.pollster),
                    shares: p.shares.iter().map(|v| *v as f64).collect(),
                    undecided: p.undecided as f64,
                    sample: p.sample,
                })
                .collect();
            let order = if mine.is_some() {
                0
            } else if r.seat.is_office() {
                1
            } else {
                2
            };
            (
                order,
                RaceView {
                    seat: seat_key(r.seat),
                    ward_number: ward_number(engine, r.seat),
                    is_player_race: mine.is_some(),
                    candidates: r
                        .candidates
                        .iter()
                        .map(|c| CandidateView {
                            politician: c.politician.0,
                            name: name_of(engine, c.politician),
                            party: c.party.map(|p| p.0),
                            is_player: c.politician == pl.politician,
                            is_incumbent: r.incumbent == Some(c.politician),
                        })
                        .collect(),
                    polls,
                    poll_average: elections::poll_average(el, ri)
                        .map(|v| v.iter().map(|x| *x as f64).collect()),
                    player_awareness: mine.map(|i| {
                        let a = &r.candidates[i].awareness;
                        a.iter().map(|x| *x as f64).sum::<f64>() / a.len().max(1) as f64
                    }),
                },
            )
        })
        .collect();
    races.sort_by_key(|(o, r)| (*o, r.ward_number.unwrap_or(0)));
    let last = el.last_election;
    let mut results: Vec<(u8, ResultView)> = el
        .results
        .iter()
        .filter(|r| Some(r.date) == last && r.seat.at() == town)
        .map(|r| {
            let ran = r.candidates.iter().any(|c| c.politician == pl.politician);
            let order = if ran {
                0
            } else if r.seat.is_office() {
                1
            } else {
                2
            };
            (
                order,
                ResultView {
                    seat: seat_key(r.seat),
                    ward_number: ward_number(engine, r.seat),
                    date: DateView::of(r.date),
                    candidates: r
                        .candidates
                        .iter()
                        .map(|c| ResultCandidateView {
                            name: name_of(engine, c.politician),
                            party: c.party.map(|p| p.0),
                            votes: c.votes.min(u32::MAX as u64) as u32,
                            share: c.share as f64,
                            final_poll: c.final_poll.map(|v| v as f64),
                            is_player: c.politician == pl.politician,
                            is_winner: c.politician == r.winner,
                            was_incumbent: r.incumbent == Some(c.politician),
                        })
                        .collect(),
                    turnout: r.turnout as f64,
                    decomposition: r
                        .decomposition
                        .iter()
                        .map(|f| FactorView {
                            factor: f.factor.clone(),
                            margin: f.margin as f64,
                        })
                        .collect(),
                    player_ran: ran,
                    player_won: r.winner == pl.politician,
                },
            )
        })
        .collect();
    results.sort_by_key(|(o, r)| (*o, r.ward_number.unwrap_or(0)));
    let today = st.calendar.date;
    let (day, opens) = match (el.next_day, el.campaign_open) {
        (Some(d), Some(o)) => (Some(d), Some(o)),
        _ => {
            // Between cycles the schedule is computed on the next daily tick; derive it here.
            let next = constitution.and_then(|c| elections::next_cycle(c, today));
            match next {
                Some((d, weeks)) => (
                    Some(d),
                    Some(mandate_core::SimDate(d.0.saturating_sub(weeks as u32 * 7))),
                ),
                None => (None, None),
            }
        }
    };
    Some(CampaignView {
        election_day: day.map(DateView::of),
        campaign_opens: opens.map(DateView::of),
        days_to_election: day.map(|d| today.days_until(d)),
        in_campaign: !el.races.is_empty(),
        // The post's constitution id (DTO unchanged until R2).
        filing: match pl.filing {
            Some(Filing::Stand { post }) => post.id().to_string(),
            Some(Filing::Retire) => "retire".into(),
            None => "default".into(),
        },
        // The town's executive office: open to the player if they do not
        // hold it and pass its nomination gate (`career.nomination`).
        mayor_nomination_open: executive.is_some_and(|post| {
            me.role.post() != Some(post)
                && constitution.is_some_and(|c| {
                    elections::nomination_open(engine.content(), c, post, pl.tracks.standing as f64)
                })
        }),
        player_in_office: me.in_office(),
        player_party: me.party.map(|p| p.0),
        parties,
        races: races.into_iter().map(|(_, r)| r).collect(),
        results: results.into_iter().map(|(_, r)| r).collect(),
        last_election: last.map(DateView::of),
    })
}
