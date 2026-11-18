//! Policy lifecycle (DEC-024 rev. 2): Draft → Under consideration → Enacted →
//! In effect → Ended. Procedures come from the constitution (DEC-017);
//! effects come only from typed mechanisms read by the domain models
//! (REQ-011).

use crate::events::{EventId, EventKind};
use crate::politics::{PoliticianId, PoliticsState, Role, alignment, mean_attitudes};
use crate::population::PopulationState;
use crate::scheduler::{Outs, Reads};
use crate::world::{ConstituencyId, MunicipalityId, World};
use mandate_content::{
    BudgetCategory, ClassKey, ContentBundle, CouncilParams, DecisionRule, HazardKey, MechanismDef,
    PolicyTemplate, StageDef, TierKey,
};
use mandate_core::math::{clamp, sigmoid};
use mandate_core::{IdAllocator, SimDate, Stream, define_id};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

define_id!(
    /// A policy instance (a bill or law).
    PolicyInstanceId
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyStatus {
    Draft,
    UnderConsideration,
    Enacted,
    InEffect,
    Ended,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndReason {
    Defeated,
    Vetoed,
    Withdrawn,
    Repealed,
    Expired,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Clause {
    /// A local project in a ward, bought with money for a member's vote.
    Earmark { ward: ConstituencyId },
    /// The law expires after this many years unless renewed.
    Sunset { years: u8 },
    /// Limit to one neighbourhood first (20% of cost and effect).
    Pilot { neighborhood: u32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoteRecord {
    pub stage: u8,
    pub date: SimDate,
    pub yes: u32,
    pub no: u32,
    pub passed: bool,
    /// Signature stages record a veto as `passed = false` before any override.
    pub veto: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolicyInstance {
    pub id: PolicyInstanceId,
    pub template: String,
    pub tier: TierKey,
    /// Municipality index for local policies.
    pub jurisdiction: u32,
    pub sponsor: Option<PoliticianId>,
    pub params: BTreeMap<String, f64>,
    pub clauses: Vec<Clause>,
    pub status: PolicyStatus,
    pub stage: u8,
    pub next_stage_on: Option<SimDate>,
    pub votes: Vec<VoteRecord>,
    pub introduced_on: Option<SimDate>,
    pub enacted_on: Option<SimDate>,
    /// 0..1 implementation progress; effectiveness ramps with it.
    pub progress: f32,
    /// Public awareness in the jurisdiction (0..1).
    pub awareness: f32,
    pub end_reason: Option<EndReason>,
    pub cause: Option<EventId>,
    /// Direct contributions recorded for "Why?" (REQ-011 attribution).
    pub ledger: BTreeMap<String, f64>,
}

impl PolicyInstance {
    pub fn param(&self, id: &str) -> f64 {
        self.params.get(id).copied().unwrap_or(0.0)
    }
    pub fn pilot_scale(&self) -> f64 {
        if self
            .clauses
            .iter()
            .any(|c| matches!(c, Clause::Pilot { .. }))
        {
            0.2
        } else {
            1.0
        }
    }
    pub fn active(&self) -> bool {
        matches!(self.status, PolicyStatus::Enacted | PolicyStatus::InEffect)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Agency {
    /// Administrative competence (0..1): speed and quality of implementation.
    pub competence: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PolicyState {
    pub ids: IdAllocator,
    pub instances: Vec<PolicyInstance>,
    /// One municipal administration per municipality.
    pub agencies: Vec<Agency>,
}

impl PolicyState {
    pub fn get(&self, id: PolicyInstanceId) -> Option<&PolicyInstance> {
        self.instances.get(id.index())
    }
    pub fn get_mut(&mut self, id: PolicyInstanceId) -> Option<&mut PolicyInstance> {
        self.instances.get_mut(id.index())
    }
    pub fn in_municipality(&self, m: u32) -> impl Iterator<Item = &PolicyInstance> {
        self.instances
            .iter()
            .filter(move |p| p.tier == TierKey::Local && p.jurisdiction == m)
    }
}

pub fn init(world: &World, rng: &mut mandate_core::SimRng) -> PolicyState {
    PolicyState {
        ids: IdAllocator::new(),
        instances: Vec::new(),
        agencies: world
            .municipalities
            .iter()
            .map(|_| Agency {
                competence: rng.range_f64(0.45, 0.9) as f32,
            })
            .collect(),
    }
}

// --- Costs and mechanism aggregation ----------------------------------------------------

pub fn category_index(c: BudgetCategory) -> usize {
    c as usize
}

/// Annual cost (nominal) of an instance for a jurisdiction of `residents`.
pub fn annual_cost(
    t: &PolicyTemplate,
    p: &PolicyInstance,
    residents: f64,
    price_level: f64,
) -> f64 {
    let c = &t.cost;
    let units = c.param.as_ref().map(|k| p.param(k)).unwrap_or(0.0);
    let base =
        c.per_capita * residents + c.per_capita_per_unit * units * residents + c.per_unit * units;
    let earmarks = p
        .clauses
        .iter()
        .filter(|x| matches!(x, Clause::Earmark { .. }))
        .count() as f64
        * 4.0
        * residents;
    (base * p.pilot_scale() + earmarks) * price_level
}

/// Aggregate effect of active policies on one municipality.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PolicyMods {
    /// Service index gain by budget category.
    pub services: [f64; 9],
    /// Annual transfer per adult by class (price level 1).
    pub transfer: [f64; 3],
    pub housing_units_per_year: f64,
    pub starts_multiplier: f64,
    /// Wage subsidy share and job elasticity by sector, in registry order
    /// (empty when no policy applies).
    pub subsidy: Vec<f64>,
    pub subsidy_elasticity: Vec<f64>,
    /// Wage floor uplift for low-paid work (share).
    pub wage_floor: f64,
    pub flood_protection: f64,
    pub storm_protection: f64,
    pub wildfire_protection: f64,
    /// Change to the local property tax rate (share of value per year).
    pub property_tax_delta: f64,
    /// Disruption from programmes under construction (DEC-073 side effects).
    pub disruption: f64,
    /// Change in labour-force participation by class (share).
    pub labour_supply: [f64; 3],
}

/// A side effect's size for an instance: `per_unit` × its parameter (or 1).
pub fn side_effect_amount(fx: &mandate_content::SideEffectDef, p: &PolicyInstance) -> f64 {
    use mandate_content::SideEffectDef as S;
    let per_unit = match fx {
        S::Disruption { per_unit, .. }
        | S::Grievance { per_unit, .. }
        | S::LabourSupply { per_unit, .. } => *per_unit,
    };
    per_unit * fx.param().map(|k| p.param(k)).unwrap_or(1.0)
}

pub fn class_index(c: ClassKey) -> usize {
    match c {
        ClassKey::Low => 0,
        ClassKey::Middle => 1,
        ClassKey::High => 2,
    }
}

/// `funded(id)` returns the funded share (0..1) from finance.
pub fn mods_for(
    st: &PolicyState,
    content: &ContentBundle,
    m: u32,
    funded: &dyn Fn(u32) -> f64,
) -> PolicyMods {
    let mut out = PolicyMods {
        starts_multiplier: 1.0,
        subsidy: vec![0.0; content.sectors.len()],
        subsidy_elasticity: vec![0.0; content.sectors.len()],
        ..Default::default()
    };
    for p in st
        .in_municipality(m)
        .filter(|p| p.status == PolicyStatus::InEffect)
    {
        let Some(t) = content.policies.get(&p.template) else {
            continue;
        };
        let k = p.progress as f64 * p.pilot_scale() * funded(p.id.0);
        // Side effects (DEC-073): disruption while building, labour supply once running.
        let building = (1.0 - p.progress as f64).max(0.0) * p.pilot_scale() * funded(p.id.0);
        for fx in &t.side_effects {
            let a = side_effect_amount(fx, p);
            match fx {
                mandate_content::SideEffectDef::Disruption { .. } => out.disruption += a * building,
                mandate_content::SideEffectDef::LabourSupply { class, .. } => {
                    out.labour_supply[class_index(*class)] += a * k
                }
                mandate_content::SideEffectDef::Grievance { .. } => {}
            }
        }
        for mech in &t.mechanisms {
            match mech {
                MechanismDef::Service {
                    category,
                    param,
                    per_unit,
                } => out.services[category_index(*category)] += per_unit * p.param(param) * k,
                MechanismDef::Transfer { class, param } => {
                    let i = match class {
                        ClassKey::Low => 0,
                        ClassKey::Middle => 1,
                        ClassKey::High => 2,
                    };
                    out.transfer[i] += p.param(param) * k;
                }
                MechanismDef::HousingSupply { param } => {
                    out.housing_units_per_year += p.param(param) * k
                }
                MechanismDef::Zoning { param, per_unit } => {
                    out.starts_multiplier *= 1.0 + per_unit * p.param(param) * k
                }
                MechanismDef::SectorSubsidy {
                    sector,
                    param,
                    elasticity,
                } => {
                    if let Some(i) = content.sector_index(sector) {
                        out.subsidy[i] += p.param(param) / 100.0 * k;
                        out.subsidy_elasticity[i] = *elasticity;
                    }
                }
                MechanismDef::WageFloor { param } => out.wage_floor += p.param(param) / 100.0 * k,
                MechanismDef::LocalTaxRate { param } => {
                    // Tax changes take effect in full once enacted (no ramp, no funding).
                    out.property_tax_delta += p.param(param) / 100.0 * p.pilot_scale().max(1.0);
                }
                MechanismDef::HazardProtection {
                    hazard,
                    param,
                    per_unit,
                } => {
                    let v = per_unit * p.param(param) * k;
                    match hazard {
                        HazardKey::Flood => out.flood_protection += v,
                        HazardKey::Storm => out.storm_protection += v,
                        HazardKey::Wildfire => out.wildfire_protection += v,
                    }
                }
            }
        }
    }
    out.flood_protection = out.flood_protection.min(0.9);
    out.storm_protection = out.storm_protection.min(0.9);
    out.wildfire_protection = out.wildfire_protection.min(0.9);
    out
}

// --- Support and voting --------------------------------------------------------------------

/// Anticipated support (−1..1) among the voters of some neighbourhoods
/// (DEC-025 anticipated channel: value alignment, turnout-weighted).
pub fn public_support(t: &PolicyTemplate, pop: &PopulationState, hoods: &[usize]) -> f64 {
    let mut acc = 0.0;
    let mut w = 0.0;
    for &h in hoods {
        for c in &pop.hoods[h].cohorts {
            let weight = c.count as f64 * c.turnout as f64;
            acc += alignment(&t.signature_weights, &c.attitudes) * weight;
            w += weight;
        }
    }
    if w > 0.0 { acc / w } else { 0.0 }
}

/// Support broken down by class (low, middle, high) for "Why?" views.
pub fn support_by_class(t: &PolicyTemplate, pop: &PopulationState, hoods: &[usize]) -> [f64; 3] {
    let mut acc = [0.0; 3];
    let mut w = [0.0; 3];
    for &h in hoods {
        for (k, c) in pop.hoods[h].cohorts.iter().enumerate() {
            let class = crate::population::class_of(k);
            let weight = c.count as f64 * c.turnout as f64;
            acc[class] += alignment(&t.signature_weights, &c.attitudes) * weight;
            w[class] += weight;
        }
    }
    std::array::from_fn(|i| if w[i] > 0.0 { acc[i] / w[i] } else { 0.0 })
}

/// Net inclination of a politician toward a policy (positive = yes).
pub struct VoteInputs<'a> {
    pub template: &'a PolicyTemplate,
    pub instance: &'a PolicyInstance,
    pub ward_support: f64,
    pub mayor_lean: f64,
    pub sway: f64,
    pub sponsor_is_player: bool,
    /// The member's opinion of the sponsor when the sponsor is the player
    /// (−100..100; DEC-031 relationship).
    pub player_opinion: f64,
    /// The member's loyalty (DEC-031): how much party discipline weighs.
    pub loyalty: f64,
    /// The sponsor's party (party discipline: members back their own side).
    pub sponsor_party: Option<crate::politics::PartyId>,
    /// A publicly declared vote overrides the model (+1 yes, −1 no, 0 abstain).
    pub declared: Option<i8>,
    /// Bill-specific idiosyncrasy (keyed RNG, stable per member and bill).
    pub noise: f64,
}

pub fn vote_lean(k: &CouncilParams, p: &crate::politics::Politician, v: &VoteInputs<'_>) -> f64 {
    let own = alignment(&v.template.signature_weights, &p.attitudes);
    let earmark = v
        .instance
        .clauses
        .iter()
        .any(|c| matches!((c, p.role), (Clause::Earmark { ward }, Role::Councillor { ward: w, .. }) if *ward == w));
    let player = if v.sponsor_is_player {
        v.player_opinion / 100.0 * k.player_opinion
    } else {
        0.0
    };
    if let Some(d) = v.declared {
        // Abstaining is not a yes.
        return if d > 0 { 3.0 } else { -3.0 };
    }
    // Riders: sunsets and pilots lower the stakes; others' ward projects grate.
    let mut riders = 0.0;
    for c in &v.instance.clauses {
        riders += match c {
            Clause::Sunset { .. } => k.sunset,
            Clause::Pilot { .. } => k.pilot,
            Clause::Earmark { .. } => 0.0,
        };
    }
    let foreign_earmarks = v
        .instance
        .clauses
        .iter()
        .filter(|c| matches!((c, p.role), (Clause::Earmark { ward }, Role::Councillor { ward: w, .. }) if *ward != w))
        .count() as f64;
    riders += k.foreign_earmark * foreign_earmarks;
    let discipline = match (v.sponsor_party, p.party) {
        (Some(a), Some(b)) if a == b => k.same_party,
        (Some(_), Some(_)) => k.rival_party,
        _ => 0.0,
    } * (1.0 + k.loyalty_discipline * (2.0 * v.loyalty - 1.0));
    // Pragmatists weigh constituents and deals; ideologues their own values.
    let prag = p.pragmatism as f64;
    k.own_values * (1.0 - prag) * own
        + k.ward_support * prag * v.ward_support
        + k.mayor * v.mayor_lean
        + v.sway
        + player
        + discipline
        + riders
        + if earmark { k.own_earmark } else { 0.0 }
        + v.noise
}

/// A member's opinion of a bill's sponsor when the sponsor is the player.
fn player_opinion(
    pol: &PoliticsState,
    member: PoliticianId,
    sponsor: Option<PoliticianId>,
    sponsor_is_player: bool,
) -> f64 {
    match sponsor {
        Some(s) if sponsor_is_player => crate::characters::opinion_between(pol, member, s) as f64,
        _ => 0.0,
    }
}

/// A politician's loyalty (their character's), 0.5 if unknown.
fn loyalty(pol: &PoliticsState, p: &crate::politics::Politician) -> f64 {
    pol.characters
        .get(p.character)
        .map(|c| c.loyalty as f64)
        .unwrap_or(0.5)
}

fn noise_for(k: &CouncilParams, seed: u64, politician: u32, policy: u32) -> f64 {
    mandate_core::rng::keyed(
        seed,
        Stream::Npc,
        &[0xB111, politician as u64, policy as u64],
    )
    .normal(0.0, k.noise_sd)
}

/// Lean of every council member for a policy (also used for whip counts).
pub fn council_leans(
    k: &CouncilParams,
    r_world: &World,
    pop: &PopulationState,
    pol: &PoliticsState,
    seed: u64,
    t: &PolicyTemplate,
    inst: &PolicyInstance,
) -> Vec<(PoliticianId, f64)> {
    let Some(council) = pol.councils.get(inst.jurisdiction as usize) else {
        return Vec::new();
    };
    let all: Vec<usize> = r_world
        .hoods_of(MunicipalityId(inst.jurisdiction))
        .collect();
    let mayor_lean = council
        .mayor
        .and_then(|m| pol.get(m))
        .map(|m| {
            alignment(&t.signature_weights, &m.attitudes) * k.mayor_own
                + public_support(t, pop, &all) * k.mayor_public
        })
        .unwrap_or(0.0);
    let sponsor_is_player = inst
        .sponsor
        .and_then(|s| pol.get(s))
        .is_some_and(|s| s.is_player);
    council
        .members
        .iter()
        .filter_map(|id| pol.get(*id))
        .map(|p| {
            let ward_support = match p.role {
                Role::Councillor { ward, .. } => {
                    let hoods: Vec<usize> = r_world
                        .constituencies
                        .get(ward.index())
                        .map(|c| c.neighborhoods.iter().map(|h| h.index()).collect())
                        .unwrap_or_default();
                    public_support(t, pop, &hoods)
                }
                Role::Mayor { .. } | Role::Private { .. } => public_support(t, pop, &all),
            };
            let lean = if inst.sponsor == Some(p.id) {
                1.0
            } else {
                vote_lean(
                    k,
                    p,
                    &VoteInputs {
                        template: t,
                        instance: inst,
                        ward_support,
                        mayor_lean,
                        sway: pol.sway(p.id, inst.id.0) as f64,
                        sponsor_is_player,
                        player_opinion: player_opinion(pol, p.id, inst.sponsor, sponsor_is_player),
                        loyalty: loyalty(pol, p),
                        sponsor_party: inst.sponsor.and_then(|s| pol.get(s)).and_then(|s| s.party),
                        declared: pol.declared_votes.get(&(p.id.0, inst.id.0)).copied(),
                        noise: noise_for(k, seed, p.id.0, inst.id.0),
                    },
                )
            };
            (p.id, lean)
        })
        .collect()
}

pub fn mayor_lean(
    k: &CouncilParams,
    r_world: &World,
    pop: &PopulationState,
    pol: &PoliticsState,
    seed: u64,
    t: &PolicyTemplate,
    inst: &PolicyInstance,
) -> f64 {
    let Some(council) = pol.councils.get(inst.jurisdiction as usize) else {
        return 0.0;
    };
    let Some(m) = council.mayor.and_then(|m| pol.get(m)) else {
        return 0.0;
    };
    if inst.sponsor == Some(m.id) {
        return 1.0;
    }
    let all: Vec<usize> = r_world
        .hoods_of(MunicipalityId(inst.jurisdiction))
        .collect();
    let sponsor_is_player = inst
        .sponsor
        .and_then(|s| pol.get(s))
        .is_some_and(|s| s.is_player);
    vote_lean(
        k,
        m,
        &VoteInputs {
            template: t,
            instance: inst,
            ward_support: public_support(t, pop, &all),
            mayor_lean: 0.0,
            sway: pol.sway(m.id, inst.id.0) as f64,
            sponsor_is_player,
            player_opinion: player_opinion(pol, m.id, inst.sponsor, sponsor_is_player),
            loyalty: loyalty(pol, m),
            sponsor_party: inst.sponsor.and_then(|s| pol.get(s)).and_then(|s| s.party),
            declared: pol.declared_votes.get(&(m.id.0, inst.id.0)).copied(),
            noise: noise_for(k, seed, m.id.0, inst.id.0),
        },
    )
}

/// Probability that a lean turns into a yes vote (shown as whip-count odds).
pub fn yes_probability(k: &CouncilParams, lean: f64) -> f64 {
    sigmoid(lean * k.vote_sharpness)
}

/// Probability that the mayor signs a bill, given their lean.
pub fn sign_probability(k: &CouncilParams, lean: f64) -> f64 {
    yes_probability(k, lean + k.sign_bias)
}

fn passes(rule: DecisionRule, yes: u32, total: u32) -> bool {
    match rule {
        DecisionRule::Majority => yes * 2 > total,
        DecisionRule::Supermajority { num, den } => {
            yes as u64 * den as u64 >= total as u64 * num as u64
        }
        DecisionRule::Unanimity => yes == total,
    }
}

// --- Systems ---------------------------------------------------------------------------------

/// Weekly: run due procedure stages (votes, signatures, overrides).
pub fn procedures(st: &mut PolicyState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let pol = r.politics();
    let pop = r.population();
    let seed = r.meta.seed;
    let Some(constitution) = r.content.constitutions.get(&r.meta.constitution) else {
        return;
    };
    for i in 0..st.instances.len() {
        let due = {
            let p = &st.instances[i];
            p.status == PolicyStatus::UnderConsideration
                && p.next_stage_on.is_some_and(|d| d <= r.date)
        };
        if !due {
            continue;
        }
        let inst = st.instances[i].clone();
        let Some(t) = r.content.policies.get(&inst.template) else {
            continue;
        };
        let Some(tier) = constitution.tier(inst.tier) else {
            continue;
        };
        let Some(proc_def) = tier.procedure("ordinance") else {
            continue;
        };
        let Some(stage) = proc_def.stages.get(inst.stage as usize) else {
            continue;
        };
        let rng = out.rng.get(Stream::Npc);
        let k = &r.content.council;
        let leans = council_leans(k, r.world, pop, pol, seed, t, &inst);
        let tally = |rng: &mut mandate_core::SimRng, leans: &[(PoliticianId, f64)]| -> u32 {
            leans
                .iter()
                .filter(|(_, l)| rng.chance(yes_probability(k, *l)))
                .count() as u32
        };
        let total = leans.len() as u32;
        let (passed, record) = match stage {
            StageDef::Vote { rule, .. } => {
                let yes = tally(rng, &leans);
                let ok = passes(*rule, yes, total);
                (
                    ok,
                    VoteRecord {
                        stage: inst.stage,
                        date: r.date,
                        yes,
                        no: total - yes,
                        passed: ok,
                        veto: false,
                    },
                )
            }
            StageDef::Sign { override_rule, .. } => {
                let lean = mayor_lean(k, r.world, pop, pol, seed, t, &inst);
                if rng.chance(sign_probability(k, lean)) {
                    (
                        true,
                        VoteRecord {
                            stage: inst.stage,
                            date: r.date,
                            yes: 1,
                            no: 0,
                            passed: true,
                            veto: false,
                        },
                    )
                } else if let Some(rule) = override_rule {
                    let yes = tally(rng, &leans);
                    let ok = passes(*rule, yes, total);
                    (
                        ok,
                        VoteRecord {
                            stage: inst.stage,
                            date: r.date,
                            yes,
                            no: total - yes,
                            passed: ok,
                            veto: true,
                        },
                    )
                } else {
                    (
                        false,
                        VoteRecord {
                            stage: inst.stage,
                            date: r.date,
                            yes: 0,
                            no: 1,
                            passed: false,
                            veto: true,
                        },
                    )
                }
            }
        };
        // A mayor's signature or veto is personal: the sponsor remembers it
        // (DEC-031; the politics domain applies it).
        if matches!(stage, StageDef::Sign { .. })
            && let (Some(sponsor), Some(mayor)) = (
                inst.sponsor,
                pol.councils
                    .get(inst.jurisdiction as usize)
                    .and_then(|c| c.mayor),
            )
            && sponsor != mayor
        {
            out.effects.push(crate::scheduler::Effect::Memory {
                holder: sponsor,
                actor: mayor,
                kind: if record.veto {
                    crate::characters::MemoryKind::VetoedMyBill
                } else {
                    crate::characters::MemoryKind::SignedMyBill
                },
                subject: Some(crate::characters::Subject::Policy { id: inst.id.0 }),
            });
        }
        let p = &mut st.instances[i];
        let veto = record.veto;
        p.votes.push(record.clone());
        if !passed {
            p.status = PolicyStatus::Ended;
            p.end_reason = Some(if veto {
                EndReason::Vetoed
            } else {
                EndReason::Defeated
            });
            p.next_stage_on = None;
            let ev = out.events.record(
                r.date,
                EventKind::PolicyFailed {
                    policy: p.id.0,
                    municipality: p.jurisdiction,
                    vetoed: veto,
                    yes: record.yes,
                    no: record.no,
                },
                p.cause.into_iter().collect(),
            );
            out.events.pin(ev);
            continue;
        }
        p.stage += 1;
        if (p.stage as usize) < proc_def.stages.len() {
            p.next_stage_on = Some(r.date.add_days(7));
        } else {
            p.status = PolicyStatus::Enacted;
            p.next_stage_on = None;
            p.enacted_on = Some(r.date);
            p.awareness = (p.awareness + r.content.council.enactment_awareness as f32).min(1.0);
            let ev = out.events.record(
                r.date,
                EventKind::PolicyEnacted {
                    policy: p.id.0,
                    municipality: p.jurisdiction,
                    yes: record.yes,
                    no: record.no,
                    overrode_veto: veto,
                },
                p.cause.into_iter().collect(),
            );
            out.events.pin(ev);
            p.cause = Some(ev);
        }
    }
}

/// Monthly: implementation progress, sunsets, awareness decay, attribution.
pub fn implementation(st: &mut PolicyState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let fin = r.finance();
    let econ = r.economy();
    for i in 0..st.instances.len() {
        let m = st.instances[i].jurisdiction as usize;
        let competence = st.agencies.get(m).map(|a| a.competence).unwrap_or(0.6) as f64;
        let p = &mut st.instances[i];
        let Some(t) = r.content.policies.get(&p.template) else {
            continue;
        };
        p.awareness = (p.awareness * (1.0 - r.content.council.awareness_fade) as f32).max(0.0);
        match p.status {
            PolicyStatus::Enacted | PolicyStatus::InEffect => {
                let funded = fin.funded_share(p.id.0);
                if funded > 0.0 {
                    p.status = PolicyStatus::InEffect;
                    let months = t.implementation_months.max(1) as f64;
                    p.progress = clamp(
                        p.progress as f64 + funded * (0.5 + competence) / months,
                        0.0,
                        1.0,
                    ) as f32;
                }
                // Sunset clauses.
                if let (Some(on), Some(years)) = (
                    p.enacted_on,
                    p.clauses.iter().find_map(|c| {
                        if let Clause::Sunset { years } = c {
                            Some(*years)
                        } else {
                            None
                        }
                    }),
                ) && on.days_until(r.date) >= years as u32 * 365
                {
                    p.status = PolicyStatus::Ended;
                    p.end_reason = Some(EndReason::Expired);
                    out.events.record(
                        r.date,
                        EventKind::PolicyExpired {
                            policy: p.id.0,
                            municipality: p.jurisdiction,
                        },
                        p.cause.into_iter().collect(),
                    );
                }
                // Attribution ledger: direct contributions this month.
                let k = p.progress as f64 * p.pilot_scale() * funded;
                let e = econ.munis.get(m);
                for mech in &t.mechanisms {
                    match mech {
                        MechanismDef::HousingSupply { param } => {
                            *p.ledger.entry("homes_built".into()).or_default() +=
                                p.param(param) * k / 12.0
                        }
                        MechanismDef::SectorSubsidy {
                            sector,
                            param,
                            elasticity,
                        } => {
                            let base = r
                                .content
                                .sector_index(sector)
                                .and_then(|i| e.and_then(|x| x.sectors.get(i)))
                                .map(|x| x.capacity)
                                .unwrap_or(0.0);
                            p.ledger.insert(
                                format!("jobs_supported_{}", sector_key(sector)),
                                base * elasticity * p.param(param) / 100.0 * k,
                            );
                        }
                        MechanismDef::Transfer { param, .. } => {
                            *p.ledger.entry("paid_per_person".into()).or_default() +=
                                p.param(param) * k / 12.0;
                        }
                        MechanismDef::Service {
                            param, per_unit, ..
                        } => {
                            p.ledger.insert(
                                "service_gain_pct".into(),
                                per_unit * p.param(param) * k * 100.0,
                            );
                        }
                        MechanismDef::Zoning { param, per_unit } => {
                            p.ledger.insert(
                                "extra_building_pct".into(),
                                per_unit * p.param(param) * k * 100.0,
                            );
                        }
                        MechanismDef::WageFloor { param } => {
                            p.ledger
                                .insert("wage_uplift_pct".into(), p.param(param) * k);
                        }
                        MechanismDef::HazardProtection {
                            param, per_unit, ..
                        } => {
                            p.ledger.insert(
                                "damage_reduction_pct".into(),
                                (per_unit * p.param(param) * k * 100.0).min(90.0),
                            );
                        }
                        MechanismDef::LocalTaxRate { param } => {
                            p.ledger.insert("tax_change_pts".into(), p.param(param));
                        }
                    }
                }
                p.ledger.insert("funded_pct".into(), funded * 100.0);
            }
            _ => {}
        }
    }
}

/// Short form of a sector's content key for ledger keys:
/// `sector.retail` → `retail`, `sector.fish.farm` → `fish_farm`.
pub fn sector_key(key: &str) -> String {
    key.strip_prefix("sector.").unwrap_or(key).replace('.', "_")
}

/// Ward neighbourhoods as indices.
pub fn ward_hoods(world: &World, ward: ConstituencyId) -> Vec<usize> {
    world
        .constituencies
        .get(ward.index())
        .map(|c| c.neighborhoods.iter().map(|h| h.index()).collect())
        .unwrap_or_default()
}

pub fn attitudes_of(pop: &PopulationState, hoods: &[usize]) -> Vec<f32> {
    mean_attitudes(pop, hoods)
}
