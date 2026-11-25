//! One condition language (DEC-098 §5, task U1; M6_ENGINE_CONTRACTS §2.4,
//! §3.3 and §4). Dilemma triggers, goal conditions, arc raises and
//! advances, policy unlocks, conversation topics, lines and voices,
//! overheard lines and visit verbs all write the same [`Condition`]: one
//! type here, one evaluator (`mandate_sim::condition`), and one validator
//! ([`check`]), whose table ([`ConditionUse::provides`] against
//! [`Condition::needs`]) says where each variant is allowed.
//!
//! **Spellings.** The narrative language named cast actors `role`; the
//! situation language named slots `slot`. They are one field, `slot`, and
//! `role` still parses (a serde alias, and the generated schema accepts
//! it). `live_crisis` is the old spelling of a `crisis` without a `kind`.
//! A `concern` with one `concern` (the dilemma spelling) and a `concern`
//! with `concerns` (the conversation spelling) are one worry condition
//! whose defaults follow the spelling ([`Condition::worry`]).
//!
//! **Tiers (DEC-098).** Each variant is tier-generic or local-only
//! ([`Condition::tier`]): a local-only one reads a town's state and never
//! holds for a region or the nation; a non-local item may use only
//! tier-generic conditions.
//!
//! **Arguments.** Each variant documents the Fluent arguments it provides
//! when it holds ([`Condition::provides`]); triggers ignore them, lines use
//! them.
//!
//! Adding a condition: `docs/development/EXTENDING.md`, "Adding a condition".

