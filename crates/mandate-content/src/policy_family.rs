//! Policy families (DEC-092; M6_ENGINE_CONTRACTS §2): reusable proposals
//! with **slots** that bind to the live town (a place, a neighbourhood, a
//! person, a sector, a crisis, a hazard), a **trigger** and an **expiry** on
//! the one condition language (`condition.rs`), **choice axes** whose options
//! carry their own mechanisms, costs and trade-offs, **sponsors** who bring
//! a ready version for a price, and **knowledge hooks** (DEC-091).
//!
//! A family is content (`content/<pack>/policy_families/*.jsonc`, `"kind":
//! "policy_families"`). Its numbers (rarity chances, the cap on open
//! proposals, sponsor goodwill, works) are `policy_family_params`; packs
//! change other packs' families with small, named **patches**
//! (`policy_family_patches`, applied in pack order).
//!
//! The engine composes a drafted family bill (`mandate_sim::policy::resolve`):
//! the base plus one option per axis, slot references (`@slot`) replaced by
//! what is bound. Works you can see (DEC-099): an option (or the base) may
//! declare a **works project** that the bill builds at a bound place or
//! neighbourhood.
//!
//! Adding a family: `docs/development/EXTENDING.md`, "Adding a policy family".

use crate::bundle::ContentBundle;
use crate::condition::{CondTier, Condition, ConditionUse, Site};
use crate::types::{
    BudgetCategory, ClassKey, ContentKey, CostDef, HazardKey, MechanismDef, ParamDef, Power,
    SectorDriver, SideEffectDef, SituationTrigger, TierKey,
};
use crate::validate::Issue;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// One policy family (§2.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyFamilyDef {
    /// `family.<name>`, never shown.
    pub id: ContentKey,
    /// Fluent keys for its title and summary; slot names are arguments.
    pub name_key: String,
    pub desc_key: String,
    /// The tier of government whose jurisdictions it opens in (DEC-098).
    pub tier: TierKey,
    /// The budget line it is paid from.
    pub category: BudgetCategory,
    /// The constitution power needed to propose it (never an office name).
    pub requires: Power,
    /// A power or competence the jurisdiction may lack (M7; shown, not used).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs: Option<NeedsDef>,
    #[serde(default)]
    pub draw: DrawDef,
    /// How many can be open at once in one jurisdiction.
    #[serde(default = "one_u8")]
    pub max_instances: u8,
    /// What it is about, bound in order (a later slot may refer to an
    /// earlier one).
    pub slots: Vec<SlotDef>,
    /// When it comes onto the table; without one, as soon as its slots bind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<SituationTrigger>,
    #[serde(default)]
    pub expiry: ExpiryDef,
    /// What every version has.
    pub base: FamilyBaseDef,
    /// The design choices (1–3 axes of 2–3 options).
    pub axes: Vec<AxisDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sponsors: Vec<SponsorDef>,
    #[serde(default)]
    pub knowledge: FamilyKnowledgeDef,
    /// Set by a `disable` patch: never drawn, never opened; bills and laws
    /// already made continue. Not part of the format.
    #[serde(skip)]
    #[schemars(skip)]
    pub disabled: bool,
}

fn one_u8() -> u8 {
    1
}
fn one_f64() -> f64 {
    1.0
}
fn yes() -> bool {
    true
}
fn three() -> u8 {
    3
}

impl PolicyFamilyDef {
    pub fn slot(&self, id: &str) -> Option<(usize, &SlotDef)> {
        self.slots.iter().enumerate().find(|(_, s)| s.id == id)
    }
    pub fn axis(&self, id: &str) -> Option<&AxisDef> {
        self.axes.iter().find(|a| a.id == id)
    }
    pub fn sponsor(&self, id: &str) -> Option<&SponsorDef> {
        self.sponsors.iter().find(|s| s.id == id)
    }
    /// Every parameter a draft can set: the base's, then each option's.
    pub fn all_params(&self) -> impl Iterator<Item = &ParamDef> {
        self.base.params.iter().chain(
            self.axes
                .iter()
                .flat_map(|a| a.options.iter())
                .flat_map(|o| o.params.iter()),
        )
    }
}

/// How often a family turns up in a run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DrawDef {
    #[serde(default)]
    pub rarity: Rarity,
    /// Priority when several are ready at once, in (0, 10].
    #[serde(default = "one_f64")]
    pub weight: f64,
}

impl Default for DrawDef {
    fn default() -> Self {
        DrawDef {
            rarity: Rarity::Common,
            weight: 1.0,
        }
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Rarity {
    Always,
    #[default]
    Common,
    Uncommon,
    Rare,
}

/// One slot: what it binds and whether a family can open without it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SlotDef {
    pub id: String,
    pub bind: BindDef,
    #[serde(default)]
    pub optional: bool,
}

/// What a slot binds, and how it is chosen (§2.3). Candidates are inside
/// the instance's jurisdiction, taken in id order, ranked by `pick`; ties go
/// to the lowest id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum BindDef {
    /// A place of one of these kinds.
    Place {
        kinds: Vec<String>,
        /// Only places whose kind has one of these sector drivers.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        drivers: Vec<SectorDriver>,
        /// Inside a bound neighbourhood.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        in_slot: Option<String>,
        /// Only open places (DEC-090 D3).
        #[serde(default = "yes")]
        open_only: bool,
        #[serde(default)]
        pick: Pick,
    },
    /// A neighbourhood.
    Neighborhood {
        /// Neighbourhood kinds (`core`, `waterfront`, …).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        kinds: Vec<String>,
        /// The neighbourhood of a bound place.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        of_slot: Option<String>,
        /// Exposed to this hazard at least `min_exposure` (DEC-090 D8).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hazard: Option<HazardKey>,
        #[serde(default)]
        min_exposure: f64,
        /// This concern among its `top` worries.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        concern: Option<ContentKey>,
        #[serde(default = "three")]
        top: u8,
        /// Place kinds it has none of.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        lacks: Vec<String>,
        #[serde(default)]
        pick: Pick,
    },
    /// A Named or Principal character.
    Person {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        occupations: Vec<ContentKey>,
        /// Works at a bound place.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at_slot: Option<String>,
        /// Lives or works in a bound neighbourhood.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        in_slot: Option<String>,
        #[serde(default)]
        fallback: PersonFallback,
    },
    /// The town's power broker (a town's instance; unbound elsewhere until M7).
    Broker,
    /// A sector.
    Sector {
        /// The sector of a bound place (its kind's drivers, the
        /// jurisdiction's largest matching sector).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        of_slot: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        drivers: Vec<SectorDriver>,
        /// The defining industry.
        #[serde(default)]
        defining: bool,
    },
    /// The live crisis (of one of these kinds).
    Crisis {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        kinds: Vec<ContentKey>,
    },
    /// A hazard.
    Hazard {
        hazards: Vec<HazardKey>,
        /// Its exposure in a bound neighbourhood (else the town's).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        in_slot: Option<String>,
        #[serde(default)]
        min_exposure: f64,
        #[serde(default)]
        pick: Pick,
    },
}

/// The kinds of thing a slot binds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlotKind {
    Place,
    Neighborhood,
    Person,
    Sector,
    Crisis,
    Hazard,
}

