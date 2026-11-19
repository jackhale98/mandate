# Engagement references (DEC-097): what other games teach

> **Folded in (owner, 2026-11-19: "Yes do it"):** §3's proposals are accepted and now live in `ENGAGEMENT.md` (§3–§7, tasks E1–E13), the document to implement from; this file keeps the sources and reasoning.

**Status:** Research for DEC-097 and `ENGAGEMENT.md` (2026-11-19). Nothing here changes an accepted decision. Changes to `ENGAGEMENT.md` are proposals for the owner (§3).
**Read with:** `ENGAGEMENT.md` (E1–E12), DEC-033, DEC-077, DEC-091, DEC-093, DEC-097; `M6_ENGINE_CONTRACTS.md` §3 (conversations).
**Method:** One web research pass. Primary sources were used where they could be reached: designer slides, designer blogs, developer diaries, and talk write-ups. Each claim is tagged:
- **[S]**: read in the cited source this session;
- **[C]**: community-maintained (a wiki, modders, forum players), not the developer;
- ***Inference***: this document's own reasoning, or something known from playing the game that could not be checked against a source this session.

Source keys in brackets (`[L4D]`) refer to §5.

---

## 1. In short: the most applicable lessons

| # | Lesson | From | Lands on |
|---|---|---|---|
| 1 | Pace with a **cycle**, not one rise per chapter: build up, peak, a short enforced relax, then build up again. Change the *frequency* of beats, never their *amplitude* | Left 4 Dead's AI Director [L4D]; RimWorld's Cassandra on/off cycle [RW-C] | E2 director (§3.1) |
| 2 | A relax phase has a **minimum and a maximum** length, and it waits for a natural break before it starts | Left 4 Dead [L4D] | E2 (§3.1) |
| 3 | Draw events from a **filtered, weighted bag** that is rebuilt every turn. Recently drawn cards leave the bag; a live theme (a war) makes its cards bigger | Reigns [REIGNS] | E2 `pick` (§3.2) |
| 4 | **Story-carrying content outranks filler.** Lines that move the story or react to what the player did take priority over generic ones; generic ones only fill gaps | Hades [HADES-GD], [C] [HADES-MOD] | Conversations C2–C5, E6 hooks (§3.4) |
| 5 | Characters who **remember and keep score** make repeats feel new ("who won this time, who won last time") | Hades (Kasavin) [HADES-RL] | Conversations, asks (§3.4) |
| 6 | Generation **slows itself as things pile up**: RimWorld cuts new-colonist events as the colony grows | RimWorld population intent [RW] [C] | E6/E8 asks (§3.3) |
| 7 | A **dominant strategy removes choice**. Test: "does the exploit drown out all other play styles?" Fix routines that dominate or grind; keep ones that are fun alternatives | Soren Johnson [WATER] | E1, E8, E12 gates (§3.5) |
| 8 | Good decisions are **situational**: the best option depends on the world at that moment. If players always pick the same option, it is not a decision | Sid Meier [MEIER] | E1 canvass, asks, promises (§3.5) |
| 9 | Late games go stale when **everything gets solved**; voters who judge levels, not trends, keep re-electing a competent incumbent | Democracy 4 (Cliff Harris) [D4-LONG] | Re-election, E2 consequences (§3.6) |
| 10 | Balance from **run data**: pick rate and win rate per card, filtered by question. Data shows *what*, not *how it feels* | Slay the Spire [STS]; Democracy 4 event trigger tables [D4-EVENTS] | E12 telemetry (§3.7) |
| 11 | **Knowledge is the progression** when nothing else is. Clues point to further clues; the world runs on its own clock whether you look or not | Outer Wilds [OW-GD] | Ledger, E9–E11 (§3.8) |
| 12 | **Familiarity comes before curiosity.** A mystery only intrigues once the player knows the place it disturbs | Outer Wilds [OW-GD] | E10 curiosities (§3.8) |
| 13 | Place finds where the eye is **already drawn** (landmarks, partly hidden spots), and check the spread with playtest heat maps. A find must be worth the detour: "simply placing rocks somewhere isn't intriguing" | Breath of the Wild [BOTW-CEDEC], [BOTW-KOROK] | E10/E11 placement (§3.8) |
| 14 | Procedural history works as lore when the system **picks the event, then explains it** from the state of the actors | Caves of Qud [QUD] | E10 lore arguments (§3.8) |
| 15 | Rules that **change from day to day** keep a fixed daily routine under pressure, and can steer the story | Papers, Please (Lucas Pope) [PP-GI] | Career escalation (§3.9) |

---

## 2. Techniques by area

### 2.1 Storytellers and event pacing

**Left 4 Dead: the AI Director's "adaptive dramatic pacing"** (Michael Booth, Valve, 2009) [L4D] [S]
- Booth's starting point was Counter-Strike's pacing: "Constant, unchanging combat is fatiguing. Long periods of inactivity are boring. Unpredictable peaks and valleys of intensity create a powerfully compelling and replayable experience."
- He keeps one number per survivor, "emotional intensity". It rises with damage taken, being incapacitated, being pulled off a ledge, and nearby kills. It decays toward zero over time, but not while enemies are actively engaging the survivor. The Director tracks the highest of the four.
- Four phases drive the enemy population:
  - **Build Up:** full threat population until intensity crosses the peak threshold.
  - **Sustain Peak:** hold the full population for 3–5 seconds after the peak.
  - **Peak Fade:** switch to the minimal population, but do not start the relax clock "until a natural break in the action occurs".
  - **Relax:** keep the minimal population for 30–45 seconds, "or until Survivors have traveled far enough toward the next safe room", then build up again.
- Relax means no wanderers, no mobs and no special infected. Mobs otherwise come at randomised 90–180-second intervals.
- The key rule: "**Amplitude (difficulty) is not changed, frequency (pacing) is.**" And: "Survivor Intensity estimation is crude, yet the resulting pacing works."
- **For Mandate:** `director.jsonc` already measures live tension (crisis, pending decisions, race, danger), which is Booth's intensity. It lacks his phase machine. Today a session has one rise to one climax, then nothing until January, which is the 168-day gap in ENGAGEMENT §1.2. Booth's rule (change frequency, not amplitude) is exactly DEC-033 B: the director shapes timing and spotlighting, never consequences. See §3.1.

**RimWorld: Cassandra, Phoebe, Randy** (Tynan Sylvester, Ludeon; his GDC 2017 talk frames the game as a story generator [RW-GDC]) [RW], [RW-C] [C]
- The storyteller weighs "colony wealth, building wealth, colonists count, animal count, if a colonist has died or been severely wounded recently, and how long it has been since the last major event" [RW].
- **Cassandra** runs major threats on a fixed on/off cycle [RW-C]:
  - the first "on" phase starts on day 11;
  - "on" phases last 4.6 days and "off" phases 6.0 days;
  - threats in one phase are at least 1.9 days apart, and each phase brings 1 or 2;
  - that averages about 8.5 major threats a year;
  - minor events come at a mean time between events of 4.8 days.
