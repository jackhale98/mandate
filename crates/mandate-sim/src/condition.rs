//! The one condition evaluator (DEC-098 §5, task U1): every
//! [`Condition`] of `mandate_content::condition`, wherever content writes
//! it, is decided here. Read-only, pure and deterministic: it writes nothing
//! and draws from no stream (a `sponsored_law` chance is a keyed draw per
//! law and condition, as before).
//!
//! The context ([`Ctx`]) carries what a use binds: the jurisdiction (`at`),
//! the cast (dilemmas, goals, storylines), a conversation's slots, where it
//! is said or done (`here`), and the whole state where the evaluation runs
//! outside the simulation's systems. A local-only variant (DEC-098) holds
//! only for a town: for a region, the nation or no jurisdiction it never
//! does. What a use cannot provide never holds; the validator's table
//! (`mandate_content::condition::ConditionUse`) refuses it there first.
//!
//! The callers keep their entry points as thin wrappers: `narrative::cast::holds`
//! (a cast, inside the simulation), `policy::situation_holds` (a town's
//! situation), `talk::context::Ctx::holds` (a conversation), the overheard
//! draw and the visit verbs.

use crate::characters::{self, CharacterId};
use crate::narrative::cast::View;
use crate::narrative::{Actor, CastEntry};
use crate::places::Location;
use crate::player::knowledge::{self, Subject};
use crate::policy::PolicyStatus;
use crate::state::State;
use crate::talk::AboutRef;
use crate::talk::context::Ctx as TalkCtx;
use crate::world::{Jurisdiction, MunicipalityId, NeighborhoodId, NeighborhoodKind};
use mandate_content::condition::{CondTier, Phase, Worry};
use mandate_content::conversation::{ConcernOf, HoodOf};
use mandate_content::routines::DayPart;
use mandate_content::{Condition, HeardSubject, LawStatus, WaterFeature};
use std::cell::Cell;
use std::collections::BTreeMap;

/// Fluent arguments a condition provides when it holds.
pub type Args = BTreeMap<String, String>;

/// How far back a story counts as news (days).
pub const NEWS_DAYS: u32 = 56;
/// Share of residents who must know the player before they have an opinion.
pub const KNOWN: f64 = 0.2;
/// The fallback for an argument naming a Fluent key the facts lack.
pub(crate) const SUBJECT_FALLBACK: &str = "talk-subject-that";

/// Everything conditions read where they are evaluated.
pub struct Ctx<'a> {
    /// Every readable domain (politics through `v.pol`, which the politics
    /// domain passes while it writes itself) and the player's answers.
    pub v: &'a View<'a>,
    /// The whole state, where the evaluation runs outside the simulation's
    /// systems (views, commands, the lab): what only it holds (the event
    /// log, people's routines, the clock) is read from it, and never holds
    /// without it.
    pub st: Option<&'a State>,
    /// The jurisdiction the conditions are about (`None`: none, e.g. the
    /// narrative without a player).
    pub at: Option<Jurisdiction>,
    /// The cast (dilemmas, goals, storylines): their roles are the slots.
    pub cast: &'a [CastEntry],
    /// A conversation: `speaker`, `about`, `player` are the slots, and its
    /// place is `here`.
    pub talk: Option<&'a TalkCtx<'a>>,
    /// Where it is said or done (a line heard, a place visited).
    pub here: Option<Location>,
    /// The day part, when the caller fixes it (a visit in the late slot);
    /// else the clock's.
    pub part: Option<DayPart>,
    /// The story a `headline` found (an overheard line retells it).
    pub story: Cell<Option<u32>>,
    /// A policy family's bound slots (its trigger and expiry, DEC-092).
    pub family: Option<&'a [crate::policy::families::SlotBinding]>,
    /// The event log, where the whole state is not at hand (the policy
    /// families system reads it for `plant_closure`).
    pub events: Option<&'a crate::events::EventLog>,
    /// An arc run (DEC-109): its marks, heat and days, and what its roles
    /// are cast to (`arc_mark`, `arc_heat`, `arc_days`, `displaced`,
    /// `recovered`).
    pub arc: Option<&'a crate::arcs::ArcRun>,
}

impl<'a> Ctx<'a> {
    /// A bare context: `at`, nothing bound.
    pub fn new(v: &'a View<'a>, st: Option<&'a State>, at: Option<Jurisdiction>) -> Self {
        Ctx {
            v,
            st,
            at,
            cast: &[],
            talk: None,
            here: None,
            part: None,
            story: Cell::new(None),
            family: None,
            events: None,
            arc: None,
        }
    }

