# Extending Mandate

How to add things to the game after release (DEC-075). There are three kinds of change:

1. **Pure content.** JSONC and Fluent in a pack, with no code. Most expansions are this.
2. **A code extension point.** New behaviour behind a closed enum. Each is one variant, one handler, and a test.
3. **A save-format change.** Needed when the serialised state's shape changes.

## Packs and load order

- A pack is a folder under `content/` with a `pack.jsonc` manifest (`id`, `version`, `name_key`, `load_order`), JSONC files of any `kind`, and Fluent files under `locale/<lang>/`.
- Every file is checked against `content/schemas/content.schema.json`, which is generated from the Rust types (DEC-053). After changing a content type, run `cargo run -p mandate-cli -- content schema` and commit the result.
- Registries (sectors, attitudes, communities, concerns, scenarios, places, traits, occupations, crises) are in **load order**: files sorted by path, then item order. The engine stores registry data densely in that order. A pack whose path sorts before `base/` puts its entries first. That is supported: saves record registry keys, not positions.
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
| Constitution | `constitution` | Tiers, bodies, offices, procedures, election calendars | Structural checks and Fluent keys | `meta.constitution`. Removing the save's constitution is refused. |
| Language | `locale/<lang>/*.ftl` | Translations of the English keys | Referenced keys are checked against English only. The web currently requests `en`, so choosing another language is a small UI change. | None |
| Tuning, economy, politics parameters | `tuning`, `economy`, `politics` | One file of each across all packs | Ranges | Read live. A balance patch changes results from load onwards, and a `RulesUpdated` event is recorded. |
| Balance parameter blocks | `opinion`, `actions`, `council`, `career`, `npc`, `characters` | One file of each across all packs, every field written | Finite and in range (`validate::param_ranges`); `[min, max]` pairs ordered | Read live, as above. |

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

### Scenarios (DEC-078 decision 4)

A scenario only says what makes its world different. Anything omitted is drawn exactly as for a random country (a fixed value makes no draw).

| Block | Fields |
|---|---|
| `geography` | `sea_sides` (1–3), `mountain_strength`, `country_extent`, `wetness`, `temp_south`, `mineral_richness`, `population` (multiplier), `industrial_towns` |
| `economy` | `endowment` (sector driver → multiplier), `wealth`, `class_mix` ([low, middle, high] multipliers) |
| `era` | `tech_growth`, `manufacturing_trend` (defaults in `economy.jsonc`), `drying`, `storminess` |
| `politics` | `attitude_shift` (attitude key → added to seeded opinions) |

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
| Place kind worldgen generates | `world::PlaceKind` | Worldgen placement | A `places` entry (`places::registry_issues` checks that one exists) |
| Indicator (a measure of a town) | `Indicator` (`mandate-content/src/types.rs`) | `concerns::indicator` | `Indicator::ALL`; a test of its range. Concerns, crisis kinds, and (later) situational policies can then use it |
| Memory kind | `characters::MemoryKind` and `MEMORY_KINDS` (`mandate-content`) | A `characters::remember` call where it happens (or an `Effect::Memory`) | A row in `characters.memories`, the Fluent key `memory-<kind>`, a test |

Rules for code changes:

- Systems write only their own domain. Cross-domain changes go through effect requests.
- Randomness comes only from named or keyed streams. Key a new draw so that it does not shift existing streams. With base content, a new code path must make no extra draws, or results change.
- Iterate registries and use the cohort helpers (`cohort_index`, `age_of`, `class_of`, `community_of`, `group_index`). Never assume a fixed count or do index arithmetic.
- Add a unit or integration test for the handler. Run the full checklist in `AGENTS.md`, including WASM parity.

## Save-format changes

- If a serialised struct or enum changes shape, bump `SAVE_FORMAT_VERSION` (`mandate-save/src/lib.rs`) and update `EXPECTED` in `crates/mandate-save/tests/saves.rs`.
- Until release, a bump may reject older development saves. After release, add a JSON-level step to `migrate` and a fixture save.
- A purely additive field marked `#[serde(default)]` can keep the version.
- New registry-keyed state records its keys (as `sector_keys`, `attitude_keys`, `community_keys`, `concern_keys`, and `characters.crisis_keys` do), so that:
  - `Engine::from_state` can remap the data by key;
  - `engine::missing_references` can refuse content that removed an entry the save needs.
