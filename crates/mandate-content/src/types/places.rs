//! Place kinds (DEC-078 decision 6, DEC-079) and the action and service registries they offer.

use super::*;

// --- Place kinds (DEC-078 decision 6, DEC-079) --------------------------------

/// The player actions a place can offer. The kinds of action stay in code
/// (DEC-078 decision 7); these keys match the action-cost keys in tuning
/// where an action costs time. `vote`, `draft`, `file`, and `withdraw` are free.
pub const PLACE_ACTIONS: [&str; 18] = [
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
    "casework",
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

/// Neighbourhood kinds, as worldgen makes them (policy-family selectors).
pub const NEIGHBORHOOD_KINDS: [&str; 6] = [
    "core",
    "inner",
    "waterfront",
    "industrial",
    "suburb",
    "rural",
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
    /// Verbs of the `visit` action offered here (`actions.jsonc`
    /// `visit.verbs` ids; places with purpose, DEC-079 note).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verbs: Vec<ContentKey>,
    /// The industry this kind of place is part of (a sector id): what a
    /// verb's `sector` grant and its risk's `shedding` read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sector: Option<ContentKey>,
    /// The sector drivers of this kind of place (DEC-092 families: a plant's
    /// sector, `drivers` selectors): the factory and the mill are
    /// `industrial`, the mine `extraction`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drivers: Vec<SectorDriver>,
    /// The concerns its service addresses (concern ids): its real need is
    /// its neighbourhood's felt level of them (DEC-092 §2.22), what a
    /// verb's `service` grant reads.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<ContentKey>,
    /// The gateway this kind of place is (DEC-110): where trade passes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gateway: Option<GatewayKind>,
    /// Where this kind's actions and services move in a town (or, for
    /// neighbourhood actions, a neighbourhood) that has no such place.
    #[serde(default)]
    pub fallback: Option<String>,
    /// How the town layout draws it (DEC-090 D4, TOWN_GENERATION.md §6.3):
    /// presentation data, validated but never read by the simulation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<crate::layout::PlaceLayoutBlock>,
    /// The interior recipe the player walks into (DEC-088; `interiors.jsonc`
    /// id), presentation data like `layout`. A kind without one keeps the
    /// place panel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interior: Option<String>,
}
