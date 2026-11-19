//! Content types. These Rust types are the single source of truth; JSON
//! Schemas are generated from them (DEC-053).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    /// Attitude dimensions: the issues voters hold views on (DEC-025, DEC-075).
    Attitudes { items: Vec<AttitudeDef> },
    /// Communities: a cohort dimension beside age and class (DEC-075).
    Communities { items: Vec<CommunityDef> },
    /// Voter concerns: what worries a town, and which issues it makes salient (DEC-078).
    Concerns { items: Vec<ConcernDef> },
    /// Name-generation pools for places and people.
    NamePools { items: Vec<NamePoolDef> },
    /// World scenarios: starting worlds as biases on the generator (DEC-078).
    Scenarios { items: Vec<ScenarioDef> },
    /// Place kinds: what the player can do and learn at each kind of place (DEC-079).
    Places { items: Vec<PlaceKindDef> },
    /// Economy and demography parameters (REQ-005).
    Economy { value: EconomyParams },
    /// A constitutional archetype built from primitives (DEC-017).
    Constitution { value: ConstitutionDef },
    /// Policy templates (DEC-024, DEC-026).
    Policies { items: Vec<PolicyTemplate> },
    /// Parties, elections, polls, campaigns, and NPC AI (DEC-027 to DEC-030, DEC-071).
    Politics { value: Box<PoliticsParams> },
    /// Opinion dynamics: how conditions and laws move approval (DEC-025, DEC-078).
    Opinion { value: OpinionParams },
    /// Player action effects and risks (DEC-073, DEC-078).
    Actions { value: Box<ActionParams> },
    /// Council leans, mayoral signatures, and lobbying odds (DEC-026, DEC-078).
    Council { value: CouncilParams },
    /// Records, Standing, Mandate, and name recognition (DEC-011, DEC-076, DEC-078).
    Career { value: CareerParams },
    /// NPC AI balance numbers beside the utility curves in `politics.ai` (DEC-030, DEC-078).
    Npc { value: NpcParams },
    /// Character trait tags with data-driven modifiers (DEC-031).
    Traits { items: Vec<TraitDef> },
    /// Occupations and roles of Named characters (DEC-018, DEC-031).
    Occupations { items: Vec<OccupationDef> },
    /// Kinds of live local crisis a town can have (DEC-077 §1).
    Crises { items: Vec<CrisisDef> },
    /// Character, memory, relationship, and town-character numbers (DEC-031, DEC-077).
    Characters { value: Box<CharacterParams> },
    /// The narrative director's pacing, fairness, and chance numbers (DEC-033).
    Director {
        value: Box<crate::narrative::DirectorParams>,
    },
    /// Storyteller profiles (DEC-033 addendum).
    Storytellers {
        items: Vec<crate::narrative::StorytellerDef>,
    },
    /// Overheard talk: what residents say at diners, markets, and union halls (DEC-079).
    Overheard { items: Vec<OverheardDef> },
    /// Dilemmas and requests: decisions where every option costs someone (DEC-077 §4).
    Dilemmas {
        items: Vec<crate::narrative::DilemmaDef>,
    },
    /// NPC goal templates (DEC-032).
    Goals {
        items: Vec<crate::narrative::GoalDef>,
    },
    /// Storyline templates (DEC-032).
    Storylines {
        items: Vec<crate::narrative::StorylineDef>,
    },
    /// Scene templates: short exchanges at a place (DEC-035, DEC-079).
    Scenes {
        items: Vec<crate::narrative::SceneDef>,
    },
    /// Outlet templates: newspapers, broadcasters, and social platforms (DEC-034).
    Outlets { items: Vec<crate::media::OutletDef> },
    /// Discovery, coverage, perception, responses, rumours, shady acts (DEC-034).
    Media {
        value: Box<crate::media::MediaParams>,
    },
    /// Investigations, charges, trials, and sentences (DEC-006 Legal Jeopardy).
    Justice {
        value: Box<crate::media::JusticeParams>,
    },
    /// Town layout v2 district types (DEC-090 D4).
    TownDistricts {
        items: Vec<crate::layout::TownDistrictDef>,
    },
    /// Town layout v2 terrain features (DEC-090 D4).
    TerrainFeatures {
        items: Vec<crate::layout::TerrainFeatureDef>,
    },
    /// Town layout v2 connectors: bridges, fords, viaducts, level crossings (DEC-090 D4).
    Connectors {
        items: Vec<crate::layout::ConnectorDef>,
    },
    /// Knowledge is earned (DEC-091): start profiles, the local option, the
    /// party primer, place introductions, and the effectiveness terms.
    Knowledge {
        value: Box<crate::knowledge::KnowledgeParams>,
    },
    /// Conversation topics (DEC-093): what the player can raise, with whom, where.
    TalkTopics {
        items: Vec<crate::conversation::TopicDef>,
    },
    /// Conversation lines: openers, memory quotes, substance and closers (DEC-093).
    TalkLines {
        items: Vec<crate::conversation::TalkLineDef>,
    },
    /// Conversation voices: who speaks how (DEC-093).
    TalkVoices {
        items: Vec<crate::conversation::VoiceDef>,
    },
    /// Conversation rules as numbers: cost, discretion, speaker knowledge (DEC-093).
    TalkParams {
        value: Box<crate::conversation::TalkParams>,
    },
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
        /// Content key of the sector, e.g. `sector.retail`.
        sector: ContentKey,
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
}

// --- Situations (DEC-077 §3; reused by DEC-092 policy-family triggers) ------------

/// A trigger on a town's situation: it holds while any of its conditions
/// does. Reusable wherever content needs "when this town is in this kind of
/// situation" (situational policies now; policy families later).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SituationTrigger {
    /// Any one of these holds.
    pub any: Vec<SituationCondition>,
    /// Fluent key naming the circumstances, e.g. `unlock-closure`.
    pub reason_key: String,
}

