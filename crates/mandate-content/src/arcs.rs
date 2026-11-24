//! Staged arcs (DEC-109; `docs/design/STAGED_ARCS.md`): one matter in one
//! jurisdiction, with a cast bound to real people and places, stages, a
//! clock, a heat and exits. Content kind `arcs`
//! (`content/base/arcs/*.jsonc`); the engine is `mandate_sim::arcs`.
//!
//! **Transitions read; beats write.** A stage's `next` edges fire on
//! conditions over simulated state, the run's own marks, heat and days, or
//! on the clock. Nothing a player picks names a stage: a beat sets marks
//! and posts effect requests, and the edges read the result.
//!
//! **One file per arc.** An `arcs` file may carry the dilemmas, asks,
//! scenes and overheard lines its beats name (`dilemmas`, `asks`, `scenes`,
//! `overheard`): they join the same registries after every other file's
//! items, so an arc never shifts the registry order that breaks ties and
//! keys draws.
//!
//! **Tiers (DEC-098).** A run is keyed by a jurisdiction and its
//! conditions are evaluated there. Roles bind through occupations, powers
//! and place kinds, never an office's or a body's name; `tiers` says where
//! a template may run, and every condition is checked against each.
//!
//! **Not yet.** The schema is the design's; the engine builds it in tasks
//! (STAGED_ARCS §9). What a template uses that the engine does not do yet
//! is listed by [`not_yet`]; the engine never raises such a template.
//!
//! Adding an arc: `docs/development/EXTENDING.md`, "Adding an arc".

use crate::bundle::ContentBundle;
use crate::condition::{Condition, ConditionUse, Site};
use crate::knowledge::Level;
use crate::types::{ContentKey, HazardKey, Indicator, Power, TierKey};
use crate::validate::Issue;
use crate::visits::VisitTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Arguments every arc text may use: the jurisdiction (`place`), the
/// player's home (`town`), the day the matter began (`day`, `month`,
/// `year`), weeks since (`weeks`) and the clock's day (`clock_day`,
/// `clock_month`). The months arrive as `month_key` and `clock_month_key`.
pub const COMMON_ARGS: [&str; 8] = [
    "place",
    "town",
    "day",
    "month",
    "year",
    "weeks",
    "clock_day",
    "clock_month",
];

/// An arc template.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArcDef {
    pub id: ContentKey,
    pub title_key: String,
    /// Chapter themes it carries (`crisis`, `scheme`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub themes: Vec<String>,
    /// An opportunity: the director times its arrival (task A2).
    #[serde(default)]
    pub opportunity: bool,
    /// Where it may run.
    pub tiers: Vec<TierKey>,
    /// Conditions over simulated state at a jurisdiction; never the ledger.
    pub raise: Vec<Condition>,
    pub roles: Vec<ArcRoleDef>,
    /// For stand-in players and the opening frame (task A4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sides: Option<Sides>,
    #[serde(default)]
    pub heat: HeatDef,
    /// Policy templates NPC agendas favour while it runs (task A2).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<ContentKey>,
    /// Conditions on the cast as bound (the arc's roles as slots): a run
    /// is raised only if they hold, so a matter too small to tell raises
    /// none (a flood that sent nobody from their home).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub given: Vec<Condition>,
    /// Readings for the text: conditions evaluated for their arguments
    /// only (one without bounds always holds), with the arc's roles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<Condition>,
    /// Days before it may be raised again in the same jurisdiction.
    #[serde(default)]
    pub cooldown_days: u32,
    /// Days since play began before it may be raised.
    #[serde(default)]
    pub min_career_days: u32,
    /// Engine features it needs (`parties.membership`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
    /// The first stage is where a run starts.
    pub stages: Vec<StageDef>,
}

impl ArcDef {
    pub fn stage(&self, id: &str) -> Option<&StageDef> {
        self.stages.iter().find(|s| s.id == id)
    }

    pub fn role(&self, id: &str) -> Option<&ArcRoleDef> {
        self.roles.iter().find(|r| r.id == id)
    }

    /// Every beat with its stage.
    pub fn beats(&self) -> impl Iterator<Item = (&StageDef, &ArcBeat)> {
        self.stages
            .iter()
            .flat_map(|s| s.beats.iter().map(move |b| (s, b)))
    }
}

/// The two sides of an arc, by role.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sides {
    pub ally: String,
    pub opponent: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArcRoleDef {
    /// Role id: a Fluent argument (`$steward`, `$steward_role`, `$hood`)
    /// and the slot conditions name.
    pub id: String,
    pub cast: ArcCast,
    /// The arc runs without it (beats and effects naming it are skipped).
    #[serde(default)]
    pub optional: bool,
}

/// How a role binds to something real, relative to the arc's jurisdiction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArcCast {
    /// A named person there with one of these occupations (in list order,
    /// then the most influential); with `near`, one who lives in that area
    /// role first.
    Occupation {
        occupations: Vec<ContentKey>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        near: Option<String>,
    },
    /// Whoever holds a post there whose constitution lists this power (not
    /// the player; an office before a seat).
    Post { power: Power },
    /// An area one level down: of a town, a neighbourhood.
    Area {
        pick: AreaPick,
        /// The hazard `most_exposed` reads.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hazard: Option<HazardKey>,
    },
    /// An open place of one of these kinds (in list order); with `near`,
    /// one in that area role first.
    Place {
        kinds: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        near: Option<String>,
    },
    /// Another jurisdiction.
    Jurisdiction { pick: JurisdictionPick },
    /// What raised the arc (a plant, a fact). Not yet: task A5.
    Subject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AreaPick {
    /// The area where the hazard damaged the largest share of homes (those
    /// not yet repaired); where a strike fell evenly, where most people
    /// are away; else the most exposed to it.
    MostExposed,
    /// The area least pleased with the player (where a crisis bites).
    LeastPleased,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JurisdictionPick {
    /// The tier above.
    Parent,
}

/// Heat: one number per run, 0 to 1, in the arc's own domain.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HeatDef {
    #[serde(default)]
    pub start: f32,
    /// Heat never falls below this simulated reading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor: Option<HeatFloor>,
}

