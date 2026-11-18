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
| DEC-005 | Nested time structure & target lengths | Proposed |
| DEC-006 | Endings & victory structure | Proposed |
| DEC-007 | Constitution variability per run | Proposed (awaiting owner) |

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

- **Status:** Proposed
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

### DEC-006 — Endings & victory structure

- **Status:** Proposed. The owner is open to alternatives to "win the midterms".
- **Options:**
  - A. Single win: hold the legislature at the midterms.
  - B. Legacy score only.
  - C. Agenda/pledge completion.
  - D. Layered: every run ends in a Legacy epilogue, and the summit victory is the **Mandate Test**, the first national verdict on your presidency as scheduled by the constitution.
- **Proposed decision:** D. The Mandate Test is whichever national electoral event the constitution places first in the presidency: midterms in a staggered presidential system, the next general election or a confidence vote in a parliamentary one. "Mandate renewed" is the top ending.
  - The Legacy epilogue scores offices reached, pledges kept, measured outcomes of enacted policies, relationships, and reputation, and grades the career. No run ends in a bare "game over".
  - Pledges, meaning campaign promises tracked as measurable objectives, are a candidate mechanic (D2/D3). They tie Legacy to the game's name.
- **Rationale:** Keeps a dramatic summit. Gives every failed career a meaningful conclusion (needed given DEC-005 run lengths). Separates the victory condition from any single constitutional system (resolves DISCOVERY C3).
- **Dependencies:** DEC-007, D2, D4, D8.
- **Revisit criteria:** Playtests show Legacy endings feel like consolation prizes, or the Mandate Test is too swingy.

### DEC-007 — Constitution variability per run

- **Status:** Proposed. Awaiting owner.
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
