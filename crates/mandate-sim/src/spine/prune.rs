//! The bound on the causal graph (S1): events leave the log by age and
//! salience, never while a live system still needs them.
//!
//! The rule, applied about once a month:
//!
//! 1. **Age by salience.** A `Trace` event is kept [`TRACE_DAYS`], a
//!    `Routine` one [`ROUTINE_DAYS`], a `Notable` one [`NOTABLE_DAYS`]; a
//!    `Landmark` (and anything pinned) is kept for the run.
//! 2. **Live state holds.** The latest event of every live thread (a bill
//!    or a law not ended, an open proposal, ask, decision or case, a
//!    scandal not over, a running storyline, recession, campaign or prison
//!    term), every event live state names (a law's, a fact's, a story's, a
//!    Legacy entry's), and a town's closures and disasters while it is in
//!    crisis.
//! 3. **Nothing unread goes by age.** Systems read the log forward from a
//!    cursor (the director's news of the town, the asks' deed log, the
//!    press, the Legacy record, a skip's summary). Nothing from the oldest
//!    live reader's cursor on is pruned by age: a reader that waits (the
//!    director while the player is in prison, for years) finds every event
//!    it would have found. [`unread_from`] lists the readers.
//! 4. **Chains stay whole.** Whatever is kept keeps everything it descends
//!    from: a link in the log always points to an event in the log.
//! 5. **A hard cap**, [`MAX_EVENTS`], as a backstop. Past it, events kept
//!    only by their age go first, least salient and oldest first; if that
//!    is not enough, unread events go, oldest first whatever their kind
//!    (as the old log's "last 2,000" did, at three times the room). Links
//!    to what went become `Cause::Forgotten`. Live chains are never cut.
//!
//! So the log holds at most what a year, four years and twelve years of
//! each class record, the run's landmarks, live chains and what a waiting
//! reader has yet to read, and never more than the cap unless live state
//! alone exceeds it.

use super::{Cause, Salience, Thread, salience};
use crate::events::{EventId, EventKind, EventLog, Retention};
use crate::policy::PolicyStatus;
use mandate_core::SimDate;

/// How long a `Trace` event is kept (days): a year.
pub const TRACE_DAYS: u32 = 366;
/// How long a `Routine` event is kept: four years, a term.
pub const ROUTINE_DAYS: u32 = 4 * 365;
/// How long a `Notable` event is kept: twelve years.
pub const NOTABLE_DAYS: u32 = 12 * 365;
/// The hard cap on kept events (live chains excepted).
pub const MAX_EVENTS: usize = 6_000;
/// Days between prunes.
pub const PRUNE_EVERY_DAYS: u32 = 28;
/// A town in crisis holds its closures and disasters from this long
/// before the crisis began.
pub const CRISIS_LOOKBACK_DAYS: u32 = 2 * 365;

/// What live state holds in the log.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Holds {
    /// Unread: nothing from this id on goes by age (the oldest live
    /// reader's cursor). Only the hard cap can take it, last.
    pub floor: Option<EventId>,
    /// Live threads: their latest event is held.
    pub threads: Vec<Thread>,
    /// Events live state names.
    pub events: Vec<EventId>,
    /// (town, since): a town in crisis holds its closures and disasters
    /// dated from `since` on.
    pub towns: Vec<(u32, SimDate)>,
}

/// The first event some live reader has yet to read: the least of the
/// cursors of the systems that read the log forward and act on what they
/// find. A reader counts while it can still read again:
///
/// - **the asks** (`asks.last_event`): daily, whenever there is a player;
/// - **the press** (`media.last_event`): weekly, once outlets exist;
/// - **the director** (`narrative.seen.last_event`: what stirs it, and the
///   news of the town it may spotlight): weekly from play's start until
///   the career ends. It waits while the player is in prison and reads the
///   whole term on release;
/// - **the Legacy record** (`act.cursor`): as each turn ends, while the
///   act's record is open;
/// - **a skip** between its chunks (`act.skip.cursor`).
///
/// Without a player nothing reads (the history, NPC-only worlds).
pub fn unread_from(state: &crate::State) -> Option<EventId> {
    let pl = state.player.as_ref()?;
    let mut cursors: Vec<u32> = vec![state.asks.last_event.saturating_add(1)];
    if state.media.active {
        cursors.push(state.media.last_event.saturating_add(1));
    }
    if state.calendar.phase != crate::state::GamePhase::BurnIn
        && !matches!(pl.status, crate::player::RunStatus::Ended { .. })
    {
        cursors.push(state.narrative.seen.last_event.saturating_add(1));
    }
    if state.act.ending.is_none() && !state.act.records.is_empty() {
        cursors.push(state.act.cursor);
    }
    if let Some(skip) = &state.act.skip {
        cursors.push(skip.cursor);
    }
    cursors.into_iter().min().map(EventId)
}