/// A floor on heat: an indicator of the jurisdiction × `scale`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HeatFloor {
    pub indicator: Indicator,
    pub scale: f64,
}

/// A stage's deadline: a keyed draw in a range of days from its entry, or
/// (not yet) the subject's own date.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClockDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days: Option<[u32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<ClockFrom>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClockFrom {
    /// The date the simulation owns (a review, a forecast). Not yet: A5.
    Subject,
}

/// A goal a role holds while a stage lasts (task A2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GoalGrant {
    pub role: String,
    pub goal: ContentKey,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StageDef {
    pub id: String,
    /// The story so far (an exit's: how it ended), with the cast and the
    /// arc's facts as arguments.
    pub key: String,
    /// Not shown until the ledger says so (task A3).
    #[serde(default)]
    pub hidden: bool,
    /// The turn: the chapter's climax when this arc leads (task A2).
    #[serde(default)]
    pub climax: bool,
    /// A deadline set on entry; later stages keep it until one sets another.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock: Option<ClockDef>,
    /// Heat added each week the stage lasts.
    #[serde(default)]
    pub rise: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub goals: Vec<GoalGrant>,
    /// Effects on entry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enter: Vec<GatedEffect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beats: Vec<ArcBeat>,
    /// Edges in priority order, checked weekly and on entry: the first
    /// that holds is taken.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub next: Vec<EdgeDef>,
    /// A terminal stage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit: Option<ExitDef>,
}

impl StageDef {
    pub fn beat(&self, id: &str) -> Option<&ArcBeat> {
        self.beats.iter().find(|b| b.id == id)
    }
}

/// An edge fires when every part given holds: its conditions, the days
/// since the stage was entered, and the clock.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeDef {
    pub to: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<Condition>,
    /// At least this many days in the stage.
    #[serde(default)]
    pub after_days: u32,
    /// The clock is at most this many days away (0: it has run out).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock_within_days: Option<u32>,
}

impl EdgeDef {
    /// On time alone: nothing but the days or the clock.
    pub fn on_time(&self) -> bool {
        self.when.is_empty() && (self.after_days > 0 || self.clock_within_days.is_some())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExitDef {
    /// A story the outlets may run as it ends.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headline_key: Option<String>,
    /// The Legacy record's line (task A2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_key: Option<String>,
    /// An arc that may follow (task A2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_up: Option<ContentKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_days: Option<[u32; 2]>,
    /// Effect requests to the owning systems, each gated on the run.
    pub effects: Vec<GatedEffect>,
}

/// An effect, applied when every `when` holds and no `unless` does (the
/// run's marks, heat and days; simulated state).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GatedEffect {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<Condition>,
    /// Applied unless all of these hold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unless: Vec<Condition>,
    #[serde(flatten)]
    pub effect: ArcEffect,
}

/// Who a displacement effect moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DisplacedOf {
    /// Those still staying away from the area: they leave for good.
    StillAway,
}

/// One effect of an arc: a request to the system that owns what changes
/// (OVERVIEW §4.3), or a change to the run's own heat.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArcEffect {
    /// The role's opinion of the player, remembered as `memory` if given.
    Opinion {
        role: String,
        delta: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        memory: Option<String>,
    },
    /// Approval of the player in an area role (−1..1), by income class
    /// (empty: all).
    Approval {
        area: String,
        delta: f64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        classes: Vec<u8>,
    },
    /// A favour: the player owes the role one (or the reverse).
    Favour {
        role: String,
        #[serde(default)]
        owed_to_player: bool,
    },
    /// The player's record of visible work (0..1).
    Record { delta: f64 },
    /// The player's Standing.
    Standing { delta: f64 },
    /// Damaged homes in an area role are repaired `repair` times as fast
    /// for `months` months (the environment applies).
    Recovery {
        area: String,
        repair: f64,
        months: u16,
    },
    /// People of an area role leave their town (the population applies).
    Displacement { area: String, of: DisplacedOf },
    /// The run's own heat.
    Heat { delta: f32 },
    /// Approval of the jurisdiction's government in an area. Not yet: A5b.
    GovApproval { area: String, delta: f64 },
    /// A place's state for the dress pass (boarded, marked). Not yet: A5a.
    PlaceState { place: String, state: String },
}

impl ArcEffect {
    /// The roles it names, with the kind of thing each must be.
    pub fn roles(&self) -> Vec<(&str, RoleKind)> {
        match self {
            ArcEffect::Opinion { role, .. } | ArcEffect::Favour { role, .. } => {
                vec![(role.as_str(), RoleKind::Person)]
            }
            ArcEffect::Approval { area, .. }
            | ArcEffect::Recovery { area, .. }
            | ArcEffect::Displacement { area, .. }
            | ArcEffect::GovApproval { area, .. } => vec![(area.as_str(), RoleKind::Area)],
            ArcEffect::PlaceState { place, .. } => vec![(place.as_str(), RoleKind::Spot)],
            ArcEffect::Record { .. } | ArcEffect::Standing { .. } | ArcEffect::Heat { .. } => {
                Vec::new()
            }
        }
    }

    /// Material: more than an opinion, a memory or the run's own heat
    /// (the doctor's `arc-exit-flavour`).
    pub fn material(&self) -> bool {
        !matches!(
            self,
            ArcEffect::Opinion { .. } | ArcEffect::Favour { .. } | ArcEffect::Heat { .. }
        )
    }

