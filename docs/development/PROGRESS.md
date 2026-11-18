# Progress

**Phase:** Implementation. The plan is approved (DEC-067).

## Milestone status
| Milestone | Status |
|---|---|
| M0 Foundation | **Done**, 2026-11-18 |
| M1 World and lab | **Done**, 2026-11-18 |
| M1s Rendering spikes | Map: Canvas 2D in production (fast, pixel-art); hub spike with M6 |
| M2 Society, economy, environment | Next |

## M0 summary
- **Rust workspace:** 8 crates, with determinism lints enforced (banned `HashMap`, `std` float transcendentals, wall clock).
- **Core primitives:** typed IDs, calendar, fixed-point money, `libm` math, ChaCha RNG streams plus keyed access, BLAKE3 hashing.
- **Content:** JSONC pipeline with line-accurate errors, generated schema (`content/schemas/`), and validation that includes Fluent keys. The base pack is embedded in the WASM.
- **Engine:** 13-phase cadence scheduler, effect queue, event log, state hash. Stub systems exercise every mechanism.
- **Saves:** versioned, compressed, integrity-hashed, with sync metadata (DEC-044 rev. 2, DEC-052). A shape-fingerprint test guards the format version.
- **Protocol:** DTOs with TypeScript generated through ts-rs. The WASM session is natively testable.
- **Lab CLI:** `content validate|schema`, `run`, `determinism`, `save-info`.
- **Web:** React 19, TypeScript 6 (strict), Vite 8 PWA; engine worker; typed client; Dexie `SaveStore`; Fluent text; Ironman autosave every turn; offline play.
- **Verified:**
  - same seed gives the same hash, twice;
  - save, load, continue gives an identical hash;
  - **native = WASM** state hash;
  - Playwright on desktop and phone: new game → end turn → save → reload offline → resume with an identical hash.
- **CI:** cached WASM job; Rust, web, and E2E jobs; Pages deploy from `main`.

## M1 summary
- **World generation pipeline:**
  1. Terrain: warped fBm, ridged mountain ranges, sea sides.
  2. Climate: latitude and lapse rate; prevailing wind with rain shadow.
  3. Hydrology: priority-flood, D8 routing, rivers, lakes.
  4. Whittaker biomes.
  5. Resources and hazards.
  6. Settlement scoring and rank-size populations.
  7. Terrain-cost territories.
  8. Regions grown along natural borders.
  9. A* road network (MST plus shortcuts) with rail spines.
  10. Neighbourhoods.
  11. Wards and regional and national districts (contiguous, balanced).
  12. Places, including the parliament in the capital.
- **Scale (medium):** about 10 regions, 56 municipalities, about 400 neighbourhoods, about 500 districts, and about 800 places. About 90 ms in WASM.
- **Five canonical presets**, tested for their intended biases.
- **World invariants are property-tested:** ownership, contiguity, population accounting, district partitions, road connectivity, and determinism.
- **Lab:** `mandate-cli gen --png` renders a map; `inspect` shows a municipality or region.
- **Web:** a Map tab with a pan, zoom, and pinch pixel map and tap-to-inspect towns; an Overview tab with a region and town list as the accessible alternative; a town panel showing climate, resources, hazards, neighbourhoods, and places; a scenario picker; a generation progress bar.
- **Saves:** format v2 (world model change). M0 saves are rejected with a clear message (pre-release policy).

## Deferred from M0 (tracked)
- Module-boundary architecture test: added in M2, when domain modules exist.
- React Compiler evaluation: M6 performance pass.

## Owner notes
- Logo and loading animation: to be supplied by the owner. The placeholder icon is in `web/public/icons/icon.svg`; run `node web/scripts/gen-icons.mjs` after replacing it.
