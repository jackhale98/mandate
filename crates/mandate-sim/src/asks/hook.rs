//! Asks in conversation (S4 talk hooks; M6 note 12): an accepted ask whose
//! current need is "talk to X" puts its own choice in the conversation
//! with X ("Put in a word for { $asker }"), with the lines its template
//! names (`NeedDef::Talk { choice }`). A reply that agrees meets the need
//! at once; one that does not leaves it open and the ask says so (its
//! `tries` count the refusals at the current need).
//!
//! This is one of the asks' own commands: it writes only `AskState`.

use super::{AskId, Need};
use crate::characters::{self, CharacterId};
use crate::narrative::{Actor, CastEntry};
use crate::state::State;
use crate::talk::hooks::{self, Choice, ChoiceCost, Done, Hook, Reply};
use mandate_content::ContentBundle;
use mandate_content::asks::{NeedDef, TalkChoiceDef};

/// The asks' hook (registered in `talk::hooks::HOOKS`).
pub const HOOK: Hook = Hook {
    owner: "ask",
    offer,
    chosen,
};

/// The id of an ask's choice: `choice.ask.<id>`.
pub fn choice_id(ask: AskId) -> String {
    hooks::choice_id(HOOK.owner, &ask.0.to_string())
}

/// The ask a choice id names.
fn ask_of(id: &str) -> Option<AskId> {
    let rest = id.strip_prefix(hooks::PREFIX)?.strip_prefix("ask.")?;
    rest.parse().ok().map(AskId)
}

/// The template's choice for an ask's current need (its needs are resolved
/// one for one from the template's, in order).
pub fn choice_def<'c>(content: &'c ContentBundle, a: &super::Ask) -> Option<&'c TalkChoiceDef> {
    let t = content.ask(&a.template)?;
    match t.needs.get(a.step as usize)? {
        NeedDef::Talk { choice, .. } => choice.as_ref(),
        _ => None,
    }
}

/// Fluent arguments for an ask's choice: its people by role (their names),
/// a bill role as `<role>_key`, a neighbourhood by its name.
pub fn cast_args(
    state: &State,
    content: &ContentBundle,
    cast: &[CastEntry],
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for e in cast {
        match e.actor {
            Actor::Person { id } => {
                if let Some(p) = characters::profile(&state.politics, &state.world, id) {
                    out.push((e.role.clone(), p.full_name()));
                }
            }
            Actor::Policy { id } => {
                if let Some(t) = state
                    .policy
                    .instances
                    .get(id as usize)
                    .and_then(|p| crate::policy::resolve(content, &state.policy, p))
                {
                    out.push((format!("{}_key", e.role), t.name_key.clone()));
                }
            }
            Actor::Neighborhood { id } => {
                if let Some(h) = state.world.neighborhoods.get(id as usize) {
                    out.push((e.role.clone(), h.name.clone()));
                }
            }
        }
    }
    out
}

/// `speaker`, `player` and `town` (the player's home), for the label and
/// the player's line (the reply gets them from the conversation too).
fn common_args(state: &State, speaker: CharacterId) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let name = |c: CharacterId| {
        characters::profile(&state.politics, &state.world, c).map(|p| p.full_name())
    };
    if let Some(n) = name(speaker) {
        out.push(("speaker".into(), n));
    }
    if let Some(n) = characters::player_character(state).and_then(name) {
        out.push(("player".into(), n));
    }
    if let Some(m) =
        crate::player::home(state).and_then(|m| state.world.municipalities.get(m.index()))
    {
        out.push(("town".into(), m.name.clone()));
    }
    out
}

/// The choices for a conversation with `speaker`: one for each accepted
/// ask whose current need is a talk with them.
fn offer(state: &State, content: &ContentBundle, speaker: CharacterId) -> Vec<Choice> {
    if state.player.is_none() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for a in state.asks.asks.iter().filter(|a| a.accepted()) {
        let Some(Need::Talk { person }) = a.current() else {
            continue;
        };
        if *person != speaker {
            continue;
        }
        let Some(def) = choice_def(content, a) else {
            continue;
        };
        let mut args = common_args(state, speaker);
        args.extend(cast_args(state, content, &a.cast));
        out.push(Choice {
            id: choice_id(a.id),
            label_key: def.label_key.clone(),
            say_key: def.say_key.clone(),
            args,
            replies: def
                .replies
                .iter()
                .map(|r| Reply {
                    key: r.key.clone(),
                    when: r.when.clone(),
                    agrees: r.agrees,
                })
                .collect(),
            cost: ChoiceCost::Conversation,
            odds: None,
            costs: Vec::new(),
            remote: false,
            bill: None,
        });
    }
    out
}

/// They agreed: the need is met (the ask moves on at once, and is kept if
/// it was the last). They did not: it stays open, one refusal counted.
fn chosen(
    state: &mut State,
    content: &ContentBundle,
    choice: &Choice,
    _speaker: CharacterId,
    reply: &Reply,
) -> Done {
    let asker = state
        .asks
        .get(ask_of(&choice.id).unwrap_or(AskId(u32::MAX)))
        .and_then(|a| a.asker_person())
        .and_then(|p| characters::profile(&state.politics, &state.world, p))
        .map(|p| p.full_name())
        .unwrap_or_default();
    let _ = content;
    let Some(a) = ask_of(&choice.id).and_then(|id| state.asks.get_mut(id)) else {
        return Done {
            outcome_key: "outcome-talk".into(),
            args: Vec::new(),
        };
    };
    let key = if reply.agrees {
        a.step = a.step.saturating_add(1);
        a.done.clear();
        a.tries = 0;
        if a.step as usize >= a.needs.len() {
            "outcome-ask-choice-kept"
        } else {
            "outcome-ask-choice-agreed"
        }
    } else {
        a.tries = a.tries.saturating_add(1);
        "outcome-ask-choice-refused"
    };
    Done {
        outcome_key: key.into(),
        args: vec![("asker".into(), asker)],
    }
}

/// An ask's current talk need was refused at least once (the ask view says so).
pub fn refused(a: &super::Ask) -> bool {
    a.tries > 0 && matches!(a.current(), Some(Need::Talk { .. }))
}
