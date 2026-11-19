# Mandate — Technical Architecture

**Status:** Accepted by the owner (2026-11-18) as the technical baseline (DEC-040 to DEC-046). Updated 2026-11-19 to describe the code on `main`, including later accepted revisions (DEC-044 rev. 2, DEC-069, DEC-074, DEC-080, DEC-083 to DEC-085). Where this document and a DEC entry disagree, the DEC entry wins.
**Section numbers are stable:** other documents cite §2 (crates), §3 (determinism), §4 (engine), §4.3 (cross-domain effects), §8 (budgets), and §10 (revisions).
**Planned** marks anything decided but not built yet.

---

## 1. Runtime layers

```text
┌──────────────────────────── Main thread ────────────────────────────┐
│ React + TypeScript UI: screens, sheets, the pocket, place panels     │
│ Canvas 2D: the national map (web/src/map); the walkable town         │
│   (web/src/hub, M6 groundwork, not yet in the app)                   │
│ UI state (Zustand): screen, view, open sheets, player's walk         │
└───────────────▲──────────────────────────────────┬──────────────────┘
                │ view models / results            │ commands / queries
                │ (versioned JSON protocol)        ▼
┌──────────────────────────── Web Worker ─────────────────────────────┐
│ mandate-wasm: decode → engine → encode; save export and import      │
│ Owns the single authoritative simulation instance                   │
└───────────────▲──────────────────────────────────┬──────────────────┘
                │                                  │ save bytes
┌───────────────┴────────── Rust engine ──────────────────────────────┐
│ mandate-sim (state, systems, scheduler), mandate-worldgen,          │
│ mandate-content (embedded base pack), mandate-save, mandate-core    │
└─────────────────────────────────────────────────────────────────────┘
                │
        IndexedDB (Dexie): run saves, settings; `.mandate` file export/import
```

Rules (brief §13.3; SIM-004, TECH-003):
- The engine crates have no browser dependencies. Only `mandate-wasm` uses `wasm-bindgen`.
- The UI never computes authoritative outcomes. It sends commands and renders view models.
- The engine never produces prose. Views carry facts and Fluent keys (DEC-054).
- The player's **location** (which place they are at) is simulation state, because actions are located (DEC-079). The **walk** between places, and everything on the walkable town screen, is UI state (DEC-009).

## 2. Crate structure (DEC-040)

Eight crates. Each one is justified by a build target, a trust boundary, or an independence boundary.

| Crate | Responsibility | Depends on |
|---|---|---|
| `mandate-core` | Typed `u32` IDs and allocators, `SimDate` and cadences, `Money` (i64 minor units), deterministic math over `libm`, ChaCha RNG streams and keyed access, BLAKE3 stable hashing, serde helpers (`map_as_pairs`) | — |
| `mandate-content` | Content types, JSONC loading with line-accurate errors, schema generation (`schemars`), validation, the bundle and its content hash, the embedded base pack (for WASM), lab overrides (`--set`, DEC-078) | core |
| `mandate-sim` | State and all domain systems as modules (below), the scheduler, commands, the event log, NPC AI, player actions | core, content |
| `mandate-worldgen` | Terrain → climate → hydrology → biomes → resources and hazards → settlements → territories → regions → transport → neighbourhoods → districts → places; seeding of every domain; the 20-year **history burn-in** (drives `mandate-sim`); terrain rebuild from the seed on load | core, content, sim |
| `mandate-save` | The save file (header, metadata, compressed JSON snapshot, integrity hash) and the JSON migration chain | core, sim |
| `mandate-protocol` | Commands, queries, and view-model DTOs, `PROTOCOL_VERSION`; generates TypeScript types with `ts-rs` into `web/src/engine/protocol/` | core, content, sim |
| `mandate-wasm` | Browser adapter: `WasmSession` (request handling, progress callback, save export and import, state hash), panic hook | core, content, sim, worldgen, save, protocol |
| `mandate-cli` | Simulation lab: `content validate|schema`, `gen` (with `--png`), `inspect`, `run`, `determinism`, `save-info`, `sweep`, `bots`, `story`, `trace-econ`, `diff` | core, content, sim, worldgen, save, protocol |

