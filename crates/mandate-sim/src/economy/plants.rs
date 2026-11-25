//! Named plants (DEC-110, `docs/design/INDUSTRIES.md` §3, §5): a town's
//! larger employers. A plant is a physical thing in a town: a place of an
//! existing kind with an industry, a product, a name in parts and capacity.
//! Its jobs are its share of its industry's jobs here, so plants, industries
//! and sectors always reconcile.
//!
//! **The closure process** is per plant, deterministic and explained. Each
//! month the plant's margin-adjusted demand is `m`: `effective` times (1 plus
//! its margin plus any relief), where `effective` is its industry's demand
//! in the town as policy leaves it:
//!
//! 1. *Short time* when `m` is under the short line for some months running.
//! 2. *A notice with a date* when `m` is under the notice line for longer.
//!    The only draw is the notice's length (keyed by plant and day), so the
//!    date shown is the date used.
//! 3. *A reprieve* when `m` is back over the reprieve line for some months
//!    before the date; else *the closure* on the date.
//!
//! Each step is an event whose cause is the weakest smoothed factor of the
//! industry's demand (the spine's `Pressure`); the four share a thread. The
//! review belongs to the economy: others read it ([`facts`]) and push on it
//! only through requests (`Effect::PlantRelief`).

use super::industries::{self, Factor, IndustryState, Month, TownIn};
use super::{EconomyState, MuniEcon, whole};
use crate::events::{EventId, EventKind};
use crate::scheduler::{Effect, Outs};
use crate::spine::{Cause, Causes, Link, Pressure, Process};
use crate::world::{MunicipalityId, NeighborhoodId, PlaceId, SettlementClass, World};
use mandate_content::{ContentBundle, ContentKey, IndustryDef, IndustryParams, PlantOpens};
use mandate_core::{IdAllocator, SimDate, Stream, define_id, rng::keyed};
use serde::{Deserialize, Serialize};

define_id!(
    /// Identifier of a named plant. Never reused: a works that reopens is a
    /// new plant at the same place.
    PlantId
);

/// How a plant's name is formed. The client composes the name from the
/// form, the stem and the works' noun (`plant-name-<form>`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NameForm {
    /// The town: "the Garrowby cannery".
    Place,
    /// A family: "Hartley's cannery".
    Family,
    /// A word from the pool: "the Albion cannery".
    Word,
    /// Its district: "the Quayside cannery".
    Hood,
}

impl NameForm {
    pub fn key(self) -> &'static str {
        match self {
            NameForm::Place => "place",
            NameForm::Family => "family",
            NameForm::Word => "word",
            NameForm::Hood => "hood",
        }
    }
}

/// A plant's name in parts: the engine never writes the name out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlantName {
    pub form: NameForm,
    pub stem: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlantStatus {
    Running,
    ShortTime {
        since: SimDate,
    },
    /// Under notice: it closes on `closes_on` unless demand comes back.
    Threatened {
        since: SimDate,
        closes_on: SimDate,
    },
    /// Shut, and why: the weakest factor of its industry's demand then.
    Closed {
        since: SimDate,
        factor: Factor,
    },
}

