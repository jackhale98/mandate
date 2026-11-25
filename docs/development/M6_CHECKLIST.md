# M6 checklist: the vertical slice

The working list for M6 (ROADMAP §2). M6 ends at the tester checkpoint: the owner plus 3–5 testers (REQ-004). Its design sources are:
- the ROADMAP M6 row;
- `docs/design/research/M6_ENGINE_CONTRACTS.md` (task ids B, K, G, C, R, U, X, F, V, A, P, L, D);
- `docs/design/research/ENGAGEMENT.md` (E1–E13);
- DEC-088 to DEC-100.

Status as of 2026-11-20.

**Order and method:** `IMPLEMENTATION_PLAN.md` (2026-11-23) sets the order of work, the shared mechanisms and the agent rules. Where it and the lists below differ on order, the plan wins once the owner confirms it.

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

   **Done (2026-11-23): measured, nothing changed.** The report is `docs/design/research/SIMULATION_AUDIT.md`; the runner is `crates/mandate-cli/src/lab/` (`mandate-cli lab run|twin`, `mandate-cli audit <topic>`). `audit gate` runs nightly and does not block yet. Verdict: stable, not yet effective. Fixes, in order (each tuned against the gate and logged in the balance log):
   1. **Record decides votes** (small): today re-election is 82% where unemployment fell and 80% where it rose.
   2. **Rents bounded, private building restarts** (medium): today 7.8% of towns run away and building is stalled in 43% of town-months.
   3. **Delivered results count in approval; the services worry reads service levels** (medium): today 12 of 15 laws cost approval.
   4. **One-party lock broken** (medium): today 38% of towns never change the executive's party in 30 years.
   5. **Honest ledgers** (small): the homes ledger overstates fourfold.
   6. Neighbourhood approval follows neighbourhood conditions (small).
   7. Dead mechanisms wired or removed: directional voting, the partisan filter, cohort trust (small).
   8. Texts corrected where the engine does not back them (small).

   Fixes 9–12 (migration, commuting, turnout, crime) are M7. After the engine play notes and the asks fix merge, re-run topics 1, 2, 5 and 7.

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
    - **Second bug (owner repro):** talked to the mayor; the next day talked to Cosmo; the ask said *broken*. Reproduce it: a step kept on a later day, or steps met out of the expected order, must never break the ask.
13. **The prose style bible** (owner), extending EXTENDING's "Writing for Mandate":
    - two or three tonal anchors, for example dry civic realism (Yes Minister), terse event text (Crusader Kings), character-driven political choice (Suzerain), spare and humane (Citizen Sleeper). Tone and craft only, never copied text;
    - owner-approved sample passages as the standard;
    - voice sheets for each speaker type;
    - a doctor lint for AI habits: stock phrases, "not X but Y", reflexive lists of three, too many em dashes, moralising closers, uniform sentence lengths, piled-up adjectives;
    - agents draft, the doctor lints, the owner blind-reviews samples.

    This comes before the cold-open and families content.
14. **A legal and compliance pass,** before the tester checkpoint. Groundwork only; a lawyer reviews before any commercial release.
    - A privacy notice: no data sent today; saves stay local; GitHub Pages logs; bug reports. Update it with DEC-104.
    - An AI-assistance disclosure: storefront rules (Steam), and the EU AI Act's transparency duties. A decision on the wording.
    - Open-source, font and art licence notices.
    - A fiction disclaimer.
    - A check on the name "Mandate".
15. **A security pass:**
    - a secret scan of the repo and its history;
    - `npm audit` and `cargo audit` in CI;
    - CI actions pinned, and token permissions narrowed;
    - a CSP in a meta tag (Pages has no headers);
    - save import hardened against hostile or huge files;
    - the service worker's scope checked;
    - no keys in the client bundle.
16. **Former officeholders are invisible** (owner question, 2026-11-23). A politician out of office now shows a trade, and nothing says they once held a post. The engine keeps their years in office, and it matters (parties re-run them 60% of the time over a recruit; seniority feeds who the power broker is; they keep their opinions, favours and memories), but the player cannot learn it. Not every out-of-office politician is a former officeholder: some ran and lost. Proposed:
    - the engine records the last post held and when it ended (a saved field; with the causal spine);
    - the person's sheet says "Former <post word>, N years" once the player knows them that well, and a talk line about them says it;
    - former officeholders can be asked about their time in office (how a past vote went, who owes whom), as a source of knowledge;
    - the trade shown is tied to a real workplace (the open caveat from note 1).
