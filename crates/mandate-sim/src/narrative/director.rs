//! The director (DEC-033 B): chapters and their tension curve, budgets,
//! cooldowns, relevance scoring, the spotlight (decisions, requests,
//! storyline beats, developments), fairness warnings (DEC-006), and the
//! exogenous levers. Writes only the narrative domain.

use super::cast::{self, View};
use super::{
    Actor, CastEntry, Chapter, ChapterKind, Decision, DecisionId, DecisionStatus, FactKind,
    NarrativeState, Reason, Resolution, RunEnd, StorylineRun, StorylineRunId, Warning, Weight,
};
use crate::characters::CharacterId;
use crate::events::EventKind;
use crate::places::Location;
use crate::scheduler::{Outs, Reads};
use crate::state::GamePhase;
use mandate_content::{
    Cast, Condition, ContentBundle, DilemmaCategory, DilemmaDef, DirectorParams, ScenePlace,
    StorytellerDef,
};
use mandate_core::{SimDate, Stream};

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
        let repeated = ch.templates.iter().any(|t| t == key)
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

/// A situation ready to be spotlighted.
struct Candidate<'c> {
    def: &'c DilemmaDef,
    cast: Vec<CastEntry>,
    reasons: Vec<Reason>,
    relevance: f64,
    goal: Option<(CharacterId, String)>,
}

