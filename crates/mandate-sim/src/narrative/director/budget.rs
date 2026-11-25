//! Budgets, the tension curve, relevance, tiers and cooldowns.

use super::*;

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

/// Tension from the simulation alone (the hottest arc or the worst
/// crisis, the race, danger): what the exogenous levers read, so that the
/// spotlight never changes chance. An arc's heat stands for its matter
/// (STAGED_ARCS §1.6); a crisis that no arc tells yet still counts by its
/// severity.
pub(super) fn world_tension(v: &View<'_>, chapter: Option<&Chapter>) -> f64 {
    let t = &v.content().director.tension;
    let crisis = v
        .town_character()
        .and_then(|c| c.crisis())
        .map(|c| c.severity as f64)
        .unwrap_or(0.0)
        .max(super::arcs::heat(v));
    let race = if v.campaigning() {
        0.5 + 0.5 * chapter.map(|c| c.progress(v.date())).unwrap_or(0.0)
    } else {
        0.0
    };
    let danger = track_dangers(v.r).iter().map(|x| x.3).fold(0.0, f64::max);
    t.crisis_weight * crisis + t.race_weight * race + t.danger_weight * danger
}

pub(super) fn pending_load(n: &NarrativeState) -> f64 {
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
            super::super::KEY_JITTER,
            date.0 as u64,
            mandate_core::hash::derive_u64(key, &[]),
            first,
        ],
    )
    .next_f64()
}

/// Relevance (DEC-033): connection to the player, their ward, the live
/// crisis, memories, running arcs, pressure tracks, the coming election, and
/// the town's concerns; a template or person already spotlighted this
/// chapter weighs less.
pub(super) fn relevance(
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
            Reason::Arc { .. } => w.arc,
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
    /// Consequences (`answered`) whose window closes within
    /// `tier_a_window_days`; for the lead, a material arc.
    A,
    /// Templates of the chapter's theme; for the lead, a standing arc of
    /// this kind of chapter.
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
