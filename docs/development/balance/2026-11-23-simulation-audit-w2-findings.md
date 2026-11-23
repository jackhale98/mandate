# Simulation audit W2 findings (no change)

**Date:** 2026-11-23
**Area:** the whole simulation (economy, housing, migration, opinion, parties, elections, policy, crises)

## Parameter and change
No parameter or behaviour change. The audit measured; fixes are a later task, tuned against the audit's commands (`docs/design/research/SIMULATION_AUDIT.md`, "Proposed fixes" and "Nightly gate").

## Evidence
`mandate-cli audit <topic>`, small worlds, the idle player, the six canonical worlds:
- Retrospective voting: re-election 81.9 / 82.0 / 80.1% by tercile of relative unemployment change (gap 1.8 pts; −0.5 pts per point, t −1.7); +0.1 pts of re-election per point of approval. A planted plant closure (unemployment +5.9 pts, approval −11) changes incumbents' wins 233 → 234 of 378 (`audit believe --seeds 8`).
- Rents: 112 of 1,440 towns (7.8%) see rent rise 4× more than the median town over 30 years; the median town ×7.9 against prices ×1.85 (seed 1); private starts under 20% of baseline in 43% of town-months; 822 housing-squeeze crises last 10+ years (`audit stuck --seeds 8`).
- Laws: 12 of 15 templates lower their town's approval at 5 years (−1.0 to −3.5 pts); service templates raise it (+3.4 to +3.7); any law flips 0.4–1.3% of home races in 5 years; upzoning changes the stock by −0.05% (t −0.7) (`audit policy --seeds 8`).
- Parties: the executive's party never changes in 30 years in 38% of towns (`audit stuck`); the share of towns not run by the dominant party falls 0.39 → 0.16 (`audit diverge --seeds 4`).
- Incumbency 81% when running (council 82%, executives 73%); competitive races 16%.
- Determinism (2026 × 80) and `bots --seeds 16` unchanged by the audit's code.

## Effect
None yet. The numbers above are the baseline the fixes are measured against; `audit gate` (nightly, non-blocking) fails on the retrospective gap and on two wrong-sign templates today.
