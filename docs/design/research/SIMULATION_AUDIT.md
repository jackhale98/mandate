# Simulation audit (W2): is the back end effective?

**Date:** 2026-11-23. **Asked by:** the owner (M6_CHECKLIST, "the simulation audit"). **Status:** measured; no simulation behaviour or content was changed. Fixes are a later task.

The question: stability is already tested (bands, determinism). Does the simulation behave believably, and do decisions move it in sensible, readable ways? This report answers with experiments on the new experiment runner (S2, `crates/mandate-cli/src/lab/`). Every table names the command that produced it. Unless stated, worlds are small, the driver is the idle player (turns end, NPC turns run), and "the six worlds" means a random country plus the five canonical scenarios (`content/base/scenarios.jsonc`). `t` is a mean over its standard error across seeds; |t| ≥ 2 is a real effect.

Commands run from the repository root with a release build: `cargo run --release -p mandate-cli -- audit <topic> [--seeds N] [--years Y] [--scenario id] [--only filter]`. `MANDATE_LAB_THREADS=3` limits the threads. Raw JSON for every table is what the command prints; nothing here is hand-computed beyond rounding.

## Executive summary: the ten findings that hurt play most

1. **Votes barely respond to how the town is doing.** Incumbents are re-elected 82% of the time when local unemployment fell against the nation and 80% when it rose (a 1.8-point gap; per point of unemployment, −0.5 points, t = −1.7). Approval moves (−1.45 points per point of unemployment), but each point of approval adds only 0.1 points to the chance of re-election. A plant closure that raises unemployment 5.9 points and cuts approval 11 points changes the incumbents' wins from 233 to 234 of 378 races. *The game's thesis ("governing well matters") is not true in the engine yet.* (Topic 2.)
2. **Rents run away.** The median town's rent rises 7.9× over 30 years while prices rise 1.85×; one town in 13 rises more than four times faster than the median town, and in the worst a year's rent reaches 196 times a middle-class income. Vacancy is pinned at its −5% floor, private building is stalled (under 20% of normal starts) in 43% of town-months, and 822 housing-squeeze crises last ten years or more. One town starts play with a year's rent at 3.9 times income. (Topic 6.)
3. **Almost every law makes the government less popular, whatever it does.** Twelve of fifteen templates lower approval in their town after five years (−1.0 to −3.5 points), including the hardship fund, homes, the closure response and the wage floor. Only the three service templates raise it (+3.4 to +3.7). Material benefits (jobs, homes, income) do not enter approval; spending of any kind raises the "services" worry, which reads budget stress. (Topic 1.)
4. **Parties do not lose.** In 38% of towns the executive's party never changes in 30 years; the party holding most town halls keeps that lead in 93.5% of years; the share of towns run by anyone else falls from 39% to 16%, and the spread of attitudes between towns shrinks by a third. (Topics 4 and 6.)
5. **Elections are won by being known, not by what the player does.** A bot that only canvasses wins its first election 75% of the time; the informed styles win 81%, exactly as often as the blind mix; idle wins 6%. The most any style moves town unemployment is −0.7 points over six years, and no style moves the ward's "services" worry at all. (Topic 5.)
6. **The "Why?" surfaces mostly blame luck.** When an incumbent loses, the election-night breakdown's largest factor is the random race shock in 56% of cases, party in 20%, issues in 20%, record in 1%. In races that a planted plant closure flipped, record is never the factor that moved most. (Topic 7.)
7. **Policies move their own indicator but rarely reach votes.** Every template, enacted at its defaults, flips between 0.4% and 1.3% of the home town's races over five years. Upzoning builds nothing (stock −0.05%, t = −0.7, because private building is stalled); small-business grants' effect on unemployment is gone by year 5; the tech incubator crowds out other jobs (total jobs −0.4%); transfers barely reach low incomes (+0.6%, t = 1.3). (Topic 1.)
8. **The bill's ledger overstates homes fourfold.** The affordable-housing ledger says 581 homes built by month 36; the twin shows 145 more homes than without it. Jobs ledgers are fair (0.6–0.8 of the twin). (Topic 7.)
9. **Several described mechanisms do nothing.** Directional voting (β) changes nothing at all when switched off; the partisan perception filter flips 1 race in 5,313; cohort `trust` is never read; turnout has no duty, cost or closeness term; challengers do not judge their chances. (Topic 8.)
10. **Migration is weak and partly wrong-signed, and places are not markets.** Population growth falls 0.06% per point of relative unemployment but also falls with real wages (t = −40.6, expected positive); switching migration off for ten years moves unemployment by 0.008 points. The three known gaps are confirmed (one labour market and one rent per town; arrivals spread by adult count; no moves between neighbourhoods except disasters), and neighbourhood approval moves as one (after a closure, the jobs worry rises 26 points in one neighbourhood and 64 in another, but approval falls 11.1 and 10.9). (Topics 2 and 8.)

What works: recessions hit sectors differently and believably; closures and disasters show up in worries, crises and neighbourhood differences; unemployment jumps are almost always explained by a visible crisis (98%); no debt spirals, no towns drained, no unemployment pinned at a bound; incumbency exists and is beatable (81% re-election, 16% of races within 10 points); service laws, the wage floor and the property tax do what they say; the lab itself is exact (a twin without an intervention differs in nothing).

## Verdicts by topic

| # | Topic | Verdict | One line |
|---|---|---|---|
| 1 | Policy responsiveness | weak | Laws move their mechanism's indicator with the right sign (26 of 40 checks; 8 show no effect, 6 the wrong sign), but approval falls for most, and votes barely move |
| 2 | Believable behaviour | weak | Rent and recessions work; retrospective voting is broken; migration and turnout are weak |
| 3 | Sensitivity | weak | No single parameter decides everything; votes are decided by the shock, swing, issue and party terms, while the opinion parameters flip almost no races; three economy parameters cannot be patched after the start |
| 4 | Towns diverging | weak | Economies stay distinct (industry mix kept at 0.98, wage spread grows); politics converge to one party |
| 5 | Player leverage | weak | Real on own elections (6% → 81%), via awareness; small on conditions; knowledge adds nothing |
| 6 | Stuck states | broken | Rent runaway and one-party towns; no debt or population spirals |
| 7 | Readability | weak | Worries and crises explain jumps; election night blames luck; the homes ledger overstates |
| 8 | Claims check | weak | Most of the vote model is used; β, the partisan filter, trust and several turnout and policy side claims are not |

## The experiment runner (S2)

`crates/mandate-cli/src/lab/` (`mod.rs`, `runner.rs`, `metrics.rs`, `cli.rs`; the old `lab.rs` overrides moved to `lab/overrides.rs` unchanged):

