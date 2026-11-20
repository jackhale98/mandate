# Mandate — Core Data Model

**Status:** Describes the state on `main` as of 2026-11-19 (save format v12), derived from `crates/mandate-sim/src`. Field lists name the main fields, not every one; the Rust types are the source of truth. **Planned** marks entities that are decided but not built.
**Conventions (DEC-041):**
- Typed `u32` IDs, monotonic and never reused.
- Content references use stable string keys (`ContentKey`, for example `sector.retail`).
- Money is `i64` minor units; counts are integers; continuous values are `f32` or `f64`, clamped.
- Collections are `Vec` indexed by ID, or `BTreeMap` (serialised as lists of pairs when keys are not strings). No hash maps in state.
- Vectors "in registry order" follow the content registry's load order; the state records the keys it was built with, so content can add or reorder entries (DEC-075).

## 0. `State`

`State` (`state.rs`) is everything a save holds and everything the state hash covers. Each domain is written only by its own systems (OVERVIEW §2, §4.3).

| Field | Type | Owner |
|---|---|---|
| `meta` | `WorldMeta`: seed, `WorldParams` (size; scenario id, DEC-078), constitution key, content hash, engine version, start date | engine |
| `calendar` | `Calendar`: date, turn, phase (`BurnIn`, `Campaign`, `Governing`), act, chapter | engine |
| `rng` | `RngStreams`: the world seed and one ChaCha state per named stream | engine |
| `world` | `World` (§1) | worldgen (read-only in play) |
| `events` | `EventLog` (§7) | engine |
| `population` | `PopulationState` (§2) | population |
| `economy` | `EconomyState` (§2) | economy |
| `finance` | `FinanceState` (§3) | finance |
| `environment` | `EnvironmentState` (§2) | environment |
| `politics` | `PoliticsState` (§4), which also stores `CharactersState` (§5) | politics, characters |
| `policy` | `PolicyState` (§3) | policy |
| `player` | `Option<PlayerState>` (§5); `None` during the burn-in | player |
| `narrative` | `NarrativeState` (§6) | narrative |

The terrain raster is not in `State`: it is rebuilt from the seed and scenario on load (`rebuild_terrain`), checked by a terrain hash (DEC-068).

## 1. World and space — `world.rs`

| Entity | ID | Main fields |
|---|---|---|
| `World` | — | `geo` (raster size, km per cell, prevailing wind, terrain hash), capital, regions, municipalities, neighbourhoods, constituencies, transport links, places |
| `Region` | `RegionId` | name, capital, municipalities |
| `Municipality` | `MunicipalityId` | name, region, `SettlementClass` (metro, city, town, rural), centre cell, coastal / river / port flags, neighbouring towns, neighbourhoods, places, `GeoProfile` (climate, resources, hazard exposure, dominant biome) |
| `Neighborhood` | `NeighborhoodId` | municipality, name, `NeighborhoodKind` (core, inner, waterfront, industrial, suburb, rural), adjacency. Residents live in `population`, not here |
| `Constituency` | `ConstituencyId` | tier (local, regional, national), jurisdiction, number, neighbourhoods (contiguous, population-balanced) |
| `Jurisdiction` | — | a government's territory at any tier: `municipality { id }`, `region { id }` or `nation` (DEC-098). The key for state that belongs to a town, a region or the nation (posts, bodies, offices); `Area` (`hood`, `constituency`, `in { at }`) says where an effect applies. Physical things in a town keep `MunicipalityId` |
| `TransportLink` | `LinkId` | two towns, mode (road, highway, rail), length, travel cost, cell path |
| `Place` | `PlaceId` | `PlaceKind` (23 kinds: town hall, council chamber, diner, market, school, clinic, community centre, park, news office, party office, union hall, business association, factory, port, farm, mine, mill, hotel, university, tech campus, regional assembly, parliament, executive residence), municipality, neighbourhood, name. What a kind offers comes from the `places` registry (DEC-079) |

The street plan of a town is not engine state: the client derives it from the `Town` view, the world seed, and a layout version (DEC-085).

## 2. Society — `population`, `economy`, `environment`

| Entity | Where | Main fields |
|---|---|---|
| `PopulationState` | `population` | `attitude_keys`, `community_keys` (registry keys the state was built with), `hoods` (one `HoodPop` per neighbourhood), demography history, `concern_keys` and `town_concerns` (one `TownConcerns` per municipality) |
| `HoodPop` | per neighbourhood | children (and `children_by_community`, empty while there is one community), `cohorts` indexed by `cohort_index(age, class, community)` |
| `Cohort` | per neighbourhood × age band (4) × income class (3) × community | count; `attitudes` (registry order); trust; turnout; approval of the local government; awareness and approval of the player; party attachment per party; last turnout (habit) |
| `TownConcerns` | per municipality | a level per concern and a salience per attitude (mean 1), recomputed monthly (DEC-078 decision 3) |
| `EconomyState` | `economy` | `sector_keys`; `Global` (demand, recession, commodity prices, a recession the director holds); `National` (price level, inflation, policy rate, output gap, unemployment, GDP, productivity, history); `munis` |
| `MuniEcon` | per municipality | one `SectorState` per sector (capacity, jobs, wage, productivity), labour force, unemployment, employment and income by class, consumption, output, `Housing` (stock, prices, rents, pipeline), attractiveness, active sector shocks, months since a disaster, monthly history |
| `EnvironmentState` | `environment` | `Trajectory` (warming, drying, storminess), one `RegionEnv` per region (temperature anomaly, water, mineral, timber and fish stocks, pollution, storm multiplier), disaster count, hazards deferred by the director (`DeferredHazard`: town, `HazardKind`, severity, date drawn) |

