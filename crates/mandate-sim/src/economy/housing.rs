//! Municipal housing markets: stock, vacancy, rents, prices, construction.

use crate::world::{Municipality, SettlementClass};
use mandate_content::EconomyParams;
use mandate_core::math::clamp;
use serde::{Deserialize, Serialize};

pub const ADULTS_PER_HOUSEHOLD: f64 = 1.9;
/// Share of the stock demolished per year.
pub const DEMOLITION_RATE: f64 = 0.005;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Housing {
    pub stock: f64,
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

pub fn init(mu: &Municipality, adults: f64, e: &EconomyParams, wealth: f64) -> Housing {
    let households = adults / ADULTS_PER_HOUSEHOLD;
    let stock = households / (1.0 - e.target_vacancy);
    let class_factor = match mu.class {
        SettlementClass::Metro => 1.7,
        SettlementClass::City => 1.25,
        SettlementClass::Town => 0.95,
        SettlementClass::Rural => 0.7,
    };
    let price = e.base_dwelling_price * class_factor * (1.0 + 0.3 * mu.geo.scenic) * wealth;
    // Baseline building replaces demolitions and houses new households.
    let base_starts = stock * (DEMOLITION_RATE + 0.004) / 12.0;
    Housing {
        stock,
        households,
        vacancy: e.target_vacancy,
        rent: price * 0.045 / 12.0,
        price,
        pipeline: vec![base_starts; e.construction_lag_months as usize],
        base_starts,
        last_starts: base_starts,
        // Building is mildly profitable at the starting equilibrium.
        build_cost: price * 0.85,
        // Recorded as play begins, after the history burn-in.
        baseline_stock: 0,
    }
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
    // Completions arrive; new starts join the back of the pipeline.
    if !h.pipeline.is_empty() {
        h.stock += h.pipeline.remove(0);
    }
    h.stock *= 1.0 - DEMOLITION_RATE / 12.0;
    h.vacancy = clamp(1.0 - h.households / h.stock.max(1.0), -0.05, 0.4);
    let gap = e.target_vacancy - h.vacancy;
    h.rent *= 1.0 + gap * 0.12 + inflation / 12.0;
    let real_rate = (policy_rate - inflation).max(-0.02);
    let multiple = 1.0 / (real_rate + 0.035).max(0.02);
    let target_price = h.rent * 12.0 * multiple;
    h.price += (target_price - h.price) * 0.05;
    let profit = h.price / (h.build_cost * price_level).max(1.0);
    let neutral_rate = e.neutral_real_rate + e.inflation_target;
    let rate_factor = clamp(1.0 - (policy_rate - neutral_rate) * 8.0, 0.3, 1.6);
    // At the starting equilibrium (profit ≈ 1/0.85) starts equal the baseline.
    // Zoning scales private building; public programmes add homes directly.
    let starts = h.base_starts
        * clamp(1.0 + (profit - 1.0 / 0.85) * 4.0, 0.0, 3.0)
        * rate_factor
        * starts_multiplier
        + public_units_per_year / 12.0;
    h.pipeline.push(starts);
    h.last_starts = starts;
}
