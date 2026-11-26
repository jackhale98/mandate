# Upkeep with faults, and casework on faults (B2)

**Date:** 2026-11-26
**Status:** built (DEC-114 §3, `BEING_THERE.md` §4 and §10.2's B2 row)

**The upkeep domain.** `mandate-sim/src/upkeep/` owns `UpkeepState` (`State.upkeep`, not written while empty): a condition (0–1) per neighbourhood of the player's focus, and faults at spots on its streets (a kind, a neighbourhood, a slot of the kind's spot kind, the day it opened, open / being repaired / fixed, and what the queue did to it). Monthly (Population phase, after the month's budget, before the worries are read): each budget line's repair queue is paid with `repair_share` of the line's month in the town's own treasury (faults moved up first, then the oldest, until the next does not fit; a job dearer than a whole month's money takes the month at the head); then wear (`condition += maintain × min(Infrastructure line / need, maintain_cap) − decay`); then new faults where the condition is below a kind's `raise.below` (keyed draws, × winter), at most `max_open` a neighbourhood. Daily: repairs finished (the spot looks new for `new_days`, the condition gains `restore`), faults moved up (`Effect::Expedite`). No money moves: finance spends the line as ever, and this is what part of it buys.

**Content.** `content/base/faults.jsonc` (types and validation in `mandate-content/src/faults.rs`, in the schema): the rules and five kinds (street light, blocked drain, pothole, bus shelter, broken swing), each with its asset, spot kind and slots, raise, the line that pays (`infrastructure`), repair cost and days, restore, the worry it feeds (`concern.services`), three cues (open, repairing, new; drawn in B4) and its Fluent prefix. English in `locale/en/upkeep.ftl`.

**Casework on faults.** `Need::Fixed { fault }` (content `{ "type": "fixed", "role": … }`): cast on the oldest fault waiting on the streets of the role's neighbourhood (a person's: their home) that no open ask is about; met when the fault is fixed, whoever fixes it; an offer whose fault is mended first is withdrawn (`ask-void-fixed`). `ask.fault` (in `asks.jsonc`, before `ask.casework`, from a doorstep, a talk or the week) is a resident bringing a fault near their door. "Sort it out" (`Action::Casework`, the services desk, one slot, today's odds) on a fault moves it up its queue once (`asks::can_casework`); it does not buy the repair (owner's answer 3). In the month it is paid for ahead of a fault that would have been, that fault is put back: `Fault.ahead_of` and `put_back_for`, and the events `fault` (`opened`, `expedited`, `repairing` with the fault it went ahead of, `delayed` linked to that repair, `fixed`) on the thread `Fault(id)`, rooted in `Process::Upkeep` (wear), `Process::Budget` (the line paid) and the player's casework. An ask kept by a repair links to it.

**The services worry reads open faults.** A neighbourhood's felt level of the worry a fault feeds gains `worry_per_fault` for each fault not yet repaired there (within the concern's range); the town's felt level follows as the residents' mean. The measured levels (politics, salience) are the indicators' as before.

**Views and web.** `FaultView` (`mandate-protocol/src/upkeep.rs`): what is wrong and its name (keys), the neighbourhood, the spot (kind, slot, slots), the status and the cue to draw now, dates, moved up, next in the queue, the queue's note (`ui-fault-ahead`, `ui-fault-put-back`), "See it" (the streets' location) and its latest event for "Why?". `AskView.fault` (Today's ask) and `PlacePanelView.faults` (a neighbourhood's streets). The web shows both (`components/faults/Faults.tsx`: the line in the ask's body with "See it", the street panel's "On these streets"); B4 draws the cues.

**Save:** v33 (`upkeep_faults`: nothing to rewrite; the domain starts empty and the focus's condition is kept from the next month).

**Departures from §4, the smaller sound option each time:**
1. `raise` is a threshold, a chance and a winter factor, not U1 conditions (no condition kinds over upkeep exist; adding them touches `condition.rs` in two crates).
2. "Sort it out" on the spot for half a slot while holding a post is not built: the services desk only, at today's casework odds (in office 0.85, out 0.5), so the odds shown stay the odds rolled.
3. A fault moved up waits for the month's turn of the queue (it is not paid for at once), so a deadline is drawn in 42–63 days to cover one turn and the repair.
4. Focus towns only, as the calendar (other towns' services worry is as before).
5. Faults feed the felt worry only; the town's measured levels (its politics) are unchanged.

**Story (DEC-109's addition).** No arc is written here. Faults can be an arc's cause: a neighbourhood whose faults stay waiting for months (a budget cut, or a queue the player keeps jumping) is a condition an arc can read ("the streets are going dark": residents organise, the paper runs the dark corners, a meeting at the town hall where the player answers for the queue, played as an exchange), and its push is a budget vote on the Infrastructure line (`Effect::Budget`), which the queue turns into repairs a month later.

Left: the street's art (B4), lines (B6), faults outside the focus, a resident's line when the fault is still out at the deadline beyond the ask's broken text.
