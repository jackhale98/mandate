//! System registry (phase, cadence, owning domain). Order within a phase is
//! registration order (OVERVIEW §4.2).

use crate::events::EventKind;
use crate::scheduler::{Outs, Phase, Reads, SystemDef, SystemFn};
use crate::spine::{Cause, Process};
use crate::{economy, environment, finance, player, policy, politics, population};
use mandate_core::Cadence;

pub fn default_systems() -> Vec<SystemDef> {
    use Cadence::*;
    use Phase as P;
    vec![
        // Staged arcs (DEC-109), first in the day: what a run asks of
        // another domain is applied by that domain's system the same day.
        SystemDef {
            name: "arcs.daily",
            phase: P::NpcActions,
            cadence: Daily,
            run: SystemFn::Arcs(crate::arcs::daily),
        },
        SystemDef {
            name: "arcs.weekly",
            phase: P::NpcActions,
            cadence: Weekly,
            run: SystemFn::Arcs(crate::arcs::weekly),
        },
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
        // Repairs other domains asked for (`Effect::Recovery`), before the
        // month's repairs.
        SystemDef {
            name: "environment.effects",
            phase: P::Environment,
            cadence: Daily,
            run: SystemFn::Environment(environment::effects),
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
        // Budget changes other domains asked for (`Effect::Budget`), before
        // the month's spending.
        SystemDef {
            name: "finance.effects",
            phase: P::Economy,
            cadence: Daily,
            run: SystemFn::Finance(finance::effects),
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
        // Upkeep (DEC-114): repairs finished and faults moved up, daily;
        // the repair queues paid, the streets worn and new faults, monthly,
        // after the month's budget and before the worries are read.
        SystemDef {
            name: "upkeep.daily",
            phase: P::Population,
            cadence: Daily,
            run: SystemFn::Upkeep(crate::upkeep::daily),
        },
        SystemDef {
            name: "upkeep.monthly",
            phase: P::Population,
            cadence: Monthly,
            run: SystemFn::Upkeep(crate::upkeep::monthly),
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
        // Decisions and asks other domains asked for (an arc's beats).
        SystemDef {
            name: "narrative.requests",
            phase: P::Director,
            cadence: Daily,
            run: SystemFn::Narrative(crate::narrative::offered::present),
        },
        SystemDef {
            name: "asks.requests",
            phase: P::Director,
            cadence: Daily,
            run: SystemFn::Asks(crate::asks::offers::requests),
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
        Cause::Process(Process::Calendar),
    );
}
