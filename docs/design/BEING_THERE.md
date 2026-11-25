# Being there: the town is lived in

**Status:** accepted (DEC-114, §12), 2026-11-25. **B1 is built** (the calendar, one date per turn, regulars and familiarity; §10.4 says what was built and where it departs from this note); the rest is not. It extends DEC-093 (conversations), DEC-097 (engagement), DEC-099 (works you can see), DEC-109 (staged arcs) and DEC-112 (exchange scenes), and leans on the pocket of DEC-113.

The owner's question: "Are there other things like talking to councilors for lobbying in the general game that makes it feel more interactive than a menu based diplomacy simulator? I want the users to feel connected to their area like with Stardew Valley but with a civic focus and the extension we get from later acts." Lobbying is already decided: it becomes a conversation (M6_CHECKLIST note 29). The owner agreed to a design pass for the rest ("Yes do the design pass").

**The idea in one line.** The town is a place with a rhythm (a calendar), people who are always somewhere (routines) and remember the player's face (familiarity), houses with families behind the doors, and streets that show what the player fixed or let rot. Every agenda verb happens to someone, somewhere. Nothing new is a minigame: each verb reuses a conversation (DEC-093), a talk hook (S4) or the exchange core (DEC-112), so it costs content, and little new system.

**Why Stardew is the right reference, and where it stops.** Stardew Valley's town works because of a few cheap things: a fixed weekly rhythm, people at known places on known days, a calendar of festivals, small personal stories that unlock with friendship, and changes the player made that stay visible. All of these exist in this game in part (below). What this game must not copy: gift-giving as a friendship meter, or a checklist of hearts to fill. DEC-091's principle of 2026-11-25 (knowledge is not a checklist) applies to people too.

## 0. What exists to build on

| Thing | State today | Where |
|---|---|---|
| Routines: who is where, by part of day, weekday, season and event | built; derived, never stored | `content/base/routines.jsonc`, `crates/mandate-sim/src/characters/routines.rs` |
| Routine events | four only: `session`, `vote`, `campaign`, `meeting_night` | `routines.rs::holds` |
| A town's meeting night | one weekday, keyed by the seed and the **municipality** | `routines.rs::meeting_night` (KEY_MEETING) |
| Hints of a calendar in routines | farmers at the market on Saturdays, clergy on Sundays, teachers off in summer | `routines.jsonc` (`routine.farmer`, `routine.clergy`, `routine.teacher`) |
| Where someone was last seen | four sightings per person | `player/knowledge.rs` (`Sighting`, `SIGHTINGS_KEPT`) |
| The turn | a fortnight (7 days in a campaign, 3 in the final stretch) played as one representative day of three slots and a late one | `tuning.jsonc` (`turn_days`, `standard_slots`), `routines.rs::representative_day` |
| Agenda verbs | 13 kinds of `Action`; costs in slots | `actions/mod.rs`, `tuning.jsonc` (`action_costs`), `places.jsonc` |
| Visit verbs at places, as data | nine verbs with time, parts, effects and risk | `actions.jsonc` (`visit.verbs`), `visits.rs` |
| A system's own choice in a conversation | built (talk hooks, S4) | `talk/hooks.rs` |
| The exchange core and the debate | built, engine side (X1) | `arcs/exchange/`, `EXCHANGE_SCENES.md` §15 |
| Casework | an ask with text by occupation (a drain, a form, a school place, a planning objection); "Sort it out" at the town hall, 0.85 in office, 0.5 out | `asks/asks.jsonc` (`ask.casework`), `locale/en/asks.ftl`, `asks/params.jsonc` |
| Doorstep asks | a canvass or a talk brings casework at 0.35 | `asks/params.jsonc` (`doorstep_chance`), `asks/offers.rs` |
| Ambient passers-by, promoted when met | built | `characters/ambient.rs`, `NamedCore.origin` |
| Named people's lives | values drift once a year, lazily; no jobs lost, moves, births or deaths | `characters/mod.rs` (`named_step`, `caught_up`) |
| Politicians age and retire | built | `politics/elections.rs` (retire ages, `Filing::Retire`) |
| Named plants, with short time and notice | built | `economy/plants.rs` (`Plant`, `PlantStatus`) |
| Works projects with stages and decay | built (engine) | `policy/works.rs` |
| The dress pass: plots built, boarded, derelict, building sites | built, client side; reads population, vacancy, unemployment | `web/src/hub/layoutV2/dress.ts` |
| Doors on the street | only on places; houses have none | `web/src/components/TownStreet.tsx` (the door hook) |
| Street dressing from state (S7), asset condition and upkeep | not built (W5) | `IMPLEMENTATION_PLAN.md` §4 |
| Encounters on arrival, conversation memory (E10, E13) | designed, not built | `research/ENGAGEMENT.md` §4.8, §6.5 |
| Home | a place with one verb, `rest`; no interior | `places.jsonc` (`place.home`); three interiors in `interiors.jsonc` |
| The Legacy record | built: posts, elections, laws, works, promises, scandals | `legacy.rs` |
| The homecoming summary | built, from the skip | `skip.rs` (REQ-010) |

---

## 1. The test

"Feels like a place" is judged by checks a playtest or the lab can make. Each names how it is measured. None is a counter shown to the player.