impl PlantStatus {
    /// Snake-case key: the Fluent suffix (`plant-status-<key>`).
    pub fn key(&self) -> &'static str {
        match self {
            PlantStatus::Running => "running",
            PlantStatus::ShortTime { .. } => "short_time",
            PlantStatus::Threatened { .. } => "threatened",
            PlantStatus::Closed { .. } => "closed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plant {
    pub id: PlantId,
    pub municipality: MunicipalityId,
    pub place: PlaceId,
    /// Content key of its industry (a plant outlives registry order).
    pub industry: ContentKey,
    /// What it makes: a product id of its industry.
    pub product: ContentKey,
    /// The good that product is (what a cause that blames external demand
    /// names; `EconomyState::good_keys`).
    pub good: industries::GoodId,
    pub name: PlantName,
    /// Jobs at neutral demand. A closed plant keeps its last size.
    pub capacity: f64,
    /// Drawn once: how much weaker demand it bears than its industry's line.
    pub margin: f32,
    /// The year it was founded.
    pub founded: i32,
    pub status: PlantStatus,
    /// Months running under the notice line.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub strain: u8,
    /// Months running toward the next change of hours: under the short
    /// line (running), back over it (short time), over the reprieve line
    /// (under notice).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub turn: u8,
    /// Relief on its margin a request granted, while `relief_months` last.
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub relief: f32,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub relief_months: u16,
}

fn is_zero(x: &u8) -> bool {
    *x == 0
}
fn is_zero_u16(x: &u16) -> bool {
    *x == 0
}
fn is_zero_f32(x: &f32) -> bool {
    *x == 0.0
}

impl Plant {
    pub fn is_open(&self) -> bool {
        !matches!(self.status, PlantStatus::Closed { .. })
    }
}

// Keyed-draw tags (DEC-041).
const KEY_NOTICE: u64 = 0x1D10;
const KEY_PLANT: u64 = 0x1D11;
const KEY_PLANT_NAME: u64 = 0x1D12;
const KEY_PRODUCT_REGION: u64 = 0x1D13;
const KEY_PRODUCT_TOWN: u64 = 0x1D14;

fn class_index(c: SettlementClass) -> usize {
    match c {
        SettlementClass::Rural => 0,
        SettlementClass::Town => 1,
        SettlementClass::City => 2,
        SettlementClass::Metro => 3,
    }
}

/// Capacity of a town's open plants in one industry.
pub(super) fn open_capacity(plants: &[Plant], mo: &Month, m: usize, k: usize) -> f64 {
    mo.town_plants
        .get(m)
        .map_or(&[][..], |v| v.as_slice())
        .iter()
        .map(|i| *i as usize)
        .filter(|i| mo.plant_at[*i].1 == k as u16 && plants[*i].is_open())
        .map(|i| plants[i].capacity)
        .sum()
}

// --- The closure process --------------------------------------------------------------

/// One plant's month: what its industry's step knows.
pub(super) struct Step<'a> {
    /// The market that weighs most on its industry's external demand.
    pub market: industries::MarketId,
    /// Its industry's demand in the town as policy leaves it.
    pub effective: f64,
    pub def: &'a IndustryDef,
    pub date: SimDate,
    pub seed: u64,
    pub recession: Option<EventId>,
    pub in_recession: bool,
    /// The weakest supply link's upstream, if the industry has links.
    pub upstream: Option<ContentKey>,
}

/// A plant's whole jobs: its share of its industry's jobs here.
fn jobs_of(plant: &Plant, st: &IndustryState) -> u32 {
    if st.capacity > 0.0 {
        whole(st.jobs * plant.capacity / st.capacity)
    } else {
        0
    }
}

/// Why, as the spine records it: the weakest smoothed factor of the
/// industry's demand, and the running recession when that factor is the
/// world cycle.
fn blame(st: &IndustryState, step: &Step<'_>, good: industries::GoodId) -> (Factor, Causes) {
    let (f, _) = st.weakest();
    let causes = Causes::from(Cause::Pressure(f.pressure(step.market, good))).and_event(
        if f == Factor::Cycle {
            step.recession
        } else {
            None
        },
        Link::Caused,
    );
    (f, causes)
}

/// Advance one open plant by a month (the module's rule). Writes the plant,
/// its industry's jobs and capacity when it closes, and the town's last
/// notice date.
pub(super) fn process(
    step: &Step<'_>,
    p: &IndustryParams,
    plant: &mut Plant,
    st: &mut IndustryState,
    last_notice: &mut Option<SimDate>,
    out: &mut Outs<'_>,
) {
    let date = step.date;
    let m = step.effective * (1.0 + plant.margin as f64 + plant.relief as f64);
    if plant.relief_months > 0 {
        plant.relief_months -= 1;
        if plant.relief_months == 0 {
            plant.relief = 0.0;
        }
    }
    let town = plant.municipality.0;
    let good = plant.good;
    let bump = |x: u8| x.saturating_add(1);
    match plant.status {
        PlantStatus::Closed { .. } => {}
        PlantStatus::Running => {
            plant.strain = if m < p.notice_line {
                bump(plant.strain)
            } else {
                0
            };
            plant.turn = if m < p.short_line {
                bump(plant.turn)
            } else {
                0
            };
            if plant.turn >= p.short_months {
                plant.status = PlantStatus::ShortTime { since: date };
                plant.turn = 0;
                let (factor, causes) = blame(st, step, good);
                out.events.record(
                    date,
                    EventKind::PlantShortTime {
                        municipality: town,
                        plant: plant.id.0,
                        industry: plant.industry.clone(),
                        jobs: jobs_of(plant, st),
                        factor,
                    },
                    causes,
                );
            }
        }
        PlantStatus::ShortTime { .. } => {
            plant.strain = if m < p.notice_line {
                bump(plant.strain)
            } else {
                0
            };
            plant.turn = if m >= p.full_line {
                bump(plant.turn)
            } else {
                0
            };
            // One new notice a town in the gap, outside a recession: a
            // second plant waits on short time.
            let free = step.in_recession
                || last_notice
                    .is_none_or(|d| industries::add_months(d, p.notice_gap_months as u32) <= date);
            if step.def.plants.closes && plant.strain >= p.notice_after_months && free {
                let mut rng = keyed(
                    step.seed,
                    Stream::Economy,
                    &[KEY_NOTICE, plant.id.0 as u64, date.0 as u64],
                );
                let months = rng.range_i64(p.notice_months[0] as i64, p.notice_months[1] as i64);
                let closes_on = industries::add_months(date, months.max(1) as u32);
                plant.status = PlantStatus::Threatened {
                    since: date,
                    closes_on,
                };
                plant.strain = 0;
                plant.turn = 0;
                *last_notice = Some(date);
                let (factor, causes) = blame(st, step, good);
                out.events.record(
                    date,
                    EventKind::PlantThreatened {
                        municipality: town,
                        plant: plant.id.0,
                        industry: plant.industry.clone(),
                        jobs: jobs_of(plant, st),
                        closes_on,
                        factor,
                    },
                    causes,
                );
            } else if plant.turn >= p.short_months {
                // Full time again: no event (the status says so).
                plant.status = PlantStatus::Running;
                plant.turn = 0;
            }
        }
        PlantStatus::Threatened { since, closes_on } => {
            plant.turn = if m >= p.reprieve_line {
                bump(plant.turn)
            } else {
                0
            };
            if plant.turn >= p.reprieve_months && date <= closes_on {
                // Demand came back in time: the notice is lifted. The plant
                // stays on short time until its hours recover.
                plant.status = PlantStatus::ShortTime { since };
                plant.strain = 0;
                plant.turn = 0;
                out.events.record(
                    date,
                    EventKind::PlantReprieved {
                        municipality: town,
                        plant: plant.id.0,
                        industry: plant.industry.clone(),
                        jobs: jobs_of(plant, st),
                    },
                    Cause::Process(Process::Markets),
                );
            } else if date >= closes_on {
                close(step, plant, st, None, out);
            }
        }
    }
}

