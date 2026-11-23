//! Scenarios (DEC-078 decision 4): geography, economy, era and politics of a start.

use super::*;

// --- Scenarios (DEC-078 decision 4) -----------------------------------------------

/// A starting world: biases on world generation (geography, economy, era,
/// politics). Everything left out is drawn at random, as for a random
/// country, so a scenario only says what makes it different.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDef {
    /// Content id, e.g. `scenario.wealthy_coast`.
    pub id: ContentKey,
    /// Fluent key of the name shown on the title screen.
    pub name_key: String,
    #[serde(default)]
    pub geography: ScenarioGeography,
    #[serde(default)]
    pub economy: ScenarioEconomy,
    #[serde(default)]
    pub era: ScenarioEra,
    #[serde(default)]
    pub politics: ScenarioPolitics,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioGeography {
    /// Sides of the country on the sea (1–3); random when omitted.
    #[serde(default)]
    pub sea_sides: Option<u8>,
    /// Strength of mountain ranges (0..1); random when omitted.
    #[serde(default)]
    pub mountain_strength: Option<f64>,
    /// Size of the country within the map.
    #[serde(default = "one")]
    pub country_extent: f64,
    /// Rainfall multiplier; random when omitted.
    #[serde(default)]
    pub wetness: Option<f64>,
    /// Mean temperature in the south (°C); random when omitted.
    #[serde(default)]
    pub temp_south: Option<f64>,
    /// Mineral deposits multiplier.
    #[serde(default = "one")]
    pub mineral_richness: f64,
    /// Population multiplier on the world size's population.
    #[serde(default = "one")]
    pub population: f64,
    /// Every town and city (not rural places) has an industrial district.
    #[serde(default)]
    pub industrial_towns: bool,
}

impl Default for ScenarioGeography {
    fn default() -> Self {
        ScenarioGeography {
            sea_sides: None,
            mountain_strength: None,
            country_extent: 1.0,
            wetness: None,
            temp_south: None,
            mineral_richness: 1.0,
            population: 1.0,
            industrial_towns: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioEconomy {
    /// Multiplies the structural presence of sectors with a driver (omitted = 1).
    #[serde(default)]
    pub endowment: BTreeMap<SectorDriver, f64>,
    /// Wages and dwelling prices multiplier.
    #[serde(default = "one")]
    pub wealth: f64,
    /// Multipliers on each neighbourhood's income-class mix: `[low, middle, high]`.
    #[serde(default = "ones3")]
    pub class_mix: [f64; 3],
}

impl Default for ScenarioEconomy {
    fn default() -> Self {
        ScenarioEconomy {
            endowment: BTreeMap::new(),
            wealth: 1.0,
            class_mix: [1.0; 3],
        }
    }
}

impl ScenarioEconomy {
    /// Endowment multiplier for a sector driver.
    pub fn endowment_of(&self, driver: SectorDriver) -> f64 {
        self.endowment.get(&driver).copied().unwrap_or(1.0)
    }
}

/// The trends of the age: technology and industry, and the climate's drift.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioEra {
    /// Annual technology growth (default `economy.tech_growth`).
    #[serde(default)]
    pub tech_growth: Option<f64>,
    /// Annual drift of manufacturing competitiveness (default `economy.manufacturing_trend`).
    #[serde(default)]
    pub manufacturing_trend: Option<f64>,
    /// Annual drying of the climate; random when omitted.
    #[serde(default)]
    pub drying: Option<f64>,
    /// Annual growth in storminess; random when omitted.
    #[serde(default)]
    pub storminess: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenarioPolitics {
    /// Added to every cohort's seeded opinion, by attitude key (omitted = 0).
    #[serde(default)]
    pub attitude_shift: BTreeMap<String, f64>,
}

pub(super) fn one() -> f64 {
    1.0
}
fn ones3() -> [f64; 3] {
    [1.0; 3]
}
