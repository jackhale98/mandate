//! Daily routines (DEC-031 "Daily routines"; DEC-013 day parts): where each
//! stored character is now. Derived, never stored: a pure function of the
//! state, the content's routines registry (`routines.jsonc`) and the turn's
//! representative day and part.
//!
//! **The day.** A turn stands for a span of days played as one stylised day
//! (DEC-013). Its *representative day* is the span's first day plus the turn
//! number modulo the span, so the weekday moves on by one each turn and
//! weekday rules ("the diner on Thursdays") come round. Its *part* follows
//! the player's standard slots (morning, afternoon, evening); once they are
//! spent only the late-night slot is left, and it is late at night.
//!
//! **Who.** A person's routines are tried in order: the broker's or the
//! rival's (a town's featured characters), their post's (by its role word
//! from the constitution, DEC-098: no office or body is named here), their
//! occupation's, then `politician` (out of office) and `anyone`. The first
//! rule that applies and names somewhere that exists in their town places
//! them; the last resort is their workplace, then their home street.
//!
//! **Variation** comes from keyed draws (`[ROUTINE, person, day, part]`),
//! never a stream, so placement is the same however often it is asked and
//! a routine can be learned. The world does not wait (DEC-091): routines
//! run whatever the player knows; knowledge gates only what is shown.

use super::{CharacterId, key_of, rng};
use crate::places::Location;
use crate::politics::{PoliticsState, Post};
use crate::state::{GamePhase, State};
use crate::world::{Jurisdiction, MunicipalityId, NeighborhoodId, PlaceId, PlaceKind};
use mandate_content::routines::{RoutineDay, RoutineDef, RoutineEvent, RoutineRole, Season};
use mandate_content::{ContentBundle, StageDef};
use mandate_core::SimDate;

pub use mandate_content::routines::DayPart;

/// Keyed-RNG tags (DEC-041), beside `characters::KEY_*`.
pub(crate) const KEY_ROUTINE: u64 = 0xC4A5;
pub(crate) const KEY_HAUNT: u64 = 0xC4A6;
pub(crate) const KEY_MEETING: u64 = 0xC4A7;
pub(crate) const KEY_SITTING: u64 = 0xC4A8;

/// The part of the day the player is in: each standard slot is a part
/// (scaled to the slots they have; one half spent on a conversation still
/// counts as under way), and with none left it is late at night.
/// Without a player (lab runs), the morning.
pub fn day_part(state: &State) -> DayPart {
    let Some(pl) = state.player.as_ref() else {
        return DayPart::Morning;
    };
    let cap = pl.slot_cap().max(1) as u32;
    // A slot half spent on a conversation is still under way (the next
    // conversation finishes it), so the day moves on when it is spent.
    let open = pl.talk.half_slot == Some(state.calendar.turn) && pl.slots_used > 0;
    let used = (pl.slots_used as u32 - open as u32).min(cap);
    if used >= cap {
        return DayPart::LateNight;
    }
    match used * 3 / cap {
        0 => DayPart::Morning,
        1 => DayPart::Afternoon,
        _ => DayPart::Evening,
    }
}

/// Days the current turn spans (as `Engine::turn_days`).
pub fn turn_days(state: &State, content: &ContentBundle) -> u32 {
    let t = &content.tuning.turn_days;
    let days: u32 = match state.calendar.phase {
        GamePhase::Campaign => t.campaign.into(),
        GamePhase::Governing | GamePhase::BurnIn => t.governing_local.into(),
    };
    days.max(1)
}

/// The turn's representative day: its first day plus the turn number modulo
/// the span (the weekday moves on each turn).
pub fn representative_day(state: &State, content: &ContentBundle) -> SimDate {
    let span = turn_days(state, content);
    state.calendar.date.add_days(state.calendar.turn % span)
}

/// The town's community meeting night (one weekday, Monday to Thursday,
/// keyed by the seed and the town).
pub fn meeting_night(seed: u64, town: MunicipalityId) -> RoutineDay {
    match rng(seed, &[KEY_MEETING, town.0 as u64]).below(4) {
        0 => RoutineDay::Mon,
        1 => RoutineDay::Tue,
        2 => RoutineDay::Wed,
        _ => RoutineDay::Thu,
    }
}

