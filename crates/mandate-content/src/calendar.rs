//! The town calendar (DEC-114 §4, `BEING_THERE.md` §5): recurring events,
//! as data. Each event belongs to a tier (`scope`; instances are keyed by
//! the jurisdiction, DEC-098), recurs by a cadence counted against turns,
//! is held at a venue (a place kind, or where the jurisdiction's
//! legislature sits) in a part of the day, and exists in a town only where
//! its `requires` hold (DEC-100: generated per town from its features and
//! history). People attend by routine: an event's `attendees` are routine
//! rules the engine tries before a person's own routines while it is held,
//! and routines can name it (`events: ["event.<name>"]`).
//!
//! **One date per turn** (DEC-114, the owner's ruling): a turn is played as
//! one day, the day of its main event, and every event of the fortnight is
//! held on that day in a part of its own, or reported as having happened.
//! So events are scheduled to turns: an occasion (`month`, `season`,
//! `year`) has a nominal date (a keyed day in its window), and falls in the
//! turn whose span holds it. The routine events (`turn`, `meeting_night`,
//! `session`) never choose the day: they are held when the day played is
//! theirs (`session` and `meeting_night` are the routine events of the
//! same names, unchanged).
//!
//! Nothing here names an office or a body: who is expected is a power, a
//! constitution role word, or "the seat whose district holds the venue".
//! The engine derives instances from the seed, the town and the turn
//! (`mandate-sim` `characters::calendar`); nothing is stored but the day.

use crate::bundle::ContentBundle;
use crate::condition::{Condition, ConditionUse, Site};
use crate::routines::{DayPart, RoutineDay, RoutineRole};
use crate::types::{ContentKey, Power, TierKey};
use crate::validate::{Issue, valid_key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How an event recurs, counted against turns (`BEING_THERE.md` §5.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CalendarCadence {
    /// Every turn, on the day played (it never chooses the day).
    Turn,
    /// The town's community meeting night (one weekday per town, keyed by
    /// the seed): held when the day played falls on it.
    MeetingNight,
    /// The jurisdiction's legislature sits (the routine event `session`): a
    /// law-making stage falls in the turn, or the day played is its regular
    /// sitting day.
    Session,
    /// Once a month, on a keyed day of the month (in `months`, if listed).
    Month,
    /// Once in each month of `months` (a term's start), early in it.
    Season,
    /// Once a year, on a keyed day in one of `months`.
    Year,
}

impl CalendarCadence {
    /// An occasion has a date of its own and can choose the day played;
    /// the routine events follow it.
    pub fn occasion(self) -> bool {
        matches!(
            self,
            CalendarCadence::Month | CalendarCadence::Season | CalendarCadence::Year
        )
    }
}

fn one() -> f64 {
    1.0
}

fn local() -> TierKey {
    TierKey::Local
}

/// Who comes, by routine: one rule, for exactly one of a post's role word,
/// an occupation or a special role (as a routine is for). While the event
/// is held, in its part, a person it covers goes to the venue with this
/// chance (a keyed draw per person and day), before their own routines.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarAttendee {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupation: Option<ContentKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<RoutineRole>,
    #[serde(default = "one")]
    pub chance: f64,
}

/// Who is expected to be there (no office named, DEC-098).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type", deny_unknown_fields)]
pub enum CalendarExpected {
    /// Whoever holds this power in the event's jurisdiction.
    Power { power: Power },
    /// Holders of a post with this role word (a body's `member_key`, an
    /// office's id) in the event's jurisdiction.
    Role { word: String },
    /// The holder of a districted seat whose district holds the venue.
    Ward,
}

/// What being expected and staying away costs (`BEING_THERE.md` §5.5;
/// owner's answer 2: small, shown a turn ahead, at most one a turn).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarAbsent {
    /// Opinion change (≤ 0) of the player among the stored people who were
    /// there, in the event's part.
    pub opinion: f64,
}

