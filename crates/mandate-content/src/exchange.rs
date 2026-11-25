//! Exchange scenes (DEC-112; `docs/design/EXCHANGE_SCENES.md`): the eighth
//! beat kind of a staged arc. A played scene of a few rounds whose prompts
//! and lines are generated from live state.
//!
//! Two things live here:
//!
//! - [`ArcExchangeDef`]: the scene as data on an arc's stage (who is across
//!   from the player, who watches, which sources may prompt each round,
//!   which kinds of line are allowed, what its outcomes do to the run);
//! - [`ExchangeParams`]: every constant of the mechanism (content kind
//!   `exchange_params`, `content/base/exchange.jsonc`), with overrides by
//!   tier and by scene. **All of them are placeholders** until the tuning
//!   pass at the end of M6.
//!
//! The types are `ArcExchange…` because `Exchange` already means one
//! logged conversation (`mandate_sim::talk`). The engine is
//! `mandate_sim::arcs::exchange`.
//!
//! Nothing here names an office, a body, a ward, a market or an industry
//! (DEC-098): an audience is a *source* the engine binds to what the
//! jurisdiction has, a member is found by a power, and a worry's readings
//! are a table in content.

use crate::arcs::OptionOutcome;
use crate::types::{ContentKey, Power, TierKey};
use crate::visits::VisitTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// --- The scene ---------------------------------------------------------------------------

/// How the scene is laid out on screen (presentation only).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// A strip of groups (a debate, a public meeting).
    #[default]
    Crowd,
    /// A row of faces with a count (a hearing).
    Members,
    /// Two short lists: what is asked and what is offered (a negotiation).
    Table,
}

/// Who is across from the player, who may chair, who may speak for them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SidesDef {
    /// A person role.
    pub other: String,
    /// A person role that may ask a round's question.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chair: Option<String>,
    /// A person role that may speak for the player (the `second` line).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub second: Option<String>,
}

/// Where an audience's groups come from (§2.2). The engine binds each to
/// live state; none names a place or a body.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AudienceFrom {
    /// The areas one level down from where the scene is.
    Areas,
    /// Income classes inside one area.
    Classes,
    /// The communities present in an area.
    Communities,
    /// Where a cast place's workers live (until plants give it: the area).
    Workers,
    /// The holders of posts with a power, one group each.
    Members,
    /// One cast person.
    Person,
}

impl AudienceFrom {
    /// Cohorts (approval and recognition) rather than characters.
    pub fn crowd(self) -> bool {
        matches!(
            self,
            AudienceFrom::Areas
                | AudienceFrom::Classes
                | AudienceFrom::Communities
                | AudienceFrom::Workers
        )
    }

    pub fn key(self) -> &'static str {
        match self {
            AudienceFrom::Areas => "areas",
            AudienceFrom::Classes => "classes",
            AudienceFrom::Communities => "communities",
            AudienceFrom::Workers => "workers",
            AudienceFrom::Members => "members",
            AudienceFrom::Person => "person",
        }
    }
}

/// `of`: the race the player is standing in (its constituency).
pub const OF_RACE: &str = "race";
/// `of`: the run's own jurisdiction.
pub const OF_HERE: &str = "here";

/// How a crowd's groups are weighed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AudienceWeight {
    /// By voters (`count × turnout`).
    #[default]
    Voters,
    /// One each.
    Even,
}

/// Where a group's lean starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AudienceStart {
    /// A crowd's approval of the player among those who know them.
    #[default]
    Approval,
    /// A member's support: opinion, a shared party, a favour owed.
    Support,
    /// A person's opinion of the player.
    Opinion,
    /// A person's opinion less what they are demanding (they start against).
    Demand,
}

/// One source of audience groups.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AudienceDef {
    pub from: AudienceFrom,
    /// For crowds: [`OF_RACE`], [`OF_HERE`] (the default), or an area or
    /// place role of the arc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
    /// For `person`: the cast role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// For `members`: the power their posts list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power: Option<Power>,
    /// At most this many groups (never more than five in all).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u8>,
    #[serde(default)]
    pub weight: AudienceWeight,
    #[serde(default)]
    pub start: AudienceStart,
}

/// Where a prompt comes from (§4.1): code that reads one owner's state and
/// gives zero or more true things to put to the player.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    BrokenPromise,
    VoteAgainstLean,
    FailedLaw,
    ThinRecord,
    Absent,
    Scandal,
    HiddenFact,
    GoverningRecord,
    TheMatter,
    WhoPays,
    Demand,
    DebtRecord,
    Bind,
    Terms,
    TheFact,
    Contradiction,
    WhoElse,
    EvidenceHeld,
    Fitness,
    TopWorry,
}

