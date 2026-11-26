# Pilot arc: the zoning fight, and what an arc costs

**Date:** 2026-11-25
**Status:** done. One new arc on the machinery X1–X4 left, with one small general engine change, and a ledger of what it took. The owner's question: will adding arcs stay as hard as X1–X4 were?

Design: DEC-109 (and the owner's additions), DEC-112, `EXCHANGE_SCENES.md`, DEC-091, DEC-098.

## The arc

`content/base/arcs/zoning.jsonc`, `arc.zoning`. A builder wants homes in a neighbourhood whose rules will not let them build that high.

- **Raised** by the town's own housing market: housing is the town's strongest measured worry, and no law that loosens the rules or builds homes is in force. The builder's scheme is the fiction of what the economy already models: private building answers price and vacancy and is scaled by zoning (`policy.upzoning`). Only a law changes the rules, so the arc is about a bill.
- **Cast** from live state: the area least pleased with the player, a business leader there (the builder), an organiser, teacher, nurse or student (wants the homes), a resident, retiree, cleric or shopkeeper (fears the change), a hall near it, and an optional member of another party whose post has `approve_ordinance`. No office or body is named.
- **Stages:** *plans* (clock 28–42 days: the paper, a heard line, walk the site, the builder's drawings, the neighbour's objections) → *split* (21–35: the neighbour asks for a word in public, the organiser can be asked to speak, a petition if nobody listened) → **meeting** (the turn, 14–21: the scene; the organiser asks for the bill; a member's vote can be asked) → *chamber* (56–84: the body votes) → exits *upzoned*, *public_homes* (a law in force), *voted_down* (a bill went forward and failed), *kept* (no bill, the player spoke for the street), *shelved* (nobody did anything).
- **The played turn:** a hostile meeting at the hall: the neighbourhood by income class, the neighbour who called it and the builder; temper is the run's heat; the organiser may speak for the player. Walking the site removes the cold first round; the drawings are a `proof` line (evidence 0.8); the organiser's word is the `second` line. The prompts are the player's record (a promise broken, a failed law of theirs, an empty chair) and the room's question.
- **Leads knowledge opens:** the site visit teaches the area's worries (the `heard` line); a builder who sees you walked it shows the drawings (to `detailed`); the neighbour, asked, says who is with them (to `known`); a member asked says how they will vote.
- **The spine:** the raise is the arcs process (no event made it true: see the gaps); every beat, scene round and closing event is on the run's thread; the bill is the player's own `Introduce`; and an edge that reads a law now names its enactment (below).

## A play-through: seed 2 (Vellenbridge, Oldlands)

Housing is Vellenbridge's strongest worry on the first day of play. Oldlands is cast, with Imre Gallowayby (business leader), Jasper Novaker (organiser), Osric Kellwell (shopkeeper) and Flora Leachwell (a member of another party).

- **Engaged** (the engaged stand-in's levers; in the meeting, the line with the best change shown): day 11 walks the site, sees the drawings, hears the objections; day 39 the organiser agrees to speak; day 42 a word in public for the street. Day 60, the meeting: round 1 the room's question, "I will table affordable housing"; round 2 the neighbour puts the player's own failed hardship-fund bill to them (it failed 1–6 on day 35), the drawings shown (stood); round 3 "I will table upzoning". **Won**, +170 ‰. The organiser's bill goes in on day 67 and fails 1–6 on day 91: **voted_down**. Organiser +5, neighbour +16, builder −8.
- **Passive:** an empty chair (−317 ‰), both asks lapse, nothing goes forward: **shelved**. Organiser −38 (`ignored_me`), neighbour −16, builder −17.
- The same engaged player who leads with the drawings loses the meeting (−412 ‰) when they do not stand up, a 20% risk shown on the line.

## What the grader says (`arcs grade --arc arc.zoning --seeds 16`)

| | |
|---|---|
| Reach | 11 of 16 worlds, all raised by the world (never raised: 1, 6, 12, 14, 15) |
| Paths | coverage 1.000 (enforced at 16 seeds): every stage and exit visited |
| Passive and engaged | exits differ in 11 of 11; passive always *shelved*; engaged *voted_down* 0.545, *kept* 0.182, *public_homes* 0.182, *upzoned* 0.091; contrarian *kept* 0.636, *public_homes* 0.364 |
| Terms (median) | agency 0.804, escalation 0.599, visibility 0.600, pacing 0.750, variety 0.000 |
| The scene | every graded meeting is an empty chair: the stand-ins do not play scenes (X3). Played by hand on seeds 2, 3, 4, 7: prepared, best line shown: won 2, split 1, lost 1; cold, best line: split 4 |
| Bars | all four report-only bars pass; the engaged median reaches the turn in 5 turns |

All arcs together still pass `arcs grade --seeds 16`. The flood is raised in 13 of 16 graded worlds with this arc and 16 of 16 without: the zoning run holds one of the two places for arcs (`TUNING_QUEUE.md`).

## The ledger

**Wall clock** (one builder, 2026-11-25, UTC): reading and design 10 min (the CLI built alongside, 5 min); content 3 min (the JSONC validated first time, bar the text); text 4 min; grading, the raise and the lints 6 min; the engine gap 8 min; tests 7 min (passed first time); fixing 15 min (a hand-written probe of the scene: the bill was voted before the meeting, and the scene could open and close inside one fortnight turn); checks, pins, two wire tests that met the new arc, and this entry about 1 h 15 min more, most of it waiting on builds and the 16-minute repin. About an hour to a green arc; about two and a half hours to a commit with its pins.

**Files and lines** (added): content JSONC 303 (82 of them comments); Fluent 116 (`arcs.ftl` 67, `exchange.ftl` 49); Rust engine +49 −8 (`arcs/step.rs`); Rust tests 403 (`mandate-worldgen/tests/arcs_zoning.rs`: about 85 lines of helpers copied from the X4 tests, about 125 a harness that plays the scene, 190 the four tests); web 0 (fixtures re-recorded only); edits to other tests +49 −12 in seven files.

**Gaps:**

| Gap | Built? | Lines | General or specific |
|---|---|---|---|
| An edge that reads a law did not name the law on the spine: the Why panel said only "a matter running its own course" for *defended*, *answered*, *upzoned* | built: the stage or exit an edge with a top-level `law_in_force` enters is also caused by that law's `PolicyEnacted` | +49 −8 | general: every arc whose edges read a law (the flood, the plant, this) |
| A raise on a standing condition (a worry, an indicator) has no root event: the Why panel cannot say "because rents rose" | no | est. 40–60 and a save bump (a root naming a concern; `Pressure` is `Copy` and saved) | general |
| No condition reads a bill before the body by template, pending or failed: *voted_down* is the arc's own mark and a wait (49 days), and an NPC bill pending at the raise can end the arc in its first turns | no | est. 40, plus a `PolicyIntroduced` root | general |
| `the_matter` prompt source (the room asks about the arc's own worry) is not built; the room asks its strongest felt worry, usually housing here, not always | no | est. 30–40 | general (every hostile meeting) |
| An area cast by worry (`most_worried`): the neighbourhood is the least pleased, not the one most squeezed | no | est. 25 | general |
| A named test seed: `tests/seeds.json` needs a requirement in `lab/seed_reqs.rs` (owned elsewhere): the test pins seed 2 with a comment, as X4 did | no | 10–20 there | general |
| Validation does not ask for a scene's `<text>-title`, which the protocol names: found only by the wire test (`scene_views`); the flood's and the plant's scenes have none in `exchange.ftl` | no (this arc's title added) | 1 line in `arcs.rs`, 2 Fluent lines | general |
| A scene promises four or five lines (§3.3), but a thin state gives fewer: a cold player in this meeting is offered three in round 2 (act, stay, move on) | no | engine or content lint | general |

**Tooling.** Helped: `content validate` lists every missing Fluent key and checks each key's arguments against what the arc gives, so the text was written against a list; `arc-mark-unread` caught a mark no exit read; `arcs grade --arc` answers in 4 s for 4 seeds and 15 s for 16, with coverage, so the raise was chosen by measurement; the doctor's `arc-names-office` caught "town" three times (a word of the body's own name, which was a surprise). Hurt: the grader cannot see a scene (stand-ins leave empty chairs), so the turn needed a probe written and thrown away; the X4 tests' helpers are copied, not shared; `arc-names-office` does not read the scene's own `exchange-*` text.

**The next arc.** On systems that exist: about an hour of focused work for the arc and its tests, then an hour or more for the other arcs' tests it meets and the pins, 250–300 lines of JSONC, about 100 of Fluent, 250–350 of tests, and 0–50 of engine. Where it needs a simulated cause the engine lacks (DEC-109's "causes get fuses"; the M7 systems), the cost is that system, not the arc. **The single change that would cut it most:** one shared arc-test harness (`only`, `raised`, `play`, `meet` with the scene policies *away*, *cold*, *best shown*) whose worlds hold the arc under test alone unless a test asks for company, used by every arc's tests. It would have kept this arc out of the other arcs' twelve tests, removed most of this arc's own test file, and with the stand-ins playing scenes by those policies (X3) it would let the grader measure the turn without a hand-written probe. What it leaves is the real interaction (places for arcs, one scene a chapter), which belongs in one test of the director, not in every arc's.

## Pins and tests outside this arc

`scripts/repin.sh` moved the spine proof's seven pins (more events: the zoning run in each world), the state-shape fingerprint (no struct changed: v31 stays), the asks web fixture (ids), and `flood_arc_exits_idle_by_month_12` from seed 7 to 8. The exchange fixtures were re-recorded (small changes).

**The largest hidden cost: other arcs' tests.** The full test run found the new arc in twelve tests of seven files, none of them its own. Most tests use the whole base pack and assume their arc meets no other: there are two places for arcs a jurisdiction, at most one scene a chapter, and a material arc leads. Two things were done:

- **The zoning fight is a `scheme`, not a `crisis`.** As a material arc it led session chapters over the budget round and campaign chapters over the rival, and its meeting took the chapter's one scene, so the debate could not be held. A matter of rules is not an emergency: it now runs beside the standing arcs and does not outrank them. It still holds one of the two places, so it can keep a flood out (the open item below).
- **Tests of other arcs now look for their own arc:** the debate's tests (`mandate-save/tests/exchange.rs`, `mandate-wasm/tests/scene_views.rs`) wait for the debate, not the first scene; the flood's view tests (`mandate-wasm/tests/pocket_views.rs`) and the flood-beside-the-round test (`budget_round.rs`) use the pack without the zoning fight; a registry-order assertion (`mandate-content/tests/arcs.rs`) names the arc files that sort after the flood; the v10 migration test's payload drops `arcs`, which a v10 save never held. The pacing bar's asked-in-office share (`narrative.rs`) is report-only, with the figures (15 of 20, 12 of 12 before; `TUNING_QUEUE.md`).

## Not done, and open

- **One test fails, in a file this task may not edit:** `mandate-cli` `lab::arcs_games::a_planted_flood_raises_the_arc_in_eight_seeds_of_eight` (7 of 8: on seed 7 the zoning run, raised at the start, holds the second place for arcs when the planted flood comes). It is the crowding the grader shows (the flood 13 of 16). The one-line fix belongs to the lab's owner: give that test the base pack with the flood alone (as `mandate-worldgen/tests/arcs.rs` does), or count a flood held back by the cap as raised-but-held. The design question behind it is the lead's: whether a disaster may wait behind a standing worry for a place, or should take precedence (DEC-109 §4.2's cap).

- The frequency of the raise and the meeting's balance are the tuning pass's (two rows in `TUNING_QUEUE.md`).
- The flood's and the plant's meetings have clocks of 7–14 days: on a fortnight turn the scene can open and close between two turns, never offered (it happened to this arc on seed 7 before its clock was widened to 14–21). Not changed here (not this task's files).
- The `act` line is often the best line shown, and can be said twice in a scene on two templates: watch for a dominant line at the tuning pass.
