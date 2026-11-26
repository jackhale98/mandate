//! Economy and population seeding: sectors, attitudes, communities, economy, hazard and tax parameters, name pools.

use super::*;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SectorClass {
    Primary,
    Secondary,
    Tertiary,
    Public,
}

/// Behaviour driver of a sector (DEC-075): the closed set of economic
/// behaviours implemented in code. Sectors themselves are content; several
/// sectors may share a driver, and a new driver needs a code change.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SectorDriver {
    /// Farming and other land-based output: fertility, crop prices, water.
    LandBased,
    /// Mining, energy, timber: commodity prices and finite stocks; plant closures.
    Extraction,
    /// Manufacturing: national competitiveness and unit labour costs; plant closures.
    Industrial,
    /// Construction: housing starts and public spending.
    Building,
    /// Retail and local services: household consumption.
    LocalServices,
    /// Tourism and hospitality: scenery, coasts, disasters.
    Visitor,
    /// Knowledge and technology: productivity follows the technology trend.
    Knowledge,
    /// Public employment: funded by government spending.
    Public,
}

impl SectorDriver {
    pub const ALL: [SectorDriver; 8] = [
        SectorDriver::LandBased,
        SectorDriver::Extraction,
        SectorDriver::Industrial,
        SectorDriver::Building,
        SectorDriver::LocalServices,
        SectorDriver::Visitor,
        SectorDriver::Knowledge,
        SectorDriver::Public,
    ];
    /// Drivers the base economy cannot run without (validated: at least one
    /// sector each). The others are optional.
    pub const REQUIRED: [SectorDriver; 3] = [
        SectorDriver::Building,
        SectorDriver::LocalServices,
        SectorDriver::Public,
    ];
}

/// An attitude dimension (DEC-075): an issue every cohort, politician, and
/// party holds a position on, in [-1, 1]. Registry order is load order (files
/// by path, then item order); the engine stores attitudes densely in that order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttitudeDef {
    /// Content id, e.g. `attitude.redistribution`.
    pub id: ContentKey,
    /// Short key used in policy signatures, party-name words, and Fluent keys
    /// (`conviction-pro-<key>`, `conviction-anti-<key>`), e.g. `redistribution`.
    pub key: String,
    /// How starting opinions are seeded (DEC-018).
    #[serde(default)]
    pub seed: AttitudeSeed,
}

/// Starting-opinion model for one attitude. A cohort's position is
/// `by_class + by_age + by_neighborhood + regional offset + noise`, clamped to
/// [-1, 1]. Each region draws its own cultural offset once per attitude.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttitudeSeed {
    /// Lean by income class: `[low, middle, high]`.
    #[serde(default)]
    pub by_class: [f64; 3],
    /// Lean by age band: `[18–29, 30–44, 45–64, 65+]`.
    #[serde(default)]
    pub by_age: [f64; 4],
    /// Lean by neighbourhood kind.
    #[serde(default)]
    pub by_neighborhood: NeighborhoodLean,
    /// Standard deviation of each region's cultural offset.
    #[serde(default = "default_regional_sd")]
    pub regional_sd: f64,
    /// Standard deviation of each cohort's individual variation.
    #[serde(default = "default_noise_sd")]
    pub noise_sd: f64,
}

fn default_regional_sd() -> f64 {
    0.14
}
fn default_noise_sd() -> f64 {
    0.08
}

impl Default for AttitudeSeed {
    fn default() -> Self {
        AttitudeSeed {
            by_class: [0.0; 3],
            by_age: [0.0; 4],
            by_neighborhood: NeighborhoodLean::default(),
            regional_sd: default_regional_sd(),
            noise_sd: default_noise_sd(),
        }
    }
}

/// A value per neighbourhood kind (omitted kinds are 0).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NeighborhoodLean {
    #[serde(default)]
    pub core: f64,
    #[serde(default)]
    pub inner: f64,
    #[serde(default)]
    pub waterfront: f64,
    #[serde(default)]
    pub industrial: f64,
    #[serde(default)]
    pub suburb: f64,
    #[serde(default)]
    pub rural: f64,
}

