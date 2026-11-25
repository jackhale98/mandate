//! The impact estimate (DEC-111, `docs/design/IMPACT_ESTIMATE.md`): what a
//! bill will probably do, before it passes.
//!
//! Three layers, each a pure function:
//!
//! 1. **Direct quantities, by the simulation's own arithmetic.** The small
//!    functions at the top ([`scale`], [`linear`], [`book`], …) are what
//!    `mods_for`, `implementation` and `delivered` compute with, so the
//!    estimate and the law's ledger cannot drift apart. [`project`] runs a
//!    bill's months forward on them: funding by finance's own rules
//!    (`finance::first_share`, `finance::capped_share`), the ramp, and the
//!    ledger as it would read at 12, 36 and 60 months.
//! 2. **Pass-through, from the lab.** [`estimate`] multiplies the direct
//!    quantities (as shares of the town: the *bases*) by the coefficients
//!    of `content.impact`, fitted on twins by `mandate-cli lab impact
//!    calibrate`, and adds the ranges in quadrature.
//! 3. **The frozen estimate.** At enactment the estimate is kept with the
//!    law as integers ([`FrozenEstimate`]), so the ledger can later be
//!    read against what the bill said.
//!
//! The estimator reads [`ImpactInputs`], published statistics built by one
//! function, never the state: it cannot see the seed, a stream, the
//! director, held hazards or hidden facts, and it draws no number. No game
//! is forked when a bill is opened.

use super::{PolicyInstance, PolicyState, Resolved};
use crate::scheduler::Reads;
use crate::world::Jurisdiction;
use mandate_content::{
    AttributionParams, ClassKey, ContentBundle, HazardKey, ImpactBasis, ImpactOutcome, ImpactTable,
    MechanismDef, SideEffectDef,
};
use mandate_core::SimDate;
use mandate_core::math::{clamp, sqrt};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use mandate_content::IMPACT_HORIZONS as HORIZONS;

/// Years from its enactment an ended law keeps what it said when it passed.
pub const KEPT_YEARS: u32 = 12;

// --- Direct arithmetic, shared with `mods_for`, `implementation`, `delivered` ----------

/// How far a law's mechanisms run now: its progress, pilot size, funded
/// share and (a family option's) effect factor.
pub fn scale(p: &PolicyInstance, funded: f64, effect_factor: f64) -> f64 {
    p.progress as f64 * p.pilot_scale() * funded * effect_factor
}

/// The share of a programme still being built (its disruption).
pub fn building(p: &PolicyInstance, funded: f64) -> f64 {
    (1.0 - p.progress as f64).max(0.0) * p.pilot_scale() * funded
}

/// A month's implementation progress: a programme of `months` takes
/// `months ÷ (0.5 + competence)` at full funding.
pub fn progress_step(funded: f64, competence: f64, months: f64) -> f64 {
    funded * (0.5 + competence) / months
}

/// A mechanism's size: `per_unit` × its parameter, at scale `k`.
pub fn linear(per_unit: f64, level: f64, k: f64) -> f64 {
    per_unit * level * k
}

/// A parameter in its own unit (homes a year, currency a year), at scale `k`.
pub fn flat(level: f64, k: f64) -> f64 {
    level * k
}

/// A parameter in per cent as a share, at scale `k`.
pub fn percent(level: f64, k: f64) -> f64 {
    level / 100.0 * k
}

/// Jobs a wage subsidy of `pct` per cent supports in a sector of `capacity`.
pub fn jobs_supported(capacity: f64, elasticity: f64, pct: f64, k: f64) -> f64 {
    capacity * elasticity * pct / 100.0 * k
}

/// A property-tax change of `points` percentage points as a rate: in force
/// in full once enacted (no ramp, no funding).
pub fn tax_delta(points: f64, pilot: f64) -> f64 {
    points / 100.0 * pilot.max(1.0)
}

/// A month's rent growth the market would bring (vacancy against its
/// target, plus inflation), and that month's inflation: what a cap is set
/// against.
pub fn rent_market(e: &mandate_content::EconomyParams, vacancy: f64, inflation: f64) -> (f64, f64) {
    let infl = inflation / 12.0;
    let market = (e.target_vacancy - vacancy) * e.housing.rent_response + infl;
    (market, infl)
}

/// A parameter's value (0 when the bill has none by that id).
fn level(params: &BTreeMap<String, f64>, id: &str) -> f64 {
    params.get(id).copied().unwrap_or(0.0)
}

/// What a month's ledger entries read besides the law's own parameters.
pub struct Books<'a> {
    /// The law's scale this month ([`scale`]).
    pub k: f64,
    /// The construction lag in months and the yearly demolition rate.
    pub lag: f64,
    pub demolition: f64,
    pub net: &'a AttributionParams,
    /// Jobs capacity of a sector, by content key.
    pub capacity: &'a dyn Fn(&str) -> f64,
    /// Private housing starts last month (all starts less public
    /// programmes'), where a zoning law is in effect.
    pub private_starts: Option<f64>,
    /// [`rent_market`], where the town's economy is known.
    pub rent_market: Option<(f64, f64)>,
}

