//! Policy lifecycle (DEC-024 rev. 2): Draft → Under consideration → Enacted →
//! In effect → Ended. Procedures come from the constitution (DEC-017);
//! effects come only from typed mechanisms read by the domain models
//! (REQ-011).

use crate::events::{EventId, EventKind};
use crate::politics::{PoliticianId, PoliticsState, alignment, mean_attitudes};
use crate::population::PopulationState;
use crate::scheduler::{Outs, Reads};
use crate::spine::{Cause, Causes, Link, Process};
use crate::world::{ConstituencyId, World};
use mandate_content::{
    BudgetCategory, ClassKey, ConstitutionDef, ContentBundle, CouncilParams, DecisionRule,
    HazardKey, MechanismDef, PartyLineParams, PolicyTemplate, StageDef, TierKey,
};
use mandate_core::math::{clamp, sigmoid};
use mandate_core::{IdAllocator, SimDate, Stream, define_id};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub mod drafting;
pub mod families;
pub mod impact;
pub mod resolve;
pub mod works;
pub use resolve::{Resolved, resolve};

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
    /// Where a policy family's bill applies (an option's `area`, DEC-092):
    /// per-capita costs count these residents, and area-aware mechanisms and
    /// side effects (hazard protection, disruption, grievance) fall on these
    /// neighbourhoods only.
    Area {
        neighborhoods: Vec<crate::world::NeighborhoodId>,
    },
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
    /// A policy family's bill (DEC-092): the proposal and the choices it was
    /// drafted with; `template` holds the family id. `None` for a standard
    /// bill (and not serialized, so standard bills read as before).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<families::FamilyDraft>,
    /// What the bill said it would do when it passed (DEC-111): the impact
    /// estimate at enactment, kept so the ledger can be read against it.
    /// `None` for a bill not yet passed, and for a law passed before save
    /// v28 or in a tier the impact table does not cover.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate: Option<impact::FrozenEstimate>,
}

impl PolicyInstance {
    /// The neighbourhoods of its `area` clause, if it has one.
    pub fn area(&self) -> Option<&[crate::world::NeighborhoodId]> {
        self.clauses.iter().find_map(|c| match c {
            Clause::Area { neighborhoods } => Some(neighborhoods.as_slice()),
            _ => None,
        })
    }
    /// The jurisdiction the bill or law belongs to (DEC-098).
    pub fn at(&self) -> crate::world::Jurisdiction {
        crate::world::Jurisdiction::of(self.tier, self.jurisdiction)
    }
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
    /// Policy families: decks, proposals, cooldowns (DEC-092).
    #[serde(default)]
    pub families: families::FamilyState,
    /// Works you can see (DEC-099).
    #[serde(default)]
    pub works: works::WorksState,
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
        families: Default::default(),
        works: Default::default(),
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

/// The residents a bill's per-capita costs count: those of its `area`
/// (DEC-092), else of its jurisdiction.
pub fn cost_residents(
    world: &World,
    pop: &PopulationState,
    p: &PolicyInstance,
    jurisdiction: f64,
) -> f64 {
    match p.area() {
        Some(hoods) => hoods
            .iter()
            .filter(|h| h.index() < world.neighborhoods.len())
            .map(|h| pop.hood_total(h.index()) as f64)
            .sum(),
        None => jurisdiction,
    }
}

/// Annual income (nominal) from a bill's `revenue` mechanisms (DEC-092
/// families), for `residents` (its area's or jurisdiction's).
pub fn annual_revenue(
    t: &PolicyTemplate,
    p: &PolicyInstance,
    residents: f64,
    price_level: f64,
) -> f64 {
    let mut v = 0.0;
    for m in &t.mechanisms {
        if let MechanismDef::Revenue {
            per_capita,
            per_unit,
            param,
        } = m
        {
            v += per_capita * residents
                + per_unit * param.as_deref().map(|k| p.param(k)).unwrap_or(0.0);
        }
    }
    v * p.pilot_scale() * price_level
}

/// The share of a bill's annual cost it borrows (`borrow` mechanisms; at most 1).
pub fn borrow_share(t: &PolicyTemplate) -> f64 {
    t.mechanisms
        .iter()
        .map(|m| match m {
            MechanismDef::Borrow { share } => *share,
            _ => 0.0,
        })
        .sum::<f64>()
        .clamp(0.0, 1.0)
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
    /// The tightest cap on yearly (nominal) rent growth of a law in force
    /// (`rent_cap`), if any.
    pub rent_cap: Option<f64>,
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
    /// Structural jobs added by sector, in registry order (`sector_capacity`;
    /// empty when no policy adds any).
    pub capacity: Vec<f64>,
    /// Hazard protection of single neighbourhoods (an `area` clause,
    /// DEC-092): (flood, storm, wildfire), added to the town's.
    pub hood_protection: BTreeMap<u32, [f64; 3]>,
    /// Disruption felt in single neighbourhoods (an `area` clause).
    pub hood_disruption: BTreeMap<u32, f64>,
}

impl PolicyMods {
    /// Protection against a hazard in a neighbourhood: the town's plus the
    /// neighbourhood's own (capped at 0.9).
    pub fn protection_at(&self, hazard: HazardKey, hood: u32) -> f64 {
        let i = match hazard {
            HazardKey::Flood => 0,
            HazardKey::Storm => 1,
            HazardKey::Wildfire => 2,
        };
        let town = match hazard {
            HazardKey::Flood => self.flood_protection,
            HazardKey::Storm => self.storm_protection,
            HazardKey::Wildfire => self.wildfire_protection,
        };
        match self.hood_protection.get(&hood) {
            Some(v) => (town + v[i]).min(0.9),
            None => town,
        }
    }

