//! NPC political AI (DEC-030, DEC-071 §8).
//!
//! NPC politicians take turns under the player's rules: the same slots per turn,
//! the same actions, the same odds, and polls rather than the truth. Each slot
//! scores every action with content-defined considerations and response
//! curves, then picks by weighted chance among the best few (seeded, so runs
//! replay exactly). Decisions are traceable for the "Why did they do that?"
//! inspector.
//!
//! Like player commands, NPC turns run before the calendar advances (the
//! Commands/NpcActions phases), not inside a domain system.

use crate::events::EventKind;
use crate::policy::{PolicyInstance, PolicyStatus};
use crate::politics::elections::{self, Seat};
use crate::politics::{PoliticianId, Role, alignment};
use crate::state::State;
use crate::world::{MunicipalityId, Tier};
use mandate_content::{AiAction, AiInput, ContentBundle, Curve, Power, TierKey};
use mandate_core::math::{clamp, exp, sigmoid};
use mandate_core::{SimRng, Stream};
use std::collections::BTreeMap;

/// Inputs for one decision, all normalised to 0..1.
#[derive(Clone, Debug, Default)]
pub struct Inputs {
    pub campaigning: f64,
    pub election_near: f64,
    pub race_closeness: f64,
    pub unknown: f64,
    pub funds: f64,
    pub bill_stake: f64,
    pub bill_closeness: f64,
    pub agenda: f64,
    pub no_own_bill: f64,
    pub council_free: f64,
    /// The bill the NPC cares most about, and which way.
    pub focus_bill: Option<(u32, f64)>,
    /// The NPC's best policy idea.
    pub best_template: Option<String>,
    /// (race index, candidate index) if campaigning.
    pub race: Option<(usize, usize)>,
}

impl Inputs {
    fn get(&self, i: AiInput) -> f64 {
        match i {
            AiInput::Campaigning => self.campaigning,
            AiInput::ElectionNear => self.election_near,
            AiInput::RaceCloseness => self.race_closeness,
            AiInput::Unknown => self.unknown,
            AiInput::Funds => self.funds,
            AiInput::BillStake => self.bill_stake,
            AiInput::BillCloseness => self.bill_closeness,
            AiInput::Agenda => self.agenda,
            AiInput::NoOwnBill => self.no_own_bill,
            AiInput::CouncilFree => self.council_free,
            AiInput::Constant => 1.0,
        }
    }
}

pub fn curve(c: Curve, x: f64) -> f64 {
    let x = clamp(x, 0.0, 1.0);
    match c {
        Curve::Linear { m, b } => clamp(m * x + b, 0.0, 1.0),
        Curve::Logistic { k, x0 } => 1.0 / (1.0 + exp(-k * (x - x0))),
        Curve::Step { x: t, lo, hi } => {
            if x >= t {
                hi
            } else {
                lo
            }
        }
    }
}

/// The strategic profile implied by a politician's traits.
pub fn profile_of(p: &crate::politics::Politician) -> &'static str {
    if p.pragmatism < 0.4 {
        "ideologue"
    } else if p.ambition > 0.75 {
        "populist"
    } else {
        "pragmatist"
    }
}

/// Which actors act this turn in a municipality: officeholders and candidates
/// (never the player, whose actions are their own commands).
fn actors(state: &State, m: u32) -> Vec<PoliticianId> {
    let mut out: Vec<PoliticianId> = Vec::new();
    if let Some(c) = state.politics.councils.get(m as usize) {
        out.extend(c.members.iter().copied());
        out.extend(c.mayor);
    }
    for race in state
        .politics
        .elections
        .races
        .iter()
        .filter(|r| r.seat.municipality() == m)
    {
        for c in &race.candidates {
            if !out.contains(&c.politician) {
                out.push(c.politician);
            }
        }
    }
    out.retain(|id| state.politics.get(*id).is_some_and(|p| !p.is_player));
    out
}

fn holds_power(state: &State, content: &ContentBundle, id: PoliticianId, power: Power) -> bool {
    let Some(p) = state.politics.get(id) else {
        return false;
    };
    let Some(local) = content
        .constitutions
        .get(&state.meta.constitution)
        .and_then(|c| c.tier(TierKey::Local))
    else {
        return false;
    };
    match p.role {
        Role::Councillor { .. } => local.bodies.iter().any(|b| b.powers.contains(&power)),
        Role::Mayor { .. } => local.offices.iter().any(|o| o.powers.contains(&power)),
        Role::Private { .. } => false,
    }
}

