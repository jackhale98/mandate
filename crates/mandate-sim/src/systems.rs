//! System registry (phase, cadence, owning domain). Order within a phase is
//! registration order (OVERVIEW §4.2).

use crate::events::EventKind;
use crate::scheduler::{Outs, Phase, Reads, SystemDef, SystemFn};
use crate::{economy, environment, finance, player, policy, politics, population};
use mandate_core::Cadence;

pub fn default_systems() -> Vec<SystemDef> {
    use Cadence::*;
    use Phase as P;
    vec![
        SystemDef {
            name: "policy.procedures",
            phase: P::Institutions,
            cadence: Weekly,
            run: SystemFn::Policy(policy::procedures),
        },
        SystemDef {
            name: "justice.cases",
            phase: P::Institutions,
            cadence: Weekly,
            run: SystemFn::Justice(crate::justice::weekly),
        },
        SystemDef {
            name: "media.weekly",
            phase: P::Information,
            cadence: Weekly,
            run: SystemFn::Media(crate::media::weekly),
        },
        SystemDef {
            name: "population.coverage",
            phase: P::Opinion,
            cadence: Daily,
            run: SystemFn::Population(crate::media::perceive),
        },
        SystemDef {
            name: "policy.implementation",
            phase: P::PolicyImplementation,
            cadence: Monthly,
            run: SystemFn::Policy(policy::implementation),
        },
        // Works you can see (DEC-099): approve, build, stall, age, decay.
        SystemDef {
            name: "policy.works",
            phase: P::PolicyImplementation,
            cadence: Monthly,
            run: SystemFn::Policy(policy::works::monthly),
        },
        SystemDef {
            name: "population.concerns",
            phase: P::Opinion,
            cadence: Monthly,
            run: SystemFn::Population(crate::concerns::refresh),
        },
        SystemDef {
            name: "population.effects",
            phase: P::Opinion,
            cadence: Daily,
            run: SystemFn::Population(population::effects),
        },
        SystemDef {
            name: "population.opinion",
            phase: P::Opinion,
            cadence: Monthly,
            run: SystemFn::Population(population::opinion),
        },
        SystemDef {
            name: "player.monthly",
            phase: P::Characters,
            cadence: Monthly,
            run: SystemFn::Player(player::monthly),
        },
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
            name: "politics.elections",
            phase: P::Elections,
            cadence: Daily,
            run: SystemFn::Politics(politics::elections::daily),
        },
        SystemDef {
            name: "politics.monthly",
            phase: P::Characters,
            cadence: Monthly,
            run: SystemFn::Politics(politics::elections::monthly),
        },
        SystemDef {
            name: "politics.yearly",
            phase: P::Characters,
            cadence: Yearly,
            run: SystemFn::Politics(politics::elections::yearly),
        },
        SystemDef {
            name: "characters.effects",
            phase: P::Characters,
            cadence: Daily,
            run: SystemFn::Politics(crate::characters::apply_effects),
        },
        SystemDef {
            name: "politics.media_effects",
            phase: P::Characters,
            cadence: Daily,
            run: SystemFn::Politics(crate::politics::media_effects),
        },
        SystemDef {
            name: "player.effects",
            phase: P::Characters,
            cadence: Daily,
            run: SystemFn::Player(player::effects),
        },
        SystemDef {
            name: "characters.monthly",
            phase: P::Characters,
            cadence: Monthly,
            run: SystemFn::Politics(crate::characters::monthly),
        },
        // Policy families (DEC-092): decks, lapses and new proposals, after
        // the town's character and crises are fresh.
        SystemDef {
            name: "policy.families",
            phase: P::Characters,
            cadence: Monthly,
            run: SystemFn::Policy(policy::families::monthly),
        },
        SystemDef {
            name: "narrative.fairness",
            phase: P::Director,
            cadence: Daily,
            run: SystemFn::Narrative(crate::narrative::daily),
        },
        SystemDef {
            name: "narrative.director",
            phase: P::Director,
            cadence: Weekly,
            run: SystemFn::Narrative(crate::narrative::weekly),
        },
        SystemDef {
            name: "asks.daily",
            phase: P::Director,
            cadence: Daily,
            run: SystemFn::Asks(crate::asks::daily),
        },
        SystemDef {
            name: "asks.weekly",
            phase: P::Director,
            cadence: Weekly,
            run: SystemFn::Asks(crate::asks::offers::weekly),
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
