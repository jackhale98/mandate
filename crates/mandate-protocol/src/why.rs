//! "Why did this happen?" (S1, the causal spine): the chain behind an event
//! as facts and keys. The event first, then what led to it, nearest first:
//! each step is an event with the engine's own text key and arguments, the
//! typed link that led on from it, and the causes it names (links to
//! earlier events and roots: the player's act, someone's own choice, a
//! scheduled process, a named pressure, or an explicit exogenous mark).
//!
//! Read-only. The client words nothing itself and computes nothing: it
//! lays the steps out. What the player may not see is left out here:
//!
//! - The director's levers and the days hidden facts become discoverable
//!   are diagnostics, never shown (as in the overview's event list). The
//!   chain passes through them: what they name is given to the step they
//!   led to, and the director's own hand in them is dropped.
//! - People are named only as the player knows them (the knowledge gate,
//!   DEC-091): someone's own choice as a root, and any name in a step's
//!   text arguments. The query takes any event id, so it must not be a way
//!   to read every name in the log. Every other kind of event is public
//!   record (laws, races, closures, disasters, scandals once they break,
//!   cases) or the player's own (their deeds, asks, decisions, warnings).
//! - A public result is public (DEC-108 §8): whoever a step of one of the
//!   [`PUBLIC_RESULTS`] kinds names is named here whatever the player
//!   knows of them, in the step's arguments and as its actor. The ledger
//!   is not written: everywhere else the player has still not heard of
//!   them.

use crate::{DateView, EventView, PersonRefView};
use mandate_sim::Engine;
use mandate_sim::events::{DomainEvent, EventId, EventKind, EventLog};
use mandate_sim::spine::{Cause, Link, Process, Thread, kind_key};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use ts_rs::TS;

/// How an earlier event led to a later one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum LinkDto {
    /// It brought this about directly.
    Caused,
    /// The same matter moving on (a bill introduced, then enacted).
    Continues,
    /// Someone acted because of it.
    Prompted,
    /// This made the earlier event known.
    Revealed,
    /// This closed what the earlier event opened.
    Ended,
}

impl From<Link> for LinkDto {
    fn from(l: Link) -> Self {
        match l {
            Link::Caused => LinkDto::Caused,
            Link::Continues => LinkDto::Continues,
            Link::Prompted => LinkDto::Prompted,
            Link::Revealed => LinkDto::Revealed,
            Link::Ended => LinkDto::Ended,
        }
    }
}

/// What an event names as a cause.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CauseView {
    /// An earlier event. `in_chain`: it is one of the steps given (else it
    /// lies beyond the depth asked for, or is no longer kept).
    Event {
        event: u32,
        link: LinkDto,
        in_chain: bool,
    },
    /// The player's own act: the command kind (`vote`, `introduce`,
    /// `take_bribe`, `let_pass`, …; the engine's `PlayerAct` keys).
    Player { act: String },
    /// Someone's own choice; `person` as the player knows them (`None`:
    /// no one they could name).
    Actor { person: Option<PersonRefView> },
    /// A scheduled process applying its standing rule (`lawmaking`,
    /// `elections`, `press`, `justice`, `asks`, `agenda`, `budget`, …).
    Process { process: String },
    /// A standing condition the engine's formula names (`world_demand`,
    /// `commodity_prices`, `depletion`, `competitiveness`, `labour_costs`,
    /// `track`).
    Pressure { pressure: String },
    /// The world as it was made.
    History,
    /// Exogenous by design (`world_economy`, `commodities`, `hazard`,
    /// `race`): uncaused inside the simulation, and saying so.
    Exogenous { shock: String },
    /// An earlier event the log no longer keeps.
    Forgotten,
    /// Recorded before the engine kept causes (an older save).
    Unknown,
}

/// The link from a step to the later step it led to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WhyLinkView {
    /// The later event (a step nearer the one asked about).
    pub event: u32,
    pub link: LinkDto,
}

/// One event of the chain.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WhyStepView {
    pub id: u32,
    /// The event's kind (`plant_closure`, `policy_enacted`, …).
    pub kind: String,
    pub date: DateView,
    /// The engine's text for the event (`event-…`) and its arguments.
    pub text_key: String,
    pub args: BTreeMap<String, String>,
    /// Links from the event asked about (which is 0).
    pub depth: u8,
    /// What it led to, and how (`None` for the event asked about).
    pub led_to: Option<WhyLinkView>,
    pub causes: Vec<CauseView>,
}

/// The player's act nearest behind the event (credit and blame).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WhyPlayerView {
    pub act: String,
    /// The step that names it.
    pub event: u32,
    pub depth: u8,
}

/// The chain behind an event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WhyView {
    pub event: u32,
    /// The event, then what led to it, nearest first.
    pub chain: Vec<WhyStepView>,
    /// More lies behind than the depth asked for.
    pub truncated: bool,
    pub player: Option<WhyPlayerView>,
    /// The links back this answer walked, and the most a query may ask
    /// for (a client offers "further back" while `depth < max_depth`).
    pub depth: u8,
    pub max_depth: u8,
}