impl SourceKind {
    pub const ALL: [SourceKind; 20] = [
        SourceKind::BrokenPromise,
        SourceKind::VoteAgainstLean,
        SourceKind::FailedLaw,
        SourceKind::ThinRecord,
        SourceKind::Absent,
        SourceKind::Scandal,
        SourceKind::HiddenFact,
        SourceKind::GoverningRecord,
        SourceKind::TheMatter,
        SourceKind::WhoPays,
        SourceKind::Demand,
        SourceKind::DebtRecord,
        SourceKind::Bind,
        SourceKind::Terms,
        SourceKind::TheFact,
        SourceKind::Contradiction,
        SourceKind::WhoElse,
        SourceKind::EvidenceHeld,
        SourceKind::Fitness,
        SourceKind::TopWorry,
    ];

    pub fn key(self) -> &'static str {
        match self {
            SourceKind::BrokenPromise => "broken_promise",
            SourceKind::VoteAgainstLean => "vote_against_lean",
            SourceKind::FailedLaw => "failed_law",
            SourceKind::ThinRecord => "thin_record",
            SourceKind::Absent => "absent",
            SourceKind::Scandal => "scandal",
            SourceKind::HiddenFact => "hidden_fact",
            SourceKind::GoverningRecord => "governing_record",
            SourceKind::TheMatter => "the_matter",
            SourceKind::WhoPays => "who_pays",
            SourceKind::Demand => "demand",
            SourceKind::DebtRecord => "debt_record",
            SourceKind::Bind => "bind",
            SourceKind::Terms => "terms",
            SourceKind::TheFact => "the_fact",
            SourceKind::Contradiction => "contradiction",
            SourceKind::WhoElse => "who_else",
            SourceKind::EvidenceHeld => "evidence_held",
            SourceKind::Fitness => "fitness",
            SourceKind::TopWorry => "top_worry",
        }
    }

    /// It always gives a prompt (a round that ends in one never falls
    /// silent): the room's question, the matter itself.
    pub fn always(self) -> bool {
        matches!(self, SourceKind::TopWorry | SourceKind::TheMatter)
    }

    /// A fault of the player's own: the `own` line answers it.
    pub fn fault(self) -> bool {
        matches!(
            self,
            SourceKind::BrokenPromise
                | SourceKind::VoteAgainstLean
                | SourceKind::FailedLaw
                | SourceKind::ThinRecord
                | SourceKind::Absent
                | SourceKind::Scandal
                | SourceKind::HiddenFact
                | SourceKind::TheFact
                | SourceKind::Contradiction
        )
    }

    /// About a fact that may not be proved: the `deny` line answers it.
    pub fn deniable(self) -> bool {
        matches!(
            self,
            SourceKind::Scandal | SourceKind::HiddenFact | SourceKind::TheFact
        )
    }

    /// Personal: a truce bars these, and a grudge reaches for them.
    pub fn personal(self) -> bool {
        matches!(
            self,
            SourceKind::Scandal | SourceKind::HiddenFact | SourceKind::Absent
        )
    }

    /// The engine builds it (the debate's nine). A scene that lists
    /// another is parsed and validated, and its arc is never raised.
    pub fn built(self) -> bool {
        matches!(
            self,
            SourceKind::BrokenPromise
                | SourceKind::VoteAgainstLean
                | SourceKind::FailedLaw
                | SourceKind::ThinRecord
                | SourceKind::Absent
                | SourceKind::Scandal
                | SourceKind::HiddenFact
                | SourceKind::GoverningRecord
                | SourceKind::TopWorry
        )
    }
}

/// The kinds of line (§3.1), in the order the list shows them.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MoveKind {
    Record,
    Heard,
    Turn,
    Promise,
    Act,
    Own,
    Deny,
    Offer,
    Second,
    Proof,
    Pass,
}

impl MoveKind {
    pub const ALL: [MoveKind; 11] = [
        MoveKind::Record,
        MoveKind::Heard,
        MoveKind::Turn,
        MoveKind::Promise,
        MoveKind::Act,
        MoveKind::Own,
        MoveKind::Deny,
        MoveKind::Offer,
        MoveKind::Second,
        MoveKind::Proof,
        MoveKind::Pass,
    ];

