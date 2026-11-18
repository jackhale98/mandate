# Mandate — Test & Validation Strategy (D14)

**Status:** Proposed. Decisions DEC-056 to DEC-060.
**Principle (brief §18.1):** engineering validation checks *the software is correct*. Game validation checks *the game is good*. Both are mandatory and are tracked separately.

## 1. Engineering validation (DEC-056)

| Layer | Tooling | What it covers |
|---|---|---|
| Unit | Rust test harness; Vitest | Formulas and rules: tax, budgets, voting tabulators, opinion updates, relationship and memory changes, mechanism effects |
| Property | `proptest` | Invariants: no negative populations; budgets reconcile; IDs unique and never reused; probabilities in [0,1]; no NaN in state; calendar monotonic; vote shares sum to 1; seats allocated equal seats available; every constitution procedure terminates |
| Integration / scenario | Rust scenario tests over canonical worlds (§3) | Cross-system chains: policy → economy → cohorts → opinion → election; factory closure chain (brief §7.2); authority gating; storyline grounded in state; hub "homecoming" consistency after a long absence |
| Determinism | CI suite (DEC-041) | Same seed and command log run twice give equal state hashes; native and WASM give equal hashes; save → load → continue equals an uninterrupted run; lazy-evaluation order independence |
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
- `sweep` (seeds × parameters, parallel across runs)
- `report` (CSV/JSON, plus an HTML summary)

**Canonical test worlds.** These are pinned fixtures made from generation overrides, so they stay stable when the generator changes:
1. Declining industrial region
2. Wealthy coastal region
3. Fast-growing tech region
4. Drought-prone agricultural region
5. Resource-rich, sparsely populated region

In addition, each constitutional archetype is a fixture configuration.

## 4. Monte Carlo health metrics (DEC-059)

Nightly sweeps over N seeds, with burn-in plus 30 years of NPC-only play. Each metric must fall inside its band. Bands are initial assumptions, tuned during balancing.

| Metric | Initial band (assumption) | Detects |
|---|---|---|
| Population change per decade | −10% to +20% | Demographic runaway |
| Unemployment | 2–20% | Economic collapse or locked equilibrium |
| Inflation (annual) | −2% to +12% | Macro instability |
| Debt-to-output trajectory | No unbounded growth without cause | Fiscal runaway |
| Competitive districts (margin < 10 pts) | 15–50% | Dead elections or chaos |
| Incumbent re-election rate | 55–85% | Over- or under-rewarding office |
| Party-system realignments per 30 years | 0–2 typical | Party churn (DEC-027) |
| Events per chapter by category | Within director budgets (DEC-033) | Pacing failures |
| Terminal-ending mix | No single cause > 50% of endings | Degenerate failure modes |
| Determinism and invariant violations | 0 | Correctness |

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
