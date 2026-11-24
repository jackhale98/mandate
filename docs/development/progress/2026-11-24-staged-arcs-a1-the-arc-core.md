# Staged arcs A1: the arc core and the flood arc

**Date:** 2026-11-24
**Status:** done (the engine and one arc; no director scheduling, no arc view, no stand-ins or grader, no fuses: tasks A2 to A5)

Design: `docs/design/STAGED_ARCS.md` (DEC-109); what the build corrected in it is marked **(A1)** there and listed in its §12. Architecture: OVERVIEW §4.2 to §4.4, DATA_MODEL §6.1. Authoring: EXTENDING, "Adding an arc".

## The arc core

- **A domain of its own:** `mandate-sim/src/arcs/` (`State.arcs`). A run is keyed by a `Jurisdiction`, has a cast bound to real things (people by occupation or by a power their post lists, an area one level down, a place of a kind, the tier above), a stage, a clock, a heat and marks. It is raised when its template's `raise` holds in a focus jurisdiction and its cast binds; at most one runs per jurisdiction until the director schedules arcs (A2).
- **Transitions read; beats write.** Edges fire on conditions over simulated state, the run's marks, heat and days, and the clock. A visit, a reply, an answer or a kept ask sets marks and posts effect requests; nothing a player picks names a stage or an exit.
- **Other domains only through requests,** each with its cause: `Opinion`, `Favour`, `Record`, `Approval`, `Displacement`, `PlayerTrack`, and three new ones in their owners' modules: `Recovery` (environment: repairs run faster for a while), `OpenDecision` (narrative) and `OfferAsk` (asks). The step runs first in the day, so each is applied the same day.
- **All seven beat kinds work:** a decision (a dilemma with the run's cast), an ask (from a cast person), a talk choice (the talk hook, owner `arc`), a visit (the `Visit` action with the beat's verb, at the cast place), a story (the town's outlets may run it), an overheard line (it holds where the run is), a cast person's move (a keyed weekly draw).
- **On the spine:** `Thread::Arc`. The strike is the run's root; the raise opens the thread, each stage continues it, each beat notes it, the exit closes it. Every request and event names its cause.
- **Not yet:** a template that uses a `subject` cast, a clock read from the subject, `gov_approval`, `place_state` or any `requires` validates and is never raised (`arcs::not_yet`; tested).

## The flood arc, from the day the water comes

`content/base/arcs/flood.jsonc` (one file: the arc, its two decisions, its ask, its scenes, its overheard line) and `locale/en/arcs.ftl`. Stages: struck, counting, meeting (the turn), rebuilding; exits: defended, rebuilt, hollowed. No warning stage and no "passed" exit until the forecast (A5b).

## Evidence

- **Raised:** a flood the lab plants raises the arc in 8 seeds of 8 (`lab/arcs_games.rs`).
- **Agency, in one seed** (seed 7, `mandate-worldgen/tests/arcs.rs`): the passive script ends *hollowed* after 14 turns; the engaged script (carry boxes, get a second roof opened, press for the money, ask for a crew) ends *rebuilt*. On the day the passive run ends, the engaged world has fewer damaged homes in the struck neighbourhood, higher approval there, and an organiser who remembers the player stood with them where the passive world's remembers being ignored.
- **Parity (DEC-051):** every beat a player takes is reached in quick mode (one routed request) and through Places (walk, then act), by the same commands.
- **No arc content, no change:** all eight pinned games of the spine proof give their digests from before arcs. With the flood arc, the two seed-31 games move (a flood strikes the player's town) and were re-recorded with the reason.
- **Determinism:** a run in progress survives save and load; a lab fork with a run in progress continues as the original; a v26 save (`v26_town.mndt`, written by the branch head ee9aa67) loads and plays on. Native and WASM agree on seed 31 (small, 120 turns), a game whose flood arc runs from the strike to *rebuilt* (`c3d111bf…`), and on the standard seed 777 (`3df789d3…`).
- **On the tree with audit fix 1** (integrated after the votes change): every arc test passes unchanged, with no threshold moved; with no arc content all eight proof games give fix 1's pins exactly, and with the flood the two seed-31 games move (13 and 17 more events); native and WASM agree on seed 31 small 120 turns (`7d87bd6c…`) and seed 777 medium 60 turns (`2d33602f…`); `bots --seeds 16` is byte-identical to fix 1's.
- **Bots:** `bots --seeds 16` is byte-identical to the branch head's (no flood strikes a bot's town inside its horizon); the seven enforced gates pass.
- **Cost:** the arcs' own step is 2.9 µs weekly and 0.75 µs daily with a run in progress: 16 µs of a 2.5 ms governing turn (0.65%). Whole turns with the run against the same world with the run taken out differ by less than two identical copies do on the build machine (−3.0% against −5.1%). A run adds about 0.8 KB of JSON to the state (about 200 bytes to a 750 KB save); an ended run leaves a 0.6 KB summary, and the last 40 are kept. (The timings come from a throwaway harness: the workspace's determinism lint forbids a wall clock in its crates, tests included, so none is kept in the repository.)

## Save

Format v27. The arcs domain is not written while empty, so a game no arc has touched hashes as it did. The step v26 to v27 rewrites nothing and refuses an older payload that carries arc state.

## What is left

A2 (the director on arcs, storylines folded), A3 (the arc view: today the surface is the event list, the place's verbs, the conversation's choices, the pocket's decisions and Today's asks), A4 (stand-ins, the grader, the doctor's dynamic lints), A5 (the fuses, `treasury`, `gov_approval`, `place_state`, tier-generic `disaster` and `law_in_force`).
