//! Lab map rendering: biome colours, hillshade, rivers, borders, roads, towns.

use mandate_sim::world::{Biome, Cover, LinkMode, SettlementClass, TerrainData, World};

pub fn biome_rgb(b: Biome) -> [u8; 3] {
    match b {
        Biome::Ocean => [38, 70, 112],
        Biome::Lake => [64, 110, 160],
        Biome::Beach => [222, 208, 160],
        Biome::Wetland => [92, 128, 104],
        Biome::Grassland => [150, 180, 96],
        Biome::TemperateForest => [74, 128, 70],
        Biome::Rainforest => [44, 104, 64],
        Biome::Boreal => [70, 104, 84],
        Biome::Shrubland => [168, 160, 98],
        Biome::Steppe => [190, 178, 120],
        Biome::Desert => [214, 192, 138],
        Biome::Tundra => [170, 170, 150],
        Biome::Alpine => [168, 164, 160],
        Biome::Rock => [130, 124, 120],
    }
}

pub fn render(t: &TerrainData, world: &World, scale: usize) -> (u32, u32, Vec<u8>) {
    let (w, h) = (t.w * scale, t.h * scale);
    let mut px = vec![0u8; w * h * 3];
    let at = |x: usize, y: usize| y * t.w + x;
    for y in 0..t.h {
        for x in 0..t.w {
            let i = at(x, y);
            let mut c = biome_rgb(t.biome[i]);
            // Hillshade from the north-west.
            let e = t.elevation[i].max(0.0);
            let enw = if x > 0 && y > 0 {
                t.elevation[at(x - 1, y - 1)].max(0.0)
            } else {
                e
            };
            let shade = ((e - enw) as f64 / 120.0).clamp(-0.35, 0.35);
            for ch in c.iter_mut() {
                *ch = ((*ch as f64) * (1.0 + shade)).clamp(0.0, 255.0) as u8;
            }
            if t.cover[i] == Cover::Foreign {
                for ch in c.iter_mut() {
                    *ch = (*ch as u16 * 5 / 10 + 60) as u8;
                }
            }
            if t.river[i] >= 2 && t.cover[i] != Cover::Sea {
                c = if t.river[i] == 3 {
                    [52, 96, 170]
                } else {
                    [80, 128, 190]
                };
            }
            // Municipal and regional borders.
            let o = t.owner[i];
            if o != u32::MAX {
                let r = world.municipalities[o as usize].region;
                let mut muni_border = false;
                let mut region_border = false;
                for (dx, dy) in [(1i32, 0i32), (0, 1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < t.w as i32 && ny < t.h as i32 {
                        let on = t.owner[at(nx as usize, ny as usize)];
                        if on != o {
                            muni_border = true;
                            if on == u32::MAX || world.municipalities[on as usize].region != r {
                                region_border = true;
                            }
                        }
                    }
                }
                if region_border {
                    c = [40, 30, 30];
                } else if muni_border {
                    for ch in c.iter_mut() {
                        *ch = (*ch as u16 * 8 / 10) as u8;
                    }
                }
            }
            for sy in 0..scale {
                for sx in 0..scale {
                    let p = ((y * scale + sy) * w + x * scale + sx) * 3;
                    px[p..p + 3].copy_from_slice(&c);
                }
            }
        }
    }
    let mut dot = |cx: usize, cy: usize, r: usize, c: [u8; 3]| {
        for y in cy.saturating_sub(r)..(cy + r + 1).min(h) {
            for x in cx.saturating_sub(r)..(cx + r + 1).min(w) {
                let p = (y * w + x) * 3;
                px[p..p + 3].copy_from_slice(&c);
            }
        }
    };
    for l in &world.links {
        let c = match l.mode {
            LinkMode::Rail => [60, 60, 60],
            LinkMode::Highway => [200, 70, 50],
            LinkMode::Road => [140, 100, 70],
        };
        for &cell in &l.path {
            let (x, y) = (cell as usize % t.w, cell as usize / t.w);
            dot(x * scale + scale / 2, y * scale + scale / 2, 0, c);
        }
    }
    for (k, m) in world.municipalities.iter().enumerate() {
        let r = match m.class {
            SettlementClass::Metro => 4,
            SettlementClass::City => 3,
            SettlementClass::Town => 2,
            SettlementClass::Rural => 1,
        };
        let c = if k == 0 {
            [255, 215, 0]
        } else {
            [250, 250, 250]
        };
        let (x, y) = (
            m.center.0 as usize * scale + scale / 2,
            m.center.1 as usize * scale + scale / 2,
        );
        dot(x, y, r + 1, [20, 20, 20]);
        dot(x, y, r, c);
    }
    (w as u32, h as u32, px)
}
