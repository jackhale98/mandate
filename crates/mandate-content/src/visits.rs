//! Places with purpose (DEC-079 note, task PL1): the verbs of the generic
//! `visit` action. Each place kind lists the verbs it offers (`places.jsonc`
//! `verbs`); each verb is data here (`actions.jsonc` `visit.verbs`): its
//! cost, when it can be done, what it teaches, who it reaches, and its risk.
//! Adding a verb to a place is content, not code (DEC-098 §4, the action
//! availability table, for these verbs).
//!
//! Availability uses the shared situation conditions (`SituationCondition`,
//! town-wide and slot-free, as policy unlocks do) plus two plain fields the
//! condition language has no word for yet: the day part (`parts`, as
//! routines write it) and someone of an occupation being here now (`with`).
//! A closed place offers nothing, as for every action. U1 folds these into
//! the one condition language.

use crate::bundle::ContentBundle;
use crate::knowledge::{GrantDef, Level};
use crate::routines::DayPart;
use crate::types::{ClassKey, ContentKey, SituationCondition};
use crate::validate::{Issue, valid_key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How long a verb takes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VisitTime {
    /// One slot.
    #[default]
    Slot,
    /// Half a slot: two half-slot things (verbs or conversations) fill one.
    Half,
}

/// Where an awareness or approval effect lands, relative to the place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VisitReach {
    /// The place's neighbourhood.
    #[default]
    Hood,
    /// Every neighbourhood of the place's town.
    Town,
}

/// Which residents an effect reaches: cohorts of `reach`, filtered by income
/// class, age band (0: 18–29, 1: 30–44, 2: 45–64, 3: 65+) and community
/// (ids from `communities.jsonc`). An empty filter takes everyone.
/// Cohorts have no occupation: "the works' people" are its neighbourhood's
/// working classes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VisitAmong {
    #[serde(default)]
    pub reach: VisitReach,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<ClassKey>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ages: Vec<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub communities: Vec<ContentKey>,
}

/// One effect of a verb. Every amount is small: a visit is depth, not a
/// campaign (DEC-079 rule 4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum VisitEffect {
    /// Knowledge grants (DEC-091), with `here` the place visited:
    /// `people_at`, `hood_concerns`, `broker`, `sector` (the place's
    /// industry in its town), `site` (pollution and hazards there) and
    /// `service` (the real need its service meets). A numeric subject
    /// granted `detailed` is a dated reading.
    Learn { grants: Vec<GrantDef> },
    /// The people here now (their daily routines, DEC-031), of these
    /// occupations (none: everyone), met to `level`; their opinion of the
    /// player moves by `opinion` (scaled by personality, as every opinion
    /// change is), remembered as a meeting.
    People {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        occupations: Vec<ContentKey>,
        level: Level,
        #[serde(default)]
        opinion: f64,
    },
    /// Name recognition: closes `amount` of the gap to fully known.
    Awareness {
        amount: f64,
        #[serde(default)]
        among: VisitAmong,
    },
    /// Approval: `amount` × the room left (negative for a setback).
    Approval {
        amount: f64,
        #[serde(default)]
        among: VisitAmong,
    },
}

/// A risk term: `add` while the town-wide condition holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VisitRiskTerm {
    pub when: SituationCondition,
    pub add: f64,
}

/// What can go wrong (DEC-073), from live state: `base`, plus each term
/// whose condition holds, plus `shedding` × how fast the place's industry is
/// shedding jobs in its town (0..1, `visit.outlook`), within [min, max].
/// The odds shown are the odds rolled (a keyed draw per turn, place and
/// verb). When it goes wrong, `effects` apply on top of the verb's and the
/// outcome is `outcome_key`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VisitRiskDef {
    pub base: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terms: Vec<VisitRiskTerm>,
    #[serde(default)]
    pub shedding: f64,
    pub min: f64,
    pub max: f64,
    pub outcome_key: String,
    #[serde(default)]
    pub effects: Vec<VisitEffect>,
}

