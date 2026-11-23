# Mandate

Mandate is a political career roguelike set in a procedurally generated fictional country. You start as a ward councillor in a generated town and try to climb toward head of government; the run's summit is the Mandate Test, the first national verdict on your rule. The country is a deterministic simulation (population, economy, environment, parties, elections, characters, and a narrative director) written in Rust, compiled to WebAssembly, and run in a Web Worker behind a React and TypeScript web app. It works offline and has no backend.

**Status:** in development. Act 1 (local politics) is playable in the browser: generated worlds, a town you move around, council votes, budgets, elections, characters with memories, and dilemmas from a narrative director. Milestone status is in [`docs/development/PROGRESS.md`](docs/development/PROGRESS.md); the plan is in [`docs/development/ROADMAP.md`](docs/development/ROADMAP.md).

## Live site

Pushes to `main` deploy to GitHub Pages:

- Landing page: <https://jackhale98.github.io/mandate/>
- The game: <https://jackhale98.github.io/mandate/play/>
- Walkable town prototype (M6 groundwork, unlinked), with the town generator (layout v2, DEC-090) and the pixel art: <https://jackhale98.github.io/mandate/spike-hub/>. The game itself now has the same town: its map, and "Walk the streets" from the map.
  - Pick a world with `seed`, e.g. [`?seed=2026`](https://jackhale98.github.io/mandate/spike-hub/?seed=2026), or press "New town".
  - `town=largest` (or `smallest`, or a town's name) visits another town; `dress=grow`, `shrink` or `closed` shows the monthly dress pass; `art=placeholder` shows the stand-in art.

## Reporting a problem

If something goes wrong in the game, please send a report file:

1. Open the pocket and choose **Report a problem**. If an error message is showing, it has a **Report a problem** button too, and so does the foot of the title screen (use that one if the game crashed and you reloaded).
2. Say what happened, if you like: what you did, what you expected, and what you saw.
3. Choose **Save report file**. On a computer the file downloads; on a phone, the share sheet opens so you can save it or send it straight on.
4. Send the file (`mandate-report-<seed>-<date>.json`, about 1–2 MB) to the developers however you like.

The file holds this game's seed and settings, every move you made in it, the game as it is now, the last few error messages, your screen size and your browser's name, and what you wrote. Nothing personal beyond that. The game never sends it anywhere: you decide where it goes. With it, the developers can replay your game exactly as it happened (see "Reproducing a tester's bug" in [`TEST_STRATEGY`](docs/development/TEST_STRATEGY.md)).

## Repository layout

| Path | What it holds |
|---|---|
| `crates/` | The Rust workspace: `mandate-core` (IDs, calendar, money, deterministic math, RNG), `mandate-content` (JSONC loading, schemas, validation), `mandate-sim` (state, domain systems, scheduler, commands), `mandate-worldgen` (world generation and history burn-in), `mandate-save` (versioned saves and migrations), `mandate-protocol` (worker DTOs, TypeScript generation), `mandate-wasm` (the browser adapter), `mandate-cli` (the simulation lab) |
| `content/base/` | The base content pack (JSONC registries and parameter blocks, Fluent text in `locale/en/`); `content/schemas/` holds the generated JSON Schema |
| `web/src/` | The game: screens, sheets, the pocket, the engine worker and typed client, saves (IndexedDB), Fluent text, the map, and `hub/` (the walkable-town groundwork for M6) |
| `web/landing/` | The landing page at the site root (plain HTML, CSS, a little TypeScript) and the scripts that capture its images from the game (`npm run capture` in `web/` retakes them all) |
| `web/spikes/hub/` | Throwaway rendering prototypes: the walkable town on the real engine, and the Canvas 2D vs PixiJS comparison |
| `web/e2e/` | Playwright tests |
| `tools/pixelkit/` | The pixel-art tool (working name, DEC-084): hand-drawn parts composed by code, and Mandate's art pack |
| `scripts/` | `build-wasm.sh` (engine → `web/src/engine/pkg/`) and `wasm-parity.mjs` (native = WASM state hash) |
| `docs/` | Design, architecture, and development docs (below) |

## Build, run, and test

Prerequisites: Rust (the toolchain is pinned by `rust-toolchain.toml`, with the `wasm32-unknown-unknown` target), Node 22, and `wasm-bindgen-cli` 0.2.130 (`cargo install wasm-bindgen-cli --version 0.2.130`).

```bash
# Engine
cargo fmt --check && cargo clippy --workspace -- -D warnings && cargo test --workspace
cargo run -p mandate-cli -- content validate   # check the content pack
cargo run -p mandate-cli -- --help             # the simulation lab (sweeps, bots, story, gen, inspect, ...)
./scripts/build-wasm.sh                        # build the engine for the web app

# Web app (after build-wasm.sh)
cd web && npm ci
npm run dev                                    # the game at http://localhost:5173/mandate/play/
npm run typecheck && npm run lint && npm test
npm run build && npm run preview               # the whole site: landing page at /mandate/, game at /mandate/play/
npx playwright test --trace off                # end-to-end tests (serves the built site on port 4173)

# Walkable town prototype (from web/, after npm run build)
npx vite build --config spikes/hub/vite.config.ts --outDir ../../dist/spike-hub --emptyOutDir
npx vite --config spikes/hub/vite.config.ts    # or run it on its own, on port 5199

# Pixel-art tool
cd tools/pixelkit && npm run check             # typecheck and tests (uses web/'s TypeScript, so run npm ci in web/ first)
```

To serve from a root domain instead of `/mandate/` (for example on Cloudflare), build with `MANDATE_BASE=/`; set `MANDATE_ORIGIN` for the landing page's absolute share links.

## CI

GitHub Actions (`.github/workflows/`):

- **CI** on pushes to `main` and `claude/**` branches and on pull requests: one job builds the engine to WASM and caches it by a hash of its inputs (DEC-066). The Rust job runs fmt, clippy, tests, content validation, a check that generated schemas and TypeScript types are current, the determinism check, a quick Monte Carlo sweep, the bot tournament, and native = WASM parity. The web job runs typecheck, lint, unit tests, the pixel-art tool's checks, the site build (with a landing-page size budget), the town prototype build, and Playwright on desktop and phone. Pushes to `main` then deploy `web/dist` to GitHub Pages (DEC-064).
- **Nightly:** sweeps across every scenario, a larger bot tournament, and property tests with more cases.

## Documentation

- Start here: [`docs/design/SYSTEM_PLAN.md`](docs/design/SYSTEM_PLAN.md) (the approved plan) and [`AGENTS.md`](AGENTS.md) (rules for agents).
- Decisions: [`docs/design/DECISIONS.md`](docs/design/DECISIONS.md). Requirements: [`docs/design/REQUIREMENTS.md`](docs/design/REQUIREMENTS.md). Vision and UI: [`docs/design/GAME_VISION.md`](docs/design/GAME_VISION.md), [`docs/design/UI.md`](docs/design/UI.md). Research: `docs/design/research/`.
- Architecture: [`docs/architecture/OVERVIEW.md`](docs/architecture/OVERVIEW.md) and [`docs/architecture/DATA_MODEL.md`](docs/architecture/DATA_MODEL.md).
- Development: [`ROADMAP`](docs/development/ROADMAP.md), [`PROGRESS`](docs/development/PROGRESS.md), [`WORKFLOW`](docs/development/WORKFLOW.md), [`TEST_STRATEGY`](docs/development/TEST_STRATEGY.md), [`EXTENDING`](docs/development/EXTENDING.md) (adding content and code extension points), [`BALANCE_LOG`](docs/development/BALANCE_LOG.md).

## Art

The app icon and loader are Mandate's own (DEC-081). The walkable town uses placeholder art drawn in code until the pixel-art tool's pack replaces it (DEC-084).

## Licence

Open source: code under MIT or Apache-2.0, text and art under CC BY 4.0. See [LICENSE.md](LICENSE.md).
