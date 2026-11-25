//! Narrative (DEC-033, DEC-032, DEC-077 §4): the director, decisions
//! (dilemmas, requests and the arcs' decision beats), the facts log, and
//! the director's exogenous levers. Storylines are arcs now (DEC-109:
//! `crate::arcs`); the director times them and nothing else.
//!
//! **Authority (DEC-033 B: a storyteller of chance, not of consequence).**
//! The director's systems write only this domain (`NarrativeState`). They
//! read live state, choose which situations to spotlight (pending
//! decisions, developments), and post *requests* on exogenous chance
//! (`Chance`): a hold on hazards in the player's town, the share of hazard
//! strikes there, a hold on the onset of a global recession. The owning
//! systems (environment, economy) accept and clamp them, and log every
//! intervention. Hidden facts get a discovery date inside their evidence
//! window. Nothing here changes an endogenous outcome: decisions change the
//! world only when answered, through `resolve` in the Commands phase (the
//! player's answer, or the default when the deadline passes, which is the
//! player's inaction).
//!
//! **Determinism.** Every draw is keyed (seed, `Stream::Director`, date,
//! subject), so the director never shifts a sequential stream.

pub mod audit;
pub mod cast;
pub mod director;
pub mod learn;
pub mod offered;
pub mod resolve;

use crate::characters::CharacterId;
use crate::places::Location;
use mandate_content::{ContentBundle, ContentKey};
use mandate_core::{SimDate, define_id};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use director::{daily, weekly};
pub use resolve::{answer, resolve_due};

define_id!(
    /// A decision the director spotlighted (never reused).
    DecisionId
);

/// Keyed-RNG tags (DEC-041): one per kind of director draw.
pub(crate) const KEY_JITTER: u64 = 0xD1E0;
pub(crate) const KEY_DISCOVERY: u64 = 0xD1E1;
/// (Used by the environment for a Steady profile's missed strikes.)
pub const KEY_HAZARD_KEEP: u64 = 0xD1E2;
/// The pick among candidates (ENGAGEMENT §4.2).
pub(crate) const KEY_PICK: u64 = 0xD1E3;
/// Phase lengths of the pacing cycle (§4.3).
pub(crate) const KEY_PHASE: u64 = 0xD1E4;
/// A chapter's theme (§4.2).
pub(crate) const KEY_THEME: u64 = 0xD1E5;
/// A `sponsored_law` condition's chance, per law (§4.4).
pub(crate) const KEY_SPONSORED_LAW: u64 = 0xD1E6;

/// A live actor cast in a role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Actor {
    Person {
        id: CharacterId,
    },
    Neighborhood {
        id: u32,
    },
    /// A bill or a law (policy instance).
    Policy {
        id: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CastEntry {
    pub role: String,
    pub actor: Actor,
}

/// Find a role's actor in a cast.
pub fn actor_of(cast: &[CastEntry], role: &str) -> Option<Actor> {
    cast.iter().find(|c| c.role == role).map(|c| c.actor)
}

pub fn person_of(cast: &[CastEntry], role: &str) -> Option<CharacterId> {
    match actor_of(cast, role)? {
        Actor::Person { id } => Some(id),
        _ => None,
    }
}

/// The weight of a decision in the chapter's budgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Weight {
    Climax,
    Significant,
    Minor,
}

/// Why a situation came up: the live facts it was grounded in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Reason {
    Crisis {
        kind: ContentKey,
        severity: f32,
    },
    Concern {
        concern: ContentKey,
    },
    Election {
        days: u32,
    },
    /// A beat of a running arc (its template).
    Arc {
        arc: ContentKey,
    },
    Memory {
        person: CharacterId,
    },
    Goal {
        person: CharacterId,
        goal: ContentKey,
    },
    Ward,
    Pressure {
        track: String,
    },
    Opportunity,
}

