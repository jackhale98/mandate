# Extending Mandate

How to add things to the game after release (DEC-075). There are three kinds of change:

1. **Pure content.** JSONC and Fluent in a pack, with no code. Most expansions are this.
2. **A code extension point.** New behaviour behind a closed enum. Each is one variant, one handler, and a test.
3. **A save-format change.** Needed when the serialised state's shape changes.

## Packs and load order

- A pack is a folder under `content/` with a `pack.jsonc` manifest (`id`, `version`, `name_key`, `load_order`), JSONC files of any `kind`, and Fluent files under `locale/<lang>/`.
- Every file is checked against `content/schemas/content.schema.json`, which is generated from the Rust types (DEC-053). After changing a content type, run `cargo run -p mandate-cli -- content schema` and commit the result.
- Registries (sectors, attitudes, communities, concerns, scenarios, places) are in **load order**: files sorted by path, then item order. The engine stores registry data densely in that order. A pack whose path sorts before `base/` puts its entries first. That is supported: saves record registry keys, not positions.
- Content ids are unique across all packs. Overriding an entry from an earlier pack ("pack patches") is not supported yet: a duplicate id is an error.
- Every `name_key` and every other Fluent key that content references must exist in English, the reference locale (DEC-054). The engine never produces prose.
- Check content with `cargo run -p mandate-cli -- content validate`.

## Pure content

| Add | Where (`kind`) | Required | Validation | Save rule on load |
|---|---|---|---|---|
| Policy | `policies` | params, cost, `mechanisms`, `signature` (attitude key → weight), `secondary_keys`, `side_effects` (at least one downside, DEC-073) | Params referenced by mechanisms exist; ranges are sane; signature keys are known attitudes; Fluent keys exist | Enacted instances name their template. Removing a template the save uses is refused (`missing_references`). |
| Sector | `sectors` | `driver` (one of the coded behaviours) plus traits | Unique ids; sane traits; at least one `building`, `local_services`, and `public` sector | `economy.sector_keys`. Matching keys keep their data. A new sector starts with no jobs and a capacity from its endowment. A removed sector is refused. |
| Attitude (an issue) | `attitudes` | `id`, short `key`, `seed` model; `conviction-pro-<key>` and `conviction-anti-<key>` Fluent keys | Unique ids and keys (`[a-z][a-z0-9_]*`); finite seeds; conviction keys exist | `population.attitude_keys`. Values are kept by key. A new attitude starts without randomness: cohorts at the seed mean (plus their community's offset), politicians at their home neighbourhood's view, parties neutral. A removed attitude is refused. |
| Community | `communities` | `id`, `name_key`; `share` for every community but one; optional `attitude_offsets`, `turnout_offset`, `immigration_share` | At least one community; exactly one without `share` (it takes the remainder); values finite and in range; offsets name known attitudes. Shares that add up to more than 1 give a warning. | `population.community_keys`. Cohorts and children keep their community. A new community starts with no people and grows through immigration and births. A removed community is refused. |
| Voter concern | `concerns` | `id`, `name_key`, `indicator` (one of the engine's measures), `attitudes` (key → signed weight), `level` (`baseline`, `slope`, `min`, `max`); optional `overheard_key` | Known attitudes; weights non-zero in [-1, 1]; `min` ≤ `max`; Fluent keys exist. An empty registry is allowed (every issue then weighs the same). | `population.concern_keys`. Levels and salience are derived: content that adds or removes a concern recomputes them on load. Never refused. |
| Scenario (a starting world) | `scenarios` | `id`, `name_key`; optional `geography`, `economy`, `era`, `politics` biases | Finite, in range; `attitude_shift` names known attitudes; the name exists in English. The title screen lists the registry. | `meta.params.scenario`. Removing the save's scenario is refused (terrain is rebuilt from it). |
| Place behaviour | `places` | `key` (a place kind worldgen generates, or `home` / `streets`), `name_key`, `actions`, `services`, optional `fallback` | Known actions and services; late-night actions are offered; fallbacks resolve; every generated kind has an entry | Behaviour is read live. Nothing in a save is keyed by it. |
| Constitution | `constitution` | Tiers, bodies, offices, procedures, election calendars | Structural checks and Fluent keys | `meta.constitution`. Removing the save's constitution is refused. |
| Language | `locale/<lang>/*.ftl` | Translations of the English keys | Referenced keys are checked against English only. The web currently requests `en`, so choosing another language is a small UI change. | None |
| Tuning, economy, politics parameters | `tuning`, `economy`, `politics` | One file of each across all packs | Ranges | Read live. A balance patch changes results from load onwards, and a `RulesUpdated` event is recorded. |
| Balance parameter blocks | `opinion`, `actions`, `council`, `career`, `npc` | One file of each across all packs, every field written | Finite and in range (`validate::param_ranges`); `[min, max]` pairs ordered | Read live, as above. |

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

Indicators (`mandate_content::Indicator`, computed in `mandate_sim::concerns::indicator`): `unemployment`, `unemployment_rise`, `rent_burden`, `pollution` (the region's), `tax_burden`, `budget_stress`, `recent_disaster`. There is no crime in the simulation yet, so there is no safety concern. Query a town's concerns with `concerns::top_concerns(pop, m)` (by level) or `concerns::felt_concerns` (for some neighbourhoods); its salience with `concerns::salience(pop, m)`.

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

**Tests to add for pure content.** Use an in-memory pack: push `Source`s onto `read_dir_sources(content/)`. Follow `crates/mandate-worldgen/tests/{sectors,attitudes,communities,concerns,scenarios}.rs`:

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
| Indicator (a measure of a town) | `Indicator` (`mandate-content/src/types.rs`) | `concerns::indicator` | `Indicator::ALL`; a test of its range. Concerns and (later) situational policies can then use it |

Rules for code changes:

- Systems write only their own domain. Cross-domain changes go through effect requests.
- Randomness comes only from named or keyed streams. Key a new draw so that it does not shift existing streams. With base content, a new code path must make no extra draws, or results change.
- Iterate registries and use the cohort helpers (`cohort_index`, `age_of`, `class_of`, `community_of`, `group_index`). Never assume a fixed count or do index arithmetic.
- Add a unit or integration test for the handler. Run the full checklist in `AGENTS.md`, including WASM parity.

## Save-format changes

- If a serialised struct or enum changes shape, bump `SAVE_FORMAT_VERSION` (`mandate-save/src/lib.rs`) and update `EXPECTED` in `crates/mandate-save/tests/saves.rs`.
- Until release, a bump may reject older development saves. After release, add a JSON-level step to `migrate` and a fixture save.
- A purely additive field marked `#[serde(default)]` can keep the version.
- New registry-keyed state records its keys (as `sector_keys`, `attitude_keys`, `community_keys`, and `concern_keys` do), so that:
  - `Engine::from_state` can remap the data by key;
  - `engine::missing_references` can refuse content that removed an entry the save needs.
