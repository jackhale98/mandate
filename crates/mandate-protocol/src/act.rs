//! The act flow's views (DEC-005, DEC-012, DEC-006): Term in review (the
//! directives and the points to skip to), a skip's progress, the interrupt
//! or homecoming summary (REQ-010), and the act epilogue with its Legacy
//! record. Facts and Fluent keys only (DEC-054): the client words them and
//! computes nothing.

use crate::DateView;
use mandate_content::act::{DirectiveLever, SkipTargetKind};
use mandate_sim::Engine;
use mandate_sim::legacy::{LegacyKind, LegacyRecord};
use mandate_sim::politics::Post;
use mandate_sim::skip::{SkipProgress, SkipStop, SkipSummary, Snapshot, SummaryItem};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DirectiveOptionView {
    pub id: String,
    pub name_key: String,
}

/// A standing directive and its options (`act_flow.jsonc`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DirectiveView {
    pub id: String,
    pub name_key: String,
    pub help_key: String,
    /// The proxy lever it steers (`focus`, `vote`, `ground`, …).
    pub lever: String,
    pub options: Vec<DirectiveOptionView>,
    /// The option chosen now.
    pub chosen: String,
}

/// A point the engine offers to skip to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SkipTargetView {
    /// `next_session`, `campaign`, `election`, `end_of_term`.
    pub target: SkipTargetDto,
    pub name_key: String,
    pub date: DateView,
}

/// The points a skip can run to (the engine's contract).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum SkipTargetDto {
    NextSession,
    Campaign,
    Election,
    EndOfTerm,
}

impl SkipTargetDto {
    pub fn to_content(self) -> SkipTargetKind {
        match self {
            SkipTargetDto::NextSession => SkipTargetKind::NextSession,
            SkipTargetDto::Campaign => SkipTargetKind::Campaign,
            SkipTargetDto::Election => SkipTargetKind::Election,
            SkipTargetDto::EndOfTerm => SkipTargetKind::EndOfTerm,
        }
    }
    pub fn of(k: SkipTargetKind) -> Self {
        match k {
            SkipTargetKind::NextSession => SkipTargetDto::NextSession,
            SkipTargetKind::Campaign => SkipTargetDto::Campaign,
            SkipTargetKind::Election => SkipTargetDto::Election,
            SkipTargetKind::EndOfTerm => SkipTargetDto::EndOfTerm,
        }
    }
}

/// One directive choice the client sends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DirectiveChoiceDto {
    pub directive: String,
    pub option: String,
}

/// Where a skip has got to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SkipProgressView {
    pub target: SkipTargetDto,
    pub from: DateView,
    pub date: DateView,
    pub until: DateView,
    pub turns: u32,
    /// The share of the way, per mille (the engine's figure).
    pub progress_pm: u32,
    /// The skip is over: fetch `QueryDto::SkipSummary`.
    pub done: bool,
}

impl SkipProgressView {
    pub fn of(p: &SkipProgress) -> Self {
        SkipProgressView {
            target: SkipTargetDto::of(p.target),
            from: DateView::of(p.from),
            date: DateView::of(p.date),
            until: DateView::of(p.until),
            turns: p.turns,
            progress_pm: (p.fraction() * 1000.0).round() as u32,
            done: p.done,
        }
    }
}

/// Term in review (DEC-012): the directives and the points to skip to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TermReviewView {
    pub directives: Vec<DirectiveView>,
    pub targets: Vec<SkipTargetView>,
    /// A skip in progress (between chunks), if any.
    pub skipping: Option<SkipProgressView>,
    /// The act is over (no more skips).
    pub act_over: bool,
}

