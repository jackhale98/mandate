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
use crate::politics::{PoliticianId, alignment};
use crate::spine::{Causes, Link};
use crate::state::State;
use crate::world::{MunicipalityId, Tier};
use mandate_content::{AiAction, AiInput, ContentBundle, Curve, Power, TierKey};
use mandate_core::math::{clamp, exp};
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
    /// When that idea is an open policy-family proposal (DEC-092): the
    /// proposal, and the sponsor whose version they would table.
    pub best_family: Option<(crate::policy::families::FamilyInstanceId, Option<String>)>,
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
pub fn profile_of(content: &ContentBundle, p: &crate::politics::Politician) -> &'static str {
    let n = &content.npc;
    if p.pragmatism < n.ideologue_below as f32 {
        "ideologue"
    } else if p.ambition > n.populist_above as f32 {
        "populist"
    } else {
        "pragmatist"
    }
}

/// The leans of a bill's legislature (whip counts), as the constitution
/// names it (DEC-098).
fn legislature_leans(
    state: &State,
    content: &ContentBundle,
    t: &mandate_content::PolicyTemplate,
    inst: &crate::policy::PolicyInstance,
) -> Vec<(PoliticianId, f64)> {
    let Some(c) = content.constitutions.get(&state.meta.constitution) else {
        return Vec::new();
    };
    crate::policy::legislature_leans(
        &content.council,
        &content.politics.party_line,
        c,
        &state.world,
        &state.population,
        &state.politics,
        state.meta.seed,
        t,
        inst,
        crate::characters::town::broker_pull(
            content,
            &state.politics,
            &state.world,
            &state.population,
            t,
            inst,
        ),
    )
}

/// Which actors act this turn in a municipality: officeholders and candidates
/// (never the player, whose actions are their own commands).
fn actors(state: &State, m: u32) -> Vec<PoliticianId> {
    let at = crate::world::Jurisdiction::town(MunicipalityId(m));
    let mut out: Vec<PoliticianId> = crate::politics::officials_at(&state.politics, at);
    for race in state
        .politics
        .elections
        .races
        .iter()
        .filter(|r| r.seat.at() == at)
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
    let Some(c) = content.constitutions.get(&state.meta.constitution) else {
        return false;
    };
    // Their post's powers, as the constitution lists them (DEC-098).
    crate::politics::holds_power(&state.politics, c, p.id, power)
}

