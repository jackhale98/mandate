//! Narrative content (DEC-033, DEC-032, DEC-077 §4, DEC-035): the director's
//! parameter block, storyteller profiles, dilemmas and requests, NPC goals,
//! and scene templates. (Storylines are arcs now: `crate::arcs`, DEC-109.)
//!
//! Casts, conditions, and option effects are closed enums: each variant is
//! cast, evaluated, or applied by `mandate_sim::narrative`, so a new variant
//! is a code extension point (one variant, one handler, a test). Everything
//! is scoped to the player's town: roles are cast from live actors there, and
//! conditions read live state there.

use crate::condition::Condition;
use crate::knowledge::Level;
use crate::types::ContentKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// --- The director's parameter block -------------------------------------------------

/// Decisions and developments allowed per chapter (DEC-033 pacing model).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DirectorBudgets {
    /// Climaxes per chapter (a campaign chapter's climax is its election).
    pub climax: u8,
    /// Significant decisions per chapter, [target minimum, maximum].
    pub significant: [u8; 2],
    /// Minor developments (requests, news of the town), [target minimum, maximum].
    pub minor: [u8; 2],
}

/// The chapter's tension curve: it rises from `start` to `peak` at `peak_at`
/// (a share of the chapter), then falls to `end`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TensionParams {
    pub start: f64,
    pub peak: f64,
    pub peak_at: f64,
    pub end: f64,
    /// The share of a session chapter in which its climax may come, [from, to].
    pub climax_window: [f64; 2],
    /// A significant decision is spotlighted while tension is below the curve plus this.
    pub slack: f64,
    /// From this share of a chapter on, a chapter short of its minimum spotlights whatever is ready.
    pub catch_up_at: f64,
    /// Weights of live tension: the town's worst crisis, pending decisions, the player's race, and track danger.
    pub crisis_weight: f64,
    pub pending_weight: f64,
    pub race_weight: f64,
    pub danger_weight: f64,
}

/// Relevance scoring (DEC-033): what connects a situation to the player.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RelevanceWeights {
    pub base: f64,
    /// A cast person has history with the player (an edge, a favour).
    pub player: f64,
    /// The situation is in the player's ward.
    pub ward: f64,
    /// × the live crisis's severity, when the situation answers to it.
    pub crisis: f64,
    /// Per salient memory a cast person holds of the player (up to three).
    pub memory: f64,
    /// A beat of a running arc.
    #[serde(alias = "storyline")]
    pub arc: f64,
    /// It reads one of the player's pressure tracks.
    pub pressure: f64,
    /// The player's seat is up within a year.
    pub election: f64,
    /// It answers one of the town's top concerns.
    pub concern: f64,
    /// Keyed random jitter (variety among equals).
    pub jitter: f64,
}

/// Fairness (DEC-006): a warning before any terminal ending. A track at or
/// past its threshold is warned about (pinned event, the pocket's alerts).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FairnessParams {
    /// Warn at or below these (0 is the terminal end of each track).
    pub standing: f64,
    pub viability: f64,
    pub health: f64,
    /// Warn at or above these (100 is the terminal end).
    pub legal_jeopardy: f64,
    pub threat: f64,
    /// Repeat a warning while the track stays in danger after this many days.
    pub repeat_days: u32,
    /// An ending may follow a warning only after this many days (a chance to respond).
    pub response_days: u32,
}

/// The director's exogenous levers (DEC-033 B).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChanceParams {
    /// A hold on hazards in the player's town lasts this long before it is reviewed.
    pub hazard_hold_days: u32,
    /// A held hazard strikes at most this many days after it was drawn.
    pub hazard_max_defer_days: u32,
    /// The onset of a global recession may be held at most this many months.
    pub recession_hold_months: u8,
    /// A hidden fact becomes discoverable within this window after it happened, [from, to] days.
    pub discovery_days: [u32; 2],
}

/// Phase lengths of the pacing cycle for one kind of chapter (ENGAGEMENT
/// §4.3), in days: `[from, to]` ranges are keyed draws.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CycleLengths {
    /// Minor beats only, then a peak.
    pub build_up: [u32; 2],
    /// A peak lasts until its decision is answered, at most this long.
    pub peak_max: u32,
    /// Minor beats only, then the next build-up.
    pub relax: [u32; 2],
    /// The relax after a session's climax (default: `relax`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relax_after_climax: Option<[u32; 2]>,
}

/// The pacing cycle (ENGAGEMENT §4.3, after Left 4 Dead's AI Director and
/// RimWorld): build-up, peak, peak fade and relax, across chapters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CycleParams {
    pub session: CycleLengths,
    pub campaign: CycleLengths,
    /// The relax after a setback is this much longer.
    pub setback_relax_mult: f64,
}

/// The narrative director (DEC-033): one file across all packs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DirectorParams {
    /// Storyteller profile of new games (a `storytellers` id).
    pub default_profile: ContentKey,
    /// A session chapter (a budget year in office) lasts at least this many
    /// days; a shorter one runs on to the following year.
    pub session_min_days: u32,
    pub budgets: DirectorBudgets,
    pub tension: TensionParams,
    /// Least days between two spotlighted decisions.
    pub spacing_days: u32,
    /// Least days between two minor developments.
    pub development_spacing_days: u32,
    /// Most decisions pending at once.
    pub max_pending: u8,
    /// Crises the player's town may carry before the director holds
    /// exogenous hazards back (the cap on simultaneous unresolved crises).
    pub crisis_cap: u8,
    /// Score multiplier for a template or a person already spotlighted this chapter.
    pub repeat_penalty: f64,
    /// Least relevance for a situation to be spotlighted.
    pub min_relevance: f64,
    pub relevance: RelevanceWeights,
    /// Days to decide when a dilemma gives none.
    pub deadline_days: u32,
    pub fairness: FairnessParams,
    pub chance: ChanceParams,
    /// Arcs running at once in a focus jurisdiction (DEC-109: one lead and
    /// one background), standing arcs aside.
    pub arcs_active: u8,
    /// Resolved decisions kept for the story so far.
    pub history: u16,
    /// The pacing cycle (ENGAGEMENT §4.3).
    pub cycle: CycleParams,
    /// After a session's climax and its relax, at most this many significant
    /// decisions (the coda): a consequence, an opportunity, or a `budget`
    /// template.
    pub post_climax_significant: u8,
    /// Nothing for the player this long (governing, campaigning): a minor
    /// beat is spotlighted at once, in any phase (the quiet rescue).
    pub quiet_days: u32,
    pub quiet_days_campaign: u32,
    /// A consequence (`answered`) whose window closes within this many days
    /// is drawn first (tier A, with the arcs' decisions that are due).
    pub tier_a_window_days: u32,
    /// The pick draws among candidates whose relevance is within this of the
    /// tier's best, weighted by relevance × novelty.
    pub pick_band: f64,
    /// Chapter themes and their draw weights (ENGAGEMENT §4.2): `crisis`,
    /// `budget`, `scheme`, `local_life` (campaign chapters take `campaign`).
    pub themes: BTreeMap<String, f64>,
    /// Every dilemma's cooldown is at least this many days …
    pub cooldown_floor_days: u32,
    /// … and the n-th use sets `max(floor, cooldown) × growth^(n−1)`.
    pub cooldown_growth: f64,
    /// A template spotlighted in a chapter is not a candidate again in it.
    pub no_repeat_in_chapter: bool,
}

