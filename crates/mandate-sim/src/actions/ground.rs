//! The ground game: town halls and canvassing, and the campaign contact they make.

use super::*;

/// How well a canvass's worry fits the neighbourhood (DEC-091 "ignorance
/// costs effectiveness, never fairness"; M6_ENGINE_CONTRACTS §1.6): the
/// multiplier on its approval gain, `1 + match_bonus × (salience − 1)`
/// within [`floor`, `ceiling`] (`knowledge.effects.canvass`). Salience is the
/// neighbourhood's true felt level of that concern over the mean of its
/// felt concerns (1: an average worry, 0: one nobody feels here). No issue
/// (a general canvass) is 1. The truth decides; what the player knows only
/// decides which worry they can aim at. The command and any preview share
/// this function.
pub fn canvass_match(
    state: &State,
    content: &ContentBundle,
    hood: NeighborhoodId,
    issue: Option<&str>,
) -> f64 {
    let Some(issue) = issue else {
        return 1.0;
    };
    let Some(h) = state.world.neighborhoods.get(hood.index()) else {
        return 1.0;
    };
    let felt = crate::concerns::felt_concerns(
        &state.population,
        content,
        h.municipality.index(),
        &[hood.index()],
    );
    let mean = felt.iter().map(|x| x.1).sum::<f64>() / felt.len().max(1) as f64;
    let mine = felt
        .iter()
        .find(|(i, _)| content.concerns.get(*i).is_some_and(|c| c.id == issue))
        .map(|x| x.1)
        .unwrap_or(0.0);
    let salience = if mean > 0.0 { mine / mean } else { 1.0 };
    let k = &content.knowledge.effects.canvass;
    clamp(1.0 + k.match_bonus * (salience - 1.0), k.floor, k.ceiling)
}

/// A canvass fit in words (`strong` | `fair` | `weak`), for the outcome.
fn fit_band(content: &ContentBundle, fit: f64) -> &'static str {
    let k = &content.knowledge.effects.canvass;
    if fit > 1.0 && fit >= 1.0 + 0.5 * (k.ceiling - 1.0) {
        "strong"
    } else if fit < 1.0 && fit <= 1.0 - 0.5 * (1.0 - k.floor) {
        "weak"
    } else {
        "fair"
    }
}

/// Chance a town hall turns hostile in a neighbourhood: likelier where the
/// player and the town hall are unpopular (shown before acting).
pub fn town_hall_risk(state: &State, content: &ContentBundle, hood: u32) -> f64 {
    let Some(h) = state.population.hoods.get(hood as usize) else {
        return 0.0;
    };
    let (mut ap, mut gov, mut n) = (0.0, 0.0, 0.0);
    for c in &h.cohorts {
        let w = c.count as f64;
        ap += c.player_approval as f64 * c.player_awareness as f64 * w;
        gov += c.gov_approval as f64 * w;
        n += w;
    }
    let ap = ap / n.max(1.0);
    let gov = gov / n.max(1.0);
    let t = &content.actions.town_hall;
    clamp(
        t.risk_base - t.risk_approval * ap - t.risk_government * (gov - 0.5),
        t.risk_min,
        t.risk_max,
    )
}

/// Apply campaign contact to the player's candidacy, if a race is running and
/// covers the neighbourhood (`None` = every neighbourhood in the race).
pub(super) fn campaign_contact(
    state: &mut State,
    content: &ContentBundle,
    hood: Option<u32>,
    scale: f64,
) -> bool {
    let Some(me) = state.player.as_ref().map(|p| p.politician) else {
        return false;
    };
    let params = &content.politics.campaign;
    let mut touched = false;
    for race in state.politics.elections.races.iter_mut() {
        let Some(c) = race.candidates.iter_mut().find(|c| c.politician == me) else {
            continue;
        };
        for (i, h) in race.hoods.iter().enumerate() {
            if hood.is_none_or(|x| x == *h) {
                crate::politics::elections::contact(c, i, params, scale);
                touched = true;
            }
        }
    }
    touched
}

