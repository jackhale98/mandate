# Test tooling: repin.sh and cached seeds

**Date:** 2026-11-24
**Status:** done (no game behaviour changes; tooling and the seed lines of tests only)

Why: every merge that changes outcomes on purpose made several recorded values stale, each re-recorded by hand with its own switch, then new seeds hunted for tests that picked a seed for its world's features. The last integration spent hours on it.

## `scripts/repin.sh`

One command, in order: `tests/seeds.json`; the state-shape fingerprint (`saves.rs`, `MANDATE_RECORD_PINS=1`); the town geography pin (`town.rs` and `engineViews.test.ts`, together); the recorded fixtures (`v18_campaign_continued.mndt` with `git add -f`, `asks.fixture.json`, the layout `base.json`); the spine proof's `PINS` and `BEFORE_ARCS` rewritten in place, comments kept. It then runs the tests that read them and prints pin, old, new. `--check` changes nothing and exits 1 listing what is stale; a dirty tree is refused without `--allow-dirty`; `--wasm` also runs the web pin test (needs a fresh WASM build). Not handled: the pixel-art goldens (`UPDATE_GOLDENS=1`, art, not a world) and `towns-v2.json` (`DUMP_GEO=1`; nothing asserts it against the engine).

## Cached seeds

`crates/mandate-seeds` (dev-only; the CLI and the test crates use it): `seed_for(name, predicate)` and `seed(name)` over `tests/seeds.json`. The cached seed is used if its predicate holds; otherwise the next qualifying seed in 1..=9999 (wrapping, at most 64 candidates) with one line saying the cache is stale; if none, a panic naming the requirement. `src/requirements.rs` holds one predicate per requirement, shared by the Rust tests, the browser specs (`web/e2e/seeds.ts` reads the file) and `mandate-cli seeds check|refresh`. `check` runs in `scripts/check.sh` (fast and full) and CI's rust job; `refresh` inside `repin.sh`. Unit tests: a fresh cache is used without searching, a stale one is replaced deterministically, an unsatisfiable requirement fails with its name.

Converted: `laws_in_force_never_fall_unexplained`, `people_by_level`, `a_plant_closure_crisis_comes_and_goes`, `voices_choose_the_closers`, `opinion_and_favours_live_on_relationship_edges` (was a 37..=60 search; the cache now says 40), `a_skip_in_chunks_equals_one_in_a_go`, `filing_for_the_executive_office_names_its_race_and_rival_plan`, the visits tests' factory town and regional capital, the lab's flood fork and explorer-law tests (predicates in `crates/mandate-cli/src/lab/seed_reqs.rs`); the specs `asks` (four), `offices` and `talk` (Notebook).

Left alone: loops over a fixed band of seeds; `scandal.spec.ts` (seed 37: its scenario needs the browser's choices, no Rust predicate yet); `talk.spec.ts`'s second test (7311) and `markers`/`report` (7311), which name no feature.
