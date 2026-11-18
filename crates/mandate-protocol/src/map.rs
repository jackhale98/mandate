//! Map and place view models (M1). Rasters are base64 so the browser can
//! paint them directly; colours are a presentation concern (frontend palette).

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
                unemployment: st
                    .economy
                    .munis
                    .get(m.id.index())
                    .map(|e| e.unemployment)
                    .unwrap_or(0.0),
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SectorJobsView {
    /// Fluent key, e.g. `sector-manufacturing`.
    pub name_key: String,
    pub jobs: f64,
    pub wage: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MuniEconomyView {
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
            MuniEconomyView {
                unemployment: e.unemployment,
                mean_wage: e.mean_wage,
                typical_income: e.class_income[1],
                monthly_rent: e.housing.rent,
                home_price: e.housing.price,
                vacancy: e.housing.vacancy,
                sectors: mandate_content::SectorKind::ALL
                    .iter()
                    .zip(&e.sectors)
                    .map(|(k, s)| SectorJobsView {
                        name_key: engine
                            .content()
                            .sector(*k)
                            .map(|d| d.name_key.clone())
                            .unwrap_or_default(),
                        jobs: s.jobs.round(),
                        wage: s.wage.round(),
                    })
                    .collect(),
                budget: t.map(|t| t.budget_total().units() as f64).unwrap_or(0.0),
                debt: t.map(|t| t.debt.units() as f64).unwrap_or(0.0),
                last_balance: t.map(|t| t.last_year.balance.units() as f64).unwrap_or(0.0),
                unemployment_history: e.history.iter().map(|h| h.unemployment as f64).collect(),
            }
        }),
    })
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