/// The chapter themes the director knows how to read from live state
/// (ENGAGEMENT §4.2).
pub const THEMES: [&str; 5] = ["crisis", "budget", "scheme", "local_life", "campaign"];
/// The theme of every campaign chapter (and of no session).
pub const THEME_CAMPAIGN: &str = "campaign";

/// A storyteller profile (DEC-033 addendum): a transparent difficulty and
/// flavour lever over chance and pacing, never over consequences.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StorytellerDef {
    pub id: ContentKey,
    pub name_key: String,
    pub desc_key: String,
    /// Pace decisions along the chapter's tension curve (otherwise spotlight
    /// whatever is ready, within the budgets).
    pub tension_curve: bool,
    /// Hold hazards in the player's town while it carries `crisis_cap`
    /// crises. Timing only: a held hazard still strikes, at its drawn severity.
    pub defer_hazards: bool,
    /// Share of hazard strikes in the player's town that happen (1 = the
    /// hazard model as drawn).
    pub hazard_rate: f64,
    /// Hold the onset of a global recession while the player's town is at the cap.
    pub hold_recessions: bool,
    /// Multiplies the significant and minor budgets.
    pub budget_scale: f64,
    /// Overrides the director's `crisis_cap`.
    #[serde(default)]
    pub crisis_cap: Option<u8>,
    /// Pace each chapter in build-up, peak and relax phases (ENGAGEMENT
    /// §4.3); off, situations come by chance within the budgets.
    #[serde(default = "yes")]
    pub cycle: bool,
    /// Scales build-up and relax lengths: frequency, not amplitude.
    #[serde(default = "one")]
    pub build_up_scale: f64,
    #[serde(default = "one")]
    pub relax_scale: f64,
}

fn one() -> f64 {
    1.0
}

// --- Casts, conditions, effects ---------------------------------------------------------

/// Which party a cast colleague belongs to, relative to the player.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PartyRel {
    #[default]
    Any,
    Same,
    Other,
}

/// Who sponsored a cast bill.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BillSponsor {
    #[default]
    Any,
    Player,
    /// Anyone but the player.
    Other,
}

/// How a role is cast from live actors in the player's town.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Cast {
    /// The town's power broker (DEC-077 §1).
    Broker,
    /// The player's named rival.
    Rival,
    /// The holder of an office in the player's town (when it is not the
    /// player): `office` names it by constitution id; by default the town's
    /// executive (the office that signs its ordinances; DEC-098). The old
    /// spelling `{ "type": "mayor" }` reads as the executive.
    #[serde(alias = "mayor")]
    Office {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        office: Option<String>,
    },
    /// A Named person with one of these occupations: one with a stake in the
    /// live crisis first, then leaders, then the most influential.
    Occupation { occupations: Vec<ContentKey> },
    /// A council colleague (not the player).
    Colleague {
        #[serde(default)]
        party: PartyRel,
    },
    /// Someone the player owes a favour (the largest debt).
    Creditor,
    /// A bill before the council.
    Bill {
        #[serde(default)]
        sponsor: BillSponsor,
        /// Sponsored by the person cast in this role.
        #[serde(default)]
        sponsored_by: Option<String>,
    },
    /// The sponsor of the bill cast in `role`.
    SponsorOf { role: String },
    /// A neighbourhood of the player's ward: the one least pleased with
    /// the town government (where a crisis bites).
    WardNeighborhood,
    /// A law of one of these templates (any, if empty) in force or being
    /// built in town (`building`: only one still being built), sponsored by
    /// `sponsor`.
    Law {
        #[serde(default)]
        templates: Vec<ContentKey>,
        #[serde(default)]
        building: bool,
        #[serde(default)]
        sponsor: BillSponsor,
    },
    /// The person whose goal raised this request (requests only).
    GoalHolder,
    /// The colleague closest to the player (the highest opinion of them):
    /// someone to lean on, or to blame.
    Ally,
}

/// A pressure track, record, or Mandate (DEC-006, DEC-011, DEC-076).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrackKey {
    Viability,
    LegalJeopardy,
    Threat,
    Standing,
    Health,
    /// Visible work in office, 0..100.
    Record,
    Mandate,
}

/// The office words a condition can name besides role words (DEC-098):
/// `any` (holds any post) and `none` (out of office).
pub const OFFICE_ANY: &str = "any";
pub const OFFICE_NONE: &str = "none";

/// A knowledge subject named by role, never by id (DEC-091;
/// M6_ENGINE_CONTRACTS §1.3): resolved against the cast and the player's
/// town by `mandate_sim::narrative::learn`. Used by narrative `learn`
/// effects, scene `teaches` hooks, `knows` conditions, and overheard lines'
/// `teaches` (which have no roles: there, the neighbourhood is where the
/// line is heard).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SubjectDef {
    /// The player's town's worries.
    TownConcerns,
    /// A neighbourhood's worries: the neighbourhood cast in `role`, or the
    /// home neighbourhood of the person cast in it; without a role, where it
    /// happens (where an overheard line is heard; a decision's scene, else
    /// the player's home neighbourhood).
    HoodConcerns {
        #[serde(default)]
        role: Option<String>,
    },
    /// Who really runs the player's town.
    Broker,
    /// The person cast in `role`.
    Person { role: String },
    /// The rival's plans.
    RivalPlans,
    /// Who supports a policy: the bill or law cast in `role`, or a template.
    PolicySupport {
        #[serde(default)]
        role: Option<String>,
        #[serde(default)]
        template: Option<ContentKey>,
    },
    /// A situational policy on the table (transitional, until the M5d
    /// templates become policy families).
    Situational { template: ContentKey },
    /// Places of these kinds (any kind if empty) in the player's town: in
    /// the neighbourhood cast in `role` (or the home neighbourhood of the
    /// person cast in it), else where it happens. Hidden places (DEC-090 D7)
    /// use this; ordinary places are always known.
    Place {
        #[serde(default)]
        role: Option<String>,
        #[serde(default)]
        kinds: Vec<String>,
    },
}

impl SubjectDef {
    /// The role a subject names, if any.
    pub fn role(&self) -> Option<&str> {
        match self {
            SubjectDef::Person { role } => Some(role.as_str()),
            SubjectDef::HoodConcerns { role }
            | SubjectDef::PolicySupport { role, .. }
            | SubjectDef::Place { role, .. } => role.as_deref(),
            SubjectDef::TownConcerns
            | SubjectDef::Broker
            | SubjectDef::RivalPlans
            | SubjectDef::Situational { .. } => None,
        }
    }

    /// Snake-case type key.
    pub fn key(&self) -> &'static str {
        match self {
            SubjectDef::TownConcerns => "town_concerns",
            SubjectDef::HoodConcerns { .. } => "hood_concerns",
            SubjectDef::Broker => "broker",
            SubjectDef::Person { .. } => "person",
            SubjectDef::RivalPlans => "rival_plans",
            SubjectDef::PolicySupport { .. } => "policy_support",
            SubjectDef::Situational { .. } => "situational",
            SubjectDef::Place { .. } => "place",
        }
    }
}

/// A scene's teaching hook: what answering the decision it presents lets
/// the player learn (DEC-091).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TeachDef {
    pub subject: SubjectDef,
    pub level: Level,
}

fn yes() -> bool {
    true
}

// The narrative conditions are the one condition language (`condition.rs`, DEC-098 §5).

/// Which of the player's laws a `sponsored_law` condition reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LawStatus {
    /// Enacted and in force (or being built).
    #[default]
    InForce,
    /// Before the council.
    Pending,
    /// Repealed, expired, defeated or withdrawn.
    Ended,
}

