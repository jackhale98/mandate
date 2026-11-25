# Implementation plan: M6 to the tester checkpoint

Written 2026-11-23, before work resumes. It turns the owner's notes and DEC-101 to DEC-107 into an order of work, names the shared mechanisms to build once, and sets the rules for using agents cheaply. `M6_CHECKLIST.md` stays the status list; this file is the order and the method.

**Status:** accepted (owner, 2026-11-23: "These suggestions sound good"), including the three points in §7.

## 1. Principles

1. **Simulate causes, template the changes.** The simulation models why things happen (needs, stress, relationships, money, memory). Writing is short templates over state changes, plus authored *structure* for emotion: crisis arcs, key moments, the player's personal stake. No plotted storylines.
2. **A new system must pass four tests:**
   - it creates real decisions for a politician;
   - its consequences can be seen (street, people, "Why?");
   - it interacts with at least two existing systems;
   - its stories come from templates, not one-off scripts.
3. **Depth and legibility before breadth.** M6 makes the existing systems responsive and visible. New systems (crime, transport, faith, immigration, living parties) come one at a time in M7.
4. **The standing rules hold:** determinism (DEC-041), every tier by data (DEC-098), no decorative attributes (DEC-095), one condition language (U1), systems write only their own domain, the engine emits facts and keys.

## 2. Shared mechanisms: build once, use many times

Most of the planned work is the same seven mechanisms wearing different clothes. Each is built once, by the first task that needs it, as a registry or a small API. Later tasks add data.

| # | Mechanism | Built by | Reused by |
|---|---|---|---|
| S1 | **The causal spine.** Every event and effect request carries its cause; a bounded causal event graph. Half exists: events have `causes`, but only 4 of about 18 effect kinds carry one | W1 | "Why?", vote on impact, V1 credit and blame, the press, skip and homecoming summaries, Legacy, conversation callbacks (E13), story sifting, the audit |
| S2 | **The experiment runner** (lab). Clone a game, run N months under a named policy, collect metrics; twin runs for counterfactuals | W2 (the audit) | policy responsiveness gate (F7), the audition (DEC-101), the engagement gate (E12), difficulty gates (DEC-102), quality-diversity search (§3) |
| S3 | **Staged arcs.** One structure for anything with stages, a clock, escalation and branching ends | W3 (crisis arcs) | crises, storylines (E4), multi-step asks, secrets (E9) |
| S4 | **Talk hooks.** Systems register dialogue choices with their own lines | W1 (the asks fix, note 12) | asks, crisis levers, casework, secrets, families' `talk` hooks |
| S5 | **The impact estimate.** One API that says what a policy or bill will do, to whom and where, gated by knowledge | W4 (F3) | vote on impact (note 10), the families' preview, "Why?" (with S1), bots |
| S6 | **Town map layers.** A registry of overlays on the town map, like the country map's layers (DEC-089) | W4 | people on the map (note 11), worries, a bill's impact by neighbourhood, asks, works, upkeep, later crime |
| S7 | **State-driven street dressing.** A registry mapping a state level to how often a visual cue appears | W5 | works (DEC-099), upkeep (potholes, lights), later crime cues |

## 3. Three techniques the owner raised (2026-11-23)