    /// Disruption felt in a neighbourhood beyond the town's.
    pub fn disruption_at(&self, hood: u32) -> f64 {
        self.hood_disruption.get(&hood).copied().unwrap_or(0.0)
    }
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

/// The yearly rent growth a `rent_cap` at `level` allows (`None` at 0).
pub fn rent_cap_of(level: f64, loosest: f64, per_unit: f64) -> Option<f64> {
    (level >= 0.5).then(|| loosest + per_unit * (level.round() - 1.0))
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
        let Some(t) = resolve(content, st, p) else {
            continue;
        };
        let area = p.area();
        // A family option's effect factor scales every mechanism (DEC-092).
        let k = impact::scale(p, funded(p.id.0), t.effect_factor);
        // Side effects (DEC-073): disruption while building, labour supply once running.
        let building = impact::building(p, funded(p.id.0));
        for fx in &t.side_effects {
            let a = side_effect_amount(fx, p);
            match fx {
                mandate_content::SideEffectDef::Disruption { .. } => match area {
                    Some(hoods) => {
                        for h in hoods {
                            *out.hood_disruption.entry(h.0).or_default() += a * building;
                        }
                    }
                    None => out.disruption += a * building,
                },
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
                } => {
                    out.services[category_index(*category)] +=
                        impact::linear(*per_unit, p.param(param), k)
                }
                MechanismDef::Transfer { class, param } => {
                    let i = match class {
                        ClassKey::Low => 0,
                        ClassKey::Middle => 1,
                        ClassKey::High => 2,
                    };
                    out.transfer[i] += impact::flat(p.param(param), k);
                }
                MechanismDef::HousingSupply { param } => {
                    out.housing_units_per_year += impact::flat(p.param(param), k)
                }
                MechanismDef::Zoning { param, per_unit } => {
                    out.starts_multiplier *= 1.0 + impact::linear(*per_unit, p.param(param), k)
                }
                // A cap is a rule: in force in full once the law is (no ramp).
                MechanismDef::RentCap {
                    param,
                    loosest,
                    per_unit,
                } => {
                    if let Some(cap) = rent_cap_of(p.param(param), *loosest, *per_unit) {
                        out.rent_cap = Some(out.rent_cap.map_or(cap, |c: f64| c.min(cap)));
                    }
                }
                MechanismDef::SectorSubsidy {
                    sector,
                    param,
                    elasticity,
                } => {
                    if let Some(i) = content.sector_index(sector) {
                        out.subsidy[i] += impact::percent(p.param(param), k);
                        out.subsidy_elasticity[i] = *elasticity;
                    }
                }
                MechanismDef::WageFloor { param } => {
                    out.wage_floor += impact::percent(p.param(param), k)
                }
                MechanismDef::LocalTaxRate { param } => {
                    // Tax changes take effect in full once enacted (no ramp, no funding).
                    out.property_tax_delta += impact::tax_delta(p.param(param), p.pilot_scale());
                }
                MechanismDef::HazardProtection {
                    hazard,
                    param,
                    per_unit,
                } => {
                    let v = impact::linear(*per_unit, p.param(param), k);
                    match (hazard.key(), area) {
                        (Some(h), Some(hoods)) => {
                            let i = match h {
                                HazardKey::Flood => 0,
                                HazardKey::Storm => 1,
                                HazardKey::Wildfire => 2,
                            };
                            for x in hoods {
                                out.hood_protection.entry(x.0).or_default()[i] += v;
                            }
                        }
                        (Some(HazardKey::Flood), None) => out.flood_protection += v,
                        (Some(HazardKey::Storm), None) => out.storm_protection += v,
                        (Some(HazardKey::Wildfire), None) => out.wildfire_protection += v,
                        (None, _) => {}
                    }
                }
                // Structural jobs (DEC-092 families; the economy reads them).
                MechanismDef::SectorCapacity {
                    sector,
                    param,
                    per_unit,
                } => {
                    if let Some(i) = content.sector_index(sector) {
                        if out.capacity.is_empty() {
                            out.capacity = vec![0.0; content.sectors.len()];
                        }
                        out.capacity[i] += impact::linear(*per_unit, p.param(param), k);
                    }
                }
                // Finance reads these (`borrow_share`, `annual_revenue`).
                MechanismDef::Borrow { .. } | MechanismDef::Revenue { .. } => {}
            }
        }
    }
    out.flood_protection = out.flood_protection.min(0.9);
    out.storm_protection = out.storm_protection.min(0.9);
    out.wildfire_protection = out.wildfire_protection.min(0.9);
    for v in out.hood_protection.values_mut() {
        for x in v.iter_mut() {
            *x = x.min(0.9);
        }
    }
    out
}

