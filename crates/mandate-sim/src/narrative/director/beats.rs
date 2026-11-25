//! Setbacks, answers, beats, the rescue, and spotlighted developments.

use super::*;
use crate::spine::{Cause, Causes, Link, Process};

/// Setbacks (ENGAGEMENT §4.3, RimWorld's adaptation) since the last weekly
/// step: an election lost, Standing or viability warned about. (A scandal
/// about the player is noted when its decision is raised.)
pub(super) fn setbacks(n: &mut NarrativeState, v: &View<'_>) {
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
pub(super) fn record_answers(n: &mut NarrativeState) {
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
    if n.answers.len() > super::super::ANSWERS_CAP {
        let drop = n.answers.len() - super::super::ANSWERS_CAP;
        n.answers.drain(..drop);
    }
}

/// Press stories about the player, by date (a minor beat; DEC-034).
pub(super) fn press_dates(v: &View<'_>) -> Vec<SimDate> {
    let me = v.player().map(|p| p.politician);
    v.r.media()
        .stories
        .iter()
        .filter(|s| s.subject.is_some() && s.subject == me)
        .map(|s| s.date)
        .collect()
}

/// Spotlit development facts (minor beats).
fn is_beat(k: &FactKind) -> bool {
    matches!(
        k,
        FactKind::TownEvent { .. }
            | FactKind::CrisisBegan { .. }
            | FactKind::CrisisWorse { .. }
            | FactKind::GoalTaken { .. }
            | FactKind::RivalNamed { .. }
    )
}

/// The last day something happened for the player: a decision opened, an
/// arc's stage entered or ended, news of the town spotlighted, press about
/// them, an ask offered or resolved (the asks and arcs domains, read only).
fn last_beat(n: &NarrativeState, v: &View<'_>) -> Option<SimDate> {
    let decided = n.decisions.iter().map(|x| x.opened).max();
    let fact = n
        .facts
        .iter()
        .rev()
        .find(|f| f.spotlit && is_beat(&f.kind))
        .map(|f| f.date);
    let press = press_dates(v).into_iter().max();
    let asked =
        v.r.asks()
            .asks
            .iter()
            .flat_map(|a| [Some(a.opened), a.status.resolved_on()])
            .flatten()
            .max();
    [
        decided,
        fact,
        press,
        asked,
        super::arcs::last_stage(v),
        n.play_began,
    ]
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
pub(super) fn rescue(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>, b: Budget) {
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
    // News of an event the log holds is that event, made known.
    let about = match &kind {
        FactKind::TownEvent { event } => Some(*event),
        _ => None,
    };
    let id = n.log(date, kind, actors, true, news);
    out.events.record(
        date,
        EventKind::StoryDevelopment {
            fact: id,
            municipality: v.town().unwrap_or(0),
        },
        Causes::from(Cause::Process(Process::Director)).and_event(about, Link::Revealed),
    );
}

fn band(sev: f32) -> u8 {
    (sev >= 0.34) as u8 + (sev >= 0.67) as u8
}

/// Developments: news of the town the player should know (minor beats).
pub(super) fn developments(
    n: &mut NarrativeState,
    v: &View<'_>,
    out: &mut Outs<'_>,
    b: Budget,
    first: bool,
) {
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
