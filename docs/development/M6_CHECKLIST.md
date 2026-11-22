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

## Next when tokens refresh (owner, 2026-11-21)
0. **Order** (owner, 2026-11-21):
   1. the owner's play notes (items 1–11 below);
   2. **the simulation audit**;
   3. the cold open;
   4. the refactor (alongside or before the cold open);
   5. then F4/F5, F3/F6 and V1.

   **The simulation audit** is one agent task: lab experiments and a written report with charts (`docs/design/research/SIMULATION_AUDIT.md`), then fixes tuned against it and logged in BALANCE_LOG. It measures:
   1. **Policy responsiveness:** each policy's effect, sign and size across town types; that is, F7's gate.
   2. **Believable behaviour:** unemployment hurts incumbents; rents follow supply; migration follows jobs and rent; incumbency advantage; turnout patterns.
   3. **Sensitivity:** which parameters dominate outcomes.
   4. **Towns diverging:** whether towns stay distinct over 20–30 years.
   5. **Player leverage:** how much one councillor can move.
   6. **Stuck states:** debt spirals, towns that never recover, parties that never lose.
   7. **Readability:** whether "Why?" explains each change.

   The harness becomes a CI or nightly gate.
   **After the audit (DEC-106):**
   - the first batch of new worries;
   - the asset-condition model, with street cues (potholes, broken lights, overgrown parks as funding falls);
   - faith communities (fictional faiths, tendencies with spread within each faith, devotion levels; writing rules first).
   It also checks **every system against what its docs and UI claim**, so nothing is described that the engine does not do. Known gaps (owner question, 2026-11-21):
   - one labour market per town, with no commuting or job access between neighbourhoods or towns;
   - rent per town only (neighbourhood rent is a worry multiplier, not a market);
   - no moves between neighbourhoods except disaster displacement.
   **Later:**
   - a commuting layer (travel times between neighbourhoods and nearby towns, employment by neighbourhood from reachable jobs, rents by neighbourhood);
   - then transport (bus routes, rail, ferries as networks with service levels changed by families and works; vehicles drawn from the service level; DEC-095).
   This needs its own decision.
1. **Cold open (the hook), with the openings drawn from the simulation:**
   - Run the 20-year history as now.
   - Scan every town in the country for situations that match **opening frames**: authored templates in U1 conditions, such as a plant shedding jobs, a recent disaster, a predecessor's scandal, a tight by-election or a budget hole.
   - Offer a draft of three real towns with real situations. If nothing matches, simulate forward month by month, up to a cap.
   - Each frame supplies:
     - the player's post (from the constitution);
     - a clock;
     - an ally and an opponent cast from the real people;
     - a seeded hidden fact (a small E9);
     - a guaranteed first beat on turn 1 (the director).
   - The first chapter is about 15–20 minutes and ends in a payoff.
   - It replaces "the draft of three openings" and the "per-run twist".
   - **Crisis arcs, not votes:** warning signs → the break → escalation (people act each turn) → levers across systems → a branching resolution that leaves a visible mark. Rewrite today's crises to that template and add about a dozen new ones (the bridge, the water, the institution, the developer, the strike, the heatwave, the missing money, the newcomers, the sinkhole, the team, the recount, the buyer).
   - **The audition:** for each candidate situation, copy the game and run it 6–12 months with passive, engaged and contrarian stand-in players. Score escalation, agency (passive against engaged), visibility and pacing, then offer the best three. Throw the copies away; the same seed gives the same draft. The same tool runs in CI as the grader for crisis content (folds into E12).
2. **A structure-only refactor:**
   - Split `types.rs` and `validate.rs` by area, `actions.rs` into action modules, and `bots.rs`, `doctor.rs` and `director.rs` into folders.
   - Make PROGRESS and BALANCE_LOG one entry per file, or lead-only.
   - Drop the content hash from the v18 golden comparison.
   - Proof: identical hashes, bot output and corpus.
3. Then F4/F5, F3/F6 and V1 as before.

## Owner notes to fix (2026-11-21), first when tokens reset
1. **"Out of office" still shows over people's heads.** 0130b5d fixed only `person_ref` in `mandate-protocol/src/characters.rs`. Check the other paths:
   - `characters::knows::role_key` (mandate-sim), used by the gated person views;
   - `office.rs` (around lines 497 and 955);
   - the street name tags and ambient townsfolk in `web/src/hub`;
   - whether the PWA service worker served a stale build.

   Add a test that no non-player name tag reads `role-private`.
2. **Talk order.** A conversation's opener ("What's this about?") must come before the player's question, not after it, in the dialogue box.
3. **Map:** the "Country" toggle button's label is not centred.
4. **Town summary:**
   - the crisis text overlaps "Defining industry";
   - each neighbourhood should say which ward it is in.