    /// What a family slot is bound to.
    fn bound(&self, slot: &str) -> Option<&'a crate::policy::families::Bound> {
        self.family?
            .iter()
            .find(|b| b.slot == slot)
            .map(|b| &b.bound)
    }

    /// The narrative's context: the player's home town, a cast, inside the
    /// simulation (no whole state).
    pub fn narrative(v: &'a View<'a>, cast: &'a [CastEntry]) -> Self {
        let mut cx = Ctx::new(
            v,
            None,
            v.town().map(|m| Jurisdiction::town(MunicipalityId(m))),
        );
        cx.cast = cast;
        cx
    }

    /// A town's situation on the whole state (a policy's unlock).
    pub fn town(v: &'a View<'a>, st: &'a State, m: MunicipalityId) -> Self {
        Ctx::new(v, Some(st), Some(Jurisdiction::town(m)))
    }

    /// At a location on the whole state (a line heard): the town of its
    /// neighbourhood.
    pub fn at_location(v: &'a View<'a>, st: &'a State, loc: Location) -> Self {
        let m = crate::places::hood_of(st, loc)
            .and_then(|h| st.world.neighborhoods.get(h.index()))
            .map(|n| n.municipality);
        let mut cx = Ctx::new(v, Some(st), m.map(Jurisdiction::town));
        cx.here = Some(loc);
        cx
    }

    /// A conversation: the player's town, its slots and place.
    pub fn talk(v: &'a View<'a>, t: &'a TalkCtx<'a>) -> Self {
        let mut cx = Ctx::new(v, Some(t.st), Some(Jurisdiction::town(t.town)));
        cx.talk = Some(t);
        cx.here = Some(t.here);
        cx
    }

    /// The town the conditions are about.
    pub fn town_id(&self) -> Option<MunicipalityId> {
        self.at.and_then(|a| a.municipality())
    }

    pub fn town_character(&self) -> Option<&'a characters::TownCharacter> {
        self.v.pol.characters.towns.get(self.town_id()?.index())
    }

    /// The player's character.
    pub fn me(&self) -> Option<CharacterId> {
        match self.talk {
            Some(t) => Some(t.me),
            None => self.v.me(),
        }
    }

    /// The person in a slot: a conversation's, or a cast role's.
    pub fn person(&self, slot: &str) -> Option<CharacterId> {
        match self.talk {
            Some(t) => t.person(slot),
            None => crate::narrative::person_of(self.cast, slot),
        }
    }

    /// The neighbourhood where it is said or done.
    pub fn here_hood(&self) -> Option<NeighborhoodId> {
        if let Some(t) = self.talk {
            return t.hood(HoodOf::Here);
        }
        crate::places::hood_of(self.st?, self.here?)
    }

    /// The neighbourhood a worry condition reads (`None` for the town).
    fn worry_hood(&self, of: ConcernOf) -> Option<NeighborhoodId> {
        match of {
            ConcernOf::Town => None,
            ConcernOf::Speaker => self.talk?.hood(HoodOf::SpeakerHome),
            ConcernOf::Here => self.here_hood(),
        }
    }

    /// The player's next election day: their running race's, else the
    /// calendar's (the scheduled day, else the constitution's next cycle).
    pub fn election_day(&self) -> Option<mandate_core::SimDate> {
        let v = self.v;
        if v.player().is_some() {
            return v.next_election();
        }
        if let Some(d) = v.pol.elections.next_day.filter(|d| *d >= v.date()) {
            return Some(d);
        }
        let c = v.content().constitutions.get(&v.r.meta.constitution)?;
        crate::politics::elections::next_cycle(c, v.date()).map(|(d, _)| d)
    }

    /// The day part now.
    fn day_part(&self) -> Option<DayPart> {
        self.part
            .or_else(|| self.st.map(crate::characters::routines::day_part))
    }
}

fn in_range(v: f64, min: Option<f64>, max: Option<f64>) -> bool {
    min.is_none_or(|a| v >= a) && max.is_none_or(|z| v <= z)
}

/// A named plant's state as content names it.
pub fn plant_state(s: &crate::economy::plants::PlantStatus) -> mandate_content::condition::PlantState {
    use crate::economy::plants::PlantStatus as P;
    use mandate_content::condition::PlantState as S;
    match s {
        P::Running => S::Running,
        P::ShortTime { .. } => S::ShortTime,
        P::Threatened { .. } => S::Threatened,
        P::Closed { .. } => S::Closed,
    }
}

/// `plant` (DEC-110): the plant at a run's place role, or any plant in the
/// jurisdiction (the first that fits, in id order), in one of the states
/// asked for; under notice, its date close enough.
fn plant_holds(
    cx: &Ctx<'_>,
    slot: Option<&str>,
    status: &[mandate_content::condition::PlantState],
    closes_within_days: Option<u32>,
    args: &mut Args,
) -> bool {
    use crate::economy::plants::{self, PlantStatus};
    let v = cx.v;
    let econ = v.r.economy();
    let world = v.r.world;
    let today = v.date();
    let fits = |p: &plants::Plant| {
        (status.is_empty() || status.contains(&plant_state(&p.status)))
            && closes_within_days.is_none_or(|d| match p.status {
                PlantStatus::Threatened { closes_on, .. } => {
                    closes_on <= today || today.days_until(closes_on) <= d
                }
                _ => false,
            })
    };
    let found = match slot {
        Some(role) => match cx.arc.and_then(|r| r.actor(role)) {
            Some(crate::arcs::ArcActor::Place { id }) => {
                plants::at_place(econ, id).filter(|p| fits(p))
            }
            _ => None,
        },
        None => {
            let Some(at) = cx.at else { return false };
            econ.plants.iter().find(|p| {
                world
                    .places
                    .get(p.place.index())
                    .is_some_and(|pl| at.contains_hood(world, pl.neighborhood))
                    && fits(p)
            })
        }
    };
    let Some(p) = found else { return false };
    for (k, x) in plants::text_args(econ, v.content(), p.id) {
        args.insert(k.into(), x);
    }
    // A closed plant keeps its last size: the jobs that went with it.
    let jobs = match p.status {
        PlantStatus::Closed { .. } => p.capacity.round().max(0.0) as u32,
        _ => plants::facts(econ, world, v.content(), p.id).map_or(0, |f| f.jobs),
    };
    args.insert("jobs".into(), jobs.to_string());
    if let PlantStatus::Threatened { closes_on, .. } = p.status {
        let c = closes_on.calendar();
        args.insert("closes_day".into(), c.day.to_string());
        args.insert("closes_month_key".into(), format!("month-{}", c.month));
        args.insert("closes_year".into(), c.year.to_string());
    }
    true
}

