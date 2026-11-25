//! The content workbench's queries (owner, 2026-11-19), compiled only with
//! the `dev-tools` feature: the release engine neither has nor answers
//! them. Read-only views over `mandate_sim::explain`: every verdict comes
//! from the engine's own evaluators; previews are built from a decision
//! that is never stored (views never write). The TypeScript types are
//! generated into `web/src/dev/protocol/` by
//! `cargo test -p mandate-protocol --features dev-tools`.

use crate::narrative::{DecisionView, cast_args, decision_view};
use crate::views::DateView;
use mandate_sim::Engine;
use mandate_sim::explain::{self, Explanation, Focus, ItemKind, Node};
use mandate_sim::narrative::{Decision, DecisionId, DecisionStatus, Weight};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../dev/protocol/")]
pub struct DevValueView {
    pub name: String,
    pub value: String,
}

/// One node of an explanation: a gate, a role, a group, or a condition,
/// with the real evaluator's verdict and what it read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../dev/protocol/")]
pub struct DevNodeView {
    /// `all` | `any` | `condition` | `gate` | `role` | `place` | `speaker` |
    /// `stage` | `info`.
    pub kind: String,
    pub label: String,
    pub detail: String,
    pub ok: bool,
    pub values: Vec<DevValueView>,
    pub children: Vec<DevNodeView>,
}

