//! `LayoutContentView` (TOWN_GENERATION.md §4.3, §6; DEC-090 D4): the town
//! layout v2 registries (district types, terrain features, connectors and
//! place layout blocks), sent once and cached by content hash.
//!
//! The engine validates these registries like any content but never
//! interprets them; the client's town generator does. The view's JSON is the
//! shape the generator reads (`web/src/hub/layoutV2/registry.ts`): a field a
//! content file leaves out stays out.

use mandate_content::ContentBundle;
use mandate_content::layout as c;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// The layout registries of a content bundle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LayoutContentView {
    /// The bundle's content hash (the client caches the view by it).
    pub content_hash: String,
    /// District types, in load order.
    pub districts: Vec<DistrictTypeView>,
    /// Terrain features, in load order.
    pub features: Vec<TerrainFeatureView>,
    /// Connectors, in load order (the generator takes the first that fits).
    pub connectors: Vec<ConnectorTypeView>,
    /// Place kinds with a layout block, in load order.
    pub places: Vec<PlaceLayoutView>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OpenRangeView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub max: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GeoConditionView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub coast: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub river: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub port: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub lake: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub creeks: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub shore: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fishing_pm: Option<OpenRangeView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub minerals_pm: Option<OpenRangeView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ruggedness_pm: Option<OpenRangeView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub woods_pm: Option<OpenRangeView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub flood_pm: Option<OpenRangeView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DistrictTypeView {
    pub id: String,
    pub name_key: String,
    pub appears: DistrictAppearsView,
    pub site: DistrictSiteView,
    pub streets: DistrictStreetsView,
    pub plots: DistrictPlotsView,
    pub buildings: DistrictBuildingsView,
    pub hosts: Vec<String>,
    pub growth: DistrictGrowthView,
    pub tags: Vec<String>,
    pub art: DistrictArtView,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DistrictAppearsView {
    pub hood_kinds: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub geo: Option<GeoConditionView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub population: Option<OpenRangeView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub town_class: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub era: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub requires_place: Option<Vec<String>>,
    pub weight: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DistrictSiteView {
    pub prefers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub avoids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub anchor: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum StreetPatternView {
    Organic,
    Grid,
    Curvy,
    Contour,
    Yards,
    Quay,
    Ribbon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum LaneStyleView {
    None,
    Back,
    Stepped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum StreetSurfaceView {
    Asphalt,
    Dirt,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DistrictStreetsView {
    pub pattern: StreetPatternView,
    pub block: [u32; 2],
    pub depth: [u32; 2],
    pub lanes: LaneStyleView,
    pub max_dead_ends: u32,
    pub style: StreetSurfaceView,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum PlotGroundView {
    Yard,
    Grass,
    Field,
    Garden,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DistrictPlotsView {
    pub frontage: [u32; 2],
    pub depth: [u32; 2],
    pub setback: u32,
    pub joined: f64,
    pub ground: PlotGroundView,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DistrictBuildingsView {
    /// `[filler kind, weight]` pairs.
    pub mix: Vec<(String, f64)>,
    pub storeys: [u32; 2],
    pub density: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum GrowthFillView {
    InfillFirst,
    Outward,
    EdgeEstates,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum DeclineLookView {
    Boarded,
    Derelict,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DistrictGrowthView {
    pub reserve: f64,
    pub fill: GrowthFillView,
    pub decline: DeclineLookView,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DistrictArtView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub kit: Option<String>,
    pub family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ground: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub roof: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub walls: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub trim_ramps: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub lamps: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub props: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum TerrainGeneratorView {
    River,
    Creek,
    Coast,
    Lake,
    Hills,
    Cliff,
    Woods,
    Marsh,
    Beach,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TerrainFeatureView {
    pub id: String,
    pub generator: TerrainGeneratorView,
    pub appears: FeatureAppearsView,
    pub params: BTreeMap<String, FeatureParamView>,
    pub art: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FeatureAppearsView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub geo: Option<GeoConditionView>,
}

/// A generator parameter: a flag, an integer, or a list or table of integers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum FeatureParamView {
    Flag(bool),
    Int(i32),
    List(Vec<i32>),
    Table(Vec<Vec<i32>>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorKindView {
    Bridge,
    Footbridge,
    Ford,
    Viaduct,
    LevelCrossing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ConnectorTypeView {
    pub id: String,
    pub kind: ConnectorKindView,
    pub max_span: u32,
    pub roads: Vec<String>,
    pub crosses: Vec<String>,
    pub town_class: Vec<String>,
    pub era: Option<String>,
    pub art: BTreeMap<String, String>,
}

/// A place kind's identity and its layout block (§6.3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlaceLayoutView {
    pub id: String,
    /// The place kind (`PlaceView.kind`).
    pub key: String,
    pub name_key: String,
    pub layout: PlaceLayoutBlockView,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlaceLayoutBlockView {
    pub slot: String,
    /// `[[min w, min h], [max w, max h]]`, tiles.
    pub footprint: [[u32; 2]; 2],
    pub place: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub signature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub with: Option<Vec<String>>,
    pub landmark: bool,
    pub art: PlaceArtView,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlaceArtView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub kit: Option<String>,
    pub family: String,
}

// ---- Conversions (field by field, so a content change that the view does
// not follow fails to compile) -------------------------------------------------

fn range(r: &c::OpenRange) -> OpenRangeView {
    OpenRangeView {
        min: r.min,
        max: r.max,
    }
}

fn geo(g: &c::GeoCondition) -> GeoConditionView {
    GeoConditionView {
        coast: g.coast,
        river: g.river,
        port: g.port,
        lake: g.lake,
        creeks: g.creeks,
        shore: g.shore.clone(),
        fishing_pm: g.fishing_pm.as_ref().map(range),
        minerals_pm: g.minerals_pm.as_ref().map(range),
        ruggedness_pm: g.ruggedness_pm.as_ref().map(range),
        woods_pm: g.woods_pm.as_ref().map(range),
        flood_pm: g.flood_pm.as_ref().map(range),
    }
}

fn district(d: &c::TownDistrictDef) -> DistrictTypeView {
    let a = &d.appears;
    DistrictTypeView {
        id: d.id.clone(),
        name_key: d.name_key.clone(),
        appears: DistrictAppearsView {
            hood_kinds: a.hood_kinds.clone(),
            geo: a.geo.as_ref().map(geo),
            population: a.population.as_ref().map(range),
            town_class: a.town_class.clone(),
            era: a.era.clone(),
            requires_place: a.requires_place.clone(),
            weight: a.weight,
        },
        site: DistrictSiteView {
            prefers: d.site.prefers.clone(),
            avoids: d.site.avoids.clone(),
            anchor: d.site.anchor.clone(),
        },
        streets: DistrictStreetsView {
            pattern: match d.streets.pattern {
                c::StreetPattern::Organic => StreetPatternView::Organic,
                c::StreetPattern::Grid => StreetPatternView::Grid,
                c::StreetPattern::Curvy => StreetPatternView::Curvy,
                c::StreetPattern::Contour => StreetPatternView::Contour,
                c::StreetPattern::Yards => StreetPatternView::Yards,
                c::StreetPattern::Quay => StreetPatternView::Quay,
                c::StreetPattern::Ribbon => StreetPatternView::Ribbon,
            },
            block: d.streets.block,
            depth: d.streets.depth,
            lanes: match d.streets.lanes {
                c::LaneStyle::None => LaneStyleView::None,
                c::LaneStyle::Back => LaneStyleView::Back,
                c::LaneStyle::Stepped => LaneStyleView::Stepped,
            },
            max_dead_ends: d.streets.max_dead_ends,
            style: match d.streets.style {
                c::StreetSurface::Asphalt => StreetSurfaceView::Asphalt,
                c::StreetSurface::Dirt => StreetSurfaceView::Dirt,
            },
        },
        plots: DistrictPlotsView {
            frontage: d.plots.frontage,
            depth: d.plots.depth,
            setback: d.plots.setback,
            joined: d.plots.joined,
            ground: match d.plots.ground {
                c::PlotGround::Yard => PlotGroundView::Yard,
                c::PlotGround::Grass => PlotGroundView::Grass,
                c::PlotGround::Field => PlotGroundView::Field,
                c::PlotGround::Garden => PlotGroundView::Garden,
            },
        },
        buildings: DistrictBuildingsView {
            mix: d.buildings.mix.clone(),
            storeys: d.buildings.storeys,
            density: d.buildings.density,
        },
        hosts: d.hosts.clone(),
        growth: DistrictGrowthView {
            reserve: d.growth.reserve,
            fill: match d.growth.fill {
                c::GrowthFill::InfillFirst => GrowthFillView::InfillFirst,
                c::GrowthFill::Outward => GrowthFillView::Outward,
                c::GrowthFill::EdgeEstates => GrowthFillView::EdgeEstates,
            },
            decline: match d.growth.decline {
                c::DeclineLook::Boarded => DeclineLookView::Boarded,
                c::DeclineLook::Derelict => DeclineLookView::Derelict,
            },
        },
        tags: d.tags.clone(),
        art: DistrictArtView {
            kit: d.art.kit.clone(),
            family: d.art.family.clone(),
            ground: d.art.ground.clone(),
            roof: d.art.roof.clone(),
            walls: d.art.walls.clone(),
            trim_ramps: d.art.trim_ramps.clone(),
            lamps: d.art.lamps.clone(),
            props: d.art.props.clone(),
        },
    }
}

fn feature(f: &c::TerrainFeatureDef) -> TerrainFeatureView {
    TerrainFeatureView {
        id: f.id.clone(),
        generator: match f.generator {
            c::TerrainGenerator::River => TerrainGeneratorView::River,
            c::TerrainGenerator::Creek => TerrainGeneratorView::Creek,
            c::TerrainGenerator::Coast => TerrainGeneratorView::Coast,
            c::TerrainGenerator::Lake => TerrainGeneratorView::Lake,
            c::TerrainGenerator::Hills => TerrainGeneratorView::Hills,
            c::TerrainGenerator::Cliff => TerrainGeneratorView::Cliff,
            c::TerrainGenerator::Woods => TerrainGeneratorView::Woods,
            c::TerrainGenerator::Marsh => TerrainGeneratorView::Marsh,
            c::TerrainGenerator::Beach => TerrainGeneratorView::Beach,
        },
        appears: FeatureAppearsView {
            geo: f.appears.geo.as_ref().map(geo),
        },
        params: f
            .params
            .iter()
            .map(|(k, v)| {
                let v = match v {
                    c::FeatureParam::Flag(b) => FeatureParamView::Flag(*b),
                    c::FeatureParam::Int(n) => FeatureParamView::Int(*n),
                    c::FeatureParam::List(l) => FeatureParamView::List(l.clone()),
                    c::FeatureParam::Table(t) => FeatureParamView::Table(t.clone()),
                };
                (k.clone(), v)
            })
            .collect(),
        art: f.art.clone(),
    }
}

fn connector(x: &c::ConnectorDef) -> ConnectorTypeView {
    ConnectorTypeView {
        id: x.id.clone(),
        kind: match x.kind {
            c::ConnectorKind::Bridge => ConnectorKindView::Bridge,
            c::ConnectorKind::Footbridge => ConnectorKindView::Footbridge,
            c::ConnectorKind::Ford => ConnectorKindView::Ford,
            c::ConnectorKind::Viaduct => ConnectorKindView::Viaduct,
            c::ConnectorKind::LevelCrossing => ConnectorKindView::LevelCrossing,
        },
        max_span: x.max_span,
        roads: x.roads.clone(),
        crosses: x.crosses.clone(),
        town_class: x.town_class.clone(),
        era: x.era.clone(),
        art: x.art.clone(),
    }
}

/// The layout registries of `content`, in load order.
pub fn layout_content(content: &ContentBundle) -> LayoutContentView {
    LayoutContentView {
        content_hash: content.content_hash.clone(),
        districts: content.town_districts.iter().map(district).collect(),
        features: content.terrain_features.iter().map(feature).collect(),
        connectors: content.connectors.iter().map(connector).collect(),
        places: content
            .places
            .iter()
            .filter_map(|p| {
                let l = p.layout.as_ref()?;
                Some(PlaceLayoutView {
                    id: p.id.clone(),
                    key: p.key.clone(),
                    name_key: p.name_key.clone(),
                    layout: PlaceLayoutBlockView {
                        slot: l.slot.clone(),
                        footprint: l.footprint,
                        place: l.place.clone(),
                        signature: l.signature.clone(),
                        with: l.with.clone(),
                        landmark: l.landmark,
                        art: PlaceArtView {
                            kit: l.art.kit.clone(),
                            family: l.art.family.clone(),
                        },
                    },
                })
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn base() -> ContentBundle {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let sources = mandate_content::bundle::read_dir_sources(&root)
            .unwrap_or_else(|e| panic!("reading content: {e}"));
        mandate_content::load(&sources)
            .unwrap_or_else(|e| panic!("{e:?}"))
            .0
    }

    fn json<T: Serialize>(x: &T) -> serde_json::Value {
        serde_json::to_value(x).unwrap_or_default()
    }

    /// The view is the content's own JSON: what a file says is what the
    /// generator reads (no field added, renamed or dropped).
    #[test]
    fn the_view_has_the_content_shape() {
        let b = base();
        let v = layout_content(&b);
        assert_eq!(v.districts.len(), b.town_districts.len());
        assert_eq!(v.features.len(), b.terrain_features.len());
        assert_eq!(v.connectors.len(), b.connectors.len());
        for (d, dv) in b.town_districts.iter().zip(&v.districts) {
            assert_eq!(json(d), json(dv), "{}", d.id);
        }
        for (f, fv) in b.terrain_features.iter().zip(&v.features) {
            assert_eq!(json(f), json(fv), "{}", f.id);
        }
        for (c, cv) in b.connectors.iter().zip(&v.connectors) {
            assert_eq!(json(c), json(cv), "{}", c.id);
        }
        let with_layout: Vec<_> = b.places.iter().filter(|p| p.layout.is_some()).collect();
        assert_eq!(with_layout.len(), v.places.len());
        for (p, pv) in with_layout.iter().zip(&v.places) {
            assert_eq!(
                (&p.id, &p.key, &p.name_key),
                (&pv.id, &pv.key, &pv.name_key)
            );
            assert_eq!(json(&p.layout), json(&pv.layout), "{}", p.id);
        }
        assert!(v.districts.len() >= 12 && v.features.len() >= 8 && v.connectors.len() >= 6);
        assert_eq!(v.content_hash, b.content_hash);
    }

    /// The client's test fixture and default recipe
    /// (`web/src/hub/layoutV2/registry/base.json`) is the base content's view
    /// without its hash. Regenerate with
    /// `MANDATE_UPDATE_FIXTURES=1 cargo test -p mandate-protocol layout`.
    #[test]
    fn the_web_fixture_is_the_base_view() {
        let v = layout_content(&base());
        let recipe = serde_json::json!({
            "districts": v.districts,
            "features": v.features,
            "connectors": v.connectors,
            "places": v.places,
        });
        let text = serde_json::to_string_pretty(&recipe).unwrap_or_default() + "\n";
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../web/src/hub/layoutV2/registry/base.json");
        if std::env::var("MANDATE_UPDATE_FIXTURES").as_deref() == Ok("1") {
            std::fs::write(&path, &text).unwrap_or_else(|e| panic!("writing fixture: {e}"));
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(
            committed == text,
            "{} is stale: run MANDATE_UPDATE_FIXTURES=1 cargo test -p mandate-protocol layout",
            path.display()
        );
    }
}