/// A typed condition on one town's live situation, read from engine state
/// (the town's character, its indicators, the event log, the economy).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SituationCondition {
    /// A live crisis of this kind (`crises.jsonc`), at least this severe.
    Crisis {
        kind: ContentKey,
        #[serde(default)]
        min_severity: f64,
    },
    /// An engine indicator of the town within [min, max].
    Indicator {
        indicator: Indicator,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// A plant closed in the town within this many months (the event log).
    PlantClosure { within_months: u16 },
    /// A disaster struck the town within this many months.
    Disaster { within_months: u16 },
    // --- Slot-relative (DEC-093 conversations; DEC-092 family slots). In a
    // conversation the slots are `speaker`, `about` and `player`; where no
    // slots are bound (a policy's unlock) these never hold. Each names the
    // Fluent arguments it provides when it holds; triggers ignore them.
    /// The person in the slot has one of these roles: occupation ids,
    /// `councillor` or `mayor`.
    Occupation {
        slot: String,
        occupations: Vec<ContentKey>,
    },
    /// The person in the slot has one of these traits.
    Trait {
        slot: String,
        traits: Vec<ContentKey>,
    },
    /// A personality axis of the person in the slot within [min, max] (0..1).
    Personality {
        slot: String,
        axis: String,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// The person in the slot holds this goal (DEC-032).
    Goal { slot: String, goal: ContentKey },
    /// Their opinion of the player within [min, max] (−100..100).
    Opinion {
        slot: String,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// Their trust in the player within [min, max] (0..1).
    Trust {
        slot: String,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// They remember the player doing one of these (memory kinds; the most
    /// salient): `$memory_key`, `$subject_key`, `$year`.
    Remembers { slot: String, kinds: Vec<String> },
    /// The player owes them a favour (or, `owed_to_player`, the reverse): `$n`.
    Favour {
        slot: String,
        #[serde(default)]
        owed_to_player: bool,
    },
    /// They are (or are not) the town's power broker. When they are:
    /// `$broker`, `$broker_place`, `$stake_key`.
    IsBroker {
        slot: String,
        #[serde(default = "yes")]
        is: bool,
    },
    /// They are (or are not) the player's rival. When they are: `$rival`.
    IsRival {
        slot: String,
        #[serde(default = "yes")]
        is: bool,
    },
    /// The speaker would tell the player at least `min` about the subject
    /// (what they know, capped by discretion): the subject's arguments.
    Tells {
        subject: crate::conversation::SubjectRef,
        min: crate::knowledge::Level,
    },
    /// The player's ledger (DEC-091) holds the subject within [min, max]:
    /// the subject's arguments.
    PlayerKnows {
        subject: crate::conversation::SubjectRef,
        #[serde(default)]
        min: Option<crate::knowledge::Level>,
        #[serde(default)]
        max: Option<crate::knowledge::Level>,
    },
    /// Any one of these holds (the first that does gives the arguments).
    Any { of: Vec<SituationCondition> },
}

fn yes() -> bool {
    true
}

impl SituationCondition {
    /// Whether the condition reads a bound slot (it never holds where none is).
    pub fn is_slot_relative(&self) -> bool {
        use SituationCondition as U;
        match self {
            U::Crisis { .. }
            | U::Indicator { .. }
            | U::PlantClosure { .. }
            | U::Disaster { .. } => false,
            U::PlayerKnows { subject, .. } => matches!(
                subject,
                crate::conversation::SubjectRef::Person { .. }
                    | crate::conversation::SubjectRef::Proposal { .. }
                    | crate::conversation::SubjectRef::PolicySupport { .. }
                    | crate::conversation::SubjectRef::HoodConcerns { .. }
            ),
            U::Any { of } => of.iter().any(|c| c.is_slot_relative()),
            _ => true,
        }
    }
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

/// Behaviour driver of a sector (DEC-075): the closed set of economic
/// behaviours implemented in code. Sectors themselves are content; several
/// sectors may share a driver, and a new driver needs a code change.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SectorDriver {
    /// Farming and other land-based output: fertility, crop prices, water.
    LandBased,
    /// Mining, energy, timber: commodity prices and finite stocks; plant closures.
    Extraction,
    /// Manufacturing: national competitiveness and unit labour costs; plant closures.
    Industrial,
    /// Construction: housing starts and public spending.
    Building,
    /// Retail and local services: household consumption.
    LocalServices,
    /// Tourism and hospitality: scenery, coasts, disasters.
    Visitor,
    /// Knowledge and technology: productivity follows the technology trend.
    Knowledge,
    /// Public employment: funded by government spending.
    Public,
}

impl SectorDriver {
    pub const ALL: [SectorDriver; 8] = [
        SectorDriver::LandBased,
        SectorDriver::Extraction,
        SectorDriver::Industrial,
        SectorDriver::Building,
        SectorDriver::LocalServices,
        SectorDriver::Visitor,
        SectorDriver::Knowledge,
        SectorDriver::Public,
    ];
    /// Drivers the base economy cannot run without (validated: at least one
    /// sector each). The others are optional.
    pub const REQUIRED: [SectorDriver; 3] = [
        SectorDriver::Building,
        SectorDriver::LocalServices,
        SectorDriver::Public,
    ];
}

/// An attitude dimension (DEC-075): an issue every cohort, politician, and
/// party holds a position on, in [-1, 1]. Registry order is load order (files
/// by path, then item order); the engine stores attitudes densely in that order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttitudeDef {
    /// Content id, e.g. `attitude.redistribution`.
    pub id: ContentKey,
    /// Short key used in policy signatures, party-name words, and Fluent keys
    /// (`conviction-pro-<key>`, `conviction-anti-<key>`), e.g. `redistribution`.
    pub key: String,
    /// How starting opinions are seeded (DEC-018).
    #[serde(default)]
    pub seed: AttitudeSeed,
}

/// Starting-opinion model for one attitude. A cohort's position is
/// `by_class + by_age + by_neighborhood + regional offset + noise`, clamped to
/// [-1, 1]. Each region draws its own cultural offset once per attitude.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttitudeSeed {
    /// Lean by income class: `[low, middle, high]`.
    #[serde(default)]
    pub by_class: [f64; 3],
    /// Lean by age band: `[18–29, 30–44, 45–64, 65+]`.
    #[serde(default)]
    pub by_age: [f64; 4],
    /// Lean by neighbourhood kind.
    #[serde(default)]
    pub by_neighborhood: NeighborhoodLean,
    /// Standard deviation of each region's cultural offset.
    #[serde(default = "default_regional_sd")]
    pub regional_sd: f64,
    /// Standard deviation of each cohort's individual variation.
    #[serde(default = "default_noise_sd")]
    pub noise_sd: f64,
}

fn default_regional_sd() -> f64 {
    0.14
}
fn default_noise_sd() -> f64 {
    0.08
}

impl Default for AttitudeSeed {
    fn default() -> Self {
        AttitudeSeed {
            by_class: [0.0; 3],
            by_age: [0.0; 4],
            by_neighborhood: NeighborhoodLean::default(),
            regional_sd: default_regional_sd(),
            noise_sd: default_noise_sd(),
        }
    }
}

/// A value per neighbourhood kind (omitted kinds are 0).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NeighborhoodLean {
    #[serde(default)]
    pub core: f64,
    #[serde(default)]
    pub inner: f64,
    #[serde(default)]
    pub waterfront: f64,
    #[serde(default)]
    pub industrial: f64,
    #[serde(default)]
    pub suburb: f64,
    #[serde(default)]
    pub rural: f64,
}

/// A community (DEC-075): a cohort dimension beside age band and income
/// class. Every neighbourhood holds age × class cohorts for each community.
/// Registry order is load order (files by path, then item order). The base
/// pack ships one community, so the dimension costs nothing until an
/// expansion adds another (an Indigenous nation, an immigrant community, a
/// linguistic minority).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommunityDef {
    /// Content id, e.g. `community.general`.
    pub id: ContentKey,
    /// Fluent key, e.g. `community-general`.
    pub name_key: String,
    /// How much of each neighbourhood's population belongs to this community.
    /// Exactly one community omits it: that community takes the remainder
    /// (in the base pack, `community.general` is everyone).
    #[serde(default)]
    pub share: Option<CommunityShare>,
    /// Added to the seeded attitudes of this community's cohorts, by attitude
    /// key (omitted = 0).
    #[serde(default)]
    pub attitude_offsets: BTreeMap<String, f64>,
    /// `attitude_offsets` resolved to a dense vector in attitude-registry
    /// order at load. Not part of the content format.
    #[serde(skip)]
    #[schemars(skip)]
    pub attitude_offset_weights: Vec<f64>,
    /// Added to the seeded turnout propensity of this community's cohorts.
    #[serde(default)]
    pub turnout_offset: f64,
    /// Fraction of international immigrants who join this community (the
    /// remainder community takes whatever the others leave).
    #[serde(default)]
    pub immigration_share: f64,
}

