# M6 S1 Daily routines (DEC-031 "Daily routines", DEC-051, DEC-091, DEC-098)

**Date:** 2026-11-20
**Status:**

**Done**, 2026-11-20: a routines registry in content (`content/base/routines.jsonc`, `RoutineDef` in `mandate-content/src/routines.rs`, validated: role words from the constitution, occupations, place kinds) places stored characters by day part (the player's slots; late night once they are spent), weekday of the turn's representative day (moves on one day a turn), season and events (session: the regular sitting day or a law-making stage this turn; vote; campaign; meeting night); derived, not stored (`mandate_sim::characters::routines`: `whereabouts`, `locate`, `present`); `stored_at`, talk, place panels, interiors' `here_as`, lead targets, "Go to them" (`talk_location`) and the bots all ask it; a room lists everyone there; passers-by met stay where met for the week; the ledger notes sightings (`KnowledgeLedger::seen`, additive, no save bump) and the notebook and character sheet show last seen and a `lead-usually-at` lead once seen at the same place and part twice; `mandate-cli routines --seed N --days D [--text]`
