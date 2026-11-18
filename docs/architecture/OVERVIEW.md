# Mandate — Technical Architecture Baseline (D11)

**Status:** Accepted by the owner (2026-11-18). A final architecture review takes place after D15 (brief §26.1).
**Scope:** Validates brief §13–14 and §20–22 against the accepted design decisions (DEC-001 to DEC-039). Decision IDs for this document are DEC-040 to DEC-046 in `docs/design/DECISIONS.md`.
**Note:** Crate and library versions are verified at Milestone 0 (brief §20.1), not here.

---

## 1. Runtime layers

```text
┌──────────────────────────── Main thread ────────────────────────────┐
│ React + TypeScript UI (panels, menus, reports, dialogue)            │
│ PixiJS scene layer (walkable hubs, national/regional maps)          │
│ UI state (Zustand): selection, camera, open panels, player position │
└───────────────▲──────────────────────────────────┬──────────────────┘
                │ view models / results            │ commands / queries
                │ (versioned protocol)             ▼
┌──────────────────────────── Web Worker ─────────────────────────────┐
│ mandate-wasm adapter: decode → validate → engine → encode           │
│ Owns the single authoritative simulation instance                   │
└───────────────▲──────────────────────────────────┬──────────────────┘
                │                                  │ save DTOs
┌───────────────┴────────── Rust engine ──────────────────────────────┐
│ mandate-sim (state + systems + scheduler), mandate-worldgen,        │
│ mandate-content, mandate-core                                       │
└─────────────────────────────────────────────────────────────────────┘
                │
        IndexedDB (Dexie): run saves, meta save, settings; file export/import
```

Rules (brief §13.3; REQ SIM-004, TECH-003):
- The engine crates have no browser dependencies.
- Only `mandate-wasm` uses `wasm-bindgen`.
- The UI never computes authoritative outcomes.
- The hub player's position and walking are **UI state** (DEC-009). Arriving at a point of interest and choosing an action sends a command.

## 2. Crate structure (DEC-040)

Eight crates. Each one is justified by a build target, a trust boundary, or an independence boundary.

| Crate | Responsibility | Depends on |
|---|---|---|
| `mandate-core` | Typed IDs, `SimDate`/calendar, `Money` (fixed-point), numeric helpers (deterministic math via `libm`), RNG streams, shared error types | — |
| `mandate-content` | Content schemas (constitutions, policies, roles, events, storylines, dialogue, scenes, cohorts, sectors), loading, **validation** (content CI) | core |
| `mandate-sim` | World state and all domain systems as modules: `constitution`, `population`, `economy`, `environment`, `policy`, `politics` (parties, factions, elections, polling), `characters`, `organizations`, `information` (facts, knowledge, media), `narrative` (director, storylines), `player`, plus `engine` (commands, scheduler, event log, view models) | core, content |
| `mandate-worldgen` | Terrain → hydrology → climate → biomes → resources and hazards → settlements → transport → regions and districts → hub layouts → population and economy seeding → founding eras → **burn-in** (drives `mandate-sim`) → draft candidates | core, content, sim |
| `mandate-save` | Versioned save DTOs that are **decoupled from internal structs**, migrations, (de)compression, integrity hash | core, sim |
| `mandate-protocol` | Command, query, and view-model DTOs; protocol version; **generates TypeScript types** | core, sim |
| `mandate-wasm` | Browser adapter: wasm-bindgen exports, worker message pump, panic hook | protocol, save, worldgen |
| `mandate-cli` | Simulation lab: generate, burn-in, advance, inspect, apply policy, Monte Carlo sweeps, determinism checks, export | protocol, save, worldgen |

- **Why `mandate-sim` is one crate for now:** the domain systems share one state graph and a single scheduler. Splitting them into separate crates up front would add interface overhead without adding real independence.
- **How boundaries are enforced inside it:**
  - module visibility (`pub(crate)` state, a narrow public API per module);
  - an architecture test that checks for forbidden cross-module imports;
  - a rule that each system writes only its own domain's state (§4).
- **When to split:** measured compile-time pain, or a module gaining a second consumer.

## 3. Determinism rules (DEC-041)

These rules are enforced by lints (`clippy::disallowed_methods` and `disallowed_types`) and by CI tests.

