//! Town layout v2 registries (DEC-090 D4, TOWN_GENERATION.md §6): district
//! types, terrain features, connectors, and the `layout` block of a place kind.
//!
//! These are presentation data. The client's town generator reads them (the
//! engine sends them as `LayoutContentView`); the engine only validates them
//! here and never interprets them, so they cannot change a simulation result.
//! The shapes are the ones the stage A generator was written against: a
//! field left out of a file stays out of the view (`skip_serializing_if`), so
//! the generator sees exactly what the file says.

use crate::bundle::ContentBundle;
use crate::types::ContentKey;
use crate::validate::{Issue, valid_key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---- Closed lists (extension points, TOWN_GENERATION.md §6.5) ---------------

/// Engine neighbourhood kinds a district type can stand for.
pub const HOOD_KINDS: &[&str] = &[
    "core",
    "inner",
    "waterfront",
    "industrial",
    "suburb",
    "rural",
];

/// Town classes (`TownGeographyView.class`).
pub const TOWN_CLASSES: &[&str] = &["metro", "city", "town", "rural"];

/// District siting predicates (§5 stage 5), coded in the generator.
pub const SITE_PREDICATES: &[&str] = &[
    "centre",
    "near_centre",
    "low_slope",
    "floodplain",
    "riverbank",
    "shore",
    "high_ground",
    "arterial",
    "near_rail",
    "by_water",
    "downwind",
    "upwind",
    "edge",
    "toward_minerals",
    "toward_woods",
    "cliff",
];

/// Place placement predicates (§6.3, §6.5). `toward:<bearing>` and
/// `near_place:<kind>` take an argument and are checked separately.
pub const PLACE_PREDICATES: &[&str] = &[
    "on_square",
    "near_square",
    "central",
    "arterial",
    "edge",
    "by_water",
    "shore",
    "riverbank",
    "near_rail",
    "high_ground",
    "largest_residential",
    "quiet",
];

/// What a `toward:<x>` placement predicate may point at.
pub const TOWARD_TARGETS: &[&str] = &["minerals", "woods"];

/// Slot kinds a place layout reserves.
pub const PLACE_SLOTS: &[&str] = &[
    "civic",
    "square",
    "works",
    "community",
    "commercial",
    "green",
    "harbour",
];

/// Filler building kinds a district's mix can name (the art draws each).
pub const FILLER_KINDS: &[&str] = &[
    "house",
    "terrace",
    "shop",
    "warehouse",
    "shed",
    "farmhouse",
    "barn",
    "boathouse",
];

/// Shore kinds in a geography condition.
pub const SHORE_KINDS: &[&str] = &["beach", "rocky", "cliff"];

/// Ways a connector can carry.
pub const CONNECTOR_ROADS: &[&str] = &["arterial", "street", "lane", "path", "rail"];

/// What a connector can cross.
pub const CONNECTOR_CROSSES: &[&str] = &["river", "sea", "lake", "creek", "rail"];

/// Terrain generators the base game needs a feature for.
pub const REQUIRED_GENERATORS: &[TerrainGenerator] = &[
    TerrainGenerator::River,
    TerrainGenerator::Creek,
    TerrainGenerator::Coast,
    TerrainGenerator::Hills,
];

/// Connector kinds the base game needs an entry for.
pub const REQUIRED_CONNECTORS: &[ConnectorKind] = &[
    ConnectorKind::Bridge,
    ConnectorKind::LevelCrossing,
    ConnectorKind::Ford,
];

// ---- Shared pieces -----------------------------------------------------------

/// An inclusive range; either end may be left open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenRange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
}

/// Geography conditions (each a field of the town's geography, §6.5). Every
/// condition given must hold; per-mille ranges are 0..1000.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GeoCondition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coast: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub river: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lake: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creeks: Option<bool>,
    /// Shore kinds (`beach`, `rocky`, `cliff`); the town's must be one of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shore: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fishing_pm: Option<OpenRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minerals_pm: Option<OpenRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruggedness_pm: Option<OpenRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub woods_pm: Option<OpenRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flood_pm: Option<OpenRange>,
}

