//! Knowledge is earned (DEC-091): the content side of the player's knowledge
//! ledger. Start profiles (what a background knows when play begins), the
//! "Start with local knowledge" option, the party contact's primer on
//! entering a new zone, first-visit introductions by place kind, and the
//! numbers (staleness, the lead cap, and the effectiveness terms).
//!
//! A grant names subjects by a kind and a selector, never by id: the engine
//! resolves them against the live town (`mandate_sim::player::knowledge`).
//! The ledger itself, its levels and subjects, is engine state.

use crate::ContentBundle;
use crate::types::ContentKey;
use crate::validate::Issue;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How much the player knows of a subject. Ordered; a subject's level only
/// ever rises (identities never fade, DEC-091).
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Nothing.
    #[default]
    Unknown,
    /// A rumour or a name.
    Heard,
    /// Met or seen first-hand.
    Known,
    /// From the right place or person.
    Detailed,
}

impl Level {
    pub const ALL: [Level; 4] = [Level::Unknown, Level::Heard, Level::Known, Level::Detailed];

    pub fn key(self) -> &'static str {
        match self {
            Level::Unknown => "unknown",
            Level::Heard => "heard",
            Level::Known => "known",
            Level::Detailed => "detailed",
        }
    }

    /// The next level up (`Detailed` stays).
    pub fn next(self) -> Level {
        match self {
            Level::Unknown => Level::Heard,
            Level::Heard => Level::Known,
            Level::Known | Level::Detailed => Level::Detailed,
        }
    }
}

/// The kind of subject a grant gives.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GrantSubject {
    /// The town's worries (`which`: `town`).
    TownConcerns,
    /// Neighbourhoods' worries (`which`: `home`, `ward`, `town`, `here`).
    HoodConcerns,
    /// Who really runs things (`which`: `town`; `here`: only if the broker
    /// is found at the place).
    Broker,
    /// Every councillor and the mayor, as people (`which`: `town`).
    Council,
    /// The people found at places (`which`: `town` for every place, `here`,
    /// or a place kind such as `union_hall`).
    PeopleAt,
    /// The rival's plans (`which`: `town`).
    RivalPlans,
    /// Who supports each standard policy (`which`: `town`).
    PolicySupport,
    /// The proposals on the table (`which`: `town`).
    Proposals,
}

impl GrantSubject {
    pub fn key(self) -> &'static str {
        match self {
            GrantSubject::TownConcerns => "town_concerns",
            GrantSubject::HoodConcerns => "hood_concerns",
            GrantSubject::Broker => "broker",
            GrantSubject::Council => "council",
            GrantSubject::PeopleAt => "people_at",
            GrantSubject::RivalPlans => "rival_plans",
            GrantSubject::PolicySupport => "policy_support",
            GrantSubject::Proposals => "proposals",
        }
    }

    /// The selectors this kind accepts (besides a place kind for `people_at`).
    pub fn selectors(self) -> &'static [&'static str] {
        match self {
            GrantSubject::HoodConcerns => &["home", "ward", "town", "here"],
            GrantSubject::Broker => &["town", "here"],
            GrantSubject::PeopleAt => &["town", "here"],
            _ => &["town"],
        }
    }
}

/// One grant: every subject of a kind that a selector picks, raised to a
/// level. Selectors: `home` (the player's home neighbourhood), `ward` (the
/// neighbourhoods of the player's ward), `town` (the zone's town), `here`
/// (the place being introduced: introductions only), or a place kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GrantDef {
    pub subject: GrantSubject,
    pub which: String,
    pub level: Level,
}

/// What a background knows when play begins (DEC-091).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StartProfile {
    /// Content id, e.g. `start.grew_up_here`.
    pub id: ContentKey,
    pub grants: Vec<GrantDef>,
}

/// A number per knowledge level.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ByLevel {
    pub unknown: f64,
    pub heard: f64,
    pub known: f64,
    pub detailed: f64,
}

