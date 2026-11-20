//! Player commands (the Commands phase, OVERVIEW §4.2). Validated against
//! Authority (constitution powers), slots, and preconditions, then applied
//! with immediate effects. Outcomes are seeded from the Player RNG stream.

use crate::characters::{self, MemoryKind, Subject};
use crate::commands::EngineError;
use crate::events::EventKind;
use crate::player::knowledge::{self, Level};
use crate::policy::{Clause, PolicyInstance, PolicyStatus, annual_cost};
use crate::politics::{PoliticianId, Role};
use crate::state::State;
use crate::world::{ConstituencyId, MunicipalityId, NeighborhoodId, Tier};
use mandate_content::{ContentBundle, Power, TierKey};
use mandate_core::math::clamp;
use mandate_core::{Money, Stream};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

/// Powers the player holds through their current role (REQ-009: read from the constitution).
pub fn player_powers(state: &State, content: &ContentBundle) -> Vec<Power> {
    let Some(pl) = &state.player else {
        return Vec::new();
    };
    let Some(me) = state.politics.get(pl.politician) else {
        return Vec::new();
    };
    powers_for(content, &state.meta.constitution, me.role)
}

/// Powers a role holds under a constitution: its post's, as the
/// constitution lists them (DEC-098). Out of office: none, only
/// campaigning and networking.
pub fn powers_for(content: &ContentBundle, constitution: &str, role: Role) -> Vec<Power> {
    let (Some(c), Some(post)) = (content.constitutions.get(constitution), role.post()) else {
        return Vec::new();
    };
    crate::politics::powers_of(c, post).to_vec()
}

/// Whether the player holds `power` for a jurisdiction (DEC-098).
pub fn player_holds_power_at(
    state: &State,
    content: &ContentBundle,
    power: Power,
    at: crate::world::Jurisdiction,
) -> bool {
    let (Some(pl), Some(c)) = (
        &state.player,
        content.constitutions.get(&state.meta.constitution),
    ) else {
        return false;
    };
    crate::politics::holds_power_at(&state.politics, c, pl.politician, power, at)
}

/// How well a canvass's worry fits the neighbourhood (DEC-091 "ignorance
/// costs effectiveness, never fairness"; M6_ENGINE_CONTRACTS §1.6): the
/// multiplier on its approval gain, `1 + match_bonus × (salience − 1)`
/// within [`floor`, `ceiling`] (`knowledge.effects.canvass`). Salience is the
/// neighbourhood's true felt level of that concern over the mean of its
/// felt concerns (1: an average worry, 0: one nobody feels here). No issue
/// (a general canvass) is 1. The truth decides; what the player knows only
/// decides which worry they can aim at. The command and any preview share
/// this function.
pub fn canvass_match(
    state: &State,
    content: &ContentBundle,
    hood: NeighborhoodId,
    issue: Option<&str>,
) -> f64 {
    let Some(issue) = issue else {
        return 1.0;
    };
    let Some(h) = state.world.neighborhoods.get(hood.index()) else {
        return 1.0;
    };
    let felt = crate::concerns::felt_concerns(
        &state.population,
        content,
        h.municipality.index(),
        &[hood.index()],
    );
    let mean = felt.iter().map(|x| x.1).sum::<f64>() / felt.len().max(1) as f64;
    let mine = felt
        .iter()
        .find(|(i, _)| content.concerns.get(*i).is_some_and(|c| c.id == issue))
        .map(|x| x.1)
        .unwrap_or(0.0);
    let salience = if mean > 0.0 { mine / mean } else { 1.0 };
    let k = &content.knowledge.effects.canvass;
    clamp(1.0 + k.match_bonus * (salience - 1.0), k.floor, k.ceiling)
}

/// A canvass fit in words (`strong` | `fair` | `weak`), for the outcome.
fn fit_band(content: &ContentBundle, fit: f64) -> &'static str {
    let k = &content.knowledge.effects.canvass;
    if fit > 1.0 && fit >= 1.0 + 0.5 * (k.ceiling - 1.0) {
        "strong"
    } else if fit < 1.0 && fit <= 1.0 - 0.5 * (1.0 - k.floor) {
        "weak"
    } else {
        "fair"
    }
}

/// Lobbying success odds shown to the player before acting (the same
/// formula as NPC deals, `npc::lobby_odds_between`).
pub fn lobby_odds(
    state: &State,
    content: &ContentBundle,
    target: PoliticianId,
    late_night: bool,
) -> f64 {
    let Some(pl) = state.player.as_ref() else {
        return 0.0;
    };
    crate::npc::lobby_odds_between(state, content, pl.politician, target, late_night)
}

fn validate_params(
    content: &ContentBundle,
    template: &str,
    params: &BTreeMap<String, f64>,
) -> Result<BTreeMap<String, f64>, EngineError> {
    let t = content
        .policies
        .get(template)
        .ok_or_else(|| bad("unknown policy template"))?;
    let mut out = BTreeMap::new();
    for p in &t.params {
        let v = params.get(&p.id).copied().unwrap_or(p.default);
        if !v.is_finite() || v < p.min || v > p.max {
            return Err(bad(&format!("parameter {} out of range", p.id)));
        }
        let snapped = p.min + ((v - p.min) / p.step).round() * p.step;
        out.insert(p.id.clone(), clamp(snapped, p.min, p.max));
    }
    Ok(out)
}

/// Pay an action's time and health (from content, DEC-073). Late-night work
/// uses the extra slot instead and costs health on top.
fn spend(
    state: &mut State,
    content: &ContentBundle,
    action: &str,
    late_night: bool,
) -> Result<(), EngineError> {
    let cost = content.tuning.action_cost(action);
    let pl = state.player.as_mut().ok_or_else(|| bad("no player"))?;
    if late_night {
        if pl.late_night_used {
            return Err(bad("late night already used this turn"));
        }
        pl.late_night_used = true;
        pl.tracks.health = (pl.tracks.health - content.actions.late_night_health as f32).max(0.0);
    } else {
        if pl.slots_left() < cost.slots {
            return Err(bad("not enough time left this turn"));
        }
        pl.slots_used += cost.slots;
    }
    pl.tracks.health = clamp(pl.tracks.health as f64 - cost.health, 0.0, 100.0) as f32;
    Ok(())
}