17. **Specific industries, not "manufacturing"** (owner, 2026-11-24). A plant is today a town's share of one generic sector, with no product, so a closure reads as "manufacturing demand fell". People care about the cannery or the steelworks. Build the cheap version, in the cold open wave beside the crisis rewrite:
    - **Industries as data.** The sector registry (DEC-075) gains specific industries under the broad sectors: for example steel and machinery, food processing, textiles, electronics, vehicles, timber and paper, chemicals; mining, quarrying, oil and gas; arable, livestock, fishing. Each has its own wage, cyclicality, productivity growth, and exposure to commodity prices, competitiveness and labour costs. The eight broad sectors stay as groups for totals, map layers and worries.
    - **Demand per industry.** Each industry has its own world demand index (a keyed random stream: drift and shocks), so a slump in one does not hit them all. Cheap local links without simulating goods: food processing reads local and regional farm output, a sawmill reads forestry, a smelter reads mining.
    - **Worldgen places them by geography:** ports get fishing, canneries and shipyards; forests get sawmills; ore gets smelters; a town has one or two defining industries.
    - **Named plants.** A town's larger employers are named workplaces with a product ("the Garrowby cannery"), as places with their people (PL1). A closure names the plant and what it made; the causal spine records the industry's weakest demand factor.
    - **Text and checks.** Crises, headlines, overheard lines and talk name the industry and the plant through arguments (the engine emits keys, never prose; nothing names a product in code). The audit gains a check per industry (closures explained, no industry dead or runaway). Lines per industry are a content cost: budget it with the crisis rewrite.
    - **Not this (later, after transport):** products as goods, firms, supply chains and trade between towns.
    - **Designed (2026-11-24):** `docs/design/INDUSTRIES.md`. Sixteen industries under the broad sectors; closures by short time, a dated notice, then a reprieve or the closure; plants also open. The lead's answers to its six questions stand unless the owner objects (fishing under resources; closures by notice and date; seeds change; old saves get plants at the works they have; no tourism or knowledge industries yet; names and status lines first, flavour with the crisis rewrite). Save version 27, after the spine's 26.
18. **Notebook and phone notes** (owner, 2026-11-24, with screenshots), one small web and protocol task, with the update prompt work:
    - "Last seen at , in the morning.": a place with no proper name (the union hall, the community centre) prints nothing. Use the place's in-sentence kind ("the union hall") as talk does; the same for "Usually at …" and the lead's `to_place`.
    - Remove the eye emoji from the seen line.
    - "(would tell you: Known)" reads badly, and "(would tell you: Heard of)" on the Leads tab worse (owner, twice). One key, `ui-notebook-lead-gives`, serves both. Say what following the lead does, in the card's own terms: "Would raise this to ●●○ Known"; where the result is only "Heard of", say "You would hear of this".
    - The date in the bottom bar is cut to "26 …" and "Turn …" on a narrow phone when three badges show.
    - **Which ward am I in?** (owner): the "where you are" chip names the neighbourhood only. Add the district in the constitution's words: "Eastfield · Ward 3". The wards layer on the town map (boundaries, numbers, yours marked, each member on their ward) goes with note 11's map work.
    - **The update prompt did not appear for the owner.** Add a browser test that serves one build, swaps in a second, and checks the bar appears and Reload switches builds; show the build identifier on the title screen and in "Report a problem"; keep the update bar above every panel.
19. **Talk: the greeting, and answers that make no sense** (owner, 2026-11-24):
    - **The greeting still comes after the first question.** Note 2's fix only reordered the first reply (opener, then the player's question, then the rest). The person should speak as the conversation opens, before any topic is chosen. The engine gives the greeting with the conversation's view (the same line it would have opened the first reply with), the dialogue shows it at once, and the first reply does not repeat it.
    - **The mayor, asked about part of their own town: "I couldn't tell you. I'm never over that way; you'd want someone from there."** The three `talk-worries-about-hood-unknown` lines fire for anyone not living in that neighbourhood. They must never fire for someone whose post covers it, who works there, or who is standing there as they speak. Officeholders know every neighbourhood of their jurisdiction at least by report, and get lines of their own ("I get the letters from there…"). Add a test over generated worlds: no speaker says they never go somewhere they hold office over, work in, live in or are standing in.
    - **The centre neighbourhood shares the town's name** (owner confirmed), so asking about it reads as asking about the town. Wherever a neighbourhood is named and it has its town's name, show it as the town centre ("Larkowick town centre": a label from the engine with the town as an argument; the stored name stays). And offer **the whole town** as its own choice in the worries question, answered from the town's worries, which any officeholder there knows.
    - **People pop into a room after a conversation** (owner): a talk takes time, routines move people, and newcomers appear at their spots. They should walk in: through the door in a room, from the street's edge outside; and those whose routine takes them away walk out the same way (note 7 did the street's leaving only).
    - While there: the talk view's choices read `player.municipality` (`mandate-protocol/src/talk.rs`); move to the player's focus (DEC-098).