    fn key(&self) -> &'static str {
        match self {
            ArcEffect::Opinion { .. } => "opinion",
            ArcEffect::Approval { .. } => "approval",
            ArcEffect::Favour { .. } => "favour",
            ArcEffect::Record { .. } => "record",
            ArcEffect::Standing { .. } => "standing",
            ArcEffect::Recovery { .. } => "recovery",
            ArcEffect::Displacement { .. } => "displacement",
            ArcEffect::Heat { .. } => "heat",
            ArcEffect::GovApproval { .. } => "gov_approval",
            ArcEffect::PlaceState { .. } => "place_state",
        }
    }
}

/// What a role is cast to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoleKind {
    Person,
    Area,
    Place,
    Jurisdiction,
    /// A place or an area (somewhere to be).
    Spot,
    /// Whatever raised the arc.
    Subject,
}

impl ArcCast {
    pub fn kind(&self) -> RoleKind {
        match self {
            ArcCast::Occupation { .. } | ArcCast::Post { .. } => RoleKind::Person,
            ArcCast::Area { .. } => RoleKind::Area,
            ArcCast::Place { .. } => RoleKind::Place,
            ArcCast::Jurisdiction { .. } => RoleKind::Jurisdiction,
            ArcCast::Subject => RoleKind::Subject,
        }
    }
}

impl RoleKind {
    fn accepts(self, got: RoleKind) -> bool {
        self == got || (self == RoleKind::Spot && matches!(got, RoleKind::Place | RoleKind::Area))
    }

    fn word(self) -> &'static str {
        match self {
            RoleKind::Person => "a person",
            RoleKind::Area => "an area",
            RoleKind::Place => "a place",
            RoleKind::Jurisdiction => "a jurisdiction",
            RoleKind::Spot => "a place or an area",
            RoleKind::Subject => "the subject",
        }
    }
}

/// A beat's weight in the chapter's budgets (task A2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BeatWeight {
    Climax,
    #[default]
    Significant,
    Minor,
}

/// What an option of a decision beat does to the run.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OptionOutcome {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sets: Option<String>,
    /// Added to the run's heat.
    #[serde(default)]
    pub heat: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<GatedEffect>,
}

/// The arc's own choice in a conversation with a cast person (a talk
/// hook): the button, what the player says, and the replies, the first
/// whose conditions hold being said. The last has no conditions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArcChoiceDef {
    pub label_key: String,
    pub say_key: String,
    pub replies: Vec<ArcReplyDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArcReplyDef {
    pub key: String,
    /// Slots `speaker` and `player`, the arc's roles, the run's marks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<Condition>,
    /// They agree: the beat is done, and its own `sets`, `relief` and
    /// `effects` apply after this reply's.
    #[serde(default)]
    pub agrees: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sets: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<GatedEffect>,
    /// What the reply teaches about the arc (task A3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teaches: Option<Level>,
}

/// The seven beat kinds, each riding on a system that exists.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BeatKind {
    /// A dilemma with the arc's cast (roles of the same ids). `on`: what
    /// each option does to the run; a default at the deadline counts.
    Decision {
        dilemma: ContentKey,
        #[serde(default)]
        weight: BeatWeight,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        on: BTreeMap<String, OptionOutcome>,
    },
    /// An ask (`from: ["arc"]`) from a cast person; the beat is done when
    /// it is kept.
    Ask { template: ContentKey, asker: String },
    /// The arc's own choice in a conversation with a cast person.
    Talk { with: String, choice: ArcChoiceDef },
    /// Something to do at a cast place or in a cast area.
    Visit {
        at: String,
        /// The button (no arguments: the place lists it).
        verb_key: String,
        outcome_key: String,
        #[serde(default)]
        time: VisitTime,
    },
    /// A story the outlets may run.
    Press { headline_key: String },
    /// An overheard line that holds while the stage does (its own
    /// conditions name the arc).
    Heard { line: ContentKey },
    /// A cast person acts: a keyed weekly draw at `weight`, scaled by
    /// their boldness.
    Move {
        actor: String,
        weight: f32,
        /// How Today and the skip summary report it.
        line_key: String,
    },
}

impl BeatKind {
    pub fn key(&self) -> &'static str {
        match self {
            BeatKind::Decision { .. } => "decision",
            BeatKind::Ask { .. } => "ask",
            BeatKind::Talk { .. } => "talk",
            BeatKind::Visit { .. } => "visit",
            BeatKind::Press { .. } => "press",
            BeatKind::Heard { .. } => "heard",
            BeatKind::Move { .. } => "move",
        }
    }

    /// A beat the player takes with a command (quick mode and Places).
    pub fn player(&self) -> bool {
        matches!(
            self,
            BeatKind::Decision { .. }
                | BeatKind::Ask { .. }
                | BeatKind::Talk { .. }
                | BeatKind::Visit { .. }
        )
    }
}

/// Something a stage offers. `ArcBeat`, because `narrative::BeatDef` is a
/// scene's spoken line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ArcBeat {
    /// Unique in its stage.
    pub id: String,
    #[serde(flatten)]
    pub kind: BeatKind,
    /// Offered while these hold (the run's marks, heat and days; state).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<Condition>,
    /// Done at most once in a run (default).
    #[serde(default = "yes")]
    pub once: bool,
    /// A mark set when it is done.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sets: Option<String>,
    /// Heat it lowers when done.
    #[serde(default)]
    pub relief: f32,
    /// What doing it teaches about the arc (task A3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teaches: Option<Level>,
    /// Effects when it is done.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<GatedEffect>,
}

fn yes() -> bool {
    true
}

// --- What the engine does not do yet ----------------------------------------------------

