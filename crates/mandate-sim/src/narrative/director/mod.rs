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

mod beats;
mod budget;
mod candidates;
mod cycle;
mod fairness;
mod lab;
mod opening;
mod runs;
mod spotlight;
mod themes;

use beats::*;
pub use budget::*;
pub use candidates::*;
pub use cycle::*;
pub use fairness::*;
pub use lab::*;
pub use opening::*;
pub(crate) use runs::*;
use spotlight::*;
pub use themes::*;

fn active(r: &Reads<'_>) -> bool {
    r.player.is_some() && r.calendar.phase != GamePhase::BurnIn
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
