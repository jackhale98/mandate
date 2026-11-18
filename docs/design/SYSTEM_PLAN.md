# Mandate — Definitive System Plan

**Status:** Final baseline, **awaiting owner approval** (brief §30 Step 6; DEC-003). Once it is approved, implementation begins at M0.
**How to read it:** this is the single entry point. Each section summarizes the accepted design and links to the authoritative decision (DEC) or spec. Where this document and a DEC entry disagree, the DEC entry wins and this document gets fixed.

---

## 1. Final game design
*Mandate* is a political career roguelike in a procedurally generated fictional country, told in a tone of heightened realism (DEC-061). You climb from local office toward head of government, and **you can only do what you can get others to do** (DEC-010).

**Pillars** (GAME_VISION.md):
1. Power through people.
2. Consequences you can walk through.
3. Real stakes, fair losses.
4. A different country, with different rules, every run.
5. A climb in acts.

**Structure** (DEC-004, DEC-005, DEC-014):

| Act | Tier |
|---|---|
| 1 | Local |
| 2 | Regional |
| 3 | National |
| 4 | Head of Government |

- Chapters last 25–45 minutes and each ends in a resolution.
- A full career takes 6–10 hours; a typical run, 2–5 hours.
- Players can skip tiers or move down the ladder.

**Endings** (DEC-006):
- **Summit:** the Mandate Test, the first national verdict on your presidency as scheduled by the constitution.
- **Terminal endings:** death, prison, disqualification, disgrace, political wilderness, age, retirement.
- **Survivable setbacks:** a narrow loss, a short sentence, a scandal, party expulsion.
- Five visible pressure tracks (Viability, Legal Jeopardy, Threat, Standing, Age/Health) work as hit points. Every ending is preceded by a warning and followed by an explanation.
- Every run closes with a **Legacy epilogue**.

**Modes:** Ironman by default, with an opt-in Story mode (DEC-008).

**Character:** drafted from 3 generated candidates (DEC-036). Personal life is light but has consequences (DEC-037).

**Career detours** (union boss, commentator, lobbyist, and others) rebuild weaknesses between offices (DEC-020).

**Variation:**
- Constitutions are built from primitives as archetypes plus quirks (DEC-007, DEC-015, DEC-017; curveball pool in `research/GOVERNANCE_CURVEBALLS.md`).
- The MVP uses the Federal Presidential Republic (DEC-016).

**Meta-progression:** unlocks of new possibilities only, plus a Hall of Legacies (DEC-039).

## 2. Complete gameplay loop
- **Turn** (1–3 min; DEC-012, DEC-013): one stylized "representative day" whose header shows the calendar span. The turn runs:
  1. Briefing.
  2. Spend Morning, Afternoon, and Evening slots, plus a risky Late Night slot, on located actions in the hub. These include conversations, negotiations, campaign activities, and desk work on bills, the budget, and agencies.
  3. Delegate routine work to staff.
  4. Continue multi-turn commitments.
  5. End Turn: WEGO resolution.
  6. Developments summary.
- **Turn length by phase:**

  | Phase | Turn length |
  |---|---|
  | Campaign | 1 week; daily in the final stretch |
  | Local and regional governing | 2 weeks |
  | National governing | 2–4 weeks |

- **Chapter:** a campaign or a session or budget year, ending in a resolution screen (election night, session record, budget).
- **Act:** an epilogue, then *term in review*. The player sets directives, time skips forward, and **interrupts** stop the skip on danger or a critical decision. Then a homecoming or a new hub.
- **Power and resources** (DEC-011): four distinct kinds, set against the five pressure tracks:

  | Kind | Form |
  |---|---|
  | Authority | Rules from the constitution |
  | Mandate | A meter that decays over a term |
  | Relationships | A per-NPC ledger |
  | Resources | Time, money, staff |

  Pledges turn campaign promises into measurable goals.

## 3. Simulation systems and responsibilities
Each system lives in a module of `mandate-sim`.

