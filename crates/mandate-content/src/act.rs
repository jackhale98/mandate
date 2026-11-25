//! The act flow (DEC-005, DEC-012, DEC-006, DEC-098, DEC-100): what a time
//! skip offers and obeys, and how an act ends.
//!
//! - **Directives** (`act_flow.jsonc` `directives`): the standing orders a
//!   player sets for a skip. Each steers one *lever* of the player-proxy
//!   (an engine contract like the constitution's procedure ids: focus,
//!   voting stance, effort on the ground game, fundraising, the press and
//!   legislation, caution). Options carry the numbers the proxy reads.
//! - **Interrupts** (`interrupts`): what stops a skip and hands play back,
//!   each with its threshold.
//! - **Skip targets** (`targets`): the points the engine offers to skip to.
//! - **The proxy's and the Legacy record's numbers** (`proxy`, `legacy`).
//! - **Endings** (`endings.jsonc`): the conditions that end an act, each
//!   with its Fluent keys and the facts its epilogue reports. Conditions
//!   read posts, powers and acts from the constitution, never office names.
//!
//! Nothing here names an office or a body (DEC-098): a regional or national
//! act works by data alone.

use crate::ContentBundle;
use crate::media::Gravity;
use crate::types::{ContentKey, Power};
use crate::validate::{Issue, valid_key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What a directive steers in the proxy (an engine contract).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DirectiveLever {
    /// Which worry the ground game and new bills aim at (`concern`; none:
    /// the strongest the player knows of).
    Focus,
    /// How the player votes on bills (`vote`).
    Vote,
    /// Effort on doorsteps and public meetings (`weight`).
    Ground,
    /// Effort on raising money (`weight`).
    Fundraising,
    /// Effort on statements to the press (`weight`).
    Press,
    /// Effort on drafting and steering bills (`weight`).
    Legislation,
    /// How cautious to be: the largest shown risk of a hidden act accepted
    /// (`max_risk`) and the health below which the proxy rests (`rest_below`).
    Caution,
}

impl DirectiveLever {
    pub fn key(self) -> &'static str {
        match self {
            DirectiveLever::Focus => "focus",
            DirectiveLever::Vote => "vote",
            DirectiveLever::Ground => "ground",
            DirectiveLever::Fundraising => "fundraising",
            DirectiveLever::Press => "press",
            DirectiveLever::Legislation => "legislation",
            DirectiveLever::Caution => "caution",
        }
    }
    /// An effort lever (its options carry a `weight`).
    pub fn is_effort(self) -> bool {
        matches!(
            self,
            DirectiveLever::Ground
                | DirectiveLever::Fundraising
                | DirectiveLever::Press
                | DirectiveLever::Legislation
        )
    }
}

/// How the proxy votes on a bill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoteStance {
    /// With the party's line (abstain when it has none).
    WithParty,
    /// By the player's own convictions (their attitudes against the bill's
    /// signature).
    Conviction,
    /// By the ward's view, as the player knows it.
    Ward,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DirectiveOptionDef {
    pub id: String,
    pub name_key: String,
    /// Effort levers: the relative share of the turn's slots (0 = none).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<u8>,
    /// `vote`: the stance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vote: Option<VoteStance>,
    /// `caution`: the largest shown chance (0..1) that a hidden act comes
    /// out which the proxy still accepts; 0 avoids every shady option.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_risk: Option<f32>,
    /// `caution`: the proxy rests when health is below this (0..100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rest_below: Option<f32>,
    /// `focus`: a concern id; none is the strongest worry the player knows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concern: Option<ContentKey>,
}

/// A standing directive for a time skip (DEC-012).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DirectiveDef {
    /// `directive.<name>`.
    pub id: String,
    pub name_key: String,
    pub help_key: String,
    pub lever: DirectiveLever,
    pub options: Vec<DirectiveOptionDef>,
    /// The option a new run starts with.
    pub default: String,
    /// `focus` only: also offer one option per concern in the registry (its
    /// id is the concern's id, its name the concern's `name_key`).
    #[serde(default)]
    pub concern_options: bool,
}

impl DirectiveDef {
    pub fn option(&self, id: &str) -> Option<&DirectiveOptionDef> {
        self.options.iter().find(|o| o.id == id)
    }
}