impl SlotKind {
    pub fn label(self) -> &'static str {
        match self {
            SlotKind::Place => "place",
            SlotKind::Neighborhood => "neighborhood",
            SlotKind::Person => "person",
            SlotKind::Sector => "sector",
            SlotKind::Crisis => "crisis",
            SlotKind::Hazard => "hazard",
        }
    }
    /// Whether its Fluent argument is a key (`<slot>_key`) rather than a name.
    pub fn keyed_arg(self) -> bool {
        matches!(self, SlotKind::Sector | SlotKind::Crisis | SlotKind::Hazard)
    }
}

impl BindDef {
    pub fn kind(&self) -> SlotKind {
        match self {
            BindDef::Place { .. } => SlotKind::Place,
            BindDef::Neighborhood { .. } => SlotKind::Neighborhood,
            BindDef::Person { .. } | BindDef::Broker => SlotKind::Person,
            BindDef::Sector { .. } => SlotKind::Sector,
            BindDef::Crisis { .. } => SlotKind::Crisis,
            BindDef::Hazard { .. } => SlotKind::Hazard,
        }
    }
    /// The earlier slots it refers to, with the kind each must be.
    pub fn refers(&self) -> Vec<(&str, SlotKind)> {
        let mut v = Vec::new();
        match self {
            BindDef::Place { in_slot, .. } => {
                v.extend(in_slot.as_deref().map(|s| (s, SlotKind::Neighborhood)))
            }
            BindDef::Neighborhood { of_slot, .. } => {
                v.extend(of_slot.as_deref().map(|s| (s, SlotKind::Place)))
            }
            BindDef::Person {
                at_slot, in_slot, ..
            } => {
                v.extend(at_slot.as_deref().map(|s| (s, SlotKind::Place)));
                v.extend(in_slot.as_deref().map(|s| (s, SlotKind::Neighborhood)));
            }
            BindDef::Sector { of_slot, .. } => {
                v.extend(of_slot.as_deref().map(|s| (s, SlotKind::Place)))
            }
            BindDef::Hazard { in_slot, .. } => {
                v.extend(in_slot.as_deref().map(|s| (s, SlotKind::Neighborhood)))
            }
            BindDef::Broker | BindDef::Crisis { .. } => {}
        }
        v
    }
}

/// How a slot chooses among candidates.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Pick {
    /// The lowest id: stable and plain.
    #[default]
    First,
    /// Places: the largest job decline in their sector (E1 baselines).
    MostAtRisk,
    /// Neighbourhoods and hazards: the highest exposure.
    MostExposed,
    /// Neighbourhoods: the highest level of `concern`.
    MostConcerned,
    /// The most residents (neighbourhoods) or jobs (places).
    Largest,
    /// A keyed draw among candidates.
    Keyed,
}

/// Who a person slot binds when nobody matches (never an office name, DEC-098).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PersonFallback {
    #[default]
    None,
    /// The town's power broker.
    Broker,
    /// Whoever holds the jurisdiction's executive office.
    Executive,
}

/// When an open instance with no bill goes away (§2.4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpiryDef {
    /// It lapses when this holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<SituationTrigger>,
    /// … or when its trigger has not held for this many months.
    #[serde(default = "grace")]
    pub grace_months: u16,
    /// … or this many months after it opened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_months: Option<u16>,
    /// Months before it can open again in the same jurisdiction.
    #[serde(default = "cooldown")]
    pub cooldown_months: u16,
}

fn grace() -> u16 {
    3
}
fn cooldown() -> u16 {
    12
}

impl Default for ExpiryDef {
    fn default() -> Self {
        ExpiryDef {
            when: None,
            grace_months: grace(),
            max_months: None,
            cooldown_months: cooldown(),
        }
    }
}

/// What every version of a family has, as in a policy template.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FamilyBaseDef {
    #[serde(default)]
    pub params: Vec<ParamDef>,
    pub cost: CostDef,
    pub mechanisms: Vec<MechanismDef>,
    pub implementation_months: u16,
    pub signature: BTreeMap<String, f64>,
    pub secondary_keys: Vec<String>,
    #[serde(default)]
    pub side_effects: Vec<SideEffectDef>,
    /// A works project every version builds (DEC-099).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub works: Option<WorksDef>,
}

/// One design choice.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AxisDef {
    pub id: String,
    pub name_key: String,
    /// The option a draft gets when it names none (default: the first).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    pub options: Vec<FamilyOptionDef>,
}

impl AxisDef {
    pub fn option(&self, id: &str) -> Option<&FamilyOptionDef> {
        self.options.iter().find(|o| o.id == id)
    }
    /// The default option's id.
    pub fn default_option(&self) -> Option<&str> {
        self.default
            .as_deref()
            .or_else(|| self.options.first().map(|o| o.id.as_str()))
    }
}

/// One option of an axis (§2.5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FamilyOptionDef {
    pub id: String,
    pub name_key: String,
    pub desc_key: String,
    /// Where it applies (at most one axis per family sets it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<AreaDef>,
    /// Extra sliders.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<ParamDef>,
    /// Hidden parameter values its mechanisms use.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fixed: BTreeMap<String, f64>,
    /// Added to the base cost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostDef>,
    /// Multiplies the whole cost.
    #[serde(default = "one_f64")]
    pub cost_factor: f64,
    /// Multiplies every mechanism's effect.
    #[serde(default = "one_f64")]
    pub effect_factor: f64,
    /// Added to the implementation time (at least 1 month overall).
    #[serde(default)]
    pub months: i16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mechanisms: Vec<MechanismDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub side_effects: Vec<SideEffectDef>,
    /// Added to the base signature per attitude (clamped to −1..1).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub signature: BTreeMap<String, f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary_keys: Vec<String>,
    /// Options it cannot be combined with (`"axis.option"`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excludes: Vec<String>,
    /// A tier or competence it needs (M7): shown unavailable in M6.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs: Option<NeedsDef>,
    /// A works project this option builds (DEC-099).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub works: Option<WorksDef>,
}

/// Where an option applies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AreaDef {
    /// The whole jurisdiction.
    Town,
    /// A bound neighbourhood.
    Slot { slot: String },
    /// A bound neighbourhood and those next to it.
    SlotAndAdjacent { slot: String },
}

impl AreaDef {
    pub fn slot(&self) -> Option<&str> {
        match self {
            AreaDef::Town => None,
            AreaDef::Slot { slot } | AreaDef::SlotAndAdjacent { slot } => Some(slot),
        }
    }
    /// Its size rank for the DEC-073 check (none 0 … town 3).
    fn rank(a: Option<&AreaDef>) -> f64 {
        match a {
            None => 0.0,
            Some(AreaDef::Slot { .. }) => 1.0,
            Some(AreaDef::SlotAndAdjacent { .. }) => 2.0,
            Some(AreaDef::Town) => 3.0,
        }
    }
}

/// A tier or competence a family or option needs (DEC-092 §5; M7).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NeedsDef {
    pub tier: TierKey,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub competence: Option<ContentKey>,
}

/// A sponsor's ready version (§2.6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SponsorDef {
    pub id: String,
    /// The person slot of whoever sponsors it.
    pub slot: String,
    pub name_key: String,
    /// One option per axis (a missing axis: its default).
    pub preset: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, f64>,
    pub price: PriceDef,
}

