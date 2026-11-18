//! Elevation and country shape (worldgen steps 1, 9a).

use crate::grid::{Grid, N4};
use crate::noise::Noise2;
use mandate_core::SimRng;
use mandate_core::math::{exp, pow, sqrt};
use mandate_sim::world::Cover;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    North,
    East,
    South,
    West,
}

pub struct TerrainParams {
    /// Sides of the map bordered by sea (others border foreign land).
    pub sea_sides: Vec<Side>,
    /// Fraction of map cells below sea level.
    pub sea_fraction: f64,
    pub mountain_strength: f64,
    /// Multiplier on the country outline radius (>1 reaches further to the coasts).
    pub country_extent: f64,
}

pub struct Terrain {
    pub elevation: Grid<f64>,
    pub cover: Grid<Cover>,
    /// Distance (cells) to the nearest mountain ridge line.
    pub ridge_dist: Grid<f64>,
}

fn side_distance(side: Side, u: f64, v: f64) -> f64 {
    match side {
        Side::North => v,
        Side::South => 1.0 - v,
        Side::West => u,
        Side::East => 1.0 - u,
    }
}

fn dist_to_segment(px: f64, py: f64, a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (cx, cy) = (a.0 + t * dx, a.1 + t * dy);
    sqrt((px - cx) * (px - cx) + (py - cy) * (py - cy))
}

/// Random polyline crossing the map (a mountain range axis), in cell units.
fn ridge_line(rng: &mut SimRng, w: f64, h: f64) -> Vec<(f64, f64)> {
    let horizontal = rng.chance(0.5);
    let n = 4;
    let mut pts = Vec::new();
    let offset = rng.range_f64(0.25, 0.75);
    for i in 0..n {
        let t = i as f64 / (n - 1) as f64;
        let jitter = rng.range_f64(-0.12, 0.12);
        let along = rng.range_f64(0.1, 0.25) + t * rng.range_f64(0.5, 0.65);
        if horizontal {
            pts.push((along * w, (offset + jitter) * h));
        } else {
            pts.push(((offset + jitter) * w, along * h));
        }
    }
    pts
}