/// One mechanism's month in a law's ledger (audit fix 5): direct counts,
/// exact, and what they added, estimated with `economy.attribution`. The
/// one place these are computed: `implementation` books the month that
/// happened, [`project`] the months a bill expects.
pub fn book(
    ledger: &mut BTreeMap<String, f64>,
    mech: &MechanismDef,
    params: &BTreeMap<String, f64>,
    b: &Books<'_>,
) {
    let k = b.k;
    match mech {
        MechanismDef::HousingSupply { param } => {
            let started = flat(level(params, param), k) / 12.0;
            super::record_homes(
                ledger,
                super::HomesStarted::Exact(started),
                b.lag,
                b.demolition,
                b.net.homes_net,
            );
        }
        MechanismDef::SectorSubsidy {
            sector,
            param,
            elasticity,
        } => {
            let supported =
                jobs_supported((b.capacity)(sector), *elasticity, level(params, param), k);
            let s = super::sector_key(sector);
            ledger.insert(format!("jobs_supported_{s}"), supported);
            ledger.insert(format!("jobs_added_{s}"), supported * b.net.jobs_net);
        }
        MechanismDef::Transfer { param, .. } => {
            *ledger.entry("paid_per_person".into()).or_default() +=
                flat(level(params, param), k) / 12.0;
        }
        MechanismDef::Service {
            param, per_unit, ..
        } => {
            ledger.insert(
                "service_gain_pct".into(),
                linear(*per_unit, level(params, param), k) * 100.0,
            );
        }
        MechanismDef::Zoning { param, per_unit } => {
            let extra = linear(*per_unit, level(params, param), k);
            ledger.insert("extra_building_pct".into(), extra * 100.0);
            // The private starts it brought: last month's private starts ×
            // extra ÷ (1 + extra), an estimate like what they add.
            if let Some(private) = b.private_starts {
                let started = private * extra / (1.0 + extra).max(0.1);
                super::record_homes(
                    ledger,
                    super::HomesStarted::Estimated(started),
                    b.lag,
                    b.demolition,
                    b.net.homes_net,
                );
            }
        }
        MechanismDef::RentCap {
            param,
            loosest,
            per_unit,
        } => {
            if let Some(cap) = super::rent_cap_of(level(params, param), *loosest, *per_unit) {
                ledger.insert("rent_cap_pct".into(), cap * 100.0);
                // How far rents are held below where the market would take
                // them (an estimate): what the cap withholds each month it
                // binds, given back as rents catch up.
                if let Some((market, infl)) = b.rent_market {
                    let held = ledger.get("rent_held_pct").copied().unwrap_or(0.0) / 100.0;
                    let held = if market > cap / 12.0 {
                        held + market - cap / 12.0
                    } else {
                        (held - (market - infl).max(0.0)).max(0.0)
                    };
                    ledger.insert("rent_held_pct".into(), held * 100.0);
                }
            }
        }
        MechanismDef::WageFloor { param } => {
            ledger.insert("wage_uplift_pct".into(), flat(level(params, param), k));
            ledger.insert(
                "low_income_added_pct".into(),
                flat(level(params, param), k) * b.net.wage_floor_net,
            );
        }
        MechanismDef::HazardProtection {
            param, per_unit, ..
        } => {
            ledger.insert(
                "damage_reduction_pct".into(),
                (linear(*per_unit, level(params, param), k) * 100.0).min(90.0),
            );
        }
        MechanismDef::LocalTaxRate { param } => {
            ledger.insert("tax_change_pts".into(), level(params, param));
        }
        MechanismDef::SectorCapacity {
            sector,
            param,
            per_unit,
        } => {
            let created = linear(*per_unit, level(params, param), k);
            let s = super::sector_key(sector);
            ledger.insert(format!("jobs_created_{s}"), created);
            ledger.insert(format!("jobs_added_{s}"), created * b.net.jobs_net);
        }
        MechanismDef::Borrow { share } => {
            ledger.insert("borrowed_pct".into(), share * 100.0);
        }
        MechanismDef::Revenue { .. } => {}
    }
}

// --- What the estimator may read ----------------------------------------------------

/// A programme the jurisdiction already funds.
#[derive(Clone, Debug, PartialEq)]
pub struct Programme {
    pub policy: u32,
    /// Its yearly cost in full, and the share of it borrowed.
    pub annual_cost: f64,
    pub borrow_share: f64,
    pub funded_share: f64,
}

/// A jurisdiction's published statistics: everything an estimate reads
/// besides the bill and the content. Built by [`inputs`] and nowhere else.
#[derive(Clone, Debug, PartialEq)]
pub struct ImpactInputs {
    pub at: Jurisdiction,
    pub date: SimDate,
    pub residents: f64,
    /// Residents by neighbourhood (a bill's `area` counts these).
    pub hood_residents: BTreeMap<u32, f64>,
    pub jobs: f64,
    /// Jobs and capacity by sector content key.
    pub sector_jobs: BTreeMap<String, f64>,
    pub sector_capacity: BTreeMap<String, f64>,
    /// Income per working-age adult by class (low, middle, high).
    pub class_income: [f64; 3],
    pub stock: f64,
    pub vacancy: f64,
    pub home_price: f64,
    /// Private housing starts last month (all starts less programmes').
    pub private_starts: f64,
    pub price_level: f64,
    pub inflation: f64,
    /// The treasury's yearly budget, and its room for a new law this year.
    pub budget: f64,
    pub room: f64,
    pub programmes: Vec<Programme>,
    /// The agency's competence (0..1).
    pub competence: f64,
    /// Months until the body that votes on bills here is next elected.
    pub months_to_election: Option<u32>,
}

