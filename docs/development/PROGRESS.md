# Progress

**Phase:** Implementation. The plan is approved (DEC-067).

## Milestone status
| Milestone | Status |
|---|---|
| M0 Foundation | **Done**, 2026-11-18 |
| M1 World and lab | **Done**, 2026-11-18 |
| M1s Rendering spikes | Map: Canvas 2D in production (fast, pixel-art). Hub: Canvas 2D (DEC-080); its M6 movement module (`web/src/hub`: routing, tap targets, camera) is built, tested, and used by the spike. M6 groundwork: a deterministic town layout from the engine's places, a chunked Canvas 2D renderer with placeholder pixel art and a continuous day cycle, and a walkable prototype on the real engine at `spike-hub/` (not yet in the app; DEC-085) |
| Pixel-art tool P0 (DEC-084) | Validation spike built: `tools/pixelkit` with Mandate's first pack (palette, town-centre kit, tiles, props, 20 townsfolk), drawn in the town prototype with `?art=pixelkit&preset=street`. Round 2 after the owner's first look: buildings with shape (masses, roofs, corners, landmarks; neighbours never copy), flat townsfolk, lamp light clipped to the ground and splashed up walls, moods cross-fade. Round 2 status: done (building character, flatter characters, occluded lamp light). Round 3 status: done (flat graphic characters for the M6 testers). Round 3 (characters only): flat, graphic townsfolk (one shade per material, per-material outlines, 1×2 eyes, no blob heads, more cuts, hats and carried things). Round 4 status: done (layout v2 terrain art, `mandate-p0.5`): levels and faces, shores, creek and ford, rail with 45° pieces, platforms and level crossings, bridge kits in both axes, headframe, spoil heaps, reeds, stepping stones, market stalls, and the plot states (reserved, construction, boarded, derelict) in every family; palette 124 → 114 colours (D11). `ArtProvider.groundCell` (optional) is implemented by the pack; the hub renderer still has to call it for v2 towns. Results in `PIXEL_ART_TOOL.md` (P0 results, Round 2 results, Round 3 results, Round 4: v2 terrain); awaiting the owner's rubric scores |
| M2 Society, economy, environment | **Done**, 2026-11-18 |
| M3 Local government and policy | **Done**, 2026-11-18 (owner checkpoint) |
| M4 Elections, parties, NPC AI | **Done**, 2026-11-18 (owner checkpoint) |
| M5 Characters, places, and narrative | **In progress** (plan: ROADMAP, DEC-079). M5a done: places as the interface (located actions, auto-routing, pocket, information depth). DEC-078 decisions 1–4 done: balance numbers in content parameter blocks, lab `--set`/`--grid`, the concern registry with issue salience, and the scenario registry. M5b done: characters (DEC-031) and town character (DEC-077 §1). M5c done: the director (DEC-033), dilemmas (DEC-077 §4), NPC goals and storylines (DEC-032), scenes. M5d part 1 done: media, scandals, and the legal path (DEC-034, DEC-006). M5d part 2 done: situational policies (DEC-077 §3), bill and favour dilemmas, the variety gate (§5), the storyteller chosen at new game (DEC-033), the policy preview query, grounded overheard talk. |
| Save export and import (DEC-082) | **Done** |
| UI polish round | **Done** |
| Town generation v2 design (`docs/design/research/TOWN_GENERATION.md`) | **Done**; build at the start of M6 |
| Town layout v2, stage A (DEC-090; client generator) | **Done**, 2026-11-19: plan + dress in `web/src/hub/layoutV2` from client-derived geography (`townGeoAdapter.ts`, TODO D3) and JSONC registries (to move to mandate-content in stage B); rivers, bridges, coast, levels, typed districts with street patterns, mines, markets, reserved land, text alternative; prototype `?layout=v2`, v1 still the default; plan over the 60 ms budget (Fenoport 256 ms cold) — see TOWN_GENERATION.md "Stage A built" |
| Town map overview (DEC-086) | **Done**, 2026-11-19: the town screen opens on an illustrated map drawn from the versioned layout (tinted neighbourhoods, main routes, landmark glyphs in the pixel-art palette, home, you, things waiting); tap a building to go there; the list is the "List" tab (the accessible text view, remembered per device) |

