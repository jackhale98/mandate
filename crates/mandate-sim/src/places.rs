//! Places as the interface (DEC-079): the player has a location in their town,
//! and every player action is done at a place that offers it. The place-kind
//! registry (content `places.jsonc`) says what each kind of place offers;
//! worldgen decides which places a town has.
//!
//! Moving within the town is free (no slot). Pseudo-locations: `Home` (the
//! player's home, in their home neighbourhood) and the `Street`s of each
//! neighbourhood. NPCs are abstracted and have no location (DEC-030 symmetry
//! for location is deferred).
//!
//! Places are never deleted during a run; one that shuts is marked closed
//! (`World::close_place`, DEC-090 D3) and then offers nothing and cannot be
//! entered. Nothing closes places yet.

use crate::actions::{Action, PlayerCommand};
use crate::commands::EngineError;
use crate::policy::PolicyStatus;
use crate::state::State;
use crate::world::{NeighborhoodId, PlaceId};
use mandate_content::{ContentBundle, PLACE_ACTIONS, PLACE_SERVICES, PlaceKindDef};
use serde::{Deserialize, Serialize};

/// Where the player is in their town.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Location {
    /// The player's home (in their home neighbourhood).
    #[default]
    Home,
    /// Out on the streets of a neighbourhood.
    Street { neighborhood: NeighborhoodId },
    /// Inside a generated place.
    Place { place: PlaceId },
}

/// Something a place can offer: an action (optionally its late-night
/// variant) or an information service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offer<'a> {
    Action { key: &'a str, late_night: bool },
    Service(&'a str),
}

/// Actions done in a particular neighbourhood (the target neighbourhood).
pub const NEIGHBORHOOD_ACTIONS: [&str; 2] = ["canvass", "town_hall"];

fn bad(msg: String) -> EngineError {
    EngineError::InvalidCommand(msg)
}

pub fn kind_offers(def: &PlaceKindDef, offer: Offer<'_>) -> bool {
    match offer {
        Offer::Action {
            key,
            late_night: false,
        } => def.actions.iter().any(|a| a == key),
        Offer::Action {
            key,
            late_night: true,
        } => def.late_night.iter().any(|a| a == key),
        Offer::Service(s) => def.services.iter().any(|x| x == s),
    }
}

/// The player's home neighbourhood: the first neighbourhood of their ward.
pub fn home_neighborhood(state: &State) -> Option<NeighborhoodId> {
    let pl = state.player.as_ref()?;
    crate::policy::ward_hoods(&state.world, pl.ward)
        .first()
        .map(|h| NeighborhoodId(*h as u32))
        .or_else(|| {
            state
                .world
                .municipalities
                .get(pl.municipality.index())
                .and_then(|m| m.neighborhoods.first().copied())
        })
}

/// Registry key of a location's kind (`home`, `streets`, or a place kind).
pub fn kind_key(state: &State, loc: Location) -> &'static str {
    match loc {
        Location::Home => "home",
        Location::Street { .. } => "streets",
        Location::Place { place } => state
            .world
            .places
            .get(place.index())
            .map(|p| p.kind.key())
            .unwrap_or("home"),
    }
}

/// The neighbourhood a location is in.
pub fn hood_of(state: &State, loc: Location) -> Option<NeighborhoodId> {
    match loc {
        Location::Home => home_neighborhood(state),
        Location::Street { neighborhood } => Some(neighborhood),
        Location::Place { place } => state
            .world
            .places
            .get(place.index())
            .map(|p| p.neighborhood),
    }
}

/// Whether a neighbourhood is in the player's town.
fn hood_in_town(state: &State, hood: u32) -> bool {
    let Some(pl) = state.player.as_ref() else {
        return false;
    };
    state
        .world
        .neighborhoods
        .get(hood as usize)
        .is_some_and(|n| n.municipality == pl.municipality)
}

/// Locations in the player's town whose kind satisfies `pred`, optionally
/// limited to one neighbourhood. Places come first (in id order), then
/// streets, then home.
fn spots(state: &State, pred: &dyn Fn(&str) -> bool, hood: Option<u32>) -> Vec<Location> {
    let mut out = Vec::new();
    let Some(pl) = state.player.as_ref() else {
        return out;
    };
    let Some(mu) = state.world.municipalities.get(pl.municipality.index()) else {
        return out;
    };
    for pid in &mu.places {
        let Some(p) = state.world.places.get(pid.index()) else {
            continue;
        };
        // A closed place offers nothing (DEC-090 D3: closed, never deleted).
        if !p.is_open() || hood.is_some_and(|h| p.neighborhood.0 != h) {
            continue;
        }
        if pred(p.kind.key()) {
            out.push(Location::Place { place: p.id });
        }
    }
    if pred("streets") {
        match hood {
            Some(h) => out.push(Location::Street {
                neighborhood: NeighborhoodId(h),
            }),
            None => out.extend(
                mu.neighborhoods
                    .iter()
                    .map(|n| Location::Street { neighborhood: *n }),
            ),
        }
    }
    if pred("home") && hood.is_none_or(|h| home_neighborhood(state).is_some_and(|x| x.0 == h)) {
        out.push(Location::Home);
    }
    out
}

