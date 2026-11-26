//! A scene of an arc as the player sees it (DEC-112; `EXCHANGE_SCENES.md`
//! §5.7, §7, task X2): the engine's own view (`arcs::exchange::view`),
//! carried to the screen with the same redaction.
//!
//! **Shown = rolled.** Every change an option previews is the engine's
//! `landing::preview`: the one formula, at the ends of what the player has
//! not learned. Here it is only put in the screen's units: points (a
//! hundredth of the lean scale), rounded *outwards* where it is a range so
//! that the truth is always inside what is shown, and steps (none, a
//! little, much, a great deal) for the tags. A chance, a cost and a side
//! effect are always exact.
//!
//! **Redaction.** A group's lean, and how far it moved, are given only
//! where the engine's view gives the lean (the player knows the group's
//! worries, or the person, at `known`). Otherwise the strip says "not known"
//! and a move is given in steps, never as a number (DEC-091: the room is
//! seen to stir; how far a stranger moved is not). The result card follows
//! the same rule.
//!
//! **Text (DEC-054).** Keys and facts only: the scene's own keys
//! (`<text>-prompt-…`, `-say-…`, `-reply-…`, `-result-…`, `-headline-…`,
//! `-title`, `-open`, `-offer`) and the shared `exchange-*` set, with the
//! cast by role and the line's own arguments. An argument named `…_key`
//! is a Fluent key.
//!
//! **Tier-generic (DEC-098).** A scene is a run's beat; its audience is
//! whatever the engine bound; nothing here names an office, a body or a
//! place.

use crate::characters::{PersonRefView, person_ref};
use crate::places::{LocationView, RouteView, location_view};
use crate::pocket::{TextView, route_to};
use crate::views::DateView;
use mandate_content::exchange::MoveKind;
use mandate_content::visits::VisitTime;
use mandate_sim::Engine;
use mandate_sim::arcs::exchange::landing::Span;
use mandate_sim::arcs::exchange::moves::AnswerSeen;
use mandate_sim::arcs::exchange::view::{GroupView, Phase, SceneView};
use mandate_sim::arcs::exchange::{self, Group};
use mandate_sim::arcs::{ArcId, ArcRun, cast};
use mandate_sim::events::{EventId, EventKind};
use mandate_sim::spine::Cause;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// What the player does in a scene (`CommandDto::ArcExchange`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneActDto {
    /// Turn up: at the scene's place (send it inside `Go`), for its time.
    Begin,
    /// Say one of the lines on offer (`SceneOptionView.id`).
    Say { option: String },
    /// Walk out: the rounds left are forfeited.
    Leave,
}

impl SceneActDto {
    pub fn to_sim(self) -> exchange::ExchangeAct {
        match self {
            SceneActDto::Begin => exchange::ExchangeAct::Begin,
            SceneActDto::Say { option } => exchange::ExchangeAct::Say { option },
            SceneActDto::Leave => exchange::ExchangeAct::Leave,
        }
    }
}

/// A change in a group's lean, as far as the player can know it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneChangeView {
    /// The group (`SceneGroupView.key`).
    pub group: String,
    /// In points (a hundredth of the lean scale), rounded outwards: the
    /// truth is always between `lo` and `hi`.
    pub lo: i32,
    pub hi: i32,
    /// Known exactly (`lo == hi`); else a range: the player does not know
    /// how much this group cares, or what the other side holds.
    pub exact: bool,
    /// The ends in steps (−3..3: none, a little, much, a great deal), for
    /// the tags ("▲▲", "▼", "?" where they differ in sign).
    pub lo_steps: i8,
    pub hi_steps: i8,
}

/// A group's lean, where the player can know it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneLeanView {
    /// On the strip's five-step scale: −2 (firmly against) to 2 (firmly with).
    pub step: i8,
    /// In points (−100..100).
    pub points: i32,
    /// In words (`exchange-lean-*`).
    pub key: String,
}