/// A jurisdiction's regular sitting day: its legislature sits that weekday
/// (Monday to Thursday, keyed by the seed and the jurisdiction) whether or
/// not a bill is due, and on any day a law-making stage falls.
pub fn sitting_day(seed: u64, at: Jurisdiction) -> RoutineDay {
    match rng(seed, &[KEY_SITTING, at.tier_index(), at.raw() as u64]).below(4) {
        0 => RoutineDay::Mon,
        1 => RoutineDay::Tue,
        2 => RoutineDay::Wed,
        _ => RoutineDay::Thu,
    }
}

/// When routines are asked about: the turn's day and part, and what is going
/// on (computed once per question).
#[derive(Clone, Debug, PartialEq)]
pub struct Now {
    /// The turn's first day.
    pub date: SimDate,
    /// The representative day (weekday, season, keyed draws).
    pub day: SimDate,
    pub span: u32,
    pub part: DayPart,
    pub weekday: RoutineDay,
    pub season: Season,
    /// Jurisdictions with a law-making stage (any) in this turn (see
    /// [`Now::session`] for the regular sitting).
    pub sessions: Vec<Jurisdiction>,
    /// Jurisdictions with a vote stage in this turn.
    pub votes: Vec<Jurisdiction>,
    /// Jurisdictions with a race running.
    pub campaigns: Vec<Jurisdiction>,
}

impl Now {
    /// Now: this turn, at the player's part of the day.
    pub fn of(state: &State, content: &ContentBundle) -> Now {
        Now::at(state, content, day_part(state))
    }

    /// This turn, at a given part of the day.
    pub fn at(state: &State, content: &ContentBundle, part: DayPart) -> Now {
        let date = state.calendar.date;
        let span = turn_days(state, content);
        let day = representative_day(state, content);
        let mut sessions = Vec::new();
        let mut votes = Vec::new();
        let c = content.constitutions.get(&state.meta.constitution);
        let until = date.add_days(span);
        for p in &state.policy.instances {
            if p.status != crate::policy::PolicyStatus::UnderConsideration
                || p.next_stage_on.is_none_or(|d| d >= until)
            {
                continue;
            }
            let at = p.at();
            let stage = c
                .and_then(|c| c.tier(p.tier))
                .and_then(|t| t.procedure("ordinance"))
                .and_then(|pr| pr.stages.get(p.stage as usize));
            if !sessions.contains(&at) {
                sessions.push(at);
            }
            if matches!(stage, Some(StageDef::Vote { .. })) && !votes.contains(&at) {
                votes.push(at);
            }
        }
        let mut campaigns = Vec::new();
        for r in &state.politics.elections.races {
            let at = r.seat.at();
            if !campaigns.contains(&at) {
                campaigns.push(at);
            }
        }
        Now {
            date,
            day,
            span,
            part,
            weekday: RoutineDay::of(day),
            season: Season::of(day),
            sessions,
            votes,
            campaigns,
        }
    }
}

impl Now {
    /// Whether a jurisdiction's legislature sits today: a law-making stage
    /// falls in this turn, or it is the regular sitting day.
    pub fn session(&self, seed: u64, at: Jurisdiction) -> bool {
        self.sessions.contains(&at) || sitting_day(seed, at) == self.weekday
    }
}

/// Who a stored character is, for their routines.
#[derive(Clone, Debug)]
struct Who<'a> {
    id: CharacterId,
    town: MunicipalityId,
    /// Home neighbourhood.
    home: u32,
    /// The jurisdiction their events are read in: their post's, else their
    /// town's.
    at: Jurisdiction,
    post: Option<Post>,
    word: Option<&'a str>,
    occupation: Option<&'a str>,
    /// Their own place (a Named character's workplace or haunt).
    place: Option<PlaceId>,
    politician: bool,
    broker: bool,
    rival: bool,
}

fn town_of_hood(state: &State, hood: u32) -> Option<MunicipalityId> {
    state
        .world
        .neighborhoods
        .get(hood as usize)
        .map(|h| h.municipality)
}