1. **A causal trace in the lab, and a causal graph for narrative: yes, as the spine (S1).**
   - **In the game:** cheap provenance. Each effect request and event names what caused it. This is bookkeeping, not inference.
   - **In the lab:** counterfactual twins. Run the same seed with and without a decision and difference the outcomes. A deterministic engine makes this exact and cheap. Continuous effects (a 0.3-point change in unemployment) are attributed this way, offline, never at run time.
   - **For narrative:** *story sifting* over the causal graph. Patterns written as data (in U1's language) find stories that actually happened ("a favour repaid", "a rival's revenge", "the closure that ended a career") and hand them to the director, the press and conversations. This is the research line of Ryan's *Curating Simulated Storyworlds* and Kreminski's Felt and Winnow. It replaces authored storylines with sifting patterns, which fits principle 1.
   - **Not this:** a planner that plots stories ahead through causal links. It is heavy, brittle, and fights the simulation.
   - **Bound it:** prune the graph by age and salience so saves stay small.
2. **Quality-diversity search (MAP-Elites): yes for strategies and openings, no for raw tuning.**
   - **Strategy space (the best use).** Replace the 13 hand-written bots as the *search* with one parameterised policy (weights over action utilities). MAP-Elites fills a grid of play styles (share of canvassing, asks, legislation, exploring, shady acts) with the best-scoring policy in each cell. The map shows dominant strategies, dead styles and exploits directly. The hand-written bots stay as named probes for the gates.
   - **Openings and worlds.** Descriptors: industry mix, crisis kind, party fragmentation. Quality: the audition score. Result: a spread of good openings instead of the same best one.
   - **Presets.** A diverse set of good parameter sets for difficulty presets and storyteller profiles.
   - **Not for single-target tuning.** Hitting one target (a gate, a band) is ordinary optimisation; a grid sweep or CMA-ES is simpler.
   - **Cost:** compute, not tokens. Twenty years take half a second, so tens of thousands of evaluations fit in a nightly job. No surrogate model is needed.
   - It is research-stage in the literature (card and board games), so it starts as a lab report, and becomes a gate only if it proves useful.

## 4. Order of work

Each wave builds on the one before. Within a wave, tasks marked ∥ have disjoint files and may run together (two or three at most).

**W0 — Clear the ground** (nothing else running; serial)
1. **Structure-only refactor** (large tier). Split `types.rs`, `validate.rs`, `actions.rs`, `bots.rs`, `doctor.rs`, `director.rs`. Proof: identical determinism and parity hashes, bot output and corpus.
2. **Process fixes** (small tier), in the same window:
   - PROGRESS and BALANCE_LOG as one file per entry;
   - the v18 golden compares everything except the content hash;
   - `scripts/check.sh fast|affected|full`, printing a ten-line summary;
   - DECISIONS split into one file per decision plus an index (it is about 66,000 tokens today; an agent that opens it spends that before starting);
   - `AGENT_CONTEXT.md`, one page: the rules, the map of the code, the commands.

**W1 — Fix what the owner found, and lay the spine**
- ∥ **Play notes, web** (mid): notes 2, 3, 4, 6, 7 and the web side of 1.
- ∥ **Play notes, engine** (mid): notes 1 (every label path, with a test), 8 (party line from the platform; bots re-run), 9, and 12 (asks: talk hooks S4, instant steps, the broken-on-a-later-day bug, the need-path audit).
- ∥ **The style bible** (large for the guide and samples, small for the lint): note 13. The owner approves the samples before any content wave.
- Then **the causal spine S1** (large): causes on every effect and event, the bounded graph, a test that nothing significant is uncaused.
- **Security pass** (small) and **legal groundwork** (small): notes 15 and 14. Any time; no conflicts.

**W2 — Know the simulation**
- **The simulation audit** (large to design, mid to run): builds the experiment runner S2 and the counterfactual twins; the report; fixes logged in BALANCE_LOG; the harness becomes a nightly gate. Checks every system against its claims.
- ∥ **Quality-diversity strategy map** (mid), once S2 exists: a lab report first.
- ∥ **DEC-102 gates** (small), once the owner confirms the targets.

**W3 — The hook (DEC-101)**
1. **Staged arcs S3** and the crisis arc template (large).
2. ∥ **Crises rewritten and a dozen added** (mid, to the style bible; the doctor lints; a separate reviewer reads samples).
   ∥ **The audition** (mid), on S2.
   ∥ **Specific industries and named plants** (large to design the registry and demand, mid for worldgen and content; owner, 2026-11-24; M6_CHECKLIST note 17): industries as data under the broad sectors, demand per industry, plants named by what they make. Before the crisis rewrite, so crises name a cannery or a steelworks.
3. **Opening frames, the draft, the first-beat guarantee** (mid), then the first chapter's pacing.
4. **Story sifting, first patterns** (mid), on S1: feeds the director, the press and callbacks.

**W4 — Policy you can read**
1. **F3 with the impact estimate S5** (large): family views, and vote on impact for ordinary bills (note 10).
2. ∥ **F4 and F5** (mid): the first pack and the coverage gate.
   ∥ **Town map layers S6** (mid): people on the map (note 11), then worries, impact, asks.
3. **F6** (mid): the proposal sheet. **V1** (large): credit and blame, reading S1.

**W5 — A town that shows its state**
- **Street dressing S7** (mid) with **works you can see** (DEC-099).
- **Asset condition and upkeep** (mid), then **the first batch of worries** (mid), each checked by the audit harness.
- ∥ **Canvassing at doors** (note 5; mid) and **people walk off** (note 7's engine side, if any).

**W6 — Content and the checkpoint**
- E3 to E5, E9 to E13 and text pass 2, through the content pipeline (§5.6).
- Difficulty modes; settings; accessibility with a screen-reader playtest; PWA and performance; docs; the old-saves corpus.
- **The landing page rewritten** (owner, 2026-11-24): the new pitch drafted at the end of W3 for the owner to read (the cold open is the hook), screenshots taken after W5 (named plants, works and upkeep change how towns look), shipped just before the checkpoint. Settle first: the name and any subtitle, a contact address, and a custom domain if there is one.
- The tester checkpoint, with the engagement questions ("did you care about anyone?").

**Every new system ships with a story (owner, 2026-11-25).** A system added from M7 on brings at least one staged arc and the simulated cause that raises it, built on the existing arc building blocks (a new kind of block only when none fits), and graded like any other arc. Otherwise the systems grow and the stories do not.

**M7, one system at a time (each passing §1's tests):** faith communities; crime with justice and prisons; commuting, then transport; energy; immigration with DEC-087; living parties (DEC-107); NPC stress and the player's wellbeing with daily life; return to a world (DEC-103); the Act 2 design (DEC-105). Account sync when the domain exists (DEC-104).

## 5. Working with agents: spend tokens where they buy something

What the research says, and what today's work showed:
- Multi-agent work costs several times the tokens of single-agent work, and pays only for parallel, separable tasks or to keep a context clean. Two or three agents with disjoint files is the ceiling here.
- Token use tracks context size. The main costs in this project were not the agents' thinking but large files read whole, long logs, long reports, and the lead waking with a very large context.

Rules:

1. **Tiers by difficulty.**
   - **Lead** (the top tier): plans, writes briefs, decides, reviews reports. It does not read code or logs.
   - **Large:** cross-cutting engine design (the spine, arcs, the impact estimate, the audit's design, V1).
   - **Mid:** well-specified engine, web and content tasks. Most tasks.
   - **Small:** mechanical work: sweeps, renames, CSS, doc rows, re-recording goldens, lookups, and watching CI.
2. **Briefs are contracts.** Goal, exact files and line ranges to read, OWNS, the acceptance commands, the report budget. "Read only what is listed." Briefs cite one decision file, never DECISIONS as a whole.
3. **Quiet tools.** Agents run `scripts/check.sh`, which prints a summary; full logs go to a file and are read only on failure, by grep.
4. **Local checks are scoped; CI runs the whole suite once.** An agent runs the tests of the crates it touched, plus the specific gates its task names. The full Playwright suite runs in CI, not in every agent.
5. **Short reports.** At most 40 lines plus tables: what changed, interfaces, results, risks, what is not done. The report lands in the lead's context, the most expensive place in the system.
6. **The content pipeline.** Draft (mid, to the style bible, in batches) → the doctor's lint (free) → a reviewer that did not write it reads samples (mid) → the owner blind-reviews a sample → merge. Never the lead writing lines.
7. **CI watching is a small-tier job.** One small agent waits for the branch run, fast-forwards `main`, runs the live check and reports one line. The lead is not woken for it.
8. **Merges.** Agents merge the branch into their own work before handing back, so the lead fast-forwards. One save bump per merge. Hot files are touched by one agent at a time.
9. **Verification over trust.** Refactors prove identical hashes. Balance changes show before and after. A reviewer is given the code, not the author's conclusion.
10. **Fresh sessions.** Start each wave in a new session that reads this plan, the checklist and `AGENT_CONTEXT.md`. A long session makes every turn pay for its whole history.

## 6. Architecture rules for what is added

- **Registries first.** Anything with kinds (crises, frames, interrupts, endings, cues, layers, worries, faiths) is a content registry with a schema, validated, with an EXTENDING section.
- **One file per area, under about 800 lines** of non-test code; tests beside the code or in the crate's `tests/`.
- **State is keyed by `Jurisdiction`** or by a physical place; nothing new reads the player's town directly.
- **Derived over stored.** Compute from state on demand where it is cheap (as N2 did); store only what must persist. Every stored field has a migration and a bound on its growth.
- **Every gate has a harness.** A claim about balance, pacing or responsiveness is a lab command with a number, then a CI or nightly gate.
- **Player and NPCs share rules** (DEC-030): new verbs are available to both unless a decision says otherwise.

## 7. Decided by the owner (2026-11-23)

1. **The refactor goes first** (W0), ahead of the play notes.
2. **DEC-102's targets are accepted:**
   - a run is a career;
   - an engaged newcomer is re-elected about 60 to 70% of the time, a skilled player about 85%;
   - top office is reached 5 to 15% of the time.

   Playtests may move the numbers.
3. **The style bible's reference voices:** dry civic realism (*Yes Minister*), terse event text (Crusader Kings), character-driven political choice (*Suzerain*), spare and humane (*Citizen Sleeper*). Tone and craft only. The owner still approves the sample passages before any content wave.

## Sources
- Anthropic, "Building multi-agent systems: when and how to use them": https://claude.com/blog/building-multi-agent-systems-when-and-how-to-use-them
- Anthropic, "Subagents in Claude Code": https://claude.com/blog/subagents-in-claude-code
- Fontaine et al., "Mapping Hearthstone Deck Spaces through MAP-Elites with Sliding Boundaries": https://arxiv.org/abs/1904.10656
- "Rinascimento: searching the behaviour space of Splendor": https://ar5iv.labs.arxiv.org/html/2106.08371
- James Ryan, *Curating Simulated Storyworlds* (PhD dissertation, UC Santa Cruz, 2018)
- Kreminski, Dickinson, Mateas, "Winnow: A Domain-Specific Language for Incremental Story Sifting": https://doi.org/10.1609/aiide.v17i1.18903
