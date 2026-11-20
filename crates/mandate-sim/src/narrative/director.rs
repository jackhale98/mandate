//! The director (DEC-033 B): chapters and their tension curve, budgets,
//! cooldowns, relevance scoring, the spotlight (decisions, requests,
//! storyline beats, developments), fairness warnings (DEC-006), and the
//! exogenous levers. Writes only the narrative domain.
//!
//! **Variety and pacing (DEC-097, ENGAGEMENT §4.1–4.3).** Four layers:
//! cooldowns (a floor of a year, growing with each use, no repeat within a
//! chapter, no climax two chapters running) decide what may return; tiers
//! (storyline stages and closing consequences, then the chapter's theme,
//! then the rest) and the keyed `pick` (relevance × novelty within a band)
//! decide what comes first; the pacing cycle (build-up, peak, peak fade,
//! relax, across chapters) decides when; `min_career_days` decides what is
//! introduced yet. Every draw is keyed, so the number of draws never depends
//! on what the player knows (DEC-091 §1.11).

use super::cast::{self, View};
use super::{
    Actor, AnswerRecord, CastEntry, Chapter, ChapterKind, Decision, DecisionId, DecisionStatus,
    DirectorPhase, FactKind, NarrativeState, PhaseKind, Reason, Resolution, RunEnd, StorylineRun,
    StorylineRunId, Warning, Weight,
};
use crate::characters::CharacterId;
use crate::events::EventKind;
use crate::places::Location;
use crate::scheduler::{Outs, Reads};
use crate::state::GamePhase;
use mandate_content::{
    Cast, Condition, ContentBundle, CycleLengths, DilemmaCategory, DilemmaDef, DirectorParams,
    ScenePlace, StorytellerDef, THEME_CAMPAIGN,
};
use mandate_core::{SimDate, Stream};
use serde::Serialize;

fn active(r: &Reads<'_>) -> bool {
    r.player.is_some() && r.calendar.phase != GamePhase::BurnIn
}

// --- Daily: fairness (DEC-006) ------------------------------------------------------------

/// The tracks the fairness rule watches: (key, value, in danger).
pub fn track_dangers(r: &Reads<'_>) -> Vec<(&'static str, f32, bool, f64)> {
    let Some(pl) = r.player else {
        return Vec::new();
    };
    let f = &r.content.director.fairness;
    let t = &pl.tracks;
    let low = |v: f32, warn: f64| ((warn - v as f64) / warn.max(1.0)).clamp(0.0, 1.0);
    let high = |v: f32, warn: f64| ((v as f64 - warn) / (100.0 - warn).max(1.0)).clamp(0.0, 1.0);
    vec![
        (
            "standing",
            t.standing,
            t.standing as f64 <= f.standing,
            low(t.standing, f.standing),
        ),
        (
            "viability",
            t.viability,
            t.viability as f64 <= f.viability,
            low(t.viability, f.viability),
        ),
        (
            "health",
            t.health,
            t.health as f64 <= f.health,
            low(t.health, f.health),
        ),
        (
            "legal_jeopardy",
            t.legal_jeopardy,
            t.legal_jeopardy as f64 >= f.legal_jeopardy,
            high(t.legal_jeopardy, f.legal_jeopardy),
        ),
        (
            "threat",
            t.threat,
            t.threat as f64 >= f.threat,
            high(t.threat, f.threat),
        ),
    ]
}

/// Narrative domain, Director phase, daily: a warning whenever a pressure
/// track reaches danger (repeated while it stays there). Warnings are never
/// budgeted: fairness comes before pacing.
pub fn daily(n: &mut NarrativeState, r: &Reads<'_>, out: &mut Outs<'_>) {
    if !active(r) {
        return;
    }
    let repeat = r.content.director.fairness.repeat_days;
    for (track, value, danger, _) in track_dangers(r) {
        if !danger {
            continue;
        }
        let recent = n
            .warnings
            .iter()
            .rev()
            .find(|w| w.track == track)
            .is_some_and(|w| w.date.add_days(repeat) > r.date);
        if recent {
            continue;
        }
        n.warnings.push(Warning {
            track: track.into(),
            date: r.date,
            value,
        });
        if n.warnings.len() > 100 {
            n.warnings.remove(0);
        }
        n.log(
            r.date,
            FactKind::TrackWarning {
                track: track.into(),
                value,
            },
            Vec::new(),
            true,
            0.8,
        );
        let ev = out.events.record(
            r.date,
            EventKind::TrackWarning {
                track: track.into(),
                value: value.round() as i32,
            },
            vec![],
        );
        out.events.pin(ev);
    }
}

// --- Weekly: the director ------------------------------------------------------------------

/// Budgets scaled by the storyteller profile.
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    pub climax: u8,
    pub sig_min: u8,
    pub sig_max: u8,
    pub minor_min: u8,
    pub minor_max: u8,
}

pub fn budget(d: &DirectorParams, p: Option<&StorytellerDef>) -> Budget {
    let k = p.map(|p| p.budget_scale).unwrap_or(1.0);
    let s = |x: u8| (x as f64 * k).round().clamp(0.0, 255.0) as u8;
    Budget {
        climax: d.budgets.climax,
        sig_min: s(d.budgets.significant[0]),
        sig_max: s(d.budgets.significant[1]).max(s(d.budgets.significant[0])),
        minor_min: s(d.budgets.minor[0]),
        minor_max: s(d.budgets.minor[1]).max(s(d.budgets.minor[0])),
    }
}

/// The tension curve at a chapter's progress `x`.
pub fn curve(d: &DirectorParams, x: f64) -> f64 {
    let t = &d.tension;
    if x <= t.peak_at {
        t.start + (t.peak - t.start) * x / t.peak_at.max(1e-6)
    } else {
        t.peak + (t.end - t.peak) * (x - t.peak_at) / (1.0 - t.peak_at).max(1e-6)
    }
}

/// Tension from the simulation alone (crisis, race, danger): what the
/// exogenous levers read, so that the spotlight never changes chance.
fn world_tension(v: &View<'_>, chapter: Option<&Chapter>) -> f64 {
    let t = &v.content().director.tension;
    let crisis = v
        .town_character()
        .and_then(|c| c.crisis())
        .map(|c| c.severity as f64)
        .unwrap_or(0.0);
    let race = if v.campaigning() {
        0.5 + 0.5 * chapter.map(|c| c.progress(v.date())).unwrap_or(0.0)
    } else {
        0.0
    };
    let danger = track_dangers(v.r).iter().map(|x| x.3).fold(0.0, f64::max);
    t.crisis_weight * crisis + t.race_weight * race + t.danger_weight * danger
}

fn pending_load(n: &NarrativeState) -> f64 {
    n.pending()
        .map(|d| match d.weight {
            Weight::Climax => 1.0,
            Weight::Significant => 0.5,
            Weight::Minor => 0.2,
        })
        .sum::<f64>()
        .min(1.0)
}

fn jitter(seed: u64, date: SimDate, key: &str, cast: &[CastEntry]) -> f64 {
    let first = cast
        .first()
        .map(|c| match c.actor {
            Actor::Person { id } => id.0 as u64,
            Actor::Neighborhood { id } | Actor::Policy { id } => id as u64 | (1 << 40),
        })
        .unwrap_or(0);
    mandate_core::rng::keyed(
        seed,
        Stream::Director,
        &[
            super::KEY_JITTER,
            date.0 as u64,
            mandate_core::hash::derive_u64(key, &[]),
            first,
        ],
    )
    .next_f64()
}

/// Relevance (DEC-033): connection to the player, their ward, the live
/// crisis, memories, storylines, pressure tracks, the coming election, and
/// the town's concerns; a template or person already spotlighted this
/// chapter weighs less.
fn relevance(
    v: &View<'_>,
    n: &NarrativeState,
    key: &str,
    cast: &[CastEntry],
    reasons: &[Reason],
) -> f64 {
    let d = &v.content().director;
    let w = &d.relevance;
    let mut s = w.base;
    for r in reasons {
        s += match r {
            Reason::Crisis { severity, .. } => w.crisis * *severity as f64,
            Reason::Concern { .. } => w.concern,
            Reason::Election { .. } => w.election,
            Reason::Storyline { .. } => w.storyline,
            Reason::Pressure { .. } => w.pressure,
            Reason::Ward => w.ward,
            Reason::Goal { .. } => w.player,
            Reason::Memory { .. } | Reason::Opportunity => 0.0,
        };
    }
    if let Some(me) = v.me() {
        let mut memories = 0usize;
        let mut known = false;
        for e in cast {
            if let Actor::Person { id } = e.actor {
                known |= v.pol.characters.edge(id, me).is_some()
                    || crate::characters::favour_balance(v.pol, id, me) != 0;
                memories +=
                    crate::characters::salient_memories(v.pol, id, Some(me), v.date(), 3).len();
            }
        }
        if known {
            s += w.player;
        }
        s += w.memory * memories.min(3) as f64;
    }
    s += w.jitter * jitter(v.r.meta.seed, v.date(), key, cast);
    if let Some(ch) = &n.chapter {
        // People spotlighted this chapter weigh less; templates cannot
        // repeat at all (`no_repeat_in_chapter`; off, they weigh less too).
        let repeated = (!d.no_repeat_in_chapter && ch.templates.iter().any(|t| t == key))
            || cast.iter().any(|e| match e.actor {
                Actor::Person { id } => ch.people.contains(&id),
                _ => false,
            });
        if repeated {
            s *= d.repeat_penalty;
        }
    }
    s
}

/// Which candidates the pick draws from first (ENGAGEMENT §4.2, after
/// Hades): story-carrying content is never starved by a generic template.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Tier {
    /// Storyline stages that are due, and consequences (`answered`) whose
    /// window closes within `tier_a_window_days`.
    A,
    /// Templates of the chapter's theme.
    B,
    /// The rest.
    C,
}

/// The cooldown a template's `uses`-th use sets (ENGAGEMENT §4.1):
/// `max(floor, cooldown) × growth^(uses − 1)` days.
pub fn cooldown_days(d: &DirectorParams, cooldown: u32, uses: u16) -> u32 {
    let base = cooldown.max(d.cooldown_floor_days) as f64;
    let grow = mandate_core::math::pow(d.cooldown_growth.max(1.0), uses.saturating_sub(1) as f64);
    (base * grow).round().min(36_500.0) as u32
}

/// A situation ready to be spotlighted.
pub(crate) struct Candidate<'c> {
    pub(crate) def: &'c DilemmaDef,
    pub(crate) cast: Vec<CastEntry>,
    pub(crate) reasons: Vec<Reason>,
    pub(crate) relevance: f64,
    pub(crate) goal: Option<(CharacterId, String)>,
    pub(crate) tier: Tier,
    /// A storyline stage's decision: the run (index into `storylines`).
    pub(crate) run: Option<usize>,
}

/// Roles an `answered` trigger carries over (`same_cast`), from the newest
/// answer it matches, while those actors still fill their roles.
fn answered_preset(v: &View<'_>, def: &DilemmaDef) -> Vec<CastEntry> {
    let mut out: Vec<CastEntry> = Vec::new();
    for c in &def.triggers {
        let Condition::Answered { same_cast, .. } = c else {
            continue;
        };
        if same_cast.is_empty() {
            continue;
        }
        let Some(rec) = cast::answered_records(v, c).first().copied() else {
            continue;
        };
        for r in same_cast {
            let (Some(role), Some(actor)) = (
                def.roles.iter().find(|x| &x.id == r),
                super::actor_of(&rec.cast, r),
            ) else {
                continue;
            };
            if !out.iter().any(|e| &e.role == r) && cast::still_cast(v, role, actor, &[], None) {
                out.push(CastEntry {
                    role: r.clone(),
                    actor,
                });
            }
        }
    }
    out
}

/// Days until the soonest `answered` window this cast satisfies closes.
fn answered_closes_in(v: &View<'_>, def: &DilemmaDef, cast: &[CastEntry]) -> Option<u32> {
    def.triggers
        .iter()
        .filter_map(|c| match c {
            Condition::Answered {
                max_days: Some(m), ..
            } => cast::answered_matches(v, c, cast)
                .into_iter()
                .map(|a| v.date().days_until(a.date.add_days(*m)))
                .min(),
            _ => None,
        })
        .min()
}

/// The template is a consequence of the player's own answers.
fn is_consequence(def: &DilemmaDef) -> bool {
    def.triggers
        .iter()
        .any(|c| matches!(c, Condition::Answered { .. }))
}

