# Tuning queue

Numbers are tuned once, at the end of M6, before the tester checkpoint (owner, 2026-11-25; `AGENT_CONTEXT.md` rule 11). Until then mechanisms land with their correctness tests, balance gates they break become report-only, and what is off is listed here with its measured figure. Nothing on this list is acted on before the pass, unless the game stops being playable.

## The pass, in order
1. Re-run the audit (`mandate-cli audit gate`, `stuck`, `policy`, `believe`) and the bots at 16 and 32 seeds; record the baseline.
2. Tune, one system at a time, from the vote model outward; each change with before and after in `docs/development/balance/`.
3. Re-fit everything fitted to the numbers, each by its own command: the impact estimate's coefficients, the start ratings, the arc scorer and audition.
4. Re-enforce every gate listed below at its full threshold, re-record the pins (`scripts/repin.sh`), run the full check.

## Waiting

| Item | Target | Measured | Since | Gate |
|---|---|---|---|---|
| `knowledge_matters`: knowing the town helps | omniscient play at least 3.0 points above blind | +0.5 (16 seeds), +0.1 (32) | 2026-11-20 | reported, not enforced |
| `skill_gap` on wins: informed play out-wins rote play | a clear margin in win rate | +0.00 (16), +0.06 (32); passes on share | 2026-11-20 | reported, not enforced |
| One-party towns | few towns never change party | 41.1% never change (38.5% before the laws fix) | 2026-11-24 | none yet |
| Pacing: decision budgets reached | 95%, no climax fallback | bar held at 80%; 84.5% and 82.1% measured | 2026-11-24 | bar lowered, dated note in the test |
| Skip-only play (DEC-102 clause 7) | first-election wins no higher than canvass-only; re-election about 40–45% | 56% first-election wins, share 0.550 | 2026-11-24 | `proxy_not_optimal` passes at its old, loose bar |
| Skipping the quiet weeks (DEC-102 clause 7) | within a few points of playing every turn | not measured: no such stand-in yet | 2026-11-24 | to be added with the grader |
| The asks-only routine | a single-action routine should not win half its races | 50% wins, share 0.496 | 2026-11-24 | none; `ladder_rote` passes |
| Services laws after the laws fix | worth passing | +1.2 to +1.5 approval (+2.9 to +3.1 before) | 2026-11-24 | none |
| `audit gate`, policy part | stable | passes at services weight 1.2; 1.25–1.4 each failed one check at 3 seeds | 2026-11-24 | enforced; noisy |
| Templates that lower approval at five years | none without a stated cost | 4 remain: property tax, fiscal bridge, tech incubator, flood defences | 2026-11-24 | reported |