// ---- District types ----------------------------------------------------------

/// A district type (§6.2): when the generator may choose it for a
/// neighbourhood, how its streets, plots and buildings are laid out, and its
/// art keys.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TownDistrictDef {
    /// Content id, e.g. `district.old_core`.
    pub id: ContentKey,
    /// Fluent key, `district-<name>`.
    pub name_key: String,
    pub appears: DistrictAppears,
    pub site: DistrictSite,
    pub streets: DistrictStreets,
    pub plots: DistrictPlots,
    pub buildings: DistrictBuildings,
    /// Place kinds this district hosts.
    #[serde(default)]
    pub hosts: Vec<String>,
    pub growth: DistrictGrowth,
    /// Gameplay tags (`flood_prone`, `storm_surge`, `mine_risk`, `pollution_downwind`).
    #[serde(default)]
    pub tags: Vec<String>,
    pub art: DistrictArt,
}

/// When a district type may be chosen. Among the eligible types the
/// generator picks by weighted rendezvous hashing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistrictAppears {
    /// Neighbourhood kinds it can stand for.
    pub hood_kinds: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geo: Option<GeoCondition>,
    /// The town's baseline population.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub population: Option<OpenRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub town_class: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub era: Option<Vec<String>>,
    /// Place kinds the town must have (e.g. a mine for pit rows).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_place: Option<Vec<String>>,
    /// Rendezvous tickets (a positive integer).
    pub weight: u32,
}

impl DistrictAppears {
    /// No condition beyond the neighbourhood kind: a pick that always exists.
    pub fn unconditional(&self) -> bool {
        self.geo.is_none()
            && self.population.is_none()
            && self.town_class.is_none()
            && self.era.is_none()
            && self.requires_place.is_none()
    }
}

/// Siting preferences for the district's seed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistrictSite {
    pub prefers: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avoids: Option<Vec<String>>,
    /// What the district is laid out around (`square`, `harbour`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum StreetPattern {
    Organic,
    Grid,
    Curvy,
    Contour,
    Yards,
    Quay,
    Ribbon,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum LaneStyle {
    None,
    Back,
    Stepped,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum StreetSurface {
    Asphalt,
    Dirt,
}

/// The district's street pattern and its parameters (tiles).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistrictStreets {
    pub pattern: StreetPattern,
    /// Block width range (6..40; §8: the minimum block is 6 × 5).
    pub block: [u32; 2],
    /// Block depth range (5..20).
    pub depth: [u32; 2],
    pub lanes: LaneStyle,
    pub max_dead_ends: u32,
    pub style: StreetSurface,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PlotGround {
    Yard,
    Grass,
    Field,
    Garden,
}

/// Building footprints and the open ground of the district's blocks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistrictPlots {
    /// Footprint width range (2..12 tiles).
    pub frontage: [u32; 2],
    /// Footprint depth range (2..8 tiles).
    pub depth: [u32; 2],
    /// Front-garden rows (0..2).
    pub setback: u32,
    /// Share of plots joined to the next (0..1).
    pub joined: f64,
    pub ground: PlotGround,
}

/// Filler buildings: `[kind, weight]` pairs, storeys and density.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistrictBuildings {
    pub mix: Vec<(String, f64)>,
    /// Storeys range (1..8).
    pub storeys: [u32; 2],
    /// Share of plots built on (0..1).
    pub density: f64,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GrowthFill {
    InfillFirst,
    Outward,
    EdgeEstates,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DeclineLook {
    Boarded,
    Derelict,
}

/// Growth behaviour (reserved land, §9.2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistrictGrowth {
    /// Share of plots held back for growth (0..0.5).
    pub reserve: f64,
    pub fill: GrowthFill,
    pub decline: DeclineLook,
}

/// A district's art keys (§7.1): keys, never files. `family` is the required
/// last-resort parent of the fallback chain (§7.3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DistrictArt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kit: Option<String>,
    pub family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roof: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walls: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trim_ramps: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lamps: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub props: Option<String>,
}

