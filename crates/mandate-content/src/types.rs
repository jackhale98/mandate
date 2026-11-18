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
    /// Parties, elections, polls, campaigns, and NPC AI (DEC-027 to DEC-030, DEC-071).
    Politics { value: Box<PoliticsParams> },
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
    /// When this tier's elections happen (all elected bodies and offices share it).
    #[serde(default)]
    pub elections: Option<ElectionCalendarDef>,
}

/// Whether a tier votes on the same day as a higher tier (DEC-071: turnout).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ElectionTiming {
    Concurrent,
    Standalone,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElectionCalendarDef {
    pub month: u8,
    pub day: u8,
    /// Elections fall in years where `year % term_years == cycle_offset`.
    pub cycle_offset: u8,
    pub campaign_weeks: u8,
    pub timing: ElectionTiming,
    /// Party labels appear on the ballot (otherwise a nonpartisan race).
    pub partisan_ballot: bool,
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
    /// Simulated downsides (DEC-073): every policy costs something besides money.
    #[serde(default)]
    pub side_effects: Vec<SideEffectDef>,
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionCost {
    pub slots: u8,
    #[serde(default)]
    pub health: f64,
}

impl Tuning {
    pub fn action_cost(&self, action: &str) -> ActionCost {
        self.action_costs
            .get(action)
            .copied()
            .unwrap_or(ActionCost {
                slots: 1,
                health: 0.0,
            })
    }
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
    /// Time and health each action costs (DEC-073). Missing actions cost one
    /// slot and no health; negative health is recovery.
    #[serde(default)]
    pub action_costs: std::collections::BTreeMap<String, ActionCost>,
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

// --- Politics parameters (DEC-027 to DEC-030, DEC-071) -----------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PoliticsParams {
    /// Parties generated per world under plurality rules (two majors plus minors).
    pub party_count: u8,
    pub party_names: PartyNameParts,
    pub vote: VoteParams,
    pub turnout: TurnoutParams,
    pub polls: PollParams,
    pub campaign: CampaignParams,
    pub party_id: PartyIdParams,
    pub ai: AiParams,
}

// --- NPC utility AI (DEC-030, DEC-071 §8) ---------------------------------------

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AiAction {
    Canvass,
    TownHall,
    Advertise,
    Fundraise,
    Lobby,
    ProposeBill,
    Rest,
}

/// Normalised (0..1) inputs the engine computes for an NPC each decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiInput {
    /// 1 while the NPC is a candidate in an open race.
    Campaigning,
    /// 0 at the start of the campaign, 1 on election eve.
    ElectionNear,
    /// 1 for a dead heat (from polls when published, else party strength).
    RaceCloseness,
    /// 1 − the NPC's mean awareness among the race's voters.
    Unknown,
    /// Campaign funds relative to a few weeks of advertising.
    Funds,
    /// Strength of the NPC's view on the most important pending bill.
    BillStake,
    /// 1 when that bill's vote is on a knife edge.
    BillCloseness,
    /// Appeal of the NPC's best policy idea (own values and constituents).
    Agenda,
    /// 1 if the NPC has no bill of their own pending.
    NoOwnBill,
    /// 1 when the council has few bills pending.
    CouncilFree,
    /// Always 1.
    Constant,
}

/// Response curve mapping an input (0..1) to a score (0..1).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Curve {
    /// m·x + b, clamped to 0..1.
    Linear { m: f64, b: f64 },
    /// 1 / (1 + e^(−k (x − x0))).
    Logistic { k: f64, x0: f64 },
    /// `lo` below `x`, `hi` at or above.
    Step { x: f64, lo: f64, hi: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Consideration {
    pub input: AiInput,
    pub curve: Curve,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiActionDef {
    pub action: AiAction,
    pub weight: f64,
    pub considerations: Vec<Consideration>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiProfileMult {
    pub action: AiAction,
    pub mult: f64,
}

/// Strategic profile (brief §9.4): multiplies action scores.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiProfile {
    pub id: String,
    pub mults: Vec<AiProfileMult>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiParams {
    /// Choose by weighted chance among the best `top_n` scores.
    pub top_n: u8,
    pub actions: Vec<AiActionDef>,
    pub profiles: Vec<AiProfile>,
}

/// Words for generated party names: an attitude-led adjective plus a noun.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PartyNameParts {
    pub nouns: Vec<String>,
    pub by_attitude: Vec<AttitudeWords>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttitudeWords {
    /// An attitude key, e.g. `redistribution`.
    pub attitude: String,
    pub pro: Vec<String>,
    pub anti: Vec<String>,
}

/// Per-cohort vote utility (DEC-028, DEC-071 §1–2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VoteParams {
    pub issue_weight: f64,
    /// Directional share β at low and high polarisation.
    pub beta_low: f64,
    pub beta_high: f64,
    /// Mean attitude standard deviation mapped to β_low and β_high.
    pub polarization_low: f64,
    pub polarization_high: f64,
    pub party_weight: f64,
    pub valence_weight: f64,
    pub retrospective_weight: f64,
    /// Retrospective weight multiplier for executive offices.
    pub executive_multiplier: f64,
    pub home_ward_bonus: f64,
    /// Share of retrospective blame partisans forgive their own side.
    pub partisan_discount: f64,
    /// Awareness floor for an incumbent within their constituency.
    pub incumbent_awareness: f64,
    /// Awareness of a fresh challenger before campaigning.
    pub challenger_awareness: f64,
    /// Per-candidate race shock (utility units).
    pub race_shock_sd: f64,
    /// National swing per party shared by every race in an election.
    pub national_swing_sd: f64,
    /// Wasted-vote penalty for candidates whose party is weak locally (Duverger).
    pub strategic_weight: f64,
    /// Minor parties contest only where their attachment share reaches this.
    pub minor_party_threshold: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TurnoutParams {
    /// Turnout multiplier for a standalone local election.
    pub standalone: f64,
    pub concurrent: f64,
    /// Weight on last election's turnout (habit).
    pub habit: f64,
    /// Extra turnout from a fully mobilised neighbourhood.
    pub mobilisation_max: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PollParams {
    pub pollsters: Vec<String>,
    pub sample_size: u32,
    /// House-effect standard deviation (share points, 0..1 scale).
    pub house_sd: f64,
    /// Election-wide shared error on election eve (share, 0..1 scale).
    pub shared_sd: f64,
    /// Shared error multiplier at the start of the campaign.
    pub early_multiplier: f64,
    pub every_days: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CampaignParams {
    pub persuasion_half_life_days: f64,
    pub awareness_per_canvass: f64,
    pub persuasion_per_canvass: f64,
    pub mobilisation_per_canvass: f64,
    /// Each repeated contact in a neighbourhood is worth this fraction of the last.
    pub diminishing: f64,
    pub ad_cost: i64,
    pub awareness_per_ad: f64,
    /// NPC campaign effort in abstracted races (contacts per week per candidate).
    pub npc_contacts_per_week: f64,
}

/// Mover–stayer party attachment (DEC-071 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PartyIdParams {
    /// Share of adults attached to any party, by age band (young → old).
    pub attached_by_age: [f64; 4],
    /// Durable share of attachment.
    pub stayer_share: f64,
    /// Yearly pull of the mover share toward current evaluations.
    pub mover_rate: f64,
    /// Sharpness of attachment by ideological distance.
    pub sharpness: f64,
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
        "politics" => ContentFile::Politics {
            value: Box::new(serde_json::from_str::<ValueFile<PoliticsParams>>(json)?.value),
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
