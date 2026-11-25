//! A scene's saved state (EXCHANGE_SCENES §11.1): the groups and their
//! leans, the other side's deck, the round, what was said. On the run
//! (`ArcRun::exchange` while it is under way, `ArcRun::played` once it is
//! over), with serde defaults: a run no scene has touched serializes as it
//! did before scenes existed.
//!
//! No `HashMap`, typed ids, ordered vectors (DEC-041).

use crate::arcs::BeatRef;
use crate::characters::CharacterId;
use crate::events::EventId;
use crate::world::Area;
use mandate_content::exchange::{MoveKind, Outcome, SourceKind, Speaker};
use mandate_core::SimDate;
use serde::{Deserialize, Serialize};

/// One group of the audience, bound from live state (§2.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Group {
    /// The residents of an area, or one income class or community of it.
    Area {
        area: Area,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        class: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        community: Option<u16>,
    },
    /// The rest of a source that gave more groups than the scene shows.
    /// A targeted line cannot name it.
    Elsewhere { areas: Vec<Area> },
    /// The holder of a post with the scene's power.
    Member { id: CharacterId },
    /// A cast person.
    Person { id: CharacterId },
}

impl Group {
    /// Cohorts (approval and recognition), not a character.
    pub fn crowd(&self) -> bool {
        matches!(self, Group::Area { .. } | Group::Elsewhere { .. })
    }

    pub fn person(&self) -> Option<CharacterId> {
        match self {
            Group::Member { id } | Group::Person { id } => Some(*id),
            _ => None,
        }
    }

    /// A stable id for option ids and tests (`hood.4`, `hood.4.c1`,
    /// `elsewhere`, `person.12`).
    pub fn key(&self) -> String {
        match self {
            Group::Area {
                area,
                class,
                community,
            } => {
                let mut s = area_key(*area);
                if let Some(c) = class {
                    s.push_str(&format!(".c{c}"));
                }
                if let Some(k) = community {
                    s.push_str(&format!(".k{k}"));
                }
                s
            }
            Group::Elsewhere { .. } => "elsewhere".into(),
            Group::Member { id } | Group::Person { id } => format!("person.{}", id.0),
        }
    }
}

/// `hood.4`, `seat.2`, `town.3`, `region.1`, `nation`.
pub fn area_key(a: Area) -> String {
    use crate::world::Jurisdiction as J;
    match a {
        Area::Hood { id } => format!("hood.{}", id.0),
        Area::Constituency { id } => format!("seat.{}", id.0),
        Area::In {
            at: J::Municipality { id },
        } => format!("town.{}", id.0),
        Area::In {
            at: J::Region { id },
        } => format!("region.{}", id.0),
        Area::In { at: J::Nation } => "nation".into(),
    }
}

/// A group with its weight and its lean, from −1 (with the other side) to
/// +1 (with the player).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupLean {
    pub group: Group,
    pub weight: f32,
    pub start: f32,
    pub lean: f32,
}

/// The thing a prompt or a line is about.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum What {
    /// Nothing with a name of its own (the safe move, an admission).
    #[default]
    None,
    /// A worry itself (the room's question).
    Worry,
    /// A law in force.
    Law { policy: u32 },
    /// A bill someone declared a vote on.
    Bill { policy: u32 },
    /// A promise or an accepted ask that ended (by its ask id).
    Promise { ask: u32 },
    /// A promise on the menu, with its chosen answer.
    Pledge {
        template: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        answer: Option<String>,
    },
    /// A policy template to table.
    Template { template: String },
    /// A running story.
    Scandal { id: u32 },
    /// A hidden fact.
    Hidden { id: u32 },
    /// Someone's record of visible work.
    Record,
    /// Things let pass in this arc.
    Arc,
    /// The government's record.
    Government,
    /// A group's own doorstep, by the group's key.
    Group { key: String },
    /// A cast person speaking for the player.
    Backer,
    /// Something found (a prep mark).
    Found { mark: String },
    /// Time: staying for every question.
    Time,
}

/// What a prompt or a line is about, and the worry it touches (the worry
/// decides how much each group cares, §5.3).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subject {
    pub what: What,
    /// A concern's content id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concern: Option<String>,
}

