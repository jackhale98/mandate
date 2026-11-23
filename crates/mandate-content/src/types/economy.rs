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