pub fn term_review_view(engine: &Engine) -> TermReviewView {
    let st = engine.state();
    let content = engine.content();
    let directives = content
        .act_flow
        .directives
        .iter()
        .map(|d| {
            let mut options: Vec<DirectiveOptionView> = d
                .options
                .iter()
                .map(|o| DirectiveOptionView {
                    id: o.id.clone(),
                    name_key: o.name_key.clone(),
                })
                .collect();
            if d.concern_options {
                options.extend(content.concerns.iter().map(|c| DirectiveOptionView {
                    id: c.id.clone(),
                    name_key: c.name_key.clone(),
                }));
            }
            DirectiveView {
                id: d.id.clone(),
                name_key: d.name_key.clone(),
                help_key: d.help_key.clone(),
                lever: lever_key(d.lever).into(),
                options,
                chosen: st.act.directives.chosen(d).to_string(),
            }
        })
        .collect();
    let targets = mandate_sim::skip::targets(st, content)
        .into_iter()
        .map(|t| SkipTargetView {
            target: SkipTargetDto::of(t.target),
            name_key: t.name_key,
            date: DateView::of(t.until),
        })
        .collect();
    let skipping = st
        .act
        .skip
        .as_ref()
        .map(|r| SkipProgressView::of(&SkipProgress::of(r, st.calendar.date, false)));
    TermReviewView {
        directives,
        targets,
        skipping,
        act_over: st.act.over(),
    }
}

fn lever_key(l: DirectiveLever) -> &'static str {
    l.key()
}

/// A figure before and after a skip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ChangeView {
    /// `mandate`, `standing`, `viability`, `legal_jeopardy`, `health`,
    /// `money`, `population`, `unemployment` (per mille), `laws`.
    pub figure: String,
    pub label_key: String,
    #[ts(type = "number")]
    pub before: i64,
    #[ts(type = "number")]
    pub after: i64,
}

/// A line of a summary or a record: a Fluent key and its arguments.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ActLineView {
    pub date: Option<DateView>,
    pub text_key: String,
    pub args: BTreeMap<String, String>,
    /// A decision this line is about (to open it).
    pub decision: Option<u32>,
}

/// Why a skip stopped.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SkipStopView {
    /// `reached`, `interrupted`, `cancelled`, `act_ended`, `limit`.
    pub kind: String,
    /// The interrupt's Fluent key, or `ui-skip-stop-<kind>`.
    pub text_key: String,
    pub args: BTreeMap<String, String>,
    /// The interrupt's content id.
    pub interrupt: Option<String>,
    /// A decision that needs the player (open it).
    pub decision: Option<u32>,
}

/// The interrupt or homecoming summary (REQ-010): what happened while the
/// player was away, as facts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SkipSummaryView {
    pub target: SkipTargetDto,
    pub target_key: String,
    pub from: DateView,
    pub to: DateView,
    pub turns: u32,
    pub stop: SkipStopView,
    pub changes: Vec<ChangeView>,
    pub items: Vec<ActLineView>,
}

fn post_key(engine: &Engine, post: Post) -> String {
    engine
        .content()
        .constitutions
        .get(&engine.state().meta.constitution)
        .and_then(|c| post.def(c))
        .map(|d| d.name_key().to_string())
        .unwrap_or_default()
}

fn policy_key(engine: &Engine, policy: u32) -> String {
    let st = engine.state();
    st.policy
        .instances
        .get(policy as usize)
        .and_then(|p| mandate_sim::policy::resolve(engine.content(), &engine.state().policy, p))
        .map(|t| t.name_key.clone())
        .unwrap_or_default()
}

/// A decision's title and its option's label as arguments.
fn decision_args(
    engine: &Engine,
    decision: u32,
    option: Option<&str>,
    args: &mut BTreeMap<String, String>,
) {
    let st = engine.state();
    let Some(d) = st
        .narrative
        .decision(mandate_sim::narrative::DecisionId(decision))
    else {
        return;
    };
    args.extend(crate::cast_args(engine, &d.cast));
    if let Some(def) = engine.content().dilemma(&d.template) {
        args.insert("title_key".into(), def.title_key.clone());
        if let Some(o) = option.and_then(|o| def.options.iter().find(|x| x.id == o)) {
            args.insert("option_key".into(), o.label_key.clone());
        }
    }
}

