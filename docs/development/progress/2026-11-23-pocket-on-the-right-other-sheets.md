# Pocket on the right, other sheets on the left (desktop)

**Date:** 2026-11-23
**Status:** done

Web only, no engine change. On screens 900px and wider the pocket (and its docked form) now sits on the right, by the Pocket button, and slides in from and out to the right. Every other sheet (place panel, character, decision and story sheets) sits on the left and slides from the left. Borders, shadows, animations, the game-body paddings and the docked margins are mirrored in `web/src/styles.css`; `main-screen.spec.ts` asserts the street ends left of the docked pocket. Phones are unchanged; swipe-to-close is vertical and phone-only, so no gesture change.
