# Play note 21: chamber seat and vote, bar separator, town card spec

**Date:** 2026-11-24
**Status:** done (web only; no engine, protocol or content change except two button labels).

- Chamber: the player's desk is the desk nearest the door by walking distance over clear floor, chosen among the recipe's player spot and the member desks like it (`web/src/interior/seat.ts`). No recipe variant needed a change; the recipe's own spot is the fallback if no desk is on an aisle. The vote hotspot follows the desk. A tap on your own desk beats a neighbour's figure box (`tap.ts`); at your desk the vote comes before the person beside you. "Take your seat" and "Go to the vote" are buttons in the room's bar.
- Phone bar: the "·" belongs to the dots' no-wrap group.
- Town card: the layout cannot overlap; the failing check read two boxes while the pocket sheet slid in. Now read in one evaluation after the layout settles, with a wide-font crisis case at 320, 360 and 390 px.
