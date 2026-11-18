//! System registry. M0 ships foundation systems that exercise every engine
//! mechanism (cadence, RNG streams, effects, event log); later milestones
//! replace them with the real domain systems.

use crate::events::EventKind;
use crate::scheduler::{Effect, Phase, SystemCtx, SystemDef};
use mandate_core::{Cadence, Stream};

pub fn default_systems() -> Vec<SystemDef> {
    vec![
        SystemDef {
            name: "calendar.year",
            phase: Phase::Commit,
            cadence: Cadence::Yearly,
            run: year_began,
        },
        SystemDef {
            name: "environment.shocks",
            phase: Phase::Environment,
            cadence: Cadence::Monthly,
            run: regional_shocks,
        },
        SystemDef {
            name: "population.growth",
            phase: Phase::Population,
            cadence: Cadence::Monthly,
            run: population_growth,
        },
    ]
}

fn year_began(ctx: &mut SystemCtx<'_>) {
    let year = ctx.date.calendar().year;
    ctx.state
        .events
        .record(ctx.date, EventKind::YearBegan { year }, vec![]);
}

/// Occasional regional shocks; emitted as effects for the population system.
fn regional_shocks(ctx: &mut SystemCtx<'_>) {
    let n = ctx.state.world.regions.len() as u64;
    if n == 0 {
        return;
    }
    let rng = ctx.state.rng.get(Stream::Environment);
    if rng.chance(0.05) {
        let region = rng.below(n) as u32;
        let pop = ctx.state.world.regions[region as usize].population as f64;
        let delta = -(pop * rng.range_f64(0.002, 0.01)) as i64;
        let ev = ctx.state.events.record(
            ctx.date,
            EventKind::RegionalShock {
                region,
                population_delta: delta,
            },
            vec![],
        );
        ctx.effects.push(Effect::PopulationShock {
            region,
            delta,
            cause: Some(ev),
        });
    }
}

/// Baseline monthly drift plus queued shocks. Populations never go negative.
fn population_growth(ctx: &mut SystemCtx<'_>) {
    let shocks = ctx
        .effects
        .take(|e| matches!(e, Effect::PopulationShock { .. }));
    for e in shocks {
        let Effect::PopulationShock { region, delta, .. } = e;
        if let Some(r) = ctx.state.world.regions.get_mut(region as usize) {
            r.population = (r.population as i64 + delta).max(0) as u64;
        }
    }
    let rng = ctx.state.rng.get(Stream::Population);
    for r in &mut ctx.state.world.regions {
        let rate = rng.normal(0.0006, 0.0004);
        let delta = (r.population as f64 * rate) as i64;
        r.population = (r.population as i64 + delta).max(0) as u64;
    }
}
