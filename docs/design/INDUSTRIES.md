# Specific industries and named plants

**Status:** design for the owner (M6 checklist, note 17). Not built. A decision record is proposed at the end. Scope is the cheap version: industries as data, demand per industry, plants named by what they make. Products as goods, firms, supply chains and trade between towns wait for transport.

In one line: the eight sectors stay as groups; under three of them sit sixteen industries, each with its own world demand; a town's larger employers are named plants that go on short time, get a closure notice with a date, and close or are reprieved for a reason the game can name.

## 1. The registry

**Decision: an industry is a new level under a sector, not a split of the sectors.** Every reader of `MuniEcon::sectors` (worries, town character, map layers, finance, policy mods, the lab) keeps working unchanged, and `sector.manufacturing` in existing laws and saves keeps its meaning.

- A sector with industries is a **split sector**. Its `SectorState` (capacity, jobs, wage, productivity, baseline jobs) becomes a total that `economy::markets` writes after stepping the industries: sums for capacity and jobs, job-weighted means for wage and productivity. Nothing else writes it. The five other sectors run today's code untouched.
- The writers of sector data today are `init`, `remap_sectors`, `record_baselines`, `markets` and the lab's planted closure (I1 confirms by search); each goes through the industries for a split sector.
- Split in the base pack: manufacturing (9), resources (5), agriculture (2). **Tourism, knowledge, retail, construction and public get none now**: they have no closure rule, three of them run on local demand, and their workplaces (hotel, university, campus, school, clinic) already have an identity. Validation refuses an industry under a `building`, `local_services` or `public` sector.
- **Fishing sits under resources**, though note 17 lists it with farming: today's extraction endowment and the region's fish stock already treat it as extraction, and keeping it there leaves every town's eight starting sector totals bit-identical to today's.

`content/base/industries.jsonc` (one entry shown in full; registry order is load order):

```jsonc
{ "kind": "industries", "items": [ {
  "id": "industry.food_processing",
  "sector": "sector.manufacturing",            // the group: totals, worries and map layers read it
  "name_key": "industry-food_processing",      // "Food processing"
  "talk_key": "talk-industry-food_processing", // the noun phrase residents use
  "weight": 0.18,                              // share of the sector's jobs in a generic economy
  "base_wage": 38000, "cyclicality": 0.3, "productivity_growth": 0.015,
  "trend": 0.0,                                // annual drift of capacity
  "world": { "volatility": 0.012, "reversion": 0.03, "shock_chance": 0.004,
             "shock_size": [0.10, 0.22], "shock_down": 0.5 },
  "exposure": {
    "prices": { "agriculture": -0.15 },        // commodity exponents: sells > 0, buys < 0
    "stock": null,                             // or { "of": "fish", "power": 0.7 }: mineral | timber | fish
    "water": 0.0, "competitiveness": 0.4, "labour_cost": 0.8,
    "low_pay": 0.5 },                          // share of its jobs a wage floor prices
  "links": [ { "from": "industry.fishing", "weight": 1.0, "local": 0.7 },   // `from`: an industry or a sector
             { "from": "industry.arable", "weight": 0.6, "local": 0.4 },    // `local`: the town against its region
             { "from": "industry.livestock", "weight": 0.6, "local": 0.4 } ],
  "link_power": 0.5,
  "siting": { "base": 0.15, "geo": { "fishing": 1.2, "fertility": 0.8 },    // weights on the town's GeoProfile
              "needs": [], "likes": { "port": 1.5, "rail": 1.2 },           // port, coastal, river, rail, industrial
              "by_class": [0.6, 1.2, 1.0, 0.6],                             // rural, town, city, metro
              "near": { "industry.fishing": 1.6 }, "regional_sd": 0.4 },    // tradition: one keyed draw per region
  "plants": { "place_kind": "factory", "area": "industrial",                // which place kind and district host it
              "named_share": [0.5, 0.8], "closes": true, "opens": "investment",   // investment | boom | none
              "founded": [15, 90], "margin_sd": 0.06 },
  "products": [                                // text and siting only; nothing is simulated per product
    { "id": "product.canned_fish", "works_key": "works-cannery", "product_key": "product-canned_fish", "with": "industry.fishing", "area": "waterfront" },
    { "id": "product.dairy", "works_key": "works-creamery", "product_key": "product-dairy", "with": "industry.livestock" },
    { "id": "product.flour", "works_key": "works-flour_mill", "product_key": "product-flour", "with": "industry.arable" } ]
} ] }
```

Shared numbers (the closure lines, opening sizes, plant counts by settlement class, name forms) sit in one `industry_params` block; an industry may override the closure numbers.

