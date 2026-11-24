//! Walking the causal graph (S1): events are nodes, `Cause::Event` links
//! are edges from an event to what led to it. Every walk is depth-limited
//! and deterministic (events in id order, causes in recorded order).
//!
//! A cause always has a smaller id than what it led to, and the log is
//! sorted by id: a walk back is a descending pass, a walk forward an
//! ascending one. No index is stored; [`Forward`] builds one for callers
//! that ask many forward questions (sifting).

use super::{Cause, Link, PlayerAct};
use crate::events::{DomainEvent, EventId, EventLog};

/// One event reached by a walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reached {
    pub event: EventId,
    /// Links from the walk's start (the start itself is 0).
    pub depth: u8,
    /// The neighbour it was reached from and the link between them (none
    /// for the start). Walking back, `from` is the later event.
    pub from: Option<(EventId, Link)>,
}

/// A root found by walking back: where a chain starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Root {
    /// The event that names the root.
    pub at: EventId,
    pub depth: u8,
    pub cause: Cause,
}

/// The chain behind an event.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Chain {
    /// The event, then what led to it, nearest first (breadth-first; ties
    /// in cause order).
    pub events: Vec<Reached>,
    /// Every root named along the way, nearest first.
    pub roots: Vec<Root>,
    /// The depth limit cut the walk short: more lies behind.
    pub truncated: bool,
}

/// The player's act nearest behind an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerBehind {
    pub act: PlayerAct,
    /// The event that names the act.
    pub at: EventId,
    /// Links from the event asked about (0: the event is the act's own).
    pub depth: u8,
}

/// What an event names as its causes (empty if the event is not kept).
pub fn causes_of(log: &EventLog, id: EventId) -> &[Cause] {
    log.get(id).map(|e| e.causes.as_slice()).unwrap_or(&[])
}

/// What an event led to directly, in id order.
pub fn effects_of(log: &EventLog, id: EventId) -> Vec<(EventId, Link)> {
    log.after(id)
        .iter()
        .flat_map(|e| {
            e.causes.iter().filter_map(move |c| match c {
                Cause::Event(c, link) if *c == id => Some((e.id, *link)),
                _ => None,
            })
        })
        .collect()
}

/// The chain from an event back to its roots, at most `max_depth` links.
pub fn chain(log: &EventLog, id: EventId, max_depth: u8) -> Chain {
    let mut out = Chain::default();
    if log.get(id).is_none() {
        return out;
    }
    out.events.push(Reached {
        event: id,
        depth: 0,
        from: None,
    });
    let mut i = 0;
    while i < out.events.len() {
        let at = out.events[i];
        i += 1;
        let Some(e) = log.get(at.event) else {
            continue;
        };
        for c in &e.causes {
            match c {
                Cause::Event(c, link) => {
                    if log.get(*c).is_none() || out.events.iter().any(|r| r.event == *c) {
                        continue;
                    }
                    if at.depth >= max_depth {
                        out.truncated = true;
                        continue;
                    }
                    out.events.push(Reached {
                        event: *c,
                        depth: at.depth + 1,
                        from: Some((at.event, *link)),
                    });
                }
                root => out.roots.push(Root {
                    at: at.event,
                    depth: at.depth,
                    cause: *root,
                }),
            }
        }
    }
    out
}

/// What an event led to within `max_links`, in id order (the event itself
/// is not listed). One ascending pass.
pub fn consequences(log: &EventLog, id: EventId, max_links: u8) -> Vec<Reached> {
    // (event, depth), sorted by event id as it is filled.
    let mut reached: Vec<(EventId, u8)> = vec![(id, 0)];
    let mut out = Vec::new();
    if log.get(id).is_none() || max_links == 0 {
        return out;
    }
    for e in log.after(id) {
        // The nearest way here: the least depth among its reached causes.
        let mut best: Option<(u8, EventId, Link)> = None;
        for c in &e.causes {
            let Cause::Event(c, link) = c else { continue };
            let Ok(k) = reached.binary_search_by_key(c, |r| r.0) else {
                continue;
            };
            let d = reached[k].1;
            if d < max_links && best.is_none_or(|b| d + 1 < b.0) {
                best = Some((d + 1, *c, *link));
            }
        }
        if let Some((depth, from, link)) = best {
            reached.push((e.id, depth));
            out.push(Reached {
                event: e.id,
                depth,
                from: Some((from, link)),
            });
        }
    }
    out
}