/// Do all conditions hold? Collects their arguments.
pub fn all_hold(cx: &Ctx<'_>, cs: &[Condition], args: &mut Args) -> bool {
    cs.iter().all(|c| holds(cx, c, args))
}

/// Does a condition hold? Adds the arguments it provides when it does.
pub fn holds(cx: &Ctx<'_>, c: &Condition, args: &mut Args) -> bool {
    use Condition as C;
    // DEC-098: a local-only variant reads a town; for a region, the nation
    // or no jurisdiction it never holds.
    if !matches!(c, C::Any { .. }) && c.tier() == CondTier::Local && cx.town_id().is_none() {
        return false;
    }
    let v = cx.v;
    let pol = v.pol;
    let content = v.content();
    let today = v.date();
    let m = cx.town_id().map(|t| t.0).unwrap_or(0);
    match c {
        // --- The town's situation ----------------------------------------------------
        C::Crisis { kind, min_severity } => {
            let Some(x) = cx.town_character().and_then(|t| {
                t.crises
                    .iter()
                    .filter(|x| {
                        kind.as_ref().is_none_or(|k| &x.kind == k)
                            && x.severity as f64 >= *min_severity
                    })
                    .max_by(|a, b| a.severity.total_cmp(&b.severity))
            }) else {
                return false;
            };
            if let Some(d) = content.crises.iter().find(|d| d.id == x.kind) {
                args.insert("crisis_key".into(), d.name_key.clone());
            }
            true
        }
        C::Indicator {
            indicator,
            min,
            max,
        } => in_range(
            crate::concerns::indicator(v.r, m as usize, *indicator),
            *min,
            *max,
        ),
        C::Concern { .. } => concern(cx, c, args),
        C::Industry { drivers } => {
            let Some(s) = cx
                .town_character()
                .and_then(|t| t.industry.as_deref())
                .and_then(|s| content.sector(s))
            else {
                return false;
            };
            if drivers.as_ref().is_some_and(|d| !d.contains(&s.driver)) {
                return false;
            }
            args.insert("industry_key".into(), crate::overheard::talk_key(s));
            true
        }
        C::PlantClosure { within_months } => {
            let Some(events) = cx.st.map(|st| &st.events).or(cx.events) else {
                return false;
            };
            let days = *within_months as u32 * 30;
            let found = events
                .iter()
                .rev()
                .take_while(|e| e.date.days_until(today) <= days)
                .find_map(|e| match &e.kind {
                    crate::events::EventKind::PlantClosure {
                        municipality,
                        sector,
                        jobs,
                        ..
                    } if *municipality == m => Some((sector.clone(), *jobs)),
                    _ => None,
                });
            let Some((sector, jobs)) = found else {
                return false;
            };
            if let Some(s) = content.sector(&sector) {
                args.insert("industry_key".into(), crate::overheard::talk_key(s));
            }
            args.insert("jobs".into(), jobs.to_string());
            true
        }
        C::Disaster {
            within_months,
            hazard: None,
        } => {
            v.r.economy()
                .munis
                .get(m as usize)
                .is_some_and(|e| e.months_since_disaster < *within_months)
        }
        // Of one hazard: the environment's footprint (homes it damaged
        // that are not yet repaired).
        C::Disaster {
            within_months,
            hazard: Some(k),
        } => {
            let kind = crate::arcs::cast::hazard_kind(*k);
            let days = u32::from(*within_months) * 30;
            let env = v.r.environment();
            v.r.world.hoods_of(MunicipalityId(m)).any(|h| {
                env.damage(NeighborhoodId(h as u32)).is_some_and(|d| {
                    d.hazard == kind && d.struck <= today && d.struck.days_until(today) < days
                })
            })
        }
        C::LawInForce { templates, absent } => {
            let any = v.r.policy().in_municipality(m).any(|p| {
                matches!(p.status, PolicyStatus::Enacted | PolicyStatus::InEffect)
                    && templates.contains(&p.template)
            });
            any != *absent
        }
        C::Water { feature } => {
            let mu = v.r.world.municipalities.get(m as usize);
            let river = mu.is_some_and(|x| x.on_river);
            let coast = mu.is_some_and(|x| x.coastal);
            match feature {
                WaterFeature::River => river,
                WaterFeature::Coast => coast,
                WaterFeature::Any => river || coast || waterfront(cx),
            }
        }
        C::Dry => {
            let mu = v.r.world.municipalities.get(m as usize);
            !(mu.is_some_and(|x| x.on_river) || mu.is_some_and(|x| x.coastal) || waterfront(cx))
        }
        C::Broker => {
            let Some(n) = cx
                .town_character()
                .and_then(|t| t.broker)
                .and_then(|b| name(cx, b))
            else {
                return false;
            };
            args.insert("broker".into(), n);
            true
        }
        C::Rival => {
            let Some(n) = cx
                .town_character()
                .and_then(|t| t.rival)
                .and_then(|r| name(cx, r))
            else {
                return false;
            };
            args.insert("rival".into(), n);
            true
        }
        C::RivalRecord { min, max } => {
            let Some((n, record)) = cx.town_character().and_then(|t| t.rival).and_then(|r| {
                let p = pol.politicians.iter().find(|p| p.character == r)?;
                Some((p.full_name(), p.record as f64))
            }) else {
                return false;
            };
            if !in_range(record, *min, *max) {
                return false;
            }
            args.insert("rival".into(), n);
            true
        }
        C::Scandal { about } => {
            let me = v.player().map(|p| p.politician);
            let rival = cx
                .town_character()
                .and_then(|t| t.rival)
                .and_then(|r| pol.politicians.iter().find(|p| p.character == r))
                .map(|p| p.id);
            let Some(x) =
                v.r.media()
                    .scandals
                    .iter()
                    .rev()
                    .filter(|x| x.running() && x.town == m)
                    .find(|x| match about {
                        HeardSubject::Anyone => true,
                        HeardSubject::Player => Some(x.subject) == me,
                        HeardSubject::Rival => Some(x.subject) == rival,
                    })
            else {
                return false;
            };
            let Some(n) = pol.get(x.subject).map(|p| p.full_name()) else {
                return false;
            };
            args.insert("subject".into(), n);
            args.insert(
                "fact_key".into(),
                format!("fact-{}", x.kind.replace('_', "-")),
            );
            true
        }
        C::Headline => {
            let media = v.r.media();
            let Some(s) = media.stories.iter().rev().find(|s| {
                s.town == m
                    && s.date.days_until(today) <= NEWS_DAYS
                    && media
                        .outlet(s.outlet)
                        .is_some_and(|o| o.covers(v.r.world, m))
            }) else {
                return false;
            };
            cx.story.set(Some(s.id));
            true
        }
        C::PlayerAwareness { min, max } => {
            let (Some((aw, _)), Some(n)) = (standing(cx), player_name(cx)) else {
                return false;
            };
            if !in_range(aw, *min, *max) {
                return false;
            }
            args.insert("player".into(), n);
            true
        }
        C::PlayerApproval { min, max } => {
            let (Some((aw, ap)), Some(n)) = (standing(cx), player_name(cx)) else {
                return false;
            };
            if aw < KNOWN || !in_range(ap, *min, *max) {
                return false;
            }
            args.insert("player".into(), n);
            true
        }
        C::Shedding { min } => {
            let (Some(st), Some(Location::Place { place })) = (cx.st, cx.here) else {
                return false;
            };
            let Some(p) = st.world.places.get(place.index()) else {
                return false;
            };
            let Some(s) = content
                .place_kind(p.kind.key())
                .and_then(|k| k.sector.as_deref())
                .and_then(|s| content.sector(s))
            else {
                return false;
            };
            let x = crate::visits::shedding(st, content, p.municipality, &s.id);
            if x <= 0.0 || x < *min {
                return false;
            }
            args.insert("industry_key".into(), crate::overheard::talk_key(s));
            true
        }
        C::Present { occupations } => {
            let (Some(st), Some(loc)) = (cx.st, cx.here) else {
                return false;
            };
            crate::visits::present(st, content, loc)
                .into_iter()
                .any(|c| {
                    crate::visits::occupation_of(st, c)
                        .is_some_and(|o| occupations.iter().any(|w| w == o))
                })
        }
        // --- Time and the calendar ---------------------------------------------------
        C::DayPart { parts } => cx.day_part().is_some_and(|p| parts.contains(&p)),
        C::Phase { phase } => {
            let Some(at) = cx.at else {
                return false;
            };
            let running = pol
                .elections
                .races
                .iter()
                .any(|r| r.seat.at() == at && r.election_day >= today);
            running == (*phase == Phase::Campaign)
        }
        C::ElectionWithin { days } => cx
            .election_day()
            .is_some_and(|d| d >= today && today.days_until(d) <= *days),
        C::MonthsInPlay { min } => {
            v.play_began().days_until(today) >= min * 30 && today >= v.play_began()
        }
        // --- The player --------------------------------------------------------------
        C::Campaigning { is } => v.campaigning() == *is,
        C::Office { posts, role } => {
            // Role words through the constitution (DEC-098), or `any` / `none`.
            let Some(p) = v.player().and_then(|pl| pol.get(pl.politician)) else {
                return false;
            };
            let word = content
                .constitutions
                .get(&v.r.meta.constitution)
                .and_then(|c| crate::politics::role_word(c, p));
            posts.iter().chain(role.iter()).any(|w| match w.as_str() {
                mandate_content::OFFICE_ANY => p.in_office(),
                mandate_content::OFFICE_NONE => !p.in_office(),
                w => word == Some(w),
            })
        }
        C::HoldsPower { power } => {
            let (Some(pl), Some(at)) = (v.player(), cx.at) else {
                return false;
            };
            let Some(c) = content.constitutions.get(&v.r.meta.constitution) else {
                return false;
            };
            crate::politics::holds_power_at(pol, c, pl.politician, *power, at)
        }
        C::Track { track, min, max } => v.track(*track).is_some_and(|x| in_range(x, *min, *max)),
        C::Money { min, max } => v
            .player()
            .is_some_and(|pl| in_range(pl.money.units() as f64, *min, *max)),
        C::Answered { .. } => !crate::narrative::cast::answered_matches(v, c, cx.cast).is_empty(),
        C::SponsoredLaw {
            status,
            min_age_days,
            max_age_days,
            chance,
            slot,
        } => {
            let Some(pl) = v.player() else { return false };
            let only = match slot {
                Some(r) => match crate::narrative::actor_of(cx.cast, r) {
                    Some(Actor::Policy { id }) => Some(id),
                    _ => return false,
                },
                None => None,
            };
            let salt = crate::narrative::cast::sponsored_law_salt(c);
            // The player's laws in any jurisdiction (DEC-098).
            v.r.policy().instances.iter().any(|p| {
                if p.sponsor != Some(pl.politician) || only.is_some_and(|id| id != p.id.0) {
                    return false;
                }
                let (ok, from) = match status {
                    LawStatus::InForce => (p.active(), p.enacted_on),
                    LawStatus::Pending => (
                        p.status == PolicyStatus::UnderConsideration,
                        p.introduced_on,
                    ),
                    LawStatus::Ended => (
                        p.status == PolicyStatus::Ended,
                        p.enacted_on.or(p.introduced_on),
                    ),
                };
                let Some(from) = from.filter(|f| *f <= today) else {
                    return false;
                };
                let age = from.days_until(today);
                ok && age >= *min_age_days
                    && max_age_days.is_none_or(|m| age <= m)
                    && (*chance >= 1.0
                        || mandate_core::rng::keyed(
                            v.r.meta.seed,
                            mandate_core::Stream::Director,
                            &[crate::narrative::KEY_SPONSORED_LAW, p.id.0 as u64, salt],
                        )
                        .next_f64()
                            < *chance)
            })
        }
        C::Knows { subject, min } => {
            crate::narrative::learn::knows(v, subject, *min, cx.cast, cx.here_hood())
        }
        C::PlayerKnows { subject, min, max } => {
            let Some(t) = cx.talk else {
                return false;
            };
            // Something that does not exist here (a town without a broker)
            // is unknown to the player, and gives no arguments.
            let s = t.subject(subject);
            let l = s
                .as_ref()
                .map(|s| knowledge::knows(t.st, s))
                .unwrap_or_default();
            if min.is_some_and(|a| l < a) || max.is_some_and(|z| l > z) {
                return false;
            }
            if let Some(s) = &s {
                t.subject_args(s, args);
            }
            true
        }
        // --- A person in a slot ------------------------------------------------------
        C::Occupation { slot, occupations } => cx
            .person(slot)
            .and_then(|p| role_key(cx, p))
            .is_some_and(|r| occupations.contains(&r)),
        C::Trait { slot, traits } => cx
            .person(slot)
            .and_then(|p| pol.characters.get(p))
            .is_some_and(|ch| ch.traits.iter().any(|t| traits.contains(t))),
        C::Personality {
            slot,
            axis,
            min,
            max,
        } => cx
            .person(slot)
            .and_then(|p| characters::profile(pol, v.r.world, p))
            .is_some_and(|p| {
                let x = &p.personality;
                let val = match axis.as_str() {
                    "ambition" => x.ambition,
                    "integrity" => x.integrity,
                    "warmth" => x.warmth,
                    "boldness" => x.boldness,
                    "loyalty" => x.loyalty,
                    _ => return false,
                };
                in_range(val as f64, *min, *max)
            }),
        C::Goal { slot, goal } => cx.person(slot).is_some_and(|p| {
            pol.characters
                .goals
                .iter()
                .any(|g| g.holder == p && &g.goal == goal)
        }),
        C::Opinion { slot, min, max } => match (cx.person(slot), cx.me()) {
            (Some(p), Some(me)) => {
                p != me && in_range(characters::opinion(pol, p, me) as f64, *min, *max)
            }
            _ => false,
        },
        C::Trust { slot, min, max } => match (cx.person(slot), cx.me()) {
            (Some(p), Some(me)) => {
                p != me && in_range(characters::trust(pol, content, p, me) as f64, *min, *max)
            }
            _ => false,
        },
        C::Remembers { slot, kinds } => remembers(cx, slot, kinds, args),
        C::Favour {
            slot,
            owed_to_player,
        } => {
            let (Some(p), Some(me)) = (cx.person(slot), cx.me()) else {
                return false;
            };
            if p == me {
                return false;
            }
            let n = if *owed_to_player {
                characters::favours_owed(pol, p, me)
            } else {
                characters::favours_owed(pol, me, p)
            };
            if n == 0 {
                return false;
            }
            args.insert("n".into(), n.to_string());
            true
        }
        C::Met { slot, is } => {
            let (Some(p), Some(me)) = (cx.person(slot), cx.me()) else {
                return false;
            };
            let talked = v.player().is_some_and(|pl| pl.talk.used.contains_key(&p));
            let met = pol.characters.get(p).is_some_and(|ch| {
                ch.memories.iter().any(|x| {
                    x.actor == me && x.kind == characters::MemoryKind::Met && x.date < today
                })
            });
            (p != me && (talked || met)) == *is
        }
        C::IsBroker { slot, is } => {
            let Some(p) = cx.person(slot) else {
                return false;
            };
            let yes = cx.town_character().and_then(|t| t.broker) == Some(p);
            if yes
                && *is
                && let Some(t) = cx.talk
            {
                t.broker_args(p, args);
            }
            yes == *is
        }
        C::IsRival { slot, is } => {
            let Some(p) = cx.person(slot) else {
                return false;
            };
            let yes = cx.town_character().and_then(|t| t.rival) == Some(p);
            if yes
                && *is
                && let Some(n) = name(cx, p)
            {
                args.insert("rival".into(), n);
            }
            yes == *is
        }
        C::PartyLineAgainstWard { slot } => match crate::narrative::actor_of(cx.cast, slot) {
            Some(Actor::Policy { id }) => {
                let line = v.party_line(id);
                let lean = v.ward_lean(id);
                line != 0 && lean.abs() > 0.02 && (line as f64) * lean < 0.0
            }
            _ => false,
        },
        C::CanVote { slot } => match crate::narrative::actor_of(cx.cast, slot) {
            Some(Actor::Policy { id }) => v.can_vote(id),
            _ => false,
        },
        // --- A conversation ----------------------------------------------------------
        C::Tells { subject, min } => {
            let Some(t) = cx.talk else {
                return false;
            };
            let Some(s) = t.subject(subject) else {
                return false;
            };
            if t.told(&s) < *min {
                return false;
            }
            let mut a = Args::new();
            t.subject_args(&s, &mut a);
            // Worries that worry nobody are no answer.
            if matches!(
                s,
                Subject::TownConcerns { .. } | Subject::HoodConcerns { .. }
            ) && !a.contains_key("concern_key")
            {
                return false;
            }
            args.extend(a);
            true
        }
        C::Voice { voices } => cx
            .talk
            .and_then(crate::talk::reply::voice)
            .is_some_and(|x| voices.contains(&x.id)),
        C::About {
            kinds,
            places,
            workplace,
            home,
            office,
            works,
            here,
            tied,
            is,
        } => {
            let Some(t) = cx.talk else {
                return false;
            };
            // How the speaker is tied to the neighbourhood asked about.
            let ties = |a: &AboutRef| -> Option<crate::talk::context::HoodTies> {
                match a {
                    AboutRef::Hood { hood } => Some(t.ties(*hood)),
                    _ => None,
                }
            };
            let holds = t.about.is_some_and(|a| {
                if !kinds.is_empty() && !kinds.contains(&a.kind()) {
                    return false;
                }
                let place = match a {
                    AboutRef::Place { place } => t.st.world.places.get(place.index()),
                    _ => None,
                };
                if !places.is_empty()
                    && !place.is_some_and(|p| places.iter().any(|k| k == p.kind.key()))
                {
                    return false;
                }
                if let Some(h) = home {
                    let mine = matches!(a, AboutRef::Hood { .. })
                        && t.hood(HoodOf::About) == t.hood(HoodOf::SpeakerHome);
                    if mine != *h {
                        return false;
                    }
                }
                let tie = ties(a);
                for (want, got) in [
                    (office, tie.map(|x| x.office)),
                    (works, tie.map(|x| x.works)),
                    (here, tie.map(|x| x.here)),
                    (tied, tie.map(|x| x.any())),
                ] {
                    if let Some(w) = want
                        && got.unwrap_or(false) != *w
                    {
                        return false;
                    }
                }
                match workplace {
                    None => true,
                    Some(w) => {
                        let mine = place
                            .is_some_and(|p| t.workplace(t.speaker).is_some_and(|q| q.id == p.id));
                        mine == *w
                    }
                }
            });
            holds == *is
        }
        // --- A policy family's bound slots (DEC-092) ------------------------------------
        C::HoodConcern {
            slot, concern, top, ..
        } => {
            use crate::policy::families::Bound;
            let Some(Bound::Neighborhood { hood }) = cx.bound(slot) else {
                return false;
            };
            let Some(i) = content.concern_index(concern) else {
                return false;
            };
            let ranking = crate::concerns::ranked(
                v.r.population(),
                content,
                v.r.world,
                crate::concerns::Scope::Neighbourhood(*hood),
                Worry::Measured,
            );
            crate::concerns::place(&ranking, i).is_some_and(|(p, _)| p < *top as usize)
        }
        C::HoodHazard { slot, hazard, min } => {
            use crate::policy::families::Bound;
            let Some(Bound::Neighborhood { hood }) = cx.bound(slot) else {
                return false;
            };
            let Some(e) = v.r.environment().exposure(*hood) else {
                return false;
            };
            let x = match hazard {
                mandate_content::HazardKey::Flood => e.flood,
                mandate_content::HazardKey::Storm => e.storm,
                mandate_content::HazardKey::Wildfire => e.wildfire,
            } as f64
                / 1000.0;
            x >= *min
        }
        C::SectorDecline { slot, min } => {
            use crate::policy::families::Bound;
            let Some(Bound::Sector { sector }) = cx.bound(slot) else {
                return false;
            };
            let Some(s) = content
                .sector_index(sector)
                .and_then(|k| v.r.economy().munis.get(m as usize)?.sectors.get(k))
            else {
                return false;
            };
            let base = s.baseline_jobs as f64;
            base > 0.0 && ((base - s.jobs) / base).max(0.0) >= *min
        }
        C::PlaceOpen { slot } | C::PlaceClosed { slot } => {
            use crate::policy::families::Bound;
            let Some(Bound::Place { place }) = cx.bound(slot) else {
                return false;
            };
            let open =
                v.r.world
                    .places
                    .get(place.index())
                    .is_some_and(|p| p.is_open());
            open == matches!(c, C::PlaceOpen { .. })
        }
        // --- Staged arcs (DEC-109) ---------------------------------------------------
        C::Arc { arc, stages, here } => {
            let (Some(at), Some(arcs)) = (cx.at, v.r.arcs()) else {
                return false;
            };
            arcs.runs.iter().any(|run| {
                run.at == at
                    && &run.template == arc
                    && (stages.is_empty() || stages.contains(&run.stage))
                    && here.as_ref().is_none_or(|role| arc_here(cx, run, role))
            })
        }
        C::ArcMark { mark, is } => cx.arc.is_some_and(|r| r.has_mark(mark) == *is),
        C::ArcHeat { min, max } => cx
            .arc
            .is_some_and(|r| in_range(f64::from(r.heat), *min, *max)),
        C::ArcDays { min, max } => cx
            .arc
            .is_some_and(|r| in_range(f64::from(r.days_in_stage(today)), *min, *max)),
        C::Displaced { slot, min, max } => {
            let Some(area) = cx.arc.and_then(|r| r.area(slot)) else {
                return false;
            };
            let (away, all) = crate::arcs::cast::away(v.r, area);
            let share = if all > 0 {
                away as f64 / all as f64
            } else {
                0.0
            };
            if !in_range(share, *min, *max) {
                return false;
            }
            args.insert("away".into(), away.to_string());
            true
        }
        C::Recovered { slot, min, max } => {
            let Some((run, area)) = cx.arc.and_then(|r| Some((r, r.area(slot)?))) else {
                return false;
            };
            // Against the damage as the run was raised.
            let base = run.base(&format!("{slot}.damage")).unwrap_or(0);
            let now = crate::arcs::cast::damage(v.r, area, None);
            let share = if base == 0 {
                1.0
            } else {
                (1.0 - now as f64 / base as f64).clamp(0.0, 1.0)
            };
            if !in_range(share, *min, *max) {
                return false;
            }
            let homes = run.base(&format!("{slot}.homes")).unwrap_or(0);
            args.insert("homes".into(), homes.to_string());
            args.insert(
                "repaired".into(),
                ((homes as f64 * share).round() as u64).to_string(),
            );
            true
        }
        C::ArcLastTurn { stage, is } => cx
            .arc
            .is_some_and(|r| (r.before.as_deref() == Some(stage.as_str())) == *is),
        // --- The budget and the body that votes it (tier-generic) --------------------
        C::BudgetYear { min, max } => {
            let Some(at) = cx.at else { return false };
            let left = today.days_until(crate::finance::budget_year_turns(at, today));
            in_range(f64::from(left), *min, *max)
        }
        C::BudgetBalance { min, max } => {
            let Some(t) = cx
                .at
                .and_then(|at| crate::finance::treasury(v.r.finance(), at))
            else {
                return false;
            };
            let revenue = t.last_year.revenue.units() as f64;
            let share = if t.last_year.year == 0 || revenue <= 0.0 {
                0.0
            } else {
                t.last_year.balance.units() as f64 / revenue
            };
            if !in_range(share, *min, *max) {
                return false;
            }
            let books = if share < -0.01 {
                "short"
            } else if share > 0.01 {
                "over"
            } else {
                "level"
            };
            args.insert("books".into(), books.into());
            args.insert(
                "gap".into(),
                ((share.abs() * 100.0).round() as u64).to_string(),
            );
            true
        }
        C::BodyMajority { power, share, is } => {
            let Some(at) = cx.at else { return false };
            let holders = crate::arcs::cast::power_holders(v, at, *power);
            let mut parties: Vec<(crate::politics::PartyId, usize)> = Vec::new();
            for p in holders.iter().filter_map(|id| pol.get(*id)) {
                let Some(party) = p.party else { continue };
                match parties.iter_mut().find(|x| x.0 == party) {
                    Some(x) => x.1 += 1,
                    None => parties.push((party, 1)),
                }
            }
            let n = holders.len();
            let held = parties.iter().any(|x| match share {
                Some(s) => n > 0 && x.1 as f64 >= *s * n as f64,
                None => x.1 * 2 > n,
            });
            held == *is
        }
        C::BodySupport {
            power,
            opinion,
            min,
            max,
        } => {
            let (Some(at), Some(pl)) = (cx.at, v.player()) else {
                return false;
            };
            let Some(me) = cx.me() else { return false };
            let mine = pol.get(pl.politician).and_then(|p| p.party);
            let line = opinion.unwrap_or(10.0);
            let others: Vec<&crate::politics::Politician> =
                crate::arcs::cast::power_holders(v, at, *power)
                    .into_iter()
                    .filter(|id| *id != pl.politician)
                    .filter_map(|id| pol.get(id))
                    .collect();
            let backers = others
                .iter()
                .filter(|p| {
                    (mine.is_some() && p.party == mine)
                        || f64::from(characters::opinion(pol, p.character, me)) >= line
                        || characters::favours_owed(pol, p.character, me) > 0
                })
                .count();
            let share = if others.is_empty() {
                0.0
            } else {
                backers as f64 / others.len() as f64
            };
            if !in_range(share, *min, *max) {
                return false;
            }
            args.insert("backers".into(), backers.to_string());
            args.insert("members".into(), others.len().to_string());
            true
        }
        C::LastRace { won } => {
            let Some(pl) = v.player() else { return false };
            let me = pl.politician;
            // Still standing in a race: the last one is not decided.
            if characters::is_candidate(pol, me) {
                return false;
            }
            pol.elections
                .results
                .iter()
                .filter(|r| r.candidates.iter().any(|c| c.politician == me))
                .max_by_key(|r| r.date)
                .is_some_and(|r| (r.winner == me) == *won)
        }
        C::Plant {
            slot,
            status,
            closes_within_days,
        } => plant_holds(cx, slot.as_deref(), status, *closes_within_days, args),
        C::SponsorsBill { slot } => {
            let Some(p) = cx.person(slot) else {
                return false;
            };
            v.r.policy().instances.iter().any(|b| {
                b.status == crate::policy::PolicyStatus::UnderConsideration
                    && b.sponsor
                        .and_then(|s| pol.get(s))
                        .is_some_and(|s| s.character == p)
                    && v.can_vote(b.id.0)
            })
        }
        // --- Composition -------------------------------------------------------------
        C::Any { of } => {
            for x in of {
                let mut a = Args::new();
                if holds(cx, x, &mut a) {
                    args.extend(a);
                    return true;
                }
            }
            false
        }
    }
}

