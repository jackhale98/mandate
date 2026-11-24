# Extending Mandate

How to add things to the game after release (DEC-075). There are three kinds of change:

1. **Pure content.** JSONC and Fluent in a pack, with no code. Most expansions are this.
2. **A code extension point.** New behaviour behind a closed enum. Each is one variant, one handler, and a test.
3. **A save-format change.** Needed when the serialised state's shape changes.

## Packs and load order

- A pack is a folder under `content/` with a `pack.jsonc` manifest (`id`, `version`, `name_key`, `load_order`), JSONC files of any `kind`, and Fluent files under `locale/<lang>/`.
- Every file is checked against `content/schemas/content.schema.json`, which is generated from the Rust types (DEC-053). After changing a content type, run `cargo run -p mandate-cli -- content schema` and commit the result.
- Registries (sectors, attitudes, communities, concerns, name pools, scenarios, places, constitutions, policies, traits, occupations, crises, storytellers, dilemmas, goals, storylines, scenes, outlets, town districts, terrain features, connectors) are in **load order**: files sorted by path, then item order. The engine stores registry data densely in that order. A pack whose path sorts before `base/` puts its entries first. That is supported: saves record registry keys, not positions.
- Content ids are unique across all packs. Overriding an entry from an earlier pack ("pack patches") is not supported yet: a duplicate id is an error.
- Every `name_key` and every other Fluent key that content references must exist in English, the reference locale (DEC-054). The engine never produces prose.
- Check content with `cargo run -p mandate-cli -- content validate`.

## Pure content

