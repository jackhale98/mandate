//! Voter concerns and issue salience (DEC-078 decision 3).
//!
//! Each month every town reads its indicators (unemployment, rent burden,
//! pollution, …), turns each concern in the content registry into a level,
//! and derives a salience per attitude: how much its voters weigh that issue
//! in the vote model. Salience is scaled to a mean of 1, so a town with no
//! pressing concern weighs every issue the same and the overall issue weight
//! keeps its meaning. Levels and salience are derived state: content that
//! adds or removes a concern simply recomputes them.
//!
//! Each neighbourhood feels the town's indicators through its own conditions
//! (`opinion.local_concerns`): its residents' income classes and the
//! sectors they work in, its rents, the industry and the floodplain around
//! it, and how hard the latest disaster hit it. A neighbourhood's indicator
//! is the town's × a relative factor whose residents' mean over the town is
//! 1, so the riverside district worries about the river, the estate by the
//! plant about jobs, and the town's own figure is unchanged. A town's levels
//! are its neighbourhoods' levels weighted by residents.
//!
//! Politics reads the levels measured from the town-wide indicators
//! (`TownConcerns::measured`), per constituency (DEC-092, N2): a ward's (or
//! any tier's constituency's) political levels are each town's measured
//! levels shifted by how much more or less its neighbourhoods feel each worry
//! than the town as a whole (`political_levels`), and its issue salience comes
//! from them by the town's formula (`salience_in`). The vote model weighs a
//! race's voters by their constituency's salience and a ward member's agenda
//! follows the ward's worries. A whole town (a mayoral race, a town-level
//! agenda) gives exactly the town's measured levels and salience.
//!
//! The population domain owns the result (`PopulationState::town_concerns`,
//! `hood_concerns`); the indicators are read-only views of other domains,
//! reusable by later systems (town character, situational policies).

use crate::population::{CLASSES, PARTICIPATION, PopulationState, age_of, class_of};
use crate::scheduler::{Outs, Reads};
use crate::world::{NeighborhoodId, NeighborhoodKind, World};
use mandate_content::{ContentBundle, Indicator, NeighborhoodLean, SectorDriver};
use mandate_core::math::pow;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One town's concerns, per concern in registry order: its level as its
/// residents feel it (its neighbourhoods' levels weighted by residents) and
/// as measured from the town-wide indicators; and a salience per attitude
/// (registry order, mean 1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TownConcerns {
    /// As felt: what the town's views, talk and town character show.
    pub levels: Vec<f32>,
    /// As measured from the town-wide indicators: what drives the town's
    /// politics (the salience below and NPC agendas; a constituency's shift
    /// it by their own neighbourhoods', `political_levels`).
    pub measured: Vec<f32>,
    /// The vote model's issue weights for the whole town, from the measured
    /// levels (a ward's: `salience_in`).
    pub salience: Vec<f32>,
}

/// One neighbourhood's concern levels (registry order), from its own
/// conditions.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HoodConcerns {
    pub levels: Vec<f32>,
}

/// An indicator's current value for a municipality (0 where it cannot be
/// measured). Pure: reads the economy, environment, finance, and policy.
pub fn indicator(r: &Reads<'_>, m: usize, which: Indicator) -> f64 {
    let econ = r.economy();
    let Some(e) = econ.munis.get(m) else {
        return 0.0;
    };
    let typical_income = e.class_income[1].max(1.0);
    match which {
        Indicator::Unemployment => e.unemployment,
        Indicator::UnemploymentRise => match (
            e.history.last(),
            e.history.get(e.history.len().saturating_sub(13)),
        ) {
            (Some(now), Some(then)) => (now.unemployment - then.unemployment) as f64,
            _ => 0.0,
        },
        Indicator::RentBurden => e.housing.rent * 12.0 / typical_income,
        Indicator::Pollution => r
            .world
            .municipalities
            .get(m)
            .and_then(|mu| r.environment().regions.get(mu.region.index()))
            .map(|reg| reg.pollution)
            .unwrap_or(0.0),
        Indicator::TaxBurden => {
            let fin = r.finance();
            let rate = fin
                .taxes
                .property
                .get(m)
                .copied()
                .unwrap_or(r.content.economy.taxes.property);
            let mods = crate::policy::mods_for(r.policy(), r.content, m as u32, &|id| {
                fin.funded_share(id)
            });
            e.housing.price * (rate + mods.property_tax_delta).max(0.0) / typical_income
        }
        Indicator::BudgetStress => r
            .finance()
            .local
            .get(m)
            .map(|t| t.debt.units() as f64 / (t.budget_total().units() as f64).max(1.0))
            .unwrap_or(0.0),
        Indicator::RecentDisaster => (1.0 - e.months_since_disaster as f64 / 24.0).max(0.0),
        Indicator::JobLoss => match (
            e.history.last(),
            e.history.get(e.history.len().saturating_sub(13)),
        ) {
            (Some(now), Some(then)) if then.jobs > 0.0 => {
                ((then.jobs - now.jobs) as f64 / then.jobs as f64).max(0.0)
            }
            _ => 0.0,
        },
    }
}