/// Whether what is said or done here is where a role of an arc run is: its
/// place, or anywhere in its area.
fn arc_here(cx: &Ctx<'_>, run: &crate::arcs::ArcRun, role: &str) -> bool {
    use crate::arcs::ArcActor;
    match (run.actor(role), cx.here) {
        (Some(ArcActor::Place { id }), Some(Location::Place { place })) => id == place,
        (Some(ArcActor::Area { area }), Some(_)) => cx
            .here_hood()
            .is_some_and(|h| area.hoods(cx.v.r.world).contains(&h.index())),
        _ => false,
    }
}

/// A person's name.
fn name(cx: &Ctx<'_>, c: CharacterId) -> Option<String> {
    characters::profile(cx.v.pol, cx.v.r.world, c).map(|p| p.full_name())
}

fn player_name(cx: &Ctx<'_>) -> Option<String> {
    let pl = cx.v.player()?;
    cx.v.pol.get(pl.politician).map(|p| p.full_name())
}

/// A waterfront neighbourhood here.
fn waterfront(cx: &Ctx<'_>) -> bool {
    cx.here_hood()
        .and_then(|h| cx.v.r.world.neighborhoods.get(h.index()))
        .is_some_and(|n| n.kind == NeighborhoodKind::Waterfront)
}