**Domain modules in `mandate-sim`** (each owns one part of `State`, see DATA_MODEL):

| Module | Domain |
|---|---|
| `world` | The generated geography and places (written by worldgen; read-only in play) |
| `population` | Cohorts per neighbourhood (age × class × community, DEC-075), demography, migration, opinion, party attachment, issue salience; `concerns` computes each town's concern levels and salience (DEC-078) |
| `economy` | The global cycle, the national macro loop, per-town sectors (a registry with coded drivers, DEC-075), jobs and wages, housing (`economy::housing`) |
| `finance` | Local, regional, and national treasuries, taxes, grants, appropriations, debt |
| `environment` | The seeded trajectory, regional resource stocks and pollution, hazards (and hazards the director deferred) |
| `policy` | Policy instances (5-state lifecycle), staged procedures, funding, implementation, mechanisms and side effects (DEC-024, DEC-070, DEC-073) |
| `politics` | Politicians, councils, parties and factions, elections and polls (`politics::elections`, `politics::parties`), declared votes, lobbying sway; it also stores `characters` |
| `characters` | Character records for every politician (Principal) and Named people, relationships, favours, memory (DEC-031); Ambient passers-by (`characters::ambient`); NPC goals (`characters::goals`, DEC-032); town character (`characters::town`, DEC-077 §1) |
| `narrative` | The director (DEC-033 B), chapters, decisions (dilemmas and requests, DEC-077 §4), storylines, the facts log, hidden facts, fairness warnings, exogenous chance requests; `narrative::audit` checks grounding |
| `player` | The player's slots, Mandate, pressure tracks, money, filing, and location |
| `places` | Places as the interface (DEC-079): `Location`, what each place offers, routing |
| `npc`, `actions` | NPC utility AI (DEC-030, DEC-071) and player actions; both run before the calendar advances |
| `engine`, `scheduler`, `systems`, `commands`, `events` | The engine API, phases and cadences, system registration, commands, the event log |

Not built yet (**planned**): `organizations` (unions, outlets, firms as actors), an `information` domain for the media layers and scandals (DEC-034, M5d part 1), regional and national government in play (M7–M8). The constitution is content (`constitution` kind, DEC-017) read by `policy` and `politics`; it has no module of its own.

- **Why `mandate-sim` is one crate:** the domains share one state graph and one scheduler. Splitting them would add interface overhead without real independence.
- **How boundaries are enforced** (DEC-069): each system receives `&mut` access only to its own domain plus a read-only `Reads` view of the rest, so the borrow checker enforces "write only your own domain". Cross-domain changes are typed `Effect`s (§4.3).
- **When to split:** measured compile-time pain, or a module gaining a second consumer.

## 3. Determinism rules (DEC-041)

Enforced by `clippy.toml` (`disallowed-types`, `disallowed-methods`) and by CI tests.

1. **Single-threaded simulation.** No threads in engine crates. The lab runs independent seeds one after another.
2. **No wall clock and no OS entropy** in engine crates (`Instant`, `SystemTime` disallowed). Save timestamps come from the client as metadata only.
3. **Ordered iteration only.** `Vec` with typed indices, or `BTreeMap`/`BTreeSet`. `HashMap`/`HashSet` are disallowed. Maps with non-string keys serialise as lists of pairs.
4. **Numerics:**
   - `Money` is `i64` minor units, with explicit rounding.
   - Population counts are integers, and people are conserved exactly (tested).
   - Continuous quantities are `f32` (most per-cohort and per-person values, to keep saves small) or `f64`, clamped where documented.
   - Transcendentals (`exp`, `ln`, `powf`, trigonometry, `tanh`) go through `mandate_core::math` (`libm`); `mul_add` is disallowed.
   - JSON float round-trips are exact (`serde_json/float_roundtrip`), so save → load → continue matches.
5. **RNG:** ChaCha (`rand_chacha`), keyed by the world seed.
   - **Named sequential streams:** worldgen, population, economy, environment, npc, elections, polling, director, dialogue, player, misc. Their states are saved.
   - **Keyed access** (`rng::keyed(seed, stream, parts)`) for order-independent draws: lazily updated Named characters, Ambient passers-by, the director, the global cycle, internal polls, scenario offsets. A view never advances a stream.
   - Hashing is a fixed portable function (BLAKE3), never `std::hash`.
