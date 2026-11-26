# M6 F6 the proposal sheet in the web

**Date:** 2026-11-26
**Status:** done (web, fixtures, Playwright). No engine, protocol or save change; one new locale file.

## What is built

- **The proposal sheet** (`web/src/components/proposal/ProposalSheet.tsx`, store `proposalStore.ts`, `proposal.css`): opened on a proposal, it asks the engine for `QueryDto::PolicyPreview` with `proposal` and `choices` and renders the answer.
  - The package as set: yearly cost, budget share, months and support, all from the preview.
  - Each axis is a radio group. An option shows the engine's cost, support (a word, the figure only where the engine gives it), months, the lead line of its estimate as `impactLine` writes it (the engine's range or words, never a point it did not give), its description, and why it cannot be chosen. An option that cannot be combined shows no figures. A choice asks the engine again, and the last answer stays on screen while the next is asked.
  - Sliders (`param_defs`), debounced as the catalogue's.
  - Sponsors: their version by name or role, their terms (option and slider names from the preview), and their price as the engine words it (`sponsor-price-*`, a count picking the plural). "Take their version" sets their preset and sliders; the engine's `matches_current` marks it, and only then is the draft sent with `sponsor`.
  - "What it does" (`ImpactLines` for the package) and "Why?": why it is on the table (`reason_key`), support by class and the trade-offs.
  - Draft (`draft_family`, auto-routed to the town hall) with the engine's reading of the choices and sliders, or the blocked reason and who may propose it (`proposers` from the constitution). After a draft, the sheet says so and lists the bills drafted from the proposal. Each of those can be amended on the sheet (`amend_policy` with `choices`, the bill's clauses kept), with the chamber's cost where the bill is before the council and the goodwill note when moving away from an adopted sponsor's version.
  - Below `known`: a stub with the title, the reason, the sponsors by role, the blocked line and the lead with its "Go there" button.
  - The sheet asks again whenever the desk view changes (a command, a turn), and closes with a new run.
- **Entries:**
  - The desk (`DeskProposals`): "On the table" at the town hall's panel and at its drafting hotspot, beside the catalogue.
  - A place's panel (`ProposalsHere`): from `proposals_here`, at places other than the desk.
  - The Notebook's Policies tab: a proposal entry still on the desk opens the sheet.
  - A family bill's card: "Amend" opens the sheet on the bill.
  - Today has no proposal entry: the pocket's views carry no proposals, and Pocket.tsx is left to B3.
- **Family bill names:** `BillCard` and the Notebook's bill rows name a family bill with its proposal's arguments (from `BillView.family.args`), so no `{ $plant }` reaches the screen.
- **Fluent:** `content/base/locale/en/proposal_sheet.ftl` (new, `ui-proposal*`).

## Evidence

- `crates/mandate-wasm/tests/proposal_sheet.rs` writes `web/src/components/fixtures/proposal/*.json` from the base pack (seed 1, small: living wage deal known, a second proposal heard; previews at the defaults, with another option and with a sponsor's version; the drafted bill), and checks them otherwise (shown = done: the bill's cost is the option's).
- `ProposalSheet.test.tsx` (9 tests): the desk list and the query; every option's figures and estimate as given; a choice asks again; an arbitrary engine answer is shown as given; sponsors, price and taking a version; draft and amend commands; the stub; a family bill's name and its amend entry.
- `e2e/proposals.spec.ts` (desktop and phone, seed 4 small): reach turn 6, go to the town hall (the budget papers teach "Close the gap"), open it from the desk, choose an option, draft, amend, see each result; open the mine's proposal from the mine's panel; at 390 × 844 and 360 × 640 no sideways scroll and every target 44 px. `PROPOSAL_SHOT=<dir>` saves phone views.

## Left

- The spec's seed is pinned in the spec, not a `tests/seeds.json` requirement (that lives in `mandate-cli`, outside this task).
- `PocketBillView` carries no arguments; the Notebook's bill row borrows them from the desk's `BillView`. A `PocketBillView.args` field would remove that lookup.
- Today could list a proposal whose sponsor is where the player is, once the pocket view carries proposals.
