# Decision Register

Statuses: Proposed · Accepted · Rejected · Deferred · Superseded.
Accepted decisions are not changed silently. Revisions get a new entry that supersedes the old one.

## Index

| ID | Topic | Status |
|---|---|---|
| DEC-001 | Project name | Accepted |
| DEC-002 | Baseline non-negotiable constraints (brief §31) | Accepted |
| DEC-003 | Discovery-before-implementation process | Accepted |
| DEC-004 | Run shape & career structure: career in acts | Accepted |
| DEC-005 | Nested time structure & target lengths | Accepted |
| DEC-006 | Endings, victory & run termination (rev. 2) | Accepted |
| DEC-007 | Constitution variability per run | Accepted |
| DEC-008 | Save model: ironman vs. reloadable | Accepted |
| DEC-009 | Presence model: embodied scenes + strategic layer | Proposed (awaiting owner, D2) |

---

### DEC-001 — Project name

- **Status:** Accepted
- **Context:** The project needs a name for the repository, the docs, and the product.
- **Decision:** The game is called **Mandate**.
- **Consequences:** Used in docs and package names. Rust crates may use a `mandate-` prefix (confirmed in D11).
- **Revisit criteria:** Trademark or branding concerns before release.

### DEC-002 — Baseline non-negotiable constraints

- **Status:** Accepted (owner-supplied brief, §31)
- **Context:** The owner set foundational constraints in the project brief.
- **Decision:** Adopt §31 items 1–20 as the baseline. They are captured as requirements in REQUIREMENTS.md (TECH-001–TECH-008, SIM-001–SIM-004, REQ-001–REQ-004, PROC-001–PROC-003).
- **Consequences:** Every later decision must stay compatible with these, or explicitly propose a revision.
- **Revisit criteria:** Owner instruction only.

### DEC-003 — Discovery before implementation

- **Status:** Accepted (brief §2.2, §30)
- **Decision:** No substantive implementation until discovery topics D1–D15 (DISCOVERY.md §3) are resolved or explicitly deferred, the architecture review is done, and the owner has approved the baseline. Throwaway prototypes used to answer a design question are allowed if the owner approves each one.
- **Consequences:** Early output is documentation only.

### DEC-004 — Run shape & career structure

- **Status:** Accepted (owner, 2026-11-18): option B. Target lengths moved to DEC-005, informed by `research/RUN_PACING.md`. The victory condition was reopened by the owner and moved to DEC-006.
- **Context:** A literal career from local office to the presidency is 20–40 in-game years and 6–10 elections. Roguelike replayability implies runs measured in hours. The answer drives turn length, simulation level of detail, content volume per stage, NPC persistence, and what failure means.
- **Options:**
  - A. A full continuous career at a uniform cadence.
  - B. A career in acts, with detailed play at key stretches and summarized time skips between them.
  - C. Single-stage runs with meta-progression.
- **Decision:** B. Four acts (Local → Regional/State → National → Presidency). Election losses are survivable setbacks rather than permadeath. The run ends at retirement, when no viable path remains, or at the final ending. *(Run lengths are in DEC-005; the original 8–12 h target is superseded there. The ending is in DEC-006.)*
- **Rationale:** Preserves the continuous climb. Avoids mid-career tedium. Gives each act distinct gameplay. Gives the engine natural level-of-detail boundaries. Makes persistent cross-act NPCs possible.
- **Consequences:** Requires a "term in review" fast-forward system, an action set for each act, level-of-detail transitions designed into the data model, and per-act content.
- **Dependencies:** D2, D3, D4, D5, D9.
- **Revisit criteria:** Playtesting shows acts feel disconnected, or run length is badly off target.

### DEC-005 — Nested time structure & target lengths

- **Status:** Accepted (owner, 2026-11-18; no objection raised)
- **Context:** Research (`research/RUN_PACING.md`) shows that retry motivation depends on how much a failure costs, and that session-sized resolution points matter. A career of 8–12 hours is long by roguelike standards.
- **Options:**
  - A. One long run of 8–12 h with no inner structure.
  - B. Nested units: turn → chapter → act → career.
  - C. Shrink the career to 3–5 h.
- **Proposed decision:** B, with these targets:

  | Unit | Real-time target | Content | Ends with |
  |---|---|---|---|
  | Turn | 1–3 min | One time step (length varies by act; set in D3) | Developments summary |
  | Chapter | 25–45 min | One campaign, or one legislative session / budget year in office | A resolution: election result, session record, budget |
  | Act | 1.5–2.5 h | 2–4 chapters at one tier of office | Act epilogue and transition (time skip, term review) |
  | Career (full success) | 6–10 h | 4 acts | Final ending (DEC-006) |

  Expected typical run: 2–5 h, because most careers end in Acts 1–3. Additions:
  - Save anywhere.
  - An autosave at every chapter boundary.
  - Optionally, "late start" scenarios that begin at Act 2 or 3 with a generated backstory (Into the Breach-style flexibility). These also serve as test fixtures.
