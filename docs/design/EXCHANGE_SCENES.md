# Exchange scenes: an arc's turn is played

**Status:** design for the owner (2026-11-25). Nothing here is built. A decision record is proposed in §14. It extends `STAGED_ARCS.md` (DEC-109) with one beat kind and one rule.

The owner's question: "Being challenged to a debate do we just click the yes button and it happens? Or for the favor do we just say yes or no? We will want our storylines to be engaging for the player." Today the answer is yes to both. The rival arc's turn is one decision with two options (`content/base/arcs/rival.jsonc`, stage `showdown`; `dilemma.debate_challenge` in `content/base/dilemmas.jsonc`), and its aftermath is one line: "You debated { $rival } in front of a packed hall. The whole town heard you." (`locale/en/narrative.ftl`, `dilemma-debate-accept-after`). The favour debt's turn is one decision with three (`arcs/favour_debt.jsonc`, stage `called_in`; `dilemma.favour_called_in`). The flood arc is the standard to reach: visits, talks, a money decision, a hall meeting, press and a law (`arcs/flood.jsonc`).

The owner agreed the direction ("Yes let's do it"). This note makes it buildable.

**The rule for every arc.** The turn is something the player does with the game's own verbs: go, talk, lobby, vote, draft, speak to the press. What they know and whom they know changes their options. A turn is never a single pop-up.

**The one new mechanism.** An **exchange**: a short scene in the conversation view, three rounds, about two minutes. The other side's prompts are true (an attack drawn from the player's record). The player's lines are earned (a law that passed, a ward they have listened to, something they found out). Each option says whom it lands with and what it risks before it is chosen.

---

## 1. What it is and is not

**It is a structured contest whose moves are generated from state.** Two sides face an audience. For three rounds the other side puts something to the player, the player answers with one of four or five lines, and the audience leans a little one way or the other. Then it ends won, split or lost, and the simulation takes the result: approval and recognition group by group, what people remember, the paper's story, the arc's marks.

**It is not a minigame with its own rules.** It has no hit points, no cards, no resource that exists only inside it. Every quantity it reads is one the game already keeps: a law's ledger (`PolicyInstance::ledger`, `policy::delivered` in `mandate-sim/src/policy/mod.rs`), a worry's rank in a place (`concerns::ranked`), the player's promises (`AskState` in `asks/mod.rs`), declared votes (`PoliticsState::declared_votes`), how well a fact is known (the ledger's `Level`), opinions, favours, the run's heat. Every quantity it writes is one the game already has, through the owner's effect request. A player who understands the town understands the scene. A player who learns the scene has learned the town.

**It is not a dialogue tree written by hand.** No writer lists "if the player says A, the rival says B". A scene is a block of data on an arc's stage: who is there, where prompts come from, which kinds of move are allowed. The engine builds this career's prompts and lines from this career's record. Writers supply sentences by kind ("an attack on a broken promise, as a rival says it"), which take the names and numbers as arguments (DEC-054). This is the rule conversations already follow: "Topics, not dialogue trees" (DEC-093).

**It is not a test of reflexes or of guessing.** There is no timer. The prompt is on screen before the player chooses, so the other side's move is known when the player moves. This is the telegraph the research file already took from Into the Breach (`research/ENGAGEMENT_REFERENCES.md`, "the rival's next move shown as a lead once learned"). Nothing is hidden behind a roll that the option did not show (rule 10 of `AGENT_CONTEXT.md`: shown = rolled).

**It is not a decider.** DEC-109's second rule holds: transitions read; beats write. An exchange sets marks and posts effect requests. It never names the next stage or the exit. A debate won in a town that will not have the player still ends with the voters choosing someone else, and "Why?" can show both facts.

**It is small on purpose.** Three rounds, only at an arc's turn, at most one a chapter (§13, question 4). The same research file records the fault reviewers found in Suzerain: sections that "go on too long". Two minutes is the budget, and §7 counts it in taps.

**Why one mechanism for four situations.** A debate, a negotiation, a hearing and a hostile meeting share a shape: someone puts a true thing to you, you answer with what you have, and people who matter to you watch. They differ in who watches, what the other side holds, and what ends it. Those three are data (§2). So one mechanism serves all four, and §6 shows they do not play alike.

**What exists to build on, and what does not.**

| Thing | State today | Where |
|---|---|---|
| Arcs, stages, marks, heat, seven beat kinds | built (A1, A2) | `mandate-sim/src/arcs/`, `mandate-content/src/arcs.rs` |
| A system's own choice inside a conversation | built: talk hooks | `mandate-sim/src/talk/hooks.rs` |
| The conversation view (dialogue box, bubbles, portrait mode) | built | `web/src/components/talk/TalkSheet.tsx`, `dialogueModel.ts`, `stage.ts` |
| Odds shown from the function the roll uses | built for lobbying, endorsements, press, visits | `talk::odds`, `actions::press_gaffe_risk`, `visits::goes_wrong` |
| The arc view, "what you can do" (A3) | not built: there is no `mandate-protocol/src/arcs.rs` | `STAGED_ARCS.md` §9 |
| Stand-ins, grader (A4) | not built: there is no `mandate-sim/src/standin.rs` | same |
| Plants with short time, a notice and a date (DEC-110) | accepted, not built: no `PlantThreatened` in the code | `INDUSTRIES.md` |
| Town secrets, digging (E9) | not built: `Subject::Secret` exists in the ledger, no dig command | `research/ENGAGEMENT.md` §5 |
| Research on debate or dialogue-contest systems | none in `docs/design/research/` | the two notes used are cited above |

---

## 2. The shape as data

An exchange is an eighth beat kind, `exchange`, on an arc stage. It sits beside the seven of `STAGED_ARCS.md` §1.5 and follows their rules: it is offered while its stage lasts, it is one beat with an id, it sets marks, and it is reachable from quick mode.

### 2.1 The parts

| Part | What it says | Bound to |
|---|---|---|
| `at` | Where it is held | a cast place role (`ArcCast::Place`, or `Workplace { of }`) |
| `time` | What it costs to take part | `slot` or `half`, as a visit beat (`VisitTime`) |
| `sides` | Who is across from the player (`other`), who may chair (`chair`), who may speak for the player (`second`) | cast person roles |
| `audience` | Who watches: one to five groups | live state (§2.2) |
| `rounds` | How many (three) | a number |
| `prompts` | For each round, which sources the other side may draw on, in the order tried | attack sources (§4.1) |
| `moves` | Which kinds of line the player may be offered | move kinds (§3) |
| `prep` | Marks from earlier beats that add a line or remove a risk | the run's marks |
| `temper`, `count`, `table` | Optional features: a room's anger, a head count, things on the table (§2.4) | the run's heat; posts by power; effects |
| `outcome` | What `won`, `split`, `lost` and `absent` do to the run: marks, heat, gated effects | `OptionOutcome`, as a decision beat's `on` |
| `text` | The key prefix for its sentences | Fluent |

### 2.2 The audience comes from live state

The audience is never named in code or in content. Content names a **source**; the engine binds it to what the jurisdiction has.

| Source | Groups it gives | Read from | Effects land through |
|---|---|---|---|
| `areas` | The areas one level down from where the scene is: a race's neighbourhoods, a town's wards, a region's towns | `Area`, `Constituency::neighborhoods`, `Jurisdiction::hoods` (`world.rs`) | `Effect::Approval { scope }`, recognition (§11.4) |
| `classes` | Income classes inside one area | cohorts (`population/mod.rs`); `Effect::Approval` already takes `classes` | the same |
| `communities` | The communities present in an area | `PopulationState::community_keys`, `community_of` | the same, by community (new: §11.4). The base pack has one community (`content/base/communities.jsonc`), so this source gives one group until a pack adds more |
| `workers` | The people who work at a cast place | DEC-110's `economy::plant_facts` (the districts its workers live in). Not built; until then the area cast for the arc stands in | the same |
| `members` | The holders of posts with a power at the jurisdiction, one group each | posts by `Power` (as `Condition::BodySupport` counts them) | `Effect::Opinion`, `Effect::Remember` |
| `person` | One cast person | a cast role | `Effect::Opinion`, `Effect::Favour` |

Two families follow. **Crowds** (the first four) are cohorts: they care about worries, and what moves them is approval and recognition. **People** (the last two) are characters: they care about their goals, their memories of the player and what they are owed, and what moves them is opinion and memory. The formula is the same for both (§5). What "fit" reads is different (§5.3).

- **Size.** At most five groups. Where a source gives more, the four with most weight are kept and the rest are one group ("elsewhere"), which targeted lines cannot name.
- **Weight.** Crowds weigh by voters (`count × turnout`, as `policy::public_support` weighs them). Members weigh one each. Weights sum to 1.
- **Starting lean.** Each group starts where the simulation has it: a crowd at its approval of the player among those who know them (`Cohort::player_approval`, weighted by `player_awareness`); a member at their opinion of the player, with a step for sharing a party and a step for owing the player a favour (the three terms of `body_support`); a person at their opinion less what they are demanding.
- **Tier-generic (DEC-098).** A source is evaluated at the run's jurisdiction, so the same scene is a ward's neighbourhoods in a town race, towns in a regional one and regions in a national one. No office or body is named: members are found by a power id.

### 2.3 The debate, in full

This replaces the decision beat in `arc.rival`'s `showdown` stage. Field names are this design's proposal; the ids of existing things are real (`arc.rival`, the `rival` cast, the place kinds, the promise menu). New text keys are named by prefix only.

```jsonc
// content/base/arcs/rival.jsonc, in stage "showdown" (climax). The roles `hall`, `chair`
// and `backer` are added to the arc's `roles` (§10.1).
{ "id": "debate", "type": "exchange", "once": true,
  "at": "hall",                         // cast: { "type": "place", "kinds": ["community_centre", "school", "town_hall"] }
  "time": "slot",                       // one slot for the whole scene, however many rounds
  "sides": { "other": "rival",          // cast: { "type": "rival" }
             "chair": "chair",          // optional. cast: occupation.editor or occupation.journalist; asks the last question
             "second": "backer" },      // optional. speaks only if the `endorsed` mark is set
  "layout": "crowd",                    // presentation only: crowd | members | table
  "audience": [
    { "from": "areas", "of": "race",    // the areas one level down of the constituency the player is standing in
      "max": 5, "weight": "voters", "start": "approval" } ],
  "rounds": 3,
  "prompts": [
    // Each round: the sources the other side may use, best first. The first that gives an
    // attack it holds (§4) is used; `top_worry` always gives one.
    { "round": 1, "by": "other", "from": ["broken_promise", "vote_against_lean", "failed_law", "thin_record"] },
    { "round": 2, "by": "other", "from": ["scandal", "hidden_fact", "absent", "vote_against_lean", "governing_record"] },
    { "round": 3, "by": "chair", "from": ["top_worry"] } ],
  "moves": ["record", "heard", "turn", "promise", "act", "own", "second", "pass"],
  "prep": [
    { "mark": "rehearsed", "removes": "cold" },             // set by a talk beat in the stage before
    { "mark": "endorsed",  "adds": "second" },              // set by a talk beat with `backer`
    { "mark": "pact",      "bars": ["hidden_fact", "scandal"] } ],   // a truce taken earlier: the rival keeps off the personal
  "outcome": {
    "won":    { "sets": "debate_won",    "heat": -0.10 },
    "split":  { "sets": "debate_split" },
    "lost":   { "sets": "debate_lost",   "heat": 0.10 },
    "absent": { "sets": "debate_ducked", "heat": 0.05 } },  // the stage's clock ran out: an empty chair (§5.5)
  "text": "exchange-debate" }           // exchange-debate-prompt-<source>, -say-<kind>, -reply-<kind>, -result-<outcome>
```

The numbers a scene may set for itself (thresholds, cover, gain) default to the shared block in content (§5.6) and are given here only when a scene must differ.

### 2.4 The other three, in outline

**The favour called in: a negotiation.** One person across a table, in private.

```jsonc
{ "id": "table", "type": "exchange", "at": "creditor_place", "time": "half", "layout": "table",
  "sides": { "other": "creditor" },
  "audience": [ { "from": "person", "role": "creditor", "start": "demand" } ],
  "table": { "wants": [ { "type": "vote", "role": "bill", "vote": "yes" } ],      // what the other side asks for
             "watchers": { "from": "members", "power": "approve_ordinance", "who": "owed_or_owing" } },
  "prompts": [ { "round": 1, "from": ["demand"] }, { "round": 2, "from": ["debt_record", "bind"] }, { "round": 3, "from": ["terms"] } ],
  "moves": ["offer", "record", "own", "promise", "turn", "pass"],   // `turn` on public record only (§3.2)
  "outcome": { "won": { "sets": "settled" }, "split": { "sets": "part_paid" }, "lost": { "sets": "no_deal" }, "absent": { "sets": "stayed_away" } },
  "text": "exchange-table" }
```

**A hearing.** The members of a body question the player about a story that broke.

