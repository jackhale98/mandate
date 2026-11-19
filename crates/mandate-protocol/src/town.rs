//! Town layout v2 engine views (TOWN_GENERATION §4.1, §4.2; DEC-090 D3).
//!
//! Read-only presentation data for the client's town layout: they never feed
//! back into the simulation, and the layout never decides an outcome.
//!
//! - [`TownGeographyView`] is static for a run: the terrain around the
//!   town's centre (derived from the seed, rebuilt and checked on load), the
//!   world's links and the baselines recorded as play began (§15 E1). Every
//!   value is a quantised integer (metres, per mille, 16-point compass
//!   steps), so a floating-point change in the engine never moves a town.
//! - [`TownStateView`] changes monthly: population against the baselines,
//!   housing, jobs, place status, protections, the last disaster. It follows
//!   DEC-079's information depth: precise only for the player's own town at
//!   the town hall, rounded everywhere else (the map is never a way to read
//!   data the player is not entitled to).

use crate::DateView;
use mandate_sim::Engine;
use mandate_sim::world::{
    Biome, COMPASS_NONE, Cover, LinkMode, MunicipalityId, PlaceStatus, SettlementClass, TerrainData,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Version of the geography view's derivation. A change that alters any
/// value for the same world bumps it (the layout plans from these values).
pub const TOWN_GEOGRAPHY_VERSION: u32 = 1;

/// Side of the terrain patch around the centre (cells).
pub const PATCH_SIZE: usize = 5;

// ---------------------------------------------------------------- compass

/// round(1024 · sin(k · 22.5°)), k = 0..15: the client's table
/// (`web/src/hub/layoutV2/imath.ts`), so both sides agree on every bearing.
const SIN16: [i64; 16] = [
    0, 392, 724, 946, 1024, 946, 724, 392, 0, -392, -724, -946, -1024, -946, -724, -392,
];

/// The unit vector of a bearing, ×1024 (x east, y south).
pub fn compass_vec(k: u8) -> (i64, i64) {
    let i = (k & 15) as usize;
    (SIN16[i], -SIN16[(i + 4) & 15])
}

/// The 16-point bearing nearest a direction (largest dot product, the first
/// on a tie), or `COMPASS_NONE` for a zero vector. Mirrors the client's
/// `bearingOf`.
pub fn bearing_of(dx: i64, dy: i64) -> u8 {
    if dx == 0 && dy == 0 {
        return COMPASS_NONE;
    }
    let mut best = 0u8;
    let mut best_dot = i64::MIN;
    for k in 0..16u8 {
        let (vx, vy) = compass_vec(k);
        let d = dx * vx + dy * vy;
        if d > best_dot {
            best_dot = d;
            best = k;
        }
    }
    best
}

fn opposite(k: u8) -> u8 {
    if k == COMPASS_NONE {
        COMPASS_NONE
    } else {
        (k + 8) & 15
    }
}

/// Steps between two bearings, 0..8.
fn gap(a: u8, b: u8) -> u8 {
    let d = (a & 15).abs_diff(b & 15);
    if d > 8 { 16 - d } else { d }
}

/// The grid step of an 8-point bearing (even values), if any.
fn step_of(k: u8) -> Option<(i64, i64)> {
    Some(match k {
        0 => (0, -1),
        2 => (1, -1),
        4 => (1, 0),
        6 => (1, 1),
        8 => (0, 1),
        10 => (-1, 1),
        12 => (-1, 0),
        14 => (-1, -1),
        _ => return None,
    })
}

/// A `[0, 1]` index as per mille.
fn pm(v: f64) -> u16 {
    (v.clamp(0.0, 1.0) * 1000.0).round() as u16
}

// ---------------------------------------------------------------- geography view

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum TownClass {
    Metro,
    City,
    Town,
    Rural,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum ShoreKind {
    Beach,
    Rocky,
    Cliff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum TownLinkMode {
    Road,
    Highway,
    Rail,
}

/// A neighbourhood as play began.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownHoodGeoView {
    pub id: u32,
    pub name: String,
    /// `core` | `inner` | `waterfront` | `industrial` | `suburb` | `rural`
    pub kind: String,
    /// Residents as play began (`HoodPop::baseline_population`).
    pub baseline_population: u32,
    /// Neighbourhoods of the same town it borders, by id.
    pub adjacent: Vec<u32>,
}

/// Jobs in one sector as play began.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownSectorBaselineView {
    /// Sector content key, e.g. `sector.manufacturing`.
    pub sector: String,
    /// The sector's driver, e.g. `industrial` (DEC-075).
    pub driver: String,
    pub jobs: u32,
}

/// The `PATCH_SIZE`² terrain cells around the centre, row-major, north row
/// first. Cells beyond the map edge repeat the edge cell.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownPatchView {
    pub size: u32,
    /// Whole metres above sea level (negative below).
    pub elevation_m: Vec<i32>,
    /// 0 sea, 1 foreign, 2 country, 3 lake.
    pub cover: Vec<u8>,
    /// Biome discriminant (see the map view).
    pub biome: Vec<u8>,
    /// River class 0–3.
    pub river: Vec<u8>,
    /// Flow direction (16-point compass, even values only; 255 none).
    pub downstream: Vec<u8>,
    pub forest_pm: Vec<u16>,
    pub minerals_pm: Vec<u16>,
}