const INDICATORS: usize = Indicator::ALL.len();
const DRIVERS: usize = SectorDriver::ALL.len();

/// Position of an indicator in `Indicator::ALL`.
fn slot(which: Indicator) -> usize {
    Indicator::ALL
        .iter()
        .position(|x| *x == which)
        .unwrap_or_default()
}

/// A neighbourhood kind's value in a per-kind table.
fn by_kind(l: &NeighborhoodLean, kind: NeighborhoodKind) -> f64 {
    match kind {
        NeighborhoodKind::Core => l.core,
        NeighborhoodKind::Inner => l.inner,
        NeighborhoodKind::Waterfront => l.waterfront,
        NeighborhoodKind::Industrial => l.industrial,
        NeighborhoodKind::Suburb => l.suburb,
        NeighborhoodKind::Rural => l.rural,
    }
}

/// Open workplaces per neighbourhood and sector driver (`SectorDriver::ALL`
/// order), from `opinion.local_concerns.workplaces`.
fn workplaces(world: &World, content: &ContentBundle) -> Vec<[u16; DRIVERS]> {
    let mut out = vec![[0u16; DRIVERS]; world.neighborhoods.len()];
    let map = &content.opinion.local_concerns.workplaces;
    if map.is_empty() {
        return out;
    }
    for p in world.places.iter().filter(|p| p.is_open()) {
        let Some(d) = map.get(p.kind.key()) else {
            continue;
        };
        let di = driver_slot(*d);
        if let Some(c) = out.get_mut(p.neighborhood.index()) {
            c[di] = c[di].saturating_add(1);
        }
    }
    out
}

fn driver_slot(d: SectorDriver) -> usize {
    SectorDriver::ALL
        .iter()
        .position(|x| *x == d)
        .unwrap_or_default()
}

fn polluting(d: SectorDriver) -> bool {
    matches!(d, SectorDriver::Industrial | SectorDriver::Extraction)
}

/// A town's sector, as its neighbourhoods' workers feel it.
struct Sector {
    driver: SectorDriver,
    jobs: f64,
    cyclicality: f64,
    /// Share of its starting jobs lost since play began (0 when it grew).
    decline: f64,
}

/// `raw` scaled so its `w`-weighted mean is 1 (all 1 when it is not
/// positive).
fn relative(raw: &[f64], w: &[f64]) -> Vec<f64> {
    let wsum: f64 = w.iter().sum();
    let mean = if wsum > 0.0 {
        raw.iter().zip(w).map(|(x, w)| x * w).sum::<f64>() / wsum
    } else {
        0.0
    };
    if mean > 0.0 && mean.is_finite() {
        raw.iter().map(|x| x / mean).collect()
    } else {
        vec![1.0; raw.len()]
    }
}