fn who<'a>(state: &State, content: &'a ContentBundle, id: CharacterId) -> Option<Who<'a>> {
    let pol = &state.politics;
    let c = pol.characters.get(id)?;
    let constitution = content.constitutions.get(&state.meta.constitution);
    let (town, home, post, word, occupation, place, politician) =
        if let Some(p) = c.politician.and_then(|p| pol.get(p)) {
            if p.is_player {
                return None;
            }
            let m = MunicipalityId(p.municipality());
            state.world.municipalities.get(m.index())?;
            let post = p.role.post();
            let word = post.and_then(|q| constitution.and_then(|c| q.word(c)));
            (m, p.home, post, word, None, None, true)
        } else {
            let n = c.named.as_deref()?;
            let m = town_of_hood(state, n.home).or_else(|| {
                n.place
                    .and_then(|p| state.world.places.get(p.index()))
                    .map(|p| p.municipality)
            })?;
            let occ = content.occupation(&n.occupation).map(|o| o.id.as_str());
            (m, n.home, None, None, occ, n.place, false)
        };
    let t = pol.characters.towns.get(town.index());
    Some(Who {
        id,
        town,
        home,
        at: post.map(|p| p.at()).unwrap_or(Jurisdiction::town(town)),
        post,
        word,
        occupation,
        place,
        politician,
        broker: t.is_some_and(|t| t.broker == Some(id)),
        rival: t.is_some_and(|t| t.rival == Some(id)),
    })
}

/// Open places of a kind in a town, in id order.
fn places_of_kind(state: &State, town: MunicipalityId, kind: PlaceKind) -> Vec<PlaceId> {
    let Some(mu) = state.world.municipalities.get(town.index()) else {
        return Vec::new();
    };
    mu.places
        .iter()
        .filter(|p| {
            state
                .world
                .places
                .get(p.index())
                .is_some_and(|x| x.kind == kind && x.is_open())
        })
        .copied()
        .collect()
}

/// Someone's own place of a kind in their town: the one in their
/// neighbourhood, else one kept for them (keyed by person and kind, so it
/// does not change from day to day).
fn haunt(state: &State, w: &Who<'_>, kind: PlaceKind) -> Option<PlaceId> {
    let opts = places_of_kind(state, w.town, kind);
    if let Some(p) = opts.iter().find(|p| {
        state
            .world
            .places
            .get(p.index())
            .is_some_and(|x| x.neighborhood.0 == w.home)
    }) {
        return Some(*p);
    }
    let mut r = rng(
        state.meta.seed,
        &[KEY_HAUNT, w.id.0 as u64, key_of(kind.key())],
    );
    r.pick(&opts).copied()
}

fn home_streets(state: &State, w: &Who<'_>) -> Option<Location> {
    (town_of_hood(state, w.home) == Some(w.town)).then_some(Location::Street {
        neighborhood: NeighborhoodId(w.home),
    })
}

/// Where a spot sends someone, if it exists in their town.
fn resolve(
    state: &State,
    content: &ContentBundle,
    pol: &PoliticsState,
    now: &Now,
    w: &Who<'_>,
    to: &str,
) -> Option<Location> {
    let place = |p: PlaceId| Location::Place { place: p };
    match to {
        "works" => {
            if let Some(post) = w.post {
                let kind = crate::politics::workplace_of(pol, post)?;
                let town = post.at().municipality()?;
                if town != w.town {
                    return None;
                }
                return haunt(state, w, kind).map(place);
            }
            let p = w.place?;
            state
                .world
                .places
                .get(p.index())
                .filter(|x| x.municipality == w.town && x.is_open())
                .map(|_| place(p))
        }
        "home_streets" => home_streets(state, w),
        "ward_streets" => {
            let hoods: Vec<u32> = w
                .post
                .and_then(|p| p.constituency())
                .map(|c| crate::policy::ward_hoods(&state.world, c))
                .unwrap_or_default()
                .into_iter()
                .map(|h| h as u32)
                .filter(|h| town_of_hood(state, *h) == Some(w.town))
                .collect();
            if hoods.is_empty() {
                return home_streets(state, w);
            }
            let mut r = rng(
                state.meta.seed,
                &[
                    KEY_ROUTINE,
                    w.id.0 as u64,
                    now.day.0 as u64,
                    now.part.index(),
                    2,
                ],
            );
            r.pick(&hoods).map(|h| Location::Street {
                neighborhood: NeighborhoodId(*h),
            })
        }
        "legislature" => {
            let c = content.constitutions.get(&state.meta.constitution)?;
            let body = crate::politics::legislature(c, w.at.tier())?;
            let kind = PlaceKind::from_key(&body.works_at)?;
            haunt(state, w, kind).map(place)
        }
        kind => haunt(state, w, PlaceKind::from_key(kind)?).map(place),
    }
}