/// The main river near the centre. Bearings point from the river's
/// nearest point to the centre toward where it comes from and goes to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownRiverView {
    /// 1 stream, 2 river, 3 major river.
    pub class: u8,
    pub enters: u8,
    pub leaves: u8,
    /// Whether it passes within a cell of the centre.
    pub through_centre: bool,
}

/// A stream (class 1) in the patch, besides the main river.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownCreekView {
    pub enters: u8,
    /// Whether it flows into the main river within a few cells.
    pub joins_river: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownCoastView {
    /// Toward the sea.
    pub sea: u8,
    pub shore: ShoreKind,
    /// Sea on several sides but not much of it: an inlet.
    pub bay: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownLakeView {
    pub bearing: u8,
    /// Three or more lake cells within three cells of the centre.
    pub near: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownReliefView {
    /// Local ruggedness, per mille: the patch's relief, 250 m and more = 1000.
    pub ruggedness_pm: u16,
    /// Up the slope of the patch's land (255 when flat).
    pub uphill: u8,
    /// Toward the patch's highest land cell (255 when flat).
    pub high_ground: u8,
    /// Highest minus lowest land cell of the patch, metres.
    pub relief_m: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownWoodsView {
    /// Mean forest of the patch's land, per mille.
    pub share_pm: u16,
    /// Toward the denser woods (255 when even or none).
    pub bearing: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownMineralsView {
    /// The municipality's mineral index, per mille (what sites a mine).
    pub value_pm: u16,
    /// Toward the richer cells of the patch (255 when even or none).
    pub bearing: u8,
}

/// The municipality's resource indexes, per mille.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownResourcesView {
    pub fertility: u16,
    pub timber: u16,
    pub minerals: u16,
    pub fishing: u16,
    pub energy: u16,
    pub scenic: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownHazardsView {
    pub flood: u16,
    pub storm: u16,
    pub wildfire: u16,
    pub seismic: u16,
    pub drought: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownClimateView {
    pub temperature_c: i32,
    pub precipitation_mm: u32,
    /// Dominant biome in snake case, e.g. `temperate_forest`.
    pub biome: String,
    pub snow: bool,
}

/// A transport link leaving the town.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownLinkView {
    pub mode: TownLinkMode,
    /// Where it leaves the town (toward a cell a few steps along its path).
    pub bearing: u8,
    /// The municipality at the other end.
    pub to: u32,
    pub to_name: String,
    /// Across the border (DEC-087; always false until neighbouring nations).
    pub foreign: bool,
}

/// A town's geography for layout v2 (TOWN_GENERATION §4.1): static for a
/// run, quantised to integers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownGeographyView {
    /// `TOWN_GEOGRAPHY_VERSION`.
    pub version: u32,
    pub municipality: u32,
    pub name: String,
    pub class: TownClass,
    pub regional_capital: bool,
    pub national_capital: bool,
    /// Residents as play began.
    pub baseline_population: u32,
    pub neighborhoods: Vec<TownHoodGeoView>,
    /// Jobs per sector as play began (sectors with jobs), registry order.
    pub baseline_sectors: Vec<TownSectorBaselineView>,
    /// Dwellings as play began.
    pub baseline_housing: u32,
    pub patch: TownPatchView,
    pub river: Option<TownRiverView>,
    pub creeks: Vec<TownCreekView>,
    pub coast: Option<TownCoastView>,
    pub lake: Option<TownLakeView>,
    pub relief: TownReliefView,
    pub woods: TownWoodsView,
    pub minerals: TownMineralsView,
    pub resources_pm: TownResourcesView,
    /// By bearing, then mode, then destination.
    pub links: Vec<TownLinkView>,
    pub hazards_pm: TownHazardsView,
    pub climate: TownClimateView,
    /// Where the prevailing wind comes from.
    pub wind_from: u8,
    /// The municipality's mean elevation, whole metres.
    pub elevation_m: i32,
    /// Scenario era tag, if any (none yet).
    pub era: Option<String>,
}

/// Terrain cell access around a centre.
struct Around<'a> {
    t: &'a TerrainData,
    cx: i64,
    cy: i64,
}

impl Around<'_> {
    /// The cell at an offset from the centre, if on the map.
    fn at(&self, dx: i64, dy: i64) -> Option<usize> {
        let (x, y) = (self.cx + dx, self.cy + dy);
        if x < 0 || y < 0 || x >= self.t.w as i64 || y >= self.t.h as i64 {
            None
        } else {
            Some(y as usize * self.t.w + x as usize)
        }
    }
    /// The cell at an offset, clamped to the map.
    fn clamped(&self, dx: i64, dy: i64) -> usize {
        let x = (self.cx + dx).clamp(0, self.t.w as i64 - 1);
        let y = (self.cy + dy).clamp(0, self.t.h as i64 - 1);
        y as usize * self.t.w + x as usize
    }
    fn offset(&self, i: usize) -> (i64, i64) {
        (
            (i % self.t.w) as i64 - self.cx,
            (i / self.t.w) as i64 - self.cy,
        )
    }
    fn is_land(&self, i: usize) -> bool {
        !matches!(self.t.cover[i], Cover::Sea | Cover::Lake)
    }
    fn elev(&self, i: usize) -> i64 {
        self.t.elevation[i].round() as i64
    }
}

/// Cells matching `test` within `rad` of the centre (not the centre): the
/// sum of their offsets, their count, and how many octants they span.
fn scan(a: &Around<'_>, rad: i64, test: &dyn Fn(usize) -> bool) -> (i64, i64, u32, u32) {
    let (mut sx, mut sy, mut n, mut oct) = (0, 0, 0, 0u8);
    for dy in -rad..=rad {
        for dx in -rad..=rad {
            if dx == 0 && dy == 0 {
                continue;
            }
            if a.at(dx, dy).is_some_and(test) {
                sx += dx;
                sy += dy;
                n += 1;
                oct |= 1 << (bearing_of(dx, dy) >> 1);
            }
        }
    }
    (sx, sy, n, oct.count_ones())
}

/// A weighted direction over the patch's land: toward cells where `v` is
/// above the land's mean (255 when even).
fn lean(a: &Around<'_>, v: &dyn Fn(usize) -> i64) -> u8 {
    let r = (PATCH_SIZE / 2) as i64;
    let (mut n, mut sv, mut sxv, mut syv, mut sx, mut sy) = (0i64, 0i64, 0i64, 0i64, 0i64, 0i64);
    for dy in -r..=r {
        for dx in -r..=r {
            let Some(i) = a.at(dx, dy).filter(|i| a.is_land(*i)) else {
                continue;
            };
            let x = v(i);
            n += 1;
            sv += x;
            sxv += dx * x;
            syv += dy * x;
            sx += dx;
            sy += dy;
        }
    }
    // Covariance of offset and value (× n²), so missing sea cells do not bias it.
    bearing_of(n * sxv - sx * sv, n * syv - sy * sv)
}

fn river_summary(
    a: &Around<'_>,
    on_river: bool,
    sea: u8,
) -> (Option<TownRiverView>, Vec<TownCreekView>) {
    let t = a.t;
    let mut max_class = 0u8;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if let Some(i) = a.at(dx, dy) {
                max_class = max_class.max(t.river[i]);
            }
        }
    }
    if max_class == 0 && !on_river {
        return (None, Vec::new());
    }
    let cls = max_class.max(1);
    // The river cell nearest the centre, ring by ring (row-major in a ring).
    let mut start = None;
    'rings: for r in 0..=3i64 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                if let Some(i) = a.at(dx, dy).filter(|i| t.river[*i] >= cls) {
                    start = Some(i);
                    break 'rings;
                }
            }
        }
    }
    let fallback_leaves = if sea != COMPASS_NONE { sea } else { 8 };
    let Some(s) = start else {
        let river = TownRiverView {
            class: cls,
            enters: opposite(fallback_leaves),
            leaves: fallback_leaves,
            through_centre: false,
        };
        return (Some(river), Vec::new());
    };
    let step = |i: usize, k: u8| -> Option<usize> {
        let (dx, dy) = step_of(k)?;
        let (ox, oy) = a.offset(i);
        a.at(ox + dx, oy + dy)
    };
    let mut path = vec![s];
    // Downstream: follow the flow.
    let mut down = s;
    for _ in 0..4 {
        match step(down, t.downstream[down]) {
            Some(n) if t.river[n] > 0 => {
                down = n;
                path.push(n);
            }
            _ => break,
        }
    }
    // Upstream: the largest tributary flowing into each cell.
    let mut up = s;
    for _ in 0..4 {
        let mut best: Option<usize> = None;
        for k in (0..16u8).step_by(2) {
            let Some(n) = step(up, k) else { continue };
            if t.river[n] == 0 || step(n, t.downstream[n]) != Some(up) {
                continue;
            }
            if best.is_none_or(|b| t.river[n] > t.river[b]) {
                best = Some(n);
            }
        }
        match best {
            Some(n) => {
                up = n;
                path.push(n);
            }
            None => break,
        }
    }
    let (sx, sy) = a.offset(s);
    let rel = |i: usize| {
        let (x, y) = a.offset(i);
        bearing_of(x - sx, y - sy)
    };
    let mut leaves = rel(down);
    if leaves == COMPASS_NONE {
        leaves = if t.downstream[s] != COMPASS_NONE {
            t.downstream[s]
        } else {
            COMPASS_NONE
        };
    }
    let mut enters = rel(up);
    if leaves == COMPASS_NONE {
        leaves = if enters != COMPASS_NONE {
            opposite(enters)
        } else {
            fallback_leaves
        };
    }
    // A river crosses the town: the layout draws it from one side of the map
    // to the other, so a bend sharper than 135° within the patch is drawn as
    // the nearest crossing (enters turned away from where it leaves).
    if enters == COMPASS_NONE {
        enters = opposite(leaves);
    } else if gap(enters, leaves) < 6 {
        let (a, b) = ((leaves + 6) & 15, (leaves + 10) & 15);
        enters = if gap(enters, a) <= gap(enters, b) { a } else { b };
    }
    let river = TownRiverView {
        class: cls.min(3),
        enters,
        leaves,
        through_centre: max_class > 0,
    };
    // Creeks: streams in the patch off the main river's course.
    let mut creeks = Vec::new();
    if cls >= 2 {
        let r = (PATCH_SIZE / 2) as i64;
        let mut seen = 0u8;
        for dy in -r..=r {
            for dx in -r..=r {
                let Some(i) = a.at(dx, dy) else { continue };
                if t.river[i] != 1 || path.contains(&i) {
                    continue;
                }
                let b = bearing_of(dx, dy);
                if b == COMPASS_NONE
                    || seen & (1 << (b >> 2)) != 0
                    || gap(b, enters) <= 1
                    || gap(b, leaves) <= 1
                {
                    continue;
                }
                let mut cur = i;
                let mut joins = false;
                for _ in 0..4 {
                    match step(cur, t.downstream[cur]) {
                        Some(n) if t.river[n] >= 2 => {
                            joins = true;
                            break;
                        }
                        Some(n) if t.river[n] > 0 => cur = n,
                        _ => break,
                    }
                }
                seen |= 1 << (b >> 2);
                if creeks.len() < 2 {
                    creeks.push(TownCreekView {
                        enters: b,
                        joins_river: joins,
                    });
                }
            }
        }
    }
    (Some(river), creeks)
}