/// Canvassing a neighbourhood (DEC-091, §1.3): its worries `known` (a
/// second canvass reads them: `detailed`, dated), and one resident found on
/// its streets met. Nobody new is created: only people already stored.
fn learn_canvass(state: &mut State, content: &ContentBundle, hood: NeighborhoodId) {
    let src = knowledge::action_source(state, "canvass");
    knowledge::learn_hood(state, content, hood, src.clone());
    let me = characters::player_character(state);
    let Some(pl) = state.player.as_ref() else {
        return;
    };
    let resident = state.politics.characters.people.iter().find(|c| {
        Some(c.id) != me
            && c.named.as_deref().is_some_and(|n| n.home == hood.0)
            && pl
                .knowledge
                .level(&knowledge::Subject::Person { person: c.id })
                < Level::Known
    });
    if let Some(person) = resident.map(|c| c.id) {
        knowledge::teach_one(
            state,
            knowledge::Subject::Person { person },
            Level::Known,
            src,
        );
    }
}

/// Hold a town hall in a neighbourhood, optionally about a bill.
pub(super) fn town_hall(
    state: &mut State,
    content: &ContentBundle,
    pl: &crate::player::PlayerState,
    m: u32,
    neighborhood: &u32,
    policy: &Option<u32>,
) -> Result<ActionResult, EngineError> {
    let mut res = ActionResult::default();
    let h = state
        .world
        .neighborhoods
        .get(*neighborhood as usize)
        .ok_or_else(|| bad("no such neighbourhood"))?;
    if h.municipality.0 != m {
        return Err(bad("that neighbourhood is not in your town"));
    }
    if let Some(id) = policy {
        local_policy(state, *id)?;
    }
    let risk = town_hall_risk(state, content, *neighborhood);
    spend(state, content, "town_hall", false)?;
    crate::politics::add_record(
        &mut state.politics,
        pl.politician,
        content.career.record_gain.town_hall as f32,
    );
    let fx = &content.actions.town_hall;
    let hostile = state.rng.get(Stream::Player).chance(risk);
    let oratory = state
        .politics
        .get(pl.politician)
        .map(|x| x.oratory)
        .unwrap_or(0.5) as f64;
    let sig = policy
        .and_then(|id| state.policy.instances.get(id as usize))
        .and_then(|p| crate::policy::resolve(content, &state.policy, p))
        .map(|t| t.signature_weights.clone());
    for c in state.population.hoods[*neighborhood as usize]
        .cohorts
        .iter_mut()
    {
        c.player_awareness = clamp(
            c.player_awareness as f64 + fx.awareness * (fx.awareness_oratory_base + oratory),
            0.0,
            1.0,
        ) as f32;
        let nudge = sig
            .as_ref()
            .map(|s| crate::politics::alignment(s, &c.attitudes) * fx.bill_nudge)
            .unwrap_or(fx.nudge);
        let room = 1.0 - (c.player_approval as f64).abs();
        // A hostile crowd: people still learn your name, but not kindly.
        let nudge = if hostile { fx.hostile_nudge } else { nudge };
        c.player_approval = clamp(c.player_approval as f64 + nudge * room, -1.0, 1.0) as f32;
    }
    if let Some(id) = policy
        && let Some(p) = state.policy.instances.get_mut(*id as usize)
    {
        p.awareness = clamp(
            p.awareness as f64 + fx.bill_awareness * (0.5 + oratory),
            0.0,
            1.0,
        ) as f32;
    }
    let scale = (fx.contact_base + fx.contact_oratory * oratory)
        * if hostile { fx.hostile_contact } else { 1.0 };
    campaign_contact(state, content, Some(*neighborhood), scale);
    // A meeting hears the neighbourhood out (DEC-091); with
    // a bill, who supports it.
    let src = knowledge::action_source(state, "town_hall");
    knowledge::learn_hood(state, content, NeighborhoodId(*neighborhood), src.clone());
    if let Some(t) = policy
        .and_then(|id| state.policy.instances.get(id as usize))
        .map(|p| p.template.clone())
    {
        knowledge::teach_one(
            state,
            knowledge::Subject::PolicySupport {
                town: MunicipalityId(m),
                policy: knowledge::PolicyRef::Template { template: t },
            },
            Level::Known,
            src,
        );
    }
    crate::asks::record_deed(
        state,
        crate::asks::Deed::TownHall {
            hood: NeighborhoodId(*neighborhood),
        },
    );
    res.outcome_key = if hostile {
        "outcome-town-hall-hostile"
    } else {
        "outcome-town-hall"
    }
    .into();
    res.args.insert(
        "place".into(),
        state.world.neighborhoods[*neighborhood as usize]
            .name
            .clone(),
    );
    Ok(res)
}

