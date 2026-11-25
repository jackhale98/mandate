//! Arcs as the player sees them (DEC-109, task A3; `STAGED_ARCS.md` §2, §5;
//! `POCKET_REWORK.md` §3.9): one view of an arc run ([`ArcView`]), its row
//! on Today ([`ArcSummaryView`]) and, below `known`, a lead and nothing
//! else ([`ArcLeadView`]).
//!
//! **Generic over arcs and tiers (DEC-098).** Nothing here names an arc, an
//! office or a body: titles, stages, buttons and lines are the template's
//! keys, the cast is the run's, a power is read from a beat's own
//! conditions, and a run is keyed by its jurisdiction.
//!
//! **What the player knows of an arc** ([`arc_level`]) is derived, never
//! stored (no arc subject is in the ledger, so nothing is saved for it):
//!
//! | Level | The player sees | It comes from |
//! |---|---|---|
//! | unknown | Nothing: the view is refused as if no such run existed | a hidden stage, and no part taken in it |
//! | heard | A lead: the matter exists, and where | a beat taken in a hidden stage, or a line overheard, that teaches `heard` (the default) |
//! | known | The title, the stage, the clock, the cast, what can be done, what has happened | a stage that is not hidden (the break never is, DEC-006); a decision or an ask of the run (aimed at the player: never gated); a beat that teaches `known` |
//! | detailed | Also what happened in the hidden stages | a beat that teaches `detailed`; the lab switch |
//!
//! The level gates this view and nothing else: no edge, raise or move reads
//! it (DEC-091: the world does not wait). Views never write.
//!
//! **Text (DEC-054).** Facts and keys only: the template's own keys with
//! the cast by role and the arc's readings as arguments; the clock and the
//! lead are engine lines (`now-clock-*`, `know-arc-lead`).

use crate::characters::{PersonRefView, person_ref};
use crate::knowledge::KnowledgeLevelDto;
use crate::places::{LocationView, RouteView, location_view};
use crate::pocket::{TextView, clock_text, route_to};
use crate::post::{JurisdictionDto, jurisdiction_dto};
use crate::views::{DateView, EventView, event_view};
use mandate_content::Condition;
use mandate_content::arcs::{ArcBeat, ArcDef, BeatKind};
use mandate_content::visits::VisitTime;
use mandate_core::SimDate;
use mandate_sim::Engine;
use mandate_sim::arcs::beats::{self, BeatAction};
use mandate_sim::arcs::{ArcActor, ArcCastEntry, ArcId, ArcRun, BeatRef, cast};
use mandate_sim::events::EventKind;
use mandate_sim::narrative::{Actor, CastEntry};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Level, Source};
use mandate_sim::spine::Cause;
use mandate_sim::world::{Area, Jurisdiction};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// A lead toward an arc the player has only heard of: that something is
/// going on, and where (never who or what).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArcLeadView {
    /// The run (`QueryDto::Arc`).
    pub arc: u32,
    /// `know-arc-lead`, with `where` (a neighbourhood's or a place's name,
    /// else the jurisdiction's; `where_key` where a Fluent key names it).
    pub text: TextView,
    /// Where to go and ask, if that is somewhere in town.
    pub to: Option<LocationView>,
    pub route: Option<RouteView>,
    pub since: DateView,
}

/// A deadline in a sentence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArcClockView {
    /// `now-clock-today`, `now-clock-tomorrow` or `now-clock-days` (`$days`).
    pub text: TextView,
    pub due: DateView,
    pub days_left: u32,
}

/// Where an arc stands: its current stage's sentence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArcStageView {
    /// The stage's id in its template (stable; for tests and keys).
    pub id: String,
    /// The story so far: the stage's own line, with the cast and the
    /// arc's readings as they are now.
    pub text: TextView,
    pub since: DateView,
    /// The event of the stage being entered, to ask "Why?" about
    /// (`QueryDto::Why`), while the log keeps it.
    pub event: Option<u32>,
}

/// The turn of an arc: reached, or still ahead.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArcTurnView {
    /// `ahead` | `reached`.
    pub state: String,
    /// The day it was reached; ahead, the clock's day when the stage's
    /// clock leads to it, else none.
    pub on: Option<DateView>,
}