/// Close a plant: its capacity leaves the industry, its share of the
/// industry's jobs goes at once, and its place is asked to close (DEC-090
/// D3). The plant keeps its record. `cause`: the lab's planted closure
/// names itself; otherwise the weakest factor is blamed.
pub(super) fn close(
    step: &Step<'_>,
    plant: &mut Plant,
    st: &mut IndustryState,
    cause: Option<Cause>,
    out: &mut Outs<'_>,
) -> EventId {
    let (_, blamed) = blame(st, step, plant.good);
    let causes = match cause {
        Some(c) => Causes::from(c),
        None => blamed,
    };
    let kind = shut(plant, st, step.def, step.date, step.upstream.clone());
    let ev = out.events.record(step.date, kind, causes);
    out.effects.push(
        Effect::PlaceStatus {
            place: plant.place.0,
            open: false,
        },
        Cause::caused_by(ev),
    );
    ev
}

/// The state change of a closure, and the event that says so.
fn shut(
    plant: &mut Plant,
    st: &mut IndustryState,
    def: &IndustryDef,
    date: SimDate,
    upstream: Option<ContentKey>,
) -> EventKind {
    let jobs = jobs_of(plant, st);
    let (factor, _) = st.weakest();
    if st.capacity > 0.0 {
        st.jobs -= st.jobs * (plant.capacity / st.capacity).min(1.0);
    }
    st.capacity = (st.capacity - plant.capacity).max(st.small).max(0.0);
    plant.status = PlantStatus::Closed {
        since: date,
        factor,
    };
    plant.strain = 0;
    plant.turn = 0;
    EventKind::PlantClosure {
        municipality: plant.municipality.0,
        sector: def.sector.clone(),
        jobs,
        industry: Some(plant.industry.clone()),
        plant: Some(plant.id.0),
        factor: Some(factor),
        upstream: if factor == Factor::Supply {
            upstream
        } else {
            None
        },
    }
}

/// Close a plant now, outside the monthly step (the lab's planted
/// closure; a test): the same change the economy's own closure makes, for
/// the cause given, with the place closed at once. The sector totals are
/// rewritten. `None` for an unknown or closed plant.
pub fn close_now(
    state: &mut crate::State,
    content: &ContentBundle,
    plant: PlantId,
    cause: Cause,
) -> Option<EventId> {
    let date = state.calendar.date;
    let econ = &mut state.economy;
    let pl = econ.plants.get_mut(plant.index())?;
    if !pl.is_open() {
        return None;
    }
    let m = pl.municipality.index();
    let k = content.industry_index(&pl.industry)?;
    let def = &content.industries[k];
    let upstream = industries::weakest_link(econ, &state.world, content, m, k);
    let pl = econ.plants.get_mut(plant.index())?;
    let st = econ
        .munis
        .get_mut(m)?
        .industries
        .iter_mut()
        .find(|x| x.k as usize == k)?;
    let kind = shut(pl, st, def, date, upstream);
    let place = pl.place;
    industries::refresh(econ, content, m);
    let ev = state.events.record(date, kind, cause);
    state.world.close_place(place, date);
    Some(ev)
}