impl DistrictArt {
    /// Every key, for the well-formedness check.
    fn keys(&self) -> Vec<&str> {
        let one = [
            &self.kit,
            &self.ground,
            &self.roof,
            &self.lamps,
            &self.props,
        ];
        let mut out: Vec<&str> = one.iter().filter_map(|k| k.as_deref()).collect();
        out.push(&self.family);
        for list in [&self.walls, &self.trim_ramps].into_iter().flatten() {
            out.extend(list.iter().map(String::as_str));
        }
        out
    }
}

// ---- Terrain features --------------------------------------------------------

/// The code that draws a terrain feature (an extension point, §6.5).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TerrainGenerator {
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

/// A terrain feature (§6.1): its generator, when it applies, the generator's
/// integer parameters, and art keys.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TerrainFeatureDef {
    /// Content id, e.g. `feature.river`.
    pub id: ContentKey,
    pub generator: TerrainGenerator,
    #[serde(default)]
    pub appears: FeatureAppears,
    /// The generator's parameters: integers (per mille where a share), lists
    /// and tables of integers, or flags. The generator is integer-only.
    #[serde(default)]
    pub params: BTreeMap<String, FeatureParam>,
    /// Art keys by role (`water`, `edge_urban`, `bank`, …).
    pub art: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FeatureAppears {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geo: Option<GeoCondition>,
}

/// One generator parameter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FeatureParam {
    Flag(bool),
    Int(i32),
    List(Vec<i32>),
    Table(Vec<Vec<i32>>),
}

// ---- Connectors ---------------------------------------------------------------

/// The kind of crossing, as the town map's connectors name it.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorKind {
    Bridge,
    Footbridge,
    Ford,
    Viaduct,
    LevelCrossing,
}

/// What carries a road, a path or the railway over water, a creek or the
/// track (§6.1, §7.7). The generator picks, per crossing, the first entry in
/// load order that allows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConnectorDef {
    /// Content id, e.g. `connector.stone_arch`.
    pub id: ContentKey,
    pub kind: ConnectorKind,
    /// The longest crossing, tiles (1..32).
    pub max_span: u32,
    /// Ways that may use it (`arterial`, `street`, `lane`, `path`, `rail`).
    pub roads: Vec<String>,
    /// What it crosses (`river`, `sea`, `lake`, `creek`, `rail`).
    pub crosses: Vec<String>,
    /// Town classes it is built in (empty: everywhere).
    #[serde(default)]
    pub town_class: Vec<String>,
    #[serde(default)]
    pub era: Option<String>,
    /// Art keys by role; `kit` is required.
    pub art: BTreeMap<String, String>,
}

// ---- Place layout blocks ------------------------------------------------------

/// A place kind's `layout` block (§6.3), in `places.jsonc`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceLayoutBlock {
    /// Slot kind (`civic`, `square`, `works`, `community`, `commercial`, `green`, `harbour`).
    pub slot: String,
    /// `[[min w, min h], [max w, max h]]`, tiles (2..16); the largest is tried first.
    pub footprint: [[u32; 2]; 2],
    /// Placement predicates in order of preference.
    pub place: Vec<String>,
    /// The landmark silhouette prop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Companion features the plan reserves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with: Option<Vec<String>>,
    pub landmark: bool,
    pub art: PlaceArt,
}

/// A place's art keys: an optional kit and the required family.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceArt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kit: Option<String>,
    pub family: String,
}

// ---- Validation ---------------------------------------------------------------

fn range_in(r: [u32; 2], lo: u32, hi: u32) -> bool {
    r[0] >= lo && r[1] <= hi && r[0] <= r[1]
}

fn share(v: f64, hi: f64) -> bool {
    v.is_finite() && (0.0..=hi).contains(&v)
}

