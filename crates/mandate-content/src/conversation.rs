//! Conversations (DEC-093; M6_ENGINE_CONTRACTS §3): the content side.
//! Topics (what the player can raise, offered by role, place and
//! conditions), reply lines (openers, memory quotes, substance and closers,
//! each with conditions in the one condition language, `condition.rs`), voices
//! (who speaks how) and the rules as numbers (cost, discretion, what
//! speakers know, the repetition memory, small talk, the mechanics).
//!
//! The engine composes a reply from these lines and applies what it
//! teaches (`mandate_sim::talk`). Topic mechanics, the `who` rules of
//! speaker knowledge, the condition variants and the request types are
//! code; everything else here is data.

use crate::ContentBundle;
use crate::knowledge::Level;
use crate::condition::{Condition, ConditionUse, Site, walk};
use crate::types::ContentKey;
use crate::validate::{Issue, fluent_keys};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The slots a conversation binds: the speaker, the subject (`about`) and
/// the player (people); the place is `here` (subject selectors only).
pub const TALK_SLOTS: [&str; 3] = ["speaker", "about", "player"];

/// Fluent arguments every line is given (`$speaker`, `$player`, `$town`,
/// `$here`).
pub const CONTEXT_ARGS: [&str; 4] = ["speaker", "player", "town", "here"];

// --- Topics ------------------------------------------------------------------------

/// Something the player can raise with someone (DEC-093).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TopicDef {
    /// Content id, e.g. `topic.who_runs`.
    pub id: ContentKey,
    /// Fluent key of the button label.
    pub name_key: String,
    pub mechanic: TalkMechanic,
    /// Which subjects it can be about (empty: none).
    #[serde(default)]
    pub about: Vec<AboutKind>,
    /// The topic needs a subject (a pitch needs a bill); otherwise one is optional.
    #[serde(default)]
    pub about_required: bool,
    #[serde(default)]
    pub offered: TopicOffer,
}

/// What a topic does besides the reply. A small closed set kept in code.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TalkMechanic {
    /// Only the reply (and what it teaches).
    Inform,
    /// Sway on a bill, by the shared lobbying formula.
    Pitch,
    /// Ask for support or a public endorsement (odds shown first).
    AskSupport,
    /// Offer a favour, or call one in (the favour ledger).
    Favour,
    /// A small opinion gain (capped weekly) and a `met` memory.
    SmallTalk,
}

impl TalkMechanic {
    pub fn key(self) -> &'static str {
        match self {
            TalkMechanic::Inform => "inform",
            TalkMechanic::Pitch => "pitch",
            TalkMechanic::AskSupport => "ask_support",
            TalkMechanic::Favour => "favour",
            TalkMechanic::SmallTalk => "small_talk",
        }
    }
}

/// The kind of subject a topic can be about.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AboutKind {
    Place,
    Person,
    Hood,
    Bill,
    Template,
    Proposal,
    Rival,
}

impl AboutKind {
    pub fn key(self) -> &'static str {
        match self {
            AboutKind::Place => "place",
            AboutKind::Person => "person",
            AboutKind::Hood => "hood",
            AboutKind::Bill => "bill",
            AboutKind::Template => "template",
            AboutKind::Proposal => "proposal",
            AboutKind::Rival => "rival",
        }
    }

    /// Kinds whose subject is a person (the `about` slot).
    pub fn is_person(self) -> bool {
        matches!(self, AboutKind::Person | AboutKind::Rival)
    }
}

/// Who offers a topic, where, and when.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TopicOffer {
    /// Occupation ids, `councillor`, `mayor`; empty: anyone.
    #[serde(default)]
    pub roles: Vec<String>,
    /// Place kinds (`streets` for a neighbourhood's streets); empty: anywhere.
    #[serde(default)]
    pub places: Vec<String>,
    /// Every condition must hold (slots: `speaker`, `player`).
    #[serde(default)]
    pub when: Vec<Condition>,
}

// --- Lines -------------------------------------------------------------------------

/// Which part of a reply a line is, in reply order.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TalkPart {
    Opener,
    Quote,
    Substance,
    Closer,
}

impl TalkPart {
    pub const ALL: [TalkPart; 4] = [
        TalkPart::Opener,
        TalkPart::Quote,
        TalkPart::Substance,
        TalkPart::Closer,
    ];

    pub fn key(self) -> &'static str {
        match self {
            TalkPart::Opener => "opener",
            TalkPart::Quote => "quote",
            TalkPart::Substance => "substance",
            TalkPart::Closer => "closer",
        }
    }

    /// The part's number in the keyed draw.
    pub fn tag(self) -> u64 {
        match self {
            TalkPart::Opener => 0,
            TalkPart::Quote => 1,
            TalkPart::Substance => 2,
            TalkPart::Closer => 3,
        }
    }
}

/// One reply line: a Fluent key, the conditions under which it may be said,
/// and what it teaches, where it points and how it moves the relationship.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TalkLineDef {
    /// Content id, e.g. `line.runs.named`.
    pub id: ContentKey,
    pub part: TalkPart,
    /// The topic a substance line answers (required for substance; an
    /// opener with a topic is preferred for that topic).
    #[serde(default)]
    pub topic: Option<ContentKey>,
    /// The voice a closer belongs to (required for closers).
    #[serde(default)]
    pub voice: Option<ContentKey>,
    /// Fluent key of the line.
    pub key: String,
    /// Every condition must hold.
    #[serde(default)]
    pub when: Vec<Condition>,
    /// Relative chance among the lines that hold (default 1).
    #[serde(default = "one_f64")]
    pub weight: f64,
    /// The topic's safe answer for the roles in its `occupation` condition
    /// (or all roles); used only when no other substance line holds.
    #[serde(default)]
    pub fallback: bool,
    /// What the player learns, each capped at the speaker's told level.
    #[serde(default)]
    pub teaches: Vec<TeachDef>,
    /// Where the line points the player.
    #[serde(default)]
    pub lead: Option<TalkLeadDef>,
    /// Changes to the relationship.
    #[serde(default)]
    pub effects: Vec<TalkEffectDef>,
}