## M5d part 2 summary (situations, variety, storytellers, previews, overheard talk)
- **Situational policies (DEC-077 §3):** `PolicyTemplate.unlock`, a reusable `SituationTrigger` (live crisis, indicator range, plant closure or disaster within N months); `mandate_sim::policy::{unlocked, triggered, situation_holds}`. Four base templates in `policies/situational.jsonc` (closure response, disaster rebuild fund, emergency rent, fiscal bridge levy), each listed among its crisis's answers. Locked ones show under "Not on the table now" with the reason and cannot be drafted; NPCs draft them only while unlocked.
- **Bill and favour dilemmas:** six more (`dilemmas.jsonc`, scenes and Fluent): an amendment that buys a vote, a sponsor's rider, the whip's deal, calling in a favour, a favour for a vote, a creditor against the rival's bill. New effects `rider` and `call_in`.
- **Variety gate (DEC-077 §5):** `bots` reports the best strategies per world and per starting situation (near-ties shared) and fails unless at least 2 strategies are the sole best somewhere and none is among the best in more than 60% of worlds; in CI and nightly. Worlds run on up to three threads (each world single-threaded; results keyed). Tuning in BALANCE_LOG.
- **Storyteller at new game (DEC-033):** profiles listed on the title screen (`QueryDto::Storytellers`), set in `NewGameParams.storyteller`, kept in the run's state and save, shown in the saved-runs list and the story sheet.
- **Policy preview (owner issue):** `QueryDto::PolicyPreview { template, params }` returns the engine's annual cost, share of the town budget, side effects (with a size in words), and support for those parameters; the catalogue's sliders ask it (debounced 150 ms) and render the answer. Side-effect sizes now come from the engine.
- **Overheard talk (owner issue):** a registry (`overheard.jsonc`, 53 lines, at least 5 per concern) with conditions on facts (water, crisis, plant closure, disaster, industry, broker, rival and the rival's record, the player's recognition and approval, a running scandal, a recent story, an election); `mandate_sim::overheard` draws 3 lines keyed by (town, neighbourhood, week, place) on the dialogue stream, at most one per concern where others hold, avoiding last week's lines. Lines carry names and facts as Fluent arguments (DEC-054). River lines need a river; plant lines need a job-loss crisis or a real closure.
- **Saves:** no format change (format v13); the shape fingerprint moved only because NPC drafts of the new templates add parameter names to the sample game.