    pub fn key(self) -> &'static str {
        match self {
            MoveKind::Record => "record",
            MoveKind::Heard => "heard",
            MoveKind::Turn => "turn",
            MoveKind::Promise => "promise",
            MoveKind::Act => "act",
            MoveKind::Own => "own",
            MoveKind::Deny => "deny",
            MoveKind::Offer => "offer",
            MoveKind::Second => "second",
            MoveKind::Proof => "proof",
            MoveKind::Pass => "pass",
        }
    }

    pub fn from_key(k: &str) -> Option<MoveKind> {
        MoveKind::ALL.into_iter().find(|m| m.key() == k)
    }

    /// What real state the kind needs before it gives a line (§3.1): the
    /// table's "Requires" column, as the lint and the locked line name it.
    /// `None` only for the safe move.
    pub fn requirement(self) -> Option<&'static str> {
        Some(match self {
            MoveKind::Record => "a law in force the player sponsored or declared for",
            MoveKind::Heard => "a group's worries in the ledger",
            MoveKind::Turn => "a fault of the other side, and the other side known",
            MoveKind::Promise => "a promise on the campaign's menu",
            MoveKind::Act => "a power to draft and a template that answers the subject",
            MoveKind::Own => "a prompt that is a fault of the player's own",
            MoveKind::Deny => "a prompt about a fact that is not proved",
            MoveKind::Offer => "something real to give",
            MoveKind::Second => "a backer who agreed to speak",
            MoveKind::Proof => "something found",
            MoveKind::Pass => return None,
        })
    }

    /// The other side may hold an answer to it (§4.3).
    pub fn answerable(self) -> bool {
        matches!(
            self,
            MoveKind::Record
                | MoveKind::Promise
                | MoveKind::Turn
                | MoveKind::Act
                | MoveKind::Second
        )
    }

    /// It answers any prompt it is allowed on, whatever its subject.
    pub fn always_answers(self) -> bool {
        matches!(self, MoveKind::Own | MoveKind::Deny | MoveKind::Pass)
    }

    /// The mark a line of this kind leaves on the run (§5.5).
    pub fn mark(self) -> Option<&'static str> {
        match self {
            MoveKind::Promise => Some("promised"),
            MoveKind::Turn => Some("aired"),
            MoveKind::Own => Some("owned"),
            MoveKind::Deny => Some("denied"),
            _ => None,
        }
    }
}

/// Who puts a round's prompt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Speaker {
    #[default]
    Other,
    Chair,
}

/// One round: who speaks and the sources they may draw on, best first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoundDef {
    /// From 1.
    pub round: u8,
    #[serde(default)]
    pub by: Speaker,
    pub from: Vec<SourceKind>,
}

/// What a `prep` mark removes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrepRemoves {
    /// The first round is not cold: the player knew what was coming.
    Cold,
}

/// A mark from an earlier beat that adds a line or removes a risk (§3.4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrepDef {
    pub mark: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removes: Option<PrepRemoves>,
    /// A kind of line the mark gives (`second`, `proof`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adds: Option<MoveKind>,
    /// Sources the other side keeps off while the mark is set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bars: Vec<SourceKind>,
    /// For `adds: proof`: the evidence what was found carries (0..1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<f32>,
}

/// What a room's temper reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TemperReads {
    /// The run's own heat.
    #[default]
    Heat,
}

/// A room's anger scales every gain down; some lines cool it (§2.4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TemperDef {
    #[serde(default)]
    pub reads: TemperReads,
    /// Gains land at `1 − drag × heat`.
    pub drag: f32,
}

/// How many members must end with the player.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CountNeeds {
    /// More than half.
    Majority,
    /// At least two thirds.
    TwoThirds,
}

impl CountNeeds {
    /// Whether `n` of `of` is enough.
    pub fn met(self, n: usize, of: usize) -> bool {
        match self {
            CountNeeds::Majority => n * 2 > of,
            CountNeeds::TwoThirds => of > 0 && n * 3 >= of * 2,
        }
    }
}

/// The result is a count of heads, not a mean (§2.4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CountDef {
    pub needs: CountNeeds,
}

/// Something the other side asks for across a table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TableWant {
    /// The player's vote on the bill cast in a role.
    Vote { role: String, vote: TableVote },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TableVote {
    Yes,
    No,
}

/// Who hears how a negotiation ended.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WatchersDef {
    pub from: AudienceFrom,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power: Option<Power>,
    /// `owed_or_owing`: those who owe the player or are owed by them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub who: Option<String>,
}

/// Offers wait on the table and take effect only if a deal is struck
/// (§2.4). **Not built yet** (the negotiation arrives with the favour's
/// arc): parsed and validated, and an arc that uses it is never raised.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TableDef {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wants: Vec<TableWant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watchers: Option<WatchersDef>,
}

/// How a scene ends.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Won,
    Split,
    Lost,
    /// The player never began before its day passed: an empty chair.
    Absent,
}

impl Outcome {
    pub const ALL: [Outcome; 4] = [Outcome::Won, Outcome::Split, Outcome::Lost, Outcome::Absent];