/// The published statistics of a jurisdiction, for estimates of bills
/// there. `None` for a tier the table has no rows for (it returns no
/// estimate, not a guess; DEC-098) or a jurisdiction with no economy or
/// treasury of its own.
pub fn inputs(r: &Reads<'_>, policy: &PolicyState, at: Jurisdiction) -> Option<ImpactInputs> {
    let content = r.content;
    if !content.impact.covers(at.tier()) {
        return None;
    }
    let town = at.municipality()?;
    let m = town.index();
    let econ = r.economy();
    let e = econ.munis.get(m)?;
    let fin = r.finance();
    let treasury = crate::finance::treasury(fin, at)?;
    let pop = r.population();
    let hood_residents: BTreeMap<u32, f64> = at
        .hoods(r.world)
        .into_iter()
        .map(|h| (h as u32, pop.hood_total(h) as f64))
        .collect();
    let by_sector = |f: &dyn Fn(&crate::economy::SectorState) -> f64| -> BTreeMap<String, f64> {
        econ.sector_keys
            .iter()
            .zip(&e.sectors)
            .map(|(k, s)| (k.to_string(), f(s)))
            .collect()
    };
    let public =
        super::mods_for(policy, content, town.0, &|id| fin.funded_share(id)).housing_units_per_year;
    let months_to_election = content
        .constitutions
        .get(&r.meta.constitution)
        .and_then(|c| {
            let tier = c.tier(at.tier())?;
            let body = tier.legislature()?;
            let cal = body.calendar.as_ref().or(tier.elections.as_ref())?;
            let day = crate::politics::elections::next_election_day(
                cal,
                (body.term_years as u32).max(1),
                r.date,
            );
            Some((r.date.days_until(day) as f64 / 30.44) as u32)
        });
    Some(ImpactInputs {
        at,
        date: r.date,
        residents: pop.municipality_total(r.world, town) as f64,
        hood_residents,
        jobs: e.total_jobs(),
        sector_jobs: by_sector(&|s| s.jobs),
        sector_capacity: by_sector(&|s| s.capacity),
        class_income: e.class_income,
        stock: e.housing.stock,
        vacancy: e.housing.vacancy,
        home_price: e.housing.price,
        private_starts: (e.housing.last_starts - public / 12.0).max(0.0),
        price_level: econ.national.price_level,
        inflation: econ.national.inflation,
        budget: treasury.budget_total().units() as f64,
        room: crate::finance::room(treasury),
        programmes: fin
            .appropriations
            .iter()
            .filter(|(_, a)| a.municipality == town.0)
            .map(|(id, a)| Programme {
                policy: *id,
                annual_cost: a.annual_cost,
                borrow_share: a.borrow_share,
                funded_share: a.funded_share,
            })
            .collect(),
        competence: policy.agencies.get(m).map(|a| a.competence).unwrap_or(0.6) as f64,
        months_to_election,
    })
}

// --- The months ahead -----------------------------------------------------------------

/// Where a bill's money comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaidFrom {
    /// The budget as it stands.
    Rates,
    /// Mostly borrowed.
    Borrowing,
    /// A levy or charge the bill brings.
    Levy,
    /// The budget, by pro-rating every programme (the quarter cap).
    Squeeze,
}

impl PaidFrom {
    pub fn key(self) -> &'static str {
        match self {
            PaidFrom::Rates => "rates",
            PaidFrom::Borrowing => "borrowing",
            PaidFrom::Levy => "levy",
            PaidFrom::Squeeze => "squeeze",
        }
    }
}

/// A bill's money: exact, never a range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cost {
    /// Yearly cost in full, and the yearly income it brings (nominal).
    pub yearly: f64,
    pub revenue: f64,
    /// The cost as a share of the jurisdiction's budget.
    pub budget_share: f64,
    pub borrow_share: f64,
    /// The share funded until the next budget year, and from then on.
    pub funded_first: f64,
    pub funded_later: f64,
    /// What the programmes already running are funded at from the next
    /// budget year without this bill: above `funded_later`, the bill
    /// dilutes them.
    pub funded_without: f64,
    pub paid_from: PaidFrom,
}

/// When a bill takes effect, in months after enactment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timing {
    /// The first month it is funded and work begins (`None`: never, at
    /// this funding).
    pub start: Option<u32>,
    /// The month it is in full effect (`None`: not within ten years).
    pub full: Option<u32>,
    /// Every mechanism is a rule in force from the day it passes.
    pub at_once: bool,
}

/// A bill's months ahead by the simulation's own arithmetic: its funding,
/// its ramp, and its ledger as it would read at each horizon.
#[derive(Clone, Debug, PartialEq)]
pub struct Projection {
    pub cost: Cost,
    pub timing: Timing,
    /// The law's scale ([`scale`]) at each horizon.
    pub k: [f64; 3],
    /// The ledger as `implementation` would have written it by each horizon.
    pub ledger: [BTreeMap<String, f64>; 3],
    /// The direct quantities as shares of the town, by basis, at each horizon.
    pub bases: BTreeMap<ImpactBasis, [f64; 3]>,
    /// Jobs the ledger would count as added, by sector content key.
    pub sector_added: BTreeMap<String, [f64; 3]>,
    /// Debt added by the bill's own borrowing, over the budget.
    pub borrowed: [f64; 3],
}

