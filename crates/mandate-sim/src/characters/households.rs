//! Doors and households (DEC-114 §2, `BEING_THERE.md` §3).
//!
//! **Doors are slots.** The engine knows neighbourhoods, not plots: each has
//! a number of door slots from its residents (`households.residents_per_door`
//! within `doors`). The street maps slot *k* to the *k*-th house of the
//! plan's fill rank, so door 7 in a neighbourhood is the same house every
//! year; in quick mode the same slots are rows.
//!
//! **Households are generated on demand,** never stored, as Ambient people
//! are: a pure function of (seed, neighbourhood, door, generation) and the
//! live state. Each part is its own keyed draw, so a family keeps its name
//! and its members whatever else changes: who lives there from the
//! neighbourhood's cohorts, owner or tenant by class, where the earner works
//! from the town's plants and sectors by jobs, what worries them from the
//! neighbourhood's felt worries weighted by their cohort's lean, what they
//! think of the player from the cohort's approval and awareness with a keyed
//! spread. An empty home (the town's vacancy) is a door with nobody home.
//! `generation` is 0 until households move (M7, §8).
//!
//! **What is remembered** is only what a doorstep changed (§3.4): a
//! [`DoorstepRecord`] per household met, on the player's character (the
//! characters domain, so DEC-103's return carries it), bounded per
//! neighbourhood and forgotten after a year. A household that brings an ask
//! has its speaker promoted to a Named resident, by the path a `Meet` uses
//! (`NamedCore.origin`), and the record names them.
//!
//! **A canvass is a street** ([`Street`]): the doors knocked since its first
//! knock, and the worries argued and heard at them. The player's actions
//! open and finish it (`actions::ground`).

use super::{CharacterId, named_character, rng};
use crate::state::State;
use crate::world::NeighborhoodId;
use mandate_content::{ContentBundle, ContentKey, HouseholdParams};
use mandate_core::SimDate;
use mandate_core::math::clamp;
use serde::{Deserialize, Serialize};

/// Keyed-RNG tags (DEC-041): one per kind of draw, never reused.
pub(crate) const KEY_HOUSEHOLD: u64 = 0xC4B1;
pub(crate) const KEY_DOOR_DAY: u64 = 0xC4B2;
pub(crate) const KEY_DOOR_ASK: u64 = 0xC4B3;
pub(crate) const KEY_DOOR_PICK: u64 = 0xC4B4;
pub(crate) const KEY_SPEAKER: u64 = 0xC4B5;

/// The part of a household a draw is for (each its own keyed draw).
#[derive(Clone, Copy)]
#[repr(u64)]
enum Part {
    Name = 1,
    Cohort = 2,
    Age = 3,
    Size = 4,
    Tenure = 5,
    Work = 6,
    Worry = 7,
    View = 8,
    Empty = 9,
}

/// The origin key of a household's member promoted to a Named resident
/// (`NamedCore.origin`): apart from every location key (`ambient`).
pub fn origin_key(hood: NeighborhoodId) -> u64 {
    (2u64 << 32) | hood.0 as u64
}

/// Where a household's earner works.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Work {
    /// At a named plant of the town (`economy::plants`).
    Plant { plant: u32 },
    /// In a sector (its content id).
    Sector { sector: ContentKey },
    Retired,
    /// Between jobs (the town's unemployment for their class).
    Looking,
}

/// A household behind a door, generated from live state (never stored).
#[derive(Clone, Debug, PartialEq)]
pub struct Household {
    pub hood: NeighborhoodId,
    pub door: u16,
    pub generation: u8,
    /// Nobody lives here (the town's vacancy).
    pub vacant: bool,
    /// Who answers the door.
    pub given: String,
    pub family: String,
    pub age: u32,
    /// Their cohort (`HoodPop::cohorts`), and its class and community.
    pub cohort: usize,
    pub class: usize,
    pub community: usize,
    pub adults: u8,
    pub children: u8,
    pub owner: bool,
    pub work: Work,
    /// What worries them most (a concern's registry index).
    pub worry: Option<usize>,
    /// Whether they know who the player is, and what they think (−1..1).
    pub aware: bool,
    pub view: f64,
    /// The member promoted to a stored character, once one brought an ask.
    pub speaker: Option<CharacterId>,
}

impl Household {
    /// Their view in a word: `stranger` (they do not know the player),
    /// `warm`, `cool` or `neutral`.
    pub fn mood(&self, content: &ContentBundle) -> &'static str {
        let p = &content.characters.households;
        if !self.aware {
            "stranger"
        } else if self.view >= p.warm {
            "warm"
        } else if self.view <= p.cool {
            "cool"
        } else {
            "neutral"
        }
    }
}