/// A pressure track (DEC-006), or Mandate.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TrackKey {
    Viability,
    LegalJeopardy,
    Threat,
    Standing,
    Health,
    Mandate,
}

impl TrackKey {
    pub fn key(self) -> &'static str {
        match self {
            TrackKey::Viability => "viability",
            TrackKey::LegalJeopardy => "legal_jeopardy",
            TrackKey::Threat => "threat",
            TrackKey::Standing => "standing",
            TrackKey::Health => "health",
            TrackKey::Mandate => "mandate",
        }
    }
}

/// A decision's weight in the director's budgets.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum WeightKey {
    Climax,
    Significant,
    Minor,
}

/// What stops a time skip (DEC-012 interrupts). Interrupts fire on a
/// change during the skip: what was already so when it began does not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum InterruptWhen {
    /// A track crosses `threshold`: rises to it or above, or with `below`
    /// falls to it or under.
    Track {
        track: TrackKey,
        threshold: f32,
        #[serde(default)]
        below: bool,
    },
    /// A decision opens with one of these weights, or (`about`) one that
    /// answers a story or an investigation of the player.
    Decision {
        #[serde(default)]
        weights: Vec<WeightKey>,
        #[serde(default)]
        about: bool,
    },
    /// A story about the player breaks, at least this grave.
    Scandal {
        #[serde(default)]
        min_severity: Gravity,
    },
    /// An election the player stands in is called (they are a candidate).
    Election,
    /// A bill the player sponsors reaches a vote within `days`.
    SponsoredVote { days: u16 },
    /// Someone's request of the player (or a promise to them) falls due
    /// within `days`. With `offers`, an ask still on offer counts too: it
    /// waits for the player's answer and stops the skip as its last day to
    /// answer nears (DEC-102 clause 7). Without, an offer left unanswered
    /// through a skip lapses as declined. `big_offers` counts only the big
    /// asks on offer: a small one may lapse, a big one is the player's to
    /// answer.
    RequestDue {
        days: u16,
        #[serde(default)]
        offers: bool,
        #[serde(default)]
        big_offers: bool,
    },
    /// A staged arc in the player's focus reaches a moment that is the
    /// player's to meet (DEC-102 clause 7). Which moments, by the kind of
    /// arc: `material` for an arc that is not standing (a flood, a plant),
    /// `standing` for one the calendar brings round (the budget year, a
    /// campaign's rival). A hidden stage never stops a skip. The message is
    /// given `arc` (the template's id), `stage` and `run`.
    ArcStage {
        material: ArcStops,
        standing: ArcStops,
    },
    /// The run's status changes (prison, release, the end of the career).
    Status,
}

/// Which stages of an arc stop a skip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArcStopStages {
    /// None.
    #[default]
    None,
    /// Its turn: the stage marked `climax`.
    Turn,
    /// Each one.
    Every,
}

/// The moments of one kind of arc that stop a skip (`InterruptWhen::ArcStage`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArcStops {
    /// Being raised stops a skip, whatever its first stage offers.
    #[serde(default)]
    pub raised: bool,
    /// Which of its stages do.
    #[serde(default)]
    pub stages: ArcStopStages,
    /// A stage counts only for a run that leads the chapter, or one the
    /// player can act in now (a beat of the stage is theirs to take: they
    /// hold the power its decision needs, they are in the race).
    #[serde(default)]
    pub lead_or_actor: bool,
    /// A stage with nothing for the player to do stops a skip too, and so
    /// does an exit. Without (the default), a stage stops a skip once a
    /// beat of it is the player's to take, and an exit never does.
    #[serde(default)]
    pub idle: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InterruptDef {
    /// `interrupt.<name>`.
    pub id: String,
    /// Fluent key of the reason shown ("The skip stopped: …").
    pub key: String,
    pub when: InterruptWhen,
}

/// A point a skip can run to (an engine contract).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SkipTargetKind {
    /// The start of the next chapter (a new session, or the campaign).
    NextSession,
    /// The opening of the campaign for the player's post.
    Campaign,
    /// Election day for the player's post.
    Election,
    /// The end of the player's term.
    EndOfTerm,
}

