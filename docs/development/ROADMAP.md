# Mandate — Roadmap & Initial Backlog

**Status:** Proposed final baseline, part of the system plan awaiting owner approval. Based on DEC-062 (MVP scope) and DEC-063 (systems-first order with checkpoints).

## 1. MVP = the M6 vertical slice (DEC-062)

| Area | MVP scope |
|---|---|
| Career | Act 1 (local) complete: ward councillor → mayor, losses, a detour, Act 1 endings, plus a "next rung" teaser |
| Constitution | Federal Presidential Republic, no quirks (DEC-016). The engine reads constitution primitives (DEC-017) |
| World | One generated medium country (DEC-021). Home-town hub plus 1 visitable town. National map as a strategic view |
| Population | Full cohort model nationwide (DEC-018). ~20–30 Principal NPCs active locally |
| Economy | Layered model with ~8 sectors and municipal housing (DEC-019). National macro runs but cannot be influenced yet |
| Environment | Drifting regional state plus hazards (DEC-023) |
| Policy | ~12 local policy templates plus the municipal budget procedure (DEC-024/026) |
| Parties and elections | 2–3 parties with factions (DEC-027). Ward and mayoral plurality elections. Polls (DEC-028/029) |
| Characters and narrative | Memory, goals, favors (DEC-031/032). ~20 storyline and event templates. Director option B, Classic profile (DEC-033) |
| Media | 2–3 local outlets plus one social platform (DEC-034) |
| Player | Draft of 3 (DEC-036). Household, health, and finances (DEC-037). 1–2 detour roles (DEC-020) |
| Modes | Ironman and Story (DEC-008). Legacy epilogue for Act 1 endings |
| UI | Hub (top-down pixel art with placeholder art), desk, news, map room, phone, scenes, "Why?" (DEC-047–051) |
| Persistence | IndexedDB `SaveStore`, sync-ready metadata, export/import (DEC-044/052) |

## 2. Milestones (DEC-063)

Every milestone ends with a playable or inspectable increment, CI green, docs updated, and a demo note in `PROGRESS.md`.

| # | Milestone | Key deliverables | Exit gate |
|---|---|---|---|
| **M0** | Foundation | 8-crate workspace; core primitives; JSONC content pipeline and validator; engine skeleton (scheduler, commands, event log, state hash); save v1; protocol and TS generation; WASM worker; React PWA shell; `SaveStore`; Fluent scaffold; CI fast tier | Command round-trip through the worker; offline shell; native = WASM state hash on a stub world |
| **M1** | Deterministic world and lab | Worldgen steps 1–10 (terrain → districts, hub layout data); `mandate-cli` lab (`gen`, `inspect`, `export`, `diff`); canonical world overrides; world view model plus basic browser overview | Reproducible worlds by seed; world invariants pass; worldgen within budget |
| **M1s** | Rendering spikes (parallel; each needs owner go-ahead, DEC-050) | Hub spike; map spike | Budgets met on a real mid-range phone; renderer confirmed |
| **M2** | Society, economy, environment | Cohorts and demography; sectors, employment, wages, housing; per-tier public finance; national macro loop; environment drift and hazards; burn-in v1 (no politics yet); Monte Carlo sweep with stability bands | Stability bands pass; causal traces for shocks (factory closure chain) |
| **M3** | Local government and policy | Constitution primitives and the Federal Presidential archetype; municipal bodies, offices, procedures; policy templates, lifecycle, mechanisms, implementation gap; budget procedure; anticipated and experienced opinion; player model (slots, Mandate, resources, Authority); first panel UI (desk, budget, "Why?") | **Owner checkpoint:** propose, negotiate, and enact policy in a generated town; consequences are visible and explained |
| **M4** | Elections and political career | Parties and factions; election pipeline with plurality tabulator; polling; campaign actions; NPC AI (symmetric) and player-proxy; pressure tracks; local career ladder; burn-in with politics; bot players; drama floors | **Owner checkpoint:** play a full local election; no dominant bot strategy; drama floors pass |
| **M5** | Characters, places, and narrative | **Places as the interface (DEC-079):** player location, place-kind registry, located actions, auto-routing, the pocket, information depth by place, interim schematic town screen; **hub rendering prototype** on a phone. **Characters:** Principal/Named/Ambient tiers; relationships, favors, memory; goals and storylines. **Narrative:** director (B) with budgets and fairness; media layers and scandals; dialogue; scene-template framework; **dilemma events**. **Town character** (concern registry, DEC-078; defining industry, live crisis, power broker, named rival, local priorities); **situational policies**; DEC-075 stage 3 (communities) | Events grounded in state; pacing within budgets; storylines span chapters; actions are located and nothing critical is only at a place; **variety check: the best bot strategy differs across worlds** |
| **M6** | **Vertical slice** | Walkable pixel-art hub replaces the M5 schematic (placeholder art); representative-day UX; draft of 3 with **varied opening situations** (challenger or incumbent, safe or marginal, inherited scandal, town in crisis, hostile council; DEC-077); household, health, finances; detours (1–2); time skips with directives and interrupts; Act 1 endings and Legacy; Ironman/Story; accessibility layer; tutorial and onboarding (first chapter as a guided chapter); settings; PWA polish | **Game-quality gate: owner plus 3–5 testers**; brief §29.3 questions; M7 starts only on approval (REQ-004) |
| M7 | Act 2 (regional) | Regional institutions, governor path, travel between hubs, LOD tier promotion and demotion at scale, homecoming; **registries for budget categories, revenue and tax types, hazards, and commodities** (DEC-078, before national play) | Gate review |
| M8 | Acts 3–4 (national, presidency) | National legislature, cabinet and appointments, macro levers, presidency, Mandate Test, full endings | A complete career is playable |
| M9 | Variety and polish | Archetypes 2–4 plus quirks (curveball pool), storyteller profiles, era volatility UI, content expansion, art pass, performance, save migration hardening, mod-pack loading, audio | Release candidate |