/// Per-town, per-turn memo of expensive public facts (vote counts and policy
/// support), shared by every NPC planning in that town this turn.
#[derive(Default)]
pub struct Memo {
    leans: BTreeMap<u32, (f64, f64)>,
    support: BTreeMap<(u32, String), f64>,
    /// Agenda bonus by (town, constituency or `u32::MAX` for the town, template).
    agenda: BTreeMap<(u32, u32, String), f64>,
    /// Situational templates on the table by (town, template) (DEC-077 §3).
    unlocked: BTreeMap<(u32, String), bool>,
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
    let n = &content.npc;
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
            x.race_closeness = clamp(1.0 - (own - best_other).abs() / n.close_margin, 0.0, 1.0);
        }
    }
    x.funds = clamp(
        me.funds / (params.campaign.ad_cost as f64 * n.funds_in_ads).max(1.0),
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
    // Floor time and budgets are finite: a busy statute book cools new bills.
    let active = local
        .iter()
        .filter(|p| matches!(p.status, PolicyStatus::Enacted | PolicyStatus::InEffect))
        .count();
    x.council_free = clamp(
        1.0 - pending.len() as f64 / n.busy_pending - active as f64 / n.busy_laws,
        0.0,
        1.0,
    );
    // At most one bill of one's own a year (councils have finite floor time).
    let today = state.calendar.date;
    x.no_own_bill = if local.iter().any(|p| {
        p.sponsor == Some(id)
            && (matches!(
                p.status,
                PolicyStatus::Draft | PolicyStatus::UnderConsideration
            ) || p
                .introduced_on
                .is_some_and(|d| d.days_until(today) < n.bill_interval_days))
    }) {
        0.0
    } else {
        1.0
    };
    if me.in_office() {
        let mut best: Option<(u32, f64, f64)> = None;
        for p in &pending {
            let Some(t) = crate::policy::resolve(content, &state.policy, p) else {
                continue;
            };
            let a = alignment(&t.signature_weights, &me.attitudes);
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
        // A goal against the player (DEC-032) makes the player's bill the
        // one to fight.
        if crate::characters::goals::opposes_player(&state.politics, me.character)
            && let Some(p) = pending.iter().find(|p| {
                p.sponsor
                    .and_then(|s| state.politics.get(s))
                    .is_some_and(|s| s.is_player)
            })
        {
            best = Some((p.id.0, 1.0, -1.0));
        }
        if let Some((bill, stake, dir)) = best {
            x.bill_stake = stake;
            x.focus_bill = Some((bill, dir));
            let counted = memo.leans.get(&bill).copied().or_else(|| {
                let inst = state.policy.instances.get(bill as usize)?;
                let t = crate::policy::resolve(content, &state.policy, inst)?;
                let leans = legislature_leans(state, content, &t, inst);
                let v = (
                    leans
                        .iter()
                        .map(|(_, l)| crate::policy::yes_probability(&content.council, *l))
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
                    yes - (needed + n.insurance_votes)
                } else {
                    (needed - n.insurance_votes) - yes
                };
                x.bill_closeness = clamp(1.0 - gap.max(0.0) / n.safe_gap, 0.0, 1.0)
                    * if gap < 0.0 { 1.0 } else { n.comfortable_weight };
            }
        }
        if holds_power(state, content, id, Power::ProposeOrdinance) {
            // A seat's district, else the whole town.
            let (key, hoods): (u32, Vec<usize>) = match me.role.constituency() {
                Some(ward) => (ward.0, crate::policy::ward_hoods(&state.world, ward)),
                None => (u32::MAX, state.world.hoods_of(MunicipalityId(m)).collect()),
            };
            let mut best_t: Option<(&str, f64, f64)> = None;
            let champion = crate::characters::goals::champion_bonus(&state.politics, me.character);
            let live = state
                .politics
                .characters
                .towns
                .get(m as usize)
                .and_then(|tc| tc.crisis());
            let crisis_severity = live.map(|c| c.severity as f64).unwrap_or(0.0);
            let crisis_policies: Vec<String> = live
                .and_then(|c| content.crises.iter().find(|d| d.id == c.kind))
                .map(|d| d.policies.clone())
                .unwrap_or_default();
            for t in content
                .policies
                .values()
                .filter(|t| t.tier == TierKey::Local && !t.retired)
            {
                // Situational policies only while the town's situation calls for them.
                if t.unlock.is_some()
                    && !*memo
                        .unlocked
                        .entry((m, t.id.clone()))
                        .or_insert_with(|| crate::policy::unlocked(state, content, t, m))
                {
                    continue;
                }
                if pending.iter().any(|p| p.template == t.id)
                    || local.iter().any(|p| {
                        p.template == t.id
                            && matches!(p.status, PolicyStatus::Enacted | PolicyStatus::InEffect)
                    })
                {
                    continue;
                }
                let own = alignment(&t.signature_weights, &me.attitudes);
                let public = *memo
                    .support
                    .entry((key, t.id.clone()))
                    .or_insert_with(|| crate::policy::public_support(t, &state.population, &hoods));
                let prag = me.pragmatism as f64;
                let agenda = *memo
                    .agenda
                    .entry((m, key, t.id.clone()))
                    .or_insert_with(|| agenda_bonus_in(state, content, m, &hoods, t));
                // The town's character shapes which idea comes first, not
                // how keen the NPC is to legislate at all.
                let score = (1.0 - prag) * own + prag * public;
                // A goal to champion a fix (DEC-032) ranks crisis answers higher.
                let champion = if champion > 0.0 && crisis_policies.contains(&t.id) {
                    champion * crisis_severity
                } else {
                    0.0
                };
                let ranked = score + agenda + champion;
                if best_t.is_none_or(|b| ranked > b.1) {
                    best_t = Some((t.id.as_str(), ranked, score));
                }
            }
            let best_f = family_idea(
                state,
                content,
                me,
                m,
                &hoods,
                &pending,
                &crisis_policies,
                champion,
                crisis_severity,
            );
            match (best_t, best_f) {
                (_, Some((proposal, sponsor, family, ranked, s)))
                    if best_t.is_none_or(|b| ranked > b.1) =>
                {
                    x.agenda = clamp(0.5 + s, 0.0, 1.0);
                    x.best_template = Some(family);
                    x.best_family = Some((proposal, sponsor));
                }
                (Some((t, _, s)), _) => {
                    x.agenda = clamp(0.5 + s, 0.0, 1.0);
                    x.best_template = Some(t.to_string());
                }
                _ => {}
            }
        }
    }
    x
}

/// The NPC's best open policy-family proposal (DEC-092 §2.10): one at
/// their town that their post has the family's power to propose, in the
/// version of the sponsor they like best (else the defaults), scored by the
/// same agenda as templates. (proposal, sponsor, family id, ranked, score).
#[allow(clippy::too_many_arguments)]
fn family_idea(
    state: &State,
    content: &ContentBundle,
    me: &crate::politics::Politician,
    m: u32,
    hoods: &[usize],
    pending: &[&PolicyInstance],
    crisis_policies: &[String],
    champion: f64,
    crisis_severity: f64,
) -> Option<(
    crate::policy::families::FamilyInstanceId,
    Option<String>,
    String,
    f64,
    f64,
)> {
    let fam = &state.policy.families;
    if fam.instances.is_empty() {
        return None;
    }
    let c = content.constitutions.get(&state.meta.constitution)?;
    let at = crate::world::Jurisdiction::town(MunicipalityId(m));
    let mut best = None;
    for inst in fam.open_at(at) {
        let Some(f) = content.family(&inst.family).filter(|f| !f.disabled) else {
            continue;
        };
        if !crate::politics::holds_power_at(&state.politics, c, me.id, f.requires, at) {
            continue;
        }
        // One bill per proposal at a time.
        if pending
            .iter()
            .any(|p| p.family.as_ref().is_some_and(|d| d.instance == inst.id))
        {
            continue;
        }
        // The sponsor they are closest to.
        let sponsor = inst
            .sponsors
            .iter()
            .map(|s| {
                (
                    crate::characters::opinion(&state.politics, me.character, s.person),
                    s,
                )
            })
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, s)| s.sponsor.clone());
        let Ok(p) = crate::policy::drafting::prepare(
            state,
            content,
            inst.id,
            &BTreeMap::new(),
            &BTreeMap::new(),
            sponsor.as_deref(),
        ) else {
            continue;
        };
        let choices: BTreeMap<String, String> = p.draft.choices.iter().cloned().collect();
        let Some(t) = crate::policy::resolve::compose(content, f, Some(inst), &choices) else {
            continue;
        };
        let own = alignment(&t.signature_weights, &me.attitudes);
        let public = crate::policy::public_support(&t, &state.population, hoods);
        let agenda = agenda_bonus_in(state, content, m, hoods, &t);
        let prag = me.pragmatism as f64;
        let score = (1.0 - prag) * own + prag * public;
        let champion = if champion > 0.0 && crisis_policies.contains(&f.id) {
            champion * crisis_severity
        } else {
            0.0
        };
        let ranked = score + agenda + champion;
        if best.as_ref().is_none_or(
            |b: &(
                crate::policy::families::FamilyInstanceId,
                Option<String>,
                String,
                f64,
                f64,
            )| ranked > b.3,
        ) {
            best = Some((inst.id, sponsor, f.id.clone(), ranked, score));
        }
    }
    best
}

