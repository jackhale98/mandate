//! The daily step: read the player's new deeds and the events that matter
//! into the deed log, then resolve asks: an offer left unanswered lapses
//! (declined), an accepted ask is kept when its needs are met in order,
//! broken at its deadline (or at once by a deed against it), and void when
//! it becomes impossible through no fault of the player. Each ask resolves
//! exactly once.

use super::effects::{self, Outcome};
use super::{Ask, AskState, AskStatus, Deed, DeedRecord, Need, StatementAbout, StatementNeed};
use crate::events::EventId;
use crate::events::EventKind;
use crate::policy::{EndReason, PolicyStatus};
use crate::scheduler::{Outs, Reads};
use crate::spine::{Cause, Causes, Link, Process};
use mandate_content::MechanismDef;
use mandate_core::SimDate;

/// What happened since the last step, for resolution.
#[derive(Default)]
pub(crate) struct News {
    pub deeds: Vec<(u32, DeedRecord)>,
    /// One of the player's own hidden facts came to light.
    pub surfaced: bool,
}

/// Read the event log since the last step.
pub(crate) fn read_events(a: &mut AskState, r: &Reads<'_>, out: &Outs<'_>) -> News {
    let mut news = News::default();
    let me = r.player.map(|p| p.politician.0);
    let mut newest = a.last_event;
    for e in out.events.iter().filter(|e| e.id.0 > a.last_event) {
        newest = newest.max(e.id.0);
        match &e.kind {
            EventKind::PlayerDeed { deed } => news.deeds.push((
                e.id.0,
                DeedRecord {
                    on: e.date,
                    deed: deed.clone(),
                },
            )),
            EventKind::ScandalBroke { politician, .. } if Some(*politician) == me => {
                news.surfaced = true;
            }
            _ => {}
        }
    }
    a.last_event = newest;
    let keep = r.content.ask_params.deeds_kept as usize;
    a.deeds.extend(news.deeds.iter().map(|(_, d)| d.clone()));
    if a.deeds.len() > keep {
        let drop = a.deeds.len() - keep;
        a.deeds.drain(..drop);
    }
    news
}

/// How one deed bears on a need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Met,
    /// A part of it (one neighbourhood of several).
    Part,
    Broken,
    Nothing,
}

/// Whether a policy instance's template is among `templates`.
fn template_in(r: &Reads<'_>, policy: u32, templates: &[String]) -> bool {
    r.policy()
        .instances
        .get(policy as usize)
        .is_some_and(|p| templates.contains(&p.template))
}

/// A bill raises the town's taxes: a `local_tax_rate` set above zero.
pub fn raises_taxes(r: &Reads<'_>, policy: u32) -> bool {
    let Some(p) = r.policy().instances.get(policy as usize) else {
        return false;
    };
    let Some(t) = r.content.policies.get(&p.template) else {
        return false;
    };
    t.mechanisms.iter().any(|m| match m {
        MechanismDef::LocalTaxRate { param } => p.param(param) > 0.0,
        _ => false,
    })
}

/// A bill has a tag (`against_all`).
pub(crate) fn tagged(r: &Reads<'_>, policy: u32, tag: &str) -> bool {
    tag == mandate_content::asks::TAG_TAX_RISE && raises_taxes(r, policy)
}

/// The templates that answer a crisis kind, or a worry.
pub fn crisis_answers(content: &mandate_content::ContentBundle, kind: &str) -> Vec<String> {
    content
        .crises
        .iter()
        .find(|c| c.id == kind)
        .map(|c| c.policies.clone())
        .unwrap_or_default()
}

pub fn concern_answers(content: &mandate_content::ContentBundle, concern: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in content
        .crises
        .iter()
        .filter(|c| c.concern.as_deref() == Some(concern))
    {
        for p in &c.policies {
            if !out.contains(p) {
                out.push(p.clone());
            }
        }
    }
    out
}