| # | Check | How it is measured |
|---|---|---|
| T1 | After an hour, the player can name five people and say where and when to find one of them | Playtest question. Lab: after an hour of the informed bot, at least five people in the ledger with a routine the player has seen twice |
| T2 | Something the player caused is visible on the street in the first chapter: a fault fixed, a work started | Playtest: "What on the street is there because of you?" Lab: in 8 of 8 seeds the engaged stand-in has a fixed fault or a started work by the first chapter's end, and the dress view shows it |
| T3 | In the first hour the player went somewhere because of the day, not the menu | Telemetry: at least one slot spent at a calendar event the turn it was on |
| T4 | Someone showed they knew the player: a greeting by name, a remark on an absence, a callback | Playtest: "Did anyone recognise you?" Lab: a familiar-face line used in the first hour |
| T5 | A household met at a door came back later: an ask, a greeting, a remark on the street | Telemetry: a doorstep record read by a later line or ask |
| T6 | After a year away from a neighbourhood the player sees at most three changes, each with its "Why?" | A scenario test (as REQ-010): skip a year, arrive, three changes, each a causal chain |
| T7 | In walk mode most of the first hour's actions start in the world (a door, a person, a place), not the agenda list | Telemetry: share of actions by where they started; target at least 60% |
| T8 | Quick mode loses nothing that affects play, and walking pays at most a little more | The existing parity gates: `walking_is_a_bonus`, the K4 blind-invariance test, and the e2e that takes every beat from lists |
| T9 | Nothing counts it up | The pocket's length budget test (`POCKET_REWORK.md` §5) extended: no count of regulars, doors, events or faults anywhere |

T7 is the one that answers the owner directly. T8 and T9 keep the answer honest.

---

## 2. Every agenda verb

**The rule.** A verb happens to someone, somewhere. Its *effect* (the formula in `actions/` and its slot cost) stays as it is, so the balance work of DEC-097 and the bot gates hold. What changes is the way in: a door, a person, a room. Three tools do it all:

- **a conversation** (DEC-093) with a **talk hook** (S4): the verb is a choice in a talk with the right person, with its odds shown (`talk::odds`);
- **the exchange core** (DEC-112) in a reduced form, for the one verb that is a room full of people (the town hall meeting);
- **a visit verb** (`visit.verbs`), for things done at a place.

**Time.** Walking is free (DEC-079). A conversation is half a slot (`ChoiceCost::Conversation`). A session at a place is a slot. A long day is two. These are today's costs; the table keeps them, so a turn's three slots still force a choice.

| Verb | Today | Becomes | Reuses | Time | Quick mode |
|---|---|---|---|---|---|
| **Canvass** | Pick a neighbourhood and a worry; a toast (`Action::Canvass`) | Walk the street and knock on doors (§3) | ambient generation, the canvass formula, doorstep asks | 2 slots a street, up to four doors; a single door half a slot | The street's doors as rows; or "Canvass the street" in one tap, the engine choosing the doors |
| **Town hall meeting** | One action in a neighbourhood (`Action::TownHall`) | The room fills from routines; the floor puts one true question; the player answers with one of three lines; then today's formula, with how the line landed | the exchange core's prompt sources and move kinds (`EXCHANGE_SCENES.md` §3, §4.1) in one round: "the floor" (question 1) | 1 slot | The same round in portrait mode |
| **Press** | One action at the news office (`Action::Press`) | An interview with a named journalist at the news office: it opens on the subject (a bill, the top worry, a running story); the journalist asks two things drawn from the outlet's interests and the player's record; the risky line shows the gaffe chance | talk hooks; `actions::press_gaffe_risk`; outlets' interests and lean (`media.rs`) | 1 slot | The same talk in portrait mode |
| **Fundraise** | One action; money appears, `[1500, 6000] × (0.5 + oratory)` | Ask a person: donors are real people (business leaders, the party chair, members at the hotel). The sum comes from who they are and what they think of the player; each gift leaves a string (an expectation they may turn into an ask). A dinner at the hotel, late, is several donors at once | talk hooks; the favour ledger (DEC-031); asks; `fundraise.donor_ties` | half a slot a donor; the dinner the late slot | The same talks from People |
| **Lobby** | One tap for a roll (being changed) | Decided: a conversation that opens on the bill; the whip panel is the count (note 29) | note 29's task | half a slot; two exchanges a member | From the whip list, no walk |
| **Trade a favour** | A separate action (`Action::TradeFavor`) | An option inside the lobbying conversation ("I'll owe you one"). The action stays for NPCs and bots (DEC-030) | note 29 | inside the conversation | As lobbying |
| **Rest** | One action at home | **Stays one tap**, from the pocket, anywhere. Going home is optional and free | — | 1 slot | Unchanged |
| **Casework** | An ask, then "Sort it out" at the town hall, then a toast | A problem with a place on the map, seen, acted on, and shown fixed (§4) | upkeep faults (W5), S7 street dressing, asks | 1 slot at the services desk; half a slot to report a fault on the spot while holding a post | The fault is a row with "See it" and "Sort it out" |
| **Advertise** | Money, no time | **Stays one tap.** It is buying space; the ad then shows in the paper | — | 0 | Unchanged |
| **Take a bribe** | An action at the diner, late | The offer comes from a person in the back booth (the broker, a donor), as a choice inside their conversation, with the risk line shown | talk hooks; the risk lines in `media.jsonc` | the late slot | The same talk |
| **Dip into funds** | An action at the party office | **Stays one tap.** It is paperwork done alone, and that loneliness is the point. The risk line stays | — | 0 | Unchanged |
| **Leak a file** | An action at the news office | A conversation with a reporter (at the news office, or late at the diner): what they will do with it, and what it costs them, shown before | talk hooks; media's hidden facts | 1 slot | The same talk |
| **Visit verbs** | Data at places (tour the works, listen at the gate, be seen…) | Unchanged; calendar events add verbs to the same registry (§5) | `visit.verbs` | as each verb says | Unchanged |

**Why rest stays quick.** Resting is the absence of activity. Making it a scene would tax a player who is already tired, and the health track exists so that tiredness is felt in slots, not in taps. A home interior (§7) gives a reason to go home, never a requirement.

**What it costs.** Press, fundraise, the bribe and the leak are four talk hooks on the lobbying conversation's pattern (note 29: open on a subject, options from what the player knows, each with its chance and cost shown). Lines are written per role group, as note 26 set for informative lines. The town hall's round is the one piece that needs more than a hook (question 1).

**Parity.** Every form above has a list form: the conversation in portrait mode, doors as rows, the meeting in the dialogue box. Nothing that affects play is walk-only (DEC-051, DEC-097 §5).

---

## 3. Canvassing at doors

Note 5 asked for it: "tap a door, canvass it. The pocket keeps the quick path."