/// A community (DEC-075): a cohort dimension beside age band and income
/// class. Every neighbourhood holds age × class cohorts for each community.
/// Registry order is load order (files by path, then item order). The base
/// pack ships one community, so the dimension costs nothing until an
/// expansion adds another (an Indigenous nation, an immigrant community, a
/// linguistic minority).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommunityDef {
    /// Content id, e.g. `community.general`.
    pub id: ContentKey,
    /// Fluent key, e.g. `community-general`.
    pub name_key: String,
    /// How much of each neighbourhood's population belongs to this community.
    /// Exactly one community omits it: that community takes the remainder
    /// (in the base pack, `community.general` is everyone).
    #[serde(default)]
    pub share: Option<CommunityShare>,
    /// Added to the seeded attitudes of this community's cohorts, by attitude
    /// key (omitted = 0).
    #[serde(default)]
    pub attitude_offsets: BTreeMap<String, f64>,
    /// `attitude_offsets` resolved to a dense vector in attitude-registry
    /// order at load. Not part of the content format.
    #[serde(skip)]
    #[schemars(skip)]
    pub attitude_offset_weights: Vec<f64>,
    /// Added to the seeded turnout propensity of this community's cohorts.
    #[serde(default)]
    pub turnout_offset: f64,
    /// Fraction of international immigrants who join this community (the
    /// remainder community takes whatever the others leave).
    #[serde(default)]
    pub immigration_share: f64,
}

/// Share model for a community: in a neighbourhood the community's share of
/// every age × class group is `base + by_neighborhood[kind] + regional
/// offset`, clamped to [0, 1]. Each region draws its offset once (sd
/// `regional_sd`). Where the communities with a share model add up to more
/// than 1, they are scaled down to 1; the remainder community takes the rest.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommunityShare {
    /// Share everywhere, before the neighbourhood and regional terms.
    #[serde(default)]
    pub base: f64,
    /// Added by neighbourhood kind (omitted kinds are 0).
    #[serde(default)]
    pub by_neighborhood: NeighborhoodLean,
    /// Standard deviation of each region's offset (0 = none).
    #[serde(default)]
    pub regional_sd: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SectorDef {
    pub id: ContentKey,
    pub name_key: String,
    /// Fluent key of the sector as residents name it in talk (a noun phrase
    /// such as "the factories"); the name is used without one.
    #[serde(default)]
    pub talk_key: Option<String>,
    pub class: SectorClass,
    /// Which coded behaviour this sector follows.
    pub driver: SectorDriver,
    /// Relative weight of this sector's base presence in a generic economy.
    pub base_weight: f64,
    /// Mean annual wage per worker at price level 1.0 (currency units).
    pub base_wage: f64,
    /// Elasticity of job demand to the global demand cycle.
    pub cyclicality: f64,
    /// Annual productivity growth (fraction).
    pub productivity_growth: f64,
}

/// Economy and demography parameters (REQ-005).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EconomyParams {
    pub inflation_target: f64,
    pub natural_unemployment: f64,
    pub neutral_real_rate: f64,
    /// Monthly probability that a global recession begins.
    pub recession_monthly_chance: f64,
    /// Peak fall in global demand during a recession, [min, max].
    pub recession_depth: [f64; 2],
    pub recession_months: [u16; 2],
    /// Monthly volatility of commodity prices (log scale).
    pub commodity_volatility: f64,
    /// Births per adult aged 18–44 per year.
    pub birth_rate: f64,
    /// Annual death rates: children, 18–29, 30–44, 45–64, 65+.
    pub death_rates: [f64; 5],
    /// Monthly share of mobile adults who reconsider where they live.
    pub migration_rate: f64,
    pub target_vacancy: f64,
    pub construction_lag_months: u8,
    /// Average dwelling price at price level 1.0.
    pub base_dwelling_price: f64,
    pub taxes: TaxParams,
    /// Unemployment benefit and pension as a share of the mean wage.
    pub unemployment_replacement: f64,
    pub pension_replacement: f64,
    /// Annual structural trends of a world without a scenario bias:
    /// technology growth, and the drift of manufacturing competitiveness.
    pub tech_growth: f64,
    pub manufacturing_trend: f64,
    /// How a hazard's damage and displacement fall across a town's
    /// neighbourhoods (DEC-090 D8).
    #[serde(default)]
    pub hazards: HazardParams,
    /// How each tier's treasury splits its discretionary budget across the
    /// budget categories at the start (DEC-098: content, not code).
    #[serde(default)]
    pub treasury_shares: TreasuryShares,
    /// The town housing market: household formation, rents, prices and
    /// private building (`economy::housing`).
    #[serde(default)]
    pub housing: HousingParams,
    /// Industries and named plants (DEC-110): the closure process, plant
    /// sizes and siting numbers every industry shares.
    #[serde(default)]
    pub industries: IndustryParams,
    /// What a jurisdiction pays to borrow: a spread over the policy rate by
    /// tier that grows with its debt against its revenue (`finance::accrue`).
    #[serde(default)]
    pub borrowing: BorrowingParams,
    /// Coefficients that turn a law's direct counts into what it added to
    /// the town, net (its ledger's "added" entries, audit fix 5; the same
    /// for every template).
    #[serde(default)]
    pub attribution: AttributionParams,
}

