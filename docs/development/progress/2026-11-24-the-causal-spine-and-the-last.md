# The causal spine (S1) and the last post held

**Date:** 2026-11-24
**Status:** done (the engine, the protocol query, the lab metric); no UI, by design

Design note: `docs/design/CAUSAL_SPINE.md`. Architecture: OVERVIEW §4.3 and §4.4, DATA_MODEL §7.

## The spine

- **Causes everywhere.** `spine::Cause`: a typed link to an earlier event (`caused`, `continues`, `prompted`, `revealed`, `ended`) or a root (the player's act, someone's own choice, a scheduled process, a named pressure, worldgen, an explicit exogenous mark, a lab intervention; `unknown` only for events from before the spine). Every `EventLog::record` takes one or more; every effect request is `Request { effect, cause }`, and its applier passes the cause on.
- **Threads.** The log links each event to the one before it on its matter (a bill, an ask, a case, a recession), so sites name only roots and links across threads. No domain state grew an event id for it.
- **What sites say.** Disasters, recessions, commodity shocks and the race shock are exogenous and say so. A plant closure names the weakest factor of its sector's demand (world demand, commodity prices, depletion, competitiveness, labour costs) and links a running recession. A scandal names the act that left the evidence. A case links its scandal; a sentence, its conviction; the player's vote is a step in a bill's life.
- **The bound.** Pruned about monthly as a turn ends: bookkeeping a year, routine events four, notable ones twelve, landmarks for the run; never what live state holds (laws in force, open asks, decisions and cases, towns in crisis, the Legacy record) or what a kept event descends from; a cap of 6,000 as a backstop. It replaces "the last 2,000 plus every pin, forever".
- **Queries** (`spine::graph`): the chain to the roots, consequences within N links, the nearest player act, patterns along a chain, with two worked sifting patterns as tests.
- **Protocol:** `QueryDto::Why { event, depth }` → `WhyView` (facts and the engine's existing event text keys; generated TS). The director's levers never show.
- **Lab:** `audit readability` reports `causal_spine` for every event kind: explained, exogenous, process-only, unnamed.

## The last post held (M6 note 16)

`Politician::last_post` (the post by constitution id, and the day it ended) is set on leaving office; `CharacterView::last_post` shows it for someone out of office, from `known` on, with the post's label and role word keys, the years in office and the end date. No text was added. A later text pass needs lines for it on the character sheet and in people lists (suggested keys: `sheet-last-post` with `$role`, `$post`, `$place`, `$year`; `sheet-last-post-years` with `$years`; `role-former` with `$role` for a short label).

## Save

Format v26. `causal_spine` migrates v25: old causes become typed links, events with none get `unknown` (nothing is invented), thread heads are replayed, pins the rule replaces are dropped, and `last_post` is filled where the save's election results show it. New fixture `v25_town.mndt` (written by the build before the spine).

## Evidence

- **No outcome changed.** `lab/proof.rs`: eight games (three seeds, idle and one bot each, eight years; and two whose player sits in prison, four years and six, by a lab edit). The event stream (ids, dates, kinds) and the state less the log and the cause fields equal the digests of the build without the spine, with pruning on and off: first at e7916f9, then again at c72cbbd (after the rents fix changed outcomes), built as it is, without the spine. `bots --seeds 16` output was byte-identical to the branch head's at e7916f9. The v25 fixture played on twelve turns equals the old build's own save twelve turns later (state and events), checked once by hand.
- **Coverage** (`audit readability --seeds 4`, six scenarios, 20 years, idle; 24 runs): every kind names a cause (unnamed 0%). Plant closures explained 100% of 304 (29% before); laws enacted, failed and expired 100%; scandals name an act 100% of 1,213 (0% before); disasters, recessions and commodity shocks carry the exogenous mark 100% (no cause before); races carry the race shock's mark and link their campaign. Process-only by design: campaign openings, budgets, rate moves, chapters, and 97% of decisions opened (the director's spotlight; the rest answer a story, a case or someone's request).
- **Save size**, 30 years, medium world: 2,126 KB → 2,115 KB; the log 4,958 → 1,722 events.
- **Time:** world generation 1.118 s → 1.093 s (medium), a turn 3.91 ms → 3.90 ms (medium), 1.725 ms → 1.727 ms (small): within noise.

## Pins that moved, and why

- `saves.rs` shape fingerprint: v25 `c8db676e2177925f` (the branch's, after the rents fix) → v26 `740f115b1c8d9bf3` (the state's shape).
- `v18_campaign_continued.mndt` re-recorded: equal to the previous golden apart from the log and the new fields; the log a subset of the old one.
- `determinism --seed 2026 --turns 80` and the parity hash changed with the state's shape.

## Review fixes (same day)

An independent review found one hazard and four defects; all fixed, each with a test.

1. **Nothing unread is pruned by age.** The director reads nothing while the player is in prison and the whole term on release; the rule now keeps everything from the oldest live reader's cursor on (asks, press, director, Legacy, a skip). Under the engine's own rules a four-year sentence lost nothing even before (the director reads in the turn of the release); a six-year lab term did, and is the proof's eighth game.
2. **Why never shows a hidden event**, at any depth or in any run.
3. **Each held hazard links its own hold** (`DeferredHazard::hold_event`); the per-town thread is gone.
4. **A lapsed proposal prompts no bill** (`EventLog::open_head`).
5. **Why names people only as the player knows them**, in a step's arguments too.

The web build's precache guard is 10 MiB (the stripped engine is 8.43 MB, 2.02 MB gzipped; it was 65 KB under the old 8 MiB guard before the spine). A size pass is planned with the performance work.

## Left for the consumers

"Why?" surfaces lay out `WhyView` and add Fluent for links and roots (keys listed in the design note). Credit and blame (V1) reads `nearest_player_act`. Sifting (W3) turns `graph::matches` steps into conditions with bindings. The proof's pinned digests are a golden: an outcome change on purpose re-records them.