/// One group of the audience, as the strip shows it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneGroupView {
    /// Stable (`hood.4`, `hood.4.c1`, `person.12`, `elsewhere`).
    pub key: String,
    /// Its full name (`exchange-group-*`), for the accessible name and *More*.
    pub label: TextView,
    /// Its name on a narrow chip: a class of a place is the class.
    pub short: TextView,
    /// A member or a cast person.
    pub person: Option<PersonRefView>,
    /// How much it counts: 1, 2 or 3 (the chip's width).
    pub size: u8,
    /// Its lean now; `None` where the player cannot know it.
    pub lean: Option<SceneLeanView>,
    /// The lean in words (`exchange-lean-*`; `exchange-lean-unknown`).
    pub lean_key: String,
    /// How it moved in the last round played, in steps (−3..3).
    pub moved_steps: Option<i8>,
    /// And in points, only where the lean is known.
    pub moved_points: Option<i32>,
}

/// The prompt on the table: the other side's line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScenePromptView {
    /// Who puts it: the other side, or the chair.
    pub speaker: Option<PersonRefView>,
    /// `other` | `chair`.
    pub by: String,
    /// The source's id (`broken_promise`, `top_worry`, …).
    pub source: String,
    pub text: TextView,
    /// The event behind it, to ask "Why?" about, while the log keeps it.
    pub event: Option<u32>,
    /// It stood up. One that did not turns back on whoever made it.
    pub stood: bool,
    /// What it leaves on each group if nothing answers it.
    pub bites: Vec<SceneChangeView>,
}

/// One line the player may say this round, with what it would do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneOptionView {
    /// Stable (`record:law.17`, `pass`): `SceneActDto::Say { option }`.
    pub id: String,
    /// `record`, `heard`, `turn`, `promise`, `act`, `own`, `deny`,
    /// `offer`, `second`, `proof`, `pass`.
    pub kind: String,
    /// The row's label (`exchange-label-<kind>`).
    pub label: TextView,
    /// The line's own arguments (a law, a group, a promise): what the
    /// sentence said, its answer and the round's notes read.
    pub args: BTreeMap<String, String>,
    /// It answers the prompt (else it changes the subject).
    pub answers: bool,
    /// The safe line: always there, needs nothing.
    pub safe: bool,
    /// Preparation added it.
    pub prep: bool,
    /// The group it names, if it names one.
    pub target: Option<String>,
    /// The chance it stands up, in percent; `None`: no chance involved.
    pub chance: Option<u32>,
    /// What it does to each group if it stands, in the groups' order.
    pub stands: Vec<SceneChangeView>,
    /// And if it does not (empty when there is no chance).
    pub fails: Vec<SceneChangeView>,
    /// Up to three groups it moves most, largest first (the row's tags).
    pub tags: Vec<SceneChangeView>,
    /// Costs and side effects, as sentences. Always exact.
    pub notes: Vec<TextView>,
    /// What the player can tell of the other side's answer:
    /// `never` | `no` | `yes` | `maybe`.
    pub answer: String,
    /// The same in words, where there is anything to say.
    pub answer_key: Option<String>,
}

/// The line the scene allows and the player cannot say, with a route to it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneLockedView {
    pub kind: String,
    /// `exchange-locked-<reason>`: the line, and how to come by it.
    pub text: TextView,
}

/// How a group moved over a round.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneMoveView {
    pub group: String,
    pub steps: i8,
    /// Only where the player knows the group's lean.
    pub points: Option<i32>,
}

/// A round that was played.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneRoundView {
    pub round: u8,
    /// The prompt's key (`<text>-prompt-<source>-<n>`); its arguments are
    /// the ones it had when it was put (`ScenePromptView.text`).
    pub prompt_key: String,
    /// The line the player said (`<text>-say-<kind>-<n>`), its option and
    /// kind; `None` for a round nobody answered.
    pub said_key: Option<String>,
    pub option: Option<String>,
    pub kind: Option<String>,
    /// The other side's answer to it, where they had one.
    pub reply_key: Option<String>,
    /// How the room took it (`exchange-room-*`).
    pub room_key: String,
    pub stood: bool,
    pub moved: Vec<SceneMoveView>,
    /// The round's event ("Why?").
    pub event: u32,
}