| System | Responsibility | Key decisions |
|---|---|---|
| `constitution` | Offices, bodies, selection, voting and decision rules, powers, procedures, triggers | DEC-015, DEC-017 |
| `population` | Cohort cells per neighborhood (12 cohorts); demography; migration; opinion state | DEC-018, DEC-025 |
| `economy` | ~8 sectors, employment, wages, housing, per-tier public finance, national macro loop | DEC-019 |
| `environment` | Regional resource stocks, water, hazards, pollution, seeded trajectory | DEC-023 |
| `policy` | Templates, 5-state lifecycle, mechanisms, implementation gap, attribution | DEC-024, DEC-026 |
| `politics` | Parties and factions, elections, polling, campaigns | DEC-027 to DEC-029 |
| `characters` | Principal and Named NPCs, relationships, favors, memory, goals, schedules, LOD tiers | DEC-018, DEC-031, DEC-032 |
| `organizations` | Unions, associations, outlets, NGOs, pollsters, firms, with internal selection | DEC-020, DEC-034 |
| `information` | Facts, knowledge, coverage, perception; scandals; rumors | DEC-034 |
| `narrative` | Director (chance, not consequence), pacing budgets, storylines, fairness warnings | DEC-033, DEC-032 |
| `player` | Player state, slots, commitments, pledges, directives | DEC-011, DEC-013 |
| `engine` | Commands and queries, scheduler, effect queue, event log, view models, state hash | DEC-042 |

**NPC AI** is shared by NPCs, the time-skip proxy, the burn-in, and the bot players. It is symmetric with player rules (DEC-030).

## 4. System interaction and update ordering
- **Calendar:** the base unit is the day. Systems run at their declared cadences, triggered by the boundaries a turn crosses.
- **Order of resolution** (OVERVIEW §4.2):
  1. Commands.
  2. NPC actions.
  3. Institutions.
  4. Policy implementation.
  5. Environment.
  6. Economy.
  7. Population.
  8. Information.
  9. Opinion.
  10. Elections and polls.
  11. Characters.
  12. Director.
  13. Log, view models, and checkpoint.
- Systems read anything but write only their own domain. Cross-domain changes go through typed effect requests.
- Domain events carry cause references. Together they form the causal graph that powers "Why?", news, memories, and Legacy.

## 5. World-generation design (DEC-021 to DEC-023)
- **Scale:** ~10 regions, ~60 municipalities, ~500 neighborhoods.
- **Pipeline:**
  1. Terrain.
  2. Hydrology.
  3. Climate.
  4. Biomes.
  5. Resources and hazards.
  6. Settlement sites.
  7. Settlement hierarchy.
  8. Transport.
  9. Regions.
  10. Neighborhoods and constituencies.
  11. Hub layouts.
  12. Seed population and economy.
  13. Founding eras.
  14. **15–25-year burn-in**: the real simulation, NPC-only.
  15. Draft candidates.
- **Standard:** plausible, not physical. Era volatility is a world parameter (DEC-059).

## 6. Population and economic models
- **Population** (DEC-018):
  - Cohorts are simulated everywhere at all times. They are cheap: about 6,000 cells.
  - Individuals are tiered:

    | Tier | Count |
    |---|---|
    | Principal | ~50–150 |
    | Named | Hundreds to thousands |
    | Ambient | Generated on demand, not stored |

  - Keyed RNG makes lazy evaluation order-independent.
- **Opinion** (DEC-025) has two channels:
  - anticipated, at proposal: values, perceived impact, trust;
  - experienced, after implementation and lagged: actual material change.
- **Economy** (DEC-019) is layered:
  - local acts feel pressure;
  - national acts gain macro levers;
  - the global economy arrives as exogenous shocks.

## 7. Government and political systems
- **Tiers and ladder** (DEC-014): Local, Regional, and National. Appointed posts are reached through Relationships. There is a "between offices" state, plus detours.
- **Constitutional primitives and archetypes** (DEC-015, DEC-017). Each archetype changes *who you must win over*.
- **Policy:**
  - Templates have 2–4 parameters, and bills bundle riders, earmarks, sunsets, and pilots (DEC-026).
  - Funding is a status, so authorization and appropriation stay distinct (DEC-024).
- **Elections** (DEC-028):
  - Per-cohort utility (issues, party ID, valence, retrospective judgment, local ties, awareness) feeds a logit model.
  - Turnout is modeled separately.
  - Seeded correlated shocks add uncertainty.
  - Tabulators are pluggable.
  - Every result includes a swing decomposition.
- **Polls are the only window** on the race (DEC-029).
- **Parties** (DEC-027) are stable, with dynamic factions. Splits and new parties happen rarely, when conditions build.

