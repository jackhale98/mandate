//! Opening a spotlight, the climax horizon, and the decision and minor-beat gates.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn open(
    n: &mut NarrativeState,
    v: &View<'_>,
    out: &mut Outs<'_>,
    c: Candidate<'_>,
    weight: Weight,
    storyline: Option<StorylineRunId>,
    about: Option<super::super::About>,
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
        for person in super::super::learn::cast_people(v, &c.cast) {
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
            if n.climaxes.len() > super::super::CLIMAXES_CAP {
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
pub(super) fn climax_due(n: &NarrativeState, v: &View<'_>, b: Budget) -> bool {
    let Some(ch) = n.chapter.as_ref() else {
        return false;
    };
    let x = climax_progress(v, ch, v.date());
    let w = &v.content().director.tension.climax_window;
    ch.kind == ChapterKind::Session && ch.climaxes < b.climax && x >= w[0] && x <= w[1]
}

pub(super) fn can_decide(n: &NarrativeState, d: &DirectorParams, date: SimDate) -> bool {
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
pub(super) fn paced(count: u8, max: u8, x: f64) -> bool {
    count < max && (count as f64) < (max as f64 * x).ceil() + 2.0
}

pub(super) fn can_minor(n: &NarrativeState, d: &DirectorParams, b: Budget, date: SimDate) -> bool {
    n.chapter.as_ref().is_some_and(|c| {
        paced(c.minor, b.minor_max, c.progress(date))
            && c.last_minor
                .is_none_or(|l| l.add_days(d.development_spacing_days) <= date)
    })
}

/// Room for a minor beat; `force` (the quiet rescue) waives pacing and
/// spacing, never the budget.
pub(super) fn minor_room(
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
