//! The town calendar (DEC-114 §4, `BEING_THERE.md` §5), and the day played.
//!
//! **One date per turn** (DEC-114, the owner's ruling). A turn is played
//! as one day, the *day played*, and everything that names a day uses it:
//! the bar, people's routines, the calendar's events, the facts the turn
//! leaves. The engine chooses it as the turn begins ([`choose_day`],
//! stored in `Calendar.day`): the date of the turn's main occasion (one the
//! player is expected at first, then the largest crowd, then the first in
//! the registry), else a working day of the span by keyed draw. Every event
//! of the turn is held that day, each in a part of its own (an event's
//! `parts`, the first preferred), so the date shown and the people present
//! always agree. An occasion that finds no part left is reported as having
//! happened on its own date, never shown on a day the player does not play.
//!
//! **Instances** are derived, never stored: a town has an event when its
//! venue exists there and its `requires` hold (DEC-100); an occasion's own
//! date is a keyed day in its window, and its name a keyed pick, both by
//! the seed, the jurisdiction and the event (DEC-098: keyed by
//! `Jurisdiction`). The routine events `session` and `meeting_night` are
//! the registry's first entries and read exactly as routines read them.
//!
//! **Expected** (owner's answer 2): an event's `expected` rules name the
//! player by a power, a role word or the seat whose district holds the
//! venue; at most one such event a turn, and only it costs when missed
//! ([`end_turn`]). Its cost is shown a turn ahead ([`next_turn`]).
//!
//! Keyed draws only (DEC-041): no stream advances.

use super::routines::{self, DayPart, Now};
use super::{CharacterId, key_of, rng};
use crate::events::EventKind;
use crate::places::Location;
use crate::spine::{Cause, Process};
use crate::state::{GamePhase, State};
use crate::world::{Jurisdiction, MunicipalityId, PlaceId, PlaceKind};
use mandate_content::calendar::{CalendarCadence, CalendarEventDef, CalendarExpected};
use mandate_content::routines::RoutineDay;
use mandate_content::{ContentBundle, StageDef};
use mandate_core::SimDate;

/// Keyed-RNG tags (DEC-041), beside `routines::KEY_*`.
pub(crate) const KEY_DAY: u64 = 0xC4A9;
pub(crate) const KEY_WHEN: u64 = 0xC4AA;
pub(crate) const KEY_NAME: u64 = 0xC4AB;
pub(crate) const KEY_VENUE: u64 = 0xC4AC;
pub(crate) const KEY_ATTEND: u64 = 0xC4AD;

/// One event of a turn in a jurisdiction.
#[derive(Clone, Debug, PartialEq)]
pub struct Held {
    /// Index in the registry (`content.calendar`).
    pub event: usize,
    pub at: Jurisdiction,
    /// The place it is held at.
    pub venue: PlaceId,
    /// The part of the day played it is held in; `None`: it could not be
    /// held that day and is reported as having happened on `on`.
    pub part: Option<DayPart>,
    /// Its date: the day played, or (reported) its own.
    pub on: SimDate,
    /// The player is expected there (at most one event a turn).
    pub expected: bool,
    /// It chose the day played.
    pub main: bool,
    /// Its name in this town (a Fluent key with `$town`).
    pub name_key: String,
}

impl Held {
    pub fn def<'a>(&self, content: &'a ContentBundle) -> &'a CalendarEventDef {
        &content.calendar[self.event]
    }

    pub fn town(&self) -> Option<MunicipalityId> {
        self.at.municipality()
    }

    /// Held on the day played (not reported).
    pub fn today(&self) -> bool {
        self.part.is_some()
    }

    /// The location it is held at.
    pub fn location(&self) -> Location {
        Location::Place { place: self.venue }
    }
}

/// A turn's span: its first day, its length and its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: SimDate,
    pub days: u32,
    pub turn: u32,
}

impl Span {
    /// This turn.
    pub fn now(state: &State, content: &ContentBundle) -> Span {
        Span {
            start: state.calendar.date,
            days: routines::turn_days(state, content),
            turn: state.calendar.turn,
        }
    }

    /// The next turn, assuming the phase holds.
    pub fn next(state: &State, content: &ContentBundle) -> Span {
        let now = Span::now(state, content);
        Span {
            start: now.start.add_days(now.days),
            days: now.days,
            turn: now.turn + 1,
        }
    }

    pub fn contains(&self, d: SimDate) -> bool {
        d >= self.start && d < self.start.add_days(self.days)
    }