impl SkipTargetKind {
    pub fn key(self) -> &'static str {
        match self {
            SkipTargetKind::NextSession => "next_session",
            SkipTargetKind::Campaign => "campaign",
            SkipTargetKind::Election => "election",
            SkipTargetKind::EndOfTerm => "end_of_term",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkipTargetDef {
    pub target: SkipTargetKind,
    pub name_key: String,
}

/// The player-proxy's numbers (DEC-012). It is kept modest on purpose: a
/// skip is a convenience, never the best way to play (DEC-097).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProxyParams {
    /// Decision weights the proxy answers; others it leaves to the player
    /// (or to their default at the deadline).
    pub answers: Vec<WeightKey>,
    /// At most this many commands a turn.
    pub max_commands: u8,
    /// The share of slots it leaves unused (a proxy is not the player).
    pub idle_share: f64,
    /// Whether what the proxy's acts would teach is kept (DEC-102 clause 7:
    /// a skip learns nothing new, so `false`). With `true` a skipped
    /// canvass fills the notebook as the player's own would.
    #[serde(default)]
    pub learns: bool,
}

/// The Legacy record's settings (DEC-006, DEC-100: a record only).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LegacyParams {
    /// Laws whose budget category counts them among the player's works.
    pub works_categories: Vec<crate::types::BudgetCategory>,
    /// Decisions of these weights the player answered are notable.
    pub notable: Vec<WeightKey>,
    /// Entries kept (the oldest notable decisions go first).
    pub max_entries: u16,
}

/// The act flow (DEC-005, DEC-012): directives, interrupts, skip targets,
/// the proxy and the Legacy record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActFlowParams {
    pub directives: Vec<DirectiveDef>,
    pub interrupts: Vec<InterruptDef>,
    pub targets: Vec<SkipTargetDef>,
    /// The longest skip, in turns.
    pub max_turns: u16,
    pub proxy: ProxyParams,
    pub legacy: LegacyParams,
}

impl Default for ActFlowParams {
    fn default() -> Self {
        ActFlowParams {
            directives: Vec::new(),
            interrupts: Vec::new(),
            targets: Vec::new(),
            max_turns: 0,
            proxy: ProxyParams {
                answers: Vec::new(),
                max_commands: 0,
                idle_share: 0.0,
                learns: false,
            },
            legacy: LegacyParams {
                works_categories: Vec::new(),
                notable: Vec::new(),
                max_entries: 0,
            },
        }
    }
}

impl ActFlowParams {
    pub fn directive(&self, id: &str) -> Option<&DirectiveDef> {
        self.directives.iter().find(|d| d.id == id)
    }
    pub fn interrupt(&self, id: &str) -> Option<&InterruptDef> {
        self.interrupts.iter().find(|d| d.id == id)
    }
}

/// A run status an ending can read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatusKey {
    Imprisoned,
    Ended,
}

/// Events about the player an ending can read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlayerEventKey {
    /// The party would not renominate them.
    Deselected,
    /// A sentence removed them from office.
    RemovedFromOffice,
    Imprisoned,
}

/// When an act ends (DEC-006; DEC-098 note). Posts are read through the
/// constitution: its acts, its powers, its terms.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum EndingWhen {
    /// The player holds a post of a later act of the career ladder.
    HoldsLaterAct,
    /// The player holds a post of this act with all these powers, which
    /// they did not hold when the act began (e.g. the tier's executive).
    WonPostWith { powers: Vec<Power> },
    /// The player stood in an election this act, lost, and holds no post.
    LostSeat,
    /// Something happened to the player this act.
    Event { event: PlayerEventKey },
    /// The run's status is one of these.
    Status { status: Vec<StatusKey> },
    /// A track at or under `value` (disgrace, the political wilderness).
    TrackAtMost { track: TrackKey, value: f32 },
    /// The act has lasted `terms` full terms of the post the player holds
    /// (its `term_years` in the constitution), and they hold it.
    Served { terms: u8 },
    /// Any of these.
    Any { any: Vec<EndingWhen> },
}

/// A fact an epilogue reports, computed by the engine.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum EndingFact {
    /// Posts held this act.
    PostsHeld,
    /// Years the act lasted.
    Years,
    LawsPassed,
    /// Laws in a works category (`legacy.works_categories`).
    Works,
    ElectionsWon,
    ElectionsLost,
    PromisesKept,
    PromisesBroken,
    /// Notable decisions taken.
    Decisions,
    Scandals,
    Mandate,
    Standing,
    Viability,
}

