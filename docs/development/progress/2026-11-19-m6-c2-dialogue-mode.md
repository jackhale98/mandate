# M6 C2 Dialogue mode (DEC-093, DEC-051, DEC-080; UI.md §3)

**Date:** 2026-11-19
**Status:**

**Done**, 2026-11-19: the talk panel redesigned as RPG dialogue mode, web only (no engine change): a pixel-framed dialogue box (one-tap topic choices with the engine's odds, a subject step in the same box, cost, out of time, someone elsewhere, Goodbye) and pixel speech bubbles (paged replies, italic stage directions, typewriter); in the street the camera frames the two speakers, the rest dims, walking, input and townsfolk pause (`HubView.beginTalk`/`endTalk`, `TownScene.folkPaused`; a talk stage bridge, `components/talk/stage.ts`); elsewhere a pixel portrait from the street sprite's look (`components/portrait/`); notebook toast and lead choices; keyboard and live region; `e2e/talk.spec.ts` (keyboard, Escape, bubble text, out of time)