## 8. NPC and narrative architecture
- **Characters** (DEC-031) have:
  - personality: 5 axes plus tags;
  - values;
  - competencies;
  - directed relationships with a favor ledger;
  - bounded, quotable memory.
- **Goals emerge from circumstances.** Storylines are templates cast from real actors, with stages gated on live state (DEC-032).
- **The director** (DEC-033):
  - shapes *exogenous chance* only, never consequences;
  - follows a tension curve with one climax per chapter;
  - has budgets, cooldowns, and a crisis cap;
  - guarantees warnings before terminal endings.
  - Storyteller profiles come after the MVP.
- **Media** (DEC-034): facts → knowledge → coverage → perception. Outlets act on their own incentives. Scandals are discoverable facts.
- **Dialogue** (DEC-035): templated beats that map to actions, with visible odds and memory-filled slots. An LLM text layer is deferred and would be presentation-only.

## 9. Technical stack
- **Engine:** Rust (stable, pinned) compiled to `wasm32-unknown-unknown` with wasm-bindgen (adapter only), running in a Web Worker.
- **Frontend:** React and TypeScript (strict, DEC-045), Vite, Zustand (UI state only), PixiJS (provisional, spike-gated), Dexie/IndexedDB, vite-plugin-pwa.
- **Text:** Fluent.
- **Content:** JSONC with generated JSON Schemas (DEC-053).
- **Tests:** Vitest and Playwright on the web side; proptest, Criterion, and cargo-fuzz on the Rust side.
- **CI:** GitHub Actions.
- **Dependencies:** permissive licenses only; checked with `cargo-deny` and `cargo-audit`.

## 10. Rust workspace and crate architecture
There are 8 crates: `mandate-core`, `-content`, `-sim`, `-worldgen`, `-save`, `-protocol`, `-wasm`, and `-cli`. The dependency graph is acyclic (OVERVIEW §2, DEC-040). Domains are modules inside `mandate-sim`, with boundaries enforced by an architecture test.

## 11. Frontend architecture
- **Hub-centred screens** (DEC-047, UI.md).
- **Mobile-first responsive layout:** bottom sheets on phones, docked drawers on desktop (DEC-048).
- **Visual style:** top-down three-quarter pixel art with layered character sprites (DEC-049).
- **Accessibility:** an accessible DOM mirror of the hub, plus "Why?" breakdowns everywhere (DEC-051).
- A single typed worker-client and query-hook layer. The frontend mirrors no simulation state.

## 12. WASM and Worker protocol
- **Envelope and errors:** versioned envelope; typed errors.
- **Commands and queries:** commands are serial and logged; queries are read-only and return per-screen view models.
- **Progress and cancellation:** progress events for generation, burn-in, and time skips; cancellable at safe points.
- **Encoding and types:** JSON first. TS types are generated from Rust DTOs (DEC-043).

## 13. Data models and schemas
- **Entities and IDs:** `docs/architecture/DATA_MODEL.md` lists all authoritative entities, IDs, owners, and key fields.
- **Content schemas:** generated from the `mandate-content` Rust types.
- **Content types:** constitutions and quirks, policy templates, roles and backgrounds, events and storylines, dialogue and scene templates, sectors, cohort definitions, and tuning parameters (REQ-005).

## 14. Persistence and save strategy
- **Run save:** a versioned DTO snapshot plus the command log since that snapshot. Saves are compressed and integrity-hashed (DEC-044).
- **Save frequency:** Ironman saves at the end of every turn. Story mode also keeps chapter checkpoints.
- **Sync-ready from day 1** (DEC-052, TECH-009):
  - a `SaveStore` abstraction;
  - `run_id` and revision lineage;
  - a fork-on-divergence rule for Ironman;
  - a grow-only meta save.
- **Cloud providers come later:** Dropbox, OneDrive, or Google Drive via browser OAuth, with no backend.
- **Migrations:** an explicit migration chain, plus a fixture corpus in CI.

## 15. Testing and simulation lab
See `docs/development/TEST_STRATEGY.md` (DEC-056 to DEC-060).
- **Engineering tests:** unit, property, scenario, determinism (native = WASM; save → load → continue), replay regressions, save corpus, content validation, fuzzing, and performance.
- **Simulation lab:** `mandate-cli` lab commands, plus 5 canonical worlds and one fixture per archetype.
- **Nightly Monte Carlo:** stability bands on baseline runs only, plus **drama floors**.
- **Bot-player tournaments:** detect dominant strategies and check pacing.
- **Playtesting:** owner checkpoints at M3 and M4; owner plus 3–5 testers at M6; opt-in local run-report export.

