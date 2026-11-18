# Mandate — Core Data Model

**Status:** Proposed final baseline. It lists the authoritative simulation entities, their identifiers, owning modules (OVERVIEW §2), and key fields. Field lists are indicative; exact types are fixed during implementation through interface-first tasks.
**Conventions (DEC-041):**
- Typed `u32` IDs, monotonic and never reused.
- Content references use stable string keys (`ContentKey`).
- Money is `i64` minor units; counts are integers; continuous values are `f64`, clamped.
- Collections are `Vec` indexed by ID, or `BTreeMap`. No hash maps in state.

## 1. World and space — module `world` (built by `mandate-worldgen`)

| Entity | ID | Key fields | Notes |
|---|---|---|---|
| `WorldMeta` | — | seed, size class, era volatility, content hash, engine version, start date | One per run |
| `TerrainRaster` | — | elevation, water, temperature, precipitation, biome, resource and hazard layers (~512²) | Generation and environment events only |
| `Region` | `RegionId` | name, capital, municipalities, environment state ref | Tier: Regional |
| `Municipality` | `MunicipalityId` | name, region, neighborhoods, settlement class, sector economy ref, housing market ref, hub layout ref | Tier: Local |
| `Neighborhood` | `NeighborhoodId` | municipality, adjacency, land use, cohort cells | Finest population resolution |
| `Constituency` | `ConstituencyId` | body, member neighborhoods, seats, electoral rule ref | Contiguous; redistrictable |
| `Place` | `PlaceId` | kind (town hall, union hall, diner, factory…), location, owning org or government, hub tile anchor, open hours by day-part | Every hub POI is a Place (DEC-009) |
| `TransportLink` | `LinkId` | endpoints, mode, travel cost, condition | Graph edges |
| `HubLayout` | `HubId` | municipality, tile map (render-agnostic), POI → `PlaceId` map | Generated deterministically |

## 2. Society — modules `population`, `economy`, `environment`

| Entity | ID | Key fields |
|---|---|---|
| `CohortCell` | (`NeighborhoodId`, `CohortKey`) | Count; distributions (sector employment, tenure, education, household type); income; attitude vector (~8); issue salience; party-ID distribution; turnout propensity; institutional trust; awareness and perceptions of officials |
| `SectorEconomy` | per `MunicipalityId` × `SectorKey` | Output, employment, wages, firms (representative), capacity, costs |
| `HousingMarket` | per `MunicipalityId` | Stock, vacancy, rents, prices, construction pipeline |
| `NationalMacro` | — | Price level, inflation, policy rate (central bank actor), national debt, external demand, commodity prices |
| `EnvironmentState` | per `RegionId` | Resource stocks, water availability, hazard intensities, pollution, trajectory params (DEC-023) |

## 3. Government — modules `constitution`, `policy`

| Entity | ID | Key fields |
|---|---|---|
| `Constitution` | — | Instance assembled from content primitives (DEC-017): offices, bodies, selection methods, voting rules, decision rules, powers, procedures, triggers |
| `Jurisdiction` | `JurisdictionId` | Tier (local, regional, national), territory ref, bodies, offices, treasury |
| `Body` | `BodyId` | Kind, seats, members (`CharacterId`), leadership, decision rule, powers |
| `Office` | `OfficeId` | Kind, holder, term start and end, limits used, powers, removal triggers |
| `Treasury` / `Budget` | per `JurisdictionId` | Revenues by source, appropriations by category, obligations, balance, debt |
| `Agency` | `AgencyId` | Jurisdiction, domain, staff, competence, leadership appointee, budget |
| `PolicyInstance` | `PolicyInstanceId` | Template key, parameters, clauses (riders, earmarks, sunset, pilot scope), sponsors, state (Draft / Under consideration / Enacted / In effect / Ended), procedure stage, funding status, implementation progress, issue flags, attribution ledger |
| `ProcedureRun` | `ProcedureRunId` | Procedure ref, subject (policy or budget), stage, votes, deadlines |

## 4. Politics — module `politics`

| Entity | ID | Key fields |
|---|---|---|
| `Party` | `PartyId` | Name, factions, leadership, platform, brand per cohort, organization (money, volunteers, branches), cohesion |
| `Faction` | `FactionId` | Party, ideology vector, leader, members, regional and cohort base |
| `Election` | `ElectionId` | Constituency or office, schedule, candidates, campaign states, result, swing decomposition |
| `CampaignState` | per candidate × election | Funds, staff, volunteers, ad spend, awareness by neighborhood, endorsements |
| `Poll` | `PollId` | Pollster org, date, sample, estimates, house effect |
| `Pledge` | `PledgeId` | Owner, measurable condition, deadline, status (DEC-011) |

## 5. People and organizations — modules `characters`, `organizations`, `player`

| Entity | ID | Key fields |
|---|---|---|
| `Character` | `CharacterId` | LOD tier (Principal / Named), identity (name, age, home, appearance params, role or background key), personality axes and tags, values vector, competencies, resources, schedule by day-part, goals, pressure tracks (if a politician), memberships |
| `Relationship` | (`CharacterId` → `CharacterId`) in `BTreeMap` | Opinion, trust, bond types, favor ledger, last interaction |
| `Memory` | owned by `Character` (bounded) | Event ref, actor, sentiment, magnitude, date, decay, tags |
| `Organization` | `OrganizationId` | Kind (union, business association, outlet, NGO, pollster, firm, party branch), leadership, membership or audience, resources, internal selection rule, places |
| `Player` | — | Character ref, slots and commitments, Mandate, money (personal and campaign), staff, pledges, pressure tracks, run mode, directives (time skip) |
| Ambient people | none (not stored) | Pure function of (seed, place, date, cohort sample) → promoted to `Character` when relevant (DEC-018) |

## 6. Information and narrative — modules `information`, `narrative`

| Entity | ID | Key fields |
|---|---|---|
| `Fact` | `FactId` | What happened (event ref), truth, secrecy, evidence items, exposure risk |
| `Knowledge` | (`ActorRef`, `FactId`) | Known since, evidence level, source |
| `Coverage` | `StoryId` | Outlet, facts, framing, salience, reach |
| `Perception` | per cohort and NPC | Belief state about facts, issues, and officials (within `CohortCell` / `Character`) |
| `StorylineInstance` | `StorylineId` | Template key, cast (role → entity), stage, branch history, follow-up hooks |
| `DirectorState` | — | Chapter tension curve, budgets used, cooldowns, open crises, chance-shaping log |
| `DomainEvent` | `EventId` | Kind, date, actors, subjects, causes (`EventId` or command index), retention class (recent / summarized / pinned) |

## 7. Engine — module `engine`

| Item | Key fields |
|---|---|
| `Calendar` | Date, turn number, phase (campaign / governing / crisis), chapter and act indices |
| `RngStates` | Per named stream; keyed derivation params |
| `CommandLog` | Commands since last snapshot (for replay and crash recovery) |
| `EffectQueue` | Pending cross-domain effect requests (transient within a turn) |

## 8. Save mapping (DEC-044, TECH-009)

Save DTOs mirror these entities in a **versioned external schema**. They are never serde-derived directly from internal structs. Each DTO has a migration function. Run metadata: `run_id`, `revision`, `parent_revision`, `device_id`, `saved_at`, content hash, engine, content, and protocol versions, run mode, integrity hash.
