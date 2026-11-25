//! Candidates and the keyed pick (relevance x novelty within a band), and scene places.

use super::*;

/// A situation ready to be spotlighted.
pub(crate) struct Candidate<'c> {
    pub(crate) def: &'c DilemmaDef,
    pub(crate) cast: Vec<CastEntry>,
    pub(crate) reasons: Vec<Reason>,
    pub(crate) relevance: f64,
    pub(crate) goal: Option<(CharacterId, String)>,
    pub(crate) tier: Tier,
}

/// Roles an `answered` trigger carries over (`same_cast`), from the newest
/// answer it matches, while those actors still fill their roles.
fn answered_preset(v: &View<'_>, def: &DilemmaDef) -> Vec<CastEntry> {
    let mut out: Vec<CastEntry> = Vec::new();
    for c in &def.triggers {
        let Condition::Answered { same_cast, .. } = c else {
            continue;
        };
        if same_cast.is_empty() {
            continue;
        }
        let Some(rec) = cast::answered_records(v, c).first().copied() else {
            continue;
        };
        for r in same_cast {
            let (Some(role), Some(actor)) = (
                def.roles.iter().find(|x| &x.id == r),
                super::super::actor_of(&rec.cast, r),
            ) else {
                continue;
            };
            if !out.iter().any(|e| &e.role == r) && cast::still_cast(v, role, actor, &[], None) {
                out.push(CastEntry {
                    role: r.clone(),
                    actor,
                });
            }
        }
    }
    out
}

/// Days until the soonest `answered` window this cast satisfies closes.
fn answered_closes_in(v: &View<'_>, def: &DilemmaDef, cast: &[CastEntry]) -> Option<u32> {
    def.triggers
        .iter()
        .filter_map(|c| match c {
            Condition::Answered {
                max_days: Some(m), ..
            } => cast::answered_matches(v, c, cast)
                .into_iter()
                .map(|a| v.date().days_until(a.date.add_days(*m)))
                .min(),
            _ => None,
        })
        .min()
}

/// The template is a consequence of the player's own answers.
pub(super) fn is_consequence(def: &DilemmaDef) -> bool {
    def.triggers
        .iter()
        .any(|c| matches!(c, Condition::Answered { .. }))
}

/// Cast a dilemma and check its triggers (plus any extra conditions); its
/// tier in the current chapter.
pub(crate) fn evaluate<'c>(
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
    let mut preset = preset.to_vec();
    for e in answered_preset(v, def) {
        if !preset.iter().any(|p| p.role == e.role) {
            preset.push(e);
        }
    }
    let cast = cast::cast_search(v, &def.roles, &preset, goal_holder, 6, &ok)?;
    let mut reasons = cast::reasons(v, &def.triggers, &cast);
    if def.opportunity {
        reasons.push(Reason::Opportunity);
    }
    let relevance = relevance(v, n, &def.id, &cast, &reasons);
    let d = &v.content().director;
    let theme = n.chapter.as_ref().and_then(|c| c.theme.as_deref());
    let tier = if answered_closes_in(v, def, &cast).is_some_and(|x| x <= d.tier_a_window_days) {
        Tier::A
    } else if theme.is_some_and(|t| def.themes.iter().any(|x| x == t)) {
        Tier::B
    } else {
        Tier::C
    };
    Some(Candidate {
        def,
        cast,
        reasons,
        relevance,
        goal: None,
        tier,
    })
}

/// The pick (ENGAGEMENT §4.2): from the highest tier with a candidate
/// (opportunities first when easing off after a setback), those within
/// `pick_band` of the tier's best relevance, drawn by relevance × novelty
/// (`1 / (1 + uses)`) with one keyed draw (date, chapter, `salt`).
pub(super) fn pick<'c>(
    v: &View<'_>,
    n: &NarrativeState,
    mut cands: Vec<Candidate<'c>>,
    opportunities_first: bool,
    salt: u64,
) -> Option<Candidate<'c>> {
    if cands.is_empty() {
        return None;
    }
    let d = &v.content().director;
    cands.sort_by(|a, b| a.def.id.cmp(&b.def.id));
    let items: Vec<Pickable> = cands
        .iter()
        .map(|c| Pickable {
            tier: c.tier,
            relevance: c.relevance,
            opportunity: c.def.opportunity,
            novelty: novelty(n, &c.def.id),
        })
        .collect();
    let chapter = n.chapter.as_ref().map(|c| c.index).unwrap_or(0);
    let u = mandate_core::rng::keyed(
        v.r.meta.seed,
        Stream::Director,
        &[
            super::super::KEY_PICK,
            v.date().0 as u64,
            chapter as u64,
            salt,
        ],
    )
    .next_f64();
    let i = pick_index(&items, d.pick_band, opportunities_first, u)?;
    Some(cands.swap_remove(i))
}

