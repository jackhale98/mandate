# The causal spine (S1)

Every event and every effect request names what led to it, so "why did this happen?" is answered by walking a graph, never by guessing. Built in W1 (IMPLEMENTATION_PLAN §2, row S1). Code: `crates/mandate-sim/src/spine/` (`mod.rs` the types, `graph.rs` the queries, `prune.rs` the bound), `events.rs` (the log), `scheduler.rs` (effect requests).

## 1. What it is, and what it is not

- **Bookkeeping, never inference.** A cause is written down where the engine already knows it: the event a system was reacting to, the command the player gave, the scheduled process whose rule fired, the factor of a formula that crossed a line, or an explicit mark that something is exogenous by design.
- **Continuous effects are not attributed at run time.** A 0.3-point change in unemployment, or which worry moved a vote, is the lab's business: counterfactual twins on the experiment runner (S2). The spine records discrete events and discrete requests only.
- **No planner.** Nothing plots ahead through causal links. The graph is read after the fact: by "Why?" panels, credit and blame, the press, skip and homecoming summaries, the Legacy record, conversation callbacks and story sifting.
- **It changes no outcome.** Nothing in the simulation reads a cause. The proof is in §7.

## 2. The cause type

`spine::Cause`, one small `Copy` enum. An event carries one or more (`DomainEvent::causes`, never empty); an effect request carries exactly one (`scheduler::Request { effect, cause }`).

| Cause | Meaning | Examples |
|---|---|---|
| `Event(id, Link)` | An earlier event, and how it led here | a closure in a recession; an enactment after an introduction |
| `Player(PlayerAct)` | The player's command, by kind (`vote`, `introduce`, `answer`, `let_pass`, `take_bribe`, …) | a bill tabled; an ask accepted; a decision answered or left to its default |
| `Actor(CharacterId)` | Someone else chose to | an NPC's bill; a resident's ask |
| `Process(Process)` | A scheduled process applying its standing rule (`lawmaking`, `elections`, `parties`, `press`, `justice`, `asks`, `agenda`, `director`, `budget`, `central_bank`, `calendar`, `markets`, `society`, `knowledge`, `update`) | a campaign opening by the calendar; a vote by the constitution's procedure |
| `Pressure(Pressure)` | A standing condition the engine's own formula names | the weakest factor of a sector's demand when a plant closes; a track in danger |
| `History` | The world as it was made | worldgen's first event |
| `Exogenous(Shock)` | Uncaused inside the simulation by design, and saying so (`hazard`, `world_economy`, `commodities`, `race`) | disasters, recessions, commodity shocks, the race shock |
| `Lab` | A lab intervention | the audit's planted closure |
| `Forgotten` | An earlier event the log no longer keeps (the hard cap only) | |
| `Unknown` | Recorded before the spine | events in a save older than v26 |

Notes:

- **A skip's acts are the player's.** The proxy plays the player's turns under their directives (DEC-012), and a skip must leave exactly the state its turns played one by one would (`act_flow` tests), so there is no separate "proxy" cause. A consumer that needs to tell them apart reads the skip's summary.
- **Exogenous stays exogenous.** A disaster the director held and released late is still `Exogenous(Hazard)`; the lever is linked, not blamed.
- **A closure explains itself.** A plant closes when its sector's demand driver is below 0.85. The driver is a product of named factors (world demand, commodity prices, the resource left, national competitiveness, local labour costs); the weakest at that moment is recorded as the `Pressure`. A running recession is linked as well. This is the engine reading its own formula, not a model of why.
- **Tier-generic (DEC-098).** No cause names an office or a body. A post appears only inside an event that already carries it.

## 3. Link types

`spine::Link`: how an earlier event led to a later one.

| Link | Meaning |
|---|---|
| `Caused` | It brought this about directly (a recession closed a plant; a conviction ended a career) |
| `Continues` | The same matter moving on (introduced, then enacted; opened, then charged) |
| `Prompted` | Someone acted because of it (a case after a scandal; a decision about a story; a bill for a proposal on the table) |
| `Revealed` | This made the earlier event known (a scandal breaking; a development the director spotlit) |
| `Ended` | This closes what the earlier event opened (a law expires; an ask is settled; a recovery) |

## 4. Threads

