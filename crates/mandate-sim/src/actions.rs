//! Player commands (the Commands phase, OVERVIEW §4.2). Validated against
//! Authority (constitution powers), slots, and preconditions, then applied
//! with immediate effects. Outcomes are seeded from the Player RNG stream.

use crate::commands::EngineError;
use crate::events::EventKind;
use crate::policy::{Clause, PolicyInstance, PolicyStatus, annual_cost};
use crate::politics::{PoliticianId, Role};
use crate::state::State;
use crate::world::{ConstituencyId, MunicipalityId, Tier};
use mandate_content::{Constituency, ContentBundle, Power, Selection, TierKey};
use mandate_core::math::{clamp, sigmoid};
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
    /// Door-knocking in a neighbourhood.
    Canvass {
        neighborhood: u32,
    },
    Fundraise,
    Rest,
    /// Buy local advertising for the campaign (costs campaign funds).
    Advertise,
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
    let Some(c) = content.constitutions.get(&state.meta.constitution) else {
        return Vec::new();
    };
    let Some(local) = c.tier(TierKey::Local) else {
        return Vec::new();
    };
    match me.role {
        Role::Councillor { .. } => local
            .bodies
            .iter()
            .filter(|b| {
                matches!(
                    b.selection,
                    Selection::Election {
                        constituency: Constituency::Ward,
                        ..
                    }
                )
            })
            .flat_map(|b| b.powers.iter().copied())
            .collect(),
        Role::Mayor { .. } => local
            .offices
            .iter()
            .filter(|o| {
                matches!(
                    o.selection,
                    Selection::Election {
                        constituency: Constituency::AtLarge,
                        ..
                    }
                )
            })
            .flat_map(|o| o.powers.iter().copied())
            .collect(),
        // Out of office: no institutional powers, only campaigning and networking.
        Role::Private { .. } => Vec::new(),
    }
}

