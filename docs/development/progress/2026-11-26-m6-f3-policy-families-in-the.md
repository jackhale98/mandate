# M6 F3 policy families in the protocol, with the impact estimate on proposals

**Date:** 2026-11-26
**Status:** done (protocol, TS types, query and command plumbing). No save change, no outcome moved. The base pack still has no families (F4), so the views are proved on the sample families of `mandate-content/tests/fixtures/families`.

## What is built

- **`ProposalView`** (`crates/mandate-protocol/src/proposals.rs`, new): id, family, name and description keys with `args` (slot names; a person the player has not heard of is an empty name; a place with no proper name as keys, a named plant as `<slot>_key` = `plant-name-<form>` with `stem` and `works_key`), category, `at: JurisdictionDto`, level and `KnowledgeView` (the lead), reason key, bindings (slot, kind, name or key, `LocationView`, `PersonRefView`), the default cost and months, sponsors, status and lapse reason, opened, bills drafted from it, `requires`, `proposers`, `can_draft` and `blocked_key`.
- **`OfficeView.proposals`**: open proposals of the focus jurisdictions the player has heard of. **`PlacePanelView.proposals_here`**: those bound to the place, to the neighbourhood whose streets these are, or brought by a sponsor who works there.
- **The extended preview.** `QueryDto::PolicyPreview` gains optional `proposal` and `choices`. `PolicyPreviewView` gains optional `impact` (also for a template, except a situational stub) and `family: FamilyPreviewView` (the proposal, the choices as read, the sliders of the package as chosen, months, `axes` of `OptionPreviewView` and the sponsors' `SponsorTermsView` with `PriceView`). Each option is the whole package with that option swapped in: cost, budget share, months, support, side effects, `impact`, availability with a key (`option-unavailable-needs`, `-excludes` with `with_key`). One composition per option, the chosen package shared: seven for three axes of three.
- **Knowledge (DEC-091).** Unknown: left out everywhere, and the preview is an error. Heard: a stub (title, reason, sponsors' roles, lead; no menu, no figures; `proposal-blocked-heard`). Known: the menu, sponsors by name with their versions and prices. Support is gated by `policy_support` as everywhere: the higher of the proposal's own subject and the family's template subject in its town (what a town hall or the chamber on a family bill teaches). The impact is the draft bill's estimate, ungated beyond `known` until the estimate's task 4.
- **Gating (DEC-098).** `proposers` are the posts of the proposal's tier whose constitution powers include the family's `requires`; `can_draft` makes the `DraftFamily` command's checks in its order (open, known, the power at `at`).
- **Commands.** `CommandDto::DraftFamily { proposal, choices, params, sponsor }` and `AmendPolicy.choices` (optional), mapped in `mandate-wasm`. `BillView.family` (`BillFamilyView`: proposal, choices, sponsor, args, area) so a family bill's sheet can amend its choices.
- **Notebook.** A proposal's entry carries its title (`policy_key`) and its arguments.
- **Engine helpers** (`crates/mandate-sim/src/policy/drafting.rs`): `Unavailable` and `unavailable` (the option checks `prepare` makes, typed; `prepare` now calls it, same refusals), `prepare_any` (a lapsed proposal's sheet), `probe_bill` (the bill a draft would make, not stored; `make_bill` shares its constructor).
- **Fluent:** `content/base/locale/en/proposals.ftl` (blocked, unavailable and price keys).
- **Web:** generated types, and `routing.ts` maps `draft_family` to the draft route.

## Evidence

- `cargo test -p mandate-wasm --test proposals` (8 tests): knowledge levels, options equal a preview with that option chosen, the cost shown equals the drafted bill's, `proposers` equal the constitution's posts and `can_draft` agrees with the command out of office, an excluded option says so, `proposals_here`, JSON round trips (old clients' previews still parse), views never write, determinism.
- `MANDATE_PRINT=1 cargo test -p mandate-wasm --test proposals worked_example -- --nocapture` prints the worked example.

## Left

- Nothing teaches `policy_support` for a proposal itself yet; until then support classes come from a town hall or the chamber on a bill of the family.
- §1.1's `detailed` "who else backs it" is not shown beyond exact support.
- F6: the sheet; F4: real families and their Fluent.
