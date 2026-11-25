# Industries and named plants (I1)

**Date:** 2026-11-24
**Area:** economy (`content/base/industries.jsonc`, `economy.jsonc` → `industries`; `economy::industries`, `economy::plants`); DEC-110, DEC-109 answer 2.

## Parameter and change

A mechanism change, with one early pass over its numbers; then the owner's rule took effect (mechanisms first, tuning once per wave) and nothing was tuned further.

- **Before:** a plant closure was a monthly roll on a sector whose demand driver was under 0.85 (chance 0.3 × the gap), removing 8 to 25% of the sector's capacity, on the sequential economy stream.
- **After:** three sectors are sixteen industries, each with its own demand; a named plant goes on short time (`m < 0.90`, 2 months), gets a dated notice (`m < 0.78`, 9 months; 4 to 9 months' notice, one keyed draw), and is reprieved (`m ≥ 0.84`, 2 months) or closes on the date. Works reopen from idle labour and from booms. No draw is on the sequential economy stream (every later draw of that stream shifted once).
- **Values that differ from the design's first table** (`INDUSTRIES.md`, departures 7 to 11), each from one audit run:

| Parameter | Design | Set | Why (audit stuck, 48 worlds × 30 years) |
|---|---|---|---|
| supply links | exponent `link_power` | `link_power ×` reliance (upstream nearby over the industry's own size, at most 1) | machinery ended at 0.16 of its jobs, metals 0.27; supply blamed for 35% of closures |
| `small_decay` | 0.015 | 0.005 | manufacturing ended at 0.45 of its jobs (0.64 before industries) |
| `named_share` | 0.5 to 0.8 | 0.21 to 0.49 by industry | closures without recovery 77 (19 before) |
| `max_town_share` | 0.25 | 0.15 | a closure cost a town up to 12.9 points of unemployment a year on |
| world `shock_size` | 0.10 to 0.22 | 0.08 to 0.18 | closures 1,061 with the index behind 75% |
| textiles `trend`; metals `trend` | −2.5%; −1.5% | −1.8%; −1.2% | textiles ended at 0.27, metals 0.41 |
| `full_line` | (none) | 0.94 | 13,046 short-time events; 8,777 after |
| `pool_share` | (all of it gathers) | 0.5 | closures without recovery 56; 48 after |

## Evidence

Before: branch head 59c390a (with audit fixes 3, 5 and 8). After: this work merged onto it. `MANDATE_LAB_THREADS=2`.

| Check (command) | Before | After |
|---|---|---|
| Plant closures in 30 years (without recovery after ten) (`audit stuck --seeds 8`) | 836 (17) | 918 (54): +9.8% |
| Highest town unemployment | 15.4% | 17.9% |
| Towns drained; debt spirals; rent runaways; burn-in growth over 4× | 0; 0; 0; 0 | 0; 0; 0; 0 |
| Real rent growth, median town; rent burden at the end, median | 1.258; 0.128 | 1.248; 0.127 |
| Private building stalled, town-months | 1.0% | 1.7% |
| Crisis spans; lasting ten years | 3,520; 1 | 4,166; 3 (housing) |
| Executive party never changed in 30 years | 41.1% | 36.1% |
| National unemployment, median of all months; median at year 30 | | 4.4%; 4.5% |
| Total jobs, year 30 over year 0, median (min to max) | | 0.994 (0.942 to 1.048) |
| Sweep unemployment, inflation, population per decade, debt change (`sweep --seeds 8 --years 30`) | 4.49%, 2.29%, +0.52%, +5.4 pts: pass | 4.83%, 2.36%, +0.52%, +5.9 pts: pass |
| Sweep town crises begun, median | 36 | 50 |
| Incumbency: executive party kept; races within 10 points (`audit believe --seeds 8`) | 76.5%; 15.4% | 77.2%; 14.5% |
| Retrospective gap; recession spread by sector | 0.235; 3.78 pts | 0.244; 3.79 pts |
| Planted closure: races flipped of 378; worry spread by neighbourhood | 134; 31.7 pts | 109; 29.3 pts |
| `audit gate` | pass | pass |
| Bots, enforced gates (`bots --seeds 16`; `--seeds 32`) | all pass; all pass | all pass; all pass |
| Bots win rates at 32 seeds: idle, explore_only, canvass, mixed, explorer, legislator, proxy, corrupt | 6%, 6%, 22%, 81%, 84%, 78%, 50%, 81% | 12%, 12%, 47%, 91%, 91%, 91%, 72%, 91% |
| World generation with history, medium (`run --seed 2026 --size medium --turns 0`, mean of 3) | 1.53 s | 1.51 s |
| A turn, medium (400 turns less generation) | 4.32 ms | 4.43 ms: +2.6% |
| Save after 820 idle turns, medium | 2,107,745 bytes | 2,147,083 bytes: +1.9% |

Industry rows (new; `audit stuck`, block `industries`): jobs at year 30 over year 0 by industry: food_processing 0.85, metals 0.46, machinery 0.62, textiles 0.33, timber_paper 0.72, chemicals 0.78, vehicles 0.46, shipbuilding 0.52, mining 0.83, quarrying 1.03, oil_gas 0.99, forestry 0.94, fishing 0.91, arable 0.71, livestock 0.80. Closures: 918, all with plant, industry, factor and a pressure; by weakest factor: world 634, stock 99, supply 84, competitiveness 38, price 31, labour_cost 18, input 14. Review steps: 8,777 short time, 1,002 notices, 100 reprieves, 248 openings. Plants (running, short time, under notice, closed): 1,813, 869, 18, 320 at year 0; 1,494, 534, 2, 1,238 at year 30. A town's unemployment a year after a closure, against the nation: median +0.6 points, 95th percentile +4.1, most +8.1. Towns whose largest industry is still in their top two: 93%.

## Effect

Closures are explained every time (they were explained only in recessions), named, and within 10% of the count before. The economy stays sane: no town drained, no spiral, no runaway; `audit gate`, the sweep's bands and the bots' enforced gates pass.

**Awaiting the wave's tuning pass** (not tuned, by the owner's rule): closures without recovery after ten years are 5.9% (2.0% before; the design's band is 5%); crisis spans are up 18%; the bots win more often (the idle probe 12%, cap 15%); textiles ends at 0.33 of its jobs (the audit's band is 0.35); openings are 27% of closures (the design's band is 40%); the industry's own index is behind 69% of closures (band 60%); 29% of plants are on short time at the start. Every industry's `export_share` is 1.0 (the design's driver has no domestic term); the table of suggested shares is in `INDUSTRIES.md`, "The seam for foreign trade".