/// Agendas follow the town's character (DEC-077 §1). When an NPC ranks its
/// policy ideas, a template that answers the live crisis gains
/// `npc.crisis_agenda` × severity; one that addresses
/// the town's concerns gains `npc.concern_agenda` × fit (its alignment with
/// each concern's attitudes, weighted by the concern's level). Public facts
/// only: the town's crisis and worries are common knowledge.
pub fn agenda_bonus(
    state: &State,
    content: &ContentBundle,
    m: u32,
    t: &mandate_content::PolicyTemplate,
) -> f64 {
    let all: Vec<usize> = state.world.hoods_of(MunicipalityId(m)).collect();
    agenda_bonus_in(state, content, m, &all, t)
}

/// `agenda_bonus` for an NPC whose post serves some neighbourhoods of town
/// `m` (a ward member's, DEC-092 N2): the concern fit reads those
/// neighbourhoods' political levels (`concerns::political_levels`); the
/// whole town reads the town's measured levels. The crisis is the town's.
pub fn agenda_bonus_in(
    state: &State,
    content: &ContentBundle,
    m: u32,
    hoods: &[usize],
    t: &mandate_content::PolicyTemplate,
) -> f64 {
    let n = &content.npc;
    let mut bonus = 0.0;
    if n.crisis_agenda != 0.0
        && let Some(c) = state
            .politics
            .characters
            .towns
            .get(m as usize)
            .and_then(|tc| tc.crisis())
        && content
            .crises
            .iter()
            .any(|d| d.id == c.kind && d.policies.contains(&t.id))
    {
        bonus += n.crisis_agenda * c.severity as f64;
    }
    if n.concern_agenda != 0.0 {
        // The measured concerns of the post's district: politics, not talk.
        let levels = if hoods.is_empty() {
            Vec::new()
        } else {
            crate::concerns::political_levels(&state.population, content, &state.world, hoods)
        };
        let (mut fit, mut w) = (0.0, 0.0);
        for (c, level) in content.concerns.iter().zip(&levels) {
            let level = *level;
            if level <= 0.0 {
                continue;
            }
            // `alignment` of the template with the concern's attitudes.
            let (mut dot, mut norm) = (0.0, 0.0);
            for (s, a) in t.signature_weights.iter().zip(&c.attitude_weights) {
                dot += s * (*a as f32) as f64;
                norm += s.abs();
            }
            let a = if norm > 0.0 {
                clamp(dot / norm * 2.0, -1.0, 1.0)
            } else {
                0.0
            };
            fit += level * a;
            w += level;
        }
        if w > 0.0 {
            bonus += n.concern_agenda * fit / w;
        }
    }
    bonus
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
        // Ads are bought with money, outside the slot loop (see `buy_ads`).
        AiAction::Advertise => false,
        AiAction::Lobby => x.focus_bill.is_some(),
        AiAction::ProposeBill => x.best_template.is_some() && x.no_own_bill > 0.5,
        AiAction::Fundraise | AiAction::Rest => true,
    }
}

