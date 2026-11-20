//! The `visit` action (places with purpose, DEC-079 note; task PL1): one
//! generic, data-driven action whose verbs are content (`actions.jsonc`
//! `visit.verbs`, listed per place kind in `places.jsonc` `verbs`). A verb
//! costs time, can be done when its conditions on live state hold, teaches
//! (DEC-091 ledger grants), reaches residents a little, meets the people
//! there, and may carry a risk rolled from live state (DEC-073: the odds
//! shown are the odds rolled).
//!
//! Tier-generic (DEC-098): a verb is about the place it is done at. Its
//! town is the place's municipality, its people are whoever is there by
//! their routines (`characters::routines::present`, posts included), and no
//! office or body is named. A regional assembly or parliament is a place
//! like any other.
//!
//! The player's own command writes the player's state (time, the ledger)
//! and the opinions and memories the meeting leaves, as every action does;
//! the roll is a keyed draw (turn, place, verb), so it advances no stream.

use crate::actions::ActionResult;
use crate::characters::{self, CharacterId, MemoryKind};
use crate::commands::EngineError;
use crate::places::Location;
use crate::player::knowledge::{self, Grant, Level, Subject};
use crate::state::State;
use crate::world::{MunicipalityId, Place, PlaceId};
use mandate_content::routines::DayPart;
use mandate_content::visits::{
    VisitAmong, VisitEffect, VisitReach, VisitTime, VisitVerbDef,
};
use mandate_content::{ContentBundle, PlaceKindDef};
use mandate_core::Stream;
use mandate_core::math::clamp;

/// Keyed-RNG tag of a visit's risk roll (DEC-041): never reused.
const KEY_VISIT: u64 = 0x7151_7E00;

fn bad(msg: &str) -> EngineError {
    EngineError::InvalidCommand(msg.into())
}

/// Why a verb cannot be done now, as a Fluent key (`visit-blocked-…`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocked {
    /// Not at this time of day.
    Hours,
    /// Not late at night.
    NotLate,
    /// The town's situation does not call for it.
    Situation,
    /// Nobody it needs is here now.
    Nobody,
    /// No time left this turn (or the late slot is used).
    Time,
}

impl Blocked {
    pub fn key(self) -> &'static str {
        match self {
            Blocked::Hours => "visit-blocked-hours",
            Blocked::NotLate => "visit-blocked-late",
            Blocked::Situation => "visit-blocked-situation",
            Blocked::Nobody => "visit-blocked-nobody",
            Blocked::Time => "visit-blocked-time",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Blocked::Hours => "that is not done at this time of day",
            Blocked::NotLate => "that is not done late at night",
            Blocked::Situation => "the town's situation does not call for that now",
            Blocked::Nobody => "nobody you would need is here now",
            Blocked::Time => "not enough time left this turn",
        }
    }
}

/// The generated place at a location (verbs are only offered at places).
pub fn place_at(state: &State, loc: Location) -> Option<&Place> {
    match loc {
        Location::Place { place } => state.world.places.get(place.index()),
        _ => None,
    }
}

fn kind_of<'a>(content: &'a ContentBundle, p: &Place) -> Option<&'a PlaceKindDef> {
    content.place_kind(p.kind.key())
}

/// The verbs offered at a location (the place kind's list, registry plus
/// fallbacks; a closed place offers none), in the kind's order.
pub fn verbs_at<'a>(
    state: &State,
    content: &'a ContentBundle,
    loc: Location,
) -> Vec<&'a VisitVerbDef> {
    let mut ids: Vec<&str> = Vec::new();
    for d in &content.places {
        for v in &d.verbs {
            if !ids.contains(&v.as_str())
                && crate::places::locations_for(
                    state,
                    content,
                    crate::places::Offer::Verb(v),
                    None,
                )
                .contains(&loc)
            {
                ids.push(v);
            }
        }
    }
    // The location's own kind's order first, then any fallback-hosted ones.
    let own: Vec<&str> = place_at(state, loc)
        .and_then(|p| kind_of(content, p))
        .map(|k| k.verbs.iter().map(String::as_str).collect())
        .unwrap_or_default();
    ids.sort_by_key(|id| own.iter().position(|x| x == id).unwrap_or(usize::MAX));
    ids.into_iter()
        .filter_map(|id| content.actions.visit.verb(id))
        .collect()
}

