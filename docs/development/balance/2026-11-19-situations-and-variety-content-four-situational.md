# Situations and variety (M5d part 2): Content: four situational policies (NPCs draft them only while their t

**Date:** 2026-11-19
**Area:** Situations and variety (M5d part 2)

## Parameter and change
Content: four situational policies (NPCs draft them only while their town's situation holds; each is listed among its crisis's answers) and six bill and favour dilemmas (the whip's deal defaults to "counted in with the party", not a vote with the ward, and the amendment's default costs the colleague's goodwill without a Standing gain: inaction must not please voters). Tuning for the new variety gate (DEC-077 §5): `politics.campaign.ad_persuasion_share` 0.25 → 0.05 (an ad still buys reach, but persuasion is the doorstep's); `politics.vote.record_penalty` 4.0 → 5.0 (a thin record costs more, holding idle play down once opponents' ads persuade less). Grid evidence (32 seeds): ad share 0.25/0.15/0.1/0.08/0.05 × record penalty 4/5/6/8; 0.05 × 5 is the smallest change passing every gate

## Evidence
Before tuning (content only), bots 32: mixed 81%, canvass 78%, corrupt 78%, proxy 72%, legislator 69%, media 25%, idle 12.5%; variety fails: mixed is among the best (within 0.2 share points) in 30 of 32 worlds. After, bots 16: canvass 75, corrupt 75, legislator 75, mixed 75, proxy 75, media 6.25, idle 6.25%; best − median 0; sole best: canvass 7, mixed 4; top among-best share 56%. Bots 32: canvass 78.1, mixed 78.1, proxy 78.1, corrupt 75, legislator 75, media 12.5, idle 9.4%; best − median 0; sole best: canvass 13, mixed 7, legislator 2; top 53%. Bots 64: sole best canvass 27, mixed 15, legislator 2; top 55%. By starting situation (32): mixed best on average in towns without a crisis (29), canvass in towns losing jobs (3). CI sweep (8 × 20): bands and floors pass; median unemployment 5.14% → 4.65% (NPC closure packages and wage subsidies), inflation 2.14% → 2.25%; decisions per chapter 3.2 → 3.15, 0 chapters over budget. Determinism (2026 × 80) identical twice and through save/load

## Effect
Gates pass: no dominant strategy, effort matters, idle ≤ 15% (6–9%), corrupt not dominant, and the variety gate (≥ 2 strategies sole best somewhere; none among the best in > 60% of worlds). The ground game and the all-round routine now trade places by world; ads alone (media) are weaker than before (12.5% at 32 seeds)