/// The nearest player act behind an event, if any within `max_depth`
/// links (credit and blame). Nearest by links; ties go to the cause named
/// first.
pub fn nearest_player_act(log: &EventLog, id: EventId, max_depth: u8) -> Option<PlayerBehind> {
    chain(log, id, max_depth).roots.iter().find_map(|r| {
        r.cause.player_act().map(|act| PlayerBehind {
            act,
            at: r.at,
            depth: r.depth,
        })
    })
}

/// A forward index: every link, sorted by the earlier event. Build it once
/// to ask many forward questions.
pub struct Forward {
    /// (cause, effect, link), sorted.
    edges: Vec<(EventId, EventId, Link)>,
}

impl Forward {
    pub fn build(log: &EventLog) -> Forward {
        let mut edges: Vec<(EventId, EventId, Link)> = log
            .iter()
            .flat_map(|e| {
                e.causes.iter().filter_map(move |c| match c {
                    Cause::Event(c, link) => Some((*c, e.id, *link)),
                    _ => None,
                })
            })
            .collect();
        edges.sort();
        Forward { edges }
    }

    /// What an event led to directly, in id order.
    pub fn of(&self, id: EventId) -> &[(EventId, EventId, Link)] {
        let lo = self.edges.partition_point(|e| e.0 < id);
        let hi = self.edges.partition_point(|e| e.0 <= id);
        &self.edges[lo..hi]
    }

    pub fn len(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

/// One step of a pattern along a chain: an event that passes `test`,
/// reached from the step before within `within` links, the last of them of
/// type `via` when given. The first step's `within` and `via` are unused.
pub struct Step<'a> {
    pub test: &'a dyn Fn(&DomainEvent) -> bool,
    pub within: u8,
    pub via: Option<Link>,
}

impl<'a> Step<'a> {
    /// The next event on the chain, directly linked.
    pub fn next(test: &'a dyn Fn(&DomainEvent) -> bool) -> Step<'a> {
        Step {
            test,
            within: 1,
            via: None,
        }
    }

    /// An event further down the chain, at most `within` links on.
    pub fn later(test: &'a dyn Fn(&DomainEvent) -> bool, within: u8) -> Step<'a> {
        Step {
            test,
            within,
            via: None,
        }
    }

    pub fn via(mut self, link: Link) -> Step<'a> {
        self.via = Some(link);
        self
    }
}

/// Every way a pattern of kinds matches along the graph's chains: one
/// event id per step, in pattern order. The base for story sifting: a
/// sifting pattern is a list of steps whose tests are conditions.
/// Deterministic: matches come in id order of their events, step by step.
pub fn matches(log: &EventLog, pattern: &[Step<'_>], limit: usize) -> Vec<Vec<EventId>> {
    let mut out = Vec::new();
    let Some(first) = pattern.first() else {
        return out;
    };
    let fwd = Forward::build(log);
    let mut path = Vec::with_capacity(pattern.len());
    for e in log.iter().filter(|e| (first.test)(e)) {
        path.push(e.id);
        extend(log, &fwd, pattern, &mut path, &mut out, limit);
        path.pop();
        if out.len() >= limit {
            break;
        }
    }
    out
}

fn extend(
    log: &EventLog,
    fwd: &Forward,
    pattern: &[Step<'_>],
    path: &mut Vec<EventId>,
    out: &mut Vec<Vec<EventId>>,
    limit: usize,
) {
    if out.len() >= limit {
        return;
    }
    let Some(step) = pattern.get(path.len()) else {
        out.push(path.clone());
        return;
    };
    let Some(from) = path.last().copied() else {
        return;
    };
    for (next, _) in within(fwd, from, step.within, step.via) {
        if log.get(next).is_some_and(|e| (step.test)(e)) {
            path.push(next);
            extend(log, fwd, pattern, path, out, limit);
            path.pop();
        }
    }
}

/// Events reached from `from` in 1..=`links` links (the last of type `via`
/// when given), in id order, each once at its least depth.
fn within(fwd: &Forward, from: EventId, links: u8, via: Option<Link>) -> Vec<(EventId, u8)> {
    // Breadth-first over the forward index.
    let mut seen: Vec<(EventId, u8)> = vec![(from, 0)];
    let mut hits: Vec<(EventId, u8)> = Vec::new();
    let mut i = 0;
    while i < seen.len() {
        let (at, d) = seen[i];
        i += 1;
        if d >= links {
            continue;
        }
        for (_, next, link) in fwd.of(at) {
            if via.is_none_or(|v| v == *link) && !hits.iter().any(|h| h.0 == *next) {
                hits.push((*next, d + 1));
            }
            if !seen.iter().any(|s| s.0 == *next) {
                seen.push((*next, d + 1));
            }
        }
    }
    hits.sort();
    hits
}