impl EndingFact {
    pub fn key(self) -> &'static str {
        match self {
            EndingFact::PostsHeld => "posts_held",
            EndingFact::Years => "years",
            EndingFact::LawsPassed => "laws_passed",
            EndingFact::Works => "works",
            EndingFact::ElectionsWon => "elections_won",
            EndingFact::ElectionsLost => "elections_lost",
            EndingFact::PromisesKept => "promises_kept",
            EndingFact::PromisesBroken => "promises_broken",
            EndingFact::Decisions => "decisions",
            EndingFact::Scandals => "scandals",
            EndingFact::Mandate => "mandate",
            EndingFact::Standing => "standing",
            EndingFact::Viability => "viability",
        }
    }
}

/// How an ending reads on the epilogue screen (presentation only).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndingTone {
    Triumph,
    Neutral,
    Defeat,
}

/// One way an act ends (DEC-006; the endings registry, DEC-098 note). The
/// first entry whose condition holds ends the act.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndingDef {
    /// `ending.<name>`.
    pub id: String,
    /// Fluent key of the ending's title.
    pub title_key: String,
    /// Fluent key of the epilogue's text (arguments: the facts by key).
    pub text_key: String,
    /// Act ids (the constitution's `career.acts`) it can end; empty: any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acts: Vec<String>,
    pub when: EndingWhen,
    /// What the epilogue reports, in order.
    pub facts: Vec<EndingFact>,
    pub tone: EndingTone,
}

/// The Fluent keys the act flow and the endings name.
pub fn fluent_refs(b: &ContentBundle) -> Vec<(&str, &str)> {
    let a = &b.act_flow;
    let mut out = Vec::new();
    for d in &a.directives {
        out.push((d.id.as_str(), d.name_key.as_str()));
        out.push((d.id.as_str(), d.help_key.as_str()));
        for o in &d.options {
            out.push((d.id.as_str(), o.name_key.as_str()));
        }
    }
    for i in &a.interrupts {
        out.push((i.id.as_str(), i.key.as_str()));
    }
    for t in &a.targets {
        out.push(("act_flow.targets", t.name_key.as_str()));
    }
    for e in &b.endings {
        out.push((e.id.as_str(), e.title_key.as_str()));
        out.push((e.id.as_str(), e.text_key.as_str()));
    }
    out
}

fn check_when(id: &str, w: &EndingWhen, issues: &mut Vec<Issue>) {
    match w {
        EndingWhen::WonPostWith { powers } if powers.is_empty() => {
            issues.push(Issue::error(id, "`won_post_with` needs at least one power"))
        }
        EndingWhen::Status { status } if status.is_empty() => {
            issues.push(Issue::error(id, "`status` needs at least one status"))
        }
        EndingWhen::TrackAtMost { value, .. } if !(0.0..=100.0).contains(value) => {
            issues.push(Issue::error(id, "a track value must be in [0, 100]"))
        }
        EndingWhen::Served { terms } if *terms == 0 => {
            issues.push(Issue::error(id, "`served` needs at least one term"))
        }
        EndingWhen::Any { any } => {
            if any.is_empty() {
                issues.push(Issue::error(id, "`any` needs at least one condition"));
            }
            for x in any {
                check_when(id, x, issues);
            }
        }
        _ => {}
    }
}

