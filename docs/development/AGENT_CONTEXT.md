# Agent context (read this before any task)

One page. It replaces opening large documents. `AGENTS.md` is binding; this summarises it. Decisions are one file each in `docs/design/decisions/DEC-NNN.md` (index: `docs/design/DECISIONS.md`): read only the ones your brief cites.

## Hard rules
1. Engine crates (`core`, `content`, `sim`, `worldgen`, `save`, `protocol`) use no browser APIs and no `wasm-bindgen`; only `mandate-wasm` does.
2. Deterministic and single-threaded (DEC-041): no `HashMap`/`HashSet` in state, no wall clock, no OS entropy, floats via `libm`, `Money` is `i64` minor units, `u32` typed IDs never reused, randomness only from named or keyed RNG streams.
3. Tier-generic (DEC-098): town, region and nation state is keyed by `Jurisdiction`, not `MunicipalityId`. Offices, seats, powers and calendars come from the constitution by id. Ask for the player's home, posts or focus; never read `player.municipality`.
4. The UI never computes outcomes: it sends commands and renders view models.
5. The engine never produces prose: facts and Fluent keys only (DEC-054).
6. Content is JSONC, validated against schemas generated from Rust types (DEC-053).
7. A system writes only its own domain; cross-domain changes go through effect requests (OVERVIEW §4.3).
8. Never silently change an accepted decision, add an unjustified dependency, weaken a test, or edit outside your brief's `OWNS` list.
9. Saves: a format change bumps the version and adds a migration; state hashes change only on purpose.
10. Shown = rolled: any odds the UI shows are the odds the engine uses.

## Code map
- `crates/mandate-core`: typed IDs, calendar, money, deterministic math, RNG streams, stable hashing.
- `crates/mandate-content`: JSONC loading, schemas, validation, the embedded base pack.
- `crates/mandate-sim`: the simulation (state, systems, scheduler, commands, player actions).
- `crates/mandate-worldgen`: generates a country (geography, towns, people) from a seed.
- `crates/mandate-protocol`: worker DTOs and views; the source of the TypeScript types (`web/src/engine/protocol`) and knowledge redaction.
- `crates/mandate-save`: versioned saves, migrations, run logs, golden fixtures.
- `crates/mandate-cli`: the lab (`mandate-cli`): validate, run, sweeps, bots, story, doctor.
- `crates/mandate-wasm`: the only browser adapter; wasm-bindgen, `dev-tools` feature.

`mandate-sim/src` folders: `asks` (asks and promises), `characters` (people, relationships, memory), `economy` (cycle, macro, sectors), `environment` (climate, resources, hazards), `finance` (budgets, debt), `narrative` (director, decisions, storylines), `player` (slots, resources, knowledge), `policy` (policy lifecycle), `politics` (bodies, parties, elections), `population` (cohorts per neighbourhood), `talk` (conversations). Others (`types`, `validate`, `actions`, `director`, `bots`, `doctor`) are being split into folders of the same names; look for the folder first.

`web/src` folders: `components` (UI pieces), `screens` (Title, Game, Town, PlacePanel), `engine` (worker client, messages, generated protocol types), `hub` (walkable street renderer, layout v2, camera), `interior` (rooms), `map` (country map and layers), `i18n` (Fluent), `persistence` (IndexedDB, run files), `state` (stores, routing, device prefs), `places`, `dev` (designer page, `?dev=1`). Also `web/e2e` (Playwright), `landing/`, `tools/pixelkit` (pixel art).

## Where content lives
- `content/base/**`: the base pack, JSONC (parameters, actions, dilemmas, conversations, policies, layout, constitutions, scenarios).
- `content/schemas/`: generated from Rust types; never edit by hand. `cargo run -p mandate-cli -- content schema` regenerates it (and the TS protocol types come from `cargo test -p mandate-protocol`).
- `content/base/locale/en/*.ftl`: Fluent text. Every key the engine names needs an English entry.
- Add content by data first (`docs/development/EXTENDING.md`); code only for a new mechanism.

## Commands
- `scripts/check.sh fast`: fmt, clippy, content validate, generated files current (quiet; logs in `target/check-logs/`).
- `scripts/check.sh affected`: fast, plus tests of changed crates and their dependants, plus web checks if `web/` changed.
- `scripts/check.sh full [--e2e]`: what CI runs.
- `cargo run -p mandate-cli -- determinism --seed 2026 --turns 80`: twice and through save/load, one hash.
- `node scripts/wasm-parity.mjs <seed> <turns> <hash>` (after `scripts/build-wasm.sh`): native = WASM.
- `... -- bots --seeds 16`: bot strategies play; the gate ladder must hold. `--grid key=a,b` and `--set` tune.
- `... -- sweep --seeds 8 --years 20`: Monte Carlo health of NPC-only worlds.
- `... -- story --seed N --metrics`: the director's pacing and variety metrics.
- `... -- content doctor --seeds 8 --years 2`: dead, over-firing, contradictory or too-long content; `--strict`, `--corpus`.
- `... -- act-length`: turns and minutes for Act 1, with and without time skips.
- `scripts/new-entry.sh progress|balance "<title>"`: a log entry (one file each); do not edit shared tables.

## Build hygiene
- Own `CARGO_TARGET_DIR` (for example `target-<task>`), `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=2`; delete it when done.
- Own ports: `PW_PORT=<port>` for Playwright, `--port` for vite.
- Commit WIP often; never bare `git stash`; stay inside your worktree and your `OWNS` list.
- Build and test only what your task touches; leave `full` to the end or to CI.

## Report format (at most 40 lines)
Use `docs/development/WORKFLOW.md` §3. Sections: what changed (files, one line each); interfaces (new or changed types, commands, queries, content keys, save version); results (a table: check, result, numbers); risks; not done. No code recaps, no prose padding.
