//! Policy templates (DEC-024, DEC-026), their mechanisms, costs and side effects, and situation triggers.

use super::*;

// --- Policy templates (DEC-024, DEC-026) --------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParamDef {
    pub id: String,
    pub name_key: String,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub step: f64,
    /// Fluent key for the unit label.
    pub unit_key: String,
    /// Not a slider: a value an option fixes for its mechanisms (DEC-092
    /// families). Hidden parameters are never shown or set by a draft.
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Budget categories (mirror the finance domain's categories).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum BudgetCategory {
    Administration,
    Education,
    Health,
    Safety,
    Infrastructure,
    Social,
    Housing,
    Environment,
    EconomicDevelopment,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ClassKey {
    Low,
    Middle,
    High,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HazardKey {
    Flood,
    Storm,
    Wildfire,
}

/// A hazard a mechanism names: a hazard, or (in a policy family) `@slot`,
/// the hazard bound to that slot. `"flood"` parses as before.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum HazardRef {
    Key(HazardKey),
    /// `@slot`: a family's hazard slot (composition replaces it).
    Slot(String),
}

impl HazardRef {
    /// The hazard, once any slot reference is resolved.
    pub fn key(&self) -> Option<HazardKey> {
        match self {
            HazardRef::Key(k) => Some(*k),
            HazardRef::Slot(_) => None,
        }
    }
}

/// Typed mechanism primitives (DEC-024). Policies never write outcomes
/// directly; the domain models compute consequences from these.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MechanismDef {
    /// Raise a service level; `per_unit` = index gain per parameter unit.
    Service {
        category: BudgetCategory,
        param: String,
        per_unit: f64,
    },
    /// Annual cash transfer per adult in a class, in currency at price level 1.
    Transfer { class: ClassKey, param: String },
    /// Extra dwellings built per year.
    HousingSupply { param: String },
    /// Multiplies private housing starts by (1 + per_unit × param).
    Zoning { param: String, per_unit: f64 },
    /// Caps how fast rent may rise while the law is in force: at a level
    /// above 0, rent may grow at most `loosest` + `per_unit` × (level − 1)
    /// a year (nominal). Level 0 caps nothing. The housing market does the
    /// rest: excess demand shows as a shortage, returns and starts fall,
    /// and rent catches up once the cap lifts.
    RentCap {
        param: String,
        loosest: f64,
        per_unit: f64,
    },
    /// Subsidy as a share of a sector's wage bill; jobs respond with `elasticity`.
    SectorSubsidy {
        /// Content key of the sector, e.g. `sector.retail`.
        sector: ContentKey,
        param: String,
        elasticity: f64,
    },
    /// Wage floor as a share above the low-class market wage.
    WageFloor { param: String },
    /// Reduces damage from a hazard by `per_unit` × param (capped at 0.9).
    HazardProtection {
        hazard: HazardRef,
        param: String,
        per_unit: f64,
    },
    /// Changes the local property tax by `param` percentage points of value.
    LocalTaxRate { param: String },
    /// Finance (DEC-092 families): this share of the annual cost is added to
    /// the jurisdiction's debt instead of being spent from its budget.
    Borrow { share: f64 },
    /// Finance: income that offsets the cost, `per_capita` per resident (of
    /// the area) plus `per_unit` × param a year, at price level 1.
    Revenue {
        #[serde(default)]
        per_capita: f64,
        #[serde(default)]
        per_unit: f64,
        #[serde(default)]
        param: Option<String>,
    },
    /// Economy: structural jobs in a sector, `per_unit` × param.
    SectorCapacity {
        sector: ContentKey,
        param: String,
        per_unit: f64,
    },
}

impl MechanismDef {
    /// The parameter it reads, if any.
    pub fn param(&self) -> Option<&str> {
        use MechanismDef as M;
        match self {
            M::Service { param, .. }
            | M::Transfer { param, .. }
            | M::HousingSupply { param }
            | M::Zoning { param, .. }
            | M::RentCap { param, .. }
            | M::SectorSubsidy { param, .. }
            | M::WageFloor { param }
            | M::HazardProtection { param, .. }
            | M::LocalTaxRate { param }
            | M::SectorCapacity { param, .. } => Some(param),
            M::Revenue { param, .. } => param.as_deref(),
            M::Borrow { .. } => None,
        }
    }

    /// The direction of its effect (−1 for a negative coefficient, else 1).
    pub fn sign(&self) -> f64 {
        use MechanismDef as M;
        let v = match self {
            M::Service { per_unit, .. }
            | M::Zoning { per_unit, .. }
            | M::HazardProtection { per_unit, .. }
            | M::SectorCapacity { per_unit, .. } => *per_unit,
            M::SectorSubsidy { elasticity, .. } => *elasticity,
            _ => 1.0,
        };
        if v < 0.0 { -1.0 } else { 1.0 }
    }

