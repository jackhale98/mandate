//! Raising decisions and requests: gathering candidates, dilemmas, and the peak.

use super::arcs::staged;
use super::*;

/// Decisions raised by a story about the player or an investigation of
/// them (DEC-034, DEC-006): fairness first, so never budgeted, spaced, or
/// capped; each story and case is raised once.
pub(super) fn raise(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>) {
    let Some(pl) = v.player() else { return };
    if !pl.status.active() {
        return;
    }
    let content = v.content();
    let me = pl.politician;
    let mut todo: Vec<(super::super::About, mandate_content::RaisedBy, &'static str)> = Vec::new();
    for s in v.r.media().scandals.iter() {
        let a = super::super::About::Scandal { id: s.id };
        if s.subject == me && s.running() && s.response.is_none() && !n.raised.contains(&a) {
            todo.push((a, mandate_content::RaisedBy::Scandal, "standing"));
        }
    }
    for c in v.r.justice().cases.iter() {
        let a = super::super::About::Case { id: c.id };
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
        if matches!(about, super::super::About::Scandal { .. }) && n.phase.is_some() {
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
pub(super) fn requests(
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
        if !available(n, d, rdef, date) {
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
/// Its cooldown has passed, it is not pending, it was not spotlighted this
/// chapter, and the career is old enough.
fn available(n: &NarrativeState, d: &DirectorParams, def: &DilemmaDef, date: SimDate) -> bool {
    if n.cooldowns.get(&def.id).is_some_and(|x| *x > date)
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
pub(super) fn career_days(n: &NarrativeState, date: SimDate) -> u32 {
    n.play_began
        .filter(|b| *b <= date)
        .map(|b| b.days_until(date))
        .unwrap_or(0)
}

/// The dilemmas the spotlight could present now, with their tiers: not
/// requests, raised decisions, an arc's own (`arc_only`), or one a live
/// run names as a beat; available (cooldowns, no repeat,
/// `min_career_days`); at or above the relevance floor. None of them is a
/// climax: a chapter's climax is its lead arc's turn (DEC-109).
pub(super) fn gather<'c>(v: &View<'c>, n: &NarrativeState) -> Vec<Candidate<'c>> {
    let content = v.content();
    let d = &content.director;
    let date = v.date();
    let staged = staged(v);
    let mut cands: Vec<Candidate<'_>> = Vec::new();
    for def in &content.dilemmas {
        if def.category != DilemmaCategory::Significant
            || def.arc_only
            || def.raised_by.is_some()
            || staged.contains(&def.id.as_str())
            || !available(n, d, def, date)
        {
            continue;
        }
        if let Some(c) = evaluate(v, n, def, &[], None, &[])
            && c.relevance >= d.min_relevance
        {
            cands.push(c);
        }
    }
    cands
}

/// Without a pacing cycle (Unfiltered): a significant decision when the
/// tension curve calls for one (or the chapter is short of its minimum),
/// by the pick.
#[allow(clippy::too_many_arguments)]
pub(super) fn dilemmas(
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
    let mut cands = gather(v, n);
    cands.retain(|c| !c.def.opportunity || quiet || behind);
    if let Some(c) = pick(v, n, cands, false, 3) {
        open(n, v, out, c, Weight::Significant, None, None);
    }
}

/// The peak (ENGAGEMENT §4.3): one significant decision, by tier (closing
/// consequences, the chapter's theme, the rest). The climax peak is the
/// lead arc's turn, whose decision the arc asked for (`narrative::offered`):
/// this opens nothing in it. After the climax, the coda allows
/// `post_climax_significant` decisions, and only consequences,
/// opportunities or `budget` templates. The tension curve is the envelope.
#[allow(clippy::too_many_arguments)]
pub(super) fn peak(
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
    let Some(ph) = n
        .phase
        .clone()
        .filter(|p| p.kind == PhaseKind::Peak && p.decision.is_none() && !p.climax)
    else {
        return;
    };
    let Some(ch) = n.chapter.clone() else { return };
    if n.pending().count() >= d.max_pending as usize || ch.significant >= b.sig_max {
        return;
    }
    // The coda: the first peak after the climax opens at most
    // `post_climax_significant`, of the right kind.
    let coda = ch.kind == ChapterKind::Session && is_coda(n, &ch);
    if coda && d.post_climax_significant == 0 {
        return;
    }
    let shaped = profile.is_some_and(|p| p.tension_curve);
    let behind = x >= d.tension.catch_up_at && ch.significant < b.sig_min;
    let allowed = !shaped || behind || actual < target + d.tension.slack;
    if !(can_decide(n, d, date) && allowed && paced(ch.significant, b.sig_max, x)) {
        return;
    }
    let quiet = !shaped || actual < target;
    let mut cands = gather(v, n);
    cands.retain(|c| !c.def.opportunity || quiet || behind || ph.eased);
    if coda {
        cands.retain(|c| {
            c.def.opportunity || is_consequence(c.def) || c.def.themes.iter().any(|t| t == "budget")
        });
    }
    if let Some(c) = pick(v, n, cands, ph.eased, 3) {
        open(n, v, out, c, Weight::Significant, None, None);
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
