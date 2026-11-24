//! Layered economy (DEC-019): global cycle and commodities (exogenous),
//! national macro loop (inflation, central bank, output gap), and per-
//! municipality sector jobs, wages, incomes, and housing markets.

pub mod housing;

use crate::events::{EventId, EventKind};
use crate::population::{CLASSES, PopulationState};
use crate::scheduler::{Effect, Outs, Reads};
use crate::spine::{Cause, Causes, Link, Pressure, Process, Shock};
use crate::state::WorldMeta;
use crate::world::{MunicipalityId, NeighborhoodKind, PlaceKind, SettlementClass, World};
use housing::Housing;
use mandate_content::{ContentBundle, ContentKey, ScenarioEconomy, SectorDriver};
use mandate_core::math::{clamp, exp, ln, pow};
use mandate_core::{SimDate, SimRng, Stream};
use serde::{Deserialize, Serialize};

/// Commodities: agriculture, minerals, energy.
pub const COMMODITIES: usize = 3;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Global {
    /// Global demand index (1.0 = trend).
    pub demand: f64,
    pub recession: Option<Recession>,
    /// Commodity price indices (1.0 = long-run mean).
    pub commodities: [f64; COMMODITIES],
    /// A recession whose onset the director holds back (DEC-033 B: timing
    /// only; it begins, as drawn, within `director.chance.recession_hold_months`).
    #[serde(default)]
    pub held_recession: Option<HeldRecession>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeldRecession {
    pub depth: f64,
    pub months_total: u16,
    pub held_on: SimDate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recession {
    pub depth: f64,
    pub months_total: u16,
    pub months_elapsed: u16,
    pub event: Option<EventId>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct National {
    pub price_level: f64,
    /// Annual inflation rate.
    pub inflation: f64,
    /// Central bank policy rate.
    pub policy_rate: f64,
    pub output_gap: f64,
    pub unemployment: f64,
    /// Annualised nominal output.
    pub gdp: f64,
    pub gdp_growth: f64,
    pub tech_index: f64,
    pub manufacturing_competitiveness: f64,
    /// Annual structural trends (economy parameters, or the scenario's era).
    pub tech_growth: f64,
    pub manufacturing_trend: f64,
    pub mean_wage: f64,
    /// Job-weighted mean productivity across all sectors (1.0 at start).
    pub productivity_index: f64,
    pub last_rate_event: f64,
    pub history: Vec<NationalSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NationalSnapshot {
    pub day: u32,
    pub unemployment: f32,
    pub inflation: f32,
    pub policy_rate: f32,
    pub gdp: f64,
    pub global_demand: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SectorState {
    /// Structural job capacity at neutral demand.
    pub capacity: f64,
    pub jobs: f64,
    /// Annual wage per worker (nominal).
    pub wage: f64,
    pub productivity: f64,
    /// Whole jobs in this sector as play began (TOWN_GENERATION §4.1, §15
    /// E1): recorded once by `record_baselines`, never changed after. A
    /// presentation input for the town layout (no system reads it); a sector
    /// that content adds later had none (0).
    pub baseline_jobs: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveShock {
    /// Content key of the shocked sector.
    pub sector: ContentKey,
    pub factor: f64,
    pub months_left: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MuniEcon {
    /// One entry per sector, in `EconomyState::sector_keys` order.
    pub sectors: Vec<SectorState>,
    pub labour_force: f64,
    pub employed: f64,
    pub unemployment: f64,
    /// Employment rate by class (low, middle, high).
    pub class_employment: [f64; CLASSES],
    /// Mean gross annual income per working-age adult by class.
    pub class_income: [f64; CLASSES],
    pub mean_wage: f64,
    /// Annual household consumption (nominal).
    pub consumption: f64,
    /// Baseline consumption at initialisation (retail demand anchor).
    pub base_consumption: f64,
    /// Annualised output (nominal value added).
    pub output: f64,
    pub housing: Housing,
    pub attractiveness: f64,
    pub shocks: Vec<ActiveShock>,
    /// Months since the last disaster (for tourism and attractiveness).
    pub months_since_disaster: u16,
    /// Mean wage at initialisation (unit-labour-cost anchor).
    pub base_mean_wage: f64,
    pub history: Vec<MuniSnapshot>,
}

impl MuniEcon {
    pub fn total_jobs(&self) -> f64 {
        self.sectors.iter().map(|s| s.jobs).sum()
    }
    /// Share of jobs in knowledge-driven sectors.
    pub fn knowledge_share(&self, content: &ContentBundle) -> f64 {
        let t = self.total_jobs().max(1.0);
        self.jobs_with(content, SectorDriver::Knowledge) / t
    }
    /// Jobs in all sectors with a driver.
    pub fn jobs_with(&self, content: &ContentBundle, driver: SectorDriver) -> f64 {
        content
            .sectors_with(driver)
            .filter_map(|(k, _)| self.sectors.get(k))
            .map(|s| s.jobs)
            .sum()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MuniSnapshot {
    pub day: u32,
    pub unemployment: f32,
    pub rent: f32,
    pub jobs: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EconomyState {
    /// Sector content keys this state was built with (DEC-074, DEC-075): the
    /// order of every per-municipality sector list. On load the engine remaps
    /// sector data by key when the content's registry differs.
    pub sector_keys: Vec<ContentKey>,
    pub global: Global,
    pub national: National,
    pub munis: Vec<MuniEcon>,
}

fn local_wage_factor(c: SettlementClass) -> f64 {
    match c {
        SettlementClass::Metro => 1.15,
        SettlementClass::City => 1.05,
        SettlementClass::Town => 0.95,
        SettlementClass::Rural => 0.88,
    }
}

/// Relative structural presence of each sector in a municipality, times the
/// scenario's endowment multiplier for its driver.
fn endowment(world: &World, m: usize, driver: SectorDriver, sc: &ScenarioEconomy) -> f64 {
    let mu = &world.municipalities[m];
    let g = &mu.geo;
    let class = mu.class;
    let industrial = world
        .hoods_of(mu.id)
        .any(|h| world.neighborhoods[h].kind == NeighborhoodKind::Industrial);
    let has = |k: PlaceKind| mu.places.iter().any(|p| world.places[p.index()].kind == k);
    let size = match class {
        SettlementClass::Metro => 3,
        SettlementClass::City => 2,
        SettlementClass::Town => 1,
        SettlementClass::Rural => 0,
    };
    let base = match driver {
        SectorDriver::LandBased => (0.2 + 2.5 * g.fertility) * [1.6, 1.0, 0.3, 0.08][size],
        SectorDriver::Extraction => {
            (g.minerals + 0.6 * g.timber + 0.5 * g.energy + 0.3 * g.fishing)
                * [1.6, 1.2, 0.5, 0.2][size]
        }
        SectorDriver::Industrial => {
            (if industrial { 1.6 } else { 0.5 }) * [0.6, 1.0, 1.2, 1.1][size]
        }
        SectorDriver::Building => 1.0,
        SectorDriver::LocalServices => [0.85, 1.0, 1.05, 1.1][size],
        SectorDriver::Visitor => {
            0.2 + 2.0 * pow(g.scenic, 1.5) + if mu.coastal { 0.3 } else { 0.0 }
        }
        SectorDriver::Knowledge => {
            [0.3, 0.6, 1.4, 2.2][size] + if has(PlaceKind::University) { 0.4 } else { 0.0 }
        }
        SectorDriver::Public => {
            1.0 + if has(PlaceKind::RegionalAssembly) {
                0.2
            } else {
                0.0
            } + if has(PlaceKind::Parliament) { 0.5 } else { 0.0 }
        }
    };
    // Drivers without a multiplier are returned untouched (no extra float op).
    match sc.endowment.get(&driver) {
        Some(f) => base * f,
        None => base,
    }
}

/// Initialise the economy from geography and the starting population.
pub fn init(
    world: &World,
    pop: &PopulationState,
    content: &ContentBundle,
    meta: &WorldMeta,
) -> EconomyState {
    let sc = content.scenario_or_random(meta.params.scenario.as_deref());
    let e = &content.economy;
    let mut munis = Vec::with_capacity(world.municipalities.len());
    let wealth = sc.economy.wealth;
    for (m, mu) in world.municipalities.iter().enumerate() {
        let lf: f64 = pop.labour_force_by_class(world, mu.id).iter().sum();
        let weights: Vec<f64> = content
            .sectors
            .iter()
            .map(|s| s.base_weight * endowment(world, m, s.driver, &sc.economy))
            .collect();
        let wsum: f64 = weights.iter().sum::<f64>().max(1e-9);
        let target_jobs = lf * (1.0 - e.natural_unemployment);
        let sectors: Vec<SectorState> = content
            .sectors
            .iter()
            .zip(&weights)
            .map(|(def, w)| {
                let jobs = target_jobs * w / wsum;
                SectorState {
                    capacity: jobs,
                    jobs,
                    wage: def.base_wage * local_wage_factor(mu.class) * wealth,
                    productivity: 1.0,
                    // Recorded as play begins, after the history burn-in.
                    baseline_jobs: 0,
                }
            })
            .collect();
        let adults: f64 = pop.municipality_groups(world, mu.id).iter().sum();
        let housing = housing::init(mu, adults, e, wealth);
        munis.push(MuniEcon {
            sectors,
            labour_force: lf,
            employed: target_jobs,
            unemployment: e.natural_unemployment,
            class_employment: [0.9, 0.95, 0.97],
            class_income: [0.0; CLASSES],
            mean_wage: 0.0,
            consumption: 0.0,
            base_consumption: 0.0,
            output: 0.0,
            housing,
            attractiveness: 0.0,
            shocks: Vec::new(),
            months_since_disaster: 120,
            base_mean_wage: 0.0,
            history: Vec::new(),
        });
    }
    let tech_growth = sc.era.tech_growth.unwrap_or(e.tech_growth);
    let manufacturing_trend = sc.era.manufacturing_trend.unwrap_or(e.manufacturing_trend);
    let mut st = EconomyState {
        sector_keys: content.sector_keys().cloned().collect(),
        global: Global {
            demand: 1.0,
            recession: None,
            commodities: [1.0; COMMODITIES],
            held_recession: None,
        },
        national: National {
            price_level: 1.0,
            inflation: e.inflation_target,
            policy_rate: e.neutral_real_rate + e.inflation_target,
            tech_index: 1.0,
            productivity_index: 1.0,
            manufacturing_competitiveness: 1.0,
            tech_growth,
            manufacturing_trend,
            unemployment: e.natural_unemployment,
            ..Default::default()
        },
        munis,
    };
    // One settling pass so incomes, consumption, and attractiveness exist.
    for m in 0..st.munis.len() {
        settle_labour_and_incomes(
            &mut st,
            m,
            world,
            pop,
            content,
            content.economy.taxes.income,
            &crate::policy::PolicyMods::default(),
        );
        let c = st.munis[m].consumption;
        st.munis[m].base_consumption = c;
        st.munis[m].base_mean_wage = st.munis[m].mean_wage;
        // Households at the starting rent burden; the stock at target vacancy.
        let adults: f64 = pop
            .municipality_groups(world, MunicipalityId(m as u32))
            .iter()
            .sum();
        let mu = &mut st.munis[m];
        housing::settle_start(&mut mu.housing, adults, mu.class_income[1], e);
    }
    st
}

/// Re-key per-municipality sector data to the content's sector registry
/// (DEC-074, DEC-075). Data for matching keys is kept (in the new order); a
/// sector the content adds starts with no jobs and a structural capacity from
/// its endowment, so it grows in over the following months. Sectors the state
/// has but the content lacks are dropped here: callers refuse such loads first
/// (see `engine::missing_references`). Returns whether anything changed.
pub fn remap_sectors(
    econ: &mut EconomyState,
    world: &World,
    meta: &WorldMeta,
    content: &ContentBundle,
) -> bool {
    let keys: Vec<ContentKey> = content.sector_keys().cloned().collect();
    if econ.sector_keys == keys {
        return false;
    }
    let sc = content.scenario_or_random(meta.params.scenario.as_deref());
    let price = econ.national.price_level.max(0.01);
    let productivity = econ.national.productivity_index.max(0.01);
    let old: Vec<Option<usize>> = keys
        .iter()
        .map(|k| econ.sector_keys.iter().position(|o| o == k))
        .collect();
    for (m, mu) in econ.munis.iter_mut().enumerate() {
        if m >= world.municipalities.len() {
            break;
        }
        let class = world.municipalities[m].class;
        let weights: Vec<f64> = content
            .sectors
            .iter()
            .map(|s| s.base_weight * endowment(world, m, s.driver, &sc.economy))
            .collect();
        let wsum: f64 = weights.iter().sum::<f64>().max(1e-9);
        let jobs = mu.total_jobs();
        let sectors = content
            .sectors
            .iter()
            .zip(&old)
            .zip(&weights)
            .map(|((def, o), w)| match o.and_then(|i| mu.sectors.get(i)) {
                Some(s) => s.clone(),
                None => SectorState {
                    capacity: jobs * w / wsum,
                    jobs: 0.0,
                    wage: def.base_wage * local_wage_factor(class) * price * productivity,
                    productivity,
                    // Not part of the town as play began.
                    baseline_jobs: 0,
                },
            })
            .collect();
        mu.sectors = sectors;
        mu.shocks.retain(|s| keys.contains(&s.sector));
    }
    econ.sector_keys = keys;
    true
}

/// Record every town's economic baselines as play begins (TOWN_GENERATION
/// §15 E1): whole jobs per sector and whole dwellings of housing stock. Called
/// once for a new game (worldgen) and never by a system, so a town's layout
/// plan, sized from these, never moves during a run (DEC-085).
pub fn record_baselines(econ: &mut EconomyState) {
    for mu in &mut econ.munis {
        for s in &mut mu.sectors {
            s.baseline_jobs = whole(s.jobs);
        }
        mu.housing.baseline_stock = whole(mu.housing.stock);
    }
}

/// A non-negative quantity rounded to whole units (saturating; NaN is 0).
pub fn whole(x: f64) -> u32 {
    x.max(0.0).round() as u32
}

// --- Systems --------------------------------------------------------------------

/// Monthly: global demand cycle, recessions, and commodity prices.
pub fn global(econ: &mut EconomyState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let p = &r.content.economy;
    // The world economy is exogenous: its draws are keyed by month so nothing
    // that happens in any town (or a balance change to local systems) can
    // reshuffle when global recessions arrive.
    let _ = &out.rng;
    let mut keyed =
        mandate_core::rng::keyed(r.meta.seed, Stream::Economy, &[0x6C0B, r.date.0 as u64]);
    let rng = &mut keyed;
    let g = &mut econ.global;
    let target = match &mut g.recession {
        Some(rec) => {
            rec.months_elapsed += 1;
            let t = rec.months_elapsed as f64 / rec.months_total.max(1) as f64;
            // Sharp fall, slower recovery.
            let shape = if t < 0.35 {
                t / 0.35
            } else {
                (1.0 - t).max(0.0) / 0.65
            };
            1.0 - rec.depth * shape
        }
        None => 1.0,
    };
    if g.recession
        .as_ref()
        .is_some_and(|rec| rec.months_elapsed >= rec.months_total)
    {
        g.recession = None;
        // The recession's length was part of the exogenous draw; the log
        // links the recovery to the onset (the recession's thread).
        out.events.record(
            r.date,
            EventKind::GlobalRecoveryBegan,
            Cause::Exogenous(Shock::WorldEconomy),
        );
    } else if g.recession.is_none() && g.held_recession.is_some() {
        // A held onset begins once the hold lifts, or at the latest after
        // the longest hold the director may impose.
        let hold = r
            .narrative()
            .chance
            .recession_hold
            .is_some_and(|u| r.date < u);
        let max_days = r.content.director.chance.recession_hold_months as u32 * 31;
        if let Some(h) = g.held_recession.clone()
            && (!hold || h.held_on.add_days(max_days) <= r.date)
        {
            g.held_recession = None;
            out.events.record(
                r.date,
                EventKind::DirectorIntervention {
                    lever: "recession_released".into(),
                    municipality: u32::MAX,
                },
                Cause::Process(Process::Director),
            );
            // Exogenous still: the director moved its date, not its cause
            // (the thread links the hold and the release).
            let ev = out.events.record(
                r.date,
                EventKind::GlobalRecessionBegan {
                    depth_pct: (h.depth * 100.0).round() as u32,
                },
                Cause::Exogenous(Shock::WorldEconomy),
            );
            out.events.pin(ev);
            g.recession = Some(Recession {
                depth: h.depth,
                months_total: h.months_total,
                months_elapsed: 0,
                event: Some(ev),
            });
        }
    } else if g.recession.is_none() && rng.chance(p.recession_monthly_chance) {
        let depth = rng.range_f64(p.recession_depth[0], p.recession_depth[1]);
        let months =
            rng.range_i64(p.recession_months[0] as i64, p.recession_months[1] as i64) as u16;
        let hold = r
            .narrative()
            .chance
            .recession_hold
            .is_some_and(|u| r.date < u);
        if hold {
            g.held_recession = Some(HeldRecession {
                depth,
                months_total: months,
                held_on: r.date,
            });
            out.events.record(
                r.date,
                EventKind::DirectorIntervention {
                    lever: "recession_held".into(),
                    municipality: u32::MAX,
                },
                [
                    Cause::Exogenous(Shock::WorldEconomy),
                    Cause::Process(Process::Director),
                ],
            );
        } else {
            let ev = out.events.record(
                r.date,
                EventKind::GlobalRecessionBegan {
                    depth_pct: (depth * 100.0).round() as u32,
                },
                Cause::Exogenous(Shock::WorldEconomy),
            );
            out.events.pin(ev);
            g.recession = Some(Recession {
                depth,
                months_total: months,
                months_elapsed: 0,
                event: Some(ev),
            });
        }
    }
    g.demand = clamp(
        0.8 * g.demand + 0.2 * target + rng.normal(0.0, 0.004),
        0.6,
        1.3,
    );
    for (i, c) in g.commodities.iter_mut().enumerate() {
        let mut lp = ln(c.max(0.05));
        lp += -0.04 * lp + rng.normal(0.0, p.commodity_volatility);
        if rng.chance(0.006) {
            let jump = rng.range_f64(0.18, 0.4) * if rng.chance(0.5) { 1.0 } else { -1.0 };
            lp += jump;
            out.events.record(
                r.date,
                EventKind::CommodityShock {
                    commodity: i as u8,
                    change_pct: (jump * 100.0).round() as i32,
                },
                Cause::Exogenous(Shock::Commodities),
            );
        }
        *c = clamp(exp(lp), 0.3, 3.0);
    }
}

/// Recompute labour market, wages, and incomes for one municipality.
fn settle_labour_and_incomes(
    econ: &mut EconomyState,
    m: usize,
    world: &World,
    pop: &PopulationState,
    content: &ContentBundle,
    income_tax: [f64; CLASSES],
    mods: &crate::policy::PolicyMods,
) {
    let e = &content.economy;
    let price = econ.national.price_level.max(0.01);
    // Higher real rates encourage saving over spending.
    let neutral = e.neutral_real_rate + e.inflation_target;
    let propensity = clamp(1.0 - 2.5 * (econ.national.policy_rate - neutral), 0.85, 1.1);
    let mid = MunicipalityId(m as u32);
    let mut lf_c = pop.labour_force_by_class(world, mid);
    for (c, v) in lf_c.iter_mut().enumerate() {
        *v *= (1.0 + mods.labour_supply[c]).max(0.5);
    }
    let groups = pop.municipality_groups(world, mid);
    let mu = &mut econ.munis[m];
    let lf: f64 = lf_c.iter().sum::<f64>().max(1.0);
    let demand = mu.total_jobs();
    // Frictional unemployment never falls below ~2.5%.
    let employed = demand.min(lf * 0.975);
    let u = clamp(1.0 - employed / lf, 0.0, 0.6);
    let wage_bill: f64 = mu.sectors.iter().map(|s| s.jobs * s.wage).sum();
    let mean_wage = wage_bill / demand.max(1.0);
    // Class employment: weaker classes absorb more of the slack.
    let raw = [1.0 - u * 1.7, 1.0 - u * 0.85, 1.0 - u * 0.45].map(|x| clamp(x, 0.2, 1.0));
    let implied: f64 = (0..CLASSES).map(|c| raw[c] * lf_c[c]).sum::<f64>().max(1.0);
    let scale = employed / implied;
    let rates = raw.map(|x| clamp(x * scale, 0.1, 1.0));
    let knowledge = mu.knowledge_share(content);
    // A local wage floor lifts low-paid wages (its job cost is applied to sector demand).
    let class_wage = [
        0.6 * mean_wage * (1.0 + mods.wage_floor),
        mean_wage,
        (2.0 + knowledge) * mean_wage,
    ];
    let class_income: [f64; CLASSES] = std::array::from_fn(|c| {
        rates[c] * class_wage[c]
            + (1.0 - rates[c]) * e.unemployment_replacement * mean_wage
            + mods.transfer[c] * price
    });
    // Consumption: working-age by class plus pensioners.
    let mut consumption = 0.0;
    for c in 0..CLASSES {
        let working: f64 = (0..3)
            .map(|a| groups[crate::population::group_index(a, c)])
            .sum();
        let seniors = groups[crate::population::group_index(3, c)];
        let disp = class_income[c] * (1.0 - income_tax[c]);
        consumption += working * disp * [0.95, 0.85, 0.7][c]
            + seniors * e.pension_replacement * mean_wage * 0.9;
    }
    let consumption = consumption * propensity;
    mu.labour_force = lf;
    mu.employed = employed;
    mu.unemployment = u;
    mu.class_employment = rates;
    mu.class_income = class_income;
    mu.mean_wage = mean_wage;
    mu.consumption = consumption;
    mu.output = wage_bill * 1.6;
    let rent_burden = mu.housing.rent * 12.0 / class_income[1].max(1.0);
    let capital_bonus = if world.capital == Some(mid) { 0.2 } else { 0.0 };
    let disaster = if mu.months_since_disaster < 24 {
        0.4 * (1.0 - mu.months_since_disaster as f64 / 24.0)
    } else {
        0.0
    };
    let services: f64 = mods.services.iter().sum();
    mu.attractiveness = -4.0 * u + 0.5 * ln((mean_wage / price) / 45_000.0) - 1.5 * rent_burden
        + 0.3 * world.municipalities[m].geo.scenic
        + 0.8 * services
        + capital_bonus
        - disaster
        - mods.disruption;
}

/// A sector's demand in a town scaled by `factor` for `months` months (a
/// shock a command asked for, such as an inspection's lost output). The
/// economy's own change; unlike a disaster's shock it does not mark the town
/// as recently struck.
pub fn add_shock(
    econ: &mut EconomyState,
    municipality: u32,
    sector: &mandate_content::ContentKey,
    factor: f64,
    months: u16,
) {
    if let Some(mu) = econ.munis.get_mut(municipality as usize)
        && factor.is_finite()
        && months > 0
    {
        mu.shocks.push(ActiveShock {
            sector: sector.clone(),
            factor: clamp(factor, 0.1, 2.0),
            months_left: months,
        });
    }
}

/// Monthly: sector demand, jobs, closures, wages, housing, incomes.
pub fn markets(econ: &mut EconomyState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let world = r.world;
    let pop = r.population();
    let env = r.environment();
    let fin = r.finance();
    let content = r.content;
    let nm = world.municipalities.len().min(econ.munis.len());
    // Apply effects from other domains.
    for e in out.take(|e| matches!(e, Effect::HousingDamage { .. } | Effect::SectorShock { .. })) {
        match e.effect {
            Effect::HousingDamage {
                municipality,
                fraction,
            } => {
                if let Some(mu) = econ.munis.get_mut(municipality as usize) {
                    mu.housing.stock *= 1.0 - clamp(fraction, 0.0, 0.5);
                    mu.months_since_disaster = 0;
                }
            }
            Effect::SectorShock {
                municipality,
                sector,
                factor,
                months,
            } => {
                if let Some(mu) = econ.munis.get_mut(municipality as usize) {
                    mu.shocks.push(ActiveShock {
                        sector,
                        factor,
                        months_left: months,
                    });
                    mu.months_since_disaster = 0;
                }
            }
            _ => {}
        }
    }
    let g = econ.global.clone();
    let nat = econ.national.clone();
    let rate = nat.policy_rate;
    let rng: &mut SimRng = out.rng.get(Stream::Economy);
    let recession_cause = g.recession.as_ref().and_then(|rec| rec.event);
    for m in 0..nm {
        let region = world.municipalities[m].region.index();
        let renv = env.regions.get(region);
        let mineral_stock = renv.map(|x| x.mineral_stock).unwrap_or(1.0);
        let water = renv.map(|x| x.water).unwrap_or(1.0);
        let public_index = fin.public_spending_index(m);
        let mods =
            crate::policy::mods_for(r.policy(), content, m as u32, &|id| fin.funded_share(id));
        let lf: f64 = pop
            .labour_force_by_class(world, MunicipalityId(m as u32))
            .iter()
            .sum();
        let mu = &mut econ.munis[m];
        mu.months_since_disaster = mu.months_since_disaster.saturating_add(1);
        // Domestic demand is measured in real terms; jobs needed fall as
        // productivity rises.
        let price = nat.price_level.max(0.01);
        // Real incomes grow with average productivity, so domestic demand is
        // normalised by it (balanced growth).
        let jobs_now = mu.total_jobs().max(1.0);
        let avg_prod = (mu
            .sectors
            .iter()
            .map(|x| x.jobs * x.productivity)
            .sum::<f64>()
            / jobs_now)
            .max(0.01);
        let retail_driver = pow(
            (mu.consumption / price / mu.base_consumption.max(1.0) / avg_prod).max(0.1),
            0.9,
        );
        let public_index = public_index / price / avg_prod;
        // Unit labour costs: wages rising faster than prices × productivity
        // erode the competitiveness of traded sectors.
        let ulc = (mu.mean_wage / (mu.base_mean_wage.max(1.0) * price * avg_prod)).max(0.2);
        let competitive = |elasticity: f64| pow(ulc, -elasticity);
        let construction_driver =
            pow(housing::starts_index(&mu.housing).max(0.1), 0.6) * pow(public_index.max(0.1), 0.3);
        for (k, def) in content.sectors.iter().enumerate() {
            if k >= mu.sectors.len() {
                break;
            }
            let gcyc = pow(g.demand, def.cyclicality);
            let driver = match def.driver {
                SectorDriver::LandBased => {
                    gcyc * pow(g.commodities[0], 0.4) * pow(water.max(0.2), 0.5)
                }
                SectorDriver::Extraction => {
                    gcyc * pow(g.commodities[1] * 0.6 + g.commodities[2] * 0.4, 0.6)
                        * pow(mineral_stock.max(0.05), 0.5)
                        * competitive(0.3)
                }
                SectorDriver::Industrial => {
                    gcyc * nat.manufacturing_competitiveness * competitive(1.0)
                }
                SectorDriver::Building => gcyc * construction_driver,
                SectorDriver::LocalServices => retail_driver,
                SectorDriver::Visitor => {
                    gcyc * competitive(0.8)
                        * if mu.months_since_disaster < 12 {
                            0.75
                        } else {
                            1.0
                        }
                }
                // Technology raises knowledge-sector productivity, not headcount.
                SectorDriver::Knowledge => gcyc * competitive(0.8),
                SectorDriver::Public => public_index,
            };
            let shock: f64 = mu
                .shocks
                .iter()
                .filter(|s| s.sector == def.id)
                .map(|s| s.factor)
                .product();
            // Policy: wage subsidies add jobs; a wage floor costs some low-paid jobs.
            let subsidy = 1.0
                + mods.subsidy_elasticity.get(k).copied().unwrap_or(0.0)
                    * mods.subsidy.get(k).copied().unwrap_or(0.0);
            let floor_cost = match def.driver {
                SectorDriver::LocalServices | SectorDriver::Visitor | SectorDriver::LandBased => {
                    1.0 - 0.4 * mods.wage_floor
                }
                _ => 1.0,
            };
            let shock = shock * subsidy * floor_cost;
            let s = &mut mu.sectors[k];
            // Local services scale with the labour force; tradables drift with trends.
            match def.driver {
                SectorDriver::LocalServices | SectorDriver::Public | SectorDriver::Building => {
                    s.capacity *= 1.0 + clamp((lf / mu.labour_force.max(1.0)) - 1.0, -0.02, 0.02);
                }
                SectorDriver::Industrial => s.capacity *= 1.0 + nat.manufacturing_trend / 12.0,
                SectorDriver::LandBased => s.capacity *= 1.0 - 0.004 / 12.0,
                _ => {}
            }
            // Structural jobs a policy adds (`sector_capacity`, DEC-092): on
            // top of the sector's own capacity while the law runs.
            let target = match mods.capacity.get(k) {
                Some(extra) => (s.capacity + extra).max(0.0) * driver * shock,
                None => s.capacity * driver * shock,
            };
            let speed = if target < s.jobs { 0.16 } else { 0.1 };
            s.jobs = (s.jobs + (target - s.jobs) * speed).max(0.0);
            let extra = if def.driver == SectorDriver::Knowledge {
                nat.tech_growth - 0.02
            } else {
                0.0
            };
            s.productivity *= 1.0 + (def.productivity_growth + extra) / 12.0;
            // Plant closures in tradable sectors under sustained weak demand.
            if matches!(
                def.driver,
                SectorDriver::Industrial | SectorDriver::Extraction
            ) && driver < 0.85
                && s.capacity > 200.0
                && rng.chance(0.03 * (0.85 - driver) * 10.0)
            {
                let frac = rng.range_f64(0.08, 0.25);
                let jobs = (s.capacity * frac).round();
                s.capacity -= jobs;
                // Why (the spine): the plant closed because the sector's
                // demand driver is weak. The driver is a product of named
                // factors; the weakest names the pressure. A running
                // recession is linked as well.
                let factors: &[(Pressure, f64)] = match def.driver {
                    SectorDriver::Extraction => &[
                        (Pressure::WorldDemand, gcyc),
                        (
                            Pressure::CommodityPrices,
                            pow(g.commodities[1] * 0.6 + g.commodities[2] * 0.4, 0.6),
                        ),
                        (Pressure::Depletion, pow(mineral_stock.max(0.05), 0.5)),
                        (Pressure::LabourCosts, competitive(0.3)),
                    ],
                    _ => &[
                        (Pressure::WorldDemand, gcyc),
                        (Pressure::Competitiveness, nat.manufacturing_competitiveness),
                        (Pressure::LabourCosts, competitive(1.0)),
                    ],
                };
                let weakest = factors
                    .iter()
                    .fold(factors[0], |a, b| if b.1 < a.1 { *b } else { a })
                    .0;
                out.events.record(
                    r.date,
                    EventKind::PlantClosure {
                        municipality: m as u32,
                        sector: def.id.clone(),
                        jobs: jobs as u32,
                    },
                    Causes::from(Cause::Pressure(weakest)).and_event(recession_cause, Link::Caused),
                );
            }
        }
        // Structural adjustment: idle labour slowly attracts new private
        // capacity (and a tight market slowly sheds it). Decline therefore
        // hurts for years but is not permanent.
        let slack = mu.unemployment - content.economy.natural_unemployment;
        let adjust = clamp(slack * 0.25, -0.01, 0.04) / 12.0;
        for (s, def) in mu.sectors.iter_mut().zip(&content.sectors) {
            if matches!(
                def.driver,
                SectorDriver::LocalServices
                    | SectorDriver::Knowledge
                    | SectorDriver::Visitor
                    | SectorDriver::Industrial
                    | SectorDriver::Building
            ) {
                s.capacity *= 1.0 + adjust;
            }
        }
        mu.shocks.retain_mut(|s| {
            s.months_left = s.months_left.saturating_sub(1);
            s.months_left > 0
        });
        // Wages respond to local tightness, prices, and productivity.
        let tight = clamp(
            content.economy.natural_unemployment - mu.unemployment,
            -0.1,
            0.1,
        );
        let class = world.municipalities[m].class;
        for (s, def) in mu.sectors.iter_mut().zip(&content.sectors) {
            let base = def.base_wage * local_wage_factor(class);
            let target = base * nat.price_level * s.productivity * (1.0 + 2.0 * tight);
            s.wage += (target - s.wage) * 0.08;
        }
        // The town's tax rate reaches prices (the finance domain's rate, a
        // law's change on top).
        let property_tax = fin
            .taxes
            .property
            .get(m)
            .copied()
            .unwrap_or(content.economy.taxes.property)
            + mods.property_tax_delta;
        housing::step(
            &mut mu.housing,
            rate,
            nat.inflation,
            nat.price_level,
            &content.economy,
            &housing::Rules {
                public_units_per_year: mods.housing_units_per_year,
                starts_multiplier: mods.starts_multiplier,
                property_tax,
                rent_cap: mods.rent_cap,
            },
        );
    }
    for m in 0..nm {
        let adults: f64 = pop
            .municipality_groups(world, MunicipalityId(m as u32))
            .iter()
            .sum();
        let mods =
            crate::policy::mods_for(r.policy(), content, m as u32, &|id| fin.funded_share(id));
        settle_labour_and_incomes(econ, m, world, pop, content, fin.taxes.income, &mods);
        let mu = &mut econ.munis[m];
        // Households form at this month's rent burden (next month's demand).
        mu.housing.households = housing::households(
            adults,
            mu.housing.rent,
            mu.class_income[1],
            &content.economy.housing,
        );
        mu.history.push(MuniSnapshot {
            day: r.date.0,
            unemployment: mu.unemployment as f32,
            rent: mu.housing.rent as f32,
            jobs: mu.total_jobs() as f32,
        });
        if mu.history.len() > 36 {
            mu.history.remove(0);
        }
    }
}

/// Monthly: national aggregates, inflation, and the central bank.
pub fn national(econ: &mut EconomyState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let p = &r.content.economy;
    let rng = out.rng.get(Stream::Economy);
    let lf: f64 = econ
        .munis
        .iter()
        .map(|m| m.labour_force)
        .sum::<f64>()
        .max(1.0);
    let emp: f64 = econ.munis.iter().map(|m| m.employed).sum();
    let gdp: f64 = econ.munis.iter().map(|m| m.output).sum();
    let wage_jobs: f64 = econ
        .munis
        .iter()
        .map(|m| m.total_jobs())
        .sum::<f64>()
        .max(1.0);
    let n = &mut econ.national;
    let u = 1.0 - emp / lf;
    n.gdp_growth = if n.gdp > 0.0 { gdp / n.gdp - 1.0 } else { 0.0 };
    n.gdp = gdp;
    n.unemployment = u;
    n.mean_wage = econ
        .munis
        .iter()
        .map(|m| m.mean_wage * m.total_jobs())
        .sum::<f64>()
        / wage_jobs;
    n.productivity_index = econ
        .munis
        .iter()
        .flat_map(|m| m.sectors.iter().map(|s| s.jobs * s.productivity))
        .sum::<f64>()
        / wage_jobs;
    n.output_gap = clamp((p.natural_unemployment - u) * 2.5, -0.15, 0.15);
    let energy_shock = (econ.global.commodities[2] - 1.0) * 0.04;
    // Expectations-augmented Phillips curve (monthly smoothing toward an
    // equilibrium set by the unemployment gap and energy prices).
    let equilibrium = p.inflation_target + 0.45 * (p.natural_unemployment - u) + energy_shock;
    n.inflation = clamp(
        0.9 * n.inflation + 0.1 * equilibrium + rng.normal(0.0, 0.0012),
        -0.03,
        0.25,
    );
    n.price_level *= 1.0 + n.inflation / 12.0;
    let taylor = p.neutral_real_rate
        + n.inflation
        + 0.5 * (n.inflation - p.inflation_target)
        + 0.5 * n.output_gap;
    n.policy_rate = clamp(n.policy_rate + (taylor - n.policy_rate) * 0.25, 0.0, 0.2);
    if (n.policy_rate - n.last_rate_event).abs() >= 0.005 {
        let bp = ((n.policy_rate - n.last_rate_event) * 10_000.0).round() as i32;
        out.events.record(
            r.date,
            EventKind::InterestRateMove {
                change_bp: bp,
                rate_bp: (n.policy_rate * 10_000.0).round() as u32,
            },
            Cause::Process(Process::CentralBank),
        );
        n.last_rate_event = n.policy_rate;
    }
    n.tech_index *= 1.0 + n.tech_growth / 12.0;
    n.manufacturing_competitiveness = clamp(
        n.manufacturing_competitiveness * (1.0 + rng.normal(0.0, 0.003)),
        0.5,
        1.5,
    );
    n.history.push(NationalSnapshot {
        day: r.date.0,
        unemployment: u as f32,
        inflation: n.inflation as f32,
        policy_rate: n.policy_rate as f32,
        gdp,
        global_demand: econ.global.demand as f32,
    });
    if n.history.len() > 480 {
        n.history.remove(0);
    }
}
