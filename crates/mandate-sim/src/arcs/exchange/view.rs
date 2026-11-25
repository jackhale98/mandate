//! What a player sees of a scene (EXCHANGE_SCENES §5.7, §11.3): facts and
//! Fluent keys, with every size as far as the player can know it.
//!
//! **Shown = rolled.** An option's changes are [`landing::preview`]: the
//! one formula at the ends of what the player has not learned. With
//! everything known ([`view_known`], or a ledger that knows everything)
//! the preview is the landing the command applies, to the bit. A chance,
//! a cost and a side effect are always exact: the ledger never gates what
//! an action does (DEC-091), only how precisely a size can be told.
//!
//! The screen and its DTO are a later task (X2): this is the engine's own
//! view, which a scripted player, a stand-in or a bot reads to choose a
//! line, and which the protocol will carry.

use super::landing::{self, Span};
use super::moves::{self, AnswerSeen, Locked, Note};
use super::state::{ExchangePlayed, Group, RoundPlayed, What};
use super::{Scene, Staging, audience};
use crate::arcs::{ArcId, ArcRun, BeatRef};
use crate::characters::CharacterId;
use crate::events::EventId;
use crate::places::Location;
use crate::state::State;
use mandate_content::ContentBundle;
use mandate_content::exchange::{Layout, MoveKind, Outcome, SourceKind, Speaker};
use mandate_content::knowledge::Level;
use mandate_content::visits::VisitTime;
use mandate_core::SimDate;

/// Where a scene stands for the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Its stage has not come (or this chapter's scene is another arc's).
    Ahead,
    /// On offer: `Begin` at its place.
    Open,
    /// Begun: a prompt is on the table.
    UnderWay,
    /// Over: won, split, lost or absent.
    Done,
}

/// One group as the strip shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupView {
    /// Stable (`hood.4`, `person.12`, `elsewhere`).
    pub key: String,
    pub group: Group,
    /// An area's or a person's name; `None` for "elsewhere".
    pub name: Option<String>,
    pub weight: f32,
    /// Its lean now, if the player knows the group well enough to read it.
    pub lean: Option<f32>,
    /// How it moved in the last round played.
    pub moved: Option<f32>,
}

/// The prompt on the table.
#[derive(Clone, Debug, PartialEq)]
pub struct PromptView {
    pub by: Speaker,
    pub speaker: Option<CharacterId>,
    pub source: SourceKind,
    /// `<text>-prompt-<source>-<variant>`.
    pub key: String,
    pub args: Vec<(String, String)>,
    /// The event behind it, for "Why?".
    pub event: Option<EventId>,
    /// It stood up. One that did not turned back on whoever made it.
    pub stood: bool,
    /// What it leaves standing on each group if nothing answers it.
    pub bites: Vec<Span>,
}

/// One line on offer, with what it would do.
#[derive(Clone, Debug, PartialEq)]
pub struct OptionView {
    pub id: String,
    pub kind: MoveKind,
    /// `exchange-label-<kind>`.
    pub label_key: String,
    /// `<text>-say-<kind>`: the sentence, less its wording's number.
    pub say_key: String,
    pub args: Vec<(String, String)>,
    /// The group a targeted line names (an index into the groups).
    pub target: Option<usize>,
    /// It answers the prompt (else it changes the subject).
    pub answers: bool,
    /// The chance it stands up (1: no chance). Always exact.
    pub chance: f64,
    /// The change in each group's lean if it stands: a number or a range.
    pub stands: Vec<Span>,
    /// And if it does not (the same as `stands` when there is no chance).
    pub fails: Vec<Span>,
    /// Costs and side effects. Always exact.
    pub notes: Vec<Note>,
    /// What the player can tell of the other side's answer.
    pub answer: AnswerSeen,
    /// Preparation added it.
    pub prep: bool,
}

