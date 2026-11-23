//! Player commands (the Commands phase, OVERVIEW §4.2). Validated against
//! Authority (constitution powers), slots, and preconditions, then applied
//! with immediate effects. Outcomes are seeded from the Player RNG stream.

use crate::characters::{self, MemoryKind, Subject};
use crate::commands::EngineError;
use crate::events::EventKind;
use crate::player::knowledge::{self, Level};
use crate::policy::families::FamilyInstanceId;
use crate::policy::{Clause, PolicyInstance, PolicyInstanceId, PolicyStatus, annual_cost};
use crate::politics::{PoliticianId, Role};
use crate::state::State;
use crate::world::{ConstituencyId, MunicipalityId, NeighborhoodId, Tier};
use mandate_content::{ContentBundle, Power, TierKey};
use mandate_core::math::clamp;
use mandate_core::{Money, Stream};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod act;
mod checks;
mod ground;
mod legislation;
mod lobbying;
mod misconduct;
mod movement;
mod powers;
mod publicity;

use act::*;
pub use checks::*;
pub use ground::*;
pub(crate) use legislation::*;
pub use lobbying::*;
pub use misconduct::*;
use movement::*;
pub use powers::*;
pub use publicity::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Persuade a council member on a bill (at their office or the diner).
    Lobby {
        politician: u32,
        policy: u32,
    },
    /// Promise a future favour in exchange for support (owed favour).
    TradeFavor {
        politician: u32,
        policy: u32,
    },
    /// Public meeting in a neighbourhood; optionally about a bill.
    TownHall {
        neighborhood: u32,
        policy: Option<u32>,
    },
    /// Statement to the local press; optionally about a bill.
    Press {
        policy: Option<u32>,
    },
    /// Door-knocking in a neighbourhood, optionally on one worry (a concern
    /// id, DEC-091 §1.6): the neighbourhood's real top worry works better,
    /// one it barely feels works worse; none is a general canvass.
    Canvass {
        neighborhood: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        issue: Option<String>,
    },
    Fundraise,
    Rest,
    /// Buy local advertising for the campaign (costs campaign funds).
    Advertise,
    /// Shady (DEC-034): take an envelope at the diner, late at night.
    TakeBribe,
    /// Shady: put household bills on the campaign card (the party office).
    DipFunds,
    /// Shady: slip a rival's confidential file to a reporter (the newspaper).
    LeakFile,
    /// Do one of the verbs the place offers (places with purpose, DEC-079
    /// note): a verb id from `actions.jsonc` `visit.verbs`, e.g.
    /// `verb.tour_works`. Its cost, conditions, effects and risk are data
    /// (`crate::visits`).
    Visit {
        verb: String,
    },
    /// "Sort it out" (DEC-097 §2): casework on an accepted ask, at the town
    /// hall's services desk, one slot. Its odds are shown before acting
    /// (`asks::casework_odds`); a failure leaves the ask open.
    Casework {
        ask: u32,
    },
}

