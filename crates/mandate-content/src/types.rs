//! Content types. These Rust types are the single source of truth; JSON
//! Schemas are generated from them (DEC-053).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stable content identifier, e.g. `sector.manufacturing`.
pub type ContentKey = String;

/// One content file. The `kind` field selects the variant.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContentFile {
    /// Pack manifest (exactly one per pack).
    Pack(PackManifest),
    /// Tuning parameters (REQ-005).
    Tuning { value: Tuning },
    /// Economic sectors (DEC-019).
    Sectors { items: Vec<SectorDef> },
    /// Name-generation pools for places and people.
    NamePools { items: Vec<NamePoolDef> },
    /// Economy and demography parameters (REQ-005).
    Economy { value: EconomyParams },
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackManifest {
    pub id: ContentKey,
    pub version: String,
    /// Fluent key for the pack's display name.
    pub name_key: String,
    /// Load order: lower first. The base pack is 0.
    #[serde(default)]
    pub load_order: i32,
}

/// Data-driven tuning parameters (REQ-005).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tuning {
    /// Standard day-part slots per turn (Morning, Afternoon, Evening).
    pub standard_slots: u8,
    /// Whether the optional Late Night slot is offered.
    pub late_night_enabled: bool,
    /// Turn lengths in days by phase (DEC-012).
    pub turn_days: TurnDays,
    /// Years of NPC-only simulation before the player starts (DEC-022).
    pub burn_in_years: u8,
    /// Calendar year the history begins.
    pub start_year: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TurnDays {
    pub campaign: u8,
    /// Days before an election that switch to daily turns.
    pub final_stretch: u8,
    pub governing_local: u8,
    pub governing_regional: u8,
    pub governing_national: u8,
}

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

/// Behavioural kind of a sector; exactly one sector per kind (validated).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SectorKind {
    Agriculture,
    Resources,
    Manufacturing,
    Construction,
    Retail,
    Tourism,
    Knowledge,
    Public,
}

impl SectorKind {
    pub const ALL: [SectorKind; 8] = [
        SectorKind::Agriculture,
        SectorKind::Resources,
        SectorKind::Manufacturing,
        SectorKind::Construction,
        SectorKind::Retail,
        SectorKind::Tourism,
        SectorKind::Knowledge,
        SectorKind::Public,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SectorDef {
    pub id: ContentKey,
    pub name_key: String,
    pub class: SectorClass,
    pub kind: SectorKind,
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

// ---- Position-preserving file wrappers -------------------------------------
// serde's internally tagged enums buffer their input and lose line numbers, so
// loading probes `kind` first, then parses into one of these structs directly.
// `ContentFile` (above) remains the schema source of truth.

#[derive(Deserialize)]
pub(crate) struct KindProbe {
    pub kind: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PackFile {
    #[allow(dead_code)]
    pub kind: String,
    pub id: ContentKey,
    pub version: String,
    pub name_key: String,
    #[serde(default)]
    pub load_order: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ValueFile<T> {
    #[allow(dead_code)]
    pub kind: String,
    pub value: T,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ItemsFile<T> {
    #[allow(dead_code)]
    pub kind: String,
    pub items: Vec<T>,
}

pub(crate) fn parse_content_file(json: &str) -> Result<ContentFile, serde_json::Error> {
    let probe: KindProbe = serde_json::from_str(json)?;
    Ok(match probe.kind.as_str() {
        "pack" => {
            let p: PackFile = serde_json::from_str(json)?;
            ContentFile::Pack(PackManifest {
                id: p.id,
                version: p.version,
                name_key: p.name_key,
                load_order: p.load_order,
            })
        }
        "tuning" => ContentFile::Tuning {
            value: serde_json::from_str::<ValueFile<Tuning>>(json)?.value,
        },
        "sectors" => ContentFile::Sectors {
            items: serde_json::from_str::<ItemsFile<SectorDef>>(json)?.items,
        },
        "name_pools" => ContentFile::NamePools {
            items: serde_json::from_str::<ItemsFile<NamePoolDef>>(json)?.items,
        },
        "economy" => ContentFile::Economy {
            value: serde_json::from_str::<ValueFile<EconomyParams>>(json)?.value,
        },
        other => {
            use serde::de::Error;
            return Err(serde_json::Error::custom(format!(
                "unknown content kind `{other}`"
            )));
        }
    })
}
