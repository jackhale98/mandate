# The impact estimate (S5)

Before a vote, a bill says what it will probably do, to whom, by when, at what cost, and how sure anyone is. After the vote, the account of what it did uses the same quantities. Shared mechanism S5 (IMPLEMENTATION_PLAN §2), built in W4 with F3; it answers play note 10 ("vote on impact, not just approval") and replaces the audit's "ledger honesty" row.

**Status:** design, 2026-11-24. Nothing here is built. The decision record is proposed in §12; the owner's questions are in §11.

**Why now.** The audit found the game's own claims unreliable: the homes ledger says 581 where the twin shows 145 (topic 7); nine texts describe effects the engine does not compute (topic 8); twelve of fifteen laws lower approval whatever they deliver (topic 1). An estimate bolted onto that would repeat it. So this design is three things that must agree: a number before, an account after, and a lab gate that checks both against twins.

## 1. What an estimate is

One engine function, `policy::impact::estimate(subject, at)`, returns a fixed set of outcomes over fixed horizons for fixed audiences, each as a range. The label on screen is "What it does" (the style bible bans "impact" as a verb in text).

**Subjects.** A bill before a vote; a draft or a family proposal with its choices (F3's preview); a budget line (a service level changed); a works project (start, resume or clear); an arc's decision option, when its effects enact or fund something. All five reduce to a list of mechanisms with parameters, an area and a cost, which is what the estimator reads.

**Outcomes** (eight, all quantities the simulation computes each month and the lab already samples in `metrics::Town`):

| Outcome | Unit | For whom |
|---|---|---|
| `target`: the mechanism's own count (homes started, sector jobs supported, money paid, service level, tax points, protection) | the mechanism's unit | the bill's area |
| `jobs`: jobs in the jurisdiction, net | count and % | jurisdiction; the sector named when one is targeted |
| `homes`: homes added, net of private building displaced | count | jurisdiction |
| `rent`: typical rent | % | jurisdiction |
| `low_income`: income per adult on low incomes | % | the low class; middle and high when a mechanism names them |
| `budget`: yearly cost net of revenue, share of the budget, debt added | currency, % | the jurisdiction's treasury |
| `services`: service level by budget category | % | jurisdiction |
| `approval`: approval of the government, as people will feel the law | points, and a felt gain −1..1 | jurisdiction, each district, each neighbourhood, each class, older residents |

- **Horizons:** 12, 36 and 60 months after enactment, the lab's own samples. The view marks the last one that falls before the next election of the body voting on it ("in time for the election").
- **Only what is simulated, where it is simulated.** There is one labour market and one rent per town, so jobs and rent are never split by ward. The by-district picture is the felt gain, works, disruption and hazard protection, which the engine does hold per neighbourhood. Classes are the three the economy has; "older residents" is the grievance primitive's age band. Nothing is apportioned for show (DEC-095).
- **A range, never a point.** Every outcome but the cost is `lo..hi` (the middle 80% of what the lab saw), with a direction when the range excludes zero and "no clear change" when it does not.

## 2. How it is computed

**Decision: a model of the bill's mechanisms, multiplied by pass-through coefficients the lab learns from twins and ships as content.** Options (b) and (c) combined; a fork at the moment of asking (a) is used only offline, as the judge.

1. **Direct quantities, by arithmetic.** For each mechanism the estimator computes what the law does directly, with the expressions `mods_for` and `implementation` already use, moved into shared functions: homes started (units × ramp × funded share), sector jobs supported (capacity × elasticity × rate), money paid by class, the wage uplift, the service gain, the starts multiplier, tax points, protection by hazard, yearly spend and the share borrowed. The ramp is `implementation_months ÷ (0.5 + competence)`. The funded share is finance's own rule read as a pure function: programmes may take a quarter of the budget, beyond which all are pro-rated; so the estimate also says when a new bill dilutes the programmes already running.
2. **Pass-through, from the lab.** `content/base/impact.jsonc` holds, per mechanism kind, outcome, horizon and state band, the 10th, 50th and 90th percentile of (outcome change ÷ direct quantity) across twins. Bases and outcomes are shares of the town (homes started ÷ stock, spend ÷ budget), so one row serves any town size. Bands are conditions in the one condition language on public indicators: housing tight, normal or slack (vacancy against target); labour slack or tight (unemployment against the natural rate); programmes under or over the quarter cap. A row for the basis `spend` carries what all spending does (budget stress, programme jobs).
3. **Composition.** An outcome is the sum over the bill's mechanisms of coefficient × basis; the range adds deviations in quadrature (`mandate_core::math::sqrt`). The table is keyed by mechanism kind, not by template, so family options and arc options compose with no table of their own. Where the gate shows a composition is not additive, a template may carry a correction row.
4. **Approval** is the engine's own reaction read forward: the signature against each cohort's attitudes, grievances by class and age, disruption on the bill's area while it builds, the service term, and (after audit fix 3) credit for delivered results where they reach. `policy::impact::felt(h)` gives the felt gain per neighbourhood; fix 3 and V1 call the same function with delivered quantities in place of planned ones (§8).

**Why not the fork.**
- *It is a spoiler.* A fork runs this world's keyed future. Flood defences show nothing without a flood and +0.31% homes (t 12.7) with one at month 30; a fork would show the second only when the held flood is coming. Opening a bill would become a way to read the director's queue and the next recession.
- *It is one draw.* The closure response cuts unemployment by more than 3 points in 29% of seeds (median −1.9). The honest figure for a player is the spread, which the table stores.
- *It is slow.* Five years in two arms is ten simulated years: about 0.5 s native, and by the arcs design's factor of three about 1.5 s in WASM on a mid phone, per bill, per slider move.

**Why not the mechanism model alone.** Its first-round figure is the ledger: 581 homes against 145 (0.25), 451 against 150 (0.33), jobs at 0.61 to 0.81, the wage floor's +10% arriving as +6.5% of low income. Small-business grants add 2.4% to their sector and −0.01 points to unemployment at year 5; the incubator costs 0.4% of all jobs. Those second-round facts are what the coefficients carry.

**Why not a table per template and town type alone.** Parameters are continuous (50 to 2,000 homes), families multiply the cases, and the audit's home town is always of class `town`. Coefficients per mechanism scale with the parameter and the town's state.

**No hidden information.** The estimator takes an `ImpactInputs` struct built by one function, not `&State`. It holds the bill, the jurisdiction's published statistics (population by class and neighbourhood, vacancy, unemployment, sector capacity, budget, programme costs, the agency's competence), cohort attitudes, and the table. It cannot see the director, the deferred-hazard queue, hidden facts, discovery dates, the seed or any stream. Hazard protection is stated as a conditional on a reference event ("in a flood the size of a one-in-twenty-year one"), with the area's public exposure, never a chance of it striking this term. A test changes the seed, the held hazards and the hidden facts of a state and requires byte-identical estimates.