/// Chance a press statement turns into a gaffe (shown before acting).
pub fn press_gaffe_risk(state: &State, content: &ContentBundle) -> f64 {
    let oratory = state
        .player
        .as_ref()
        .and_then(|p| state.politics.get(p.politician))
        .map(|p| p.oratory as f64)
        .unwrap_or(0.5);
    let p = &content.actions.press;
    clamp(
        p.gaffe_base + p.gaffe_oratory * (1.0 - oratory),
        p.gaffe_min,
        p.gaffe_max,
    )
}

/// Chance a town hall turns hostile in a neighbourhood: likelier where the
/// player and the town hall are unpopular (shown before acting).
pub fn town_hall_risk(state: &State, content: &ContentBundle, hood: u32) -> f64 {
    let Some(h) = state.population.hoods.get(hood as usize) else {
        return 0.0;
    };
    let (mut ap, mut gov, mut n) = (0.0, 0.0, 0.0);
    for c in &h.cohorts {
        let w = c.count as f64;
        ap += c.player_approval as f64 * c.player_awareness as f64 * w;
        gov += c.gov_approval as f64 * w;
        n += w;
    }
    let ap = ap / n.max(1.0);
    let gov = gov / n.max(1.0);
    let t = &content.actions.town_hall;
    clamp(
        t.risk_base - t.risk_approval * ap - t.risk_government * (gov - 0.5),
        t.risk_min,
        t.risk_max,
    )
}

/// What the player's party expects on a bill: +1 yes, −1 no, 0 free vote.
pub fn party_line(state: &State, policy: u32) -> i8 {
    let Some(pl) = &state.player else { return 0 };
    let mine = state.politics.get(pl.politician).and_then(|p| p.party);
    let sponsor = state
        .policy
        .instances
        .get(policy as usize)
        .and_then(|p| p.sponsor)
        .and_then(|s| state.politics.get(s))
        .and_then(|s| s.party);
    match (mine, sponsor) {
        (Some(a), Some(b)) if a == b => 1,
        (Some(_), Some(_)) => -1,
        _ => 0,
    }
}

/// Apply campaign contact to the player's candidacy, if a race is running and
/// covers the neighbourhood (`None` = every neighbourhood in the race).
fn campaign_contact(
    state: &mut State,
    content: &ContentBundle,
    hood: Option<u32>,
    scale: f64,
) -> bool {
    let Some(me) = state.player.as_ref().map(|p| p.politician) else {
        return false;
    };
    let params = &content.politics.campaign;
    let mut touched = false;
    for race in state.politics.elections.races.iter_mut() {
        let Some(c) = race.candidates.iter_mut().find(|c| c.politician == me) else {
            continue;
        };
        for (i, h) in race.hoods.iter().enumerate() {
            if hood.is_none_or(|x| x == *h) {
                crate::politics::elections::contact(c, i, params, scale);
                touched = true;
            }
        }
    }
    touched
}

fn own_policy(state: &State, id: u32) -> Result<&PolicyInstance, EngineError> {
    let pl = state.player.as_ref().ok_or_else(|| bad("no player"))?;
    let p = state
        .policy
        .instances
        .get(id as usize)
        .ok_or_else(|| bad("no such policy"))?;
    if p.sponsor != Some(pl.politician) {
        return Err(bad("you are not the sponsor"));
    }
    Ok(p)
}

fn local_policy(state: &State, id: u32) -> Result<&PolicyInstance, EngineError> {
    let pl = state.player.as_ref().ok_or_else(|| bad("no player"))?;
    let p = state
        .policy
        .instances
        .get(id as usize)
        .ok_or_else(|| bad("no such policy"))?;
    if p.jurisdiction != pl.municipality.0 {
        return Err(bad("that policy is not in your town"));
    }
    Ok(p)
}

fn validate_clauses(state: &State, m: u32, clauses: &[Clause]) -> Result<(), EngineError> {
    let earmarks = clauses
        .iter()
        .filter(|c| matches!(c, Clause::Earmark { .. }))
        .count();
    if earmarks > 3 {
        return Err(bad("at most three earmarks"));
    }
    for c in clauses {
        match c {
            Clause::Earmark { ward } => {
                let ok = state
                    .world
                    .constituencies
                    .get(ward.index())
                    .is_some_and(|w| w.tier == Tier::Local && w.jurisdiction == m);
                if !ok {
                    return Err(bad("earmark ward is not in this town"));
                }
            }
            Clause::Sunset { years } => {
                if !(1..=10).contains(years) {
                    return Err(bad("sunset must be 1–10 years"));
                }
            }
            Clause::Pilot { neighborhood } => {
                let ok = state
                    .world
                    .neighborhoods
                    .get(*neighborhood as usize)
                    .is_some_and(|n| n.municipality.0 == m);
                if !ok {
                    return Err(bad("pilot neighbourhood is not in this town"));
                }
            }
        }
    }
    Ok(())
}

