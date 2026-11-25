# The impact estimate, first build

**Date:** 2026-11-25
**Status:** tasks 1, 2 and 3 of `docs/design/IMPACT_ESTIMATE.md` §10 are built (DEC-111). The estimate's direct counts, cost, signs and overall calibration hold on fresh seeds; its finer honesty rules are reported, not enforced, with figures. Tasks 4, 5, 7 and 8 are not built.

A bill, before it passes, says what it will do: direct counts by the simulation's own arithmetic, what reaches the town by coefficients fitted on twins, each outcome as a range. No game is forked when a bill is opened. What the bill said is kept with the law when it passes.

## What is built

- **One arithmetic** (`crates/mandate-sim/src/policy/impact.rs`). The scale of a law, a mechanism's size, and a month of its ledger (`book`) are functions that `mods_for`, `implementation`, `delivered` and the estimate all call. `project` runs a bill's months forward on them: funding by finance's two rules, now pure functions (`finance::first_share`, `capped_share`, `room`), the ramp, and the ledger as it would read at 12, 36 and 60 months. No outcome moved: the spine proof's pins held before the frozen estimate was added, and its event streams hold after.
- **The estimate.** `ImpactInputs` holds a jurisdiction's published statistics and is the only thing the estimator reads besides the bill and the content; it takes the law's `Jurisdiction` (`inst.at()`) and returns nothing for a tier the table does not cover. Outcomes: homes, jobs, a named sector's jobs, rent, low incomes, debt, the service level, approval (all, and by class). Direct counts: homes started, extra private building, jobs supported or created, money paid, the wage uplift, the service gain, tax points, the rent cap, protection against a hazard, labour supply. Side effects by arithmetic. Who gains and who loses. Cost, where it comes from, the funded share in the first year and after.
- **The table** (`content/base/impact.jsonc`, kind `impact_table`, written by the calibrator; `crates/mandate-content/src/types/impact.rs`). Validation refuses a mechanism or side-effect kind in use that has neither a row nor a place in `arithmetic`. The Fluent keys the engine names are checked against `impact.ftl` (71 keys).
- **The view** (`crates/mandate-protocol/src/impact.rs`): `BillView.impact` for a draft or a bill before the body. The engine chooses the card's lines and the horizon (the last before the next election of the body that votes), rounds a range outwards to two significant figures, and says a range in words when its far end is over three times its near one. The UI formats numbers and renders keys. `bill_view` now asks for the player's post in the bill's jurisdiction (`player::posts`) for "may vote" and for the district whose support it shows; it no longer reads `player.municipality` or `player.ward`. A player with no seat there sees the jurisdiction's figure.
- **The lines on screen** (`ImpactLines` in `web/src/components/BillCard.tsx`, also under a bill in the chamber): what it does, cost, who gains and loses, when and how sure.
- **Calibration** (`mandate-cli lab impact calibrate`, `--refit` for the shipped table's own run again; `scripts/repin.sh --impact`). Deterministic: the same arguments give the same file.
- **The honesty check** (`mandate-cli audit estimate`), and a three-seed cut inside `audit gate`, reported and not counted.
- **Kept with the law.** As a bill passes its last stage, `procedures` freezes the estimate on the instance (`estimate`: yearly cost, funded shares, direct counts by ledger key, outcomes as low, middle and high at three horizons, in thousandths). `impact::ledger_key` names the ledger entry an outcome is read against. An ended law's is dropped twelve years from enactment.
- **Save format 28** (`frozen_estimates`). An older law has no frozen estimate and none is invented. The step renames a ledger's `homes_built` to `homes_started` (the v26 fixture has such entries; they read in today's words from the moment it loads) and changes nothing else. It names no version: when the two bumps ahead of this branch merge, only its key in `migrate` (27) and `SAVE_FORMAT_VERSION` need renumbering. Fixture `v27_town.mndt` (seed 2026, small, sixty idle turns, written by 1ca677c).

## Interfaces

- Content: kind `impact_table` (`ImpactTable`, `ImpactRow`, `ImpactBasis`, `ImpactOutcome`, `ImpactFit`), `ContentBundle.impact`, `IMPACT_HORIZONS`.
- Engine: `policy::impact::{inputs, project, estimate, estimate_for, book, scale, …}`, `Estimate`, `FrozenEstimate`, `PolicyInstance.estimate`; `finance::{room, first_share, capped_share, treasury_at}`.
- Protocol: `ImpactView`, `ImpactLine`, `CostView`, `GroupImpact`, `ImpactText`; `BillView.impact` (optional).
- CLI: `lab impact calibrate [--seeds --first-seed --world --levels --enact-at --towns --refit --out]`; `audit estimate [--seeds --years --scenario --only]`; `audit gate` gains `estimate_report_only`.
- Fluent: `impact-*` in `content/base/locale/en/impact.ftl`.
- Save: format 28.