    fn dates(&self) -> impl Iterator<Item = SimDate> + '_ {
        (0..self.days).map(|i| self.start.add_days(i))
    }
}

/// The jurisdictions whose legislature has business in a span: a
/// law-making stage of any kind (`sessions`), and a vote stage (`votes`).
pub fn law_making(
    state: &State,
    content: &ContentBundle,
    span: Span,
) -> (Vec<Jurisdiction>, Vec<Jurisdiction>) {
    let mut sessions = Vec::new();
    let mut votes = Vec::new();
    let c = content.constitutions.get(&state.meta.constitution);
    let until = span.start.add_days(span.days);
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
    (sessions, votes)
}

/// Where an event is held in a town: an open place of the venue's kind (the
/// legislature's, for `legislature`), one kept for the event by a keyed
/// pick. `None`: the town has none, so it has no such event.
fn venue(
    state: &State,
    content: &ContentBundle,
    town: MunicipalityId,
    def: &CalendarEventDef,
) -> Option<PlaceId> {
    let kind = if def.venue == "legislature" {
        let c = content.constitutions.get(&state.meta.constitution)?;
        let body = crate::politics::legislature(c, mandate_content::TierKey::Local)?;
        PlaceKind::from_key(&body.works_at)?
    } else {
        PlaceKind::from_key(&def.venue)?
    };
    let mu = state.world.municipalities.get(town.index())?;
    let opts: Vec<PlaceId> = mu
        .places
        .iter()
        .filter(|p| {
            state
                .world
                .places
                .get(p.index())
                .is_some_and(|x| x.kind == kind && x.is_open())
        })
        .copied()
        .collect();
    let mut r = rng(
        state.meta.seed,
        &[KEY_VENUE, town.0 as u64, key_of(&def.id)],
    );
    r.pick(&opts).copied()
}

fn requires_hold(
    state: &State,
    content: &ContentBundle,
    town: MunicipalityId,
    def: &CalendarEventDef,
) -> bool {
    def.requires.is_empty()
        || crate::condition::on_state(state, content, |v| {
            let cx = crate::condition::Ctx::town(v, state, town);
            crate::condition::all_hold(&cx, &def.requires, &mut Default::default())
        })
}

/// The event's name in a jurisdiction (a keyed pick among its names).
pub fn name_key(seed: u64, at: Jurisdiction, def: &CalendarEventDef) -> String {
    let n = def.names.len().max(1) as u64;
    let i = rng(
        seed,
        &[KEY_NAME, at.tier_index(), at.raw() as u64, key_of(&def.id)],
    )
    .below(n) as usize;
    def.names.get(i).cloned().unwrap_or_default()
}

/// Move a date on to a weekday (0 to 6 days later).
fn on_weekday(d: SimDate, w: Option<RoutineDay>) -> SimDate {
    let Some(w) = w else {
        return d;
    };
    let mut x = d;
    for _ in 0..7 {
        if RoutineDay::of(x) == w {
            return x;
        }
        x = x.add_days(1);
    }
    d
}

