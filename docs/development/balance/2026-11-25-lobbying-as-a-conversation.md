# Lobbying as a conversation (M6 note 29)

**Date:** 2026-11-25
**Area:** lobbying, the vote model, the bot ladder

A new mechanism, not a tuning (`AGENT_CONTEXT.md` rule 11). The plain pitch reuses the lobbying parameters (`council.lobby` odds and sway; `talk.pitch.sway_share` = ½ of a lobby's sway for half a slot, so two exchanges equal the old one-slot lobby; favours `actions.lobby.trade_gain`, offered at `talk.favour.sway_share`; late night `council.lobby.late_night`, `actions.late_night_health`, `actions.lobby.late_night_legal`). The arguments' weights are first values, for the tuning pass.

## Parameter and change

`actions.jsonc` `lobby.arguments`:

| Parameter | Value | What it does |
|---|---|---|
| `exchanges` | 2 | lobbying exchanges with one member on one bill a week |
| `ward` | 2.0 | the ward argument's logit term: × the ward's support for the bill (−1..1, on the player's side) |
| `party` | 1.0 | the party-line argument: × the member's loyalty (0..1) |
| `word` | 1.5 | the kept-word argument: × (2 × trust − 1) |
| `broker` | 1.0 | the broker argument: × the member's compliance with the broker (0..1) |
| `rally_sway` | 0.1 | bring others: sway per other member × (liking of the one asked / 100 + party loyalty), 0..1 |

NPC lobbying keeps its odds and sway, and now skips a settled member (a bill's sponsor, a declared vote) and a bill not before the body, as the player's does.

## Evidence

`mandate-cli bots --seeds 16` (release), on the same base (notes 26 and 27 merged), before and after:

| Gate (enforced?) | Before | After |
|---|---|---|
| idle_cap (report-only since note 27) | fail, idle 3/16 | pass, idle 2/16 |
| effort_matters (yes) | pass, 1.0 | pass, 1.0 |
| exploring_alone_never_wins (yes) | pass, 3/16 | pass, 2/16 |
| ladder_rote (yes) | pass, mixed 0.747 / 1.0 | pass, 0.722 / 1.0 |
| ladder_learned (yes) | pass, +0.0199 | pass, +0.0237 |
| knowledge_matters (report) | fail, 0.7469 vs 0.7522 | fail, 0.7224 vs 0.7267 |
| skill_gap (report) | fail, 0.0314 | fail, 0.0295 |
| good_styles_vary (report) | fail (c) | fail (a) |
| corrupt_not_dominant (yes) | pass, 0.0 | pass, −0.0625 |
| proxy_not_optimal (yes) | pass, 0.659 vs 0.770 | pass, 0.647 vs 0.746 |

Mean vote share, before → after: idle 0.372 → 0.347, media 0.438 → 0.403, canvass 0.501 → 0.464, asks only 0.640 → 0.603, legislator 0.721 → 0.718, mixed 0.747 → 0.722, explorer 0.767 → 0.746, explorer legislator 0.771 → 0.752, explorer coalition 0.778 → 0.746, proxy 0.582 → 0.638. The best by mean share moved from explorer coalition to explorer legislator. No enforced gate failed, so none was made report-only.

## Effect

Strategies that never lobby (idle, media, canvass) lose 0.025 to 0.037 of share: the NPCs' lobbying changed (they no longer lobby a bill's sponsor or a bill not before the body), which moves bills and so everyone's record. Lobbying strategies move about as much, so the plain pitch at half a slot and half the sway matches the old lobby. The proxy gains (0.582 → 0.638): it lobbies twice per legislating turn now.

Seen in seed 31 (the worked example): the party argument's odds (63 to 82%) beat the plain pitch's (33 to 65%) at the same cost for every member whose party is on the player's side. DEC-091's principle 1 (no option strictly better) needs the party argument to carry a cost of its own or a smaller `party`: for the tuning pass, with `knowledge_matters` and the grader.