pub fn generate(rng: &mut SimRng, w: usize, h: usize, p: &TerrainParams) -> Terrain {
    let base = Noise2::new(rng);
    let detail = Noise2::new(rng);
    let warp = Noise2::new(rng);
    let ridge = Noise2::new(rng);
    let shape = Noise2::new(rng);

    let mut ranges = vec![ridge_line(rng, w as f64, h as f64)];
    if rng.chance(0.6) {
        ranges.push(ridge_line(rng, w as f64, h as f64));
    }
    let range_width = (w.min(h) as f64) * rng.range_f64(0.06, 0.1);

    let mut raw = Grid::new(w, h, 0.0f64);
    let mut ridge_dist = Grid::new(w, h, f64::MAX);
    let aspect = w as f64 / h as f64;
    for i in 0..raw.len() {
        let (x, y) = raw.xy(i);
        let (u, v) = (x as f64 / w as f64, y as f64 / h as f64);
        let (su, sv) = (u * 3.0 * aspect, v * 3.0);
        let wx = warp.fbm(su * 0.7, sv * 0.7, 3, 2.0, 0.5) * 0.6;
        let wy = warp.fbm(su * 0.7 + 9.1, sv * 0.7 - 4.3, 3, 2.0, 0.5) * 0.6;
        let mut e = 0.5
            + 0.45 * base.fbm(su + wx, sv + wy, 5, 2.0, 0.5)
            + 0.12 * detail.fbm(su * 4.0, sv * 4.0, 3, 2.0, 0.5);

        // Sea along the configured sides.
        for side in &p.sea_sides {
            let d = side_distance(*side, u, v) + 0.08 * shape.fbm(su * 1.5, sv * 1.5, 3, 2.0, 0.5);
            if d < 0.38 {
                e -= pow((0.38 - d) / 0.38, 1.6) * 0.9;
            }
        }
        // Mountain ranges.
        let mut dmin = f64::MAX;
        for r in &ranges {
            for seg in r.windows(2) {
                dmin = dmin.min(dist_to_segment(x as f64, y as f64, seg[0], seg[1]));
            }
        }
        ridge_dist.set(i, dmin);
        let falloff = exp(-(dmin * dmin) / (2.0 * range_width * range_width));
        e += p.mountain_strength * falloff * (0.35 + 0.65 * ridge.ridged(su * 2.5, sv * 2.5, 5));
        raw.set(i, e);
    }

    // Sea level at the requested quantile.
    let mut sorted: Vec<f64> = raw.data.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let sea_idx = ((sorted.len() as f64 * p.sea_fraction) as usize).min(sorted.len() - 1);
    let sea = sorted[sea_idx];
    let max = sorted[sorted.len() - 1].max(sea + 1e-6);
    let min = sorted[0].min(sea - 1e-6);

    let mut elevation = Grid::new(w, h, 0.0f64);
    for i in 0..raw.len() {
        let e = raw.data[i];
        let m = if e >= sea {
            pow((e - sea) / (max - sea), 1.75) * 3000.0 + 1.0
        } else {
            -((sea - e) / (sea - min)) * 2500.0 - 1.0
        };
        elevation.set(i, m);
    }

    // Seas connected to sea sides stay sea; inland basins below sea level
    // become low land (lakes are found later by hydrology).
    let mut cover = Grid::new(w, h, Cover::Foreign);
    let mut queue = VecDeque::new();
    for i in 0..elevation.len() {
        let (x, y) = elevation.xy(i);
        let on_sea_side = p.sea_sides.iter().any(|s| match s {
            Side::North => y == 0,
            Side::South => y + 1 == h,
            Side::West => x == 0,
            Side::East => x + 1 == w,
        });
        if on_sea_side && elevation.data[i] < 0.0 {
            cover.set(i, Cover::Sea);
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        for d in N4 {
            if let Some(n) = elevation.offset(i, d)
                && cover.data[n] != Cover::Sea
                && elevation.data[n] < 0.0
            {
                cover.set(n, Cover::Sea);
                queue.push_back(n);
            }
        }
    }
    for i in 0..elevation.len() {
        if cover.data[i] != Cover::Sea && elevation.data[i] < 1.0 {
            elevation.set(i, 1.0 + (elevation.data[i].abs() % 7.0));
        }
    }

    // Country: a noise-warped ellipse over land, biased away from foreign
    // edges, then the largest connected component.
    let (cx, cy) = (rng.range_f64(0.42, 0.58), rng.range_f64(0.42, 0.58));
    let (rx, ry) = (
        rng.range_f64(0.36, 0.44) * p.country_extent,
        rng.range_f64(0.36, 0.44) * p.country_extent,
    );
    for i in 0..cover.len() {
        if cover.data[i] == Cover::Sea {
            continue;
        }
        let (x, y) = cover.xy(i);
        let (u, v) = (x as f64 / w as f64, y as f64 / h as f64);
        let dx = (u - cx) / rx;
        let dy = (v - cy) / ry;
        let r = sqrt(dx * dx + dy * dy) + 0.22 * shape.fbm(u * 4.0 * aspect, v * 4.0, 4, 2.0, 0.5);
        if r < 1.0 {
            cover.set(i, Cover::Country);
        }
    }
    keep_largest_country_component(&mut cover);
    Terrain {
        elevation,
        cover,
        ridge_dist,
    }
}

fn keep_largest_country_component(cover: &mut Grid<Cover>) {
    let mut comp = vec![u32::MAX; cover.len()];
    let mut sizes: Vec<usize> = Vec::new();
    for start in 0..cover.len() {
        if cover.data[start] != Cover::Country || comp[start] != u32::MAX {
            continue;
        }
        let id = sizes.len() as u32;
        let mut size = 0;
        let mut q = VecDeque::from([start]);
        comp[start] = id;
        while let Some(i) = q.pop_front() {
            size += 1;
            for d in N4 {
                if let Some(n) = cover.offset(i, d)
                    && cover.data[n] == Cover::Country
                    && comp[n] == u32::MAX
                {
                    comp[n] = id;
                    q.push_back(n);
                }
            }
        }
        sizes.push(size);
    }
    let Some(best) = sizes
        .iter()
        .enumerate()
        .max_by_key(|(i, s)| (**s, usize::MAX - i))
        .map(|(i, _)| i as u32)
    else {
        return;
    };
    for i in 0..cover.len() {
        if cover.data[i] == Cover::Country && comp[i] != best {
            cover.set(i, Cover::Foreign);
        }
    }
}