use crate::bundle::ContentBundle;
use crate::conversation::{AboutKind, ConcernOf, SubjectRef};
use crate::knowledge::Level;
use crate::narrative::{LawStatus, SubjectDef, TrackKey};
use crate::policy_family::SlotKind;
use crate::routines::DayPart;
use crate::types::{ContentKey, HazardKey, Indicator, Power, SectorDriver, TierKey};
use crate::validate::Issue;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A condition on live state: the jurisdiction's situation, the player, a
/// person in a slot (a cast role, or a conversation's `speaker`, `about`,
/// `player`), a conversation, or where something is said or done.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(transform = old_spellings)]
pub enum Condition {
    // --- The town's situation (local-only) ------------------------------------------
    /// A live crisis in the town (`crises.jsonc`; of this kind, if given) at
    /// least `min_severity` severe: `$crisis_key` (the most severe such
    /// crisis). `live_crisis` is the old spelling without a kind.
    #[serde(alias = "live_crisis")]
    Crisis {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<ContentKey>,
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
    /// One worry condition, felt or measured, of the town or a
    /// neighbourhood: one of the listed concerns (`concern`, or any of
    /// `concerns`) is among the `top` strongest worries `of` the town
    /// (default), the speaker's home neighbourhood, or the neighbourhood
    /// here, `by` how residents feel them (`felt`, as the notebook ranks
    /// them) or by their measured levels (`measured`; the town's are its
    /// priorities, its three strongest, kept with its character). Defaults
    /// follow the spelling: with one `concern`, measured, top 3 (the dilemma
    /// spelling); with `concerns`, felt, top 1 (the conversation spelling).
    /// Gives `$concern_key` (the name) and `$concern_talk_key` (the noun
    /// phrase of talk) of the strongest listed worry that holds.
    Concern {
        #[serde(default, skip_serializing_if = "is_town")]
        of: ConcernOf,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        concern: Option<ContentKey>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        concerns: Vec<ContentKey>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        top: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        by: Option<Worry>,
    },
    /// The town has a defining industry (with one of these drivers, if
    /// given): `$industry_key` (the sector as residents name it).
    Industry {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        drivers: Option<Vec<SectorDriver>>,
    },
    /// A plant closed in the town within this many months (the event log):
    /// `$industry_key` (the closed plant's sector), `$jobs`.
    PlantClosure { within_months: u16 },
    /// A disaster struck the town within this many months; with `hazard`,
    /// one of that kind whose damage is not yet repaired (the environment's
    /// footprint: a strike that damaged no home does not count).
    Disaster {
        within_months: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hazard: Option<HazardKey>,
    },
    /// A law of one of these templates is in force or being built in the
    /// town (`absent`: none is).
    LawInForce {
        templates: Vec<ContentKey>,
        #[serde(default)]
        absent: bool,
    },
    /// The town has this water (`any`: also a waterfront neighbourhood here).
    Water { feature: WaterFeature },
    /// No water at all: no river, no coast, no waterfront here.
    Dry,
    /// The town has a power broker: `$broker`.
    Broker,
    /// The player has a named rival in the town: `$rival`.
    Rival,
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
    /// A story about the town within the last eight weeks (an overheard line
    /// retells it: `$outlet_key`, `$headline_key` and the headline's own
    /// arguments, added by the protocol).
    Headline,
    /// The player's name recognition among the residents here within
    /// [min, max] (0..1): `$player`.
    PlayerAwareness {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// Approval of the player among the residents here (−1..1) within
    /// [min, max]; only where at least a fifth know them: `$player`.
    PlayerApproval {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// The industry of the place here (its kind's `sector`) is shedding jobs
    /// in its town, at least `min` of the way to full shedding
    /// (`visit.outlook`: 0 just below `shedding`, 1 at `shed_full`):
    /// `$industry_key`.
    Shedding {
        #[serde(default)]
        min: f64,
    },
    /// Someone of one of these occupations is here now (people's routines;
    /// not the player).
    Present { occupations: Vec<ContentKey> },
    // --- Time and the calendar (tier-generic) ---------------------------------------
    /// It is one of these parts of the player's day.
    DayPart { parts: Vec<DayPart> },
    /// The jurisdiction is in this phase: a campaign for one of its posts is
    /// running (`campaign`), or none is (`governing`).
    Phase { phase: Phase },
    /// The player's next election is within this many days.
    ElectionWithin { days: u32 },
    /// At least this many months since play began.
    MonthsInPlay { min: u32 },
    // --- The player (tier-generic) --------------------------------------------------
    /// The player is (or is not) standing in a running campaign.
    Campaigning {
        #[serde(default = "yes")]
        is: bool,
    },
    /// The player holds one of these posts (DEC-098): role words (a body's
    /// `member_key` or an office's id, from the run's constitution), `any`
    /// (any post) or `none` (out of office). `role` is the one-word
    /// spelling (`{ "type": "office", "role": "councillor" }`).
    Office {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        posts: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        role: Option<String>,
    },
    /// The player holds a post for this jurisdiction whose constitution
    /// lists this power (read through the constitution, never an office's
    /// name: `requires_power`).
    HoldsPower { power: Power },
    /// A track within [min, max].
    Track {
        track: TrackKey,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// The player's campaign funds within [min, max] (currency units).
    Money {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// The player answered `template` (with `option`, if given; a default
    /// that happened counts) between `min_days` and `max_days` ago
    /// (ENGAGEMENT §4.4: consequences of the player's own choices). Roles in
    /// `same_cast` are cast as they were in that decision.
    Answered {
        template: ContentKey,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        option: Option<String>,
        #[serde(default)]
        min_days: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_days: Option<u32>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        same_cast: Vec<String>,
    },
    /// The player sponsored a law of this status, between `min_age_days`
    /// and `max_age_days` old (since enacted; a pending bill since it was
    /// introduced): the law cast in `slot`, or any. `chance` is the share of
    /// such laws that count, a keyed draw per law and condition, so the same
    /// law always gives the same answer (ENGAGEMENT §4.4, §4.7).
    SponsoredLaw {
        #[serde(default)]
        status: LawStatus,
        #[serde(default)]
        min_age_days: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_age_days: Option<u32>,
        #[serde(default = "one")]
        chance: f64,
        #[serde(default, alias = "role", skip_serializing_if = "Option::is_none")]
        slot: Option<String>,
    },
    /// The player knows the subject at least this well (DEC-091; default
    /// `heard`), its roles from the cast. A subject naming several places
    /// holds if any is known. Framing only (refused where a condition
    /// decides whether something happens: the world does not wait for the
    /// player).
    Knows {
        subject: SubjectDef,
        #[serde(default = "heard")]
        min: Level,
    },
    /// The player's ledger (DEC-091) holds the subject, named from the
    /// conversation, within [min, max]: the subject's arguments. Framing
    /// only.
    PlayerKnows {
        subject: SubjectRef,
        #[serde(default)]
        min: Option<Level>,
        #[serde(default)]
        max: Option<Level>,
    },
    // --- A person in a slot (tier-generic unless noted) -----------------------------
    /// The person in the slot has one of these roles: occupation ids, or
    /// role words of the constitution's posts (a body's `member_key`, an
    /// office's id; DEC-098).
    Occupation {
        #[serde(alias = "role")]
        slot: String,
        occupations: Vec<ContentKey>,
    },
    /// The person in the slot has one of these traits.
    Trait {
        #[serde(alias = "role")]
        slot: String,
        traits: Vec<ContentKey>,
    },
    /// A personality axis of the person in the slot within [min, max] (0..1).
    Personality {
        #[serde(alias = "role")]
        slot: String,
        axis: String,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// The person in the slot holds this goal (DEC-032).
    Goal {
        #[serde(alias = "role")]
        slot: String,
        goal: ContentKey,
    },
    /// Their opinion of the player within [min, max] (−100..100).
    Opinion {
        #[serde(alias = "role")]
        slot: String,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// Their trust in the player within [min, max] (0..1).
    Trust {
        #[serde(alias = "role")]
        slot: String,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// They remember the player doing one of these (memory kinds; the most
    /// salient): `$memory_key`, `$subject_key`, `$subject_kind`, `$year`.
    Remembers {
        #[serde(alias = "role")]
        slot: String,
        kinds: Vec<String>,
    },
    /// The player owes them a favour (or, `owed_to_player`, the reverse): `$n`.
    /// With `is: false`, none is owed (the debt was paid, or never was).
    Favour {
        #[serde(alias = "role")]
        slot: String,
        #[serde(default)]
        owed_to_player: bool,
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        is: bool,
    },
    /// The person in the slot sponsors a bill the player can still vote on,
    /// and a yes would cost the player: their ward leans against it, their
    /// party's line is against it, or a promise they hold forbids it (the
    /// first such bill, the first bind that holds). `$law_key` (the bill's
    /// template), `$bind` (`seat`, `party` or `promise`).
    BillCosts {
        #[serde(alias = "role")]
        slot: String,
    },
    /// They have (or have not) met the player before now: talked with them
    /// (a line of theirs in the conversation guard), or met them on an
    /// earlier day (a `met` memory). Meeting a passer-by today is not
    /// having met them before.
    Met {
        #[serde(alias = "role")]
        slot: String,
        #[serde(default = "yes")]
        is: bool,
    },
    /// They are (or are not) the town's power broker (local-only). When
    /// they are: `$broker`, `$broker_place`, `$stake_key`.
    IsBroker {
        #[serde(alias = "role")]
        slot: String,
        #[serde(default = "yes")]
        is: bool,
    },
    /// They are (or are not) the player's rival in the town (local-only).
    /// When they are: `$rival`.
    IsRival {
        #[serde(alias = "role")]
        slot: String,
        #[serde(default = "yes")]
        is: bool,
    },
    /// The player's party line on the bill cast in `slot` runs against the
    /// mood of the player's ward.
    PartyLineAgainstWard {
        #[serde(alias = "role")]
        slot: String,
    },
    /// The player can still declare a vote on the bill cast in `slot`.
    CanVote {
        #[serde(alias = "role")]
        slot: String,
    },
    // --- A conversation (tier-generic) ----------------------------------------------
    /// The speaker would tell the player at least `min` about the subject
    /// (what they know, capped by discretion): the subject's arguments.
    Tells { subject: SubjectRef, min: Level },
    /// The speaker's voice (`talk_voices`) is one of these, so a line (an
    /// opener) follows the voice the closers use. Not allowed in a voice's
    /// own conditions.
    Voice { voices: Vec<ContentKey> },
    /// What the conversation is about (the `about` subject): one of these
    /// kinds (empty: any subject); for a place, one of `places` (place kinds;
    /// empty: any) and, if `workplace` is given, the speaker's own
    /// workplace (`true`) or not (`false`): where they work, or where their
    /// post sits; for a neighbourhood, if `home` is given, the speaker's home
    /// (`true`) or not, if `office` is given, inside the jurisdiction or
    /// district of a post the speaker holds (`true`) or not, if `works` is
    /// given, where the speaker works (`true`) or not, if `here` is given,
    /// where the conversation happens (`true`) or not, and if `tied` is
    /// given, any of those four (`true`) or none of them (`false`: somewhere
    /// the speaker has no reason to know). With `is: false` it holds when
    /// the subject is not that (or there is none).
    About {
        #[serde(default)]
        kinds: Vec<AboutKind>,
        #[serde(default)]
        places: Vec<String>,
        #[serde(default)]
        workplace: Option<bool>,
        #[serde(default)]
        home: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        office: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        works: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        here: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tied: Option<bool>,
        #[serde(default = "yes")]
        is: bool,
    },
    // --- A policy family's bound slots (local-only; F1, DEC-092) -------------------
    /// This concern is among the `top` worries (measured) of the
    /// neighbourhood bound to `slot`.
    HoodConcern {
        #[serde(alias = "role")]
        slot: String,
        concern: ContentKey,
        #[serde(default = "three")]
        top: u8,
    },
    /// The neighbourhood bound to `slot` is exposed to this hazard at least
    /// `min` (0..1; DEC-090 D8).
    HoodHazard {
        #[serde(alias = "role")]
        slot: String,
        hazard: HazardKey,
        min: f64,
    },
    /// The sector bound to `slot` has lost at least `min` of its jobs in the
    /// jurisdiction since the baseline (E1; a share, 0..1).
    SectorDecline {
        #[serde(alias = "role")]
        slot: String,
        min: f64,
    },
    /// The place bound to `slot` is open (DEC-090 D3).
    PlaceOpen {
        #[serde(alias = "role")]
        slot: String,
    },
    /// The place bound to `slot` has closed.
    PlaceClosed {
        #[serde(alias = "role")]
        slot: String,
    },
    // --- Staged arcs (DEC-109; tier-generic) ------------------------------------------
    /// A run of this arc is at one of these stages (any, if empty) in the
    /// jurisdiction: for overheard lines, topics and whatever holds while a
    /// stage does. With `here`, only where that role of the run is: its
    /// place, or a place or street in its area.
    Arc {
        arc: ContentKey,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        stages: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        here: Option<String>,
    },
    /// The run has (or has not) this mark. Inside an arc only.
    ArcMark {
        mark: String,
        #[serde(default = "yes")]
        is: bool,
    },
    /// The run's heat within [min, max] (0..1). Inside an arc only.
    ArcHeat {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// Days since the run's current stage was entered, within [min, max].
    /// Inside an arc only.
    ArcDays {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// The share of the residents of the area cast in `slot` who are
    /// staying elsewhere after a disaster, within [min, max] (0..1; without
    /// bounds it always holds): `$away` (people). Inside an arc only.
    Displaced {
        #[serde(alias = "role")]
        slot: String,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// The share of the homes damaged in the area cast in `slot` when the
    /// arc was raised that are repaired, within [min, max] (0..1; without
    /// bounds it always holds): `$homes` (damaged then), `$repaired`.
    /// Inside an arc only.
    Recovered {
        #[serde(alias = "role")]
        slot: String,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// The last run of this arc in the jurisdiction that reached a turn
    /// reached it at this stage (`is: false`: at another, or none has).
    /// Climax memory by variant: the same turn does not come two years
    /// running. Inside an arc only.
    ArcLastTurn {
        stage: String,
        #[serde(default = "yes")]
        is: bool,
    },
    // --- The budget and the body that votes it (DEC-109; tier-generic) --------------
    /// Days until the jurisdiction's budget year turns (its treasury closes
    /// its books), within [min, max]: the budget calendar.
    BudgetYear {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// The balance of the jurisdiction's treasury over its last closed
    /// budget year, as a share of that year's revenue, within [min, max]
    /// (negative: it spent more than it took; no closed year reads 0;
    /// without bounds it always holds): `$books` (`short`, `over` or
    /// `level`: beyond a hundredth either way) and `$gap` (the share in
    /// whole per cent, never negative).
    BudgetBalance {
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// One party holds (or, `is: false`, none holds) more than half of the
    /// posts for the jurisdiction whose constitution lists this power; with
    /// `share`, at least that share of them (0.6: a majority with a seat
    /// to spare in a body of seven).
    BodyMajority {
        power: Power,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        share: Option<f64>,
        #[serde(default = "yes")]
        is: bool,
    },
    /// The share of the holders of posts for the jurisdiction that list
    /// this power (the player not counted) who would back the player,
    /// within [min, max] (0..1; without bounds it always holds): those of
    /// the player's party, those whose opinion of the player is at least
    /// `opinion` (default 10), and those who owe the player a favour.
    /// `$backers` and `$members` (counts).
    BodySupport {
        power: Power,
        #[serde(default)]
        opinion: Option<f64>,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
    },
    /// The last race the player stood in is decided, and they won it (or,
    /// `won: false`, lost it). Never holds while they have stood in none.
    LastRace {
        #[serde(default = "yes")]
        won: bool,
    },
    /// The person in the slot sponsors a bill the player can still declare
    /// a vote on.
    SponsorsBill {
        #[serde(alias = "role")]
        slot: String,
    },
    // --- Named plants (DEC-110; tier-generic) -----------------------------------------
    /// A named plant in one of these states (none given: any): with `slot`,
    /// the plant at the place cast in that role of an arc (the open one,
    /// else the last that closed there); without, any plant in the
    /// jurisdiction's towns. With `closes_within_days`, under notice with
    /// its date at most that many days off. `$plant_key`, `$stem`,
    /// `$works_key`, `$product_key`, `$jobs`; under notice also
    /// `$closes_day`, `$closes_month_key`, `$closes_year` (the date).
    Plant {
        #[serde(default, alias = "role", skip_serializing_if = "Option::is_none")]
        slot: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        status: Vec<PlantState>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        closes_within_days: Option<u32>,
    },
    // --- Composition ----------------------------------------------------------------
    /// Any one of these holds (the first that does gives the arguments).
    Any { of: Vec<Condition> },
}

fn three() -> u8 {
    3
}

fn yes() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

fn one() -> f64 {
    1.0
}

fn heard() -> Level {
    Level::Heard
}

fn is_town(of: &ConcernOf) -> bool {
    *of == ConcernOf::Town
}

/// A named plant's state in its review (DEC-110, `INDUSTRIES.md` §3).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PlantState {
    /// Working full time.
    Running,
    /// On short time.
    ShortTime,
    /// Under notice, with a date.
    Threatened,
    /// Shut.
    Closed,
}

impl PlantState {
    pub fn key(self) -> &'static str {
        match self {
            PlantState::Running => "running",
            PlantState::ShortTime => "short_time",
            PlantState::Threatened => "threatened",
            PlantState::Closed => "closed",
        }
    }
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

/// Whose scandal a condition is about.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HeardSubject {
    #[default]
    Anyone,
    Player,
    Rival,
}

/// How a worry is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Worry {
    /// As residents feel it (their levels weighed by their attitudes), as
    /// the notebook ranks worries.
    Felt,
    /// By measured levels.
    Measured,
}

/// A jurisdiction's phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// A campaign for one of its posts is running.
    Campaign,
    /// No campaign is running.
    Governing,
}

/// A worry condition with its defaults resolved ([`Condition::worry`]).
#[derive(Clone, Debug, PartialEq)]
pub struct WorryReading<'a> {
    pub of: ConcernOf,
    pub concerns: Vec<&'a ContentKey>,
    pub top: u8,
    pub by: Worry,
}

/// Whether a variant reads only what every tier has, or a town's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CondTier {
    /// Holds for any jurisdiction (the player, a person, the calendar, a
    /// conversation).
    Generic,
    /// Reads a town's state; never holds for a region or the nation.
    Local,
}

/// What a variant needs from where it is used (the validator's table).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Needs {
    /// It reads what the player knows (or what a speaker would tell them):
    /// framing only, refused where a condition decides whether something
    /// happens (DEC-091, K6).
    pub ledger: bool,
    /// It reads a person in a slot (a cast role or a conversation's slot).
    pub slot: bool,
    /// It reads an actor cast in a role (a bill): a cast is needed.
    pub cast: bool,
    /// It reads the conversation (its speaker's view, its subject, its voice).
    pub talk: bool,
    /// It reads what only the whole game state holds (the event log,
    /// people's routines, the clock): never inside the simulation's
    /// systems (dilemmas, goals, arcs).
    pub state: bool,
    /// It reads where it is said or done (a place, a neighbourhood).
    pub here: bool,
    /// It reads a policy family's bound slot (a place, neighbourhood or
    /// sector): only a family's trigger or expiry binds them.
    pub family: bool,
    /// It reads an arc run (its marks, heat, days, or what a role of it is
    /// cast to): only inside an arc.
    pub arc: bool,
}

impl Condition {
    /// The type tag, as content writes it.
    pub fn key(&self) -> &'static str {
        use Condition as C;
        match self {
            C::Crisis { .. } => "crisis",
            C::Indicator { .. } => "indicator",
            C::Concern { .. } => "concern",
            C::Industry { .. } => "industry",
            C::PlantClosure { .. } => "plant_closure",
            C::Disaster { .. } => "disaster",
            C::LawInForce { .. } => "law_in_force",
            C::Water { .. } => "water",
            C::Dry => "dry",
            C::Broker => "broker",
            C::Rival => "rival",
            C::RivalRecord { .. } => "rival_record",
            C::Scandal { .. } => "scandal",
            C::Headline => "headline",
            C::PlayerAwareness { .. } => "player_awareness",
            C::PlayerApproval { .. } => "player_approval",
            C::Shedding { .. } => "shedding",
            C::Present { .. } => "present",
            C::DayPart { .. } => "day_part",
            C::Phase { .. } => "phase",
            C::ElectionWithin { .. } => "election_within",
            C::MonthsInPlay { .. } => "months_in_play",
            C::Campaigning { .. } => "campaigning",
            C::Office { .. } => "office",
            C::HoldsPower { .. } => "holds_power",
            C::Track { .. } => "track",
            C::Money { .. } => "money",
            C::Answered { .. } => "answered",
            C::SponsoredLaw { .. } => "sponsored_law",
            C::Knows { .. } => "knows",
            C::PlayerKnows { .. } => "player_knows",
            C::Occupation { .. } => "occupation",
            C::Trait { .. } => "trait",
            C::Personality { .. } => "personality",
            C::Goal { .. } => "goal",
            C::Opinion { .. } => "opinion",
            C::Trust { .. } => "trust",
            C::Remembers { .. } => "remembers",
            C::Favour { .. } => "favour",
            C::BillCosts { .. } => "bill_costs",
            C::Met { .. } => "met",
            C::IsBroker { .. } => "is_broker",
            C::IsRival { .. } => "is_rival",
            C::PartyLineAgainstWard { .. } => "party_line_against_ward",
            C::CanVote { .. } => "can_vote",
            C::Tells { .. } => "tells",
            C::Voice { .. } => "voice",
            C::About { .. } => "about",
            C::HoodConcern { .. } => "hood_concern",
            C::HoodHazard { .. } => "hood_hazard",
            C::SectorDecline { .. } => "sector_decline",
            C::PlaceOpen { .. } => "place_open",
            C::PlaceClosed { .. } => "place_closed",
            C::Arc { .. } => "arc",
            C::ArcMark { .. } => "arc_mark",
            C::ArcHeat { .. } => "arc_heat",
            C::ArcDays { .. } => "arc_days",
            C::Displaced { .. } => "displaced",
            C::Recovered { .. } => "recovered",
            C::ArcLastTurn { .. } => "arc_last_turn",
            C::BudgetYear { .. } => "budget_year",
            C::BudgetBalance { .. } => "budget_balance",
            C::BodyMajority { .. } => "body_majority",
            C::BodySupport { .. } => "body_support",
            C::LastRace { .. } => "last_race",
            C::SponsorsBill { .. } => "sponsors_bill",
            C::Plant { .. } => "plant",
            C::Any { .. } => "any",
        }
    }

    /// Tier-generic or local-only (DEC-098). An `any` is local-only if one
    /// of its alternatives is.
    pub fn tier(&self) -> CondTier {
        use Condition as C;
        match self {
            C::Crisis { .. }
            | C::Indicator { .. }
            | C::Concern { .. }
            | C::Industry { .. }
            | C::PlantClosure { .. }
            | C::Disaster { .. }
            | C::LawInForce { .. }
            | C::Water { .. }
            | C::Dry
            | C::Broker
            | C::Rival
            | C::RivalRecord { .. }
            | C::Scandal { .. }
            | C::Headline
            | C::PlayerAwareness { .. }
            | C::PlayerApproval { .. }
            | C::Shedding { .. }
            | C::Present { .. }
            | C::IsBroker { .. }
            | C::IsRival { .. }
            | C::HoodConcern { .. }
            | C::HoodHazard { .. }
            | C::SectorDecline { .. }
            | C::PlaceOpen { .. }
            | C::PlaceClosed { .. } => CondTier::Local,
            C::Any { of } => {
                if of.iter().any(|c| c.tier() == CondTier::Local) {
                    CondTier::Local
                } else {
                    CondTier::Generic
                }
            }
            _ => CondTier::Generic,
        }
    }

    /// What this variant needs from where it is used (not its `any`
    /// alternatives: the validator checks each).
    pub fn needs(&self) -> Needs {
        use Condition as C;
        let mut n = Needs::default();
        match self {
            C::Knows { .. } => n.ledger = true,
            C::PlayerKnows { .. } | C::Tells { .. } => {
                n.ledger = true;
                n.talk = true;
            }
            C::Voice { .. } | C::About { .. } => n.talk = true,
            C::Concern { of, .. } => match of {
                ConcernOf::Speaker => n.talk = true,
                ConcernOf::Here => n.here = true,
                ConcernOf::Town => {}
            },
            C::Occupation { .. }
            | C::Trait { .. }
            | C::Personality { .. }
            | C::Goal { .. }
            | C::Opinion { .. }
            | C::Trust { .. }
            | C::Remembers { .. }
            | C::Favour { .. }
            | C::BillCosts { .. }
            | C::Met { .. }
            | C::SponsorsBill { .. }
            | C::IsBroker { .. }
            | C::IsRival { .. } => n.slot = true,
            C::PartyLineAgainstWard { .. } | C::CanVote { .. } => {
                n.slot = true;
                n.cast = true;
            }
            C::SponsoredLaw { slot, .. } => {
                n.cast = slot.is_some();
                n.slot = slot.is_some();
            }
            C::Answered { same_cast, .. } => {
                n.cast = !same_cast.is_empty();
                n.slot = !same_cast.is_empty();
            }
            C::PlantClosure { .. } | C::DayPart { .. } => n.state = true,
            C::Present { .. } | C::Shedding { .. } => {
                n.state = true;
                n.here = true;
            }
            C::Water {
                feature: WaterFeature::Any,
            }
            | C::Dry
            | C::PlayerAwareness { .. }
            | C::PlayerApproval { .. } => n.here = true,
            C::HoodConcern { .. }
            | C::HoodHazard { .. }
            | C::SectorDecline { .. }
            | C::PlaceOpen { .. }
            | C::PlaceClosed { .. } => n.family = true,
            C::ArcMark { .. }
            | C::ArcHeat { .. }
            | C::ArcDays { .. }
            | C::Displaced { .. }
            | C::Recovered { .. }
            | C::ArcLastTurn { .. } => n.arc = true,
            C::Arc { here, .. } => n.here = here.is_some(),
            // With a slot it reads a run's role.
            C::Plant { slot, .. } => n.arc = slot.is_some(),
            _ => {}
        }
        n
    }

    /// Whether a condition reads an arc run anywhere in it (`any` included).
    pub fn reads_arc(&self) -> bool {
        match self {
            Condition::Any { of } => of.iter().any(|c| c.reads_arc()),
            c => c.needs().arc,
        }
    }

    /// The family slot a slot-relative family condition names, and the kind
    /// of slot it needs.
    pub fn family_slot(&self) -> Option<(&str, SlotKind)> {
        use Condition as C;
        match self {
            C::HoodConcern { slot, .. } | C::HoodHazard { slot, .. } => {
                Some((slot, SlotKind::Neighborhood))
            }
            C::SectorDecline { slot, .. } => Some((slot, SlotKind::Sector)),
            C::PlaceOpen { slot } | C::PlaceClosed { slot } => Some((slot, SlotKind::Place)),
            _ => None,
        }
    }

    /// The slots (cast roles, or a conversation's slots) a condition names,
    /// not inside `any`.
    pub fn slots(&self) -> Vec<&str> {
        use Condition as C;
        match self {
            C::Occupation { slot, .. }
            | C::Trait { slot, .. }
            | C::Personality { slot, .. }
            | C::Goal { slot, .. }
            | C::Opinion { slot, .. }
            | C::Trust { slot, .. }
            | C::Remembers { slot, .. }
            | C::Favour { slot, .. }
            | C::BillCosts { slot }
            | C::Met { slot, .. }
            | C::SponsorsBill { slot }
            | C::IsBroker { slot, .. }
            | C::IsRival { slot, .. }
            | C::PartyLineAgainstWard { slot }
            | C::CanVote { slot }
            | C::HoodConcern { slot, .. }
            | C::HoodHazard { slot, .. }
            | C::SectorDecline { slot, .. }
            | C::PlaceOpen { slot }
            | C::PlaceClosed { slot }
            | C::Displaced { slot, .. }
            | C::Recovered { slot, .. } => vec![slot.as_str()],
            C::Knows { subject, .. } => subject.role().into_iter().collect(),
            C::Answered { same_cast, .. } => same_cast.iter().map(|s| s.as_str()).collect(),
            C::SponsoredLaw { slot, .. } | C::Plant { slot, .. } => {
                slot.iter().map(|s| s.as_str()).collect()
            }
            _ => Vec::new(),
        }
    }

    /// Every slot a condition names, `any` alternatives included.
    pub fn all_slots(&self) -> Vec<&str> {
        let mut out = self.slots();
        if let Condition::Any { of } = self {
            for c in of {
                out.extend(c.all_slots());
            }
        }
        out
    }

    /// The Fluent arguments a condition provides when it holds (for `any`,
    /// those every alternative provides); `None` where they are not static
    /// (a retold story brings its headline's own).
    pub fn provides(&self) -> Option<BTreeSet<String>> {
        use Condition as C;
        let set = |xs: &[&str]| Some(xs.iter().map(|x| x.to_string()).collect());
        match self {
            C::Crisis { .. } => set(&["crisis_key"]),
            C::Concern { .. } => set(&["concern_key", "concern_talk_key"]),
            C::Industry { .. } | C::Shedding { .. } => set(&["industry_key"]),
            C::PlantClosure { .. } => set(&["industry_key", "jobs"]),
            C::Broker => set(&["broker"]),
            C::Rival | C::RivalRecord { .. } => set(&["rival"]),
            C::PlayerAwareness { .. } | C::PlayerApproval { .. } => set(&["player"]),
            C::Scandal { .. } => set(&["subject", "fact_key"]),
            C::Headline => None,
            C::Remembers { .. } => set(&["memory_key", "subject_key", "subject_kind", "year"]),
            C::Favour { .. } => set(&["n"]),
            C::BillCosts { .. } => set(&["law_key", "bind"]),
            C::IsBroker { is: true, .. } => set(&["broker", "broker_place", "stake_key"]),
            C::IsRival { is: true, .. } => set(&["rival"]),
            C::Displaced { .. } => set(&["away"]),
            C::Recovered { .. } => set(&["homes", "repaired"]),
            C::BudgetBalance { .. } => set(&["books", "gap"]),
            C::BodySupport { .. } => set(&["backers", "members"]),
            C::Plant { .. } => set(&[
                "plant_key",
                "stem",
                "works_key",
                "product_key",
                "jobs",
                "closes_day",
                "closes_month_key",
                "closes_year",
            ]),
            C::Tells { subject, .. } | C::PlayerKnows { subject, .. } => set(subject.args()),
            C::Any { of } => {
                let mut it = of.iter().map(|c| c.provides());
                let mut acc = it.next().flatten()?;
                for x in it {
                    acc = acc.intersection(&x?).cloned().collect();
                }
                Some(acc)
            }
            _ => set(&[]),
        }
    }

    /// A worry condition with its defaults resolved (`None` for any other
    /// condition): with one `concern`, measured and top 3; with `concerns`,
    /// felt and top 1; explicit `top` and `by` win.
    pub fn worry(&self) -> Option<WorryReading<'_>> {
        let Condition::Concern {
            of,
            concern,
            concerns,
            top,
            by,
        } = self
        else {
            return None;
        };
        let single = concern.is_some();
        Some(WorryReading {
            of: *of,
            concerns: concern.iter().chain(concerns.iter()).collect(),
            top: top.unwrap_or(if single { 3 } else { 1 }),
            by: by.unwrap_or(if single { Worry::Measured } else { Worry::Felt }),
        })
    }

    /// Whether a condition reads the player's ledger (or what a speaker
    /// would tell them) anywhere in it, `any` alternatives included.
    pub fn reads_knowledge(&self) -> bool {
        match self {
            Condition::Any { of } => of.iter().any(|c| c.reads_knowledge()),
            c => c.needs().ledger,
        }
    }
}

/// Every condition, depth first (`any` alternatives too).
pub fn walk<'a>(cs: &'a [Condition], out: &mut Vec<&'a Condition>) {
    for c in cs {
        out.push(c);
        if let Condition::Any { of } = c {
            walk(of, out);
        }
    }
}

// --- Where conditions are used: the validator's table -----------------------------------

/// Every place content writes conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConditionUse {
    /// A dilemma's triggers (the director, inside the simulation).
    DilemmaTrigger,
    /// An NPC goal's conditions (the politics domain).
    GoalCondition,
    /// A scene variant (framing; no content field yet).
    SceneVariant,
    /// An ask's trigger (E6–E8; no content field yet).
    Ask,
    /// A policy family's trigger or expiry (F1): its slots are bound.
    FamilyTrigger,
    /// A situational policy's unlock.
    PolicyUnlock,
    /// When a conversation topic is offered.
    TalkTopic,
    /// When a conversation line can be said.
    TalkLine,
    /// When a voice is a speaker's.
    TalkVoice,
    /// When an overheard line can be heard.
    Overheard,
    /// When a visit verb can be done.
    VisitVerb,
    /// A visit risk term.
    VisitRisk,
    /// An arc's `raise`, at a jurisdiction (no cast yet).
    ArcRaise,
    /// An arc stage's edge: the run's marks, heat and days, its roles.
    ArcEdge,
    /// When an arc's beat is offered, and what gates an effect.
    ArcGate,
    /// An arc's readings for its text.
    ArcFacts,
    /// A reply to an arc's choice in a conversation: the conversation's
    /// slots, and the run.
    ArcReply,
}

/// Which slots a use binds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slots {
    /// None (a town's situation, a place).
    None,
    /// The item's cast roles.
    Cast,
    /// A conversation's `speaker`, `about`, `player`.
    Talk,
    /// A policy family's slots (places, neighbourhoods, people, sectors,
    /// crises, hazards).
    Family,
}

/// What a use provides (the other half of the validator's table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Provides {
    pub slots: Slots,
    /// Evaluated on the whole state (views, commands), not inside the
    /// simulation's systems.
    pub state: bool,
    /// A place or neighbourhood where it is said or done.
    pub here: bool,
    /// It decides whether something happens (DEC-091: the ledger is refused).
    pub decides: bool,
    /// An arc run is bound: its marks, heat and days, and its roles.
    pub arc: bool,
}

impl ConditionUse {
    pub const ALL: [ConditionUse; 17] = [
        ConditionUse::DilemmaTrigger,
        ConditionUse::GoalCondition,
        ConditionUse::SceneVariant,
        ConditionUse::Ask,
        ConditionUse::FamilyTrigger,
        ConditionUse::PolicyUnlock,
        ConditionUse::TalkTopic,
        ConditionUse::TalkLine,
        ConditionUse::TalkVoice,
        ConditionUse::Overheard,
        ConditionUse::VisitVerb,
        ConditionUse::VisitRisk,
        ConditionUse::ArcRaise,
        ConditionUse::ArcEdge,
        ConditionUse::ArcGate,
        ConditionUse::ArcFacts,
        ConditionUse::ArcReply,
    ];