```rust
// mandate-content: IndustryDef mirrors the JSONC; `sector_index` and link indices are resolved at load.
// mandate-sim::economy
pub struct IndustryState {            // one per town and industry, in `industry_keys` order; absent = zeros
    pub small: f64,                   // capacity of the unnamed firms
    pub capacity: f64,                // derived each month: small + its open plants
    pub jobs: f64, pub wage: f64, pub productivity: f64,
    pub trend_jobs: f64,              // ten-year moving average of jobs (the anchor for links)
    pub product: Option<ContentKey>,  // what this town's branch of the industry makes
    pub factors: [f32; FACTORS],      // smoothed log of each demand factor (who to blame)
    pub baseline_jobs: u32,
}
pub struct Plant {
    pub id: PlantId,                  // u32, never reused
    pub municipality: MunicipalityId, pub place: PlaceId,
    pub industry: ContentKey, pub name: PlantName,   // { form: place | hood | family | word, stem: String }
    pub capacity: f64, pub margin: f32, pub founded: i32,
    pub status: PlantStatus,          // Running | ShortTime{since} | Threatened{since, closes_on} | Closed{since, factor}
    pub strain: u8,
}
pub enum Factor { Cycle, World, Price, Input, Stock, Water, Competitiveness, LabourCost, Supply }
// EconomyState gains: industry_keys: Vec<ContentKey>, world: Vec<f64>, plants: Vec<Plant>, plant_ids: IdAllocator;
// MuniEcon gains: industries: Vec<IndustryState>, prospect: f64.
```