/// The monthly step (counted from 1 after enactment) at which the next
/// budget year's funding takes over: the January step still runs on the
/// first year's share.
fn budget_step(date: SimDate) -> u32 {
    let c = date.calendar();
    // On the first of a month that month's step is still to come.
    13 - c.month + u32::from(c.day == 1)
}

/// Run a bill's months forward from enactment on `x.date`.
pub fn project(
    x: &ImpactInputs,
    content: &ContentBundle,
    t: &Resolved<'_>,
    p: &PolicyInstance,
) -> Projection {
    let e = &content.economy;
    let residents = match p.area() {
        Some(hoods) => hoods
            .iter()
            .filter_map(|h| x.hood_residents.get(&h.0))
            .sum(),
        None => x.residents,
    };
    let yearly = super::annual_cost(t, p, residents, x.price_level);
    let revenue = super::annual_revenue(t, p, residents, x.price_level);
    let borrow = super::borrow_share(t);
    // Funding: finance's two rules, with this bill beside the programmes
    // already running (its own appropriation, if it has one, set aside).
    let others = || x.programmes.iter().filter(|a| a.policy != p.id.0);
    let needs = yearly * (1.0 - borrow);
    let existing: f64 = others()
        .map(|a| a.annual_cost * a.funded_share * (1.0 - a.borrow_share))
        .sum();
    let full: f64 = others()
        .map(|a| a.annual_cost * (1.0 - a.borrow_share))
        .sum();
    let funded_first = crate::finance::first_share(x.room, existing, needs);
    let funded_later = crate::finance::capped_share(x.budget, full + needs);
    let funded_without = crate::finance::capped_share(x.budget, full);
    let turn = budget_step(x.date);
    let funded_at = |step: u32| -> f64 {
        if step <= 1 {
            // The appropriation is made at the end of the first month.
            0.0
        } else if step <= turn {
            funded_first
        } else {
            funded_later
        }
    };
    let levy = t.mechanisms.iter().any(|m| match m {
        MechanismDef::LocalTaxRate { param } => p.param(param) > 0.0,
        MechanismDef::Revenue { .. } => revenue >= yearly && revenue > 0.0,
        _ => false,
    });
    let paid_from = if borrow >= 0.5 {
        PaidFrom::Borrowing
    } else if levy {
        PaidFrom::Levy
    } else if funded_later < 1.0 && needs > 0.0 {
        PaidFrom::Squeeze
    } else {
        PaidFrom::Rates
    };
    let cost = Cost {
        yearly,
        revenue,
        budget_share: if x.budget > 0.0 {
            yearly / x.budget
        } else {
            0.0
        },
        borrow_share: borrow,
        funded_first,
        funded_later,
        funded_without,
        paid_from,
    };

    // The ramp and the ledger, month by month, as `implementation` does.
    let months = t.implementation_months.max(1) as f64;
    let mut law = p.clone();
    law.progress = 0.0;
    let lag = e.construction_lag_months.max(1) as f64;
    let capacity = |sector: &str| x.sector_capacity.get(sector).copied().unwrap_or(0.0);
    let market = rent_market(e, x.vacancy, x.inflation);
    let mut ledger: BTreeMap<String, f64> = BTreeMap::new();
    // One ledger per mechanism beside the law's, to tell their bases apart.
    let mut own: Vec<BTreeMap<String, f64>> = vec![BTreeMap::new(); t.mechanisms.len()];
    let mut out = Projection {
        cost,
        timing: Timing {
            start: None,
            full: None,
            at_once: t.mechanisms.iter().all(|m| {
                matches!(
                    m,
                    MechanismDef::LocalTaxRate { .. }
                        | MechanismDef::RentCap { .. }
                        | MechanismDef::Borrow { .. }
                        | MechanismDef::Revenue { .. }
                )
            }),
        },
        k: [0.0; 3],
        ledger: Default::default(),
        bases: BTreeMap::new(),
        sector_added: BTreeMap::new(),
        borrowed: [0.0; 3],
    };
    let mut borrowed = 0.0;
    let last = HORIZONS[2];
    for step in 1..=120u32 {
        let funded = funded_at(step);
        if funded > 0.0 {
            out.timing.start.get_or_insert(step);
            law.progress = clamp(
                law.progress as f64 + progress_step(funded, x.competence, months),
                0.0,
                1.0,
            ) as f32;
            if law.progress >= 1.0 {
                out.timing.full.get_or_insert(step);
            }
        }
        if step > last {
            if out.timing.full.is_some() || funded <= 0.0 {
                break;
            }
            continue;
        }
        let k = scale(&law, funded, t.effect_factor);
        // Looser zoning lifts the private starts it is booked against.
        let extra: f64 = t
            .mechanisms
            .iter()
            .map(|m| match m {
                MechanismDef::Zoning { param, per_unit } => linear(*per_unit, p.param(param), k),
                _ => 0.0,
            })
            .sum();
        let books = Books {
            k,
            lag,
            demolition: e.housing.demolition_rate,
            net: &e.attribution,
            capacity: &capacity,
            private_starts: Some(x.private_starts * (1.0 + extra).max(0.0)),
            rent_market: Some(market),
        };
        for (j, mech) in t.mechanisms.iter().enumerate() {
            book(&mut ledger, mech, &law.params, &books);
            book(&mut own[j], mech, &law.params, &books);
        }
        ledger.insert("funded_pct".into(), funded * 100.0);
        borrowed += yearly * funded * borrow / 12.0;
        let Some(h) = HORIZONS.iter().position(|m| *m == step) else {
            continue;
        };
        out.k[h] = k;
        out.ledger[h] = ledger.clone();
        out.borrowed[h] = if x.budget > 0.0 {
            borrowed / x.budget
        } else {
            0.0
        };
        let mut add = |b: ImpactBasis, v: f64| {
            out.bases.entry(b).or_insert([0.0; 3])[h] += v;
        };
        // Every bill is a law.
        add(ImpactBasis::Law, 1.0);
        if x.budget > 0.0 {
            add(
                ImpactBasis::Spend,
                yearly * funded * (1.0 - borrow) / x.budget,
            );
        }
        for (j, mech) in t.mechanisms.iter().enumerate() {
            let get = |key: &str| own[j].get(key).copied().unwrap_or(0.0);
            match mech {
                MechanismDef::HousingSupply { .. } => add(
                    ImpactBasis::HousingSupply,
                    get("homes_added") / x.stock.max(1.0),
                ),
                MechanismDef::Zoning { .. } => {
                    add(ImpactBasis::Zoning, get("homes_added") / x.stock.max(1.0))
                }
                MechanismDef::RentCap { .. } => {
                    add(ImpactBasis::RentCap, get("rent_held_pct") / 100.0)
                }
                MechanismDef::SectorSubsidy { sector, .. }
                | MechanismDef::SectorCapacity { sector, .. } => {
                    let v = get(&format!("jobs_added_{}", super::sector_key(sector)));
                    add(ImpactBasis::JobsAdded, v / x.jobs.max(1.0));
                    out.sector_added
                        .entry(sector.to_string())
                        .or_insert([0.0; 3])[h] += v;
                }
                MechanismDef::WageFloor { .. } => {
                    add(ImpactBasis::WageFloor, get("low_income_added_pct") / 100.0)
                }
                MechanismDef::Transfer { class, param } => add(
                    ImpactBasis::Transfer,
                    flat(p.param(param), k) * x.price_level
                        / x.class_income[super::class_index(*class)].max(1.0),
                ),
                MechanismDef::Service { .. } => {
                    add(ImpactBasis::Service, get("service_gain_pct") / 100.0)
                }
                MechanismDef::LocalTaxRate { param } => add(
                    ImpactBasis::LocalTaxRate,
                    tax_delta(p.param(param), p.pilot_scale()) * x.home_price * x.stock
                        / x.budget.max(1.0),
                ),
                MechanismDef::HazardProtection { .. }
                | MechanismDef::Borrow { .. }
                | MechanismDef::Revenue { .. } => {}
            }
        }
    }
    out
}

