# Asks in conversation and talk hooks

**Date:** 2026-11-23
**Status:** Done (M6 note 12; IMPLEMENTATION_PLAN S4).

## The owner's report, reproduced
- **Silent official.** `ask.introduce_me` had no choice and no lines of its own. Asking the official `topic.about` the asker was the nearest thing; when the player had not heard of the asker (an offer teaches nothing), the command was refused, the dialogue dropped the question, and no `Deed::Talk` was recorded. Even a talk that worked moved "Next:" only at the next day's `asks.daily`.
- **Broken with days left.** The step with the official never registered, so the later talk with the asker counted for nothing (needs are met in order). A governing turn is 14 days: "End turn" with 12 days left ran past the deadline and broke the ask. Reproduced with the real offer path (small worlds, seeds 0, 3, 4, 5, 6 and 11: the talk about the asker refused, the ask broken at the end of the next turn); the regression tests are in `crates/mandate-wasm/tests/asks_talk.rs`.

## What changed
- **Talk hooks** (`mandate-sim/src/talk/hooks.rs`): a `Hook { owner, offer, chosen }` in `HOOKS`; `offer` lists `Choice`s for a speaker (id `choice.<owner>.<local>`, label and player's line keys with args, replies with conditions, a cost); `chosen` applies the owner's effect in its own domain. `talk::apply` routes a `choice.*` topic to it: same presence, late-night and time checks, the first reply whose conditions hold, the exchange in the log, the outcome line. `talk::check_choice` is the panel's check. The talk panel lists choices first (`TopicView.mechanic = "choice"`, `name_args`, `say_key`).
- **Asks use it** (`asks/hook.rs`): an accepted ask whose current need is a talk offers its template's choice (`NeedDef::Talk { choice }` in `asks.jsonc`). A reply that agrees meets the need; one that does not leaves it open (`tries` counts refusals) and the ask view says so (`AskView.note_key = ui-ask-talk-refused`). A talk need with a choice is no longer met by any talk with the person.
- **At once**: `asks::settle_now` (the daily step, same date and keyed draws, effects through `apply_now`) runs after every player command (`actions::apply`).
- **Never silent**: a topic whose reply composes to nothing says `talk-nothing-to-say`.
- **Routes**: a talk need's `how` is "talk" with where the person is; watch needs' is "wait".
- Content: choices for `ask.introduce_me` (official: warm, curt or no by opinion; asker: thanks), `ask.second_my_bill`, `ask.meet_the_newcomer`; validation requires a choice on every talk need, keys and args that exist, an agreeing reply and an unconditioned last reply.
- Web (minimal): `components/asks/choiceText.ts` (label and line with args), one call each in `TalkSheet.tsx` and `dialogueModel.ts`, the note in `Asks.tsx`. Bots use the ask's choice for talk needs.

## Results
- Tests: `asks_talk.rs` (bugs 1 and 2, decline, hooks listing, at-once, no silent exchange, need-path audit); workspace tests pass.
- Determinism 2026 × 80: identical twice and through save/load; parity 777 × 60 native = WASM.
- Bots 16 seeds, enforced gates pass. mixed 0.614 → 0.615 share, 0.81 → 0.81 wins; asks_only 0.504 → 0.504, 0.56 → 0.56.
- Doctor `--seeds 0`: no errors or warnings; the new lines pass `--strict-prose`.
- Save: no shape change (no v26).

## Left
- EXTENDING's "Adding an ask" should mention `choice` on talk needs.
- Crises, casework, secrets and families can register hooks; none do yet.