/// One content item in the catalogue, with whether it could fire now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../dev/protocol/")]
pub struct DevItemView {
    /// `dilemma` | `situational` | `overheard` | `topic` |
    /// `line` | `voice` | `scene`.
    pub kind: String,
    pub id: String,
    /// The Fluent key that names it (a title, or the line itself).
    pub title_key: Option<String>,
    /// Its kind of kind: a dilemma's category, a line's part and topic.
    pub group: String,
    pub could_fire: bool,
    /// The first thing in the way (empty when it could fire).
    pub blocked_by: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../dev/protocol/")]
pub struct DevPersonView {
    pub id: u32,
    pub name: String,
    /// `councillor`, `mayor` or an occupation id.
    pub role: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../dev/protocol/")]
pub struct DevPlaceView {
    pub id: u32,
    pub name: String,
}

/// Every item the workbench lists, for the game as it stands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../dev/protocol/")]
pub struct DevCatalogView {
    pub date: DateView,
    pub town: String,
    pub player: String,
    pub content_hash: String,
    pub items: Vec<DevItemView>,
    /// People in town the conversation items can be explained for.
    pub speakers: Vec<DevPersonView>,
    /// Places in town where people talk (overheard lines).
    pub places: Vec<DevPlaceView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../dev/protocol/")]
pub struct DevTextView {
    /// `title` | `situation` | `line` | `reason` | `stage:<id>` | `beat:<speaker>`.
    pub slot: String,
    pub key: String,
    pub args: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../dev/protocol/")]
pub struct DevCastView {
    pub role: String,
    pub actor: String,
}

/// Why an item could or could not fire now, and how it would read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../dev/protocol/")]
pub struct DevExplainView {
    pub kind: String,
    pub id: String,
    pub could_fire: bool,
    pub tree: DevNodeView,
    pub cast: Vec<DevCastView>,
    /// The cast was forced (roles filled regardless of conditions) to
    /// preview an item that could not fire.
    pub forced: bool,
    pub texts: Vec<DevTextView>,
    /// A dilemma as the player would see it with this cast: options and
    /// their previews (a decision that is never stored).
    pub decision: Option<DecisionView>,
    pub speaker: Option<u32>,
    pub place: Option<u32>,
}

fn node_view(n: &Node) -> DevNodeView {
    DevNodeView {
        kind: n.kind.clone(),
        label: n.label.clone(),
        detail: n.detail.clone(),
        ok: n.ok,
        values: n
            .values
            .iter()
            .map(|(k, v)| DevValueView {
                name: k.clone(),
                value: v.clone(),
            })
            .collect(),
        children: n.children.iter().map(node_view).collect(),
    }
}

fn title_and_group(engine: &Engine, kind: ItemKind, id: &str) -> (Option<String>, String) {
    let c = engine.content();
    match kind {
        ItemKind::Dilemma => c
            .dilemma(id)
            .map(|d| {
                let mut g = format!("{:?}", d.category).to_lowercase();
                if d.opportunity {
                    g.push_str(" · opportunity");
                }
                if d.arc_only {
                    g.push_str(" · arc");
                }
                if let Some(by) = d.raised_by {
                    g.push_str(&format!(" · raised by {by:?}").to_lowercase());
                }
                (Some(d.title_key.clone()), g)
            })
            .unwrap_or_default(),
        ItemKind::Situational => c
            .policies
            .get(id)
            .map(|p| (Some(p.name_key.clone()), "policy".into()))
            .unwrap_or_default(),
        ItemKind::Overheard => c
            .overheard
            .iter()
            .find(|o| o.id == id)
            .map(|o| {
                (
                    Some(o.key.clone()),
                    o.concern.clone().unwrap_or_else(|| "facts".into()),
                )
            })
            .unwrap_or_default(),
        ItemKind::Topic => c
            .talk_topic(id)
            .map(|t| (Some(t.name_key.clone()), t.mechanic.key().into()))
            .unwrap_or_default(),
        ItemKind::Line => c
            .talk_lines
            .iter()
            .find(|l| l.id == id)
            .map(|l| {
                let mut g = l.part.key().to_string();
                if let Some(t) = &l.topic {
                    g.push_str(&format!(" · {t}"));
                }
                if let Some(v) = &l.voice {
                    g.push_str(&format!(" · {v}"));
                }
                if l.fallback {
                    g.push_str(" · fallback");
                }
                (Some(l.key.clone()), g)
            })
            .unwrap_or_default(),
        ItemKind::Voice => c
            .talk_voice(id)
            .map(|v| (None, format!("priority {}", v.priority)))
            .unwrap_or_default(),
        ItemKind::Scene => c
            .scene(id)
            .map(|s| {
                (
                    s.beats.first().map(|b| b.line.clone()),
                    explain::describe(&s.place).1,
                )
            })
            .unwrap_or_default(),
    }
}

/// The catalogue: every item, with whether it could fire now.
pub fn dev_catalog(engine: &Engine) -> DevCatalogView {
    let st = engine.state();
    let content = engine.content();
    let items = explain::items(content)
        .into_iter()
        .map(|(kind, id)| {
            let x = explain::explain(st, content, kind, &id, Focus::default());
            let (title_key, group) = title_and_group(engine, kind, &id);
            DevItemView {
                kind: kind.key().into(),
                could_fire: x.as_ref().is_some_and(|x| x.could_fire),
                blocked_by: x
                    .as_ref()
                    .and_then(|x| x.tree.first_failure())
                    .map(|n| match n.kind.as_str() {
                        "condition" | "role" => format!("{} ({})", n.label, n.detail),
                        _ => n.label.clone(),
                    })
                    .unwrap_or_default(),
                id,
                title_key,
                group,
            }
        })
        .collect();
    let speakers = explain::speakers(st, content)
        .into_iter()
        .map(|s| {
            let (name, role) = explain::speaker_info(st, content, s);
            DevPersonView {
                id: s.0,
                name,
                role: role.unwrap_or_default(),
            }
        })
        .collect();
    let places = explain::talk_places(st, content)
        .into_iter()
        .filter_map(|p| {
            let x = st.world.places.get(p.index())?;
            Some(DevPlaceView {
                id: p.0,
                name: if x.name.is_empty() {
                    format!("{} #{}", x.kind.key(), p.0)
                } else {
                    x.name.clone()
                },
            })
        })
        .collect();
    // The town and the player as every decision names them.
    let named = cast_args(engine, &[]);
    let town = named.get("town").cloned().unwrap_or_default();
    let player = named.get("player").cloned().unwrap_or_default();
    DevCatalogView {
        date: DateView::of(st.calendar.date),
        town,
        player,
        content_hash: content.content_hash.clone(),
        items,
        speakers,
        places,
    }
}

/// A dilemma as the player would see it with this cast, from a decision
/// that is never stored.
fn preview_decision(engine: &Engine, x: &Explanation) -> Option<DecisionView> {
    let st = engine.state();
    let content = engine.content();
    let def = content.dilemma(&x.id)?;
    let date = st.calendar.date;
    let r = mandate_sim::scheduler::Reads::of_state(st, content);
    let v = mandate_sim::narrative::cast::View::new(&r, &st.politics);
    let place = def
        .scene
        .as_deref()
        .and_then(|s| content.scene(s))
        .and_then(|s| mandate_sim::narrative::director::scene_place(&v, &s.place, &x.cast));
    let d = Decision {
        arc: None,
        id: DecisionId(u32::MAX),
        template: def.id.clone(),
        weight: match def.category {
            mandate_content::DilemmaCategory::Climax => Weight::Climax,
            mandate_content::DilemmaCategory::Significant => Weight::Significant,
            mandate_content::DilemmaCategory::Request => Weight::Minor,
        },
        cast: x.cast.clone(),
        reasons: Vec::new(),
        opened: date,
        deadline: date.add_days(def.deadline_days.unwrap_or(content.director.deadline_days)),
        chapter: 0,
        relevance: 0.0,
        goal: None,
        place,
        status: DecisionStatus::Pending,
        resolution: None,
        about: None,
    };
    decision_view(engine, &d)
}

fn text(slot: &str, key: &str, args: &BTreeMap<String, String>) -> DevTextView {
    DevTextView {
        slot: slot.into(),
        key: key.into(),
        args: args.clone(),
    }
}

/// Explain one item now (`speaker`, `place`: whom or where to explain a
/// conversation item or an overheard line for).
pub fn dev_explain(
    engine: &Engine,
    kind: &str,
    id: &str,
    speaker: Option<u32>,
    place: Option<u32>,
) -> Result<DevExplainView, String> {
    let st = engine.state();
    let content = engine.content();
    let k = ItemKind::parse(kind).ok_or_else(|| format!("unknown item kind `{kind}`"))?;
    let focus = Focus {
        speaker: speaker.map(mandate_sim::characters::CharacterId),
        place: place.map(mandate_sim::world::PlaceId),
    };
    let x = explain::explain(st, content, k, id, focus)
        .ok_or_else(|| format!("no {kind} `{id}` to explain (or no player)"))?;
    let args = cast_args(engine, &x.cast);
    let r = mandate_sim::scheduler::Reads::of_state(st, content);
    let v = mandate_sim::narrative::cast::View::new(&r, &st.politics);
    let mut texts: Vec<DevTextView> = Vec::new();
    let mut decision = None;
    match k {
        ItemKind::Dilemma => {
            decision = preview_decision(engine, &x);
            if let Some(d) = &decision {
                texts.push(text("title", &d.title_key, &d.args));
                texts.push(text("situation", &d.situation_key, &d.args));
            }
        }
        ItemKind::Scene => {
            if let Some(def) = content.scene(id) {
                for b in &def.beats {
                    let who = if b.speaker == "player" {
                        "player".to_string()
                    } else {
                        mandate_sim::narrative::actor_of(&x.cast, &b.speaker)
                            .map(|a| explain::actor_label(&v, a))
                            .unwrap_or_else(|| b.speaker.clone())
                    };
                    texts.push(text(&format!("beat:{who}"), &b.line, &args));
                }
            }
        }
        ItemKind::Overheard => {
            // The line as the draw gives it at that place (with a retold
            // story's headline), else with the arguments its conditions give.
            let real = x.place.and_then(|p| {
                let loc = mandate_sim::places::Location::Place { place: p };
                mandate_sim::overheard::available(st, content, loc)
                    .into_iter()
                    .find(|l| l.id == id)
            });
            match real {
                Some(l) => {
                    let mut a = l.args;
                    if let Some(art) = l
                        .story
                        .and_then(|sid| st.media.stories.iter().find(|s| s.id == sid))
                        .and_then(|s| crate::media::article(engine, s))
                    {
                        for (k, v) in art.args {
                            a.entry(k).or_insert(v);
                        }
                        a.insert("headline_key".into(), art.headline_key);
                        a.insert("outlet_key".into(), art.outlet.name_key);
                        a.insert("outlet_place".into(), art.outlet.place);
                    }
                    texts.push(text("line", &l.key, &a));
                }
                None => texts.extend(x.texts.iter().map(|t| text(&t.slot, &t.key, &t.args))),
            }
        }
        _ => texts.extend(x.texts.iter().map(|t| text(&t.slot, &t.key, &t.args))),
    }
    Ok(DevExplainView {
        kind: k.key().into(),
        id: x.id.clone(),
        could_fire: x.could_fire,
        tree: node_view(&x.tree),
        cast: x
            .cast
            .iter()
            .map(|e| DevCastView {
                role: e.role.clone(),
                actor: explain::actor_label(&v, e.actor),
            })
            .collect(),
        forced: x.forced,
        texts,
        decision,
        speaker: x.speaker.map(|s| s.0),
        place: x.place.map(|p| p.0),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    /// The views serialise (the TypeScript types are generated with them).
    #[test]
    fn a_node_serialises_with_its_children() {
        let n = Node::group(
            "all",
            "x",
            vec![Node::new("condition", "track", "track=standing", false).with("value", 3)],
        );
        let j = serde_json::to_value(node_view(&n)).unwrap();
        assert_eq!(j["children"][0]["values"][0]["value"], "3");
        assert_eq!(j["ok"], false);
    }
}
