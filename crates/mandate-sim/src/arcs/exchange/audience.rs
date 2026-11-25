//! The audience (EXCHANGE_SCENES §2.2): groups bound from live state.
//!
//! Content names a *source*; this binds it to what the jurisdiction has.
//! A crowd (areas, classes, communities, a workforce) is cohorts: it cares
//! about worries, and approval and recognition move it. Members and people
//! are characters: they care about who they are, and opinion and memory
//! move them. Nothing here reads the player's municipality or ward: "the
//! seat" is the constituency of the race the player is standing in.

use super::landing::Known;
use super::state::{Group, GroupLean};
use crate::arcs::{ArcActor, ArcRun};
use crate::characters::{self, CharacterId};
use crate::concerns;
use crate::narrative::cast::View;
use crate::player::knowledge::{self, Subject as Learned};
use crate::politics::Post;
use crate::world::{Area, Jurisdiction, NeighborhoodId, World};
use mandate_content::condition::Worry;
use mandate_content::exchange::{
    ArcExchangeDef, AudienceDef, AudienceFrom, AudienceStart, AudienceWeight, ExchangeParams,
    MoveKind, OF_HERE, OF_RACE,
};
use mandate_content::knowledge::Level;

/// A scene shows at most this many groups.
pub const MAX_GROUPS: usize = 5;

/// The race the player is standing in: its post and its neighbourhoods.
pub fn my_race(v: &View<'_>) -> Option<(Post, Vec<usize>)> {
    let me = v.player()?.politician;
    v.pol
        .elections
        .races
        .iter()
        .find(|r| r.candidates.iter().any(|c| c.politician == me))
        .map(|r| (r.seat, r.hoods.iter().map(|h| *h as usize).collect()))
}

/// The area a crowd source is taken `of`: the race's constituency, the
/// run's jurisdiction, or a cast area (a cast place: its neighbourhood).
fn base_area(v: &View<'_>, run: &ArcRun, of: Option<&str>) -> Option<Area> {
    match of.unwrap_or(OF_HERE) {
        OF_RACE => {
            let (post, _) = my_race(v)?;
            Some(match post {
                Post::Seat {
                    constituency: Some(id),
                    ..
                } => Area::Constituency { id },
                other => Area::In { at: other.at() },
            })
        }
        OF_HERE => Some(Area::In { at: run.at }),
        role => match run.actor(role)? {
            ArcActor::Area { area } => Some(area),
            ArcActor::Place { id } => v.r.world.places.get(id.index()).map(|p| Area::Hood {
                id: p.neighborhood,
            }),
            ArcActor::Jurisdiction { at } => Some(Area::In { at }),
            ArcActor::Person { .. } => None,
        },
    }
}

/// The areas one level down from an area: a constituency's or a town's
/// neighbourhoods, a region's towns, the nation's regions. A constituency
/// that spans towns is its towns.
fn below(world: &World, area: Area) -> Vec<Area> {
    let hoods_as_areas = |hoods: Vec<usize>| -> Vec<Area> {
        let mut towns: Vec<u32> = hoods
            .iter()
            .filter_map(|h| world.neighborhoods.get(*h).map(|n| n.municipality.0))
            .collect();
        towns.sort_unstable();
        towns.dedup();
        if towns.len() > 1 {
            towns
                .into_iter()
                .map(|m| Area::In {
                    at: Jurisdiction::town(crate::world::MunicipalityId(m)),
                })
                .collect()
        } else {
            hoods
                .into_iter()
                .map(|h| Area::Hood {
                    id: NeighborhoodId(h as u32),
                })
                .collect()
        }
    };
    match area {
        Area::Hood { .. } => vec![area],
        Area::Constituency { .. } => hoods_as_areas(area.hoods(world)),
        Area::In { at } => crate::arcs::cast::areas_below(world, at),
    }
}

