//! Upkeep made visible (DEC-114 §3, `BEING_THERE.md` §4): the `upkeep`
//! domain.
//!
//! Each neighbourhood of the player's focus has a **condition** (0–1). It
//! falls each month and routine maintenance keeps it up in proportion to the
//! town's Infrastructure line against need, so a budget cut lets it run
//! down. Where it is low, **faults** open at spots on its streets (a light
//! out, a blocked drain, a pothole), at most `max_open` at once.
//!
//! Each month the **repair queue** of each budget line that pays for faults
//! is funded with `repair_share` of that line's month in the town's own
//! treasury: faults are paid for in queue order (those moved up first, then
//! the oldest) until the next one does not fit; a paid fault is repaired
//! after its `days`, and the street looks new for `new_days`. No money moves:
//! the line is spent by finance as ever, and this is what part of it buys.
//! When the money runs out, nothing more is fixed until next month or a
//! budget change.
//!
//! **"Sort it out" moves a fault up the queue** (the owner's answer 3): it
//! does not create money. A fault moved up and paid for in a month it would
//! not have been takes the money of a fault that would have been; that one is
//! put back, and both say so ("Fixed ahead of …", "Put back for …"), on the
//! spine: the player's casework, the fault moved up, its repair, the fault
//! put back.
//!
//! The domain owns [`UpkeepState`] and writes nothing else. Moving a fault up
//! is asked for by the asks' casework command as an effect request
//! ([`crate::scheduler::Effect::Expedite`]). The services worry and the asks
//! read faults; the protocol shows them. Focus towns only (as the calendar):
//! with no fault kinds in the content, or no player, the domain stays empty
//! and the state hashes as it did before upkeep existed.
//!
//! Tier-generic (DEC-098): a fault is a physical thing in a town, so it is
//! keyed by its neighbourhood; the line that pays is the town's own
//! treasury's (`finance::treasury` of its jurisdiction).

#[cfg(test)]
mod tests;

use crate::events::{EventId, EventKind, EventLog};
use crate::scheduler::{Effect, Outs, Reads};
use crate::spine::{Cause, Causes, Link, Process};
use crate::world::{Jurisdiction, MunicipalityId, NeighborhoodId};
use mandate_content::faults::FaultDef;
use mandate_content::{BudgetCategory, ContentBundle, ContentKey};
use mandate_core::{IdAllocator, SimDate, Stream};
use serde::{Deserialize, Serialize};

mandate_core::define_id!(
    /// A fault; typed `u32`, never reused (DEC-041).
    FaultId
);

/// Keyed-draw tags (`Stream::Policy`, the works' stream: upkeep is the
/// town's physical fabric).
const KEY_START: u64 = 0xFA17_0001;
const KEY_RAISE: u64 = 0xFA17_0002;
const KEY_SPOT: u64 = 0xFA17_0003;

/// The upkeep domain's state (`State.upkeep`; save v33). Not written while
/// empty.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UpkeepState {
    /// The focus's neighbourhoods' condition, sorted by neighbourhood.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hoods: Vec<HoodUpkeep>,
    /// Faults open, being repaired, or repaired within `new_days`, in id
    /// order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub faults: Vec<Fault>,
    #[serde(default)]
    pub ids: IdAllocator,
}

impl UpkeepState {
    /// Nothing kept: never written (a game with no fault content hashes as
    /// it did before upkeep).
    pub fn is_empty(&self) -> bool {
        self.hoods.is_empty() && self.faults.is_empty() && self.ids.peek_next() == 0
    }

    pub fn get(&self, id: FaultId) -> Option<&Fault> {
        self.faults.iter().find(|f| f.id == id)
    }

    fn get_mut(&mut self, id: FaultId) -> Option<&mut Fault> {
        self.faults.iter_mut().find(|f| f.id == id)
    }

    /// A neighbourhood's condition, if kept.
    pub fn condition(&self, hood: NeighborhoodId) -> Option<f32> {
        self.hoods
            .binary_search_by_key(&hood, |h| h.hood)
            .ok()
            .map(|i| self.hoods[i].condition)
    }

    /// Faults on a neighbourhood's streets (any status), in id order.
    pub fn in_hood(&self, hood: NeighborhoodId) -> impl Iterator<Item = &Fault> {
        self.faults.iter().filter(move |f| f.hood == hood)
    }