**Deterministic and identical on native and WASM.** A pure function of state and content: no RNG of any kind, floats through `libm`, ordered maps. `scripts/wasm-parity.mjs` gains a digest of every template's estimate in the home town at the end of its run.

**Cost.** About a hundred multiply-adds for the outcomes, and one pass over the area's cohorts for approval (tens of neighbourhoods × cohorts × attitudes: a few thousand products). Budget: 0.2 ms native and 2 ms in WASM on the reference phone at the 95th percentile per bill; 10 ms for an office view with eight bills. Measured three ways before the web task ships: `mandate-cli lab impact bench` (10,000 estimates, p50 and p95), `node scripts/wasm-bench.mjs impact` on the built module, and a `?dev=1` button that times a hundred queries with `performance.now()` on a real phone. The number goes in the progress entry. Calibration is the dear part and runs offline (§3).

## 3. Honesty

**Calibration.** `mandate-cli lab impact calibrate` runs twins for every template and family preset at its default and maximum parameters, in the six worlds × 16 seeds (seeds 101–116), enacted at months 1 and 61 in the home town and in one town of each other class, to 60 months. It fits the coefficients and writes `impact.jsonc`. By the audit's run times that is one to two hours on three threads (an estimate; task 2 measures it). It is run by hand when a mechanism or the economy changes; the nightly gate says when that is.

**The gate: `mandate-cli audit estimate`** (the "estimate honesty" row, replacing "ledger honesty"). On fresh seeds (1–8), for every template and every family preset: take the estimate exactly as the best-informed player would see it at enactment, enact, twin, and compare at 12, 36 and 60 months.