/// What one effect of an answered decision did (facts for the aftermath).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Applied {
    Opinion {
        person: CharacterId,
        delta: f32,
    },
    /// Mean change in approval of the player among the voters reached.
    Approval {
        scope: String,
        neighborhood: Option<u32>,
        classes: Vec<u8>,
        delta: f32,
    },
    Awareness {
        scope: String,
        delta: f32,
    },
    Money {
        amount: f64,
    },
    Health {
        delta: f32,
    },
    Slots {
        n: u8,
    },
    Standing {
        delta: f32,
    },
    LegalJeopardy {
        delta: f32,
    },
    Record {
        delta: f32,
    },
    Favour {
        person: CharacterId,
        owed_to_player: bool,
    },
    Repaid {
        person: CharacterId,
    },
    Vote {
        policy: u32,
        vote: String,
    },
    Drafted {
        policy: u32,
    },
    /// A rider was attached to a bill (`ward_project` or `sunset`).
    Rider {
        policy: u32,
        rider: String,
    },
    /// Someone who owed the player a favour repaid it on a bill.
    CalledIn {
        person: CharacterId,
        policy: u32,
    },
    /// A borderline or illegal act was done: it may come out (`risk`: the
    /// chance within a year, as shown before the choice).
    Hidden {
        kind: String,
        risk: f32,
    },
    /// The player answered a story or an investigation.
    Responded {
        response: String,
    },
    /// The effect could not happen any more (e.g. the vote was already held).
    Skipped {
        effect: String,
    },
    /// The player learned something (DEC-091): `n` subjects of this type
    /// (`person`, `hood_concerns`, `place`, …) are now at `level` or above.
    Learned {
        subject: String,
        level: String,
        n: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Pending,
    Answered,
    /// The deadline passed: the default option happened.
    Defaulted,
    /// The situation dissolved before an answer (a bill was decided, its
    /// template was removed by an update).
    Lapsed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    pub option: String,
    pub date: SimDate,
    pub by_default: bool,
    pub applied: Vec<Applied>,
}

/// A decision the director spotlighted: a dilemma, a request, or an
/// arc's decision beat. It is visible from `opened` until `deadline`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    pub id: DecisionId,
    /// Dilemma content id.
    pub template: ContentKey,
    pub weight: Weight,
    pub cast: Vec<CastEntry>,
    pub reasons: Vec<Reason>,
    pub opened: SimDate,
    pub deadline: SimDate,
    pub chapter: u32,
    pub relevance: f32,
    /// The goal that raised a request: (holder, goal id).
    pub goal: Option<(CharacterId, ContentKey)>,
    /// Where its scene happens.
    pub place: Option<Location>,
    pub status: DecisionStatus,
    pub resolution: Option<Resolution>,
    /// The story or case a raised decision answers.
    #[serde(default)]
    pub about: Option<About>,
    /// The arc run and beat it presents (DEC-109): the run reads its
    /// answer from here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arc: Option<crate::arcs::ArcRef>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChapterKind {
    /// A budget year in office.
    Session,
    /// The player's campaign; its climax is the election.
    Campaign,
}

impl ChapterKind {
    pub fn key(self) -> &'static str {
        match self {
            ChapterKind::Session => "session",
            ChapterKind::Campaign => "campaign",
        }
    }
}

/// A chapter (DEC-005) and what the director spotlighted in it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    pub index: u32,
    pub kind: ChapterKind,
    pub start: SimDate,
    /// Planned end (a session: the next budget year; a campaign: election day).
    pub end: SimDate,
    /// When it actually ended.
    pub closed: Option<SimDate>,
    pub climaxes: u8,
    pub significant: u8,
    pub minor: u8,
    /// The decision chosen as its climax (a session), if any.
    pub climax: Option<DecisionId>,
    pub peak_tension: f32,
    pub last_decision: Option<SimDate>,
    pub last_minor: Option<SimDate>,
    /// Templates and people spotlighted this chapter (no repeats; the
    /// repetition penalty on people).
    pub templates: Vec<ContentKey>,
    pub people: Vec<CharacterId>,
    /// The chapter's theme (ENGAGEMENT §4.2): `crisis`, `budget`, `scheme`,
    /// `local_life`, or `campaign`; drawn when it opens.
    #[serde(default)]
    pub theme: Option<ContentKey>,
    /// How many themes the live state supported when it opened (a theme
    /// repeats from the last session chapter only when this is 1).
    #[serde(default)]
    pub theme_options: u8,
    /// Peaks of the pacing cycle that began in this chapter (§4.3).
    #[serde(default)]
    pub peaks: u8,
    /// The arc run that leads the chapter (DEC-109: the director's
    /// choice among those running; its turn is the chapter's climax).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lead: Option<u32>,
    /// The turn that was the chapter's climax: `<arc template>/<stage>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<String>,
}