`Factor` names terms of the formula, never a product. The first list (share-weighted means stay within 3% of today's sector values: manufacturing 46,400 / 0.82 / 2.0% a year / trend −0.75%; resources 51,300 / 0.59 / 1.2%; agriculture 31,100 / 0.30 / 1.5% / −0.4%):

| Industry | Weight | Wage | Cyc. | Prod. | Trend | Vol. | Exposures (exponents) and links | Works words |
|---|---|---|---|---|---|---|---|---|
| food_processing | 0.18 | 38,000 | 0.3 | 1.5% | 0 | 0.012 | farm prices −0.15; compete 0.4; labour 0.8; reads fishing, arable, livestock | cannery, creamery, flour mill |
| metals | 0.12 | 54,000 | 1.1 | 1.8% | −1.5% | 0.025 | minerals −0.15, energy −0.25; compete 1.0; labour 0.8; reads mining | steelworks, foundry, smelter |
| machinery | 0.16 | 50,000 | 1.0 | 2.2% | −0.6% | 0.018 | compete 1.0; labour 1.0; reads metals | engine works, tool works, loom works |
| textiles | 0.10 | 34,000 | 0.6 | 1.5% | −2.5% | 0.020 | compete 1.2; labour 1.4 | woollen mill, hosiery works, carpet works |
| timber_paper | 0.10 | 41,000 | 0.7 | 1.6% | −0.5% | 0.016 | energy −0.10; compete 0.7; labour 0.9; reads forestry | sawmill, paper mill, furniture works |
| chemicals | 0.10 | 56,000 | 0.7 | 2.2% | −0.3% | 0.018 | energy −0.35; compete 0.8; labour 0.6; reads oil_gas | chemical works, fertiliser works |
| vehicles | 0.12 | 52,000 | 1.2 | 2.4% | −1.0% | 0.022 | compete 1.2; labour 1.0; reads metals | motor works, wagon works |
| electronics | 0.08 | 48,000 | 0.9 | 3.5% | +0.5% | 0.024 | compete 1.0; labour 1.1 | appliance works, instrument works |
| shipbuilding | 0.04 | 47,000 | 1.4 | 1.2% | −1.5% | 0.028 | minerals −0.10; compete 1.1; labour 1.0; reads fishing; needs a port | shipyard, boatyard |
| mining | 0.35 | 58,000 | 0.7 | 1.2% | 0 | 0.022 | minerals +0.6; mineral stock 0.5; labour 0.3 | colliery, ore mine |
| quarrying | 0.15 | 44,000 | 0.6 | 1.0% | 0 | 0.012 | minerals +0.2; labour 0.3; reads the region's construction | quarry, brickworks |
| oil_gas | 0.15 | 72,000 | 0.6 | 1.5% | 0 | 0.026 | energy +0.8; mineral stock 0.4; labour 0.2 | oil field, gas terminal |
| forestry | 0.20 | 40,000 | 0.6 | 1.2% | 0 | 0.014 | timber stock 0.5; labour 0.4 | (small crews; no named plant) |
| fishing | 0.15 | 37,000 | 0.3 | 1.0% | −0.5% | 0.018 | farm prices +0.3, energy −0.15; fish stock 0.7; needs a coast | fish quay |
| arable | 0.55 | 32,000 | 0.3 | 1.8% | −0.5% | 0.012 | farm prices +0.5; water 0.6; wage floor 1.0 | grain co-operative, estate |
| livestock | 0.45 | 30,000 | 0.3 | 1.1% | −0.3% | 0.010 | farm prices +0.3; water 0.3; wage floor 1.0 | auction mart, dairy farm |

The per-industry `trend` replaces the one `manufacturing_trend`; a scenario's era value becomes an offset added to every manufacturing industry's trend. Validation: known parent; unique ids; weights above zero; share-weighted wage within 5% of the sector's (a warning); every link's `from` exists; at least one product; every name, talk, works and product key present in English.

## 2. Demand per industry

**Its own world index.** `EconomyState::world[i]` moves monthly in `economy::global`, on a draw keyed by the industry's id and the month (`keyed(seed, Stream::Economy, [KEY_INDUSTRY, key_of(id), month])`), so nothing in a town, and no industry added later, reshuffles another's history:

```text
ln x ← (1 − reversion) × ln x + normal(0, volatility) + jump          x kept in [0.6, 1.5]
jump = ±uniform(shock_size), monthly chance shock_chance, down with probability shock_down
```

- The index reverts to 1 (half-life about two years), so it cannot kill or inflate an industry; long decline is the `trend` on capacity. Its resting spread is about 7% for a typical industry and 11% for shipbuilding.
- A jump records `IndustryShock { industry, change_pct }` (exogenous, `world_economy`): about one for each industry every twenty years, a headline in the national paper.
- **The global cycle stays the common factor.** Every industry's demand is the cycle raised to its `cyclicality` times its own index, so a recession still hits them all (hardest in shipbuilding, vehicles and metals, least in food), while a steel slump leaves the cannery alone.

**The driver**, for industry *i* in town *m* of region *r*, is one generic product of named factors (exponents from content; today's three coded formulas are special cases of it):

```text
cycle    = global.demand ^ cyclicality                     as today
world    = x_i
price    = Π commodity_c ^ max(prices_c, 0)                what it sells
input    = Π commodity_c ^ min(prices_c, 0)                what it buys
stock    = max(stock_r, 0.05) ^ stock.power                the region's mineral, timber or fish stock
water    = max(water_r, 0.2) ^ water
compete  = national.manufacturing_competitiveness ^ competitiveness
labour   = ulc_m ^ −labour_cost                            unit labour costs, as today
supply   = (Σ_j w_j a_j u_j / Σ_j w_j a_j) ^ link_power    1 when nothing upstream is near
driver   = cycle × world × price × input × stock × water × compete × labour × supply
```

- **Local links without goods.** For each link *j*, `a_j = local × trend_jobs(town, j) + (1 − local) × trend_jobs(rest of region, j)` and `u_j` is last month's jobs on the same blend over `a_j`, kept in [0.3, 1.5]. So the cannery runs on the town's boats and the region's farms; the sawmill on forestry; the smelter on mining; the quarry on the region's building. Reading last month's values removes any ordering between industries. The anchor is a ten-year average, so when the fleet halves the cannery is short for years and then stops expecting it. A shipyard reads fishing lightly and takes "trade" from its high cyclicality: world trade is the cycle.
- The environment draws each stock down by the jobs of the industries that name it, in place of all resource jobs (today a quarry town empties its fish stock).

**Jobs**, then the sector totals:

```text
support   = 1 + elasticity × subsidy          laws on the industry plus laws on its sector
floor     = 1 − 0.4 × low_pay × wage_floor
effective = driver × support × floor          what the closure rule reads
target    = (capacity + law_capacity) × effective × shock
jobs     += (target − jobs) × (0.16 if target < jobs else 0.10)
```

- **Subsidies:** `sector_subsidy` on a split sector reaches each of its industries, as today; a mechanism may now name one industry (§7). **Wage floor:** today's cost, scaled by the industry's `low_pay`. **Shocks:** an `ActiveShock` on a sector hits all its industries; it gains an optional `industry` for one. Hazard shocks stay out of `effective`, as today, so a flooded works is not closed by the rule.
- **Cost.** Sixteen index draws a month for the country, and for each town about nine `pow` calls for each industry it has (three to five), against about three for each of eight sectors today. `markets` roughly doubles, and it is a small part of a month. Estimate: 2–4% more time on the 20-year burn-in, about 40 KB more save JSON for every forty towns. Task I1 measures both; the limit is 5% on world generation.

## 3. Closures and openings

**Per plant, deterministic, with a deadline.** Each plant has a fixed `margin` (drawn once; newer plants draw higher). Monthly, `m = effective × (1 + margin + relief)`:

| Step | Rule (starting values, in content) | Event |
|---|---|---|
| Short time | `m < 0.90` for 2 months | `PlantShortTime` |
| Threatened | `m < 0.78` for 9 months running | `PlantThreatened { closes_on }`: a notice of 4–9 months, one keyed draw |
| Reprieved | `m ≥ 0.84` for 2 months before the date | `PlantReprieved` |
| Closed | the date arrives | `PlantClosure` (extended) |

- No monthly dice: the only draw is the length of the notice, so the date shown is the date used (shown = rolled). A town gets at most one new notice every 12 months outside a recession; a second plant waits on short time.
- On closing, the plant's capacity leaves the industry, its share of the industry's jobs goes at once, and its place is marked closed through an effect request (DEC-090 D3). The plant keeps its record: "the old Albion mill".
- The unnamed firms have no event: `small` shrinks 1.5% a month while `effective < 0.78`. Agriculture's workplaces never close (`closes: false`); they go on short time ("a bad year").
- **What the spine records.** `PlantClosure` gains `industry`, `plant`, `factor` and, for `Supply`, the upstream industry. Its cause is `Pressure(..)` for the factor lowest in the industry's smoothed factors (a six-month average, so one bad month is not blamed), linked to a running recession when the factor is `Cycle`. The spine's `Pressure` gains `IndustryDemand`, `InputCosts`, `Supply` and `Water`. The four plant events share a `Thread::Plant(PlantId)`, so a closure links back to its notice and its short time.

**Openings, so towns do not only lose.**
- **Investment** (manufacturing). Today idle labour grows manufacturing capacity a little every month, unseen. That same amount now gathers in the town's `prospect` pool. When the pool reaches the smallest plant for the town's class (25, 60, 150, 250 jobs), a plant opens in the industry with the best `siting score × smoothed driver`: often not the one that left. It reopens a closed works of the right place kind, else takes a reserved works plot, else joins the unnamed firms. Policy can feed the pool (§7). Cause: `Pressure(SlackLabour)`, or the law.
- **Boom** (resources). A smoothed driver above 1.12 for 24 months with stock above 0.3 reopens the last closed pit at 60% of its size. Cause: the strongest factor.
- `PlantOpened { plant, industry, jobs, reopened }` is recorded either way; the audit requires openings to be at least 40% of closures over 30 years.

## 4. Worldgen

- **Sector totals first, exactly as today** (`base_weight × endowment(driver)`). Each split sector's jobs are then shared among its industries by `score ^ 2`, where `score = (base + Σ geo weights × the town's geography) × likes × by_class × near × the region's tradition × a town draw`, and zero when a `needs` flag fails. Flags come from the world: `port`, `coastal`, `river`, `rail` (a rail link), `industrial` (an industrial district). All draws are keyed by town and industry id; nothing joins worldgen's sequential stream.
- **One or two defining industries.** A town keeps at most 1, 2, 3, 4 industries per split sector (rural, town, city, metro); shares under 12% fold into the others. Ports get fishing, canneries and shipyards; forests get forestry and sawmills; ore gets mines and smelters; farmland gets its farms and their mills; rivers get textiles and paper; rail gets machinery and vehicles. The region's tradition draw makes a textile valley or a steel coast.
- **Product.** A town's branch of an industry makes one product: the one whose `with` industry is strongest nearby, else a keyed draw that favours the region's choice.
- **Plants** are placed after the economy is initialised (§5) and before the burn-in.
- **The burn-in is the history.** Twenty years of indices, trends, closures and openings run on these plants, so play begins with towns that already lost a mill (a closed place, drawn derelict, with its name), plants on short time, and newer works beside old ones. Plants standing at the start draw a founding year from the industry's `founded` range. Places opened during history count as original when baselines are recorded, so the layout plans them.
- **Showing it without naming a product in code.**
  - *Name:* an industry may carry name affixes with a chance (`"place_names": { "suffixes": ["haven", "mouth"], "chance": 0.3 }`) and neighbourhood words for the plant's district (`"hood_words": ["Quayside", "Net Loft"]`), applied by keyed draws from the town's top score.
  - *Layout:* the industry names a place kind and district, and the kind's `layout` block already carries placement and props (chimney by the water, headframe toward the ore). A product may override the `signature` prop in a later art round.
  - *Character:* `TownCharacter::industry` becomes an industry id where the sector is split (a sector id otherwise), with a second for two-industry towns; the union hall, the steward and the manager follow the plant.

## 5. Named plants

- **In state:** `Plant` (§1), owned by the economy and tied to a `Place` of an existing kind (`factory`, `mill`, `mine`, `port`, `farm`). No new place kind is needed, so no code names a works. The place's own `name` is empty; the plant's name parts are shown.
- **How many:** named plants per town across the split sectors: rural 0–1, town 1–2, city 2–4, metro 3–6, each at least the class's smallest size and at most 25% of the town's jobs. Within an industry the named plants take `named_share` of capacity (the first about 60% of it); the rest is `small`.
- **Names** are parts, composed by the client: a form and a stem from the engine, the noun from the product's `works_key`.

  | Form | Stem | Reads |
  |---|---|---|
  | `place` (45%) | the town | "the Garrowby cannery" |
  | `family` (30%) | a family-name draw | "Hartley's cannery" |
  | `word` (15%) | a pool in `names.jsonc` | "the Albion cannery" |
  | `hood` (10%) | its district | "the Quayside cannery" |

  One keyed draw per plant (`[KEY_PLANT_NAME, town, key_of(industry), ordinal]`); a clash in the town takes the next form. Residents say "the cannery" (`plant_form = works`) when the town has only one.
- **Its people** are real characters found through the place, never copied onto the plant: the head (`occupation.plant_manager`, or the owner's role word and the stem as family name for a `family` works) and the steward (`occupation.shop_steward`), seeded by the existing keyed draws on place and occupation. A plant that opens during play seeds its people then.
- **As a place:** it is on the street and the Places tab like any works, with the existing verbs (`tour_works`, `listen_at_gate`, `photo_op_gate`). Their readings become the plant's: jobs, hours, status, the closing date and the weakest factor, so a tour teaches the reason before the paper prints it (`Subject::Plant` in the ledger). Works interiors are not drawn yet; one shared `interior.works` room is a later art task.
- **Jobs reconcile exactly, because totals are sums.** A plant stores capacity only. `industry.capacity = small + Σ open plants` and `industry.jobs` are the state; a plant's jobs are `industry.jobs × its capacity / industry.capacity`; a sector's jobs are the sum of its industries in registry order. Whole numbers come from one `apportion(total, parts)` (largest remainder, ties to the lowest id) used top-down: sector, then industries, then plants and the small firms. The figures on a plant's sheet therefore add up to the town's sector line every time, and a debug check asserts both sums each month.
- **Note 16, a trade tied to a real workplace.** A politician out of office has a trade *at a place*: `{ occupation, workplace: PlaceId }`, drawn once (keyed by the character) from the open places in their town that host that occupation, weighted by jobs for plants. The sheet reads "Former { $post }, 8 years. Fitter at the Garrowby cannery." When the plant closes, the line becomes "laid off from the old cannery", derived from the place's status. Staged arcs can ask who that is.

**For the closure arc** (`STAGED_ARCS.md`), one query, `economy::plant_facts(state, plant)`: the plant, place, town and district; industry, product and name parts; head, steward and the named people whose workplace it is; whole jobs and their share of the town's; status, since when and `closes_on`; the weakest factor and the upstream industry; the districts its workers live in. The four events are the arc's stage triggers.

## 6. What the player sees

- **Town summary** (pocket and town data): under a split sector, its industries with jobs and trend; then "Main employers": each plant's name, what it makes, jobs, status, and one "why" line when it is on short time or worse. Exact jobs need the statistics desk or a tour; otherwise "about 200".
- **Map layers** (DEC-089, content plus one engine field each): the Industry layer keeps the eight groups; add `map_layer.works` ("What it makes": the defining industry, five classes) and `map_layer.plants` (short time, notice served, closed in two years, opened in two years).
- **Overheard, talk, headlines** take the plant, product and factor as arguments. **Crises:** `crisis.plant_closure` and its `job_loss` indicator are unchanged; its text gains the plant. The staged arc owns the beats.
- **Bill and policy views:** a bill that targets an industry or a plant shows which, its jobs, and the forecast in that industry's jobs.

Arguments (the engine sends keys and values; the client composes `$plant` as it resolves `industry_key` today): `plant_form`, `plant_stem`, `works_key` → `$plant`, `$works`; `product_key` → `$product`; `industry_key` → `$industry`; `upstream_key` → `$upstream`; `input_key` → `$input`; `factor`; `$town`, `$hood`, `$jobs`, `$month`; people come from the cast.

| Keys | Count |
|---|---|
| `industry-*`, `talk-industry-*` (2 for each of 16) | 32 |
| `works-*`, `product-*` (2 for each of 34 products) | 68 |
| `plant-name`, `plant-old`, `plant-small-firms`, form variants | 5 |
| `plant-status-*`, `factor-*`, sheet and town-data labels, two map layers and legends | 34 |
| `plant-why-<factor>` (9 sentences) | 9 |
| `event-plant-*`, `event-industry-shock`, `skip-item-plant-*` | 11 |
| `headline-plant-*` (short time, notice, reprieve, closure, opening) and `headline-industry-*` | 11 |
| `heard-plant-<status>-*` (4 statuses × 2) | 8 |
| `talk-steward-plant-*`, `talk-manager-plant-*` (4 statuses × 2 each), `talk-about-plant-*` (6) | 22 |
| two policy families and two mechanisms (names, options, parameters, effect lines) | 30 |
| flavour for each industry: `heard-industry-<id>-1..2`, `talk-trade-<id>-1..2` (4 × 16) | 64 |
| **Total** | **294** |

About 145 are sentences that need the style bible; the rest are nouns and labels. **The cost of one more industry is about ten keys** (two names, two for each product, four flavour lines), because every other line is written once for a status or a factor and takes the plant as an argument. Writing each industry its own headlines, overheard lines and talk would be about 480 sentences; this design does not. The arc's beats are budgeted in `STAGED_ARCS.md`.

Two rules for these templates, from the style bible: `$plant` is an object, or the subject of a past or modal verb only ("the steelworks" has no safe "is"); `$product` is a mass noun after "for" or "of". Flavour lines carry colour, never a claim about a simulated quantity. Sample:

```ftl
industry-food_processing = Food processing
talk-industry-food_processing = the food factories
works-cannery = cannery
product-canned_fish = canned fish
heard-industry-food_processing-1 = "You can smell what they're packing from the bus stop. I'd miss it, and I never thought I'd say so."
talk-trade-food_processing-1 = "Six till two on the filling line. You take the smell home with you."

industry-mining = Mining
talk-industry-mining = the pits
works-colliery = colliery
product-coal = coal
heard-industry-mining-1 = "Three of our family went down { $plant }. My lad drives a van."
talk-trade-mining-1 = "Nobody talks about the seam at home. You talk about the rota."

heard-plant-short-time-1 = "They've put { $plant } on short time. Nobody's saying for how long."
headline-plant-threatened = { $town } to lose { $plant }: { $jobs } jobs to go by { $month }
headline-plant-closure = Gates shut at { $plant }: { $jobs } jobs gone in { $town }
plant-why-supply = Less is coming in from { $upstream }, and { $plant } ran on it.
plant-why-world = Orders for { $product } are down everywhere, not only in { $town }.
talk-steward-plant-threatened-1 = "We've had the notice. { $month }, unless the orders come back. I want you at the gate before then, not after."
```

Rendered: "Garrowby to lose Hartley's cannery: 212 jobs to go by March"; "Less is coming in from the boats, and the Quayside cannery ran on it."

## 7. Policy

- **Targets.** `sector_subsidy` and `sector_capacity` accept `"industry"` in place of `"sector"`: a fixed id, or a family slot. `BindDef` gains `Industry { of_slot, defining, threatened }` (the industry of a bound plant's place, the town's defining one, or one with a plant under notice); a plant is bound as its place. Sector-wide laws keep working and reach every industry in the sector.
- **One new mechanism, `plant_support { plant, param }`:** relief on one plant's `margin` (rates relief, a grant against the notice), costed for each job at the plant. With an industry subsidy it is what lifts `m` over the reprieve line, so a rescue is arithmetic the bill view can show: "needs +0.06; this bill gives +0.04".
- **What a councillor can do about a threatened plant:** tour it and learn the factor before the press; pass plant support or an industry wage subsidy before the date; widen `policy.closure_response` (its rehire subsidy binds the threatened industry in place of all manufacturing); feed the `prospect` pool with an investment family (serviced land as a works under DEC-099, a rates holiday for a new works) so something opens after; or let it close and fund retraining. Each costs money and someone's goodwill (DEC-073): the steward wants the rescue, the business association resents a subsidy for one firm.
- **Two families** in `policy_families/industry.jsonc`: a plant rescue (trigger: a plant under notice) and inward investment (trigger: a closed works, or unemployment two points over the nation's).
- **The audit checks,** by twins on eight seeds for each mechanism and each industry class: the target industry's jobs move with the right sign in 8 of 8; other industries in the town move less than a quarter as much; a plant under notice with support survives 24 months in at least 20 points more twins than without; the forecast on the bill is within a factor of two of the twin's result.

## 8. Balance and checks

`mandate-cli audit industries --seeds 8 --years 30`, and bands in `sweep`:

| Check | Band |
|---|---|
| No industry dead or runaway: national jobs at year 30 over year 0 | 0.35–2.0, and within ±25% of its `trend` compounded |
| World index at its clamp | under 5% of months |
| Closures explained | 100% carry plant, industry, factor and a `Pressure`; the factor is the lowest and below 1 |
| One story does not dominate | no factor over 60% of closures; `Supply` and `Input` each appear |
| Capacity lost to closures | within ±25% of today's rate (today a driver at 0.78 removes about 4% of a sector a year) |
| Openings | at least 40% of closures over 30 years; closures without recovery after ten years at most 5% (today 3%) |
| Towns stay distinct | rank persistence of industry shares, year 0 to 30, at least 0.6; at least 8 different defining industries in a medium world; none defines over 25% of towns |
| Totals | both sums exact every month; sector whole numbers equal the apportioned parts |
| Existing bands | unemployment, population, inflation and debt bands hold; medians within 0.5 points of the run before; plant-closure crises per town-decade within ±25% |

What could go wrong:
- **Too many closures.** Sixteen independent indices produce more separate bad spells than one sector driver did. Guards: mean reversion; nine consecutive months below a low line (an index alone spends under 2% of months there, so a closure nearly always needs two factors or a jump); the notice with a reprieve; one notice per town per year. Knobs, in order: `line`, `margin_sd`, `shock_size`.
- **A national wave.** A deep jump puts every weak plant of one industry under notice at once. That is the steel crisis and worth having, but the audit caps the share of an industry's plants closed within 24 months of a jump at 35%.
- **Small towns with one plant.** A plant is at most 25% of a town's jobs at placement; the audit reports the unemployment jump after each closure by town class and fails over 10 points.
- **Layout.** A city may now need four works plots; the plan reserves them, but I2 must prove every town class fits.
- **Hash churn.** Closures leave the shared `Stream::Economy` for keyed draws, which shifts every later draw on that stream. Every state hash changes once, on purpose.

## 9. Save and compatibility

- **Save v27** (the version after the causal spine's v26), with one migration. For each town and split sector, capacity, jobs, wage and productivity are shared among industries by the same pure siting function worldgen uses (geography and keyed draws; no stream advances), the last industry taking the remainder. Plants are made by the same keyed placement from current capacity but **only at works places the town already has** (one each, largest industry first); the rest is `small`. So the factory a player has visited becomes "the Garrowby cannery" with the same place id, manager and steward, and no building appears in a town they know. Indices start at 1, anchors at current jobs, strain at 0. Old `PlantClosure` events keep no plant, and their text falls back to today's sector line.
- `economy.industry_keys` is saved. Content that adds an industry starts it with no capacity (it can arrive by an opening); content that removes one is refused, as for sectors.
- **Worlds generated before:** a saved world is its state and migrates. A seed generated afresh gives a different country (plants, some names, a different history): golden worlds regenerate once, before testers.

## 10. Tier-generic (DEC-098)

- A plant is a physical thing in a town: `Plant::municipality` and its `PlaceId` are right.
- **A region and the nation have industries too:** `economy::industries_at(at: Jurisdiction)` sums the towns inside (jobs, plants, statuses). Nothing is stored for it: a region's link anchor is the sum of its towns' anchors.
- **Industry policy is keyed by `Jurisdiction`.** A law's support reaches every plant in the towns its jurisdiction contains: a region's steel subsidy, later a national one. Today `policy::mods_for` reads local laws of one town only (`in_municipality`); I4 replaces it with a fold over every jurisdiction containing the town.
- Views and conditions ask for `player::home`, posts or `state.focus()`. No code names an industry, a product or an office: ids, place kinds, `Factor` and Fluent keys only.

## 11. Implementation plan

**First: the cannery slice** (I0, I1, then P1), so one port town has a named cannery that can go on short time, get a notice and close. It uses the full numeric registry, plants at existing works places only, names and the status lines, and a lab switch (`--industry-shock industry.fishing:0.6:24`) to bring the notice on.

| Task | Tier | What | OWNS | Acceptance |
|---|---|---|---|---|
| I0 | mid | Content types, schema, validation, `industries.jsonc` (numbers), names of industries and products | `mandate-content/src/types/industry.rs`, `types/mod.rs`, `content/base/industries.jsonc`, `locale/en/industries.ftl`, generated schemas | `content validate` passes; weighted means within 3% of the sectors'; bad parent, link or key refused |
| I1 | large | Industry state, world indices, the driver, totals, plants at existing places, the closure rule and events, spine pressures and thread, save v27 and migration, lab closure and industry shock | `mandate-sim/src/economy/{mod,industry,plants}.rs`, `events.rs`, `spine/`, `mandate-save`, `mandate-cli/src/lab/runner.rs` | sums exact for 30 years on 3 seeds; determinism and save round trip; unsplit sectors' traces unchanged with indices pinned at 1; a v26 save loads with the same sector totals; world generation at most 5% slower |
| P1 | mid | Minimal views and web: the plant's name, status, date and reason on the place panel and in town data; closure headline arguments | `mandate-protocol/src/{town,places}.rs`, `mandate-sim/src/media.rs`, `web/src/screens/PlacePanel*`, `web/src/components/town*` | a seeded port town shows the cannery; a lab shock gives short time, notice, closure, each with the factor |
| I2 | large | Worldgen: siting, products, several plants per town with new places, names, affixes, founding years, burn-in, layout fit | `mandate-worldgen/src/{industry.rs,places.rs,lib.rs}`, `content/base/names.jsonc`, `content/base/layout/` | eight seeds: every town class fits its works; at least 8 defining industries; ports hold all fishing; goldens regenerated |
| I3 | mid | Openings: the prospect pool, the boom rule, reopening and opening places, seeding people at a new plant | `economy/plants.rs` (after I1), `characters/mod.rs` | openings at least 40% of closures over 30 years; a reopened works keeps its place id |
| I4 | mid | Policy: industry targets, `plant_support`, mods by jurisdiction, `BindDef::Industry`, two families, bill view arguments | `mandate-content/src/types/policy.rs`, `policy_family.rs`, `mandate-sim/src/policy/`, `content/base/policies/`, `policy_families/industry.jsonc` | the audit's policy checks of §7 |
| I5 | mid | Views and conditions: industries and plants in town and map views, two layers, `Subject::Plant`, conditions and arguments, town character on industries, `plant_facts` | `mandate-protocol/src/{town,map}.rs`, `mandate-sim/src/{condition,overheard,visits}.rs`, `characters/town.rs`, `talk/context.rs`, `mandate-content/src/{condition,map_layers}.rs`, `content/base/map_layers.jsonc` | every key the engine names has English; redaction tests for exact jobs |
| I6 | mid | Web: town summary, plant sheet, map legends | `web/src/` (after I5) | typecheck, tests, one Playwright path to a plant |
| I7 | mid | `audit industries`, sweep bands, per-industry policy twins, tuning to the bands | `mandate-cli/src/audit/industries.rs`, `sweep.rs`, `lab/metrics.rs`, numbers in `industries.jsonc` | §8's table passes on 8 seeds |
| I8 | small | Environment stocks by industry; the plant's district in the jobs worry | `mandate-sim/src/environment/mod.rs`, `concerns.rs` | a quarry town's fish stock is untouched; the closure's worry is highest in the plant's district |
| I9 | small | Note 16: a trade at a workplace, and "laid off" | `mandate-sim/src/characters/knows.rs`, `mandate-protocol/src/characters.rs` | every out-of-office politician's workplace is an open place of their town at the draw |
| C1 | mid (content) | Status, factor, headline, event, overheard and talk lines (about 60 sentences) and 34 labels | `locale/en/industries.ftl`, `overheard.jsonc`, `conversations/` | content doctor clean; three fills read for every line |
| C2 | mid (content) | Flavour lines (64) and the two families' text, with the crisis rewrite | the same files, after C1 | as C1 |

Order: I0 → I1 → P1 (the slice). Then I2, I3, I4, I5, I7, I8, I9 and C1 run in parallel (disjoint OWNS; I3 starts when I1's `plants.rs` has merged); I6 after I5; C2 beside the staged-arc text; I7 finishes last, tuning against everything.

## 12. Open questions for the owner

1. **Fishing under resources, not with farming?** Recommended: yes (§1); a fishing town's talk comes from the industry, so it never reads "the quarries and the mines".
2. **Closures by notice and date, with no monthly roll?** Recommended: yes. A deadline is what an arc and a rescue bill need, and the date shown is the date used.
3. **A seed gives a different country after this lands.** Recommended: accept now, before testers, and regenerate the golden worlds once.
4. **Old saves get plants only at the works they already have** (fewer named plants than a new world). Recommended: accept; nothing appears in a town the player knows.
5. **Industries for tourism and knowledge?** Recommended: none in this pass; two each later as content, once the hotel and the campus are workplaces with jobs in the plant sense.
6. **How much text now?** Recommended: names and the lines by status and factor with the slice (I0 and C1, about 200 keys); the 64 flavour lines and the two families' text with the crisis rewrite (C2).

## DEC proposal: specific industries

- **Status:** Proposed (design in `docs/design/INDUSTRIES.md`; owner's note 17, 2026-11-24: "People are more interested in something specific as opposed to 'manufacturing demand'.").
- **Context:** a plant is a town's share of one generic sector with no product, so a closure reads as "manufacturing layoffs". DEC-075 made sectors a registry and foresaw "industries with their own quirks".
- **Decision:**
  1. **Industries are a registry level under sectors** (`industries.jsonc`). The eight sectors stay as groups for totals, worries and map layers; a split sector's state is the sum of its industries. The base pack splits manufacturing, resources and agriculture into sixteen.
  2. **Demand per industry** is one generic product of named factors with exponents in content: the world cycle, the industry's own world index (a keyed stream: reversion, noise, rare jumps), commodity prices sold and bought, the resource stock, water, national competitiveness, local labour costs, and local supply links that read upstream jobs in the town and its region.
  3. **Named plants.** A town's larger employers are plants: a place of an existing kind with an industry, a product, a name in parts and capacity. Their people are characters at the place. Plant jobs, industry jobs and sector jobs reconcile exactly because totals are sums.
  4. **Closures are per plant, deterministic and explained:** short time, a notice with a date, a reprieve or the closure, each an event; the cause is the weakest factor of the industry's formula (the spine's `Pressure`). Plants also open: from idle labour and policy (an investment pool) and from booms.
  5. **Worldgen places industries by geography and regional tradition** with keyed draws; the burn-in is their history.
  6. **Policy can target an industry or one plant**, at any tier (keyed by `Jurisdiction`); plants themselves belong to a town.
  7. **No code names an industry, a product or a works.** Text takes the plant, product and factor as arguments; lines are written per status and factor, with four flavour lines for each industry.
- **Not this (later, after transport):** products as goods, firms and ownership, supply chains with quantities, trade between towns, prices per product.
- **Cost:** save v27 with a migration; state hashes and generated worlds change once; about 294 Fluent keys (145 sentences); an estimated 2–4% on world generation; the tasks in §11.
- **Changes:** DEC-075 stage 1 (sectors gain a child registry; `SectorState` becomes derived for split sectors); the closure rule (was a monthly chance below 0.85 on the sector's driver); `economy.manufacturing_trend` (per-industry trends); the spine's `Pressure` (four more).
- **Dependencies:** DEC-041, DEC-053, DEC-054, DEC-075, DEC-078, DEC-085, DEC-089, DEC-090, DEC-092, DEC-098, DEC-099; the causal spine (S1); the staged arcs design.
