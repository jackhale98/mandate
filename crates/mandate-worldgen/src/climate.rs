//! Temperature, precipitation (prevailing wind with rain shadow), and biomes
//! (worldgen steps 3–4). "Plausible, not physical" (DEC-021).

use crate::grid::Grid;
use crate::noise::Noise2;
use mandate_core::SimRng;
use mandate_sim::world::{Biome, Cover};

pub struct ClimateParams {
    /// True when the prevailing wind blows from the west.
    pub wind_from_west: bool,
    /// Multiplier on precipitation (drought presets use < 1).
    pub wetness: f64,
    /// Temperature at the southern (warm) edge and northern edge, °C.
    pub temp_south: f64,
    pub temp_north: f64,
}

pub struct Climate {
    pub temperature: Grid<f64>,
    pub precipitation: Grid<f64>,
}

pub fn generate(
    rng: &mut SimRng,
    elev: &Grid<f64>,
    cover: &Grid<Cover>,
    p: &ClimateParams,
) -> Climate {
    let (w, h) = (elev.w, elev.h);
    let tnoise = Noise2::new(rng);
    let pnoise = Noise2::new(rng);
    let mut temperature = Grid::new(w, h, 0.0);
    for i in 0..elev.len() {
        let (x, y) = elev.xy(i);
        let lat = y as f64 / (h - 1).max(1) as f64; // 0 north → 1 south
        let base = p.temp_north + (p.temp_south - p.temp_north) * lat;
        let alt = elev.data[i].max(0.0) / 1000.0 * 6.5;
        let n = tnoise.fbm(x as f64 * 0.03, y as f64 * 0.03, 3, 2.0, 0.5) * 1.5;
        temperature.set(i, base - alt + n);
    }

    let mut rain = Grid::new(w, h, 0.0);
    for y in 0..h {
        let mut moisture: f64 = 1.0;
        let mut prev_elev = 0.0f64;
        for k in 0..w {
            let x = if p.wind_from_west { k } else { w - 1 - k };
            let i = elev.idx(x, y);
            let e = elev.data[i].max(0.0);
            if cover.data[i] == Cover::Sea {
                moisture = (moisture + 0.12).min(1.0);
                rain.set(i, 0.25 * moisture);
            } else {
                let lift = ((e - prev_elev) / 80.0).max(0.0);
                let r = (0.032 + 0.35 * lift) * moisture;
                rain.set(i, r);
                // Slow depletion plus evapotranspiration recharge over land.
                moisture = (moisture - r * 0.22 + 0.003).clamp(0.12, 1.0);
            }
            prev_elev = e;
        }
    }
    // Smooth, scale to mm/yr, and add regional variation.
    let rain = blur(&blur(&rain));
    let mut precipitation = Grid::new(w, h, 0.0);
    for i in 0..rain.len() {
        let (x, y) = rain.xy(i);
        let n = 1.0 + 0.25 * pnoise.fbm(x as f64 * 0.02, y as f64 * 0.02, 3, 2.0, 0.5);
        let mm = (260.0 + rain.data[i] * 15_000.0) * n * p.wetness;
        precipitation.set(i, mm.clamp(80.0, 3_500.0));
    }
    Climate {
        temperature,
        precipitation,
    }
}

fn blur(g: &Grid<f64>) -> Grid<f64> {
    let mut out = g.clone();
    for i in 0..g.len() {
        let mut sum = g.data[i];
        let mut n = 1.0;
        for j in g.neighbors8(i) {
            sum += g.data[j];
            n += 1.0;
        }
        out.set(i, sum / n);
    }
    out
}

/// Whittaker-style classification with elevation and water overrides.
pub fn biome(
    temp: f64,
    precip: f64,
    elev: f64,
    cover: Cover,
    coastal: bool,
    wet_lowland: bool,
) -> Biome {
    match cover {
        Cover::Sea => return Biome::Ocean,
        Cover::Lake => return Biome::Lake,
        _ => {}
    }
    if elev > 2_200.0 {
        return Biome::Rock;
    }
    if elev > 1_700.0 || temp < -2.0 {
        return Biome::Alpine;
    }
    if coastal && elev < 6.0 {
        return Biome::Beach;
    }
    if wet_lowland {
        return Biome::Wetland;
    }
    if temp < 3.0 {
        return if precip > 450.0 {
            Biome::Boreal
        } else {
            Biome::Tundra
        };
    }
    if temp < 17.0 {
        return match precip {
            p if p < 280.0 => Biome::Desert,
            p if p < 480.0 => Biome::Steppe,
            p if p < 720.0 => Biome::Grassland,
            p if p < 1_500.0 => Biome::TemperateForest,
            _ => Biome::Rainforest,
        };
    }
    match precip {
        p if p < 350.0 => Biome::Desert,
        p if p < 700.0 => Biome::Shrubland,
        p if p < 1_150.0 => Biome::Grassland,
        _ => Biome::Rainforest,
    }
}
