# Why panels: the causal spine's first surface

**Date:** 2026-11-24
**Status:** done (the panel, the control on nine surfaces, the text, the public-results correction); three surfaces left, listed below

Design: `docs/design/CAUSAL_SPINE.md` §9.1. Decisions: DEC-108 (§8, the follow-up it names), DEC-091, DEC-054, DEC-098, DEC-051.

## What a player gets

A "Why?" on an outcome opens a sheet that reads back what led to it, newest at the top: each step one sentence (the event's own line) with its date, a phrase between two steps from the link that joins them ("because of", "after", "prompted by", "which brought to light", "which ended"), and under a step what it names as a start: the player's own act, someone's own choice, a standing rule, a pressure, an outside shock. When something the player did is behind the outcome, a line at the top says so and names the act. When more lies further back than the walk went, the panel says so and offers one "Go further back". For a race the result's factor decomposition sits under the chain.

## Built

- **Web** (`web/src/components/why/`): `whyModel.ts` (pure: reading order, the sentences, the lines for links and roots), `WhyPanel.tsx` (a sheet by the app's conventions: Escape, the close button, a swipe down or a tap outside; the keyboard goes to it and comes back; no trap; over election night too), `WhyButton.tsx` (the control: it renders nothing when the view names no event), `whyStore.ts` (the open panel: one `QueryDto::Why`, and one more at the engine's `max_depth` for "Go further back"), `why.css`.
- **Text** (`content/base/locale/en/why.ftl`): 5 links, 27 acts, 16 processes, 6 pressures, 4 shocks, history, forgotten, unknown, the two actor lines, and the panel's own strings. A Rust test lists every variant in an exhaustive match and fails when one has no line or a line has no variant.
- **Protocol** (facts only; generated TS):
  - `WhyView.depth`, `WhyView.max_depth`: how far the answer walked and how far a query may ask.
  - The event to ask about, on the views that show an outcome (optional; absent when the log no longer keeps it): `BillView.outcome_event`, `ResultView.event`, `ArticleView.event`, `ScandalView.event`, `CaseView.event`, `AskView.event`, `AftermathView.event`, `ActLineView.event` (a Legacy entry's `source_event`; a skip-summary line's where a thread names it).
  - `PUBLIC_RESULTS` and `public_result(kind)`: a public result names whom it names (the winner of a race, a law's recorded vote, the subject of a published story or of a court's ruling), in the step's arguments and as its actor. Everything else stays under DEC-091. The query writes nothing.
- **Where the control is**: the overview's event list; a race's result card (the party office and election night); a bill's card; the newspaper's stories; the pocket's scandals and cases; an ask or promise once it has ended (Today and the story sheet); a decision's aftermath; the Legacy record; the skip summary.

## Left, and why

- **The story sheet's "Recent" lines and storyline stages.** Their `EventView.id` is a fact id or a run id, not an event id. Naming the event needs a lookup from a fact to the event that recorded it for each of the fact kinds: more than a field.
- **The skip summary's elections, closures and disasters.** `SummaryItem` keeps no event id for them and no thread names them; it would be a field in `mandate-sim`'s summary (saved state).
- **A list of decided laws.** The bill card carries the control, but the app lists only drafts and bills before the body (`MyBills`), so a law's "Why?" is met today in the overview, the newspaper, the summary and the Legacy record. A "decided lately" list in the chamber would be a small product change.
- **The turn passage** shows the date only: nothing there to ask about.

## Notes for whoever reads the chains

- The director's spotlight on a town event is an event of its own with the same sentence as the event it spotlights, so such a chain opens with the sentence twice ("… which brought to light …"). The overview's list already shows both lines. Not changed here.
- An ask's asker is an actor root, gated by DEC-091: the ask's own card names them (it is aimed at the player) while the panel says "Someone's own choice" until the player has heard of them.
- `event-campaign-opened` named its month as `{ $month_key }`, which rendered as the raw placeholder in the overview's list and in every race's chain: fixed to `{ $month }` (one token in `base.ftl`).

- An ask whose name takes an argument (`Meet { $newcomer }`) shows the raw placeholder in its `event-ask-*` line: the event's view passes the name's key without the ask's own arguments. It shows in the overview's list today and so in a chain that includes such an ask. Not changed here (`views.rs`).

## Evidence

- `mandate-wasm --test why`: 18 pass (5 existing, one moved to a kind that stays gated; 13 new: the public kinds one by one, everything else gated, a line for every variant, the views' event ids, the view's depth).
- `mandate-protocol`: 251 pass; the generated TS is current.
- `scripts/check.sh fast` steps: fmt, clippy, content validate (63 files, 0 warnings), seeds (18 requirements, all fresh), generated files: pass.
- `scripts/repin.sh --check`: no pin moved. Two seeds were added to `tests/seeds.json` (`law_decided_early_with_its_chain` 1, `closure_in_a_recession_listed_early` 5); no other seed changed.
- Web: typecheck and lint clean; 384 unit tests pass (13 new in `components/why/`).
- Playwright `why.spec.ts`: 10 pass, 4 skipped by project (a law by keyboard, the paper by the Places list, a closure, election night with its factors, and the panel at 320, 360 and 390 px with wide fonts); `sheets.spec.ts` and `decisions.spec.ts` pass beside it.
- Not run here: `cargo test` for the whole workspace and `scripts/check.sh full` (the shared disk filled twice; left to integration and CI).