// --- The estimate ---------------------------------------------------------------------

/// An outcome at one horizon: the middle 80% of what the lab saw, and its
/// median. In the outcome's own unit.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Range {
    pub lo: f64,
    pub mid: f64,
    pub hi: f64,
}

impl Range {
    /// `up`, `down`, or `flat` when the range does not exclude zero ("no
    /// clear change").
    pub fn direction(&self) -> &'static str {
        if self.lo > 0.0 {
            "up"
        } else if self.hi < 0.0 {
            "down"
        } else {
            "flat"
        }
    }

    /// Too wide to state as a number (the design's "not vacuous" rule):
    /// the far end is more than three times the near one.
    pub fn vague(&self) -> bool {
        let (near, far) = if self.lo > 0.0 {
            (self.lo, self.hi)
        } else if self.hi < 0.0 {
            (-self.hi, -self.lo)
        } else {
            return false;
        };
        far > 3.0 * near
    }

    pub fn contains(&self, v: f64) -> bool {
        self.lo <= v && v <= self.hi
    }
}

/// The unit an outcome is stated in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    /// Homes or jobs.
    Count,
    /// Per cent of the level without the law.
    Percent,
    /// Points of approval (of 100).
    Points,
    /// Currency.
    Money,
}

impl Unit {
    pub fn key(self) -> &'static str {
        match self {
            Unit::Count => "count",
            Unit::Percent => "percent",
            Unit::Points => "points",
            Unit::Money => "money",
        }
    }
}

/// One outcome of a bill at the three horizons.
#[derive(Clone, Debug, PartialEq)]
pub struct OutcomeLine {
    pub outcome: ImpactOutcome,
    /// The sector, for `sector_jobs` (content key).
    pub sector: Option<String>,
    pub unit: Unit,
    pub at: [Range; 3],
    /// The bill's own purpose (its first mechanism's outcome).
    pub lead: bool,
}

impl OutcomeLine {
    /// The key the frozen estimate files it under (`sector_jobs.retail`).
    pub fn key(&self) -> String {
        match &self.sector {
            Some(s) => format!("{}.{}", self.outcome.key(), super::sector_key(s)),
            None => self.outcome.key().to_string(),
        }
    }

    /// Whether a movement in `direction` is a cost (a trade-off) for
    /// this outcome.
    pub fn adverse(&self, direction: &str) -> bool {
        match self.outcome {
            ImpactOutcome::Rent | ImpactOutcome::Debt => direction == "up",
            _ => direction == "down",
        }
    }
}