fn changes(a: &Snapshot, b: &Snapshot) -> Vec<ChangeView> {
    let r = |x: f32| x.round() as i64;
    [
        ("mandate", r(a.mandate), r(b.mandate)),
        ("standing", r(a.standing), r(b.standing)),
        ("viability", r(a.viability), r(b.viability)),
        ("legal_jeopardy", r(a.legal_jeopardy), r(b.legal_jeopardy)),
        ("health", r(a.health), r(b.health)),
        ("money", a.money, b.money),
        ("population", a.population as i64, b.population as i64),
        (
            "unemployment",
            (a.unemployment * 1000.0).round() as i64,
            (b.unemployment * 1000.0).round() as i64,
        ),
        ("laws", a.laws as i64, b.laws as i64),
    ]
    .into_iter()
    .map(|(f, x, y)| ChangeView {
        figure: f.into(),
        label_key: format!("ui-skip-change-{f}"),
        before: x,
        after: y,
    })
    .collect()
}

/// An ask's title and its arguments (`ask_key`); whether it lapsed
/// unanswered.
fn ask_args(
    engine: &Engine,
    ask: u32,
    template: &str,
    args: &mut BTreeMap<String, String>,
) -> bool {
    let content = engine.content();
    let title = content
        .ask(template)
        .map(|t| t.name_key.clone())
        .or_else(|| content.promise(template).map(|t| t.name_key.clone()))
        .unwrap_or_default();
    let found = engine
        .state()
        .asks
        .get(mandate_sim::asks::AskId(ask))
        .filter(|a| a.template == template);
    if let Some(a) = found {
        let mut a_args = crate::narrative::ask_args(engine, a);
        // The days left mean nothing once it is over.
        a_args.remove("days");
        args.extend(a_args);
    }
    args.insert("ask_key".into(), title);
    found.is_some_and(|a| {
        matches!(
            a.status,
            mandate_sim::asks::AskStatus::Declined {
                unanswered: true,
                ..
            }
        )
    })
}