/// Every location in the player's town that offers `offer` (within `hood`
/// for neighbourhood actions). When no place in scope offers it, the
/// registry's fallbacks host it: a neighbourhood without a community centre
/// meets on its streets, a town without a newspaper talks to the press at the
/// town hall.
pub fn locations_for(
    state: &State,
    content: &ContentBundle,
    offer: Offer<'_>,
    hood: Option<u32>,
) -> Vec<Location> {
    if hood.is_some_and(|h| !hood_in_town(state, h)) {
        return Vec::new();
    }
    let offers = |k: &str| content.place_kind(k).is_some_and(|d| kind_offers(d, offer));
    let mut out = spots(state, &offers, hood);
    if out.is_empty() {
        for d in content.places.iter().filter(|d| kind_offers(d, offer)) {
            if let Some(f) = &d.fallback {
                for l in spots(state, &|k: &str| k == f, hood) {
                    if !out.contains(&l) {
                        out.push(l);
                    }
                }
            }
        }
    }
    out
}

/// The action key, late-night flag, and target neighbourhood of a player
/// command (`None` for movement).
pub fn action_of(state: &State, cmd: &PlayerCommand) -> Option<(&'static str, bool, Option<u32>)> {
    Some(match cmd {
        PlayerCommand::MoveTo { .. }
        | PlayerCommand::Go { .. }
        | PlayerCommand::Meet { .. }
        | PlayerCommand::Answer { .. } => {
            return None;
        }
        PlayerCommand::CastVote { .. } => ("vote", false, None),
        PlayerCommand::File { .. } => ("file", false, None),
        PlayerCommand::DraftPolicy { .. } => ("draft", false, None),
        PlayerCommand::AmendPolicy { policy, .. } => {
            // Drafts are revised at the clerk's desk; bills on the floor.
            let draft = state
                .policy
                .instances
                .get(*policy as usize)
                .is_some_and(|p| p.status == PolicyStatus::Draft);
            (if draft { "draft" } else { "amend" }, false, None)
        }
        PlayerCommand::IntroducePolicy { .. } => ("introduce", false, None),
        PlayerCommand::WithdrawPolicy { .. } => ("withdraw", false, None),
        PlayerCommand::Act { action, late_night } => match action {
            Action::Lobby { .. } => ("lobby", *late_night, None),
            Action::TradeFavor { .. } => ("trade_favor", *late_night, None),
            Action::Fundraise => ("fundraise", *late_night, None),
            // Other actions have no late-night variant; `apply` rejects them.
            Action::TownHall { neighborhood, .. } => ("town_hall", false, Some(*neighborhood)),
            Action::Canvass { neighborhood } => ("canvass", false, Some(*neighborhood)),
            Action::Press { .. } => ("press", false, None),
            Action::Advertise => ("advertise", false, None),
            Action::Rest => ("rest", false, None),
            Action::TakeBribe => ("take_bribe", *late_night, None),
            Action::DipFunds => ("dip_funds", false, None),
            Action::LeakFile => ("leak_file", false, None),
        },
    })
}

/// The best location to do a command: where the player already is if that
/// works, otherwise the first matching place in their town. `None` for
/// movement commands, or when the town offers the action nowhere.
pub fn best_location(
    state: &State,
    content: &ContentBundle,
    cmd: &PlayerCommand,
) -> Option<Location> {
    let (key, late_night, hood) = action_of(state, cmd)?;
    best_for(state, content, key, late_night, hood)
}

/// The best location for an action key (see [`best_location`]).
pub fn best_for(
    state: &State,
    content: &ContentBundle,
    key: &str,
    late_night: bool,
    hood: Option<u32>,
) -> Option<Location> {
    let here = state.player.as_ref()?.location;
    let locs = locations_for(state, content, Offer::Action { key, late_night }, hood);
    if locs.contains(&here) {
        Some(here)
    } else {
        locs.first().copied()
    }
}

/// Auto-routing (DEC-079 rule 2): wrap a command in a `Go` to the best
/// location for it, unless the player is already there.
pub fn routed(state: &State, content: &ContentBundle, cmd: PlayerCommand) -> PlayerCommand {
    let here = state.player.as_ref().map(|p| p.location);
    match best_location(state, content, &cmd) {
        Some(l) if Some(l) != here => PlayerCommand::Go {
            location: l,
            then: Some(Box::new(cmd)),
        },
        _ => cmd,
    }
}

