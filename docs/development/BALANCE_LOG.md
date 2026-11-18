# Balance Log

Each tuning change records the parameter, old → new, the evidence (sweep or bot result), and the observed effect (TEST_STRATEGY §5).

| Date | Area | Change | Evidence | Effect |
|---|---|---|---|---|
| 2026-11-18 | Economy | Retail and public demand measured in real terms, normalised by average productivity | `trace-econ` seed 7: nominal demand caused a wage–price spiral (u 1.5%, π 10%) | Removed the inflationary feedback |
| 2026-11-18 | Economy | Phillips curve: monthly smoothing 0.9/0.1 toward `target + 0.45·(u* − u) + energy` | Deflation stuck at the −3% floor | Inflation about 2% in normal times |
| 2026-11-18 | Housing | Build cost anchored to the initial local price; baseline starts = demolition (0.5%) + household growth (0.4%) | Construction at 25% of capacity | Construction cycles around 1.0× capacity |
| 2026-11-18 | Economy | Consumption interest channel; unit-labour-cost competitiveness for traded sectors | Slow overheating drift | Labour market self-corrects |
| 2026-11-18 | Economy | Technology raises knowledge-sector productivity, not headcount; frictional floor (employment ≤ 97.5% of the labour force) | Knowledge jobs at 1.3× capacity | Sector mix stable |
| 2026-11-18 | Finance | Budgets indexed to price × productivity × population; grants indexed; balanced-budget baseline for local and regional tiers; national gap closed half by taxes, half by trims (trims ≤ 5%/yr); debt brake above 60% | Debt +280% of output, then austerity spirals (u 24%) | 16–24-seed sweeps pass: debt stable (end 32–66%) |
| 2026-11-18 | Economy | Structural adjustment: idle labour attracts private capacity (0.25·slack per year); IndustrialDecline trend −2.5% → −2.0%/yr | IndustrialDecline preset: u 25.6% | All presets pass; decline preset keeps higher u (8.4%) |
| 2026-11-18 | Environment | Droughts are regional (one roll per region in July, p = 0.06·mean drought / water) | Six droughts in one month | About one regional drought every few years |
| 2026-11-18 | Parties | Majors split at the weighted median of the first principal axis; minors from the tail of the next axes (was k-means) | k-means: one party took 65% of seats; then a centrist "minor" took 40% | Two competitive majors; minors win a few seats |
| 2026-11-18 | Elections | Strategic (wasted-vote) penalty `1.6·(1 − viability)²`; minor parties stand where attachment ≥ 16% | Minor party won 25–40% of seats under plurality | Duverger-like outcomes |
| 2026-11-18 | Elections | `party_weight` 2.2 → 1.4; race shock 0.18 → 0.25; national swing 0.12 → 0.2 | Re-election 87%, mean margin 30 pts | Re-election 76–83% (target 65–80%) |
| 2026-11-18 | NPC AI | Propose weight 0.8 → 0.35, agenda curve x₀ 0.65 → 0.75; NPC laws carry 4–8-year sunsets; busy statute book cools proposals (`−active/5`) | Every template enacted in every town by the player's start (nothing left to propose) | About 4 programmes in effect per town at start |
| 2026-11-18 | Campaign | Canvass approval +0.04 → +0.015 (saturating); town-hall nudge halved; ad reach 0.06 → 0.12 | Bots: canvass-only won 88% (+18 pts vs idle), ads +0 | Canvass 88% (+12), legislate 81% (+11), mixed 62%, proxy 69%, media 56%, idle 50% (16 seeds): no dominant strategy |
| 2026-11-18 | Finance | Each budget year adopts every enacted programme, pro-rated if programmes exceed 25% of the town budget | A law appropriated at 0% stayed unfunded forever | Laws wait at most until the next budget year |
| 2026-11-18 | Trade-offs | Ads: no slot, money only, ≤ 3 a turn, own saturation (0.7ⁿ), no turnout; canvass −3 health; fundraise +0.5 Standing, −2 health, low-income approval −0.01; press gaffe risk 0.05 + 0.3·(1 − oratory); town-hall hostile risk from local approval; health < 35 / < 15 costs a slot | DEC-073 (owner direction); CI bots (8 seeds) showed canvass 100% | 16 seeds: mixed 81%, canvass 69%, legislate 69%, proxy 62%, media 50%, idle 38%; best − median 0.125 |
| 2026-11-18 | Votes | Party discipline on declared votes (−6 Standing for defying, +1 for loyalty); riders: sunset +0.08, pilot +0.12 lean; each other ward's earmark −0.04 lean | DEC-073 | Votes are a real decision; riders trade cost for passage |