fn item_line(engine: &Engine, it: &SummaryItem) -> ActLineView {
    let mut args = BTreeMap::new();
    let mut decision = None;
    let key = match it {
        SummaryItem::Election { post, won } => {
            args.insert("post_key".into(), post_key(engine, *post));
            if *won {
                "skip-item-election-won"
            } else {
                "skip-item-election-lost"
            }
        }
        SummaryItem::LawEnacted { policy, mine } => {
            args.insert("policy_key".into(), policy_key(engine, *policy));
            if *mine {
                "skip-item-law-mine"
            } else {
                "skip-item-law"
            }
        }
        SummaryItem::BillFailed { policy } => {
            args.insert("policy_key".into(), policy_key(engine, *policy));
            "skip-item-bill-failed"
        }
        SummaryItem::Answered {
            decision: d,
            option,
            ..
        } => {
            decision_args(engine, *d, Some(option), &mut args);
            decision = Some(*d);
            "skip-item-answered"
        }
        SummaryItem::Defaulted { decision: d, .. } => {
            decision_args(engine, *d, None, &mut args);
            decision = Some(*d);
            "skip-item-defaulted"
        }
        SummaryItem::Scandal { kind, .. } => {
            args.insert("kind_key".into(), format!("fact-{kind}"));
            "skip-item-scandal"
        }
        SummaryItem::Chapter { index, kind } => {
            args.insert("index".into(), index.to_string());
            args.insert("kind_key".into(), format!("chapter-kind-{kind}"));
            "skip-item-chapter"
        }
        SummaryItem::Disaster { hazard } => {
            args.insert("hazard".into(), hazard.clone());
            "skip-item-disaster"
        }
        SummaryItem::PlantClosure { sector, jobs } => {
            args.insert("jobs".into(), jobs.to_string());
            if let Some(s) = engine.content().sector(sector) {
                args.insert("sector_key".into(), s.name_key.clone());
            }
            "skip-item-plant-closure"
        }
        SummaryItem::Voted { policy, vote } => {
            args.insert("policy_key".into(), policy_key(engine, *policy));
            args.insert("vote".into(), vote.clone());
            "skip-item-voted"
        }
        SummaryItem::LawEnded { policy, reason } => {
            args.insert("policy_key".into(), policy_key(engine, *policy));
            if reason == "repealed" {
                "skip-item-law-repealed"
            } else {
                "skip-item-law-expired"
            }
        }
        SummaryItem::BillWithdrawn { policy } => {
            args.insert("policy_key".into(), policy_key(engine, *policy));
            "skip-item-bill-withdrawn"
        }
        SummaryItem::AskOffered { ask, template } => {
            ask_args(engine, *ask, template, &mut args);
            "skip-item-ask-offered"
        }
        SummaryItem::AskResolved {
            ask,
            template,
            outcome,
        } => {
            let lapsed = ask_args(engine, *ask, template, &mut args);
            match outcome.as_str() {
                "kept" => "skip-item-ask-kept",
                "broken" => "skip-item-ask-broken",
                "void" => "skip-item-ask-void",
                _ if lapsed => "skip-item-ask-lapsed",
                _ => "skip-item-ask-declined",
            }
        }
        SummaryItem::Crisis { kind, began } => {
            if let Some(c) = engine.content().crises.iter().find(|c| &c.id == kind) {
                args.insert("crisis_key".into(), c.name_key.clone());
            }
            if *began {
                "skip-item-crisis-began"
            } else {
                "skip-item-crisis-ended"
            }
        }
        SummaryItem::Deselected => "skip-item-deselected",
        SummaryItem::Case { stage, .. } => {
            args.insert("stage_key".into(), format!("case-stage-{stage}"));
            "skip-item-case"
        }
        SummaryItem::Imprisoned { years } => {
            args.insert("n".into(), years.to_string());
            "skip-item-imprisoned"
        }
        SummaryItem::Released => "skip-item-released",
        SummaryItem::Rumour { .. } => "skip-item-rumour",
        SummaryItem::Recession { began } => {
            if *began {
                "skip-item-recession-began"
            } else {
                "skip-item-recession-ended"
            }
        }
    };
    ActLineView {
        date: None,
        text_key: key.into(),
        args,
        decision,
    }
}

fn stop_view(stop: &SkipStop, target_key: &str) -> SkipStopView {
    let mut args = BTreeMap::new();
    let (kind, key, interrupt, decision) = match stop {
        SkipStop::Reached => {
            args.insert("target_key".into(), target_key.to_string());
            ("reached", "ui-skip-stop-reached".to_string(), None, None)
        }
        SkipStop::Interrupted {
            interrupt,
            key,
            args: a,
        } => {
            let mut decision = None;
            for (k, v) in a {
                if k == "decision" {
                    decision = v.parse().ok();
                }
                args.insert(k.clone(), v.clone());
            }
            (
                "interrupted",
                key.clone(),
                Some(interrupt.clone()),
                decision,
            )
        }
        SkipStop::Cancelled => ("cancelled", "ui-skip-stop-cancelled".into(), None, None),
        SkipStop::ActEnded { ending } => {
            args.insert("ending".into(), ending.clone());
            ("act_ended", "ui-skip-stop-act_ended".into(), None, None)
        }
        SkipStop::Limit => ("limit", "ui-skip-stop-limit".into(), None, None),
    };
    SkipStopView {
        kind: kind.into(),
        text_key: key,
        args,
        interrupt,
        decision,
    }
}

