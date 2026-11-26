# The career card and stepping away (M6 note 23)

**Date:** 2026-11-26
**Status:** done: stepping away (engine, protocol, web), the race ratings and the opening's rating, the card view, Copy and Share with a fallback, the world code and starting from it. No save-format bump: every new field is optional (the state-shape fingerprint is unchanged). Built on A8 (`career-card` from `08817e3`).

Design: `docs/development/M6_CHECKLIST.md` note 23; DEC-102 clause 6 (a start's rating is a win rate measured by the stand-ins); STAGED_ARCS §6 and §14 (the stand-ins, `lab rate-start`), §15 (the opening in the run file); DEC-006, DEC-100 (the Legacy record); DEC-054, DEC-098, DEC-041.

## What was built

- **Stepping away** (`Command::StepAway`, `CommandDto::StepAway`; `act::step_away`): at any time while the act runs (mid-turn, mid-skip, in a campaign, inside a scene). The record reads what happened since the last turn, takes a `LegacyKind::SteppedAway { race }` entry (the race the player stands in, walked away from), and closes with the endings registry's first entry, `ending.stepped_away` (`EndingWhen::SteppedAway`, tone neutral, a short epilogue). `ActState::stepped_away` holds the day; after it, `EndTurn`, `Skip`, player commands and a second step away are refused.
- **The Legacy record keeps how each arc ended** (`LegacyKind::ArcEnded`, from the `ArcEnded` event of a run in the player's focus). An exit's `legacy_key` is now its line (no longer inert); without one, the exit's own text stands.
- **Ratings** (`mandate-sim/src/career`, DEC-102 clause 6). No rating of a race at the time existed (`lab rate-start` is the lab's, and plays the opening race with a CLI-only bot beside the stand-in). The smallest honest version: `career::rate_race` plays the race on forks of the state as it stands, lucks `0..legacy.rating_plays` (content, 4), by the engaged stand-in (`standin::rate_standin`), and keeps the win rate, the plays and the three factors of the vote model's decomposition that drove it. `standin::rate` no longer files for a race the player already stands in (no change to `rate-start`).
  - **A race** is rated by the session as it opens with the player in it (after any command; `career::races_open`), and kept in the run file (`RunLog.ratings`). The state is only read: no state hash changes.
  - **The opening** is rated the first time the card is asked for, on the world as play began, rebuilt from the run file (`NewGameSetup`, so the drafted opening included), then kept (`RunLog.opening_rating`). A run loaded under other rules keeps its ratings but cannot rebuild its start: its opening says "not rated".
- **The card** (`QueryDto::CareerCard` → `CareerCardView`, `mandate-protocol/src/card.rs`): the character, party, town and kind of world; the opening (town, arc, months ahead, seat) with its rating and facts; every race entered by election day with its rating at the time and its result (`won`, `lost`, `walked_away`, `undecided`); the highest post (latest act of the ladder, then an office over a seat, by constitution id) and the years served (days holding a post, from the record); the notable decisions; how each arc ended (and those still under way); the ending and when, as what; the figure: wins less the wins the ratings expected, over the races rated and decided (`career::tally`, in thousandths); the world code and the build. Unfinished when the player stepped away. Nothing personal: no run id, device or time of day.
- **The world code** (`career::code::WorldCode`): `M1.<seed>.<size><knowledge>.<scenario>.<constitution>.<storyteller>.<opening>.<build>.<check>`, base 36, content ids as four digits of their stable hash resolved against this build's registries, a three-digit check. Read with `QueryDto::WorldCode` (what it starts, `same_build`, or why not: `malformed`, `version`, `check`, `unknown`, `other_build`); started with `RequestBody::StartFromCode` (the one new-game path, `start_game`, opening applied by `NewGameSetup::prepare`).
- **Web**: `components/card/` (`CareerCard`, `cardText`, `cardImage`, `share`, `StepAway`, `CodeStart`). The epilogue carries the card once the act is over (progress `progress-card` while the opening is rated). **Copy** writes the text block; **Share** uses the Web Share API with a PNG drawn on a canvas and the text, and where there is no share sheet (or it is refused) saves the PNG and copies the text. The ⋯ menu has **Step away from politics** with one confirmation (it says when a race is under way); the title has **Start from a code**. Text in `locale/en/card.ftl`.

## Tests

- `mandate-wasm/tests/career_card.rs`: stepped away on the first turn (the ending, the record, no more turns, the same card through a save and when the opening is rated in another session from the file saved before the card, the code read back); a drafted opening's code starts the same game (the same state hash at the start and after the first turn); mid-race and mid-scene (seed 4242, a round into the campaign debate: the race was rated as it opened, the card names it walked away from, the figure counts it in neither); every key in the English locale; fixtures `web/src/components/fixtures/card/{first_turn,mid_race,code}.json`.
- `mandate-sim/src/career`: the tally, win rates in thousandths; the code round-trips, a typo, another build, a later form.
- Web: `components/card/CareerCard.test.tsx` (both fixtures: every section, no unfilled argument, unfinished, the walked-away race, the text block, Copy, Share and its fallback, the confirmation); `engine/card.parity.test.ts` (Node WASM makes the native card, to the last rating).
- `web/e2e/career-card.spec.ts`, desktop and phone 390×844: Begin → ⋯ → Step away (Keep going first) → the epilogue with the card, unfinished → Copy (the clipboard holds the text) → Share with no share sheet (a PNG download, the text copied) → title → paste the code (lower case, spaces) → Play this world: the same character in the same town; a mistyped code says so. No sideways scroll; every button ≥ 44 px.

## Measured (Node WASM, this machine under load 9)

| What | Time |
|---|---|
| The card the first time, seed 3 small (world rebuilt, opening rated on 4 plays of 10 months) | 3.4 s |
| A race rated as it opens (4 plays of the 8-week campaign), on the command that opens it | 1.1 s |
| Native, `career_card.rs` (3 tests, incl. a draft and 80 turns to the debate) | 19 s |

## Not done, and open

- Crises (`crises.jsonc`) have no ending event; the card says how each staged arc ended, which A6 makes of the crises.
- The ratings use the engaged stand-in only (the informed bot of `rate-start` is not in the WASM build); 4 plays a race. Both numbers are for the tuning pass.
- A race the session never saw open (one opened and decided inside a single skip chunk, or a run loaded under other rules) is listed without a rating and left out of the figure.
- Stepping away mid-race and mid-scene is proved in the engine and the session (`career_card.rs`); the browser test steps away on the first turn, and whether the ⋯ menu is reachable while a scene's sheet is open was not checked in the browser.
- A race is rated on the command that opens it (in play, the end of the turn the campaign opens): about 1 s more in WASM on a small world, more on a large one. It could move to a background query, one play a message, as the draft does.
- The first card costs a world rebuild (about 2 s small, 6 s large) in one worker message, with progress.
- The fixtures' codes and builds carry the content hash: any content change makes them stale (`MANDATE_WRITE_FIXTURES=1 cargo test -p mandate-wasm --test career_card`).