    pub fn key(self) -> &'static str {
        match self {
            Outcome::Won => "won",
            Outcome::Split => "split",
            Outcome::Lost => "lost",
            Outcome::Absent => "absent",
        }
    }

    pub fn from_key(k: &str) -> Option<Outcome> {
        Outcome::ALL.into_iter().find(|o| o.key() == k)
    }
}

/// The exchange beat (§2.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArcExchangeDef {
    /// Where it is held: a place role.
    pub at: String,
    /// What taking part costs, for the whole scene.
    #[serde(default)]
    pub time: VisitTime,
    pub sides: SidesDef,
    #[serde(default)]
    pub layout: Layout,
    /// One to five groups in all.
    pub audience: Vec<AudienceDef>,
    /// Rounds (default: `exchange_params.rounds`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rounds: Option<u8>,
    pub prompts: Vec<RoundDef>,
    pub moves: Vec<MoveKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prep: Vec<PrepDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temper: Option<TemperDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<CountDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<TableDef>,
    /// What each end state does to the run (marks, heat, gated effects).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub outcome: BTreeMap<Outcome, OptionOutcome>,
    /// The key prefix of its sentences (`exchange-debate`).
    pub text: String,
    /// Constants this scene sets for itself: dotted paths into
    /// [`ExchangeParams`] (`"win": 0.1`, `"cover.own": 0.5`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, f64>,
}

impl ArcExchangeDef {
    /// The rounds it runs (its own, else the shared block's).
    pub fn rounds(&self, p: &ExchangeParams) -> u8 {
        self.rounds.unwrap_or(p.rounds).max(1)
    }

    pub fn round(&self, n: u8) -> Option<&RoundDef> {
        self.prompts.iter().find(|r| r.round == n)
    }

    /// Every person role it names.
    pub fn person_roles(&self) -> Vec<&str> {
        let mut out = vec![self.sides.other.as_str()];
        out.extend(self.sides.chair.as_deref());
        out.extend(self.sides.second.as_deref());
        for a in &self.audience {
            if a.from == AudienceFrom::Person {
                out.extend(a.role.as_deref());
            }
        }
        out
    }

    /// The parts the engine does not do yet (the arc is never raised).
    pub fn not_yet(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.table.is_some() {
            out.push(
                "a `table` (the negotiation's conditional offers: with the favour's arc)".into(),
            );
        }
        for r in &self.prompts {
            for s in &r.from {
                if !s.built() {
                    out.push(format!(
                        "prompt source `{}` (with the scene that needs it)",
                        s.key()
                    ));
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }
}

// --- The constants -----------------------------------------------------------------------

/// How much a group cares about a subject, by the worry's rank there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FitParams {
    pub top: f64,
    pub next: f64,
    pub low: f64,
    /// A subject that is no worry (a thin record, a promise to turn up).
    pub flat: f64,
}

/// A number for each kind of line that has one (missing: 0).
pub type ByKind = BTreeMap<MoveKind, f64>;

/// The chance a claim stands up, by how well it is known.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HoldParams {
    pub heard: f64,
    pub known: f64,
    pub detailed: f64,
}

/// An attack's size, by source, where it is not read from state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SizeParams {
    /// A broken promise.
    pub promise: f64,
    /// The room's question: not an attack.
    pub question: f64,
    /// A beat never taken, a decision let pass (each).
    pub absent: f64,
    /// The record under which "thin" is an attack.
    pub thin_line: f64,
    /// A law whose ledger delivers less than this is "failed".
    pub failed_line: f64,
    /// How far a group must lean against a vote before it is an attack.
    pub lean_line: f64,
    /// The government's approval under which its record is an attack.
    pub governing_line: f64,
    /// Days over which a broken promise fades to half.
    pub recency_days: u32,
    /// How grave a hidden fact is, by its severity.
    pub gravity_ethics: f64,
    pub gravity_misconduct: f64,
    pub gravity_crime: f64,
}

/// The other side's deck (§4.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeckParams {
    /// Attacks held: `base + round(per_skill × skill)`.
    pub base: f64,
    pub per_skill: f64,
    /// Answers a scene: `answers_base + round(answers_per_skill × oratory)`.
    pub answers_base: f64,
    pub answers_per_skill: f64,
    /// Skill at or over which they read how residents feel, not what a
    /// statistic shows, and always pick their best.
    pub sharp: f64,
    /// A weaker opponent picks among their best this many.
    pub pick_top: u8,
    /// A hidden fact held on evidence under this is kept out by integrity
    /// at or over `integrity_bar`, and used only by boldness at or over
    /// `bold_bar`.
    pub weak_evidence: f64,
    pub integrity_bar: f64,
    pub bold_bar: f64,
    /// Weight added to the personal sources: × boldness.
    pub bold_personal: f64,
    /// Weight added to the personal sources by a grudge (a trait in
    /// `grudge_traits`, or a memory of the player against them).
    pub grudge_personal: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grudge_traits: Vec<ContentKey>,
    /// Weight added to the room's question: × warmth.
    pub warm_question: f64,
}