// --- Placement and names --------------------------------------------------------------

/// What a town's branch of an industry makes: the product whose `with`
/// industry is largest in the town; else the region's choice (a keyed
/// draw), which most towns of the region share.
pub fn product_for(
    content: &ContentBundle,
    seed: u64,
    world: &World,
    mu: &MuniEcon,
    m: usize,
    def: &IndustryDef,
) -> ContentKey {
    let mut best: Option<(&ContentKey, f64)> = None;
    for x in &def.products {
        let Some(w) = x.with.as_ref().and_then(|w| content.industry_index(w)) else {
            continue;
        };
        let jobs = mu
            .industries
            .iter()
            .find(|s| s.k as usize == w)
            .map_or(0.0, |s| s.jobs.max(s.capacity));
        if jobs > 0.0 && best.is_none_or(|b| jobs > b.1) {
            best = Some((&x.id, jobs));
        }
    }
    if let Some((id, _)) = best {
        return id.clone();
    }
    let id = industries::key_of(&def.id);
    let region = world.municipalities[m].region.0 as u64;
    let n = def.products.len() as u64;
    let mut r = keyed(seed, Stream::Worldgen, &[KEY_PRODUCT_REGION, region, id]);
    let regional = r.below(n) as usize;
    let mut t = keyed(seed, Stream::Worldgen, &[KEY_PRODUCT_TOWN, m as u64, id]);
    let pick = if t.chance(0.6) {
        regional
    } else {
        t.below(n) as usize
    };
    def.products
        .get(pick)
        .or(def.products.first())
        .map(|x| x.id.clone())
        .unwrap_or_default()
}

/// The good a product is, as the state names it (the first good if the
/// product or its good is unknown: validation refuses that content).
pub(super) fn good_of(
    good_keys: &[ContentKey],
    content: &ContentBundle,
    product: &str,
) -> industries::GoodId {
    let i = content
        .product(product)
        .and_then(|(_, x)| good_keys.iter().position(|k| *k == x.good))
        .unwrap_or(0);
    industries::GoodId(i as u32)
}

/// A plant's name in parts: one keyed draw of the form (the town, a
/// family, a word, its district); a clash with another plant of the town
/// that makes the same thing takes the next form.
#[allow(clippy::too_many_arguments)]
fn name_for(
    content: &ContentBundle,
    seed: u64,
    world: &World,
    plants: &[Plant],
    m: usize,
    hood: NeighborhoodId,
    def: &IndustryDef,
    product: &ContentKey,
    ordinal: u32,
) -> PlantName {
    let p = &content.economy.industries;
    let mut r = keyed(
        seed,
        Stream::Worldgen,
        &[
            KEY_PLANT_NAME,
            m as u64,
            industries::key_of(&def.id),
            ordinal as u64,
        ],
    );
    let forms = [
        NameForm::Place,
        NameForm::Family,
        NameForm::Word,
        NameForm::Hood,
    ];
    let first = r.weighted_index(&p.name_forms).unwrap_or(0);
    let families: &[String] = content
        .name_pools
        .get("names.family")
        .map_or(&[], |x| x.prefixes.as_slice());
    let family = r.pick(families).cloned();
    let word = r.pick(&p.works_words).cloned();
    let town = world.municipalities[m].name.clone();
    let district = world
        .neighborhoods
        .get(hood.index())
        .map(|h| h.name.clone());
    for n in 0..forms.len() {
        let form = forms[(first + n) % forms.len()];
        let stem = match form {
            NameForm::Place => Some(town.clone()),
            NameForm::Family => family.clone(),
            NameForm::Word => word.clone(),
            NameForm::Hood => district.clone(),
        };
        let Some(stem) = stem.filter(|s| !s.is_empty()) else {
            continue;
        };
        let clash = plants.iter().any(|x| {
            x.municipality.index() == m
                && x.is_open()
                && x.product == *product
                && x.name.form == form
                && x.name.stem == stem
        });
        if !clash {
            return PlantName { form, stem };
        }
    }
    PlantName {
        form: NameForm::Place,
        stem: town,
    }
}