/// What live state needs kept (the whole state is read; nothing is written).
pub fn holds(state: &crate::State) -> Holds {
    let mut h = Holds {
        floor: unread_from(state),
        ..Holds::default()
    };
    // Bills on the table and laws in force.
    for p in &state.policy.instances {
        if p.status != PolicyStatus::Ended {
            h.threads.push(Thread::Bill(p.id.0));
            h.events.extend(p.cause);
        }
    }
    for f in &state.policy.families.instances {
        if f.is_open() {
            h.threads.push(Thread::Proposal(f.id.0));
        }
    }
    for a in &state.asks.asks {
        if matches!(
            a.status,
            crate::asks::AskStatus::Offered | crate::asks::AskStatus::Accepted { .. }
        ) {
            h.threads.push(Thread::Ask(a.id.0));
        }
    }
    let n = &state.narrative;
    for d in n.pending() {
        h.threads.push(Thread::Decision(d.id.0));
    }
    for r in n.active_runs() {
        h.threads.push(Thread::Storyline(r.id.0));
    }
    // A live arc run holds its latest event (and so its root).
    for r in &state.arcs.runs {
        h.threads.push(Thread::Arc(r.id.0));
    }
    for x in &n.hidden {
        if !x.surfaced {
            h.threads.push(Thread::Hidden(x.id));
        }
    }
    for f in &n.facts {
        if let crate::narrative::FactKind::TownEvent { event } = &f.kind {
            h.events.push(*event);
        }
    }
    for s in &state.media.scandals {
        if s.ended.is_none() {
            h.threads.push(Thread::Scandal(s.id));
        }
    }
    for r in &state.media.rumours {
        if r.corrected.is_none() {
            h.threads.push(Thread::Rumour(r.id));
        }
    }
    for s in &state.media.stories {
        if let crate::media::StorySource::Town { event } = &s.source {
            h.events.push(*event);
        }
    }
    for c in &state.justice.cases {
        if c.stage.open() {
            h.threads.push(Thread::Case(c.id));
            h.threads.push(Thread::Scandal(c.scandal));
        }
    }
    let g = &state.economy.global;
    if g.recession.is_some() || g.held_recession.is_some() {
        h.threads.push(Thread::Recession);
        h.events.extend(g.recession.as_ref().and_then(|r| r.event));
    }
    for d in &state.environment.deferred {
        h.events.extend(d.hold_event);
    }
    let el = &state.politics.elections;
    if let Some(day) = el.next_day
        && el.campaign_open.is_some()
    {
        h.threads.push(Thread::Election(day.0));
    }
    if state
        .player
        .as_ref()
        .is_some_and(|p| matches!(p.status, crate::player::RunStatus::Imprisoned { .. }))
    {
        h.threads.push(Thread::Sentence);
    }
    // The Legacy record: every entry that came from an event.
    for r in &state.act.records {
        h.events
            .extend(r.entries.iter().filter_map(|e| e.source_event));
    }
    // Towns in crisis.
    for (m, t) in state.politics.characters.towns.iter().enumerate() {
        if let Some(since) = t.crises.iter().map(|c| c.since).min() {
            h.towns.push((
                m as u32,
                SimDate(since.0.saturating_sub(CRISIS_LOOKBACK_DAYS)),
            ));
        }
    }
    h
}

/// Prune the state's log if it is due (about monthly): the engine calls
/// this as a turn ends, after an advance of days, and between the
/// history's steps. It changes only the log, and nothing reads the log for
/// an outcome further back than the rule keeps.
pub fn due(state: &mut crate::State) -> Option<Pruned> {
    let today = state.calendar.date;
    let mut log = std::mem::take(&mut state.events);
    let done = log.prune_if_due(today, || holds(state));
    state.events = log;
    done
}

/// What a prune did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pruned {
    pub before: usize,
    pub after: usize,
    /// Dropped by the hard cap (their links became `Forgotten`).
    pub capped: usize,
}