fn summary_view(engine: &Engine, s: &SkipSummary) -> SkipSummaryView {
    let target_key = engine
        .content()
        .act_flow
        .targets
        .iter()
        .find(|t| t.target == s.target)
        .map(|t| t.name_key.clone())
        .unwrap_or_default();
    let mut items: Vec<ActLineView> = s.items.iter().map(|i| item_line(engine, i)).collect();
    if s.start.executive != s.end.executive
        && let Some(p) = s
            .end
            .executive
            .and_then(|id| engine.state().politics.get(id))
    {
        let mut args = BTreeMap::new();
        args.insert("name".into(), p.full_name());
        items.push(ActLineView {
            date: None,
            text_key: "ui-skip-change-executive".into(),
            args,
            decision: None,
        });
    }
    SkipSummaryView {
        target: SkipTargetDto::of(s.target),
        stop: stop_view(&s.stop, &target_key),
        target_key,
        from: DateView::of(s.start.date),
        to: DateView::of(s.end.date),
        turns: s.turns,
        changes: changes(&s.start, &s.end),
        items,
    }
}

/// The last finished skip's summary, if any.
pub fn skip_summary_view(engine: &Engine) -> Option<SkipSummaryView> {
    engine
        .state()
        .act
        .last_skip
        .as_ref()
        .map(|s| summary_view(engine, s))
}

/// One fact of an epilogue.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EpilogueFactView {
    pub fact: String,
    pub label_key: String,
    #[ts(type = "number")]
    pub value: i64,
}

/// How the act ended.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EndingView {
    pub id: String,
    pub title_key: String,
    pub text_key: String,
    /// `triumph`, `neutral`, `defeat` (presentation).
    pub tone: String,
    pub date: DateView,
    pub facts: Vec<EpilogueFactView>,
    /// The facts as the text's arguments.
    pub args: BTreeMap<String, String>,
}

/// The tracks at the close.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FinalStandingView {
    #[ts(type = "number")]
    pub mandate: i64,
    #[ts(type = "number")]
    pub standing: i64,
    #[ts(type = "number")]
    pub viability: i64,
    #[ts(type = "number")]
    pub legal_jeopardy: i64,
    #[ts(type = "number")]
    pub health: i64,
    #[ts(type = "number")]
    pub money: i64,
    /// Fluent name keys of the posts held at the close.
    pub post_keys: Vec<String>,
}

/// The act's Legacy record (DEC-006, DEC-100) and, once it is over, its
/// epilogue.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LegacyView {
    /// The act's constitution id.
    pub act: String,
    pub began: DateView,
    pub closed: Option<DateView>,
    pub ending: Option<EndingView>,
    pub final_standing: Option<FinalStandingView>,
    /// Newest first.
    pub entries: Vec<ActLineView>,
}

fn person_name(engine: &Engine, person: u32) -> String {
    crate::person_ref(engine, mandate_sim::characters::CharacterId(person))
        .map(|p| p.name)
        .unwrap_or_default()
}