impl Chapter {
    pub fn new(index: u32, kind: ChapterKind, start: SimDate, end: SimDate) -> Self {
        Chapter {
            index,
            kind,
            start,
            end,
            closed: None,
            climaxes: 0,
            significant: 0,
            minor: 0,
            climax: None,
            peak_tension: 0.0,
            last_decision: None,
            last_minor: None,
            templates: Vec::new(),
            people: Vec::new(),
            theme: None,
            theme_options: 0,
            peaks: 0,
            lead: None,
            turn: None,
        }
    }

    /// Share of the planned chapter elapsed (0..1).
    pub fn progress(&self, on: SimDate) -> f64 {
        let total = self.start.days_until(self.end).max(1) as f64;
        (self.start.days_until(on) as f64 / total).clamp(0.0, 1.0)
    }
}

/// What a fact is about (the facts log; DEC-034's media layer will read it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FactKind {
    ChapterBegan {
        index: u32,
        kind: ChapterKind,
    },
    DecisionOpened {
        decision: DecisionId,
    },
    DecisionResolved {
        decision: DecisionId,
        option: String,
        by_default: bool,
    },
    DecisionLapsed {
        decision: DecisionId,
    },
    CrisisBegan {
        kind: ContentKey,
    },
    CrisisWorse {
        kind: ContentKey,
    },
    GoalTaken {
        person: CharacterId,
        goal: ContentKey,
    },
    RivalNamed {
        person: CharacterId,
    },
    TrackWarning {
        track: String,
        value: f32,
    },
    /// A hidden fact can now be discovered (the media's hook).
    Discoverable {
        hidden: u32,
    },
    /// Something that happened in the player's town (a logged event: a
    /// closure, a disaster, a colleague's bill), spotlighted as news.
    TownEvent {
        event: crate::events::EventId,
    },
}

/// One entry of the facts log. `spotlit` facts were presented to the player
/// (and count against the chapter's budgets when minor).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    pub id: u32,
    pub date: SimDate,
    pub chapter: u32,
    pub kind: FactKind,
    pub actors: Vec<Actor>,
    pub spotlit: bool,
    /// How newsworthy (0..1): relevance for the media task.
    pub newsworthiness: f32,
}

/// Whether a hidden fact will come out, and to whom: settled when it
/// happened (evidence × the discoverers' interest × their access, a keyed
/// draw), never by the director (DEC-033 B: the director times it within the
/// evidence window, it never decides whether).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fate {
    /// The chance it comes out at all.
    pub chance: f32,
    pub discovered: bool,
    pub by: mandate_content::Discoverer,
    /// The person who finds it (a journalist, a rival), if a person does.
    pub who: Option<CharacterId>,
}

/// Something that happened out of sight (DEC-034): it may come to light
/// within its evidence window; the director chooses when (DEC-033 B).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HiddenFact {
    pub id: u32,
    /// What happened (Fluent `fact-<kind>`), e.g. `bribe`, `back_room_deal`.
    pub kind: String,
    pub date: SimDate,
    pub actors: Vec<Actor>,
    /// Evidence window: it cannot surface before [0] and will by [1].
    pub window: [SimDate; 2],
    pub discoverable_on: Option<SimDate>,
    pub surfaced: bool,
    /// Whose secret it is.
    #[serde(default)]
    pub subject: Option<crate::politics::PoliticianId>,
    #[serde(default)]
    pub severity: mandate_content::Gravity,
    /// Evidence it left (0..1).
    #[serde(default)]
    pub evidence: f32,
    /// The subject's town when it happened.
    #[serde(default)]
    pub town: Option<u32>,
    #[serde(default)]
    pub fate: Option<Fate>,
    /// What made it (the causal spine): the act itself, the player's or
    /// someone else's. Absent in a fact from before the spine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<crate::spine::Cause>,
}