| Outcome | Passes when |
|---|---|
| `target` | within 0.8–1.25 of the ledger's direct count (it is the same arithmetic; this catches drift) |
| `budget` (yearly cost) | within 10% |
| `homes`, `jobs`, debt | the median twin ÷ the range's midpoint is within 0.67–1.5 |
| `rent`, `low_income`, `services` | the midpoint is within 1 point, or half the twin's mean, whichever is larger |
| `approval` | the stated direction is right in at least 80% of cells; estimated and measured felt gain by neighbourhood rank alike (Spearman ≥ 0.5) |
| every range | at least 60% of single twins fall inside it, and no template × outcome × horizon cell is below 50%: right more often than not |

Four rules across all outcomes:
- **No wrong sign.** No cell where the estimate states a direction and the twins show the opposite at |t| ≥ 2.
- **No silent effect.** Any twin effect at |t| ≥ 3 and above half a point (or half a per cent) that the estimate does not list fails the gate. This is how the incubator's lost jobs and the rent rise behind every service law get onto the bill.
- **Not vacuous.** A count whose `hi ÷ lo` exceeds 3 at full knowledge is shown in words, with no number.
- **Calibrated.** Across everything shown, 70–90% of ranges contain the twin: neither wishful nor padded.

A three-seed, three-year cut joins the existing `audit gate` in CI (about 40 seconds); the full gate is nightly. The gate tolerates, so regenerating the table is never forced by a byte difference.

**The ledger afterwards (audit fix 5).** The ledger keeps two kinds of entry, named apart:
- **Direct, bookkept exactly:** `homes_started`, `jobs_supported_<sector>`, `paid_<class>`, `service_gain`, `spent`, `borrowed`. These are what happened, with the funding and progress the law actually got. `homes_built` is renamed; counts that accumulate and levels that do not are marked as such.
- **Added, estimated:** `homes_added`, `jobs_added`, `low_income_change`: the direct entry × the same coefficient the estimate used, at the law's age, as a range. The spine does not attribute continuous effects at run time (DEC-108), so "added" is always labelled an estimate.

At enactment the engine freezes the estimate on the law (§9). "Why?" then shows three columns in the same units: estimated then, started, added since. The usual gap is explained by a direct fact: funded at 60%, so fewer started.

## 4. Uncertainty and knowledge

**Never gated** (DEC-091): the cost, who pays, when it starts, and the kind and direction of every effect, trade-offs included. A newcomer sees "more homes; fewer built privately; streets dug up" with no figures.

**Gated: the size.** Two things decide how wide a range is and how it is worded:

| What the player has | Level | Sizes shown as |
|---|---|---|
| nothing but the bill | unknown | words only ("some", "many"); claims, attributed |
| a claim heard (the sponsor's pitch, the party's research, the press) | heard | the claimant's figure, in their name |
| the papers circulated to the body's members, or the press report of them | known | the lab range widened by half and rounded to one significant figure |
| the statistician's note, read at a place with `town_data_detailed` (dated; kept in the notebook) | detailed | the lab range, two significant figures |

- New subject `Subject::BillImpact { at, policy }`, keyed by `Jurisdiction`. Budget lines also read `budget_papers`.
- **Areas.** A district's or neighbourhood's felt gain is as sure as the worse of `PolicySupport` and that neighbourhood's `HoodConcerns`. A neighbourhood the player has canvassed is shown with its size; one they have not is shown as a direction; one they have never heard of is not placed (note 11).
- **Coarse, never biased.** A wider range is the true range rounded outwards on a coarser grid, so the truth stays inside and the midpoint leaks nothing finer than the grid.
- **The sureness word** follows from the level: *firm* for arithmetic (the cost, direct counts), *fair* at detailed, *rough* at known. Below that there is no size to be sure of, and the line names the claimant.

**Claims are sources with a slant.** When a bill is tabled, up to four claimants each make one claim on its lead outcome: the sponsor, the player's party, the largest party whose line is against, and the paper. A claim is the unslanted estimate pushed towards the claimant's side: `claimed = mid + side × (1 − candour) × (hi − mid)` for a benefit, mirrored for a cost. `side` is +1 for the bill and −1 against; `candour` is the person's integrity (a party: its leader's; the paper: its outlet's own number). Below a candour of 0.3 a supporter quotes the direct count as if it were the result: "900 homes". No draw is involved, so a claim is a fact about who said what.

**Who turns out to be right.** At each horizon the engine scores every claim against the account: held up, overstated or understated. The score is an event on the bill's thread (cause: the claim), shown in "Why?", remembered by the people involved, and after two scored claims the player knows that person's candour ("tends to oversell"). The statistician is scored too; the gate is what keeps them right.

