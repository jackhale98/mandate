//! Municipal housing markets: stock, households, vacancy, rents, prices,
//! construction.
//!
//! Rent clears vacancy: it rises while vacancy is below target. Households
//! answer rent: as a year's rent takes more of a middle-class income,
//! adults double up or form households later, so effective demand falls and
//! rent is bounded by incomes. Builders answer price over build cost and
//! vacancy; zoning scales private building and public programmes add homes
//! directly. The starting state is an equilibrium: at the target vacancy and
//! the neutral rate, starts equal the baseline and stay there. Parameters:
//! `EconomyParams::housing` (content).

use crate::world::{Municipality, SettlementClass};
use mandate_content::{EconomyParams, HousingParams};
use mandate_core::math::{clamp, pow};
use serde::{Deserialize, Serialize};

/// Numerical guard on vacancy (headship keeps it far inside in practice).
const VACANCY_BOUNDS: (f64, f64) = (-0.5, 0.6);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Housing {
    pub stock: f64,
    /// Households formed: adults over adults per household at the current
    /// rent burden (`households`).
    pub households: f64,
    pub vacancy: f64,
    /// Average monthly rent (nominal).
    pub rent: f64,
    /// Average dwelling price (nominal).
    pub price: f64,
    /// Completions due in each coming month (front = next month).
    pub pipeline: Vec<f64>,
    /// Baseline monthly starts (for the construction-demand index).
    pub base_starts: f64,
    pub last_starts: f64,
    /// Local build cost at price level 1.0 (land and labour are local).
    pub build_cost: f64,
    /// Whole dwellings as play began (TOWN_GENERATION §4.2, §15 E1):
    /// recorded once by `economy::record_baselines`, never changed after.
    /// The town layout reserves land over it; no system reads it.
    pub baseline_stock: u32,
}

/// Households `adults` form when a year's `rent` is set against a
/// middle-class income: fewer (doubling up) as the burden rises.
pub fn households(adults: f64, rent: f64, middle_income: f64, p: &HousingParams) -> f64 {
    let burden = (rent * 12.0 / middle_income.max(1.0)).max(1e-3);
    let ratio = burden / p.reference_burden.max(1e-3);
    let per = p.adults_per_household * pow(ratio, p.headship_elasticity);
    let [lo, hi] = p.adults_per_household_range;
    adults / clamp(per, lo, hi.max(lo)).max(0.1)
}

/// Prices, rent and build cost from geography; stock and households follow
/// in `settle_start` once incomes exist.
pub fn init(mu: &Municipality, adults: f64, e: &EconomyParams, wealth: f64) -> Housing {
    let p = &e.housing;
    let class_factor = match mu.class {
        SettlementClass::Metro => 1.7,
        SettlementClass::City => 1.25,
        SettlementClass::Town => 0.95,
        SettlementClass::Rural => 0.7,
    };
    let price = e.base_dwelling_price * class_factor * (1.0 + 0.3 * mu.geo.scenic) * wealth;
    let mut h = Housing {
        rent: price * p.rent_yield / 12.0,
        price,
        // Building is mildly profitable at the starting equilibrium.
        build_cost: price * p.build_cost_share,
        // Recorded as play begins, after the history burn-in.
        baseline_stock: 0,
        ..Housing::default()
    };
    // Until incomes are known, households form at the reference rate.
    let per = p.adults_per_household.max(0.1);
    size_stock(&mut h, adults / per, e);
    h
}

/// The starting equilibrium, once incomes are known: households at the
/// starting rent burden, a stock at the target vacancy, and baseline starts
/// that replace demolitions and house new households.
pub fn settle_start(h: &mut Housing, adults: f64, middle_income: f64, e: &EconomyParams) {
    let n = households(adults, h.rent, middle_income, &e.housing);
    size_stock(h, n, e);
}

fn size_stock(h: &mut Housing, households: f64, e: &EconomyParams) {
    let p = &e.housing;
    h.households = households;
    h.stock = households / (1.0 - e.target_vacancy);
    h.vacancy = e.target_vacancy;
    h.base_starts = h.stock * (p.demolition_rate + p.base_growth) / 12.0;
    h.last_starts = h.base_starts;
    h.pipeline = vec![h.base_starts; e.construction_lag_months as usize];
}

pub fn starts_index(h: &Housing) -> f64 {
    h.last_starts / h.base_starts.max(1e-9)
}