/// Place named plants at the works a town already has, one each (DEC-110):
/// each works place takes the town's largest industry that names its kind,
/// if that industry is big enough here. The plant takes a share of the
/// industry's capacity out of its unnamed firms, so nothing about the
/// town's jobs changes. Used for a new world (before history) and for a
/// save from before industries (at its current capacity); all draws are
/// keyed by town and industry.
pub fn place_all(
    econ: &mut EconomyState,
    world: &World,
    content: &ContentBundle,
    seed: u64,
    date: SimDate,
    start: SimDate,
) {
    let p = &content.economy.industries;
    let year = date.calendar().year as i32;
    let _ = start;
    let n = world.municipalities.len().min(econ.munis.len());
    for m in 0..n {
        let wm = &world.municipalities[m];
        let class = class_index(wm.class);
        let town_jobs = econ.munis[m].total_jobs();
        for pid in &wm.places {
            let Some(place) = world.places.get(pid.index()) else {
                continue;
            };
            if !place.is_open() {
                continue;
            }
            let kind = place.kind.key();
            // The town's largest industry whose works this kind of place is
            // and that has no plant here yet (ties to registry order).
            let mut pick: Option<(usize, f64)> = None;
            for st in &econ.munis[m].industries {
                let Some(def) = content.industries.get(st.k as usize) else {
                    continue;
                };
                if def.plants.place_kind.as_deref() != Some(kind) {
                    continue;
                }
                let taken = econ
                    .plants
                    .iter()
                    .any(|x| x.municipality.index() == m && x.industry == def.id);
                if !taken && pick.is_none_or(|b| st.capacity > b.1) {
                    pick = Some((st.k as usize, st.capacity));
                }
            }
            let Some((k, cap)) = pick else {
                continue;
            };
            let def = &content.industries[k];
            let mut r = keyed(
                seed,
                Stream::Worldgen,
                &[KEY_PLANT, m as u64, industries::key_of(&def.id), 0],
            );
            let share = r.range_f64(def.plants.named_share[0], def.plants.named_share[1]);
            let margin = r
                .normal(0.0, def.plants.margin_sd)
                .clamp(-3.0 * def.plants.margin_sd, 3.0 * def.plants.margin_sd);
            let age = r.range_i64(def.plants.founded[0] as i64, def.plants.founded[1] as i64);
            let size = (cap * share).min(p.max_town_share * town_jobs);
            if size < p.min_plant[class] {
                continue;
            }
            let product = product_for(content, seed, world, &econ.munis[m], m, def);
            let good = good_of(&econ.good_keys, content, &product);
            let id = PlantId(econ.plant_ids.alloc_raw());
            let name = name_for(
                content,
                seed,
                world,
                &econ.plants,
                m,
                place.neighborhood,
                def,
                &product,
                id.0,
            );
            if let Some(st) = econ.munis[m]
                .industries
                .iter_mut()
                .find(|s| s.k as usize == k)
            {
                st.small = (st.small - size).max(0.0);
            }
            econ.plants.push(Plant {
                id,
                municipality: wm.id,
                place: *pid,
                industry: def.id.clone(),
                product,
                good,
                name,
                capacity: size,
                margin: margin as f32,
                founded: year - age as i32,
                status: PlantStatus::Running,
                strain: 0,
                turn: 0,
                relief: 0.0,
                relief_months: 0,
            });
        }
    }
}

// --- Openings ---------------------------------------------------------------------------

/// The most recently closed plant in a town that `fits` and whose place no
/// open plant has taken: the works an opening would take.
fn closed_site(
    plants: &[Plant],
    mo: &Month,
    m: usize,
    fits: impl Fn(&Plant, u16) -> bool,
) -> Option<usize> {
    let mine = mo.town_plants.get(m).map_or(&[][..], |v| v.as_slice());
    let mut best: Option<(usize, SimDate)> = None;
    for &i in mine {
        let i = i as usize;
        let pl = &plants[i];
        let PlantStatus::Closed { since, .. } = pl.status else {
            continue;
        };
        if !fits(pl, mo.plant_at[i].1) {
            continue;
        }
        // Taken already: by a plant of the town as the month began, or by
        // one opened this month (after them in the list).
        let taken = |x: &Plant| x.place == pl.place && x.is_open();
        if mine.iter().any(|j| taken(&plants[*j as usize]))
            || plants[mo.plant_at.len()..].iter().any(taken)
        {
            continue;
        }
        if best.is_none_or(|b| since > b.1) {
            best = Some((i, since));
        }
    }
    best.map(|b| b.0)
}