20. **Filing for a post and silently ending up on no ballot** (owner, 2026-11-24: "I had selected to run for Mayor, but I wasn't placed on the ballot, and at Election Day I didn't get any screen or notification"). The engine (`politics/elections.rs`, where a campaign opens) nominates a player who filed for an office only if (a) their Standing meets the post's gate (`career.nomination`, 40 for the base pack's executive) and (b) their own party's sitting holder is not standing again (85% do, and the party then nominates the holder). Gate (b) is never shown. A player who filed for the office is not defending their seat, so when the nomination fails they are in no race, get no election night, and leave office without a word. Fix:
    - **Never off every ballot by accident:** when the nomination for the post filed for fails, the player defends the seat they hold (if their party would renominate them), and is told.
    - **Say so when the campaign opens:** an event and a notice in plain words ("Your party has nominated { $holder } again for { $post }. You are standing for your seat."), with the reason (the holder stands again; Standing below the gate).
    - **Show the rule before filing:** the filing screen names who holds the post, their party, whether they are expected to stand again, and the Standing gate with the player's Standing.
    - **Election day always has a screen:** the results where the player lives, whether or not they were on a ballot, and what became of their seat.
    - A test that walks: file for the office with the holder standing; with Standing below the gate; with the post open. Tier-generic: no office named in code.
    - Later (DEC-107): challenging your own party's holder for the nomination.
21. **The chamber: reaching your seat and voting** (owner, 2026-11-24): after the council changed, "our chair is the far end so with all the members there we need to go the long way around to vote, and with someone in the chair next to us it is difficult to pull up your vote instead of talk to them".
    - The player's desk is the one nearest the door along a clear aisle, whatever the body's make-up; the route to it never needs a detour round seated members.
    - A tap on your own seat is yours: your seat's spot wins over a neighbour's figure where they overlap.
    - The room offers "Take your seat" (and the vote, when one is open) as a button, so nothing depends on a precise tap; Places and quick mode keep their own route to the vote (DEC-051).
    - With it: the phone bar's dangling "·" when the turn line wraps, and the town summary's crisis line overlapping the defining-industry line (a browser test for it fails now and then in CI, so the overlap the owner reported is real and depends on timing).
22. **One ranking of worries** (found by the re-record command's trial run, 2026-11-24; fixed the same day). Two readings of "a town's top worry" disagreed in close cases: by level alone (the map's town card, a town's priorities) and as felt (talk, the notebook, canvassing). `concerns::ranked(scope, by)` is now the only ranking, with `Felt` and `Measured` as named variants. Two follow-ups:
    - The map's town card shows three worries of any town whatever the player knows, while the notebook shows one at "heard". The card should follow the knowledge ledger (DEC-091).
    - Promises and dilemmas ask for the measured ranking while the notebook shows the felt one, so a resident can ask for a promise on a worry outside the notebook's top three. What residents ask for should follow what they feel; decisions raised by an indicator may stay measured.


23. **The career card** (owner, 2026-11-24). When a run ends, the player gets a card to copy or share, so they can compare how they did with others against how hard their conditions were. Built with the opening draft, after the grader (STAGED_ARCS A4) supplies the ratings.
    - **On the card:** the character, party, town and kind of world; the opening with its measured rating and the facts behind it; every race entered with its rating at the time and the result; the highest post, years served, the notable decisions from the Legacy record and how each crisis ended; how the run ended; and one comparable figure, wins less the wins expected from the ratings.
    - **Sharing, with no server:** "Copy" gives a short text block; "Share" opens the device's share sheet with an image. The card carries a world code (seed, scenario, constitution, opening) so someone else can play the same opening; it also carries the build, and a code from another build says so.
    - **Stepping away:** the player may abandon a run at any time ("step away from politics"), after one confirmation. The run closes with a short epilogue, enters the Legacy record, and gives the same card marked as unfinished, with the year and post; a race under way is named as one walked away from.
    - Nothing personal is on the card: the character and the world only.

24. **"Why?" panels** (built 2026-11-25, on the causal spine). Follow-ups, small, for the next web batch:
    - A chain that starts from the storyteller's spotlight on an event shows that event's sentence twice.
    - The person behind an ask reads "Someone's own choice" while the ask card names them: being asked by someone should count as having heard of them (knowledge ledger), which the panel then follows.
    - On a wide screen the panel covers the left of the list it was opened from.
    - Decided bills are listed nowhere in the app, so the control on a bill's outcome never shows; skip summaries carry no event for elections, closures and disasters; the story sheet's recent lines and storyline stages have no "Why?".
    - An ask event older than the 60 ended asks kept has no arguments for its name.

25. **The label and the pocket named two places** (owner, 2026-11-25; fixed the same day). The label above the street reads the ground under the player while they walk; the pocket and the Places tab's "You are here" read the engine's location. Two causes. From the owner's bug report (a phone, seed 13263019044349966038): the people of a street stood anywhere within six cells of the player, across a neighbourhood's border too, so walking up to one put the player on the next neighbourhood's ground (Ward 6 underfoot, Ward 7 in the pocket). They now stand in the player's own neighbourhood while there is room (`townScene.ts`, with a unit test that fails without it). And walking by keys never told the engine: a player who comes to rest away from the engine's location, outside a conversation and not at a door, is now on that neighbourhood's streets, as after a tap on open ground (`TownStreet.tsx`; the keyboard-walk spec asserts the pocket agrees).

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
