//! Turn resolution order, cadence scheduling, and domain write isolation
//! (OVERVIEW §4.2–4.3).
//!
//! Each system receives `&mut` access to **its own domain only** plus a
//! read-only view of everything else (`Reads`). Cross-domain changes travel
//! as typed `Effect`s applied by the owning domain. The borrow checker
//! therefore enforces the architecture rule.

use crate::economy::EconomyState;
use crate::environment::EnvironmentState;
use crate::events::{EventId, EventLog};
use crate::finance::FinanceState;
use crate::justice::JusticeState;
use crate::media::MediaState;
use crate::narrative::NarrativeState;
use crate::player::PlayerState;
use crate::policy::PolicyState;
use crate::politics::PoliticsState;
use crate::population::PopulationState;
use crate::state::{Calendar, State, WorldMeta};
use crate::world::World;
use mandate_content::{ContentBundle, ContentKey};
use mandate_core::{Cadence, RngStreams, SimDate};
use serde::{Deserialize, Serialize};

/// The 13 resolution phases, in order. Changing this order is a recorded
/// architecture change and requires a determinism test update.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Phase {
    Commands,
    NpcActions,
    Institutions,
    PolicyImplementation,
    Environment,
    Economy,
    Population,
    Information,
    Opinion,
    Elections,
    Characters,
    Director,
    Commit,
}

/// Cross-domain change request, applied by the owning system (OVERVIEW §4.3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// Destroy a share of a municipality's housing stock (economy applies).
    HousingDamage {
        municipality: u32,
        fraction: f64,
        cause: Option<EventId>,
    },
    /// Temporarily scale a sector's demand in a municipality (economy applies).
    SectorShock {
        municipality: u32,
        /// Content key of the sector.
        sector: ContentKey,
        factor: f64,
        months: u16,
        cause: Option<EventId>,
    },
    /// Residents leave a municipality after a disaster (population applies).
    Displacement {
        municipality: u32,
        fraction: f64,
        cause: Option<EventId>,
    },
    /// A politician remembers what another did (politics applies, DEC-031).
    Memory {
        holder: crate::politics::PoliticianId,
        actor: crate::politics::PoliticianId,
        kind: crate::characters::MemoryKind,
        subject: Option<crate::characters::Subject>,
    },
    /// A story about the player ran (DEC-034 perception): population applies
    /// it to the cohorts the outlet reaches.
    Perceive { story: u32 },
    /// A story moved a politician's scandal valence (politics applies).
    Valence {
        politician: crate::politics::PoliticianId,
        delta: f32,
    },
    /// Anyone remembers what someone did (politics applies, DEC-031).
    Remember {
        holder: crate::characters::CharacterId,
        actor: crate::characters::CharacterId,
        kind: crate::characters::MemoryKind,
    },
    /// The player's tracks move (the player applies): Standing, viability,
    /// money, and a Legal Jeopardy floor while a case is open.
    PlayerTrack {
        standing: f32,
        viability: f32,
        money: f64,
        legal_floor: Option<f32>,
    },
    /// A conviction removes someone from office (politics applies; the seat
    /// is filled by their party). `barred`: they cannot stand again yet.
    Removal {
        politician: crate::politics::PoliticianId,
        barred: bool,
    },
    /// A sentence lands (the player applies to the player; politics to NPCs).
    Sentenced {
        politician: crate::politics::PoliticianId,
        case: u32,
        sentence: crate::justice::Sentence,
    },
    /// A disaster's share in one neighbourhood (DEC-090 D8; population
    /// applies): `relative` is how hard it was hit against its town's
    /// average (1 = average); `leave` the share of its mobile adults who
    /// leave the town for good; `temporary` the share of its adults who stay
    /// in less-hit neighbourhoods (or the next town) until they can return.
    HoodDisaster {
        neighborhood: u32,
        relative: f64,
        leave: f64,
        temporary: f64,
        cause: Option<EventId>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct EffectQueue {
    pending: Vec<Effect>,
}

impl EffectQueue {
    pub fn push(&mut self, e: Effect) {
        self.pending.push(e);
    }

    /// Remove and return effects matching a predicate, preserving order.
    pub fn take(&mut self, pred: impl Fn(&Effect) -> bool) -> Vec<Effect> {
        let (taken, keep): (Vec<_>, Vec<_>) = self.pending.drain(..).partition(|e| pred(e));
        self.pending = keep;
        taken
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// Read-only view of the state, minus the domain being written.
pub struct Reads<'a> {
    pub content: &'a ContentBundle,
    pub date: SimDate,
    pub meta: &'a WorldMeta,
    pub calendar: &'a Calendar,
    pub world: &'a World,
    population: Option<&'a PopulationState>,
    economy: Option<&'a EconomyState>,
    finance: Option<&'a FinanceState>,
    environment: Option<&'a EnvironmentState>,
    politics: Option<&'a PoliticsState>,
    policy: Option<&'a PolicyState>,
    narrative: Option<&'a NarrativeState>,
    media: Option<&'a MediaState>,
    justice: Option<&'a JusticeState>,
    /// The player, if one exists (lab runs may have none).
    pub player: Option<&'a PlayerState>,
}

// The scheduler always provides every domain except the one being written;
// a system asking for its own domain through `Reads` is a programming error.
#[allow(clippy::expect_used)]
impl<'a> Reads<'a> {
    /// Read-only access to every domain of a whole state, for views and tools
    /// outside the scheduler (nothing is written through it).
    pub fn of_state(state: &'a State, content: &'a ContentBundle) -> Self {
        Reads {
            content,
            date: state.calendar.date,
            meta: &state.meta,
            calendar: &state.calendar,
            world: &state.world,
            population: Some(&state.population),
            economy: Some(&state.economy),
            finance: Some(&state.finance),
            environment: Some(&state.environment),
            politics: Some(&state.politics),
            policy: Some(&state.policy),
            narrative: Some(&state.narrative),
            media: Some(&state.media),
            justice: Some(&state.justice),
            player: state.player.as_ref(),
        }
    }

    pub fn population(&self) -> &'a PopulationState {
        self.population
            .expect("population is not readable from its own systems")
    }
    pub fn economy(&self) -> &'a EconomyState {
        self.economy
            .expect("economy is not readable from its own systems")
    }
    pub fn finance(&self) -> &'a FinanceState {
        self.finance
            .expect("finance is not readable from its own systems")
    }
    pub fn environment(&self) -> &'a EnvironmentState {
        self.environment
            .expect("environment is not readable from its own systems")
    }
    pub fn politics(&self) -> &'a PoliticsState {
        self.politics
            .expect("politics is not readable from its own systems")
    }
    pub fn policy(&self) -> &'a PolicyState {
        self.policy
            .expect("policy is not readable from its own systems")
    }
    pub fn narrative(&self) -> &'a NarrativeState {
        self.narrative
            .expect("narrative is not readable from its own systems")
    }
    pub fn media(&self) -> &'a MediaState {
        self.media
            .expect("media is not readable from its own systems")
    }
    pub fn justice(&self) -> &'a JusticeState {
        self.justice
            .expect("justice is not readable from its own systems")
    }
}

