# Requirements Register

Prefixes: `REQ` gameplay · `SIM` simulation · `TECH` technical · `UI` interface · `TEST` verification · `PROC` process.
Priority: P0 (must, MVP) · P1 (must, post-MVP) · P2 (should) · P3 (could).
Seeded from the brief's non-negotiable constraints (§31, DEC-002). Discovery will add more.

| ID | Description | Rationale | Priority | Acceptance criteria | Related systems | Verification | Status |
|---|---|---|---|---|---|---|---|
| REQ-001 | The world is procedurally generated and fictional: no real countries, politicians, or parties | §31.7; avoids real-world political claims | P0 | Generated names and entities are drawn from content pools; no real-world political entities appear in content | Worldgen, content | Content validation, review | Established |
| REQ-002 | Geography and environment measurably influence settlement, economy, and political interests | §31.8 | P0 | In canonical test worlds, regional industry mix and top voter priorities differ in line with geography (e.g. coastal regions weight flooding and shipping) | Worldgen, economy, population | Scenario tests | Established |
| REQ-003 | Emergent narrative arises from persistent NPCs and world events | §31.10 | P0 | Every presented event references at least one simulation fact (entity, state change, or prior decision) | Characters, narrative | Integration tests | Established |
| REQ-004 | A playable local vertical slice exists before national-scale systems are built | §31.15 | P0 | M6 game-quality gate is passed and signed off by the owner before M7 work starts | All | Milestone review | Established |
| SIM-001 | Determinism: same seed, configuration, content version, engine version, and action sequence give identical state | §31.9, §14.1 | P0 | State hashes are equal across repeated runs, across save → load → resume, and between native and WASM builds | Engine, RNG, save | Determinism test suite in CI | Established |
| SIM-002 | All randomness comes from explicit seeded RNG streams, with no global or ambient randomness | §14.2 | P0 | No uses of `thread_rng` or OS entropy in simulation crates (lint/grep check); RNG state is persisted in saves | RNG, save | CI check, tests | Established |
| SIM-003 | The narrative layer presents simulation outcomes and never sets authoritative state | §31.11 | P0 | Narrative types have read-only access to state; player choices feed back only through engine commands | Narrative, engine | Architecture review, tests | Established |
| SIM-004 | The simulation core is independent of the frontend and browser | §31.6, §13.3 | P0 | Core crates build and test natively with no `wasm-bindgen`, `web-sys`, or `js-sys` in their dependency tree | Engine | `cargo tree` check in CI | Established |
| TECH-001 | Simulation engine written in Rust (stable, pinned toolchain) | §31.1 | P0 | `rust-toolchain.toml` pins a stable version | Engine | CI | Established |
| TECH-002 | Engine compiles to WebAssembly | §31.2 | P0 | `wasm32-unknown-unknown` build succeeds in CI | Engine, WASM adapter | CI | Established |
| TECH-003 | Browser simulation work runs in a Web Worker | §31.3 | P0 | No simulation calls on the main thread; the UI stays responsive during turn advancement | Worker, frontend | E2E test, profiling | Established |
| TECH-004 | Primary product is a static-hosted PWA | §31.4 | P0 | Build output is static files only; installable; manifest and service worker present | Frontend, deploy | Lighthouse/PWA check | Established |
| TECH-005 | Supports desktop and mobile | §31.5 | P0 | Core loop is completable at 360 px width and on desktop, by touch and by keyboard | Frontend | Playwright device tests | Established |
| TECH-006 | Offline-first: playable with no network after first load | §1.3 | P0 | E2E: start, play, and save a game with the network disabled after install | Frontend, PWA | Playwright offline test | Established |
| TECH-007 | No backend infrastructure for single-player | §31.17 | P0 | No server runtime in the deployment | Deploy | Review | Established |
| TECH-008 | Avoid premature complexity: no full ECS, multithreaded WASM, or plugin runtime without a documented, measured need | §31.18 | P0 | Any such adoption has an accepted DEC entry with profiling or design evidence | Architecture | Decision register review | Established |
| PROC-001 | Architecture and design decisions are recorded in DECISIONS.md, never silently improvised | §31.16 | P0 | Every architectural PR references a DEC ID | Process | Review | Established |
| PROC-002 | Agent-generated code follows documented interfaces and acceptance criteria | §31.13 | P0 | Every task brief includes the §23.2 fields; reviewer checklist is applied | Process | Review | Established |
| PROC-003 | Tests cover both unit correctness and cross-system behavior | §31.14 | P0 | Every simulation feature ships with at least one integration test | All | CI | Established |
| REQ-005 | Turn calendar spans per phase, slots per turn, Late Night availability and costs, and per-action slot costs are data-driven tuning parameters | DEC-012/013; the owner expects to rebalance slot counts | P0 | Changing these values requires no code change; the simulation lab can sweep them | Engine, content | Content-config test | Established |