/// Cast a dilemma and check its triggers (plus any extra conditions); its
/// tier in the current chapter.
pub(crate) fn evaluate<'c>(
    v: &View<'_>,
    n: &NarrativeState,
    def: &'c DilemmaDef,
    preset: &[CastEntry],
    goal_holder: Option<CharacterId>,
    extra: &[Condition],
) -> Option<Candidate<'c>> {
    let ok = |c: &[CastEntry]| {
        def.triggers
            .iter()
            .chain(extra)
            .all(|x| cast::holds(v, x, c))
    };
    let mut preset = preset.to_vec();
    for e in answered_preset(v, def) {
        if !preset.iter().any(|p| p.role == e.role) {
            preset.push(e);
        }
    }
    let cast = cast::cast_search(v, &def.roles, &preset, goal_holder, 6, &ok)?;
    let mut reasons = cast::reasons(v, &def.triggers, &cast);
    if def.opportunity {
        reasons.push(Reason::Opportunity);
    }
    let relevance = relevance(v, n, &def.id, &cast, &reasons);
    let d = &v.content().director;
    let theme = n.chapter.as_ref().and_then(|c| c.theme.as_deref());
    let tier = if answered_closes_in(v, def, &cast).is_some_and(|x| x <= d.tier_a_window_days) {
        Tier::A
    } else if theme.is_some_and(|t| def.themes.iter().any(|x| x == t)) {
        Tier::B
    } else {
        Tier::C
    };
    Some(Candidate {
        def,
        cast,
        reasons,
        relevance,
        goal: None,
        tier,
        run: None,
    })
}

/// The pick (ENGAGEMENT §4.2): from the highest tier with a candidate
/// (opportunities first when easing off after a setback), those within
/// `pick_band` of the tier's best relevance, drawn by relevance × novelty
/// (`1 / (1 + uses)`) with one keyed draw (date, chapter, `salt`).
fn pick<'c>(
    v: &View<'_>,
    n: &NarrativeState,
    mut cands: Vec<Candidate<'c>>,
    opportunities_first: bool,
    salt: u64,
) -> Option<Candidate<'c>> {
    if cands.is_empty() {
        return None;
    }
    let d = &v.content().director;
    cands.sort_by(|a, b| a.def.id.cmp(&b.def.id).then(a.run.cmp(&b.run)));
    let items: Vec<Pickable> = cands
        .iter()
        .map(|c| Pickable {
            tier: c.tier,
            relevance: c.relevance,
            opportunity: c.def.opportunity,
            novelty: novelty(n, &c.def.id),
        })
        .collect();
    let chapter = n.chapter.as_ref().map(|c| c.index).unwrap_or(0);
    let u = mandate_core::rng::keyed(
        v.r.meta.seed,
        Stream::Director,
        &[super::KEY_PICK, v.date().0 as u64, chapter as u64, salt],
    )
    .next_f64();
    let i = pick_index(&items, d.pick_band, opportunities_first, u)?;
    Some(cands.swap_remove(i))
}

/// What the pick weighs of a candidate.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pickable {
    pub tier: Tier,
    pub relevance: f64,
    pub opportunity: bool,
    pub novelty: f64,
}

/// The pick's choice for a draw `u` in [0, 1): the highest tier present;
/// within it opportunities only when `opportunities_first` and there are
/// any; those within `band` of the best relevance; weighted by relevance ×
/// novelty, in the given order.
pub(crate) fn pick_index(
    items: &[Pickable],
    band: f64,
    opportunities_first: bool,
    u: f64,
) -> Option<usize> {
    let tier = items.iter().map(|c| c.tier).min()?;
    let opp = opportunities_first && items.iter().any(|c| c.tier == tier && c.opportunity);
    let pool: Vec<usize> = (0..items.len())
        .filter(|&i| items[i].tier == tier && (!opp || items[i].opportunity))
        .collect();
    let best = pool
        .iter()
        .map(|&i| items[i].relevance)
        .fold(f64::NEG_INFINITY, f64::max);
    let pool: Vec<usize> = pool
        .into_iter()
        .filter(|&i| items[i].relevance >= best - band)
        .collect();
    let weight = |i: usize| items[i].relevance.max(1e-6) * items[i].novelty;
    let total: f64 = pool.iter().map(|&i| weight(i)).sum();
    let mut at = u.clamp(0.0, 1.0) * total;
    for &i in &pool {
        at -= weight(i);
        if at < 0.0 {
            return Some(i);
        }
    }
    pool.last().copied()
}

/// `1 / (1 + uses)`: a template seen before weighs less.
fn novelty(n: &NarrativeState, key: &str) -> f64 {
    1.0 / (1.0 + n.uses.get(key).copied().unwrap_or(0) as f64)
}

/// Where a scene happens, in the player's town.
pub fn scene_place(v: &View<'_>, scene: &ScenePlace, cast: &[CastEntry]) -> Option<Location> {
    let m = v.town()?;
    let kind_place = |kind: &str| -> Option<Location> {
        if kind == "streets" {
            let hood = cast
                .iter()
                .find_map(|e| match e.actor {
                    Actor::Neighborhood { id } => Some(id),
                    _ => None,
                })
                .or_else(|| {
                    v.player().and_then(|pl| {
                        crate::policy::ward_hoods(v.r.world, pl.ward)
                            .first()
                            .map(|h| *h as u32)
                    })
                })?;
            return Some(Location::Street {
                neighborhood: crate::world::NeighborhoodId(hood),
            });
        }
        v.r.world
            .places
            .iter()
            .find(|p| p.municipality.0 == m && p.kind.key() == kind)
            .map(|p| Location::Place { place: p.id })
    };
    match scene {
        ScenePlace::Workplace { role, fallback } => super::person_of(cast, role)
            .and_then(|p| v.pol.characters.get(p))
            .and_then(|c| c.named.as_deref())
            .and_then(|nc| nc.place)
            .filter(|p| {
                v.r.world
                    .places
                    .get(p.index())
                    .is_some_and(|x| x.municipality.0 == m)
            })
            .map(|place| Location::Place { place })
            .or_else(|| kind_place(fallback)),
        ScenePlace::Kind { kind } => kind_place(kind),
    }
}

#[allow(clippy::too_many_arguments)]
fn open(
    n: &mut NarrativeState,
    v: &View<'_>,
    out: &mut Outs<'_>,
    c: Candidate<'_>,
    weight: Weight,
    storyline: Option<StorylineRunId>,
    about: Option<super::About>,
) -> DecisionId {
    let d = &v.content().director;
    let date = v.date();
    let id: DecisionId = n.decision_ids.alloc();
    let mut deadline = date.add_days(c.def.deadline_days.unwrap_or(d.deadline_days));
    // A vote cannot wait for a decision: decide before the bill's next stage.
    for e in &c.cast {
        let is_bill = c
            .def
            .roles
            .iter()
            .any(|r| r.id == e.role && matches!(r.cast, Cast::Bill { .. }));
        if let (true, Actor::Policy { id }) = (is_bill, e.actor)
            && let Some(next) =
                v.r.policy()
                    .instances
                    .get(id as usize)
                    .and_then(|p| p.next_stage_on)
        {
            let by = SimDate(next.0.saturating_sub(1)).max(date.add_days(1));
            deadline = deadline.min(by);
        }
    }
    let place = c
        .def
        .scene
        .as_deref()
        .and_then(|s| v.content().scene(s))
        .and_then(|s| scene_place(v, &s.place, &c.cast));
    let people: Vec<CharacterId> = c
        .cast
        .iter()
        .filter_map(|e| match e.actor {
            Actor::Person { id } => Some(id),
            _ => None,
        })
        .collect();
    // A decision names its cast (DEC-091, §1.3): the player hears of them;
    // someone who comes to the player with a request is met (`known`).
    // Effect requests to the player domain, posted only when there is one.
    if v.player().is_some() {
        let holder = c.goal.as_ref().map(|(h, _)| *h);
        for person in super::learn::cast_people(v, &c.cast) {
            out.effects.push(crate::scheduler::Effect::Learn {
                subject: crate::player::knowledge::Subject::Person { person },
                level: if Some(person) == holder {
                    crate::player::knowledge::Level::Known
                } else {
                    crate::player::knowledge::Level::Heard
                },
                source: crate::player::knowledge::Source::Decision { decision: id },
            });
        }
    }
    let actors: Vec<Actor> = c.cast.iter().map(|e| e.actor).collect();
    let chapter_index = n.chapter.as_ref().map(|ch| ch.index).unwrap_or(0);
    // A raised decision (a story, an investigation) is fairness, not pacing:
    // it never counts against the chapter's budgets (DEC-006).
    if let Some(ch) = n.chapter.as_mut().filter(|_| about.is_none()) {
        match weight {
            Weight::Climax => {
                ch.climaxes += 1;
                ch.climax = Some(id);
                ch.last_decision = Some(date);
            }
            Weight::Significant => {
                ch.significant += 1;
                ch.last_decision = Some(date);
            }
            Weight::Minor => {
                ch.minor += 1;
                ch.last_minor = Some(date);
            }
        }
        ch.templates.push(c.def.id.clone());
        ch.people.extend(people);
    }
    if about.is_none() {
        // Variety (ENGAGEMENT §4.1): each use cools the template longer.
        let uses = n.uses.entry(c.def.id.clone()).or_insert(0);
        *uses = uses.saturating_add(1);
        let cd = cooldown_days(d, c.def.cooldown_days, *uses);
        n.cooldowns.insert(c.def.id.clone(), date.add_days(cd));
        if weight == Weight::Climax {
            n.climaxes.push((chapter_index, c.def.id.clone()));
            if n.climaxes.len() > super::CLIMAXES_CAP {
                n.climaxes.remove(0);
            }
        }
        if let Some(ph) = n.phase.as_mut() {
            match weight {
                Weight::Minor => ph.beats = ph.beats.saturating_add(1),
                _ if ph.kind == PhaseKind::Peak && ph.decision.is_none() => ph.decision = Some(id),
                _ => {}
            }
        }
    }
    let news = (c.relevance / 3.0).clamp(0.0, 1.0) as f32;
    n.decisions.push(Decision {
        id,
        template: c.def.id.clone(),
        weight,
        cast: c.cast,
        reasons: c.reasons,
        opened: date,
        deadline,
        chapter: chapter_index,
        relevance: c.relevance as f32,
        storyline,
        goal: c.goal,
        place,
        status: DecisionStatus::Pending,
        resolution: None,
        about,
    });
    n.log(
        date,
        FactKind::DecisionOpened { decision: id },
        actors,
        true,
        news,
    );
    out.events.record(
        date,
        EventKind::DecisionOpened {
            decision: id.0,
            template: c.def.id.clone(),
        },
        vec![],
    );
    id
}

/// When a session's climax must have come by: the end of its budget year,
/// or the opening of the next campaign if that comes first (a campaign
/// closes the session, and its own climax is the election).
pub fn climax_horizon(v: &View<'_>, ch: &Chapter) -> SimDate {
    let opens = v
        .content()
        .constitutions
        .get(&v.r.meta.constitution)
        .and_then(|c| crate::politics::elections::next_cycle(c, ch.start))
        .map(|(day, weeks)| SimDate(day.0.saturating_sub(weeks as u32 * 7)));
    match opens {
        Some(o) if o > ch.start && o < ch.end => o,
        _ => ch.end,
    }
}

/// Share of a session's way to its climax horizon (0..1).
fn climax_progress(v: &View<'_>, ch: &Chapter, on: SimDate) -> f64 {
    let total = ch.start.days_until(climax_horizon(v, ch)).max(1) as f64;
    (ch.start.days_until(on) as f64 / total).clamp(0.0, 1.0)
}

/// A session chapter is in its climax window and has no climax yet.
fn climax_due(n: &NarrativeState, v: &View<'_>, b: Budget) -> bool {
    let Some(ch) = n.chapter.as_ref() else {
        return false;
    };
    let x = climax_progress(v, ch, v.date());
    let w = &v.content().director.tension.climax_window;
    ch.kind == ChapterKind::Session && ch.climaxes < b.climax && x >= w[0] && x <= w[1]
}

fn can_decide(n: &NarrativeState, d: &DirectorParams, date: SimDate) -> bool {
    let pending = n.pending().count();
    // Spaced across chapter boundaries too.
    let last = n
        .decisions
        .iter()
        .filter(|x| x.weight != Weight::Minor)
        .map(|x| x.opened)
        .max();
    let spaced = last.is_none_or(|l| l.add_days(d.spacing_days) <= date);
    pending < d.max_pending as usize && spaced
}

