# Progress

**Phase:** Implementation. The plan is approved (DEC-067).

## Milestone status
| Milestone | Status |
|---|---|
| M0 Foundation | **Done**, 2026-11-18 |
| M1 World and lab | **Done**, 2026-11-18 |
| M1s Rendering spikes | Map: Canvas 2D in production (fast, pixel-art); hub spike with M6 |
| M2 Society, economy, environment | **Done**, 2026-11-18 |
| M3 Local government and policy | **Done**, 2026-11-18 (owner checkpoint) |
| M4 Elections, parties, NPC AI | **Done**, 2026-11-18 (owner checkpoint) |
| M5 Characters, places, and narrative | **In progress** (plan: ROADMAP, DEC-079). M5a done: places as the interface (located actions, auto-routing, pocket, information depth). DEC-078 decisions 1–4 done: balance numbers in content parameter blocks, lab `--set`/`--grid`, the concern registry with issue salience, and the scenario registry. |

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
- Logo and loading animation: to be supplied by the owner. The placeholder icon is in `web/public/icons/icon.svg`; run `node web/scripts/gen-icons.mjs` after replacing it.
