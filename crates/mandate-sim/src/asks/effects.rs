//! An ask's effects (content `keep`, `break`, `decline`) as effect requests
//! (OVERVIEW §4.3): the asks system resolves asks inside the scheduler, so
//! it never writes another domain; each request is applied by its owner
//! (`Opinion`, `Favour`, `Record`: characters and politics; `Approval`:
//! population; `PlayerTrack`, `Learn`: the player). The asks' own commands
//! (declining an offer, accepting one) run outside the scheduler and apply
//! the same requests at once through the same domain functions
//! ([`apply_now`]).

use super::Ask;
use crate::characters::{CharacterId, MemoryKind};
use crate::narrative::cast::View;
use crate::narrative::{Actor, actor_of, person_of};
use crate::scheduler::{Effect, Reads};
use crate::world::{Area, Jurisdiction, NeighborhoodId};
use mandate_content::{ContentBundle, EffectDef, Scope};

/// The memory kind a content key names.
pub fn memory_kind(key: &str) -> Option<MemoryKind> {
    MemoryKind::ALL.iter().copied().find(|k| k.key() == key)
}

/// The player's own constituency (their seat's), else their ward.
pub fn player_constituency(r: &Reads<'_>) -> Option<crate::world::ConstituencyId> {
    let pl = r.player?;
    let post = r.politics().get(pl.politician).and_then(|p| p.role.post());
    match post {
        Some(crate::politics::Post::Seat {
            constituency: Some(c),
            ..
        }) => Some(c),
        _ => Some(pl.ward),
    }
}

/// The area an approval effect reaches: the neighbourhood cast in its
/// role; the race's constituency (a promise) or the player's (`ward`); the
/// player's home jurisdiction (`town`).
fn area(r: &Reads<'_>, ask: &Ask, scope: Scope, neighborhood: Option<&str>) -> Option<Area> {
    if let Some(role) = neighborhood {
        return match actor_of(&ask.cast, role)? {
            Actor::Neighborhood { id } => Some(Area::Hood {
                id: NeighborhoodId(id),
            }),
            _ => None,
        };
    }
    match scope {
        Scope::Ward => {
            if let Some(post) = ask.race {
                return Some(match post {
                    crate::politics::Post::Seat {
                        constituency: Some(c),
                        ..
                    } => Area::Constituency { id: c },
                    other => Area::In { at: other.at() },
                });
            }
            player_constituency(r).map(|id| Area::Constituency { id })
        }
        Scope::Town => r.player.map(|p| Area::In {
            at: Jurisdiction::town(p.home()),
        }),
    }
}

/// Effect requests for a list of an ask's effects.
pub fn requests(r: &Reads<'_>, ask: &Ask, defs: &[EffectDef]) -> Vec<Effect> {
    let Some(pl) = r.player else {
        return Vec::new();
    };
    let pol = r.politics();
    let me = pol.get(pl.politician).map(|p| p.character);
    let mut out = Vec::new();
    for d in defs {
        match d {
            EffectDef::Opinion {
                role,
                delta,
                memory,
            } => {
                if let (Some(holder), Some(toward)) = (person_of(&ask.cast, role), me) {
                    out.push(Effect::Opinion {
                        holder,
                        toward,
                        delta: *delta,
                        memory: memory.as_deref().and_then(memory_kind),
                    });
                }
            }
            EffectDef::Approval {
                scope,
                classes,
                neighborhood,
                delta,
            } => {
                if let Some(a) = area(r, ask, *scope, neighborhood.as_deref()) {
                    out.push(Effect::Approval {
                        scope: a,
                        delta: *delta,
                        classes: classes.clone(),
                        communities: Vec::new(),
                    });
                }
            }
            EffectDef::Standing { delta } => out.push(Effect::PlayerTrack {
                standing: *delta as f32,
                viability: 0.0,
                money: 0.0,
                legal_floor: None,
            }),
            EffectDef::Money { amount } => out.push(Effect::PlayerTrack {
                standing: 0.0,
                viability: 0.0,
                money: *amount,
                legal_floor: None,
            }),
            EffectDef::Record { delta } => out.push(Effect::Record {
                politician: pl.politician,
                delta: *delta,
            }),
            EffectDef::Favour {
                role,
                owed_to_player,
            } => {
                if let (Some(holder), Some(toward)) = (person_of(&ask.cast, role), me) {
                    out.push(Effect::Favour {
                        holder,
                        toward,
                        owed_to_player: *owed_to_player,
                    });
                }
            }
            EffectDef::Learn { subject, level } => {
                let v = View::new(r, pol);
                let source = match ask.asker_person() {
                    Some(person) => crate::player::knowledge::Source::Met { person },
                    None => crate::player::knowledge::Source::Action {
                        action: "ask".into(),
                        place: None,
                    },
                };
                for g in crate::narrative::learn::grants(&v, subject, *level, &ask.cast, None) {
                    out.push(Effect::Learn {
                        subject: g.subject,
                        level: g.level,
                        source: source.clone(),
                    });
                }
            }
            // Validation refuses the rest for asks.
            _ => {}
        }
    }
    out
}