/// What the pick weighs of a candidate.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Pickable {
    pub tier: Tier,
    pub relevance: f64,
    pub opportunity: bool,
    pub novelty: f64,
}

/// The pick's choice for a draw `u` in [0, 1): the highest tier present;
/// within it opportunities only when `opportunities_first` and there are
/// any; those within `band` of the best relevance; weighted by relevance ×
/// novelty, in the given order.
pub(crate) fn pick_index(
    items: &[Pickable],
    band: f64,
    opportunities_first: bool,
    u: f64,
) -> Option<usize> {
    let tier = items.iter().map(|c| c.tier).min()?;
    let opp = opportunities_first && items.iter().any(|c| c.tier == tier && c.opportunity);
    let pool: Vec<usize> = (0..items.len())
        .filter(|&i| items[i].tier == tier && (!opp || items[i].opportunity))
        .collect();
    let best = pool
        .iter()
        .map(|&i| items[i].relevance)
        .fold(f64::NEG_INFINITY, f64::max);
    let pool: Vec<usize> = pool
        .into_iter()
        .filter(|&i| items[i].relevance >= best - band)
        .collect();
    let weight = |i: usize| items[i].relevance.max(1e-6) * items[i].novelty;
    let total: f64 = pool.iter().map(|&i| weight(i)).sum();
    let mut at = u.clamp(0.0, 1.0) * total;
    for &i in &pool {
        at -= weight(i);
        if at < 0.0 {
            return Some(i);
        }
    }
    pool.last().copied()
}

/// `1 / (1 + uses)`: a template seen before weighs less.
pub(super) fn novelty(n: &NarrativeState, key: &str) -> f64 {
    1.0 / (1.0 + n.uses.get(key).copied().unwrap_or(0) as f64)
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
        ScenePlace::Workplace { role, fallback } => super::super::person_of(cast, role)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn item(tier: Tier, relevance: f64, opportunity: bool, uses: u16) -> Pickable {
        Pickable {
            tier,
            relevance,
            opportunity,
            novelty: 1.0 / (1.0 + uses as f64),
        }
    }

    /// What the pick chooses over a fine grid of draws (index → count).
    fn spread(items: &[Pickable], opp: bool) -> Vec<u32> {
        let mut out = vec![0u32; items.len()];
        for k in 0..1000 {
            let u = (k as f64 + 0.5) / 1000.0;
            out[pick_index(items, 0.8, opp, u).unwrap()] += 1;
        }
        out
    }

    #[test]
    fn tier_a_is_drawn_before_b_and_c() {
        // A closing consequence of low relevance beats a themed template and
        // a generic one of high relevance (story-carrying content first).
        let items = [
            item(Tier::C, 3.0, false, 0),
            item(Tier::B, 2.5, false, 0),
            item(Tier::A, 0.7, false, 0),
        ];
        assert_eq!(spread(&items, false), vec![0, 0, 1000]);
        // Without a tier A candidate, the chapter's theme comes first.
        assert_eq!(spread(&items[..2], false), vec![0, 1000]);
        assert_eq!(pick_index(&[], 0.8, false, 0.3), None);
    }

    #[test]
    fn the_pick_draws_within_the_band_by_relevance_and_novelty() {
        let items = [
            item(Tier::C, 2.0, false, 0),
            // In the band but seen three times: a quarter of the weight.
            item(Tier::C, 1.9, false, 3),
            // Outside the band (2.0 − 0.8): never drawn.
            item(Tier::C, 1.1, false, 0),
        ];
        let s = spread(&items, false);
        assert_eq!(s[2], 0);
        let expect = 2.0 / (2.0 + 1.9 / 4.0);
        let share = s[0] as f64 / 1000.0;
        assert!((share - expect).abs() < 0.01, "{share} vs {expect}");
    }

    #[test]
    fn easing_off_draws_opportunities_first() {
        let items = [item(Tier::C, 2.5, false, 0), item(Tier::C, 0.8, true, 0)];
        assert_eq!(spread(&items, true), vec![0, 1000]);
        // Not easing off: the opportunity is outside the band.
        assert_eq!(spread(&items, false), vec![1000, 0]);
    }
}