/// Share model for a community: in a neighbourhood the community's share of
/// every age × class group is `base + by_neighborhood[kind] + regional
/// offset`, clamped to [0, 1]. Each region draws its offset once (sd
/// `regional_sd`). Where the communities with a share model add up to more
/// than 1, they are scaled down to 1; the remainder community takes the rest.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommunityShare {
    /// Share everywhere, before the neighbourhood and regional terms.
    #[serde(default)]
    pub base: f64,
    /// Added by neighbourhood kind (omitted kinds are 0).
    #[serde(default)]
    pub by_neighborhood: NeighborhoodLean,
    /// Standard deviation of each region's offset (0 = none).
    #[serde(default)]
    pub regional_sd: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SectorDef {
    pub id: ContentKey,
    pub name_key: String,
    /// Fluent key of the sector as residents name it in talk (a noun phrase
    /// such as "the factories"); the name is used without one.
    #[serde(default)]
    pub talk_key: Option<String>,
    pub class: SectorClass,
    /// Which coded behaviour this sector follows.
    pub driver: SectorDriver,
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
    /// Annual structural trends of a world without a scenario bias:
    /// technology growth, and the drift of manufacturing competitiveness.
    pub tech_growth: f64,
    pub manufacturing_trend: f64,
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

// --- Indicators and concerns (DEC-078 decision 3) --------------------------------

/// Engine-provided measures of a municipality, recomputed monthly. Concerns
/// read them; situational policies (DEC-077) will use them as unlock
/// conditions. Adding one is a code extension point (one variant, one
/// computation in `mandate_sim::concerns::indicator`, and a test).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Indicator {
    /// Unemployment rate (0..1).
    Unemployment,
    /// Change in the unemployment rate over the last year (points, 0..1 scale).
    UnemploymentRise,
    /// Annual rent of an average home as a share of a typical (middle-class) income.
    RentBurden,
    /// The region's pollution index (0 clean, about 0.2 at start, up to 3).
    Pollution,
    /// Annual property tax on an average home as a share of a typical income.
    TaxBurden,
    /// The town's debt as a share of its annual budget.
    BudgetStress,
    /// 1 just after a disaster, fading to 0 over two years.
    RecentDisaster,
    /// Share of the town's jobs lost over the last year (0 when jobs grew):
    /// a plant closure or a failing industry.
    JobLoss,
}

impl Indicator {
    pub const ALL: [Indicator; 8] = [
        Indicator::Unemployment,
        Indicator::UnemploymentRise,
        Indicator::RentBurden,
        Indicator::Pollution,
        Indicator::TaxBurden,
        Indicator::BudgetStress,
        Indicator::RecentDisaster,
        Indicator::JobLoss,
    ];
}

/// A voter concern (DEC-078 decision 3): something that worries a town,
/// read from an indicator, and the issues (attitudes) it makes voters weigh.
/// Registry order is load order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConcernDef {
    /// Content id, e.g. `concern.jobs`.
    pub id: ContentKey,
    /// Fluent key of the concern's name, e.g. `concern-jobs`.
    pub name_key: String,
    pub indicator: Indicator,
    /// The attitudes behind the concern, by key. The magnitude is how much a
    /// worried town weighs that issue; the sign is the side worried voters
    /// lean toward (used to rank concerns by neighbourhood).
    pub attitudes: BTreeMap<String, f64>,
    /// `attitudes` resolved to a dense vector in attitude-registry order at
    /// load. Not part of the content format.
    #[serde(skip)]
    #[schemars(skip)]
    pub attitude_weights: Vec<f64>,
    /// How the indicator's value becomes the concern's level.
    pub level: ConcernLevel,
}

/// `level = clamp(baseline + slope × indicator, min, max)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConcernLevel {
    pub baseline: f64,
    pub slope: f64,
    pub min: f64,
    pub max: f64,
}

impl ConcernLevel {
    pub fn of(&self, indicator: f64) -> f64 {
        (self.baseline + self.slope * indicator).clamp(self.min, self.max)
    }
}

// --- Overheard talk (DEC-079 information services) -------------------------------

/// One thing residents may be heard saying at a diner, market, or union hall
/// (the "overheard" service). Each week every such place draws a few lines
/// that hold for its neighbourhood: lines about the neighbourhood's top
/// concerns, and opinions and news grounded in facts. Registry order is
/// load order. Fluent arguments always include `$town` and `$hood`; each
/// condition adds its own (see `HeardCondition`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OverheardDef {
    /// Content id, e.g. `overheard.jobs.hiring`.
    pub id: ContentKey,
    /// Fluent key of the line.
    pub key: String,
    /// Heard only where this concern is among the neighbourhood's top three
    /// (a concern id); the stronger the concern, the likelier the line.
    #[serde(default)]
    pub concern: Option<ContentKey>,
    /// Every condition must hold.
    #[serde(default)]
    pub when: Vec<HeardCondition>,
    /// Relative chance among the lines that hold (default 1).
    #[serde(default = "one_f64")]
    pub weight: f64,
}

fn one_f64() -> f64 {
    1.0
}

/// Water a town or neighbourhood can have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WaterFeature {
    /// The town is on a river.
    River,
    /// The town is on the coast.
    Coast,
    /// A river, the coast, or a waterfront neighbourhood.
    Any,
}

/// Whose scandal a line is about.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HeardSubject {
    #[default]
    Anyone,
    Player,
    Rival,
}