/// Borrowing costs (`finance::accrue`): spread = the tier's spread +
/// `per_ratio` × (debt ÷ a year's revenue − the tier's `free_ratio`), from 0
/// to `max_extra`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BorrowingParams {
    /// Spread over the policy rate at a prudent debt: local, regional, national.
    pub spreads: [f64; 3],
    /// Debt, in years of revenue, that lenders do not price, by tier
    /// (local, regional, national).
    pub free_ratio: [f64; 3],
    /// Added spread per year of revenue owed beyond `free_ratio`.
    pub per_ratio: f64,
    /// The most the spread rises above the tier's.
    pub max_extra: f64,
}

impl Default for BorrowingParams {
    fn default() -> Self {
        BorrowingParams {
            spreads: [0.012, 0.008, 0.0],
            free_ratio: [0.2, 0.5, 4.0],
            per_ratio: 0.02,
            max_extra: 0.06,
        }
    }
}

/// Pass-through of a law's direct counts to what it added, net (the
/// ledger's "added" estimates; measured on twins by the audit).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttributionParams {
    /// Share of a public programme's completed homes that private building
    /// would not have built anyway (1 − the starts it displaces).
    pub homes_net: f64,
    /// Sector jobs added per job a subsidy supports (or a programme creates).
    pub jobs_net: f64,
    /// Low incomes' rise per unit of wage-floor uplift.
    pub wage_floor_net: f64,
}

impl Default for AttributionParams {
    fn default() -> Self {
        AttributionParams {
            homes_net: 1.0,
            jobs_net: 1.0,
            wage_floor_net: 1.0,
        }
    }
}

/// The town housing market (`economy::housing`). Rent clears vacancy;
/// households form more slowly as rent takes more of an income, so rent is
/// bounded by incomes; builders answer price over cost and vacancy. The
/// starting state is an equilibrium: at the target vacancy and the neutral
/// rate, private starts equal their baseline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HousingParams {
    /// Adults per household at the reference rent burden.
    pub adults_per_household: f64,
    /// Adults per household, [fewest, most]: the bounds of doubling up.
    pub adults_per_household_range: [f64; 2],
    /// A year's rent over a middle-class income at which households form at
    /// `adults_per_household`.
    pub reference_burden: f64,
    /// Elasticity of adults per household to the rent burden: households
    /// double up (or form later) as rent takes more of an income.
    pub headship_elasticity: f64,
    /// Gross rent yield at the neutral rate: a year's rent over a dwelling's
    /// price as play begins.
    pub rent_yield: f64,
    /// Premium over the real rate in the user cost of housing (sets how
    /// prices answer interest rates).
    pub risk_premium: f64,
    /// Build cost as a share of the starting price (below 1: building is
    /// mildly profitable at the starting equilibrium).
    pub build_cost_share: f64,
    /// Monthly rent growth per unit of vacancy below target.
    pub rent_response: f64,
    /// Monthly share of the gap to the target price closed.
    pub price_adjustment: f64,
    /// Share of the stock demolished per year.
    pub demolition_rate: f64,
    /// Baseline building beyond replacing demolitions, as a share of the
    /// stock per year.
    pub base_growth: f64,
    /// Private starts per unit of price over build cost (against the start).
    pub starts_price_response: f64,
    /// Private starts per unit of vacancy below target.
    pub starts_vacancy_response: f64,
    /// Most private starts, as a multiple of the baseline.
    pub starts_max: f64,
    /// Share of the vacancy signal builders still answer while a rent cap
    /// binds (a shortage promises no rise in rents they may charge).
    pub capped_vacancy_signal: f64,
}