- `Setup { seed, size, scenario }`, `Driver::{None, Idle, Bot { strategy }}` (none: days advance, no turns; idle: turns end, NPC turns run; a bot: one of the `bots` strategies plays every turn).
- `Game::new(content, &setup, driver)`, `step_month()`, `run_to(month, samples, &mut out)`, `fork()` (an exact copy through `Engine::from_state`; tested equal to continuing), `snapshot()`, `intervene(&Intervention)`.
- `Intervention::{Policy { template, town, params }, Set { sets }, SectorShock, PlantClosure { town, sector, share }, Disaster { town, hazard, severity }, Recession { depth, months }}`. A law is enacted at once (no vote) and then funded and implemented by the engine as any law; `Set` reloads the state with changed content, as a balance patch does; the shocks use `LabSectorShock` or `Engine::setup` state edits of the kind a lab command makes (a closure removes capacity and records the closure event; a disaster is released through the director's deferred-hazard queue; a recession sets the global recession and its event). **No engine hook was added** (`mandate-sim/src/lab.rs` was not needed).
- `metrics::Snapshot` (nation; every town's economy, housing, jobs by sector, approval, worries felt and measured, treasury, crises, executive party; neighbourhoods on request; the player's ward) and `metrics::Tally` (every race with incumbent, winner, runner-up, margin, turnout and the engine's factor decomposition; closures, disasters, recessions, laws with their causes; crisis spans).
- `run(content, &plan, Option<(month, Intervention)>)`, `twins(content, &plan, at, &[Intervention])` (one prefix, one fork per branch; `diff` per metric path and sample), `par_map(items, threads, f)` (deterministic order).
- CLI: `mandate-cli lab run|twin --seed --scenario --driver --months --sample --at --policy tpl@home[,p=v] --closure home:0.5 --disaster home:flood:0.8 --recession 0.1:18 --shock home:sector.retail:0.5:6 --set-at path=value --hoods [--full]`.
- Tests (`cargo test -p mandate-cli lab::`): a fork continues exactly as the original; a twin without an intervention differs in nothing and matches a straight run; the same experiment twice gives identical JSON, and parallel equals serial; every intervention moves what it touches.

A whole small world runs 20 years in about a second under the idle driver; topic runs take 1–4 minutes on three threads (sensitivity 23 minutes).

---

## 1. Policy responsiveness

**Experiment.** For each of the 15 templates in the base pack (11 local, 4 situational), each of the six worlds and 8 seeds (48 twins per template): run to month 1, fork, enact the template at its default parameters in the player's home town in one branch, run both to month 60 under the idle driver, and difference the home town at 1, 3 and 5 years. Templates with hazard protection also get a flood (severity 0.8) at month 30 in both arms. Expected signs are derived from each template's mechanisms and side effects (`audit/policy.rs`, `expectations`).

`cargo run --release -p mandate-cli -- audit policy --seeds 8`

**Year 5, home town, treated − base (mean, t in brackets).**

| Template | Funded | Unemp. pts | Jobs % | Low income % | Rent % | Homes % | Approval pts | Jobs worry pts | Services worry pts | Taxes worry pts | Races flipped |
|---|---|---|---|---|---|---|---|---|---|---|---|
| affordable_housing | 0.94 | −0.83 (−6.9) | +2.4 (10.7) | +0.5 (0.9) | −0.67 (−2.8) | +1.19 (20.2) | **−2.05 (−3.8)** | −7.4 | +1.9 | −0.6 | 9/756 |
| closure_response | 0.90 | −1.80 (−8.8) | +3.7 (15.4) | +5.0 (7.1) | +1.3 (6.1) | −0.10 | **−1.39 (−2.4)** | −16.2 | +3.4 | −0.2 | 4/756 |
| emergency_rent | 0.95 | +0.37 (3.3) | −1.2 (−7.4) | −0.2 (−0.4) | −0.72 (−4.2) | −0.11 | **−2.20 (−4.7)** | +3.3 | +1.9 | +0.4 | 5/756 |
| fiscal_bridge | 0.98 | +0.13 (1.1) | −0.5 (−3.0) | −0.7 (−1.4) | −0.18 | −0.10 | −3.48 (−7.9) | +1.2 | −0.5 | +2.3 | 8/756 |
| flood_defenses | 0.98 | +0.24 (2.2) | −0.8 (−5.1) | −0.7 | −0.99 (−5.4) | −0.08 | −1.07 (−2.6) | +2.2 | +0.4 | −0.1 | 8/756 |
| hardship_fund | 0.95 | −0.35 (−3.1) | +0.4 (2.3) | +0.6 (1.3) | −0.17 | −0.09 | **−1.00 (−2.2)** | −3.1 | +2.2 | +0.1 | 5/756 |
| local_wage_floor | 0.98 | +0.51 (4.6) | −1.4 (−8.7) | +5.6 (9.5) | −1.29 (−7.6) | −0.06 | −1.27 (−3.1) | +4.5 | +0.1 | −0.1 | 3/756 |
| parks | 0.98 | +0.08 | +0.1 | −1.4 (−3.1) | +1.74 (8.9) | −0.10 | +3.54 (5.8) | +0.8 | +0.1 | +0.5 | 4/756 |
| police_staffing | 0.98 | +0.30 (2.8) | −0.1 | −1.2 (−2.3) | +1.90 (10.0) | −0.10 | +3.43 (7.4) | +2.7 | +0.6 | +0.8 | 5/756 |
| property_tax (+0.1 pt) | 0.99 | +0.16 | −0.5 (−3.1) | −1.0 (−1.9) | −0.17 | −0.09 | −2.60 (−6.5) | +1.4 | −0.7 | +1.2 | 8/756 |
| rebuild_fund | 0.96 | −0.56 (−5.0) | +1.7 (9.9) | −0.4 | −0.52 (−2.5) | +0.89 (18.4) | −1.86 (−3.6) | −5.0 | +0.8 | −0.5 | 8/756 |
| small_business_grants | 0.98 | −0.01 (−0.1) | +0.1 (0.8) | −0.3 | +0.76 (4.4) | −0.10 | −1.69 (−3.1) | −0.1 | +0.5 | +0.2 | 10/756 |
| tech_incubator | 0.98 | +0.13 (1.1) | −0.4 (−2.9) | +0.2 | +0.0 | −0.10 | −2.49 (−6.5) | +1.2 | +0.3 | −0.1 | 4/756 |
| transit_frequency | 0.96 | +0.25 (2.4) | −0.2 | −1.1 (−2.3) | +1.43 (8.1) | −0.10 | +3.74 (7.1) | +2.3 | +0.8 | +0.7 | 5/756 |
| upzoning | 0.99 | +0.11 | −0.5 (−2.2) | −0.9 (−1.9) | −0.40 (−1.9) | **−0.05 (−0.7)** | −2.84 (−6.6) | +1.0 | −0.3 | −0.2 | 7/756 |

**Checks against what the templates say** (year 5; `checks` and `flags` in the output):

| Claim | Result | Verdict |
|---|---|---|
| housing_supply builds homes, rents ease, the housing worry eases (affordable housing, rebuild fund) | homes +1.2% / +0.9%, rent −0.7% / −0.5%, worry −0.7 / −0.5 | works (small: a 1% stock rise for 300 homes a year) |
| zoning lets private builders add homes (upzoning) | homes −0.05% (t −0.7), rent −0.4% (t −1.9) | **no effect**: private starts are stalled in most towns (topic 6) |
| sector subsidies add jobs in the sector and cut unemployment | sector jobs +2.4–2.5%, but unemployment −0.01 (grants) and +0.13 (incubator) by year 5 | weak: the gain is offset (structural adjustment and inflow) |
| transfers raise recipients' income and approval | low income +0.6% (t 1.3) hardship, −0.2% rent relief; approval −1.0 and −2.2 | **wrong sign** on approval, no measurable income |
| wage floor raises low pay, costs some jobs | low income +5.6%, jobs −1.4% | works |
| tax rate raises revenue and the tax worry | cash +157 a head, taxes worry +1.2 | works |
| services raise approval and draw people | approval +3.4 to +3.7, population +0.2%, rent +1.4 to +1.9% | works (but police has nothing to act on: there is no crime) |
| closure response retrains (labour supply down) | labour force +0.4% (t 4.0): more people arrive than leave work | wrong sign (side effect swamped) |
| flood protection reduces damage | with a flood at month 30: homes +0.31% (t 12.7) against the unprotected twin, approval −1.6, the recovery worry unchanged | works for homes, invisible in opinion |
| "Rents ease over time as supply grows" | −0.5 to −0.7% after five years | weak (true, barely felt) |
| emergency rent "freeze" holds rents | modelled only as less building (zoning −0.08 per level); no rent cap exists | described differently from what is modelled |

**Verdict: weak.** Laws reach their mechanism's indicator with the right sign in 26 of 40 checks (8 no effect, 6 wrong sign, two of which are the side effects swamped or the freeze modelled as less building), and only one effect is very large: the closure response cuts unemployment by more than 3 points in 29% of seeds (median −1.9). But (a) approval falls for 12 of 15 templates, because material gains never enter approval (`population::opinion`: the experienced term reads only unemployment change, rent growth, services, disruption and disasters) and every programme's spending raises budget stress, the "services" worry's indicator; (b) votes move in about 1% of the home town's races; (c) upzoning does nothing; (d) the home town is always of class `town` (`by_home_class`), so "town types" vary only through the scenarios.

**What would fix it.** Credit for delivered results in approval (a law's ledger deltas: jobs supported, homes built, income paid, felt by the classes and neighbourhoods they reach); a services indicator that reads service levels, not budget stress; private building that responds to vacancy (topic 6) so zoning matters; stronger approval → vote transmission (topic 2).

## 2. Believable behaviour

`cargo run --release -p mandate-cli -- audit believe --seeds 8` (six worlds × 8 seeds × 20 years; recession and closure twins at month 24).

| Relationship | Measure | Result | Verdict |
|---|---|---|---|
| Unemployment up → re-election down | re-election by tercile of the town's 12-month unemployment change minus the nation's (−0.65, 0.00, +0.70 pts) | 81.9%, 82.0%, 80.1%; executives 73.7%, 72.1%, 73.4% | **broken** |
| | linear probability per point | −0.5 pts (t −1.7) | |
| | approval per point of relative unemployment rise (town-years) | −1.45 pts (t −75.6) | works |
| | re-election per point of approval | +0.1 pts (t 4.4) | **broken** |
| Rent follows supply and demand | yearly rent growth on vacancy and on household-minus-stock growth (28,800 town-years) | −1.54 per vacancy pt (t −1567), +0.46 per pt of excess demand (t 73), R² 0.99 | works (mechanically) |
| | rent growth on wage growth | +1.44 (t 51), R² 0.08 | rent ignores incomes |
| Migration follows jobs, real wages, rent | population growth % on relative unemployment, log relative real wage, rent burden | −0.061 per pt (t −41), **−0.007 per pt (t −41, expected +)**, −0.001 per pt (t −110), R² 0.31 | weak |
| Incumbency advantage, beatable | re-election when the incumbent runs | 81% (council 82%, executives 73%); incumbents run 70%; 16% of races within 10 pts | works (above SIMULATION_MODELS' 65–80%, inside TEST_STRATEGY's 55–85%) |
| Turnout by age and class | turnout habit by band (18–29 … 65+) and class | 47, 47, 49, 53%; 47, 49, 51% | weak (seeded 42–74% by age, but habit flattens it) |
| Recessions hit sectors differently | national jobs 12 months into a 10%, 18-month recession | tourism −3.5%, manufacturing −3.1%, resources −2.7%, construction −1.5%, knowledge −1.4%, agriculture −1.1%, public +0.1%, retail +0.4%; unemployment +0.8 pts | works (retail rising in a recession is odd) |
| A plant closure shows in its neighbourhoods' worries and votes | half the home town's largest industrial sector, month 24; at +12 months | unemployment +5.9 pts, the jobs worry +26 to +64 pts by neighbourhood (town +42), approval −11.0 in every neighbourhood (−10.9 to −11.1), crisis raised in 92% | worries work; votes **broken** |
| | at +48 months | unemployment still +3.3, approval **+2.5** (the experienced term reads 12-month changes, so recovery is rewarded) | |
| | elections after it | incumbents won 233 of 378 races without, 234 with; 9 races flipped | |

**Verdict: weak.** The economy behaves believably; the vote does not respond to it. The record term (`politics/elections.rs`, `utilities`: `retrospective_weight × 0.1 × exec × media × ((gov_approval − 0.5) + retro_economy_weight × retro)`) is small beside party attachment, viability and the race shock.

**What would fix it.** Re-weight the vote so that a 10-point approval swing moves an incumbent's share by several points (raise `retrospective_weight` or drop the `0.1`; reduce `race_shock_sd`), then tune to the 65–80% band; make approval remember a closure (a level term against the national benchmark, not only the 12-month change); let rent respond to incomes; give migration a positive real-wage response (it is negative because high-wage towns also have high rents and the attractiveness formula's `ln` term is small).

## 3. Sensitivity

`cargo run --release -p mandate-cli -- audit sensitivity --seeds 6` (36 parameters × ±20%; each variant is applied as a balance patch at month 0 to the same six worlds, so race winners compare one to one; 10 years, idle; the idle and mixed bots' first-election win rates on 12 seeds under the variant content).

Base: {"debt_change": 0.01, "gov_approval": 0.554, "idle_win": 0.083, "mixed_win": 0.917, "pop_growth_pct": 1.0, "reelection": 0.811, "unemployment_median": 0.047}  noise (standard error): {"gov_approval": 0.016, "reelection": 0.023, "unemployment_median": 0.002, "win_rate": 0.144}

| Parameter | Base | −20%: Δ unemp. pts / Δ re-election pts / Δ approval pts / Δ idle win / Δ mixed win / races flipped | +20%: same | Largest effect pts |
|---|---|---|---|---|
| `economy.migration_rate` | 0.004 | -0.04 / +0.3 / +0.5 / +0 / +0 / 3.9% | +0.03 / -0.0 / -0.7 / +0 / +8 / 1.9% | 8.3 |
| `economy.target_vacancy` | 0.05 | +0.10 / +0.3 / +1.7 / +0 / +8 / 0.3% | -0.10 / -0.2 / -2.0 / +0 / +0 / 0.1% | 8.3 |
| `economy.unemployment_replacement` | 0.38 | +0.00 / +0.0 / -0.1 / +0 / +8 / 0.0% | +0.01 / +0.0 / -0.1 / +0 / +0 / 0.0% | 8.3 |
| `economy.tech_growth` | 0.02 | +0.00 / +0.0 / +0.0 / +0 / +8 / 0.0% | +0.00 / +0.0 / +0.0 / +0 / +8 / 0.0% | 8.3 (no effect at all) |
| `economy.manufacturing_trend` | -0.008 | +0.00 / +0.0 / +0.0 / +0 / +8 / 0.0% | +0.00 / +0.0 / +0.0 / +0 / +8 / 0.0% | 8.3 (no effect at all) |
| `economy.birth_rate` | 0.03 | +0.03 / +0.1 / +0.6 / +0 / +8 / 0.1% | +0.00 / +0.1 / -0.5 / +0 / +0 / 0.1% | 8.3 |
| `economy.construction_lag_months` | 18 | +0.00 / +0.0 / +0.0 / +0 / +8 / 0.0% | +0.00 / +0.0 / +0.0 / +0 / +0 / 0.0% | 8.3 (no effect at all) |
| `politics.vote.issue_weight` | 3.0 | -0.01 / +0.6 / -0.3 / +0 / +8 / 17.8% | +0.01 / -0.2 / -0.1 / +0 / +8 / 14.8% | 8.3 |
| `politics.vote.valence_weight` | 1.0 | +0.01 / -0.2 / -0.1 / +8 / +0 / 12.8% | +0.01 / +0.2 / -0.4 / +0 / +8 / 10.1% | 8.3 |
| `politics.vote.retrospective_weight` | 6.0 | -0.00 / +0.0 / -0.1 / +0 / +0 / 5.2% | -0.01 / +0.1 / -0.1 / +0 / +8 / 5.7% | 8.3 |
| `politics.vote.executive_multiplier` | 1.8 | +0.00 / -0.0 / -0.1 / +0 / +0 / 1.8% | +0.00 / +0.0 / +0.0 / +0 / +8 / 0.0% | 8.3 |
| `politics.vote.incumbent_awareness` | 0.55 | +0.00 / -0.3 / +0.0 / +0 / +0 / 7.2% | +0.03 / +0.3 / -0.1 / +0 / +8 / 5.7% | 8.3 |
| `politics.vote.challenger_awareness` | 0.12 | +0.00 / +0.0 / -0.1 / +0 / +0 / 2.2% | -0.00 / +0.1 / +0.2 / +0 / +8 / 5.2% | 8.3 |
| `politics.vote.race_shock_sd` | 0.25 | +0.00 / +2.0 / -0.2 / +0 / +0 / 20.0% | +0.01 / -1.5 / -0.1 / +8 / +8 / 17.3% | 8.3 |
| `politics.vote.national_swing_sd` | 0.2 | +0.01 / +1.0 / -0.3 / -8 / +8 / 17.9% | +0.01 / -0.7 / -0.2 / +8 / -8 / 17.4% | 8.3 |
| `politics.vote.strategic_weight` | 1.6 | +0.00 / -0.6 / -0.2 / +8 / +8 / 13.7% | -0.02 / +1.0 / -0.2 / +0 / +0 / 11.8% | 8.3 |
| `politics.vote.record_penalty` | 5.0 | +0.00 / +0.0 / -0.0 / +8 / +8 / 0.1% | +0.00 / +0.0 / +0.0 / +0 / +8 / 0.0% | 8.3 |
| `politics.vote.record_bonus` | 0.15 | -0.00 / -0.1 / +0.1 / +0 / +0 / 5.2% | -0.00 / +0.3 / -0.0 / +0 / +8 / 3.8% | 8.3 |
| `politics.vote.retro_economy_weight` | 0.5 | -0.00 / +0.0 / -0.0 / +0 / +8 / 3.5% | -0.00 / +0.1 / +0.2 / +0 / +0 / 1.9% | 8.3 |
| `politics.vote.media_base` | 0.5 | -0.00 / +0.1 / -0.0 / +0 / +0 / 3.4% | -0.01 / +0.2 / -0.2 / +0 / +8 / 3.9% | 8.3 |
| `politics.vote.home_ward_bonus` | 0.25 | +0.01 / -0.2 / +0.0 / +0 / -8 / 5.4% | +0.01 / +0.5 / -0.1 / +0 / +8 / 5.6% | 8.3 |
| `politics.turnout.habit` | 0.3 | -0.00 / +0.1 / -0.2 / +0 / +8 / 3.8% | -0.00 / +0.2 / +0.2 / +0 / +8 / 5.2% | 8.3 |
| `politics.campaign.awareness_per_canvass` | 0.18 | +0.01 / +0.0 / -0.3 / +0 / +0 / 8.7% | +0.01 / +0.3 / +0.1 / +0 / +8 / 5.7% | 8.3 |
| `politics.campaign.persuasion_per_canvass` | 0.08 | +0.02 / +0.0 / -0.0 / +0 / +0 / 5.4% | -0.00 / +0.4 / -0.0 / +0 / +8 / 3.9% | 8.3 |
| `opinion.experience.unemployment` | 8.0 | +0.00 / +0.0 / +0.0 / +0 / +0 / 0.0% | +0.00 / +0.0 / -0.0 / +0 / +8 / 0.0% | 8.3 |
| `opinion.experience.rent_growth` | 2.0 | +0.00 / +0.0 / +1.0 / +0 / +0 / 0.1% | +0.00 / +0.1 / -1.0 / +0 / +8 / 0.1% | 8.3 |
| `opinion.policy_reaction` | 2.0 | +0.00 / -0.0 / -0.4 / +0 / +0 / 0.0% | +0.00 / +0.1 / +0.4 / +0 / +8 / 0.1% | 8.3 |
| `opinion.law_reaction_weight` | 0.6 | +0.00 / +0.1 / -0.2 / +0 / +0 / 0.1% | +0.00 / +0.0 / +0.2 / +0 / +8 / 0.0% | 8.3 |
| `opinion.experience.services` | 2.0 | +0.00 / -0.1 / -2.1 / +0 / +0 / 7.0% | -0.00 / +0.1 / +1.7 / +0 / +0 / 3.7% | 2.1 |
| `politics.vote.party_weight` | 1.4 | +0.03 / -0.7 / -0.3 / +0 / +0 / 16.4% | -0.02 / +1.1 / -0.3 / +0 / +0 / 8.1% | 1.1 |
| `economy.natural_unemployment` | 0.055 | -0.82 / +0.5 / +0.3 / +0 / +0 / 2.2% | +0.84 / -0.0 / -0.6 / +0 / +0 / 1.9% | 0.8 |
| `opinion.gov_approval_rate` | 0.08 | +0.00 / +0.1 / +0.2 / +0 / +0 / 0.1% | +0.00 / +0.0 / -0.1 / +0 / +0 / 0.0% | 0.2 |
| `economy.recession_monthly_chance` | 0.011 | +0.00 / +0.0 / -0.0 / +0 / +0 / 0.0% | +0.00 / +0.0 / +0.0 / +0 / +0 / 0.0% | 0.0 |
| `politics.vote.partisan_discount` | 0.4 | +0.00 / +0.0 / +0.0 / +0 / +0 / 0.0% | +0.00 / +0.0 / +0.0 / +0 / +0 / 0.0% | 0.0 (no effect at all) |
| `politics.vote.player_approval_weight` | 1.2 | +0.00 / +0.0 / +0.0 / +0 / +0 / 0.0% | +0.00 / +0.0 / +0.0 / +0 / +0 / 0.0% | 0.0 (no effect at all) |
| `politics.turnout.standalone` | 0.55 | +0.00 / +0.0 / +0.0 / +0 / +0 / 0.0% | +0.00 / +0.0 / +0.0 / +0 / +0 / 0.0% | 0.0 (no effect at all) |

dominant: None  no effect at all: ['economy.tech_growth', 'economy.manufacturing_trend', 'economy.construction_lag_months', 'politics.vote.partisan_discount', 'politics.vote.player_approval_weight', 'politics.turnout.standalone']

How to read it. Win rates come from 12 seeds, so a ±8-point change is one seed (standard error 14 points): read the bot columns as noise unless both directions agree. "Largest effect" is therefore dominated by that noise; the races-flipped column is the reliable measure of what reaches votes, and the unemployment and approval columns of what reaches the town.

- **What decides elections:** the race shock (±20% flips 17–20% of races and moves re-election +2.0 / −1.5 points), the national swing (17–18%), the issue weight (15–18%), party weight (8–16%), strategic desertion (12–14%) and valence (10–13%). The record's own weights flip 2–6% (`retrospective_weight` 5–6%, `retro_economy_weight` 2–3.5%, `executive_multiplier` 0–2%), and the opinion parameters that turn conditions into approval flip almost nothing (`experience.unemployment` 0.0%, `experience.rent_growth` 0.1%, `policy_reaction` 0.0–0.1%, `law_reaction_weight` 0.0–0.1%), although they move approval by up to 2 points. This is finding 1 seen from the parameters: the path from conditions to votes is cut at the vote.
- **What decides unemployment:** `natural_unemployment` alone (±0.8 points); every other parameter moves it by 0.1 points or less.
- **What decides approval:** `experience.services` (±2 points), `target_vacancy` (±2, through rents), `experience.rent_growth` (±1).
- **No single parameter quietly decides everything** (`dominant`: none). The vote's noise terms together do.
- **No effect after the start:** `tech_growth`, `manufacturing_trend` and `construction_lag_months` change nothing when patched at month 0, because they are read only when the world is generated (into `national.tech_growth`, the trend and the pipeline's length); a balance patch to them reaches no running game. `partisan_discount` has no effect at all (also topic 8); `turnout.standalone` has none because the base constitution's local elections are concurrent; `player_approval_weight` has none here because the player is idle (it matters for bots: see the bots' own grid runs).

## 4. Towns diverging

`cargo run --release -p mandate-cli -- audit diverge --seeds 4` (six worlds × 4 seeds × 30 years; spread across a world's towns, averaged over worlds).

| Year | Wage CV | Rent CV | Unemp. SD pts | Industry dissimilarity | Attitude SD | Towns not run by the top party | Worry SD |
|---|---|---|---|---|---|---|---|
| 0 | 0.167 | 2.30 | 2.18 | 0.161 | 0.081 | 0.39 | 0.143 |
| 5 | 0.183 | 3.20 | 2.24 | 0.163 | 0.074 | 0.29 | 0.165 |
| 10 | 0.196 | 3.80 | 2.13 | 0.161 | 0.069 | 0.22 | 0.169 |
| 20 | 0.216 | 4.57 | 1.73 | 0.153 | 0.060 | 0.10 | 0.174 |
| 30 | 0.234 | 5.19 | 2.21 | 0.157 | 0.055 | 0.16 | 0.190 |

Rank persistence from year 0 to 30 (Spearman): population 0.97, knowledge share 0.94, manufacturing share 0.80, wage 0.75, unemployment 0.11.

**Verdict: weak.** Economies stay distinct and keep their identity (industry mix 0.98 of its start, wage spread up 40%; towns' unemployment ranks reshuffle, which is healthy). Politics converge: the attitude spread between towns falls by a third and the share of towns not run by the dominant party falls from 39% to 16%. The rent spread is the runaway of topic 6, not character.

**What would fix it.** Migrants carry attitudes (they do) but settle by adult count everywhere; sorting by neighbourhood and town character would keep places distinct. Party ID's governing credit (`party_id.governing_credit`) compounds the incumbent party's attachment; a counterweight (time-for-a-change, scandal fatigue, challenger quality from the record) would stop the lock (topic 6).

## 5. Player leverage

`cargo run --release -p mandate-cli -- audit leverage --seeds 16` (random country, 16 seeds, 6 years; each bot against the idle player on the same seed, so differences are the player's doing). Re-run after the merge (see "Re-runs after the merge").

| Driver | Class | First election won | Races won | Own laws | Ward: player approval pts | Ward: awareness pts | Ward: jobs worry pts | Ward: housing worry pts | Ward: services worry pts | Town: unemployment pts | Town: rent % | Town: approval pts |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| idle | probe | 0.06 | 0.06 of 16 | 12 | +0.0 | +0 | +0.0 | +0.00 | +0.0 | +0.00 | +0.00 | +0.0 |
| canvass | single_verb | 0.75 | 0.71 of 28 | 12 | +4.9 | +94 | +1.1 | -0.08 | +0.0 | +0.09 | -0.20 | -0.7 |
| media | single_verb | 0.06 | 0.06 of 17 | 12 | -0.1 | +0 | +1.5 | +0.05 | +0.0 | +0.13 | -0.07 | +0.1 |
| asks_only | single_verb | 0.56 | 0.52 of 25 | 19 | +2.6 | +33 | -1.2 | -0.07 | -0.0 | -0.13 | +0.12 | -1.2 |
| mixed | blind_mix | 0.81 | 0.76 of 29 | 143 | +5.1 | +79 | -5.7 | -0.25 | +0.0 | -0.68 | +1.11 | +0.5 |
| legislator | blind_mix | 0.75 | 0.75 of 28 | 133 | +18.5 | +97 | +1.0 | +0.19 | +0.0 | -0.04 | +0.85 | +1.0 |
| explorer | informed | 0.81 | 0.76 of 29 | 142 | +9.0 | +73 | -4.4 | -0.06 | +0.0 | -0.47 | +1.00 | +0.6 |
| explorer_legislator | informed | 0.81 | 0.76 of 29 | 139 | +9.7 | +97 | -5.3 | -0.22 | +0.0 | -0.58 | +0.86 | +1.6 |
| explorer_campaigner | informed | 0.81 | 0.76 of 29 | 130 | +7.8 | +92 | -0.2 | +0.17 | -0.0 | -0.12 | +0.92 | +0.9 |
| corrupt | probe | 0.75 | 0.72 of 25 | 111 | -1.0 | +57 | +0.6 | +0.03 | +0.0 | -0.00 | +0.72 | -1.7 |

Differences are against the idle player on the same seed, at month 72. "Own laws" are laws enacted with the player as sponsor, summed over 16 careers; the 12 that idle, canvass and media share are in every arm, so they are not the player's doing.

Summary: {"best_informed_first_win": 0.813, "idle_first_win": 0.063, "largest_mean_town_unemployment_move_pts": 0.677, "largest_mean_ward_worry_move_pts": 5.674} verdicts: works works

**Verdict: weak.** One seat-holder changes their own fate a great deal (6% → 75–81% first-election wins), almost all of it through awareness (canvass alone gets 75%; the ward's awareness of the player rises 73–97 points). Their effect on conditions is real but small: the busiest styles move town unemployment −0.5 to −0.7 points and the ward's jobs worry about −5 points over six years, and nothing moves the services worry. The informed styles win exactly as often as the blind mix (0.813), so learning the town does not pay in elections; it shows only in ward approval (+7.8 to +9.7 against +5.1 for the mixed bot).

**What would fix it.** Make the record term matter (topic 2), so a councillor's laws and asks are worth votes; make awareness saturate (it reaches 95–97 for canvass and legislator); let the services worry read service levels so services laws can be felt in the ward.

## 6. Stuck states

`cargo run --release -p mandate-cli -- audit stuck --seeds 8` (six worlds × 8 seeds × 30 years; 1,440 towns).

| Check | Result | Verdict |
|---|---|---|
| Town debt spiral (debt a head ×3 over 20 years, rising every year) | 0 towns | works |
| National debt-to-output change over 30 years | median −0.2 pts (range −10 to +11) | works |
| Unemployment at 25%+ for two years | 0 towns; highest town rate 21% | works |
| Unemployment under 1% for two years | 0 | works |
| Towns below 60% of their population | 0; 6 towns (0.4%) more than doubled (all one resource town) | works |
| Approval pinned below 0.1 or above 0.9 for two years | 0 | works |
| Closures without recovery after ten years (unemployment against the nation's 2+ pts above before) | 28 of 920 closures | works |
| **Rent runaway** (rent rising 4× more than the median town's) | 112 towns (7.8%); median town ×3.3 to ×11.7 in the seeds listed (seed 1: ×7.9 against prices ×1.85); worst ×179 | **broken** |
| **Private building stalled** (starts under 20% of baseline) | 43% of town-months | **broken** |
| Housing-squeeze crises lasting ten years or more | 822 (also 143 fiscal holes, 48 closures) | broken |
| **Executive party never changes in 30 years** | 547 towns (38%) | **broken** |
| The party running most towns keeps the lead | median 93.5% of years | weak |

A single run shows the rent runaway: `cargo run --release -p mandate-cli -- lab run --seed 1 --driver none --months 360 --sample 0,120,240,360`. At month 0 the median rent is 666 and the highest 18,223 (burden 3.9× income); at year 30 the median is 6,636 (burden 0.34) and the highest 2.46 million (burden 196×); vacancy is below zero in 25 of 30 towns at year 20; median starts are 10% of baseline at month 0 and zero at year 10.

**Cause.** `economy/housing.rs::step`: rent grows by `gap × 0.12 + inflation/12` a month while vacancy is below target, with no ceiling from incomes; vacancy is clamped at −5%, so a short town compounds at about 1.3% a month forever. Private starts depend only on price over build cost (`clamp(1 + (profit − 1/0.85) × 4, 0, 3)`), and build cost tracks the price level; where prices lag, starts are zero and stay zero. Households leave too slowly (migration's rent-burden response is −0.001% per point).

**What would fix it.** Cap rent growth by affordability (rent burden pulls rent back, households double up or leave); let starts respond to vacancy and rent, not only to price over cost; a much stronger rent-burden push in migration; a sweep band on median real rent growth and on the rent-burden maximum. For parties: see topic 4.

## 7. Readability

`cargo run --release -p mandate-cli -- audit readability --seeds 6` (six worlds × 6 seeds × 20 years; ledger and closure twins on the same worlds).

**Large changes in baseline worlds, and what the engine shows.**

| Change | Cases | Surface that names a cause | Share explained |
|---|---|---|---|
| Town unemployment up 3+ pts in a year | 307 | a town crisis 284, a disaster 15, a closure event 2; only a national recession 2; nothing 4 | 98% |
| Real rent up 10%+ in a year | 2,418 | a housing-squeeze crisis 1,793; a disaster 23; nothing 602 | 75% |
| Incumbent lost | 5,145 | largest factor in the breakdown: shock 2,878 (56%), party 1,042, issues 1,017, valence 137, record 58, campaign 13 | names luck |

**The bill's "Why?" ledger against the twin (month 36, 36 twins each).**

| Template | Ledger says | Twin shows | Twin / ledger |
|---|---|---|---|
| affordable_housing | 581 homes built | +145 homes | **0.25** |
| rebuild_fund | 451 homes built | +150 homes | **0.33** |
| small_business_grants | 684 retail jobs supported | +556 | 0.81 |
| tech_incubator | 260 knowledge jobs | +187 | 0.72 |
| closure_response | 415 manufacturing jobs | +254 | 0.61 |
| local_wage_floor | +10.0% low wages | +6.5% low income | 0.65 |

**Races flipped by a planted closure** (43 flips across 36 twins): in the 4 comparable ones the factor that moved most was party (3) or shock (1), never record.

**Causal spine** (events in the tally with a recorded cause): laws 100%, plant closures 29% (those caused by a recession), disasters, recessions and scandals 0% (exogenous by design).

**Verdict: weak.** Jumps in unemployment and rent are almost always attached to a visible crisis, which is good. Election night tells the player that losses are mostly luck, and in the engine they are (topic 2). The housing ledger counts homes started by the programme, not homes added net of the private building it displaces and of demolition.

**What would fix it.** A ledger entry computed against a counterfactual estimate (S5's impact estimate, or the policy's marginal effect on starts); once the record term matters, the decomposition will name it; split "shock" into named components the player can see (the national swing, the race's own noise).

## 8. Claims check

`cargo run --release -p mandate-cli -- audit claims --seeds 8` (16 mechanisms, each switched to neutral as a patch at month 0 on the same 8 worlds, 10 years, 5,313 local races).

| Mechanism (where claimed) | Switched off by | Races flipped | Δ re-election pts | Δ approval pts | Status |
|---|---|---|---|---|---|
| Directional voting with polarisation, β (SIMULATION_MODELS §1) | `beta_high = beta_low = 0.15` | 0 | 0 | 0 | **no effect at all** |
| Benchmarked retrospective economy (§2) | `retro_economy_weight = 0` | 342 (6.4%) | +0.2 | +0.1 | used |
| Executive weighted for record (§2) | `executive_multiplier = 1` | 121 (2.3%) | 0 | 0 | used, small |
| Media weight by the town's share (§2) | `media_town_share = 0` | 286 (5.4%) | −0.1 | −0.1 | used |
| Partisan perception filter (§3) | `partisan_discount = 0` | 1 | 0 | 0 | **no visible effect** |
| Party ID drifts with the town hall's record (§3) | `governing_credit = 0` | 128 (2.4%) | 0 | −0.1 | used, small |
| Turnout habit (§4) | `turnout.habit = 0` | 538 (10%) | +0.1 | −0.5 | used |
| Strategic desertion of weak parties (DEC-071) | `strategic_weight = 0` | 1,973 (37%) | −7.2 | −0.1 | used, large |
| Home-ward bonus (DEC-028) | `home_ward_bonus = 0` | 771 (15%) | −1.7 | −0.2 | used |
| Rent growth in approval (opinion) | `experience.rent_growth = 0` | 23 (0.4%) | +0.4 | **+5.3** | moves approval, not votes |
| Services in approval (opinion) | `experience.services = 0` | 397 (7.5%) | −1.5 | **−10.9** | used |
| Disruption in approval (opinion) | `experience.disruption = 0` | 4 | +0.1 | +0.6 | moves approval, not votes |
| Laws' signatures in approval (opinion) | `law_reaction_weight = 0` | 83 (1.6%) | 0 | −0.8 | used, small |
| Neighbourhood job-loss tilt (local concerns) | `jobs_sector_tilt = 0` | 2 | −0.1 | −0.2 | worries only |
| Gravity migration (§9) | `migration_rate = 0` | 853 (16%) | +0.2 | +1.8 | used; unemployment Δ 0.008 pts |
| Polls' shared error (§6) | `polls.shared_sd = 0` | 0 | 0 | 0 | polls only (by design) |

**From reading the code** (file and function named so each is checkable):

| Claim (source) | Status | Evidence |
|---|---|---|
| One labour market per town, no commuting (known gap) | confirmed | `economy::MuniEcon` holds jobs and labour force per town; `markets` matches them within the town |
| Rent per town only (known gap) | confirmed | `economy/housing.rs::Housing` lives on `MuniEcon`; neighbourhood rent is a worry multiplier (`opinion.local_concerns.rent`) |
| No moves between neighbourhoods except disasters (known gap) | confirmed, and wider | `population::settle` spreads arrivals by adult count; movers are only the two youngest bands; displacement is the only within-town move |
| Incumbency emerges from fewer credible challengers who read the polls (SIMULATION_MODELS §2) | not modelled | `politics/elections.rs`: sitting members run with a fixed 82% chance; every party above the threshold fields a candidate |
| Turnout from duty (age, trust), cost (weather, hazards, distance), importance, closeness (§4) | partly | `population::seed_turnout` is fixed by age and class; habit and concurrency are modelled; no cost, closeness or trust term |
| Cohort `trust` | decorative | written at seeding and mixed on moves; never read (DEC-095's rule against decorative attributes) |
| Retrospective "plus local rent growth relative to inflation" in the vote (§2) | moved | rent growth enters approval, not the vote's retro term |
| Gravity γ ≈ 1.2 (§9) | modelled at γ = 1 | `population::migration` (comment says γ = 1) |
| NPC whips seek a margin and counter-lobby (§7) | modelled | `npc.rs`, `insurance_votes` |
| Police staffing (template, "Tough on crime") | no target | there is no crime indicator; it acts only as a service and through its signature |
| Transit: "less traffic downtown" | not modelled | no traffic or congestion |
| Tech incubator: "rents near the incubator" | not modelled | no neighbourhood rent |
| Upzoning: "a building boom adds construction jobs" | not reached | private starts stalled (topic 6) |
| Property tax: "renters see it later in rents" | not modelled | `property_tax_delta` feeds revenue and the tax worry only |
| Fiscal bridge: "the town's borrowing costs ease" | not modelled | `finance::accrue` uses fixed spreads by tier |
| Rebuild fund: "builders are scarce; other projects wait" | not modelled | no construction capacity |
| Emergency rent "freeze" | modelled as less building | zoning −0.08 per level; no cap on rents |
| Pollution dilemma: an inspection | no material effect | the options change opinions and approval only; pollution is regional and drifts with industrial jobs (`environment::drift`); no policy reduces it |
| "Services" worry | reads budget stress | `concerns.jsonc`: indicator `budget_stress`; a services law raises it |
| NPC turns everywhere | home town every turn, others every fourth | OVERVIEW §4.2; the home town is simulated more actively than the towns it is compared with |

**Verdict: weak.** Most of the vote model's terms are used and visible. Directional voting is dead (polarisation never exceeds `polarization_low`, so β is always `beta_low`), the partisan filter has no visible effect, `trust` is decorative, and nine player-facing texts describe effects the engine does not compute.

---

## Proposed fixes

Ranked by how much they would help play. Size: small (a day, a parameter or a formula), medium (a few days, one system), large (a new mechanism and its decision).

| # | Fix | Expected effect | Files | Size | When |
|---|---|---|---|---|---|
| 1 | **Make the record decide votes.** Drop the `0.1` in the record term or raise `retrospective_weight`; reduce `race_shock_sd`; add a benchmarked level term (not only the 12-month change) | Re-election gap between bad and good towns ≥ 10 pts; a closure costs incumbents seats; "record" appears on election night | `mandate-sim/src/politics/elections.rs` (`utilities`), `content/base/politics.jsonc`, `opinion.jsonc` | small (tuning against the gate) | M6 |
| 2 | **Bound rents and restart private building.** Rent growth pulled back by burden; starts respond to vacancy and rent; stronger rent push in migration | No runaway towns; housing-squeeze crises end; upzoning works | `mandate-sim/src/economy/housing.rs`, `economy/mod.rs` (attractiveness), `population/mod.rs` | medium | M6 |
| 3 | **Credit delivered results in approval.** A law's ledger deltas (jobs supported, homes, income paid) enter the experienced term for the classes and neighbourhoods they reach; services worry reads service levels | Transfers, homes and jobs raise approval; services laws ease the services worry | `population/mod.rs` (opinion), `concerns.rs`, `content/base/concerns.jsonc` | medium | M6 |
| 4 | **Break one-party lock.** Cap governing credit's compounding, add a time-for-a-change term, challenger quality from the incumbent's record | Executive party changes in most towns within 30 years; top party below 80% of years | `population/mod.rs` (`update_party_id`), `politics/elections.rs` (nomination) | medium | M6 |
| 5 | **Honest ledgers.** Homes net of displaced private starts; label "started" vs "added" | Ledger / twin within 0.5–2 for every template | `policy/mod.rs` (`implementation`), protocol ledger keys and Fluent | small | M6 |
| 6 | Neighbourhood approval follows neighbourhood conditions (the experienced term per neighbourhood, from its worries) | A closure costs the incumbent in the estate by the plant, not everywhere | `population/mod.rs` | small | M6 |
| 7 | Remove or wire dead mechanisms: β thresholds to the polarisation actually seen; trust into turnout (duty); partisan discount size | Each described mechanism moves something | `politics/elections.rs`, `population/mod.rs`, `politics.jsonc` | small | M6 |
| 8 | Correct the texts the engine does not back (traffic, incubator rents, renters' rents, borrowing costs, builder scarcity, rent freeze, inspection) or model them | No claim without a mechanism | `content/base/locale/en/*.ftl`, `policies/*.jsonc`, `dilemmas.jsonc` | small (texts) / large (models) | M6 texts, M7 models |
| 9 | Migration: positive real-wage response, mobile bands beyond 18–44, neighbourhood choice on arrival | Movers follow jobs and pay; places sort | `population/mod.rs`, `economy/mod.rs` | medium | M7 |
| 10 | Commuting layer and neighbourhood rents (the known gaps; needs its own decision) | Jobs and rents by neighbourhood | new system | large | M7 |
| 11 | Turnout: duty, cost and closeness; steeper age gradient | 18–29 vs 65+ gap of 20+ pts | `population/mod.rs`, `politics/elections.rs` | small | M7 |
| 12 | A crime indicator for police staffing to act on | Police has a target | new indicator, `concerns.jsonc` | large | M7 |

## Nightly gate (proposed)

Numbers the audit can compute every night, with thresholds that hold once fixes 1–4 land. Today seven of the eight fail.

| Check | Command | Threshold | Today |
|---|---|---|---|
| Retrospective voting | `audit gate` (believe core) | re-election gap between the lowest and highest tercile of relative unemployment change ≥ 10 pts | 1.8 pts |
| Approval reaches votes | `audit believe` | re-election per approval point ≥ +0.5 pts | +0.1 |
| No wrong-sign policy | `audit gate` (policy) | no template with a significant wrong-sign check (3 seeds, random country, 3 years) | 2 templates |
| Rent sanity | `audit stuck` | no town's rent rising 4× more than the median town's; median real rent growth within ±50% over 30 years | 7.8% of towns; ×4.3 real |
| Party turnover | `audit stuck` | executive party never changed in 30 years ≤ 20% of towns | 38% |
| Incumbency band | `audit believe` | re-election when running in 65–80% | 81% |
| Ledger honesty | `audit readability` | every ledger within 0.5–2× its twin | 4 of 6 |
| Rent and migration signs | `audit gate` (believe core) | rent coefficients have their signs; migration ≥ 1 of 3 expected signs | pass |

**Wired now (non-blocking):** `.github/workflows/nightly.yml`, job `bots`, step "Simulation audit gate (non-blocking)": `mandate-cli audit gate` runs the two cheapest checks (believe core: six worlds × 4 seeds × 12 years; policy: 15 templates × 3 seeds × 3 years; about 40 seconds on three threads) and exits non-zero when either fails, with `continue-on-error: true` until fixes 1–3 land.

## Re-runs after the merge

Before handing back, `git merge claude/bold-brown-shyeug` reported "Already up to date": the other agents' changes to the party-line rule and the asks had not reached that branch (its head is `5d66329`). Topic 5 was re-run anyway (`audit leverage --seeds 16`) and its JSON is byte-identical to the first run, as determinism requires. **Topics 1 (policy), 2 (the retrospective and closure rows), 5 (leverage) and 7 (the election-night rows) should be re-run once those changes land**: the party-line rule changes how councils vote and so which laws pass, and the asks change what the bots do and what approval the player earns. The commands above regenerate every table.

## Limits

- Small worlds only (the size the bots play); medium and large worlds were not audited.
- Policies were enacted at their defaults, at month 1, without a vote and without a sponsor; a sponsor's record and the vote itself are not part of the measured effect.
- The home town is always of class `town`, so policy effects by settlement class could not be measured; scenarios supply the variety.
- Sensitivity ran one parameter at a time; interactions are not measured. Bot win rates under a variant use whole games with that content (including the history), so they mix the rule change with a different world.
- Readability measures what the engine records (events, crises, ledgers, the decomposition), not what the UI chooses to show.
