# Pocket rework R1: the views, with the arc view

**Date:** 2026-11-25
**Status:** done (protocol only: the people list, needs you, arcs under way, next up, places' reasons and order, notes, the arc view, their Fluent lines, tests and fixtures); the web task R2 builds on it; not done is listed at the end

`POCKET_REWORK.md` task R1 (DEC-113, structure B) with `STAGED_ARCS.md` task A3's protocol half (DEC-109). Views only: no simulation state, no save change, no outcome moves. Decisions: DEC-113, DEC-091 (and its principle of 2026-11-25), DEC-109, DEC-054, DEC-098, DEC-079.

## What the engine now says

**The engine orders; the UI only lays out.** Every order, group, reason and count the reworked pocket shows travels in a view.

| View or field | What it carries | Who orders it |
|---|---|---|
| `QueryDto::People` → `PeopleView` | Every person the player has heard of as a `PersonRowView`: who, role, the one-chip stance (`band` and `shape`), section (`group`), `rank`, why they matter now (`now`), `go`, `usually`, `last_seen`, `note`, `new_this_turn`, `level`; and each section's count and how many it shows (`groups`) | The engine: sections `now`, `office`, `rest`, `gone`; in `now` the soonest deadline, then who is here, then by reason; elsewhere most recently seen first |
| `NowReasonView` | A reason: `kind`, a `now-*` line, a `title` in the matter's own words, what it opens (`ask`, `arc`, `decision`, `bill`), `due`, `days_left` | Strongest first; one a matter |
| `PocketView.needs_you`, `needs_count` | `NeedView`: pending decisions, asks to answer, votes to cast, accepted asks due within the turn, a known arc's beats in their last turn before its clock | Soonest first |
| `PocketView.arcs` | `ArcSummaryView`: each running arc the player knows, its stage's line, its clock, how many beats are open | The soonest clock first |
| `PocketView.next_up` | `NextUpView`: the next dated thing ahead (a vote on the player's bill, a promise falling due, an arc's clock, election day) | The soonest |
| `SpotView.now`, `rank`, `note` | A place's reasons (a decision's scene, an ask's next need, an arc's visit, the player), its order in the town, its note | Places that matter first; the rest as the town lists them |
| `note` on `PersonRowView`, `TownHoodView`, `SpotView`, `BillView`, `PocketBillView`, `NotebookEntryView` | One sentence (`know-*`), only where a gap or a fact changes something the player could do | |
| `QueryDto::Arc { id }` → `ArcView` | One run: title, stage in words (with the event to ask "Why?" about), clock, turn, cast, the beats that are the player's to take with what each needs (a place, a person, a power, time) and the command that takes it, what has happened, the latest headline; an ended run with how it ended | Beats: what has a deadline first |
| `NotebookView.arc_leads` | `ArcLeadView`: a lead toward each running arc only heard of | Oldest first |

## What the player knows of an arc

The ledger has no arc subject, and this task adds no state. The level is derived (`mandate_protocol::arcs::arc_level`): `known` when the run's stage is not hidden (the break never is), when a decision or an ask of the run is open or was (aimed at the player: never gated), or when a beat the player took teaches it; `heard` when the player took a beat of a hidden stage, or overheard one of its lines since it began, and that teaches no more (the default); `detailed` only by a beat's `teaches` or the lab switch. So `hidden` and `teaches`, parsed and inert since A1, are now read, by this view and nothing else: no edge, raise or move reads the level.

- Below `known` `ArcView` is a lead and nothing else: where to ask (`know-arc-lead`, a neighbourhood's or a place's name), no title, no stage, no cast.
- An arc the player knows nothing of reads as no such arc (`QueryDto::Arc` is refused the same way), has no row on Today, no reason on anyone and no need.
- From `known` the arc names its cast, as its stages' lines already do wherever they are told; each person's own `level` travels.

## Knowledge in words

`mandate_protocol::notebook`: seven lines, each tied to something that changes. A person who matters now and is only a name; a member with a vote on the player's bill whom the player knows too little for full lobbying odds (`knowledge.effects.lobby_logit`); a neighbourhood where a canvass is in hand (an ask needs it, the player is there, their race is fought there) and whose worries are unknown; a bill that is the player's to move or vote on with support known only as a band; a reading older than `knowledge.fresh_days`; a place that has shut; the lead toward an arc. Elsewhere there is no note. `level` stays in the views for tests.

## Text

`content/base/locale/en/ui.ftl`: 21 `now-*` lines (34 characters at most; a reason names nobody and takes no argument but `$days`) and 7 `know-*` lines (whole sentences). The engine's lines are two enums (`NowLine`, `KnowLine`); a test holds each equal to the file's keys by prefix.

## Tests

- `mandate-protocol` (9 new unit tests): Fluent coverage both ways, the 34 characters, whole sentences, the clock's line by days, reasons ordered and one a matter, what a beat teaches, a power read from a beat's conditions.
- `mandate-wasm/tests/pocket_views.rs` (20): one query lists every person in sections with contiguous ranks; an asker's row against their sheet; needs you soonest first; a beat needs the player in its last turn; arcs under way and next up; places' reasons and ranks; notes only where they change something (a neighbourhood, a person, a bill with an old reading, a shut place); a vote on the player's bill away from the chamber and in it; a known arc, a talk beat, an ended arc; below `known` a lead only; no reason, note, need, arc view or notebook line names what the player does not know (an empty ledger, three worlds; places have no ledger subject and are public); the same twice, through a save and load, and in a second game; the K4 guard (a default ledger, the lab's, and a game never queried: one world); the session's two queries; the fixtures.
- Fixtures for R2, written by the test (`MANDATE_WRITE_FIXTURES=1 cargo test -p mandate-wasm --test pocket_views fixtures`), read back as today's types on every run: `web/src/components/fixtures/pocket/` (`people`, `pocket`, `town`, `notebook`, `arc`, `arc-heard`, `notebook-arc-lead`, `arc-ended`).

Native cost a call in a small world, measured once by hand: pocket 0.6 ms, people 0.2 ms, town 0.8 ms, notebook 0.4 ms, arc 0.3 ms.

## Not done

- **The story sheet, the log and the paper do not gate arcs.** `StoryView.storylines` (`narrative.rs`) and an arc's events list every run whatever the player knows. No arc content has a hidden stage yet; the first that does needs `arc_level` there too.
- **`mandate_content::arcs::inert`** still lists `hidden` and `teaches` as inert (that file is another task's).
- **A scene and a case are not needs.** `NeedKind` has no `scene` (the exchange scene's view is X1's) and no `case` (a case asks the player through a decision, which is a need already).
- **The five short stance words** ("Warm" beside "Warm toward you") are the web task's strings: the row gives the band and the shape.
- **No `look` on a row:** the portrait is drawn from the person's id, as the talk sheet does now.
- **A member's lean outside the chamber is not known** (DEC-079), so there every member the player has heard of has "Has a vote on your bill"; in the chamber only the undecided have a reason.
