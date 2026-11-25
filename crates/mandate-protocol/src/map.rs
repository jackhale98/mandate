//! Map and place view models (M1). Rasters are base64 so the browser can
//! paint them directly; colours are a presentation concern (frontend palette).

use mandate_content::map_layers::{
    MapLayerBreaks, MapLayerDef, MapLayerFormat, MapLayerMidpoint, MapLayerScale, MapLayerSource,
};
use mandate_sim::Engine;
use mandate_sim::world::{Cover, MunicipalityId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MapMunicipality {
    pub id: u32,
    pub name: String,
    pub region: u32,
    /// `metro` | `city` | `town` | `rural`
    pub class: String,
    pub x: u32,
    pub y: u32,
    pub population: f64,
    /// When the map was loaded, rounded for the player's own town as in
    /// `MapTownFigures`. The map's layers read `MapLayersView`, which is kept
    /// current; this stays for older readers.
    pub unemployment: f64,
    pub port: bool,
    pub regional_capital: bool,
    pub national_capital: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MapRegion {
    pub id: u32,
    pub name: String,
    pub capital: u32,
    pub label_x: f64,
    pub label_y: f64,
    pub population: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MapLink {
    /// `road` | `highway` | `rail`
    pub mode: String,
    pub path: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MapView {
    pub width: u32,
    pub height: u32,
    pub km_per_cell: f64,
    /// One byte per cell: biome discriminant (see `Biome`).
    pub biome_b64: String,
    /// One byte per cell: 0 sea, 1 foreign, 2 country, 3 lake.
    pub cover_b64: String,
    /// One byte per cell: hillshade, 128 = flat.
    pub shade_b64: String,
    /// One byte per cell: river class 0–3.
    pub river_b64: String,
    /// Two bytes (LE) per cell: municipality id, 65535 = none.
    pub owner_b64: String,
    pub municipalities: Vec<MapMunicipality>,
    pub regions: Vec<MapRegion>,
    pub links: Vec<MapLink>,
}

fn class_str(c: mandate_sim::world::SettlementClass) -> &'static str {
    use mandate_sim::world::SettlementClass as S;
    match c {
        S::Metro => "metro",
        S::City => "city",
        S::Town => "town",
        S::Rural => "rural",
    }
}

pub fn map_view(engine: &Engine) -> Option<MapView> {
    let t = engine.terrain()?;
    let st = engine.state();
    let w = &st.world;
    let geo = w.geo.as_ref()?;
    let n = t.w * t.h;
    let mut shade = vec![128u8; n];
    for y in 1..t.h {
        for x in 1..t.w {
            let i = y * t.w + x;
            if t.cover[i] == Cover::Sea {
                continue;
            }
            let d = t.elevation[i].max(0.0) - t.elevation[i - t.w - 1].max(0.0);
            shade[i] = (128.0 + (d as f64 / 120.0).clamp(-0.5, 0.5) * 200.0) as u8;
        }
    }
    let owner: Vec<u8> = t
        .owner
        .iter()
        .flat_map(|o| (if *o == u32::MAX { 65535u16 } else { *o as u16 }).to_le_bytes())
        .collect();
    let capital = w.capital;
    Some(MapView {
        width: t.w as u32,
        height: t.h as u32,
        km_per_cell: geo.km_per_cell,
        biome_b64: base64(&t.biome.iter().map(|b| *b as u8).collect::<Vec<_>>()),
        cover_b64: base64(&t.cover.iter().map(|c| *c as u8).collect::<Vec<_>>()),
        shade_b64: base64(&shade),
        river_b64: base64(&t.river),
        owner_b64: base64(&owner),
        municipalities: w
            .municipalities
            .iter()
            .map(|m| MapMunicipality {
                id: m.id.0,
                name: m.name.clone(),
                region: m.region.0,
                class: class_str(m.class).into(),
                x: m.center.0,
                y: m.center.1,
                population: st.population.municipality_total(w, m.id) as f64,
                unemployment: st.economy.munis.get(m.id.index()).map_or(0.0, |e| {
                    if precise_figures(engine, m.id.0) {
                        e.unemployment
                    } else {
                        crate::office::coarse(e.unemployment, 0.01)
                    }
                }),
                port: m.port,
                regional_capital: w.regions[m.region.index()].capital == m.id,
                national_capital: capital == Some(m.id),
            })
            .collect(),
        regions: w
            .regions
            .iter()
            .map(|r| {
                let k = r.municipalities.len().max(1) as f64;
                let (sx, sy) = r.municipalities.iter().fold((0.0, 0.0), |(a, b), m| {
                    let c = w.municipalities[m.index()].center;
                    (a + c.0 as f64, b + c.1 as f64)
                });
                MapRegion {
                    id: r.id.0,
                    name: r.name.clone(),
                    capital: r.capital.0,
                    label_x: sx / k,
                    label_y: sy / k,
                    population: st.population.region_total(w, r.id) as f64,
                }
            })
            .collect(),
        links: w
            .links
            .iter()
            .map(|l| MapLink {
                mode: match l.mode {
                    mandate_sim::world::LinkMode::Road => "road",
                    mandate_sim::world::LinkMode::Highway => "highway",
                    mandate_sim::world::LinkMode::Rail => "rail",
                }
                .into(),
                path: l.path.clone(),
            })
            .collect(),
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NeighborhoodView {
    pub id: u32,
    pub name: String,
    /// `core` | `inner` | `waterfront` | `industrial` | `suburb` | `rural`
    pub kind: String,
    pub population: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlaceView {
    pub id: u32,
    /// Fluent key suffix, e.g. `town_hall` → `place-town_hall`.
    pub kind: String,
    pub name: String,
    pub neighborhood: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GeoIndexView {
    /// Fluent key suffix, e.g. `fertility` → `geo-fertility`.
    pub key: String,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MunicipalityView {
    pub id: u32,
    pub name: String,
    pub region: String,
    pub class: String,
    pub population: f64,
    pub coastal: bool,
    pub port: bool,
    pub on_river: bool,
    pub elevation_m: f64,
    pub temperature_c: f64,
    pub precipitation_mm: f64,
    /// Biome discriminant name in snake case, e.g. `temperate_forest`.
    pub biome: String,
    pub resources: Vec<GeoIndexView>,
    pub hazards: Vec<GeoIndexView>,
    pub neighborhoods: Vec<NeighborhoodView>,
    pub places: Vec<PlaceView>,
    pub economy: Option<MuniEconomyView>,
    /// Communities with adults here, in registry order (DEC-075). The base
    /// game has one, which the UI does not show.
    pub communities: Vec<CommunityShareView>,
    /// The town's strongest concerns, strongest first (at most three;
    /// DEC-078), as far as the player knows them (DEC-091, M6 note 22): none
    /// until heard of, the top one at `heard`, three from `known`, as the
    /// notebook shows them. Unweighted below `known`.
    pub top_concerns: Vec<crate::office::ConcernView>,
}

/// One community's share of a town's adults (DEC-075).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CommunityShareView {
    /// Community content id, e.g. `community.general`.
    pub id: String,
    /// Fluent key, e.g. `community-general`.
    pub name_key: String,
    /// Share of the town's adults (0..1).
    pub share: f64,
}

/// The communities with adults in a municipality and their shares.
pub fn community_shares(engine: &Engine, m: MunicipalityId) -> Vec<CommunityShareView> {
    let st = engine.state();
    let content = engine.content();
    let adults = st.population.municipality_communities(&st.world, m);
    let total = adults.iter().sum::<u64>().max(1) as f64;
    st.population
        .community_keys
        .iter()
        .zip(&adults)
        .filter(|(_, n)| **n > 0)
        .map(|(id, n)| CommunityShareView {
            id: id.clone(),
            name_key: content
                .community_index(id)
                .map(|i| content.communities[i].name_key.clone())
                .unwrap_or_default(),
            share: *n as f64 / total,
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SectorJobsView {
    /// Sector content key, e.g. `sector.manufacturing` (registry order, DEC-075).
    pub id: String,
    /// Fluent key, e.g. `sector-manufacturing`.
    pub name_key: String,
    pub jobs: f64,
    pub wage: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MuniEconomyView {
    /// For the player's own town the precise figures (sectors, budget, the
    /// trend) are at the town hall; elsewhere in town they are rounded and
    /// the detail is empty (DEC-079 rule 4). Other towns show their public
    /// figures (visiting them is a later milestone).
    pub detailed: bool,
    pub unemployment: f64,
    pub mean_wage: f64,
    /// Middle-class gross income per working-age adult.
    pub typical_income: f64,
    pub monthly_rent: f64,
    pub home_price: f64,
    pub vacancy: f64,
    pub sectors: Vec<SectorJobsView>,
    pub budget: f64,
    pub debt: f64,
    pub last_balance: f64,
    /// Monthly unemployment history (oldest first).
    pub unemployment_history: Vec<f64>,
}

fn snake(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Whether a town's economic figures are served precisely (DEC-079 rule 4):
/// the player's own town is rounded except at its town hall; other towns
/// show their public figures.
fn precise_figures(engine: &Engine, id: u32) -> bool {
    let st = engine.state();
    let home = st.player.as_ref().is_some_and(|p| p.municipality.0 == id);
    !home || mandate_sim::places::player_has_service(st, engine.content(), "town_data_detailed")
}

pub fn municipality_view(engine: &Engine, id: u32) -> Option<MunicipalityView> {
    let st = engine.state();
    let w = &st.world;
    let m = w.municipalities.get(id as usize)?;
    let g = &m.geo;
    let idx = |k: &str, v: f64| GeoIndexView {
        key: k.into(),
        value: v,
    };
    Some(MunicipalityView {
        id,
        name: m.name.clone(),
        region: w.regions[m.region.index()].name.clone(),
        class: class_str(m.class).into(),
        population: st.population.municipality_total(w, MunicipalityId(id)) as f64,
        coastal: m.coastal,
        port: m.port,
        on_river: m.on_river,
        elevation_m: g.elevation_m,
        temperature_c: g.temperature_c,
        precipitation_mm: g.precipitation_mm,
        biome: g
            .dominant_biome
            .map(|b| snake(&format!("{b:?}")))
            .unwrap_or_default(),
        resources: vec![
            idx("fertility", g.fertility),
            idx("timber", g.timber),
            idx("minerals", g.minerals),
            idx("fishing", g.fishing),
            idx("energy", g.energy),
            idx("scenic", g.scenic),
        ],
        hazards: vec![
            idx("flood", g.flood),
            idx("drought", g.drought),
            idx("wildfire", g.wildfire),
            idx("seismic", g.seismic),
            idx("storm", g.storm),
        ],
        neighborhoods: m
            .neighborhoods
            .iter()
            .map(|h| {
                let n = &w.neighborhoods[h.index()];
                NeighborhoodView {
                    id: n.id.0,
                    name: n.name.clone(),
                    kind: snake(&format!("{:?}", n.kind)),
                    population: st.population.hood_total(n.id.index()) as f64,
                }
            })
            .collect(),
        places: m
            .places
            .iter()
            .map(|p| {
                let pl = &w.places[p.index()];
                PlaceView {
                    id: pl.id.0,
                    kind: snake(&format!("{:?}", pl.kind)),
                    name: pl.name.clone(),
                    neighborhood: pl.neighborhood.0,
                }
            })
            .collect(),
        economy: st.economy.munis.get(id as usize).map(|e| {
            let t = st.finance.local.get(id as usize);
            let detailed = precise_figures(engine, id);
            let c = |v: f64, step: f64| {
                if detailed {
                    v
                } else {
                    crate::office::coarse(v, step)
                }
            };
            MuniEconomyView {
                detailed,
                unemployment: c(e.unemployment, 0.01),
                mean_wage: c(e.mean_wage, 1000.0),
                typical_income: c(e.class_income[1], 1000.0),
                monthly_rent: c(e.housing.rent, 10.0),
                home_price: c(e.housing.price, 10_000.0),
                vacancy: c(e.housing.vacancy, 0.01),
                sectors: if detailed {
                    engine
                        .content()
                        .sectors
                        .iter()
                        .zip(&e.sectors)
                        .map(|(d, s)| SectorJobsView {
                            id: d.id.clone(),
                            name_key: d.name_key.clone(),
                            jobs: s.jobs.round(),
                            wage: s.wage.round(),
                        })
                        .collect()
                } else {
                    Vec::new()
                },
                budget: if detailed {
                    t.map(|t| t.budget_total().units() as f64).unwrap_or(0.0)
                } else {
                    0.0
                },
                debt: if detailed {
                    t.map(|t| t.debt.units() as f64).unwrap_or(0.0)
                } else {
                    0.0
                },
                last_balance: if detailed {
                    t.map(|t| t.last_year.balance.units() as f64).unwrap_or(0.0)
                } else {
                    0.0
                },
                unemployment_history: if detailed {
                    e.history.iter().map(|h| h.unemployment as f64).collect()
                } else {
                    Vec::new()
                },
            }
        }),
        communities: community_shares(engine, MunicipalityId(id)),
        // One ranking, cut to what the ledger allows (a query never writes it).
        top_concerns: crate::knowledge::gate_concerns(
            crate::knowledge::Gate::of(engine).town_concerns(MunicipalityId(id)),
            crate::office::town_concerns(engine, MunicipalityId(id), usize::MAX),
            false,
            3,
        ),
    })
}

// ---- Map layers (DEC-089) ----------------------------------------------------

/// A town's category for a categorical layer (its main industry or resource).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MapCategoryFigure {
    /// The category's id: a sector's content id, or a resource key.
    pub id: String,
    /// Fluent key of its name, e.g. `sector-manufacturing`, `geo-timber`.
    pub label_key: String,
    /// Industry: its share of the town's jobs. Resource: its index. 0..1.
    pub share: f64,
}

/// One town's public figures for the map layers. The field names are the
/// layer registry's sources (`mandate_content::map_layers::MapLayerSource`).
/// A gated layer's figure (M7) will be optional here: `null` is "unknown".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MapTownFigures {
    pub id: u32,
    /// Residents.
    pub population: f64,
    /// Residents per km² of the town's land (one decimal).
    pub density: f64,
    /// Share of the labour force out of work (0..1).
    pub unemployment: f64,
    /// Middle-class gross income per working-age adult, a year (whole units).
    pub median_income: f64,
    /// Typical monthly rent (whole units).
    pub rent: f64,
    pub main_industry: Option<MapCategoryFigure>,
    pub main_resource: Option<MapCategoryFigure>,
}

/// A layer's figure in a town, as the layer reads it.
enum LayerValue<'a> {
    Number(f64),
    Category(&'a MapCategoryFigure),
    Unknown,
}

impl MapTownFigures {
    fn value(&self, source: MapLayerSource) -> LayerValue<'_> {
        use LayerValue::*;
        match source {
            MapLayerSource::Population => Number(self.population),
            MapLayerSource::Density => Number(self.density),
            MapLayerSource::Unemployment => Number(self.unemployment),
            MapLayerSource::MedianIncome => Number(self.median_income),
            MapLayerSource::Rent => Number(self.rent),
            MapLayerSource::MainIndustry => self.main_industry.as_ref().map_or(Unknown, Category),
            MapLayerSource::MainResource => self.main_resource.as_ref().map_or(Unknown, Category),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum MapLayerScaleView {
    Sequential,
    Diverging,
    Categorical,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum MapLayerFormatView {
    Count,
    PerKm2,
    Percent,
    Money,
    Category,
}

/// One class of a layer's legend.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MapLegendBin {
    /// Numeric classes: the lower edge (inclusive); `null` for the first
    /// class ("under `hi`").
    pub lo: Option<f64>,
    /// Numeric classes: the upper edge (exclusive); `null` for the last
    /// class ("`lo` and over").
    pub hi: Option<f64>,
    /// Categorical classes: the category's id; `null` for "Other".
    pub category: Option<String>,
    /// Categorical classes: Fluent key of the category's name (`null` for
    /// "Other", which the client names).
    pub label_key: Option<String>,
    /// Towns in this class.
    pub count: u32,
}

/// A map layer from the registry with its legend and each town's class.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MapLayerView {
    /// Content id, e.g. `map_layer.population`.
    pub id: String,
    pub label_key: String,
    pub unit_key: String,
    pub note_key: Option<String>,
    /// Categorical layers: the readout's detail, given `$pct`.
    pub detail_key: Option<String>,
    /// The field of `MapTownFigures` the layer shows.
    pub source: String,
    pub scale: MapLayerScaleView,
    pub format: MapLayerFormatView,
    /// Diverging layers: the midpoint's value (its grey class holds it).
    pub midpoint: Option<f64>,
    /// The legend, lowest class first (categorical: the named categories in
    /// registry order, then Other).
    pub bins: Vec<MapLegendBin>,
    /// Per town, in `MapLayersView.towns` order: the index of its class in
    /// `bins`, or `null` where its figure is unknown.
    pub classes: Vec<Option<u8>>,
    /// When the figures date from, for a layer that is not this month's
    /// public statistics (a gated layer, M7); `null` here.
    pub as_of: Option<crate::views::DateView>,
}

/// The country map's layers (DEC-089): every town's public figures and, per
/// registry layer, its legend and each town's class. Small and refreshed each
/// turn, apart from the map's static raster (`MapView`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MapLayersView {
    /// The date of the figures (this month's public statistics).
    pub date: crate::views::DateView,
    /// In `MapView.municipalities` order.
    pub towns: Vec<MapTownFigures>,
    /// In registry (picker) order.
    pub layers: Vec<MapLayerView>,
}

/// Natural resources in the order the municipality panel lists them.
fn resources(g: &mandate_sim::world::GeoProfile) -> [(&'static str, f64); 6] {
    [
        ("fertility", g.fertility),
        ("timber", g.timber),
        ("minerals", g.minerals),
        ("fishing", g.fishing),
        ("energy", g.energy),
        ("scenic", g.scenic),
    ]
}

/// Every category a categorical source can take, in registry order: (id,
/// Fluent key).
fn category_universe(engine: &Engine, source: MapLayerSource) -> Vec<(String, String)> {
    match source {
        MapLayerSource::MainIndustry => engine
            .content()
            .sectors
            .iter()
            .map(|s| (s.id.clone(), s.name_key.clone()))
            .collect(),
        MapLayerSource::MainResource => resources(&Default::default())
            .iter()
            .map(|(k, _)| (k.to_string(), format!("geo-{k}")))
            .collect(),
        _ => Vec::new(),
    }
}

/// `v` to a multiple of `step`; a fractional step divides by its inverse so
/// the result is the shortest decimal (0.07, not 0.07000000000000001).
fn round_to(v: f64, step: f64) -> f64 {
    if step < 1.0 {
        let inv = (1.0 / step).round();
        (v * inv).round() / inv
    } else {
        (v / step).round() * step
    }
}

/// `v` rounded to two significant figures (no transcendentals: the same
/// result native and in WASM).
fn sig2(v: f64) -> f64 {
    if !v.is_finite() || v == 0.0 {
        return v;
    }
    let a = v.abs();
    let mut scale = 1.0;
    while a / scale >= 100.0 {
        scale *= 10.0;
    }
    while a / scale < 10.0 {
        scale /= 10.0;
    }
    let r = (a / scale).round();
    // Divide by an inverse scale below 1 for the shortest decimal.
    v.signum()
        * if scale < 1.0 {
            r / (1.0 / scale).round()
        } else {
            r * scale
        }
}

/// The "nice" step (1, 2, 2.5 or 5 × 10ⁿ) closest to `x` by ratio.
fn nice_step(x: f64) -> f64 {
    if !(x.is_finite() && x > 0.0) {
        return 1.0;
    }
    let mut p = 1.0;
    while p > x {
        p /= 10.0;
    }
    while p * 10.0 <= x {
        p *= 10.0;
    }
    let mut best = p;
    for c in [1.0, 2.0, 2.5, 5.0, 10.0] {
        let c = c * p;
        let r = |y: f64| if y > x { y / x } else { x / y };
        if r(c) < r(best) {
            best = c;
        }
    }
    best
}

/// Class breaks for a sequential layer: `k - 1` edges, rounded to two
/// significant figures, strictly rising, each leaving a town on either side
/// (so a class can merge away but never be empty at the ends).
fn sequential_edges(sorted: &[f64], k: usize, breaks: MapLayerBreaks) -> Vec<f64> {
    let (Some(&lo), Some(&hi)) = (sorted.first(), sorted.last()) else {
        return Vec::new();
    };
    let n = sorted.len();
    let mut edges: Vec<f64> = Vec::new();
    for i in 1..k {
        let raw = match breaks {
            MapLayerBreaks::Quantile => sorted[(i * n / k).min(n - 1)],
            MapLayerBreaks::Equal => lo + (hi - lo) * i as f64 / k as f64,
        };
        let e = sig2(raw);
        if e > edges.last().copied().unwrap_or(lo) && e <= hi {
            edges.push(e);
        }
    }
    edges
}

/// Class breaks for a diverging layer: `k - 1` edges at equal, nice steps
/// either side of the midpoint, the middle class centred on it. The step is
/// sized so the outer classes start near the 90th percentile of the towns'
/// distance from the midpoint.
fn diverging_edges(values: &[f64], mid: f64, k: usize) -> Vec<f64> {
    let arms = (k.max(3) - 1) / 2;
    let mut d: Vec<f64> = values.iter().map(|v| (v - mid).abs()).collect();
    d.sort_by(f64::total_cmp);
    let spread = d
        .get((d.len().saturating_sub(1)) * 9 / 10)
        .copied()
        .unwrap_or(0.0);
    let raw = if spread > 0.0 {
        spread / (arms as f64 - 0.5)
    } else if mid != 0.0 {
        mid.abs() / 10.0
    } else {
        1.0
    };
    let step = nice_step(raw);
    (0..2 * arms)
        .map(|j| round_to(mid + (j as f64 - arms as f64 + 0.5) * step, step / 10.0))
        .collect()
}

fn class_of(edges: &[f64], v: f64) -> u8 {
    edges.iter().take_while(|e| v >= **e).count() as u8
}

fn numeric_bins(edges: &[f64], classes: &[Option<u8>]) -> Vec<MapLegendBin> {
    (0..=edges.len())
        .map(|i| MapLegendBin {
            lo: i.checked_sub(1).and_then(|j| edges.get(j)).copied(),
            hi: edges.get(i).copied(),
            category: None,
            label_key: None,
            count: classes.iter().filter(|c| **c == Some(i as u8)).count() as u32,
        })
        .collect()
}

/// One registry layer over the towns' figures.
fn layer_view(engine: &Engine, def: &MapLayerDef, towns: &[MapTownFigures]) -> MapLayerView {
    let values: Vec<LayerValue> = towns.iter().map(|t| t.value(def.source)).collect();
    let k = def.legend.classes as usize;
    let mut midpoint = None;
    let (bins, classes) = match def.scale {
        MapLayerScale::Categorical => {
            let universe = category_universe(engine, def.source);
            let index = |c: &MapCategoryFigure| universe.iter().position(|(id, _)| *id == c.id);
            let mut counts = vec![0u32; universe.len()];
            for v in &values {
                if let LayerValue::Category(c) = v
                    && let Some(i) = index(c)
                {
                    counts[i] += 1;
                }
            }
            // The most common categories (ties: registry order), shown in
            // registry order so a category keeps its colour while it stays
            // among them; the rest fold into Other.
            let mut by_count: Vec<usize> = (0..universe.len()).filter(|i| counts[*i] > 0).collect();
            by_count.sort_by(|a, b| counts[*b].cmp(&counts[*a]).then(a.cmp(b)));
            let mut named: Vec<usize> = by_count.iter().take(k).copied().collect();
            named.sort_unstable();
            let other = named.len() as u8;
            let classes: Vec<Option<u8>> = values
                .iter()
                .map(|v| match v {
                    LayerValue::Category(c) => Some(
                        index(c)
                            .and_then(|i| named.iter().position(|n| *n == i))
                            .map_or(other, |p| p as u8),
                    ),
                    _ => None,
                })
                .collect();
            let mut bins: Vec<MapLegendBin> = named
                .iter()
                .map(|i| MapLegendBin {
                    lo: None,
                    hi: None,
                    category: Some(universe[*i].0.clone()),
                    label_key: Some(universe[*i].1.clone()),
                    count: 0,
                })
                .collect();
            if classes.contains(&Some(other)) {
                bins.push(MapLegendBin {
                    lo: None,
                    hi: None,
                    category: None,
                    label_key: None,
                    count: 0,
                });
            }
            for (i, b) in bins.iter_mut().enumerate() {
                b.count = classes.iter().filter(|c| **c == Some(i as u8)).count() as u32;
            }
            (bins, classes)
        }
        MapLayerScale::Sequential | MapLayerScale::Diverging => {
            let known: Vec<(f64, f64)> = values
                .iter()
                .zip(towns)
                .filter_map(|(v, t)| match v {
                    LayerValue::Number(x) if x.is_finite() => Some((*x, t.population)),
                    _ => None,
                })
                .collect();
            let nums: Vec<f64> = known.iter().map(|(v, _)| *v).collect();
            let edges = if def.scale == MapLayerScale::Diverging {
                let mid = match def.midpoint {
                    Some(MapLayerMidpoint::Value(v)) => v,
                    _ => {
                        let pop: f64 = known.iter().map(|(_, p)| p).sum();
                        let sum: f64 = known.iter().map(|(v, p)| v * p).sum();
                        if pop > 0.0 { sum / pop } else { 0.0 }
                    }
                };
                midpoint = Some(mid);
                diverging_edges(&nums, mid, k)
            } else {
                let mut sorted = nums.clone();
                sorted.sort_by(f64::total_cmp);
                sequential_edges(
                    &sorted,
                    k,
                    def.legend.breaks.unwrap_or(MapLayerBreaks::Quantile),
                )
            };
            let classes: Vec<Option<u8>> = values
                .iter()
                .map(|v| match v {
                    LayerValue::Number(x) if x.is_finite() => Some(class_of(&edges, *x)),
                    _ => None,
                })
                .collect();
            (numeric_bins(&edges, &classes), classes)
        }
    };
    MapLayerView {
        id: def.id.clone(),
        label_key: def.label_key.clone(),
        unit_key: def.unit_key.clone(),
        note_key: def.note_key.clone(),
        detail_key: def.detail_key.clone(),
        source: def.source.key().into(),
        scale: match def.scale {
            MapLayerScale::Sequential => MapLayerScaleView::Sequential,
            MapLayerScale::Diverging => MapLayerScaleView::Diverging,
            MapLayerScale::Categorical => MapLayerScaleView::Categorical,
        },
        format: match def.format {
            MapLayerFormat::Count => MapLayerFormatView::Count,
            MapLayerFormat::PerKm2 => MapLayerFormatView::PerKm2,
            MapLayerFormat::Percent => MapLayerFormatView::Percent,
            MapLayerFormat::Money => MapLayerFormatView::Money,
            MapLayerFormat::Category => MapLayerFormatView::Category,
        },
        midpoint,
        bins,
        classes,
        as_of: None,
    }
}

/// Every town's public figures for the map. The player's own town is rounded
/// as the municipality panel rounds it (DEC-079 rule 4).
pub fn map_town_figures(engine: &Engine) -> Vec<MapTownFigures> {
    let st = engine.state();
    let w = &st.world;
    let content = engine.content();
    let km2 = w
        .geo
        .as_ref()
        .map(|g| g.km_per_cell * g.km_per_cell)
        .unwrap_or(1.0);
    // Each resource's mean index over the towns.
    let mut mean = [0.0f64; 6];
    for m in &w.municipalities {
        for (a, (_, v)) in mean.iter_mut().zip(resources(&m.geo)) {
            *a += v / w.municipalities.len() as f64;
        }
    }
    w.municipalities
        .iter()
        .map(|m| {
            let id = m.id.0;
            let precise = precise_figures(engine, id);
            let c = |v: f64, fine: f64, rough: f64| round_to(v, if precise { fine } else { rough });
            let pop = st.population.municipality_total(w, m.id) as f64;
            let e = st.economy.munis.get(m.id.index());
            let main_industry = st
                .politics
                .characters
                .towns
                .get(m.id.index())
                .and_then(|t| {
                    let s = content.sector(t.industry.as_deref()?)?;
                    Some(MapCategoryFigure {
                        id: s.id.clone(),
                        label_key: s.name_key.clone(),
                        share: round_to(t.industry_share as f64, 0.05),
                    })
                });
            // The resource richest for the country's average (as the main
            // industry is the largest for its national share), so a resource
            // common everywhere does not name every town.
            let main_resource = resources(&m.geo)
                .into_iter()
                .zip(&mean)
                .filter(|((_, v), _)| *v > 0.0)
                .fold(None, |best: Option<(&str, f64, f64)>, ((k, v), avg)| {
                    let score = v / avg.max(1e-6);
                    match best {
                        Some((_, _, b)) if b >= score => best,
                        _ => Some((k, v, score)),
                    }
                })
                .map(|(k, v, _)| MapCategoryFigure {
                    id: k.into(),
                    label_key: format!("geo-{k}"),
                    share: round_to(v, 0.01),
                });
            MapTownFigures {
                id,
                population: pop,
                density: round_to(pop / (m.cells.max(1) as f64 * km2), 0.1),
                unemployment: e.map_or(0.0, |e| c(e.unemployment, 0.001, 0.01)),
                median_income: e.map_or(0.0, |e| c(e.class_income[1], 1.0, 1000.0)),
                rent: e.map_or(0.0, |e| c(e.housing.rent, 1.0, 10.0)),
                main_industry,
                main_resource,
            }
        })
        .collect()
}

/// The country map's layers (DEC-089): the towns' figures and every registry
/// layer's legend and classes, all computed here.
pub fn map_layers_view(engine: &Engine) -> MapLayersView {
    let towns = map_town_figures(engine);
    let layers = engine
        .content()
        .map_layers
        .iter()
        .map(|d| layer_view(engine, d, &towns))
        .collect();
    MapLayersView {
        date: crate::views::DateView::of(engine.state().calendar.date),
        towns,
        layers,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_rfc4648() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
    }
}