impl Subject {
    pub fn of(what: What, concern: Option<String>) -> Self {
        Subject { what, concern }
    }

    /// The same subject (§3.2): the same law, bill, promise, story or fact;
    /// or the same worry.
    pub fn same(&self, other: &Subject) -> bool {
        let named = |w: &What| {
            matches!(
                w,
                What::Law { .. }
                    | What::Bill { .. }
                    | What::Promise { .. }
                    | What::Scandal { .. }
                    | What::Hidden { .. }
            )
        };
        if named(&self.what) && self.what == other.what {
            return true;
        }
        // A law and the bill it was are one matter.
        if let (What::Law { policy: a }, What::Bill { policy: b })
        | (What::Bill { policy: a }, What::Law { policy: b }) = (&self.what, &other.what)
            && a == b
        {
            return true;
        }
        self.concern.is_some() && self.concern == other.concern
    }
}

/// A true thing put to the player (§4.1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attack {
    pub source: SourceKind,
    pub subject: Subject,
    /// 0..1.
    pub size: f32,
    /// The event on the spine it is anchored to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<EventId>,
    /// The chance it stands up when said (1 for the public record; less
    /// for a hidden fact held on weak evidence).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub p: f32,
    /// The group it names, if it names one (a vote against that group's
    /// lean), by the group's key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

fn one() -> f32 {
    1.0
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_one(v: &f32) -> bool {
    *v == 1.0
}

impl Attack {
    /// A stable key (`broken_promise:ask.3`): what `own` retires.
    pub fn key(&self) -> String {
        let what = match &self.subject.what {
            What::Law { policy } => format!("law.{policy}"),
            What::Bill { policy } => format!("bill.{policy}"),
            What::Promise { ask } => format!("ask.{ask}"),
            What::Scandal { id } => format!("story.{id}"),
            What::Hidden { id } => format!("secret.{id}"),
            _ => self.subject.concern.clone().unwrap_or_default(),
        };
        format!("{}:{what}", self.source.key())
    }
}

/// An answer the other side holds to a kind of line (§4.3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnswerHeld {
    pub kind: MoveKind,
    /// The one subject it answers (a law's grievance); any when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub what: Option<What>,
    pub size: f32,
}

/// The prompt on the table this round.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PromptNow {
    pub attack: Attack,
    pub by: Speaker,
    /// Which wording (from 1).
    pub variant: u8,
    /// Whether it stood up (an attack with a chance that failed turns back
    /// on whoever made it).
    pub stood: bool,
}

/// One round as it was played.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoundPlayed {
    pub prompt: PromptNow,
    /// The option's id (`record:law.17`); empty for a round forfeited.
    pub option: String,
    pub kind: Option<MoveKind>,
    /// Which wording of the player's line (from 1; 0 when forfeited).
    #[serde(default)]
    pub variant: u8,
    pub stood: bool,
    /// The other side answered it.
    pub answered: bool,
    /// The change in each group's lean.
    pub deltas: Vec<f32>,
    pub event: EventId,
}

/// A scene begun and not finished. It never spans a turn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArcExchangeState {
    pub beat: BeatRef,
    pub on: SimDate,
    pub groups: Vec<GroupLean>,
    /// The round being played, from 1.
    pub round: u8,
    pub prompt: Option<PromptNow>,
    /// The other side's attacks, best first. Built once, as the scene
    /// begins, so a reload meets the same opponent.
    pub deck: Vec<Attack>,
    pub answers: Vec<AnswerHeld>,
    pub answers_left: u8,
    /// Attacks owned or used (their keys).
    pub retired: Vec<String>,
    /// Options said (their ids): a subject is said once.
    pub used: Vec<String>,
    /// Offers waiting on the table (not built yet).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table: Vec<String>,
    pub rounds: Vec<RoundPlayed>,
}

/// A finished scene, kept while its run lasts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExchangePlayed {
    pub beat: BeatRef,
    pub on: SimDate,
    pub outcome: Outcome,
    /// Σ weight × change in lean.
    pub result: f32,
    pub groups: Vec<GroupLean>,
    pub rounds: Vec<RoundPlayed>,
    /// The closing event.
    pub event: EventId,
}