/// The content key of an AI action's time cost (shared with the player).
pub fn cost_key(a: AiAction) -> &'static str {
    match a {
        AiAction::Canvass => "canvass",
        AiAction::TownHall => "town_hall",
        AiAction::Advertise => "advertise",
        AiAction::Fundraise => "fundraise",
        AiAction::Lobby => "lobby",
        AiAction::ProposeBill => "introduce",
        AiAction::Rest => "rest",
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
            .map(|p| profile_of(content, p))
            .unwrap_or("pragmatist");
        if full {
            buy_ads(state, content, id, profile, &mut memo);
        }
        // Inputs are read once per turn (the NPC plans its week), then updated
        // for the obvious consequences of its own actions.
        let mut x = inputs_memo(state, content, id, &mut memo);
        if !full && x.campaigning > 0.5 {
            continue;
        }
        // The same time budget and action costs as the player (DEC-030, DEC-073).
        let mut left = slots;
        for slot in 0..slots {
            if left == 0 {
                break;
            }
            let scored: Vec<(AiAction, f64)> = score(content, profile, &x)
                .into_iter()
                .filter(|(a, _)| content.tuning.action_cost(cost_key(*a)).slots <= left)
                .collect();
            let mut rng = mandate_core::rng::keyed(
                state.meta.seed,
                Stream::Npc,
                &[0xA1, date.0 as u64, id.0 as u64, slot as u64],
            );
            let Some(action) = choose(&scored, content.politics.ai.top_n as usize, &mut rng) else {
                break;
            };
            act(state, content, id, action, &x, &mut rng);
            // The same visible work counts toward NPC records.
            let g = &content.career.record_gain;
            let rec = match action {
                AiAction::ProposeBill => g.introduce as f32,
                AiAction::TownHall => g.town_hall as f32,
                AiAction::Canvass => g.canvass as f32,
                AiAction::Lobby => g.lobby as f32,
                _ => 0.0,
            };
            crate::politics::add_record(&mut state.politics, id, rec);
            left = left.saturating_sub(content.tuning.action_cost(cost_key(action)).slots.max(1));
            match action {
                AiAction::ProposeBill => {
                    x.no_own_bill = 0.0;
                    x.best_template = None;
                    x.best_family = None;
                }
                AiAction::Fundraise | AiAction::Advertise => {
                    let funds = state.politics.get(id).map(|p| p.funds).unwrap_or(0.0);
                    x.funds = clamp(
                        funds
                            / (content.politics.campaign.ad_cost as f64 * content.npc.funds_in_ads)
                                .max(1.0),
                        0.0,
                        1.0,
                    );
                }
                AiAction::Lobby => x.bill_closeness *= content.npc.lobby_closeness_keep,
                AiAction::Canvass | AiAction::TownHall => {
                    x.unknown *= content.npc.contact_unknown_keep
                }
                AiAction::Rest => {}
            }
        }
        // The late-night temptation (DEC-030, DEC-034): the same acts and
        // the same exposure as the player's. Towns other than the player's
        // take NPC turns every fourth turn, so their chance is scaled to match.
        tempt(state, content, id, if full { 1.0 } else { 4.0 });
    }
}