/// One of an arc's cast, by role.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArcCastView {
    /// The role's id in the template (the argument name in its lines).
    pub role: String,
    /// `person` | `area` | `place` | `jurisdiction`.
    pub kind: String,
    pub name: String,
    /// A person, named as the arc names them (its lines do), with what the
    /// player knows of them (`level`) and whether they can be spoken to.
    pub person: Option<PersonRefView>,
    /// A place, or a neighbourhood's streets.
    pub at: Option<LocationView>,
}

/// What taking a beat needs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArcBeatNeedsView {
    /// Where it is done.
    pub place: Option<LocationView>,
    /// Whom it is done with.
    pub person: Option<PersonRefView>,
    /// A power the beat is offered to (a constitution power's id), or
    /// `office` for any post: read from the beat's own conditions.
    pub power: Option<String>,
    /// Time in half slots (0: free).
    pub halves: u32,
}

/// A beat that is the player's to take now, as the command that takes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArcBeatView {
    pub stage: String,
    pub beat: String,
    /// `visit` (`ActionDto::Visit { verb }` at the place), `talk`
    /// (`CommandDto::Talk { to, topic }`), `decision`
    /// (`CommandDto::AnswerDecision`), `ask` (`CommandDto::RespondAsk`,
    /// then whatever keeps it) or `exchange` (a scene: `QueryDto::ArcExchange`
    /// with `stage` and `beat`, begun with `CommandDto::ArcExchange`).
    pub kind: String,
    /// The button: the beat's own label.
    pub label: TextView,
    pub needs: ArcBeatNeedsView,
    /// Auto-routing to where it is done (DEC-079), for a visit or a talk.
    pub route: Option<RouteView>,
    pub verb: Option<String>,
    pub topic: Option<String>,
    pub decision: Option<u32>,
    pub ask: Option<u32>,
    /// It can be done this turn (the time is there, the person can be found).
    pub available: bool,
    /// Why not, as a Fluent key (`visit-blocked-…`), when it cannot.
    pub blocked_key: Option<String>,
    /// Its own deadline (a decision's, an ask's), else the arc's clock.
    pub due: Option<DateView>,
    pub days_left: Option<u32>,
}

/// How an arc ended.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArcEndView {
    /// The exit's own line (how it ended).
    pub text: Option<TextView>,
    pub on: DateView,
}

/// One arc run as the player sees it (`QueryDto::Arc`). Below `known`
/// only `id`, `level` and `lead` are filled.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArcView {
    pub id: u32,
    /// What the player knows of it (derived; for tests, never drawn).
    pub level: KnowledgeLevelDto,
    /// Below `known`: where to ask. From `known` on, none.
    pub lead: Option<ArcLeadView>,
    pub title: Option<TextView>,
    /// Where it runs.
    pub at: Option<JurisdictionDto>,
    /// When the matter began, and for how many days it has run.
    pub began: Option<DateView>,
    pub days: Option<u32>,
    pub stage: Option<ArcStageView>,
    pub clock: Option<ArcClockView>,
    pub turn: Option<ArcTurnView>,
    pub cast: Vec<ArcCastView>,
    /// What the player can do in it now, in the engine's order: what has a
    /// deadline first, soonest first.
    pub beats: Vec<ArcBeatView>,
    /// What has happened in it so far, newest first: the stages reached
    /// and the beats done, as the log keeps them.
    pub history: Vec<EventView>,
    /// The latest story the outlets ran about it.
    pub headline: Option<EventView>,
    /// Once it has ended.
    pub ended: Option<ArcEndView>,
}

/// A running arc as a row (Today's "Under way"): from `known` on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArcSummaryView {
    pub id: u32,
    pub title_key: String,
    pub args: BTreeMap<String, String>,
    /// Where it stands: the current stage's line.
    pub stage: TextView,
    /// Where it runs (its jurisdiction's name).
    pub place: String,
    pub at: JurisdictionDto,
    pub clock: Option<ArcClockView>,
    /// How many beats are the player's to take now.
    pub open_beats: u32,
}

// --- What the player knows of a run --------------------------------------------------------

fn beat_def<'c>(def: &'c ArcDef, stage: &str, beat: &str) -> Option<&'c ArcBeat> {
    def.stage(stage)?.beat(beat)
}

