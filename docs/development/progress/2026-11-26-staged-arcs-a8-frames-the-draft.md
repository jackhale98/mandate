# Staged arcs A8: frames, the draft screen, the first-beat guarantee

**Date:** 2026-11-26
**Status:** done for the frames, the one start rule, the opening applied to the real game, the cached draft, `recent_openings`, the scores in the run log, the first-beat guarantee and the draft screen (phone first). No save-format bump (see below). Content: a `frame` on all six arcs and six `pitch-*` lines. Nothing tuned.

Design: `docs/design/STAGED_ARCS.md` §6 and the new §15 (every correction this task made); DEC-101 §2, §3, §5; DEC-102 clause 6; DEC-098; DEC-041. A7's entry (`2026-11-26-staged-arcs-a7-the-audition.md`) listed what this task picks up.

## What was built

- **One start rule** (`mandate-sim/src/player/start.rs`): `Start` (moved from worldgen, re-exported there), `choose_seat` (the entry post's holders in a town; those for the district holding a neighbourhood, if any; then the most marginal of a major party) and `take_seat` (a fresh `PlayerState` with the start's stats). Worldgen's `seat_player` and the opening's `seat_in` both call them; worldgen's outcomes are unchanged by the move.
- **Frames** (`ArcDef::frame`: `post.seat_for`, `pitch_key`; validated) and **arriving** (`mandate-sim/src/arcs/opening.rs`): `open` = the forward scan's months (`advance`), then `arrive`: the player seated afresh in the town, in the seat for the frame's neighbourhood; what was put to the player before lapses and runs outside the new focus end without a word; `Effect::OpenArc` asks the arcs to raise the template there; one day is stepped, on which `arcs::daily` raises it by the weekly rule (`step::raise_one`, factored out of `raise`). Nothing is logged.
- **The audition on frames**: a candidate is kept only if its frame on the unforked world leaves a beat of its run on offer on turn 1 that the player is shown (`opening::first_beat`); its `Arrival` (seat, run, line's arguments) is what the view shows. Forks apply the frame first and then their own luck, so each plays the game the player would start. `Seated` is gone; `Audition::progress` counts forks.
- **Protocol and session**: `NewGameParams.recent_openings`; `QueryDto::Draft` (no arguments: the game's own `recent_openings`) and `QueryDto::DraftStep` (one fork a message, `DraftProgressView`, then the draft); `RequestBody::ChooseOpening { index }` (only before any command; answers `GameStarted`). The session keeps the draft once made. `OpeningView.pitch_key` and `args` (the run's text arguments as play begins), `post` (the frame's seat); `DraftView.opens` is the day play begins.
- **Run file**: `NewGameSetup.recent_openings` and `.opening` (`OpeningChoice`: months ahead, town, template), applied in `prepare`, so `replay` rebuilds the game without auditioning; `RunLog.draft` (`DraftLog`): every candidate's terms, its place in the draft, the one chosen, the digest.
- **Web**: `screens/Draft.tsx` (audition progress, then each opening's town, title, line, seat and tag, "Begin in <town>"; "Surprise me" takes the first; "Other openings" is a new world with the same choices; Back). The title's primary action is **Choose an opening**; **Begin** stays as the quick start (straight in, the largest town, no draft), so the 38 specs that start with it are unchanged. The last three openings begun are kept on the device (`devicePrefs.rememberOpening`) and sent as `recent_openings`.

## Tests

- `mandate-wasm/tests/draft.rs`: A7's four (on the new query), plus: **a beat on turn 1 in every drafted opening**, 16 small and 3 medium worlds, 57 of 57 openings, each opened on the real game, its arrival equal to the audition's, its arc on Today with something to do and its sheet listing a beat, and the first opening chosen through the wire; **the same seed and choice give the same game** twice, through a save, and replayed from the run file (which holds the choice and the scores); a second query reads the draft made; `DraftStep` progresses one fork a message; the view names no score.
- `web/src/screens/Draft.test.tsx` (the fixture: every card, no unfilled argument, no score, the buttons, the progress state); `Title.test.tsx` (the new primary action); `draft.parity.test.ts` (Node WASM drafts the fixture, a second query, choose, a beat on Today).
- `web/e2e/draft.spec.ts`, desktop and phone at 390×844: title → draft → choose → turn 1 shows the opening's arc on Today with "to do", its sheet's beats; no sideways scroll, every button ≥ 44 px; leaving the draft and Begin still work.

## Measured

16 small worlds (`lab audition --seed 1..16`): 3 openings in every draft (A7: 2 of 16 had three); zoning fight 24, budget round 21, plant (at its notice) 3; 14 worlds open four months on, 2 as generated. Raising the run on the first day is what lets the zoning fight build in its forks.

| Node WASM (this machine, load 10–13) | worldgen | audition | messages | first message (scan, frames) | longest fork | RSS |
|---|---|---|---|---|---|---|
| small, seeds 3, 5 | 1.9–2.3 s | 2.8–3.5 s | 11 | 0.3–0.6 s | 0.47 s | 129–146 MB |
| medium, seeds 11, 12 | 4.1 s | 4.9–5.8 s | 11–12 | 0.3–0.9 s | 0.69 s | 149–156 MB |
| large, seed 1 | 6.5 s | 8.4 s | 10 | 2.5 s | 0.84 s | 170 MB |

Choosing takes 0.1–0.3 s. A7 measured 1.7–2.4 s (small) to 6 s (large); the frames cost a world copy and a day each, and forks of the zoning fight now run where they did not. No phone was measured.

## Not done, and open

- The frame's `secret` (a hidden fact to uncover) is not built; a hidden first stage fails the guarantee, so a plant on short time is never drafted (a plant at its notice is).
- "Every world opens four months on": kept the design's forward scan. 2 of 16 small worlds now open as generated; the rest wait for the budget round's window in May (content, `TUNING_QUEUE.md`).
- The large world's first message (the forward scan and the frames) takes 2.5 s in one worker message; the scan could step a month a message.
- `recent_openings` lives in `localStorage` until the Legacy record keeps careers on the device.
- No save-format bump: `Effect::OpenArc` is consumed on the day it is posted and never reaches a save; the run log's new fields are optional.