/// How it ended.
#[derive(Clone, Debug, PartialEq)]
pub struct ResultView {
    pub outcome: Outcome,
    /// Σ weight × change in lean.
    pub result: f32,
    /// `<text>-result-<outcome>`.
    pub key: String,
    /// Each group: (key, lean before, lean after).
    pub groups: Vec<(String, f32, f32)>,
    pub event: EventId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneView {
    pub run: ArcId,
    pub beat: BeatRef,
    pub phase: Phase,
    pub place: Option<Location>,
    pub time: VisitTime,
    /// The last day it can be begun.
    pub last_day: Option<SimDate>,
    pub layout: Layout,
    /// The key prefix of its sentences.
    pub text: String,
    pub other: Option<CharacterId>,
    pub chair: Option<CharacterId>,
    pub second: Option<CharacterId>,
    pub groups: Vec<GroupView>,
    pub round: u8,
    pub rounds: u8,
    pub prompt: Option<PromptView>,
    /// Four or five lines, in the table's order (never "best first").
    pub options: Vec<OptionView>,
    /// One line the scene allows and the player cannot say, with a route.
    pub locked: Option<Locked>,
    pub played: Vec<RoundPlayed>,
    pub result: Option<ResultView>,
}

/// Whether a scene can be begun now: its stage is current, it is not
/// done, its day has not passed, its roles are cast, the player can act,
/// and this chapter's scene is this run's to hold.
pub fn open_now(state: &State, content: &ContentBundle, run: &ArcRun, sc: &Scene<'_>) -> bool {
    if run.stage != sc.bref.stage
        || run.is_done(&sc.bref)
        || run.exchange.is_some()
        || super::day_passed(run, state.calendar.date)
        || super::place(run, sc.def).is_none()
        || !state.player.as_ref().is_some_and(|p| p.status.active())
    {
        return false;
    }
    let staging = super::staging(&state.arcs.runs, content, state.narrative.chapter.as_ref());
    if !staging.allows(run.id) {
        return false;
    }
    crate::condition::on_state(state, content, |v| {
        crate::arcs::step::available(v, run, &run.narrative_cast(), sc.stage, sc.beat)
    })
}

/// The scenes on offer now: (run, beat, place).
pub fn open_scenes(state: &State, content: &ContentBundle) -> Vec<(ArcId, BeatRef, Location)> {
    let mut out = Vec::new();
    for run in &state.arcs.runs {
        let Some(stage) = content.arc(&run.template).and_then(|a| a.stage(&run.stage)) else {
            continue;
        };
        for beat in &stage.beats {
            let bref = BeatRef::new(&stage.id, &beat.id);
            let Some(sc) = super::scene(content, run, &bref) else {
                continue;
            };
            if open_now(state, content, run, &sc)
                && let Some(at) = super::place(run, sc.def)
            {
                out.push((run.id, bref, at));
            }
        }
    }
    out
}

fn span_all(v: &[f64]) -> Vec<Span> {
    v.iter().map(|x| Span { lo: *x, hi: *x }).collect()
}

/// The scene as the player sees it.
pub fn view(state: &State, content: &ContentBundle, run: u32, beat: &str) -> Option<SceneView> {
    build(state, content, run, beat, false)
}

/// The scene with every size known (the lab's reading, and the test that
/// the preview is the landing): the same view, no ranges.
pub fn view_known(
    state: &State,
    content: &ContentBundle,
    run: u32,
    beat: &str,
) -> Option<SceneView> {
    build(state, content, run, beat, true)
}

fn result_of(sc: &Scene<'_>, done: &ExchangePlayed) -> ResultView {
    ResultView {
        outcome: done.outcome,
        result: done.result,
        key: format!("{}-result-{}", sc.def.text, done.outcome.key()),
        groups: done
            .groups
            .iter()
            .map(|g| (g.group.key(), g.start, g.lean))
            .collect(),
        event: done.event,
    }
}

#[allow(clippy::too_many_lines)]
fn build(
    state: &State,
    content: &ContentBundle,
    run: u32,
    beat: &str,
    all_known: bool,
) -> Option<SceneView> {
    let run = state.arcs.run(ArcId(run))?;
    let bref = super::beat_ref(run, beat);
    let sc = super::scene(content, run, &bref)?;
    let p = &sc.params;
    let done = run.played.iter().rev().find(|x| x.beat == bref);
    let xs = run.exchange.as_ref().filter(|x| x.beat == bref);
    let phase = if xs.is_some() {
        Phase::UnderWay
    } else if done.is_some() || run.is_done(&bref) {
        Phase::Done
    } else if open_now(state, content, run, &sc) {
        Phase::Open
    } else {
        Phase::Ahead
    };
    let mut out = SceneView {
        run: run.id,
        beat: bref.clone(),
        phase,
        place: super::place(run, sc.def),
        time: sc.def.time,
        last_day: super::last_day(run),
        layout: sc.def.layout,
        text: sc.def.text.clone(),
        other: run.person(&sc.def.sides.other),
        chair: sc.def.sides.chair.as_deref().and_then(|r| run.person(r)),
        second: sc.def.sides.second.as_deref().and_then(|r| run.person(r)),
        groups: Vec::new(),
        round: xs.map_or(0, |x| x.round),
        rounds: sc.def.rounds(p),
        prompt: None,
        options: Vec::new(),
        locked: None,
        played: xs
            .map(|x| x.rounds.clone())
            .or_else(|| done.map(|d| d.rounds.clone()))
            .unwrap_or_default(),
        result: done.map(|d| result_of(&sc, d)),
    };
    // The audience: as it stands in a scene under way, as it ended in one
    // that is over, as it would bind now in one still ahead.
    let groups = match (xs, done) {
        (Some(x), _) => x.groups.clone(),
        (None, Some(d)) => d.groups.clone(),
        (None, None) => {
            crate::condition::on_state(state, content, |v| audience::bind(v, run, sc.def, p))
        }
    };
    let last = out.played.last().map(|r| r.deltas.clone());
    crate::condition::on_state(state, content, |v| {
        for (gi, g) in groups.iter().enumerate() {
            let level = match g.group.person() {
                Some(id) => audience::person_known(v, id),
                None => audience::worries_known(v, &g.group),
            };
            out.groups.push(GroupView {
                key: g.group.key(),
                name: audience::name(v, &g.group),
                group: g.group.clone(),
                weight: g.weight,
                lean: (all_known || level >= Level::Known).then_some(g.lean),
                moved: last.as_ref().and_then(|d| d.get(gi).copied()),
            });
        }
    });
    let Some(xs) = xs else {
        return Some(out);
    };
    let Some(prompt) = xs.prompt.as_ref() else {
        return Some(out);
    };
    let opts = moves::options(state, content, p, run, sc.arc, sc.def, xs);
    let r = crate::condition::on_state(state, content, |v| moves::round_in(v, p, run, sc.def, xs))?;
    let weights: Vec<f64> = xs.groups.iter().map(|g| f64::from(g.weight)).collect();
    let shown = moves::shown(p, &r, &weights, &opts);
    // The prompt, with what it leaves standing if nothing answers it.
    let seen_r = if all_known {
        landing::RoundIn {
            fit: r.fit.iter().map(|f| f.truth()).collect(),
            ..r.clone()
        }
    } else {
        r.clone()
    };
    let bites: Vec<Span> = (0..xs.groups.len())
        .map(|g| {
            let f = seen_r.fit[g];
            let at = |fit: f64| landing::bounded(seen_r.lean[g], -landing::bite(p, &seen_r, fit));
            let (a, b) = (at(f.lo), at(f.hi));
            Span {
                lo: a.min(b),
                hi: a.max(b),
            }
        })
        .collect();
    let speaker = match prompt.by {
        Speaker::Chair => out.chair,
        Speaker::Other => out.other,
    };
    let mut pargs: Vec<(String, String)> = super::play::text_args(state, run);
    pargs.extend(prompt_args(state, content, &prompt.attack));
    out.prompt = Some(PromptView {
        by: prompt.by,
        speaker,
        source: prompt.attack.source,
        key: format!(
            "{}-prompt-{}-{}",
            sc.def.text,
            prompt.attack.source.key(),
            prompt.variant
        ),
        args: pargs,
        event: prompt.attack.anchor,
        stood: prompt.stood,
        bites,
    });
    for i in shown {
        let o = &opts[i];
        let pre = if all_known {
            let (kr, kl) = landing::known(&r, &o.line);
            let l = landing::land(p, &kr, &kl);
            landing::Preview {
                p: l.p,
                stands: span_all(&l.stands),
                fails: span_all(&l.fails),
            }
        } else {
            landing::preview(p, &r, &o.line)
        };
        out.options.push(OptionView {
            id: o.id.clone(),
            kind: o.kind,
            label_key: format!("exchange-label-{}", o.kind.key()),
            say_key: format!("{}-say-{}", sc.def.text, o.kind.key()),
            args: o.args.clone(),
            target: o.target,
            answers: o.line.answers,
            chance: pre.p,
            fails: if pre.p >= 1.0 {
                pre.stands.clone()
            } else {
                pre.fails
            },
            stands: pre.stands,
            notes: o.notes.clone(),
            answer: if all_known && o.answer_seen == AnswerSeen::Maybe {
                if o.line.answer.value > 0.0 {
                    AnswerSeen::Yes
                } else {
                    AnswerSeen::No
                }
            } else {
                o.answer_seen
            },
            prep: o.prep,
        });
    }
    out.locked = moves::locked(state, content, run, sc.def, xs, &opts);
    Some(out)
}

/// What a prompt's sentence is given: the thing it names, as keys and
/// names (facts, never prose).
fn prompt_args(
    state: &State,
    content: &ContentBundle,
    a: &super::state::Attack,
) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let law_key = |policy: u32| {
        state
            .policy
            .instances
            .get(policy as usize)
            .and_then(|i| content.policies.get(&i.template))
            .map(|t| t.name_key.clone())
            .unwrap_or_default()
    };
    match &a.subject.what {
        What::Law { policy } | What::Bill { policy } => {
            out.push(("law_key".into(), law_key(*policy)));
        }
        What::Promise { ask } => {
            let key = state
                .asks
                .get(crate::asks::AskId(*ask))
                .and_then(|x| {
                    content
                        .promise(&x.template)
                        .map(|t| t.name_key.clone())
                        .or_else(|| content.ask(&x.template).map(|t| t.name_key.clone()))
                })
                .unwrap_or_default();
            out.push(("promise_key".into(), key));
        }
        What::Scandal { id } => {
            if let Some(s) = state.media.scandal(*id) {
                out.push(("fact_key".into(), format!("fact-{}", s.kind)));
            }
        }
        What::Hidden { id } => {
            if let Some(h) = state.narrative.hidden.iter().find(|h| h.id == *id) {
                out.push(("fact_key".into(), format!("fact-{}", h.kind)));
            }
        }
        _ => {}
    }
    if let Some(c) = &a.subject.concern
        && let Some(def) = content.concerns.iter().find(|x| &x.id == c)
    {
        out.push((
            "worry_key".into(),
            def.talk_key.clone().unwrap_or_else(|| def.name_key.clone()),
        ));
    }
    if let Some(g) = &a.group {
        // The group it names, by the area's own name where it has one.
        let name = crate::condition::on_state(state, content, |v| {
            state
                .arcs
                .runs
                .iter()
                .filter_map(|r| r.exchange.as_ref())
                .flat_map(|x| x.groups.iter())
                .find(|x| &x.group.key() == g)
                .and_then(|x| audience::name(v, &x.group))
        });
        out.push(("group".into(), name.unwrap_or_default()));
    }
    out.push((
        "size".into(),
        format!("{}", (f64::from(a.size) * 100.0).round() as i64),
    ));
    out
}

/// A stand-in's reading of the staging, for the arc sheet: whether this
/// chapter's scene is still to be held, and by whom.
pub fn staging_now(state: &State, content: &ContentBundle) -> Staging {
    super::staging(&state.arcs.runs, content, state.narrative.chapter.as_ref())
}