## The table and the audit

The shipped table: `lab impact calibrate --seeds 8 --enact-at 1 --towns home` (seeds 101–108, six worlds, default and maximum parameters; 1,440 twins; five minutes on two threads, development build). The design's full fit (16 seeds, months 1 and 61, a town of each class) is the command's default and is hours: it belongs to the end-of-M6 pass.

`audit estimate --seeds 4` (seeds 1–4, six worlds; 24 twins a template):

| Check | Result |
|---|---|
| Direct counts against the ledger | pass, 15 of 15 (0.91–1.07; housing 1.00 at every horizon) |
| Yearly cost | pass, 15 of 15 (0.975–0.977) |
| No wrong sign | pass: no cell |
| Calibrated (70–90% of ranges hold the twin) | pass: 79.5% of 11,160 |
| No silent effect | reported: 102 of 465 cells, every template |
| Level (rent, low incomes, services) | reported: 33 cells, 13 templates |
| Size (homes, jobs, debt) | reported: 19 cells, 10 templates |
| The ledger's "added" within 0.67–1.5 of its twin | reported for 4 templates: homes 1.55 at five years (affordable housing), 1.50 (upzoning); knowledge jobs 0.63; the closure response's construction jobs 3.1 and 5.8, manufacturing 0.16 |
| Coverage of the worst cell | reported for 6 templates |
| Approval's direction | reported for 3 templates (two on one or two stated cells) |

With those rows reported, `audit estimate` exits "pass": it enforces the first four rows only. Nothing was tuned to pass a rule (AGENT_CONTEXT rule 11). The notes with figures are `REPORT_ONLY` in `crates/mandate-cli/src/audit/estimate.rs`; the design's §13 says what each means.

## Evidence

- `scripts/repin.sh` after the save-shape change: the state half of all ten spine-proof digests moved and the v18 continuation fixture was re-recorded; every event-stream digest and event count is as it was, and the shape fingerprint, the geography pin and the seed cache did not move. Its confirming tests pass.
- Native against WASM (`scripts/wasm-parity.mjs 777 60`, medium world): one state hash, with every passed law's estimate in the state.
- `mandate-save --test impact`: an older save loads with no estimate and plays on the same way twice; a law passed after loading keeps its estimate through a save; an older ledger's `homes_built` reads `homes_started` (and on the real v26 fixture, `--test arcs`); the estimate is unchanged when the seed, the streams, a held flood and the hidden facts change.
- `mandate-cli` lab test: the same calibration twice gives the same file. The shipped table refitted with `--refit` (1,440 twins again) is byte-identical, and the audit after it gives the same figures.
- A new save of the fixture's game (seed 2026, small, sixty turns) is 908 KB against 835 KB: 541 laws carry what they said.
- Web: `BillCard.test.tsx` (the lines in the real English locale, no raw key); Playwright `impact.spec.ts` (a draft's card at 390, 360 and 320 px with a wide face), desktop and phone projects.
- `scripts/check.sh affected`: fmt, clippy, content validate, seeds, generated files, web typecheck and lint pass. `cargo test` for the eight crates passes (run in two parts: the first stopped at a save test this work had broken, since fixed). `npm test`: 384 of 385 pass; `hub/layoutV2/plan.test.ts`'s stage budget read 531 ms against 500 while the Rust tests ran beside it, and passes alone (32 of 32).

## For the tuning queue

- `economy.attribution.homes_net` (0.6): twins show 1.07 of the ledger at three years and 1.5 at five.
- `economy.attribution.jobs_net` (0.8) is one number for every sector: 0.63 to 1.08 at five years for subsidised sectors, 5.8 for construction under the closure response.
- The honesty gate itself: refit at full size, then enforce `audit estimate` in `audit gate`.

## Left

- The phone bench and its budget (§2 "Cost"); the nightly workflow row.
- Knowledge levels, sources and claims (task 4); the full sheet and the map layer (5); members reading it (7); families, arcs, works and "Why?" (8); `PolicyPreviewView.impact`.
- Bands; rows of its own for `sector_capacity`; felt gain by neighbourhood.