/// Which voters an approval or awareness effect reaches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// The player's ward.
    #[default]
    Ward,
    /// The whole town.
    Town,
}

/// How the player votes through an option.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
    /// As the player's party expects.
    WithParty,
    /// As the player's ward leans.
    WithWard,
}

/// One real engine effect of choosing an option (DEC-073: costs and benefits
/// are simulated, not narrated).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum EffectDef {
    /// The role's opinion of the player changes (scaled by their personality,
    /// as every opinion change is), remembered as `memory` if given.
    Opinion {
        role: String,
        delta: f64,
        #[serde(default)]
        memory: Option<String>,
    },
    /// Approval of the player among voters (−1..1 scale), by income class
    /// (0 low, 1 middle, 2 high; empty = all), in the ward or the town, or in
    /// the neighbourhood cast in `neighborhood`.
    Approval {
        #[serde(default)]
        scope: Scope,
        #[serde(default)]
        classes: Vec<u8>,
        #[serde(default)]
        neighborhood: Option<String>,
        delta: f64,
    },
    /// Name recognition among voters.
    Awareness {
        #[serde(default)]
        scope: Scope,
        delta: f64,
    },
    /// Campaign funds (currency units; negative is a cost).
    Money {
        amount: f64,
    },
    Health {
        delta: f64,
    },
    /// Time this turn (an option that needs more time than is left cannot be chosen).
    Slots {
        n: u8,
    },
    Standing {
        delta: f64,
    },
    LegalJeopardy {
        delta: f64,
    },
    /// Visible work in office (0..1 scale).
    Record {
        delta: f64,
    },
    /// A favour: the player owes the role one (or, `owed_to_player`, the reverse).
    Favour {
        role: String,
        #[serde(default)]
        owed_to_player: bool,
    },
    /// The player repays a favour owed to the role (a kept promise).
    Repay {
        role: String,
    },
    /// The player declares a vote on the bill in `role` (the council-vote
    /// rules apply: party, ward, and sponsor react).
    Vote {
        role: String,
        vote: VoteChoice,
    },
    /// The bill in `role` gains a rider: a ward project for the player's
    /// ward, or a sunset (its sponsor agrees, or it is the player's bill).
    /// The council-vote rules price it (other wards' members grate at a
    /// ward project; a sunset makes passage easier and the law temporary).
    Rider {
        role: String,
        rider: RiderKind,
        /// A ward project for the ward of the councillor cast in this role
        /// (the vote it buys); the player's ward without one.
        #[serde(default)]
        ward_of: Option<String>,
    },
    /// The role repays a favour they owe the player by backing the player's
    /// side of the bill in `bill` (their lean moves as a traded favour does).
    CallIn {
        role: String,
        bill: String,
    },
    /// The player drafts a bill of this template (parameters not given take
    /// their defaults).
    Draft {
        template: ContentKey,
        #[serde(default)]
        params: BTreeMap<String, f64>,
    },
    /// A borderline or illegal act (DEC-034): a hidden fact that may come to
    /// light (its risk is shown before the player commits, DEC-073).
    Hidden {
        kind: String,
        severity: crate::media::Gravity,
        evidence: f64,
        #[serde(default)]
        discoverers: Vec<crate::media::Discoverer>,
        /// A cast person in on it (named in the fact).
        #[serde(default)]
        with_role: Option<String>,
    },
    /// The player's answer to a story about them (scandal decisions only).
    ScandalResponse {
        response: ScandalResponseKind,
    },
    /// The player's answer to an investigation (investigation decisions
    /// only); `blame` names the cast ally in `role`.
    LegalResponse {
        response: LegalResponseKind,
        #[serde(default)]
        role: Option<String>,
    },
    /// The player learns something (DEC-091): a subject raised to a level,
    /// never lowered.
    Learn {
        subject: SubjectDef,
        level: Level,
    },
    /// A hidden place is revealed (DEC-090 D7): places of these kinds in the
    /// neighbourhood cast in `role` (or the home neighbourhood of the person
    /// cast in it), else in the player's town, become `known`.
    RevealPlace {
        #[serde(default)]
        role: Option<String>,
        #[serde(default)]
        kinds: Vec<String>,
    },
    /// An inspection of the town's polluting industry (audit fix 8): its
    /// operators must clean up, so the region's pollution falls at once by
    /// `pollution` × the town's share of the region's polluting jobs (then
    /// drifts back as industry keeps working), and the town's largest
    /// polluting sector loses `output` of its demand for `months` months.
    /// Effect requests to the environment and the economy, caused by the
    /// player's answer.
    Inspection {
        pollution: f64,
        output: f64,
        months: u16,
    },
}

/// A rider a decision can attach to a bill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RiderKind {
    /// A project in the player's ward (costs the town money).
    WardProject,
    /// The law expires after four years.
    Sunset,
}

/// How the player answers a story (DEC-034 player communication).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScandalResponseKind {
    /// Say nothing (the story runs its course).
    None,
    Deny,
    Explain,
    Apologise,
    CounterAttack,
}

impl ScandalResponseKind {
    pub fn key(self) -> &'static str {
        match self {
            ScandalResponseKind::None => "none",
            ScandalResponseKind::Deny => "deny",
            ScandalResponseKind::Explain => "explain",
            ScandalResponseKind::Apologise => "apologise",
            ScandalResponseKind::CounterAttack => "counter_attack",
        }
    }
}

/// How the player answers an investigation (DEC-006 Legal Jeopardy).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LegalResponseKind {
    /// Wait and see.
    None,
    Cooperate,
    Lawyer,
    Stonewall,
    BlameAlly,
}

impl LegalResponseKind {
    pub fn key(self) -> &'static str {
        match self {
            LegalResponseKind::None => "none",
            LegalResponseKind::Cooperate => "cooperate",
            LegalResponseKind::Lawyer => "lawyer",
            LegalResponseKind::Stonewall => "stonewall",
            LegalResponseKind::BlameAlly => "blame_ally",
        }
    }
}

/// What raises a decision other than the director's spotlight: a story
/// about the player, or an investigation of them. Raised decisions are
/// fairness first (DEC-006): never budgeted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RaisedBy {
    Scandal,
    Investigation,
}

impl EffectDef {
    /// (is a cost, is a benefit) for validation (DEC-073: every option costs
    /// someone). A vote is both: party, ward, and sponsor react either way.
    pub fn valence(&self) -> (bool, bool) {
        let s = |v: f64| (v < 0.0, v > 0.0);
        match self {
            EffectDef::Opinion { delta, .. } => s(*delta),
            EffectDef::Approval { delta, .. } | EffectDef::Awareness { delta, .. } => s(*delta),
            EffectDef::Money { amount } => s(*amount),
            EffectDef::Health { delta }
            | EffectDef::Standing { delta }
            | EffectDef::Record { delta } => s(*delta),
            EffectDef::LegalJeopardy { delta } => (*delta > 0.0, *delta < 0.0),
            EffectDef::Slots { n } => (*n > 0, false),
            EffectDef::Favour { owed_to_player, .. } => (!owed_to_player, *owed_to_player),
            EffectDef::Repay { .. } => (true, true),
            EffectDef::Vote { .. } => (true, true),
            // A rider buys a vote and costs the town (or the law's life);
            // calling in a favour spends it.
            EffectDef::Rider { .. } | EffectDef::CallIn { .. } => (true, true),
            EffectDef::Draft { .. } => (false, true),
            // A risk, never a benefit: the option's payoff is elsewhere.
            EffectDef::Hidden { .. } => (true, false),
            EffectDef::ScandalResponse { .. } | EffectDef::LegalResponse { .. } => (true, true),
            // Knowledge is a benefit, never a cost.
            EffectDef::Learn { .. } | EffectDef::RevealPlace { .. } => (false, true),
            // Cleaner air, at the cost of an industry's output.
            EffectDef::Inspection { .. } => (true, true),
        }
    }