impl Default for HousingParams {
    fn default() -> Self {
        HousingParams {
            adults_per_household: 1.9,
            adults_per_household_range: [1.2, 6.0],
            reference_burden: 0.16,
            headship_elasticity: 0.6,
            rent_yield: 0.025,
            risk_premium: 0.035,
            build_cost_share: 0.85,
            rent_response: 0.12,
            price_adjustment: 0.05,
            demolition_rate: 0.005,
            base_growth: 0.004,
            starts_price_response: 4.0,
            starts_vacancy_response: 20.0,
            starts_max: 3.0,
            capped_vacancy_signal: 0.0,
        }
    }
}

/// Starting budget shares by category (in `BudgetCategory` order:
/// administration, education, health, safety, infrastructure, social,
/// housing, environment, economic development), per tier.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TreasuryShares {
    pub local: [f64; 9],
    pub regional: [f64; 9],
    pub national: [f64; 9],
}

impl Default for TreasuryShares {
    fn default() -> Self {
        TreasuryShares {
            local: [0.15, 0.25, 0.0, 0.2, 0.2, 0.05, 0.05, 0.05, 0.05],
            regional: [0.05, 0.3, 0.35, 0.0, 0.2, 0.0, 0.0, 0.05, 0.05],
            national: [0.15, 0.1, 0.2, 0.2, 0.15, 0.0, 0.05, 0.05, 0.1],
        }
    }
}

/// Where a hazard falls within a town (DEC-090 D8). The town-level rolls and
/// damage are unchanged; these set how they are shared by neighbourhood
/// exposure, who is displaced, and how fast a neighbourhood recovers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HazardParams {
    /// Most of a neighbourhood's homes one strike can destroy (a share).
    pub max_hood_damage: f64,
    /// Most a neighbourhood's hit can exceed the town's average (× the
    /// town's damage), for damage and for how hard residents feel it.
    pub max_relative: f64,
    /// Share of a neighbourhood's damage whose mobile adults leave the town
    /// for good (floods, storms and earthquakes; wildfires displace only for
    /// a while).
    pub leave_share: f64,
    /// Adults displaced for a while per unit of a neighbourhood's damage:
    /// they stay in less-hit neighbourhoods (or the next town) and return.
    pub temporary_share: f64,
    /// Monthly share of the displaced who return home.
    pub return_rate: f64,
    /// Monthly share of a neighbourhood's damaged homes repaired.
    pub repair_rate: f64,
}

impl Default for HazardParams {
    fn default() -> Self {
        HazardParams {
            max_hood_damage: 0.35,
            max_relative: 4.0,
            leave_share: 0.25,
            temporary_share: 0.75,
            return_rate: 0.12,
            repair_rate: 0.08,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaxParams {
    /// Annual property tax as a share of dwelling value (local tier).
    pub property: f64,
    /// Sales tax on consumption (regional tier).
    pub sales: f64,
    /// Effective income tax by class: low, middle, high (national tier).
    pub income: [f64; 3],
    /// Tax on business output (national tier).
    pub business: f64,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum NamePoolKind {
    Place,
    Region,
    GivenName,
    FamilyName,
    Neighborhood,
}

/// Syllable-based name generator pool.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NamePoolDef {
    pub id: ContentKey,
    pub kind: NamePoolKind,
    pub prefixes: Vec<String>,
    #[serde(default)]
    pub middles: Vec<String>,
    pub suffixes: Vec<String>,
    /// Optional whole-word list mixed in with generated names.
    #[serde(default)]
    pub words: Vec<String>,
}

// --- Industries (DEC-110) -------------------------------------------------------

/// A world commodity an industry sells or buys (the economy's three price
/// indices, in their order).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Commodity {
    Agriculture,
    Minerals,
    Energy,
}

impl Commodity {
    /// Index into the economy's commodity prices.
    pub fn index(self) -> usize {
        match self {
            Commodity::Agriculture => 0,
            Commodity::Minerals => 1,
            Commodity::Energy => 2,
        }
    }
}

/// A regional resource stock an industry draws on.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ResourceStock {
    Mineral,
    Timber,
    Fish,
}

/// A term of a town's geography an industry's siting weighs.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GeoTerm {
    Fertility,
    Timber,
    Minerals,
    Fishing,
    Energy,
    Scenic,
}

