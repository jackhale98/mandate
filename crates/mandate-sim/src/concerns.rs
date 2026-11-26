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
//! "What does this place worry about most" has one answer, from one function:
//! `ranked` gives the ranked worries of a scope (a jurisdiction of any tier,
//! a district, a neighbourhood) by a named reading, `felt` (what people say
//! and the player is told) or `measured` (by level alone), with one
//! tie-break (equal weights keep registry order). Views, talk, conditions,
//! pickers and bots read it; none ranks worries itself.
//!
//! The population domain owns the result (`PopulationState::town_concerns`,
//! `hood_concerns`); the indicators are read-only views of other domains,
//! reusable by later systems (town character, situational policies).

use crate::population::{AGE_BANDS, CLASSES, PARTICIPATION, PopulationState, age_of, class_of};
use crate::scheduler::{Outs, Reads};
use crate::world::{Jurisdiction, MunicipalityId, NeighborhoodId, NeighborhoodKind, World};
use mandate_content::condition::Worry;
use mandate_content::{BudgetCategory, ContentBundle, Indicator, NeighborhoodLean, SectorDriver};
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
    indicator_with(r, r.population_opt(), m, which)
}

/// The town's discretionary budget against the level its budget rule aims
/// for (its starting budget indexed to prices × productivity and to the
/// nation's population, `finance::new_budget_year`): 1 at need, below
/// after cuts or when programmes and interest crowd out the baseline.
pub fn budget_index(r: &Reads<'_>, pop: Option<&PopulationState>, m: usize) -> f64 {
    let econ = r.economy();
    let fin = r.finance();
    let Some(t) = fin.local.get(m) else {
        return 1.0;
    };
    let wage_index = (econ.national.price_level * econ.national.productivity_index).max(0.1);
    let pop_index = pop
        .map(|p| (p.total() as f64 / fin.base_population.max(1.0)).max(0.1))
        .unwrap_or(1.0);
    let need = t.base_budget_total.units() as f64 * wage_index * pop_index;
    t.budget_total().units() as f64 / need.max(1.0)
}

/// The lines of the budget that schools and health care read
/// (`Indicator::CareShortfall`); the services worry reads the others but
/// Infrastructure (`SERVICE_EXCLUDED`).
const CARE_LINES: [BudgetCategory; 2] = [BudgetCategory::Education, BudgetCategory::Health];
/// Lines the services worry leaves to schools and health care and to the
/// streets (DEC-106).
const SERVICE_EXCLUDED: [BudgetCategory; 3] = [
    BudgetCategory::Education,
    BudgetCategory::Health,
    BudgetCategory::Infrastructure,
];

/// The budget rule's index of real need (prices × productivity × the
/// nation's population against its start), as `finance::new_budget_year`
/// aims budgets.
fn need_index(r: &Reads<'_>, pop: Option<&PopulationState>) -> f64 {
    let econ = r.economy();
    let fin = r.finance();
    let wage_index = (econ.national.price_level * econ.national.productivity_index).max(0.1);
    let pop_index = pop
        .map(|p| (p.total() as f64 / fin.base_population.max(1.0)).max(0.1))
        .unwrap_or(1.0);
    wage_index * pop_index
}

/// Some lines of a treasury against their need (their shares of its
/// starting budget, indexed by `need_index`): (have, need), yearly.
fn lines_against_need(
    r: &Reads<'_>,
    t: &crate::finance::Treasury,
    lines: &dyn Fn(usize) -> bool,
    index: f64,
) -> (f64, f64) {
    use crate::finance::TierKind;
    let ts = &r.content.economy.treasury_shares;
    let shares = match t.tier {
        TierKind::Local => &ts.local,
        TierKind::Regional => &ts.regional,
        TierKind::National => &ts.national,
    };
    let total: f64 = shares.iter().sum::<f64>().max(1e-9);
    let (mut have, mut share) = (0.0, 0.0);
    for (i, b) in t.budget.iter().enumerate().filter(|(i, _)| lines(*i)) {
        have += b.units() as f64;
        share += shares.get(i).copied().unwrap_or(0.0) / total;
    }
    (have, t.base_budget_total.units() as f64 * share * index)
}