fn check_geo(g: Option<&GeoCondition>, id: &str, issues: &mut Vec<Issue>) {
    let Some(g) = g else { return };
    for (name, r) in [
        ("fishing_pm", g.fishing_pm),
        ("minerals_pm", g.minerals_pm),
        ("ruggedness_pm", g.ruggedness_pm),
        ("woods_pm", g.woods_pm),
        ("flood_pm", g.flood_pm),
    ] {
        let Some(r) = r else { continue };
        let (lo, hi) = (r.min.unwrap_or(0), r.max.unwrap_or(1000));
        if hi > 1000 || lo > hi {
            issues.push(Issue::error(
                id,
                &format!("{name} must be a range within 0..1000"),
            ));
        }
    }
    for s in g.shore.iter().flatten() {
        if !SHORE_KINDS.contains(&s.as_str()) {
            issues.push(Issue::error(
                id,
                &format!("unknown shore kind `{s}` (beach, rocky, cliff)"),
            ));
        }
    }
}

fn check_art_key(k: &str, id: &str, issues: &mut Vec<Issue>) {
    if !valid_key(k) {
        issues.push(Issue::error(
            id,
            &format!("art key `{k}` must be lowercase dot-separated segments of [a-z0-9_]"),
        ));
    }
}

fn check_list(values: &[String], allowed: &[&str], what: &str, id: &str, issues: &mut Vec<Issue>) {
    for v in values {
        if !allowed.contains(&v.as_str()) {
            issues.push(Issue::error(id, &format!("unknown {what} `{v}`")));
        }
    }
}