/// A group at the end.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneResultGroupView {
    pub group: String,
    /// Before and after, where the player can know the lean.
    pub before: Option<SceneLeanView>,
    pub after: Option<SceneLeanView>,
    /// The change over the scene in steps; in points only where known.
    pub steps: i8,
    pub points: Option<i32>,
    /// The change in words (`exchange-moved-*`).
    pub moved_key: String,
}

/// A promise made on stage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScenePromiseView {
    pub ask: u32,
    pub title: TextView,
    pub due: DateView,
}

/// How it ended.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SceneResultView {
    /// `won` | `split` | `lost` | `absent`.
    pub outcome: String,
    /// The outcome in a word (`exchange-outcome-*`).
    pub outcome_key: String,
    /// How it ended (`<text>-result-<outcome>`), as the log keeps it.
    pub text: TextView,
    /// What the paper will say (`<text>-headline-<outcome>`).
    pub headline: TextView,
    pub groups: Vec<SceneResultGroupView>,
    pub promises: Vec<ScenePromiseView>,
    /// The closing event: "Why?" reads the rounds back from it.
    pub event: u32,
}

/// A scene (`QueryDto::ArcExchange`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArcSceneView {
    pub arc: u32,
    /// `<stage>.<beat>`, as the command names it.
    pub beat: String,
    /// `ahead` | `open` | `under_way` | `done`.
    pub phase: String,
    /// `crowd` | `members` | `table`.
    pub layout: String,
    /// Its title (`<text>-title`), the day it is on (`<text>-offer`) and
    /// turning up (`<text>-open`).
    pub title: TextView,
    pub offer: TextView,
    pub opening: TextView,
    /// The cast by role, the scene's dates and the sides' names: every
    /// sentence of the scene takes these, with its own on top.
    pub args: BTreeMap<String, String>,
    pub place: Option<LocationView>,
    /// Where to go to begin it (`CommandDto::Go` with the begin act).
    pub route: Option<RouteView>,
    /// Its time, in half slots.
    pub halves: u32,
    /// It can be begun now (open, and the time is there).
    pub available: bool,
    /// Why not, as a Fluent key, when it is open and cannot be begun.
    pub blocked_key: Option<String>,
    pub last_day: Option<DateView>,
    pub days_left: Option<u32>,
    pub other: Option<PersonRefView>,
    pub chair: Option<PersonRefView>,
    pub second: Option<PersonRefView>,
    pub groups: Vec<SceneGroupView>,
    /// The round on the table (from 1; 0 before it begins).
    pub round: u8,
    pub rounds: u8,
    pub prompt: Option<ScenePromptView>,
    /// Four or five lines, in the engine's order (never best first).
    pub options: Vec<SceneOptionView>,
    /// At most one.
    pub locked: Option<SceneLockedView>,
    /// The rounds played so far, in order.
    pub played: Vec<SceneRoundView>,
    pub result: Option<SceneResultView>,
}

// --- Units ---------------------------------------------------------------------------------

fn points(x: f64) -> i32 {
    (x * 100.0).round() as i32
}

/// A change in steps against the scene's own margin for a group: under
/// half of it none, under two margins a little, under four much, beyond a
/// great deal. Monotone, so a value inside a range has a step inside the
/// range's steps.
fn steps(x: f64, margin: f64) -> i8 {
    let a = x.abs();
    let s = if a < margin / 2.0 {
        0
    } else if a < 2.0 * margin {
        1
    } else if a < 4.0 * margin {
        2
    } else {
        3
    };
    if x < 0.0 { -s } else { s }
}

/// A lean on the strip's five-step scale.
fn lean_step(lean: f64, margin: f64) -> i8 {
    if lean <= -0.25 {
        -2
    } else if lean <= -margin {
        -1
    } else if lean < margin {
        0
    } else if lean < 0.25 {
        1
    } else {
        2
    }
}