/// One verb a place can offer (`verb.<name>`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VisitVerbDef {
    pub id: ContentKey,
    /// Fluent key of the verb's label (`verb-<name>`).
    pub name_key: String,
    /// Fluent key of the outcome (`outcome-verb-<name>`; args `place`).
    pub outcome_key: String,
    #[serde(default)]
    pub time: VisitTime,
    #[serde(default)]
    pub health: f64,
    /// May also be done in the late-night slot.
    #[serde(default)]
    pub late_night: bool,
    /// The standard day parts it can be done in (none: any). Late at night
    /// is `late_night`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<DayPart>,
    /// Town-wide conditions that must all hold (slot-free situation
    /// conditions, as a policy unlock uses).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub when: Vec<SituationCondition>,
    /// Someone of one of these occupations must be here now.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with: Vec<ContentKey>,
    #[serde(default)]
    pub effects: Vec<VisitEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<VisitRiskDef>,
}

/// How a town's industry is doing, from its jobs over its capacity at
/// neutral demand: at or above `growing` it is taking people on, below
/// `shedding` it is letting them go, and at `shed_full` or lower shedding
/// counts in full for a verb's risk (0 at `shedding`, 1 at `shed_full`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VisitOutlook {
    pub growing: f64,
    pub shedding: f64,
    pub shed_full: f64,
}

impl Default for VisitOutlook {
    fn default() -> Self {
        VisitOutlook {
            growing: 1.02,
            shedding: 0.95,
            shed_full: 0.85,
        }
    }
}

/// The `visit` action's verbs and numbers (`actions.jsonc` `visit`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VisitParams {
    #[serde(default)]
    pub outlook: VisitOutlook,
    #[serde(default)]
    pub verbs: Vec<VisitVerbDef>,
}

impl VisitParams {
    pub fn verb(&self, id: &str) -> Option<&VisitVerbDef> {
        self.verbs.iter().find(|v| v.id == id)
    }
}

fn check_among(b: &ContentBundle, owner: &str, a: &VisitAmong, issues: &mut Vec<Issue>) {
    for age in &a.ages {
        if *age > 3 {
            issues.push(Issue::error(owner, "age bands are 0..=3"));
        }
    }
    for c in &a.communities {
        if b.community_index(c).is_none() {
            issues.push(Issue::error(owner, &format!("unknown community `{c}`")));
        }
    }
}

fn check_effects(b: &ContentBundle, owner: &str, fx: &[VisitEffect], issues: &mut Vec<Issue>) {
    let small = |v: f64| v.is_finite() && v.abs() <= 0.25;
    for e in fx {
        match e {
            VisitEffect::Learn { grants } => {
                if grants.is_empty() {
                    issues.push(Issue::error(owner, "a learn effect needs grants"));
                }
                crate::knowledge::check_visit_grants(b, owner, grants, issues);
            }
            VisitEffect::People {
                occupations,
                level,
                opinion,
            } => {
                for o in occupations {
                    if b.occupation(o).is_none() {
                        issues.push(Issue::error(owner, &format!("unknown occupation `{o}`")));
                    }
                }
                if *level == Level::Unknown {
                    issues.push(Issue::error(owner, "people are met to heard, known or detailed"));
                }
                if !opinion.is_finite() || opinion.abs() > 10.0 {
                    issues.push(Issue::error(owner, "a visit's opinion change is within ±10"));
                }
            }
            VisitEffect::Awareness { amount, among } | VisitEffect::Approval { amount, among } => {
                if !small(*amount) {
                    issues.push(Issue::error(
                        owner,
                        "a visit's awareness or approval is small (within ±0.25)",
                    ));
                }
                check_among(b, owner, among, issues);
            }
        }
    }
}