Most links are between events about the same matter. `spine::Thread` names the matter: `Bill`, `Proposal`, `Ask`, `Decision`, `Hidden`, `Scandal`, `Case`, `Rumour`, `Storyline`, `Recession`, `Election` (by its day), `Sentence`. `spine::thread(kind, date)` gives an event's thread and its part in it (opens, advances, notes, closes).

The log keeps each thread's latest kept event (`EventLog::heads`) and, as it records an event, links it to that one: `Continues`, or `Ended` for a closing event. So a site names only what the thread cannot know: the root, and links across threads (a case to its scandal, a bill to its proposal). No domain state had to grow an event id for this.

- A closed thread keeps its last event as its head: a second correction of a rumour, or a story on a verdict, links to how the matter ended. An event that would move a closed thread on starts it afresh (the next recession is not the last one's sequel). A link that means "this is still going on" asks for the open head (`EventLog::open_head`): a bill is prompted by a proposal on the table, never by one that lapsed.
- A matter that can have several instances under one key is not a thread. A town can have two hazards held at once, so each hold keeps its own event (`DeferredHazard::hold_event`) and its late strike, or its averting, links to that one.
- The player's vote on a bill is a step in the bill's thread: what follows on the bill links to it. Credit and blame read which way the vote went.

## 5. The bound

The log is pruned about once a month, as a turn ends (`spine::prune`). Events have a salience by kind (`spine::salience`).

1. **Age by salience.** `Trace` (turns, years, polls, the deed log, the director's levers) one year; `Routine` (bills tabled and expired, campaigns, asks offered, cases moving) four years; `Notable` (closures, disasters, recessions, laws enacted or failed, scandals, decisions resolved, asks settled) twelve years; `Landmark` (the player's own races, deselection, prison, the career's end, the game's start, a rules update, anything pinned) the whole run.
2. **Live state holds.** The latest event of every live thread (a bill or a law not ended, an open proposal, ask, decision or case, a scandal not over, a running storyline, recession, campaign or prison term), every event live state names (a law's, a fact's, a story's, a Legacy entry's), and a town's closures and disasters while it is in crisis (from two years before the crisis began).
3. **Nothing unread goes by age.** Five systems read the log forward from a cursor and act on what they find: the asks (the deed log; daily), the press (town news; weekly), the director (what stirs it, and the news of the town it may spotlight; weekly), the Legacy record (each turn) and a skip's summary (each skipped turn). Nothing from the oldest live cursor on is pruned by age (`prune::unread_from`). It matters when a reader waits: the director reads nothing while the player is in prison and the whole term on release. A reader counts while it can read again: the director until the career ends, the Legacy record while the act's is open, the press once outlets exist, none without a player.
4. **Chains stay whole.** Whatever is kept keeps everything it descends from. A link in the log always points to an event in the log.
5. **A hard cap** of 6,000 events as a backstop. Past it, events kept only by their age go first, least salient and oldest first. If that is not enough, unread events go too, oldest first whatever their kind: a reader more than a cap behind loses the oldest of what it had not read, as it did under the old log's "last 2,000", at three times the room. Links to what went become `Forgotten`. Live chains are never cut, so only live state alone can take the log past the cap.

What the rule replaced: the 2,000 most recent events plus every pinned one forever (every law enacted or failed was pinned, so the old log grew without bound, about 58 events a year in a medium world).

The log is pruned as a turn ends (and after a lab advance of days), so a skip prunes exactly as its turns played one by one would. Nothing in the simulation reads the log further back than the rule keeps: systems read it forward by cursor, which clause 3 covers, and the only dated look back is the `plant_closure` condition (`within_months`, a year in the base pack; closures are kept twelve).

Measured on a 30-year medium world (seed 2026, 820 idle turns): the log holds 1,722 events (4,958 before), 335 KB of the save's JSON (740 KB before), 33 KB compressed (67 KB before). The whole save is 2,115 KB (2,126 KB before): causes cost less than the unbounded pins did. The `heads` table held 623 threads.

## 6. The queries

`spine::graph`, all deterministic and depth-limited; a cause always has a smaller id than what it led to, so a walk back is a descending pass and a walk forward an ascending one. No index is stored.

| Query | Answers |
|---|---|
| `chain(log, event, depth)` | The chain from an event back to its roots: events nearest first, each with the link it was reached by, and every root named on the way |
| `consequences(log, event, links)` | What an event (a player decision) led to within N links |
| `nearest_player_act(log, event, depth)` | The player's act nearest behind an event, if any: credit and blame |
| `effects_of`, `causes_of` | One step either way |
| `matches(log, pattern, limit)` | Events matching a pattern of kinds along a chain: the base for sifting (§8) |

Protocol: `QueryDto::Why { event, depth }` returns `WhyView`: the event and what led to it, nearest first; each step with its kind, date, the engine's existing event text key and args, the typed link it led on by, and the causes it names; plus the player's act nearest behind it. Facts and keys only. The director's levers and the days hidden facts become discoverable are never shown, at any depth and however many lie in a row: the chain passes through them, the director's hand is dropped, and a run longer than the walk's limit ends the walk there (`truncated`). People are named only as the player knows them (DEC-091): someone's own choice as a root, and any name in a step's text arguments is left empty until the player has heard of them, because the query takes any event id and must not be a way to read every name in the log. Every other kind of event is public record (laws, races, closures, disasters, scandals once they break, cases) or the player's own (deeds, asks, decisions, warnings); the overview's event list is unchanged.

## 7. Proof that nothing changed, and coverage

- **Outcome unchanged.** `crates/mandate-cli/src/lab/proof.rs` pins, from the build without the spine, two digests of eight games: every event's id, date and kind in order (no causes), and the whole state less the event log and the cause fields. Six games are three seeds, each idle and with one active bot, eight years. Two put the player in prison by a lab edit, for four years (the longest sentence a career survives) and for six: the director waits out the term and then reads all of it. Under the engine's own rules a four-year term loses nothing even without clause 3 (the director reads in the turn of the release, just inside the routine events' four years); the six-year game is the one whose state changes without it. The spine leaves all sixteen digests unchanged. Each game is played with pruning on (the state) and with the lab switch that keeps every event (the full stream).
- **Nothing significant is uncaused.** `lab/spine_games.rs` plays three seeds, idle and with one bot each, six years: every event names a known cause; by kind, the right one (an exogenous mark, a pressure for a closure, a link for whatever happens to a bill, an ask or a case); every effect request applied had a named cause; the log keeps its invariants and its cap.
- **The metric.** `audit readability` reports `causal_spine`: for every event kind, the share with a cause and the share by the best class of cause (`linked`, `player`, `actor`, `pressure`, `process`, `exogenous`, `unknown`).
- **Old saves.** v25 → v26: old causes become typed links, events with none get `Unknown`, thread heads are replayed. Nothing is invented, not even "exogenous" for an old disaster.

## 8. How sifting will sit on it

A sifting pattern is a list of steps along a chain: "an event like this, then within N links (of this type) an event like that". `graph::matches` is that walk today, with a Rust predicate per step and two worked patterns as tests (`spine/tests.rs`): *a favour repaid* (a closure, an ask it prompted, the player's promise, the ask kept) and *the bribe that ended a career* (the player's own act, the story, the case, the conviction, the end).

What story sifting (W3) adds, and no more:

1. **Steps as data.** A step's test becomes a `mandate_content::Condition` evaluated with the event as its focus (its kind, its town, its actors bound to the pattern's roles), plus `within` and `via`. Patterns live in content, validated like every other condition.
2. **Bindings.** A pattern binds roles (the asker, the town, the bill) as it matches, so later steps can require "the same person". The walk already returns one event per step; binding is a filter on it.
3. **Conditions beside the chain.** Some stories are not causal chains in the engine: "the closure that ended a career" joins a closure to a lost race through approval, which is continuous. Such a pattern uses a chain step for what is linked and a condition (same town, within a term) for what is not. The spine does not pretend the link exists.
4. **Consumers.** The director, the press and conversation callbacks ask for matches since a cursor; a match is facts and keys.

## 9. What the first consumers should build next

- **"Why?" surfaces: built** (below).
- **Credit and blame (V1):** `nearest_player_act` plus the lab's twins for size. A vote behind a law is one of many: weigh it by the margin.
- **Sifting:** §8.

### 9.1 The "Why?" panel (built)

One panel and one control (`web/src/components/why/`): `WhyButton` on an outcome asks `QueryDto::Why` for the event the view names and `WhyPanel` lays the answer out. The layout is a pure model (`whyModel.ts`); the panel computes nothing about the world.

- **The panel** is a sheet like the others (a bottom sheet on phones, the left panel on wide screens; Escape, the close button, a swipe down or a tap outside close it; the keyboard goes to it when it opens and back to the control when it closes; nothing holds it there). It opens over whatever showed the outcome, election night included.
- **The reading order** is newest at the top: the event asked about, then each earlier step as one sentence (the event's own line) with its date. Between two steps a phrase from the link type leads from the later event down to the earlier one: `caused` "because of", `continues` "after", `prompted` "prompted by", `revealed` "which brought to light", `ended` "which ended". The engine gives the steps level by level; the panel follows one line of causes back before it starts the next, and a step that joins a row further up says which ("And “…” which ended").
- **Roots** are named under the step that names them, one line each: the player's own act, someone's own choice (by name as the player knows them, else "Someone's own choice"), a process, a pressure, an outside shock, history, forgotten, unknown.
- **The player's act**: when `player` is set, a line at the top says something the player did is behind it, with the act and the day of the step that names it.
- **Further back**: when `truncated`, the panel says more lies further back and offers one "Go further back", which asks again at `max_depth` (the view now carries `depth` and `max_depth`, so the client holds no constant).
- **An election**: the result's own factor decomposition (`ResultView.decomposition`) is shown under the chain, as the result card shows it. The chain says what happened, the decomposition what moved the votes.

**Text** (`content/base/locale/en/why.ftl`): `why-link-{caused,continues,prompted,revealed,ended}`; `why-player-{act}` for each `PlayerAct`; `why-process-{process}`; `why-pressure-{pressure}`; `why-exogenous-{shock}`; `why-history`, `why-forgotten`, `why-unknown`, `why-actor`, `why-actor-unnamed`; the panel's own strings are `ui-why-*`. `mandate-wasm/tests/why.rs` lists every variant of `Link`, `PlayerAct`, `Process`, `Pressure` and `Shock` in an exhaustive match and fails when one has no line (or a line has no variant), so a new cause cannot ship without text. No line names an office, a body, a market or an industry: a name comes from the step's own sentence.

**A public result is public** (DEC-108 §8). §6's rule (a name is left empty until the player has heard of the person) has one exception, by event kind, as a list in `why.rs` (`PUBLIC_RESULTS`) with a test a kind: `race_decided` (the winner), `policy_enacted` and `policy_failed` (a law's recorded vote), `scandal_broke` (the subject of a published story), `case_developed` (the subject of a court's ruling). A step of one of these kinds reads as the public line does, and a person it names is named as its actor too. Everything else (an investigation opening, a rumour, a bill's sponsor, a storyline's cast) stays under DEC-091. The query writes nothing: the knowledge ledger is as it was.

**Where the control is.** A view names the event to ask about (an optional id, absent when the log no longer keeps it; never a diagnostic):

| Surface | Field | The event |
|---|---|---|
| The overview's event list | `EventView.id` | the event itself |
| A race's result card (the party office, election night) | `ResultView.event` | the race decided, with the factors beside the chain |
| A bill's card | `BillView.outcome_event` | enacted, failed, expired or withdrawn (the card's control reads "What led to this": its "Why?" is the bill's support) |
| The newspaper's stories | `ArticleView.event` | the town event, the scandal breaking, the case's step, the rumour or its correction |
| The pocket's scandals and cases | `ScandalView.event`, `CaseView.event` | the story breaking; the case's latest step |
| An ask or promise, once it has ended (Today, the story sheet's promises) | `AskView.event` | kept, broken, declined or void |
| A decision's aftermath | `AftermathView.event` | the decision resolved |
| A Legacy entry | `ActLineView.event` | its `source_event` |
| A line of the skip summary | `ActLineView.event` | where a thread names it: a law, a decision, an ask, a story, a case |

Not yet: the story sheet's "Recent" lines and storyline stages (their ids are fact and run ids, not events); the skip summary's elections, closures and disasters (the summary keeps no event for them); a list of decided laws for the bill card to appear in (the app lists drafts and bills before the body only, so today a law's "Why?" is met in the overview, the newspaper, the summary and the Legacy record).