5. **Canvassing in the street.** Canvassing (and other ward activities) should start by walking up to actual houses, rather than opening the ward menu: tap a door, canvass it. The pocket keeps the quick path (DEC-051).
6. **Arriving at a building.** Going to a place from the map (for example the town hall) opens its menu on arrival instead of entering the building. Places with an interior should be entered on arrival.
7. **Leaving a conversation.** Talking moves the time of day, so on leaving, the person has vanished because of their routine. Instead, they should walk off towards their next routine location so the change is visible. The same applies to anyone whose slot changes while on screen.

8. **The party line ignores the party's views.** `actions::party_line` returns yes if the bill's sponsor is in the player's party, otherwise no, so a socially liberal reformer party whips against affordable housing proposed by another party. Fix in the engine:
   - the line comes from the party's platform against the bill's attitude axes;
   - it is adjusted by deals, coalition and how strongly the party whips;
   - conscience issues get free votes;
   - the bill view says why ("Your party backs it: it fits their housing pledge").

   Check the NPCs' own votes use the same rule, then re-run the bots.
9. **The ward view is unreadable.** "Your ward's view: +20" is net support on −100..+100, rounded by what the player knows (to 25 below known, to 10 at known). Show words and shares instead, for example "Leans for: about 60% for, 40% against (rough estimate)", with how sure the estimate is.
10. **Vote on impact, not just approval** (owner). Before voting, the bill view should show:
    - what the bill does: cost, who pays, and who gains or loses by neighbourhood and group;
    - side effects and trade-offs (DEC-073);
    - what it does to the player's record and credit (V1);
    - with knowledge gating, as for everything else.

    This is F3's extended policy preview (contracts §2.12) brought forward to ordinary bills, plus "Why?" afterwards.

11. **Show dense information on the map** (owner). People, councillors and other dense lists should have a map view on the town map overview (DEC-086), so players link people to places and remember them:
    - councillors' portraits in their wards;
    - key figures at their workplaces or where last seen (S1);
    - tap a portrait for their card, plus "Go to them";
    - filters: council, party, people met.

    Use the same pattern for:
    - worries by neighbourhood (a shaded layer);
    - a bill's impact by neighbourhood (item 10);
    - asks pinned where they happen;
    - works.

    The list views stay for accessibility and quick mode (DEC-051). Only what the player knows is placed (DEC-091).

12. **Asks that need a conversation have no conversation for it** (owner report, "An introduction"). `ask.introduce_me` needs "talk to the official, then the asker". There is no "Put in a word for <asker>" choice and no lines for it, so the official says nothing when asked about the asker. Also, `Deed::Talk` is only recorded if `talk::apply` succeeds, and ask progress only advances when `asks.daily` runs, so "Next:" doesn't move until the day ticks over. Fix:
    - an ask-aware talk option (the ask's need gives the dialogue a choice with its own lines and reply, for example the official agreeing to meet);
    - resolve the step at once when the command lands (or show "done, confirmed tomorrow");
    - make a failed or empty exchange visible, never silent;
    - audit every ask need for a player-visible way to meet it (a test: each need kind has a command path and a UI path).

## M6 or M7 split (owner, 2026-11-21)
- **M6 (needed for the tester checkpoint):**
  - the play notes;
  - the simulation audit;
  - the cold open (crisis arcs, new crises, the audition, frames);
  - the refactor;
  - families F3–F6 and V1;
  - DEC-102 targets and gates;
  - the first batch of new worries, with upkeep you can see;
  - the planned E3–E5 and E9–E13 content;
  - text pass 2.
- **Protect first if tokens are tight:**
  1. the play notes;
  2. the audit;
  3. the cold open;
  4. the families pack and screens;
  5. the first worries.

  V1, the difficulty modes and E9–E13 may slip to M7.
- **M7:**
  - faith communities;
  - the crime model with the justice system and prisons;
  - the commuting layer, then transport;
  - energy;
  - immigration and emigration (with DEC-087 neighbours);
  - return to a world (DEC-103);
  - the Act 2 design (DEC-105).
- **When the custom domain exists:** account sync (DEC-104).

## Later (owner, 2026-11-20)
- **Account sync between devices:** passkey sign-in, saves synced through a Cloudflare Worker, D1 for metadata and R2 for save blobs (the free tier fits tester scale).
  - Waits for a custom domain: the passkey's domain must not change after launch.
  - Needs a new DEC first: it changes "offline-first, no backend" to "offline-first, with optional sync".
  - Plan:
    1. The Worker, the D1 schema, R2, passkeys and a recovery code.
    2. Client sync: per-slot revisions, a conflict prompt, and "delete my account".