pub fn apply(
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
        PlayerCommand::MoveTo { .. } | PlayerCommand::Go { .. } | PlayerCommand::Answer { .. } => {}
        PlayerCommand::Talk {
            to,
            topic,
            about,
            late_night,
        } => {
            res = crate::talk::apply(state, content, to, topic, about.as_ref(), *late_night)?;
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
        PlayerCommand::AmendPolicy {
            policy,
            params,
            clauses,
        } => {
            let p = own_policy(state, *policy)?;
            let status = p.status;
            let template = p.template.clone();
            if !matches!(
                status,
                PolicyStatus::Draft | PolicyStatus::UnderConsideration
            ) {
                return Err(bad("only drafts and bills can be amended"));
            }
            let params = validate_params(content, &template, params)?;
            validate_clauses(state, m, clauses)?;
            if status == PolicyStatus::UnderConsideration {
                spend(state, content, "amend", false)?;
                crate::politics::add_record(
                    &mut state.politics,
                    pl.politician,
                    content.career.record_gain.amend as f32,
                );
                // Amendments cost goodwill already built.
                let keep = content.actions.amend.sway_keep as f32;
                for (k, v) in state.politics.sway.iter_mut() {
                    if k.1 == *policy {
                        *v *= keep;
                    }
                }
            }
            if let Some(p) = state.policy.instances.get_mut(*policy as usize) {
                p.params = params;
                p.clauses = clauses.clone();
            }
            res.outcome_key = "outcome-amended".into();
        }
        PlayerCommand::IntroducePolicy { policy } => {
            let p = own_policy(state, *policy)?;
            if p.status != PolicyStatus::Draft {
                return Err(bad("only drafts can be introduced"));
            }
            let days = content
                .constitutions
                .get(&state.meta.constitution)
                .and_then(|c| c.tier(TierKey::Local))
                .and_then(|t| t.procedure("ordinance"))
                .map(|p| p.deliberation_days)
                .unwrap_or(21);
            spend(state, content, "introduce", false)?;
            crate::politics::add_record(
                &mut state.politics,
                pl.politician,
                content.career.record_gain.introduce as f32,
            );
            let ev = state.events.record(
                date,
                EventKind::PolicyIntroduced {
                    policy: *policy,
                    municipality: m,
                    sponsor: pl.politician.0,
                },
                vec![],
            );
            if let Some(p) = state.policy.instances.get_mut(*policy as usize) {
                p.status = PolicyStatus::UnderConsideration;
                p.introduced_on = Some(date);
                p.next_stage_on = Some(date.add_days(days as u32));
                p.awareness = content.actions.introduce.awareness as f32;
                p.cause = Some(ev);
            }
            learn_council_work(state, *policy);
            res.outcome_key = "outcome-introduced".into();
            res.args.insert("days".into(), days.to_string());
        }
        PlayerCommand::WithdrawPolicy { policy } => {
            let p = own_policy(state, *policy)?;
            if !matches!(
                p.status,
                PolicyStatus::Draft | PolicyStatus::UnderConsideration
            ) {
                return Err(bad("only drafts and bills can be withdrawn"));
            }
            let was_bill = p.status == PolicyStatus::UnderConsideration;
            if let Some(p) = state.policy.instances.get_mut(*policy as usize) {
                p.status = PolicyStatus::Ended;
                p.end_reason = Some(crate::policy::EndReason::Withdrawn);
                p.next_stage_on = None;
            }
            if was_bill {
                state.events.record(
                    date,
                    EventKind::PolicyWithdrawn {
                        policy: *policy,
                        municipality: m,
                    },
                    vec![],
                );
            }
            res.outcome_key = "outcome-withdrawn".into();
        }
        PlayerCommand::Act { action, late_night } => {
            if *late_night && matches!(action, Action::Advertise) {
                return Err(bad("ads are bought, not held late at night"));
            }
            if *late_night
                && !matches!(
                    action,
                    Action::Lobby { .. }
                        | Action::TradeFavor { .. }
                        | Action::Fundraise
                        | Action::TakeBribe
                )
            {
                return Err(bad(
                    "only private meetings and fundraising happen late at night",
                ));
            }
            if !*late_night && matches!(action, Action::TakeBribe) {
                return Err(bad("envelopes change hands late at night"));
            }
            match action {
                Action::Lobby { politician, policy }
                | Action::TradeFavor { politician, policy } => {
                    local_policy(state, *policy)?;
                    let target = PoliticianId(*politician);
                    let t = state
                        .politics
                        .get(target)
                        .ok_or_else(|| bad("no such politician"))?;
                    if t.municipality() != m || t.is_player {
                        return Err(bad("you can only lobby colleagues in your town"));
                    }
                    let (tc, me) = (
                        t.character,
                        state
                            .politics
                            .get(pl.politician)
                            .map(|p| p.character)
                            .ok_or_else(|| bad("no player"))?,
                    );
                    if matches!(action, Action::TradeFavor { .. })
                        && characters::favours_owed(&state.politics, me, tc) as i32
                            >= MAX_FAVOR_DEBT
                    {
                        return Err(bad("you already owe them too much"));
                    }
                    let odds = lobby_odds(state, content, target, *late_night);
                    let negotiation = state
                        .politics
                        .get(pl.politician)
                        .map(|x| x.negotiation)
                        .unwrap_or(0.5) as f64;
                    spend(
                        state,
                        content,
                        if matches!(action, Action::TradeFavor { .. }) {
                            "trade_favor"
                        } else {
                            "lobby"
                        },
                        *late_night,
                    )?;
                    crate::politics::add_record(
                        &mut state.politics,
                        pl.politician,
                        content.career.record_gain.lobby as f32,
                    );
                    let rng = state.rng.get(Stream::Player);
                    let trade = matches!(action, Action::TradeFavor { .. });
                    let success = trade || rng.chance(odds);
                    let lp = &content.council.lobby;
                    let fx = &content.actions.lobby;
                    let gain = if trade {
                        fx.trade_gain
                    } else if success {
                        rng.range_f64(lp.gain[0], lp.gain[1]) * (0.5 + negotiation)
                    } else {
                        lp.fail_gain
                    };
                    *state
                        .politics
                        .sway
                        .entry((*politician, *policy))
                        .or_default() += gain as f32;
                    let subject = Some(Subject::Policy { id: *policy });
                    if trade {
                        // A promise: the player owes them one (DEC-031 favour ledger).
                        characters::owe(&mut state.politics, me, tc, "favour-bill", subject, date);
                    }
                    let kind = if trade {
                        MemoryKind::PromisedFavour
                    } else if success {
                        MemoryKind::LobbiedMe
                    } else {
                        MemoryKind::PressedMe
                    };
                    characters::remember(
                        &mut state.politics,
                        content,
                        date,
                        tc,
                        me,
                        kind,
                        subject,
                        Some(if success {
                            fx.opinion_success
                        } else {
                            fx.opinion_fail
                        }),
                    );
                    // Lobbying is meeting (DEC-091): `known`; a second
                    // meeting, or one at the diner late at night, `detailed`.
                    let deep = *late_night
                        || state.player.as_ref().is_some_and(|p| {
                            p.knowledge
                                .level(&knowledge::Subject::Person { person: tc })
                                >= Level::Known
                        });
                    let src = knowledge::action_source(state, action.key());
                    knowledge::teach_one(
                        state,
                        knowledge::Subject::Person { person: tc },
                        if deep { Level::Detailed } else { Level::Known },
                        src,
                    );
                    if *late_night && let Some(p) = state.player.as_mut() {
                        // Back-room deals carry legal exposure (DEC-006).
                        p.tracks.legal_jeopardy =
                            (p.tracks.legal_jeopardy + fx.late_night_legal as f32).min(100.0);
                    }
                    if *late_night {
                        // A back-room deal is a hidden fact (DEC-034): its fate
                        // is settled now; the director times when it surfaces.
                        crate::media::hide(
                            state,
                            content,
                            &content.media.late_night_deal,
                            pl.politician,
                            vec![crate::narrative::Actor::Person { id: tc }],
                        );
                    }
                    res.outcome_key = if trade {
                        "outcome-favor-traded"
                    } else if success {
                        "outcome-lobby-success"
                    } else {
                        "outcome-lobby-fail"
                    }
                    .into();
                    let name = state
                        .politics
                        .get(target)
                        .map(|t| format!("{} {}", t.given, t.family))
                        .unwrap_or_default();
                    res.args.insert("name".into(), name);
                }
                Action::TownHall {
                    neighborhood,
                    policy,
                } => {
                    let h = state
                        .world
                        .neighborhoods
                        .get(*neighborhood as usize)
                        .ok_or_else(|| bad("no such neighbourhood"))?;
                    if h.municipality.0 != m {
                        return Err(bad("that neighbourhood is not in your town"));
                    }
                    if let Some(id) = policy {
                        local_policy(state, *id)?;
                    }
                    let risk = town_hall_risk(state, content, *neighborhood);
                    spend(state, content, "town_hall", false)?;
                    crate::politics::add_record(
                        &mut state.politics,
                        pl.politician,
                        content.career.record_gain.town_hall as f32,
                    );
                    let fx = &content.actions.town_hall;
                    let hostile = state.rng.get(Stream::Player).chance(risk);
                    let oratory = state
                        .politics
                        .get(pl.politician)
                        .map(|x| x.oratory)
                        .unwrap_or(0.5) as f64;
                    let sig = policy
                        .and_then(|id| state.policy.instances.get(id as usize))
                        .and_then(|p| content.policies.get(&p.template))
                        .map(|t| t.signature_weights.clone());
                    for c in state.population.hoods[*neighborhood as usize]
                        .cohorts
                        .iter_mut()
                    {
                        c.player_awareness = clamp(
                            c.player_awareness as f64
                                + fx.awareness * (fx.awareness_oratory_base + oratory),
                            0.0,
                            1.0,
                        ) as f32;
                        let nudge = sig
                            .as_ref()
                            .map(|s| crate::politics::alignment(s, &c.attitudes) * fx.bill_nudge)
                            .unwrap_or(fx.nudge);
                        let room = 1.0 - (c.player_approval as f64).abs();
                        // A hostile crowd: people still learn your name, but not kindly.
                        let nudge = if hostile { fx.hostile_nudge } else { nudge };
                        c.player_approval =
                            clamp(c.player_approval as f64 + nudge * room, -1.0, 1.0) as f32;
                    }
                    if let Some(id) = policy
                        && let Some(p) = state.policy.instances.get_mut(*id as usize)
                    {
                        p.awareness = clamp(
                            p.awareness as f64 + fx.bill_awareness * (0.5 + oratory),
                            0.0,
                            1.0,
                        ) as f32;
                    }
                    let scale = (fx.contact_base + fx.contact_oratory * oratory)
                        * if hostile { fx.hostile_contact } else { 1.0 };
                    campaign_contact(state, content, Some(*neighborhood), scale);
                    // A meeting hears the neighbourhood out (DEC-091); with
                    // a bill, who supports it.
                    let src = knowledge::action_source(state, "town_hall");
                    knowledge::learn_hood(
                        state,
                        content,
                        NeighborhoodId(*neighborhood),
                        src.clone(),
                    );
                    if let Some(t) = policy
                        .and_then(|id| state.policy.instances.get(id as usize))
                        .map(|p| p.template.clone())
                    {
                        knowledge::teach_one(
                            state,
                            knowledge::Subject::PolicySupport {
                                town: MunicipalityId(m),
                                policy: knowledge::PolicyRef::Template { template: t },
                            },
                            Level::Known,
                            src,
                        );
                    }
                    res.outcome_key = if hostile {
                        "outcome-town-hall-hostile"
                    } else {
                        "outcome-town-hall"
                    }
                    .into();
                    res.args.insert(
                        "place".into(),
                        state.world.neighborhoods[*neighborhood as usize]
                            .name
                            .clone(),
                    );
                }
                Action::Press { policy } => {
                    if let Some(id) = policy {
                        local_policy(state, *id)?;
                    }
                    let risk = press_gaffe_risk(state, content);
                    spend(state, content, "press", false)?;
                    crate::politics::add_record(
                        &mut state.politics,
                        pl.politician,
                        content.career.record_gain.press as f32,
                    );
                    let fx = &content.actions.press;
                    let gaffe = state.rng.get(Stream::Player).chance(risk);
                    let hoods: Vec<usize> = state.world.hoods_of(MunicipalityId(m)).collect();
                    for h in hoods {
                        for c in state.population.hoods[h].cohorts.iter_mut() {
                            c.player_awareness =
                                clamp(c.player_awareness as f64 + fx.awareness, 0.0, 1.0) as f32;
                            if gaffe {
                                c.player_approval = clamp(
                                    c.player_approval as f64
                                        - fx.gaffe_approval * c.player_awareness as f64,
                                    -1.0,
                                    1.0,
                                ) as f32;
                            }
                        }
                    }
                    if let Some(id) = policy
                        && let Some(p) = state.policy.instances.get_mut(*id as usize)
                    {
                        p.awareness =
                            clamp(p.awareness as f64 + fx.bill_awareness, 0.0, 1.0) as f32;
                    }
                    if !gaffe {
                        campaign_contact(state, content, None, fx.contact);
                    }
                    // Outlets decide whether and how to cover it (DEC-034).
                    crate::media::statement(state, content, pl.politician, *policy);
                    res.outcome_key = if gaffe {
                        "outcome-press-gaffe"
                    } else {
                        "outcome-press"
                    }
                    .into();
                }
                Action::Canvass {
                    neighborhood,
                    issue,
                } => {
                    let h = state
                        .world
                        .neighborhoods
                        .get(*neighborhood as usize)
                        .ok_or_else(|| bad("no such neighbourhood"))?;
                    if Some(h.municipality) != crate::player::home(state) {
                        return Err(bad("that neighbourhood is not in your town"));
                    }
                    if let Some(i) = issue
                        && !content.concerns.iter().any(|c| &c.id == i)
                    {
                        return Err(bad("no such worry"));
                    }
                    let hood = NeighborhoodId(*neighborhood);
                    let fx = &content.actions.canvass;
                    // Decided before the doorsteps change anything: how the
                    // worry fits, and how fresh the doorsteps are (DEC-097 §1).
                    let fit = canvass_match(state, content, hood, issue.as_deref());
                    let curve = crate::player::ground::canvass_curve(state, fx, hood);
                    spend(state, content, "canvass", false)?;
                    crate::politics::add_record(
                        &mut state.politics,
                        pl.politician,
                        content.career.record_gain.canvass as f32,
                    );
                    for c in state.population.hoods[hood.index()].cohorts.iter_mut() {
                        // Contact builds recognition first; persuasion is
                        // small, saturating, fades with repeated visits, and
                        // counts for more in campaign season (DEC-071 §5,
                        // DEC-097 §1).
                        let aw = c.player_awareness as f64;
                        c.player_awareness =
                            clamp(aw + fx.awareness * (1.0 - aw) * curve.awareness, 0.0, 1.0)
                                as f32;
                        let room = 1.0 - (c.player_approval as f64).abs();
                        c.player_approval = clamp(
                            c.player_approval as f64 + fx.approval * fit * room * curve.approval,
                            -1.0,
                            1.0,
                        ) as f32;
                    }
                    // A full day on the doorsteps: deeper than any other
                    // contact (the race model has its own diminishing).
                    campaign_contact(
                        state,
                        content,
                        Some(*neighborhood),
                        fx.contact * curve.same_turn,
                    );
                    let (on, turn) = (state.calendar.date, state.calendar.turn);
                    if let Some(p) = state.player.as_mut() {
                        p.ground.remember(hood, curve.visits + 1.0, on, turn);
                    }
                    learn_canvass(state, content, hood);
                    res.outcome_key = "outcome-canvass".into();
                    // How fresh the doorsteps were (the player's own memory).
                    res.args.insert(
                        "fresh".into(),
                        crate::player::ground::fresh_band(curve.fresh).into(),
                    );
                    if let Some(i) = issue {
                        // How the worry landed (the player has just heard
                        // the doorsteps, so this tells nothing hidden).
                        res.args.insert("issue".into(), i.clone());
                        res.args.insert("fit".into(), fit_band(content, fit).into());
                    }
                    res.args.insert(
                        "place".into(),
                        state.world.neighborhoods[*neighborhood as usize]
                            .name
                            .clone(),
                    );
                }
                Action::Fundraise => {
                    spend(state, content, "fundraise", *late_night)?;
                    let oratory = state
                        .politics
                        .get(pl.politician)
                        .map(|x| x.oratory)
                        .unwrap_or(0.5) as f64;
                    let fx = &content.actions.fundraise;
                    let raised = state
                        .rng
                        .get(Stream::Player)
                        .range_f64(fx.raised[0], fx.raised[1])
                        * (0.5 + oratory);
                    let money = Money::from_units(raised as i64);
                    if let Some(p) = state.player.as_mut() {
                        p.money += money;
                        // The party likes a fundraiser; late-night money invites scrutiny.
                        p.tracks.standing = (p.tracks.standing + fx.standing as f32).min(100.0);
                        if *late_night {
                            p.tracks.legal_jeopardy =
                                (p.tracks.legal_jeopardy + fx.late_night_legal as f32).min(100.0);
                        }
                    }
                    if *late_night {
                        crate::media::hide(
                            state,
                            content,
                            &content.media.late_night_fundraising,
                            pl.politician,
                            Vec::new(),
                        );
                    }
                    // Donor ties: low-income voters who know you notice who you court.
                    let hoods: Vec<usize> = state.world.hoods_of(MunicipalityId(m)).collect();
                    for h in hoods {
                        for (_, c) in state.population.hoods[h]
                            .cohorts
                            .iter_mut()
                            .enumerate()
                            .filter(|(ci, _)| crate::population::class_of(*ci) == 0)
                        {
                            c.player_approval = clamp(
                                c.player_approval as f64
                                    - fx.donor_ties * c.player_awareness as f64,
                                -1.0,
                                1.0,
                            ) as f32;
                        }
                    }
                    // The party office (DEC-091): whoever is found there is
                    // met, and there is talk of the rival's plans.
                    let src = knowledge::action_source(state, "fundraise");
                    let met: Vec<characters::CharacterId> = state
                        .player
                        .as_ref()
                        .map(|p| characters::ambient::stored_at(state, content, p.location))
                        .unwrap_or_default();
                    let mut grants: Vec<knowledge::Grant> = met
                        .into_iter()
                        .filter(|c| Some(*c) != characters::player_character(state))
                        .map(|person| knowledge::Grant {
                            subject: knowledge::Subject::Person { person },
                            level: Level::Known,
                        })
                        .collect();
                    grants.push(knowledge::Grant {
                        subject: knowledge::Subject::RivalPlans {
                            town: MunicipalityId(m),
                        },
                        level: Level::Heard,
                    });
                    knowledge::teach(state, grants, &src);
                    res.outcome_key = "outcome-fundraise".into();
                    res.args.insert("amount".into(), money.units().to_string());
                }
                Action::Advertise => {
                    let cost = Money::from_units(content.politics.campaign.ad_cost);
                    if state.player.as_ref().is_none_or(|p| p.money < cost) {
                        return Err(bad("not enough campaign funds"));
                    }
                    let me = pl.politician;
                    if !state
                        .politics
                        .elections
                        .races
                        .iter()
                        .any(|r| r.candidates.iter().any(|c| c.politician == me))
                    {
                        return Err(bad("you are not running in a campaign"));
                    }
                    // Money, not time: no slot, but at most three buys a turn.
                    let n = state.player.as_ref().map(|p| p.ads_this_turn).unwrap_or(0);
                    if n >= MAX_ADS_PER_TURN {
                        return Err(bad("the ad market is saturated this turn"));
                    }
                    if let Some(p) = state.player.as_mut() {
                        p.money -= cost;
                        p.ads_this_turn += 1;
                    }
                    for race in state.politics.elections.races.iter_mut() {
                        if let Some(c) = race.candidates.iter_mut().find(|c| c.politician == me) {
                            crate::politics::elections::advertise(c, &content.politics.campaign, n);
                        }
                    }
                    // Changing the subject cools a running scandal a little.
                    crate::media::advertised(state, content, me);
                    res.outcome_key = "outcome-advertise".into();
                    res.args.insert("amount".into(), cost.units().to_string());
                }
                Action::Rest => {
                    spend(state, content, "rest", false)?;
                    res.outcome_key = "outcome-rest".into();
                }
                Action::TakeBribe | Action::DipFunds | Action::LeakFile => {
                    res = shady(state, content, action, *late_night)?;
                }
            }
            let action_key = format!("action-{}", action.key().replace('_', "-"));
            if let Some(p) = state.player.as_mut() {
                p.log(
                    date,
                    &action_key,
                    &res.outcome_key,
                    res.args
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                );
            }
        }
    }
    Ok(res)
}