    /// The hidden-fact spec of a `Hidden` effect.
    pub fn hidden_spec(&self) -> Option<crate::media::HiddenSpec> {
        match self {
            EffectDef::Hidden {
                kind,
                severity,
                evidence,
                discoverers,
                ..
            } => Some(crate::media::HiddenSpec {
                kind: kind.clone(),
                severity: *severity,
                evidence: *evidence,
                discoverers: discoverers.clone(),
            }),
            _ => None,
        }
    }

    /// The roles an effect names.
    pub fn roles(&self) -> Vec<&str> {
        match self {
            EffectDef::Opinion { role, .. }
            | EffectDef::Favour { role, .. }
            | EffectDef::Repay { role }
            | EffectDef::Vote { role, .. } => vec![role.as_str()],
            EffectDef::Rider { role, ward_of, .. } => std::iter::once(role.as_str())
                .chain(ward_of.as_deref())
                .collect(),
            EffectDef::CallIn { role, bill } => vec![role.as_str(), bill.as_str()],
            EffectDef::Approval { neighborhood, .. } => {
                neighborhood.iter().map(|s| s.as_str()).collect()
            }
            EffectDef::Hidden { with_role, .. } => with_role.iter().map(|s| s.as_str()).collect(),
            EffectDef::LegalResponse { role, .. } => role.iter().map(|s| s.as_str()).collect(),
            EffectDef::Learn { subject, .. } => subject.role().into_iter().collect(),
            EffectDef::RevealPlace { role, .. } => role.iter().map(|s| s.as_str()).collect(),
            _ => Vec::new(),
        }
    }
}

// --- Dilemmas and requests ------------------------------------------------------------

/// The weight of a decision in the director's budgets.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DilemmaCategory {
    /// Only ever a chapter's climax.
    Climax,
    /// A significant decision (may also be chosen as a chapter's climax).
    Significant,
    /// A request raised by an NPC's goal: a lightweight dilemma (minor).
    Request,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleDef {
    /// Role id, referenced by conditions, effects, and Fluent arguments
    /// (`<id>` is the person's name, `<id>-role` their role key).
    pub id: String,
    pub cast: Cast,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OptionDef {
    pub id: String,
    pub label_key: String,
    /// What happened and why, after the choice (with the cast as arguments).
    pub aftermath_key: String,
    pub effects: Vec<EffectDef>,
    /// What happens when the deadline passes without an answer (exactly one
    /// option per dilemma; usually the costly one).
    #[serde(default)]
    pub default: bool,
}

/// A dilemma (DEC-077 §4): a moment where every option costs someone, built
/// from live state, warned in advance, and explained afterwards.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DilemmaDef {
    pub id: ContentKey,
    pub title_key: String,
    /// The situation, with the cast as arguments.
    pub situation_key: String,
    pub category: DilemmaCategory,
    /// An opportunity: its arrival is timed by the director rather than
    /// forced by a crisis (DEC-033 B).
    #[serde(default)]
    pub opportunity: bool,
    pub roles: Vec<RoleDef>,
    #[serde(default)]
    pub triggers: Vec<Condition>,
    /// Days to decide (default: the director's `deadline_days`).
    #[serde(default)]
    pub deadline_days: Option<u32>,
    /// Days before the same dilemma may be spotlighted again.
    pub cooldown_days: u32,
    pub options: Vec<OptionDef>,
    /// The scene that presents it (`scenes`).
    #[serde(default)]
    pub scene: Option<ContentKey>,
    /// Only presented as an arc's beat, never on its own (the old spelling
    /// `storyline_only` still reads).
    #[serde(default, alias = "storyline_only")]
    pub arc_only: bool,
    /// Raised by a story or an investigation about the player, not by the
    /// spotlight (DEC-006 fairness: never budgeted).
    #[serde(default)]
    pub raised_by: Option<RaisedBy>,
    /// Chapter themes it belongs to (ENGAGEMENT §4.2): a themed template is
    /// drawn before the rest in a chapter of its theme.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub themes: Vec<String>,
    /// Days since play began before the director may spotlight it
    /// (ENGAGEMENT §4.7: new pressures year by year).
    #[serde(default)]
    pub min_career_days: u32,
}

// --- NPC goals (DEC-032) ----------------------------------------------------------------

/// Who may hold a goal.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GoalHolder {
    /// Named people with one of these occupations.
    #[serde(default)]
    pub occupations: Vec<ContentKey>,
    /// Council members and the mayor (not the player).
    #[serde(default)]
    pub officeholders: bool,
}

/// How a goal drives the NPC AI (DEC-030 utility AI; the same rules as the player).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum GoalDrive {
    /// No AI change: the goal only surfaces as a request.
    None,
    /// Leans against the player's bills (added to their council lean) and
    /// lobbies against them.
    OpposePlayer { lean: f64 },
    /// Leans toward the player's bills.
    BackPlayer { lean: f64 },
    /// Ranks bills that answer the town's live crisis higher (added to the
    /// agenda bonus, like `npc.crisis_agenda`).
    Champion { bonus: f64 },
}

/// An NPC goal template (DEC-032): emerges from circumstances plus personality.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GoalDef {
    pub id: ContentKey,
    pub name_key: String,
    pub holder: GoalHolder,
    /// Personality axes of the holder within [min, max].
    #[serde(default)]
    pub personality: BTreeMap<String, [f64; 2]>,
    /// The holder has one of these traits (empty = no requirement).
    #[serde(default)]
    pub traits: Vec<ContentKey>,
    /// Conditions on the town and the holder (role `holder`).
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub drive: GoalDrive,
    /// When the goal needs the player: a request (a dilemma of category `request`).
    #[serde(default)]
    pub request: Option<ContentKey>,
    /// When the goal needs something done later: an ask (`asks.jsonc`) whose
    /// asker is cast `goal_holder` (ENGAGEMENT §3.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask: Option<ContentKey>,
    /// A goal is given up after this many days (or when its conditions fail).
    pub max_days: u32,
}

// --- Scenes (DEC-035, DEC-079) -----------------------------------------------------------

/// Where a scene happens.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScenePlace {
    /// Where the person in `role` works, else a place of the `fallback` kind.
    Workplace { role: String, fallback: String },
    /// A place of this kind in town (`streets` = the streets of the cast
    /// neighbourhood or the player's home neighbourhood).
    Kind { kind: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BeatDef {
    /// A role id, or `player`.
    pub speaker: String,
    /// Fluent key, with the cast as arguments.
    pub line: String,
    /// The speaker quotes their most salient memory of the player in this
    /// line (arguments `memory`, `subject`, `year`); this key is used when
    /// they remember nothing.
    #[serde(default)]
    pub quote_fallback: Option<String>,
}

/// A scene template: a short exchange with a character at a place.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneDef {
    pub id: ContentKey,
    pub place: ScenePlace,
    pub beats: Vec<BeatDef>,
    /// What answering the decision this scene presents teaches (DEC-091),
    /// with the dilemma's roles. The cast people are met (`known`) anyway.
    #[serde(default)]
    pub teaches: Vec<TeachDef>,
}

