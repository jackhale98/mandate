# Staged arcs A7: the audition

**Date:** 2026-11-26
**Status:** done for the audition, the draft's view and query, the lab command and the cost table; no save change, no content change, nothing tuned. The frames, the draft screen and the first-beat guarantee are A8's.

Design: `docs/design/STAGED_ARCS.md` §6 (DEC-101 §5); DEC-102 clause 6; DEC-041.

## What was built

- **`mandate-sim/src/arcs/audition.rs`.** A pure function of the world, the content and `Params`.
  - *Candidates* (`scan`): every town whose entry body (the legislature of the constitution's first career act) has members, the capital aside; every template whose `raise` holds there with a cast (`score::probe`), or whose run is live there. A `Candidate` holds day-0 facts only: the town, the template and title key, the `raise` arguments, the entry seat (`entry_seat`, today's start rule), the opening's tag, the heat it opens at, variety against `recent`, and its fork luck.
  - *K* (`keep`): by the prior heat × variety (the design's "cast quality" has no measure yet), no two in a town, at most two of a template. K is 8, 5, 3 by size. Under three kept, the world is advanced `SCAN_STEP_DAYS` (30) with nobody playing and scanned again, up to 36 times.
  - *Forks* (`play`): a copy of the state, re-keyed by `standin::reluck` with a luck keyed on (seed, town, template), so no fork draws from a stream the real game uses; then a `Frame` (A8's seam; today `Seated`: `seat_in` puts the player in the town's entry seat as a new game would), then a stand-in plays to 39 weeks under a `Watch`.
  - *Two passes*: passive for all K; engaged and contrarian for the best three by E × V × P.
  - *Score*: the four terms and variety from `score`, with §6.5's weights (`WEIGHTS`); the floors are measured (`Floors`) and applied only with `Params::enforce_floors` (off; `TUNING_QUEUE.md`).
  - *Draft* (`choose`): the best by score among the candidates whose passive fork built (`escalates`: the run raised and moved past its first stage, or heated above its start), no two of one template while others qualify, a standing arc in the last place if none is drafted and one qualifies.
  - *Incremental*: `Audition::begin`, then `step` plays one fork (one worker message), then `finish`. `audition` does all three. `digest` hashes everything measured.
- **`mandate-protocol` `DraftView`** (`draft.rs`): `OpeningView` (town, name, class, template, title key, `raise` args, live or not, the seat as a `PostDto`, `pitch_key` empty for A8, the `OpeningTagDto` for variety), `months_ahead`, `scan_step_days`, `opens`. No score: §6.6 keeps the scores for the lab. `QueryDto::Draft { recent }` and `ResponseBody::Draft`, wired in `mandate-wasm`.
- **`mandate-cli lab audition --seed S [--size] [--candidates] [--second-pass] [--horizon-weeks] [--enforce-floors] [--cost] [--text]`**: every candidate with its terms, exits by stand-in and floors, the draft, the digest; with `--cost`, the timings and memory. It fails if the real state's hash moved.

## Tests

- `mandate-sim` unit tests: `keep`, `choose`, `escalates`.
- `mandate-wasm/tests/draft.rs`: every no-argument view reads byte for byte the same after a draft, the state hash and command log are unchanged, and the next turn equals a twin's; the same seed gives the same draft (and the same `Draft`, to the last score, from two sessions), pinned in `web/src/components/fixtures/draft/draft_3.json`; each opening offered is a candidate as the scan read it before any fork, built in its passive fork, and the view names no score; advancing the real game `months_ahead` steps shows the same candidates.
- `web/src/engine/draft.parity.test.ts`: the Node WASM build drafts exactly the fixture.

## Cost (this machine, shared and loaded; Node for WASM; final defaults)

| | small | medium | large |
|---|---|---|---|
| fork-month, native | 6–9 ms | 12–20 ms | 21–34 ms |
| fork-month, WASM | about 2–2.5× native | | |
| scan | 0.3–1.0 ms | 1–1.7 ms | 1.6–6 ms |
| forward scan, 4 months | 18 ms | 26–46 ms | 50–117 ms |
| audition, native | 0.6–1.1 s (10 forks) | 1.1–1.9 s (10–11) | 1.7–2.9 s (9) |
| audition, WASM | 1.7–2.4 s | 2.6–4.4 s | 3.7–6.0 s |
| memory | state 4.0 MB JSON; peak RSS 40 MB native, 131–147 MB Node | 7.8–8.2 MB; 61 MB; 155–170 MB | 12.5 MB; 77 MB; 162–168 MB |

Forks run one at a time, so a fork costs one more state copy. Cut taken: the second pass is three, not five (in 16 small, 4 medium and 2 large worlds the fourth and fifth never changed a draft; a sixth of the time). Cut measured and not taken: 26 weeks instead of 39 saves 30–40% but changed 1 of 16 small, 1 of 4 medium and 1 of 2 large drafts and lowered agency. No phone was measured.

## Departures from the design, each with its reason

1. Second pass of three, not five (above).
2. "No two of one template" is relaxed to fill the draft when too few templates qualify: today only the budget round and the flood build within 39 weeks.
3. The prior leaves out cast quality (no measure), and K is the engine's constant by size class, not content (content is another task's).
4. A drafted opening must escalate in its passive fork (this task's acceptance); the design has no such rule.
5. The forward scan advances by 30-day steps with `AdvanceDays`, so A8 can bring the real game to the same day.

## Not done, and open

- A8: frames, `pitch-*` keys, the draft screen, the first-beat guarantee, applying the chosen opening to the real game (`seat_in` and `months_ahead` advances are the seam), caching the draft (a second query recomputes it), `recent_openings` in `NewGameParams` and the run file, and the scores in the run log.
- `seat_in` repeats worldgen's `seat_player` rule; one of the two should call the other once A8 settles where the start lives.
- Every world opens four months on, and drafts are thin (`TUNING_QUEUE.md`, four rows of 2026-11-26).