/// Who offers a bribe in the player's town: the power broker, else a
/// business leader or plant manager (a Named person who wants something).
pub fn briber(state: &State) -> Option<characters::CharacterId> {
    let pl = state.player.as_ref()?;
    let m = pl.municipality.0;
    let me = characters::player_character(state);
    let town = state.politics.characters.towns.get(m as usize);
    town.and_then(|t| t.broker)
        .filter(|b| {
            Some(*b) != me
                && state
                    .politics
                    .characters
                    .get(*b)
                    .is_some_and(|c| c.politician.is_none())
        })
        .or_else(|| {
            state
                .politics
                .characters
                .people
                .iter()
                .filter(|c| {
                    c.named.as_deref().is_some_and(|n| {
                        matches!(
                            n.occupation.as_str(),
                            "occupation.business_leader" | "occupation.plant_manager"
                        ) && n
                            .place
                            .and_then(|p| state.world.places.get(p.index()))
                            .is_some_and(|p| p.municipality.0 == m)
                    })
                })
                .map(|c| c.id)
                .next()
        })
}

/// The rival a leak targets: the town's named rival (a politician).
pub fn leak_target(state: &State) -> Option<PoliticianId> {
    let pl = state.player.as_ref()?;
    let rival = state
        .politics
        .characters
        .towns
        .get(pl.municipality.index())?
        .rival?;
    state.politics.characters.get(rival)?.politician
}

