//! Turn resolution order, cadence scheduling, and domain write isolation
//! (OVERVIEW §4.2–4.3).
//!
//! Each system receives `&mut` access to **its own domain only** plus a
//! read-only view of everything else (`Reads`). Cross-domain changes travel
//! as typed `Effect`s applied by the owning domain. The borrow checker
//! therefore enforces the architecture rule.

use crate::arcs::ArcsState;
use crate::asks::AskState;
use crate::economy::EconomyState;
use crate::environment::EnvironmentState;
use crate::events::EventLog;
use crate::finance::FinanceState;
use crate::justice::JusticeState;
use crate::media::MediaState;
use crate::narrative::NarrativeState;
use crate::player::PlayerState;
use crate::policy::PolicyState;
use crate::politics::PoliticsState;
use crate::population::PopulationState;
use crate::spine::Cause;
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

/// A cross-domain change, applied by the owning system (OVERVIEW §4.3). It
/// is always requested with its cause ([`Request`], the causal spine S1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// Destroy a share of a municipality's housing stock (economy applies).
    HousingDamage { municipality: u32, fraction: f64 },
    /// Temporarily scale a sector's demand in a municipality (economy applies).
    SectorShock {
        municipality: u32,
        /// Content key of the sector.
        sector: ContentKey,
        factor: f64,
        months: u16,
    },
    /// A place closes, or reopens, where it stands (DEC-090 D3): it keeps
    /// its id and its plot. The world is no system's domain, so the
    /// scheduler applies this as the day ends.
    PlaceStatus { place: u32, open: bool },
    /// Relief on one plant's margin for some months (the economy applies;
    /// DEC-110): how anything outside the economy pushes on a plant's
    /// review.
    PlantRelief {
        plant: u32,
        relief: f64,
        months: u16,
    },
    /// Residents leave a municipality after a disaster (population applies).
    Displacement { municipality: u32, fraction: f64 },
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
    /// The player learns something (the player applies to the knowledge
    /// ledger, DEC-091): a subject raised to a level, never lowered.
    Learn {
        subject: crate::player::knowledge::Subject,
        level: crate::player::knowledge::Level,
        source: crate::player::knowledge::Source,
    },
    /// A dated reading of a numeric subject the player already knows (the
    /// player applies; it raises the subject to `detailed`).
    Read {
        subject: crate::player::knowledge::Subject,
        reading: crate::player::knowledge::Reading,
    },
    /// A lead for the player's notebook (the player applies).
    Lead {
        lead: crate::player::knowledge::Lead,
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
    },
    /// Someone's opinion of someone else changes, remembered as `memory`
    /// (politics applies; scaled by personality like every opinion change).
    Opinion {
        holder: crate::characters::CharacterId,
        toward: crate::characters::CharacterId,
        delta: f64,
        memory: Option<crate::characters::MemoryKind>,
    },
    /// A favour (politics applies): `holder` owes `toward` one, or with
    /// `owed_to_player` the reverse.
    Favour {
        holder: crate::characters::CharacterId,
        toward: crate::characters::CharacterId,
        owed_to_player: bool,
    },
    /// One favour `debtor` owes `creditor` is paid off (politics applies;
    /// the creditor remembers it kept): a debt settled in another coin.
    Repay {
        debtor: crate::characters::CharacterId,
        creditor: crate::characters::CharacterId,
    },
    /// Approval of the player in an area (population applies), × room, by
    /// income class (empty: all) and by community (registry indices;
    /// empty: all).
    Approval {
        scope: crate::world::Area,
        delta: f64,
        classes: Vec<u8>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        communities: Vec<u16>,
    },
    /// Recognition of the player in an area (population applies): a gain
    /// is × the room left, a loss is taken whole; by income class and
    /// community as `Approval` (DEC-112: being seen is being known).
    Awareness {
        scope: crate::world::Area,
        delta: f64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        classes: Vec<u8>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        communities: Vec<u16>,
    },
    /// A hidden fact was aired in public (DEC-112): the media open it as a
    /// story by their own rules, as a leak does. `by`: who said it.
    Surface {
        hidden: u32,
        by: Option<crate::characters::CharacterId>,
    },
    /// A campaign promise made in front of a room (DEC-112): the asks
    /// make it, with its deadline, its keep and its break. The room's own
    /// lean is its persuasion.
    Promise {
        template: ContentKey,
        answer: Option<ContentKey>,
    },
    /// A politician's record of visible work (politics applies).
    Record {
        politician: crate::politics::PoliticianId,
        delta: f64,
    },
    /// Damaged homes in a neighbourhood are repaired `factor` times as
    /// fast for `months` months (the environment applies; DEC-109).
    Recovery {
        neighborhood: u32,
        factor: f64,
        months: u16,
    },
    /// A change voted into a jurisdiction's next budget: one line of the
    /// discretionary budget (an index into `finance::Category::ALL`), or
    /// every line, × `factor` when its budget year turns (finance applies,
    /// to the tier's own treasury; DEC-109, DEC-098).
    Budget {
        at: crate::world::Jurisdiction,
        category: Option<usize>,
        factor: f64,
    },
    /// An arc's ask beat (DEC-109): the asks offer this template from the
    /// run's cast person, if they are free to ask and it can be kept.
    OfferAsk {
        template: ContentKey,
        asker: crate::characters::CharacterId,
        cast: Vec<crate::narrative::CastEntry>,
        arc: crate::arcs::ArcRef,
    },
}

