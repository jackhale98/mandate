//! Content types. These Rust types are the single source of truth; JSON
//! Schemas are generated from them (DEC-053).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stable content identifier, e.g. `sector.manufacturing`.
pub type ContentKey = String;

/// One content file. The `kind` field selects the variant.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContentFile {
    /// Pack manifest (exactly one per pack).
    Pack(PackManifest),
    /// Tuning parameters (REQ-005).
    Tuning { value: Tuning },
    /// Economic sectors (DEC-019).
    Sectors { items: Vec<SectorDef> },
    /// Name-generation pools for places and people.
    NamePools { items: Vec<NamePoolDef> },
    /// Economy and demography parameters (REQ-005).
    Economy { value: EconomyParams },
    /// A constitutional archetype built from primitives (DEC-017).
    Constitution { value: ConstitutionDef },
    /// Policy templates (DEC-024, DEC-026).
    Policies { items: Vec<PolicyTemplate> },
}

// --- Constitution primitives (DEC-017) -----------------------------------------

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TierKey {
    Local,
    Regional,
    National,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Power {
    ProposeOrdinance,
    ApproveOrdinance,
    Veto,
    Override,
    ProposeBudget,
    ApproveBudget,
    Appoint,
    SetLocalTaxes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum DecisionRule {
    Majority,
    Supermajority { num: u8, den: u8 },
    Unanimity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VotingRule {
    Plurality,
    TwoRound,
    RankedChoice,
    ListPr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Constituency {
    /// One seat per generated ward / district.
    Ward,
    /// The whole jurisdiction.
    AtLarge,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Selection {
    Election {
        rule: VotingRule,
        constituency: Constituency,
    },
    AppointedBy {
        office: String,
    },
    ElectedByBody {
        body: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BodyDef {
    pub id: String,
    pub name_key: String,
    pub selection: Selection,
    pub term_years: u8,
    pub decision: DecisionRule,
    pub powers: Vec<Power>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OfficeDef {
    pub id: String,
    pub name_key: String,
    pub selection: Selection,
    pub term_years: u8,
    #[serde(default)]
    pub term_limit: Option<u8>,
    pub powers: Vec<Power>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StageDef {
    /// A vote of a body under a decision rule.
    Vote { body: String, rule: DecisionRule },
    /// An office signs or vetoes; a veto may be overridden by a body.
    Sign {
        office: String,
        override_body: Option<String>,
        override_rule: Option<DecisionRule>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcedureDef {
    /// `ordinance` or `budget`.
    pub id: String,
    /// Days between introduction and the first stage (deliberation).
    pub deliberation_days: u16,
    pub stages: Vec<StageDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TierDef {
    pub tier: TierKey,
    pub bodies: Vec<BodyDef>,
    pub offices: Vec<OfficeDef>,
    pub procedures: Vec<ProcedureDef>,
    /// Sub-national balanced-budget rule.
    #[serde(default)]
    pub balanced_budget: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstitutionDef {
    pub id: ContentKey,
    pub name_key: String,
    pub tiers: Vec<TierDef>,
}

impl ConstitutionDef {
    pub fn tier(&self, t: TierKey) -> Option<&TierDef> {
        self.tiers.iter().find(|x| x.tier == t)
    }
}

impl TierDef {
    pub fn procedure(&self, id: &str) -> Option<&ProcedureDef> {
        self.procedures.iter().find(|p| p.id == id)
    }
    pub fn body(&self, id: &str) -> Option<&BodyDef> {
        self.bodies.iter().find(|b| b.id == id)
    }
    pub fn office(&self, id: &str) -> Option<&OfficeDef> {
        self.offices.iter().find(|o| o.id == id)
    }
    /// Which body or office holds a power (first match; REQ-009: no archetype names).
    pub fn holder_of(&self, p: Power) -> Option<&str> {
        self.offices
            .iter()
            .find(|o| o.powers.contains(&p))
            .map(|o| o.id.as_str())
            .or_else(|| {
                self.bodies
                    .iter()
                    .find(|b| b.powers.contains(&p))
                    .map(|b| b.id.as_str())
            })
    }
}

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
    /// Subsidy as a share of a sector's wage bill; jobs respond with `elasticity`.
    SectorSubsidy {
        sector: SectorKind,
        param: String,
        elasticity: f64,
    },
    /// Wage floor as a share above the low-class market wage.
    WageFloor { param: String },
    /// Reduces damage from a hazard by `per_unit` × param (capped at 0.9).
    HazardProtection {
        hazard: HazardKey,
        param: String,
        per_unit: f64,
    },
    /// Changes the local property tax by `param` percentage points of value.
    LocalTaxRate { param: String },
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
    /// Value alignment over the 8 attitude dimensions (DEC-025 anticipated channel).
    pub signature: [f64; 8],
    /// Fluent keys for at least one secondary effect (REQ-006).
    pub secondary_keys: Vec<String>,
    /// Power required to propose it.
    pub requires: Power,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    pub id: ContentKey,
    pub version: String,
    /// Fluent key for the pack's display name.
    pub name_key: String,
    /// Load order: lower first. The base pack is 0.
    #[serde(default)]
    pub load_order: i32,
}

/// Data-driven tuning parameters (REQ-005).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tuning {
    /// Standard day-part slots per turn (Morning, Afternoon, Evening).
    pub standard_slots: u8,
    /// Whether the optional Late Night slot is offered.
    pub late_night_enabled: bool,
    /// Turn lengths in days by phase (DEC-012).
    pub turn_days: TurnDays,
    /// Years of NPC-only simulation before the player starts (DEC-022).
    pub burn_in_years: u8,
    /// Calendar year the history begins.
    pub start_year: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TurnDays {
    pub campaign: u8,
    /// Days before an election that switch to daily turns.
    pub final_stretch: u8,
    pub governing_local: u8,
    pub governing_regional: u8,
    pub governing_national: u8,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SectorClass {
    Primary,
    Secondary,
    Tertiary,
    Public,
}

/// Behavioural kind of a sector; exactly one sector per kind (validated).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SectorKind {
    Agriculture,
    Resources,
    Manufacturing,
    Construction,
    Retail,
    Tourism,
    Knowledge,
    Public,
}

impl SectorKind {
    pub const ALL: [SectorKind; 8] = [
        SectorKind::Agriculture,
        SectorKind::Resources,
        SectorKind::Manufacturing,
        SectorKind::Construction,
        SectorKind::Retail,
        SectorKind::Tourism,
        SectorKind::Knowledge,
        SectorKind::Public,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SectorDef {
    pub id: ContentKey,
    pub name_key: String,
    pub class: SectorClass,
    pub kind: SectorKind,
    /// Relative weight of this sector's base presence in a generic economy.
    pub base_weight: f64,
    /// Mean annual wage per worker at price level 1.0 (currency units).
    pub base_wage: f64,
    /// Elasticity of job demand to the global demand cycle.
    pub cyclicality: f64,
    /// Annual productivity growth (fraction).
    pub productivity_growth: f64,
}

/// Economy and demography parameters (REQ-005).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EconomyParams {
    pub inflation_target: f64,
    pub natural_unemployment: f64,
    pub neutral_real_rate: f64,
    /// Monthly probability that a global recession begins.
    pub recession_monthly_chance: f64,
    /// Peak fall in global demand during a recession, [min, max].
    pub recession_depth: [f64; 2],
    pub recession_months: [u16; 2],
    /// Monthly volatility of commodity prices (log scale).
    pub commodity_volatility: f64,
    /// Births per adult aged 18–44 per year.
    pub birth_rate: f64,
    /// Annual death rates: children, 18–29, 30–44, 45–64, 65+.
    pub death_rates: [f64; 5],
    /// Monthly share of mobile adults who reconsider where they live.
    pub migration_rate: f64,
    pub target_vacancy: f64,
    pub construction_lag_months: u8,
    /// Average dwelling price at price level 1.0.
    pub base_dwelling_price: f64,
    pub taxes: TaxParams,
    /// Unemployment benefit and pension as a share of the mean wage.
    pub unemployment_replacement: f64,
    pub pension_replacement: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaxParams {
    /// Annual property tax as a share of dwelling value (local tier).
    pub property: f64,
    /// Sales tax on consumption (regional tier).
    pub sales: f64,
    /// Effective income tax by class: low, middle, high (national tier).
    pub income: [f64; 3],
    /// Tax on business output (national tier).
    pub business: f64,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum NamePoolKind {
    Place,
    Region,
    GivenName,
    FamilyName,
    Neighborhood,
}

/// Syllable-based name generator pool.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NamePoolDef {
    pub id: ContentKey,
    pub kind: NamePoolKind,
    pub prefixes: Vec<String>,
    #[serde(default)]
    pub middles: Vec<String>,
    pub suffixes: Vec<String>,
    /// Optional whole-word list mixed in with generated names.
    #[serde(default)]
    pub words: Vec<String>,
}

// ---- Position-preserving file wrappers -------------------------------------
// serde's internally tagged enums buffer their input and lose line numbers, so
// loading probes `kind` first, then parses into one of these structs directly.
// `ContentFile` (above) remains the schema source of truth.

#[derive(Deserialize)]
pub(crate) struct KindProbe {
    pub kind: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackFile {
    #[allow(dead_code)]
    pub kind: String,
    pub id: ContentKey,
    pub version: String,
    pub name_key: String,
    #[serde(default)]
    pub load_order: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ValueFile<T> {
    #[allow(dead_code)]
    pub kind: String,
    pub value: T,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemsFile<T> {
    #[allow(dead_code)]
    pub kind: String,
    pub items: Vec<T>,
}

pub(crate) fn parse_content_file(json: &str) -> Result<ContentFile, serde_json::Error> {
    let probe: KindProbe = serde_json::from_str(json)?;
    Ok(match probe.kind.as_str() {
        "pack" => {
            let p: PackFile = serde_json::from_str(json)?;
            ContentFile::Pack(PackManifest {
                id: p.id,
                version: p.version,
                name_key: p.name_key,
                load_order: p.load_order,
            })
        }
        "tuning" => ContentFile::Tuning {
            value: serde_json::from_str::<ValueFile<Tuning>>(json)?.value,
        },
        "sectors" => ContentFile::Sectors {
            items: serde_json::from_str::<ItemsFile<SectorDef>>(json)?.items,
        },
        "name_pools" => ContentFile::NamePools {
            items: serde_json::from_str::<ItemsFile<NamePoolDef>>(json)?.items,
        },
        "economy" => ContentFile::Economy {
            value: serde_json::from_str::<ValueFile<EconomyParams>>(json)?.value,
        },
        "constitution" => ContentFile::Constitution {
            value: serde_json::from_str::<ValueFile<ConstitutionDef>>(json)?.value,
        },
        "policies" => ContentFile::Policies {
            items: serde_json::from_str::<ItemsFile<PolicyTemplate>>(json)?.items,
        },
        other => {
            use serde::de::Error;
            return Err(serde_json::Error::custom(format!(
                "unknown content kind `{other}`"
            )));
        }
    })
}