// --- Validation -------------------------------------------------------------------------

use crate::bundle::ContentBundle;
use crate::validate::Issue;

/// Ranges of the director's numbers (DEC-078 decision 1 style guardrails):
/// (path, value, min, max).
pub(crate) fn director_ranges(d: &DirectorParams, out: &mut Vec<(String, f64, f64, f64)>) {
    let mut r = |name: &str, v: f64, lo: f64, hi: f64| {
        out.push((format!("director.{name}"), v, lo, hi));
    };
    r("session_min_days", d.session_min_days.into(), 30.0, 730.0);
    r("budgets.climax", d.budgets.climax.into(), 0.0, 3.0);
    r(
        "budgets.significant[0]",
        d.budgets.significant[0].into(),
        0.0,
        20.0,
    );
    r(
        "budgets.significant[1]",
        d.budgets.significant[1].into(),
        d.budgets.significant[0].into(),
        20.0,
    );
    r("budgets.minor[0]", d.budgets.minor[0].into(), 0.0, 40.0);
    r(
        "budgets.minor[1]",
        d.budgets.minor[1].into(),
        d.budgets.minor[0].into(),
        40.0,
    );
    let t = &d.tension;
    for (k, v) in [
        ("start", t.start),
        ("peak", t.peak),
        ("end", t.end),
        ("slack", t.slack),
        ("catch_up_at", t.catch_up_at),
        ("crisis_weight", t.crisis_weight),
        ("pending_weight", t.pending_weight),
        ("race_weight", t.race_weight),
        ("danger_weight", t.danger_weight),
    ] {
        r(&format!("tension.{k}"), v, 0.0, 1.0);
    }
    r("tension.peak_at", t.peak_at, 0.05, 0.95);
    r("tension.climax_window[0]", t.climax_window[0], 0.0, 1.0);
    r(
        "tension.climax_window[1]",
        t.climax_window[1],
        t.climax_window[0],
        1.0,
    );
    r("spacing_days", d.spacing_days.into(), 0.0, 365.0);
    r(
        "development_spacing_days",
        d.development_spacing_days.into(),
        0.0,
        365.0,
    );
    r("max_pending", d.max_pending.into(), 1.0, 10.0);
    r("crisis_cap", d.crisis_cap.into(), 1.0, 10.0);
    r("repeat_penalty", d.repeat_penalty, 0.0, 1.0);
    r("min_relevance", d.min_relevance, 0.0, 10.0);
    let w = &d.relevance;
    for (k, v) in [
        ("base", w.base),
        ("player", w.player),
        ("ward", w.ward),
        ("crisis", w.crisis),
        ("memory", w.memory),
        ("arc", w.arc),
        ("pressure", w.pressure),
        ("election", w.election),
        ("concern", w.concern),
        ("jitter", w.jitter),
    ] {
        r(&format!("relevance.{k}"), v, 0.0, 10.0);
    }
    r("deadline_days", d.deadline_days.into(), 7.0, 365.0);
    let f = &d.fairness;
    for (k, v) in [
        ("standing", f.standing),
        ("viability", f.viability),
        ("health", f.health),
        ("legal_jeopardy", f.legal_jeopardy),
        ("threat", f.threat),
    ] {
        r(&format!("fairness.{k}"), v, 0.0, 100.0);
    }
    r("fairness.repeat_days", f.repeat_days.into(), 7.0, 730.0);
    r("fairness.response_days", f.response_days.into(), 0.0, 365.0);
    let c = &d.chance;
    r(
        "chance.hazard_hold_days",
        c.hazard_hold_days.into(),
        1.0,
        365.0,
    );
    r(
        "chance.hazard_max_defer_days",
        c.hazard_max_defer_days.into(),
        0.0,
        365.0,
    );
    r(
        "chance.recession_hold_months",
        c.recession_hold_months.into(),
        0.0,
        12.0,
    );
    r(
        "chance.discovery_days[0]",
        c.discovery_days[0].into(),
        0.0,
        3650.0,
    );
    r(
        "chance.discovery_days[1]",
        c.discovery_days[1].into(),
        c.discovery_days[0].into(),
        3650.0,
    );
    r("arcs_active", d.arcs_active.into(), 0.0, 10.0);
    r("history", d.history.into(), 1.0, 2000.0);
    for (k, c) in [
        ("session", &d.cycle.session),
        ("campaign", &d.cycle.campaign),
    ] {
        let mut span = |name: &str, x: [u32; 2]| {
            r(&format!("cycle.{k}.{name}[0]"), x[0].into(), 1.0, 365.0);
            r(
                &format!("cycle.{k}.{name}[1]"),
                x[1].into(),
                x[0].into(),
                365.0,
            );
        };
        span("build_up", c.build_up);
        span("relax", c.relax);
        if let Some(x) = c.relax_after_climax {
            span("relax_after_climax", x);
        }
        r(&format!("cycle.{k}.peak_max"), c.peak_max.into(), 1.0, 90.0);
    }
    r(
        "cycle.setback_relax_mult",
        d.cycle.setback_relax_mult,
        1.0,
        4.0,
    );
    r(
        "post_climax_significant",
        d.post_climax_significant.into(),
        0.0,
        5.0,
    );
    r("quiet_days", d.quiet_days.into(), 7.0, 365.0);
    r(
        "quiet_days_campaign",
        d.quiet_days_campaign.into(),
        7.0,
        365.0,
    );
    r(
        "tier_a_window_days",
        d.tier_a_window_days.into(),
        0.0,
        730.0,
    );
    r("pick_band", d.pick_band, 0.0, 10.0);
    for (k, w) in &d.themes {
        r(&format!("themes.{k}"), *w, 0.0, 10.0);
    }
    r(
        "cooldown_floor_days",
        d.cooldown_floor_days.into(),
        0.0,
        3650.0,
    );
    r("cooldown_growth", d.cooldown_growth, 1.0, 4.0);
}

fn in_range(v: f64, lo: f64, hi: f64) -> bool {
    v.is_finite() && (lo..=hi).contains(&v)
}

/// What a condition, cast, or effect may refer to, for one template.
struct Refs<'a> {
    owner: &'a str,
    b: &'a ContentBundle,
    roles: Vec<&'a str>,
}

impl Refs<'_> {
    fn role(&self, r: &str, issues: &mut Vec<Issue>) {
        if !self.roles.contains(&r) {
            issues.push(Issue::error(
                self.owner,
                &format!(
                    "names unknown role `{r}` (roles: {})",
                    self.roles.join(", ")
                ),
            ));
        }
    }

    /// A knowledge subject (DEC-091): its role is cast here (checked by the
    /// caller through `roles()`), and what it names exists.
    fn subject(&self, s: &SubjectDef, issues: &mut Vec<Issue>) {
        check_subject(self.owner, self.b, s, issues);
    }

    /// Conditions where they are used, checked by the one condition
    /// validator (`condition.rs`) with this item's roles as its slots: the
    /// table refuses what the use cannot read (`knows` where a condition
    /// decides whether something happens, DEC-091; the event log, people's
    /// routines and conversations inside the simulation), and each
    /// condition names real things with sane ranges.
    fn conditions(
        &self,
        at: crate::condition::ConditionUse,
        cs: &[Condition],
        issues: &mut Vec<Issue>,
    ) {
        let site = crate::condition::Site::new(self.owner, at, &self.roles);
        crate::condition::check_all(self.b, &site, cs, issues);
    }
}

