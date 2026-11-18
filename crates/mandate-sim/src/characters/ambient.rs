//! Ambient people (DEC-018): passers-by generated on demand from a place's
//! cohorts — a pure function of (seed, place, period, slot) and the current
//! state — and never stored. Meeting one promotes them to a Named character.
//! Also: who can be found at a place (its Named regulars and staff).

use super::{CharacterId, KEY_AMBIENT, named_character, rng};
use crate::places::Location;
use crate::politics::PoliticsState;
use crate::state::State;
use mandate_content::ContentBundle;
use mandate_core::SimDate;

/// A person at a place: stored, or Ambient (`slot`).
#[derive(Clone, Debug, PartialEq)]
pub enum Present {
    Stored(CharacterId),
    /// An Ambient person and the character they would be if met.
    Ambient {
        slot: u8,
        person: Box<super::Character>,
    },
}

/// The key of a location for Ambient draws (`None` at home).
pub fn location_key(loc: Location) -> Option<u64> {
    match loc {
        Location::Home => None,
        Location::Place { place } => Some(place.0 as u64),
        Location::Street { neighborhood } => Some((1u64 << 32) | neighborhood.0 as u64),
    }
}

/// The period an Ambient crowd lasts (`characters.ambient.period_days`).
pub fn period(content: &ContentBundle, on: SimDate) -> u32 {
    on.0 / content.characters.ambient.period_days.max(1)
}

/// Ambient people at a location this period, minus anyone already met
/// (promoted). Pure: views may call it freely.
pub fn ambient_at(
    state: &State,
    content: &ContentBundle,
    loc: Location,
) -> Vec<(u8, super::Character)> {
    let Some(key) = location_key(loc) else {
        return Vec::new();
    };
    let kind = crate::places::kind_key(state, loc);
    let Some(hood) = crate::places::hood_of(state, loc) else {
        return Vec::new();
    };
    let occs: Vec<&mandate_content::OccupationDef> = content
        .occupations
        .iter()
        .filter(|o| o.ambient && o.places.iter().any(|k| k == kind))
        .collect();
    if occs.is_empty() {
        return Vec::new();
    }
    let seed = state.meta.seed;
    let per = period(content, state.calendar.date);
    let a = &content.characters.ambient;
    let n = rng(seed, &[KEY_AMBIENT, key, per as u64])
        .range_i64(a.present[0] as i64, a.present[1] as i64);
    let year = state.calendar.date.calendar().year;
    let met = &state.politics.characters;
    (0..n.max(0) as u8)
        .filter(|slot| {
            !met.people.iter().any(|c| {
                c.named
                    .as_deref()
                    .is_some_and(|nc| nc.origin == Some((key, per, *slot)))
            })
        })
        .filter_map(|slot| {
            let mut r = rng(seed, &[KEY_AMBIENT, key, per as u64, slot as u64]);
            let o = *r.pick(&occs)?;
            let mut c = named_character(
                content,
                &state.population,
                &mut r,
                CharacterId(u32::MAX),
                o,
                hood.0,
                None,
                year,
            );
            if let Some(nc) = c.named.as_deref_mut() {
                nc.origin = Some((key, per, slot));
            }
            Some((slot, c))
        })
        .collect()
}

/// Stored characters found at a location: politicians at their chamber or
/// town hall, Named people at their workplace, and people met on a
/// neighbourhood's streets.
pub fn stored_at(pol: &PoliticsState, state: &State, loc: Location) -> Vec<CharacterId> {
    let mut out = Vec::new();
    match loc {
        Location::Home => {}
        Location::Place { place } => {
            let Some(p) = state.world.places.get(place.index()) else {
                return out;
            };
            let m = p.municipality.index();
            match p.kind {
                crate::world::PlaceKind::CouncilChamber => {
                    if let Some(c) = pol.councils.get(m) {
                        out.extend(
                            c.members
                                .iter()
                                .filter_map(|id| pol.get(*id))
                                .filter(|x| !x.is_player)
                                .map(|x| x.character),
                        );
                    }
                }
                crate::world::PlaceKind::TownHall => {
                    if let Some(x) = pol
                        .councils
                        .get(m)
                        .and_then(|c| c.mayor)
                        .and_then(|id| pol.get(id))
                        && !x.is_player
                    {
                        out.push(x.character);
                    }
                }
                _ => {}
            }
            out.extend(
                pol.characters
                    .people
                    .iter()
                    .filter(|c| c.named.as_deref().is_some_and(|n| n.place == Some(place)))
                    .map(|c| c.id),
            );
        }
        Location::Street { neighborhood } => {
            out.extend(
                pol.characters
                    .people
                    .iter()
                    .filter(|c| {
                        c.named
                            .as_deref()
                            .is_some_and(|n| n.place.is_none() && n.home == neighborhood.0)
                    })
                    .map(|c| c.id),
            );
        }
    }
    out
}

/// Promote an Ambient person to a stored Named character (the player met
/// them). Returns their new id, or `None` if nobody is in that slot now.
pub fn promote(
    state: &mut State,
    content: &ContentBundle,
    loc: Location,
    slot: u8,
) -> Option<CharacterId> {
    let (_, mut c) = ambient_at(state, content, loc)
        .into_iter()
        .find(|(s, _)| *s == slot)?;
    let id: CharacterId = state.politics.characters.ids.alloc();
    c.id = id;
    if let (Some(n), Location::Place { place }) = (c.named.as_deref_mut(), loc) {
        n.place = Some(place);
    }
    state.politics.characters.people.push(c);
    Some(id)
}
