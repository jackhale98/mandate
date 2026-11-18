//! Per-cell resource and hazard layers (worldgen step 5).

use crate::grid::Grid;
use crate::noise::Noise2;
use mandate_core::SimRng;
use mandate_core::math::clamp01;
use mandate_sim::world::{Biome, Cover};

pub struct CellProfile {
    pub slope: Grid<f64>,
    pub fertility: Grid<f64>,
    pub timber: Grid<f64>,
    pub minerals: Grid<f64>,
    pub fishing: Grid<f64>,
    pub energy: Grid<f64>,
    pub scenic: Grid<f64>,
    pub flood: Grid<f64>,
    pub drought: Grid<f64>,
    pub wildfire: Grid<f64>,
    pub seismic: Grid<f64>,
    pub storm: Grid<f64>,
    pub coastal: Grid<bool>,
}

pub struct Inputs<'a> {
    pub elev: &'a Grid<f64>,
    pub cover: &'a Grid<Cover>,
    pub biome: &'a Grid<Biome>,
    pub river: &'a Grid<u8>,
    pub temp: &'a Grid<f64>,
    pub precip: &'a Grid<f64>,
    pub ridge_dist: &'a Grid<f64>,
    pub mineral_richness: f64,
}

pub fn compute(rng: &mut SimRng, inp: &Inputs<'_>) -> CellProfile {
    let (w, h) = (inp.elev.w, inp.elev.h);
    let ore = Noise2::new(rng);
    let fuel = Noise2::new(rng);
    let mut p = CellProfile {
        slope: Grid::new(w, h, 0.0),
        fertility: Grid::new(w, h, 0.0),
        timber: Grid::new(w, h, 0.0),
        minerals: Grid::new(w, h, 0.0),
        fishing: Grid::new(w, h, 0.0),
        energy: Grid::new(w, h, 0.0),
        scenic: Grid::new(w, h, 0.0),
        flood: Grid::new(w, h, 0.0),
        drought: Grid::new(w, h, 0.0),
        wildfire: Grid::new(w, h, 0.0),
        seismic: Grid::new(w, h, 0.0),
        storm: Grid::new(w, h, 0.0),
        coastal: Grid::new(w, h, false),
    };
    for i in 0..inp.elev.len() {
        let e = inp.elev.data[i];
        let mut max_drop: f64 = 0.0;
        let mut near_sea = false;
        let mut near_lake = false;
        let mut near_river = 0u8;
        for j in inp.elev.neighbors8(i) {
            max_drop = max_drop.max((e - inp.elev.data[j]).abs());
            near_sea |= inp.cover.data[j] == Cover::Sea;
            near_lake |= inp.cover.data[j] == Cover::Lake;
            near_river = near_river.max(inp.river.data[j]);
        }
        let slope = clamp01(max_drop / 250.0);
        p.slope.set(i, slope);
        let land = matches!(inp.cover.data[i], Cover::Country | Cover::Foreign);
        if !land {
            continue;
        }
        p.coastal.set(i, near_sea);
        let (x, y) = inp.elev.xy(i);
        let (fx, fy) = (x as f64 * 0.05, y as f64 * 0.05);
        let t = inp.temp.data[i];
        let pr = inp.precip.data[i];
        let water = near_river.max(inp.river.data[i]);

        let biome_fert = match inp.biome.data[i] {
            Biome::Grassland => 0.9,
            Biome::TemperateForest => 0.7,
            Biome::Rainforest => 0.55,
            Biome::Wetland => 0.5,
            Biome::Steppe => 0.45,
            Biome::Shrubland => 0.35,
            Biome::Boreal => 0.25,
            Biome::Beach => 0.1,
            _ => 0.03,
        };
        let temp_opt = clamp01(1.0 - ((t - 14.0).abs() / 16.0));
        p.fertility.set(
            i,
            clamp01(
                biome_fert * (1.0 - slope) * (0.75 + 0.1 * water as f64) * (0.5 + 0.5 * temp_opt),
            ),
        );
        p.timber.set(
            i,
            match inp.biome.data[i] {
                Biome::Boreal => 0.95,
                Biome::TemperateForest => 0.85,
                Biome::Rainforest => 0.75,
                Biome::Wetland => 0.3,
                Biome::Shrubland => 0.15,
                _ => 0.0,
            },
        );
        let rugged = clamp01(e / 2_000.0 + slope);
        let hotspot = clamp01((ore.ridged(fx * 1.5, fy * 1.5, 3) - 0.55) * 3.0);
        p.minerals.set(
            i,
            clamp01((0.35 * rugged + 0.75 * hotspot) * inp.mineral_richness),
        );
        p.fishing.set(
            i,
            if near_sea {
                0.9
            } else if near_lake {
                0.45
            } else {
                0.0
            },
        );
        let fossil = clamp01((fuel.fbm(fx, fy, 3, 2.0, 0.5) - 0.25) * 2.5) * (1.0 - slope);
        let hydro = if water >= 2 && slope > 0.15 { 0.6 } else { 0.0 };
        let wind = if near_sea || e > 900.0 { 0.35 } else { 0.0 };
        p.energy.set(i, clamp01(fossil.max(hydro).max(wind)));
        let mut sc: f64 = 0.0;
        if near_sea {
            sc += 0.45;
        }
        if near_lake {
            sc += 0.35;
        }
        sc += clamp01(e / 1_500.0) * 0.5;
        if matches!(
            inp.biome.data[i],
            Biome::Rainforest | Biome::TemperateForest | Biome::Beach | Biome::Alpine
        ) {
            sc += 0.15;
        }
        p.scenic.set(i, clamp01(sc));
        let low = clamp01(1.0 - e / 60.0);
        p.flood.set(
            i,
            clamp01(low * (0.35 * water as f64 + if near_sea { 0.4 } else { 0.0 })),
        );
        p.drought.set(
            i,
            clamp01((1.0 - pr / 900.0) * 0.8 + clamp01((t - 12.0) / 20.0) * 0.4),
        );
        let fuel_load = match inp.biome.data[i] {
            Biome::Shrubland | Biome::Steppe => 0.8,
            Biome::TemperateForest | Biome::Boreal => 0.6,
            Biome::Grassland => 0.45,
            _ => 0.1,
        };
        p.wildfire.set(
            i,
            clamp01(fuel_load * clamp01(1.2 - pr / 1_100.0) * clamp01(0.4 + t / 25.0)),
        );
        p.seismic
            .set(i, clamp01(1.0 - inp.ridge_dist.data[i] / 40.0));
        p.storm.set(i, if near_sea { 0.55 } else { 0.05 });
    }
    p
}