/// Lobbying success odds shown to the player before acting.
pub fn lobby_odds(state: &State, target: PoliticianId, late_night: bool) -> f64 {
    let (Some(pl), Some(t)) = (state.player.as_ref(), state.politics.get(target)) else {
        return 0.0;
    };
    let Some(me) = state.politics.get(pl.politician) else {
        return 0.0;
    };
    let skill = me.negotiation as f64 * 0.6 + me.oratory as f64 * 0.4;
    let x = (skill - 0.5) * 3.0 + t.pragmatism as f64 * 1.5 - 0.6
        + t.opinion_of_player as f64 / 50.0
        + if late_night { 0.35 } else { 0.0 };
    clamp(sigmoid(x), 0.08, 0.92)
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

fn use_slot(state: &mut State, late_night: bool) -> Result<(), EngineError> {
    let pl = state.player.as_mut().ok_or_else(|| bad("no player"))?;
    if late_night {
        if pl.late_night_used {
            return Err(bad("late night already used this turn"));
        }
        pl.late_night_used = true;
        pl.tracks.health = (pl.tracks.health - 8.0).max(0.0);
    } else {
        if pl.slots_left() == 0 {
            return Err(bad("no slots left this turn"));
        }
        pl.slots_used += 1;
    }
    Ok(())
}

/// Chance a press statement turns into a gaffe (shown before acting).
pub fn press_gaffe_risk(state: &State) -> f64 {
    let oratory = state
        .player
        .as_ref()
        .and_then(|p| state.politics.get(p.politician))
        .map(|p| p.oratory as f64)
        .unwrap_or(0.5);
    clamp(0.05 + 0.3 * (1.0 - oratory), 0.05, 0.35)
}

/// Chance a town hall turns hostile in a neighbourhood: likelier where the
/// player and the town hall are unpopular (shown before acting).
pub fn town_hall_risk(state: &State, hood: u32) -> f64 {
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
    clamp(0.2 - 0.8 * ap - 0.4 * (gov - 0.5), 0.05, 0.6)
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
    match cmd {
        PlayerCommand::CastVote { policy, vote } => {
            use crate::politics::Vote;
            let p = local_policy(state, *policy)?;
            if p.status != PolicyStatus::UnderConsideration {
                return Err(bad("only bills before the council can be voted on"));
            }
            if p.sponsor == Some(pl.politician) {
                return Err(bad("sponsors vote for their own bills"));
            }
            let powers = player_powers(state, content);
            if !powers.contains(&Power::ApproveOrdinance) && !powers.contains(&Power::Veto) {
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
                .map(|t| t.signature)
                .unwrap_or([0.0; 8]);
            let line = party_line(state, *policy);
            let sign = match vote {
                Vote::Yes => 1.0,
                Vote::No => -1.0,
                Vote::Abstain => 0.0,
            };
            state.politics.declared_votes.insert(key, sign as i8);
            // Party: loyalty is noticed a little, rebellion a lot.
            if let Some(pls) = state.player.as_mut() {
                let d = if line == 0 || *vote == Vote::Abstain {
                    if *vote == Vote::Abstain { -1.0 } else { 0.0 }
                } else if (line as f64) == sign {
                    1.0
                } else {
                    -6.0
                };
                pls.tracks.standing = clamp(pls.tracks.standing as f64 + d, 0.0, 100.0) as f32;
            }
            // The ward: voters who care reward or punish the stand; dodging costs a little.
            for h in crate::policy::ward_hoods(&state.world, pl.ward) {
                for c in state.population.hoods[h].cohorts.iter_mut() {
                    let a = crate::politics::alignment(&sig, &c.attitudes);
                    let room = 1.0 - (c.player_approval as f64).abs();
                    let delta = if *vote == Vote::Abstain {
                        -0.01
                    } else {
                        0.06 * a * sign * (0.5 + awareness) * room
                    };
                    c.player_approval = clamp(c.player_approval as f64 + delta, -1.0, 1.0) as f32;
                }
            }
            // The sponsor: support earns goodwill and repays an owed favour.
            if let Some(sid) = sponsor
                && let Some(sp) = state.politics.get_mut(sid)
                && !sp.is_player
            {
                match vote {
                    Vote::Yes => {
                        sp.opinion_of_player =
                            clamp(sp.opinion_of_player as f64 + 5.0, -100.0, 100.0) as f32;
                        if sp.favors < 0 {
                            sp.favors += 1;
                            res.args.insert("repaid".into(), sp.full_name());
                        }
                    }
                    Vote::No => {
                        let hit = if sp.favors < 0 { 15.0 } else { 5.0 };
                        sp.opinion_of_player =
                            clamp(sp.opinion_of_player as f64 - hit, -100.0, 100.0) as f32;
                    }
                    Vote::Abstain => {}
                }
            }
            res.outcome_key = match vote {
                Vote::Yes => "outcome-vote-yes",
                Vote::No => "outcome-vote-no",
                Vote::Abstain => "outcome-vote-abstain",
            }
            .into();
        }
        PlayerCommand::File { filing } => {
            if let Some(p) = state.player.as_mut() {
                p.filing = Some(*filing);
            }
            res.outcome_key = match filing {
                crate::politics::elections::Filing::Council => "outcome-filed-council",
                crate::politics::elections::Filing::Mayor => "outcome-filed-mayor",
                crate::politics::elections::Filing::Retire => "outcome-filed-retire",
            }
            .into();
        }
        PlayerCommand::DraftPolicy { template, params } => {
            let t = content
                .policies
                .get(template)
                .ok_or_else(|| bad("unknown policy template"))?;
            if t.tier != TierKey::Local {
                return Err(bad(
                    "that policy is not available at your level of government",
                ));
            }
            if !player_powers(state, content).contains(&t.requires) {
                return Err(bad("your office cannot propose this"));
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
            res.created = Some(id.0);
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
                use_slot(state, false)?;
                // Amendments cost goodwill already built: halve existing sway.
                for (k, v) in state.politics.sway.iter_mut() {
                    if k.1 == *policy {
                        *v *= 0.5;
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
            use_slot(state, false)?;
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
                p.awareness = 0.15;
                p.cause = Some(ev);
            }
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
                    Action::Lobby { .. } | Action::TradeFavor { .. } | Action::Fundraise
                )
            {
                return Err(bad(
                    "only private meetings and fundraising happen late at night",
                ));
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
                    if matches!(action, Action::TradeFavor { .. }) && t.favors <= -MAX_FAVOR_DEBT {
                        return Err(bad("you already owe them too much"));
                    }
                    let odds = lobby_odds(state, target, *late_night);
                    let negotiation = state
                        .politics
                        .get(pl.politician)
                        .map(|x| x.negotiation)
                        .unwrap_or(0.5) as f64;
                    use_slot(state, *late_night)?;
                    let rng = state.rng.get(Stream::Player);
                    let trade = matches!(action, Action::TradeFavor { .. });
                    let success = trade || rng.chance(odds);
                    let gain = if trade {
                        0.35
                    } else if success {
                        rng.range_f64(0.2, 0.35) * (0.5 + negotiation)
                    } else {
                        0.04
                    };
                    *state
                        .politics
                        .sway
                        .entry((*politician, *policy))
                        .or_default() += gain as f32;
                    if let Some(t) = state.politics.get_mut(target) {
                        if trade {
                            t.favors -= 1;
                        }
                        t.opinion_of_player = clamp(
                            t.opinion_of_player as f64 + if success { 3.0 } else { -2.0 },
                            -100.0,
                            100.0,
                        ) as f32;
                    }
                    if *late_night && let Some(p) = state.player.as_mut() {
                        // Back-room deals carry legal exposure (DEC-006).
                        p.tracks.legal_jeopardy = (p.tracks.legal_jeopardy + 1.5).min(100.0);
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
                    let risk = town_hall_risk(state, *neighborhood);
                    use_slot(state, false)?;
                    let hostile = state.rng.get(Stream::Player).chance(risk);
                    let oratory = state
                        .politics
                        .get(pl.politician)
                        .map(|x| x.oratory)
                        .unwrap_or(0.5) as f64;
                    let sig = policy
                        .and_then(|id| state.policy.instances.get(id as usize))
                        .and_then(|p| content.policies.get(&p.template))
                        .map(|t| t.signature);
                    for c in state.population.hoods[*neighborhood as usize]
                        .cohorts
                        .iter_mut()
                    {
                        c.player_awareness =
                            clamp(c.player_awareness as f64 + 0.25 * (0.6 + oratory), 0.0, 1.0)
                                as f32;
                        let nudge = sig
                            .map(|s| crate::politics::alignment(&s, &c.attitudes) * 0.05)
                            .unwrap_or(0.02);
                        let room = 1.0 - (c.player_approval as f64).abs();
                        // A hostile crowd: people still learn your name, but not kindly.
                        let nudge = if hostile { -0.04 } else { nudge };
                        c.player_approval =
                            clamp(c.player_approval as f64 + nudge * room, -1.0, 1.0) as f32;
                    }
                    if let Some(id) = policy
                        && let Some(p) = state.policy.instances.get_mut(*id as usize)
                    {
                        p.awareness =
                            clamp(p.awareness as f64 + 0.12 * (0.5 + oratory), 0.0, 1.0) as f32;
                    }
                    let scale = (0.8 + 0.6 * oratory) * if hostile { 0.4 } else { 1.0 };
                    campaign_contact(state, content, Some(*neighborhood), scale);
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
                    let risk = press_gaffe_risk(state);
                    use_slot(state, false)?;
                    let gaffe = state.rng.get(Stream::Player).chance(risk);
                    let hoods: Vec<usize> = state.world.hoods_of(MunicipalityId(m)).collect();
                    for h in hoods {
                        for c in state.population.hoods[h].cohorts.iter_mut() {
                            c.player_awareness =
                                clamp(c.player_awareness as f64 + 0.06, 0.0, 1.0) as f32;
                            if gaffe {
                                c.player_approval = clamp(
                                    c.player_approval as f64 - 0.03 * c.player_awareness as f64,
                                    -1.0,
                                    1.0,
                                ) as f32;
                            }
                        }
                    }
                    if let Some(id) = policy
                        && let Some(p) = state.policy.instances.get_mut(*id as usize)
                    {
                        p.awareness = clamp(p.awareness as f64 + 0.15, 0.0, 1.0) as f32;
                    }
                    if !gaffe {
                        campaign_contact(state, content, None, 0.25);
                    }
                    res.outcome_key = if gaffe {
                        "outcome-press-gaffe"
                    } else {
                        "outcome-press"
                    }
                    .into();
                }
                Action::Canvass { neighborhood } => {
                    let h = state
                        .world
                        .neighborhoods
                        .get(*neighborhood as usize)
                        .ok_or_else(|| bad("no such neighbourhood"))?;
                    if h.municipality.0 != m {
                        return Err(bad("that neighbourhood is not in your town"));
                    }
                    use_slot(state, false)?;
                    for c in state.population.hoods[*neighborhood as usize]
                        .cohorts
                        .iter_mut()
                    {
                        // Contact mostly builds recognition; persuasion is small,
                        // saturating, and decays (DEC-071 §5).
                        let aw = c.player_awareness as f64;
                        c.player_awareness = clamp(aw + 0.3 * (1.0 - aw), 0.0, 1.0) as f32;
                        let room = 1.0 - (c.player_approval as f64).abs();
                        c.player_approval =
                            clamp(c.player_approval as f64 + 0.015 * room, -1.0, 1.0) as f32;
                    }
                    campaign_contact(state, content, Some(*neighborhood), 1.0);
                    // Door-knocking is the slow, tiring way to win (time and health).
                    if let Some(p) = state.player.as_mut() {
                        p.tracks.health = (p.tracks.health - 3.0).max(0.0);
                    }
                    res.outcome_key = "outcome-canvass".into();
                    res.args.insert(
                        "place".into(),
                        state.world.neighborhoods[*neighborhood as usize]
                            .name
                            .clone(),
                    );
                }
                Action::Fundraise => {
                    use_slot(state, *late_night)?;
                    let oratory = state
                        .politics
                        .get(pl.politician)
                        .map(|x| x.oratory)
                        .unwrap_or(0.5) as f64;
                    let raised =
                        state.rng.get(Stream::Player).range_f64(1_500.0, 6_000.0) * (0.5 + oratory);
                    let money = Money::from_units(raised as i64);
                    if let Some(p) = state.player.as_mut() {
                        p.money += money;
                        // The party likes a fundraiser; late-night money invites scrutiny.
                        p.tracks.standing = (p.tracks.standing + 0.5).min(100.0);
                        p.tracks.health = (p.tracks.health - 2.0).max(0.0);
                        if *late_night {
                            p.tracks.legal_jeopardy = (p.tracks.legal_jeopardy + 1.0).min(100.0);
                        }
                    }
                    // Donor ties: low-income voters who know you notice who you court.
                    let hoods: Vec<usize> = state.world.hoods_of(MunicipalityId(m)).collect();
                    for h in hoods {
                        for c in state.population.hoods[h]
                            .cohorts
                            .iter_mut()
                            .step_by(crate::population::CLASSES)
                        {
                            c.player_approval = clamp(
                                c.player_approval as f64 - 0.01 * c.player_awareness as f64,
                                -1.0,
                                1.0,
                            ) as f32;
                        }
                    }
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
                    res.outcome_key = "outcome-advertise".into();
                    res.args.insert("amount".into(), cost.units().to_string());
                }
                Action::Rest => {
                    use_slot(state, false)?;
                    if let Some(p) = state.player.as_mut() {
                        p.tracks.health = (p.tracks.health + 12.0).min(100.0);
                    }
                    res.outcome_key = "outcome-rest".into();
                }
            }
            let action_key = match action {
                Action::Lobby { .. } => "action-lobby",
                Action::TradeFavor { .. } => "action-trade-favor",
                Action::TownHall { .. } => "action-town-hall",
                Action::Press { .. } => "action-press",
                Action::Canvass { .. } => "action-canvass",
                Action::Fundraise => "action-fundraise",
                Action::Advertise => "action-advertise",
                Action::Rest => "action-rest",
            };
            if let Some(p) = state.player.as_mut() {
                p.log(
                    date,
                    action_key,
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

pub fn ward_of(state: &State, politician: PoliticianId) -> Option<ConstituencyId> {
    match state.politics.get(politician)?.role {
        Role::Councillor { ward, .. } => Some(ward),
        Role::Mayor { .. } | Role::Private { .. } => None,
    }
}
