# Staged arcs (S3) and the crisis arc template

**Status:** accepted design (DEC-109, 2026-11-24) for DEC-101 item 4 and IMPLEMENTATION_PLAN W3.1. Tasks A1 (the arc core and the flood arc from *struck*; save v27) and A2 (the director on arcs, storylines folded, the budget round and the rival as standing arcs; save v28) are built; where a build showed this note wrong, the text is corrected in place and marked **(A1)** or **(A2)**, and §12 and §13 list every correction. It sits on the causal spine (`CAUSAL_SPINE.md`), the experiment runner (SIMULATION_AUDIT, "The experiment runner"), talk hooks (`talk/hooks.rs`) and the one condition language (`mandate_content::Condition`).

The owner's question: "Do you think these crises will actually be exciting? Or will they just come up as a quick decision vote?" Today a crisis is an indicator past a threshold (`crises.jsonc`), a flag on the town and one dilemma. This note replaces that with one structure, the **arc**: a matter that unfolds in stages the player can see coming, act inside and change. The rule throughout is the plan's first principle: simulate causes, template the changes.

## 1. The model

**An arc is one matter in one jurisdiction, with a cast, a current stage, a clock and a heat.** It is raised by simulated state, moved by simulated state and by what people do, and ended by simulated state. A template (`ArcDef`, content) says how; a run (`ArcRun`, saved) is one telling.