fn lean_view(lean: f64, margin: f64) -> SceneLeanView {
    let step = lean_step(lean, margin);
    SceneLeanView {
        step,
        points: points(lean),
        key: LEAN_KEYS[(step + 2) as usize].to_string(),
    }
}

const LEAN_KEYS: [&str; 5] = [
    "exchange-lean-against-2",
    "exchange-lean-against-1",
    "exchange-lean-even",
    "exchange-lean-with-1",
    "exchange-lean-with-2",
];

fn lean_key(lean: Option<&SceneLeanView>) -> String {
    lean.map_or_else(|| "exchange-lean-unknown".to_string(), |l| l.key.clone())
}

fn moved_key(steps: i8) -> String {
    match steps {
        s if s >= 2 => "exchange-moved-with-much",
        1 => "exchange-moved-with",
        0 => "exchange-moved-none",
        -1 => "exchange-moved-against",
        _ => "exchange-moved-against-much",
    }
    .to_string()
}

fn change(group: &str, s: &Span, margin: f64) -> SceneChangeView {
    let exact = s.exact();
    // Rounded from the f32 the result is kept in (`moved`), so a change at a
    // half point shows as it will read (−0.475 is −47 there, not −48).
    let (slo, shi) = (f64::from(s.lo as f32), f64::from(s.hi as f32));
    let (lo, hi) = if exact {
        (points(slo), points(slo))
    } else {
        ((slo * 100.0).floor() as i32, (shi * 100.0).ceil() as i32)
    };
    SceneChangeView {
        group: group.to_string(),
        lo,
        hi,
        exact,
        lo_steps: steps(s.lo, margin),
        hi_steps: steps(s.hi, margin),
    }
}

fn changes(groups: &[String], spans: &[Span], margin: f64) -> Vec<SceneChangeView> {
    groups
        .iter()
        .zip(spans)
        .map(|(g, s)| change(g, s, margin))
        .collect()
}

fn args_of(v: &[(String, String)]) -> BTreeMap<String, String> {
    v.iter().cloned().collect()
}

fn halves(t: VisitTime) -> u32 {
    match t {
        VisitTime::Half => 1,
        VisitTime::Slot => 2,
    }
}

fn phase_key(p: Phase) -> &'static str {
    match p {
        Phase::Ahead => "ahead",
        Phase::Open => "open",
        Phase::UnderWay => "under_way",
        Phase::Done => "done",
    }
}

fn answer_key(a: AnswerSeen) -> (&'static str, Option<String>) {
    match a {
        AnswerSeen::Never => ("never", None),
        AnswerSeen::No => ("no", Some("exchange-answer-no".into())),
        AnswerSeen::Yes => ("yes", Some("exchange-answer-yes".into())),
        AnswerSeen::Maybe => ("maybe", Some("exchange-answer-maybe".into())),
    }
}

/// A group's name, full and short (facts and keys).
fn group_names(engine: &Engine, g: &GroupView) -> (TextView, TextView) {
    let name = g.name.clone().unwrap_or_default();
    match &g.group {
        Group::Area {
            class: Some(c),
            community: None,
            ..
        } => {
            let class = format!("exchange-class-{c}");
            (
                TextView::new("exchange-group-class")
                    .with("name", name)
                    .with("class_key", class.clone()),
                TextView::new(class),
            )
        }
        Group::Area {
            community: Some(k), ..
        } => {
            let st = engine.state();
            let community = st
                .population
                .community_keys
                .get(usize::from(*k))
                .and_then(|id| engine.content().community_index(id))
                .map(|i| engine.content().communities[i].name_key.clone())
                .unwrap_or_else(|| "community-general".into());
            (
                TextView::new("exchange-group-community")
                    .with("name", name)
                    .with("community_key", community.clone()),
                TextView::new(community),
            )
        }
        Group::Elsewhere { .. } => (
            TextView::new("exchange-group-elsewhere"),
            TextView::new("exchange-group-elsewhere"),
        ),
        _ => (
            TextView::new("exchange-group-named").with("name", name.clone()),
            TextView::new("exchange-group-named").with("name", name),
        ),
    }
}