/// A fact a line needs. Each names the Fluent arguments it provides.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum HeardCondition {
    /// The town (or neighbourhood) has this water.
    Water { feature: WaterFeature },
    /// No water at all: no river, no coast, no waterfront.
    Dry,
    /// A live crisis in the town (of this kind, if given): `$crisis_key`.
    Crisis {
        #[serde(default)]
        kind: Option<ContentKey>,
    },
    /// A plant closed in the town within this many months: `$industry_key`
    /// (the closed plant's sector), `$jobs`.
    PlantClosure { within_months: u16 },
    /// A disaster struck the town within this many months.
    Disaster { within_months: u16 },
    /// The town has a defining industry: `$industry_key`.
    Industry,
    /// The town has a power broker: `$broker`.
    Broker,
    /// The player has a named rival: `$rival`.
    Rival,
    /// The player's name recognition in the neighbourhood within [min, max]
    /// (0..1): `$player`.
    PlayerAwareness {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// Approval of the player among neighbourhood residents (−1..1), within
    /// [min, max]; only where at least a fifth know them: `$player`.
    PlayerApproval {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// The rival's record of visible work within [min, max] (0..1): `$rival`.
    RivalRecord {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// A scandal running in the town (DEC-034) about this subject:
    /// `$subject` (their name), `$fact_key`.
    Scandal {
        #[serde(default)]
        about: HeardSubject,
    },
    /// A story about the town within the last eight weeks: `$outlet_key`
    /// (with `$place`), `$headline_key`, and the headline's own arguments.
    Headline,
    /// The player's next local election is within this many days.
    ElectionWithin { days: u32 },
    /// Any one of these holds (the first that does gives the arguments).
    Any { of: Vec<HeardCondition> },
}

// --- Scenarios (DEC-078 decision 4) -----------------------------------------------

/// A starting world: biases on world generation (geography, economy, era,
/// politics). Everything left out is drawn at random, as for a random
/// country, so a scenario only says what makes it different.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDef {
    /// Content id, e.g. `scenario.wealthy_coast`.
    pub id: ContentKey,
    /// Fluent key of the name shown on the title screen.
    pub name_key: String,
    #[serde(default)]
    pub geography: ScenarioGeography,
    #[serde(default)]
    pub economy: ScenarioEconomy,
    #[serde(default)]
    pub era: ScenarioEra,
    #[serde(default)]
    pub politics: ScenarioPolitics,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioGeography {
    /// Sides of the country on the sea (1–3); random when omitted.
    #[serde(default)]
    pub sea_sides: Option<u8>,
    /// Strength of mountain ranges (0..1); random when omitted.
    #[serde(default)]
    pub mountain_strength: Option<f64>,
    /// Size of the country within the map.
    #[serde(default = "one")]
    pub country_extent: f64,
    /// Rainfall multiplier; random when omitted.
    #[serde(default)]
    pub wetness: Option<f64>,
    /// Mean temperature in the south (°C); random when omitted.
    #[serde(default)]
    pub temp_south: Option<f64>,
    /// Mineral deposits multiplier.
    #[serde(default = "one")]
    pub mineral_richness: f64,
    /// Population multiplier on the world size's population.
    #[serde(default = "one")]
    pub population: f64,
    /// Every town and city (not rural places) has an industrial district.
    #[serde(default)]
    pub industrial_towns: bool,
}

impl Default for ScenarioGeography {
    fn default() -> Self {
        ScenarioGeography {
            sea_sides: None,
            mountain_strength: None,
            country_extent: 1.0,
            wetness: None,
            temp_south: None,
            mineral_richness: 1.0,
            population: 1.0,
            industrial_towns: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioEconomy {
    /// Multiplies the structural presence of sectors with a driver (omitted = 1).
    #[serde(default)]
    pub endowment: BTreeMap<SectorDriver, f64>,
    /// Wages and dwelling prices multiplier.
    #[serde(default = "one")]
    pub wealth: f64,
    /// Multipliers on each neighbourhood's income-class mix: `[low, middle, high]`.
    #[serde(default = "ones3")]
    pub class_mix: [f64; 3],
}

impl Default for ScenarioEconomy {
    fn default() -> Self {
        ScenarioEconomy {
            endowment: BTreeMap::new(),
            wealth: 1.0,
            class_mix: [1.0; 3],
        }
    }
}

impl ScenarioEconomy {
    /// Endowment multiplier for a sector driver.
    pub fn endowment_of(&self, driver: SectorDriver) -> f64 {
        self.endowment.get(&driver).copied().unwrap_or(1.0)
    }
}

/// The trends of the age: technology and industry, and the climate's drift.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioEra {
    /// Annual technology growth (default `economy.tech_growth`).
    #[serde(default)]
    pub tech_growth: Option<f64>,
    /// Annual drift of manufacturing competitiveness (default `economy.manufacturing_trend`).
    #[serde(default)]
    pub manufacturing_trend: Option<f64>,
    /// Annual drying of the climate; random when omitted.
    #[serde(default)]
    pub drying: Option<f64>,
    /// Annual growth in storminess; random when omitted.
    #[serde(default)]
    pub storminess: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioPolitics {
    /// Added to every cohort's seeded opinion, by attitude key (omitted = 0).
    #[serde(default)]
    pub attitude_shift: BTreeMap<String, f64>,
}

fn one() -> f64 {
    1.0
}
fn ones3() -> [f64; 3] {
    [1.0; 3]
}

// --- Place kinds (DEC-078 decision 6, DEC-079) --------------------------------

/// The player actions a place can offer. The kinds of action stay in code
/// (DEC-078 decision 7); these keys match the action-cost keys in tuning
/// where an action costs time. `vote`, `draft`, `file`, and `withdraw` are free.
pub const PLACE_ACTIONS: [&str; 17] = [
    "canvass",
    "town_hall",
    "press",
    "fundraise",
    "lobby",
    "trade_favor",
    "advertise",
    "rest",
    "introduce",
    "amend",
    "vote",
    "draft",
    "file",
    "withdraw",
    "take_bribe",
    "dip_funds",
    "leak_file",
];

/// Actions with a late-night variant (the optional fourth slot).
pub const LATE_NIGHT_ACTIONS: [&str; 4] = ["lobby", "trade_favor", "fundraise", "take_bribe"];

/// Actions done only late at night (no daytime variant).
pub const LATE_ONLY_ACTIONS: [&str; 1] = ["take_bribe"];

/// Borderline and illegal actions (DEC-034, DEC-073): each pays off now and
/// leaves a hidden fact (`media.shady_actions`).
pub const SHADY_ACTIONS: [&str; 3] = ["take_bribe", "dip_funds", "leak_file"];

/// Information services a place can offer: precise data the pocket only has
/// in coarse form (DEC-079 rule 4).
pub const PLACE_SERVICES: [&str; 6] = [
    "whip_exact",
    "town_data_detailed",
    "budget_papers",
    "internal_polls",
    "overheard",
    "newspaper",
];

/// Pseudo-locations that are not generated places: the player's home, and
/// the streets of each neighbourhood.
pub const PSEUDO_PLACES: [&str; 2] = ["home", "streets"];

/// One kind of place and what it offers (DEC-079). Worldgen decides which
/// places exist; this registry supplies their behaviour.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceKindDef {
    /// Content id, e.g. `place.council_chamber`.
    pub id: ContentKey,
    /// Snake-case place kind, as worldgen names it (e.g. `council_chamber`),
    /// or a pseudo-location (`home`, `streets`).
    pub key: String,
    pub name_key: String,
    /// Actions offered here (keys from the action list).
    #[serde(default)]
    pub actions: Vec<String>,
    /// Actions that may also be taken here late at night (a subset of `actions`).
    #[serde(default)]
    pub late_night: Vec<String>,
    /// Information services offered here.
    #[serde(default)]
    pub services: Vec<String>,
    /// Where this kind's actions and services move in a town (or, for
    /// neighbourhood actions, a neighbourhood) that has no such place.
    #[serde(default)]
    pub fallback: Option<String>,
    /// How the town layout draws it (DEC-090 D4, TOWN_GENERATION.md §6.3):
    /// presentation data, validated but never read by the simulation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<crate::layout::PlaceLayoutBlock>,
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
    /// An incumbent's record of visible work (DEC-076). Below the baseline,
    /// opponents campaign on it and every voter hears (`record_penalty` per
    /// point, not discounted by awareness); above it, voters who know the
    /// incumbent reward it a little (`record_bonus`).
    pub record_penalty: f64,
    pub record_bonus: f64,
    pub record_baseline: f64,
    /// Below this Standing the party will not renominate its own officeholder.
    pub renomination_standing: f64,
    /// Issue salience (DEC-078 decision 3): every attitude's raw salience in
    /// a town is this base plus each concern's level × |its weight on the
    /// attitude|; the vector is then scaled to a mean of 1, so the issue
    /// weight keeps its meaning. A larger base mutes local priorities.
    pub salience_base: f64,
    /// Perceived position on a partisan ballot: the candidate's own views and
    /// their party's platform, blended with these weights.
    pub perceived_own_weight: f64,
    pub perceived_party_weight: f64,
    /// Valence: weight of oratory and of integrity (each around its midpoint).
    pub valence_oratory: f64,
    pub valence_integrity: f64,
    /// Share of the incumbent's record carried by the governing party's new nominee.
    pub governing_party_record: f64,
    /// Weight of the town's benchmarked economy in retrospection, beside
    /// approval of the local government.
    pub retro_economy_weight: f64,
    /// Retrospective signal per point of local unemployment change beyond the
    /// national change over a year (clamped to ±1).
    pub retro_unemployment_scale: f64,
    /// Media attention on a town's record: a base, plus its share of the region's people.
    pub media_base: f64,
    pub media_town_share: f64,
    /// Weight of voters' approval of the player in the player's own races.
    pub player_approval_weight: f64,
    /// Local viability of a candidate without a party.
    pub independent_viability: f64,
    /// Starting awareness: the player as incumbent and other sitting
    /// officeholders in town (shares of `incumbent_awareness`), the bonus in a
    /// candidate's home neighbourhood, and the cap.
    pub player_incumbent_awareness: f64,
    pub officeholder_awareness: f64,
    pub home_awareness_bonus: f64,
    pub awareness_cap: f64,
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
    /// After an election, each cohort's habit is its neighbourhood's turnout ×
    /// (`habit_base` + `habit_propensity` × its own turnout propensity).
    pub habit_base: f64,
    pub habit_propensity: f64,
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
    /// Abstracted NPC effort scales with `npc_effort_base` + oratory.
    pub npc_effort_base: f64,
    /// Share of recent contacts remembered each week (diminishing returns recover).
    pub contact_memory: f64,
    /// Each further ad in a turn is worth this fraction of the last.
    pub ad_saturation: f64,
    /// Persuasion of one ad buy, as a share of one canvass contact's.
    pub ad_persuasion_share: f64,
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
    /// Credit or blame for the party running the town hall: attachment to it
    /// scales by 1 + (approval − 0.5) × this.
    pub governing_credit: f64,
}

// --- Balance parameter blocks (DEC-078 decision 1) ---------------------------
// Numbers we expect to tune live here, one block per system; structural
// constants (midpoints, unit conversions, array sizes) stay in code. Ranges
// are validated in `validate::param_ranges`. The lab overrides any of them
// with `mandate-cli sweep|bots --set <kind>.<path>=<value>`.

/// Monthly opinion dynamics (DEC-025): experienced conditions and reactions
/// to laws and bills move approval of the local government and the player.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpinionParams {
    /// What residents experience, in approval units (tanh scale).
    pub experience: ExperienceWeights,
    /// Weight of a bill or law's anticipated reaction per unit of awareness.
    pub policy_reaction: f64,
    /// Weight of reactions to laws in government approval, beside experience.
    pub law_reaction_weight: f64,
    /// Monthly pull of government approval toward its target.
    pub gov_approval_rate: f64,
    /// Monthly pull of approval of the player toward reactions to their bills.
    pub player_approval_rate: f64,
    /// Sharpness of reactions to the player's bills.
    pub player_reaction_scale: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperienceWeights {
    /// Per point of local unemployment rise beyond the national rise (a year).
    pub unemployment: f64,
    /// Per point of real rent growth (a year).
    pub rent_growth: f64,
    /// Per unit of service improvement from laws in effect.
    pub services: f64,
    /// While a disaster is recent.
    pub disaster: f64,
    /// Months a disaster stays recent.
    pub disaster_months: u16,
    /// Per unit of construction disruption.
    pub disruption: f64,
}

/// Player action effects and risks (DEC-073). Time and health costs are in
/// `tuning.action_costs`; record gains in `career.record_gain`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionParams {
    /// Health a late-night action costs on top of its own cost.
    pub late_night_health: f64,
    pub vote: CouncilVoteEffects,
    pub amend: AmendEffects,
    pub introduce: IntroduceEffects,
    pub lobby: LobbyEffects,
    pub town_hall: TownHallEffects,
    pub press: PressEffects,
    pub canvass: CanvassEffects,
    pub fundraise: FundraiseEffects,
}

/// The player's declared council vote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CouncilVoteEffects {
    /// Standing for voting with the party line, against it, and abstaining.
    pub standing_loyal: f64,
    pub standing_defy: f64,
    pub standing_abstain: f64,
    /// Ward approval per unit of alignment × (0.5 + bill awareness) × room.
    pub ward_reaction: f64,
    /// Ward approval change for abstaining.
    pub ward_abstain: f64,
    /// The sponsor's opinion of the player: a yes, a no, and a no while they
    /// are owed a favour.
    pub sponsor_yes: f64,
    pub sponsor_no: f64,
    pub sponsor_no_owed: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AmendEffects {
    /// Share of the goodwill already built on a bill that survives an amendment.
    pub sway_keep: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IntroduceEffects {
    /// Public awareness of a bill when it is introduced (player and NPCs).
    pub awareness: f64,
}

/// The player's lobbying (the odds and the sway gained are in `council.lobby`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LobbyEffects {
    /// Sway bought by trading a favour.
    pub trade_gain: f64,
    /// The target's opinion of the player after a success and a failure.
    pub opinion_success: f64,
    pub opinion_fail: f64,
    /// Legal jeopardy of a late-night deal.
    pub late_night_legal: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TownHallEffects {
    /// Awareness gained: `awareness` × (`awareness_oratory_base` + oratory).
    pub awareness: f64,
    pub awareness_oratory_base: f64,
    /// Approval nudge per unit of alignment with the bill discussed, and
    /// without a bill.
    pub bill_nudge: f64,
    pub nudge: f64,
    /// Approval nudge from a hostile crowd.
    pub hostile_nudge: f64,
    /// Bill awareness gained: `bill_awareness` × (0.5 + oratory).
    pub bill_awareness: f64,
    /// Campaign contact: (`contact_base` + `contact_oratory` × oratory), ×
    /// `hostile_contact` when hostile. NPC town halls use the same scale.
    pub contact_base: f64,
    pub contact_oratory: f64,
    pub hostile_contact: f64,
    /// Chance of a hostile crowd: `risk_base` − `risk_approval` × the
    /// player's net approval − `risk_government` × (government approval −
    /// 0.5), within [`risk_min`, `risk_max`].
    pub risk_base: f64,
    pub risk_approval: f64,
    pub risk_government: f64,
    pub risk_min: f64,
    pub risk_max: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PressEffects {
    /// Town-wide awareness gained.
    pub awareness: f64,
    /// Approval lost to a gaffe, per unit of awareness.
    pub gaffe_approval: f64,
    /// Bill awareness gained.
    pub bill_awareness: f64,
    /// Campaign contact scale (everywhere in the race), unless there was a gaffe.
    pub contact: f64,
    /// Gaffe risk: `gaffe_base` + `gaffe_oratory` × (1 − oratory), within
    /// [`gaffe_min`, `gaffe_max`].
    pub gaffe_base: f64,
    pub gaffe_oratory: f64,
    pub gaffe_min: f64,
    pub gaffe_max: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CanvassEffects {
    /// Share of the remaining awareness gap closed.
    pub awareness: f64,
    /// Approval gained per unit of room.
    pub approval: f64,
    /// Campaign contact scale (a full day on the doorsteps).
    pub contact: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FundraiseEffects {
    /// Money raised: uniform in [min, max] × (0.5 + oratory). NPCs too.
    pub raised: [f64; 2],
    /// Standing gained (the party likes a fundraiser).
    pub standing: f64,
    /// Legal jeopardy of late-night fundraising.
    pub late_night_legal: f64,
    /// Approval lost among low-income voters, per unit of awareness.
    pub donor_ties: f64,
}

/// How council members lean on bills, how mayors sign, and lobbying odds
/// (DEC-026, DEC-071 §7). Shared by the player and NPCs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CouncilParams {
    /// A member's lean: own values (weighted by 1 − pragmatism), the ward's
    /// support (by pragmatism), and the mayor's lean.
    pub own_values: f64,
    pub ward_support: f64,
    pub mayor: f64,
    /// Goodwill toward the player as sponsor, per 100 points of opinion.
    pub player_opinion: f64,
    /// Party discipline: the sponsor's party, and a rival party.
    pub same_party: f64,
    pub rival_party: f64,
    /// Riders: a sunset, a pilot, each other ward's earmark, and an earmark
    /// for the member's own ward.
    pub sunset: f64,
    pub pilot: f64,
    pub foreign_earmark: f64,
    pub own_earmark: f64,
    /// Standard deviation of each member's idiosyncrasy per bill.
    pub noise_sd: f64,
    /// The mayor's lean on a bill: own values and the town's support.
    pub mayor_own: f64,
    pub mayor_public: f64,
    /// Logistic slope from a lean to the chance of a yes vote.
    pub vote_sharpness: f64,
    /// Mayors sign unless their lean is below −`sign_bias`.
    pub sign_bias: f64,
    /// Public awareness a bill gains at enactment, and the share it loses each month.
    pub enactment_awareness: f64,
    pub awareness_fade: f64,
    pub lobby: LobbyParams,
    /// Personality (DEC-031): party discipline × (1 + this × (2 × loyalty − 1)).
    pub loyalty_discipline: f64,
}

/// Lobbying odds and sway (player and NPCs alike).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LobbyParams {
    /// Skill = `skill_negotiation` × negotiation + `skill_oratory` × oratory.
    pub skill_negotiation: f64,
    pub skill_oratory: f64,
    /// Logit of success: `skill_scale` × (skill − 0.5) + `pragmatism` × the
    /// target's pragmatism + `bias` + ties (+ `late_night`).
    pub skill_scale: f64,
    pub pragmatism: f64,
    pub bias: f64,
    /// Ties with the player: the target's opinion of them divided by this.
    pub opinion_scale: f64,
    /// Ties between NPCs: the same party, or not.
    pub same_party: f64,
    pub other_party: f64,
    pub late_night: f64,
    /// Personality (DEC-031): − this × (target's integrity − 0.5) in the logit.
    pub integrity: f64,
    pub odds_min: f64,
    pub odds_max: f64,
    /// Sway gained on success: uniform in [min, max] × (0.5 + negotiation).
    pub gain: [f64; 2],
    /// Sway gained on failure.
    pub fail_gain: f64,
}

/// Careers (DEC-011, DEC-076): the record of visible work, the party's
/// patience, Mandate and viability, name recognition, and election results.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CareerParams {
    /// Record gained per piece of visible work (the player and NPCs).
    pub record_gain: RecordGains,
    /// Share of every record lost each month.
    pub record_fade: f64,
    /// NPC officeholders' monthly record gain per unit of diligence, and the
    /// weights of integrity and ambition in diligence.
    pub npc_diligence_rate: f64,
    pub diligence_integrity: f64,
    pub diligence_ambition: f64,
    /// Party patience: monthly Standing change below a thin record and above
    /// a strong one.
    pub thin_record: f64,
    pub thin_record_standing: f64,
    pub strong_record: f64,
    pub strong_record_standing: f64,
    /// Name recognition kept each month, and while the record is below
    /// `invisible_record`.
    pub invisible_record: f64,
    pub awareness_keep: f64,
    pub awareness_keep_invisible: f64,
    /// Mandate: monthly pull toward 50 + `mandate_per_net_approval` × net
    /// approval among the ward's voters.
    pub mandate_rate: f64,
    pub mandate_per_net_approval: f64,
    /// Viability: monthly pull toward a mix of Mandate and Standing (this
    /// share Standing).
    pub viability_rate: f64,
    pub viability_standing_share: f64,
    /// Health recovered each month.
    pub health_recovery: f64,
    /// After a win: Mandate = `win_mandate_base` + `win_mandate_per_margin` ×
    /// margin (at least `win_mandate_min`), and track gains.
    pub win_mandate_base: f64,
    pub win_mandate_per_margin: f64,
    pub win_mandate_min: f64,
    pub win_viability: f64,
    pub win_standing: f64,
    /// After a loss: viability changes by `loss_viability` +
    /// `loss_viability_per_margin` × margin (margin ≥ −0.5), and Standing.
    pub loss_viability: f64,
    pub loss_viability_per_margin: f64,
    pub loss_standing: f64,
    /// Share of scandal damage kept each year.
    pub scandal_keep_yearly: f64,
    /// Officeholders' yearly war-chest income: base + per unit of oratory.
    pub officeholder_funds: f64,
    pub officeholder_funds_oratory: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordGains {
    pub introduce: f64,
    pub amend: f64,
    pub vote: f64,
    pub lobby: f64,
    pub town_hall: f64,
    pub canvass: f64,
    pub press: f64,
}

/// NPC AI balance numbers (DEC-030). Scores and curves are in `politics.ai`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NpcParams {
    /// Strategic profile from traits: an ideologue below this pragmatism,
    /// else a populist above this ambition, else a pragmatist.
    pub ideologue_below: f64,
    pub populist_above: f64,
    /// A race is a dead heat at 0 and not close at this poll margin.
    pub close_margin: f64,
    /// Funds count as full at this many ad buys.
    pub funds_in_ads: f64,
    /// The council is busy at this many pending bills, or this many laws in force.
    pub busy_pending: f64,
    pub busy_laws: f64,
    /// At most one bill of one's own in this many days.
    pub bill_interval_days: u32,
    /// Whip counts: yes votes of insurance wanted beyond the threshold, the
    /// gap at which a count stops being close, and the weight of a count
    /// already on the right side.
    pub insurance_votes: f64,
    pub safe_gap: f64,
    pub comfortable_weight: f64,
    /// Planning within a turn: closeness kept after lobbying, and the unknown
    /// share kept after a contact.
    pub lobby_closeness_keep: f64,
    pub contact_unknown_keep: f64,
    /// Ads: two buys above this ad score, one above this, and the reserve
    /// (in ad costs) a candidate keeps.
    pub two_ads_above: f64,
    pub one_ad_above: f64,
    pub ad_reserve: f64,
    /// NPC bills sunset after this many years, [min, max].
    pub bill_sunset_years: [u8; 2],
    /// Agendas follow the town's character (DEC-077 §1): ranking its ideas, an
    /// NPC adds this × severity to a template that answers the live crisis,
    /// and `concern_agenda` × fit to one that addresses the town's concerns
    /// (alignment with the concerns' attitudes, weighted by their levels).
    /// The bonus changes which idea comes first, not how keen it is.
    pub crisis_agenda: f64,
    pub concern_agenda: f64,
}

// --- Characters (DEC-031, DEC-018) -------------------------------------------------

/// The five personality axes (DEC-031), as written in content maps.
pub const PERSONALITY_AXES: [&str; 5] = ["ambition", "integrity", "warmth", "boldness", "loyalty"];

/// Competencies (DEC-031) besides the one expertise domain, as written in content maps.
pub const COMPETENCIES: [&str; 5] = [
    "oratory",
    "negotiation",
    "administration",
    "organising",
    "intrigue",
];

/// The kinds of memory the engine records (DEC-031). Each has a parameter
/// row in `characters.memories` and a Fluent key `memory-<kind>`. New kinds
/// are a code extension point: the engine must record them somewhere.
pub const MEMORY_KINDS: [&str; 18] = [
    "backed_my_bill",
    "opposed_my_bill",
    "promised_favour",
    "kept_promise",
    "broke_promise",
    "broke_ranks",
    "lobbied_me",
    "pressed_me",
    "defeated_me",
    "lost_to_me",
    "vetoed_my_bill",
    "signed_my_bill",
    "met",
    "scandal",
    "stood_with_me",
    "stood_against_me",
    "ignored_me",
    "blamed_me",
];

/// A multiplier pair for good and bad news.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TraitScale {
    #[serde(default = "one")]
    pub positive: f64,
    #[serde(default = "one")]
    pub negative: f64,
}

impl Default for TraitScale {
    fn default() -> Self {
        TraitScale {
            positive: 1.0,
            negative: 1.0,
        }
    }
}

/// A character trait tag (DEC-031): a name plus data-driven modifiers.
/// Characters get 0–3 traits at generation. Registry order is load order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TraitDef {
    /// Content id, e.g. `trait.grudge_holder`.
    pub id: ContentKey,
    pub name_key: String,
    /// Chance a generated character has this trait (before the three-trait cap).
    pub chance: f64,
    /// Added to personality axes (`ambition`, `integrity`, `warmth`,
    /// `boldness`, `loyalty`), clamped to 0..1.
    #[serde(default)]
    pub personality: BTreeMap<String, f64>,
    /// Multiplies how fast memories fade: good ones (`positive`) and bad
    /// ones (`negative`). A grudge-holder's bad memories fade slowly.
    #[serde(default)]
    pub memory_decay: TraitScale,
    /// Multiplies opinion changes toward others: gains and losses.
    #[serde(default)]
    pub opinion_change: TraitScale,
    /// Traits that cannot be held together with this one.
    #[serde(default)]
    pub excludes: Vec<ContentKey>,
}

/// What an occupation has a stake in: the power broker of a town is the
/// leader with the biggest stake in its defining industry or live crisis.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OccupationStake {
    /// Sector drivers (`industrial`, `extraction`, …) the occupation speaks for.
    #[serde(default)]
    pub drivers: Vec<SectorDriver>,
    /// Crisis kinds (content ids) the occupation is drawn into.
    #[serde(default)]
    pub crises: Vec<ContentKey>,
}

/// An occupation or role of Named characters (DEC-018): shopkeepers,
/// organisers, journalists, clergy, union stewards. Registry order is load order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OccupationDef {
    /// Content id, e.g. `occupation.shopkeeper`.
    pub id: ContentKey,
    pub name_key: String,
    /// Place kinds where they work and can be met (or `streets`, one per
    /// neighbourhood).
    pub places: Vec<String>,
    /// Chance that one such person is generated per place (or neighbourhood).
    pub chance: f64,
    /// Organisation leaders (stewards, editors, chairs) are Principal
    /// characters and can be a town's power broker.
    #[serde(default)]
    pub leader: bool,
    /// How much weight a leader carries in town (0..1; power broker choice).
    #[serde(default)]
    pub influence: f64,
    #[serde(default)]
    pub stake: OccupationStake,
    /// Weights of the income classes they come from: [low, middle, high].
    pub class: [f64; 3],
    /// Age range, [min, max].
    pub age: [u8; 2],
    /// Added to personality axes (see `TraitDef::personality`).
    #[serde(default)]
    pub personality: BTreeMap<String, f64>,
    /// Added to competencies (`oratory`, `negotiation`, `administration`,
    /// `organising`, `intrigue`).
    #[serde(default)]
    pub competencies: BTreeMap<String, f64>,
    /// The issue they know best (an attitude key); random when omitted.
    #[serde(default)]
    pub expertise: Option<String>,
    /// Passers-by (Ambient people) may have this occupation.
    #[serde(default)]
    pub ambient: bool,
}