/// The part of the day a verb would be done in (`late_night`: the late slot).
fn part_now(state: &State, late_night: bool) -> DayPart {
    if late_night {
        DayPart::LateNight
    } else {
        characters::routines::day_part(state)
    }
}

/// Who is at a location now (their routines), not the player.
fn present(state: &State, content: &ContentBundle, loc: Location) -> Vec<CharacterId> {
    let now = characters::routines::Now::of(state, content);
    let me = characters::player_character(state);
    characters::routines::present(state, content, &now, loc)
        .into_iter()
        .filter(|c| Some(*c) != me)
        .collect()
}

fn occupation_of(state: &State, id: CharacterId) -> Option<&str> {
    state
        .politics
        .characters
        .get(id)
        .and_then(|c| c.named.as_deref())
        .map(|n| n.occupation.as_str())
}

/// Whether the player has the time for a verb now.
fn has_time(state: &State, def: &VisitVerbDef, late_night: bool) -> bool {
    let Some(pl) = state.player.as_ref() else {
        return false;
    };
    if late_night {
        return !pl.late_night_used;
    }
    let half_open = pl.talk.half_slot == Some(state.calendar.turn);
    match def.time {
        VisitTime::Half if half_open => true,
        _ => pl.slots_left() >= 1,
    }
}

/// Whether a verb can be done at a location now (the place offering it is
/// checked by `places::check_at`). `Err` names why not.
pub fn availability(
    state: &State,
    content: &ContentBundle,
    loc: Location,
    def: &VisitVerbDef,
    late_night: bool,
) -> Result<(), Blocked> {
    let Some(p) = place_at(state, loc) else {
        return Err(Blocked::Nobody);
    };
    if late_night && !def.late_night {
        return Err(Blocked::NotLate);
    }
    let part = part_now(state, late_night);
    if !late_night && part == DayPart::LateNight {
        // The standard slots are spent: only the late slot is left.
        return Err(Blocked::Time);
    }
    if !late_night && !def.parts.is_empty() && !def.parts.contains(&part) {
        return Err(Blocked::Hours);
    }
    let m = p.municipality.0;
    if !def
        .when
        .iter()
        .all(|c| crate::policy::situation_holds(state, content, c, m))
    {
        return Err(Blocked::Situation);
    }
    if !def.with.is_empty()
        && !present(state, content, loc)
            .into_iter()
            .any(|c| occupation_of(state, c).is_some_and(|o| def.with.iter().any(|w| w == o)))
    {
        return Err(Blocked::Nobody);
    }
    if !has_time(state, def, late_night) {
        return Err(Blocked::Time);
    }
    Ok(())
}

/// How far a town's industry is working to its capacity at neutral demand
/// (jobs over capacity; 1 when it has none).
pub fn sector_use(state: &State, content: &ContentBundle, town: MunicipalityId, sector: &str) -> f64 {
    let Some(k) = content.sector_index(sector) else {
        return 1.0;
    };
    state
        .economy
        .munis
        .get(town.index())
        .and_then(|m| m.sectors.get(k))
        .filter(|s| s.capacity > 0.0)
        .map(|s| s.jobs / s.capacity)
        .unwrap_or(1.0)
}

/// How fast a town's industry is letting people go (0..1, `visit.outlook`).
pub fn shedding(state: &State, content: &ContentBundle, town: MunicipalityId, sector: &str) -> f64 {
    let o = &content.actions.visit.outlook;
    let u = sector_use(state, content, town, sector);
    clamp((o.shedding - u) / (o.shedding - o.shed_full).max(1e-6), 0.0, 1.0)
}

/// An industry's outlook in words: `growing` | `steady` | `shedding`.
pub fn outlook_band(state: &State, content: &ContentBundle, town: MunicipalityId, sector: &str) -> &'static str {
    let o = &content.actions.visit.outlook;
    let u = sector_use(state, content, town, sector);
    if u >= o.growing {
        "growing"
    } else if u < o.shedding {
        "shedding"
    } else {
        "steady"
    }
}