/// Paced through the chapter: by progress `x`, at most `max × x + 2` (so a
/// budget is not spent in the first weeks).
fn paced(count: u8, max: u8, x: f64) -> bool {
    count < max && (count as f64) < (max as f64 * x).ceil() + 2.0
}

fn can_minor(n: &NarrativeState, d: &DirectorParams, b: Budget, date: SimDate) -> bool {
    n.chapter.as_ref().is_some_and(|c| {
        paced(c.minor, b.minor_max, c.progress(date))
            && c.last_minor
                .is_none_or(|l| l.add_days(d.development_spacing_days) <= date)
    })
}

/// Room for a minor beat; `force` (the quiet rescue) waives pacing and
/// spacing, never the budget.
fn minor_room(
    n: &NarrativeState,
    d: &DirectorParams,
    b: Budget,
    date: SimDate,
    force: bool,
) -> bool {
    if force {
        n.chapter.as_ref().is_some_and(|c| c.minor < b.minor_max)
    } else {
        can_minor(n, d, b, date)
    }
}

/// The jurisdiction whose budget year a session chapter runs over (DEC-098):
/// that of the player's main post, else of their home town.
pub fn session_jurisdiction(v: &View<'_>) -> Option<crate::world::Jurisdiction> {
    let pl = v.player()?;
    crate::politics::posts_of(v.pol, pl.politician)
        .first()
        .map(|p| p.at())
        .or(Some(crate::world::Jurisdiction::town(pl.home())))
}

/// The next turn of `at`'s budget year that leaves a session at least
/// `min_days` long. Every treasury, whatever its tier, closes its books on
/// 1 January (`finance::yearly`); a tier with its own fiscal year would be
/// read here.
pub fn session_end(
    at: Option<crate::world::Jurisdiction>,
    start: SimDate,
    min_days: u32,
) -> SimDate {
    let _ = at;
    let mut y = start.calendar().year + 1;
    loop {
        let d = SimDate::from_ymd(y, 1, 1);
        if start.days_until(d) >= min_days {
            return d;
        }
        y += 1;
    }
}

/// The towns of the player's focus (DEC-098): the jurisdictions simulated
/// in detail around them.
fn focus_towns(v: &View<'_>) -> Vec<u32> {
    v.player()
        .map(|pl| pl.focus())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|j| match j {
            crate::world::Jurisdiction::Municipality { id } => Some(id.0),
            _ => None,
        })
        .collect()
}

/// Whether the live state supports a theme (ENGAGEMENT §4.2). It reads live
/// state, never the ledger: a scheme counts whether the player knows of it.
fn theme_eligible(n: &NarrativeState, v: &View<'_>, theme: &str) -> bool {
    let content = v.content();
    let towns = focus_towns(v);
    let crises = || {
        towns.iter().flat_map(|m| {
            v.pol
                .characters
                .towns
                .get(*m as usize)
                .map(|t| t.crises.as_slice())
                .unwrap_or_default()
                .iter()
                .filter(|c| c.severity > 0.0)
        })
    };
    match theme {
        "crisis" => crises().next().is_some(),
        // Fiscal stress: a budget-stress crisis, or a deficit last year.
        "budget" => {
            crises().any(|c| {
                content
                    .crises
                    .iter()
                    .find(|k| k.id == c.kind)
                    .is_some_and(|k| k.indicator == mandate_content::Indicator::BudgetStress)
            }) || towns.iter().any(|m| {
                v.r.finance()
                    .local
                    .get(*m as usize)
                    .is_some_and(|t| t.last_year.year > 0 && t.last_year.balance.0 < 0)
            })
        }
        "scheme" => n.active_runs().any(|r| {
            content
                .storyline(&r.template)
                .is_some_and(|s| s.themes.iter().any(|t| t == "scheme"))
        }),
        "local_life" => true,
        _ => false,
    }
}

/// A chapter's theme (ENGAGEMENT §4.2, after Reigns): campaigns take
/// `campaign`; a session draws among the themes the live state supports, by
/// weight, never the last session's while another is eligible. Returns the
/// theme and how many were eligible.
fn draw_theme(
    n: &NarrativeState,
    v: &View<'_>,
    kind: ChapterKind,
    index: u32,
) -> (Option<String>, u8) {
    if kind == ChapterKind::Campaign {
        return (Some(THEME_CAMPAIGN.into()), 1);
    }
    let d = &v.content().director;
    let eligible: Vec<(&str, f64)> = d
        .themes
        .iter()
        .filter(|(k, w)| **w > 0.0 && theme_eligible(n, v, k))
        .map(|(k, w)| (k.as_str(), *w))
        .collect();
    let last = n
        .chapters
        .iter()
        .rev()
        .find(|c| c.kind == ChapterKind::Session)
        .and_then(|c| c.theme.clone());
    let fresh: Vec<(&str, f64)> = eligible
        .iter()
        .copied()
        .filter(|(k, _)| last.as_deref() != Some(*k))
        .collect();
    let pool = if fresh.is_empty() { &eligible } else { &fresh };
    let total: f64 = pool.iter().map(|x| x.1).sum();
    let u = mandate_core::rng::keyed(
        v.r.meta.seed,
        Stream::Director,
        &[super::KEY_THEME, index as u64],
    )
    .next_f64();
    let mut at = u * total;
    let mut chosen = pool.last().map(|x| x.0);
    for (k, w) in pool {
        at -= w;
        if at < 0.0 {
            chosen = Some(k);
            break;
        }
    }
    (
        chosen.map(String::from),
        eligible.len().min(u8::MAX as usize) as u8,
    )
}

/// Open the next chapter when the player's situation changes (a campaign
/// begins or ends) or a session's budget year turns. Returns whether the
/// kind of chapter changed.
fn chapters(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>) -> bool {
    let date = v.date();
    let d = &v.content().director;
    let want = if v.campaigning() {
        ChapterKind::Campaign
    } else {
        ChapterKind::Session
    };
    let change = match &n.chapter {
        None => true,
        Some(c) => c.kind != want || (c.kind == ChapterKind::Session && date >= c.end),
    };
    if !change {
        return false;
    }
    let kind_changed = n.chapter.as_ref().is_some_and(|c| c.kind != want);
    let mut index = 1;
    if let Some(mut c) = n.chapter.take() {
        index = c.index + 1;
        // A campaign's climax is the election (DEC-005).
        if c.kind == ChapterKind::Campaign
            && let Some(pl) = v.player()
        {
            let ran = v.pol.elections.results.iter().any(|res| {
                res.date >= c.start
                    && res.date <= date
                    && res.candidates.iter().any(|x| x.politician == pl.politician)
            });
            if ran {
                c.climaxes += 1;
            }
        }
        c.closed = Some(date);
        n.chapters.push(c);
    }
    let end = match want {
        ChapterKind::Session => session_end(session_jurisdiction(v), date, d.session_min_days),
        ChapterKind::Campaign => v
            .next_election()
            .filter(|e| *e > date)
            .unwrap_or(date.add_days(60)),
    };
    let mut ch = Chapter::new(index, want, date, end);
    (ch.theme, ch.theme_options) = draw_theme(n, v, want, index);
    n.chapter = Some(ch);
    n.log(
        date,
        FactKind::ChapterBegan { index, kind: want },
        Vec::new(),
        true,
        0.3,
    );
    out.events.record(
        date,
        EventKind::ChapterBegan {
            index,
            kind: want.key().into(),
        },
        vec![],
    );
    kind_changed
}

/// The exogenous levers (DEC-033 B), from the simulation alone.
fn chance(n: &mut NarrativeState, v: &View<'_>, profile: Option<&StorytellerDef>) {
    let (Some(m), Some(p)) = (v.town(), profile) else {
        n.chance = Default::default();
        return;
    };
    let d = &v.content().director;
    let cap = p.crisis_cap.unwrap_or(d.crisis_cap) as usize;
    let crises = v
        .town_character()
        .map(|t| t.crises.iter().filter(|c| c.severity > 0.0).count())
        .unwrap_or(0);
    let at_cap = crises >= cap;
    let until = v.date().add_days(d.chance.hazard_hold_days);
    n.chance.hazard_hold = (p.defer_hazards && at_cap).then_some((m, until));
    n.chance.hazard_keep = (p.hazard_rate < 1.0).then_some((m, p.hazard_rate as f32));
    n.chance.recession_hold =
        (p.hold_recessions && at_cap && d.chance.recession_hold_months > 0).then_some(until);
}

/// Discovery timing of hidden facts within their evidence windows (DEC-033
/// B): a quiet chapter draws them earlier, a tense one later.
fn discovery(
    n: &mut NarrativeState,
    v: &View<'_>,
    out: &mut Outs<'_>,
    profile: Option<&StorytellerDef>,
    quiet: bool,
) {
    let date = v.date();
    let shape = n.shape_discovery && profile.is_some_and(|p| p.tension_curve);
    let seed = v.r.meta.seed;
    let mut surfaced = Vec::new();
    for h in n.hidden.iter_mut() {
        if h.discoverable_on.is_none() {
            let mut u = mandate_core::rng::keyed(
                seed,
                Stream::Director,
                &[super::KEY_DISCOVERY, h.id as u64],
            )
            .next_f64();
            if shape {
                u = if quiet { u * 0.5 } else { 0.5 + u * 0.5 };
            }
            let span = h.window[0].days_until(h.window[1]) as f64;
            h.discoverable_on = Some(h.window[0].add_days((u * span) as u32));
        }
        if !h.surfaced && h.discoverable_on.is_some_and(|d| d <= date) {
            h.surfaced = true;
            surfaced.push((h.id, h.actors.clone()));
        }
    }
    for (id, actors) in surfaced {
        n.log(
            date,
            FactKind::Discoverable { hidden: id },
            actors,
            false,
            0.6,
        );
        out.events
            .record(date, EventKind::FactDiscoverable { hidden: id }, vec![]);
    }
}

/// Does a pending decision's situation still exist? A bill that was voted
/// on, a vote the player can no longer cast, or a person who is gone.
fn still_possible(v: &View<'_>, d: &Decision) -> bool {
    let Some(def) = v.content().dilemma(&d.template) else {
        return false;
    };
    if !cast::actors_exist(v, &d.cast) {
        return false;
    }
    for r in &def.roles {
        if let (Cast::Bill { .. }, Some(Actor::Policy { id })) =
            (&r.cast, super::actor_of(&d.cast, &r.id))
        {
            let open =
                v.r.policy()
                    .instances
                    .get(id as usize)
                    .is_some_and(|p| p.status == crate::policy::PolicyStatus::UnderConsideration);
            if !open {
                return false;
            }
        }
    }
    def.triggers.iter().all(|c| match c {
        Condition::CanVote { .. } => cast::holds(v, c, &d.cast),
        _ => true,
    })
}

fn lapse(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>) {
    let date = v.date();
    let gone: Vec<DecisionId> = n
        .pending()
        .filter(|d| !still_possible(v, d))
        .map(|d| d.id)
        .collect();
    for id in gone {
        let mut run = None;
        if let Some(d) = n.decisions.iter_mut().find(|d| d.id == id) {
            d.status = DecisionStatus::Lapsed;
            d.resolution = Some(Resolution {
                option: String::new(),
                date,
                by_default: false,
                applied: Vec::new(),
            });
            run = d.storyline;
            out.events.record(
                date,
                EventKind::DecisionLapsed {
                    decision: id.0,
                    template: d.template.clone(),
                },
                vec![],
            );
        }
        // A storyline stage whose decision lapsed waits for another chance.
        if let Some(rid) = run
            && let Some(r) = n.storylines.iter_mut().find(|r| r.id == rid)
        {
            r.decision = None;
            r.entered = false;
        }
        n.log(
            date,
            FactKind::DecisionLapsed { decision: id },
            Vec::new(),
            false,
            0.2,
        );
    }
}

/// End a storyline run: cooldown, and its follow-up hook.
pub(crate) fn end_run(
    n: &mut NarrativeState,
    content: &ContentBundle,
    i: usize,
    date: SimDate,
    how: RunEnd,
) {
    let (template, cast, id) = {
        let r = &mut n.storylines[i];
        r.ended = Some((date, how));
        r.decision = None;
        (r.template.clone(), r.cast.clone(), r.id)
    };
    if let Some(def) = content.storyline(&template) {
        n.cooldowns
            .insert(template.clone(), date.add_days(def.cooldown_days));
        if how == RunEnd::Ended
            && let Some(f) = &def.follow_up
            && let Some(fd) = content.storyline(f)
        {
            n.follow_ups
                .push((f.clone(), cast, date.add_days(fd.cooldown_days)));
        }
    }
    n.log(
        date,
        FactKind::StorylineEnded { run: id, end: how },
        Vec::new(),
        true,
        0.4,
    );
}