/// Open a plant at a closed works: a new plant (ids are never reused) at
/// the same place, which is asked to reopen.
#[allow(clippy::too_many_arguments)]
fn open_at(
    mo: &Month,
    t: &TownIn<'_>,
    def: &IndustryDef,
    old: usize,
    capacity: f64,
    keep_name: bool,
    st: &mut IndustryState,
    plants: &mut Vec<Plant>,
    ids: &mut IdAllocator,
    cause: Cause,
    out: &mut Outs<'_>,
) {
    let p = &t.content.economy.industries;
    let id = PlantId(ids.alloc_raw());
    let place = plants[old].place;
    let hood = t
        .world
        .places
        .get(place.index())
        .map_or(NeighborhoodId(0), |x| x.neighborhood);
    let mut r = keyed(
        mo.seed,
        Stream::Economy,
        &[
            KEY_PLANT,
            t.m as u64,
            industries::key_of(&def.id),
            id.0 as u64 + 1,
        ],
    );
    // A new works is the last to go.
    let margin = r
        .normal(0.0, def.plants.margin_sd)
        .clamp(-3.0 * def.plants.margin_sd, 3.0 * def.plants.margin_sd)
        + p.new_plant_margin;
    let (product, name) = if keep_name {
        (plants[old].product.clone(), plants[old].name.clone())
    } else {
        // The product by region and town alone: the town's own mix is being
        // written, and a keyed pick needs none of it.
        let product = product_for(t.content, mo.seed, t.world, &MuniEcon::default(), t.m, def);
        let name = name_for(
            t.content, mo.seed, t.world, plants, t.m, hood, def, &product, id.0,
        );
        (product, name)
    };
    st.capacity += capacity;
    let good = good_of(&mo.good_keys, t.content, &product);
    plants.push(Plant {
        id,
        municipality: MunicipalityId(t.m as u32),
        place,
        industry: def.id.clone(),
        product,
        good,
        name,
        capacity,
        margin: margin as f32,
        founded: mo.date.calendar().year as i32,
        status: PlantStatus::Running,
        strain: 0,
        turn: 0,
        relief: 0.0,
        relief_months: 0,
    });
    let ev = out.events.record(
        mo.date,
        EventKind::PlantOpened {
            municipality: t.m as u32,
            plant: id.0,
            industry: def.id.clone(),
            jobs: whole(capacity),
            reopened: true,
        },
        cause,
    );
    out.effects.push(
        Effect::PlaceStatus {
            place: place.0,
            open: true,
        },
        Cause::caused_by(ev),
    );
}

/// A long boom reopens the last closed works of an industry here, at a
/// share of its old size. Returns whether one reopened.
#[allow(clippy::too_many_arguments)]
pub(super) fn reopen_after_boom(
    mo: &Month,
    p: &IndustryParams,
    def: &IndustryDef,
    k: usize,
    t: &TownIn<'_>,
    st: &mut IndustryState,
    plants: &mut Vec<Plant>,
    ids: &mut IdAllocator,
    out: &mut Outs<'_>,
) -> bool {
    let Some(old) = closed_site(plants, mo, t.m, |_, pk| pk as usize == k) else {
        return false;
    };
    let capacity = plants[old].capacity * p.boom_size;
    if capacity < 1.0 {
        return false;
    }
    let cause = Cause::Pressure(Pressure::Boom);
    open_at(mo, t, def, old, capacity, true, st, plants, ids, cause, out);
    true
}

