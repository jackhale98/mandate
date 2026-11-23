# Neighbourhood concerns (M6 N1): Each neighbourhood's concern levels from its own conditions (`opinion.

**Date:** 2026-11-19
**Area:** Neighbourhood concerns (M6 N1)

## Parameter and change
Each neighbourhood's concern levels from its own conditions (`opinion.local_concerns`: rent by kind core 1.15, inner 1.1, waterfront 1.3, industrial 0.8, suburb 1.0, rural 0.75, `rent_growth` 2.0; pollution by kind core 1.0, inner 0.8, waterfront 0.9, industrial 1.6, suburb 0.5, rural 0.4, + 1.0 per polluting workplace, + 0.35 per one next door, + 0.8 × flood exposure; `workplace_affinity` 1.5; sector leans by kind; `job_loss_decline` 4.0; `jobs_sector_tilt` 1.0), each scaled to a residents' mean of 1. Felt concerns (canvass match, notebook, talk, overheard) read them; the vote's salience and NPC agendas keep the town-wide (measured) levels

## Evidence
Before → after, bots 16: every win rate identical (canvass 87.5, explorer 87.5, corrupt 81.25, legislator 81.25, mixed 81.25, proxy 81.25, media 6.25, idle 6.25, explore_only 6.25%); mean share canvass 0.6296 → 0.6305, explorer 0.6296 → 0.6300, mixed 0.6181 → 0.6185, corrupt 0.6085 → 0.6089, proxy 0.5637 → 0.5638, legislator, media, idle, explore_only identical to 4 places; explorer − mixed 1.157 → 1.144 points (gate 1); sole best canvass 8, explorer 6 (unchanged). `run --seed 777 --size large --turns 120` overview identical apart from the state hash; determinism 2026 × 80 identical twice and through save/load

## Effect
No town-level change (the political machinery reads the same numbers); the canvass match now rewards the district's own worry, a small gain for the bots that aim it. The explorer margin over mixed is still thin (1.14 points)
