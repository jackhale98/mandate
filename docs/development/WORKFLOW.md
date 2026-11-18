# Mandate — Agentic Development Workflow

**Status:** Proposed final baseline. Implements brief §23–25 and §29.

## 1. Roles
The **lead agent** owns architecture consistency, decision and requirement traceability, interface stability, integration, quality gates, and scope. Specialist roles (simulation, worldgen, politics, narrative, frontend, test, reviewer) are logical hats, not standing agents. Spin them up per bounded task.

## 2. Task brief template (required for every delegated task)

```text
OBJECTIVE:        one sentence
REQUIREMENTS:     REQ/SIM/TECH/UI IDs
DECISIONS:        DEC IDs that constrain the work
CONTRACT:         public types/functions in and out (paste signatures; interface-first)
OWNS:             files/modules the agent may modify
OUT OF SCOPE:     explicitly forbidden areas
ACCEPTANCE:       testable criteria
TESTS:            unit / property / integration / determinism required
DELIVERABLES:     code, tests, doc updates
INTEGRATION NOTES: known neighbours, follow-ups
```

## 3. Task report template (required on return)

```text
TASK:        Implemented:
FILES:       - ...
INTERFACES:  - added/changed public items
TESTS:       - what ran, results (paste command + summary)
DECISIONS:   - new DEC proposals (never silently decided)
ASSUMPTIONS: - ...
RISKS:       - ...
FOLLOW-UP:   - ...
```
Reports summarize outcomes and evidence. No narration.

## 4. Rules

**Parallelism:**
- Parallelize only tasks with disjoint `OWNS` sets and agreed contracts.
- Never have two agents edit core shared types at once.
- The flow is: contract → parallel implementation → independent review → integration → cross-system tests → docs update.

**Context loading (brief §24.3):**
- Always load: `AGENTS.md`, the task brief, and its acceptance criteria.
- Load as needed: the referenced DEC entries, the relevant spec section, public interfaces, related tests.
- Load only when necessary: full docs and research files.

**Review:**
- Important changes get an independent reviewer, who is given the diff and the task brief, not the implementer's conclusions.
- The review checklist covers requirement coverage, architecture boundaries, determinism, error handling, test strength, dependencies, and scope creep.

**The lead agent rejects changes that:**
- introduce unapproved architecture;
- duplicate functionality;
- add unjustified dependencies;
- touch unrelated files;
- bypass domain interfaces;
- weaken tests;
- hide failures;
- introduce nondeterminism.

**Dependencies:**
- New crates and npm packages need justification: maintenance, license, WASM compatibility, size, alternatives.
- Significant ones get a DEC entry.

**Research:**
- One agent researches a question and returns a sourced comparison.
- The result is recorded under `docs/design/research/` and reused, not repeated.

## 5. Definition of done (brief §29)

**Every task:**
- requirements addressed;
- interfaces respected;
- tests added and passing;
- fmt and lint clean;
- docs updated;
- no unexplained architectural change;
- no unrelated edits;
- acceptance criteria met;
- review done where appropriate.

**Simulation features also need:**
- determinism verified;
- invariants covered by property tests;
- at least one integration test;
- diagnostics sufficient to debug.

**UI features also need:**
- responsive behavior (phone and desktop);
- keyboard use;
- loading and error states;
- an accessibility check;
- a browser test where appropriate.

## 6. Change control
- Accepted decisions change only through a new or superseding DEC entry, with the owner's approval for gameplay or architecture tradeoffs.
- `PROGRESS.md` is updated at the end of every working session.