/// What a beat the player took teaches about its arc: its own `teaches`,
/// else that the matter exists (`heard`); a decision or an ask was aimed at
/// them (`known`). A conversation teaches what the reply said teaches (the
/// most any reply that agreed, or refused, as this one did, could: which of
/// them was said is not kept), and the beat's own once they agreed.
/// `agreed`: how a talk went (`None`: the beat is on the run's done list,
/// so they agreed).
fn taught(beat: &ArcBeat, agreed: Option<bool>) -> Level {
    match &beat.kind {
        // A decision, an ask or a scene (DEC-112) is aimed at the player.
        BeatKind::Decision { .. } | BeatKind::Ask { .. } | BeatKind::Exchange(_) => Level::Known,
        BeatKind::Visit { .. } => beat.teaches.unwrap_or(Level::Heard),
        BeatKind::Talk { choice, .. } => {
            let agreed = agreed.unwrap_or(true);
            choice
                .replies
                .iter()
                .filter(|r| r.agrees == agreed)
                .filter_map(|r| r.teaches)
                .chain(beat.teaches.filter(|_| agreed))
                .max()
                .unwrap_or(Level::Heard)
        }
        BeatKind::Press { .. } | BeatKind::Heard { .. } | BeatKind::Move { .. } => Level::Unknown,
    }
}

/// What the player knows of a run (DEC-091; `STAGED_ARCS.md` §2), derived
/// from where the run stands and the part the player took in it. `detailed`
/// without a player or with the lab switch on, as every gate reads.
pub fn arc_level(engine: &Engine, run: &ArcRun, def: &ArcDef) -> Level {
    let st = engine.state();
    let Some(pl) = st.player.as_ref() else {
        return Level::Detailed;
    };
    if pl.knowledge.lab_omniscient {
        return Level::Detailed;
    }
    let mut level = Level::Unknown;
    // A stage that is not hidden is in the open (the break never is hidden).
    if def.stage(&run.stage).is_some_and(|s| !s.hidden) {
        level = Level::Known;
    }
    // Aimed at the player: never gated.
    let mine = |a: &Option<mandate_sim::arcs::ArcRef>| a.as_ref().is_some_and(|x| x.run == run.id);
    if st.narrative.pending().any(|d| mine(&d.arc)) || st.asks.open().any(|a| mine(&a.arc)) {
        level = level.max(Level::Known);
    }
    // The part the player took: beats done (kept on the run), and beats
    // tried (in the log while it keeps them).
    for d in &run.done {
        if let Some(b) = beat_def(def, &d.beat.stage, &d.beat.beat) {
            level = level.max(taught(b, None));
        }
    }
    for e in st.events.iter() {
        match &e.kind {
            EventKind::ArcStage { arc, stage, .. } if *arc == run.id.0 => {
                if def.stage(stage).is_some_and(|s| !s.hidden) {
                    level = level.max(Level::Known);
                }
            }
            EventKind::ArcBeat {
                arc,
                stage,
                beat,
                ok,
                ..
            } if *arc == run.id.0 && e.causes.iter().any(|c| matches!(c, Cause::Player(_))) => {
                if let Some(b) = beat_def(def, stage, beat) {
                    level = level.max(taught(b, Some(*ok)));
                }
            }
            _ => {}
        }
    }
    // A line of the arc overheard since it began.
    let lines: Vec<(&str, Level)> = def
        .beats()
        .filter_map(|(_, b)| match &b.kind {
            BeatKind::Heard { line } => Some((line.as_str(), b.teaches.unwrap_or(Level::Heard))),
            _ => None,
        })
        .collect();
    if !lines.is_empty() {
        for e in pl.knowledge.entries.values() {
            for l in &e.learned {
                if let Source::Overheard { line, .. } = &l.source
                    && l.on >= run.started
                    && let Some((_, t)) = lines.iter().find(|(id, _)| id == line)
                {
                    level = level.max(*t);
                }
            }
        }
    }
    level
}

// --- Text ---------------------------------------------------------------------------------------

/// The people of a cast, as the narrative's casts name them.
fn people_of(c: &[ArcCastEntry]) -> Vec<CastEntry> {
    c.iter()
        .filter_map(|x| match x.actor {
            ArcActor::Person { id } => Some(CastEntry {
                role: x.role.clone(),
                actor: Actor::Person { id },
            }),
            _ => None,
        })
        .collect()
}