/// What a knowledge subject names exists: a policy template, a
/// situational template, place kinds; a policy-support subject names a role
/// or a template (exactly one).
pub(crate) fn check_subject(
    owner: &str,
    b: &ContentBundle,
    s: &SubjectDef,
    issues: &mut Vec<Issue>,
) {
    match s {
        SubjectDef::PolicySupport { role, template } => {
            if role.is_some() == template.is_some() {
                issues.push(Issue::error(
                    owner,
                    "a policy_support subject names a role or a template (exactly one)",
                ));
            }
            if let Some(t) = template
                && !b.policies.contains_key(t)
            {
                issues.push(Issue::error(owner, &format!("unknown policy `{t}`")));
            }
        }
        SubjectDef::Situational { template } => match b.policies.get(template) {
            None => issues.push(Issue::error(owner, &format!("unknown policy `{template}`"))),
            Some(t) if t.unlock.is_none() => issues.push(Issue::error(
                owner,
                &format!("`{template}` is a standard policy, not a situational one"),
            )),
            Some(_) => {}
        },
        SubjectDef::Place { kinds, .. } => {
            for k in kinds {
                if b.place_kind(k).is_none() {
                    issues.push(Issue::error(owner, &format!("unknown place kind `{k}`")));
                }
            }
        }
        SubjectDef::TownConcerns
        | SubjectDef::HoodConcerns { .. }
        | SubjectDef::Broker
        | SubjectDef::Person { .. }
        | SubjectDef::RivalPlans => {}
    }
}

/// Cast definitions: earlier roles only, known references.
pub(crate) fn check_roles<'a>(
    owner: &'a str,
    b: &'a ContentBundle,
    roles: &'a [RoleDef],
    request: bool,
    issues: &mut Vec<Issue>,
) -> Vec<&'a str> {
    let mut seen: Vec<&str> = Vec::new();
    for r in roles {
        if r.id.is_empty()
            || r.id == "player"
            || !r.id.chars().all(|c| c.is_ascii_lowercase() || c == '_')
        {
            issues.push(Issue::error(
                owner,
                &format!(
                    "role id `{}` must be lower-case letters and `_` (not `player`)",
                    r.id
                ),
            ));
        }
        if seen.contains(&r.id.as_str()) {
            issues.push(Issue::error(
                owner,
                &format!("role `{}` defined twice", r.id),
            ));
        }
        match &r.cast {
            Cast::Occupation { occupations } => {
                if occupations.is_empty() {
                    issues.push(Issue::error(owner, "an occupation cast needs occupations"));
                }
                for o in occupations {
                    if b.occupation(o).is_none() {
                        issues.push(Issue::error(owner, &format!("unknown occupation `{o}`")));
                    }
                }
            }
            Cast::Bill {
                sponsored_by: Some(s),
                ..
            }
            | Cast::SponsorOf { role: s } => {
                if !seen.contains(&s.as_str()) {
                    issues.push(Issue::error(
                        owner,
                        &format!(
                            "role `{}` refers to `{s}`, which must be cast before it",
                            r.id
                        ),
                    ));
                }
            }
            Cast::Law { templates, .. } => {
                for t in templates {
                    if !b.policies.contains_key(t) {
                        issues.push(Issue::error(owner, &format!("unknown policy `{t}`")));
                    }
                }
            }
            Cast::Office { office: Some(o) } => {
                if !b
                    .constitutions
                    .values()
                    .any(|c| c.tiers.iter().any(|t| t.office(o).is_some()))
                {
                    issues.push(Issue::error(owner, &format!("unknown office `{o}`")));
                }
            }
            Cast::GoalHolder if !request => {
                issues.push(Issue::error(
                    owner,
                    "only requests (and goals' conditions) cast the goal holder",
                ));
            }
            _ => {}
        }
        seen.push(&r.id);
    }
    seen
}