    /// How content and messages name it.
    pub fn label(self) -> &'static str {
        match self {
            ConditionUse::DilemmaTrigger => "a dilemma's triggers",
            ConditionUse::GoalCondition => "a goal's conditions",
            ConditionUse::SceneVariant => "a scene variant",
            ConditionUse::Ask => "an ask's trigger",
            ConditionUse::FamilyTrigger => "a policy family's trigger",
            ConditionUse::PolicyUnlock => "a policy's unlock",
            ConditionUse::TalkTopic => "a topic's offer",
            ConditionUse::TalkLine => "a conversation line",
            ConditionUse::TalkVoice => "a voice's conditions",
            ConditionUse::Overheard => "an overheard line",
            ConditionUse::VisitVerb => "a visit verb",
            ConditionUse::VisitRisk => "a visit risk term",
            ConditionUse::ArcRaise => "an arc's raise",
            ConditionUse::ArcEdge => "an arc stage's edge",
            ConditionUse::ArcGate => "an arc's beat or effect",
            ConditionUse::ArcFacts => "an arc's facts",
            ConditionUse::ArcReply => "a reply to an arc's choice",
        }
    }

    pub fn provides(self) -> Provides {
        use ConditionUse as U;
        let p = |slots, state, here, decides| Provides {
            slots,
            state,
            here,
            decides,
            arc: false,
        };
        let arc = |slots, state, here| Provides {
            slots,
            state,
            here,
            decides: true,
            arc: true,
        };
        match self {
            // Inside the simulation's systems: the scheduler's reads only.
            U::DilemmaTrigger | U::GoalCondition | U::Ask => p(Slots::Cast, false, false, true),
            U::SceneVariant => p(Slots::Cast, true, true, false),
            U::FamilyTrigger => p(Slots::Family, true, false, true),
            U::PolicyUnlock => p(Slots::None, true, false, true),
            U::TalkTopic | U::TalkLine | U::TalkVoice => p(Slots::Talk, true, true, false),
            U::Overheard => p(Slots::None, true, true, false),
            U::VisitVerb | U::VisitRisk => p(Slots::None, true, true, true),
            // Inside the simulation (the arcs system), at a jurisdiction;
            // the ledger never decides (STAGED_ARCS §2).
            U::ArcRaise => p(Slots::None, false, false, true),
            U::ArcEdge | U::ArcGate | U::ArcFacts => arc(Slots::Cast, false, false),
            // In a conversation (the whole state, its place).
            U::ArcReply => arc(Slots::Talk, true, true),
        }
    }