/// Cast a dilemma and check its triggers (plus any extra conditions).
fn evaluate<'c>(
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
    let cast = cast::cast_search(v, &def.roles, preset, goal_holder, 6, &ok)?;
    let mut reasons = cast::reasons(v, &def.triggers, &cast);
    if def.opportunity {
        reasons.push(Reason::Opportunity);
    }
    let relevance = relevance(v, n, &def.id, &cast, &reasons);
    Some(Candidate {
        def,
        cast,
        reasons,
        relevance,
        goal: None,
    })
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
    let actors: Vec<Actor> = c.cast.iter().map(|e| e.actor).collect();
    let chapter_index = n.chapter.as_ref().map(|ch| ch.index).unwrap_or(0);
    if let Some(ch) = n.chapter.as_mut() {
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
    n.cooldowns
        .insert(c.def.id.clone(), date.add_days(c.def.cooldown_days));
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

/// A session chapter is in its climax window and has no climax yet.
fn climax_due(n: &NarrativeState, d: &DirectorParams, b: Budget, date: SimDate) -> bool {
    let Some(ch) = n.chapter.as_ref() else {
        return false;
    };
    let x = ch.progress(date);
    let w = &d.tension.climax_window;
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

/// The next budget year that leaves a session at least `min_days` long.
fn session_end(start: SimDate, min_days: u32) -> SimDate {
    let mut y = start.calendar().year + 1;
    loop {
        let d = SimDate::from_ymd(y, 1, 1);
        if start.days_until(d) >= min_days {
            return d;
        }
        y += 1;
    }
}

fn chapters(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>) {
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
        return;
    }
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
        ChapterKind::Session => session_end(date, d.session_min_days),
        ChapterKind::Campaign => v
            .next_election()
            .filter(|e| *e > date)
            .unwrap_or(date.add_days(60)),
    };
    n.chapter = Some(Chapter::new(index, want, date, end));
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
    let shape = profile.is_some_and(|p| p.tension_curve);
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
        if let Some(c) = cast::cast_roles(v, &def.roles, &[], None)
            && def.start.iter().all(|x| cast::holds(v, x, &c))
        {
            starts.push((def.id.clone(), c));
        }
    }
    for (k, c) in starts {
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

fn advance_runs(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>, b: Budget) {
    let content = v.content();
    let d = &content.director;
    let date = v.date();
    for i in 0..n.storylines.len() {
        if n.storylines[i].ended.is_some() {
            continue;
        }
        let Some(def) = content.storyline(&n.storylines[i].template) else {
            end_run(n, content, i, date, RunEnd::Removed);
            continue;
        };
        let r = &n.storylines[i];
        if r.decision.is_some() || r.entered {
            continue;
        }
        let Some(stage) = def.stages.get(r.stage as usize) else {
            end_run(n, content, i, date, RunEnd::Ended);
            continue;
        };
        let ready_from = r.since.add_days(stage.wait_days);
        if date < ready_from {
            continue;
        }
        if stage
            .timeout_days
            .is_some_and(|t| date > ready_from.add_days(t))
        {
            end_run(n, content, i, date, RunEnd::Faded);
            continue;
        }
        // The arc is about these people: if one no longer fills their role
        // (a new rival, a debt repaid another way), it fades.
        let still = def.roles.iter().all(|role| {
            super::actor_of(&r.cast, &role.id)
                .is_some_and(|a| cast::still_cast(v, role, a, &r.cast, None))
        });
        if !still {
            end_run(n, content, i, date, RunEnd::Faded);
            continue;
        }
        if !stage.advance.iter().all(|c| cast::holds(v, c, &r.cast)) {
            continue;
        }
        let run_id = r.id;
        if let Some(dk) = &stage.decision {
            let Some(ddef) = content.dilemma(dk) else {
                end_run(n, content, i, date, RunEnd::Removed);
                continue;
            };
            let sig = n.chapter.as_ref().map(|c| c.significant).unwrap_or(0);
            // A storyline's turn has priority: it waits for room (pending
            // decisions, the budget), not for the spacing between decisions.
            if n.pending().count() >= d.max_pending as usize
                || (sig >= b.sig_max && !climax_due(n, d, b, date))
            {
                continue;
            }
            let preset = n.storylines[i].cast.clone();
            let Some(mut c) = evaluate(v, n, ddef, &preset, None, &[]) else {
                continue;
            };
            c.reasons.push(Reason::Storyline {
                storyline: def.id.clone(),
            });
            c.relevance += d.relevance.storyline;
            let merged = c.cast.clone();
            // A storyline's turn in the climax window is the chapter's climax.
            let weight = if climax_due(n, d, b, date) {
                Weight::Climax
            } else {
                Weight::Significant
            };
            let id = open(n, v, out, c, weight, Some(run_id));
            let r = &mut n.storylines[i];
            r.cast = merged;
            r.decision = Some(id);
            r.entered = true;
            r.stages.push((stage.id.clone(), date));
            let actors = r.cast.iter().map(|e| e.actor).collect();
            n.log(
                date,
                FactKind::StorylineStage {
                    run: run_id,
                    stage: stage.id.clone(),
                },
                actors,
                true,
                0.5,
            );
        } else {
            if !can_minor(n, d, b, date) {
                continue;
            }
            if let Some(ch) = n.chapter.as_mut() {
                ch.minor += 1;
                ch.last_minor = Some(date);
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
}

/// Requests (DEC-032): a goal that needs the player.
fn requests(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>, b: Budget) {
    let content = v.content();
    let d = &content.director;
    let date = v.date();
    if !can_minor(n, d, b, date) || n.pending().count() >= d.max_pending as usize {
        return;
    }
    let mut best: Option<Candidate<'_>> = None;
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
        if n.cooldowns.get(rk).is_some_and(|x| *x > date)
            || n.pending().any(|p| p.template == rdef.id)
        {
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
        if best.as_ref().is_none_or(|x| c.relevance > x.relevance) {
            best = Some(c);
        }
    }
    if let Some(c) = best.filter(|c| c.relevance >= d.min_relevance) {
        let (h, k) = c.goal.clone().unwrap_or((CharacterId(0), String::new()));
        let since = v
            .pol
            .characters
            .goals
            .iter()
            .find(|g| g.holder == h && g.goal == k)
            .map(|g| g.since)
            .unwrap_or(date);
        open(n, v, out, c, Weight::Minor, None);
        n.requests_raised.push((h, k, since));
        if n.requests_raised.len() > 200 {
            n.requests_raised.remove(0);
        }
    }
}

fn best<'c>(
    cands: Vec<Candidate<'c>>,
    f: &dyn Fn(&Candidate<'c>) -> bool,
) -> Option<Candidate<'c>> {
    cands.into_iter().filter(|c| f(c)).max_by(|a, b| {
        a.relevance
            .total_cmp(&b.relevance)
            .then(b.def.id.cmp(&a.def.id))
    })
}

/// Dilemmas: the climax in its window, else a significant decision when the
/// tension curve calls for one (or the chapter is short of its minimum).
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
    let content = v.content();
    let d = &content.director;
    let date = v.date();
    if !can_decide(n, d, date) {
        return;
    }
    let Some(ch) = n.chapter.clone() else { return };
    let staged: Vec<&str> = n
        .active_runs()
        .filter_map(|r| content.storyline(&r.template))
        .flat_map(|s| s.stages.iter().filter_map(|st| st.decision.as_deref()))
        .collect();
    // The climax is the best grounded situation in its window: a theme
    // from earlier in the chapter may come to a head (no cooldown, no
    // relevance floor; repetition still weighs it down).
    let climax = climax_due(n, d, b, date);
    let mut cands: Vec<Candidate<'_>> = Vec::new();
    for def in &content.dilemmas {
        if def.category == DilemmaCategory::Request
            || def.storyline_only
            || staged.contains(&def.id.as_str())
            || (!climax && n.cooldowns.get(&def.id).is_some_and(|x| *x > date))
            || n.pending().any(|p| p.template == def.id)
        {
            continue;
        }
        if let Some(c) = evaluate(v, n, def, &[], None, &[]) {
            cands.push(c);
        }
    }
    let shaped = profile.is_some_and(|p| p.tension_curve);
    if climax {
        if let Some(c) = best(cands, &|_| true) {
            open(n, v, out, c, Weight::Climax, None);
        }
        return;
    }
    cands.retain(|c| c.relevance >= d.min_relevance);
    if !paced(ch.significant, b.sig_max, x) {
        return;
    }
    let behind = x >= d.tension.catch_up_at && ch.significant < b.sig_min;
    let allowed = !shaped || behind || actual < target + d.tension.slack;
    if !allowed {
        return;
    }
    // Opportunities arrive when the chapter is quiet (the director times them).
    let quiet = !shaped || actual < target;
    if let Some(c) = best(cands, &|c| {
        c.def.category == DilemmaCategory::Significant && (!c.def.opportunity || quiet || behind)
    }) {
        open(n, v, out, c, Weight::Significant, None);
    }
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
        let spot = can_minor(n, d, b, date);
        if spot && let Some(ch) = n.chapter.as_mut() {
            ch.minor += 1;
            ch.last_minor = Some(date);
        }
        let id = n.log(date, kind.clone(), actors, spot, news);
        if spot {
            out.events.record(
                date,
                EventKind::StoryDevelopment {
                    fact: id,
                    municipality: m,
                },
                vec![],
            );
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
    // Variety: people spotlighted in this or the last chapter are cast last.
    let recent_chapter = n.chapters.last().map(|c| c.index).unwrap_or(0);
    v.recent = n
        .decisions
        .iter()
        .filter(|d| d.chapter >= recent_chapter)
        .flat_map(|d| d.cast.iter())
        .filter_map(|e| match e.actor {
            Actor::Person { id } => Some(id),
            _ => None,
        })
        .collect();
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
    chapters(n, &v, out);
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
    let actual = world + d.tension.pending_weight * pending_load(n);
    n.tension = actual as f32;
    if let Some(ch) = n.chapter.as_mut() {
        ch.peak_tension = ch.peak_tension.max(actual as f32);
    }
    lapse(n, &v, out);
    start_runs(n, &v);
    advance_runs(n, &v, out, b);
    requests(n, &v, out, b);
    dilemmas(n, &v, out, profile.as_ref(), b, x, target, actual);
    developments(n, &v, out, b, first);
    n.trim(d.history as usize);
}