/// How a doorstep went (§3.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Went {
    Warm,
    Cool,
    NotIn,
}

impl Went {
    pub fn key(self) -> &'static str {
        match self {
            Went::Warm => "warm",
            Went::Cool => "cool",
            Went::NotIn => "not_in",
        }
    }
}

/// A promise a household made at the door.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoorPromise {
    /// Their vote (asked in a campaign).
    Vote,
    /// A poster in the window (the street shows it).
    Poster,
}

impl DoorPromise {
    pub fn key(self) -> &'static str {
        match self {
            DoorPromise::Vote => "vote",
            DoorPromise::Poster => "poster",
        }
    }
}

/// What the player did at a door, kept on the record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Said {
    Listened,
    MadeTheCase,
    Asked,
    Knocked,
}

/// A household met at its door (§3.4): what the doorstep changed, and
/// nothing the household's generation gives again.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DoorstepRecord {
    /// The player's character (whoever made the call).
    pub by: CharacterId,
    pub hood: NeighborhoodId,
    pub door: u16,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub generation: u8,
    pub on: SimDate,
    pub went: Went,
    pub said: Said,
    /// The worry heard (listening) or argued (making the case).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worry: Option<ContentKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promise: Option<DoorPromise>,
    /// The member promoted to a Named resident (they brought an ask).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker: Option<CharacterId>,
}

fn is_zero(n: &u8) -> bool {
    *n == 0
}

/// A canvass under way (§3.3): its neighbourhood, the day, the doors
/// knocked since its first knock (which spent its time), and the worries
/// argued and heard at them, in order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Street {
    pub hood: NeighborhoodId,
    pub on: SimDate,
    pub doors: Vec<u16>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argued: Vec<ContentKey>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub heard: Vec<ContentKey>,
}

impl Street {
    /// The worry argued at most doors (ties: the first argued), or none.
    pub fn issue(&self) -> Option<ContentKey> {
        let mut best: Option<(&ContentKey, usize)> = None;
        for w in &self.argued {
            let n = self.argued.iter().filter(|x| *x == w).count();
            if best.is_none_or(|(_, m)| n > m) {
                best = Some((w, n));
            }
        }
        best.map(|(w, _)| w.clone())
    }
}

/// The characters domain's doors: the doorstep records and the canvass
/// under way. Empty in a world nobody has canvassed.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Households {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub records: Vec<DoorstepRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub street: Option<Street>,
}

impl Households {
    pub fn is_empty(&self) -> bool {
        self.records.is_empty() && self.street.is_none()
    }

    /// The record of a household (by, neighbourhood, door, generation).
    pub fn record(
        &self,
        by: CharacterId,
        hood: NeighborhoodId,
        door: u16,
        generation: u8,
    ) -> Option<&DoorstepRecord> {
        self.records
            .iter()
            .find(|r| r.by == by && r.hood == hood && r.door == door && r.generation == generation)
    }

    /// Keep a doorstep (one record a household: the latest, keeping its
    /// speaker and an earlier promise), then forget
    /// what is too old and the oldest beyond the neighbourhood's bound.
    pub fn remember(&mut self, p: &HouseholdParams, rec: DoorstepRecord) {
        // The speaker met, and a promise not taken back, stay with the
        // household.
        let (speaker, promise) = self
            .record(rec.by, rec.hood, rec.door, rec.generation)
            .map_or((None, None), |r| (r.speaker, r.promise));
        self.records.retain(|r| {
            !(r.by == rec.by
                && r.hood == rec.hood
                && r.door == rec.door
                && r.generation == rec.generation)
        });
        let on = rec.on;
        let hood = rec.hood;
        let mut rec = rec;
        rec.speaker = rec.speaker.or(speaker);
        rec.promise = rec.promise.or(promise);
        self.records.push(rec);
        self.forget(on, p.forget_days);
        let keep = p.records_per_hood.max(1) as usize;
        while self.records.iter().filter(|r| r.hood == hood).count() > keep {
            // The oldest call in the neighbourhood (ties: the first kept).
            let Some(i) = self
                .records
                .iter()
                .enumerate()
                .filter(|(_, r)| r.hood == hood)
                .min_by_key(|(i, r)| (r.on, *i))
                .map(|(i, _)| i)
            else {
                break;
            };
            self.records.remove(i);
        }
    }