/// Shape rules the schema cannot express.
pub fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let a = &b.act_flow;
    let mut seen: Vec<&str> = Vec::new();
    let mut levers: Vec<DirectiveLever> = Vec::new();
    for d in &a.directives {
        let id = d.id.as_str();
        let mut err = |m: &str| issues.push(Issue::error(id, m));
        if !valid_key(id) || !id.starts_with("directive.") {
            err("a directive id must be a key starting with `directive.`");
        }
        if seen.contains(&id) {
            err("duplicate directive id");
        }
        seen.push(id);
        if levers.contains(&d.lever) {
            err(&format!(
                "a second directive for the `{}` lever",
                d.lever.key()
            ));
        }
        levers.push(d.lever);
        if d.options.is_empty() {
            err("a directive needs at least one option");
        }
        if d.concern_options && d.lever != DirectiveLever::Focus {
            err("`concern_options` is for the `focus` lever only");
        }
        let mut ids: Vec<&str> = Vec::new();
        for o in &d.options {
            if ids.contains(&o.id.as_str()) {
                err(&format!("duplicate option `{}`", o.id));
            }
            ids.push(&o.id);
            if d.concern_options && b.concern_keys().any(|k| *k == o.id) {
                err(&format!("option `{}` clashes with a concern id", o.id));
            }
            let has = |x: bool, what: &str, err: &mut dyn FnMut(&str)| {
                if !x {
                    err(&format!("option `{}` needs `{what}`", o.id));
                }
            };
            match d.lever {
                l if l.is_effort() => has(o.weight.is_some(), "weight", &mut err),
                DirectiveLever::Vote => has(o.vote.is_some(), "vote", &mut err),
                DirectiveLever::Caution => {
                    has(o.max_risk.is_some(), "max_risk", &mut err);
                    has(o.rest_below.is_some(), "rest_below", &mut err);
                }
                _ => {}
            }
            if o.weight.is_some_and(|w| w > 3) {
                err(&format!("option `{}`: weight must be 0..3", o.id));
            }
            if o.max_risk.is_some_and(|r| !(0.0..=1.0).contains(&r)) {
                err(&format!("option `{}`: max_risk must be in [0, 1]", o.id));
            }
            if o.rest_below.is_some_and(|r| !(0.0..=100.0).contains(&r)) {
                err(&format!(
                    "option `{}`: rest_below must be in [0, 100]",
                    o.id
                ));
            }
            if let Some(c) = &o.concern
                && !b.concern_keys().any(|k| k == c)
            {
                err(&format!("option `{}`: unknown concern `{c}`", o.id));
            }
        }
        let default_ok = d.option(&d.default).is_some()
            || (d.concern_options && b.concern_keys().any(|k| *k == d.default));
        if !default_ok {
            err(&format!("default `{}` is not an option", d.default));
        }
    }
    let mut ids: Vec<&str> = Vec::new();
    for i in &a.interrupts {
        let id = i.id.as_str();
        if !valid_key(id) || !id.starts_with("interrupt.") {
            issues.push(Issue::error(
                id,
                "an interrupt id must be a key starting with `interrupt.`",
            ));
        }
        if ids.contains(&id) {
            issues.push(Issue::error(id, "duplicate interrupt id"));
        }
        ids.push(id);
        match &i.when {
            InterruptWhen::Track { threshold, .. } if !(0.0..=100.0).contains(threshold) => {
                issues.push(Issue::error(id, "a track threshold must be in [0, 100]"))
            }
            InterruptWhen::Decision { weights, about } if weights.is_empty() && !about => issues
                .push(Issue::error(
                    id,
                    "a decision interrupt needs weights or `about`",
                )),
            _ => {}
        }
    }
    let mut kinds: Vec<SkipTargetKind> = Vec::new();
    for t in &a.targets {
        if kinds.contains(&t.target) {
            issues.push(Issue::error(
                "act_flow.targets",
                &format!("target `{}` listed twice", t.target.key()),
            ));
        }
        kinds.push(t.target);
    }
    if !a.directives.is_empty() && a.max_turns == 0 {
        issues.push(Issue::error("act_flow", "max_turns must be at least 1"));
    }
    if !(0.0..=0.9).contains(&a.proxy.idle_share) {
        issues.push(Issue::error(
            "act_flow.proxy",
            "idle_share must be in [0, 0.9]",
        ));
    }
    let mut ids: Vec<&str> = Vec::new();
    for e in &b.endings {
        let id = e.id.as_str();
        if !valid_key(id) || !id.starts_with("ending.") {
            issues.push(Issue::error(
                id,
                "an ending id must be a key starting with `ending.`",
            ));
        }
        if ids.contains(&id) {
            issues.push(Issue::error(id, "duplicate ending id"));
        }
        ids.push(id);
        check_when(id, &e.when, issues);
        // Acts are the constitutions' (DEC-098): every one named must exist
        // in at least one constitution.
        for act in &e.acts {
            let known = b
                .constitutions
                .values()
                .any(|c| c.career.acts.iter().any(|x| &x.id == act));
            if !known {
                issues.push(Issue::error(
                    id,
                    &format!("unknown act `{act}` (no constitution's career has it)"),
                ));
            }
        }
    }
}