/// Located actions (DEC-079): a command is valid only where it is offered.
pub fn check_at(
    state: &State,
    content: &ContentBundle,
    cmd: &PlayerCommand,
) -> Result<(), EngineError> {
    let Some((key, late_night, hood)) = action_of(state, cmd) else {
        return Ok(());
    };
    let Some(pl) = state.player.as_ref() else {
        return Ok(());
    };
    // A neighbourhood outside the town is reported by the action itself.
    if hood.is_some_and(|h| !hood_in_town(state, h)) {
        return Ok(());
    }
    let locs = locations_for(state, content, Offer::Action { key, late_night }, hood);
    if locs.contains(&pl.location) {
        return Ok(());
    }
    let when = if late_night { " late at night" } else { "" };
    match locs.first() {
        Some(l) => Err(bad(format!(
            "go to the {} first ({key}{when} is done there)",
            kind_key(state, *l)
        ))),
        None => Err(bad(format!("nowhere in your town offers {key}{when}"))),
    }
}

/// Whether the player may move to a location: anywhere in their own town.
/// Travel to other towns is not yet possible.
pub fn validate_move(state: &State, loc: Location) -> Result<(), EngineError> {
    let pl = state
        .player
        .as_ref()
        .ok_or_else(|| bad("no player".into()))?;
    let m = pl.municipality;
    let ok = match loc {
        Location::Home => true,
        Location::Street { neighborhood } => state
            .world
            .neighborhoods
            .get(neighborhood.index())
            .is_some_and(|n| n.municipality == m),
        Location::Place { place } => state
            .world
            .places
            .get(place.index())
            .is_some_and(|p| p.municipality == m),
    };
    if let Location::Place { place } = loc
        && state
            .world
            .places
            .get(place.index())
            .is_some_and(|p| p.municipality == m && !p.is_open())
    {
        return Err(bad("that place is closed".into()));
    }
    if ok {
        Ok(())
    } else {
        Err(bad(
            "you can only move within your own town (travel is not yet possible)".into(),
        ))
    }
}

/// The arrival hook (DEC-091, §1.3): the player has just moved to `loc`
/// (`MoveTo`, or a `Go` landing there, which the map, the List view and
/// quick mode all send; the client-side walk teaches nothing). A place's
/// first visit gives its introduction, and where people talk (`overheard`)
/// this week's lines teach what they mention at `heard`. Arrival teaches
/// lightly; anything deeper needs an action or a person. Returns how many
/// subjects changed in the player's ledger.
pub fn arrive(state: &mut State, content: &ContentBundle, loc: Location) -> usize {
    let Location::Place { place } = loc else {
        return 0;
    };
    let mut n = crate::player::knowledge::arrive_at(state, content, place);
    if services_at(state, content, loc).contains(&"overheard") {
        n += crate::overheard::overhear(state, content, loc);
    }
    n
}

/// Whether the player is at a location offering an information service.
pub fn player_has_service(state: &State, content: &ContentBundle, service: &str) -> bool {
    let Some(pl) = state.player.as_ref() else {
        return false;
    };
    locations_for(state, content, Offer::Service(service), None).contains(&pl.location)
}

/// Information services offered at a location (registry plus fallbacks).
pub fn services_at(state: &State, content: &ContentBundle, loc: Location) -> Vec<&'static str> {
    PLACE_SERVICES
        .iter()
        .copied()
        .filter(|s| locations_for(state, content, Offer::Service(s), None).contains(&loc))
        .collect()
}

/// Actions offered at a location, as `(key, late_night)`, registry plus
/// fallbacks. Neighbourhood actions are offered for the location's own
/// neighbourhood.
pub fn actions_at(
    state: &State,
    content: &ContentBundle,
    loc: Location,
) -> Vec<(&'static str, bool)> {
    let hood = hood_of(state, loc).map(|h| h.0);
    let mut out = Vec::new();
    for key in PLACE_ACTIONS {
        let scope = if NEIGHBORHOOD_ACTIONS.contains(&key) {
            hood
        } else {
            None
        };
        for late_night in [false, true] {
            if late_night && !mandate_content::LATE_NIGHT_ACTIONS.contains(&key) {
                continue;
            }
            if !late_night && mandate_content::LATE_ONLY_ACTIONS.contains(&key) {
                continue;
            }
            if locations_for(state, content, Offer::Action { key, late_night }, scope)
                .contains(&loc)
            {
                out.push((key, late_night));
            }
        }
    }
    out
}

/// Place kinds worldgen can generate that the registry lacks (DEC-079
/// validation; run by `mandate-cli content validate` and the tests).
pub fn registry_issues(content: &ContentBundle) -> Vec<String> {
    crate::world::PlaceKind::ALL
        .iter()
        .filter(|k| content.place_kind(k.key()).is_none())
        .map(|k| {
            format!(
                "place kind `{}` has no entry in the place registry",
                k.key()
            )
        })
        .collect()
}