/// The chance a verb goes wrong at a place now (`None`: it cannot). The
/// command rolls exactly this (DEC-073: odds shown are odds rolled).
pub fn risk(state: &State, content: &ContentBundle, place: PlaceId, def: &VisitVerbDef) -> Option<f64> {
    let r = def.risk.as_ref()?;
    let p = state.world.places.get(place.index())?;
    let m = p.municipality.0;
    let mut x = r.base;
    for t in &r.terms {
        if crate::policy::situation_holds(state, content, &t.when, m) {
            x += t.add;
        }
    }
    if r.shedding > 0.0
        && let Some(s) = kind_of(content, p).and_then(|k| k.sector.as_deref())
    {
        x += r.shedding * shedding(state, content, p.municipality, s);
    }
    Some(clamp(x, r.min, r.max))
}

// --- Readings (DEC-091: numbers learned first-hand are dated readings) ---------------

/// A town industry's reading: jobs, jobs over capacity (per mille), and how
/// fast it sheds (per mille).
pub fn sector_reading(
    state: &State,
    content: &ContentBundle,
    town: MunicipalityId,
    sector: &str,
) -> Vec<(String, i32)> {
    let jobs = content
        .sector_index(sector)
        .and_then(|k| state.economy.munis.get(town.index())?.sectors.get(k))
        .map(|s| s.jobs.round() as i32)
        .unwrap_or(0);
    vec![
        ("jobs".into(), jobs),
        (
            "use".into(),
            (sector_use(state, content, town, sector) * 1000.0).round() as i32,
        ),
        (
            "shedding".into(),
            (shedding(state, content, town, sector) * 1000.0).round() as i32,
        ),
    ]
}

/// A place's site reading: the region's pollution index and its
/// neighbourhood's exposure to floods, wildfire and storms (per mille).
pub fn site_reading(state: &State, place: PlaceId) -> Vec<(String, i32)> {
    let Some(p) = state.world.places.get(place.index()) else {
        return Vec::new();
    };
    let pollution = state
        .world
        .municipalities
        .get(p.municipality.index())
        .and_then(|m| state.environment.regions.get(m.region.index()))
        .map(|r| r.pollution)
        .unwrap_or(0.0);
    let x = state
        .environment
        .exposure
        .get(p.neighborhood.index())
        .copied()
        .unwrap_or_default();
    vec![
        ("pollution".into(), (pollution * 1000.0).round() as i32),
        ("flood".into(), x.flood as i32),
        ("wildfire".into(), x.wildfire as i32),
        ("storm".into(), x.storm as i32),
    ]
}

/// A service's need reading: its neighbourhood's felt level of each concern
/// the place kind addresses (per mille), the quantity F7's need term reads
/// (DEC-092 §2.22).
pub fn service_reading(state: &State, content: &ContentBundle, place: PlaceId) -> Vec<(String, i32)> {
    let Some(p) = state.world.places.get(place.index()) else {
        return Vec::new();
    };
    let Some(k) = kind_of(content, p) else {
        return Vec::new();
    };
    let felt = knowledge::hood_reading(state, content, p.neighborhood);
    k.addresses
        .iter()
        .map(|c| {
            let v = felt.iter().find(|(id, _)| id == c).map(|x| x.1).unwrap_or(0);
            (c.clone(), v)
        })
        .collect()
}

/// The dated reading of a numeric subject, now.
pub fn reading_of(state: &State, content: &ContentBundle, s: &Subject) -> Option<Vec<(String, i32)>> {
    match s {
        Subject::HoodConcerns { hood } => Some(knowledge::hood_reading(state, content, *hood)),
        Subject::Sector { town, sector } => Some(sector_reading(state, content, *town, sector)),
        Subject::Site { place } => Some(site_reading(state, *place)),
        Subject::Service { place } => Some(service_reading(state, content, *place)),
        _ => None,
    }
}

// --- Effects ----------------------------------------------------------------------