/// A direct count of a bill at the three horizons: arithmetic, the same the
/// law's ledger will book (`ledger_key`).
#[derive(Clone, Debug, PartialEq)]
pub struct DirectLine {
    /// `homes_started` | `extra_building` | `jobs_supported` |
    /// `jobs_created` | `paid` | `wage_uplift` | `service_level` |
    /// `tax_points` | `rent_cap` | `protection` | `labour_supply`
    pub kind: &'static str,
    /// The ledger entry it will be read against, if the ledger has one.
    pub ledger_key: Option<String>,
    pub at: [f64; 3],
    /// What it names: a sector (content key), a class, a hazard or a
    /// budget category.
    pub sector: Option<String>,
    pub class: Option<ClassKey>,
    pub hazard: Option<HazardKey>,
    pub category: Option<mandate_content::BudgetCategory>,
}

/// A simulated downside (DEC-073), stated by arithmetic: its kind, whom it
/// touches and its size are never a lab figure.
#[derive(Clone, Debug, PartialEq)]
pub struct SideLine {
    /// `disruption` | `grievance` | `labour_supply`
    pub kind: &'static str,
    pub class: Option<ClassKey>,
    /// Only residents of 45 and over.
    pub older: bool,
    /// Its size at these parameters (positive is harm).
    pub amount: f64,
}

/// A group that gains or loses, and by how much it is felt.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupLine {
    /// `low` | `middle` | `high` | `older` | `area`
    pub group: &'static str,
    pub gains: bool,
    /// Approval points among the group (the middle of the range), or a
    /// side effect's size × 100: for ordering, not for show.
    pub size: f64,
}

/// What a bill will probably do. Unslanted, at full knowledge: views gate
/// and round it (the protocol crate).
#[derive(Clone, Debug, PartialEq)]
pub struct Estimate {
    pub at: Jurisdiction,
    pub on: SimDate,
    pub cost: Cost,
    pub timing: Timing,
    pub direct: Vec<DirectLine>,
    pub outcomes: Vec<OutcomeLine>,
    pub side_effects: Vec<SideLine>,
    pub groups: Vec<GroupLine>,
    /// The horizon the lead lines are read at (an index into [`HORIZONS`]):
    /// the last before the next election of the body that votes on it.
    pub focus: usize,
    /// Whether that horizon falls before the election.
    pub in_time: bool,
}

/// `coefficient × basis`, summed over a bill's bases, with the deviations
/// added in quadrature. `None` when the table has no row for any of them.
fn compose(
    table: &ImpactTable,
    bases: &[(ImpactBasis, f64)],
    outcome: ImpactOutcome,
    months: u32,
) -> Option<Range> {
    let (mut mid, mut below, mut above) = (0.0, 0.0, 0.0);
    let mut any = false;
    for (basis, b) in bases {
        let Some(r) = table.row(*basis, outcome, months) else {
            continue;
        };
        any = true;
        let m = r.p50 * b;
        let (a, z) = (r.p10 * b, r.p90 * b);
        let (lo, hi) = (a.min(z), a.max(z));
        mid += m;
        below += (m - lo) * (m - lo);
        above += (hi - m) * (hi - m);
    }
    any.then(|| Range {
        lo: mid - sqrt(below),
        mid,
        hi: mid + sqrt(above),
    })
}

fn scaled(r: Range, by: f64) -> Range {
    Range {
        lo: r.lo * by,
        mid: r.mid * by,
        hi: r.hi * by,
    }
}

/// The outcome a mechanism is for.
fn purpose(m: &MechanismDef) -> Option<ImpactOutcome> {
    Some(match m {
        MechanismDef::HousingSupply { .. } | MechanismDef::Zoning { .. } => ImpactOutcome::Homes,
        MechanismDef::SectorSubsidy { .. } | MechanismDef::SectorCapacity { .. } => {
            ImpactOutcome::SectorJobs
        }
        MechanismDef::Transfer { .. } | MechanismDef::WageFloor { .. } => ImpactOutcome::LowIncome,
        // A levy is for what its revenue buys.
        MechanismDef::Service { .. } | MechanismDef::LocalTaxRate { .. } => ImpactOutcome::Services,
        MechanismDef::RentCap { .. } => ImpactOutcome::Rent,
        MechanismDef::HazardProtection { .. }
        | MechanismDef::Borrow { .. }
        | MechanismDef::Revenue { .. } => return None,
    })
}

