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