/// What a template uses that the engine does not do yet (STAGED_ARCS §9),
/// in template order: the engine never raises a template with any.
pub fn not_yet(a: &ArcDef) -> Vec<String> {
    let mut out = Vec::new();
    if !a.requires.is_empty() {
        out.push(format!(
            "requires `{}` (no engine feature is registered yet)",
            a.requires.join("`, `")
        ));
    }
    for r in &a.roles {
        if matches!(r.cast, ArcCast::Subject) {
            out.push(format!(
                "role `{}`: a `subject` cast (the plant's and the fact's fuses, tasks A5a and A5c)",
                r.id
            ));
        }
    }
    let effect = |at: &str, e: &GatedEffect, out: &mut Vec<String>| {
        if matches!(
            e.effect,
            ArcEffect::GovApproval { .. } | ArcEffect::PlaceState { .. }
        ) {
            out.push(format!(
                "{at}: a `{}` effect (its owner's request, task A5)",
                e.effect.key()
            ));
        }
    };
    for s in &a.stages {
        if s.clock.as_ref().is_some_and(|c| c.from.is_some()) {
            out.push(format!(
                "stage `{}`: a clock read from the subject (task A5)",
                s.id
            ));
        }
        for e in &s.enter {
            effect(&format!("stage `{}`", s.id), e, &mut out);
        }
        for b in &s.beats {
            let at = format!("beat `{}.{}`", s.id, b.id);
            for e in &b.effects {
                effect(&at, e, &mut out);
            }
            match &b.kind {
                BeatKind::Decision { on, .. } => {
                    for e in on.values().flat_map(|o| o.effects.iter()) {
                        effect(&at, e, &mut out);
                    }
                }
                BeatKind::Talk { choice, .. } => {
                    for e in choice.replies.iter().flat_map(|r| r.effects.iter()) {
                        effect(&at, e, &mut out);
                    }
                }
                _ => {}
            }
        }
        if let Some(x) = &s.exit {
            for e in &x.effects {
                effect(&format!("exit `{}`", s.id), e, &mut out);
            }
        }
    }
    out
}

/// What a template names that the engine parses and does nothing with
/// until its task (A2: the director; A3: the ledger and the view; A4: the
/// stand-ins): the run is the same with or without it.
pub fn inert(a: &ArcDef) -> Vec<String> {
    let mut out = Vec::new();
    if a.opportunity {
        out.push("`opportunity`: the director times its arrival (A2)".to_string());
    }
    if a.sides.is_some() {
        out.push("`sides`: for the stand-ins (A4)".to_string());
    }
    if !a.answers.is_empty() {
        out.push("`answers`: NPC agendas favour them (A2)".to_string());
    }
    for s in &a.stages {
        if s.hidden {
            out.push(format!("stage `{}`: `hidden` (the ledger, A3)", s.id));
        }
        if s.climax {
            out.push(format!("stage `{}`: `climax` (the director, A2)", s.id));
        }
        if !s.goals.is_empty() {
            out.push(format!("stage `{}`: `goals` (NPC goals, A2)", s.id));
        }
        if s.beats.iter().any(|b| {
            b.teaches.is_some()
                || matches!(&b.kind, BeatKind::Talk { choice, .. }
                    if choice.replies.iter().any(|r| r.teaches.is_some()))
        }) {
            out.push(format!(
                "stage `{}`: `teaches` (the arc's ledger subject, A3)",
                s.id
            ));
        }
        if let Some(x) = &s.exit {
            if x.legacy_key.is_some() {
                out.push(format!("exit `{}`: `legacy_key` (A2)", s.id));
            }
            if x.follow_up.is_some() {
                out.push(format!("exit `{}`: `follow_up` (A2)", s.id));
            }
        }
    }
    out
}

// --- Validation -------------------------------------------------------------------------

/// The marks a list of conditions reads (`arc_mark`), `any` included.
fn marks_read<'a>(cs: &'a [Condition], out: &mut BTreeSet<&'a str>) {
    let mut all = Vec::new();
    crate::condition::walk(cs, &mut all);
    for c in all {
        if let Condition::ArcMark { mark, .. } = c {
            out.insert(mark.as_str());
        }
    }
}

struct Checker<'a> {
    b: &'a ContentBundle,
    a: &'a ArcDef,
    en: &'a BTreeSet<String>,
    en_text: &'a str,
    /// Arguments every key of this arc may use.
    provided: BTreeSet<String>,
    /// The people's role words (`$<role>_role`): only in text the view
    /// renders (a stage's story, a headline, a move), never in what the
    /// simulation's own commands hand back (a visit, a conversation).
    role_words: BTreeSet<String>,
    roles: Vec<&'a str>,
}