/// What a bill will probably do in the jurisdiction `x` describes, enacted
/// now. A pure function of its arguments: no state, no stream.
pub fn estimate(
    x: &ImpactInputs,
    content: &ContentBundle,
    t: &Resolved<'_>,
    p: &PolicyInstance,
) -> Estimate {
    let table = &content.impact;
    let pr = project(x, content, t, p);
    let (focus, in_time) = match x.months_to_election {
        Some(m) => match HORIZONS.iter().rposition(|h| *h <= m) {
            Some(i) => (i, true),
            None => (0, false),
        },
        None => (1, false),
    };

    // Direct counts, read off the projected ledger.
    let mut direct: Vec<DirectLine> = Vec::new();
    let line = |kind: &'static str, key: Option<String>, at: [f64; 3]| DirectLine {
        kind,
        ledger_key: key,
        at,
        sector: None,
        class: None,
        hazard: None,
        category: None,
    };
    let read = |key: &str| -> [f64; 3] {
        std::array::from_fn(|h| pr.ledger[h].get(key).copied().unwrap_or(0.0))
    };
    let mut homes = false;
    for mech in &t.mechanisms {
        match mech {
            MechanismDef::HousingSupply { .. } => {
                if !homes {
                    homes = true;
                    direct.push(line(
                        "homes_started",
                        Some("homes_started".into()),
                        read("homes_started"),
                    ));
                }
            }
            MechanismDef::Zoning { .. } => direct.push(line(
                "extra_building",
                Some("extra_building_pct".into()),
                read("extra_building_pct"),
            )),
            MechanismDef::SectorSubsidy { sector, .. } => {
                let key = format!("jobs_supported_{}", super::sector_key(sector));
                direct.push(DirectLine {
                    sector: Some(sector.to_string()),
                    ..line("jobs_supported", Some(key.clone()), read(&key))
                });
            }
            MechanismDef::SectorCapacity { sector, .. } => {
                let key = format!("jobs_created_{}", super::sector_key(sector));
                direct.push(DirectLine {
                    sector: Some(sector.to_string()),
                    ..line("jobs_created", Some(key.clone()), read(&key))
                });
            }
            MechanismDef::Transfer { class, param } => direct.push(DirectLine {
                class: Some(*class),
                // Paid a year to an adult of the class, in today's money.
                ..line(
                    "paid",
                    None,
                    std::array::from_fn(|h| flat(p.param(param), pr.k[h]) * x.price_level),
                )
            }),
            MechanismDef::WageFloor { .. } => direct.push(line(
                "wage_uplift",
                Some("wage_uplift_pct".into()),
                read("wage_uplift_pct"),
            )),
            MechanismDef::Service {
                category,
                param,
                per_unit,
            } => direct.push(DirectLine {
                category: Some(*category),
                ..line(
                    "service_level",
                    Some("service_gain_pct".into()),
                    std::array::from_fn(|h| linear(*per_unit, p.param(param), pr.k[h]) * 100.0),
                )
            }),
            MechanismDef::LocalTaxRate { .. } => direct.push(line(
                "tax_points",
                Some("tax_change_pts".into()),
                read("tax_change_pts"),
            )),
            MechanismDef::RentCap { .. } => {
                if pr.ledger[0].contains_key("rent_cap_pct") {
                    direct.push(line(
                        "rent_cap",
                        Some("rent_cap_pct".into()),
                        read("rent_cap_pct"),
                    ));
                }
            }
            MechanismDef::HazardProtection {
                hazard,
                param,
                per_unit,
            } => {
                if let Some(hz) = hazard.key() {
                    direct.push(DirectLine {
                        hazard: Some(hz),
                        ..line(
                            "protection",
                            None,
                            std::array::from_fn(|h| {
                                (linear(*per_unit, p.param(param), pr.k[h]) * 100.0).min(90.0)
                            }),
                        )
                    });
                }
            }
            MechanismDef::Borrow { .. } | MechanismDef::Revenue { .. } => {}
        }
    }

    // Side effects, by arithmetic.
    let mut side_effects: Vec<SideLine> = Vec::new();
    for fx in &t.side_effects {
        let raw = super::side_effect_amount(fx, p);
        let a = raw * p.pilot_scale();
        match fx {
            SideEffectDef::Disruption { .. } => side_effects.push(SideLine {
                kind: "disruption",
                class: None,
                older: false,
                amount: a,
            }),
            SideEffectDef::Grievance {
                class,
                min_age_band,
                ..
            } => side_effects.push(SideLine {
                kind: "grievance",
                class: *class,
                older: *min_age_band >= 2,
                amount: a,
            }),
            SideEffectDef::LabourSupply { class, .. } => {
                side_effects.push(SideLine {
                    kind: "labour_supply",
                    class: Some(*class),
                    older: false,
                    // Fewer people working is the harm.
                    amount: -a,
                });
                direct.push(DirectLine {
                    class: Some(*class),
                    ..line(
                        "labour_supply",
                        None,
                        // The share of the class's labour force, in per cent.
                        std::array::from_fn(|h| raw * pr.k[h] * 100.0),
                    )
                });
            }
        }
    }

    // Outcomes: coefficient × basis.
    let bases_at = |h: usize| -> Vec<(ImpactBasis, f64)> {
        pr.bases.iter().map(|(b, v)| (*b, v[h])).collect()
    };
    let lead = t.mechanisms.iter().find_map(purpose);
    let mut outcomes: Vec<OutcomeLine> = Vec::new();
    for outcome in ImpactOutcome::ALL {
        if outcome == ImpactOutcome::SectorJobs {
            for (i, (sector, added)) in pr.sector_added.iter().enumerate() {
                let jobs = x.sector_jobs.get(sector).copied().unwrap_or(0.0);
                if jobs <= 0.0 {
                    continue;
                }
                let at: Option<Vec<Range>> = (0..3)
                    .map(|h| {
                        compose(
                            table,
                            &[(ImpactBasis::JobsAdded, added[h] / jobs)],
                            outcome,
                            HORIZONS[h],
                        )
                        .map(|r| scaled(r, jobs))
                    })
                    .collect();
                if let Some(at) = at {
                    outcomes.push(OutcomeLine {
                        outcome,
                        sector: Some(sector.clone()),
                        unit: Unit::Count,
                        at: [at[0], at[1], at[2]],
                        lead: lead == Some(outcome) && i == 0,
                    });
                }
            }
            continue;
        }
        let (unit, by) = match outcome {
            ImpactOutcome::Homes => (Unit::Count, x.stock),
            ImpactOutcome::Jobs => (Unit::Count, x.jobs),
            ImpactOutcome::Debt => (Unit::Money, x.budget),
            ImpactOutcome::Rent | ImpactOutcome::LowIncome | ImpactOutcome::Services => {
                (Unit::Percent, 100.0)
            }
            _ => (Unit::Points, 100.0),
        };
        let at: Option<Vec<Range>> = (0..3)
            .map(|h| {
                compose(table, &bases_at(h), outcome, HORIZONS[h]).map(|mut r| {
                    if outcome == ImpactOutcome::Debt {
                        // The bill's own borrowing: arithmetic, no spread.
                        r.lo += pr.borrowed[h];
                        r.mid += pr.borrowed[h];
                        r.hi += pr.borrowed[h];
                    }
                    scaled(r, by)
                })
            })
            .collect();
        if let Some(at) = at {
            outcomes.push(OutcomeLine {
                outcome,
                sector: None,
                unit,
                at: [at[0], at[1], at[2]],
                lead: lead == Some(outcome),
            });
        }
    }

    // Who gains and who loses: approval by class where the range is
    // clear, and the groups a side effect names.
    let mut groups: Vec<GroupLine> = Vec::new();
    for (outcome, group) in [
        (ImpactOutcome::ApprovalLow, "low"),
        (ImpactOutcome::ApprovalMiddle, "middle"),
        (ImpactOutcome::ApprovalHigh, "high"),
    ] {
        if let Some(o) = outcomes.iter().find(|o| o.outcome == outcome) {
            let r = o.at[focus];
            match r.direction() {
                "up" => groups.push(GroupLine {
                    group,
                    gains: true,
                    size: r.mid.abs(),
                }),
                "down" => groups.push(GroupLine {
                    group,
                    gains: false,
                    size: r.mid.abs(),
                }),
                _ => {}
            }
        }
    }
    for fx in &side_effects {
        let group = match fx.kind {
            "grievance" if fx.older => "older",
            "disruption" => "area",
            _ => continue,
        };
        if fx.amount != 0.0 && !groups.iter().any(|g| g.group == group) {
            groups.push(GroupLine {
                group,
                gains: fx.amount < 0.0,
                size: fx.amount.abs() * 100.0,
            });
        }
    }
    groups.sort_by(|a, b| b.size.total_cmp(&a.size));

    Estimate {
        at: x.at,
        on: x.date,
        cost: pr.cost,
        timing: pr.timing,
        direct,
        outcomes,
        side_effects,
        groups,
        focus,
        in_time,
    }
}

