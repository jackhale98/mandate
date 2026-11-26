//! Policy families in the simulation (DEC-092; M6_ENGINE_CONTRACTS §2, task
//! F2b): each focus jurisdiction's run **deck** (a keyed draw), monthly
//! **binding** of a family's slots inside the jurisdiction, **triggers** and
//! **expiry** on the one condition language, open **instances** (proposals),
//! and the bills drafted from them.
//!
//! Everything here is keyed by [`Jurisdiction`] (DEC-098): decks, cooldowns
//! and instances. Draws are keyed by `at.tier_index()` and `at.raw()` on
//! `Stream::Policy`, so a region's deck never collides with a town's, and a
//! save loaded later draws the same deck. The system runs for the focus
//! jurisdictions only (`State::focus`, the home town in M6); it writes only
//! `policy.families` and posts knowledge effects.

use crate::characters::CharacterId;
use crate::condition::{Args, Ctx};
use crate::narrative::cast::View;
use crate::policy::{PolicyInstanceId, PolicyState};
use crate::scheduler::{Effect, Outs, Reads};
use crate::spine::{Cause, Process};
use crate::world::{Jurisdiction, NeighborhoodId, PlaceId};
use mandate_content::policy_family::{
    AreaDef, BindDef, LeadDef, LeadToDef, PersonFallback, Pick, PolicyFamilyDef, SlotKind,
};
use mandate_content::{ContentBundle, ContentKey, HazardKey, SituationTrigger};
use mandate_core::serde_util::map_as_pairs;
use mandate_core::{IdAllocator, SimDate, Stream};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use crate::player::knowledge::FamilyInstanceId;

/// Keyed-draw tags (`Stream::Policy`).
pub const TAG_DECK: u64 = 0xDEC0;
pub const TAG_BIND: u64 = 0xB14D;

/// Families in play, their open and lapsed proposals.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FamilyState {
    pub ids: IdAllocator,
    /// Families in play this run, per jurisdiction, in registry order (a
    /// keyed draw).
    #[serde(with = "map_as_pairs")]
    pub decks: BTreeMap<Jurisdiction, Vec<ContentKey>>,
    /// Indexed by `FamilyInstanceId`.
    pub instances: Vec<FamilyInstance>,
    /// When each (jurisdiction, family) last lapsed: cooldowns.
    #[serde(with = "map_as_pairs")]
    pub lapsed: BTreeMap<(Jurisdiction, ContentKey), SimDate>,
    /// Registry keys the decks were drawn with (new families are drawn on load).
    pub family_keys: Vec<ContentKey>,
}

impl FamilyState {
    pub fn get(&self, id: FamilyInstanceId) -> Option<&FamilyInstance> {
        self.instances.get(id.index())
    }
    /// Open proposals in a jurisdiction.
    pub fn open_at(&self, at: Jurisdiction) -> impl Iterator<Item = &FamilyInstance> {
        self.instances
            .iter()
            .filter(move |i| i.at == at && i.is_open())
    }
}

/// A proposal: a family bound in one jurisdiction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FamilyInstance {
    pub id: FamilyInstanceId,
    pub family: ContentKey,
    /// The family's `tier` equals `at.tier()` (checked when it opens).
    pub at: Jurisdiction,
    /// In slot order.
    pub bindings: Vec<SlotBinding>,
    /// Sponsor id → the person bound to its slot.
    pub sponsors: Vec<SponsorCast>,
    pub opened: SimDate,
    /// The last month its trigger held.
    pub trigger_held: SimDate,
    pub status: FamilyStatus,
    /// Bills drafted from it.
    pub drafts: Vec<PolicyInstanceId>,
    /// Fluent arguments of its slots as bound: names, and `<slot>_key` for
    /// sectors, crises and hazards.
    #[serde(default)]
    pub args: BTreeMap<String, String>,
}