/// A worry's reading in a law's ledger, and the amount that answers it in
/// full (§3.1, `record`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LedgerReading {
    /// A ledger key, or a prefix ending in `_` (`jobs_added_`).
    pub key: String,
    /// The amount that is a full answer; with `per_thousand`, for every
    /// thousand residents of the law's jurisdiction.
    pub full: f64,
    #[serde(default)]
    pub per_thousand: bool,
}

/// How a law's ledger speaks to one worry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorryReadings {
    pub concern: ContentKey,
    pub readings: Vec<LedgerReading>,
}

/// How a member or a person cares (§5.3): a base, and weights on who they
/// are. The result is kept in `[min, max]`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemberFit {
    pub base: f64,
    pub min: f64,
    pub max: f64,
    /// × integrity, by kind (`own` up, `deny` down).
    #[serde(default)]
    pub integrity: ByKind,
    /// × loyalty, for one of the player's party, by kind.
    #[serde(default)]
    pub loyalty: ByKind,
    /// Added when they remember something good of the player; taken when
    /// something bad.
    pub memory: f64,
    /// A subject that matches one of their goals counts as this.
    pub goal: f64,
    /// A member's starting lean: opinion ÷ 100, plus these.
    pub party_step: f64,
    pub favour_step: f64,
    /// What a `demand` takes from a person's starting lean.
    pub demand: f64,
}

/// A room's temper (§2.4): what cools it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TemperParams {
    pub cool_own: f64,
    pub cool_stay: f64,
    pub cool_heard: f64,
}

/// What the scene does in the world (§5.5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorldParams {
    /// Approval in a crowd group: × its change in lean.
    pub approve: f64,
    /// Recognition in each crowd group, win or lose.
    pub aware: f64,
    /// A member's or a person's opinion: × their change in lean.
    pub opinion: f64,
    /// A change in lean at or over this is remembered.
    pub memory_at: f64,
    /// Standing an admission costs.
    pub own_standing: f64,
    /// The backer's opinion when a scene they spoke in is lost.
    pub second_lost: f64,
    /// The other side's opinion when a `turn` is used on them.
    pub pressed: f64,
    /// The paper's story: its weight in the media's own perception rule
    /// when the result is at a threshold (`win`, `lose`), less under it.
    pub story_weight: f64,
}

/// Every constant of the exchange (§5.6). Placeholders, all of them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExchangeParams {
    pub rounds: u8,
    /// How hard any attack bites.
    pub bite: f64,
    /// How much any line gains.
    pub gain: f64,
    /// Round 1's bite when the player did not know what was coming.
    pub cold: f64,
    pub fit: FitParams,
    /// What a targeted line gives the groups it did not name.
    pub spill: f64,
    pub size: SizeParams,
    /// A public fault counts at this: people have heard it.
    pub public_share: f64,
    /// A law the player only voted for counts at this.
    pub voted_share: f64,
    pub second_base: f64,
    pub act_base: f64,
    /// Added to `act` when the body would back it.
    pub act_votes: f64,
    /// A worry learned only second-hand lands at this share.
    pub heard_floor: f64,
    /// Visits to a place that make `heard` land in full.
    pub visits_full: f64,
    /// How long a visit counts (the ground game's `keep` a year).
    pub visits_keep: f64,
    /// Staying for every question.
    pub stay_strength: f64,
    /// How far a kind covers a prompt it answers.
    pub cover: ByKind,
    /// What a line that does not stand costs.
    pub backfire: ByKind,
    pub hold: HoldParams,
    /// A kind's second use.
    pub repeat: f64,
    pub win: f64,
    pub lose: f64,
    pub group_margin: f64,
    /// A member is with the player at or over this lean.
    pub member_line: f64,
    /// A table is lost under this lean.
    pub walk_away: f64,
    /// The size of the other side's answer, by the kind it answers.
    pub answer: ByKind,
    pub voice_base: f64,
    pub voice_span: f64,
    pub list_max: u8,
    pub prep_pins: u8,
    /// Wordings written for each prompt and each line of the player's.
    pub prompt_variants: u8,
    pub say_variants: u8,
    pub deck: DeckParams,
    pub member_fit: MemberFit,
    pub temper: TemperParams,
    pub world: WorldParams,
    /// How a law's ledger speaks to each worry.
    pub worries: Vec<WorryReadings>,
    /// A bill is about the worry whose attitudes it lines up with at least
    /// this much (else it is no worry: `fit.flat`).
    pub worry_match: f64,
    /// Overrides by tier (DEC-098): dotted paths, as a scene's `params`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tiers: BTreeMap<TierKey, BTreeMap<String, f64>>,
}

