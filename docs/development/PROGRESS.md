# Progress

**Phase:** Implementation. The plan is approved (DEC-067).

## Milestone status
| Milestone | Status |
|---|---|
| M0 Foundation | **Done**, 2026-11-18 |
| M1 World and lab | Next |

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

## Deferred from M0 (tracked)
- Module-boundary architecture test: added in M2, when domain modules exist.
- React Compiler evaluation: M6 performance pass.

## Owner notes
- Logo and loading animation: to be supplied by the owner. The placeholder icon is in `web/public/icons/icon.svg`; run `node web/scripts/gen-icons.mjs` after replacing it.
