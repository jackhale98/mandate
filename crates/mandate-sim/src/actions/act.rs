//! Actions (`PlayerCommand::Act`): the late-night rules, the dispatch to each family, and the log.

use super::*;

/// Do an action (`PlayerCommand::Act`): the late-night rules, the action, and its log line.
pub(super) fn perform(
    state: &mut State,
    content: &ContentBundle,
    pl: &crate::player::PlayerState,
    date: mandate_core::SimDate,
    m: u32,
    action: &Action,
    late_night: &bool,
) -> Result<ActionResult, EngineError> {
    let mut res = ActionResult::default();
    if *late_night && matches!(action, Action::Advertise) {
        return Err(bad("ads are bought, not held late at night"));
    }
    if *late_night
        && !matches!(
            action,
            Action::Lobby { .. }
                | Action::TradeFavor { .. }
                | Action::Fundraise
                | Action::TakeBribe
                // A verb says itself whether it is done late.
                | Action::Visit { .. }
        )
    {
        return Err(bad(
            "only private meetings and fundraising happen late at night",
        ));
    }
    if !*late_night && matches!(action, Action::TakeBribe) {
        return Err(bad("envelopes change hands late at night"));
    }
    match action {
        // Lobbying is a conversation (M6 note 29): the one-tap path is
        // retired; the variants stay so old run logs still read.
        Action::Lobby { .. } | Action::TradeFavor { .. } => {
            return Err(bad(
                "lobbying is done by talking to the member: talk to them about the bill",
            ));
        }
        Action::TownHall {
            neighborhood,
            policy,
        } => {
            res = town_hall(state, content, pl, m, neighborhood, policy)?;
        }
        Action::Press { policy } => {
            res = press(state, content, pl, m, policy)?;
        }
        Action::Canvass {
            neighborhood,
            issue,
        } => {
            res = canvass(state, content, pl, neighborhood, issue)?;
        }
        Action::Fundraise => {
            res = fundraise(state, content, pl, m, late_night)?;
        }
        Action::Advertise => {
            res = advertise(state, content, pl)?;
        }
        Action::Rest => {
            spend(state, content, "rest", false)?;
            res.outcome_key = "outcome-rest".into();
        }
        Action::TakeBribe | Action::DipFunds | Action::LeakFile => {
            let target = matches!(action, Action::LeakFile)
                .then(|| leak_target(state))
                .flatten();
            res = shady(state, content, action, *late_night)?;
            if let Some(t) = target {
                crate::asks::record_deed(state, crate::asks::Deed::Leak { politician: t.0 });
            }
        }
        Action::Visit { verb } => {
            res = crate::visits::visit(state, content, verb, *late_night)?;
        }
        Action::Casework { ask } => {
            // Checked before the slot is spent; the roll is the asks'.
            let ok = state.asks.get(crate::asks::AskId(*ask)).is_some_and(|a| {
                a.accepted() && matches!(a.current(), Some(crate::asks::Need::Casework))
            });
            if !ok {
                return Err(bad("there is no casework to sort out for that ask"));
            }
            spend(state, content, "casework", false)?;
            res = crate::asks::commands::casework(state, content, *ask)?;
        }
    }
    let word = match action {
        Action::Visit { verb } => crate::visits::action_word(verb),
        _ => action.key().to_string(),
    };
    let action_key = format!("action-{}", word.replace('_', "-"));
    if let Some(p) = state.player.as_mut() {
        p.log(
            date,
            &action_key,
            &res.outcome_key,
            res.args
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        );
    }
    Ok(res)
}
