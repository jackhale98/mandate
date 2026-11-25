//! Campaign views (M4): the election calendar, the player's filing, races in
//! the player's town with **polls only** (never the model truth, DEC-029), and
//! the latest results with their explanation (DEC-028).

use crate::post::{PostDto, post_dto};
use crate::views::DateView;
use mandate_sim::Engine;
use mandate_sim::politics::elections::{self, Filing};
use mandate_sim::politics::{INSTANTIATED_TIERS, PoliticianId, Post};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    /// The power broker puts money and their organisation behind them
    /// (M6 note 27): shown once the player knows who the broker is.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backed_by_broker: Option<bool>,
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
    /// The post contested (its label is the constitution's).
    pub post: PostDto,
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
    pub post: PostDto,
    pub date: DateView,
    pub candidates: Vec<ResultCandidateView>,
    pub turnout: f64,
    pub decomposition: Vec<FactorView>,
    pub player_ran: bool,
    pub player_won: bool,
    /// The event that records the race's result, to ask "Why?" about
    /// (`QueryDto::Why`); none where the log has none for this race.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CampaignView {
    pub election_day: Option<DateView>,
    pub campaign_opens: Option<DateView>,
    pub days_to_election: Option<u32>,
    pub in_campaign: bool,
    /// What the player has filed for; `None`: stand again for the post they
    /// hold (out of office, nothing).
    pub filing: Option<FilingView>,
    /// The posts the player may file for (elected posts whose constituency
    /// holds their home, in the tiers that hold elections), seats before
    /// offices, and whether their party would nominate them.
    pub nominations: Vec<NominationView>,
    pub player_in_office: bool,
    pub player_party: Option<u32>,
    pub parties: Vec<PartyView>,
    /// The player's race first, then offices, then seats by district.
    pub races: Vec<RaceView>,
    /// The latest election's results in the player's town (player's race first).
    pub results: Vec<ResultView>,
    pub last_election: Option<DateView>,
    /// What became of the post the player held going into the latest
    /// election (election night shows it whether or not they stood).
    pub seat_outcome: Option<SeatOutcomeView>,
    /// Why the player is not on the ballot they filed for this campaign
    /// (`EventKind::NominationRefused`): a Fluent key with its arguments.
    pub refusal: Option<RefusalView>,
}

/// What became of the player's post at the latest election.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SeatOutcomeView {
    pub post: PostDto,
    /// `ui-seat-held` (they stood and won it), `ui-seat-lost` (they stood
    /// and lost it), `ui-seat-passed` (they did not stand for it; `$winner`
    /// holds it now), `ui-seat-not-contested` (it was not up).
    pub key: String,
    /// Who won it, when it was contested.
    pub winner: Option<String>,
}

/// A refused nomination as text: `key` with `args` (`*_key` arguments are
/// Fluent keys, DEC-054).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RefusalView {
    pub key: String,
    pub args: BTreeMap<String, String>,
}

/// The player's filing (DEC-098).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FilingView {
    Stand { post: PostDto },
    Retire,
}

/// A post the player may stand for at the next election.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NominationView {
    pub post: PostDto,
    /// The player holds it now (standing again defends it).
    pub held: bool,
    /// Whether the party would nominate the player for it (its nomination
    /// gate, `career.nomination`; for a post held, renomination).
    pub open: bool,
    /// Why not, when closed: `ui-nomination-standing` (with `$post` and
    /// `$min`) or `ui-nomination-renomination` (with `$min`).
    pub reason_key: Option<String>,
    /// The Standing the party asks, when closed.
    pub min_standing: Option<f64>,
    /// What the player stands for now: this post if filed for, or the post
    /// held when nothing is filed.
    pub chosen: bool,
    /// Who holds the post now, when it is not the player.
    pub holder: Option<String>,
    pub holder_party: Option<u32>,
    /// The holder is of the player's party: the party backs them if they
    /// stand again.
    pub holder_same_party: bool,
    /// Whether the holder stands again, in words (the engine's draw is not
    /// shown): `ui-holder-usually-stands`, `ui-holder-term-limited`,
    /// `ui-holder-retiring`.
    pub holder_outlook_key: Option<String>,
    /// The Standing the party asks before it nominates the player for this
    /// post (`career.nomination`), if it asks any; shown whether met or not.
    pub gate_standing: Option<f64>,
    /// The player's Standing.
    pub standing: f64,
    /// What filing for it most likely means, when the party will most
    /// likely back its sitting holder instead: `ui-nomination-warn-seat`
    /// (the player will stand for their seat) or `ui-nomination-warn-none`
    /// (they will be on no ballot).
    pub warning_key: Option<String>,
}

