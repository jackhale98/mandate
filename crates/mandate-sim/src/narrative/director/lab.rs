//! Lab hooks (ENGAGEMENT 7.4): the week's candidates for metrics.

use super::*;

// --- The lab (ENGAGEMENT §7.4) -------------------------------------------------------------

/// One candidate the director weighs this week: the lab's "eligible but
/// never picked" table.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CandidateInfo {
    pub template: String,
    pub tier: Tier,
    pub relevance: f64,
    /// `1 / (1 + uses)`.
    pub novelty: f64,
    /// Within `pick_band` of its tier's best (the pick may draw it).
    pub in_band: bool,
    pub opportunity: bool,
}

/// The candidates the spotlight would weigh now, from a whole state: the
/// dilemmas whose triggers hold and that are available (cooldowns, no
/// repeat, `min_career_days`). An arc's decision beats are not candidates:
/// the run asks for them. Best tier first, then by relevance. Pure: it
/// writes nothing and draws nothing (ENGAGEMENT §7.4).
pub fn week_candidates(state: &crate::State, content: &ContentBundle) -> Vec<CandidateInfo> {
    let r = Reads::of_state(state, content);
    let n = &state.narrative;
    if !active(&r) || n.chapter.is_none() {
        return Vec::new();
    }
    let mut v = View::new(&r, &state.politics);
    v.recent = recent(n);
    let d = &content.director;
    let cands = gather(&v, n);
    let best = |t: Tier| {
        cands
            .iter()
            .filter(|c| c.tier == t)
            .map(|c| c.relevance)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let mut out: Vec<CandidateInfo> = cands
        .iter()
        .map(|c| CandidateInfo {
            template: c.def.id.clone(),
            tier: c.tier,
            relevance: c.relevance,
            novelty: novelty(n, &c.def.id),
            in_band: c.relevance >= best(c.tier) - d.pick_band,
            opportunity: c.def.opportunity,
        })
        .collect();
    out.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then(b.relevance.total_cmp(&a.relevance))
            .then(a.template.cmp(&b.template))
    });
    out
}