1. **Raised by the world.** `raise` is a list of conditions over simulated state at a jurisdiction: a plant under review, a flood forecast, a hidden fact that will surface. It never reads the ledger (the world does not wait). The event that made it true is the arc's root on the spine.
2. **Stages.** Three to seven, in a graph. The template fixes their shape: *signs* (often hidden) → *break* (a face and a clock) → *escalation* → *levers* → *turn* (the stage marked `climax`) → exits. A crisis arc must have signs, a break and a turn; other arcs need only a turn.
3. **Transitions read; beats write.** A stage lists `next` edges in priority order, checked weekly and on entry (so an arc raised mid-matter starts where the world is). An edge fires on conditions over simulated state, over the arc's own marks, heat and days, or on the clock. Nothing a player picks names the next stage: a choice posts effect requests and sets marks, and the edges read the result. So the same lever can fail in a town whose numbers do not support it, and "Why?" can always show the chain.
4. **Clocks.** A stage may set a deadline the player sees; later stages keep it until one sets another. When the simulation owns the date (a plant's review, a forecast, a vote, an election) the clock reads it (`"from": "subject"`). Otherwise it is a keyed draw in a content range. Every stage ends on time: its last edge is on time alone, or its edges cover every state of a subject whose owner guarantees an end (a review is always lifted, kept or closed).
5. **What a stage offers.** Seven beat kinds, each riding on a system that exists:

| Beat | What it is | Rides on |
|---|---|---|
| `decision` | A dilemma with the arc's cast; options may set marks | `dilemmas.jsonc`, scenes, the pocket |
| `ask` | A request from a cast person, kept or broken later | the asks system, Today |
| `talk` | The arc's own choice in a conversation with a cast person; each reply has conditions and may set a mark, teach, or post effects | talk hooks (owner `arc`) |
| `visit` | Something to do or see at a cast place | visit verbs, Places, "See it" (DEC-099) |
| `press` | A story the outlets may run | media coverage |
| `heard` | Overheard lines and small talk that hold while the stage does | `overheard.jsonc`, conversation topics |
| `move` | A cast person acts: a ballot, a letter, a phone call | NPC goals; a keyed draw weighted by personality |

   Ordinary commands are levers too: a bill from the arc's `answers` templates is read by `law_in_force`, and NPC agendas already favour those templates.
6. **Heat: escalation and relief.** One number per run, 0 to 1, in the arc's own domain. It rises by the stage's `rise` each week and by moves; beats done lower it (`relief`); a `floor` ties it to a simulated reading where there is one (job loss, a scandal's own heat). Edges read it (`arc_heat`), the director's tension reads it in place of raw crisis severity, overheard lines grow likelier with it, and street dressing (S7) reads it. It changes no material outcome by itself. The player meets it as people and places, never as a bar.
7. **Exits.** A terminal stage with an `exit` block: at least three per arc, and at least one reachable with no player act. An exit is reached like any stage. Its `effects` are effect requests to the owning systems (§4.3), and may be gated on marks, so two players at one exit still leave different memories. An exit never repeats what the simulation already did: when a plant closes, the economy removed the jobs; the exit boards the gate, settles who remembers what, and may start a follow-up arc.
8. **On the spine.** `Thread::Arc(ArcId)`: the raise event opens it, each stage entry `Continues`, each beat is `Prompted`, the exit `Ended`. Sifting, the press, Legacy and conversation callbacks read arcs through the graph with no plumbing of their own. **(A1)** A beat's own event *notes* the thread, so the log links it `Continues` to the run's latest event, with its own root beside it (the player's act, the cast person who chose to, the decision's answer). What a beat opens in another domain (a decision, an ask) is `Prompted` by the run's latest event. The raise event's cause is the event that made `raise` true (the strike), linked `Caused`.

**What happens to what exists (decided):**
- **Storylines fold in.** A storyline is already a staged run with a cast, decisions and branches, and two such machines would drift. `storylines.jsonc` and `StorylineRun` become arcs: the plant storyline becomes §7.1; the rival and favour-debt storylines become `arc.rival` and `arc.favour_debt` with their stages kept. A save migration maps a running storyline to the arc stage of the same id, else ends it as faded. E4's six planned storylines are written as arcs. **(A2)** Built: the storyline machinery is gone. Until the plant has its fuse (A5a) the plant storyline is `arc.plant_closure`, its stages kept; each folded arc gained the exits a storyline never had (three each).
- **Asks stay, and arcs use them.** An ask is one person's request with needs in order, a deadline and a price for breaking it. That atom works, and its chain was fixed in W1. Something becomes an arc only when it needs branching or escalation; then it offers asks as beats.
- **Dilemmas and scenes stay** as the decision atom (`storyline_only` becomes `arc_only`). **(A2)** Done (the old spelling still reads). A dilemma that also stands alone may be an arc's beat (the layoffs, the favour called in): the spotlight leaves it to the run while a run that names it lives.
- **The crisis registry stays as the indicator layer.** `crises.jsonc` still says when a town is in a jobs or recovery crisis; concerns, NPC agendas and conditions read it. A crisis is a level; an arc is the story of one matter inside it.
- **Secrets (E9) stay hidden facts** in the media domain; an investigation of one is an arc.
- **Opening frames (DEC-101 §2) are a block on the arc** (`frame`), not a second registry. Every opening is an arc, including the non-crisis hooks (a resignation, a rival's scandal, a budget fight).

**Where arcs run.** Causes run everywhere: plants are reviewed and rivers rise in every town. Arc runs exist only in focus jurisdictions (`state.focus()`), where someone can see them, and in the audition's forks (§6). Elsewhere the owning system resolves the matter by its own rule.

## 2. Agency

- **Every stage has a drift edge and a lever edge.** Drift is what happens when nobody acts: usually the more escalated stage. The lever edge needs a mark or a state change that an act produces. The doctor refuses a stage without both (§4.5).
- **Timing is a lever.** Acting in *signs* is cheap and quiet (a word with the manager while nothing is public); after the break the same act costs more and is watched. Some beats exist only before a stage (`"until": "notice"`).
- **A passive player gets the drift path with its cast.** Decisions take their defaults at the deadline (the player's inaction, as today), asks lapse, moves run. The arc still reaches an exit, and its gated effects record the absence: who noticed, where approval fell.
- **NPCs push.** A stage may grant goals to roles while it lasts (`goals`), so the existing NPC AI works on it (the steward organises, an official champions the answering bills). `move` beats are their visible acts, each with real effects, each a keyed draw whose weight reads their personality and their opinion of the player. A cultivated ally moves later and gentler; an enemy moves first.
- **What can be learned in advance.** A hidden *signs* stage shows only as ambient facts: a worry rising in the ward, an overheard line, a guarded reply, a small press item, a cue at the place. A new ledger subject, `Arc { arc }`, gates the view:

| Level | The player sees | Typical source |
|---|---|---|
| unknown | Only the ambient signs | |
| heard | A lead in the notebook: the matter exists, and where | an overheard line, small talk |
| known | The stage, the cast, the clock | the break itself (always, for a focus arc); a `talk` reply that agrees |
| detailed | The next stage and its window, the real numbers (the gap, the count), which reply each person would give | trust, a favour called in, the arc's secret |

   The ledger gates views and framing only. No edge, raise or move reads it (the existing `ledger` rule in `Condition::needs`). The break is never hidden (DEC-006).
- **The passive and engaged ends must differ.** This is the audition's agency term (§6) and a CI gate for every arc.

## 3. The director

DEC-033 holds: the director times, and never decides whether. For arcs that means exactly five powers, each logged as a `DirectorIntervention`. **(A2)** Built: 1, 2 and 5 (`narrative/director/arcs.rs`, `narrative/offered.rs`); 4 was there already (`Chance`); 3 waits for the first opportunity arc (`opportunity` is parsed and inert). What each came to is in §13.

1. choose which running arc **leads** the chapter (relevance × novelty, the existing pick);
2. place a **director-owned clock** inside its content range (a keyed draw, biased so the turn falls in the chapter's `climax_window`); a clock the simulation owns is never moved;
3. time the **arrival of an opportunity arc** (`"opportunity": true`: a buyer, a developer, a retirement) inside its window once its `raise` holds;
4. hold a **hazard** or set a hidden fact's **discovery date** within the windows it already has;
5. choose **when a beat without a clock is presented** (at a peak of the pacing cycle).

It can never start an arc whose `raise` does not hold, block or force an edge, pick an exit, or change an effect.

- **Budgets.** Per chapter: one lead arc, at most one background arc, plus the rival's arc (which leads only campaign chapters). `director.jsonc`: `arcs_active` 2 replaces `storylines_active`; `crisis_cap` 2 stays, so hazards are held while two arcs run. A background arc keeps moving (the world does not wait), but its decisions are minor weight unless clocked. A clocked decision opens in any phase, as `raised_by` decisions do today, and spends the significant budget. **(A2)** As built: `arcs_active` caps the runs in a jurisdiction that are not standing arcs (`standing: "session" | "campaign"` on the template); a standing arc is raised whatever else runs. The lead's decisions and any decision under a clock are significant (minor once the chapter's significant budget is spent; waiting once both are, and while `max_pending` decisions wait on the player). A decision in a stage that ends on its clock is due the day before the clock.
- **The climax.** A chapter's climax is the lead arc's turn stage, reached inside the chapter; its decision has weight `climax`. Tension reads `max(heat)` of focus arcs in place of `crisis_weight × severity`. **(A2)** As built: a run records the first `climax` stage it enters (`ArcRun::turn`); a session's climax is counted the day its lead's turn is reached, decision or none (a player out of office has no vote in the budget round, and the chapter still turns). The lead is picked among the runs that can still turn in the chapter (a turn ahead by the template's edges, or reached inside it): a material arc (theme `crisis`), then the standing arc of the chapter's kind, then the rest, by the existing pick. If the lead never turns, the chapter takes the first turn reached inside it as it closes. A campaign's climax stays the election; the rival's showdown is a significant decision. Tension reads `max(heat, crisis severity)`: a crisis no arc tells yet (the registry's, until A6) still counts.
- **The pacing cycle** (ENGAGEMENT §4.3) is kept. A lead arc maps onto it: signs and levers in build-up, the break and the turn as peaks, the aftermath in the post-climax relax. A stage entry counts as a beat for the quiet rescue.
- **A calm world still has a chapter with shape.** Two guarantees, both simulated:
  - **Standing arcs.** A session chapter *is* the budget year of the player's main post, so `arc.budget_round` can always lead it: the estimates, the group's line, an amendment, the vote. Its turn has four variants chosen by state (a hole to fill, a surplus to place, a revolt on the line, the executive against the body), and climax memory counts the variant. A campaign chapter always has `arc.rival`, whose turn is the election. **(A2)** Built (`arcs/budget_round.jsonc`, `arcs/rival.jsonc`); the four variants and the signals that pick each are in §13.
  - **Opportunity arcs** arrive when the director has room for them (power 3): the developer, the buyer, the institution.

  A material arc outranks both. The pacing test's "a climax in 85% of chapters" (passing today by one chapter, with DEC-097's climax fallback) becomes ≥ 95% with `climax_fallbacks` at 0, and the fallback is deleted. **(A2)** Done: 84 of 84 chapters on the test's seeds (71 of 84 before, with the fallback), 84 of 84 on seeds 13 to 24 (69 of 84 before). The fallback, the forced climax peak and climax memory by template are deleted; no dilemma is a climax on its own.

## 4. Data and state

### 4.1 Content: `content/base/arcs/*.jsonc` (`kind: "arcs"`)

```jsonc
{ "id": "arc.<name>", "title_key": "arc-<name>", "themes": ["crisis"], "opportunity": false,
  "tiers": ["municipality", "region"],             // where it may run; the doctor checks its conditions against each
  "raise": [ /* Condition, evaluated at a jurisdiction; never the ledger */ ],
  "roles": [ { "id": "steward", "cast": { /* ArcCast */ }, "optional": false } ],
  "sides": { "ally": "steward", "opponent": "manager" },     // for stand-in players and the opening frame
  "heat": { "start": 0.2, "floor": { "indicator": "job_loss", "scale": 4.0 } },   // or "floor": { "scandal": "<role>" }
  "answers": ["policy.closure_response"],          // templates NPC agendas favour while it runs
  "frame": { "post": { "seat_for": "hood" }, "secret": "arc_clock", "pitch_key": "pitch-<name>" },
  "cooldown_days": 730, "min_career_days": 0, "requires": [],   // engine features it needs ("parties.membership")
  "stages": [
    { "id": "notice", "key": "arc-<name>-notice", "hidden": false, "climax": false,
      "clock": { "from": "subject" },              // or { "days": [28, 42] }
      "rise": 0.06, "goals": [ { "role": "steward", "goal": "goal.organise_workers" } ], "enter": [ /* ArcEffect */ ],
      "beats": [ { "id": "gate", "type": "visit", "at": "plant", "verb_key": "…", "outcome_key": "…", "when": [], "until": null,
                   "once": true, "sets": "at_gate", "relief": 0.05, "teaches": "known", "effects": [ /* ArcEffect */ ] } ],
      "next": [ { "to": "talks", "when": [ { "type": "arc_mark", "mark": "table" } ], "after_days": 0 }, { "to": "decision", "clock_within_days": 14 } ] },
    { "id": "closed", "key": "arc-<name>-closed", "exit": { "headline_key": "…", "legacy_key": "…", "follow_up": "arc.buyer", "follow_days": [180, 540],
        "effects": [ { "type": "place_state", "place": "plant", "state": "boarded" }, { "when": [ /* marks */ ], "type": "opinion", "role": "steward", "delta": 6 } ] } } ] }
```

- **Roles bind to real things** through `ArcCast`, relative to the arc's jurisdiction, never the player's town: `subject` (what raised it: a plant, a disaster, a fact), `occupation` (with `near` or `works_at`), `post` (a holder of a post there by `power`, or by party and faction; never an office's name), `area` (one level down, by `pick`: `most_exposed`, `most_workers_of`, `least_pleased`), `place` (by kind, `near` an area), `jurisdiction` (`parent`), `party_group`, `discoverer`. Each gives Fluent arguments (`$steward`, `$steward_role`, `$plant`, `$hood`); a post's role word comes from the constitution.
- **New conditions** (U1, each with its tier and arguments): `arc_mark`, `arc_heat`, `arc_days`, and `arc` (a run of this template is at this stage here: for overheard lines and topics); and the readings the first three arcs need: `plant_review` (gives `$parent`, `$day`, `$jobs`, `$gap`, `$kept`), `hazard_forecast`, `displaced` and `recovered` (an area), `hidden_fact` and `case` (a slot), `holds_post` (a slot), `group_opinion` (the share of a party group whose opinion of a slot is under a line). `indicator`, `crisis`, `disaster` and `law_in_force` are promoted from local-only to tier-generic by evaluating over `Area::In { at }`. **(A1)** Built: `arc` (with `here`: where a role of the run is), `arc_mark`, `arc_heat`, `arc_days`, `displaced`, `recovered`, and a `hazard` on `disaster`. Not built yet: the promotion to tier-generic (so the flood runs in towns only), `hazard_forecast`, `plant_review`, `hidden_fact`, `case`, `holds_post`, `group_opinion`. The `area` pick `most_exposed` reads the share of homes the hazard damaged, then who is away, then exposure. An arc also lists `given` (conditions on its cast as bound) and `facts` (the readings its text is given); see §12.
- **Text keys by kind**, so the prose lint knows the register: `arc-*` (the story so far: one or two sentences, present tense, under 130 characters, a fact and a date), `pitch-*` (the draft's line), `scene-*`, `dilemma-*`, `ask-*`, `talk-arc-*`, `visit-arc-*` (a button), `outcome-arc-*`, `headline-arc-*`, `heard-*`, `skip-item-arc-*` (a move, as Today and the skip summary report it), `legacy-arc-*`.

### 4.2 Rust sketch

```rust
// mandate-content/src/arcs.rs (the schema's source)
pub struct ArcDef { pub id: ContentKey, pub title_key: String, pub tiers: Vec<TierKey>, pub opportunity: bool, pub raise: Vec<Condition>,
    pub roles: Vec<ArcRoleDef>, pub sides: Option<Sides>, pub heat: HeatDef, pub answers: Vec<ContentKey>, pub frame: Option<FrameDef>,
    pub stages: Vec<StageDef>, pub cooldown_days: u32, pub min_career_days: u32, pub requires: Vec<String>, pub themes: Vec<String> }
pub struct StageDef { pub id: String, pub key: String, pub hidden: bool, pub climax: bool, pub clock: Option<ClockDef>, pub rise: f32,
    pub goals: Vec<GoalGrant>, pub enter: Vec<Gated<ArcEffect>>, pub beats: Vec<ArcBeat>, pub next: Vec<EdgeDef>, pub exit: Option<ExitDef> }
pub struct EdgeDef { pub to: String, pub when: Vec<Condition>, pub after_days: u32, pub clock_within_days: Option<u32> }
pub enum BeatKind { Decision { dilemma: ContentKey, weight: Weight, marks: BTreeMap<String, String> }, Ask { template: ContentKey, asker: String },
    Talk { with: Who, choice: ArcChoiceDef }, Visit { at: String, verb_key: String, outcome_key: String },
    Press { headline_key: String }, Heard { line: ContentKey }, Move { actor: String, weight: f32, line_key: String } }
// mandate-sim/src/arcs/ (a domain of its own: state, step, cast, beats, effects, view, score)
pub struct ArcsState { pub runs: Vec<ArcRun>, pub next_id: u32, pub cooldowns: BTreeMap<(ContentKey, Jurisdiction), SimDate>, pub past: Vec<ArcSummary> }
pub struct ArcRun { pub id: ArcId, pub template: ContentKey, pub at: Jurisdiction, pub cast: Vec<CastEntry>, pub stage: u8, pub since: SimDate,
    pub started: SimDate, pub clock: Option<SimDate>, pub heat: f32, pub marks: Vec<u8>, pub done: Vec<BeatDone>, pub root: EventId, pub ended: Option<(SimDate, u8)> }
```

`ArcBeat` is named so because `narrative::BeatDef` is already a scene's spoken line. `Actor` gains `Place`, `Plant`, `Party` and `Jurisdiction`.

**(A1) As built.** The types are in `mandate-content/src/arcs.rs` and `mandate-sim/src/arcs/mod.rs`; where they differ from the sketch:
- A run names its stage by id and its marks by name (strings), so a content update that reorders stages cannot move a run; `done` holds one entry a beat; `open` holds the decision and ask beats asked of their owners and not yet settled; `base` holds the readings taken as it was raised (what `recovered` is measured against); `began` is the root event's day. An ended run leaves `runs` for `past` at once. A run is about 1 KB of JSON, not 200 bytes.
- The arcs' cast has its own actor type (`ArcActor`: a person, an `Area`, a place, a jurisdiction); the narrative's `Actor` is unchanged. A decision or an ask a run opens carries the people and the neighbourhoods of its cast.
- A decision beat maps an option to what it does to the run with `on: { "<option>": { "sets", "heat", "effects" } }`, not `marks`: the dilemma's own effects are the narrative's and cannot post an arc's requests.
- `ArcDef` gains `given` (conditions on the cast as bound: a matter too small to tell raises nothing) and `facts` (conditions evaluated for their arguments: the readings the text is given). It has no `frame` yet (A8), a beat no `until` or `same_as`, and a talk beat's `with` is one role.
- `ArcsState.deferred` holds requests whose applier runs monthly until the month turns (they would otherwise wait outside the state, and a save between would lose them).

**The step.** Arcs are their own domain, stepped weekly before the director (clocks daily: a date comparison). **(A1)** It runs first in the day (the NPC actions phase), not in the director's phase: every request a run posts is then applied by its owner's system the same day, and none is left for the end-of-turn settle step, which drops what it does not know. Until A2 the step also opens a stage's decisions and asks itself (requests `OpenDecision` and `OfferAsk`, applied by the narrative and the asks), on the day the stage is entered. Raise and cast new runs in focus jurisdictions; for each run in id order, apply `rise` and the floor, draw moves, then take the first edge that holds. It reads through `Reads`, writes `ArcsState`, and posts effect requests and events. The director reads `ArcsState` and writes only its own domain: which arc leads, which beats are open. Arcs run with the spotlight off and under the Unfiltered profile; only presentation changes.

### 4.3 Effects, and who applies them

An `ArcEffect` is `narrative::EffectDef` plus the arc kinds. It reuses the requests that exist: `sector_shock`, `housing_damage`, `displacement`, `opinion` (anyone's of anyone, or a group's), `remember`, `favour`, `approval`, `record`, `learn`, `lead`, `removal`, `valence`, and the player's `draft`, `hidden`, `standing`, `slots`, `money`. Six requests are new, each priced and applied by its owner:

| Effect | Owner | What it does |
|---|---|---|
| `plant_review { plant, act: extend \| advance \| concede \| offer }` | economy | Moves the review's date or its gap; the economy still decides |
| `recovery { area, prep \| repair }` | environment | Less damage from a forecast strike; a faster repair rate for a while |
| `treasury { at, amount, purpose }` | finance | A one-off spend or grant in a jurisdiction's budget |
| `gov_approval { area, delta }` | population | Approval of the jurisdiction's government in an area |
| `faction_shift { party, faction, delta }` | politics | A wing's strength; the platform, and so the party line, follow |
| `place_state { place, state }` | places | Boarded, on short time, marked; the dress pass shows it (DEC-099) |

**(A1) As built.** `ArcEffect` is its own short list, not `EffectDef` plus arc kinds: `opinion`, `approval` (an area role), `favour`, `record`, `standing`, `recovery`, `displacement`, and `heat` (the run's own). `recovery { area, repair, months }` was brought forward from A5b (the environment applies it: `Effect::Recovery`, `EnvironmentState::repairs`), because without it the flood had no material lever; its `prep` half waits for the forecast. `gov_approval` and `place_state` parse and are *not yet*: a template that uses one is never raised (`mandate_content::arcs::not_yet`). Two requests were added so that a run can open its own beats: `OpenDecision` (narrative) and `OfferAsk` (asks).

### 4.4 Saved, bounded, deterministic

- **Saved:** `ArcsState` (one version bump, with the storyline migration). A run is about 200 bytes; `past` keeps 40 summaries (template, place, exit, dates, people, root event). Stage entries and exits are `Notable` events on the spine and age out with everything else. Available beats, the view, the "signed" count and the next stage are derived, never stored.
- **Determinism:** a new named stream `Stream::Arcs`, keyed draws only: `keyed(seed, Arcs, [tag, arc, stage, beat, week])` for moves, clocks and cast ties. **(A1)** The draws are keyed on the existing `Stream::Misc` with the arcs' own tags (`KEY_CLOCK`, `KEY_MOVE`): every named stream's generator is saved in `State.rng`, so a new one would change the hash of every game, arc content or none. A cast draws nothing: ties go to simulated state, then to the lowest id. No draw depends on what the player knows. Runs are visited in id order and edges in listed order. A fork (§6) is a copy; dropping it leaves the original's state and hash untouched.

### 4.5 What the doctor lints (`content doctor`, kind `arcs`)

- **Graph:** `arc-unreachable-stage`; `arc-no-exit-from` (a stage from which no exit can be reached); `arc-stuck` (a stage that does not end on time, §1.4); `arc-few-exits` (under three); `arc-no-passive-exit`.
- **Play:** `arc-nothing-to-do` (a visible stage with no beat a player can take in quick mode); `arc-no-lever` and `arc-no-drift` (§2); `arc-mark-unset`, `arc-mark-unread`; `arc-heat-unreachable` (a threshold the stage's rise cannot reach before its clock).
- **Consequences:** `arc-exit-flavour` (an exit whose effects are only opinions and memories: it needs a material request or a follow-up).
- **Binding:** `arc-role-uncast` (a beat or key names a role the template lacks, or an optional role without a guard); `arc-tier` (a local-only condition in a template that lists another tier); `arc-names-office` (an office or body named outside the constitution).
- **Text:** every key exists; every argument a key uses is given by the cast or by a condition on that path (the existing check); `prose-habit` by key prefix.
- **Dynamic, with `--seeds`:** `arc-never-raised` (no generated world raised it in N seeds × Y years, with and without the lab planting its cause); `arc-cast-fails` (raised but uncastable in over a fifth of worlds); `arc-stage-unvisited` and `arc-exit-unvisited` (across the three stand-ins of §6); `arc-too-long` (the engaged median runs past its chapter).

## 5. Tier-generic and parity

**One template, three tiers (DEC-098).**
- A run is keyed by `Jurisdiction`. Conditions are evaluated *at* it: the evaluator's context gains `at`, and old content defaults to the player's home through `player::home`. Nothing reads `player.municipality`.
- People are cast by occupation, or by `post` through powers and party. Text names a post only through its role word (`$official_role`) and never names a body: "the vote", "the branch".
- Places and areas step down one level: the `hood` of a town's arc is a neighbourhood; of a region's, its worst-hit town; of the nation's, a region. A place role in a wider arc binds in that area's seat. `tiers` says where a template makes sense (a plant review is a town's or a region's matter; a party row runs anywhere), and the doctor checks every condition against every listed tier.
- Scale lives in content numbers (`heat.floor.scale`, clock ranges) under the per-tier overrides DEC-098 already plans.

**Every beat in quick mode and through Places (DEC-051).** An arc has one view, `ArcView` (`QueryDto::Arcs`, redacted by the ledger): the title, the story so far, the clock, the cast, and **what you can do**: every available beat as a located action with its place, person and cost.

| Beat | Walking | Quick mode and Places |
|---|---|---|
| `decision` | the scene at its place | the pocket's decisions (as today) |
| `ask` | the asker, in person | Today |
| `talk` | the person, where their routine puts them | Places → the person → Talk; the arc view lists the choice |
| `visit` | the place | Places → the place → the verb |
| `press` | a paper on a stand | the pocket's paper |
| `heard` | standing in the diner | the place's overheard lines in Places; and a talk topic with the same condition teaches whatever a heard line teaches |
| `move` | seen if you are there | Today, the paper, the skip summary |

The stand-ins of §6 send only commands and never walk, so every stage they visit is proof of parity; `arc-stage-unvisited` is the gate.

## 6. The audition (DEC-101 §5)

A pure function of the world after its history, the content and the new-game parameters. It runs in the engine, in the worker, at world start.

1. **Candidates.** For every jurisdiction where the constitution's `career` block offers an entry post, test every arc's `raise` and `frame`. Rank by a static prior (heat floor × cast quality × novelty against `recent_openings`). Keep K: 8 in a small world, 5 in a medium, 3 in a large. K comes from content by size class, never from measured time, which would break determinism. No two share a town; at most two share a template. If fewer than three clear the prior, advance the world a month and rescan, up to 36 months; then let opportunity arcs arrive.
2. **Forks.** For each candidate: copy the state, apply the frame (the stand-in holds the post from turn 1), and play 39 weeks.
3. **Stand-ins.** Three directive sets on the skip proxy (`proxy.rs`: it already sends only `PlayerCommand`s, knows only what a player would, and draws by key). *Passive* ends each turn: defaults fire, asks lapse. *Engaged* takes every lever the arc view lists, for the frame's ally. *Contrarian* does the same for the opponent and declines the ally's asks. They live in `mandate-sim` because the lab's bots (`mandate-cli/src/bots`) are not in the WASM build. The lab gains `Driver::StandIn`, and the CI grader also runs the bots `idle`, `mixed` and `explorer` as a check on the stand-ins.
4. **Two passes.** Passive for all K; engaged and contrarian for the best five on E × V × P. Eighteen forks, where three stand-ins for all eight would be 24.
5. **The score**, each term 0 to 1:

| Term | Weight | Measure |
|---|---|---|
| Agency A | 0.30 | 0.6 if the passive and engaged exits differ; 0.2 if contrarian and engaged differ; 0.2 × the distance between the passive and engaged worlds at week 39 over a fixed vector (jobs in the subject's sector, homes, people away, government approval in the area, the player's approval in the seat, the cast's opinions, the treasury) |
| Escalation E | 0.25 | Passive run: stages entered ÷ 3 (capped at 1), × peak heat, halved if the turn was never reached |
| Visibility V | 0.20 | The share of stage entries and exits that produced something seen without seeking: a Today item, a story, a place's state, a top-three worry in the seat |
| Pacing P | 0.15 | Engaged run: a beat on turn 1 (required), the turn within 8–14 turns, no gap over 3 turns, an exit by week 39 |
| Variety N | 0.10 | Distance from the last three openings in template, industry or hazard, post and town class |

   Floors: A ≥ 0.3, V ≥ 0.4 and the turn-1 beat, or the candidate is dropped. Elections stay out of the vector until audit fix 1 lands, because votes do not yet follow conditions.
6. **The draft.** The best three by score, no two of one template, one non-material arc if any qualifies. Each is a real place with one line built from day-0 facts only (the `pitch-*` keys of §7). "Surprise me" takes the first. Then the copies are dropped and play starts from the unforked state with the chosen frame. Nothing from a fork is shown or kept, except the scores in the run log for the lab.
7. **Variety is a parameter.** `NewGameParams.recent_openings` (up to three tags, from the device's Legacy record) is part of the run file, so the same seed with the same parameters gives the same draft. A shared seed passes none.
8. **Cost.** Task A7 measures it. Until then these are estimates from one known number (a small world runs 20 years in about a second with nobody playing): a scan under 5 ms and a forward scan about 4 ms a month; a fork 10–30 ms (a state copy, today through `Engine::from_state`); a 39-week run about 40 ms idle and about 100 ms with a stand-in's turns, because the home town steps its NPCs every turn. Eighteen runs: about 2 s native, about 6 s in WASM on a mid phone. Budget: 8 s on the reference phone for a small world, one fork per worker message under the history animation.
9. **The same scorer grades content in CI.** `mandate-cli arcs grade --seeds 16 [--arc id]`: for each template, find worlds that raise it or plant its cause (`Intervention`), run the stand-ins and the three bots, and print the five terms and coverage. Gates: every stage and exit visited by some stand-in; passive and engaged exits differ in ≥ 60% of worlds; median A ≥ 0.4; no exit takes over 70% of engaged runs; the engaged median reaches the turn within one chapter. Nightly at first; blocking once six arcs pass. It folds into E12.

## 7. Three worked arcs

Fills in the traces: Mireenwick, its neighbourhood Quarryrow, Sofi Brook (union steward), Rafe Garrowby (plant manager), Halle Talliswell (community organiser), Celia Janssenfield (councillor), the *Corriton Ledger*. `$plant` is the workplace's name as a noun phrase with its article ("the Garrowby cannery", note 17) and never opens a sentence; `$parent` is its generated owning firm. Text is listed by kind; the key is the prefix plus the id shown. Each choice also has a line the player says; one is shown per arc.

### 7.1 `arc.plant_review`: the review at the plant

Needs the economy to give a closure a fuse (task A5a): a plant whose demand stays weak goes *under review* in private, is *announced* 6–10 weeks before a decision date, and on that date is lifted, kept in part or closed by the economy's own rule on its gap.

```jsonc
{ "id": "arc.plant_review", "title_key": "arc-plant", "themes": ["crisis"], "tiers": ["municipality", "region"], "cooldown_days": 730,
  "raise": [ { "type": "plant_review", "slot": "plant", "state": "open" } ],
  "roles": [ { "id": "plant", "cast": { "type": "subject" } }, { "id": "manager", "cast": { "type": "occupation", "occupations": ["occupation.plant_manager"], "works_at": "plant" } },
    { "id": "steward", "cast": { "type": "occupation", "occupations": ["occupation.union_steward", "occupation.shop_steward"], "works_at": "plant" } },
    { "id": "hood", "cast": { "type": "area", "pick": "most_workers_of", "place": "plant" } }, { "id": "official", "cast": { "type": "post", "power": "propose_budget" }, "optional": true } ],
  "sides": { "ally": "steward", "opponent": "manager" }, "heat": { "start": 0.15, "floor": { "indicator": "job_loss", "scale": 4.0 } },
  "answers": ["policy.closure_response", "policy.small_business_grants"], "frame": { "post": { "seat_for": "hood" }, "secret": "arc_clock", "pitch_key": "pitch-plant" },
  "stages": [
    { "id": "signs", "key": "arc-plant-signs", "hidden": true, "beats": [ { "id": "overtime", "type": "heard", "line": "overheard.plant.overtime", "teaches": "heard" }, { "id": "visitors", "type": "heard", "line": "overheard.plant.visitors", "teaches": "heard" },
        { "id": "need", "type": "talk", "with": "manager", "choice": { "label_key": "talk-arc-plant-need", "say_key": "talk-arc-plant-need-say", "replies": [ { "key": "talk-arc-plant-need-yes", "when": [ { "type": "trust", "slot": "speaker", "min": 0.5 } ], "agrees": true, "sets": "need", "teaches": "detailed" }, { "key": "talk-arc-plant-need-no" } ] } } ],
      "next": [ { "to": "notice", "when": [ { "type": "plant_review", "slot": "plant", "state": "announced" } ] }, { "to": "scare", "when": [ { "type": "plant_review", "slot": "plant", "state": "lifted" } ] } ] },
    { "id": "notice", "key": "arc-plant-notice", "clock": { "from": "subject" }, "rise": 0.06, "goals": [ { "role": "steward", "goal": "goal.organise_workers" } ], "beats": [ { "id": "press", "type": "press", "headline_key": "headline-arc-plant-notice" },
        { "id": "stand", "type": "decision", "dilemma": "dilemma.plant_layoffs", "weight": "significant", "marks": { "broker_talks": "table" } },
        { "id": "gate", "type": "visit", "at": "plant", "verb_key": "visit-arc-plant-gate", "outcome_key": "outcome-arc-plant-gate", "sets": "at_gate", "relief": 0.05, "effects": [ { "type": "opinion", "role": "steward", "delta": 8, "memory": "stood_with_me" }, { "type": "awareness", "area": "hood", "delta": 0.04 } ] },
        { "id": "need", "type": "talk", "with": "manager", "same_as": "signs.need" },
        { "id": "official_table", "type": "move", "actor": "official", "weight": 0.3, "when": [ { "type": "goal", "slot": "official", "goal": "goal.champion_fix" } ], "sets": "table", "line_key": "skip-item-arc-plant-table" },
        { "id": "ballot", "type": "move", "actor": "steward", "weight": 0.6, "when": [ { "type": "arc_days", "min": 14 }, { "type": "arc_heat", "min": 0.5 } ], "sets": "balloted", "line_key": "skip-item-arc-plant-ballot" } ],
      "next": [ { "to": "talks", "when": [ { "type": "arc_mark", "mark": "table" } ] }, { "to": "walkout", "when": [ { "type": "arc_mark", "mark": "balloted" } ] }, { "to": "decision", "clock_within_days": 14 } ] },
    { "id": "walkout", "key": "arc-plant-walkout", "rise": 0.1, "enter": [ { "type": "sector_shock", "sector_of": "plant", "factor": 0.9, "months": 1 } ], "beats": [ { "id": "press", "type": "press", "headline_key": "headline-arc-plant-walkout" }, { "id": "picket", "type": "ask", "template": "ask.join_picket", "asker": "steward" },
        { "id": "call", "type": "move", "actor": "manager", "weight": 0.7, "when": [ { "type": "arc_days", "min": 7 } ], "line_key": "skip-item-arc-plant-call", "effects": [ { "type": "plant_review", "plant": "plant", "act": "advance", "days": 14 } ] },
        { "id": "table", "type": "talk", "with": "steward", "choice": { "label_key": "talk-arc-plant-table", "say_key": "talk-arc-plant-table-say", "replies": [ { "key": "talk-arc-plant-table-yes", "when": [ { "type": "any", "of": [ { "type": "arc_mark", "mark": "at_gate" }, { "type": "trust", "slot": "speaker", "min": 0.6 } ] } ], "agrees": true, "sets": "table" }, { "key": "talk-arc-plant-table-no" } ] } } ],
      "next": [ { "to": "talks", "when": [ { "type": "arc_mark", "mark": "table" } ] }, { "to": "decision", "clock_within_days": 14 } ] },
    { "id": "talks", "key": "arc-plant-talks", "rise": 0.02, "beats": [ { "id": "terms", "type": "decision", "dilemma": "dilemma.plant_terms", "weight": "significant", "marks": { "rates": "drafted", "both": "drafted" } },
        { "id": "short", "type": "talk", "with": "steward", "choice": { "label_key": "talk-arc-plant-short", "say_key": "talk-arc-plant-short-say", "replies": [ { "key": "talk-arc-plant-short-yes", "when": [ { "type": "any", "of": [ { "type": "arc_mark", "mark": "at_gate" }, { "type": "trust", "slot": "speaker", "min": 0.5 } ] } ], "agrees": true, "sets": "conceded", "effects": [ { "type": "plant_review", "plant": "plant", "act": "concede", "wage_factor": 0.9 } ] }, { "key": "talk-arc-plant-short-no" } ] } },
        { "id": "more", "type": "talk", "with": "manager", "choice": { "label_key": "talk-arc-plant-more", "say_key": "talk-arc-plant-more-say", "replies": [ { "key": "talk-arc-plant-more-yes", "when": [ { "type": "any", "of": [ { "type": "arc_mark", "mark": "conceded" }, { "type": "arc_mark", "mark": "drafted" } ] } ], "agrees": true, "sets": "extended", "effects": [ { "type": "plant_review", "plant": "plant", "act": "extend", "days": 60 } ] }, { "key": "talk-arc-plant-more-no" } ] } } ],
      "next": [ { "to": "reprieve", "when": [ { "type": "arc_mark", "mark": "extended" } ], "after_days": 7 }, { "to": "decision", "clock_within_days": 14 } ] },
    { "id": "decision", "key": "arc-plant-decision", "climax": true, "rise": 0.05, "beats": [ { "id": "eve", "type": "decision", "dilemma": "dilemma.plant_eve", "weight": "climax" } ],
      "next": [ { "to": "saved", "when": [ { "type": "plant_review", "slot": "plant", "state": "lifted" } ] }, { "to": "kept", "when": [ { "type": "plant_review", "slot": "plant", "state": "kept" } ] }, { "to": "closed", "when": [ { "type": "place_closed", "slot": "plant" } ] } ] },
    { "id": "closed", "key": "arc-plant-closed", "exit": { "headline_key": "headline-arc-plant-closed", "legacy_key": "legacy-arc-plant-closed", "follow_up": "arc.buyer", "follow_days": [180, 540], "effects": [ { "type": "place_state", "place": "plant", "state": "boarded" },
        { "when": [ { "type": "arc_mark", "mark": "at_gate" } ], "type": "opinion", "role": "steward", "delta": 6 }, { "unless": [ { "type": "arc_mark", "mark": "at_gate" } ], "type": "opinion", "role": "steward", "delta": -10, "memory": "ignored_me" }, { "unless": [ { "type": "arc_mark", "mark": "at_gate" } ], "type": "approval", "area": "hood", "delta": -0.03 } ] } },
    { "id": "kept", "key": "arc-plant-kept", "exit": { "headline_key": "headline-arc-plant-kept", "legacy_key": "legacy-arc-plant-kept", "effects": [ { "type": "place_state", "place": "plant", "state": "short_time" }, { "when": [ { "type": "arc_mark", "mark": "conceded" } ], "type": "record", "delta": 0.05 }, { "when": [ { "type": "arc_mark", "mark": "conceded" } ], "type": "favour", "role": "manager", "owed_to_player": true } ] } },
    { "id": "saved", "key": "arc-plant-saved", "exit": { "legacy_key": "legacy-arc-plant-saved", "effects": [ { "type": "gov_approval", "area": "hood", "delta": 0.02 }, { "when": [ { "type": "arc_mark", "mark": "drafted" } ], "type": "record", "delta": 0.08 } ] } },
    { "id": "reprieve", "key": "arc-plant-reprieve", "exit": { "follow_up": "arc.plant_review", "follow_days": [30, 60], "effects": [ { "type": "favour", "role": "manager", "owed_to_player": true } ] } },
    { "id": "scare", "key": "arc-plant-scare", "exit": { "follow_up": "arc.plant_review", "follow_days": [365, 900], "effects": [ { "when": [ { "type": "arc_mark", "mark": "need" } ], "type": "learn", "subject": { "type": "person", "role": "manager" }, "level": "detailed" } ] } } ] }
```

- **Story so far** (`arc-plant-…`). *signs:* "Overtime has stopped at { $plant }, and nobody has said why." *notice:* "{ $parent } will decide on { $day } whether { $plant } stays open. { $jobs } jobs depend on it." *walkout:* "{ $steward } has members on the gate at { $plant }. The line is stopped." *talks:* "{ $manager } and { $steward } are talking. { $parent } still decides on { $day }." *decision:* "The call from { $parent } comes on { $day }."
- **Exits.** *closed:* "{ $parent } closed { $plant } on { $day }. { $jobs } jobs went with it." *kept:* "{ $parent } kept { $plant } open on short time. { $kept } of { $jobs } jobs stay." *saved:* "{ $parent } took { $plant } off its list. Nobody was laid off." *reprieve:* "{ $parent } put its decision back to { $day }." *scare:* "Overtime is back at { $plant }. The review ended before anyone was told of it."
- **Pitch** (`pitch-plant`). "{ $place }. { $parent } decides on { $day } whether { $plant } stays open. You hold the seat for { $hood }."
- **Overheard** (`heard-plant-…`). *overtime:* "No overtime at { $plant } for a month now. And the agency lads weren't asked back on Monday." *visitors:* "Two men in a hire car spent all Tuesday at { $plant }. They didn't go near the canteen."
- **At the notice** (`scene-plant-notice-…`). *steward:* "They told us by letter at ten, with the line still running. We meet at the hall on Thursday. Come to the gate at six first, and let them see you." *manager:* "{ $parent } decides on { $day }. They'll look at one page, and the wage bill is most of it. If the town has anything to put on that page, I need it in writing."
- **Talk** (`talk-arc-plant-…`). *need:* Ask what { $parent } would need to see · you say "What would change their mind?" · yes "Off the record? We're { $gap } a year short. A year off the rates covers a third. Short time covers the rest, if { $steward } will wear it." · no "I can't go through the numbers with you. Ask me again when it's public." *table:* Ask { $steward } to sit down with { $manager } · yes "One room, one afternoon. Tell { $manager } to bring the real numbers." · no "We're past rooms. Ask me again when { $manager } has something to offer." *short:* Ask { $steward } about short time · yes "I'll put four days a week to them. I won't put a pay cut to them. Get me { $manager }'s word that it's one and not both." · no "You weren't at the gate. I don't carry messages for people who weren't at the gate." *more:* Ask { $manager } for more time · yes "With that in writing I can ask for two months. I can ask once." · no "More time for what? Bring me something to show them and I'll make the call."
- **Visit.** Stand at the gate at six · "You stood at the gate of { $plant } while the shifts changed. { $steward } saw you there."
- **Decisions.** `dilemma-plant-layoffs-situation`, rewritten: "{ $parent } has put { $jobs } jobs at { $plant } under review, and { $place } is choosing sides. { $manager } ({ $manager_role }) wants nothing done that adds to the plant's costs. { $steward } ({ $steward_role }) wants you at the gate." `dilemma-plant-terms`, **what the town puts in:** "{ $parent } decides on { $day }, and { $manager } ({ $manager_role }) says the plant is { $gap } a year short. A year off the rates would cover a third of that, and cost the town the same. { $steward } ({ $steward_role }) wants any money tied to the jobs, in writing." Options: "Draft a year off the rates, tied to every job kept" · "Back the rates cut only if short time comes with it" · "Offer nothing: the town can't buy jobs it can't afford". After *rates:* "You drafted a year off the rates for { $plant }, tied to every job kept. { $manager } has something to show { $parent }, and { $steward } has it in writing." `dilemma-plant-eve`, **the night before:** "{ $parent } rings { $manager } in the morning. { $steward } ({ $steward_role }) has the members at the hall tonight and wants you in the room. { $manager } ({ $manager_role }) would like one more thing to say on the call." Options: "Go to the hall, and promise a retraining fund if it closes" · "Give { $manager } the town's offer as a letter, with your name on it" · "Stay home: it is { $parent }'s decision now".
- **Moves and press.** `skip-item-arc-plant-ballot`: "{ $steward } balloted the members at { $plant }. They voted to walk out." `skip-item-arc-plant-call`: "{ $manager } told { $parent } about the walkout. The decision moved up to { $day }." Headlines: "{ $parent } to decide future of { $plant } by { $day }" · "Walkout stops the line at { $plant }" · "Closure at { $plant }: { $jobs } jobs go in { $place }". Legacy: "Kept { $plant } open on short time ({ $kept } jobs)."

**Passive.** Nobody asks Rafe Garrowby anything in *signs*. The notice comes; the decision lapses to staying out. Heat climbs six points a week, Sofi Brook ballots in week three, and the line stops (the plant's sector loses a tenth of a month's output). Garrowby reports the walkout and the date moves up a fortnight. Nothing narrows the gap, so the economy closes the plant: 140 jobs, the gate boarded, three points of approval gone in Quarryrow, and Brook remembers being ignored. The buyer arc may follow in six to eighteen months.
**Engaged.** In *signs* the player hears about the overtime at the diner and asks Garrowby what head office needs; he trusts them enough to give the gap, weeks before anyone else knows. At the notice they stand at the gate, then get both into one room, so the arc goes to *talks* and skips the walkout. Brook, who saw them at the gate, agrees to put short time to the members; with that and a drafted rates bill the economy may find the gap inside its partial band: *kept*, 110 of 140 jobs, a record entry, a manager who owes a favour. If the player asks for more time first, it ends in *reprieve* and comes back in two months with the same people. In a town with a deeper slump the same play still ends *closed*, with Brook's goodwill intact.

### 7.2 `arc.flood`: after the water

Needs a forecast lead on hazards and the `recovery` request (task A5b). Without them the arc starts at *struck*, which is how it is built first (§9).

**(A1) As built** (`content/base/arcs/flood.jsonc`; the JSONC below is the design's and is not what ships until A5b and A6):
- From *struck*: no *warning* stage and no *passed* exit. `tiers` is `["local"]`: `disaster` and `law_in_force` still read a town.
- The numbers follow the simulation's rates. Damaged homes are repaired at 8% a month, so a year repairs about 63%: the rebuilding edges read `recovered` at 0.72 by the clock and 0.85 before it (0.7 with a wall voted through), and *counting* turns to *rebuilding* when someone has paid or sent a crew and a fifth is repaired. A strike sends about 1% of a neighbourhood away, so the story reads people away as a count, and `given` asks for one in a thousand.
- The money decision's lever is `recovery` (repairs twice as fast for eight months, or half as much again for six): the town's own money (`treasury`) waits for A5b, and until then the cost is the player's Standing.
- The ask is for "a second roof", not the school gym: a town need not have a school. A talk beat was added to *counting* (ask the official for a repair crew), which had nothing a player could do.
- Exits move the player's approval in the neighbourhood (`approval`), not the government's (`gov_approval`), and board nothing (`place_state`). *Hollowed* sends those still away from the town for good through the population's own `Displacement`.
- The decisions' text names the organiser and the neighbourhood only: a decision's cast carries people and neighbourhoods, so `$hall`, `$official`, `$above` and the numbers reach the stages, the headlines and the visits, not the dilemmas, until the arc view (A3).

```jsonc
{ "id": "arc.flood", "title_key": "arc-flood", "themes": ["crisis"], "tiers": ["municipality", "region"], "cooldown_days": 0,
  "raise": [ { "type": "any", "of": [ { "type": "hazard_forecast", "hazard": "flood", "min": 0.5 }, { "type": "disaster", "hazard": "flood", "within_months": 1 } ] } ],
  "roles": [ { "id": "hood", "cast": { "type": "area", "pick": "most_exposed", "hazard": "flood" } }, { "id": "organiser", "cast": { "type": "occupation", "occupations": ["occupation.organiser", "occupation.clergy"], "near": "hood" } },
    { "id": "hall", "cast": { "type": "place", "kinds": ["community_centre"], "near": "hood" } }, { "id": "official", "cast": { "type": "post", "power": "propose_budget" }, "optional": true }, { "id": "above", "cast": { "type": "jurisdiction", "pick": "parent" } } ],
  "sides": { "ally": "organiser", "opponent": "official" }, "heat": { "start": 0.3, "floor": { "indicator": "recent_disaster", "scale": 0.5 } },
  "answers": ["policy.rebuild_fund", "policy.flood_defenses", "policy.hardship_fund"], "frame": { "post": { "seat_for": "hood" }, "pitch_key": "pitch-flood" },
  "stages": [
    { "id": "warning", "key": "arc-flood-warning", "clock": { "from": "subject" }, "rise": 0.1, "beats": [ { "id": "cars", "type": "heard", "line": "overheard.flood.cars", "teaches": "known" },
        { "id": "open", "type": "talk", "with": "official", "choice": { "label_key": "talk-arc-flood-open", "say_key": "talk-arc-flood-open-say", "replies": [ { "key": "talk-arc-flood-open-yes", "when": [ { "type": "opinion", "slot": "speaker", "min": 0 } ], "agrees": true, "sets": "hall_open", "effects": [ { "type": "recovery", "area": "hood", "prep": 0.1 } ] }, { "key": "talk-arc-flood-open-no" } ] } },
        { "id": "sandbags", "type": "visit", "at": "hood", "verb_key": "visit-arc-flood-sandbags", "outcome_key": "outcome-arc-flood-sandbags", "sets": "sandbagged", "effects": [ { "type": "slots", "n": 1 }, { "type": "recovery", "area": "hood", "prep": 0.15 }, { "type": "opinion", "role": "organiser", "delta": 8 } ] } ],
      "next": [ { "to": "struck", "when": [ { "type": "disaster", "hazard": "flood", "within_months": 1 } ] }, { "to": "passed", "clock_within_days": 0 } ] },
    { "id": "struck", "key": "arc-flood-struck", "clock": { "days": [14, 21] }, "beats": [ { "id": "press", "type": "press", "headline_key": "headline-arc-flood-struck" },
        { "id": "box", "type": "visit", "at": "hall", "verb_key": "visit-arc-flood-box", "outcome_key": "outcome-arc-flood-box", "sets": "at_hall", "relief": 0.05, "effects": [ { "type": "opinion", "role": "organiser", "delta": 8, "memory": "stood_with_me" } ] },
        { "id": "gym", "type": "ask", "template": "ask.flood_gym", "asker": "organiser" }, { "id": "money", "type": "decision", "dilemma": "dilemma.flood_money", "weight": "significant", "marks": { "now": "paid_first", "worst": "paid_first" } } ],
      "next": [ { "to": "counting", "clock_within_days": 0 } ] },
    { "id": "counting", "key": "arc-flood-counting", "rise": 0.05, "beats": [ { "id": "petition", "type": "move", "actor": "organiser", "weight": 0.5, "when": [ { "type": "arc_days", "min": 28 }, { "type": "displaced", "slot": "hood", "min": 0.1 } ], "line_key": "skip-item-arc-flood-petition" },
        { "id": "weeks", "type": "press", "headline_key": "headline-arc-flood-weeks", "when": [ { "type": "arc_days", "min": 42 } ] } ],
      "next": [ { "to": "rebuilding", "when": [ { "type": "law_in_force", "templates": ["policy.rebuild_fund", "policy.flood_defenses"] } ] }, { "to": "rebuilding", "when": [ { "type": "arc_mark", "mark": "paid_first" }, { "type": "displaced", "slot": "hood", "max": 0.05 } ] },
                { "to": "meeting", "when": [ { "type": "arc_heat", "min": 0.7 } ] }, { "to": "meeting", "after_days": 56 } ] },
    { "id": "meeting", "key": "arc-flood-meeting", "climax": true, "clock": { "days": [7, 14] }, "beats": [ { "id": "room", "type": "decision", "dilemma": "dilemma.flood_meeting", "weight": "climax", "marks": { "wall": "wall" } } ],
      "next": [ { "to": "rebuilding", "when": [ { "type": "law_in_force", "templates": ["policy.rebuild_fund", "policy.flood_defenses"] } ] }, { "to": "hollowed", "after_days": 120 } ] },
    { "id": "rebuilding", "key": "arc-flood-rebuilding", "clock": { "days": [240, 360] }, "beats": [ { "id": "works", "type": "visit", "at": "hood", "verb_key": "visit-arc-flood-works", "outcome_key": "outcome-arc-flood-works" },
        { "id": "wall", "type": "press", "headline_key": "headline-arc-flood-wall", "when": [ { "type": "law_in_force", "templates": ["policy.flood_defenses"] } ] } ],
      "next": [ { "to": "defended", "when": [ { "type": "law_in_force", "templates": ["policy.flood_defenses"] }, { "type": "recovered", "slot": "hood", "min": 0.9 } ] }, { "to": "rebuilt", "when": [ { "type": "recovered", "slot": "hood", "min": 0.95 } ] },
                { "to": "rebuilt", "when": [ { "type": "recovered", "slot": "hood", "min": 0.8 } ], "clock_within_days": 0 }, { "to": "hollowed", "clock_within_days": 0 } ] },
    { "id": "defended", "key": "arc-flood-defended", "exit": { "legacy_key": "legacy-arc-flood-defended", "effects": [ { "type": "place_state", "place": "hall", "state": "marked" }, { "type": "gov_approval", "area": "hood", "delta": 0.03 }, { "when": [ { "type": "arc_mark", "mark": "wall" } ], "type": "record", "delta": 0.08 } ] } },
    { "id": "rebuilt", "key": "arc-flood-rebuilt", "exit": { "legacy_key": "legacy-arc-flood-rebuilt", "effects": [ { "type": "place_state", "place": "hall", "state": "marked" }, { "when": [ { "type": "arc_mark", "mark": "at_hall" } ], "type": "approval", "area": "hood", "delta": 0.03 } ] } },
    { "id": "hollowed", "key": "arc-flood-hollowed", "exit": { "headline_key": "headline-arc-flood-hollowed", "effects": [ { "type": "displacement", "area": "hood", "of": "still_away" }, { "type": "place_state", "place": "hood", "state": "boarded" }, { "type": "gov_approval", "area": "hood", "delta": -0.05 },
        { "unless": [ { "type": "arc_mark", "mark": "at_hall" } ], "type": "opinion", "role": "organiser", "delta": -12, "memory": "ignored_me" } ] } },
    { "id": "passed", "key": "arc-flood-passed", "exit": { "effects": [ { "when": [ { "type": "arc_mark", "mark": "hall_open" } ], "type": "opinion", "role": "official", "delta": -4 }, { "when": [ { "type": "arc_mark", "mark": "sandbagged" } ], "type": "awareness", "area": "hood", "delta": 0.03 } ] } } ] }
```

`dilemma.flood_money`: *now* posts `treasury` for the full cost and `recovery` (repairs at 1.5× for six months); *worst* posts half of each; *wait* is the default. `dilemma.flood_meeting`: *date* makes a tracked promise (home within 60 days); *wall* drafts flood defences; *honest* lowers heat; *absent* is the default and raises it by 0.2.

- **Story so far** (`arc-flood-…`). *warning:* "The river at { $place } is forecast to top its banks by { $day }." *struck:* "The river came through { $hood } on { $day }. { $homes } homes are damaged, and { $away } people are sleeping somewhere else." *counting:* "The water has gone from { $hood }. { $away } people are still not home, and nobody has said who pays." *meeting:* "{ $weeks } weeks on, { $away } people from { $hood } are still not home. { $organiser } has called a meeting at { $hall }." *rebuilding:* "Work has started in { $hood }. { $repaired } of { $homes } homes are fit to live in."
- **Exits.** *defended:* "{ $hood } is rebuilt behind a wall. { $back } of the people who left are home." *rebuilt:* "{ $hood } is rebuilt as it was, at the same height above the river." *hollowed:* "{ $gone } people who left { $hood } have not come back. Their homes are boarded." *passed:* "The river peaked below its banks on { $day }. Nothing in { $hood } was lost."
- **Pitch.** "{ $place }. The river came through { $hood } on { $day }, and { $away } people are not home. You hold the seat for { $hood }."
- **Overheard** (`heard-flood-…`). *cars:* "They've moved the cars up to the top road already. Nobody told them to." *skips:* "There's a skip outside every door in { $hood }. Carpets, mostly. You forget how much of a house is carpet."
- **Talk** (`talk-arc-flood-open`). Ask { $official } to open { $hall } tonight · you say "Open the hall tonight. If the river holds, we've lost an evening." · yes "Fine. I'll have a caretaker there by six. If it's a dry night, it was your idea." · no "Forecasts. I'm not paying a caretaker to sit in an empty hall on the strength of a forecast."
- **Visits.** Fill sandbags in { $hood } · "You filled sandbags in { $hood } until dark. { $organiser } put you at the low end of the street." Carry boxes at { $hall } · "You sorted donations at { $hall } for an afternoon. { $organiser } gave you the tinned food and no speech."
- **At the hall** (`scene-flood-hall-organiser`). "{ $away } people came through that door last night, and I've beds for half. The school has a gym. Get me the keys to it." **At the meeting** (`scene-flood-meeting-organiser`). "They've heard 'soon' from everyone who has stood where you're standing. Don't say soon. Say a month, or say you don't know."
- **Ask** (`ask-flood-gym`). **The keys to the school gym.** "{ $asker } ({ $asker_role }) is running the relief at { $hall }, and it is full. The school gym would take the rest, and { $official } holds the keys." Need: "Get { $official } to open the school gym". Kept: "The school gym opened the same night, and { $asker } stopped turning people away from { $hall }." Broken: "The gym stayed locked. { $asker } put the overflow in the corridor at { $hall }, and told them why."
- **Decisions.** `dilemma-flood-money`, **who pays first:** "{ $homes } homes in { $hood } are not fit to live in, and no repair starts until someone says who pays. The town could put up { $cost } now and argue with { $above } afterwards. { $official } ({ $official_role }) would sooner wait for { $above } to offer." Options: "Spend the town's money now, and send { $above } the bill" · "Pay for the worst-hit streets now, and hold the rest back" · "Wait for { $above }: nothing starts until they answer". `dilemma-flood-meeting`, **the meeting at the hall:** "{ $organiser } ({ $organiser_role }) has filled { $hall } with people from { $hood } who have been out of their homes for { $weeks } weeks. They want a date. They also want to know whether the low streets are being rebuilt or written off." Options: "Give them a date, and put your name to it" · "Tell them it will flood again: argue for the wall, however long it takes" · "Say the money isn't there yet, and stay for every question" · "Send apologies". After *honest:* "You told the meeting at { $hall } the money was not there yet, and stayed until the last question. { $organiser } thanked you for not saying soon."
- **Moves and press.** `skip-item-arc-flood-petition`: "{ $organiser } took a petition round { $hood } asking when the repairs start." Headlines: "River floods { $hood }: { $homes } homes damaged in { $place }" · "{ $weeks } weeks on, { $away } still out of their homes in { $hood }" · "Work starts on flood wall for { $hood }" · "{ $gone } have left { $hood } for good since the flood".

**Passive.** Nobody asks for the hall to be opened; the river takes Quarryrow at full damage. The money decision lapses to waiting for the region. Halle Talliswell's petition goes round in week four, the *Ledger* counts the weeks, heat passes 0.7, and the meeting is held with apologies sent. If other members pass a rebuild fund the arc still ends *rebuilt*, with the player in nobody's memory of it. If they do not, 120 days later the people still away become people who left: *hollowed*, boarded houses in Quarryrow, five points off the government there.
**Engaged.** The player fills sandbags and gets the hall opened (a quarter less damage in Quarryrow), carries boxes, gets the gym opened, and spends the town's money first, so repairs run half as fast again for six months. Fewer people are away, heat never reaches the meeting's line, and the arc goes from *counting* straight to *rebuilding*. Whether it ends *defended* or *rebuilt* is the player's flood-defences bill and the vote on it. A contrarian who backs the official and waits gets the meeting, and must choose in front of the room between a date they may not keep and a wall that takes two years.

### 7.3 `arc.party_row`: the question about a colleague

Political: nothing physical is at stake. It uses hidden facts, a scandal's heat, cases, colleagues' opinions of each other, removal and the party's wings, which all exist; it adds `group_opinion` and `faction_shift` (task A5c). The player's own opinion of the accused is counted with the group's, so their vote at the meeting is one vote. The `split` exit waits for DEC-107: it has no edge yet, and the doctor skips a stage whose `requires` is unmet.

```jsonc
{ "id": "arc.party_row", "title_key": "arc-row", "themes": ["scheme"], "tiers": ["municipality", "region", "nation"], "cooldown_days": 900,
  "raise": [ { "type": "hidden_fact", "slot": "accused", "surfaces_within_days": 120 } ],
  "roles": [ { "id": "accused", "cast": { "type": "post", "party": "players_or_governing", "with": "hidden_fact" } }, { "id": "chair", "cast": { "type": "occupation", "occupations": ["occupation.party_chair"], "party_of": "accused" } },
    { "id": "challenger", "cast": { "type": "post", "party_of": "accused", "faction": "other", "pick": "most_ambitious" } }, { "id": "reporter", "cast": { "type": "discoverer", "of": "accused" } }, { "id": "group", "cast": { "type": "party_group", "of": "accused" } } ],
  "sides": { "ally": "challenger", "opponent": "accused" }, "heat": { "start": 0.1, "floor": { "scandal": "accused" } }, "frame": { "post": { "seat_in_group": "group" }, "secret": "hidden_fact", "pitch_key": "pitch-row" },
  "stages": [
    { "id": "whispers", "key": "arc-row-whispers", "hidden": true, "beats": [ { "id": "asking", "type": "heard", "line": "overheard.row.asking", "teaches": "heard" },
        { "id": "working", "type": "talk", "with": "reporter", "choice": { "label_key": "talk-arc-row-working", "say_key": "talk-arc-row-working-say", "replies": [ { "key": "talk-arc-row-working-yes", "when": [ { "type": "any", "of": [ { "type": "trust", "slot": "speaker", "min": 0.5 }, { "type": "favour", "slot": "speaker", "owed_to_player": true } ] } ], "agrees": true, "sets": "asked", "teaches": "detailed" }, { "key": "talk-arc-row-working-no", "sets": "asked", "teaches": "known" } ] } },
        { "id": "warn", "type": "talk", "with": "accused", "when": [ { "type": "arc_mark", "mark": "asked" } ], "choice": { "label_key": "talk-arc-row-warn", "say_key": "talk-arc-row-warn-say", "replies": [ { "key": "talk-arc-row-warn-reply", "agrees": true, "sets": "warned", "effects": [ { "type": "opinion", "role": "accused", "delta": 12, "memory": "stood_with_me" } ] } ] } },
        { "id": "go_quietly", "type": "move", "actor": "accused", "weight": 0.5, "when": [ { "type": "arc_mark", "mark": "warned" }, { "type": "personality", "slot": "accused", "axis": "boldness", "max": 0.4 } ], "line_key": "skip-item-arc-row-quiet", "effects": [ { "type": "removal", "role": "accused" } ] } ],
      "next": [ { "to": "quiet", "when": [ { "type": "holds_post", "slot": "accused", "is": false } ] }, { "to": "story", "when": [ { "type": "scandal", "slot": "accused" } ] } ] },
    { "id": "story", "key": "arc-row-story", "clock": { "days": [10, 21] }, "enter": [ { "when": [ { "type": "arc_mark", "mark": "warned" } ], "type": "valence", "role": "accused", "delta": -0.15 } ], "beats": [ { "id": "press", "type": "press", "headline_key": "headline-arc-row-story" },
        { "id": "line", "type": "decision", "dilemma": "dilemma.row_line", "weight": "significant", "marks": { "stand_by": "stood_by", "go": "called" } }, { "id": "hold", "type": "ask", "template": "ask.row_hold", "asker": "chair" },
        { "id": "letter", "type": "move", "actor": "challenger", "weight": 0.8, "when": [ { "type": "arc_days", "min": 3 } ], "sets": "letter", "line_key": "skip-item-arc-row-letter", "effects": [ { "type": "opinion", "holders": { "group": "group", "faction": "other" }, "toward": "accused", "delta": -10 } ] } ],
      "next": [ { "to": "charged", "when": [ { "type": "case", "slot": "accused", "stage": "charged" } ] }, { "to": "sides", "when": [ { "type": "arc_mark", "mark": "letter" } ] }, { "to": "meeting", "clock_within_days": 3 } ] },
    { "id": "sides", "key": "arc-row-sides", "rise": 0.04, "beats": [
        { "id": "sign", "type": "talk", "with": { "each": "group" }, "once": false, "choice": { "label_key": "talk-arc-row-sign", "say_key": "talk-arc-row-sign-say", "replies": [ { "key": "talk-arc-row-sign-yes", "when": [ { "type": "any", "of": [ { "type": "favour", "slot": "speaker", "owed_to_player": true }, { "type": "opinion", "slot": "speaker", "min": 20 } ] } ], "agrees": true, "sets": "signed", "effects": [ { "type": "opinion", "holder": "speaker", "toward": "accused", "delta": -15 } ] }, { "key": "talk-arc-row-sign-no" } ] } },
        { "id": "wait", "type": "talk", "with": { "each": "group" }, "once": false, "choice": { "label_key": "talk-arc-row-wait", "say_key": "talk-arc-row-wait-say", "replies": [ { "key": "talk-arc-row-wait-yes", "when": [ { "type": "opinion", "slot": "speaker", "min": 10 } ], "agrees": true, "sets": "held", "effects": [ { "type": "opinion", "holder": "speaker", "toward": "accused", "delta": 12 } ] }, { "key": "talk-arc-row-wait-no" } ] } },
        { "id": "count", "type": "press", "headline_key": "headline-arc-row-letter", "when": [ { "type": "group_opinion", "group": "group", "of": "accused", "max": -10, "share_min": 0.3 } ] },
        { "id": "resign", "type": "move", "actor": "accused", "weight": 0.6, "when": [ { "type": "group_opinion", "group": "group", "of": "accused", "max": -10, "share_min": 0.5 }, { "type": "arc_heat", "min": 0.5 } ], "line_key": "skip-item-arc-row-resign", "effects": [ { "type": "removal", "role": "accused" } ] } ],
      "next": [ { "to": "went", "when": [ { "type": "holds_post", "slot": "accused", "is": false } ] }, { "to": "charged", "when": [ { "type": "case", "slot": "accused", "stage": "charged" } ] }, { "to": "meeting", "clock_within_days": 3 } ] },
    { "id": "meeting", "key": "arc-row-meeting", "climax": true, "clock": { "days": [3, 5] }, "beats": [ { "id": "vote", "type": "decision", "dilemma": "dilemma.row_vote", "weight": "climax" },
        { "id": "resign", "type": "move", "actor": "accused", "weight": 1.0, "when": [ { "type": "group_opinion", "group": "group", "of": "accused", "max": -10, "share_min": 0.5 } ], "line_key": "skip-item-arc-row-resign", "effects": [ { "type": "removal", "role": "accused" } ] } ],
      "next": [ { "to": "went", "when": [ { "type": "holds_post", "slot": "accused", "is": false } ] }, { "to": "stayed", "clock_within_days": 0 } ] },
    { "id": "went", "key": "arc-row-went", "exit": { "headline_key": "headline-arc-row-went", "legacy_key": "legacy-arc-row-went", "effects": [ { "type": "faction_shift", "faction_of": "accused", "delta": -0.08 }, { "when": [ { "type": "arc_mark", "mark": "signed" } ], "type": "favour", "role": "challenger", "owed_to_player": true }, { "when": [ { "type": "arc_mark", "mark": "stood_by" } ], "type": "standing", "delta": -3 } ] } },
    { "id": "stayed", "key": "arc-row-stayed", "exit": { "headline_key": "headline-arc-row-stayed", "effects": [ { "type": "faction_shift", "faction_of": "accused", "delta": 0.05 }, { "when": [ { "type": "arc_mark", "mark": "signed" } ], "type": "opinion", "role": "chair", "delta": -10, "memory": "broke_ranks" }, { "when": [ { "type": "arc_mark", "mark": "held" } ], "type": "favour", "role": "accused", "owed_to_player": true } ] } },
    { "id": "charged", "key": "arc-row-charged", "exit": { "effects": [ { "type": "faction_shift", "faction_of": "accused", "delta": -0.12 }, { "when": [ { "type": "arc_mark", "mark": "stood_by" } ], "type": "approval", "area": "at", "delta": -0.02 }, { "when": [ { "type": "arc_mark", "mark": "called" } ], "type": "record", "delta": 0.03 } ] } },
    { "id": "quiet", "key": "arc-row-quiet", "exit": { "effects": [ { "type": "faction_shift", "faction_of": "accused", "delta": -0.03 }, { "type": "favour", "role": "accused", "owed_to_player": true } ] } },
    { "id": "split", "key": "arc-row-split", "requires": ["parties.membership"], "exit": { "effects": [ { "type": "membership", "role": "challenger", "act": "leave_with_faction" } ] } } ] }
```

- **Story so far** (`arc-row-…`). *whispers:* "{ $reporter } is asking people about { $accused }. Nothing has been printed." *story:* "The { $outlet } has run a story on { $accused } over { $fact }. The branch meets on { $day }." *sides:* "{ $challenger } has a letter asking { $accused } to go. { $signed } of { $group_n } have signed." *meeting:* "The branch votes on { $day } on whether { $accused } keeps the party's backing."
- **Exits.** *went:* "{ $accused } resigned as { $accused_role } on { $day }. The party named { $successor } in their place." *stayed:* "The branch backed { $accused }, { $for } to { $against }. { $challenger } withdrew the letter." *charged:* "Prosecutors charged { $accused } over { $fact }. The branch never voted." *quiet:* "{ $accused } stood down as { $accused_role } before anything was printed."
- **Pitch.** "{ $place }. { $reporter } of the { $outlet } is asking about { $accused }, who sits in your group. Nothing has been printed yet."
- **Overheard** (`heard-row-asking`). "That reporter from the { $outlet } was in again, asking who { $accused } has lunch with. I said I only see who pays."
- **Talk** (`talk-arc-row-…`). *working:* Ask what { $reporter } is working on · you say "You've been asking about { $accused }." · yes "I have it from one person who'll put their name to it. I need two. On the record or off?" · no "I ask about everybody. This week it's { $accused }. Next week it might be you." *warn:* Warn { $accused } · "Then they've rung the wrong people. Who told you? No. Don't tell me. Thank you." *sign:* Ask { $speaker } to sign the letter · yes "I'd have signed on Monday if anyone had asked. Give it here." · no "Not on a newspaper's say-so. Bring me something that isn't a cutting." *wait:* Ask { $speaker } to wait for the meeting · yes "Till the meeting, then. Not a day after it." · no "I've waited. So have the people who voted for me, and they read the paper."
- **Ask** (`ask-row-hold`). **Nothing before the meeting.** "{ $asker } ({ $asker_role }) is trying to keep the branch in one piece until it meets on { $day }. They want your word that you will sign nothing and brief nobody before then." Need: "Sign nothing and say nothing until { $day }". Kept: "You said nothing before the meeting. { $asker } told the branch so from the chair." Broken: "You gave { $asker } your word and then signed the letter. { $asker } read your name out with the others."
- **Decisions.** `dilemma-row-line`, **a line for the paper:** "The { $outlet } has run a story on { $accused } ({ $accused_role }) over { $fact }. { $reporter } wants a line from you by six. { $chair } ({ $chair_role }) wants nobody to say anything before the branch meets on { $day }." Options: "Stand by { $accused } in print" · "Say { $accused } should consider their position" · "Say nothing: { $reporter } will print that you were asked". After *stand by:* "You stood by { $accused } in print, against { $chair }'s advice. { $accused } thanked you. { $chair } did not." `dilemma-row-vote`, **the vote at the branch:** "The branch votes tonight on whether { $accused } keeps the party's backing. { $signed } of { $group_n } have signed { $challenger }'s letter, and it carries with { $needed }. Your vote is counted with the rest, and read out." Options: "Vote to withdraw the party's backing" · "Vote to keep { $accused }" · "Abstain, and let the count fall where it falls".
- **Moves and press.** `skip-item-arc-row-letter`: "{ $challenger } began collecting names on a letter asking { $accused } to go." `skip-item-arc-row-resign`: "{ $accused } resigned as { $accused_role } after losing the branch." Headlines: "{ $accused } faces questions over { $fact }" · "{ $signed } party colleagues call on { $accused } to go" · "{ $accused } resigns as { $accused_role }" · "Party branch backs { $accused }, { $for }–{ $against }".

**Passive.** The player learns of it from the front page. They give no line, and the chair's ask goes unanswered. The challenger's letter moves the other wing ten points against Celia Janssenfield. The count is whatever the colleagues' opinions make it. If she goes, her wing shrinks eight points and the party's line shifts on the next bills; if she stays, it grows five. Either way nobody owes the player anything.
**Engaged.** The player hears the line at the diner, gets the reporter to say what she has, and warns Janssenfield. A cautious Janssenfield stands down before the story (*quiet*: a small loss for her wing, a favour owed). A bold one fights, and the story lands softer because she was ready. The player then works the group one conversation at a time. Three signatures called in on favours tip the share past half and she resigns before the vote (*went*: the challenger in the player's debt, the line moving their way). Or three colleagues held get her through (*stayed*: a favour from her, and the chair cooler if the player broke their word to say nothing). A case, if the evidence is strong, overrides the room: *charged*, and whoever stood by her in print pays for it in the town.

## 8. A dozen more

Each is one arc template. "Now" means the simulation can raise it once task A1 is in; otherwise the missing system is named.

1. **The bridge.** Raised when a crossing's condition falls below safe and it is shut. The town is cut in two: visits and asks across the river cost more of a day, and shops on the far side shed trade (`sector_shock`). The lever is money from the tier above; a stalled works site is the worst exit. Uses works (DEC-099), finance, places. *Needs asset condition and upkeep (W5).*
2. **The water.** Raised when pollution is high in a town with an industrial plant upstream of a neighbourhood. A knowledge arc: someone had a report (a hidden fact with an auditor as discoverer), and the inspection that would prove it stops a line that pays 200 wages. Uses environment, plants, media, the ledger. *Needs an inspection effect that lowers pollution (small; the audit found none).*
3. **The beloved institution.** Raised when budget stress puts a library, pool or hall on the cuts list. The sum is tiny and the feeling is not; the place has regulars with routines, and a volunteer takeover is an exit that costs the player weekends. Uses finance, `place_state`, routines. *Now.*
4. **The developer.** An opportunity arc: an application for a reserved plot when vacancy is low. The housing worry pulls one way and the neighbours the other; the broker has a stake, and a consultancy fee is on offer (an existing hidden-fact kind). Uses housing (builders now answer vacancy), zoning laws, plan slots, the broker. *Now.*
5. **The strike.** Raised when prices outrun public pay for a year. For once the player sits on the employer's side; bins go uncollected where everyone can see. Uses the public sector, the budget, wages and prices. *Needs a service-level indicator (audit fix 3) and street dressing (S7).*
6. **The heatwave.** A hazard with a forecast and no rubble: a clock in days, and levers that are small and immediate (open the hall, get people checked on, which is casework by the street). The cost falls on the oldest cohorts and the clinic. *Needs a hazard kind and an effect on older cohorts.*
7. **The missing money.** Raised when an audit finds the treasury short. Nobody knows whether it is theft or an error until the case stage, and whoever accuses first can be wrong in print. Uses finance, hidden facts, justice, the press. *Now, once NPC fact kinds are in the registry (E9).*
8. **The newcomers.** Raised when arrivals outrun homes and clinic places after a new employer opens. The worry is the wait and the rent, never the people (style bible §6); the levers are homes and services, against a rival who would rather have a villain. Uses migration, housing, services. *The employer version now; resettlement needs immigration (M7).*
9. **The sinkhole.** Raised in a town with old workings: a street drops, with no signs stage at all. The arc is the liability trail to the plant's parent firm, and whether the street is moved. Uses hazards, housing damage, named plants, the tier above. *Needs a hazard kind keyed to extraction history (small).*
10. **The team.** An opportunity arc: the town's club will fold or move unless it gets a ground. Almost nothing material is at stake and awareness swings more than in any other arc; being seen on the wrong side of it costs for years. *Needs a club place kind with attendance (small).*
11. **The recount.** Raised when a race ends inside the recount margin. Five days in the tail of a campaign chapter; the levers are legal and public; the exits are confirmed, overturned and re-run. Uses elections, the press, justice. *Needs a count-error model and a recount rule in the constitution's election law.*
12. **The buyer.** The closure's mirror and its follow-up: a firm considers the boarded plant, and a real second town in the region is bidding too. What the player gives away (rates, land, an exemption) is a law with a cost. Uses economy capacity, plants, policy, the region. *Now, with task A5a.*

## 9. Implementation plan

Engine tasks bump the save once each. **The first playable arc is the flood from *struck*** (A1): disasters, displacement, repairs, the rebuild fund and flood defences all exist, and the lab can plant a flood (`Intervention::Disaster`).

| # | Task | Tier | Kind | OWNS | After | Acceptance |
|---|---|---|---|---|---|---|
| A1 | Arc core, and the flood arc from *struck* | large | engine, one arc | `mandate-content/src/arcs.rs` and `condition.rs`; `mandate-sim/src/arcs/**`, `condition.rs`, `spine/mod.rs` (`Thread::Arc`), `state.rs`, `systems.rs`, `scheduler.rs`, `talk/hooks.rs` (register); `mandate-save/**`; `content/base/arcs/flood.jsonc`, `locale/en/arcs.ftl`; schemas | S1 merged | A planted flood raises the arc in 8 of 8 seeds; scripted passive and engaged command lists reach different exits; one hash twice and through save and load; a fork equals continuing; with no arc content every pinned digest of the spine proof is unchanged |
| A2 ✓ | The director on arcs; storylines folded; standing arcs | large | engine, content | `narrative/director/**`, `narrative/mod.rs`, `director.jsonc`, `storylines.jsonc` → `arcs/rival.jsonc`, `favour_debt.jsonc`, `budget_round.jsonc`; `mandate-worldgen/tests/director.rs`; the save migration | A1 | A climax in ≥ 95% of chapters over the pacing test's seeds with `climax_fallbacks` 0; spotlight on against off leaves every endogenous outcome identical; an old save with a running storyline loads at the matching stage |
| A3 ∥ | `ArcView`, the story sheet, "what you can do", Today items | mid | protocol, web | `mandate-protocol/src/arcs.rs`, `web/src/components/StorySheet.tsx`, a new `ArcSheet.tsx`, `web/e2e/arcs.spec.ts` | A1 | Every beat kind taken from quick mode in one e2e; below `known` the view shows a lead only |
| A4 ∥ | Stand-ins, the scorer, the grader, the doctor's arc lints | mid | engine, lab | `mandate-sim/src/standin.rs`, `proxy.rs`, `arcs/score.rs`; `mandate-cli/src/lab/**` (`Driver::StandIn`), `arcs_grade.rs`, `doctor/arcs.rs` | A1 | `arcs grade --arc arc.flood` prints the five terms; each static lint has a failing fixture; stand-ins send only commands |
| A5a | The plant's fuse (review, announcement, decision); `plant_review`, `place_state` | mid | engine | `economy/**`, `places.rs`, `events.rs`, `economy.jsonc` | note 17's registry | The `audit stuck` closure count within 10% of today's 964; every closure is announced first; the audit's closure rows re-run |
| A5b ∥ | Forecast lead; `recovery`, `treasury`, `gov_approval`; `displaced`, `recovered` | mid | engine | `environment/**`, `finance/mod.rs`, `population/mod.rs` (effects only) | A1 | A forecast precedes every flood by its content lead; prep lowers damage in a twin; outcomes with no arc unchanged |
| A5c ∥ | `group_opinion`, `faction_shift`, `hidden_fact`, `case`, `holds_post`; one NPC fact kind seeded | mid | engine | `politics/parties.rs`, `politics/mod.rs` (effects), `media.rs`, `justice.rs` (conditions only) | A1 | A shifted wing moves the platform and a party line in a twin; an NPC fact surfaces in 8 seeds |
| A6 | The three arcs complete; then today's crises rewritten and the dozen (W3.2) | mid | content | `content/base/arcs/**`, `dilemmas.jsonc`, `asks/asks.jsonc`, `overheard.jsonc`, `locale/en/arcs.ftl` | A4, and the A5 each needs | The grader's gates per arc; the content pipeline (a reviewer, the blind sample) |
| A7 | The audition | mid | engine, web | `mandate-sim/src/arcs/audition.rs`, `mandate-protocol` (`DraftView`), `web/src/screens/Title.tsx` | A4, three arcs | Same seed and parameters give the same draft, native and WASM; the measured cost table; nothing from a fork in any view |
| A8 | Frames, the draft screen, the first-beat guarantee (W3.3) | mid | engine, web | as W3.3 | A7 | A beat on turn 1 in every drafted opening |
| A9 ∥ | Style bible: the `arc-*`, `pitch-*` and `outcome-arc-*` registers; lint prefixes | small | docs, lint | `STYLE_BIBLE.md`, `mandate-cli/src/doctor/text.rs` | any time | The three arcs' text lints clean |

A3, A4 and one A5 can run together after A1 (disjoint files). A2 is serial with A1: both touch the save and `narrative`. Content (A6) starts on the flood as soon as A4's grader runs.

## 10. Open questions for the owner

1. **Do arcs run outside the towns the player is watching?** Recommend no. Causes run everywhere and their owners resolve them; arc runs cost time and nobody sees them. Returning to a world (DEC-103) reads the spine.
2. **The plant's fuse changes the simulation:** every closure, everywhere, gets a review of two to five months, and in a played town levers can sometimes avert it. Recommend yes, with the audit's closure rows re-run and the change logged in the balance log.
3. **A yearly budget round as the standing arc.** It guarantees a climax in a calm year at the price of a familiar frame. Recommend yes, with four variants and climax memory by variant, and delete the climax fallback.
4. **Variety against recent runs** makes the draft depend on the device's history. Recommend recording the last three openings as new-game parameters in the run file; shared seeds pass none.
5. **Eight seconds at world start** on a phone for the audition, under the history animation. Recommend accepting it for small worlds and taking fewer candidates in larger ones, never trimming by measured time.
6. **The party row ships without its split.** A wing walking out needs DEC-107's membership rules. Recommend shipping four exits in M6 and adding `split` with DEC-107, with nothing faked in between.

## 11. DEC proposal: staged arcs

*(A draft in the style of `docs/design/decisions/`; a number is assigned on acceptance.)*

**DEC-NNN — Staged arcs: one structure for crises, storylines and openings**
- **Status:** Proposed (2026-11-24). The owner asked of DEC-101: "Do you think these crises will actually be exciting? Or will they just come up as a quick decision vote?"
- **Context:**
  - A crisis is an indicator past a threshold and one dilemma. A plant closes in a single monthly draw, with no warning and nothing to do about it.
  - Storylines are a second staged structure with three templates, and the narrative domain reads only the player's town.
  - The pacing test finds a climax in 85% of chapters by a margin of one chapter, with a fallback that reuses cooling templates.
  - DEC-101 calls for arcs and an audition; IMPLEMENTATION_PLAN names staged arcs as shared mechanism S3.
- **Decision:**
  1. **An arc** is one matter in one jurisdiction: a cast bound to real people and places, stages, a clock, a heat, and at least three exits. Design: `docs/design/STAGED_ARCS.md`.
  2. **Transitions read; beats write.** Stages change only on conditions over simulated state, the arc's marks, heat and time. A player's choice posts effect requests and sets marks; it never names the next stage or the exit.
  3. **Seven beat kinds** on existing systems: decision, ask, talk (talk hooks), visit, press, heard, and an NPC's move.
  4. **Exits have simulated consequences** applied by the owning systems, and never repeat what the simulation already did.
  5. **Causes get fuses.** Where a cause is instantaneous today (a closure, a hazard's strike), its owning system gains a simulated lead: a review, a forecast. An arc reads it and pushes on it only through requests the owner prices.
  6. **Storylines fold into arcs; asks, dilemmas and the crisis registry stay.** Opening frames are a block on the arc.
  7. **The director times arcs and nothing else** (DEC-033): the lead arc, director-owned clocks, the arrival of opportunity arcs, hazard holds and discovery dates, and when a beat is presented. A chapter's climax is the lead arc's turn. The budget round and the rival are standing arcs, so every chapter has a turn; the climax fallback is removed.
  8. **Arcs run in focus jurisdictions**, keyed by `Jurisdiction`, with roles bound through occupations, powers and parties; every beat is reachable in quick mode and through Places.
  9. **The audition** forks each candidate opening for 39 weeks under passive, engaged and contrarian stand-ins (the skip proxy with directives), scores agency, escalation, visibility, pacing and variety, and offers the best three. The same scorer gates arc content in CI.
- **Order:** the arc core with the flood arc; the director and the storyline fold; protocol and web beside the stand-ins and grader; the simulated fuses; the three arcs and the rewrite; the audition; frames and the draft.
- **Dependencies:** DEC-006, DEC-030, DEC-032, DEC-033, DEC-041, DEC-051, DEC-054, DEC-091, DEC-097, DEC-098, DEC-099, DEC-101, DEC-102, DEC-107.

## 12. What task A1 built, and every correction it made to this note

Built (save v27): the arcs domain (`mandate-sim/src/arcs/`: `mod.rs` the state, `cast.rs`, `step.rs`, `beats.rs`, `effects.rs`), the content types and their validation (`mandate-content/src/arcs.rs`), the arc conditions, `Thread::Arc` and the arc events, and the flood arc from *struck* (`content/base/arcs/flood.jsonc`, `locale/en/arcs.ftl`). All seven beat kinds work end to end. Tests: `mandate-worldgen/tests/arcs.rs`, `mandate-save/tests/arcs.rs`, `mandate-cli/src/lab/arcs_games.rs`, and the proof's `with_no_arc_content_every_pin_holds`.

Corrections, each marked **(A1)** where it applies:

| # | The note said | As built, and why |
|---|---|---|
| 1 | A new stream `Stream::Arcs` (§4.4) | Keyed draws on `Stream::Misc` with the arcs' tags. A named stream's generator is part of the saved state: a new one changes every game's hash |
| 2 | Stepped weekly before the director (§4.2) | First in the day, so each request is applied the same day; a request for a monthly applier is held in `ArcsState::deferred` until the month turns |
| 3 | `ArcRun` about 200 bytes, a stage index, mark bits (§4.2, §4.4) | Stage and marks by id; `open`, `base`, `began`; about 1 KB. Robust to a content update |
| 4 | A decision beat's `marks: { option: mark }` (§4.1) | `on: { option: { sets, heat, effects } }`: an option may post the arc's own requests |
| 5 | `ArcEffect` is `EffectDef` plus six new requests (§4.3) | Its own list of eight; `recovery` built now (from A5b); `gov_approval` and `place_state` not yet; `OpenDecision` and `OfferAsk` added |
| 6 | `indicator`, `crisis`, `disaster`, `law_in_force` promoted to tier-generic (§4.1) | Not done: they still read a town, so the flood is `tiers: ["local"]`. `disaster` gains an optional `hazard` |
| 7 | `displaced`, `recovered` come with A5b (§9) | Built now, as readings of an arc's area role (`recovered` against the damage as the run was raised). `hazard_forecast`, `plant_review`, `hidden_fact`, `case`, `holds_post`, `group_opinion` are not built |
| 8 | Arguments come from the cast or "a condition on that path" (§4.1) | Explicit: an arc lists `facts`; each stage's event records them as they were |
| 9 | Each beat is `Prompted` on the spine (§1.8) | A beat notes the run's thread (`Continues`) with its own root; what it opens elsewhere is `Prompted` |
| 10 | `most_exposed` picks by exposure (§4.1) | By the share of homes the hazard damaged, then by people away (a strike that fell evenly), then by exposure |
| 11 | The `arc` condition for overheard lines and topics (§4.1) | Gains `here` (where a role of the run is). An arc's own conditions cannot use it |
| 12 | The flood's JSONC and numbers (§7.2) | See the note under §7.2 |
| 13 | The doctor's static lints (§4.5) | Unreachable stage, no exit from a stage, a stage that does not end on time, an exit without effects, a mark read and never set (errors), under three exits and a mark never read (warnings), an unknown role, a missing key or argument: in content validation. The rest are A4's |

What the engine parses and does not do yet (never raised: `arcs::not_yet`): a `subject` cast, a clock read from the subject, `gov_approval`, `place_state`, any `requires`. Parsed and inert until their task (`mandate_content::arcs::inert`): `hidden`, `teaches` (the arc's ledger subject, A3), `climax`, `goals`, `answers`, `opportunity`, `follow_up`, `legacy_key` (A2), `sides` (A4).

## 13. What task A2 built, and every correction it made to this note

Built (save v28): the director on arcs (`narrative/director/arcs.rs`: the lead, the placed clock, the chapter's turn; `narrative/offered.rs`: the arcs' decision beats, presented within the budgets), the storylines folded into arcs and their machinery removed, the budget round and the rival as standing arcs, changes voted into the next budget (finance), seven conditions, and the v27 → v28 migration. Tests: `mandate-worldgen/tests/budget_round.rs`, `narrative.rs` (the pacing bar at 95%, spotlight on against off with arcs among the outcomes, the folded arcs), `director.rs`, `mandate-save/tests/storylines.rs`.

**The budget round** (`arc.budget_round`, `standing: "session"`): *estimates* (read them at the hall, ask the proposer where there is room) → *line* (where the player stands; a colleague's ask; the colleague counts heads) → one of four turns → four exits (*carried*, *defeated*, *passed*, *late*). It is raised by the budget calendar (`budget_year`: 250 to 110 days before the treasury's year turns), once a budget year. The vote's day is a director-owned clock, 190 to 40 days before the year turns. The turn, chosen as the vote nears by the first edge that holds, never the turn of the last round (`arc_last_turn`):

| Turn | Signal (simulated state) | What the player may move |
|---|---|---|
| a hole to fill | the treasury closed its last year at least 1% short of its revenue | 3% off every line (the room decides), or a rise in the rates (a bill of their own) |
| a deal across parties | no party holds 60% of the posts that approve the budget | sit down with the other side (the room decides), or hold the party's line |
| a surplus to spend | it closed at least 5% over | 3% more on every line (the room decides), or a cut in the rates (a bill) |
| a row over one line | otherwise | a colleague's amendment for one neighbourhood, for or against |

When the last turn was the row, the smaller shortfall or the smaller surplus picks the turn, so no two years match. Whether a change the player moved carries is the room's arithmetic on the day (`body_support`: the player's party, members who think well of them, members who owe them a favour; half of the others). A carried change is voted into the next budget and applied when the year turns. Forty generated towns, as play begins: every one has a one-party majority (so "no majority" would never fire, and the signal is a majority without a seat to spare, in about a quarter); most close a few per cent over (the balanced-budget rule), so the hole is rare until a town's finances turn.

Corrections, each marked **(A2)** where it applies:

| # | The note said | As built, and why |
|---|---|---|
| 1 | Five powers (§3) | Three built here: the lead, the placed clock, when a decision beat is presented. Hazard holds and discovery dates were built (`Chance`). The arrival of an opportunity arc waits for the first one |
| 2 | A director-owned clock is "a keyed draw, biased" (§3.2) | `clock.placed`: the run's own keyed draw, taken inside the overlap of the content's range and the chapter's climax window (shifted by the turn's lead); at the nearer end of the range when they do not overlap; anywhere in the range without a tension curve (Unfiltered). It reads the chapter's dates and the profile, never the spotlight |
| 3 | The step asks the narrative with `OpenDecision` (A1) | The request is gone. A run lists the beat in `open`; the director reads `ArcsState` and presents it (`Decision::arc`). The run's cast fills the roles the dilemma shares; a role the arc lacks (a bill) is cast, and the dilemma's triggers checked, as for any decision |
| 4 | The chapter's climax is the lead's turn, "its decision has weight climax" (§3) | Counted when the stage is reached; the decision is `climax` when the beat asks for it and the run leads. A lost bet on the lead is settled as the chapter closes (the first turn reached inside it) |
| 5 | "A material arc outranks both" (§3) | Material is theme `crisis`. The favour debt is not: it ranks below a standing arc |
| 6 | Tension reads `max(heat)` in place of crisis severity (§3) | `max(heat, severity)` until the crisis registry's matters are arcs (A6) |
| 7 | The budget round's variants: "a revolt on the line, the executive against the body" (§3) | A row over one amendment, and a deal across parties (the owner's four). The executive's own side of the round (a player who holds the proposing power) is not written: the proposer role cannot be the player, so such a player gets no round yet |
| 8 | Raised by "the constitution's budget calendar" | The constitution has no budget calendar; every treasury closes its books on 1 January (`finance::budget_year_turns`, the one place a tier's own fiscal year would be read). `budget_year` reads it |
| 9 | `treasury { at, amount, purpose }` (§4.3) | Not built. `budget { factor, category? }` → `Effect::Budget` → `finance::vote_budget`: next year's discretionary budget (one line or all) × factor, applied as the year turns. A change to the current year would be undone by the yearly rule |
| 10 | Conditions (§4.1) | Added: `budget_year`, `budget_balance` (`$books`, `$gap`), `body_majority` (with `share`), `body_support` (`$backers`, `$members`), `arc_last_turn`, `last_race`, `sponsors_bill`. All tier-generic; the round is `tiers: ["local"]` because the bills its turn drafts are a town's |
| 11 | Casts (§4.1) | Added: `rival`, `creditor`, `workplace { of }` (where a cast person's post sits), and `post.party` (a preference for the player's party or another, never a filter) |
| 12 | A run per jurisdiction (A1) | A run a template in a jurisdiction; `arcs_active` caps those that are not standing |
| 13 | The clock is checked daily (A1) | And on the day it runs out the run first settles what was answered since the week's step |
| 14 | The save migration (§1) | v27 → v28: a running storyline becomes a run at the stage of the same id (the last it reached), with its cast, the marks its answered decisions set and a waiting decision as the run's open beat; a storyline at a stage the arc lacks ends. The narrative loses `storylines`, `run_ids`, `follow_ups`. The log's `storyline_stage` events stay |
| 15 | `ArcRun`, `ArcSummary`, `Chapter` | Gain `turn` and `before` (the last run's turn), `turn`, and `lead` and `turn` |

Still parsed and inert (`mandate_content::arcs::inert`): `hidden`, `teaches` (A3), `sides` (A4), `goals` (a goal's own conditions can read the `arc` condition), `answers`, `follow_up`, `legacy_key`, `opportunity`. A stage's `climax` is no longer among them.