/// An effect request: the change, and what led to it (the causal spine,
/// S1). Nothing crosses a domain without naming its cause.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub effect: Effect,
    pub cause: Cause,
}

#[derive(Clone, Debug, Default)]
pub struct EffectQueue {
    pending: Vec<Request>,
}

impl EffectQueue {
    /// Request an effect, naming what led to it.
    pub fn push(&mut self, effect: Effect, cause: Cause) {
        self.pending.push(Request { effect, cause });
    }

    pub fn push_request(&mut self, r: Request) {
        self.pending.push(r);
    }

    /// Remove and return the requests whose effect matches a predicate,
    /// preserving order.
    pub fn take(&mut self, pred: impl Fn(&Effect) -> bool) -> Vec<Request> {
        let (taken, keep): (Vec<_>, Vec<_>) = self.pending.drain(..).partition(|r| pred(&r.effect));
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
    asks: Option<&'a AskState>,
    arcs: Option<&'a ArcsState>,
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
            asks: Some(&state.asks),
            arcs: Some(&state.arcs),
            player: state.player.as_ref(),
        }
    }

    /// The same reads with the policy domain readable (the policy families
    /// system writes only `policy.families`, taken out of the domain while
    /// it runs, and reads the rest of the domain through this).
    pub fn with_policy<'b>(&self, policy: &'b PolicyState) -> Reads<'b>
    where
        'a: 'b,
    {
        Reads {
            policy: Some(policy),
            player: self.player,
            ..*self
        }
    }

    /// The population, unless it is the domain being written (its own
    /// systems pass it in themselves).
    pub fn population_opt(&self) -> Option<&'a PopulationState> {
        self.population
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
    pub fn asks(&self) -> &'a AskState {
        self.asks
            .expect("asks are not readable from their own systems")
    }
    /// The arcs domain: `None` inside its own systems (a run's conditions
    /// read the run itself, never the domain).
    pub fn arcs(&self) -> Option<&'a ArcsState> {
        self.arcs
    }
}

/// Shared outputs every system may use.
pub struct Outs<'a> {
    pub rng: &'a mut RngStreams,
    pub events: &'a mut EventLog,
    pub effects: &'a mut EffectQueue,
}

impl Outs<'_> {
    /// Take the effect requests an applier owns (those whose effect matches),
    /// in order, counting them for the spine's audit.
    pub fn take(&mut self, pred: impl Fn(&Effect) -> bool) -> Vec<Request> {
        let taken = self.effects.take(pred);
        for r in &taken {
            self.events.note_effect(&r.cause);
        }
        taken
    }
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
    /// Asks and promises (DEC-097 §2).
    Asks(fn(&mut AskState, &Reads<'_>, &mut Outs<'_>)),
    /// Staged arcs (DEC-109).
    Arcs(fn(&mut ArcsState, &Reads<'_>, &mut Outs<'_>)),
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
        seed_effects: Vec<Request>,
        on_day: &mut dyn FnMut(u32),
    ) -> Vec<Request> {
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
            apply_world(state, &mut effects, date);
            on_day(d + 1);
        }
        effects.pending
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.systems.iter().map(|s| s.name).collect()
    }
}

/// The world's own effects (DEC-090 D3): places close and reopen where
/// they stand. No system writes the world, so the requests are applied
/// here, as the day ends, in the order they were made.
fn apply_world(state: &mut State, effects: &mut EffectQueue, date: SimDate) {
    if !effects
        .pending
        .iter()
        .any(|r| matches!(r.effect, Effect::PlaceStatus { .. }))
    {
        return;
    }
    for r in effects.take(|e| matches!(e, Effect::PlaceStatus { .. })) {
        state.events.note_effect(&r.cause);
        if let Effect::PlaceStatus { place, open } = r.effect {
            let id = crate::world::PlaceId(place);
            if open {
                state.world.reopen_place(id);
            } else {
                state.world.close_place(id, date);
            }
        }
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
        asks,
        // The act domain is engine turn code, not a scheduled system.
        act: _,
        arcs,
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
            reads!(
                $pop,
                $eco,
                $fin,
                $env,
                $pol,
                $poly,
                $ply,
                $narr,
                $med,
                $jus,
                Some(&*asks)
            )
        };
        ($pop:expr, $eco:expr, $fin:expr, $env:expr, $pol:expr, $poly:expr, $ply:expr, $narr:expr, $med:expr, $jus:expr, $asks:expr) => {
            reads!(
                $pop,
                $eco,
                $fin,
                $env,
                $pol,
                $poly,
                $ply,
                $narr,
                $med,
                $jus,
                $asks,
                Some(&*arcs)
            )
        };
        ($pop:expr, $eco:expr, $fin:expr, $env:expr, $pol:expr, $poly:expr, $ply:expr, $narr:expr, $med:expr, $jus:expr, $asks:expr, $arcs:expr) => {
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
                asks: $asks,
                arcs: $arcs,
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
        SystemFn::Asks(g) => {
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
                Some(&*justice),
                None
            );
            g(asks, &r, &mut outs)
        }
        SystemFn::Arcs(g) => {
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
                Some(&*justice),
                Some(&*asks),
                None
            );
            g(arcs, &r, &mut outs)
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
