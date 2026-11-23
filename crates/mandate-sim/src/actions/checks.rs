//! Shared checks and costs: parameters, clauses, slots and health, own and local bills.

use super::*;

pub(super) fn validate_params(
    content: &ContentBundle,
    template: &str,
    params: &BTreeMap<String, f64>,
) -> Result<BTreeMap<String, f64>, EngineError> {
    let t = content
        .policies
        .get(template)
        .ok_or_else(|| bad("unknown policy template"))?;
    let mut out = BTreeMap::new();
    for p in &t.params {
        let v = params.get(&p.id).copied().unwrap_or(p.default);
        if !v.is_finite() || v < p.min || v > p.max {
            return Err(bad(&format!("parameter {} out of range", p.id)));
        }
        let snapped = p.min + ((v - p.min) / p.step).round() * p.step;
        out.insert(p.id.clone(), clamp(snapped, p.min, p.max));
    }
    Ok(out)
}

/// Pay an action's time and health (from content, DEC-073). Late-night work
/// uses the extra slot instead and costs health on top.
pub(super) fn spend(
    state: &mut State,
    content: &ContentBundle,
    action: &str,
    late_night: bool,
) -> Result<(), EngineError> {
    let cost = content.tuning.action_cost(action);
    let pl = state.player.as_mut().ok_or_else(|| bad("no player"))?;
    if late_night {
        if pl.late_night_used {
            return Err(bad("late night already used this turn"));
        }
        pl.late_night_used = true;
        pl.tracks.health = (pl.tracks.health - content.actions.late_night_health as f32).max(0.0);
    } else {
        if pl.slots_left() < cost.slots {
            return Err(bad("not enough time left this turn"));
        }
        pl.slots_used += cost.slots;
    }
    pl.tracks.health = clamp(pl.tracks.health as f64 - cost.health, 0.0, 100.0) as f32;
    Ok(())
}

pub(super) fn own_policy(state: &State, id: u32) -> Result<&PolicyInstance, EngineError> {
    let pl = state.player.as_ref().ok_or_else(|| bad("no player"))?;
    let p = state
        .policy
        .instances
        .get(id as usize)
        .ok_or_else(|| bad("no such policy"))?;
    if p.sponsor != Some(pl.politician) {
        return Err(bad("you are not the sponsor"));
    }
    Ok(p)
}

pub(super) fn local_policy(state: &State, id: u32) -> Result<&PolicyInstance, EngineError> {
    let pl = state.player.as_ref().ok_or_else(|| bad("no player"))?;
    let p = state
        .policy
        .instances
        .get(id as usize)
        .ok_or_else(|| bad("no such policy"))?;
    if p.jurisdiction != pl.municipality.0 {
        return Err(bad("that policy is not in your town"));
    }
    Ok(p)
}

pub(super) fn validate_clauses(
    state: &State,
    m: u32,
    clauses: &[Clause],
) -> Result<(), EngineError> {
    let earmarks = clauses
        .iter()
        .filter(|c| matches!(c, Clause::Earmark { .. }))
        .count();
    if earmarks > 3 {
        return Err(bad("at most three earmarks"));
    }
    for c in clauses {
        match c {
            Clause::Earmark { ward } => {
                let ok = state
                    .world
                    .constituencies
                    .get(ward.index())
                    .is_some_and(|w| w.tier == Tier::Local && w.jurisdiction == m);
                if !ok {
                    return Err(bad("earmark ward is not in this town"));
                }
            }
            Clause::Sunset { years } => {
                if !(1..=10).contains(years) {
                    return Err(bad("sunset must be 1–10 years"));
                }
            }
            Clause::Pilot { neighborhood } => {
                let ok = state
                    .world
                    .neighborhoods
                    .get(*neighborhood as usize)
                    .is_some_and(|n| n.municipality.0 == m);
                if !ok {
                    return Err(bad("pilot neighbourhood is not in this town"));
                }
            }
            // Where a family bill applies follows its choices (DEC-092).
            Clause::Area { .. } => {
                return Err(bad("where a proposal applies is one of its choices"));
            }
        }
    }
    Ok(())
}

/// The sliders of an amendment (a family bill's are checked when composed).
pub(super) fn params_in(cmd: &PlayerCommand) -> &BTreeMap<String, f64> {
    static EMPTY: BTreeMap<String, f64> = BTreeMap::new();
    match cmd {
        PlayerCommand::AmendPolicy { params, .. } => params,
        _ => &EMPTY,
    }
}

/// Estimated annual cost of a draft or bill, for display.
pub fn estimate_cost(state: &State, content: &ContentBundle, p: &PolicyInstance) -> f64 {
    let Some(t) = crate::policy::resolve(content, &state.policy, p) else {
        return 0.0;
    };
    let town = state
        .population
        .municipality_total(&state.world, MunicipalityId(p.jurisdiction)) as f64;
    let residents = crate::policy::cost_residents(&state.world, &state.population, p, town);
    annual_cost(&t, p, residents, state.economy.national.price_level)
}