fn start_runs(n: &mut NarrativeState, v: &View<'_>) {
    let content = v.content();
    let date = v.date();
    let cap = content.director.storylines_active as usize;
    // Follow-ups first (the rival arc follows the player), then new arcs.
    let mut starts: Vec<(String, Vec<CastEntry>)> = Vec::new();
    let mut keep_follow = Vec::new();
    for (k, preset, from) in std::mem::take(&mut n.follow_ups) {
        let Some(def) = content.storyline(&k) else {
            continue;
        };
        if from > date || n.active_runs().count() + starts.len() >= cap {
            keep_follow.push((k, preset, from));
            continue;
        }
        if n.active_runs().any(|r| r.template == k) {
            keep_follow.push((k, preset, from));
            continue;
        }
        // Recast whoever no longer fills their role (a new rival).
        let preset: Vec<CastEntry> = preset
            .into_iter()
            .filter(|e| {
                def.roles
                    .iter()
                    .find(|r| r.id == e.role)
                    .is_some_and(|r| cast::still_cast(v, r, e.actor, &[], None))
            })
            .collect();
        match cast::cast_roles(v, &def.roles, &preset, None) {
            Some(c) if def.start.iter().all(|x| cast::holds(v, x, &c)) => starts.push((k, c)),
            _ => keep_follow.push((k, preset, from)),
        }
    }
    n.follow_ups = keep_follow;
    for def in &content.storylines {
        if n.active_runs().count() + starts.len() >= cap {
            break;
        }
        if n.active_runs().any(|r| r.template == def.id) || starts.iter().any(|s| s.0 == def.id) {
            continue;
        }
        if n.cooldowns.get(&def.id).is_some_and(|d| *d > date) {
            continue;
        }
        if n.follow_ups.iter().any(|f| f.0 == def.id) {
            continue;
        }
        if career_days(n, date) < def.min_career_days {
            continue;
        }
        if let Some(c) = cast::cast_roles(v, &def.roles, &[], None)
            && def.start.iter().all(|x| cast::holds(v, x, &c))
        {
            starts.push((def.id.clone(), c));
        }
    }
    for (k, c) in starts {
        let uses = n.uses.entry(k.clone()).or_insert(0);
        *uses = uses.saturating_add(1);
        let id: StorylineRunId = n.run_ids.alloc();
        n.storylines.push(StorylineRun {
            id,
            template: k,
            cast: c,
            stage: 0,
            entered: false,
            since: date,
            started: date,
            decision: None,
            stages: Vec::new(),
            ended: None,
        });
    }
}

/// A storyline run whose current stage holds a decision that is due: the
/// decision as a tier-A candidate, cast from the run.
fn staged_candidate<'c>(v: &View<'c>, n: &NarrativeState, i: usize) -> Option<Candidate<'c>> {
    let content = v.content();
    let d = &content.director;
    let r = n.storylines.get(i)?;
    let def = content.storyline(&r.template)?;
    let stage = def.stages.get(r.stage as usize)?;
    let ddef = content.dilemma(stage.decision.as_deref()?)?;
    if d.no_repeat_in_chapter
        && n.chapter
            .as_ref()
            .is_some_and(|ch| ch.templates.contains(&ddef.id))
    {
        return None;
    }
    let mut c = evaluate(v, n, ddef, &r.cast, None, &[])?;
    c.reasons.push(Reason::Storyline {
        storyline: def.id.clone(),
    });
    c.relevance += d.relevance.storyline;
    c.tier = Tier::A;
    c.run = Some(i);
    Some(c)
}

/// Every due stage's decision as a candidate.
fn staged_all<'c>(v: &View<'c>, n: &NarrativeState, due: &[usize]) -> Vec<Candidate<'c>> {
    due.iter()
        .filter_map(|&i| staged_candidate(v, n, i))
        .collect()
}

/// Open a storyline stage's decision and enter the stage.
fn open_staged(
    n: &mut NarrativeState,
    v: &View<'_>,
    out: &mut Outs<'_>,
    c: Candidate<'_>,
    weight: Weight,
) {
    let Some(i) = c.run else { return };
    let date = v.date();
    let (run_id, stage_id) = {
        let r = &n.storylines[i];
        let stage = v
            .content()
            .storyline(&r.template)
            .and_then(|s| s.stages.get(r.stage as usize))
            .map(|s| s.id.clone())
            .unwrap_or_default();
        (r.id, stage)
    };
    let merged = c.cast.clone();
    let id = open(n, v, out, c, weight, Some(run_id), None);
    let r = &mut n.storylines[i];
    r.cast = merged;
    r.decision = Some(id);
    r.entered = true;
    r.stages.push((stage_id.clone(), date));
    let actors = r.cast.iter().map(|e| e.actor).collect();
    n.log(
        date,
        FactKind::StorylineStage {
            run: run_id,
            stage: stage_id,
        },
        actors,
        true,
        0.5,
    );
}

/// A storyline run's turn this week (pure: the lab reads it too).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunStep {
    /// Nothing to do (ended, waiting, or its conditions do not hold).
    Wait,
    /// Its template (or its stage's decision) was removed by an update.
    Removed,
    /// It ran out of stages.
    Ended,
    /// A stage timed out, or someone no longer fills their role.
    Faded,
    /// Its next stage is a decision that is due.
    Due,
    /// Its next stage, without a decision, is reached (a minor beat).
    Beat,
}

fn run_step(v: &View<'_>, n: &NarrativeState, i: usize) -> RunStep {
    let content = v.content();
    let date = v.date();
    let Some(r) = n.storylines.get(i).filter(|r| r.ended.is_none()) else {
        return RunStep::Wait;
    };
    let Some(def) = content.storyline(&r.template) else {
        return RunStep::Removed;
    };
    if r.decision.is_some() || r.entered {
        return RunStep::Wait;
    }
    let Some(stage) = def.stages.get(r.stage as usize) else {
        return RunStep::Ended;
    };
    let ready_from = r.since.add_days(stage.wait_days);
    if date < ready_from {
        return RunStep::Wait;
    }
    if stage
        .timeout_days
        .is_some_and(|t| date > ready_from.add_days(t))
    {
        return RunStep::Faded;
    }
    // The arc is about these people: if one no longer fills their role
    // (a new rival, a debt repaid another way), it fades.
    let still = def.roles.iter().all(|role| {
        super::actor_of(&r.cast, &role.id)
            .is_some_and(|a| cast::still_cast(v, role, a, &r.cast, None))
    });
    if !still {
        return RunStep::Faded;
    }
    if !stage.advance.iter().all(|c| cast::holds(v, c, &r.cast)) {
        return RunStep::Wait;
    }
    match &stage.decision {
        Some(dk) if content.dilemma(dk).is_none() => RunStep::Removed,
        Some(_) => RunStep::Due,
        None => RunStep::Beat,
    }
}

/// Storyline runs: fades, timeouts and stages without a decision (minor
/// beats, in any phase). Returns the runs whose next stage is a decision
/// that is due: the spotlight presents them (at a peak, tier A).
fn advance_runs(
    n: &mut NarrativeState,
    v: &View<'_>,
    out: &mut Outs<'_>,
    b: Budget,
    force: bool,
) -> Vec<usize> {
    let content = v.content();
    let d = &content.director;
    let date = v.date();
    let mut due = Vec::new();
    for i in 0..n.storylines.len() {
        match run_step(v, n, i) {
            RunStep::Wait => continue,
            RunStep::Removed => {
                end_run(n, content, i, date, RunEnd::Removed);
                continue;
            }
            RunStep::Ended => {
                end_run(n, content, i, date, RunEnd::Ended);
                continue;
            }
            RunStep::Faded => {
                end_run(n, content, i, date, RunEnd::Faded);
                continue;
            }
            RunStep::Due => {
                due.push(i);
                continue;
            }
            RunStep::Beat => {}
        }
        let Some(def) = content.storyline(&n.storylines[i].template) else {
            continue;
        };
        let Some(stage) = def.stages.get(n.storylines[i].stage as usize) else {
            continue;
        };
        let run_id = n.storylines[i].id;
        {
            if !minor_room(n, d, b, date, force) {
                continue;
            }
            if let Some(ch) = n.chapter.as_mut() {
                ch.minor += 1;
                ch.last_minor = Some(date);
            }
            if let Some(ph) = n.phase.as_mut() {
                ph.beats = ph.beats.saturating_add(1);
            }
            let r = &mut n.storylines[i];
            r.stages.push((stage.id.clone(), date));
            let actors: Vec<Actor> = r.cast.iter().map(|e| e.actor).collect();
            let ends = stage.ends;
            if !ends {
                r.stage += 1;
                r.entered = false;
                r.since = date;
            }
            n.log(
                date,
                FactKind::StorylineStage {
                    run: run_id,
                    stage: stage.id.clone(),
                },
                actors,
                true,
                0.4,
            );
            out.events.record(
                date,
                EventKind::StorylineStage {
                    run: run_id.0,
                    storyline: def.id.clone(),
                    stage: stage.id.clone(),
                },
                vec![],
            );
            if ends || n.storylines[i].stage as usize >= def.stages.len() {
                end_run(n, content, i, date, RunEnd::Ended);
            }
        }
    }
    due
}

/// Without a pacing cycle (Unfiltered): a storyline's turn has priority. It
/// waits for room (pending decisions, the budget), not for the spacing
/// between decisions; in the climax window it is the chapter's climax.
fn open_due_stages(
    n: &mut NarrativeState,
    v: &View<'_>,
    out: &mut Outs<'_>,
    b: Budget,
    due: &[usize],
) {
    let d = &v.content().director;
    for &i in due {
        let sig = n.chapter.as_ref().map(|c| c.significant).unwrap_or(0);
        if n.pending().count() >= d.max_pending as usize
            || (sig >= b.sig_max && !climax_due(n, v, b))
        {
            continue;
        }
        let Some(c) = staged_candidate(v, n, i) else {
            continue;
        };
        let weight = if climax_due(n, v, b) && climax_allowed(n, d, &c.def.id) {
            Weight::Climax
        } else {
            Weight::Significant
        };
        open_staged(n, v, out, c, weight);
    }
}