/// Shady actions (DEC-034, DEC-073): a real payoff now, a hidden fact that
/// may come out later (the risk is shown before acting).
fn shady(
    state: &mut State,
    content: &ContentBundle,
    action: &Action,
    late_night: bool,
) -> Result<ActionResult, EngineError> {
    let key = action.key();
    let def = content
        .media
        .shady_actions
        .get(key)
        .ok_or_else(|| bad("that is not possible here"))?
        .clone();
    let me = state
        .player
        .as_ref()
        .map(|p| p.politician)
        .ok_or_else(|| bad("no player"))?;
    let mut res = ActionResult::default();
    let mut actors = Vec::new();
    let rival = match action {
        Action::TakeBribe => {
            let b = briber(state).ok_or_else(|| bad("nobody here is offering"))?;
            actors.push(crate::narrative::Actor::Person { id: b });
            if let Some(p) = characters::profile(&state.politics, &state.world, b) {
                res.args.insert("name".into(), p.full_name());
            }
            None
        }
        Action::DipFunds => {
            if state
                .player
                .as_ref()
                .is_some_and(|p| (p.money.units() as f64) < -def.money)
            {
                return Err(bad("not enough campaign funds"));
            }
            None
        }
        _ => {
            let r = leak_target(state).ok_or_else(|| bad("you have no rival to leak about"))?;
            if let Some(c) = state.politics.get(r).map(|p| p.character) {
                actors.push(crate::narrative::Actor::Person { id: c });
            }
            res.args.insert(
                "name".into(),
                state
                    .politics
                    .get(r)
                    .map(|p| p.full_name())
                    .unwrap_or_default(),
            );
            Some(r)
        }
    };
    spend(state, content, key, late_night)?;
    if let Some(p) = state.player.as_mut() {
        let money = (p.money.units() as f64 + def.money).max(0.0);
        p.money = Money::from_units(money.round() as i64);
        p.tracks.health = clamp(p.tracks.health as f64 + def.health, 0.0, 100.0) as f32;
        p.tracks.legal_jeopardy =
            clamp(p.tracks.legal_jeopardy as f64 + def.legal, 0.0, 100.0) as f32;
    }
    let hidden = crate::media::hide(state, content, &def.hidden, me, actors);
    res.outcome_key = format!("outcome-{}", key.replace('_', "-"));
    if def.money > 0.0 {
        res.args
            .insert("amount".into(), (def.money as i64).to_string());
    }
    if let Some(r) = rival {
        match crate::media::leak(state, r, hidden) {
            Some(crate::media::Leak::Exposed { .. }) => {
                res.outcome_key = "outcome-leak-file-exposed".into();
            }
            Some(crate::media::Leak::Rumour { .. }) => {
                res.outcome_key = "outcome-leak-file-rumour".into();
            }
            None => {}
        }
    }
    Ok(res)
}