/// What adopting a sponsor's version costs the player.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PriceDef {
    /// Favours owed to the sponsor (DEC-031); they call them in later.
    Favour { count: u8, context_key: String },
    /// A public endorsement: approval rises among the sponsor's people
    /// (`classes`, or the residents of `area`) and falls a little elsewhere.
    Endorsement {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        classes: Vec<ClassKey>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        area: Option<AreaDef>,
        approval: f64,
        backlash: f64,
    },
}

/// How the player finds out about a proposal (§2.7, DEC-091).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FamilyKnowledgeDef {
    /// In the paper when it opens (heard by all).
    #[serde(default)]
    pub public: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heard: Vec<HookDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known: Vec<HookDef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub detailed: Vec<HookDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead: Option<LeadDef>,
}

/// A moment that teaches the player about a proposal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum HookDef {
    /// Arriving at a place of one of these kinds in the town.
    Overheard { at: Vec<String> },
    /// Arriving at the bound place, or the streets of the bound neighbourhood.
    Visit { slot: String },
    /// Meeting, lobbying or trading a favour with the bound person.
    Meet { slot: String },
    /// Doing this action in or with the bound thing.
    Action {
        action: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slot: Option<String>,
    },
    /// Arriving where this service is offered.
    Service { service: String },
    /// Talking to the bound person on this topic.
    Talk { slot: String, topic: ContentKey },
    /// The sponsor asks for a meeting this many months after it opens.
    Approach { slot: String, after_months: u16 },
}

impl HookDef {
    pub fn slot(&self) -> Option<&str> {
        match self {
            HookDef::Visit { slot }
            | HookDef::Meet { slot }
            | HookDef::Talk { slot, .. }
            | HookDef::Approach { slot, .. } => Some(slot),
            HookDef::Action { slot, .. } => slot.as_deref(),
            HookDef::Overheard { .. } | HookDef::Service { .. } => None,
        }
    }
}

/// A lead posted when the proposal opens: who or where would know more.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LeadDef {
    pub key: String,
    pub to: LeadToDef,
    /// Used when `to` names an unbound slot.
    #[serde(default, rename = "else", skip_serializing_if = "Option::is_none")]
    pub r#else: Option<Box<LeadDef>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LeadToDef {
    /// The bound person or place.
    Slot { slot: String },
    /// A place of this kind in the town.
    PlaceKind { kind: String },
    /// The streets of a bound neighbourhood.
    Streets { slot: String },
}

/// A works project (DEC-099): a physical work a drafted bill builds at a
/// bound place or neighbourhood, which the map and street can show.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorksDef {
    /// The kind of work, as the art and street layers name it (`flood_wall`,
    /// `market_stalls`, `clinic_wing`): `[a-z0-9_]+`.
    pub kind: String,
    /// The place or neighbourhood slot it stands at.
    pub at: String,
    /// Months of building (default: the bill's implementation time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub months: Option<u16>,
    /// Monthly chance, while building, that the contractor fails and the
    /// work stalls (a keyed draw), in [0, 0.2].
    #[serde(default)]
    pub contractor_risk: f64,
}

// --- Parameters (`policy_family_params`) -------------------------------------------------

/// The family numbers (§2.6, §2.8; DEC-099).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FamilyParams {
    /// Proposals open at once in one jurisdiction.
    pub max_open_per_jurisdiction: u8,
    /// The chance a family of each rarity is in a run's deck.
    pub rarity: RarityChances,
    /// The sponsor's opinion of the player when the player adopts their
    /// version (−100..100 scale).
    pub adopt_opinion: f64,
    /// Opinion per axis changed away from the sponsor's preset.
    pub amend_opinion: f64,
    /// Slider changes within this many steps of the preset are free.
    #[serde(default = "one_f64")]
    pub free_steps: f64,
    /// Works projects (DEC-099).
    #[serde(default)]
    pub works: WorksParams,
}

impl Default for FamilyParams {
    fn default() -> Self {
        FamilyParams {
            max_open_per_jurisdiction: 4,
            rarity: RarityChances::default(),
            adopt_opinion: 8.0,
            amend_opinion: -4.0,
            free_steps: 1.0,
            works: WorksParams::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RarityChances {
    pub always: f64,
    pub common: f64,
    pub uncommon: f64,
    pub rare: f64,
}

impl Default for RarityChances {
    fn default() -> Self {
        RarityChances {
            always: 1.0,
            common: 0.85,
            uncommon: 0.5,
            rare: 0.2,
        }
    }
}

impl RarityChances {
    pub fn of(&self, r: Rarity) -> f64 {
        match r {
            Rarity::Always => self.always,
            Rarity::Common => self.common,
            Rarity::Uncommon => self.uncommon,
            Rarity::Rare => self.rare,
        }
    }
}

/// How works progress, stall and decay (DEC-099).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorksParams {
    /// Months without funding before building stalls.
    pub stall_unfunded_months: u16,
    /// Condition lost per month by a stalled or abandoned site (0..1 scale).
    pub decay_per_month: f64,
    /// Condition lost per month by an open work once it is ageing.
    pub wear_per_month: f64,
    /// Months after opening before a work starts to age.
    pub ageing_after_months: u16,
    /// Months a stalled site waits before it counts as abandoned.
    pub abandon_after_months: u16,
}

impl Default for WorksParams {
    fn default() -> Self {
        WorksParams {
            stall_unfunded_months: 3,
            decay_per_month: 0.04,
            wear_per_month: 0.002,
            ageing_after_months: 60,
            abandon_after_months: 12,
        }
    }
}

// --- Patches ------------------------------------------------------------------------------

/// A targeted change to another pack's family, applied in pack order
/// (DEC-092: "mods can … patch them"; §5 question 10).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum FamilyPatch {
    /// Change how often it turns up.
    SetDraw { family: ContentKey, draw: DrawDef },
    /// Add an option to an axis.
    AddOption {
        family: ContentKey,
        axis: String,
        option: Box<FamilyOptionDef>,
    },
    /// Remove an option from an axis.
    RemoveOption {
        family: ContentKey,
        axis: String,
        option: String,
    },
    /// Add a sponsor.
    AddSponsor {
        family: ContentKey,
        sponsor: Box<SponsorDef>,
    },
    /// Remove a sponsor.
    RemoveSponsor { family: ContentKey, sponsor: String },
    /// Never draw or open it again (bills and laws continue).
    Disable { family: ContentKey },
}

impl FamilyPatch {
    pub fn family(&self) -> &str {
        match self {
            FamilyPatch::SetDraw { family, .. }
            | FamilyPatch::AddOption { family, .. }
            | FamilyPatch::RemoveOption { family, .. }
            | FamilyPatch::AddSponsor { family, .. }
            | FamilyPatch::RemoveSponsor { family, .. }
            | FamilyPatch::Disable { family } => family,
        }
    }