/// Decisions raised by a story about the player or an investigation of
/// them (DEC-034, DEC-006): fairness first, so never budgeted, spaced, or
/// capped; each story and case is raised once.
fn raise(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>) {
    let Some(pl) = v.player() else { return };
    if !pl.status.active() {
        return;
    }
    let content = v.content();
    let me = pl.politician;
    let mut todo: Vec<(super::About, mandate_content::RaisedBy, &'static str)> = Vec::new();
    for s in v.r.media().scandals.iter() {
        let a = super::About::Scandal { id: s.id };
        if s.subject == me && s.running() && s.response.is_none() && !n.raised.contains(&a) {
            todo.push((a, mandate_content::RaisedBy::Scandal, "standing"));
        }
    }
    for c in v.r.justice().cases.iter() {
        let a = super::About::Case { id: c.id };
        if c.subject == me
            && c.stage == crate::justice::Stage::Investigation
            && c.response.is_none()
            && !n.raised.contains(&a)
        {
            todo.push((
                a,
                mandate_content::RaisedBy::Investigation,
                "legal_jeopardy",
            ));
        }
    }
    for (about, by, track) in todo {
        let Some(def) = content.dilemmas.iter().find(|d| d.raised_by == Some(by)) else {
            continue;
        };
        let Some(mut c) = evaluate(v, n, def, &[], None, &[]) else {
            continue;
        };
        c.reasons.push(Reason::Pressure {
            track: track.into(),
        });
        open(n, v, out, c, Weight::Significant, None, Some(about));
        // A scandal about the player is a setback: the director eases off.
        if matches!(about, super::About::Scandal { .. }) && n.phase.is_some() {
            n.setback = Some(v.date());
        }
        n.raised.push(about);
        if n.raised.len() > 200 {
            n.raised.remove(0);
        }
    }
}

/// Requests (DEC-032): a goal that needs the player (a minor beat, in any
/// phase). `force`: the quiet rescue (pacing and spacing waived, never the
/// budget). Returns whether one opened.
fn requests(
    n: &mut NarrativeState,
    v: &View<'_>,
    out: &mut Outs<'_>,
    b: Budget,
    force: bool,
) -> bool {
    let content = v.content();
    let d = &content.director;
    let date = v.date();
    if !minor_room(n, d, b, date, force) || n.pending().count() >= d.max_pending as usize {
        return false;
    }
    let mut cands: Vec<Candidate<'_>> = Vec::new();
    for g in &v.pol.characters.goals {
        let Some(rk) = content.goal(&g.goal).and_then(|x| x.request.as_deref()) else {
            continue;
        };
        if n.requests_raised
            .iter()
            .any(|(h, k, s)| *h == g.holder && *k == g.goal && *s == g.since)
        {
            continue;
        }
        let Some(rdef) = content.dilemma(rk) else {
            continue;
        };
        if !available(n, d, rdef, date, false) {
            continue;
        }
        let Some(mut c) = evaluate(v, n, rdef, &[], Some(g.holder), &[]) else {
            continue;
        };
        c.reasons.push(Reason::Goal {
            person: g.holder,
            goal: g.goal.clone(),
        });
        c.relevance += d.relevance.player;
        c.goal = Some((g.holder, g.goal.clone()));
        if c.relevance < d.min_relevance {
            continue;
        }
        // One candidate per request: the holder it matters most to.
        match cands.iter_mut().find(|x| x.def.id == c.def.id) {
            Some(x) if x.relevance >= c.relevance => {}
            Some(x) => *x = c,
            None => cands.push(c),
        }
    }
    let picked = pick(v, n, cands, false, 1);
    let opened = picked.is_some();
    if let Some(c) = picked {
        let (h, k) = c.goal.clone().unwrap_or((CharacterId(0), String::new()));
        let since = v
            .pol
            .characters
            .goals
            .iter()
            .find(|g| g.holder == h && g.goal == k)
            .map(|g| g.since)
            .unwrap_or(date);
        open(n, v, out, c, Weight::Minor, None, None);
        n.requests_raised.push((h, k, since));
        if n.requests_raised.len() > 200 {
            n.requests_raised.remove(0);
        }
    }
    opened
}

/// May the spotlight present this template now (ENGAGEMENT §4.1, §4.7)?
/// Its cooldown has passed (unless `relax_cooldowns`: the climax fallback),
/// it is not pending, it was not spotlighted this chapter, and the career is
/// old enough.
fn available(
    n: &NarrativeState,
    d: &DirectorParams,
    def: &DilemmaDef,
    date: SimDate,
    relax_cooldowns: bool,
) -> bool {
    if (!relax_cooldowns && n.cooldowns.get(&def.id).is_some_and(|x| *x > date))
        || n.pending().any(|p| p.template == def.id)
    {
        return false;
    }
    if d.no_repeat_in_chapter
        && n.chapter
            .as_ref()
            .is_some_and(|c| c.templates.contains(&def.id))
    {
        return false;
    }
    career_days(n, date) >= def.min_career_days
}

/// Days since play began (`min_career_days`).
fn career_days(n: &NarrativeState, date: SimDate) -> u32 {
    n.play_began
        .filter(|b| *b <= date)
        .map(|b| b.days_until(date))
        .unwrap_or(0)
}

/// A template that was the climax in one of the last
/// `climax_memory_chapters` chapters cannot be the climax again (nor, ever,
/// the previous chapter's: the fallback relies on it).
fn climax_allowed(n: &NarrativeState, d: &DirectorParams, key: &str) -> bool {
    let now = n.chapter.as_ref().map(|c| c.index).unwrap_or(0);
    let memory = (d.climax_memory_chapters as u32).max(1);
    !n.climaxes
        .iter()
        .any(|(ch, k)| k == key && now.saturating_sub(*ch) <= memory)
}

/// The dilemmas the spotlight could present now, with their tiers: not
/// requests, storyline stages or raised decisions; available (cooldowns, no
/// repeat, `min_career_days`). The climax's candidates skip the relevance
/// floor and recent climaxes; the others must reach it. `relax_cooldowns`:
/// the climax fallback (DEC-097 note), which waives template cooldowns only.
fn gather<'c>(
    v: &View<'c>,
    n: &NarrativeState,
    climax: bool,
    relax_cooldowns: bool,
) -> Vec<Candidate<'c>> {
    let content = v.content();
    let d = &content.director;
    let date = v.date();
    let staged: Vec<&str> = n
        .active_runs()
        .filter_map(|r| content.storyline(&r.template))
        .flat_map(|s| s.stages.iter().filter_map(|st| st.decision.as_deref()))
        .collect();
    let mut cands: Vec<Candidate<'_>> = Vec::new();
    for def in &content.dilemmas {
        if def.category == DilemmaCategory::Request
            || def.storyline_only
            || def.raised_by.is_some()
            || staged.contains(&def.id.as_str())
            || !available(n, d, def, date, relax_cooldowns && climax)
            // The fallback never takes a fourth use (`FALLBACK_MAX_USES`).
            || (relax_cooldowns && n.uses.get(&def.id).copied().unwrap_or(0) > FALLBACK_MAX_USES)
            || (climax && !climax_allowed(n, d, &def.id))
            || (!climax && def.category == DilemmaCategory::Climax)
        {
            continue;
        }
        if let Some(c) = evaluate(v, n, def, &[], None, &[])
            && (climax || c.relevance >= d.min_relevance)
        {
            cands.push(c);
        }
    }
    cands
}

/// Without a pacing cycle (Unfiltered): the climax in its window, else a
/// significant decision when the tension curve calls for one (or the chapter
/// is short of its minimum), by the pick.
#[allow(clippy::too_many_arguments)]
fn dilemmas(
    n: &mut NarrativeState,
    v: &View<'_>,
    out: &mut Outs<'_>,
    profile: Option<&StorytellerDef>,
    b: Budget,
    x: f64,
    target: f64,
    actual: f64,
) {
    let d = &v.content().director;
    let date = v.date();
    if !can_decide(n, d, date) {
        return;
    }
    let Some(ch) = n.chapter.clone() else { return };
    // The climax: the pick among grounded situations in its window (it
    // respects cooldowns; no relevance floor).
    if climax_due(n, v, b) {
        let mut cands = gather(v, n, true, false);
        // The climax fallback, as at a climax peak (DEC-097 note).
        let fallback = cands.is_empty();
        if fallback {
            cands = least_delayed(n, d, date, gather(v, n, true, true));
        }
        if let Some(c) = pick(v, n, cands, false, 2) {
            let key = c.def.id.clone();
            let base = fallback_cooldown(n, d, &c.def.id, c.def.cooldown_days);
            open(n, v, out, c, Weight::Climax, None, None);
            if fallback {
                n.cooldowns.insert(key, date.add_days(base));
            }
        }
        return;
    }
    if !paced(ch.significant, b.sig_max, x) {
        return;
    }
    let shaped = profile.is_some_and(|p| p.tension_curve);
    let behind = x >= d.tension.catch_up_at && ch.significant < b.sig_min;
    if shaped && !behind && actual >= target + d.tension.slack {
        return;
    }
    // Opportunities arrive when the chapter is quiet (the director times them).
    let quiet = !shaped || actual < target;
    let mut cands = gather(v, n, false, false);
    cands.retain(|c| {
        c.def.category == DilemmaCategory::Significant && (!c.def.opportunity || quiet || behind)
    });
    if let Some(c) = pick(v, n, cands, false, 3) {
        open(n, v, out, c, Weight::Significant, None, None);
    }
}

/// The peak (ENGAGEMENT §4.3): one significant decision, by tier (due
/// storyline stages and closing consequences, the chapter's theme, the
/// rest). A climax peak opens the chapter's climax. After the climax, the
/// coda allows `post_climax_significant` decisions, and only consequences,
/// opportunities, `budget` templates or storyline stages. The tension curve
/// stays the envelope for dilemmas; storyline stages wait only for room.
#[allow(clippy::too_many_arguments)]
fn peak(
    n: &mut NarrativeState,
    v: &View<'_>,
    out: &mut Outs<'_>,
    profile: Option<&StorytellerDef>,
    b: Budget,
    x: f64,
    target: f64,
    actual: f64,
    due: &[usize],
) {
    let d = &v.content().director;
    let date = v.date();
    let Some(ph) = n
        .phase
        .clone()
        .filter(|p| p.kind == PhaseKind::Peak && p.decision.is_none())
    else {
        return;
    };
    let Some(ch) = n.chapter.clone() else { return };
    if n.pending().count() >= d.max_pending as usize {
        return;
    }
    let spaced = can_decide(n, d, date);
    if ph.climax && ch.kind == ChapterKind::Session && ch.climaxes < b.climax {
        let mut cands = staged_all(v, n, due);
        cands.retain(|c| climax_allowed(n, d, &c.def.id));
        if spaced {
            cands.extend(gather(v, n, true, false));
        }
        // The climax fallback (DEC-097 note, until E3 widens the pool):
        // nothing is eligible, so template cooldowns are waived; climax
        // memory, the last chapter's climax and no repeat in a chapter hold.
        let fallback = cands.is_empty() && spaced;
        if fallback {
            cands = least_delayed(n, d, date, gather(v, n, true, true));
        }
        if let Some(c) = pick(v, n, cands, ph.eased, 2) {
            let key = c.def.id.clone();
            let base = fallback_cooldown(n, d, &c.def.id, c.def.cooldown_days);
            if c.run.is_some() {
                open_staged(n, v, out, c, Weight::Climax);
            } else {
                open(n, v, out, c, Weight::Climax, None, None);
            }
            // A borrowed climax cools the template for its base cooldown,
            // not a grown one, so it does not leave the rest of the career.
            if fallback {
                n.cooldowns.insert(key, date.add_days(base));
            }
            if let Some(p) = n.phase.as_mut() {
                p.climax_fallback = fallback;
            }
        }
        return;
    }
    if ch.significant >= b.sig_max {
        return;
    }
    // The coda: the first peak after the climax opens at most
    // `post_climax_significant`, of the right kind.
    let coda = ch.kind == ChapterKind::Session && is_coda(n, &ch);
    if coda && d.post_climax_significant == 0 {
        return;
    }
    let mut cands = staged_all(v, n, due);
    let shaped = profile.is_some_and(|p| p.tension_curve);
    let behind = x >= d.tension.catch_up_at && ch.significant < b.sig_min;
    let allowed = !shaped || behind || actual < target + d.tension.slack;
    if spaced && allowed && paced(ch.significant, b.sig_max, x) {
        let quiet = !shaped || actual < target;
        let mut more = gather(v, n, false, false);
        more.retain(|c| {
            c.def.category == DilemmaCategory::Significant
                && (!c.def.opportunity || quiet || behind || ph.eased)
        });
        cands.extend(more);
    }
    if coda {
        cands.retain(|c| {
            c.run.is_some()
                || c.def.opportunity
                || is_consequence(c.def)
                || c.def.themes.iter().any(|t| t == "budget")
        });
    }
    if let Some(c) = pick(v, n, cands, ph.eased, 3) {
        if c.run.is_some() {
            open_staged(n, v, out, c, Weight::Significant);
        } else {
            open(n, v, out, c, Weight::Significant, None, None);
        }
    }
}

/// The current peak is the first one after the chapter's climax (the coda).
fn is_coda(n: &NarrativeState, ch: &Chapter) -> bool {
    let Some(at) = ch.climax.and_then(|id| n.decision(id)).map(|x| x.opened) else {
        return false;
    };
    !n.phase_log
        .iter()
        .any(|p| p.kind == PhaseKind::Peak && p.chapter == ch.index && p.since > at)
}

// --- The pacing cycle (ENGAGEMENT §4.3) ----------------------------------------------------

/// The climax fallback takes a template's second or third use, never a
/// fourth (ENGAGEMENT §4.1: at most three uses in five years).
const FALLBACK_MAX_USES: u16 = 2;

/// The cooldown a fallback climax sets: the template's base cooldown, not a
/// grown one, so a borrowed use does not take it out of the rest of the
/// career.
fn fallback_cooldown(_n: &NarrativeState, d: &DirectorParams, _key: &str, cooldown: u32) -> u32 {
    cooldown_days(d, cooldown, 1)
}