    /// Why `c` (not its `any` alternatives) is not allowed here, if it is
    /// not: the first need this use does not meet.
    pub fn refuses(self, c: &Condition) -> Option<String> {
        let n = c.needs();
        let p = self.provides();
        let what = self.label();
        let t = c.key();
        if n.ledger && p.decides {
            return Some(format!(
                "a `{t}` condition in {what}: what the player knows frames what they are \
                 shown; it never decides whether something happens (DEC-091)"
            ));
        }
        if n.talk && p.slots != Slots::Talk {
            return Some(format!(
                "`{t}` reads a conversation (its speaker, subject or voice); {what} has none"
            ));
        }
        if n.arc && !p.arc {
            return Some(format!(
                "`{t}` reads an arc run (its marks, heat, days or roles); {what} has none"
            ));
        }
        if n.family && p.slots != Slots::Family {
            return Some(format!(
                "`{t}` reads a policy family's bound slot; only a family's trigger or expiry \
                 binds them, and {what} has none"
            ));
        }
        if n.cast && p.slots != Slots::Cast {
            return Some(format!(
                "`{t}` reads an actor cast in a role; {what} has no cast"
            ));
        }
        if n.slot && matches!(p.slots, Slots::None | Slots::Family) {
            return Some(format!(
                "this condition reads a slot (a conversation's speaker or subject, or a cast \
                 role); no slot is bound in {what}"
            ));
        }
        if n.state && !p.state {
            return Some(format!(
                "`{t}` reads what only the whole game state holds (the event log, people's \
                 routines, the clock); {what} is evaluated inside the simulation, where it never holds"
            ));
        }
        if n.here && !p.here {
            return Some(format!(
                "`{t}` reads where it is said or done (a place, a neighbourhood); {what} has none"
            ));
        }
        None
    }
}