    /// Apply to the registry; an error message if it does not fit.
    pub fn apply(&self, families: &mut [PolicyFamilyDef]) -> Result<(), String> {
        let id = self.family();
        let Some(f) = families.iter_mut().find(|f| f.id == id) else {
            return Err(format!("patch for unknown family `{id}`"));
        };
        match self {
            FamilyPatch::SetDraw { draw, .. } => f.draw = draw.clone(),
            FamilyPatch::AddOption { axis, option, .. } => {
                let a = f
                    .axes
                    .iter_mut()
                    .find(|a| &a.id == axis)
                    .ok_or_else(|| format!("`{id}` has no axis `{axis}`"))?;
                if a.option(&option.id).is_some() {
                    return Err(format!("`{id}` axis `{axis}` already has `{}`", option.id));
                }
                a.options.push((**option).clone());
            }
            FamilyPatch::RemoveOption { axis, option, .. } => {
                let a = f
                    .axes
                    .iter_mut()
                    .find(|a| &a.id == axis)
                    .ok_or_else(|| format!("`{id}` has no axis `{axis}`"))?;
                let n = a.options.len();
                a.options.retain(|o| &o.id != option);
                if a.options.len() == n {
                    return Err(format!("`{id}` axis `{axis}` has no option `{option}`"));
                }
                if a.default.as_deref() == Some(option.as_str()) {
                    a.default = None;
                }
            }
            FamilyPatch::AddSponsor { sponsor, .. } => {
                if f.sponsor(&sponsor.id).is_some() {
                    return Err(format!("`{id}` already has sponsor `{}`", sponsor.id));
                }
                f.sponsors.push((**sponsor).clone());
            }
            FamilyPatch::RemoveSponsor { sponsor, .. } => {
                let n = f.sponsors.len();
                f.sponsors.retain(|s| &s.id != sponsor);
                if f.sponsors.len() == n {
                    return Err(format!("`{id}` has no sponsor `{sponsor}`"));
                }
            }
            FamilyPatch::Disable { .. } => f.disabled = true,
        }
        Ok(())
    }
}

// --- Slot references ----------------------------------------------------------------------

/// The slot a `@slot` reference names (`"@sector"` → `sector`).
pub fn slot_ref(s: &str) -> Option<&str> {
    s.strip_prefix('@')
}

/// Mechanism variants declared for families and simulated (F2b): `borrow`,
/// `revenue`, `sector_capacity`.
pub fn is_family_mechanism(m: &MechanismDef) -> bool {
    matches!(
        m,
        MechanismDef::Borrow { .. }
            | MechanismDef::Revenue { .. }
            | MechanismDef::SectorCapacity { .. }
    )
}

// --- Validation (§2.13, items 1–10) ---------------------------------------------------------

/// Validate the family registry, its parameters and patches (`content
/// validate`; items 1–10 of §2.13). Item 11 (every family binds in a
/// canonical test world) needs worldgen and lives in the CLI (F5).
pub fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let p = &b.policy_family_params;
    let owner = "policy_family_params";
    if !(1..=12).contains(&p.max_open_per_jurisdiction) {
        issues.push(Issue::error(
            owner,
            "max_open_per_jurisdiction must be 1–12",
        ));
    }
    let r = &p.rarity;
    for (k, v) in [
        ("always", r.always),
        ("common", r.common),
        ("uncommon", r.uncommon),
        ("rare", r.rare),
    ] {
        if !(v.is_finite() && (0.0..=1.0).contains(&v)) {
            issues.push(Issue::error(
                owner,
                &format!("rarity.{k} must be in [0, 1]"),
            ));
        }
    }
    if !(r.always >= r.common && r.common >= r.uncommon && r.uncommon >= r.rare) {
        issues.push(Issue::error(
            owner,
            "rarity chances must not rise as rarity rises (always ≥ common ≥ uncommon ≥ rare)",
        ));
    }
    if !(p.adopt_opinion.is_finite() && (0.0..=30.0).contains(&p.adopt_opinion)) {
        issues.push(Issue::error(owner, "adopt_opinion must be in [0, 30]"));
    }
    if !(p.amend_opinion.is_finite() && (-30.0..=0.0).contains(&p.amend_opinion)) {
        issues.push(Issue::error(owner, "amend_opinion must be in [-30, 0]"));
    }
    if !(p.free_steps.is_finite() && (0.0..=10.0).contains(&p.free_steps)) {
        issues.push(Issue::error(owner, "free_steps must be in [0, 10]"));
    }
    let w = &p.works;
    if !(w.decay_per_month.is_finite() && (0.0..=0.5).contains(&w.decay_per_month))
        || !(w.wear_per_month.is_finite() && (0.0..=0.1).contains(&w.wear_per_month))
    {
        issues.push(Issue::error(
            owner,
            "works decay_per_month must be in [0, 0.5] and wear_per_month in [0, 0.1]",
        ));
    }
    if w.stall_unfunded_months == 0 || w.abandon_after_months == 0 {
        issues.push(Issue::error(
            owner,
            "works stall_unfunded_months and abandon_after_months must be at least 1",
        ));
    }
    let en = crate::validate::fluent_placeholders(&b.locale_text("en"));
    for f in &b.policy_families {
        family(b, f, &en, issues);
    }
}