fn size_of(weight: f32) -> u8 {
    if weight >= 0.3 {
        3
    } else if weight >= 0.15 {
        2
    } else {
        1
    }
}

/// The scene's shared arguments: the run's cast and dates, the sides.
fn scene_args(engine: &Engine, run: &ArcRun, v: &SceneView) -> BTreeMap<String, String> {
    let st = engine.state();
    let mut a = crate::arcs::live_args(engine, run);
    a.extend(cast::dates(run, st.calendar.date));
    let name = |id: Option<mandate_sim::characters::CharacterId>| {
        id.and_then(|c| mandate_sim::characters::profile(&st.politics, &st.world, c))
            .map(|p| p.full_name())
    };
    if let Some(n) = name(v.other) {
        a.insert("other".into(), n);
    }
    if let Some(n) = name(v.second) {
        a.insert("second".into(), n);
    }
    if let Some(n) = name(v.chair) {
        a.insert("chair".into(), n);
    }
    a
}

/// The promises a scene's rounds made: asks accepted with a round's event
/// as their cause.
fn promises(engine: &Engine, rounds: &[EventId]) -> Vec<ScenePromiseView> {
    let st = engine.state();
    st.events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::AskAccepted { ask, .. }
                if e.causes
                    .iter()
                    .any(|c| matches!(c, Cause::Event(id, _) if rounds.contains(id))) =>
            {
                Some(*ask)
            }
            _ => None,
        })
        .filter_map(|id| {
            let a = st.asks.get(mandate_sim::asks::AskId(id))?;
            let v = crate::asks::ask_view(engine, a);
            Some(ScenePromiseView {
                ask: id,
                title: TextView::of(v.title_key, v.args),
                due: DateView::of(a.due),
            })
        })
        .collect()
}

// --- The view ------------------------------------------------------------------------------

/// A scene of a run as the player sees it, by `<stage>.<beat>` (or the
/// beat's id in the run's current stage). `None` where there is no such
/// scene, or the player knows nothing of its arc (the two read the same).
pub fn scene_view(engine: &Engine, arc: u32, beat: &str) -> Option<ArcSceneView> {
    build(engine, arc, beat, false)
}

/// The same with nothing hidden (the lab's reading; tests).
pub fn scene_view_known(engine: &Engine, arc: u32, beat: &str) -> Option<ArcSceneView> {
    build(engine, arc, beat, true)
}