/// The posts the player may file for (DEC-098 §1; audit §7.4): the elected
/// posts of the tiers that hold elections, for the jurisdictions holding the
/// player's home, whose constituency holds the player's district; in ladder
/// order (seats before offices within a tier). In M6: their seat in the
/// town's legislature and the town's executive office.
pub fn fileable_posts(engine: &Engine) -> Vec<Post> {
    let st = engine.state();
    let (Some(pl), Some(c)) = (st.player.as_ref(), crate::post::constitution(engine)) else {
        return Vec::new();
    };
    let Some(home) = crate::post::home_at(st) else {
        return Vec::new();
    };
    // The player's district's first neighbourhood: the constituency of
    // each districted body that holds it is theirs.
    let hood = st
        .world
        .constituencies
        .get(pl.ward.index())
        .and_then(|w| w.neighborhoods.first().copied());
    let mut chain = Vec::new();
    let mut j = Some(home);
    while let Some(x) = j {
        chain.push(x);
        j = x.parent(&st.world);
    }
    let mut out = Vec::new();
    for (tier, d) in c.ladder() {
        if !INSTANTIATED_TIERS.contains(&tier) || !d.elected() {
            continue;
        }
        let Some(at) = chain.iter().copied().find(|a| a.tier() == tier) else {
            continue;
        };
        let Some(key) = mandate_content::PostKey::new(d.id()) else {
            continue;
        };
        let post = match d {
            mandate_content::PostRef::Office(_) => Post::Office { office: key, at },
            mandate_content::PostRef::Body(_) => {
                let constituency = match d.constituency() {
                    Some(mandate_content::Constituency::Ward) => {
                        let Some(h) = hood else { continue };
                        let Some(w) = at
                            .constituencies(&st.world)
                            .find(|w| w.neighborhoods.contains(&h))
                        else {
                            continue;
                        };
                        Some(w.id)
                    }
                    _ => None,
                };
                Post::Seat {
                    body: key,
                    at,
                    constituency,
                }
            }
        };
        out.push(post);
    }
    out
}

/// The post a `File` command names, if the player may file for it.
pub fn fileable(engine: &Engine, dto: &PostDto) -> Option<Post> {
    let post = crate::post::post_of(engine, dto)?;
    fileable_posts(engine).into_iter().find(|p| *p == post)
}