/// What keeping, breaking or declining adds to the template's own effects:
/// the record (ENGAGEMENT §2.2), and for a kept big ask a favour owed to
/// the player.
pub fn automatic(r: &Reads<'_>, ask: &Ask, outcome: Outcome) -> Vec<Effect> {
    let Some(pl) = r.player else {
        return Vec::new();
    };
    let g = &r.content.career.record_gain;
    let mut out = Vec::new();
    let record = match outcome {
        Outcome::Kept if ask.promise => g.promise_kept,
        Outcome::Kept => g.ask_kept,
        Outcome::Broken => g.ask_broken,
        Outcome::Declined => 0.0,
    };
    if record != 0.0 {
        out.push(Effect::Record {
            politician: pl.politician,
            delta: record,
        });
    }
    let me = r.politics().get(pl.politician).map(|p| p.character);
    if outcome == Outcome::Kept
        && ask.big
        && let (Some(holder), Some(toward)) = (ask.asker_person(), me)
    {
        out.push(Effect::Favour {
            holder,
            toward,
            owed_to_player: true,
        });
    }
    out
}

/// How an ask ended (for its effects).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Kept,
    Broken,
    Declined,
}

/// Apply effect requests at once, outside the scheduler (the asks' own
/// commands), through the functions the appliers use.
pub fn apply_now(
    state: &mut crate::State,
    content: &ContentBundle,
    effects: Vec<crate::scheduler::Request>,
) {
    let date = state.calendar.date;
    let seed = state.meta.seed;
    for r in effects {
        state.events.note_effect(&r.cause);
        let e = r.effect;
        match e {
            Effect::Opinion { .. } | Effect::Favour { .. } | Effect::Record { .. } => {
                crate::characters::apply_ask_effect(&mut state.politics, content, seed, date, &e);
            }
            Effect::Approval {
                scope,
                delta,
                classes,
                communities,
            } => {
                crate::population::apply_approval_in(
                    &mut state.population,
                    &state.world,
                    scope,
                    delta,
                    &classes,
                    &communities,
                );
            }
            Effect::PlayerTrack {
                standing, money, ..
            } => {
                if let Some(p) = state.player.as_mut() {
                    p.tracks.standing = (p.tracks.standing + standing).clamp(0.0, 100.0);
                    if money != 0.0 {
                        let m = (p.money.units() as f64 + money).max(0.0);
                        p.money = mandate_core::Money::from_units(m.round() as i64);
                    }
                }
            }
            Effect::Learn {
                subject,
                level,
                source,
            } => {
                crate::player::knowledge::teach_one(state, subject, level, source);
            }
            _ => {}
        }
    }
}

/// The asker remembers the player's promise (accepting an offer).
pub fn promised(asker: Option<CharacterId>, me: Option<CharacterId>) -> Vec<Effect> {
    match (asker, me) {
        (Some(holder), Some(toward)) => vec![Effect::Opinion {
            holder,
            toward,
            delta: 0.0,
            memory: Some(MemoryKind::PromisedFavour),
        }],
        _ => Vec::new(),
    }
}