impl Default for ExchangeParams {
    /// The design's first values (§5.6); the base pack states them again in
    /// `exchange.jsonc`, where they are tuned.
    fn default() -> Self {
        let by = |v: &[(MoveKind, f64)]| v.iter().copied().collect::<ByKind>();
        ExchangeParams {
            rounds: 3,
            bite: 0.30,
            gain: 0.25,
            cold: 1.25,
            fit: FitParams {
                top: 1.0,
                next: 0.6,
                low: 0.25,
                flat: 0.6,
            },
            spill: 0.3,
            size: SizeParams {
                promise: 0.6,
                question: 0.3,
                absent: 0.3,
                thin_line: 0.25,
                failed_line: 0.25,
                lean_line: 0.15,
                governing_line: 0.5,
                recency_days: 730,
                gravity_ethics: 0.4,
                gravity_misconduct: 0.7,
                gravity_crime: 1.0,
            },
            public_share: 0.6,
            voted_share: 0.5,
            second_base: 0.6,
            act_base: 0.5,
            act_votes: 0.2,
            heard_floor: 0.5,
            visits_full: 3.0,
            visits_keep: 0.5,
            stay_strength: 0.5,
            cover: by(&[
                (MoveKind::Record, 0.8),
                (MoveKind::Proof, 0.9),
                (MoveKind::Own, 0.6),
                (MoveKind::Act, 0.6),
                (MoveKind::Promise, 0.5),
                (MoveKind::Heard, 0.4),
                (MoveKind::Second, 0.4),
                (MoveKind::Turn, 0.3),
                (MoveKind::Pass, 0.2),
                (MoveKind::Deny, 1.0),
                (MoveKind::Offer, 0.4),
            ]),
            backfire: by(&[
                (MoveKind::Turn, 0.25),
                (MoveKind::Proof, 0.15),
                (MoveKind::Deny, 0.25),
                (MoveKind::Heard, 0.10),
            ]),
            hold: HoldParams {
                heard: 0.4,
                known: 0.75,
                detailed: 1.0,
            },
            repeat: 0.6,
            win: 0.08,
            lose: 0.08,
            group_margin: 0.05,
            member_line: 0.2,
            walk_away: -0.5,
            answer: by(&[
                (MoveKind::Promise, 0.5),
                (MoveKind::Turn, 0.4),
                (MoveKind::Act, 0.4),
                (MoveKind::Record, 0.4),
                (MoveKind::Second, 0.3),
            ]),
            voice_base: 0.8,
            voice_span: 0.4,
            list_max: 5,
            prep_pins: 2,
            prompt_variants: 3,
            say_variants: 2,
            deck: DeckParams {
                base: 2.0,
                per_skill: 4.0,
                answers_base: 1.0,
                answers_per_skill: 2.0,
                sharp: 0.6,
                pick_top: 2,
                weak_evidence: 0.5,
                integrity_bar: 0.6,
                bold_bar: 0.6,
                bold_personal: 0.3,
                grudge_personal: 0.3,
                grudge_traits: Vec::new(),
                warm_question: 0.2,
            },
            member_fit: MemberFit {
                base: 0.6,
                min: 0.1,
                max: 1.2,
                integrity: ByKind::new(),
                loyalty: ByKind::new(),
                memory: 0.2,
                goal: 1.0,
                party_step: 0.2,
                favour_step: 0.2,
                demand: 1.0,
            },
            temper: TemperParams {
                cool_own: 0.15,
                cool_stay: 0.20,
                cool_heard: 0.05,
            },
            world: WorldParams {
                approve: 0.08,
                aware: 0.04,
                opinion: 20.0,
                memory_at: 0.15,
                own_standing: -1.0,
                second_lost: -5.0,
                pressed: -4.0,
                story_weight: 0.5,
            },
            worries: Vec::new(),
            worry_match: 0.3,
            tiers: BTreeMap::new(),
        }
    }
}

impl ExchangeParams {
    pub fn cover(&self, k: MoveKind) -> f64 {
        self.cover.get(&k).copied().unwrap_or(0.0)
    }

    pub fn backfire(&self, k: MoveKind) -> f64 {
        self.backfire.get(&k).copied().unwrap_or(0.0)
    }

    pub fn answer(&self, k: MoveKind) -> f64 {
        self.answer.get(&k).copied().unwrap_or(0.0)
    }

    /// The readings of a worry in a law's ledger.
    pub fn readings(&self, concern: &str) -> &[LedgerReading] {
        self.worries
            .iter()
            .find(|w| w.concern == concern)
            .map(|w| w.readings.as_slice())
            .unwrap_or(&[])
    }