/// Per-town, per-turn memo of expensive public facts (vote counts and policy
/// support), shared by every NPC planning in that town this turn.
#[derive(Default)]
pub struct Memo {
    leans: BTreeMap<u32, (f64, f64)>,
    support: BTreeMap<(u32, String), f64>,
}

/// Compute an NPC's decision inputs (from polls and public knowledge only).
pub fn inputs(state: &State, content: &ContentBundle, id: PoliticianId) -> Inputs {
    inputs_memo(state, content, id, &mut Memo::default())
}

fn inputs_memo(
    state: &State,
    content: &ContentBundle,
    id: PoliticianId,
    memo: &mut Memo,
) -> Inputs {
    let mut x = Inputs::default();
    let Some(me) = state.politics.get(id) else {
        return x;
    };
    let m = me.municipality();
    let params = &content.politics;
    // Campaign.
    for (ri, race) in state.politics.elections.races.iter().enumerate() {
        if let Some(ci) = race.candidates.iter().position(|c| c.politician == id) {
            x.race = Some((ri, ci));
            x.campaigning = 1.0;
            let total = race.opened.days_until(race.election_day).max(1) as f64;
            let left = state.calendar.date.days_until(race.election_day) as f64;
            x.election_near = clamp(1.0 - left / total, 0.0, 1.0);
            let c = &race.candidates[ci];
            x.unknown = 1.0
                - c.awareness.iter().map(|a| *a as f64).sum::<f64>()
                    / c.awareness.len().max(1) as f64;
            // Polls when published; otherwise party strength (never the model truth).
            let shares: Vec<f64> = match elections::poll_average(&state.politics.elections, ri) {
                Some(p) => p.iter().map(|v| *v as f64).collect(),
                None => party_guess(state, race),
            };
            let own = shares.get(ci).copied().unwrap_or(0.0);
            let best_other = shares
                .iter()
                .enumerate()
                .filter(|(k, _)| *k != ci)
                .map(|(_, v)| *v)
                .fold(0.0, f64::max);
            x.race_closeness = clamp(1.0 - (own - best_other).abs() / 0.4, 0.0, 1.0);
        }
    }
    x.funds = clamp(
        me.funds / (params.campaign.ad_cost as f64 * 4.0).max(1.0),
        0.0,
        1.0,
    );
    // Legislative agenda (one scan of the town's policies).
    let local: Vec<&PolicyInstance> = state.policy.in_municipality(m).collect();
    let pending: Vec<&PolicyInstance> = local
        .iter()
        .copied()
        .filter(|p| p.status == PolicyStatus::UnderConsideration)
        .collect();
    x.council_free = clamp(1.0 - pending.len() as f64 / 3.0, 0.0, 1.0);
    // At most one bill of one's own a year (councils have finite floor time).
    let today = state.calendar.date;
    x.no_own_bill = if local.iter().any(|p| {
        p.sponsor == Some(id)
            && (matches!(
                p.status,
                PolicyStatus::Draft | PolicyStatus::UnderConsideration
            ) || p.introduced_on.is_some_and(|d| d.days_until(today) < 365))
    }) {
        0.0
    } else {
        1.0
    };
    if me.in_office() {
        let mut best: Option<(u32, f64, f64)> = None;
        for p in &pending {
            let Some(t) = content.policies.get(&p.template) else {
                continue;
            };
            let a = alignment(&t.signature, &me.attitudes);
            let stake = if p.sponsor == Some(id) { 1.0 } else { a.abs() };
            if best.is_none_or(|b| stake > b.1) {
                best = Some((
                    p.id.0,
                    stake,
                    if p.sponsor == Some(id) {
                        1.0
                    } else {
                        a.signum()
                    },
                ));
            }
        }
        if let Some((bill, stake, dir)) = best {
            x.bill_stake = stake;
            x.focus_bill = Some((bill, dir));
            let counted = memo.leans.get(&bill).copied().or_else(|| {
                let inst = state.policy.instances.get(bill as usize)?;
                let t = content.policies.get(&inst.template)?;
                let leans = crate::policy::council_leans(
                    &state.world,
                    &state.population,
                    &state.politics,
                    state.meta.seed,
                    t,
                    inst,
                );
                let v = (
                    leans
                        .iter()
                        .map(|(_, l)| crate::policy::yes_probability(*l))
                        .sum(),
                    leans.len().max(1) as f64,
                );
                memo.leans.insert(bill, v);
                Some(v)
            });
            if let Some((yes, seats)) = counted {
                let needed = (seats / 2.0).floor() + 1.0;
                // Coalition-builders want insurance above the threshold (DEC-071 §7).
                let gap = if dir > 0.0 {
                    yes - (needed + 1.0)
                } else {
                    (needed - 1.0) - yes
                };
                x.bill_closeness =
                    clamp(1.0 - gap.max(0.0) / 2.5, 0.0, 1.0) * if gap < 0.0 { 1.0 } else { 0.6 };
            }
        }
        if holds_power(state, content, id, Power::ProposeOrdinance) {
            let (key, hoods): (u32, Vec<usize>) = match me.role {
                Role::Councillor { ward, .. } => {
                    (ward.0, crate::policy::ward_hoods(&state.world, ward))
                }
                _ => (u32::MAX, state.world.hoods_of(MunicipalityId(m)).collect()),
            };
            let mut best_t: Option<(&str, f64)> = None;
            for t in content
                .policies
                .values()
                .filter(|t| t.tier == TierKey::Local)
            {
                if pending.iter().any(|p| p.template == t.id)
                    || local.iter().any(|p| {
                        p.template == t.id
                            && matches!(p.status, PolicyStatus::Enacted | PolicyStatus::InEffect)
                    })
                {
                    continue;
                }
                let own = alignment(&t.signature, &me.attitudes);
                let public = *memo
                    .support
                    .entry((key, t.id.clone()))
                    .or_insert_with(|| crate::policy::public_support(t, &state.population, &hoods));
                let prag = me.pragmatism as f64;
                let score = (1.0 - prag) * own + prag * public;
                if best_t.is_none_or(|b| score > b.1) {
                    best_t = Some((t.id.as_str(), score));
                }
            }
            if let Some((t, s)) = best_t {
                x.agenda = clamp(0.5 + s, 0.0, 1.0);
                x.best_template = Some(t.to_string());
            }
        }
    }
    x
}