// --- Situational policies (DEC-077 §3) ------------------------------------------------

/// Does a condition hold in town `m`'s situation (a policy's unlock, a
/// family trigger)? The one evaluator (`crate::condition`) on the whole
/// state, with no slots bound. Read-only.
pub fn situation_holds(
    state: &crate::state::State,
    content: &ContentBundle,
    c: &mandate_content::Condition,
    m: u32,
) -> bool {
    crate::condition::on_state(state, content, |v| {
        let cx = crate::condition::Ctx::town(v, state, crate::world::MunicipalityId(m));
        crate::condition::holds(&cx, c, &mut crate::condition::Args::new())
    })
}

/// Is a template on the table in town `m`? Templates without an unlock
/// always are; a situational one while any of its conditions holds.
pub fn unlocked(
    state: &crate::state::State,
    content: &ContentBundle,
    t: &PolicyTemplate,
    m: u32,
) -> bool {
    match &t.unlock {
        None => true,
        Some(u) => triggered(state, content, u, m),
    }
}

/// Does a situation trigger hold in town `m` (any of its conditions)?
pub fn triggered(
    state: &crate::state::State,
    content: &ContentBundle,
    t: &mandate_content::SituationTrigger,
    m: u32,
) -> bool {
    t.any.iter().any(|c| situation_holds(state, content, c, m))
}