    /// Faults not yet repaired in a neighbourhood that feed this worry.
    pub fn open_feeding(
        &self,
        content: &ContentBundle,
        hood: NeighborhoodId,
        concern: &str,
    ) -> u32 {
        self.in_hood(hood)
            .filter(|f| !f.fixed())
            .filter(|f| content.fault(&f.kind).is_some_and(|d| d.feeds == concern))
            .count() as u32
    }
}

/// One neighbourhood's condition.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HoodUpkeep {
    pub hood: NeighborhoodId,
    pub condition: f32,
}

/// A fault at a spot on a neighbourhood's streets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fault {
    pub id: FaultId,
    /// Its kind (`fault.<name>`).
    pub kind: ContentKey,
    pub hood: NeighborhoodId,
    /// Its slot among the neighbourhood's spots of the kind's spot kind.
    pub spot: u8,
    pub opened: SimDate,
    pub status: FaultStatus,
    /// Moved up the repair queue (casework), on this day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expedited: Option<SimDate>,
    /// The fault it was repaired ahead of (whose money it took).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ahead_of: Option<FaultId>,
    /// The fault repaired ahead of it, the last time it was put back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub put_back_for: Option<FaultId>,
}

impl Fault {
    pub fn fixed(&self) -> bool {
        matches!(self.status, FaultStatus::Fixed { .. })
    }

    pub fn open(&self) -> bool {
        self.status == FaultStatus::Open
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FaultStatus {
    /// Waiting in the repair queue.
    Open,
    /// Paid for: the crew is on it until `until`.
    Repairing {
        since: SimDate,
        until: SimDate,
    },
    Fixed {
        on: SimDate,
    },
}

impl FaultStatus {
    /// `open`, `repairing`, `fixed`.
    pub fn key(&self) -> &'static str {
        match self {
            FaultStatus::Open => "open",
            FaultStatus::Repairing { .. } => "repairing",
            FaultStatus::Fixed { .. } => "fixed",
        }
    }
}

/// What happened to a fault (`EventKind::Fault`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultStage {
    Opened,
    /// Moved up the repair queue.
    Expedited,
    /// Paid for and being repaired (`other`: the fault it went ahead of).
    Repairing,
    /// Its money went to a fault moved up the queue (`other`).
    Delayed,
    Fixed,
}

impl FaultStage {
    pub fn key(self) -> &'static str {
        match self {
            FaultStage::Opened => "opened",
            FaultStage::Expedited => "expedited",
            FaultStage::Repairing => "repairing",
            FaultStage::Delayed => "delayed",
            FaultStage::Fixed => "fixed",
        }
    }
}

fn record(
    events: &mut EventLog,
    date: SimDate,
    f: &Fault,
    stage: FaultStage,
    other: Option<FaultId>,
    causes: impl Into<Causes>,
) -> EventId {
    events.record(
        date,
        EventKind::Fault {
            fault: f.id.0,
            template: f.kind.clone(),
            hood: f.hood.0,
            stage,
            other: other.map(|o| o.0),
        },
        causes,
    )
}

/// The towns of the player's focus.
fn focus_towns(r: &Reads<'_>) -> Vec<MunicipalityId> {
    r.player
        .map(|p| p.focus())
        .unwrap_or_default()
        .into_iter()
        .filter_map(Jurisdiction::municipality)
        .collect()
}

fn in_winter(d: SimDate) -> bool {
    matches!(d.calendar().month, 12 | 1 | 2)
}

/// A budget line's yearly amount in a town's treasury and its need (its
/// share of the starting budget indexed to prices × productivity and the
/// nation's population, as the services worry's budget index reads it).
fn line(r: &Reads<'_>, m: MunicipalityId, c: BudgetCategory) -> Option<(f64, f64)> {
    let fin = r.finance();
    let t = crate::finance::treasury(fin, Jurisdiction::town(m))?;
    let i = crate::policy::category_index(c);
    let have = t.budget.get(i)?.units() as f64;
    let shares = &r.content.economy.treasury_shares.local;
    let total: f64 = shares.iter().sum();
    let share = shares.get(i).copied().unwrap_or(0.0) / total.max(1e-9);
    let econ = r.economy();
    let wage_index = (econ.national.price_level * econ.national.productivity_index).max(0.1);
    let pop_index = r
        .population_opt()
        .map(|p| (p.total() as f64 / fin.base_population.max(1.0)).max(0.1))
        .unwrap_or(1.0);
    let need = t.base_budget_total.units() as f64 * share * wage_index * pop_index;
    Some((have, need))
}