/// The arguments of an arc's lines: its cast by role (a person with their
/// role word, `<role>_role_key`), where it runs (`place`), the player's
/// home (`town`) and the given readings. An arc the player knows names its
/// cast, as its stages' lines do wherever they are told.
fn text_args(
    engine: &Engine,
    c: &[ArcCastEntry],
    at: Jurisdiction,
    facts: Vec<(String, String)>,
) -> BTreeMap<String, String> {
    let st = engine.state();
    let mut args: BTreeMap<String, String> = facts.into_iter().collect();
    args.insert("place".into(), cast::jurisdiction_name(&st.world, at));
    for (role, name) in cast::names(&st.politics, &st.world, c) {
        args.insert(role, name);
    }
    for (k, v) in crate::narrative::cast_args(engine, &people_of(c)) {
        args.insert(k, v);
    }
    args
}

/// A live run's arguments now: its dates and its template's readings.
fn run_args(engine: &Engine, run: &ArcRun, def: &ArcDef) -> BTreeMap<String, String> {
    let facts = mandate_sim::condition::on_state(engine.state(), engine.content(), |v| {
        cast::facts(v, def, run)
    });
    text_args(engine, &run.cast, run.at, facts)
}

/// A live run's arguments, looked up by its template (a scene's view).
pub(crate) fn live_args(engine: &Engine, run: &ArcRun) -> BTreeMap<String, String> {
    match engine.content().arc(&run.template) {
        Some(def) => run_args(engine, run, def),
        None => text_args(engine, &run.cast, run.at, Vec::new()),
    }
}

/// A run's title, with its arguments.
pub(crate) fn title(engine: &Engine, run: &ArcRun, def: &ArcDef) -> TextView {
    TextView::of(def.title_key.clone(), run_args(engine, run, def))
}

/// The label of a beat the player takes: a visit's button, a talk's
/// choice; a decision's and an ask's own titles are their views'.
pub(crate) fn beat_label(
    engine: &Engine,
    run: &ArcRun,
    def: &ArcDef,
    b: &BeatRef,
) -> Option<TextView> {
    let beat = beat_def(def, &b.stage, &b.beat)?;
    let key = match &beat.kind {
        BeatKind::Visit { verb_key, .. } => verb_key.clone(),
        BeatKind::Talk { choice, .. } => choice.label_key.clone(),
        _ => return None,
    };
    Some(TextView::of(key, run_args(engine, run, def)))
}

fn clock_view(today: SimDate, due: SimDate) -> ArcClockView {
    ArcClockView {
        text: clock_text(today, due),
        due: DateView::of(due),
        days_left: today.days_until(due),
    }
}

// --- The lead ------------------------------------------------------------------------------------

/// Where a run is, for someone who has only heard of it: the streets of
/// its first neighbourhood, else its first place, else its jurisdiction.
fn lead_view(engine: &Engine, run: &ArcRun) -> ArcLeadView {
    let st = engine.state();
    let mut text = TextView::new(crate::notebook::KnowLine::ArcLead.key());
    let hood = run.cast.iter().find_map(|c| match c.actor {
        ArcActor::Area {
            area: Area::Hood { id },
        } => Some(id),
        _ => None,
    });
    let place = run.cast.iter().find_map(|c| match c.actor {
        ArcActor::Place { id } => Some(id),
        _ => None,
    });
    let loc = match (hood, place) {
        (Some(h), _) => {
            text.args.insert(
                "where".into(),
                cast::area_name(&st.world, Area::Hood { id: h }),
            );
            if let Some((key, extra)) = crate::talk::hood_centre_label(st, h) {
                text.args.insert("where_key".into(), key);
                text.args.extend(extra);
            }
            Some(Location::Street { neighborhood: h })
        }
        (None, Some(p)) => {
            text.args
                .insert("where".into(), cast::place_name(&st.world, p));
            Some(Location::Place { place: p })
        }
        (None, None) => {
            text.args
                .insert("where".into(), cast::jurisdiction_name(&st.world, run.at));
            None
        }
    };
    let route = loc.and_then(|l| route_to(engine, "go", l));
    ArcLeadView {
        arc: run.id.0,
        text,
        to: route.as_ref().and_then(|r| r.to.clone()),
        route,
        since: DateView::of(run.started),
    }
}