#[allow(clippy::too_many_lines)]
fn build(engine: &Engine, arc: u32, beat: &str, known: bool) -> Option<ArcSceneView> {
    let st = engine.state();
    let content = engine.content();
    st.player.as_ref()?;
    let run = st.arcs.run(ArcId(arc))?;
    let bref = exchange::beat_ref(run, beat);
    let sc = exchange::scene(content, run, &bref)?;
    let margin = sc.params.group_margin;
    let v = if known {
        exchange::view_known(st, content, arc, &bref.key())?
    } else {
        exchange::view(st, content, arc, &bref.key())?
    };
    let today = st.calendar.date;
    let args = scene_args(engine, run, &v);
    let text = |suffix: &str| TextView::of(format!("{}-{suffix}", v.text), args.clone());
    let keys: Vec<String> = v.groups.iter().map(|g| g.key.clone()).collect();
    let knows: Vec<bool> = v.groups.iter().map(|g| g.lean.is_some()).collect();

    let groups: Vec<SceneGroupView> = v
        .groups
        .iter()
        .map(|g| {
            let (label, short) = group_names(engine, g);
            let lean = g.lean.map(|l| lean_view(f64::from(l), margin));
            SceneGroupView {
                key: g.key.clone(),
                label,
                short,
                person: g.group.person().and_then(|id| person_ref(engine, id)),
                size: size_of(g.weight),
                lean_key: lean_key(lean.as_ref()),
                moved_steps: g.moved.map(|m| steps(f64::from(m), margin)),
                moved_points: g
                    .moved
                    .filter(|_| lean.is_some())
                    .map(|m| points(f64::from(m))),
                lean,
            }
        })
        .collect();

    let prompt = v.prompt.as_ref().map(|p| ScenePromptView {
        speaker: p.speaker.and_then(|id| person_ref(engine, id)),
        by: match p.by {
            mandate_content::exchange::Speaker::Other => "other",
            mandate_content::exchange::Speaker::Chair => "chair",
        }
        .into(),
        source: p.source.key().into(),
        text: {
            let mut a = args.clone();
            a.extend(args_of(&p.args));
            TextView::of(p.key.clone(), a)
        },
        event: p.event.map(|e| e.0),
        stood: p.stood,
        bites: changes(&keys, &p.bites, margin),
    });

    let options: Vec<SceneOptionView> = v
        .options
        .iter()
        .map(|o| {
            let stands = changes(&keys, &o.stands, margin);
            let chance = (o.chance < 1.0).then(|| (o.chance * 100.0).round() as u32);
            let fails = if chance.is_some() {
                changes(&keys, &o.fails, margin)
            } else {
                Vec::new()
            };
            // The tags: the groups it moves most if it stands, largest first
            // (ties in the groups' order).
            let mut order: Vec<usize> = (0..stands.len()).collect();
            let reach = |c: &SceneChangeView| c.lo.abs().max(c.hi.abs());
            order.sort_by_key(|i| std::cmp::Reverse(reach(&stands[*i])));
            let tags: Vec<SceneChangeView> = order
                .into_iter()
                .filter(|i| reach(&stands[*i]) > 0)
                .take(3)
                .map(|i| stands[i].clone())
                .collect();
            let (answer, answer_key) = answer_key(o.answer);
            let mut largs = args.clone();
            largs.extend(args_of(&o.args));
            SceneOptionView {
                id: o.id.clone(),
                kind: o.kind.key().into(),
                label: TextView::of(o.label_key.clone(), largs),
                args: args_of(&o.args),
                answers: o.answers,
                safe: o.kind == MoveKind::Pass,
                prep: o.prep,
                target: o.target.and_then(|t| keys.get(t).cloned()),
                chance,
                stands,
                fails,
                tags,
                notes: o
                    .notes
                    .iter()
                    .map(|n| {
                        let mut a = args.clone();
                        a.extend(args_of(&n.args));
                        TextView::of(n.key.clone(), a)
                    })
                    .collect(),
                answer: answer.into(),
                answer_key,
            }
        })
        .collect();

    let locked = v.locked.as_ref().map(|l| {
        let mut a = args.clone();
        a.extend(args_of(&l.args));
        SceneLockedView {
            kind: l.kind.key().into(),
            text: TextView::of(l.key.clone(), a),
        }
    });

    let played: Vec<SceneRoundView> = v
        .played
        .iter()
        .zip(&v.notes)
        .map(|(r, n)| SceneRoundView {
            round: n.round,
            prompt_key: n.prompt_key.clone(),
            said_key: n.say_key.clone(),
            option: r.kind.map(|_| r.option.clone()),
            kind: r.kind.map(|k| k.key().into()),
            reply_key: n.reply_key.clone(),
            room_key: n.room_key.clone(),
            stood: r.stood,
            moved: keys
                .iter()
                .zip(&r.deltas)
                .zip(&knows)
                .map(|((g, d), k)| SceneMoveView {
                    group: g.clone(),
                    steps: steps(f64::from(*d), margin),
                    points: k.then(|| points(f64::from(*d))),
                })
                .collect(),
            event: r.event.0,
        })
        .collect();

    let result = v.result.as_ref().map(|r| {
        let rounds: Vec<EventId> = v.played.iter().map(|x| x.event).collect();
        SceneResultView {
            outcome: r.outcome.key().into(),
            outcome_key: format!("exchange-outcome-{}", r.outcome.key()),
            text: TextView::of(r.key.clone(), args.clone()),
            headline: text(&format!("headline-{}", r.outcome.key())),
            groups: r
                .groups
                .iter()
                .zip(&knows)
                .map(|((key, before, after), k)| {
                    let d = f64::from(*after) - f64::from(*before);
                    let s = steps(d, margin);
                    SceneResultGroupView {
                        group: key.clone(),
                        before: k.then(|| lean_view(f64::from(*before), margin)),
                        after: k.then(|| lean_view(f64::from(*after), margin)),
                        steps: s,
                        points: k.then(|| points(d)),
                        moved_key: moved_key(s),
                    }
                })
                .collect(),
            promises: promises(engine, &rounds),
            event: r.event.0,
        }
    });

    // Turning up: open, at the place, with the time there.
    let route = v.place.and_then(|l| route_to(engine, "visit", l));
    let (available, blocked_key) = match v.phase {
        Phase::Open if route.is_none() => (false, Some("visit-blocked-situation".to_string())),
        Phase::Open if !mandate_sim::arcs::beats::has_time(st, v.time) => {
            (false, Some("visit-blocked-time".to_string()))
        }
        Phase::Open => (true, None),
        _ => (false, None),
    };
    Some(ArcSceneView {
        arc,
        beat: bref.key(),
        phase: phase_key(v.phase).into(),
        layout: match v.layout {
            mandate_content::exchange::Layout::Crowd => "crowd",
            mandate_content::exchange::Layout::Members => "members",
            mandate_content::exchange::Layout::Table => "table",
        }
        .into(),
        title: text("title"),
        offer: text("offer"),
        opening: text("open"),
        place: v.place.map(|l| location_view(st, l)),
        route,
        halves: halves(v.time),
        available,
        blocked_key,
        last_day: v.last_day.map(DateView::of),
        days_left: v
            .last_day
            .filter(|d| *d >= today)
            .map(|d| today.days_until(d)),
        other: v.other.and_then(|id| person_ref(engine, id)),
        chair: v.chair.and_then(|id| person_ref(engine, id)),
        second: v.second.and_then(|id| person_ref(engine, id)),
        groups,
        round: v.round,
        rounds: v.rounds,
        prompt,
        options,
        locked,
        played,
        result,
        args,
    })
}

