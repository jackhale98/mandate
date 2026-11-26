//! Faults for the client (DEC-114, upkeep made visible): what is wrong and
//! where ("See it": the neighbourhood's streets and the spot on them), how
//! it stands in the repair queue, and the street cue it shows now (open,
//! being repaired, new). Facts and Fluent keys only. The client draws the
//! cue on the spot (B4) and words nothing itself; nothing here counts
//! faults (DEC-113: no checklist).
//!
//! Faults are on the street for anyone to see: no knowledge gate.

use crate::places::{LocationView, location_view};
use crate::views::DateView;
use mandate_sim::Engine;
use mandate_sim::places::Location;
use mandate_sim::upkeep::{Fault, FaultId, FaultStatus};
use mandate_sim::world::NeighborhoodId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// Where on the neighbourhood's streets: a spot kind and its slot (the
/// client maps them to a cell of the street, as it does doors).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FaultSpotView {
    /// `road`, `pavement`, `kerb`, `stop`, `green`.
    pub kind: String,
    pub slot: u32,
    /// The neighbourhood's spots of this kind.
    pub slots: u32,
}

/// One fault.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FaultView {
    pub id: u32,
    /// Its kind (`fault.streetlight`).
    pub kind: String,
    /// What it is on (`street_light`, `drain`, `road`): for the street's art.
    pub asset: String,
    /// Its name mid-sentence ("the street light"), with `args`.
    pub name_key: String,
    /// What is wrong, as the street shows it, with `args`.
    pub what_key: String,
    /// `hood`; with a note, `other_key` (the other fault's name) and
    /// `other_hood`.
    pub args: BTreeMap<String, String>,
    pub neighborhood: u32,
    pub spot: FaultSpotView,
    /// `open` (waiting in the repair queue), `repairing`, `fixed`.
    pub status: String,
    /// The street cue to draw now (`faults.jsonc` `cue`): open, being
    /// repaired, or new.
    pub cue: String,
    pub opened: DateView,
    /// Being repaired: the day it will be done.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub done_by: Option<DateView>,
    /// Repaired: the day.
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_on: Option<DateView>,
    /// Moved up the repair queue (casework).
    pub moved_up: bool,
    /// Waiting, and first in its queue now.
    pub next_in_queue: bool,
    /// What the queue did to it, with `args`: `ui-fault-ahead` (fixed ahead
    /// of another fault) or `ui-fault-put-back` (another took its money).
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_key: Option<String>,
    /// "See it": the streets it is on.
    pub see: LocationView,
    /// The player is on those streets now.
    pub here: bool,
    /// Its latest event, to ask "Why?" about (`QueryDto::Why`).
    #[ts(optional)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<u32>,
}

fn hood_name(engine: &Engine, h: NeighborhoodId) -> String {
    engine
        .state()
        .world
        .neighborhoods
        .get(h.index())
        .map(|n| n.name.clone())
        .unwrap_or_default()
}

/// A fault as the client shows it.
pub fn fault_view(engine: &Engine, f: &Fault) -> FaultView {
    let st = engine.state();
    let content = engine.content();
    let def = content.fault(&f.kind);
    let text = def.map(|d| d.text.clone()).unwrap_or_default();
    let mut args = BTreeMap::new();
    args.insert("hood".to_string(), hood_name(engine, f.hood));
    let other = |id: FaultId, args: &mut BTreeMap<String, String>| {
        if let Some(o) = st.upkeep.get(id) {
            let t = content
                .fault(&o.kind)
                .map(|d| d.text.clone())
                .unwrap_or_default();
            args.insert("other_key".into(), format!("{t}name"));
            args.insert("other_hood".into(), hood_name(engine, o.hood));
            true
        } else {
            false
        }
    };
    let note_key = match (f.ahead_of, f.put_back_for) {
        (Some(o), _) if other(o, &mut args) => Some("ui-fault-ahead".to_string()),
        (None, Some(o)) if !f.fixed() && other(o, &mut args) => {
            Some("ui-fault-put-back".to_string())
        }
        _ => None,
    };
    let (status, cue) = match (f.status, def) {
        (FaultStatus::Open, Some(d)) => ("open", d.cue.open.clone()),
        (FaultStatus::Repairing { .. }, Some(d)) => ("repairing", d.cue.repairing.clone()),
        (FaultStatus::Fixed { .. }, Some(d)) => ("fixed", d.cue.new.clone()),
        (s, None) => (s.key(), String::new()),
    };
    let loc = Location::Street {
        neighborhood: f.hood,
    };
    let here = st.player.as_ref().is_some_and(|p| p.location == loc);
    FaultView {
        id: f.id.0,
        kind: f.kind.clone(),
        asset: def.map(|d| d.asset.clone()).unwrap_or_default(),
        name_key: format!("{text}name"),
        what_key: format!("{text}what"),
        args,
        neighborhood: f.hood.0,
        spot: FaultSpotView {
            kind: def.map(|d| d.spot.kind.key()).unwrap_or_default().into(),
            slot: f.spot.into(),
            slots: def.map_or(0, |d| d.spot.slots.into()),
        },
        status: status.into(),
        cue,
        opened: DateView::of(f.opened),
        done_by: match f.status {
            FaultStatus::Repairing { until, .. } => Some(DateView::of(until)),
            _ => None,
        },
        fixed_on: match f.status {
            FaultStatus::Fixed { on } => Some(DateView::of(on)),
            _ => None,
        },
        moved_up: f.expedited.is_some(),
        next_in_queue: f.open()
            && mandate_sim::upkeep::ahead_in_queue(&st.upkeep, content, &st.world, f.id).is_empty(),
        note_key,
        see: location_view(st, loc),
        here,
        event: crate::why::thread_event(
            &st.events,
            mandate_sim::spine::Thread::Fault(f.id.0),
            |_| true,
        ),
    }
}

/// The faults on a neighbourhood's streets: open, being repaired, and
/// repaired lately (still looking new), oldest first.
pub fn faults_in(engine: &Engine, hood: NeighborhoodId) -> Vec<FaultView> {
    engine
        .state()
        .upkeep
        .in_hood(hood)
        .map(|f| fault_view(engine, f))
        .collect()
}

/// The fault an ask is about, if it is about one.
pub fn fault_of_ask(engine: &Engine, a: &mandate_sim::asks::Ask) -> Option<FaultView> {
    let id = a.needs.iter().find_map(|n| match n {
        mandate_sim::asks::Need::Fixed { fault } => Some(*fault),
        _ => None,
    })?;
    engine.state().upkeep.get(id).map(|f| fault_view(engine, f))
}