/// Every indicator (`Indicator::ALL` order) for each neighbourhood of a
/// town, in `Municipality::neighborhoods` order, with the residents that
/// weigh it: the town's value × the neighbourhood's relative factor.
fn hood_indicators(
    r: &Reads<'_>,
    pop: &PopulationState,
    m: usize,
    town: &[f64],
    wp: &[[u16; DRIVERS]],
) -> Vec<(NeighborhoodId, f64, [f64; INDICATORS])> {
    let content = r.content;
    let lc = &content.opinion.local_concerns;
    let Some(mu) = r.world.municipalities.get(m) else {
        return Vec::new();
    };
    let hoods = &mu.neighborhoods;
    if hoods.is_empty() {
        return Vec::new();
    }
    let econ = r.economy().munis.get(m);
    // Residents weigh every mean; a town with none weighs its districts evenly.
    let residents: Vec<f64> = hoods
        .iter()
        .map(|h| pop.hood_total(h.index()) as f64)
        .collect();
    let w: Vec<f64> = if residents.iter().sum::<f64>() > 0.0 {
        residents.clone()
    } else {
        vec![1.0; hoods.len()]
    };
    let kind = |h: NeighborhoodId| {
        r.world
            .neighborhoods
            .get(h.index())
            .map_or(NeighborhoodKind::Core, |n| n.kind)
    };
    // Adults and labour force by income class.
    let classes: Vec<([f64; CLASSES], [f64; CLASSES])> = hoods
        .iter()
        .map(|h| {
            let mut adults = [0.0; CLASSES];
            let mut lf = [0.0; CLASSES];
            if let Some(hp) = pop.hoods.get(h.index()) {
                for (ci, c) in hp.cohorts.iter().enumerate() {
                    let n = c.count as f64;
                    adults[class_of(ci)] += n;
                    lf[class_of(ci)] += n * PARTICIPATION[age_of(ci)];
                }
            }
            (adults, lf)
        })
        .collect();
    // Sector shares of each district's workers: the town's jobs, leaning
    // toward the sectors whose workplaces are in or next to it and those its
    // kind of district works in.
    let sectors: Vec<Sector> = match econ {
        Some(e) => content
            .sectors
            .iter()
            .zip(&e.sectors)
            .map(|(d, s)| {
                let decline = if s.baseline_jobs > 0 {
                    (1.0 - s.jobs / s.baseline_jobs as f64).max(0.0)
                } else {
                    0.0
                };
                Sector {
                    driver: d.driver,
                    jobs: s.jobs.max(0.0),
                    cyclicality: d.cyclicality.max(0.0),
                    decline,
                }
            })
            .collect(),
        None => Vec::new(),
    };
    // Workplaces in each district and next door, by driver.
    let count = |h: NeighborhoodId| -> [f64; DRIVERS] {
        wp.get(h.index())
            .map_or([0.0; DRIVERS], |c| c.map(f64::from))
    };
    let near: Vec<([f64; DRIVERS], [f64; DRIVERS])> = hoods
        .iter()
        .map(|&h| {
            let mut next = [0.0; DRIVERS];
            if let Some(n) = r.world.neighborhoods.get(h.index()) {
                for x in &n.neighbors {
                    for (a, b) in next.iter_mut().zip(count(*x)) {
                        *a += b;
                    }
                }
            }
            (count(h), next)
        })
        .collect();
    let leans: Vec<Option<&NeighborhoodLean>> = sectors
        .iter()
        .map(|x| {
            lc.sector_kinds
                .iter()
                .find(|s| s.driver == x.driver)
                .map(|s| &s.kinds)
        })
        .collect();
    // Each district's workers' exposure to sectors' decline since play began
    // and to job losses (cyclical and declining sectors).
    let (decline, losses): (Vec<f64>, Vec<f64>) = hoods
        .iter()
        .zip(&near)
        .map(|(&h, (inside, next))| {
            let k = kind(h);
            let (mut total, mut decline, mut losses) = (0.0, 0.0, 0.0);
            for (x, lean) in sectors.iter().zip(&leans) {
                let di = driver_slot(x.driver);
                let share = x.jobs
                    * (1.0
                        + lean.map_or(0.0, |l| by_kind(l, k))
                        + lc.workplace_affinity * (inside[di] + 0.5 * next[di]));
                total += share;
                decline += share * x.decline;
                losses += share * (x.cyclicality + lc.job_loss_decline * x.decline);
            }
            if total > 0.0 {
                (decline / total, losses / total)
            } else {
                (0.0, 0.0)
            }
        })
        .unzip();
    // Unemployment: the town's employment rate by income class over the
    // district's labour force, plus the jobs its workers' sectors have lost
    // since play began beyond the town's average (the plant's estate).
    let decline_mean = {
        let ws: f64 = w.iter().sum();
        decline.iter().zip(&w).map(|(d, w)| d * w).sum::<f64>() / ws
    };
    let unemployment: Vec<f64> = classes
        .iter()
        .zip(&decline)
        .map(|((_, lf), d)| {
            let total: f64 = lf.iter().sum();
            let u = match econ {
                Some(e) if total > 0.0 => {
                    1.0 - (0..CLASSES)
                        .map(|c| lf[c] * e.class_employment[c])
                        .sum::<f64>()
                        / total
                }
                _ => econ.map_or(0.0, |e| e.unemployment),
            };
            // The jobs its sectors lost beyond the town's share, as a rate.
            (u + lc.jobs_sector_tilt * (d - decline_mean)).max(0.0)
        })
        .collect();
    let unemployment = relative(&unemployment, &w);
    // Job losses: the town's, felt by the workers of cyclical and declining
    // sectors.
    let job_loss = relative(&losses, &w);
    // Rent burden: the district's rent (its kind, and how much faster it
    // has grown than the town) over its residents' incomes.
    let town_growth = {
        let (now, base) = hoods
            .iter()
            .zip(&residents)
            .fold((0.0, 0.0), |(n, b), (h, here)| {
                let base = pop
                    .hoods
                    .get(h.index())
                    .map_or(0, |x| x.baseline_population);
                if base > 0 {
                    (n + here, b + base as f64)
                } else {
                    (n, b)
                }
            });
        if base > 0.0 { now / base } else { 1.0 }
    };
    let rent: Vec<f64> = hoods
        .iter()
        .zip(&classes)
        .zip(&residents)
        .map(|((&h, (adults, _)), here)| {
            let base = pop
                .hoods
                .get(h.index())
                .map_or(0, |x| x.baseline_population);
            let growth = if base > 0 && town_growth > 0.0 {
                (here / base as f64 / town_growth).clamp(0.5, 2.0)
            } else {
                1.0
            };
            let n: f64 = adults.iter().sum();
            let burden = match econ {
                Some(e) if n > 0.0 => (0..CLASSES)
                    .map(|c| adults[c] / n / e.class_income[c].max(1.0))
                    .sum::<f64>(),
                Some(e) => 1.0 / e.class_income[1].max(1.0),
                None => 1.0,
            };
            by_kind(&lc.rent, kind(h)).max(0.0) * pow(growth, lc.rent_growth) * burden
        })
        .collect();
    let rent = relative(&rent, &w);
    // Pollution: the district's kind, polluting workplaces in and next to
    // it, and the floodplain's runoff.
    let env = r.environment();
    let pollution: Vec<f64> = hoods
        .iter()
        .zip(&near)
        .map(|(&h, (here, next))| {
            let (inside, near) = SectorDriver::ALL
                .iter()
                .filter(|d| polluting(**d))
                .map(|d| driver_slot(*d))
                .fold((0.0, 0.0), |(a, b), di| (a + here[di], b + next[di]));
            let flood = env.exposure(h).map_or(0.0, |x| f64::from(x.flood) / 1000.0);
            by_kind(&lc.pollution, kind(h)).max(0.0)
                + lc.pollution_workplace * inside
                + lc.pollution_next_door * near
                + lc.pollution_floodplain * flood
        })
        .collect();
    let pollution = relative(&pollution, &w);
    hoods
        .iter()
        .enumerate()
        .map(|(i, &h)| {
            let mut v = [0.0; INDICATORS];
            for (j, which) in Indicator::ALL.iter().enumerate() {
                let t = town.get(j).copied().unwrap_or(0.0);
                v[j] = match which {
                    Indicator::Unemployment | Indicator::UnemploymentRise => t * unemployment[i],
                    Indicator::RentBurden => t * rent[i],
                    Indicator::Pollution => t * pollution[i],
                    Indicator::JobLoss => t * job_loss[i],
                    Indicator::TaxBurden | Indicator::BudgetStress | Indicator::RecentDisaster => t,
                };
            }
            (h, w[i], v)
        })
        .collect()
}