/// What idle labour drew this month (DEC-110 §3, investment). With a closed
/// works to take, it gathers in the town's pool, and a plant opens there
/// when the pool is the smallest plant for the town's class: in the
/// industry with the best siting score × national demand among those whose
/// works that kind of place is, often not the one that left. With no works
/// to take, the unnamed firms grow by it, as they did before industries.
#[allow(clippy::too_many_arguments)]
pub(super) fn invest(
    mo: &Month,
    p: &IndustryParams,
    t: &TownIn<'_>,
    mu: &mut MuniEcon,
    plants: &mut Vec<Plant>,
    ids: &mut IdAllocator,
    amount: f64,
    class: SettlementClass,
    town_jobs: f64,
    out: &mut Outs<'_>,
) {
    let content = t.content;
    let opens = |k: u16| {
        content
            .industries
            .get(k as usize)
            .is_some_and(|d| d.plants.opens == PlantOpens::Investment)
    };
    let site = closed_site(plants, mo, t.m, |_, k| opens(k));
    // The unnamed firms of each investing industry grow, by capacity.
    let grow = |mu: &mut MuniEcon, total: f64| {
        let base: f64 = mu
            .industries
            .iter()
            .filter(|x| opens(x.k))
            .map(|x| x.capacity)
            .sum();
        if base > 0.0 {
            for x in mu.industries.iter_mut().filter(|x| opens(x.k)) {
                let add = total * x.capacity / base;
                x.small += add;
                x.capacity += add;
            }
        }
    };
    let Some(old) = site else {
        let total = amount + mu.prospect;
        mu.prospect = 0.0;
        grow(mu, total);
        return;
    };
    // A works waits: part gathers toward it, the rest grows the firms now.
    mu.prospect += amount * p.pool_share;
    grow(mu, amount * (1.0 - p.pool_share));
    if mu.prospect < p.min_plant[class_index(class)] {
        return;
    }
    // Which industry takes the works.
    let kind = content
        .industry(&plants[old].industry)
        .and_then(|d| d.plants.place_kind.clone());
    let scores = industries::scores(t.world, content, mo.seed, t.m);
    let mut pick: Option<(usize, f64)> = None;
    for (i, d) in content.industries.iter().enumerate() {
        if d.plants.opens != PlantOpens::Investment || d.plants.place_kind != kind {
            continue;
        }
        let x = scores[i] * mo.national_demand(i);
        if x > 0.0 && pick.is_none_or(|b| x > b.1) {
            pick = Some((i, x));
        }
    }
    let Some((k, _)) = pick else {
        return;
    };
    let def = &content.industries[k];
    let capacity = mu.prospect.min(p.max_town_share * town_jobs).max(1.0);
    let left = (mu.prospect - capacity).max(0.0);
    mu.prospect = 0.0;
    if !mu.industries.iter().any(|x| x.k as usize == k) {
        let productivity = mu
            .sectors
            .get(def.sector_index)
            .map(|s| s.productivity)
            .filter(|x| *x > 0.0)
            .unwrap_or(1.0);
        mu.industries.push(IndustryState {
            k: k as u16,
            wage: def.base_wage * t.local_wage * t.price * productivity,
            productivity,
            ..Default::default()
        });
        mu.industries.sort_by_key(|x| x.k);
    }
    let Some(st) = mu.industries.iter_mut().find(|x| x.k as usize == k) else {
        return;
    };
    st.small += left;
    st.capacity += left;
    open_at(
        mo,
        t,
        def,
        old,
        capacity,
        false,
        st,
        plants,
        ids,
        Cause::Pressure(Pressure::SlackLabour),
        out,
    );
}

// --- Requests ---------------------------------------------------------------------------

/// Apply a relief request to a plant's margin (the economy applies; the
/// arc of a plant's review and a rescue law push on the review this way).
pub(super) fn relieve(plants: &mut [Plant], plant: u32, relief: f64, months: u16) {
    if let Some(pl) = plants.get_mut(plant as usize)
        && pl.is_open()
    {
        pl.relief = (pl.relief as f64 + relief).clamp(-0.5, 0.5) as f32;
        pl.relief_months = pl.relief_months.max(months);
    }
}

// --- Whole numbers and facts -----------------------------------------------------------

/// A plant's whole jobs on a town's sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlantJobs {
    pub plant: PlantId,
    pub jobs: u32,
}

/// An industry's whole jobs in a town: its plants and its unnamed firms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndustryJobs {
    /// Registry index of the industry.
    pub k: usize,
    pub jobs: u32,
    pub plants: Vec<PlantJobs>,
    pub small: u32,
}

/// A town's whole jobs, top down (DEC-110 §5): each sector's jobs rounded;
/// a split sector's shared among its industries; an industry's among its
/// open plants and its unnamed firms (by capacity), each by the one
/// largest-remainder rounding. So the figures on a plant's sheet add up to
/// its industry's, and those to the town's sector line, every time.
pub fn town_jobs(
    econ: &EconomyState,
    content: &ContentBundle,
    m: usize,
) -> (Vec<u32>, Vec<IndustryJobs>) {
    let Some(mu) = econ.munis.get(m) else {
        return (Vec::new(), Vec::new());
    };
    let sectors: Vec<u32> = mu.sectors.iter().map(|s| whole(s.jobs)).collect();
    let mut out = Vec::with_capacity(mu.industries.len());
    for (k, total) in sectors.iter().enumerate() {
        let members: Vec<&IndustryState> = mu
            .industries
            .iter()
            .filter(|x| {
                content
                    .industries
                    .get(x.k as usize)
                    .is_some_and(|d| d.sector_index == k)
            })
            .collect();
        if members.is_empty() {
            continue;
        }
        let parts =
            industries::apportion(*total, &members.iter().map(|x| x.jobs).collect::<Vec<_>>());
        for (st, jobs) in members.iter().zip(parts) {
            let key = econ.industry_keys.get(st.k as usize);
            let open: Vec<&Plant> = econ
                .plants
                .iter()
                .filter(|p| p.municipality.index() == m && p.is_open() && Some(&p.industry) == key)
                .collect();
            let mut caps: Vec<f64> = open.iter().map(|p| p.capacity).collect();
            caps.push(st.small);
            let split = industries::apportion(jobs, &caps);
            out.push(IndustryJobs {
                k: st.k as usize,
                jobs,
                plants: open
                    .iter()
                    .zip(&split)
                    .map(|(p, j)| PlantJobs {
                        plant: p.id,
                        jobs: *j,
                    })
                    .collect(),
                small: split.last().copied().unwrap_or(0),
            });
        }
    }
    (sectors, out)
}

