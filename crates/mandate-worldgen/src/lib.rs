// Grid algorithms index several parallel rasters by cell; range loops are clearer here.
#![allow(clippy::needless_range_loop)]

//! World generation pipeline (DEC-021): terrain → climate → hydrology →
//! biomes → resources/hazards → settlements → territories → regions →
//! transport → neighbourhoods → districts → places. Deterministic from the
//! seed; all randomness comes from the `Worldgen` stream.

pub mod climate;
pub mod districts;
pub mod grid;
pub mod hoods;
pub mod hydrology;
pub mod names;
pub mod noise;
pub mod places;
pub mod profile;
pub mod settle;
pub mod terrain;
pub mod transport;

use grid::{Grid, HeapItem, cell_dist};
use mandate_content::ContentBundle;
use mandate_core::hash::StableHash;
use mandate_core::{ENGINE_VERSION, RngStreams, SimDate, SimRng, Stream};
use mandate_sim::events::EventLog;
use mandate_sim::state::{Calendar, GamePhase, State, WorldMeta};
pub use mandate_sim::state::{Preset, WorldParams, WorldSize};
use mandate_sim::world::*;
use mandate_sim::{Engine, EngineError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldConfig {
    pub seed: u64,
    pub size: WorldSize,
    pub preset: Option<Preset>,
}

impl WorldConfig {
    pub fn new(seed: u64, size: WorldSize) -> Self {
        WorldConfig {
            seed,
            size,
            preset: None,
        }
    }
}

struct Dims {
    w: usize,
    h: usize,
    municipalities: usize,
    regions: usize,
    national_districts: usize,
    population: u64,
}

fn dims(size: WorldSize) -> Dims {
    match size {
        WorldSize::Small => Dims {
            w: 180,
            h: 120,
            municipalities: 30,
            regions: 6,
            national_districts: 30,
            population: 5_000_000,
        },
        WorldSize::Medium => Dims {
            w: 240,
            h: 160,
            municipalities: 56,
            regions: 10,
            national_districts: 50,
            population: 12_000_000,
        },
        WorldSize::Large => Dims {
            w: 300,
            h: 200,
            municipalities: 84,
            regions: 13,
            national_districts: 75,
            population: 24_000_000,
        },
    }
}

pub const KM_PER_CELL: f64 = 2.5;

/// Progress callback: (Fluent stage key, fraction 0..1).
pub type Progress<'a> = &'a mut dyn FnMut(&str, f64);

/// Output of the geography pipeline.
pub struct Generated {
    pub world: World,
    pub terrain: TerrainData,
    /// Starting residents per neighbourhood (seeds the population domain).
    pub hood_population: Vec<u64>,
}

/// Generate the world data and the derived terrain raster.
pub fn generate_world(
    config: &WorldConfig,
    content: &ContentBundle,
    progress: Progress<'_>,
) -> Result<Generated, EngineError> {
    let pool = |id: &str| {
        content
            .name_pools
            .get(id)
            .ok_or_else(|| EngineError::Content(format!("missing name pool {id}")))
    };
    let (place_pool, region_pool, hood_pool, given_pool) = (
        pool("names.place")?,
        pool("names.region")?,
        pool("names.neighborhood")?,
        pool("names.given")?,
    );

    let d = dims(config.size);
    let mut streams = RngStreams::new(config.seed);
    let rng: &mut SimRng = streams.get(Stream::Worldgen);
    let preset = config.preset;

    // --- Parameters (presets bias the canonical test worlds) ---------------
    let mut sides = vec![
        terrain::Side::North,
        terrain::Side::East,
        terrain::Side::South,
        terrain::Side::West,
    ];
    rng.shuffle(&mut sides);
    let n_sea = match preset {
        Some(Preset::WealthyCoast) => 3,
        Some(Preset::DroughtFarmland) | Some(Preset::ResourceFrontier) => 1,
        _ => 1 + rng.below(3) as usize,
    };
    let sea_sides: Vec<terrain::Side> = sides[..n_sea].to_vec();
    let tp = terrain::TerrainParams {
        sea_fraction: 0.14 + 0.08 * n_sea as f64,
        mountain_strength: match preset {
            Some(Preset::ResourceFrontier) => 0.85,
            Some(Preset::DroughtFarmland) => 0.35,
            _ => rng.range_f64(0.45, 0.7),
        },
        sea_sides,
        country_extent: if preset == Some(Preset::WealthyCoast) {
            1.3
        } else {
            1.0
        },
    };
    let wind_from_west = rng.chance(0.75);
    let cp = climate::ClimateParams {
        wind_from_west,
        wetness: match preset {
            Some(Preset::DroughtFarmland) => 0.5,
            Some(Preset::WealthyCoast) => 1.15,
            _ => rng.range_f64(0.85, 1.15),
        },
        temp_south: if preset == Some(Preset::DroughtFarmland) {
            24.0
        } else {
            rng.range_f64(16.0, 21.0)
        },
        temp_north: rng.range_f64(3.0, 8.0),
    };
    let mineral_richness = match preset {
        Some(Preset::ResourceFrontier) => 1.6,
        Some(Preset::IndustrialDecline) => 1.3,
        _ => 1.0,
    };
    let total_pop = match preset {
        Some(Preset::ResourceFrontier) => d.population / 2,
        Some(Preset::TechGrowth) => d.population * 6 / 5,
        _ => d.population,
    };

    // --- 1. Terrain ---------------------------------------------------------
    progress("progress-terrain", 0.05);
    let t = terrain::generate(rng, d.w, d.h, &tp);
    let elev = t.elevation;
    let mut cover = t.cover;

    // --- 2–3. Climate and hydrology ----------------------------------------
    progress("progress-climate", 0.2);
    let clim = climate::generate(rng, &elev, &cover, &cp);
    let filled = hydrology::fill_depressions(&elev, &cover);
    let hyd = hydrology::route(&elev, &filled, &mut cover, &clim.precipitation);

    // --- 4. Biomes ----------------------------------------------------------
    progress("progress-biomes", 0.35);
    let wet = noise::Noise2::new(rng);
    let mut biome = Grid::new(d.w, d.h, Biome::Ocean);
    for i in 0..elev.len() {
        let coastal = cover.neighbors8(i).any(|j| cover.data[j] == Cover::Sea);
        let (x, y) = elev.xy(i);
        let wet_lowland = hyd.river.data[i] >= 2
            && elev.data[i] < 80.0
            && clim.precipitation.data[i] > 850.0
            && wet.get(x as f64 * 0.15, y as f64 * 0.15) > 0.15;
        biome.set(
            i,
            climate::biome(
                clim.temperature.data[i],
                clim.precipitation.data[i],
                elev.data[i],
                cover.data[i],
                coastal,
                wet_lowland,
            ),
        );
    }

    // --- 5. Resources and hazards ------------------------------------------
    let prof = profile::compute(
        rng,
        &profile::Inputs {
            elev: &elev,
            cover: &cover,
            biome: &biome,
            river: &hyd.river,
            temp: &clim.temperature,
            precip: &clim.precipitation,
            ridge_dist: &t.ridge_dist,
            mineral_richness,
        },
    );

    // --- 6. Settlements ----------------------------------------------------
    progress("progress-settlements", 0.5);
    let country_cells = cover.data.iter().filter(|c| **c == Cover::Country).count();
    if country_cells < d.municipalities * 8 {
        return Err(EngineError::Content("generated country too small".into()));
    }
    let scores: Vec<(usize, f64)> = (0..elev.len())
        .filter_map(|i| {
            settle::site_score(i, &cover, &biome, &hyd.river, &elev, &prof).map(|s| (i, s))
        })
        .collect();
    let mut sites = settle::choose_sites(rng, &scores, d.w, country_cells, d.municipalities);
    let score_of: BTreeMap<usize, f64> = scores.iter().copied().collect();
    sites.sort_by(|a, b| score_of[b].total_cmp(&score_of[a]).then(a.cmp(b)));
    let pops = settle::populations(rng, sites.len(), total_pop);
    let n_m = sites.len();

    // --- 7. Territories ------------------------------------------------------
    progress("progress-territories", 0.6);
    let owner = settle::territories(&sites, &cover, &hyd.river, &prof);
    let adjacency = settle::adjacency(&owner, n_m);

    // Aggregate a geographic profile per municipality.
    let mut acc: Vec<Vec<usize>> = vec![Vec::new(); n_m];
    for i in 0..owner.len() {
        let o = owner.data[i];
        if o != u32::MAX {
            acc[o as usize].push(i);
        }
    }
    let mut used_names = BTreeSet::new();
    let mut municipalities: Vec<Municipality> = Vec::with_capacity(n_m);
    for k in 0..n_m {
        let cells = &acc[k];
        let geo = aggregate(cells, &elev, &clim, &prof, &biome);
        let coastal = cells.iter().any(|&i| prof.coastal.data[i]);
        let on_river = cells.iter().any(|&i| hyd.river.data[i] >= 2);
        let (cx, cy) = elev.xy(sites[k]);
        municipalities.push(Municipality {
            id: MunicipalityId(k as u32),
            name: names::unique(place_pool, rng, &mut used_names),
            region: RegionId(0),
            class: settle::class_for(pops[k], k),
            center: (cx as u32, cy as u32),
            cells: cells.len() as u32,
            coastal,
            on_river,
            port: false,
            neighbors: adjacency[k].iter().map(|&n| MunicipalityId(n)).collect(),
            neighborhoods: Vec::new(),
            places: Vec::new(),
            geo,
        });
    }
    // Ports: coastal municipalities with gentle coasts; at least one if any coast.
    for m in municipalities.iter_mut() {
        m.port = m.coastal && m.class != SettlementClass::Rural && m.geo.ruggedness < 0.45;
    }
    if !municipalities.iter().any(|m| m.port)
        && let Some(m) = municipalities.iter_mut().find(|m| m.coastal)
    {
        m.port = true;
    }

    // --- 8. Regions -----------------------------------------------------------
    progress("progress-regions", 0.68);
    let region_of = assign_regions(&municipalities, &pops, d.regions, &elev, &hyd.river);
    let n_r = region_of.iter().map(|r| r + 1).max().unwrap_or(1);
    let mut regions: Vec<Region> = (0..n_r)
        .map(|r| Region {
            id: RegionId(r as u32),
            name: names::unique(region_pool, rng, &mut used_names),
            capital: MunicipalityId(0),
            municipalities: Vec::new(),
        })
        .collect();
    for (k, m) in municipalities.iter_mut().enumerate() {
        m.region = RegionId(region_of[k] as u32);
        regions[region_of[k]].municipalities.push(m.id);
    }
    for r in regions.iter_mut() {
        // Largest member is the capital (municipalities are ordered by rank).
        r.capital = r
            .municipalities
            .iter()
            .copied()
            .min()
            .unwrap_or(MunicipalityId(0));
    }

    // --- 9. Transport ---------------------------------------------------------
    progress("progress-transport", 0.75);
    let mut candidates = Vec::new();
    for a in 0..n_m {
        for &b in &adjacency[a] {
            let b = b as usize;
            if a < b
                && let Some((cost, len, path)) =
                    transport::path(sites[a], sites[b], &cover, &hyd.river, &prof)
            {
                candidates.push(transport::RoadEdge {
                    a,
                    b,
                    cost,
                    length_cells: len,
                    path,
                });
            }
        }
    }
    let roads = transport::select_network(n_m, candidates);
    let mut links = Vec::new();
    for e in &roads {
        let major = |k: usize| {
            matches!(
                municipalities[k].class,
                SettlementClass::Metro | SettlementClass::City
            )
        };
        let mode = if major(e.a) && major(e.b) {
            LinkMode::Highway
        } else {
            LinkMode::Road
        };
        let speed = if mode == LinkMode::Highway {
            90.0
        } else {
            60.0
        };
        let km = e.length_cells * KM_PER_CELL;
        links.push(TransportLink {
            id: LinkId(links.len() as u32),
            a: MunicipalityId(e.a as u32),
            b: MunicipalityId(e.b as u32),
            mode,
            length_km: (km * 10.0).round() / 10.0,
            cost: ((km / speed) * (e.cost / e.length_cells.max(1.0)) * 100.0).round() / 100.0,
            path: e.path.clone(),
        });
    }
    // Rail spines: national capital to every regional capital along roads.
    let refs: Vec<&transport::RoadEdge> = roads.iter().collect();
    for r in &regions {
        let target = r.capital.index();
        if target == 0 {
            continue;
        }
        let (_, route) = transport::graph_dist(n_m, &refs, 0, target);
        if route.is_empty() {
            continue;
        }
        let mut path: Vec<u32> = Vec::new();
        let mut km = 0.0;
        for k in route {
            km += roads[k].length_cells * KM_PER_CELL;
            path.extend(roads[k].path.iter().copied());
        }
        links.push(TransportLink {
            id: LinkId(links.len() as u32),
            a: MunicipalityId(0),
            b: r.capital,
            mode: LinkMode::Rail,
            length_km: (km * 10.0).round() / 10.0,
            cost: ((km / 120.0) * 100.0).round() / 100.0,
            path,
        });
    }

    // --- 10. Neighbourhoods ---------------------------------------------------
    progress("progress-neighborhoods", 0.82);
    let mut neighborhoods: Vec<Neighborhood> = Vec::new();
    let mut hood_population: Vec<u64> = Vec::new();
    let mut industrial = vec![false; n_m];
    for k in 0..n_m {
        let m = &municipalities[k];
        industrial[k] = m.geo.minerals > 0.4
            || m.geo.energy > 0.5
            || (matches!(m.class, SettlementClass::City | SettlementClass::Metro)
                && rng.chance(0.6))
            || (preset == Some(Preset::IndustrialDecline) && m.class != SettlementClass::Rural);
        let plan = hoods::plan(rng, pops[k], m.class, m.coastal, industrial[k]);
        let first = neighborhoods.len();
        let local = hoods::local_adjacency(plan.len());
        let mut hood_names = BTreeSet::new();
        for (j, hp) in plan.iter().enumerate() {
            let name = if j == 0 {
                municipalities[k].name.clone()
            } else {
                names::unique(hood_pool, rng, &mut hood_names)
            };
            hood_population.push(hp.population);
            neighborhoods.push(Neighborhood {
                id: NeighborhoodId(neighborhoods.len() as u32),
                municipality: MunicipalityId(k as u32),
                name,
                kind: hp.kind,
                neighbors: local[j]
                    .iter()
                    .map(|&x| NeighborhoodId((first + x) as u32))
                    .collect(),
            });
        }
        municipalities[k].neighborhoods = (first..neighborhoods.len())
            .map(|i| NeighborhoodId(i as u32))
            .collect();
    }
    // Cross-municipality links between "edge" neighbourhoods (last of each).
    for a in 0..n_m {
        for &b in &adjacency[a] {
            let b = b as usize;
            if a < b {
                let ha = *municipalities[a]
                    .neighborhoods
                    .last()
                    .unwrap_or(&NeighborhoodId(0));
                let hb = *municipalities[b]
                    .neighborhoods
                    .last()
                    .unwrap_or(&NeighborhoodId(0));
                neighborhoods[ha.index()].neighbors.push(hb);
                neighborhoods[hb.index()].neighbors.push(ha);
            }
        }
    }
    for n in neighborhoods.iter_mut() {
        n.neighbors.sort_unstable();
        n.neighbors.dedup();
    }

    // --- 11. Constituencies ----------------------------------------------------
    progress("progress-districts", 0.9);
    let hood_adj: Vec<Vec<usize>> = neighborhoods
        .iter()
        .map(|n| n.neighbors.iter().map(|x| x.index()).collect())
        .collect();
    let hood_pop: Vec<u64> = hood_population.clone();
    let mut constituencies = Vec::new();
    let push_districts =
        |tier: Tier, jurisdiction: u32, units: &[usize], k: usize, out: &mut Vec<Constituency>| {
            let assign = districts::partition(
                &districts::PartitionInput {
                    units,
                    adjacency: &hood_adj,
                    population: &hood_pop,
                },
                k,
            );
            let kk = assign.iter().map(|a| a + 1).max().unwrap_or(0);
            for d in 0..kk {
                out.push(Constituency {
                    id: ConstituencyId(out.len() as u32),
                    tier,
                    jurisdiction,
                    number: d as u32 + 1,
                    neighborhoods: units
                        .iter()
                        .zip(&assign)
                        .filter(|(_, a)| **a == d)
                        .map(|(u, _)| NeighborhoodId(*u as u32))
                        .collect(),
                });
            }
        };
    for m in &municipalities {
        let units: Vec<usize> = m.neighborhoods.iter().map(|n| n.index()).collect();
        let target = match m.class {
            SettlementClass::Metro => 15,
            SettlementClass::City => 11,
            SettlementClass::Town => 7,
            SettlementClass::Rural => 5,
        };
        push_districts(
            Tier::Local,
            m.id.0,
            &units,
            target.min(units.len()),
            &mut constituencies,
        );
    }
    for r in &regions {
        let units: Vec<usize> = r
            .municipalities
            .iter()
            .flat_map(|m| {
                municipalities[m.index()]
                    .neighborhoods
                    .iter()
                    .map(|n| n.index())
            })
            .collect();
        let k = (units.len() / 4).clamp(3, 12).min(units.len());
        push_districts(Tier::Regional, r.id.0, &units, k, &mut constituencies);
    }
    let all: Vec<usize> = (0..neighborhoods.len()).collect();
    let k = d.national_districts.min(all.len() / 3).max(1);
    push_districts(Tier::National, 0, &all, k, &mut constituencies);

    // --- 12. Places ------------------------------------------------------------
    let given: Vec<String> = given_pool.prefixes.clone();
    let mut places = Vec::new();
    for k in 0..n_m {
        let m = &municipalities[k];
        let regional_capital = regions[m.region.index()].capital == m.id;
        let plans = places::plan(
            rng,
            &places::Ctx {
                muni_name: &m.name,
                class: m.class,
                geo: &m.geo,
                port: m.port,
                industrial: industrial[k],
                regional_capital,
                national_capital: k == 0,
                given_names: &given,
            },
        );
        let mut ids = Vec::new();
        for p in plans {
            let hood = m
                .neighborhoods
                .iter()
                .copied()
                .find(|h| neighborhoods[h.index()].kind == p.area)
                .unwrap_or(m.neighborhoods[0]);
            let id = PlaceId(places.len() as u32);
            places.push(Place {
                id,
                kind: p.kind,
                municipality: m.id,
                neighborhood: hood,
                name: p.name,
            });
            ids.push(id);
        }
        municipalities[k].places = ids;
    }

    // --- Derived terrain -----------------------------------------------------
    progress("progress-finishing", 0.97);
    let terrain = TerrainData {
        w: d.w,
        h: d.h,
        elevation: elev.data.iter().map(|e| *e as f32).collect(),
        biome: biome.data.clone(),
        cover: cover.data.clone(),
        river: hyd.river.data.clone(),
        owner: owner.data.clone(),
        temperature: clim.temperature.data.iter().map(|v| *v as f32).collect(),
        precipitation: clim.precipitation.data.iter().map(|v| *v as f32).collect(),
    };
    let world = World {
        geo: Some(WorldGeo {
            width: d.w as u32,
            height: d.h as u32,
            km_per_cell: KM_PER_CELL,
            terrain_hash: terrain_hash(&terrain),
            wind_from_deg: if wind_from_west { 0.0 } else { 180.0 },
        }),
        capital: Some(MunicipalityId(0)),
        regions,
        municipalities,
        neighborhoods,
        constituencies,
        links,
        places,
    };
    Ok(Generated {
        world,
        terrain,
        hood_population,
    })
}

fn aggregate(
    cells: &[usize],
    elev: &Grid<f64>,
    clim: &climate::Climate,
    prof: &profile::CellProfile,
    biome: &Grid<Biome>,
) -> GeoProfile {
    let n = cells.len().max(1) as f64;
    let mean = |g: &Grid<f64>| cells.iter().map(|&i| g.data[i]).sum::<f64>() / n;
    let peak = |g: &Grid<f64>| {
        let mut v: Vec<f64> = cells.iter().map(|&i| g.data[i]).collect();
        v.sort_by(|a, b| b.total_cmp(a));
        let top = (v.len() / 5).max(1);
        v.iter().take(top).sum::<f64>() / top as f64
    };
    let mut counts: BTreeMap<Biome, usize> = BTreeMap::new();
    for &i in cells {
        *counts.entry(biome.data[i]).or_default() += 1;
    }
    let dominant_biome = counts
        .iter()
        .max_by_key(|(b, c)| (**c, std::cmp::Reverse(**b)))
        .map(|(b, _)| *b);
    let r3 = |v: f64| (v * 1000.0).round() / 1000.0;
    GeoProfile {
        elevation_m: mean(elev).round(),
        ruggedness: r3(mean(&prof.slope) * 2.0).min(1.0),
        temperature_c: (mean(&clim.temperature) * 10.0).round() / 10.0,
        precipitation_mm: mean(&clim.precipitation).round(),
        forest_share: r3(cells.iter().filter(|&&i| prof.timber.data[i] > 0.5).count() as f64 / n),
        fertility: r3(mean(&prof.fertility)),
        timber: r3(mean(&prof.timber)),
        minerals: r3(0.5 * mean(&prof.minerals) + 0.5 * peak(&prof.minerals)),
        fishing: r3(peak(&prof.fishing)),
        energy: r3(0.5 * mean(&prof.energy) + 0.5 * peak(&prof.energy)),
        scenic: r3(0.5 * mean(&prof.scenic) + 0.5 * peak(&prof.scenic)),
        flood: r3(peak(&prof.flood)),
        drought: r3(mean(&prof.drought)),
        wildfire: r3(mean(&prof.wildfire)),
        seismic: r3(mean(&prof.seismic)),
        storm: r3(peak(&prof.storm)),
        dominant_biome,
    }
}

/// Regions grown from spread-out seeds over the municipality graph, with
/// extra cost for crossing major rivers and high ground (natural borders).
fn assign_regions(
    ms: &[Municipality],
    pops: &[u64],
    k: usize,
    elev: &Grid<f64>,
    river: &Grid<u8>,
) -> Vec<usize> {
    let n = ms.len();
    let k = k.min(n).max(1);
    let center = |m: &Municipality| elev.idx(m.center.0 as usize, m.center.1 as usize);
    let mut seeds = vec![0usize];
    while seeds.len() < k {
        let next = (0..n).filter(|i| !seeds.contains(i)).max_by(|&a, &b| {
            let score = |i: usize| {
                let dmin = seeds
                    .iter()
                    .map(|&s| cell_dist(elev, center(&ms[i]), center(&ms[s])))
                    .fold(f64::MAX, f64::min);
                dmin * mandate_core::math::pow(pops[i] as f64, 0.15)
            };
            score(a).total_cmp(&score(b)).then(b.cmp(&a))
        });
        match next {
            Some(i) => seeds.push(i),
            None => break,
        }
    }
    let edge_cost = |a: usize, b: usize| {
        let (ca, cb) = (center(&ms[a]), center(&ms[b]));
        let base = cell_dist(elev, ca, cb);
        let (ax, ay) = elev.xy(ca);
        let (bx, by) = elev.xy(cb);
        let steps = base.ceil().max(1.0) as usize;
        let mut max_e: f64 = 0.0;
        let mut crosses = 0.0;
        for s in 0..=steps {
            let t = s as f64 / steps as f64;
            let x = (ax as f64 + (bx as f64 - ax as f64) * t).round() as usize;
            let y = (ay as f64 + (by as f64 - ay as f64) * t).round() as usize;
            let i = elev.idx(x.min(elev.w - 1), y.min(elev.h - 1));
            max_e = max_e.max(elev.data[i]);
            if river.data[i] >= 3 {
                crosses = 1.0;
            }
        }
        base * (1.0 + 1.5 * crosses + max_e / 900.0)
    };
    let mut region = vec![usize::MAX; n];
    let mut best = vec![f64::MAX; n];
    let mut heap = BinaryHeap::new();
    for (r, &s) in seeds.iter().enumerate() {
        region[s] = r;
        best[s] = 0.0;
        heap.push(HeapItem { cost: 0.0, idx: s });
    }
    while let Some(HeapItem { cost, idx }) = heap.pop() {
        if cost > best[idx] {
            continue;
        }
        for nb in &ms[idx].neighbors {
            let j = nb.index();
            let c = cost + edge_cost(idx, j);
            if c < best[j] {
                best[j] = c;
                region[j] = region[idx];
                heap.push(HeapItem { cost: c, idx: j });
            }
        }
    }
    // Unreachable municipalities (should not happen) join region 0.
    region
        .iter()
        .map(|r| if *r == usize::MAX { 0 } else { *r })
        .collect()
}

pub fn terrain_hash(t: &TerrainData) -> String {
    let mut bytes = Vec::with_capacity(t.elevation.len() * 7);
    for i in 0..t.elevation.len() {
        bytes.extend_from_slice(&t.elevation[i].to_le_bytes());
        bytes.push(t.biome[i] as u8);
        bytes.push(t.cover[i] as u8);
        bytes.push(t.river[i]);
    }
    for o in &t.owner {
        bytes.extend_from_slice(&o.to_le_bytes());
    }
    StableHash::of_bytes(&bytes).hex()
}

/// Generate a new world and return an engine ready for play.
pub fn generate(config: &WorldConfig, content: Arc<ContentBundle>) -> Result<Engine, EngineError> {
    generate_with_progress(config, content, &mut |_, _| {})
}

pub fn generate_with_progress(
    config: &WorldConfig,
    content: Arc<ContentBundle>,
    progress: Progress<'_>,
) -> Result<Engine, EngineError> {
    // Geography takes the first ~40% of the progress bar, history the rest.
    let Generated {
        world,
        terrain,
        hood_population,
    } = generate_world(config, &content, &mut |k, f| progress(k, f * 0.4))?;
    let start_date = SimDate::from_ymd(content.tuning.start_year, 1, 1);
    let meta = WorldMeta {
        seed: config.seed,
        params: WorldParams {
            size: config.size,
            preset: config.preset,
        },
        content_hash: content.content_hash.clone(),
        engine_version: ENGINE_VERSION.to_string(),
        start_date,
    };
    let mut rng = RngStreams::new(config.seed);
    let population = mandate_sim::population::init(
        &world,
        &hood_population,
        config.seed,
        config.preset == Some(Preset::WealthyCoast),
    );
    let economy = mandate_sim::economy::init(&world, &population, &content, &meta);
    let finance = mandate_sim::finance::init(&world, &population, &economy, &content.economy);
    let environment = mandate_sim::environment::init(&world, &meta, rng.get(Stream::Environment));
    let mut events = EventLog::default();
    let began = events.record(
        start_date,
        mandate_sim::events::EventKind::HistoryBegan {
            year: content.tuning.start_year,
        },
        vec![],
    );
    events.pin(began);
    let state = State {
        meta,
        calendar: Calendar {
            date: start_date,
            turn: 0,
            phase: GamePhase::Governing,
            act: 1,
            chapter: 1,
        },
        rng,
        world,
        events,
        population,
        economy,
        finance,
        environment,
    };
    let years = content.tuning.burn_in_years as u32;
    let mut engine = Engine::from_state(state, content)?;
    engine.set_terrain(Arc::new(terrain));
    progress("progress-history", 0.4);
    engine.burn_in(years, &mut |y| {
        progress(
            "progress-history",
            0.4 + 0.6 * y as f64 / years.max(1) as f64,
        )
    });
    engine.mark_started();
    progress("progress-done", 1.0);
    Ok(engine)
}

/// Regenerate the derived terrain for a loaded state. Returns the terrain and
/// whether it matches the hash recorded at generation (it will not if the
/// generator changed between versions; simulation data is unaffected).
pub fn rebuild_terrain(
    state: &State,
    content: &ContentBundle,
) -> Result<(TerrainData, bool), EngineError> {
    let cfg = WorldConfig {
        seed: state.meta.seed,
        size: state.meta.params.size,
        preset: state.meta.params.preset,
    };
    let terrain = generate_world(&cfg, content, &mut |_, _| {})?.terrain;
    let ok = state
        .world
        .geo
        .as_ref()
        .map(|g| g.terrain_hash == terrain_hash(&terrain))
        .unwrap_or(false);
    Ok((terrain, ok))
}
