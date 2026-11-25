//! The town map's layer registry (DEC-086, DEC-089's pattern; M6 note 11):
//! what the town map overview can show over the town. Each entry names what
//! the engine serves for it (its `source`), how it is drawn (its `kind`:
//! boundaries, markers or a shaded figure), how it is coloured (its `scale`)
//! and how it is read (the Fluent keys of its label, its legend's line and
//! an optional note).
//!
//! The engine computes everything a layer shows, as far as the player knows
//! it (DEC-091): the districts and who holds each seat, who is where, each
//! neighbourhood's class and the legend
//! (`mandate_protocol::town_layers::town_map_layers_view`). The client only
//! paints what it is given, and lists the same data as text (DEC-051).
//! Adding a layer is one entry here, plus a source when what it shows is new
//! (`docs/development/EXTENDING.md`, "Adding a town map layer"). These are
//! presentation data: nothing in the simulation reads them.
//!
//! A source is a tagged object so that it can carry its own settings, and it
//! names the parameter the viewer chooses, if it has one
//! ([`TownLayerSource::param`]): the worries layer's is which worry. A later
//! source (a bill's impact by neighbourhood, with the bill as its parameter)
//! is a new variant here and one function in the view; the registry's shape
//! and the client do not change.

use crate::bundle::ContentBundle;
use crate::types::ContentKey;
use crate::validate::{Issue, valid_key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How a layer is drawn over the town.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TownLayerKind {
    /// Outlines of areas made of neighbourhoods, each labelled, with a
    /// marker for whoever holds it.
    Boundaries,
    /// People or things at places, clustered where they would overlap.
    Markers,
    /// Every neighbourhood filled by its class of a figure; an unknown
    /// neighbourhood is drawn as unknown, never as the lowest class.
    Shaded,
}

impl TownLayerKind {
    pub fn key(self) -> &'static str {
        match self {
            TownLayerKind::Boundaries => "boundaries",
            TownLayerKind::Markers => "markers",
            TownLayerKind::Shaded => "shaded",
        }
    }
}

/// What the viewer chooses within a layer (the picker beside the layer's
/// name). The engine lists the choices; the registry never does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TownLayerParam {
    /// One of the concern registry's worries.
    Worry,
}

impl TownLayerParam {
    pub fn key(self) -> &'static str {
        match self {
            TownLayerParam::Worry => "worry",
        }
    }
}

fn yes() -> bool {
    true
}

/// What the engine serves for a layer. An unknown `type` fails when the
/// content loads; each variant is an object so a source can carry settings
/// of its own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TownLayerSource {
    /// The districts of the legislature of the player's home jurisdiction
    /// (the constitution's body, by procedure), from the
    /// neighbourhood-to-district mapping its elections use, with each
    /// seat's holder.
    Districts {},
    /// The people the player has at least heard of, at their workplace or
    /// where the player last saw them, as far as the player knows either.
    People {},
    /// What each neighbourhood worries about (the one ranking, as felt), as
    /// far as the player knows it. Its parameter is the worry.
    Worries {
        /// Also offer "each neighbourhood's top worry" as the first choice
        /// (categories, not a magnitude).
        #[serde(default = "yes")]
        top: bool,
    },
}

impl TownLayerSource {
    /// The source's name (its `type`, as written in content and served).
    pub fn key(&self) -> &'static str {
        match self {
            TownLayerSource::Districts {} => "districts",
            TownLayerSource::People {} => "people",
            TownLayerSource::Worries { .. } => "worries",
        }
    }

    /// How what it serves is drawn: a layer's `kind` must be this.
    pub fn kind(&self) -> TownLayerKind {
        match self {
            TownLayerSource::Districts {} => TownLayerKind::Boundaries,
            TownLayerSource::People {} => TownLayerKind::Markers,
            TownLayerSource::Worries { .. } => TownLayerKind::Shaded,
        }
    }

    /// How it is coloured: a layer's `scale` must be this.
    pub fn scale(&self) -> TownLayerScale {
        match self {
            TownLayerSource::Districts {} | TownLayerSource::People {} => TownLayerScale::Party,
            TownLayerSource::Worries { .. } => TownLayerScale::Sequential,
        }
    }

    /// The parameter the viewer chooses, if the source has one.
    pub fn param(&self) -> Option<TownLayerParam> {
        match self {
            TownLayerSource::Districts {} | TownLayerSource::People {} => None,
            TownLayerSource::Worries { .. } => Some(TownLayerParam::Worry),
        }
    }
}