/// A draft-shaped instance of a template with the given parameters
/// (validated: unknown ones ignored, missing ones at their default, values
/// clamped and snapped to the step), for previews of cost and side effects.
pub fn probe(t: &PolicyTemplate, m: u32, params: &BTreeMap<String, f64>) -> PolicyInstance {
    let params = t
        .params
        .iter()
        .map(|p| {
            let v = params
                .get(&p.id)
                .copied()
                .filter(|v| v.is_finite())
                .unwrap_or(p.default);
            let snapped = p.min + ((v.clamp(p.min, p.max) - p.min) / p.step).round() * p.step;
            (p.id.clone(), clamp(snapped, p.min, p.max))
        })
        .collect();
    PolicyInstance {
        id: PolicyInstanceId(u32::MAX),
        template: t.id.clone(),
        tier: t.tier,
        jurisdiction: m,
        sponsor: None,
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
        ledger: Default::default(),
        family: None,
        estimate: None,
    }
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
    /// The executive's (the signer's) public lean on the bill.
    pub signer_lean: f64,
    pub sway: f64,
    pub sponsor_is_player: bool,
    /// The member's opinion of the sponsor when the sponsor is the player
    /// (−100..100; DEC-031 relationship).
    pub player_opinion: f64,
    /// The member's loyalty (DEC-031): how much party discipline weighs.
    pub loyalty: f64,
    /// The pull of the member's party on this bill: the party line's score
    /// (`politics::party_line::npc_pull`), nothing in a free vote.
    pub party_pull: f64,
    /// A publicly declared vote overrides the model (+1 yes, −1 no, 0 abstain).
    pub declared: Option<i8>,
    /// Bill-specific idiosyncrasy (keyed RNG, stable per member and bill).
    pub noise: f64,
}

pub fn vote_lean(k: &CouncilParams, p: &crate::politics::Politician, v: &VoteInputs<'_>) -> f64 {
    let own = alignment(&v.template.signature_weights, &p.attitudes);
    // Earmarks name a district: the member for it gains, others' grate.
    let district = p.role.constituency();
    let earmark = v
        .instance
        .clauses
        .iter()
        .any(|c| matches!(c, Clause::Earmark { ward } if district == Some(*ward)));
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
            Clause::Earmark { .. } | Clause::Area { .. } => 0.0,
        };
    }
    let foreign_earmarks = v
        .instance
        .clauses
        .iter()
        .filter(|c| matches!(c, Clause::Earmark { ward } if district.is_some_and(|w| w != *ward)))
        .count() as f64;
    riders += k.foreign_earmark * foreign_earmarks;
    let discipline = v.party_pull * (1.0 + k.loyalty_discipline * (2.0 * v.loyalty - 1.0));
    // Pragmatists weigh constituents and deals; ideologues their own values.
    let prag = p.pragmatism as f64;
    k.own_values * (1.0 - prag) * own
        + k.ward_support * prag * v.ward_support
        + k.mayor * v.signer_lean
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