/// One recurring event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarEventDef {
    /// Content id, `event.<name>`.
    pub id: ContentKey,
    /// Its names (Fluent keys, with `$town`): each town keeps one, a keyed
    /// pick, so two towns' years differ.
    pub names: Vec<String>,
    pub cadence: CalendarCadence,
    /// Months (1–12) an occasion falls in (`month`: any, if empty).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub months: Vec<u8>,
    /// The weekday an occasion's date moves on to (the show on a Saturday).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weekday: Option<RoutineDay>,
    /// The parts of the day it can be held in, the first preferred. A
    /// second event on the day takes a part no earlier event holds; an
    /// occasion with none left is reported as having happened.
    pub parts: Vec<DayPart>,
    /// The tier it belongs to (instances are keyed by its jurisdiction).
    #[serde(default = "local")]
    pub scope: TierKey,
    /// A place kind (`places.jsonc` `key`), or `legislature` (where the
    /// jurisdiction's legislature sits). A town without one has no such
    /// event.
    pub venue: String,
    /// Situation conditions (slot-free) that must all hold in the town.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<Condition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attendees: Vec<CalendarAttendee>,
    /// Extra passers-by at the venue while it is held (how full it looks).
    #[serde(default)]
    pub crowd: u8,
    /// Visit verbs done there while it is held (`actions.jsonc`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verbs: Vec<ContentKey>,
    /// People here have time to talk: presence counts double for
    /// familiarity (`BEING_THERE.md` §6.2).
    #[serde(default)]
    pub talk: bool,
    /// Who is expected (any rule): only the player's absence costs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected: Vec<CalendarExpected>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absent: Option<CalendarAbsent>,
}

/// The venue words besides place kinds.
pub const VENUE_WORDS: [&str; 1] = ["legislature"];

impl ContentBundle {
    pub fn calendar_event(&self, id: &str) -> Option<&CalendarEventDef> {
        self.calendar.iter().find(|e| e.id == id)
    }
}

/// Shape and reference rules the schema cannot express.
pub fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    for (i, e) in b.calendar.iter().enumerate() {
        let id = e.id.as_str();
        let mut err = |m: String| issues.push(Issue::error(id, &m));
        if !valid_key(id) || !id.starts_with("event.") {
            err("a calendar event id must be a key starting with `event.`".into());
        }
        if b.calendar[..i].iter().any(|x| x.id == e.id) {
            err("duplicate calendar event id".into());
        }
        if e.names.is_empty() || e.names.iter().any(|n| n.is_empty()) {
            err("an event needs at least one name key".into());
        }
        if e.parts.is_empty() {
            err("an event needs a part of the day".into());
        }
        if e.parts.contains(&DayPart::LateNight) {
            err("events are held in the standard parts, not late at night".into());
        }
        if e.months.iter().any(|m| !(1..=12).contains(m)) {
            err("months are 1 to 12".into());
        }
        match e.cadence {
            CalendarCadence::Season | CalendarCadence::Year if e.months.is_empty() => {
                err("a `season` or `year` event lists its months".into());
            }
            c if !c.occasion() && (!e.months.is_empty() || e.weekday.is_some()) => {
                err("only an occasion (`month`, `season`, `year`) has months or a weekday".into());
            }
            _ => {}
        }
        let word = VENUE_WORDS.contains(&e.venue.as_str());
        let kind = b
            .place_kind(&e.venue)
            .is_some_and(|k| k.key != "home" && k.key != "streets");
        if !word && !kind {
            err(format!(
                "venue `{}` is neither `legislature` nor a place kind",
                e.venue
            ));
        }
        for a in &e.attendees {
            let n = a.post.is_some() as u8 + a.occupation.is_some() as u8 + a.role.is_some() as u8;
            if n != 1 {
                err(
                    "an attendee rule is for exactly one of `post`, `occupation` and `role`".into(),
                );
            }
            if let Some(w) = &a.post
                && !b.is_role_word(w)
            {
                err(format!(
                    "attendee post `{w}` is not a constitution role word"
                ));
            }
            if let Some(o) = &a.occupation
                && b.occupation(o).is_none()
            {
                err(format!("unknown attendee occupation `{o}`"));
            }
            if !(a.chance > 0.0 && a.chance <= 1.0) {
                err("an attendee's chance must be in (0, 1]".into());
            }
        }
        for x in &e.expected {
            if let CalendarExpected::Role { word } = x
                && !b.is_role_word(word)
            {
                err(format!(
                    "expected role `{word}` is not a constitution role word"
                ));
            }
        }
        if let Some(a) = &e.absent {
            if !(a.opinion.is_finite() && (-10.0..=0.0).contains(&a.opinion)) {
                err("an absence costs a small opinion fall, within [-10, 0]".into());
            }
            if e.expected.is_empty() {
                err("`absent` without `expected`: only an expected absence costs".into());
            }
        }
        if e.crowd > 12 {
            err("a crowd of at most 12 extra passers-by".into());
        }
        for v in &e.verbs {
            if b.actions.visit.verb(v).is_none() {
                err(format!("unknown visit verb `{v}`"));
            }
        }
        if e.scope != TierKey::Local {
            err("M6 holds town events only (`scope: local`)".into());
        }
        crate::condition::check_all(
            b,
            &Site::new(id, ConditionUse::Calendar, &[]),
            &e.requires,
            issues,
        );
    }
}