- **Phoebe** runs 8 days on, 8 days off, one threat per cycle. **Randy** has no cycle; he just makes major threats less likely per day, and the wiki warns he "will sometimes send extremely difficult or unfair groups of events."
- **Adaptation** "grows over time, and is knocked down when you take damage" [RW]. After losses, the storyteller eases off.
- **Population intent** [RW] [C]: the chance of events that add colonists is multiplied by a curve over the current population (0 colonists ×8.0, 1 ×2.0, 5 ×1.0, 9 ×0.4, 12 ×0.0, 20 ×−1.0) and by a time curve (×0 at day 0, rising to ×1 by day 10). Generation throttles itself as the colony fills.
- **Sylvester on story** [RPS-RW] [S]: apophenia is "players extract more story meaning from a game event than is really represented in the mechanics"; "just two lines of code choosing a random thought creates a reason, a connection"; and "You have to leave something undefined for the player to create the story." Colonies are capped near 12 people so players know each one.
- **For Mandate:**
  - Cassandra's on/off cycle with a minimum spacing is a calendar form of Booth's machine, which suits a turn-based game (§3.1).
  - Adaptation maps to easing off after an election loss or a scandal (§3.1).
  - Population intent maps to ask generation slowing as open asks pile up (§3.3).
  - The ~12-person cap supports keeping asks, encounters and schemes on a small recurring cast (the broker, the rival, a few Named people), so memories and callbacks land (§3.4).

**Crusader Kings II/III: event pools, mean time to happen, story cycles** (Paradox)
- **CK II** events fire on a "mean time to happen", which `modifier` blocks scale by a `factor`. A factor of 0.01 makes an event far likelier for the right character; 10.0 makes it rarer for the wrong one [CK2-MTTH] [C]. Relevance changes frequency instead of switching events on and off.
- **CK III story cycles** are persistent objects with `on_setup`, `on_end` and `on_owner_death`, and `effect_group`s that fire on randomised intervals. The base game's peasant affair uses `effect_group = { days = { 300 400 } chance = 5 … }` [CK3-SC] [C]. A slow story runs in the background for years and surfaces only now and then.
- **Dev diary #50:** the yearly random event pools were restructured and many triggers loosened, so a player sees roughly one random event from the pool every 4 years instead of every 5–6 [CK3-DD50] [C: a forum repost].
- **Dev diary #75 (court events):** each Hold Court decision presents "a random selection of three events" from a pool of nearly 100. A reply in the thread warns that without restraint the feature "would basically just become a pretty looking overlay for a barrage of inconsequential random events" [CK3-DD75] [C].
- **For Mandate:**
  - NPC schemes (E4) are story cycles: they persist, wake on intervals, and advance on NPC deeds.
  - Over-gated pools give the "same five templates" problem (ENGAGEMENT §1.1: 21 of 31 templates ever appear). Paradox's fix was to *loosen triggers* so more of the pool is eligible. E3 should check each new template's eligibility rate, not only its existence (§3.7).

**Stellaris: event chains and archaeology** (Paradox) [STEL-145], [STEL-146] [C]
- Archaeological sites have an introduction plus 1–6 chapters, each with its own text and reward. A random event may fire at the end of any excavation phase. Sites are found on survey, at game start, or through events.
- Precursor chains were once fixed by the player's starting location, so a given spawn point always led to the same chain. Paradox changed it so the chain is unknown "until the first event fires" (a developer reply in the thread [STEL-146]).
- **For Mandate:** a long chain of chapters with rewards along the way is the secret and investigation shape (E9, `storyline.investigation`). The precursor fix is a warning against sameness: if a scenario or defining industry maps 1:1 to one storyline, every run of that kind will tell the same story. Storyline starts should draw among the eligible arcs, not take a fixed one (§3.2).

### 2.2 Political and narrative games