## 5. Who decides how members vote

**Decision: members read it.** A member weighs what their district stands to gain, beside values, the party line and deals. Votes then follow impact, a member's no has a reason the player can act on, and widening a bill's area to a member's district buys their vote for a cause the simulation can see.

The rule, beside the party-line rule in `vote_lean`:

```
gain_d = Σ v_h × felt_h ÷ Σ v_h              over the member's district d
lean  += council.district_gain × pragmatism × gain_d     when |gain_d| ≥ council.gain_band
```

- `felt_h` is the unslanted felt gain of neighbourhood h (−1..1) at the last horizon before the member's next election, 60 months for one not standing again; `v_h` is its likely voters. Members are locals with staff: they read the full estimate (contracts question 9).
- A member with no district, and the signer, use the whole jurisdiction.
- Values in `council.jsonc`: `district_gain` 0.35 (the weight ward support has), `gain_band` 0.05. Ideologues (low pragmatism) barely move. Earmark terms stay as they are in this pass.
- The reason is a fact with a key: `vote-reason-district-gains` or `-loses`, with the district and the outcome that weighs most. The whip count shows it for members the player knows.
- **State:** nothing saved. `gain_d` is a pure function of state and content, recomputed at the vote and for whip counts, so shown odds are rolled odds. The table is content, in the content hash; a new table is a rules update like any balance patch.
- **Order:** after audit fix 3, behind `district_gain: 0` until tuned.

**The audit re-checks:** `audit policy` (pass rate per template: none never passes, none always; the year-5 table); a new row "votes follow impact" (yes-rate at least 10 points higher in the top tercile of `gain_d` than the bottom, within one party); party unity stays at or above 75%; the bots' ladder and `good_styles_vary`; the odds calibration gate; the DEC-095 check that pragmatism still moves votes.

## 6. What the player sees

**The bill view on a phone.** Four lines under the bill's name, then ward and party as today:

1. **What it does:** the lead outcome at the horizon in time for the election, with the direct count beside the net one.
2. **Cost:** yearly cost, share of the budget, where the money comes from (rates, borrowing, a levy, or squeezing other programmes).
3. **Who gains, who loses:** up to two groups or places each way; "Show on map" opens the layer.
4. **When, and how sure:** when building starts and when it is in full effect; the sureness word and the source.

A tap opens the full sheet: every outcome at the three horizons, the claims side by side, the trade-offs, and "your record" (V1: what a yes or a no adds to your credit, by neighbourhood). Quick mode and the List view show the same four lines as text and the sheet as tables.

**On the town map** (S6 layer `bill_impact`, note 11): districts shaded by felt gain on a two-ended scale with a pattern for loss as well as colour (DEC-051); the bill's works pinned where they would stand; disruption hatched while building; each member's portrait on their district with their lean. Districts the player does not know stay blank. The list alternative is a table of districts: gain or loss in a word, the member, their lean.