/// What a raised decision answers (DEC-034, DEC-006).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum About {
    /// A story about the player (`media::Scandal` id).
    Scandal { id: u32 },
    /// An investigation of the player (`justice::Case` id).
    Case { id: u32 },
}

/// The director's requests on exogenous chance (DEC-033 B). Owning systems
/// accept and clamp them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Chance {
    /// Hold hazard strikes on this town until this date (they are deferred,
    /// not removed: they strike within `director.chance.hazard_max_defer_days`).
    pub hazard_hold: Option<(u32, SimDate)>,
    /// Share of hazard strikes on this town that happen (Steady).
    pub hazard_keep: Option<(u32, f32)>,
    /// Hold the onset of a global recession until this date.
    pub recession_hold: Option<SimDate>,
}

/// A fairness warning issued (DEC-006).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Warning {
    pub track: String,
    pub date: SimDate,
    pub value: f32,
}

/// A phase of the director's pacing cycle (ENGAGEMENT §4.3, after Left 4
/// Dead's AI Director and RimWorld's Cassandra): a build-up of minor beats,
/// a peak with one significant decision, its fade, then a relax.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseKind {
    BuildUp,
    Peak,
    PeakFade,
    Relax,
}

impl PhaseKind {
    pub fn key(self) -> &'static str {
        match self {
            PhaseKind::BuildUp => "build_up",
            PhaseKind::Peak => "peak",
            PhaseKind::PeakFade => "peak_fade",
            PhaseKind::Relax => "relax",
        }
    }
}

/// The phase in force (and, in `NarrativeState::phase_log`, one that ended).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectorPhase {
    pub kind: PhaseKind,
    pub since: SimDate,
    /// A build-up's or relax's drawn end, a peak's latest end; in the log,
    /// when it actually ended.
    pub until: Option<SimDate>,
    /// The cycle (one build-up, peak, fade and relax), counted over the run.
    pub cycle: u32,
    /// The chapter it began in.
    pub chapter: u32,
    /// A peak that is the chapter's climax (its relax is the longer one).
    #[serde(default)]
    pub climax: bool,
    /// Easing off after a setback (§4.3): a lengthened relax, or a peak that
    /// draws opportunities first.
    #[serde(default)]
    pub eased: bool,
    /// The decision a peak spotlighted.
    #[serde(default)]
    pub decision: Option<DecisionId>,
    /// Minor beats spotlighted while it lasted.
    #[serde(default)]
    pub beats: u16,
}

/// One of the player's past answers (ENGAGEMENT §4.4): what `answered`
/// conditions read, kept longer than the bounded decisions history.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnswerRecord {
    pub decision: DecisionId,
    pub template: ContentKey,
    pub option: String,
    pub date: SimDate,
    /// The deadline passed and the default happened.
    #[serde(default)]
    pub by_default: bool,
    pub cast: Vec<CastEntry>,
}

/// What the director saw last week (to notice developments).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Seen {
    /// Crisis kinds with a severity band (0..3).
    pub crises: Vec<(ContentKey, u8)>,
    pub rival: Option<CharacterId>,
    pub goals: Vec<(CharacterId, ContentKey)>,
    /// The last event-log entry looked at.
    #[serde(default)]
    pub last_event: u32,
    /// The player had filed for office (relax ends when they file, §4.3).
    #[serde(default)]
    pub filed: bool,
}

