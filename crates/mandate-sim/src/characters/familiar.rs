//! Familiarity (DEC-114 §5, `BEING_THERE.md` §6.2): how well someone knows
//! the player's face. A deed, not knowledge: it records being there, so it
//! may change what people offer (a greeting by name, what is on their
//! mind) but never which answer is right (DEC-091). Kept as one small
//! number on the person's relationship toward the player
//! (`Relationship.familiar`) and shown only in words
//! ([`Familiarity::key`]), never counted.
//!
//! It rises when the player is where the person is, on arrival or doing
//! anything there (`player::knowledge::note_sightings`), once a turn a
//! person, by more at a calendar event with time to talk; quick mode counts
//! the same. It fades each month without a meeting.

use super::{CharacterId, edge_mut};
use crate::politics::PoliticsState;
use crate::state::State;
use mandate_content::ContentBundle;
use mandate_content::condition::Familiarity;

/// The number on the holder's relationship toward the other (0: none).
pub fn familiar(pol: &PoliticsState, holder: CharacterId, other: CharacterId) -> u8 {
    pol.characters
        .edge(holder, other)
        .map(|e| e.familiar)
        .unwrap_or(0)
}

/// How well the holder knows the other's face, in words.
pub fn band(
    pol: &PoliticsState,
    content: &ContentBundle,
    holder: CharacterId,
    other: CharacterId,
) -> Familiarity {
    let f = &content.characters.familiarity;
    let n = familiar(pol, holder, other);
    if n >= f.ours && (super::opinion(pol, holder, other) as f64) > f.ours_opinion {
        Familiarity::Ours
    } else if n >= f.nod {
        Familiarity::Nod
    } else if n >= f.face {
        Familiarity::Face
    } else {
        Familiarity::Stranger
    }
}

/// The player was where `holder` is, this turn for the first time: their
/// familiarity rises (by `event` at a calendar event with time to talk).
pub fn met(
    pol: &mut PoliticsState,
    content: &ContentBundle,
    holder: CharacterId,
    player: CharacterId,
    at_event: bool,
) {
    if holder == player {
        return;
    }
    let f = &content.characters.familiarity;
    let add = if at_event { f.event } else { f.presence };
    if add == 0 {
        return;
    }
    let e = edge_mut(pol, content, holder, player);
    e.familiar = e.familiar.saturating_add(add).min(f.max);
}

/// The monthly fade: everyone who knows the player's face and has not been
/// seen by them within `idle_days` loses `decay`.
pub fn fade(state: &mut State, content: &ContentBundle, player: CharacterId) {
    let f = content.characters.familiarity.clone();
    let today = state.calendar.date;
    let seen = |c: CharacterId| {
        state
            .player
            .as_ref()
            .and_then(|p| p.knowledge.last_seen(c))
            .is_some_and(|s| s.on.add_days(f.idle_days) > today)
    };
    let fading: Vec<(u32, u32)> = state
        .politics
        .characters
        .edges
        .iter()
        .filter(|((_, o), e)| *o == player.0 && e.familiar > 0)
        .map(|(k, _)| *k)
        .filter(|(h, _)| !seen(CharacterId(*h)))
        .collect();
    for k in fading {
        if let Some(e) = state.politics.characters.edges.get_mut(&k) {
            e.familiar = e.familiar.saturating_sub(f.decay);
        }
    }
}