fn one_f64() -> f64 {
    1.0
}

fn half() -> f64 {
    0.5
}

/// A subject of the player's ledger, named from the conversation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SubjectRef {
    /// The town's worries: `$concern_key` (the strongest; its noun phrase
    /// of talk, `$concern_talk_key`).
    TownConcerns,
    /// A neighbourhood's worries: `$hood`, `$concern_key`, `$concern_talk_key`.
    HoodConcerns { of: HoodOf },
    /// Who really runs the town: `$broker`, `$broker_place`, `$stake_key`.
    Broker,
    /// A person in a slot (`speaker`, `about`): `$person`.
    Person { slot: String },
    /// The rival's plans: `$rival`.
    RivalPlans,
    /// A proposal sponsored by a person in a slot (DEC-092; holds nothing
    /// until policy families exist).
    Proposal { sponsored_by: String },
    /// Who supports the policy in a slot (`about`): `$policy_key`, `$support_key`.
    PolicySupport { of: String },
}

impl SubjectRef {
    pub fn key(&self) -> &'static str {
        match self {
            SubjectRef::TownConcerns => "town_concerns",
            SubjectRef::HoodConcerns { .. } => "hood_concerns",
            SubjectRef::Broker => "broker",
            SubjectRef::Person { .. } => "person",
            SubjectRef::RivalPlans => "rival_plans",
            SubjectRef::Proposal { .. } => "proposal",
            SubjectRef::PolicySupport { .. } => "policy_support",
        }
    }

    /// The Fluent arguments a condition on this subject provides.
    pub fn args(&self) -> &'static [&'static str] {
        match self {
            SubjectRef::TownConcerns => &["concern_key", "concern_talk_key"],
            SubjectRef::HoodConcerns { .. } => &["hood", "concern_key", "concern_talk_key"],
            SubjectRef::Broker => &["broker", "broker_place", "stake_key"],
            SubjectRef::Person { .. } => &["person"],
            SubjectRef::RivalPlans => &["rival"],
            SubjectRef::Proposal { .. } => &[],
            SubjectRef::PolicySupport { .. } => &["policy_key", "support_key"],
        }
    }
}

/// Whose worries a `concern` condition reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConcernOf {
    /// The speaker's home neighbourhood (what worries them; conversations).
    Speaker,
    /// The town.
    #[default]
    Town,
    /// The neighbourhood where it is said or done (a line heard, a
    /// conversation's place, a place visited).
    Here,
}

/// Which neighbourhood a subject names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HoodOf {
    /// Where the conversation happens.
    Here,
    /// The speaker's home neighbourhood.
    SpeakerHome,
    /// The subject (a neighbourhood, or a place's neighbourhood).
    About,
}

/// One thing a line teaches, capped at the speaker's told level.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TeachDef {
    pub subject: SubjectRef,
    pub level: Level,
}

/// A lead a line opens (a deflecting line usually points somewhere).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TalkLeadDef {
    /// Fluent key (`lead-…`).
    pub key: String,
    pub to: LeadToDef,
    /// The subject following the lead teaches, and to what level.
    pub subject: SubjectRef,
    pub level: Level,
}

/// Where a lead points.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LeadToDef {
    /// The first place of this kind in the town.
    PlaceKind { kind: String },
    /// A person in a slot (`speaker`, `about`).
    Person { slot: String },
    /// The streets of a neighbourhood.
    Streets { of: HoodOf },
}

/// A change to the relationship a line makes (the speaker toward the player).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TalkEffectDef {
    /// Opinion change, scaled by the speaker's personality.
    Opinion { delta: f64 },
    /// A memory of the player (a memory kind).
    Remember { kind: String },
}

// --- Voices ------------------------------------------------------------------------

/// A way of speaking. A speaker's voice is the highest-priority voice whose
/// conditions all hold (ties: load order); `voice.plain` has none.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VoiceDef {
    /// Content id, e.g. `voice.blunt`.
    pub id: ContentKey,
    pub priority: i32,
    /// Every condition must hold (slots: `speaker`, `player`).
    #[serde(default)]
    pub when: Vec<Condition>,
    /// How often a reply in this voice ends with a closer (0..1).
    #[serde(default = "half")]
    pub closer_share: f64,
}

// --- Parameters --------------------------------------------------------------------

/// What a conversation costs (DEC-073; owner, 2026-11-19: half a slot).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TalkCost {
    /// Two conversations fill one slot.
    #[default]
    HalfSlot,
    Slot,
}

/// Up to (but not including) an opinion of the player, the most a speaker tells.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscretionBand {
    /// The band's upper opinion bound (−100..100), exclusive.
    pub below: f64,
    pub tells: Level,
}

/// Discretion: the most a speaker tells, by their opinion of the player and
/// their trust (the bands of the protocol's `opinion_band`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscretionDef {
    /// Ascending bands; above the last, `top`.
    pub bands: Vec<DiscretionBand>,
    /// What the friendliest tell, with trust at least `trust_for_detailed`
    /// (with less trust, one level lower).
    pub top: Level,
    pub trust_for_detailed: f64,
}

/// Who knows a rule's subject (kept in code; the levels are content).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum KnowsWho {
    /// Anyone from the subject's town.
    AnyoneLocal,
    /// An organisation leader (an occupation with `leader: true`).
    Leader,
    /// Someone whose stake covers the town's defining industry or its live crisis.
    StakeOverlap,
    /// The town's power broker.
    Broker,
    /// A politician.
    Politician,
    /// A sponsor of proposals (DEC-092; nobody until policy families exist).
    Sponsor,
    /// Someone with a workplace.
    Workplace,
}