/// What a fault's repair costs now.
pub fn repair_cost(r: &Reads<'_>, d: &FaultDef) -> f64 {
    d.repair.cost * r.economy().national.price_level.max(0.1)
}

/// The money a town's queue for a budget line has this month.
pub fn monthly_fund(r: &Reads<'_>, m: MunicipalityId, c: BudgetCategory) -> f64 {
    line(r, m, c).map_or(0.0, |(have, _)| {
        have.max(0.0) / 12.0 * r.content.upkeep.params.repair_share
    })
}

/// The queue order: moved up first (by the day), then the oldest.
fn queue_key(f: &Fault, moved_up: bool) -> (bool, SimDate, SimDate, FaultId) {
    match (moved_up, f.expedited) {
        (true, Some(on)) => (false, on, f.opened, f.id),
        _ => (true, SimDate(0), f.opened, f.id),
    }
}

/// The faults of a queue paid for with `fund`, in order, up to the first
/// that does not fit. A repair dearer than a whole month's money takes the
/// month's money when it heads the queue (a big job is done in its turn,
/// never left at the head for ever).
fn paid(queue: &[(FaultId, f64)], fund: f64) -> Vec<FaultId> {
    let mut left = fund;
    let mut out = Vec::new();
    for (id, cost) in queue {
        if *cost > left {
            if out.is_empty() && fund > 0.0 && cost.is_finite() {
                out.push(*id);
            }
            break;
        }
        left -= cost;
        out.push(*id);
    }
    out
}

/// Upkeep domain, Population phase, monthly: pay the repair queues, then
/// wear the streets and open new faults where they are worn.
pub fn monthly(u: &mut UpkeepState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let content = r.content;
    if content.upkeep.faults.is_empty() {
        return;
    }
    let towns = focus_towns(r);
    // A town no longer in focus (the player moved) is forgotten.
    let hood_town =
        |h: NeighborhoodId| r.world.neighborhoods.get(h.index()).map(|n| n.municipality);
    u.faults
        .retain(|f| hood_town(f.hood).is_some_and(|m| towns.contains(&m)));
    u.hoods
        .retain(|h| hood_town(h.hood).is_some_and(|m| towns.contains(&m)));
    for m in towns {
        fund_queues(u, r, out, m);
        wear(u, r, m);
        raise(u, r, out, m);
    }
}

/// Pay each budget line's queue in a town (see the module note).
fn fund_queues(u: &mut UpkeepState, r: &Reads<'_>, out: &mut Outs<'_>, m: MunicipalityId) {
    let content = r.content;
    let date = r.date;
    let mut lines: Vec<BudgetCategory> = content.upkeep.faults.iter().map(|d| d.pays).collect();
    lines.sort();
    lines.dedup();
    for c in lines {
        let fund = monthly_fund(r, m, c);
        let mut waiting: Vec<&Fault> = u
            .faults
            .iter()
            .filter(|f| f.open())
            .filter(|f| {
                r.world
                    .neighborhoods
                    .get(f.hood.index())
                    .is_some_and(|n| n.municipality == m)
            })
            .filter(|f| content.fault(&f.kind).is_some_and(|d| d.pays == c))
            .collect();
        if waiting.is_empty() {
            continue;
        }
        let cost = |f: &Fault| {
            content
                .fault(&f.kind)
                .map_or(f64::INFINITY, |d| repair_cost(r, d))
        };
        waiting.sort_by_key(|f| queue_key(f, true));
        let actual: Vec<(FaultId, f64)> = waiting.iter().map(|f| (f.id, cost(f))).collect();
        waiting.sort_by_key(|f| queue_key(f, false));
        let natural: Vec<(FaultId, f64)> = waiting.iter().map(|f| (f.id, cost(f))).collect();
        let now = paid(&actual, fund);
        let would = paid(&natural, fund);
        // Faults moved up that took the money of faults that would have
        // been paid for: paired in queue order (the last one moved up
        // answers for any more it displaced).
        let jumpers: Vec<FaultId> = now
            .iter()
            .copied()
            .filter(|id| !would.contains(id))
            .collect();
        let displaced: Vec<FaultId> = would
            .iter()
            .copied()
            .filter(|id| !now.contains(id))
            .collect();
        let pair = |i: usize| jumpers.get(i.min(jumpers.len().saturating_sub(1))).copied();
        let mut ahead: Vec<(FaultId, FaultId)> = Vec::new();
        for (i, d) in displaced.iter().enumerate() {
            if let Some(j) = pair(i) {
                ahead.push((j, *d));
            }
        }
        let mut started: Vec<(FaultId, EventId)> = Vec::new();
        for id in &now {
            let Some(days) = u
                .get(*id)
                .and_then(|f| content.fault(&f.kind))
                .map(|d| d.repair.days)
            else {
                continue;
            };
            let other = ahead.iter().find(|(j, _)| j == id).map(|(_, d)| *d);
            let Some(f) = u.get_mut(*id) else { continue };
            f.status = FaultStatus::Repairing {
                since: date,
                until: date.add_days(days),
            };
            if other.is_some() {
                f.ahead_of = other;
            }
            let f = f.clone();
            // The budget line paid for it (the thread links its opening,
            // and the casework that moved it up).
            let ev = record(
                out.events,
                date,
                &f,
                FaultStage::Repairing,
                other,
                Cause::Process(Process::Budget),
            );
            started.push((*id, ev));
        }
        for (j, d) in ahead {
            let by = started.iter().find(|(id, _)| *id == j).map(|(_, e)| *e);
            let Some(f) = u.get_mut(d) else { continue };
            f.put_back_for = Some(j);
            let f = f.clone();
            record(
                out.events,
                date,
                &f,
                FaultStage::Delayed,
                Some(j),
                Causes::from(Cause::Process(Process::Budget)).and_event(by, Link::Caused),
            );
        }
    }
}

