//! Daily routines (DEC-031, "Daily routines"; DEC-013 day parts): where
//! stored characters are through the day. Each routine is for a post (a
//! constitution role word: a body's `member_key` or an office's id, DEC-098),
//! an occupation, or a special role (the town's broker or rival, a
//! politician out of office, anyone else). Its rules are tried in order: the
//! first whose conditions hold (day part, weekday, season, events, a keyed
//! chance) and that names somewhere in the person's town places them there,
//! picked by weight with a keyed draw. A routine none of whose rules applies
//! hands over to the next routine that covers them; the last resort is their
//! workplace, then their home street.
//!
//! The engine derives placement from state and date (`characters::routines`);
//! nothing is stored. No office or body is named in code: posts are matched
//! by their role word, read from the constitution.

use crate::bundle::ContentBundle;
use crate::types::ContentKey;
use crate::validate::{Issue, valid_key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A part of the representative day (DEC-013): the three standard slots and
/// the late-night one. The engine reads it from the player's progress
/// through the turn (`characters::routines::day_part`).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DayPart {
    Morning,
    Afternoon,
    Evening,
    LateNight,
}

impl DayPart {
    pub const ALL: [DayPart; 4] = [
        DayPart::Morning,
        DayPart::Afternoon,
        DayPart::Evening,
        DayPart::LateNight,
    ];

    /// The content key (`late_night`); Fluent `day-part-<key with dashes>`.
    pub fn key(self) -> &'static str {
        match self {
            DayPart::Morning => "morning",
            DayPart::Afternoon => "afternoon",
            DayPart::Evening => "evening",
            DayPart::LateNight => "late_night",
        }
    }

    /// Position in the day (0..4).
    pub fn index(self) -> u64 {
        self as u64
    }
}

/// A day of the week, as content writes it.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RoutineDay {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl RoutineDay {
    /// The weekday of a simulation date.
    pub fn of(d: mandate_core::SimDate) -> RoutineDay {
        use mandate_core::time::Weekday as W;
        match d.weekday() {
            W::Mon => RoutineDay::Mon,
            W::Tue => RoutineDay::Tue,
            W::Wed => RoutineDay::Wed,
            W::Thu => RoutineDay::Thu,
            W::Fri => RoutineDay::Fri,
            W::Sat => RoutineDay::Sat,
            W::Sun => RoutineDay::Sun,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            RoutineDay::Mon => "mon",
            RoutineDay::Tue => "tue",
            RoutineDay::Wed => "wed",
            RoutineDay::Thu => "thu",
            RoutineDay::Fri => "fri",
            RoutineDay::Sat => "sat",
            RoutineDay::Sun => "sun",
        }
    }
}

/// A season (months 12–2 are winter, 3–5 spring, 6–8 summer, 9–11 autumn).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Season {
    Winter,
    Spring,
    Summer,
    Autumn,
}

impl Season {
    /// The season of a simulation date.
    pub fn of(d: mandate_core::SimDate) -> Season {
        match d.calendar().month {
            3..=5 => Season::Spring,
            6..=8 => Season::Summer,
            9..=11 => Season::Autumn,
            _ => Season::Winter,
        }
    }
}

/// Something going on in the person's jurisdiction this turn.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RoutineEvent {
    /// The jurisdiction's legislature sits: a law-making stage (a vote, a
    /// signature) falls in this turn, or it is its regular sitting day (one
    /// weekday per jurisdiction, keyed by the world seed).
    Session,
    /// A vote stage of the jurisdiction falls in this turn.
    Vote,
    /// A race for one of the jurisdiction's posts is running.
    Campaign,
    /// The town's community meeting night (one weekday per town, keyed by
    /// the world seed).
    MeetingNight,
}

/// Special roles a routine can be for.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RoutineRole {
    /// The town's power broker (DEC-077 §1).
    Broker,
    /// The named rival of the town's leading figure.
    Rival,
    /// A politician holding no post (a challenger, a former official).
    Politician,
    /// Any stored character (the last routine tried).
    Anyone,
}

impl RoutineRole {
    pub fn key(self) -> &'static str {
        match self {
            RoutineRole::Broker => "broker",
            RoutineRole::Rival => "rival",
            RoutineRole::Politician => "politician",
            RoutineRole::Anyone => "anyone",
        }
    }
}

fn one() -> f64 {
    1.0
}

/// Where a rule sends someone, with a weight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoutineSpot {
    /// A location word or a place kind (`places.jsonc` `key`):
    /// - `works`: their workplace (an official: a place of the kind their
    ///   post works at, in its town; anyone else: their own place);
    /// - `home_streets`: their home neighbourhood's streets;
    /// - `ward_streets`: the streets of a neighbourhood of the district
    ///   their seat represents (their home streets without one);
    /// - `legislature`: where their jurisdiction's legislature works;
    /// - a place kind (`diner`, `market`, …): their own one of that kind in
    ///   their town (the one in their neighbourhood, else one kept for them).
    pub to: String,
    #[serde(default = "one")]
    pub weight: f64,
}