/// Declare the player's vote on a bill (public and final): party, ward, and
/// colleagues react now. Shared by the council chamber and decisions.
pub(crate) fn declare_vote(
    state: &mut State,
    content: &ContentBundle,
    policy: &u32,
    vote: &crate::politics::Vote,
) -> Result<ActionResult, EngineError> {
    let date = state.calendar.date;
    let pl = state
        .player
        .as_ref()
        .ok_or_else(|| bad("no player"))?
        .clone();
    let mut res = ActionResult::default();
    use crate::politics::Vote;
    let p = local_policy(state, *policy)?;
    if p.status != PolicyStatus::UnderConsideration {
        return Err(bad("only bills before the council can be voted on"));
    }
    if p.sponsor == Some(pl.politician) {
        return Err(bad("sponsors vote for their own bills"));
    }
    // A vote (or a signature) on the bill, for the bill's jurisdiction.
    let at = p.at();
    if !player_holds_power_at(state, content, Power::ApproveOrdinance, at)
        && !player_holds_power_at(state, content, Power::Veto, at)
    {
        return Err(bad("you have no vote on this bill"));
    }
    let key = (pl.politician.0, *policy);
    if state.politics.declared_votes.contains_key(&key) {
        return Err(bad("you have already declared your vote"));
    }
    let template = p.template.clone();
    let sponsor = p.sponsor;
    let awareness = p.awareness as f64;
    let sig = content
        .policies
        .get(&template)
        .map(|t| t.signature_weights.clone())
        .unwrap_or_default();
    let line = party_line(state, *policy);
    let sign = match vote {
        Vote::Yes => 1.0,
        Vote::No => -1.0,
        Vote::Abstain => 0.0,
    };
    state.politics.declared_votes.insert(key, sign as i8);
    let fx = &content.actions.vote;
    crate::politics::add_record(
        &mut state.politics,
        pl.politician,
        content.career.record_gain.vote as f32,
    );
    // Party: loyalty is noticed a little, rebellion a lot.
    if let Some(pls) = state.player.as_mut() {
        let d = if line == 0 || *vote == Vote::Abstain {
            if *vote == Vote::Abstain {
                fx.standing_abstain
            } else {
                0.0
            }
        } else if (line as f64) == sign {
            fx.standing_loyal
        } else {
            fx.standing_defy
        };
        pls.tracks.standing = clamp(pls.tracks.standing as f64 + d, 0.0, 100.0) as f32;
    }
    // The ward: voters who care reward or punish the stand; dodging costs a little.
    for h in crate::policy::ward_hoods(&state.world, pl.ward) {
        for c in state.population.hoods[h].cohorts.iter_mut() {
            let a = crate::politics::alignment(&sig, &c.attitudes);
            let room = 1.0 - (c.player_approval as f64).abs();
            let delta = if *vote == Vote::Abstain {
                fx.ward_abstain
            } else {
                fx.ward_reaction * a * sign * (0.5 + awareness) * room
            };
            c.player_approval = clamp(c.player_approval as f64 + delta, -1.0, 1.0) as f32;
        }
    }
    // The sponsor: support earns goodwill and repays an owed favour;
    // voting against them while owing them is a broken promise.
    let me = state.politics.get(pl.politician).map(|p| p.character);
    let subject = Some(Subject::Policy { id: *policy });
    if let Some(sid) = sponsor
        && let Some(sp) = state.politics.get(sid)
        && !sp.is_player
        && let Some(me) = me
    {
        let (sc, name) = (sp.character, sp.full_name());
        let owed = characters::favours_owed(&state.politics, me, sc) > 0;
        let pol = &mut state.politics;
        match vote {
            Vote::Yes => {
                characters::remember(
                    pol,
                    content,
                    date,
                    sc,
                    me,
                    MemoryKind::BackedMyBill,
                    subject,
                    Some(fx.sponsor_yes),
                );
                if owed && let Some(f) = characters::repay(pol, me, sc) {
                    characters::remember(
                        pol,
                        content,
                        date,
                        sc,
                        me,
                        MemoryKind::KeptPromise,
                        f.subject.or(subject),
                        None,
                    );
                    res.args.insert("repaid".into(), name);
                }
            }
            Vote::No => {
                let (kind, hit) = if owed {
                    (MemoryKind::BrokePromise, fx.sponsor_no_owed)
                } else {
                    (MemoryKind::OpposedMyBill, fx.sponsor_no)
                };
                characters::remember(pol, content, date, sc, me, kind, subject, Some(-hit));
            }
            Vote::Abstain => {}
        }
    }
    // Breaking with the party line is noticed by party colleagues.
    if line != 0
        && *vote != Vote::Abstain
        && (line as f64) != sign
        && let Some(me) = me
    {
        let party = state.politics.get(pl.politician).and_then(|p| p.party);
        let colleagues: Vec<_> = crate::politics::officials_at(&state.politics, at)
            .into_iter()
            .filter_map(|id| state.politics.get(id))
            .filter(|p| !p.is_player && p.party.is_some() && p.party == party)
            .map(|p| p.character)
            .collect();
        for c in colleagues {
            characters::remember(
                &mut state.politics,
                content,
                date,
                c,
                me,
                MemoryKind::BrokeRanks,
                subject,
                None,
            );
        }
    }
    res.outcome_key = match vote {
        Vote::Yes => "outcome-vote-yes",
        Vote::No => "outcome-vote-no",
        Vote::Abstain => "outcome-vote-abstain",
    }
    .into();

    Ok(res)
}