fn horizon(s: Salience) -> Option<u32> {
    match s {
        Salience::Trace => Some(TRACE_DAYS),
        Salience::Routine => Some(ROUTINE_DAYS),
        Salience::Notable => Some(NOTABLE_DAYS),
        Salience::Landmark => None,
    }
}

/// The salience the rule uses: a pinned event is a landmark.
pub fn kept_as(retention: Retention, kind: &EventKind) -> Salience {
    match retention {
        Retention::Pinned => Salience::Landmark,
        Retention::Recent => salience(kind),
    }
}

const DROP: u8 = 0;
/// Kept by its age.
const KEPT: u8 = 1;
/// Not yet read by a live reader.
const UNREAD: u8 = 2;
/// Held by live state.
const LIVE: u8 = 3;

/// Apply the rule now.
pub fn prune(log: &mut EventLog, today: SimDate, holds: &Holds) -> Pruned {
    let n = log.events.len();
    let mut out = Pruned {
        before: n,
        after: n,
        capped: 0,
    };
    let mut keep = vec![DROP; n];
    let mut sal = Vec::with_capacity(n);
    // 1 and 3. Age by salience; nothing unread; towns in crisis.
    for (i, e) in log.events.iter().enumerate() {
        let s = kept_as(e.retention, &e.kind);
        sal.push(s);
        if horizon(s).is_none_or(|days| e.date.days_until(today) <= days) {
            keep[i] = KEPT;
        }
        if holds.floor.is_some_and(|f| e.id >= f) {
            keep[i] = UNREAD;
        }
        if !holds.towns.is_empty()
            && let EventKind::PlantClosure { municipality, .. }
            | EventKind::Disaster { municipality, .. } = &e.kind
            && holds
                .towns
                .iter()
                .any(|(m, since)| m == municipality && e.date >= *since)
        {
            keep[i] = LIVE;
        }
    }
    // 2. What live state holds.
    let index = |log: &EventLog, id: EventId| log.events.binary_search_by_key(&id, |e| e.id).ok();
    for id in holds
        .events
        .iter()
        .copied()
        .chain(holds.threads.iter().filter_map(|t| log.head(*t)))
    {
        if let Some(i) = index(log, id) {
            keep[i] = LIVE;
        }
    }
    // 4. Chains stay whole: a cause has a smaller id than its effect, so
    // one descending pass closes the set.
    for i in (0..n).rev() {
        let k = keep[i];
        if k == DROP {
            continue;
        }
        for c in &log.events[i].causes {
            if let Cause::Event(c, _) = c
                && let Some(j) = index(log, *c)
                && keep[j] < k
            {
                keep[j] = k;
            }
        }
    }
    // 5. The hard cap: what only its age keeps, least salient and oldest
    // first; then what is unread, oldest first; never a live chain.
    let mut kept = keep.iter().filter(|k| **k != DROP).count();
    if kept > MAX_EVENTS {
        'cap: for (class, s) in [
            (KEPT, Some(Salience::Trace)),
            (KEPT, Some(Salience::Routine)),
            (KEPT, Some(Salience::Notable)),
            (KEPT, Some(Salience::Landmark)),
            (UNREAD, None),
        ] {
            for i in 0..n {
                if kept <= MAX_EVENTS {
                    break 'cap;
                }
                if keep[i] == class && s.is_none_or(|s| sal[i] == s) {
                    keep[i] = DROP;
                    kept -= 1;
                    out.capped += 1;
                }
            }
        }
    }
    if kept == n {
        return out;
    }
    let mut k = keep.iter();
    log.events.retain(|_| k.next().is_some_and(|k| *k != DROP));
    out.after = log.events.len();
    if out.capped > 0 {
        mend(log);
    }
    log.drop_dead_heads();
    out
}

/// Links to events that are no longer kept become `Forgotten` (after the
/// hard cap, and once for a save from before the spine).
pub fn mend(log: &mut EventLog) {
    let ids: Vec<EventId> = log.events.iter().map(|e| e.id).collect();
    for e in &mut log.events {
        let mut forgot = false;
        e.causes.retain(|c| match c {
            Cause::Event(c, _) if ids.binary_search(c).is_err() => {
                forgot = true;
                false
            }
            _ => true,
        });
        if forgot && !e.causes.contains(&Cause::Forgotten) {
            e.causes.push(Cause::Forgotten);
        }
    }
}
