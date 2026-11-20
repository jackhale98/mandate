# M6 checklist: the vertical slice

The working list for M6 (ROADMAP §2). M6 ends at the tester checkpoint: the owner plus 3–5 testers (REQ-004). Its design sources are:
- the ROADMAP M6 row;
- `docs/design/research/M6_ENGINE_CONTRACTS.md` (task ids B, K, G, C, R, U, X, F, V, A, P, L, D);
- `docs/design/research/ENGAGEMENT.md` (E1–E13);
- DEC-088 to DEC-100.

Status as of 2026-11-20.

## Done
- **Town:**
  - Layout v2 with the street rework, diagonals and street ends (DEC-090).
  - The walkable town as the main screen, with the tabbed pocket, keyboard walking, battery fixes, the Town | Country toggle and the night passage (W1–W3, DEC-088).
  - The town map overview (DEC-086).
- **Baselines, town views and hazards:** B1, B2, B3 and G1.
- **Knowledge:** the ledger and notebook (K1–K6, DEC-091), including the local-knowledge option.
- **Conversations:**
  - the engine, lines and protocol (C1–C3);
  - the talk panel, now the **dialogue mode** with speech bubbles and portraits (C4, DEC-093).
- **Per-neighbourhood concerns:** N1.
- **Offices from the constitution:** R1 (DEC-098).
- **Ground-game rebalance, informed bot styles and the gate ladder:** E1 (DEC-097).
- **Daily routines and last-seen:** S1 (DEC-031).
- **Enterable places:** the council chamber, town hall and diner (DEC-088).
- **Country map open layers:** DEC-089.
- **Art:** interiors (round 7) and the works kit with stall and decay (round 8, DEC-099).
- **Tooling:** bug report and replay; the content workbench (doctor, designer page, corpus).
- **Designs:**
  - ENGAGEMENT plus its research;
  - the extensibility audit;
  - DEC-097 to DEC-100.

## Running
- **E2:** the director's variety, chapter themes and pacing cycle.
- **Text read-through, pass 1:** the style guide, fixes and doctor rules.

## Next (queued)
- **N2:** voters weigh their own district's worries (DEC-092 note).
- **PL1:** places with purpose. Verbs at the factory, mill, mine, port, farm, school, clinic, university, park, hotel, tech campus and business association, plus named people at each (DEC-079 note).
- **Talk markers:** 💬 and "!" in the street and rooms, and the pocket's People tab merged into the Notebook (four tabs).
- **U1:** one condition language (DEC-098 §5).
- **R2:** offices from the constitution in the protocol and web.
- **X1:** a lab timing check of layout v2 on a regional capital.
- **C5:** a talk lab and repetition gate.
- **More rooms:** the union hall, party office, news office and community centre. Awaiting the owner's go.

## Engagement (DEC-097, DEC-100)
- **E3:** the dilemma pool. Written to recombine with the town's details; content targets below.
- **E4:** storylines, with NPC schemes running in parallel.
- **E5:** the press covers the player's work.
- **E6–E8:** asks and promises (engine, Today tab, content and bots).
- **E9:** secrets, plus the fact-kinds registry.
- **E10–E11:** walking rewards: curiosities, Easter eggs, encounters and plaques.
- **E12:** the engagement gate, including the cross-career repetition gate (DEC-100).
- **E13:** conversation memory: priority, once-only lines and callbacks.
- **Act 1 content targets (DEC-100):**
  - about 90–110 dilemma templates;
  - 12–15 storylines;
  - 40+ asks;
  - about 800 conversation lines.
- **A per-run twist:** an opening-condition registry (DEC-100).

## Policy families (DEC-092)
- **F1:** types and validation.
- **F2a:** the lookup seam.
- **F2b:** families in the simulation, including **works projects** (DEC-099).
- **F3:** protocol.
- **F4:** the first pack of 15–20 families.
- **F5:** lab and coverage.
- **F6:** the proposal sheet (web).
- **F7:** need-aware reactions.
- **Works you can see (after F3):** the dress pass, the street, Today items and "See it".

## Politics depth
- **V1:** credit and blame for laws.
- **V2:** endorsements.
- **A1:** no decorative attributes (DEC-095).
- **P1:** the power broker acts (DEC-096).
- **L1:** the odds calibration gate.

## Town and places
- **G2:** hidden places on the knowledge ledger.
- **Layout polish:** two-cell diagonal pavements, per-district street fields, and the map bake moved into the worker.
- **Rooms for PL1 places**, such as the factory, school and clinic, in later art rounds.
- **Character appearance as a registry** (pixel-art P1, DEC-098 §4).

## Game structure
- **The draft of three openings.** Varied situations; each start names a constitution post (DEC-036, DEC-077).
- **Representative-day UX.**
- **Daily life:** household, health and finances.
- **Detours (1–2):** organisation posts (DEC-020 note).
- **Time skips** with directives and interrupts, to meet the Act 1 length target of 1.5–2.5 h (REQ-008).
- **Act 1 endings and the Legacy record,** as a content registry. Legacy unlocks are deferred (DEC-100).
- **Ironman and Story modes.**
- **Tutorial:** the first chapter as a guided chapter, from a web step list.
- **Settings:** quick mode, battery saver, local knowledge, reduced motion.
- **Difficulty presets as data;** the DEC-076 difficulty and story targets become gates.

## Quality and tooling
- **Text read-through, pass 2,** after the story content lands.
- **The accessibility layer, plus a real screen-reader playtest** (VoiceOver/TalkBack) at the tester checkpoint.
- **PWA polish.**
- **Performance:** a budget re-baseline and the React Compiler evaluation.
- **Saves:** migrations for every format change (DEC-074), and a corpus of old saves loaded in CI.
- **Tooling:**
  - the constitution sandbox;
  - performance and size tracking in CI;
  - the town atlas;
  - an offline playtest summary for testers.

## Release
- **D1 docs:**
  - OVERVIEW and DATA_MODEL;
  - EXTENDING (adding an office, a routine, a condition; writing for Mandate);
  - TEST_STRATEGY.
- **Refresh the landing page** with the walkable town (DEC-083).
- **The tester checkpoint:** the owner plus 3–5 testers, with the §29.3 questions. M7 starts only on approval.
