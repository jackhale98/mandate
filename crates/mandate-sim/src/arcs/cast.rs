//! Casting an arc's roles (STAGED_ARCS §4.1): each role binds to something
//! real, relative to the arc's jurisdiction and never the player's town.
//! People are cast by occupation or by a power their post lists (never an
//! office's name); areas step down one level; places are of a kind, near an
//! area. Deterministic: candidates are ranked by simulated state and then
//! by id, so a cast draws nothing.
//!
//! Also here: the facts a run's text is given (its cast's names, its dates,
//! the readings of its template's `facts`), and the readings a run takes
//! as it is raised.

use super::{ArcActor, ArcCastEntry, ArcRun};
use crate::characters::{self, CharacterId};
use crate::condition::Args;
use crate::narrative::cast::View;
use crate::scheduler::Reads;
use crate::world::{Area, Jurisdiction, NeighborhoodId, World};
use mandate_content::arcs::{ArcCast, ArcDef, ArcRoleDef, AreaPick, JurisdictionPick};
use mandate_content::{HazardKey, Power};
use mandate_core::SimDate;

/// The areas one level down from a jurisdiction: of a town, its
/// neighbourhoods; of a region, its towns; of the nation, its regions.
pub fn areas_below(world: &World, at: Jurisdiction) -> Vec<Area> {
    match at {
        Jurisdiction::Municipality { id } => world
            .hoods_of(id)
            .map(|h| Area::Hood {
                id: NeighborhoodId(h as u32),
            })
            .collect(),
        Jurisdiction::Region { id } => world
            .regions
            .get(id.index())
            .map(|r| {
                r.municipalities
                    .iter()
                    .map(|m| Area::In {
                        at: Jurisdiction::town(*m),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        Jurisdiction::Nation => world
            .regions
            .iter()
            .map(|r| Area::In {
                at: Jurisdiction::Region { id: r.id },
            })
            .collect(),
    }
}

/// The hazard as the environment keeps it.
pub fn hazard_kind(h: HazardKey) -> crate::environment::HazardKind {
    use crate::environment::HazardKind;
    match h {
        HazardKey::Flood => HazardKind::Flood,
        HazardKey::Storm => HazardKind::Storm,
        HazardKey::Wildfire => HazardKind::Wildfire,
    }
}

/// An area's unrepaired damage: per mille of each neighbourhood's homes ×
/// its residents as play began (a weight that never moves), summed; with
/// `hazard`, only what that hazard did.
pub fn damage(r: &Reads<'_>, area: Area, hazard: Option<HazardKey>) -> u64 {
    let env = r.environment();
    let pop = r.population();
    area.hoods(r.world)
        .into_iter()
        .filter_map(|h| {
            let d = env.damage(NeighborhoodId(h as u32))?;
            if hazard.is_some_and(|k| hazard_kind(k) != d.hazard) {
                return None;
            }
            let w = pop.hoods.get(h).map_or(0, |x| x.baseline_population.max(1));
            Some(u64::from(d.damaged_pm) * u64::from(w))
        })
        .sum()
}

/// Homes in an area: its town's dwellings by its share of the town's
/// residents as play began.
fn homes_in(r: &Reads<'_>, area: Area) -> f64 {
    let pop = r.population();
    let econ = r.economy();
    let base = |h: usize| {
        pop.hoods
            .get(h)
            .map_or(0.0, |x| f64::from(x.baseline_population))
    };
    area.hoods(r.world)
        .into_iter()
        .filter_map(|h| {
            let m = r.world.neighborhoods.get(h)?.municipality;
            let town: f64 = r.world.hoods_of(m).map(base).sum();
            let stock = econ.munis.get(m.index())?.housing.stock;
            (town > 0.0).then(|| stock * base(h) / town)
        })
        .sum()
}

/// Homes damaged and not yet repaired in an area (an estimate in whole
/// homes: the damaged share of each neighbourhood × its homes).
pub fn damaged_homes(r: &Reads<'_>, area: Area) -> u64 {
    let env = r.environment();
    area.hoods(r.world)
        .into_iter()
        .map(|h| {
            let id = NeighborhoodId(h as u32);
            let pm = env.damage(id).map_or(0.0, |d| f64::from(d.damaged_pm));
            pm / 1000.0 * homes_in(r, Area::Hood { id })
        })
        .sum::<f64>()
        .round() as u64
}

/// People of an area staying elsewhere after a disaster, and its residents
/// (those away included).
pub fn away(r: &Reads<'_>, area: Area) -> (u64, u64) {
    let pop = r.population();
    let (mut away, mut home) = (0u64, 0u64);
    for h in area.hoods(r.world) {
        away += pop.displaced_from(NeighborhoodId(h as u32));
        home += pop.hood_total(h);
    }
    (away, home + away)
}

/// The player's mean approval among an area's residents.
fn approval(r: &Reads<'_>, area: Area) -> f64 {
    let pop = r.population();
    let (mut acc, mut n) = (0.0, 0.0);
    for h in area.hoods(r.world) {
        for c in pop
            .hoods
            .get(h)
            .map(|x| x.cohorts.as_slice())
            .unwrap_or(&[])
        {
            acc += f64::from(c.player_approval) * f64::from(c.count);
            n += f64::from(c.count);
        }
    }
    if n > 0.0 { acc / n } else { 0.0 }
}

/// A person's town (a named person's home, a politician's).
fn town_of(v: &View<'_>, id: CharacterId) -> Option<u32> {
    v.town_of(id)
}

/// Whether a town is inside a jurisdiction.
fn town_in(world: &World, at: Jurisdiction, m: u32) -> bool {
    match at {
        Jurisdiction::Municipality { id } => id.0 == m,
        Jurisdiction::Region { id } => world
            .municipalities
            .get(m as usize)
            .is_some_and(|x| x.region == id),
        Jurisdiction::Nation => true,
    }
}

/// Every actor that could fill a role, best first.
fn candidates(
    v: &View<'_>,
    at: Jurisdiction,
    role: &ArcRoleDef,
    so_far: &[ArcCastEntry],
) -> Vec<ArcActor> {
    let r = v.r;
    let world = r.world;
    let me = v.me();
    let taken = |a: &ArcActor| so_far.iter().any(|c| &c.actor == a);
    let area_of = |near: &Option<String>| -> Option<Vec<usize>> {
        let n = near.as_deref()?;
        match so_far.iter().find(|c| c.role == n)?.actor {
            ArcActor::Area { area } => Some(area.hoods(world)),
            _ => None,
        }
    };
    let mut out: Vec<ArcActor> = match &role.cast {
        ArcCast::Occupation {
            occupations,
            near,
            at: works_at,
        } => {
            let near = area_of(near);
            // A place role: someone whose own workplace it is comes first.
            let place = works_at
                .as_deref()
                .and_then(|n| so_far.iter().find(|c| c.role == n))
                .and_then(|c| match c.actor {
                    ArcActor::Place { id } => Some(id),
                    _ => None,
                });
            let mut people: Vec<(bool, bool, usize, i64, u32)> = v
                .pol
                .characters
                .people
                .iter()
                .filter(|c| Some(c.id) != me)
                .filter_map(|c| {
                    let n = c.named.as_deref()?;
                    let order = occupations.iter().position(|o| *o == n.occupation)?;
                    if !town_of(v, c.id).is_some_and(|m| town_in(world, at, m)) {
                        return None;
                    }
                    let elsewhere = place.is_some_and(|p| n.place != Some(p));
                    let far = near
                        .as_ref()
                        .is_some_and(|hs| !hs.contains(&(n.home as usize)));
                    let influence = r
                        .content
                        .occupation(&n.occupation)
                        .map_or(0, |o| (o.influence * 1000.0) as i64);
                    Some((elsewhere, far, order, -influence, c.id.0))
                })
                .collect();
            people.sort();
            people
                .into_iter()
                .map(|p| ArcActor::Person {
                    id: CharacterId(p.4),
                })
                .collect()
        }
        ArcCast::Post { power, party } => {
            use mandate_content::PartyRel;
            let mine = v
                .player()
                .and_then(|pl| v.pol.get(pl.politician))
                .and_then(|p| p.party);
            let party_of = |c: CharacterId| {
                v.pol
                    .characters
                    .get(c)
                    .and_then(|x| x.politician)
                    .and_then(|p| v.pol.get(p))
                    .and_then(|p| p.party)
            };
            let mut holders: Vec<CharacterId> = post_holders(v, at, *power)
                .into_iter()
                .filter(|c| Some(*c) != me)
                .collect();
            // A preference, never a filter: of the player's party first
            // (whoever thinks most of them), or of another first (whoever
            // thinks least); then the rest, in the constitution's order,
            // which is the order when no party is asked for.
            if let (Some(me), false) = (me, *party == PartyRel::Any) {
                let same = *party == PartyRel::Same;
                let mut ranked: Vec<(bool, i64, usize, CharacterId)> = holders
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let theirs = party_of(*c);
                        let fits = if same {
                            mine.is_some() && theirs == mine
                        } else {
                            theirs != mine
                        };
                        let o = f64::from(characters::opinion(v.pol, *c, me));
                        let sign = if same { -1.0 } else { 1.0 };
                        (!fits, (sign * o * 100.0) as i64, i, *c)
                    })
                    .collect();
                ranked.sort();
                holders = ranked.into_iter().map(|x| x.3).collect();
            }
            holders
                .into_iter()
                .map(|id| ArcActor::Person { id })
                .collect()
        }
        // The town's named rival; in a campaign, whoever stands against
        // the player (the narrative's own rule; a town's matter).
        ArcCast::Rival => match at {
            Jurisdiction::Municipality { id } if v.town() == Some(id.0) => {
                crate::narrative::cast::candidates(v, &mandate_content::Cast::Rival, &[], None)
                    .into_iter()
                    .filter_map(|a| match a {
                        crate::narrative::Actor::Person { id } => Some(ArcActor::Person { id }),
                        _ => None,
                    })
                    .collect()
            }
            _ => Vec::new(),
        },
        // Whoever there the player owes most, the oldest debt first.
        ArcCast::Creditor => {
            let Some(me) = me else { return Vec::new() };
            let mut owed: Vec<(u32, SimDate, CharacterId)> = Vec::new();
            for f in v.pol.characters.favours.iter().filter(|f| f.debtor == me) {
                if !town_of(v, f.creditor).is_some_and(|m| town_in(world, at, m)) {
                    continue;
                }
                match owed.iter_mut().find(|x| x.2 == f.creditor) {
                    Some(x) => {
                        x.0 += u32::from(f.count);
                        x.1 = x.1.min(f.since);
                    }
                    None => owed.push((u32::from(f.count), f.since, f.creditor)),
                }
            }
            owed.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
            owed.into_iter()
                .map(|x| ArcActor::Person { id: x.2 })
                .collect()
        }
        // Where a cast person works: the place their post sits, else
        // their own workplace; inside the jurisdiction.
        ArcCast::Workplace { of } => {
            let person = so_far
                .iter()
                .find(|c| &c.role == of)
                .and_then(|c| match c.actor {
                    ArcActor::Person { id } => Some(id),
                    _ => None,
                });
            let Some(ch) = person.and_then(|p| v.pol.characters.get(p)) else {
                return Vec::new();
            };
            let kind = ch
                .politician
                .and_then(|p| crate::politics::posts_of(v.pol, p).into_iter().next())
                .filter(|post| post.at() == at)
                .and_then(|post| crate::politics::workplace_of(v.pol, post));
            let own = ch.named.as_deref().and_then(|n| n.place);
            let mut places: Vec<(u8, u32)> = world
                .places
                .iter()
                .filter(|p| p.is_open() && town_in(world, at, p.municipality.0))
                .filter_map(|p| {
                    if kind == Some(p.kind) {
                        Some((0, p.id.0))
                    } else if own == Some(p.id) {
                        Some((1, p.id.0))
                    } else {
                        None
                    }
                })
                .collect();
            places.sort();
            places
                .into_iter()
                .map(|p| ArcActor::Place {
                    id: crate::world::PlaceId(p.1),
                })
                .collect()
        }
        ArcCast::Area { pick, hazard } => {
            let env = r.environment();
            let mut areas: Vec<(i64, i64, i64, Area)> = areas_below(world, at)
                .into_iter()
                .map(|a| match pick {
                    // The largest share of homes the hazard damaged; where a
                    // strike fell evenly, where most people are away; then
                    // the most exposed.
                    AreaPick::MostExposed => {
                        let hoods = a.hoods(world);
                        let worst = hoods
                            .iter()
                            .filter_map(|h| env.damage(NeighborhoodId(*h as u32)))
                            .filter(|d| hazard.is_none_or(|k| hazard_kind(k) == d.hazard))
                            .map(|d| i64::from(d.damaged_pm))
                            .max()
                            .unwrap_or(0);
                        let exposure = hoods
                            .iter()
                            .filter_map(|h| {
                                let e = env.exposure(NeighborhoodId(*h as u32))?;
                                hazard.and_then(|k| e.of(hazard_kind(k))).map(i64::from)
                            })
                            .max()
                            .unwrap_or(0);
                        (-worst, -(away(r, a).0 as i64), -exposure, a)
                    }
                    AreaPick::LeastPleased => ((approval(r, a) * 1e6) as i64, 0, 0, a),
                })
                .collect();
            areas.sort();
            areas
                .into_iter()
                .map(|x| ArcActor::Area { area: x.3 })
                .collect()
        }
        ArcCast::Place { kinds, near } => {
            let near = area_of(near);
            let mut places: Vec<(bool, usize, u32)> = world
                .places
                .iter()
                .filter(|p| p.is_open() && town_in(world, at, p.municipality.0))
                .filter_map(|p| {
                    let order = kinds.iter().position(|k| k == p.kind.key())?;
                    let far = near
                        .as_ref()
                        .is_some_and(|hs| !hs.contains(&p.neighborhood.index()));
                    Some((far, order, p.id.0))
                })
                .collect();
            places.sort();
            places
                .into_iter()
                .map(|p| ArcActor::Place {
                    id: crate::world::PlaceId(p.2),
                })
                .collect()
        }
        ArcCast::Jurisdiction { pick } => match (pick, at) {
            (JurisdictionPick::Parent, Jurisdiction::Municipality { id }) => world
                .municipalities
                .get(id.index())
                .map(|m| ArcActor::Jurisdiction {
                    at: Jurisdiction::Region { id: m.region },
                })
                .into_iter()
                .collect(),
            (JurisdictionPick::Parent, Jurisdiction::Region { .. }) => {
                vec![ArcActor::Jurisdiction {
                    at: Jurisdiction::Nation,
                }]
            }
            (JurisdictionPick::Parent, Jurisdiction::Nation) => Vec::new(),
        },
        // A named plant in one of the states asked for: the furthest on in
        // its review first, then the most jobs, then the lowest id.
        ArcCast::Plant { status } => {
            use mandate_content::condition::PlantState as S;
            let econ = r.economy();
            let mut plants: Vec<(u8, i64, u32, crate::world::PlaceId)> = econ
                .plants
                .iter()
                .filter(|p| {
                    world
                        .places
                        .get(p.place.index())
                        .is_some_and(|pl| at.contains_hood(world, pl.neighborhood))
                })
                .filter_map(|p| {
                    let s = crate::condition::plant_state(&p.status);
                    let fits = if status.is_empty() {
                        s != S::Closed
                    } else {
                        status.contains(&s)
                    };
                    if !fits {
                        return None;
                    }
                    let rank = match s {
                        S::Threatened => 0,
                        S::ShortTime => 1,
                        S::Running => 2,
                        S::Closed => 3,
                    };
                    Some((rank, -(p.capacity.round() as i64), p.id.0, p.place))
                })
                .collect();
            plants.sort();
            plants
                .into_iter()
                .map(|p| ArcActor::Place { id: p.3 })
                .collect()
        }
        // Not yet (`mandate_content::arcs::not_yet`): never raised.
        ArcCast::Subject => Vec::new(),
    };
    out.retain(|a| !taken(a));
    out
}

/// The holders of posts at a jurisdiction whose constitution lists a
/// power: offices first, then seats, in the constitution's order.
fn post_holders(v: &View<'_>, at: Jurisdiction, power: Power) -> Vec<CharacterId> {
    let mut out = Vec::new();
    for id in power_holders(v, at, power) {
        if let Some(ch) = v.pol.get(id).map(|p| p.character)
            && !out.contains(&ch)
        {
            out.push(ch);
        }
    }
    out
}

/// The politicians who hold a post for a jurisdiction that lists a power
/// (the player included): offices first, then seats, each once.
pub fn power_holders(
    v: &View<'_>,
    at: Jurisdiction,
    power: Power,
) -> Vec<crate::politics::PoliticianId> {
    let pol = v.pol;
    let Some(c) = v.content().constitutions.get(&v.r.meta.constitution) else {
        return Vec::new();
    };
    let holders = pol
        .offices
        .iter()
        .filter(|o| o.at == at)
        .filter_map(|o| o.holder)
        .chain(crate::politics::officials_at(pol, at));
    let mut out = Vec::new();
    for id in holders {
        if crate::politics::holds_power_at(pol, c, id, power, at) && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Bind a template's roles at a jurisdiction, in order (a later role may
/// name an earlier one). `None` when a required role cannot be filled.
pub fn cast(v: &View<'_>, at: Jurisdiction, def: &ArcDef) -> Option<Vec<ArcCastEntry>> {
    let mut out: Vec<ArcCastEntry> = Vec::new();
    for role in &def.roles {
        match candidates(v, at, role, &out).into_iter().next() {
            Some(actor) => out.push(ArcCastEntry {
                role: role.id.clone(),
                actor,
            }),
            None if role.optional => {}
            None => return None,
        }
    }
    Some(out)
}

/// The readings a run takes as it is raised: for each area role, its
/// unrepaired damage (`<role>.damage`) and homes damaged (`<role>.homes`).
pub fn base_readings(r: &Reads<'_>, cast: &[ArcCastEntry]) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    for c in cast {
        if let ArcActor::Area { area } = c.actor {
            out.push((format!("{}.damage", c.role), damage(r, area, None)));
            out.push((format!("{}.homes", c.role), damaged_homes(r, area)));
        }
    }
    out
}

/// A jurisdiction's name (the nation has none: the capital's region is
/// not it; an empty string).
pub fn jurisdiction_name(world: &World, at: Jurisdiction) -> String {
    match at {
        Jurisdiction::Municipality { id } => world
            .municipalities
            .get(id.index())
            .map(|m| m.name.clone())
            .unwrap_or_default(),
        Jurisdiction::Region { id } => world
            .regions
            .get(id.index())
            .map(|r| r.name.clone())
            .unwrap_or_default(),
        Jurisdiction::Nation => String::new(),
    }
}

/// An area's name.
pub fn area_name(world: &World, area: Area) -> String {
    match area {
        Area::Hood { id } => world
            .neighborhoods
            .get(id.index())
            .map(|h| h.name.clone())
            .unwrap_or_default(),
        Area::Constituency { id } => world
            .constituencies
            .get(id.index())
            .map(|c| c.number.to_string())
            .unwrap_or_default(),
        Area::In { at } => jurisdiction_name(world, at),
    }
}

/// A place's name: its own, else its neighbourhood's.
pub fn place_name(world: &World, id: crate::world::PlaceId) -> String {
    let Some(p) = world.places.get(id.index()) else {
        return String::new();
    };
    if !p.name.is_empty() {
        return p.name.clone();
    }
    world
        .neighborhoods
        .get(p.neighborhood.index())
        .map(|h| h.name.clone())
        .unwrap_or_default()
}

/// The cast's names by role (facts, never prose): a person's full name, an
/// area's, a place's, a jurisdiction's.
pub fn names(
    pol: &crate::politics::PoliticsState,
    world: &World,
    cast: &[ArcCastEntry],
) -> Vec<(String, String)> {
    cast.iter()
        .map(|c| {
            let name = match c.actor {
                ArcActor::Person { id } => characters::profile(pol, world, id)
                    .map(|p| p.full_name())
                    .unwrap_or_default(),
                ArcActor::Area { area } => area_name(world, area),
                ArcActor::Place { id } => place_name(world, id),
                ArcActor::Jurisdiction { at } => jurisdiction_name(world, at),
            };
            (c.role.clone(), name)
        })
        .collect()
}

/// A date's arguments: `<prefix>day` and `<prefix>month_key`.
fn date_args(prefix: &str, d: SimDate, out: &mut Args) {
    let c = d.calendar();
    out.insert(format!("{prefix}day"), c.day.to_string());
    out.insert(format!("{prefix}month_key"), format!("month-{}", c.month));
}

/// The dated facts of a run now (`COMMON_ARGS` less the names): the day
/// the matter began, weeks since, the clock's day.
pub fn dates(run: &ArcRun, today: SimDate) -> Args {
    let mut a = Args::new();
    date_args("", run.began, &mut a);
    a.insert("year".into(), run.began.calendar().year.to_string());
    let days = if today >= run.began {
        run.began.days_until(today)
    } else {
        0
    };
    a.insert("weeks".into(), (days / 7).to_string());
    if let Some(c) = run.clock {
        date_args("clock_", c, &mut a);
    }
    a
}

/// The facts recorded with a run's event: its dates and the readings of
/// its template's `facts` (conditions evaluated for their arguments), as
/// they are now. Names are resolved when the event is read.
pub fn facts(v: &View<'_>, def: &ArcDef, run: &ArcRun) -> Vec<(String, String)> {
    let mut a = dates(run, v.date());
    let people = run.narrative_cast();
    let cx = super::step::context(v, run, &people);
    for f in &def.facts {
        let mut x = Args::new();
        if crate::condition::holds(&cx, f, &mut x) {
            a.extend(x);
        }
    }
    a.into_iter().collect()
}