/// The lean of each member of a body on a bill (also whip counts), in seat
/// order. `signer` is whoever signs the bill (the executive), whose public
/// lean pulls members along (`council.mayor`).
#[allow(clippy::too_many_arguments)]
pub fn member_leans(
    k: &CouncilParams,
    pl: &PartyLineParams,
    r_world: &World,
    pop: &PopulationState,
    pol: &PoliticsState,
    seed: u64,
    t: &PolicyTemplate,
    inst: &PolicyInstance,
    members: &[PoliticianId],
    signer: Option<PoliticianId>,
) -> Vec<(PoliticianId, f64)> {
    let all: Vec<usize> = inst.at().hoods(r_world);
    let signer_lean = signer
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
    let sponsor_party = inst.sponsor.and_then(|s| pol.get(s)).and_then(|s| s.party);
    let governing = signer.and_then(|m| pol.get(m)).and_then(|m| m.party);
    members
        .iter()
        .filter_map(|id| pol.get(*id))
        .map(|p| {
            // A member weighs their own district's support, else the
            // jurisdiction's.
            let ward_support = match p.role.constituency() {
                Some(ward) => {
                    let hoods: Vec<usize> = r_world
                        .constituencies
                        .get(ward.index())
                        .map(|c| c.neighborhoods.iter().map(|h| h.index()).collect())
                        .unwrap_or_default();
                    public_support(t, pop, &hoods)
                }
                None => public_support(t, pop, &all),
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
                        signer_lean,
                        // Goals (DEC-032) lean on the player's bills like lobbying does.
                        sway: pol.sway(p.id, inst.id.0) as f64
                            + crate::characters::goals::lean_on(
                                pol,
                                p.character,
                                sponsor_is_player,
                            ),
                        sponsor_is_player,
                        player_opinion: player_opinion(pol, p.id, inst.sponsor, sponsor_is_player),
                        loyalty: loyalty(pol, p),
                        party_pull: crate::politics::party_line::npc_pull(
                            pl,
                            pol,
                            p.party,
                            sponsor_party,
                            governing,
                            &t.signature_weights,
                        ),
                        declared: pol.declared_votes.get(&(p.id.0, inst.id.0)).copied(),
                        noise: noise_for(k, seed, p.id.0, inst.id.0),
                    },
                )
            };
            (p.id, lean)
        })
        .collect()
}

/// The leans of the bill's legislature (the body of its jurisdiction's
/// `ordinance` procedure's first vote; DEC-098), with its executive as the
/// signer: the whip count.
#[allow(clippy::too_many_arguments)]
pub fn legislature_leans(
    k: &CouncilParams,
    pl: &PartyLineParams,
    c: &ConstitutionDef,
    r_world: &World,
    pop: &PopulationState,
    pol: &PoliticsState,
    seed: u64,
    t: &PolicyTemplate,
    inst: &PolicyInstance,
) -> Vec<(PoliticianId, f64)> {
    let at = inst.at();
    let members = crate::politics::legislature_of(pol, c, at);
    let signer = crate::politics::executive_of(pol, c, at);
    member_leans(k, pl, r_world, pop, pol, seed, t, inst, members, signer)
}

/// The lean of a bill's signer (whoever holds the signing office) on it.
#[allow(clippy::too_many_arguments)]
pub fn signer_lean(
    k: &CouncilParams,
    pl: &PartyLineParams,
    r_world: &World,
    pop: &PopulationState,
    pol: &PoliticsState,
    seed: u64,
    t: &PolicyTemplate,
    inst: &PolicyInstance,
    signer: Option<PoliticianId>,
) -> f64 {
    let Some(m) = signer.and_then(|m| pol.get(m)) else {
        return 0.0;
    };
    if inst.sponsor == Some(m.id) {
        return 1.0;
    }
    let all: Vec<usize> = inst.at().hoods(r_world);
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
            signer_lean: 0.0,
            sway: pol.sway(m.id, inst.id.0) as f64,
            sponsor_is_player,
            player_opinion: player_opinion(pol, m.id, inst.sponsor, sponsor_is_player),
            loyalty: loyalty(pol, m),
            party_pull: crate::politics::party_line::npc_pull(
                pl,
                pol,
                m.party,
                inst.sponsor.and_then(|s| pol.get(s)).and_then(|s| s.party),
                m.party,
                &t.signature_weights,
            ),
            declared: pol.declared_votes.get(&(m.id.0, inst.id.0)).copied(),
            noise: noise_for(k, seed, m.id.0, inst.id.0),
        },
    )
}