/// The player's mean awareness and approval among the residents here.
pub fn standing(cx: &Ctx<'_>) -> Option<(f64, f64)> {
    cx.v.player()?;
    let h = cx.here_hood()?;
    let cs = &cx.v.r.population().hoods.get(h.index())?.cohorts;
    let n: f64 = cs.iter().map(|c| c.count as f64).sum();
    if n <= 0.0 {
        return None;
    }
    let aw = cs
        .iter()
        .map(|c| c.player_awareness as f64 * c.count as f64)
        .sum::<f64>()
        / n;
    let ap = cs
        .iter()
        .map(|c| c.player_approval as f64 * c.count as f64)
        .sum::<f64>()
        / n;
    Some((aw, ap))
}

/// A person's role word: their post's (DEC-098), else their occupation.
pub fn role_key(cx: &Ctx<'_>, id: CharacterId) -> Option<String> {
    let pol = cx.v.pol;
    let c = pol.characters.get(id)?;
    if let Some(p) = c.politician.and_then(|p| pol.get(p)) {
        let constitution =
            cx.v.content()
                .constitutions
                .get(&cx.v.r.meta.constitution)?;
        return crate::politics::role_word(constitution, p).map(str::to_string);
    }
    c.named.as_deref().map(|n| n.occupation.clone())
}