    /// Forget the calls older than `days` (a household not called on for a
    /// year forgets the call).
    pub fn forget(&mut self, on: SimDate, days: u32) {
        self.records.retain(|r| r.on.days_until(on) <= days);
    }
}

/// The door slots of a neighbourhood: one per `residents_per_door`
/// residents (adults and children), within `doors`. Zero for a
/// neighbourhood with nobody.
pub fn doors(state: &State, content: &ContentBundle, hood: NeighborhoodId) -> u32 {
    let Some(h) = state.population.hoods.get(hood.index()) else {
        return 0;
    };
    let residents = h.adults() + h.children as u64;
    if residents == 0 {
        return 0;
    }
    let p = &content.characters.households;
    let [lo, hi] = p.doors;
    ((residents / p.residents_per_door.max(1) as u64) as u32).clamp(lo, hi.max(lo))
}

/// The generation of a door's household (households move in M7).
pub fn generation(_state: &State, _hood: NeighborhoodId, _door: u16) -> u8 {
    0
}

fn draw(state: &State, hood: NeighborhoodId, door: u16, generation: u8, part: Part) -> mandate_core::SimRng {
    rng(
        state.meta.seed,
        &[
            KEY_HOUSEHOLD,
            hood.0 as u64,
            door as u64,
            generation as u64,
            part as u64,
        ],
    )
}

/// The household behind a door, from live state; `None` past the
/// neighbourhood's doors or outside the world.
pub fn household(
    state: &State,
    content: &ContentBundle,
    hood: NeighborhoodId,
    door: u16,
) -> Option<Household> {
    if door as u32 >= doors(state, content, hood) {
        return None;
    }
    let n = state.world.neighborhoods.get(hood.index())?;
    let pop = state.population.hoods.get(hood.index())?;
    let p = &content.characters.households;
    let generation = generation(state, hood, door);
    let d = |part| draw(state, hood, door, generation, part);

    // Who answers: a name that never changes with the state.
    let pool = |id: &str| -> Vec<String> {
        content
            .name_pools
            .get(id)
            .map(|p| p.prefixes.clone())
            .unwrap_or_default()
    };
    let (given, family) =
        crate::politics::person_name(&mut d(Part::Name), &pool("names.given"), &pool("names.family"));

    // Their cohort, by adults in each.
    let weights: Vec<f64> = pop.cohorts.iter().map(|c| c.count as f64).collect();
    let cohort = d(Part::Cohort).weighted_index(&weights).unwrap_or(0);
    let c = pop.cohorts.get(cohort)?;
    let band = crate::population::age_of(cohort);
    let class = crate::population::class_of(cohort);
    let community = cohort / crate::population::GROUPS;
    let lo: f64 = 18.0
        + crate::population::BAND_YEARS
            .iter()
            .take(band)
            .sum::<f64>();
    let hi = lo + crate::population::BAND_YEARS.get(band).copied().unwrap_or(15.0);
    let age = d(Part::Age).range_f64(lo, hi) as u32;

    // How many live there: adults, and children by the neighbourhood's
    // children per adult (none past working age).
    let mut size = d(Part::Size);
    let adults = size.range_i64(p.adults[0] as i64, p.adults[1].max(p.adults[0]) as i64) as u8;
    let per_adult = pop.children as f64 / pop.adults().max(1) as f64;
    let children = if band >= crate::population::AGE_BANDS - 1 {
        0
    } else {
        let mean = per_adult * adults as f64;
        (mean.floor() as u8) + u8::from(size.chance(mean.fract()))
    };

    let owner = d(Part::Tenure).chance(p.owner_by_class.get(class).copied().unwrap_or(0.5));

    // Where the earner works: retired past working age; between jobs by the
    // town's employment for their class; else a named plant of the town by
    // its jobs, or a sector by its jobs.
    let econ = state.economy.munis.get(n.municipality.index());
    let mut wr = d(Part::Work);
    let u = wr.next_f64();
    let work = if band >= crate::population::AGE_BANDS - 1 {
        Work::Retired
    } else if econ.is_some_and(|e| u >= e.class_employment.get(class).copied().unwrap_or(1.0)) {
        Work::Looking
    } else {
        let plants: Vec<&crate::economy::plants::Plant> = state
            .economy
            .plants
            .iter()
            .filter(|pl| {
                pl.municipality == n.municipality
                    && !matches!(pl.status, crate::economy::plants::PlantStatus::Closed { .. })
            })
            .collect();
        let sectors = econ.map(|e| e.sectors.as_slice()).unwrap_or(&[]);
        let mut w: Vec<f64> = plants.iter().map(|pl| pl.capacity.max(0.0)).collect();
        // A sector's jobs, less the plants' (which are in it).
        let in_plants: f64 = w.iter().sum();
        let jobs: f64 = sectors.iter().map(|s| s.jobs.max(0.0)).sum();
        let scale = if jobs > 0.0 {
            ((jobs - in_plants).max(0.0) / jobs).max(0.0)
        } else {
            0.0
        };
        w.extend(sectors.iter().map(|s| s.jobs.max(0.0) * scale));
        match wr.weighted_index(&w) {
            Some(i) if i < plants.len() => Work::Plant {
                plant: plants[i].id.0,
            },
            Some(i) => state
                .economy
                .sector_keys
                .get(i - plants.len())
                .map(|k| Work::Sector { sector: k.clone() })
                .unwrap_or(Work::Looking),
            None => Work::Looking,
        }
    };

    // What worries them: the neighbourhood's felt worries, each weighted by
    // how far their cohort leans toward it beyond the neighbourhood.
    let felt = crate::concerns::ranked(
        &state.population,
        content,
        &state.world,
        crate::concerns::Scope::Neighbourhood(hood),
        mandate_content::condition::Worry::Felt,
    );
    let lean = |weights: &[f64], att: &[f32]| -> f64 {
        let norm: f64 = weights.iter().map(|w| w.abs()).sum();
        if norm <= 0.0 {
            return 0.0;
        }
        weights
            .iter()
            .zip(att)
            .map(|(w, x)| w * *x as f64)
            .sum::<f64>()
            / norm
    };
    let mean = crate::politics::mean_attitudes(&state.population, &[hood.index()]);
    let ww: Vec<f64> = felt
        .iter()
        .map(|(i, level)| {
            let Some(def) = content.concerns.get(*i) else {
                return 0.0;
            };
            let mine = lean(&def.attitude_weights, &c.attitudes);
            let theirs = lean(&def.attitude_weights, &mean);
            level.max(0.0) * (1.0 + clamp(mine - theirs, -0.5, 0.5))
        })
        .collect();
    let worry = d(Part::Worry)
        .weighted_index(&ww)
        .and_then(|k| felt.get(k))
        .map(|(i, _)| *i);

    // What they think of the player: their cohort's, with a keyed spread.
    let mut vr = d(Part::View);
    let aware = vr.next_f64() < c.player_awareness as f64;
    let view = clamp(
        c.player_approval as f64 + vr.normal(0.0, p.view_sd),
        -1.0,
        1.0,
    );

    // An empty home, by the town's vacancy (the same doors as it rises).
    let vacancy = econ.map(|e| clamp(e.housing.vacancy, 0.0, 0.6)).unwrap_or(0.0);
    let vacant = d(Part::Empty).next_f64() < vacancy;

    let speaker = super::player_character(state).and_then(|me| {
        state
            .politics
            .characters
            .households
            .record(me, hood, door, generation)
            .and_then(|r| r.speaker)
    });
    Some(Household {
        hood,
        door,
        generation,
        vacant,
        given,
        family,
        age,
        cohort,
        class,
        community,
        adults,
        children,
        owner,
        work,
        worry,
        aware,
        view,
        speaker,
    })
}