/// The cohorts of a crowd group: (neighbourhood index, cohort index).
fn cohorts(v: &View<'_>, g: &Group) -> Vec<(usize, usize)> {
    let pop = v.r.population();
    let mut out = Vec::new();
    let mut add = |area: Area, class: Option<u8>, community: Option<u16>| {
        for h in area.hoods(v.r.world) {
            let Some(hp) = pop.hoods.get(h) else { continue };
            for ci in 0..hp.cohorts.len() {
                if class.is_some_and(|c| crate::population::class_of(ci) != usize::from(c)) {
                    continue;
                }
                if community
                    .is_some_and(|k| crate::population::community_of(ci) != usize::from(k))
                {
                    continue;
                }
                out.push((h, ci));
            }
        }
    };
    match g {
        Group::Area {
            area,
            class,
            community,
        } => add(*area, *class, *community),
        Group::Elsewhere { areas } => {
            for a in areas {
                add(*a, None, None);
            }
        }
        Group::Member { .. } | Group::Person { .. } => {}
    }
    out
}

/// A crowd group's neighbourhoods (world indices).
pub fn hoods_of(world: &World, g: &Group) -> Vec<usize> {
    let mut out: Vec<usize> = match g {
        Group::Area { area, .. } => area.hoods(world),
        Group::Elsewhere { areas } => areas.iter().flat_map(|a| a.hoods(world)).collect(),
        Group::Member { .. } | Group::Person { .. } => Vec::new(),
    };
    out.sort_unstable();
    out.dedup();
    out
}

/// A crowd group's voters (`count × turnout`), and its approval of the
/// player among those who know them.
fn crowd_reading(v: &View<'_>, g: &Group) -> (f64, f64) {
    let pop = v.r.population();
    let (mut voters, mut acc, mut aware) = (0.0, 0.0, 0.0);
    for (h, ci) in cohorts(v, g) {
        let c = &pop.hoods[h].cohorts[ci];
        let n = f64::from(c.count);
        voters += n * f64::from(c.turnout);
        let a = n * f64::from(c.player_awareness);
        acc += a * f64::from(c.player_approval);
        aware += a;
    }
    (voters, if aware > 0.0 { acc / aware } else { 0.0 })
}

/// A member's support of the player: their opinion, a step for sharing a
/// party and a step for owing the player a favour (the three terms the
/// `body_support` condition counts).
fn support(v: &View<'_>, p: &ExchangeParams, id: CharacterId) -> f64 {
    let Some(me) = v.me() else { return 0.0 };
    let party_of = |c: CharacterId| {
        v.pol
            .characters
            .get(c)
            .and_then(|x| x.politician)
            .and_then(|x| v.pol.get(x))
            .and_then(|x| x.party)
    };
    let mut s = f64::from(characters::opinion(v.pol, id, me)) / 100.0;
    if party_of(me).is_some() && party_of(me) == party_of(id) {
        s += p.member_fit.party_step;
    }
    if characters::favours_owed(v.pol, id, me) > 0 {
        s += p.member_fit.favour_step;
    }
    s.clamp(-1.0, 1.0)
}