fn entry_line(engine: &Engine, date: mandate_core::SimDate, k: &LegacyKind) -> ActLineView {
    let mut args = BTreeMap::new();
    let mut decision = None;
    let key = match k {
        LegacyKind::PostTaken { post } => {
            args.insert("post_key".into(), post_key(engine, *post));
            "legacy-post_taken"
        }
        LegacyKind::PostLeft { post } => {
            args.insert("post_key".into(), post_key(engine, *post));
            "legacy-post_left"
        }
        LegacyKind::Election { post, won, share } => {
            args.insert("post_key".into(), post_key(engine, *post));
            args.insert("share".into(), format!("{:.0}%", share * 100.0));
            if *won {
                "legacy-election-won"
            } else {
                "legacy-election-lost"
            }
        }
        LegacyKind::LawPassed { policy, .. } => {
            args.insert("policy_key".into(), policy_key(engine, *policy));
            "legacy-law_passed"
        }
        LegacyKind::BillFailed { policy, .. } => {
            args.insert("policy_key".into(), policy_key(engine, *policy));
            "legacy-bill_failed"
        }
        LegacyKind::Decision {
            decision: d,
            option,
            template,
        } => {
            decision_args(engine, *d, Some(option), &mut args);
            if !args.contains_key("title_key")
                && let Some(def) = engine.content().dilemma(template)
            {
                args.insert("title_key".into(), def.title_key.clone());
            }
            decision = Some(*d);
            "legacy-decision"
        }
        LegacyKind::PromiseKept { person } => {
            args.insert("name".into(), person_name(engine, *person));
            "legacy-promise_kept"
        }
        LegacyKind::PromiseBroken { person } => {
            args.insert("name".into(), person_name(engine, *person));
            "legacy-promise_broken"
        }
        LegacyKind::CampaignPromise { template, kept } => {
            if let Some(t) = engine.content().promise(template) {
                args.insert("promise_key".into(), t.name_key.clone());
            }
            if *kept {
                "legacy-campaign_promise-kept"
            } else {
                "legacy-campaign_promise-broken"
            }
        }
        LegacyKind::Scandal { kind, .. } => {
            args.insert("kind_key".into(), format!("fact-{kind}"));
            "legacy-scandal"
        }
        LegacyKind::Case { stage, .. } => {
            args.insert("stage".into(), stage.clone());
            "legacy-case"
        }
        LegacyKind::RemovedFromOffice { .. } => "legacy-removed",
        LegacyKind::Deselected => "legacy-deselected",
        LegacyKind::Imprisoned { years } => {
            args.insert("years".into(), years.to_string());
            "legacy-imprisoned"
        }
        LegacyKind::Released => "legacy-released",
    };
    ActLineView {
        date: Some(DateView::of(date)),
        text_key: key.into(),
        args,
        decision,
    }
}

fn record_view(engine: &Engine, rec: &LegacyRecord) -> LegacyView {
    let ending = rec.ending.as_ref().map(|e| EndingView {
        id: e.id.clone(),
        title_key: e.title_key.clone(),
        text_key: e.text_key.clone(),
        tone: e.tone.clone(),
        date: DateView::of(rec.closed.unwrap_or(engine.state().calendar.date)),
        facts: e
            .facts
            .iter()
            .map(|(k, v)| EpilogueFactView {
                fact: k.clone(),
                label_key: format!("epilogue-fact-{k}"),
                value: *v,
            })
            .collect(),
        args: e
            .facts
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect(),
    });
    let final_standing = rec.final_standing.as_ref().map(|f| FinalStandingView {
        mandate: f.mandate.round() as i64,
        standing: f.standing.round() as i64,
        viability: f.viability.round() as i64,
        legal_jeopardy: f.legal_jeopardy.round() as i64,
        health: f.health.round() as i64,
        money: f.money,
        post_keys: f.posts.iter().map(|p| post_key(engine, *p)).collect(),
    });
    LegacyView {
        act: rec.act.clone(),
        began: DateView::of(rec.began),
        closed: rec.closed.map(DateView::of),
        ending,
        final_standing,
        entries: rec
            .entries
            .iter()
            .rev()
            .map(|e| entry_line(engine, e.date, &e.kind))
            .collect(),
    }
}

/// The current act's Legacy record (and epilogue once it is over).
pub fn legacy_view(engine: &Engine) -> Option<LegacyView> {
    engine.state().act.current().map(|r| record_view(engine, r))
}

/// The act's state on the overview: over, how, and a skip in progress.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ActStatusView {
    /// The ending's title key once the act is over.
    pub ended_key: Option<String>,
    pub skipping: bool,
}

pub fn act_status(engine: &Engine) -> ActStatusView {
    let st = engine.state();
    ActStatusView {
        ended_key: st
            .act
            .current()
            .and_then(|r| r.ending.as_ref())
            .map(|e| e.title_key.clone())
            .filter(|_| st.act.over()),
        skipping: st.act.skip.is_some(),
    }
}