/// One town's concerns and its neighbourhoods' levels (in
/// `Municipality::neighborhoods` order), from the indicators.
fn compute_town(
    r: &Reads<'_>,
    pop: &PopulationState,
    m: usize,
    wp: &[[u16; DRIVERS]],
) -> (TownConcerns, Vec<(NeighborhoodId, HoodConcerns)>) {
    let content = r.content;
    let town: Vec<f64> = Indicator::ALL.iter().map(|i| indicator(r, m, *i)).collect();
    let town_levels: Vec<f64> = content
        .concerns
        .iter()
        .map(|c| c.level.of(town[slot(c.indicator)]))
        .collect();
    let mut sum = vec![0.0; content.concerns.len()];
    let mut wsum = 0.0;
    let hoods: Vec<(NeighborhoodId, HoodConcerns)> = hood_indicators(r, pop, m, &town, wp)
        .into_iter()
        .map(|(h, w, v)| {
            let levels: Vec<f64> = content
                .concerns
                .iter()
                .zip(&town_levels)
                .map(|(c, tl)| {
                    if c.indicator == Indicator::RecentDisaster {
                        // Felt by how hard the disaster hit here (DEC-090 D8).
                        (tl * pop.impact(h.index())).min(1.0)
                    } else {
                        c.level.of(v[slot(c.indicator)])
                    }
                })
                .collect();
            for (s, l) in sum.iter_mut().zip(&levels) {
                *s += w * l;
            }
            wsum += w;
            (
                h,
                HoodConcerns {
                    levels: levels.iter().map(|l| *l as f32).collect(),
                },
            )
        })
        .collect();
    let levels: Vec<f64> = if wsum > 0.0 {
        sum.iter().map(|s| s / wsum).collect()
    } else {
        town_levels.clone()
    };
    (
        TownConcerns {
            salience: salience_from(content, &town_levels),
            levels: levels.iter().map(|l| *l as f32).collect(),
            measured: town_levels.iter().map(|l| *l as f32).collect(),
        },
        hoods,
    )
}