/// The groups one source gives, each with a raw weight and its start.
fn source(
    v: &View<'_>,
    run: &ArcRun,
    p: &ExchangeParams,
    a: &AudienceDef,
) -> Vec<(Group, f64, f64)> {
    let world = v.r.world;
    let weigh = |g: Group| {
        let (voters, approval) = crowd_reading(v, &g);
        let w = match a.weight {
            AudienceWeight::Voters => voters,
            AudienceWeight::Even => 1.0,
        };
        (g, w, approval)
    };
    let mut out: Vec<(Group, f64, f64)> = match a.from {
        AudienceFrom::Areas | AudienceFrom::Workers => {
            let Some(base) = base_area(v, run, a.of.as_deref()) else {
                return Vec::new();
            };
            let areas = if a.from == AudienceFrom::Workers {
                // Until plants say where their workers live (DEC-110):
                // the area itself.
                vec![base]
            } else {
                below(world, base)
            };
            areas
                .into_iter()
                .map(|area| {
                    weigh(Group::Area {
                        area,
                        class: None,
                        community: None,
                    })
                })
                .collect()
        }
        AudienceFrom::Classes => {
            let Some(area) = base_area(v, run, a.of.as_deref()) else {
                return Vec::new();
            };
            (0..crate::population::CLASSES as u8)
                .map(|c| {
                    weigh(Group::Area {
                        area,
                        class: Some(c),
                        community: None,
                    })
                })
                .collect()
        }
        AudienceFrom::Communities => {
            let Some(area) = base_area(v, run, a.of.as_deref()) else {
                return Vec::new();
            };
            let n = v.r.population().community_keys.len().max(1);
            (0..n as u16)
                .map(|k| {
                    weigh(Group::Area {
                        area,
                        class: None,
                        community: Some(k),
                    })
                })
                .collect()
        }
        AudienceFrom::Members => {
            let Some(power) = a.power else {
                return Vec::new();
            };
            let me = v.me();
            crate::arcs::cast::power_holders(v, run.at, power)
                .into_iter()
                .filter_map(|id| v.pol.get(id).map(|x| x.character))
                .filter(|c| Some(*c) != me)
                .map(|id| (Group::Member { id }, 1.0, support(v, p, id)))
                .collect()
        }
        AudienceFrom::Person => {
            let Some(id) = a.role.as_deref().and_then(|r| run.person(r)) else {
                return Vec::new();
            };
            let opinion = v
                .me()
                .map_or(0.0, |me| f64::from(characters::opinion(v.pol, id, me)) / 100.0);
            let start = match a.start {
                AudienceStart::Demand => opinion - p.member_fit.demand,
                AudienceStart::Support => support(v, p, id),
                _ => opinion,
            };
            vec![(Group::Person { id }, 1.0, start.clamp(-1.0, 1.0))]
        }
    };
    // Nobody lives there: no group.
    if a.from.crowd() {
        out.retain(|x| x.1 > 0.0);
    }
    // Where a source gives more than it may show, the heaviest are kept
    // and the rest are one group ("elsewhere").
    let max = usize::from(a.max.unwrap_or(MAX_GROUPS as u8)).clamp(1, MAX_GROUPS);
    if out.len() > max && a.from.crowd() && max >= 2 {
        let mut order: Vec<usize> = (0..out.len()).collect();
        order.sort_by(|x, y| out[*y].1.total_cmp(&out[*x].1).then(x.cmp(y)));
        let keep: Vec<usize> = {
            let mut k = order[..max - 1].to_vec();
            k.sort_unstable();
            k
        };
        let rest: Vec<Area> = out
            .iter()
            .enumerate()
            .filter(|(i, _)| !keep.contains(i))
            .filter_map(|(_, x)| match &x.0 {
                Group::Area { area, .. } => Some(*area),
                _ => None,
            })
            .collect();
        let mut kept: Vec<(Group, f64, f64)> =
            keep.into_iter().map(|i| out[i].clone()).collect();
        kept.push(weigh(Group::Elsewhere { areas: rest }));
        out = kept;
    } else if out.len() > max {
        out.truncate(max);
    }
    out
}

/// Bind a scene's audience (§2.2): each source's groups, weights summing
/// to 1 (each source an equal share), each starting where the simulation
/// has it. Empty when nothing binds (the scene cannot be held).
pub fn bind(
    v: &View<'_>,
    run: &ArcRun,
    def: &ArcExchangeDef,
    p: &ExchangeParams,
) -> Vec<GroupLean> {
    let sources: Vec<Vec<(Group, f64, f64)>> = def
        .audience
        .iter()
        .map(|a| source(v, run, p, a))
        .filter(|s| !s.is_empty())
        .collect();
    let share = 1.0 / sources.len().max(1) as f64;
    let mut out: Vec<GroupLean> = Vec::new();
    for s in sources {
        let total: f64 = s.iter().map(|x| x.1).sum();
        for (group, w, start) in s {
            if out.len() >= MAX_GROUPS || out.iter().any(|g| g.group == group) {
                continue;
            }
            let weight = if total > 0.0 { share * w / total } else { 0.0 };
            out.push(GroupLean {
                group,
                weight: weight as f32,
                start: start as f32,
                lean: start as f32,
            });
        }
    }
    // Weights sum to 1 whatever was dropped.
    let total: f32 = out.iter().map(|g| g.weight).sum();
    if total > 0.0 {
        for g in &mut out {
            g.weight /= total;
        }
    }
    out
}

