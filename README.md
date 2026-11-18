# Mandate

A political career roguelike set in a procedurally generated fictional country. Start in local politics, climb toward head of government, and try to win the Mandate Test: the first national verdict on your rule.

**Status:** Design complete. The final system plan is awaiting approval; there is no code yet.

- **Start here:** [`docs/design/SYSTEM_PLAN.md`](docs/design/SYSTEM_PLAN.md)
- Agent rules: [`AGENTS.md`](AGENTS.md)

- Discovery synthesis and roadmap: [`docs/design/DISCOVERY.md`](docs/design/DISCOVERY.md)
- Decisions: [`docs/design/DECISIONS.md`](docs/design/DECISIONS.md)
- Requirements: [`docs/design/REQUIREMENTS.md`](docs/design/REQUIREMENTS.md)
- Progress: [`docs/development/PROGRESS.md`](docs/development/PROGRESS.md)

## Development

Prerequisites: Rust (the toolchain is pinned by `rust-toolchain.toml`), Node 22, and `wasm-bindgen-cli` 0.2.130 (`cargo install wasm-bindgen-cli --version 0.2.130`).

```bash
./scripts/build-wasm.sh                 # engine → web/src/engine/pkg
cd web && npm ci && npm run dev         # http://localhost:5173/mandate/play/ (the game); landing page: npm run build && npm run preview
cargo test --workspace                  # engine tests
cargo run -p mandate-cli -- --help      # simulation lab
cd web && npm test && npx playwright test --trace off
```

CI builds the WASM once and caches it by a hash of its inputs (DEC-066). Pushes to `main` deploy to GitHub Pages (DEC-064). To serve from a root domain instead (e.g. Cloudflare), build with `MANDATE_BASE=/`.

## Assets
The placeholder icon is temporary; the owner's logo and loading animation will replace it. Kenney CC0 art arrives with the hub renderer (DEC-065); see `ASSETS.md` when added.
