# One date per turn and the town calendar (B1)

**Date:** 2026-11-25
**Area:** routines (where people stand), the calendar, familiarity; the bot ladder

A new mechanism, not a tuning (`AGENT_CONTEXT.md` rule 11): DEC-114's one date per turn replaces the rotating representative day, and calendar events pull their attendees to the venue in their part. Numbers are first values (`calendar.jsonc` chances and crowds, `characters.jsonc` `familiarity`), to be settled at the tuning pass.

## Parameter and change

| Parameter | Value | What it does |
|---|---|---|
| the day played | main occasion's date, else a working day by keyed draw | replaces `representative_day` (first day + turn mod span): weekday rules hold on that day |
| attendees | per event, chance 0.1 to 1.0 | tried before a person's routines while the event is held, in its part |
| `crowd` | 2 to 8 | extra passers-by at the venue while held |
| `absent.opinion` | −2 (show), −3 (memorial) | among the stored people there, only where the player is expected |
| `familiarity` | face 3, nod 6, ours 10 (opinion > 20); +1 a turn present, +2 at an event with talk; −1 a month unseen | words only |
| farmer, farmhand | their Saturday market rule removed | market day is the calendar's (every turn, morning) |

## Evidence

`mandate-cli bots --seeds 16` (release), before (the branch base, `de933ac`) and after:

| Gate (enforced?) | Before | After |
|---|---|---|
| idle_cap (report-only already) | fail, idle 0.1875 | fail, idle 0.1875 |
| effort_matters (yes) | pass, best 1.0 | pass, 1.0 |
| exploring_alone_never_wins (yes) | pass, 0.1875 vs 0.75 | pass, the same |
| ladder_rote (yes) | pass, mixed 0.7469 / 1.0 | pass, 0.7468 / 1.0 |
| ladder_learned (yes) | pass, +0.0199 | pass, +0.0181 |
| knowledge_matters (report) | fail, 0.7469 vs 0.7522 | fail, 0.7468 vs 0.7528 |
| skill_gap (report) | fail, 0.0314 | fail, 0.0365 |
| good_styles_vary (report) | fail (c) | fail (b and c) |
| corrupt_not_dominant (yes) | pass | pass |
| proxy_not_optimal (yes) | pass, 0.659 vs 0.770 | pass, 0.659 vs 0.768 |

Mean vote share, before → after: explorer 0.7668 → 0.7648, explorer campaigner 0.7688 → 0.7648, explorer coalition 0.7783 → 0.7833, explorer legislator 0.7710 → 0.7721, mixed 0.7469 → 0.7468, directed 0.6591 → 0.6588, explore only 0.372 → 0.373; idle, canvass, media, asks only, legislator, proxy and corrupt unchanged to four places.

## Effect

No enforced gate moved; no gate was made report-only. Strategies that seek people out (the explorers) move by up to half a point either way, because people stand elsewhere on the day played (working days instead of the rotating weekday; the market full in the morning) and familiarity creates relationship edges the director reads. Strategies that never go to people do not move. `good_styles_vary` clause b now fails as well (report-only before this task).

The calendar in the 2-year test worlds (`crates/mandate-wasm/tests/calendar.rs`, seeds 1 to 8): market day held 51 to 53 turns, the sitting 6 to 25, the meeting night 0 to 16 (towns without a community centre have none), the show 2, the school term 4, the plant's gala 2 in 7 of 8 towns, the memorial 2 in 1 of 8; none reported (no two occasions met in one turn).
