# Mandate — Agentic Development Workflow

**Status:** Accepted with the system plan (DEC-067). Implements brief §23–25 and §29. §7 describes current practice for parallel agents (2026-11-19).

## 1. Roles
The **lead agent** owns architecture consistency, decision and requirement traceability, interface stability, integration, quality gates, and scope. Specialist roles (simulation, worldgen, politics, narrative, frontend, test, reviewer) are logical hats, not standing agents. Spin them up per bounded task.

## 2. Task brief template (required for every delegated task)

```text
OBJECTIVE:        one sentence
REQUIREMENTS:     REQ/SIM/TECH/UI IDs
DECISIONS:        DEC IDs that constrain the work; always DEC-098 with the
                  TIER-GENERIC rule below, copied into the brief
CONTRACT:         public types/functions in and out (paste signatures; interface-first)
OWNS:             files/modules the agent may modify
OUT OF SCOPE:     explicitly forbidden areas
ACCEPTANCE:       testable criteria
TESTS:            unit / property / integration / determinism required
DELIVERABLES:     code, tests, doc updates
INTEGRATION NOTES: known neighbours, follow-ups
```

Every brief carries this rule under DECISIONS (DEC-098; `docs/design/research/EXTENSIBILITY_AUDIT.md` §5.1):

```text
TIER-GENERIC: State that belongs to "a town, a region or the nation" is keyed by
  `Jurisdiction`, never by `MunicipalityId` or a raw u32 (keep `MunicipalityId`
  for physical things in a town: places, neighbourhoods, curiosities).
  Offices, seats, powers, terms and calendars are read from the constitution;
  no Rust code or content names an office or body ("mayor", "council") except
  through constitution ids. New code does not read `player.municipality` or
  `player.ward`: it asks `player::home(state)`, `player::posts(state)` or
  `state.focus()`. A brief that cannot follow this says why in DECISIONS.
```

Until R1 merges, `player::home`, `player::posts`, `state.focus()` and `Jurisdiction` do not exist yet. A brief that runs before R1 uses `Jurisdiction::Municipality { id }` from R1's first commit, or a local stand-in with the same serde shape, and routes its reads of the player's town through one helper that R1 replaces.

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

## 7. Parallel agents in practice

- **One worktree per agent.** Each agent works in its own git worktree under `.claude/worktrees/` (ignored by git) on its own branch, and never edits another worktree or the main checkout. The lead merges branches into `main`. Files another agent owns at the time are listed in the brief's `OUT OF SCOPE`; changes they need go into the report as suggested edits.
- **Ports.** The lead gives each agent its own ports for dev and preview servers (`npx vite --port <n>`, `npx vite preview -c landing/vite.config.ts --port <n>`). The Playwright config serves on port 4173 and reuses a server already running there, so an agent running E2E tests in parallel uses a temporary config on its own port with `reuseExistingServer: false`, and deletes it before committing.
- **Commit early.** Commit at each working step on your own branch (WIP commits are fine). A container restart can interrupt any agent, and uncommitted work in a worktree is the work most at risk. Never use a bare `git stash`: the stash is shared by every worktree.
- **Shared machine.** Builds compete for a few cores. Limit Rust build parallelism with `CARGO_BUILD_JOBS` (the lead sets the number in the brief; for example 2 on a 4-core machine), build and test only the crates the task touches while iterating, and leave full-workspace and WASM builds to the end or to CI. Docs-only tasks run no heavy builds.
- **Generated files.** WASM bindings (`web/src/engine/pkg/`) and `web/node_modules` are not in a fresh worktree. Build them, or link or copy them from the main checkout for a check, and remove the link before committing.

