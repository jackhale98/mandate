# Director and dilemmas (M5c): The narrative director (DEC-033 B, Classic) spotlights 12 base dilemma

**Date:** 2026-11-19
**Area:** Director and dilemmas (M5c)

## Parameter and change
The narrative director (DEC-033 B, Classic) spotlights 12 base dilemmas, 3 storyline turns, and 3 requests (`dilemmas.jsonc`); bots answer by a per-strategy taste (idle never answers: defaults, which cost someone, happen at deadlines). Effect sizes: opinion ±3–16, approval ±0.5–3 points (room-scaled like canvassing), standing ±1–3, money ±800–3000, slots 1. Defaults give no Standing (a first version's "+1 Standing for staying quiet" offset the thin-record drain, so idle careers were never deselected). Goals: oppose/back lean 0.35/0.2, champion 0.3. Budgets per chapter: 1 climax, 2–4 significant, 5–10 minor (`director.jsonc`)

## Evidence
Bots 16: canvass 75 → 75%, idle 6.25 → 6.25%, legislator 62.5 → 68.8%, media 12.5 → 12.5%, mixed 81.2 → 81.2%, proxy 62.5 → 62.5%; best − median 0.19 → 0.125. Bots 32: canvass 75 → 71.9%, idle 9.4 → 9.4%, legislator 65.6 → 68.8%, media 12.5 → 18.8%, mixed 78.1 → 78.1%, proxy 62.5 → 62.5%; best − median 0.125 → 0.094. CI sweep (8 × 20) bands and floors identical to before (idle player; median unemployment 5.14%, inflation 2.14%); all six 24-seed × 30-year scenario sweeps pass

## Effect
Gates pass (no dominant strategy, effort matters, idle ≤ 15% at 16 and 32 seeds). Story density, scenario sweeps (idle player, no NPC turns): 30 chapters, 2.6–2.8 decisions per chapter, 1.4–1.6 minor per chapter, a climax in every chapter, 0 chapters over budget. Bot play (12 seeds × 6 years): sessions mostly 1–4 significant decisions and one climax (59–72 of 61–72); campaigns 2–4 decisions; minor beats 8–10 per session for the mixed strategy, 0–5 for idle and canvass (fewer bills, so less news). Medium generation + 20-year burn-in 0.81 s → 0.83 s median (+2%); 60 medium turns 1.05 s → 1.02 s