pub fn step(
    h: &mut Housing,
    policy_rate: f64,
    inflation: f64,
    price_level: f64,
    e: &EconomyParams,
    public_units_per_year: f64,
    starts_multiplier: f64,
) {
    let p = &e.housing;
    // Completions arrive; new starts join the back of the pipeline.
    if !h.pipeline.is_empty() {
        h.stock += h.pipeline.remove(0);
    }
    h.stock *= 1.0 - p.demolition_rate / 12.0;
    h.vacancy = clamp(
        1.0 - h.households / h.stock.max(1.0),
        VACANCY_BOUNDS.0,
        VACANCY_BOUNDS.1,
    );
    let gap = e.target_vacancy - h.vacancy;
    h.rent *= 1.0 + gap * p.rent_response + inflation / 12.0;
    // Price: the rent capitalised at the user cost of housing, against the
    // neutral rate's (at which a year's rent is `rent_yield` of the price).
    // Expected prices keep up with inflation; the gap closes slowly.
    let real_rate = (policy_rate - inflation).max(-0.02);
    let user_cost = |r: f64| (r + p.risk_premium).max(0.02);
    let multiple = user_cost(e.neutral_real_rate) / user_cost(real_rate) / p.rent_yield;
    let target_price = h.rent * 12.0 * multiple;
    h.price *= 1.0 + inflation / 12.0;
    h.price += (target_price - h.price) * p.price_adjustment;
    // Builders: price over build cost (1 at the start) and vacancy.
    let q = h.price * p.build_cost_share / (h.build_cost * price_level).max(1.0);
    let neutral_rate = e.neutral_real_rate + e.inflation_target;
    let rate_factor = clamp(1.0 - (policy_rate - neutral_rate) * 8.0, 0.3, 1.6);
    let signal = 1.0 + (q - 1.0) * p.starts_price_response + gap * p.starts_vacancy_response;
    // Zoning scales private building; public programmes add homes directly.
    let starts = h.base_starts * clamp(signal, 0.0, p.starts_max) * rate_factor * starts_multiplier
        + public_units_per_year / 12.0;
    h.pipeline.push(starts);
    h.last_starts = starts;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> EconomyParams {
        let src = mandate_content::bundle::read_dir_sources(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        )
        .unwrap();
        mandate_content::load(&src).unwrap().0.economy
    }

    fn start(e: &EconomyParams, burden: f64) -> (Housing, f64) {
        let middle_income = 45_000.0;
        let mut h = Housing {
            rent: burden * middle_income / 12.0,
            ..Housing::default()
        };
        h.price = h.rent * 12.0 / e.housing.rent_yield;
        h.build_cost = h.price * e.housing.build_cost_share;
        settle_start(&mut h, 10_000.0, middle_income, e);
        (h, middle_income)
    }

    /// At the target vacancy, neutral rates and steady incomes, the market
    /// stays put: vacancy at target, starts at the baseline, real rent flat.
    #[test]
    fn the_start_is_an_equilibrium() {
        let e = params();
        let neutral = e.neutral_real_rate + e.inflation_target;
        for burden in [0.12, 0.19, 0.3] {
            let (mut h, income) = start(&e, burden);
            // Adults grow as the baseline building assumes.
            let mut adults = 10_000.0;
            let r0 = h.rent;
            let mut pl = 1.0;
            for _ in 0..240 {
                adults += 10_000.0 * e.housing.base_growth / 12.0;
                pl *= 1.0 + e.inflation_target / 12.0;
                step(&mut h, neutral, e.inflation_target, pl, &e, 0.0, 1.0);
                h.households = households(adults, h.rent, income * pl, &e.housing);
                assert!(
                    (starts_index(&h) - 1.0).abs() < 0.02,
                    "{}",
                    starts_index(&h)
                );
            }
            assert!(
                (h.vacancy - e.target_vacancy).abs() < 0.002,
                "{}",
                h.vacancy
            );
            assert!(
                (h.rent / pl / r0 - 1.0).abs() < 0.03,
                "{}",
                h.rent / pl / r0
            );
        }
    }

    /// A town that keeps gaining adults faster than it builds: rent rises
    /// but is bounded by incomes (households double up), vacancy is a real
    /// quantity, and builders answer the shortage.
    #[test]
    fn a_shortage_is_bounded_by_incomes() {
        let e = params();
        let neutral = e.neutral_real_rate + e.inflation_target;
        let (mut h, income) = start(&e, 0.19);
        let mut adults = 10_000.0;
        for _ in 0..360 {
            adults *= 1.0 + 0.02 / 12.0;
            step(&mut h, neutral, 0.0, 1.0, &e, 0.0, 1.0);
            h.households = households(adults, h.rent, income, &e.housing);
        }
        let burden = h.rent * 12.0 / income;
        assert!(burden < 0.5, "burden {burden}");
        assert!(starts_index(&h) > 1.5, "starts {}", starts_index(&h));
        assert!(h.vacancy > -0.02, "vacancy {}", h.vacancy);
    }

    /// Zoning scales private building once builders are building.
    #[test]
    fn zoning_scales_private_starts() {
        let e = params();
        let neutral = e.neutral_real_rate + e.inflation_target;
        let (mut a, _) = start(&e, 0.19);
        let mut b = a.clone();
        step(&mut a, neutral, 0.0, 1.0, &e, 0.0, 1.0);
        step(&mut b, neutral, 0.0, 1.0, &e, 0.0, 1.5);
        assert!((b.last_starts / a.last_starts - 1.5).abs() < 1e-9);
    }
}