/// An officeholder may take a bribe or a donor's quiet deal, by personality
/// (integrity resists, ambition tempts). The act pays into their war chest
/// and leaves a hidden fact with a fate settled like the player's.
pub fn tempt(state: &mut State, content: &ContentBundle, id: PoliticianId, scale: f64) {
    if !state.media.active || state.player.is_none() {
        return;
    }
    let t = &content.media.temptation;
    if t.acts.is_empty() || t.chance <= 0.0 {
        return;
    }
    let Some(p) = state.politics.get(id) else {
        return;
    };
    if !p.in_office() || p.is_player {
        return;
    }
    let chance = t.chance
        * mandate_core::math::pow((1.0 - p.integrity as f64).max(0.0), t.integrity_power)
        * (t.ambition_base + p.ambition as f64)
        * scale;
    let mut rng = mandate_core::rng::keyed(
        state.meta.seed,
        Stream::Npc,
        &[0xB1BE, state.calendar.date.0 as u64, id.0 as u64],
    );
    if !rng.chance(chance) {
        return;
    }
    let act = &t.acts[rng.below(t.acts.len() as u64) as usize];
    let Some(def) = content.media.npc_acts.get(act).cloned() else {
        return;
    };
    let m = p.municipality() as usize;
    let briber = state
        .politics
        .characters
        .towns
        .get(m)
        .and_then(|tc| tc.broker)
        .filter(|b| {
            state
                .politics
                .characters
                .get(*b)
                .is_some_and(|c| c.politician != Some(id))
        });
    if let Some(p) = state.politics.get_mut(id) {
        p.funds += def.money.max(0.0);
    }
    let actors = briber
        .map(|b| crate::narrative::Actor::Person { id: b })
        .into_iter()
        .collect();
    let cause = crate::spine::actor(&state.politics, id);
    crate::media::hide(state, content, &def.hidden, id, actors, cause);
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
                let th = &content.actions.town_hall;
                let scale = if action == AiAction::TownHall {
                    th.contact_base + th.contact_oratory * oratory
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
            let [lo, hi] = content.actions.fundraise.raised;
            let raised = rng.range_f64(lo, hi) * (0.5 + oratory);
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
            let Some(t) = crate::policy::resolve(content, &state.policy, inst) else {
                return;
            };
            let leans = legislature_leans(state, content, &t, inst);
            // The most persuadable colleague not already on our side (and
            // not settled: the sponsor, or committed, as for the player).
            let target = leans
                .iter()
                .filter(|(p, _)| *p != id && state.politics.get(*p).is_some_and(|q| !q.is_player))
                .filter(|(p, _)| !crate::actions::lobbying::settled(state, *p, bill))
                .map(|(p, l)| {
                    let y = crate::policy::yes_probability(&content.council, *l);
                    let room = if dir > 0.0 { 1.0 - y } else { y };
                    (*p, room * (1.0 - (y - 0.5).abs()))
                })
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(p, _)| p);
            let Some(target) = target else { return };
            // The player's rules (DEC-030): the plain pitch, through the
            // one function the player's conversation uses.
            let Some(pitch) = crate::actions::lobbying::pitch(
                state,
                content,
                id,
                target,
                bill,
                crate::actions::lobbying::Argument::Plain,
                dir,
            ) else {
                return;
            };
            let (success, gain) = crate::actions::lobbying::roll(rng, &pitch);
            *state.politics.sway.entry((target.0, bill)).or_default() += (gain * dir) as f32;
            // The colleague remembers being worked on (DEC-031).
            if let (Some(h), Some(a)) = (
                state.politics.get(target).map(|p| p.character),
                state.politics.get(id).map(|p| p.character),
            ) {
                crate::characters::remember(
                    &mut state.politics,
                    content,
                    date,
                    h,
                    a,
                    if success {
                        crate::characters::MemoryKind::LobbiedMe
                    } else {
                        crate::characters::MemoryKind::PressedMe
                    },
                    Some(crate::characters::Subject::Policy { id: bill }),
                    None,
                );
            }
        }
        AiAction::ProposeBill => {
            if let Some((proposal, sponsor)) = x.best_family.clone() {
                propose_family(state, content, id, rng, proposal, sponsor.as_deref());
                return;
            }
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
            // Their own choice (the utility that picked it is not a cause
            // the engine names).
            let cause = crate::spine::actor(&state.politics, id);
            let ev = state.events.record(
                date,
                EventKind::PolicyIntroduced {
                    policy: pid.0,
                    municipality: m,
                    sponsor: id.0,
                },
                cause,
            );
            state.policy.instances.push(PolicyInstance {
                id: pid,
                template,
                tier: TierKey::Local,
                jurisdiction: m,
                sponsor: Some(id),
                params,
                // NPC programmes are time-limited, so towns' agendas turn over.
                clauses: vec![crate::policy::Clause::Sunset {
                    years: {
                        let [lo, hi] = content.npc.bill_sunset_years;
                        lo + rng.below((hi - lo) as u64 + 1) as u8
                    },
                }],
                status: PolicyStatus::UnderConsideration,
                stage: 0,
                next_stage_on: Some(date.add_days(days as u32)),
                votes: Vec::new(),
                introduced_on: Some(date),
                enacted_on: None,
                progress: 0.0,
                awareness: content.actions.introduce.awareness as f32,
                end_reason: None,
                cause: Some(ev),
                ledger: BTreeMap::new(),
                family: None,
                estimate: None,
            });
        }
        AiAction::Rest => {}
    }
}