/// Whether anyone answers a door today (an occupied home; nobody in by a
/// keyed draw on the door and the day).
pub fn answers(state: &State, content: &ContentBundle, h: &Household) -> bool {
    if h.vacant {
        return false;
    }
    let p = &content.characters.households;
    let u = rng(
        state.meta.seed,
        &[
            KEY_DOOR_DAY,
            h.hood.0 as u64,
            h.door as u64,
            state.calendar.date.0 as u64,
        ],
    )
    .next_f64();
    u >= p.not_in
}

/// The chance a household says yes when asked for their vote or a poster
/// (shown in words only; the same chance is rolled).
pub fn ask_chance(content: &ContentBundle, h: &Household) -> f64 {
    let p = &content.characters.households;
    let view = if h.aware { h.view } else { 0.0 };
    clamp(p.ask_base + p.ask_view * view, p.ask_min, p.ask_max)
}

/// The keyed draw for an ask at a door today (`ask_chance` is its odds).
pub fn ask_roll(state: &State, h: &Household) -> f64 {
    rng(
        state.meta.seed,
        &[
            KEY_DOOR_ASK,
            h.hood.0 as u64,
            h.door as u64,
            state.calendar.date.0 as u64,
        ],
    )
    .next_f64()
}

/// The doors a canvass knocks when the engine chooses (the one-tap
/// canvass, §3.5): up to `street_doors`, those not called on lately first,
/// from a keyed starting door that turns with the date.
pub fn engine_doors(state: &State, content: &ContentBundle, hood: NeighborhoodId) -> Vec<u16> {
    let n = doors(state, content, hood);
    if n == 0 {
        return Vec::new();
    }
    let p = &content.characters.households;
    let start = rng(
        state.meta.seed,
        &[KEY_DOOR_PICK, hood.0 as u64, state.calendar.date.0 as u64],
    )
    .below(n as u64) as u32;
    let me = super::player_character(state);
    let called = |door: u16| {
        me.is_some_and(|me| {
            state
                .politics
                .characters
                .households
                .records
                .iter()
                .any(|r| r.by == me && r.hood == hood && r.door == door)
        })
    };
    let mut order: Vec<u16> = (0..n).map(|k| ((start + k) % n) as u16).collect();
    // Stable: the rotation's order within each group.
    order.sort_by_key(|d| called(*d));
    order.truncate(p.street_doors as usize);
    order
}