fn nominations(engine: &Engine) -> Vec<NominationView> {
    let st = engine.state();
    let content = engine.content();
    let (Some(pl), Some(c)) = (st.player.as_ref(), crate::post::constitution(engine)) else {
        return Vec::new();
    };
    let held = mandate_sim::player::posts(st);
    let standing = pl.tracks.standing as f64;
    let renominate = content.politics.vote.renomination_standing;
    let year = st.calendar.date.calendar().year;
    let my_party = st.politics.get(pl.politician).and_then(|p| p.party);
    // Would the party stand the player for a post they hold, if passed over?
    let keeps_seat = !held.is_empty() && standing >= renominate;
    fileable_posts(engine)
        .into_iter()
        .map(|post| {
            let is_held = held.contains(&post);
            let gate = elections::nomination_min(content, c, post);
            // The sitting holder (not the player), and whether they stand again.
            let holder = sitting_holder(engine, post).filter(|id| *id != pl.politician);
            let hp = holder.and_then(|id| st.politics.get(id));
            let outlook = holder.map(|id| match post {
                Post::Office { .. } => elections::outlook(
                    &st.politics,
                    id,
                    year,
                    post.def(c).and_then(|d| d.term_limit()),
                    elections::OFFICE_RETIRE_AGE,
                ),
                Post::Seat { .. } => {
                    elections::outlook(&st.politics, id, year, None, elections::SEAT_RETIRE_AGE)
                }
            });
            let same_party = hp.is_some_and(|h| h.party.is_some() && h.party == my_party);
            let warning_key = (!is_held
                && same_party
                && outlook == Some(elections::Outlook::Usually))
            .then(|| {
                if keeps_seat {
                    "ui-nomination-warn-seat"
                } else {
                    "ui-nomination-warn-none"
                }
                .to_string()
            });
            let (open, reason_key, min) = if is_held {
                let open = standing >= renominate;
                (
                    open,
                    (!open).then(|| "ui-nomination-renomination".to_string()),
                    (!open).then_some(renominate),
                )
            } else {
                let open = elections::nomination_open(content, c, post, standing);
                (
                    open,
                    (!open).then(|| "ui-nomination-standing".to_string()),
                    if open { None } else { gate },
                )
            };
            let chosen = match pl.filing {
                Some(Filing::Stand { post: p }) => p == post,
                Some(Filing::Retire) => false,
                None => is_held,
            };
            NominationView {
                post: post_dto(engine, post),
                held: is_held,
                open,
                reason_key,
                min_standing: min,
                chosen,
                holder: hp.map(|h| h.full_name()),
                holder_party: hp.and_then(|h| h.party).map(|p| p.0),
                holder_same_party: same_party,
                holder_outlook_key: outlook.map(|o| {
                    match o {
                        elections::Outlook::Usually => "ui-holder-usually-stands",
                        elections::Outlook::TermLimited => "ui-holder-term-limited",
                        elections::Outlook::Retiring => "ui-holder-retiring",
                    }
                    .to_string()
                }),
                gate_standing: if is_held { None } else { gate },
                standing,
                warning_key,
            }
        })
        .collect()
}

/// Who sits in a post now: an office's holder, or a seat's member.
fn sitting_holder(engine: &Engine, post: Post) -> Option<PoliticianId> {
    let st = engine.state();
    st.politics
        .politicians
        .iter()
        .find(|p| p.role.post() == Some(post))
        .map(|p| p.id)
}

/// The text of a refused nomination (play note 20): `nomination-refused`
/// ("{ $why } { $then }") with the reason's line (`nomination-why-<reason>`)
/// and what the player stands for now (`nomination-then-*`).
#[allow(clippy::too_many_arguments)]
pub fn refusal_args(
    engine: &Engine,
    post: Post,
    reason: &str,
    nominee: Option<u32>,
    standing: u32,
    min_standing: Option<u32>,
    instead: Option<Post>,
    args: &mut BTreeMap<String, String>,
) -> &'static str {
    let mut put = |k: &str, v: String| {
        args.insert(k.to_string(), v);
    };
    put("post_key", post_dto(engine, post).label_key);
    if let Some(n) = nominee {
        put("holder", name_of(engine, PoliticianId(n)));
    }
    put("standing", standing.to_string());
    put("min", min_standing.unwrap_or(0).to_string());
    let why = match reason {
        "holder" | "standing" | "party" => reason,
        _ => "other",
    };
    put("why_key", format!("nomination-why-{why}"));
    let then = match instead.map(|p| post_dto(engine, p)) {
        Some(d) => match d.constituency_number {
            Some(n) => {
                put("district_key", "ui-post-seat".into());
                put("tier", d.at.tier.clone());
                put("n", n.to_string());
                "nomination-then-seat"
            }
            None => {
                put("instead_key", d.label_key);
                "nomination-then-post"
            }
        },
        None => "nomination-then-none",
    };
    put("then_key", then.into());
    "nomination-refused"
}

/// This campaign's refused nomination, if the player's party refused them.
pub fn current_refusal(engine: &Engine) -> Option<RefusalView> {
    let st = engine.state();
    let opened = st.politics.elections.races.first()?.opened;
    st.events
        .iter()
        .rev()
        .take_while(|e| e.date >= opened)
        .find_map(|e| match &e.kind {
            mandate_sim::events::EventKind::NominationRefused {
                post,
                reason,
                nominee,
                standing,
                min_standing,
                instead,
            } => {
                let mut args = BTreeMap::new();
                let key = refusal_args(
                    engine,
                    *post,
                    reason,
                    *nominee,
                    *standing,
                    *min_standing,
                    *instead,
                    &mut args,
                );
                Some(RefusalView {
                    key: key.into(),
                    args,
                })
            }
            _ => None,
        })
}

