# Laws pay off: delivered results in approval (audit fix 3)

**Date:** 2026-11-24
**Area:** opinion, concerns, policy ledger, housing, finance (audit fixes 3, 5, 8; DEC-111 answer 6)

## Parameter and change

- `opinion.experience.delivered` (new): `jobs` 8 (per unit of jobs added over the labour force, working ages), `income` 10 (per unit of income paid or raised over the class's income, that class), `homes` 120 (per unit of homes added a year over the stock, low and middle classes; the rent a cap holds down is credited as income to tenants).
- `opinion.experience.service_budget` (new) 1.0: the town's budget against what its budget rule aims for (1 = need).
- `opinion.experience.services` 2.0 → 1.2.
- `concerns.jsonc`, `concern.services`: indicator `budget_stress` → `service_shortfall` (new), level baseline 0.1 → 0.25, slope 2.0 → 2.5.
- `economy.attribution` (new): `homes_net` 0.6, `jobs_net` 0.8, `wage_floor_net` 0.65 (the ledger's "added" estimates).
- `economy.housing.capped_vacancy_signal` (new) 0.5; the price now capitalises rent at a user cost that includes the property tax in force.
- `economy.borrowing` (new): `spreads` [0.012, 0.008, 0] (as the code had), `free_ratio` [0.2, 0.5, 4.0], `per_ratio` 0.02, `max_extra` 0.06.
- Emergency rent: the freeze is `rent_cap` (`loosest` 0.03, `per_unit` −0.015) in place of `zoning` −0.08. Fiscal bridge: its levy grievance falls on the middle (0.5) and high (0.7) classes, not the rebated poorest.
- The pollution decision's inspection: `{ "type": "inspection", "pollution": 0.15, "output": 0.08, "months": 12 }`.

## Evidence

Tuning rounds (`audit policy --seeds 8`, templates lowering approval at five years): 11 at the start; 9 with delivered credit alone (services weight 2: every law keeps out an NPC services law, which outweighed any other result); 4 at services 1.0 with the cap credit, but `audit gate` failed on police staffing (3 seeds); services 1.1 and 1.2 pass the gate (1.25, 1.3, 1.4 each fail one noisy three-seed check), so 1.2. `jobs` 10 failed the gate on transit attractiveness, so 8. Full before/after tables: `SIMULATION_AUDIT.md`, "After fix 3".

## Effect

Templates lowering approval at five years: 11 → 4 (property tax, fiscal bridge, tech incubator, flood defences: material effect negative or nil). The hardship fund (+3.9 low class), affordable housing (+7.2), the closure response (+13.2) and the wage floor (+14.4) raise approval among those they reach. Ledger over twin 0.78–1.32 for every row (homes were 0.32 and 0.38). `audit gate` passes. Retrospective checks hold (gap 23.5 pts, re-election 77.3%). Bots: canvass-only 0.19 / 0.22 first-election wins at 16 / 32 seeds; enforced gates pass; `knowledge_matters` and `skill_gap` still fail.