/// Draft a bill sponsored by the player. Returns the new policy id.
pub(crate) fn draft(
    state: &mut State,
    content: &ContentBundle,
    template: &String,
    params: &BTreeMap<String, f64>,
) -> Result<u32, EngineError> {
    let pl = state
        .player
        .as_ref()
        .ok_or_else(|| bad("no player"))?
        .clone();
    let m = pl.municipality.0;
    let t = content
        .policies
        .get(template)
        .ok_or_else(|| bad("unknown policy template"))?;
    if t.tier != TierKey::Local {
        return Err(bad(
            "that policy is not available at your level of government",
        ));
    }
    // Drafted for the player's town (the local tier, checked above).
    let at = crate::world::Jurisdiction::town(pl.home());
    if !player_holds_power_at(state, content, t.requires, at) {
        return Err(bad("your office cannot propose this"));
    }
    if !crate::policy::unlocked(state, content, t, m) {
        return Err(bad(
            "that policy is not on the table while the town's situation does not call for it",
        ));
    }
    let params = validate_params(content, template, params)?;
    let id = state.policy.ids.alloc();
    state.policy.instances.push(PolicyInstance {
        id,
        template: template.clone(),
        tier: TierKey::Local,
        jurisdiction: m,
        sponsor: Some(pl.politician),
        params,
        clauses: Vec::new(),
        status: PolicyStatus::Draft,
        stage: 0,
        next_stage_on: None,
        votes: Vec::new(),
        introduced_on: None,
        enacted_on: None,
        progress: 0.0,
        awareness: 0.0,
        end_reason: None,
        cause: None,
        ledger: BTreeMap::new(),
    });
    Ok(id.0)
}

