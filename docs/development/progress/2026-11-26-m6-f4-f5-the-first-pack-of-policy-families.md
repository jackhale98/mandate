# M6 F4 + F5 the first pack of policy families, the trim, and the policies lab

**Date:** 2026-11-26
**Status:** done (content, lab, CI gate). No save change (state shape unchanged); outcomes move on purpose (the base pack now has families), so pins are re-recorded.

## F4: content

- **The first pack** (M6_ENGINE_CONTRACTS §2.21): 18 local families in `content/base/policy_families/{economy,hazards,town_life,relief}.jsonc`, Fluent in `locale/en/families.ftl`. Every family is keyed by its jurisdiction (DEC-098): `requires: propose_ordinance` is a constitution power, people bind by occupation or the `broker`/`executive` fallbacks, never by office. Two or three axes each, every option a cost and a benefit (DEC-073, validated), sponsors with favour or endorsement prices, knowledge hooks (overheard, visit, meet, town hall, canvass, services) and leads. Seven carry works (DEC-099): flood wall, community centre, park, incubator, port berths, market stalls, reservoir.
- **The trim** (§2.19): `policy.flood_defenses`, `policy.tech_incubator`, `policy.local_wage_floor` are `retired`; eight general levers remain.
- **The four conversions** (§2.20): `policy.closure_response`, `policy.rebuild_fund`, `policy.emergency_rent`, `policy.fiscal_bridge` are `retired`; `family.closure_response`, `family.rebuild`, `family.rent_relief`, `family.fiscal_bridge` take their unlocks as triggers (always drawn, weight 4). Two triggers gained an alternative because the unlock never held in 24 months at 16 seeds (budget stress stays 0; rent burden peaks at 0.17): Close the gap also opens after a year in the red (`budget_balance` ≤ −0.02), rent relief where housing is a neighbourhood's first worry. Retiring the transitional `situational` path: `policy::unlocked` is false for a retired template, and retired templates leave the catalogue, the standard-menu support grants, the talk topics, `Knowing::on_menu` and the doctor's frequency list. `Subject::Situational` entries become inert.
- **Proposal overheard lines**: a new tier-generic condition `proposal { family }` (an open proposal of that family in the jurisdiction; `$proposal_<slot>` for its places and neighbourhoods, a named plant's or a place kind's key where a place has no name, `$proposal_<slot>_key` for sectors and hazards; never a person). 18 lines in `overheard.jsonc`, one per family.
- **Crisis and dilemma references**: `crises.jsonc` answers name families (and the fiscal hole the property tax); `dilemma.disaster_relief` drafts `family.flood_wall` (the narrative `draft` effect now accepts a family: the open proposal in the player's focus, taught `known`, drafted at its defaults; a retired template is refused by validation). The flood and plant-closure arcs list the families beside the retired templates in `answers` and `law_in_force`. `Cast::Law` stays template-only (a family law's name needs its slot arguments in the scene).

## F5: the lab

- `mandate-cli policies --seed <s> [--size] [--scenario] [--town <id>] [--months 24]`: the home town's deck, every proposal (opened, lapsed and why, bindings by name, sponsors), the default package's cost, budget share and support and each sponsor's version with its price (previews through `family_preview` with the lab switch on), and per family the months it bound at home, why it failed, and the jurisdictions of its tier where it bound.
- `policies --coverage --seeds <n>`: per family the drawn share, binds share (anywhere, and at home), opened share, proposals per run, median months open, median default cost and support, what failed most, jurisdictions bound; proposals per jurisdiction; the home town's indicator maxima. **Gate:** a family that binds in no world fails (`--report-only` prints without failing).
- `policies --check`: validation item 11 on the canonical worlds (five scenarios at two sizes, 36 months).
- "Binds" is `policy::families::probe` (slots and trigger) in the home town every month and every jurisdiction of the family's tier each quarter.
- `--set` reaches families and their parameters (tests in `mandate-content/src/overrides.rs` and `policies.rs`).
- Bots: legislating bots draft the best known open proposal when it beats the best of the standard menu by the ward support they know; `bots` JSON gains `families` (drafted per strategy and by each world's best informed style).
- CI: the lab job runs `policies --coverage --seeds 16` and `policies --check`.

## Evidence

- `policies --coverage --seeds 16`: 18 of 18 bind (`never_bind: []`); `policies --check`: 18 of 18. The family table is in the task report.
- `bots --seeds 16`: every enforced gate passes before and after; mean shares fall (TUNING_QUEUE). Winners drafted 8 distinct families in 13 of 16 worlds.

## Left

- Campus incubator, port expansion and water for the farms never bind in a home town (no home town has a campus, a port or an open farm): they bind in other towns and wait for M7's wider focus.
- Standing families fill the four places early; crisis families rarely find room (TUNING_QUEUE).
- The coverage report does not yet run the estimate's honesty per preset (IMPACT_ESTIMATE §189).
- Overheard proposal lines are weighted, not forced into the week's draw (§2.7's rule).