/// The worries a `concern` condition ranks, strongest first, as registry
/// indices (those that worry at all): the town's or a neighbourhood's, felt
/// or measured, as `concerns::ranked` gives them.
pub fn ranked_worries(cx: &Ctx<'_>, of: ConcernOf, by: Worry) -> Vec<usize> {
    use crate::concerns::Scope;
    let v = cx.v;
    let scope = match cx.worry_hood(of) {
        Some(h) => Scope::Neighbourhood(h),
        None if of == ConcernOf::Town => match cx.town_id() {
            Some(m) => Scope::town(m),
            None => return Vec::new(),
        },
        None => return Vec::new(),
    };
    let ranking = crate::concerns::ranked(v.r.population(), v.content(), v.r.world, scope, by);
    crate::concerns::top(&ranking, usize::MAX)
        .map(|(i, _)| i)
        .collect()
}

/// `$concern_key` and `$concern_talk_key` (the noun phrase of talk, or the
/// name without one) for the concern at registry index `i`.
pub fn concern_args(content: &mandate_content::ContentBundle, i: usize, args: &mut Args) {
    if let Some(c) = content.concerns.get(i) {
        args.insert("concern_key".into(), c.name_key.clone());
        let talk = c.talk_key.as_ref().unwrap_or(&c.name_key);
        args.insert("concern_talk_key".into(), talk.clone());
    }
}