### 3.1 Doors on the street

The engine knows neighbourhoods, not plots: plots are drawn by the client from the versioned layout (`layoutV2/`). So doors are **slots**, not geometry.

- Each neighbourhood has a number of door slots, from its population (proposed: one per 150 residents, between 16 and 60).
- The client maps slot *k* to the *k*-th residential plot by the plan's fill rank, the order the dress pass already uses (`dress.ts`). The plan never moves (DEC-090, DEC-099), so door 7 in Mill Row is the same house every year.
- A plot the dress pass boards up is a door with nobody home: vacancy is seen, and knocked on.
- In quick mode the same slots are rows: "The Harrows, number 12".

### 3.2 Households from live state

A household is generated when its door is knocked, never before, as ambient people are (`characters/ambient.rs`). It is keyed by (seed, neighbourhood, door, generation), so the same door gives the same family however often it is asked.

| Part | Drawn from |
|---|---|
| Family name | `names.jsonc` |
| Who lives there (ages, class, community) | the neighbourhood's cohorts (`population/`) |
| Owner or tenant | the neighbourhood's housing (`economy/housing.rs`) |
| Where the earner works | the town's employers weighted by jobs; a named plant where there is one (`economy/plants.rs`) |
| What worries them | `concerns::ranked(hood, Felt)`, weighted by their cohort |
| What they think of the player | the cohort's approval and awareness, with a keyed spread |

`generation` starts at 0 and rises when a household moves out (§8, M7), so a house can change hands. Until then households are stable. Nothing is stored until something happens worth remembering (§3.4).

### 3.3 The doorstep exchange

Short on purpose: one line from them, one choice, one line back. About ten seconds a door.

1. **Knock.** Who answers, and their opener: "Ivy Harrow. I work at the cannery, when it's open." The opener reads their worry and their view of the player.
2. **One choice of three:**
   - *Listen:* they say what worries them. Two doors that agree raise the neighbourhood's worries to `known` (`Subject::HoodConcerns`). A miss teaches something too: what this house cares about.
   - *Make the case* on a worry: as today's canvass issue (`canvass_match`). Their reply shows how it landed.
   - *Ask for their vote* (in a campaign) or *a poster in the window*: a promise from them, which the street then shows (a poster on that plot).
3. **Their reply,** and sometimes an ask: casework about a fault near their door (§4).

**A canvass is a street, not a door.** The first knock starts the canvass and spends its two slots (`action_costs.canvass`). The player knocks on up to four doors. "Finish the street" applies today's formula, with the worry argued at most doors as its issue (ties: the first). The other doors are assumed knocked, as now. The doors add **no** approval beyond the formula (DEC-097's balance principle): they add knowledge, memory and asks. The doorstep ask is still one roll a canvass (`doorstep_chance`), taken from a house the player knocked on.

A single door outside a canvass is a conversation: half a slot, no canvass effect.

### 3.4 What is remembered

- A **doorstep record** per household met: the date, how it went (warm, cool, not in), the worry heard, any promise made either way. It belongs to the characters domain and names the player's character id, not the player domain, so it survives DEC-103's return (§9).
- Bounded: 16 a neighbourhood, the oldest forgotten. A household not called on for a year forgets the call.
- A household that brings an ask has its speaker promoted to a Named resident, by the path a `Meet` uses (`NamedCore.origin`).
- Next time at that door: "You came round in March about the buses." The Notebook's neighbourhood row lists the households the player knows by name, never as a count.

### 3.5 Never a chore

- Four doors at most a canvass. "Canvass the street" (one tap) is always there and loses only the choosing.
- The informed play the gates want (DEC-097) is knowing which street to canvass, not knocking more doors.
- **Scale.** In a town-wide race the party's volunteers canvass the streets the player does not. At the region (Act 2) the player has a **campaign team**: named organisers and party members, recruited through an ask chain, each given an area for the campaign. Areas are the jurisdiction's areas one level down, as the exchange audience reads them (`EXCHANGE_SCENES.md` §2.2). A **phone bank** at the party office reaches several areas thinly (contact, no doors). The team reports as Today items with one quote from one household each: "Sam at the Leeside phone bank: they keep asking about the bridge." The player's own doors are only where they stand. The formula is the same at every tier.

---

## 4. Casework you can see

Today casework has no place and no cause: the problem is a Fluent variant chosen by the asker's occupation (`ask-casework` in `asks.ftl`), and "Sort it out" is a roll. The audit found that nothing the player does moves the services worry (`research/SIMULATION_AUDIT.md`, the leverage verdict). Casework you can see gives that worry a cause the player can find and a lever they can pull.

### 4.1 Faults: upkeep made visible

**Decision: a fault is the unit of upkeep.** W5's "asset condition and upkeep" task is not built. Build it so that a neighbourhood's condition shows as **faults at spots**: a light out on a named street, a blocked drain, a pothole, a bus shelter shut, a broken swing. Then casework, the street and the budget are one thing.

A registry, `content/base/faults.jsonc` (proposed):

| Field | Says | Example (`fault.streetlight`) |
|---|---|---|
| `asset` | What it is on | a street light |
| `spot` | Where, as a slot kind (like doors: the client maps it to a cell) | a street of the neighbourhood |
| `raise` | U1 conditions over upkeep's condition, the season, the weather | condition below 0.6; more likely in winter |
| `pays` | The budget category that repairs it (`finance::Category`) | `infrastructure` |
| `repair` | Cost and days | £800, 5 days |
| `feeds` | The worry it adds to while open | `concern.services` |
| `cue` | The S7 street cue while open, being repaired, and new | a dark lamp; a van and cones; a new lamp for 3 months |
| `text` | Its Fluent prefix | `fault-streetlight-` |

The upkeep domain owns faults (systems write only their own domain). Condition falls each month; faults open where it is low; each month the **Infrastructure appropriation** pays for repairs in a queue, oldest first. Open faults are bounded (proposed: six a neighbourhood). The paper kinds of casework (a school place, a benefits form, a planning objection) stay as they are, with "See it" pointing at the school or the town hall.