/// A month's wear and maintenance in a town's neighbourhoods.
fn wear(u: &mut UpkeepState, r: &Reads<'_>, m: MunicipalityId) {
    let p = &r.content.upkeep.params;
    let Some(mu) = r.world.municipalities.get(m.index()) else {
        return;
    };
    let index = line(r, m, BudgetCategory::Infrastructure)
        .map_or(1.0, |(have, need)| have.max(0.0) / need.max(1.0))
        .min(p.maintain_cap.max(0.0));
    for &h in &mu.neighborhoods {
        let i = match u.hoods.binary_search_by_key(&h, |x| x.hood) {
            Ok(i) => i,
            Err(i) => {
                let [lo, hi] = p.start;
                let c = lo
                    + (hi - lo)
                        * mandate_core::rng::keyed(
                            r.meta.seed,
                            Stream::Policy,
                            &[KEY_START, u64::from(h.0)],
                        )
                        .next_f64();
                u.hoods.insert(
                    i,
                    HoodUpkeep {
                        hood: h,
                        condition: c as f32,
                    },
                );
                i
            }
        };
        let c = f64::from(u.hoods[i].condition) + p.maintain * index - p.decay;
        u.hoods[i].condition = c.clamp(0.0, 1.0) as f32;
    }
}

/// Open faults where the streets are worn (one keyed draw per
/// neighbourhood, kind and month).
fn raise(u: &mut UpkeepState, r: &Reads<'_>, out: &mut Outs<'_>, m: MunicipalityId) {
    let content = r.content;
    let p = &content.upkeep.params;
    let date = r.date;
    let Some(mu) = r.world.municipalities.get(m.index()) else {
        return;
    };
    for &h in &mu.neighborhoods {
        let Some(cond) = u.condition(h).map(f64::from) else {
            continue;
        };
        for d in &content.upkeep.faults {
            let open = u.in_hood(h).filter(|f| !f.fixed()).count();
            if open >= p.max_open as usize {
                break;
            }
            let below = d.raise.below.max(1e-9);
            let mut chance = d.raise.chance * ((below - cond) / below).clamp(0.0, 1.0);
            if in_winter(date) {
                chance *= d.raise.winter;
            }
            if chance <= 0.0 {
                continue;
            }
            let kind = mandate_core::hash::derive_u64(&d.id, &[]);
            let hit = mandate_core::rng::keyed(
                r.meta.seed,
                Stream::Policy,
                &[KEY_RAISE, u64::from(date.0), u64::from(h.0), kind],
            )
            .chance(chance.min(1.0));
            if !hit {
                continue;
            }
            // A free spot of its kind, from a keyed start.
            let slots = d.spot.slots.max(1);
            let taken: Vec<u8> = u
                .in_hood(h)
                .filter(|f| !f.fixed())
                .filter(|f| {
                    content
                        .fault(&f.kind)
                        .is_some_and(|x| x.spot.kind == d.spot.kind)
                })
                .map(|f| f.spot)
                .collect();
            let start = mandate_core::rng::keyed(
                r.meta.seed,
                Stream::Policy,
                &[KEY_SPOT, u64::from(date.0), u64::from(h.0), kind],
            )
            .below(u64::from(slots)) as u8;
            let Some(spot) = (0..slots)
                .map(|k| (start + k) % slots)
                .find(|s| !taken.contains(s))
            else {
                continue;
            };
            let f = Fault {
                id: u.ids.alloc(),
                kind: d.id.clone(),
                hood: h,
                spot,
                opened: date,
                status: FaultStatus::Open,
                expedited: None,
                ahead_of: None,
                put_back_for: None,
            };
            record(
                out.events,
                date,
                &f,
                FaultStage::Opened,
                None,
                Cause::Process(Process::Upkeep),
            );
            u.faults.push(f);
        }
    }
}