fn concern(cx: &Ctx<'_>, c: &Condition, args: &mut Args) -> bool {
    let Some(w) = c.worry() else {
        return false;
    };
    let content = cx.v.content();
    let Some(i) = ranked_worries(cx, w.of, w.by)
        .into_iter()
        .take(w.top as usize)
        .find(|i| {
            content
                .concerns
                .get(*i)
                .is_some_and(|x| w.concerns.contains(&&x.id))
        })
    else {
        return false;
    };
    concern_args(content, i, args);
    true
}

fn remembers(cx: &Ctx<'_>, slot: &str, kinds: &[String], args: &mut Args) -> bool {
    let (Some(p), Some(me)) = (cx.person(slot), cx.me()) else {
        return false;
    };
    let v = cx.v;
    let Some(m) = characters::salient_memories(v.pol, p, Some(me), v.date(), usize::MAX)
        .into_iter()
        .find(|m| kinds.iter().any(|k| k == m.kind.key()))
    else {
        return false;
    };
    // What it was about, and what kind of thing that is (`$subject_kind`:
    // `bill`, a policy's name; `decision`, a decision's title, a noun
    // phrase; `none`), so a line can say it naturally or leave it out. A
    // decision is read from the whole state only (the narrative is not
    // readable while it is written).
    let content = v.content();
    let subject = match m.subject {
        Some(characters::Subject::Policy { id }) => cx
            .st
            .and_then(|st| {
                st.policy
                    .instances
                    .get(id as usize)
                    .and_then(|i| crate::policy::resolve(content, &st.policy, i))
            })
            .map(|t| (t.name_key.clone(), "bill")),
        Some(characters::Subject::Decision { id }) => cx
            .st
            .and_then(|st| st.narrative.decision(crate::narrative::DecisionId(id)))
            .and_then(|d| content.dilemma(&d.template))
            .map(|t| (t.title_key.clone(), "decision")),
        _ => None,
    };
    let (subject, kind) = subject.unwrap_or_else(|| (SUBJECT_FALLBACK.into(), "none"));
    args.insert("memory_key".into(), m.kind.fluent_key());
    args.insert("subject_key".into(), subject);
    args.insert("subject_kind".into(), kind.into());
    args.insert("year".into(), m.date.calendar().year.to_string());
    true
}

/// Evaluate with a context on the whole state, built here.
pub fn on_state<R>(
    st: &State,
    content: &mandate_content::ContentBundle,
    f: impl FnOnce(&View<'_>) -> R,
) -> R {
    let r = crate::scheduler::Reads::of_state(st, content);
    let v = View::new(&r, &st.politics);
    f(&v)
}
