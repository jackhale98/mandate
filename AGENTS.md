# AGENTS.md — Mandate

A political career roguelike: a deterministic Rust simulation compiled to WASM, running in a Web Worker behind a React/TypeScript PWA. Offline-first, no backend.

## Status
The system plan (`docs/design/SYSTEM_PLAN.md`) is **approved** (DEC-067). Implementation follows `docs/development/ROADMAP.md`. Current status is in `docs/development/PROGRESS.md`.

## Read first (only what your task needs)
- Your task brief, written to the template in `docs/development/WORKFLOW.md`.
- The decisions your brief cites: `docs/design/DECISIONS.md`.
- Architecture: `docs/architecture/OVERVIEW.md`, `docs/architecture/DATA_MODEL.md`.
- Testing: `docs/development/TEST_STRATEGY.md`.

## Hard rules
- **Engine crates** (`mandate-core`, `-content`, `-sim`, `-worldgen`, `-save`, `-protocol`) must not use browser APIs or `wasm-bindgen`. Only `mandate-wasm` touches the browser.
- **Determinism (DEC-041):**
  - single-threaded simulation;
  - no `HashMap`/`HashSet` in state;
  - no wall clock, OS entropy, or `thread_rng`;
  - float transcendentals via `libm`;
  - `Money` is an `i64` of minor units;
  - typed `u32` IDs, never reused;
  - all randomness comes from named or keyed RNG streams.
- **The UI never computes authoritative outcomes.** It sends commands and renders view models.
- **The engine never produces prose.** It emits facts and Fluent keys (DEC-054).
- **Content** is JSONC, validated against schemas generated from Rust types (DEC-053).
- **Systems write only their own domain.** Cross-domain changes go through effect requests (OVERVIEW §4.3).
- **Never** silently change an accepted decision, add unjustified dependencies, weaken tests, or edit outside your task's `OWNS` list.

## Commands (available from M0)
```text
cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace
cargo build -p mandate-wasm --target wasm32-unknown-unknown
cargo run -p mandate-cli -- content validate
cd web && npm run typecheck && npm run lint && npm test && npm run build
cd web && npx playwright test --trace off   # PW_PORT=<port> to run beside another checkout
cd tools/pixelkit && npm run check
```

## Reporting
Return the report template from `docs/development/WORKFLOW.md` §3.
