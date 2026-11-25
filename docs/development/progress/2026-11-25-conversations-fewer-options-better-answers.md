# Conversations: fewer options, better answers (M6 note 26)

**Date:** 2026-11-25
**Status:** done (the engine's order, free misses with leads, trust steps, the open question, no nonsense pairings, role-written answers, the doctor's coverage check, the talk view); not done is listed at the end

Checklist note 26. Decisions: DEC-093, DEC-091 (knowledge is not a checklist), DEC-054, DEC-098, DEC-051.

## What a player gets

- **Likely subjects first.** A topic's subjects list first what the person could be expected to know, judged from what anyone can see of them (occupation, home, workplace, post, bills, their trade's issue); the rest are behind "Something else…", with search. Everything stays askable, and the order never shows who secretly knows.
- **A miss is free.** "I don't know" on a question costs no time ("couldn't help. It cost you no time."); where they know who would, they say so and the notebook gains the lead.
- **Trust sets the depth.** Who runs things, the rival and gossip about a person are always askable; a cool speaker deflects, a neutral one hints, a warm one names, a trusted one says what is at stake.
- **"What's on your mind?"** The person says what they most want to: an ask of theirs the player has not settled, their own neighbourhood's worry, or gossip about someone they know.
- **Answers from where they stand.** A councillor or the mayor asked who runs things starts from their own post and the body ("the council votes and the mayor signs") and names who decides only from the `known` step; the broker stays coy; a steward, a manager, an editor or a farmer names them in their trade's words; those far from power name the visible power and point to someone who would know.

## Built

- **Engine:** `talk::expect` (the order from public facts, `nonsense`, `who_knows`), `knows::expected_knows` (the `speaker_knows` rules without hidden ones), `talk::open` (the open question), `reply::deepest` (trust steps), free misses in `talk::talk_to`, the role's facts in `Ctx::role_args`, `LeadToDef::WhoKnows`, `TalkMechanic::Open`, `TalkLineDef::miss`, `talk_params` `miss_free`, `likely_from`, `open`, `role_groups`. The `occupation` condition gains `is: false`. `AboutChoiceView::likely`.
- **Content:** `topic.open`; who runs things always offered; a `speaker_knows` rule (an officeholder knows the broker at `known`); role-written lines for who runs things and worries; posts and bodies named mid-sentence by constitution id (`talk-post-*`, `talk-body-*`). No saved state.
- **Doctor:** `role-coverage` and `role-ungrouped` (`crates/mandate-cli/src/doctor/roles.rs`).
- **Web:** the subject step's likely list and "Something else…" step with search (`TalkSheet.tsx`, `dialogueModel.ts::subjectSplit`).

## Tests

`crates/mandate-wasm/tests/talk_note26.rs` (the menu does not depend on hidden knowledge; a miss is free and leads to someone expected to know; depth follows the steps; the open question; no nonsense pairing over three worlds; an officeholder never gives a general line; `is: false`), `doctor::roles` (a planted group with no line is reported), `TalkSheet.test.tsx`, `web/e2e/talk-menu.spec.ts`.

## Not done

- Care, trade and residents have no informative "who runs things" line of their own: nobody in those groups knows who runs things (`speaker_knows`), so theirs are role-written safe answers (the doctor lists the three).
- Beliefs that may be wrong, and the broker's levers, are note 27.
