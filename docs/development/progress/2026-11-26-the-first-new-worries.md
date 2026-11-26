# The first new worries (DEC-106 batch 1)

**Date:** 2026-11-26
**Status:** built (DEC-106 item 1 "first batch, with signals that exist"; M6_CHECKLIST item 4)

**Seven worries, each from a simulated signal (DEC-095).** `content/base/concerns.jsonc` gains `concern.cost_of_living`, `concern.newcomers`, `concern.young_leaving`, `concern.schools_health`, `concern.upkeep`, `concern.trust` and `concern.development` (13 worries in all). A worry may read more than one signal: `ConcernDef.also` (`[{ indicator, slope }]`) adds `slope × indicator` to the level before the clamp (`ConcernDef::level_from`). Ten indicators are new (`mandate-content` `Indicator`, each computed in `mandate_sim::concerns::indicator_with` and, per neighbourhood, in `hood_indicators`). How neighbourhoods feel the new signals: `opinion.local_concerns.signals` (`WorrySignalParams`).

| Worry | Signal → indicator (the Why chain) | Who weighs it | Laws on the base that move it |
|---|---|---|---|
| Cost of living | prices rise (`national.price_level`), wages follow at 8% a month; each class's income (wages, benefits, transfers) over prices is kept monthly (`MuniSnapshot.real_income`) → `cost_of_living` = the share of real income lost in a year, by class | each class by how much it spends on essentials (`cost_of_living_class` low 1.3, middle 1.0, high 0.6) and its own income's loss; a neighbourhood by its residents' classes; felt more by voters for redistribution | wage floor, hardship fund (they raise the low class's income, so they lower its loss for the year after) |
| Newcomers and change | internal migration (`population::migration`) → `PopulationState.flows` (adults in from other towns, a year's total, losing a twelfth a month) → `arrivals` = a year's arrivals ÷ adults | the same rate in every neighbourhood (arrivals settle by adult count, the audit's known gap); felt more by voters for tradition and localism, less by social liberals | none lowers it; homes (affordable housing, upzoning) and jobs draw more movers and raise it |
| Young people leaving | the same flows for 18–29s out and in → `young_leaving` = net 18–29s lost in a year ÷ the town's 18–29s; plus `old_age_share` = 65+ share of adults − the nation's | the town's net loss everywhere; each neighbourhood its own 65+ share (older districts more); felt by localism and reform | jobs and rents through attractiveness (small business grants, tech incubator, affordable housing) |
| Schools and health care | the Education and Health lines of the town, its region and the nation (the town's share by residents) against need → `care_shortfall`, − service gains of laws on those lines | × the neighbourhood's share of children (weight `care_children` 1) and 65+ (`care_seniors` 1.5) against the town's; felt by tax tolerance and redistribution | none on the base (no Education or Health service template) |
| Roads and upkeep | the Infrastructure line against need → `upkeep_shortfall` (1 − min(line/need, `maintain_cap`)); in focus towns B2's condition → `street_wear` (start midpoint − condition), and each open fault adds `worry_per_fault` (faults now feed this worry) | the line everywhere; the wear and the faults per neighbourhood; felt by tax tolerance and localism | a budget vote on the Infrastructure line (`Effect::Budget`, the budget round) |
| Trust and corruption | scandals about the town's politicians (`media.scandals`, by gravity, fading over `scandal_days`) and their cases (`justice.cases`: open, or convicted within the days) → `scandals` | the same in every neighbourhood; felt by reform appetite and law and order | none |
| Development and character | homes under construction (`housing.pipeline`) ÷ stock → `construction`; works being built (`mods.disruption`, `hood_disruption`) → `works_disruption` | construction × how much faster the district has grown than its town; works by the town's share and the district's own; felt by tradition, localism, environment | upzoning and affordable housing raise it (more building); none lowers it |

**Services, split.** `ServiceShortfall` now reads the town's lines other than Education, Health and Infrastructure (`concerns::services_index`) and the service gains of laws on lines other than Education and Health; schools and health care and the streets are their own worries. While every line moves together (the budget rule) the index is the budget's. `concern.services` keeps its id, attitudes and numbers, so the ask on services (`asks.jsonc`), its promise (`promises.jsonc`), `crisis.fiscal_hole`, the outlets' interests and the exchange readings still read it. Approval still reads the whole budget (`budget_index`). Faults feed `concern.upkeep` (`faults.jsonc` `feeds`); the school and the clinic address `concern.schools_health` (`places.jsonc`); the clinic and school overheard lines moved to it.

**Text.** Names and talk phrases (`base.ftl`), five or more overheard lines per worry (`overheard.jsonc`, `overheard.ftl`; newcomers' lines speak of change, never blame), the generic worry lines name the new worries (`lines.jsonc`), and four lines read the new signals (a teacher and a nurse on `care_shortfall`, anyone on `cost_of_living` and `scandals`). The text audit's worry words cover them (`doctor/worry.rs`). Notebook lines and views are generic (`concern_key`): no web change.

**Lab.** `mandate-cli audit worries [--seeds 16 --years 5]`: each worry's share of the strongest felt worry (towns and neighbourhoods), mean felt level, the measured strongest and top three, the worlds in which it is the home town's strongest measured worry in the first 14 months, and each signal's spread (p10, p50, p90, p99), from which the first numbers were set (typical town 0.2–0.5, the worst few percent 0.7–1).

**Save:** v34 (`worry_signals`: nothing to rewrite; flows start empty, real incomes read 0 until a year is kept).

**Twins** (`crates/mandate-wasm/tests/new_worries.rs`; seeds 2026 and 7, a year in, then the signal moved; the worry levels in the home town's neighbourhoods; a third copy with nothing moved gives the same worry exactly):

| Worry | Moved | Still → moved (town mean, two seeds) | Where |
|---|---|---|---|
| Cost of living | prices +3% | 0.23 → 0.49; 0.64 → 0.89 | every neighbourhood; the most low-income one rises at least as much as the least (unclamped) |
| Newcomers | home town's housing stock +25% (room and lower rents draw movers), 90 days | 0.28 → 0.76; 0.20 → 0.79 | every neighbourhood alike |
| Young people leaving | biggest sector's demand × 0.3 for a year, 120 days | 0.11 → 0.25; 0.24 → 0.36 | every neighbourhood; the older ones stay highest |
| Schools and health care | Education line halved | 0.13 → 0.57; 0.07 → 0.49 | every neighbourhood, a little more where more children and old live |
| Roads and upkeep | Infrastructure line halved | 0.21 → 1.0; 0.38 → 1.0 | every neighbourhood (B2's two-year twin with faults, `faults.rs`, now reads this worry) |
| Trust | a misconduct scandal in the town | 0.00 → 0.57; 0.00 → 0.57 | every neighbourhood |
| Development | homes under construction × 3 | 0.31 → 1.0; 0.21 → 0.92 | every neighbourhood, by growth |
| The split | Education halved: services unchanged; Administration, Safety and Social halved: schools and health unchanged, services 0.00 → 0.69, 0.06 → 0.97 | | |

Determinism through a save: flows and real incomes survive, and the games go on with one hash (`the_new_signals_survive_a_save`).

**No worry dominates** (`audit worries --seeds 16 --years 5`, idle, 28,800 town-months, 197,700 neighbourhood-months; share of months a worry is the strongest felt):

| Worry | Towns before → after | Neighbourhoods before → after | Mean felt | Measured strongest / in top 3 (towns) |
|---|---|---|---|---|
| Housing | 74.4 → 36.6% | 62.5 → 30.7% | 0.46 | 35.5 / 85.7% |
| Jobs | 11.1 → 4.0% | 12.6 → 4.7% | 0.22 | 3.9 / 16.8% |
| Environment | 11.5 → 8.6% | 18.4 → 14.4% | 0.23 | 5.6 / 19.1% |
| Taxes | 0.9 → 0.2% | 4.3 → 0.5% | 0.30 | 0.1 / 17.6% |
| Services | 0.6 → 0.0% | 0.7 → 0.0% | 0.05 | 0.0 / 1.0% |
| Recovery | 1.6 → 0.9% | 1.5 → 0.9% | 0.02 | 0.9 / 1.9% |
| Cost of living | 6.9% | 5.6% | 0.22 | 6.3 / 15.2% |
| Newcomers | 9.9% | 6.3% | 0.26 | 12.7 / 37.2% |
| Young people leaving | 10.7% | 18.6% | 0.21 | 11.1 / 25.1% |
| Schools and health | 4.2% | 3.5% | 0.26 | 4.9 / 22.9% |
| Roads and upkeep | 6.0% | 5.1% | 0.28 | 7.0 / 25.9% |
| Trust | 3.5% | 3.8% | 0.04 | 3.4 / 4.6% |
| Development | 8.4% | 5.9% | 0.26 | 8.6 / 27.1% |

**Bots** (`bots --seeds 16`): every enforced gate holds. `good_styles_vary` (report-only) fails on clause (c), the best style differing by starting situation: `explorer_legislator` is best in both situations (before, one world in each other situation had another best). Win rates fall for the weaker routines: idle 0.125 → 0, explore-only 0.125 → 0, media 0.25 → 0.06, proxy 1.0 → 0.75, corrupt 0.94 → 0.75, directed 0.81 → 0.69; every mean share falls 0.03 to 0.08 (the issue weights are spread over more worries).

**Numbers to tune** (first guesses, rule 11): every DEC-106 level (`concerns.jsonc`), `signals` (`opinion.jsonc`). Also: the services worry is never the strongest and is in a town's measured top three 1.0% of months (its shortfall sits near −0.12: budgets run above need), so the ask on services (`top: 3`) seldom fires; housing as the home town's strongest measured worry within 14 months falls from 11 of 16 worlds (TUNING_QUEUE, the zoning arc's raise) to 9 of 16.

**Not built, and why.**
- Migration by age exists only as which bands move (18–29 and 30–44); arrivals by neighbourhood do not (they settle by adult count), so the newcomers worry does not differ between a town's districts.
- International arrivals (`demography`'s immigration) are not counted as newcomers: DEC-106 item 4 makes immigration and emigration their own worries.
- Street wear exists only in the player's focus (B2's upkeep); elsewhere roads and upkeep reads the line alone.
- No per-worry "Why?" view: the chain above is the explanation; a view would need the indicator values on the protocol.

**Tests moved with the worlds.** The worlds changed (the burn-in's politics weigh thirteen worries), so pins were re-recorded (`scripts/repin.sh`) and the scripted flood arcs re-found their world the way they did before (`arcs.rs`, `director_arcs.rs`: 106 to 111, the first after 106 where the scripts hold). Two world searches now ask for what their tests use: a colleague not yet settled on the bill (`fresh_colleague_and_bill_before_council`) and a regular there at the start (`councillor_and_regular_at_a_place`); `talk.rs`'s lobbying world moved from 31 to 32 (in 31 the player already owed the colleague the most favours allowed, so "owe" was not offered).

**Found, not fixed (the arcs' and the director's):** `narrative.rs` `the_spotlight_never_changes_endogenous_outcomes` fails on seed 11 in the new worlds: with the spotlight on, the rival's debate is staged (the chapter's lead) and the player's absence sets `debate_ducked` (record −0.01); with it off, the chapter has no lead, the zoning meeting takes the chapter's one scene and the debate is never staged. The scene rule (DEC-112, one scene a chapter, the lead's) reads `Chapter.lead`, which only the spotlight sets (`narrative/director/arcs.rs` `lead`), so the spotlight changes an outcome (DEC-033 B).