/// An occasion's own dates that fall in a span (keyed by the seed, the
/// jurisdiction, the event and the year or month).
fn occasion_dates(seed: u64, at: Jurisdiction, def: &CalendarEventDef, span: Span) -> Vec<SimDate> {
    let first = span.start.calendar().year;
    let last = span
        .start
        .add_days(span.days.saturating_sub(1))
        .calendar()
        .year;
    let id = key_of(&def.id);
    let key = |year: u32, month: u32, part: u64| {
        rng(
            seed,
            &[
                KEY_WHEN,
                at.tier_index(),
                at.raw() as u64,
                id,
                year as u64,
                month as u64,
                part,
            ],
        )
    };
    let mut out = Vec::new();
    // A year before the span too: a date moved on to its weekday can cross
    // into the next month or year.
    for year in first.saturating_sub(1)..=last {
        match def.cadence {
            CalendarCadence::Month => {
                for month in 1..=12u32 {
                    if !def.months.is_empty() && !def.months.contains(&(month as u8)) {
                        continue;
                    }
                    let d = 1 + key(year, month, 0).below(28) as u32;
                    out.push(SimDate::from_ymd(year, month, d));
                }
            }
            CalendarCadence::Season => {
                for &m in &def.months {
                    let d = 1 + key(year, m as u32, 1).below(7) as u32;
                    out.push(SimDate::from_ymd(year, m as u32, d));
                }
            }
            CalendarCadence::Year => {
                let mut r = key(year, 0, 2);
                let m = def.months[r.below(def.months.len().max(1) as u64) as usize];
                let d = 1 + r.below(28) as u32;
                out.push(SimDate::from_ymd(year, m as u32, d));
            }
            _ => {}
        }
    }
    let mut v: Vec<SimDate> = out
        .into_iter()
        .map(|d| on_weekday(d, def.weekday))
        .filter(|d| span.contains(*d))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Whether the player is expected at an event (any of its rules).
fn player_expected(
    state: &State,
    content: &ContentBundle,
    at: Jurisdiction,
    def: &CalendarEventDef,
    venue: PlaceId,
) -> bool {
    let Some(pl) = state.player.as_ref() else {
        return false;
    };
    let Some(c) = content.constitutions.get(&state.meta.constitution) else {
        return false;
    };
    let pol = &state.politics;
    let posts = crate::player::posts(state);
    def.expected.iter().any(|x| match x {
        CalendarExpected::Power { power } => {
            crate::politics::holds_power_at(pol, c, pl.politician, *power, at)
        }
        CalendarExpected::Role { word } => posts
            .iter()
            .any(|p| p.at() == at && p.word(c) == Some(word.as_str())),
        CalendarExpected::Ward => {
            let hood = state
                .world
                .places
                .get(venue.index())
                .map(|p| p.neighborhood);
            posts.iter().any(|p| {
                p.at() == at
                    && p.constituency().is_some_and(|k| {
                        crate::policy::ward_hoods(&state.world, k)
                            .into_iter()
                            .any(|h| Some(h as u32) == hood.map(|n| n.0))
                    })
            })
        }
    })
}

/// A turn's events in a town, and the day played. With `day` given (the
/// stored day) the events are laid on it; without, the day is chosen.
pub fn plan(
    state: &State,
    content: &ContentBundle,
    span: Span,
    town: MunicipalityId,
    sessions: &[Jurisdiction],
    day: Option<SimDate>,
) -> (SimDate, Vec<Held>) {
    let seed = state.meta.seed;
    let at = Jurisdiction::town(town);
    // The town's events, with their venues.
    let mut here: Vec<(usize, &CalendarEventDef, PlaceId)> = Vec::new();
    for (i, def) in content.calendar.iter().enumerate() {
        if def.scope != mandate_content::TierKey::Local {
            continue;
        }
        let Some(v) = venue(state, content, town, def) else {
            continue;
        };
        if requires_hold(state, content, town, def) {
            here.push((i, def, v));
        }
    }
    // The occasions of the span, strongest first: expected, then crowd,
    // then registry order (then the earlier date).
    let mut occasions: Vec<(usize, &CalendarEventDef, PlaceId, SimDate, bool)> = Vec::new();
    for (i, def, v) in &here {
        if !def.cadence.occasion() {
            continue;
        }
        if let Some(d) = occasion_dates(seed, at, def, span).first() {
            let exp = player_expected(state, content, at, def, *v);
            occasions.push((*i, *def, *v, *d, exp));
        }
    }
    occasions.sort_by(|a, b| {
        b.4.cmp(&a.4)
            .then(b.1.crowd.cmp(&a.1.crowd))
            .then(a.0.cmp(&b.0))
            .then(a.3.cmp(&b.3))
    });
    let day = day.unwrap_or_else(|| match occasions.first() {
        Some(o) => o.3,
        None => working_day(seed, at, span),
    });
    let weekday = RoutineDay::of(day);
    let mut taken: Vec<DayPart> = Vec::new();
    let mut out: Vec<Held> = Vec::new();
    let hold = |i: usize,
                def: &CalendarEventDef,
                v: PlaceId,
                on: SimDate,
                main: bool,
                shared: bool,
                out: &mut Vec<Held>,
                taken: &mut Vec<DayPart>| {
        let part = def
            .parts
            .iter()
            .find(|p| !taken.contains(p))
            .copied()
            .or_else(|| shared.then(|| def.parts.first().copied()).flatten());
        if let Some(p) = part
            && !taken.contains(&p)
        {
            taken.push(p);
        }
        let held = part.is_some();
        if !held && !def.cadence.occasion() {
            return;
        }
        out.push(Held {
            event: i,
            at,
            venue: v,
            part,
            on: if held { day } else { on },
            expected: false,
            main,
            name_key: name_key(seed, at, def),
        });
    };
    // The main occasion, when it chose this day.
    let mut rest = occasions.clone();
    if let Some(pos) = rest.iter().position(|o| o.3 == day) {
        let (i, def, v, on, _) = rest.remove(pos);
        hold(i, def, v, on, true, false, &mut out, &mut taken);
    }
    // The legislature sitting and the meeting night, when the day is theirs
    // (the sitting keeps its part even when it is taken).
    for (i, def, v) in &here {
        let today = match def.cadence {
            CalendarCadence::Session => {
                sessions.contains(&at) || routines::sitting_day(seed, at) == weekday
            }
            CalendarCadence::MeetingNight => routines::meeting_night(seed, town) == weekday,
            _ => continue,
        };
        if today {
            let shared = def.cadence == CalendarCadence::Session;
            hold(*i, def, *v, day, false, shared, &mut out, &mut taken);
        }
    }
    // The other occasions of the span: the same day if a part is left.
    for (i, def, v, on, _) in rest {
        hold(i, def, v, on, false, false, &mut out, &mut taken);
    }
    // Every-turn events last.
    for (i, def, v) in &here {
        if def.cadence == CalendarCadence::Turn {
            hold(*i, def, *v, day, false, false, &mut out, &mut taken);
        }
    }
    // At most one expected event a turn: the first held that expects the
    // player.
    if let Some(h) = out
        .iter_mut()
        .filter(|h| h.today())
        .find(|h| player_expected(state, content, at, &content.calendar[h.event], h.venue))
    {
        h.expected = true;
    }
    // In the day's order, then the reported ones by date.
    out.sort_by_key(|h| (h.part.is_none(), h.part.map(|p| p.index()), h.on, h.event));
    (day, out)
}

/// A working day of the span (Monday to Friday) by keyed draw on the
/// jurisdiction and the turn; any day of a span without one.
fn working_day(seed: u64, at: Jurisdiction, span: Span) -> SimDate {
    let work: Vec<SimDate> = span
        .dates()
        .filter(|d| !matches!(RoutineDay::of(*d), RoutineDay::Sat | RoutineDay::Sun))
        .collect();
    let all: Vec<SimDate> = span.dates().collect();
    let pool = if work.is_empty() { &all } else { &work };
    let mut r = rng(
        seed,
        &[KEY_DAY, at.tier_index(), at.raw() as u64, span.turn as u64],
    );
    r.pick(pool).copied().unwrap_or(span.start)
}

/// The town whose calendar sets the day: the player's home (DEC-098: their
/// home, not `player.municipality`).
fn home(state: &State) -> Option<MunicipalityId> {
    crate::player::home(state).filter(|m| state.world.municipalities.get(m.index()).is_some())
}

/// Choose the day played for this turn (as the turn begins): the home
/// town's main occasion's date, else a working day. Without a player, a
/// working day keyed on the nation.
pub fn choose_day(state: &State, content: &ContentBundle) -> SimDate {
    let span = Span::now(state, content);
    match home(state) {
        Some(town) => {
            let (sessions, _) = law_making(state, content, span);
            plan(state, content, span, town, &sessions, None).0
        }
        None => working_day(state.meta.seed, Jurisdiction::Nation, span),
    }
}

/// The day played this turn: the stored day while it is inside the turn,
/// else chosen now (a save from before it, a lab advance).
pub fn play_day(state: &State, content: &ContentBundle) -> SimDate {
    let span = Span::now(state, content);
    if span.contains(state.calendar.day) {
        state.calendar.day
    } else {
        choose_day(state, content)
    }
}

/// This turn's events in the player's home town, on the day played.
pub fn this_turn(state: &State, content: &ContentBundle) -> Vec<Held> {
    let Some(town) = home(state) else {
        return Vec::new();
    };
    let span = Span::now(state, content);
    let (sessions, _) = law_making(state, content, span);
    let day = play_day(state, content);
    plan(state, content, span, town, &sessions, Some(day)).1
}

/// The next turn's events in the player's home town, on the day it would
/// be played (for "shown a turn ahead"), assuming the phase holds.
pub fn next_turn(state: &State, content: &ContentBundle) -> (SimDate, Vec<Held>) {
    let Some(town) = home(state) else {
        return (SimDate::default(), Vec::new());
    };
    let span = Span::next(state, content);
    let (sessions, _) = law_making(state, content, span);
    plan(state, content, span, town, &sessions, None)
}

/// The events held now (`Now`): this turn's, on the day played, in the
/// player's focus towns.
pub(crate) fn held_today(
    state: &State,
    content: &ContentBundle,
    span: Span,
    day: SimDate,
    sessions: &[Jurisdiction],
) -> Vec<Held> {
    let Some(town) = home(state) else {
        return Vec::new();
    };
    plan(state, content, span, town, sessions, Some(day))
        .1
        .into_iter()
        .filter(Held::today)
        .collect()
}

/// Whether an attendee rule covers someone, by their post's role word,
/// occupation or special role.
pub(crate) fn covers(
    a: &mandate_content::calendar::CalendarAttendee,
    word: Option<&str>,
    occupation: Option<&str>,
    roles: &[mandate_content::routines::RoutineRole],
) -> bool {
    if let Some(w) = &a.post {
        return word == Some(w.as_str());
    }
    if let Some(o) = &a.occupation {
        return occupation == Some(o.as_str());
    }
    a.role.is_some_and(|r| roles.contains(&r))
}

/// The keyed draw of an attendee rule (person, day, event, rule).
pub(crate) fn attends(
    seed: u64,
    person: CharacterId,
    day: SimDate,
    def: &CalendarEventDef,
    rule: usize,
    chance: f64,
) -> bool {
    rng(
        seed,
        &[
            KEY_ATTEND,
            person.0 as u64,
            day.0 as u64,
            key_of(&def.id),
            rule as u64,
        ],
    )
    .chance(chance)
}

/// The event held at a location now, if any.
pub fn at_location<'a>(now: &'a Now, loc: Location) -> Option<&'a Held> {
    now.held
        .iter()
        .find(|h| h.part == Some(now.part) && h.location() == loc)
}