/// The cohorts an effect reaches: (neighbourhood index, cohort index).
fn reached(state: &State, content: &ContentBundle, p: &Place, a: &VisitAmong) -> Vec<(usize, usize)> {
    let hoods: Vec<usize> = match a.reach {
        VisitReach::Hood => vec![p.neighborhood.index()],
        VisitReach::Town => state.world.hoods_of(p.municipality).collect(),
    };
    let communities: Vec<usize> = a
        .communities
        .iter()
        .filter_map(|c| content.community_index(c))
        .collect();
    let mut out = Vec::new();
    for h in hoods {
        let Some(hp) = state.population.hoods.get(h) else {
            continue;
        };
        for ci in 0..hp.cohorts.len() {
            let class = crate::population::class_of(ci);
            let age = crate::population::age_of(ci);
            let community = crate::population::community_of(ci);
            if (a.classes.is_empty() || a.classes.iter().any(|k| *k as usize == class))
                && (a.ages.is_empty() || a.ages.iter().any(|x| *x as usize == age))
                && (a.communities.is_empty() || communities.contains(&community))
            {
                out.push((h, ci));
            }
        }
    }
    out
}

/// What a visit did, for the outcome's arguments.
#[derive(Default)]
struct Done {
    learned: usize,
    met: Vec<CharacterId>,
}

fn apply_effects(
    state: &mut State,
    content: &ContentBundle,
    place: PlaceId,
    verb: &str,
    effects: &[VisitEffect],
    done: &mut Done,
) {
    let Some(p) = state.world.places.get(place.index()).cloned() else {
        return;
    };
    let loc = Location::Place { place };
    let src = knowledge::Source::Action {
        action: action_word(verb),
        place: Some(place),
    };
    let date = state.calendar.date;
    for e in effects {
        match e {
            VisitEffect::Learn { grants } => {
                let Some(mut scope) = knowledge::Scope::of_player(state) else {
                    continue;
                };
                scope.town = p.municipality;
                scope.here = Some(place);
                for g in knowledge::resolve(state, content, grants, &scope) {
                    done.learned += learn(state, content, g, &src) as usize;
                }
            }
            VisitEffect::People {
                occupations,
                level,
                opinion,
            } => {
                let me = characters::player_character(state);
                let people: Vec<CharacterId> = present(state, content, loc)
                    .into_iter()
                    .filter(|c| {
                        occupations.is_empty()
                            || occupation_of(state, *c)
                                .is_some_and(|o| occupations.iter().any(|x| x == o))
                    })
                    .collect();
                for person in people {
                    let changed = knowledge::teach_one(
                        state,
                        Subject::Person { person },
                        *level,
                        knowledge::Source::Met { person },
                    );
                    done.learned += changed as usize;
                    if let Some(me) = me
                        && *level >= Level::Known
                    {
                        let seed = state.meta.seed;
                        characters::touch(&mut state.politics, content, seed, person, date);
                        characters::remember(
                            &mut state.politics,
                            content,
                            date,
                            person,
                            me,
                            MemoryKind::Met,
                            None,
                            (*opinion != 0.0).then_some(*opinion),
                        );
                    }
                    if !done.met.contains(&person) {
                        done.met.push(person);
                    }
                }
            }
            VisitEffect::Awareness { amount, among } => {
                for (h, ci) in reached(state, content, &p, among) {
                    let c = &mut state.population.hoods[h].cohorts[ci];
                    let aw = c.player_awareness as f64;
                    c.player_awareness = clamp(aw + amount * (1.0 - aw), 0.0, 1.0) as f32;
                }
            }
            VisitEffect::Approval { amount, among } => {
                for (h, ci) in reached(state, content, &p, among) {
                    let c = &mut state.population.hoods[h].cohorts[ci];
                    let room = 1.0 - (c.player_approval as f64).abs();
                    c.player_approval =
                        clamp(c.player_approval as f64 + amount * room, -1.0, 1.0) as f32;
                }
            }
        }
    }
}

/// Teach one grant. A numeric subject granted `detailed` is a dated
/// reading (its identity first, as the ledger requires). Returns whether
/// the ledger changed.
fn learn(state: &mut State, content: &ContentBundle, g: Grant, src: &knowledge::Source) -> bool {
    if g.level == Level::Detailed
        && let Some(values) = reading_of(state, content, &g.subject)
    {
        let known = knowledge::teach_one(state, g.subject.clone(), Level::Known, src.clone());
        return knowledge::read_now(state, g.subject, values, src.clone()) || known;
    }
    knowledge::teach(state, [g], src) > 0
}