/// Shared outputs every system may use.
pub struct Outs<'a> {
    pub rng: &'a mut RngStreams,
    pub events: &'a mut EventLog,
    pub effects: &'a mut EffectQueue,
}

/// A system function bound to the domain it writes.
#[derive(Clone, Copy)]
pub enum SystemFn {
    Population(fn(&mut PopulationState, &Reads<'_>, &mut Outs<'_>)),
    Economy(fn(&mut EconomyState, &Reads<'_>, &mut Outs<'_>)),
    Finance(fn(&mut FinanceState, &Reads<'_>, &mut Outs<'_>)),
    Environment(fn(&mut EnvironmentState, &Reads<'_>, &mut Outs<'_>)),
    Politics(fn(&mut PoliticsState, &Reads<'_>, &mut Outs<'_>)),
    Policy(fn(&mut PolicyState, &Reads<'_>, &mut Outs<'_>)),
    /// Runs only when a player exists.
    Player(fn(&mut PlayerState, &Reads<'_>, &mut Outs<'_>)),
    /// The narrative director (DEC-033): writes only its own domain.
    Narrative(fn(&mut NarrativeState, &Reads<'_>, &mut Outs<'_>)),
    /// Media and information (DEC-034).
    Media(fn(&mut MediaState, &Reads<'_>, &mut Outs<'_>)),
    /// The legal path (DEC-006).
    Justice(fn(&mut JusticeState, &Reads<'_>, &mut Outs<'_>)),
    /// Writes nothing but the event log.
    Log(fn(&Reads<'_>, &mut Outs<'_>)),
}

#[derive(Clone, Copy)]
pub struct SystemDef {
    pub name: &'static str,
    pub phase: Phase,
    pub cadence: Cadence,
    pub run: SystemFn,
}

/// Systems sorted by phase, keeping registration order within a phase.
pub struct Schedule {
    systems: Vec<SystemDef>,
}

impl Schedule {
    pub fn new(mut systems: Vec<SystemDef>) -> Self {
        systems.sort_by_key(|s| s.phase); // stable: keeps registration order
        Schedule { systems }
    }

    /// Advance the calendar day by day, running each system whose cadence hits.
    /// `on_day` is called after each day (progress reporting).
    pub fn advance(
        &self,
        state: &mut State,
        content: &ContentBundle,
        days: u32,
        on_day: &mut dyn FnMut(u32),
    ) {
        let _ = self.advance_with(state, content, days, Vec::new(), on_day);
    }

    /// Advance with effects pre-seeded into the queue (lab commands).
    /// Returns effects still queued at the end (for a following advance).
    pub fn advance_with(
        &self,
        state: &mut State,
        content: &ContentBundle,
        days: u32,
        seed_effects: Vec<Effect>,
        on_day: &mut dyn FnMut(u32),
    ) -> Vec<Effect> {
        let mut effects = EffectQueue {
            pending: seed_effects,
        };
        for d in 0..days {
            let date = state.calendar.date.add_days(1);
            state.calendar.date = date;
            for sys in &self.systems {
                if sys.cadence.hits(date) {
                    run_one(sys.run, state, content, date, &mut effects);
                }
            }
            on_day(d + 1);
        }
        effects.pending
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.systems.iter().map(|s| s.name).collect()
    }
}

