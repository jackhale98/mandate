//! System registry (phase, cadence, owning domain). Order within a phase is
//! registration order (OVERVIEW §4.2).

use crate::events::EventKind;
use crate::scheduler::{Outs, Phase, Reads, SystemDef, SystemFn};
use crate::{economy, environment, finance, population};
use mandate_core::Cadence;

pub fn default_systems() -> Vec<SystemDef> {
    use Cadence::*;
    use Phase as P;
    vec![
        SystemDef {
            name: "environment.drift",
            phase: P::Environment,
            cadence: Monthly,
            run: SystemFn::Environment(environment::drift),
        },
        SystemDef {
            name: "environment.hazards",
            phase: P::Environment,
            cadence: Monthly,
            run: SystemFn::Environment(environment::hazards),
        },
        SystemDef {
            name: "economy.global",
            phase: P::Economy,
            cadence: Monthly,
            run: SystemFn::Economy(economy::global),
        },
        SystemDef {
            name: "economy.markets",
            phase: P::Economy,
            cadence: Monthly,
            run: SystemFn::Economy(economy::markets),
        },
        SystemDef {
            name: "economy.national",
            phase: P::Economy,
            cadence: Monthly,
            run: SystemFn::Economy(economy::national),
        },
        SystemDef {
            name: "finance.accrue",
            phase: P::Economy,
            cadence: Monthly,
            run: SystemFn::Finance(finance::accrue),
        },
        SystemDef {
            name: "finance.budget_year",
            phase: P::Economy,
            cadence: Yearly,
            run: SystemFn::Finance(finance::new_budget_year),
        },
        SystemDef {
            name: "population.migration",
            phase: P::Population,
            cadence: Monthly,
            run: SystemFn::Population(population::migration),
        },
        SystemDef {
            name: "population.demography",
            phase: P::Population,
            cadence: Yearly,
            run: SystemFn::Population(population::demography),
        },
        SystemDef {
            name: "calendar.year",
            phase: P::Commit,
            cadence: Yearly,
            run: SystemFn::Log(year_began),
        },
    ]
}

fn year_began(r: &Reads<'_>, out: &mut Outs<'_>) {
    out.events.record(
        r.date,
        EventKind::YearBegan {
            year: r.date.calendar().year,
        },
        vec![],
    );
}
