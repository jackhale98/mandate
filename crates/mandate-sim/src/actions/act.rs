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
        // Fundraising, the press, the bribe and the leak are conversations
        // (DEC-114, B5): had with someone where they are done (`verbs`;
        // routing an `Act` for one makes it that `Talk`). The variants stay
        // so old run logs still read.
        Action::Press { .. } | Action::Fundraise | Action::TakeBribe | Action::LeakFile => {
            return Err(bad(
                "that is done by talking to someone where it is done: talk to them",
            ));
        }
        Action::Canvass {
            neighborhood,
            issue,
        } => {
            res = canvass(state, content, pl, neighborhood, issue)?;
        }
        Action::Advertise => {
            res = advertise(state, content, pl)?;
        }
        Action::Rest => {
            spend(state, content, "rest", false)?;
            res.outcome_key = "outcome-rest".into();
        }
        Action::DipFunds => {
            res = shady(state, content, action, *late_night)?;
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