/// Promote a household's answerer to a stored Named resident (they brought
/// an ask), by the path a `Meet` uses: a resident (a retiree past working
/// age) of the neighbourhood with the household's name and age, and its
/// origin. Returns them, or the one promoted before.
pub fn promote(
    state: &mut State,
    content: &ContentBundle,
    h: &Household,
) -> Option<CharacterId> {
    if let Some(id) = h.speaker {
        return Some(id);
    }
    let retired = matches!(h.work, Work::Retired);
    let occ = content
        .occupations
        .iter()
        .find(|o| {
            o.id == if retired {
                "occupation.retiree"
            } else {
                "occupation.resident"
            }
        })
        .or_else(|| content.occupations.iter().find(|o| o.ambient))?;
    let year = state.calendar.date.calendar().year;
    let mut r = rng(
        state.meta.seed,
        &[
            KEY_SPEAKER,
            h.hood.0 as u64,
            h.door as u64,
            h.generation as u64,
        ],
    );
    let id: CharacterId = state.politics.characters.ids.alloc();
    let mut c = named_character(
        content,
        &state.population,
        &mut r,
        id,
        occ,
        h.hood.0,
        None,
        year,
    );
    if let Some(n) = c.named.as_deref_mut() {
        n.given = h.given.clone();
        n.family = h.family.clone();
        n.birth_year = year.saturating_sub(h.age);
        n.origin = Some((origin_key(h.hood), h.door as u32, h.generation));
    }
    state.politics.characters.people.push(c);
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(hood: u32, door: u16, on: u32) -> DoorstepRecord {
        DoorstepRecord {
            by: CharacterId(1),
            hood: NeighborhoodId(hood),
            door,
            generation: 0,
            on: SimDate(on),
            went: Went::Warm,
            said: Said::Listened,
            worry: None,
            promise: None,
            speaker: None,
        }
    }

    #[test]
    fn records_are_one_a_household_bounded_and_forgotten() {
        let p = &HouseholdParams::default();
        let mut h = Households::default();
        h.remember(p, rec(1, 3, 100));
        let mut again = rec(1, 3, 120);
        again.speaker = None;
        h.records[0].speaker = Some(CharacterId(9));
        h.remember(p, again);
        assert_eq!(h.records.len(), 1, "one record a household");
        assert_eq!(h.records[0].on, SimDate(120));
        assert_eq!(h.records[0].speaker, Some(CharacterId(9)), "the speaker is kept");
        for d in 0..40u16 {
            h.remember(p, rec(1, 10 + d, 200 + d as u32));
        }
        assert_eq!(
            h.records.iter().filter(|r| r.hood == NeighborhoodId(1)).count(),
            p.records_per_hood as usize
        );
        assert!(h.records.iter().all(|r| r.on.0 >= 200), "the oldest forgotten");
        h.remember(p, rec(2, 0, 200 + p.forget_days + 100));
        assert_eq!(h.records.len(), 1, "a year on, the calls are forgotten");
    }

    #[test]
    fn a_street_argues_its_most_argued_worry_ties_the_first() {
        let mut s = Street {
            hood: NeighborhoodId(0),
            on: SimDate(0),
            doors: vec![0, 1, 2, 3],
            argued: vec!["a".into(), "b".into()],
            heard: Vec::new(),
        };
        assert_eq!(s.issue().as_deref(), Some("a"));
        s.argued.push("b".into());
        assert_eq!(s.issue().as_deref(), Some("b"));
        s.argued.clear();
        assert_eq!(s.issue(), None);
    }
}