/// The fallback borrows the templates whose return the climax delays
/// least: the new cooldown's end against the one it was cooling to.
fn least_delayed<'c>(
    n: &NarrativeState,
    d: &DirectorParams,
    date: SimDate,
    mut cands: Vec<Candidate<'c>>,
) -> Vec<Candidate<'c>> {
    let delay = |c: &Candidate<'_>| {
        let new = date.add_days(fallback_cooldown(n, d, &c.def.id, c.def.cooldown_days));
        let old = n.cooldowns.get(&c.def.id).copied().unwrap_or(date);
        old.days_until(new)
    };
    let Some(least) = cands.iter().map(delay).min() else {
        return cands;
    };
    cands.retain(|c| delay(c) == least);
    cands
}

/// Draw tags for phase lengths.
const TAG_BUILD_UP: u64 = 1;
const TAG_RELAX: u64 = 2;

/// What happened since the last weekly step that ends a relax early: the
/// player started something (introduced a bill, filed for office), or a
/// crisis began in a focus town.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Stir {
    pub started: bool,
    pub crisis: bool,
}

fn stir(n: &mut NarrativeState, v: &View<'_>, out: &Outs<'_>, first: bool) -> Stir {
    let me = v.player().map(|p| p.politician.0);
    let last = n.seen.last_event;
    let mut started = false;
    for e in out.events.iter().rev() {
        if e.id.0 <= last {
            break;
        }
        if let EventKind::PolicyIntroduced { sponsor, .. } = &e.kind
            && Some(*sponsor) == me
        {
            started = true;
        }
    }
    let filed = v.player().is_some_and(|p| p.filing.is_some());
    started |= filed && !n.seen.filed;
    n.seen.filed = filed;
    let crisis = !first
        && focus_towns(v).iter().any(|m| {
            v.pol.characters.towns.get(*m as usize).is_some_and(|t| {
                t.crises
                    .iter()
                    .any(|c| c.severity > 0.0 && !n.seen.crises.iter().any(|(k, _)| *k == c.kind))
            })
        });
    Stir { started, crisis }
}

/// The phase lengths of a kind of chapter.
pub fn lengths(d: &DirectorParams, kind: ChapterKind) -> &CycleLengths {
    match kind {
        ChapterKind::Session => &d.cycle.session,
        ChapterKind::Campaign => &d.cycle.campaign,
    }
}

/// A phase length: a keyed draw in `range` (days, inclusive), scaled by the
/// profile (frequency, not amplitude), at least a day.
pub fn span(seed: u64, chapter: u32, cycle: u32, tag: u64, range: [u32; 2], scale: f64) -> u32 {
    let lo = range[0].min(range[1]);
    let hi = range[1].max(lo);
    let u = mandate_core::rng::keyed(
        seed,
        Stream::Director,
        &[super::KEY_PHASE, chapter as u64, cycle as u64, tag],
    )
    .next_f64();
    let days = lo as f64 + (u * (hi - lo + 1) as f64).floor().min((hi - lo) as f64);
    ((days * scale).round() as u32).max(1)
}

/// End the phase in force (into the log) and begin another.
#[allow(clippy::too_many_arguments)]
fn begin(
    n: &mut NarrativeState,
    kind: PhaseKind,
    date: SimDate,
    until: Option<SimDate>,
    cycle: u32,
    climax: bool,
    eased: bool,
) {
    let chapter = n.chapter.as_ref().map(|c| c.index).unwrap_or(0);
    if let Some(mut old) = n.phase.take() {
        old.until = Some(date);
        n.phase_log.push(old);
        if n.phase_log.len() > super::PHASE_LOG_CAP {
            n.phase_log.remove(0);
        }
    }
    if kind == PhaseKind::Peak
        && let Some(ch) = n.chapter.as_mut()
    {
        ch.peaks = ch.peaks.saturating_add(1);
    }
    n.phase = Some(DirectorPhase {
        kind,
        since: date,
        until,
        cycle,
        chapter,
        climax,
        eased,
        decision: None,
        beats: 0,
        climax_fallback: false,
    });
}

/// What a step of the pacing cycle reads, beyond the narrative state: the
/// date, the draw seed, phase lengths and the climax window. (A plain value,
/// so the cycle can be tested without a world.)
#[derive(Clone, Debug)]
pub(crate) struct CycleCtx {
    pub date: SimDate,
    pub seed: u64,
    pub lens: CycleLengths,
    pub build_up_scale: f64,
    pub relax_scale: f64,
    pub setback_relax_mult: f64,
    pub climax_force_days: u32,
    /// The session's climax window, [first day, last day]; `None` in a
    /// campaign.
    pub window: Option<[SimDate; 2]>,
    /// The chapter still wants its climax.
    pub climax_open: bool,
    pub stir: Stir,
    pub kind_changed: bool,
}

/// Step the pacing cycle to today (several transitions may fall due at
/// once). The cycle runs across chapters; a change of kind of chapter clips
/// a long phase to the new kind's lengths.
fn step_cycle(
    n: &mut NarrativeState,
    v: &View<'_>,
    p: &StorytellerDef,
    b: Budget,
    s: Stir,
    kind_changed: bool,
) {
    let d = &v.content().director;
    let Some(ch) = n.chapter.as_ref() else { return };
    let window = (ch.kind == ChapterKind::Session).then(|| {
        let total = ch.start.days_until(climax_horizon(v, ch)) as f64;
        let w = &d.tension.climax_window;
        [
            ch.start.add_days((total * w[0]).ceil() as u32),
            ch.start.add_days((total * w[1]).floor() as u32),
        ]
    });
    let ctx = CycleCtx {
        date: v.date(),
        seed: v.r.meta.seed,
        lens: lengths(d, ch.kind).clone(),
        build_up_scale: p.build_up_scale,
        relax_scale: p.relax_scale,
        setback_relax_mult: d.cycle.setback_relax_mult,
        climax_force_days: d.cycle.climax_force_days,
        window,
        climax_open: ch.kind == ChapterKind::Session
            && ch.climaxes < b.climax
            && ch.climax.is_none(),
        stir: s,
        kind_changed,
    };
    cycle_core(n, &ctx);
}

/// The pacing cycle's transitions (ENGAGEMENT §4.3):
/// - build-up, at its drawn length → peak (the climax peak when its
///   build-up ends inside the climax window, or easing off after a setback);
/// - peak, once its decision is answered or at `peak_max` → peak fade;
/// - peak fade, once nothing significant is pending → relax (the longer one
///   after the climax; × `setback_relax_mult` after a setback);
/// - relax, at its drawn length or early when the player starts something
///   or a crisis begins → the next cycle's build-up;
/// - a session with no climax within `climax_force_days` of its window's
///   end: the phase ends and the climax peak opens.
pub(crate) fn cycle_core(n: &mut NarrativeState, c: &CycleCtx) {
    let date = c.date;
    let Some(chapter) = n.chapter.as_ref().map(|ch| ch.index) else {
        return;
    };
    let lens = &c.lens;
    if n.phase.is_none() {
        let cycle = n.phase_log.last().map(|p| p.cycle + 1).unwrap_or(1);
        let len = span(
            c.seed,
            chapter,
            cycle,
            TAG_BUILD_UP,
            lens.build_up,
            c.build_up_scale,
        );
        begin(
            n,
            PhaseKind::BuildUp,
            date,
            Some(date.add_days(len)),
            cycle,
            false,
            false,
        );
    }
    if c.kind_changed
        && let Some(ph) = n.phase.as_mut()
    {
        let longest = |r: [u32; 2], k: f64| ((r[1] as f64 * k).round() as u32).max(1);
        let cap = match ph.kind {
            PhaseKind::BuildUp => Some(longest(lens.build_up, c.build_up_scale)),
            PhaseKind::Relax => Some(longest(
                lens.relax_after_climax.unwrap_or(lens.relax),
                c.relax_scale * c.setback_relax_mult,
            )),
            _ => None,
        };
        if let (Some(cap), Some(u)) = (cap, ph.until) {
            ph.until = Some(u.min(date.add_days(cap)));
        }
    }
    let in_window = |on: SimDate| c.window.is_some_and(|[a, b]| on >= a && on <= b);
    let force = c.climax_open
        && c.window
            .is_some_and(|[_, end]| date <= end && date.add_days(c.climax_force_days) >= end);
    for _ in 0..6 {
        let Some(ph) = n.phase.clone() else { return };
        if force && !(ph.kind == PhaseKind::Peak && ph.climax) {
            let cycle = ph.cycle + (ph.kind != PhaseKind::BuildUp) as u32;
            let eased = n.setback.is_some() || (ph.kind == PhaseKind::BuildUp && ph.eased);
            let until = Some(date.add_days(lens.peak_max));
            begin(n, PhaseKind::Peak, date, until, cycle, true, eased);
            continue;
        }
        let due = |u: Option<SimDate>| u.is_some_and(|u| date >= u);
        match ph.kind {
            PhaseKind::BuildUp => {
                if !due(ph.until) {
                    break;
                }
                let climax = c.climax_open && in_window(date);
                let eased = ph.eased || n.setback.is_some();
                let until = Some(date.add_days(lens.peak_max));
                begin(n, PhaseKind::Peak, date, until, ph.cycle, climax, eased);
            }
            PhaseKind::Peak => {
                let answered = ph.decision.is_some_and(|id| {
                    n.decision(id)
                        .is_none_or(|x| x.status != DecisionStatus::Pending)
                });
                if !answered && !due(ph.until) {
                    break;
                }
                let climax = ph.climax && ph.decision.is_some();
                begin(
                    n,
                    PhaseKind::PeakFade,
                    date,
                    None,
                    ph.cycle,
                    climax,
                    ph.eased,
                );
            }
            PhaseKind::PeakFade => {
                let waiting = n
                    .pending()
                    .any(|x| x.weight != Weight::Minor && x.about.is_none());
                if waiting {
                    break;
                }
                let range = if ph.climax {
                    lens.relax_after_climax.unwrap_or(lens.relax)
                } else {
                    lens.relax
                };
                let mut len = span(c.seed, chapter, ph.cycle, TAG_RELAX, range, c.relax_scale);
                // Easing off after a setback (RimWorld): a longer relax.
                let eased = n.setback.take().is_some();
                if eased {
                    len = ((len as f64 * c.setback_relax_mult).round() as u32).max(1);
                }
                let until = Some(date.add_days(len));
                begin(n, PhaseKind::Relax, date, until, ph.cycle, ph.climax, eased);
            }
            PhaseKind::Relax => {
                // It ends early when the player starts something (Booth's
                // relax ends when the survivors move on), or a crisis begins.
                let early = ph.since < date && (c.stir.started || c.stir.crisis);
                if !early && !due(ph.until) {
                    break;
                }
                let cycle = ph.cycle + 1;
                let len = span(
                    c.seed,
                    chapter,
                    cycle,
                    TAG_BUILD_UP,
                    lens.build_up,
                    c.build_up_scale,
                );
                let until = Some(date.add_days(len));
                begin(n, PhaseKind::BuildUp, date, until, cycle, false, ph.eased);
            }
        }
    }
}

/// Setbacks (ENGAGEMENT §4.3, RimWorld's adaptation) since the last weekly
/// step: an election lost, Standing or viability warned about. (A scandal
/// about the player is noted when its decision is raised.)
fn setbacks(n: &mut NarrativeState, v: &View<'_>) {
    let Some(pl) = v.player() else { return };
    let date = v.date();
    let since = SimDate(date.0.saturating_sub(7));
    let warned = n
        .warnings
        .iter()
        .any(|w| w.date > since && (w.track == "standing" || w.track == "viability"));
    let lost = v.pol.elections.results.iter().any(|res| {
        res.date > since
            && res.date <= date
            && res.winner != pl.politician
            && res.candidates.iter().any(|c| c.politician == pl.politician)
    });
    if warned || lost {
        n.setback = Some(date);
    }
}

/// Answers the player gave (or let default) since the last weekly step, for
/// `answered` conditions (ENGAGEMENT §4.4).
fn record_answers(n: &mut NarrativeState) {
    let newest = n.answers.last().map(|a| a.date);
    let mut fresh: Vec<AnswerRecord> = n
        .decisions
        .iter()
        .filter(|x| {
            matches!(
                x.status,
                DecisionStatus::Answered | DecisionStatus::Defaulted
            )
        })
        .filter_map(|x| {
            let r = x.resolution.as_ref()?;
            // Older than what is kept already: recorded (or dropped) before.
            if newest.is_some_and(|nw| r.date.add_days(14) < nw)
                || n.answers.iter().any(|a| a.decision == x.id)
            {
                return None;
            }
            Some(AnswerRecord {
                decision: x.id,
                template: x.template.clone(),
                option: r.option.clone(),
                date: r.date,
                by_default: r.by_default,
                cast: x.cast.clone(),
            })
        })
        .collect();
    fresh.sort_by_key(|a| (a.date, a.decision));
    n.answers.extend(fresh);
    if n.answers.len() > super::ANSWERS_CAP {
        let drop = n.answers.len() - super::ANSWERS_CAP;
        n.answers.drain(..drop);
    }
}

