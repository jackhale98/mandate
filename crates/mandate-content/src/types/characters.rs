//! Characters (DEC-031, DEC-018): traits, occupations, crises, memories, bonds.

use super::*;

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
    /// Seeding round: a town's people of later rounds are generated after
    /// everyone of earlier rounds, so occupations added later append to the
    /// cast instead of renumbering it (draws are keyed by place and
    /// occupation either way).
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub round: u8,
}

fn is_zero_u8(v: &u8) -> bool {
    *v == 0
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

/// What the power broker does (M6 note 27): leans on votes in their
/// interest, backs a candidate, and what knowing them opens to the player.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BrokerParams {
    /// The vote model's `broker` term: the pull on a member's lean on a
    /// bill in the broker's interest, × the broker's influence × how far
    /// the member goes along with them (0 when they loathe the broker, 1
    /// when they admire them; ½ when indifferent).
    pub vote_pull: f64,
    /// The influence of a broker with no organisation (an officeholder),
    /// in place of an occupation's `influence`.
    pub officeholder_influence: f64,
    /// Campaign backing: money a week to the candidate the broker favours,
    /// and the organisation's contacts (canvass-equivalents) a week.
    pub back_money: f64,
    pub back_contacts: u32,
    /// How much the broker thinks of a candidate: their opinion of them,
    /// plus this × how far the candidate's views match theirs (−1..1).
    pub back_values: f64,
    /// The broker backs no one they think less of than this.
    pub back_min_opinion: f64,
    /// Lobbying the broker: sway on each member of the bill's body, × how
    /// far they go along with the broker. The broker refuses below
    /// `lobby_min_opinion` of the player.
    pub lobby_sway: f64,
    pub lobby_min_opinion: f64,
    /// Doing them a favour: their opinion of the player, and approval among
    /// those who resent them (× room; usually negative).
    pub favour_opinion: f64,
    pub favour_resent_approval: f64,
    /// Opposing them openly: approval among those who resent them and those
    /// who depend on them; their opinion of the player; and the opinion of
    /// the player of each member who likes them (× their liking, 0..1).
    pub oppose_resent_approval: f64,
    pub oppose_depend_approval: f64,
    pub oppose_opinion: f64,
    pub oppose_ally_opinion: f64,
    /// Exposing them through the press (the press's gaffe risk applies):
    /// on success each member's opinion of the broker and approval among
    /// those who resent them; either way their opinion of the player; on a
    /// gaffe, approval among everyone (× room).
    pub expose_member_opinion: f64,
    pub expose_resent_approval: f64,
    pub expose_opinion: f64,
    pub expose_gaffe_approval: f64,
    /// A stand against them (opposing, exposing) is not offered again
    /// within this many days of the last.
    pub cooldown_days: u32,
    /// Who an exposé is told to (occupation ids: the press).
    pub press_occupations: Vec<ContentKey>,
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
    pub broker: BrokerParams,
}

impl CharacterParams {
    pub fn memory(&self, kind: &str) -> Option<&MemoryKindParams> {
        self.memories.get(kind)
    }
}