fn run_one(
    f: SystemFn,
    state: &mut State,
    content: &ContentBundle,
    date: SimDate,
    effects: &mut EffectQueue,
) {
    let State {
        meta,
        calendar,
        rng,
        world,
        events,
        population,
        economy,
        finance,
        environment,
        politics,
        policy,
        player,
        narrative,
        media,
        justice,
    } = state;
    let mut outs = Outs {
        rng,
        events,
        effects,
    };
    // Each arm leaves its own domain out of `Reads` statically, so the borrow
    // checker proves a system cannot read-and-write its own state through two paths.
    macro_rules! reads {
        ($pop:expr, $eco:expr, $fin:expr, $env:expr, $pol:expr, $poly:expr, $ply:expr) => {
            reads!(
                $pop,
                $eco,
                $fin,
                $env,
                $pol,
                $poly,
                $ply,
                Some(&*narrative),
                Some(&*media),
                Some(&*justice)
            )
        };
        ($pop:expr, $eco:expr, $fin:expr, $env:expr, $pol:expr, $poly:expr, $ply:expr, $narr:expr, $med:expr, $jus:expr) => {
            Reads {
                content,
                date,
                meta,
                calendar,
                world,
                population: $pop,
                economy: $eco,
                finance: $fin,
                environment: $env,
                politics: $pol,
                policy: $poly,
                narrative: $narr,
                media: $med,
                justice: $jus,
                player: $ply,
            }
        };
    }
    match f {
        SystemFn::Population(g) => {
            let r = reads!(
                None,
                Some(&*economy),
                Some(&*finance),
                Some(&*environment),
                Some(&*politics),
                Some(&*policy),
                player.as_ref()
            );
            g(population, &r, &mut outs)
        }
        SystemFn::Economy(g) => {
            let r = reads!(
                Some(&*population),
                None,
                Some(&*finance),
                Some(&*environment),
                Some(&*politics),
                Some(&*policy),
                player.as_ref()
            );
            g(economy, &r, &mut outs)
        }
        SystemFn::Finance(g) => {
            let r = reads!(
                Some(&*population),
                Some(&*economy),
                None,
                Some(&*environment),
                Some(&*politics),
                Some(&*policy),
                player.as_ref()
            );
            g(finance, &r, &mut outs)
        }
        SystemFn::Environment(g) => {
            let r = reads!(
                Some(&*population),
                Some(&*economy),
                Some(&*finance),
                None,
                Some(&*politics),
                Some(&*policy),
                player.as_ref()
            );
            g(environment, &r, &mut outs)
        }
        SystemFn::Politics(g) => {
            let r = reads!(
                Some(&*population),
                Some(&*economy),
                Some(&*finance),
                Some(&*environment),
                None,
                Some(&*policy),
                player.as_ref()
            );
            g(politics, &r, &mut outs)
        }
        SystemFn::Policy(g) => {
            let r = reads!(
                Some(&*population),
                Some(&*economy),
                Some(&*finance),
                Some(&*environment),
                Some(&*politics),
                None,
                player.as_ref()
            );
            g(policy, &r, &mut outs)
        }
        SystemFn::Player(g) => {
            if let Some(p) = player.as_mut() {
                let r = reads!(
                    Some(&*population),
                    Some(&*economy),
                    Some(&*finance),
                    Some(&*environment),
                    Some(&*politics),
                    Some(&*policy),
                    None
                );
                g(p, &r, &mut outs)
            }
        }
        SystemFn::Narrative(g) => {
            let r = reads!(
                Some(&*population),
                Some(&*economy),
                Some(&*finance),
                Some(&*environment),
                Some(&*politics),
                Some(&*policy),
                player.as_ref(),
                None,
                Some(&*media),
                Some(&*justice)
            );
            g(narrative, &r, &mut outs)
        }
        SystemFn::Media(g) => {
            let r = reads!(
                Some(&*population),
                Some(&*economy),
                Some(&*finance),
                Some(&*environment),
                Some(&*politics),
                Some(&*policy),
                player.as_ref(),
                Some(&*narrative),
                None,
                Some(&*justice)
            );
            g(media, &r, &mut outs)
        }
        SystemFn::Justice(g) => {
            let r = reads!(
                Some(&*population),
                Some(&*economy),
                Some(&*finance),
                Some(&*environment),
                Some(&*politics),
                Some(&*policy),
                player.as_ref(),
                Some(&*narrative),
                Some(&*media),
                None
            );
            g(justice, &r, &mut outs)
        }
        SystemFn::Log(g) => {
            let r = reads!(
                Some(&*population),
                Some(&*economy),
                Some(&*finance),
                Some(&*environment),
                Some(&*politics),
                Some(&*policy),
                player.as_ref()
            );
            g(&r, &mut outs)
        }
    }
}