6. **IDs:** typed `u32` newtypes (`RegionId`, `MunicipalityId`, `NeighborhoodId`, `ConstituencyId`, `LinkId`, `PlaceId`, `PoliticianId`, `CharacterId`, `PartyId`, `PolicyInstanceId`, `EventId`, `DecisionId`, `StorylineRunId`) from monotonic allocators, never reused. Content IDs are stable string keys (`policy.transit_frequency`).
7. **CI checks:** the same seed and commands give the same state hash twice; native = WASM (Node) hash; save → load → continue equals an uninterrupted run (`mandate-cli determinism`, `scripts/wasm-parity.mjs`).
8. **Scope:** determinism is guaranteed within one (engine, content) version (DEC-074). A save loaded under updated rules continues under the new rules.
9. **On the client:** the town layout (DEC-085) follows the same ideas in TypeScript: keyed seeded streams (`web/src/hub/rng.ts`), no `Math.random`, no clock, no functions that differ between JavaScript engines, and a layout version.

## 4. Simulation engine (DEC-042)

### 4.1 Time
- The base unit is the **day** (`SimDate`). A turn spans a number of days set by the phase (campaign: 1 week; governing: 2 weeks; DEC-012).
- Systems declare a cadence (daily, weekly, monthly, yearly). When a turn advances, each system runs once for every boundary crossed.
- Before play, worldgen runs a 20-year NPC-only burn-in (`GamePhase::BurnIn`, DEC-022).

### 4.2 Turn resolution order (WEGO, DEC-012)

Player commands and NPC turns run before the calendar advances. Then the scheduler runs the registered systems in phase order:

| # | Phase | Systems now (`systems.rs`) |
|---|---|---|
| 1 | Commands | Player commands (validated against slots, location, and preconditions), decision answers and deadline defaults (`narrative::resolve`) |
| 2 | NpcActions | NPC turns (`npc`): the player's town every turn, other towns every fourth turn |
| 3 | Institutions | `policy.procedures` (weekly) |
| 4 | PolicyImplementation | `policy.implementation` (monthly) |
| 5 | Environment | `environment.drift`, `environment.hazards` (monthly) |
| 6 | Economy | `economy.global`, `economy.markets`, `economy.national`, `finance.accrue` (monthly); `finance.budget_year` (yearly) |
| 7 | Population | `population.migration` (monthly), `population.demography` (yearly) |
| 8 | Information | None yet (**planned**: media coverage and scandal discovery, M5d part 1) |
| 9 | Opinion | `population.concerns`, `population.opinion` (monthly) |
| 10 | Elections | `politics.elections` (daily) |
| 11 | Characters | `characters.effects` (daily); `player.monthly`, `politics.monthly`, `characters.monthly` (monthly); `politics.yearly` (yearly) |
| 12 | Director | `narrative.fairness` (daily), `narrative.director` (weekly) |
| 13 | Commit | `calendar.year` (yearly) |

Changing this order is a recorded architecture change and needs a determinism test update.

### 4.3 Cross-system effects
- During its phase, a system reads any state but writes only its own domain.
- A cross-domain change is a typed `Effect` (today: `HousingDamage`, `SectorShock`, `Displacement`, `Memory`), applied by the owning domain in its own phase.
- Policy never writes another domain: domains read the active laws' combined `PolicyMods`.
- The director's exogenous requests (`Chance`: hazard holds, a thinner hazard rate, a held recession onset) are posted in the narrative domain and applied, clamped, and logged by environment and economy (DEC-033 B).

### 4.4 Event log and causality
- `DomainEvent`s carry a kind, a date, and `causes` (earlier events), which form a causal graph.
- Retention: the 2,000 most recent events are kept; events that memories, decisions, or updates refer to are pinned (`RulesUpdated` is always pinned).
- The log feeds news lines, memories, "Why?" panels, and election-night breakdowns. The narrative domain keeps its own bounded facts log for the director (and, **planned**, the media layer).

