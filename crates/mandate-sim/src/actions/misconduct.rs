//! Shady actions (DEC-034, DEC-073): bribes, dipping into funds, leaks.

use super::*;
use crate::spine::{Cause, PlayerAct};

/// Who offers a bribe in the player's town: the power broker, else a
/// business leader or plant manager (a Named person who wants something).
pub fn briber(state: &State) -> Option<characters::CharacterId> {
    let pl = state.player.as_ref()?;
    let m = pl.municipality.0;
    let me = characters::player_character(state);
    let town = state.politics.characters.towns.get(m as usize);
    town.and_then(|t| t.broker)
        .filter(|b| {
            Some(*b) != me
                && state
                    .politics
                    .characters
                    .get(*b)
                    .is_some_and(|c| c.politician.is_none())
        })
        .or_else(|| {
            state
                .politics
                .characters
                .people
                .iter()
                .filter(|c| {
                    c.named.as_deref().is_some_and(|n| {
                        matches!(
                            n.occupation.as_str(),
                            "occupation.business_leader" | "occupation.plant_manager"
                        ) && n
                            .place
                            .and_then(|p| state.world.places.get(p.index()))
                            .is_some_and(|p| p.municipality.0 == m)
                    })
                })
                .map(|c| c.id)
                .next()
        })
}

/// The rival a leak targets: the town's named rival (a politician).
pub fn leak_target(state: &State) -> Option<PoliticianId> {
    let pl = state.player.as_ref()?;
    let rival = state
        .politics
        .characters
        .towns
        .get(pl.municipality.index())?
        .rival?;
    state.politics.characters.get(rival)?.politician
}

/// Shady actions (DEC-034, DEC-073): a real payoff now, a hidden fact that
/// may come out later (the risk is shown before acting).
pub(super) fn shady(
    state: &mut State,
    content: &ContentBundle,
    action: &Action,
    late_night: bool,
) -> Result<ActionResult, EngineError> {
    let key = action.key();
    let def = content
        .media
        .shady_actions
        .get(key)
        .ok_or_else(|| bad("that is not possible here"))?
        .clone();
    let me = state
        .player
        .as_ref()
        .map(|p| p.politician)
        .ok_or_else(|| bad("no player"))?;
    let mut res = ActionResult::default();
    let mut actors = Vec::new();
    let rival = match action {
        Action::TakeBribe => {
            let b = briber(state).ok_or_else(|| bad("nobody here is offering"))?;
            actors.push(crate::narrative::Actor::Person { id: b });
            if let Some(p) = characters::profile(&state.politics, &state.world, b) {
                res.args.insert("name".into(), p.full_name());
            }
            None
        }
        Action::DipFunds => {
            if state
                .player
                .as_ref()
                .is_some_and(|p| (p.money.units() as f64) < -def.money)
            {
                return Err(bad("not enough campaign funds"));
            }
            None
        }
        _ => {
            let r = leak_target(state).ok_or_else(|| bad("you have no rival to leak about"))?;
            if let Some(c) = state.politics.get(r).map(|p| p.character) {
                actors.push(crate::narrative::Actor::Person { id: c });
            }
            res.args.insert(
                "name".into(),
                state
                    .politics
                    .get(r)
                    .map(|p| p.full_name())
                    .unwrap_or_default(),
            );
            Some(r)
        }
    };
    spend(state, content, key, late_night)?;
    if let Some(p) = state.player.as_mut() {
        let money = (p.money.units() as f64 + def.money).max(0.0);
        p.money = Money::from_units(money.round() as i64);
        p.tracks.health = clamp(p.tracks.health as f64 + def.health, 0.0, 100.0) as f32;
        p.tracks.legal_jeopardy =
            clamp(p.tracks.legal_jeopardy as f64 + def.legal, 0.0, 100.0) as f32;
    }
    let cause = Cause::Player(PlayerAct::of_action(action));
    let hidden = crate::media::hide(state, content, &def.hidden, me, actors, cause);
    res.outcome_key = format!("outcome-{}", key.replace('_', "-"));
    if def.money > 0.0 {
        res.args
            .insert("amount".into(), (def.money as i64).to_string());
    }
    if let Some(r) = rival {
        match crate::media::leak(state, r, hidden) {
            Some(crate::media::Leak::Exposed { .. }) => {
                res.outcome_key = "outcome-leak-file-exposed".into();
            }
            Some(crate::media::Leak::Rumour { .. }) => {
                res.outcome_key = "outcome-leak-file-rumour".into();
            }
            None => {}
        }
    }
    Ok(res)
}