/// How a layer's marks are coloured. Colour never carries identity alone:
/// every district and neighbourhood is labelled and the legend is always
/// shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TownLayerScale {
    /// The app's party colours, each party named in the legend.
    Party,
    /// One hue, light to dark: a magnitude.
    Sequential,
}

impl TownLayerScale {
    pub fn key(self) -> &'static str {
        match self {
            TownLayerScale::Party => "party",
            TownLayerScale::Sequential => "sequential",
        }
    }
}

/// A way to narrow a markers layer. The engine says which markers each
/// filter keeps; the client only hides the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TownLayerFilter {
    /// The members of the home legislature (named by the constitution).
    Body,
    /// One filter per party that has someone on the layer.
    Party,
    /// People the player knows (met or seen), not only heard of.
    Met,
}

/// A layer's legend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TownLegendDef {
    /// Shaded layers: where a choice names a category per neighbourhood
    /// (each one's top worry), how many categories get a colour of their own
    /// (1–3, the most common; the rest fold into "Other"). Default 3.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<u8>,
}

/// One town map layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TownMapLayerDef {
    /// Content id, `town_map_layer.<name>`. The picker lists layers in load order.
    pub id: ContentKey,
    /// Fluent key of the layer's name in the picker and the legend. Given
    /// `$tier` (the home jurisdiction's tier), so a name can follow the
    /// constitution's words.
    pub label_key: String,
    /// Fluent key of the legend's line (what the marks are); given `$tier`.
    pub unit_key: String,
    /// Fluent key of a short note under the legend.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_key: Option<String>,
    pub kind: TownLayerKind,
    /// What the engine serves for it.
    pub source: TownLayerSource,
    pub scale: TownLayerScale,
    /// Markers layers: the filters offered, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<TownLayerFilter>,
    #[serde(default)]
    pub legend: TownLegendDef,
}

impl TownMapLayerDef {
    /// The categories a categorical choice names (the legend's setting, or 3).
    pub fn categories(&self) -> usize {
        self.legend.categories.unwrap_or(3) as usize
    }
}

/// The Fluent keys a town map layer names (checked against the English locale).
pub fn fluent_refs(b: &ContentBundle) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    for l in &b.town_map_layers {
        out.push((l.id.as_str(), l.label_key.as_str()));
        out.push((l.id.as_str(), l.unit_key.as_str()));
        if let Some(k) = &l.note_key {
            out.push((l.id.as_str(), k.as_str()));
        }
    }
    out
}

/// Shape rules the schema cannot express.
pub fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    for l in &b.town_map_layers {
        let id = l.id.as_str();
        let mut err = |m: &str| issues.push(Issue::error(id, m));
        if !valid_key(id) || !id.starts_with("town_map_layer.") {
            err("a town map layer id must be a key starting with `town_map_layer.`");
        }
        if l.kind != l.source.kind() {
            err(&format!(
                "source `{}` is drawn as `{}`, not `{}`",
                l.source.key(),
                l.source.kind().key(),
                l.kind.key()
            ));
        }
        if l.scale != l.source.scale() {
            err(&format!(
                "source `{}` needs the `{}` scale",
                l.source.key(),
                l.source.scale().key()
            ));
        }
        if !l.filters.is_empty() && l.kind != TownLayerKind::Markers {
            err("filters are for markers layers");
        }
        for (i, f) in l.filters.iter().enumerate() {
            if l.filters[..i].contains(f) {
                err("a filter is listed twice");
            }
        }
        match l.legend.categories {
            Some(_) if l.kind != TownLayerKind::Shaded => {
                err("legend.categories is for shaded layers")
            }
            Some(k) if !(1..=3).contains(&k) => {
                err("a shaded layer names 1–3 categories (the rest are Other)")
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_keys_match_serde_names() {
        for s in [
            TownLayerSource::Districts {},
            TownLayerSource::People {},
            TownLayerSource::Worries { top: true },
        ] {
            let v = serde_json::to_value(&s).unwrap_or_default();
            assert_eq!(v.get("type").and_then(|t| t.as_str()), Some(s.key()));
        }
    }

    #[test]
    fn a_source_refuses_settings_it_does_not_have() {
        assert!(
            serde_json::from_str::<TownLayerSource>(r#"{ "type": "people", "bill": 3 }"#).is_err()
        );
        assert_eq!(
            serde_json::from_str::<TownLayerSource>(r#"{ "type": "worries" }"#).ok(),
            Some(TownLayerSource::Worries { top: true })
        );
    }
}
