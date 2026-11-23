//! Storyline runs: starting, staging, advancing and ending them.

use super::*;

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

pub(super) fn start_runs(n: &mut NarrativeState, v: &View<'_>) {
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
pub(super) fn staged_all<'c>(
    v: &View<'c>,
    n: &NarrativeState,
    due: &[usize],
) -> Vec<Candidate<'c>> {
    due.iter()
        .filter_map(|&i| staged_candidate(v, n, i))
        .collect()
}

/// Open a storyline stage's decision and enter the stage.
pub(super) fn open_staged(
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
pub(super) enum RunStep {
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

pub(super) fn run_step(v: &View<'_>, n: &NarrativeState, i: usize) -> RunStep {
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
        super::super::actor_of(&r.cast, &role.id)
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
pub(super) fn advance_runs(
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
pub(super) fn open_due_stages(
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