### 4.5 Commands and queries
- **Commands** mutate state and are deterministic: `EndTurn`, `AdvanceDays`, `DraftPolicy`, `AmendPolicy`, `IntroducePolicy`, `WithdrawPolicy`, `Act` (lobby, trade a favour, town hall, press, canvass, fundraise, rest, advertise; optionally late at night), `File`, `CastVote`, `MoveTo`, `Go` (move, then act: auto-routing), `Meet`, `AnswerDecision`. `LabSectorShock` is lab-only.
- **Queries** are read-only and return per-screen view models: `Overview`, `Office`, `Campaign`, `Map`, `Municipality`, `Pocket`, `Town`, `Place`, `Character`, `Decision`, `Story`, `Locale`, `Scenarios`, `Diagnostics`. The engine decides information depth by place (DEC-079): exact figures at the right place, coarse ones in the pocket.
- The protocol never ships the whole state to the UI.

## 5. Worker protocol (DEC-043)

- **Request:** `{ protocol_version, request_id, body }`, where `body` is `new_game`, `command`, or `query`. **Response:** `{ request_id, result: ok { body } | err { code, message } }`. A version mismatch is refused.
- **Types:** the Rust DTOs in `mandate-protocol` (`views`, `office`, `campaign`, `map`, `places`, `characters`, `narrative`) are the source of truth; `cargo test -p mandate-protocol` regenerates the TypeScript types, and CI fails if they are stale.
- **Encoding:** JSON. A binary encoding is only worth it if profiling shows a need.
- **Progress:** world generation and the burn-in report `(stage key, fraction)` through `setProgress`. Cancellation at safe points is **planned**.
- **Serial:** the worker (`web/src/engine/engine.worker.ts`) handles requests strictly in arrival order. `web/src/engine/client.ts` is the typed client; save bytes travel through `exportSave`, `importSave`, and `readSaveMeta`.

## 6. Persistence (DEC-044 rev. 2, DEC-052, DEC-074, DEC-082)