/// Services delivered against need, as a shortfall (`Indicator::ServiceShortfall`):
/// 1 − the town's lines other than Education, Health and Infrastructure
/// against their need − the service gains of laws in effect on lines
/// other than Education and Health. Programmes that crowd out the
/// baseline, cuts and interest raise it; services laws lower it. While
/// every line moves together (the budget rule) the index is the budget's
/// (`budget_index`, which approval reads).
pub fn service_shortfall(r: &Reads<'_>, pop: Option<&PopulationState>, m: usize) -> f64 {
    let fin = r.finance();
    let mods = crate::policy::mods_for(r.policy(), r.content, m as u32, &|id| fin.funded_share(id));
    let care = CARE_LINES.map(crate::policy::category_index);
    let gains: f64 = mods
        .services
        .iter()
        .enumerate()
        .filter(|(i, _)| !care.contains(i))
        .map(|(_, g)| g)
        .sum();
    1.0 - services_index(r, pop, m) - gains
}

/// The town's lines other than Education, Health and Infrastructure
/// against their need (1 at need), the budget index the services worry
/// reads (`service_shortfall`).
pub fn services_index(r: &Reads<'_>, pop: Option<&PopulationState>, m: usize) -> f64 {
    let Some(t) = r.finance().local.get(m) else {
        return 1.0;
    };
    let excluded = SERVICE_EXCLUDED.map(crate::policy::category_index);
    let (have, need) = lines_against_need(r, t, &|i| !excluded.contains(&i), need_index(r, pop));
    if need > 0.0 { have / need } else { 1.0 }
}

/// Schools and health care against need, as a shortfall
/// (`Indicator::CareShortfall`): 1 − the Education and Health lines of
/// every tier that serves the town (its own treasury's whole; its
/// region's and the nation's in the town's share of their residents)
/// against their need − the service gains of laws on those lines.
pub fn care_shortfall(r: &Reads<'_>, pop: Option<&PopulationState>, m: usize) -> f64 {
    use crate::world::Jurisdiction as J;
    let fin = r.finance();
    let Some(mu) = r.world.municipalities.get(m) else {
        return 0.0;
    };
    let index = need_index(r, pop);
    let care = CARE_LINES.map(crate::policy::category_index);
    let lines = |i: usize| care.contains(&i);
    let town_pop = pop.map_or(0.0, |p| p.municipality_total(r.world, mu.id) as f64);
    let region_pop = pop.map_or(0.0, |p| p.region_total(r.world, mu.region) as f64);
    let nation_pop = pop.map_or(0.0, |p| p.total() as f64);
    let share = |part: f64, of: f64| if of > 0.0 { part / of } else { 0.0 };
    let (mut have, mut need) = (0.0, 0.0);
    for (at, part) in [
        (J::town(mu.id), 1.0),
        (J::Region { id: mu.region }, share(town_pop, region_pop)),
        (J::Nation, share(town_pop, nation_pop)),
    ] {
        if let Some(t) = crate::finance::treasury(fin, at) {
            let (h, n) = lines_against_need(r, t, &lines, index);
            have += h * part;
            need += n * part;
        }
    }
    let mods = crate::policy::mods_for(r.policy(), r.content, m as u32, &|id| fin.funded_share(id));
    let gains: f64 = care.iter().filter_map(|i| mods.services.get(*i)).sum();
    let ratio = if need > 0.0 { have / need } else { 1.0 };
    1.0 - ratio - gains
}

/// The Infrastructure line against need, as a shortfall
/// (`Indicator::UpkeepShortfall`): 1 − min(line ÷ need, the upkeep's
/// `maintain_cap`), the index upkeep's wear reads.
pub fn upkeep_shortfall(r: &Reads<'_>, pop: Option<&PopulationState>, m: usize) -> f64 {
    let Some(t) = r.finance().local.get(m) else {
        return 0.0;
    };
    let infra = crate::policy::category_index(BudgetCategory::Infrastructure);
    let (have, need) = lines_against_need(r, t, &|i| i == infra, need_index(r, pop));
    let cap = r.content.upkeep.params.maintain_cap.max(1.0);
    let ratio = if need > 0.0 {
        have.max(0.0) / need
    } else {
        1.0
    };
    1.0 - ratio.min(cap)
}