- **Rationale:** Chapters give a resolution roughly every session (fits the ~30–40 min mobile strategy sessions). Acts give roguelike-sized arcs. A failure costs at most an act and always produces an ending.
- **Consequences:**
  - The narrative director must guarantee one climax per chapter.
  - The time-skip system must summarize skipped terms well.
  - Each act needs a distinct action set.
  - Late starts require a generated career history (which helps NPC persistence).
- **Dependencies:** DEC-004, D3, D9.
- **Revisit criteria:** Prototype playtests miss chapter or act timing by more than 50%.

### DEC-006 — Endings, victory & run termination (revision 2)

- **Status:** Accepted (owner, 2026-11-18). Owner addendum: terminal endings must happen to the player's character on screen (staged scenes in the embodied layer, DEC-009), not be reported as a news brief.
- **History:** Revision 1 (layered endings with "no bare game over") drew an owner concern: a roguelike needs real, final failure. This revision adds terminal endings.
- **Options:**
  - A. Single win: hold the legislature at the midterms.
  - B. Legacy score only.
  - C. Agenda/pledge completion.
  - D1. Layered, with soft endings only (revision 1).
  - D2. Layered, with hard terminal endings driven by visible pressure tracks (this revision).
- **Proposed decision:** D2.
  - **Summit:** the Mandate Test, the first national electoral verdict on the presidency, as scheduled by the constitution. Passing it is "Mandate renewed". Failing it ends the run.
  - **Terminal endings (the run is over):**
    - Death: assassination, or health.
    - Long or life imprisonment, or legal disqualification from office (e.g. impeachment with a ban).
    - Career-ending disgrace.
    - Political wilderness: Viability exhausted by a landslide wipeout or repeated defeats.
    - Time: age or the career clock runs out.
    - Voluntary retirement.
  - **Setbacks (survivable):**
    - An election loss with Viability remaining. It costs years, because the player ages.
    - A short prison sentence: out of politics for N years, reputation damage, a possible comeback arc.
    - A scandal that is survived.
    - Party expulsion: go independent or switch parties, if the constitution allows it.
  - **Pressure tracks** act as the roguelike "hit points". Each terminal ending is the end state of a visible track. Candidate tracks:
    - **Viability:** electability and party willingness to back you.
    - **Legal Jeopardy:** investigations, evidence, prosecutor and court state.
    - **Threat:** personal security risk, driven by polarization, enemies, and exposure.
    - **Standing:** scandal exposure and public trust.
    - **Age / career clock**, plus health.
  - **Fairness rules:**
    - No terminal ending without (1) a visible, elevated track, (2) at least one prior warning with a chance to respond, and (3) a causal explanation afterwards.
    - No random "bolt from the blue" deaths by default. An optional hard modifier may relax this later.
  - **Legacy epilogue:** still shown at every ending, as the run-summary screen. It grades the career and feeds meta-progression. It reports a failure; it does not soften it.
- **Rationale:** Real stakes and final loss (roguelike identity). Failures are fair and readable (FTL-style visible hull). Real careers survive single defeats, so that alone is not terminal, but a landslide or repeated losses are. The aging clock adds mounting pressure, as the storm does in Against the Storm.
- **Consequences:**
  - The player character needs age and health.
  - Justice-system and security models are needed (at least as abstracted tracks).
  - The narrative director must escalate tracks with warning events.
  - Balance targets are needed for how often each ending occurs (simulation lab).
- **Dependencies:** DEC-005, DEC-007, DEC-008, D2, D4, D8, D9, D10.
- **Revisit criteria:** Playtests show terminal endings feel unfair or arbitrary, or so rare that tension is absent.

### DEC-007 — Constitution variability per run

- **Status:** Accepted (owner, 2026-11-18): option D, staged as below.
- **Context:** The owner suggested procedurally generating the constitution each run, so that political and legal mechanics differ between runs, provided it stays reviewable at a high level.
- **Options:**
  - A. One fixed constitution.
  - B. Independent parameters, each generated separately.
  - C. Fully procedural institutions.
  - D. Authored constitutional archetypes plus a few generated quirks, shown on a one-screen "Constitution card".
- **Proposed decision:** D, delivered in stages:
  1. The engine treats the constitution as data from day 1. Institutional rules (offices, terms, electoral systems, veto and confidence rules, which tier holds which powers) are read from a `Constitution` definition, never hardcoded.
  2. The MVP ships one archetype with no quirks.
  3. Later: 3–4 archetypes, each with 0–2 quirks drawn from a curated, compatibility-checked pool. Additional archetypes are a candidate meta-progression unlock.
  4. Rule differences are revealed progressively. Act 1 is mostly municipal and varies little; national rules become relevant as the player rises.
