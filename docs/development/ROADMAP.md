# Mandate — Roadmap (draft, D15)

**Status:** Proposed (DEC-062, DEC-063). The final version is produced with the full system plan (§30 Step 6).

## 1. MVP = the M6 vertical slice (DEC-062)

| Area | MVP scope |
|---|---|
| Career | Act 1 (local) complete: ward councillor → mayor, losses, a detour, Act 1 endings, plus a "next rung" teaser |
| Constitution | Federal Presidential Republic archetype, no quirks (DEC-016). The engine reads constitution data (DEC-017) |
| World | One generated medium country (DEC-021). One home-town hub plus 1 visitable town. National map as a strategic view only |
| Population | Full cohort model nationwide (DEC-018). ~20–30 Principal NPCs active locally |
| Economy | Layered model with ~8 sectors and municipal housing (DEC-019). National macro runs but cannot be influenced yet |
| Environment | Drifting regional state plus hazards (DEC-023) |
| Policy | ~12 local policy templates plus the municipal budget procedure (DEC-024/026) |
| Parties and elections | 2–3 parties with factions (DEC-027). Ward and mayoral plurality elections. Polls (DEC-028/029) |
| Characters and narrative | Memory, goals, favors (DEC-031/032). ~20 storyline and event templates. Director option B, Classic profile (DEC-033) |
| Media | 2–3 local outlets plus one social platform (DEC-034) |
| Player | Draft of 3 (DEC-036). Household, health, and finances (DEC-037). 1–2 detour roles (DEC-020) |
| Modes | Ironman and Story (DEC-008). Legacy epilogue for Act 1 endings |
| UI | Hub (top-down pixel art with placeholder art), desk, news, map room, phone, scenes, "Why?" (DEC-047–051) |

## 2. Milestones (DEC-063 option B, proposed)

| # | Milestone | Key deliverables | Gate |
|---|---|---|---|
| M0 | Foundation | Cargo workspace (8 crates), Vite/React PWA shell, worker + WASM + protocol round-trip, JSONC content pipeline and validator, determinism harness, CI fast tier | Builds, offline shell, native/WASM hash parity on a stub world |
| M1 | **Thin playable loop** | Hand-authored micro-town (1 ward, ~8 NPCs, 4 policies, 1 council, 1 election); slots and representative day; Mandate, Relationships, and the five pressure tracks (simple); director-lite; panel UI (no hub art) | **Owner playtest: is the core loop promising?** |
| M1s | Rendering spikes (parallel) | Hub spike and map spike (DEC-050) | Performance budgets on a real mid-range phone |
| M2 | World generation and lab | Terrain → settlements → regions/districts → hub layouts; burn-in; `mandate-cli` lab; canonical worlds | Reproducible worlds; burn-in within budget |
| M3 | Society and economy depth | Full cohorts, sectors, housing, public finance, environment drift; Monte Carlo health sweep | Stability bands and drama floors pass |
| M4 | Politics depth | Parties and factions, full election pipeline, polling, NPC AI with symmetry, bot players | Elections explainable; no dominant bot strategy |
| M5 | Characters and narrative depth | Memory, goals, storylines, director B, media layers, dialogue, scenes | Events grounded in state; pacing within budgets |
| M6 | **Vertical slice** | Hub rendering integrated; draft; detours; Act 1 endings; Legacy; saves and sync hooks; accessibility | **Game-quality gate: owner plus 3–5 testers** |
| M7 | Act 2 (regional) | Regional institutions, governor path, multi-hub travel, level-of-detail transitions | Gate review |
| M8 | Acts 3–4 (national, presidency) | National legislature, cabinet, macro levers, presidency, Mandate Test | Full career playable |
| M9 | Variety and polish | Archetypes 2–4 plus quirks, storyteller profiles, more content, art pass, performance, save migration | Release candidate |