/// Town layout v2 registries (DEC-090 D4, TOWN_GENERATION.md §6, §7.2):
/// shapes and ranges the generator relies on, references to place kinds,
/// art keys present and well formed, a district type that always applies for
/// every neighbourhood kind, the base features and connectors, and a layout
/// block for every place kind that is drawn. Ids are unique (claimed on load).
pub(crate) fn validate_layout(b: &ContentBundle, issues: &mut Vec<Issue>) {
    use crate::types::PSEUDO_PLACES;
    let is_place = |k: &str| b.places.iter().any(|p| p.key == k);
    for d in &b.town_districts {
        let id = d.id.as_str();
        if !valid_key(id) {
            issues.push(Issue::error(
                id,
                "content id must be lowercase dot-separated segments of [a-z0-9_]",
            ));
        }
        let name_ok = d.name_key.strip_prefix("district-").is_some_and(|n| {
            !n.is_empty()
                && n.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        });
        if !name_ok {
            issues.push(Issue::error(id, "name_key must be district-<name>"));
        }
        let a = &d.appears;
        if a.hood_kinds.is_empty() {
            issues.push(Issue::error(id, "appears.hood_kinds is empty"));
        }
        check_list(&a.hood_kinds, HOOD_KINDS, "neighbourhood kind", id, issues);
        if a.weight == 0 {
            issues.push(Issue::error(
                id,
                "appears.weight must be a positive integer",
            ));
        }
        check_geo(a.geo.as_ref(), id, issues);
        if let Some(p) = a.population
            && p.min.unwrap_or(0) > p.max.unwrap_or(u32::MAX)
        {
            issues.push(Issue::error(id, "appears.population: min > max"));
        }
        check_list(
            a.town_class.as_deref().unwrap_or_default(),
            TOWN_CLASSES,
            "town class",
            id,
            issues,
        );
        for e in a.era.iter().flatten() {
            if !valid_key(e) {
                issues.push(Issue::error(id, &format!("era `{e}` is not a key")));
            }
        }
        for k in a.requires_place.iter().flatten().chain(&d.hosts) {
            if !is_place(k) {
                issues.push(Issue::error(
                    id,
                    &format!("references unknown place kind `{k}`"),
                ));
            }
        }
        check_list(
            &d.site.prefers,
            SITE_PREDICATES,
            "site predicate",
            id,
            issues,
        );
        check_list(
            d.site.avoids.as_deref().unwrap_or_default(),
            SITE_PREDICATES,
            "site predicate",
            id,
            issues,
        );
        if let Some(anchor) = &d.site.anchor
            && !valid_key(anchor)
        {
            issues.push(Issue::error(id, "site.anchor must be a key"));
        }
        let s = &d.streets;
        if !range_in(s.block, 6, 40) {
            issues.push(Issue::error(
                id,
                "streets.block must be a range within 6..40 (§8: minimum block 6 × 5)",
            ));
        }
        if !range_in(s.depth, 5, 20) {
            issues.push(Issue::error(
                id,
                "streets.depth must be a range within 5..20",
            ));
        }
        let p = &d.plots;
        if !range_in(p.frontage, 2, 12) {
            issues.push(Issue::error(
                id,
                "plots.frontage must be a range within 2..12",
            ));
        }
        if !range_in(p.depth, 2, 8) {
            issues.push(Issue::error(id, "plots.depth must be a range within 2..8"));
        }
        if p.setback > 2 {
            issues.push(Issue::error(id, "plots.setback must be 0..2"));
        }
        if !share(p.joined, 1.0) {
            issues.push(Issue::error(id, "plots.joined must be 0..1"));
        }
        let bl = &d.buildings;
        if bl.mix.is_empty() {
            issues.push(Issue::error(id, "buildings.mix is empty"));
        }
        for (kind, w) in &bl.mix {
            if !FILLER_KINDS.contains(&kind.as_str()) {
                issues.push(Issue::error(
                    id,
                    &format!("unknown filler building kind `{kind}`"),
                ));
            }
            if !(w.is_finite() && *w > 0.0) {
                issues.push(Issue::error(
                    id,
                    &format!("buildings.mix weight of `{kind}` must be > 0"),
                ));
            }
        }
        if !range_in(bl.storeys, 1, 8) {
            issues.push(Issue::error(
                id,
                "buildings.storeys must be a range within 1..8",
            ));
        }
        if !share(bl.density, 1.0) {
            issues.push(Issue::error(id, "buildings.density must be 0..1"));
        }
        if !share(d.growth.reserve, 0.5) {
            issues.push(Issue::error(id, "growth.reserve must be 0..0.5"));
        }
        for t in &d.tags {
            if !valid_key(t) {
                issues.push(Issue::error(id, &format!("tag `{t}` is not a key")));
            }
        }
        if d.art.family.is_empty() {
            issues.push(Issue::error(
                id,
                "art.family is required (the last-resort art parent, §7.2)",
            ));
        }
        for k in d.art.keys() {
            check_art_key(k, id, issues);
        }
    }
    if !b.town_districts.is_empty() || !b.terrain_features.is_empty() {
        for kind in HOOD_KINDS {
            let covered = b.town_districts.iter().any(|d| {
                d.appears.unconditional() && d.appears.hood_kinds.iter().any(|k| k == kind)
            });
            if !covered {
                issues.push(Issue::error(
                    "town_districts",
                    &format!(
                        "no unconditional district type for neighbourhood kind `{kind}` (a pick must always exist)"
                    ),
                ));
            }
        }
    }

    for f in &b.terrain_features {
        let id = f.id.as_str();
        if !valid_key(id) {
            issues.push(Issue::error(
                id,
                "content id must be lowercase dot-separated segments of [a-z0-9_]",
            ));
        }
        check_geo(f.appears.geo.as_ref(), id, issues);
        for k in f.params.keys() {
            if !valid_key(k) || k.contains('.') {
                issues.push(Issue::error(id, &format!("param `{k}` is not a name")));
            }
        }
        if f.art.is_empty() {
            issues.push(Issue::error(id, "art names no keys"));
        }
        for (role, k) in &f.art {
            check_art_key(role, id, issues);
            check_art_key(k, id, issues);
        }
    }

    for c in &b.connectors {
        let id = c.id.as_str();
        if !valid_key(id) {
            issues.push(Issue::error(
                id,
                "content id must be lowercase dot-separated segments of [a-z0-9_]",
            ));
        }
        if !(1..=32).contains(&c.max_span) {
            issues.push(Issue::error(id, "max_span must be 1..32"));
        }
        if c.roads.is_empty() || c.crosses.is_empty() {
            issues.push(Issue::error(id, "roads and crosses must not be empty"));
        }
        check_list(&c.roads, CONNECTOR_ROADS, "road", id, issues);
        check_list(&c.crosses, CONNECTOR_CROSSES, "crossing", id, issues);
        check_list(&c.town_class, TOWN_CLASSES, "town class", id, issues);
        if let Some(e) = &c.era
            && !valid_key(e)
        {
            issues.push(Issue::error(id, &format!("era `{e}` is not a key")));
        }
        if !c.art.contains_key("kit") {
            issues.push(Issue::error(id, "art.kit is required"));
        }
        for (role, k) in &c.art {
            check_art_key(role, id, issues);
            check_art_key(k, id, issues);
        }
    }

    // What the base game's towns always need. Checked once any layout content
    // exists, so a bundle without town layout (tests, tools) still loads.
    let any_layout = !b.town_districts.is_empty()
        || !b.terrain_features.is_empty()
        || !b.connectors.is_empty()
        || b.places.iter().any(|p| p.layout.is_some());
    if any_layout {
        for g in REQUIRED_GENERATORS {
            if !b.terrain_features.iter().any(|f| f.generator == *g) {
                issues.push(Issue::error(
                    "terrain_features",
                    &format!("towns need a `{}` feature", generator_name(*g)),
                ));
            }
        }
        for k in REQUIRED_CONNECTORS {
            if !b.connectors.iter().any(|c| c.kind == *k) {
                issues.push(Issue::error(
                    "connectors",
                    &format!("towns need a `{}` connector", connector_name(*k)),
                ));
            }
        }
    }

    for p in &b.places {
        let id = p.id.as_str();
        let pseudo = PSEUDO_PLACES.contains(&p.key.as_str());
        let Some(l) = &p.layout else {
            if any_layout && !pseudo {
                issues.push(Issue::warning(
                    id,
                    "no layout block: towns draw it with the generator's generic footprint",
                ));
            }
            continue;
        };
        if pseudo {
            issues.push(Issue::error(
                id,
                "a pseudo-location is not a building and has no layout block",
            ));
        }
        if !PLACE_SLOTS.contains(&l.slot.as_str()) {
            issues.push(Issue::error(
                id,
                &format!("unknown layout slot `{}`", l.slot),
            ));
        }
        let [lo, hi] = l.footprint;
        if !(range_in([lo[0], hi[0]], 2, 16) && range_in([lo[1], hi[1]], 2, 16)) {
            issues.push(Issue::error(
                id,
                "layout.footprint must be [[min w, min h], [max w, max h]] within 2..16, min ≤ max",
            ));
        }
        if l.place.is_empty() {
            issues.push(Issue::error(id, "layout.place names no predicate"));
        }
        for pr in &l.place {
            match pr.split_once(':') {
                Some(("toward", t)) => {
                    if !TOWARD_TARGETS.contains(&t) {
                        issues.push(Issue::error(
                            id,
                            &format!("`toward:` takes minerals or woods, not `{t}`"),
                        ));
                    }
                }
                Some(("near_place", k)) => {
                    if !is_place(k) {
                        issues.push(Issue::error(
                            id,
                            &format!("references unknown place kind `{k}`"),
                        ));
                    }
                }
                _ => {
                    if !PLACE_PREDICATES.contains(&pr.as_str()) {
                        issues.push(Issue::error(
                            id,
                            &format!("unknown placement predicate `{pr}`"),
                        ));
                    }
                }
            }
        }
        for k in l.signature.iter().chain(l.with.iter().flatten()) {
            if !valid_key(k) {
                issues.push(Issue::error(id, &format!("`{k}` is not a key")));
            }
        }
        if l.art.family.is_empty() {
            issues.push(Issue::error(id, "layout.art.family is required"));
        }
        for k in l.art.kit.iter().chain(std::iter::once(&l.art.family)) {
            check_art_key(k, id, issues);
        }
    }
}

fn generator_name(g: TerrainGenerator) -> String {
    serde_json::to_value(g)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn connector_name(k: ConnectorKind) -> String {
    serde_json::to_value(k)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}