/// The kinds of the crises live in the player's home town.
fn live_crises(r: &Reads<'_>) -> Vec<String> {
    let Some(pl) = r.player else {
        return Vec::new();
    };
    r.politics()
        .characters
        .towns
        .get(pl.home().index())
        .map(|t| {
            t.crises
                .iter()
                .filter(|c| c.severity > 0.0)
                .map(|c| c.kind.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn step(r: &Reads<'_>, ask: &mut Ask, need: &Need, deed: &Deed) -> Step {
    match (need, deed) {
        (Need::Vote { policy, vote }, Deed::Vote { policy: p, vote: v }) if p == policy => {
            if v == vote { Step::Met } else { Step::Broken }
        }
        (Need::VoteAgainstAll { tag }, Deed::Vote { policy, vote })
            if *vote == crate::politics::Vote::Yes && tagged(r, *policy, tag) =>
        {
            Step::Broken
        }
        (Need::VoteAgainstAll { tag }, Deed::Introduce { policy }) if tagged(r, *policy, tag) => {
            Step::Broken
        }
        (Need::Statement { about }, Deed::Statement { about: said }) => {
            let ok = match (about, said) {
                (StatementNeed::Any, _) => true,
                (StatementNeed::Bill { policy }, StatementAbout::Bill { policy: p }) => p == policy,
                (StatementNeed::Bill { .. }, StatementAbout::General) => false,
                (StatementNeed::Crisis { kind }, StatementAbout::General) => {
                    live_crises(r).contains(kind)
                }
                (StatementNeed::Crisis { kind }, StatementAbout::Bill { policy }) => {
                    template_in(r, *policy, &crisis_answers(r.content, kind))
                }
                (StatementNeed::Concern { .. }, StatementAbout::General) => true,
                (StatementNeed::Concern { concern }, StatementAbout::Bill { policy }) => {
                    template_in(r, *policy, &concern_answers(r.content, concern))
                }
            };
            if ok { Step::Met } else { Step::Nothing }
        }
        (Need::TownHall { hoods }, Deed::TownHall { hood }) if hoods.contains(hood) => {
            if !ask.done.contains(hood) {
                ask.done.push(*hood);
            }
            if hoods.iter().all(|h| ask.done.contains(h)) {
                Step::Met
            } else {
                Step::Part
            }
        }
        (Need::Canvass { hood, .. }, Deed::Canvass { hood: h }) if h == hood => Step::Met,
        (
            Need::Canvass {
                hood,
                or_town_hall: true,
            },
            Deed::TownHall { hood: h },
        ) if h == hood => Step::Met,
        (Need::Visit { place }, Deed::Arrive { place: p }) if p == place => Step::Met,
        // A talk need with a choice is met by that choice (`asks::hook`),
        // when the person agrees; any talk with them meets one without.
        (Need::Talk { person }, Deed::Talk { person: p, .. })
            if p == person && super::hook::choice_def(r.content, ask).is_none() =>
        {
            Step::Met
        }
        (Need::Introduce { templates }, Deed::Introduce { policy })
            if template_in(r, *policy, templates) =>
        {
            Step::Met
        }
        (Need::Casework, Deed::Casework { ask: id }) if *id == ask.id => Step::Met,
        (Need::Quiet { person }, Deed::Leak { politician }) => {
            let leaked = r
                .politics()
                .get(crate::politics::PoliticianId(*politician))
                .map(|p| p.character);
            if leaked == Some(*person) {
                Step::Broken
            } else {
                Step::Nothing
            }
        }
        _ => Step::Nothing,
    }
}

/// A state test for the current need: `Some` when it settles the ask.
fn state_check(r: &Reads<'_>, ask: &Ask, need: &Need) -> Option<Result<(), String>> {
    let me = r.player.map(|p| p.politician);
    match need {
        Need::Vote { policy, .. } => {
            let p = r.policy().instances.get(*policy as usize)?;
            if p.status == PolicyStatus::UnderConsideration {
                // Still before the council, but the player lost their vote.
                let at = p.at();
                let c = r.content.constitutions.get(&r.meta.constitution)?;
                let can = me.is_some_and(|me| {
                    crate::politics::holds_power_at(
                        r.politics(),
                        c,
                        me,
                        mandate_content::Power::ApproveOrdinance,
                        at,
                    ) || crate::politics::holds_power_at(
                        r.politics(),
                        c,
                        me,
                        mandate_content::Power::Veto,
                        at,
                    )
                });
                return (!can).then(|| Err("ask-void-no-vote".into()));
            }
            if p.end_reason == Some(EndReason::Withdrawn) {
                return Some(Err("ask-void-withdrawn".into()));
            }
            // Decided without the vote asked for: broken (a vote the player
            // did declare was read as a deed first).
            Some(Err(String::new()))
        }
        // Met whoever fixed it (DEC-091: the world does not wait).
        Need::Fixed { fault } => match r.upkeep().and_then(|u| u.get(*fault)) {
            Some(f) if f.fixed() => Some(Ok(())),
            Some(_) => None,
            None => Some(Err("ask-void-fault-gone".into())),
        },
        Need::Enacted {
            templates,
            sponsor_player,
        } => {
            let mut half = false;
            for p in &r.policy().instances {
                if !templates.contains(&p.template) {
                    continue;
                }
                if *sponsor_player && p.sponsor != me {
                    continue;
                }
                if p.enacted_on.is_some_and(|d| d >= ask.opened) {
                    return Some(Ok(()));
                }
                if ask.promise
                    && p.sponsor == me
                    && p.introduced_on.is_some_and(|d| d >= ask.opened)
                    && p.status == PolicyStatus::Ended
                    && matches!(
                        p.end_reason,
                        Some(EndReason::Defeated) | Some(EndReason::Vetoed)
                    )
                {
                    half = true;
                }
            }
            half.then(|| Err("ask-void-half-kept".into()))
        }
        _ => None,
    }
}

/// How an open ask settles today.
enum Settle {
    Kept,
    Broken,
    /// An offer left unanswered past `respond_by` (declined).
    Lapsed,
    Void(String),
}

/// Resolve every open ask for today.
pub(crate) fn resolve_all(a: &mut AskState, r: &Reads<'_>, out: &mut Outs<'_>, news: &News) {
    let date = r.date;
    let Some(pl) = r.player else {
        return;
    };
    let home = pl.home().0;
    for i in 0..a.asks.len() {
        if !a.asks[i].open() {
            continue;
        }
        let settle = {
            let ask = &mut a.asks[i];
            settle_one(r, ask, news, date, home)
        };
        if let Some((s, mut deed)) = settle {
            // Kept by a repair (whoever made it): the fault's last event.
            if deed.is_none()
                && let Some(f) = a.asks[i].needs.iter().find_map(|n| match n {
                    Need::Fixed { fault } => Some(*fault),
                    _ => None,
                })
            {
                deed = out.events.head(crate::spine::Thread::Fault(f.0));
            }
            finish(a, i, s, deed, r, out);
        }
    }
}

fn settle_one(
    r: &Reads<'_>,
    ask: &mut Ask,
    news: &News,
    date: SimDate,
    home: u32,
) -> Option<(Settle, Option<EventId>)> {
    // The deed that last moved the ask on, or broke it (the spine's link).
    let mut by: Option<EventId> = None;
    settle_inner(r, ask, news, date, home, &mut by).map(|s| (s, by))
}

fn settle_inner(
    r: &Reads<'_>,
    ask: &mut Ask,
    news: &News,
    date: SimDate,
    home: u32,
    by: &mut Option<EventId>,
) -> Option<Settle> {
    // The asker left town (a person), or the race was lost (a promise).
    if let Some(p) = ask.asker_person()
        && crate::characters::municipality_of(r.politics(), r.world, p) != Some(home)
    {
        return Some(Settle::Void("ask-void-left".into()));
    }
    if let Some(post) = ask.race
        && let Some(pl) = r.player
    {
        let pol = r.politics();
        let running = crate::characters::is_candidate(pol, pl.politician);
        let holds = crate::politics::posts_of(pol, pl.politician).contains(&post);
        if !running && !holds && ask.accepted() {
            return Some(Settle::Void("ask-void-lost".into()));
        }
    }
    if ask.offered() {
        if date > ask.respond_by {
            return Some(Settle::Lapsed);
        }
        // An offer whose bill has gone is withdrawn quietly.
        if let Some(Need::Vote { policy, .. }) = ask.needs.first()
            && r.policy()
                .instances
                .get(*policy as usize)
                .is_some_and(|p| p.status != PolicyStatus::UnderConsideration)
        {
            return Some(Settle::Void("ask-void-withdrawn".into()));
        }
        // An offer about a fault mended before it was answered.
        if let Some(Need::Fixed { fault }) = ask.needs.first()
            && r.upkeep()
                .and_then(|u| u.get(*fault))
                .is_none_or(|f| f.fixed())
        {
            return Some(Settle::Void("ask-void-fixed".into()));
        }
        return None;
    }
    let AskStatus::Accepted { on } = ask.status else {
        return None;
    };
    // Deeds since it was accepted, in order.
    for (ev, d) in news.deeds.iter().filter(|(_, d)| d.on >= on) {
        let Some(need) = ask.current().cloned() else {
            break;
        };
        match step(r, ask, &need, &d.deed) {
            Step::Met => {
                ask.step = ask.step.saturating_add(1);
                ask.done.clear();
                *by = Some(EventId(*ev));
            }
            Step::Broken => {
                *by = Some(EventId(*ev));
                return Some(Settle::Broken);
            }
            Step::Part | Step::Nothing => {}
        }
    }
    if news.surfaced
        && ask
            .needs
            .iter()
            .any(|n| matches!(n, Need::NoSurfacedHiddenFact))
    {
        return Some(Settle::Broken);
    }
    // State tests for the current need.
    while let Some(need) = ask.current().cloned() {
        match state_check(r, ask, &need) {
            Some(Ok(())) => {
                ask.step = ask.step.saturating_add(1);
                ask.done.clear();
            }
            Some(Err(reason)) if reason.is_empty() => return Some(Settle::Broken),
            Some(Err(reason)) => return Some(Settle::Void(reason)),
            None => break,
        }
    }
    let rest_watch = ask.needs[(ask.step as usize).min(ask.needs.len())..]
        .iter()
        .all(Need::watch);
    if ask.step as usize >= ask.needs.len() {
        return Some(Settle::Kept);
    }
    if date > ask.due {
        return Some(if rest_watch {
            Settle::Kept
        } else {
            Settle::Broken
        });
    }
    None
}

/// Close an ask: status, effects, the tally, an event.
fn finish(
    a: &mut AskState,
    i: usize,
    s: Settle,
    deed: Option<EventId>,
    r: &Reads<'_>,
    out: &mut Outs<'_>,
) {
    let date = r.date;
    let ask = &a.asks[i];
    let (status, outcome) = match s {
        Settle::Kept => (AskStatus::Kept { on: date }, Some(Outcome::Kept)),
        Settle::Broken => (AskStatus::Broken { on: date }, Some(Outcome::Broken)),
        Settle::Lapsed => (
            AskStatus::Declined {
                on: date,
                unanswered: true,
            },
            Some(Outcome::Declined),
        ),
        Settle::Void(reason) => (AskStatus::Void { on: date, reason }, None),
    };
    let mut fx = Vec::new();
    if let Some(o) = outcome {
        let defs = template_effects(r, ask, o);
        fx.extend(effects::requests(r, ask, &defs));
        fx.extend(effects::automatic(r, ask, o));
    }
    let t = &mut a.tally;
    let promise = a.asks[i].promise;
    match (&status, promise) {
        (AskStatus::Kept { .. }, false) => t.kept += 1,
        (AskStatus::Kept { .. }, true) => t.promises_kept += 1,
        (AskStatus::Broken { .. }, false) => t.broken += 1,
        (AskStatus::Broken { .. }, true) => t.promises_broken += 1,
        (AskStatus::Void { .. }, false) => t.void += 1,
        (AskStatus::Void { .. }, true) => t.promises_void += 1,
        (AskStatus::Declined { unanswered, .. }, _) => {
            t.declined += 1;
            if *unanswered {
                t.lapsed += 1;
            }
        }
        _ => {}
    }
    let ask = &mut a.asks[i];
    ask.status = status;
    // The deed that settled it, when one did; else the deadline or the
    // state of the world the asks system read. The effects are requested
    // after the event (same order among themselves), naming it.
    let ev = out.events.record(
        date,
        EventKind::AskResolved {
            ask: ask.id.0,
            template: ask.template.clone(),
            outcome: ask.status.key().into(),
            public: ask.public,
        },
        Causes::from(Cause::Process(Process::Asks)).and_event(deed, Link::Caused),
    );
    for e in fx {
        out.effects.push(e, Cause::caused_by(ev));
    }
}

/// The template's effects for an outcome.
pub(crate) fn template_effects(
    r: &Reads<'_>,
    ask: &Ask,
    o: Outcome,
) -> Vec<mandate_content::EffectDef> {
    let c = r.content;
    if ask.promise {
        return c
            .promise(&ask.template)
            .map(|t| match o {
                Outcome::Kept => t.keep.clone(),
                Outcome::Broken => t.broken.clone(),
                Outcome::Declined => Vec::new(),
            })
            .unwrap_or_default();
    }
    c.ask(&ask.template)
        .map(|t| match o {
            Outcome::Kept => t.keep.clone(),
            Outcome::Broken => t.broken.clone(),
            Outcome::Declined => t.decline.clone(),
        })
        .unwrap_or_default()
}

/// Keep `history` resolved asks (and every open one), oldest dropped first.
pub(crate) fn trim(a: &mut AskState, history: usize) {
    let resolved = a.asks.iter().filter(|x| !x.open()).count();
    if resolved <= history {
        return;
    }
    let mut drop = resolved - history;
    a.asks.retain(|x| {
        if drop > 0 && !x.open() {
            drop -= 1;
            false
        } else {
            true
        }
    });
}