/// The lean of the bill's jurisdiction's executive (its `ordinance`
/// procedure's signer).
#[allow(clippy::too_many_arguments)]
pub fn executive_lean(
    k: &CouncilParams,
    pl: &PartyLineParams,
    c: &ConstitutionDef,
    r_world: &World,
    pop: &PopulationState,
    pol: &PoliticsState,
    seed: u64,
    t: &PolicyTemplate,
    inst: &PolicyInstance,
) -> f64 {
    let signer = crate::politics::executive_of(pol, c, inst.at());
    signer_lean(k, pl, r_world, pop, pol, seed, t, inst, signer)
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
        let Some(t) = resolve(r.content, st, &inst) else {
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
        let pl = &r.content.politics.party_line;
        // Each stage reads its own body and office (DEC-098): a vote tallies
        // the stage's body, a signature asks the stage's office's holder,
        // and an override tallies the override body. The procedure's
        // signer's lean pulls members along.
        let at = inst.at();
        let signer = proc_def.stages.iter().find_map(|s| match s {
            StageDef::Sign { office, .. } => crate::politics::holder_of(pol, office, at),
            StageDef::Vote { .. } => None,
        });
        let first_body = proc_def.stages.iter().find_map(|s| match s {
            StageDef::Vote { body, .. } => Some(body.as_str()),
            StageDef::Sign { .. } => None,
        });
        let leans_of = |body: Option<&str>| -> Vec<(PoliticianId, f64)> {
            let members = body
                .map(|b| crate::politics::members_of(pol, b, at))
                .unwrap_or(&[]);
            member_leans(k, pl, r.world, pop, pol, seed, &t, &inst, members, signer)
        };
        let tally = |rng: &mut mandate_core::SimRng, leans: &[(PoliticianId, f64)]| -> u32 {
            leans
                .iter()
                .filter(|(_, l)| rng.chance(yes_probability(k, *l)))
                .count() as u32
        };
        let (passed, record) = match stage {
            StageDef::Vote { body, rule } => {
                let leans = leans_of(Some(body));
                let total = leans.len() as u32;
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
            StageDef::Sign {
                office,
                override_body,
                override_rule,
            } => {
                let holder = crate::politics::holder_of(pol, office, at);
                let lean = signer_lean(k, pl, r.world, pop, pol, seed, &t, &inst, holder);
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
                    let leans = leans_of(override_body.as_deref().or(first_body));
                    let total = leans.len() as u32;
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
        // A signature or veto is personal: the sponsor remembers the
        // signer (DEC-031; the politics domain applies it).
        if let StageDef::Sign { office, .. } = stage
            && let (Some(sponsor), Some(holder)) =
                (inst.sponsor, crate::politics::holder_of(pol, office, at))
            && sponsor != holder
        {
            out.effects.push(
                crate::scheduler::Effect::Memory {
                    holder: sponsor,
                    actor: holder,
                    kind: if record.veto {
                        crate::characters::MemoryKind::VetoedMyBill
                    } else {
                        crate::characters::MemoryKind::SignedMyBill
                    },
                    subject: Some(crate::characters::Subject::Policy { id: inst.id.0 }),
                },
                // The bill (its latest event), at this stage of the procedure.
                Cause::event_or(inst.cause, Link::Caused, Cause::Process(Process::Lawmaking)),
            );
        }
        // What the bill says it will do, as it passes its last stage
        // (DEC-111): kept with the law, so the ledger can later be read
        // against it. A pure function of published state: it draws nothing.
        let enacting = passed && (inst.stage as usize + 1) >= proc_def.stages.len();
        let frozen = if enacting {
            impact::inputs(r, st, at).map(|x| impact::estimate(&x, r.content, &t, &inst).freeze())
        } else {
            None
        };
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
            out.events.record(
                r.date,
                EventKind::PolicyFailed {
                    policy: p.id.0,
                    municipality: p.jurisdiction,
                    vetoed: veto,
                    yes: record.yes,
                    no: record.no,
                },
                Causes::from(Cause::Process(Process::Lawmaking)).and_event(p.cause, Link::Ended),
            );
            continue;
        }
        p.stage += 1;
        if (p.stage as usize) < proc_def.stages.len() {
            p.next_stage_on = Some(r.date.add_days(7));
        } else {
            p.status = PolicyStatus::Enacted;
            p.next_stage_on = None;
            p.enacted_on = Some(r.date);
            p.estimate = frozen;
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
                Causes::from(Cause::Process(Process::Lawmaking))
                    .and_event(p.cause, Link::Continues),
            );
            p.cause = Some(ev);
        }
    }
}

/// Monthly: implementation progress, sunsets, awareness decay, attribution.
pub fn implementation(st: &mut PolicyState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let fin = r.finance();
    let econ = r.economy();
    // Private starts last month in towns where a zoning law is in effect
    // (all starts less public programmes'), for the homes it adds.
    let zoned: BTreeMap<u32, f64> = st
        .instances
        .iter()
        .filter(|p| p.tier == TierKey::Local && p.status == PolicyStatus::InEffect)
        .filter(|p| {
            resolve(r.content, st, p).is_some_and(|t| {
                t.mechanisms
                    .iter()
                    .any(|m| matches!(m, MechanismDef::Zoning { .. }))
            })
        })
        .map(|p| p.jurisdiction)
        .collect::<std::collections::BTreeSet<u32>>()
        .into_iter()
        .filter_map(|m| {
            let e = econ.munis.get(m as usize)?;
            let public =
                mods_for(st, r.content, m, &|id| fin.funded_share(id)).housing_units_per_year;
            Some((m, (e.housing.last_starts - public / 12.0).max(0.0)))
        })
        .collect();
    for i in 0..st.instances.len() {
        let m = st.instances[i].jurisdiction as usize;
        let competence = st.agencies.get(m).map(|a| a.competence).unwrap_or(0.6) as f64;
        let Some(t) = resolve(r.content, st, &st.instances[i]) else {
            continue;
        };
        let p = &mut st.instances[i];
        p.awareness = (p.awareness * (1.0 - r.content.council.awareness_fade) as f32).max(0.0);
        match p.status {
            PolicyStatus::Enacted | PolicyStatus::InEffect => {
                let funded = fin.funded_share(p.id.0);
                if funded > 0.0 {
                    p.status = PolicyStatus::InEffect;
                    let months = t.implementation_months.max(1) as f64;
                    p.progress = clamp(
                        p.progress as f64 + impact::progress_step(funded, competence, months),
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
                        // Its own sunset clause.
                        Causes::from(Cause::Process(Process::Lawmaking))
                            .and_event(p.cause, Link::Ended),
                    );
                }
                // Attribution ledger (audit fix 5): direct counts, exact, and
                // what they added, estimated with the same coefficient for
                // every template (`economy.attribution`). The same scale as
                // the mechanisms' own (`mods_for`), a family option's
                // effect factor included.
                let k = impact::scale(p, funded, t.effect_factor);
                let e = econ.munis.get(m);
                // Saves from before the split named homes started `homes_built`.
                if let Some(v) = p.ledger.remove("homes_built") {
                    *p.ledger.entry("homes_started".into()).or_default() += v;
                }
                let capacity = |sector: &str| {
                    r.content
                        .sector_index(sector)
                        .and_then(|i| e.and_then(|x| x.sectors.get(i)))
                        .map(|x| x.capacity)
                        .unwrap_or(0.0)
                };
                let books = impact::Books {
                    k,
                    lag: r.content.economy.construction_lag_months.max(1) as f64,
                    demolition: r.content.economy.housing.demolition_rate,
                    net: &r.content.economy.attribution,
                    capacity: &capacity,
                    private_starts: zoned.get(&p.jurisdiction).copied(),
                    rent_market: e.map(|x| {
                        impact::rent_market(
                            &r.content.economy,
                            x.housing.vacancy,
                            econ.national.inflation,
                        )
                    }),
                };
                // One place books a month (`impact::book`): the estimate
                // before the vote runs the same arithmetic forward.
                for mech in &t.mechanisms {
                    impact::book(&mut p.ledger, mech, &p.params, &books);
                }
                p.ledger.insert("funded_pct".into(), funded * 100.0);
            }
            // What an ended law said when it passed is kept twelve years
            // from its enactment (as the spine keeps notable events).
            PolicyStatus::Ended
                if p.estimate.is_some()
                    && p.enacted_on
                        .is_none_or(|on| on.days_until(r.date) >= impact::KEPT_YEARS * 365) =>
            {
                p.estimate = None;
            }
            _ => {}
        }
    }
}

/// Homes a law started this month: counted (a programme's own) or
/// estimated (private building a rule brought on, not booked as started).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HomesStarted {
    Exact(f64),
    Estimated(f64),
}