/// Leads toward the running arcs the player has only heard of, oldest
/// first (the notebook's loose ends).
pub fn arc_leads(engine: &Engine) -> Vec<ArcLeadView> {
    let st = engine.state();
    let content = engine.content();
    if st.player.is_none() {
        return Vec::new();
    }
    st.arcs
        .runs
        .iter()
        .filter(|run| {
            content
                .arc(&run.template)
                .is_some_and(|def| arc_level(engine, run, def) == Level::Heard)
        })
        .map(|run| lead_view(engine, run))
        .collect()
}

// --- The view ------------------------------------------------------------------------------------

fn cast_view(engine: &Engine, c: &[ArcCastEntry]) -> Vec<ArcCastView> {
    let st = engine.state();
    c.iter()
        .map(|x| {
            let (kind, person, at) = match x.actor {
                ArcActor::Person { id } => ("person", person_ref(engine, id), None),
                ArcActor::Area { area } => (
                    "area",
                    None,
                    match area {
                        Area::Hood { id } => Some(Location::Street { neighborhood: id }),
                        _ => None,
                    },
                ),
                ArcActor::Place { id } => ("place", None, Some(Location::Place { place: id })),
                ArcActor::Jurisdiction { .. } => ("jurisdiction", None, None),
            };
            let name = cast::names(&st.politics, &st.world, std::slice::from_ref(x))
                .into_iter()
                .next()
                .map(|n| n.1)
                .unwrap_or_default();
            ArcCastView {
                role: x.role.clone(),
                kind: kind.into(),
                name,
                person,
                at: at
                    .filter(|l| mandate_sim::places::validate_move(st, *l).is_ok())
                    .map(|l| location_view(st, l)),
            }
        })
        .collect()
}

/// The power a beat is offered to, read from its own conditions: a
/// constitution power's id, or `office` for any post.
fn power_of(beat: &ArcBeat) -> Option<String> {
    beat.when.iter().find_map(|c| match c {
        Condition::HoldsPower { power } => serde_json::to_value(power)
            .ok()
            .and_then(|v| v.as_str().map(String::from)),
        Condition::Office { .. } => Some("office".into()),
        _ => None,
    })
}

fn halves(t: VisitTime) -> u32 {
    match t {
        VisitTime::Half => 1,
        VisitTime::Slot => 2,
    }
}

