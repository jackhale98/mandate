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
//! The population domain owns the result (`PopulationState::town_concerns`);
//! the indicators are read-only views of other domains, reusable by later
//! systems (town character, situational policies).

use crate::population::PopulationState;
use crate::scheduler::{Outs, Reads};
use mandate_content::{ContentBundle, Indicator};
use serde::{Deserialize, Serialize};

/// One town's concerns: a level per concern (registry order) and a salience
/// per attitude (registry order, mean 1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TownConcerns {
    pub levels: Vec<f32>,
    pub salience: Vec<f32>,
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

/// Levels and salience for one town from its indicators.
pub fn compute(r: &Reads<'_>, m: usize) -> TownConcerns {
    let content = r.content;
    let levels: Vec<f64> = content
        .concerns
        .iter()
        .map(|c| c.level.of(indicator(r, m, c.indicator)))
        .collect();
    TownConcerns {
        salience: salience_from(content, &levels),
        levels: levels.iter().map(|l| *l as f32).collect(),
    }
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

/// Every town's concerns, from the current state of the other domains.
pub fn compute_all(r: &Reads<'_>) -> Vec<TownConcerns> {
    (0..r.world.municipalities.len())
        .map(|m| compute(r, m))
        .collect()
}

/// Population domain, Opinion phase, monthly: recompute every town's
/// concern levels and issue salience.
pub fn refresh(pop: &mut PopulationState, r: &Reads<'_>, _out: &mut Outs<'_>) {
    pop.town_concerns = compute_all(r);
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

/// A town's concerns, strongest first: (registry index, level). Ties keep
/// registry order.
pub fn top_concerns(pop: &PopulationState, m: usize) -> Vec<(usize, f64)> {
    let mut v: Vec<(usize, f64)> = pop
        .town_concerns
        .get(m)
        .map(|t| t.levels.iter().map(|l| *l as f64).enumerate().collect())
        .unwrap_or_default();
    v.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// Concerns as felt in some neighbourhoods of a town, strongest first:
/// the town's level × (1 + ½ × how far residents lean the way worried voters
/// do), in [0, 1.5 × level]. For display and NPC talk, not the vote model.
pub fn felt_concerns(
    pop: &PopulationState,
    content: &ContentBundle,
    m: usize,
    hoods: &[usize],
) -> Vec<(usize, f64)> {
    let mean = crate::politics::mean_attitudes(pop, hoods);
    let mut v: Vec<(usize, f64)> = top_concerns(pop, m)
        .into_iter()
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
