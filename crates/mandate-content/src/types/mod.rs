//! Content types. These Rust types are the single source of truth; JSON
//! Schemas are generated from them (DEC-053).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod action_params;
mod characters;
mod concerns;
mod constitution;
mod council;
mod economy;
mod files;
mod impact;
mod opinion;
mod places;
mod policy;
mod politics;
mod scenario;
mod tuning;

pub use action_params::*;
pub use characters::*;
pub use concerns::*;
pub use constitution::*;
pub use council::*;
pub use economy::*;
pub(crate) use files::*;
pub use impact::*;
pub use opinion::*;
pub use places::*;
pub use policy::*;
pub use politics::*;
pub use scenario::*;
pub use tuning::*;

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
    /// Industries: a registry level under the sectors (DEC-110).
    Industries { items: Vec<IndustryDef> },
    /// Markets: where industries sell outside the country (DEC-110).
    Markets { items: Vec<MarketDef> },
    /// Goods: what products are, for trade to name (DEC-110).
    Goods { items: Vec<GoodDef> },
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
    Economy { value: Box<EconomyParams> },
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
    /// Interior recipes: the rooms of enterable places (DEC-088, DEC-098 §4).
    Interiors {
        items: Vec<crate::interiors::InteriorRecipe>,
    },
    /// The country map's layers (DEC-089): what the map can colour towns by.
    MapLayers {
        items: Vec<crate::map_layers::MapLayerDef>,
    },
    /// The town map's layers (DEC-086): what the town map can show over the town.
    TownMapLayers {
        items: Vec<crate::town_map_layers::TownMapLayerDef>,
    },
    /// Daily routines: where people are through the day (DEC-031).
    Routines {
        items: Vec<crate::routines::RoutineDef>,
    },
    /// The town calendar: recurring events attended by routine (DEC-114).
    Calendar {
        items: Vec<crate::calendar::CalendarEventDef>,
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
    /// The asks system's numbers: the sliding rate, caps, casework, promises (DEC-097 §2).
    AskParams { value: Box<crate::asks::AskParams> },
    /// Ask templates: concrete requests with a deadline (DEC-097 §2).
    Asks { items: Vec<crate::asks::AskDef> },
    /// Campaign promise templates (DEC-097 §2).
    Promises { items: Vec<crate::asks::PromiseDef> },
    /// The act flow (DEC-012): time-skip directives, interrupts, skip
    /// targets, the player-proxy and the Legacy record.
    ActFlow {
        value: Box<crate::act::ActFlowParams>,
    },
    /// How acts end (DEC-006; DEC-098 note): the endings registry.
    Endings { items: Vec<crate::act::EndingDef> },
    /// Staged arcs (DEC-109): one matter in one jurisdiction, in stages.
    /// A file may carry the dilemmas, asks, scenes and overheard lines its
    /// beats name; they join the same registries after every other file's.
    Arcs {
        items: Vec<crate::arcs::ArcDef>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        dilemmas: Vec<crate::narrative::DilemmaDef>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        asks: Vec<crate::asks::AskDef>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        scenes: Vec<crate::narrative::SceneDef>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        overheard: Vec<OverheardDef>,
    },
    /// Exchange scenes (DEC-112): every constant of the mechanism.
    ExchangeParams {
        value: Box<crate::exchange::ExchangeParams>,
    },
    /// Policy families: proposals grounded in the town (DEC-092).
    PolicyFamilies {
        items: Vec<crate::policy_family::PolicyFamilyDef>,
    },
    /// The family numbers: rarity chances, the cap on open proposals,
    /// sponsor goodwill, works (DEC-092, DEC-099).
    PolicyFamilyParams {
        value: Box<crate::policy_family::FamilyParams>,
    },
    /// Targeted changes to families of earlier packs (DEC-092), in pack order.
    PolicyFamilyPatches {
        items: Vec<crate::policy_family::FamilyPatch>,
    },
    /// The impact table (DEC-111): pass-through coefficients fitted in the
    /// lab (`mandate-cli lab impact calibrate` writes the file).
    ImpactTable { value: Box<ImpactTable> },
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
