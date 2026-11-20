# M6 checklist: the vertical slice

The working list for M6 (ROADMAP §2). M6 ends at the tester checkpoint: the owner plus 3–5 testers (REQ-004). Its design sources are:
- the ROADMAP M6 row;
- `docs/design/research/M6_ENGINE_CONTRACTS.md` (task ids B, K, G, C, R, U, X, F, V, A, P, L, D);
- `docs/design/research/ENGAGEMENT.md` (E1–E13);
- DEC-088 to DEC-100.

Status as of 2026-11-20.

## Priority while tokens are tight (owner, 2026-11-20)

Run 2–3 agents at a time, in this order:
1. **U1:** one condition language. It unblocks E3, E6, F1 and E13.
2. **E6–E8:** asks and promises, the goals layer.
3. **Time skips, then Act 1 endings and Legacy:** Act 1 can be finished, and its length target met.
4. **F1–F3:** policy families, engine and views.
5. **R2,** then **V1**.

On hold until then:
- the content-pipeline pilot (E3, which needs U1 anyway);
- the four-room art round;
- tooling extras;
- the draft of three openings;
- the tutorial.

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

- **Done 2026-11-20 (live):**
  - N2: voters weigh their own district's worries.
  - PL1: places with purpose.
  - Talk markers, and People merged into the Notebook.
  - Text read-through pass 1, with the relevance fixes.
  - E2: the director's variety and pacing.
  - U1: one condition language.
  - Time skips with directives, interrupts and the summary.
  - Act 1 endings and the Legacy record.
  - E6–E8: asks and promises (engine, content, bots).
  - F1, F2a and F2b: policy families in the simulation, with works projects.
- **Done on the branch:** E7, the asks screens (the Today block, casework, the promise picker, the badge), with the skip-summary fixes.

## Running
- **R2:** offices from the constitution in the protocol and web.

## Next (queued)
- **F3–F6:** family views, the first pack, the lab and the proposal sheet, after R2.
- **V1:** credit and blame for laws.
- **X1:** a lab timing check of layout v2 on a regional capital.
- **C5:** a talk lab and repetition gate.
- **Proxy and asks:** the skip's proxy answers offers by its directives (offers now lapse during skips).
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

## Later (owner, 2026-11-20)
- **Account sync between devices:** passkey sign-in, saves synced through a Cloudflare Worker, D1 for metadata and R2 for save blobs (the free tier fits tester scale).
  - Waits for a custom domain: the passkey's domain must not change after launch.
  - Needs a new DEC first: it changes "offline-first, no backend" to "offline-first, with optional sync".
  - Plan:
    1. The Worker, the D1 schema, R2, passkeys and a recovery code.
    2. Client sync: per-slot revisions, a conflict prompt, and "delete my account".
