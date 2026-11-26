//! The player (DEC-011, DEC-013): day-part slots, Mandate, resources, and the
//! pressure tracks (DEC-006). Player commands are the Commands phase: they may
//! write the immediate effects of an action into other domains (sway,
//! awareness), which is the only cross-domain write outside effects.
//! The knowledge ledger (DEC-091) is player state too (`knowledge`).

pub mod ground;
pub mod knowledge;

use crate::politics::PoliticianId;
use crate::population::CLASSES;
use crate::scheduler::{Outs, Reads};
use crate::world::{ConstituencyId, MunicipalityId};
use mandate_core::math::clamp;
use mandate_core::{Money, SimDate};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tracks {
    /// Electability and backing (0..100; 0 = political wilderness).
    pub viability: f32,
    /// Legal exposure (0..100; 100 = prosecution).
    pub legal_jeopardy: f32,
    /// Personal security risk (0..100).
    pub threat: f32,
    /// Public trust and scandal exposure (0..100; 0 = disgrace).
    pub standing: f32,
    /// Health (0..100).
    pub health: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionRecord {
    pub date: SimDate,
    /// Fluent key of the action, e.g. `action-lobby`.
    pub action: String,
    /// Fluent key of the outcome.
    pub outcome: String,
    pub args: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub politician: PoliticianId,
    pub municipality: MunicipalityId,
    pub ward: ConstituencyId,
    pub standard_slots: u8,
    pub slots_used: u8,
    pub late_night_used: bool,
    /// Public legitimacy behind the player (0..100), DEC-011.
    pub mandate: f32,
    pub money: Money,
    pub staff: u8,
    pub tracks: Tracks,
    pub actions: Vec<ActionRecord>,
    /// What the player intends at the next local election (default: defend their seat).
    #[serde(default)]
    pub filing: Option<crate::politics::elections::Filing>,
    /// Ad buys this turn (the local market saturates).
    #[serde(default)]
    pub ads_this_turn: u8,
    /// Calendar events the player was at this turn (DEC-114; ids, cleared
    /// as the turn ends).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub at_events: Vec<String>,
    /// The last election whose result has been applied to the tracks.
    #[serde(default)]
    pub last_election_seen: Option<SimDate>,
    /// Where the player is in their town (DEC-079). Moving is free.
    #[serde(default)]
    pub location: crate::places::Location,
    /// In play, in prison, or over (DEC-006).
    #[serde(default)]
    pub status: RunStatus,
    /// What the player knows (DEC-091). Older saves load it unseeded and
    /// are seeded on load (`knowledge::on_load`).
    #[serde(default)]
    pub knowledge: knowledge::KnowledgeLedger,
    /// Conversations (DEC-093): the repetition guard, the exchange log,
    /// and the half slot left over (`talk`).
    #[serde(default)]
    pub talk: crate::talk::TalkState,
    /// The ground game's memory (DEC-097 §1): when each neighbourhood was
    /// last canvassed, and how often lately. Older saves start empty.
    #[serde(default)]
    pub ground: ground::GroundState,
}

/// Where a career stands (DEC-006): in play, out of politics for a while
/// (a short sentence: time passes, then a possible comeback), or over.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunStatus {
    #[default]
    Active,
    Imprisoned {
        until: SimDate,
        case: u32,
    },
    /// The run is over: `reason` is `prison` (a long sentence) or
    /// `disqualified`.
    Ended {
        date: SimDate,
        case: u32,
        reason: EndReason,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    Prison,
    Disqualified,
}

impl EndReason {
    pub fn key(self) -> &'static str {
        match self {
            EndReason::Prison => "prison",
            EndReason::Disqualified => "disqualified",
        }
    }
}

impl RunStatus {
    pub fn active(&self) -> bool {
        matches!(self, RunStatus::Active)
    }
}

