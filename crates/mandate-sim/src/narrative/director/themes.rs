//! Sessions and chapters, their themes, chance and discovery, and lapsing decisions.

use super::*;

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
pub(super) fn focus_towns(v: &View<'_>) -> Vec<u32> {
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
        &[super::super::KEY_THEME, index as u64],
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
pub(super) fn chapters(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>) -> bool {
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
pub(super) fn chance(n: &mut NarrativeState, v: &View<'_>, profile: Option<&StorytellerDef>) {
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
pub(super) fn discovery(
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
                &[super::super::KEY_DISCOVERY, h.id as u64],
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
            (&r.cast, super::super::actor_of(&d.cast, &r.id))
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

pub(super) fn lapse(n: &mut NarrativeState, v: &View<'_>, out: &mut Outs<'_>) {
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