/// The beats of a run that are the player's to take now.
fn beats_view(engine: &Engine, run: &ArcRun, def: &ArcDef) -> Vec<ArcBeatView> {
    let st = engine.state();
    let content = engine.content();
    let today = st.calendar.date;
    let args = run_args(engine, run, def);
    let mut out: Vec<(u32, usize, ArcBeatView)> = Vec::new();
    for (i, o) in beats::offers(st, content)
        .into_iter()
        .filter(|o| o.run == run.id)
        .enumerate()
    {
        let Some(beat) = beat_def(def, &o.beat.stage, &o.beat.beat) else {
            continue;
        };
        let mut v = ArcBeatView {
            stage: o.beat.stage.clone(),
            beat: o.beat.beat.clone(),
            kind: beat.kind.key().into(),
            label: TextView::default(),
            needs: ArcBeatNeedsView {
                place: None,
                person: None,
                power: power_of(beat),
                halves: 0,
            },
            route: None,
            verb: None,
            topic: None,
            decision: None,
            ask: None,
            available: true,
            blocked_key: None,
            due: run.clock.map(DateView::of),
            days_left: run.clock.map(|c| today.days_until(c)),
        };
        let mut own_due = None;
        match (&o.action, &beat.kind) {
            (BeatAction::Visit { verb, location }, BeatKind::Visit { verb_key, time, .. }) => {
                v.label = TextView::of(verb_key.clone(), args.clone());
                v.needs.place = Some(location_view(st, *location));
                v.needs.halves = halves(*time);
                v.route = route_to(engine, "visit", *location);
                v.verb = Some(verb.clone());
                if v.route.is_none() {
                    v.available = false;
                    v.blocked_key = Some("visit-blocked-situation".into());
                } else if !beats::has_time(st, *time) {
                    v.available = false;
                    v.blocked_key = Some("visit-blocked-time".into());
                }
            }
            (BeatAction::Talk { person, topic }, BeatKind::Talk { choice, .. }) => {
                v.label = TextView::of(choice.label_key.clone(), args.clone());
                v.needs.person = person_ref(engine, *person);
                v.needs.halves = 1;
                v.topic = Some(topic.clone());
                let to = crate::talk::talk_location(engine, *person);
                v.needs.place = to.map(|l| location_view(st, l));
                v.route = to.and_then(|l| route_to(engine, "talk", l));
                if v.route.is_none() {
                    v.available = false;
                    v.blocked_key = Some("visit-blocked-nobody".into());
                } else if !beats::has_time(st, VisitTime::Half) {
                    v.available = false;
                    v.blocked_key = Some("visit-blocked-time".into());
                }
            }
            (BeatAction::Decision { decision }, _) => {
                let Some(d) = crate::narrative::decision_by_id(engine, *decision) else {
                    continue;
                };
                own_due = Some(SimDate(d.deadline.day_number));
                v.label = TextView::of(d.title_key, d.args);
                v.decision = Some(*decision);
                v.needs.place = st
                    .narrative
                    .decision(mandate_sim::narrative::DecisionId(*decision))
                    .and_then(|x| x.place)
                    .map(|l| location_view(st, l));
            }
            (BeatAction::Ask { ask }, _) => {
                let Some(a) = st.asks.open().find(|a| a.id.0 == *ask) else {
                    continue;
                };
                let av = crate::asks::ask_view(engine, a);
                own_due = a.next_date();
                v.label = TextView::of(av.title_key, av.args);
                v.ask = Some(*ask);
                // Whom its next need is done with, else who asks.
                v.needs.person = av
                    .person
                    .clone()
                    .or_else(|| a.asker_person().and_then(|p| person_ref(engine, p)));
                v.needs.place = av.how.as_ref().and_then(|h| h.to.clone());
                v.route = av.how.filter(|h| h.action != "wait");
            }
            (
                BeatAction::Exchange {
                    run: r,
                    beat: b,
                    location,
                },
                BeatKind::Exchange(x),
            ) => {
                // A scene (DEC-112): its title, place and time; begun with
                // `CommandDto::ArcExchange` (inside `Go`) and played in its
                // own view (`QueryDto::ArcExchange`).
                let Some(scene) = crate::exchange::scene_view(engine, *r, b) else {
                    continue;
                };
                v.label = scene.title.clone();
                v.needs.place = Some(location_view(st, *location));
                v.needs.person = scene.other.clone();
                v.needs.halves = halves(x.time);
                v.route = scene.route.clone();
                v.available = scene.available;
                v.blocked_key = scene.blocked_key.clone();
                own_due = mandate_sim::arcs::exchange::last_day(run);
            }
            _ => continue,
        }
        if let Some(d) = own_due {
            v.due = Some(DateView::of(d));
            v.days_left = Some(today.days_until(d));
        }
        let day = v.due.as_ref().map_or(u32::MAX, |d| d.day_number);
        out.push((day, i, v));
    }
    out.sort_by_key(|a| (a.0, a.1));
    out.into_iter().map(|x| x.2).collect()
}

/// The events of a run the log keeps, newest first: every stage reached
/// and beat done; those of its hidden stages only in detail (the player's
/// own acts always).
fn history(engine: &Engine, id: ArcId, def: &ArcDef, level: Level) -> Vec<EventView> {
    let st = engine.state();
    let hidden = |stage: &str| def.stage(stage).is_some_and(|s| s.hidden);
    st.events
        .iter()
        .rev()
        .filter(|e| match &e.kind {
            EventKind::ArcStage { arc, stage, .. } => {
                *arc == id.0 && (level >= Level::Detailed || !hidden(stage))
            }
            EventKind::ArcEnded { arc, .. } => *arc == id.0,
            EventKind::ArcBeat { arc, stage, .. } => {
                *arc == id.0
                    && (level >= Level::Detailed
                        || !hidden(stage)
                        || e.causes.iter().any(|c| matches!(c, Cause::Player(_))))
            }
            _ => false,
        })
        .map(|e| event_view(e, engine))
        .collect()
}

/// The latest story the outlets ran about a run.
fn headline(engine: &Engine, id: ArcId) -> Option<EventView> {
    engine
        .state()
        .events
        .iter()
        .rev()
        .find(|e| matches!(&e.kind, EventKind::ArcBeat { arc, kind, .. } if *arc == id.0 && kind == "press"))
        .map(|e| event_view(e, engine))
}