/// Where a condition list is checked: the item, its use, its tier and the
/// slots bound there (a cast's role ids, or a conversation's slots).
#[derive(Clone, Copy, Debug)]
pub struct Site<'a> {
    pub owner: &'a str,
    pub at: ConditionUse,
    /// The item's tier (a non-local item uses tier-generic conditions only).
    pub tier: TierKey,
    pub slots: &'a [&'a str],
}

impl<'a> Site<'a> {
    pub fn new(owner: &'a str, at: ConditionUse, slots: &'a [&'a str]) -> Self {
        Site {
            owner,
            at,
            tier: TierKey::Local,
            slots,
        }
    }

    pub fn tier(mut self, tier: TierKey) -> Self {
        self.tier = tier;
        self
    }
}

// --- The one validator ----------------------------------------------------------------

/// Check a list of conditions (each, and `any` alternatives) where it is used.
pub fn check_all(b: &ContentBundle, site: &Site<'_>, cs: &[Condition], issues: &mut Vec<Issue>) {
    for c in cs {
        check(b, site, c, issues);
    }
}

/// Check one condition where it is used: allowed here (the table), tier,
/// slots that exist, and its own fields (real ids, sane ranges).
pub fn check(b: &ContentBundle, site: &Site<'_>, c: &Condition, issues: &mut Vec<Issue>) {
    let owner = site.owner;
    if let Some(why) = site.at.refuses(c) {
        issues.push(Issue::error(owner, &why));
        return;
    }
    if site.tier != TierKey::Local
        && c.tier() == CondTier::Local
        && !matches!(c, Condition::Any { .. })
    {
        issues.push(Issue::error(
            owner,
            &format!(
                "`{}` reads a town's state; a {} item uses only tier-generic conditions (DEC-098)",
                c.key(),
                match site.tier {
                    TierKey::Local => "local",
                    TierKey::Regional => "regional",
                    TierKey::National => "national",
                }
            ),
        ));
        return;
    }
    let cast = site.at.provides().slots == Slots::Cast;
    for s in c.slots() {
        if site.slots.contains(&s) {
            continue;
        }
        let msg = if cast {
            format!(
                "names unknown role `{s}` (roles: {})",
                site.slots.join(", ")
            )
        } else {
            format!(
                "unknown slot `{s}` here (expected {})",
                site.slots.join(", ")
            )
        };
        issues.push(Issue::error(owner, &msg));
    }
    fields(b, site, c, issues);
}

/// [min, max] within [lo, hi], with at least one bound.
fn range(
    owner: &str,
    what: &str,
    min: &Option<f64>,
    max: &Option<f64>,
    lo: f64,
    hi: f64,
    issues: &mut Vec<Issue>,
) {
    if min.is_none() && max.is_none() {
        issues.push(Issue::error(
            owner,
            &format!("a {what} condition needs a min or a max"),
        ));
    }
    for v in min.iter().chain(max.iter()) {
        if !(v.is_finite() && (lo..=hi).contains(v)) {
            issues.push(Issue::error(
                owner,
                &format!("a {what} bound {v} is outside [{lo}, {hi}]"),
            ));
        }
    }
    if let (Some(a), Some(z)) = (min, max)
        && a > z
    {
        issues.push(Issue::error(owner, &format!("{what} min is above max")));
    }
}

fn fields(b: &ContentBundle, site: &Site<'_>, c: &Condition, issues: &mut Vec<Issue>) {
    use Condition as C;
    let owner = site.owner;
    let err = |issues: &mut Vec<Issue>, m: String| issues.push(Issue::error(owner, &m));
    let warn = |issues: &mut Vec<Issue>, m: String| issues.push(Issue::warning(owner, &m));
    // A view of the player (or a favour, the broker, the rival) is someone
    // else's, never the player's own.
    let not_player = |slot: &str, issues: &mut Vec<Issue>| {
        if slot == "player" {
            issues.push(Issue::error(
                owner,
                "this condition is about someone's view of the player, not the player",
            ));
        }
    };
    let within = |m: &u16, issues: &mut Vec<Issue>| {
        if !(1..=120).contains(m) {
            issues.push(Issue::error(owner, "within_months must be 1–120"));
        }
    };
    match c {
        C::Crisis { kind, min_severity } => {
            // A pack may remove a crisis kind: the condition then never
            // holds (a warning).
            if let Some(k) = kind
                && b.crisis_index(k).is_none()
            {
                warn(
                    issues,
                    format!("unknown crisis `{k}` (the condition never holds)"),
                );
            }
            if !(min_severity.is_finite() && (0.0..=1.0).contains(min_severity)) {
                err(issues, "min_severity must be in [0, 1]".into());
            }
        }
        C::Indicator { min, max, .. } => {
            range(owner, "indicator", min, max, -1000.0, 1000.0, issues)
        }
        C::Concern {
            concern,
            concerns,
            top,
            ..
        } => {
            if concern.is_some() && !concerns.is_empty() {
                err(
                    issues,
                    "a concern condition names `concern` or `concerns`, not both".into(),
                );
            }
            if concern.is_none() && concerns.is_empty() {
                err(issues, "a concern condition needs concerns".into());
            }
            for k in concern.iter().chain(concerns.iter()) {
                // A pack may remove a concern: that alternative never holds.
                if b.concern_index(k).is_none() {
                    warn(
                        issues,
                        format!("unknown concern `{k}` (the condition never holds)"),
                    );
                }
            }
            if *top == Some(0) {
                err(issues, "concern `top` must be at least 1".into());
            }
        }
        C::Industry { drivers } => {
            if drivers.as_ref().is_some_and(|d| d.is_empty()) {
                err(
                    issues,
                    "industry `drivers`, when given, needs at least one driver".into(),
                );
            }
        }
        C::PlantClosure { within_months } | C::Disaster { within_months, .. } => {
            within(within_months, issues)
        }
        C::LawInForce { templates, .. } => {
            if templates.is_empty() {
                err(issues, "law_in_force needs templates".into());
            }
            for t in templates {
                if !b.policies.contains_key(t) && b.family(t).is_none() {
                    err(issues, format!("unknown policy or family `{t}`"));
                }
            }
        }
        C::RivalRecord { min, max } => range(owner, "rival_record", min, max, 0.0, 1.0, issues),
        C::PlayerAwareness { min, max } => {
            range(owner, "player_awareness", min, max, 0.0, 1.0, issues)
        }
        C::PlayerApproval { min, max } => {
            range(owner, "player_approval", min, max, -1.0, 1.0, issues)
        }
        C::Shedding { min } => {
            if !(min.is_finite() && (0.0..=1.0).contains(min)) {
                err(issues, "shedding `min` must be in [0, 1]".into());
            }
        }
        C::Present { occupations } => {
            if occupations.is_empty() {
                err(issues, "a present condition needs occupations".into());
            }
            for o in occupations {
                if b.occupation(o).is_none() {
                    err(issues, format!("unknown occupation `{o}`"));
                }
            }
        }
        C::DayPart { parts } => {
            if parts.is_empty() {
                err(issues, "a day_part condition needs parts".into());
            }
        }
        C::ElectionWithin { days } => {
            if *days == 0 || *days > 3650 {
                err(issues, "election_within days must be in 1..3650".into());
            }
        }
        C::Office { posts, role } => {
            if posts.is_empty() && role.is_none() {
                err(issues, "an office condition needs `posts` or `role`".into());
            }
            for w in posts.iter().chain(role.iter()) {
                if w != crate::OFFICE_ANY && w != crate::OFFICE_NONE && !b.is_role_word(w) {
                    err(
                        issues,
                        format!("unknown office word `{w}` (not a constitution role word)"),
                    );
                }
            }
        }
        C::Track { min, max, .. } => range(owner, "track", min, max, 0.0, 100.0, issues),
        C::Money { min, max } => range(owner, "money", min, max, -1e9, 1e9, issues),
        C::Answered {
            template,
            option,
            min_days,
            max_days,
            same_cast,
        } => match b.dilemma(template) {
            // A pack may remove the template: the condition never holds.
            None => warn(
                issues,
                format!("unknown dilemma `{template}` (the condition never holds)"),
            ),
            Some(t) => {
                if let Some(o) = option
                    && !t.options.iter().any(|x| &x.id == o)
                {
                    err(issues, format!("`{template}` has no option `{o}`"));
                }
                for r in same_cast {
                    if !t.roles.iter().any(|x| &x.id == r) {
                        err(
                            issues,
                            format!("same_cast role `{r}` is not a role of `{template}`"),
                        );
                    }
                }
                if *min_days > 3650 || max_days.is_some_and(|m| m < *min_days || m > 3650) {
                    err(
                        issues,
                        "answered days must be in 0..3650 with min ≤ max".into(),
                    );
                }
            }
        },
        C::SponsoredLaw {
            min_age_days,
            max_age_days,
            chance,
            ..
        } => {
            if *min_age_days > 3650 || max_age_days.is_some_and(|m| m < *min_age_days || m > 3650) {
                err(
                    issues,
                    "sponsored_law ages must be in 0..3650 with min ≤ max".into(),
                );
            }
            if !(chance.is_finite() && (0.0..=1.0).contains(chance)) {
                err(issues, "sponsored_law chance must be in [0, 1]".into());
            }
        }
        C::Knows { subject, min } => {
            crate::narrative::check_subject(owner, b, subject, issues);
            if *min == Level::Unknown {
                err(
                    issues,
                    "knows `min` must be heard, known or detailed".into(),
                );
            }
        }
        C::PlayerKnows { min, max, .. } => {
            if min.is_none() && max.is_none() {
                err(
                    issues,
                    "a player_knows condition needs a min or a max".into(),
                );
            }
            if let (Some(a), Some(z)) = (min, max)
                && a > z
            {
                err(issues, "player_knows min is above max".into());
            }
        }
        C::Occupation { occupations, .. } => {
            if occupations.is_empty() {
                err(issues, "an occupation condition needs roles".into());
            }
            for o in occupations {
                if b.occupation(o).is_none() && !b.is_role_word(o) {
                    err(issues, format!("unknown role `{o}`"));
                }
            }
        }
        C::Trait { traits, .. } => {
            if traits.is_empty() {
                err(issues, "a trait condition needs traits".into());
            }
            for t in traits {
                if b.trait_def(t).is_none() {
                    err(issues, format!("unknown trait `{t}`"));
                }
            }
        }
        C::Personality { axis, min, max, .. } => {
            if !crate::types::PERSONALITY_AXES.contains(&axis.as_str()) {
                err(issues, format!("unknown personality axis `{axis}`"));
            }
            range(owner, "personality", min, max, 0.0, 1.0, issues);
        }
        C::Goal { slot, goal } => {
            not_player(slot, issues);
            if b.goal(goal).is_none() {
                err(issues, format!("unknown goal `{goal}`"));
            }
        }
        C::Opinion { slot, min, max } => {
            not_player(slot, issues);
            range(owner, "opinion", min, max, -100.0, 100.0, issues);
        }
        C::Trust { slot, min, max } => {
            not_player(slot, issues);
            range(owner, "trust", min, max, 0.0, 1.0, issues);
        }
        C::Remembers { slot, kinds } => {
            not_player(slot, issues);
            if kinds.is_empty() {
                err(issues, "a remembers condition needs memory kinds".into());
            }
            for k in kinds {
                if !crate::types::MEMORY_KINDS.contains(&k.as_str()) {
                    err(issues, format!("unknown memory kind `{k}`"));
                }
            }
        }
        C::Favour { slot, .. }
        | C::BillCosts { slot }
        | C::Met { slot, .. }
        | C::IsBroker { slot, .. }
        | C::IsRival { slot, .. } => not_player(slot, issues),
        C::Tells { min, .. } => {
            if *min == Level::Unknown {
                err(
                    issues,
                    "a tells condition needs `heard`, `known` or `detailed` (unknown always holds)"
                        .into(),
                );
            }
        }
        C::Voice { voices } => {
            if voices.is_empty() {
                err(issues, "a voice condition needs voices".into());
            }
            for v in voices {
                if b.talk_voice(v).is_none() {
                    err(issues, format!("unknown voice `{v}`"));
                }
            }
        }
        C::About {
            kinds,
            places,
            workplace,
            home,
            office,
            works,
            here,
            tied,
            ..
        } => {
            let place_ok = kinds.is_empty() || kinds.contains(&AboutKind::Place);
            if (!places.is_empty() || workplace.is_some()) && !place_ok {
                err(
                    issues,
                    "an about condition with `places` or `workplace` needs the place kind".into(),
                );
            }
            for k in places {
                if b.place_kind(k).is_none() || crate::types::PSEUDO_PLACES.contains(&k.as_str()) {
                    err(issues, format!("unknown place kind `{k}`"));
                }
            }
            let ties = [home, office, works, here, tied];
            if ties.iter().any(|x| x.is_some())
                && !(kinds.is_empty() || kinds.contains(&AboutKind::Hood))
            {
                err(
                    issues,
                    "an about condition with `home`, `office`, `works`, `here` or `tied` needs the hood kind"
                        .into(),
                );
            }
        }
        C::HoodConcern { concern, top, .. } => {
            if b.concern_index(concern).is_none() {
                warn(
                    issues,
                    format!("unknown concern `{concern}` (the condition never holds)"),
                );
            }
            if *top == 0 {
                err(issues, "hood_concern `top` must be at least 1".into());
            }
        }
        C::HoodHazard { min, .. } | C::SectorDecline { min, .. } => {
            if !(min.is_finite() && (0.0..=1.0).contains(min)) {
                err(issues, format!("{} `min` must be in [0, 1]", c.key()));
            }
        }
        C::Arc { arc, stages, .. } => match b.arcs.iter().find(|a| &a.id == arc) {
            // A pack may remove an arc: the condition then never holds.
            None => warn(
                issues,
                format!("unknown arc `{arc}` (the condition never holds)"),
            ),
            Some(a) => {
                for s in stages {
                    if a.stage(s).is_none() {
                        err(issues, format!("`{arc}` has no stage `{s}`"));
                    }
                }
            }
        },
        C::ArcMark { mark, .. } => {
            if mark.is_empty() {
                err(issues, "an arc_mark condition names a mark".into());
            }
        }
        C::ArcHeat { min, max } => range(owner, "arc_heat", min, max, 0.0, 1.0, issues),
        C::ArcDays { min, max } => range(owner, "arc_days", min, max, 0.0, 3650.0, issues),
        C::Displaced { min, max, .. } | C::Recovered { min, max, .. } => {
            // Without bounds it is a reading (an arc's `facts`).
            if min.is_some() || max.is_some() {
                range(owner, c.key(), min, max, 0.0, 1.0, issues);
            }
        }
        C::ArcLastTurn { stage, .. } => {
            if stage.is_empty() {
                err(issues, "an arc_last_turn condition names a stage".into());
            }
        }
        C::BodyMajority { share, .. } => {
            if share.is_some_and(|s| !(0.5..=1.0).contains(&s)) {
                err(issues, "body_majority: `share` is 0.5..1".into());
            }
        }
        C::BudgetYear { min, max } => range(owner, "budget_year", min, max, 0.0, 3660.0, issues),
        C::BudgetBalance { min, max } => {
            if min.is_some() || max.is_some() {
                range(owner, "budget_balance", min, max, -10.0, 10.0, issues);
            }
        }
        C::BodySupport {
            opinion, min, max, ..
        } => {
            if opinion.is_some_and(|o| !(-100.0..=100.0).contains(&o)) {
                err(issues, "body_support: `opinion` is −100..100".into());
            }
            if min.is_some() || max.is_some() {
                range(owner, "body_support", min, max, 0.0, 1.0, issues);
            }
        }
        C::Plant {
            status,
            closes_within_days,
            ..
        } => {
            if closes_within_days.is_some_and(|d| d > 3660) {
                err(issues, "plant: `closes_within_days` is at most 3660".into());
            }
            if closes_within_days.is_some()
                && !status.is_empty()
                && !status.contains(&PlantState::Threatened)
            {
                err(
                    issues,
                    "plant: `closes_within_days` reads a notice, so `status` allows `threatened`"
                        .into(),
                );
            }
        }
        C::Any { of } => {
            if of.is_empty() {
                err(issues, "an `any` condition needs alternatives".into());
            }
            for x in of {
                check(b, site, x, issues);
            }
        }
        C::Water { .. }
        | C::Dry
        | C::Broker
        | C::Rival
        | C::Scandal { .. }
        | C::Headline
        | C::Phase { .. }
        | C::MonthsInPlay { .. }
        | C::Campaigning { .. }
        | C::HoldsPower { .. }
        | C::LastRace { .. }
        | C::SponsorsBill { .. }
        | C::PartyLineAgainstWard { .. }
        | C::CanVote { .. }
        | C::PlaceOpen { .. }
        | C::PlaceClosed { .. } => {}
    }
}

// --- The schema accepts the old spellings ------------------------------------------------

/// The generated schema knows nothing of serde aliases: accept `role`
/// wherever `slot` is a field, and `live_crisis` as a `crisis` without a
/// kind, as the parser does.
fn old_spellings(schema: &mut schemars::Schema) {
    use serde_json::{Value, json};
    let Some(Value::Array(variants)) = schema.get_mut("oneOf") else {
        return;
    };
    let mut live = None;
    for v in variants.iter_mut() {
        let Some(o) = v.as_object_mut() else {
            continue;
        };
        let tag = o
            .get("properties")
            .and_then(|p| p.get("type"))
            .and_then(|t| t.get("const"))
            .and_then(|c| c.as_str())
            .map(str::to_string);
        if tag.as_deref() == Some("crisis") {
            let mut l = Value::Object(o.clone());
            if let Some(p) = l.get_mut("properties").and_then(|p| p.as_object_mut()) {
                p.remove("kind");
                p.insert(
                    "type".into(),
                    json!({ "const": "live_crisis", "type": "string" }),
                );
            }
            if let Some(m) = l.as_object_mut() {
                m.insert(
                    "description".into(),
                    json!("The old spelling of a `crisis` without a `kind`."),
                );
            }
            live = Some(l);
        }
        let slot = o.get("properties").and_then(|p| p.get("slot")).cloned();
        let Some(slot) = slot else {
            continue;
        };
        if let Some(p) = o.get_mut("properties").and_then(|p| p.as_object_mut()) {
            p.insert("role".into(), slot);
        }
        let required = o
            .get_mut("required")
            .and_then(|r| r.as_array_mut())
            .is_some_and(|r| {
                let had = r.iter().any(|x| x == "slot");
                r.retain(|x| x != "slot");
                had
            });
        if required {
            o.insert(
                "oneOf".into(),
                json!([{ "required": ["slot"] }, { "required": ["role"] }]),
            );
        }
    }
    if let Some(l) = live {
        variants.push(l);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Condition {
        serde_json::from_str(s).unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn old_spellings_parse() {
        assert_eq!(
            parse(r#"{ "type": "live_crisis", "min_severity": 0.3 }"#),
            Condition::Crisis {
                kind: None,
                min_severity: 0.3
            }
        );
        assert_eq!(
            parse(r#"{ "type": "opinion", "role": "holder", "min": 10 }"#),
            Condition::Opinion {
                slot: "holder".into(),
                min: Some(10.0),
                max: None
            }
        );
        assert!(
            serde_json::from_str::<Condition>(
                r#"{ "type": "opinion", "role": "a", "slot": "b", "min": 1 }"#
            )
            .is_err()
        );
    }

    #[test]
    fn worry_defaults_follow_the_spelling() {
        let d = parse(r#"{ "type": "concern", "concern": "concern.jobs" }"#);
        let w = d.worry().unwrap();
        assert_eq!((w.top, w.by, w.of), (3, Worry::Measured, ConcernOf::Town));
        let t = parse(r#"{ "type": "concern", "of": "speaker", "concerns": ["concern.jobs"] }"#);
        let w = t.worry().unwrap();
        assert_eq!((w.top, w.by, w.of), (1, Worry::Felt, ConcernOf::Speaker));
    }
}