/// Levels and salience for one town from its indicators.
pub fn compute(r: &Reads<'_>, pop: &PopulationState, m: usize) -> TownConcerns {
    compute_town(r, pop, m, &workplaces(r.world, r.content)).0
}

/// Salience per attitude: `salience_base` + Σ level × |weight|, scaled to a
/// mean of 1.
pub fn salience_from(content: &ContentBundle, levels: &[f64]) -> Vec<f32> {
    let base = content.politics.vote.salience_base;
    let mut raw = vec![base; content.attitudes.len()];
    for (c, level) in content.concerns.iter().zip(levels) {
        for (a, w) in raw.iter_mut().zip(&c.attitude_weights) {
            *a += level * w.abs();
        }
    }
    let mean = raw.iter().sum::<f64>() / raw.len().max(1) as f64;
    raw.iter()
        .map(|v| if mean > 0.0 { (v / mean) as f32 } else { 1.0 })
        .collect()
}

/// Every town's concerns and every neighbourhood's levels (world order),
/// from the current state of the other domains. `pop` is the population
/// being written (the scheduler does not lend it through `r`).
pub fn compute_all(r: &Reads<'_>, pop: &PopulationState) -> (Vec<TownConcerns>, Vec<HoodConcerns>) {
    let wp = workplaces(r.world, r.content);
    let mut hoods = vec![HoodConcerns::default(); r.world.neighborhoods.len()];
    let towns = (0..r.world.municipalities.len())
        .map(|m| {
            let (t, hs) = compute_town(r, pop, m, &wp);
            for (h, c) in hs {
                if let Some(x) = hoods.get_mut(h.index()) {
                    *x = c;
                }
            }
            t
        })
        .collect();
    (towns, hoods)
}

/// Whether the stored concerns fit the world and the registry (else the
/// engine recomputes them on load).
pub fn hoods_current(pop: &PopulationState, world: &World) -> bool {
    pop.hood_concerns.len() == world.neighborhoods.len()
        && pop
            .hood_concerns
            .iter()
            .all(|h| h.levels.len() == pop.concern_keys.len())
}

/// Population domain, Opinion phase, monthly: recompute every town's and
/// neighbourhood's concern levels and the towns' issue salience.
pub fn refresh(pop: &mut PopulationState, r: &Reads<'_>, _out: &mut Outs<'_>) {
    let (towns, hoods) = compute_all(r, pop);
    pop.town_concerns = towns;
    pop.hood_concerns = hoods;
    pop.concern_keys = r.content.concern_keys().cloned().collect();
}