1. **Single-threaded simulation.** No Rayon and no threads in `mandate-sim`. Parallelism is allowed only *between* independent runs, in the CLI Monte Carlo.
2. **No wall clock and no OS entropy** in engine crates.
3. **Ordered iteration only.** Use `Vec` with typed indices, or `BTreeMap`/`BTreeSet`. `HashMap`/`HashSet` are disallowed in simulation state.
4. **Numerics:**
   - `Money` is `i64` minor units, with explicit rounding rules.
   - Population counts are integers.
   - Continuous quantities (attitudes, probabilities, rates) are `f64`, with documented clamping.
   - Transcendental functions (exp, ln, pow, sin, and so on) go through `libm`, never `std`, so native and WASM produce identical bits. There must be no reliance on fused multiply-add.
   - NaN is guarded at system boundaries (debug assertions plus property tests).
   - Serialized state uses explicit `u32`/`u64` widths, never `usize`.
5. **RNG:** ChaCha (via `rand_chacha`), keyed by the world seed.
   - **Sequential streams** per subsystem: worldgen, population, economy, environment, NPC, elections, director, dialogue.
   - **Counter-based access for lazily evaluated entities** (DEC-018): `stream id = hash(subsystem, entity id, period)`. The ChaCha stream selection plus word position gives order-independent draws.
   - Every RNG state is persisted in saves.
   - The hash used is a fixed, portable function. Never use `std::hash` with random keys.
6. **IDs:** typed `u32` newtypes (`NpcId`, `PlaceId`, `PolicyInstanceId`, and so on). Each type has a monotonically increasing allocator, and IDs are never reused within a world. Content IDs are stable string keys (e.g. `policy.transit_expansion`), interned at load. UUID and ULID are not needed: there is a single world and no merging.
7. **CI determinism checks:**
   - the same seed and command log give the same state hash, run twice;
   - native and WASM give the same state hash (WASM run under Node or wasmtime in CI);
   - save, load, then continue gives the same result as an uninterrupted run.

## 4. Simulation engine (DEC-042)

### 4.1 Time
- The base calendar unit is the **day** (`SimDate`).
- A turn spans N days, set by the current phase (DEC-012).
- Systems declare a cadence in calendar terms (daily, weekly, monthly, quarterly, yearly, on event). When a turn advances, each system runs once for every cadence boundary crossed.

### 4.2 Turn resolution order (WEGO, DEC-012)

```text
 1. Player commands (validated against Authority, slots, preconditions); immediate effects
 2. NPC and organization actions (DEC-030 AI, same rules as the player)
 3. Institutions: procedures advance (bills, budgets, appointments, courts)
 4. Policy implementation (progress, capacity, funding, legal challenges)
 5. Environment (hazards, resource stocks, drift)
 6. Economy (sectors, employment, wages, housing, prices, public finance)
 7. Population (demography, migration, cohort transitions)
 8. Information (fact discovery, media coverage, perception)
 9. Opinion (anticipated and experienced channels, DEC-025)
10. Elections, if scheduled (DEC-028), then polling (DEC-029)
11. Characters (goals, memories decay, schedules, promotion and demotion between level-of-detail tiers)
12. Narrative director (situation detection, scoring, budgets, exogenous-chance shaping, DEC-033)
13. Event log commit, reports, view-model refresh, autosave checkpoint if at a chapter boundary
```

This order is a baseline. Changes must be recorded in this document, with a determinism test proving the change is intentional.

### 4.3 Cross-system effects
- During its phase, each system **reads** any state but **writes only its own domain**.
- A cross-domain change is emitted as a typed **effect request** (e.g. `FundingChange`, `EmploymentShock`, `MemoryRecord`). The owning system applies it during its own phase.
- This makes dependencies explicit and testable (brief §22.3).

### 4.4 Event log and causality
- **Domain events** are meaningful occurrences: a policy enacted, a factory closed, a scandal exposed, an election result. Each carries `cause` references to earlier events or commands, which forms a causal graph.
- Retention:
  - recent detail is kept;
  - older events are summarized;
  - events referenced by memories, storylines, or Legacy are pinned.
- The log feeds news, NPC memories, policy reports, "why" screens, Legacy, and debugging.
- Transient intermediate calculations are **not** logged. In debug builds they are available as lab diagnostics.