/// A scene the player can begin now or has under way, for Today's "Needs
/// you": (run, `<stage>.<beat>`, the last day it can be begun).
pub fn scenes_waiting(engine: &Engine) -> Vec<(u32, String, Option<mandate_core::SimDate>)> {
    let st = engine.state();
    let content = engine.content();
    let mut out: Vec<(u32, String, Option<mandate_core::SimDate>)> =
        exchange::view::open_scenes(st, content)
            .into_iter()
            .filter_map(|(run, b, _)| {
                let r = st.arcs.run(run)?;
                Some((run.0, b.key(), exchange::last_day(r)))
            })
            .collect();
    for r in &st.arcs.runs {
        if let Some(x) = &r.exchange
            && !out.iter().any(|o| o.0 == r.id.0 && o.1 == x.beat.key())
        {
            out.push((r.id.0, x.beat.key(), Some(st.calendar.date)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_monotone_and_signed() {
        let m = 0.05;
        let xs: Vec<f64> = (-40..=40).map(|i| f64::from(i) / 100.0).collect();
        for w in xs.windows(2) {
            assert!(steps(w[0], m) <= steps(w[1], m));
        }
        assert_eq!(steps(0.0, m), 0);
        assert_eq!(steps(0.06, m), 1);
        assert_eq!(steps(-0.12, m), -2);
        assert_eq!(steps(0.5, m), 3);
    }

    #[test]
    fn a_range_is_rounded_outwards() {
        let c = change(
            "g",
            &Span {
                lo: -0.031,
                hi: 0.049,
            },
            0.05,
        );
        assert_eq!((c.lo, c.hi, c.exact), (-4, 5, false));
        let e = change(
            "g",
            &Span {
                lo: 0.031,
                hi: 0.031,
            },
            0.05,
        );
        assert_eq!((e.lo, e.hi, e.exact), (3, 3, true));
    }
}