/// The location words a spot can name besides place kinds.
pub const ROUTINE_WORDS: [&str; 4] = ["works", "home_streets", "ward_streets", "legislature"];

/// One rule of a routine: when it applies and where it sends them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoutineRule {
    /// Day parts it applies in (none listed: all).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<DayPart>,
    /// Weekdays of the turn's representative day (none listed: all).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub days: Vec<RoutineDay>,
    /// Seasons (none listed: all).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seasons: Vec<Season>,
    /// Events that must all be going on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<RoutineEvent>,
    /// Events none of which may be going on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unless: Vec<RoutineEvent>,
    /// Chance (0..1] the rule applies on a given day and part, keyed by the
    /// person, the day and the part.
    #[serde(default = "one")]
    pub chance: f64,
    /// Where they go, by weight (spots their town lacks are skipped; a rule
    /// with none left does not apply).
    pub at: Vec<RoutineSpot>,
}

/// A routine: whom it is for, and its rules in order. Exactly one of
/// `post`, `occupation` and `role`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoutineDef {
    /// Content id, `routine.<name>`.
    pub id: ContentKey,
    /// A post's role word from the constitution (a body's `member_key`, an
    /// office's id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post: Option<String>,
    /// An occupation id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupation: Option<ContentKey>,
    /// A special role. The broker's and the rival's routines come before
    /// their post's or occupation's; `politician` and `anyone` after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<RoutineRole>,
    pub rules: Vec<RoutineRule>,
}

impl ContentBundle {
    /// The routines that cover a role word, an occupation or a special role,
    /// in load order.
    pub fn routines_for_post<'a>(&'a self, word: &'a str) -> impl Iterator<Item = &'a RoutineDef> {
        self.routines
            .iter()
            .filter(move |r| r.post.as_deref() == Some(word))
    }

    pub fn routines_for_occupation<'a>(
        &'a self,
        occupation: &'a str,
    ) -> impl Iterator<Item = &'a RoutineDef> {
        self.routines
            .iter()
            .filter(move |r| r.occupation.as_deref() == Some(occupation))
    }

    pub fn routines_for_role(&self, role: RoutineRole) -> impl Iterator<Item = &RoutineDef> {
        self.routines.iter().filter(move |r| r.role == Some(role))
    }
}

/// Shape and reference rules the schema cannot express.
pub fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    for r in &b.routines {
        let id = r.id.as_str();
        let mut err = |m: String| issues.push(Issue::error(id, &m));
        if !valid_key(id) || !id.starts_with("routine.") {
            err("a routine id must be a key starting with `routine.`".into());
        }
        let n = r.post.is_some() as u8 + r.occupation.is_some() as u8 + r.role.is_some() as u8;
        if n != 1 {
            err("a routine is for exactly one of `post`, `occupation` and `role`".into());
        }
        if let Some(w) = &r.post
            && !b.is_role_word(w)
        {
            err(format!(
                "post `{w}` is not a role word of any constitution's posts (a body's member_key or an office's id)"
            ));
        }
        if let Some(o) = &r.occupation
            && b.occupation(o).is_none()
        {
            err(format!("unknown occupation `{o}`"));
        }
        if r.rules.is_empty() {
            err("a routine needs at least one rule".into());
        }
        for (i, rule) in r.rules.iter().enumerate() {
            if !(rule.chance > 0.0 && rule.chance <= 1.0) {
                err(format!("rule {i}: chance must be in (0, 1]"));
            }
            if rule.at.is_empty() {
                err(format!("rule {i}: `at` names nowhere"));
            }
            if rule.events.iter().any(|e| rule.unless.contains(e)) {
                err(format!("rule {i}: an event is both required and excluded"));
            }
            for s in &rule.at {
                if !(s.weight.is_finite() && s.weight > 0.0) {
                    err(format!("rule {i}: weight of `{}` must be positive", s.to));
                }
                let word = ROUTINE_WORDS.contains(&s.to.as_str());
                let kind = b
                    .place_kind(&s.to)
                    .is_some_and(|k| k.key != "home" && k.key != "streets");
                if !word && !kind {
                    err(format!(
                        "rule {i}: `{}` is neither a location word ({}) nor a place kind",
                        s.to,
                        ROUTINE_WORDS.join(", ")
                    ));
                }
            }
        }
    }
    // Every kind of stored character has a routine to fall back on.
    if !b.routines.is_empty() && b.routines_for_role(RoutineRole::Anyone).next().is_none() {
        issues.push(Issue::warning(
            "routines",
            "no routine for role `anyone`: people no other routine covers stay at their workplace or home street",
        ));
    }
}