## 3. Government — `policy`, `finance`, constitution content

| Entity | ID | Main fields |
|---|---|---|
| Constitution | content key in `meta.constitution` | Not state: a content definition built from primitives (DEC-017): bodies (with a member word, seats, workplace and optional own calendar), offices (with term limits, workplace and optional own calendar), powers, decision rules, constituencies, staged procedures (`ordinance` and `budget` are a contract with the engine), election calendars, and the `career` ladder (summit and acts). Politicians hold its posts by id (DEC-098). The Federal Presidential archetype ships; regional and national tiers are defined but not simulated yet |
| `PolicyState` | — | id allocator, `instances`, one `Agency` (competence) per municipality |
| `PolicyInstance` | `PolicyInstanceId` | template key, tier, jurisdiction, sponsor, parameters, clauses (`Earmark` for a ward, `Sunset`, `Pilot` in a neighbourhood), `PolicyStatus` (draft, under consideration, enacted, in effect, ended), procedure stage and next date, vote records (including vetoes), dates, implementation progress, public awareness, end reason, cause, attribution ledger |
| `FinanceState` | — | tax rates (`TaxRates`: property per town, sales per region, national income tax by class, business), `appropriations` keyed by policy id (funded share), local treasuries (per town), regional treasuries, the national treasury, grant rates, structural deficit, indexation bases |
| `Treasury` | per jurisdiction | tier, cash, debt, budget by category (9 fixed categories), revenue by source (6 fixed sources), year-to-date figures, last year's summary |

Budget categories, revenue types, and hazards become registries before national play (DEC-078 decision 5, **planned** for M7).

## 4. Politics — `politics`

| Entity | ID | Main fields |
|---|---|---|
| `PoliticsState` | — | id allocator, politicians, `bodies` (`BodySeats`: a constitution body id, its `Jurisdiction`, members by constituency, where it sits) and `offices` (`OfficeHolder`: an office id, its `Jurisdiction`, the holder, where they work), parties, `ElectionState`, `sway` (politician, policy) → lobbying persuasion, `declared_votes` (politician, policy) → yes / no / abstain, `characters` (§5) |
| `Politician` | `PoliticianId` | name, `Role` (a `Post`, a seat in a constitution body or a constitution office for a `Jurisdiction`, or private with a home town; DEC-098), `is_player`, birth year, attitudes (registry order), pragmatism, integrity, oratory, negotiation, ambition, `character` (their `CharacterId`), party, home neighbourhood, scandal damage, funds, years and terms in office, `record` of visible work (DEC-076) |
| `Party` | `PartyId` | name, short name, platform (blended from factions), `factions` (wing key, position, strength), colour slot |
| `ElectionState` | — | the current campaign's `Race`s, polls, hidden national swing, pollster house effects, results, last turnout per neighbourhood, dates of the last election, the campaign opening, and the next election |
| `Race` | by `Post` (a seat in a body, by constituency, or an office; each post on its own term, limit and calendar, DEC-098) | dates, neighbourhoods, incumbent, `Candidate`s (awareness, persuasion, mobilisation, contacts per neighbourhood; spending), partisan ballot, hidden shocks and poll bias |
| `RaceResult` | — | seat, date, per-candidate votes, shares, and final poll, winner, incumbent, turnout, the winner's share by neighbourhood, and the six-factor decomposition (issues, party, candidate, record, campaign, luck) for player-era races |
| `Poll` | — | race, date, pollster, shares among decided voters, undecided, sample size |

Not built (**planned**): pledges (DEC-011), primaries, regional and national races.

## 5. People and the player — `characters`, `player`, `places`