/// The event of a run entering a stage, while the log keeps it.
fn stage_event(engine: &Engine, id: ArcId, stage: &str) -> Option<u32> {
    engine
        .state()
        .events
        .iter()
        .rev()
        .find(|e| matches!(&e.kind, EventKind::ArcStage { arc, stage: s, .. } if *arc == id.0 && s == stage))
        .map(|e| e.id.0)
}

fn turn_view(run: &ArcRun, def: &ArcDef) -> Option<ArcTurnView> {
    if let Some(t) = &run.turn {
        return Some(ArcTurnView {
            state: "reached".into(),
            on: Some(DateView::of(t.on)),
        });
    }
    if !mandate_sim::arcs::turn_ahead(def, &run.stage) {
        return None;
    }
    // The clock's day is the turn's when an edge on the clock leads there.
    let on_clock = def.stage(&run.stage).is_some_and(|s| {
        s.next
            .iter()
            .any(|e| e.clock_within_days.is_some() && def.stage(&e.to).is_some_and(|t| t.climax))
    });
    Some(ArcTurnView {
        state: "ahead".into(),
        on: run.clock.filter(|_| on_clock).map(DateView::of),
    })
}

fn blank(id: u32, level: Level) -> ArcView {
    ArcView {
        id,
        level: level.into(),
        lead: None,
        title: None,
        at: None,
        began: None,
        days: None,
        stage: None,
        clock: None,
        turn: None,
        cast: Vec::new(),
        beats: Vec::new(),
        history: Vec::new(),
        headline: None,
        ended: None,
    }
}

/// One arc run as the player sees it: a running one, or one that ended
/// and is still on record. `None` where there is no such run, or the
/// player knows nothing of it (the two read the same: DEC-091).
pub fn arc_view(engine: &Engine, id: u32) -> Option<ArcView> {
    let st = engine.state();
    let content = engine.content();
    st.player.as_ref()?;
    let today = st.calendar.date;
    let id = ArcId(id);
    if let Some(run) = st.arcs.run(id) {
        let def = content.arc(&run.template)?;
        let lv = arc_level(engine, run, def);
        if lv == Level::Unknown {
            return None;
        }
        let mut v = blank(id.0, lv);
        if lv < Level::Known {
            v.lead = Some(lead_view(engine, run));
            return Some(v);
        }
        let args = run_args(engine, run, def);
        v.title = Some(TextView::of(def.title_key.clone(), args.clone()));
        v.at = Some(jurisdiction_dto(&st.world, run.at));
        v.began = Some(DateView::of(run.began));
        v.days = Some(run.began.days_until(today));
        v.stage = def.stage(&run.stage).map(|s| ArcStageView {
            id: s.id.clone(),
            text: TextView::of(s.key.clone(), args),
            since: DateView::of(run.since),
            event: stage_event(engine, id, &s.id),
        });
        v.clock = run.clock.map(|c| clock_view(today, c));
        v.turn = turn_view(run, def);
        v.cast = cast_view(engine, &run.cast);
        v.beats = beats_view(engine, run, def);
        v.history = history(engine, id, def, lv);
        v.headline = headline(engine, id);
        return Some(v);
    }
    // One that ended: what the record keeps of it.
    let past = st.arcs.past.iter().find(|p| p.id == id)?;
    let def = content.arc(&past.template)?;
    let omniscient = st
        .player
        .as_ref()
        .is_some_and(|p| p.knowledge.lab_omniscient);
    let lv = if omniscient {
        Level::Detailed
    } else if def.stage(&past.exit).is_some_and(|s| s.hidden) {
        Level::Unknown
    } else {
        Level::Known
    };
    if lv == Level::Unknown {
        return None;
    }
    let mut v = blank(id.0, lv);
    let args = text_args(engine, &past.cast, past.at, Vec::new());
    v.title = Some(TextView::of(def.title_key.clone(), args));
    v.at = Some(jurisdiction_dto(&st.world, past.at));
    v.began = Some(DateView::of(past.began));
    v.days = Some(past.began.days_until(past.ended));
    v.turn = past.turn.as_ref().map(|t| ArcTurnView {
        state: "reached".into(),
        on: Some(DateView::of(t.on)),
    });
    v.cast = cast_view(engine, &past.cast);
    v.history = history(engine, id, def, lv);
    v.headline = headline(engine, id);
    // How it ended, with the facts as they were: the log's own line.
    let end = st
        .events
        .iter()
        .rev()
        .find(|e| matches!(&e.kind, EventKind::ArcEnded { arc, .. } if *arc == id.0))
        .map(|e| event_view(e, engine));
    v.ended = Some(ArcEndView {
        text: end.map(|e| TextView::of(e.text_key, e.args)),
        on: DateView::of(past.ended),
    });
    Some(v)
}