impl Action {
    /// The action-cost and place key.
    pub fn key(&self) -> &'static str {
        match self {
            Action::Lobby { .. } => "lobby",
            Action::TradeFavor { .. } => "trade_favor",
            Action::TownHall { .. } => "town_hall",
            Action::Press { .. } => "press",
            Action::Canvass { .. } => "canvass",
            Action::Fundraise => "fundraise",
            Action::Rest => "rest",
            Action::Advertise => "advertise",
            Action::TakeBribe => "take_bribe",
            Action::DipFunds => "dip_funds",
            Action::LeakFile => "leak_file",
            Action::Visit { .. } => "visit",
            Action::Casework { .. } => "casework",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlayerCommand {
    /// Declare how the player will vote on (or, as mayor, act on) a bill.
    /// Public and final: party, ward, and colleagues react now (no slot).
    CastVote {
        policy: u32,
        vote: crate::politics::Vote,
    },
    /// Declare the player's intention for the next local election (no slot).
    File {
        filing: crate::politics::elections::Filing,
    },
    DraftPolicy {
        template: String,
        params: BTreeMap<String, f64>,
    },
    AmendPolicy {
        policy: u32,
        params: BTreeMap<String, f64>,
        clauses: Vec<Clause>,
        /// A family bill's choices, axis → option (DEC-092; missing axes
        /// keep theirs). Refused on a standard bill.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        choices: Option<BTreeMap<String, String>>,
    },
    /// Draft a policy family's open proposal (DEC-092, §2.11): one option
    /// per axis (missing: the axis default), the sliders, and optionally a
    /// sponsor whose version is adopted as it is (its price is paid now).
    /// At the town hall, free. Needs the proposal `known` (DEC-091) and a
    /// post with the family's power at the proposal's jurisdiction.
    DraftFamily {
        proposal: u32,
        #[serde(default)]
        choices: BTreeMap<String, String>,
        #[serde(default)]
        params: BTreeMap<String, f64>,
        #[serde(default)]
        sponsor: Option<String>,
    },
    IntroducePolicy {
        policy: u32,
    },
    WithdrawPolicy {
        policy: u32,
    },
    Act {
        action: Action,
        late_night: bool,
    },
    /// Move within the player's town (DEC-079): free, no slot.
    MoveTo {
        location: crate::places::Location,
    },
    /// Move, then do a command there, in one request (auto-routing). If the
    /// command fails, the move still stands.
    Go {
        location: crate::places::Location,
        then: Option<Box<PlayerCommand>>,
    },
    /// Introduce yourself to someone here (free). Meeting a passer-by
    /// (Ambient, `slot`) makes them a Named character (DEC-018).
    Meet {
        slot: u8,
    },
    /// Answer a pending decision (DEC-077 §4) with one of its options. It
    /// can be done from anywhere: the pocket lists every decision (DEC-079).
    Answer {
        decision: u32,
        option: String,
    },
    /// Talk to someone here on a topic (DEC-093): half a slot (late at
    /// night, the late slot, where private meetings are held). The result's
    /// `created` is the exchange's id.
    Talk {
        to: crate::talk::TalkTarget,
        /// Topic content id, e.g. `topic.who_runs`.
        topic: String,
        #[serde(default)]
        about: Option<crate::talk::AboutRef>,
        #[serde(default)]
        late_night: bool,
    },
    /// Answer an ask (DEC-097 §2): accept it (a promise) or decline it. Free,
    /// from anywhere.
    RespondAsk {
        ask: u32,
        accept: bool,
    },
    /// Make a campaign promise (DEC-097 §2): while campaigning, free, at most
    /// `promises.max_per_campaign` a race; `args` carries a promise's
    /// chosen answer (`answer`: a policy template), from the promise menu.
    Promise {
        template: String,
        #[serde(default)]
        args: Vec<(String, String)>,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionResult {
    pub created: Option<u32>,
    /// Fluent key describing the outcome.
    pub outcome_key: String,
    pub args: BTreeMap<String, String>,
}

/// Ad buys allowed per turn (the local market saturates).
pub const MAX_ADS_PER_TURN: u8 = 3;

/// How many favours the player may owe one colleague at once.
pub const MAX_FAVOR_DEBT: i32 = 2;

fn bad(msg: &str) -> EngineError {
    EngineError::InvalidCommand(msg.into())
}

pub fn apply(
    state: &mut State,
    content: &ContentBundle,
    cmd: &PlayerCommand,
) -> Result<ActionResult, EngineError> {
    let res = apply_command(state, content, cmd)?;
    // What the command taught about open proposals (DEC-091 hooks).
    crate::policy::drafting::learn_from(state, content, cmd);
    // A need the command met moves its ask on now (M6 note 12).
    crate::asks::settle_now(state, content);
    Ok(res)
}

fn apply_command(
    state: &mut State,
    content: &ContentBundle,
    cmd: &PlayerCommand,
) -> Result<ActionResult, EngineError> {
    let date = state.calendar.date;
    let pl = state
        .player
        .as_ref()
        .ok_or_else(|| bad("no player"))?
        .clone();
    let m = pl.municipality.0;
    let mut res = ActionResult::default();
    // In prison, or the career is over (DEC-006): nothing to do but wait.
    match pl.status {
        crate::player::RunStatus::Active => {}
        crate::player::RunStatus::Imprisoned { .. } => {
            return Err(bad("you are in prison"));
        }
        crate::player::RunStatus::Ended { .. } => {
            return Err(bad("your career is over"));
        }
    }
    match cmd {
        PlayerCommand::MoveTo { location } => return move_to(state, content, *location),
        PlayerCommand::Answer { decision, option } => {
            return crate::narrative::answer(
                state,
                content,
                crate::narrative::DecisionId(*decision),
                option,
                false,
            );
        }
        PlayerCommand::RespondAsk { ask, accept } => {
            return crate::asks::commands::respond(state, content, *ask, *accept);
        }
        PlayerCommand::Promise { template, args } => {
            return crate::asks::commands::promise(state, content, template, args);
        }
        PlayerCommand::Go { location, then } => {
            if then.as_deref().is_some_and(|c| {
                matches!(c, PlayerCommand::MoveTo { .. } | PlayerCommand::Go { .. })
            }) {
                return Err(bad("a move cannot be followed by another move"));
            }
            let moved = move_to(state, content, *location)?;
            return match then {
                Some(c) => apply(state, content, c),
                None => Ok(moved),
            };
        }
        _ => {}
    }
    // Located actions (DEC-079): each is done at a place that offers it.
    crate::places::check_at(state, content, cmd)?;
    match cmd {
        PlayerCommand::MoveTo { .. }
        | PlayerCommand::Go { .. }
        | PlayerCommand::Answer { .. }
        | PlayerCommand::RespondAsk { .. }
        | PlayerCommand::Promise { .. } => {}
        PlayerCommand::Talk {
            to,
            topic,
            about,
            late_night,
        } => {
            res = crate::talk::apply(state, content, to, topic, about.as_ref(), *late_night)?;
            if let crate::talk::TalkTarget::Person { person } = to {
                let about = match about {
                    Some(crate::talk::AboutRef::Person { person }) => Some(*person),
                    _ => None,
                };
                crate::asks::record_deed(
                    state,
                    crate::asks::Deed::Talk {
                        person: *person,
                        topic: topic.clone(),
                        about,
                    },
                );
            }
        }
        PlayerCommand::Meet { slot } => {
            let id = characters::ambient::promote(state, content, pl.location, *slot)
                .ok_or_else(|| bad("nobody like that is here now"))?;
            if let Some(me) = characters::player_character(state) {
                characters::remember(
                    &mut state.politics,
                    content,
                    date,
                    id,
                    me,
                    MemoryKind::Met,
                    None,
                    None,
                );
            }
            // Meeting someone is knowing them (DEC-091).
            knowledge::teach_one(
                state,
                knowledge::Subject::Person { person: id },
                Level::Known,
                knowledge::Source::Met { person: id },
            );
            res.created = Some(id.0);
            res.outcome_key = "outcome-met".into();
            if let Some(p) = characters::profile(&state.politics, &state.world, id) {
                res.args.insert("name".into(), p.full_name());
            }
        }
        PlayerCommand::CastVote { policy, vote } => {
            res = declare_vote(state, content, policy, vote)?;
            learn_council_work(state, *policy);
        }
        PlayerCommand::File { filing } => {
            if let Some(p) = state.player.as_mut() {
                p.filing = Some(*filing);
            }
            // New plans, new rival (DEC-077 §1).
            characters::refresh_rival(state, content);
            // `outcome-filed-<word>`: the post's role word's family (a seat
            // in a body files under the body's id, an office under its own).
            res.outcome_key = match filing {
                crate::politics::elections::Filing::Stand { post } => {
                    format!("outcome-filed-{}", post.id())
                }
                crate::politics::elections::Filing::Retire => "outcome-filed-retire".into(),
            };
        }
        PlayerCommand::DraftPolicy { template, params } => {
            res.created = Some(draft(state, content, template, params)?);
            res.outcome_key = "outcome-drafted".into();
        }
        PlayerCommand::DraftFamily {
            proposal,
            choices,
            params,
            sponsor,
        } => {
            res.created = Some(draft_family(
                state,
                content,
                FamilyInstanceId(*proposal),
                choices,
                params,
                sponsor.as_deref(),
            )?);
            res.outcome_key = "outcome-drafted".into();
        }
        PlayerCommand::AmendPolicy { .. } => {
            res = amend(state, content, cmd, &pl, m)?;
        }
        PlayerCommand::IntroducePolicy { policy } => {
            res = introduce(state, content, &pl, date, m, policy)?;
        }
        PlayerCommand::WithdrawPolicy { policy } => {
            res = withdraw(state, date, m, policy)?;
        }
        PlayerCommand::Act { action, late_night } => {
            res = perform(state, content, &pl, date, m, action, late_night)?;
        }
    }
    Ok(res)
}