impl ByLevel {
    pub fn get(&self, l: Level) -> f64 {
        match l {
            Level::Unknown => self.unknown,
            Level::Heard => self.heard,
            Level::Known => self.known,
            Level::Detailed => self.detailed,
        }
    }
}

/// Canvassing on an issue (DEC-091 "ignorance costs effectiveness"): the
/// approval gain is multiplied by `1 + match_bonus × (salience − 1)`, at
/// least `floor`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CanvassMatch {
    pub match_bonus: f64,
    pub floor: f64,
}

/// The effectiveness terms, applied symmetrically by the shared formulas.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeEffects {
    /// A logit term on lobbying odds by the actor's knowledge of the
    /// target as a person (NPCs, who keep no ledger, use `known`).
    pub lobby_logit: ByLevel,
    pub canvass: CanvassMatch,
}

/// The knowledge parameter block (`kind: "knowledge"`, one file across packs).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeParams {
    /// Start profiles. The draft of three (DEC-036) names one per
    /// background; until then every run uses `default_start`.
    pub starts: Vec<StartProfile>,
    pub default_start: ContentKey,
    /// "Start with local knowledge" (off by default), applied on top of the
    /// profile. Never `detailed`, never hidden places.
    pub local_option: Vec<GrantDef>,
    /// A party contact's primer on entering a new zone.
    pub primer: Vec<GrantDef>,
    /// Given at the first visit to any place in a zone (beside its kind's
    /// introduction).
    #[serde(default)]
    pub visit_all: Vec<GrantDef>,
    /// First-visit introductions by place kind (`union_hall`, …).
    #[serde(default)]
    pub visit_intro: BTreeMap<String, Vec<GrantDef>>,
    /// A reading older than this many days is shown as stale.
    pub fresh_days: u32,
    /// Leads kept at once (oldest dropped first).
    pub leads_cap: u32,
    pub effects: KnowledgeEffects,
}

impl KnowledgeParams {
    pub fn start(&self, id: &str) -> Option<&StartProfile> {
        self.starts.iter().find(|s| s.id == id)
    }
}

/// Knowledge with no profiles: what a bundle holds before its file loads.
impl Default for KnowledgeParams {
    fn default() -> Self {
        KnowledgeParams {
            starts: Vec::new(),
            default_start: String::new(),
            local_option: Vec::new(),
            primer: Vec::new(),
            visit_all: Vec::new(),
            visit_intro: BTreeMap::new(),
            fresh_days: 90,
            leads_cap: 24,
            effects: KnowledgeEffects {
                lobby_logit: ByLevel {
                    unknown: 0.0,
                    heard: 0.0,
                    known: 0.0,
                    detailed: 0.0,
                },
                canvass: CanvassMatch {
                    match_bonus: 0.0,
                    floor: 1.0,
                },
            },
        }
    }
}

/// Range checks for the knowledge block (DEC-078 decision 1; `--set` paths).
pub fn param_ranges(b: &ContentBundle, r: &mut dyn FnMut(&str, f64, f64, f64)) {
    let k = &b.knowledge;
    r("knowledge.fresh_days", k.fresh_days.into(), 1.0, 3650.0);
    r("knowledge.leads_cap", k.leads_cap.into(), 1.0, 200.0);
    for l in Level::ALL {
        r(
            &format!("knowledge.effects.lobby_logit.{}", l.key()),
            k.effects.lobby_logit.get(l),
            -3.0,
            3.0,
        );
    }
    r(
        "knowledge.effects.canvass.match_bonus",
        k.effects.canvass.match_bonus,
        0.0,
        2.0,
    );
    r(
        "knowledge.effects.canvass.floor",
        k.effects.canvass.floor,
        0.0,
        1.0,
    );
}

/// Where a grant list is used: it decides which selectors make sense.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Use {
    Start,
    LocalOption,
    Primer,
    Visit,
}