/// Where trade physically passes (DEC-110, the seam for foreign trade): a
/// kind of place. A port today; a border crossing and a rail head are kinds
/// a later layout can place. An industry's siting can weight one, an
/// industry can name the one its external trade ships through, and a later
/// disruption can hit one.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GatewayKind {
    Port,
    BorderCrossing,
    RailHead,
}

/// A source of demand at home (DEC-110): what the domestic part of an
/// industry's demand reads.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HomeSource {
    /// Household consumption across the nation, in real terms.
    NationalConsumption,
    /// Household consumption in the town.
    LocalConsumption,
    /// The town's building: housing starts and public works.
    Construction,
    /// Public spending in the town.
    Public,
}

impl HomeSource {
    pub const ALL: [HomeSource; 4] = [
        HomeSource::NationalConsumption,
        HomeSource::LocalConsumption,
        HomeSource::Construction,
        HomeSource::Public,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
}

/// A market (DEC-110, the seam for foreign trade; DEC-087): a place an
/// industry sells to outside the country. The base pack has one, the rest
/// of the world. An industry's external demand is the sum over markets of
/// its share in the market × the market's index for the industry × the
/// market's access. A neighbouring nation added later is another market,
/// with an index its own economy drives. Registry order is load order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MarketDef {
    /// Content id, e.g. `market.world`.
    pub id: ContentKey,
    pub name_key: String,
    /// Its weight in every industry's external sales (shares are these
    /// weights over their sum); an industry may set its own
    /// (`exposure.markets`).
    #[serde(default = "one")]
    pub share: f64,
    /// Access as the world begins: 1 is open trade. A tariff, a treaty, a
    /// dispute or a closed border changes it later.
    #[serde(default = "one")]
    pub access: f64,
}

fn one() -> f64 {
    1.0
}

/// A good (DEC-110, the seam for foreign trade): what a product is, for a
/// tariff or a trade deal to name. Every product belongs to one. A good
/// may carry one of the world's price indices: the prices industries sell
/// and buy at are read through this registry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GoodDef {
    /// Content id, e.g. `good.fuels`.
    pub id: ContentKey,
    pub name_key: String,
    /// The world price index this good's price is (none: no price index;
    /// an industry cannot be exposed to its price).
    #[serde(default)]
    pub price: Option<Commodity>,
}

/// A yes-or-no fact about a town an industry's siting needs or likes.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SiteFlag {
    Coastal,
    /// The town is on a river.
    River,
    /// A rail link reaches the town.
    Rail,
    /// The town has an industrial district.
    Industrial,
}

/// How a plant of an industry comes back after one has closed.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PlantOpens {
    /// Idle labour gathers in the town's investment pool; a works reopens
    /// when the pool is a plant's size.
    Investment,
    /// A long boom in the industry's demand reopens the works.
    Boom,
    /// Never.
    #[default]
    None,
}

/// An industry's own world demand index (DEC-110): a mean-reverting walk
/// on a keyed stream, with rare jumps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorldIndexDef {
    /// Monthly standard deviation of the log index.
    pub volatility: f64,
    /// Monthly pull of the log index toward 0 (0.03: a half-life of about
    /// two years).
    pub reversion: f64,
    /// Monthly chance of a jump.
    pub shock_chance: f64,
    /// A jump's size in logs, [least, most].
    pub shock_size: [f64; 2],
    /// Chance a jump is down.
    pub shock_down: f64,
}

/// A stock an industry depends on: demand scales with `stock ^ power`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StockExposure {
    pub of: ResourceStock,
    pub power: f64,
}