- **Rejected:** C (incoherent combinations, untestable, unlearnable). B (combinatorial explosion, implausible combinations).
- **Consequences:**
  - D4 must define the constitutional parameter space up front.
  - Event and policy content needs institutional preconditions.
  - Opponent AI must reason about rules generically.
  - Each archetype becomes a canonical test configuration.
  - Quirk combinations are validated by Monte Carlo runs in the simulation lab.
- **Dependencies:** D4, D8, D11, D13, D14, DEC-006.
- **Revisit criteria:** The cost of supporting a second archetype exceeds its gameplay value in prototype evaluation.

### DEC-008 — Save model: ironman vs. reloadable

- **Status:** Accepted (owner, 2026-11-18): option B.
- **Context:** Final failure (DEC-006) is meaningless if the player can reload. The brief (§16.1) asks for manual saves and multiple slots, which conflicts with a roguelike structure.
- **Options:**
  - A. Ironman only: one continuous save per run, save-and-quit.
  - B. Ironman by default, plus an opt-in "Story mode" that allows reloading chapter checkpoints, with runs flagged as such.
  - C. Free saving.
- **Proposed decision:** B.
  - Multiple slots means multiple concurrent runs, not multiple saves of the same run.
  - Determinism helps here. Reloading and repeating the same actions reproduces the same outcome (persisted RNG streams), so reloading cannot fish for a different dice roll.
  - Developer and simulation-lab builds keep arbitrary snapshots.
- **Consequences:** Revises §16.1 (manual saves become save-and-quit). Story-mode runs may be excluded from some unlocks.
- **Dependencies:** DEC-006, D13.

### DEC-009 — Presence model: embodied scenes + strategic layer

- **Status:** Proposed. Awaiting owner (opens D2).
- **Context:** The owner wants the player to walk around an actual screen of their town, regional capital, or national capital. Dramatic outcomes (arrest, assassination) must happen to the on-screen character. The game must still offer high-level policy reading, decisions, and impacts. This is the brief's §5.5 question ("explorable map or strategic visualization?"), now answered in favor of embodiment.
- **Options:**
  - A. Free-roam open world at all scales (Stardew-style), with walking time as a cost.
  - B. Walkable hubs per act, plus a strategic layer, with located actions and staged scenes.
  - C. Illustrated location vignettes without free movement (Citizen Sleeper / Darkest Dungeon hamlet).
  - D. Dashboards plus cutscenes only.
- **Proposed decision:** B.
  - **Hubs:** each act has a small walkable "stage" (~10–20 points of interest), generated from the world's settlement data.
    - Act 1: home town district.
    - Act 2: regional capital.
    - Acts 3–4: national capital core.
    - Any other settlement on the map can be *visited* (campaign trips, crises), using the same generator.
  - **Movement:** tap or click to move (mobile-friendly), with optional keyboard movement on desktop. Walking is free *within* a turn; actions cost time slots (time budget set in D3).
  - **Located actions:** actions happen at places and with people (council chamber, diner, union hall, press room). An agenda or quick menu can auto-route the character there, so walking is never a chore, but the scene always plays.
  - **Strategic layer through in-world portals:** the office desk is the policy/budget desk, the TV or newspaper is news, the map room is the national map. Each is also reachable from a global menu.
  - **The street as a living readout:** ambient townsfolk are representative samples of the simulated cohorts. Their remarks, visible conditions (boarded storefronts, construction, protests, flood damage), and crowds are derived from simulation state. Walking rewards observation (rumors, overheard concerns, noticed problems) but is never mandatory for critical information (accessibility, §15.5).
  - **Faces on statistics:** policy and economic reports pair aggregate figures with named NPCs who are affected. Hybrid-population NPCs are the bridge between layers.
  - **Staged scenes:** key moments (debates, election night, bill signing, arrest, assassination attempt, protest) are data-driven scene templates choreographed in the hub, using the actual characters and location.
    - Decision points inside a scene are engine commands. Outcomes are always determined by the simulation. There are no reflex or skill mini-games.
- **Rationale:** Gives the owner's embodied feel without building an open-world game. Keeps navigation fast for strategic players. Makes consequences visible and personal. Reuses one settlement generator across all scales.
- **Consequences:**
  - Requires a 2D scene renderer (PixiJS likely; prototype in D12).
  - Requires layered procedural character sprites, building and interior templates, and a scene-template system.
  - The simulation must model places (buildings as entities) and named-NPC schedules or locations, deterministically.
  - Player position and walking are UI state, not simulation state.
  - Raises art-pipeline and scope risk. This is the largest scope addition so far.
  - MVP: one town hub, an office, a council chamber, and a few scene templates.
- **Dependencies:** D2, D3 (time slots), D6 (settlement layout generation), D9 (NPC schedules, scene templates), D11, D12 (camera, art style, renderer), D13.
- **Revisit criteria:** The prototype shows walking adds friction without insight, or art cost threatens the vertical slice.