/// The player is where they are, now: if an event is held here in this
/// part, they were there (recorded for the turn).
pub fn note_presence(state: &mut State, content: &ContentBundle) {
    let Some(loc) = state.player.as_ref().map(|p| p.location) else {
        return;
    };
    let now = Now::of(state, content);
    let Some(id) = at_location(&now, loc).map(|h| h.def(content).id.clone()) else {
        return;
    };
    if let Some(pl) = state.player.as_mut()
        && !pl.at_events.contains(&id)
    {
        pl.at_events.push(id);
    }
}

/// Whether the player was at an event this turn.
pub fn attended(state: &State, content: &ContentBundle, h: &Held) -> bool {
    let id = &h.def(content).id;
    state
        .player
        .as_ref()
        .is_some_and(|p| p.at_events.iter().any(|x| x == id))
}

/// As a turn ends, before time moves (from `Engine::end_one_turn`): the
/// facts of the turn's occasions (held, with whether the player came, or
/// reported), an expected absence's cost among those who were there, and
/// familiarity's monthly fade. Then the turn's attendance is cleared.
pub fn end_turn(state: &mut State, content: &ContentBundle) {
    if state.calendar.phase == GamePhase::BurnIn || state.player.is_none() {
        return;
    }
    let me = super::player_character(state);
    let events = this_turn(state, content);
    let date = state.calendar.date;
    for h in &events {
        let def = h.def(content);
        let came = attended(state, content, h);
        let missed = h.expected && !came;
        if missed
            && let (Some(part), Some(cost)) = (h.part, def.absent.as_ref())
            && let Some(me) = me
        {
            let now = Now::at(state, content, part);
            let there: Vec<CharacterId> = routines::present(state, content, &now, h.location())
                .into_iter()
                .filter(|c| *c != me)
                .collect();
            for c in there {
                super::change_opinion(&mut state.politics, content, c, me, cost.opinion);
            }
        }
        if def.cadence.occasion() || missed {
            state.events.record(
                date,
                EventKind::CalendarEvent {
                    municipality: h.town().map_or(0, |m| m.0),
                    event: def.id.clone(),
                    name_key: h.name_key.clone(),
                    on: h.on,
                    part: h.part.map(|p| p.key().to_string()),
                    attended: came,
                    missed,
                },
                Cause::Process(Process::Calendar),
            );
        }
    }
    // Familiarity fades once a month: on the turn whose span starts a month.
    let span = Span::now(state, content);
    let starts_month = span.dates().any(|d| d.calendar().day == 1);
    if starts_month && let Some(me) = me {
        super::familiar::fade(state, content, me);
    }
    if let Some(pl) = state.player.as_mut() {
        pl.at_events.clear();
    }
}