```jsonc
{ "id": "hearing", "type": "exchange", "at": "chamber", "time": "slot", "layout": "members",
  "sides": { "other": "questioner", "second": "ally" },
  "audience": [ { "from": "members", "power": "approve_ordinance", "max": 7, "start": "support" } ],
  "count": { "needs": "majority" },                                  // the result is a count of heads, not a mean
  "prompts": [ { "round": 1, "from": ["the_fact"] }, { "round": 2, "from": ["contradiction", "who_else", "evidence_held"] }, { "round": 3, "from": ["fitness"] } ],
  "moves": ["own", "deny", "proof", "record", "turn", "second", "pass"],
  "outcome": { "won": { "sets": "cleared" }, "split": { "sets": "warned" }, "lost": { "sets": "censured" }, "absent": { "sets": "stayed_away" } },
  "text": "exchange-hearing" }
```

**A hostile public meeting.** One room, angry, after a flood or a closure.

```jsonc
{ "id": "room", "type": "exchange", "at": "hall", "time": "slot", "layout": "crowd",
  "sides": { "other": "organiser" },                                 // voices the floor; has no record of their own to attack
  "audience": [ { "from": "classes", "of": "hood", "start": "approval" }, { "from": "person", "role": "organiser", "start": "opinion" } ],
  "temper": { "reads": "heat", "drag": 0.5 },                        // the run's own heat: a hot room hears less
  "prompts": [ { "round": 1, "from": ["the_matter"] }, { "round": 2, "from": ["absent", "who_pays", "broken_promise"] }, { "round": 3, "from": ["top_worry"] } ],
  "moves": ["promise", "own", "record", "act", "heard", "offer", "turn", "pass"],   // `offer` here is time: stay for every question
  "outcome": { "won": { "heat": -0.25 }, "split": { "heat": -0.10 }, "lost": { "heat": 0.10 }, "absent": { "heat": 0.20 } },
  "text": "exchange-meeting" }
```

The three optional features are the whole difference in rules between the four:

| Feature | What it adds | Used by |
|---|---|---|
| `temper` | The run's heat scales every gain down; some moves cool it. Order matters | the meeting |
| `count` | The result is how many members end on the player's side, against a threshold | the hearing |
| `table` | Offers wait on the table and take effect only if a deal is struck; watchers learn the result | the negotiation |

### 2.5 The content types (sketch)

```rust
// mandate-content/src/arcs.rs: BeatKind gains Exchange(ArcExchangeDef)
pub struct ArcExchangeDef { pub at: String, pub time: VisitTime, pub sides: SidesDef, pub layout: Layout,
    pub audience: Vec<AudienceDef>, pub rounds: u8, pub prompts: Vec<RoundDef>, pub moves: Vec<MoveKind>,
    pub prep: Vec<PrepDef>, pub temper: Option<TemperDef>, pub count: Option<CountDef>, pub table: Option<TableDef>,
    pub outcome: BTreeMap<Outcome, OptionOutcome>, pub text: String, pub params: Option<ExchangeParams> }
pub enum AudienceFrom { Areas, Classes, Communities, Workers, Members, Person }
pub enum MoveKind { Record, Heard, Turn, Promise, Act, Own, Deny, Offer, Second, Proof, Pass }
pub enum Outcome { Won, Split, Lost, Absent }
```

The types are named `ArcExchange…` because `Exchange`, `ExchangeId` and `ExchangeView` already mean one logged conversation (`talk/state.rs`, `mandate-protocol/src/talk.rs`), and `SceneDef` is a decision's spoken beats (`mandate-content/src/narrative.rs`). `ArcBeat` got its name the same way (`STAGED_ARCS.md` §4.2).

---

## 3. Moves

A move is one line the player may say. It has a **kind** (code: it knows what to read) and a **subject** (state: this law, this neighbourhood, this vote). The engine builds every move the state supports, then shows four or five.

### 3.1 The kinds

Strength is a number from 0 to 1, computed from the quantities named. Every constant in this table is content (§5.6).

