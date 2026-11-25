# Conversations: free misses and the open question (M6 note 26)

**Date:** 2026-11-25
**Area:** conversations (`content/base/conversations/params.jsonc` → `talk_params.miss_free`, `likely_from`, `open`; `talk::talk_to`)

## Parameter and change

A mechanism change; no number was tuned.

- `miss_free: true`: a reply on a topic that informs whose substance is a safe answer (a fallback or a `miss` line) costs no time.
- `likely_from: heard`, `open.order: [ask, worry, gossip]`: the order of the talk panel and the open question (neither costs or rolls anything of its own).
- A `speaker_knows` rule: an officeholder knows the broker at `known`.

## Evidence

After only: the before run on the base branch was not cheap (a second release build) and was not made. `MANDATE_LAB_THREADS=2`, `bots --seeds 16`.

| Strategy | Mean share | Win rate |
|---|---|---|
| explorer_legislator | 0.743 | 0.938 |
| explorer_coalition | 0.742 | 0.938 |
| explorer | 0.739 | 0.938 |
| mixed | 0.725 | 0.938 |
| legislator | 0.715 | 0.938 |
| corrupt | 0.706 | 0.938 |
| directed | 0.633 | 0.875 |
| proxy | 0.579 | 0.750 |
| asks_only | 0.576 | 0.812 |
| canvass | 0.453 | 0.500 |
| media | 0.383 | 0.188 |
| idle, explore_only | 0.320 | 0.062 |

Gates: every enforced gate passes (idle_cap, effort_matters, exploring_alone_never_wins, ladder_rote, ladder_learned, corrupt_not_dominant, proxy_not_optimal). Not enforced before or after: knowledge_matters (false), skill_gap (false); good_styles_vary true. explorer − mixed share 0.014; directed − informed median share −0.108.

Spine proof: seed 31 explorer_legislator 3334 → 3341 events (3188 → 3205 with no arc content); the other careers' event streams did not change.

## Effect

No gate moved to report-only. The explorer's misses are now free, so it fits a few more actions into a career.