/// Upkeep domain, Population phase, daily: faults moved up the queue
/// (`Effect::Expedite`), repairs finished, and repaired faults forgotten
/// once they no longer look new.
pub fn daily(u: &mut UpkeepState, r: &Reads<'_>, out: &mut Outs<'_>) {
    for q in out.take(|e| matches!(e, Effect::Expedite { .. })) {
        if let Effect::Expedite { fault } = q.effect {
            expedite(u, out.events, r.date, FaultId(fault), q.cause);
        }
    }
    if u.faults.is_empty() {
        return;
    }
    finish(u, r.content, out.events, r.date);
}

/// Repairs due by `date` are done: the spot looks new, the neighbourhood's
/// condition rises by the kind's `restore`.
fn finish(u: &mut UpkeepState, content: &ContentBundle, events: &mut EventLog, date: SimDate) {
    let mut done: Vec<(FaultId, NeighborhoodId, f64)> = Vec::new();
    for f in u.faults.iter_mut() {
        if let FaultStatus::Repairing { until, .. } = f.status
            && until <= date
        {
            f.status = FaultStatus::Fixed { on: date };
            let restore = content.fault(&f.kind).map_or(0.0, |d| d.restore);
            done.push((f.id, f.hood, restore));
        }
    }
    for (id, hood, restore) in done {
        if let Ok(i) = u.hoods.binary_search_by_key(&hood, |h| h.hood) {
            let c = f64::from(u.hoods[i].condition) + restore;
            u.hoods[i].condition = c.clamp(0.0, 1.0) as f32;
        }
        if let Some(f) = u.get(id).cloned() {
            record(
                events,
                date,
                &f,
                FaultStage::Fixed,
                None,
                Cause::Process(Process::Upkeep),
            );
        }
    }
    let keep = content.upkeep.params.new_days;
    u.faults.retain(|f| match f.status {
        FaultStatus::Fixed { on } => on.days_until(date) <= keep,
        _ => true,
    });
}

/// Move a fault up the repair queue (the applier of `Effect::Expedite`;
/// the asks' casework command applies it at once through here). Only a
/// fault waiting in the queue moves, once; it is paid for at the next
/// month's turn of the queue, ahead of the others, if the money allows.
pub fn expedite(
    u: &mut UpkeepState,
    events: &mut EventLog,
    date: SimDate,
    fault: FaultId,
    cause: Cause,
) -> bool {
    let Some(f) = u.get_mut(fault) else {
        return false;
    };
    if !f.open() || f.expedited.is_some() {
        return false;
    }
    f.expedited = Some(date);
    let f = f.clone();
    record(events, date, &f, FaultStage::Expedited, None, cause);
    true
}

/// The faults of a queue ahead of this one now: those that would be paid
/// for before it (moved up first, then the oldest), in order. Empty for a
/// fault not waiting.
pub fn ahead_in_queue(
    u: &UpkeepState,
    content: &ContentBundle,
    world: &crate::world::World,
    id: FaultId,
) -> Vec<FaultId> {
    let Some(f) = u.get(id).filter(|f| f.open()) else {
        return Vec::new();
    };
    let town = |h: NeighborhoodId| world.neighborhoods.get(h.index()).map(|n| n.municipality);
    let pays = content.fault(&f.kind).map(|d| d.pays);
    let mut q: Vec<&Fault> = u
        .faults
        .iter()
        .filter(|x| x.open() && town(x.hood) == town(f.hood))
        .filter(|x| content.fault(&x.kind).map(|d| d.pays) == pays)
        .collect();
    q.sort_by_key(|x| queue_key(x, true));
    q.iter().take_while(|x| x.id != id).map(|x| x.id).collect()
}