| Kind | The line | Requires | Strength, from real quantities | Risk | Real side effects |
|---|---|---|---|---|---|
| `record` | "Answer with your record" | A law in force that the player sponsored, or declared for (`declared_votes`), whose ledger shows delivery on the subject | What it delivered against what the worry needs: the ledger reading for that worry (`jobs_added_*`, `homes_added`, `damage_reduction_pct`, `tax_change_pts`, `funded_pct`) ÷ the worry's `full` amount for a place that size, × the law's `progress`. A law the player only voted for counts at `voted_share` | The other side may hold an answer: the group the law hurt (its `side_effects`, DEC-073) | None. A law's `area` clause decides whom it reaches |
| `heard` | "What you heard on the doorstep" | The group's worries at `known` or better in the ledger (`Subject::HoodConcerns`, `TownConcerns`) | The felt level of the group's top worry (`concerns::ranked`, `Worry::Felt`) × how much the player has been there (`GroundState::visits_now` against `visits_full`, floor `heard_floor` for a worry learned only by talk) | None | None. Lands with that group; others get `spill` |
| `turn` | "Turn it on their record" | Something true about the other side: a declared vote against a group's lean, a thin record (`Politician::record`), a running story (`MediaState::running_about`), or a hidden fact the player holds (`Subject::Secret`, once E9 can find one). The player must know the person at `known` or better: a public fault still has to be looked up | The fault's own size (the group's lean on that vote; `1 − record`; the story's heat; the fact's gravity). A public fault counts at `public_share`: people have heard it | A hidden fact stands up with chance `hold[level]` (heard, known, detailed), shown. If it does not, it backfires | The other side remembers (`pressed_me`). A hidden fact aired is out: the media open it as a story (§11.4). Not allowed with a hidden fact in a private scene (§13, question 2) |
| `promise` | "Promise it now" | In a campaign: a promise on the menu (`asks::commands::promise_menu`), within `promises.max_per_campaign`. Outside one: the scene names an ask its `other` or `chair` makes, accepted on the spot | The worry's weight in the group × the player's word: halved for each public promise broken (the discount promises already carry, `AskState::broken_public_promises`) | None now. The other side may answer "another promise" if one was broken and not owned | A real promise in the asks system, with its deadline, its keep and its break (`asks/promises.jsonc`) |
| `act` | "I will table it" | A power that lets the player draft (`HoldsPower`), and a template that answers the subject (the arc's `answers`, a crisis's answers) | `act_base`, more if the room for it is there (`body_support` at or over a half): "and I have the votes" | The other side may answer "you have had N years" if the player has held a post a full term with no law on it | A draft exists (the path a dilemma's `draft` effect uses). The paper names it |
| `own` | "Own it" | The prompt is a fault of the player's own | No gain. It covers most of the bite (`cover.own`) | None | The attack is retired: it cannot be used again in this scene, nor as an answer to a later move. Standing falls a little. In a hearing it is an admission: the story's evidence in the open rises (the scandal's own rule: "an admission adds to it", `media::Scandal`) |
| `deny` | "Deny it" | The fact is not proved: its evidence in the open is under 1 | No gain. It covers all of the bite if it stands | It unravels with chance equal to the evidence that exists (`HiddenFact::evidence`), shown. If it does, the prompt bites whole and the denial backfires (`backfire.deny`) | A denial on record. If it unravels: the story the media already write for one (`StorySource::Backfire`) |
| `offer` | "Put something on the table" | Something real to give: a vote the player can still declare (`can_vote`), a favour, money, a rider the sponsor must accept, time (a second slot tonight) | What it is worth to the other side ÷ what they asked for (§6.2). Time: `stay_strength`, and it cools a room | None. The cost is the thing given | With a `table`: applied only if a deal is struck. Without: applied when said (time is spent now) |
| `second` | "Let them speak for you" | The `endorsed` mark, from a talk beat with the cast backer | `second_base`. Its fit is the backer's tie to each group, not a worry: top where they live, work or hold a post (`Ctx::ties`), low elsewhere | None | The backer has spent their name: their opinion of the player falls if the scene is lost |
| `proof` | "Show them" | Something found: the arc's subject at `detailed` in the ledger, or a mark set by a visit beat ("you have the letter") | The evidence it carries | Stands up with chance equal to that evidence, shown | What it shows is public from then on |
| `pass` | "Move on" | Nothing. The safe move | No gain. It covers a little (`cover.pass`) | None | None |

Notes:

- **Every kind but `pass` has a requirement.** Content validation refuses a scene that lists a kind whose requirement the arc can never meet (§9.4).
- **A subject is used once.** One law, one neighbourhood's doorstep, one fault: each can be said once in a scene. A kind used a second time, on another subject, lands at `repeat` of its strength.
- **The player's oratory** scales every gain a little (`0.8 + 0.4 × oratory`, from `Politician::oratory`, as `actions/publicity.rs` already reads it). It is in the preview. The record matters more than the voice (§13, question 3).

### 3.2 A move answers, or changes the subject

A move **answers** the prompt when it is `own`, `deny` or `pass`; when it is `proof` on the prompt's fact; or when its subject is the prompt's subject (the same worry, the same law, the same promise). An answer takes away part of the prompt's bite (its **cover**). Any other move leaves the bite whole and adds its own gain. Changing the subject is allowed, and it costs what was left unanswered.

In a private scene `turn` reads the public record only. Pressing someone in private with a secret is the act DEC-097 (owner's answer 4) keeps out of M6.

### 3.3 Which moves the player sees

1. **Build** every move the state supports for this round: each allowed kind × each subject that meets its requirement and is not used.
2. **Score** each by its expected change in lean, summed over the groups by weight (§5.2), using the engine's true numbers.
3. **Pick**, in this order, skipping a pick that repeats one already taken:
   1. the safe move (`pass`), always;
   2. each move a `prep` mark added, if it applies this round (at most two): preparation is never hidden by the cap;
   3. the best move that answers the prompt;
   4. the best move that changes the subject;
   5. the best move that carries a chance, if there is one;
   6. the best of the rest, until there are five.
4. **Show one locked line**, greyed, when a kind the scene allows has no subject: "Answer with your record: you have no law on housing." It is not a choice. It tells the player what earlier play would have given them, as an unavailable topic gives its reason today (`TopicView::unavailable_key`).

So the list is four or five lines and one hint. The order of the list is fixed by kind (the table's order), not by score: the first line is not "the right answer". Ties in a pick go to the kind earlier in the table, then to the lower id.

The score in step 2 uses truth, not what the player knows. The player's ignorance changes what the **preview** can tell them (§5.7), never which lines exist beyond the requirements above. This keeps DEC-091's guard: with the same commands, a player who knows everything and one who knows the default get the same world. It also means the list tells an unprepared player a little: these are the lines worth weighing. The preview still does not tell them why.

### 3.4 Preparation

Preparation is ordinary play in the stages before the turn. Most of it works through state the game already keeps; marks are for the two things state does not hold.

| Before the scene, the player… | By | Which gives |
|---|---|---|
| walks a neighbourhood, canvasses it, talks to someone who lives there | existing actions; a `visit` or `talk` beat | Its worries in the ledger: the `heard` line for that group, and exact previews of how anything lands there |
| passes a law, or gets one funded | existing commands | A `record` line, as strong as the ledger says |
| looks into the other side (the paper's cuttings, a colleague who knows them) | a `visit` or `talk` beat that teaches `Subject::Person` to `detailed` | Their public faults as `turn` lines; exact previews of the answers they hold |
| digs something up (E9, when built) | the dig | A `turn` line with a chance by how well it is known |
| keeps their promises, votes with the groups they answer to, turns up | play | Fewer attacks in the other side's deck (§4.1) |
| rehearses with an ally | a `talk` beat that sets `rehearsed` and teaches the arc to `detailed` | Round 1 is not "cold" (§5.2); the arc sheet lists what the other side holds, as of that day |
| asks someone to speak for them | a `talk` beat with the cast backer that sets `endorsed`; the reply's conditions are opinion, trust or a favour, as today's arc replies | The `second` line |
| lobbies members, calls in a favour | `Lobby`, `Talk`, `TradeFavor` | A member who starts the hearing on the player's side; a vote the creditor no longer needs |

An endorsement is a mark because nothing stores one today: `talk/mod.rs` (`Mech::Endorse`) applies an approval bump in the endorser's home neighbourhood and keeps nothing.

---

## 4. The other side

### 4.1 Where a prompt comes from

A prompt is a true thing about the player, or about the matter, put by someone with a reason to put it. Each **source** is code that reads one owner's state and gives zero or more **attacks**. An attack has a kind, a subject, a size from 0 to 1, the groups it bites with, and the event on the spine it is anchored to.

| Source | Reads | Size | Anchor (`spine::Thread`) |
|---|---|---|---|
| `broken_promise` | Promises and accepted asks that ended broken (`AskState::resolved`, `AskStatus::Broken`) | `promise_size`, × recency | the ask's closing event (`Thread::Ask`) |
| `vote_against_lean` | The player's declared votes (`declared_votes`; the deed log's `Deed::Vote`) on bills where a group in the audience leans the other way (`policy::public_support` over that group's neighbourhoods) | how far the group leans | the vote, a step in `Thread::Bill` |
| `failed_law` | A bill the player sponsored that failed, expired or was withdrawn; or one in force whose ledger shows little (`funded_pct` low, `delivered` near nothing) | `1 −` what it delivered | the bill's closing or enacting event |
| `thin_record` | `Politician::record` under a line (the reading `Condition::RivalRecord` uses for the rival) | `1 − record` | none: a level, not an event |
| `absent` | In this arc: beats never taken, a decision let pass (`PlayerAct::LetPass`), an ask left to lapse; a cast person who remembers `ignored_me` | by count | the arc's own events (`Thread::Arc`) |
| `scandal` | A story running about the player (`MediaState::running_about`) | its heat | `Thread::Scandal` |
| `hidden_fact` | A hidden fact about the player that the speaker knows (`MediaState::knowledge` with `Knower::Person`) | its gravity × the evidence they hold | `Thread::Hidden`. Saying it makes it public |
| `governing_record` | Where the player's party governs: the group's approval of the government (`Cohort::gov_approval`) | how far under a half | none |
| `the_matter`, `who_pays` | The arc's own readings (`facts`): weeks, people away, homes repaired, jobs; and whether a named mark of the run is missing (the money was never put up) | by the reading, in content | the run's root event |
| `demand`, `debt_record`, `bind`, `terms` | The negotiation's: what is asked; how the player has paid before (`AskState::history_of`, `Condition::Remembers`); what the vote would cost them (§10.2) | by the favour and the bill | the favour's event; `Thread::Bill` |
| `the_fact`, `contradiction`, `who_else`, `evidence_held`, `fitness` | The hearing's: the story; an earlier denial on record (`StorySource::Response`); the others the fact names; what the questioner holds; the player's record | the story's evidence and gravity | `Thread::Scandal`, `Thread::Case` |
| `top_worry` | The audience's strongest worry (`concerns::ranked` over all its groups) | `question_size`: a question, not an attack | none |

- **Through the spine.** An attack that names an event exists only while the log keeps that event. The log already forgets by salience: routine events in four years, notable ones (a law enacted or failed, an ask settled) in twelve (`CAUSAL_SPINE.md` §5). So the town forgets a broken promise when the game does, by one rule, and "Why?" on a prompt can always walk back from it.
- **What a lean is.** `vote_against_lean` reads the group's lean now, not on the day of the vote. Voters judge with today's attitudes.
- **Bites by group.** An attack bites a group by how much that group cares about its subject (§5.3). A vote for a rates rise bites hardest where taxes are the top worry.

### 4.2 What the other side knows and holds

Public record needs no knowledge: votes, laws, public promises and stories are public. A hidden fact needs the speaker to be one of its knowers. Nothing reads the player's ledger: the world does not wait for the player (DEC-091).

As the scene begins the engine builds the other side's **deck**:

1. Every attack every source allowed in the scene gives, and every **answer** they could give to a move of the player's (§4.3), ranked by weighted bite.
2. **Skill sets its size.** `deck_base + round(deck_per_skill × skill)`, where skill is the mean of their `oratory` and `intrigue` (`characters::Competencies`). The difficulty lever "rival competence" (DEC-102 clause 3) scales it.
3. **Skill sets its aim.** A weak opponent ranks by what a statistic shows (`Worry::Measured`); a strong one by how residents feel (`Worry::Felt`). This is DEC-030's "polls, not truth", with better information as the only advantage.
4. **Character shapes it.** High boldness moves `hidden_fact` and `scandal` up; high integrity bars a hidden fact held on weak evidence; a grudge (`trait.grudge_holder`, or a memory of the player against them) moves the personal sources up; warmth moves `top_worry` up. These are weights in content, by personality axis and trait id.
5. A `prep` rule may bar sources (the truce bars the personal ones).

Each round the prompt is drawn from the round's sources that are in the deck: the best, or one of the best `k` for a weaker opponent (`pick_top` by skill), by a keyed draw.

### 4.3 How they answer a move

After the player's line the other side may **answer** it, if their deck holds an answer to it and they have answers left (`answers_base + round(answers_per_skill × oratory)` a scene). An answer cuts the move's strength by its size.

| The player's move | The answer they may hold | Its size |
|---|---|---|
| `record` | The group the law hurt (its declared grievance) | the grievance, with that group |
| `promise` | A broken public promise not owned in this scene | `answer.promise` |
| `turn` | The same fault in the player (they attack a vote; the player has one too); or their own law on the subject | `answer.turn`; their law's delivery |
| `act` | A full term in a post with no law on the subject (`years_in_office`, the player's laws) | `answer.act` |
| `second` | The backer's own standing with the group | by the backer's ties |
| `heard`, `own`, `pass`, `offer`, `proof`, `deny` | none | |

They also follow **the player's rules** (DEC-030). Their attack is a `turn` on the player's record: a hidden fact they hold on weak evidence carries the same chance of not standing up, and if it does not, the bite turns back on them. A bold opponent takes that chance; a cautious one keeps it out of the deck.

### 4.4 Determinism

- Every draw is keyed: `keyed(seed, Stream::Misc, [tag, run, hash(beat), round, hash(option)])`, with the arcs' own tags beside `KEY_CLOCK` and `KEY_MOVE` (`arcs/mod.rs`). A1 found that a new named stream changes every game's hash, so there is none.
- No draw is keyed on a list position or on anything the player knows. The view may ask as often as it likes: a keyed draw advances no stream (`talk/reply.rs` and `visits::goes_wrong` work the same way).
- The deck is built once, as the scene begins, and saved. A reload in round 2 meets the same opponent.

### 4.5 When there is no weak spot

**The player has none.** A player with every promise kept, no vote against any group in the room and a full record leaves the personal sources empty. The round falls through its list: to `governing_record` if the player's party governs and the room is unhappy with the government; then to `top_worry`, which always gives a question. A question bites lightly (`question_size`) and opens the door to `record`, `heard` and `promise`. So a clean record is not attacked with something invented. It is asked what it will do, and it has the best lines to answer with.

A newcomer is not clean. They have a thin record, which is the oldest attack there is ("you have been here five minutes"), and no `record` line to answer it with. Their good lines are `heard` and `promise`. This is the shape of a first campaign.

**The opponent has none.** `turn` needs a fault. Where the other side has none the kind gives no move, and the locked hint says so if the player has looked into them ("Nothing on { $rival } that would stand up"). The player wins on their own ground or not at all. In a meeting the other side is the floor and has no record; `turn` there can only name the tier above, and is as strong as that tier's real silence.

---

## 5. Resolution

### 5.1 In words

Each group has a **lean**, from −1 (with the other side) to +1 (with the player). It starts where the simulation has the group. Each round does two things to it. The prompt **bites**: it pulls a group down by the attack's size times how much that group cares about its subject. The player's move **covers** part of the bite if it answers the prompt, and adds its own **gain**: its strength times how much that group cares about *its* subject. If the move carried a chance and failed, it gains nothing and **backfires**. After the last round the change in each group's lean, weighed by the group's size, is the result.

### 5.2 In symbols

For group `g`, round `r`, prompt `P`, move `M` of kind `k`:

```text
bite(g)   = B · size(P) · fit(P, g) · cold(r)
cover     = C[k] · strength'(M)          if M answers P   (own, deny, pass: C[k] alone)
          = 0                            otherwise
gain(g)   = G · voice · strength'(M) · fit(M, g) · reach(M, g) · temper
Δlean(g)  = − bite(g) · (1 − h · cover)  +  h · gain(g)  −  (1 − h) · F[k] · reach(M, g)
lean(g)  ←  clamp(lean(g) + Δlean(g), −1, 1)
```

| Symbol | Meaning | Named in content as |
|---|---|---|
| `B` | How hard any attack bites | `bite` |
| `G` | How much any line gains | `gain` |
| `C[k]` | How far kind `k` covers a prompt it answers | `cover.<kind>` |
| `F[k]` | What a failed line of kind `k` costs | `backfire.<kind>` |
| `size(P)` | The attack's size, 0 to 1 (§4.1) | per source: `promise_size`, `question_size`, … |
| `fit(x, g)` | How much group `g` cares about the subject of `x` (§5.3) | `fit.top`, `fit.next`, `fit.low`, `fit.flat`; `member_fit` |
| `cold(r)` | In round 1, `cold` unless the player knew what was coming (`rehearsed`, or the arc at `detailed`); else 1 | `cold` |
| `strength'(M)` | `strength(M)` (§3.1), × `repeat` on a kind's second use, × (1 − the answer's size) if the other side answers | `repeat`, `answer.<kind>` |
| `voice` | `0.8 + 0.4 × oratory` | `voice_base`, `voice_span` |
| `reach(M, g)` | 1 for the group a targeted line names, `spill` for the others; 1 for all when the line is for the room | `spill` |
| `temper` | `1 − drag × heat` where the scene has a `temper`; else 1 | `temper.drag` |
| `h` | 1 if the line stands, 0 if it does not. A line with no chance always stands | the keyed draw at chance `p(M)` |

**The result.**

```text
D = Σ_g weight(g) · (lean_end(g) − lean_start(g))
won if D ≥ win      lost if D ≤ −lose      else split
a group is won if its own change ≥ group_margin, lost if ≤ −group_margin
```

With a `count`: a member is with the player if their lean ends at or over `member_line`, against if at or under its negative, and undecided between. The scene is won if those with the player are the `needs` share of the body, lost if those against are, else split. With a `table`: won if the person's lean ends at or over 0 (they are paid), lost if it ends under `walk_away`, else split.

### 5.3 Fit

- **A crowd** cares by rank. `fit` is `fit.top` if the subject's worry is the group's strongest, `fit.next` if second or third, `fit.low` otherwise, read from `concerns::ranked` (`Worry::Felt`) for the group's scope. A subject that is no worry (a promise to hold meetings, a thin record) takes `fit.flat`.
- **A member or a person** cares by who they are. `fit` is a sum of content weights (`member_fit`) over personality and ties: integrity raises `own` and lowers `deny`; loyalty and a shared party raise `record` and `second`; a memory of the player (`Remembers`) raises or lowers everything; a subject that matches one of their goals (`Condition::Goal`) counts as a top worry.

### 5.4 End states

| State | When | Always true |
|---|---|---|
| `won` | `D ≥ win` (or the count, or the table) | The outcome's marks and heat are applied |
| `split` | between | The player won some groups and lost others, and the result says which |
| `lost` | `D ≤ −lose` | |
| `absent` | The stage's clock ran out and the player never began | Every prompt bites in full, the first one cold, with no answer. The other side spoke to an empty chair |

A player who begins and leaves, or ends the turn mid-scene, forfeits the rounds left: each bites in full. The scene never spans a turn.

### 5.5 The real effects

All of these are effect requests to the owners (§11.4). Each names the round's event as its cause.

| Effect | Size | Owner |
|---|---|---|
| **Approval by group** | `approve × (lean_end − lean_start)` for each crowd group, on the cohorts of that group; the population's rule scales it by the room left | population (`Effect::Approval`) |
| **Recognition by group** | `aware` for each crowd group, win or lose: being seen is being known | population (a new request) |
| **Opinion and memory** | Each member and person: `opinion` × their change in lean, with a memory when it is large. The other side remembers a `turn` used on them (`pressed_me`), a blame (`blamed_me`), a debt paid or not (`kept_promise`, `broke_promise`) | politics (`Effect::Opinion`, `Effect::Remember`) |
| **The press** | One story. The media read the scene's closing event as they read the log for town news, and write it with the outlet's own lean. Its facts are arguments: the outcome, each round's prompt kind and subject, each line's kind and subject, the group most moved, any promise made, any fact aired. Its effect on those who were not in the room goes through the media's existing perception rule (`Effect::Perceive`) | media |
| **Promises, drafts, votes, favours, money, time** | What the lines themselves did (§3.1) | asks, policy, politics, the player |
| **The arc** | The outcome's marks and heat; one mark for each kind of line that leaves one (`promised`, `aired`, `owned`, `denied`). Edges and exit gates read them | arcs (its own domain) |

The exit still belongs to the simulation. The debate moves approval, and the voters decide the race. The hearing moves members, and their count is the vote. The meeting moves the room's heat and makes a promise, and the repairs decide the neighbourhood. The table agrees terms, and the edges read what the player then does.

### 5.6 The constants

One block in content, proposed as `content/base/exchange.jsonc` (kind `exchange_params`; it does not exist yet), with per-scene overrides in the beat's `params` and per-tier overrides as DEC-098 plans. Every number in §5.2 and §3.1 is there by the name given. **All of them are placeholders** for the tuning pass at the end of M6 (rule 11 of `AGENT_CONTEXT.md`); §12 lists them. The values used in §6's examples:

| Name | Placeholder | Name | Placeholder |
|---|---|---|---|
| `rounds` | 3 | `cover.record` / `.proof` / `.own` / `.act` | 0.8 / 0.9 / 0.6 / 0.6 |
| `bite` | 0.30 | `cover.promise` / `.heard` / `.second` / `.turn` / `.pass` | 0.5 / 0.4 / 0.4 / 0.3 / 0.2 |
| `gain` | 0.25 | `backfire.turn` / `.proof` | 0.25 / 0.15 |
| `cold` | 1.25 | `hold.heard` / `.known` / `.detailed` | 0.4 / 0.75 / 1.0 |
| `fit.top` / `.next` / `.low` / `.flat` | 1.0 / 0.6 / 0.25 / 0.6 | `repeat` | 0.6 |
| `spill` | 0.3 | `win`, `lose` | 0.08, 0.08 |
| `promise_size` / `question_size` | 0.6 / 0.3 | `group_margin` | 0.05 |
| `public_share` | 0.6 | `approve` / `aware` | 0.08 / 0.04 |
| `voted_share` | 0.5 | `list_max` / `prep_pins` | 5 / 2 |
| `second_base` | 0.6 | `answer.promise` | 0.5 |

### 5.7 Shown = rolled

**What an option shows before it is chosen:**

1. What the player says (the line), and its kind.
2. **Whom it lands with:** for each group, the change in lean this line would make, as a number or a range (below), with the bite it leaves standing.
3. **Any chance:** "Stands up 3 times in 4", and the change for each group if it does not.
4. **Its cost and what it does in the world:** "Makes a promise (2 of 3 this campaign), due when the budget year turns; breaking it costs approval in your seat and Standing"; "Costs a second slot tonight"; "{ $rival } will remember it".
5. **What the other side has, if the player knows them:** "They have an answer to this", and its size.

**Proof that it is the number the engine uses.** One function computes a line's effect, and both the view and the command call it:

```text
landing(state, run, option) -> Landing { p, stands: [(group, Δ)], fails: [(group, Δ)], effects, needs }
view:     preview = redact(landing(...), what the player knows)
command:  l = landing(...);  h = keyed(...).chance(l.p);  apply(if h { l.stands } else { l.fails });  post(l.effects)
```

There is no second formula for the preview. Four tests hold it there (§12, X1):

1. **Exact when known.** With a ledger that knows everything, `redact` changes nothing: the preview equals the landing, to the bit.
2. **Inside when not.** For any ledger, any state and any option (a property test), the change applied lies inside the range previewed, group by group.
3. **The command is the view.** Ask for the view, send the option, compare the result's changes with the view's `stands` or `fails`: equal.
4. **Calibration** (DEC-073's gate, task L1): over the bots' scenes, lines shown at a chance stand up that often, within noise.

**What is knowable in advance, and what is not.** DEC-091 never gates what an action does or its trade-offs. So a chance, a cost and a side effect are always shown exactly. What the ledger gates is the precision of a size that depends on something the player may not know. The pattern is the canvass preview's: a word once the worries are known, the number once known in detail, "isn't known yet" before (`CanvassView`, `UI.md` §3).

| The size depends on | The player knows it… | The preview shows |
|---|---|---|
| The line's own strength (their law, their promise, their word) | always | the number |
| How much a group cares (`fit`) | group's worries unknown | a range, from `fit.low` to `fit.top`: "Not known how this lands in Millfield" |
| | `known` | a band: lands well, some, little |
| | `detailed` | the number |
| Whether the other side holds an answer | the person unknown or `heard` | a range that includes the answer: "They may have an answer" |
| | `known` | "They have an answer" or "They have none" |
| | `detailed` | its size |
| A member's lean (a hearing) | by `Subject::Person` for that member | unknown, a band, the number |
| What the other side will open with | the arc at `detailed`, or `rehearsed` | the arc sheet lists what they hold as of today (the deck is fixed only as the scene begins); otherwise each prompt is seen when its round opens |

A range is computed by the engine: the same formula at the two ends of what the unknown input can be. It is never a guess by the UI, and the truth is always inside it. Ignorance costs effectiveness, never fairness: a player who has not been to Millfield can still speak to Millfield, and is told they are speaking blind.

---

## 6. The four scenes worked through

Fills are from `STAGED_ARCS.md` §7: Mireenwick, Quarryrow, Sofi Brook (union steward), Rafe Garrowby (plant manager), Halle Talliswell (organiser), Celia Janssenfield (councillor), the *Corriton Ledger*. Marek Oldacre (the rival), Millfield and Harbourside are added here. Numbers use §5.6's placeholders, with `voice` at 1.

### 6.1 The debate

**State read as it begins.** The player holds a seat and is standing again.

| Read | Value | From |
|---|---|---|
| Audience | Quarryrow (weight 0.45, lean +0.10), Millfield (0.35, −0.05), Harbourside (0.20, 0.00) | the constituency's neighbourhoods; voters; approval among those who know the player |
| Worries, strongest first | Quarryrow: jobs, recovery, housing. Millfield: taxes, services, housing. Harbourside: housing, jobs, environment | `concerns::ranked`, felt |
| The player's record | Sponsored the hardship fund (`policy.hardship_fund`), in force, strength 0.7 on jobs. Broke the promise to hold town halls (`promise.town_halls`). Declared for a rates rise that Millfield leans against by 0.5 | the law's ledger; `AskState`; `declared_votes` and `public_support` |
| The player knows | Quarryrow's worries in detail (canvassed), Harbourside's (known), Millfield's not at all. Oldacre in detail (read the cuttings) | the ledger |
| Preparation | Rehearsed with Talliswell (`rehearsed`). Brook will speak (`endorsed`) | the run's marks |
| Oldacre | Oratory 0.7, bold. Record 0.2. Declared against the rebuild fund (`policy.rebuild_fund`), which Quarryrow leaned for by 0.7. Holds no hidden fact. Two answers a scene | `Politician`; `declared_votes`; `MediaState::knowledge` |

**Round 1. Oldacre:** the broken promise. "Six town halls a year, you said. I count two." Size 0.6, no worry as its subject, so `fit.flat`: a bite of 0.30 × 0.6 × 0.6 = **0.108** in every group. Rehearsed, so not cold (it would have been 0.135).

| The player may say | Quarryrow | Millfield | Harbourside | Says before it is chosen |
|---|---|---|---|---|
| Own it: "Two. It should have been six." | −0.043 | −0.043 | −0.043 | Retires the attack. Standing −1 |
| Answer with your record: the hardship fund (changes the subject) | +0.067 | −0.064 to +0.067 | −0.003 | "Not known how this lands in Millfield" (truly −0.064) |
| What you heard in Quarryrow (changes the subject) | +0.092 | −0.093 to −0.048 | −0.072 | Lands in Quarryrow; a little elsewhere |
| Let Sofi Brook speak (changes the subject) | +0.042 | −0.071 | −0.071 | Brook works in Quarryrow and is known there. Her name is spent if you lose |
| Move on | −0.086 | −0.086 | −0.086 | Safe |
| *locked:* Promise it now | | | | "You have promised this once already" |

The player owns it. Leans: +0.057, −0.093, −0.043. Oldacre has nothing to answer, and the broken promise leaves his deck.

**Round 2. Oldacre:** the rates vote. "Ask Millfield what your vote cost them." Subject taxes; size 0.5. Bites: Millfield (taxes on top) 0.150; the other two 0.038.

- *Promise it now:* no rise in the rates (`promise.no_tax_rise`, on the menu because taxes are among the seat's top three worries). It answers the prompt. The player's word is halved by the one broken promise: strength 0.5, cover 0.25. Millfield: −0.150 × 0.75 + 0.25 × 0.5 × 1.0 = **+0.013** (shown as a range, +0.003 to +0.013, because Millfield's worries are not known: the bite and the gain both turn on how much it cares about taxes). The others: +0.003. Shown too: "A promise, 2 of 3 this campaign. Due when the budget year turns. Breaking it: approval in your seat, Standing −1."
- *Turn it on his record:* he voted against the rebuild fund. Public, so it always stands, at `public_share`: strength 0.42 on recovery. Quarryrow (recovery second) +0.026; Millfield −0.124; Harbourside −0.011. "He has no answer" (known, because the player read the cuttings). "He will remember it."
- *Own it*, *Move on*, and one locked line: "Answer with your record: you have no law on taxes."

The player promises. Had they not owned the broken promise in round 1, Oldacre would hold "another promise" as an answer, the promise would land at half, and Millfield would move by −0.069 that round instead of +0.013. The first round's choice set up the second's.

Leans: +0.060, −0.081, −0.040.

**Round 3. The chair** (the *Ledger*'s editor): the room's top worry. "Jobs. What have you done, and what will you do?" A question, size 0.3. Bites: Quarryrow 0.090, Millfield 0.023, Harbourside 0.054.

- *Answer with your record:* the hardship fund answers it. Cover 0.8 × 0.7 = 0.56. Quarryrow −0.090 × 0.44 + 0.175 = **+0.135**; Millfield +0.034; Harbourside +0.081.
- *What you heard in Quarryrow:* +0.139 there, nothing elsewhere.
- *I will table it* (a closure response, if the player holds the power to draft), *Promise it now* (`promise.jobs`, the last of three), *Move on*.

The player answers with the record. Final leans: +0.195, −0.047, +0.041.

**The result.** Changes: Quarryrow +0.095 (won), Millfield +0.003, Harbourside +0.041. `D` = 0.45 × 0.095 + 0.35 × 0.003 + 0.20 × 0.041 = **0.052**: under `win` at 0.08, so **split**. "You took Quarryrow. Millfield heard a promise and is waiting to see it kept."

- Approval: +0.008 in Quarryrow, +0.003 in Harbourside, none in Millfield. Recognition +0.04 in each.
- A promise on the books. Standing −1 for the admission. Oldacre remembers nothing new: he was never attacked.
- The *Ledger*: "Debate at the hall: { $player } admits town-hall pledge was missed, promises no rates rise". The mark `debate_split`, and `promised`.

**The same evening, played otherwise.**

| Player | What happens | `D` |
|---|---|---|
| Stayed away | Three bites in full, the first cold: 0.135, 0.077 and 0.059 by weight | −0.27: lost, `debate_ducked` |
| Turned up unprepared and moved on three times | The same bites at four fifths | −0.22: lost |
| Played as in the rounds above, but had walked Millfield in the weeks before | A `heard` line for Millfield in round 2 in place of the promise: +0.067 there that round, and exact numbers for Millfield throughout | +0.060: split, with Millfield moved and no promise spent |

**What makes it itself.** The room is split by district and cannot all be won: each line chooses whom to speak to. The other side has a record and can be hit. The result is public and feeds an election days later.

### 6.2 The favour called in

**State read.** The player owes Celia Janssenfield one favour (`characters::favours_owed`). Her bill is before the body and the vote is on Thursday.

| Read | Value | From |
|---|---|---|
| The bind | The player's seat leans against her bill by 0.35. The count stands at three for, three against: the player's vote decides it | `public_support` over the seat's neighbourhoods; the members' leans (`policy::member_leans`) |
| What she wants | The vote. Worth 1.0 to her while it decides the bill | the `table`'s `wants`; the count |
| Her lean | −1.0: she is owed, and nothing is on the table | `start: demand` |
| Her record of the player | The player kept the last thing she asked (`History::Kept`) | `AskState::history_of` |
| What the player knows | Her goals, in detail: she wants this bill, and she wants a second term | the ledger (`Subject::Person`) |
| Watchers | Two members who are owed by, or owe, the player | the `table`'s `watchers` |

There is no audience strip. The screen shows a table: **hers** (your vote on Thursday) and **yours** (empty).

**Round 1. Janssenfield:** the demand. "Thursday. I need it, and you know why I can ask."

- *Put the vote on the table.* Worth 1.0 to her: it settles everything. Cost, shown: "Your seat leans against this bill. Declaring for it costs approval there." (the vote's own rule when it is cast).
- *Put a statement for her bill on the table* (a press statement, `Action::Press` on her bill). Worth 0.4 to her. Cost: a slot tomorrow; the same lean, less sharply.
- *Offer the vote if the bill takes a sunset clause* (a rider she must accept: the `rider` effect a dilemma uses). Worth 0.8 to her. With the sunset the seat's lean is −0.05, not −0.35.
- *Own the debt:* "I owe you. I am not disputing it." Covers her pressure this round; puts nothing on the table.
- *Move on.*

The player offers the vote with a sunset. On the table: the vote, if she takes the sunset. Her lean: −0.2.

**Round 2. Janssenfield:** how the player has paid before. Here that helps them: "You were good for it last time. That is the only reason I am still sitting here." A player who had broken a word to her would be bitten here instead.

- *Put the statement on the table as well.* +0.4. Her lean would be +0.2: paid.
- *Promise something later:* an ask from her, accepted now ("your vote on my next bill"). Worth 0.5 × her trust in the player.
- *Tell her what it costs you:* the seat's lean, from the public record. It is true, so it lowers what she can fairly ask: cover on this round's pressure. This is `turn` on the public record, the only kind a private scene allows.

The player adds the statement. Her lean: +0.2.

**Round 3. Janssenfield:** terms. "A sunset, and you say so in the *Ledger* before Thursday. What do I tell the others?"

- *Shake on it.* (`pass`: nothing more is offered.)
- *Ask for something back:* "Then we are square, and you owe me the next one." It takes 0.3 off the table's worth to her and, if the deal holds, leaves her owing a favour. Her lean would end at −0.1: a split.

The player shakes on it.

**The result: won, a deal.** Now, and only now, the table takes effect: the bill gains a sunset; the player's vote for it is declared; the statement is owed (an accepted ask, kept by making it); the favour is repaid. Janssenfield remembers a debt paid. The two watchers learn of it: each thinks a little better of the player, and the next time the player asks one of them for a favour the odds, which read opinion and trust, are better. The mark is `settled`.

**Played otherwise.**

- *Stayed away.* Nothing is agreed. On Thursday the edges read the vote. An abstention is the old "dodge": she counts it as a promise broken, and the watchers learn that too.
- *Lobbied a fourth vote first* (the ordinary `Lobby` action in the stage before). The bill no longer hangs on the player. The vote is worth 0.2 to her. A statement alone settles it, and the seat never sees the player's name beside the bill.
- *Offered nothing and argued the cost.* Her lean ends at −0.8: no deal. The debt stands, she is colder, and the vote is still to come.

**What makes it itself.** Nobody is watching, so nothing said costs approval; only what is agreed does. Offers are conditional: nothing is spent unless a deal is struck. The player is not trying to land blows. They are trying to pay in the cheapest coin the other side will take, and the best move was made before they sat down.

### 6.3 A hearing

No arc for this exists yet. It is outlined as the mirror of `STAGED_ARCS.md` §7.3, with the player as the one accused, and needs that arc's conditions (task A5c). The story that starts it exists today: a scandal about the player, answered by a single decision (`About::Scandal`, `ScandalResponseKind`).

**State read.** The *Ledger* has run a story: the player took a consultancy fee (the hidden fact a dilemma leaves, `dilemma.consultancy_fee`), evidence in the open 0.45. The body's other six members hear it before they vote on a motion of censure.

| Read | Value | From |
|---|---|---|
| Audience | Six members, one each. Three start with the player (party colleagues; one owes a favour), three against | holders of the power; opinion, party and favours, as `body_support` counts them |
| The count | Four of six clear the player. Three and three is a warning | `count: majority` |
| The questioner | The member who moved the motion. Holds the story, and one more thing: a note of a meeting (evidence 0.3) | the cast; `MediaState::knowledge` |
| On record | The player told the paper last week they had "never discussed business" (a denial) | `StorySource::Response` |
| Preparation | One member owes the player a favour, and has been reminded of it (it is why three start with them). The player holds the invoice, which shows the work was real | `favours_owed`; a mark from a visit beat |
| The player knows | Four members well; two not at all: their leans show as "unknown" | the ledger |

The screen shows six faces in a row, each with a lean. The player needs four.

**Round 1. The questioner:** the fact. "Did you take { $amount } from { $developer } while the application was before us?"

- *Own it:* cover 0.6. Lands best with the two members of high integrity. It is an admission: the evidence in the open rises, and any case reads that.
- *Deny it:* covers everything if it stands. It unravels 45 times in 100 (the evidence that exists), shown. If it does, the question bites whole, the denial backfires, and the members of high integrity take it worst.
- *Show them the invoice:* stands up by its evidence. It answers the fact.
- *Move on:* "I will not answer that here." Safe with nobody.

The player shows the invoice. Two waverers move a step toward them.

**Round 2. The questioner:** the contradiction. "You told the *Ledger* you had never discussed business. Here is an invoice for discussing business." This prompt exists because the player denied to the paper last week. A player who had said nothing then does not face it.

- *Own it:* "I should not have said that." Standing −1. Retires it.
- *Turn it:* name the developer as the one who pressed the arrangement (a blame, as `LegalResponseKind::BlameAlly` means it today). Lands with the two members who dislike the developer. The developer remembers (`blamed_me`). It backfires if the evidence points the other way.
- *Let your ally speak* (`second`).

The player owns it.

**Round 3. The questioner:** fitness. "Why should this body keep its confidence in you?"

- *Answer with your record:* twelve years of visible work (`Politician::record`). Lands with the loyal and with the player's party.
- *Promise it now:* an ask from the chair, accepted on the spot: declare every outside interest within thirty days. A real commitment with a deadline.

**The result.** Four members end at or over the line: **cleared**, four to two. Had the player denied in round 1 and been caught, it would be two to four: **censured**, a story, a mark the arc's exits read, and the party's Standing to pay.

**What makes it itself.** The player counts heads. Each member is a person with a known or unknown lean and their own reasons, so the same line moves one and loses another. The questions come from one file, and the worst one in it is something the player said themselves. Lying is possible, priced and shown.

### 6.4 A hostile public meeting

This is the flood arc's `meeting` stage (`arcs/flood.jsonc`), where today `dilemma.flood_meeting` offers four options: a date, the wall, honesty, apologies.

**State read.** Nine weeks after the river came through Quarryrow.

| Read | Value | From |
|---|---|---|
| Audience | Quarryrow by income class (three groups), and Halle Talliswell | cohorts; the cast organiser |
| Temper | The run's heat: 0.8. Every gain lands at 1 − 0.5 × 0.8 = 0.6 | `ArcRun::heat` |
| The matter | 41 people still away; 38% of damaged homes repaired; 63 days | the arc's `facts` (`displaced`, `recovered`), `arc_days` |
| The player | Carried boxes at the hall (`at_hall`). Did not pay first (no `paid_first`) | the run's marks |
| The player knows | Quarryrow's worries, in detail | the ledger |

There is no rival. The prompts come from the floor, in Talliswell's voice.

**Round 1. The floor:** the matter. "Nine weeks. Forty-one of us are still not home. When?"

- *Give them a date:* an ask from Talliswell, accepted now: everyone home within sixty days. It answers the prompt. Shown: "A promise. In this room, now, it lands at six tenths." And what breaking it costs.
- *Own it:* "The money is not there yet." Covers most of the bite. **Cools the room** by 0.15.
- *What you heard on these streets:* lands; cools a little.
- *Move on.*

The player owns it. Heat 0.65: gains now land at 0.675.

**Round 2. The floor:** who pays. "The town waited for the region. We waited for the town." This is drawn because `paid_first` is not set. A player who never went to the hall would get "Where were you?" in its place, which bites harder.

- *I will table it:* a rebuild fund (`policy.rebuild_fund`, one of the arc's `answers`), if the player holds the power. A draft exists from tonight.
- *Blame the tier above* (`turn` on `above`): as strong as the region's real silence. The room dislikes it either way.
- *Stay for every question* (`offer`, time): a second slot and some health. Cools the room by 0.2.

The player stays. Heat 0.45: gains land at 0.775.

**Round 3. The floor:** the top worry, recovery. "Rebuilt, or written off?"

- *Give them a date*, now into a cooler room: it lands at 0.775, not 0.6.
- *I will table it:* flood defences (`policy.flood_defenses`): the wall.

The player gives the date.

**The result: split.** The room's heat ends at 0.45 and falls a further 0.10. Talliswell's opinion rises; she remembers the player stayed. The mark `dated` is set, and a promise is on the books with sixty days to run. The arc's edges are unchanged: it goes to *rebuilding* when a law is in force, or when `dated` is set and repairs pass 45%; otherwise, 120 days on, to *hollowed*.

A player who gave the date in round 1 made the same promise to a room that heard six tenths of it.

**What makes it itself.** There is nobody to beat. The room has one temper and the player manages it before they spend anything on it. Order matters more than in any other scene. What ends it is a promise with a date, and the weeks after decide whether it was wise.

### 6.5 Side by side

| | Debate | Negotiation | Hearing | Meeting |
|---|---|---|---|---|
| Who watches | Districts, split | Nobody; two colleagues hear of it | Members, counted | One room, and its organiser |
| The other side | A rival with a record | A creditor with a claim | A questioner with a file | The floor, with the facts |
| The player is trying to | Win the groups they can | Pay in the cheapest coin | Get four of six | Cool the room, then commit |
| The signature move | `turn`, `heard` | `offer` | `deny`, `proof` | `own`, staying |
| What ends it | A mean of leans | Whether a deal stands | A count of heads | The room's heat |
| Public? | Yes: the paper, the vote | No, unless it fails | Yes | Yes |
| On screen | A strip of districts | A table with two sides | A row of faces | One meter of temper |

---

## 7. The player's side of the screen

The scene is shown in the conversation view that exists (`web/src/components/talk/TalkSheet.tsx`; `UI.md` §3): the dialogue box at the bottom, speech bubbles over the speakers in the street or beside a portrait elsewhere. The UI computes nothing. It renders a view and sends a command (rule 4 of `AGENT_CONTEXT.md`).

### 7.1 Getting there

| Mode | How the scene is reached |
|---|---|
| Walking | Go to the place. The other side stands there. Talking to them, or the place's own verb, opens the scene |
| Quick mode and Places | The arc's sheet lists it under "what you can do" with its place, its day and its cost (`ArcView`, task A3). Choosing it sends the move and the start in one request (`Go { location, then }`), as a visit beat does today |
| Today | An item on the days it is open: "The debate is at { $hall }. It closes on { $day }." |

### 7.2 The view

- **The audience strip** sits between the stage and the box. One chip a group, at most five: the group's name; a marker on a five-step scale (two against, even, two for), drawn as a shape at a position, never by colour alone (DEC-051); an arrow when it moved this round; the chip's width in three sizes for the group's weight. A group whose lean the player cannot know shows "?" in place of the marker.
  - `members`: the chips are faces (the portrait component that exists), in a row, with the count beside them: "3 of 6 with you; 4 needed".
  - `table`: no strip. Two short lists: *theirs* (what is asked) and *yours* (what is on the table).
  - With a `temper`: one meter for the room beside the strip, in words as well ("angry", "listening").
- **The prompt** is the other side's bubble. It types out; a tap shows it whole; with reduced motion it is whole at once. A "Why?" control on it opens the chain behind the attack ("You promised six town halls on 3 March…").
- **The options** are rows in the box, one a line of the player's. Each row has two lines:
  1. the label: the kind and the subject ("Answer with your record: the hardship fund");
  2. tags: up to three groups with a direction and a size in steps ("▲▲ Quarryrow", "▼ Millfield", "? Millfield" for a range that crosses zero); a chance in the form odds already take in this box ("75%"); a cost ("promise, 2 of 3"; "second slot").
- **More.** Each row has a second control that opens the detail in the same box, as a topic about something opens its subjects today: every group with its number or range; what the line leaves standing of the prompt; what happens if it fails; the side effects as sentences; whether the other side has an answer. *Back* returns.
- **The locked line** is the last row, muted, with its reason. It cannot be chosen.
- **After a choice:** the player's line goes up in their bubble; the strip moves; one stage direction says how the room took it ("The Quarryrow end of the hall claps."); the other side's answer follows if they have one; *Continue* opens the next round.
- **The result** is a card in the same box: the outcome in a word; a table of groups with lean before, after and change; what it did in the world (approval and recognition by group, each as far as the player may know it; a promise made; who will remember); and "Why?".

### 7.3 Quick mode

One sheet, no stage and no bubbles: a heading, the audience as a list, the prompt as a paragraph, the options as a list with the same tags written out, then the result as text and a table. The choices, the numbers and the commands are the same. A player in quick mode finishes a scene in about eight taps.

### 7.4 Keyboard and screen reader

- **Keyboard.** As the box works today: one Tab stop for the list; arrows, Home and End move; Enter chooses. Added: digits 1 to 5 choose a row; Right arrow opens *More* and Left arrow or Escape leaves it. Escape on the list asks before leaving, because leaving forfeits the rounds left; today Escape leaves a conversation freely.
- **Screen reader.** The prompt is read by the polite live region the box has ("Marek Oldacre: …"), then "Five choices". A row's accessible name is its label and its tags in words: "Answer with your record: the hardship fund. Lands well with Quarryrow. Not known how it lands in Millfield. No cost." The strip is a list ("Quarryrow: leaning to you; moved towards you"). After a choice the live region reads the stage direction and the answer. The result's table is a real table. Focus goes to the list when a round opens and to the result when the scene ends.
- **Reduced motion.** No typewriter, no chip animation: the strip shows the new state and the arrow.

### 7.5 Phone widths

At 360 px the box takes about half the screen during a scene (a third in an ordinary conversation), so five rows of two lines fit without scrolling (56 px a row). The strip is one row 44 px high; names shorten to fit and the full name is in the chip's accessible name and in *More*. The locked line folds under a one-line control ("What you could have said") when height is short. The bubbles page as they do now, about five lines a page.

### 7.6 Length and pacing

| Step | Seconds | Taps |
|---|---|---|
| The scene opens: who is here, how they lean | 10 | 1 |
| A round: read the prompt | 8 | 1 |
| A round: read the options and choose | 15 | 1 |
| A round: the reaction and the answer | 7 | 1 |
| Three rounds | 90 | 9 |
| The result | 15 | 1 |
| **In all** | **about 115** | **11** (8 in quick mode) |

To hold it there: a prompt is at most 140 characters, a label 60, the player's line 120 (the prose lint, by key prefix); a browser test fails if a debate takes more than 12 activations; the doctor reports the median commands a scene.

### 7.7 Afterwards

- The result card's "Why?" opens the panel that exists (`web/src/components/why/`) on the scene's closing event. It reads newest first: the result; round 3, with the line the player chose and whether it stood; round 2; round 1. Each round names the player's act and the earlier event its prompt came from, so the chain runs from "split at the hall" back to "you broke the town-hall promise in March".
- The paper's story carries the same event (`ArticleView.event`).
- The arc's "story so far" changes with the outcome's mark. A promise made is in Today with its date.
- The scene is kept on the run (§11.5), so the arc sheet can show its rounds again while the run lasts.

---

## 8. Skips, stand-ins and bots

### 8.1 Skips

- **A skip stops at an exchange.** DEC-102 clause 7 says every stage of a staged arc stops a skip, and that a skip never makes one of the player's choices. The stop for arc stages is not built: `act_flow.jsonc` lists interrupts for tracks, climax decisions, scandals, elections, sponsored votes and requests due, and `InterruptWhen` (`mandate-content/src/act.rs`) has no arc kind. Task A4 adds it. An exchange opening is a stop of its own, with its day in the text: "If you skip past { $day }, your chair is empty."
- **The skip stand-in cannot play one.** The proxy proposes commands from a fixed list (`proxy.rs`, `Pending`). It gets no entry for an exchange and never sends the command.
- **A player who skips on anyway** gets `absent` when the stage's clock runs out. That is their choice, made with the cost in front of them.
- **This closes a gap that is open today.** The proxy answers decisions of weight `minor` and `significant` (`act_flow.jsonc`, `proxy.answers`), and the debate challenge is `significant`. So today a skip can accept or decline a debate for the player. With the decision gone, it cannot.

### 8.2 Stand-ins

The grader's stand-ins (`STAGED_ARCS.md` §6, task A4, not built) each get a stated policy. They read only the view a player would get, redacted by what that player knows, and send only commands.

| Stand-in | Before the scene | In the scene | What it proves |
|---|---|---|---|
| **Passive** | Nothing | Never begins: `absent` | The absent path, and an exit reached with no act |
| **Engaged** | Every preparation beat the arc lists, for the ally's side | Begins. Each round picks the option whose previewed change is highest: each group's range at its middle, weighed by the group; a line with a chance at its expected value. Ties go to the table's order. Never a line with a shady side effect. A promise only when it beats the next best by `promise_margin`, so it does not spend all three | The won and split paths; that preparation pays |
| **Contrarian** | The preparation of the other side's ally, and none of its own | Begins. Each round picks the line with the widest range or the largest chance of failing. Never `own`; `pass` only if nothing else is listed | The lost and backfire paths; the marks `aired` and `denied` |
| **Attends the stops** | Nothing: it skips to each stop | As engaged | DEC-102's third bar. Engaged minus this one is what preparation is worth in that scene, and the grader prints it |

None of them walks, so every scene one of them finishes is proof that quick mode reaches it (DEC-051).

### 8.3 Bots

The lab's bots (`mandate-cli/src/bots`) play scenes by the same command, knowing only what the player knows (`Knowing`).

| Bot | Policy in a scene | Class |
|---|---|---|
| `idle` | Never begins | probe |
| `pass_only` (new) | Begins; always moves on | rote |
| `first_line` (new) | Begins; always the first line that is not the safe one | rote |
| `mixed` | Begins; a keyed draw among the lines that are not the safe one | blind mix |
| `explorer` and its styles | As the engaged stand-in | informed |

- **The ladder** (DEC-097's balance principle), by mean result: absent < `pass_only` < `mixed` < informed. Report-only until the tuning pass.
- **The report stays meaningful.** A table for each scene kind in the style the research file took from Slay the Spire: how often the scene opened and was played; for each kind of move, how often it was offered, how often picked, and the mean result when picked; the shares of won, split, lost and absent; the mean result by bot; and for each chance shown, how often the line stood (the calibration gate).
- **The audition's forks** play scenes with the same stand-ins. A scene is four commands, so the cost is nothing beside a 39-week run.
- **The pacing test** counts a chapter's turn as "put to the player" when a decision of climax weight is asked (`TUNING_QUEUE.md`, the pacing row). An exchange opened to the player must count too. A debate needs a candidacy, not a post, and a public meeting needs neither, so this also gives a player out of office a turn they can take; that row stands at 2 of 60 today.

---

## 9. Keeping it honest and fresh

### 9.1 What stops one dominant line

The balance principle (DEC-097): the winning way to play must be the engaging one, and no rote routine may be the best.

1. **Lines are earned and spent.** A subject is said once a scene. A kind's second use lands at `repeat`. A player with one law has one `record` line.
2. **The prompt changes what is good.** A line that answers this round's prompt covers its bite; the same line next round changes the subject and covers nothing. The best line is a fact about this round.
3. **Groups differ.** A line lands by how much each group cares. The line that wins Quarryrow loses Millfield. Which trade is right depends on the seat and on where the player stands in each.
4. **Costs are real and outlast the scene.** A promise binds, there are three a campaign, and breaking one costs more than keeping it gains (`asks/promises.jsonc`). `turn` makes an enemy. `own` costs Standing. `act` leaves a bill to carry. `deny` can unravel. `pass` is safe and loses slowly.
5. **The other side adapts within its skill.** It holds answers to the lines a record like this one invites, and a skilled one saves them.
6. **Rounds lean on each other.** Owning a broken promise in round 1 disarms the answer to a promise in round 2 (§6.1). A cooled room hears a date (§6.4). There is no fixed order that is always right.
7. **The lab checks it.** A gate, `exchange_kinds_vary`: over the informed bots' scenes no one kind is the pick in more than 60% of rounds, and the two rote bots finish below the blind mix. It joins the gate ladder, report-only until tuned.

### 9.2 What stops repetition across careers

DEC-100's rule is "recombinable templates first": casting and conditions do the varying. An exchange is that rule applied to a scene. The same debate comes round every campaign, and its content is this career.

Estimates for a debate, to be measured by the doctor once built:

| | A newcomer's first campaign | A second-term player |
|---|---|---|
| Attacks the record gives | 2 to 4 (a thin record, the party's record, the room's worries) | 6 to 15 (0–2 broken promises, 3–8 votes against some group's lean, 0–2 laws that failed, things let pass in this arc, a story) |
| In the other side's deck | 2 to 3 | 3 to 6 |
| Used | 3 | 3 |
| Lines built each round | 3 to 6 | 6 to 20 |
| Lines shown | 4 or 5 | 4 or 5 |

- **Distinct prompts.** Five of the crowd sources take a worry as their subject (a promise about it, a vote on it, a law for it, the government's record on it, the room's question about it), and the base pack has six worries (`content/base/concerns.jsonc`). With three wordings each, fifteen written sentences read as ninety different prompts before a name or a number is filled in. The other sources take a thing with its own name: this story, this fact, this arc's missed beat.
- **Voices.** The other side's closers come from the voice registry conversations use (`talk/reply.rs`, `voice`), so a blunt rival and a smooth one put the same attack differently, with no new lines.
- **The measure.** DEC-100's gate counts the share of a career's content seen in the one before. For scenes the unit is the pair (source, subject). Target: under 40% of a third career's pairs seen in its second, reported with the gate.

### 9.3 What must be written

Sentences, by kind of scene. A prompt is written three ways; a line of the player's two ways.

| Text | Debate | Negotiation | Hearing | Meeting | Shared |
|---|---|---|---|---|---|
| Prompts (sources × 3) | 9 × 3 = 27 | 4 × 3 = 12 | 5 × 3 = 15 | 5 × 3 = 15 | |
| The player's lines (kinds × 2) | 8 × 2 = 16 | 6 × 2 = 12 | 7 × 2 = 14 | 8 × 2 = 16 | labels: 11 |
| The other side's answers | 12 | 10 | 12 | 10 | |
| How the room takes it (stage directions) | | | | | 30 |
| Openings, the chair, closings | 6 | 4 | 6 | 4 | |
| Results (4 outcomes × story so far, headline, aftermath) | 12 | 12 | 12 | 12 | |
| Tags, costs, risks, locked reasons, the screen's own words | | | | | 40 |
| **Total** | **73** | **50** | **59** | **57** | **81** |

About 320 in all; the debate and the shared set (154) come first. A new scene that reuses a layout and its sources costs about 20: its results, its openings, and three for each source it adds. The style bible needs a register for `exchange-*` keys (task A9's kind of work): a prompt is one or two spoken sentences with a fact in them; a line of the player's is first person and short.

### 9.4 Lints

In content validation (static; where A1 put the arcs' static checks, `mandate-content/src/arcs.rs`):

| Lint | Flags |
|---|---|
| `arc-turn-single-decision` | A stage marked `climax` whose only beat a player can take is one `decision`, with no visit, talk, ask or exchange in it. Today this is the rival's `showdown`, the favour's `called_in`, the plant's `organising` and the flood's `meeting`. A warning until those are rewritten (§10), then an error |
| `exchange-no-safe-move` | A scene whose `moves` lack `pass` |
| `exchange-move-no-requirement` | A kind the arc can never give: `second` with no beat that sets its mark; `promise` in an arc no campaign raises, with no ask named; `act` in an arc with no `answers`; `proof` with nothing that teaches or marks. In code, a test that every kind but `pass` declares a requirement |
| `exchange-audience-uncast` | A source that cannot bind in a tier the arc lists: members by a power no post of that tier has; `of: race` in an arc a campaign does not raise; a person role the arc lacks, or an optional one with no other group |
| `exchange-no-fallback` | A round whose sources can all be empty and that does not end in `top_worry` or `the_matter` |
| `exchange-no-clock` | A scene in a stage that does not end on a clock: `absent` needs a day |
| `exchange-private-secret` | A scene with a `table` that lets `turn` read hidden facts |
| the arcs' own | An outcome's mark no edge or gate reads (`arc-mark-unread`); a `prep` mark nothing sets (`arc-mark-unset`); a role not cast; an office named; a key or argument missing; the prose lint by prefix |

In the doctor (dynamic, with `--seeds`; task A4's `doctor/arcs.rs`):

| Lint | Flags |
|---|---|
| `exchange-audience-thin` | A crowd scene bound to fewer than two groups in over a fifth of worlds |
| `exchange-nothing-to-say` | All three rounds fell back to the question in over a third of engaged runs |
| `exchange-kind-dominant` | One kind is the engaged stand-in's pick in over 60% of rounds |
| `exchange-outcome-unvisited`, `-lopsided` | An outcome no stand-in reached; one outcome in over 70% of engaged runs |
| `exchange-prep-idle` | The engaged stand-in does no better than the one that only attends the stops |
| `exchange-too-long` | The median scene takes more commands, or more characters of text, than §7.6 allows |

---

## 10. The three arcs rewritten to the standard

Outlines, not content. Stage ids that exist are kept, so a run in a save continues (§11.5). Beats without an id shown are new.

### 10.1 The rival

Today (`arcs/rival.jsonc`): *shadow* → *offer* (a decision) → *pact* → *showdown* (a decision: accept or decline) → the voters. One role, the rival. New roles: a hall, a chair, a backer and an ally, cast as the flood arc casts a place and a person near an area.

| Stage | Beats, by kind | Leaves when |
|---|---|---|
| *shadow* (kept) | **heard:** the rival has been asking about the player. **press:** the rival's first statement. **talk** (ally): "who is with them?", which teaches the rival to `known` | Five days on → *offer*; the race is decided or called off → an exit |
| *offer* (kept) | **decision:** the truce (`dilemma.rival_offer`, as now). **visit** (the paper's office): read the rival's cuttings, which teaches them to `detailed` | The truce taken → *pact*; the vote within 45 days → *lead_up* |
| *pact* (kept) | The truce holds: it bars the personal sources in the debate | The vote within 45 days → *lead_up* |
| *lead_up* (new) | **move** (rival): the challenge is issued, in the paper and in Today. **visit** (each area of the seat, repeatable): walk it with someone who lives there, which teaches its worries. **talk** (ally): rehearse; sets `rehearsed`, teaches the arc to `detailed`. **talk** (backer): ask them to speak; sets `endorsed` if they agree. **ask** (a resident): raise their street's matter at the hall, kept by a `heard` line for their area | The vote within 21 days → *showdown* |
| *showdown* (kept; the turn) | **exchange:** the debate (§2.3). **press:** the paper's account | The race decided → *won* or *lost*; called off → *cooled* |

**Exits** (kept: *won*, *lost*, *cooled*). The voters still end it. The gates read the debate: after a win in the race, `debate_won` adds to the record; `debate_ducked` takes from it; `aired` leaves the rival an enemy with a memory of it; after a loss, `debate_won` softens the fall in Standing. The debate's own effects (approval and recognition by area, the story) fell a week before the vote, so the vote has already read them.

**Passive.** No walk, no rehearsal, an empty chair. Three attacks land unanswered and cold in every area of the seat, the paper prints the chair, and the race is run from there.
**Engaged.** The player has walked two areas, read the cuttings, rehearsed and found a backer. They hold five kinds of line, see exact numbers and are not caught cold. Whether that wins the seat depends on the seat, and the arc says which lines were theirs.

### 10.2 The favour debt

Today (`arcs/favour_debt.jsonc`): *debt* → *reminder* → *called_in* (a decision: pay, refuse, dodge) → *repaid*, *burned* or *settled*.

**The bind is taken from live state.** A favour called in is a turning point only when paying costs something. As the creditor's bill comes up, the arc reads what the vote would cost the player, and its `given` asks for at least one:

| Bind | Read from |
|---|---|
| The seat leans against the bill | `policy::public_support` over the seat's neighbourhoods (the reading `PartyLineAgainstWard` uses) |
| The party's line is against it | `politics::party_line::player_stance` |
| An open promise forbids it | a promise whose need is "against all" bills of a tag the bill has (`asks/promises.jsonc`) |
| Someone else the player owes is against it | `favours_owed`, the members' leans |

With no bind the favour is paid by an ordinary vote and the run ends at *repaid* with no turn. A matter too small to tell raises nothing.

| Stage | Beats, by kind | Leaves when |
|---|---|---|
| *debt* (kept) | | 45 days on → *reminder* |
| *reminder* (kept) | **talk** (creditor): "what will you want for it?", which teaches their bill and their goals to `detailed` if they trust the player | The creditor has a bill before the body (`sponsors_bill`) → *bind* |
| *bind* (new) | Clock: the bill's own vote. **The ways out are existing actions:** lobby or trade for one more vote, so the bill no longer hangs on the player (`Lobby`, `TradeFavor`, the pitch topic); **talk** (creditor): ask for a sunset or a change, which moves the seat's lean (the `rider` effect); make a statement for the bill (`Press`) | A week before the vote → *called_in* |
| *called_in* (kept; the turn) | **exchange:** the table (§2.4, §6.2), at the creditor's workplace. Then the vote itself, which the player casts with the command that exists | The vote declared, or the bill decided → an exit |

**Exits.** *repaid* (kept): the debt is cleared, by the vote or in the coin agreed. *deferred* (new): part paid; the debt stands and the arc returns with the creditor's next bill. *burned* (kept): refused or dodged. *settled* (kept): the call never came; reachable with no act.

**What other members learn.** The exit's effects reach the `watchers`: members who owe the player or are owed by them, and members who think well of the creditor. A debt paid raises their opinion of the player a little; a debt burned lowers it more. Lobbying already reads a colleague's opinion of the player (`npc::lobby_odds_between`), and an ask for support reads opinion and trust (`talk::support_odds`), so how the player treats one debt changes what the next favour costs. This needs an opinion effect with a group of holders (the party row's design has the same need; not built).

**Passive.** The player never talks to the creditor, the bill hangs on them, they stay away from the table and the vote takes its default. The creditor counts a promise broken, and two colleagues hear of it.
**Engaged.** The player finds a fourth vote, or wins a sunset, before sitting down. At the table they pay with a statement, or with a vote the seat can live with. The debt is cleared and the colleagues have seen it paid.

### 10.3 The plant closure

Today (`arcs/plant_closure.jsonc`): raised by the crisis indicator once jobs are already going; *layoffs* (a decision) → *walkout* → *organising* (a decision) → three exits. `STAGED_ARCS.md` §7.1 designed its replacement on a review with a date. DEC-110 has since fixed how plants fail: short time, a notice with a date, then a reprieve or the closure, each an event on its own thread, with `economy::plant_facts` as the arc's one query and `plant_support` as the rescue's arithmetic (`INDUSTRIES.md` §3, §5, §7). None of it is in the code yet. The outline below is §7.1 on those events.

| Stage | Raised or entered on | Beats, by kind | Leaves when |
|---|---|---|---|
| *short_time* (signs; hidden) | the plant goes on short time | **heard:** the overtime has stopped. **visit** (the works): tour it. **talk** (manager): what would head office need? Which gives the gap, in detail, to someone trusted | The notice is served → *notice*; orders come back → *scare* |
| *notice* (the break) | the notice, with its date; the clock is the plant's | **press.** **visit** (the gate). **decision:** where the player stands (`dilemma.plant_layoffs`, kept as a beat, no longer the turn). **ask** (steward): the picket. **move** (steward): a ballot | A table is agreed → *talks*; the ballot carries → *walkout*; 60 days to the date → *table* |
| *walkout* | | As §7.1: the line stops; the manager reports it | → *talks* or *table* |
| *talks* (levers) | | The rescue bill, drafted with the command that exists for a family's proposal, its support shown as arithmetic ("needs +0.06; this bill gives +0.04"). **talk** (steward): short time. **talk** (manager): more time | 30 days to the date → *table* |
| *table* (the turn) | | **exchange:** a negotiation at the works. Across the table, the manager for the parent firm; watching, the steward and the members' area. On the table: the bill's relief, short time the steward will put to the members, a date | The plant is reprieved → *reprieved*; it closes → *aftermath* |
| *aftermath* (a second turn, on the closure path) | the closure | **exchange:** a hostile meeting at the hall (§2.4): the workforce, the steward for the floor, heat as temper. What can still be said is real: a fund for those laid off, the site | 60 days on → an exit |

**Exits.** *reprieved*: the plant is off the list; the gates read whether the player's bill or the steward's concession was on the table. *scare*: orders came back before any notice; reachable with no act. *answered*: closed, with something in force for those laid off and the meeting not lost. *closed*: closed, and nothing; the buyer's arc may follow.

**Passive.** Nobody asks the manager anything. The notice comes, the ballot carries, the date moves up, nobody sits at the table and the plant closes. The player sends apologies to the hall, and the neighbourhood remembers that.
**Engaged.** The player learns the gap in the *short_time* stage, weeks before anyone else. They stand at the gate, bring both sides to the table with a bill that covers part of the gap and a steward willing to carry short time, and the economy's own rule may then find the plant over its line. In a town with a deeper slump the same play still ends in a closure, and the meeting afterwards goes very differently for someone who was at the gate.

**Until DEC-110 is built** the arc as it stands can take one step to the standard: its turn, *organising*, becomes the hall meeting in place of `dilemma.workers_campaign`.

---

## 11. Architecture

### 11.1 Who owns it

**The arcs domain.** A scene is a beat of a run. Its state is on the run, its step is the arcs' own code, and everything it changes elsewhere is an effect request (OVERVIEW §4.3). It is not a talk hook, though it is shown in the conversation view. A hook's contract is one reply chosen by conditions, half a slot each time, one logged conversation each time, and "nothing here draws a random number" (`talk/hooks.rs`). A scene needs a preview for each option, an audience, three rounds for one cost and a keyed draw.

```rust
// mandate-sim/src/arcs/exchange/: state, audience, attacks, moves, landing, play, view
pub struct ArcExchangeState { pub beat: BeatRef, pub on: SimDate, pub groups: Vec<GroupLean>, pub round: u8,
    pub prompt: Option<Attack>, pub deck: Vec<Attack>, pub answers_left: u8, pub retired: Vec<AttackKey>,
    pub used: Vec<String>, pub table: Vec<String>, pub rounds: Vec<RoundPlayed> }
pub struct GroupLean { pub group: Group, pub weight: f32, pub start: f32, pub lean: f32 }
pub enum Group { Area { area: Area, class: Option<u8>, community: Option<u16> }, Member { id: CharacterId }, Person { id: CharacterId } }
pub struct Attack { pub source: SourceKind, pub subject: SubjectRef, pub size: f32, pub anchor: Option<EventId> }
pub struct RoundPlayed { pub prompt: Attack, pub option: String, pub stood: bool, pub answered: bool, pub deltas: Vec<f32>, pub event: EventId }
// On ArcRun, both with serde defaults and not written when empty:
//   exchange: Option<ArcExchangeState>     a scene begun and not finished (never across a turn)
//   played:   Vec<ExchangePlayed>          finished scenes: the beat, the outcome, the groups before and after, the rounds
```

No `HashMap`, typed ids, ordered vectors (DEC-041). A finished scene is about 0.6 KB of JSON, and a run has one or two.

### 11.2 Commands

```rust
PlayerCommand::Exchange { run: u32, beat: String, act: ExchangeAct }
pub enum ExchangeAct { Begin, Say { option: String }, Leave }
```

- `Begin` is a located act, checked like a visit beat (`places::check_at`), and spends the scene's time. Quick mode sends it inside `Go`.
- `Say` names an option by a stable id (`record:law.17`, `heard:hood.4`, `promise:promise.no_tax_rise`, `pass`), never by its place in the list. It must be one the view lists.
- `Leave`, or ending the turn with a scene open, forfeits the rounds left.
- A new `PlayerAct::Exchange` on the spine. `mandate-wasm/tests/why.rs` fails until it has its line of text.
- `arcs::beats::BeatAction` gains `Exchange`, so the arc's list of offers carries it.

### 11.3 Views

`QueryDto::ArcExchange { run, beat }` returns `ArcExchangeView`: whether it is ahead, open, under way or done; the place, the cost and the last day; the layout; the sides; the groups (name arguments, weight, lean as a number, a band or unknown); the round; the prompt (speaker, key, arguments, the event behind it); the options (id, kind, label and line keys, arguments, the change for each group as a range, the chance and the changes on failure, costs and side effects as keys, what is known of the other side's answer); the locked line; the rounds played; the result. Facts and Fluent keys only (DEC-054).

Redaction is in `mandate-protocol`, where the ledger's redaction lives. The engine hands over, for each size, the exact value, the two ends it could take, and the ledger subject it turns on; the protocol shows the value or the ends by what the player knows. The UI never sees a number it must not show.

### 11.4 Effect requests

Every change outside the arcs' own state, with its owner. The cause of each is the round's event (`Cause::caused_by`), as a visit beat's requests name the beat's event today (`arcs/beats.rs`, `effects::apply_now`).

| Change | Request | Owner | Today |
|---|---|---|---|
| Approval in a group | `Effect::Approval { scope, delta, classes }` | population | exists |
| …by community | the same, with a community filter | population | new field |
| Recognition in a group | `Effect::Awareness { scope, delta }` | population | new. The narrative applies a dilemma's `awareness` in its own resolver (`narrative/resolve.rs`); arcs cannot move it |
| A member's or a person's opinion; a memory | `Effect::Opinion`, `Effect::Remember` | politics | exist. A group of holders (the watchers) is new |
| A favour owed | `Effect::Favour` | politics | exists |
| A favour repaid | `Effect::Repay` | politics | new as a request (a dilemma's `repay` exists) |
| Standing, money | `Effect::PlayerTrack` | player | exists |
| Time | spent as a visit beat spends a slot | player | exists |
| Health (staying late) | none | player | no request: a dilemma's `health` is applied by the narrative. Staying costs time only until there is one |
| A campaign promise | the asks' own `promise`, as the command calls it | asks | exists |
| A promise outside a campaign | `Effect::OfferAsk`, accepted in the same step | asks | `OfferAsk` exists for an arc's ask beat; accepting at once is new |
| A declared vote; a rider | the paths the `CastVote` command and a dilemma's `rider` take | politics, policy | exist as paths; new as arc effects |
| A draft | the path a dilemma's `draft` takes | policy | the same |
| A hidden fact aired | `Effect::Surface { hidden, by }` | media, which opens the story by its own rules as `media::leak` does | new |
| The story | none. The media read the scene's closing event from the log by their cursor, as town news does, and post `Effect::Perceive` themselves | media | a new `StorySource` |
| What the player learns | `Effect::Learn` | player | exists |
| Marks, heat, the scene | none | arcs | own domain |

### 11.5 Saved state and the cheap migration

- **Save v29.** `ArcRun::exchange` and `ArcRun::played` take serde defaults and are not written when empty. So the migration step from v28 is empty, and a game with no scene hashes as it did (the device A1 used for `ArcsState`). The state-shape pin is re-recorded on purpose (`scripts/repin.sh`).
- **Content that changes under a running arc.** Stage ids are kept, and a run names its stage by id, so most runs carry on. One case needs care: a save in which the debate's or the favour's decision is waiting on the player. Its beat is now an exchange. The cheap rule: on load, a waiting decision whose beat is no longer a decision is withdrawn with no effect, and the scene is on offer for what is left of the stage. If that proves more than a few lines, the in-flight decision is dropped and the report says so (rule 9 of `AGENT_CONTEXT.md`).
- **A template that goes** (the plant's, when DEC-110's arc replaces it): the rule that exists ends its runs as `removed` (`arcs::EXIT_REMOVED`).

### 11.6 On the spine

- **Events.** `ArcExchange { run, beat, round, source, option, stood }` for each round, and `ArcExchangeEnded { run, beat, outcome }`. Each notes the run's thread (`Thread::Arc`), so the log links it to the run's latest event. A round's own causes are the player's act and, where the attack has an anchor the log still keeps, that event with the link `Prompted`. The closing event is notable; the rounds are kept with it because chains stay whole (`CAUSAL_SPINE.md` §5.4).
- **Absent.** The closing event's causes are the arcs' process and the player letting it pass (`PlayerAct::LetPass`).
- **Nothing reads a cause.** The spine's proof still holds: with no exchange in the content, every pinned digest is unchanged (`lab/proof.rs`); the "nothing significant is uncaused" games cover the two new kinds.

### 11.7 Tier-generic (DEC-098)

- A scene is on a run, and a run is keyed by `Jurisdiction`.
- The audience is bound by source at that jurisdiction: areas one level down, members by a power id, people by cast. Nothing reads `player.municipality` or `player.ward`. "The seat" is the constituency of the race or the post, asked for as `asks/commands.rs` asks (`my_race`).
- No sentence names an office or a body. A person's role word comes from the constitution as an argument (`$other_role`).
- Scale lives in content: the constants take per-tier overrides, and `max` caps the groups whatever the tier gives.

### 11.8 Content and code

| Content (JSONC, validated against schemas from the Rust types) | Code |
|---|---|
| Which scenes exist: the `exchange` beat on a stage | The scene's step: begin, a round, the end, absence |
| The audience's sources, sizes and starting reading | Each source's binding to state |
| Which sources may prompt each round, in what order | Each attack source: what it reads and how it sizes an attack |
| Which kinds of move are allowed | Each kind: its requirement, its strength, its side effects |
| The features: `temper`, `count`, `table` | The landing formula; the choice of lines; the deck |
| Every constant; the weights of character on the deck; `member_fit` | The view and its ranges |
| What the outcomes do to the run; what `prep` marks do | The effect requests |
| All text | Nothing: keys and arguments only |

A new scene, or a new arrangement of an old one, is data. A new source or a new kind of move reads a new quantity, so it is code, with its own test and its own line in the lints.

### 11.9 What in today's code will fight this

1. **The names are taken.** `Exchange`, `ExchangeId` and `ExchangeView` are a logged conversation; `SceneDef` is a decision's spoken beats. Hence `ArcExchange…`.
2. **The conversation view is built on `TalkView`:** one person, their topics, one reply. It needs a second mode and an audience strip (`TalkSheet.tsx` is about 640 lines around that model).
3. **Arcs can move approval and not recognition,** and approval only by area and income class. `ArcEffect` is a short list (`mandate-content/src/arcs.rs`): no vote, draft, promise, hidden fact or learning.
4. **Promises exist only in a campaign.** `promise_menu` is empty otherwise and needs a race (`asks/commands.rs`).
5. **Only the player has approval by cohort.** An opponent has one `record` number, a scandal valence and declared votes. So a lean is the player's standing in a group, and "their record" is thinner than the player's.
6. **The proxy answers `significant` decisions** and no interrupt stops at an arc's stage: DEC-102 clause 7 is not built.
7. **A3, A4, A5 and E9 are not built.** The arc view, the stand-ins and grader, the plant's fuse and the digging of secrets are all needed by parts of this: the screen's entry point, §8, §10.3 and the dug-up `turn`.
8. **Old reads of the player's ward.** `PlayerState::ward` and `narrative::cast::View::ward_lean` read it directly. The audience must not.
9. **The director and the pacing test count decisions.** A turn that is an exchange must count as put to the player.
10. **The lean of a vote is today's.** `public_support` is computed from attitudes now; nothing stores how a group leaned on the day.
11. **The rival is a town's** (`arcs/rival.jsonc`: "until rivals are kept for a region"), so the debate is `tiers: ["local"]` for now whatever the scene allows.

---

## 12. Implementation plan

Four tasks, bundled so that four agents do everything, two of them agents the arcs plan already needs.

| # | Task | Tier | OWNS | After | Acceptance |
|---|---|---|---|---|---|
| X1 | The exchange core, and the debate as its first scene | large | `mandate-content/src/arcs.rs`, a new `exchange.rs`; `mandate-sim/src/arcs/exchange/**`, `arcs/{mod,beats,step,effects}.rs`; `actions/mod.rs` (the command), `spine/mod.rs` (the act), `events.rs`; `scheduler.rs` and `population/mod.rs` (recognition, the community filter); `media.rs` (the story source, `Surface`); `asks/commands.rs` (accept at once); `mandate-save/**` (v29); `content/base/exchange.jsonc`, `arcs/rival.jsonc`, `locale/en/exchange.ftl` (the debate and the shared set); schemas | A2 (done). Beside A3 and A4 | One hash twice, and through a save and load in round 2. A fork equals continuing. Tests 1 to 3 of §5.7. The same commands under a ledger that knows everything and one that knows the default give the same world. With no exchange in the content every pin of the spine's proof holds. Scripted passive and engaged command lists end at `debate_ducked` and not. Each static lint of §9.4 has a failing fixture. A scene is four commands |
| X2 | The view and the screen (with A3, as one agent) | mid | `mandate-protocol/src/arcs.rs` (A3's) and the query; `web/src/components/talk/**` (the scene's mode, the strip); a new `web/src/components/exchange/**` (the quick list, the result card); `web/e2e/exchange.spec.ts`; the screen's Fluent | X1's types agreed first, then in parallel | One browser test plays a debate from quick mode by keyboard alone in 12 activations or fewer. At 360 px five options show without scrolling. A group the player does not know shows a range, and the result is inside it. The accessibility check passes. "Why?" on the result shows three rounds |
| X3 | Stand-ins, bots, the grader's terms, the skip stop (inside A4, as one agent) | mid | `mandate-sim/src/standin.rs`, `proxy.rs`, `skip.rs`; `mandate-content/src/act.rs`, `content/base/act_flow.jsonc`; `mandate-cli/src/bots/**`, `arcs_grade.rs`, `doctor/arcs.rs`; the pacing test | X1 | Stand-ins and bots send only commands. A skip stops at an exchange in 8 of 8 seeds, and the proxy never sends the command. The report table of §8.3. Each dynamic lint has a fixture. The gates are printed with their figures and not enforced |
| X4 | The arcs to the standard (inside A6, the content agent) | mid | `content/base/arcs/**`, `dilemmas.jsonc` (three decisions retired), `asks/asks.jsonc`, `locale/en/{arcs,exchange}.ftl`, `STYLE_BIBLE.md` (the `exchange-*` register) | X1 to X3. The table needs nothing more. The hearing needs A5c. The plant needs DEC-110's plants and A5a | The grader's gates for each arc. `arc-turn-single-decision` as an error with no findings. The content pipeline's review |

**Before A6:** X1, X2 and X3. The debate needs only what is built: a campaign, the promise menu, laws' ledgers, declared votes, worries, the ledger. It is to the exchange what the flood from *struck* was to arcs: the first playable one.
**Inside A6:** the rival's preparation beats beyond the three X1 ships; the favour's bind and table; the flood's meeting; the plant's two scenes; the hearing and its arc; all text past the debate.

**Placeholders for the tuning pass.** Mechanisms land with correctness tests; numbers are tuned once at the end of M6 (rule 11). Every constant of §5.6 is a placeholder, and these most of all:

| Group | Constants | What the pass sets them against |
|---|---|---|
| The size of a scene in the world | `approve`, `aware`, and the story's weight | A won debate should move a close race and not a safe one; an empty chair should cost about what declining costs today |
| The thresholds | `win`, `lose`, `group_margin`, `member_line`, `walk_away` | Engaged stand-ins: no outcome over 70% |
| The balance of kinds | `cover.*`, `gain`, `bite`, `repeat`, `spill`, `backfire.*`, `hold.*`, `public_share`, `voted_share` | `exchange_kinds_vary`; the rote bots below the blind mix |
| Strength scales | each worry's `full` amount; `heard_floor`, `visits_full`, `second_base`, `act_base`, `stay_strength` | A typical good law gives a strength near 0.6 |
| The other side | `deck_base`, `deck_per_skill`, `answers_*`, `pick_top`, `answer.*`, the weights of character | Difficulty presets (DEC-102 clause 3) |
| Preparation | `cold`, `promise_margin` | `exchange-prep-idle`: engaged beats attends-the-stops by a margin a player can feel |
| The room | `temper.drag`, the cooling of `own` and of staying | The meeting's order matters, and no order is always right |

X1 is expected to move first-election results, since the proxy and the idle bot now meet an empty chair. Under rule 11 any gate this breaks (the ladder, `proxy_not_optimal`, a sweep band) goes report-only with a dated comment and its figures before and after, and is listed for `TUNING_QUEUE.md`.

---

## 13. Open questions for the owner

1. **Does the debate replace the yes or no?** Recommend yes: turning up is accepting, staying away is declining, and the empty chair is its cost. *If yes:* one decision template fewer in campaigns (ENGAGEMENT counts `debate_challenge` among four campaign-only ones), and a skip can no longer answer the challenge for the player. *If no:* a pop-up stays in front of the scene, and a player who declines never sees it.
2. **May a secret be aired in public?** Recommend yes in a debate or a hearing, and no across a private table. In public it is exposure, with the media's rules: it becomes a story, the person remembers, and on weak knowledge it can fail and come back on the player. In private it is pressure, which DEC-097 (answer 4) keeps out of M6. *If no everywhere:* `turn` reads only the public record, and what E9 lets a player find has no stage. *If yes everywhere:* the game has blackmail before it has a design for it.
3. **How much is luck, and how much the character's voice?** Recommend chance only where a claim may not stand up (`turn` on a secret, `proof`, `deny`), always shown, and a mild effect of oratory on every gain. Uncertainty then comes mostly from what the player has not learned, which preparation removes. *More dice* (a delivery roll on every line): scenes swing more and are less solvable, the calibration gate has more to hold, and a prepared player can lose to a roll. *None at all:* a scene is a sum the player can work out, and risky lines are not risky.
4. **How often?** Recommend only at a turn, and at most one a chapter: the lead arc's. A background arc's turn must still be more than one decision, but need not be a scene. That is about five to eight scenes a career. *More:* more play for the text written, and the form wears. *Fewer* (debates only): the favour and the meeting keep their pop-ups.
5. **May a player out of office take part?** Recommend yes wherever the fiction allows: a debate needs a candidacy, a public meeting needs nothing, a hearing can call a former member. It gives a turn to the chapters that today reach one the player cannot take (2 of 60 out of office, `TUNING_QUEUE.md`). *Cost:* lines that do not assume a seat, and `act` is not on offer without a power. *If no:* out of office stays a time for watching.
6. **Which scenes ship in M6?** Recommend the debate first (X1), then the favour's table and the flood's meeting in A6; the plant's two scenes when DEC-110's plants are in; the hearing with its arc after A5c, which may fall after the tester checkpoint. *Cost of all four in M6:* about 320 sentences and two more engine tasks before the checkpoint. *Cost of the debate alone:* testers meet the form once and cannot tell whether it wears.

---

## 14. Decision proposal

*(Accepted by the owner on 2026-11-25 with all six recommendations of §13: the decision of record is `docs/design/decisions/DEC-112.md`.)*

## DEC-112 — Exchange scenes: an arc's turn is played

- **Status:** Proposed (2026-11-25). The owner asked of the arcs: "Being challenged to a debate do we just click the yes button and it happens? Or for the favor do we just say yes or no? We will want our storylines to be engaging for the player", and agreed the direction ("Yes let's do it").
- **Context:**
  - DEC-109 gave arcs stages, beats and exits. The flood arc uses them. Four turns do not: the rival's, the favour's, the plant's and the flood's own meeting are each one decision.
  - The rival's turn has two options and one line of aftermath. A skip can answer it for the player.
  - The game already keeps what a contest of words would read: laws' ledgers, promises, declared votes, worries by place, what the player knows, opinions, favours, an arc's heat.
- **Decision:**
  1. **A rule for every arc.** The turn is something the player does with the game's own verbs, and what they know and whom they know changes their options. A turn is never a single decision. Content validation flags a turn whose only beat is one.
  2. **One new beat kind, the exchange.** A scene of three rounds in the conversation view, about two minutes, only at a turn. Design: `docs/design/EXCHANGE_SCENES.md`.
  3. **Prompts are true.** The other side's attacks are drawn from the player's record through its owners' state, anchored on the causal spine, and limited to what that person can know. An attack the log has forgotten is gone. A clean record is asked a question, never charged with something invented.
  4. **Lines are earned.** Each kind of move has a requirement in real state (a law, a ward's worries learned, a fact found, a power, a promise still to give, a backer) and a strength computed from real quantities. One safe move is always there. Preparation in the stages before adds lines and removes risks.
  5. **The audience is live state.** Groups are bound from the jurisdiction (its areas, classes, communities, a workforce, the members of a body by a power, a cast person), never named in code or content (DEC-098).
  6. **Shown = rolled.** One function computes what a line does; the preview and the command both call it. A chance, a cost and a side effect are always shown exactly. A size that turns on something the player has not learned is shown as the range it could take, and the result is always inside it (DEC-073, DEC-091).
  7. **The other side plays by the player's rules** (DEC-030): the same formula, the same chance on a weak claim, with skill as better information and more answers, and character as what it reaches for. Every draw is keyed (DEC-041).
  8. **Beats write; transitions read.** A scene ends won, split, lost or absent. It moves approval and recognition by group, opinions and memories, makes real promises and drafts, gives the paper its story and sets the arc's marks, all through the owners' effect requests. It never picks an exit.
  9. **One mechanism, four scenes.** A debate, a negotiation, a hearing and a hostile meeting are data over the same core, told apart by their audience, their sources and three optional features (a room's temper, a head count, a table of conditional offers).
  10. **Skips and stand-ins.** A skip stops at a scene and the skip stand-in cannot play one (DEC-102 clause 7). The grader's stand-ins and the lab's bots play by stated policies, sending only commands.
  11. **It belongs to the arcs domain.** One command, one view, state on the run, save v29 by defaults. Engine never produces prose (DEC-054); every scene is reachable in quick mode (DEC-051).
- **Order:** the core with the debate (X1); the view and screen with the arc view (X2, A3); the stand-ins, bots and skip stop with the grader (X3, A4); the rival, the favour, the flood and the plant rewritten in the arc content task (X4, A6). Numbers are placeholders until the tuning pass at the end of M6.
- **Dependencies:** DEC-030, DEC-033, DEC-041, DEC-051, DEC-054, DEC-073, DEC-091, DEC-093, DEC-097, DEC-098, DEC-100, DEC-102, DEC-109, DEC-110.
