//! Act endings (DEC-006; the endings registry, DEC-098 note). The first
//! entry of `endings.jsonc` whose condition holds ends the act. Conditions
//! read posts, acts, powers and terms through the constitution, never an
//! office's name, so a regional or national act ends by data alone.

use crate::legacy::{LegacyKind, LegacyRecord, act_of_post};
use crate::player::RunStatus;
use mandate_content::act::{EndingDef, EndingWhen, PlayerEventKey, StatusKey, TrackKey};
use mandate_content::{ConstitutionDef, ContentBundle};

/// A track's value (0..100), or Mandate.
pub fn track(pl: &crate::player::PlayerState, t: TrackKey) -> f32 {
    match t {
        TrackKey::Viability => pl.tracks.viability,
        TrackKey::LegalJeopardy => pl.tracks.legal_jeopardy,
        TrackKey::Threat => pl.tracks.threat,
        TrackKey::Standing => pl.tracks.standing,
        TrackKey::Health => pl.tracks.health,
        TrackKey::Mandate => pl.mandate,
    }
}

fn holds(state: &crate::State, c: &ConstitutionDef, rec: &LegacyRecord, w: &EndingWhen) -> bool {
    let Some(pl) = state.player.as_ref() else {
        return false;
    };
    let act = c.career.acts.iter().position(|a| a.id == rec.act);
    let posts = crate::player::posts(state);
    match w {
        EndingWhen::HoldsLaterAct => posts
            .iter()
            .filter_map(|p| act_of_post(c, *p))
            .any(|i| act.is_some_and(|a| i > a)),
        EndingWhen::WonPostWith { powers } => posts.iter().any(|p| {
            !rec.start_posts.contains(p)
                && act_of_post(c, *p) == act
                && p.def(c)
                    .is_some_and(|d| powers.iter().all(|x| d.powers().contains(x)))
        }),
        EndingWhen::LostSeat => {
            posts.is_empty()
                && pl.status.active()
                && rec.count(|k| matches!(k, LegacyKind::Election { won: false, .. })) > 0
        }
        EndingWhen::Event { event } => {
            rec.count(|k| match event {
                PlayerEventKey::Deselected => matches!(k, LegacyKind::Deselected),
                PlayerEventKey::RemovedFromOffice => {
                    matches!(k, LegacyKind::RemovedFromOffice { .. })
                }
                PlayerEventKey::Imprisoned => matches!(k, LegacyKind::Imprisoned { .. }),
            }) > 0
        }
        EndingWhen::Status { status } => status.iter().any(|s| match s {
            StatusKey::Imprisoned => matches!(pl.status, RunStatus::Imprisoned { .. }),
            StatusKey::Ended => matches!(pl.status, RunStatus::Ended { .. }),
        }),
        EndingWhen::TrackAtMost { track: t, value } => track(pl, *t) <= *value,
        EndingWhen::Served { terms } => posts.iter().any(|p| served(state, c, rec, *p, *terms)),
        EndingWhen::SteppedAway => state.act.stepped_away.is_some(),
        EndingWhen::Any { any } => any.iter().any(|x| holds(state, c, rec, x)),
    }
}

/// The player has won `terms` elections to `post` this act (the term
/// they were serving when it began is not a full one), and the last term is at its
/// end: its campaign has opened (the post's term and calendar in the
/// constitution). The act ends there rather than in another campaign.
fn served(
    state: &crate::State,
    c: &ConstitutionDef,
    rec: &LegacyRecord,
    post: crate::politics::Post,
    terms: u8,
) -> bool {
    let Some((cal, term)) = crate::politics::elections::calendar_for(c, post) else {
        return false;
    };
    let wins: Vec<mandate_core::SimDate> = rec
        .entries
        .iter()
        .filter_map(|e| match e.kind {
            LegacyKind::Election {
                post: p, won: true, ..
            } if p == post => Some(e.date),
            _ => None,
        })
        .collect();
    if wins.len() < terms as usize {
        return false;
    }
    let Some(last) = wins.last() else {
        return false;
    };
    let term_days = term * mandate_core::time::DAYS_PER_YEAR;
    let campaign_days = cal.campaign_weeks as u32 * 7;
    last.days_until(state.calendar.date) >= term_days.saturating_sub(campaign_days)
}

/// The ending that holds now for the current act, if any.
pub fn check<'a>(state: &crate::State, content: &'a ContentBundle) -> Option<&'a EndingDef> {
    let rec = state.act.current()?;
    if rec.closed.is_some() {
        return None;
    }
    let c = content.constitutions.get(&state.meta.constitution)?;
    content
        .endings
        .iter()
        .filter(|e| e.acts.is_empty() || e.acts.contains(&rec.act))
        .find(|e| holds(state, c, rec, &e.when))
}