/// Pay a verb's time and health (checked first by `availability`). Half a
/// slot shares the conversation's half-slot bank (two halves fill one).
fn pay(state: &mut State, content: &ContentBundle, def: &VisitVerbDef, late_night: bool) {
    let turn = state.calendar.turn;
    let late = content.actions.late_night_health;
    let Some(pl) = state.player.as_mut() else {
        return;
    };
    if late_night {
        pl.late_night_used = true;
        pl.tracks.health = clamp(pl.tracks.health as f64 - late, 0.0, 100.0) as f32;
    } else {
        match def.time {
            VisitTime::Half if pl.talk.half_slot == Some(turn) => pl.talk.half_slot = None,
            VisitTime::Half => {
                pl.slots_used += 1;
                pl.talk.half_slot = Some(turn);
            }
            VisitTime::Slot => pl.slots_used += 1,
        }
    }
    pl.tracks.health = clamp(pl.tracks.health as f64 - def.health, 0.0, 100.0) as f32;
}

/// Do a verb where the player stands (the `Visit` arm of `actions::apply`;
/// `places::check_at` has checked the place offers it).
pub fn visit(
    state: &mut State,
    content: &ContentBundle,
    verb: &str,
    late_night: bool,
) -> Result<ActionResult, EngineError> {
    let def = content
        .actions
        .visit
        .verb(verb)
        .ok_or_else(|| bad("no such thing to do"))?;
    let loc = state
        .player
        .as_ref()
        .map(|p| p.location)
        .ok_or_else(|| bad("no player"))?;
    let place = place_at(state, loc)
        .filter(|p| p.is_open())
        .map(|p| p.id)
        .ok_or_else(|| bad("that is done at a place"))?;
    availability(state, content, loc, def, late_night).map_err(|b| bad(b.message()))?;
    // Decided before anything changes: the odds shown.
    let odds = risk(state, content, place, def);
    pay(state, content, def, late_night);
    let hit = odds.is_some_and(|p| {
        state
            .rng
            .keyed(
                Stream::Player,
                &[
                    KEY_VISIT,
                    state.calendar.turn as u64,
                    place.0 as u64,
                    mandate_core::hash::derive_u64(verb, &[]),
                ],
            )
            .chance(p)
    });
    let mut done = Done::default();
    apply_effects(state, content, place, verb, &def.effects, &mut done);
    let mut res = ActionResult {
        outcome_key: def.outcome_key.clone(),
        ..Default::default()
    };
    if hit && let Some(r) = &def.risk {
        apply_effects(state, content, place, verb, &r.effects, &mut done);
        res.outcome_key = r.outcome_key.clone();
    }
    // The people met are noted where they were seen (S1).
    knowledge::note_sightings(state, content);
    if let Some(p) = state.world.places.get(place.index()) {
        let name = if p.name.is_empty() {
            state
                .world
                .neighborhoods
                .get(p.neighborhood.index())
                .map(|h| h.name.clone())
                .unwrap_or_default()
        } else {
            p.name.clone()
        };
        res.args.insert("place".into(), name);
        res.args.insert("kind".into(), p.kind.key().into());
        if let Some(s) = kind_of(content, p).and_then(|k| k.sector.clone()) {
            res.args.insert(
                "outlook".into(),
                outlook_band(state, content, p.municipality, &s).into(),
            );
            res.args.insert(
                "sector_key".into(),
                content
                    .sector(&s)
                    .map(|d| d.name_key.clone())
                    .unwrap_or_default(),
            );
        }
    }
    res.args.insert("verb".into(), verb.into());
    res.args.insert("verb_key".into(), def.name_key.clone());
    res.args.insert("learned".into(), done.learned.to_string());
    res.args.insert("met".into(), done.met.len().to_string());
    if let Some(first) = done.met.first()
        && let Some(p) = characters::profile(&state.politics, &state.world, *first)
    {
        res.args.insert("name".into(), p.full_name());
    }
    Ok(res)
}

/// The action word of a verb in the log and the ledger's sources:
/// `verb_<name>` (Fluent `action-verb-<name>`, underscores as dashes).
pub fn action_word(verb: &str) -> String {
    format!("verb_{}", verb.strip_prefix("verb.").unwrap_or(verb))
}