    /// The block as a scene in a tier sees it: the shared values, then the
    /// tier's overrides, then the scene's own. An override that names no
    /// number (validation refuses it) is ignored.
    pub fn resolved(&self, tier: TierKey, scene: &BTreeMap<String, f64>) -> ExchangeParams {
        let of_tier = self.tiers.get(&tier);
        if scene.is_empty() && of_tier.is_none_or(|t| t.is_empty()) {
            return self.clone();
        }
        let mut out = self.clone();
        for (path, v) in of_tier.into_iter().flatten().chain(scene) {
            if let Ok(p) = out.with(path, *v) {
                out = p;
            }
        }
        out
    }

    /// One number replaced by its dotted path (`cover.own`).
    pub fn with(&self, path: &str, value: f64) -> Result<ExchangeParams, String> {
        if path == "tiers" || path.starts_with("tiers.") || path.starts_with("worries") {
            return Err(format!("`{path}` cannot be overridden"));
        }
        let mut root = serde_json::to_value(self).map_err(|e| e.to_string())?;
        let mut at = &mut root;
        let mut parts = path.split('.').peekable();
        while let Some(part) = parts.next() {
            let obj = at
                .as_object_mut()
                .ok_or_else(|| format!("`{path}`: `{part}` is not inside a block"))?;
            if parts.peek().is_none() {
                match obj.get(part) {
                    Some(old) if old.is_number() => {
                        let n = if old.is_u64() || old.is_i64() {
                            serde_json::json!(value.round() as i64)
                        } else {
                            serde_json::json!(value)
                        };
                        obj.insert(part.to_string(), n);
                    }
                    Some(_) => return Err(format!("`{path}` is not a number")),
                    // A kind's entry in a by-kind table may be new.
                    None if MoveKind::from_key(part).is_some() => {
                        obj.insert(part.to_string(), serde_json::json!(value));
                    }
                    None => return Err(format!("`{path}` names no constant")),
                }
                break;
            }
            at = obj
                .get_mut(part)
                .ok_or_else(|| format!("`{path}` names no constant"))?;
        }
        serde_json::from_value(root).map_err(|e| format!("`{path}`: {e}"))
    }

    /// The worries the table names that the pack does not have: no error
    /// (a pack that drops a concern has no reading for it), a warning.
    pub fn unknown_worries(&self, concern_known: &dyn Fn(&str) -> bool) -> Vec<&str> {
        self.worries
            .iter()
            .map(|w| w.concern.as_str())
            .filter(|c| !concern_known(c))
            .collect()
    }