/// Verbs: ids, keys, known occupations and communities, slot-free
/// conditions, sane amounts and odds; every verb offered somewhere, and
/// every verb a place lists defined.
pub(crate) fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let v = &b.actions.visit;
    let o = &v.outlook;
    if !(o.shed_full < o.shedding && o.shedding < o.growing) || o.shed_full <= 0.0 {
        issues.push(Issue::error(
            "actions.visit.outlook",
            "expected 0 < shed_full < shedding < growing",
        ));
    }
    for (i, d) in v.verbs.iter().enumerate() {
        let owner = d.id.as_str();
        if !valid_key(&d.id) || !d.id.starts_with("verb.") {
            issues.push(Issue::error(owner, "verb ids look like `verb.name`"));
        }
        if v.verbs[..i].iter().any(|x| x.id == d.id) {
            issues.push(Issue::error(owner, "duplicate verb id"));
        }
        if d.name_key.is_empty() || d.outcome_key.is_empty() {
            issues.push(Issue::error(owner, "a verb needs a name and an outcome key"));
        }
        if !d.health.is_finite() || !(0.0..=20.0).contains(&d.health) {
            issues.push(Issue::error(owner, "health cost within [0, 20]"));
        }
        if d.parts.contains(&DayPart::LateNight) {
            issues.push(Issue::error(
                owner,
                "late at night is `late_night: true`, not a part",
            ));
        }
        for c in &d.when {
            if c.is_slot_relative() {
                issues.push(Issue::error(
                    owner,
                    "a verb's conditions are about the town (slot-free, DEC-091)",
                ));
            }
            crate::validate::check_situation_condition(b, owner, c, None, issues);
        }
        for o in &d.with {
            if b.occupation(o).is_none() {
                issues.push(Issue::error(owner, &format!("unknown occupation `{o}`")));
            }
        }
        if d.effects.is_empty() {
            issues.push(Issue::error(owner, "a verb needs at least one effect"));
        }
        check_effects(b, owner, &d.effects, issues);
        if let Some(r) = &d.risk {
            let p = |x: f64| x.is_finite() && (0.0..=1.0).contains(&x);
            if !p(r.base) || !p(r.min) || !p(r.max) || r.min > r.max {
                issues.push(Issue::error(
                    owner,
                    "risk odds within [0, 1] with min ≤ max",
                ));
            }
            if !r.shedding.is_finite() || !(0.0..=1.0).contains(&r.shedding) {
                issues.push(Issue::error(owner, "risk shedding weight within [0, 1]"));
            }
            for t in &r.terms {
                if !t.add.is_finite() || t.add.abs() > 1.0 {
                    issues.push(Issue::error(owner, "a risk term adds within ±1"));
                }
                if t.when.is_slot_relative() {
                    issues.push(Issue::error(
                        owner,
                        "a risk term's condition is about the town (slot-free)",
                    ));
                }
                crate::validate::check_situation_condition(b, owner, &t.when, None, issues);
            }
            if r.outcome_key.is_empty() {
                issues.push(Issue::error(owner, "a risk needs an outcome key"));
            }
            check_effects(b, owner, &r.effects, issues);
        }
        if !b.places.iter().any(|p| p.verbs.contains(&d.id)) {
            issues.push(Issue::warning(owner, "no place offers this verb"));
        }
    }
    for p in &b.places {
        for id in &p.verbs {
            if v.verb(id).is_none() {
                issues.push(Issue::error(&p.id, &format!("unknown verb `{id}`")));
            }
        }
        if let Some(s) = &p.sector
            && b.sector(s).is_none()
        {
            issues.push(Issue::warning(&p.id, &format!("unknown sector `{s}` (its verbs read nothing for it)")));
        }
        for c in &p.addresses {
            if b.concern_index(c).is_none() {
                issues.push(Issue::warning(
                    &p.id,
                    &format!("unknown concern `{c}` (its service reads nothing for it)"),
                ));
            }
        }
        // A verb that teaches about the place's industry or service needs the
        // kind to name them.
        for d in p.verbs.iter().filter_map(|id| v.verb(id)) {
            let grants = d.effects.iter().chain(d.risk.iter().flat_map(|r| &r.effects));
            for g in grants.flat_map(|e| match e {
                VisitEffect::Learn { grants } => grants.as_slice(),
                _ => &[],
            }) {
                use crate::knowledge::GrantSubject as G;
                if g.subject == G::Sector && p.sector.is_none() {
                    issues.push(Issue::error(
                        &p.id,
                        &format!("verb `{}` teaches the place's industry: the kind needs a `sector`", d.id),
                    ));
                }
                if g.subject == G::Service && p.addresses.is_empty() {
                    issues.push(Issue::error(
                        &p.id,
                        &format!("verb `{}` teaches the service's need: the kind needs `addresses`", d.id),
                    ));
                }
            }
        }
    }
}