/// Press stories about the player, by date (a minor beat; DEC-034).
fn press_dates(v: &View<'_>) -> Vec<SimDate> {
    let me = v.player().map(|p| p.politician);
    v.r.media()
        .stories
        .iter()
        .filter(|s| s.subject.is_some() && s.subject == me)
        .map(|s| s.date)
        .collect()
}

/// Spotlit development and storyline facts (minor beats).
fn is_beat(k: &FactKind) -> bool {
    matches!(
        k,
        FactKind::StorylineStage { .. }
            | FactKind::TownEvent { .. }
            | FactKind::CrisisBegan { .. }
            | FactKind::CrisisWorse { .. }
            | FactKind::GoalTaken { .. }
            | FactKind::RivalNamed { .. }
    )
}

/// The last day something happened for the player: a decision opened, a
/// storyline stage, news of the town spotlighted, press about them.
fn last_beat(n: &NarrativeState, v: &View<'_>) -> Option<SimDate> {
    let decided = n.decisions.iter().map(|x| x.opened).max();
    let fact = n
        .facts
        .iter()
        .rev()
        .find(|f| f.spotlit && is_beat(&f.kind))
        .map(|f| f.date);
    let press = press_dates(v).into_iter().max();
    [decided, fact, press, n.play_began]
        .into_iter()
        .flatten()
        .max()
}

/// A development fact still grounded in live state (it may be spotlighted
/// again by the quiet rescue).
fn still_grounded(v: &View<'_>, k: &FactKind) -> bool {
    let town = v.town_character();
    match k {
        FactKind::CrisisBegan { kind } | FactKind::CrisisWorse { kind } => {
            town.is_some_and(|t| t.crises.iter().any(|c| &c.kind == kind))
        }
        FactKind::GoalTaken { person, goal } => v
            .pol
            .characters
            .goals
            .iter()
            .any(|g| g.holder == *person && &g.goal == goal),
        FactKind::RivalNamed { person } => town.is_some_and(|t| t.rival == Some(*person)),
        FactKind::TownEvent { .. } => true,
        _ => false,
    }
}

/// The quiet rescue (ENGAGEMENT §4.3): nothing for the player for
/// `quiet_days` (governing) or `quiet_days_campaign`: one minor beat at once,
/// in any phase, never a significant decision. Until asks (E6), the beat is
/// a request, else the most newsworthy recent news of the town that was not
/// spotlighted. The budget still caps it.
fn rescue(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>, b: Budget) {
    let d = &v.content().director;
    let date = v.date();
    let Some(ch) = n.chapter.as_ref() else { return };
    let quiet = if ch.kind == ChapterKind::Campaign {
        d.quiet_days_campaign
    } else {
        d.quiet_days
    };
    if last_beat(n, v).is_some_and(|l| l.add_days(quiet) > date) {
        return;
    }
    if requests(n, v, out, b, true) {
        return;
    }
    if !minor_room(n, d, b, date, true) {
        return;
    }
    let from = SimDate(date.0.saturating_sub(quiet));
    let Some(f) = n
        .facts
        .iter()
        .filter(|f| !f.spotlit && f.date >= from && is_beat(&f.kind))
        .filter(|f| still_grounded(v, &f.kind))
        .max_by(|a, b| {
            a.newsworthiness
                .total_cmp(&b.newsworthiness)
                .then(a.id.cmp(&b.id))
        })
        .cloned()
    else {
        return;
    };
    spotlight_development(n, v, out, f.kind, f.actors, f.newsworthiness);
}

/// Spotlight a development: a minor beat, news of the town to the player.
fn spotlight_development(
    n: &mut NarrativeState,
    v: &View<'_>,
    out: &mut Outs<'_>,
    kind: FactKind,
    actors: Vec<Actor>,
    news: f32,
) {
    let date = v.date();
    if let Some(ch) = n.chapter.as_mut() {
        ch.minor += 1;
        ch.last_minor = Some(date);
    }
    if let Some(ph) = n.phase.as_mut() {
        ph.beats = ph.beats.saturating_add(1);
    }
    let id = n.log(date, kind, actors, true, news);
    out.events.record(
        date,
        EventKind::StoryDevelopment {
            fact: id,
            municipality: v.town().unwrap_or(0),
        },
        vec![],
    );
}

fn band(sev: f32) -> u8 {
    (sev >= 0.34) as u8 + (sev >= 0.67) as u8
}

/// Developments: news of the town the player should know (minor beats).
fn developments(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>, b: Budget, first: bool) {
    let content = v.content();
    let d = &content.director;
    let date = v.date();
    let Some(town) = v.town_character() else {
        return;
    };
    let m = v.town().unwrap_or(0);
    let crises: Vec<(String, u8)> = town
        .crises
        .iter()
        .map(|c| (c.kind.clone(), band(c.severity)))
        .collect();
    let goals: Vec<(CharacterId, String)> = v
        .pol
        .characters
        .goals
        .iter()
        .map(|g| (g.holder, g.goal.clone()))
        .collect();
    let rival = town.rival;
    let mut found: Vec<(FactKind, Vec<Actor>, f32)> = Vec::new();
    if !first {
        for (k, bnd) in &crises {
            match n.seen.crises.iter().find(|(x, _)| x == k) {
                None => found.push((FactKind::CrisisBegan { kind: k.clone() }, Vec::new(), 0.7)),
                Some((_, old)) if bnd > old => {
                    found.push((FactKind::CrisisWorse { kind: k.clone() }, Vec::new(), 0.6))
                }
                _ => {}
            }
        }
        for (h, g) in &goals {
            if !n.seen.goals.iter().any(|(a, b)| a == h && b == g) {
                found.push((
                    FactKind::GoalTaken {
                        person: *h,
                        goal: g.clone(),
                    },
                    vec![Actor::Person { id: *h }],
                    0.4,
                ));
            }
        }
        if let Some(r) = rival
            && n.seen.rival != Some(r)
        {
            found.push((
                FactKind::RivalNamed { person: r },
                vec![Actor::Person { id: r }],
                0.5,
            ));
        }
    }
    // News of the town from the event log: closures, disasters, and
    // colleagues' bills (the player's own are not news to them).
    let me = v.player().map(|p| p.politician.0);
    let last = n.seen.last_event;
    let mut newest = last;
    for e in out.events.iter().rev() {
        if e.id.0 <= last {
            break;
        }
        newest = newest.max(e.id.0);
        if first {
            continue;
        }
        let news = match &e.kind {
            EventKind::PlantClosure { municipality, .. } if *municipality == m => 0.8,
            EventKind::Disaster { municipality, .. } if *municipality == m => 0.8,
            EventKind::PolicyIntroduced {
                municipality,
                sponsor,
                ..
            } if *municipality == m && Some(*sponsor) != me => 0.3,
            EventKind::PolicyEnacted { municipality, .. }
            | EventKind::PolicyFailed { municipality, .. }
                if *municipality == m =>
            {
                0.4
            }
            // The campaign: polls on the player's race, other races in town.
            EventKind::PollsPublished { .. } if v.campaigning() => 0.5,
            EventKind::CampaignOpened { .. } => 0.5,
            EventKind::RaceDecided {
                municipality,
                player_ran: false,
                ..
            } if *municipality == m => 0.3,
            _ => continue,
        };
        found.push((FactKind::TownEvent { event: e.id }, Vec::new(), news));
    }
    n.seen.last_event = newest;
    // The most newsworthy first; ties: oldest first.
    found.sort_by(|a, b| b.2.total_cmp(&a.2));
    n.seen.crises = crises;
    n.seen.goals = goals;
    n.seen.rival = rival;
    for (kind, actors, news) in found {
        if can_minor(n, d, b, date) {
            spotlight_development(n, v, out, kind, actors, news);
        } else {
            n.log(date, kind, actors, false, news);
        }
    }
}

/// Narrative domain, Director phase, weekly.
pub fn weekly(n: &mut NarrativeState, r: &Reads<'_>, out: &mut Outs<'_>) {
    if !active(r) {
        return;
    }
    let pol = r.politics();
    let mut v = View::new(r, pol);
    v.recent = recent_people(n);
    v.answers = Some(n.answers.clone());
    let content = r.content;
    let d = &content.director;
    let first = n.play_began.is_none();
    if first {
        n.play_began = Some(r.date);
    }
    if n.profile.is_empty() {
        n.profile = d.default_profile.clone();
    }
    let profile = n.profile(content).cloned();
    let b = budget(d, profile.as_ref());
    let kind_changed = chapters(n, &v, out);
    let x = n
        .chapter
        .as_ref()
        .map(|c| c.progress(r.date))
        .unwrap_or(0.0);
    let target = curve(d, x);
    let world = world_tension(&v, n.chapter.as_ref());
    // The exogenous levers read the simulation alone (never the spotlight).
    chance(n, &v, profile.as_ref());
    discovery(n, &v, out, profile.as_ref(), world < target);
    if !n.spotlight {
        n.tension = world as f32;
        return;
    }
    // In prison, or the run is over (DEC-006): nothing is put to the player;
    // what was waiting lapses (they cannot act on it).
    if r.player.is_some_and(|p| !p.status.active()) {
        n.tension = world as f32;
        let date = r.date;
        let mut runs = Vec::new();
        for x in n.decisions.iter_mut() {
            if x.status == DecisionStatus::Pending {
                x.status = DecisionStatus::Lapsed;
                x.resolution = Some(Resolution {
                    option: String::new(),
                    date,
                    by_default: false,
                    applied: Vec::new(),
                });
                runs.extend(x.storyline);
            }
        }
        for run in n.storylines.iter_mut().filter(|x| runs.contains(&x.id)) {
            run.decision = None;
            run.entered = false;
        }
        return;
    }
    let actual = world + d.tension.pending_weight * pending_load(n);
    n.tension = actual as f32;
    if let Some(ch) = n.chapter.as_mut() {
        ch.peak_tension = ch.peak_tension.max(actual as f32);
    }
    record_answers(n);
    v.answers = Some(n.answers.clone());
    let cycle = profile.as_ref().filter(|p| p.cycle);
    if cycle.is_none()
        && let Some(mut old) = n.phase.take()
    {
        // Unfiltered: no phases (the profile changed).
        old.until = Some(r.date);
        n.phase_log.push(old);
        n.setback = None;
    }
    let s = stir(n, &v, out, first);
    lapse(n, &v, out);
    raise(n, &v, out);
    if cycle.is_some() {
        setbacks(n, &v);
    }
    // Press about the player is a minor beat of the phase it falls in.
    let week_ago = SimDate(r.date.0.saturating_sub(7));
    let press = press_dates(&v)
        .into_iter()
        .filter(|x| *x > week_ago && *x <= r.date)
        .count();
    if let Some(ph) = n.phase.as_mut() {
        ph.beats = ph.beats.saturating_add(press.min(u16::MAX as usize) as u16);
    }
    start_runs(n, &v);
    if let Some(p) = cycle {
        step_cycle(n, &v, p, b, s, kind_changed);
    }
    let due = advance_runs(n, &v, out, b, false);
    requests(n, &v, out, b, false);
    if cycle.is_some() {
        peak(n, &v, out, profile.as_ref(), b, x, target, actual, &due);
    } else {
        open_due_stages(n, &v, out, b, &due);
        dilemmas(n, &v, out, profile.as_ref(), b, x, target, actual);
    }
    developments(n, &v, out, b, first);
    if !first {
        rescue(n, &v, out, b);
    }
    n.trim(d.history as usize);
}

/// People spotlighted in this or the last chapter: cast last, for variety.
fn recent_people(n: &NarrativeState) -> Vec<CharacterId> {
    let recent_chapter = n.chapters.last().map(|c| c.index).unwrap_or(0);
    n.decisions
        .iter()
        .filter(|d| d.chapter >= recent_chapter)
        .flat_map(|d| d.cast.iter())
        .filter_map(|e| match e.actor {
            Actor::Person { id } => Some(id),
            _ => None,
        })
        .collect()
}

// --- The lab (ENGAGEMENT §7.4) -------------------------------------------------------------