### 4.5 Commands and queries
- **Commands** mutate state. They are logged and replayable. Examples: `NewGame`, `ChooseCandidate`, `PerformAction`, `EndTurn`, `SetDirectives`, `SceneChoice`, `Save`.
- **Queries** are read-only and return **view models** shaped for each screen. Examples: hub scene, office desk, policy detail, election breakdown, character, map overlay.
- The protocol never ships the whole simulation state to the UI.

## 5. Worker protocol (DEC-043)

- **Envelope:** `{ protocol_version, request_id, kind, payload }`.
- **Response:** `{ request_id, ok | error { code, message, details } }`.
- **Types:** Rust protocol DTOs are the single source of truth. TypeScript types are generated from them (candidate tool: `ts-rs` or `tsify`; chosen at Milestone 0). Runtime validation runs in dev builds and at persistence boundaries.
- **Encoding:** JSON initially. Switch to a binary encoding (e.g. `postcard` in a transferable `ArrayBuffer`) only if profiling shows a need.
- **Long operations** report progress: world generation, burn-in, time skips. Progress events are streamed, and the operation can be cancelled at safe points.
- **Commands are processed serially.** Queries are processed between commands.

## 6. Persistence (DEC-044)

- **Run save:** a full snapshot as a versioned save DTO (not internal structs; brief §16.2), compressed, plus the **command log since the snapshot** for crash recovery and replay debugging.
  - The save contains: seed, engine version, content version, protocol version, RNG states, simulation date, run mode (Ironman or Story, DEC-008), and an integrity hash.
- **Ironman:** saves at the end of every turn, and on save-and-quit.
- **Story mode:** additionally keeps chapter-boundary checkpoints.
- **Meta save** (DEC-039) is separate from run saves.
- **Settings** are stored in IndexedDB.
- **Export and import** use files. Exported runs carry their mode flag.
- **Migrations:** an explicit chain `vN → vN+1`. A save fixture corpus in CI must load and continue deterministically.

## 7. Frontend (DEC-045)

| Concern | Choice | Note |
|---|---|---|
| Language and build | TypeScript (strict), Vite | |
| UI | React | Chosen for AI-agent maintainability (DEC-045) |
| UI state | Zustand (UI-only state) | The simulation is the source of truth. No mirrored simulation state |
| Scenes and maps | **PixiJS** (provisional) | Pending the hub and map prototype benchmark on mid-range mobile (D12) |
| Persistence | Dexie (IndexedDB) | |
| PWA | vite-plugin-pwa / Workbox | Offline shell plus cached content |
| Tests | Vitest, Testing Library, Playwright (including mobile emulation and offline) | `--trace off` by default |
| Localization | String keys everywhere from day 1; library chosen later | |

## 8. Initial performance budgets (DEC-046, assumptions to validate)

| Metric | Target |
|---|---|
| WASM download | < 3 MB gzipped |
| New game (worldgen + 15–25-year burn-in + draft) | < 10 s on a mid-range phone, with a progress display |
| End turn (typical) | < 300 ms |
| One-year time skip | < 3 s |
| Run save size | < 5 MB compressed |
| Hub scene | 60 fps desktop; ≥ 30 fps mid-range phone |

These are re-baselined after the first prototype (brief §29.2). Benchmarks run natively with Criterion, and in the browser through Playwright timing.

## 9. Dependency policy

- Use permissive licenses only (MIT, Apache-2.0, BSD, Zlib), checked with `cargo-deny` and an npm license check. This keeps every option open for brief §19.
- Run `cargo-audit` and `cargo-deny` in CI.
- Every new dependency needs a justification in the PR (brief §25.4).
- The engine crates must build for `wasm32-unknown-unknown`.
- The minimum supported Rust version (MSRV) is the pinned stable toolchain.

## 10. Brief proposals revised by this baseline

| Brief proposal | Revision | Reason |
|---|---|---|
| ~15 domain crates (§21) | 8 crates, with domains as modules inside `mandate-sim` | Avoids trivial crates; boundaries are enforced in-crate |
| `uuid` or `ulid` for IDs (§20.1) | Typed `u32` monotonic IDs | Deterministic, compact, single world |
| Independent RNG streams only (§14.2) | Streams plus counter-based keyed access | Order-independent lazy evaluation (DEC-018) |
| Manual saves and multiple slots (§16.1) | Ironman autosave; slots are concurrent runs; Story-mode checkpoints | DEC-008 |
| PixiJS "if justified" (§15.3) | PixiJS is the provisional default | Walkable hubs (DEC-009) need a scene renderer; still benchmark-gated |