| Add | Where (`kind`) | Required | Validation | Save rule on load |
|---|---|---|---|---|
| Policy | `policies` | params, cost, `mechanisms`, `signature` (attitude key → weight), `secondary_keys`, `side_effects` (at least one downside, DEC-073) | Params referenced by mechanisms exist; ranges are sane; signature keys are known attitudes; Fluent keys exist | Enacted instances name their template. Removing a template the save uses is refused (`missing_references`). |
| Sector | `sectors` | `driver` (one of the coded behaviours) plus traits | Unique ids; sane traits; at least one `building`, `local_services`, and `public` sector | `economy.sector_keys`. Matching keys keep their data. A new sector starts with no jobs and a capacity from its endowment. A removed sector is refused. |
| Attitude (an issue) | `attitudes` | `id`, short `key`, `seed` model; `conviction-pro-<key>`, `conviction-anti-<key>`, and `issue-<key>` Fluent keys | Unique ids and keys (`[a-z][a-z0-9_]*`); finite seeds; conviction and issue keys exist | `population.attitude_keys`. Values are kept by key. A new attitude starts without randomness: cohorts at the seed mean (plus their community's offset), politicians and Named people at their home neighbourhood's view, parties neutral. A removed attitude is refused. |
| Community | `communities` | `id`, `name_key`; `share` for every community but one; optional `attitude_offsets`, `turnout_offset`, `immigration_share` | At least one community; exactly one without `share` (it takes the remainder); values finite and in range; offsets name known attitudes. Shares that add up to more than 1 give a warning. | `population.community_keys`. Cohorts and children keep their community. A new community starts with no people and grows through immigration and births. A removed community is refused. |
| Voter concern | `concerns` | `id`, `name_key`, `indicator` (one of the engine's measures), `attitudes` (key → signed weight), `level` (`baseline`, `slope`, `min`, `max`); optional `overheard_key` | Known attitudes; weights non-zero in [-1, 1]; `min` ≤ `max`; Fluent keys exist. An empty registry is allowed (every issue then weighs the same). | `population.concern_keys`. Levels and salience are derived: content that adds or removes a concern recomputes them on load. Never refused. |
| Scenario (a starting world) | `scenarios` | `id`, `name_key`; optional `geography`, `economy`, `era`, `politics` biases | Finite, in range; `attitude_shift` names known attitudes; the name exists in English. The title screen lists the registry. | `meta.params.scenario`. Removing the save's scenario is refused (terrain is rebuilt from it). |
| Place behaviour | `places` | `key` (a place kind worldgen generates, or `home` / `streets`), `name_key`, `actions`, `services`, optional `fallback` | Known actions and services; late-night actions are offered; fallbacks resolve; every generated kind has an entry | Behaviour is read live. Nothing in a save is keyed by it. |
| Character trait | `traits` | `id`, `name_key`, `chance`; optional `personality` (axis → offset), `memory_decay` and `opinion_change` (`positive`/`negative` multipliers), `excludes` | Axes are `ambition`, `integrity`, `warmth`, `boldness`, `loyalty`; offsets in [-1, 1]; multipliers in [0, 10]; excluded traits exist; the name exists in English | Characters store trait ids. Removing a trait a stored character has is refused (`missing_references`); a new trait is drawn only for characters created from then on. |
| Occupation (a role of Named people) | `occupations` | `id`, `name_key`, `places` (place kinds or `streets`), `chance`, `class` ([low, middle, high] weights), `age` ([min, max]); optional `leader`, `influence`, `stake` (`drivers`, `crises`), `personality`, `competencies`, `expertise` (an attitude key), `ambient` | Known place kinds, attitudes, and crisis kinds; values in range; the name exists in English | Named characters store their occupation id. Removing one a stored character has is refused. New occupations appear in new games (Named people are generated once, when play begins) and as passers-by at once if `ambient`. |
| Crisis kind | `crises` | `id`, `name_key`, `indicator`, `begin`, `end`, `severe`; optional `concern`, `policies` | `end` ≤ `begin` ≤ `severe`; the concern and policies exist; the name exists in English | Derived: `characters.crisis_keys`; content that adds or removes a kind recomputes every town's character on load. Never refused. |
| Dilemma or request | `dilemmas` | `id`, `title_key` (a noun phrase: memories quote it), `situation_key`, `category` (`significant`, `climax`, `request`), `roles`, `cooldown_days`, 2–4 `options` (each `label_key`, `aftermath_key`, `effects`; exactly one `default`); optional `triggers`, `deadline_days`, `scene`, `opportunity`, `storyline_only` | Roles resolve (cast before use); every option has a cost and a benefit (DEC-073); one default, which costs no time; memory kinds, templates, occupations, scenes exist; a request is raised by a goal or a storyline; Fluent keys exist. An unknown crisis or concern in a condition is a warning (it never holds) | Decisions name their template. A pending decision whose template was removed lapses on load; nothing is refused |
| NPC goal | `goals` | `id`, `name_key`, `holder` (`occupations`, `officeholders`), `drive`, `max_days`; optional `personality` ranges, `traits`, `conditions` (role `holder`), `request` | Known axes, traits, occupations; council drives need officeholders; the request is a `request` dilemma | `characters.goals` name their template; removed goals are dropped on load |
| Storyline | `storylines` | `id`, `title_key`, `faded_key`, `roles`, `stages` (`id`, `key`, optional `advance`, `decision`, `branches`, `ends`, `timeout_days`, `wait_days`), `cooldown_days`; optional `start`, `follow_up` | Stage ids unique; branches name real options and stages; a stage ends the arc; roles shared with a stage's dilemma are cast the same way | Runs name their template; a run whose template was removed ends on load |
| Scene | `scenes` | `id`, `place` (`workplace` of a role with a `fallback` kind, or a `kind`), `beats` (`speaker` role or `player`, `line`, optional `quote_fallback`) | Place kinds exist; every speaker is cast by each dilemma that uses the scene; keys exist | Read live |
| Outlet (a newspaper, broadcaster, or social platform) | `outlets` | `id`, `name_key` (`$outlet_place`), `scope` (`town`, `region`, `nation`), `reach`, `audience` (`class` [3], `age` [4] weights), `credibility` [min, max], `lean_sd`, `finances` [min, max], `stories_per_week`; a town outlet's `place` (its home place kind, DEC-079); optional `social`, `interests` (concerns), `journalists` (occupations at its place) | A town outlet names a known place kind; ranges; known occupations; unknown concerns warn; the name exists in English | Outlets are generated when play begins (and on loading a save from before media) and stored with their template id; a removed template's outlets stay as they are |
| Shady dilemma option | `dilemmas` (an option's `hidden` effect) | `kind` (Fluent `fact-<kind>`), `severity` (`ethics`, `misconduct`, `crime`), `evidence` (0..1); optional `discoverers`, `with_role` | Evidence in range; the kind is snake case and its `fact-<kind>` key exists; the option still needs a real benefit (the hidden fact is only a cost) | Hidden facts store their kind, fate, and window; nothing is refused |
| Storyteller profile | `storytellers` | `id`, `name_key`, `desc_key`, `tension_curve`, `defer_hazards`, `hazard_rate`, `hold_recessions`, `budget_scale`; optional `crisis_cap` | Rates and scales in range; `director.default_profile` exists | `narrative.profile` names one; an unknown one falls back to the default |
| Constitution | `constitution` | Tiers, bodies, offices, procedures, election calendars | Structural checks and Fluent keys | `meta.constitution`. Removing the save's constitution is refused. |
| Language | `locale/<lang>/*.ftl` | Translations of the English keys | Referenced keys are checked against English only. The web currently requests `en`, so choosing another language is a small UI change. | None |
| Tuning, economy, politics parameters | `tuning`, `economy`, `politics` | One file of each across all packs | Ranges | Read live. A balance patch changes results from load onwards, and a `RulesUpdated` event is recorded. |
| Balance parameter blocks | `opinion`, `actions`, `council`, `career`, `npc`, `characters`, `director`, `media`, `justice` | One file of each across all packs, every field written | Finite and in range (`validate::param_ranges`); `[min, max]` pairs ordered | Read live, as above. |

Community share model, as an example of the content format:

```jsonc
{ "id": "community.first_nation", "name_key": "community-first_nation",
  // Share of every age × class group: base + by_neighborhood[kind] + a
  // regional offset (sd regional_sd), clamped to [0, 1].
  "share": { "base": 0.0, "by_neighborhood": { "rural": 0.35 }, "regional_sd": 0.03 },
  "attitude_offsets": { "localism": 0.3, "environment": 0.25 },
  "turnout_offset": -0.1,
  "immigration_share": 0.0 }
```

### Concerns and issue salience (DEC-078 decision 3)

Each month every town reads its indicators and turns each concern into a level, `clamp(baseline + slope × indicator, min, max)`. Each attitude's raw salience is `politics.vote.salience_base` plus every concern's level × |its weight on that attitude|. The vector is scaled to a mean of 1, and the vote model's issue term weighs each attitude by it. A larger `salience_base` mutes local priorities. The sign of a weight is the side worried voters lean toward; it ranks concerns by neighbourhood for display (`concerns::felt_concerns`).

```jsonc
{ "id": "concern.jobs", "name_key": "concern-jobs", "overheard_key": "overheard-jobs",
  "indicator": "unemployment",
  "attitudes": { "redistribution": 0.6, "reform_appetite": 0.3, "localism": 0.2 },
  "level": { "baseline": -0.15, "slope": 9.0, "min": 0.0, "max": 1.0 } }
```

Indicators (`mandate_content::Indicator`, computed in `mandate_sim::concerns::indicator`): `unemployment`, `unemployment_rise`, `rent_burden`, `pollution` (the region's), `tax_burden`, `budget_stress`, `recent_disaster`, `job_loss` (share of jobs lost over a year). There is no crime in the simulation yet, so there is no safety concern. Query a town's concerns with `concerns::top_concerns(pop, m)` (by level) or `concerns::felt_concerns` (for some neighbourhoods); its salience with `concerns::salience(pop, m)`.

### Characters (DEC-031, DEC-018)

Every politician is a character: `Politician::character` names its record in `PoliticsState::characters`. A politician's identity, values, ambition, integrity, oratory, and negotiation stay on `Politician` (elections and councils read them there); the character record adds warmth, boldness, loyalty, 0–3 traits, administration, organising, intrigue, an expertise issue, a first impression, and memory. Named people (from the occupation registry) hold all of it in `Character::named`. Read anyone through `characters::profile`.

- **Tiers** are derived (`characters::tier_of`): politicians in office or standing, organisation leaders (`leader: true`), and a town's broker and rival are Principal (20 memories); other stored people are Named (5). Ambient people are generated on demand (`characters::ambient::ambient_at`, keyed by seed, place, week, and slot) and never stored; `PlayerCommand::Meet { slot }` promotes one to Named.
- **Named people** are generated once, when play begins (`characters::seed_named`): for each place, each occupation found there by its `chance`, keyed by place and occupation id. Their values start from their home neighbourhood's cohort for their age and a class drawn from `class`, plus noise. They update lazily, one keyed step per year (`characters::touch`, `caught_up`), so the result does not depend on when they are evaluated.
- **Relationships** are directed edges (`(holder, other)` in a `BTreeMap`): opinion, trust, bonds. With no edge, the opinion is the holder's first impression. Change them with `characters::change_opinion` (scaled by traits' `opinion_change` and warmth), `bond`, `owe`, and `repay`.
- **Memory**: `characters::remember(pol, content, date, holder, actor, kind, subject, opinion)` records one. Each kind has a row in `characters.memories` (sentiment, magnitude, decay per year, default opinion and trust change, tags) and a Fluent key `memory-<kind>` (dashes). Salience is `magnitude × e^(−decay × years)`, with the holder's traits' `memory_decay` applied when recorded; memories below `characters.memory.forget_below` are forgotten monthly (their opinion change stays). Adding a memory kind is a code extension point (below).
- Other domains ask for memories with `Effect::Memory` (applied daily in the Characters phase), as policy procedures do for a mayor's signature or veto.

Example trait and occupation:

```jsonc
{ "id": "trait.grudge_holder", "name_key": "trait-grudge_holder", "chance": 0.12,
  "personality": { "warmth": -0.1 },
  "memory_decay": { "negative": 0.3 },        // bad memories fade at 30% of the speed
  "opinion_change": { "positive": 0.8 } }     // and slow to warm up again
{ "id": "occupation.union_steward", "name_key": "occupation-union_steward",
  "places": ["union_hall"], "chance": 1.0, "leader": true, "influence": 0.8,
  "stake": { "drivers": ["industrial", "extraction"], "crises": ["crisis.plant_closure"] },
  "class": [0.6, 0.4, 0.0], "age": [34, 64],
  "competencies": { "organising": 0.25 }, "expertise": "redistribution" }
```

### Adding a routine (DEC-031 daily routines)

Where stored characters are through the day comes from `content/base/routines.jsonc` (`RoutineDef` in `mandate-content/src/routines.rs`). The engine derives it from the state and the turn's day (`mandate_sim::characters::routines`): nothing is stored, so a new routine needs no save change.

- **Whom it is for**: exactly one of `post` (a role word from the constitution: a body's `member_key` or an office's id; validation refuses anything else, and code never names an office), `occupation` (an occupation id), or `role` (`broker`, `rival`, `politician` for people out of office, `anyone`). A person's routines are tried in this order: broker or rival, their post's, their occupation's, `politician`, `anyone`; then their workplace, then their home street.
- **Rules** are tried in order. Each has optional `parts` (`morning`, `afternoon`, `evening`, `late_night`: the player's three standard slots, then the late slot once they are spent), `days` (the weekday of the turn's representative day, which moves on one day each turn), `seasons`, `events` (all of them) and `unless` (none of them): `session` (the jurisdiction's legislature sits: a law-making stage this turn, or its regular sitting day), `vote`, `campaign`, `meeting_night` (the town's meeting weekday). A `chance` is a keyed draw per person, day and part. The first rule that applies and names somewhere that exists picks a spot by weight.
- **Spots** (`at[].to`): `works` (a post's workplace, from its `works_at`, or the person's own place), `home_streets`, `ward_streets` (their seat's district), `legislature` (where their jurisdiction's legislature sits), or any place kind (`diner`, `market`, …: the one in their neighbourhood, else one kept for them). Spots the town lacks are skipped; a rule with none left does not apply, so nobody is placed where their town has nothing.
- **Check a pack** with `mandate-cli routines --seed N --days D [--text] [--all]`: every turn's representative day, the events going on, and where the town's key people are in each part.

Example:

```jsonc
{ "id": "routine.journalist", "occupation": "occupation.journalist",
  "rules": [
    { "parts": ["morning", "afternoon"], "events": ["vote"], "chance": 0.8, "at": [{ "to": "legislature" }] },
    { "parts": ["morning"], "at": [{ "to": "works" }] },
    { "parts": ["afternoon"], "at": [{ "to": "works", "weight": 3 }, { "to": "diner", "weight": 1 }] },
    { "at": [{ "to": "home_streets" }] } ] }
```

Who is where is asked through `characters::routines` (`whereabouts`, `locate`, `present`) or `characters::ambient::stored_at`; a place's *regulars* (first-visit introductions, grants) stay `ambient::regulars_at`. The player's ledger notes sightings (`KnowledgeLedger::seen`, on arriving and when talking); two at the same place and part on different turns make a habit, shown as the lead `lead-usually-at`.

### Adding a verb to a place (places with purpose, DEC-079 note)

What the player can do at a place beyond the coded actions is one generic action, `visit` (`Action::Visit { verb }`), whose verbs are data. A verb is defined once in `content/base/actions.jsonc` under `visit.verbs` (`VisitVerbDef` in `mandate-content/src/visits.rs`) and offered by every place kind that lists its id in `places.jsonc` `verbs`. Adding one is content: no code, no save change.

- **Cost:** `time` (`slot`, or `half`: half a slot, sharing the conversation's half-slot bank, so two halves fill one slot) and `health`; `late_night: true` also allows the late slot.
- **When:** `parts` (`morning`, `afternoon`, `evening`; none: any standard part), `when` (conditions of the one condition language that must all hold, slot-free, with the place as `here`: see "Conditions" below), and `with` (someone of one of these occupations is here now, by their routines). `parts` and `with` are short spellings of the `day_part` and `present` conditions, and the engine evaluates them as those. A closed place offers nothing.
- **Effects** (small; a visit is depth, not a campaign): `learn` (knowledge grants with `which: here`, the place: `people_at`, `hood_concerns`, `broker`, and the visit subjects `sector`, the place kind's industry in its town, `site`, its pollution and hazard exposure, and `service`, its neighbourhood's felt level of the concerns the kind `addresses`; a numeric subject granted `detailed` is a dated reading), `people` (who is here now, met to a level, with an opinion change), `awareness` and `approval` (`among`: `reach` `hood` or `town`, `classes`, `ages` 0–3, `communities`).
- **Risk:** `base`, `terms` (a condition and what it adds), and `shedding` × how fast the place's industry is letting people go (`visit.outlook`), within `[min, max]`. The place panel shows `mandate_sim::visits::risk`, and the command rolls exactly that (a keyed draw per turn, place and verb, `visits::goes_wrong`). A bad roll adds the risk's `effects` and its `outcome_key` replaces the verb's.
- **The place kind** names its `sector` (a sector id) for `sector` grants and the risk's `shedding`, and the concerns it `addresses` for `service` grants; validation refuses a verb the kind cannot support.
- **Text:** the verb's `name_key` and `outcome_key` (args `place`, `kind`, `outlook`, `sector_key`, `name` for the first person met, `learned`, `met`), and `action-verb-<name>` for the log (underscores as dashes).
- **People to meet:** an occupation in `occupations.jsonc` with the place kind in `places`, and a routine for it. Occupations added after a release take `"round": 1` (or higher): their people are seeded after everyone of earlier rounds, so the existing cast keeps its ids.

Example (the farm's verb):

```jsonc
{ "id": "verb.walk_fields", "name_key": "verb-walk_fields", "outcome_key": "outcome-verb-walk_fields",
  "time": "slot", "health": 1, "parts": ["morning", "afternoon"],
  "effects": [
    { "type": "people", "level": "known", "opinion": 2 },
    { "type": "learn", "grants": [
      { "subject": "hood_concerns", "which": "here", "level": "known" },
      { "subject": "sector", "which": "here", "level": "detailed" } ] },
    { "type": "awareness", "amount": 0.03 } ] }
```

The panel lists a place's verbs (`PlacePanelView.verbs`: cost in half slots, whether it can be done now and why not, the risk), the town's spots name them (`SpotView.verbs`), and the bots use them through the same command: the coalition builder visits to learn and meet people while it explores (`bots.rs` `learning_visit`), and the campaigner takes a photo op at a ward works between elections when the shown risk is at most 15% (`photo_op`).

### Adding a directive, an interrupt or an ending (the act flow, DEC-012, DEC-006)

Time skips ("Term in review") and act endings are two content files, `content/base/act_flow.jsonc` (`ActFlowParams` in `mandate-content/src/act.rs`) and `content/base/endings.jsonc` (`EndingDef`). Nothing in either names an office or a body (DEC-098): posts are read through the constitution, so a regional or national act needs only data. Text lives in `content/base/locale/en/act.ftl`.

- **A directive** steers one *lever* of the player-proxy (`mandate_sim::proxy`): `focus` (the worry the ground game and new bills aim at: an option's `concern`, none for the strongest the player knows; `concern_options: true` adds one option per concern), `vote` (an option's `vote`: `with_party`, `conviction`, `ward`), the effort levers `ground`, `fundraising`, `press`, `legislation` (an option's `weight`, 0–3: its share of the turn's slots) and `caution` (`max_risk`, the largest shown chance a hidden act comes out that the proxy accepts, 0 avoids every shady option; `rest_below`, the health below which it rests). One directive per lever; `default` is the option a run starts with. The player's choices are `State::act.directives` (`Command::SetDirectives`). A new *lever* is code: a `DirectiveLever` variant, its reading in `skip::settings` and its use in `proxy::ProxyTurn`.
- **An interrupt** stops a skip when its condition *newly* holds (what was already so when the skip began does not fire): `track` (a track, or `mandate`, crossing `threshold`; `below` for falling), `decision` (a decision opens with one of `weights`, or `about` a story or a case), `scandal` (a story about the player at least `min_severity`), `election` (the player is a candidate in a race), `sponsored_vote` (a bill the player sponsors reaches a stage within `days`), `request_due` (a request raised by someone's goal, or a storyline's turn, due within `days`; the asks layer adds its own), `status` (prison, release, the end). Order matters: the first that fires names the stop. `key` is the message (arguments: `value`, `track`, `days`, `decision`, `template`, `policy`, `status`). A new *kind* is code: an `InterruptWhen` variant and its keys in `skip::interrupt_keys`.
- **Skip targets** are an engine contract (`next_session`, `campaign`, `election` (the eve of election week), `end_of_term` (through the vote)); `targets` lists the ones offered and their names. `max_turns` caps one skip; `proxy` sets which decision weights the proxy may answer (`answers`; climaxes and anything about a story or a case stay the player's), its command cap and the share of its time it leaves unused (`idle_share`, which keeps a skip below engaged play: `bots`' `proxy_not_optimal` gate).
- **An ending** is one entry in `endings.jsonc`; the first whose `when` holds ends the act and closes its Legacy record. Conditions: `holds_later_act` (a post of a later act of the constitution's `career.acts`), `won_post_with` (a post of this act with all of these `powers`, not held when the act began: a tier's executive), `lost_seat`, `event` (`deselected`, `removed_from_office`, `imprisoned`), `status` (`imprisoned`, `ended`), `track_at_most`, `served` (`terms` elections won to the post held, the last term at its end: the post's `term_years` and campaign in the constitution), `any`. `acts` limits it to some of the career's acts. `facts` are what the epilogue reports (`posts_held`, `years`, `laws_passed`, `works` (laws in `legacy.works_categories`), `elections_won`, `elections_lost`, `promises_kept`, `promises_broken`, `decisions`, `scandals`, `mandate`, `standing`, `viability`); `text_key` gets them as arguments. `tone` is presentation only. A new *condition* is code: an `EndingWhen` variant and its test in `endings::holds`.
- **The Legacy record** (`mandate_sim::legacy`, `State::act.records`) is read from what happened at the end of every turn, played or skipped: posts taken and left, elections, laws and failed bills of the player's, notable decisions (`legacy.notable` weights), promises kept and broken (people's memories of the player), scandals and the courts. It is append-only and a record only: no unlocks (DEC-100).
- **Check** with `content validate`, `mandate-cli skip-determinism`, `mandate-cli act-length --seeds 8` (REQ-008: turns per chapter and per act, with and without skips, and minutes at `--minutes-per-turn`) and `bots --seeds 16` (`directed`, the proxy alone, must stay at or below the median informed style). Tests: `crates/mandate-save/tests/act_flow.rs`.

Example (an ending for a disgrace in any act, and an interrupt when Mandate falls):

```jsonc
{ "id": "ending.disgraced", "title_key": "ending-disgraced", "text_key": "ending-disgraced-text", "tone": "defeat",
  "when": { "type": "track_at_most", "track": "standing", "value": 0 },
  "facts": ["years", "posts_held", "laws_passed", "scandals", "standing"] }

{ "id": "interrupt.mandate", "key": "interrupt-mandate", "when": { "type": "track", "track": "mandate", "threshold": 25, "below": true } }
```

### Town character (DEC-077 §1)

Each month (and on load when the crisis registry changed) every town gets a `TownCharacter` in `PoliticsState::characters.towns`:

- **Defining industry:** the sector with the largest share × location quotient (share squared over its national share); public employment is left out.
- **Crises:** each crisis kind reads its indicator; it begins at `begin`, ends below `end`, and its severity runs from 0 at `end` to 1 at `severe`. All crises in progress are kept, with their start date and up to `characters.town.trend_months` of monthly severity; `TownCharacter::crisis()` is the most severe (`ActiveCrisis::trend()` is its change over the history).
- **Power broker:** the organisation leader whose `stake` matches the defining industry's driver or the live crisis, by `influence + broker_stake × matches`, else the most senior officeholder (the mayor counts double).
- **Rival:** whoever competes with the town's leading figure (the player in their town, by their filing; elsewhere the mayor) for the next seat: the strongest opponent in a running race, else the seat's holder, else last time's runner-up, else the most ambitious officeholder of another party. Filing recomputes it at once.
- **Priorities:** the town's top three concerns.

NPC agendas follow it: when an NPC picks its next bill, a template listed in the live crisis's `policies` ranks `npc.crisis_agenda` × severity higher, and one aligned with the town's concerns `npc.concern_agenda` × fit (`npc::agenda_bonus`). The bonus changes which idea comes first, not how keen the NPC is to legislate.

Example crisis kind:

```jsonc
{ "id": "crisis.plant_closure", "name_key": "crisis-plant_closure",
  "indicator": "job_loss", "begin": 0.025, "end": 0.01, "severe": 0.08,
  "concern": "concern.jobs", "policies": ["policy.small_business_grants", "policy.hardship_fund"] }
```

### Hooks for the director (DEC-033)

Read-only queries the narrative layer can build on:

| Question | Call |
|---|---|
| What A remembers about B, most salient first | `characters::salient_memories(pol, a, Some(b), date, n)` |
| How A's opinion of B moved since a date | `characters::relationship_delta(pol, a, b, since)` |
| Opinion, trust, bonds, favours between two people | `characters::opinion`, `trust`, `bonds`, `favours_owed`, `favour_balance` |
| Anyone's identity, personality, values, competencies | `characters::profile(pol, world, id)` |
| A town's live crisis, its severity and trend | `pol.characters.towns[m].crisis()`, `ActiveCrisis::{severity, history, trend()}` |
| A town's broker, rival, industry, priorities | `TownCharacter::{broker, rival, industry, priorities}` |
| Who is at a place | `characters::ambient::{stored_at, ambient_at}` |

### The narrative director (DEC-033, DEC-077 §4, DEC-032, DEC-035)

`mandate_sim::narrative` is its own domain (`State::narrative`). Its systems (`narrative.fairness`, daily; `narrative.director`, weekly, in the Director phase) write only that domain; every draw is keyed (`Stream::Director`). It never changes an endogenous outcome:

- **Spotlight.** Each week the director casts every dilemma's roles from live actors in the player's town (`narrative::cast`), checks its triggers on live state, scores relevance (`director.relevance`), and spotlights at most one decision (paced along the chapter's tension curve, within the chapter's budgets, `spacing_days` apart, at most `max_pending` at once), one request or storyline beat, and news of the town. Chapters are a campaign (its climax is the election) or a session (a budget year; its climax is the best grounded situation in `tension.climax_window`).
- **Decisions change the world only when answered**: `PlayerCommand::Answer` (from anywhere: the pocket lists them, DEC-079), or the default when the deadline passes (`narrative::resolve_due`, at the start of the turn that would pass it). Effects are real engine changes; the aftermath records what each did (`Applied`).
- **Exogenous levers** (`Chance`): a hold on hazards in the player's town while it carries `crisis_cap` crises (environment defers the strike, at its drawn severity, at most `chance.hazard_max_defer_days`), a thinner hazard rate (Steady), a hold on the onset of a global recession (economy, at most `chance.recession_hold_months`), and discovery dates of hidden facts within their window. The owning systems log every intervention (`EventKind::DirectorIntervention`). The levers read the simulation alone, never the spotlight (tested).
- **Fairness (DEC-006).** A track in danger (`director.fairness`) gets a pinned warning; `NarrativeState::ending_allowed(content, track, date)` is the gate the ending systems must ask (a warning, then `response_days`).
- **Facts log** (`NarrativeState::facts`): every spotlit and unspotlit fact with its actors and a newsworthiness; `HiddenFact` (every borderline or illegal act, the player's and NPCs') with its fate, an evidence window, and the director's discovery date (`FactKind::Discoverable`). The media domain reads them (below).

Lab switches: `narrative.spotlight = false` (chance shaping unchanged, nothing spotlighted) and `narrative.resolve_defaults = false` (past-deadline decisions lapse without consequence). `mandate-cli story --seed N --years Y --strategy S [--profile P]` prints a career's decisions as Fluent keys and arguments.

**A dilemma**, as content:

```jsonc
{ "id": "dilemma.pollution", "category": "significant",
  "title_key": "dilemma-pollution", "situation_key": "dilemma-pollution-situation",
  "roles": [
    { "id": "owner", "cast": { "type": "occupation", "occupations": ["occupation.plant_manager"] } },
    { "id": "voice", "cast": { "type": "occupation", "occupations": ["occupation.organiser", "occupation.teacher"] } } ],
  "triggers": [ { "type": "concern", "concern": "concern.environment", "top": 3 } ],
  "deadline_days": 35, "cooldown_days": 400, "scene": "scene.pollution",
  "options": [
    { "id": "inspection", "label_key": "…", "aftermath_key": "…", "effects": [
        { "type": "opinion", "role": "owner", "delta": -12, "memory": "stood_against_me" },
        { "type": "opinion", "role": "voice", "delta": 12, "memory": "stood_with_me" },
        { "type": "approval", "classes": [0], "delta": -0.01 } ] },
    { "id": "wait_for_data", "default": true, "label_key": "…", "aftermath_key": "…", "effects": [
        { "type": "opinion", "role": "voice", "delta": -8, "memory": "ignored_me" },
        { "type": "opinion", "role": "owner", "delta": 3 } ] } ] }
```

Casts: `broker`, `rival` (in a campaign, whoever stands against the player), `mayor`, `occupation`, `colleague`, `creditor`, `bill` (optionally `sponsored_by` a cast role), `sponsor_of`, `ward_neighborhood`, `law` (optionally `building`), `goal_holder`. Conditions: the one condition language ("Conditions" below), with the cast's roles as its slots (`slot`, or the older spelling `role`). Effects: `opinion` (with a memory kind), `approval`, `awareness`, `money`, `health`, `slots`, `standing`, `legal_jeopardy`, `record`, `favour`, `repay`, `vote` (`yes`, `no`, `abstain`, `with_party`, `with_ward`; the council-vote rules apply), `draft` (with `params`). Fluent arguments: each role as `$<role>` (a bill or law by its policy name), a person's role as `$<role>_role`, and `$town`, `$crisis`, `$player`. Each new cast, condition, or effect is a code extension point (below).

**Goals** (`characters::goals`, politics domain): each month the people of the player's town take up goals whose conditions hold and give them up when they fail or after `max_days`. Drives: `oppose_player` and `back_player` add to their council lean on the player's bills (like lobbying sway) and an opposer lobbies against them; `champion` ranks bills that answer the live crisis higher. A goal with a `request` surfaces it (once per adoption). Goals are kept for the player's town only (NPC goals elsewhere are deferred).

**Storylines**: a run casts its roles once; if a person no longer fills their role (a rival stays cast while they stand against the player), the arc fades; stage decisions take the run's cast for shared role ids and cast the rest; a stage decision has priority over standalone dilemmas (it waits for room, not spacing). `follow_up` starts the same arc again later with the same cast where it still fits.

**Scenes**: a decision's scene happens at the place resolved when it opens (`Decision::place`). The pocket and the sheet always list it with every option; at the place, the conversation appears, and a speaker with `quote_fallback` quotes their most salient memory of the player (`salient_memories`).

**Storyteller profiles**: Classic (default, `director.default_profile`), Unfiltered (no levers, no tension gating), Steady (30% of strikes on the player's town miss, one crisis at a time, three quarters of the budgets). A new game can set `narrative.profile`.

### Adding an ask (asks and promises, DEC-097 §2)

An ask is a concrete request with a deadline, made to the player by someone with a reason in the live state; a campaign promise is a public ask from a constituency's voters. The engine is `mandate_sim::asks` (its own domain, `State.asks`); the content is `content/base/asks/`: `params.jsonc` (`ask_params`: the sliding rate, caps, casework, promises), `asks.jsonc` (`asks`) and `promises.jsonc` (`promises`); the text is `locale/en/asks.ftl`. Types: `mandate-content/src/asks.rs`.

```jsonc
{ "id": "ask.say_something", "name_key": "ask-say-something",
  "asker": { "type": "occupation", "occupations": ["occupation.journalist"] },   // a person, cast like a dilemma role: the role `asker`
  "triggers": [ { "type": "crisis", "min_severity": 0.1 } ],                     // the one condition language; never `knows`
  "needs": [ { "type": "statement", "about": { "type": "crisis" } } ],          // met in order
  "deadline_days": [7, 14], "cooldown_days": 120, "public": true,
  "keep":    [ { "type": "opinion", "role": "asker", "delta": 6, "memory": "kept_promise" },
               { "type": "approval", "scope": "town", "delta": 0.004 } ],
  "break":   [ { "type": "opinion", "role": "asker", "delta": -10, "memory": "broke_promise" } ],
  "decline": [ { "type": "opinion", "role": "asker", "delta": -3 } ],
  "detail_key": "ask-say-something-detail", "need_key": "ask-say-something-need",
  "kept_key": "ask-say-something-kept", "broken_key": "ask-say-something-broken" }
```

- **Who asks.** `asker` is any dilemma cast (`occupation`, `broker`, `colleague` — a holder of a post in the same body —, `office` — the jurisdiction's executive, or `{ "office": "<constitution id>" }` —, `goal_holder` for an ask an NPC goal names with `"ask": "ask.…"` in `goals.jsonc`). Never write an office or body word: the test `asks_name_no_town_key_and_no_office` scans the asks' content and text. `on_behalf: "party"` makes it the party's ask (one at a time). The asker's home neighbourhood is the implicit role `asker_hood`; further roles go in `roles`.
- **Where offers come from** (`from`): `week` (default; the weekly draw at the sliding rate), `doorstep` (a canvass: someone living in the canvassed neighbourhood, `doorstep_chance`), `talk` (the person the player talked with), `talked_about` (someone the player asked around about within `talked_about_days`). `min_career_days` holds an ask back until later in a career; `fresh_days` skips it while the player did its first need lately ("no town hall there in 180 days").
- **Needs** (each an ordinary command the engine sees, DEC-032: no fetch quests): `vote` (`yes`, `no`, `with_party`, on a `bill` role; without `deadline_days` the ask is due by the vote), `statement` (about `crisis`, a `concern`, a `bill` role, or `any`), `town_hall` (a neighbourhood role or a person's home; `constituency: true` for every neighbourhood of a promise's race), `canvass` (`or_town_hall`), `visit` (place kinds, near a role), `talk`, `introduce` and `enacted` (bills: `templates`, `crisis_answer`, `concern_answer`, or a promise's `chosen` answer), `casework` (the one new verb, "Sort it out" at the town hall), and the needs kept by not acting, which stand alone: `against_all` (`tax_rise`), `no_surfaced_hidden_fact`, `quiet`.
- **Effects** reuse the narrative effect list, limited to what the asks system can request from other domains: `opinion` (with a memory), `approval` (a neighbourhood role, the ward — a promise's race —, or the town), `standing`, `record`, `favour`, `learn`, `money`. Validation demands a gain on keeping and a cost on breaking (DEC-073); the record moves too (`career.record_gain`: `ask_kept`, `promise_kept`, `ask_broken`), and a kept `big` ask leaves the asker owing the player a favour. Accepting a `big` ask ends the director's relax phase.
- **Text.** Five keys per ask. Arguments: each role as `$<role>` (a bill by its policy name), `$<role>_role`, `$<role>_occupation` (the occupation id without its prefix, for variants: `ask-casework` reads a drain, a form, an objection or a school place from it), `$history` (`kept`, `broken`, `none`: how the asker's last ask ended, for callbacks), `$days`, `$place`, `$crisis`, `$concern`, `$town`; promises add `$answer` and `$constituency`. Validation checks each key exists and uses only arguments the ask provides; the views build them with `mandate_protocol::narrative::ask_args`.
- **A talk need needs a `choice`.** The validator refuses a `talk` need without one: the button's `label_key`, what the player says (`say_key`), and `replies`, each a `key`, optional `when` conditions on the slots `speaker` and `player`, and `agrees` (default true). The first reply whose conditions hold is said; a reply with `agrees: false` leaves the need open. The last reply must have no conditions, so there is always one, and at least one reply must agree. Keys take `$speaker`, `$player`, `$town`, each role as `$<role>` and a bill role as `$<role>_key`; see the `second_my_bill` ask in `asks/asks.jsonc` and `TalkChoiceDef` in `mandate-content/src/asks.rs`.
- **Promises** name what they answer (`answers`: `crisis`, a `concern`, `any`) and when they fall due (`due`: `term` of the post run for, `budget_year`, `days`). The menu (`asks::promise_menu`) offers, while campaigning, each answer to the town's live crisis, the promises on the race's three most felt worries, and those that answer none.

Read API for other systems: `AskState::due_within(date, days)` (the time skip's `request_due` interrupt reads the accepted ones), `promise_outcomes()` (the Legacy record), `deeds_since(date)` (the player's deed log), `history_of(asker)`. Lab: `bots --careers Y` prints each career's `asks` tally and per-template counts.

### Adding an arc (staged arcs, DEC-109)

An arc is one matter in one jurisdiction that unfolds in stages: a cast bound to real people and places, a clock, a heat, marks, and exits with consequences. Design: `docs/design/STAGED_ARCS.md`. The engine is `mandate_sim::arcs` (its own domain, `State.arcs`); the content is one file an arc, `content/base/arcs/<name>.jsonc` (kind `arcs`), which also carries the dilemmas, asks, scenes and overheard lines its beats name (`dilemmas`, `asks`, `scenes`, `overheard`: they join the same registries, after every other file's items, so an arc never shifts the registry order that breaks ties and keys draws); the text is `locale/en/arcs.ftl`. Types: `mandate-content/src/arcs.rs`. The worked example is `arcs/flood.jsonc`.

- **Transitions read; beats write.** A stage's `next` edges fire, in order, on conditions over simulated state, the run's marks, heat and days (`arc_mark`, `arc_heat`, `arc_days`), `after_days` in the stage, and `clock_within_days`. Nothing a player picks names a stage: a beat sets a mark (`sets`) and posts effects, and the edges read the result. Write every stage a drift edge (what happens when nobody acts) and a lever edge (a mark or a state change an act produces). The last edge of every stage is on time alone; validation refuses a stage that could last for ever.
- **Raise and cast.** `raise` is conditions at a jurisdiction, never the ledger. `roles` bind in order, relative to the arc's jurisdiction: `occupation` (a named person there; `near` an area role), `post` (whoever holds a post with a power: never an office's name), `area` (one level down: `most_exposed` to a hazard, `least_pleased`), `place` (of a kind, `near` an area), `jurisdiction` (`parent`). A role may be `optional`: beats and effects naming an empty role are skipped. `given` are conditions on the cast as bound (a flood that sent nobody from their home raises no arc). `tiers` lists where it may run, and every condition is checked against each: a local-only condition (`disaster`, `law_in_force`) keeps an arc local.
- **Beats** (seven kinds, each on a system that exists):
  - `decision`: a dilemma with `storyline_only: true` whose roles are roles of the arc (people and areas); `on` maps an option to a mark, a heat change and effects. A default at the deadline counts.
  - `ask`: an ask template with `"from": ["arc"]`, offered from a cast person; the beat is done when the ask is kept.
  - `talk`: a `choice` in a conversation with a cast person (a talk hook): replies with conditions on `speaker`, `player`, the arc's roles and its marks; one that `agrees` does the beat.
  - `visit`: a verb at a cast place or on the streets of a cast neighbourhood. Its `verb_key` is a button: no arguments.
  - `press`: a headline the town's outlets may run. `heard`: an overheard line whose own conditions name the arc and the stage (`{ "type": "arc", "arc": …, "stages": […], "here": "<role>" }`). `move`: a cast person acts, a keyed weekly draw at `weight`, likelier for the bold and for someone who thinks little of the player.
  - A beat is offered while its stage lasts, its roles are cast and its `when` holds; `once` (default) means once a run; `relief` lowers heat when it is done.
- **Effects** (`effects`, a stage's `enter`, an exit's): `opinion` (with a memory), `approval` (an area role), `favour`, `record`, `standing`, `recovery` (repairs in an area run `repair` times as fast for `months`), `displacement` (those still away leave for good), `heat` (the run's own). Each may be gated with `when` and `unless`. They are requests to the owning systems; an exit needs at least one, and at least three exits is the rule.
- **Text** by kind: `arc-*` (a stage's story so far, an exit's ending: one or two sentences, present tense, a fact and a date), `headline-arc-*`, `visit-arc-*`, `outcome-arc-*`, `talk-arc-*`, `skip-item-arc-*` (a move), `legacy-arc-*`. Arguments: each role as `$<role>` (and a person's `$<role>_role` in what the view renders: stages, headlines, moves), `$place`, `$day` and `$month` (when it began), `$weeks`, `$clock_day`, and the arguments of the arc's `facts` (conditions evaluated for their arguments: `displaced` gives `$away`, `recovered` gives `$homes` and `$repaired`). Validation checks every key exists and uses only what the arc provides. Numbers arrive as strings: write so that 0 and 1 read, or make a condition (`given`, a beat's `when`) rule them out.
- **Not yet.** `mandate_content::arcs::not_yet` lists what a template uses that the engine does not do yet (a `subject` cast, a clock read from the subject, `gov_approval`, `place_state`, any `requires`): such a template validates and is never raised. `inert` lists what is parsed for later tasks (`hidden`, `climax`, `goals`, `teaches`, `sides`, `answers`, `opportunity`, `follow_up`, `legacy_key`).
- **Check it.** `content validate` (the graph, the marks, the roles, the keys), then play it: `mandate-worldgen/tests/arcs.rs` plants a flood and walks a passive and an engaged script; `arcs::beats::offers(state, content)` lists what a player can do now as the command that does it.

### Media, scandals, and the legal path (DEC-034, DEC-006)

Two domains: `State::media` (`mandate_sim::media`: outlets, knowledge, stories, scandals, rumours) and `State::justice` (`mandate_sim::justice`: cases and bars from office). Both run only while play is on (outlets exist), never in the history warm-up.

- **Four layers.** *Facts*: the narrative's facts log and its hidden facts (`NarrativeState::hidden`). *Knowledge*: who uncovered which hidden fact with what evidence (`MediaState::knowledge`). *Coverage*: the stories outlets run (`MediaState::stories`), each with a `StorySource` that traces it to a fact (`media::audit_story` is the test's check). *Perception*: only coverage moves beliefs. A story about the player moves cohort approval and awareness by the outlet's audience weights (`Effect::Perceive`, applied by population); a story about anyone moves their scandal valence in elections (`Effect::Valence`, politics); colleagues and the discoverer remember (`Effect::Remember`); the party's Standing pays for a breaking story (`Effect::PlayerTrack`).
- **Hidden facts.** A borderline or illegal act calls `media::hide(state, content, spec, subject, actors)`: its exposure is evidence × each discoverer's interest (by severity, `media.discovery.interest`) × access (journalists in town, a rival, auditors, investigators), its fate (whether, and to whom) is a keyed draw settled at once, and its window opens after an evidence-dependent delay (`window_strong`, `window_weak`). The director then picks the date inside the window (`narrative::director::discovery`, shaped by the chapter's tension; `narrative.shape_discovery = false` makes it uniform). The director never touches the fate (tested on and off).
- **Scandals and coverage.** On its date a fated fact becomes a `Scandal` (heat 1, fading by `coverage.heat_keep` a week). Each week each outlet scores candidates (scandals, case developments, rumours, corrections, statements, and neutral town news for the town paper) by newsworthiness × interest (more when its lean is far from the subject's views) × access (a regional or national outlet picks up only serious affairs), runs the best few (`stories_per_week` × budget), and frames each by its hostility. The player's press statement is a `Statement` the outlets may cover (sympathetically or not); an ad buy cools a running scandal a little.
- **Responses.** A scandal about the player raises `dilemma.scandal_response` (deny, explain, apologise, counter-attack, or nothing); an investigation raises `dilemma.legal_response` (cooperate, a lawyer, stonewall, blame an ally, or wait). Raised decisions (`raised_by`) are fairness first: never budgeted, spaced, or capped. NPCs answer by personality.
- **The legal path.** Prosecutors consider each scandal once (`justice.open` × evidence, after `open_days`); an investigation lasts `investigation_days` and ends in charges (`justice.charge`, adjusted by the answer) or is dropped; a trial comes `trial_days` later (at least the fairness response time): conviction is `logistic(convict_base + convict_evidence × evidence − lawyer + convict_mood × (0.5 − mood))`. Sentences: a fine (ethics), a fine or removal (misconduct), prison years (crime) or disqualification. A sentence of `long_years` or more, or a disqualification, ends the player's career, and only once `NarrativeState::ending_allowed("legal_jeopardy")` holds; otherwise the trial is adjourned (the same keyed draws later). Legal Jeopardy shows the case (`track_investigation`, `track_charged`) and decays without one.
- **Prison and the end of a run.** `PlayerState.status` is `active`, `imprisoned { until }`, or `ended`. In prison every player command is refused and ending the turn serves the sentence (turn by turn, NPCs acting, until release); out, the player is out of office with Standing and viability capped (`release_standing`, `release_viability`) and may file to run again. An ended run refuses commands and turns; the client shows it plainly (the Legacy screen is M6).
- **NPCs (DEC-030).** Officeholders are tempted each turn by personality (`media.temptation`: chance × (1 − integrity)^power × (base + ambition)); their acts (`media.npc_acts`) go through `media::hide` like the player's, and their cases through the same stages and odds. A removal from office is filled by the member's party (`elections::appoint_replacement`); a barred politician cannot hold office.
- **Rumours.** Leaking a rival's file (the newspaper's shady action) exposes one of the rival's unknown hidden facts at once if there is one; otherwise an unverified claim spreads on social platforms for `rumours.spread_weeks`, and a credible outlet may correct it (a correction gives back `correction_share` of the harm). The leak is itself a hidden fact of the leaker.

**Shady actions** are `take_bribe` (the diner, late at night only), `dip_funds` (the party office), and `leak_file` (the newspaper office). Each is a `media.shady_actions` entry (payoff `money`, `health`, `legal`, and its `hidden` spec), a `tuning.action_costs` entry, and a place that offers it. The offer view carries its risk line: the chance it comes out within `discovery.horizon_days`, as a band (`bands`: low, real, high) and the severity.

```jsonc
// An option in a dilemma: a real payoff now, a hidden fact that may come out.
{ "id": "take_fee", "label_key": "dilemma-consultancy-fee-take", "aftermath_key": "dilemma-consultancy-fee-take-after",
  "effects": [
    { "type": "money", "amount": 4000 },
    { "type": "favour", "role": "developer" },
    { "type": "hidden", "kind": "consultancy_fee", "severity": "crime", "evidence": 0.45, "with_role": "developer" } ] }
```

Lab switches: `narrative.shape_discovery = false` (uniform discovery dates). `mandate-cli bots --careers 8` prints the career distribution (elections, scandals, cases, prison, careers ended) of the `corrupt` and `mixed` strategies.

### Scenarios (DEC-078 decision 4)

A scenario only says what makes its world different. Anything omitted is drawn exactly as for a random country (a fixed value makes no draw).

| Block | Fields |
|---|---|
| `geography` | `sea_sides` (1–3), `mountain_strength`, `country_extent`, `wetness`, `temp_south`, `mineral_richness`, `population` (multiplier), `industrial_towns` |
| `economy` | `endowment` (sector driver → multiplier), `wealth`, `class_mix` ([low, middle, high] multipliers) |
| `era` | `tech_growth`, `manufacturing_trend` (defaults in `economy.jsonc`), `drying`, `storminess` |
| `politics` | `attitude_shift` (attitude key → added to seeded opinions) |

### Adding a district type / terrain feature / connector (DEC-090 D4)

Town layout v2 draws each town from content registries (TOWN_GENERATION.md §6). They are presentation data: the engine validates them on load and sends them to the client as `LayoutContentView` (query `layout_content`), but never reads them, so they cannot change a simulation result. Types: `crates/mandate-content/src/layout.rs`.

| Add | Where (`kind`) | Required | Validation |
|---|---|---|---|
| District type | `town_districts` (base: `content/base/layout/town_districts.jsonc`) | `id`, `name_key` (`district-<name>`), `appears` (`hood_kinds`, `weight`; optional `geo`, `population`, `town_class`, `era`, `requires_place`), `site` (`prefers`; optional `avoids`, `anchor`), `streets` (`pattern`, `block`, `depth`, `lanes`, `max_dead_ends`, `style`), `plots`, `buildings` (`mix` of `[filler kind, weight]`, `storeys`, `density`), `growth`, `art` (`family` required; `kit`, `ground`, `roof`, `walls`, `trim_ramps`, `lamps`, `props`); optional `hosts`, `tags` | Known neighbourhood kinds, siting predicates, street patterns (organic, grid, curvy, contour, yards, quay, ribbon), filler kinds and town classes; geography fields only from the list in `GeoCondition`, per-mille ranges within 0..1000; ranges in bounds (block 6..40, depth 5..20, frontage 2..12, plot depth 2..8, storeys 1..8, shares 0..1, reserve 0..0.5); `hosts` and `requires_place` name place kinds; art keys well formed. Every neighbourhood kind keeps at least one type with no condition but its kind (a pick must always exist) |
| Terrain feature | `terrain_features` (`content/base/layout/terrain_features.jsonc`) | `id`, `generator` (river, creek, coast, lake, hills, cliff, woods, marsh, beach), `art`; optional `appears.geo`, `params` | `params` are integers, integer lists or tables, or flags (the generator is integer-only); art keys well formed. The towns need a river, creek, coast and hills feature. The first feature in load order with a generator is the one used |
| Connector | `connectors` (`content/base/layout/connectors.jsonc`) | `id`, `kind` (bridge, footbridge, ford, viaduct, level_crossing), `max_span` (1..32), `roads`, `crosses`, `art.kit`; optional `town_class`, `era` | Known roads (arterial, street, lane, path, rail) and crossings (river, sea, lake, creek, rail). The towns need a bridge, a ford and a level crossing. The generator takes the first connector in load order that allows a crossing, so a pack adds a special case by sorting before `base/` |
| How a place kind is drawn | `layout` block of its `places` entry | `slot`, `footprint` (`[[min w, min h], [max w, max h]]`, 2..16), `place` (predicates in order of preference: the list in `PLACE_PREDICATES`, `toward:minerals`, `toward:woods`, `near_place:<kind>`), `landmark`, `art.family`; optional `signature`, `with`, `art.kit` | Known slots and predicates; `near_place` names a place kind. A kind without a block is drawn with a generic footprint (a warning); `home` and `streets` are not buildings and have none |

The content-only path, for example a fishing quay:

1. Add an entry like the sample in TOWN_GENERATION.md §6.2 to a `town_districts` file in your pack: when it appears, a street pattern from the list, plots, the building mix, the places it hosts, tags and art keys (existing keys, or new parts in your art pack, §7.5).
2. Add its Fluent name (`district-fishing_quay`) to your pack's English strings. (Validation checks the key's form only until the base district strings exist; they come with the town text alternative's FTL.)
3. Run `cargo run -p mandate-cli -- content validate`, then `pixelkit lint` for the art keys.
4. New runs pick the type wherever its `appears` rules match. A town already shown keeps the entries it was planned with (its recipe snapshot, §6.6 option C), so a content change never rearranges it.

When the base entries change, refresh the client's fixture (`MANDATE_UPDATE_FIXTURES=1 cargo test -p mandate-protocol layout` rewrites `web/src/hub/layoutV2/registry/base.json`) and run the web tests: the pinned plan and dress hashes in `web/src/hub/layoutV2/__snapshots__` show which committed towns moved. A new street pattern, terrain generator, placement predicate, geography condition or connector behaviour is code (TOWN_GENERATION.md §6.5): the client function, the name in the closed list in `layout.rs`, and a test.

### Lab overrides (DEC-078 decision 2)

`mandate-cli sweep` and `mandate-cli bots` take `--set <path>=<value>` (repeatable) and up to two `--grid <path>=<v1>,<v2>,…`. Overrides are applied to the content sources before loading, so schema and range validation run on the result, and the content hash changes with them.

- Single-value kinds: `--set politics.vote.party_weight=1.6`, `--set opinion.experience.unemployment=6`.
- Registry entries by id: `--set concerns[concern.jobs].level.slope=12`, `--set sectors[sector.retail].base_wage=31000`.
- Array elements: `--set actions.fundraise.raised[0]=1000` (or `.0`).
- Only values written in a content file can be overridden. An unknown path is refused with what exists there, and the new value must have the old one's JSON type.
- `--grid` prints one summary row per combination (the first parameter varies slowest): bands and drama floors for the sweep; win rates and the gates for bots. In grid mode gates are reported, not enforced.

Example: `cargo run --release -p mandate-cli -- bots --seeds 32 --grid politics.vote.salience_base=1.0,1.5,2.0`.

**Tests to add for pure content.** Use an in-memory pack: push `Source`s onto `read_dir_sources(content/)`. Follow `crates/mandate-worldgen/tests/{sectors,attitudes,communities,concerns,scenarios,characters}.rs`:

- the bundle loads, and the entry is in the registry where expected;
- a world generates, a two-year run gives the same state hash twice, and people are conserved (yearly accounting);
- the new entry has its intended effect, for example support by neighbourhood kind, or a community's location and its attitude offsets;
- bad entries give the expected validation errors;
- a save from base content loads into the extended content, and the extended save is refused by base content (`crates/mandate-save/tests/saves.rs`).

Base-content results must not change: compare the `sweep`, `bots`, and `trace-econ` output, and decoded saves, before and after the change.

### Adding a policy family (DEC-092)

A policy family is a proposal grounded in the town: "Rescue deal for {plant}", "Flood wall for {hood}". It lives in `content/<pack>/policy_families/*.jsonc` (`"kind": "policy_families"`), typed by `mandate_content::policy_family` (the schema is generated from it). The contract, with three worked examples, is `docs/design/research/M6_ENGINE_CONTRACTS.md` §2; a small working sample is `crates/mandate-content/tests/fixtures/families/`.

**What you write, field by field:**

1. `id` (`family.<name>`), `name_key`, `desc_key`; `tier` (the jurisdictions it opens in, DEC-098), `category` (the budget line) and `requires` (a constitution power, never an office name).
2. `draw`: `rarity` (`always`, `common`, `uncommon`, `rare`; the chances are in `policy_families/params.jsonc`) and `weight` (priority when several are ready, in (0, 10]); `max_instances` (default 1).
3. `slots`, bound in order inside the jurisdiction: `place` (`kinds`, `drivers` from the place kind's new `drivers`, `in_slot`, `open_only`), `neighborhood` (`kinds`, `of_slot`, `hazard` + `min_exposure`, `concern` + `top`, `lacks`), `person` (`occupations`, `at_slot`, `in_slot`, `fallback`: `broker` or `executive`), `broker`, `sector` (`of_slot`, `drivers`, `defining`), `crisis`, `hazard`. `pick` ranks candidates (`first`, `most_at_risk`, `most_exposed`, `most_concerned`, `largest`, `keyed`). A required slot that cannot bind means no proposal; mark the others `optional`.
4. `trigger` (`all` and `any`, on the one condition language; the family conditions `hood_concern`, `hood_hazard`, `sector_decline`, `place_open`, `place_closed` name a slot) and `expiry` (`when`, `grace_months`, `max_months`, `cooldown_months`). A family whose tier is not `local` uses tier-generic conditions only.
5. `base`: sliders, cost, mechanisms (`"@slot"` names the sector or hazard bound to a slot), implementation months, signature, secondary keys, side effects, and optionally `works` (a physical work at a bound place or neighbourhood: `kind`, `at`, `months`, `contractor_risk`; DEC-099).
6. `axes`: 1–3, each with 2–3 options. An option may set `area` (at most one axis per family), add sliders, `fixed` hidden values, cost, `cost_factor`, `effect_factor`, `months`, mechanisms (`borrow`, `revenue` and `sector_capacity` are for financing and structural jobs), side effects, signature, secondary keys, `excludes` and `works`. Every option needs a cost and a benefit against the others of its axis (DEC-073: validation refuses one that is identical to or better in every way than another).
7. `sponsors`: a person slot, a `preset` (an option per axis), slider values, and a `price` (`favour` or `endorsement`). Adopting a sponsor's version pleases them (`adopt_opinion`); amending away from it costs `amend_opinion` per change.
8. `knowledge`: `public`, `heard`, `known` and `detailed` hooks (`overheard`, `visit`, `meet`, `action`, `service`, `talk`, `approach`) and a `lead`. Drafting needs `known` (DEC-091).
9. Fluent keys in English (a family file of its own under `locale/en/`). Each `{ $x }` in a family's keys must be one of its slots, or `<slot>_key` for sectors, crises and hazards (`hazard-text-<kind>`).

**Changing another pack's family** without replacing it: `"kind": "policy_family_patches"` with `set_draw`, `add_option`, `remove_option`, `add_sponsor`, `remove_sponsor` or `disable`, applied in pack order. Disabling keeps bills and laws already made; open proposals of a disabled or removed family lapse on load.

**Retiring a template** into a family: set `"retired": true` on the template. It is offered to no one and drafted by no one, but laws in force continue and saves keep loading.

**Checking it:** `content validate` runs the family rules (§2.13 items 1–10). That every family binds in a canonical test world (item 11), and the `policies` lab tool, come with F5. A test that loads your family beside the base pack is the pattern of `crates/mandate-content/tests/policy_families.rs` and `crates/mandate-save/tests/policy_families.rs`.

**In the engine** (for orientation): `policy::families` draws each focus jurisdiction's deck (keyed by `Stream::Policy`, `at.tier_index()`, `at.raw()` and the family id), binds slots, opens and lapses proposals monthly (`policy.families`, Characters phase); `policy::resolve` composes a family bill from its base and chosen options; `policy::drafting` checks drafts, pays prices and fires knowledge hooks; `policy::works` builds, stalls and decays works (`works::by_hood` is the read API for the art and street layers).

### Conditions (the one condition language, DEC-098 §5)

Every condition content writes is one type, `Condition` (`mandate-content/src/condition.rs`): dilemma triggers, goal conditions, storyline starts and advances, policy unlocks, conversation topics, lines and voices, overheard lines, and visit verbs and their risk terms (asks and policy families will use it too). One evaluator decides it (`mandate_sim::condition::holds`) and one validator checks it (`condition::check`).

- **Where each variant is allowed** is a table: each use (`ConditionUse`) says what it binds (`provides`: a cast, a conversation's slots, the whole state, a place `here`, and whether it decides that something happens), and each variant says what it reads (`Condition::needs`). Validation refuses a variant where a need is not met, with the reason: `knows`, `player_knows` and `tells` where a condition decides whether something happens (DEC-091), a slot where none is bound, a conversation's reading (`voice`, `about`, `tells`, a speaker's worries) outside a conversation, a cast's bill outside a cast, the event log, routines or the clock (`plant_closure`, `present`, `day_part`) inside the simulation (dilemmas, goals, storylines), a place's reading (`dry`, `water: any`, `player_awareness`, `player_approval`, `shedding`, `present`, a worry `of: here`) where there is no place.
- **Tiers (DEC-098):** each variant is tier-generic (the player, a person, the calendar, a conversation) or local-only (it reads a town's state). A local-only variant never holds for a region or the nation, and a non-local item (a regional policy template) may use tier-generic conditions only.
- **Arguments:** each variant documents the Fluent arguments it gives when it holds (`Condition::provides`); validation checks a line's placeholders against them.
- **Spellings:** `slot` and the older `role` are one field; `live_crisis` is a `crisis` without a `kind`. A worry is one condition: `{ "type": "concern", "concern": "concern.jobs", "top": 2 }` (measured, the town's priorities, default top 3) or `{ "type": "concern", "of": "speaker", "concerns": [...] }` (felt, default top 1), with `of` (`town`, `speaker`, `here`), `top` and `by` (`felt`, `measured`) to say it either way.

| Variant | Tier | Allowed in |
|---|---|---|
| `crisis` (`live_crisis`), `indicator`, `concern` (of the town), `industry`, `disaster`, `law_in_force`, `water` (river, coast), `broker`, `rival`, `rival_record`, `scandal`, `headline` | local | everywhere |
| `plant_closure` | local | outside the simulation's systems: policy unlocks and family triggers, conversations, overheard lines, visits, scene variants |
| `dry`, `water: any`, `player_awareness`, `player_approval`, `concern of: here` | local | where there is a place: conversations, overheard lines, visits, scene variants |
| `present`, `shedding` | local | where there is a place, outside the simulation's systems (the same uses) |
| `is_broker`, `is_rival` | local | where slots are bound: dilemmas, goals, storylines, scenes, asks, conversations |
| `day_part` | generic | outside the simulation's systems |
| `phase`, `election_within`, `months_in_play`, `campaigning`, `office`, `holds_power`, `track`, `money`, `answered` (without `same_cast`), `sponsored_law` (without a slot) | generic | everywhere |
| `occupation`, `trait`, `personality`, `goal`, `opinion`, `trust`, `remembers`, `favour`, `met` | generic | where slots are bound |
| `party_line_against_ward`, `can_vote`, `answered` with `same_cast`, `sponsored_law` with a slot | generic | where a cast is bound: dilemmas, goals, storylines, scenes, asks |
| `knows` | generic | framing only: scene variants, conversations, overheard lines |
| `player_knows`, `tells` | generic | conversations only |
| `voice`, `about`, `concern of: speaker` | generic | conversations only (`voice` not in a voice's own conditions) |
| `arc` (a run of an arc is at one of these stages here) | generic | everywhere outside an arc's own conditions; with `here` (where a role of the run is), where there is a place |
| `arc_mark`, `arc_heat`, `arc_days`, `displaced`, `recovered` | generic | inside an arc only: its edges, its beats' and effects' gates, its `given` and `facts`, and the replies to its choice in a conversation |
| `disaster` with a `hazard` | local | as `disaster`; it reads the hazard's unrepaired damage, so a strike that damaged no home does not count |
| `any` | its alternatives' | each alternative where it is allowed |

#### Adding a condition

One place for each part, all in two files:

1. **The variant**, with its fields and its doc comment naming the Fluent arguments it gives: `Condition` in `mandate-content/src/condition.rs`. Add its type tag to `Condition::key`.
2. **Its tier flag:** `Condition::tier` (local-only if it reads a town's state).
3. **Where it is allowed:** what it reads in `Condition::needs` (and, for a new kind of context, a field in `Provides` and the uses that provide it in `ConditionUse::provides`).
4. **Its arguments:** `Condition::provides`.
5. **Its validation:** an arm in `condition::fields` (real ids, sane ranges), and its slots in `Condition::slots` if it names one.
6. **Its evaluation:** an arm in `mandate_sim::condition::holds` (`mandate-sim/src/condition.rs`), reading only through the context (`Ctx`: `at`, the cast or conversation, `here`, the whole state where there is one). Pure and deterministic: no stream draws; a chance is a keyed draw, as `sponsored_law`'s.
7. **Its explanation:** an arm in `mandate_sim::explain::observe` (what the evaluator read, for display).
8. **The doctor**, if it bounds a number or a discrete value (`doctor::cond_bound`, `cond_discrete`), names ids (`cond_refs`) or backs a worry (`cond_worries`).
9. **Tests:** a probe in `crates/mandate-worldgen/tests/conditions.rs` (the test fails until every variant the schema lists is evaluated), and a refusal in `crates/mandate-content/tests/conditions.rs` if the table refuses it somewhere. Regenerate the schema (`content schema`).

Conditions are content, not state: adding one needs no save change.

### Checking your content (the content workbench)

`content validate` says whether a pack is well formed. Two lab tools say whether it *works*: whether each item can come up, how often it does, and how it reads on a phone.

**`content doctor`** reports problems across the pack, with severities (error, warn, info), a summary table, coverage by kind, and findings filed under the file that defines each item:

```text
cargo run --release -p mandate-cli -- content doctor --seeds 16 --years 4
cargo run --release -p mandate-cli -- content doctor --seeds 0 --json   # static checks only, as JSON
```

- **Can never fire** (error): contradictory or impossible bounds (`min` above `max`, an opinion above 100), conditions that contradict each other in one list (`office` mayor and none, two `track` bounds that do not meet, `dry` with `water`), a line's conditions together with its voice's or its topic's, references to ids that do not exist, requests no goal raises, scenes no dilemma presents, and roles or places no generated town had (`no-town-has`, from the careers).
- **Never fired** (warn): items that did not come up in N bot-played careers (`--seeds`, `--years`, `--strategy`, default `mixed`): dilemmas, situational policies, storylines and each of their steps, scenes, overheard lines, topics, conversation lines and voices. A line whose conditions held for someone but that was never picked is an info.
- **Fires too often** (warn): in more than 60% of careers (`--often-careers`), or more than 25% of the picks of its kind and 2.5 times its fair share (`--often-share`).
- **Fluent**: keys content references but English lacks (error; the doctor stubs them so the rest can still be checked), keys the engine names but English lacks (warn, a scan of `crates/*/src`), keys nothing references (info; content, other messages, engine and UI literals and the prefixes keys are built from count as references), and arguments a message uses that its caller never supplies (dilemma, storyline and scene casts; overheard conditions).
- **Too long** (warn): titles over 60 characters, lines over 280, situations and aftermaths over 600 (`--max-title`, `--max-line`, `--max-body`), rendered with long representative names and the longest variant of every selector.
- **Voice**: a closer whose voice's speakers never have the line's roles, a substance line for roles its topic is never offered to, a voice that an unconditional voice of higher priority always beats, and (from the careers) voice and role pairs never met.
- **Text** (warn): doubled punctuation and stray braces written into a message (`text-doubled-punct`, `text-raw`), and a line that names a worry with nothing in its conditions behind it (`worry-unconditioned`); replies that do not answer what was asked (`subject-unread`, `subject-drift`). See "Writing for Mandate" below.
- **Prose** (info; warn with `--strict-prose`): `prose-habit` reads every message in every variant for habits that read as machine-written (`docs/design/STYLE_BIBLE.md` §7):
  stock phrases (`STOCK_PHRASES` in `doctor/text.rs`, meant to grow), "not just X, it's Y" frames, two lists of three in one message, more than one dash,
  an exclamation outside speech, and a line over its kind's length target (by key prefix). It prefers missing a case to flagging a good line.

`--strict` exits non-zero on errors. CI runs the doctor as a report step only, until the base pack's counts are clean.

**The rendered-text corpus** (`--corpus <dir>`) is for reading content through, the way a player meets it, to catch lines that do not make sense or read oddly:

```text
cargo run --release -p mandate-cli -- content doctor --corpus /tmp/corpus --seeds 12 --years 4
```

It plays the careers (rotating scenarios, storytellers and bot strategies) and writes every piece of player-visible prose: dilemma titles, situations, options with their trade-off lines, reasons and aftermaths; scene beats as the place shows them; storyline steps; press headlines; overheard lines; conversations (every line that holds for every speaker in town, passers-by included, on every topic and kind of subject, in the speaker's voice, plus the exchanges the bots really had, with what they taught and their outcome); notebook entries, sources, gaps and leads; and outcome lines (the action log, aftermath lines). Text is rendered by the game's own Fluent code (the web `Translator` and the screens' helpers, run with Vitest from `web/src/dev/corpus/`), with the real arguments.

Each item carries its **situation**: seed, date, strategy and storyteller, the town and its class; the speaker (role or post word, voice, traits, opinion band, strongest convictions, home neighbourhood and its worries); where; the topic and subject (with the truth about it: a bill's status and the town's support, a neighbourhood's worries); the template or line id; and the town's truth at that moment (top worries, live crisis, broker, rival, the player's post). Identical renderings are kept once; different argument fills are kept (up to six per line and voice). Files:

- `corpus.jsonl`: one rendered item per line (`kind`, `part`, `id`, `voice`, `text`, `key`, `args`, `situation`, `flags`).
- `corpus.md`: the same, grouped by kind and id, with the situation under each line.
- `counts.json`: items and ids per kind, flags (a missing key `⟦…⟧`, an unfilled `{$x}`, doubled spaces), and the line × voice coverage: pairs a speaker could say, pairs sampled, lines that never held.
- `raw.jsonl`: the keys and arguments before rendering. Rendering needs `web/node_modules`; without it the command stops after this file.

After rendering, the doctor reads the corpus back and reports what reads wrong in it: raw placeables or missing keys, empty arguments and doubled punctuation, one line per template with a count and an example. `--check-corpus <dir>` does only that, on a corpus already written.

**The designer page** shows one game at a time: open the app with `?dev=1` under `npm run dev` (optionally `&seed=…`). It needs an engine built with the `dev-tools` feature (the release engine never has it, and the page says so):

```text
cargo build -p mandate-wasm --target wasm32-unknown-unknown --profile release-wasm --features dev-tools
wasm-bindgen target/wasm32-unknown-unknown/release-wasm/mandate_wasm.wasm --out-dir web/src/engine/pkg --target web --omit-default-module-path
```

It starts a new game from the seed, or loads a copy of the current run's save (never written back), and can pass turns in its own sandbox. It lists every dilemma, situational policy, storyline, scene, overheard line, topic, conversation line and voice, with search and filters by kind and by whether it could fire now. For the item you pick, the engine answers `QueryDto::DevExplain`: the gates and conditions as a tree, each with the engine's own verdict (✓ or ✗) and the values the evaluator read; the cast it found (or, to preview something that cannot fire, a forced cast); and the text rendered with its real arguments, including a dilemma's options and their trade-off lines. Conversation items are explained for a speaker you pick, overheard lines at a place where people talk. A `vite build --mode devtools` bundle carries the page too; a normal build drops it.

The explanations come from `mandate_sim::explain`: one adapter (`CondExplainer`) for the one condition language, whose verdict is the one evaluator's (`mandate_sim::condition::holds`) in the context the item's use binds, with what it read for each variant (one table, `explain::observe`).

## Writing for Mandate

Players meet the text on a phone, a bubble at a time. They read it as one person speaking to them in a real town. This section is the house style for conversation lines, overheard talk, dilemmas, scenes, the press and outcomes. The first read-through found the problems it guards against (`docs/design/research/TEXT_AUDIT_1.md`), and `docs/design/research/TEXT_SAMPLES.md` shows what the style reads like.

### The one rule: true whenever it can be said

A line can be drawn whenever its conditions hold, so everything it claims must be true in **every** such case, not only the one you pictured.

- **A worry is real.** A line that names a worry ("It's the rent", "another lot laid off", "the air") needs a condition behind that worry: a `concern` condition naming it, an indicator (`rent_burden`, `job_loss`, `pollution`, `tax_burden`, `budget_stress`, `recent_disaster`), a crisis, or a plant closure or disaster. For an overheard line, its `concern`; for a dilemma, a trigger. A line that only says "the top worry" uses `$concern`/`$concern_talk` from a `tells` or `concern` condition, which is true by construction. The doctor's `worry-unconditioned` check catches a named worry with nothing behind it.
- **Strength matches the condition.** `"concern", "top": 2` means "among the top two", so the line says "comes up in every conversation", not "the first thing anyone talks about".
- **The player may hold no post.** A line that assumes the player sits in the chamber ("the planning item last week", "you get the same postbag", "your seat") needs `{ "type": "occupation", "slot": "player", "occupations": ["councillor", "mayor"] }`. Better, write it so it is true either way: "is after the same thing you are".
- **Familiarity is earned.** Do not say "you again" or "isn't it? I've seen you about" unless the conditions say they know the player. Warmth (`opinion`) is not acquaintance.
- **The speaker's own facts.** A role line speaks from the role ("people tell me all sorts across the counter"). A line that states the speaker's view on an issue needs a conviction or trait condition.
- **Only what happened.** An outcome or aftermath says what this option did and who noticed. A headline reports an event that is in the log.

When a condition cannot express what a line needs (a disaster's hazard, whether the player has met someone, an option shown only in some cases), say so in the content's comment, and write the line so that it stays true without it.

### Answer the question, and teach only what was said

A reply answers what the player asked. Asked about the newspaper office, a reporter talks about the paper, not the cost of living.

- **Lines for a subject read the subject.** In a topic that informs ("What are people worried about?", "Who runs things?", "Ask about…"), the engine shows only lines whose conditions read the subject, whenever the player gave one (`reply::candidates`). Those are lines with `slot: "about"`, a subject `of: "about"`, or an `about` condition. If none holds, it shows the topic's fallbacks. So a line about the topic in general never answers a question about one neighbourhood or one place. Write the on-subject line; do not rely on a general one.
- **Say which subject.** `{ "type": "about", "kinds": ["place"], "places": ["school"], "workplace": false }`. The fields:
  - `kinds`: place, person, hood, bill, template, rival;
  - `places`: place kinds;
  - `workplace`: the speaker's own place (their named place, or where their post sits);
  - `home`: the speaker's home neighbourhood;
  - `is: false`: the subject is not that.

  A place resolves to its neighbourhood for `hood_concerns of about`, so a neighbourhood line needs `"kinds": ["hood"]`, or it answers every place with that district's worries.
- **From inside, and from outside.** Someone asked about where they work answers as an insider: what they are chasing, who leans on them, the rota, the deadline. Everyone else says what the place is, who goes there and what people say of it. A cool speaker still answers the question ("what goes on there is my business, not yours").
- **Name it.** The reply names what was asked about (`$about`, or the subject's own argument). Only a fallback deflection may leave it out, and a deflection points somewhere ("Ask someone who lives there", "The stallholders at the market hear more than I do").
- **Teach only what was said.** A line's `teaches` must match its words. A line that names the town's or a neighbourhood's worries teaches that subject. Its `$concern` comes from the same ranking the notebook shows, so what was heard is what was noted. A speaker's own opinion is not a fact about the town. Talking to someone teaches who they are (name, role, where they work), never what they believe: a conviction is learned (the person at `detailed`) only from a line that voices it.

### Voices and warmth

A reply is an **opener** (by relationship, or in the speaker's voice), sometimes a **quote** of a memory, the **substance** (by the facts and what the speaker will tell), and sometimes a **closer** in the voice. Each part must read on its own and next to any of the others, so a closer cannot assume what the substance said, and an opener cannot promise a secret when the substance might be small talk.

Warmth bands (opinion of the player): **hostile** below −50, **cool** −50 to −15, **neutral** −15 to 15, **warm** 15 and up, **trusted** 50 and up with trust 0.5. Cool speakers tell nothing, so they get a topic's fallbacks: deflections in their role's words (weight 3), which are polite or prickly but never informative. Warm and trusted speakers can be generous, but they are still the same person in the same voice.

| Voice | Who (conditions, `voices.jsonc`) | Sounds like | Openers and closers | Avoid |
|---|---|---|---|---|
| `voice.plain` | everyone else | ordinary, direct, a little dry | "Anyway, that's how I see it." | colour borrowed from the other voices |
| `voice.blunt` | bold (≥ 0.6) and cool (warmth ≤ 0.45) | short sentences, no pleasantries, no closer most of the time | "Well?" · "Don't make me say it twice." | apologies, hedges, jokes |
| `voice.folksy` | warm (warmth ≥ 0.68) | homely, roundabout, kettle-and-coat; British idiom | "How are you keeping? Honestly, now." · "Now go on, before you catch your death." | endearments ("love") and mothering for cool speakers (condition them `min: -15`) |
| `voice.wry` | cool but not bold | dry, amused, unimpressed | "To what do I owe the displeasure?" · "It might never happen. It usually does, mind." | cruelty, puns |
| `voice.anxious` | timid (boldness ≤ 0.28) | apologetic, trailing off, dashes | "Oh! Sorry, I was miles away." · "You won't say it was me, will you?" | confidence, long speeches |
| `voice.guarded` | cautious trait | hedged, watching who is about | "That depends what you're asking." · "Best not to repeat that." | volunteering anything |
| `voice.earnest` | zealous or reformer trait | means every word, a little intense | "I'll be honest with you, because somebody ought to be." | irony |
| `voice.smooth` | charmer or schemer trait | flattering, deniable, always a little transactional | "I've been hearing good things," and doesn't say what. | warmth that costs them anything |
| `voice.pompous` | vain trait | their standing in every sentence | "You'll have heard I'm rather in demand these days." · pauses for applause | claiming posts they may not hold (say "the last public meeting", not "the last committee") |

Voice openers name their voice (`{ "type": "voice", … }`), so opener and closer always agree. Each voice that closes keeps at least two closers that fit anyone; closers that look back on something told ("Best not to repeat that") need `opinion min -15`, because cool speakers deflected.

### Writing for fills

Every argument comes with a shape. Write the sentence around the shape.

- **`$speaker`, `$player`, `$person`, `$rival`, `$broker`**: full names. Name someone once, at most twice, in a line; after that say "they" (singular, never "he" or "she": characters have no pronouns yet). Not "{ $person } wants more than { $person } has got … except possibly { $person }".
- **Strangers and passers-by** are named too (`$speaker` is always a name), and their role comes from the role word; never write "the steward" when the speaker may be a resident.
- **Places** (`$here`, `$about`, `$broker_place`): a place without a proper name arrives as its kind's noun phrase, already with "the" ("the union hall"), so it reads mid-sentence: "Spend an afternoon at { $broker_place }". Never put "the" before it.
- **Neighbourhoods and towns** (`$hood`, `$town`) are proper names. A neighbourhood can share the town's name (the centre).
- **Worries**: `$concern` is a capitalised label ("Housing costs", "Environment"); use it only in labels and lists. Mid-sentence, use `$concern_talk`, a lower-case noun phrase ("the rent", "jobs", "the pollution", "the state of the services"): "From what I hear, it's { $concern_talk }."
- **Policies** (`$policy`, `$bill`, `$law`): titles in sentence case, often plural ("Small business grants"). Put one first ("{ $policy }? From what I hear…"), after a colon, or framed as a name: "the { $bill } bill", "your bill, { $bill },", "the { $law } works". Never bare as a sentence's subject ("Small business grants was defeated") and never bare after a preposition ("counted heads on Local wage floor").
- **Decision titles** (`$title`, and `$subject` of a memory about a decision) are lower-case noun phrases that the cast will quote back ("You stood with me over the layoffs at the plant"). Name the business, not the player's view of it: "the truce offer", not "the rival's offer", which reads oddly from the rival.
- **Memories** (`$subject`, `$subject_kind`, `$year`): every line that quotes one selects on `$subject_kind` (`bill`, `decision`, else none) and has a variant that leaves the subject out. A memory about nothing in particular must never read "over that business" or "pressed them on  and". On the character sheet `memory-…` is third person ("You stood with them…, 2020"); a speaker quoting one to the player's face uses `scene-quote-…`, first person, the year spoken, not bracketed.
- **Labels** (tracks, goals, crises, storyline titles, support bands) are capitalised: after a colon only. "At stake: Health", "What Sofi Brook is after: Rallying the traders", not "Your Health is at stake".
- **Headlines** (`headline-…`, and `$headline` in overheard lines) are third person, present tense, no full stop. A line that quotes one adds its own stop: "'{ $headline }.'".
- **Numbers**: use a selector, `{ $n -> [one] a favour *[other] { $n } favours }`, never "1 days". Only `$n` arrives as a number; other counts are strings, so write around them ("{ $jobs } jobs gone" is fine; plural agreement on them is not).
- **Facts** (`$fact`) are noun phrases with their article ("a cash bribe").

### Stage directions and quotation

- Stage directions are present tense, third person, and name the speaker: `{ $speaker } barely looks up.` They describe what anyone could see. They do not narrate thoughts.
- Speech is in double quotes; a quotation inside speech in single quotes. Substance and quotes are always spoken; openers and closers may be either.
- A direction must not contradict the speech next to it: someone who "carries on as if you weren't there" does not then answer.
- British English: "neighbour", "organise", "the rates", "the council", "queue". Contractions in speech ("It's your campaign. Who's going to look?"), so nobody sounds like a form.

### Length on a phone

The doctor's limits are hard limits (titles 60 characters, option labels 90, lines 280, bodies 600, rendered with long names). Aim well under them:

- openers and closers: one short sentence (under 90 characters);
- substance: one or two sentences, one idea (under 160);
- dilemma situations: three sentences: what is happening, who wants what, what it costs you;
- option labels: what you do, in the imperative, and the catch if it is not obvious ("Haggle: less money, but the whip owes you one");
- aftermaths: what happened, then who noticed, in that order.

### Tone

- Local and specific: a bus shelter, a rates bill, a darts team, the food bank in the church hall. The world is small and the stakes are people's lives, so hardship lines are never jokes. Humour belongs in small talk and in voices (wry, folksy), never at the expense of someone struggling.
- Politically even. No line implies that one side is right; each option costs someone, and the text says who.
- No real people, parties, countries, cities or brands. Towns, outlets and names come from the generator.
- Offices and bodies come from the constitution (DEC-098): a role word through `$x_role` or an `occupation` condition. Existing lines that say "the council" or "Mayor" are debt, not precedent.
- Do not name a role that can mislead in a political game: the clergy role is "Vicar", because "Minister" reads as a government minister.

### Things to avoid

- A claim the conditions do not guarantee (above).
- The same phrase in many lines ("I've counted heads", "Ask anyone"). Read a topic's lines side by side before adding one. When the doctor says a line takes too large a share of its kind, add a sibling with the same conditions.
- Lines that only make sense in one order or one conversation ("as I said").
- Empty or generic fallbacks ("that business", "something"). Write a variant without the subject.
- A capitalised label mid-sentence, a bare policy name as a subject, the same name three times.
- Second person in the press ("You were re-elected"): the paper is about the player, not addressed to them.
- Meta words: "the director", "the storyteller" (the storyteller settings screen is the exception).

### Dilemmas: options a player can weigh

- The situation says who wants what and why now. The options differ in kind, not in degree, and each label says what you would do.
- Trade-off lines come from the effects. The label must not contradict them: an option that drafts flood defences is labelled as drafting flood defences.
- The default (no answer) option says so ("Don't answer, and let the offer lapse").
- An aftermath names the people who reacted and how. It never contradicts the situation: if the dilemma could follow any disaster, the aftermath must not assume a flood.

### Checking your writing

```text
cargo run -p mandate-cli -- content validate                          # well formed; every key in English
cargo run --release -p mandate-cli -- content doctor --seeds 0         # static: Fluent, length, text, worries
cargo run --release -p mandate-cli -- content doctor --seeds 16 --years 4   # also: never fired, fires too often
cargo run --release -p mandate-cli -- content doctor --corpus /tmp/corpus --seeds 12 --years 4
cargo run --release -p mandate-cli -- content doctor --check-corpus /tmp/corpus   # re-check an existing corpus
```

The text rules (`doctor.rs`, `text_problems`, `worry_checks`):

- **`text-raw`**: a placeable or a missing key in the text (`{$policy}`, `⟦key⟧`).
- **`text-empty-arg`**: an argument that came out empty: two spaces, a space before punctuation, empty brackets or quotes ("pressed them on  and got nowhere"). Only real fills show this, so it runs on the corpus.
- **`text-doubled-punct`**: two stops where one belongs (`..'`, `.,'`). An ellipsis and "?!" are fine. It runs on every message and on the corpus.
- **`worry-unconditioned`**: a line that names a worry with nothing in its conditions behind it.
- **`subject-unread`**: a line of a topic that is always about something ("Ask about…") whose conditions never read the subject.
- **`subject-drift`**: a line that can take a place but reads it only as its neighbourhood's worries.
- **`talk-off-subject`** (corpus): a reply to "Ask about…", or to a worries question about a neighbourhood, that does not name what was asked, unless it is a fallback.

Then read the corpus, not the `.ftl`. Read by kind, then by template, with all the fills side by side, and with the situation beside each line:

```text
jq -r 'select(.kind=="talk" and .id=="line.about.rival") | "\(.text)  [\(.situation.player_post)]"' /tmp/corpus/corpus.jsonl
jq -r 'select(.kind=="dilemma" and .part=="situation") | .text' /tmp/corpus/corpus.jsonl | sort | uniq -c
```

For each line, ask: is it true here? Does it read naturally with this name, this place, this policy? Would this speaker, in this voice, at this warmth, say it? Have I read it five times already? Then fix the text, or the condition, or both, and run the corpus again.

## Code extension points

Each of these is a closed enum by design, because new behaviour needs code. Adding one is one enum variant, one handler, content that uses it, and a test.

| Extension point | Enum | Handler | Also update |
|---|---|---|---|
| Policy mechanism | `MechanismDef` (`mandate-content/src/types.rs`) | `policy::mods_for` and the domain model that reads the new modifier (finance reads `borrow` and `revenue`: `policy::borrow_share`, `policy::annual_revenue`) | Validation of its params (templates and `policy_family::validate`); `MechanismDef::param`, `sign`, `key`; the DEC-073 vector (`policy_family::tradeoff`) if it finances; the protocol's trade-off view if it is shown |
| Family binding kind | `BindDef` (`mandate-content/src/policy_family.rs`) | `policy::families` (`Binder::bind_slot`, `binding_args`) | `BindDef::kind`, `refers`; `SlotKind`; `Bound`; validation (`bind_targets`); a test |
| Family `pick` rule | `Pick` | `Binder::bind_slot` (its score) | Validation of which slot kinds may use it |
| Sponsor price | `PriceDef` | `policy::drafting::adopt` | Validation (item 7); the preview's price view (F3) |
| Family knowledge hook | `HookDef` | `policy::drafting::matches` (and `learn_from` for a new kind of moment) | Validation (item 8) |
| Side-effect primitive | `SideEffectDef` | `policy::mods_for` or `population::opinion` (grievances) | `protocol::office` trade-off rows |
| Sector behaviour | `SectorDriver` | The `match` on drivers in `economy` | `SectorDriver::REQUIRED` if every economy needs one |
| Player action | `actions::Action` | `actions::apply` | `PLACE_ACTIONS` (and `LATE_NIGHT_ACTIONS`), the action cost in `tuning.jsonc`, the protocol offer, the web `ActionForm`, and a place that offers it |
| Information service | `PLACE_SERVICES` | Its payload builder in `protocol::places` | `PlacePanelView`, the web `PlacePanel`, and a place kind that offers it |
| Place kind worldgen generates | `world::PlaceKind` | Worldgen placement | A `places` entry (`places::registry_issues` checks that one exists); its web icon in `web/src/places/labels.ts`; the kind in `web/src/hub/testTowns.ts` so the town tests generate it. If the town layout changes with it, bump `LAYOUT_VERSION` (`web/src/hub/townLayout.ts`) and keep the old version callable, so a town in a saved run never rearranges (DEC-085) |
| Indicator (a measure of a town) | `Indicator` (`mandate-content/src/types.rs`) | `concerns::indicator` | `Indicator::ALL`; a test of its range. Concerns, crisis kinds, and (later) situational policies can then use it |
| Narrative cast or effect | `Cast`, `EffectDef` (`mandate-content/src/narrative.rs`) | `narrative::cast::candidates`, `narrative::resolve::apply_effect` | Validation in `narrative::validate`; `EffectDef::valence` (cost, benefit); the protocol's trade-off line (`protocol::narrative::preview`, `applied_line`); a test |
| Condition (any use) | `Condition` (`mandate-content/src/condition.rs`) | `mandate_sim::condition::holds` | Its tier, needs, arguments and validation in `condition.rs`; `explain::observe`; a probe in the conditions test ("Adding a condition" above) |
| Goal drive | `GoalDrive` | `characters::goals::refresh` (what it stores) and its reader in the NPC AI or council lean | Validation; a test that it changes the AI |
| Director lever (exogenous) | `narrative::Chance` | The owning system that clamps it, logging `DirectorIntervention` | The spotlight-independence test; never an endogenous outcome |
| Shady action | `actions::Action` (a variant), `SHADY_ACTIONS` (`mandate-content`) | `actions::shady` | `media.shady_actions` (payoff and hidden spec), `tuning.action_costs`, `PLACE_ACTIONS` (and `LATE_NIGHT_ACTIONS`, `LATE_ONLY_ACTIONS`), a place that offers it, `places::action_of`, the protocol `ActionDto` and the web `ActionForm` and routing |
| Discoverer | `mandate_content::Discoverer` | `media::exposure` (its access) | `media.discovery.access`; the scandal's breaking outlet in `media::open_scandal` |
| Story source | `media::StorySource` | `media::candidates` (when it is news, and its weight) and `media::cover` | `media::audit_story` (it must trace to a fact), the protocol's `media::article` headline key, and a Fluent `headline-*` key |
| Scandal or legal response | `ScandalResponseKind`, `LegalResponseKind` (`mandate-content`) | `media::respond`, `justice::respond` and `justice::charge_chance` | `media.responses` or `justice.responses`; the raised dilemma's options; the protocol's `effect-respond-*` / `effect-legal-*` lines; the bots' taste |
| Memory kind | `characters::MemoryKind` and `MEMORY_KINDS` (`mandate-content`) | A `characters::remember` call where it happens (or an `Effect::Memory`) | A row in `characters.memories`, the Fluent key `memory-<kind>`, a test |

Rules for code changes:

- Systems write only their own domain. Cross-domain changes go through effect requests.
- Randomness comes only from named or keyed streams. Key a new draw so that it does not shift existing streams. With base content, a new code path must make no extra draws, or results change.
- Iterate registries and use the cohort helpers (`cohort_index`, `age_of`, `class_of`, `community_of`, `group_index`). Never assume a fixed count or do index arithmetic.
- Add a unit or integration test for the handler. Run the full checklist in `AGENTS.md`, including WASM parity.

## Save-format changes

Current format: v13. Migrations exist from v9.

- If a serialised struct or enum changes shape, bump `SAVE_FORMAT_VERSION` (`mandate-save/src/lib.rs`) and update `EXPECTED` in `crates/mandate-save/tests/saves.rs`.
- Until release, a bump may reject older development saves. After release, add a JSON-level step to `migrate` and a fixture save.
- A purely additive field marked `#[serde(default)]` can keep the version.
- Narrative state names templates (dilemmas, storylines, goals) but never needs them: `narrative::on_load` lapses decisions and ends runs whose template was removed (v12).
- Media and justice state (v13) name outlet templates and fact kinds but never need them; a save from before v13 gets its outlets on load (`media::on_load`) and starts with no scandals or cases. Hidden facts from before have no fate and never come out.
- New registry-keyed state records its keys (as `sector_keys`, `attitude_keys`, `community_keys`, `concern_keys`, and `characters.crisis_keys` do), so that:
  - `Engine::from_state` can remap the data by key;
  - `engine::missing_references` can refuse content that removed an entry the save needs.