fn family(
    b: &ContentBundle,
    f: &PolicyFamilyDef,
    en: &BTreeMap<String, BTreeSet<String>>,
    issues: &mut Vec<Issue>,
) {
    let owner = f.id.as_str();
    let err = |issues: &mut Vec<Issue>, m: String| issues.push(Issue::error(owner, &m));
    // 1. Ids.
    let name = f.id.strip_prefix("family.");
    if !name.is_some_and(|n| !n.is_empty() && crate::validate::valid_key(&f.id)) {
        err(issues, "family ids look like `family.<name>`".into());
    }
    let unique = |what: &str, ids: Vec<&str>, issues: &mut Vec<Issue>| {
        let mut seen = BTreeSet::new();
        for id in ids {
            if !seen.insert(id) {
                issues.push(Issue::error(owner, &format!("duplicate {what} id `{id}`")));
            }
        }
    };
    unique(
        "slot",
        f.slots.iter().map(|s| s.id.as_str()).collect(),
        issues,
    );
    unique(
        "axis",
        f.axes.iter().map(|a| a.id.as_str()).collect(),
        issues,
    );
    unique(
        "sponsor",
        f.sponsors.iter().map(|s| s.id.as_str()).collect(),
        issues,
    );
    for a in &f.axes {
        unique(
            "option",
            a.options.iter().map(|o| o.id.as_str()).collect(),
            issues,
        );
    }
    unique(
        "parameter",
        f.all_params().map(|p| p.id.as_str()).collect(),
        issues,
    );
    if f.max_instances == 0 || f.max_instances > 4 {
        err(issues, "max_instances must be 1–4".into());
    }
    if f.slots.is_empty() {
        err(issues, "a family needs at least one slot".into());
    }

    // 2. Slots bind real things; references go to earlier slots of the right kind.
    let kind_of = |id: &str| f.slot(id).map(|(_, s)| s.bind.kind());
    for (i, s) in f.slots.iter().enumerate() {
        if !(s.id.starts_with(|c: char| c.is_ascii_lowercase())
            && s.id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
        {
            err(
                issues,
                format!(
                    "slot id `{}` must be [a-z][a-z0-9_]* (a Fluent argument)",
                    s.id
                ),
            );
        }
        for (r, want) in s.bind.refers() {
            match f.slot(r) {
                Some((j, other)) if j < i => {
                    if other.bind.kind() != want {
                        err(
                            issues,
                            format!(
                                "slot `{}` refers to `{r}`, which is a {} slot, not a {}",
                                s.id,
                                other.bind.kind().label(),
                                want.label()
                            ),
                        );
                    }
                    if other.optional && !s.optional {
                        err(
                            issues,
                            format!("required slot `{}` depends on optional slot `{r}`", s.id),
                        );
                    }
                }
                Some(_) => err(
                    issues,
                    format!("slot `{}` refers to a later slot `{r}`", s.id),
                ),
                None => err(
                    issues,
                    format!("slot `{}` refers to unknown slot `{r}`", s.id),
                ),
            }
        }
        bind_targets(b, owner, &s.id, &s.bind, issues);
    }

    // 3. `@slot` references and mechanisms.
    let all_params: BTreeSet<&str> = f.all_params().map(|p| p.id.as_str()).collect();
    for prm in f.all_params() {
        if !(prm.min <= prm.default && prm.default <= prm.max && prm.step > 0.0) {
            err(
                issues,
                format!("parameter `{}` has an invalid range", prm.id),
            );
        }
    }
    let fixed: BTreeSet<&str> = f
        .axes
        .iter()
        .flat_map(|a| &a.options)
        .flat_map(|o| o.fixed.keys().map(|k| k.as_str()))
        .collect();
    for k in &fixed {
        if all_params.contains(k) {
            err(
                issues,
                format!("fixed value `{k}` shadows a slider of the same id"),
            );
        }
    }
    let base_params: BTreeSet<&str> = f.base.params.iter().map(|p| p.id.as_str()).collect();
    let mech_check = |ms: &[MechanismDef], known: &BTreeSet<&str>, issues: &mut Vec<Issue>| {
        for m in ms {
            mechanism(b, owner, m, known, &kind_of, issues);
        }
    };
    mech_check(&f.base.mechanisms, &base_params, issues);
    if f.base.mechanisms.is_empty() {
        err(
            issues,
            "a family's base needs at least one mechanism".into(),
        );
    }
    cost(owner, "base", &f.base.cost, &base_params, issues);
    for fx in &f.base.side_effects {
        side_effect(owner, fx, &base_params, issues);
    }
    signature(b, owner, &f.base.signature, issues);
    if f.base.secondary_keys.is_empty() {
        err(
            issues,
            "a family needs at least one secondary effect (REQ-006)".into(),
        );
    }
    if let Some(w) = &f.base.works {
        works(owner, w, &kind_of, issues);
    }

    // 5. Axes.
    if !(1..=3).contains(&f.axes.len()) {
        err(issues, "a family has 1–3 axes".into());
    }
    let area_axes = f
        .axes
        .iter()
        .filter(|a| a.options.iter().any(|o| o.area.is_some()))
        .count();
    if area_axes > 1 {
        err(issues, "at most one axis per family sets `area`".into());
    }
    let option_exists = |r: &str| -> bool {
        r.split_once('.')
            .is_some_and(|(a, o)| f.axis(a).is_some_and(|ax| ax.option(o).is_some()))
    };
    for a in &f.axes {
        if !(2..=3).contains(&a.options.len()) {
            err(issues, format!("axis `{}` has 2–3 options", a.id));
        }
        if let Some(d) = &a.default
            && a.option(d).is_none()
        {
            err(
                issues,
                format!("axis `{}` default `{d}` is not one of its options", a.id),
            );
        }
        for o in &a.options {
            let mut known: BTreeSet<&str> = base_params.clone();
            known.extend(o.params.iter().map(|p| p.id.as_str()));
            known.extend(o.fixed.keys().map(|k| k.as_str()));
            mech_check(&o.mechanisms, &known, issues);
            for fx in &o.side_effects {
                side_effect(owner, fx, &known, issues);
            }
            if let Some(c) = &o.cost {
                cost(
                    owner,
                    &format!("option `{}.{}`", a.id, o.id),
                    c,
                    &known,
                    issues,
                );
                if let (Some(bp), Some(op)) = (&f.base.cost.param, &c.param)
                    && bp != op
                {
                    err(
                        issues,
                        format!(
                            "option `{}.{}` cost uses `{op}`; an option's cost uses the base's \
                             parameter `{bp}` or none",
                            a.id, o.id
                        ),
                    );
                }
            }
            signature(b, owner, &o.signature, issues);
            for (what, v, lo, hi) in [
                ("cost_factor", o.cost_factor, 0.1, 5.0),
                ("effect_factor", o.effect_factor, 0.1, 5.0),
            ] {
                if !(v.is_finite() && (lo..=hi).contains(&v)) {
                    err(
                        issues,
                        format!("option `{}.{}` {what} must be in [{lo}, {hi}]", a.id, o.id),
                    );
                }
            }
            if !(-60..=60).contains(&o.months) {
                err(
                    issues,
                    format!("option `{}.{}` months must be in -60..60", a.id, o.id),
                );
            }
            if let Some(ar) = &o.area {
                area(owner, ar, &kind_of, issues);
            }
            for x in &o.excludes {
                if !option_exists(x) {
                    err(
                        issues,
                        format!("option `{}.{}` excludes unknown `{x}`", a.id, o.id),
                    );
                } else if x.split_once('.').is_some_and(|(ax, _)| ax == a.id) {
                    err(
                        issues,
                        format!("option `{}.{}` excludes `{x}` on its own axis", a.id, o.id),
                    );
                }
            }
            if let Some(w) = &o.works {
                works(owner, w, &kind_of, issues);
            }
        }
    }
    // Every axis keeps at least one compatible choice for each option of
    // every other axis (excludes never empty an axis).
    for a in &f.axes {
        for o in &a.options {
            let key = format!("{}.{}", a.id, o.id);
            for other in f.axes.iter().filter(|x| x.id != a.id) {
                let ok = other.options.iter().any(|p| {
                    let pk = format!("{}.{}", other.id, p.id);
                    !o.excludes.contains(&pk) && !p.excludes.contains(&key)
                });
                if !ok {
                    err(
                        issues,
                        format!("`{key}` excludes every option of axis `{}`", other.id),
                    );
                }
            }
        }
    }

    // 6. DEC-073 per option, and a simulated downside for the family.
    for a in &f.axes {
        for (i, x) in a.options.iter().enumerate() {
            for y in a.options.iter().skip(i + 1) {
                let (vx, vy) = (tradeoff(x), tradeoff(y));
                if vx == vy {
                    err(
                        issues,
                        format!(
                            "options `{}` and `{}` of axis `{}` are identical in cost and \
                             effect (DEC-073)",
                            x.id, y.id, a.id
                        ),
                    );
                } else if dominates(&vx, &vy) || dominates(&vy, &vx) {
                    let (w, l) = if dominates(&vx, &vy) { (x, y) } else { (y, x) };
                    err(
                        issues,
                        format!(
                            "option `{}` of axis `{}` is better than `{}` in every way: each \
                             option needs a cost and a benefit (DEC-073)",
                            w.id, a.id, l.id
                        ),
                    );
                }
            }
        }
    }
    let downside = !f.base.side_effects.is_empty()
        || f.base
            .mechanisms
            .iter()
            .any(|m| matches!(m, MechanismDef::WageFloor { .. }))
        || f.axes
            .iter()
            .all(|a| a.options.iter().all(|o| !o.side_effects.is_empty()));
    if !downside {
        err(
            issues,
            "family has no simulated trade-off besides money: give the base a side effect \
             (DEC-073)"
                .into(),
        );
    }

    // 4. Triggers and expiry, on the one condition language.
    let slot_ids: Vec<&str> = f.slots.iter().map(|s| s.id.as_str()).collect();
    for (what, t) in [
        ("trigger", f.trigger.as_ref()),
        ("expiry", f.expiry.when.as_ref()),
    ] {
        let Some(t) = t else {
            continue;
        };
        if t.all.is_empty() && t.any.is_empty() {
            err(issues, format!("the {what} needs at least one condition"));
        }
        let site = Site::new(owner, ConditionUse::FamilyTrigger, &slot_ids).tier(f.tier);
        crate::condition::check_all(b, &site, &t.all, issues);
        crate::condition::check_all(b, &site, &t.any, issues);
        let mut flat = Vec::new();
        crate::condition::walk(&t.all, &mut flat);
        crate::condition::walk(&t.any, &mut flat);
        for c in flat {
            // The family system runs monthly on the simulation's reads:
            // there is no part of the day.
            if matches!(c, Condition::DayPart { .. }) {
                err(
                    issues,
                    format!("`day_part` in a family's {what}: it never holds there"),
                );
            }
            if let Some((slot, want)) = c.family_slot()
                && let Some(k) = kind_of(slot)
                && k != want
            {
                err(
                    issues,
                    format!(
                        "`{}` names slot `{slot}`, a {} slot; it needs a {} slot",
                        c.key(),
                        k.label(),
                        want.label()
                    ),
                );
            }
            if f.tier != TierKey::Local
                && c.tier() == CondTier::Local
                && matches!(c, Condition::Any { .. })
            {
                err(
                    issues,
                    "an `any` with a town-only alternative in a non-local family (DEC-098)".into(),
                );
            }
        }
    }
    if f.expiry.grace_months == 0 || f.expiry.grace_months > 120 {
        err(issues, "expiry grace_months must be 1–120".into());
    }
    if f.expiry.max_months.is_some_and(|m| m == 0 || m > 240) {
        err(issues, "expiry max_months must be 1–240".into());
    }
    if f.expiry.cooldown_months > 240 {
        err(issues, "expiry cooldown_months must be at most 240".into());
    }

    // 7. Sponsors.
    for s in &f.sponsors {
        match kind_of(&s.slot) {
            Some(SlotKind::Person) => {}
            Some(k) => err(
                issues,
                format!(
                    "sponsor `{}` slot `{}` is a {} slot, not a person",
                    s.id,
                    s.slot,
                    k.label()
                ),
            ),
            None => err(
                issues,
                format!("sponsor `{}` names unknown slot `{}`", s.id, s.slot),
            ),
        }
        let mut chosen = Vec::new();
        for (axis, opt) in &s.preset {
            match f.axis(axis) {
                None => err(
                    issues,
                    format!("sponsor `{}` preset names unknown axis `{axis}`", s.id),
                ),
                Some(a) => match a.option(opt) {
                    None => err(
                        issues,
                        format!(
                            "sponsor `{}` preset names unknown option `{axis}.{opt}`",
                            s.id
                        ),
                    ),
                    Some(o) => {
                        if o.needs.is_some() {
                            err(
                                issues,
                                format!(
                                    "sponsor `{}` presets an unavailable option `{axis}.{opt}`",
                                    s.id
                                ),
                            );
                        }
                        chosen.push((format!("{axis}.{opt}"), o));
                    }
                },
            }
        }
        for (k, o) in &chosen {
            for x in &o.excludes {
                if chosen.iter().any(|(k2, _)| k2 == x) {
                    err(
                        issues,
                        format!(
                            "sponsor `{}` preset combines `{k}` with excluded `{x}`",
                            s.id
                        ),
                    );
                }
            }
        }
        for (k, v) in &s.params {
            match f.all_params().find(|p| &p.id == k) {
                None => err(
                    issues,
                    format!("sponsor `{}` sets unknown parameter `{k}`", s.id),
                ),
                Some(p) => {
                    if !(v.is_finite() && *v >= p.min && *v <= p.max) {
                        err(
                            issues,
                            format!("sponsor `{}` parameter `{k}` out of range", s.id),
                        );
                    }
                }
            }
        }
        match &s.price {
            PriceDef::Favour { count, .. } => {
                if !(1..=3).contains(count) {
                    err(
                        issues,
                        format!("sponsor `{}` favour count must be 1–3", s.id),
                    );
                }
            }
            PriceDef::Endorsement {
                approval,
                backlash,
                area: ar,
                ..
            } => {
                for (w, v) in [("approval", approval), ("backlash", backlash)] {
                    if !(v.is_finite() && (0.0..=0.1).contains(v)) {
                        err(
                            issues,
                            format!("sponsor `{}` {w} must be in [0, 0.1]", s.id),
                        );
                    }
                }
                if let Some(ar) = ar {
                    area(owner, ar, &kind_of, issues);
                }
            }
        }
    }

    // 8. Knowledge hooks.
    let k = &f.knowledge;
    for h in k.heard.iter().chain(&k.known).chain(&k.detailed) {
        hook(b, owner, h, &kind_of, issues);
    }
    if !k.public && k.heard.is_empty() {
        err(
            issues,
            "a family that is not public needs at least one `heard` hook".into(),
        );
    }
    // Drafting needs `known`. (DEC-079: nothing critical only at a hidden
    // place. The registry has no hidden place kinds until G2; its check
    // joins here then.)
    if k.known.is_empty() {
        err(
            issues,
            "a family needs at least one `known` hook (drafting needs it)".into(),
        );
    }
    let mut lead = k.lead.as_ref();
    while let Some(l) = lead {
        match &l.to {
            LeadToDef::Slot { slot } => {
                if kind_of(slot).is_none() {
                    err(issues, format!("lead names unknown slot `{slot}`"));
                }
            }
            LeadToDef::Streets { slot } => {
                if kind_of(slot) != Some(SlotKind::Neighborhood) {
                    err(
                        issues,
                        format!("lead streets `{slot}` is not a neighborhood slot"),
                    );
                }
            }
            LeadToDef::PlaceKind { kind } => place_kind(b, owner, kind, issues),
        }
        lead = l.r#else.as_deref();
    }

    // 10. Draw.
    if !(f.draw.weight.is_finite() && f.draw.weight > 0.0 && f.draw.weight <= 10.0) {
        err(issues, "draw weight must be in (0, 10]".into());
    }

    // 9. Fluent keys exist, with only slot arguments.
    let allowed: BTreeSet<String> = f
        .slots
        .iter()
        .map(|s| {
            if s.bind.kind().keyed_arg() {
                format!("{}_key", s.id)
            } else {
                s.id.clone()
            }
        })
        .collect();
    for key in fluent_keys_of(f) {
        match en.get(key) {
            None => err(issues, format!("missing Fluent key `{key}` in locale en")),
            Some(vars) => {
                for v in vars {
                    if !allowed.contains(v) {
                        err(
                            issues,
                            format!(
                                "Fluent key `{key}` uses `{{ ${v} }}`, which is not a slot of \
                                 this family (slots: {})",
                                allowed.iter().cloned().collect::<Vec<_>>().join(", ")
                            ),
                        );
                    }
                }
            }
        }
    }
}

/// Every Fluent key a family names.
pub fn fluent_keys_of(f: &PolicyFamilyDef) -> Vec<&str> {
    let mut v: Vec<&str> = vec![&f.name_key, &f.desc_key];
    for t in f.trigger.iter().chain(f.expiry.when.iter()) {
        v.push(&t.reason_key);
    }
    v.extend(f.base.secondary_keys.iter().map(|s| s.as_str()));
    for p in f.all_params() {
        v.push(&p.name_key);
        v.push(&p.unit_key);
    }
    for a in &f.axes {
        v.push(&a.name_key);
        for o in &a.options {
            v.push(&o.name_key);
            v.push(&o.desc_key);
            v.extend(o.secondary_keys.iter().map(|s| s.as_str()));
        }
    }
    for s in &f.sponsors {
        v.push(&s.name_key);
        if let PriceDef::Favour { context_key, .. } = &s.price {
            v.push(context_key);
        }
    }
    let mut lead = f.knowledge.lead.as_ref();
    while let Some(l) = lead {
        v.push(&l.key);
        lead = l.r#else.as_deref();
    }
    v.sort();
    v.dedup();
    v
}

fn place_kind(b: &ContentBundle, owner: &str, kind: &str, issues: &mut Vec<Issue>) {
    if b.place_kind(kind).is_none() || crate::types::PSEUDO_PLACES.contains(&kind) {
        issues.push(Issue::error(owner, &format!("unknown place kind `{kind}`")));
    }
}

fn bind_targets(
    b: &ContentBundle,
    owner: &str,
    slot: &str,
    bind: &BindDef,
    issues: &mut Vec<Issue>,
) {
    let err = |issues: &mut Vec<Issue>, m: String| issues.push(Issue::error(owner, &m));
    let exposure = |v: f64, issues: &mut Vec<Issue>| {
        if !(v.is_finite() && (0.0..=1.0).contains(&v)) {
            issues.push(Issue::error(
                owner,
                &format!("slot `{slot}` min_exposure must be in [0, 1]"),
            ));
        }
    };
    match bind {
        BindDef::Place { kinds, pick, .. } => {
            if kinds.is_empty() {
                err(issues, format!("place slot `{slot}` needs kinds"));
            }
            for k in kinds {
                place_kind(b, owner, k, issues);
            }
            if !matches!(
                pick,
                Pick::First | Pick::MostAtRisk | Pick::Largest | Pick::Keyed
            ) {
                err(
                    issues,
                    format!("place slot `{slot}` cannot pick `{pick:?}`"),
                );
            }
        }
        BindDef::Neighborhood {
            kinds,
            concern,
            lacks,
            pick,
            min_exposure,
            hazard,
            top,
            ..
        } => {
            for k in kinds {
                if !crate::types::NEIGHBORHOOD_KINDS.contains(&k.as_str()) {
                    err(
                        issues,
                        format!("slot `{slot}`: unknown neighbourhood kind `{k}`"),
                    );
                }
            }
            if let Some(c) = concern
                && b.concern_index(c).is_none()
            {
                err(issues, format!("slot `{slot}`: unknown concern `{c}`"));
            }
            if *top == 0 {
                err(issues, format!("slot `{slot}`: top must be at least 1"));
            }
            for k in lacks {
                place_kind(b, owner, k, issues);
            }
            exposure(*min_exposure, issues);
            if *pick == Pick::MostExposed && hazard.is_none() {
                err(
                    issues,
                    format!("slot `{slot}` picks most_exposed without a hazard"),
                );
            }
            if *pick == Pick::MostConcerned && concern.is_none() {
                err(
                    issues,
                    format!("slot `{slot}` picks most_concerned without a concern"),
                );
            }
            if *pick == Pick::MostAtRisk {
                err(
                    issues,
                    format!("neighbourhood slot `{slot}` cannot pick most_at_risk"),
                );
            }
        }
        BindDef::Person { occupations, .. } => {
            for o in occupations {
                if b.occupation(o).is_none() {
                    err(issues, format!("slot `{slot}`: unknown occupation `{o}`"));
                }
            }
        }
        BindDef::Broker => {}
        BindDef::Sector {
            of_slot,
            drivers,
            defining,
        } => {
            if of_slot.is_none() && drivers.is_empty() && !defining {
                err(
                    issues,
                    format!("sector slot `{slot}` needs `of_slot`, `drivers` or `defining`"),
                );
            }
        }
        BindDef::Crisis { kinds } => {
            for k in kinds {
                if b.crisis_index(k).is_none() {
                    err(issues, format!("slot `{slot}`: unknown crisis `{k}`"));
                }
            }
        }
        BindDef::Hazard {
            hazards,
            min_exposure,
            pick,
            ..
        } => {
            if hazards.is_empty() {
                err(issues, format!("hazard slot `{slot}` needs hazards"));
            }
            exposure(*min_exposure, issues);
            if !matches!(pick, Pick::First | Pick::MostExposed | Pick::Keyed) {
                err(
                    issues,
                    format!("hazard slot `{slot}` cannot pick `{pick:?}`"),
                );
            }
        }
    }
}

fn mechanism(
    b: &ContentBundle,
    owner: &str,
    m: &MechanismDef,
    known: &BTreeSet<&str>,
    kind_of: &dyn Fn(&str) -> Option<SlotKind>,
    issues: &mut Vec<Issue>,
) {
    use MechanismDef as M;
    let err = |issues: &mut Vec<Issue>, s: String| issues.push(Issue::error(owner, &s));
    if let Some(p) = m.param()
        && !known.contains(p)
    {
        err(
            issues,
            format!("mechanism references unknown parameter `{p}`"),
        );
    }
    let sector = |s: &str, issues: &mut Vec<Issue>| match slot_ref(s) {
        Some(slot) => {
            if kind_of(slot) != Some(SlotKind::Sector) {
                issues.push(Issue::error(
                    owner,
                    &format!("`{s}` must name a sector slot of this family"),
                ));
            }
        }
        None => {
            if b.sector_index(s).is_none() {
                issues.push(Issue::error(owner, &format!("unknown sector `{s}`")));
            }
        }
    };
    if is_family_mechanism(m) && !crate::validate::FAMILY_MECHANISMS_SIMULATED {
        err(
            issues,
            format!(
                "mechanism `{}` is declared but not yet simulated (F2b)",
                m.key()
            ),
        );
    }
    match m {
        M::SectorSubsidy { sector: s, .. } | M::SectorCapacity { sector: s, .. } => {
            sector(s, issues)
        }
        M::HazardProtection {
            hazard: crate::types::HazardRef::Slot(s),
            ..
        } => {
            if slot_ref(s).and_then(kind_of) != Some(SlotKind::Hazard) {
                err(
                    issues,
                    format!("`{s}` must be `@` and a hazard slot of this family"),
                );
            }
        }
        M::Borrow { share } => {
            if !(share.is_finite() && *share > 0.0 && *share <= 1.0) {
                err(issues, "borrow share must be in (0, 1]".into());
            }
        }
        M::Revenue {
            per_capita,
            per_unit,
            param,
        } => {
            if !(per_capita.is_finite()
                && per_unit.is_finite()
                && *per_capita >= 0.0
                && *per_unit >= 0.0)
                || (*per_capita == 0.0 && *per_unit == 0.0)
            {
                err(
                    issues,
                    "revenue needs a positive per_capita or per_unit".into(),
                );
            }
            if *per_unit != 0.0 && param.is_none() {
                err(issues, "revenue per_unit needs a param".into());
            }
        }
        _ => {}
    }
}

fn cost(owner: &str, what: &str, c: &CostDef, known: &BTreeSet<&str>, issues: &mut Vec<Issue>) {
    if let Some(p) = &c.param
        && !known.contains(p.as_str())
    {
        issues.push(Issue::error(
            owner,
            &format!("{what} cost references unknown parameter `{p}`"),
        ));
    }
    for v in [c.per_capita, c.per_capita_per_unit, c.per_unit] {
        if !v.is_finite() {
            issues.push(Issue::error(owner, &format!("{what} cost must be finite")));
        }
    }
}

fn side_effect(owner: &str, fx: &SideEffectDef, known: &BTreeSet<&str>, issues: &mut Vec<Issue>) {
    if let Some(p) = fx.param()
        && !known.contains(p)
    {
        issues.push(Issue::error(
            owner,
            &format!("side effect references unknown parameter `{p}`"),
        ));
    }
}

fn signature(b: &ContentBundle, owner: &str, s: &BTreeMap<String, f64>, issues: &mut Vec<Issue>) {
    for (k, v) in s {
        if b.attitude_index(k).is_none() {
            issues.push(Issue::error(
                owner,
                &format!("signature references unknown attitude `{k}`"),
            ));
        }
        if !(v.is_finite() && (-1.0..=1.0).contains(v)) {
            issues.push(Issue::error(
                owner,
                &format!("signature weight for `{k}` must be in [-1, 1]"),
            ));
        }
    }
}

fn area(
    owner: &str,
    a: &AreaDef,
    kind_of: &dyn Fn(&str) -> Option<SlotKind>,
    issues: &mut Vec<Issue>,
) {
    if let Some(s) = a.slot()
        && kind_of(s) != Some(SlotKind::Neighborhood)
    {
        issues.push(Issue::error(
            owner,
            &format!("area slot `{s}` is not a neighborhood slot"),
        ));
    }
}

fn works(
    owner: &str,
    w: &WorksDef,
    kind_of: &dyn Fn(&str) -> Option<SlotKind>,
    issues: &mut Vec<Issue>,
) {
    if w.kind.is_empty()
        || !w
            .kind
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        issues.push(Issue::error(owner, "works kind must be [a-z0-9_]+"));
    }
    if !matches!(
        kind_of(&w.at),
        Some(SlotKind::Place | SlotKind::Neighborhood)
    ) {
        issues.push(Issue::error(
            owner,
            &format!("works `at` `{}` must be a place or neighborhood slot", w.at),
        ));
    }
    if w.months.is_some_and(|m| m == 0 || m > 120) {
        issues.push(Issue::error(owner, "works months must be 1–120"));
    }
    if !(w.contractor_risk.is_finite() && (0.0..=0.2).contains(&w.contractor_risk)) {
        issues.push(Issue::error(
            owner,
            "works contractor_risk must be in [0, 0.2]",
        ));
    }
}

