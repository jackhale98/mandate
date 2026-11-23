# M6 Landing page refresh (DEC-083 follow-up)

**Date:** 2026-11-20
**Status:**

**Done (visual)**, 2026-11-20: the landing page shows the game as it is now, captured from the real build: the walkable street with the slim bar (phone and desktop, the pocket docked), a conversation in the street (the two framed, a speech bubble, the dialogue box), the council chamber with its members, the notebook, the country with the unemployment layer beside three generated countries, and a fresh election night (the results box updated to it). `npm run capture` (`web/landing/scripts/capture.mjs`) builds, serves, captures with fixed seeds, viewports and themes and the page clock paused (deterministic), then encodes (AVIF, WebP fallback) and renders `og.png`. Copy kept; only what had become false changed. Wording to revisit when M6 is done