/// Party-strength shares for a race's candidates (the NPC's prior before polls).
fn party_guess(state: &State, race: &elections::Race) -> Vec<f64> {
    let mut v: Vec<f64> = race
        .candidates
        .iter()
        .map(|c| {
            let mut acc = 0.0;
            let mut n = 0.0;
            for h in &race.hoods {
                for co in &state.population.hoods[*h as usize].cohorts {
                    let w = co.count as f64;
                    acc += c
                        .party
                        .and_then(|p| co.party_id.get(p.index()))
                        .copied()
                        .unwrap_or(0.0) as f64
                        * w;
                    n += w;
                }
            }
            acc / n.max(1.0) + 0.05
        })
        .collect();
    let s: f64 = v.iter().sum::<f64>().max(1e-9);
    for x in v.iter_mut() {
        *x /= s;
    }
    v
}

/// Score every action for an NPC. Returns (action, score) sorted best first.
pub fn score(content: &ContentBundle, profile: &str, x: &Inputs) -> Vec<(AiAction, f64)> {
    let ai = &content.politics.ai;
    let prof = ai.profiles.iter().find(|p| p.id == profile);
    let mut out: Vec<(AiAction, f64)> = ai
        .actions
        .iter()
        .map(|a| {
            let mult = prof
                .and_then(|p| p.mults.iter().find(|m| m.action == a.action))
                .map(|m| m.mult)
                .unwrap_or(1.0);
            let s = a
                .considerations
                .iter()
                .map(|c| curve(c.curve, x.get(c.input)))
                .product::<f64>();
            (a.action, a.weight * mult * s)
        })
        .filter(|(a, _)| feasible(*a, x))
        .collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

fn feasible(a: AiAction, x: &Inputs) -> bool {
    match a {
        AiAction::Canvass | AiAction::TownHall => x.race.is_some(),
        AiAction::Advertise => x.race.is_some() && x.funds > 0.25,
        AiAction::Lobby => x.focus_bill.is_some(),
        AiAction::ProposeBill => x.best_template.is_some() && x.no_own_bill > 0.5,
        AiAction::Fundraise | AiAction::Rest => true,
    }
}

/// Weighted chance among the best `top_n` (seeded).
fn choose(scored: &[(AiAction, f64)], top_n: usize, rng: &mut SimRng) -> Option<AiAction> {
    let top: Vec<&(AiAction, f64)> = scored
        .iter()
        .take(top_n.max(1))
        .filter(|(_, s)| *s > 1e-6)
        .collect();
    let weights: Vec<f64> = top.iter().map(|(_, s)| *s).collect();
    rng.weighted_index(&weights).map(|i| top[i].0)
}

/// NPC turns for one municipality (`full`: campaign actions too; otherwise
/// only legislative work, as in the burn-in where campaigns are abstracted).
pub fn take_turn(state: &mut State, content: &ContentBundle, m: u32, full: bool) {
    let slots = content.tuning.standard_slots;
    let date = state.calendar.date;
    let mut memo = Memo::default();
    for id in actors(state, m) {
        let profile = state
            .politics
            .get(id)
            .map(profile_of)
            .unwrap_or("pragmatist");
        // Inputs are read once per turn (the NPC plans its week), then updated
        // for the obvious consequences of its own actions.
        let mut x = inputs_memo(state, content, id, &mut memo);
        if !full && x.campaigning > 0.5 {
            continue;
        }
        for slot in 0..slots {
            let scored = score(content, profile, &x);
            let mut rng = mandate_core::rng::keyed(
                state.meta.seed,
                Stream::Npc,
                &[0xA1, date.0 as u64, id.0 as u64, slot as u64],
            );
            let Some(action) = choose(&scored, content.politics.ai.top_n as usize, &mut rng) else {
                break;
            };
            act(state, content, id, action, &x, &mut rng);
            match action {
                AiAction::ProposeBill => {
                    x.no_own_bill = 0.0;
                    x.best_template = None;
                }
                AiAction::Fundraise | AiAction::Advertise => {
                    let funds = state.politics.get(id).map(|p| p.funds).unwrap_or(0.0);
                    x.funds = clamp(
                        funds / (content.politics.campaign.ad_cost as f64 * 4.0).max(1.0),
                        0.0,
                        1.0,
                    );
                }
                AiAction::Lobby => x.bill_closeness *= 0.7,
                AiAction::Canvass | AiAction::TownHall => x.unknown *= 0.9,
                AiAction::Rest => {}
            }
        }
    }
}

fn act(
    state: &mut State,
    content: &ContentBundle,
    id: PoliticianId,
    action: AiAction,
    x: &Inputs,
    rng: &mut SimRng,
) {
    let params = &content.politics.campaign;
    let date = state.calendar.date;
    let oratory = state
        .politics
        .get(id)
        .map(|p| p.oratory as f64)
        .unwrap_or(0.5);
    match action {
        AiAction::Canvass | AiAction::TownHall => {
            let Some((ri, ci)) = x.race else { return };
            let pop = &state.population;
            let Some(race) = state.politics.elections.races.get_mut(ri) else {
                return;
            };
            let c = &mut race.candidates[ci];
            let best = race
                .hoods
                .iter()
                .enumerate()
                .map(|(i, h)| {
                    let adults = pop.hoods[*h as usize].adults() as f64;
                    (
                        i,
                        adults * (1.05 - c.awareness[i] as f64) / (1.0 + c.contacts[i] as f64),
                    )
                })
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(i, _)| i);
            if let Some(i) = best {
                let scale = if action == AiAction::TownHall {
                    0.8 + 0.6 * oratory
                } else {
                    1.0
                };
                elections::contact(c, i, params, scale);
            }
        }
        AiAction::Advertise => {
            let Some((ri, ci)) = x.race else { return };
            let cost = params.ad_cost as f64;
            if state.politics.get(id).is_none_or(|p| p.funds < cost) {
                return;
            }
            if let Some(p) = state.politics.get_mut(id) {
                p.funds -= cost;
            }
            let per = params.awareness_per_ad / params.awareness_per_canvass.max(1e-6);
            if let Some(race) = state.politics.elections.races.get_mut(ri) {
                let c = &mut race.candidates[ci];
                c.spent += cost;
                for i in 0..c.awareness.len() {
                    elections::contact(c, i, params, per);
                }
            }
        }
        AiAction::Fundraise => {
            let raised = rng.range_f64(1_500.0, 6_000.0) * (0.5 + oratory);
            if let Some(p) = state.politics.get_mut(id) {
                p.funds += raised;
            }
        }
        AiAction::Lobby => {
            let Some((bill, dir)) = x.focus_bill else {
                return;
            };
            let Some(inst) = state.policy.instances.get(bill as usize) else {
                return;
            };
            let Some(t) = content.policies.get(&inst.template) else {
                return;
            };
            let leans = crate::policy::council_leans(
                &state.world,
                &state.population,
                &state.politics,
                state.meta.seed,
                t,
                inst,
            );
            // The most persuadable colleague not already on our side.
            let target = leans
                .iter()
                .filter(|(p, _)| *p != id && state.politics.get(*p).is_some_and(|q| !q.is_player))
                .map(|(p, l)| {
                    let y = crate::policy::yes_probability(*l);
                    let room = if dir > 0.0 { 1.0 - y } else { y };
                    (*p, room * (1.0 - (y - 0.5).abs()))
                })
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(p, _)| p);
            let Some(target) = target else { return };
            let odds = lobby_odds_between(state, id, target, false);
            let negotiation = state
                .politics
                .get(id)
                .map(|p| p.negotiation as f64)
                .unwrap_or(0.5);
            let success = rng.chance(odds);
            let gain = if success {
                rng.range_f64(0.2, 0.35) * (0.5 + negotiation)
            } else {
                0.04
            };
            *state.politics.sway.entry((target.0, bill)).or_default() += (gain * dir) as f32;
        }
        AiAction::ProposeBill => {
            let Some(template) = x.best_template.clone() else {
                return;
            };
            let Some(t) = content.policies.get(&template) else {
                return;
            };
            let m = state
                .politics
                .get(id)
                .map(|p| p.municipality())
                .unwrap_or(0);
            let days = content
                .constitutions
                .get(&state.meta.constitution)
                .and_then(|c| c.tier(TierKey::Local))
                .and_then(|tier| tier.procedure("ordinance"))
                .map(|p| p.deliberation_days)
                .unwrap_or(21);
            let params: BTreeMap<String, f64> =
                t.params.iter().map(|p| (p.id.clone(), p.default)).collect();
            let pid: crate::policy::PolicyInstanceId = state.policy.ids.alloc();
            let ev = state.events.record(
                date,
                EventKind::PolicyIntroduced {
                    policy: pid.0,
                    municipality: m,
                    sponsor: id.0,
                },
                vec![],
            );
            state.policy.instances.push(PolicyInstance {
                id: pid,
                template,
                tier: TierKey::Local,
                jurisdiction: m,
                sponsor: Some(id),
                params,
                clauses: Vec::new(),
                status: PolicyStatus::UnderConsideration,
                stage: 0,
                next_stage_on: Some(date.add_days(days as u32)),
                votes: Vec::new(),
                introduced_on: Some(date),
                enacted_on: None,
                progress: 0.0,
                awareness: 0.15,
                end_reason: None,
                cause: Some(ev),
                ledger: BTreeMap::new(),
            });
        }
        AiAction::Rest => {}
    }
}