/// The exponents of an industry's demand formula (DEC-110): each named
/// factor raised to its exponent, multiplied together.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExposureDef {
    /// Price exponents by good (a good id with a price index): above zero
    /// for what it sells, below for what it buys.
    #[serde(default)]
    pub prices: BTreeMap<ContentKey, f64>,
    /// `prices` resolved at load to (price index, exponent). Not part of
    /// the content format.
    #[serde(skip)]
    #[schemars(skip)]
    pub price_index: Vec<(Commodity, f64)>,
    /// Its own weights across markets (market id to weight); omitted
    /// markets take the market's own `share`.
    #[serde(default)]
    pub markets: BTreeMap<ContentKey, f64>,
    /// The region's stock it draws on, if any.
    #[serde(default)]
    pub stock: Option<StockExposure>,
    /// Exponent on the region's water availability.
    #[serde(default)]
    pub water: f64,
    /// Import exposure: how hard foreign competition bites (the exponent
    /// on the nation's manufacturing competitiveness; a later tariff on its
    /// goods works through this term).
    #[serde(default)]
    pub import_exposure: f64,
    /// Elasticity to the town's unit labour costs.
    #[serde(default)]
    pub labour_cost: f64,
    /// Share of its jobs a wage floor prices (0 = none of them).
    #[serde(default)]
    pub low_pay: f64,
}

/// A local supply link (DEC-110): this industry runs on what another
/// industry, or a whole sector, does nearby. No goods move; the link reads
/// the upstream's jobs against their ten-year average.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IndustryLink {
    /// An industry id or a sector id.
    pub from: ContentKey,
    pub weight: f64,
    /// Share read from the town itself; the rest from the rest of its region.
    pub local: f64,
    /// `from` resolved at load: an index into the industry registry, or
    /// (with `from_sector`) the sector registry. Not part of the format.
    #[serde(skip)]
    #[schemars(skip)]
    pub from_index: usize,
    #[serde(skip)]
    #[schemars(skip)]
    pub from_sector: bool,
}

/// Where an industry settles (DEC-110): a town's score is
/// `(base + Σ geo weights × the town's geography) × likes × by_class × near
/// × the region's tradition × a town draw`, and zero when a `needs` flag
/// fails.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SitingDef {
    #[serde(default)]
    pub base: f64,
    /// Weights on the town's geography.
    #[serde(default)]
    pub geo: BTreeMap<GeoTerm, f64>,
    /// Flags a town must have.
    #[serde(default)]
    pub needs: Vec<SiteFlag>,
    /// Gateways a town must have (a port for a shipyard).
    #[serde(default)]
    pub needs_gateway: Vec<GatewayKind>,
    /// Multipliers for gateways a town has.
    #[serde(default)]
    pub gateways: BTreeMap<GatewayKind, f64>,
    /// Multipliers for flags a town has.
    #[serde(default)]
    pub likes: BTreeMap<SiteFlag, f64>,
    /// Multiplier by settlement class: rural, town, city, metro.
    #[serde(default = "ones4")]
    pub by_class: [f64; 4],
    /// Multipliers where another industry is strong in the town.
    #[serde(default)]
    pub near: BTreeMap<ContentKey, f64>,
    /// Standard deviation (in logs) of each region's tradition: one keyed
    /// draw per region and industry.
    #[serde(default)]
    pub regional_sd: f64,
}

fn ones4() -> [f64; 4] {
    [1.0; 4]
}

impl Default for SitingDef {
    fn default() -> Self {
        SitingDef {
            base: 1.0,
            geo: BTreeMap::new(),
            needs: Vec::new(),
            needs_gateway: Vec::new(),
            gateways: BTreeMap::new(),
            likes: BTreeMap::new(),
            by_class: ones4(),
            near: BTreeMap::new(),
            regional_sd: 0.0,
        }
    }
}

/// An industry's named plants (DEC-110).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlantsDef {
    /// The place kind (its `key`) that hosts this industry's plant; none:
    /// the industry is small firms and crews only.
    #[serde(default)]
    pub place_kind: Option<String>,
    /// Share of the industry's capacity in a town its named plant takes,
    /// [least, most]: one keyed draw.
    #[serde(default = "default_named_share")]
    pub named_share: [f64; 2],
    /// Whether its plants close (false: they go on short time and stay).
    #[serde(default = "yes")]
    pub closes: bool,
    #[serde(default)]
    pub opens: PlantOpens,
    /// Age in years of a plant standing as history begins, [least, most].
    #[serde(default = "default_founded")]
    pub founded: [u16; 2],
    /// Standard deviation of a plant's fixed margin.
    #[serde(default = "default_margin_sd")]
    pub margin_sd: f64,
}