### 4.2 The resident brings it

The casework ask is cast on a fault near the asker's door: "The light on Ash Lane has been out since October. My mother won't walk to the shop after dark." Its need becomes "this fault is fixed" (proposed `Need::Fixed { fault }`), met whoever fixes it: the world does not wait (DEC-091).

### 4.3 The player sees it, and acts

- The Today item has **"See it"** (DEC-099): it walks there, or puts a pin on the map. The cue is on the street: the dark lamp at dusk, the puddle after rain.
- **"Sort it out"** at the town hall's services desk (1 slot), or on the spot for half a slot while holding a post. In office it **moves the fault up the repair queue**; it does not create money. Out of office it is chasing it up, at today's lower chance.
- **The trade-off is real.** The queue is finite. Moving one fault up pushes the next one back, and "Why?" says so: "Fixed ahead of the Quarry Road drain." Question 3 asks the owner to confirm this over a cheaper rule.
- **The real lever is the budget.** When the queue has no money left this month, nothing is fixed until the next month or a budget change. A vote to raise Infrastructure, or a works family that renews a whole district (DEC-099), clears many faults at once. Casework is how the player learns where the budget hurts.

### 4.4 The street shows it fixed

When the repair is done, the cue changes the turn it is made (a van, cones), then the spot looks new for a few months (fresh tarmac, a new lamp), then ages with the rest. The resident remembers (`kept_promise`, as today's keep effects), the doorstep record says so, and the neighbours' approval moves by today's small amount.

### 4.5 Failure shows too

- The deadline passes and the light is still out: today's break effects, the resident's line changes, and the cue stays.
- After a budget cut, faults outrun repairs and the street visibly runs down: more dark lamps, more cones that nobody moves. The services worry rises in that neighbourhood.
- The chain is on the spine (S1): the budget vote, then the appropriation, then the queue, then the fault left, then the worry. "Why?" can walk it, and the rival can quote it in a debate (DEC-112's prompts are true).

---

## 5. The town calendar

### 5.1 A turn is one day

A turn is a fortnight played as one representative day (DEC-013, `representative_day`). So cadence is counted in turns:

| Cadence | Held | Example |
|---|---|---|
| `turn` | every turn, in its part | market day (morning) |
| `month` | every other governing turn | a residents' association |
| `season` | once in a season's window | the school term starting |
| `year` | the turn whose span holds its date | the summer show, the memorial |

The existing weekday events (`session`, `meeting_night`) keep working as now; they become the first entries of the registry, with the same keys, so no pin moves.

### 5.2 The registry

`content/base/calendar.jsonc` (proposed, `kind: "calendar"`):

| Field | Says |
|---|---|
| `id`, `name_key` | The event, and its name with arguments (the town, a saint, the plant) |
| `cadence`, `window`, `part` | When (§5.1) and the part of day |
| `scope` | The tier it belongs to: a town, a region, the nation. Instances are keyed by `Jurisdiction` (DEC-098) |
| `venue` | A place kind, a neighbourhood's streets, or `legislature` (where the jurisdiction's legislature sits) |
| `requires` | U1 conditions: the place kind exists, a community's share, an industry, the coast, a past disaster |
| `attendees` | Routine rules with `events: ["event.<id>"]`, so routines place people there (a new `RoutineEvent::Calendar`) |
| `crowd` | How full the place looks (ambient density) |
| `verbs` | Visit verb ids done there (be seen, say a few words, a stall) |
| `talk` | People here answer the open question freely, and presence counts double for familiarity (§6) |
| `expected` | Posts expected to attend, by constitution role word or power, or by a constituency that holds the venue. No office named |
| `absent` | What being expected and absent costs (§5.5) |
| `heard` | Overheard lines about it, before and after |

### 5.3 Generated per town (DEC-100)

Each town gets the events whose `requires` hold, with a keyed date inside the window and a keyed name from a name registry. Derived from the seed and the town, never stored, like `meeting_night`. Some events come from history: a memorial exists only if the world had the flood it remembers, and it is named after it. Two towns therefore never have the same year.

### 5.4 The first events

| Event | Cadence | Venue | Who is there | What the player can do |
|---|---|---|---|---|
| Market day | turn, morning | the market | farmers, shopkeepers, retirees, the broker (who is at the market by day already) | talk; be seen; hear the price of things |
| Council night | the body's sitting | the legislature | members, the journalist, the public gallery | vote; `verb.public_gallery`; talk in the corridor |
| Meeting night | as today | the community centre | the people `meeting_night` places now | a town hall meeting there is fuller (§2) |
| The summer show or fair | year, July–August | the park | most of the town, by crowd density | be seen; say a few words; open it (expected of the executive's holder) |
| The memorial | year, November, or a disaster's anniversary | the square | officeholders, clergy, older residents | attend (expected of holders whose constituency holds it); lay a wreath |
| The school term | season, September and July | the school gate | parents, the head teacher, teachers | hear the services worry first-hand; `verb.talk_to_head` |
| Match day | turn, in season | the ground | fans by class and age | be seen; needs a club place kind (not built; STAGED_ARCS §8 "the team") |
| The plant's gala | year | a named plant | its workforce, the steward, the manager | `verb.listen_at_gate`; the workers' mood |

**Religious festivals** wait for M7's faith communities (question 5): the base pack has one community (`communities.jsonc`) and no place of worship kind (`places.jsonc`), so a festival now would be decoration (DEC-095).

### 5.5 Going, and staying away

- **Today shows the fortnight:** "This fortnight: the Harrowgate Show, Saturday afternoon, the park", with "Go" (it walks there, or opens the place in quick mode).
- An event takes the slot of its part, so it competes with the agenda. That is the point: going somewhere because of the day.
- **Absence costs only where a post is expected,** and is shown a turn ahead with its cost: a little recognition among those who came, the organiser's memory (`absent`, a small opinion fall), and a line in the paper if the outlet covers the event. At most one expected event a turn. Ordinary events cost nothing to miss except what would have happened there (question 2).

---

## 6. Regulars and personal stories

### 6.1 Regulars come from routines

A person is a **regular** at a place, in a part of the day, when their routine puts them there most days: the councillors at the diner on Tuesday and Thursday afternoons, the broker in the back booth late, the retiree in the park in the morning (`routines.jsonc`). This is derived, never stored. Once the player has seen them there twice, the opened person row says it: "Usually at the diner on Tuesday and Thursday afternoons" (`POCKET_REWORK.md` §3.2 already asks for this line).

### 6.2 Familiarity is built by presence

- **Stored:** one small number on the person's relationship toward the player (proposed `Relationship.familiar`, 0 to 255, in the characters domain). It rises by one when the player is at the place and part where that person is: on arrival, or doing anything there. At most once a turn a person; double at an event with `talk`. It falls by one each month without a meeting.
- **In words, never as a number:** a stranger; "knows your face" (3); "a nod at the diner" (6); "one of ours" (10, with opinion above 20).
- **Quick mode counts the same.** "Go to the diner" is presence; walking is not required.

### 6.3 What a familiar face tells you

| Familiarity | What changes | Built on |
|---|---|---|
| Knows your face | Greets the player by name; a new condition `familiar` in the condition language picks the opener | talk openers (DEC-093) |
| A nod | Once a turn, on arrival, walks up and says what is on their mind, free (`ChoiceCost::Free`). Off the street, a portrait on arrival | the open question (note 26); E10's encounter, limited to familiar people |
| One of ours | Brings a tip or a lead more often; makes asks earlier and bigger; turns up at the player's town hall meeting; notices an absence ("Haven't seen you at the market") | E10, asks, routines |

**Why this is not a checklist** (DEC-091, 2026-11-25). Familiarity is a deed, not knowledge: it records being there, so the K4 guard holds. It changes what the world offers (principle 5), not which answer is right. It grows at one place a slot, so depth in a few places beats a tour of all of them (principle 3). Nothing counts how many regulars the player has (T9).

### 6.4 Personal stories: arcs at a person's scale

A **personal arc** is a staged arc (DEC-109) whose cast is one person and their household, with the same rules: raised by the world, transitions read and beats write, at least three exits, one reachable with no act. Its turn is played with talk, visit and ask beats. It is a background arc, so it needs more than one decision but no exchange scene (DEC-112, answer 4).

| Arc | Raised by | Levers | Exits |
|---|---|---|---|
| The lease | a shopkeeper's lease ends while rents run ahead of trade | a word with the landlord; a rates relief bill; a unit on another street | stays; moves; closes, and the shop boards up (S7) |
| The child leaving | a regular's child is of age and youth jobs are scarce | an apprenticeship at the plant (a talk with the manager); the college | stays; leaves and writes; leaves |
| The last shift | a steward's plant goes on notice (`PlantStatus`) | the plant arc's own levers | kept on; the gate on the last day; another job in town |
| The retirement | a teacher or nurse reaches their age | the farewell, an event the arc makes (§5) | a send-off the player came to; a quiet leaving |
| The long illness | a regular stops coming | a visit at home; clinic waiting times as a law | recovery; a move to family; death (M7, question 4) |

- **Raised for people who know the player.** The raise reads familiarity or trust, which are deeds; never the ledger (DEC-091). The cause runs for everyone: leases end everywhere; the arc exists only where someone the player knows lives it.
- **Bounded:** at most two personal arcs running, timed by the director as minor; never the lead arc (DEC-109 §7).
- **On the ledger,** as any arc (`Subject::Arc`): heard when a regular mentions it, known when the person says it, detailed with trust. A player who never knew misses a story, never an answer.
- **The story for a system** (DEC-109, owner's addition): the life model of §8 ships with these arcs.

---

## 7. Home and office

| | Today | Proposed |
|---|---|---|
| Home | `place.home` with `rest`; the dress pass draws the player's house (`dress.ts`) | An interior (`interior.home`) by household: a terrace, a semi, a flat in the capital. The morning paper on the table, letters on the mat (asks by post), the memento shelf. Rest is still one tap from anywhere |
| Office | Routines place holders at their post's workplace (`works`, the body's `works_at`) | A room per place kind, named by the constitution's post (DEC-088 interiors, DEC-098): a desk in the members' room for a seat, the executive's room, an assembly office, a ministry. In Act 2, a named aide (DEC-105 staff) |

**Mementos.** A registry (proposed `mementos.jsonc`) maps a Legacy entry (`legacy.rs` `LegacyKind`) and conditions to an object: the rosette from the first win, the first bill framed, the scissors from a works opening, a card from a household whose casework was done (with their name), the cutting of a scandal, the poster of a debate won. The engine emits mementos as facts with ids (DEC-054); the client draws them from an art kit. The shelf shows the eight most salient, and each opens its "Why?" on the spine. It is the Legacy record as objects. It unlocks nothing (DEC-100).

**It changes with the career.** The office moves with the post. Losing an election is a box of things on the desk. The home stays in the home town until the player moves (Act 2 adds a flat in the capital and keeps the house). At the end of a run the career card (note 23) can show the shelf.

---

## 8. Lives move on

### 8.1 What exists and what is missing

| Life change | Today | Needed |
|---|---|---|
| Ageing, retiring from office | politicians, `politics/elections.rs` | — |
| Values drifting | Named people, once a year, lazily | — |
| Jobs lost or found | plants close (`economy/plants.rs`), but a Named person's `occupation` and `place` never change | a job change when their workplace closes or hires |
| Moving within or out of town | aggregate households only (`economy/housing.rs`) | moves driven by rent, income and work |
| Partners, children, children leaving | none | household facts on Named people |
| Retiring from work, illness, death | none for Named people | by age and health |

### 8.2 The life model (M7)

- **Household facts** on Named people (proposed): a partner, children with birth years, owner or tenant. A child who comes of age can become a Named person (a new id, never reused, DEC-041).
- **A life-events registry** (proposed `life_events.jsonc`): kind, U1 conditions over the person and live state (their plant's status, the neighbourhood's rents, unemployment, age), a yearly chance, and the change it makes. The characters domain changes its own records; anything else (a vacancy at a workplace) is an effect request.
- **Where it runs.** In focus towns, a monthly system steps life events for stored Named people (a few hundred), reading live state. Elsewhere, lazily, keyed by (seed, person, year) on seed-level draws only, so a long skip or a world gap stays cheap: DEC-105's middle level of detail. Keyed draws keep it order-independent.

### 8.3 How the player notices

- **The street:** a sold sign at a known household's door; a shop boarded; a new name over it; a pram outside (S7 cues).
- **Conversation:** a `life_event` condition on lines: "Since the cannery went I've been driving for the depot."
- **The paper:** a notices column for people the outlet's readers know: births, retirements, deaths.
- **The Notebook's note line** (`POCKET_REWORK.md` §3.1): "Moved to Eastport in March." "Retired from the school."
- **The door:** a new family answers (`generation` has moved on), and says who lived there before.

### 8.4 Coming back after a year

On arriving in a neighbourhood not visited for six months or more, one card: **"Since you were last here"**, at most three changes, ordered by the engine by what matters to the player (people known first, then their own works and faults, then shops). "The Harrows moved away. The bus shelter is fixed. The corner shop has new owners." Each opens its "Why?". It reuses the homecoming summary (`skip.rs`) and the spine. It is never a list of everything.

---

## 9. The later acts, and return to a world

### 9.1 The same pattern at every tier

| Piece | Region | Nation |
|---|---|---|
| Calendar | the regional show, the assembly's opening (by constitution id) | the national day, the budget day, the party conference |
| Regulars | the assembly's members' bar, the regional paper's office | the press gallery bar, party headquarters, a ministry canteen |
| Doors | a campaign team and phone banks (§3.5) | the same, more areas |
| Casework | constituency work (below) | the same |
| Works and faults | regional works on the country map (DEC-099 §5) | national works |

Everything is keyed by `Jurisdiction`; no event, place or post is named in code (DEC-098).

### 9.2 The home town stays connected

- **A constituency surgery:** a monthly calendar event in the home town, expected of the seat's holder whose constituency holds it. Residents bring faults and paper cases from the home town's upkeep, as casework.
- **Letters:** asks that arrive at the office by post (`from: letter`), from people the player knew.
- **Visits:** travel home (M7 travel), where the home town's calendar invites them.
- **"Our old councillor":** lines and openers read posts held (the Legacy record), so the town introduces the player by what they were.
- **The middle level** (DEC-105 §3): faults and upkeep run monthly, life events lazily, familiarity is kept and falls slowly, doorstep records are kept, works and V1's credit carry the player's later decisions home.

### 9.3 What this design must hold for DEC-103

When the player returns to a world as a new person, the old character becomes a person in it. This design keeps that cheap:

1. **Everything people remember of the player is on the player's character id** in the characters domain (relationships, familiarity, memories, doorstep records), never in the player domain. Retiring the player into an ordinary character (DEC-103's first open question) then carries it all.
2. **Doors and fault spots are slots keyed to the plan,** so the same house is the same house a decade on.
3. **The calendar is derived from the seed and the town,** so the same show runs. A memorial can remember a matter the old character handled (an arc's exit on the spine).
4. **Mementos become the town's things:** the plaque at the works (DEC-099), a portrait in the town hall corridor for a past holder of the executive post, the booth at the diner the regulars say they kept.
5. **Lives run through the gap** (§8.2, lazily), so households have moved on. Doorstep records older than the gap shrink to one fact: "knew the old councillor".
6. **The new character knows only reputations** (DEC-103 §4). The regulars who knew the old one talk about them: "You remind me of the one who used to sit there."
7. **Bounded:** 16 doorstep records a neighbourhood, one byte of familiarity per edge, life events kept only as memories. Save size is measured in the lab before DEC-103 ships.

---

## 10. What ships before the tester checkpoint, and what is M7

### 10.1 The lead's guess, checked against cost

| Piece | Depends on | Cost | Verdict |
|---|---|---|---|
| Canvassing at doors | ambient generation, the canvass formula, doorstep asks, plots in the dress pass | mid engine, mid web | **Checkpoint.** Note 5 is already in W5 |
| Casework you can see fixed | W5 upkeep (not built), S7 (not built) | upkeep is planned; faults add a small step if upkeep is built with them; S7 adds the cues | **Checkpoint,** if faults are upkeep's unit from the first build (§4.1). Built separately later it costs a second pass over upkeep |
| The town calendar, a few events | routines | mid engine and protocol; content small | **Checkpoint,** with market day, council night, meeting night, the show, the memorial and the school term. Match day waits for a club place kind |
| Visible works | S7, `policy/works.rs` | planned in W5 | **Checkpoint** (already planned) |
| **Regulars and familiarity** (not in the guess) | routines, talk openers | small to mid; same files as the calendar | **Add to the checkpoint.** Without someone noticing the player, events are scenery. It is the cheapest part of the owner's "connected to their area" |
| Lobbying as a conversation | note 27 (the broker) | planned (note 29) | **Checkpoint** (already planned) |
| Fundraise, press, the bribe and the leak as conversations | note 29's conversation | one mid task on the same code | **Checkpoint,** after note 29 (question 6) |
| The town hall's floor | the exchange core (X1, built), question 1 | mid | **M7,** unless the owner wants it sooner |
| The life model and personal arcs | new | large to design, mid to build, content | **M7** |
| "Since you were last here" | the life model, the spine | small once the life model exists | **M7** |
| Home and office interiors, mementos | an art round, `legacy.rs` | mid and art | **M7** |
| Campaign teams, phone banks, surgeries, letters | Act 2, DEC-107 | with Act 2 | **M7** |
| Religious festivals | faith communities | with faith | **M7** |

**Recommendation:** the lead's four, plus regulars and familiarity, plus the four menu verbs as conversations. That makes the checkpoint answer T1 to T5 and T7 to T9; T6 waits for the life model in M7.

### 10.2 The task plan before the checkpoint

Bundled so that few agents do it, at most two at a time, with disjoint files.

| # | Task | Tier | OWNS (in short) | After | Reuses | Acceptance |
|---|---|---|---|---|---|---|
| B1 (**built**, §10.4) | **The calendar and regulars:** the registry, `RoutineEvent::Calendar`, instances by `Jurisdiction` with `meeting_night` and `session` folded in unchanged; familiarity on the relationship; the `familiar` condition; greeting openers; "usually at" in the person view; "this fortnight" in Today; expected presence and its cost | mid | `mandate-content/src/routines.rs`, a new `calendar.rs`; `characters/routines.rs`, `characters/mod.rs` (the edge field); `condition.rs`; `content/base/calendar.jsonc`, `routines.jsonc`; the protocol's pocket and people views; save bump by defaults | — | routines, visit verbs, talk openers | Same seed, same calendar; every pin unchanged with the new events removed; K4 holds; every event reachable from Today in quick mode; a stand-in that goes everywhere does not beat one that goes where its plan needs |
| B2 | **Upkeep with faults, and casework on faults:** condition per neighbourhood, faults at spots, the repair queue from the Infrastructure appropriation, `Need::Fixed`, "Sort it out" as a priority, the services worry reads open faults, "See it" | mid | a new `mandate-sim/src/upkeep/**`; `finance/` (effects only); `asks/` (casting, needs); `concerns.rs` (reads); `content/base/faults.jsonc`, `asks/asks.jsonc`; the protocol's fault view | — (∥ B1) | the W5 upkeep task's scope; asks; the spine | A budget cut raises open faults and the services worry in a twin run; an expedited fault delays the next, and "Why?" names it; outcomes with no faults content unchanged |
| B3 | **Doors:** door slots, households on demand, the doorstep exchange, a canvass as a street, doorstep records, the quick-mode list; the web's door hotspots on residential plots | mid | a new `characters/households.rs`; `actions/ground.rs`; `asks/offers.rs`; the protocol's street view; `web/src/hub/townScene.ts`, `TownStreet.tsx`; the pocket's door rows | B1 (both touch `characters`) | ambient generation, `canvass_match`, `doorstep_chance`, the dress pass's fill ranks | The same door gives the same family twice and through a save; a street canvass equals today's formula in a twin; e2e: four doors by walking, and by list |
| B4 | **Street dressing (S7)** with works and faults: the registry from state to cue; the art for faults and repairs | mid, plus the art kit | `web/src/hub/layoutV2/dress.ts`, a dressing registry, `tools/pixelkit` (fault and repair cues) | B2's view (∥ B3: different web files) | the dress pass, the works kit (DEC-099) | A fault opened, repaired and new shows three cues on its spot; T2 passes in 8 of 8 seeds |
| B5 | **Fundraise, press, the bribe and the leak as conversations** | mid | `talk/hooks.rs` registrations; `actions/publicity.rs`, `actions/misconduct.rs` (the effect stays, the way in changes); talk content | note 29 | note 29's conversation, `press_gaffe_risk`, risk lines | Each verb's effect equals today's in a twin; odds shown equal odds rolled; the bot gates hold or are made report-only by rule 11 |
| B6 | **Lines,** through the content pipeline: doorstep openers and replies, greetings by familiarity, events, faults, the four verbs' talks | mid drafts, a reviewer reads samples | `locale/en/*.ftl`, the content registries above | each task's keys | the style bible, the doctor | The doctor's lints clean; the owner's blind sample |

**Order:** B1 and B2 together; then B3 and B4 together; B5 when note 29 lands; B6 rolls behind each. Four engine saves at most (B1, B2, B3, B5), one per merge.

### 10.3 M7, in order

1. **The life model** (large to design, mid to build), with its story: the personal arcs of §6.4 and "Since you were last here" (DEC-109's addition: a system ships with a story).
2. **Home and office,** with mementos (mid, plus an art round).
3. **The town hall's floor,** if not taken earlier (mid).
4. **Faith festivals,** with faith communities.
5. **Act 2's scale:** campaign teams, phone banks, surgeries, letters, regional and national events, with DEC-105.
6. **DEC-103's return,** reading §9.3.

### 10.4 B1 as built (2026-11-25)

**Built.** `content/base/calendar.jsonc` (types `mandate-content/src/calendar.rs`, schema, validation, the doctor's never-fired check); the day played and the instances in `mandate-sim/src/characters/calendar.rs`; familiarity in `characters/familiar.rs` (`Relationship.familiar`, `characters.jsonc` `familiarity`); the `familiar` condition; five familiar openers; "usually at … on Tuesdays and Thursdays" from routines (`routines::regular_days`); Today's "This fortnight" with Go, the bar's day with its weekday, the turn passage's lines (`OverviewView.passage`); save v32.

**One date per turn** (DEC-114) as built: `Calendar.day` is chosen as each turn begins (`calendar::choose_day`): the home town's main occasion's own date (expected first, then the largest crowd, then registry order), else a working day (Monday to Friday) of the span by keyed draw on the jurisdiction and the turn. Routines, the bar, Today and the facts all read it; one outside the turn (a save from before it, a lab advance) is chosen again when read.

**Departures from §5 and §6**, each for a reason:

1. **Only occasions choose the day.** `turn` events (market day), `meeting_night` and `session` never do: market day every turn would make every turn its weekday. They are held on the day played: market day always (in its part, if free), the meeting night and the regular sitting when the day falls on their weekday, as before; a sitting with business (a law-making stage in the turn) whatever the day, as `session` already read. So weekday rules still come round, on working days mostly; weekends come with occasions.
2. **Parts.** The main occasion takes its part first, then the sitting and the meeting night, then other occasions, then every-turn events; an event with no part left is not held (an occasion is reported on its own date). A sitting keeps its part even when it is taken (the legislature sits on business).
3. **The memorial** is at the town hall (there is no square place kind) in November, where the town had a disaster in the last ten years (`disaster` within 120 months: the economy keeps the months since, not the disaster's kind or date, so no anniversary and no name after it).
4. **Expected:** the show and the memorial of the executive (`power: veto`), the memorial of the seat whose district holds the town hall (`ward`). Council night expects nobody: votes are cast from the bill, and expecting every sitting would cost most turns.
5. **Absence** costs an opinion fall among the stored people who were there in its part; no recognition and no paper line (outlets do not cover events yet).
6. **Familiarity's effects:** greetings by name (openers on `familiar`) and, from "a nod", the name tag's "!" for "What's on your mind?" when they have something to say. The walk-up on arrival (E10), tips, earlier asks and "haven't seen you at the market" wait for E10 and B6.
7. **Conversations naming the day:** the passage and the event log name the date; lines about the day's event wait for B6 (lines).
8. **Match day** is not built (no club place kind); **the plant's gala** is, where the town has a factory. Faith festivals wait for M7.
9. **Focus towns only:** the calendar is computed for the player's home town; other towns' routines run as before.

---

## 11. Questions for the owner

1. **May the town hall meeting use one round of the exchange, outside an arc?** DEC-112 (answer 4) keeps scenes to an arc's turn, at most one a chapter. Recommend yes, as "the floor": one true question, three lines, about thirty seconds, not counted as a scene. *Yes:* the exchange core runs outside an arc (its state on the action, not the arc run), one mid task; the most social verb stops being a menu. *No:* the meeting stays one tap with a better outcome line.
2. **What does missing an event cost?** Recommend a cost only where a post is expected, small (recognition among those present, the organiser's memory, a paper line), shown a turn ahead, at most one such event a turn. *Harsher:* the calendar becomes a chore. *None:* events are scenery for an officeholder.
3. **Does "Sort it out" move a fault up a finite queue, or buy the repair?** Recommend the queue: the trade-off is visible and leads to the budget. *Queue:* someone else's fault waits, and some players will find that hard. *Money:* simpler, but casework becomes a slot-for-repair machine and the budget never bites.
4. **Do the lives of people the player knows move on, deaths included?** Recommend yes, in M7: old age and illness only, heard of and never shown, with a funeral the player may attend. *Yes:* a monthly life system for focus towns and its lines. *No deaths:* nobody old ever goes, and a return after twelve years (DEC-103) looks frozen.
5. **Do faith festivals wait for M7's faith communities?** Recommend yes: the base pack has one community and no place of worship, so a festival now would be decoration (DEC-095). *Cost:* the checkpoint's calendar is civic and secular only.
6. **Do fundraise, press, the bribe and the leak become conversations before the checkpoint?** Recommend yes, as one mid task after note 29, because it reuses the same conversation. Rest, advertising and dipping into funds stay one tap. *Yes:* one more mid task in W6. *No:* testers keep three menu verbs, the ones they will use most after canvassing.

---

## 12. Decision proposal

## DEC-114 — Being there: the town is lived in

- **Status:** Proposed (2026-11-25). The owner asked whether other parts of the game could be "more interactive than a menu based diplomacy simulator", wanting players to "feel connected to their area like with Stardew Valley but with a civic focus and the extension we get from later acts", and agreed to a design pass ("Yes do the design pass"). Design: `docs/design/BEING_THERE.md`.
- **Context:**
  - Canvass, the town hall meeting, fundraising, the press and casework are each one action and a toast. Lobbying is already becoming a conversation (note 29).
  - The town has routines, named plants, works and a dress pass, but no calendar the player sees, no doors on houses, no faults on streets, nobody who notices the player's presence, and no life events for its people.
- **Decision:**
  1. **A verb happens to someone, somewhere.** Each agenda verb's effect and slot cost stay. Its way in becomes a door, a person or a room, through conversations (DEC-093), talk hooks (S4) or the exchange core (DEC-112). Rest, advertising and dipping into funds stay one tap.
  2. **Doors.** Neighbourhoods have door slots that the client maps to residential plots by the plan's fill rank. Households are generated on demand from live state, keyed by door. A canvass is a street of up to four doorstep exchanges and adds no approval beyond today's formula. Doorstep records are bounded and kept on the player's character.
  3. **Faults are upkeep's unit.** Upkeep shows as faults at spots, repaired monthly in a queue paid from the budget. Casework is cast on a fault; "Sort it out" moves it up the queue; the street shows it open, being repaired and new, and shows neglect too.
  4. **A town calendar as data.** Recurring events, keyed by `Jurisdiction`, with cadence in turns, generated per town from its features and history, attended through routines. Absence costs only where a post is expected, shown in advance.
  5. **Regulars and familiarity.** Regulars are derived from routines. Familiarity is a deed (presence), stored as one small number on the relationship, shown in words, never counted. It changes what people offer, not which answer is right (DEC-091).
  6. **Personal arcs** (M7): staged arcs at a person's scale, raised by the world for people who know the player, played with talk, visit and ask beats, at most two at a time.
  7. **Lives move on** (M7): household facts and a life-events registry for Named people, monthly in focus towns and lazily elsewhere; noticed on the street, in talk, in the paper, in the Notebook, and in "Since you were last here".
  8. **Home and office** (M7): interiors by household and by post; mementos from the Legacy record, unlocking nothing.
  9. **Every tier, and the return.** The same registries serve the region and the nation. What people remember of the player stays on the player's character, so DEC-103's return carries it.
  10. **Parity and no checklist.** Everything that affects play is reachable from lists in quick mode (DEC-051); nothing counts regulars, doors, events or faults (DEC-113).
- **Before the tester checkpoint:** doors, faults and casework you can see, the calendar with six events, visible works, regulars and familiarity, and fundraise, press, the bribe and the leak as conversations (tasks B1 to B6). The rest is M7.
- **Dependencies:** DEC-013, DEC-030, DEC-031, DEC-041, DEC-051, DEC-054, DEC-073, DEC-079, DEC-086, DEC-088, DEC-091, DEC-093, DEC-095, DEC-097, DEC-098, DEC-099, DEC-100, DEC-103, DEC-105, DEC-109, DEC-112, DEC-113.