## M5d part 1 summary (media, scandals, and the legal path)
- **Media (DEC-034):** `mandate_sim::media`, its own domain, active once play begins. Outlets from `outlets.jsonc` (a town paper at each newspaper office, a regional paper per region, a national broadcaster, a social platform): audience by class and age, reach, a lean drawn around their audience, credibility, interests, finances, journalists. Four layers: facts (facts log, hidden facts), knowledge, coverage (weekly stories by newsworthiness × interest × access, framed by hostility), perception (only through coverage: cohort approval and awareness of the player, any politician's scandal valence, colleagues' memories). Rumours from empty leaks, corrected by credible outlets. Press statements are covered (or not) on the outlets' terms; ads cool a scandal.
- **Hidden facts and scandals:** every borderline or illegal act (the player's or an NPC's) leaves a hidden fact whose fate (whether, and to whom) is settled when it happens from evidence × interest × access; the director times it within its evidence window (DEC-033 B). A fated fact becomes a scandal: stories, Standing, memories, and a response decision (deny, explain, apologise, counter-attack).
- **The legal path (DEC-006):** `mandate_sim::justice`: investigation (a decision: cooperate, a lawyer, stonewall, blame an ally), charges or dropped, trial (keyed odds), and a fine, removal, or prison. A short sentence is a time skip and a possible comeback; a long one or a disqualification ends the run, only after the warning and the response window. NPCs face the same path; their party fills a vacated seat.
- **Borderline and illegal choices:** five new dilemmas (a consultancy fee, a steered contract, a paid crowd, the bills at home, a job for a vote), shady options added to three (a buried report, an undeclared donation, a smear), and three shady actions (an envelope at the diner, the campaign card at the party office, a rival's file at the newspaper), each with a coarse risk line ("a low / real / high chance this comes out within a year") before the choice. NPCs are tempted by integrity and ambition.
- **Web:** the newspaper at the newspaper office (outlets, credibility, headlines, framing), headlines and a scandals-and-law card in the pocket, shady actions in the agenda and at their places with the risk line, response decisions in the decision sheet, a prison banner (ending the turn serves the sentence) and an end-of-run banner.
- **Lab:** a `corrupt` bot strategy (in the dominance gate) and `bots --careers 8` for career distributions; `story` prints the press and the case record; the sweep reports scandals and cases (zero there: a sweep has no NPC turns; `bots --careers` measures them).
- **Saves:** format v13 with a v12 migration (media and justice start empty; outlets are generated on load).

## M5c summary (the director, dilemmas, goals, storylines, scenes)
- **Director (DEC-033 B):** `mandate_sim::narrative`, its own domain and keyed RNG. Chapters are a campaign (climax: the election) or a session (a budget year; climax: the best grounded situation near the peak of its tension curve). Budgets (1 climax, 2–4 significant, 5–10 minor, `director.jsonc`), spacing, cooldowns, a repetition penalty, relevance scoring, and paced spending. Exogenous levers only, accepted and logged by their owners: hazards held while the player's town is at its crisis cap (timing only), a thinner hazard rate (Steady), a held recession onset, discovery dates of hidden facts. Fairness warnings (DEC-006) and an ending gate (`ending_allowed`). A facts log for the media task.
- **Storyteller profiles:** Classic (default), Unfiltered, Steady (`storytellers.jsonc`).
- **Dilemmas (DEC-077 §4):** 12 base dilemmas (the plant, the housing squeeze, rebuilding, the budget hole, the tax bills, pollution, building work, the jobless figures, a favour called in, the party line, a donor's offer, an attack ad) plus 3 storyline turns and 3 requests. Roles cast from live actors, triggers on live state, a deadline, every option costing someone (validated), real engine effects, and an aftermath of what each did.
- **Goals and storylines (DEC-032):** five goal templates (organise the workers, oppose the player, seek backing, champion a fix, rally the traders) that drive the AI and raise requests; three storylines (the plant, the rival, the favour debt) with branches, timeouts, and a follow-up hook.
- **Scenes (DEC-035, DEC-079):** decisions happen somewhere; at the place, the conversation, with quoted memories. The pocket always lists every decision and answers it.
- **Web:** pocket decisions with a badge, the decision sheet (trade-offs, deadline, default, aftermath), decisions waiting at a place, the story so far.
- **Lab:** bots answer by strategy; sweep story density; `mandate-cli story`.
- **Saves:** format v12 with a v11 migration (the director starts fresh); removed templates lapse instead of refusing.

## M5b summary (characters and town character)
- **Characters (DEC-031, DEC-018):** every politician is a Principal character; Named people from an occupation registry (shopkeepers, organisers, journalists, clergy, union stewards, editors, party chairs, plant managers, farmers, …) are generated when play begins and update lazily, order-independently; Ambient passers-by are generated per place and week and promoted when met. Personality (five axes, 0–3 traits from `traits.jsonc`), values conditioned on the home cohort, competencies.
- **Relationships, favours, memory:** directed edges (opinion, trust, bonds) and a favour ledger replace the player's `opinion_of_player`/`favors` with identical results. Bounded memory (20/5) from votes, party lines, favours, lobbying, elections, and mayoral signatures and vetoes; betrayals persist, trivia fades.
- **NPC AI reads personality:** loyalty weighs the party line, integrity resists lobbying, warmth and traits scale opinion changes.
- **Town character (DEC-077 §1):** defining industry, live crises from `crises.jsonc` (new `job_loss` indicator), power broker, the player's rival, priorities; NPC agendas favour bills that answer them.
- **Protocol and web:** `CharacterView`, `TownCharacterView`, people at places, `Meet`; a Town card, "People here" on place panels, and character sheets (also from the whip list).
- **Saves:** format v11, with a migration from v10.
- **Balance:** see BALANCE_LOG (bots and sweeps pass; generation +8–10%).
- **Director hooks:** `salient_memories`, `relationship_delta`, `TownCharacter::crisis()` with severity history, broker and rival ids (EXTENDING.md).

**Site (DEC-083):** a landing page at the site root (`web/landing/`, a broadsheet front page with images captured from the game); the game moved to `play/`, with a retiring service worker at the old path so installed copies switch over (migration tested in Playwright).

## M0 summary
- **Rust workspace:** 8 crates, with determinism lints enforced (banned `HashMap`, `std` float transcendentals, wall clock).
- **Core primitives:** typed IDs, calendar, fixed-point money, `libm` math, ChaCha RNG streams plus keyed access, BLAKE3 hashing.
- **Content:** JSONC pipeline with line-accurate errors, generated schema (`content/schemas/`), and validation that includes Fluent keys. The base pack is embedded in the WASM.
- **Engine:** 13-phase cadence scheduler, effect queue, event log, state hash. Stub systems exercise every mechanism.
- **Saves:** versioned, compressed, integrity-hashed, with sync metadata (DEC-044 rev. 2, DEC-052). A shape-fingerprint test guards the format version.
- **Protocol:** DTOs with TypeScript generated through ts-rs. The WASM session is natively testable.
- **Lab CLI:** `content validate|schema`, `run`, `determinism`, `save-info`.
- **Web:** React 19, TypeScript 6 (strict), Vite 8 PWA; engine worker; typed client; Dexie `SaveStore`; Fluent text; Ironman autosave every turn; offline play.
- **Verified:**
  - same seed gives the same hash, twice;
  - save, load, continue gives an identical hash;
  - **native = WASM** state hash;
  - Playwright on desktop and phone: new game → end turn → save → reload offline → resume with an identical hash.
- **CI:** cached WASM job; Rust, web, and E2E jobs; Pages deploy from `main`.

## M2 summary
- **Population:** 12 cohorts per neighbourhood with births, ageing, deaths, class mobility, immigration, internal migration toward attractive towns, and disaster displacement. Exact people accounting (tested).
- **Economy:**
  - the global cycle with recessions and commodity shocks;
  - a national macro loop (Phillips curve, Taylor rule, output gap);
  - per-municipality jobs, wages, and incomes in 8 sectors, with plant closures;
  - housing markets with a construction pipeline.
- **Public finance:** local, regional, and national treasuries; taxes, grants, entitlements, interest, and debt; a baseline fiscal rule.
- **Environment:** a seeded trajectory, resource stocks, pollution, and hazards (floods, storms, wildfires, earthquakes, regional droughts), with consequences delivered as effects.
- **History warm-up:** 20 years of NPC-only simulation before play. About 240 ms for medium worlds in WASM.
- **Lab:** `trace-econ`, `sweep`, `diff`. All presets pass the stability bands and drama floors over 24 seeds × 30 years.
- **Tests:** finiteness and bounds, population accounting, the factory-closure causal chain (brief §7.2), burn-in history.
- **UI:**
  - a national economy card (stat tiles with deltas and sparklines);
  - a recession badge;
  - a town economy section (stat tiles, jobs-by-sector meters, town budget);
  - an unemployment map layer with a legend;
  - Fluent text for all new events.
- **Saves:** format v3.

## After M4 (owner feedback)
- **Every decision is a trade-off (DEC-073):**
  - ads buy reach with money instead of time;
  - canvassing costs health;
  - fundraising courts donors;
  - press and town halls carry visible risks;
  - the player now casts their own council vote, weighing party, ward, and favours;
  - riders trade cost for passage;
  - exhaustion costs slots.
- **Saves survive app updates (DEC-074):** a full save after every action (no replay), and saves load under updated rules when nothing they need was removed.

## M4 summary
- **Research pass** before building: `docs/design/research/SIMULATION_MODELS.md` (DEC-071).
- **Parties** generated from cleavages (two competitive majors and a minor party); mover–stayer party attachment; gravity-model migration.
- **Elections:**
  - calendar from the constitution, candidacy and nomination;
  - per-cohort logit vote with separate turnout and strategic voting;
  - hidden shocks, plurality tabulation, seat changes, and a "why" decomposition.
- **Polls** with shared error, house effects, and sampling noise.
- **Campaigns:**
  - weekly turns;
  - canvassing, town halls, and ads build awareness and turnout, with diminishing, decaying persuasion;
  - fundraising.
- **NPC utility AI** (content-defined): bills with sunsets, lobbying for a safety margin, campaigning, fundraising. Party discipline in votes.
- **Career:** defend your seat, run for mayor, or stand down; losing leaves you out of office.
- **UI:**
  - a Campaign tab: election calendar and filing, your race with a poll-trend chart (with a table view), other races, last results, and parties;
  - an election-night screen with result bars, final-poll markers, and the factor breakdown;
  - party colours from the validated categorical palette, always labelled.
- **Balance:**
  - burn-in re-election rate 76–83%, turnout about 62%;
  - bots: no dominant strategy, and effort matters (BALANCE_LOG);
  - sweeps still pass.
- **Tests:**
  - elections are plausible across seeds;
  - councils and roles stay consistent;
  - a full player election (Rust and Playwright, phone and desktop);
  - laws are funded by the next budget year.
- **Saves:** format v5.

## M3 summary
- **Constitution primitives** with the Federal Presidential archetype. Each town has a council elected by ward and a mayor. Ordinances and budgets pass by council vote, then mayoral signature, with a two-thirds override.
- **Policy:** 11 local templates over 8 typed mechanisms. A 5-state lifecycle, staged procedures with deliberation periods, an implementation gap, and appropriations-driven funding. Laws keep a ledger of what they have done.
- **Opinion:** anticipated support by income class before a vote; experienced effects after enactment. Government approval and the player's awareness and approval per neighbourhood.
- **Player:** a seat on the council of the largest town; slots, late-night slot, Mandate, pressure tracks, and campaign funds; seven actions; draft, amend (riders: ward project, sunset, pilot), introduce, and withdraw.
- **UI:**
  - **Office tab:** player card, agenda, bills with "Why?" (support by class, side effects, ledger, vote record), policy catalogue with parameter sliders, and town opinion.
  - **Council tab:** whip list with lobbying odds, late-night and favour options, and members' convictions.
  - HUD slot pips and an action-result toast.
- **Saves:** format v4. Every player command is journalled, so a reload mid-turn resumes exactly (tested end-to-end).
- **Tests:**
  - a bill goes from draft to law;
  - a housing programme adds homes compared with a control world;
  - slots are spent and validated;
  - player commands are deterministic;
  - state with lobbying round-trips through JSON;
  - favour debt is capped;
  - Playwright: draft → introduce → lobby → reload mid-turn → identical state hash.

## M1 summary
- **World generation pipeline:**
  1. Terrain: warped fBm, ridged mountain ranges, sea sides.
  2. Climate: latitude and lapse rate; prevailing wind with rain shadow.
  3. Hydrology: priority-flood, D8 routing, rivers, lakes.
  4. Whittaker biomes.
  5. Resources and hazards.
  6. Settlement scoring and rank-size populations.
  7. Terrain-cost territories.
  8. Regions grown along natural borders.
  9. A* road network (MST plus shortcuts) with rail spines.
  10. Neighbourhoods.
  11. Wards and regional and national districts (contiguous, balanced).
  12. Places, including the parliament in the capital.
- **Scale (medium):** about 10 regions, 56 municipalities, about 400 neighbourhoods, about 500 districts, and about 800 places. About 90 ms in WASM.
- **Five canonical presets**, tested for their intended biases.
- **World invariants are property-tested:** ownership, contiguity, population accounting, district partitions, road connectivity, and determinism.
- **Lab:** `mandate-cli gen --png` renders a map; `inspect` shows a municipality or region.
- **Web:** a Map tab with a pan, zoom, and pinch pixel map and tap-to-inspect towns; an Overview tab with a region and town list as the accessible alternative; a town panel showing climate, resources, hazards, neighbourhoods, and places; a scenario picker; a generation progress bar.
- **Saves:** format v2 (world model change). M0 saves are rejected with a clear message (pre-release policy).

## Deferred from M0 (tracked)
- Module-boundary architecture test: added in M2, when domain modules exist.
- React Compiler evaluation: M6 performance pass.

## Owner notes
- Logo and loader: the poll-line M and the career-ladder loader (DEC-081, `web/src/components/Brand.tsx`); the owner may still supply their own. After changing `web/public/icons/icon.svg`, regenerate the PNG icons.