fn holds(now: &Now, w: &Who<'_>, e: RoutineEvent, seed: u64) -> bool {
    match e {
        RoutineEvent::Session => now.session(seed, w.at),
        RoutineEvent::Vote => now.votes.contains(&w.at),
        RoutineEvent::Campaign => now.campaigns.contains(&w.at),
        RoutineEvent::MeetingNight => meeting_night(seed, w.town) == now.weekday,
    }
}

/// The first rule of a routine that places someone now.
fn place_by(
    state: &State,
    content: &ContentBundle,
    now: &Now,
    w: &Who<'_>,
    r: &RoutineDef,
) -> Option<Location> {
    let pol = &state.politics;
    let seed = state.meta.seed;
    let rid = key_of(&r.id);
    for (i, rule) in r.rules.iter().enumerate() {
        if (!rule.parts.is_empty() && !rule.parts.contains(&now.part))
            || (!rule.days.is_empty() && !rule.days.contains(&now.weekday))
            || (!rule.seasons.is_empty() && !rule.seasons.contains(&now.season))
            || !rule.events.iter().all(|e| holds(now, w, *e, seed))
            || rule.unless.iter().any(|e| holds(now, w, *e, seed))
        {
            continue;
        }
        let key = [
            KEY_ROUTINE,
            w.id.0 as u64,
            now.day.0 as u64,
            now.part.index(),
            rid,
            i as u64,
        ];
        let mut g = rng(seed, &key);
        // Two draws, always: the chance, then the pick.
        let hit = g.chance(rule.chance);
        let pick = g.next_f64();
        if !hit {
            continue;
        }
        let spots: Vec<(Location, f64)> = rule
            .at
            .iter()
            .filter_map(|s| resolve(state, content, pol, now, w, &s.to).map(|l| (l, s.weight)))
            .collect();
        let total: f64 = spots.iter().map(|s| s.1).sum();
        if spots.is_empty() || total <= 0.0 {
            continue;
        }
        let mut x = pick * total;
        for (l, wt) in &spots {
            if x < *wt {
                return Some(*l);
            }
            x -= wt;
        }
        return spots.last().map(|s| s.0);
    }
    None
}

/// Where a stored character is at `now`: a place or a neighbourhood's
/// streets in their own town. `None` for the player, and for anyone with
/// no town (a higher-tier post's holder until M7).
pub fn locate(
    state: &State,
    content: &ContentBundle,
    now: &Now,
    id: CharacterId,
) -> Option<Location> {
    let w = who(state, content, id)?;
    // A passer-by the player met stays where they were met while that
    // crowd lasts (the Ambient period), then follows their routine.
    if let Some((key, per, _)) = state
        .politics
        .characters
        .get(id)
        .and_then(|c| c.named.as_deref())
        .and_then(|n| n.origin)
        && per == super::ambient::period(content, state.calendar.date)
    {
        let met = if key >> 32 == 1 {
            Location::Street {
                neighborhood: NeighborhoodId(key as u32),
            }
        } else {
            Location::Place {
                place: PlaceId(key as u32),
            }
        };
        let open = match met {
            Location::Place { place } => state
                .world
                .places
                .get(place.index())
                .is_some_and(|p| p.is_open()),
            _ => true,
        };
        if open && town_of(state, met) == Some(w.town) {
            return Some(met);
        }
    }
    let mut order: Vec<&RoutineDef> = Vec::new();
    if w.broker {
        order.extend(content.routines_for_role(RoutineRole::Broker));
    }
    if w.rival {
        order.extend(content.routines_for_role(RoutineRole::Rival));
    }
    if let Some(word) = w.word {
        order.extend(content.routines_for_post(word));
    }
    if let Some(o) = w.occupation {
        order.extend(content.routines_for_occupation(o));
    }
    if w.politician && w.post.is_none() {
        order.extend(content.routines_for_role(RoutineRole::Politician));
    }
    order.extend(content.routines_for_role(RoutineRole::Anyone));
    for r in order {
        if let Some(l) = place_by(state, content, now, &w, r) {
            return Some(l);
        }
    }
    resolve(state, content, &state.politics, now, &w, "works").or_else(|| home_streets(state, &w))
}