fn hook(
    b: &ContentBundle,
    owner: &str,
    h: &HookDef,
    kind_of: &dyn Fn(&str) -> Option<SlotKind>,
    issues: &mut Vec<Issue>,
) {
    let err = |issues: &mut Vec<Issue>, m: String| issues.push(Issue::error(owner, &m));
    match h {
        HookDef::Overheard { at } => {
            if at.is_empty() {
                err(issues, "an overheard hook needs place kinds".into());
            }
            for k in at {
                place_kind(b, owner, k, issues);
            }
        }
        HookDef::Visit { slot } => {
            if !matches!(
                kind_of(slot),
                Some(SlotKind::Place | SlotKind::Neighborhood)
            ) {
                err(
                    issues,
                    format!("visit hook `{slot}` must be a place or neighborhood slot"),
                );
            }
        }
        HookDef::Meet { slot } | HookDef::Approach { slot, .. } => {
            if kind_of(slot) != Some(SlotKind::Person) {
                err(issues, format!("hook slot `{slot}` must be a person slot"));
            }
        }
        HookDef::Talk { slot, topic } => {
            if kind_of(slot) != Some(SlotKind::Person) {
                err(
                    issues,
                    format!("talk hook slot `{slot}` must be a person slot"),
                );
            }
            if b.talk_topic(topic).is_none() {
                err(issues, format!("talk hook names unknown topic `{topic}`"));
            }
        }
        HookDef::Action { action, slot } => {
            if !crate::types::PLACE_ACTIONS.contains(&action.as_str()) {
                err(issues, format!("unknown action `{action}`"));
            }
            if let Some(s) = slot
                && kind_of(s).is_none()
            {
                err(issues, format!("action hook names unknown slot `{s}`"));
            }
        }
        HookDef::Service { service } => {
            if !crate::types::PLACE_SERVICES.contains(&service.as_str()) {
                err(issues, format!("unknown service `{service}`"));
            }
        }
    }
}

