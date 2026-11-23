# M6 K2 Protocol redaction (DEC-091 §1.7–1.8)

**Date:** 2026-11-19
**Status:**

**Done**, 2026-11-19: every §1.7 view field goes through `mandate_protocol::knowledge::Gate` (`knows`), with its fallback per level and a `KnowledgeLevelDto`/`KnowledgeView` (level, reading date, lead) beside it — concerns (pocket, town card, office, map hoods, the desk), the broker (rumoured when heard; stake in detail), the rival's plan (new), names, stances, pragmatism, goals and favours of people, policy support (band, classes, ward), situational policies (omitted, stub, full); place precision and knowledge compose (the stricter wins); decisions, stories, own state, odds, the standard menu, elections and the rival's identity stay ungated; `QueryDto::Notebook` (`NotebookView`), `PocketView.notebook_new`, `NewGameParams.local_knowledge` → `Engine::set_knowledge_start(None, true)`; `knowledge.ftl`; the web renders the placeholders (notebook page is K5); state hashes unchanged (views never write)