fn yes() -> bool {
    true
}
fn default_named_share() -> [f64; 2] {
    [0.3, 0.5]
}
fn default_founded() -> [u16; 2] {
    [15, 90]
}
fn default_margin_sd() -> f64 {
    0.06
}

impl Default for PlantsDef {
    fn default() -> Self {
        PlantsDef {
            place_kind: None,
            named_share: default_named_share(),
            closes: true,
            opens: PlantOpens::None,
            founded: default_founded(),
            margin_sd: default_margin_sd(),
        }
    }
}

/// What a town's branch of an industry makes (DEC-110): text and siting
/// only; nothing is simulated per product.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductDef {
    /// Content id, e.g. `product.canned_fish`.
    pub id: ContentKey,
    /// Fluent key of the works' noun ("cannery").
    pub works_key: String,
    /// Fluent key of the product as a mass noun ("canned fish").
    pub product_key: String,
    /// The good it is (a good id): what a tariff or a trade deal names.
    pub good: ContentKey,
    /// The industry whose presence nearby picks this product.
    #[serde(default)]
    pub with: Option<ContentKey>,
}

/// An industry (DEC-110): a registry level under a sector. The sector stays
/// the group that totals, worries and map layers read; a sector with
/// industries is a split sector, whose state is the sum of its industries.
/// Registry order is load order (files by path, then item order).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IndustryDef {
    /// Content id, e.g. `industry.food_processing`.
    pub id: ContentKey,
    /// The sector it belongs to.
    pub sector: ContentKey,
    pub name_key: String,
    /// Fluent key of the industry as residents name it (a noun phrase).
    #[serde(default)]
    pub talk_key: Option<String>,
    /// Share of the sector's jobs in a generic economy.
    pub weight: f64,
    /// Mean annual wage per worker at price level 1.0.
    pub base_wage: f64,
    /// Elasticity of demand to the global cycle.
    pub cyclicality: f64,
    /// Annual productivity growth (fraction).
    pub productivity_growth: f64,
    /// Annual drift of capacity (fraction).
    #[serde(default)]
    pub trend: f64,
    /// How much of its demand is external (sold to the markets) against
    /// domestic (`home`): demand is `export_share × external + (1 −
    /// export_share) × home`.
    #[serde(default = "one")]
    pub export_share: f64,
    /// What its domestic demand reads, as weights (omitted: national
    /// consumption).
    #[serde(default)]
    pub home: BTreeMap<HomeSource, f64>,
    /// The gateway its external trade ships through, if one: what a later
    /// disruption at a gateway reaches.
    #[serde(default)]
    pub ships_through: Option<GatewayKind>,
    /// How its index moves in each market (one walk per market).
    pub world: WorldIndexDef,
    #[serde(default)]
    pub exposure: ExposureDef,
    #[serde(default)]
    pub links: Vec<IndustryLink>,
    /// Exponent on the supply term the links make.
    #[serde(default = "default_link_power")]
    pub link_power: f64,
    #[serde(default)]
    pub siting: SitingDef,
    #[serde(default)]
    pub plants: PlantsDef,
    /// What its works make: at least one.
    pub products: Vec<ProductDef>,
    /// `sector` resolved to its registry index at load. Not part of the
    /// content format.
    #[serde(skip)]
    #[schemars(skip)]
    pub sector_index: usize,
    /// `siting.near` resolved to (industry index, multiplier) at load.
    #[serde(skip)]
    #[schemars(skip)]
    pub near_index: Vec<(usize, f64)>,
}

fn default_link_power() -> f64 {
    0.5
}