**Sample text** (fills are illustrative: the audit's default-parameter results where it has them, invented budget shares and names where it does not).

*Affordable homes, 300 a year.*
> About 580 homes started in three years. Between 110 and 190 more than { $town } would have had: private builders start fewer.
> Costs 12.6m a year, 9% of the budget, from the rates. Other programmes drop to 88% funded.
> Gains: people on low incomes, and about 2% more jobs while it builds. Loses: older home-owners near the sites. Eastfield is dug up for a year.
> First homes in 18 months. Rents: no clear change in five years.
> Fair estimate. The town statistician's note, read 4 March.
> { $sponsor } says 900 homes in three years. { $party } research says 150 the town would not have had anyway.

*Local wage floor, 10%.*
> Low pay rises 10%. Incomes on low pay end 3% to 8% higher after five years.
> Between 1 and 2 jobs in every 100 go, most in shops, cafés and on farms.
> Costs the town 2 a head to enforce. No borrowing.
> Bites within six months. Rents ease by about 1%.
> Rough estimate. The papers sent to members.

**Fluent keys and arguments.** The engine names keys and gives typed arguments; every branch is a whole sentence; horizons are headings, never spliced into a line.

| Key | Arguments |
|---|---|
| `impact-<outcome>-<up\|down\|flat>` for `homes`, `jobs`, `rent`, `low-income`, `services`, `approval`, `debt` | `form` (`range`, `about`, `words`), `lo`, `hi`, `place`; `sector` or `category` where the outcome has one |
| `impact-target-<basis>` (`homes-started`, `jobs-supported`, `paid`, `wage-uplift`, `service-level`, `tax-points`, `protection`) | `n`, `place`, `sector`, `class`, `hazard` |
| `impact-cost`, `impact-paid-<rates\|borrowing\|levy\|squeeze>` | `amount`, `share`, `funded` |
| `impact-gains-<group>`, `impact-loses-<group>` (`low`, `middle`, `high`, `older`, `area`) | `place`, `size` |
| `impact-when-<building\|full\|ends>`, `impact-horizon-<y1\|y3\|y5>`, `impact-in-time` | `months` |
| `impact-sure-<rough\|fair\|firm>`, `impact-source-<own\|sponsor\|party\|rival\|press\|papers\|statistician>` | `person`, `date` |
| `impact-claim`, `impact-claim-<held\|overstated\|understated>` | `who`, the claimed line's key and arguments |
| `account-<basis>-started`, `account-<outcome>-added`, `account-gap-<funding\|progress\|repealed\|stalled>` | `n`, `lo`, `hi`, `funded` |
| `vote-reason-district-<gains\|loses>` | `district`, the outcome's key |

**After the vote.** "Why?" on a law walks the spine for what happened (tabled, amended, the vote, the signature) and lays the ledger beside the frozen estimate: estimated then, started, added since, and each claim's score. One vocabulary before and after.

## 7. The ward view, fixed

"+20" was net support: 60% for less 40% against, on a scale nobody was shown. The view now says:

> { $district } leans for. About 60% of likely voters for, 40% against.
> Fair estimate: you have canvassed Eastfield and the Quays. Canvass Hillcrest to firm it up.

- **The word** (`ward-lean-*`, seven steps) comes from for minus against.
- **The two numbers count people.** `for` is the turnout-weighted share of the district's adults in cohorts whose attitudes align with the bill; `against` the share that do not; cohorts within ±0.05 are split between them. They sum to 100. Today's `(1 + net) × 50` is a rescaled mean and counts nobody; it goes.
- **Certainty is how much of the district the player knows.** *Rough:* under half its likely voters live in neighbourhoods known to the player; shown as a band of two in ten ("between 5 and 7 in 10 for"). *Fair:* half or more, or the town's support for the policy is known; to 5 points. *Exact:* every neighbourhood detailed, or an internal poll read this month at a place with `internal_polls`; to the point, with its date.
- **Canvassing** a neighbourhood raises it to known (as `learn_hood` does today), which moves the district towards fair; the view names the neighbourhood that would help most.
- **Tier-generic:** the district is the constituency of the post the player holds in the body voting on the bill, in the constitution's word for it. A player with no district there sees the jurisdiction's figure. Nothing reads `player.ward` or `player.municipality`.

## 8. Ties

- **Laws that pay off (audit fix 3).** Approval credits delivered results through `impact::felt` with the ledger's direct entries: the same function, coefficients and reach as the estimate. A hardship fund then raises approval among those it pays, and the estimate said so first. S5's approval rows are calibrated only after fix 3 lands.
- **Policy families.** F3's `OptionPreviewView` gains `impact`, the four-line summary for the package with that option swapped in (at most nine compositions a query). F4 adds no estimate data: mechanisms compose; validation rejects a mechanism kind with no table row. F5's coverage report gains the honesty gate per preset. F6's sheet shows the lead line per option. F7's need is the table's bands: a slack housing market passes less through.
- **Arcs (DEC-109).** A decision option whose `effects` enact, fund or cut something carries an estimate from the same call; options that only move opinions keep their trade-off chips. An arc never invents a figure.
- **Credit and blame (V1).** Credit is a share (sponsor whole, voters less, by the margin) of the delivered felt gain per neighbourhood. "Your record" on the bill view is that rule applied to the estimate.
- **Works you can see (DEC-099).** A works project is a subject: its site is pinned on the layer; a stalled site's account reads "started, not delivered"; resume and clear are options with estimates.
- **The strategy map.** Bots read the estimate through `Knowing`, under the same gating. The quality-diversity grid gains a descriptor (share of votes cast with the district's gain) and a utility weight on estimated gain, so the map shows whether voting on impact is a living style, and `knowledge_matters` has something to measure: a better-informed bot sees narrower ranges.

## 9. Data and state

```rust
pub struct ImpactView {
    pub subject: ImpactSubjectDto,   // Bill | Draft | Proposal | BudgetLine | Works | Option
    pub at: JurisdictionDto,
    pub cost: CostView,              // yearly, budget share, paid_from, funded share after; never gated
    pub lines: Vec<ImpactLine>,      // lead lines first (at most four flagged `lead`)
    pub areas: Vec<AreaImpact>,      // districts, then neighbourhoods; only those the player knows
    pub groups: Vec<GroupImpact>,    // classes, older residents
    pub timing: TimingView,          // months to start, to full effect, to its end
    pub claims: Vec<ClaimView>,      // who, their line, its score once due
    pub sureness: String,            // rough | fair | firm
    pub source: SourceView,          // which source the sizes come from, and its date
    pub record: Option<RecordView>,  // V1: credit by neighbourhood for a yes and a no
    pub then: Option<AccountView>,   // enacted laws: frozen estimate, started, added
}
pub struct ImpactLine {
    pub outcome: String, pub direction: String, pub horizon_months: u32, pub in_time: bool,
    pub form: String,                // range | about | words
    pub lo: Option<f64>, pub hi: Option<f64>, pub unit: String,
    pub key: String, pub args: BTreeMap<String, String>,
    pub lead: bool, pub tradeoff: bool,
}
pub struct AreaImpact {
    pub area: AreaRef,               // Jurisdiction | District { constituency } | Neighbourhood
    pub label: LabelView,            // the constitution's word and number, or the place's name
    pub felt: Option<f64>, pub band: String, pub level: KnowledgeLevelDto,
    pub works: Vec<u32>, pub disrupted: bool, pub member: Option<u32>,
}
```

- **Queries.** `BillView.impact` and `PolicyPreviewView.impact`; `QueryDto::Impact { subject }` for the sheet; the S6 layer reads `areas`. Redaction happens in the protocol crate, so a figure the player lacks never reaches the UI.
- **Cached:** nothing between queries in the first build. One `ImpactInputs` per jurisdiction is built per view and shared by its bills. If the phone measurement breaks the budget, a memo keyed by (subject, date, command count) is the remedy, in the protocol layer and never in state.
- **Saved** (save v28, with a migration):
  - on a law, `estimate: Option<FrozenEstimate>`: the unslanted estimate at enactment as integers (eight outcomes × three horizons × lo, mid, hi; under a hundred numbers, dropped when the law has been ended twelve years, as the spine keeps notable events);
  - `claims` (at most four: claimant, outcome, figure, score);
  - the renamed ledger keys (`homes_built` → `homes_started`);
  - `Subject::BillImpact` in the knowledge ledger.
- **Content:** `impact.jsonc` (kind `impact_table`), schema generated from `mandate-content` types; every mechanism kind and side effect has a default row or validation fails.
- **Tier-generic.** Everything is keyed by `Jurisdiction` (`inst.at()`). Districts are the constituencies of the body in the bill's procedure, labelled by the constitution. Table rows name the tiers they were calibrated for; the base pack ships local rows only, and a tier with no rows returns no estimate, not a guess. No office or body is named in code or keys.

## 10. Implementation plan

| # | Task | Tier | OWNS | After |
|---|---|---|---|---|
| 1 | **One honest bill, end to end.** `impact.rs` with direct quantities shared with `mods_for` and `implementation`; `housing_supply`, cost and funding only; the table type with one row from the audit (0.25–0.33); `BillView.impact`; four text lines in the bill view | large | `crates/mandate-sim/src/policy/{impact.rs (new), mod.rs}`; `crates/mandate-content/src/types/impact.rs` (new); `content/base/impact.jsonc` (new); `content/schemas/**`; `crates/mandate-protocol/src/{impact.rs (new), office.rs, lib.rs}`; `web/src/engine/protocol/**`; `web/src/components/BillCard.tsx`; `content/base/locale/en/impact.ftl` (new) | — |
| 2 | **Calibration, gate, bench.** `lab impact calibrate\|bench`, `audit estimate`, the nightly row, the generated table | mid | `crates/mandate-cli/src/{lab/impact.rs (new), lab/cli.rs, audit/estimate.rs (new), audit/mod.rs, audit/readability.rs}`; `content/base/impact.jsonc`; `.github/workflows/nightly.yml`; `scripts/wasm-bench.mjs` (new), `scripts/wasm-parity.mjs` | 1 |
| 3 | **Every mechanism, and the ledger.** The other ten mechanism kinds and three side effects; started against added; the frozen estimate; save v28 | mid | `crates/mandate-sim/src/policy/{impact.rs, mod.rs}`; `crates/mandate-save/**`; `impact.ftl` | 1; ∥ 2 |
| 4 | **Knowledge, claims, the ward view.** `Subject::BillImpact`, sources and levels, redaction, claims and their scoring, the counted shares and certainty, no `player.ward` | mid | `crates/mandate-sim/src/player/knowledge.rs`; `crates/mandate-sim/src/policy/claims.rs` (new); `crates/mandate-protocol/src/{impact.rs, knowledge.rs, office.rs}`; `content/base/{knowledge,places}.jsonc` | 3 |
| 5 | **The sheet, quick mode, the map layer.** Full sheet, tables, `bill_impact` on S6, the dev timing button, Playwright | mid | `web/src/**` except `engine/protocol` | 3, S6; ∥ 4 |
| 6 | **Laws that pay off (audit fix 3)** through `impact::felt`; approval rows calibrated | mid | `crates/mandate-sim/src/{population/mod.rs, concerns.rs}`; `content/base/{opinion,concerns}.jsonc`; a balance entry | 3; ∥ 4, 5 |
| 7 | **Members read it.** The `vote_lean` term, reasons, parameters, the audit's new row, bots re-run | mid | `crates/mandate-sim/src/policy/mod.rs` (votes only); `crates/mandate-content` council parameters; `content/base/council.jsonc`; `crates/mandate-cli/src/audit/policy.rs`; a balance entry | 6 |
| 8 | **Families, arcs, works, "Why?".** `OptionPreviewView.impact`, option estimates, works subjects, the account view | mid | `crates/mandate-protocol/src/{office.rs, why.rs, narrative.rs}`; the arc option's view of its effects in `crates/mandate-sim/src/narrative/**` (a read-only call into `impact`) | 4, F3 |
| 9 | **Texts and docs.** Remove the `secondary_keys` the table does not back (audit fix 8); EXTENDING ("a new mechanism needs a table row"), TEST_STRATEGY, OVERVIEW | small | `content/base/policies/**`; `content/base/locale/en/*.ftl`; `docs/**` | 2 |

**Acceptance tests.**
- Task 1: the estimate is unchanged when the seed, the held hazards and the hidden facts change; native and WASM digests match; the affordable-housing homes line passes the gate's homes row on three twins; no state hash moves.
- Task 2: `audit estimate` passes for every template; calibration twice gives the same file; the bench is inside the budget on the phone.
- Task 3: every template's ledger "added" is within 0.67–1.5 of its twin; a v27 save loads and shows the renamed entries.
- Task 4: an omniscient and a default player with the same commands produce identical worlds (the K6 guard); no figure reaches a view below its level; the true value lies inside every shown range.
- Task 7: the audit rows in §5; outcomes change on purpose, so the spine's golden is re-recorded and the commit says so.

**What in today's code will fight this** (each is in the task named):
- `bill_view`, `policy_preview`, `office_view` and `Knowing::ward_support_of` read `player.municipality` and `player.ward`; `PolicyInstance.jurisdiction` is a town index and `mods_for`, the agencies and `in_municipality` are per town (tasks 1 and 4 move the views; the estimator takes `inst.at()` from the start).
- The ledger mixes running totals (`homes_built`, `paid_per_person`) with levels overwritten each month (`jobs_supported_*`), under one map of bare numbers (task 3).
- Finance sets the funded share inside its yearly budget pass; the estimate needs that rule as a pure function of a jurisdiction and an extra cost (task 1).
- `Subject::PolicySupport` is keyed by town and template, and reads nothing for a family proposal (task 4).
- The lab enacts at month 1, at defaults, in the home town, which is always of class `town` (task 2 adds the month, the parameter level and the other towns).
- Approval ignores delivered results and the "services" worry reads budget stress, so an honest approval line is bad news for every law until task 6.
- `secondary_keys` are free text on the bill; nine are unbacked (task 9).

**Build first:** task 1. It puts "580 started, 110 to 190 added" on the worst offender of the audit, with a coefficient the audit has already measured, before any calibration exists.

## 11. Open questions for the owner

| # | Question | Recommendation |
|---|---|---|
| 1 | Members weigh their district's gain, so some bills that pass today will fail and some will pass. Accept? | **Yes**, at ward support's weight, after fix 3, tuned so parties still vote together at least three times in four. |
| 2 | Should the statistician's note cost anything? | **No charge.** It is read at the town hall and kept, dated, in the notebook. Members get the rounded papers without a visit. Depth from a place, not a fee (DEC-079). |
| 3 | Jobs and rent are town-wide in the engine, so the map shades felt gain, works and disruption, not jobs by ward. Enough for now? | **Yes.** Jobs and rents by neighbourhood arrive with commuting and neighbourhood rents (audit fix 10, M7). |
| 4 | Until fix 3 lands, an honest estimate says most laws cost approval. Hold the approval line back? | **Yes.** Ship the first build without approval; add it with task 6. |
| 5 | Should an oversold claim cost the claimant? | **Record and show it now** ("Why?", the notebook, their memory). Votes and press stories from it come with V1. |
| 6 | The pitch and old texts promise things the estimate will not ("rents ease as supply grows"; after fix 2 they do not within five years). Delete those texts, or model the effects? | **Delete in M6** (task 9), model in M7 where worth it. A sponsor may still claim it, and be scored. |

## 12. DEC proposal: the impact estimate

- **Status:** Proposed (design: `docs/design/IMPACT_ESTIMATE.md`; the owner's play note 10, 2026-11-21: "giving details on these votes would be helpful for users to think about an impact for their decision instead of just approval from their ward or party").
- **Context:**
  - A bill shows a party line and a ward figure, and nothing of what it does. The policy preview gives a cost and side effects in words.
  - The audit measured the game's own claims: the homes ledger overstates fourfold; nine texts describe effects the engine does not compute; twelve of fifteen laws lower approval whatever they deliver.
  - IMPLEMENTATION_PLAN names the impact estimate as shared mechanism S5, used by vote on impact, the families' preview, "Why?" and the bots.
- **Decision:**
  1. **One estimate, one function.** Bills, drafts, proposals, budget lines, works and arc options are estimated by the same engine call: eight outcomes the simulation computes, at 12, 36 and 60 months, for the jurisdiction, its districts and neighbourhoods and its classes, each as a range.
  2. **Mechanisms times measured pass-through.** Direct quantities come from the bill's mechanisms by the simulation's own arithmetic; what reaches the town comes from coefficients the lab fits on twins and ships as content. No game is forked when a bill is opened.
  3. **The estimate sees no future.** It reads published state and the table, never the seed, the director, held hazards or hidden facts, and draws no random number. Hazard protection is stated against a reference event.
  4. **The lab is the judge.** A nightly gate compares the estimate shown at enactment with what the twin measures, per outcome, and fails on a wrong sign, a silent effect, a vacuous range or miscalibration. It replaces the audit's ledger-honesty row.
  5. **Started against added.** The ledger keeps direct counts exactly and labels net effects as estimates made with the same coefficients. The estimate at enactment is frozen on the law, so before and after share their quantities.
  6. **Sizes are earned; kinds are not.** Cost, timing and the direction of every effect are never gated. Sizes come from a named source at the player's level of knowledge: a claim, the members' papers, the statistician's note.
  7. **Claims have a slant and are scored.** Sponsors, parties and the paper state a figure pushed towards their side by their candour, with no draw; the engine later records who was right.
  8. **Members read it.** A member's lean gains their district's felt gain, weighted by pragmatism, with a reason the player can see. Shown odds remain rolled odds.
  9. **The ward view counts people:** a word, the share for and the share against among likely voters, and a certainty that is the share of the district the player knows.
  10. **No figure without a mechanism and a gate row.** Texts that claim more are removed or become scored claims.
- **Cost:** save v28 with a migration; outcomes change once when members begin to read it (goldens re-recorded, bots and the audit re-run, a balance entry); one to two hours of calibration compute when mechanisms or the economy change (an estimate); about 60 Fluent keys.
- **Changes:** the audit's "ledger honesty" gate (replaced); audit fixes 3, 5 and 8 (specified here); `BillView` (`impact`; `secondary_keys` leave the bill view; the ward shares are counted); contracts §2.12 (`OptionPreviewView.impact`); `council.jsonc` (`district_gain`, `gain_band`).
- **Dependencies:** DEC-041, DEC-051, DEC-054, DEC-073, DEC-079, DEC-091, DEC-092, DEC-095, DEC-098, DEC-099, DEC-108, DEC-109; the experiment runner (S2); town map layers (S6).