impl Checker<'_> {
    fn err(&self, issues: &mut Vec<Issue>, msg: String) {
        issues.push(Issue::error(&self.a.id, &msg));
    }

    /// A key the view renders: the cast's role words too.
    fn story(&self, key: &str, issues: &mut Vec<Issue>) {
        let words: Vec<&str> = self.role_words.iter().map(|s| s.as_str()).collect();
        self.key(key, &words, issues);
    }

    fn key(&self, key: &str, extra: &[&str], issues: &mut Vec<Issue>) {
        if !self.en.contains(key) {
            self.err(issues, format!("missing Fluent key `{key}` in locale en"));
            return;
        }
        for v in crate::asks::message_vars(self.en_text, key) {
            if !self.provided.contains(&v) && !extra.contains(&v.as_str()) {
                self.err(
                    issues,
                    format!("`{key}` uses `${v}`, which this arc does not provide"),
                );
            }
        }
    }

    fn role(&self, at: &str, role: &str, want: RoleKind, issues: &mut Vec<Issue>) {
        match self.a.role(role) {
            None => self.err(
                issues,
                format!("{at} names unknown role `{role}` (arc-role-uncast)"),
            ),
            Some(r) if !want.accepts(r.cast.kind()) => self.err(
                issues,
                format!("{at}: role `{role}` must be {}", want.word()),
            ),
            Some(_) => {}
        }
    }

    /// Conditions with the arc's roles as slots, checked for each tier.
    fn conditions(
        &self,
        at: ConditionUse,
        slots: &[&str],
        cs: &[Condition],
        issues: &mut Vec<Issue>,
    ) {
        for tier in &self.a.tiers {
            let site = Site::new(&self.a.id, at, slots).tier(*tier);
            let before = issues.len();
            crate::condition::check_all(self.b, &site, cs, issues);
            if issues.len() > before {
                // One tier's report is enough.
                break;
            }
        }
        // A run's conditions are decided inside the arcs' own step, where
        // the domain is not readable: they read the run, never other runs.
        if at != ConditionUse::ArcReply {
            let mut all = Vec::new();
            crate::condition::walk(cs, &mut all);
            if all.iter().any(|c| matches!(c, Condition::Arc { .. })) {
                self.err(
                    issues,
                    "an `arc` condition inside an arc: its own conditions read the run \
                     (`arc_mark`, `arc_heat`, `arc_days`), not other runs"
                        .into(),
                );
            }
        }
    }

    fn effects(&self, at: &str, es: &[GatedEffect], issues: &mut Vec<Issue>) {
        for e in es {
            self.conditions(ConditionUse::ArcGate, &self.roles, &e.when, issues);
            self.conditions(ConditionUse::ArcGate, &self.roles, &e.unless, issues);
            for (role, want) in e.effect.roles() {
                self.role(at, role, want, issues);
            }
            let bad = match &e.effect {
                ArcEffect::Opinion { delta, memory, .. } => {
                    if let Some(m) = memory
                        && !crate::types::MEMORY_KINDS.contains(&m.as_str())
                    {
                        self.err(issues, format!("{at}: unknown memory kind `{m}`"));
                    }
                    !(-100.0..=100.0).contains(delta)
                }
                ArcEffect::Approval { delta, classes, .. } => {
                    !(-1.0..=1.0).contains(delta) || classes.iter().any(|c| *c > 2)
                }
                ArcEffect::GovApproval { delta, .. } => !(-1.0..=1.0).contains(delta),
                ArcEffect::Record { delta } => !(-1.0..=1.0).contains(delta),
                ArcEffect::Standing { delta } => !(-100.0..=100.0).contains(delta),
                ArcEffect::Recovery { repair, months, .. } => {
                    !(1.0..=4.0).contains(repair) || *months == 0 || *months > 36
                }
                ArcEffect::Heat { delta } => !(-1.0..=1.0).contains(delta),
                ArcEffect::Favour { .. }
                | ArcEffect::Displacement { .. }
                | ArcEffect::PlaceState { .. } => false,
            };
            if bad {
                self.err(
                    issues,
                    format!("{at}: a `{}` effect is out of range", e.effect.key()),
                );
            }
        }
    }
}

