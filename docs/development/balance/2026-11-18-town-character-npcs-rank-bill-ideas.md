# Town character (AI): NPCs rank bill ideas that answer the town's live crisis (`npc.crisis_a

**Date:** 2026-11-18
**Area:** Town character (AI)

## Parameter and change
NPCs rank bill ideas that answer the town's live crisis (`npc.crisis_agenda` 0.4 × severity) and its concerns (`npc.concern_agenda` 0.3 × fit) higher; the bonus changes which idea comes first, not how keen the NPC is (a first version that raised keenness too added 12% more bills and cost +17% generation time)

## Evidence
Bots 16: mixed 81 → 81%, canvass 69 → 75%, legislator 75 → 62.5%, proxy 75 → 62.5%, media 19 → 12.5%, idle 6.25 → 6.25%; best − median 0.06 → 0.19. Bots 32: mixed 81 → 78%, canvass 75 → 75%, legislator 78 → 66%, proxy 75 → 62.5%, media 19 → 12.5%, idle 12.5 → 9.4%; best − median 0.06 → 0.125. CI sweep, six scenario sweeps (24 × 30) and a 24-seed medium sweep pass; median unemployment within ±0.4 points of before in every scenario

## Effect
Gates pass (no dominant strategy, effort matters, idle ≤ 15%). Lobbying is harder against principled colleagues, so the legislator and proxy strategies win less. Every town in every sweep has a defining industry, a broker, and a rival; medium worlds see about 111 crises begin per 30 years (about 2 per town). Generation 0.86 s (+8–10% vs before)