/// The narrative domain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NarrativeState {
    /// Storyteller profile id (`storytellers`); empty = the content's default.
    pub profile: ContentKey,
    /// When play began (the director's first day with a player).
    pub play_began: Option<SimDate>,
    pub chapter: Option<Chapter>,
    /// Closed chapters, oldest first (bounded).
    pub chapters: Vec<Chapter>,
    pub tension: f32,
    pub decision_ids: mandate_core::IdAllocator,
    /// Pending decisions, then recent resolved ones (bounded).
    pub decisions: Vec<Decision>,
    /// Dilemma → the earliest date it may return.
    #[serde(with = "mandate_core::serde_util::map_as_pairs")]
    pub cooldowns: BTreeMap<ContentKey, SimDate>,
    /// Goals whose request was raised: (holder, goal, adopted on).
    pub requests_raised: Vec<(CharacterId, ContentKey, SimDate)>,
    pub next_fact: u32,
    /// The facts log (bounded).
    pub facts: Vec<Fact>,
    pub hidden: Vec<HiddenFact>,
    pub next_hidden: u32,
    pub chance: Chance,
    pub warnings: Vec<Warning>,
    pub seen: Seen,
    /// Lab switch: spotlight situations (decisions, the arcs' decision
    /// beats, developments). Off, the director still shapes chance
    /// identically, and arcs run as they would (only their presentation
    /// is the director's).
    pub spotlight: bool,
    /// Lab switch: apply default options when deadlines pass (off, a
    /// decision past its deadline lapses without consequence).
    pub resolve_defaults: bool,
    /// Stories and cases a decision has been raised for.
    #[serde(default)]
    pub raised: Vec<About>,
    /// Lab switch: shape discovery timing by the chapter's tension (DEC-033
    /// B). Off, a hidden fact's date is uniform in its window.
    #[serde(default = "yes")]
    pub shape_discovery: bool,
    /// Times each template was spotlighted (a dilemma) or led a chapter
    /// (an arc) this run (cooldown growth, novelty; ENGAGEMENT §4.1).
    #[serde(default, with = "mandate_core::serde_util::map_as_pairs")]
    pub uses: BTreeMap<ContentKey, u16>,
    /// Recent climaxes: (chapter index, `<arc template>/<turn stage>`),
    /// oldest first (bounded).
    #[serde(default)]
    pub climaxes: Vec<(u32, ContentKey)>,
    /// The player's answers, oldest first (bounded at `ANSWERS_CAP`).
    #[serde(default)]
    pub answers: Vec<AnswerRecord>,
    /// The pacing cycle's phase; `None` under a profile without a cycle
    /// (Unfiltered) or before play.
    #[serde(default)]
    pub phase: Option<DirectorPhase>,
    /// A setback not yet eased (§4.3): an election lost, Standing or
    /// viability at its warning level, a scandal about the player.
    #[serde(default)]
    pub setback: Option<SimDate>,
    /// Phases that ended, oldest first (bounded): the lab's pacing metrics.
    #[serde(default)]
    pub phase_log: Vec<DirectorPhase>,
}

fn yes() -> bool {
    true
}

impl Default for NarrativeState {
    fn default() -> Self {
        NarrativeState {
            profile: String::new(),
            play_began: None,
            chapter: None,
            chapters: Vec::new(),
            tension: 0.0,
            decision_ids: Default::default(),
            decisions: Vec::new(),
            cooldowns: BTreeMap::new(),
            requests_raised: Vec::new(),
            next_fact: 0,
            facts: Vec::new(),
            hidden: Vec::new(),
            next_hidden: 0,
            chance: Chance::default(),
            warnings: Vec::new(),
            seen: Seen::default(),
            spotlight: true,
            resolve_defaults: true,
            raised: Vec::new(),
            shape_discovery: true,
            uses: BTreeMap::new(),
            climaxes: Vec::new(),
            answers: Vec::new(),
            phase: None,
            setback: None,
            phase_log: Vec::new(),
        }
    }
}

/// Facts kept in the log.
pub const FACTS_CAP: usize = 400;
/// Closed chapters kept.
pub const CHAPTERS_CAP: usize = 60;
/// Hidden facts kept after they surfaced.
pub const HIDDEN_CAP: usize = 200;
/// Answers kept for `answered` conditions (a five-year look-back).
pub const ANSWERS_CAP: usize = 200;
/// Ended phases kept.
pub const PHASE_LOG_CAP: usize = 200;
/// Climaxes remembered.
pub const CLIMAXES_CAP: usize = 8;