/// The geography of municipality `id` (`None` for an unknown one, or
/// before the terrain exists).
pub fn town_geography_view(engine: &Engine, id: u32) -> Option<TownGeographyView> {
    let t = engine.terrain()?;
    let st = engine.state();
    let w = &st.world;
    let content = engine.content();
    let m = w.municipalities.get(id as usize)?;
    let g = &m.geo;
    let baseline = st.town_baseline(MunicipalityId(id))?;
    let a = Around {
        t,
        cx: i64::from(m.center.0),
        cy: i64::from(m.center.1),
    };

    // ---- the patch
    let r = (PATCH_SIZE / 2) as i64;
    let cells: Vec<usize> = (-r..=r)
        .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| a.clamped(dx, dy))
        .collect();
    let patch = TownPatchView {
        size: PATCH_SIZE as u32,
        elevation_m: cells.iter().map(|&i| a.elev(i) as i32).collect(),
        cover: cells.iter().map(|&i| t.cover[i] as u8).collect(),
        biome: cells.iter().map(|&i| t.biome[i] as u8).collect(),
        river: cells.iter().map(|&i| t.river[i]).collect(),
        downstream: cells.iter().map(|&i| t.downstream[i]).collect(),
        forest_pm: cells.iter().map(|&i| t.forest_pm[i]).collect(),
        minerals_pm: cells.iter().map(|&i| t.minerals_pm[i]).collect(),
    };

    // ---- relief
    let land: Vec<usize> = (-r..=r)
        .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
        .filter_map(|(dx, dy)| a.at(dx, dy))
        .filter(|i| a.is_land(*i))
        .collect();
    let hi = land.iter().map(|&i| a.elev(i).max(0)).max().unwrap_or(0);
    let lo = land.iter().map(|&i| a.elev(i).max(0)).min().unwrap_or(0);
    let relief_m = (hi - lo) as u32;
    // Local relief: 250 m between the patch's highest and lowest land is
    // fully rugged. (The municipality's ruggedness index saturates for a
    // third of towns, which would make nearly every town hilly.)
    let ruggedness_pm = relief_m.saturating_mul(4).min(1000) as u16;
    let flat = relief_m < 15;
    let uphill = if flat {
        COMPASS_NONE
    } else {
        lean(&a, &|i| a.elev(i).max(0))
    };
    let high_ground = if flat {
        COMPASS_NONE
    } else {
        // The first highest land cell, row-major (the centre has no bearing).
        land.iter()
            .copied()
            .fold(None::<usize>, |b, i| match b {
                Some(b) if a.elev(b) >= a.elev(i) => Some(b),
                _ => Some(i),
            })
            .map(|i| {
                let (dx, dy) = a.offset(i);
                bearing_of(dx, dy)
            })
            .unwrap_or(COMPASS_NONE)
    };

    // ---- the sea
    let is_sea = |i: usize| t.cover[i] == Cover::Sea;
    let mut sea = scan(&a, 3, &is_sea);
    let mut rad = 4;
    while sea.2 == 0 && m.coastal && rad <= 10 {
        sea = scan(&a, rad, &is_sea);
        rad += 2;
    }
    let sea_bearing = if sea.2 > 0 {
        bearing_of(sea.0, sea.1)
    } else {
        COMPASS_NONE
    };
    let coast = (sea.2 > 0 || m.coastal).then(|| {
        // The shore: land cells within three of the centre beside the sea.
        let mut shore = Vec::new();
        for dy in -3..=3i64 {
            for dx in -3..=3i64 {
                let Some(i) = a.at(dx, dy).filter(|i| a.is_land(*i)) else {
                    continue;
                };
                let by_sea = (-1..=1i64)
                    .flat_map(|y| (-1..=1i64).map(move |x| (x, y)))
                    .any(|(x, y)| a.at(dx + x, dy + y).is_some_and(is_sea));
                if by_sea {
                    shore.push(i);
                }
            }
        }
        let n = shore.len() as i64;
        let beaches = shore
            .iter()
            .filter(|&&i| t.biome[i] == Biome::Beach)
            .count() as i64;
        let highest = shore.iter().map(|&i| a.elev(i)).max().unwrap_or(0);
        // Sand where most of the shore is beach; otherwise rock, or cliffs
        // where the land around is steep.
        let shore = if n > 0 && beaches * 2 >= n {
            ShoreKind::Beach
        } else if ruggedness_pm > 700 {
            ShoreKind::Cliff
        } else if ruggedness_pm > 350 || highest >= 20 {
            ShoreKind::Rocky
        } else {
            ShoreKind::Beach
        };
        TownCoastView {
            sea: if sea_bearing == COMPASS_NONE {
                if uphill == COMPASS_NONE {
                    8
                } else {
                    opposite(uphill)
                }
            } else {
                sea_bearing
            },
            shore,
            bay: sea.3 >= 3 && sea.2 <= 24,
        }
    });

    // ---- water
    let (river, creeks) = river_summary(&a, m.on_river, sea_bearing);
    let lake_cells = scan(&a, 3, &|i| t.cover[i] == Cover::Lake);
    let lake = (lake_cells.2 > 0).then(|| TownLakeView {
        bearing: match bearing_of(lake_cells.0, lake_cells.1) {
            COMPASS_NONE => 0,
            b => b,
        },
        near: lake_cells.2 >= 3,
    });

    // ---- woods and minerals
    let woods_share = if land.is_empty() {
        0
    } else {
        (land.iter().map(|&i| u32::from(t.forest_pm[i])).sum::<u32>() / land.len() as u32) as u16
    };
    let woods = TownWoodsView {
        share_pm: woods_share,
        bearing: if woods_share == 0 {
            COMPASS_NONE
        } else {
            lean(&a, &|i| i64::from(t.forest_pm[i]))
        },
    };
    // A seam is worked on land: richer cells toward the sea turn the bearing
    // to the nearer landward side (at least 90° from the sea).
    let mut minerals_bearing = lean(&a, &|i| i64::from(t.minerals_pm[i]));
    if minerals_bearing != COMPASS_NONE
        && sea_bearing != COMPASS_NONE
        && gap(minerals_bearing, sea_bearing) < 4
    {
        let (l, r) = ((sea_bearing + 4) & 15, (sea_bearing + 12) & 15);
        minerals_bearing = if gap(minerals_bearing, l) <= gap(minerals_bearing, r) {
            l
        } else {
            r
        };
    }
    let minerals = TownMineralsView {
        value_pm: pm(g.minerals),
        bearing: minerals_bearing,
    };

    // ---- links
    let centre_cell = a.at(0, 0).unwrap_or(0) as u32;
    let mut links: Vec<TownLinkView> = Vec::new();
    let mut add = |mode: LinkMode, path: &[u32], from_start: bool, to: MunicipalityId| {
        if path.len() < 2 {
            return;
        }
        let k = 4.min(path.len() - 1);
        let c = if from_start {
            path[k]
        } else {
            path[path.len() - 1 - k]
        } as usize;
        let (dx, dy) = a.offset(c);
        let bearing = bearing_of(dx, dy);
        let mode = match mode {
            LinkMode::Road => TownLinkMode::Road,
            LinkMode::Highway => TownLinkMode::Highway,
            LinkMode::Rail => TownLinkMode::Rail,
        };
        if bearing == COMPASS_NONE || links.iter().any(|l| l.mode == mode && l.bearing == bearing) {
            return;
        }
        links.push(TownLinkView {
            mode,
            bearing,
            to: to.0,
            to_name: w
                .municipalities
                .get(to.index())
                .map(|x| x.name.clone())
                .unwrap_or_default(),
            foreign: false,
        });
    };
    for l in &w.links {
        let p = &l.path;
        if l.a == m.id || l.b == m.id {
            let other = if l.a == m.id { l.b } else { l.a };
            let from_start = if p.first() == Some(&centre_cell) {
                true
            } else if p.last() == Some(&centre_cell) {
                false
            } else {
                l.a == m.id
            };
            add(l.mode, p, from_start, other);
        } else if let Some(i) = p.iter().position(|c| *c == centre_cell) {
            // A link passing through the town leaves it both ways.
            add(l.mode, &p[i..], true, l.b);
            add(l.mode, &p[..=i], false, l.a);
        }
    }
    links.sort_by(|x, y| (x.bearing, x.mode, x.to).cmp(&(y.bearing, y.mode, y.to)));

    let snake = |v: &dyn std::fmt::Debug| crate::office::snake(&v);
    let neighborhoods = baseline
        .neighborhoods
        .iter()
        .filter_map(|hb| {
            let n = w.neighborhoods.get(hb.id.index())?;
            let mut adjacent: Vec<u32> = n
                .neighbors
                .iter()
                .filter(|x| {
                    w.neighborhoods
                        .get(x.index())
                        .is_some_and(|o| o.municipality == m.id)
                })
                .map(|x| x.0)
                .collect();
            adjacent.sort_unstable();
            adjacent.dedup();
            Some(TownHoodGeoView {
                id: n.id.0,
                name: n.name.clone(),
                kind: snake(&n.kind),
                baseline_population: hb.population,
                adjacent,
            })
        })
        .collect();
    let baseline_sectors = baseline
        .sectors
        .iter()
        .filter(|s| s.jobs > 0)
        .map(|s| TownSectorBaselineView {
            sector: s.sector.clone(),
            driver: content
                .sectors
                .iter()
                .find(|d| d.id == s.sector)
                .and_then(|d| serde_json::to_value(d.driver).ok())
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default(),
            jobs: s.jobs,
        })
        .collect();

    Some(TownGeographyView {
        version: TOWN_GEOGRAPHY_VERSION,
        municipality: id,
        name: m.name.clone(),
        class: match m.class {
            SettlementClass::Metro => TownClass::Metro,
            SettlementClass::City => TownClass::City,
            SettlementClass::Town => TownClass::Town,
            SettlementClass::Rural => TownClass::Rural,
        },
        regional_capital: w
            .regions
            .get(m.region.index())
            .is_some_and(|r| r.capital == m.id),
        national_capital: w.capital == Some(m.id),
        baseline_population: u32::try_from(baseline.population).unwrap_or(u32::MAX),
        neighborhoods,
        baseline_sectors,
        baseline_housing: baseline.housing_stock,
        patch,
        river,
        creeks,
        coast,
        lake,
        relief: TownReliefView {
            ruggedness_pm,
            uphill,
            high_ground,
            relief_m,
        },
        woods,
        minerals,
        resources_pm: TownResourcesView {
            fertility: pm(g.fertility),
            timber: pm(g.timber),
            minerals: pm(g.minerals),
            fishing: pm(g.fishing),
            energy: pm(g.energy),
            scenic: pm(g.scenic),
        },
        links,
        hazards_pm: TownHazardsView {
            flood: pm(g.flood),
            storm: pm(g.storm),
            wildfire: pm(g.wildfire),
            seismic: pm(g.seismic),
            drought: pm(g.drought),
        },
        climate: TownClimateView {
            temperature_c: g.temperature_c.round() as i32,
            precipitation_mm: g.precipitation_mm.max(0.0).round() as u32,
            biome: g.dominant_biome.map(|b| snake(&b)).unwrap_or_default(),
            snow: g.temperature_c < 2.0,
        },
        wind_from: w.geo.as_ref().map_or(12, |geo| {
            // Degrees with 0 = from the west.
            let steps = (geo.wind_from_deg / 22.5).round() as i64;
            (12 + steps).rem_euclid(16) as u8
        }),
        elevation_m: g.elevation_m.round() as i32,
        era: None,
    })
}

