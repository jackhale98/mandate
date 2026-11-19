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

Casts: `broker`, `rival` (in a campaign, whoever stands against the player), `mayor`, `occupation`, `colleague`, `creditor`, `bill` (optionally `sponsored_by` a cast role), `sponsor_of`, `ward_neighborhood`, `law` (optionally `building`), `goal_holder`. Conditions: `crisis`, `live_crisis`, `indicator`, `concern`, `industry`, `election_within`, `campaigning`, `office`, `track`, `money`, `opinion`, `favour`, `remembers`, `personality`, `trait`, `law_in_force`, `party_line_against_ward`, `can_vote`, `goal`, `months_in_play`. Effects: `opinion` (with a memory kind), `approval`, `awareness`, `money`, `health`, `slots`, `standing`, `legal_jeopardy`, `record`, `favour`, `repay`, `vote` (`yes`, `no`, `abstain`, `with_party`, `with_ward`; the council-vote rules apply), `draft` (with `params`). Fluent arguments: each role as `$<role>` (a bill or law by its policy name), a person's role as `$<role>_role`, and `$town`, `$crisis`, `$player`. Each new cast, condition, or effect is a code extension point (below).

**Goals** (`characters::goals`, politics domain): each month the people of the player's town take up goals whose conditions hold and give them up when they fail or after `max_days`. Drives: `oppose_player` and `back_player` add to their council lean on the player's bills (like lobbying sway) and an opposer lobbies against them; `champion` ranks bills that answer the live crisis higher. A goal with a `request` surfaces it (once per adoption). Goals are kept for the player's town only (NPC goals elsewhere are deferred).

**Storylines**: a run casts its roles once; if a person no longer fills their role (a rival stays cast while they stand against the player), the arc fades; stage decisions take the run's cast for shared role ids and cast the rest; a stage decision has priority over standalone dilemmas (it waits for room, not spacing). `follow_up` starts the same arc again later with the same cast where it still fits.

**Scenes**: a decision's scene happens at the place resolved when it opens (`Decision::place`). The pocket and the sheet always list it with every option; at the place, the conversation appears, and a speaker with `quote_fallback` quotes their most salient memory of the player (`salient_memories`).

**Storyteller profiles**: Classic (default, `director.default_profile`), Unfiltered (no levers, no tension gating), Steady (30% of strikes on the player's town miss, one crisis at a time, three quarters of the budgets). A new game can set `narrative.profile`.

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

## Code extension points

Each of these is a closed enum by design, because new behaviour needs code. Adding one is one enum variant, one handler, content that uses it, and a test.

| Extension point | Enum | Handler | Also update |
|---|---|---|---|
| Policy mechanism | `MechanismDef` (`mandate-content/src/types.rs`) | `policy::mods_for` and the domain model that reads the new modifier | Validation of its params; the protocol's trade-off view if it is shown |
| Side-effect primitive | `SideEffectDef` | `policy::mods_for` or `population::opinion` (grievances) | `protocol::office` trade-off rows |
| Sector behaviour | `SectorDriver` | The `match` on drivers in `economy` | `SectorDriver::REQUIRED` if every economy needs one |
| Player action | `actions::Action` | `actions::apply` | `PLACE_ACTIONS` (and `LATE_NIGHT_ACTIONS`), the action cost in `tuning.jsonc`, the protocol offer, the web `ActionForm`, and a place that offers it |
| Information service | `PLACE_SERVICES` | Its payload builder in `protocol::places` | `PlacePanelView`, the web `PlacePanel`, and a place kind that offers it |
| Place kind worldgen generates | `world::PlaceKind` | Worldgen placement | A `places` entry (`places::registry_issues` checks that one exists); its web icon in `web/src/places/labels.ts`; the kind in `web/src/hub/testTowns.ts` so the town tests generate it. If the town layout changes with it, bump `LAYOUT_VERSION` (`web/src/hub/townLayout.ts`) and keep the old version callable, so a town in a saved run never rearranges (DEC-085) |
| Indicator (a measure of a town) | `Indicator` (`mandate-content/src/types.rs`) | `concerns::indicator` | `Indicator::ALL`; a test of its range. Concerns, crisis kinds, and (later) situational policies can then use it |
| Narrative cast, condition, or effect | `Cast`, `Condition`, `EffectDef` (`mandate-content/src/narrative.rs`) | `narrative::cast::{candidates, holds}`, `narrative::resolve::apply_effect` | Validation in `narrative::validate`; `EffectDef::valence` (cost, benefit); the protocol's trade-off line (`protocol::narrative::preview`, `applied_line`); a test |
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
