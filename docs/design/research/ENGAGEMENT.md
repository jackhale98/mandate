# Engagement and variety (DEC-097): design pass

**Status:** Design brief for DEC-097 (2026-11-19). It turns the six parts of DEC-097 into contracts, numbers and tasks. The owner accepted every recommendation (§9) and, the same day, the proposals of `ENGAGEMENT_REFERENCES.md` §3 ("Yes do it"). Both are folded in here, so this is the one document to implement from. Numbers in "Start" columns are starting values: E1, E8 and E12 tune them on the lab grid and record each change in BALANCE_LOG. Where the research changed a number, a note says so, for example "(research §3.1, accepted 2026-11-19)".
**Built on:** `claude/bold-brown-shyeug` at save format v17 (K1–K4, K6, C1, C3, B1–B3, G1 merged).
**Read with:** DEC-030–DEC-035, DEC-041, DEC-051, DEC-054, DEC-072, DEC-076, DEC-077 (and its DEC-097 amendment), DEC-078, DEC-079, DEC-088, DEC-090, DEC-091, DEC-093, DEC-094, DEC-095, DEC-096, DEC-097, DEC-098; `EXTENSIBILITY_AUDIT.md` §3; `M6_ENGINE_CONTRACTS.md` §1 (ledger), §3 (conversations), §4 (tasks); `TOWN_GENERATION.md` §5, §10, §11; `ENGAGEMENT_REFERENCES.md` §2 (sources) for the reasoning behind §4.2, §4.3, §4.7, §4.8, §7.2 and §7.4.

**Tier-generic (DEC-098, accepted 2026-11-19).** The contracts here follow the audit `EXTENSIBILITY_AUDIT.md` §3.2, §3.3, §3.6 and §3.7, so the new state works for the regional and national tiers without re-keying. They use R1's keys and accessors (`M6_ENGINE_CONTRACTS.md` §4): `Jurisdiction` and `Area` for state that belongs to a government's territory (`MunicipalityId` stays for physical things in a town, such as curiosities); `player::home(state)`, `player::posts(state)` and `state.focus()` in place of reads of `player.municipality`; offices and bodies by constitution id (a colleague is a holder of a post in the same body; the executive is `executive_of(at)`). Conditions use U1's one condition type (DEC-098 §5).

**Task ids.** Tasks here are `E1`–`E13` (engagement). They are not the town v2 "phase E" steps of `TOWN_GENERATION.md` §15 (done, as B1 and B2).

---

## 0. In short

- **The ground game.** A canvass mostly builds name recognition. Its approval gain falls fast when the same neighbourhood is canvassed again within weeks, and is doubled in campaign season. Approval between elections comes mainly from delivery: kept asks and promises, help to individuals, the press covering the player's work, and (with V1) credit for laws.
- **Asks and promises.** People, the party, the broker, organisations and the press ask the player for concrete things with deadlines. The player accepts or declines; an accepted ask is a promise, kept or broken by what the player actually does (a vote, a statement, a visit, a town hall, a draft, a piece of casework). Campaign promises are public asks from the ward. Offers come at a sliding rate: sooner when the player has nothing open, rarely when they are busy. A new `asks` domain owns them; the Today tab lists them.
- **Storyteller variety.** The director paces each chapter in cycles: a build-up of 35–60 days, a peak with one significant decision, then a relax of 21–35 days (30–45 after the climax) with only minor beats. It picks by tier: storyline stages and closing consequences first, then the chapter's theme, then the rest. Each session chapter has a theme (crisis, budget, scheme or local life; campaigns have their own), never the same as the last. Every template has a cooldown of at least a year that doubles with each use and never repeats within a chapter. About 26 new dilemmas (10 of them consequences of the player's own answers, 4 campaign-only), 6 new storylines including NPC schemes whose next move is shown once the player knows the schemer's plans, press stories about the player's work, and new pressures introduced year by year.
- **Conversations that remember (E13).** Lines have a priority, can be said only once, and can follow earlier lines. A line that reacts to what the player did ("Still waiting on that drain") comes before small talk.
- **Secrets.** Town secrets are hidden facts (DEC-034) seeded from live state by a content registry: the broker's deal, a buried report, who funds the rival, a councillor's debts, a planned closure. People who know them tell at their discretion (DEC-093); the player can investigate or tip off the press. They exist and can surface whether or not the player finds them. Every find points to another.
- **Walking rewards.** A registry of curiosities (plaques, murals, old posters, a lost cat, a bench with a story, a view), placed by the engine per neighbourhood and drawn by the client in slots the layout reserves. Rare street encounters with real people. Lore is collected in the notebook and ties to people and places the player already knows and to the town's live trouble. Anything that affects play is also found by "Look around" from the Places tab; pure flavour may be walk-only.
- **The engagement gate.** `mandate-cli engagement` plays 5-year careers and checks template variety, repeats, the pacing cycle, quiet stretches, storylines, press about the player, asks kept and broken, where approval comes from, and conversation callbacks. `engagement --tables` lists templates never opened or always opened, and options nearly always picked. In `bots`, one **gate ladder** encodes the owner's balance principle: single-verb routines < a blind mix < informed play, knowing the town is worth a clear margin, and several informed styles each win somewhere. It runs in CI beside the bot gates.

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

| Canvass approval | Record gain | canvass | explorer | mixed | legislator | Gates failing (old set) |
|---|---|---|---|---|---|---|
| 0.02 (today) | 0.06 | 87.5% | 87.5% | 81.2% | 81.2% | none |
| 0.02 | 0.03 | 87.5% | 87.5% | 81.2% | 81.2% | none |
| 0.008 | 0.06 | 75.0% | 81.2% | 81.2% | 81.2% | variety |
| 0.008 | 0.03 | 68.8% | 81.2% | 81.2% | 81.2% | variety |
| 0 | 0.06 or 0.03 | 68.8% | 75.0% | 75.0% | 81.2% | variety |

At 0.008 / 0.03 the canvass bot's mean share falls 4.7 points (0.630 → 0.583), mixed 2.0 (0.618 → 0.598), explorer 2.0 (0.630 → 0.610), and the explorer becomes the best in 13 of 16 worlds (the old variety gate fails: 81% > 60%).

A second grid, `--grid politics.campaign.contact_memory=0.8,0.9 --grid actions.canvass.approval=0.02,0.012`:

| Canvass approval | Contact memory | canvass | explorer | mixed | legislator | Top among-best share | Gates failing (old set) |
|---|---|---|---|---|---|---|---|
| 0.02 | 0.8 (today) | 87.5% | 87.5% | 81.2% | 81.2% | 0.50 | none |
| 0.012 | 0.8 | 81.2% | 81.2% | 81.2% | 81.2% | 0.69 | variety |
| 0.02 | 0.9 | 81.2% | 75.0% | 75.0% | 68.8% | 0.62 | variety |
| 0.012 | 0.9 | 81.2% | 75.0% | 75.0% | 68.8% | 0.62 | variety |

A longer race-contact memory hurts the strategies that canvass hard in the campaign (mixed, explorer) more than canvass-only, which built its awareness all year: the wrong direction. It stays at 0.8.

**Lesson, read against the gate ladder (§7.2).** The "variety" failures above are the old gate counting the intended way of playing (the explorer rising) as dominance; under the DEC-077 amendment variety is measured among informed styles only, so they are not failures. What the grids do show is that a flat cut only clears the bottom rung. At 0.008 / 0.03, `mixed` leads `canvass` by 1.5 points of share and 12 points of win rate (`ladder_rote` passes), but `explorer` leads `mixed` by only 1.2 points, well short of `ladder_learned`'s 3. So the rebalance must do two things. It must cut *repeated, out-of-season* canvassing (what canvass-only does and mixed does not). And it must land with new sources of approval whose value depends on knowing the town: asks and promises aimed at known concerns, delivery, and the canvass `fit`. Without them the upper rungs fail. §2 is designed that way; E1 is tuned with E6–E8 in view.

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

| Constant (`content/base/actions.jsonc` → `canvass`) | Today | Start | Grid for tuning |
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

The canvass `fit` reads the worry the player *knows* (K4), so a canvass aimed at a known concern pays more. That is the situational value the gate ladder relies on (§7.2): the verb is worth more to a player who has read the town.

The canvass outcome line says how fresh the doorsteps were ("people here saw you last month"), so the curve is visible (DEC-051 "Why?"). The pocket's canvass route shows the same band before acting.

### 2.2 Record

| Constant (`content/base/career.jsonc` → `record_gain`) | Today | Start |
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
| An ask kept | 0.015 (a person's ask), 0.006 (an organisation's, across the constituency) | The asker's neighbourhood, or the player's constituency (`Area`) | asks (E6) |
| Casework kept (help to an individual) | 0.012, plus the person +10 and `helped_me` | The person's neighbourhood | asks (E6) |
| A campaign promise kept | 0.02; broken −0.03 and a press story | The constituency of the race (`Area::Constituency`) | asks (E6), media (E5) |
| Press about the player's work | `perception.approval` 0.03 × `work` weight 0.3 × heat × exposure × credibility × framing (≈ 0.003–0.006 a story) | The outlet's audience | media (E5) |
| A law's credit or blame (V1) | Set in V1; target 0.03–0.06 a year in the neighbourhoods a well-received law served | Per neighbourhood | politics (V1) |
| Reactions to the player's bills | Unchanged (`opinion.player_approval_rate`) | Cohorts aware of the player | population |

**Target composition** (lab, mixed bot, positive approval gained between elections): canvassing ≤ 40%; asks, casework and promises ≥ 30%; bills, laws and press ≥ 20%. Measured by the engagement command (§7), which attributes each command's change in weighted approval to that command and the monthly update to "laws and drift". It is a lab-side measurement: nothing new in state.

### 2.4 Bots

The bots fall into the classes the gate ladder compares (§7.2):

| Class | Strategies | Notes |
|---|---|---|
| Single-verb routines | `canvass`, `media`, `asks_only` (new) | `canvass` stays as it is: the probe of the single-verb routine. `asks_only` accepts and works every ask and does nothing else, so the gate also stops asks becoming the new dominant verb |
| Blind mixes | `mixed`, `legislator`, `proxy` | `mixed` accepts the asks it can meet and fulfils them (§3.9); in campaign season it canvasses as today |
| Informed styles (new set) | `explorer` (learn first, then the mix), `explorer_coalition` (talk, lobbying, favours, asks), `explorer_legislator` (bills, delivery, asks that need votes or drafts), `explorer_campaigner` (the ground game where `fit` is high, media, promises) | All learn the town first (`EXPLORE_TURNS`), read it through `Knowing` (K4), then lean on one way of playing. E1 builds the three variants; E8 adds their asks |
| Probes (own gates only) | `idle`, `explore_only`, `walker` (E10), `secret_hunter` (E9), `corrupt` | Outside the class comparisons |

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
| A colleague (a holder of a post in the same body as the player) or the executive (`executive_of(at)`; the mayor in a town) | Vote with me; back my amendment; second my bill | Goals `seek_backing`, `champion_fix` |
| The party (chair or whip) | Vote the party line on the budget; appear at a colleague's event; fundraise | Party patience, the budget, a campaign |
| The broker (DEC-096) | Vote for or against a bill touching their stake; meet their people | P1 broker alignment |
| Organisations (union, business association, clergy, organisers) | Join the picket; come to the meeting; hold a town hall here; draft an answer to the crisis | Goals `organise_workers`, `traders_help`; the live crisis |
| The press (a journalist) | Say something about the closure (a statement within 7–14 days); an interview | A crisis, a story about the town |
| The voters of a constituency (the ward, in a town) | Campaign promises (§3.5) | The player, while campaigning |

**Generation** (weekly, the asks system, for `state.focus()`: in M6 the home town). The asker must live in, or hold a post in, a focus jurisdiction. First a keyed draw decides whether an offer comes this week, at the sliding rate below. Then each template whose triggers hold, whose `min_career_days` has passed (§4.7) and whose asker can be cast is a candidate. Candidates are weighted by `weight` × the asker's relationship and goal × the concern's salience × the asker's history (below), and drawn with a keyed draw (`Stream::Asks`, keyed by week and template).