/// A kind of live local crisis (DEC-077 §1), read from an indicator with
/// hysteresis: it begins at `begin`, ends below `end`, and its severity runs
/// from 0 at `end` to 1 at `severe`. Registry order is load order and breaks
/// ties between equally severe crises.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CrisisDef {
    /// Content id, e.g. `crisis.plant_closure`.
    pub id: ContentKey,
    pub name_key: String,
    pub indicator: Indicator,
    pub begin: f64,
    pub end: f64,
    pub severe: f64,
    /// The concern it sharpens (a concern id), if any.
    #[serde(default)]
    pub concern: Option<ContentKey>,
    /// Policy templates that answer it: NPC agendas favour them while the
    /// crisis lasts.
    #[serde(default)]
    pub policies: Vec<ContentKey>,
}

impl CrisisDef {
    /// Severity (0..1) of an indicator value.
    pub fn severity(&self, v: f64) -> f64 {
        let span = self.severe - self.end;
        if span <= 0.0 {
            return if v >= self.begin { 1.0 } else { 0.0 };
        }
        ((v - self.end) / span).clamp(0.0, 1.0)
    }
}

/// Parameters of one memory kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemoryKindParams {
    /// How the holder felt about it (−1..1).
    pub sentiment: f64,
    /// How much it mattered (0..1): its salience when fresh.
    pub magnitude: f64,
    /// Fading per year (salience × e^(−decay × years)); near 0 persists.
    pub decay: f64,
    /// Opinion change (−100..100) when the engine does not set one itself
    /// (the player's actions take theirs from `actions.jsonc`).
    pub opinion: f64,
    /// Trust change (−1..1).
    pub trust: f64,
    /// Free-form tags for queries (`promise`, `vote`, `election`, …).
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemoryCaps {
    /// Memories kept per Principal and per Named character.
    pub principal: u8,
    pub named: u8,
    /// A memory whose salience falls below this has faded into the
    /// relationship baseline and is forgotten.
    pub forget_below: f64,
}