/// A template's themes are ones the director reads (and weighs), listed
/// once; `min_career_days` stays within a long career.
fn check_themes(
    owner: &str,
    b: &ContentBundle,
    themes: &[String],
    min_career_days: u32,
    issues: &mut Vec<Issue>,
) {
    for (i, t) in themes.iter().enumerate() {
        if themes[..i].contains(t) {
            issues.push(Issue::error(owner, &format!("theme `{t}` listed twice")));
        }
        if t != THEME_CAMPAIGN && !b.director.themes.contains_key(t) {
            issues.push(Issue::error(
                owner,
                &format!(
                    "unknown theme `{t}` (themes: {}, {THEME_CAMPAIGN})",
                    b.director
                        .themes
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
    if min_career_days > 3650 {
        issues.push(Issue::error(owner, "min_career_days must be at most 3650"));
    }
}

fn check_dilemma(b: &ContentBundle, d: &DilemmaDef, issues: &mut Vec<Issue>) {
    let owner = d.id.as_str();
    let request = d.category == DilemmaCategory::Request;
    let roles = check_roles(owner, b, &d.roles, request, issues);
    let scope = Refs { owner, b, roles };
    scope.conditions(
        crate::condition::ConditionUse::DilemmaTrigger,
        &d.triggers,
        issues,
    );
    // A raised decision answers a story or a case: four answers and silence.
    let most = if d.raised_by.is_some() { 5 } else { 4 };
    if !(2..=most).contains(&d.options.len()) {
        issues.push(Issue::error(
            owner,
            &format!("a dilemma has 2–{most} options"),
        ));
    }
    let defaults = d.options.iter().filter(|o| o.default).count();
    if defaults != 1 {
        issues.push(Issue::error(
            owner,
            "exactly one option is the default (what happens if the deadline passes)",
        ));
    }
    if d.deadline_days.is_some_and(|x| !(7..=365).contains(&x)) {
        issues.push(Issue::error(owner, "deadline_days must be 7–365"));
    }
    if d.cooldown_days > 3650 {
        issues.push(Issue::error(owner, "cooldown_days must be at most 3650"));
    }
    check_themes(owner, b, &d.themes, d.min_career_days, issues);
    let mut ids: Vec<&str> = Vec::new();
    for o in &d.options {
        let at = format!("{owner} option `{}`", o.id);
        if ids.contains(&o.id.as_str()) {
            issues.push(Issue::error(&at, "option ids must be unique"));
        }
        ids.push(&o.id);
        if o.effects.is_empty() {
            issues.push(Issue::error(&at, "an option needs effects"));
        }
        // DEC-073 / DEC-077 §4: every option costs someone, and buys something.
        let (cost, gain) = o.effects.iter().fold((false, false), |(c, g), e| {
            let (ec, eg) = e.valence();
            (c || ec, g || eg)
        });
        if !cost {
            issues.push(Issue::error(
                &at,
                "every option needs at least one cost (DEC-073)",
            ));
        }
        if !gain {
            issues.push(Issue::error(
                &at,
                "every option needs at least one benefit (DEC-073)",
            ));
        }
        for e in &o.effects {
            for r in e.roles() {
                scope.role(r, issues);
            }
            match e {
                EffectDef::Opinion { delta, memory, .. } => {
                    if !in_range(*delta, -100.0, 100.0) {
                        issues.push(Issue::error(&at, "opinion delta must be in [-100, 100]"));
                    }
                    if let Some(m) = memory
                        && !crate::types::MEMORY_KINDS.contains(&m.as_str())
                    {
                        issues.push(Issue::error(&at, &format!("unknown memory kind `{m}`")));
                    }
                }
                EffectDef::Approval { classes, delta, .. } => {
                    if classes.iter().any(|c| *c > 2) {
                        issues.push(Issue::error(&at, "approval classes are 0, 1, 2"));
                    }
                    if !in_range(*delta, -0.2, 0.2) {
                        issues.push(Issue::error(&at, "approval delta must be in [-0.2, 0.2]"));
                    }
                }
                EffectDef::Awareness { delta, .. } => {
                    if !in_range(*delta, -0.5, 0.5) {
                        issues.push(Issue::error(&at, "awareness delta must be in [-0.5, 0.5]"));
                    }
                }
                EffectDef::Money { amount } => {
                    if !in_range(*amount, -1e6, 1e6) {
                        issues.push(Issue::error(&at, "money must be finite and modest"));
                    }
                }
                EffectDef::Health { delta }
                | EffectDef::Standing { delta }
                | EffectDef::LegalJeopardy { delta } => {
                    if !in_range(*delta, -50.0, 50.0) {
                        issues.push(Issue::error(&at, "track changes must be in [-50, 50]"));
                    }
                }
                EffectDef::Record { delta } => {
                    if !in_range(*delta, -0.5, 0.5) {
                        issues.push(Issue::error(&at, "record changes must be in [-0.5, 0.5]"));
                    }
                }
                EffectDef::Inspection {
                    pollution,
                    output,
                    months,
                } => {
                    if !in_range(*pollution, 0.0, 0.5)
                        || !in_range(*output, 0.0, 0.5)
                        || *months == 0
                        || *months > 36
                    {
                        issues.push(Issue::error(
                            &at,
                            "inspection: pollution and output in [0, 0.5], 1–36 months",
                        ));
                    }
                }
                EffectDef::Slots { n } => {
                    if o.default {
                        issues.push(Issue::error(
                            &at,
                            "the default option cannot cost time (it happens when the player is away)",
                        ));
                    }
                    if *n == 0 || *n > 3 {
                        issues.push(Issue::error(&at, "slots must be 1–3"));
                    }
                }
                EffectDef::Draft { template, params } => match b.policies.get(template) {
                    None => issues.push(Issue::error(&at, &format!("unknown policy `{template}`"))),
                    Some(t) => {
                        for (k, v) in params {
                            let ok = t
                                .params
                                .iter()
                                .any(|p| &p.id == k && in_range(*v, p.min, p.max));
                            if !ok {
                                issues.push(Issue::error(
                                    &at,
                                    &format!("draft parameter `{k}` is unknown or out of range"),
                                ));
                            }
                        }
                    }
                },
                EffectDef::Vote { role, .. } => {
                    let is_bill = d
                        .roles
                        .iter()
                        .any(|r| &r.id == role && matches!(r.cast, Cast::Bill { .. }));
                    if !is_bill {
                        issues.push(Issue::error(&at, "a vote needs a role cast as a bill"));
                    }
                }
                EffectDef::Rider { role, .. } | EffectDef::CallIn { bill: role, .. } => {
                    let is_bill = d
                        .roles
                        .iter()
                        .any(|r| &r.id == role && matches!(r.cast, Cast::Bill { .. }));
                    if !is_bill {
                        issues.push(Issue::error(
                            &at,
                            "a rider or a favour called in needs a role cast as a bill",
                        ));
                    }
                }
                EffectDef::Favour { .. } | EffectDef::Repay { .. } => {}
                EffectDef::Hidden { kind, evidence, .. } => {
                    if !in_range(*evidence, 0.0, 1.0) {
                        issues.push(Issue::error(&at, "hidden evidence must be in [0, 1]"));
                    }
                    if kind.is_empty() || !kind.chars().all(|c| c.is_ascii_lowercase() || c == '_')
                    {
                        issues.push(Issue::error(
                            &at,
                            "a hidden fact kind is lower-case letters and `_`",
                        ));
                    }
                }
                EffectDef::ScandalResponse { .. } => {
                    if d.raised_by != Some(RaisedBy::Scandal) {
                        issues.push(Issue::error(
                            &at,
                            "a scandal response belongs to a decision raised by a scandal",
                        ));
                    }
                }
                EffectDef::Learn { subject, level } => {
                    scope.subject(subject, issues);
                    if *level == Level::Unknown {
                        issues.push(Issue::error(
                            &at,
                            "learn `level` must be heard, known or detailed",
                        ));
                    }
                }
                EffectDef::RevealPlace { role, kinds } => {
                    if role.is_none() && kinds.is_empty() {
                        issues.push(Issue::error(
                            &at,
                            "reveal_place names a role or place kinds (or both)",
                        ));
                    }
                    for k in kinds {
                        if b.place_kind(k).is_none() {
                            issues.push(Issue::error(&at, &format!("unknown place kind `{k}`")));
                        }
                    }
                }
                EffectDef::LegalResponse { response, role } => {
                    if d.raised_by != Some(RaisedBy::Investigation) {
                        issues.push(Issue::error(
                            &at,
                            "a legal response belongs to a decision raised by an investigation",
                        ));
                    }
                    let blame = *response == LegalResponseKind::BlameAlly;
                    if blame != role.is_some() {
                        issues.push(Issue::error(
                            &at,
                            "blaming an ally names the ally's role (and only blaming does)",
                        ));
                    }
                }
            }
        }
        if let Some(by) = d.raised_by {
            let answers = o.effects.iter().any(|e| match by {
                RaisedBy::Scandal => matches!(e, EffectDef::ScandalResponse { .. }),
                RaisedBy::Investigation => matches!(e, EffectDef::LegalResponse { .. }),
            });
            if !answers {
                issues.push(Issue::error(
                    &at,
                    "every option of a raised decision is a response to it",
                ));
            }
        }
    }
    if d.raised_by.is_some() && (d.category != DilemmaCategory::Significant || d.arc_only) {
        issues.push(Issue::error(
            owner,
            "a raised decision is significant and stands on its own",
        ));
    }
    if let Some(s) = &d.scene {
        match b.scene(s) {
            None => issues.push(Issue::error(owner, &format!("unknown scene `{s}`"))),
            Some(sc) => {
                let speakers = sc.beats.iter().map(|x| x.speaker.as_str());
                let mut place_roles: Vec<&str> = Vec::new();
                if let ScenePlace::Workplace { role, .. } = &sc.place {
                    place_roles.push(role);
                }
                for t in &sc.teaches {
                    place_roles.extend(t.subject.role());
                }
                for r in speakers.chain(place_roles) {
                    if r != "player" && !scope.roles.contains(&r) {
                        issues.push(Issue::error(
                            owner,
                            &format!(
                                "scene `{s}` needs role `{r}`, which this dilemma does not cast"
                            ),
                        ));
                    }
                }
            }
        }
    }
}

/// Narrative content (DEC-033, DEC-032, DEC-077 §4): references resolve, every
/// option costs someone and buys something, scenes are
/// consistent, and every Fluent key exists in English.
pub(crate) fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let d = &b.director;
    if b.storyteller(&d.default_profile).is_none() {
        issues.push(Issue::error(
            "director",
            &format!(
                "default_profile names unknown storyteller `{}`",
                d.default_profile
            ),
        ));
    }
    for k in d.themes.keys() {
        if !THEMES.contains(&k.as_str()) || k == THEME_CAMPAIGN {
            issues.push(Issue::error(
                "director",
                &format!(
                    "unknown theme `{k}`: the director reads crisis, budget, scheme and local_life \
                     (campaign chapters always take campaign)"
                ),
            ));
        }
    }
    if !d.themes.contains_key("local_life") {
        issues.push(Issue::error(
            "director",
            "themes need `local_life` (always eligible: every session chapter has a theme)",
        ));
    }
    for s in &b.storytellers {
        if !in_range(s.build_up_scale, 0.1, 4.0) || !in_range(s.relax_scale, 0.1, 4.0) {
            issues.push(Issue::error(
                &s.id,
                "build_up_scale and relax_scale must be in [0.1, 4]",
            ));
        }
        if !crate::validate::valid_key(&s.id) {
            issues.push(Issue::error(
                &s.id,
                "storyteller ids look like `storyteller.name`",
            ));
        }
        if !in_range(s.hazard_rate, 0.0, 1.0) || !in_range(s.budget_scale, 0.0, 4.0) {
            issues.push(Issue::error(
                &s.id,
                "hazard_rate must be in [0, 1] and budget_scale in [0, 4]",
            ));
        }
        if s.crisis_cap.is_some_and(|c| c == 0 || c > 10) {
            issues.push(Issue::error(&s.id, "crisis_cap must be 1–10"));
        }
    }
    for x in &b.dilemmas {
        if !crate::validate::valid_key(&x.id) {
            issues.push(Issue::error(
                &x.id,
                "dilemma ids look like `dilemma.name` (requests `request.name`)",
            ));
        }
        check_dilemma(b, x, issues);
    }
    for g in &b.goals {
        let owner = g.id.as_str();
        if !crate::validate::valid_key(owner) {
            issues.push(Issue::error(owner, "goal ids look like `goal.name`"));
        }
        if g.holder.occupations.is_empty() && !g.holder.officeholders {
            issues.push(Issue::error(
                owner,
                "a goal needs holders (occupations or officeholders)",
            ));
        }
        for o in &g.holder.occupations {
            if b.occupation(o).is_none() {
                issues.push(Issue::error(owner, &format!("unknown occupation `{o}`")));
            }
        }
        for (k, [lo, hi]) in &g.personality {
            if !crate::types::PERSONALITY_AXES.contains(&k.as_str())
                || !in_range(*lo, 0.0, 1.0)
                || !in_range(*hi, *lo, 1.0)
            {
                issues.push(Issue::error(
                    owner,
                    &format!("personality `{k}` must be a known axis with 0 ≤ min ≤ max ≤ 1"),
                ));
            }
        }
        for t in &g.traits {
            if b.trait_def(t).is_none() {
                issues.push(Issue::error(owner, &format!("unknown trait `{t}`")));
            }
        }
        let scope = Refs {
            owner,
            b,
            roles: vec!["holder"],
        };
        scope.conditions(
            crate::condition::ConditionUse::GoalCondition,
            &g.conditions,
            issues,
        );
        let ok = match &g.drive {
            GoalDrive::None => true,
            GoalDrive::OpposePlayer { lean } | GoalDrive::BackPlayer { lean } => {
                in_range(*lean, 0.0, 5.0)
            }
            GoalDrive::Champion { bonus } => in_range(*bonus, 0.0, 5.0),
        };
        if !ok {
            issues.push(Issue::error(owner, "drive strength must be in [0, 5]"));
        }
        if matches!(
            g.drive,
            GoalDrive::OpposePlayer { .. }
                | GoalDrive::BackPlayer { .. }
                | GoalDrive::Champion { .. }
        ) && !g.holder.officeholders
        {
            issues.push(Issue::error(
                owner,
                "council drives need officeholders (Named people take no council turns)",
            ));
        }
        if let Some(r) = &g.request {
            match b.dilemma(r) {
                Some(x) if x.category == DilemmaCategory::Request => {}
                Some(_) => issues.push(Issue::error(
                    owner,
                    &format!("request `{r}` must be a dilemma of category `request`"),
                )),
                None => issues.push(Issue::error(owner, &format!("unknown request `{r}`"))),
            }
        }
        if let Some(a) = &g.ask {
            match b.asks.iter().find(|x| &x.id == a) {
                Some(x) if matches!(x.asker, Cast::GoalHolder) => {}
                Some(_) => issues.push(Issue::error(
                    owner,
                    &format!("ask `{a}` must cast its asker `goal_holder`"),
                )),
                None => issues.push(Issue::error(owner, &format!("unknown ask `{a}`"))),
            }
            if g.request.is_some() {
                issues.push(Issue::error(
                    owner,
                    "a goal needs the player through a request or an ask, not both",
                ));
            }
        }
        if g.max_days == 0 || g.max_days > 3650 {
            issues.push(Issue::error(owner, "max_days must be 1–3650"));
        }
    }
    // A request is raised by a goal: every request needs one.
    for x in b
        .dilemmas
        .iter()
        .filter(|x| x.category == DilemmaCategory::Request)
    {
        let raised = b
            .goals
            .iter()
            .any(|g| g.request.as_deref() == Some(x.id.as_str()));
        let staged = b.arcs.iter().any(|a| {
            a.beats().any(|(_, beat)| {
                matches!(&beat.kind, crate::arcs::BeatKind::Decision { dilemma, .. } if dilemma == &x.id)
            })
        });
        if !raised && !staged {
            issues.push(Issue::error(
                &x.id,
                "a request must be raised by a goal (or an arc's beat)",
            ));
        }
    }
    for sc in &b.scenes {
        if !crate::validate::valid_key(&sc.id) {
            issues.push(Issue::error(&sc.id, "scene ids look like `scene.name`"));
        }
        if sc.beats.is_empty() {
            issues.push(Issue::error(&sc.id, "a scene needs at least one beat"));
        }
        let kind = match &sc.place {
            ScenePlace::Workplace { fallback, .. } => fallback,
            ScenePlace::Kind { kind } => kind,
        };
        if b.place_kind(kind).is_none() || kind == "home" {
            issues.push(Issue::error(
                &sc.id,
                &format!("unknown place kind `{kind}`"),
            ));
        }
        for t in &sc.teaches {
            check_subject(&sc.id, b, &t.subject, issues);
            if t.level == Level::Unknown {
                issues.push(Issue::error(
                    &sc.id,
                    "a scene teaches at heard, known or detailed",
                ));
            }
        }
    }
    // Fluent keys (DEC-054): the engine emits keys; English must have them.
    let en = crate::validate::fluent_keys(&b.locale_text("en"));
    let mut keys: Vec<(&str, &str)> = Vec::new();
    for s in &b.storytellers {
        keys.push((&s.id, &s.name_key));
        keys.push((&s.id, &s.desc_key));
    }
    for x in &b.dilemmas {
        keys.push((&x.id, &x.title_key));
        keys.push((&x.id, &x.situation_key));
        for o in &x.options {
            keys.push((&x.id, &o.label_key));
            keys.push((&x.id, &o.aftermath_key));
        }
    }
    for g in &b.goals {
        keys.push((&g.id, &g.name_key));
    }
    for sc in &b.scenes {
        for beat in &sc.beats {
            keys.push((&sc.id, &beat.line));
            if let Some(f) = &beat.quote_fallback {
                keys.push((&sc.id, f));
            }
        }
    }
    for (owner, key) in keys {
        if !en.contains(key) {
            issues.push(Issue::error(
                owner,
                &format!("missing Fluent key `{key}` in locale en"),
            ));
        }
    }
}