/// What an arc, a view or the lab may read of a plant (DEC-110 §5; the
/// review belongs to the economy, DEC-109 answer 2). Its people are found
/// through the place, by the characters domain.
#[derive(Clone, Debug, PartialEq)]
pub struct PlantFacts {
    pub plant: PlantId,
    pub place: PlaceId,
    pub municipality: MunicipalityId,
    pub neighborhood: Option<NeighborhoodId>,
    pub industry: ContentKey,
    pub product: ContentKey,
    pub name: PlantName,
    /// Whole jobs now (0 once closed), and their share of the town's.
    pub jobs: u32,
    pub town_share: f64,
    pub status: PlantStatus,
    /// The weakest smoothed factor of its industry's demand here (for a
    /// closed plant: as it closed), and its level (1 = neutral).
    pub factor: Factor,
    pub factor_level: f64,
    /// The upstream industry or sector, when the factor is supply.
    pub upstream: Option<ContentKey>,
    /// Its margin-adjusted demand as the smoothed factors give it, and the
    /// lines it is read against: what a rescue has to lift.
    pub demand: f64,
    pub founded: i32,
}

/// The facts of one plant.
pub fn facts(
    econ: &EconomyState,
    world: &World,
    content: &ContentBundle,
    plant: PlantId,
) -> Option<PlantFacts> {
    let pl = econ.plants.get(plant.index())?;
    let m = pl.municipality.index();
    let mu = econ.munis.get(m)?;
    let k = content.industry_index(&pl.industry);
    let st = k.and_then(|k| mu.industries.iter().find(|x| x.k as usize == k));
    let (_, inds) = town_jobs(econ, content, m);
    let jobs = inds
        .iter()
        .flat_map(|i| i.plants.iter())
        .find(|p| p.plant == plant)
        .map_or(0, |p| p.jobs);
    let total = mu.total_jobs().max(1.0);
    let (factor, level) = match (pl.status, st) {
        (PlantStatus::Closed { factor, .. }, _) => (factor, 1.0),
        (_, Some(st)) => st.weakest(),
        _ => (Factor::Cycle, 1.0),
    };
    let upstream = match (factor, k) {
        (Factor::Supply, Some(k)) => industries::weakest_link(econ, world, content, m, k),
        _ => None,
    };
    Some(PlantFacts {
        plant,
        place: pl.place,
        municipality: pl.municipality,
        neighborhood: world.places.get(pl.place.index()).map(|p| p.neighborhood),
        industry: pl.industry.clone(),
        product: pl.product.clone(),
        name: pl.name.clone(),
        jobs,
        town_share: jobs as f64 / total,
        status: pl.status,
        factor,
        factor_level: level,
        upstream,
        demand: st.map_or(1.0, |s| s.smoothed_demand())
            * (1.0 + pl.margin as f64 + pl.relief as f64),
        founded: pl.founded,
    })
}

/// A plant's name, product and industry as Fluent arguments (DEC-054: the
/// engine sends parts and keys; the client composes `$plant` from
/// `plant_key`, `stem` and `works_key`). Empty for an unknown plant.
pub fn text_args(
    econ: &EconomyState,
    content: &ContentBundle,
    plant: PlantId,
) -> Vec<(&'static str, String)> {
    let Some(pl) = econ.plants.get(plant.index()) else {
        return Vec::new();
    };
    let mut out = vec![
        ("plant_key", format!("plant-name-{}", pl.name.form.key())),
        ("stem", pl.name.stem.clone()),
    ];
    if let Some((_, x)) = content.product(&pl.product) {
        out.push(("works_key", x.works_key.clone()));
        out.push(("product_key", x.product_key.clone()));
    }
    if let Some(d) = content.industry(&pl.industry) {
        out.push(("industry_key", d.name_key.clone()));
    }
    out
}

/// The plant at a place: the open one, else the last that closed there.
pub fn at_place(econ: &EconomyState, place: PlaceId) -> Option<&Plant> {
    econ.plants
        .iter()
        .rev()
        .find(|p| p.place == place && p.is_open())
        .or_else(|| econ.plants.iter().rev().find(|p| p.place == place))
}

/// A town's plants, open and closed, in id order.
pub fn of_town(econ: &EconomyState, m: MunicipalityId) -> impl Iterator<Item = &Plant> {
    econ.plants.iter().filter(move |p| p.municipality == m)
}