/// An NPC tables a policy family's proposal (DEC-092): the version of the
/// sponsor they like best, time-limited like their other bills.
fn propose_family(
    state: &mut State,
    content: &ContentBundle,
    id: PoliticianId,
    rng: &mut mandate_core::SimRng,
    proposal: crate::policy::families::FamilyInstanceId,
    sponsor: Option<&str>,
) {
    let Ok(p) = crate::policy::drafting::prepare(
        state,
        content,
        proposal,
        &BTreeMap::new(),
        &BTreeMap::new(),
        sponsor,
    ) else {
        return;
    };
    let date = state.calendar.date;
    let m = p.at.raw();
    let days = content
        .constitutions
        .get(&state.meta.constitution)
        .and_then(|c| c.tier(p.at.tier()))
        .and_then(|tier| tier.procedure("ordinance"))
        .map(|p| p.deliberation_days)
        .unwrap_or(21);
    let sunset = crate::policy::Clause::Sunset {
        years: {
            let [lo, hi] = content.npc.bill_sunset_years;
            lo + rng.below((hi - lo) as u64 + 1) as u8
        },
    };
    let pid = crate::policy::drafting::make_bill(
        state,
        content,
        p,
        id,
        PolicyStatus::UnderConsideration,
        vec![sunset],
    );
    // Their own choice, answering the proposal on the table.
    let cause = Causes::from(crate::spine::actor(&state.politics, id))
        .and_event(crate::spine::proposal_of(state, pid), Link::Prompted);
    let ev = state.events.record(
        date,
        EventKind::PolicyIntroduced {
            policy: pid.0,
            municipality: m,
            sponsor: id.0,
        },
        cause,
    );
    if let Some(b) = state.policy.get_mut(pid) {
        b.next_stage_on = Some(date.add_days(days as u32));
        b.introduced_on = Some(date);
        b.awareness = content.actions.introduce.awareness as f32;
        b.cause = Some(ev);
    }
}