/// A neighbourhood's streets' wear (`Indicator::StreetWear`): the
/// midpoint of upkeep's starting condition − its condition, or 0 where
/// upkeep keeps none.
fn hood_wear(r: &Reads<'_>, h: NeighborhoodId) -> f64 {
    let Some(c) = r.upkeep().and_then(|u| u.condition(h)) else {
        return 0.0;
    };
    let [lo, hi] = r.content.upkeep.params.start;
    (lo + hi) / 2.0 - f64::from(c)
}

/// A town's streets' wear: its neighbourhoods' (`hood_wear`) weighted by
/// residents (0 where upkeep keeps no condition).
fn town_wear(r: &Reads<'_>, pop: Option<&PopulationState>, m: usize) -> f64 {
    let Some(mu) = r.world.municipalities.get(m) else {
        return 0.0;
    };
    if r.upkeep().is_none_or(|u| u.hoods.is_empty()) {
        return 0.0;
    }
    let (mut sum, mut w) = (0.0, 0.0);
    for &h in &mu.neighborhoods {
        let n = pop.map_or(1.0, |p| (p.hood_total(h.index()) as f64).max(1.0));
        sum += n * hood_wear(r, h);
        w += n;
    }
    if w > 0.0 { sum / w } else { 0.0 }
}

/// The town's politicians' scandals and cases (`Indicator::Scandals`):
/// each scandal that broke within `signals.scandal_days` by its gravity,
/// fading to 0 with age; each open case; each conviction within the days,
/// fading.
pub fn scandals(r: &Reads<'_>, m: usize) -> f64 {
    use mandate_content::Gravity;
    let sg = &r.content.opinion.local_concerns.signals;
    let days = f64::from(sg.scandal_days.max(1));
    let fade = |d: mandate_core::SimDate| (1.0 - f64::from(d.days_until(r.date)) / days).max(0.0);
    let gravity = |g: Gravity| {
        Gravity::ALL
            .iter()
            .position(|x| *x == g)
            .and_then(|i| sg.scandal_gravity.get(i))
            .copied()
            .unwrap_or(1.0)
    };
    let town = m as u32;
    let scandals: f64 = r
        .media()
        .scandals
        .iter()
        .filter(|s| s.town == town && s.broke <= r.date)
        .map(|s| gravity(s.severity) * fade(s.broke))
        .sum();
    let cases: f64 = r
        .justice()
        .cases
        .iter()
        .filter(|c| c.town == town)
        .map(|c| {
            if c.stage.open() {
                sg.case_open
            } else if c.stage == crate::justice::Stage::Convicted {
                let on = c.history.last().map_or(c.opened, |x| x.1);
                sg.case_convicted * fade(on)
            } else {
                0.0
            }
        })
        .sum();
    scandals + cases
}

/// The share of its real value each income class's income (wages,
/// benefits and transfers over prices) lost over the last year: 1 − now ÷
/// a year ago (below 0 when incomes beat prices); 0 without a year of
/// history.
fn real_income_loss(e: &crate::economy::MuniEcon) -> [f64; CLASSES] {
    match (
        e.history.last(),
        e.history.get(e.history.len().saturating_sub(13)),
    ) {
        (Some(now), Some(then)) => std::array::from_fn(|c| {
            let (n, t) = (
                f64::from(now.real_income[c]),
                f64::from(then.real_income[c]),
            );
            if n > 0.0 && t > 0.0 { 1.0 - n / t } else { 0.0 }
        }),
        _ => [0.0; CLASSES],
    }
}

/// Prices against incomes as some adults feel them
/// (`Indicator::CostOfLiving`): each income class's loss
/// (`real_income_loss`) × how strongly it feels prices
/// (`signals.cost_of_living_class`), weighted by the adults of each class.
fn price_squeeze(content: &ContentBundle, loss: &[f64; CLASSES], adults: &[f64; CLASSES]) -> f64 {
    let w = &content.opinion.local_concerns.signals.cost_of_living_class;
    let n: f64 = adults.iter().sum();
    let feel = |c: usize| w.get(c).copied().unwrap_or(1.0);
    if n <= 0.0 {
        return loss[1] * feel(1);
    }
    (0..CLASSES)
        .map(|c| adults[c] / n * feel(c) * loss[c])
        .sum()
}

