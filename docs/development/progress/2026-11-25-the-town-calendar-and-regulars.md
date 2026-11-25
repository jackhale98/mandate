# The town calendar and regulars (B1)

**Date:** 2026-11-25
**Status:** built (DEC-114, `BEING_THERE.md` §10.4)

**One date per turn.** Every turn has one day played, chosen by the engine as the turn begins (`Calendar.day`, `characters::calendar::choose_day`): the date of the home town's main occasion (one the player is expected at first, then the largest crowd), else a working day of the span by keyed draw. The bar shows it with its weekday, routines use it (the rotating `representative_day` is gone), the fortnight's events are held on it, and the facts the turn leaves (`EventKind::CalendarEvent`) and the turn's passage name it. Campaign turns (7 and 3 days) the same.

**The calendar registry.** `content/base/calendar.jsonc` (types and validation in `mandate-content/src/calendar.rs`, in the schema; the doctor counts events never held): market day (every turn), council night (the `session` routine event), meeting night (as before), the summer show, the memorial (where the town had a disaster in the last ten years), the school term, and the plant's gala (where the town has a factory). Match day is not built (no club place kind). Instances are keyed by `Jurisdiction`, generated per town from the venue and `requires`, with keyed names and dates. A second event of the fortnight takes another part of the day; an occasion with none left is reported as having happened, never shown on a day the player does not play.

**Going and staying away.** Today's "This fortnight" lists the turn's events with part and place and a Go (walk, or open the place in quick mode); the next turn's expected event is shown a turn ahead with its cost. People attend by routine (each event's `attendees`, and `event.<id>` in routines' `events`); the venue fills (`crowd` passers-by) and offers the event's verbs. Absence costs only where the player is expected (a small opinion fall among those there), at most one event a turn.

**Regulars and familiarity.** `Relationship.familiar` (characters domain), built by presence (once a turn a person, more at an event with time to talk), fading monthly; shown only in words (`familiar-*`) on the person's row and sheet; the `familiar` condition; five openers that greet a familiar person by name; from "a nod" the name tag's "!" for "What's on your mind?" when they have something to say. "Usually at the diner on Tuesdays and Thursdays, in the afternoon" from their routines once seen there twice.

**Save:** v32 (`calendar_familiarity`: no change to the JSON; everyone starts a stranger and the day is chosen as the state loads).

Left: walk-up encounters for a nod (E10), "one of ours" effects, lines about the day's event (B6), recognition and the paper's line for an absence, match day.