/// One candidate the director weighs this week: the lab's "eligible but
/// never picked" table.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CandidateInfo {
    pub template: String,
    pub tier: Tier,
    pub relevance: f64,
    /// `1 / (1 + uses)`.
    pub novelty: f64,
    /// Within `pick_band` of its tier's best (the pick may draw it).
    pub in_band: bool,
    pub opportunity: bool,
    /// A storyline stage's decision.
    pub storyline: bool,
}

/// The candidates the spotlight would weigh now, from a whole state: due
/// storyline stages, and the dilemmas whose triggers hold and that are
/// available (cooldowns, no repeat, `min_career_days`); the climax's when a
/// climax is due. Best tier first, then by relevance. Pure: it writes
/// nothing and draws nothing (ENGAGEMENT §7.4).
pub fn week_candidates(state: &crate::State, content: &ContentBundle) -> Vec<CandidateInfo> {
    let r = Reads::of_state(state, content);
    let n = &state.narrative;
    if !active(&r) || n.chapter.is_none() {
        return Vec::new();
    }
    let mut v = View::new(&r, &state.politics);
    v.recent = recent_people(n);
    let d = &content.director;
    let b = budget(d, n.profile(content));
    let climax = match &n.phase {
        Some(ph) => ph.climax && ph.kind == PhaseKind::Peak,
        None => climax_due(n, &v, b),
    };
    let due: Vec<usize> = (0..n.storylines.len())
        .filter(|&i| run_step(&v, n, i) == RunStep::Due)
        .collect();
    let mut cands = staged_all(&v, n, &due);
    cands.extend(gather(&v, n, climax, false));
    let best = |t: Tier| {
        cands
            .iter()
            .filter(|c| c.tier == t)
            .map(|c| c.relevance)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let mut out: Vec<CandidateInfo> = cands
        .iter()
        .map(|c| CandidateInfo {
            template: c.def.id.clone(),
            tier: c.tier,
            relevance: c.relevance,
            novelty: novelty(n, &c.def.id),
            in_band: c.relevance >= best(c.tier) - d.pick_band,
            opportunity: c.def.opportunity,
            storyline: c.run.is_some(),
        })
        .collect();
    out.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then(b.relevance.total_cmp(&a.relevance))
            .then(a.template.cmp(&b.template))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(tier: Tier, relevance: f64, opportunity: bool, uses: u16) -> Pickable {
        Pickable {
            tier,
            relevance,
            opportunity,
            novelty: 1.0 / (1.0 + uses as f64),
        }
    }

    /// What the pick chooses over a fine grid of draws (index → count).
    fn spread(items: &[Pickable], opp: bool) -> Vec<u32> {
        let mut out = vec![0u32; items.len()];
        for k in 0..1000 {
            let u = (k as f64 + 0.5) / 1000.0;
            out[pick_index(items, 0.8, opp, u).unwrap()] += 1;
        }
        out
    }

    #[test]
    fn tier_a_is_drawn_before_b_and_c() {
        // A storyline stage of low relevance beats a themed template and a
        // generic one of high relevance (story-carrying content first).
        let items = [
            item(Tier::C, 3.0, false, 0),
            item(Tier::B, 2.5, false, 0),
            item(Tier::A, 0.7, false, 0),
        ];
        assert_eq!(spread(&items, false), vec![0, 0, 1000]);
        // Without a tier A candidate, the chapter's theme comes first.
        assert_eq!(spread(&items[..2], false), vec![0, 1000]);
        assert_eq!(pick_index(&[], 0.8, false, 0.3), None);
    }

    #[test]
    fn the_pick_draws_within_the_band_by_relevance_and_novelty() {
        let items = [
            item(Tier::C, 2.0, false, 0),
            // In the band but seen three times: a quarter of the weight.
            item(Tier::C, 1.9, false, 3),
            // Outside the band (2.0 − 0.8): never drawn.
            item(Tier::C, 1.1, false, 0),
        ];
        let s = spread(&items, false);
        assert_eq!(s[2], 0);
        let expect = 2.0 / (2.0 + 1.9 / 4.0);
        let share = s[0] as f64 / 1000.0;
        assert!((share - expect).abs() < 0.01, "{share} vs {expect}");
    }

    #[test]
    fn easing_off_draws_opportunities_first() {
        let items = [item(Tier::C, 2.5, false, 0), item(Tier::C, 0.8, true, 0)];
        assert_eq!(spread(&items, true), vec![0, 1000]);
        // Not easing off: the opportunity is outside the band.
        assert_eq!(spread(&items, false), vec![1000, 0]);
    }

    fn params() -> DirectorParams {
        let src = mandate_content::bundle::read_dir_sources(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        )
        .unwrap();
        mandate_content::load(&src).unwrap().0.director
    }

    #[test]
    fn cooldowns_have_a_floor_and_grow_with_each_use() {
        let d = params();
        assert_eq!(d.cooldown_floor_days, 365);
        assert_eq!(cooldown_days(&d, 200, 1), 365);
        assert_eq!(cooldown_days(&d, 200, 2), 730);
        assert_eq!(cooldown_days(&d, 540, 1), 540);
        assert_eq!(cooldown_days(&d, 540, 3), 2160);
    }

    fn lens() -> CycleLengths {
        CycleLengths {
            build_up: [35, 60],
            peak_max: 14,
            relax: [21, 35],
            relax_after_climax: Some([30, 45]),
        }
    }

    /// A narrative in a session chapter from day 1000 to 1365, its climax
    /// window [1237, 1310], and a context on `date`.
    fn session(date: u32) -> (NarrativeState, CycleCtx) {
        let n = NarrativeState {
            chapter: Some(Chapter::new(
                1,
                ChapterKind::Session,
                SimDate(1000),
                SimDate(1365),
            )),
            ..Default::default()
        };
        let ctx = CycleCtx {
            date: SimDate(date),
            seed: 7,
            lens: lens(),
            build_up_scale: 1.0,
            relax_scale: 1.0,
            setback_relax_mult: 1.5,
            climax_force_days: 14,
            window: Some([SimDate(1237), SimDate(1310)]),
            climax_open: true,
            stir: Stir::default(),
            kind_changed: false,
        };
        (n, ctx)
    }

    fn phase(kind: PhaseKind, since: u32, until: Option<u32>) -> DirectorPhase {
        DirectorPhase {
            kind,
            since: SimDate(since),
            until: until.map(SimDate),
            cycle: 3,
            chapter: 1,
            climax: false,
            eased: false,
            decision: None,
            beats: 0,
            climax_fallback: false,
        }
    }

    #[test]
    fn the_climax_peak_is_forced_two_weeks_before_the_window_closes() {
        // A long build-up that would end after the window: 15 days before
        // its end nothing happens; 14 days before, the climax peak opens.
        let (mut n, mut c) = session(1295);
        n.phase = Some(phase(PhaseKind::BuildUp, 1260, Some(1330)));
        cycle_core(&mut n, &c);
        assert_eq!(n.phase.as_ref().unwrap().kind, PhaseKind::BuildUp);
        c.date = SimDate(1296);
        cycle_core(&mut n, &c);
        let p = n.phase.as_ref().unwrap();
        assert_eq!((p.kind, p.climax), (PhaseKind::Peak, true));
        assert_eq!(p.until, Some(SimDate(1310)));
        assert_eq!(n.chapter.as_ref().unwrap().peaks, 1);
        // A relax is cut short the same way, into a new cycle.
        let (mut n, c) = session(1300);
        n.phase = Some(phase(PhaseKind::Relax, 1290, Some(1320)));
        cycle_core(&mut n, &c);
        let p = n.phase.as_ref().unwrap();
        assert_eq!((p.kind, p.climax, p.cycle), (PhaseKind::Peak, true, 4));
    }

    #[test]
    fn a_build_up_that_ends_in_the_window_makes_the_climax_peak() {
        let (mut n, c) = session(1240);
        n.phase = Some(phase(PhaseKind::BuildUp, 1190, Some(1240)));
        cycle_core(&mut n, &c);
        let p = n.phase.as_ref().unwrap();
        assert_eq!((p.kind, p.climax), (PhaseKind::Peak, true));
        // Before the window: an ordinary peak.
        let (mut n, c) = session(1200);
        n.phase = Some(phase(PhaseKind::BuildUp, 1150, Some(1200)));
        cycle_core(&mut n, &c);
        let p = n.phase.as_ref().unwrap();
        assert_eq!((p.kind, p.climax), (PhaseKind::Peak, false));
    }

    #[test]
    fn a_setback_lengthens_the_next_relax_by_half() {
        let relax_len = |setback: bool| {
            let (mut n, mut c) = session(1100);
            c.climax_open = false;
            n.phase = Some(phase(PhaseKind::PeakFade, 1095, None));
            if setback {
                n.setback = Some(SimDate(1090));
            }
            cycle_core(&mut n, &c);
            let p = n.phase.clone().unwrap();
            assert_eq!(p.kind, PhaseKind::Relax);
            assert_eq!(p.eased, setback);
            assert!(n.setback.is_none(), "eased once");
            p.since.days_until(p.until.unwrap())
        };
        let plain = relax_len(false);
        assert!((21..=35).contains(&plain), "{plain}");
        assert_eq!(relax_len(true), (plain as f64 * 1.5).round() as u32);
    }

    #[test]
    fn the_peak_after_a_setback_draws_opportunities_first() {
        let (mut n, mut c) = session(1100);
        c.climax_open = false;
        n.phase = Some(phase(PhaseKind::BuildUp, 1050, Some(1100)));
        n.setback = Some(SimDate(1080));
        cycle_core(&mut n, &c);
        let p = n.phase.as_ref().unwrap();
        assert_eq!((p.kind, p.eased), (PhaseKind::Peak, true));
    }

    #[test]
    fn relax_ends_early_when_the_player_starts_something() {
        let (mut n, mut c) = session(1100);
        c.climax_open = false;
        n.phase = Some(phase(PhaseKind::Relax, 1093, Some(1120)));
        cycle_core(&mut n, &c);
        assert_eq!(n.phase.as_ref().unwrap().kind, PhaseKind::Relax);
        c.stir.started = true;
        cycle_core(&mut n, &c);
        let p = n.phase.as_ref().unwrap();
        assert_eq!((p.kind, p.cycle), (PhaseKind::BuildUp, 4));
        let log = n.phase_log.last().unwrap();
        assert_eq!(
            (log.kind, log.until),
            (PhaseKind::Relax, Some(SimDate(1100)))
        );
        // A crisis beginning ends it too; a relax that begins this very step
        // does not end on what came before it.
        let (mut n, mut c) = session(1100);
        c.climax_open = false;
        c.stir.crisis = true;
        n.phase = Some(phase(PhaseKind::PeakFade, 1090, None));
        cycle_core(&mut n, &c);
        assert_eq!(n.phase.as_ref().unwrap().kind, PhaseKind::Relax);
    }

    #[test]
    fn a_peak_lasts_until_its_decision_is_answered_or_two_weeks() {
        let (mut n, mut c) = session(1100);
        c.climax_open = false;
        n.phase = Some(phase(PhaseKind::Peak, 1093, Some(1107)));
        cycle_core(&mut n, &c);
        assert_eq!(n.phase.as_ref().unwrap().kind, PhaseKind::Peak);
        c.date = SimDate(1107);
        cycle_core(&mut n, &c);
        // Nothing pending: the fade passes straight into the relax.
        assert_eq!(n.phase.as_ref().unwrap().kind, PhaseKind::Relax);
        let kinds: Vec<_> = n.phase_log.iter().map(|p| p.kind).collect();
        assert_eq!(kinds, vec![PhaseKind::Peak, PhaseKind::PeakFade]);
    }

    #[test]
    fn phase_lengths_are_keyed_draws_scaled_by_the_profile() {
        let a = span(7, 1, 3, TAG_RELAX, [21, 35], 1.0);
        assert_eq!(a, span(7, 1, 3, TAG_RELAX, [21, 35], 1.0));
        assert!((21..=35).contains(&a));
        assert_eq!(
            span(7, 1, 3, TAG_RELAX, [21, 35], 1.3),
            (a as f64 * 1.3).round() as u32
        );
        let draws: std::collections::BTreeSet<u32> = (0..40)
            .map(|c| span(7, 1, c, TAG_BUILD_UP, [35, 60], 1.0))
            .collect();
        assert!(draws.len() > 10 && draws.iter().all(|x| (35..=60).contains(x)));
    }
}