/// The share of a town's adults aged 65 and over (`None` without adults).
fn old_share(groups: &[f64]) -> Option<f64> {
    let all: f64 = groups.iter().sum();
    let old: f64 = (0..CLASSES)
        .map(|c| groups[crate::population::group_index(AGE_BANDS - 1, c)])
        .sum();
    (all > 0.0).then(|| old / all)
}

/// The share of the nation's adults aged 65 and over.
fn nation_old_share(pop: &PopulationState) -> f64 {
    let (mut old, mut all) = (0u64, 0u64);
    for h in &pop.hoods {
        for (ci, c) in h.cohorts.iter().enumerate() {
            all += u64::from(c.count);
            if age_of(ci) == AGE_BANDS - 1 {
                old += u64::from(c.count);
            }
        }
    }
    if all > 0 {
        old as f64 / all as f64
    } else {
        0.0
    }
}

/// [`indicator`] with the population given (its own systems hold it).
pub fn indicator_with(
    r: &Reads<'_>,
    pop: Option<&PopulationState>,
    m: usize,
    which: Indicator,
) -> f64 {
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
        Indicator::ServiceShortfall => service_shortfall(r, pop, m),
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
        Indicator::CostOfLiving => {
            let groups = pop.map(|p| p.municipality_groups(r.world, MunicipalityId(m as u32)));
            let adults: [f64; CLASSES] = std::array::from_fn(|c| {
                groups.map_or(0.0, |g| {
                    (0..AGE_BANDS)
                        .map(|a| g[crate::population::group_index(a, c)])
                        .sum()
                })
            });
            price_squeeze(r.content, &real_income_loss(e), &adults)
        }
        Indicator::Arrivals => match (pop, pop.and_then(|p| p.flows.get(m))) {
            (Some(p), Some(f)) => {
                let adults: f64 = p
                    .municipality_groups(r.world, MunicipalityId(m as u32))
                    .iter()
                    .sum();
                if adults > 0.0 {
                    f64::from(f.arrived) / adults
                } else {
                    0.0
                }
            }
            _ => 0.0,
        },
        Indicator::YoungLeaving => match (pop, pop.and_then(|p| p.flows.get(m))) {
            (Some(p), Some(f)) => {
                let g = p.municipality_groups(r.world, MunicipalityId(m as u32));
                let young: f64 = (0..CLASSES)
                    .map(|c| g[crate::population::group_index(0, c)])
                    .sum();
                if young > 0.0 {
                    f64::from(f.young_left - f.young_arrived) / young
                } else {
                    0.0
                }
            }
            _ => 0.0,
        },
        Indicator::OldAgeShare => match pop {
            Some(p) => old_share(&p.municipality_groups(r.world, MunicipalityId(m as u32)))
                .map_or(0.0, |s| s - nation_old_share(p)),
            None => 0.0,
        },
        Indicator::CareShortfall => care_shortfall(r, pop, m),
        Indicator::UpkeepShortfall => upkeep_shortfall(r, pop, m),
        Indicator::StreetWear => town_wear(r, pop, m),
        Indicator::Scandals => scandals(r, m),
        Indicator::Construction => {
            let h = &e.housing;
            h.pipeline.iter().sum::<f64>().max(0.0) / h.stock.max(1.0)
        }
        Indicator::WorksDisruption => {
            let fin = r.finance();
            let mods = crate::policy::mods_for(r.policy(), r.content, m as u32, &|id| {
                fin.funded_share(id)
            });
            let local = r.world.municipalities.get(m).map_or(0.0, |mu| {
                let (mut sum, mut w) = (0.0, 0.0);
                for h in &mu.neighborhoods {
                    let n = pop.map_or(1.0, |p| (p.hood_total(h.index()) as f64).max(1.0));
                    sum += n * mods.disruption_at(h.0);
                    w += n;
                }
                if w > 0.0 { sum / w } else { 0.0 }
            });
            mods.disruption + local
        }
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
    // The DEC-106 batch. Prices against each class's income, as its
    // residents' classes feel them.
    let loss = econ.map_or([0.0; CLASSES], real_income_loss);
    let squeeze: Vec<f64> = classes
        .iter()
        .map(|(adults, _)| price_squeeze(content, &loss, adults))
        .collect();
    // Schools and clinics: the residents who rely on them (children and
    // the old weigh more).
    let sg = &lc.signals;
    let users: Vec<f64> = hoods
        .iter()
        .map(|h| {
            let Some(hp) = pop.hoods.get(h.index()) else {
                return 0.0;
            };
            let total = hp.total() as f64;
            if total <= 0.0 {
                return 0.0;
            }
            let seniors: f64 = hp
                .cohorts
                .iter()
                .enumerate()
                .filter(|(ci, _)| age_of(*ci) == AGE_BANDS - 1)
                .map(|(_, c)| c.count as f64)
                .sum();
            (total + sg.care_children * hp.children as f64 + sg.care_seniors * seniors) / total
        })
        .collect();
    let users = relative(&users, &w);
    // An ageing town: each district's own share of adults aged 65+.
    let nation_old = nation_old_share(pop);
    let old: Vec<f64> = hoods
        .iter()
        .map(|h| {
            let Some(hp) = pop.hoods.get(h.index()) else {
                return 0.0;
            };
            let mut g = [0.0; crate::population::GROUPS];
            for (ci, c) in hp.cohorts.iter().enumerate() {
                g[crate::population::group_of(ci)] += c.count as f64;
            }
            old_share(&g).map_or(0.0, |s| s - nation_old)
        })
        .collect();
    // Building: where the town has grown faster than its average.
    let growth: Vec<f64> = hoods
        .iter()
        .zip(&residents)
        .map(|(h, here)| {
            let base = pop
                .hoods
                .get(h.index())
                .map_or(0, |x| x.baseline_population);
            if base > 0 && town_growth > 0.0 {
                (here / base as f64 / town_growth).clamp(0.5, 2.0)
            } else {
                1.0
            }
        })
        .collect();
    let growth = relative(&growth, &w);
    // Works: the town's share and each district's own.
    let (works_town, works_hood): (f64, Vec<f64>) = {
        let fin = r.finance();
        let mods =
            crate::policy::mods_for(r.policy(), content, m as u32, &|id| fin.funded_share(id));
        (
            mods.disruption,
            hoods.iter().map(|h| mods.disruption_at(h.0)).collect(),
        )
    };
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
                    Indicator::TaxBurden
                    | Indicator::BudgetStress
                    | Indicator::ServiceShortfall
                    | Indicator::RecentDisaster
                    | Indicator::Arrivals
                    | Indicator::YoungLeaving
                    | Indicator::UpkeepShortfall
                    | Indicator::Scandals => t,
                    Indicator::CostOfLiving => squeeze[i],
                    Indicator::OldAgeShare => old[i],
                    Indicator::CareShortfall => t * users[i],
                    Indicator::StreetWear => hood_wear(r, h),
                    Indicator::Construction => t * growth[i],
                    Indicator::WorksDisruption => works_town + works_hood[i],
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
    let town: Vec<f64> = Indicator::ALL
        .iter()
        .map(|i| indicator_with(r, Some(pop), m, *i))
        .collect();
    let town_levels: Vec<f64> = content
        .concerns
        .iter()
        .map(|c| c.level_from(|i| town[slot(i)]))
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
                    let level = if c.indicator == Indicator::RecentDisaster {
                        // Felt by how hard the disaster hit here (DEC-090 D8).
                        (tl * pop.impact(h.index())).min(1.0)
                    } else {
                        c.level_from(|i| v[slot(i)])
                    };
                    let add = faults_felt(r, h, c);
                    if add > 0.0 {
                        (level + add).min(c.level.max.max(level))
                    } else {
                        level
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

/// What the faults open on a neighbourhood's streets add to its level of a
/// worry they feed (DEC-114; since DEC-106 the roads-and-upkeep worry): each
/// adds `worry_per_fault`, within the concern's level range. Felt only:
/// the town's measured levels, which drive its politics, are the
/// indicators'. Nothing without upkeep's faults.
fn faults_felt(r: &Reads<'_>, h: NeighborhoodId, c: &mandate_content::ConcernDef) -> f64 {
    let Some(u) = r.upkeep().filter(|u| !u.faults.is_empty()) else {
        return 0.0;
    };
    let n = u.open_feeding(r.content, h, &c.id);
    if n == 0 {
        return 0.0;
    }
    f64::from(n) * r.content.upkeep.params.worry_per_fault
}

/// Each neighbourhood's unemployment against its town's (residents' mean
/// 1), per town (world order) in `Municipality::neighborhoods` order: its
/// income classes' rates and the jobs its workers' sectors have lost since
/// play began (the factor the jobs worry reads). `pop` as `compute_all`.
pub fn hood_unemployment(r: &Reads<'_>, pop: &PopulationState) -> Vec<Vec<f64>> {
    let wp = workplaces(r.world, r.content);
    let ones = [1.0; INDICATORS];
    (0..r.world.municipalities.len())
        .map(|m| {
            hood_indicators(r, pop, m, &ones, &wp)
                .into_iter()
                .map(|(_, _, v)| v[slot(Indicator::Unemployment)])
                .collect()
        })
        .collect()
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
    across_towns(parts, content.concerns.len())
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

// --- The ranked worries of a scope -------------------------------------------------
//
// "What does this place worry about most" has one answer per scope and
// reading, and `ranked` is the only code that gives it: every view, line of
// talk, condition, picker and bot reads it.

/// Whose worries are ranked (tier-generic, DEC-098): everyone a jurisdiction
/// of any tier covers, a district, or one neighbourhood.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope<'a> {
    /// A town, a region or the nation: all its neighbourhoods.
    Jurisdiction(Jurisdiction),
    /// A district: some neighbourhoods (world indices) of one town or of
    /// several (a ward, a race's constituency).
    District(&'a [usize]),
    /// One neighbourhood.
    Neighbourhood(NeighborhoodId),
}

impl Scope<'_> {
    /// A whole town.
    pub fn town(m: MunicipalityId) -> Scope<'static> {
        Scope::Jurisdiction(Jurisdiction::town(m))
    }

    /// The scope's neighbourhoods (world indices).
    pub fn hoods(&self, world: &World) -> Vec<usize> {
        match *self {
            Scope::Jurisdiction(j) => j.hoods(world),
            Scope::District(hs) => hs.to_vec(),
            Scope::Neighbourhood(h) => vec![h.index()],
        }
    }
}

/// The one order of worries: the stronger first (`f64::total_cmp` on the
/// weights); worries of exactly equal weight keep registry order (the
/// lower index first, the order of `content/base/concerns.jsonc`). Nothing
/// else breaks a tie: not a name, a cached order or the caller.
fn order(v: &mut [(usize, f64)]) {
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
}

/// One town's levels for some of its neighbourhoods (registry order): the
/// town's own stored levels when they are all of the town, a single
/// neighbourhood's own, else theirs weighted by residents (`levels_in`).
fn town_levels(
    pop: &PopulationState,
    content: &ContentBundle,
    world: &World,
    m: usize,
    hoods: &[usize],
) -> Vec<f64> {
    let town = pop.town_concerns.get(m).map(|t| t.levels.as_slice());
    if covers_town(world, m, hoods) {
        return town
            .map(|t| t.iter().map(|l| *l as f64).collect())
            .unwrap_or_default();
    }
    if let [h] = hoods
        && let Some(own) = pop.hood_concerns.get(*h)
        && town.is_some_and(|t| t.len() == own.levels.len())
    {
        return own.levels.iter().map(|l| *l as f64).collect();
    }
    levels_in(pop, content, m, hoods)
}

/// Towns' levels as one, each weighted by its residents counted (evenly
/// when none has any).
fn across_towns(mut parts: Vec<(f64, Vec<f64>)>, n: usize) -> Vec<f64> {
    if parts.len() == 1 {
        return parts.pop().map(|(_, l)| l).unwrap_or_default();
    }
    let even = parts.iter().all(|(w, _)| *w <= 0.0);
    let mut sum = vec![0.0; n];
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

/// The level of every worry in a scope (registry order), as its residents
/// live it: a town's stored levels (its neighbourhoods' weighted by
/// residents), a neighbourhood's own, a district's neighbourhoods' weighted
/// by residents; towns are weighted by their residents in the scope. Empty
/// when the scope has no neighbourhood or its concerns are not computed.
pub fn levels_of(
    pop: &PopulationState,
    content: &ContentBundle,
    world: &World,
    scope: Scope<'_>,
) -> Vec<f64> {
    let hoods = scope.hoods(world);
    let n = content.concerns.len();
    let parts: Vec<(f64, Vec<f64>)> = by_town(world, &hoods)
        .iter()
        .map(|(m, hs)| {
            let residents: f64 = hs.iter().map(|h| pop.hood_total(*h) as f64).sum();
            (residents, town_levels(pop, content, world, *m, hs))
        })
        .collect();
    if parts.len() > 1 && parts.iter().any(|(_, l)| l.len() != n) {
        return Vec::new();
    }
    across_towns(parts, n)
}

/// The ranked worries of a scope, strongest first: (registry index, weight)
/// for every concern, those nobody worries about (weight 0) last. The one
/// source of "what this place worries about most"; `top` and `place` read
/// the result.
///
/// `by` names the reading (the two a condition can ask for):
/// - `Worry::Felt`: as residents feel it, each level (`levels_of`) × (1 + ½ ×
///   how far the scope's residents lean the way worried voters do), in
///   [0.5, 1.5] × the level. What people say and what the player is told:
///   the notebook, the town card, the pocket, the canvass, talk, overheard
///   lines and the canvass match.
/// - `Worry::Measured`: by level alone, what a statistic of the place
///   shows. A town's priorities, dilemmas, asks and promises, and a
///   policy family's choice of neighbourhood read it.
///
/// What the player knows of either is a cut of the same list (the first
/// one, three or all by knowledge level), never another ranking. The vote
/// model does not rank: it weighs issues by `salience_in`.
///
/// Ties: see `order` (equal weights keep registry order).
pub fn ranked(
    pop: &PopulationState,
    content: &ContentBundle,
    world: &World,
    scope: Scope<'_>,
    by: Worry,
) -> Vec<(usize, f64)> {
    let levels = levels_of(pop, content, world, scope)
        .into_iter()
        .enumerate();
    let mut v: Vec<(usize, f64)> = match by {
        Worry::Measured => levels.collect(),
        Worry::Felt => {
            let hoods: Vec<usize> = scope
                .hoods(world)
                .into_iter()
                .filter(|h| *h < pop.hoods.len())
                .collect();
            let mean = crate::politics::mean_attitudes(pop, &hoods);
            levels
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
                .collect()
        }
    };
    order(&mut v);
    v
}

/// The `n` strongest worries of a ranking, of those anyone worries about
/// (weight above 0): "the top n".
pub fn top(ranking: &[(usize, f64)], n: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
    ranking.iter().copied().filter(|(_, w)| *w > 0.0).take(n)
}

/// A worry's place in a ranking (0 = the strongest) and its weight, if
/// anyone worries about it: it is among the top n when its place is below n.
pub fn place(ranking: &[(usize, f64)], concern: usize) -> Option<(usize, f64)> {
    top(ranking, usize::MAX)
        .enumerate()
        .find(|(_, (i, _))| *i == concern)
        .map(|(p, (_, w))| (p, w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_weights_keep_registry_order_and_stronger_come_first() {
        let mut v = vec![(0, 0.2), (1, 0.5), (2, 0.2), (3, 0.5), (4, 0.0), (5, 0.9)];
        order(&mut v);
        let ids: Vec<usize> = v.iter().map(|x| x.0).collect();
        assert_eq!(ids, [5, 1, 3, 0, 2, 4]);
        // The same from any starting order.
        let mut w = vec![(4, 0.0), (3, 0.5), (2, 0.2), (5, 0.9), (1, 0.5), (0, 0.2)];
        order(&mut w);
        assert_eq!(v, w);
    }

    #[test]
    fn the_top_and_a_place_leave_out_what_nobody_worries_about() {
        let mut v = vec![(0, 0.0), (1, 0.4), (2, 0.4), (3, 0.0), (4, 0.7)];
        order(&mut v);
        let top3: Vec<usize> = top(&v, 3).map(|x| x.0).collect();
        assert_eq!(top3, [4, 1, 2]);
        assert_eq!(top(&v, 9).count(), 3);
        // Tied worries take successive places, in registry order.
        assert_eq!(place(&v, 1), Some((1, 0.4)));
        assert_eq!(place(&v, 2), Some((2, 0.4)));
        assert_eq!(place(&v, 0), None);
        assert_eq!(place(&v, 9), None);
    }
}