**A sliding rate, not only a cap** (after RimWorld's population intent; research §3.3, accepted 2026-11-19). The week's chance of an offer is

```text
chance = min(1, new_per_month × 12 / 52 × intent_by_open(open asks) × intent_by_quiet(days since the last offer))
```

with both curves interpolated linearly between their points. A player with nothing open is asked sooner, so asks fill the director's relax phases (§4.3). A busy player is left alone (the Tropico treadmill, research §2.2).

| Parameter (`content/base/asks/params.jsonc`) | Start |
|---|---|
| `max_open` (offered + accepted) | 4 (a hard cap) |
| `new_per_month` | 1.5 governing, 2.5 in a campaign: the base rate the two curves multiply. E8 tunes it so the median stays at ≥ 15 offers in 5 years |
| `intent_by_open` (new) | open asks → factor: 0 → 2.0, 1 → 1.3, 2 → 1.0, 3 → 0.5, 4 → 0 |
| `intent_by_quiet` (new) | days since the last offer → factor: 0 → 0, 7 → 0.5, 21 → 1.0, 42 → 1.5 |
| `respond_days` (an offer lapses unanswered) | 7 |
| `per_asker_cooldown_days` | 60 |
| `asker_kept_mult` / `asker_broken_mult` (new) | 1.5 / 0.5 |
| `doorstep_chance` (a canvass brings a casework ask) | 0.35 |
| `history` (resolved asks kept for views and the lab) | 60 |
| `deeds_kept` (new: the player's deed log, §3.6) | 120 |

**Callbacks** (a cheap Hades-style tally, research §3.3). A person whose earlier ask the player kept is ×`asker_kept_mult` as likely to be the next asker of that kind; one whose ask was broken is ×`asker_broken_mult`. The ask's text names the history ("You sorted the drain last spring…"), as a Fluent variant chosen by an `AskOutcome` condition (§4.8).

**The world does not wait (DEC-091).** Asks are aimed at the player, so they are never gated by knowledge. Their triggers may not use `knows` (K6's validation rule extends to ask triggers). Whether an ask exists never depends on what the player knows; an ask can teach the player about its asker (`person known`).

### 3.3 Rewards and costs

| Outcome | Asker | Player | Others |
|---|---|---|---|
| Accepted | memory `promised_favour` | — | — |
| Kept | opinion +6 to +12, memory `kept_promise`; a big ask (`big: true` in its template): a favour owed to the player | record +0.05; approval (§2.3); sometimes knowledge (the asker tells what they know: a secret `heard`, §5) or an endorsement (V2) | a press story if public (§4.6) |
| Broken | opinion −10 to −15, memory `broke_promise` (feeds `goal.oppose_player`) | record −0.04; if public: approval −0.02 to −0.03 in scope, Standing −1, a press story, rival material | — |
| Declined | opinion −3 (no memory) | — | — |
| Void | — | — | — |

Sizes are per template in content (effects reuse the narrative `EffectDef` list, so validation already checks them). Every template must have a cost on breaking and a gain on keeping (DEC-073 validation, as for dilemmas). Accepting a `big` ask also ends the director's relax phase (§4.3).

### 3.4 Casework: help to individuals

Casework is the "fix the drain" ask. It is met by one new located action, **Sort it out** (`Action::Casework { ask }`), done at the town hall (the council's services desk) for 1 slot.

- In office it succeeds with chance `casework.success_in_office` 0.85; out of office (you can only chase it up) 0.5. The odds are shown before acting (L1 records them).
- A failure leaves the ask open; a second try costs another slot.
- There is no casework without an ask, so it is not a new grind verb: the number of cases is bounded by the asks system (owner's answer, §9).

### 3.5 Campaign promises

While campaigning (from filing to election day), the player may make up to `promises.max_per_campaign` 3 public promises, chosen from a menu built from the live state of the constituency they are running in: its top concerns, the jurisdiction's crisis, open proposals (DEC-092 families, once F2b lands), and the player's own bills. Each promise is an ask whose asker is that constituency's voters (`Asker::Constituency`), with `public: true` and a deadline at the end of the budget year or the term **of the post the player is running for** (read from the post's jurisdiction and its own term, R1's `calendar_for`). A promise belongs to one race: `AskState` counts promises against the race's `Post` (§3.6).

| Promise template (examples) | Kept by |
|---|---|
| "I'll get the flood wall for {hood} on the books" | The player drafts and introduces the family (or template) before the deadline; kept when it is enacted, half-kept (no cost, no reward) if it failed a vote the player sponsored |
| "No rise in the town's taxes" | The player votes against every tax rise until the deadline (broken by a yes vote or sponsoring one) |
| "A town hall in every neighbourhood of the ward" | Town halls in each neighbourhood of the constituency within 180 days |
| "Jobs first" | The player sponsors a bill answering `concern.jobs` that is enacted |
| "Clean politics" | No hidden fact of the player's own surfaces before the deadline (an honest promise that a shady player can break by being found out) |

During the campaign a promise adds persuasion in the neighbourhoods where its concern is salient: `promises.persuasion` 0.04 × salience, discounted by the player's broken public promises (`× 0.5` per broken one, from the asks history). A promise the player already kept in the previous term adds no persuasion: an incumbent cannot sell last term's delivery twice (the second-term bar, §4.7). Promises are where the campaign and governing connect: what was promised in November is checked by the next budget.

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
    /// Promises made in the current campaign, and the race they belong to.
    pub promises_this_campaign: u8,
    pub promises_race: Option<Post>,            // R1's `Post`; a new race resets the count
    pub last_offer: Option<SimDate>,            // for `intent_by_quiet`
    /// The player's deed log, oldest first (bounded at `deeds_kept`). Read by
    /// conversation lines (§4.8) and `ask.keep_it_quiet`; the town-wide event
    /// log (2,000 recent events) is too short for a look-back of months.
    pub deeds: Vec<DeedRecord>,
    pub last_event: u32,                        // event-log position read up to (as media)
}

pub struct Ask {
    pub id: AskId,
    pub template: ContentKey,
    pub asker: Asker,
    pub cast: Vec<CastEntry>,                   // narrative cast entries: hood, bill, place, person
    pub need: Need,                             // resolved from the template's `needs` and the cast
    pub public: bool,
    pub big: bool,
    pub opened: SimDate,
    pub respond_by: SimDate,
    pub due: SimDate,
    pub status: AskStatus,
}

pub enum Asker {
    Person { id: CharacterId },
    Party { id: PartyId },
    Outlet { id: u32 },                         // a typed `OutletId` when media ids are typed (M6 §6 item 12)
    /// A constituency's voters: a ward, or (M7) a regional district. Tiered already.
    Constituency { id: ConstituencyId },
}

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

pub struct DeedRecord { pub on: SimDate, pub deed: Deed }

/// What the player did (`EventKind::PlayerDeed`).
pub enum Deed {
    Vote { policy: u32, vote: Vote },
    Statement { about: StatementAbout },
    TownHall { hood: NeighborhoodId },
    Canvass { hood: NeighborhoodId },
    Arrive { place: PlaceId },
    Talk { person: CharacterId, topic: ContentKey },
    Introduce { policy: u32 },
    Casework { ask: AskId },
    Investigate { hidden: u32 },                // E9 appends
    TipPress { hidden: u32 },                   // E9 appends
}
```

**Deeds.** The asks system learns what the player did from the event log, as media does: command handlers append `EventKind::PlayerDeed { deed: Deed }` (new; not shown as news) for votes, statements, town halls, canvasses, arrivals at places, talks, introductions and casework. Laws enacted and hidden facts surfacing are already events. The weekly system copies each deed into `AskState.deeds`, the deed log other domains read. No command handler writes the asks domain.

**Commands** (`PlayerCommand`, appended):

```rust
RespondAsk { ask: u32, accept: bool },           // free
Promise { template: String, args: Vec<(String, String)> },   // campaigning only; free; at most 3
// Action (located, at the town hall): Casework { ask: u32 }   — 1 slot
```

**Systems.** `asks::weekly` (Narrative phase, for `state.focus()`): read new deeds and events into the deed log, resolve kept, broken and void asks, generate offers at the sliding rate (§3.2). In a term after a re-election, offers draw `deadline_days` from the lower half of their range (the second-term bar, §4.7). Unlike a dilemma answer (applied inside the `Answer` command by `narrative::resolve`), asks resolve in a system, so their `EffectDef` lists go out as effect requests: the existing `Remember` and `Memory` (memories), `PlayerTrack` (Standing), `Learn` and `Lead` (the ledger); new `Opinion { holder, toward, delta }` and `Favour { holder, owed_to_player }` (characters), `Approval { scope: Area, delta }` (population: `Area::Hood`, `Area::Constituency` or `Area::In { at }`, × room as today; R1's `Area`), `Record { politician, delta }` (politics); `Endorse` arrives with V2. The director reads open asks (they count as minor beats and as pending load in tension) but never writes them; its quiet rescue (§4.3) can force the next week's offer draw.

**Validation** (`mandate-cli content validate`): every template has a keep reward and a break cost; `needs` refer to cast roles of the right type; deadlines are within [7, 900] days; no `knows` in triggers; Fluent keys exist with the arguments the cast provides; every promise template names the concern or crisis it answers; the two intent curves have increasing `x` and factors ≥ 0, and `intent_by_open` is 0 at `max_open`.

### 3.7 Content format (one example)

Ask triggers are U1's one condition type, so they read like dilemma triggers. An asker or cast role that names an office uses the constitution's ids through R1's cast (`{ "type": "colleague" }`, `{ "type": "office", "office": "mayor" }`), never a fixed office word in code.

```jsonc
// content/base/asks/asks.jsonc
{ "id": "ask.say_something", "name_key": "ask-say-something",
  "asker": { "type": "occupation", "occupations": ["occupation.journalist"] },
  "roles": [ { "id": "outlet", "cast": { "type": "outlet_of", "role": "asker" } } ],
  "triggers": [ { "type": "live_crisis", "min_severity": 0.3 } ],
  "needs": { "type": "statement", "about": "crisis" },
  "deadline_days": [7, 14], "weight": 1.0, "cooldown_days": 120, "public": true,
  "big": false, "min_career_days": 0,
  "keep":    [ { "type": "opinion", "role": "asker", "delta": 6, "memory": "kept_promise" },
               { "type": "awareness", "delta": 0.01 } ],
  "break":   [ { "type": "opinion", "role": "asker", "delta": -10, "memory": "broke_promise" } ],
  "decline": [ { "type": "opinion", "role": "asker", "delta": -3 } ],
  "detail_key": "ask-say-something-detail", "kept_key": "ask-say-something-kept", "broken_key": "ask-say-something-broken" },
```

### 3.8 The first pack

16 asks and 6 promises. Each is grounded in a live condition. "From day" is the template's `min_career_days` (§4.7).

| Ask | Asker | Trigger | Needs | Days | From day |
|---|---|---|---|---|---|
| `ask.casework` (drain, form, objection, school place: a Fluent variant by the person's situation) | Resident | Doorstep, talk, encounter | Casework | 21–35 | 0 |
| `ask.vote_with_me` (was `request.ally_vote`) | A colleague (a holder of a post in the same body, R1's `Cast::Colleague`) with `seek_backing` | Their bill before the council | Vote yes | Until the vote | 0 |
| `ask.party_budget` | Party chair | Budget before the council | Vote yes on the budget | Until the vote | 0 |
| `ask.colleague_event` | Party chair | Campaign; a colleague in a close race | Visit that ward's streets | 14 | 0 |
| `ask.say_something` | Journalist | Live crisis ≥ 0.3 | Statement on the crisis | 7–14 | 0 |
| `ask.attend_meeting` (was `request.traders_meeting`) | Shopkeeper, business leader | `concern.jobs` top | Visit the business association or market | 21–28 | 0 |
| `ask.join_picket` (was `request.back_the_workers`) | Union steward | Plant-closure crisis | Visit the union hall, then a statement | 14–21 | 0 |
| `ask.town_hall_here` | Organiser, clergy | A neighbourhood's top worry high, no town hall there in 180 days | Town hall in that neighbourhood | 30–45 | 0 |
| `ask.draft_answer` (`big`) | Organisation leader | Live crisis; an answer on the menu (a situational template, later a family) | Introduce it | 60–90 | 0 |
| `ask.broker_vote` (`big`) | The broker | A bill touching the broker's stake | Vote as asked | Until the vote | 300 |
| `ask.visit_after_flood` | Resident of a struck neighbourhood | A disaster within 30 days (G1 exposure) | Canvass or town hall there | 14 | 0 |
| `ask.second_my_bill` | A colleague (as above) with `champion_fix` | Their crisis bill drafted | Talk to them, then vote yes | Until the vote | 0 |
| `ask.fund_the_school` | Teacher, nurse | `concern.services` top; a budget coming | Statement on services, vote for the services line | Until the budget | 0 |
| `ask.meet_the_newcomer` | Business leader | A new employer in town (sector capacity rising) | Talk to them | 30 | 0 |
| `ask.keep_it_quiet` | Someone with a secret (§5) | The player dug into it or asked about it (a deed in the deed log that the subject hears of, never the ledger) | No statement or press tip about it | 90 (kept by not acting) | 300 |
| `ask.introduce_me` | Resident with a goal | A post holder in their jurisdiction they want to meet | Talk to the post holder, then the asker | 30 | 0 |

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
    pub asker: AskerView,                 // a PersonRefView, the party, an outlet, or AskerView::Constituency { name_key, number }
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
- **Bots:** `mixed` and the informed styles (§2.4) accept an ask when they can meet it within the deadline by their own routine (votes and statements always; visits and town halls if in reach; casework when a slot is free), and decline the rest; `asks_only` accepts and works every ask; `idle` never answers (offers lapse).

### 3.10 Save

`State.asks` is new (serde default: empty). One save bump; the migration step leaves the default. `Stream::Asks` joins the named streams (OVERVIEW §3). `PlayerDeed` events are bounded with the event log; the deed log is bounded at `deeds_kept`. Size: at most 4 open plus 60 resolved asks and 120 deeds, a few kilobytes.

---

## 4. Storyteller variety (DEC-097 §3)

The director now works in four layers: cooldowns decide what may return (§4.1), tiers and the chapter theme decide what comes first (§4.2), the pacing cycle decides when (§4.3), and the career stage decides what is introduced yet (§4.7).

### 4.1 Cooldowns and no repeats

New director rules (`content/base/director.jsonc`, `DirectorParams`):

| Field | Start | Rule |
|---|---|---|
| `cooldown_floor_days` | 365 | Every dilemma's cooldown is at least this |
| `cooldown_growth` | 2.0 | The *n*-th use sets a cooldown of `max(floor, template cooldown) × growth^(n−1)` |
| `no_repeat_in_chapter` | true | A template spotlighted in this chapter is not a candidate again in it, the climax included |
| `climax_memory_chapters` | 2 | A template that was the climax in either of the last two chapters cannot be the climax |
| Climax and cooldowns | — | The climax respects cooldowns (today it skips them). It still skips the relevance floor |
| `repeat_penalty` | 0.4 (unchanged) | Now applies to people only: templates cannot repeat within a chapter at all |

`NarrativeState` gains `uses: BTreeMap<ContentKey, u16>` (times each template opened this run) and `climaxes: Vec<ContentKey>` (the last few). With these, a template can appear at most three times in five years (0, 365, 1,095 days).

### 4.2 Choosing among candidates: tiers, the chapter theme, the pick

The director builds its candidate list each week as today: cast, triggers, cooldowns, `min_relevance` 0.6 (the climax skips the floor). Then it chooses in three steps (after Reigns and Hades; research §3.2, accepted 2026-11-19).

**1. Tiers.** Candidates fall into three tiers, and the pick draws from the highest tier that has a candidate:

| Tier | Templates |
|---|---|
| A | Storyline stages that are due, and `answered` consequences (§4.4) whose window closes within `tier_a_window_days` 60 |
| B | Templates whose `themes` include the chapter's theme |
| C | The rest |

Story-carrying content can therefore no longer be starved by a high-relevance generic template, which is today's failure (five templates carried 60% of decisions, §1.1). The research also proposed a ×2 weight for themed templates. Tier B already does that job, so there is one mechanism, not two.

**2. The pick.** `best` becomes `pick`. Within the tier, the candidates are those within `pick_band` 0.8 of the tier's best relevance. They are weighted by `relevance × novelty`, where `novelty = 1 / (1 + uses)`, and drawn on the director stream with a keyed draw (date, chapter). The draw count never depends on the player's knowledge (DEC-091 §1.11). In the peak after a setback (§4.3), `opportunity` templates (the existing flag) are drawn first within the tier. The climax uses the same pick within its window.

**3. The chapter theme.** When a chapter opens, a keyed draw (director stream, chapter index) picks one theme among those the live state supports. Templates carry `themes: [...]` (E3 tags every dilemma, the existing 31 included).

| Theme | Eligible when | Typical templates |
|---|---|---|
| `crisis` | A live crisis in a focus town | Jobs plea, pollution, strike vote, disaster relief |
| `budget` | Fiscal stress: the fiscal-hole crisis, or a budget before the council with a deficit | Fiscal hole, clinic hours, bus route cut |
| `scheme` | An NPC scheme storyline is running (§4.5) | Broker ultimatum, truce tested, journalist source |
| `local_life` | Always | School closure, market permit, heritage building, building protest |
| `campaign` | Campaign chapters only, always | Attack ad, debate, gaffe, hostile doorstep |

- **No theme twice running.** A session chapter never takes the theme of the last session chapter while another theme is eligible. Campaign chapters always take `campaign` and sessions never do. So no theme appears in two consecutive chapters unless it is the only one eligible. With `climax_memory_chapters` 2, that rotation is what stops the climax returning to pollution four years running (§1.1).
- Theme weights are content (`director.themes`, all 1.0 to start).
- Eligibility reads live state, never the ledger (K6): a scheme makes the `scheme` theme eligible whether or not the player knows of it.

**Storyline starts draw, not take** (after Stellaris's precursor fix; E4). When several storylines can start, a keyed draw picks among them, weighted by relevance × novelty (`uses` counts starts). A town with a plant crisis therefore does not always get `storyline.plant_closure` first.

### 4.3 The pacing cycle

Today a session rises once to its climax and then holds everything back until January: the 140–182-day gap of §1.1. The director now paces each chapter in cycles, after Left 4 Dead's AI Director and RimWorld's Cassandra (research §2.1 and §3.1, accepted 2026-11-19). The tension curve stays the envelope: at a peak it decides whether a significant decision may open (`live < curve + slack`, `catch_up_at` as today). The cycle decides when.

| Phase | Session (days) | Campaign (days) | What the director may spotlight | Then |
|---|---|---|---|---|
| Build-up | 35–60 (keyed draw) | 7–14 | Minor beats: asks, developments, storyline stages without a decision | At its drawn length: peak |
| Peak | Until its decision is answered, at most 14 | At most 7 | One significant decision (the pick, §4.2) with its scene, and press about it. If the budget is spent or nothing qualifies, a storyline stage or an ask offer instead | Peak fade |
| Peak fade | Until no spotlighted decision is pending (at most that decision's deadline) | Same | Nothing new that is significant | Relax |
| Relax | 21–35; after the climax 30–45 | 5–10 | Minor beats only: asks, press about the player, encounters, overheard lines, storyline stages without a decision | At its drawn length, or early (below): build-up |

**Rules.**
- **The cycle runs across chapter boundaries**; a new chapter does not reset it.
- **Whose budget year (DEC-098).** Chapters stay `session` and `campaign`. A session chapter runs over the budget year of the jurisdiction of the player's main post (`player::posts(state).first()`; out of office, the home town's), not a hard-coded local year. `crisis_cap` stays per town, because crises are town state.
- **Relax ends early** when the player starts something: introduces a bill, files for office, or accepts a `big` ask (§3.3). A crisis beginning in a focus town also ends it. This is Booth's relax ending when the survivors move on.
- **The world is never hidden** (DEC-033 B). Decisions raised by the world (`raised_by`: a scandal or an investigation about the player) open in any phase. A storyline stage whose decision falls in a relax waits for the next peak, where it is tier A.
- **The climax** is the peak whose build-up ends inside `climax_window`, narrowed from [0.6, 0.92] to [0.65, 0.85] (research §3.1, accepted 2026-11-19). So about 55–130 days of the session follow it. If a session chapter is within `climax_force_days` 14 of the window's end with no climax, the current phase ends and the climax peak opens.
- **The coda.** After the climax's relax, at most `post_climax_significant` 1 decision may open in the next peak, and only a consequence (`answered`), an opportunity, or a `budget` template.
- **Budgets** stay the ceiling: significant `[2, 4]` → `[3, 5]` per chapter, plus the climax and the coda.
- **The quiet rescue.** If nothing has happened for the player for `quiet_days` 45 days (governing) or `quiet_days_campaign` 14 (campaign), the director spotlights one minor beat at once, in any phase. "Nothing" means no decision opened, no ask offered or resolved, no storyline stage, and no press story about the player. The beat is an ask offer (the asks system's next draw is forced) or a development. It never opens a significant decision during a relax. `quiet_days` was 60. It is now 45, the longest Classic relax, so a quiet stretch cannot outlast one relax plus one weekly step (research §3.1, accepted 2026-11-19).
- **Easing off after a setback** (RimWorld's adaptation). The setbacks are an election lost, Standing or viability below its fairness warning level (`fairness.standing` 30, `viability` 20), or a scandal about the player surfacing. After one, the next relax is ×`setback_relax_mult` 1.5 and the next peak draws `opportunity` templates first (§4.2). Only spotlighting changes; consequences are untouched.
- **Profiles change frequency, not amplitude** (Booth: "Amplitude (difficulty) is not changed, frequency (pacing) is"). `storytellers.jsonc` gains `cycle`, `build_up_scale` and `relax_scale`:

| Profile | `cycle` | `build_up_scale` | `relax_scale` |
|---|---|---|---|
| Classic (default) | true | 1.0 | 1.0 |
| Steady | true | 1.2 | 1.3 |
| Unfiltered | false: no phases, pure chance within budgets as today; the quiet rescue still applies | — | — |

**What the cycle guarantees** (Classic; the director steps weekly; the player answers within a governing turn of 14 days, as every bot does). These bounds set the gates of §7.2.

| Measure | Bound | Gate (§7.2) |
|---|---|---|
| Peaks per 365-day session | 3–6 (a cycle is about 60–110 days), capped by the budgets | — |
| Longest decisions-only gap, governing | Peak 14 + post-climax relax 45 + build-up 60 = 119 days; after a setback 14 + 68 + 60 = 142 | Median ≤ 120, every seed ≤ 145 |
| Longest stretch with no beat, governing | 45 + 7 = 52 days when the rescue finds a beat (after E6 an ask can almost always be cast) | Median ≤ 60, every seed ≤ 90 |
| Longest stretch with no beat, campaign | 14 + 7 = 21 days | Median ≤ 21, every seed ≤ 28 |

A player who leaves a decision pending until its deadline (35 days) lengthens the decisions-only gap by up to 21 days. That is their choice, not the director's.

**Content** (`content/base/director.jsonc`; changes from today noted):

```jsonc
"budgets": { "climax": 1, "significant": [3, 5], "minor": [5, 10] },      // significant was [2, 4]
"tension": { /* … */ "climax_window": [0.65, 0.85] },                       // was [0.6, 0.92] (research §3.1)
"cycle": {
  "session":  { "build_up": [35, 60], "peak_max": 14, "relax": [21, 35], "relax_after_climax": [30, 45] },
  "campaign": { "build_up": [7, 14],  "peak_max": 7,  "relax": [5, 10] },
  "setback_relax_mult": 1.5,
  "climax_force_days": 14,
},
"post_climax_significant": 1,
"quiet_days": 45, "quiet_days_campaign": 14,                                // quiet_days was 60
"tier_a_window_days": 60, "pick_band": 0.8,
"themes": { "crisis": 1.0, "budget": 1.0, "scheme": 1.0, "local_life": 1.0 },
"cooldown_floor_days": 365, "cooldown_growth": 2.0, "no_repeat_in_chapter": true, "climax_memory_chapters": 2,
"storylines_active": 3, "storyline_spacing_days": 90, "min_storylines": 1,  // §4.5
```

`spacing_days` 21 and `max_pending` 2 stay as floors under the cycle.

**State** (`NarrativeState`, in E2's save bump):

```rust
pub phase: Option<DirectorPhase>,   // None: the Unfiltered profile, or before play
pub setback: Option<SimDate>,       // a setback not yet eased (§4.3)

pub struct DirectorPhase { pub kind: PhaseKind, pub since: SimDate, pub until: Option<SimDate>, pub cycle: u32 }
pub enum PhaseKind { BuildUp, Peak, PeakFade, Relax }
// Chapter gains: theme: Option<ContentKey>, peaks: u8
```

Phase lengths come from keyed draws `keyed(seed, Stream::Director, [TAG_PHASE, chapter, cycle])`, so the draw count never depends on the player's knowledge.

**Metrics** for `story --metrics`: peaks per chapter, theme per chapter, the longest relax, and the share of relax phases that carried at least one minor beat (target ≥ 50%).

### 4.4 A bigger pool, with consequences

**A new condition** for dilemma and storyline triggers, `answered`, reads the player's past answers: `{ "type": "answered", "template": "dilemma.jobs_plea", "option": "wage_subsidy", "min_days": 120, "max_days": 720 }`. Its cast can carry over (`same_cast: ["voice"]`). `NarrativeState` gains `answers: Vec<AnswerRecord { template, option, date, cast }>` (bounded at 200; the bounded `decisions` history is too short for a 5-year look-back). A companion condition `sponsored_law { status, min_age_days, max_age_days, chance }` reads the player's laws. Its `chance` is a keyed draw per law and template, so the same law always gives the same answer (the second-term bar, §4.7).

Every dilemma carries `themes` (§4.2) and may carry `min_career_days` (§4.7).

**26 new dilemmas** (31 → 57 templates). Ten follow from the player's own choices:

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
| `dilemma.law_anniversary` | A law the player sponsored, 12–24 months in force (`sponsored_law`, `chance` 0.5) | Mixed results (V1's track record when built): extend, amend, or let it sunset |
| `dilemma.promise_called_out` | A broken public promise (§3.5) | The press asks: apologise, blame others, or say nothing |

Twelve more from the town's live state and the new systems:

| Template | Trigger | Notes |
|---|---|---|
| `dilemma.school_closure` | Falling rolls in a declining neighbourhood (baseline population, B1) | |
| `dilemma.clinic_hours` | Budget stress, `concern.services` | |
| `dilemma.market_permit` | The market's trade falling; a festival proposal | Noise against trade; a minor-weight option set |
| `dilemma.bus_route_cut` | Budget stress; a neighbourhood far from the centre | |
| `dilemma.heritage_building` | A housing or development bill touching a neighbourhood with an old-building curiosity (a plaque or foundation stone, §6.2: engine-side) | Links to walking lore (§6); the layout stays on the client. After E10 |
| `dilemma.mine_safety` | A working mine (G3) | Already planned with G3; listed for the count |
| `dilemma.councillor_debts` | A `councillor_debts` secret (§5) whose creditor is pressing the councillor before a vote | The trigger is the debt, never the ledger; `knows` only picks the variant (a player who knows decides whether to use or shield it; one who does not is approached by the creditor). After E9 |
| `dilemma.journalist_source` | A journalist investigating a secret (§5) | Confirm, deny, or trade. After E9 |
| `dilemma.endorsement_price` | An endorser (V2) wants a policy for their backing | After V2 |
| `dilemma.broker_ultimatum` | The broker opposes a player bill (P1) | After P1 |
| `dilemma.strike_vote` | Plant-closure crisis, the union organising | |
| `dilemma.minor_party_courts` | A minor party strong in the player's ward | |

Four more for the campaign (after The Political Machine's warning and Not For Broadcast's staging; research §3.9, accepted 2026-11-19):

| Template | Trigger | The trade-off |
|---|---|---|
| `dilemma.campaign_gaffe` | Campaigning; the player made a press statement or ran an ad in the last 14 days | A remark taken out of context: clarify, apologise, or double down |
| `dilemma.hostile_doorstep` | Campaigning; a ward neighbourhood where the player's approval is low | A door slams on camera: engage, move on, or come back with an answer |
| `dilemma.promise_challenged` | Campaigning; a promise made (§3.5) that the budget or the rival's record contradicts | The rival calls it uncosted: cost it, scale it down, or stand on it |
| `dilemma.endorsement_contest` | Campaigning; an endorser (V2) courted by both the player and the rival | Pay their price, offer a smaller one, or go without. After V2 |

**Campaign-only templates and stages.** Today there are 4: `attack_ad`, `paid_crowd`, `debate_challenge`, and the rival storyline's `rival_offer` stage. With these four new ones there are 8, and 7 before V2, against a target of at least 6.

Templates after E3: 57 (31 + 26); after E8 turns the three requests into asks, 54 dilemmas. Seven of the new ones wait on another task and are enabled by their triggers: `mine_safety` (G3), `councillor_debts` and `journalist_source` (E9), `heritage_building` (E10), `endorsement_price` and `endorsement_contest` (V2), `broker_ultimatum` (P1). With E4 and E8: 9 storylines, 16 asks, 6 promises.

### 4.5 Storylines: 2–4 per career, NPC schemes in parallel

| Field (`director.jsonc`) | Today | Start |
|---|---|---|
| `storylines_active` | 2 | 3 |
| `storyline_spacing_days` (new) | — | 90 (two arcs never start within 90 days) |
| `min_storylines` (new) | — | 1 non-rival storyline active whenever one can start |

Starts are drawn among the eligible arcs, not taken in order (§4.2).

Six new storylines (from day: `min_career_days`, §4.7):

| Storyline | Starts when | Driven by | From day |
|---|---|---|---|
| `storyline.rival_scheme` | Between campaigns, the rival has the `oppose_player` goal or is the next candidate | The rival's NPC AI: courting the player's allies, lobbying against the player's bills, a story about the player's record. The rival arc no longer appears only in campaigns | 300 |
| `storyline.broker_scheme` | The broker has a stake and a bill touching it (P1) | The broker's lobbying, funding and endorsements (DEC-096) | 300 |
| `storyline.newcomer` | A new employer considers the town (sector capacity rising) | Site, tax ask, local protest | 0 |
| `storyline.investigation` | A journalist starts on a town secret (§5) | Media discovery; the player can help or hinder | 730 |
| `storyline.flood_recovery` | A disaster in a focus town (G1) | Relief, the inquiry, the defences | 0 |
| `storyline.big_promise` | A campaign promise of `promise.answer_crisis` | The promise's ask: the draft, the vote, the delivery or failure | 0 (needs a campaign) |

**NPC schemes are presented, not invented (DEC-033 B).** A scheme's stages advance on what NPCs actually do through their own AI (DEC-030). A new storyline condition reads a bounded log of NPC deeds kept by the NPC domain:

```jsonc
{ "type": "npc_did", "slot": "rival", "deed": "lobby_against", "target": "player_bill", "within_days": 60, "at": "same" }
// deeds: lobby_for, lobby_against, endorse, fund, statement, talk_to (a cast person), file
```

`PoliticsState` (or the characters domain, whichever `npc.rs` writes) gains `npc_deeds: Vec<NpcDeed>` (bounded at 200). Each `NpcDeed` carries `at: Jurisdiction` (where it was done), and only deeds in `state.focus()` jurisdictions are recorded, so M7 widens the filter without a migration. The `npc_did` condition gains an optional `at` selector (`same`, the default: the jurisdiction of the storyline's cast). Schemes run whether or not the player knows; their stage views are gated by the ledger (`RivalPlans`, `Broker`, K2), shown as leads until learned.

**The next move, once known** (after Into the Breach's telegraphed attacks; research §3.8, accepted 2026-11-19; the design reasoning is *inference*). When the player knows `RivalPlans` (the rival scheme) or `Broker` (the broker scheme) at `known`, the scheme's view shows its next stage as a pending move. The view gives the stage's key, its likely target (the cast bill, ally or neighbourhood its `advance` condition names), and its window (the stage's `wait_days` to `timeout_days` from the last stage). Below `known` it stays a lead. It is a view over content and the run: it adds no state and never changes what the scheme does. `StorylineView` gains `next: Option<NextMoveView { key, args, from: DateView, until: DateView }>` (E4).

### 4.6 The press covers the player's work

A new story source, symmetric for every politician (DEC-030). It is keyed by politician, so it needs no change for other tiers (DEC-098):

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

Perception uses the existing formula with a new severity-like weight `perception.work` 0.3 and framing by the outlet's lean. **Target** (mixed bot): at least 6 stories about the player in a 5-year career, at least half of them about work. Outlet capacity is unchanged, so scandals still lead when there are any. Press about the player is a minor beat, so it fills relax phases (§4.3).

### 4.7 Escalation over a career, and the second term

**New pressures year by year** (after Papers, Please and Not For Broadcast; research §3.9, accepted 2026-11-19). Dilemmas, storylines and asks may carry `min_career_days`: days since play began before the director or the asks system may spotlight them. It gates spotlighting only. The world's events happen regardless (DEC-033 B), secrets still seed, and it never reads the ledger. The first year teaches the loop, and each later year adds something to master.

| From | What may be spotlighted |
|---|---|
| Day 0 | Casework, local asks, council votes, the town's crisis, local life, the campaign |
| Day 300 | NPC schemes (`rival_scheme`, `broker_scheme`), secrets (`councillor_debts`, `journalist_source`, `ask.keep_it_quiet`), the broker's asks |
| Day 730 (about the first re-election) | `storyline.investigation` |

**A rising bar in the second term** (Democracy 4's "everything gets solved"; research §3.6, accepted 2026-11-19):
- **Consequences keep problems alive.** Each enacted law the player sponsored has a 50% chance (keyed per law) of a follow-up 12–24 months on: `dilemma.law_anniversary` now, and one follow-up per family once families land (F4 content, using the same `sponsored_law` condition).
- **Expectations rise for incumbents.** In a term after re-election, asks draw `deadline_days` from the lower half of their range (§3.6), and a promise already kept in the previous term adds no persuasion (§3.5).
- **V1 interaction.** V1's track-record decay sets how much first-term credit is left at the second election. With today's 10% monthly approval pull, an ask's bonus already halves in about 6.6 months (research §2.2, *inference*). V1 is tuned so that the second-term gates pass (§7.2: `reelection_delivery`, `second_term_active`): delivery last term is not coasting credit this term.

### 4.8 Conversations that remember (E13)

M6 §3.5 picks each part of a reply by `weight` after a repetition guard. Nothing makes an NPC react to what the player did ahead of small talk. Hades does this with story lines that take priority over filler, once-only lines, and tallies of the player's history. That reading of Hades comes from community sources and play: the exact Hades priority rules could not be verified (research §2.3), so what follows is *inference*, and its targets are starting points (research §3.4, accepted 2026-11-19).

**Line fields** (`TalkLineDef`, content):

```text
priority: u8                 // 0 filler (the default; fallbacks), 1 situational, 2 reactive (the player's recent deeds or a memory), 3 story (a storyline, secret or ask beat)
once: none | speaker | run   // said once per speaker, or once per run by anyone
after: [line ids]            // only after the player has heard all of these lines (from anyone)
```

**Selection, per part** (replaces steps 2–4 of M6 §3.5):
1. Gather the lines whose `when` holds.
2. Drop `once` lines already said in their scope, and lines whose `after` lines have not all been heard.
3. The repetition guard, as today: drop lines in the speaker's used memory while any other candidate remains. It works across all priorities, so C5's repetition gate is unchanged.
4. Take the highest `priority` that has a candidate, and draw by `weight` within it with the existing keyed draw (`Stream::Dialogue`, same key).

Filler fills only when nothing reactive or story-bearing applies.

**New conditions** on U1's one condition type (M6 §3.3 and §4, task U1):

```rust
/// The player did this within the window (the deed log, §3.6).
/// scope: speaker (the deed concerned the speaker: their bill, a talk with them, their ask),
///        speaker_home (in the speaker's home neighbourhood), here (at this place).
PlayerDid { deed: DeedKind, within_days: u32, scope: DeedScope },   // gives $deed_key, $days, $count
/// An ask from the speaker is accepted (open), kept or broken, within the window.
AskOutcome { slot: String, status: AskOutcomeKind, within_days: u32 }, // gives $ask_key
```

`Remembers` gains `$count`: how many memories of those kinds the speaker holds ("third time you've come asking about the plant"). Examples: "You voted for the depot. People here noticed." (`PlayerDid { deed: vote, scope: speaker_home }`); "Still waiting on that drain." (`AskOutcome { status: accepted }`).

**State.** `TalkState.heard_once: Vec<HeardOnce { line: ContentKey, speaker: Option<CharacterId>, on: SimDate }>`, kept sorted by (line, speaker) and bounded at `talk_params.heard_once_cap` 400 (no `HashSet`, DEC-041). It records the lines that are `once` or are named in some line's `after`, from conversations and encounters (§6.5). Over the cap the oldest entry goes, so a once-only line may return after years: acceptable.

**Content.** C3's 292 lines get priorities (fallbacks and condition-free lines 0, conditioned lines 1). E13 adds about 20–30 reactive and story lines, so that about 10% of substance lines have priority ≥ 2.

**Validation.** `priority` ≤ 3. Fallback lines are priority 0 and never `once` (a topic can always answer). `after` names existing lines, with no cycles. A reactive line (priority 2) carries a `PlayerDid`, `AskOutcome` or `Remembers` condition. `$count` is used only with `Remembers` or `PlayerDid`.

**Metrics** (C5's `talk` and E12's `engagement` report them):
- `once_violations`: a `once` line said twice in its scope while under the cap. It must be 0 (a gate, like C5's repetition violations).
- `reactive_share`: of talks with a recurring speaker (someone the player has talked to before in the run), the share whose reply carries a line of priority ≥ 2. Target ≥ 25% in 5-year careers.
- `line_repeat_rate`: the share of substance lines that this speaker has said to the player before in the run. The guard remembers only 12 lines per person, so this is the long-run repetition. Reported; E12 sets a threshold after the first runs.

---

## 5. Secrets (DEC-097 §4)

### 5.1 What a secret is

A secret is a hidden fact (DEC-034) about a person in a focus jurisdiction (in M6, the home town), seeded from live state by a content registry. It reuses `narrative.hidden`: subject, evidence, discovery window and fate. Journalists, the rival, auditors and investigators discover secrets by the existing rules, so a secret can surface whether or not the player finds it. Not every secret is a misdeed: `severity` is optional (a private matter has no legal path but still has newsworthiness).

### 5.2 Kinds (the first registry)

| Kind | Seeded from | Subject | Who knows (for conversations) | Severity | Leads to |
|---|---|---|---|---|---|
| `secret.broker_deal` | P1: the broker's lobbying of a member of the legislature (a councillor, in a town) succeeds with a favour or money | The member and the broker | Their staff, the union steward, the diner owner | misconduct | The member's staff; the broker |
| `secret.buried_report` | Pollution over a threshold with a plant or mine owner; or the `bury_report` dilemma option | The owner (or the player) | Plant workers, a nurse, the clinic | misconduct | The clinic; the plant's records |
| `secret.rival_backer` | Campaign money from the broker or a donor to the rival (P1, the funds path) | The rival | The party chair, the backer's circle | ethics | The backer; `RivalPlans` |
| `secret.councillor_debts` | At play start and monthly, a member of the legislature (by post, `members_of`) with low integrity and high ambition (keyed per person) | The member | Their creditor, the diner owner | private | The creditor |
| `secret.steered_contract` | NPC corruption's existing hidden facts (`steer_contract` kinds) | The NPC | Contractors, the town clerk | crime | The town hall records |
| `secret.planned_closure` | A plant at risk by the same inputs the economy uses to close plants (its sector's job-loss trend and margins) | The plant owner | Managers, the union steward (rumour) | private | The union steward |
| `secret.land_deal` | Someone bought land beside a site before a rezoning bill (a family binding a neighbourhood) | The buyer and a member of the legislature | The estate agent, the planning clerk | ethics | The planning clerk |
| `secret.party_challenge` | A faction plans to deselect a post holder of its party in the jurisdiction (the executive, a member, or the player; party Standing) | The faction leader | Party members | private | The party chair |

Caps: at most `secrets.max_live_per_jurisdiction` 6 unsurfaced secrets per jurisdiction; seeding draws on a keyed stream (`Stream::Media`, keyed by `[month, kind, at.tier_index(), at.raw(), cast]`). Seed triggers are U1 conditions and may not use `knows` (K6, in U1's validator). Kind ids are names: who a kind casts is read from posts, never from the word in its id.

**Every find leads somewhere** (after Outer Wilds; research §3.8, accepted 2026-11-19). Each secret kind declares at least one `lead`: a person, place or subject the notebook points to next (the "Leads to" column). Validation enforces it (E9). The notebook shows per subject that there is "more to learn", never a total (M6 §1.8).

### 5.3 How the player finds them

| Route | Level reached | Cost |
|---|---|---|
| Talk (DEC-093): someone in the kind's `knows` list, within their discretion (M6 §3.4) | Cool or neutral: `heard` (a rumour); warm: `known`; a friend with trust: `detailed` | Half a slot (the talk) |
| Overheard lines and street encounters (§6.5) | `heard`, with a lead | Free |
| **Dig into it** (`Action::Investigate { secret }`) at the news office (with a journalist met), the town hall records, or the old union archive (a hidden place, G2) | `known` → `detailed`; odds `0.35 + 0.3 × intrigue + 0.2 if known`, shown first (L1 records them) | 1 slot |
| A storyline step or a dilemma option (`learn`) | As stated | — |

Detail needs evidence: `detailed` takes a second independent source or a successful dig. The notebook lists each secret with where it was learned and a lead to the next level (M6 §1.8).

**Finds become talk** (after Disco Elysium's "loot for your mind", without a new system; research §3.8). A secret the player holds at `heard` or above becomes an `about` option for `topic.about` (`AboutKind::Secret`) with the people in its kind's `knows` list. Exploring feeds the conversation loop.

### 5.4 What a known secret does (never decisive alone)

- **Tip off the press** (`Action::TipPress { secret }`, news office, 1 slot, needs `known`). The fact's fate becomes discovered by a journalist now; coverage, perception and the legal path follow the existing rules. The subject learns the source with chance `secrets.source_exposed` 0.25 (memory `exposed_me`).
- **Framing.** Dilemmas, scenes and lines may pick a variant by `knows` (allowed: framing, DEC-091). A known `planned_closure` lets the player act early (draft the closure response before the crisis).
- **Asks.** `ask.keep_it_quiet` (§3.8) and a dilemma `councillor_debts`.
- **No direct leverage in M6** (owner's answer, §9 question 4). Pressure may come later as a shady act with its own hidden fact (DEC-034).

**Caps and the gate.** A probe bot `secret_hunter` (mixed plus every dig and tip it can) is held by `secrets_not_decisive` (§7.2), which also bounds what an omniscient run's secrets are worth. Knowledge in general must pay a clear margin (`knowledge_matters`); secrets in particular are one small part of it.

### 5.5 Engine and save

- `HiddenFact` gains `secret: Option<ContentKey>` (the kind) and `cast: Vec<CastEntry>`. In the same save bump (DEC-098): `HiddenFact.town: Option<u32>` becomes `at: Option<Jurisdiction>` (migration: `town: m` → `{ "type": "municipality", "id": m }`), and hidden facts take a typed `HiddenId` (M6 §6 item 12).
- **One fact-kinds registry** (DEC-098 §4; audit §4.2 row 8). `content/base/fact_kinds.jsonc` lists every kind of fact that can become a scandal or a secret: kind id, severity (optional: a private matter has none), evidence, a newsworthiness override and Fluent keys. The shady acts and NPC acts in `media.jsonc` and the secret kinds in `secrets.jsonc` reference it by id instead of carrying their own strings. Validation: every reference names a registered kind. Mods add misdeeds as data.
- A `secrets` step in the narrative domain seeds them monthly (it writes its own `hidden` list). Seeds from other domains (P1's deals, NPC corruption) arrive as effect requests (`Effect::Hide { kind, cast, evidence }`).
- `talk_params.speaker_knows` gains `KnownSubject::Secret { kinds }` rules from each kind's `knows` list; C3-style lines teach `secret` subjects; `AboutKind::Secret` makes a held secret a topic.
- `Subject::Secret { hidden }` exists already (K1).

---

## 6. Walking rewards (DEC-097 §5)

### 6.1 The kinds registry

`content/base/curiosities.jsonc`, one entry per kind:

```jsonc
{ "id": "curiosity.plaque", "name_key": "curiosity-plaque", "category": "lore",
  "anchors": ["wall", "bridge", "square"],          // where the client may place it, in order of preference
  "eligible": [ { "type": "history", "fact": "past_executive" } ],   // the town must have this to say (a past holder of its executive office); a U1 condition
  "per_town": [1, 3], "weight": 1.0,
  "reveal": "walk",                                  // walk | streets
  "when": null,                                      // optional: slots (evening), a season, a storyline step
  "art": "prop.plaque",
  "text_key": "curiosity-plaque-text",               // args from the instance (a name, a year)
  "reward": [],
  "leads": [ { "to": "subject" } ] },                // where the notebook points next, if the subject is living
```

| Category | Kinds (first pack: 14) | Reward | `reveal` |
|---|---|---|---|
| Lore | plaque (a past mayor, a year), mural (the defining industry), old campaign poster (a past race and its result), bench with a story (a disaster year, a person), foundation stone (the founding era), war or pit memorial | A notebook lore entry, nothing else (a kind that also teaches something, such as a dedication naming a living former mayor, is a separate `streets` kind) | walk |
| Find | lost cat, a petition on a noticeboard, slogan on a wall | Cat: a casework-like ask from its owner (return it: owner +10, `helped_me`). Petition: an `ask.town_hall_here` offer and the neighbourhood's worries `heard`. Slogan: the neighbourhood's top worry `heard` | streets |
| Vista | the view from the hill (highest level), the harbour wall at dusk | A dated `read` of the neighbourhood worries the player already knows (as the town data desk; once per 90 days), health +3 once a month | streets |
| Easter egg | a shop sign, a poster for a long-gone band, a cat that is always on the same wall | Nothing (in-world only, owner's answer §9 question 5; heightened realism, DEC-061) | walk |

Rare **encounters** (§6.5) are people, not registry entries.

Variety by town: kinds are eligible only where the town can ground them (`eligible` conditions on history, industry, geography: a pit memorial needs a mine, the harbour wall a coast), and `per_town` with keyed draws gives each town its own set. The registry is data, so packs add kinds with no code; anchors are the one extension point (code, §6.3).

**Every find leads somewhere** (research §3.8, accepted 2026-11-19). Every `streets` kind, and every lore kind whose subject is a living person, declares at least one `lead` (a person, place or subject). Pure-flavour `walk` kinds are exempt. Validation enforces it (E10).

### 6.2 Seeding (engine, deterministic)

At play start, and when the player enters a new zone (M7), the engine creates curiosity instances for the town. The seeding function takes the town as an argument (`seed_curiosities(state, content, town: MunicipalityId)`), never reading `player.municipality`: `world.curiosities: Vec<Curiosity>` (append-only, typed `CuriosityId`, never reused; created after all places so place ids never shift, as TOWN_GENERATION §11 requires for optional places).

```rust
pub struct Curiosity {
    pub id: CuriosityId,
    pub kind: ContentKey,
    pub town: MunicipalityId,             // kept (DEC-098): curiosities are physical things in a town
    pub hood: NeighborhoodId,
    pub args: BTreeMap<String, String>,   // facts from history: a name, a year, a result
    pub subject: Option<CastEntry>,       // a person or place it teaches about
}
```

- Density: `curiosities.per_district` 1.5 (12–30 per town), at most 40% `streets` (play-affecting).
- Draws: `stream(seed, "curiosity", town, kind)` keyed draws; neighbourhoods weighted by their anchors' likelihood (a vista needs high ground: the engine reads `TownGeographyView`'s relief, B2).
- Arguments come from the simulation's own history (the burn-in's office holders, elections, disasters and plant closures): lore is true to the town.
- **Lore that bears on now** (Caves of Qud's "pick the event, then explain it"; research §3.8, accepted 2026-11-19). When a kind's arguments are drawn from history, facts that share a kind with the live crisis or the defining industry weigh ×`curiosities.live_weight` 2.0: a flood plaque in a flood-prone town, a pit memorial in a mining town. Still keyed and deterministic.
- **Lore attaches to known ground.** A curiosity's `subject` is a real person or place of the town. Seeding never depends on what the player knows (DEC-091), but text may: lore lines carry variants chosen by `PlayerKnows` ("the plaque names Dana Price's grandmother" once Dana is known). That is framing, which is allowed.

### 6.3 Placement in layout v2 (client)

- **Plan, stage 11 (props):** reserve `curiositySlots` per district by anchor kind: up to 2 `wall` (a street-facing wall cell beside a pavement), 1 `bench` (an existing bench prop), `bridge` (each bridge's end), `high_ground` (the highest walkable cell in the district), `waterside`, `park`, `square`, `back_lane`, `street_end` (bollards and field gates), `memorial`, `station`, `market`. The slots are part of the plan (`TownMap.curiositySlots: { anchor, x, y, district }[]`); they never block a door or route (stage 11's rule).
- **Dress (D2):** each engine curiosity takes the free slot of its district whose rendezvous hash `h(townSeed, curiosity id, slot index)` is lowest among its preferred anchors, in the registry's order, assigned in id order. New curiosities only take free slots, so nothing already drawn moves (DEC-085). With no matching slot, a `walk` curiosity is not drawn and a `streets` one is still found by "Look around" (§6.4).
- **Hidden-place cues** (G2: alley gate, basement steps, overgrown path, lookout bench) use the same slot mechanism with their own anchors.
- **Art:** new prop art keys `prop.plaque`, `prop.mural`, `prop.poster_old`, `prop.cat`, `prop.noticeboard`, `prop.memorial_stone`, `prop.telescope`, `prop.slogan` (pixel-art P1; placeholders until then, with the fallback chain of TOWN_GENERATION §7.3).
- Plans change shape (`curiositySlots`), so the pinned v2 plan hashes are refreshed. No run stores a layout version yet, so `LAYOUT_VERSION` stays 2 (TOWN_GENERATION "In the game").
- **Placement check** (after Breath of the Wild's heat maps; research §3.8, accepted 2026-11-19). From the Playwright walker or a scripted set of paths, E11 computes per district how often each slot is passed within 3 tiles. It flags districts where more than 30% of slots are never passed. This is a test-coverage report; it changes no state.

### 6.4 Discovery, parity and the engine–web split

| Step | Web (presentation and input) | Engine (authority) |
|---|---|---|
| Show | Draws a marker prop in its slot (unfound and found look the same in the street; found ones get a small tick in the map) | `TownStateView.curiosities: [{ id, kind, art, hood, found }]` |
| Walk-find | The player walks next to a marker and taps it: sends `PlayerCommand::Inspect { curiosity }` | Checks the player is on that neighbourhood's streets (`Location::Street`), that it is not found yet and its `when` holds; applies the reward once; records `Subject::Curiosity { id }` at `known` in the ledger; the outcome carries the text key and arguments |
| Look around (parity) | Places tab: on a neighbourhood's streets, "Look around" | `PlayerCommand::LookAround`: finds every `streets` curiosity present there whose `when` holds. Free, as walking is |
| Notebook | A "Town lore" section: found lore with its text, its lead, and a count of what has been found (never a total, M6 §1.8) | `NotebookView` section `lore` |
| Talk | A found curiosity's subject appears as an `about` option for `topic.about` | `AboutKind::Curiosity`; residents of its neighbourhood know its curiosities at `known` (a `speaker_knows` rule) |

**The accessibility rule (DEC-051, DEC-097 §5).** Anything with a reward (`streets`) is found by `LookAround` from the Places tab or quick mode, under the same conditions and with the same result. Only `walk` curiosities (reward: lore text, no play effect) are walk-only. Validation enforces it: a `walk` kind may not have reward effects; a plaque that teaches a person must be `streets`.

The client never decides a reward: it only says which marker the player tapped. The engine cannot check tile adjacency (layouts live on the client, DEC-085), so it checks the neighbourhood. That is enough for a single-player game; nothing competitive rests on it.

### 6.5 Encounters

When the player arrives on a neighbourhood's streets (`MoveTo` or `Go` to `Location::Street`, from any mode), the engine may produce an encounter with a real person whose routine puts them there: a resident of that neighbourhood with something to say, a journalist off duty (their office's neighbourhood), the broker taking a walk (their home neighbourhood), the rival canvassing (in a campaign).

| Parameter (`curiosities.encounters`) | Start |
|---|---|
| `chance` per street arrival | 0.08 |
| `max_per_turn` | 1 |
| `person_cooldown_days` | 120 |
| `max_per_year` | 8 |

An encounter is a free exchange of one topic chosen by the engine (the person's best: a secret `heard` with a lead if they know one and will tell, an ask, else small talk), through the conversation engine (C1, with E13's priorities) with no slot cost and the usual repetition guard. Keyed draw `stream(seed, "encounter", date, hood)`; the draw is made on every street arrival, whatever the player knows.

**Familiarity first** (Outer Wilds: a mystery intrigues once the player knows the place it disturbs; research §3.8, accepted 2026-11-19). In the first year, the encounter's person is weighted toward people the player has talked to (the talk log) and neighbourhoods the player has been to (arrivals). These are the player's deeds, not the ledger, and the draw count is unchanged, so new mysteries attach to known ground without breaking DEC-091.

### 6.6 Reward sizes and caps

- Every reward is one-off per curiosity (the ledger records it), except the vista (once per 90 days, health once a month).
- Expected play-affecting finds per town: 6–12, about the value of two canvasses in total.
- Lab: the `walker` probe (mixed plus `LookAround` on every street arrival) is held by `walking_is_a_bonus`, `explore_only` by `exploring_alone_never_wins` (§7.2); the blind-invariance test covers curiosities and encounters.

---

## 7. The engagement gate (DEC-097 §6)

### 7.1 The command

`mandate-cli engagement --seeds N --years Y [--strategies mixed,canvass,idle] [--set …] [--tables]` plays careers with the bot strategies (the `bots --careers` loop) and prints one row per career and medians per strategy. It exits non-zero when a gate fails, like `bots`. It also accepts `--set` and `--grid` (DEC-078). Careers use the Classic profile.

Metrics per career:
- decisions, distinct templates, uses of the most-used template, repeats within a chapter, distinct climax templates;
- pacing (§4.3): peaks per chapter, the theme of each chapter, the longest relax, the share of relax phases with a minor beat, the longest decisions-only gap;
- the longest quiet stretch: the most days with no beat (a decision opened, an ask offered or resolved, a storyline stage, a press story about the player), separately for governing and campaigns;
- storyline templates started, the most active at once, scheme stages that advanced outside a campaign;
- press stories about the player, by source (work, scandal, case, statement);
- asks offered, accepted, kept, broken, declined, void; promises made, kept, broken;
- secrets seeded and learned; curiosities found (walker only);
- conversations (§4.8): `once_violations`, `reactive_share`, `line_repeat_rate`;
- the approval sources split (§2.3);
- elections won and lost (5 years covers two local elections), and decisions per chapter in each term.

### 7.2 Thresholds

Starting thresholds come from the measurements in §1.1, the cycle's bounds (§4.3) and research §3.5–§3.7 (accepted 2026-11-19). E12 tunes them, and BALANCE_LOG records each change.

**Career gates** (`engagement`, `mixed` unless stated, 5-year careers, Classic). Medians are across seeds; "every seed" is a floor or ceiling per career.

| Gate | Today (mixed, 5 y, 12 seeds) | Median | Every seed |
|---|---|---|---|
| Distinct decision templates | 11.5 (9–17) | ≥ 18 | ≥ 14 |
| Decisions | 34.5 (22–38) | 30–50 | ≥ 22 |
| Most uses of one template | 6 (5–6) | ≤ 2 | ≤ 3 |
| Repeats within a chapter | 1 (0–3) | 0 | 0 |
| Same climax template in consecutive chapters | 10 of 12 careers (4 y) | 0 | 0 |
| Same theme in consecutive session chapters while another was eligible | — | 0 | 0 |
| Longest decisions-only gap, governing (days) | 168 (154–210) | ≤ 120 | ≤ 145 |
| Longest quiet stretch, governing (days, any beat) | 168 (154–210), decisions only | ≤ 60 | ≤ 90 |
| Longest quiet stretch, campaign (days) | — | ≤ 21 | ≤ 28 |
| Relax phases with at least one minor beat | — | ≥ 50% | — |
| Storyline templates started | 2 (1–3) | ≥ 3 | ≥ 2 |
| A scheme stage outside a campaign | 0 | ≥ 75% of careers | — |
| Press stories about the player | 0 | ≥ 6, half on work | ≥ 2 |
| Asks offered | — | ≥ 15 | ≥ 8 |
| Asks kept, of those accepted | — | ≥ 60% | — |
| Asks broken | — | ≥ 1 in 25% of careers; ≤ 30% of accepted | — |
| Canvass share of approval gained between elections | not measured | ≤ 40% | — |
| `idle`: press stories about the player | 0 | ≥ 1 (the thin record is reported) | — |
| Conversations: `once_violations` (§4.8) | — | 0 | 0 |
| Conversations: `reactive_share` (§4.8; `mixed` and `explorer_coalition`) | — | ≥ 25% | — |

Two gates compare across careers:
- `reelection_delivery`: `canvass`'s re-election rate (the second election in a 5-year career) ≤ `mixed`'s. Delivery should matter most for an incumbent.
- `second_term_active`: `mixed`'s decisions per chapter in its second term ≥ 80% of its first-term rate (research §3.6).

**The gate ladder** (`bots`, first election, mean vote share and win rate, 16 and 32 seeds). It encodes the owner's balance principle (DEC-097; DEC-077 §5 amended): single-verb routines < a blind mix < informed play, knowledge worth a clear margin, and several informed styles that each win somewhere. The strategy classes are those of §2.4: single-verb routines and blind mixes together are the **rote routines**; probes are compared only by their own gates. It replaces the earlier gate list and the amended rows added after the owner's principle. Each gate has one name and one threshold.

| Rung | Gate | Rule (start) | Today (§1.1) | Enforced |
|---|---|---|---|---|
| 0 | `idle_cap` (was `idle_rarely_wins`) | `idle` wins ≤ 15% of careers | 6.2%: passes | Now |
| 0 | `effort_matters` | The best strategy wins more often than `idle` | Passes | Now |
| 0 | `exploring_alone_never_wins` | `explore_only` wins ≤ `idle` + 5 points, and less often than the median rote routine | Passes | Now |
| 1 | `ladder_rote` | `mixed` ≥ every single-verb routine + 1.0 point of mean share, and wins at least 5 points more often than each. It merges the earlier `single_verb_not_best` and the research's `ladder_rote` | `mixed` 0.618 vs `canvass` 0.630: fails | From E8 (reported from E1) |
| 2 | `ladder_learned` (was `exploring_pays`) | `explorer` ≥ `mixed` + 3.0 points of mean share, and wins at least as often. Raised from + 1.0 by the balance principle; research §3.5's + 1.0 was today's value | + 1.2: passes at + 1.0 | + 1.0 until E8, + 3.0 from E8 |
| 2 | `knowledge_matters` | `mixed --omniscient` ≥ `mixed` + 3.0 points of mean share. Knowing the town must be worth at least what `ladder_learned` asks learning to pay (research §3.5 proposed + 1.5; raised so the ladder is consistent). Runs `mixed` only under the switch (16 careers, ≈ 2 s) | Not measured | From E8 (reported from E1) |
| 3 | `skill_gap` | The best informed style ≥ the best rote routine + 3.0 points of mean share and + 10 points of win rate | `explorer` = `canvass` at 0.630: fails | From E8 (reported from E1) |
| 3 | `good_styles_vary` (replaces `variety` and `no_dominant_strategy`) | Among the informed styles: (a) at least 2 whose mean share is within 1.5 points of the best style's are each the sole best in ≥ 20% of worlds; (b) none is among the best (within 0.2 points) in more than 60% of worlds; (c) the best style differs by starting situation (crisis, industry) | Not measured (no variants yet) | (b) from E1; all from E8 |
| — | `corrupt_not_dominant` | `corrupt` wins ≤ the best honest strategy + 10 points | Passes | Now |
| — | `walking_is_a_bonus` | `walker` ≤ `mixed` + 1.0 point of mean share | — | From E10 |
| — | `secrets_not_decisive` | `secret_hunter` ≤ `mixed` + 1.0 point of mean share; under `--omniscient`, secrets move `mixed` by ≤ 1.0 point | — | From E9 |

Under `--omniscient` (a lab baseline) `ladder_learned` is reported, not enforced, as `exploring_pays` is today.

**Why (owner, 2026-11-19).** Players optimise toward whatever wins, so the winning play must be the engaging one. The old variety gate failed whenever the explorer, the intended way to play, rose to the top; §1.3's grid shows exactly that. The principle replaces "every routine roughly equal" with "good play clearly beats rote play, and good play has several styles that suit different towns".

**How the ladder can pass without flat bonuses** (Sid Meier's "situational" decisions; research §3.5). Each verb's value must depend on what the player knows of the town: the canvass `fit` reads the known worry (§2.1), and asks and promises pay most when aimed at known concerns. If `knowledge_matters` fails, widen the `fit` spread; do not give knowledge a flat bonus. `walking_is_a_bonus` and `secrets_not_decisive` concern the input mode and single shortcuts, not knowledge in general, so they sit beside `knowledge_matters` without contradicting it.

### 7.3 CI and nightly

| Run | What | Measured cost today | Budget |
|---|---|---|---|
| CI (beside `bots --seeds 16`) | `engagement --seeds 8 --years 5 --strategies mixed,canvass,idle,explorer_coalition` | ≈ 0.6 s a career: 32 careers ≈ 20 s | ≤ 60 s after E6–E13 add work |
| CI | `bots --seeds 16` with the new strategies (`asks_only`, the three informed variants, `walker`, `secret_hunter`) and the `knowledge_matters` pass | 18 s for 9 strategies today; ≈ 30 s for 15, plus ≈ 2 s | ≤ 45 s |
| Nightly | `engagement --seeds 32 --years 8 --strategies mixed,canvass,idle,explorer,asks_only --tables` | ≈ 1 s a career: 160 careers ≈ 3 min | ≤ 10 min |
| Nightly | `bots --seeds 64` | ≈ 70 s | — |

Per the owner's answer (§9, question 6), the engagement gates report only (nightly, no failure) until E2–E8 have merged, then block in CI. The conversation gates join when E13 merges. The ladder in `bots` is enforced in steps, as its last column says.

### 7.4 Telemetry tables

After Slay the Spire's pick and win rates and Democracy 4's event trigger tables (research §3.7, accepted 2026-11-19), `engagement --tables` prints, across all careers:
- **per template:** the share of careers where it opened; mean opens per career; the share answered with each option; each option's mean approval and Standing change after 90 days; "never opened in 32 seeds" and "eligible but never picked" (E2 exposes each week's candidate set to the lab as a pure function, with nothing added to state);
- **per verb:** uses per career and the marginal approval per use, by season, for each strategy;
- **per ask template:** offered, accepted, kept, broken.

Checks (report-only at first, like the other engagement gates):
- `template_never_opened`: a template opens in none of 32 seeds and is not listed as gated on a later task (E3's rule, made automatic);
- `template_always_there`: a template opens in more than 60% of careers *and* more than twice per career;
- `option_non_decision`: an option picked by `mixed` in more than 90% of its opens. That is not a real decision: rework or cut it.

The tables say *what* happens. The playtest notes in BALANCE_LOG remain the record of how it feels.

---

## 8. How this fits the M6 tasks

| M6 work | Interaction |
|---|---|
| **Talk work C2–C5** | E1–E5 run beside them (DEC-097 order). E13 (conversation memory, §4.8) starts after C2 and C4 merge and runs beside C5. It extends C1's line selection with `priority`, `once` and `after`, adds the `PlayerDid` and `AskOutcome` conditions (reading E6's deed log), and adds its metrics to C5's `talk` report. Asks can come up in conversation (a reply's substance may offer an ask; DEC-035's "promise" option type); `Need::Talk` is met by `Talk`; encounters use the C1 engine with no slot cost; secrets are taught by lines (C3 format, new subjects); held secrets and found curiosities become `topic.about` options (E9, E10); C5's repetition gate and the engagement gate share the lab command style |
| **Policy families F1–F7** | Asks land before F1 (DEC-097). `Need::Introduce` and `Need::Enacted` name a template until F2b, then a family instance (the `policy::resolve` seam of F2a); a sponsor's price can be an ask ("back my next bill"); campaign promises draw on open proposals; F7's need-aware reactions feed V1's credit, which feeds law-result press stories (§4.6). F4 adds one follow-up per family for the second-term bar (§4.7) |
| **V1, V2, P1, A1** | V1 is the main "delivery" source of approval (§2.3): E1 is tuned before V1 and re-checked after it, and V1's decay is tuned against the second-term gates (§4.7). V2 adds endorsements as ask rewards and enables `endorsement_price` and `endorsement_contest`. P1 drives `storyline.broker_scheme`, `ask.broker_vote`, `secret.broker_deal` and `secret.rival_backer`. A1's intrigue sets dig odds; organising could scale the canvass `awareness` (A1's own lever, not changed here) |
| **Hidden places G2** | E9–E11 run with it (DEC-097 order): hidden places are a walking-reward category; their cues use the curiosity slots; the union archive is a dig site; `LookAround` and G2's walking discovery share the street-arrival hook |
| **Odds calibration L1** | E12 runs beside it. New shown odds go into L1's recording list: casework success, dig odds, and a press tip's chance of running |
| **K4 bots** | The informed styles (§2.4) build on K4's `explorer` and `Knowing`; the gate ladder (§7.2) replaces K4's gate list in `bots` |
| **R1, R2, U1 (DEC-098)** | R1 (offices from the constitution, engine and save) comes before E2, E4, E6 and E9, so asks, deeds, secrets and the director are written against posts, `Jurisdiction`, `Area` and `state.focus()` from the start; E1 runs beside it. R2 (protocol and web) runs beside E6–E8; it and E7 both edit `protocol/src/{lib,places}.rs` and `web/src`, and the second to merge rebases. U1 (one condition language) comes after E2 and before E3, E4, E6 and E8: every new condition in this document (`answered`, `sponsored_law`, `npc_did`, ask triggers, secret seeds, curiosity `eligible` and `when`, `PlayerDid`, `AskOutcome`) is a variant of U1's one type, checked by its one validator |
| **K6** | Its validation rule (no `knows` where a condition decides whether something happens) extends to ask triggers, secret seeds and curiosity eligibility; U1 moves it into the one condition validator. Themes, tiers, phases and `min_career_days` read live state and dates, never the ledger; the scheme telegraph (§4.5) is a view only |

---

## 9. Questions for the owner (answered 2026-11-19: all recommendations accepted)

| # | Question | Answer (owner, 2026-11-19) |
|---|---|---|
| 1 | Should canvassing between elections still raise approval at all? | **Yes, a little** (0.6× today on a first visit, falling fast on repeats). Awareness is unchanged, so canvassing stays the way to be known; approval between elections comes from delivery |
| 2 | Casework: a free-standing verb, or only through asks? | **Only through asks.** A free-standing "help someone" action would become the new grind; tying it to asks keeps it bounded and grounded |
| 3 | How many campaign promises, and should broken ones make the press? | **Three per campaign; broken public promises make the press and give the rival material.** It makes the campaign and the term one story |
| 4 | Can a known secret be used to pressure someone? | **Not in M6.** Tipping the press and framing are enough. Pressure (blackmail) later as a shady act with its own hidden fact (DEC-034), if wanted |
| 5 | Easter eggs: in-world only, or the odd fourth-wall joke? | **In-world only** (heightened realism, DEC-061): odd shop signs, an old band poster, the cat on the wall. No completion reward for lore, so quick-mode players lose nothing |
| 6 | Should the engagement gate block CI from the start? | **Report-only (nightly) until E2–E8 merge, then block in CI.** The thresholds need a few runs on the new content first |

**Research proposals accepted (owner, 2026-11-19: "Yes do it").** `ENGAGEMENT_REFERENCES.md` §3 is folded in: the pacing cycle (§4.3), tiers and chapter themes (§4.2), conversation memory (§4.8, E13), the gate ladder (§7.2), knowledge as progression (§5.2–§5.3, §6.1–§6.5, §4.5's telegraph, §7.4's tables), and the smaller ideas: the sliding ask rate (§3.2), the second-term bar and pressures by year (§4.7), and campaign-only templates (§4.4). No question is open.

---

## 10. Risks

- **Balance churn.** The ladder asks for a margin a player can feel (+ 3 points of share, + 10 points of win rate), which flat tuning cannot give: §1.3 shows a flat cut clears `ladder_rote` and nothing above it. Mitigation: make value situational (the `fit` spread, asks and promises aimed at known concerns, §7.2). E1 tunes on a grid with the season-aware curve, re-checks every enforced gate at 16 and 32 seeds, and is re-checked after E8 and V1. The thresholds are starting points: E12 may move them with evidence in BALANCE_LOG, but never to the point where rote play ties informed play.
- **Informed styles collapsing into one.** If one variant wins everywhere, the ladder passes but the game has a new routine. `good_styles_vary` catches it; the fix is to tie each style's strength to the town's situation (crisis, industry, a hostile council), not to nerf the winner flatly.
- **Pacing that feels mechanical.** A visible rhythm could feel like a timer. Phase lengths are drawn, relax ends early when the player acts or a crisis begins, and world-raised decisions never wait.
- **Asks as busywork.** A to-do list is not a story. Caps (4 open), the sliding rate (fewer offers when busy), a live reason for every ask, no fetch quests, and declining costs little.
- **Callbacks that misfire.** A reactive line with the wrong scope ("You voted for the depot" to someone in another ward) is worse than filler. Validation of conditions and scopes, and the C3-style render test over four towns, cover it.
- **Notification overload** on the Today tab with decisions, asks and promises together. One sorted list by deadline, a badge count, and a collapsed "recent" block.
- **The client–engine split for curiosities.** The engine places curiosities by neighbourhood; the client finds slots. A town with too few slots draws fewer markers; `streets` curiosities stay reachable by Look around, so play is unaffected. The placement check (§6.3) flags districts nobody walks past.
- **Secrets tilting play.** Capped uses, no leverage in M6, and `secrets_not_decisive` with its omniscient check.
- **Press volume.** Work stories could crowd out scandals and town news; outlet capacity is unchanged and work stories score below a breaking scandal.
- **Content volume.** About 26 dilemmas, 6 storylines, 16 asks, 6 promises, 8 secret kinds, 14 curiosity kinds and 20–30 reactive lines, each with Fluent text, plus themes on every dilemma: content tasks are split so they can run in parallel with the engine work. The telemetry tables (§7.4) show which templates to cut.
- **CI time.** About 20 s more for the engagement run (budget 60 s) and about 14 s more for `bots` (budget 45 s).
- **Save churn.** Seven bumps (E1, E2, E4, E6, E9, E10, E13), each additive with serde defaults; the lead numbers them at merge and may fold neighbours together (M6 §4 rules).

---

## 11. Task breakdown

**Order (DEC-097 and DEC-098):**
1. E1 runs beside R1 (R1 runs after N1, as a short freeze on its files). E1 keeps to the `Canvass` arm of `actions.rs`; until R1 merges, its reads of the player's town go through one local helper that R1's `player::home(state)` replaces.
2. E2 after R1; then U1 (one condition language, `M6_ENGINE_CONTRACTS.md` §4); then E3 and E6. E4 also follows U1 (it adds `npc_did`); E5 needs only E2.
3. E7 and E8 after E6, with R2 (offices from the constitution: protocol and web) beside the asks work.
4. E6–E8 before the policy families (F1).
5. E9–E11 with hidden places (G2); E12 beside L1; E13 after E6 (C2 and C4 are done), beside C5.

**Rules:** as M6 §4: the lead numbers save bumps at merge; append-only edits to shared enums (`ContentFile`, `ContentBundle`, `QueryDto`, `ResponseBody`, `Effect`, `EventKind`, `PlayerCommand`, `Action`, U1's `Condition`, `AboutKind`, `Deed`); `CARGO_BUILD_JOBS` set per brief. E9 and E13 both append to `content/base/conversations/lines.jsonc` and `talk.ftl`, and E4, E10 and E13 each add variants to U1's `condition.rs` files (content and sim); where they overlap, the lead merges them.

| # | Task | After | Runs beside | Save bump |
|---|---|---|---|---|
| **E1** | Ground-game curve and record; informed bot styles; the gate ladder's names and classes | K4 | R1, C5, E2 | yes |
| **E2** | Director: cooldowns, tiers and themes, `pick`, the pacing cycle, `uses`, `answers`, `answered`, `sponsored_law`, `min_career_days`, story metrics; the session's budget year from the player's main post | K6, R1 | C5, E1 | yes |
| **E3** | Dilemma pool: 26 templates (4 campaign-only), themes on every dilemma, written in U1's condition type | E2, U1 | C5, E4, E5, E6 | no |
| **E4** | Storylines and NPC schemes (`npc_did` with an `at` selector, `NpcDeed.at`, 6 storylines, start draw, the next-move view; broker scheme after P1) | E2, U1 | E3, E5 | yes (`npc_deeds`) |
| **E5** | The press covers work (`StorySource::Work`) | E2 | E3, E4 | no |
| **E6** | Asks engine (domain, sliding rate, deed log, commands, casework, effects; askers and scopes by constituency and `Area`) | E1, C1, U1 | E3, E7 drafting, R2 | yes |
| **E7** | Asks protocol and Today tab | E6, C2 | E8, E13, R2 | no |
| **E8** | Asks content, promises, request conversions, bots; the ladder enforced | E6, U1 | E7, E13, R2 | no |
| **E9** | Secrets: registry, seeding, leads, dig, tip, talk hooks; `HiddenFact.at`, typed `HiddenId`; the `fact_kinds` registry | E6, K3, C1, E5 | G2, E10 | yes |
| **E10** | Walking rewards engine: curiosities, leads, live weighting, Inspect, LookAround, encounters, lore notebook and topics | G2, U1 | E9, E11 | yes |
| **E11** | Walking rewards web: slots, dress, props, inspect, Look around, lore page, placement check | E10, B3 | E9 | no |
| **E12** | Engagement lab command, gates and tables in CI and nightly; ladder tuning | E2, E8 (E13 for the conversation columns) | L1 | no |
| **E13** | Conversation memory: priorities, once-only lines, callbacks | C1, C2, C4 (done), E6 | C5, E7, E8 | yes |

### E1 — Ground-game curve, record and informed styles

- **Goal:** canvassing builds awareness first, with fast per-neighbourhood diminishing and a campaign-season boost (§2.1); record from delivery (§2.2); the informed styles `explorer_coalition`, `explorer_legislator`, `explorer_campaigner` (§2.4); the bot gates renamed and grouped as the ladder (§7.2), enforced as its "Enforced" column says for E1, the rest reported.
- **OWNS:** `crates/mandate-sim/src/actions.rs` (the `Canvass` arm and its outcome args only); `crates/mandate-sim/src/player/{mod.rs,ground.rs (new)}` (`PlayerState.ground`: per-neighbourhood visits and dates); `crates/mandate-content/src/types.rs` (canvass action params and `record_gain` fields); `content/base/{actions,career}.jsonc`; `content/schemas/**`; `crates/mandate-save/**` (migration); `crates/mandate-cli/src/bots.rs` (the strategy classes, the three informed variants, an `asks_only` stub, the ladder gates: `idle_cap`, `effort_matters`, `exploring_alone_never_wins`, `corrupt_not_dominant`, `good_styles_vary` clause (b) and `ladder_learned` at + 1.0 enforced; `ladder_rote`, `knowledge_matters`, `skill_gap` and the rest of `good_styles_vary` reported); `content/base/locale/en/base.ftl` (canvass freshness lines); `docs/development/BALANCE_LOG.md`.
- **Depends on:** K4 (done). Runs beside R1 (DEC-098): E1 stays inside the `Canvass` arm; both add to `player/mod.rs`, `bots.rs` and the save migrations, each additively, and the second to merge rebases. New reads of the player's town go through one helper that R1's `player::home(state)` replaces.
- **Acceptance:** unit tests of the curve (same neighbourhood every turn → fresh < 0.1; rotation of three → 0.4–0.6; campaign multiplier applies only while a candidate); `bots --seeds 16` and `--seeds 32` pass every gate enforced from E1; the reported rungs show `canvass` no longer has the highest win rate and mean share; the three informed variants each differ from `explorer` in action mix (reported); grid evidence in BALANCE_LOG; determinism (2026 × 80) twice and through save/load; native = WASM (777 × 60).
- **Save:** bump (`PlayerState.ground`, default empty).

### E2 — The director: variety, tiers, themes and the pacing cycle

- **Goal:** cooldown floor and growth, no repeat in a chapter (climax included), climax memory (§4.1); tiers, chapter themes and `pick` (§4.2); the pacing cycle with profiles, the climax window, the coda, the quiet rescue and easing after a setback (§4.3); the `answered` and `sponsored_law` conditions (§4.4; U1 carries them into the one condition type); `min_career_days` on dilemmas and storylines (§4.7); a session chapter's budget year is that of the jurisdiction of the player's main post (`player::posts(state).first()`, §4.3; DEC-098), with `crisis_cap` per town; `story --metrics` prints the §7.1 decision and pacing metrics; each week's candidate set exposed to the lab as a pure function (§7.4).
- **OWNS:** `crates/mandate-sim/src/narrative/{director.rs,mod.rs,cast.rs}`; `crates/mandate-content/src/narrative.rs` (`DirectorParams`, storyteller profile fields, `Condition::Answered`, `Condition::SponsoredLaw`, `themes` and `min_career_days` on templates, validation); `content/base/{director,storytellers}.jsonc`; `content/schemas/**`; `crates/mandate-save/**`; `crates/mandate-cli/src/main.rs` (the `Story` arm only).
- **Depends on:** K6 (done); R1 (DEC-098: posts and `state.focus()` exist, and `narrative/cast.rs` is out of R1's freeze).
- **Acceptance:**
  - 12 seeds × 5 years, mixed, Classic: most uses of a template ≤ 3; no repeat within a chapter; no climax template in consecutive chapters; no theme in consecutive session chapters while another was eligible; 3–6 peaks per session chapter; longest governing decisions-only gap median ≤ 120 days, every seed ≤ 145 (the no-beat gates need E5 and E6).
  - Unit tests: tier A is drawn before B and C; the climax peak is forced 14 days before the window closes; a setback lengthens the next relax by 1.5; relax ends on an introduced bill; a `raised_by` decision opens during a relax; Unfiltered has no phases.
  - The director on/off test (endogenous outcomes identical) still passes; the draw count is identical with `--omniscient`; CI sweep decisions per chapter stay within budgets (0 over).
- **Save:** bump (`NarrativeState.uses`, `climaxes`, `answers`, `phase`, `setback`; `Chapter.theme`, `peaks`).

### E3 — Dilemma pool

- **Goal:** the 26 dilemmas of §4.4 with scenes and Fluent text, including the 4 campaign-only ones; `themes` on every dilemma, the existing 31 included (§4.2); `min_career_days` values (§4.7). The seven that need G3, E9, E10, V2 or P1 are written now and enabled by their triggers. Triggers use U1's one condition type; cast roles that name an office use constitution ids (`Cast::Office`, `{ "type": "office", "posts": [...] }`), never a fixed office word (DEC-098).
- **OWNS:** `content/base/{dilemmas,scenes}.jsonc`; `content/base/locale/en/narrative.ftl`.
- **Depends on:** E2; U1 (DEC-098: one condition language before the content volume lands).
- **Acceptance:** `content validate` (every option has a cost and a benefit; Fluent arguments; every dilemma has a theme); at least 4 dilemmas per session theme and at least 6 campaign-only templates or stages without V2; 12 seeds × 5 years: distinct templates median ≥ 18, every seed ≥ 14; each new template opens in at least one of 32 seeds or is listed as gated on a later task.
- **Save:** none.

### E4 — Storylines and NPC schemes

- **Goal:** `npc_did` condition (a variant of U1's type, with an optional `at` selector, default `same`) and the NPC deeds log (`NpcDeed.at: Jurisdiction`, recorded for `state.focus()` only; DEC-098); `storylines_active` 3, spacing, minimum, and starts drawn among the eligible arcs (§4.2); the six storylines of §4.5 with their `min_career_days` (`broker_scheme` content lands with P1); the next-move view once the schemer's plans are known (§4.5).
- **OWNS:** `crates/mandate-sim/src/npc.rs` (appending deeds only); `crates/mandate-sim/src/politics/mod.rs` (`npc_deeds` field); `crates/mandate-sim/src/narrative/director.rs` (`start_runs` only, after E2 merges); `crates/mandate-content/src/condition.rs` (`Condition::NpcDid`, after U1); `crates/mandate-sim/src/condition.rs` (its evaluation arm); `content/base/{storylines,director}.jsonc`; `content/base/locale/en/narrative.ftl` (storyline keys); `crates/mandate-protocol/src/narrative.rs` (`StorylineView.next`); `web/src/engine/protocol/**` (generated); the web story sheet's storyline card only; `crates/mandate-save/**`.
- **Depends on:** E2; U1 (it adds a condition); R1 (through E2).
- **Acceptance:** 12 seeds × 5 years: storyline templates median ≥ 3, every seed ≥ 2; a scheme stage outside a campaign in ≥ 75% of careers; in towns where two arcs can start, each starts first in some seeds; schemes advance identically with and without the player knowing (K6-style test); storyline views gated by the ledger (K2 fallbacks); with `RivalPlans` at `known` the rival scheme shows its next move, below it a lead (protocol test).
- **Save:** bump (`npc_deeds`, default empty).

### E5 — The press covers the player's work

- **Goal:** `StorySource::Work` (§4.6) for every politician; bill passed or failed replaces subject-less town news for bills; appearances; promise stories hook for E6. Keyed by politician, so no tier change (DEC-098).
- **OWNS:** `crates/mandate-sim/src/media.rs`; `crates/mandate-content/src/types.rs` (media section); `content/base/media.jsonc`; `content/schemas/**`; `crates/mandate-protocol/src/media.rs` (article view for the new source); `content/base/locale/en/media.ftl`.
- **Depends on:** E2 (metrics).
- **Acceptance:** 12 seeds × 5 years mixed: press about the player median ≥ 6, every seed ≥ 2, at least half on work; idle ≥ 1; the perception twin test (DEC-034) passes for work stories; bot gates pass.
- **Save:** none (`StorySource` gains a variant; the stories list is already in saves and old saves have none of it).

### E6 — Asks engine

- **Goal:** the `asks` domain (§3.6): state, weekly system for `state.focus()` with the sliding rate and asker callbacks (§3.2), `Asker::Constituency`, `Effect::Approval { scope: Area }`, colleagues cast by post (R1's `Cast::Colleague`), promise deadlines from the post being run for and promises counted per race (`promises_race: Option<Post>`; §3.5; DEC-098), ask triggers in U1's condition type, `PlayerDeed` events and the deed log, `RespondAsk`, `Promise`, `Action::Casework`, `big` asks ending the director's relax, effects, validation, `Stream::Asks`; `min_career_days` on asks; the second-term rules (lower-half deadlines after re-election, no persuasion for a promise kept last term; §4.7); one sample ask and one promise.
- **OWNS:** `crates/mandate-sim/src/asks/**` (new); `crates/mandate-sim/src/events.rs` (`PlayerDeed`); `crates/mandate-sim/src/actions.rs` (deed appends, `RespondAsk`, `Promise`, `Casework` arms); `crates/mandate-sim/src/{systems.rs,scheduler.rs}` (registration; the new `Effect` variants `Opinion`, `Favour`, `Approval`, `Record`); the arms that apply them in `crates/mandate-sim/src/{characters,population,politics}/**` (effect application only); `crates/mandate-core/src/rng.rs` (`Stream::Asks`); `crates/mandate-content/src/asks.rs` (new) and its `ContentFile`/bundle lines; `crates/mandate-content/src/validate.rs` (asks section); `content/base/asks/{params,asks,promises}.jsonc` (samples); `content/schemas/**`; `crates/mandate-save/**`.
- **Depends on:** E1, C1 (done); U1, and through it R1 and E2 (DEC-098 order).
- **Acceptance:** unit tests for each `Need` (kept, broken, void); property test: an ask resolves exactly once; unit tests of the curves (no offer the week after one; none at 4 open; twice the base chance with nothing open and 21 quiet days); the deed log stays bounded and ordered; asks never depend on `knows` (validation test); no `MunicipalityId` key and no office word in the asks state or its content (a test); determinism, save/load/continue parity, native = WASM; the blind-invariance test still passes.
- **Save:** bump (`State.asks`, including the deed log).

### E7 — Asks protocol and the Today tab

- **Goal:** `AskView` (with `AskerView::Constituency { name_key, number }` for a constituency's voters; DEC-098), `PocketView.asks`, `asks_due_soon`, `QueryDto::Asks`, `QueryDto::PromiseMenu`, TS types; the Today tab block, the story sheet's promises, the promise picker while campaigning.
- **OWNS:** `crates/mandate-protocol/src/{asks.rs (new),places.rs (PocketView fields),lib.rs}`; `web/src/engine/protocol/**` (generated); `web/src/**` except `engine/protocol` (Today tab, promise picker, story sheet).
- **Depends on:** E6, C2 (done). Runs beside R2: both edit `crates/mandate-protocol/src/{lib,places}.rs` and `web/src` (different items); the second to merge rebases.
- **Acceptance:** Rust ⇄ TS round-trip; accept, decline and keep an ask in Playwright (phone and desktop); keyboard and screen reader labels; the badge counts offers and promises due within a turn.
- **Save:** none.

### E8 — Asks content, promises, conversions, bots and the ladder

- **Goal:** the first pack (§3.8) with Fluent text, `big` flags and `min_career_days`, written in U1's condition type, with colleagues and office holders cast by post (`ask.vote_with_me`, `ask.second_my_bill`: "a holder of a post in the same body", never the word councillor; DEC-098); `request.ally_vote`, `request.traders_meeting`, `request.back_the_workers` become asks (goals point at them); `mixed` and the informed styles accept and fulfil asks (§3.9); `asks_only`; the base offer rate tuned with the curves; the gate ladder enforced as its "Enforced" column says for E8 (`ladder_rote`, `ladder_learned` at + 3.0, `knowledge_matters`, `skill_gap`, all of `good_styles_vary`); final ground-game tuning with E1's grid.
- **OWNS:** `content/base/asks/**`; `content/base/{goals,dilemmas}.jsonc` (the three conversions only); `content/base/locale/en/asks.ftl` (new); `crates/mandate-cli/src/bots.rs`; `content/base/{actions,career}.jsonc` (tuning values only); `docs/development/BALANCE_LOG.md`.
- **Depends on:** E6 (drafting can start after E6's formats); U1 (DEC-098: one condition language before the content volume lands).
- **Acceptance:** 12 seeds × 5 years: asks offered median ≥ 15, every seed ≥ 8; mixed keeps ≥ 60% of accepted; broken in ≥ 25% of careers; canvass share of approval ≤ 40%; `bots --seeds 16` and `32`: every gate enforced from E8 passes.
- **Save:** none.

### E9 — Secrets

- **Goal:** the secret kinds registry with leads and seeding (§5.2), `HiddenFact.secret` and `cast`; in the same bump `HiddenFact.town` becomes `at: Option<Jurisdiction>` and hidden facts take a typed `HiddenId` (§5.5); `max_live_per_jurisdiction` and seeding keyed by `[month, kind, at.tier_index(), at.raw(), cast]`; secret kinds cast by post, never by an office word (DEC-098); the `fact_kinds` registry that shady acts and secret kinds reference (§5.5; DEC-098 §4); `Effect::Hide`, `speaker_knows` rules, `AboutKind::Secret` (finds become talk, §5.3), `Investigate` and `TipPress` actions (and their deeds), notebook entries and leads, the `secret_hunter` probe and `secrets_not_decisive`.
- **OWNS:** `crates/mandate-sim/src/narrative/{mod.rs (HiddenFact fields, HiddenId),secrets.rs (new)}`; `crates/mandate-sim/src/media.rs` (fact-kind lookups only, after E5 merges); `crates/mandate-content/src/{fact_kinds.rs (new),types.rs (media section: references to fact kinds)}`; `content/base/fact_kinds.jsonc` (new); `content/base/media.jsonc` (shady and NPC acts reference fact kinds); `crates/mandate-sim/src/actions.rs` (`Investigate`, `TipPress` arms); `crates/mandate-sim/src/characters/knows.rs` (secret rules); `crates/mandate-sim/src/talk/context.rs` (the secret `about` only); `crates/mandate-content/src/{secrets.rs (new),conversation.rs (KnownSubject::Secret, AboutKind::Secret)}`; `content/base/secrets.jsonc` (new); `content/base/conversations/lines.jsonc` (secret substance lines); `content/base/locale/en/{secrets.ftl (new),talk.ftl}`; `content/schemas/**`; `crates/mandate-save/**`; `crates/mandate-cli/src/bots.rs` (`secret_hunter`).
- **Depends on:** E6 (`ask.keep_it_quiet`, the deed log), K3, C1 (done); E5 (it owns `media.rs` first); P1 for the broker kinds (they seed nothing until P1).
- **Acceptance:** secrets seed with and without the player knowing (identical worlds except the ledger, K6 test); journalists and the rival discover them by the existing rules; dig and tip odds shown = rolled (L1 hook); `secrets_not_decisive` passes; a held secret is a `topic.about` option with the people who know it; validation: no `knows` in seeds, every kind has a lead, every shady act and secret kind names a registered fact kind; a fixture from the previous format migrates `HiddenFact.town` to `at`; media outcomes are unchanged at fixed seeds apart from the new secrets.
- **Save:** bump (`HiddenFact` fields, `town` → `at`, typed `HiddenId`).

### E10 — Walking rewards, engine

- **Goal:** the curiosity registry and validation (§6.1: the accessibility rule and leads; `eligible` and `when` are variants of U1's condition type, not a fourth language), seeding at play start with the live-trouble weighting (§6.2; the seed function takes the town as a `MunicipalityId` argument, and `Curiosity.town` stays; DEC-098), `Inspect`, `LookAround`, encounters with the first-year familiarity weighting (§6.5), `Subject::Curiosity`, `AboutKind::Curiosity` and its `speaker_knows` rule, the `lore` notebook section, `TownStateView.curiosities`, the `walker` probe and `walking_is_a_bonus`.
- **OWNS:** `crates/mandate-sim/src/{explore.rs (new),places.rs (street-arrival hook)}`; `crates/mandate-sim/src/world.rs` (`curiosities` field); `crates/mandate-sim/src/player/knowledge.rs` (`Subject::Curiosity`); `crates/mandate-sim/src/actions.rs` (`Inspect`, `LookAround` arms); `crates/mandate-content/src/{curiosity.rs (new),conversation.rs (AboutKind::Curiosity),condition.rs (the `history` variant)}`; `crates/mandate-sim/src/condition.rs` (its evaluation arm); `content/base/curiosities.jsonc` (new, 14 kinds); `content/base/locale/en/curiosities.ftl` (new, with `PlayerKnows` variants); `crates/mandate-protocol/src/{town.rs (TownStateView field),knowledge.rs (lore section)}`; `web/src/engine/protocol/**` (generated); `content/schemas/**`; `crates/mandate-save/**`; `crates/mandate-cli/src/bots.rs` (`walker`).
- **Depends on:** G2 (shares the street-arrival discovery hook and `Subject::Place`); U1 (its conditions).
- **Acceptance:** every town of 16 seeds has 12–30 curiosities, ≥ 4 kinds, none of them `streets` with no reward or `walk` with one, and every `streets` kind with a lead (validation); in a flood-prone town of a fixed seed, flood facts are over-represented in lore arguments against a run with `live_weight` 1.0; `Inspect` and `LookAround` give the same rewards; encounters fire on arrivals from any mode, and the draw count does not depend on the talk log; `walking_is_a_bonus` and `exploring_alone_never_wins` pass; determinism and native = WASM.
- **Save:** bump (`world.curiosities`).

### E11 — Walking rewards, web

- **Goal:** curiosity slots in plan stage 11, dress assignment by rendezvous hash, placeholder props and art keys, tap to inspect, the encounter card, "Look around" in the Places tab, the notebook's lore page, the placement check (§6.3).
- **OWNS:** `web/src/hub/layoutV2/{plan.ts,dress.ts,check.ts}` and their tests and snapshots; `web/src/hub/townLayout.ts` (`PropKind`, `TownMap.curiositySlots`); `web/src/hub/render/**` (placeholder props); `web/src/**` pocket and notebook components; the placement-check script under `web/tests/**`; `tools/pixelkit` art keys list only (P1 draws the art).
- **Depends on:** E10, B3 (done).
- **Acceptance:** slots never block a door or route (§13 checks extended); the dress never moves a placed marker as curiosities are found (test); pinned v2 hashes refreshed; Playwright: walk to a marker and inspect it; Look around from the Places tab finds the same `streets` curiosity; screen reader labels; plan timings within 5% of before; the placement report lists districts where more than 30% of slots are never passed (report only).
- **Save:** none.

### E12 — The engagement gate

- **Goal:** `mandate-cli engagement` (§7.1), the career gates of §7.2, `--tables` and its checks (§7.4), CI and nightly wiring (§7.3), the approval-source split, the conversation columns (§4.8), the `knowledge_matters` pass in `bots`, and the first tuning of the ladder's thresholds from runs after E1 and E8; TEST_STRATEGY and BALANCE_LOG entries.
- **OWNS:** `crates/mandate-cli/src/{engagement.rs (new),main.rs}`; `crates/mandate-cli/src/bots.rs` (shared career loop and the omniscient pass only); `.github/workflows/{ci,nightly}.yml`; `docs/development/{TEST_STRATEGY.md,BALANCE_LOG.md}`.
- **Depends on:** E2 and E8 (metrics exist); E13 for the conversation columns (they read "n/a" until it merges); runs beside L1 (both add CI steps; the lead merges the workflow edits).
- **Acceptance:** CI run ≤ 60 s; gates report-only until E2–E8 merge (§9, question 6), then enforced; a seeded failure (a content override that disables cooldowns) fails the gate; another (a theme weight forcing `crisis`) fails the theme gate; `--tables` lists every template, verb and ask template; output is stable across runs (deterministic rows).
- **Save:** none.

### E13 — Conversation memory

- **Goal:** lines that remember (§4.8): `TalkLineDef.priority`, `once` and `after`; tiered selection with the repetition guard across tiers; the `PlayerDid` and `AskOutcome` conditions (variants of U1's one type) and `$count` on `Remembers`; bounded `TalkState.heard_once`; priorities on C3's lines and about 20–30 new reactive and story lines; validation; the `once_violations`, `reactive_share` and `line_repeat_rate` metrics. Hades' exact rules are unverified inference (research §2.3): the targets are starting points.
- **OWNS:** `crates/mandate-content/src/conversation.rs` (`TalkLineDef` fields, `TalkParams.heard_once_cap`); `crates/mandate-content/src/condition.rs` (the two new variants, `$count`); `crates/mandate-sim/src/condition.rs` (their evaluation arms); `crates/mandate-content/src/validate.rs` (conversation section); `crates/mandate-sim/src/talk/{reply.rs,context.rs,state.rs}`; `content/base/conversations/{lines,params}.jsonc`; `content/base/locale/en/talk.ftl`; `content/schemas/**`; `crates/mandate-save/**`; `crates/mandate-cli/src/talk.rs` (the metric columns only, after C5 merges).
- **Depends on:** C1, C2 and C4 (done: the panel shows the new lines with no protocol change); E6 (the deed log and ask outcomes; through it, U1).
- **Runs beside:** C5 (its CLI columns land after C5), E7, E8.
- **Acceptance:** unit tests: the highest priority with a candidate wins; `once` lines never repeat in their scope; `after` gating; the repetition guard still applies across priorities (C5's repetition violations stay 0); each `PlayerDid` scope (a vote on the speaker's bill, a canvass in the speaker's home neighbourhood, a deed here); `AskOutcome` accepted, kept and broken; `$count`; `heard_once` stays sorted and bounded. `talk --coverage --seeds 16`: `once_violations` 0, `reactive_share` and `line_repeat_rate` reported (E12 checks `reactive_share` ≥ 25% over 5-year careers). The C3 render test renders every new line in four towns. Determinism, save/load/continue parity, native = WASM.
- **Save:** bump (`TalkState.heard_once`, default empty; the lead may fold it into a neighbouring bump).