## 3. Initial backlog — M0 (agent-sized tasks)

Each task is delegated using the brief template in `docs/development/WORKFLOW.md`. Dependencies are listed in the table. Library and version choices marked "select" must be verified (maintenance, license, WASM compatibility) and recorded as DEC entries.

| ID | Task | Depends on | Acceptance criteria |
|---|---|---|---|
| M0-01 | **Workspace scaffold:** `rust-toolchain.toml` (current stable, pinned); workspace with 8 empty crates and the dependency graph from OVERVIEW §2; `rustfmt`/`clippy` config, including determinism lints (disallow `HashMap`/`HashSet`, `std` float transcendentals, `Instant`/`SystemTime`, `thread_rng`) in engine crates; `cargo-deny` (license allowlist) | — | `cargo build`, `clippy -D warnings`, and `cargo deny check` pass; a lint test proves a forbidden call fails |
| M0-02 | **`mandate-core`:** typed `u32` ID macro plus monotonic allocators; `SimDate`/calendar (day unit, cadence-boundary iteration); `Money` (i64 minor units, rounding rules); deterministic math wrapper over `libm`; RNG: named ChaCha streams plus keyed counter access | M0-01 | Unit and property tests (calendar monotonic, money arithmetic, RNG reproducibility and order independence); WASM build |
| M0-03 | **`mandate-content`:** JSONC loader (select parser); content type registry; JSON Schema generation (select, e.g. `schemars`); validator framework (duplicate IDs, references, ranges, pluggable rules); bundle compile plus content hash; sample pack | M0-02 | `mandate-cli content validate` passes on the sample and reports seeded errors with file and line; the hash is stable across runs and platforms |
| M0-04 | **`mandate-sim` skeleton:** `State` container; `Command`/`Query` enums; scheduler with the 13 phases (OVERVIEW §4.2) and cadence registration; effect-request queue; event-log skeleton with cause references; canonical state hash; architecture test enforcing module boundaries | M0-02, M0-03 | A stub system runs at the declared cadence; same commands give the same hash; the boundary test fails on a forbidden import |
| M0-05 | **`mandate-save`:** save DTO v1 including TECH-009 sync metadata; compression (select); integrity hash; migration-chain scaffold; fixture corpus folder | M0-04 | Round-trip test; save → load → continue equals uninterrupted play (stub); a corrupted file is rejected with a typed error |
| M0-06 | **`mandate-protocol`:** envelope (`protocol_version`, `request_id`, kind, payload); error model; progress events; TS type generation (select `ts-rs` or `tsify`) | M0-04 | Generated TS compiles; Rust ⇄ JSON ⇄ TS round-trip tests; a version mismatch is rejected |
| M0-07 | **`mandate-wasm`:** wasm-bindgen exports; worker message pump (serial commands, queries between them, cancellation at safe points); panic hook in dev | M0-05, M0-06 | Node-based test drives `NewGame(stub)` → `EndTurn` → `Save` through the adapter |
| M0-08 | **`mandate-cli`:** clap skeleton with `content validate`, `gen --stub`, `run --turns`, `hash`, `save`/`load` | M0-05, M0-06 | Commands work on the stub; output is JSON |
| M0-09 | **`web/` shell:** Vite + React + TS strict; ESLint (hooks rules); Vitest; Playwright; typed worker client and query-hook layer; Zustand UI store; routes (title, runs, settings, stub game screen); PWA manifest and service worker; Dexie `SaveStore` interface plus `IndexedDbStore` | M0-06 | `tsc`, lint, and unit tests pass; E2E: start a stub game, end a turn, save, reload the page offline, resume |
| M0-10 | **Fluent scaffold:** message loading in web; key-existence check added to the validator | M0-03, M0-09 | A missing key fails validation; a rendered string test passes |
| M0-11 | **Determinism CI:** run the stub world twice natively; native versus WASM (Node) hash parity; save/load/continue parity | M0-07, M0-08 | CI job fails on an injected nondeterminism (test-only feature flag) |
| M0-12 | **CI fast tier** (GitHub Actions): Rust fmt, clippy, test, WASM build, deny and audit, content validate, TS typecheck, lint, test, build, smoke E2E (`--trace off`) | M0-01…M0-11 | Green on main; under ~15 min |
| M0-13 | **Repo docs:** finalize `AGENTS.md`; `docs/development/WORKFLOW.md`; README dev quick-start | M0-12 | A new agent can run all checks from `AGENTS.md` alone |

