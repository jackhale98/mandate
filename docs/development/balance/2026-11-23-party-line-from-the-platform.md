# Party line from the platform

**Date:** 2026-11-23
**Area:** politics (party line, NPC votes)

## Parameter and change

The party line was "yes if the sponsor is in your party, else no". It is now one rule for the player's whip and NPC members: score = `platform` × fit(bill signature, party platform) + a sponsor term (`own_sponsor` 0.5, `governing_sponsor` 0.2, `other_sponsor` -0.2); inside ±`free_band` (0.25) the vote is free. NPC members are pulled by `npc_pull` × score × their loyalty factor, nothing in a free vote. New block `politics.party_line` in `politics.jsonc`. `council.same_party` and `council.rival_party` are removed (replaced by the rule).

## Evidence

`bots --seeds 16`, mean vote share (win rate):

| strategy | before | npc_pull 0.3 | npc_pull 0.12 | npc_pull 0.6 (kept) |
|---|---|---|---|---|
| mixed | 0.6145 (0.81) | 0.6148 (0.81) | 0.6057 (0.75) | 0.6069 (0.81) |
| explorer | 0.6293 (0.81) | 0.6237 (0.81) | 0.6174 (0.81) | 0.6232 (0.81) |
| canvass | 0.5712 (0.75) | 0.5655 (0.69) | 0.5558 (0.69) | 0.5596 (0.69) |
| idle | 0.2803 (0.06) | 0.2743 (0.06) | 0.2712 (0.06) | 0.2713 (0.06) |

At 0.3 `ladder_learned` failed (explorer − mixed = +0.0088 < 0.01).

## Effect

At 0.6 every enforced gate passes (explorer − mixed = +0.0163; before +0.0148). Votes follow platforms: a rival's bill that fits a party's platform is no longer opposed on sponsorship alone.