fn check_grants(b: &ContentBundle, owner: &str, grants: &[GrantDef], u: Use, out: &mut Vec<Issue>) {
    for g in grants {
        let what = format!("{} `{}`", g.subject.key(), g.which);
        if g.level == Level::Unknown {
            out.push(Issue::error(
                owner,
                &format!("{what}: a grant gives `heard`, `known` or `detailed`"),
            ));
        }
        let place_kind = g.subject == GrantSubject::PeopleAt
            && b.place_kind(&g.which).is_some()
            && !crate::types::PSEUDO_PLACES.contains(&g.which.as_str());
        if !place_kind && !g.subject.selectors().contains(&g.which.as_str()) {
            let mut ok: Vec<&str> = g.subject.selectors().to_vec();
            if g.subject == GrantSubject::PeopleAt {
                ok.push("<place kind>");
            }
            out.push(Issue::error(
                owner,
                &format!("{what}: unknown selector (expected {})", ok.join(", ")),
            ));
        }
        if g.which == "here" && u != Use::Visit {
            out.push(Issue::error(
                owner,
                &format!("{what}: `here` is for place introductions only"),
            ));
        }
        if matches!(g.which.as_str(), "home" | "ward") && u == Use::Primer {
            out.push(Issue::error(
                owner,
                &format!("{what}: a primer is about the new zone's town, not the player's ward"),
            ));
        }
        // DEC-091: even a native doesn't know who really runs things.
        if g.subject == GrantSubject::Broker && u == Use::Start {
            out.push(Issue::error(
                owner,
                "a start profile never gives the broker (only the local option does)",
            ));
        }
        // The local option is a convenience: depth still comes from exploring.
        if g.level == Level::Detailed && u == Use::LocalOption {
            out.push(Issue::error(
                owner,
                &format!("{what}: the local option never gives `detailed`"),
            ));
        }
    }
}

/// Profiles, the option, the primer and the introductions: known
/// selectors and place kinds, a default profile that exists, and the
/// DEC-091 limits (no broker in a profile, nothing detailed from the option).
pub(crate) fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let k = &b.knowledge;
    if k.start(&k.default_start).is_none() {
        issues.push(Issue::error(
            "knowledge",
            &format!("default_start `{}` is not a start profile", k.default_start),
        ));
    }
    let mut seen: Vec<&str> = Vec::new();
    for s in &k.starts {
        if !crate::validate::valid_key(&s.id) || !s.id.starts_with("start.") {
            issues.push(Issue::error(
                &s.id,
                "start profile ids look like `start.name`",
            ));
        }
        if seen.contains(&s.id.as_str()) {
            issues.push(Issue::error(
                &s.id,
                "duplicate start profile id in knowledge",
            ));
        }
        seen.push(&s.id);
        check_grants(b, &s.id, &s.grants, Use::Start, issues);
    }
    check_grants(
        b,
        "knowledge.local_option",
        &k.local_option,
        Use::LocalOption,
        issues,
    );
    check_grants(b, "knowledge.primer", &k.primer, Use::Primer, issues);
    check_grants(b, "knowledge.visit_all", &k.visit_all, Use::Visit, issues);
    for (kind, grants) in &k.visit_intro {
        if b.place_kind(kind).is_none() || crate::types::PSEUDO_PLACES.contains(&kind.as_str()) {
            issues.push(Issue::error(
                "knowledge.visit_intro",
                &format!("unknown place kind `{kind}`"),
            ));
        }
        check_grants(
            b,
            &format!("knowledge.visit_intro.{kind}"),
            grants,
            Use::Visit,
            issues,
        );
    }
    let l = &k.effects.lobby_logit;
    if !(l.unknown <= l.heard && l.heard <= l.known && l.known <= l.detailed) {
        issues.push(Issue::warning(
            "knowledge.effects.lobby_logit",
            "knowing someone better should never make lobbying them harder",
        ));
    }
}