## 16. Agentic development workflow
See `docs/development/WORKFLOW.md` and `AGENTS.md`:
- task brief and report templates;
- interface-first delegation;
- rules for parallel work;
- context-loading levels;
- independent review;
- drift rejection rules;
- the definition of done.

## 17. Milestones and acceptance criteria
See `docs/development/ROADMAP.md` §2 (DEC-063, systems first with checkpoints):

| Milestone | Content |
|---|---|
| M0 | Foundation |
| M1 | World and lab (M1s: rendering spikes) |
| M2 | Society, economy, environment |
| M3 | Local government and policy **(owner checkpoint)** |
| M4 | Elections and career **(owner checkpoint)** |
| M5 | Characters and narrative |
| M6 | **Vertical slice gate** |
| M7 | Act 2 |
| M8 | Acts 3–4 |
| M9 | Variety and polish |

## 18. Initial implementation backlog
See `docs/development/ROADMAP.md` §3: M0-01 to M0-13, with dependencies and acceptance criteria. The M1 outline is in §4.

## 19. Risks and deferred decisions

**Top risks:**

| Risk | Mitigation |
|---|---|
| Not fun (validated later, by owner choice DEC-063) | M3/M4 checkpoints, bot players, data-driven tuning, M6 gate |
| Scope (hubs, scenes, art are the largest addition) | MVP limited to Act 1; placeholder art; scene templates |
| Art pipeline (agents are weak at art) | Owner to choose an art source before M6 |
| Coupled-system balance | Lab, Monte Carlo bands, drama floors, balance log |
| Cross-platform determinism | Lint rules, `libm`, native = WASM CI parity |
| Burn-in performance on mobile | Budget under 10 s (DEC-046); measure in M2 and adjust burn-in length or cadence |
| Content volume | Templates parameterized by state; depth over breadth |
| Agent drift | WORKFLOW rules, independent review, DEC discipline |

**Deferred (non-blocking):**
- cloud sync providers;
- mod loader UI;
- storyteller profiles;
- archetypes 2–4 and quirks;
- legacy worlds;
- LLM text layer;
- multithreaded WASM;
- desktop wrapper;
- binary protocol and save encoding;
- chart library;
- localization coverage;
- international relations;
- licensing and monetization;
- hosting choice.

---

## Architecture review (brief §26.1)

| Question | Answer |
|---|---|
| Are subsystem responsibilities clear? | Yes. See §3 and DATA_MODEL ownership |
| Are dependencies acyclic? | Yes. The crate graph is a DAG (OVERVIEW §2). Module boundaries are enforced by an architecture test |
| Is state ownership explicit? | Yes. Each system writes only its own domain; cross-domain changes go through effect requests |
| Can systems be tested independently? | Yes. Systems are pure functions over state plus effect queues; there are scenario fixtures per system |
| Are update ordering and RNG ownership defined? | Yes. 13 phases; named streams per subsystem plus keyed access |
| Can the engine run natively and in WASM? | Yes. Engine crates have no browser dependencies, and CI checks parity |
| Can saves survive version changes? | Yes. DTOs are decoupled from internal structs; there is a migration chain and a fixture corpus |
| Can content be validated independently? | Yes. `mandate-cli content validate` against generated schemas, in CI |
| Is the UI decoupled from simulation logic? | Yes. The UI sends commands and queries and renders view models |
| Is the MVP achievable? | Yes, if scoped to Act 1 with placeholder art. The largest unknowns are hub rendering performance (M1s) and burn-in time (M2), and both are measured early |
| Any premature abstractions? | Watch the constitution primitives: build only the primitives the MVP archetype needs, while keeping the composable shape. Watch NPC LOD tiers: start with Principal and Named only, and add Ambient generation with the hub |
| Any critical systems missing? | Mostly covered. Two gaps were closed in this plan: the **tutorial and onboarding** flow and a **settings/options** surface, now added to the M6 scope. **Audio** is unplanned; recommend deferring to M9 with placeholder sound effects |
