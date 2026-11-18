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
| DEC-009 | Presence model: embodied scenes + strategic layer | Accepted |
| DEC-010 | Player role: politician first, administration mediated | Accepted |
| DEC-011 | Player power & resource model | Accepted |
| DEC-012 | Time model: discrete turns, phase-based cadence, time skips | Accepted |
| DEC-013 | Action economy: representative day, slots, delegation, commitments | Accepted |
| DEC-014 | Government tiers & office ladder | Accepted |
| DEC-015 | Constitution parameter space & archetypes | Accepted |
| DEC-016 | Default / MVP constitutional archetype | Accepted |
| DEC-017 | Constitution primitives (engine model) | Proposed |
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

- **Status:** Accepted (owner, 2026-11-18): option B.
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

- **Status:** Proposed. Technical; to be confirmed in D11.
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

### DEC-033 — Narrative director: authority & pacing

- **Status:** Accepted (owner, 2026-11-18): option B.
- **Precedent** (agent's knowledge, not re-verified this session):
  - Classic roguelikes (Rogue, NetHack, Brogue) are effectively A, with pacing coming only from depth-scaled generation.
  - Modern roguelites pace mainly through *structure* (act maps, guaranteed rest and shop nodes, bosses) plus luck-smoothing. Slay the Spire raises rare-card and potion odds after misses; XCOM 2 adds hidden aim assistance on lower difficulties.
  - Reactive directors (Left 4 Dead's AI Director, RimWorld's storytellers) are B-like and belong to long-form emergent-story games, Mandate's closest cousins. Over a 6–10 h career, an unlucky pile-up is a ruined run rather than a funny story.
- **Addendum (proposed, post-MVP):** selectable **storyteller profiles**, following RimWorld:
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