- **Save file** (`mandate-save`): `MNDT` magic, format version, metadata length, metadata JSON, then the DEFLATE-compressed JSON snapshot of `State`. The metadata (run id, revision, parent revision, device id, saved-at, engine version, content hash, protocol version, run mode, seed, date, turn, payload BLAKE3 hash) is readable without inflating the payload.
- **Format version 12.** A shape-fingerprint test fails if the serialised state changes without a bump. Migrations are JSON-level steps `v9 → v10 → v11 → v12` (scenario ids, characters, the narrative domain). Older development saves are refused with a clear message. From M6 (testers), every bump ships with a migration (DEC-074).
- **When:** a full save after every player command (about 90 ms) and at every turn end. There is no command log and no replay (DEC-074).
- **Loading under updated rules** (`Engine::from_state`): if the content hash or engine version differs, `missing_references` lists rules the save needs that no longer exist (its constitution, policy templates in use, scenario, sectors, attitudes, communities, characters' traits and occupations); any missing reference refuses the save with their names. Otherwise a pinned `RulesUpdated` event records the change, and the state is brought in line with the registries:
  - sectors, attitudes, and communities are re-keyed by id (the save records `sector_keys`, `attitude_keys`, `community_keys`); new entries start without randomness (empty, or at their seed mean);
  - concerns and town characters are derived and recomputed when their registries differ;
  - characters gain records missing from older saves; pending decisions and storylines whose templates were removed lapse.
- Terrain is not saved; it is rebuilt from the seed and scenario on load (DEC-068).
- **Web storage:** `SaveStore` (`web/src/persistence/saveStore.ts`) over Dexie (`indexedDbStore.ts`): runs and settings. The store has a table for Story-mode chapter checkpoints; the app does not write them yet (**planned**). A meta save (DEC-039) is **planned**.
- **Export and import** (DEC-082): one `.mandate` text file per run (magic line, JSON header with checksum, base64 save). Import validates before storing; an older revision of an Ironman run is refused, an older Story run is added beside the current one. On the title screen's run list.

## 7. Frontend (DEC-045)

| Concern | Choice |
|---|---|
| Language and build | TypeScript 6 (strict), Vite 8 |
| UI | React 19 |
| UI state | Zustand (`web/src/state/gameStore.ts`); no mirrored simulation state |
| Map and town | Canvas 2D (map since M1; town per DEC-080). PixiJS is only a dev dependency of the renderer comparison spike |
| Persistence | Dexie (IndexedDB) |
| PWA | vite-plugin-pwa (Workbox), scoped to `play/` |
| Text | Fluent (`@fluent/bundle`), messages from the content pack's `locale/en/` |
| Tests | Vitest, Testing Library, Playwright (desktop and phone projects, `--trace off`) |

**App structure (`web/src/`):**
- `screens/`: `Title` (new game with scenario and size, saved runs, export and import), `Game` (HUD with date, slots, money, and mode; the main view; the action bar with the pocket button, current location, and End turn), `Town` (the town screen: the Town card, then each neighbourhood with its places as buttons), `PlacePanel` (what a place offers: actions, its data service, "People here", decisions waiting there).
- The main view is the town; the map (`map/WorldMap.tsx`) and the national overview open from the pocket.
- `components/`: the **pocket** (`Pocket.tsx`: coarse figures, alerts, bills, pending decisions with a badge, and the agenda, auto-routed with a short skippable walk, `Walking.tsx`); **sheets** that slide up over the town: `DecisionSheet` (trade-offs, deadline, default, and the aftermath), `CharacterSheet`, `StorySheet` (the story so far); `Desk` and `Chamber` (bills, catalogue, whip count) inside place panels; `CampaignCards` and `ElectionCharts` (campaign hub, election night); `NationalEconomy`, `MunicipalityPanel`, `StatTile`, `TradeOffs`; `Brand` (logo and loader, DEC-081); `RunTransfer` (DEC-082).
- **Town character card** (`Town.tsx`, DEC-077 §1): the defining industry, the live crisis, the power broker, the player's rival, and the town's priorities.
- `places/labels.ts`: place names and icons; `state/routing.ts`: the client side of auto-routing.
- `engine/`: worker, client, generated protocol types (`engine/protocol/`), and the WASM build (`engine/pkg/`, generated).
- `hub/`: the walkable-town groundwork for M6 (§12); not imported by the app yet.

**Planned:** the town map overview replaces the place list as the town screen's main view (DEC-086), with the list kept as the accessible text view; in M6 it zooms into the walkable town.

## 8. Performance budgets (DEC-046) and measurements

| Metric | Target | Measured (latest) |
|---|---|---|
| WASM download | < 3 MB gzipped | Not re-measured since M0 |
| New game (worldgen + burn-in) | < 10 s on a mid-range phone, with progress | Medium world with 20 years of history: about 0.83 s native, 1.2–3 s in the browser (DEC-072, BALANCE_LOG) |
| End turn (typical) | < 300 ms | 60 medium turns about 1.0 s native |
| Save | — | About 90 ms per full save (DEC-074) |
| One-year time skip | < 3 s | Not built (M6) |
| Run save size | < 5 MB compressed | — |
| Hub scene | 60 fps desktop; ≥ 30 fps mid-range phone | Canvas 2D: 40–60 fps on an emulated phone with the CPU slowed 4× (DEC-080); 58–60 fps with 30 townsfolk and the pixel-art pack (P0); worked well on the owner's phone |
| Town layout | — | 15–25 ms per town on a desktop, under 80 ms on a slowed phone (DEC-085) |

A formal re-baseline is part of the M6 performance pass.

## 9. Dependency policy

- Permissive licenses only (MIT, Apache-2.0, BSD, Zlib). `cargo-deny` and `cargo-audit` in CI are **planned** (not wired yet); new dependencies are reviewed by hand meanwhile.
- Every new dependency needs a justification in the PR (brief §25.4); significant ones get a DEC entry.
- The engine crates must build for `wasm32-unknown-unknown`. The toolchain is pinned in `rust-toolchain.toml`.
- `tools/pixelkit` has no runtime dependencies (§13).

## 10. Brief proposals revised by this baseline, and later revisions

| Brief proposal | Revision | Reason |
|---|---|---|
| ~15 domain crates (§21) | 8 crates, with domains as modules inside `mandate-sim` | Avoids trivial crates; boundaries are enforced in-crate |
| `uuid` or `ulid` for IDs (§20.1) | Typed `u32` monotonic IDs | Deterministic, compact, single world |
| Independent RNG streams only (§14.2) | Streams plus keyed access | Order-independent lazy evaluation (DEC-018) |
| Manual saves and multiple slots (§16.1) | Ironman autosave; slots are concurrent runs; Story-mode checkpoints | DEC-008 |
| Save DTOs separate from internal structs (§16.2) | A versioned JSON snapshot with JSON migrations and a shape-fingerprint test | DEC-044 rev. 2 |
| A command log beside the snapshot | A full save after every action; no replay | DEC-074 |
| A grep-based architecture test | Compile-time domain write isolation | DEC-069 |
| PixiJS "if justified" (§15.3); provisional default in this baseline | Canvas 2D for the map and the town | DEC-080 |

## 11. Site and hosting (DEC-064, DEC-083)

- GitHub Pages, deployed from `main` by the CI workflow. One build variable, `MANDATE_BASE` (default `/mandate/`), sets every path; `/` serves from a root domain.
- **Layout:** the landing page at the base (`https://jackhale98.github.io/mandate/`), the game at `<base>play/`, the town prototype unlinked at `<base>spike-hub/`.
- **Landing page** (`web/landing/`): plain HTML, CSS, and about 1 KB of TypeScript, its own Vite build into `dist/`; the app builds into `dist/play/`. Images are captured from the running game (`web/landing/scripts/`). It reads the game's IndexedDB read-only to offer "Continue your run". CI checks a 60 KB gzipped budget. A refresh with the walkable town is **planned** for M6.
- **Service workers:** the game's worker is at `play/sw.js`, scoped to `play/`. A retiring worker at the old `<base>sw.js` moves installs from before DEC-083 to the new layout; it must stay in the build while old installs may exist.
- Moving to a custom domain needs players to export their runs first (IndexedDB belongs to the origin), which DEC-082's files make possible.

## 12. Walkable town groundwork (DEC-080, DEC-085)

Built in `web/src/hub/`, tested, used by the prototype at `spike-hub/`, and not yet imported by the app:
- `townLayout.ts`: `layoutTown(input, seed, version)` lays out the player's town from the engine's `Town` view (neighbourhoods and places) as a tile map with streets, districts, a door per place, filler buildings, landmarks, and water. Pure and versioned (`LAYOUT_VERSION` 1); tests pin hashes of committed engine fixtures. **Planned:** the app records the layout version per run, and reserves plots for places added during a run (DEC-085).
- `pathfinding.ts` (8-direction A*, pedestrian costs, string-pulling, tap targets), `camera.ts` (pinch, pan, wheel, eased limits), `townScene.ts` (the player and townsfolk as UI state), `townInput.ts` (adapters from protocol views).
- `render/`: the Canvas 2D renderer (`townRenderer.ts`: chunks baked lazily into an LRU, sun-following shadows, per-frame walkers, lit windows and lamps). Art sits behind `ArtProvider` (`render/art.ts`), with two implementations: `PlaceholderArt` (drawn in code) and `pixelkitArt.ts` (the pixel-art tool's pack, `?art=pixelkit`).
- `dayCycle.ts`: a continuous clock graded through keyframes; `hubView.ts`: the DOM controller (canvas at CSS resolution, HTML labels over it, sleeps when idle).

## 13. Pixel-art tool boundary (DEC-084)

- `tools/pixelkit/` (working name; MIT OR Apache-2.0; TypeScript; no dependencies): `src/core` (palette, compositor, autotiling, hashing, lints, serialisation), `src/runtime` (what the game imports), `src/cli` and `src/node` (sheets, PNG output), and Mandate's art pack in `art/mandate/` (all rights reserved).
- **Hard boundary,** enforced by `test/boundary.test.ts`: the core and runtime import nothing from the game or from `art/`, and use no DOM, clock, or randomness. The game imports the runtime and the pack (`web/src/hub/render/pixelkitArt.ts`); the prototype's Vite config builds the pack at build time.
- Checked in CI (`npm run check`: typecheck and tests, including golden pixel hashes).
- **Planned:** P1 (the MVP pack, appearance genome in Rust content, and integration) is part of M6; the spin-out to its own repository is P3.
