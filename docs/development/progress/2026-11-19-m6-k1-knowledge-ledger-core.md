# M6 K1 Knowledge ledger core (DEC-091)

**Date:** 2026-11-19
**Status:**

**Done**, 2026-11-19: `PlayerState.knowledge` (`mandate_sim::player::knowledge`: levels, subjects, sources, readings, leads, zones; `learn`/`read`/`lead`/`enter_zone`, `knows`, `Knowing` for bots, `Effect::Learn/Read/Lead`), `knowledge.jsonc` (start profiles, local option, primer, introductions, effect numbers) with validation, seeded at play start (`start.sitting_councillor`, today's only start) and on load (older runs `Migrated`, generously), save format v15 (after B1's v14), `bots --omniscient`; nothing reads the ledger for outcomes yet (K2–K5)
