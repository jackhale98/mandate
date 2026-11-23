# Time skips: the proxy (M6, DEC-012, DEC-097 balance principle): No existing parameter changed. New `act_flow.proxy`: answers minor and

**Date:** 2026-11-20
**Area:** Time skips: the proxy (M6, DEC-012, DEC-097 balance principle)

## Parameter and change
No existing parameter changed. New `act_flow.proxy`: answers minor and significant decisions only, at most 8 commands a turn, leaves 34% of its picks idle; default directives ground some, fundraising some, press none, bills some, careful (no shady options, rest below health 50). New probe `directed` (the proxy alone, every turn) and an enforced gate `proxy_not_optimal` (directed mean share ≤ the median informed style's)

## Evidence
`bots --seeds 16` before/after: every existing strategy identical (shares and win rates to the last digit); directed 0.550 mean share, 62.5% wins; informed median 0.622 (explorer 0.626, campaigner 0.624, legislator 0.621, coalition 0.619): −7.3 points, below `mixed` (0.612) and near `canvass` (0.572). Enforced gates pass, as before (`knowledge_matters`, `skill_gap`, `good_styles_vary` still fail as reported-only)

## Effect
Skipping is a convenience: a skipped career does clearly worse than engaged informed play