    /// Problems with the block itself (ranges).
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let unit = |name: &str, v: f64, out: &mut Vec<String>| {
            if !(0.0..=1.0).contains(&v) {
                out.push(format!("`{name}` must be in [0, 1]"));
            }
        };
        if !(1..=5).contains(&self.rounds) {
            out.push("`rounds` must be 1..5".into());
        }
        for (n, v) in [
            ("bite", self.bite),
            ("gain", self.gain),
            ("spill", self.spill),
            ("public_share", self.public_share),
            ("voted_share", self.voted_share),
            ("second_base", self.second_base),
            ("act_base", self.act_base),
            ("act_votes", self.act_votes),
            ("heard_floor", self.heard_floor),
            ("stay_strength", self.stay_strength),
            ("repeat", self.repeat),
            ("win", self.win),
            ("lose", self.lose),
            ("group_margin", self.group_margin),
            ("member_line", self.member_line),
            ("hold.heard", self.hold.heard),
            ("hold.known", self.hold.known),
            ("hold.detailed", self.hold.detailed),
            ("fit.low", self.fit.low),
            ("fit.next", self.fit.next),
            ("fit.top", self.fit.top),
            ("fit.flat", self.fit.flat),
            ("size.promise", self.size.promise),
            ("size.question", self.size.question),
            ("size.absent", self.size.absent),
            ("world.approve", self.world.approve),
            ("world.aware", self.world.aware),
            ("worry_match", self.worry_match),
        ] {
            unit(n, v, &mut out);
        }
        if !(self.fit.low <= self.fit.next && self.fit.next <= self.fit.top) {
            out.push("`fit` must rise: low ≤ next ≤ top".into());
        }
        if !(1.0..=2.0).contains(&self.cold) {
            out.push("`cold` must be in [1, 2]".into());
        }
        if !(self.hold.heard <= self.hold.known && self.hold.known <= self.hold.detailed) {
            out.push("`hold` must rise with knowledge".into());
        }
        for (name, table) in [
            ("cover", &self.cover),
            ("backfire", &self.backfire),
            ("answer", &self.answer),
        ] {
            for (k, v) in table {
                if !(0.0..=1.0).contains(v) {
                    out.push(format!("`{name}.{}` must be in [0, 1]", k.key()));
                }
            }
        }
        if !self.cover.contains_key(&MoveKind::Pass) {
            out.push("`cover.pass` is missing: the safe move covers a little".into());
        }
        if !(3..=5).contains(&self.list_max) || self.prep_pins > 3 {
            out.push("`list_max` is 3..5 and `prep_pins` at most 3".into());
        }
        if !(1..=5).contains(&self.prompt_variants) || !(1..=5).contains(&self.say_variants) {
            out.push("`prompt_variants` and `say_variants` are 1..5".into());
        }
        if self.voice_base <= 0.0 || self.voice_span < 0.0 {
            out.push("`voice_base` is above 0 and `voice_span` is not negative".into());
        }
        if self.walk_away > 0.0 || self.walk_away < -1.0 {
            out.push("`walk_away` must be in [−1, 0]".into());
        }
        if self.deck.base < 1.0 || self.deck.per_skill < 0.0 || self.deck.pick_top == 0 {
            out.push("`deck`: at least one attack held, and `pick_top` at least 1".into());
        }
        if !(self.member_fit.min <= self.member_fit.base
            && self.member_fit.base <= self.member_fit.max)
        {
            out.push("`member_fit`: min ≤ base ≤ max".into());
        }
        for w in &self.worries {
            for r in &w.readings {
                if r.key.is_empty() || r.full <= 0.0 {
                    out.push(format!(
                        "`worries`: `{}` needs a ledger key and a full amount above 0",
                        w.concern
                    ));
                }
            }
        }
        for (tier, patch) in &self.tiers {
            for (path, v) in patch {
                if let Err(e) = self.with(path, *v) {
                    out.push(format!("`tiers.{tier:?}`: {e}"));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_designs_first_values() {
        let p = ExchangeParams::default();
        assert_eq!(p.rounds, 3);
        assert_eq!((p.bite, p.gain, p.cold), (0.30, 0.25, 1.25));
        assert_eq!(p.cover(MoveKind::Record), 0.8);
        assert_eq!(p.cover(MoveKind::Pass), 0.2);
        assert_eq!(p.backfire(MoveKind::Turn), 0.25);
        assert_eq!(p.backfire(MoveKind::Pass), 0.0);
        assert_eq!((p.win, p.lose, p.group_margin), (0.08, 0.08, 0.05));
        assert!(p.problems().is_empty(), "{:?}", p.problems());
    }

    #[test]
    fn an_override_replaces_one_number_by_its_path() {
        let p = ExchangeParams::default();
        let q = p.with("cover.own", 0.5).unwrap();
        assert_eq!(q.cover(MoveKind::Own), 0.5);
        assert_eq!(q.cover(MoveKind::Record), p.cover(MoveKind::Record));
        let q = p.with("fit.top", 0.9).unwrap();
        assert_eq!(q.fit.top, 0.9);
        let q = p.with("rounds", 2.0).unwrap();
        assert_eq!(q.rounds, 2);
        assert!(p.with("no_such", 1.0).is_err());
        assert!(p.with("fit.none", 1.0).is_err());
        assert!(p.with("fit", 1.0).is_err());
        assert!(p.with("tiers", 1.0).is_err());
    }

    #[test]
    fn a_tier_then_a_scene_override_the_shared_block() {
        let mut p = ExchangeParams::default();
        p.tiers.insert(
            TierKey::Regional,
            [("win".to_string(), 0.1), ("lose".to_string(), 0.1)].into(),
        );
        let scene: BTreeMap<String, f64> = [("win".to_string(), 0.2)].into();
        let local = p.resolved(TierKey::Local, &BTreeMap::new());
        assert_eq!(local.win, 0.08);
        let regional = p.resolved(TierKey::Regional, &scene);
        assert_eq!((regional.win, regional.lose), (0.2, 0.1));
    }

    /// §9.4: every kind but the safe one declares a requirement.
    #[test]
    fn every_kind_but_pass_declares_a_requirement() {
        for k in MoveKind::ALL {
            assert_eq!(k.requirement().is_none(), k == MoveKind::Pass, "{k:?}");
        }
        // And the list order is the table's.
        let mut sorted = MoveKind::ALL;
        sorted.sort();
        assert_eq!(sorted, MoveKind::ALL);
    }

    #[test]
    fn a_count_is_a_share_of_heads() {
        assert!(CountNeeds::Majority.met(4, 6));
        assert!(!CountNeeds::Majority.met(3, 6));
        assert!(CountNeeds::TwoThirds.met(4, 6));
        assert!(!CountNeeds::TwoThirds.met(3, 6));
    }
}