/// Where someone is right now (this turn, the player's part of the day).
pub fn whereabouts(state: &State, content: &ContentBundle, id: CharacterId) -> Option<Location> {
    locate(state, content, &Now::of(state, content), id)
}

/// The stored characters of a town (not the player), in id order.
pub fn townsfolk(state: &State, town: MunicipalityId) -> Vec<CharacterId> {
    let pol = &state.politics;
    pol.characters
        .people
        .iter()
        .filter(|c| {
            if let Some(p) = c.politician.and_then(|p| pol.get(p)) {
                return !p.is_player && p.municipality() == town.0;
            }
            c.named.as_deref().is_some_and(|n| {
                town_of_hood(state, n.home)
                    .or_else(|| {
                        n.place
                            .and_then(|p| state.world.places.get(p.index()))
                            .map(|p| p.municipality)
                    })
                    .is_some_and(|m| m == town)
            })
        })
        .map(|c| c.id)
        .collect()
}

/// The town a location is in.
pub fn town_of(state: &State, loc: Location) -> Option<MunicipalityId> {
    match loc {
        Location::Home => None,
        Location::Place { place } => state
            .world
            .places
            .get(place.index())
            .map(|p| p.municipality),
        Location::Street { neighborhood } => town_of_hood(state, neighborhood.0),
    }
}

/// Everyone in a town and where they are at `now`, in id order.
pub fn placements(
    state: &State,
    content: &ContentBundle,
    now: &Now,
    town: MunicipalityId,
) -> Vec<(CharacterId, Location)> {
    townsfolk(state, town)
        .into_iter()
        .filter_map(|id| locate(state, content, now, id).map(|l| (id, l)))
        .collect()
}

/// Who is at a location at `now`: officials whose post works at the place
/// first (body members, then office holders), then the people whose own
/// place it is, then everyone else, each in id order.
pub fn present(
    state: &State,
    content: &ContentBundle,
    now: &Now,
    loc: Location,
) -> Vec<CharacterId> {
    let Some(town) = town_of(state, loc) else {
        return Vec::new();
    };
    let pol = &state.politics;
    let place = match loc {
        Location::Place { place } => state.world.places.get(place.index()),
        _ => None,
    };
    let officials: Vec<CharacterId> = place
        .map(|p| {
            crate::politics::working_at(pol, Jurisdiction::town(town), p.kind)
                .into_iter()
                .filter_map(|id| pol.get(id))
                .map(|x| x.character)
                .collect()
        })
        .unwrap_or_default();
    let mut here: Vec<(u8, usize, CharacterId)> = placements(state, content, now, town)
        .into_iter()
        .filter(|(_, l)| *l == loc)
        .map(|(id, _)| {
            if let Some(i) = officials.iter().position(|x| *x == id) {
                return (0, i, id);
            }
            let own = place.is_some_and(|p| {
                pol.characters
                    .get(id)
                    .and_then(|c| c.named.as_deref())
                    .is_some_and(|n| n.place == Some(p.id))
            });
            (if own { 1 } else { 2 }, 0, id)
        })
        .collect();
    here.sort();
    here.into_iter().map(|x| x.2).collect()
}