/// What a rule knows.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum KnownSubject {
    /// The town's worries.
    TownConcerns,
    /// The worries of the speaker's own neighbourhood.
    HomeConcerns,
    /// The councillors and the mayor, as people.
    Council,
    Broker,
    RivalPlans,
    /// Support for policies (bills before the council and the menu).
    PolicySupport,
    /// Proposals the speaker sponsors (DEC-092).
    SponsoredProposals,
    /// Proposals of the speaker's workplace (DEC-092).
    WorkplaceProposals,
    /// The people who work where the speaker works.
    WorkplacePeople,
    /// The speaker, as a person.
    Themselves,
}

/// One rule of speaker knowledge: who knows what, to what level. NPCs keep
/// no ledgers: what a speaker knows is derived from these, never stored.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowsDef {
    pub who: KnowsWho,
    pub subject: KnownSubject,
    pub level: Level,
}

/// Pitching a bill (the shared lobbying formula and odds).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PitchParams {
    /// The share of a lobbying meeting's sway a pitch gives (a pitch costs
    /// half the time).
    pub sway_share: f64,
}

/// Asking for support or a public endorsement.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SupportParams {
    /// Odds for a non-politician: sigmoid(base + opinion × per_opinion / 100
    /// + (trust − 0.5) × per_trust), clamped to [min, max].
    pub base: f64,
    pub per_opinion: f64,
    pub per_trust: f64,
    pub min: f64,
    pub max: f64,
    /// A politician who agrees backs the bill: this much sway.
    pub sway: f64,
    /// A public endorsement: approval of the player among the residents of
    /// the endorser's neighbourhood (× the room left).
    pub endorse_approval: f64,
    /// Backing a bill in public raises its awareness by this much.
    pub bill_awareness: f64,
    /// Below this opinion, someone who agrees asks a favour in return.
    pub favour_below: f64,
    /// A refusal's opinion change.
    pub refused_opinion: f64,
}

/// Trading favours in conversation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FavourParams {
    /// The share of a favour trade's sway an offered favour gives.
    pub sway_share: f64,
}

/// The conversation parameter block (`kind: "talk_params"`, once across packs).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TalkParams {
    pub cost: TalkCost,
    pub discretion: DiscretionDef,
    /// Whole levels added to the cap by traits (`trait.schemer`: −1).
    #[serde(default)]
    pub trait_discretion: BTreeMap<ContentKey, i8>,
    /// Added when flattered (after small talk with them this week): `trait.vain`: +1.
    #[serde(default)]
    pub flattered_discretion: BTreeMap<ContentKey, i8>,
    /// Added when a favour they owe the player is called in.
    pub favour_discretion: i8,
    pub speaker_knows: Vec<KnowsDef>,
    /// Lines remembered per person for the repetition guard.
    pub memory_lines: u8,
    /// Exchanges kept in the log (the notebook's "where and when").
    pub log_cap: u16,
    /// How often a memory quote is used when one holds (0..1).
    pub quote_share: f64,
    /// Small talk's opinion gain (scaled by personality).
    pub small_talk_opinion: f64,
    /// Small talks per person per week that still gain anything.
    pub small_talk_weekly_cap: u8,
    pub pitch: PitchParams,
    pub support: SupportParams,
    pub favour: FavourParams,
}

impl Default for TalkParams {
    fn default() -> Self {
        TalkParams {
            cost: TalkCost::HalfSlot,
            discretion: DiscretionDef {
                bands: Vec::new(),
                top: Level::Unknown,
                trust_for_detailed: 0.5,
            },
            trait_discretion: BTreeMap::new(),
            flattered_discretion: BTreeMap::new(),
            favour_discretion: 1,
            speaker_knows: Vec::new(),
            memory_lines: 12,
            log_cap: 40,
            quote_share: 0.0,
            small_talk_opinion: 0.0,
            small_talk_weekly_cap: 1,
            pitch: PitchParams { sway_share: 0.5 },
            support: SupportParams {
                base: 0.0,
                per_opinion: 0.0,
                per_trust: 0.0,
                min: 0.0,
                max: 0.0,
                sway: 0.0,
                endorse_approval: 0.0,
                bill_awareness: 0.0,
                favour_below: 0.0,
                refused_opinion: 0.0,
            },
            favour: FavourParams { sway_share: 0.5 },
        }
    }
}

// --- Lookups -----------------------------------------------------------------------

impl ContentBundle {
    /// A conversation topic by id.
    pub fn talk_topic(&self, id: &str) -> Option<&TopicDef> {
        self.talk_topics.iter().find(|t| t.id == id)
    }

    /// A voice by id.
    pub fn talk_voice(&self, id: &str) -> Option<&VoiceDef> {
        self.talk_voices.iter().find(|v| v.id == id)
    }

    /// Every role a topic can be offered to: occupations, then the role
    /// words of the posts politicians hold (DEC-098).
    pub fn talk_roles(&self) -> Vec<String> {
        self.occupations
            .iter()
            .map(|o| o.id.clone())
            .chain(self.role_words())
            .collect()
    }

    /// The role words of the posts politicians hold and speak as (DEC-098):
    /// the local tier's bodies' `member_key`s and offices' ids, from every
    /// constitution, in constitution order, without repeats. M6 seats the
    /// local tier only; M7 adds the regional tier here.
    pub fn role_words(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for c in self.constitutions.values() {
            let Some(t) = c.tier(crate::types::TierKey::Local) else {
                continue;
            };
            for p in t.posts() {
                if !out.iter().any(|w| w == p.word()) {
                    out.push(p.word().to_string());
                }
            }
        }
        out
    }

    /// Whether `word` is a role word of any constitution's posts, on any
    /// tier (what content may name; DEC-098).
    pub fn is_role_word(&self, word: &str) -> bool {
        self.constitutions
            .values()
            .any(|c| c.role_words().contains(&word))
    }
}

// --- Parameter ranges ----------------------------------------------------------------