/// The running arcs the player knows, as rows: the soonest clock first,
/// then the oldest run.
pub fn arc_summaries(engine: &Engine) -> Vec<ArcSummaryView> {
    let st = engine.state();
    let content = engine.content();
    if st.player.is_none() || st.arcs.runs.is_empty() {
        return Vec::new();
    }
    let today = st.calendar.date;
    let offers = beats::offers(st, content);
    let mut out: Vec<(u32, u32, ArcSummaryView)> = Vec::new();
    for run in &st.arcs.runs {
        let Some(def) = content.arc(&run.template) else {
            continue;
        };
        let Some(stage) = def.stage(&run.stage) else {
            continue;
        };
        if arc_level(engine, run, def) < Level::Known {
            continue;
        }
        let args = run_args(engine, run, def);
        out.push((
            run.clock.map_or(u32::MAX, |c| c.0),
            run.id.0,
            ArcSummaryView {
                id: run.id.0,
                title_key: def.title_key.clone(),
                stage: TextView::of(stage.key.clone(), args.clone()),
                args,
                place: cast::jurisdiction_name(&st.world, run.at),
                at: jurisdiction_dto(&st.world, run.at),
                clock: run.clock.map(|c| clock_view(today, c)),
                open_beats: offers.iter().filter(|o| o.run == run.id).count() as u32,
            },
        ));
    }
    out.sort_by_key(|a| (a.0, a.1));
    out.into_iter().map(|x| x.2).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn beat(v: serde_json::Value) -> ArcBeat {
        serde_json::from_value(v).unwrap()
    }

    /// A beat the player took teaches what it says it does, else that the
    /// matter exists; what is aimed at the player is known; what nobody
    /// takes teaches the player nothing.
    #[test]
    fn what_a_beat_teaches() {
        let visit = |teaches: serde_json::Value| {
            beat(
                json!({ "id": "a", "type": "visit", "at": "x", "verb_key": "v",
                "outcome_key": "o", "teaches": teaches }),
            )
        };
        assert_eq!(taught(&visit(json!(null)), None), Level::Heard);
        assert_eq!(taught(&visit(json!("known")), None), Level::Known);
        let talk = beat(
            json!({ "id": "t", "type": "talk", "with": "p", "teaches": "known",
            "choice": { "label_key": "l", "say_key": "s", "replies": [
                { "key": "yes", "agrees": true, "teaches": "detailed" },
                { "key": "no" } ] } }),
        );
        assert_eq!(taught(&talk, Some(true)), Level::Detailed);
        assert_eq!(taught(&talk, None), Level::Detailed);
        // A refusal teaches only that there is something to refuse.
        assert_eq!(taught(&talk, Some(false)), Level::Heard);
        let decision = beat(json!({ "id": "d", "type": "decision", "dilemma": "dilemma.x" }));
        assert_eq!(taught(&decision, None), Level::Known);
        let press = beat(json!({ "id": "p", "type": "press", "headline_key": "h" }));
        assert_eq!(taught(&press, None), Level::Unknown);
    }

    /// A power is read from the beat's own conditions, by the
    /// constitution's id: no office or body is named.
    #[test]
    fn a_power_is_read_from_the_beats_conditions() {
        let with = |when: serde_json::Value| {
            beat(
                json!({ "id": "a", "type": "visit", "at": "x", "verb_key": "v",
                "outcome_key": "o", "when": when }),
            )
        };
        assert_eq!(power_of(&with(json!([]))), None);
        assert_eq!(
            power_of(&with(json!([{ "type": "office", "posts": ["any"] }]))),
            Some("office".to_string())
        );
        assert_eq!(
            power_of(&with(
                json!([{ "type": "holds_power", "power": "propose_budget" }])
            )),
            Some("propose_budget".to_string())
        );
    }

    #[test]
    fn a_clock_is_a_sentence_and_a_date() {
        let c = clock_view(SimDate(10), SimDate(19));
        assert_eq!((c.days_left, c.due.day_number), (9, 19));
        assert_eq!(c.text.key, "now-clock-days");
    }
}
