//! The town calendar as the player sees it (DEC-114): the day played, this
//! fortnight's events in Today, the next turn's expected event (shown a
//! turn ahead), and what the turn just played held (the turn's passage).
//!
//! The engine decides everything here (`characters::calendar`): the day,
//! the events, their parts and places, who is expected, what staying away
//! costs. The UI lays it out and sends `MoveTo` to go (rule 4).

use crate::places::{LocationView, location_view};
use crate::pocket::TextView;
use crate::views::{DateView, EventView, event_view, weekday};
use mandate_sim::Engine;
use mandate_sim::characters::calendar::{self, Held};
use mandate_sim::events::EventKind;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// One event of a turn (DEC-114).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CalendarEventView {
    /// Content id (`event.market_day`).
    pub id: String,
    /// Its name in the town (`cal-*`, with `$town`).
    pub name: TextView,
    /// Its date: the day played, or (reported) its own.
    pub date: DateView,
    /// `weekday-<mon…sun>`.
    pub weekday_key: String,
    /// `day-part-<part>`; none when it was not held on the day played.
    pub part_key: Option<String>,
    /// `morning` | `afternoon` | `evening`; none when reported.
    pub part: Option<String>,
    /// Where it is held ("Go" moves there).
    pub place: LocationView,
    /// Held on the day played (else reported as having happened).
    pub today: bool,
    /// It chose the day.
    pub main: bool,
    /// Its part is the player's part of the day now.
    pub now: bool,
    /// The player is there now.
    pub here: bool,
    /// The player has been there this turn, in its part.
    pub attended: bool,
    /// The player is expected (at most one event a turn).
    pub expected: bool,
    /// What staying away costs, when expected (`cal-cost-*`).
    pub cost: Option<TextView>,
}

fn view(engine: &Engine, h: &Held) -> CalendarEventView {
    let st = engine.state();
    let content = engine.content();
    let def = h.def(content);
    let town = h
        .town()
        .and_then(|m| st.world.municipalities.get(m.index()))
        .map(|m| m.name.clone())
        .unwrap_or_default();
    let now = mandate_sim::characters::routines::day_part(st);
    let pl = st.player.as_ref();
    CalendarEventView {
        id: def.id.clone(),
        name: TextView::new(h.name_key.clone()).with("town", town),
        date: DateView::of(h.on),
        weekday_key: format!("weekday-{}", weekday(h.on)),
        part_key: h
            .part
            .map(|p| format!("day-part-{}", p.key().replace('_', "-"))),
        part: h.part.map(|p| p.key().to_string()),
        place: location_view(st, h.location()),
        today: h.today(),
        main: h.main,
        now: h.part == Some(now),
        here: pl.is_some_and(|p| p.location == h.location()),
        attended: calendar::attended(st, content, h),
        expected: h.expected,
        cost: def
            .absent
            .as_ref()
            .filter(|_| h.expected)
            .map(|_| TextView::new("cal-cost-goodwill")),
    }
}

/// This fortnight's events in the player's town, on the day played: those
/// held, in the day's order, then any reported.
pub fn fortnight(engine: &Engine) -> Vec<CalendarEventView> {
    calendar::this_turn(engine.state(), engine.content())
        .iter()
        .map(|h| view(engine, h))
        .collect()
}

/// The next turn's event the player is expected at, if any, with what
/// staying away costs (owner's answer 2: shown a turn ahead).
pub fn next_expected(engine: &Engine) -> Option<CalendarEventView> {
    let (_, next) = calendar::next_turn(engine.state(), engine.content());
    next.iter().find(|h| h.expected).map(|h| {
        let mut v = view(engine, h);
        v.now = false;
        v.here = false;
        v.attended = false;
        v
    })
}

/// The day played, as the bar shows it: its date and weekday key.
pub fn day_played(engine: &Engine) -> (DateView, String) {
    let d = calendar::play_day(engine.state(), engine.content());
    (DateView::of(d), format!("weekday-{}", weekday(d)))
}

/// What the turn just played held (its occasions, as event lines with
/// their dates): the turn's passage tells them. Newest turn only.
pub fn passage(engine: &Engine) -> Vec<EventView> {
    let st = engine.state();
    let mut ends = 0;
    let mut out = Vec::new();
    for e in st.events.iter().rev() {
        match e.kind {
            EventKind::TurnEnded { .. } => {
                ends += 1;
                if ends > 1 {
                    break;
                }
            }
            EventKind::CalendarEvent { .. } if ends == 1 => out.push(event_view(e, engine)),
            _ => {}
        }
    }
    out.reverse();
    out
}