/// A month of a housing programme in its ledger: `started` homes join the
/// pipeline (`homes_building`), which completes over the construction lag
/// (a first-order estimate of the fixed lag); `homes_added` is what the
/// completions added to the stock, net of the private starts they displace
/// (`net`) and of demolition since. `homes_started` is exact.
pub fn record_homes(
    ledger: &mut BTreeMap<String, f64>,
    started: HomesStarted,
    lag: f64,
    demolition: f64,
    net: f64,
) {
    let started = match started {
        HomesStarted::Exact(n) => {
            *ledger.entry("homes_started".into()).or_default() += n;
            n
        }
        HomesStarted::Estimated(n) => n,
    };
    let building = ledger.get("homes_building").copied().unwrap_or(0.0) + started;
    let done = building / lag.max(1.0);
    ledger.insert("homes_building".into(), building - done);
    let added =
        ledger.get("homes_added").copied().unwrap_or(0.0) * (1.0 - demolition / 12.0) + done * net;
    ledger.insert("homes_added".into(), added);
}

/// What a law delivers now, from its ledger (audit fix 3): jobs added
/// (the sum of its `jobs_added_*` levels) and homes added a year (its
/// pipeline completing, net); and what it pays a year per adult by class
/// (at price level 1), the share by which it raises low incomes, and how far
/// a rent cap holds rents down.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Delivered {
    pub jobs: f64,
    pub homes_per_year: f64,
    pub paid: [f64; 3],
    pub low_income_share: f64,
    /// The share by which a rent cap holds rents below the market's (an
    /// estimate; tenants keep it).
    pub rent_held: f64,
}

pub fn delivered(
    content: &ContentBundle,
    t: &Resolved<'_>,
    p: &PolicyInstance,
    funded: f64,
) -> Delivered {
    let mut d = Delivered::default();
    if p.status != PolicyStatus::InEffect {
        return d;
    }
    let net = &content.economy.attribution;
    let lag = content.economy.construction_lag_months.max(1) as f64;
    d.jobs = p
        .ledger
        .iter()
        .filter(|(k, _)| k.starts_with("jobs_added_"))
        .map(|(_, v)| *v)
        .sum();
    d.homes_per_year =
        p.ledger.get("homes_building").copied().unwrap_or(0.0) / lag * 12.0 * net.homes_net;
    d.rent_held = p.ledger.get("rent_held_pct").copied().unwrap_or(0.0) / 100.0;
    let k = impact::scale(p, funded, t.effect_factor);
    for mech in &t.mechanisms {
        match mech {
            MechanismDef::Transfer { class, param } => {
                d.paid[class_index(*class)] += impact::flat(p.param(param), k);
            }
            MechanismDef::WageFloor { param } => {
                d.low_income_share += impact::percent(p.param(param), k) * net.wage_floor_net;
            }
            _ => {}
        }
    }
    d
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