/// Canvassing a neighbourhood (DEC-091, §1.3): its worries `known` (a
/// second canvass reads them: `detailed`, dated), and one resident found on
/// its streets met. Nobody new is created: only people already stored.
fn learn_canvass(state: &mut State, content: &ContentBundle, hood: NeighborhoodId) {
    let src = knowledge::action_source(state, "canvass");
    knowledge::learn_hood(state, content, hood, src.clone());
    let me = characters::player_character(state);
    let Some(pl) = state.player.as_ref() else {
        return;
    };
    let resident = state.politics.characters.people.iter().find(|c| {
        Some(c.id) != me
            && c.named.as_deref().is_some_and(|n| n.home == hood.0)
            && pl
                .knowledge
                .level(&knowledge::Subject::Person { person: c.id })
                < Level::Known
    });
    if let Some(person) = resident.map(|c| c.id) {
        knowledge::teach_one(
            state,
            knowledge::Subject::Person { person },
            Level::Known,
            src,
        );
    }
}

/// Council work on a bill (introducing it, declaring a vote, DEC-091): the
/// debate shows every colleague's stance (`known`) and who backs the bill.
fn learn_council_work(state: &mut State, policy: u32) {
    let Some(p) = state.policy.instances.get(policy as usize) else {
        return;
    };
    let (template, town) = (p.template.clone(), MunicipalityId(p.jurisdiction));
    let src = knowledge::Source::Council {
        policy: crate::policy::PolicyInstanceId(policy),
    };
    let mut grants: Vec<knowledge::Grant> = knowledge::council_people(state)
        .into_iter()
        .map(|person| knowledge::Grant {
            subject: knowledge::Subject::Person { person },
            level: Level::Known,
        })
        .collect();
    grants.push(knowledge::Grant {
        subject: knowledge::Subject::PolicySupport {
            town,
            policy: knowledge::PolicyRef::Template { template },
        },
        level: Level::Known,
    });
    knowledge::teach(state, grants, &src);
}

/// Move within the town: free (DEC-079 rule 1). Arriving teaches lightly
/// (DEC-091: a place's introduction, overheard names and rumours).
fn move_to(
    state: &mut State,
    content: &ContentBundle,
    location: crate::places::Location,
) -> Result<ActionResult, EngineError> {
    crate::places::validate_move(state, location)?;
    let kind = crate::places::kind_key(state, location);
    if let Some(p) = state.player.as_mut() {
        p.location = location;
    }
    crate::places::arrive(state, content, location);
    let mut res = ActionResult {
        outcome_key: "outcome-moved".into(),
        ..Default::default()
    };
    res.args.insert("kind".into(), kind.into());
    Ok(res)
}

/// Estimated annual cost of a draft or bill, for display.
pub fn estimate_cost(state: &State, content: &ContentBundle, p: &PolicyInstance) -> f64 {
    let Some(t) = content.policies.get(&p.template) else {
        return 0.0;
    };
    let residents = state
        .population
        .municipality_total(&state.world, MunicipalityId(p.jurisdiction))
        as f64;
    annual_cost(t, p, residents, state.economy.national.price_level)
}

/// The district of the seat a politician holds, if any.
pub fn ward_of(state: &State, politician: PoliticianId) -> Option<ConstituencyId> {
    state.politics.get(politician)?.role.constituency()
}
