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
| DEC-009 | Presence model: embodied scenes + strategic layer | Accepted (renderer revised by DEC-080; located actions brought forward by DEC-079) |
| DEC-010 | Player role: politician first, administration mediated | Accepted |
| DEC-011 | Player power & resource model | Accepted |
| DEC-012 | Time model: discrete turns, phase-based cadence, time skips | Accepted |
| DEC-013 | Action economy: representative day, slots, delegation, commitments | Accepted |
| DEC-014 | Government tiers & office ladder | Accepted |
| DEC-015 | Constitution parameter space & archetypes | Accepted |
| DEC-016 | Default / MVP constitutional archetype | Accepted |
| DEC-017 | Constitution primitives (engine model) | Accepted (confirmed with DEC-040–046; built in M3, DEC-070) |
| DEC-018 | Population model & level of detail | Accepted |
| DEC-019 | Economy fidelity | Accepted |
| DEC-020 | Career detours: non-office power roles | Accepted |
| DEC-021 | World scale & spatial representation | Accepted |
| DEC-022 | World history via burn-in simulation | Accepted |
| DEC-023 | Environment dynamics over a career | Accepted |
| DEC-024 | Policy model: pipeline, mechanisms, implementation (rev. 2: 5 lifecycle states) | Accepted |
| DEC-025 | Public response: anticipated vs. experienced | Accepted |
| DEC-026 | Bill composition & customization | Accepted |
| DEC-027 | Parties & factions: dynamism | Accepted |
| DEC-028 | Election model | Accepted |
| DEC-029 | Polling & imperfect information | Accepted |
| DEC-030 | NPC political AI: rule symmetry | Accepted |
| DEC-031 | Character model, relationships & memory | Accepted |
| DEC-032 | NPC goals & storylines | Accepted |
| DEC-033 | Narrative director: authority & pacing | Accepted |
| DEC-034 | Media & information model | Accepted |
| DEC-035 | Conversations & dialogue | Accepted |
| DEC-036 | Character creation: draft of 3 | Accepted |
| DEC-037 | Personal life scope | Accepted |
| DEC-038 | Difficulty & complexity layers | Accepted |
| DEC-039 | Meta-progression | Accepted |
| DEC-040 | Crate structure (8 crates) | Accepted |
| DEC-041 | Determinism rules | Accepted |
| DEC-042 | Engine: calendar, turn order, effect requests, event log | Accepted |
| DEC-043 | Worker protocol | Accepted |
| DEC-044 | Persistence (rev. 2: versioned JSON snapshot plus migrations) | Accepted (save timing revised by DEC-074) |
| DEC-045 | Frontend stack: React + TypeScript | Accepted |
| DEC-046 | Initial performance budgets | Accepted (assumptions to validate) |
| DEC-047 | Screen architecture: hub-centred | Accepted |
| DEC-048 | Responsive & mobile strategy | Accepted |
| DEC-049 | Visual style & camera: top-down pixel art | Accepted (art source: DEC-065, then DEC-084) |
| DEC-050 | Rendering prototype spikes | Accepted (each spike needs go-ahead) |
| DEC-051 | Accessibility & explainability UI | Accepted |
| DEC-052 | Save sync readiness (future cloud sync) | Accepted |
| DEC-053 | Content authoring format & pipeline (rev. 2: JSONC) | Accepted |
| DEC-054 | Text & localization pipeline | Accepted |
| DEC-055 | Modding scope | Accepted |
| DEC-056 | Engineering test layers | Accepted |
| DEC-057 | CI tiers | Accepted |
| DEC-058 | Simulation lab & canonical worlds | Accepted |
| DEC-059 | Monte Carlo health metrics (rev. 2: baseline-only bands plus drama floors) | Accepted |
| DEC-060 | Game-quality validation & playtesting | Accepted |
| DEC-061 | Tone: heightened realism | Accepted |
| DEC-062 | MVP scope (vertical slice) | Accepted |
| DEC-063 | Milestone order: real systems first, with early playable checkpoints | Accepted |
| DEC-064 | Hosting: GitHub Pages first, portable to a Cloudflare domain | Accepted |
| DEC-065 | Initial art: Kenney CC0 asset packs | Accepted (game art from M6 follows DEC-084) |
| DEC-066 | CI: build once, cache WASM by source hash | Accepted |
| DEC-067 | System plan approved; implementation begins | Accepted |
| DEC-068 | World generation implementation choices (M1) | Accepted |
| DEC-069 | Society and economy implementation choices (M2) | Accepted |
| DEC-070 | Local government and policy implementation choices (M3) | Accepted (command journal revised by DEC-074) |
| DEC-071 | Research-informed model refinements | Accepted |
| DEC-072 | Elections, parties, NPC AI implementation (M4) | Accepted |
| DEC-073 | Every decision is a trade-off | Accepted |
| DEC-074 | Saves across app updates | Accepted |
| DEC-075 | Extensibility: registries over hard-coded dimensions | Accepted |
| DEC-076 | Difficulty target: rare wins, rich stories | Accepted |
| DEC-077 | Run-to-run variety: town character, varied openings, situational policies, variety check | Accepted |
| DEC-078 | Data-driven tuning and further registries | Accepted |
| DEC-079 | Places as the interface: go somewhere to do something, with convenience | Accepted |
| DEC-080 | Hub renderer: Canvas 2D (revises the provisional PixiJS choice) | Accepted |
| DEC-081 | Logo (poll-line M) and loading animation (career ladder) | Accepted |
| DEC-082 | Save export and import (`.mandate` files; light Ironman rule) | Accepted |
| DEC-083 | Landing page at the site root; the game moves to `play/` | Accepted |
| DEC-084 | Pixel-art tool: hand-drawn parts composed by code; spike before M6 art | Accepted |
| DEC-085 | Town layout on the client, versioned so a town never rearranges | Accepted |
| DEC-086 | Town map overview replaces the place list as the main town view | Accepted |
| DEC-087 | Foreign nations as low-detail simulated neighbours; foreign policy in M7–M8 | Accepted |
| DEC-088 | Enterable key places with data-driven interiors; walking by default, quick mode as an option | Accepted |
| DEC-089 | Map layers from a registry; some gated by where you have been | Accepted |
| DEC-090 | Town layout v2: terrain-first, data-driven districts, size from population, hidden places | Accepted |
| DEC-091 | Knowledge is earned: who and what you know comes from places and people | Accepted |
| DEC-092 | Policy families grounded in the town, with design choices and sponsors, all in content | Accepted |
| DEC-093 | Conversations: talk to people by topic; replies composed from grounded content lines | Accepted |
| DEC-094 | Credit and blame for laws; endorsements that move their followers' votes | Accepted |
| DEC-095 | Every character attribute changes behaviour; a lab check forbids decorative attributes | Accepted |
| DEC-096 | The power broker acts: lobbies, backs people, and helps or hinders the player | Accepted |
| DEC-097 | Engagement and variety: ground-game rebalance, asks and promises, storyteller variety, secrets and walking rewards, an engagement gate | Accepted |

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
- **Implemented in part (M5d part 1, 2026-11-19): Legal Jeopardy and prison.** `mandate_sim::justice`: a scandal may draw an investigation (prosecutors' interest by severity × evidence), which ends in charges or is dropped; charges go to trial (keyed odds from evidence, a lawyer, and public mood); a conviction brings a fine, removal from office, or prison. The track shows the case (55 while investigated, 80 once charged, plus evidence) and eases without one. A short sentence is a setback: out of office, a time skip (ending the turn serves it, the world goes on), then release with Standing and viability capped and the right to file again. A long sentence (5+ years) or a disqualification is a terminal ending, handed down only once `ending_allowed("legal_jeopardy")` holds (the warning, then the response window); otherwise the trial is adjourned with the same draws (tested). The run's status (`PlayerState.status`: active, imprisoned, ended) is shown plainly; the Legacy screen is M6. NPCs face the same path; a removed member's party fills the seat. Not yet: the other terminal endings and the Legacy epilogue.

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

- **Status:** Accepted (owner, 2026-11-18): option B. Later revisions: the hub renderer is Canvas 2D, not PixiJS (DEC-080); located actions came forward to M5 (DEC-079).
- **Owner addendum: persistent hubs.** Careers can move *down* the ladder (e.g. losing national office and running for governor or mayor). Returning sends the player back to the earlier hub, which must show what happened in the wider country while they were away. Consequences:
  - Hubs are persistent world places, rendered from live simulation state. They are never static snapshots.
  - When the player leaves a scope, its places and named NPCs are kept, and the area keeps simulating at reduced detail.
  - On return, detail is re-derived consistently with the aggregate history.
  - NPCs remember the player.
  - A "homecoming" scene summarizes what changed.
  - This is a hard requirement for D5 (level-of-detail transitions) and D9 (NPC persistence and memory).
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

### DEC-010 — Player role: politician first, administration mediated

- **Status:** Accepted (owner, 2026-11-18): option A.
- **Context:** Brief §3.1 and §4.4. Is the player a politician who persuades, an administrator who allocates, or both? This determines how direct policy control is at every level, and so the policy layer's design.
- **Options:**
  - A. Politician first: power flows through people at every level. Even in executive office, policy goes through legislatures, agencies, and appointees, as set by the constitution.
  - B. Hybrid by office: legislative acts are persuasion; executive offices grant direct, Democracy-style policy control.
  - C. Administrator first: a policy and budget management game with political events.
- **Proposed decision:** A. Roughly 70% politics and 30% administration, with the administrative share growing in executive offices. Administration is always mediated: you appoint, fund, direct, and oversee, but you do not set outcome sliders.
  - The core fantasy: *you can only do what you can get others to do, and what the rules let you attempt.*
- **Rationale:** Differentiates Mandate from Democracy. Makes NPCs and relationships mechanically central, which pays off DEC-009. Makes constitution variety (DEC-007) matter. Makes the policy implementation gap (§8.4) a natural source of play.
- **Consequences:**
  - The policy layer is a pipeline of proposals, coalitions, votes, funding, and implementation, not a settings panel.
  - Executive acts need delegation and oversight mechanics.
  - Opponent and ally AI quality becomes critical.
- **Dependencies:** DEC-007, DEC-009, D3, D4, D7, D9.

### DEC-011 — Player power & resource model

- **Status:** Accepted (owner, 2026-11-18), including the merge of legitimacy and political capital into Mandate
- **Context:** Brief §4.2 and §4.4 list authority, influence, resources, legitimacy, and political capital, and warn against them becoming interchangeable currencies.
- **Proposed decision:** Four distinct kinds of power, plus the risk side (DEC-006 pressure tracks).

  | Kind | What it is | Form | Gained by | Used for |
  |---|---|---|---|---|
  | **Authority** | What your office legally permits | Rule set from constitution data, not a currency | Holding office | Gates which actions exist |
  | **Mandate** | Public legitimacy behind you; merges the brief's "legitimacy" and "political capital" | One global meter that decays over a term | Winning (by margin), clear pledges, kept pledges, visible successes | Pushing contested actions, holding allies, overriding resistance |
  | **Relationships** | Influence, kept as a per-NPC ledger | Directional trust, favors owed, grudges, memory | Helping, trading, keeping your word | Persuasion, votes, endorsements, information |
  | **Resources** | Time slots, money (personal finances and campaign funds, kept separate from the government budget you control), staff capacity | Budgets | Fundraising, salary, staff hires | Paying action costs |

  - **Pledges:** campaign promises are tracked as measurable objectives. Winning on clear pledges creates Mandate. Breaking them drains it. Kept pledges feed the Legacy score.
- **Rationale:** Each kind of power answers a different question (may I, will the public back me, will *they* help me, can I afford it). Kinds do not convert one-to-one, though actions can trade between them with friction. Using the name "Mandate" ties the main meter to the game's title.
- **Deviation from the brief:** "legitimacy" and "political capital" are merged into Mandate to reduce cognitive load. Elite acceptance is carried by Relationships.
- **Dependencies:** DEC-006, DEC-010, D3 (action economy), D4, D8, D9.

### DEC-012 — Time model: discrete turns, phase-based cadence, time skips

- **Status:** Accepted (owner, 2026-11-18): option C.
- **Context:** DEC-005 sets real-time targets (turn 1–3 min, chapter 25–45 min). This decision sets how much calendar time a turn represents, and how skipped time works.
- **Options:**
  - A. Real-time with pause (RimWorld / Paradox).
  - B. Discrete player-ended turns, fixed length throughout.
  - C. Discrete turns whose calendar length depends on the phase, plus summarized time skips.
- **Proposed decision:** C.
  - **Discrete, player-ended turns.** Nothing happens while the player thinks. This suits mobile, determinism, and deliberate roguelike decisions.
  - **Turn length by phase** (numbers are assumptions, to be tuned in the prototype):

    | Phase | Turn spans | Notes |
    |---|---|---|
    | Campaign | 1 week | Final stretch: the last ~3 turns before election day become daily (tension ramp) |
    | Governing (local/regional) | 2 weeks | A budget year or legislative session is one chapter |
    | Governing (national/presidency) | 2–4 weeks | Crisis events can temporarily force shorter turns |

  - **Resolution:** WEGO. Player actions spend slots during the turn. Conversations and other immediate effects resolve at once. Scheduled actions resolve at end of turn, together with NPC and institutional actions, then the system cadences run (brief §12.1 table).
  - **Time skips** between chapters ("term in review"):
    - The player sets standing directives (focus issues, voting stance, staff priorities, how cautious to be).
    - The simulation fast-forwards with a player-proxy following those directives.
    - **Interrupts** pull the player back in when a pressure track crosses a threshold, or when a critical event or decision occurs.
    - A summary and the hub's visible changes show what happened.
  - **Career clock:** roughly 4–8 in-game years per act, plus years lost to defeats or prison. A career spans ~25–35 years; with a start age of ~30–40, the player is ~55–70 at the presidency. This feeds the Age track (DEC-006).
- **Rationale:** Compression where nothing is happening, detail where drama is. A tension ramp before elections. Interrupts keep skips from hiding run-ending events, which honors the DEC-006 fairness rules.
- **Consequences:**
  - The engine scheduler must support a variable turn length, with cadences expressed in calendar time, not turns.
  - A player-proxy AI is required for skips (it can share logic with NPC politician AI).
  - The narrative director must budget events per chapter, not per turn.
- **Dependencies:** DEC-005, DEC-006, DEC-013, D5, D9, D11.

### DEC-013 — Action economy: representative day, slots, delegation, commitments

- **Status:** Accepted (owner, 2026-11-18): option C, with two notes:
  - **Framing risk.** The owner is concerned that "one day" stands for a week or more. UX mitigation (resolve in D12): the turn header shows the calendar span (e.g. "Week of 3 March"), and slots are presented as agenda blocks for that span ("this week's mornings"). The hub keeps day-part lighting as a stylization.
  - **Slot count is a tuning parameter** (REQ-005). Three standard slots is a starting value. Levers: base slots, Late Night, delegated staff slots, and slot cost per action.
- **Context:** The brief (§4.2) asks for limited actions per turn, without tedium. DEC-009 requires actions to happen at places in the hub. DEC-011 lists time as a resource.
- **Options:**
  - A. Abstract action points per turn.
  - B. Full calendar scheduling (week grid).
  - C. A "representative day": each turn plays in the hub as one stylized day, with day-part slots, plus delegation and multi-turn commitments.
- **Proposed decision:** C.
  - **Slots:** Morning, Afternoon, Evening (standard), plus an optional **Late Night** slot. Late Night costs Health and raises the risk of some actions (private deals, off-record meetings), which suits backroom politics.
  - Typical actions cost 1 slot; major ones cost 2. The hub's lighting and its NPC schedules follow the day-part: different people are at different places at different times.
  - **Delegation:** staff provide extra delegated slots for suitable actions (canvassing, research, routine casework), with reduced effectiveness and their own risks (a staffer's mistake, or loyalty). Staff capacity is a Resource (DEC-011).
  - **Commitments:** multi-turn efforts (drafting a bill, building a coalition, running an investigation) reserve slots in future turns until complete or abandoned.
  - Slots per turn stay roughly constant across acts. What changes is what each slot can accomplish and which actions are available (Authority).
- **Rationale:** Integrates time pressure with the walkable hub. A small, stable number of decisions per turn fits the 1–3 min turn target. Delegation scales with office without inflating micromanagement. Late Night gives a natural, risky source of extra effort.
- **Consequences:**
  - The engine needs an NPC schedule or availability model by day-part.
  - Action definitions need slot cost, location, eligible participants, whether the action can be delegated, and duration (feeds the D7 and D9 content schemas).
- **Dependencies:** DEC-009, DEC-011, DEC-012, D9, D12.

### DEC-014 — Government tiers & office ladder

- **Status:** Accepted (owner, 2026-11-18).
- **Context:** Brief §3.3 lists 8 levels. DEC-004 has 4 acts. Offices, and how one reaches them, must come from constitution data (DEC-007).
- **Proposed decision:**
  - **Three tiers of government:** Local (municipality, divided into wards), Regional (state or province), and National. County and district governments are dropped for scope; they could return later as a constitutional quirk.
  - **Acts follow the scope of the office held or sought**, not a fixed sequence:

    | Act | Tier | Typical offices (actual offices come from the constitution) |
    |---|---|---|
    | 1 | Local | Ward councillor → mayor (or council leader) |
    | 2 | Regional | Regional legislator → regional executive (governor or premier) |
    | 3 | National | National legislator, minister, party leadership |
    | 4 | Head of government | President or prime minister (whichever the constitution provides) |

  - **Ladder rules:**
    - Skipping a tier is allowed but harder, e.g. a big-city mayor running directly for the national legislature with lower name recognition and Viability. This gives shorter, riskier runs.
    - Moving down the ladder is allowed (DEC-009 persistent hubs).
    - Appointed offices (minister, agency head) are reachable through Relationships rather than elections.
    - Losing office means a "between offices" state: private sector, party work, an appointment, or out of politics. Time passes, and Viability and Age apply.
  - "Presidency" in player-facing text becomes **"Head of Government"** wherever the archetype is parliamentary.
- **Dependencies:** DEC-004, DEC-006, DEC-007, DEC-015, D8.

### DEC-015 — Constitution parameter space & archetypes

- **Status:** Accepted (owner, 2026-11-18). The owner asked for unusual real or proposed systems as curveballs; see `research/GOVERNANCE_CURVEBALLS.md`. The parameter table above is a starting set; DEC-017 generalizes it into primitives so curveballs are mostly configuration.
- **Context:** DEC-007 requires a defined parameter space that the engine reads, and authored archetypes made from it.
- **Proposed parameter space** (each parameter takes a small set of values that are each fully supported):

  | Parameter | Values |
  |---|---|
  | Executive model | Presidential · Parliamentary · Semi-presidential |
  | National legislature | Unicameral · Bicameral (upper house elected or regional) |
  | Electoral system (per tier) | Single-member plurality · Two-round · Proportional list · Mixed-member |
  | Candidate selection | Open primaries · Party members · Party leadership |
  | Territorial structure | Unitary (weak regions) · Federal (strong regions) |
  | Election timing | Fixed terms · Staggered (midterms) · Flexible (snap elections) |
  | Terms & limits | Term length per office; limits none / 2 / 1 |
  | Executive checks | Veto plus override threshold · Confirmation of appointments · Decree power (none / limited) |
  | Removal | Impeachment · No-confidence vote · Recall (quirk) |
  | Judicial review | Weak · Strong |
  | Direct democracy | None · Referendums · Citizen initiatives (quirk) |
  | Local government form | Strong mayor · Council-manager · Council-elected leader |
  | Fiscal rules | None · Balanced-budget rule · Debt ceiling (quirk) |

- **Proposed archetypes:**
  1. **Federal Presidential Republic:** presidential, bicameral, plurality, primaries, staggered midterms, veto, strong regions.
  2. **Unitary Parliamentary (majoritarian):** PM from the legislature, plurality, party-selected candidates, flexible elections, no-confidence.
  3. **Parliamentary Proportional:** list PR, coalition governments, multiparty.
  4. **Semi-Presidential:** elected president plus PM, two-round elections, possible "cohabitation".
- **Gameplay impact:** archetypes change *who you must win over*:
  - In 1, the voters (primaries and elections).
  - In 2, the party and then the voters.
  - In 3, party list rank and coalition partners.
  - In 4, both voters and the legislature.
  This is the source of meaningful variation between runs.
- **Consequences:**
  - The election model (D8) must support plurality, two-round, and list PR from the start of its design, even if only one ships first.
  - Each archetype is a canonical test configuration (D14).
- **Dependencies:** DEC-007, DEC-014, DEC-016, D7, D8.

### DEC-016 — Default / MVP constitutional archetype

- **Status:** Accepted (owner, 2026-11-18). The MVP/demo uses the Federal Presidential Republic; the engine must handle variable constitutions from the start.
- **Options:** any archetype in DEC-015.
- **Proposed decision:** Archetype 1, **Federal Presidential Republic**, for the MVP and the first-run default. Candidate selection in the MVP is by local party nomination (a light phase inside the campaign chapter), not full primaries, to save scope.
- **Rationale:**
  - Matches the brief's original vision (presidency, midterms).
  - The player's personal campaign matters most when elections are candidate-centered.
  - Strong regions make Act 2 meaningful.
  - Midterms give a clean Mandate Test.
  - Parliamentary and PR archetypes emphasize party-internal play and coalitions, which need deeper AI first.
- **Dependencies:** DEC-006, DEC-015, D8, D15.

### DEC-017 — Constitution primitives (engine model)

- **Status:** Accepted. Proposed for confirmation in D11; confirmed as part of the technical baseline (DEC-040 to DEC-046, 2026-11-18). Implemented in M3 for the primitives the MVP archetype needs (DEC-070).
- **Context:** A fixed enum of parameters (DEC-015) cannot express curveballs such as collegial executives, sortition chambers, functional seats, or concurrent majorities. The engine needs composable building blocks so that archetypes and quirks are data.
- **Proposed decision:** a constitution is a validated assembly of these primitives:
  - **Office:** a single holder. Fields: selection method, term, limits, powers, removal procedures.
  - **Body:** a collective (legislature, council, assembly, court, directorate). Fields: seats, composition rules, selection method, term, internal leadership, decision rule, powers.
  - **SelectionMethod:**
    - `Election { voting_rule, constituency, candidate_selection }`
    - `Appointment { by, confirmation }`
    - `Sortition { pool, size }`
    - `Rotation { among, period }`
    - `ElectedByBody { body, rule }`
    - `ExOfficio`
  - **VotingRule:** plurality, two-round, ranked-choice (IRV), STV, approval, list PR (D'Hondt / Sainte-Laguë), mixed-member, quadratic (internal bodies only). Implemented as pluggable tabulators that share a ballot model.
  - **Constituency:** geographic district, region-at-large, national list, or functional (sector or occupation from the economy model).
  - **DecisionRule:** simple majority, supermajority(x), consensus, unanimity, concurrent majorities across blocs.
  - **Power:** a typed capability. Examples: propose_law, approve_law, veto, override, appoint(office), confirm, remove(office), propose_budget, approve_budget, decree, dissolve / call_election, call_referendum, judicial_review, initiative.
  - **Procedure:** an ordered pipeline of stages for laws, budgets, and constitutional amendments. Each stage references a Body or Office, a DecisionRule, and timing. Bicameralism, vetoes, overrides, referendums, and court review are all just stages.
  - **Trigger:** recalls, no-confidence votes, snap elections, sunset expiry.
  - **Validation (content CI):**
    - Every office is reachable.
    - Every procedure terminates.
    - Powers do not conflict.
    - The player has a viable path to the summit.
    - The Mandate Test event is defined.
- **Consequences:**
  - Election, policy, and AI systems must query powers and procedures rather than branch on archetype names.
  - Archetypes and quirks become content files (D13).
- **Dependencies:** DEC-007, DEC-015, D7, D8, D11, D13, D14.

### DEC-018 — Population model & level of detail

- **Status:** Accepted (owner, 2026-11-18).
- **Context:** H4 (hybrid population) and the DEC-009 persistent-hub requirement. Hubs keep simulating while the player is away and must be consistent when the player returns.
- **Key observation:** aggregate cohorts are computationally cheap. A country of ~10 regions, ~60 municipalities, and ~500 neighborhoods with 12 cohorts each is ~6,000 cohort cells, trivial to update monthly in Rust/WASM. The expensive things are *individuals*: NPC goals, schedules, AI, memory, and event generation.
- **Proposed decision:** **aggregate everywhere, individuals where it matters.**
  - **Cohorts** are simulated at neighborhood resolution across the whole country at all times (no aggregation level of detail). Each neighborhood has 12 cohorts: age band (18–29, 30–44, 45–64, 65+) × economic position (low, middle, high).
  - Other characteristics are distributions inside a cohort, not extra dimensions:
    - employment by sector,
    - housing tenure,
    - education,
    - household type.
  - Political state per cohort:
    - an attitude vector (~8 dimensions),
    - issue salience,
    - party identification distribution,
    - turnout propensity,
    - institutional trust,
    - awareness of and perception of officials.
  - **Individuals use tiered level of detail:**

    | Tier | Count | Simulation |
    |---|---|---|
    | Principal | ~50–150 active | Full: goals, schedules, AI decisions, rich memory |
    | Named | Hundreds to low thousands | Persistent identity, stats, relationships, summarized memory; updated lazily |
    | Ambient | Unbounded, not stored | Generated on demand from cohort samples (seed + place + date); promoted to Named if they become relevant |

    - Promotion and demotion are driven by relevance to the player and to storylines.
    - Dormant Named NPCs catch up on re-activation, using cohort transition history (ageing, job changes, moves).
  - **Determinism under lazy evaluation:** randomness for lazily evaluated entities is derived from keyed, counter-based RNG (seed, entity ID, period), not from sequential streams. Results are therefore identical regardless of *when* or *whether* they are evaluated. (Confirm in D11.)
- **Rationale:**
  - Returning to an old hub is automatically consistent, because its population never stopped being simulated.
  - Elections are computed bottom-up everywhere.
  - Cost is concentrated in characters, where it matters.
- **Consequences:**
  - The world-generation scale (D6) must stay within roughly these magnitudes.
  - Named-NPC catch-up rules are needed.
  - Ambient generation must be a pure function.
- **Dependencies:** DEC-009, DEC-019, D6, D8, D9, D11.

### DEC-019 — Economy fidelity

- **Status:** Accepted (owner, 2026-11-18): option B (layered).
- **Options:**
  - A. Light: sectors, jobs, wages, housing, and public budgets per municipality and region. National macro conditions are exogenous "weather" (scripted or random shocks).
  - B. Layered: A, plus an endogenous national macro loop (inflation, interest rates set by a central bank actor, national debt dynamics, simple inter-regional trade). The loop always runs, but the player can only influence it in Acts 3–4.
  - C. Deep: input-output matrices, firm-level agents, full markets.
- **Proposed decision:** B, with these components:
  - ~8 sectors: agriculture; resources (mining, forestry, energy); manufacturing; construction; retail and services; tourism and hospitality; knowledge and tech; public sector.
  - Municipal housing markets: supply, demand, rents, and prices. Housing is a primary local political issue.
  - Per-tier public finance with the brief's §7.3 elements.
  - Integer fixed-point money.
  - The global economy as exogenous shocks: commodity prices, external demand.
- **Rationale:**
  - Local acts feel real economic pressure (closures, housing, tax base) without needing macro control.
  - National acts gain real macro levers and tradeoffs.
  - Option C adds cost and opacity with little gameplay return. It conflicts with explainability (§1.3).
- **Dependencies:** DEC-010, DEC-018, D6, D7, D11.

### DEC-020 — Career detours: non-office power roles

- **Status:** Accepted (owner idea; agent design accepted without objection, 2026-11-18). Finalize the role list in D10.
- **Context:** DEC-014 had a passive "between offices" state. The owner wants losing to open interesting alternatives, such as becoming a union boss to win over a demographic or fix an earlier weakness.
- **Proposed decision:**
  - When out of office, the player can pursue a **detour role**: leadership of a real organization in the simulation. Candidates:
    - union leader,
    - business association head,
    - advocacy or NGO director,
    - media commentator or columnist,
    - party organizer or chair,
    - lobbyist,
    - think-tank or academic post,
    - appointed official (if offered).
  - **Detours are earned, not picked from a menu.** Organizations exist in the simulation with NPC leaders and internal selection (a union election, a board vote, an editor's hire). Taking the role means winning it, and the displaced leader may become a rival.
  - **A detour is a playable chapter** (DEC-005 sizing), set in its own hub location (union hall, newsroom, association offices), with a role-specific action set. Examples:
    - Organize a strike.
    - Run an exposé.
    - Endorse candidates.
    - Lobby legislators.
  - **Strategic purpose** (the roguelike "rest site / shop" analogue):
    - Rebuild Viability.
    - Shift your reputation with specific cohorts.
    - Gain Relationships.
    - Learn skills.
    - Accumulate organizational power that you keep using after returning to office (endorsements, volunteers, money, media access).
  - **Tradeoffs:**
    - Each role pleases some constituencies and alienates others (a union boss gains working-class trust but loses business standing).
    - Some roles carry Legal Jeopardy risk (lobbying, union funds).
    - Time and Age still pass.
  - **Power-broker path:** a player may choose to stay in a detour role and influence politics from outside. This could be a recognized Legacy ending, e.g. "Kingmaker".
  - **Reuse:** detour roles share content definitions with starting backgrounds (D10). A "former union organizer" background and a union-leader detour use the same role data.
- **Consequences:**
  - Organizations (unions, associations, outlets, NGOs) must be simulated actors with leadership, membership, resources, and internal selection.
  - Each role needs an action set and a hub location. MVP candidates: union leader, business association head, media commentator.
- **Dependencies:** DEC-005, DEC-006, DEC-009, DEC-011, DEC-014, D8, D9, D10.

### DEC-021 — World scale & spatial representation

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Proposed decision:**
  - **Scale:** one country of ~8–12 regions, ~40–80 municipalities, and ~400–600 neighborhoods.
    - Large cities: 10–20 neighborhoods.
    - Towns: 3–6.
    - Rural municipalities: 1–3.
    - This fits the DEC-018 cost envelope. Scale is a world-generation parameter; the MVP uses one medium size.
  - **Layered representation:**
    - **Terrain raster** (~512² cells): elevation, hydrology, climate, biome, resources, hazard maps. Used during generation and for environmental events.
    - **Spatial hierarchy graph:** neighborhood → municipality → region → nation, with adjacency. This is the simulation's primary structure.
    - **Transport graph:** roads, rail, ports, with travel costs.
    - **Political districts:** contiguous groupings of neighborhoods, generated to near-equal population per the constitution's constituency rules. Redistricting (and gerrymandering) is therefore possible as a policy or constitutional mechanic.
    - **Hub layouts:** walkable district scenes (DEC-009) generated deterministically from settlement data. Every point of interest maps to a simulation entity (the actual town hall, union local, or factory). A hub is a stylized representative district, not a full city.
  - **Generation pipeline** (brief §5.2, refined):
    1. Elevation (noise plus ridge shaping).
    2. Hydrology (flow accumulation → rivers, lakes).
    3. Climate (latitude, prevailing wind, rain shadow).
    4. Biomes (Whittaker-style).
    5. Resources and hazard maps (floodplains, drought, wildfire, seismic).
    6. Settlement-site scoring (water, flat land, coast, resources).
    7. Settlement hierarchy (rank-size).
    8. Transport (least-cost paths).
    9. Regions (grown from capitals with terrain costs, preferring rivers and ridgelines as borders).
    10. Neighborhoods and districts.
    11. Population and economy seeded from geography.
    12. History burn-in (DEC-022).
  - **Map role:** the national and regional maps are strategic visualizations with overlays (polls, economy, hazards, districts). Walking happens only in hubs.
- **Standard:** "plausible, not physical". Use believable geography from well-understood techniques, not climate or tectonic simulation.
- **Dependencies:** DEC-009, DEC-017, DEC-018, DEC-019, D11, D12.

### DEC-022 — World history via burn-in simulation

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Context:** A fresh world needs a past: incumbents, party strongholds, legacy industries, old grudges, prior policies.
- **Options:**
  - A. Rule-based authored backstory generation only.
  - B. Rule-based founding layers plus a burn-in, i.e. running the actual simulation for ~15–25 in-game years with NPC-only politics before the player starts.
- **Proposed decision:** B.
  - Coarse rule-based eras set the founding conditions: settlement age and early industries.
  - The burn-in then produces a consistent present: real election histories, incumbents with records, NPC relationships and memories, declining or booming towns.
  - The same mechanism generates late-start backstories (DEC-005).
- **Rationale:** the starting state is produced by the same rules that govern play, so it is internally consistent. Every generation also exercises system stability.
- **Consequences:**
  - New-game time must include the burn-in. Performance budget to be set in D11/D14; a target of under ~10 s on mid-range mobile is an assumption.
  - The NPC-only political AI must be good enough to run the country unattended. It is the same AI as the time-skip player-proxy (DEC-012).
- **Dependencies:** DEC-005, DEC-012, DEC-018, D9, D11, D14.

### DEC-023 — Environment dynamics over a career

- **Status:** Accepted (owner, 2026-11-18): option B.
- **Context:** A career spans ~25–35 in-game years (DEC-012), which is long enough for environmental change to become politics.
- **Options:**
  - A. Static geography: hazards are random events with fixed probabilities.
  - B. Slow dynamic environment: a small set of state variables per region drifts along a seeded trajectory and responds to policy. Variables: resource stocks (mines, forests, fisheries, aquifers), water availability, hazard intensities, pollution, land use.
  - C. Full environmental simulation (ecosystems, an emissions-to-climate model).
- **Proposed decision:** B.
  - Each world has a seeded environmental trajectory (stable, warming and drying, increasingly stormy, and so on).
  - Resources deplete with extraction.
  - Policy acts mainly through adaptation and management (levees, water rights, extraction limits, zoning, pollution rules). Within a single career these have visible effects. National mitigation policy affects the long-term trend weakly and realistically, and feeds Legacy.
- **Rationale:**
  - Makes geography an active political force over a career (the mine running dry, the worsening floods).
  - Produces visible hub changes on return (DEC-009).
  - Stays explainable and cheap.
  - Option C is costly and would dominate the game's tone.
- **Dependencies:** DEC-019, DEC-021, D7, D9.

### DEC-024 — Policy model: pipeline, mechanisms, implementation

- **Status:** Accepted (owner, 2026-11-18), with the lifecycle simplified at the owner's request (revision 2).
- **Context:** Brief §8. Policies are structured simulation objects, not approval modifiers. DEC-010: the player works through institutions. DEC-017: procedures come from the constitution.
- **Proposed decision:**
  - **Two kinds of policy lever:**
    - **Standing settings:** tax rates, fees, and budget allocations by category. Changed through the recurring **budget procedure**.
    - **Programs and laws:** discrete initiatives authored as **policy templates** in content.
  - **Lifecycle (revision 2, owner-simplified).** Five states, plus attributes. (Revision 1 had 9 states; the owner flagged redundancy.)

    | State | Meaning | Key attributes |
    |---|---|---|
    | **Draft** | Private; being shaped, with co-sponsors gathered | Sponsors, parameters |
    | **Under consideration** | Moving through the constitution's procedure. Negotiation and amendments happen here as *activity*, not as a separate state | Current procedure stage (e.g. "in committee", "passed lower house", "awaiting signature"), vote counts |
    | **Enacted** | Law on the books | **Funding status:** funded / partial / unfunded / not required |
    | **In effect** | Being carried out | **Implementation progress** 0–100% (ramps effectiveness), issue flags (delayed, court challenge, overrun) |
    | **Ended** | Repealed / expired / struck down / died | Reason |

    - "Review" is an *event* (a scheduled report or sunset vote), not a state.
    - **Funding is a status, not a stage.** Authorization and funding stay separate because unfunded laws are real politics: pass it for credit and then starve it, or opponents defund your law at the next budget. This is the brief's §7.3 requirement. Regulations need no appropriation.
  - **Effects as typed mechanisms.** Policies never write outcome variables directly. They apply mechanism primitives, and the economy, population, and environment models compute the consequences in context:

    | Mechanism | Example |
    |---|---|
    | Transfer | Benefit to a target cohort |
    | Tax change | Rate or base change |
    | Service provision | Transit frequency, school staffing, policing level |
    | Investment | Capital stock: housing units, roads, levees |
    | Subsidy / price | Sector cost relief, fees |
    | Regulation / mandate | Zoning density, wage floor, extraction limit, emissions rule |
    | Institutional | Create or reform an agency, ethics rules, procedures |

    Magnitudes depend on local conditions: sector mix, housing supply, agency capacity, geography. The same policy therefore performs differently in different places (brief §8.2).
  - **Implementation gap (brief §8.4):** each active policy has an implementing agency with capacity (staff, competence, leadership appointee) and funding adequacy. Implementation also depends on cooperation from other tiers, legal challenges (judicial review per the constitution), and public resistance. Consequences can include:
    - ramp-up delays,
    - partial effectiveness,
    - cost overruns,
    - procurement scandals. These raise Legal Jeopardy for whoever is responsible.
  - **Required template fields** (brief §2.5 example, now REQ-006):
    - affected cohorts, sectors, or places;
    - implementation duration;
    - fiscal impact;
    - at least one secondary effect;
    - institutional preconditions (constitution powers);
    - responsible agency type.
  - **Attribution for explanations:** mechanisms record their direct contributions each tick, which feed policy reports and the causal history. Full counterfactual re-simulation is a simulation-lab tool, not an in-game feature, unless D11 shows it is affordable.
  - **Player knowledge:** in-game reports present *analyst estimates* (noisy, and possibly biased by the source). The Legacy score uses true simulation values.
- **Dependencies:** DEC-010, DEC-017, DEC-018, DEC-019, DEC-023, D8, D9, D11, D13.

### DEC-025 — Public response: anticipated vs. experienced

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Proposed decision:** cohort opinion reacts to policy through two separate channels:
  1. **Anticipated:** at proposal and passage. Driven by value alignment (the policy's ideological signature against the cohort's attitude vector), *perceived* personal impact as shaped by messaging and media, salience, and trust in the messenger.
  2. **Experienced:** after implementation, lagged. Driven by *actual* computed material changes (income, prices, rents, service quality, hazard damage) that the cohort can notice, weighted by visibility.
- **Gameplay consequence:** the core timing tension of political careers is built in.
  - Some policies are popular when proposed but resented when their costs bite.
  - Others are unpopular up front but pay off after the next election.
  - Election calendars therefore interact with implementation lags.
- **Dependencies:** DEC-018, DEC-024, D8 (elections), D9 (media and perception).

### DEC-026 — Bill composition & customization

- **Status:** Accepted (owner, 2026-11-18): option B.
- **Options:**
  - A. A fixed catalog: enact or repeal, with one intensity slider (Democracy-style).
  - B. Templates with 2–4 tunable parameters (amount, target group, timeline, funding source). A bill can bundle clauses during negotiation (riders, earmarks or local projects, sunset clauses, pilot-program scoping).
  - C. A free-form bill composer built from mechanism primitives.
- **Proposed decision:** B.
  - Parameters and bundling are the currency of negotiation. NPC legislators ask for changes or local projects in exchange for votes, which is logrolling.
  - "Pilot program" scoping (limit a policy to one area first) lets the player test a policy before committing.
- **Rationale:**
  - Option A leaves nothing to negotiate, which undermines DEC-010.
  - Option C overwhelms casual players, is hard to balance, and is hard for AI to evaluate.
  - Option B makes legislation a political game while keeping content authorable.
- **Dependencies:** DEC-010, DEC-024, D8, D9 (NPC preferences), D12.

### DEC-027 — Parties & factions: dynamism

- **Status:** Accepted (owner, 2026-11-18): option B.
- **Context:** Brief §9.1. Careers span 25–35 years, long enough for realignments. The number of parties should follow the electoral system: plurality voting tends toward two major parties, proportional voting toward several.
- **Model (common to all options):**
  - **Factions are the fundamental unit.** Each has an ideological position (attitude vector), a leader NPC, member politicians, and a regional and cohort base.
  - **A party is a coalition of factions.** It has leadership, a platform (a weighted blend of its factions), a brand (reputation per cohort), organization (money, volunteers, branches per region), and cohesion.
  - Parties are generated at world creation from the society's main cleavages (attitude dimensions and cohort coalitions). The number of parties fits the constitution's electoral system, then is shaped by the burn-in (DEC-022).
- **Options:**
  - A. Fixed party roster and fixed factions.
  - B. Stable parties with dynamic factions. Factions rise and fall within parties, and politicians (including the player) can switch parties.
  - C. A fully dynamic party system. Factions can split off, parties can merge or collapse, and new parties can emerge, including one founded by the player.
- **Proposed decision:** B as the baseline, with C-type events possible but rare and conditional. Splits, insurgent parties, and mergers become likely only when their conditions build up: cohesion collapse, a large unrepresented cohort, or a permissive electoral system (more likely under PR, rare under plurality).
  - Founding a party is a late-career, high-risk path.
  - Factions-as-units makes this the same architecture as C, just tuned.
- **Rationale:** Party brands stay legible for most of a run. Realignments happen as earned, dramatic events, not noise. Supports DEC-020 power-broker play and DEC-015 archetype differences.
- **Dependencies:** DEC-015, DEC-017, DEC-018, DEC-022, DEC-028, D9.

### DEC-028 — Election model

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Proposed decision:** a bottom-up, deterministic-expectation model computed per neighborhood cohort. Stages:
  1. **Candidacy.** NPCs decide whether to run (ambition, opportunity, Viability). Nomination follows the constitution's candidate-selection rule; the MVP uses local party nomination.
  2. **Campaign** (weekly turns, DEC-012). Money, volunteers, staff, ads, canvassing, events, debates, and endorsements change *awareness* and *perceived positions and valence* per neighborhood and cohort.
  3. **Vote utility** per cohort for each candidate, made up of:
     - issue proximity (cohort attitudes, weighted by salience, compared with the candidate's *perceived* positions);
     - party identification;
     - valence: competence, likability, scandals, and retrospective judgment of incumbents (experienced conditions against expectations, from DEC-025);
     - local ties;
     - an awareness discount for unknown candidates.

     Utilities become choice shares through multinomial logit.
  4. **Turnout** per cohort: propensity × mobilization × enthusiasm × the constitution's rules (e.g. compulsory voting).
  5. **Shocks:** a small number of structured, seeded random shocks per election (election RNG stream). These include regionally and cohort-correlated swings and turnout shocks. There is no per-voter randomness.
  6. **Tabulation** with the constitution's voting rule (DEC-017 tabulators). Logit utilities give full preference distributions, so plurality, two-round, ranked-choice, approval, and list PR all use one voter model.
  7. **Seats and control**, then **consequences**: Mandate from margin and pledges, NPC memories, and narrative events.
- **Explainability:** each result carries a decomposition of swing by factor (issues, economy, scandal, campaign effort, turnout, shock) per district. This powers the "why you won or lost" screen.
- **Dependencies:** DEC-012, DEC-017, DEC-018, DEC-025, DEC-027, DEC-029.

### DEC-029 — Polling & imperfect information

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Proposed decision:**
  - The player never sees the true electoral state, only **polls**. Pollsters are NPC organizations that sample the true model with sampling noise, house effects, and occasional systematic error.
  - Private polling costs money and gives better precision.
  - The election result is the truth.
  - Polling misses are therefore organic drama, and seeded shocks (DEC-028) keep the outcome uncertain without being unfair.
  - A difficulty or complexity layer may show uncertainty bands (brief §3.5). It never shows the truth.
- **Dependencies:** DEC-028, D9 (media), D10 (difficulty).

### DEC-030 — NPC political AI: rule symmetry

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Proposed decision:**
  - **NPC politicians play by the player's rules.** They have the same action set, authority, slots and resources (abstracted), and the same pressure tracks. They get no hidden bonuses.
  - They plan with utility-based AI that evaluates actions using **polls, not truth**. Strategic profiles weight the utilities (brief §9.4): ideologue, pragmatist, populist, insider, local advocate, wealthy outsider.
  - **Difficulty** changes NPC competence (information quality, planning depth, efficiency), not stat bonuses. This keeps difficulty honest (brief §3.5).
  - The same AI drives the time-skip player-proxy (DEC-012) and the burn-in (DEC-022).
- **Consequences:**
  - The action system must be actor-agnostic.
  - Simplified ("abstracted") NPC turns are needed for performance, but they must use the same rules.
- **Dependencies:** DEC-011, DEC-012, DEC-022, DEC-028, DEC-029, D9, D10.

### DEC-031 — Character model, relationships & memory

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Proposed decision** (applies to Principal and Named NPCs, DEC-018):
  - **Identity:**
    - name,
    - age,
    - home neighborhood,
    - appearance parameters (for the layered sprite generator, DEC-009),
    - background or role (shared with DEC-020 role data),
    - occupation and organization memberships.
  - **Personality:** 5 continuous axes plus 0–3 trait tags.
    - Axes: ambition, integrity, warmth, boldness, loyalty.
    - Tags: e.g. vain, zealous, grudge-holder, reformer.
  - **Values:** an attitude vector in the same space as cohorts. Generation is *conditioned on* cohort context, not determined by it (brief §6.4).
  - **Competencies:** oratory, negotiation, administration, organizing, intrigue, and expertise (one domain).
  - **State:** resources, schedule (by day-part, DEC-013), and pressure tracks if the character is a politician (DEC-030).
  - **Relationships:** directed edges with:
    - opinion (−100..100),
    - trust,
    - typed bonds (family, friend, ally, rival, mentor, employer, patron),
    - a **favor ledger** (favors owed, with context).
  - **Memory:** bounded, about 20 salient memories per Principal NPC and about 5 per Named NPC. Each memory records:
    - the event it refers to,
    - the actor,
    - sentiment and magnitude,
    - the date,
    - a decay rate,
    - tags.

    Low-salience memories decay into the relationship baseline. Strong ones (betrayals, kept promises) persist and are quotable in dialogue ("You promised the bridge in '31").
- **Dependencies:** DEC-009, DEC-011, DEC-018, DEC-020, DEC-030.
- **Implemented (M5b, 2026-11-18):**
  - `mandate_sim::characters`, stored in the politics domain (`PoliticsState::characters`), because politics systems create politicians and read relationships on their hot paths. Every politician is a Principal character: `Politician::character` names a record holding warmth, boldness, loyalty, 0–3 traits, administration, organising, intrigue, an expertise issue, a first impression, and memory. Identity, values, ambition, integrity, oratory, and negotiation stay on `Politician`; `characters::profile` resolves anyone. Character draws use keyed RNG (seed, politician id), so no earlier stream shifts.
  - Named characters come from an occupation registry (`occupations.jsonc`: places, chance, class and age, leader, influence, stake, offsets, expertise, ambient). They are generated once when play begins, with values conditioned on their home cohort, and updated lazily one keyed step per year (order-independent, tested). Ambient passers-by are generated per place and week and never stored; meeting one (`Meet`) promotes them. Tiers are derived (`tier_of`). Not yet: appearance parameters, schedules, and organisation memberships beyond a leader's workplace.
  - Relationships are directed edges (opinion, trust, typed bonds) in a `BTreeMap`, with a favour ledger (debtor, creditor, count, Fluent context, subject). The player's `opinion_of_player`/`favors` became these, with identical results (normalised state dumps, sweeps, bots, trace-econ).
  - Memory: 20 per Principal and 5 per Named (`characters.jsonc`); kinds with sentiment, magnitude, decay, opinion and trust change, tags; traits (`traits.jsonc`) scale memory decay and opinion changes. Recorded from votes on a sponsor's bill, broken party lines, favours promised, kept, and broken, lobbying (player and NPC), election defeats (rival bonds), and a mayor's signature or veto. Faded memories are forgotten monthly; their opinion change stays. No scandal source exists yet, so the `scandal` kind is a hook.
  - NPC AI reads personality (DEC-030, the same rules for the player): loyalty scales party discipline (`council.loyalty_discipline`), integrity resists lobbying (`council.lobby.integrity`), warmth and traits scale opinion changes. A politician who becomes the player keeps the relationships their seat-holder had with colleagues.
  - Protocol `CharacterView` (facts and keys: role, tier, personality labels, opinion and trust bands, bonds, favours, three most salient memories); character sheets in the web. Save format v11 with a v10 migration.

### DEC-032 — NPC goals & storylines

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Proposed decision:**
  - **Goals emerge from circumstances.** Goal templates trigger on simulation conditions plus personality. Examples:
    - laid off and high warmth → organize the workers;
    - journalist and an anomaly in procurement data → investigate;
    - ambition and an open seat → run.
  - Goals drive NPC actions through the DEC-030 utility AI.
  - When a goal needs the player, it surfaces as a **request**.
  - **Storylines** are multi-stage arcs instantiated from authored templates. Each defines:
    - **roles** cast from existing simulation actors (originator, antagonist, affected place);
    - **stages** that advance on simulation conditions or player actions;
    - **branches and outcomes** that write back only through engine commands;
    - **follow-up hooks**.
  - Every stage must reference live simulation state. There are no fetch quests (brief §10.5).
  - Storylines persist across acts and level-of-detail changes. The rival arc follows you up the ladder.
- **Dependencies:** DEC-024, DEC-030, DEC-031, DEC-033.
- **Implemented (M5c, 2026-11-19):** goal templates in content (`goals.jsonc`: holders, personality, traits, live conditions, a drive, an optional request, `max_days`), taken up monthly in the player's town (`characters::goals`); drives add to the council lean on the player's bills, set an opposer's lobbying focus, or rank crisis answers higher; requests are lightweight dilemmas. Storyline templates (`storylines.jsonc`: the plant, the rival, the favour debt) cast roles from live actors, advance on conditions and answers, branch, fade on timeout or when a person no longer fills their role, and persist in saves. Not yet: goals for NPCs outside the player's town (they need no player), and goals that change candidacy.

### DEC-033 — Narrative director: authority & pacing

- **Status:** Accepted (owner, 2026-11-18): option B.
- **Precedent** (agent's knowledge, not re-verified this session):
  - Classic roguelikes (Rogue, NetHack, Brogue) are effectively A, with pacing coming only from depth-scaled generation.
  - Modern roguelites pace mainly through *structure* (act maps, guaranteed rest and shop nodes, bosses) plus luck-smoothing. Slay the Spire raises rare-card and potion odds after misses; XCOM 2 adds hidden aim assistance on lower difficulties.
  - Reactive directors (Left 4 Dead's AI Director, RimWorld's storytellers) are B-like and belong to long-form emergent-story games, Mandate's closest cousins. Over a 6–10 h career, an unlucky pile-up is a ruined run rather than a funny story.
- **Addendum (proposed, post-MVP; built early, in M5c, see "Implemented" below):** selectable **storyteller profiles**, following RimWorld:
  - "Classic": B with a rising-tension curve. This is the default.
  - "Unfiltered": A-like pure chance, for purists.
  - "Steady": lower crisis rate.

  Profiles are a transparent difficulty and flavor lever (brief §3.5). They never touch consequences.
- **Context:** Brief §10.6 and §31.11 say narrative must present, not invent. Yet a pure observer cannot prevent dull stretches or crisis pile-ups, and RimWorld's storytellers work precisely by modulating event generation.
- **Options:**
  - A. **Pure observer.** All events come from simulation processes. The director only selects which situations are spotlighted (scenes, decisions) and which become news or ambient detail.
  - B. **Storyteller of chance, not of consequence.** A plus limited authority over *exogenous* randomness, within plausible bounds:
    - timing and severity of hazards (drawn from the regional hazard model),
    - external economic shocks,
    - discovery timing of hidden facts, within evidence-based windows,
    - arrival of opportunities.

    It can **never** alter endogenous consequences (outcomes of actions, policies, elections, NPC decisions).
  - C. A free storyteller that can invent events. This violates §31.11.
- **Proposed decision:** B.
  - The director is a deterministic part of the simulation, with its own RNG stream, and everything it does is logged.
  - **Pacing model:**
    - Each chapter has a tension curve rising to one climax.
    - Event budgets per chapter (targets, to be tuned): ~1 climax, 2–4 significant decisions, ~5–10 minor developments.
    - Category frequencies follow the brief (minor frequent → career-defining very rare).
    - Cooldowns and repetition penalties apply.
    - There is a cap on simultaneous unresolved crises.
  - **Relevance scoring:** connection to the player, their constituency, past decisions, active storylines, pressure tracks, and upcoming elections.
  - **Enforces the DEC-006 fairness rules:** warning events before any terminal ending.
- **Rationale:** RimWorld-grade pacing without the player ever being able to say "the game decided my policy failed". Chance is shaped; consequences are honest.
- **Dependencies:** DEC-005, DEC-006, DEC-012, DEC-023, DEC-032, DEC-034.
- **Implemented (M5c, 2026-11-19):** `mandate_sim::narrative` (option B): chapters (a campaign or a session year) with a tension curve, budgets and spacing in `director.jsonc`, cooldowns, a repetition penalty, relevance scoring, and fairness warnings with an ending gate. The director writes only its own domain; its exogenous requests (`Chance`: hazard holds at the crisis cap, a thinner hazard rate, a held recession onset, discovery dates of hidden facts) are applied and logged by the environment and economy. Tests: spotlight on vs off leaves every endogenous outcome identical; held hazards are the same hazards, each within the longest deferral. Storyteller profiles Classic (default), Unfiltered, and Steady are content (`storytellers.jsonc`); choosing one is a new-game setting (`narrative.profile`), not yet offered on the title screen. Hidden facts so far: the player's late-night deals and fundraising (the media task will expose them).
- **Storyteller choice (M5d part 2, 2026-11-19):** the new-game screen offers the profiles from the registry (`QueryDto::Storytellers`: name and one-line description, the content default preselected); `NewGameParams.storyteller` sets `narrative.profile` before play (unknown ids are refused), so it is kept in the run's state and saves (no format change). The overview carries the profile (`storyteller`, `storyteller_name_key`); the saved-runs list and the story sheet show "<name> storyteller".

### DEC-034 — Media & information model

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Proposed decision:**
  - **Four information layers** (brief §11):

    | Layer | What it holds |
    |---|---|
    | Facts | What actually happened, recorded as fact records |
    | Knowledge | Which actors know which facts, and with what evidence |
    | Coverage | What outlets report and with what framing |
    | Perception | What cohorts and NPCs believe |

  - **Outlets** are organizations with:
    - audience by cohort,
    - reach by region,
    - editorial lean (attitude vector),
    - credibility,
    - issue interests,
    - finances,
    - journalists (NPCs).
  - Outlets select stories by newsworthiness × editorial interest × access. Social platforms are a high-reach, low-credibility, viral outlet type.
  - **Scandals:** hidden facts with evidence and exposure risk. Investigators and journalists discover them over time; this drives Legal Jeopardy and Standing (DEC-006).
  - **Rumors:** unverified or false beliefs can spread and be corrected.
  - **Player communication** changes framing and salience. Outlets respond according to their own incentives; they do not repeat the player's message.
- **Dependencies:** DEC-006, DEC-025, DEC-029, DEC-031, DEC-033.
- **Implemented (M5d part 1, 2026-11-19):** `mandate_sim::media` (its own domain). Outlets from a registry (`outlets.jsonc`: a town paper at each newspaper office, a regional paper, a national broadcaster, a social platform) with audience by class and age, reach, a lean drawn around their audience's views, credibility, interests, finances, and journalists (Named characters at their office), generated when play begins. Facts: the facts log and hidden facts; knowledge: who uncovered what; coverage: each outlet's weekly stories by newsworthiness × interest (more when it disagrees with the subject) × access, framed by its hostility; perception only through coverage (cohort approval and awareness of the player by audience weight and credibility; any politician's scandal valence in elections; memories of colleagues), tested against a twin without the fact. Whether a hidden fact comes out is settled when it happens; the director only times it within its evidence window (DEC-033 B, tested on and off). Rumours from a leak with nothing behind it spread on social platforms and may be corrected. The player's press statements are covered (or not, and framed) by the outlets; ads cool a running scandal; a story raises a response decision (deny, explain, apologise, counter-attack). Borderline and illegal choices: shady options in dilemmas and three shady actions (an envelope at the diner, the campaign card at the party office, a rival's file at the newspaper), each with a risk line before the choice. NPCs are tempted by personality with the same mechanics (DEC-030). The newspaper is a place service; headlines and scandals are in the pocket.

### DEC-035 — Conversations & dialogue

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Proposed decision:**
  - Conversations are sequences of beats drawn from authored, parameterized dialogue templates. Option types:
    - persuade,
    - promise (which may create a pledge or favor),
    - trade a favor,
    - pressure or threaten,
    - ask,
    - concede.
  - Options map to engine actions.
  - Contested options show their odds, computed from competencies, relationship, Mandate, and context, then roll on the NPC RNG stream.
  - Memories and facts fill template slots, so dialogue references real history.
  - All text goes through localization keys.
  - An LLM text layer is **deferred**. If ever added, it only rewrites already-validated facts (§10.7).
- **Dependencies:** DEC-011, DEC-013, DEC-031, DEC-034, D12, D13.

### DEC-036 — Character creation: draft of 3

- **Status:** Accepted (owner, 2026-11-18).
- **Options:**
  - A. Full custom creation.
  - B. Draft: pick one of 3 generated candidates.
  - C. A single generated character.
- **Decision:** B.
  - Each run offers 3 candidates generated for *this* world. Each candidate has:
    - a background (shared role data with DEC-020),
    - a hometown,
    - personality,
    - one notable strength and one flaw,
    - starting relationships,
    - starting resources,
    - a situation hook.
  - Rerolls are allowed (count to be tuned). Name and appearance are editable.
  - Backgrounds grant **different opportunities and constraints**, not stat inflation (brief §3.2).
- **Consequences:**
  - Candidate generation runs after the burn-in (DEC-022), so candidates are embedded in real local history.
  - The three candidates should differ in background, region, and strength.

### DEC-037 — Personal life scope

- **Status:** Accepted (owner, 2026-11-18): option B.
- **Options:**
  - A. None: the character is purely a political avatar.
  - B. Light but consequential: household, health, and personal finances, included only where they create decisions.
  - C. Deep life simulation (Crusader Kings-style family management, romance, dynasties).
- **Proposed decision:** B.
  - **Household:** an optional partner and children (as drafted), as Named NPCs. They provide:
    - support: a campaign surrogate, morale;
    - strain: family time competes for slots, and neglect leads to relationship decline, which can end in separation and becomes a Standing hit;
    - vulnerability: a partner's business conflicts, or a child's trouble, raise Legal Jeopardy and Standing exposure;
    - a family section in the Legacy epilogue.
  - **Health:** part of the Age/Health track. Late Night slots, crises, and stress wear it down. Rest restores it. Illness events come with warnings (DEC-006 fairness).
  - **Personal finances:** wealth and income. These affect self-funding, vulnerability to bribes, and financial-disclosure scandals. Detour roles pay differently.
  - **Skill growth:** competencies improve through use and detours. This is the main in-run character progression.
  - **Excluded:** romance and dating mechanics, and explicit content.
- **Dependencies:** DEC-006, DEC-013, DEC-020, DEC-031, DEC-036.

### DEC-038 — Difficulty & complexity layers

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Proposed decision:** difficulty and complexity are separate settings. Neither makes the simulation dishonest (brief §3.5).
  - **Difficulty levers**, bundled into presets plus a custom option:
    1. Storyteller profile (DEC-033).
    2. Starting world conditions: economic outlook, polarization, institutional trust, corruption baseline.
    3. Draft candidate strength.
    4. NPC competence (DEC-030).
    5. Information quality: poll precision and analyst accuracy (DEC-029).
    6. Save mode: Ironman or Story (DEC-008).
  - **Complexity layers** (UI only, changeable at any time):
    - Casual: summaries and recommended actions.
    - Standard.
    - Advanced: demographics and causal breakdowns.
    - Simulation: diagnostics.

    The underlying simulation is identical across layers.
- **Dependencies:** DEC-008, DEC-029, DEC-030, DEC-033, D12.

### DEC-039 — Meta-progression

- **Status:** Accepted (owner, 2026-11-18; no objection raised). Brief §19: keep it separate from run state; no permanent stat inflation.
- **Proposed decision:**
  - **Unlocks of new possibilities only:**
    - constitutional archetypes and quirks,
    - backgrounds and detour roles,
    - storyteller profiles,
    - starting scenarios (late starts, DEC-005),
    - challenge modifiers (opt-in "heat"-style handicaps for experienced players).
  - Unlocks are earned through Legacy milestones.
  - A **Hall of Legacies** records past careers.
- **Parking-lot idea (unratified):** **Legacy worlds.** A new run starts in the *same* world decades after a previous career ends, as a protégé or descendant, with the old career's laws, institutions, and grudges still in place (in the spirit of Rogue Legacy). The burn-in (DEC-022) makes this feasible. Evaluate after the MVP.
- **Dependencies:** DEC-005, DEC-006, DEC-007, DEC-020, DEC-033, D13 (meta-save separate from run saves).

### DEC-040 to DEC-046 — Technical architecture baseline

- **Status:** Accepted by the owner (2026-11-18) as a package. The pre-implementation architecture review (brief §26.1) is still required after D15.
- **Where the content lives:** `docs/architecture/OVERVIEW.md`. Sections map to decisions as follows:
  - §2: DEC-040, crates
  - §3: DEC-041, determinism
  - §4: DEC-042, engine
  - §5: DEC-043, protocol
  - §6: DEC-044, persistence
  - §7: DEC-045, frontend
  - §8: DEC-046, budgets
- **Revisions to the brief:** the brief's original proposals that this baseline changes are listed in OVERVIEW §10.
- **DEC-017** (constitution primitives) is confirmed as part of this baseline.
- **DEC-045 resolution:** the owner has no framework preference. Criteria: fast, robust, responsive, maintainable by AI agents. Decision: **React + TypeScript (strict)**.
  - **AI maintainability:** React has by far the largest corpus of idiomatic code. Svelte 5 changed its reactivity syntax (runes), and agents frequently mix old and new syntax. Solid's look-alike JSX invites React habits that silently break its reactivity (e.g. destructuring props).
  - **Speed:** the heavy work runs in the Worker (simulation) and in PixiJS (scenes), so the UI framework is not the performance bottleneck.
  - **Guardrails:**
    - a small, fixed set of state patterns (Zustand for UI state; worker queries through one typed hook layer);
    - no ad-hoc global stores;
    - lint rules for hooks;
    - evaluate the React Compiler at Milestone 0 to remove manual memoization (deferred to the M6 performance pass).
- **Later revisions:** DEC-044 by its revision 2 (below DEC-067) and by DEC-074; the scene renderer is Canvas 2D, not PixiJS (DEC-080). OVERVIEW is kept current with them.

### DEC-047 — Screen architecture: hub-centred

- **Status:** Accepted (owner, 2026-11-18; no objection raised). Detail in `docs/design/UI.md` §1–2.
- **Proposed decision:** the hub scene is the single primary screen.
  - All other screens are panels, overlays, or flow screens around it.
  - In-world portals (desk, news, map room, phone, campaign HQ) are mirrored in a global menu.
  - Turn, chapter, and act flows have dedicated resolution screens.
  - Every important figure has a "Why?" breakdown.

### DEC-048 — Responsive & mobile strategy

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Proposed decision:**
  - **Mobile-first, not desktop-shrunk** (brief §15.4).
  - **Phone:**
    - the hub fills the screen;
    - a minimal HUD;
    - panels open as bottom sheets or full-screen sheets;
    - tap-to-move;
    - large touch targets.
    - Both orientations are supported. The hub camera and HUD adapt; portrait gives a narrower slice of the hub, with the camera following the player.
  - **Tablet and desktop:** panels dock as side drawers with the hub still visible. Keyboard shortcuts and optional WASD movement.
  - **One component library with responsive variants**, not two separate UIs.
  - Breakpoints and orientation priority are confirmed by the DEC-050 hub spike on real devices.

### DEC-049 — Visual style & camera

- **Status:** Accepted (owner, 2026-11-18): option A. **Open:** the source of the final art (owner, commissioned, or licensed pack) has not been answered yet. Initial art: DEC-065. From M6 the art comes from the pixel-art tool (DEC-084); whether a pixel artist is commissioned for its part library is decided by the owner's P0 rubric scores (DEC-084 D5).
- **Options:**
  - A. Top-down three-quarter pixel art (Stardew Valley, Eastward), 16–32 px tiles.
  - B. Isometric pixel art.
  - C. Flat illustrated / vector style.
  - D. Low-poly 3D.
- **Proposed decision:** A.
  - The brief proposes lightweight pixel art (§5.5).
  - Grid tiles suit procedural district layouts best.
  - Layered "paper-doll" character sprites need only 4 facing directions (isometric needs more angles and more complex depth sorting).
  - Small art budget; crisp at any scale with integer scaling.
  - Day-part lighting via tinting and lightmaps.
  - National and regional maps use the same palette in a stylized cartographic look.
- **Consequences:**
  - The art pipeline is a tileset plus building and interior templates, plus layered character parts (bodies, heads, hair, clothing, accessories) composed procedurally from NPC appearance parameters (DEC-031).
  - Art is the area where AI agents are weakest. Plan for owner-sourced or commissioned art, or a licensed asset pack, with placeholder art during development.
- **Dependencies:** DEC-009, DEC-021, DEC-031, DEC-050.

### DEC-050 — Rendering prototype spikes

- **Status:** Accepted (owner, 2026-11-18; no objection raised). These are throwaway prototypes under DEC-003; each needs owner approval before it starts.
- **Proposed spikes:**
  1. **Hub spike:**
     - PixiJS scene;
     - a procedurally generated district (~64×64 tiles, ~15 points of interest);
     - ~30 layered NPC sprites following day-part schedules;
     - tap-to-move with pathfinding;
     - day-part lighting.

     Measure frame rate, memory, and bundle size on a mid-range Android phone and on desktop. Check that portrait and landscape both work.
  2. **Map spike:** a national map with ~500 neighborhood polygons, ~60 municipalities, and overlays (choropleth, districts), with pan, zoom, and hit-testing. Compare PixiJS with SVG/Canvas.
- **Exit criteria:** budgets in OVERVIEW §8 are met, or the renderer choice is revised.
- **Outcome:** the map is Canvas 2D (M1); the hub spike chose Canvas 2D (DEC-080).

### DEC-051 — Accessibility & explainability UI

- **Status:** Accepted (owner, 2026-11-18; no objection raised). Brief §15.5.
  - **Reaffirmed (owner, 2026-11-19):** keep the current accessibility level — full screen-reader support and a text alternative for every visual view — above what most games offer. A real screen-reader playtest (VoiceOver/TalkBack) is planned for the M6 tester checkpoint.
- **Proposed decision:**
  - **The hub is never the only way to reach anything.** An accessible DOM layer mirrors the scene: a list of "places here" and "people here" with actions. The Agenda can perform any located action.
  - Full keyboard navigation and screen-reader labels.
  - Colour-independent indicators (icons and patterns on gauges and maps).
  - Text scaling, reduced motion, and a high-contrast theme.
  - Charts and maps have tabular alternatives.
  - **"Why?" breakdowns** are built from the event log and causal records (OVERVIEW §4.4) and from per-factor decompositions (DEC-028 elections, DEC-024 policy attribution).

### DEC-052 — Save sync readiness (future cloud sync)

- **Status:** Accepted (owner, 2026-11-18; no objection raised). Requested by the owner: hooks for future account sync of saves, through a database or through Dropbox or OneDrive.
- **Context:** TECH-006 (offline-first) and TECH-007 (no backend for single-player). Sync must be an optional future layer that never makes the game depend on a network.
- **Options for the eventual provider:**
  - A. **User-owned cloud storage.** Dropbox, OneDrive (Microsoft Graph), or Google Drive's app-data folder, using browser OAuth with PKCE. These work from a static site with **no backend of ours**; saves live in the user's own app folder.
  - B. Our own sync service with accounts and a database. This needs a backend, so it is a separate future capability (and a TECH-007 revision).
  - C. Platform sync (e.g. Steam Cloud) if a desktop wrapper ships.
- **Proposed decision:** build the **hooks now** and ship **no provider** in the MVP. A is the first provider candidate. B and C stay possible.
  - **Storage abstraction:** the frontend persistence layer sits behind a `SaveStore` interface. The MVP implements `IndexedDbStore`; providers are added later as additional stores plus a sync coordinator. File export and import remain the manual fallback.
  - **Sync-ready save identity and metadata.** Every run save carries:
    - `run_id`: a random 128-bit ID. Globally unique; it lives outside simulation state, so DEC-041 is unaffected.
    - `revision`: monotonic, incremented on every save.
    - `parent_revision`.
    - `device_id`.
    - `saved_at` (wall clock: metadata only, never simulation input).
    - `content_hash`.
    - engine, content, and protocol versions.
  - Save blobs are **self-contained** (one file per run plus a small manifest), so any file-based provider can carry them.
  - **Conflict policy for Ironman runs** (at most one continuous timeline): when two devices diverge from the same parent, the higher revision wins. The other branch is kept as an archived fork flagged "diverged". It is never silently merged.
  - **Meta save designed to merge** (DEC-039): unlocks are a grow-only set, and the Hall of Legacies is an append-only log keyed by `run_id`. Merging is then a union with no conflicts.
  - **Settings** are split into synced settings (preferences) and device-local settings (graphics, input).
  - **Not a security boundary:** sync is not anti-cheat. Copying an old save back is possible and acceptable for a single-player game without leaderboards.
- **Consequences:**
  - Adds fields to the DEC-044 save format.
  - Adds a requirement (TECH-009).
  - A provider later needs a registered OAuth client ID per provider and a privacy notice. No server is needed.
- **Dependencies:** DEC-008, DEC-039, DEC-044.

### DEC-053 — Content authoring format & pipeline (revision 2)

- **Status:** Accepted (owner, 2026-11-18): JSONC.
- **History:** revision 1 chose YAML. The owner questioned YAML's significant whitespace, and on review the concern holds.
- **YAML problems found:**
  - Indentation errors can silently re-parent data. Strict schemas catch most of these, but not all; a mis-indented list item can still be valid.
  - Implicit typing: `no`/`yes` read as booleans in YAML 1.1 parsers (the "Norway problem"), and `1.10` reads as a float.
  - Anchors and merge keys add complexity.
  - Agents editing deeply nested files by text replacement tend to break indentation.
  - The standard Rust crate, `serde_yaml`, is no longer maintained.
- **Options:**

  | Format | Whitespace-safe | Comments | Deep nesting and enums | Editor schema validation | Agent fluency | Rust parsing health |
  |---|---|---|---|---|---|---|
  | YAML | ✗ | ✓ | ✓ | ✓ | Good, but indentation errors | ✗ unmaintained `serde_yaml` |
  | TOML | ✓ | ✓ | ✗ awkward beyond 2 levels | Partial | Good | ✓ excellent (`toml`, used by Cargo) |
  | RON | ✓ | ✓ | ✓ very natural for Rust enums | ✗ weak | Fair (Rust-like syntax) | ✓ |
  | **JSONC** (JSON plus comments and trailing commas) | ✓ | ✓ | ✓ | ✓ native in VS Code and other editors | Excellent | ✓ via a JSONC parser, then serde |

- **Proposed decision:** **JSONC**, validated against JSON Schemas generated from the Rust content types.
  - Prose never lives in content files; it lives in Fluent `.ftl` files (DEC-054). This removes JSON's main ergonomic weakness, multi-line strings.
  - Enums are internally tagged: `{ "kind": "transfer", ... }`.
  - A formatter enforces consistent layout in CI (e.g. dprint or prettier).
  - **Runner-up:** RON, if mod-facing editor tooling turns out not to matter.
- **Unchanged from revision 1:**
  - `mandate-cli content validate` in CI;
  - compiled content bundle with a content hash;
  - Rust types as the single source of truth.
- **Verification at Milestone 0:** pick the JSONC parser crate (candidates: `jsonc-parser`, or strip comments and trailing commas and then use `serde_json`); confirm maintenance and WASM build.
- **Dependencies:** DEC-017, DEC-024, DEC-041, DEC-044, DEC-054, DEC-055, REQ-006.

### DEC-054 — Text & localization pipeline

- **Status:** Accepted (owner, 2026-11-18; no objection raised).
- **Proposed decision:**
  - **The engine never produces prose.** It emits structured facts and template keys with arguments.
  - Player-facing text is rendered in the frontend from **Fluent** (`.ftl`) message files. Fluent handles plurals, grammatical variants, and NPC pronouns properly.
  - Dialogue and narrative templates reference message keys.
  - This keeps localization possible later, and keeps any future LLM text layer purely presentational (§10.7).
- **Dependencies:** DEC-035, DEC-053.

### DEC-055 — Modding scope

- **Status:** Accepted (owner, 2026-11-18): option B. Brief §17.3.
- **Options:**
  - A. No modding intent. Content is data-driven only for internal productivity.
  - B. **Mod-ready, no mod support in the MVP.** Content is organized as packs (base pack plus optional packs) with ID-based overrides and load order. Pack lists feed the content hash. Schemas and the validator are published as-is. A mod loader UI and sharing are deferred.
  - C. First-class modding at launch: a loader UI, documentation, and a sharing workflow.
- **Proposed decision:** B. It costs little beyond what DEC-053 already requires, and keeps C open.
- **Dependencies:** DEC-053, DEC-054, D15.

### DEC-056 to DEC-060 — Test & validation strategy

- **Status:** Accepted (owner, 2026-11-18). The owner can recruit a few playtesters for M-gates.
  - DEC-059 was revised in response to the owner's concern about real-world ranges being boring: bands now apply to baseline runs only, distributions must include tails, drama floors were added, and world era volatility is a parameter.
- **Content:** `docs/development/TEST_STRATEGY.md`. Sections map to decisions as follows:
  - §1: DEC-056
  - §2: DEC-057
  - §3: DEC-058
  - §4: DEC-059
  - §5: DEC-060
- **Key choices:**
  - Exact-output golden tests are reserved for determinism. Balance-sensitive outputs are asserted with metric bands.
  - Replay fixtures are the regression format.
  - Bot-player tournaments detect dominant strategies and check run-length and pacing targets.
  - Human playtesting happens at every milestone gate, plus a small group at M6.
  - Opt-in local run-report export only; no telemetry backend (TECH-007).

### DEC-061 — Tone: heightened realism

- **Status:** Accepted (owner, 2026-11-18; no objection raised). Prompted by the owner's question about whether realism makes the game boring.
- **Proposed decision:** Mandate aims for **heightened realism**, in the register of a political drama series rather than a civics textbook.
  - Institutions, economics, and causality are believable and consistent.
  - Drama is concentrated at the human and narrative scale (scandals, betrayals, upsets, crises, comebacks). The director compresses and spotlights it (DEC-033), and every chapter has a climax (DEC-005).
  - Macro aggregates move plausibly but can swing harder than real-world norms in turbulent eras.
- **Consequences:** DEC-059 drama floors; era volatility as a world parameter; a GAME_VISION pillar.
- **Dependencies:** DEC-005, DEC-033, DEC-038, DEC-059.

### DEC-062 — MVP scope (vertical slice)

- **Status:** Accepted (owner, 2026-11-18; no objection raised). Detail in `docs/development/ROADMAP.md`.

### DEC-063 — Milestone order: real systems first, with early playable checkpoints

- **Status:** Accepted (owner, 2026-11-18): option A, the brief's systems-first order, adapted to the accepted decisions. Option B (a thin loop on a hand-authored micro-town first) was **rejected**.
- **Owner's rationale:** AI agents can build the real structure quickly, so validating on a temporary world saves little time. Building the real architecture from the start avoids technical debt.
- **Residual risk (DISCOVERY C5):** fun is validated later than under B.
- **Mitigation (no extra code):**
  - Each milestone ends with a playable or inspectable increment in the *real* game.
  - Lightweight owner checkpoints at **M3** (a policy loop in a generated town, panel UI) and **M4** (a full local election) come before the M6 tester gate.
  - Bot players (DEC-060) give early balance signals from M4 on.
  - Mechanics are data-driven (DEC-053, REQ-005), so design changes found at checkpoints are mostly content and tuning changes.
- **Consequences:** milestones as in `docs/development/ROADMAP.md` §2.

### DEC-064 — Hosting: GitHub Pages first, portable to a Cloudflare domain

- **Status:** Accepted (owner, 2026-11-18). The owner enabled GitHub Actions for deploys. The repository is public.
- **Decision:**
  - Deploy the static PWA to **GitHub Pages** from `main` with an Actions workflow.
  - The site initially serves at a project sub-path (`/mandate/`). The Vite `base`, the PWA manifest `start_url`/`scope`, and the service-worker scope all come from one build variable, `MANDATE_BASE` (default `/mandate/`).
- **Moving to Cloudflare later is supported.** Either:
  - point a custom domain (managed in Cloudflare DNS) at GitHub Pages; or
  - deploy the same `dist/` to Cloudflare Pages.

  In both cases, rebuild with `MANDATE_BASE=/`.
- **Caveats:**
  - Changing origin (domain) means IndexedDB saves do not carry over automatically. Before switching, ship file export/import and tell players to export (DEC-052; shipped as DEC-082).
  - GitHub Pages cannot set custom headers, which rules out cross-origin isolation. This is acceptable, because multithreaded WASM is deferred (TECH-008).

### DEC-065 — Initial art: Kenney CC0 asset packs

- **Status:** Accepted (owner, 2026-11-18). This resolves DEC-049's open item for the initial phase.
- **Decision:**
  - Use Kenney (kenney.nl) top-down packs. They are CC0 (public domain); for example, Tiny Town and RPG Urban Pack. They serve as the initial art for hubs, characters, and UI icons.
- **Sourcing:**
  - kenney.nl blocks automated downloads from the build environment. Fetch the packs from the official OpenGameArt mirrors, or have the owner add them to the repo.
  - Store them under `web/public/assets/kenney/` with license notes in `ASSETS.md`.
- **Later:** the owner will provide a logo and a loading animation.
- **Consequences:** sprite tile size follows the chosen pack (16 px). The layered character generator adapts to the pack's character parts, or uses tinting where parts are unavailable.
- **Status note (2026-11-19):** no Kenney packs have been added to the repository. The hub's placeholder art is drawn in code (DEC-080), and the game's art from M6 comes from the pixel-art tool (DEC-084). The 16 px tile size stands.

### DEC-066 — CI: build once, cache WASM by source hash

- **Status:** Accepted (owner, 2026-11-18).
- **Decision:**
  - A `wasm` job builds `mandate-wasm` and runs `wasm-bindgen`.
  - Its output (`web/src/engine/pkg/`) is cached with a key that hashes `Cargo.lock`, `rust-toolchain.toml`, and every `crates/**` source file. A cache hit skips the Rust build entirely.
  - The output is uploaded as an artifact for the web, E2E, and deploy jobs.
  - Rust build caching uses `Swatinem/rust-cache`. The `wasm-bindgen-cli` binary is cached and pinned to the `wasm-bindgen` crate version.
  - npm is cached through `setup-node`.

### DEC-067 — System plan approved; implementation begins

- **Status:** Accepted (owner, 2026-11-18).
- **Owner instructions:**
  - Proceed through all milestones.
  - Push to `main` when ready so Pages builds and the owner can test on a phone.
  - Ask if questions remain.
- **Decision:**
  - `SYSTEM_PLAN.md` is the implementation baseline. DEC-003 is satisfied.
  - The owner-checkpoint gates (M3, M4, M6) remain: report at each one, and continue unless the owner objects.

### DEC-044 revision 2 — Save format compatibility mechanism

- **Status:** Accepted (agent technical decision during M0, 2026-11-18; within the owner's delegated technical authority).
- **Change:** revision 1 specified separate save DTO structs mirroring every entity. Duplicating every simulation type would double maintenance across all milestones. Revision 2 achieves the same goal, a stored format independent of internal refactors, through a different mechanism:
  - **The stored payload is a versioned JSON snapshot** of the serialized state, DEFLATE-compressed, with a BLAKE3 integrity hash. It has a binary header (`MNDT`, format version, metadata length, metadata JSON) so metadata can be read without inflating the payload.
  - **Compatibility comes from JSON-level migrations** (`vN → vN+1` functions on `serde_json::Value`) plus a fixture corpus.
  - **A shape-fingerprint test** (`mandate-save/tests/saves.rs`) fails whenever the serialized state shape changes without a `SAVE_FORMAT_VERSION` bump. Refactors can never silently break saves.
  - **Pre-release policy:** development builds may bump the version without a migration. Such saves are rejected with a clear message. From the first public release, every bump requires a migration and a fixture.
- **Protocol DTOs remain separate** from internal types (DEC-043). Only the save payload uses this mechanism.
- **Later:** DEC-074 replaced the command log with a full save after every action, and lets saves load under updated rules.

### DEC-068 — World generation implementation choices (M1)

- **Status:** Accepted (agent technical decision during M1, 2026-11-18; within delegated technical authority). Refines DEC-021 and DEC-022.
- **Raster size.** The terrain raster is 180×120 (small), 240×160 (medium), or 300×200 (large) at about 2.5 km per cell. DEC-021 suggested about 512². Smaller grids keep generation at about 90 ms (medium, WASM) with believable results, and the simulation runs on the region and neighbourhood graph, not the raster.
- **Terrain is derived, not saved.** The raster is regenerated from the seed on load (`rebuild_terrain`), and a terrain hash in the world detects generator drift. Saves hold only simulation data.
- **Territories and roads cross lakes** at high cost. Lakes belong to municipalities; bridges and ferries keep the network connected. This was found by the world invariant tests.
- **Canonical test worlds** are generation presets (`Preset`), selectable from the lab CLI and the new-game screen ("Scenario"), and tested for their intended biases.
- **Neighbourhoods are graph units**, not raster polygons. Electoral districts at all three tiers are generated by a contiguity-preserving, population-balancing partitioner (property-tested).
- **Deferred:**
  - The hub tile-layout generator (M1-09) moves to M6, beside the hub renderer. Places already exist as entities. (Built on the client as M6 groundwork: DEC-085.)
  - The lab `diff` command moves to M2, where there is evolving state to compare.

### DEC-069 — Society and economy implementation choices (M2)

- **Status:** Accepted (agent technical decision during M2, 2026-11-18; within delegated technical authority). Refines DEC-018, DEC-019, DEC-023, DEC-042, and DEC-044.
- **Domain write isolation is compile-time enforced.** Each system receives `&mut` access only to its own domain (population, economy, finance, environment) and `Reads` for the others. Cross-domain changes are typed `Effect`s. This replaces the planned grep-based architecture test.
- **Population owns resident counts.** `Neighborhood` no longer stores population; the population domain holds 12 cohorts per neighbourhood (count, 8 attitudes, trust, turnout). People are conserved exactly by integer accounting, which is tested yearly.
- **Economic circumstance is per municipality and class**, held in the economy domain (employment rate, income by class). It is not stored per cohort, which keeps saves small; cohorts read it.
- **Tax rates are finance state** (standing settings, DEC-024), seeded from content. A baseline fiscal rule stands in for governments until M3–M4:
  - budgets are indexed to prices × productivity × population;
  - grants are indexed the same way;
  - local and regional tiers follow a balanced-budget baseline with local tax adjustment;
  - the national gap is closed half by taxes and half by trims, with a debt brake above 60%.
- **Economic dynamics:**
  - domestic demand is measured in real, productivity-normalised terms;
  - expectations-augmented Phillips curve and a Taylor-rule central bank;
  - interest channel on consumption;
  - unit-labour-cost competitiveness for traded sectors;
  - structural adjustment toward full employment over years.

  Tuning history is in `docs/development/BALANCE_LOG.md`.
- **Exact JSON float round-trips** (`serde_json/float_roundtrip`) are required for save → load → continue determinism. A guard test protects this.
- **Lab tooling:** `Command::LabSectorShock` (lab only, never exposed to players) and the `trace-econ`, `sweep`, and `diff` CLI commands. The quick sweep runs in CI; the full sweep across every preset runs nightly.

### DEC-070 — Local government and policy implementation choices (M3)

- **Status:** Accepted (agent technical decision during M3, 2026-11-18; within delegated technical authority). Refines DEC-020, DEC-021, DEC-026, DEC-030, DEC-033, DEC-044, and DEC-069. The mid-turn command journal below was replaced by a full save after every action (DEC-074).
- **Constitutions are data built from primitives:** bodies, offices, powers, decision rules, constituencies, and staged procedures. Validation checks that every tier can pass an ordinance and a budget (REQ-006). The Federal Presidential archetype ships first: a ward council and mayor locally, with regional and national tiers defined but not yet simulated.
- **Policy lifecycle has five states** (Draft, Under consideration, Enacted, In effect, Ended with a reason), as agreed in discovery. Funding is a status of an enacted law: appropriations decide the funded share each month, and the funded share drives implementation progress.
- **Mechanisms are typed and closed:**
  - Service, Transfer, HousingSupply, Zoning, SectorSubsidy, WageFloor, HazardProtection, LocalTaxRate.
  - Templates combine one mechanism with parameters, costs, and secondary-effect keys.
  - The domains read the resulting `PolicyMods`; policy never writes another domain directly.
- **Council votes are probabilistic but explained.** Each member's lean combines constituents' anticipated support, their convictions, party-free pragmatism, opinion of the sponsor, and lobbying sway. `yes = sigmoid(8 × lean)`. The whip view shows every member's lean, yes chance, and lobbying odds.
- **Player model:**
  - three standard slots per turn plus one late-night slot;
  - Mandate and five pressure tracks;
  - campaign funds;
  - actions: lobby, trade a favour, town hall, press, canvass, fundraise, and rest.
  - Introducing a bill and amending one before the council each use a slot.
  - Late-night lobbying improves odds but costs health and adds legal jeopardy (DEC-006).
  - A favour trade is a guaranteed vote. The player may owe one colleague at most two favours at a time (`MAX_FAVOR_DEBT`); repaying them arrives with NPC agendas in M4.
- **Ironman integrity mid-turn (DEC-044):** the snapshot is still written at each turn boundary. Every player command since then is appended to a journal in IndexedDB. On load, the snapshot is restored and the journal is replayed; replay is exact because commands are deterministic. Closing the tab cannot undo a failed lobby. This costs one small write per action instead of a full 450 KB save (about 88 ms).
- **Non-string-keyed maps in state** serialise as lists of pairs (`mandate_core::serde_util::map_as_pairs`), because saves and the state hash are JSON. A state that cannot be serialised now fails loudly in debug builds and tests instead of producing an empty hash.
- **Deferred:**
  - NPC-sponsored bills and NPC agendas: M4 (NPC AI).
  - Authority (the executive-power resource): M4, with the mayor's office.
  - A dedicated budget-procedure screen: M4. Appropriations follow the fiscal rule until then.

### DEC-071 — Research-informed refinements to the election, party, NPC, and migration models

- **Status:** Accepted (agent technical decision at the start of M4, 2026-11-18, after the owner asked for a research pass; within delegated technical authority). Refines DEC-025, DEC-027, DEC-028, DEC-029, and DEC-030. Evidence and sources: `docs/design/research/SIMULATION_MODELS.md`.
- **Vote choice:**
  - Issue utility blends proximity and directional components. The blend weight β rises with electorate polarisation.
  - Candidate awareness *multiplies* the issue and valence terms.
  - Party labels count only where the constitution prints them on the ballot.
- **Incumbency** is not a flat bonus. It emerges from awareness, record, and challenger entry, and is tuned so that local incumbents win about 65–80% of races. This is a sweep metric.
- **Retrospective judgment** uses local conditions *relative to national* ones. It weighs more on executives and on towns that dominate their region.
- **Party attachment** is mover–stayer: a durable share plus a slowly updating share. Partisans discount blame aimed at their own side.
- **Turnout** = duty + resources + habit + election importance − cost + mobilisation. Constitutions gain an election-timing field (concurrent or standalone).
- **Campaign contact** mainly moves awareness and turnout. Its persuasion decays (half-life of about 2 months) and has diminishing returns, so canvassing alone cannot be a dominant strategy.
- **Polls** = truth + a shared per-election bias + a per-pollster house effect + sampling noise. Error shrinks as election day approaches.
- **NPC AI:**
  - Utility scoring uses content-defined considerations and response curves.
  - Selection is a seeded weighted-random choice among the top three, and it can be traced.
  - Acting against personality is discounted and costs Standing or Health.
  - Coalition-builders aim for a margin above the threshold and counter-lobby.
- **Migration** uses a gravity model with distance decay: `(1 + d/d₀)^−γ`, γ ≈ 1.2.
- **Deferred:** parameterising demography by fertility rate and life expectancy (M7 balance pass).

### DEC-072 — Elections, parties, and NPC AI implementation choices (M4)

- **Status:** Accepted (agent technical decision during M4, 2026-11-18; within delegated technical authority). Implements DEC-027 to DEC-030 with the refinements of DEC-071.
- **Parties** are generated after the society exists:
  - The two majors split the electorate at the weighted median of its first principal axis (competitive under plurality by construction).
  - Minor parties come from the tail of the next, cross-cutting axes.
  - Each party has two wings (factions) that set its platform.
  - Names are composed from content word lists keyed to the platform's most distinctive attitude.
- **The player is seated after the burn-in,** in the most marginal major-party ward of their town (M6's draft replaces this), so history's elections shape the council they join.
- **Election timing** comes from the constitution (`elections` per tier: date, cycle, campaign length, concurrency, partisan ballot). Local elections are on 3 November in years divisible by 4 (concurrent with national ones), so the first election falls in the player's first year.
- **Vote model:**
  - Per-cohort logit over candidates; turnout is modelled separately.
  - A strategic wasted-vote penalty implements Duverger dynamics.
  - Hidden per-race shocks and a national swing are drawn from keyed streams when the campaign opens.
  - The result is the expectation including those shocks, so results are deterministic and fair.
  - Every player-era result stores a six-factor decomposition: issues, party, candidate quality, record, campaign, luck. History-era results skip it for speed.
- **Polls** see the current electorate plus part of the late shock. They add a per-race shared error, house effects, and sampling noise. NPCs plan from polls (or party strength before the first poll), never from the truth.
- **Turn order:** NPC turns run before the calendar advances, like player commands. The player's town gets the full AI every turn; other towns get legislative AI every fourth turn and abstracted campaigning. During the burn-in, NPCs legislate every six months.
- **Utility AI is content data** (`politics.jsonc` → `ai`): considerations over ten engine inputs, three curve types, weights, and three profiles (ideologue, pragmatist, populist). Choice is a seeded weighted pick among the top three.
- **Party discipline** in council votes: +0.15 lean for the sponsor's party, −0.08 for others. Sponsors always vote for their own bills, and a mayor signs their own.
- **Career:** the player files to defend their seat, run for mayor (needs Standing ≥ 40), or stand down. Losing leaves the player out of office (no powers) but still playing. Results move Mandate, Viability, and Standing.
- **Funding:** each budget year adopts every enacted programme, pro-rated above 25% of the town budget (DEC-024).
- **Bot tournament** (`mandate-cli bots`):
  - six strategies play the first election across seeds;
  - CI runs 8 seeds and the nightly runs 32;
  - the gate is "best win rate − median ≤ 0.3" and "effort beats idle".
- **Performance:** a medium world generates (with 20 years of politics) in about 0.85 s natively and 1.2–3 s in the browser. Optimisation is tracked for M6's performance pass.
- **Deferred:**
  - regional and national elections (the bodies exist in the constitution);
  - primaries;
  - debates and endorsements;
  - NPC favour calls;
  - the budget-procedure screen;
  - the "Why did they do that?" NPC inspector (traces are available in the engine).

### DEC-073 — Every decision is a trade-off

- **Status:** Accepted (owner direction, 2026-11-18: "all decisions … should have some trade-off that makes it feel more like a decision").
- **Rule:**
  - Every player choice has a real cost and a real benefit.
  - No option is free, and none is best in every situation.
  - Costs come from a small set of currencies: **time** (slots), **money**, **health**, **Standing** (party), **goodwill** (a group of voters or a colleague), **legal risk**, and **risk of failure** (shown as odds).
  - The UI states each option's costs and risks *before* the player commits.
- **Applied in M4:**

  | Decision | Gains | Costs / risks |
  |---|---|---|
  | Canvass | The deepest contact: name recognition and turnout in one neighbourhood; lasting | **2 slots** (a long day), −4 health; slow; diminishing returns |
  | Town hall | Reach plus a spotlight on a bill | 1 slot; hostile-crowd risk where you or the town hall are unpopular (odds shown) |
  | Press | Town-wide reach | 1 slot; gaffe risk falling with oratory (odds shown) |
  | Fundraise | Money, +Standing | 1 slot, −2 health; low-income voters cool on you; late at night adds legal risk |
  | Ads | Thin reach everywhere with **no time** | Money; fades fast; little turnout; at most 3 a turn |
  | Rest | Health | 1 slot |
  | Lobby / late night / trade favour | A colleague's vote | Slot; failure costs their opinion; late nights cost health and add legal risk; favours become debts |
  | Your council vote (new) | Party Standing (loyalty), ward approval, repaying favours | Defying your party costs Standing (−6); your ward reacts to your stand; voting against someone you owe is remembered; abstaining pleases no one |
  | Riders | Ward project buys a vote; sunsets and pilots make bills easier to pass | Ward projects cost money and irritate other wards' councillors; sunsets expire; pilots are small |
  | Run for mayor | A bigger office | You give up your council seat; a bigger, riskier race |

- **Policies too:**
  - Every policy template declares simulated side effects (`side_effects` in content). There are three primitives:
    - **disruption** while it is built (lowers the town's appeal and residents' approval);
    - **grievance** among a group it hurts (approval of the government and the sponsor, by class and age);
    - **labour supply** (people work less or more).
  - Content validation rejects a policy whose only cost is money, unless one of its mechanisms carries a built-in downside (a wage floor's job losses).
  - The catalogue and "Why?" panels list each policy's trade-offs with a size in words.
- **Action costs are data:** `tuning.jsonc` → `action_costs` (slots, and health where negative values are recovery). NPCs pay the same costs (DEC-030).
- **Health is a resource:** below 35 health the player has one fewer slot, and below 15 two fewer. Recovery comes from rest and slow monthly recovery.
- **Verification:** the bot tournament (DEC-072) must show mixed approaches beating single-lever strategies, with no dominant strategy. Each new action added to content must list its costs here.
- **Deferred:** NPC health and exhaustion (NPCs remain symmetric in slots and odds, but their health is abstracted).

- **Honest odds (owner, 2026-11-19).** The owner asked whether shown probabilities are real.
  - Shown and rolled odds come from the same function at the same moment: town hall hostility, press gaffe, lobbying, the shady-act exposure chance (rolled once at the act; the director shifts only the date), charges and conviction.
  - The formulas' inputs are design values tuned in the lab, not real-world statistics.
  - A **calibration gate** (M6 task L1) checks that, over many bot runs, events happen as often as the odds shown before them said, within statistical noise.

### DEC-074 — Saves across app updates (revises DEC-044 and DEC-070)

- **Status:** Accepted (owner question, 2026-11-18: "is there a risk … a web update to the engine would make it not deterministic?").
- **Save after every action:**
  - The game is saved in full after every player command (about 90 ms) and at every turn end.
  - The command journal and its replay are removed (Dexie schema v3 drops the table).
  - Nothing is ever replayed, so an update cannot re-roll an outcome the player has seen. Ironman integrity is unchanged.
- **Loading under new rules:**
  - A save from another content or engine version loads if every rule it refers to still exists (the constitution and every policy template).
  - The new rules apply from that day, and a pinned "the game was updated" event records it.
  - A save that needs removed rules is refused with a message naming them. A future migration step can map them instead.
- **Determinism** is guaranteed within one (engine, content) version and tested: two runs, plus save, load, and continue, natively and in WASM.
- **Save-format changes:** before release, they still reject old saves with a clear message. From M6 (testers), format changes ship with JSON migrations.

### DEC-075 — Extensibility: registries over hard-coded dimensions

- **Status:** Accepted (no objection; scheduled before M5). Stages 1–3 and the extension guide are done (below); pack patches (item 5) are not built yet. Owner question, 2026-11-18: the game should stay easy to expand after release, for example with new policy types, industries with their own quirks, and new demographic groups such as Indigenous communities).
- **Already data-driven:**
  - policy templates and their parameters, costs, and side effects;
  - constitutions (bodies, offices, procedures, election calendars);
  - party naming;
  - election, poll, and campaign tuning;
  - NPC AI considerations and profiles;
  - action costs;
  - names and all text (Fluent).
  - Content packs load in order with schemas and validation.
- **Still fixed in code:**
  - 8 attitude dimensions, a fixed array in cohorts, politicians, parties, and policy signatures;
  - 8 economic sectors (an enum with arrays sized 8);
  - 12 cohorts (4 age bands × 3 income classes);
  - neighbourhood kinds and hazard types (enums);
  - policy mechanisms and side-effect primitives (closed enums by design, since new behaviour needs code).
- **Proposal** (an extensibility pass before M5, while there are few saves and little code depends on the fixed shapes):
  1. **Sectors as a registry:** sectors are content entries with traits (tradable, cyclicality, wage level, resource input, automation exposure, pollution, seasonality, unionisation). Code iterates the registry instead of 8 fixed slots. "Quirks" are trait combinations plus optional typed behaviours (for example `boom_bust` for extraction, `seasonal` for tourism, `public_funded` for health).
  2. **Communities as a cohort dimension:** cohorts are keyed by age × class × community. Communities are content: region shares, attitude offsets, turnout, language, institutions and rights hooks (for example a treaty council as a constitution body). The base pack ships one community, so there is no cost now; an expansion adds Indigenous nations or immigrant communities.
  3. **Attitudes as a registry:** the attitude list moves to content and is stored as vectors, so an expansion can add an issue (for example land rights). Signatures name attitudes by key, not by position.
  4. **An extension guide** (`docs/development/EXTENDING.md`) covering three kinds of change:
     - pure content: a new policy, sector, community, attitude, constitution, or language;
     - a code extension point: a new mechanism, side-effect primitive, action, or sector behaviour, each a single enum variant plus one handler and a test;
     - save-format migrations.
  5. **Pack patches:** an expansion pack can add to or override entries from earlier packs by id (today duplicates are an error).
- **Cost:** a one-time refactor touching the economy, population, politics, and protocol, plus a save-format bump. It is done before testers arrive (M6) so post-release saves never need that migration.
- **Stage 1 implemented (sectors as a registry, 2026-11-18):**
  - The closed `SectorKind` enum became `SectorDriver`, the coded behaviours: `land_based` (agriculture), `extraction` (resources), `industrial` (manufacturing), `building` (construction), `local_services` (retail), `visitor` (tourism), `knowledge`, and `public`. Code branches on a sector's driver; several sectors may share one.
  - Sectors are content entries (`content/base/sectors.jsonc`) with `driver` and the existing traits. Validation requires unique ids, sane traits, and at least one sector each for `building`, `local_services`, and `public`. The other drivers are optional.
  - The registry order is load order: files by path, then item order. The engine indexes sector data densely in that order, with `Vec`s sized to the registry instead of `[_; 8]` arrays.
  - Content, commands, events, and effects name sectors by content key: `sector_subsidy.sector`, `LabSectorShock`, `PlantClosure`, `SectorShock`, and active shocks. Map views list sectors from the registry, with `id` and `name_key`.
  - Saves record `economy.sector_keys`. `Engine::from_state` remaps sector data by key when the content's registry differs. Matching keys keep their data. A new sector starts with no jobs and a capacity from its endowment, so it grows in over the following months. A removed sector is reported by `missing_references`, and the save is refused.
  - The save format moved to v6. Base-content results are unchanged: sweep metrics, bot win rates, and the economic trace are identical.
- **Stage 2 implemented (attitudes as a registry, 2026-11-18):**
  - Attitudes are content entries (`content/base/attitudes.jsonc`), each with an `id` (`attitude.redistribution`), a short `key` (`redistribution`), and a `seed` model: `by_class` [low, middle, high], `by_age` [four bands], `by_neighborhood` (per neighbourhood kind, omitted = 0), `regional_sd` (default 0.14), and `noise_sd` (default 0.08). The numbers that `seed_attitudes` hard-coded moved into content unchanged. The short keys are kept, so Fluent keys (`conviction-pro-<key>`, `conviction-anti-<key>`) and party-name words keyed by attitude still work.
  - Registry order is load order, as for sectors. The engine stores attitudes as `Vec<f32>` in that order: cohorts, politicians, party platforms, and factions.
  - Policy `signature` is a map from attitude key to weight (`{ "environment": 0.6, "localism": 0.1 }`, omitted = 0). At load it is resolved to a dense vector in registry order (`PolicyTemplate::signature_weights`), so hot loops never look up keys.
  - Code that reads a specific issue looks it up by key (`ContentBundle::attitude_index`). The Office card's safety, environment, and taxes concerns read neutral if their attitude is absent. Member convictions and party stances iterate the registry.
  - Validation requires at least one attitude, unique ids and short keys (`[a-z][a-z0-9_]*`), finite seed values, both conviction Fluent keys for each attitude, and known attitude keys in signatures and party-name words.
  - Saves record `population.attitude_keys`. `Engine::from_state` re-keys every attitude vector when the content's registry differs. Matching keys keep their values. A new attitude starts without randomness:
    - cohorts at their seed model's mean for their class, age, and neighbourhood kind (no regional offset or noise);
    - politicians at the turnout-weighted view of their home neighbourhood;
    - party platforms and factions at neutral (0).

    A removed attitude is reported by `missing_references`, and the save is refused.
  - The save format moved to v7. Base-content results are unchanged: sweep metrics, bot win rates, and the economic trace are identical. Decoded states differ only in the content hash and the new `attitude_keys` field.
- **Stage 3 implemented (communities as a cohort dimension, 2026-11-18):**
  - Communities are content entries (`content/base/communities.jsonc`) with `id`, `name_key`, an optional `share` model, `attitude_offsets` (by attitude key, resolved at load to a dense `CommunityDef::attitude_offset_weights`), `turnout_offset`, and `immigration_share`. The base pack ships one, `community.general`.
  - Exactly one community omits `share`; it takes each neighbourhood's remainder. Every other community's share of each age × class group in a neighbourhood is `base + by_neighborhood[kind] + regional offset`, clamped to [0, 1]. The regional offset is one draw per region (sd `regional_sd`) from a stream keyed by the community id. Shares that add up to more than 1 are scaled down, with a validation warning.
  - Cohorts are age × class × community, laid out community-major (`cohort_index(age, class, community)`). With one community the layout is the old age × class grid. Code reads a cohort's dimensions through `age_of`, `class_of`, `community_of`, and `group_of`, never index arithmetic. Per-group aggregates for the economy and finance sum over communities (`municipality_groups`, indexed by `group_index`).
  - Generation splits each age × class group by share, with exact integer accounting, and seeds each community's cohorts with its offsets. With one community it makes no extra draws, so the cohorts are identical to before.
  - Births stay in the parents' community. Children are tracked per community in `HoodPop::children_by_community`, which stays empty, and is not serialised, while a state has one community. Ageing, deaths, class mobility, migration, opinion, elections, polls, and party attachment all run per cohort, and movers keep their community. International immigrants join communities by `immigration_share`; the remainder community takes the rest.
  - Validation requires at least one community and exactly one without `share`. Share, offset, and turnout values must be finite and in range, offsets must name known attitudes, and `name_key` must exist in English.
  - Saves record `population.community_keys`. `Engine::from_state` re-lays-out cohorts by community id after the attitude remap. Existing cohorts and children keep their community. A new community starts with no people: its cohorts are empty, seeded without randomness (seed-model mean plus offsets, seeded turnout, party attachment from those attitudes). It grows through immigration and births from then on. A removed community is reported by `missing_references`, and the save is refused.
  - The municipality view and the town hall's town data list the communities present, with their share of adults (`CommunityShareView`). The web shows a "Communities" line only when there is more than one.
  - The save format moved to v9. Base-content results are unchanged: sweep metrics, bot win rates, and the economic trace are identical. Decoded states differ only in the content hash and the new `community_keys` field. Medium-world generation time is unchanged.
  - Deferred: party-affinity offsets, language, and community institutions or rights hooks (for example a treaty council body). An extension guide is in `docs/development/EXTENDING.md`.

### DEC-076 — Difficulty target: rare wins, rich stories

- **Status:** Accepted (owner, 2026-11-18: "Most people don't win a roguelike, but we don't want it to be brutally hard or uninteresting … if we generate interesting stories most of the time and rarely the user wins, that is probably a good goal." Tuning waits until the simulation is complete).
- **Targets** (default difficulty, measured by full-career bot players and later by testers):

  | Measure | Target |
  |---|---|
  | Full win (Mandate renewed, DEC-006) | Rare: about 5–10% for a competent player, higher on easier presets; never 0% on any preset |
  | Brutal early end (run over within Act 1's first election cycle) | Under 15% |
  | Story-rich runs | At least 80% of runs reach several distinct beats: an election won and lost, a law that changed the town, a crisis or scandal, a rival or ally, a comeback or a fall |
  | Where runs end | Most in Acts 2–3, with a spread of endings; no single terminal ending above 50% |
  | Skill matters | The best strategy clearly beats random play, and no strategy dominates (DEC-072) |

- **How difficulty is reached:** through honest levers only (DEC-038): world conditions, storyteller tension, NPC competence, and information quality. NPCs never get hidden bonuses.
- **Doing nothing should rarely work** (owner, 2026-11-18). Implemented in three parts:
  1. **A record of visible work.** Each politician has a record (0–1) that fades 8% a month and grows with bills (+0.12), town halls and canvassing (+0.06), declared votes (+0.04), lobbying and press (+0.03), and amendments (+0.02). NPCs earn it the same way; NPC officeholders elsewhere drift toward their own diligence. An incumbent whose record is below 0.35 is attacked in the campaign, which costs utility with every voter (×4 per point, not discounted by awareness). A strong record earns a small bonus with voters who know the incumbent.
  2. **Party patience.** A thin record drains Standing (−1.5 a month), and a strong one adds +0.5. Below 25 Standing the party will not renominate the player (a "deselected" event, warned in advance on the Office card).
  3. **Visibility.** The player's name recognition fades faster (4% rather than 1.5% a month) while their record is thin.

  Bots, first re-election, 32 seeds: idle 9%, ads-only 22%, active strategies 75–84%. CI gate: idle ≤ 15%.
- **Harder as you climb** (owner, 2026-11-18; tune with Acts 2–4). Difficulty should rise by tier mostly through structure:
  - bigger constituencies, where personal contact matters less;
  - better-funded and more competent opponents (higher NPC competence and campaign budgets per tier);
  - heavier media scrutiny, so scandals surface more easily;
  - more veto players: legislatures, courts, and coalition partners;
  - national shocks that voters blame on the incumbent.

  Per-tier NPC competence and poll noise are the tuning levers (DEC-038). The target is that each step up wins less often, with the overall win rate landing at 5–10%.
- **When:** metrics are added to the lab (story-beat counter, ending distribution, full-career bots) as Acts 2–4 come online. Gates go into CI and the nightly runs from M6. No tuning toward these targets before then.

### DEC-077 — Run-to-run variety

- **Status:** Accepted (owner, 2026-11-18). Context: the owner asked whether runs will vary enough to stay interesting. The assessment: variety in the world itself (geography, history, parties, shocks, NPCs) is strong. Variety in decisions is the risk: every run opens the same way, with the same policy menu and actions, and an optimal routine is easy to find.
- **Decisions:**
  1. **Town character (M5).** Each town takes on a character that the player sees and feels, derived from simulation state rather than authored per run:
     - a defining industry (from the sector registry, DEC-075);
     - a live local crisis (for example a plant at risk, a housing squeeze, a flood-prone district, or a fiscal hole);
     - a power broker (an organisation or a named NPC with a stake);
     - a named rival;
     - local priorities that change which issues voters weigh.

     The town's policy emphasis and NPC agendas follow from it.

     **Implemented (M5b, 2026-11-18):** `characters::town`, recomputed monthly and stored per town: the defining industry (share × location quotient, public sector aside), live crises from a crisis registry (`crises.jsonc`: indicator, begin/end hysteresis, severity, trend; base: jobs disappearing via the new `job_loss` indicator, disaster recovery, housing squeeze, fiscal hole), a power broker (the organisation leader with the biggest stake in the industry or crisis, else the senior officeholder), the player's rival (recomputed when they file), and the top three concerns. NPCs rank bills that answer the live crisis and the town's concerns higher (`npc.crisis_agenda`, `npc.concern_agenda`). A Town card on the town screen shows it.
  2. **Varied openings (M6, with the draft of three).** Candidates differ in situation as well as background:
     - challenger or incumbent;
     - a safe or a marginal seat;
     - an inherited scandal;
     - a town in crisis;
     - a hostile or a friendly council;
     - a party in or out of power.
  3. **Situational policies (M5).** Some policy templates unlock only when circumstances call for them: a closure-response package after a plant shuts, a rebuild fund after a disaster, an emergency rent measure in a housing crisis. They are content templates with unlock conditions, validated like all policies (DEC-073 trade-offs apply).
  4. **Dilemma events (M5, through the director, DEC-033).** These are moments where every option costs someone. They are built from live state, warned in advance, and explained afterwards.

     **Implemented (M5c, 2026-11-19):** `dilemmas.jsonc` (12 base dilemmas, 3 storyline turns, 3 requests): roles cast from live actors, triggers on live state, visible with every option's trade-offs from the day they appear to their deadline, then the default; every option has at least one cost and one benefit (validated); effects are engine changes; the aftermath lists what each did.
  5. **A variety check in the lab (M5).** Beyond "no dominant strategy" (DEC-072), the bot tournament reports the best strategy per world. It must differ across worlds and situations, so that no single routine wins everywhere. It becomes a CI gate alongside the DEC-076 story targets.

     **Implemented (M5d part 2, 2026-11-19):**
     - **§3 Situational policies.** A policy template may carry `unlock`, a `SituationTrigger` (a reusable typed condition on a town's situation, for DEC-092 policy families later): any of a live crisis (`crises.jsonc`, min severity), an indicator range, a plant closure in the event log within N months, or a disaster within N months. Locked templates are listed under "Not on the table now" with the reason, cannot be drafted (by the player, an NPC, or a dilemma's draft effect), and NPCs consider them only while unlocked; bills and laws already made stay. Base templates: a closure response package, a disaster rebuild fund, an emergency rent measure, a fiscal bridge levy, each with simulated trade-offs and listed among its crisis's answers. Unknown crisis ids warn (the condition never holds), like dilemma triggers.
     - **§4 Bill and favour dilemmas.** Six more: a vote bought with a ward project in the player's own bill, a sponsor's rider, the whip's deal, calling in a favour owed to the player, a favour offered for a vote, and a creditor who wants the rival's bill sunk. Two new effects: `rider` (a ward project, optionally for a cast councillor's ward, or a sunset, on a cast bill) and `call_in` (a favour owed to the player repaid as a traded favour's lean on a cast bill).
     - **§5 The variety gate.** `mandate-cli bots` reports, per world, the best active strategies by vote share (strategies within 0.2 points share the world), and by starting situation. Gate: at least 2 strategies are the sole best somewhere, and none is among the best in more than 60% of worlds. Enforced with the other bot gates in CI (16 seeds) and nightly (32). Passing it took tuning (BALANCE_LOG 2026-11-19): ads persuade less (`ad_persuasion_share` 0.25 → 0.05) and a thin record costs more (`record_penalty` 4 → 5).
- **Dependencies:** DEC-032, DEC-033, DEC-036, DEC-073, DEC-075, DEC-076.

### DEC-078 — Data-driven tuning and further registries

- **Status:** Accepted (owner, 2026-11-18). This extends DEC-075, after the owner asked what else should become data while the game is small.
- **Context:** besides the attitudes, sectors, and cohorts covered by DEC-075, the simulation code holds about 1,200 numeric literals. Many of them are balance numbers inside formulas: opinion weights, action effect sizes, risk odds, council lean weights, the rate a record fades. Several fixed lists are also enums.
- **Decisions:**
  1. **Tuning numbers move to content, one system at a time.** These are the numbers we expect to tune: opinion and retrospection weights; action effects and risks; council lean weights and lobbying odds; record, Standing, and recognition dynamics; and then economy formula coefficients. Structural constants (midpoints, unit conversions) stay in code. Each system gets a parameter block in content with a schema and validation.
  2. **Lab parameter overrides.** `mandate-cli` sweep and bot commands accept overrides such as `--set politics.vote.party_weight=1.6`, and grid sweeps over one or two parameters, so DEC-076 targets can be tuned from data.
  3. **Concern registry (now).** Voter concerns (jobs, housing, safety, environment, taxes, …) become content. Each concern names its indicator (an engine-provided measure such as unemployment, rent burden, crime, pollution, or tax burden), the attitudes behind it, and how it feeds issue salience in the vote model (which today weighs every issue the same). Town character (DEC-077) builds on it.
  4. **Scenario registry (now).** The five world presets become content scenario definitions. These are biases on geography, economy, era, and politics, so new starting worlds are content.
  5. **Before national play (M7):**
     - budget categories, revenue and tax types become registries (the national tier adds categories such as defence and foreign affairs);
     - hazards become a registry with behaviour drivers, like sectors (a heatwave, an epidemic);
     - commodities are derived from extraction-driver sectors.
  6. **With the walkable town screens (M6):** place kinds become a registry carrying art and interaction data. (The registry, with each kind's actions and services, came forward to M5a under DEC-079; art and interaction data remain M6.)
  7. **Kept in code:**
     - the pressure tracks and core rules (they are the game's identity);
     - income classes and age bands (communities, DEC-075 stage 3, provide the flexible dimension);
     - the kinds of player action (their costs are already data, and a new kind is a small coded extension point).
- **Order:** finish DEC-075 stages 2–3, then decisions 3, 4, 1 (starting with politics, opinion, and actions), and 2. Then decision 6 with M6 and decision 5 before M7.
- **Decisions 1–4 implemented (2026-11-18):**
  - **Decision 1 (first systems).** Politics, opinion, action, council, career (record, Standing, recognition, Mandate), and NPC balance numbers are content parameter blocks: additions to `politics.jsonc` and new `opinion`, `actions`, `council`, `career`, and `npc` files, each required once, with a generated schema and range validation. Player and NPC lobbying now share one formula. Structural constants stay in code. Base-content results are identical (normalised state hashes, sweeps, bots, econ trace). Not yet moved: candidacy odds in elections, opening values in worldgen, the slot rules (ads per turn, favour debt, tiredness thresholds), and the economy's coefficients (the next system).
  - **Decision 2.** `mandate-cli sweep` and `bots` take `--set <path>=<value>` and up to two `--grid` parameters (one summary row per combination). Paths name a content kind and then a field, or a registry entry by id (`concerns[concern.jobs].level.slope`). Unknown paths and type mismatches are refused.
  - **Decision 3.** Concerns are a registry (`content/base/concerns.jsonc`): jobs, housing, environment, taxes, public services, and rebuilding after a disaster, each reading one engine indicator (unemployment, its rise, rent burden, pollution, tax burden, budget stress, a recent disaster). There is no crime in the simulation yet, so there is no safety concern. Each town's concern levels and per-attitude salience (mean 1) are recomputed monthly and stored in the population state. The vote model's issue term weighs each attitude by it (`politics.vote.salience_base` = 1.5 after tuning). Towns' top concerns appear in the town panel, pocket, town data, and overheard talk. Saves record `concern_keys`; content that adds or removes a concern recomputes them.
  - **Decision 4.** The five presets are scenarios (`content/base/scenarios.jsonc`) with geography, economy, era, and politics biases. Each gives the same world as before for the same seed. Saves record the scenario id (format v10, with a migration from v9). The title screen lists the registry.

### DEC-079 — Places as the interface: go somewhere to do something, with convenience

- **Status:** Accepted (owner, 2026-11-18: "instead of … a bunch of menus … they would actually go somewhere to do something, like going to town hall to vote, or somewhere to get higher-fidelity town and country data … balance so there is some convenience and information they can always get, so it's not busy work"). This brings DEC-009's located actions forward from M6 into M5.
- **The player has a location:** a place in their hub (Act 1: their town). Places already exist in the generated world. Their kinds become a registry (DEC-078 decision 6, moved to M5) carrying each place's actions, information, opening hours (later), and art and interaction data (M6).
- **Actions happen at places:**

  | Where | What you do there |
  |---|---|
  | Council chamber | Declare votes, introduce and amend bills, see the full whip count |
  | Town hall (mayor's office, clerk) | Policy drafting, budget papers, filing to run |
  | A colleague's office, or the diner | Lobby, trade favours (late-night meetings) |
  | Streets of a neighbourhood | Canvass |
  | Community centre | Town halls |
  | Newspaper office | Press statements, buying ads |
  | Party office | Fundraising, internal polls, Standing with the party |
  | Statistics office or library | Detailed town and country data, trends, history, past results |
  | Diner, market, union hall | Overheard concerns: qualitative early warnings |
  | Home | Rest |

- **Convenience rules (no busywork):**
  1. **Moving within the hub is free:** it costs no slot. Only actions cost time. Visiting another town costs a slot (travel).
  2. **Auto-routing.** The always-available agenda lists every action. Choosing one takes the player there automatically (a short, skippable walk), then performs it. Players who like to wander can walk; players who don't, never have to.
  3. **The pocket** (phone and notebook), always open, holds:
     - date, slots, tracks, and money;
     - your bills and the calendar;
     - messages and alerts;
     - headline numbers in coarse form (rounded, last month's).

     Nothing needed to avoid a terminal ending is ever only at a place (DEC-006 fairness, accessibility).
  4. **Places give depth, not access.** Going to the statistics office gives precise, current, by-neighbourhood data with trends. The party office gives internal polls. The council chamber gives the exact whip count. The diner gives early, qualitative signals. A visit rewards curiosity; skipping it costs precision, not playability.
  5. **Deep information can cost something** (DEC-073). Commissioning a private poll or a research report costs money and a slot, and gives better precision than public data.
- **Interface staging:**
  - **M5:** an interim town screen. A schematic district of tappable buildings with the player's marker replaces the current tabs. The tabs' content moves into the places and the pocket. Engine and protocol work in terms of places from here on. (Shipped in M5a as neighbourhood sections with place buttons; DEC-086 replaces it with a town map overview before M6.)
  - **M6:** the walkable pixel-art hub replaces the schematic, on the same engine model.
  - **Early M5:** a throwaway hub rendering prototype on a phone (one street block, tap-to-move character, ambient townsfolk, one enterable place) to settle Canvas versus PixiJS and performance before M6 builds on it (DEC-050).
- **Fairness and accessibility:** every place's information and actions are also reachable from a list (screen readers, no map). The schematic and the hub are the primary way to play, never the only one.
- **Implemented (M5a):**
  - **Registry:** `content/base/places.jsonc` (content kind `places`), one entry per place kind keyed by the snake-case kind (`key`), plus the pseudo-locations `home` and `streets`. Each entry lists `actions` (action keys: the tuning action-cost keys plus the free `vote`, `draft`, `file`, `withdraw`), `late_night` (actions also allowed in the late-night slot there), `services` (`whip_exact`, `town_data_detailed`, `budget_papers`, `internal_polls`, `overheard`), and an optional `fallback` kind. Validation checks keys, actions, services, and fallbacks, that `home` and `streets` exist, and that every action is offered somewhere; `mandate-cli content validate` and a test also check that every worldgen `PlaceKind` has an entry. Worldgen keeps its enum; the registry supplies behaviour.
  - **Mapping:** council chamber: vote, introduce, amend (bills), withdraw, lobby, trade favours, exact whip. Town hall (clerk, mayor's office, statistics desk): draft (and amend drafts), file, withdraw, detailed town data, budget papers. Community centre: town hall meetings. Newspaper: press, ads. Party office: fundraise (also late), internal polls. Diner: lobby and favours, also late at night; overheard. Market and union hall: lobby and favours (daytime), overheard. Home: rest. Streets of a neighbourhood: canvass. Other kinds offer nothing yet (M6).
  - **Fallbacks:** a neighbourhood without a community centre holds its town hall meetings on its streets; a town without a newspaper (rural towns) talks to the press and buys ads at the town hall; the statistics desk is at the town hall (there is no statistics office kind yet). Worldgen now gives every town a party office in its core (no RNG draw, so simulation outcomes are unchanged); the party office also falls back to the town hall.
  - **Location:** `PlayerState.location: Location { Home, Street { neighborhood }, Place { place } }` (default `Home`; home is the first neighbourhood of the player's ward). `MoveTo` is free and limited to the player's town (travel to other towns is rejected for now). Every player command is validated against where the player is (`go to the <kind> first`). Votes, introducing, and amending bills: council chamber; drafting (and amending drafts), filing, withdrawing: town hall (withdrawing also in the chamber); lobbying: chamber, diner, market, or union hall by day, the diner late at night; town hall meetings: a community centre in the target neighbourhood, else its streets; canvassing: the target neighbourhood's streets. Costs and effects are unchanged.
  - **Auto-routing:** `Go { location, then }` moves and acts in one command (one log entry). If the command fails the move stands, and the engine logs it so a replay reproduces the location. `PocketView.routes` gives, for every action (and late variant, and neighbourhood for canvassing and town halls), the best location: where the player is if that works, else the first matching place in town. Bots use the same `Go` (via `Engine::route`); win rates are unchanged.
  - **Information depth:** the engine decides precision. Away from the chamber a bill's whip is "about N of M" (rounded, no per-member chances or lobbying odds). Town opinion and the home town's economy are rounded (5 points, whole percent, no sectors, budget, or trend) except at the town hall. Internal polls (party office) are the model's expected shares plus noise (SD 1.5 points) from a keyed `Polling` RNG per week; views never advance a stream. Overheard lines (diner, market, union hall) are Fluent keys for the top two concerns of the place's neighbourhood. New queries: `Pocket` (`PocketView`), `Town` (`TownPlacesView`), and `Place { location }` (`PlacePanelView`; the names avoid clashing with the existing map `PlaceView` and office `TownView`).
  - **Interface:** the town screen (neighbourhood sections with place buttons) replaces the tabs; tapping a place walks there and opens its panel; the pocket drawer holds the coarse figures, alerts, bills, and the agenda (auto-routed, with a skippable walk of at most 600 ms), and links to the map and national overview.
  - **Deferred:** NPC locations (DEC-030 symmetry for location), travel between towns, other towns' detailed data (still public on the map), opening hours, commissioned polls (rule 5).

### DEC-080 — Hub renderer: Canvas 2D

- **Status:** Accepted (owner, 2026-11-18: "Canvas 2D is fine if you think we can still make the final game beautiful with it"). This revises the provisional PixiJS choice in DEC-009 and OVERVIEW §10. Evidence: the hub spike (DEC-050) on branch `spike/hub-render`, `web/spikes/hub/RESULTS.md`.
- **Findings:**
  - Frame cost depends on pixels drawn, not sprite count. Scene script time stays under 3 ms per frame with 150 townsfolk.
  - Canvas 2D held 40–48 fps at full device resolution and 60 fps at CSS resolution on an emulated mid-range phone (CPU slowed 4×).
  - PixiJS adds 145 KB gzipped (more than the whole app), with twice the time to first frame and twice the memory, plus WebGL context-loss handling.
  - The test machine has no GPU, so the spike's PixiJS frame rates are not representative.
- **Decision:**
  - The M6 hub uses Canvas 2D, like the strategic map: static layers pre-rendered, sprites from one atlas, the whole view redrawn per frame.
  - Draw at CSS resolution or lower and let the browser upscale with `image-rendering: pixelated`.
  - Zoom snaps to whole-number steps.
  - Pause when nothing moves, when a panel is open, or when the page is hidden; consider a 30 fps cap for ambient scenes.
  - Text and UI are HTML over the canvas (sharp, translatable, accessible).
- **Beauty:** the look is carried by the art itself, which Canvas draws cheaply. Day and night grading, light pools, weather particles, animated tiles, and palette moods are pre-rendered or drawn at pixel-art resolution. Heavy per-pixel shader effects (bloom, dynamic lighting, distortion) would need WebGL. If wanted, they are added later as a separate effects layer, not by switching renderers. The renderer stays behind a small interface.
- **Revisit if:**
  - a real phone misses 30 fps with Canvas at CSS resolution;
  - M6 needs per-pixel lighting or weather shaders;
  - scenes exceed about 1,000 moving sprites.
- **Real-device check:** the spike is published unlinked at `/mandate/spike-hub/` (`?r=canvas|pixi&n=20|60|150&res=device|css|low&light=dusk`) for the owner to try on a phone. The app's service worker excludes `/spike-` paths.
- **Real-device result (owner, 2026-11-18):** Canvas 2D "worked well" on the owner's phone. The owner found the routes to places "a little strange". The causes in the spike are 4-neighbour A* (staircase zig-zags), road tiles costing four times as much as pavement (long detours to avoid short crossings), and taps snapping to the nearest walkable tile (which can be behind a building).
- **M6 pathfinding requirements:**
  - 8-direction A* with an octile heuristic and no corner-cutting past obstacles;
  - path straightening (string-pulling with line-of-sight checks);
  - pedestrian costs that prefer pavements and crossings but allow short road crossings;
  - a tap on a building routes to its door, and a tap on open ground goes to the nearest reachable cell on the player's side;
  - smooth sub-tile movement with interpolation.
  - a test that a route between two points with open ground between them is visibly direct: no more turns than the obstacles force, and a length within a small factor of the straight line.
- **M6 zoom requirements** (owner, 2026-11-18: the town "jumps" when zooming; fixed in the spike):
  - a pinch zooms about the midpoint between the fingers, so the point under them stays put;
  - when one finger lifts or a pointer is cancelled, the gesture re-baselines from the remaining finger instead of jumping;
  - limits are eased in when zoomed past them, never snapped.
- **M6 movement module (status, 2026-11-18):** the pathfinding and zoom requirements above are built as a renderer-independent module, `web/src/hub/` (`pathfinding.ts`: 8-direction A*, pedestrian costs, string-pulling, tap-to-target, smooth movement; `camera.ts`: pinch, pan, wheel, eased limits), with unit and seeded property tests. The hub spike uses it for routing and the camera, so `/mandate/spike-hub/` shows the new routes. The app does not import it yet: M6 will.
- **M6 groundwork (status, 2026-11-19):** built in `web/src/hub/`, not yet imported by the app:
  - `townLayout.ts`: the player's town laid out from the engine's `town` view (neighbourhoods and places) as a tile map with streets, contiguous districts, a door for every place, filler buildings, landmarks on a central square and optional water; deterministic and versioned (DEC-085). It feeds `buildGrid`, `planRoute` and `resolveTap` directly. Town routes add a small straightening slack and a jog-removal pass (`pathfinding.ts`, off by default), so a route cuts a road corner instead of stepping round the zebra cells.
  - `render/`: the renderer as decided above. Ground, buildings and props are baked in 512 px chunks on first sight (kept in an LRU, so memory follows the view, not the town); shadows are baked per sun-angle bucket at half resolution; walkers, roof overhangs over walkers behind them, the grade, lit windows and lamp pools are drawn per frame; identical frames are skipped. The art is behind `ArtProvider` (`render/art.ts`), shaped after the pixel-art tool plan (`PIXEL_ART_TOOL.md` §3.5, §5.1): ground tiles by kind, neighbour mask and variant; buildings as albedo plus windows and doors with emissive frames and use tags, and a shadow footprint and height; props with emissive parts, light pools and shadows; 16×24 characters in three facings (side mirrored); `grade(minuteOfDay)`; SVG icons. `PlaceholderArt` implements it in code, from a palette derived from the app's tokens, so the tool's art pack drops in without renderer changes.
  - `dayCycle.ts`: a continuous clock (minutes) graded through one keyframe table (night, pre-dawn, sunrise, morning, midday, afternoon, golden hour, dusk), with shadows that follow the sun and windows and lamps that come on one by one. In the game the clock comes from the day-part and the player's progress through it (`clockForDayPart`).
  - `hubView.ts`: the DOM controller (canvas at CSS resolution, HTML labels, camera input, a loop that sleeps when idle, 30 fps for ambient-only frames, paused when hidden or a panel is open).
  - The hub spike's main page is now this town on the real engine (`/mandate/spike-hub/?seed=`); the original renderer comparison moved to `spike-hub/compare.html`. Measurements: `web/spikes/hub/RESULTS.md`.

- **Battery (owner, 2026-11-19).** Townsfolk kept the street drawing at 30 fps whenever it was on screen, at device resolution. Agreed fixes (W3):
  - idle sleep: after about 20 s without input, townsfolk pause and drawing stops, and any input wakes them;
  - ambient townsfolk at 15 fps (the player and the camera stay smooth);
  - the loop pauses while the pocket sheet covers most of the street on a phone, or the map overlay is open;
  - a Playwright budget test with a throttled phone CPU measures idle CPU use on the street view.
  - Later, with Settings: a battery saver (fewer townsfolk, no ambient animation, lower resolution), on automatically with reduced motion.
### DEC-081 — Logo and loading animation

- **Status:** Accepted (owner, 2026-11-18: "I agree b for logo c for loader", choosing from four concepts: ballot box, rising poll line, career-ladder seal, pixel capitol).
- **Logo:** a pixel-art M drawn as a stepped poll line. The right peak rises higher than the left and ends in a flag: a career with a setback on the way up. It is a 16×16 grid drawn with crisp edges in the accent gold, with the flagpole in the text colour and the flag in `--logo-flag` (#eb6834). The wordmark stays Georgia. The mark appears above the title on the title screen. App icons (`web/public/icons`: SVG, 192, 512, and a maskable 512 with the mark inside the safe zone) are rendered from the same drawing.
- **Loader:** a ladder whose four rungs light from the bottom (Local, Regional, National, Presidency), one per act. It runs on a 4-second cycle and replaces the old spinner everywhere. The full version, with tier labels, is used while the engine starts; a compact one without labels is used for in-screen waits and while a world is generated.
- **Rules:**
  - Both follow the light and dark themes through `--accent` and `--text`.
  - Under `prefers-reduced-motion` the ladder is drawn fully lit and still.
  - The loader announces itself as a status. Its tier names are decorative, with English fallbacks because they show before the translations load.
  - Component: `web/src/components/Brand.tsx` (`LogoMark`, `Loader`).
- **Open:** the owner may still send their own logo and loading animation. If so, they replace these through the same component, and the icons are re-rendered.

### DEC-082 — Save export and import

- **Status:** Accepted (owner, 2026-11-18: "I don't think we need to be that strict on Ironman, it's a game"). The light rule below stands; the tombstone option is declined.
- **Context:** DEC-052 keeps file export and import as the manual fallback for moving saves until a sync provider exists. Players need to back up a run, move it to another device, or restore it after clearing the browser. An import is also a way to put an old save back, which would undo Ironman (DEC-008).
- **File:** one `.mandate` file per run, named `mandate-<town>-<YYYY-MM-DD>.mandate` (or `mandate-run-<id>-…` before the town is known). It is UTF-8 text in three lines:
  - the magic line `MANDATE RUN EXPORT`;
  - a JSON header: file format (1), app version, export time, and the run's metadata (run id, mode, status, revision, town name, saved-at time, save format version, engine version, content hash, turn, game date), plus the save's length and CRC-32;
  - the engine save bytes exactly as stored (DEC-044 rev. 2: versioned, compressed, BLAKE3-hashed), in base64.

  Text was chosen over a binary frame so a file survives e-mail, chat apps, and cloud drives that mangle unknown binaries, and so its header can be read by eye. Base64 adds about a third to the size (saves are small).
- **Validation, in order:** the magic line (otherwise "not a Mandate file"); the header format (a newer one: "made by a newer version"); the header shape, length, and checksum ("damaged or incomplete"); then the engine loads the save through the normal worker path, under the current rules (DEC-074). The engine's errors map to "newer version", "damaged", or "can't be loaded under the current rules". Nothing is stored until all of this passes.
- **Import rule:**
  - A run with no local counterpart (same run id) is added as it is.
  - A file whose revision is greater than or equal to the local revision replaces the local run.
  - An older revision of an **Ironman** run that exists here is refused with an explanation: Ironman runs cannot go back in time. If either the file or the local run is Ironman, the rule applies.
  - An older revision of a **Story** run is added as a separate run (a new run id), next to the current one.
- **Trade-off:** this stops casual rewinding, not all cheating. A determined player can keep copies of files, delete the local run, and import an old copy, or clear the browser first. This matches DEC-052 ("sync is not anti-cheat"): the game has no leaderboards, and the rule exists so a player cannot undo a bad election with two taps, not to police them. A stronger option, not implemented, keeps a tombstone of the highest revision seen for each deleted Ironman run and refuses older imports of it too. It costs a little storage and still fails after clearing site data. **Declined by the owner:** Ironman protection stays light. It stops accidental or casual rewinding, not deliberate workarounds.
- **Phones:** export downloads through a Blob, an object URL, and `<a download>`. On touch devices that can share files, the share sheet is offered first, with the download as the fallback. The file picker has no `accept` filter, because phones grey out unknown extensions. Desktop browsers also take a file dropped anywhere on the title screen.
- **Where:** `web/src/persistence/runFile.ts` (format and rules, over any `SaveStore`), `web/src/components/RunTransfer.tsx` (browser wiring), on the title screen's saved-runs list. There is no in-game export yet: the pocket has only "Save & quit", and the run can be exported from the title screen after it.
- **Dependencies:** DEC-008, DEC-044, DEC-052, DEC-074, DEC-048, DEC-051.

### DEC-083 — Landing page at the site root; the game moves to `play/`

- **Status:** Accepted (owner, 2026-11-18: "It can go live. We'll want to update once we have actual towns to walk around so it's closer to the real final gameplay"). Requested by the owner: a public page that sells the game to new players without looking like a template site. Research: `docs/design/research/LANDING_PAGE.md`. After review the copy was rewritten in plain language (the first draft read as machine-written), and a desktop screenshot was added beside the phone ones.
- **Follow-up (owner):** refresh the page's screenshots and copy when the walkable pixel-art town lands (M6), so the page shows gameplay close to the final game. Re-run `web/landing/scripts/capture.mjs`, `encode.py` and `og.mjs`, and update the hard-coded election-night figures.
- **Decision:**
  - The site base serves a **landing page**; the game is served at **`<base>play/`**. `MANDATE_BASE` stays the single deploy variable (DEC-064): `/mandate/` gives the landing page at `/mandate/` and the game at `/mandate/play/`; `/` gives `/` and `/play/`. The hub spike stays at `<base>spike-hub/`.
  - The landing page is plain HTML, CSS, and about 1 KB of TypeScript, with no framework and no web fonts (Georgia and the system font, as in the app). It has its own Vite build (`web/landing/vite.config.ts`) into `dist/`; the app builds into `dist/play/`. `npm run build` runs both; `npm run preview` serves the whole site.
  - **Concept:** a broadsheet front page from a generated country, in the game's palette. The dark theme is the night edition, the light theme the morning edition. Every image is captured from the running game (`web/landing/scripts/capture.mjs`, `encode.py`, `og.mjs`): the generated maps, the town and council-chamber screens, and a real election night, each captioned with its seed.
  - **Honesty rules:** no testimonials, ratings, player counts, press quotes or signup. What is built and what is coming are labelled as such (the career ladder lights only the local rung).
  - **One action:** "Play in your browser", a plain link to `play/` that works without JavaScript. If this browser holds an active saved run, the page also offers "Continue your run" (it reads the game's IndexedDB read-only and never creates the database; the space is reserved, so nothing shifts).
  - **Absolute URLs** (canonical, `og:url`, `og:image`) need an origin: `https://jackhale98.github.io` for the default base, or `MANDATE_ORIGIN` for a custom domain. Without one, those tags are omitted and the share image is root-relative.
  - **Budgets:** landing HTML, CSS, and JS at most 60 KB gzipped (checked in CI; about 12 KB today); the LCP image at most 150 KB (49 KB AVIF, 69 KB WebP); no layout shift (tested); tap targets of at least 44 px and text contrast of at least 4.5:1 in both themes (tested).
- **Migration for installed players:**
  - Before this change the app's service worker lived at `<base>sw.js` with scope `<base>`, answering every navigation under the base with the cached app shell. The new build puts a small **retiring worker** at that same URL (`web/landing/public/sw.js`). When a returning browser checks for an update it installs at once, deletes only the caches made for its own scope (the game's caches at `play/` share the origin and are kept), unregisters, and reloads the pages it controlled. They then get the landing page. vite-plugin-pwa's `selfDestroying` was not used because it deletes every cache on the origin.
  - The app's new worker is at `<base>play/sw.js`, scoped to `play/`.
  - The manifest keeps the old **`id`** (the site base, which was the old `start_url`), so browsers that update installed apps can move an existing install to `play/` instead of treating it as a new app. An installed app that still opens the site base in standalone mode is sent on to `play/` by the landing page.
  - Saved runs are unaffected: IndexedDB belongs to the origin, not the path.
  - **Tested** (`web/e2e/sw-migration.spec.ts`): a stand-in for the old root-scoped worker gives way to the landing page, its caches go, the game's caches stay, and the game registers its own worker at `play/` (runs in CI). A one-off test against a real build from before this change (install the old app, save a run, deploy the new build, continue the run from the landing page) passed locally before the move shipped; it was retired on 2026-11-19 once the move was live.
- **Consequences:** links shared to the old base now land on the landing page, one tap from the game. `<base>sw.js` must stay in the build for as long as old installs may exist. The old installed icon opens the base, which forwards to `play/` (online); for offline use from the home screen, players should add the game again from `play/`.
- **Dependencies:** DEC-048, DEC-049, DEC-051, DEC-052, DEC-064, DEC-079, DEC-081, DEC-082.

### DEC-084 — Pixel-art tool

- **Status:** Accepted (owner, 2026-11-19: "I approve the plan"). Plan and research: `docs/design/research/PIXEL_ART_TOOL.md`.
- **Decision:** the game's pixel art comes from a tool that **composes hand-drawn parts with code**, coloured from designed palette ramps. Code selects, layers, recolours, outlines, autotiles and lays out facades. It does not invent shapes. All of the plan's recommendations (D1–D15) are accepted:
  - **D1:** characters are 16×24 frames in 4 directions, confirmed or overturned by the spike's readability check. Tiles stay 16 px (DEC-065).
  - **D2:** an automatic 1 px dark-tone outline on people, vehicles and props; no outline on tiles and buildings.
  - **D3:** our own palette, at most 128 colours, built in OKLCH and anchored on the brand tokens (DEC-081). Skin ramps span the Monk Skin Tone scale.
  - **D4:** no AI-generated pixels in shipped art; mood boards only.
  - **D5:** agents draw the parts for the spike. If the spike's rubric items 1 or 2 fail, a pixel artist is commissioned for the MVP part library (about 500 parts).
  - **D6:** the tool lives in `tools/pixelkit/` (working name) behind a hard boundary, and moves to its own repository later (P3).
  - **D7:** the tool is licensed MIT OR Apache-2.0.
  - **D8:** Mandate's own art is all rights reserved; a CC0 sample pack ships with the tool.
  - **D9:** the tool is written in TypeScript; the appearance registry and genome live in Rust content.
  - **D10:** characters and buildings are composed on the device from engine data; tiles, props and icons are built ahead of time.
  - **D11:** portraits are sprite-bust crops in M6; dedicated 64×64 portraits are decided after seeing the town.
  - **D12:** the night look (multiply grading only, or palette moods) is decided from the spike, which renders both.
  - **D13:** parked cars only in M6.
  - **D14:** no community-specific appearance by default.
  - **D15:** timing. The validation spike (P0, about 6 tasks) runs before M6 art integration, and the MVP (P1, about 16 tasks) is part of M6. This replaces the "after v1" roadmap note.
  - **D16:** the tool's name is open until the spin-out.
- **Rules:**
  - The same appearance parameters always produce the same sprite. Art changes are versioned, so a known character does not change look mid-run unless intended.
  - Assets carry emissive layers and shadow footprints for the continuous day cycle; lighting stays pre-rendered (DEC-080).
  - Quality control combines a style guide, automated lints, golden images, and contact sheets reviewed by the owner.
- **Dependencies:** DEC-009, DEC-013, DEC-018, DEC-031, DEC-049, DEC-051, DEC-065, DEC-077, DEC-080, DEC-081.

- **No commissioned artist for now (owner, 2026-11-19: "it's unlikely I hire an artist unless this game gets popular").** All art comes from the pixel-art tool: hand-drawn parts composed by code, made in our own art rounds. Items once listed as "an artist could add" become art-round tasks: rock strata, curved track and switches, paving laid along diagonals, kerb stones, a diagonal level-crossing set, and a drawn mine. Commissioning or buying art is reconsidered only if the game finds an audience. Packs from third parties are not mixed in, to keep one coherent style and clean licensing.

### DEC-085 — Town layout on the client, versioned so a town never rearranges

- **Status:** Accepted (owner, 2026-11-19: "I'm fine with the versioned town layout"). Proposed with the M6 groundwork. M6 must reserve plots for places that can be added during a run, so a town never rearranges mid-run.
- **Context:** The walkable hub (DEC-009, DEC-079) needs a street plan with a building for every place. The engine knows which places and neighbourhoods a town has, but not where they stand. Walking is UI state (DEC-009), so the plan does not affect outcomes, but players will learn their town: it must look the same every time they open it, on every device, and after app updates.
- **Options:**
  - A. The engine generates the plan (Rust, in worldgen) and stores it in the save.
  - B. The client derives the plan from the engine's town view, the world seed and a layout version, and keeps old versions callable.
  - C. As B, but the plan is cached in the app's save store after the first visit.
- **Proposed decision:** B, built in `web/src/hub/townLayout.ts`.
  - `layoutTown(input, seed, version)` is a pure function: keyed seeded streams (no `Math.random`, no clock), no `Math.hypot` or other functions that differ between JavaScript engines, input sorted by id so the engine's listing order does not matter.
  - `LAYOUT_VERSION` (now 1). A change that moves anything for the same input bumps it, and the previous algorithm stays callable for runs that started with it. Tests pin map hashes of committed engine fixtures, so an accidental change fails CI.
  - The app records the layout version with each run when it first shows the town (in its own run metadata, not the engine save), and lays the town out with that version from then on.
  - The town seed is derived from the world seed and the town's name (`townSeed`), independent of the version.
- **Rationale:** No engine, protocol or save-format change; the plan is cheap to compute (about 15–25 ms for a town, 25 ms for the largest capital on a desktop, under 80 ms on a phone slowed 4×); the art and layout can iterate quickly on the client. A keeps the plan in one authoritative place but puts presentation into the simulation and the save. C adds storage and a cache to invalidate for little gain.
- **Consequences and risks:**
  - The plan depends on the town's place list. If the engine later adds or removes places during a run (new businesses, closures), the plan would rearrange. Before that happens, the layout needs reserved lots for later places, or the plan must be frozen (option C or A).
  - Old layout versions accumulate as code. They are small, and could be retired at a save-format break.
  - Other clients (a native app) would have to port the algorithm bit-exactly, or switch to A.
- **Dependencies:** DEC-009, DEC-041 (its ideas, applied on the client), DEC-044, DEC-074, DEC-079, DEC-080.

- **Pre-release (owner, 2026-11-19: "for town generation we don't need backward compatibility yet since this isn't released").** Until the first public release:
  - Town layouts need no compatibility. The generator may change freely; a development run may show a rearranged town after an update; pinned layout hashes are updated as needed.
  - Layout v1 is retired when v2 becomes the game's default (after the street rework and the town views, B2). The game's town map moves to v2 at the same time.
  - Per-town recipe snapshots are not needed.
  - **At release,** the rule above applies in full: the shipped layout version is frozen, recipe snapshots go into run metadata, and later changes ship as a new version that old runs never see.
  - Save-format migrations for simulation state are unaffected by this note.

### DEC-086 — Town map overview

- **Status:** Accepted (owner, 2026-11-19). The owner's idea: "instead of showing a list of places like we have now we should consider a method for generating a lower resolution map of the area with the connections shown and the buildings listed. This would make it feel more like a game and less like a job and make you care more about the places since you can see how they're connected." Owner: "Yes add it to the queue."
- **Decision:**
  - The town screen's main view becomes an illustrated, low-resolution **town map** drawn from the same versioned layout as the walkable street (DEC-085), so the two always agree.
  - It shows neighbourhoods as tinted districts, the main roads and the routes between places, landmarks as small recognisable buildings with names, your home and current location, and markers on places where something is waiting (a decision, a person to meet).
  - Tapping a building travels there (the existing located-action rules, DEC-079). In M6 the map zooms into the walkable street.
  - The place list stays as an accessible text view (DEC-051) and for screen readers; nothing is reachable only through the map.
  - It is presentation only: no engine or protocol change. It ships before M6's walkable town, as soon as the UI polish round and M5d part 1 have landed.
- **Later:** the same treatment for the region and the country in M7 (towns joined by roads and rail, for travel).
- **Dependencies:** DEC-047, DEC-049, DEC-051, DEC-079, DEC-080, DEC-084, DEC-085.
- **Superseded in part (2026-11-19, DEC-088 "Main screen"):** the main screen is now the walkable street. The town map opens as a full-screen overlay from the bar (tap a place to walk there), and the place list is the pocket's Places tab. The Map | List tabs and the "Walk the streets" button are gone. The notes below describe the map itself, which is unchanged.
- **Built (status, 2026-11-19):** the town screen opens on the map; the list is its "List" tab.
  - **Scene** (`web/src/hub/townMapScene.ts`): one hotspot per entry of the town's place list, in the list's order (home, the streets of each neighbourhood by its name, every place). Places with nothing to do yet (the list's "Also here") are hotspots too; tapping one says "nothing to do here yet", as the list does. Main routes are the streets from the central square to every hotspot (one street-preferring, few-turn shortest-path tree); the same trees give the route from where you are to any place. Pure and deterministic; it does not depend on where the player stands, so moving never re-bakes.
  - **Renderer** (`web/src/hub/render/townMapRenderer.ts`, Canvas 2D): the layout baked once per town and layout version, 4–8 px per tile (at most 1,400 px a side): ground one pixel per tile scaled up natively, then textures over runs of cells (ripples, furrows, tufts, paving), zebra crossings, shores, neighbourhoods as soft tints with dotted borders, the main routes, filler blocks as quiet roofs and walls, trees, and every named place as a map-scale landmark (the town hall's clock tower, the chamber's dome, the market's canopies, the diner's awning and sign, the newspaper's broadsheet, the party office's bunting, the union hall's banner, the clinic's cross, the school's belfry, the factory's sawtooth roof and chimney, the port's containers and crane, and so on) in the pixel-art pack's palette (`townMapPalette.ts`, frozen from the pack's ramp anchors). A second layer of lit windows and lamps fades in at dusk. Per frame: one scaled blit, the day-cycle grade at 0.5 strength (morning to evening as the turn's slots are used, then evening or late night), the route being previewed or walked, and markers drawn as shapes (a pin for you, a house for home, a speech bubble for something waiting). Idle maps draw no frames.
  - **View** (`web/src/hub/townMapView.ts`, `web/src/components/TownMap.tsx`): names are HTML chips placed without overlaps, most important first (you, home, waiting, neighbourhoods, civic landmarks, the rest; quiet places only when zoomed in), above a landmark's tallest part or else below it. Every hotspot is a real button (Tab follows the list's order, arrow keys move to the nearest place in that direction, Enter goes there, `+`/`-`/`0` zoom); its label carries the place, its kind, neighbourhood, "You are here" and what is waiting. The canvas has a short label naming the List view as the text alternative. Camera: `HubCamera` (anchored pinch, re-baselined fingers, eased limits); the fitted zoom frames the whole town, zooming fully out centres it, a resize keeps the top-left point (or refits if untouched) and brings the last place gone to back into view. Tapping a place calls the store's `visit` (as the list's buttons) and shows the player walking there along the streets (at most 600 ms; instant with reduced motion); auto-routed agenda actions walk on the map too. Waiting markers: pending decisions whose scene is at a place, and the chamber when a vote is yours to cast (from the pocket view). People to meet are not marked: no view says where people are without opening a place.
  - **Layout version:** recorded per run on the device when the town is first shown (`mandate.townLayoutVersion.<run>` in localStorage) and used from then on; it is not yet in the synced run metadata.
  - **Map | List** (`web/src/components/TownViewTabs.tsx`): an ARIA tab list; the choice is remembered per device. The town card stays above the map on phones and beside it when the town column is wide enough (a container query, so a docked place panel moves it above). UI strings not yet in the content pack fall back to English (`townMapText.ts`).
  - **Timings** (Playwright, Pixel 7 profile with the CPU slowed 4×, shared machine): bake 32–54 ms for the player's town (about 100×65 tiles) and 50–56 ms for the largest capital (258×157) from cold; the layout itself (DEC-085) takes 118–143 ms for that capital under the same load. Frame draw 0.2–1.4 ms while panning; no frames when idle.

### DEC-087 — Foreign nations and foreign policy

- **Status:** Accepted (owner, 2026-11-19). The owner asked to plan for foreign policy: "simulate neighbors as well with randomized constitution types and allow for more robust interaction in the future if it isn't a major overhaul." Owner: "Record the foreign policy decision."
- **Decision:**
  - **Neighbours are separate, low-detail nations,** not a multi-country rebuild of the engine. The home country keeps its full simulation (cohorts per neighbourhood, towns, politicians). Each neighbour is simulated at national level only, at about the cost of one town today, in a new foreign domain beside the existing ones (writing only its own domain; cross-domain effects go through effect requests, OVERVIEW §4.3).
  - **Generated with the world:** 2–5 neighbours with randomly drawn constitutions from the constitution registry (DEC-078 content), a ruling party and leader, population, a national economy (growth, unemployment, inflation), stability, an attitude vector, and a relationship with the home country. The greyed land drawn around the country on the map (UI polish round) becomes these real nations, with names and borders.
  - **How they reach the player, by tier:**
    - Local: foreign investment opening or closing a plant, migrants after a neighbour's crisis, border-town trade.
    - Regional: cross-border trade and shared rivers and hazards.
    - National and presidency: trade deals and tariffs, treaties, disputes, aid, alliances, summits, and defence and foreign-affairs budget lines (DEC-078 §5 budget registries).
  - **Same rules as everything else:** foreign shocks (a neighbour's recession, a refugee wave, a trade war) are exogenous events the director may time within plausible bounds (DEC-033 B); it never fakes outcomes. NPC foreign leaders use the same utility AI and rule symmetry as NPC politicians (DEC-030).
- **Timing:**
  - Now: nothing; the current structure doesn't block it.
  - M7: generate the neighbours and their low-detail simulation; show them on the map; cross-border trade and migration start to matter.
  - M8: foreign policy as play (deals, treaties, crises, the foreign minister, summits).
  - After v1: deeper diplomacy; possibly starting a career in another country.
- **Dependencies:** DEC-018, DEC-030, DEC-033, DEC-041, DEC-064, DEC-078, DEC-086.

### DEC-088 — Enterable places and walking as the default

- **Status:** Accepted (owner, 2026-11-19). The owner asked whether players should "actually enter buildings to interact with them and the people inside". Owner: "For the enterable places I'm good with your recommendations. For accessibility maybe we make it an option to turn on once we have the town walkable but I think my default preference would be for people to actually walk to places and talk to people."
- **Decision:**
  - **Key places can be entered.** Inside is a small, single-screen interior: a room with furniture, the people present standing where they work, and hotspots for the place's actions (the clerk's counter, the chamber's benches, the diner's booths). Talking to someone means walking up to them.
  - **Interiors are data-driven.** Each place kind names an interior recipe in content (room size, furniture kit, where people stand, which action sits at which hotspot). Like the town layout, the interior is generated on the client from a keyed seed and a versioned recipe (DEC-085), and it is presentation only. Who is present and what they can do still comes from the engine's place view (DEC-079); the UI decides nothing.
  - **M6 ships three interiors:** the chamber, the town hall and the diner. Other places keep the place panel until their interiors are added. A missing recipe always falls back to the panel.
  - **Walking and talking is the default.** Once the town is walkable (M6), the default is to walk to places and talk to people.
  - **A quick mode is an option to turn on** in Settings. It gives the town map plus place panels, with no walking, and the List view stays for screen readers (DEC-051). Both modes reach every action. The quick mode is also offered on the first run.
- **Dependencies:** DEC-047, DEC-049, DEC-051, DEC-079, DEC-084, DEC-085, DEC-086.
- **Main screen (owner, 2026-11-19: "The main screen default should really show the town for you to walk around in so you can optimize screen real estate. The town info should probably go in the pocket. The pocket ui should have tabs").**
  - The game's main screen is the walkable street view, full screen, with a slim bar: a map button (the town map, DEC-086, as a full-screen overlay; tapping a place walks there), the date and slots, and End turn.
  - The town card moves into the pocket.
  - The pocket gets tabs: **Today** (slots, actions, decisions, alerts, votes), **Town** (the town card, gated by knowledge, DEC-091), **Places** (the place list, every place reachable without walking: the accessible path, DEC-051, and quick mode's home), and **People** (who you have met; it later joins the notebook, K5). Tabs keep their last choice and show badges. On desktop the pocket can stay docked open beside the town.
  - A skip link and landmarks reach Places and the pocket without the canvas.
  - **Map overlay toggle (owner, 2026-11-19):** the overlay has a "Town | Country" switch. Country shows the national map (layers, your town, municipality panels) in the same overlay, so it is never hunted for. The last choice is remembered. Travel between towns comes with M7.
### DEC-089 — Map layers, with some gated by where you are

- **Status:** Accepted (owner, 2026-11-19: "Yes to DEC-089"). The owner's idea: "make that button a dropdown so users can filter by different metrics … resources, population, income, rent, voting statistics etc but maybe have some details gated based off where they are like a polling location or town hall."
- **Proposal:**
  - The map's layer picker (built: terrain and unemployment) grows into a registry of layers. Each layer has a key, a source in the map view, a colour scale (sequential or diverging) and a legend. New layers are content plus one engine field.
  - **Open layers** come from public statistics: population, density, unemployment, median income, rent, the main industries and resources.
  - **Gated layers** need you to have been somewhere. The last election's results by town become available once you have visited a polling place or the town hall on election day, or read the newspaper's results. Party support and polling become available after commissioning a poll, or as a party member from the party office. The engine decides what you know (the knowledge already used for hidden facts, DEC-034). A gated layer shows its date ("as of March, from your poll"), and is greyed out with the place to unlock it while you don't know it.
  - **The UI never computes these figures.** The map view carries each layer's values, or "unknown", per town.
- **Timing:** the open layers in the M6 UI round; the gated layers with M7's regional play.
- **Dependencies:** DEC-034, DEC-051, DEC-079, DEC-086.

### DEC-090 — Town layout v2

- **Status:** Accepted (owner, 2026-11-19: "Yes to … the town gen recommendations"). The owner asked for towns that are "real but readable": bridges and creeks, mines, open-air markets, hills; size that scales with population and the simulation; links to gameplay; hidden or optional places; data-driven district types that the pixel art can keep up with.
- **Decision:** the design in `docs/design/research/TOWN_GENERATION.md`, with all twelve of its §16 recommendations:
  - D1: a terrain-first hybrid generator on the tile grid.
  - D2: a static plan, plus a monthly dress pass that never moves anything.
  - D3: small engine views (`TownGeographyView`, `TownStateView`).
  - D4: registries for district types, terrain features, connectors and place layout blocks, with a recipe snapshot per town (option C).
  - D5: orthogonal streets.
  - D6: size from log(population), capped at 240 × 160 tiles.
  - D7: hidden places, starting with four kinds in M6.
  - D8: per-neighbourhood hazard exposure.
  - D9: a higher mine threshold, with a balance check.
  - D10: names that follow geography.
  - D11: palette headroom in pixel-art P1.
  - D12: built at the start of M6. The town map (DEC-086) ships on v1 and moves to v2.
- **Versioning:** v1 stays callable for runs that started with it (DEC-085).
- **Dependencies:** DEC-051, DEC-053, DEC-075, DEC-078, DEC-079, DEC-084, DEC-085, DEC-086, DEC-087.
- **D5 amended (owner, 2026-11-19: "Go with A + B").** Stage A's towns still read as grids: every street pattern was rectangular subdivision. Sketches compared three options (`town-gen-sketches/streets-*.png`).
  - **A:** streets grow from the square along desire lines to the bridges, gates and neighbouring towns, then side streets branch. This gives irregular L and T blocks, T-junctions and offset crossings, mixed widths (high streets wide, old-core lanes 1–2 tiles), greens at junctions, and cul-de-sacs with turning heads.
  - **B:** A plus 45° streets anywhere terrain or radial roads call for them, with buildings staying square and stepping along the diagonals. Wedges left by diagonals become greens, yards or small triangular plazas.
  - **Accepted:** A + B, as layout v2 (no saved run uses v2 yet), with diagonal road and pavement art.
  - **The owner also reported connection faults:** roads starting from the wrong side, roads not joining straight, stubs ending in nothing, near-parallel duplicate roads, roads meeting rail with no crossing, rail looping round the centre. Fixing these is part of the same task, with checks.
  - **Curved roads (C)** stay for a later version with the art round.
  - **Street ends (owner, 2026-11-19: "roads have terminations instead of just ending abruptly").** Only the declared links (roads to neighbouring towns), rail, rivers and the coast cross the map edge. Every other street ends in a junction, or in one of these terminations:
    - a turning circle or hammerhead;
    - bollards with a footpath beyond;
    - paving into a square or market;
    - a field gate or farm track at the countryside;
    - a railing at water.

    The generator records each (`TownMap.streetEnds`), the art draws them, and the checks reject a street that just stops.

### DEC-091 — Knowledge is earned

- **Status:** Accepted (owner, 2026-11-19). The owner proposed that players "visit places and people to start learning the information such as who cares about what, who the power broker is, what policies are … this incentivizes exploration in the beginning of the game and as new zones are achieved like moving to the capital", balanced with accessibility. The owner then answered the open points: "Standard menu known, background-based start, local knowledge option off by default."
- **Decision:** extends DEC-079 ("places give depth, not access") from precision to identity. Some things are now learned, not just made more exact.
  - **Levels:** each subject is unknown, then heard of (a rumour or a name), then known (met or seen), then detailed (from the right place or person). Subjects include:
    - the town's concerns by neighbourhood;
    - the power broker;
    - councillors' and other people's goals, stances and favours;
    - the rival's plans;
    - who supports a policy;
    - situational and proposed policies;
    - hidden places (DEC-090 D7);
    - gated map layers (DEC-089).
  - **Never gated:** the rules, your own state, what an action does and its trade-offs (DEC-073), deadlines, anything aimed at you (decisions, stories, investigations), and the standard policy menu.
  - **Ignorance costs effectiveness, never fairness.** Canvassing on an issue a neighbourhood doesn't care about does less. Lobbying someone whose goals you don't know works less often.
  - **The starting knowledge follows your background.** Someone who grew up in the town knows its main worries and the council by name, but not who really runs things. A newcomer knows little. Moving to a new zone (the region, the capital) starts local knowledge again, and party contacts give a short primer.
  - **No busywork:**
    - You learn through actions, not movement, so the List view and quick mode (DEC-088) learn the same things.
    - Most learning is a side effect of what you'd do anyway: canvassing, council, the market.
    - The first visit to each place in a new town includes a short introduction.
    - Identities never fade. Numbers carry their date.
  - **The notebook:** the pocket gets a "what you know" page, listing each thing with where and when you learned it. It also lists leads ("the union hall would know who's behind the plant deal"), so finding things is guided, not hunted.
  - **The "Start with local knowledge" option** is offered at setup next to quick mode, and is **off by default**.
  - **Engine:**
    - A per-player knowledge ledger in the player domain (ordered maps, DEC-041) and part of the save.
    - Protocol views hide what the player doesn't know. The UI never decides it.
    - It reuses the knowledge model of hidden facts (DEC-034).
    - Bots learn by the same rules. A lab gate requires that exploring early pays off, but that exploring alone never wins.
- **Timing:** the first M6 engine task, alongside town layout v2 (DEC-090), after M5d part 2 merges.
- **Dependencies:** DEC-034, DEC-051, DEC-073, DEC-077, DEC-079, DEC-088, DEC-089, DEC-090.
- **The world does not wait for the player (owner, 2026-11-19: "we shouldn't stop things from happening just because the user doesn't know about it yet. It's on them to investigate the town").** Knowledge gates only what the player sees and how well the player acts. It never gates what happens.
  - Crises build, NPCs pursue goals and draft bills, policy-family proposals appear and expire, storylines advance, scandals surface and hazards strike, whether or not the player has learned of them.
  - The `knows` condition may choose how something is framed for the player: which scene variant, which line, whether a lead is offered. It may not decide whether an event, decision, proposal or storyline step happens. Validation rejects `knows` in dilemma, storyline and family triggers.
  - Things aimed at the player stay never-gated (above). Something the player hasn't learned of still happens, and its consequences reach them like anyone else's.
  - **Lab guard:** with the same commands, an omniscient player and a default player produce identical worlds, except for the ledger and the player's own action outcomes (K4). Bots that take no knowledge-dependent action (idle) must match exactly. The test runs with the knowledge tests.

### DEC-092 — Policy families grounded in the town

- **Status:** Accepted (owner, 2026-11-19). The owner asked whether the policies are "boiler plate … too boring", because learning and finding them would only be interesting if they differ between runs: "if they're the same every time that would feel like a grind." Owner: "Yes, record DEC-092 and put it in M6. We should likely look at having these data file driven as well so we can easily add more, expand, or tweak the families later."
- **Context:** the base pack has 11 local templates. Each is one slider applied to the whole town, and they are the same in every run.
- **Decision:**
  1. **The standard menu stays, small and always known** (DEC-091). It holds the general levers: transit, housing, zoning, grants, policing, parks, the hardship fund, property tax.
  2. **Policy families bind to the town.** A family is a reusable template with **slots** that bind to live entities when its conditions hold:
     - a place (the plant, the bridge, the market);
     - a neighbourhood (the flood-prone district);
     - an organisation or a person (the plant's owner, the union);
     - a sector, a crisis or a hazard.

     For example, "Rescue deal for {plant}", "Flood wall for {neighbourhood}", "Reopen {bridge}", "Licences for {market}". Each town produces different instances, and each run draws a different subset of families. A binding is chosen by a keyed RNG stream from the run seed and the family id (DEC-041). Instances are stored in the run, so they stay stable.
  3. **Design choices instead of one slider.** A family has two or three **choice axes**, each with options that carry their own mechanisms, side effects, cost and signature (DEC-073):
     - where it applies;
     - how it is paid for (bond, tax, a levy on developers, cuts elsewhere);
     - who delivers it (the council, a contractor, a charity).

     Sliders remain where a quantity is the real choice. The engine computes every option's cost and support (the policy preview query of M5d part 2). The UI never does.
  4. **Proposals come from people.** A family can name **sponsors** by role (the union rep, the business association, a residents' group). Each brings a pre-set version and a price (a favour, a vote, a public endorsement). Meeting the sponsor is how the player learns the proposal exists (DEC-091). Adopting a sponsor's version builds the relationship; amending it costs some goodwill.
  5. **Powers follow the constitution.** A family states the tier and the power it needs. Where the town lacks that power (DEC-078 constitutions), the instance becomes something to lobby the tier above for. This feeds the career ladder.
  6. **Situational policies become families.** The unlock conditions built in M5d part 2 become family triggers.
- **Everything is data:** the owner's requirement.
  - Families live in a content registry, `content/<pack>/policy_families/*.jsonc`. It is typed in `mandate-content`, with schemas generated from the types (DEC-053) and loaded in pack order, so mods can add families or patch them.
  - In content, a family declares:
    - its id and Fluent keys, with slot arguments;
    - its tier, power and category;
    - **slots**, each with a binding kind and a selector: place kind, neighbourhood tag, hazard, sector, crisis, organisation type, character role, and conditions on indicators;
    - **triggers** (conditions for an instance to appear) and **expiry**;
    - **choice axes and options**, each with mechanisms, side effects, cost, signature and secondary keys;
    - **sponsors** (role, their pre-set options, their price);
    - knowledge (where and from whom it is learned, DEC-091);
    - weight and rarity for drawing per run.
  - **Code only for new kinds:**
    - a new binding kind (a selector over a new entity type);
    - a new mechanism type;
    - a new price type.

    These are typed extension points listed in EXTENDING.md. Everything else (new families, new options, retuning, new sponsors) is a content edit.
  - **Validation (`mandate-cli content validate`):**
    - slots reference real kinds and tags;
    - every option has at least one cost and one benefit (DEC-073);
    - every family can bind in at least one generated test world;
    - Fluent keys exist with the right arguments;
    - sponsor prices are valid.
  - **Lab tools:**
    - `mandate-cli policies --seed <s> [--town <id>]` lists the instances a world would produce, with bindings and computed costs;
    - a **coverage report** over N seeds shows how often each family appears, its median cost and support, and which families never bind;
    - `--set` overrides (DEC-078) reach family parameters;
    - the bot variety check (DEC-077 §5) counts which families winning strategies use.
- **Timing:** M6, after the knowledge ledger (DEC-091). It depends on per-neighbourhood effects (DEC-090 D8).
  - A first pack of 15–20 families, including the M5d part 2 situational policies converted.
  - The 11 standard templates are trimmed to the general levers.
  - Saves gain policy instances with a migration.
- **Need-aware reactions (owner, 2026-11-19).** The owner asked whether a housing bill matters less in a town with many empty homes. Today a bill's anticipated reaction is need-blind value alignment, and only its outcomes (rents, jobs, services) respond to the town's state, without diminishing returns.
  - Each policy and family option names the concerns it **addresses**.
  - A group's reaction scales with how bad those concerns are in its neighbourhood (a base level plus a weight × the level).
  - A waste term ("why spend on this?") applies from tax-averse groups when need is low, scaled by cost.
  - Outcome models gain diminishing returns where they are real: homes above target vacancy, subsidies without spare labour, saturating services, hazard protection where there is no hazard.
  - Previews and "Why?" show the need part.
  - A lab **policy responsiveness** gate checks that each policy's effect differs between high-need and low-need towns.
  - The contract is in `M6_ENGINE_CONTRACTS.md` §2.22.
- **Open questions answered (owner, 2026-11-19): all recommendations** in `M6_ENGINE_CONTRACTS.md` §5:
  - arrival teaches lightly;
  - councillors' names are public;
  - a heard proposal is a stub with a lead;
  - a paid survey at the statistics desk;
  - flood defences, the tech incubator and the wage floor are retired into families (8 standard levers remain);
  - about two-thirds of eligible families per run, at most 4 open;
  - sponsors ask a favour or an endorsement, never cash;
  - lobbying the tier above is deferred to M7;
  - NPCs keep no notebooks;
  - small mod patches are allowed;
  - unknowns show a placeholder with a lead;
  - "Reopen the bridge" is deferred;
  - a conversation costs half a slot, and "say hello" stays free;
  - guardedness follows the relationship;
  - repeat questions give a different line and nothing new;
  - pronouns come with pixel-art P1.
- **Dependencies:** DEC-024, DEC-026, DEC-041, DEC-053, DEC-073, DEC-077, DEC-078, DEC-079, DEC-090, DEC-091.

### DEC-093 — Conversations

- **Status:** Accepted (owner, 2026-11-19). The owner asked how robust sentence generation is, and whether NPCs just stand near the player while information updates, or whether the player actually talks to them. Owner: "Yes, record DEC-093 and add it to the brief."
- **Context:** lines are hand-written Fluent templates with slots, chosen by the engine (DEC-054). They are grammatical, grounded, deterministic and translatable, but variety is limited by how many lines exist. Today talking means three things, and none of them lets the player choose what to say:
  - passive overheard chatter at places;
  - "Say hello" (`Meet`: free, builds the relationship, no dialogue);
  - scripted two- or three-beat scenes for pending decisions (DEC-035).
- **Decision:**
  - **Topics, not dialogue trees.** Talking to someone (walking up to them, DEC-088, or choosing them in the List view) offers topics:
    - what people are worried about;
    - who really runs things;
    - about a subject (the plant, the bridge, a bill, the rival);
    - pitch a policy;
    - ask for support or an endorsement;
    - call in or offer a favour;
    - small talk.

    Topics are offered by role, place, relationship and what the player already knows.
  - **Replies are chosen from content lines by conditions:**
    - the speaker's role, traits and goals;
    - what they know, and what they are willing to tell you, given your relationship and their discretion;
    - their memories of you (quotes, DEC-031);
    - the live facts.

    Conditions use the same reusable condition type as situational policies and policy-family triggers (DEC-092).
  - **Effects:**
    - A reply can teach the player something (the DEC-091 ledger, with this person as the source), open a lead, reveal a proposal (DEC-092 sponsors), or change the relationship.
    - Effects go through effect requests (OVERVIEW §4.3).
    - Each conversation costs time (a slot, or part of one), so choosing whom to talk to is a decision.
  - **Variety by composition:** a reply is up to three parts:
    - an opener, which depends on the relationship;
    - the substance, which depends on the facts;
    - a closer, in the speaker's voice.

    Voices come from traits: a blunt union steward and a smooth developer say the same fact differently. Memory quotes layer on top. A few hundred lines give thousands of distinct, grounded exchanges.
  - **Repetition guard:** a person never repeats a line to you while an alternative exists. Picks are keyed by (person, topic, week) on a keyed RNG stream (DEC-041), with a short per-person memory of lines used. Overheard chatter follows the same rule.
  - **Scenes stay:** decision scenes (DEC-035) remain authored beats. Conversations are the everyday layer around them.
  - **Never prose from the engine:** the engine emits keys and facts. Fluent renders the text, with select expressions for gender, plurals and variants.
  - **Everything is data:**
    - topics, reply lines, voices and openers or closers go in content registries;
    - schemas are generated from Rust types (DEC-053);
    - validation checks that every topic has a fallback reply for every role, that conditions reference real fields, and that Fluent keys exist with the right arguments;
    - a lab command samples conversations for a seed and reports repetition and coverage.
  - **Accessibility:** topics are buttons, replies are text, and the List view and quick mode (DEC-088) reach the same conversations.
- **Timing:** M6, with or right after the knowledge ledger (DEC-091). Conversations are the ledger's main source.
- **Dependencies:** DEC-031, DEC-033, DEC-035, DEC-041, DEC-051, DEC-053, DEC-054, DEC-079, DEC-088, DEC-091, DEC-092.

### DEC-094 — Credit, blame, and endorsements

- **Status:** Accepted (owner, 2026-11-19: "Yes add it to the plan"). Two gaps came up while reviewing how the simulation turns state into votes. First, a law's outcomes reach votes only through general approval of the government (and approval of the player for the player's own bills), so voters don't credit or blame the politicians who passed it. Second, relationships with characters don't move mass votes.
- **Decision:**
  - **Credit and blame for laws.**
    - Each law already keeps an attribution ledger (homes built, jobs supported, and so on).
    - Each month, a law's experienced effects are credited (or blamed) to its sponsor, and with a smaller share to everyone who voted for it (or against it, when the law turned out well and the vote is remembered).
    - Each politician gains a per-neighbourhood **track record** that decays slowly.
    - The vote model's record term reads it for every candidate, not only incumbents, scaled by how aware the voter is of the candidate and by media attention.
    - Need-aware reactions (DEC-092) feed it: a law that met a real need earns more credit.
    - The media (DEC-034) can attach a law to a name ("the Hale rent freeze"), raising awareness of that credit or blame.
  - **Endorsements.**
    - Characters with a following (union leaders, business association heads, community and faith leaders, the power broker, popular local figures) can endorse a candidate or a bill.
    - An endorsement moves the cohorts the endorser speaks for, through a **follower weight** per community, class and occupation (data-driven, per role), scaled by the endorser's standing with those cohorts. It shifts persuasion and turnout, and never more than a cap.
    - The player earns endorsements through conversations (DEC-093: the "ask for support" topic), favours, and policy-family sponsors (DEC-092: adopting a sponsor's version can carry their endorsement). NPCs seek and give endorsements by the same rules (DEC-030).
    - Endorsing has a cost to the endorser: their standing falls with followers who disagree.
  - **Transparency:** the "Why?" breakdown and election night show credit, blame and endorsements as named factors ("The union's backing: +3 in Millside").
  - **Lab:** bots can seek endorsements. A gate requires that endorsements matter (a measurable effect in the races where they occur) but never dominate. Credit and blame must track outcomes: in the lab, sponsors of laws that did well gain votes on average.
- **Timing:** M6, after conversations (C1) and alongside policy families. It is in `M6_ENGINE_CONTRACTS.md` §4 as tasks V1 (credit and blame) and V2 (endorsements).
- **Dependencies:** DEC-025, DEC-027, DEC-030, DEC-031, DEC-034, DEC-051, DEC-073, DEC-092, DEC-093.

### DEC-095 — No decorative attributes

- **Status:** Accepted (owner, 2026-11-19: "Yes add dec-095"). The owner asked whether NPC attributes really affect how NPCs play, or are only drawn from a list.
- **Context:** most attributes already drive behaviour:
  - attitudes: bills, votes and voter perception;
  - pragmatism and ambition: the strategic profile;
  - integrity and ambition: corruption and resisting lobbying;
  - oratory and negotiation: campaign reach, fundraising and lobbying;
  - warmth: how fast opinions change;
  - personality: goals and casting;
  - traits: memory and opinion dynamics.

  Four competencies (administration, organising, intrigue, expertise) are generated and shown but read by nothing. Loyalty and boldness only gate goals and casting, not day-to-day choices.
- **Decision:** every attribute a character has must change what they do, through the same formulas for NPCs and the player (DEC-030). The magnitudes go in content parameter blocks (DEC-078):
  - **Administration:** an officeholder's laws implement faster and cost less to run (sponsor, or the mayor for town programmes).
  - **Organising:** canvassing reach and the turnout boost (mobilisation), for the person and for campaigns they run.
  - **Intrigue:** better odds at leaks, digging up rivals' hidden facts and back-room deals, and their own hidden facts are harder to discover (DEC-034).
  - **Expertise:** more persuasive in lobbying, debates and conversations on bills and topics in their field (a nurse on health, a builder on housing). It feeds DEC-093 replies ("talks like someone who knows").
  - **Loyalty:** votes with the party line more often, resists lobbying that cuts against the party, and is slower to defect or endorse across party lines (DEC-094).
  - **Boldness:** takes more risks: late-night meetings, attack ads, shady offers made and accepted, challenging incumbents. Cautious characters prefer safe actions.
  - **Warmth:** stays as is (opinion change, conversation discretion, DEC-093).
- **Lab gate, no decorative attributes:**
  - For every attribute in the registry of character fields, `mandate-cli` runs paired worlds where only that attribute is shifted for a set of NPCs.
  - It checks that their behaviour moves measurably in the expected direction (action mix, vote alignment, corruption rate, campaign reach, implementation time and so on).
  - It runs in CI with the bots. A new attribute cannot ship without a declared effect and a passing check.
- **Views:** the character sheet's "Why?" lines say what each notable attribute does ("Loyal: rarely breaks with the party").
- **Timing:** M6, beside K4 (effectiveness terms and bots). It is in `M6_ENGINE_CONTRACTS.md` §4 as task A1.
- **Dependencies:** DEC-030, DEC-031, DEC-032, DEC-034, DEC-073, DEC-078, DEC-093, DEC-094.

### DEC-096 — The power broker acts

- **Status:** Accepted (owner, 2026-11-19: "Yeah let's do it"). The owner asked what role the town's power broker plays.
- **Context:** each town's broker is recomputed monthly (DEC-077 §1). It is the organisation leader with the most influence and a stake in the defining industry or the live crisis; failing that, the most senior officeholder (the mayor counts double). Today the broker is cast in scenes and dilemmas, offers the player bribes, is recorded as the briber in NPC corruption, is named in overheard chatter and conversations, and is a knowledge subject. The broker has no direct power over votes, money or careers.
- **Decision:** the broker becomes an actor with real weight, under the same rules as everyone else (DEC-030):
  1. **Lobbies the council** for their stake, with the standard lobbying formula. They push bills that help their industry or crisis interests and fight bills that hurt them, and the effect shows in whip counts.
  2. **Backs people:**
     - endorsements with a large follower weight among the cohorts they speak for (DEC-094 V2);
     - campaign money for candidates who line up with their interests, through the existing funds path.
  3. **Helps or hinders the player, by relationship and alignment:**
     - **An ally** lends lobbying help, endorsements and donations. The strings come through existing systems: favours called in, sponsor deals (DEC-092), and scandal risk if the help is shady (DEC-034).
     - **An opponent** feeds rumours through media contacts (as stories with their usual discovery rules, DEC-034), funds the player's rival, and turns council votes against the player's bills.
  4. **The role changes hands** as crises and industries shift. A falling broker loses influence, and a later-career player can become a broker.
  5. **The world does not wait (DEC-091 note):** the broker acts whether or not the player knows who they are. Learning who they are and what they want is how the player stops being surprised.
- **Balance:** magnitudes in content parameter blocks (DEC-078). The lab gates require that broker alignment matters (measurable vote and money effects) but never dominates, and that the variety check still passes.
- **Timing:** M6, with endorsements (V2) and attributes (A1). It is in `M6_ENGINE_CONTRACTS.md` §4 as task P1.
- **Dependencies:** DEC-030, DEC-031, DEC-032, DEC-034, DEC-077, DEC-078, DEC-091, DEC-092, DEC-094, DEC-095.

### DEC-097 — Engagement and variety

- **Status:** Accepted (owner, 2026-11-19: "Yes let's do it"). The owner worried that day-to-day play would be repetitive ("canvas and pass an unemployment bill every day") and asked whether the storyteller's output had been audited. They also asked for "hidden Easter eggs or secrets in the actual town, so people who are walking are rewarded with something".
- **Evidence (lab audit, 2026-11-19):**
  - Two 4-year careers (`mandate-cli story`, seeds 777 and 2026, mixed bot) had 27 and 29 decisions from only 10 and 12 templates.
  - The same few repeated: amendment-for-a-vote ×5, jobs plea ×5, pollution ×3–4, traders' meeting ×3–4.
  - Each career had one storyline (the rival), no press stories about the player, no scandals, and quiet stretches of up to 168 days.
  - The bot tournament (K4): canvass-only wins 84–87% of careers, the most of any strategy.
- **Decision:**
  1. **Ground game rebalance.** Canvassing mainly raises awareness, with fast diminishing returns per neighbourhood and per turn. Approval comes mostly from delivery: credit and blame for laws (DEC-094 V1), kept promises, and help to individuals. Canvassing stays strongest in campaign season.
  2. **Asks and promises (a goals layer).** People the player meets, the party, the broker, organisations and the press make concrete asks with deadlines ("fix the drain on Mill Lane", "vote with us on the budget", "say something about the closure"). Rewards are relationships, record, endorsements and knowledge; a broken promise costs. Campaign promises become tracked commitments. They are generated from live state and characters' goals (DEC-032) as data-driven ask templates, and shown in the pocket's Today tab.
  3. **Storyteller variety:**
     - a per-template cooldown, and no repeat within a chapter;
     - a larger pool, including consequences of the player's own past choices;
     - 2–4 storylines per career, with NPC schemes (rival, broker) running in parallel;
     - the press covers the player's work (bills passed, promises kept or broken, appearances), not only scandals.
  4. **Secrets (information).** Town secrets are uncovered by asking around, investigating, or tipping off the press, building on knowledge (DEC-091) and conversations (DEC-093): broker deals, buried reports, who backs whom, a councillor's debts. They are seeded from live state, never decisive alone, and follow "the world does not wait" (secrets exist and matter whether found or not).
  5. **Walking rewards (exploration).** Things found only by walking the town (owner request):
     - hidden places (DEC-090 D7);
     - curiosities and Easter eggs: a plaque, a mural, an old campaign poster, a lost cat, a bench with a story, a view from the hill;
     - rare ambient encounters: a resident with a tip, a journalist off duty, the broker taking a walk;
     - small collectible town lore for the notebook.

     Rewards are small: knowledge, a lead, goodwill, a story beat, an occasional modest boost. They are never required, and their kinds are data-driven so they vary by town. Quick mode and the Places tab reach anything that affects play. Pure flavour may stay walk-only, but nothing that affects play does (DEC-051).
  6. **An engagement gate in the lab.** Per simulated career:
     - distinct decision templates and repeats per template;
     - the longest quiet stretch;
     - storylines;
     - press stories about the player;
     - asks offered, kept and broken;
     - the action mix of winning bots: no single-verb strategy may be the best overall, which replaces today's canvass dominance.

     Thresholds are set from the design pass, and it runs in CI beside the bot gates.
- **Order:** the design pass first (`docs/design/research/ENGAGEMENT.md`); then 1 and 3 with the talk work; then 2 (before policy families); 4 and 5 with hidden places (G2); 6 alongside L1.
- **Dependencies:** DEC-030, DEC-031, DEC-032, DEC-033, DEC-034, DEC-051, DEC-072, DEC-076, DEC-077, DEC-090, DEC-091, DEC-093, DEC-094, DEC-096.