/// How well the player knows a crowd group's worries: every one of its
/// neighbourhoods', or its town's where it is a whole town.
pub fn worries_known(v: &View<'_>, g: &Group) -> Level {
    let pl = v.player();
    let hoods = hoods_of(v.r.world, g);
    let by_hood = hoods
        .iter()
        .map(|h| {
            knowledge::level_of(
                pl,
                &Learned::HoodConcerns {
                    hood: NeighborhoodId(*h as u32),
                },
            )
        })
        .min()
        .unwrap_or(Level::Unknown);
    let by_town = match g {
        Group::Area {
            area:
                Area::In {
                    at: Jurisdiction::Municipality { id },
                },
            class: None,
            community: None,
        } => knowledge::level_of(pl, &Learned::TownConcerns { town: *id }),
        _ => Level::Unknown,
    };
    by_hood.max(by_town)
}

/// How well the player knows a member or a person.
pub fn person_known(v: &View<'_>, id: CharacterId) -> Level {
    knowledge::level_of(v.player(), &Learned::Person { person: id })
}

/// A group's worries, strongest first: (concern index, weight), as its
/// residents feel them or as a statistic shows them.
pub fn worries(v: &View<'_>, g: &Group, by: Worry) -> Vec<(usize, f64)> {
    let hoods = hoods_of(v.r.world, g);
    if hoods.is_empty() {
        return Vec::new();
    }
    concerns::ranked(
        v.r.population(),
        v.content(),
        v.r.world,
        concerns::Scope::District(&hoods),
        by,
    )
}

/// A group's top worry, as its residents feel it: (content id, level 0..1).
pub fn top_worry(v: &View<'_>, g: &Group) -> Option<(String, f64)> {
    let ranking = worries(v, g, Worry::Felt);
    let (i, w) = concerns::top(&ranking, 1).next()?;
    Some((v.content().concerns.get(i)?.id.clone(), w.clamp(0.0, 1.0)))
}

/// The fit of a worry by its place in a ranking (§5.3).
fn by_rank(p: &ExchangeParams, ranking: &[(usize, f64)], concern: usize) -> f64 {
    match concerns::place(ranking, concern) {
        Some((0, _)) => p.fit.top,
        Some((1 | 2, _)) => p.fit.next,
        _ => p.fit.low,
    }
}

/// How much a crowd group cares about a subject, read `by` a ranking (the
/// truth is what residents feel; a weak opponent reads the statistic).
pub fn crowd_fit(
    v: &View<'_>,
    p: &ExchangeParams,
    g: &Group,
    concern: Option<&str>,
    by: Worry,
) -> f64 {
    let Some(c) = concern else { return p.fit.flat };
    let Some(i) = v.content().concerns.iter().position(|x| x.id == c) else {
        return p.fit.flat;
    };
    by_rank(p, &worries(v, g, by), i)
}

/// A crowd group's fit for a subject, with the ends the player's knowledge
/// of its worries allows (§5.7). The ledger gives the first worry at
/// `heard`, three at `known`, all at `detailed`: so the place of any worry
/// among "top, next, low" is exact from `known`, and from `heard` only the
/// top is.
fn crowd_fit_known(v: &View<'_>, p: &ExchangeParams, g: &Group, concern: Option<&str>) -> Known {
    let value = crowd_fit(v, p, g, concern, Worry::Felt);
    if concern.is_none() {
        return Known::exact(value);
    }
    match worries_known(v, g) {
        Level::Detailed | Level::Known => Known::exact(value),
        Level::Heard if value == p.fit.top => Known::exact(value),
        Level::Heard => Known::between(value, p.fit.low, p.fit.next),
        Level::Unknown => Known::between(value, p.fit.low, p.fit.top),
    }
}

