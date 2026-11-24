# Housing: rent bounded by incomes, builders answer vacancy (audit fix 2)

**Date:** 2026-11-23
**Area:** economy (`economy::housing`, `content/base/economy.jsonc` → `housing`) and migration (`population::apportion`)

## Parameter and change
- Households answer rent: adults per household = 1.9 × (burden / 0.16)^0.6, within [1.2, 6.0], where the burden is a year's rent over a middle-class income. As rent climbs, adults double up and demand falls, so rent clears vacancy at a burden set by incomes. Vacancy is a real quantity (its old −5% clamp is now only a numerical guard at −50%).
- Builders: private starts = baseline × clamp(1 + 4 × (price over cost − 1) + 20 × (target vacancy − vacancy), 0, 3) × the rate factor × zoning, plus public homes. Prices keep up with inflation, so at the target vacancy and the neutral rate starts equal the baseline and stay there.
- The starting stock is sized from households at each town's starting burden. The gross rent yield at the start is 2.5% (was 4.5%), so rents start lower; prices, and so property taxes, keep their level.
- Every housing constant now lives in content (`EconomyParams::housing`, 14 fields).
- Migration splits each origin's movers, each town's arrivals over its neighbourhoods and the displaced by the largest-remainder method, ties rotating with the origin or cohort; before, every rounding remainder went to the last entry in the list. A rent floor and a rent ceiling, needed while that town ran away, never bind once it does not, and were removed.

## Evidence
`audit stuck --seeds 8` (1,440 towns, 30 years), before → after: rent runaway 112 towns (7.8%) → 0; median real rent growth ×4.3 → ×1.23; private building stalled 43% → 0.7% of town-months; housing squeezes of 10+ years 822 → 1; highest starting burden 4.67 → 0.22; highest town growth over the burn-in ×15.4 → ×1.57; no debt spirals, drained or doubled towns.
`audit policy --seeds 8`: upzoning stock −0.05% (t −0.7) → +0.52% (t 9.5); affordable housing +1.19% → +1.26%; rebuild fund +0.89% → +0.88%.
`bots --seeds 16`: enforced gates pass; idle 6.25% → 12.5% (one world of sixteen); at 32 seeds idle 9.4% → 9.4%, mixed 81.3% → 84.4%.

## Effect
- Migration now runs everywhere and follows unemployment more strongly (−0.06 → −0.17 points of population growth per point).
- Public homes draw movers (+0.45% population for +1.26% homes) and their programme jobs lift wages, so their rents do not measurably ease at five years.
- Plant closures 920 → 964 in 30 years (21 without recovery).
- The narrative pacing band (`pacing_stays_within_budgets_and_cooldowns_hold`) sees a climax in 72 of 84 chapters (85.7%; band ≥ 85%), one chapter above its edge; it was 71 before arrivals were shared fairly.