pub(crate) fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    if b.arcs.is_empty() {
        return;
    }
    let en_text = b.locale_text("en");
    let en = crate::validate::fluent_keys(&en_text);
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for a in &b.arcs {
        let owner = a.id.as_str();
        let err = |issues: &mut Vec<Issue>, msg: String| issues.push(Issue::error(owner, &msg));
        if !ids.insert(owner) {
            err(issues, "arc ids must be unique".into());
        }
        if !a.id.starts_with("arc.") {
            err(issues, "arc ids start with `arc.`".into());
        }
        if a.tiers.is_empty() {
            err(issues, "an arc lists the tiers it may run in".into());
        }
        if a.cooldown_days > 36_500 || a.min_career_days > 3650 {
            err(
                issues,
                "cooldown_days and min_career_days are out of range".into(),
            );
        }
        if !(0.0..=1.0).contains(&a.heat.start)
            || a.heat
                .floor
                .as_ref()
                .is_some_and(|f| !(0.0..=100.0).contains(&f.scale))
        {
            err(
                issues,
                "heat start is 0..1 and a floor's scale 0..100".into(),
            );
        }
        for t in &a.answers {
            if !b.policies.contains_key(t) && b.family(t).is_none() {
                err(issues, format!("answers: unknown policy or family `{t}`"));
            }
        }

        // Roles: unique, and a `near` names an earlier role of the right kind.
        let mut roles: Vec<&str> = Vec::new();
        for r in &a.roles {
            if roles.contains(&r.id.as_str())
                || ["speaker", "about", "player"].contains(&r.id.as_str())
                || COMMON_ARGS.contains(&r.id.as_str())
            {
                err(
                    issues,
                    format!("role `{}` is taken (unique, and not a common name)", r.id),
                );
            }
            let near = |n: &Option<String>, issues: &mut Vec<Issue>| {
                if let Some(n) = n {
                    let ok = a
                        .roles
                        .iter()
                        .take_while(|x| x.id != r.id)
                        .any(|x| &x.id == n && x.cast.kind() == RoleKind::Area);
                    if !ok {
                        err(
                            issues,
                            format!("role `{}`: `near` names an earlier area role", r.id),
                        );
                    }
                }
            };
            match &r.cast {
                ArcCast::Occupation {
                    occupations,
                    near: n,
                } => {
                    if occupations.is_empty() {
                        err(issues, format!("role `{}` needs occupations", r.id));
                    }
                    for o in occupations {
                        if b.occupation(o).is_none() {
                            err(issues, format!("role `{}`: unknown occupation `{o}`", r.id));
                        }
                    }
                    near(n, issues);
                }
                ArcCast::Place { kinds, near: n } => {
                    if kinds.is_empty() {
                        err(issues, format!("role `{}` needs place kinds", r.id));
                    }
                    for k in kinds {
                        if b.place_kind(k).is_none()
                            || crate::types::PSEUDO_PLACES.contains(&k.as_str())
                        {
                            err(issues, format!("role `{}`: unknown place kind `{k}`", r.id));
                        }
                    }
                    near(n, issues);
                }
                ArcCast::Area { pick, hazard } => {
                    if *pick == AreaPick::MostExposed && hazard.is_none() {
                        err(
                            issues,
                            format!("role `{}`: `most_exposed` needs a hazard", r.id),
                        );
                    }
                }
                ArcCast::Post { .. } | ArcCast::Jurisdiction { .. } | ArcCast::Subject => {}
            }
            roles.push(&r.id);
        }
        if let Some(s) = &a.sides {
            for x in [&s.ally, &s.opponent] {
                if a.role(x).is_none() {
                    err(issues, format!("sides names unknown role `{x}`"));
                }
            }
        }

        // What the text may use: the cast, the common arguments, the facts'.
        let mut provided: BTreeSet<String> = COMMON_ARGS.iter().map(|s| s.to_string()).collect();
        let mut role_words = BTreeSet::new();
        for r in &a.roles {
            provided.insert(r.id.clone());
            if r.cast.kind() == RoleKind::Person {
                role_words.insert(format!("{}_role", r.id));
            }
        }
        for f in &a.facts {
            if let Some(p) = f.provides() {
                provided.extend(p);
            }
        }
        let ck = Checker {
            b,
            a,
            en: &en,
            en_text: &en_text,
            provided,
            role_words,
            roles: roles.clone(),
        };
        ck.conditions(ConditionUse::ArcRaise, &[], &a.raise, issues);
        if a.raise.is_empty() {
            err(
                issues,
                "an arc is raised by conditions over simulated state: `raise` is empty".into(),
            );
        }
        ck.conditions(ConditionUse::ArcGate, &roles, &a.given, issues);
        ck.conditions(ConditionUse::ArcFacts, &roles, &a.facts, issues);
        ck.story(&a.title_key, issues);

        // Stages: unique ids, a graph with exits, every stage ending on time.
        let n = a.stages.len();
        if n < 2 {
            err(issues, "an arc needs at least a stage and an exit".into());
        }
        let index = |id: &str| a.stages.iter().position(|s| s.id == id);
        let mut marks_set: BTreeSet<&str> = BTreeSet::new();
        let mut marks_seen: BTreeSet<&str> = BTreeSet::new();
        let mut out_edges: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (si, s) in a.stages.iter().enumerate() {
            let at = format!("stage `{}`", s.id);
            if a.stages.iter().take(si).any(|x| x.id == s.id) {
                err(issues, format!("{at}: stage ids must be unique"));
            }
            ck.story(&s.key, issues);
            if !(0.0..=0.5).contains(&s.rise) {
                err(issues, format!("{at}: rise is 0..0.5 a week"));
            }
            if let Some(c) = &s.clock {
                match (c.days, c.from) {
                    (Some([lo, hi]), None) => {
                        if lo == 0 || lo > hi || hi > 3650 {
                            err(issues, format!("{at}: clock days are 1..3650, min ≤ max"));
                        }
                    }
                    (None, Some(_)) => {}
                    _ => err(issues, format!("{at}: a clock has `days` or `from`")),
                }
            }
            for g in &s.goals {
                ck.role(&at, &g.role, RoleKind::Person, issues);
                if b.goal(&g.goal).is_none() {
                    err(issues, format!("{at}: unknown goal `{}`", g.goal));
                }
            }
            ck.effects(&at, &s.enter, issues);
            match &s.exit {
                Some(x) => {
                    if si == 0 {
                        err(issues, format!("{at}: the first stage is not an exit"));
                    }
                    if !s.next.is_empty() || !s.beats.is_empty() || s.clock.is_some() {
                        err(
                            issues,
                            format!("{at}: an exit has no beats, edges or clock"),
                        );
                    }
                    if x.effects.is_empty() {
                        err(
                            issues,
                            format!(
                                "{at}: an exit without effects (arc-exit-no-effects): an ending \
                                 has consequences the owning systems apply"
                            ),
                        );
                    }
                    ck.effects(&at, &x.effects, issues);
                    for g in &x.effects {
                        marks_read(&g.when, &mut marks_seen);
                        marks_read(&g.unless, &mut marks_seen);
                    }
                    for k in x.headline_key.iter().chain(x.legacy_key.iter()) {
                        ck.story(k, issues);
                    }
                    if let Some(f) = &x.follow_up
                        && !b.arcs.iter().any(|y| &y.id == f)
                    {
                        err(issues, format!("{at}: unknown follow-up arc `{f}`"));
                    }
                }
                None => {
                    if s.next.is_empty() {
                        err(issues, format!("{at}: a stage needs edges or an exit"));
                    }
                    // §1.4: every stage ends on time.
                    if s.next.last().is_some_and(|e| !e.on_time()) {
                        err(
                            issues,
                            format!(
                                "{at}: its last edge is not on time alone (arc-stuck): a stage \
                                 ends on its days or its clock whatever else happens"
                            ),
                        );
                    }
                }
            }
            for e in &s.next {
                match index(&e.to) {
                    Some(t) if t == si => err(issues, format!("{at}: an edge to itself")),
                    Some(t) => out_edges[si].push(t),
                    None => err(issues, format!("{at}: an edge to unknown stage `{}`", e.to)),
                }
                ck.conditions(ConditionUse::ArcEdge, &roles, &e.when, issues);
                marks_read(&e.when, &mut marks_seen);
                if e.when.is_empty() && e.after_days == 0 && e.clock_within_days.is_none() {
                    err(
                        issues,
                        format!("{at}: the edge to `{}` has no condition at all", e.to),
                    );
                }
                if e.after_days > 3650 || e.clock_within_days.is_some_and(|d| d > 3650) {
                    err(issues, format!("{at}: an edge's days are at most 3650"));
                }
            }
            // Beats.
            for (bi, beat) in s.beats.iter().enumerate() {
                let at = format!("beat `{}.{}`", s.id, beat.id);
                if s.beats.iter().take(bi).any(|x| x.id == beat.id) {
                    err(issues, format!("{at}: beat ids are unique in a stage"));
                }
                if beat.id.is_empty() || beat.id.contains('.') || s.id.contains('.') {
                    err(issues, format!("{at}: ids are not empty and have no dot"));
                }
                if !(0.0..=1.0).contains(&beat.relief) {
                    err(issues, format!("{at}: relief is 0..1"));
                }
                ck.conditions(ConditionUse::ArcGate, &roles, &beat.when, issues);
                marks_read(&beat.when, &mut marks_seen);
                ck.effects(&at, &beat.effects, issues);
                for g in &beat.effects {
                    marks_read(&g.when, &mut marks_seen);
                    marks_read(&g.unless, &mut marks_seen);
                }
                marks_set.extend(beat.sets.as_deref());
                match &beat.kind {
                    BeatKind::Decision { dilemma, on, .. } => match b.dilemma(dilemma) {
                        None => err(issues, format!("{at}: unknown dilemma `{dilemma}`")),
                        Some(d) => {
                            if !d.storyline_only {
                                err(
                                    issues,
                                    format!(
                                        "{at}: `{dilemma}` must be `storyline_only` (an arc's \
                                         decision is never spotlit on its own)"
                                    ),
                                );
                            }
                            for r in &d.roles {
                                match a.role(&r.id) {
                                    Some(x)
                                        if matches!(
                                            x.cast.kind(),
                                            RoleKind::Person | RoleKind::Area
                                        ) && !x.optional => {}
                                    _ => err(
                                        issues,
                                        format!(
                                            "{at}: `{dilemma}` casts role `{}`, which is not a \
                                             required person or area role of the arc",
                                            r.id
                                        ),
                                    ),
                                }
                            }
                            for (o, out) in on {
                                if !d.options.iter().any(|x| &x.id == o) {
                                    err(issues, format!("{at}: `{dilemma}` has no option `{o}`"));
                                }
                                if !(-1.0..=1.0).contains(&out.heat) {
                                    err(issues, format!("{at}: an option's heat is −1..1"));
                                }
                                ck.effects(&at, &out.effects, issues);
                                marks_set.extend(out.sets.as_deref());
                            }
                        }
                    },
                    BeatKind::Ask { template, asker } => {
                        ck.role(&at, asker, RoleKind::Person, issues);
                        match b.ask(template) {
                            None => err(issues, format!("{at}: unknown ask `{template}`")),
                            Some(t) => {
                                if t.from != [crate::asks::AskFrom::Arc] {
                                    err(
                                        issues,
                                        format!(
                                            "{at}: `{template}` must come `from: [\"arc\"]` only"
                                        ),
                                    );
                                }
                                for r in &t.roles {
                                    if a.role(&r.id)
                                        .is_some_and(|x| x.cast.kind() != RoleKind::Person)
                                    {
                                        err(
                                            issues,
                                            format!(
                                                "{at}: `{template}` shares role `{}` with the \
                                                 arc, where it is not a person",
                                                r.id
                                            ),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    BeatKind::Talk { with, choice } => {
                        ck.role(&at, with, RoleKind::Person, issues);
                        let talk = ["speaker", "player"];
                        ck.key(&choice.label_key, &talk, issues);
                        ck.key(&choice.say_key, &talk, issues);
                        if choice.replies.is_empty() {
                            err(issues, format!("{at}: a choice needs replies"));
                        }
                        if choice.replies.last().is_some_and(|r| !r.when.is_empty()) {
                            err(
                                issues,
                                format!("{at}: the last reply has no conditions (always one)"),
                            );
                        }
                        if !choice.replies.iter().any(|r| r.agrees) {
                            err(issues, format!("{at}: no reply agrees"));
                        }
                        let slots: Vec<&str> = talk.iter().copied().chain(roles.clone()).collect();
                        for r in &choice.replies {
                            ck.key(&r.key, &talk, issues);
                            ck.conditions(ConditionUse::ArcReply, &slots, &r.when, issues);
                            marks_read(&r.when, &mut marks_seen);
                            ck.effects(&at, &r.effects, issues);
                            marks_set.extend(r.sets.as_deref());
                        }
                    }
                    BeatKind::Visit {
                        at: place,
                        verb_key,
                        outcome_key,
                        ..
                    } => {
                        ck.role(&at, place, RoleKind::Spot, issues);
                        // The place's verb list has no arguments.
                        if en.contains(verb_key)
                            && !crate::asks::message_vars(&en_text, verb_key).is_empty()
                        {
                            err(
                                issues,
                                format!("{at}: `{verb_key}` is a button: it takes no arguments"),
                            );
                        }
                        ck.key(verb_key, &[], issues);
                        ck.key(outcome_key, &[], issues);
                    }
                    BeatKind::Press { headline_key } => ck.story(headline_key, issues),
                    BeatKind::Heard { line } => match b.overheard.iter().find(|l| &l.id == line) {
                        None => err(issues, format!("{at}: unknown overheard line `{line}`")),
                        Some(l) => {
                            let mut all = Vec::new();
                            crate::condition::walk(&l.when, &mut all);
                            let names = all.iter().any(|c| {
                                matches!(c, Condition::Arc { arc, stages, .. }
                                        if arc == &a.id && stages.contains(&s.id))
                            });
                            if !names {
                                err(
                                    issues,
                                    format!(
                                        "{at}: `{line}` has no `arc` condition naming this \
                                             stage (it would be heard whatever the arc does)"
                                    ),
                                );
                            }
                        }
                    },
                    BeatKind::Move {
                        actor,
                        weight,
                        line_key,
                    } => {
                        ck.role(&at, actor, RoleKind::Person, issues);
                        if !(0.0..=1.0).contains(weight) {
                            err(issues, format!("{at}: a move's weekly weight is 0..1"));
                        }
                        ck.story(line_key, issues);
                    }
                }
            }
        }

        // The graph: every stage reachable, an exit reachable from each,
        // a clock in force wherever an edge reads one.
        let mut reach = vec![false; n];
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            if i >= n || std::mem::replace(&mut reach[i], true) {
                continue;
            }
            stack.extend(out_edges[i].iter().copied());
        }
        let mut exits = vec![false; n];
        for (i, s) in a.stages.iter().enumerate() {
            exits[i] = s.exit.is_some();
        }
        loop {
            let mut changed = false;
            for i in 0..n {
                if !exits[i] && out_edges[i].iter().any(|t| exits[*t]) {
                    exits[i] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        // A clock is in force in a stage that sets one, or whose every
        // (reachable) predecessor has one.
        let mut clocked: Vec<bool> = a.stages.iter().map(|_| true).collect();
        loop {
            let mut changed = false;
            for i in 0..n {
                if a.stages[i].clock.is_some() || !clocked[i] {
                    continue;
                }
                let preds: Vec<usize> = (0..n)
                    .filter(|p| reach[*p] && out_edges[*p].contains(&i))
                    .collect();
                if i == 0 || preds.iter().any(|p| !clocked[*p]) {
                    clocked[i] = false;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for (i, s) in a.stages.iter().enumerate() {
            if !reach[i] {
                err(
                    issues,
                    format!(
                        "stage `{}` cannot be reached from the first (arc-unreachable-stage)",
                        s.id
                    ),
                );
            } else if !exits[i] {
                err(
                    issues,
                    format!(
                        "no exit can be reached from stage `{}` (arc-no-exit-from)",
                        s.id
                    ),
                );
            }
            if !clocked[i] && s.next.iter().any(|e| e.clock_within_days.is_some()) {
                err(
                    issues,
                    format!(
                        "stage `{}` has an edge on the clock, and no clock is in force there",
                        s.id
                    ),
                );
            }
        }
        let exit_count = a.stages.iter().filter(|s| s.exit.is_some()).count();
        if exit_count == 0 {
            err(issues, "an arc needs an exit".into());
        } else if exit_count < 3 {
            issues.push(Issue::warning(
                owner,
                &format!("{exit_count} exits: an arc has at least three (arc-few-exits)"),
            ));
        }
        // Marks: one read is set somewhere; one set is read somewhere.
        for m in marks_seen.difference(&marks_set) {
            err(
                issues,
                format!("mark `{m}` is read and never set (arc-mark-unset)"),
            );
        }
        for m in marks_set.difference(&marks_seen) {
            issues.push(Issue::warning(
                owner,
                &format!("mark `{m}` is set and never read (arc-mark-unread)"),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> ArcDef {
        serde_json::from_str(s).unwrap_or_else(|e| panic!("{e}"))
    }

    const MINI: &str = r#"{
      "id": "arc.mini", "title_key": "arc-mini", "tiers": ["local"],
      "raise": [ { "type": "disaster", "within_months": 1 } ],
      "roles": [ { "id": "plant", "cast": { "type": "subject" } },
                 { "id": "hood", "cast": { "type": "area", "pick": "least_pleased" } } ],
      "stages": [
        { "id": "a", "key": "arc-mini-a", "clock": { "from": "subject" },
          "beats": [ { "id": "seen", "type": "visit", "at": "hood", "verb_key": "v", "outcome_key": "o",
                       "sets": "seen", "effects": [ { "type": "gov_approval", "area": "hood", "delta": 0.01 } ] } ],
          "next": [ { "to": "z", "after_days": 7 } ] },
        { "id": "z", "key": "arc-mini-z", "exit": { "effects": [
            { "when": [ { "type": "arc_mark", "mark": "seen" } ], "type": "place_state", "place": "hood", "state": "marked" } ] } }
      ] }"#;

    #[test]
    fn a_gated_effect_reads_flat() {
        let a = parse(MINI);
        let x = a.stage("z").and_then(|s| s.exit.as_ref()).unwrap();
        assert_eq!(x.effects[0].when.len(), 1);
        assert!(matches!(x.effects[0].effect, ArcEffect::PlaceState { .. }));
        let b = &a.stage("a").unwrap().beats[0];
        assert!(b.once, "a beat is done once unless it says otherwise");
        assert!(matches!(b.kind, BeatKind::Visit { .. }));
        // And it writes back as it reads.
        let again: ArcDef = serde_json::from_value(serde_json::to_value(&a).unwrap()).unwrap();
        assert_eq!(a, again);
    }

    /// The parts of the design the engine does not do yet: a template that
    /// uses one is parsed and validated, and never raised.
    #[test]
    fn not_yet_names_what_the_engine_lacks() {
        let a = parse(MINI);
        let n = not_yet(&a);
        assert_eq!(n.len(), 4, "{n:#?}");
        assert!(n[0].contains("`subject` cast"));
        assert!(n[1].contains("clock read from the subject"));
        assert!(n[2].contains("`gov_approval`"));
        assert!(n[3].contains("`place_state`"));
        let mut b = a.clone();
        b.requires = vec!["parties.membership".into()];
        assert!(not_yet(&b)[0].contains("parties.membership"));
    }
}
