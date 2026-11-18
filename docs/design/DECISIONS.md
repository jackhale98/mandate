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
| DEC-020 | Career detours: non-office power roles | Proposed (owner idea) |
| DEC-021 | World scale & spatial representation | Proposed (D6) |
| DEC-022 | World history via burn-in simulation | Proposed (D6) |
| DEC-023 | Environment dynamics over a career | Proposed (awaiting owner, D6) |

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

- **Status:** Proposed. Originated by the owner; design details are the agent's proposal. Finalize the role list in D10.
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

- **Status:** Proposed (D6).
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

- **Status:** Proposed (D6).
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

- **Status:** Proposed. Awaiting owner (D6).
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