/// How much a member or a person cares about a line of a kind (§5.3): who
/// they are, not a worry.
fn person_fit(
    v: &View<'_>,
    p: &ExchangeParams,
    id: CharacterId,
    kind: Option<MoveKind>,
    concern: Option<&str>,
) -> f64 {
    let f = &p.member_fit;
    let Some(profile) = characters::profile(v.pol, v.r.world, id) else {
        return f.base;
    };
    let mut x = f.base;
    if let Some(k) = kind {
        let w = |t: &mandate_content::exchange::ByKind| t.get(&k).copied().unwrap_or(0.0);
        x += w(&f.integrity) * f64::from(profile.personality.integrity);
        let mine = v
            .player()
            .and_then(|pl| v.pol.get(pl.politician))
            .and_then(|me| me.party);
        let theirs = profile
            .politician
            .and_then(|q| v.pol.get(q))
            .and_then(|q| q.party);
        if mine.is_some() && mine == theirs {
            x += w(&f.loyalty) * f64::from(profile.personality.loyalty);
        }
    }
    if let Some(me) = v.me() {
        let memories = characters::salient_memories(v.pol, id, Some(me), v.date(), 3);
        let feeling: f32 = memories.iter().map(|m| m.sentiment).sum();
        if feeling > 0.0 {
            x += f.memory;
        } else if feeling < 0.0 {
            x -= f.memory;
        }
    }
    // (A subject that matches one of their goals will count as `goal`
    // once goals name what they are about: with the hearing's arc.)
    let _ = concern;
    x.clamp(f.min, f.max)
}

/// How much each group cares about a subject (§5.3), with what the player
/// can know of it. `kind`: the line's, where a person's fit turns on it.
pub fn fit(
    v: &View<'_>,
    p: &ExchangeParams,
    groups: &[GroupLean],
    kind: Option<MoveKind>,
    concern: Option<&str>,
) -> Vec<Known> {
    groups
        .iter()
        .map(|g| match g.group.person() {
            None => crowd_fit_known(v, p, &g.group, concern),
            Some(id) => {
                let value = person_fit(v, p, id, kind, concern);
                match person_known(v, id) {
                    Level::Detailed | Level::Known => Known::exact(value),
                    _ => Known::between(value, p.member_fit.min, p.member_fit.max),
                }
            }
        })
        .collect()
}

/// A backer's tie to each group (§3.1, `second`): top where they live,
/// work or are a member; low elsewhere.
pub fn ties(
    v: &View<'_>,
    p: &ExchangeParams,
    groups: &[GroupLean],
    backer: CharacterId,
) -> Vec<Known> {
    let world = v.r.world;
    let profile = characters::profile(v.pol, world, backer);
    let home = profile.as_ref().map(|x| x.home as usize);
    let work = profile
        .as_ref()
        .and_then(|x| x.place)
        .and_then(|pl| world.places.get(pl.index()))
        .map(|pl| pl.neighborhood.index());
    groups
        .iter()
        .map(|g| {
            let tied = match g.group.person() {
                Some(id) => {
                    id == backer || f64::from(characters::opinion(v.pol, id, backer)) >= 10.0
                }
                None => {
                    let hoods = hoods_of(world, &g.group);
                    home.is_some_and(|h| hoods.contains(&h))
                        || work.is_some_and(|h| hoods.contains(&h))
                }
            };
            Known::exact(if tied { p.fit.top } else { p.fit.low })
        })
        .collect()
}

/// A group's name for the text: an area's own name; `None` for the rest
/// ("elsewhere") and for a class or a community, which the view words by
/// key.
pub fn name(v: &View<'_>, g: &Group) -> Option<String> {
    match g {
        Group::Area { area, .. } => Some(crate::arcs::cast::area_name(v.r.world, *area)),
        Group::Elsewhere { .. } => None,
        Group::Member { id } | Group::Person { id } => {
            characters::profile(v.pol, v.r.world, *id).map(|x| x.full_name())
        }
    }
}

/// The approval and recognition requests for a crowd group (§5.5): the
/// area, the classes and the communities they land on.
pub fn scopes(g: &Group) -> Vec<(Area, Vec<u8>, Vec<u16>)> {
    match g {
        Group::Area {
            area,
            class,
            community,
        } => vec![(
            *area,
            class.iter().copied().collect(),
            community.iter().copied().collect(),
        )],
        Group::Elsewhere { areas } => areas
            .iter()
            .map(|a| (*a, Vec::new(), Vec::new()))
            .collect(),
        Group::Member { .. } | Group::Person { .. } => Vec::new(),
    }
}