/// Canvass a neighbourhood, optionally on one worry.
pub(super) fn canvass(
    state: &mut State,
    content: &ContentBundle,
    pl: &crate::player::PlayerState,
    neighborhood: &u32,
    issue: &Option<String>,
) -> Result<ActionResult, EngineError> {
    let mut res = ActionResult::default();
    let h = state
        .world
        .neighborhoods
        .get(*neighborhood as usize)
        .ok_or_else(|| bad("no such neighbourhood"))?;
    if Some(h.municipality) != crate::player::home(state) {
        return Err(bad("that neighbourhood is not in your town"));
    }
    if let Some(i) = issue
        && !content.concerns.iter().any(|c| &c.id == i)
    {
        return Err(bad("no such worry"));
    }
    let hood = NeighborhoodId(*neighborhood);
    let fx = &content.actions.canvass;
    // Decided before the doorsteps change anything: how the
    // worry fits, and how fresh the doorsteps are (DEC-097 §1).
    let fit = canvass_match(state, content, hood, issue.as_deref());
    let curve = crate::player::ground::canvass_curve(state, fx, hood);
    spend(state, content, "canvass", false)?;
    crate::politics::add_record(
        &mut state.politics,
        pl.politician,
        content.career.record_gain.canvass as f32,
    );
    for c in state.population.hoods[hood.index()].cohorts.iter_mut() {
        // Contact builds recognition first; persuasion is
        // small, saturating, fades with repeated visits, and
        // counts for more in campaign season (DEC-071 §5,
        // DEC-097 §1).
        let aw = c.player_awareness as f64;
        c.player_awareness =
            clamp(aw + fx.awareness * (1.0 - aw) * curve.awareness, 0.0, 1.0) as f32;
        let room = 1.0 - (c.player_approval as f64).abs();
        c.player_approval = clamp(
            c.player_approval as f64 + fx.approval * fit * room * curve.approval,
            -1.0,
            1.0,
        ) as f32;
    }
    // A full day on the doorsteps: deeper than any other
    // contact (the race model has its own diminishing).
    campaign_contact(
        state,
        content,
        Some(*neighborhood),
        fx.contact * curve.same_turn,
    );
    let (on, turn) = (state.calendar.date, state.calendar.turn);
    if let Some(p) = state.player.as_mut() {
        p.ground.remember(hood, curve.visits + 1.0, on, turn);
    }
    learn_canvass(state, content, hood);
    crate::asks::record_deed(state, crate::asks::Deed::Canvass { hood });
    res.outcome_key = "outcome-canvass".into();
    // How fresh the doorsteps were (the player's own memory).
    res.args.insert(
        "fresh".into(),
        crate::player::ground::fresh_band(curve.fresh).into(),
    );
    if let Some(i) = issue {
        // How the worry landed (the player has just heard
        // the doorsteps, so this tells nothing hidden).
        res.args.insert("issue".into(), i.clone());
        res.args.insert("fit".into(), fit_band(content, fit).into());
    }
    res.args.insert(
        "place".into(),
        state.world.neighborhoods[*neighborhood as usize]
            .name
            .clone(),
    );
    Ok(res)
}
