# Hazards by neighbourhood (M6 G1, DEC-090 D8): Rolls and town damage unchanged; a strike's damage is shared by neighb

**Date:** 2026-11-19
**Area:** Hazards by neighbourhood (M6 G1, DEC-090 D8)

## Parameter and change
Rolls and town damage unchanged; a strike's damage is shared by neighbourhood exposure × residents (residents' mean of the relative hit 1, capped at `economy.hazards.max_relative` 4, excess spread; at most `max_hood_damage` 0.35 of a neighbourhood's homes). Permanent departures as before (`leave_share` 0.25 of the damage, mobile adults; none for wildfires), now from the struck neighbourhoods; new: `temporary_share` 0.75 of the damage stay in less-hit neighbourhoods (or the next town) and return at `return_rate` 0.12 a month; damaged homes repaired at `repair_rate` 0.08 a month (a record for views; the stock recovers through construction as before). Opinion's disaster term and the felt recovery concern × the neighbourhood's relative hit

## Evidence
Before → after, same seeds (medians: population change per decade, unemployment, inflation, debt change; disasters per 30 years). CI sweep 8 × 20: 0.65% → 0.65%, 4.65% → 4.65%, 2.25% → 2.26%, +0.016 → +0.013; disasters 15 → 15. 8 × 30, no scenario: unemployment 4.42% → 4.47%, disasters 17 → 17. Scenarios 8 × 30 (unemployment): industrial decline 6.31% → 6.42%, wealthy coast 4.57% → 4.58%, tech growth 4.57% → 4.63%, drought farmland 4.49% → 4.48%, resource frontier 5.49% → 5.55%; disasters per scenario identical; all pass. Bots 16: canvass, corrupt, legislator, mixed, proxy 75 → 81.25%; media 6.25 → 6.25%; idle 6.25 → 6.25%; best − median 0 → 0; sole best: canvass 8 → 7, mixed 9 → 9, corrupt 5 → 4, proxy 1 → 0; top share 0.6, variety passes. Concentration (8 small worlds, 58 towns with flood index ≥ 0.2, a 2% flood): the most exposed neighbourhood takes 1.90× the town's average (median; 90th percentile 2.85×), the least 0.31×

## Effect
No tuning needed: every gate and band passes with town-level outcomes within noise; the flood-prone district now takes about twice its share of a flood and reacts to it, the spared ones barely