| Entity | ID | Main fields |
|---|---|---|
| `CharactersState` | — | id allocator; `people`; `edges` (holder, other) → `Relationship`; `favours`; `towns` (one `TownCharacter` per municipality); `named_seeded`; `crisis_keys`; `goals` and recently `ended_goals` (DEC-032) |
| `Character` | `CharacterId` | `politician` (if they are one) or `named` (`NamedCore`: name, birth year, home, occupation key, workplace, ambition, integrity, oratory, negotiation, values, last lazy update, Ambient origin); warmth, boldness, loyalty; administration, organising, intrigue, expertise (an attitude key); 0–3 trait keys; first impression; `memories` |
| Tier | derived | Principal (every politician), Named (stored non-politicians); Ambient passers-by are not stored: generated per place and week from keyed RNG, and promoted to Named by `Meet` (DEC-018) |
| `Relationship` | (holder, other) | opinion (−100..100), trust (0..1), bonds (family, friend, ally, rival, mentor, employer, patron). Pairs without history are absent |
| `Favour` | — | debtor, creditor, count, Fluent context key, subject, since |
| `Memory` | owned by a character (at most 20 for Principal, 5 for Named) | `MemoryKind` (17 kinds: backed or opposed my bill, promised, kept, or broke a favour, broke ranks, lobbied or pressed me, defeated me, lost to me, vetoed or signed my bill, met, scandal, stood with or against me, ignored me), actor, subject (a policy, a seat, or a decision), sentiment, magnitude, date, decay, opinion change |
| `Goal` | (holder, goal key) | since, lean, champion strength |
| `TownCharacter` | per municipality | defining industry (sector key) and its share, `crises` (`ActiveCrisis`: kind, severity, since, monthly history), power broker, the player's rival, priorities (top concern keys) (DEC-077 §1) |
| `PlayerState` | — | politician, municipality, ward, standard slots and slots used, late night used, Mandate, money, staff, `Tracks` (viability, legal jeopardy, threat, standing, health), recent actions, election filing, ads this turn, last election seen, `location` |
| `Location` | — | `Home`, `Street { neighborhood }`, or `Place { place }` (DEC-079) |

Not built (**planned**): appearance parameters (with the pixel-art tool, DEC-084 P1), NPC schedules and locations, organisations as actors (DEC-020, DEC-034), household and personal finances (DEC-037, M6).

## 6. Narrative — `narrative`

| Entity | ID | Main fields |
|---|---|---|
| `NarrativeState` | — | storyteller profile key; when play began; the current `Chapter` and closed ones; tension; `decisions`; `storylines`; cooldowns by template; requests raised by goals; follow-up storylines waiting; the facts log (`Fact`, bounded); `hidden` facts; `Chance` requests; fairness `warnings`; what the director saw last week (`Seen`); lab switches (`spotlight`, `resolve_defaults`) |
| `Chapter` | index | `ChapterKind` (session: a budget year; campaign: ends on election day), start, planned and actual end, counts of climaxes, significant, and minor beats, the climax decision, peak tension, templates and people spotlighted |
| `Decision` | `DecisionId` | dilemma template, weight, cast (role → actor), reasons, opened, deadline, chapter, relevance, storyline, the goal that raised it, the place of its scene, `DecisionStatus` (pending, answered, defaulted, lapsed), `Resolution` (option, date, by default, what was applied) |
| `StorylineRun` | `StorylineRunId` | template, cast, current stage, entered, since, started, the stage's decision, stages reached, end (ended, faded, removed) |
| `Fact` | — | id, date, chapter, `FactKind`, actors, spotlit, newsworthiness |
| `HiddenFact` | — | kind (late-night deal, late-night fundraising, donation), date, actors, evidence window, discovery date chosen by the director, surfaced |
| `Chance` | — | hazard hold on a town until a date, the share of hazard strikes kept (Steady), recession hold |

Not built (**planned**, M5d part 1): outlets, coverage, perception, and scandals as media layers (DEC-034).

## 7. Engine

| Item | Main fields |
|---|---|
| `EventLog` | id allocator, `DomainEvent`s (id, date, `EventKind`, causes, retention `Recent` or `Pinned`); the 2,000 most recent are kept |
| `Effect` | transient cross-domain requests within a turn: `HousingDamage`, `SectorShock`, `Displacement`, `Memory` |
| Commands | not stored: each player command is applied, then the whole game is saved (DEC-074) |

## 8. Save mapping (DEC-044 rev. 2, DEC-052, DEC-074)

- The save payload is the JSON of `State`, DEFLATE-compressed, with a BLAKE3 hash in the metadata. Format version **12**; a shape-fingerprint test forces a bump on any change.
- Run metadata (`SaveMeta`): format version, `run_id`, `revision`, `parent_revision`, `device_id`, `saved_at`, engine version, content hash, protocol version, run mode (Ironman or Story), seed, simulation day, turn, payload hash.
- Registry keys saved for remapping on load: `economy.sector_keys`, `population.attitude_keys`, `population.community_keys`, `population.concern_keys`, `politics.characters.crisis_keys`, and `meta.params.scenario`. How `Engine::from_state` uses them: OVERVIEW §6.
- Migrations: `v9 → v10` (scenario ids), `v10 → v11` (politicians become characters), `v11 → v12` (the narrative domain starts fresh). Older saves are refused.
- The web app's own run metadata (`RunRecord` in `web/src/persistence/`) sits beside the save bytes; the town layout version per run will be recorded there (DEC-085, **planned**).