/// A town's issue salience, if it has been computed for this attitude
/// registry (otherwise every issue weighs the same).
pub fn salience(pop: &PopulationState, m: usize) -> Option<&[f32]> {
    pop.town_concerns
        .get(m)
        .map(|t| t.salience.as_slice())
        .filter(|s| s.len() == pop.attitude_count())
}

/// Whether `hoods` are exactly the neighbourhoods of town `m` (in any order).
fn covers_town(world: &World, m: usize, hoods: &[usize]) -> bool {
    let Some(mu) = world.municipalities.get(m) else {
        return false;
    };
    let mut all: Vec<usize> = mu.neighborhoods.iter().map(|h| h.index()).collect();
    let mut here = hoods.to_vec();
    all.sort_unstable();
    here.sort_unstable();
    here.dedup();
    all == here
}

/// The neighbourhoods given, grouped by their town (town order, then the
/// order given).
fn by_town(world: &World, hoods: &[usize]) -> BTreeMap<usize, Vec<usize>> {
    let mut out: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &h in hoods {
        if let Some(n) = world.neighborhoods.get(h) {
            out.entry(n.municipality.index()).or_default().push(h);
        }
    }
    out
}

/// One town's political levels for some of its neighbourhoods: the town's
/// measured levels, shifted by how much more (or less) they feel each worry
/// than the town (`levels_in` − the town's felt level), within the
/// concern's level range. All of a town gives its measured levels exactly.
/// None while the town's concerns are not computed for this registry.
fn town_political_levels(
    pop: &PopulationState,
    content: &ContentBundle,
    world: &World,
    m: usize,
    hoods: &[usize],
) -> Option<Vec<f64>> {
    let n = content.concerns.len();
    let tc = pop.town_concerns.get(m)?;
    if tc.measured.len() != n {
        return None;
    }
    let measured = tc.measured.iter().map(|l| *l as f64);
    if covers_town(world, m, hoods) || tc.levels.len() != n {
        return Some(measured.collect());
    }
    let here = levels_in(pop, content, m, hoods);
    Some(
        measured
            .zip(&here)
            .zip(&tc.levels)
            .zip(&content.concerns)
            .map(|(((m, h), t), c)| {
                let (lo, hi) = (c.level.min, c.level.max.max(c.level.min));
                (m + h - *t as f64).clamp(lo, hi)
            })
            .collect(),
    )
}

/// The concern levels that drive the politics of a constituency of any tier
/// (registry order), from its neighbourhoods (world indices): per town,
/// its measured levels shifted by its listed neighbourhoods' own worries
/// (`town_political_levels`), then the towns weighted by those
/// neighbourhoods' residents. Whole towns give their measured levels, so a
/// mayoral race or a town-level agenda reads the town's figure. Empty when
/// no neighbourhood is given or a town's concerns are not computed yet.
pub fn political_levels(
    pop: &PopulationState,
    content: &ContentBundle,
    world: &World,
    hoods: &[usize],
) -> Vec<f64> {
    let towns = by_town(world, hoods);
    let mut parts = Vec::with_capacity(towns.len());
    for (m, hs) in &towns {
        let Some(levels) = town_political_levels(pop, content, world, *m, hs) else {
            return Vec::new();
        };
        let residents: f64 = hs.iter().map(|h| pop.hood_total(*h) as f64).sum();
        parts.push((residents, levels));
    }
    if parts.len() == 1 {
        return parts.pop().map(|(_, l)| l).unwrap_or_default();
    }
    let even = parts.iter().all(|(w, _)| *w <= 0.0);
    let mut sum = vec![0.0; content.concerns.len()];
    let mut wsum = 0.0;
    for (w, levels) in &parts {
        let w = if even { 1.0 } else { *w };
        for (s, l) in sum.iter_mut().zip(levels) {
            *s += w * l;
        }
        wsum += w;
    }
    if wsum > 0.0 {
        sum.iter().map(|s| s / wsum).collect()
    } else {
        sum
    }
}