    /// Its Fluent-free type tag, as content writes it.
    pub fn key(&self) -> &'static str {
        use MechanismDef as M;
        match self {
            M::Service { .. } => "service",
            M::Transfer { .. } => "transfer",
            M::HousingSupply { .. } => "housing_supply",
            M::Zoning { .. } => "zoning",
            M::RentCap { .. } => "rent_cap",
            M::SectorSubsidy { .. } => "sector_subsidy",
            M::WageFloor { .. } => "wage_floor",
            M::HazardProtection { .. } => "hazard_protection",
            M::LocalTaxRate { .. } => "local_tax_rate",
            M::Borrow { .. } => "borrow",
            M::Revenue { .. } => "revenue",
            M::SectorCapacity { .. } => "sector_capacity",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CostDef {
    /// Fixed annual cost per resident (price level 1).
    #[serde(default)]
    pub per_capita: f64,
    /// Annual cost per resident per parameter unit.
    #[serde(default)]
    pub per_capita_per_unit: f64,
    /// Annual cost per parameter unit (e.g. per dwelling).
    #[serde(default)]
    pub per_unit: f64,
    /// The parameter the per-unit costs multiply.
    #[serde(default)]
    pub param: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyTemplate {
    pub id: ContentKey,
    pub name_key: String,
    pub desc_key: String,
    pub tier: TierKey,
    pub category: BudgetCategory,
    pub params: Vec<ParamDef>,
    pub cost: CostDef,
    pub mechanisms: Vec<MechanismDef>,
    pub implementation_months: u16,
    /// Value alignment by attitude key (DEC-025 anticipated channel, DEC-075).
    /// Attitudes left out weigh 0.
    pub signature: BTreeMap<String, f64>,
    /// `signature` resolved to a dense vector in attitude-registry order at
    /// load, so hot loops never look up keys. Not part of the content format.
    #[serde(skip)]
    #[schemars(skip)]
    pub signature_weights: Vec<f64>,
    /// Fluent keys for at least one secondary effect (REQ-006).
    pub secondary_keys: Vec<String>,
    /// What backs each secondary line (audit fix 8): a mechanism or side
    /// effect of this template by its type (`rent_cap`, `grievance`), `cost`
    /// (its money), or `measured:<metric>` (a side effect the audit measures
    /// on twins, `SECONDARY_MEASURED`). Validation requires one for every
    /// key, so a claim with nothing behind it cannot return unnoticed.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secondary_backing: BTreeMap<String, String>,
    /// Simulated downsides (DEC-073): every policy costs something besides money.
    #[serde(default)]
    pub side_effects: Vec<SideEffectDef>,
    /// Power required to propose it.
    pub requires: Power,
    /// A situational policy (DEC-077 §3): on the table (offered, and
    /// draftable by the player or an NPC) only while this trigger holds in
    /// the town. A bill already drafted or a law already passed stays.
    /// Always available without one.
    #[serde(default)]
    pub unlock: Option<SituationTrigger>,
    /// Retired (DEC-092 §2.19): not offered and not draftable by anyone;
    /// bills and laws already made continue and saves that have them load.
    #[serde(default, skip_serializing_if = "is_false")]
    pub retired: bool,
}

/// Side effects a secondary line may cite as `measured:<metric>`: metrics
/// the policy audit measures on twins (`mandate-cli audit policy`, `twins`).
pub const SECONDARY_MEASURED: &[&str] = &[
    "rent_pct",
    "population_pct",
    "labour_force_pct",
    "starts_index",
    "spread_bp",
    "jobs_pct",
    "jobs_agriculture_pct",
    "jobs_construction_pct",
    "jobs_knowledge_pct",
    "jobs_manufacturing_pct",
    "jobs_public_pct",
    "jobs_resources_pct",
    "jobs_retail_pct",
    "jobs_tourism_pct",
];

// --- Situations (DEC-077 §3; reused by DEC-092 policy-family triggers) ------------

/// A trigger on a jurisdiction's situation: it holds while every `all`
/// condition holds and, when any are given, one of the `any` conditions
/// does. Situational policies' unlocks and policy families' triggers and
/// expiry (DEC-092).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SituationTrigger {
    /// Every one of these holds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all: Vec<crate::condition::Condition>,
    /// At least one of these holds, when any are given (the one condition
    /// language, `condition.rs`).
    #[serde(default)]
    pub any: Vec<crate::condition::Condition>,
    /// Fluent key naming the circumstances, e.g. `unlock-closure`.
    pub reason_key: String,
}

/// A simulated downside of a policy (DEC-073). The amount is `per_unit`
/// times the named parameter (or 1 without one), scaled by implementation
/// progress, funding, and pilot size like the mechanisms.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SideEffectDef {
    /// Disruption while the programme is being built (roadworks, construction):
    /// lowers the town's appeal and residents' approval until complete.
    Disruption {
        per_unit: f64,
        #[serde(default)]
        param: Option<String>,
    },
    /// A lasting grievance among a group the policy hurts materially (approval
    /// of the government, and of the sponsor). Negative amounts are relief.
    Grievance {
        #[serde(default)]
        class: Option<ClassKey>,
        /// Only adults in this age band or older (0 = 18–29 … 3 = 65+).
        #[serde(default)]
        min_age_band: u8,
        per_unit: f64,
        #[serde(default)]
        param: Option<String>,
    },
    /// Changes how much a class works (share of its labour force).
    LabourSupply {
        class: ClassKey,
        per_unit: f64,
        #[serde(default)]
        param: Option<String>,
    },
}

impl SideEffectDef {
    pub fn param(&self) -> Option<&str> {
        match self {
            SideEffectDef::Disruption { param, .. }
            | SideEffectDef::Grievance { param, .. }
            | SideEffectDef::LabourSupply { param, .. } => param.as_deref(),
        }
    }
    /// Fluent key suffix describing the effect (`sidefx-…`).
    pub fn key(&self) -> &'static str {
        match self {
            SideEffectDef::Disruption { .. } => "disruption",
            SideEffectDef::Grievance { .. } => "grievance",
            SideEffectDef::LabourSupply { .. } => "labour_supply",
        }
    }
}