// --- DEC-073: the trade-off vector ----------------------------------------------------------

/// One option's place on the cost/benefit vector, each entry oriented so
/// that larger is better.
fn tradeoff(o: &FamilyOptionDef) -> Vec<f64> {
    use MechanismDef as M;
    let added = o
        .cost
        .as_ref()
        .map(|c| c.per_capita + c.per_capita_per_unit + c.per_unit / 1000.0)
        .unwrap_or(0.0);
    let mut debt = 0.0;
    let mut tax = 0.0;
    let mut revenue = 0.0;
    let mut gains = 0.0;
    let mut harm = 0.0;
    let mut finances = false;
    for m in &o.mechanisms {
        match m {
            M::Borrow { share } => {
                debt += share;
                finances = true;
            }
            M::LocalTaxRate { .. } => {
                tax += 1.0;
                finances = true;
            }
            M::Revenue { .. } => {
                revenue += 1.0;
                finances = true;
            }
            other => {
                if other.sign() < 0.0 {
                    harm += 1.0;
                } else {
                    gains += 1.0;
                }
            }
        }
    }
    for fx in &o.side_effects {
        let a = match fx {
            SideEffectDef::Disruption { per_unit, .. }
            | SideEffectDef::Grievance { per_unit, .. }
            | SideEffectDef::LabourSupply { per_unit, .. } => *per_unit,
        };
        if a > 0.0 {
            harm += 1.0 + a;
        } else {
            gains += 1.0 - a;
        }
    }
    // Paid from this year's budget unless it is financed another way.
    let budget = if finances { 0.0 } else { 1.0 };
    vec![
        -added,
        -o.cost_factor,
        o.effect_factor,
        -(o.months as f64),
        -harm,
        gains,
        AreaDef::rank(o.area.as_ref()),
        -debt,
        -tax,
        revenue,
        -budget,
        // Unavailable options (M7) are their own trade-off.
        if o.needs.is_some() { -1.0 } else { 0.0 },
    ]
}

/// `a` is at least as good as `b` everywhere and better somewhere.
fn dominates(a: &[f64], b: &[f64]) -> bool {
    a.iter().zip(b).all(|(x, y)| x >= y) && a.iter().zip(b).any(|(x, y)| x > y)
}