/// Issue salience for a constituency of any tier (its neighbourhoods, world
/// indices): `salience_from` its political levels (`political_levels`).
/// A whole town gives the town's own salience (`salience`), bit for bit.
/// None when no neighbourhood is given or a town touched has no salience
/// for this attitude registry (every issue then weighs the same).
pub fn salience_in(
    pop: &PopulationState,
    content: &ContentBundle,
    world: &World,
    hoods: &[usize],
) -> Option<Vec<f32>> {
    let towns = by_town(world, hoods);
    if towns.is_empty() || towns.keys().any(|m| salience(pop, *m).is_none()) {
        return None;
    }
    if let Some((m, hs)) = towns.iter().next()
        && towns.len() == 1
        && covers_town(world, *m, hs)
    {
        return salience(pop, *m).map(<[f32]>::to_vec);
    }
    let levels = political_levels(pop, content, world, hoods);
    if levels.len() != content.concerns.len() {
        return None;
    }
    Some(salience_from(content, &levels)).filter(|s| s.len() == pop.attitude_count())
}

/// A town's concerns, strongest first: (registry index, level), the levels
/// of its neighbourhoods weighted by residents. Ties keep registry order.
pub fn top_concerns(pop: &PopulationState, m: usize) -> Vec<(usize, f64)> {
    let mut v: Vec<(usize, f64)> = pop
        .town_concerns
        .get(m)
        .map(|t| t.levels.iter().map(|l| *l as f64).enumerate().collect())
        .unwrap_or_default();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// Concern levels in some neighbourhoods of a town (registry order): their
/// own levels weighted by residents, or the town's when none are given. A
/// neighbourhood whose levels are not computed yet falls back on the town's,
/// a recent disaster's felt by its impact.
pub fn levels_in(
    pop: &PopulationState,
    content: &ContentBundle,
    m: usize,
    hoods: &[usize],
) -> Vec<f64> {
    let town: Vec<f64> = pop
        .town_concerns
        .get(m)
        .map(|t| t.levels.iter().map(|l| *l as f64).collect())
        .unwrap_or_default();
    if hoods.is_empty() {
        return town;
    }
    let mut weights: Vec<f64> = hoods.iter().map(|&h| pop.hood_total(h) as f64).collect();
    if weights.iter().all(|w| *w <= 0.0) {
        weights = vec![1.0; hoods.len()];
    }
    let mut sum = vec![0.0; town.len()];
    let mut wsum = 0.0;
    for (&h, &w) in hoods.iter().zip(&weights) {
        let own = pop
            .hood_concerns
            .get(h)
            .map(|x| x.levels.as_slice())
            .filter(|l| l.len() == town.len());
        for (i, s) in sum.iter_mut().enumerate() {
            let l = match own {
                Some(l) => l[i] as f64,
                None => {
                    let tl = town[i];
                    match content.concerns.get(i) {
                        Some(c) if c.indicator == Indicator::RecentDisaster => {
                            (tl * pop.impact(h)).min(1.0)
                        }
                        _ => tl,
                    }
                }
            };
            *s += w * l;
        }
        wsum += w;
    }
    sum.iter().map(|s| s / wsum).collect()
}

/// Concerns as felt in some neighbourhoods of a town, strongest first:
/// their own levels (`levels_in`) × (1 + ½ × how far residents lean the way
/// worried voters do), in [0, 1.5 × level]. For display, NPC talk and the
/// canvass match, not the vote model.
pub fn felt_concerns(
    pop: &PopulationState,
    content: &ContentBundle,
    m: usize,
    hoods: &[usize],
) -> Vec<(usize, f64)> {
    let mean = crate::politics::mean_attitudes(pop, hoods);
    let mut v: Vec<(usize, f64)> = levels_in(pop, content, m, hoods)
        .into_iter()
        .enumerate()
        .filter_map(|(i, level)| {
            let c = content.concerns.get(i)?;
            let norm: f64 = c.attitude_weights.iter().map(|w| w.abs()).sum();
            let lean = if norm > 0.0 {
                c.attitude_weights
                    .iter()
                    .zip(&mean)
                    .map(|(w, x)| w * *x as f64)
                    .sum::<f64>()
                    / norm
            } else {
                0.0
            };
            Some((i, level * (1.0 + 0.5 * lean.clamp(-1.0, 1.0))))
        })
        .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}
