# Mandate — Test & Validation Strategy (D14)

**Status:** Accepted (owner, 2026-11-18). Decisions DEC-056 to DEC-060. Tables mark what is not built yet.
**Principle (brief §18.1):** engineering validation checks *the software is correct*. Game validation checks *the game is good*. Both are mandatory and are tracked separately.

## 1. Engineering validation (DEC-056)

| Layer | Tooling | What it covers |
|---|---|---|
| Unit | Rust test harness; Vitest | Formulas and rules: tax, budgets, voting tabulators, opinion updates, relationship and memory changes, mechanism effects |
| Property | `proptest` | Invariants: no negative populations; budgets reconcile; IDs unique and never reused; probabilities in [0,1]; no NaN in state; calendar monotonic; vote shares sum to 1; seats allocated equal seats available; every constitution procedure terminates |
| Integration / scenario | Rust scenario tests over canonical worlds (§3) | Cross-system chains: policy → economy → cohorts → opinion → election; factory closure chain (brief §7.2); authority gating; storyline grounded in state; hub "homecoming" consistency after a long absence |
| Narrative (DEC-033, DEC-077 §4, DEC-032) | `worldgen/tests/narrative.rs`, `content/tests/narrative.rs` | Grounding audit (`narrative::audit`): 24 seeds × 10 years, every decision, request, storyline turn, and spotlit fact resolves to live actors and true conditions on the day it is emitted; budgets, spacing, and cooldowns per chapter; the crisis cap holds hazards (timing only: the same hazards as Unfiltered, each within the longest deferral); a warning precedes every terminal track end and deselection; spotlight on vs off gives identical endogenous state (elections, policies, every domain); answers apply real effects and memories; defaults at deadlines; storylines branch; an opposing goal turns a colleague's lean; determinism with answers through save/load; content: every option costs someone |
| Media and the legal path (DEC-034, DEC-006, DEC-030, DEC-033) | `worldgen/tests/media.rs`, `save/tests/saves.rs` | Grounding audit (`media::audit_story`): 10 careers × 4 years with shady acts and answers, every story traces to a fact (a scandal's hidden fact with its fate and its window, a case's stage, a rumour's leak, a statement, a town event) on the day it runs; perception only through coverage (a planted hidden fact leaves every cohort's view of the player identical to its twin's until the first story runs); the director shifts when, never whether (shaping on and off: equal fates and windows for the same facts, dates inside windows, some dates shifted); NPC corruption through the same exposure, scandal, case stages, sentence, and seat-filling as the player's, and NPCs tempted on their own; each legal stage and outcome (investigation, charges, trial; dropped, acquitted, fined or removed, prison); a long sentence only after the Legal Jeopardy warning and its response window, and adjourned when no warning is possible; a short sentence is a time skip with no decisions, then release and a filing; shady actions pay now and leave a hidden fact; determinism through saves; a v12 save migrates and the press picks up |
| Determinism | CI suite (DEC-041) | Same seed and command log run twice give equal state hashes; native and WASM give equal hashes; save → load → continue equals an uninterrupted run; lazy-evaluation order independence (Named characters caught up in either order, or in steps, give the same state: `worldgen/tests/characters.rs`) |
| Replay regression | Recorded `(seed, content hash, command log)` fixtures | Every significant bug gets a replay fixture. Re-recording a fixture requires a stated reason in the PR |
| Save compatibility | `crates/mandate-save/tests/saves.rs` | Built: each migration step (v9 onwards) has a test with a hand-made older payload, and a shape fingerprint of the state fails the build when the state changes without a format bump. **Not built:** a corpus of real saves; it starts with the M6 testers' saves, after which every released version must load, migrate, and continue deterministically |
| Content | `mandate-cli content validate` | Schemas, references, duplicate IDs, ranges, constitution playability, REQ-006 fields, localization keys |
| Protocol contract | Generated TS types plus round-trip tests | Rust ⇄ TS DTO compatibility; protocol version gates |
| Fuzzing (not built yet) | `cargo-fuzz` (nightly) | JSONC content loader, save decoder, protocol decoder |
| Performance (not built yet; measured by hand so far) | Criterion (native); Playwright timing (browser) | Budgets in architecture OVERVIEW §8 |
| Frontend | Vitest, Testing Library, Playwright (mobile emulation, offline, keyboard-only, `--trace off`) | Flows, accessibility, PWA offline, worker error states. Vitest covers the hub (`web/src/hub`: camera, day cycle, pathfinding, town layout, town scene, town map scene, the pixel-art provider and renderers), the engine client, routing, Fluent, persistence, run files, and sheet gestures. `tools/pixelkit` has its own tests (core, pack, boundary, and a golden file) run by `npm run check`. Playwright specs in `web/e2e`: `smoke`, `decisions`, `scandal`, `sheets`, `town-map`, `hub-town`, `hub-pixelkit`, `run-transfer`, `landing`, `sw-migration` |

**Exact-output "golden" tests are limited to determinism.** Balance-sensitive outputs are asserted with **metric bands**, not exact values, so that tuning does not break hundreds of tests.

## 2. CI tiers (DEC-057)

| Tier | When | Contents |
|---|---|---|
| Fast (blocking) | Every PR and push to `main` or `claude/**` (`.github/workflows/ci.yml`) | Rust job: fmt, clippy (`-D warnings`), `cargo test --workspace` (unit and property tests at the default case count), content validation, generated schemas and TS protocol types up to date, determinism (twice plus save/load/continue), a quick Monte Carlo sweep (8 seeds, 20 years), the bot gates (16 seeds), native = WASM hash parity. Web job: typecheck, lint, Vitest, the pixel-art tool's `npm run check`, production build, the hub spike build, a site layout and landing-page size check, and the whole Playwright suite (Chromium). A push to `main` then deploys to GitHub Pages |
| Nightly | Scheduled (`.github/workflows/nightly.yml`) | Monte Carlo sweep, 32 seeds by 30 years, across every canonical world and with no scenario; the bot tournament (32 seeds); `cargo test --workspace --release` with `PROPTEST_CASES=256`. **Not built:** fuzzing, benchmarks with trend tracking, a browser matrix, a save corpus |
| Release (not built yet) | Tag | Nightly suite plus the cross-browser matrix and real-device performance check |

## 3. Simulation lab and canonical worlds (DEC-058)

**`mandate-cli` lab commands:**
Built:
- `content validate` and `content schema`
- `gen` (seed, size, scenario; `--png` writes a map, `--json` the full world)
- `run` (seed, size, `--turns`, `--days`, `--hash-only`, `--save`)
- `determinism` (twice, and through save/load/continue)
- `inspect` (a municipality or region)
- `save-info` (a save file's metadata)
- `sweep` (seeds × years, parallel across runs, optional `--scenario`; bands and drama floors)
- `bots` (strategy tournament and gates, per-seed results, `--careers` outcome distributions)
- `story` (a bot career's chapters, decisions, storylines, and facts log)
- `trace-econ` (yearly economic diagnostics)
- `diff` (two runs' health metrics side by side)
- `sweep` and `bots` take `--set <path>=<value>` and `--grid <path>=<v1>,<v2>,…` over one or two content parameters (DEC-078 decision 2; syntax in `EXTENDING.md`)

Not built yet: `burnin`, `apply-policy`, `trigger`, `trace` (a causal chain from the event log), `report` (CSV and HTML), and `inspect` for entities, places, and cohorts.

**Canonical test worlds.** These are the base pack's scenarios (`content/base/scenarios.jsonc`, DEC-078 decision 4): generator biases, so a new canonical world is content (`sweep --scenario <id>`):
1. Declining industrial region
2. Wealthy coastal region
3. Fast-growing tech region
4. Drought-prone agricultural region
5. Resource-rich, sparsely populated region

In addition, each constitutional archetype is a fixture configuration.

## 4. Monte Carlo health metrics (DEC-059, revision 2)

The owner asked whether real-world-aligned ranges would make the game boring. Revision 2 clarifies three things:

1. **Bands are guardrails against broken simulation, not targets for drama.** They apply to **baseline runs**: burn-in plus 30 years of NPC-only play, with no player. The player is the agent of drama. Player choices **may** push outcomes far outside the bands (reckless policy can produce hyperinflation); that is a consequence, not a bug.
2. **Bands constrain distributions, not individual runs.** Assertions are on medians and percentiles. Tails are required to exist.
3. **Drama floors.** Sweeps fail if worlds are *too calm*, just as they fail if worlds are broken. Bands use "heightened realism": wider than real-world norms.

**Stability bands** (detect broken simulation; initial assumptions):

| Metric | Band on median across seeds | Detects |
|---|---|---|
| Population change per decade | −10% to +20% | Demographic runaway |
| Unemployment | 2–20% | Collapse or locked equilibrium |
| Inflation (annual) | −2% to +12% | Macro instability |
| Debt-to-output | No unbounded growth without cause | Fiscal runaway |
| Competitive districts (margin < 10 pts) | 15–50% | Dead or chaotic elections |
| Incumbent re-election rate | 55–85% | Over- or under-rewarding office |
| Terminal-ending mix | No single cause > 50% | Degenerate failure modes |
| Determinism and invariant violations | 0 | Correctness |
| Towns with a defining industry (DEC-077 §1) | 100% (every seed) | Town character broken; broker and rival shares and crises begun/ended are reported alongside (`town_character`) |

**Drama floors** (detect boring worlds; minimum frequency per 30-year baseline run, as the median across seeds):

| Event class | Floor (assumption) |
|---|---|
| Recessions or regional economic shocks | ≥ 2 |
| Major environmental disasters | ≥ 1 |
| Electoral upsets (the poll underdog wins) | 10–25% of contested races |
| Major scandals exposed (misconduct or crime). Measured in play by `bots --careers` (NPC scandals per career; a sweep advances days without NPC turns, so it reports 0 and does not gate) | ≥ 3 |
| Party realignment events (splits, insurgents, leadership coups) | ≥ 1 in at least 40% of seeds |
| Storyline arcs reaching a climax | Within director budgets (DEC-033) |
| Story density (DEC-033), reported per scenario | Decisions (climax + significant) and minor developments per chapter, the share of chapters with a climax, storylines; no chapter over budget (gate) |
| Turbulent worlds (top-decile volatility) | Present: some seeds must be wild |

**Volatility as a world parameter.** Each world draws an "era" volatility (calm, unsettled, turbulent) that scales exogenous shocks, polarization drift, and hazard trajectories. It is exposed in world setup and difficulty (DEC-038), and it combines with storyteller profiles (DEC-033).

## 5. Game-quality validation (DEC-060)

**Bot players.** Scripted proxy strategies play the first local election across seeds (`bots`) and, with `--careers`, longer careers. The strategies built (`crates/mandate-cli/src/bots.rs`):
- `idle`: never acts or answers; the baseline
- `canvass`: campaigns on the ground
- `media`: campaigns through the press
- `legislator`: works through bills and policy
- `mixed`: a blend of the above
- `proxy`: the NPC AI drives the player (the time-skip proxy)
- `corrupt`: mixed play plus every shady option and action (M5d); `bots --careers 8` reports its career distribution: elections, scandals, investigations, convictions, prison, careers ended
- `explorer` (M6, DEC-091): `mixed`, but its first ten turns go on learning the town: it walks every place (first-visit introductions), says hello to the passers-by on its ward's streets, reads each ward neighbourhood's worries on the doorsteps, and talks to the people it has only heard of (asking who really runs things while that is a rumour); then it plays the mix
- `explore_only` (M6, a probe like `idle`): only finds out (walks, meets, asks); it never campaigns, never answers a decision, never rests

The earlier plan (passive, populist, policy wonk, coalition-builder, random) was replaced by these.

**Bots know only what the player knows** (DEC-091; M6_ENGINE_CONTRACTS §1.10). Strategy code reads gated facts (worries, policy support, people, situational policies) only through `mandate_sim::player::knowledge::Knowing`, and lobbies by what the player can see (shown odds, parties, colleagues' memories of the player), never the whip's exact leans (they carry hidden goals). The shared mix canvasses on the worry the player knows a neighbourhood feels most and, with a campaign turn's odd slot, asks ward residents it has met for their support (shown odds at least 0.3, each once). `bots::tests::bots_are_blind_to_what_they_have_not_learned` perturbs truths a bot has not learned (the goals of people it does not know in detail; two unheard neighbourhoods' views, so their worries) and checks that six strategies send identical commands, turn by turn, from the start into the campaign. `--omniscient` (the lab switch) gives every bot's player every subject in detail, for baselines.

**Gates** (`bots` exits non-zero if any fails; `idle` and `explore_only` are probes, outside the dominance and variety checks):
- no dominant strategy: the best win rate is within 0.3 of the median;
- effort matters: the best strategy beats `idle`;
- `idle` rarely wins (at most 15%);
- corruption does not dominate: `corrupt` wins at most 10 points more often than the best honest strategy;
- variety (DEC-077 §5): at least two strategies are the sole best somewhere, none among the best in more than 60% of worlds;
- exploring early pays (DEC-091): `explorer`'s mean first-election vote share is at least 1 point above `mixed`'s, and it wins at least as often (reported, not enforced, under `--omniscient`);
- exploring alone never wins: `explore_only` wins at most 5 points more often than `idle`, and less often than the median strategy.

CI runs 16 seeds, the nightly 32 (and a 32-seed `--omniscient` baseline, uploaded beside it).

Measured:
- win rate and ending distribution per strategy; any strategy that dominates is flagged;
- estimated run length in turns and chapters, against DEC-005 targets;
- decision density: meaningful choices per chapter;
- the rate at which "Why?" explanations can be generated for outcomes.

**Human playtesting:**
- At every milestone gate, the owner reviews the game against the questions in brief §29.3.
- At the M6 vertical-slice gate, a small group (3–5 people) also plays.
- Playtesters can make an opt-in **local run-report export**: a file with the run summary, key decisions, endings, and timings, containing no personal data. They share it manually. There is no telemetry backend.

**Balance log.** `docs/development/BALANCE_LOG.md` records each tuning change: the parameter, old and new values, the evidence (sweep or bot result), and the observed effect.

## Difficulty and story targets (DEC-076)
Once careers span several acts, the lab adds three things:
- full-career bot players;
- a story-beat counter (elections won and lost, laws with visible effects, crises and scandals, rivals and allies, comebacks);
- the distribution of endings.

Nightly runs report the win rate (target 5–10% for a competent bot at default difficulty), the share of brutal early ends (under 15%), the share of story-rich runs (at least 80%), and the spread of endings (no single ending above 50%). They become gates from M6.
