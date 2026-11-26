# Lobbying is a conversation (M6 note 29)

**Date:** 2026-11-25
**Status:** done. Open points are listed at the end.

The owner asked whether whipping votes is too easy ("Taking late night discussions or lobbying right in the council menu") and decided: lobbying as a conversation. Decisions: DEC-093, DEC-091 and its principle, DEC-073, DEC-030, DEC-051, DEC-098.

## What the player does now

- **The whip panel is the count.** Each member's row shows their lean and chance of a yes, and why as far as the player knows (`WhipView.reasons`, Fluent `whip-why-*`): committed (sponsor or a declared vote), their party's line, their ward's view (once the player knows a neighbourhood's worries there), the broker leaning on them (once the broker is known), favours either way, how they see the player, and whether the player talked to them this week. A committed member shows as settled. In place of "Lobby", "Late night" and "Trade favour", each row has **Talk to them**.
- **Talk to them** opens the conversation on the bill, at its arguments, with no walk (DEC-051: the choices are `remote`). Each argument shows its chance and its cost before it is made (`TopicView.odds`, `TopicView.costs`).
- **The arguments** (`actions::lobbying::Argument`), each its own choice `choice.lobby.<bill>.<argument>`:

| Argument | Offered when | Chance | Cost |
|---|---|---|---|
| Make the case (plain) | always | the lobbying odds (opinion of the player, knowledge term) | half a slot; their opinion falls if it fails |
| Their ward | the player knows the worries of a neighbourhood in the member's ward | `arguments.ward` × the ward's support for the bill, in place of the opinion term | as plain |
| Their party's line | their party's line is on the player's side | `arguments.party` × the member's loyalty | as plain |
| Your word | the player kept an ask the member made | `arguments.word` × (2 × trust − 1) | as plain |
| The broker | the broker is known and wants what the player wants | `arguments.broker` × the member's compliance with the broker | as plain |
| Bring others | the member is an ally of the player, or their party's whip (its longest-serving member) | as plain; on success other unsettled members gain `rally_sway` × (liking of them + party loyalty) | half a slot; the player owes them a favour |
| Call in a favour | they owe the player | certain | the favour |
| Offer a favour | the player owes them fewer than two | certain | the player owes them |
| Meet after hours | the late slot is free | plain with the late-night bonus | the late slot, −8 health, legal risk +1.5, a back-room deal as a hidden fact |

- **Against tedium:** two lobbying exchanges with a member on a bill a week (`arguments.exchanges`); a settled member offers none; an ally, the party whip or the broker (note 27's lobby-the-broker choice) moves several members in one conversation.
- The one-tap lobby is retired for the player: `Act { Lobby | TradeFavor }` is refused (the variants stay so old run logs read), `ActionDto` no longer has them, and the agenda, place and booth forms are gone (a booth at the diner says lobbying is done by talking). The topics "Pitch a bill", "Ask for support" and "Ask a favour" no longer act on a colleague for a bill before the body (the arguments do); on a draft a pitch is noted.

## One function

`actions::lobbying::pitch(actor, target, bill, argument, side)` is the chance, the sway and the costs: the talk panel's label and odds, the command (which rolls the odds the choice showed, fixed before the conversation teaches anything: DEC-073), the bots (they lobby through `Talk` with the best day's argument they are offered) and the NPC AI (`npc`: the plain pitch, rolled by `lobbying::roll`, with an NPC's lobbying a whole action at full sway; NPCs no longer lobby a settled member). `npc::lobby_odds_between` is `lobbying::odds_with` with the ties term, so NPC odds are unchanged.

## Built

- Content: `actions.jsonc` `lobby.arguments` (six first values); `LobbyArguments` in `mandate-content`; the schema. Text: `talk.ftl` (labels, lines, replies, costs, outcomes), `base.ftl` (whip, reasons). Note 27's broker lobby lines named `$bill_key`, which the client renders as `$bill`: fixed.
- Engine: `actions/lobbying.rs` (rewritten: the hook, `pitch`, `roll`, `settled`, `party_whip`, `brings_others`, `arguments`); `talk/hooks.rs` (`ChoiceCost::Day` and `LateSlot`; a choice's `odds`, `costs`, `remote`, `bill`); `talk/mod.rs` (remote choices need no walk; the colleague mechanics moved out); `npc.rs`; `actions/act.rs`.
- Protocol: `WhipView` (`character`, `settled`, `exchanges_left`, `brings_others`, `reasons`; the lobby odds removed); `TopicView` (`costs`, `bill`, `remote`; a choice's `odds`); `ActionDto`.
- Web: `Chamber.tsx` (the count and Talk to them), `talk/TalkSheet.tsx` (arguments grouped by bill, a bill step with odds and costs, opened on the bill from the whip), `talk/focus.ts`.
- Tests: `mandate-save/tests/lobbying.rs` (8: knowledge opens arguments and the plain pitch is always there; shown = rolled for every argument; after hours; settled; an ally brings several; two a week; NPC symmetry; determinism and save); the talk view's shown = rolled test now covers every argument offered; worldgen and save tests moved to the conversation; `web/e2e/lobby.spec.ts` (keyboard, desktop and phone, no raw keys); the smoke test; a unit test of the grouping.

## Not done, and risks

- **The party argument beats the plain pitch at these first values** in most worlds (seed 31: 63 to 82% against 33 to 65%) at the same cost, which DEC-091's principle 1 forbids. It needs a cost of its own or a smaller weight: for the tuning pass (rule 11), listed in the balance entry.
- The whip's reasons say which way the party and the ward lean, not by how much.
- The broker argument is not yet exercised by a test of its own (it needs a world where the broker wants the player's bill).
