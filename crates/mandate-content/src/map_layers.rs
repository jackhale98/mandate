//! The country map's layer registry (DEC-089): what the map can colour its
//! towns by. Each entry names a figure the engine serves per town (its
//! `source`, a field of the map's town figures), how to colour it (the scale)
//! and how to read it (the number format, the legend's classes, the Fluent
//! keys of its label and units).
//!
//! The engine computes every figure, the class breaks and the legend bins
//! (`mandate_protocol::map_layers_view`); the client only paints the classes
//! it is given. Adding a layer is one entry here, plus a source (one engine
//! field) when the figure is new. These are presentation data: nothing in the
//! simulation reads them.

use crate::bundle::ContentBundle;
use crate::types::ContentKey;
use crate::validate::{Issue, valid_key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A figure the engine serves per town for the map. The names are the field
/// names of the map's town figures (`MapTownFigures`), so an unknown source
/// fails when the content loads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MapLayerSource {
    /// Residents.
    Population,
    /// Residents per km² of the town's land.
    Density,
    /// Share of the labour force out of work (0..1).
    Unemployment,
    /// Middle-class gross income per working-age adult, a year.
    MedianIncome,
    /// Typical monthly rent.
    Rent,
    /// The industry that defines the town: the largest relative to its
    /// national share, public employment aside (DEC-077 §1). Categorical.
    MainIndustry,
    /// The town's strongest natural resource (farmland, timber, minerals,
    /// fishing, energy, scenery). Categorical.
    MainResource,
}

impl MapLayerSource {
    /// Every source, in declaration order.
    pub const ALL: [MapLayerSource; 7] = [
        MapLayerSource::Population,
        MapLayerSource::Density,
        MapLayerSource::Unemployment,
        MapLayerSource::MedianIncome,
        MapLayerSource::Rent,
        MapLayerSource::MainIndustry,
        MapLayerSource::MainResource,
    ];

    /// The field name (as written in content and served in the view).
    pub fn key(self) -> &'static str {
        match self {
            MapLayerSource::Population => "population",
            MapLayerSource::Density => "density",
            MapLayerSource::Unemployment => "unemployment",
            MapLayerSource::MedianIncome => "median_income",
            MapLayerSource::Rent => "rent",
            MapLayerSource::MainIndustry => "main_industry",
            MapLayerSource::MainResource => "main_resource",
        }
    }

    /// Whether the figure names a category (an industry, a resource) rather
    /// than a quantity.
    pub fn categorical(self) -> bool {
        matches!(
            self,
            MapLayerSource::MainIndustry | MapLayerSource::MainResource
        )
    }
}

/// How a layer's classes are coloured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MapLayerScale {
    /// One hue, light to dark: magnitude.
    Sequential,
    /// Two hues either side of a grey midpoint: above or below a baseline.
    Diverging,
    /// A fixed order of a few hues, the rest folded into "Other": identity.
    Categorical,
}

/// How the client writes a layer's numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MapLayerFormat {
    /// A whole number (residents).
    Count,
    /// A number per km².
    PerKm2,
    /// A share (0..1) written as a percentage.
    Percent,
    /// An amount of money.
    Money,
    /// A category's name (categorical layers).
    Category,
}

/// How a sequential layer's class breaks are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MapLayerBreaks {
    /// About as many towns in each class (for skewed figures).
    Quantile,
    /// Classes of equal width from the lowest town to the highest.
    Equal,
}

/// A diverging layer's midpoint (its grey class).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MapLayerMidpoint {
    /// The national figure: the towns' figures weighted by population.
    National,
    /// A fixed value in the figure's units.
    Value(f64),
}

/// A layer's legend: how many classes, and how they are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MapLegendDef {
    /// Sequential: 3–7 classes. Diverging: an odd 3–7 (the middle one grey).
    /// Categorical: 1–3 named categories (the most common), the rest folded
    /// into "Other". Three is the most a map can tell apart for every reader
    /// (the colours are validated for all pairs).
    pub classes: u8,
    /// Sequential layers only (default quantile). A diverging layer's classes
    /// are equal steps either side of its midpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breaks: Option<MapLayerBreaks>,
}

/// One map layer (DEC-089).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MapLayerDef {
    /// Content id, `map_layer.<name>`. The layer picker lists layers in load order.
    pub id: ContentKey,
    /// Fluent key of the layer's name in the picker and the legend.
    pub label_key: String,
    /// Fluent key of the legend's units line (what the numbers are).
    pub unit_key: String,
    /// Fluent key of a short note under the legend (how the figure is chosen).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_key: Option<String>,
    /// Categorical layers: Fluent key of the readout's detail, given `$pct`
    /// (the category's share or index, 0–100).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail_key: Option<String>,
    /// The town figure the layer shows.
    pub source: MapLayerSource,
    pub scale: MapLayerScale,
    /// Diverging layers only (required there).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub midpoint: Option<MapLayerMidpoint>,
    pub format: MapLayerFormat,
    pub legend: MapLegendDef,
}

/// The Fluent keys a map layer names (checked against the English locale).
pub fn fluent_refs(b: &ContentBundle) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    for l in &b.map_layers {
        out.push((l.id.as_str(), l.label_key.as_str()));
        out.push((l.id.as_str(), l.unit_key.as_str()));
        for k in [&l.note_key, &l.detail_key].into_iter().flatten() {
            out.push((l.id.as_str(), k.as_str()));
        }
    }
    out
}

/// Shape rules the schema cannot express.
pub fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    for l in &b.map_layers {
        let id = l.id.as_str();
        let mut err = |m: &str| issues.push(Issue::error(id, m));
        if !valid_key(id) || !id.starts_with("map_layer.") {
            err("a map layer id must be a key starting with `map_layer.`");
        }
        let cat_source = l.source.categorical();
        let cat_scale = l.scale == MapLayerScale::Categorical;
        let cat_format = l.format == MapLayerFormat::Category;
        if cat_source != cat_scale || cat_source != cat_format {
            err(&format!(
                "source `{}` needs {}",
                l.source.key(),
                if cat_source {
                    "the categorical scale and the category format"
                } else {
                    "a sequential or diverging scale and a number format"
                }
            ));
        }
        if l.detail_key.is_some() && !cat_scale {
            err("detail_key is for categorical layers");
        }
        let k = l.legend.classes;
        match l.scale {
            MapLayerScale::Sequential => {
                if !(3..=7).contains(&k) {
                    err("a sequential layer has 3–7 classes");
                }
                if l.midpoint.is_some() {
                    err("only a diverging layer has a midpoint");
                }
            }
            MapLayerScale::Diverging => {
                if !(3..=7).contains(&k) || k % 2 == 0 {
                    err("a diverging layer has an odd 3–7 classes");
                }
                match l.midpoint {
                    None => err("a diverging layer needs a midpoint"),
                    Some(MapLayerMidpoint::Value(v)) if !v.is_finite() => {
                        err("the midpoint must be a finite number")
                    }
                    _ => {}
                }
                if l.legend.breaks.is_some() {
                    err("breaks are for sequential layers");
                }
            }
            MapLayerScale::Categorical => {
                if !(1..=3).contains(&k) {
                    err("a categorical layer names 1–3 categories (the rest are Other)");
                }
                if l.midpoint.is_some() {
                    err("only a diverging layer has a midpoint");
                }
                if l.legend.breaks.is_some() {
                    err("breaks are for sequential layers");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_keys_match_serde_names() {
        for s in MapLayerSource::ALL {
            assert_eq!(
                serde_json::to_value(s).ok(),
                Some(serde_json::Value::String(s.key().into()))
            );
        }
    }
}