**Reigns** (François Alliot, Nerial; GDC 2017 talk [REIGNS-GDC]) [REIGNS] [S]
- The deck is a bag holding all the game's cards. Before each draw, cards whose requirements fail are removed ("if a range of cards are related to the presence of a queen but you're not married yet, I remove those"). Each remaining card gets a size, and "the larger the card, the more space it will occupy in the bag".
- During a war, the war's roughly 10 cards are large and the jester's cards small. When the war ends, the war cards leave the bag.
- Cards played too recently are removed. "The bag of available cards is recompiled every time the player swipes."
- Some events, such as the dungeon or duels, "lock the game in a smaller sub-system", even down to a single card for a linear path.
- **For Mandate:** this is the `pick` of ENGAGEMENT §4.2, plus two things it lacks: a **chapter theme** that enlarges the cards of the live situation, and **removal of recent cards** as a hard rule (E2's `no_repeat_in_chapter` already does this). See §3.2.

**Papers, Please** (Lucas Pope) [PP-GI] [S]
- Pope on the bureaucratic premise: "rules can change from day to day arbitrarily to ratchet up the pressure or direct the story." The core loop is fixed (check papers, stamp), but each day changes what a correct stamp is.
- Critics split on the monotony. A *Press Start* review calls it "repetitive and monotonous, but then again so is the work of an immigration officer in a totalitarian state"; some players could only play in short bursts [PP-REV] [S, via search excerpts].
- **For Mandate:** keep the verbs stable and change the conditions. Mandate's version is town state (a new crisis, a council that turns hostile, a budget squeeze) and new pressures introduced over the career (§3.9). Do not copy deliberate monotony (§4).

**Frostpunk: the Book of Laws** (11 bit studios) [FP-PCG], [FP-GP], [FP-WIKI]
- The book sequences laws so the game can explore "creeping normality or the boiling frog syndrome": each step is small, and the sum is dark [S, via search excerpts].
- Laws are irreversible, "however, future laws can amend what you've decided in the past" [S].
- Darker options appear as conditions worsen (sawdust in the food) [S]. Each law has its own cooldown, such as 36 hours [C].
- **For Mandate:** ENGAGEMENT's consequence templates (`answered`, §4.4) are the boiling frog: a small answer comes back months later as a harder question. Campaign promises are Frostpunk's irreversibility, made public. Already well matched.

**Democracy 4** (Cliff Harris, Positech) [D4-LONG], [D4-EVENTS] [S]
- Harris found long games went stale: "everything gets solved as things go on, and things are too easy in the late game". Wins at 72–75% of the vote were "still not 'fun'".
- He traced it partly to voters judging *levels* ("Healthcare is at 60% so I am 60% happy") rather than *changes*. His contrast: "Healthcare has fallen 2%. Things are getting worse. I'm voting for the opposition."
- For events, he builds tables per game version of how often each event triggers, then adjusts inputs so some events do not fire far more than others [D4-EVENTS, opening only].
- **For Mandate:**
  - Mandate's retrospection already reacts to *change*: unemployment rise beyond the national rise, real rent growth (`opinion.jsonc`). Keep it that way.
  - The player's approval is pulled toward reactions to their bills at 10% a month (`player_approval_rate`). Bonuses from asks therefore fade with a half-life of about 6.6 months (*inference*, from 0.9^6.6 ≈ 0.5). That is a healthy "what have you done for me lately".
  - The risk Harris names, solved towns and coasting incumbents, applies to Mandate's second term (§3.6). His trigger tables are a model for E12's template table (§3.7).

**The Political Machine** (Stardock): a warning [PM-GA] [S]
- Each turn, "you travel from state to state, using your energy to give speeches, run ads, hold fundraisers". Gaming Age (2024) calls the campaigning "repetitive and kind of mindless", "clicking the same buttons over and over".
- Reviews across the series repeat the complaint [PM-SP] [S]. An older review credits the 41-week, four-quarter structure with keeping it varied [PM-HG] [S].
- **For Mandate:** this is the canvass-only bot as a commercial game. Campaign season, where ENGAGEMENT §2.1 makes canvassing strongest, is where this risk is highest. The campaign gate (quiet ≤ 21 days) is right; §3.9 adds campaign-specific beats.

**Tropico 6** (Limbic): a warning for asks [TROP] [S]
- Reviewers name faction demands as the weak point: they can leave you "running in circles just to keep up with everyone's demands", and pleasing critics is "an infuriating guessing game". Edicts are praised as the interesting part.
- **For Mandate:** asks must not become Tropico's demand treadmill. ENGAGEMENT's caps (4 open, 1.5 a month, cheap declines, a visible live reason for each) are the right defence. §3.3 adds a self-throttling curve.

**Suzerain** (Torpor) and **Not For Broadcast** (NotGames) [SUZ], [NFB] [S]
- Suzerain's reviewers praise "how responsive the narrative is to the player" and fault sections that "unnecessarily go on too long".
- Not For Broadcast's chapters are staged as introductory, escalation, and turmoil and censorship. One review finds the evenings between broadcasts, when "you get to go home", the forgettable part.
- **For Mandate:** players forgive a long political game when it reacts to them. The lull between set pieces is where interest drops: that is the 168-day gap again, and the downtime is where asks, press and encounters have to carry the day (§3.1).

### 2.3 Run variety in roguelikes

**Hades: reactive dialogue across repeated runs** (Greg Kasavin, Supergiant)
- The game "peeks at the moments players encounter mid run, like encountering a character while Zagreus' health is below a certain threshold, and feeds one of several possible pre-written events". Developers can weight events so some appear more often [HADES-GD] [S].
- Kasavin: "It was an explicit goal of our early development, to take the pain out of dying and having to restart" [HADES-GD] [S]. In another interview he sketches the premise of a roguelike where "every time you run into a boss, they remember, you start keeping a tally of who won this time, who won last time" [HADES-RL] [S].
- **Priority.** Players and modders report that story lines take precedence over ordinary dialogue, so a qualifying line can wait. Requirement fields such as `RequiredTextLines` gate a line on earlier lines. A mod "sets key story lines as priority" in `NPCData.lua` [HADES-MOD] [C].
- The GDC 2021 talk "Breathing Life into Greek Myth: The Dialogue of Hades" covers the system [HADES-GDC], but no transcript could be reached. The exact priority rules are **unverified**.
- ***Inference:*** a working reading is that each NPC holds candidate line sets with requirements. Eligible "priority" sets are played before others, most sets play once, and repeatable filler covers the gaps. §3.4 builds on that reading, not on a documented spec.
- **For Mandate:** conversations (DEC-093) compose replies by `weight` with a repetition guard (M6 §3.5). Hades adds two things: **tiers** (reactive and story lines beat filler when eligible) and **once-only callbacks** that react to the player's history (§3.4).

**Slay the Spire: data-driven balance** (Anthony Giovannetti, Mega Crit; GDC 2019 talk [STS-GDC]) [STS] [S]
- The team tracks each card's pick rate (a very low one means the card is "basically not a card in our game at that point") and its win rate, plus damage taken from specific enemies with that card in the deck. Dashboards grew from 3 graphs to "at least 90".
- Example: Dual Wield was rarely picked, so it was changed to copy any card. The data then showed infinite-damage combos, so it was limited to Skill cards.
- Caveats: "The numbers are really useful but they're not telling us how things feel", and "a single well-reasoned post is a lot more useful than lots of players saying 'nerf this.'"
- **For Mandate:** Mandate has no backend (offline-first), so its "players" are the lab bots. E12 should publish per-template and per-verb tables in the Slay the Spire style: open rate, pick rate per option, and outcome by option (§3.7).

**Into the Breach** (Subset Games; GDC 2019 postmortem [ITB-GDC]) [ITB-K] [S]
- Justin Ma and Matthew Davis cut "maybe 60 percent of the game": city building, multiple squads, research trees, and FTL-style text events ("All of it. It was just terrible."). Ma's advice: "This part of the game works. This part of the game doesn't work. Cut all that, and just focus on the part that does work."
- ***Inference (from the game itself, not a cited talk):*** enemies show their next attack before the player moves, so every turn is a fully informed puzzle.
- **For Mandate:**
  - The telegraph is ENGAGEMENT's NPC-scheme stage views: the rival's next move shown as a lead once learned (§4.5, K2). It is a good payoff for knowledge (§3.8).
  - The cutting lesson applies to content volume. ENGAGEMENT §10 lists about 80 new content items; the E12 template table should be used to cut templates that never open or never change a choice (§3.7).

**Dwarf Fortress and Caves of Qud: procedural history as lore**
- A Game Developer blog on both games names the mechanisms: chains of lasting consequences that set off new events, autonomy of characters, and minimal presentation that leaves room for apophenia [DF-RW-GD] [S].
- Dwarf Fortress's Legends mode is a read-only log of the generated world's history "from year 1"; some players skip play and generate thousands of years just to read it [DF-PCG] [S, via search excerpts].
- Caves of Qud's history generator inverts cause and effect: "The system first decides that a particular historical event will occur, then it rationalizes a cause based on the states of the historical entities at play" (Grinblat and Bucklew, 2017) [QUD] [S]; Grinblat's GDC 2018 talk covers the same system [QUD-GDC].
- **For Mandate:** ENGAGEMENT §6.2 already takes curiosity arguments from the burn-in's real history (mayors, elections, disasters, closures). That is DF-style truth. Qud adds *selection for meaning*: pick which history facts become plaques by their bearing on the town's present (§3.8).

### 2.4 Loop and balance theory

**Soren Johnson: "Water Finds a Crack"** (Game Developer, 2011) [WATER] [S]
- "Given the opportunity, players will optimize the fun out of a game" (Johnson credits the idea to Raph Koster).
- "A single dominant strategy actually takes away choice from a game because all other options are provably sub-optimal"; "Games lose their dynamic quality once a strategy emerges that dominates under all conditions." The ideal is a decision that is right in some circumstances and wrong in others.
- His test: "Does the exploit drown out all other play styles, or is it a fun, alternative way to play?" Also ask whether it creates an endless grind.
- Examples:
  - Civ III players replanted and chopped forests: tedious but optimal.
  - Civilization's "infinite city sleaze", where many small cities beat a few large ones. Civ IV answered it with a per-city maintenance cost that grows with the number of cities.
  - Morrowind's skill system rewarded running into walls.
- **For Mandate:** canvass-only is Civ III's lumberjacking: tedious, optimal, and it drowns out other styles. ENGAGEMENT's per-neighbourhood memory (§2.1) is Civ IV's per-city maintenance: the cost grows with repetition, not with use. That is the right shape.

**Sid Meier: "Interesting Decisions"** (GDC 2012) [MEIER] [S]
- "Games are a series of interesting decisions." If players always take the first of three options, "probably not an interesting choice."
- "Good decisions are situational." The kinds he lists are risk against reward, short term against long term, and expression of play style.
- He advises "erring on the side of providing the player with too much information", warns that a run of complex decisions makes players feel out of control, and says "probably a third of the things that we try" get cut.
- **For Mandate:** an ask, a promise or a canvass should be worth more or less depending on the town: whose worry, which neighbourhood, how close the election is. ENGAGEMENT's canvass `fit` (K4) and its season multiplier are situational. §3.5 proposes the gate that checks it.

**David Sirlin: "Balancing Multiplayer Games"** [SIRLIN] [S]
- "The worst thing you can have in a competitive multiplayer game is a dominant move"; "A dominant move also probably has no real counter." Touch "as many corners of the design space as possible", which "tends to protect you from dominant moves".
- "Eliminate all the worthless options because they confuse the player and add nothing."
- Local balance: "the local level, meaning a particular situation in the middle of gameplay, does NOT need to be fair."
- **For Mandate:** the owner's principle (good styles vary; rote ones lose) is Sirlin's "many viable options" with a floor. Not every option must be equal. The good ones must differ by world (DEC-077's variety gate), and the worthless ones should go. This argues for cutting dead templates and verbs, not only adding (§3.7).

**Daniel Cook: loops and arcs** [COOK] [C: summaries]
- Loops are repeated interactions where players learn and master skills. Arcs deliver content once. Games need both.
- **For Mandate:** the daily verbs (canvass, talk, lobby) are loops; dilemmas, storylines, secrets and lore are arcs. Repetition complaints come from a loop with no arcs arriving. The director's job is to keep arcs arriving at a steady rate (§3.1).

**Raph Koster: *A Theory of Fun*** [KOSTER] [C: summaries]
- Fun comes from mastering patterns; "Boredom is always the signal" that the game has stopped teaching.
- **For Mandate:** once a routine has nothing left to teach, the player should be pushed to a new one. The canvass curve makes the routine stop paying. Escalation over a career (§3.9) gives the player something new to master.

### 2.5 Exploration rewards

**Breath of the Wild: Korok seeds and field density** (Nintendo) [BOTW-CEDEC], [BOTW-KOROK], [BOTW-WIKI]
- At CEDEC 2017, Hidemaro Fujibayashi and Makoto Yonezu described the **triangle rule** [C: a translated summary]:
  - Hills and triangular landforms make the player choose over or around, and hide what lies behind ("surprise players, make them wonder what they'll find on the other side").
  - Large triangles are landmarks, medium ones block views, small ones set the pace.
  - Structures of different visibility are placed with **"gravity"** to pull players off their path.
  - Heat maps of playtesters' paths checked the spread.
  - Korok seeds sit in the visually interesting variations of the triangle motif.
- Fujibayashi says Koroks began as rocks, dropped because "simply placing rocks somewhere isn't intriguing or challenging enough to be rewarding" [BOTW-KOROK] [S, via search excerpt].
- There are 900 seeds, but only 441 are needed to max out the inventory; the reward for all 900 is a joke item [BOTW-WIKI] [C].
- **For Mandate:**
  - Put curiosity slots where the eye already goes: bridge ends, high ground, squares, corners at the end of a street. E11 does this with its anchors.
  - Every find must have a small hook (a story or a name), not just a marker.
  - Keep the total small, with no completion reward. ENGAGEMENT's 12–30 per town and "never a total" are right.
  - A walking heat map from the Playwright walker could check that slots are spread out (§3.8).

**Outer Wilds: knowledge as progression** (Alex Beachum, Loan Verneau, Mobius Digital) [OW-GD], [OW-WAND] [S]
- "The goal was to make a game where the only possible reason you could have to explore was for the sake of curiosity." The team removed collectibles and upgrades. The time loop "really focuses on knowledge being the only real thing of value in this world."
- "Everything is on its own clock and things are changing and happening whether you're there or not."
- **Familiarity before curiosity:** an early version crashed an object into the starting village to spark interest. It failed because players did not yet know the village.
- Clues answer one question and point to the next mystery. Beachum wanted to "remove aimless wandering": paths are telegraphed so that the player makes "conscious choices", without "telegraphing where things go" [OW-WAND].
- **For Mandate:**
  - DEC-091's ledger is this progression, and "the world does not wait" is Outer Wilds' clock.
  - Missing: a rule that every find leads somewhere, and the familiarity principle. Lore and encounters should hook onto people and places the player already knows (§3.8).

**Disco Elysium: the Thought Cabinet** (Robert Kurvitz, ZA/UM) [DE] [S]
- Thoughts are "loot for your mind that you collect from the world by talking to people". Each one takes 30 in-world minutes to 3 days to internalise, and slots are limited. They often bring "diabolical side effects" alongside the bonus.
- **For Mandate:** don't add a new system (§4). The cheap lesson is that a found fact should become something the player can *use in talk*. A learned secret or lore subject opens a `topic.about` with it, so finding things feeds the conversation loop (§3.8).

---

## 3. Gap check of `ENGAGEMENT.md`

### What it already does well

- **Repetition costs, not use.** The per-neighbourhood canvass memory (§2.1) is Civ IV's per-city maintenance applied to the right thing [WATER]. It hits the routine and spares campaign canvassing.
- **Cooldowns that grow, and no repeats in a chapter** (§4.1). These match Reigns' "recently played cards leave the bag" [REIGNS] and Frostpunk's per-law cooldowns.
- **Consequences of past answers** (§4.4, `answered`). This is Frostpunk's boiling frog and Hades' reactivity. It is the strongest variety lever in the doc.
- **NPC schemes advancing on real NPC deeds** (§4.5). These are CK III story cycles kept honest (DEC-033 B), shown as leads until learned, like Into the Breach's telegraph.
- **Caps and cheap declines on asks** (§3.2, §3.3). This is the defence against Tropico's demand treadmill.
- **True history as lore, small totals, no completion reward** (§6). This combines Dwarf Fortress's truth with the BotW lesson against hundreds of seeds.
- **The lab as telemetry** (§7). This is Slay the Spire's data-driven balance without a backend.

### What it is missing

1. **A pacing shape.** It fixes the post-climax gap with a floor rule (`quiet_days` 60, `post_climax_significant` 1) instead of a rhythm. Booth and Cassandra both use repeated cycles with a bounded relax.
2. **Priority tiers and a theme in selection.** `pick` weighs relevance × novelty in a band. Nothing guarantees that story-carrying content (storyline stages, consequences whose window is closing) wins over generic templates, and nothing rotates the chapter's theme, so the climax can still come from the same crisis family.
3. **Easing off after a setback** (RimWorld adaptation).
4. **Self-throttling ask generation** (RimWorld population intent). It has a hard cap but no slope.
5. **Conversation priority and callbacks.** M6 §3.5 picks lines by weight with a repetition guard. Nothing makes an NPC *react* to what the player did (a kept or broken ask, a vote) ahead of small talk.
6. **A gate that knowledge pays.** The owner's principle is that learning the town should clearly beat rote play. ENGAGEMENT caps finds (walker ≤ mixed + 1), which is right, but nothing requires that *knowing* the town is worth a clear margin.
7. **Per-template telemetry** to find dead or dominant templates (Slay the Spire, Democracy 4).
8. **A second-term problem.** Democracy 4's "everything gets solved" is not addressed. The re-election check exists (§7.2), but nothing raises the bar for incumbents.
9. **Escalation over a career.** It has no plan to introduce pressures over time (Papers, Please; Not For Broadcast's chapter stages).

Proposed changes follow, each marked with the E-task it changes. All numbers are **Proposed** starting points for tuning on the E12 grid.

### 3.1 Director: a build-up / peak / relax cycle (E2)

Replace the single rise per session with a repeating cycle inside each chapter, after Booth [L4D] and Cassandra [RW-C]. The existing tension curve stays as the envelope that sets how high each peak may go. The cycle sets when.

| Phase | Session (days) | Campaign (days) | What the director may spotlight |
|---|---|---|---|
| Build up | 35–60 (keyed draw) | 7–14 | Minor developments, asks; one significant decision may open once live tension ≥ the curve |
| Peak | until the peak's decision is answered or expires; at most 14 more | at most 7 | That decision; its scene; related press |
| Peak fade | until no decision is pending (Booth's "natural break") | same | Nothing new that is significant |
| Relax | 21–35; after the climax 30–45 | 5–10 | Minor beats only: asks, press about the player, encounters, overheard lines; no significant decision |

- A cycle lasts about 60–110 days, so a 365-day session has room for 3–6 peaks. The budgets still cap them at the proposed significant `[3, 5]` plus the climax (ENGAGEMENT §4.3); a peak that finds the budget spent carries a storyline stage or an ask instead.
- The climax is the peak whose build-up ends inside `climax_window`. Narrow it from `[0.6, 0.92]` to `[0.65, 0.85]` so that at most about 55–125 days follow it: one relax and one "coda" build-up. The coda takes only consequence (`answered`), opportunity, or budget-preparation templates (ENGAGEMENT's `post_climax_significant` rule, now with a place in the cycle).
- **Relax ends early** when the player starts something (introduces a bill, files for office, accepts a big ask), as Booth's relax ends when survivors move on. An endogenous crisis also ends it: the director never hides the world (DEC-033 B), it only stops spotlighting new optional decisions.
- **Expected effect:** the decisions-only gap is at most a relax plus a build-up (≤ 45 + 60 = 105 days), under E2's acceptance of ≤ 120. Gaps with no beat at all are filled by asks and press during relax, which brings the target of ≤ 60 within reach (ENGAGEMENT §7.2).
- **Profiles** (`storytellers.jsonc`) scale phase lengths, not severity: "amplitude is not changed, frequency is" [L4D]. Steady: relax × 1.3, build-up × 1.2. Unfiltered: no phases (pure chance within budgets, as today).
- **Adaptation** [RW]: after a setback (an election lost, Standing or viability below the fairness warning level, a scandal about the player surfacing), the next relax is × 1.5. The next build-up draws opportunity or recovery templates before crisis templates. This changes only spotlighting.
- **State:** `NarrativeState` gains `phase: DirectorPhase { kind, since: SimDate, until: SimDate }` (one enum, fits the save bump E2 already has). Phase lengths come from keyed draws on the director stream, keyed by chapter and cycle number, so the draw count never depends on the player's knowledge.
- **Metric for `story --metrics`:** peaks per chapter, the longest relax, and the share of relax days that carried at least one minor beat (target ≥ 50%).

### 3.2 Choosing templates: tiers, a chapter theme, a varied climax (E2, E4)

Keep ENGAGEMENT §4.2's `pick` (a band of relevance, × novelty, keyed draw), and add:

1. **Tiers** (after Hades; *inference*, see §2.3). Candidates fall into three tiers, and `pick` draws from the highest non-empty tier.
   - Tier A: storyline stages, and `answered` consequences whose window closes within 60 days.
   - Tier B: templates tied to the chapter theme.
   - Tier C: the rest.

   Story-carrying content therefore cannot be starved by a high-relevance generic template, which is today's failure mode (five templates carried 60% of decisions).
2. **A chapter theme** (after Reigns' war cards [REIGNS]). At chapter open, a keyed draw picks a theme from those the live state supports:
   - `crisis` (the live crisis);
   - `budget` (fiscal stress);
   - `scheme` (an NPC scheme is active);
   - `campaign` (always, in campaigns);
   - `local_life` (no crisis: services, schools, the market, heritage).

   Templates carry `themes: [...]`; themed ones get × 2.0 weight. The same theme cannot be drawn in two consecutive session chapters while another is eligible. That rotation, together with `climax_memory_chapters` 2, is what stops the climax returning to pollution four years running.
3. **Storyline starts draw, not take** (after the Stellaris precursor fix [STEL-146]). When several storylines can start, draw among them weighted by relevance × novelty, so a town with a plant crisis does not always get `storyline.plant` first.

### 3.3 Asks: an intent curve, not only a cap (E6, E8)

Following RimWorld's population intent [RW], scale each week's chance of an offer by two curves, interpolated linearly. The hard cap of 4 stays.

| Open asks (offered + accepted) | 0 | 1 | 2 | 3 | 4 |
|---|---|---|---|---|---|
| Factor | 2.0 | 1.3 | 1.0 | 0.5 | 0 |

| Days since the last offer | 0 | 7 | 21 | 42 |
|---|---|---|---|---|
| Factor | 0 | 0.5 | 1.0 | 1.5 |

- In content these are `asks.params.intent_by_open` and `intent_by_quiet` (`[x, factor]` pairs, as in RimWorld's curves).
- Effect: a player with nothing open gets asked sooner, so asks fill relax phases. A busy player is left alone (Tropico's treadmill [TROP]).
- `new_per_month` becomes the *base* rate that the curves multiply. E8 tunes it so the median stays at ≥ 15 offers in 5 years.
- **Callbacks in asks.** A person whose earlier ask the player kept is × 1.5 likelier to be the next asker of that kind; one whose ask was broken is × 0.5. The ask text names the history ("You sorted the drain last spring…"). This is a cheap Hades-style tally [HADES-RL].

### 3.4 Conversations: priority tiers, once-only lines, callbacks (C2–C5; uses E6's deeds)

M6 §3.5 picks each part's line by `weight` after a repetition guard. Add, as content fields on `TalkLineDef`:

```text
priority: u8          // 0 filler, 1 situational, 2 reactive (to the player's recent deeds or a memory), 3 story (a storyline, secret or ask beat)
once: none | speaker | run   // a callback said once per speaker, or once per run by anyone
after: [line ids]     // only after the player has heard these lines (from anyone)
```

- **Selection, per part:** gather the lines whose `when` holds; drop `once` lines already said and lines in the speaker's used memory (as today); take the **highest priority tier** that has a candidate; draw by `weight` within it with the existing keyed draw. Filler fills only when nothing reactive or story-bearing applies, as in Hades (*inference*, §2.3).
- **New condition for reactive lines** (needs E6's `PlayerDeed` log): `PlayerDid { deed, within_days, scope: speaker | speaker_home | here }`, plus `AskOutcome { with: speaker, status: kept | broken, within_days }`. For example, "You voted for the depot. People here noticed." or "Still waiting on that drain."
- **The tally** [HADES-RL]: `Remembers` (M6 §3.3) gains `$count` (how many memories of that kind the speaker holds), so lines can say "third time you've come asking about the plant".
- **State:** `heard_once: Vec<(ContentKey, Option<CharacterId>)>`, sorted and bounded at 400 (no `HashSet`, DEC-041), in the conversations domain. One save bump, foldable into C-task bumps.
- **Content target** (*inference*): about 10% of substance lines reactive or story (priority ≥ 2). In 5-year careers, at least 25% of talks with a recurring speaker carry a priority ≥ 2 line, measured by C5's repetition gate.

### 3.5 Balance: a skill-gap ladder that encodes the owner's principle (E1, E8, E12)

Johnson's test [WATER] and Sirlin's viable options [SIRLIN], applied to the owner's rule ("the intended way clearly beats rote routines; variety among good styles"). ENGAGEMENT's `single_verb_not_best` is the bottom rung. Add the rest of the ladder to `bots` (first election, mean vote share, 16 and 32 seeds):

| Gate | Rule (**Proposed**) | Today (ENGAGEMENT §1.1) |
|---|---|---|
| `ladder_rote` | `mixed` ≥ every single-verb strategy + 1.0 point | mixed 0.618 vs canvass 0.630: fails |
| `ladder_learned` | `explorer` (learn first, then mix) ≥ every single-verb strategy + 2.0 points, and ≥ `mixed` + 1.0 (today's `exploring_pays`) | explorer 0.630 = canvass: fails |
| `knowledge_matters` | `mixed --omniscient` ≥ `mixed` + 1.5 points: knowing the town is worth a clear margin | not measured |
| `bonus_not_route` | `walker`, `secret_hunter` ≤ `mixed` + 1.0 (ENGAGEMENT's `walking_is_a_bonus`, `secrets_not_decisive`) | — |
| `good_styles_vary` | Among multi-verb strategies within 1.5 points of the best, at least 2 are each the sole best in ≥ 20% of worlds | variety gate today: 2 sole-best somewhere |

- Meier's "situational" rule [MEIER] is what makes the ladder passable without a flat nerf. The value of each verb must depend on what the player knows of the world: the canvass `fit` reads the worry the player knows (K4, `bots.rs` `canvass`), and promises and asks pay most when aimed at known concerns.
- If `knowledge_matters` fails, the fix is to make outcomes depend more on matching the town (bigger `fit` spread), not to give knowledge flat bonuses.

### 3.6 The second term: a rising bar (E2, E3; V1)

From Democracy 4 [D4-LONG]: a solved town and a coasting incumbent make the late game dull.
- **Consequences keep problems alive.** A law the player sponsored produces a follow-up dilemma a year on (`dilemma.law_anniversary` exists). Add a rule: each enacted player law has a 50% chance (keyed) of seeding one `answered`-style follow-up within 12–24 months, drawn from that law's family. **Proposed**; families arrive with F2.
- **Expectations rise for incumbents.** In a term after re-election, ask templates draw their `deadline_days` from the lower half of their range, and campaign-promise persuasion counts only promises not already kept in the previous term. **Proposed**, *inference*: the intent is that delivery last term is not coasting credit this term. Today's 10% monthly approval pull already fades old delivery (§2.2).
- **Gate:** ENGAGEMENT's re-election check (`canvass` re-election rate ≤ `mixed`'s) stays. Add one: `mixed`'s second-term decisions per chapter ≥ 80% of its first-term rate.

### 3.7 Telemetry: template and verb tables (E12)

After Slay the Spire [STS] and Democracy 4's trigger tables [D4-EVENTS], `engagement --tables` prints these across all careers:
- **per template:**
  - share of careers where it opened;
  - mean opens per career;
  - share answered with each option;
  - the option's mean approval and Standing change after 90 days;
  - "never opened in 32 seeds" and "eligible but never picked";
- **per verb:** uses per career and the marginal approval per use, by season, for each strategy;
- **per ask template:** offered, accepted, kept, broken.

Checks (report-only at first, like the other engagement gates):
- no template opens in more than 60% of careers *and* more than twice per career;
- every template opens in at least one of 32 seeds, or is listed as gated on a later task (E3's rule, made automatic);
- no option is picked by `mixed` in more than 90% of its opens. A Meier "always the first option" signals a non-decision; rework or cut it (Ma's cutting [ITB-K]).

The tables are the "what". The playtest notes in BALANCE_LOG remain the "how it feels" [STS].

### 3.8 Knowledge as progression (E9, E10, E11)

From Outer Wilds [OW-GD], BotW [BOTW-CEDEC], Caves of Qud [QUD] and Disco Elysium [DE]:
1. **Every find leads somewhere.** Validation (E9, E10): every `streets` curiosity kind, secret kind and lore kind with a living subject declares at least one `lead` (a person, place or subject). Pure-flavour `walk` kinds are exempt. The notebook shows per subject that there is "more to learn", never a total (M6 §1.8).
2. **Familiarity first.** Seeding must not depend on knowledge (DEC-091), but *text* may. Lore lines get a variant chosen by `PlayerKnows` ("the plaque names Dana Price's grandmother" when Dana is known). That is framing, which is allowed. The first-year encounter topics (E10 §6.5) prefer people the player has met or places the player has visited, so new mysteries attach to known ground.
3. **Lore that bears on now.** When E10 picks history facts for curiosity arguments, weight them × 2 if they share a kind with the live crisis or the defining industry (a flood plaque in a flood-prone town, a pit memorial in a mining town). Qud's "pick the event, then explain" [QUD], applied at seeding; still deterministic.
4. **Finds become talk.** A learned secret, lore subject or curiosity person becomes an `about` option for `topic.about` with the people who know of it (M6 §3.4 `speaker_knows`). Exploring then feeds the conversation loop, which is the cheap version of the Thought Cabinet's "loot for your mind" [DE].
5. **Telegraphs as a payoff.** Once the player knows `RivalPlans` or `Broker` at `known`, a scheme's next stage shows as a pending threat with its likely target and timing, as Into the Breach shows enemy intents (*inference*). Unknown, it stays a lead. ENGAGEMENT §4.5 already gates the views; this names the payoff.
6. **Placement check** (E11): from the Playwright walker or a scripted path set, compute a per-district heat map of how often slots are passed within 3 tiles. Flag districts where more than 30% of slots are never passed (BotW's heat maps [BOTW-CEDEC]). This check is about test coverage; it does not affect state.

### 3.9 Escalation over a career, and the campaign (E2, E3, E4)

- **New pressures by year** (Papers, Please [PP-GI]; Not For Broadcast's staged chapters [NFB]). **Proposed:** templates and storylines may carry `min_career_days`. As a starting split:
  - year 1: casework, local asks, council votes;
  - from about day 300: schemes and secrets;
  - from the first re-election: big storylines and the investigation.

  It is a soft gate on spotlighting only; the world's events happen regardless. The first year then teaches the loop, and each later year adds something to master (Koster [KOSTER]).
- **The campaign is not The Political Machine** [PM-GA]. Campaign chapters get their own phase lengths (§3.1) and the `campaign` theme. E3/E4 should make sure at least 6 campaign-only templates or stages exist: a debate, a rival attack on the record, an endorsement contest, a gaffe, a door that slams, and a promise challenged. Whether that many exist today is not checked (*inference*: the current pool is mostly governing templates). The campaign quiet gate (≤ 21 days) stays.

---

## 4. Things not to copy

| Technique | Why not |
|---|---|
| RimWorld's **wealth-scaled threat points** [RW] | They punish success. In Mandate that would punish delivery, and scaling consequences breaks DEC-033 B. Scale *frequency* (§3.1), never stakes |
| **Randy Random** as a default [RW] | Pile-ups are funny over a 1-hour colony but ruin an 8-hour career (DEC-033's own reasoning). Keep him as the opt-in Unfiltered profile |
| Democracy 4's hidden **"winning" difficulty measure** [D4-LONG] | Rubber-banding the player cannot see conflicts with DEC-051 ("Why?") and honest consequences. Raise the bar through visible, causal means (§3.6) |
| Live **telemetry servers** (Slay the Spire) [STS] | Mandate is offline-first with no backend. The bot lab is the telemetry; add no analytics |
| **900 Koroks** and completion rewards [BOTW-WIKI] | Hundreds of finds become a checklist. Mandate's 12–30 per town with no totals is the right scale (ENGAGEMENT §6) |
| **Hades-scale scripts** (400,000+ words in Hades II [HADES-GR]) | Mandate composes replies from parts (DEC-093). Callbacks (§3.4) need tens of lines per pack, not hand-written dialogue per event |
| Papers, Please's **deliberate monotony** [PP-REV] | It serves that game's argument. The owner wants variety |
| Tropico's **faction demand treadmill** [TROP] | Many always-on demands with unclear payoffs. Asks stay few, grounded, and cheap to decline (§3.3) |
| A **new Thought Cabinet** system [DE] | It took ZA/UM years and nearly sank the project [DE-GS]. Use finds as topics (§3.8) instead |
| Stellaris's **location-locked chains** [STEL-146] | A fixed mapping from start to story makes every run of that kind the same. Draw among the eligible arcs (§3.2) |
| CK III's **barrage of small random events** [CK3-DD75] | Volume is not variety. Keep minor budgets and spacing; prefer consequences over fresh random beats |
| Left 4 Dead's **per-moment difficulty** [L4D] | Booth himself changes only frequency. Copy the phase machine, not any adjustment to outcomes |

---

## 5. Sources

Access dates are 2026-11-19. "Search excerpt" means the claim was read in a search result's quoted text, not on the full page.

| Key | Title | Author / publisher | URL |
|---|---|---|---|
| [L4D] | The AI Systems of Left 4 Dead (slides, 2009) | Michael Booth, Valve | https://cdn.fastly.steamstatic.com/apps/valve/2009/ai_systems_of_l4d_mike_booth.pdf |
| [RW] | AI Storytellers | RimWorld Wiki (community) | https://rimworldwiki.com/wiki/AI_Storytellers |
| [RW-C] | Cassandra Classic | RimWorld Wiki (community) | https://rimworldwiki.com/wiki/Cassandra |
| [RPS-RW] | How RimWorld Generates Great Stories (2016) | Rock Paper Shotgun, interview with Tynan Sylvester | https://www.rockpapershotgun.com/2016/08/12/how-rimworld-generates-great-stories |
| [RW-GDC] | RimWorld: Contrarian, Ridiculous, and Impossible Game Design Methods (GDC 2017) | Tynan Sylvester | https://gdcvault.com/play/1024686/-RimWorld-Contrarian-Ridiculous-and |
| [DF-RW-GD] | RimWorld, Dwarf Fortress, and procedurally generated story telling (2019) | Henri Brouard, Game Developer | https://www.gamedeveloper.com/design/rimworld-dwarf-fortress-and-procedurally-generated-story-telling |
| [CK2-MTTH] | Help with events (mean_time_to_happen modifiers) | Paradox forum (community) | https://admin-forum.paradoxplaza.com/forum/threads/help-with-events.407123/latest |
| [CK3-SC] | Story cycle effect_group chance (bug report quoting the peasant affair cycle) | Paradox forum (community) | https://admin-forum.paradoxplaza.com/forum/threads/story-cycle-effect-group-chance-out-of-1-instead-of-100.1594492 |
| [CK3-DD50] | Crusader Kings 3 Dev Diary #50 – Poetry to my Ears | Paradox (forum) | https://forum.paradoxplaza.com/forum/threads/crusader-kings-3-dev-diary-50-poetry-to-my-ears.1460062/ |
| [CK3-DD75] | Crusader Kings 3 Dev Diary #75 – In the Event of Court Events | Paradox (forum) | https://forum.paradoxplaza.com/forum/developer-diary/crusader-kings-3-dev-diary-75-in-the-event-of-court-events.1492792/ |
| [STEL-145] | Stellaris Dev Diary #145 – Archaeology | Paradox (forum) | https://forum.paradoxplaza.com/forum/threads/stellaris-dev-diary-145-archaeology.1170190/ |
| [STEL-146] | Stellaris Dev Diary #146 – The Baol and the Zroni (thread replies) | Paradox (forum) | https://admin-forum.paradoxplaza.com/forum/developer-diary/stellaris-dev-diary-146-the-baol-and-the-zroni.1173457 |
| [REIGNS] | Game Design Deep Dive: Creating an adaptive narrative in Reigns | François Alliot, Game Developer | https://www.gamedeveloper.com/design/game-design-deep-dive-creating-an-adaptive-narrative-in-i-reigns-i- |
| [REIGNS-GDC] | The Casual (but Regal) Swipe: Creating Game Mechanics in Reigns (GDC 2017) | François Alliot | https://www.gdcvault.com/play/1024278 |
| [PP-GI] | Papers, Please 10 Years Later (2024) | Game Informer, interview with Lucas Pope | https://www.gameinformer.com/2024/01/02/papers-please-10-years-later |
| [PP-REV] | Papers, Please review (Press Start journal) and reviews (search excerpts) | Press Start; various | https://press-start.gla.ac.uk/press-start/article/download/70/61/448 |
| [FP-PCG] | Frostpunk developers on hope, misery, and the ultimately terrifying book of laws | PC Gamer (search excerpt) | https://www.pcgamer.com/frostpunk-developers-on-hope-misery-and-the-ultimately-terrifying-book-of-laws |
| [FP-GP] | A shard of ice instead of a heart: on moral choices (with 11 bit studios) | Gamepressure (search excerpt) | https://www.gamepressure.com/editorials/fifth-dont-kill-on-moral-dilemmas-in-video-games/zf4b9-2 |
| [FP-WIKI] | Public Penance (Law) | Frostpunk Wiki (community) | https://frostpunk.fandom.com/wiki/Public_Penance_(Law) |
| [D4-LONG] | An update on balancing Democracy 4 for longer games (2022-02-24) | Cliff Harris, Positech | https://positech.co.uk/cliffsblog/2022/02/24/an-update-on-balancing-democracy-4-for-longer-games |
| [D4-EVENTS] | Improving Democracy 4 events balance by using a LOT of data (opening, search excerpt) | Cliff Harris, Positech | https://www.positech.co.uk/cliffsblog/category/game-design/page/5 |
| [PM-GA] | The Political Machine 2024 review | Matthew Pollesel, Gaming Age | https://gaming-age.com/?p=95507 |
| [PM-SP] | The Political Machine 2016 review | Softpedia | https://www.softpedia.com/reviews/games/pc/the-political-machine-2016-review-500164.shtml |
| [PM-HG] | The Political Machine 2008 review | Honest Gamers | https://www.honestgamers.com/7452/pc/the-political-machine-2008/review.html |
| [TROP] | Tropico 6 reviews (search excerpts) | TechRaptor; Thumbsticks; others | https://techraptor.net/content/tropico-6-review |
| [SUZ] | Suzerain reviews (search excerpts) | Gaming Trend; Jump Dash Roll | https://gamingtrend.com/reviews/youve-got-my-vote-suzerain-review/ |
| [NFB] | Not For Broadcast full review (search excerpt) | Otaku Study | https://www.otakustudy.com/video-game/2022/01/not-for-broadcast-full-review/ |
| [HADES-GD] | How Supergiant weaves narrative rewards into Hades' cycle of perpetual death | Game Developer, quoting Greg Kasavin | https://www.gamedeveloper.com/design/how-supergiant-weaves-narrative-rewards-into-i-hades-i-cycle-of-perpetual-death |
| [HADES-RL] | Roguelikes and narrative design with Hades creative director Greg Kasavin | Game Developer | https://www.gamedeveloper.com/design/roguelikes-and-narrative-design-with-i-hades-i-creative-director-greg-kasavin |
| [HADES-GDC] | Breathing Life into Greek Myth: The Dialogue of Hades (GDC 2021) | Greg Kasavin, Darren Korb | https://gdconf.com/news/dive-dialogue-hades-gdc-2021 |
| [HADES-MOD] | Story-priority mod notes; Steam threads on dialogue requirements (community) | Nexus Mods; Steam Community | https://www.nexusmods.com/hades/mods/47 |
| [HADES-GR] | Hades 2 has a script of over 400,000 words | GamesRadar | https://www.gamesradar.com/games/hades/hades-2-has-an-epic-script-of-over-400-000-words-and-30-000-voice-lines-around-50-percent-more-than-the-original-roguelike |
| [STS] | How Slay the Spire's devs use data to balance their roguelike deck-builder | Game Developer, quoting Anthony Giovannetti and Casey Yano | https://www.gamedeveloper.com/design/how-i-slay-the-spire-i-s-devs-use-data-to-balance-their-roguelike-deck-builder |
| [STS-GDC] | Slay the Spire: Metrics Driven Design and Balance (GDC 2019) | Anthony Giovannetti | https://gdcvault.com/play/1025731 |
| [ITB-K] | The creators of Into the Breach came very close to giving up on it | Kotaku (Splitscreen, GDC 2019) | https://kotaku.com/the-creators-of-into-the-breach-came-very-close-to-givi-1833498295 |
| [ITB-GDC] | Into the Breach Design Postmortem (GDC 2019) | Matthew Davis | https://www.gdcvault.com/play/1026333 |
| [DF-PCG] | Dwarf Fortress creator Tarn Adams interview (search excerpt) | PC Gamer | https://www.pcgamer.com/dwarf-fortress-creator-tarn-adams-talks-about-simulating-the-most-complex-magic-system-ever |
| [QUD] | Subverting Historical Cause & Effect: Generation of Mythic Biographies in Caves of Qud (FDG 2017) | Jason Grinblat, C. Brian Bucklew | https://pcgworkshop.com/archive/grinblat2017subverting.pdf |
| [QUD-GDC] | Procedurally Generating History in Caves of Qud (GDC 2018) | Jason Grinblat | https://gdcvault.com/play/1025379/Procedurally-Generating-History-in-Caves |
| [WATER] | GD Column 17: Water Finds a Crack (2011) | Soren Johnson, Designer Notes | https://www.designer-notes.com/game-developer-column-17-water-finds-a-crack/ |
| [MEIER] | GDC 2012: Sid Meier on how to see games as sets of interesting decisions | Game Developer | https://www.gamedeveloper.com/design/gdc-2012-sid-meier-on-how-to-see-games-as-sets-of-interesting-decisions |
| [SIRLIN] | Balancing Multiplayer Games, Part 2: Viable Options | David Sirlin | https://sirlin.net/articles/balancing-multiplayer-games-part-2-viable-options |
| [COOK] | Loops and Arcs (summaries and sketchnotes) | Daniel Cook (Lostgarden); Mechanics of Magic | https://mechanicsofmagic.com/?p=43051 |
| [KOSTER] | A Theory of Fun, 10 years later (GDC Online 2012 keynote) | Raph Koster; Engadget report | https://www.engadget.com/2012/10/10/gdc-online-2012-raph-kosters-keynote-on-a-theory-of-fun-10-ye/ |
| [BOTW-CEDEC] | CEDEC 2017 Breath of the Wild field design talk (translated summary) | Matt Walker translation, reposted | https://gist.github.com/idbrii/e39fe96279aa1670319bfa521d907399 |
| [BOTW-KOROK] | BotW director: Korok seeds were meant to be stones (search excerpt) | DualShockers, quoting Hidemaro Fujibayashi | https://www.dualshockers.com/breath-of-the-wild-korok-seeds/ |
| [BOTW-WIKI] | Korok Seed | Zelda Dungeon Wiki (community) | https://www.zeldadungeon.net/wiki/Korok_Seed |
| [OW-GD] | Live, die, repeat: how Outer Wilds piques curiosity in an ambivalent solar system (2019) | Game Developer, quoting Alex Beachum and Loan Verneau | https://www.gamedeveloper.com/design/live-die-repeat-how-i-outer-wilds-i-piques-curiosity-in-an-ambivalent-solar-system |
| [OW-WAND] | The Intentionality of Wandering (2016) | Mobius Digital, interview with Alex Beachum | https://www.mobiusdigitalgames.com/blog/the-intentionality-of-wandering |
| [DE] | Disco Elysium: Introducing the Thought Cabinet (2019) | GameBanshee, summarising Robert Kurvitz's dev blog | http://www.gamebanshee.com/39hc9 |
| [DE-GS] | How the feature that almost sunk Disco Elysium was saved | GameSpot (search excerpt) | https://gamespot.com/articles/how-the-feature-that-almost-sunk-disco-elysium-was/1100-6472676/ |

**Not verified this session:**
- the exact Hades priority rules (the GDC talk could not be read);
- CK III's `cooldown` event field and story-cycle documentation (the CK III wiki blocked automated access);
- Into the Breach's design reasoning on telegraphed attacks (only the cutting lesson was sourced);
- Cliff Harris's conclusions in the events-balance post (only its opening was read).

The claims built on these are marked as inference above.