/// Range checks for the talk block (DEC-078 decision 1; `--set` paths).
pub fn param_ranges(b: &ContentBundle, r: &mut dyn FnMut(&str, f64, f64, f64)) {
    let t = &b.talk;
    r(
        "talk.discretion.trust_for_detailed",
        t.discretion.trust_for_detailed,
        0.0,
        1.0,
    );
    r(
        "talk.favour_discretion",
        t.favour_discretion.into(),
        -3.0,
        3.0,
    );
    r("talk.memory_lines", t.memory_lines.into(), 1.0, 64.0);
    r("talk.log_cap", t.log_cap.into(), 1.0, 400.0);
    r("talk.quote_share", t.quote_share, 0.0, 1.0);
    r("talk.small_talk_opinion", t.small_talk_opinion, 0.0, 20.0);
    r(
        "talk.small_talk_weekly_cap",
        t.small_talk_weekly_cap.into(),
        0.0,
        14.0,
    );
    r("talk.pitch.sway_share", t.pitch.sway_share, 0.0, 2.0);
    r("talk.favour.sway_share", t.favour.sway_share, 0.0, 2.0);
    let s = &t.support;
    r("talk.support.base", s.base, -5.0, 5.0);
    r("talk.support.per_opinion", s.per_opinion, 0.0, 10.0);
    r("talk.support.per_trust", s.per_trust, 0.0, 10.0);
    r("talk.support.min", s.min, 0.0, 1.0);
    r("talk.support.max", s.max, 0.0, 1.0);
    r("talk.support.sway", s.sway, 0.0, 5.0);
    r(
        "talk.support.endorse_approval",
        s.endorse_approval,
        0.0,
        0.2,
    );
    r("talk.support.bill_awareness", s.bill_awareness, 0.0, 1.0);
    r("talk.support.favour_below", s.favour_below, -100.0, 100.0);
    r(
        "talk.support.refused_opinion",
        s.refused_opinion,
        -20.0,
        0.0,
    );
    for (k, v) in &t.trait_discretion {
        r(
            &format!("talk.trait_discretion.{k}"),
            (*v).into(),
            -3.0,
            3.0,
        );
    }
    for (k, v) in &t.flattered_discretion {
        r(
            &format!("talk.flattered_discretion.{k}"),
            (*v).into(),
            -3.0,
            3.0,
        );
    }
}

// --- Fluent placeholders -------------------------------------------------------------

/// Each Fluent message's variables (`$name`), by message id, over a
/// language's sources (values, attributes and continuation lines).
pub fn fluent_placeholders(text: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let starts_id = line.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
        let body = if starts_id {
            match line.split_once('=') {
                Some((id, rest))
                    if id
                        .trim()
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') =>
                {
                    let id = id.trim().to_string();
                    out.entry(id.clone()).or_default();
                    current = Some(id);
                    rest
                }
                _ => {
                    current = None;
                    continue;
                }
            }
        } else if line.starts_with(' ') || line.starts_with('\t') {
            line
        } else {
            // A comment or a blank line ends nothing in Fluent, but nothing
            // after it belongs to a variable list either.
            if line.trim_start().starts_with('#') {
                current = None;
            }
            continue;
        };
        let Some(id) = current.as_ref() else {
            continue;
        };
        let vars = out.entry(id.clone()).or_default();
        let mut rest = body;
        while let Some(i) = rest.find('$') {
            rest = &rest[i + 1..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if !name.is_empty() {
                vars.insert(name);
            }
        }
    }
    out
}

// --- Validation (M6_ENGINE_CONTRACTS §3.11) -------------------------------------------

/// The Fluent arguments a condition provides when it holds (for `any`, those
/// every alternative provides; `Condition::provides`).
pub fn condition_args(c: &Condition) -> BTreeSet<String> {
    c.provides().unwrap_or_default()
}

/// Whether a line's conditions read the conversation's subject (the
/// `about` slot, a subject `of: about`, an `about` condition that holds for
/// a subject): a line that answers what was asked about, not the topic in
/// general. Inside `any`, every alternative must read it.
pub fn reads_about(cs: &[Condition]) -> bool {
    fn one(c: &Condition) -> bool {
        use Condition as U;
        let subject = |s: &SubjectRef| match s {
            SubjectRef::HoodConcerns { of } => *of == HoodOf::About,
            SubjectRef::Person { slot } => slot == "about",
            SubjectRef::PolicySupport { of } => of == "about",
            _ => false,
        };
        match c {
            U::About { is, .. } => *is,
            U::Tells { subject: s, .. } | U::PlayerKnows { subject: s, .. } => subject(s),
            U::Occupation { slot, .. }
            | U::Trait { slot, .. }
            | U::Personality { slot, .. }
            | U::IsRival { slot, is: true }
            | U::IsBroker { slot, is: true } => slot == "about",
            U::Any { of } => !of.is_empty() && of.iter().all(one),
            _ => false,
        }
    }
    cs.iter().any(one)
}

/// The roles a fallback line covers (`None`: its conditions are not only on
/// the speaker's role; every role when it has no conditions).
fn fallback_roles(l: &TalkLineDef, all: &[String]) -> Option<Vec<String>> {
    let mut roles: Option<Vec<String>> = None;
    for c in &l.when {
        match c {
            Condition::Occupation { slot, occupations } if slot == "speaker" => {
                let next: Vec<String> = match roles {
                    None => occupations.clone(),
                    Some(r) => r.into_iter().filter(|x| occupations.contains(x)).collect(),
                };
                roles = Some(next);
            }
            _ => return None,
        }
    }
    Some(roles.unwrap_or_else(|| all.to_vec()))
}

fn subject_ok(s: &SubjectRef, about: &[AboutKind]) -> Result<(), String> {
    match s {
        SubjectRef::HoodConcerns { of: HoodOf::About }
            if !about
                .iter()
                .any(|k| matches!(k, AboutKind::Hood | AboutKind::Place)) =>
        {
            Err("`hood_concerns of about` needs a topic about a neighbourhood or a place".into())
        }
        SubjectRef::Person { slot } if !matches!(slot.as_str(), "speaker" | "about") => {
            Err(format!("unknown person slot `{slot}` (speaker, about)"))
        }
        SubjectRef::Person { slot } if slot == "about" && !about.iter().any(|k| k.is_person()) => {
            Err("the `about` slot needs a topic about a person or the rival".into())
        }
        SubjectRef::Proposal { sponsored_by } if sponsored_by != "speaker" => {
            Err(format!("unknown sponsor slot `{sponsored_by}` (speaker)"))
        }
        SubjectRef::PolicySupport { of } if of != "about" => {
            Err(format!("unknown policy slot `{of}` (about)"))
        }
        SubjectRef::PolicySupport { .. }
            if !about
                .iter()
                .any(|k| matches!(k, AboutKind::Bill | AboutKind::Template)) =>
        {
            Err("`policy_support` needs a topic about a bill or a template".into())
        }
        _ => Ok(()),
    }
}