/// Generation of characters (DEC-031): personality and competency ranges.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CharacterGenParams {
    /// Personality axes are drawn uniformly in this range, plus offsets.
    pub personality: [f64; 2],
    /// Competencies likewise.
    pub competency: [f64; 2],
    /// Named characters' values: their home cohort's view plus noise of this SD.
    pub values_sd: f64,
    /// A Named character's first impression of the player: normal, this SD.
    pub first_impression_sd: f64,
    /// Chance a Named character is family of a local politician.
    pub family_chance: f64,
    /// Lazy yearly updates of Named characters: value drift (SD per year).
    pub drift_sd: f64,
    /// Trust in someone they have no history with (0..1).
    pub trust_initial: f64,
}

/// Bonds that follow from opinion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BondParams {
    /// An ally at or above this opinion; a rival at or below `rival_below`.
    pub ally_above: f64,
    pub rival_below: f64,
}

/// Personality in opinion changes (DEC-031): the same rules for the player
/// and NPCs. (Loyalty in votes is `council.loyalty_discipline`; integrity
/// against lobbying is `council.lobby.integrity`.)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PersonalityAiParams {
    /// Opinion gains × (1 + w × (2 × warmth − 1)) of the holder.
    pub warmth_opinion: f64,
}

/// Ambient people: generated on demand at places, never stored (DEC-018).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AmbientParams {
    /// People present at a public place, [min, max].
    pub present: [u8; 2],
    /// Who is around changes every this many days.
    pub period_days: u32,
    /// Most people listed at a place, Named and Ambient together.
    pub max_listed: u8,
}

