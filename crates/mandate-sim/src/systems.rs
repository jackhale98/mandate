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
        let pop = ctx
            .state
            .world
            .region_population(crate::world::RegionId(region)) as f64;
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
    let world = &mut ctx.state.world;
    for e in shocks {
        let Effect::PopulationShock { region, delta, .. } = e;
        // Spread the shock across the region's neighbourhoods by population.
        let total = world
            .region_population(crate::world::RegionId(region))
            .max(1) as f64;
        let Some(r) = world.regions.get(region as usize) else {
            continue;
        };
        let hoods: Vec<usize> = r
            .municipalities
            .iter()
            .flat_map(|m| {
                world.municipalities[m.index()]
                    .neighborhoods
                    .iter()
                    .map(|n| n.index())
            })
            .collect();
        for h in hoods {
            let n = &mut world.neighborhoods[h];
            let share = (delta as f64 * n.population as f64 / total) as i64;
            n.population = (n.population as i64 + share).max(0) as u64;
        }
    }
    // Placeholder demography until the cohort model lands (M2).
    let rng = ctx.state.rng.get(Stream::Population);
    for n in &mut world.neighborhoods {
        let rate = rng.normal(0.0006, 0.0004);
        let delta = (n.population as f64 * rate) as i64;
        n.population = (n.population as i64 + delta).max(0) as u64;
    }
}