impl NarrativeState {
    pub fn decision(&self, id: DecisionId) -> Option<&Decision> {
        self.decisions.iter().find(|d| d.id == id)
    }

    pub fn pending(&self) -> impl Iterator<Item = &Decision> {
        self.decisions
            .iter()
            .filter(|d| d.status == DecisionStatus::Pending)
    }

    /// The storyteller profile in force.
    pub fn profile<'a>(
        &self,
        content: &'a ContentBundle,
    ) -> Option<&'a mandate_content::StorytellerDef> {
        content
            .storyteller(&self.profile)
            .or_else(|| content.storyteller(&content.director.default_profile))
            .or(content.storytellers.first())
    }

    pub(crate) fn log(
        &mut self,
        date: SimDate,
        kind: FactKind,
        actors: Vec<Actor>,
        spotlit: bool,
        newsworthiness: f32,
    ) -> u32 {
        let id = self.next_fact;
        self.next_fact += 1;
        let chapter = self.chapter.as_ref().map(|c| c.index).unwrap_or(0);
        self.facts.push(Fact {
            id,
            date,
            chapter,
            kind,
            actors,
            spotlit,
            newsworthiness,
        });
        if self.facts.len() > FACTS_CAP {
            let drop = self.facts.len() - FACTS_CAP;
            self.facts.drain(..drop);
        }
        id
    }

    /// Record a hidden fact (the Commands phase and NPC turns: the act
    /// itself). `media::hide` settles its fate and window first.
    pub fn hide(&mut self, mut h: HiddenFact) -> u32 {
        let id = self.next_hidden;
        self.next_hidden += 1;
        h.id = id;
        self.hidden.push(h);
        if self.hidden.len() > HIDDEN_CAP
            && let Some(i) = self.hidden.iter().position(|h| h.surfaced)
        {
            self.hidden.remove(i);
        }
        id
    }

    /// Whether a terminal ending on `track` may happen on `on` (DEC-006
    /// fairness): only after a warning, with `fairness.response_days` to
    /// respond. The ending systems (M6+) must ask this first.
    pub fn ending_allowed(&self, content: &ContentBundle, track: &str, on: SimDate) -> bool {
        let wait = content.director.fairness.response_days;
        self.warnings
            .iter()
            .any(|w| w.track == track && w.date.add_days(wait) <= on)
    }

    /// Keep pending decisions and the most recent resolved ones.
    pub(crate) fn trim(&mut self, keep: usize) {
        let resolved = self
            .decisions
            .iter()
            .filter(|d| d.status != DecisionStatus::Pending)
            .count();
        if resolved > keep {
            let mut drop = resolved - keep;
            self.decisions.retain(|d| {
                if drop > 0 && d.status != DecisionStatus::Pending {
                    drop -= 1;
                    false
                } else {
                    true
                }
            });
        }
        if self.chapters.len() > CHAPTERS_CAP {
            let drop = self.chapters.len() - CHAPTERS_CAP;
            self.chapters.drain(..drop);
        }
    }
}

/// Bring the narrative in line with loaded content (DEC-074): a pending
/// decision whose template was removed lapses, and cooldowns for removed
/// templates are dropped (an arc run whose template was removed ends at
/// its next step). Never refuses a save.
pub fn on_load(state: &mut crate::State, content: &ContentBundle) {
    let date = state.calendar.date;
    let n = &mut state.narrative;
    for d in n.decisions.iter_mut() {
        if d.status == DecisionStatus::Pending && content.dilemma(&d.template).is_none() {
            d.status = DecisionStatus::Lapsed;
            d.resolution = Some(Resolution {
                option: String::new(),
                date,
                by_default: false,
                applied: Vec::new(),
            });
        }
    }
    n.cooldowns.retain(|k, _| content.dilemma(k).is_some());
    crate::characters::goals::on_load(&mut state.politics, content);
}