/// Money, not time: candidates with funds buy ads (up to the same per-turn
/// limit as the player) when the utility of advertising is high enough.
fn buy_ads(
    state: &mut State,
    content: &ContentBundle,
    id: PoliticianId,
    profile: &str,
    memo: &mut Memo,
) {
    let x = inputs_memo(state, content, id, memo);
    let Some((ri, ci)) = x.race else { return };
    let ai = &content.politics.ai;
    let Some(def) = ai.actions.iter().find(|a| a.action == AiAction::Advertise) else {
        return;
    };
    let mult = ai
        .profiles
        .iter()
        .find(|p| p.id == profile)
        .and_then(|p| p.mults.iter().find(|m| m.action == AiAction::Advertise))
        .map(|m| m.mult)
        .unwrap_or(1.0);
    let score = def.weight
        * mult
        * def
            .considerations
            .iter()
            .map(|c| curve(c.curve, x.get(c.input)))
            .product::<f64>();
    let cost = content.politics.campaign.ad_cost as f64;
    let np = &content.npc;
    let buys = if score > np.two_ads_above {
        2
    } else if score > np.one_ad_above {
        1
    } else {
        0
    };
    for n in 0..buys.min(crate::actions::MAX_ADS_PER_TURN) {
        if state
            .politics
            .get(id)
            .is_none_or(|p| p.funds < cost * np.ad_reserve)
        {
            break;
        }
        if let Some(p) = state.politics.get_mut(id) {
            p.funds -= cost;
        }
        if let Some(c) = state
            .politics
            .elections
            .races
            .get_mut(ri)
            .and_then(|r| r.candidates.get_mut(ci))
        {
            crate::politics::elections::advertise(c, &content.politics.campaign, n);
        }
    }
}

/// The knowledge term of lobbying odds (DEC-091 "ignorance costs
/// effectiveness, never fairness"; M6_ENGINE_CONTRACTS §1.6): a logit by
/// how well the actor knows the target as a person (`knowledge.effects.
/// lobby_logit`; `detailed` is knowing their goals). The player reads their
/// ledger (every subject `detailed` under the lab switch); NPCs keep no
/// ledger and use `known`, so the shared formula stays symmetric.
pub fn lobby_knowledge_logit(
    state: &State,
    content: &ContentBundle,
    actor: PoliticianId,
    target: PoliticianId,
) -> f64 {
    use crate::player::knowledge::{Level, Subject, level_of};
    let (Some(me), Some(t)) = (state.politics.get(actor), state.politics.get(target)) else {
        return 0.0;
    };
    let level = if me.is_player {
        level_of(
            state.player.as_ref(),
            &Subject::Person {
                person: t.character,
            },
        )
    } else {
        Level::Known
    };
    content.knowledge.effects.lobby_logit.get(level)
}

/// Lobbying odds for any actor (the player's formula, with party ties in place
/// of the player's personal standing for NPC-to-NPC deals). The odds shown to
/// the player and the roll both come from here (DEC-073), knowledge term
/// included.
pub fn lobby_odds_between(
    state: &State,
    content: &ContentBundle,
    actor: PoliticianId,
    target: PoliticianId,
    late_night: bool,
) -> f64 {
    let ties = crate::actions::lobbying::ties(state, content, actor, target);
    crate::actions::lobbying::odds_with(state, content, actor, target, ties, late_night)
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