/// Player domain, Characters phase, daily: track changes other domains asked
/// for (a scandal's cost in Standing, a case's Legal Jeopardy floor), a
/// sentence landing, release from prison, and what other systems taught the
/// player (`Learn`, `Read`, `Lead`: the knowledge ledger, DEC-091).
pub fn effects(pl: &mut PlayerState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let j = &r.content.justice;
    let me = pl.politician;
    for req in out.take(|e| match e {
        crate::scheduler::Effect::PlayerTrack { .. } => true,
        crate::scheduler::Effect::Sentenced { politician, .. } => *politician == me,
        crate::scheduler::Effect::Learn { .. }
        | crate::scheduler::Effect::Read { .. }
        | crate::scheduler::Effect::Lead { .. } => true,
        _ => false,
    }) {
        // What a sentence leads to is caused by what caused the sentence.
        let cause = req.cause;
        match req.effect {
            e @ (crate::scheduler::Effect::Learn { .. }
            | crate::scheduler::Effect::Read { .. }
            | crate::scheduler::Effect::Lead { .. }) => {
                knowledge::apply_effect(&mut pl.knowledge, e, r.date, r.content);
            }
            crate::scheduler::Effect::PlayerTrack {
                standing,
                viability,
                money,
                legal_floor,
            } => {
                let t = &mut pl.tracks;
                t.standing = (t.standing + standing).clamp(0.0, 100.0);
                t.viability = (t.viability + viability).clamp(0.0, 100.0);
                if let Some(f) = legal_floor {
                    t.legal_jeopardy = t.legal_jeopardy.max(f).min(100.0);
                }
                if money != 0.0 {
                    let m = (pl.money.units() as f64 + money).max(0.0);
                    pl.money = Money::from_units(m.round() as i64);
                }
            }
            crate::scheduler::Effect::Sentenced {
                politician,
                case,
                sentence,
            } => {
                let _ = politician;
                use crate::justice::Sentence;
                let fine = match sentence {
                    Sentence::Fine { amount } | Sentence::Removed { amount } => amount,
                    _ => 0.0,
                };
                let m = (pl.money.units() as f64 - fine).max(0.0);
                pl.money = Money::from_units(m.round() as i64);
                pl.tracks.standing = (pl.tracks.standing - j.fine_standing as f32).max(0.0);
                if !matches!(sentence, Sentence::Fine { .. }) {
                    pl.tracks.viability =
                        (pl.tracks.viability - j.removal_viability as f32).max(0.0);
                    pl.filing = None;
                }
                let years = sentence.prison_years();
                if sentence.ends_career(j.long_years) {
                    let reason = if matches!(sentence, Sentence::Disqualified { .. }) {
                        EndReason::Disqualified
                    } else {
                        EndReason::Prison
                    };
                    pl.status = RunStatus::Ended {
                        date: r.date,
                        case,
                        reason,
                    };
                    let ev = out.events.record(
                        r.date,
                        crate::events::EventKind::CareerEnded {
                            reason: reason.key().into(),
                        },
                        cause,
                    );
                    out.events.pin(ev);
                } else if years > 0 {
                    let until = r
                        .date
                        .add_days(years as u32 * mandate_core::time::DAYS_PER_YEAR);
                    pl.status = RunStatus::Imprisoned { until, case };
                    pl.location = crate::places::Location::Home;
                    let ev = out.events.record(
                        r.date,
                        crate::events::EventKind::PlayerImprisoned {
                            years: years as u32,
                            until,
                        },
                        cause,
                    );
                    out.events.pin(ev);
                }
            }
            _ => {}
        }
    }
    // Out of prison: back in town, out of office, with a long way back.
    if let RunStatus::Imprisoned { until, .. } = pl.status
        && until <= r.date
    {
        pl.status = RunStatus::Active;
        pl.tracks.standing = pl.tracks.standing.min(j.release_standing as f32);
        pl.tracks.viability = pl.tracks.viability.min(j.release_viability as f32);
        pl.tracks.legal_jeopardy = pl.tracks.legal_jeopardy.min(30.0);
        // The term ran out (the log links the release to the sentence).
        let ev = out.events.record(
            r.date,
            crate::events::EventKind::PlayerReleased,
            crate::spine::Cause::Process(crate::spine::Process::Justice),
        );
        out.events.pin(ev);
    }
}

impl PlayerState {
    /// Slots available this turn: exhaustion costs time (health is a resource).
    pub fn slot_cap(&self) -> u8 {
        let tired = (self.tracks.health < 35.0) as u8 + (self.tracks.health < 15.0) as u8;
        self.standard_slots.saturating_sub(tired).max(1)
    }

    pub fn slots_left(&self) -> u8 {
        self.slot_cap().saturating_sub(self.slots_used)
    }

    pub fn log(&mut self, date: SimDate, action: &str, outcome: &str, args: Vec<(String, String)>) {
        self.actions.push(ActionRecord {
            date,
            action: action.into(),
            outcome: outcome.into(),
            args,
        });
        if self.actions.len() > 40 {
            self.actions.remove(0);
        }
    }

    /// The town the player lives in (DEC-098: read this, not the field).
    pub fn home(&self) -> MunicipalityId {
        self.municipality
    }

    /// The jurisdictions simulated in detail around the player (DEC-098):
    /// in M6, their home town.
    pub fn focus(&self) -> Vec<crate::world::Jurisdiction> {
        vec![crate::world::Jurisdiction::town(self.home())]
    }
}

/// The town the player lives in, if there is a player (DEC-098).
pub fn home(state: &crate::State) -> Option<MunicipalityId> {
    state.player.as_ref().map(|p| p.home())
}