impl FamilyInstance {
    pub fn is_open(&self) -> bool {
        self.status == FamilyStatus::Open
    }
    pub fn bound(&self, slot: &str) -> Option<&Bound> {
        self.bindings
            .iter()
            .find(|b| b.slot == slot)
            .map(|b| &b.bound)
            .filter(|b| **b != Bound::Unbound)
    }
    pub fn person(&self, slot: &str) -> Option<CharacterId> {
        match self.bound(slot)? {
            Bound::Person { person } => Some(*person),
            _ => None,
        }
    }
    pub fn hood(&self, slot: &str) -> Option<NeighborhoodId> {
        match self.bound(slot)? {
            Bound::Neighborhood { hood } => Some(*hood),
            _ => None,
        }
    }
    pub fn place(&self, slot: &str) -> Option<PlaceId> {
        match self.bound(slot)? {
            Bound::Place { place } => Some(*place),
            _ => None,
        }
    }
    /// The sponsor cast in a sponsor's slot.
    pub fn sponsor(&self, id: &str) -> Option<CharacterId> {
        self.sponsors
            .iter()
            .find(|s| s.sponsor == id)
            .map(|s| s.person)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FamilyStatus {
    Open,
    Lapsed { on: SimDate, reason: LapseReason },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LapseReason {
    /// Its expiry condition held.
    Expired,
    /// Its trigger stopped holding for `grace_months`.
    Eased,
    /// `max_months` passed.
    TooLong,
    /// The content no longer has the family (or a patch disabled it).
    Removed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotBinding {
    pub slot: String,
    pub bound: Bound,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Bound {
    Place { place: PlaceId },
    Neighborhood { hood: NeighborhoodId },
    Person { person: CharacterId },
    Sector { sector: ContentKey },
    Crisis { kind: ContentKey },
    Hazard { hazard: HazardKey },
    Unbound,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SponsorCast {
    pub sponsor: String,
    pub person: CharacterId,
}

/// What a family bill was drafted with (on `PolicyInstance.family`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyDraft {
    pub instance: FamilyInstanceId,
    /// One (axis, option) per axis, in axis order.
    pub choices: Vec<(String, String)>,
    /// The sponsor whose version was adopted, if any.
    pub sponsor: Option<String>,
    /// Axes changed away from that sponsor's preset since (amendments).
    pub changed_axes: u8,
}

impl FamilyDraft {
    pub fn choice(&self, axis: &str) -> Option<&str> {
        self.choices
            .iter()
            .find(|(a, _)| a == axis)
            .map(|(_, o)| o.as_str())
    }
}

fn key_of(id: &str) -> u64 {
    mandate_core::hash::derive_u64(id, &[])
}

fn months_between(a: SimDate, b: SimDate) -> u32 {
    a.days_until(b) / 30
}

// --- Decks ----------------------------------------------------------------------------

/// Whether a family is in a jurisdiction's deck: a keyed draw under its
/// rarity's chance.
pub fn drawn(seed: u64, content: &ContentBundle, f: &PolicyFamilyDef, at: Jurisdiction) -> bool {
    let chance = content.policy_family_params.rarity.of(f.draw.rarity);
    mandate_core::rng::keyed(
        seed,
        Stream::Policy,
        &[TAG_DECK, at.tier_index(), at.raw() as u64, key_of(&f.id)],
    )
    .chance(chance)
}

/// A jurisdiction's deck: the families of its tier drawn in, in registry order.
pub fn draw_deck(seed: u64, content: &ContentBundle, at: Jurisdiction) -> Vec<ContentKey> {
    content
        .policy_families
        .iter()
        .filter(|f| !f.disabled && f.tier == at.tier() && drawn(seed, content, f, at))
        .map(|f| f.id.clone())
        .collect()
}

/// Make sure every focus jurisdiction has a deck, and draw families the
/// content added since the decks were drawn (keyed: the same as a fresh
/// start would draw).
pub fn ensure_decks(
    fam: &mut FamilyState,
    seed: u64,
    content: &ContentBundle,
    focus: &[Jurisdiction],
) {
    if content.policy_families.is_empty() && fam.decks.is_empty() {
        return;
    }
    let keys: Vec<ContentKey> = content
        .policy_families
        .iter()
        .map(|f| f.id.clone())
        .collect();
    if fam.family_keys != keys {
        for (at, deck) in fam.decks.iter_mut() {
            let mut next: Vec<ContentKey> = Vec::new();
            for f in &content.policy_families {
                let had = deck.contains(&f.id);
                let new = !fam.family_keys.contains(&f.id);
                if had || (new && !f.disabled && f.tier == at.tier() && drawn(seed, content, f, *at)) {
                    next.push(f.id.clone());
                }
            }
            // Families the content removed stay out (their open
            // proposals lapse).
            *deck = next;
        }
        fam.family_keys = keys;
    }
    for at in focus {
        fam.decks
            .entry(*at)
            .or_insert_with(|| draw_deck(seed, content, *at));
    }
}

/// On load and at play start: decks for the focus jurisdictions, families
/// the content added drawn in, and open proposals of a family the content
/// no longer has (or a patch disabled) lapsed.
pub fn on_load(state: &mut crate::state::State, content: &ContentBundle) {
    let focus = state.focus();
    let seed = state.meta.seed;
    let today = state.calendar.date;
    let fam = &mut state.policy.families;
    ensure_decks(fam, seed, content, &focus);
    for inst in fam.instances.iter_mut() {
        if inst.is_open() && content.family(&inst.family).is_none_or(|f| f.disabled) {
            inst.status = FamilyStatus::Lapsed {
                on: today,
                reason: LapseReason::Removed,
            };
            fam.lapsed.insert((inst.at, inst.family.clone()), today);
        }
    }
}

// --- Binding ------------------------------------------------------------------------------

/// The inputs binding reads.
struct Binder<'a> {
    r: &'a Reads<'a>,
    at: Jurisdiction,
    hoods: Vec<usize>,
}

impl<'a> Binder<'a> {
    fn new(r: &'a Reads<'a>, at: Jurisdiction) -> Self {
        Binder {
            r,
            at,
            hoods: at.hoods(r.world),
        }
    }

    fn town(&self) -> Option<usize> {
        self.at.municipality().map(|m| m.index())
    }

    fn town_character(&self) -> Option<&'a crate::characters::TownCharacter> {
        self.r.politics().characters.towns.get(self.town()?)
    }

    /// Jobs and their decline for a sector in the jurisdiction (E1 baselines).
    fn sector_jobs(&self, sector: &str) -> (f64, f64) {
        let Some(k) = self.r.content.sector_index(sector) else {
            return (0.0, 0.0);
        };
        let econ = self.r.economy();
        let mut jobs = 0.0;
        let mut base = 0.0;
        let towns: Vec<usize> = match self.at {
            Jurisdiction::Municipality { id } => vec![id.index()],
            _ => {
                let mut v: Vec<usize> = self
                    .hoods
                    .iter()
                    .filter_map(|h| self.r.world.neighborhoods.get(*h))
                    .map(|n| n.municipality.index())
                    .collect();
                v.dedup();
                v
            }
        };
        for m in towns {
            if let Some(s) = econ.munis.get(m).and_then(|mu| mu.sectors.get(k)) {
                jobs += s.jobs;
                base += s.baseline_jobs as f64;
            }
        }
        (jobs, base)
    }

    /// Share of a sector's baseline jobs lost (0 when it grew).
    fn decline(&self, sector: &str) -> f64 {
        let (jobs, base) = self.sector_jobs(sector);
        if base <= 0.0 {
            0.0
        } else {
            ((base - jobs) / base).max(0.0)
        }
    }

    /// The sector of a place: its kind's drivers, the jurisdiction's largest
    /// matching sector; else its kind's `sector`.
    fn sector_of_place(&self, place: PlaceId) -> Option<ContentKey> {
        let p = self.r.world.places.get(place.index())?;
        let kind = self.r.content.place_kind(p.kind.key())?;
        if !kind.drivers.is_empty() {
            let best = self.largest_sector(&kind.drivers);
            if best.is_some() {
                return best;
            }
        }
        kind.sector.clone()
    }

    fn largest_sector(&self, drivers: &[mandate_content::SectorDriver]) -> Option<ContentKey> {
        let mut best: Option<(f64, &ContentKey)> = None;
        for s in &self.r.content.sectors {
            if !drivers.contains(&s.driver) {
                continue;
            }
            let (jobs, _) = self.sector_jobs(&s.id);
            if best.is_none_or(|b| jobs > b.0) {
                best = Some((jobs, &s.id));
            }
        }
        best.map(|(_, k)| k.clone())
    }

    fn exposure(&self, hood: usize, h: HazardKey) -> f64 {
        let Some(e) = self.r.environment().exposure.get(hood) else {
            return 0.0;
        };
        (match h {
            HazardKey::Flood => e.flood,
            HazardKey::Storm => e.storm,
            HazardKey::Wildfire => e.wildfire,
        }) as f64
            / 1000.0
    }

    fn town_exposure(&self, h: HazardKey) -> f64 {
        let Some(m) = self.at.municipality() else {
            // A region: its most exposed town (M7).
            return self
                .hoods
                .iter()
                .filter_map(|x| self.r.world.neighborhoods.get(*x))
                .filter_map(|n| self.r.world.municipalities.get(n.municipality.index()))
                .map(|mu| geo_of(&mu.geo, h))
                .fold(0.0, f64::max);
        };
        self.r
            .world
            .municipalities
            .get(m.index())
            .map(|mu| geo_of(&mu.geo, h))
            .unwrap_or(0.0)
    }

    fn hood_level(&self, hood: usize, concern: &str) -> Option<(f64, usize)> {
        let i = self.r.content.concern_index(concern)?;
        let ranking = crate::concerns::ranked(
            self.r.population(),
            self.r.content,
            self.r.world,
            crate::concerns::Scope::Neighbourhood(NeighborhoodId(hood as u32)),
            mandate_content::condition::Worry::Measured,
        );
        crate::concerns::place(&ranking, i).map(|(place, level)| (level, place))
    }

    fn residents(&self, hood: usize) -> f64 {
        self.r.population().hood_total(hood) as f64
    }

    fn person_ok(&self, c: &crate::characters::Character) -> Option<(u32, Option<PlaceId>)> {
        let n = c.named.as_deref()?;
        let home_in = self.hoods.contains(&(n.home as usize));
        let work_in = n.place.is_some_and(|p| {
            self.r
                .world
                .places
                .get(p.index())
                .is_some_and(|pl| self.at.contains_hood(self.r.world, pl.neighborhood))
        });
        (home_in || work_in).then_some((n.home, n.place))
    }

    fn executive(&self) -> Option<CharacterId> {
        let c = self
            .r
            .content
            .constitutions
            .get(&self.r.meta.constitution)?;
        let pol = self.r.politics();
        let id = crate::politics::executive_of(pol, c, self.at)?;
        pol.get(id).map(|p| p.character)
    }

    /// Bind one slot (`bound`: the slots bound so far).
    fn bind_slot(
        &self,
        f: &PolicyFamilyDef,
        slot_index: usize,
        bind: &BindDef,
        bound: &[SlotBinding],
        exclude: &[Bound],
        ordinal: u32,
    ) -> Bound {
        let get = |s: &Option<String>| -> Option<&Bound> {
            let s = s.as_deref()?;
            bound.iter().find(|b| b.slot == s).map(|b| &b.bound)
        };
        let world = self.r.world;
        let content = self.r.content;
        let keyed = |n: usize| -> usize {
            mandate_core::rng::keyed(
                self.r.meta.seed,
                Stream::Policy,
                &[
                    TAG_BIND,
                    self.at.tier_index(),
                    self.at.raw() as u64,
                    key_of(&f.id),
                    slot_index as u64,
                    ordinal as u64,
                ],
            )
            .below(n as u64) as usize
        };
        // Candidates as (id, score): the highest score wins, ties the lowest id.
        let choose = |mut c: Vec<(Bound, f64)>, pick: Pick| -> Bound {
            c.retain(|(b, _)| !exclude.contains(b));
            if c.is_empty() {
                return Bound::Unbound;
            }
            match pick {
                Pick::First => c.swap_remove(0).0,
                Pick::Keyed => {
                    let i = keyed(c.len());
                    c.swap_remove(i).0
                }
                _ => {
                    let mut best = 0;
                    for i in 1..c.len() {
                        if c[i].1 > c[best].1 {
                            best = i;
                        }
                    }
                    c.swap_remove(best).0
                }
            }
        };
        match bind {
            BindDef::Place {
                kinds,
                drivers,
                in_slot,
                open_only,
                pick,
            } => {
                let hood = match (in_slot, get(in_slot)) {
                    (Some(_), Some(Bound::Neighborhood { hood })) => Some(*hood),
                    (Some(_), _) => return Bound::Unbound,
                    _ => None,
                };
                let mut c = Vec::new();
                for p in &world.places {
                    if !kinds.iter().any(|k| k == p.kind.key())
                        || !self.at.contains_hood(world, p.neighborhood)
                        || hood.is_some_and(|h| h != p.neighborhood)
                        || (*open_only && !p.is_open())
                    {
                        continue;
                    }
                    if !drivers.is_empty() {
                        let ok = content
                            .place_kind(p.kind.key())
                            .is_some_and(|k| k.drivers.iter().any(|d| drivers.contains(d)));
                        if !ok {
                            continue;
                        }
                    }
                    let score = match pick {
                        Pick::MostAtRisk => self
                            .sector_of_place(p.id)
                            .map(|s| self.decline(&s))
                            .unwrap_or(0.0),
                        Pick::Largest => self
                            .sector_of_place(p.id)
                            .map(|s| self.sector_jobs(&s).0)
                            .unwrap_or(0.0),
                        _ => 0.0,
                    };
                    c.push((Bound::Place { place: p.id }, score));
                }
                choose(c, *pick)
            }
            BindDef::Neighborhood {
                kinds,
                of_slot,
                hazard,
                min_exposure,
                concern,
                top,
                lacks,
                pick,
            } => {
                let only = match (of_slot, get(of_slot)) {
                    (Some(_), Some(Bound::Place { place })) => {
                        world.places.get(place.index()).map(|p| p.neighborhood)
                    }
                    (Some(_), _) => return Bound::Unbound,
                    _ => None,
                };
                let mut c = Vec::new();
                for &h in &self.hoods {
                    let Some(n) = world.neighborhoods.get(h) else {
                        continue;
                    };
                    if only.is_some_and(|o| o != n.id) {
                        continue;
                    }
                    if !kinds.is_empty() && !kinds.iter().any(|k| *k == hood_kind_key(n.kind)) {
                        continue;
                    }
                    let exposure = hazard.map(|x| self.exposure(h, x)).unwrap_or(0.0);
                    if hazard.is_some() && exposure < *min_exposure {
                        continue;
                    }
                    let level = match concern {
                        Some(k) => match self.hood_level(h, k) {
                            Some((v, rank)) if rank < *top as usize => v,
                            _ => continue,
                        },
                        None => 0.0,
                    };
                    if !lacks.is_empty()
                        && world.places.iter().any(|p| {
                            p.neighborhood == n.id
                                && p.is_open()
                                && lacks.iter().any(|k| k == p.kind.key())
                        })
                    {
                        continue;
                    }
                    let score = match pick {
                        Pick::MostExposed => exposure,
                        Pick::MostConcerned => level,
                        Pick::Largest => self.residents(h),
                        _ => 0.0,
                    };
                    c.push((Bound::Neighborhood { hood: n.id }, score));
                }
                choose(c, *pick)
            }
            BindDef::Person {
                occupations,
                at_slot,
                in_slot,
                fallback,
            } => {
                let place = match (at_slot, get(at_slot)) {
                    (Some(_), Some(Bound::Place { place })) => Some(*place),
                    (Some(_), _) => None,
                    _ => None,
                };
                let hood = match (in_slot, get(in_slot)) {
                    (Some(_), Some(Bound::Neighborhood { hood })) => Some(*hood),
                    _ => None,
                };
                let needs_place = at_slot.is_some();
                let needs_hood = in_slot.is_some();
                let mut c = Vec::new();
                if !(needs_place && place.is_none() || needs_hood && hood.is_none()) {
                    for ch in &self.r.politics().characters.people {
                        let Some((home, work)) = self.person_ok(ch) else {
                            continue;
                        };
                        let n = ch.named.as_deref();
                        if !occupations.is_empty()
                            && !n.is_some_and(|n| occupations.contains(&n.occupation))
                        {
                            continue;
                        }
                        if let Some(p) = place
                            && work != Some(p)
                        {
                            continue;
                        }
                        if let Some(h) = hood {
                            let works_there = work
                                .and_then(|p| world.places.get(p.index()))
                                .is_some_and(|pl| pl.neighborhood == h);
                            if home != h.0 && !works_there {
                                continue;
                            }
                        }
                        c.push((Bound::Person { person: ch.id }, 0.0));
                    }
                }
                let b = choose(c, Pick::First);
                if b != Bound::Unbound {
                    return b;
                }
                let fb = match fallback {
                    PersonFallback::None => None,
                    PersonFallback::Broker => self.town_character().and_then(|t| t.broker),
                    PersonFallback::Executive => self.executive(),
                };
                fb.map(|person| Bound::Person { person })
                    .filter(|b| !exclude.contains(b))
                    .unwrap_or(Bound::Unbound)
            }
            BindDef::Broker => self
                .town_character()
                .and_then(|t| t.broker)
                .map(|person| Bound::Person { person })
                .unwrap_or(Bound::Unbound),
            BindDef::Sector {
                of_slot,
                drivers,
                defining,
            } => {
                let s = if let Some(slot) = of_slot {
                    match get(&Some(slot.clone())) {
                        Some(Bound::Place { place }) => self.sector_of_place(*place),
                        _ => None,
                    }
                } else if *defining {
                    self.town_character()
                        .and_then(|t| t.industry.clone())
                        .filter(|s| {
                            drivers.is_empty()
                                || content
                                    .sector(s)
                                    .is_some_and(|d| drivers.contains(&d.driver))
                        })
                } else {
                    self.largest_sector(drivers)
                };
                s.map(|sector| Bound::Sector { sector })
                    .filter(|b| !exclude.contains(b))
                    .unwrap_or(Bound::Unbound)
            }
            BindDef::Crisis { kinds } => self
                .town_character()
                .and_then(|t| {
                    t.crises
                        .iter()
                        .filter(|c| kinds.is_empty() || kinds.contains(&c.kind))
                        .max_by(|a, b| a.severity.total_cmp(&b.severity))
                })
                .map(|c| Bound::Crisis {
                    kind: c.kind.clone(),
                })
                .filter(|b| !exclude.contains(b))
                .unwrap_or(Bound::Unbound),
            BindDef::Hazard {
                hazards,
                in_slot,
                min_exposure,
                pick,
            } => {
                let hood = match (in_slot, get(in_slot)) {
                    (Some(_), Some(Bound::Neighborhood { hood })) => Some(hood.index()),
                    (Some(_), _) => return Bound::Unbound,
                    _ => None,
                };
                let mut c = Vec::new();
                for h in hazards {
                    let e = match hood {
                        Some(x) => self.exposure(x, *h),
                        None => self.town_exposure(*h),
                    };
                    if e < *min_exposure || (e <= 0.0 && *min_exposure > 0.0) {
                        continue;
                    }
                    c.push((Bound::Hazard { hazard: *h }, e));
                }
                choose(c, *pick)
            }
        }
    }

    /// Bind every slot in order. `Err(slot)`: a required slot did not bind.
    fn bind(
        &self,
        f: &PolicyFamilyDef,
        exclude_first: &[Bound],
        ordinal: u32,
    ) -> Result<Vec<SlotBinding>, String> {
        let mut out: Vec<SlotBinding> = Vec::new();
        for (i, s) in f.slots.iter().enumerate() {
            let exclude: &[Bound] = if i == 0 { exclude_first } else { &[] };
            let b = self.bind_slot(f, i, &s.bind, &out, exclude, ordinal);
            if b == Bound::Unbound && !s.optional {
                return Err(s.id.clone());
            }
            out.push(SlotBinding {
                slot: s.id.clone(),
                bound: b,
            });
        }
        Ok(out)
    }
}

fn geo_of(g: &crate::world::GeoProfile, h: HazardKey) -> f64 {
    match h {
        HazardKey::Flood => g.flood,
        HazardKey::Storm => g.storm,
        HazardKey::Wildfire => g.wildfire,
    }
}

/// The content's spelling of a neighbourhood kind.
pub fn hood_kind_key(k: crate::world::NeighborhoodKind) -> &'static str {
    use crate::world::NeighborhoodKind as K;
    match k {
        K::Core => "core",
        K::Inner => "inner",
        K::Waterfront => "waterfront",
        K::Industrial => "industrial",
        K::Suburb => "suburb",
        K::Rural => "rural",
    }
}

/// Fluent arguments of bindings: names, and keys for sectors, crises and
/// hazards (`<slot>_key`).
pub fn binding_args(r: &Reads<'_>, bindings: &[SlotBinding]) -> BTreeMap<String, String> {
    let mut a = BTreeMap::new();
    for b in bindings {
        match &b.bound {
            Bound::Place { place } => {
                if let Some(p) = r.world.places.get(place.index()) {
                    a.insert(b.slot.clone(), p.name.clone());
                }
            }
            Bound::Neighborhood { hood } => {
                if let Some(n) = r.world.neighborhoods.get(hood.index()) {
                    a.insert(b.slot.clone(), n.name.clone());
                }
            }
            Bound::Person { person } => {
                if let Some(p) = crate::characters::profile(r.politics(), r.world, *person) {
                    a.insert(b.slot.clone(), p.full_name());
                }
            }
            Bound::Sector { sector } => {
                if let Some(s) = r.content.sector(sector) {
                    a.insert(format!("{}_key", b.slot), s.name_key.clone());
                }
            }
            Bound::Crisis { kind } => {
                if let Some(c) = r.content.crises.iter().find(|c| &c.id == kind) {
                    a.insert(format!("{}_key", b.slot), c.name_key.clone());
                }
            }
            Bound::Hazard { hazard } => {
                a.insert(
                    format!("{}_key", b.slot),
                    format!("hazard-text-{}", hazard_word(*hazard)),
                );
            }
            Bound::Unbound => {}
        }
    }
    a
}

pub fn hazard_word(h: HazardKey) -> &'static str {
    match h {
        HazardKey::Flood => "flood",
        HazardKey::Storm => "storm",
        HazardKey::Wildfire => "wildfire",
    }
}

// --- Triggers ------------------------------------------------------------------------------

/// Does a situation trigger hold for these bindings in `at`? Every `all`
/// condition, and one `any` condition when any are given.
pub fn trigger_holds(
    v: &View<'_>,
    events: Option<&crate::events::EventLog>,
    t: &SituationTrigger,
    at: Jurisdiction,
    bindings: &[SlotBinding],
) -> bool {
    let mut cx = Ctx::new(v, None, Some(at));
    cx.family = Some(bindings);
    cx.events = events;
    let mut args = Args::new();
    t.all
        .iter()
        .all(|c| crate::condition::holds(&cx, c, &mut args))
        && (t.any.is_empty()
            || t.any
                .iter()
                .any(|c| crate::condition::holds(&cx, c, &mut args)))
}

// --- The monthly system ------------------------------------------------------------------

/// Monthly (Characters phase, after `characters.monthly`): decks for the
/// focus jurisdictions, lapses, and new proposals. Writes only
/// `policy.families`; reads the policy domain itself through a view of it.
pub fn monthly(st: &mut PolicyState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let mut fam = std::mem::take(&mut st.families);
    {
        let r = r.with_policy(st);
        run(&mut fam, &r, st, out);
    }
    st.families = fam;
}

fn run(fam: &mut FamilyState, r: &Reads<'_>, st: &PolicyState, out: &mut Outs<'_>) {
    let content = r.content;
    if content.policy_families.is_empty() && fam.instances.is_empty() {
        return;
    }
    let focus: Vec<Jurisdiction> = r.player.map(|p| p.focus()).unwrap_or_default();
    ensure_decks(fam, r.meta.seed, content, &focus);
    let pol = r.politics();
    let v = View::new(r, pol);
    let today = r.date;
    // Lapses (every jurisdiction with open proposals).
    for i in 0..fam.instances.len() {
        if !fam.instances[i].is_open() {
            continue;
        }
        let inst = &fam.instances[i];
        let reason = match content.family(&inst.family).filter(|f| !f.disabled) {
            None => Some(LapseReason::Removed),
            Some(f) => {
                // A drafted bill or a passed law stays when the situation eases.
                let live = inst.drafts.iter().any(|d| {
                    st.get(*d)
                        .is_some_and(|p| p.status != crate::policy::PolicyStatus::Ended)
                });
                let held = f.trigger.as_ref().is_none_or(|t| {
                    trigger_holds(&v, Some(&*out.events), t, inst.at, &inst.bindings)
                });
                let expired = f.expiry.when.as_ref().is_some_and(|t| {
                    trigger_holds(&v, Some(&*out.events), t, inst.at, &inst.bindings)
                });
                if held {
                    fam.instances[i].trigger_held = today;
                }
                let inst = &fam.instances[i];
                if live {
                    None
                } else if expired {
                    Some(LapseReason::Expired)
                } else if months_between(inst.trigger_held, today) >= f.expiry.grace_months as u32 {
                    Some(LapseReason::Eased)
                } else if f
                    .expiry
                    .max_months
                    .is_some_and(|m| months_between(inst.opened, today) >= m as u32)
                {
                    Some(LapseReason::TooLong)
                } else {
                    None
                }
            }
        };
        if let Some(reason) = reason {
            let inst = &mut fam.instances[i];
            inst.status = FamilyStatus::Lapsed { on: today, reason };
            fam.lapsed.insert((inst.at, inst.family.clone()), today);
            out.events.record(
                today,
                crate::events::EventKind::ProposalLapsed {
                    proposal: inst.id.0,
                    family: inst.family.clone(),
                    at: inst.at,
                },
                Cause::Process(Process::Agenda),
            );
        }
    }
    // New proposals, per focus jurisdiction.
    let params = &content.policy_family_params;
    for at in focus {
        let deck = fam.decks.get(&at).cloned().unwrap_or_default();
        let binder = Binder::new(r, at);
        let mut open_now = fam.open_at(at).count();
        let mut ready: Vec<(f64, usize, &PolicyFamilyDef, Vec<SlotBinding>)> = Vec::new();
        for (reg, f) in content.policy_families.iter().enumerate() {
            if f.disabled || f.tier != at.tier() || !deck.contains(&f.id) {
                continue;
            }
            if let Some(on) = fam.lapsed.get(&(at, f.id.clone()))
                && months_between(*on, today) < f.expiry.cooldown_months as u32
            {
                continue;
            }
            let mine: Vec<&FamilyInstance> = fam.open_at(at).filter(|i| i.family == f.id).collect();
            if mine.len() >= f.max_instances as usize {
                continue;
            }
            // A second proposal of a family is about something else.
            let exclude: Vec<Bound> = mine
                .iter()
                .filter_map(|i| i.bindings.first().map(|b| b.bound.clone()))
                .collect();
            let ordinal = fam
                .instances
                .iter()
                .filter(|i| i.at == at && i.family == f.id)
                .count() as u32;
            let Ok(bindings) = binder.bind(f, &exclude, ordinal) else {
                continue;
            };
            let holds = f
                .trigger
                .as_ref()
                .is_none_or(|t| trigger_holds(&v, Some(&*out.events), t, at, &bindings));
            if holds {
                ready.push((f.draw.weight, reg, f, bindings));
            }
        }
        ready.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        for (_, _, f, bindings) in ready {
            if open_now >= params.max_open_per_jurisdiction as usize {
                break;
            }
            let id: FamilyInstanceId = fam.ids.alloc();
            let sponsors =
                f.sponsors
                    .iter()
                    .filter_map(|s| {
                        let person = bindings.iter().find(|b| b.slot == s.slot).and_then(|b| {
                            match b.bound {
                                Bound::Person { person } => Some(person),
                                _ => None,
                            }
                        })?;
                        Some(SponsorCast {
                            sponsor: s.id.clone(),
                            person,
                        })
                    })
                    .collect();
            let args = binding_args(r, &bindings);
            let inst = FamilyInstance {
                id,
                family: f.id.clone(),
                at,
                bindings,
                sponsors,
                opened: today,
                trigger_held: today,
                status: FamilyStatus::Open,
                drafts: Vec::new(),
                args,
            };
            // The family's trigger held here (the proposal names what it
            // is bound to).
            let ev = out.events.record(
                today,
                crate::events::EventKind::ProposalOpened {
                    proposal: id.0,
                    family: f.id.clone(),
                    at,
                },
                Cause::Process(Process::Agenda),
            );
            announce(r, f, &inst, out, Cause::caused_by(ev));
            fam.instances.push(inst);
            open_now += 1;
        }
    }
}

/// A new proposal's knowledge (DEC-091): a public one is heard by all; the
/// lead is posted (to the bound person or place, else its `else`).
fn announce(
    r: &Reads<'_>,
    f: &PolicyFamilyDef,
    inst: &FamilyInstance,
    out: &mut Outs<'_>,
    cause: Cause,
) {
    use crate::player::knowledge::{Lead, LeadTarget, Level, Source, Subject};
    if r.player.is_none() {
        return;
    }
    let subject = Subject::Proposal { proposal: inst.id };
    if f.knowledge.public {
        out.effects.push(
            Effect::Learn {
                subject: subject.clone(),
                level: Level::Heard,
                source: Source::Proposal { proposal: inst.id },
            },
            cause,
        );
    }
    let mut lead: Option<&LeadDef> = f.knowledge.lead.as_ref();
    while let Some(l) = lead {
        let to = match &l.to {
            LeadToDef::Slot { slot } => match inst.bound(slot) {
                Some(Bound::Person { person }) => Some(LeadTarget::Person { person: *person }),
                Some(Bound::Place { place }) => Some(LeadTarget::Place { place: *place }),
                Some(Bound::Neighborhood { hood }) => Some(LeadTarget::Streets { hood: *hood }),
                _ => None,
            },
            LeadToDef::Streets { slot } => inst.hood(slot).map(|hood| LeadTarget::Streets { hood }),
            LeadToDef::PlaceKind { kind } => {
                inst.at.municipality().map(|town| LeadTarget::PlaceKind {
                    town,
                    kind: kind.clone(),
                })
            }
        };
        if let Some(to) = to {
            out.effects.push(
                Effect::Lead {
                    lead: Lead {
                        subject: subject.clone(),
                        level: Level::Known,
                        key: l.key.clone(),
                        to,
                        since: r.date,
                        source: Source::Proposal { proposal: inst.id },
                    },
                },
                cause,
            );
            return;
        }
        lead = l.r#else.as_deref();
    }
}

/// Would a family open in a jurisdiction now (the lab, tests and F5's
/// coverage)? Its bindings, or why not: `slot <id>` (a required slot did not
/// bind), `trigger`, or `tier` (the family is of another tier). Any
/// jurisdiction, whatever the focus; read-only.
pub fn probe(
    state: &crate::state::State,
    content: &ContentBundle,
    f: &PolicyFamilyDef,
    at: Jurisdiction,
) -> Result<Vec<SlotBinding>, String> {
    if f.tier != at.tier() {
        return Err("tier".into());
    }
    let r = Reads::of_state(state, content);
    let binder = Binder::new(&r, at);
    let bindings = binder.bind(f, &[], 0).map_err(|s| format!("slot {s}"))?;
    let v = View::new(&r, &state.politics);
    let holds = f
        .trigger
        .as_ref()
        .is_none_or(|t| trigger_holds(&v, Some(&state.events), t, at, &bindings));
    if holds {
        Ok(bindings)
    } else {
        Err("trigger".into())
    }
}

// --- Area -------------------------------------------------------------------------------

/// The neighbourhoods an option's area covers for an instance (`None`: the
/// whole jurisdiction).
pub fn area_hoods(
    world: &crate::world::World,
    inst: &FamilyInstance,
    area: &AreaDef,
) -> Option<Vec<NeighborhoodId>> {
    match area {
        AreaDef::Town => None,
        AreaDef::Slot { slot } => Some(inst.hood(slot).into_iter().collect()),
        AreaDef::SlotAndAdjacent { slot } => {
            let h = inst.hood(slot)?;
            let mut v = vec![h];
            if let Some(n) = world.neighborhoods.get(h.index()) {
                for x in &n.neighbors {
                    if inst.at.contains_hood(world, *x) && !v.contains(x) {
                        v.push(*x);
                    }
                }
            }
            v.sort();
            Some(v)
        }
    }
}

/// Which slot kind an argument of a family's keys names (for views).
pub fn slot_kind(f: &PolicyFamilyDef, slot: &str) -> Option<SlotKind> {
    f.slot(slot).map(|(_, s)| s.bind.kind())
}
