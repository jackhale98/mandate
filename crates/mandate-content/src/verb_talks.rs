//! Fundraising, the press, the bribe and the leak are conversations
//! (DEC-114, B5): each verb is a choice in a talk with a person, its effect
//! and its time as the one action had them (`actions`, `tuning`). This is
//! whom each is had with, and what they say back.

use crate::asks::ChoiceReplyDef;
use crate::condition::{ConditionUse, Site};
use crate::validate::Issue;
use crate::{ContentBundle, ContentKey};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The four verbs' conversations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerbTalks {
    pub press: VerbTalk,
    pub fundraise: VerbTalk,
    /// The bribe is offered by whoever is offering (the broker, else a
    /// business leader or plant manager: `actions::briber`); `with` is not
    /// read.
    pub take_bribe: VerbTalk,
    pub leak_file: VerbTalk,
}

/// One verb's conversation.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerbTalk {
    /// Occupations of the Named people of the player's town it is had with
    /// (they see the player where the verb is done).
    #[serde(default)]
    pub with: Vec<ContentKey>,
    /// What they say: the first whose conditions hold (slots `speaker`,
    /// `player`); the last has none. `agrees` is not read.
    pub replies: Vec<ChoiceReplyDef>,
}

impl VerbTalks {
    /// Each verb's conversation, with its action key.
    pub fn all(&self) -> [(&'static str, &VerbTalk); 4] {
        [
            ("press", &self.press),
            ("fundraise", &self.fundraise),
            ("take_bribe", &self.take_bribe),
            ("leak_file", &self.leak_file),
        ]
    }

    /// A verb's conversation by its action key.
    pub fn of(&self, key: &str) -> Option<&VerbTalk> {
        self.all().into_iter().find(|(k, _)| *k == key).map(|x| x.1)
    }
}

/// Occupations that exist, someone to talk to, replies whose keys exist in
/// English using only the conversation's arguments, conditions that check,
/// and a last reply that always holds.
pub(crate) fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let text = b.locale_text("en");
    let en = crate::validate::fluent_keys(&text);
    let provided: BTreeSet<&str> = ["speaker", "player", "town"].into_iter().collect();
    for (key, t) in b.actions.talks.all() {
        let owner = format!("actions.talks.{key}");
        for o in &t.with {
            if b.occupation(o).is_none() {
                issues.push(Issue::error(&owner, &format!("unknown occupation `{o}`")));
            }
        }
        if key != "take_bribe" && t.with.is_empty() {
            issues.push(Issue::error(
                &owner,
                "the verb needs someone to be had with (`with`)",
            ));
        }
        if !t.replies.last().is_some_and(|r| r.when.is_empty()) {
            issues.push(Issue::error(
                &owner,
                "the last reply has no conditions (so there is always a reply)",
            ));
        }
        let site = Site::new(&owner, ConditionUse::TalkLine, &["speaker", "player"]);
        for r in &t.replies {
            if !en.contains(&r.key) {
                issues.push(Issue::error(
                    &owner,
                    &format!("missing Fluent key `{}` in locale en", r.key),
                ));
                continue;
            }
            for v in crate::asks::message_vars(&text, &r.key) {
                if !provided.contains(v.as_str()) {
                    issues.push(Issue::error(
                        &owner,
                        &format!(
                            "`{}` uses `${v}`, which the conversation does not provide",
                            r.key
                        ),
                    ));
                }
            }
            crate::condition::check_all(b, &site, &r.when, issues);
        }
    }
}
