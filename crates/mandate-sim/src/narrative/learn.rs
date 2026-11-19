//! Knowledge from narrative content (DEC-091; M6_ENGINE_CONTRACTS §1.3):
//! resolving a [`SubjectDef`] (a subject named by role) against a cast and
//! the player's town, the `knows` condition, and the grants that narrative
//! effects and scene hooks give. Read-only; the writers are the Commands
//! phase (`resolve::answer`) and effect requests. No randomness: every
//! subject is a function of the cast and state.

use super::cast::View;
use super::{Actor, CastEntry};
use crate::player::knowledge::{self, Grant, Level, PolicyRef, Subject};
use crate::world::{MunicipalityId, NeighborhoodId, PlaceId};
use mandate_content::SubjectDef;

/// The neighbourhood an actor stands for: a cast neighbourhood itself, a
/// person's home neighbourhood.
fn hood_of_actor(v: &View<'_>, a: Actor) -> Option<NeighborhoodId> {
    match a {
        Actor::Neighborhood { id } => Some(NeighborhoodId(id)),
        Actor::Person { id } => {
            crate::characters::profile(v.pol, v.r.world, id).map(|p| NeighborhoodId(p.home))
        }
        Actor::Policy { .. } => None,
    }
}

/// Places of these kinds (any if empty) in a town, within a neighbourhood
/// if given, in id order.
pub fn places_in(
    v: &View<'_>,
    town: MunicipalityId,
    hood: Option<NeighborhoodId>,
    kinds: &[String],
) -> Vec<PlaceId> {
    v.r.world
        .places
        .iter()
        .filter(|p| p.municipality == town)
        .filter(|p| hood.is_none_or(|h| p.neighborhood == h))
        .filter(|p| kinds.is_empty() || kinds.iter().any(|k| k == p.kind.key()))
        .map(|p| p.id)
        .collect()
}

/// The subjects a definition names, for this cast in the player's town.
/// `here` is the neighbourhood where it happens (where a line is heard,
/// where a scene is set). A role that is not cast (or casts something of
/// the wrong kind) names nothing; so does everything without a player.
pub fn subjects(
    v: &View<'_>,
    def: &SubjectDef,
    cast: &[CastEntry],
    here: Option<NeighborhoodId>,
) -> Vec<Subject> {
    let Some(town) = v.town().map(MunicipalityId) else {
        return Vec::new();
    };
    let actor = |role: &str| super::actor_of(cast, role);
    // A role names a neighbourhood (or nothing, if not cast); no role: here.
    let hood = |role: &Option<String>| -> Result<Option<NeighborhoodId>, ()> {
        match role {
            Some(r) => actor(r)
                .and_then(|a| hood_of_actor(v, a))
                .map(Some)
                .ok_or(()),
            None => Ok(here),
        }
    };
    match def {
        SubjectDef::TownConcerns => vec![Subject::TownConcerns { town }],
        SubjectDef::HoodConcerns { role } => hood(role)
            .ok()
            .flatten()
            .map(|hood| Subject::HoodConcerns { hood })
            .into_iter()
            .collect(),
        SubjectDef::Broker => vec![Subject::Broker { town }],
        SubjectDef::RivalPlans => vec![Subject::RivalPlans { town }],
        SubjectDef::Person { role } => match actor(role) {
            Some(Actor::Person { id }) if Some(id) != v.me() => {
                vec![Subject::Person { person: id }]
            }
            _ => Vec::new(),
        },
        SubjectDef::PolicySupport { role, template } => {
            let t = match (role, template) {
                (Some(r), _) => match actor(r) {
                    Some(Actor::Policy { id }) => {
                        v.r.policy()
                            .instances
                            .get(id as usize)
                            .map(|p| p.template.clone())
                    }
                    _ => None,
                },
                (None, t) => t.clone(),
            };
            t.map(|template| Subject::PolicySupport {
                town,
                policy: PolicyRef::Template { template },
            })
            .into_iter()
            .collect()
        }
        SubjectDef::Situational { template } => vec![Subject::Situational {
            town,
            template: template.clone(),
        }],
        SubjectDef::Place { role, kinds } => match hood(role) {
            Ok(h) => places_in(v, town, h, kinds)
                .into_iter()
                .map(|place| Subject::Place { place })
                .collect(),
            Err(()) => Vec::new(),
        },
    }
}

/// The `knows` condition: the player knows the subject at least at `min`
/// (any of several places). Without a player (lab runs) everything is
/// known, as `knowledge::knows` reads it.
pub fn knows(
    v: &View<'_>,
    def: &SubjectDef,
    min: Level,
    cast: &[CastEntry],
    here: Option<NeighborhoodId>,
) -> bool {
    let p = v.player();
    if p.is_none() {
        return true;
    }
    subjects(v, def, cast, here)
        .iter()
        .any(|s| knowledge::level_of(p, s) >= min)
}

/// Grants for a definition at a level.
pub fn grants(
    v: &View<'_>,
    def: &SubjectDef,
    level: Level,
    cast: &[CastEntry],
    here: Option<NeighborhoodId>,
) -> Vec<Grant> {
    subjects(v, def, cast, here)
        .into_iter()
        .map(|subject| Grant { subject, level })
        .collect()
}

/// The people a cast names (not the player), in cast order.
pub fn cast_people(v: &View<'_>, cast: &[CastEntry]) -> Vec<crate::characters::CharacterId> {
    let me = v.me();
    let mut out = Vec::new();
    for e in cast {
        if let Actor::Person { id } = e.actor
            && Some(id) != me
            && !out.contains(&id)
        {
            out.push(id);
        }
    }
    out
}