/// The constitution posts the player holds (DEC-098; one in M6, none out of
/// office).
pub fn posts(state: &crate::State) -> Vec<crate::politics::Post> {
    state
        .player
        .as_ref()
        .map(|p| crate::politics::posts_of(&state.politics, p.politician))
        .unwrap_or_default()
}

/// Monthly: Mandate from the player's standing with their own voters, and
/// slow movement of the pressure tracks.
pub fn monthly(pl: &mut PlayerState, r: &Reads<'_>, _out: &mut Outs<'_>) {
    let pop = r.population();
    let hoods = crate::policy::ward_hoods(r.world, pl.ward);
    let mut acc = 0.0;
    let mut w = 0.0;
    for h in hoods {
        for c in &pop.hoods[h].cohorts {
            let weight = c.count as f64 * c.turnout as f64;
            acc += (c.player_approval as f64) * (c.player_awareness as f64) * weight;
            w += weight;
        }
    }
    let k = &r.content.career;
    let net = if w > 0.0 { acc / w } else { 0.0 };
    let target = clamp(50.0 + k.mandate_per_net_approval * net, 0.0, 100.0);
    pl.mandate = (pl.mandate as f64 * (1.0 - k.mandate_rate) + target * k.mandate_rate) as f32;
    pl.tracks.viability = clamp(
        pl.tracks.viability as f64 * (1.0 - k.viability_rate)
            + (pl.mandate as f64 * (1.0 - k.viability_standing_share)
                + pl.tracks.standing as f64 * k.viability_standing_share)
                * k.viability_rate,
        0.0,
        100.0,
    ) as f32;
    pl.tracks.health = (pl.tracks.health + k.health_recovery as f32).min(100.0);
    // Legal Jeopardy eases while no case is open (a case holds its floor).
    if r.justice().open_against(pl.politician).next().is_none() {
        pl.tracks.legal_jeopardy =
            (pl.tracks.legal_jeopardy - r.content.justice.track_decay as f32).max(0.0);
    }
    // The party notices a passenger, and rewards a worker (DEC-076).
    let record = r
        .politics()
        .get(pl.politician)
        .map(|p| p.record as f64)
        .unwrap_or(0.0);
    let d = if record < k.thin_record {
        k.thin_record_standing
    } else if record > k.strong_record {
        k.strong_record_standing
    } else {
        0.0
    };
    pl.tracks.standing = clamp(pl.tracks.standing as f64 + d, 0.0, 100.0) as f32;
    // An election result lands on the career: margins become Mandate (DEC-011).
    let pol = r.politics();
    if pol.elections.last_election.is_some() && pol.elections.last_election != pl.last_election_seen
    {
        pl.last_election_seen = pol.elections.last_election;
        let date = pol.elections.last_election;
        if let Some(res) = pol
            .elections
            .results
            .iter()
            .filter(|x| Some(x.date) == date)
            .find(|x| x.candidates.iter().any(|c| c.politician == pl.politician))
        {
            let mine = res
                .candidates
                .iter()
                .find(|c| c.politician == pl.politician)
                .map(|c| c.share)
                .unwrap_or(0.0) as f64;
            let best_other = res
                .candidates
                .iter()
                .filter(|c| c.politician != pl.politician)
                .map(|c| c.share as f64)
                .fold(0.0, f64::max);
            let margin = mine - best_other;
            if res.winner == pl.politician {
                pl.mandate = clamp(
                    k.win_mandate_base + k.win_mandate_per_margin * margin,
                    k.win_mandate_min,
                    100.0,
                ) as f32;
                pl.tracks.viability =
                    clamp(pl.tracks.viability as f64 + k.win_viability, 0.0, 100.0) as f32;
                pl.tracks.standing =
                    clamp(pl.tracks.standing as f64 + k.win_standing, 0.0, 100.0) as f32;
            } else {
                pl.tracks.viability = clamp(
                    pl.tracks.viability as f64
                        + k.loss_viability
                        + k.loss_viability_per_margin * margin.max(-0.5),
                    0.0,
                    100.0,
                ) as f32;
                pl.tracks.standing =
                    clamp(pl.tracks.standing as f64 + k.loss_standing, 0.0, 100.0) as f32;
            }
        }
        // A new cycle: by default the player defends whatever they hold.
        // Only an election held during play ends a filing: the history's
        // last election lands on the career in the first month, but a
        // filing made since is for the next one.
        let play_began = r
            .meta
            .start_date
            .add_days(r.content.tuning.burn_in_years as u32 * mandate_core::time::DAYS_PER_YEAR);
        if date.is_some_and(|d| d >= play_began) {
            pl.filing = None;
        }
    }
    let _ = CLASSES;
}