// ---------------------------------------------------------------- state view

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum TownPlaceStatus {
    Open,
    Closed,
    /// Opened during the run (it takes a reserved slot).
    New,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownHoodStateView {
    pub id: u32,
    /// Residents now (rounded to 100 unless detailed).
    pub population: u32,
    /// Against the baseline, per mille (to 10 unless detailed).
    pub population_pm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownHousingView {
    /// Dwellings now (rounded to 100 unless detailed).
    pub stock: u32,
    pub baseline_stock: u32,
    /// Stock against the baseline, per mille.
    pub stock_pm: u32,
    /// Vacant share of the stock, per mille.
    pub vacancy_pm: u32,
    /// Dwellings due to complete in the coming months.
    pub pipeline: u32,
    /// The pipeline as a share of the stock, per mille.
    pub pipeline_pm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownSectorStateView {
    pub sector: String,
    pub jobs: u32,
    pub baseline_jobs: u32,
    /// Against the baseline, per mille.
    pub jobs_pm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownPlaceStateView {
    pub id: u32,
    pub status: TownPlaceStatus,
    /// Whole months since it closed or opened (0 while open since play began).
    pub months: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownProtectionsView {
    /// Damage avoided by defences in force, per mille.
    pub flood_pm: u32,
    pub storm_pm: u32,
    pub wildfire_pm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownDisasterView {
    /// `flood` | `drought` | `wildfire` | `storm` | `earthquake`
    pub hazard: String,
    pub months_ago: u32,
    pub severity_pct: u32,
}

/// A town as it is this month, for layout v2's dress pass (TOWN_GENERATION
/// §4.2). Precise only for the player's own town at the town hall
/// (`detailed`); otherwise counts are rounded to 100, shares to whole
/// percent (10 per mille), and jobs per sector are left out (DEC-079 rule 4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownStateView {
    pub municipality: u32,
    pub date: DateView,
    pub detailed: bool,
    pub population: u32,
    pub population_pm: u32,
    /// In the municipality's neighbourhood order.
    pub neighborhoods: Vec<TownHoodStateView>,
    pub housing: TownHousingView,
    /// Jobs per sector now and as play began (empty unless detailed).
    pub sectors: Vec<TownSectorStateView>,
    pub unemployment_pm: u32,
    /// Every place of the town, by id; closed places are kept (DEC-090 D3).
    pub places: Vec<TownPlaceStateView>,
    pub protections: TownProtectionsView,
    /// The town's most recent disaster still in the event log.
    pub last_disaster: Option<TownDisasterView>,
    /// The region's remaining mineral stock, per mille of the initial stock.
    pub mineral_stock_pm: u32,
}

fn months_between(a: mandate_core::SimDate, b: mandate_core::SimDate) -> u32 {
    let (x, y) = (a.calendar(), b.calendar());
    (y.year * 12 + y.month).saturating_sub(x.year * 12 + x.month)
}

/// The state of municipality `id` this month (`None` for an unknown one).
pub fn town_state_view(engine: &Engine, id: u32) -> Option<TownStateView> {
    let st = engine.state();
    let content = engine.content();
    let w = &st.world;
    let m = w.municipalities.get(id as usize)?;
    let mid = MunicipalityId(id);
    let baseline = st.town_baseline(mid)?;
    let econ = st.economy.munis.get(id as usize)?;
    let home = st.player.as_ref().is_some_and(|p| p.municipality == mid);
    let detailed =
        home && mandate_sim::places::player_has_service(st, content, "town_data_detailed");
    let count = |v: f64| -> u32 {
        let v = v.max(0.0).round();
        let v = if detailed {
            v
        } else {
            (v / 100.0).round() * 100.0
        };
        v.min(f64::from(u32::MAX)) as u32
    };
    let share = |num: f64, den: f64| -> u32 {
        if den <= 0.0 {
            return 1000;
        }
        let v = (num.max(0.0) * 1000.0 / den).round();
        let v = if detailed {
            v
        } else {
            (v / 10.0).round() * 10.0
        };
        v.min(f64::from(u32::MAX)) as u32
    };
    let frac = |v: f64| share(v, 1.0);

    let neighborhoods = baseline
        .neighborhoods
        .iter()
        .map(|hb| {
            let now = st.population.hood_total(hb.id.index()) as f64;
            TownHoodStateView {
                id: hb.id.0,
                population: count(now),
                population_pm: share(now, f64::from(hb.population)),
            }
        })
        .collect();
    let pop_now = st.population.municipality_total(w, mid) as f64;
    let h = &econ.housing;
    let pipeline: f64 = h.pipeline.iter().sum();
    let sectors = if detailed {
        baseline
            .sectors
            .iter()
            .zip(&econ.sectors)
            .map(|(b, s)| TownSectorStateView {
                sector: b.sector.clone(),
                jobs: count(s.jobs),
                baseline_jobs: b.jobs,
                jobs_pm: share(s.jobs, f64::from(b.jobs)),
            })
            .collect()
    } else {
        Vec::new()
    };
    let today = st.calendar.date;
    let places = m
        .places
        .iter()
        .filter_map(|pid| w.places.get(pid.index()))
        .map(|p| {
            let (status, months) = match (p.status, p.opened) {
                (PlaceStatus::Closed { since }, _) => {
                    (TownPlaceStatus::Closed, months_between(since, today))
                }
                (PlaceStatus::Open, Some(opened)) => {
                    (TownPlaceStatus::New, months_between(opened, today))
                }
                (PlaceStatus::Open, None) => (TownPlaceStatus::Open, 0),
            };
            TownPlaceStateView {
                id: p.id.0,
                status,
                months,
            }
        })
        .collect();
    let mods =
        mandate_sim::policy::mods_for(&st.policy, content, id, &|p| st.finance.funded_share(p));
    let last_disaster = st.events.iter().rev().find_map(|ev| match &ev.kind {
        mandate_sim::events::EventKind::Disaster {
            municipality,
            hazard,
            severity_pct,
        } if *municipality == id => Some(TownDisasterView {
            hazard: crate::office::snake(hazard),
            months_ago: months_between(ev.date, today),
            severity_pct: *severity_pct,
        }),
        _ => None,
    });
    Some(TownStateView {
        municipality: id,
        date: DateView::of(today),
        detailed,
        population: count(pop_now),
        population_pm: share(pop_now, baseline.population as f64),
        neighborhoods,
        housing: TownHousingView {
            stock: count(h.stock),
            baseline_stock: baseline.housing_stock,
            stock_pm: share(h.stock, f64::from(baseline.housing_stock)),
            vacancy_pm: frac(h.vacancy),
            pipeline: count(pipeline),
            pipeline_pm: share(pipeline, h.stock),
        },
        sectors,
        unemployment_pm: frac(econ.unemployment),
        places,
        protections: TownProtectionsView {
            flood_pm: frac(mods.flood_protection),
            storm_pm: frac(mods.storm_protection),
            wildfire_pm: frac(mods.wildfire_protection),
        },
        last_disaster,
        mineral_stock_pm: frac(
            st.environment
                .regions
                .get(m.region.index())
                .map_or(1.0, |r| r.mineral_stock),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearings_match_the_client_table() {
        assert_eq!(bearing_of(0, -1), 0);
        assert_eq!(bearing_of(1, 0), 4);
        assert_eq!(bearing_of(0, 1), 8);
        assert_eq!(bearing_of(-1, 0), 12);
        assert_eq!(bearing_of(1, -1), 2);
        assert_eq!(bearing_of(-3, -1), 13);
        assert_eq!(bearing_of(0, 0), COMPASS_NONE);
        for k in (0..16u8).step_by(2) {
            let (dx, dy) = step_of(k).unwrap_or((0, 0));
            assert_eq!(bearing_of(dx, dy), k);
            assert_eq!(mandate_sim::world::compass8(dx as i32, dy as i32), k);
        }
        assert_eq!(gap(1, 15), 2);
        assert_eq!(gap(0, 8), 8);
        assert_eq!(opposite(3), 11);
    }
}