/// The estimate of a bill or law in its own jurisdiction, as things stand
/// (`inst.at()`, DEC-098). `None` where the table covers no such tier.
pub fn estimate_for(r: &Reads<'_>, policy: &PolicyState, p: &PolicyInstance) -> Option<Estimate> {
    let t = super::resolve(r.content, policy, p)?;
    let x = inputs(r, policy, p.at())?;
    Some(estimate(&x, r.content, &t, p))
}

// --- Kept with the law ------------------------------------------------------------------

/// What the bill said when it passed (DEC-111 decision 5): the unslanted
/// estimate at enactment, as integers, so "what it did" can later be read
/// against it in the same quantities. Thousandths of each quantity's unit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenEstimate {
    pub on: SimDate,
    /// Yearly cost in currency units.
    pub cost: i64,
    /// The share funded until the next budget year and after it, per mille.
    pub funded: [u16; 2],
    /// Direct counts at 12, 36 and 60 months, by the ledger key each will
    /// be booked under (`homes_started`, `jobs_supported_retail`, …).
    pub direct: BTreeMap<String, [i64; 3]>,
    /// Net outcomes, by outcome key (`homes`, `sector_jobs.retail`,
    /// `rent`, …): at each horizon, the range's low end, middle and high end.
    pub outcomes: BTreeMap<String, [[i64; 3]; 3]>,
}

fn milli(v: f64) -> i64 {
    if v.is_finite() {
        mandate_core::math::round_half_away(v * 1000.0) as i64
    } else {
        0
    }
}

impl Estimate {
    pub fn freeze(&self) -> FrozenEstimate {
        FrozenEstimate {
            on: self.on,
            cost: self.cost.yearly as i64,
            funded: [
                milli(self.cost.funded_first) as u16,
                milli(self.cost.funded_later) as u16,
            ],
            direct: self
                .direct
                .iter()
                .filter_map(|d| Some((d.ledger_key.clone()?, d.at.map(milli))))
                .collect(),
            outcomes: self
                .outcomes
                .iter()
                .map(|o| {
                    (
                        o.key(),
                        o.at.map(|r| [milli(r.lo), milli(r.mid), milli(r.hi)]),
                    )
                })
                .collect(),
        }
    }
}

/// The ledger entry a frozen outcome is read against, where the ledger
/// keeps one (`homes` → `homes_added`): the same quantity, before and after.
pub fn ledger_key(outcome: &str) -> Option<String> {
    match outcome {
        "homes" => Some("homes_added".into()),
        "low_income" => Some("low_income_added_pct".into()),
        k => k
            .strip_prefix("sector_jobs.")
            .map(|s| format!("jobs_added_{s}")),
    }
}