/// Topics, lines, voices and the parameters (§3.11 items 1–7).
pub(crate) fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let t = &b.talk;
    let en = fluent_keys(&b.locale_text("en"));
    let vars = fluent_placeholders(&b.locale_text("en"));
    let roles = b.talk_roles();
    let key_exists = |owner: &str, key: &str, issues: &mut Vec<Issue>| {
        if !en.contains(key) {
            issues.push(Issue::error(
                owner,
                &format!("missing Fluent key `{key}` in locale en"),
            ));
        }
    };
    // Parameters.
    let d = &t.discretion;
    if d.bands.is_empty() {
        issues.push(Issue::error(
            "talk_params",
            "discretion needs at least one band",
        ));
    }
    for w in d.bands.windows(2) {
        if w[0].below >= w[1].below {
            issues.push(Issue::error(
                "talk_params",
                "discretion bands must rise (each `below` above the last)",
            ));
        }
        if w[0].tells > w[1].tells {
            issues.push(Issue::warning(
                "talk_params",
                "a warmer band should never tell less",
            ));
        }
    }
    if d.bands.last().is_some_and(|x| x.tells > d.top) {
        issues.push(Issue::warning(
            "talk_params",
            "the friendliest should tell the most (`top`)",
        ));
    }
    for k in t
        .trait_discretion
        .keys()
        .chain(t.flattered_discretion.keys())
    {
        if b.trait_def(k).is_none() {
            issues.push(Issue::error(
                "talk_params",
                &format!("discretion names unknown trait `{k}`"),
            ));
        }
    }
    if t.speaker_knows.is_empty() {
        issues.push(Issue::error(
            "talk_params",
            "speaker_knows needs at least one rule",
        ));
    }
    // Topics.
    for p in &b.talk_topics {
        if !crate::validate::valid_key(&p.id) || !p.id.starts_with("topic.") {
            issues.push(Issue::error(&p.id, "topic ids look like `topic.name`"));
        }
        key_exists(&p.id, &p.name_key, issues);
        if p.about_required && p.about.is_empty() {
            issues.push(Issue::error(
                &p.id,
                "`about_required` needs at least one `about` kind",
            ));
        }
        let allowed: &[AboutKind] = match p.mechanic {
            TalkMechanic::Inform => &[
                AboutKind::Place,
                AboutKind::Person,
                AboutKind::Hood,
                AboutKind::Bill,
                AboutKind::Template,
                AboutKind::Proposal,
                AboutKind::Rival,
            ],
            TalkMechanic::Pitch => &[AboutKind::Bill, AboutKind::Template, AboutKind::Proposal],
            TalkMechanic::AskSupport | TalkMechanic::Favour => &[AboutKind::Bill],
            TalkMechanic::SmallTalk => &[],
        };
        for k in &p.about {
            if !allowed.contains(k) {
                issues.push(Issue::error(
                    &p.id,
                    &format!(
                        "a `{}` topic cannot be about a {}",
                        p.mechanic.key(),
                        k.key()
                    ),
                ));
            }
        }
        if p.mechanic == TalkMechanic::Pitch && !p.about_required {
            issues.push(Issue::error(
                &p.id,
                "a pitch needs a subject (`about_required`)",
            ));
        }
        for r in &p.offered.roles {
            if !roles.contains(r) {
                issues.push(Issue::error(
                    &p.id,
                    &format!("topic offered to unknown role `{r}`"),
                ));
            }
        }
        for k in &p.offered.places {
            if b.place_kind(k).is_none() {
                issues.push(Issue::error(
                    &p.id,
                    &format!("topic offered at unknown place kind `{k}`"),
                ));
            }
        }
        crate::condition::check_all(
            b,
            &Site::new(&p.id, ConditionUse::TalkTopic, &["speaker", "player"]),
            &p.offered.when,
            issues,
        );
        let mut all = Vec::new();
        walk(&p.offered.when, &mut all);
        for c in all {
            if let Condition::Tells { subject, .. }
            | Condition::PlayerKnows { subject, .. } = c
                && let Err(e) = subject_ok(subject, &[])
            {
                issues.push(Issue::error(&p.id, &e));
            }
        }
    }
    // Voices.
    if !b.talk_voices.iter().any(|v| v.when.is_empty()) {
        issues.push(Issue::error(
            "talk_voices",
            "at least one voice needs no conditions (the fallback voice)",
        ));
    }
    for v in &b.talk_voices {
        if !crate::validate::valid_key(&v.id) || !v.id.starts_with("voice.") {
            issues.push(Issue::error(&v.id, "voice ids look like `voice.name`"));
        }
        if !(v.closer_share.is_finite() && (0.0..=1.0).contains(&v.closer_share)) {
            issues.push(Issue::error(&v.id, "closer_share must be in [0, 1]"));
        }
        let closers = b
            .talk_lines
            .iter()
            .filter(|l| l.part == TalkPart::Closer && l.voice.as_deref() == Some(v.id.as_str()))
            .count();
        if v.closer_share > 0.0 && closers < 2 {
            issues.push(Issue::error(
                &v.id,
                "a voice that closes needs at least two closers (so the guard has an alternative)",
            ));
        }
        crate::condition::check_all(
            b,
            &Site::new(&v.id, ConditionUse::TalkVoice, &["speaker", "player"]),
            &v.when,
            issues,
        );
        let mut all = Vec::new();
        walk(&v.when, &mut all);
        if all
            .iter()
            .any(|c| matches!(c, Condition::Voice { .. }))
        {
            issues.push(Issue::error(
                &v.id,
                "a voice's conditions cannot name a voice (it is what they decide)",
            ));
        }
    }
    // Talk names a place without a proper name by its kind
    // (`talk-place-<kind>`, a lower-case noun phrase: "the union hall").
    for p in &b.places {
        if !crate::types::PSEUDO_PLACES.contains(&p.key.as_str()) {
            key_exists(&p.id, &format!("talk-place-{}", p.key), issues);
        }
    }
    // Lines.
    for l in &b.talk_lines {
        let id = l.id.as_str();
        if !crate::validate::valid_key(id) || !id.starts_with("line.") {
            issues.push(Issue::error(id, "line ids look like `line.name`"));
        }
        if !(l.weight.is_finite() && l.weight > 0.0 && l.weight <= 10.0) {
            issues.push(Issue::error(id, "line weight must be in (0, 10]"));
        }
        let topic = l.topic.as_deref().and_then(|k| b.talk_topic(k));
        if let Some(k) = &l.topic
            && topic.is_none()
        {
            issues.push(Issue::error(id, &format!("unknown topic `{k}`")));
        }
        match l.part {
            TalkPart::Substance if l.topic.is_none() => {
                issues.push(Issue::error(id, "a substance line needs a topic"));
            }
            TalkPart::Closer => match &l.voice {
                None => issues.push(Issue::error(id, "a closer needs a voice")),
                Some(v) if b.talk_voice(v).is_none() => {
                    issues.push(Issue::error(id, &format!("unknown voice `{v}`")));
                }
                _ => {}
            },
            TalkPart::Quote
                if !l
                    .when
                    .iter()
                    .any(|c| matches!(c, Condition::Remembers { .. })) =>
            {
                issues.push(Issue::error(
                    id,
                    "a quote line needs a `remembers` condition",
                ));
            }
            _ => {}
        }
        if l.part != TalkPart::Closer && l.voice.is_some() {
            issues.push(Issue::error(id, "only closers have a voice"));
        }
        if l.part != TalkPart::Substance && l.topic.is_some() && l.part != TalkPart::Opener {
            issues.push(Issue::error(
                id,
                "only substance lines and openers have a topic",
            ));
        }
        if l.fallback && l.part != TalkPart::Substance {
            issues.push(Issue::error(id, "only substance lines are fallbacks"));
        }
        if l.fallback && fallback_roles(l, &roles).is_none() {
            issues.push(Issue::error(
                id,
                "a fallback line's only conditions are on the speaker's role (`occupation`, slot `speaker`)",
            ));
        }
        let about: &[AboutKind] = topic.map(|t| t.about.as_slice()).unwrap_or(&[]);
        let slots: Vec<&str> = if about.iter().any(|k| k.is_person()) {
            TALK_SLOTS.to_vec()
        } else {
            vec!["speaker", "player"]
        };
        crate::condition::check_all(
            b,
            &Site::new(id, ConditionUse::TalkLine, &slots),
            &l.when,
            issues,
        );
        let mut conds = Vec::new();
        walk(&l.when, &mut conds);
        for c in &conds {
            if let Condition::Tells { subject, .. }
            | Condition::PlayerKnows { subject, .. } = c
                && let Err(e) = subject_ok(subject, about)
            {
                issues.push(Issue::error(id, &e));
            }
        }
        // What it teaches: subjects valid for the topic, never past discretion.
        for x in &l.teaches {
            if let Err(e) = subject_ok(&x.subject, about) {
                issues.push(Issue::error(id, &e));
            }
            if x.level == Level::Unknown {
                issues.push(Issue::error(
                    id,
                    "a line teaches `heard`, `known` or `detailed`",
                ));
            }
            let guarded = l.when.iter().any(|c| {
                matches!(c, Condition::Tells { subject, min }
                    if subject == &x.subject && *min >= x.level)
            });
            if !guarded {
                issues.push(Issue::error(
                    id,
                    &format!(
                        "a line that teaches `{}` at `{}` needs a `tells` condition on it at that level or higher",
                        x.subject.key(),
                        x.level.key()
                    ),
                ));
            }
        }
        if let Some(lead) = &l.lead {
            key_exists(id, &lead.key, issues);
            if let Err(e) = subject_ok(&lead.subject, about) {
                issues.push(Issue::error(id, &e));
            }
            if lead.level == Level::Unknown {
                issues.push(Issue::error(
                    id,
                    "a lead gives `heard`, `known` or `detailed`",
                ));
            }
            match &lead.to {
                LeadToDef::PlaceKind { kind }
                    if b.place_kind(kind).is_none()
                        || crate::types::PSEUDO_PLACES.contains(&kind.as_str()) =>
                {
                    issues.push(Issue::error(
                        id,
                        &format!("lead to unknown place kind `{kind}`"),
                    ));
                }
                LeadToDef::Person { slot } if !matches!(slot.as_str(), "speaker" | "about") => {
                    issues.push(Issue::error(
                        id,
                        &format!("unknown lead slot `{slot}` (speaker, about)"),
                    ));
                }
                LeadToDef::Person { slot }
                    if slot == "about" && !about.iter().any(|k| k.is_person()) =>
                {
                    issues.push(Issue::error(
                        id,
                        "a lead to `about` needs a topic about a person or the rival",
                    ));
                }
                LeadToDef::Streets { of: HoodOf::About }
                    if !about
                        .iter()
                        .any(|k| matches!(k, AboutKind::Hood | AboutKind::Place)) =>
                {
                    issues.push(Issue::error(
                        id,
                        "a lead to the streets `of about` needs a topic about a neighbourhood or a place",
                    ));
                }
                _ => {}
            }
        }
        for e in &l.effects {
            match e {
                TalkEffectDef::Opinion { delta } if !(-20.0..=20.0).contains(delta) => {
                    issues.push(Issue::error(id, "an opinion effect must be within ±20"));
                }
                TalkEffectDef::Remember { kind }
                    if !crate::types::MEMORY_KINDS.contains(&kind.as_str()) =>
                {
                    issues.push(Issue::error(id, &format!("unknown memory kind `{kind}`")));
                }
                _ => {}
            }
        }
        // The Fluent key exists, and its placeholders are among the arguments.
        key_exists(id, &l.key, issues);
        let mut provided: BTreeSet<String> = CONTEXT_ARGS.iter().map(|x| x.to_string()).collect();
        if !about.is_empty() {
            provided.insert("about".into());
            provided.insert("about_key".into());
        }
        for c in &l.when {
            provided.extend(condition_args(c));
        }
        // An argument named `x_key` is a Fluent key the client renders as `$x`.
        let rendered: Vec<String> = provided
            .iter()
            .filter_map(|a| a.strip_suffix("_key").map(str::to_string))
            .collect();
        provided.extend(rendered);
        if let Some(used) = vars.get(&l.key) {
            for v in used.difference(&provided) {
                issues.push(Issue::error(
                    id,
                    &format!(
                        "Fluent key `{}` uses `${v}`, which this line's conditions do not provide",
                        l.key
                    ),
                ));
            }
        }
    }
    // Every topic has a fallback substance line for every role it is offered to.
    for p in &b.talk_topics {
        let offered: Vec<String> = if p.offered.roles.is_empty() {
            roles.clone()
        } else {
            p.offered.roles.clone()
        };
        let covered: BTreeSet<String> = b
            .talk_lines
            .iter()
            .filter(|l| {
                l.part == TalkPart::Substance && l.fallback && l.topic.as_deref() == Some(&p.id)
            })
            .filter_map(|l| fallback_roles(l, &roles))
            .flatten()
            .collect();
        let missing: Vec<&String> = offered.iter().filter(|r| !covered.contains(*r)).collect();
        if !missing.is_empty() {
            issues.push(Issue::error(
                &p.id,
                &format!(
                    "no fallback reply for {}",
                    missing
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::bundle::{Source, load};

    fn base() -> Vec<Source> {
        fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<Source>) {
            let mut v: Vec<_> = std::fs::read_dir(dir)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            v.sort();
            for p in v {
                if p.is_dir() {
                    walk(root, &p, out);
                } else if matches!(
                    p.extension().and_then(|x| x.to_str()),
                    Some("jsonc" | "ftl")
                ) {
                    out.push(Source {
                        path: p
                            .strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .replace('\\', "/"),
                        text: std::fs::read_to_string(&p).unwrap(),
                    });
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let mut out = Vec::new();
        walk(&root, &root, &mut out);
        out
    }

    /// Every issue after replacing `from` with `to` in a base file.
    fn issues_with(file: &str, from: &str, to: &str) -> String {
        let mut s = base();
        let f = s.iter_mut().find(|x| x.path == file).unwrap();
        assert!(f.text.contains(from), "{from}");
        f.text = f.text.replacen(from, to, 1);
        let all = match load(&s) {
            Ok((_, w)) => w,
            Err(e) => e,
        };
        all.iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    const LINES: &str = "base/conversations/lines.jsonc";

    #[test]
    fn the_base_conversations_load() {
        let (b, w) = load(&base()).unwrap();
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(b.talk_topics.len(), 7);
        let mechanics: BTreeSet<TalkMechanic> = b.talk_topics.iter().map(|t| t.mechanic).collect();
        assert_eq!(mechanics.len(), 5, "every mechanic has a topic");
        assert!(
            b.talk_voice("voice.plain")
                .is_some_and(|v| v.when.is_empty())
        );
        assert_eq!(b.talk.cost, TalkCost::HalfSlot);
        assert_eq!(b.talk.trait_discretion.get("trait.schemer"), Some(&-1));
        assert_eq!(b.talk.trait_discretion.get("trait.cautious"), Some(&-1));
        assert_eq!(b.talk.flattered_discretion.get("trait.vain"), Some(&1));
        assert_eq!(b.talk_roles().len(), b.occupations.len() + 2);
    }

    /// The follow-ups' rules: `voice` names real voices and never stands in
    /// a voice's own conditions; `concern` names real concerns; every place
    /// kind has a talk name; concern talk keys exist.
    #[test]
    fn voice_concern_and_place_names_are_validated() {
        const VOICES: &str = "base/conversations/voices.jsonc";
        let e = issues_with(
            LINES,
            r#""voices": ["voice.blunt"]"#,
            r#""voices": ["voice.nope"]"#,
        );
        assert!(e.contains("unknown voice `voice.nope`"), "{e}");
        let e = issues_with(
            VOICES,
            r#"{ "id": "voice.plain", "priority": 0, "closer_share": 0.3 }"#,
            r#"{ "id": "voice.plain", "priority": 0, "closer_share": 0.3,
                 "when": [ { "type": "voice", "voices": ["voice.blunt"] } ] }"#,
        );
        assert!(e.contains("cannot name a voice"), "{e}");
        let e = issues_with(
            LINES,
            r#""concerns": ["concern.housing"]"#,
            r#""concerns": ["concern.nope"]"#,
        );
        assert!(e.contains("unknown concern `concern.nope`"), "{e}");
        let e = issues_with(
            "base/locale/en/talk.ftl",
            "talk-place-union_hall = the union hall\n",
            "",
        );
        assert!(e.contains("talk-place-union_hall"), "{e}");
        let e = issues_with(
            "base/locale/en/base.ftl",
            "talk-concern-housing = the rent\n",
            "",
        );
        assert!(e.contains("talk-concern-housing"), "{e}");
    }

    #[test]
    fn validation_holds_the_conversation_rules() {
        // Item 2: a fallback for every role.
        let e = issues_with(
            LINES,
            r#""key": "talk-small-weather", "fallback": true },"#,
            r#""key": "talk-small-weather" },"#,
        );
        assert!(
            e.is_empty(),
            "the other fallback still covers everyone: {e}"
        );
        let e = issues_with(
            LINES,
            r#""key": "talk-small-family", "fallback": true },"#,
            r#""key": "talk-small-family", "fallback": true,
               "when": [ { "type": "occupation", "slot": "speaker", "occupations": ["occupation.nurse"] } ] },"#,
        );
        assert!(e.is_empty(), "{e}");
        let mut s = base();
        let f = s.iter_mut().find(|x| x.path == LINES).unwrap();
        f.text = f
            .text
            .replacen(r#""key": "talk-small-weather", "fallback": true"#, r#""key": "talk-small-weather""#, 1)
            .replacen(
                r#""key": "talk-small-family", "fallback": true },"#,
                r#""key": "talk-small-family", "fallback": true,
                   "when": [ { "type": "occupation", "slot": "speaker", "occupations": ["occupation.nurse"] } ] },"#,
                1,
            );
        let e = load(&s).err().unwrap();
        let e = e
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            e.contains("topic.small_talk: no fallback reply for occupation.union_steward"),
            "{e}"
        );
        // A fallback's only conditions are on the role.
        let e = issues_with(
            LINES,
            r#""key": "talk-pitch-listen", "fallback": true },"#,
            r#""key": "talk-pitch-listen", "fallback": true,
               "when": [ { "type": "opinion", "slot": "speaker", "min": 0 } ] },"#,
        );
        assert!(
            e.contains("a fallback line's only conditions are on the speaker's role"),
            "{e}"
        );
        // Item 4: discretion is never bypassed.
        let e = issues_with(
            LINES,
            r#""teaches": [ { "subject": { "type": "broker" }, "level": "heard" } ] },"#,
            r#""teaches": [ { "subject": { "type": "broker" }, "level": "known" } ] },"#,
        );
        assert!(
            e.contains("needs a `tells` condition on it at that level or higher"),
            "{e}"
        );
        let e = issues_with(
            LINES,
            r#""key": "talk-worries-shrug", "fallback": true },"#,
            r#""key": "talk-worries-shrug", "fallback": true,
               "teaches": [ { "subject": { "type": "person", "slot": "about" }, "level": "heard" } ] },"#,
        );
        assert!(
            e.contains("the `about` slot needs a topic about a person"),
            "{e}"
        );
        // Item 5: placeholders come from the conditions.
        let e = issues_with(
            "base/locale/en/talk.ftl",
            r#"talk-pitch-listen = "Go on, then. I'm listening.""#,
            r#"talk-pitch-listen = "Go on, { $broker }. I'm listening.""#,
        );
        assert!(
            e.contains("uses `$broker`, which this line's conditions do not provide"),
            "{e}"
        );
        let e = issues_with(
            "base/locale/en/talk.ftl",
            "talk-runs-self =",
            "talk-runs-self-x =",
        );
        assert!(e.contains("missing Fluent key `talk-runs-self`"), "{e}");
        // Item 6: a closing voice has an alternative.
        let e = issues_with(
            LINES,
            r#""voice": "voice.blunt", "key": "talk-close-blunt-2" },"#,
            r#""voice": "voice.plain", "key": "talk-close-blunt-2" },"#,
        );
        assert!(
            e.contains("voice.blunt: a voice that closes needs at least two closers"),
            "{e}"
        );
        // Item 1 and 3: real topics, voices, slots, roles, traits.
        let e = issues_with(
            LINES,
            r#""topic": "topic.small_talk","#,
            r#""topic": "topic.chat","#,
        );
        assert!(e.contains("unknown topic `topic.chat`"), "{e}");
        let e = issues_with(
            "base/conversations/voices.jsonc",
            r#""traits": ["trait.cautious"]"#,
            r#""traits": ["trait.sly"]"#,
        );
        assert!(e.contains("unknown trait `trait.sly`"), "{e}");
        let e = issues_with(
            LINES,
            r#"{ "type": "remembers", "slot": "speaker", "kinds": ["broke_promise"] }"#,
            r#"{ "type": "remembers", "slot": "here", "kinds": ["broke_promise"] }"#,
        );
        assert!(e.contains("unknown slot `here`"), "{e}");
        let e = issues_with(
            "base/conversations/topics.jsonc",
            r#""roles": ["councillor", "mayor"]"#,
            r#""roles": ["councillor", "pope"]"#,
        );
        assert!(e.contains("topic offered to unknown role `pope`"), "{e}");
        // Item 7: weights and shares.
        let e = issues_with(LINES, r#""weight": 5,"#, r#""weight": 50,"#);
        assert!(e.contains("line weight must be in (0, 10]"), "{e}");
        let e = issues_with(
            "base/conversations/params.jsonc",
            r#""quote_share": 0.5"#,
            r#""quote_share": 1.5"#,
        );
        assert!(e.contains("talk.quote_share = 1.5 outside [0, 1]"), "{e}");
    }

    #[test]
    fn slot_conditions_need_a_conversation() {
        let e = issues_with(
            "base/policies/situational.jsonc",
            r#""any": ["#,
            r#""any": [ { "type": "trait", "slot": "speaker", "traits": ["trait.vain"] },"#,
        );
        assert!(e.contains("this condition reads a slot"), "{e}");
        // The player's ledger never decides whether something happens (DEC-091).
        let e = issues_with(
            "base/policies/situational.jsonc",
            r#""any": ["#,
            r#""any": [ { "type": "any", "of": [ { "type": "player_knows", "subject": { "type": "broker" }, "min": "known" } ] },"#,
        );
        assert!(e.contains("never decides whether something happens"), "{e}");
    }

    #[test]
    fn placeholders_by_message() {
        let v = fluent_placeholders(
            "# c\na = Hi { $speaker }\n    and { $town }\nb = { $n ->\n    [one] one\n   *[other] { $n } of { $who }\n}\nc = none\n",
        );
        assert_eq!(
            v["a"],
            ["speaker", "town"].iter().map(|s| s.to_string()).collect()
        );
        assert_eq!(v["b"], ["n", "who"].iter().map(|s| s.to_string()).collect());
        assert!(v["c"].is_empty());
    }
}