/// Town character (DEC-077 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TownCharacterParams {
    /// Months of crisis severity kept for the trend.
    pub trend_months: u8,
    /// Power broker: weight of an officeholder's seniority (per year in
    /// office) against a leader's influence and stake.
    pub broker_seniority: f64,
    /// Stake weight of a leader whose stake matches the defining industry
    /// or the live crisis.
    pub broker_stake: f64,
}

/// Characters, memory, relationships, and town character (DEC-031, DEC-077).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CharacterParams {
    pub memory: MemoryCaps,
    /// One row per memory kind (`MEMORY_KINDS`).
    pub memories: BTreeMap<String, MemoryKindParams>,
    pub generation: CharacterGenParams,
    pub bonds: BondParams,
    pub ai: PersonalityAiParams,
    pub ambient: AmbientParams,
    pub town: TownCharacterParams,
}

impl CharacterParams {
    pub fn memory(&self, kind: &str) -> Option<&MemoryKindParams> {
        self.memories.get(kind)
    }
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
        "attitudes" => ContentFile::Attitudes {
            items: serde_json::from_str::<ItemsFile<AttitudeDef>>(json)?.items,
        },
        "communities" => ContentFile::Communities {
            items: serde_json::from_str::<ItemsFile<CommunityDef>>(json)?.items,
        },
        "concerns" => ContentFile::Concerns {
            items: serde_json::from_str::<ItemsFile<ConcernDef>>(json)?.items,
        },
        "scenarios" => ContentFile::Scenarios {
            items: serde_json::from_str::<ItemsFile<ScenarioDef>>(json)?.items,
        },
        "name_pools" => ContentFile::NamePools {
            items: serde_json::from_str::<ItemsFile<NamePoolDef>>(json)?.items,
        },
        "places" => ContentFile::Places {
            items: serde_json::from_str::<ItemsFile<PlaceKindDef>>(json)?.items,
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
        "opinion" => ContentFile::Opinion {
            value: serde_json::from_str::<ValueFile<OpinionParams>>(json)?.value,
        },
        "actions" => ContentFile::Actions {
            value: Box::new(serde_json::from_str::<ValueFile<ActionParams>>(json)?.value),
        },
        "council" => ContentFile::Council {
            value: serde_json::from_str::<ValueFile<CouncilParams>>(json)?.value,
        },
        "career" => ContentFile::Career {
            value: serde_json::from_str::<ValueFile<CareerParams>>(json)?.value,
        },
        "npc" => ContentFile::Npc {
            value: serde_json::from_str::<ValueFile<NpcParams>>(json)?.value,
        },
        "traits" => ContentFile::Traits {
            items: serde_json::from_str::<ItemsFile<TraitDef>>(json)?.items,
        },
        "occupations" => ContentFile::Occupations {
            items: serde_json::from_str::<ItemsFile<OccupationDef>>(json)?.items,
        },
        "crises" => ContentFile::Crises {
            items: serde_json::from_str::<ItemsFile<CrisisDef>>(json)?.items,
        },
        "characters" => ContentFile::Characters {
            value: Box::new(serde_json::from_str::<ValueFile<CharacterParams>>(json)?.value),
        },
        "director" => ContentFile::Director {
            value: Box::new(
                serde_json::from_str::<ValueFile<crate::narrative::DirectorParams>>(json)?.value,
            ),
        },
        "storytellers" => ContentFile::Storytellers {
            items: serde_json::from_str::<ItemsFile<crate::narrative::StorytellerDef>>(json)?.items,
        },
        "overheard" => ContentFile::Overheard {
            items: serde_json::from_str::<ItemsFile<OverheardDef>>(json)?.items,
        },
        "dilemmas" => ContentFile::Dilemmas {
            items: serde_json::from_str::<ItemsFile<crate::narrative::DilemmaDef>>(json)?.items,
        },
        "goals" => ContentFile::Goals {
            items: serde_json::from_str::<ItemsFile<crate::narrative::GoalDef>>(json)?.items,
        },
        "storylines" => ContentFile::Storylines {
            items: serde_json::from_str::<ItemsFile<crate::narrative::StorylineDef>>(json)?.items,
        },
        "scenes" => ContentFile::Scenes {
            items: serde_json::from_str::<ItemsFile<crate::narrative::SceneDef>>(json)?.items,
        },
        "outlets" => ContentFile::Outlets {
            items: serde_json::from_str::<ItemsFile<crate::media::OutletDef>>(json)?.items,
        },
        "media" => ContentFile::Media {
            value: Box::new(
                serde_json::from_str::<ValueFile<crate::media::MediaParams>>(json)?.value,
            ),
        },
        "justice" => ContentFile::Justice {
            value: Box::new(
                serde_json::from_str::<ValueFile<crate::media::JusticeParams>>(json)?.value,
            ),
        },
        "town_districts" => ContentFile::TownDistricts {
            items: serde_json::from_str::<ItemsFile<crate::layout::TownDistrictDef>>(json)?.items,
        },
        "terrain_features" => ContentFile::TerrainFeatures {
            items: serde_json::from_str::<ItemsFile<crate::layout::TerrainFeatureDef>>(json)?.items,
        },
        "connectors" => ContentFile::Connectors {
            items: serde_json::from_str::<ItemsFile<crate::layout::ConnectorDef>>(json)?.items,
        },
        "knowledge" => ContentFile::Knowledge {
            value: Box::new(
                serde_json::from_str::<ValueFile<crate::knowledge::KnowledgeParams>>(json)?.value,
            ),
        },
        "talk_topics" => ContentFile::TalkTopics {
            items: serde_json::from_str::<ItemsFile<crate::conversation::TopicDef>>(json)?.items,
        },
        "talk_lines" => ContentFile::TalkLines {
            items: serde_json::from_str::<ItemsFile<crate::conversation::TalkLineDef>>(json)?.items,
        },
        "talk_voices" => ContentFile::TalkVoices {
            items: serde_json::from_str::<ItemsFile<crate::conversation::VoiceDef>>(json)?.items,
        },
        "talk_params" => ContentFile::TalkParams {
            value: Box::new(
                serde_json::from_str::<ValueFile<crate::conversation::TalkParams>>(json)?.value,
            ),
        },
        other => {
            use serde::de::Error;
            return Err(serde_json::Error::custom(format!(
                "unknown content kind `{other}`"
            )));
        }
    })
}
