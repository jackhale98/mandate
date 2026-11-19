# Mandate — Test & Validation Strategy (D14)

**Status:** Proposed. Decisions DEC-056 to DEC-060.
**Principle (brief §18.1):** engineering validation checks *the software is correct*. Game validation checks *the game is good*. Both are mandatory and are tracked separately.

## 1. Engineering validation (DEC-056)

| Layer | Tooling | What it covers |
|---|---|---|
| Unit | Rust test harness; Vitest | Formulas and rules: tax, budgets, voting tabulators, opinion updates, relationship and memory changes, mechanism effects |
| Property | `proptest` | Invariants: no negative populations; budgets reconcile; IDs unique and never reused; probabilities in [0,1]; no NaN in state; calendar monotonic; vote shares sum to 1; seats allocated equal seats available; every constitution procedure terminates |
| Integration / scenario | Rust scenario tests over canonical worlds (§3) | Cross-system chains: policy → economy → cohorts → opinion → election; factory closure chain (brief §7.2); authority gating; storyline grounded in state; hub "homecoming" consistency after a long absence |
| Narrative (DEC-033, DEC-077 §4, DEC-032) | `worldgen/tests/narrative.rs`, `content/tests/narrative.rs` | Grounding audit (`narrative::audit`): 24 seeds × 10 years, every decision, request, storyline turn, and spotlit fact resolves to live actors and true conditions on the day it is emitted; budgets, spacing, and cooldowns per chapter; the crisis cap holds hazards (timing only: the same hazards as Unfiltered, each within the longest deferral); a warning precedes every terminal track end and deselection; spotlight on vs off gives identical endogenous state (elections, policies, every domain); answers apply real effects and memories; defaults at deadlines; storylines branch; an opposing goal turns a colleague's lean; determinism with answers through save/load; content: every option costs someone |
| Determinism | CI suite (DEC-041) | Same seed and command log run twice give equal state hashes; native and WASM give equal hashes; save → load → continue equals an uninterrupted run; lazy-evaluation order independence (Named characters caught up in either order, or in steps, give the same state: `worldgen/tests/characters.rs`) |
| Replay regression | Recorded `(seed, content hash, command log)` fixtures | Every significant bug gets a replay fixture. Re-recording a fixture requires a stated reason in the PR |
| Save compatibility | Save fixture corpus | Every released save version loads, migrates, and continues deterministically |
| Content | `mandate-cli content validate` | Schemas, references, duplicate IDs, ranges, constitution playability, REQ-006 fields, localization keys |
| Protocol contract | Generated TS types plus round-trip tests | Rust ⇄ TS DTO compatibility; protocol version gates |
| Fuzzing | `cargo-fuzz` (nightly) | JSONC content loader, save decoder, protocol decoder |
| Performance | Criterion (native); Playwright timing (browser) | Budgets in architecture OVERVIEW §8 |
| Frontend | Vitest, Testing Library, Playwright (mobile emulation, offline, keyboard-only, `--trace off`) | Flows, accessibility, PWA offline, worker error states |

**Exact-output "golden" tests are limited to determinism.** Balance-sensitive outputs are asserted with **metric bands**, not exact values, so that tuning does not break hundreds of tests.

## 2. CI tiers (DEC-057)

| Tier | When | Contents |
|---|---|---|
| Fast (blocking) | Every PR | fmt, clippy (with determinism lints), unit tests, property tests (low case count), short determinism (native and WASM), content validation, TS typecheck, lint, unit tests, production build, smoke E2E |
| Nightly | Scheduled | Property tests (high case count), Monte Carlo health sweep (§4), bot-player tournament (§5), fuzzing, benchmarks with trend tracking, full E2E matrix, save corpus |
| Release | Tag | Nightly suite plus the cross-browser matrix and real-device performance check |

## 3. Simulation lab and canonical worlds (DEC-058)

**`mandate-cli` lab commands:**
- `gen` (seed, size, constitution, overrides)
- `burnin`
- `run --turns/--years`
- `inspect` (entity, place, region, cohort)
- `apply-policy`
- `trigger`
- `trace` (causal chain from the event log)
- `diff` (two runs)
- `sweep` (seeds × parameters, parallel across runs); `sweep` and `bots` take `--set <path>=<value>` and `--grid <path>=<v1>,<v2>,…` over one or two content parameters (DEC-078 decision 2; syntax in `EXTENDING.md`)
- `report` (CSV/JSON, plus an HTML summary)

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
| Major scandals exposed | ≥ 3 |
| Party realignment events (splits, insurgents, leadership coups) | ≥ 1 in at least 40% of seeds |
| Storyline arcs reaching a climax | Within director budgets (DEC-033) |
| Story density (DEC-033), reported per scenario | Decisions (climax + significant) and minor developments per chapter, the share of chapters with a climax, storylines; no chapter over budget (gate) |
| Turbulent worlds (top-decile volatility) | Present: some seeds must be wild |

**Volatility as a world parameter.** Each world draws an "era" volatility (calm, unsettled, turbulent) that scales exogenous shocks, polarization drift, and hazard trajectories. It is exposed in world setup and difficulty (DEC-038), and it combines with storyteller profiles (DEC-033).

## 5. Game-quality validation (DEC-060)

**Bot players.** Scripted proxy strategies play full careers using the DEC-030 AI with fixed profiles:
- passive
- populist
- policy wonk
- coalition-builder
- corrupt opportunist
- random

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