/// The depth a chain is walked when none is asked for, and the most.
pub const WHY_DEPTH: u8 = 6;
pub const WHY_MAX_DEPTH: u8 = 12;

/// Diagnostics the player never sees as events.
fn hidden(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::DirectorIntervention { .. } | EventKind::FactDiscoverable { .. }
    )
}

/// The kinds of event that are a public result (their `kind_key`s): the
/// people such a step names are public record, so they are named here
/// whatever the player has learned (DEC-108 §8). Everything else stays
/// under the knowledge gate (DEC-091).
///
/// - `race_decided`: the winner of an election.
/// - `policy_enacted`, `policy_failed`: a law's recorded vote.
/// - `scandal_broke`: the subject of a published story.
/// - `case_developed`: the subject of a court's ruling (charged, dropped,
///   acquitted, convicted).
pub const PUBLIC_RESULTS: [&str; 5] = [
    "race_decided",
    "policy_enacted",
    "policy_failed",
    "scandal_broke",
    "case_developed",
];

/// Whether an event is a public result ([`PUBLIC_RESULTS`]).
pub fn public_result(kind: &EventKind) -> bool {
    PUBLIC_RESULTS.contains(&kind_key(kind))
}

/// An event the player may ask "Why?" about, as a view carries it: one the
/// log still keeps and that is not a diagnostic.
pub(crate) fn askable(log: &EventLog, id: EventId) -> Option<u32> {
    log.get(id).filter(|e| !hidden(&e.kind)).map(|e| e.id.0)
}

/// The latest event of a matter (a bill, an ask, a decision, a scandal, a
/// case), when it is of the kind a view shows as the matter's outcome.
pub(crate) fn thread_event(
    log: &EventLog,
    thread: Thread,
    is: impl Fn(&EventKind) -> bool,
) -> Option<u32> {
    let e = log.get(log.head(thread)?)?;
    if is(&e.kind) {
        askable(log, e.id)
    } else {
        None
    }
}

/// The latest event that holds `is`, looking back no further than `since`
/// (a view names an outcome by its day when no thread keeps it).
pub(crate) fn event_since(
    log: &EventLog,
    since: mandate_core::SimDate,
    is: impl Fn(&DomainEvent) -> bool,
) -> Option<u32> {
    log.iter()
        .rev()
        .take_while(|e| e.date >= since)
        .find(|e| is(e))
        .and_then(|e| askable(log, e.id))
}

/// An event's causes as the player may see them: a link to a hidden event
/// is replaced by what that event names (its links keep the link type that
/// led into the hidden run; the director's hand in a hidden event is
/// dropped). A hidden event is never returned, however long the run of
/// them: past [`WHY_MAX_DEPTH`] hidden events in a row the walk stops
/// there and says so (`cut`).
fn visible_causes(log: &EventLog, id: EventId) -> (Vec<Cause>, bool) {
    let mut out: Vec<Cause> = Vec::new();
    let mut cut = false;
    // The event, then the hidden events behind it, breadth-first: (event,
    // the link that led into the hidden run, hidden events still allowed).
    let mut through: Vec<(EventId, Option<Link>, u8)> = vec![(id, None, WHY_MAX_DEPTH)];
    let mut i = 0;
    while i < through.len() {
        let (at, via, left) = through[i];
        i += 1;
        let Some(e) = log.get(at) else { continue };
        for c in &e.causes {
            let c = match (*c, via) {
                // Inside a hidden event: its links carry the outer type.
                (Cause::Event(to, _), Some(link)) => Cause::Event(to, link),
                (Cause::Process(Process::Director), Some(_)) => continue,
                (c, _) => c,
            };
            match c {
                Cause::Event(to, link) if log.get(to).is_some_and(|x| hidden(&x.kind)) => {
                    if through.iter().any(|t| t.0 == to) {
                        continue;
                    }
                    if left == 0 {
                        cut = true;
                        continue;
                    }
                    through.push((to, Some(link), left - 1));
                }
                c => {
                    if !out.contains(&c) {
                        out.push(c);
                    }
                }
            }
        }
    }
    (out, cut)
}

/// The full names of everyone the player has not heard of (DEC-091): no
/// step's arguments may carry one.
fn unknown_names(engine: &Engine, gate: &crate::knowledge::Gate<'_>) -> BTreeSet<String> {
    let st = engine.state();
    let pol = &st.politics;
    let heard = mandate_sim::player::knowledge::Level::Heard;
    pol.characters
        .people
        .iter()
        .filter(|c| gate.person(c.id) < heard)
        .filter_map(|c| mandate_sim::characters::profile(pol, &st.world, c.id))
        .map(|p| p.full_name())
        .filter(|n| !n.trim().is_empty())
        .collect()
}

/// The engine's text for an event, as the player may read it here: the
/// overview's line, with the name of anyone the player has not heard of
/// left empty (the unknown form, as in `PersonRefView`). The Why query
/// takes any event id, so it must not be a way to read every name. A
/// public result names whom it names ([`PUBLIC_RESULTS`]).
fn gated_event_view(e: &DomainEvent, engine: &Engine, unknown: &BTreeSet<String>) -> EventView {
    let mut v = crate::views::event_view(e, engine);
    if public_result(&e.kind) {
        return v;
    }
    for value in v.args.values_mut() {
        if unknown.contains(value.as_str()) {
            value.clear();
        }
    }
    v
}

