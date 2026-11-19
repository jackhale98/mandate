//! Conversation state (player domain, M6_ENGINE_CONTRACTS §3.7): the
//! repetition guard's memory of lines used, the log of recent exchanges,
//! and the half slot left over from a conversation.

use super::AboutRef;
use crate::characters::CharacterId;
use crate::player::knowledge::{Lead, Level, Subject};
use crate::world::PlaceId;
use mandate_content::ContentKey;
use mandate_content::conversation::TalkPart;
use mandate_core::{IdAllocator, SimDate, define_id};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

define_id!(
    /// One conversation (an exchange) the player had.
    ExchangeId
);

/// A line someone used with the player (or heard at a place), and when.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsedLine {
    pub line: ContentKey,
    pub on: SimDate,
}

/// One part of a reply as spoken: the line, its Fluent key, and the
/// arguments it was spoken with (names and numbers captured then, so the
/// exchange reads the same later; facts, not prose).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpokenLine {
    pub part: TalkPart,
    pub line: ContentKey,
    pub key: String,
    pub args: Vec<(String, String)>,
}

/// A conversation the player had.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Exchange {
    pub id: ExchangeId,
    pub on: SimDate,
    pub speaker: CharacterId,
    pub place: Option<PlaceId>,
    pub topic: ContentKey,
    pub about: Option<AboutRef>,
    /// The reply, in order (opener, quote, substance, closer).
    pub parts: Vec<SpokenLine>,
    /// What the player learned from it (only what was new).
    pub learned: Vec<(Subject, Level)>,
    /// Leads it opened.
    pub leads: Vec<Lead>,
    /// The speaker's discretion cap at the time (a repeat teaches only if
    /// it rose: the relationship changed).
    pub cap: Level,
    /// Asked the same thing of the same person earlier this week.
    pub repeat: bool,
    /// How the speaker's opinion of the player moved (−: cooler, +: warmer).
    pub opinion: f32,
    /// A mechanic's outcome key (`outcome-talk-…`) and its arguments.
    pub outcome: Option<String>,
    pub outcome_args: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TalkState {
    pub ids: IdAllocator,
    /// The repetition guard: lines each person has used with the player,
    /// newest last, at most `talk_params.memory_lines` per person.
    #[serde(with = "mandate_core::serde_util::map_as_pairs")]
    pub used: BTreeMap<CharacterId, Vec<UsedLine>>,
    /// The same guard for overheard talk, per place (DEC-093: chatter
    /// follows the same rule). Recorded on arrival by the overheard producer.
    #[serde(with = "mandate_core::serde_util::map_as_pairs")]
    pub heard_at: BTreeMap<PlaceId, Vec<UsedLine>>,
    /// Recent exchanges, oldest first (capped at `talk_params.log_cap`):
    /// the notebook's "where and when".
    pub log: Vec<Exchange>,
    /// The turn in which a conversation left half a slot unspent (the next
    /// conversation that turn uses it). A turn stamp, so a new turn needs
    /// no reset.
    pub half_slot: Option<u32>,
}

impl TalkState {
    /// An exchange by id.
    pub fn exchange(&self, id: ExchangeId) -> Option<&Exchange> {
        self.log.iter().find(|e| e.id == id)
    }

    /// Lines a person has used with the player, newest last.
    pub fn used_by(&self, person: CharacterId) -> &[UsedLine] {
        self.used.get(&person).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Exchanges with a person in a week (day number / 7), oldest first.
    pub fn this_week(&self, person: CharacterId, week: u32) -> impl Iterator<Item = &Exchange> {
        self.log
            .iter()
            .filter(move |e| e.speaker == person && e.on.0 / 7 == week)
    }

    /// Remember the lines a person just used (capped, oldest dropped first).
    pub fn record_used(
        &mut self,
        person: CharacterId,
        lines: &[ContentKey],
        on: SimDate,
        cap: usize,
    ) {
        let v = self.used.entry(person).or_default();
        for l in lines {
            v.retain(|u| &u.line != l);
            v.push(UsedLine {
                line: l.clone(),
                on,
            });
        }
        let over = v.len().saturating_sub(cap.max(1));
        v.drain(..over);
    }

    /// Record the overheard lines drawn at a place this week (the overheard
    /// producer calls this on arrival; capped like `used`).
    pub fn record_heard(&mut self, place: PlaceId, lines: &[ContentKey], on: SimDate, cap: usize) {
        let v = self.heard_at.entry(place).or_default();
        for l in lines {
            if v.iter().any(|u| &u.line == l && u.on.0 / 7 == on.0 / 7) {
                continue;
            }
            v.retain(|u| &u.line != l);
            v.push(UsedLine {
                line: l.clone(),
                on,
            });
        }
        let over = v.len().saturating_sub(cap.max(1));
        v.drain(..over);
    }

    /// Overheard lines heard at a place before a week (the draw avoids them
    /// while alternatives exist; views and commands agree).
    pub fn heard_before(&self, place: PlaceId, week: u32) -> Vec<&ContentKey> {
        self.heard_at
            .get(&place)
            .map(|v| {
                v.iter()
                    .filter(|u| u.on.0 / 7 < week)
                    .map(|u| &u.line)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn push(&mut self, e: Exchange, cap: usize) {
        self.log.push(e);
        let over = self.log.len().saturating_sub(cap.max(1));
        self.log.drain(..over);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn used_lines_are_capped_and_newest_last() {
        let mut t = TalkState::default();
        let p = CharacterId(3);
        let k = |s: &str| s.to_string();
        t.record_used(p, &[k("a"), k("b"), k("c")], SimDate(1), 3);
        t.record_used(p, &[k("a"), k("d")], SimDate(2), 3);
        let v: Vec<&str> = t.used_by(p).iter().map(|u| u.line.as_str()).collect();
        assert_eq!(v, ["c", "a", "d"]);
        t.record_heard(PlaceId(1), &[k("x")], SimDate(3), 4);
        t.record_heard(PlaceId(1), &[k("x"), k("y")], SimDate(4), 4);
        assert_eq!(t.heard_at[&PlaceId(1)].len(), 2);
        assert!(t.heard_before(PlaceId(1), 0).is_empty());
        assert_eq!(t.heard_before(PlaceId(1), 1).len(), 2);
    }

    #[test]
    fn talk_state_round_trips() {
        let mut t = TalkState::default();
        t.record_used(CharacterId(1), &["line.a".into()], SimDate(4), 12);
        t.half_slot = Some(7);
        let j = serde_json::to_string(&t).unwrap();
        let back: TalkState = serde_json::from_str(&j).unwrap();
        assert_eq!(back, t);
    }
}