/// What became of the post the player held going into the latest election:
/// the race whose incumbent they were, else a post they hold that was not up.
fn seat_outcome(engine: &Engine) -> Option<SeatOutcomeView> {
    let st = engine.state();
    let pl = st.player.as_ref()?;
    let el = &st.politics.elections;
    let last = el.last_election?;
    let at_last = || el.results.iter().filter(move |r| r.date == last);
    if let Some(r) = at_last().find(|r| r.incumbent == Some(pl.politician)) {
        let ran = r.candidates.iter().any(|c| c.politician == pl.politician);
        let key = match (ran, r.winner == pl.politician) {
            (true, true) => "ui-seat-held",
            (true, false) => "ui-seat-lost",
            _ => "ui-seat-passed",
        };
        return Some(SeatOutcomeView {
            post: post_dto(engine, r.seat),
            key: key.into(),
            winner: Some(name_of(engine, r.winner)),
        });
    }
    let held = mandate_sim::player::posts(st);
    held.into_iter()
        .find(|p| !at_last().any(|r| r.seat == *p))
        .map(|p| SeatOutcomeView {
            post: post_dto(engine, p),
            key: "ui-seat-not-contested".into(),
            winner: None,
        })
}

fn name_of(engine: &Engine, id: PoliticianId) -> String {
    engine
        .state()
        .politics
        .get(id)
        .map(|p| p.full_name())
        .unwrap_or_default()
}

pub fn campaign_view(engine: &Engine) -> Option<CampaignView> {
    let st = engine.state();
    let pl = st.player.as_ref()?;
    let me = st.politics.get(pl.politician)?;
    let town = crate::post::home_at(st)?;
    let constitution = crate::post::constitution(engine);
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
    let known_broker = mandate_sim::player::knowledge::Knowing::of(engine).broker();
    let mut races: Vec<(u8, RaceView)> = el
        .races_in(town)
        .map(|(ri, r)| {
            let mine = r
                .candidates
                .iter()
                .position(|c| c.politician == pl.politician);
            let st = engine.state();
            let backing =
                elections::backed(&st.politics, &st.world, &st.population, engine.content(), r);
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
                    post: post_dto(engine, r.seat),
                    is_player_race: mine.is_some(),
                    candidates: r
                        .candidates
                        .iter()
                        .enumerate()
                        .map(|(ci, c)| CandidateView {
                            politician: c.politician.0,
                            name: name_of(engine, c.politician),
                            party: c.party.map(|p| p.0),
                            is_player: c.politician == pl.politician,
                            is_incumbent: r.incumbent == Some(c.politician),
                            backed_by_broker: backing
                                .is_some_and(|(b, i)| i == ci && known_broker == Some(b))
                                .then_some(true),
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
    races.sort_by_key(|(o, r)| (*o, r.post.constituency_number.unwrap_or(0)));
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
                    post: post_dto(engine, r.seat),
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
                    event: crate::why::event_since(&st.events, r.date, |e| {
                        e.date == r.date
                            && matches!(
                                &e.kind,
                                mandate_sim::events::EventKind::RaceDecided { post, winner, .. }
                                    if *post == Some(r.seat) && *winner == r.winner.0
                            )
                    }),
                },
            )
        })
        .collect();
    results.sort_by_key(|(o, r)| (*o, r.post.constituency_number.unwrap_or(0)));
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
        filing: match pl.filing {
            Some(Filing::Stand { post }) => Some(FilingView::Stand {
                post: post_dto(engine, post),
            }),
            Some(Filing::Retire) => Some(FilingView::Retire),
            None => None,
        },
        nominations: nominations(engine),
        player_in_office: me.in_office(),
        player_party: me.party.map(|p| p.0),
        parties,
        races: races.into_iter().map(|(_, r)| r).collect(),
        results: results.into_iter().map(|(_, r)| r).collect(),
        last_election: last.map(DateView::of),
        seat_outcome: seat_outcome(engine),
        refusal: current_refusal(engine),
    })
}
