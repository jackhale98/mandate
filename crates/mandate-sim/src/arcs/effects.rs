//! An arc's effects as effect requests (OVERVIEW §4.3, STAGED_ARCS §4.3).
//! The arcs domain writes nothing but its own state: what a beat, a stage
//! or an exit changes elsewhere is a request the owning system applies
//! (`Opinion`, `Favour`, `Record`: characters and politics; `Approval`,
//! `Displacement`: population; `PlayerTrack`: the player; `Recovery`: the
//! environment; `Budget`: finance), each with its cause. Only the run's
//! heat is changed here.
//!
//! The beats a player takes with a command (a visit, a reply) run outside
//! the scheduler and apply the same requests at once through the owners'
//! own functions ([`apply_now`]), as the asks' commands do.

use super::ArcRun;
use crate::narrative::cast::View;
use crate::scheduler::{Effect, Outs, Request};
use crate::spine::Cause;
use crate::world::{Area, NeighborhoodId};
use mandate_content::ContentBundle;
use mandate_content::arcs::{ArcEffect, DisplacedOf, GatedEffect};

/// What a list of effects asks for: requests to other domains, and a
/// change to the run's own heat.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asked {
    pub requests: Vec<Effect>,
    pub heat: f32,
}

/// The effects whose gates hold now, as requests. A role the run could
/// not cast (an optional one) is skipped, as is anything about the player
/// when there is none.
pub fn resolve(v: &View<'_>, run: &ArcRun, effects: &[GatedEffect]) -> Asked {
    let mut out = Asked::default();
    if effects.is_empty() {
        return out;
    }
    let r = v.r;
    let people = run.narrative_cast();
    let me = v.me();
    for g in effects {
        if !super::step::holds(v, run, &people, &g.when) {
            continue;
        }
        if !g.unless.is_empty() && super::step::holds(v, run, &people, &g.unless) {
            continue;
        }
        match &g.effect {
            ArcEffect::Opinion {
                role,
                delta,
                memory,
            } => {
                if let (Some(holder), Some(toward)) = (run.person(role), me) {
                    out.requests.push(Effect::Opinion {
                        holder,
                        toward,
                        delta: *delta,
                        memory: memory
                            .as_deref()
                            .and_then(crate::asks::effects::memory_kind),
                    });
                }
            }
            ArcEffect::Approval {
                area,
                delta,
                classes,
            } => {
                if let Some(scope) = run.area(area) {
                    out.requests.push(Effect::Approval {
                        scope,
                        delta: *delta,
                        classes: classes.clone(),
                    });
                }
            }
            ArcEffect::Favour {
                role,
                owed_to_player,
            } => {
                if let (Some(holder), Some(toward)) = (run.person(role), me) {
                    out.requests.push(Effect::Favour {
                        holder,
                        toward,
                        owed_to_player: *owed_to_player,
                    });
                }
            }
            ArcEffect::Record { delta } => {
                if let Some(pl) = v.player() {
                    out.requests.push(Effect::Record {
                        politician: pl.politician,
                        delta: *delta,
                    });
                }
            }
            ArcEffect::Standing { delta } => {
                if v.player().is_some() {
                    out.requests.push(Effect::PlayerTrack {
                        standing: *delta as f32,
                        viability: 0.0,
                        money: 0.0,
                        legal_floor: None,
                    });
                }
            }
            ArcEffect::Recovery {
                area,
                repair,
                months,
            } => {
                for h in run.area(area).map(|a| a.hoods(r.world)).unwrap_or_default() {
                    out.requests.push(Effect::Recovery {
                        neighborhood: h as u32,
                        factor: *repair,
                        months: *months,
                    });
                }
            }
            ArcEffect::Displacement {
                area,
                of: DisplacedOf::StillAway,
            } => {
                let Some(a) = run.area(area) else { continue };
                out.requests.extend(still_away(v, a));
            }
            ArcEffect::Budget { factor, category } => out.requests.push(Effect::Budget {
                at: run.at,
                category: category.map(crate::policy::category_index),
                factor: *factor,
            }),
            ArcEffect::Heat { delta } => out.heat += *delta,
            // Not yet (`mandate_content::arcs::not_yet`): never raised.
            ArcEffect::GovApproval { .. } | ArcEffect::PlaceState { .. } => {}
        }
    }
    out
}

/// Those still staying away from an area leave for good: a share of each
/// of its towns' residents equal to them (the population applies it to
/// the town's mobile adults, as it does after a disaster).
fn still_away(v: &View<'_>, area: Area) -> Vec<Effect> {
    let r = v.r;
    let pop = r.population();
    let mut by_town: Vec<(u32, u64)> = Vec::new();
    for h in area.hoods(r.world) {
        let Some(m) = r.world.neighborhoods.get(h).map(|n| n.municipality.0) else {
            continue;
        };
        let n = pop.displaced_from(NeighborhoodId(h as u32));
        match by_town.iter_mut().find(|x| x.0 == m) {
            Some(x) => x.1 += n,
            None => by_town.push((m, n)),
        }
    }
    by_town
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .filter_map(|(m, n)| {
            let total = pop.municipality_total(r.world, crate::world::MunicipalityId(m));
            (total > 0).then(|| Effect::Displacement {
                municipality: m,
                fraction: n as f64 / total as f64,
            })
        })
        .collect()
}

/// A request whose applier runs monthly: it would wait outside the state
/// between advances, so the arcs domain holds it until the month turns.
pub fn monthly(e: &Effect) -> bool {
    matches!(e, Effect::Displacement { .. })
}

/// Post requests from inside the scheduler, each with its cause. Those for
/// a monthly applier are kept in `deferred` until the month's first day.
pub fn post(out: &mut Outs<'_>, deferred: &mut Vec<Request>, effects: Vec<Effect>, cause: Cause) {
    for effect in effects {
        if monthly(&effect) {
            deferred.push(Request { effect, cause });
        } else {
            out.effects.push(effect, cause);
        }
    }
}

/// Apply requests at once, outside the scheduler (a beat the player takes
/// with a command), through the functions the appliers use.
pub fn apply_now(
    state: &mut crate::State,
    content: &ContentBundle,
    effects: Vec<Effect>,
    cause: Cause,
) {
    let mut rest = Vec::new();
    for effect in effects {
        match effect {
            Effect::Recovery {
                neighborhood,
                factor,
                months,
            } => {
                state.events.note_effect(&cause);
                crate::environment::boost_repairs(
                    &mut state.environment,
                    state.calendar.date,
                    neighborhood,
                    factor,
                    months,
                );
            }
            Effect::Budget {
                at,
                category,
                factor,
            } => {
                state.events.note_effect(&cause);
                crate::finance::vote_budget(&mut state.finance, at, category, factor);
            }
            e if monthly(&e) => state.arcs.deferred.push(Request { effect: e, cause }),
            e => rest.push(Request { effect: e, cause }),
        }
    }
    crate::asks::effects::apply_now(state, content, rest);
}