/// The closure process and plant sizes shared by every industry (DEC-110).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IndustryParams {
    /// A plant goes on short time when its margin-adjusted demand is under
    /// this for `short_months` months running.
    pub short_line: f64,
    pub short_months: u8,
    /// It is back on full time when demand is at or over this for
    /// `short_months` months (above the short line, so hours do not flap).
    pub full_line: f64,
    /// A notice is served when it is under this for `notice_after_months`
    /// months running.
    pub notice_line: f64,
    pub notice_after_months: u8,
    /// A notice's length in months, [least, most]: the one draw.
    pub notice_months: [u8; 2],
    /// Short time ends in a review after this many months (DEC-110,
    /// amended 2026-11-26): demand still under `review_line` gives a
    /// notice where the industry can close; else full time again.
    pub review_months: u16,
    pub review_line: f64,
    /// The share of its capacity a plant sheds when a review puts it back
    /// on full time while its demand is still under `full_line`: the jobs
    /// that do not come back when orders do.
    pub review_cut: f64,
    /// A notice is lifted when demand is at or over this for
    /// `reprieve_months` months before the date.
    pub reprieve_line: f64,
    pub reprieve_months: u8,
    /// Months between new notices in one town (waived in a recession).
    pub notice_gap_months: u16,
    /// Monthly shrinkage of an industry's unnamed firms while its demand is
    /// under the notice line.
    pub small_decay: f64,
    /// An industry smaller than this in a town sheds no firms (jobs).
    pub min_capacity: f64,
    /// The smallest named plant by settlement class: rural, town, city,
    /// metro (jobs).
    pub min_plant: [f64; 4],
    /// The most of a town's jobs one plant holds when it is placed.
    pub max_town_share: f64,
    /// Months in the average of each demand factor that names the cause.
    pub factor_months: f64,
    /// Months in the long average of jobs that anchors the supply links.
    pub trend_months: f64,
    /// Supply links read upstream jobs over their average within this range.
    pub link_range: [f64; 2],
    /// Bounds of an industry's world index.
    pub world_range: [f64; 2],
    /// Siting: the most industries a town keeps in one split sector, by
    /// settlement class (rural, town, city, metro).
    pub max_industries: [u8; 4],
    /// Siting: a share of the sector under this folds into the others.
    pub min_share: f64,
    /// Siting: standard deviation (in logs) of a town's own draw.
    pub town_sd: f64,
    /// A boom reopens a works when the industry's demand has averaged over
    /// `boom_line` and the stock is over `boom_stock`, `boom_months` after
    /// it closed at the earliest, at `boom_size` of the closed plant's size.
    pub boom_line: f64,
    pub boom_months: u16,
    pub boom_stock: f64,
    pub boom_size: f64,
    /// How plant names are formed, as weights: the town, a family, a word
    /// from `works_words`, the district.
    pub name_forms: [f64; 4],
    /// Words a works is named by ("the Albion mill"). Fictional (REQ-001).
    pub works_words: Vec<String>,
    /// A plant that opens during a run draws its margin this much higher
    /// (newer works are the last to go).
    pub new_plant_margin: f64,
    /// While a closed works waits to reopen, the share of what idle labour
    /// draws each month that gathers toward it; the rest grows the unnamed
    /// firms at once, so a town's recovery does not wait for the works.
    pub pool_share: f64,
}

impl Default for IndustryParams {
    fn default() -> Self {
        IndustryParams {
            short_line: 0.90,
            short_months: 2,
            full_line: 0.94,
            notice_line: 0.78,
            notice_after_months: 9,
            notice_months: [4, 9],
            review_months: 12,
            review_line: 0.88,
            review_cut: 0.10,
            reprieve_line: 0.84,
            reprieve_months: 2,
            notice_gap_months: 12,
            small_decay: 0.015,
            min_capacity: 200.0,
            min_plant: [25.0, 60.0, 150.0, 250.0],
            max_town_share: 0.25,
            factor_months: 6.0,
            trend_months: 120.0,
            link_range: [0.3, 1.5],
            world_range: [0.6, 1.5],
            max_industries: [1, 2, 3, 4],
            min_share: 0.12,
            town_sd: 0.25,
            boom_line: 1.12,
            boom_months: 24,
            boom_stock: 0.3,
            boom_size: 0.6,
            name_forms: [0.45, 0.30, 0.15, 0.10],
            works_words: Vec::new(),
            new_plant_margin: 0.05,
            pool_share: 0.5,
        }
    }
}