/// A step of the walk: the event, its depth, what it led to, its causes.
type Visited = (EventId, u8, Option<(EventId, Link)>, Vec<Cause>);

fn key<T: Serialize>(v: &T) -> String {
    match serde_json::to_value(v) {
        Ok(serde_json::Value::String(s)) => s,
        _ => String::new(),
    }
}

/// Why an event happened, at most `depth` links back (default
/// [`WHY_DEPTH`], at most [`WHY_MAX_DEPTH`]). `None`: no such event, or
/// one the player never sees.
pub fn why_view(engine: &Engine, event: u32, depth: Option<u8>) -> Option<WhyView> {
    let st = engine.state();
    let log = &st.events;
    let start = log.get(EventId(event))?;
    if hidden(&start.kind) {
        return None;
    }
    let max = depth.unwrap_or(WHY_DEPTH).clamp(1, WHY_MAX_DEPTH);
    let gate = crate::knowledge::Gate::of(engine);
    // Breadth-first over the visible graph: (event, depth, led to).
    let (first, mut truncated) = visible_causes(log, start.id);
    let mut steps: Vec<Visited> = vec![(start.id, 0, None, first)];
    let mut i = 0;
    while i < steps.len() {
        let (at, d, _, causes) = steps[i].clone();
        i += 1;
        for c in &causes {
            let Cause::Event(to, link) = c else { continue };
            // (A hidden event is never among the causes: `visible_causes`.)
            let Some(next) = log.get(*to).filter(|x| !hidden(&x.kind)) else {
                continue;
            };
            if steps.iter().any(|s| s.0 == next.id) {
                continue;
            }
            if d >= max {
                truncated = true;
                continue;
            }
            let (causes, cut) = visible_causes(log, next.id);
            truncated |= cut;
            steps.push((next.id, d + 1, Some((at, *link)), causes));
        }
    }
    let in_chain = |id: EventId| steps.iter().any(|s| s.0 == id);
    let unknown = unknown_names(engine, &gate);
    let player = steps.iter().find_map(|(id, d, _, causes)| {
        causes
            .iter()
            .find_map(Cause::player_act)
            .map(|act| WhyPlayerView {
                act: key(&act),
                event: id.0,
                depth: *d,
            })
    });
    let chain = steps
        .iter()
        .filter_map(|(id, d, led_to, causes)| {
            let e = log.get(*id)?;
            let EventView {
                id,
                date,
                text_key,
                args,
            } = gated_event_view(e, engine, &unknown);
            // Whom a public result names is named as its actor too.
            let named: BTreeSet<&str> = if public_result(&e.kind) {
                args.values().map(String::as_str).collect()
            } else {
                BTreeSet::new()
            };
            let named_here = |w: mandate_sim::characters::CharacterId| {
                mandate_sim::characters::profile(&st.politics, &st.world, w)
                    .map(|p| p.full_name())
                    .is_some_and(|n| !n.trim().is_empty() && named.contains(n.as_str()))
            };
            let causes = causes
                .iter()
                .filter_map(|c| {
                    Some(match c {
                        Cause::Event(to, link) => {
                            // A link the log no longer holds reads as forgotten.
                            if log.get(*to).is_none() {
                                CauseView::Forgotten
                            } else {
                                CauseView::Event {
                                    event: to.0,
                                    link: (*link).into(),
                                    in_chain: in_chain(*to),
                                }
                            }
                        }
                        Cause::Player(act) => CauseView::Player { act: key(act) },
                        Cause::Actor(who) => CauseView::Actor {
                            person: if gate.person(*who)
                                >= mandate_sim::player::knowledge::Level::Heard
                            {
                                crate::person_ref_known(engine, &gate, *who)
                            } else if named_here(*who) {
                                crate::person_ref(engine, *who)
                            } else {
                                None
                            },
                        },
                        Cause::Process(p) => CauseView::Process { process: key(p) },
                        Cause::Pressure(p) => CauseView::Pressure { pressure: key(p) },
                        Cause::History => CauseView::History,
                        Cause::Exogenous(s) => CauseView::Exogenous { shock: key(s) },
                        // A lab intervention is not a player's game.
                        Cause::Lab => return None,
                        Cause::Forgotten => CauseView::Forgotten,
                        Cause::Unknown => CauseView::Unknown,
                    })
                })
                .collect();
            Some(WhyStepView {
                id,
                kind: kind_key(&e.kind).to_string(),
                date,
                text_key,
                args,
                depth: *d,
                led_to: led_to.map(|(to, link)| WhyLinkView {
                    event: to.0,
                    link: link.into(),
                }),
                causes,
            })
        })
        .collect();
    Some(WhyView {
        event,
        chain,
        truncated,
        player,
        depth: max,
        max_depth: WHY_MAX_DEPTH,
    })
}
