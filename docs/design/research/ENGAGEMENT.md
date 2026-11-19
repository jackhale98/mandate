# Engagement and variety (DEC-097): design pass

**Status:** Design brief for DEC-097 (2026-11-19). It turns the six parts of DEC-097 into contracts, numbers and tasks. Nothing here changes an accepted decision. Choices DEC-097 leaves open are marked **Proposed** and are listed again in §9 for the owner.
**Built on:** `claude/bold-brown-shyeug` at save format v17 (K1–K4, K6, C1, C3, B1–B3, G1 merged).
**Read with:** DEC-030–DEC-035, DEC-041, DEC-051, DEC-054, DEC-072, DEC-076, DEC-077, DEC-078, DEC-079, DEC-088, DEC-090, DEC-091, DEC-093, DEC-094, DEC-095, DEC-096, DEC-097; `M6_ENGINE_CONTRACTS.md` §1 (ledger), §3 (conversations), §4 (tasks); `TOWN_GENERATION.md` §5, §10, §11.

**Task ids.** Tasks here are `E1`–`E12` (engagement). They are not the town v2 "phase E" steps of `TOWN_GENERATION.md` §15 (done, as B1 and B2).

---

## 0. In short

- **The ground game.** A canvass mostly builds name recognition. Its approval gain falls fast when the same neighbourhood is canvassed again within weeks, and is doubled in campaign season. Approval between elections comes mainly from delivery: kept asks and promises, help to individuals, the press covering the player's work, and (with V1) credit for laws.
- **Asks and promises.** People, the party, the broker, organisations and the press ask the player for concrete things with deadlines. The player accepts or declines; an accepted ask is a promise, kept or broken by what the player actually does (a vote, a statement, a visit, a town hall, a draft, a piece of casework). Campaign promises are public asks from the ward. A new `asks` domain owns them; the Today tab lists them.
- **Storyteller variety.** Every template gets a cooldown of at least a year that doubles with each use, never repeats within a chapter (the climax included), and is drawn from the best few candidates by relevance and novelty instead of always taking the single best. About 22 new dilemmas (10 of them consequences of the player's own past answers), 6 new storylines including NPC schemes (rival, broker) that run between elections, and press stories about the player's work.
- **Secrets.** Town secrets are hidden facts (DEC-034) seeded from live state by a content registry: the broker's deal, a buried report, who funds the rival, a councillor's debts, a planned closure. People who know them tell at their discretion (DEC-093); the player can investigate or tip off the press. They exist and can surface whether or not the player finds them.
- **Walking rewards.** A registry of curiosities (plaques, murals, old posters, a lost cat, a bench with a story, a view), placed by the engine per neighbourhood and drawn by the client in slots the layout reserves. Rare street encounters with real people. Lore is collected in the notebook. Anything that affects play is also found by "Look around" from the Places tab; pure flavour may be walk-only.
- **The engagement gate.** `mandate-cli engagement` plays 5-year careers and checks template variety, repeats, quiet stretches, storylines, press about the player, asks kept and broken, where approval comes from, and that no single-verb strategy is the best. It runs in CI beside the bot gates (about 15 s today; a 60 s budget).

---

## 1. What the audit shows, and why

### 1.1 Measurements

The owner's evidence (seeds 777 and 2026), re-run on the current tree, plus 12 more seeds per strategy. `mandate-cli story --years N --strategy S`, small world. "Longest gap" is the longest stretch between two decisions opening.

| Measure (per career) | mixed, 4 y (12 seeds) | idle, 4 y | canvass, 4 y | explorer, 4 y | mixed, 5 y |
|---|---|---|---|---|---|
| Decisions | 25 (19–29) | 22 (18–26) | 25 (22–29) | 25.5 (22–30) | 34.5 (22–38) |
| Distinct templates | 11 (9–16) | 11 (9–14) | 13 (11–17) | 10 (9–16) | 11.5 (9–17) |
| Most uses of one template | 4.5 (4–5) | 5 (3–6) | 4 (4–5) | 4 (4–6) | 6 (5–6) |
| Repeats within a chapter | 1 (0–3) | 1 (0–4) | 1 (0–2) | 1 (0–2) | 1 (0–3) |
| Longest gap, days | 168 (140–182) | 259 (168–301) | 168 (147–259) | 168 (154–224) | 168 (154–210) |
| Storyline templates started | 2 (1–3) | 1 (1–2) | 1 (1–2) | 2 (2–3) | 2 (1–3) |
| Press stories about the player | 0 | 0 | 0 | 0 | 0 |
| Scandals and legal events | 0 | 0 | 0 | 0 | 0 |

Seeds 777 and 2026 (mixed, 4 y) give 27 and 29 decisions from 12 templates each, jobs plea ×5 and pollution ×4 (777), four templates ×4 (2026), longest gap 168 days in both: the same picture as DEC-097's evidence.

Further findings:
- Across 12 mixed careers only 21 of the 31 templates ever appeared. Five carried 60% of all 307 decisions: amendment for a vote (39 uses), building protest (38), personal bills (38), pollution (37), consultancy fee (31).
- **The climax repeats.** Each 4-year career has 4 climaxes from a median of 2.5 templates; in 10 of 12 careers the same template was the climax of two consecutive chapters (seed 5: pollution was the climax four years running).
- **The quiet stretch is structural.** Sessions run January to January. The 2–4 significant decisions come in the first half, the climax in July or August, and nothing follows until the next session opens in January: the 140–182-day gap is the falling action after each climax.
- **Storylines:** the rival every time (once: one campaign in four years); the favour debt in 5 of 12 (only when the player owes a favour); the plant in 4 of 12 (only with a plant-closure crisis).
- **The bot tournament** (`bots --seeds 16`, 18 s): canvass 87.5% and explorer 87.5% (mean share 0.630 each), mixed 81.2% (0.618), legislator, corrupt and proxy 81.2%, media and idle 6.2%. Variety: canvass sole best in 8 worlds, explorer in 6. Every gate passes.

### 1.2 Root causes in the code

| Symptom | Cause |
|---|---|
| Same templates again and again | `director::dilemmas` takes the single highest-relevance candidate (`best`). The same few templates match the town's crisis and the player's ward every time, so they win whenever they are off cooldown. Cooldowns are 90–540 days (`dilemmas.jsonc`), shorter than a career. |
| The climax repeats | The climax skips cooldowns by design ("no cooldown, no relevance floor; repetition still weighs it down", `director.rs`), and the repeat penalty (×0.4) only looks at the current chapter. |
| Repeats within a chapter | Only a ×0.4 penalty, not a rule; the climax can reopen a template from earlier in its chapter. |
| Quiet stretches | The tension curve (`allowed = actual < target + slack`) and the per-chapter budget hold everything back after the climax; nothing but decisions fills a session. |
| One storyline | Three storyline templates, each with a narrow start (a campaign, a favour owed, a plant crisis). No scheme runs between elections. |
| No press about the player | Town news has no subject (`StorySource::Town`, `subject: None`): the player's bills passing are reported without the player's name. Only scandals, cases and press statements name a politician, and the bots never make statements. |
| Canvass is the best routine | A canvass gives awareness (+45% of the gap), approval (+0.02 × room × fit) with no memory of past visits, record (+0.06, as much as a town hall), and in a race a contact whose diminishing fades in about three weeks (`contact_memory` 0.8 a week). Canvass-only does it every turn all year. |

### 1.3 A first tuning probe (evidence for §2)

`bots --seeds 16 --grid actions.canvass.approval=0.02,0.008,0.0 --grid career.record_gain.canvass=0.06,0.03`:

| Canvass approval | Record gain | canvass | explorer | mixed | legislator | Gates failing |
|---|---|---|---|---|---|---|
| 0.02 (today) | 0.06 | 87.5% | 87.5% | 81.2% | 81.2% | none |
| 0.02 | 0.03 | 87.5% | 87.5% | 81.2% | 81.2% | none |
| 0.008 | 0.06 | 75.0% | 81.2% | 81.2% | 81.2% | variety |
| 0.008 | 0.03 | 68.8% | 81.2% | 81.2% | 81.2% | variety |
| 0 | 0.06 or 0.03 | 68.8% | 75.0% | 75.0% | 81.2% | variety |

At 0.008 / 0.03 the canvass bot's mean share falls 4.7 points (0.630 → 0.583), mixed 2.0 (0.618 → 0.598), explorer 2.0 (0.630 → 0.610), and the explorer becomes the best in 13 of 16 worlds (variety fails: 81% > 60%).

A second grid, `--grid politics.campaign.contact_memory=0.8,0.9 --grid actions.canvass.approval=0.02,0.012`:

| Canvass approval | Contact memory | canvass | explorer | mixed | legislator | Top among-best share | Gates failing |
|---|---|---|---|---|---|---|---|
| 0.02 | 0.8 (today) | 87.5% | 87.5% | 81.2% | 81.2% | 0.50 | none |
| 0.012 | 0.8 | 81.2% | 81.2% | 81.2% | 81.2% | 0.69 | variety |
| 0.02 | 0.9 | 81.2% | 75.0% | 75.0% | 68.8% | 0.62 | variety |
| 0.012 | 0.9 | 81.2% | 75.0% | 75.0% | 68.8% | 0.62 | variety |

A longer race-contact memory hurts the strategies that canvass hard in the campaign (mixed, explorer) more than canvass-only, which built its awareness all year: the wrong direction. It stays at 0.8.

**Lesson:** a flat cut hurts every strategy that canvasses in the campaign and hands the lead to the next all-rounder. Since the owner's balance principle (DEC-097, §7.2), the explorer taking the lead is the *intended* result, not a failure. The "variety" failures in these grids are an artefact of the old gate, which counted the intended way of playing as one routine among many. The rebalance must cut *repeated, out-of-season* canvassing (what canvass-only does and mixed does not), and it must land with new, world-dependent sources of approval (asks, delivery), or the variety gate fails. §2 is designed that way; E1 is tuned with E6–E8 in view.

---

## 2. The ground game (DEC-097 §1)

### 2.1 The canvass curve

A canvass of neighbourhood *h* now reads a per-neighbourhood memory kept by the player domain:

```text
visits_now  = visits[h] × keep ^ (days since last canvass of h / 30)
fresh       = hood_decay ^ visits_now                      (1 for a neighbourhood never canvassed)
season      = campaign_mult if the player is a candidate in an open race, else 1
same_turn   = same_turn_mult if the player already canvassed this turn, else 1

awareness  += awareness × (1 − aw) × max(fresh, awareness_floor)      (awareness first)
approval   += approval × fit × room × fresh × season × same_turn      (fit: canvass_match, K4)
race contact scale = contact × same_turn                             (unchanged otherwise: the race model has its own diminishing)
record     += record_gain.canvass                                      (lower, §2.2)
visits[h]   = visits_now + 1
```

In campaign season `hood_decay` is replaced by `hood_decay_campaign` (doorsteps are expected; each visit counts for more).

| Constant (`content/base/actions.jsonc` → `canvass`) | Today | **Proposed** start | Grid for tuning |
|---|---|---|---|
| `awareness` | 0.45 | 0.45 | — |
| `approval` | 0.02 | 0.012 | 0.008–0.016 |
| `contact` | 1.6 | 1.6 | — |
| `hood_decay` (new) | — | 0.5 | 0.4–0.7 |
| `hood_decay_campaign` (new) | — | 0.75 | 0.6–0.9 |
| `keep` (new, monthly) | — | 0.6 | 0.5–0.75 |
| `campaign_mult` (new) | — | 2.0 | 1.5–2.5 |
| `same_turn_mult` (new) | — | 0.5 | — |
| `awareness_floor` (new) | — | 0.5 | — |
| `politics.campaign.contact_memory` | 0.8 | 0.8 (unchanged, §1.3) | — |

What this does (14-day governing turns, one canvass a turn):
- The same neighbourhood every turn: `fresh` ≈ 0.08 at steady state. Approval per canvass ≈ 0.001.
- Rotating three neighbourhoods (each every six weeks): `fresh` ≈ 0.5, approval ≈ 0.006 per canvass (today 0.02).
- In a campaign, weekly turns rotating three neighbourhoods: `fresh` ≈ 0.52 with `hood_decay_campaign`, approval ≈ 0.012 per canvass, plus the race contact, as today.
- A first visit to a neighbourhood is worth as much as today in awareness, and 0.6× today's approval outside a campaign (1.2× in one).

The canvass outcome line says how fresh the doorsteps were ("people here saw you last month"), so the curve is visible (DEC-051 "Why?"). The pocket's canvass route shows the same band before acting.

### 2.2 Record

| Constant (`content/base/career.jsonc` → `record_gain`) | Today | **Proposed** |
|---|---|---|
| `canvass` | 0.06 | 0.03 (presence, not delivery) |
| `ask_kept` (new) | — | 0.05 |
| `promise_kept` (new, a campaign promise) | — | 0.08 |
| `ask_broken` (new) | — | −0.04 |
| `law_enacted` (new: the player's own bill becomes law) | — | 0.08 |

A thin record still punishes idling (DEC-076); canvass-only keeps a record above the 0.35 baseline (two canvasses a month give 0.06 against a fade of 8%), so it stays well above idle.

### 2.3 Where approval comes from instead

| Source | Size (approval units × room, per cohort in scope) | Scope | Owner |
|---|---|---|---|
| Canvass | 0.006 typical between elections, 0.012 in a campaign | The neighbourhood | actions (E1) |
| An ask kept | 0.015 (a person's ask), 0.006 (an organisation's, ward-wide) | The asker's neighbourhood, or the ward | asks (E6) |
| Casework kept (help to an individual) | 0.012, plus the person +10 and `helped_me` | The person's neighbourhood | asks (E6) |
| A campaign promise kept | 0.02; broken −0.03 and a press story | The ward | asks (E6), media (E5) |
| Press about the player's work | `perception.approval` 0.03 × `work` weight 0.3 × heat × exposure × credibility × framing (≈ 0.003–0.006 a story) | The outlet's audience | media (E5) |
| A law's credit or blame (V1) | Set in V1; target 0.03–0.06 a year in the neighbourhoods a well-received law served | Per neighbourhood | politics (V1) |
| Reactions to the player's bills | Unchanged (`opinion.player_approval_rate`) | Cohorts aware of the player | population |

**Target composition** (lab, mixed bot, positive approval gained between elections): canvassing ≤ 40%; asks, casework and promises ≥ 30%; bills, laws and press ≥ 20%. Measured by the engagement command (§7), which attributes each command's change in weighted approval to that command and the monthly update to "laws and drift". It is a lab-side measurement: nothing new in state.

### 2.4 Bots

- `canvass` stays as it is (the probe of the single-verb routine).
- `mixed` and `explorer` accept asks they can meet and fulfil them (§3.9); in campaign season they canvass as today.
- New `asks_only` (a single-verb routine: accepts and fulfils asks, nothing else) so the gate also stops asks from becoming the new dominant verb.

---

## 3. Asks and promises (DEC-097 §2)

### 3.1 What an ask is

An ask is a concrete request with a deadline, made to the player by someone with a reason in the live state: "Fix the drain in Millside" (casework), "Vote with us on the budget", "Say something about the closure", "Come to the traders' meeting before the vote". The player **accepts** (a promise), **declines**, or lets it lapse unanswered (treated as declined). An accepted ask is **kept** when the engine sees the player do what it needs before the deadline, **broken** when the deadline passes, and **void** when it becomes impossible through no fault of the player (the bill was withdrawn, the asker left town).

Asks are a goals layer over DEC-032: an NPC goal (`goals.jsonc`) can name an ask instead of a request dilemma. Requests stay for choices made on the spot; anything that needs doing later becomes an ask. Today's three requests all need doing later, so all three convert: `request.ally_vote` → `ask.vote_with_me`, `request.traders_meeting` → `ask.attend_meeting`, `request.back_the_workers` → `ask.join_picket`. That also removes `request.traders_meeting` from the director's repeats (20 uses in 12 careers).

**No fetch quests (DEC-032).** Every ask is met by an ordinary command checked by the engine (a vote, a statement, a canvass, a town hall, a visit, a talk, a draft, an introduction) or by casework, the one new verb (§3.4). There is no "go to X and come back".

### 3.2 Who asks, and when

| Asker | Typical asks | Comes from |
|---|---|---|
| A resident (Named, met or on the doorstep) | Casework: a drain, a benefits form, a planning objection, a school place | A canvass (chance `asks.params.doorstep_chance` 0.35 a canvass), a talk, an encounter (§6.5) |
| A councillor or the mayor | Vote with me; back my amendment; second my bill | Goals `seek_backing`, `champion_fix` |
| The party (chair or whip) | Vote the party line on the budget; appear at a colleague's event; fundraise | Party patience, the budget, a campaign |
| The broker (DEC-096) | Vote for or against a bill touching their stake; meet their people | P1 broker alignment |
| Organisations (union, business association, clergy, organisers) | Join the picket; come to the meeting; hold a town hall here; draft an answer to the crisis | Goals `organise_workers`, `traders_help`; the live crisis |
| The press (a journalist) | Say something about the closure (a statement within 7–14 days); an interview | A crisis, a story about the town |
| The ward's voters | Campaign promises (§3.5) | The player, while campaigning |

Generation (weekly, the asks system, player's town only): each template whose triggers hold and whose asker can be cast is a candidate; candidates are weighted by `weight` × the asker's relationship and goal × the concern's salience, and drawn with a keyed draw (`Stream::Asks`, keyed by week and template). Caps keep it from becoming a to-do list:

| Parameter (`content/base/asks/params.jsonc`) | **Proposed** |
|---|---|
| `max_open` (offered + accepted) | 4 |
| `new_per_month` | 1.5 governing, 2.5 in a campaign (a mean; keyed draw) |
| `respond_days` (an offer lapses unanswered) | 7 |
| `per_asker_cooldown_days` | 60 |
| `doorstep_chance` (a canvass brings a casework ask) | 0.35 |
| `history` (resolved asks kept for views and the lab) | 60 |

**The world does not wait (DEC-091).** Asks are aimed at the player, so they are never gated by knowledge. Their triggers may not use `knows` (K6's validation rule extends to ask triggers). Whether an ask exists never depends on what the player knows; an ask can teach the player about its asker (`person known`).

### 3.3 Rewards and costs

| Outcome | Asker | Player | Others |
|---|---|---|---|
| Accepted | memory `promised_favour` | — | — |
| Kept | opinion +6 to +12, memory `kept_promise`; a big ask: a favour owed to the player | record +0.05; approval (§2.3); sometimes knowledge (the asker tells what they know: a secret `heard`, §5) or an endorsement (V2) | a press story if public (§4.6) |
| Broken | opinion −10 to −15, memory `broke_promise` (feeds `goal.oppose_player`) | record −0.04; if public: approval −0.02 to −0.03 in scope, Standing −1, a press story, rival material | — |
| Declined | opinion −3 (no memory) | — | — |
| Void | — | — | — |

Sizes are per template in content (effects reuse the narrative `EffectDef` list, so validation already checks them). Every template must have a cost on breaking and a gain on keeping (DEC-073 validation, as for dilemmas).

### 3.4 Casework: help to individuals

Casework is the "fix the drain" ask. It is met by one new located action, **Sort it out** (`Action::Casework { ask }`), done at the town hall (the council's services desk) for 1 slot.

- In office it succeeds with chance `casework.success_in_office` 0.85; out of office (you can only chase it up) 0.5. The odds are shown before acting (L1 records them).
- A failure leaves the ask open; a second try costs another slot.
- There is no casework without an ask, so it is not a new grind verb: the number of cases is bounded by the asks system.

### 3.5 Campaign promises

While campaigning (from filing to election day), the player may make up to `promises.max_per_campaign` 3 public promises, chosen from a menu built from the ward's live state: its top concerns, the town's crisis, open proposals (DEC-092 families, once F2b lands), and the player's own bills. Each promise is an ask whose asker is the ward's voters, with `public: true` and a deadline of the next budget year's end or a term.

| Promise template (examples) | Kept by |
|---|---|
| "I'll get the flood wall for {hood} on the books" | The player drafts and introduces the family (or template) before the deadline; kept when it is enacted, half-kept (no cost, no reward) if it failed a vote the player sponsored |
| "No rise in the town's taxes" | The player votes against every tax rise until the deadline (broken by a yes vote or sponsoring one) |
| "A town hall in every neighbourhood of the ward" | Town halls in each ward neighbourhood within 180 days |
| "Jobs first" | The player sponsors a bill answering `concern.jobs` that is enacted |
| "Clean politics" | No hidden fact of the player's own surfaces before the deadline (an honest promise that a shady player can break by being found out) |

During the campaign a promise adds persuasion in the neighbourhoods where its concern is salient: `promises.persuasion` 0.04 × salience, discounted by the player's broken public promises (`× 0.5` per broken one, from the asks history). Promises are where the campaign and governing connect: what was promised in November is checked by the next budget.

### 3.6 Engine: domain, types, commands

A new domain, `mandate_sim::asks` (its own state; it writes only that state; everything else goes through effect requests, OVERVIEW §4.3).

```rust
// crates/mandate-sim/src/asks/mod.rs (new)
mandate_core::define_id!(AskId);   // typed u32, never reused (DEC-041)

pub struct AskState {
    pub asks: Vec<Ask>,                         // open asks, then resolved ones (bounded by `history`)
    pub ids: mandate_core::IdAllocator,
    #[serde(with = "map_as_pairs")]
    pub cooldowns: BTreeMap<ContentKey, SimDate>,   // template → earliest return
    pub asker_cooldowns: Vec<(Asker, SimDate)>,
    pub promises_this_campaign: u8,
    pub last_event: u32,                        // event-log position read up to (as media)
}

pub struct Ask {
    pub id: AskId,
    pub template: ContentKey,
    pub asker: Asker,
    pub cast: Vec<CastEntry>,                   // narrative cast entries: hood, bill, place, person
    pub need: Need,                             // resolved from the template's `needs` and the cast
    pub public: bool,
    pub opened: SimDate,
    pub respond_by: SimDate,
    pub due: SimDate,
    pub status: AskStatus,
}

pub enum Asker { Person { id: CharacterId }, Party { id: PartyId }, Outlet { id: u32 }, Ward { id: ConstituencyId } }

pub enum AskStatus {
    Offered,
    Accepted { on: SimDate },
    Kept { on: SimDate },
    Broken { on: SimDate },
    Declined { on: SimDate },     // also an unanswered offer
    Void { on: SimDate, reason: ContentKey },
}

/// What keeps the ask: one of the player's deeds (below), or a state test.
pub enum Need {
    Vote { policy: u32, vote: Vote },
    VoteAgainstAll { tag: ContentKey },         // "no tax rise": a broken-by rule
    Statement { about: StatementAbout },        // a press statement on a policy, a crisis or a concern
    TownHall { hoods: Vec<NeighborhoodId> },    // every one of them
    Canvass { hood: NeighborhoodId },
    Visit { place: PlaceId },                   // be there (the meeting) before the due date
    Talk { person: CharacterId },
    Introduce { template_or_family: ContentKey },
    Enacted { template_or_family: ContentKey, sponsor_player: bool },
    Casework,
    NoSurfacedHiddenFact,                       // "clean politics"
}
```

**Deeds.** The asks system learns what the player did from the event log, as media does: command handlers append `EventKind::PlayerDeed { deed: Deed }` (new; not shown as news) for votes, statements, town halls, canvasses, arrivals at places, talks, introductions and casework. Laws enacted and hidden facts surfacing are already events. No command handler writes the asks domain.

**Commands** (`PlayerCommand`, appended):

```rust
RespondAsk { ask: u32, accept: bool },           // free
Promise { template: String, args: Vec<(String, String)> },   // campaigning only; free; at most 3
// Action (located, at the town hall): Casework { ask: u32 }   — 1 slot
```

**Systems.** `asks::weekly` (Narrative phase, player's town): read new deeds and events, resolve kept, broken and void asks, generate offers. Unlike a dilemma answer (applied inside the `Answer` command by `narrative::resolve`), asks resolve in a system, so their `EffectDef` lists go out as effect requests: the existing `Remember` and `Memory` (memories), `PlayerTrack` (Standing), `Learn` and `Lead` (the ledger); new `Opinion { holder, toward, delta }` and `Favour { holder, owed_to_player }` (characters), `Approval { scope, delta }` (population: a neighbourhood, the ward or the town, × room as today), `Record { politician, delta }` (politics); `Endorse` arrives with V2. The director reads open asks (they count as minor beats and as pending load in tension) but never writes them.

**Validation** (`mandate-cli content validate`): every template has a keep reward and a break cost; `needs` refer to cast roles of the right type; deadlines are within [7, 900] days; no `knows` in triggers; Fluent keys exist with the arguments the cast provides; every promise template names the concern or crisis it answers.

### 3.7 Content format (one example)

```jsonc
// content/base/asks/asks.jsonc
{ "id": "ask.say_something", "name_key": "ask-say-something",
  "asker": { "type": "occupation", "occupations": ["occupation.journalist"] },
  "roles": [ { "id": "outlet", "cast": { "type": "outlet_of", "role": "asker" } } ],
  "triggers": [ { "type": "live_crisis", "min_severity": 0.3 } ],
  "needs": { "type": "statement", "about": "crisis" },
  "deadline_days": [7, 14], "weight": 1.0, "cooldown_days": 120, "public": true,
  "keep":    [ { "type": "opinion", "role": "asker", "delta": 6, "memory": "kept_promise" },
               { "type": "awareness", "delta": 0.01 } ],
  "break":   [ { "type": "opinion", "role": "asker", "delta": -10, "memory": "broke_promise" } ],
  "decline": [ { "type": "opinion", "role": "asker", "delta": -3 } ],
  "detail_key": "ask-say-something-detail", "kept_key": "ask-say-something-kept", "broken_key": "ask-say-something-broken" },
```

### 3.8 The first pack

16 asks and 6 promises. Each is grounded in a live condition.

| Ask | Asker | Trigger | Needs | Days |
|---|---|---|---|---|
| `ask.casework` (drain, form, objection, school place: a Fluent variant by the person's situation) | Resident | Doorstep, talk, encounter | Casework | 21–35 |
| `ask.vote_with_me` (was `request.ally_vote`) | Councillor with `seek_backing` | Their bill before the council | Vote yes | Until the vote |
| `ask.party_budget` | Party chair | Budget before the council | Vote yes on the budget | Until the vote |
| `ask.colleague_event` | Party chair | Campaign; a colleague in a close race | Visit that ward's streets | 14 |
| `ask.say_something` | Journalist | Live crisis ≥ 0.3 | Statement on the crisis | 7–14 |
| `ask.attend_meeting` (was `request.traders_meeting`) | Shopkeeper, business leader | `concern.jobs` top | Visit the business association or market | 21–28 |
| `ask.join_picket` (was `request.back_the_workers`) | Union steward | Plant-closure crisis | Visit the union hall, then a statement | 14–21 |
| `ask.town_hall_here` | Organiser, clergy | A neighbourhood's top worry high, no town hall there in 180 days | Town hall in that neighbourhood | 30–45 |
| `ask.draft_answer` | Organisation leader | Live crisis; an answer on the menu (a situational template, later a family) | Introduce it | 60–90 |
| `ask.broker_vote` | The broker | A bill touching the broker's stake | Vote as asked | Until the vote |
| `ask.visit_after_flood` | Resident of a struck neighbourhood | A disaster within 30 days (G1 exposure) | Canvass or town hall there | 14 |
| `ask.second_my_bill` | Councillor with `champion_fix` | Their crisis bill drafted | Talk to them, then vote yes | Until the vote |
| `ask.fund_the_school` | Teacher, nurse | `concern.services` top; a budget coming | Statement on services, vote for the services line | Until the budget |
| `ask.meet_the_newcomer` | Business leader | A new employer in town (sector capacity rising) | Talk to them | 30 |
| `ask.keep_it_quiet` | Someone with a secret (§5) | The player dug into it or asked about it (a deed the subject hears of, never the ledger) | No statement or press tip about it | 90 (kept by not acting) |
| `ask.introduce_me` | Resident with a goal | A councillor they want to meet | Talk to the councillor, then the asker | 30 |

| Promise | Answers |
|---|---|
| `promise.answer_crisis` | The town's live crisis (the flood wall, the rescue deal, the rent measure: the answer is a menu argument) |
| `promise.no_tax_rise` | `concern.taxes` |
| `promise.town_halls` | Any (a promise of presence) |
| `promise.jobs` | `concern.jobs` |
| `promise.services` | `concern.services` |
| `promise.clean_politics` | Any |

### 3.9 Protocol, Today tab, bots

```rust
// crates/mandate-protocol/src/asks.rs (new)
pub struct AskView {
    pub id: u32,
    pub template: String,
    pub title_key: String,
    pub detail_key: String,
    pub args: BTreeMap<String, String>,
    pub asker: AskerView,                 // a PersonRefView, the party, an outlet, or the ward
    pub status: String,                   // offered | accepted | kept | broken | declined | void
    pub public: bool,
    pub respond_by: Option<DateView>,     // offered only
    pub due: DateView,
    pub days_left: u32,
    pub need_key: String,                 // "Vote yes on {bill}" …
    pub progress: Option<String>,         // "2 of 3 neighbourhoods"
    pub how: Option<RouteView>,           // where the need is done (auto-routing, DEC-079)
    pub odds: Option<f64>,                // casework only
    pub keep_lines: Vec<EffectLineView>,
    pub break_lines: Vec<EffectLineView>,
}
// PocketView gains: asks: Vec<AskView> (offered, then accepted by due date), asks_due_soon: u32 (badge)
// QueryDto::Asks → ResponseBody::Asks(AsksView { open, recent, promises, kept, broken })
// QueryDto::PromiseMenu → ResponseBody::PromiseMenu(Vec<PromiseOptionView>)  (campaigning only)
```

- **Today tab:** an "Asks and promises" block under Decisions: offers with Accept and Decline, promises with the deadline, progress and a "Go" route, and the last few outcomes. The badge counts offers and promises due within a turn.
- **Story sheet:** promises made, kept and broken (the run's record).
- **Bots:** `mixed` and `explorer` accept an ask when they can meet it within the deadline by their own routine (votes and statements always; visits and town halls if in reach; casework when a slot is free), and decline the rest; `asks_only` accepts and works every ask; `idle` never answers (offers lapse).

### 3.10 Save

`State.asks` is new (serde default: empty). One save bump; the migration step leaves the default. `Stream::Asks` joins the named streams (OVERVIEW §3). `PlayerDeed` events are bounded with the event log. Size: at most 4 open plus 60 resolved asks, a few kilobytes.

---

## 4. Storyteller variety (DEC-097 §3)

### 4.1 Cooldowns and no repeats

New director rules (`content/base/director.jsonc`, `DirectorParams`):

| Field | **Proposed** | Rule |
|---|---|---|
| `cooldown_floor_days` | 365 | Every dilemma's cooldown is at least this |
| `cooldown_growth` | 2.0 | The *n*-th use sets a cooldown of `max(floor, template cooldown) × growth^(n−1)` |
| `no_repeat_in_chapter` | true | A template spotlighted in this chapter is not a candidate again in it, the climax included |
| `climax_memory_chapters` | 2 | A template that was the climax in either of the last two chapters cannot be the climax |
| Climax and cooldowns | — | The climax respects cooldowns (today it skips them). It still skips the relevance floor |

`NarrativeState` gains `uses: BTreeMap<ContentKey, u16>` (times each template opened this run) and `climaxes: Vec<ContentKey>` (the last few). With these, a template can appear at most three times in five years (0, 365, 1,095 days).

### 4.2 Choosing among candidates

`best` becomes `pick`: candidates within `pick_band` 0.8 of the best relevance, weighted by `relevance × novelty`, where `novelty = 1 / (1 + uses)`, drawn on the director stream with a keyed draw (date, chapter). The draw count never depends on the player's knowledge (DEC-091 §1.11). The climax uses the same pick within its window.

### 4.3 Filling the falling action

- Budgets: significant `[2, 4]` → `[3, 5]` per chapter.
- `post_climax_significant` 1: after the climax, one more significant decision may come, but only a consequence template (§4.4) or an opportunity.
- `quiet_days` 60: if nothing has happened for the player (no decision opened, no ask offered or resolved, no storyline stage, no press story about the player) for this long, the tension gate is lifted for one significant decision or one ask offer (still within the chapter's maximum). Asks (§3) and work stories (§4.6) fill most of the gap on their own.

### 4.4 A bigger pool, with consequences

**A new condition** for dilemma and storyline triggers, `answered`, reads the player's past answers: `{ "type": "answered", "template": "dilemma.jobs_plea", "option": "wage_subsidy", "min_days": 120, "max_days": 720 }`. Its cast can carry over (`same_cast: ["voice"]`). `NarrativeState` gains `answers: Vec<AnswerRecord { template, option, date, cast }>` (bounded at 200; the bounded `decisions` history is too short for a 5-year look-back). A companion condition `sponsored_law { status, min_age_days }` reads the player's laws.

**22 new dilemmas** (31 → 53 templates). Ten follow from the player's own choices:

| Template | Follows | The trade-off |
|---|---|---|
| `dilemma.subsidy_audit` | `jobs_plea` → wage subsidy | The auditor finds firms gamed it: claw back (jobs at risk) or defend it (the press) |
| `dilemma.inspection_fallout` | `pollution` → inspection | The plant threatens to cut shifts unless the order is softened |
| `dilemma.report_resurfaces` | `pollution` → bury the report | A whistle-blower has the report (a secret, §5): get ahead of it or deny |
| `dilemma.project_overrun` | `amendment_for_vote` → their project | The colleague's ward project overruns: pay or cut, and their vote next time |
| `dilemma.tower_objection` | `housing_squeeze` → upzone, or an upzoning law in force | Residents object to a tower on their street |
| `dilemma.relief_partner` | `disaster_relief` → relief drive | The charity partner is accused of skimming |
| `dilemma.donor_returns` | `donor_offer` → accept | The donor wants a planning favour |
| `dilemma.truce_tested` | `rival_offer` → truce | The rival's ally attacks the player: does the pact hold? |
| `dilemma.law_anniversary` | A law the player sponsored, a year in force | Mixed results (V1's track record when built): extend, amend, or let it sunset |
| `dilemma.promise_called_out` | A broken public promise (§3.5) | The press asks: apologise, blame others, or say nothing |

Twelve more from the town's live state and the new systems:

| Template | Trigger | Notes |
|---|---|---|
| `dilemma.school_closure` | Falling rolls in a declining neighbourhood (baseline population, B1) | |
| `dilemma.clinic_hours` | Budget stress, `concern.services` | |
| `dilemma.market_permit` | The market's trade falling; a festival proposal | Noise against trade; a minor-weight option set |
| `dilemma.bus_route_cut` | Budget stress; a neighbourhood far from the centre | |
| `dilemma.heritage_building` | A housing or development bill touching a neighbourhood with an old-building curiosity (a plaque or foundation stone, §6.2: engine-side) | Links to walking lore (§6); the layout stays on the client |
| `dilemma.mine_safety` | A working mine (G3) | Already planned with G3; listed for the count |
| `dilemma.councillor_debts` | A `councillor_debts` secret (§5) whose creditor is pressing the councillor before a vote | The trigger is the debt, never the ledger; `knows` only picks the variant (a player who knows decides whether to use or shield it; one who does not is approached by the creditor) |
| `dilemma.journalist_source` | A journalist investigating a secret (§5) | Confirm, deny, or trade |
| `dilemma.endorsement_price` | An endorser (V2) wants a policy for their backing | After V2 |
| `dilemma.broker_ultimatum` | The broker opposes a player bill (P1) | After P1 |
| `dilemma.strike_vote` | Plant-closure crisis, the union organising | |
| `dilemma.minor_party_courts` | A minor party strong in the player's ward | |

Templates after E3: 53 (31 + 22); after E8 turns the three requests into asks, 50 dilemmas. With E4 and E8: 9 storylines, 16 asks, 6 promises.

### 4.5 Storylines: 2–4 per career, NPC schemes in parallel

| Field (`director.jsonc`) | Today | **Proposed** |
|---|---|---|
| `storylines_active` | 2 | 3 |
| `storyline_spacing_days` (new) | — | 90 (two arcs never start within 90 days) |
| `min_storylines` (new) | — | 1 non-rival storyline active whenever one can start |

Six new storylines:

| Storyline | Starts when | Driven by |
|---|---|---|
| `storyline.rival_scheme` | Between campaigns, the rival has the `oppose_player` goal or is the next candidate | The rival's NPC AI: courting the player's allies, lobbying against the player's bills, a story about the player's record. The rival arc no longer appears only in campaigns |
| `storyline.broker_scheme` | The broker has a stake and a bill touching it (P1) | The broker's lobbying, funding and endorsements (DEC-096) |
| `storyline.newcomer` | A new employer considers the town (sector capacity rising) | Site, tax ask, local protest |
| `storyline.investigation` | A journalist starts on a town secret (§5) | Media discovery; the player can help or hinder |
| `storyline.flood_recovery` | A disaster in the player's town (G1) | Relief, the inquiry, the defences |
| `storyline.big_promise` | A campaign promise of `promise.answer_crisis` | The promise's ask: the draft, the vote, the delivery or failure |

**NPC schemes are presented, not invented (DEC-033 B).** A scheme's stages advance on what NPCs actually do through their own AI (DEC-030). A new storyline condition reads a bounded log of NPC deeds kept by the NPC domain:

```jsonc
{ "type": "npc_did", "role": "rival", "deed": "lobby_against", "target": "player_bill", "within_days": 60 }
// deeds: lobby_for, lobby_against, endorse, fund, statement, talk_to (a cast person), file
```

`PoliticsState` (or the characters domain, whichever `npc.rs` writes) gains `npc_deeds: Vec<NpcDeed>` (bounded at 200, the player's town only). Schemes run whether or not the player knows; their stage views are gated by the ledger (`RivalPlans`, `Broker`, K2), shown as leads until learned.

### 4.6 The press covers the player's work

A new story source, symmetric for every politician (DEC-030):

```rust
StorySource::Work { politician: PoliticianId, kind: WorkKind, policy: Option<u32>, ask: Option<u32> }
pub enum WorkKind { BillPassed, BillFailed, LawResult, PromiseKept, PromiseBroken, Appearance }
```

| Kind | From | Newsworthiness (`media.coverage.work`) |
|---|---|---|
| Bill passed or failed | `PolicyEnacted` / `PolicyFailed` (replaces the subject-less town news for bills) | 0.5 / 0.4 |
| Law result | V1's track record, a year on (until V1: the law's attribution ledger crossing a threshold) | 0.6 |
| Promise kept / broken | The asks system (public asks only) | 0.45 / 0.7 |
| Appearance | A town hall, a statement, a meeting kept (an ask) | 0.3 |

Perception uses the existing formula with a new severity-like weight `perception.work` 0.3 and framing by the outlet's lean. **Target** (mixed bot): at least 6 stories about the player in a 5-year career, at least half of them about work. Outlet capacity is unchanged, so scandals still lead when there are any.

---

## 5. Secrets (DEC-097 §4)

### 5.1 What a secret is

A secret is a hidden fact (DEC-034) about a person in the player's town, seeded from live state by a content registry. It reuses `narrative.hidden`: subject, evidence, discovery window and fate. Journalists, the rival, auditors and investigators discover secrets by the existing rules, so a secret can surface whether or not the player finds it. Not every secret is a misdeed: `severity` is optional (a private matter has no legal path but still has newsworthiness).

### 5.2 Kinds (the first registry)

| Kind | Seeded from | Subject | Who knows (for conversations) | Severity |
|---|---|---|---|---|
| `secret.broker_deal` | P1: the broker's lobbying of a councillor succeeds with a favour or money | The councillor and the broker | Their staff, the union steward, the diner owner | misconduct |
| `secret.buried_report` | Pollution over a threshold with a plant or mine owner; or the `bury_report` dilemma option | The owner (or the player) | Plant workers, a nurse, the clinic | misconduct |
| `secret.rival_backer` | Campaign money from the broker or a donor to the rival (P1, the funds path) | The rival | The party chair, the backer's circle | ethics |
| `secret.councillor_debts` | At play start and monthly, a councillor with low integrity and high ambition (keyed per person) | The councillor | Their creditor, the diner owner | private |
| `secret.steered_contract` | NPC corruption's existing hidden facts (`steer_contract` kinds) | The NPC | Contractors, the town clerk | crime |
| `secret.planned_closure` | A plant at risk by the same inputs the economy uses to close plants (its sector's job-loss trend and margins) | The plant owner | Managers, the union steward (rumour) | private |
| `secret.land_deal` | Someone bought land beside a site before a rezoning bill (a family binding a neighbourhood) | The buyer and a councillor | The estate agent, the planning clerk | ethics |
| `secret.party_challenge` | A faction plans to deselect the mayor or the player (party Standing) | The faction leader | Party members | private |

Caps: at most `secrets.max_live_per_town` 6 unsurfaced secrets; seeding draws on a keyed stream (`Stream::Media`, keyed by month, kind and cast). Seed triggers may not use `knows` (K6).

### 5.3 How the player finds them

| Route | Level reached | Cost |
|---|---|---|
| Talk (DEC-093): someone in the kind's `knows` list, within their discretion (M6 §3.4) | Cool or neutral: `heard` (a rumour); warm: `known`; a friend with trust: `detailed` | Half a slot (the talk) |
| Overheard lines and street encounters (§6.5) | `heard`, with a lead | Free |
| **Dig into it** (`Action::Investigate { secret }`) at the news office (with a journalist met), the town hall records, or the old union archive (a hidden place, G2) | `known` → `detailed`; odds `0.35 + 0.3 × intrigue + 0.2 if known`, shown first (L1 records them) | 1 slot |
| A storyline step or a dilemma option (`learn`) | As stated | — |

Detail needs evidence: `detailed` takes a second independent source or a successful dig. The notebook lists each secret with where it was learned and a lead to the next level (M6 §1.8).

### 5.4 What a known secret does (never decisive alone)

- **Tip off the press** (`Action::TipPress { secret }`, news office, 1 slot, needs `known`). The fact's fate becomes discovered by a journalist now; coverage, perception and the legal path follow the existing rules. The subject learns the source with chance `secrets.source_exposed` 0.25 (memory `exposed_me`).
- **Framing.** Dilemmas, scenes and lines may pick a variant by `knows` (allowed: framing, DEC-091). A known `planned_closure` lets the player act early (draft the closure response before the crisis).
- **Asks.** `ask.keep_it_quiet` (§3.8) and a dilemma `councillor_debts`.
- **No direct leverage in M6.** Using a secret to pressure someone is a shady act; **Proposed** for later (§9, question 4).

**Caps and the gate.** A probe bot `secret_hunter` (mixed plus every dig and tip it can) must not beat `mixed`'s mean first-election share by more than 1.0 point; an omniscient run's secrets may not move `mixed` by more than 1.0 point either.

### 5.5 Engine and save

- `HiddenFact` gains `secret: Option<ContentKey>` (the kind) and `cast: Vec<CastEntry>`; save bump with defaults. Recommended at the same time: a typed `HiddenId` (M6 §6 item 12).
- A `secrets` step in the narrative domain seeds them monthly (it writes its own `hidden` list). Seeds from other domains (P1's deals, NPC corruption) arrive as effect requests (`Effect::Hide { kind, cast, evidence }`).
- `talk_params.speaker_knows` gains `KnownSubject::Secret { kinds }` rules from each kind's `knows` list; C3-style lines teach `secret` subjects.
- `Subject::Secret { hidden }` exists already (K1).

---

## 6. Walking rewards (DEC-097 §5)

### 6.1 The kinds registry

`content/base/curiosities.jsonc`, one entry per kind:

```jsonc
{ "id": "curiosity.plaque", "name_key": "curiosity-plaque", "category": "lore",
  "anchors": ["wall", "bridge", "square"],          // where the client may place it, in order of preference
  "eligible": [ { "type": "history", "fact": "past_mayor" } ],   // the town must have this to say
  "per_town": [1, 3], "weight": 1.0,
  "reveal": "walk",                                  // walk | streets
  "when": null,                                      // optional: slots (evening), a season, a storyline step
  "art": "prop.plaque",
  "text_key": "curiosity-plaque-text",               // args from the instance (a name, a year)
  "reward": [] }
```

| Category | Kinds (first pack: 14) | Reward | `reveal` |
|---|---|---|---|
| Lore | plaque (a past mayor, a year), mural (the defining industry), old campaign poster (a past race and its result), bench with a story (a disaster year, a person), foundation stone (the founding era), war or pit memorial | A notebook lore entry, nothing else (a kind that also teaches something, such as a dedication naming a living former mayor, is a separate `streets` kind) | walk |
| Find | lost cat, a petition on a noticeboard, slogan on a wall | Cat: a casework-like ask from its owner (return it: owner +10, `helped_me`). Petition: an `ask.town_hall_here` offer and the neighbourhood's worries `heard`. Slogan: the neighbourhood's top worry `heard` | streets |
| Vista | the view from the hill (highest level), the harbour wall at dusk | A dated `read` of the neighbourhood worries the player already knows (as the town data desk; once per 90 days), health +3 once a month | streets |
| Easter egg | a shop sign, a poster for a long-gone band, a cat that is always on the same wall | Nothing (in-world; heightened realism, DEC-061) | walk |

Rare **encounters** (§6.5) are people, not registry entries.

Variety by town: kinds are eligible only where the town can ground them (`eligible` conditions on history, industry, geography: a pit memorial needs a mine, the harbour wall a coast), and `per_town` with keyed draws gives each town its own set. The registry is data, so packs add kinds with no code; anchors are the one extension point (code, §6.3).

### 6.2 Seeding (engine, deterministic)

At play start, and when the player enters a new zone (M7), the engine creates curiosity instances for the town: `world.curiosities: Vec<Curiosity>` (append-only, typed `CuriosityId`, never reused; created after all places so place ids never shift, as TOWN_GENERATION §11 requires for optional places).

```rust
pub struct Curiosity {
    pub id: CuriosityId,
    pub kind: ContentKey,
    pub town: MunicipalityId,
    pub hood: NeighborhoodId,
    pub args: BTreeMap<String, String>,   // facts from history: a name, a year, a result
    pub subject: Option<CastEntry>,       // a person or place it teaches about
}
```

- Density: `curiosities.per_district` 1.5 (12–30 per town), at most 40% `streets` (play-affecting).
- Draws: `stream(seed, "curiosity", town, kind)` keyed draws; neighbourhoods weighted by their anchors' likelihood (a vista needs high ground: the engine reads `TownGeographyView`'s relief, B2).
- Arguments come from the simulation's own history (the burn-in's mayors, elections, disasters and plant closures): lore is true to the town.

### 6.3 Placement in layout v2 (client)

- **Plan, stage 11 (props):** reserve `curiositySlots` per district by anchor kind: up to 2 `wall` (a street-facing wall cell beside a pavement), 1 `bench` (an existing bench prop), `bridge` (each bridge's end), `high_ground` (the highest walkable cell in the district), `waterside`, `park`, `square`, `back_lane`, `street_end` (bollards and field gates), `memorial`, `station`, `market`. The slots are part of the plan (`TownMap.curiositySlots: { anchor, x, y, district }[]`); they never block a door or route (stage 11's rule).
- **Dress (D2):** each engine curiosity takes the free slot of its district whose rendezvous hash `h(townSeed, curiosity id, slot index)` is lowest among its preferred anchors, in the registry's order, assigned in id order. New curiosities only take free slots, so nothing already drawn moves (DEC-085). With no matching slot, a `walk` curiosity is not drawn and a `streets` one is still found by "Look around" (§6.4).
- **Hidden-place cues** (G2: alley gate, basement steps, overgrown path, lookout bench) use the same slot mechanism with their own anchors.
- **Art:** new prop art keys `prop.plaque`, `prop.mural`, `prop.poster_old`, `prop.cat`, `prop.noticeboard`, `prop.memorial_stone`, `prop.telescope`, `prop.slogan` (pixel-art P1; placeholders until then, with the fallback chain of TOWN_GENERATION §7.3).
- Plans change shape (`curiositySlots`), so the pinned v2 plan hashes are refreshed. No run stores a layout version yet, so `LAYOUT_VERSION` stays 2 (TOWN_GENERATION "In the game").

### 6.4 Discovery, parity and the engine–web split

| Step | Web (presentation and input) | Engine (authority) |
|---|---|---|
| Show | Draws a marker prop in its slot (unfound and found look the same in the street; found ones get a small tick in the map) | `TownStateView.curiosities: [{ id, kind, art, hood, found }]` |
| Walk-find | The player walks next to a marker and taps it: sends `PlayerCommand::Inspect { curiosity }` | Checks the player is on that neighbourhood's streets (`Location::Street`), that it is not found yet and its `when` holds; applies the reward once; records `Subject::Curiosity { id }` at `known` in the ledger; the outcome carries the text key and arguments |
| Look around (parity) | Places tab: on a neighbourhood's streets, "Look around" | `PlayerCommand::LookAround`: finds every `streets` curiosity present there whose `when` holds. Free, as walking is |
| Notebook | A "Town lore" section: found lore with its text, and a count of what has been found (never a total, M6 §1.8) | `NotebookView` section `lore` |

**The accessibility rule (DEC-051, DEC-097 §5).** Anything with a reward (`streets`) is found by `LookAround` from the Places tab or quick mode, under the same conditions and with the same result. Only `walk` curiosities (reward: lore text, no play effect) are walk-only. Validation enforces it: a `walk` kind may not have reward effects; a plaque that teaches a person must be `streets`.

The client never decides a reward: it only says which marker the player tapped. The engine cannot check tile adjacency (layouts live on the client, DEC-085), so it checks the neighbourhood. That is enough for a single-player game; nothing competitive rests on it.

### 6.5 Encounters

When the player arrives on a neighbourhood's streets (`MoveTo` or `Go` to `Location::Street`, from any mode), the engine may produce an encounter with a real person whose routine puts them there: a resident of that neighbourhood with something to say, a journalist off duty (their office's neighbourhood), the broker taking a walk (their home neighbourhood), the rival canvassing (in a campaign).

| Parameter (`curiosities.encounters`) | **Proposed** |
|---|---|
| `chance` per street arrival | 0.08 |
| `max_per_turn` | 1 |
| `person_cooldown_days` | 120 |
| `max_per_year` | 8 |

An encounter is a free exchange of one topic chosen by the engine (the person's best: a secret `heard` with a lead if they know one and will tell, an ask, else small talk), through the conversation engine (C1) with no slot cost and the usual repetition guard. Keyed draw `stream(seed, "encounter", date, hood)`; the draw is made on every street arrival, whatever the player knows.

### 6.6 Reward sizes and caps

- Every reward is one-off per curiosity (the ledger records it), except the vista (once per 90 days, health once a month).
- Expected play-affecting finds per town: 6–12, about the value of two canvasses in total.
- Lab: a `walker` probe (mixed plus `LookAround` on every street arrival) beats `mixed` by at most 1.0 point of mean first-election share; `explore_only` stays within `idle` + 5 points (the K4 gate); the blind-invariance test covers curiosities and encounters.

---

## 7. The engagement gate (DEC-097 §6)

### 7.1 The command

`mandate-cli engagement --seeds N --years Y [--strategies mixed,canvass,idle] [--set …]` plays careers with the bot strategies (the `bots --careers` loop) and prints one row per career and medians per strategy. It exits non-zero when a gate fails, like `bots`. It also accepts `--set` and `--grid` (DEC-078).

Metrics per career:
- decisions, distinct templates, uses of the most-used template, repeats within a chapter, distinct climax templates;
- the longest quiet stretch: the most days with no beat (a decision opened, an ask offered or resolved, a storyline stage, a press story about the player), separately for governing and campaigns;
- storyline templates started, the most active at once, scheme stages that advanced outside a campaign;
- press stories about the player, by source (work, scandal, case, statement);
- asks offered, accepted, kept, broken, declined, void; promises made, kept, broken;
- secrets seeded and learned; curiosities found (walker only);
- the approval sources split (§2.3);
- elections won and lost (5 years covers two local elections).

### 7.2 Thresholds

Proposed from the measurements in §1.1. Medians are across seeds for `mixed`; "every seed" is a floor per career. 5-year careers.

| Gate | Today (mixed, 5 y, 12 seeds) | Median | Every seed |
|---|---|---|---|
| Distinct decision templates | 11.5 (9–17) | ≥ 18 | ≥ 14 |
| Decisions | 34.5 (22–38) | 30–50 | ≥ 22 |
| Most uses of one template | 6 (5–6) | ≤ 2 | ≤ 3 |
| Repeats within a chapter | 1 (0–3) | 0 | 0 |
| Same climax template in consecutive chapters | 10 of 12 careers (4 y) | 0 | 0 |
| Longest quiet stretch, governing (days) | 168 (154–210), decisions only | ≤ 60 | ≤ 90 |
| Longest quiet stretch, campaign (days) | — | ≤ 21 | ≤ 28 |
| Storyline templates started | 2 (1–3) | ≥ 3 | ≥ 2 |
| A scheme stage outside a campaign | 0 | ≥ 75% of careers | — |
| Press stories about the player | 0 | ≥ 6, half on work | ≥ 2 |
| Asks offered | — | ≥ 15 | ≥ 8 |
| Asks kept, of those accepted (mixed) | — | ≥ 60% | — |
| Asks broken | — | ≥ 1 in 25% of careers; ≤ 30% of accepted | — |
| Canvass share of approval gained between elections | not measured | ≤ 40% | — |
| `idle`: press stories about the player | 0 | ≥ 1 (the thin record is reported) | — |

From the bot tournament (first election, `bots`), replacing today's informal canvass check:

| Gate | Rule |
|---|---|
| `single_verb_not_best` | Single-verb strategies are `canvass`, `media`, `asks_only`. Each one's win rate ≤ the best multi-verb strategy's − 5 points **and** its mean share ≤ the best multi-verb mean share − 1 point; together they are the sole best in ≤ 30% of worlds. Today canvass ties explorer at 87.5% and 0.630: it fails |
| `walking_is_a_bonus` | `walker` mean share ≤ `mixed` + 1.0 point |
| `secrets_not_decisive` | `secret_hunter` mean share ≤ `mixed` + 1.0 point |
| `skill_gap` | The best informed style beats the best rote routine (single-verb strategies, blind `mixed`, `legislator`, `proxy`) by ≥ 3 points of mean share and ≥ 10 points of win rate. Proposed; E12 tunes it from the first runs after E1 and E8 |
| `variety` (amended, DEC-097 balance principle) | Measured among the informed styles only: `explorer` plus new informed variants, `explorer_coalition` (talk, lobby, favours, asks), `explorer_legislator` (bills, delivery) and `explorer_campaigner` (ground game, media, promises). At least 2 are the sole best somewhere, none is among the best in more than 60% of worlds, and the best style differs by situation (crisis, industry) |
| `no_dominant_strategy` (amended) | Applies among informed styles. A rote routine being far below the best is the intended result, not a failure |
| `exploring pays` (raised) | `explorer` mean share ≥ blind `mixed` + 3 points (was + 1) |
| Unchanged | Effort matters, idle ≤ 15%, corrupt not dominant, exploring alone never wins, `walking_is_a_bonus` and `secrets_not_decisive` (both concern the input mode and single shortcuts, not knowledge in general) |

**Why (owner, 2026-11-19).** Players optimise toward whatever wins, so the winning play must be the engaging one. The old variety gate failed whenever the explorer, the intended way to play, rose to the top: §1.3's grid shows exactly that. The principle in DEC-097 replaces "every routine roughly equal" with "good play clearly beats rote play, and good play has several styles that suit different towns". The old wording follows for reference:

| The existing gates (before the amendment) | No dominant strategy, effort matters, idle ≤ 15%, corrupt not dominant, variety, exploring pays, exploring alone never wins |

The re-election (the second election in a 5-year career) adds one check: `canvass` re-election rate ≤ `mixed`'s (delivery should matter most for an incumbent).

### 7.3 CI and nightly

| Run | What | Measured cost today | Budget |
|---|---|---|---|
| CI (beside `bots --seeds 16`) | `engagement --seeds 8 --years 5 --strategies mixed,canvass,idle` | ≈ 0.6 s a career: 24 careers ≈ 15 s | ≤ 60 s after E6–E11 add work |
| CI | `bots --seeds 16` with the new strategies (`asks_only`, `walker`, `secret_hunter`) | 18 s for 9 strategies today; ≈ 24 s for 12 | ≤ 45 s |
| Nightly | `engagement --seeds 32 --years 8 --strategies mixed,canvass,idle,explorer,asks_only` | ≈ 1 s a career: 160 careers ≈ 3 min | ≤ 10 min |
| Nightly | `bots --seeds 64` | ≈ 70 s | — |

**Proposed:** the engagement gates report only (nightly, no failure) until E2–E8 have merged, then block in CI (§9, question 6). BALANCE_LOG records each threshold change.

---

## 8. How this fits the M6 tasks

| M6 work | Interaction |
|---|---|
| **Talk work C2–C5** | E1–E5 run beside them (DEC-097 order). Asks can come up in conversation (a reply's substance may offer an ask; DEC-035's "promise" option type); `Need::Talk` is met by `Talk`; encounters use the C1 engine with no slot cost; secrets are taught by lines (C3 format, new subjects); C5's repetition gate and the engagement gate share the lab command style |
| **Policy families F1–F7** | Asks land before F1 (DEC-097). `Need::Introduce` and `Need::Enacted` name a template until F2b, then a family instance (the `policy::resolve` seam of F2a); a sponsor's price can be an ask ("back my next bill"); campaign promises draw on open proposals; F7's need-aware reactions feed V1's credit, which feeds law-result press stories (§4.6) |
| **V1, V2, P1, A1** | V1 is the main "delivery" source of approval (§2.3): E1 is tuned before V1 and re-checked after it. V2 adds endorsements as ask rewards. P1 drives `storyline.broker_scheme`, `ask.broker_vote`, `secret.broker_deal` and `secret.rival_backer`. A1's intrigue sets dig odds; organising could scale the canvass `awareness` (A1's own lever, not changed here) |
| **Hidden places G2** | E9–E11 run with it (DEC-097 order): hidden places are a walking-reward category; their cues use the curiosity slots; the union archive is a dig site; `LookAround` and G2's walking discovery share the street-arrival hook |
| **Odds calibration L1** | E12 runs beside it. New shown odds go into L1's recording list: casework success, dig odds, and a press tip's chance of running |
| **K6** | Its validation rule (no `knows` where a condition decides whether something happens) extends to ask triggers, secret seeds and curiosity eligibility |

---

## 9. Questions for the owner (answered 2026-11-19: all recommendations accepted)

| # | Question | Recommendation |
|---|---|---|
| 1 | Should canvassing between elections still raise approval at all? | **Yes, a little** (0.6× today on a first visit, falling fast on repeats). Awareness is unchanged, so canvassing stays the way to be known; approval between elections comes from delivery |
| 2 | Casework: a free-standing verb, or only through asks? | **Only through asks.** A free-standing "help someone" action would become the new grind; tying it to asks keeps it bounded and grounded |
| 3 | How many campaign promises, and should broken ones make the press? | **Three per campaign; broken public promises make the press and give the rival material.** It makes the campaign and the term one story |
| 4 | Can a known secret be used to pressure someone? | **Not in M6.** Tipping the press and framing are enough. Pressure (blackmail) later as a shady act with its own hidden fact (DEC-034), if wanted |
| 5 | Easter eggs: in-world only, or the odd fourth-wall joke? | **In-world only** (heightened realism, DEC-061): odd shop signs, an old band poster, the cat on the wall. No completion reward for lore, so quick-mode players lose nothing |
| 6 | Should the engagement gate block CI from the start? | **Report-only (nightly) until E2–E8 merge, then block in CI.** The thresholds need a few runs on the new content first |

---

## 10. Risks

- **Balance churn.** §1.3 shows a flat canvass cut fails the variety gate and moves the dominance to the explorer. The explorer's lead over mixed is already thin (1.2 points against a 1-point gate, BALANCE_LOG K4). Mitigation: E1 tunes on a grid with the season-aware curve, re-checks every gate at 16 and 32 seeds, and is re-checked after E8 and V1.
- **Asks as busywork.** A to-do list is not a story. Caps (4 open, 1.5 a month), a live reason for every ask, no fetch quests, and declining costs little.
- **Notification overload** on the Today tab with decisions, asks and promises together. One sorted list by deadline, a badge count, and a collapsed "recent" block.
- **The client–engine split for curiosities.** The engine places curiosities by neighbourhood; the client finds slots. A town with too few slots draws fewer markers; `streets` curiosities stay reachable by Look around, so play is unaffected.
- **Secrets tilting play.** Capped uses, no leverage in M6, and two gates (`secrets_not_decisive`, the omniscient check).
- **Press volume.** Work stories could crowd out scandals and town news; outlet capacity is unchanged and work stories score below a breaking scandal.
- **Content volume.** About 22 dilemmas, 6 storylines, 16 asks, 6 promises, 8 secret kinds and 14 curiosity kinds, each with Fluent text: content tasks are split so they can run in parallel with the engine work.
- **CI time.** About 15 s more now; the budget is 60 s for the engagement run.
- **Save churn.** Six bumps (E1, E2, E4, E6, E9, E10), each additive with serde defaults; the lead numbers them at merge and may fold neighbours together (M6 §4 rules).

---

## 11. Task breakdown

**Order (DEC-097):** E1–E5 with the talk work (C2–C5); E6–E8 before the policy families (F1); E9–E11 with hidden places (G2); E12 beside L1. **Rules:** as M6 §4 (the lead numbers save bumps at merge; append-only edits to shared enums `ContentFile`, `ContentBundle`, `QueryDto`, `ResponseBody`, `Effect`, `EventKind`, `PlayerCommand`, `Action`; `CARGO_BUILD_JOBS` set per brief).

| # | Task | After | Runs beside | Save bump |
|---|---|---|---|---|
| **E1** | Ground-game curve and record | K4 | C2–C5, E2 | yes |
| **E2** | Director variety rules, `uses`, `answers`, `answered` condition, story metrics | K6 | C2–C5, E1 | yes |
| **E3** | Dilemma pool: 22 templates | E2 | C3–C5 | no |
| **E4** | Storylines and NPC schemes (`npc_did`, 6 storylines; broker scheme after P1) | E2 | E3, E5 | yes (`npc_deeds`) |
| **E5** | The press covers work (`StorySource::Work`) | E2 | E3, E4 | no |
| **E6** | Asks engine (domain, commands, casework, deeds, effects) | E1, C1 | E7 drafting | yes |
| **E7** | Asks protocol and Today tab | E6, C2 | E8 | no |
| **E8** | Asks content, promises, request conversions, bots | E6 | E7 | no |
| **E9** | Secrets: registry, seeding, dig, tip, talk hooks | E6, K3, C1 | G2, E10 | yes |
| **E10** | Walking rewards engine: curiosities, Inspect, LookAround, encounters, lore notebook | G2 | E9, E11 | yes |
| **E11** | Walking rewards web: slots, dress, props, inspect, Look around, lore page | E10, B3 | E9 | no |
| **E12** | Engagement lab command and gates in CI and nightly | E2, E8 | L1 | no |

### E1 — Ground-game curve and record

- **Goal:** canvassing builds awareness first, with fast per-neighbourhood diminishing and a campaign-season boost (§2.1); record from delivery (§2.2); no single-verb strategy best at the end of E8 (re-checked there).
- **OWNS:** `crates/mandate-sim/src/actions.rs` (the `Canvass` arm and its outcome args only); `crates/mandate-sim/src/player/{mod.rs,ground.rs (new)}` (`PlayerState.ground`: per-neighbourhood visits and dates); `crates/mandate-content/src/types.rs` (canvass action params and `record_gain` fields); `content/base/{actions,career}.jsonc`; `content/schemas/**`; `crates/mandate-save/**` (migration); `crates/mandate-cli/src/bots.rs` (`asks_only` stub, the `single_verb_not_best` gate reported, not enforced until E8); `content/base/locale/en/base.ftl` (canvass freshness lines); `docs/development/BALANCE_LOG.md`.
- **Depends on:** K4 (done).
- **Acceptance:** unit tests of the curve (same neighbourhood every turn → fresh < 0.1; rotation of three → 0.4–0.6; campaign multiplier applies only while a candidate); `bots --seeds 16` and `--seeds 32` pass every existing gate; canvass win rate and mean share no longer the highest (reported); grid evidence in BALANCE_LOG; determinism (2026 × 80) twice and through save/load; native = WASM (777 × 60).
- **Save:** bump (`PlayerState.ground`, default empty).

### E2 — Director variety rules

- **Goal:** cooldown floor and growth, no repeat in a chapter (climax included), climax memory, `pick` by relevance and novelty, post-climax and quiet-days rules (§4.1–4.3); the `answered` and `sponsored_law` conditions (§4.4); `story --metrics` prints the §7.1 decision metrics.
- **OWNS:** `crates/mandate-sim/src/narrative/{director.rs,mod.rs,cast.rs}`; `crates/mandate-content/src/narrative.rs` (`DirectorParams`, `Condition::Answered`, `Condition::SponsoredLaw`, validation); `content/base/director.jsonc`; `content/schemas/**`; `crates/mandate-save/**`; `crates/mandate-cli/src/main.rs` (the `Story` arm only).
- **Depends on:** K6 (done).
- **Acceptance:** 12 seeds × 5 years mixed: most uses of a template ≤ 3, no repeat within a chapter, no climax template in consecutive chapters, longest governing gap ≤ 120 days (≤ 60 needs E6 and E5); the director on/off test (endogenous outcomes identical) still passes; the draw count is identical with `--omniscient`; CI sweep decisions per chapter stay within budgets (0 over).
- **Save:** bump (`NarrativeState.uses`, `climaxes`, `answers`).

### E3 — Dilemma pool

- **Goal:** the 22 dilemmas of §4.4 with scenes and Fluent text (the four that need V2, P1, G3 or E9 ship with those tasks, written now and enabled by their triggers).
- **OWNS:** `content/base/{dilemmas,scenes}.jsonc`; `content/base/locale/en/narrative.ftl`.
- **Depends on:** E2.
- **Acceptance:** `content validate` (every option has a cost and a benefit; Fluent arguments); 12 seeds × 5 years: distinct templates median ≥ 18, every seed ≥ 14; each new template opens in at least one of 32 seeds or is listed as gated on a later task.
- **Save:** none.

### E4 — Storylines and NPC schemes

- **Goal:** `npc_did` condition and the NPC deeds log; `storylines_active` 3, spacing, minimum; the six storylines of §4.5 (`broker_scheme` content lands with P1).
- **OWNS:** `crates/mandate-sim/src/npc.rs` (appending deeds only); `crates/mandate-sim/src/politics/mod.rs` (`npc_deeds` field); `crates/mandate-sim/src/narrative/director.rs` (`start_runs` only, after E2 merges); `crates/mandate-content/src/narrative.rs` (`Condition::NpcDid`); `content/base/{storylines,director}.jsonc`; `content/base/locale/en/narrative.ftl` (storyline keys); `crates/mandate-save/**`.
- **Depends on:** E2.
- **Acceptance:** 12 seeds × 5 years: storyline templates median ≥ 3, every seed ≥ 2; a scheme stage outside a campaign in ≥ 75% of careers; schemes advance identically with and without the player knowing (K6-style test); storyline views gated by the ledger (K2 fallbacks).
- **Save:** bump (`npc_deeds`, default empty).

### E5 — The press covers the player's work

- **Goal:** `StorySource::Work` (§4.6) for every politician; bill passed or failed replaces subject-less town news for bills; appearances; promise stories hook for E6.
- **OWNS:** `crates/mandate-sim/src/media.rs`; `crates/mandate-content/src/types.rs` (media section); `content/base/media.jsonc`; `content/schemas/**`; `crates/mandate-protocol/src/media.rs` (article view for the new source); `content/base/locale/en/media.ftl`.
- **Depends on:** E2 (metrics).
- **Acceptance:** 12 seeds × 5 years mixed: press about the player median ≥ 6, every seed ≥ 2, at least half on work; idle ≥ 1; the perception twin test (DEC-034) passes for work stories; bot gates pass.
- **Save:** none (`StorySource` gains a variant; the stories list is already in saves and old saves have none of it).

### E6 — Asks engine

- **Goal:** the `asks` domain (§3.6): state, weekly system, `PlayerDeed` events, `RespondAsk`, `Promise`, `Action::Casework`, effects, validation, `Stream::Asks`; one sample ask and one promise.
- **OWNS:** `crates/mandate-sim/src/asks/**` (new); `crates/mandate-sim/src/events.rs` (`PlayerDeed`); `crates/mandate-sim/src/actions.rs` (deed appends, `RespondAsk`, `Promise`, `Casework` arms); `crates/mandate-sim/src/{systems.rs,scheduler.rs}` (registration; the new `Effect` variants `Opinion`, `Favour`, `Approval`, `Record`); the arms that apply them in `crates/mandate-sim/src/{characters,population,politics}/**` (effect application only); `crates/mandate-core/src/rng.rs` (`Stream::Asks`); `crates/mandate-content/src/asks.rs` (new) and its `ContentFile`/bundle lines; `crates/mandate-content/src/validate.rs` (asks section); `content/base/asks/{params,asks,promises}.jsonc` (samples); `content/schemas/**`; `crates/mandate-save/**`.
- **Depends on:** E1, C1 (done).
- **Acceptance:** unit tests for each `Need` (kept, broken, void); property test: an ask resolves exactly once; asks never depend on `knows` (validation test); determinism, save/load/continue parity, native = WASM; the blind-invariance test still passes.
- **Save:** bump (`State.asks`).

### E7 — Asks protocol and the Today tab

- **Goal:** `AskView`, `PocketView.asks`, `asks_due_soon`, `QueryDto::Asks`, `QueryDto::PromiseMenu`, TS types; the Today tab block, the story sheet's promises, the promise picker while campaigning.
- **OWNS:** `crates/mandate-protocol/src/{asks.rs (new),places.rs (PocketView fields),lib.rs}`; `web/src/engine/protocol/**` (generated); `web/src/**` except `engine/protocol` (Today tab, promise picker, story sheet).
- **Depends on:** E6, C2.
- **Acceptance:** Rust ⇄ TS round-trip; accept, decline and keep an ask in Playwright (phone and desktop); keyboard and screen reader labels; the badge counts offers and promises due within a turn.
- **Save:** none.

### E8 — Asks content, promises, conversions and bots

- **Goal:** the first pack (§3.8) with Fluent text; `request.ally_vote`, `request.traders_meeting`, `request.back_the_workers` become asks (goals point at them); bots accept and fulfil (§3.9); `asks_only`; the `single_verb_not_best` gate enforced; final ground-game tuning with E1's grid.
- **OWNS:** `content/base/asks/**`; `content/base/{goals,dilemmas}.jsonc` (the three conversions only); `content/base/locale/en/asks.ftl` (new); `crates/mandate-cli/src/bots.rs`; `content/base/{actions,career}.jsonc` (tuning values only); `docs/development/BALANCE_LOG.md`.
- **Depends on:** E6 (drafting can start after E6's formats).
- **Acceptance:** 12 seeds × 5 years: asks offered median ≥ 15, every seed ≥ 8; mixed keeps ≥ 60% of accepted; broken in ≥ 25% of careers; canvass share of approval ≤ 40%; `bots --seeds 16` and `32`: every gate including `single_verb_not_best` passes.
- **Save:** none.

### E9 — Secrets

- **Goal:** the secret kinds registry and seeding (§5.2), `HiddenFact.secret` and `cast`, `Effect::Hide`, `speaker_knows` rules, `Investigate` and `TipPress` actions, notebook entries and leads, `secret_hunter` probe and its gate.
- **OWNS:** `crates/mandate-sim/src/narrative/{mod.rs (HiddenFact fields),secrets.rs (new)}`; `crates/mandate-sim/src/actions.rs` (`Investigate`, `TipPress` arms); `crates/mandate-sim/src/characters/knows.rs` (secret rules); `crates/mandate-content/src/{secrets.rs (new),conversation.rs (KnownSubject::Secret)}`; `content/base/secrets.jsonc` (new); `content/base/conversations/lines.jsonc` (secret substance lines); `content/base/locale/en/{secrets.ftl (new),talk.ftl}`; `content/schemas/**`; `crates/mandate-save/**`; `crates/mandate-cli/src/bots.rs` (`secret_hunter`).
- **Depends on:** E6 (`ask.keep_it_quiet`), K3, C1 (done); P1 for the broker kinds (they seed nothing until P1).
- **Acceptance:** secrets seed with and without the player knowing (identical worlds except the ledger, K6 test); journalists and the rival discover them by the existing rules; dig and tip odds shown = rolled (L1 hook); `secrets_not_decisive` passes; validation: no `knows` in seeds.
- **Save:** bump (`HiddenFact` fields; typed `HiddenId` recommended).

### E10 — Walking rewards, engine

- **Goal:** the curiosity registry and validation (§6.1, the accessibility rule), seeding at play start (§6.2), `Inspect`, `LookAround`, encounters (§6.5), `Subject::Curiosity`, the `lore` notebook section, `TownStateView.curiosities`, the `walker` probe and gate.
- **OWNS:** `crates/mandate-sim/src/{explore.rs (new),places.rs (street-arrival hook)}`; `crates/mandate-sim/src/world.rs` (`curiosities` field); `crates/mandate-sim/src/player/knowledge.rs` (`Subject::Curiosity`); `crates/mandate-sim/src/actions.rs` (`Inspect`, `LookAround` arms); `crates/mandate-content/src/curiosity.rs` (new); `content/base/curiosities.jsonc` (new, 14 kinds); `content/base/locale/en/curiosities.ftl` (new); `crates/mandate-protocol/src/{town.rs (TownStateView field),knowledge.rs (lore section)}`; `web/src/engine/protocol/**` (generated); `content/schemas/**`; `crates/mandate-save/**`; `crates/mandate-cli/src/bots.rs` (`walker`).
- **Depends on:** G2 (shares the street-arrival discovery hook and `Subject::Place`).
- **Acceptance:** every town of 16 seeds has 12–30 curiosities, ≥ 4 kinds, none of them `streets` with no reward or `walk` with one (validation); `Inspect` and `LookAround` give the same rewards; encounters fire on arrivals from any mode; `walking_is_a_bonus` and the K4 gates pass; determinism and native = WASM.
- **Save:** bump (`world.curiosities`).

### E11 — Walking rewards, web

- **Goal:** curiosity slots in plan stage 11, dress assignment by rendezvous hash, placeholder props and art keys, tap to inspect, the encounter card, "Look around" in the Places tab, the notebook's lore page.
- **OWNS:** `web/src/hub/layoutV2/{plan.ts,dress.ts,check.ts}` and their tests and snapshots; `web/src/hub/townLayout.ts` (`PropKind`, `TownMap.curiositySlots`); `web/src/hub/render/**` (placeholder props); `web/src/**` pocket and notebook components; `tools/pixelkit` art keys list only (P1 draws the art).
- **Depends on:** E10, B3 (done).
- **Acceptance:** slots never block a door or route (§13 checks extended); the dress never moves a placed marker as curiosities are found (test); pinned v2 hashes refreshed; Playwright: walk to a marker and inspect it; Look around from the Places tab finds the same `streets` curiosity; screen reader labels; plan timings within 5% of before.
- **Save:** none.

### E12 — The engagement gate

- **Goal:** `mandate-cli engagement` (§7.1), the gates of §7.2, CI and nightly wiring (§7.3), the approval-source split, TEST_STRATEGY and BALANCE_LOG entries.
- **OWNS:** `crates/mandate-cli/src/{engagement.rs (new),main.rs}`; `crates/mandate-cli/src/bots.rs` (shared career loop only); `.github/workflows/{ci,nightly}.yml`; `docs/development/{TEST_STRATEGY.md,BALANCE_LOG.md}`.
- **Depends on:** E2 and E8 (metrics exist); runs beside L1 (both add CI steps; the lead merges the workflow edits).
- **Acceptance:** CI run ≤ 60 s; gates report-only until the owner's go (§9, question 6), then enforced; a seeded failure (a content override that disables cooldowns) fails the gate; output is stable across runs (deterministic rows).
- **Save:** none.