/// Lobbying odds for any actor (the player's formula, with party ties in place
/// of the player's personal standing for NPC-to-NPC deals).
pub fn lobby_odds_between(
    state: &State,
    actor: PoliticianId,
    target: PoliticianId,
    late_night: bool,
) -> f64 {
    let (Some(me), Some(t)) = (state.politics.get(actor), state.politics.get(target)) else {
        return 0.0;
    };
    let skill = me.negotiation as f64 * 0.6 + me.oratory as f64 * 0.4;
    let ties = if me.is_player {
        t.opinion_of_player as f64 / 50.0
    } else if me.party.is_some() && me.party == t.party {
        0.3
    } else {
        -0.1
    };
    let x = (skill - 0.5) * 3.0 + t.pragmatism as f64 * 1.5 - 0.6
        + ties
        + if late_night { 0.35 } else { 0.0 };
    clamp(sigmoid(x), 0.08, 0.92)
}

/// Every municipality's NPCs (burn-in and abstracted towns).
pub fn take_turn_all(state: &mut State, content: &ContentBundle, full: bool) {
    let n = state.world.municipalities.len() as u32;
    for m in 0..n {
        take_turn(state, content, m, full);
    }
}

/// Wards in a municipality (for tests and tools).
pub fn ward_count(state: &State, m: u32) -> usize {
    state
        .world
        .constituencies
        .iter()
        .filter(|c| c.tier == Tier::Local && c.jurisdiction == m)
        .count()
}

/// The seat a politician is contesting, if any.
pub fn contesting(state: &State, id: PoliticianId) -> Option<Seat> {
    state
        .politics
        .elections
        .races
        .iter()
        .find(|r| r.candidates.iter().any(|c| c.politician == id))
        .map(|r| r.seat)
}
