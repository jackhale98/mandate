//! Talk hooks (S4): any system can put a **choice** of its own in a
//! conversation with a given person, next to the ordinary topics.
//!
//! A system registers a [`Hook`] in [`HOOKS`]: `offer` lists its choices
//! for a speaker (read-only, called by the talk panel and the command
//! alike), and `chosen` applies what the choice does when the player picks
//! it, in the owner's own code (it writes only its own domain; anything
//! else goes through effect requests). The talk system does the rest: the
//! checks (they are here, the time), the cost, the reply (the first whose
//! conditions hold, decided by the one condition evaluator with the
//! conversation's slots), the exchange in the log, and the outcome line.
//!
//! A choice's id is its `Talk` command's `topic`: `choice.<owner>.<local>`.
//! Facts and Fluent keys only (DEC-054); nothing here draws a random number.

use super::context::{Args, Ctx};
use crate::characters::CharacterId;
use crate::state::State;
use mandate_content::{Condition, ContentBundle};

/// The prefix of a choice's id (the `Talk` command's `topic`).
pub const PREFIX: &str = "choice.";

/// What a choice costs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceCost {
    /// As a conversation (half a slot; the late slot late at night).
    Conversation,
    /// Nothing beyond being there.
    Free,
}

/// One possible reply: said when its conditions hold (slots `speaker`,
/// `player`); `agrees` tells the owner how it went.
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    pub key: String,
    pub when: Vec<Condition>,
    pub agrees: bool,
}

/// A choice a system offers in a conversation with one person.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// `choice.<owner>.<local>`: the `Talk` command's `topic`.
    pub id: String,
    /// The button.
    pub label_key: String,
    /// What the player says.
    pub say_key: String,
    /// Arguments of the label, the player's line and the reply (`*_key`
    /// arguments are Fluent keys).
    pub args: Vec<(String, String)>,
    /// In order: the first whose conditions hold is said. The last should
    /// hold always (content validation makes it so for asks).
    pub replies: Vec<Reply>,
    pub cost: ChoiceCost,
}

/// What the owner did when the choice was picked: the exchange's outcome.
#[derive(Clone, Debug, PartialEq)]
pub struct Done {
    pub outcome_key: String,
    pub args: Vec<(String, String)>,
}

/// A system's hook into conversations.
pub struct Hook {
    /// The `<owner>` in a choice's id.
    pub owner: &'static str,
    /// The choices for a conversation with `speaker` now (read-only).
    pub offer: fn(&State, &ContentBundle, CharacterId) -> Vec<Choice>,
    /// Apply the choice (the owner's domain only), given the reply said.
    pub chosen: fn(&mut State, &ContentBundle, &Choice, CharacterId, &Reply) -> Done,
}

/// Every registered hook, in listing order.
pub const HOOKS: &[Hook] = &[crate::asks::hook::HOOK, crate::arcs::beats::HOOK];

/// Whether a `Talk` topic is a hook's choice.
pub fn is_choice(topic: &str) -> bool {
    topic.starts_with(PREFIX)
}

/// The id of a choice: `choice.<owner>.<local>`.
pub fn choice_id(owner: &str, local: &str) -> String {
    format!("{PREFIX}{owner}.{local}")
}

/// Every choice offered for a conversation with `speaker` now, hook by hook.
pub fn offered(state: &State, content: &ContentBundle, speaker: CharacterId) -> Vec<Choice> {
    HOOKS
        .iter()
        .flat_map(|h| (h.offer)(state, content, speaker))
        .collect()
}

/// The hook and the choice a topic names, if it is offered now.
pub fn find(
    state: &State,
    content: &ContentBundle,
    speaker: CharacterId,
    id: &str,
) -> Option<(&'static Hook, Choice)> {
    let owner = id.strip_prefix(PREFIX)?.split('.').next()?;
    let hook = HOOKS.iter().find(|h| h.owner == owner)?;
    (hook.offer)(state, content, speaker)
        .into_iter()
        .find(|c| c.id == id)
        .map(|c| (hook, c))
}

/// The reply said: the first whose conditions hold, with the arguments the
/// conditions give.
pub fn reply<'c>(ctx: &Ctx<'_>, c: &'c Choice) -> Option<(&'c Reply, Args)> {
    c.replies.iter().find_map(|r| {
        let mut a = Args::new();
        ctx.all_hold(&r.when, &mut a).then_some((r, a))
    })
}