**Parallelizable after M0-02:** M0-03 ∥ M0-04 (an interface stub first), and M0-06 ∥ M0-05. M0-09 can start against the M0-06 types using a mock worker.

## 4. Backlog outline — M1 (to be refined into tasks at the end of M0)

- M1-01 Terrain: elevation noise, ridge shaping, sea level, coastline.
- M1-02 Hydrology: flow direction and accumulation, rivers, lakes, watersheds.
- M1-03 Climate: temperature by latitude and elevation, precipitation with prevailing wind and rain shadow.
- M1-04 Biomes (Whittaker-style); resources; hazard maps (flood, drought, wildfire, seismic).
- M1-05 Settlement-site scoring; settlement hierarchy (rank-size); municipality footprints.
- M1-06 Transport graph (least-cost paths, ports, rail).
- M1-07 Regions (grown from capitals with terrain-cost borders).
- M1-08 Neighborhoods; constituencies (contiguous, near-equal population, per constitution rules).
- M1-09 Hub layout data generator: district tile layout, POIs mapped to entity IDs (render-agnostic).
- M1-10 Founding eras (coarse history layers) as input to the M2 burn-in.
- M1-11 World view models plus a basic browser world overview (temporary SVG view until the map spike decides).
- M1-12 Lab: `gen`, `inspect`, `export`, `diff`; canonical world overrides (5 worlds).
- M1-13 World invariants (property tests); worldgen benchmarks.
- M1s-01 / M1s-02 Hub and map rendering spikes (owner go-ahead required).

## 5. Open items carried into implementation

| Item | Owner | Needed by |
|---|---|---|
| Source of final art (DEC-049) | Owner | Before M6 art integration (placeholders until then) |
| Library selections marked "select" in M0 | Agents, recorded as DECs | During M0 |
| Performance budget re-baseline (DEC-046) | Agents | End of M1s / M2 |
| Era volatility and drama floor tuning (DEC-059) | Agents plus owner | M2–M4 |
| Hosting target (static host choice, domain) | Owner | Before the first public deploy (not blocking M0–M6) |
| License and monetization (brief §19) | Owner | Deferred; permissive dependencies keep it open |
