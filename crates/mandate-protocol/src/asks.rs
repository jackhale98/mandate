//! Asks and promises for the client (DEC-097 §2; ENGAGEMENT §3.9): the
//! pocket's Today block, the asks query (the story sheet's promises), and
//! the promise picker while campaigning. Facts and Fluent keys only: the
//! engine says what each ask needs, how far along it is, where it is done,
//! the casework odds, and what keeping or breaking it does.
//!
//! Tier-generic (DEC-098): a promise's voters are a constituency or the
//! electorate of a jurisdiction, named from the world, never from an office
//! word.

use crate::characters::{PersonRefView, person_ref};
use crate::narrative::EffectLineView;
use crate::places::{RouteView, location_view};
use crate::views::DateView;
use mandate_content::{EffectDef, Scope};
use mandate_core::SimDate;
use mandate_sim::Engine;
use mandate_sim::asks::{Ask, AskId, AskStatus, Asker, History, Need};
use mandate_sim::narrative::{Actor, CastEntry};
use mandate_sim::places::{self, Location};
use mandate_sim::politics::Post;
use mandate_sim::world::{Jurisdiction, NeighborhoodId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// A jurisdiction named for the client (a promise's electorate).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JurisdictionRefView {
    /// `local` | `regional` | `national`.
    pub tier: String,
    /// The town's or region's name (empty for the nation).
    pub name: String,
}

/// Who asks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AskerView {
    Person {
        person: PersonRefView,
    },
    /// The party, voiced by the person who asked for it (its chair).
    Party {
        party: u32,
        name: String,
        voice: Option<PersonRefView>,
    },
    Outlet {
        name: String,
    },
    /// A constituency's voters (a promise): Fluent `name_key` with `number`.
    Constituency {
        name_key: String,
        number: u32,
    },
    /// The voters of a whole jurisdiction (a promise made running for an
    /// office or an at-large seat): Fluent `name_key` with the ask's args;
    /// `at` names the jurisdiction.
    Electorate {
        name_key: String,
        at: JurisdictionRefView,
    },
}

/// How far along a need is ("2 of 3 neighbourhoods").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AskProgressView {
    pub done: u32,
    pub total: u32,
}

/// One of an ask's needs, met in order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AskNeedView {
    /// `vote`, `against_all`, `statement`, `town_hall`, `canvass`, `visit`,
    /// `talk`, `introduce`, `enacted`, `casework`,
    /// `no_surfaced_hidden_fact`, `quiet`.
    pub kind: String,
    /// `done` | `current` | `later`.
    pub state: String,
    /// Kept by not acting: only the deadline keeps it.
    pub watch: bool,
    pub progress: Option<AskProgressView>,
}

/// An ask or a campaign promise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AskView {
    pub id: u32,
    pub template: String,
    /// A campaign promise (its asker is a constituency's voters).
    pub promise: bool,
    /// The ask in a few words.
    pub title_key: String,
    /// The situation.
    pub detail_key: String,
    /// What keeps it ("Vote yes on the { $bill } bill").
    pub need_key: String,
    /// How it ended (kept, broken, or why it no longer stands), once resolved.
    pub outcome_key: Option<String>,
    /// Fluent arguments for every key above (`*_key` arguments are keys).
    pub args: BTreeMap<String, String>,
    pub asker: AskerView,
    /// `offered` | `accepted` | `kept` | `broken` | `declined` | `void`.
    pub status: String,
    /// Declined by letting the offer lapse unanswered.
    pub unanswered: bool,
    /// The asker's last earlier ask: `none`, `kept`, `broken`.
    pub history: String,
    pub public: bool,
    pub big: bool,
    pub opened: DateView,
    /// Offers only: the day it lapses unanswered.
    pub respond_by: Option<DateView>,
    pub due: DateView,
    /// Days to the answer (an offer) or the deadline (accepted); 0 once resolved.
    pub days_left: u32,
    pub resolved_on: Option<DateView>,
    /// Due (or to be answered) within the current turn.
    pub due_this_turn: bool,
    pub needs: Vec<AskNeedView>,
    /// The current need's progress (a town hall in each neighbourhood).
    pub progress: Option<AskProgressView>,
    /// Where the current need is done (auto-routing, DEC-079), if a place does it.
    pub how: Option<RouteView>,
    /// Whom to talk to, for a conversation need.
    pub person: Option<PersonRefView>,
    /// Casework now: the chance "Sort it out" works (the odds rolled, DEC-073).
    pub odds: Option<f64>,
    pub keep_lines: Vec<EffectLineView>,
    pub break_lines: Vec<EffectLineView>,
    /// Offers only: what saying no (or not answering) does.
    pub decline_lines: Vec<EffectLineView>,
}

/// Whole-career counts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AskTallyView {
    pub offered: u32,
    pub accepted: u32,
    pub declined: u32,
    pub lapsed: u32,
    pub kept: u32,
    pub broken: u32,
    pub void: u32,
    pub promises_made: u32,
    pub promises_kept: u32,
    pub promises_broken: u32,
    pub promises_void: u32,
    pub promises_open: u32,
}

/// Asks and promises: the open ones, recent outcomes, every promise kept on
/// record, and the career's counts (the story sheet).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AsksView {
    /// Offers, then accepted asks and promises by due date.
    pub open: Vec<AskView>,
    /// Resolved asks (not promises), newest first.
    pub recent: Vec<AskView>,
    /// Campaign promises on record, newest first (open and resolved).
    pub promises: Vec<AskView>,
    pub tally: AskTallyView,
}

/// A promise the player could make now (the promise picker).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PromiseOptionView {
    /// `CommandDto::Promise { template, answer }`.
    pub template: String,
    /// The chosen answer (a policy template), for a promise on the crisis.
    pub answer: Option<String>,
    /// The worry it answers (a concern id), if one.
    pub concern: Option<String>,
    pub title_key: String,
    pub detail_key: String,
    pub need_key: String,
    pub args: BTreeMap<String, String>,
    /// When it falls due: `ui-promise-due-term`, `ui-promise-due-budget-year`
    /// or `ui-promise-due-days` (with `days`).
    pub due_key: String,
    pub keep_lines: Vec<EffectLineView>,
    pub break_lines: Vec<EffectLineView>,
}

/// The promise picker (campaigning only).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PromiseMenuView {
    pub options: Vec<PromiseOptionView>,
    /// Promises made in this race, and the most a race allows.
    pub made: u32,
    pub max: u32,
    /// Who the promises are made to, and the Fluent arguments of its `name_key`.
    pub voters: Option<AskerView>,
    pub voters_args: BTreeMap<String, String>,
}

pub(crate) fn jurisdiction_ref(engine: &Engine, at: Jurisdiction) -> JurisdictionRefView {
    let w = &engine.state().world;
    let (tier, name) = match at {
        Jurisdiction::Municipality { id } => (
            "local",
            w.municipalities
                .get(id.index())
                .map(|m| m.name.clone())
                .unwrap_or_default(),
        ),
        Jurisdiction::Region { id } => (
            "regional",
            w.regions
                .get(id.index())
                .map(|r| r.name.clone())
                .unwrap_or_default(),
        ),
        Jurisdiction::Nation => ("national", String::new()),
    };
    JurisdictionRefView {
        tier: tier.into(),
        name,
    }
}

fn asker_view(engine: &Engine, a: &Ask) -> AskerView {
    let st = engine.state();
    let voice = || a.asker_person().and_then(|p| person_ref(engine, p));
    match a.asker {
        Asker::Person { id } => match person_ref(engine, id) {
            Some(person) => AskerView::Person { person },
            None => AskerView::Outlet {
                name: String::new(),
            },
        },
        Asker::Party { id } => AskerView::Party {
            party: id.0,
            name: st
                .politics
                .party(id)
                .map(|p| p.name.clone())
                .unwrap_or_default(),
            voice: voice(),
        },
        Asker::Outlet { id } => AskerView::Outlet {
            name: st
                .media
                .outlets
                .iter()
                .find(|o| o.id == id)
                .map(|o| o.name.clone())
                .unwrap_or_default(),
        },
        Asker::Constituency { id } => AskerView::Constituency {
            name_key: "ask-constituency".into(),
            number: st
                .world
                .constituencies
                .get(id.index())
                .map(|c| c.number)
                .unwrap_or_default(),
        },
        Asker::Electorate { at } => AskerView::Electorate {
            name_key: "ask-electorate".into(),
            at: jurisdiction_ref(engine, at),
        },
    }
}

/// The voters of a race's post.
fn voters_of(post: Post) -> Asker {
    match post {
        Post::Seat {
            constituency: Some(id),
            ..
        } => Asker::Constituency { id },
        other => Asker::Electorate { at: other.at() },
    }
}

fn signed(v: f64) -> String {
    let digits = if v != 0.0 && v.abs() < 0.95 { 1 } else { 0 };
    if v > 0.0 {
        format!("+{v:.digits$}")
    } else if v < 0.0 {
        format!("−{:.digits$}", -v)
    } else {
        format!("{v:.digits$}")
    }
}

fn line(key: &str, args: Vec<(&str, String)>, tone: &str) -> EffectLineView {
    EffectLineView {
        key: key.into(),
        args: args.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        tone: tone.into(),
    }
}

/// What one of an ask's effects would do, before it happens (DEC-073).
fn effect_line(engine: &Engine, cast: &[CastEntry], e: &EffectDef) -> Option<EffectLineView> {
    let st = engine.state();
    let name = |role: &str| {
        mandate_sim::narrative::person_of(cast, role)
            .and_then(|p| person_ref(engine, p))
            .map(|p| p.name)
            .unwrap_or_default()
    };
    let tone = |cost: bool| if cost { "cost" } else { "gain" };
    Some(match e {
        EffectDef::Opinion { role, delta, .. } => line(
            "effect-opinion",
            vec![("name", name(role)), ("delta", signed(*delta))],
            tone(*delta < 0.0),
        ),
        EffectDef::Approval {
            scope,
            classes,
            neighborhood,
            delta,
        } => {
            let d = signed(delta * 100.0);
            let t = tone(*delta < 0.0);
            let hood =
                neighborhood.as_deref().and_then(|r| {
                    match mandate_sim::narrative::actor_of(cast, r) {
                        Some(Actor::Neighborhood { id }) => st
                            .world
                            .neighborhoods
                            .get(id as usize)
                            .map(|h| h.name.clone()),
                        _ => None,
                    }
                });
            if let Some(h) = hood {
                line("effect-approval-hood", vec![("delta", d), ("hood", h)], t)
            } else {
                let who = match classes.as_slice() {
                    [c] => format!("effect-who-class-{c}"),
                    _ if *scope == Scope::Town => "effect-who-town".into(),
                    _ => "effect-who-ward".into(),
                };
                line("effect-approval", vec![("delta", d), ("who_key", who)], t)
            }
        }
        EffectDef::Standing { delta } => line(
            "effect-standing",
            vec![("delta", signed(*delta))],
            tone(*delta < 0.0),
        ),
        EffectDef::Record { delta } => line(
            "effect-record",
            vec![("delta", signed(delta * 100.0))],
            tone(*delta < 0.0),
        ),
        EffectDef::Money { amount } => line(
            "effect-money",
            vec![("delta", signed(*amount))],
            tone(*amount < 0.0),
        ),
        EffectDef::Favour {
            role,
            owed_to_player,
        } => line(
            if *owed_to_player {
                "effect-favour-owed-to-you"
            } else {
                "effect-favour-owed"
            },
            vec![("name", name(role))],
            tone(!owed_to_player),
        ),
        EffectDef::Learn { subject, .. } => line(
            "effect-learn",
            vec![(
                "what_key",
                format!("knows-{}", subject.key().replace('_', "-")),
            )],
            "gain",
        ),
        // Validation allows no other effect on an ask.
        _ => return None,
    })
}

fn lines(engine: &Engine, cast: &[CastEntry], defs: &[EffectDef]) -> Vec<EffectLineView> {
    defs.iter()
        .filter_map(|e| effect_line(engine, cast, e))
        .collect()
}

/// Effects of a template: (keep, break, decline).
fn template_effects(engine: &Engine, a: &Ask) -> (Vec<EffectDef>, Vec<EffectDef>, Vec<EffectDef>) {
    let c = engine.content();
    if a.promise {
        c.promise(&a.template)
            .map(|t| (t.keep.clone(), t.broken.clone(), Vec::new()))
            .unwrap_or_default()
    } else {
        c.ask(&a.template)
            .map(|t| (t.keep.clone(), t.broken.clone(), t.decline.clone()))
            .unwrap_or_default()
    }
}

/// Template keys: (title, detail, need, kept, broken).
fn template_keys(engine: &Engine, a: &Ask) -> [String; 5] {
    let c = engine.content();
    let keys = if a.promise {
        c.promise(&a.template).map(|t| {
            [
                t.name_key.clone(),
                t.detail_key.clone(),
                t.need_key.clone(),
                t.kept_key.clone(),
                t.broken_key.clone(),
            ]
        })
    } else {
        c.ask(&a.template).map(|t| {
            [
                t.name_key.clone(),
                t.detail_key.clone(),
                t.need_key.clone(),
                t.kept_key.clone(),
                t.broken_key.clone(),
            ]
        })
    };
    keys.unwrap_or_default()
}

/// Fluent arguments for an ask or a promise (the engine's seam, `ask_args`).
fn args_of(engine: &Engine, a: &Ask) -> BTreeMap<String, String> {
    crate::narrative::ask_args(engine, a)
}

/// Fluent arguments for the outcome of an ask's own command (the toast):
/// the ask's, under the outcome's own.
pub fn outcome_args(engine: &Engine, ask: u32, args: &mut BTreeMap<String, String>) {
    if let Some(a) = engine.state().asks.get(AskId(ask)) {
        for (k, v) in args_of(engine, a) {
            args.entry(k).or_insert(v);
        }
    }
}

fn route(
    engine: &Engine,
    key: &str,
    hood: Option<NeighborhoodId>,
    at: Option<Location>,
) -> Option<RouteView> {
    let st = engine.state();
    let pl = st.player.as_ref()?;
    let hood = hood.map(|h| h.0);
    let to = match at {
        Some(l) => Some(l),
        None => places::best_for(st, engine.content(), key, false, hood),
    };
    Some(RouteView {
        action: key.into(),
        late_night: false,
        neighborhood: hood,
        here: to == Some(pl.location),
        to: to.map(|l| location_view(st, l)),
    })
}

/// Where a need is done, if a place does it.
fn need_route(engine: &Engine, a: &Ask, n: &Need) -> Option<RouteView> {
    match n {
        Need::Vote { .. } => route(engine, "vote", None, None),
        Need::Statement { .. } => route(engine, "press", None, None),
        Need::TownHall { hoods } => {
            let next = hoods.iter().find(|h| !a.done.contains(h)).copied();
            route(engine, "town_hall", next, None)
        }
        Need::Canvass { hood, .. } => route(engine, "canvass", Some(*hood), None),
        Need::Visit { place } => {
            let p = engine.state().world.places.get(place.index())?;
            route(
                engine,
                "visit",
                Some(p.neighborhood),
                Some(Location::Place { place: *place }),
            )
        }
        Need::Introduce { .. } => route(engine, "introduce", None, None),
        Need::Enacted { .. } => route(engine, "draft", None, None),
        Need::Casework => route(engine, "casework", None, None),
        Need::Talk { .. }
        | Need::VoteAgainstAll { .. }
        | Need::NoSurfacedHiddenFact
        | Need::Quiet { .. } => None,
    }
}

fn progress_of(a: &Ask, n: &Need) -> Option<AskProgressView> {
    match n {
        Need::TownHall { hoods } if hoods.len() > 1 => Some(AskProgressView {
            done: hoods.iter().filter(|h| a.done.contains(h)).count() as u32,
            total: hoods.len() as u32,
        }),
        _ => None,
    }
}

/// Whether an open ask waits on the player within `days` of `date`: an
/// offer to answer, or an accepted ask or promise with something left to
/// do (one kept by not acting needs nothing) due in that time.
fn waits_within(a: &Ask, date: SimDate, days: u32) -> bool {
    let until = date.add_days(days);
    match a.status {
        AskStatus::Offered => a.respond_by <= until,
        AskStatus::Accepted { .. } => a.due <= until && a.current().is_some_and(|n| !n.watch()),
        _ => false,
    }
}

/// An ask or promise as the client shows it.
pub fn ask_view(engine: &Engine, a: &Ask) -> AskView {
    let st = engine.state();
    let content = engine.content();
    let date = st.calendar.date;
    let [title_key, detail_key, need_key, kept_key, broken_key] = template_keys(engine, a);
    let (keep, broken, decline) = template_effects(engine, a);
    let open = a.open();
    let kept = matches!(a.status, AskStatus::Kept { .. });
    let current = if a.accepted() { a.current() } else { None };
    let needs = a
        .needs
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let state = if kept || i < a.step as usize {
                "done"
            } else if i == a.step as usize {
                "current"
            } else {
                "later"
            };
            AskNeedView {
                kind: n.key().into(),
                state: state.into(),
                watch: n.watch(),
                progress: progress_of(a, n),
            }
        })
        .collect();
    let outcome_key = match &a.status {
        AskStatus::Kept { .. } => Some(kept_key),
        AskStatus::Broken { .. } => Some(broken_key),
        AskStatus::Void { reason, .. } => Some(reason.clone()),
        _ => None,
    };
    let (unanswered, resolved_on) = match &a.status {
        AskStatus::Declined { on, unanswered } => (*unanswered, Some(*on)),
        s => (false, s.resolved_on()),
    };
    // The next need (an offer shows its first): where it is done, whom to talk to.
    let shown = if open {
        a.current().or(a.needs.first())
    } else {
        None
    };
    let person = match shown {
        Some(Need::Talk { person }) => person_ref(engine, *person),
        _ => None,
    };
    let odds = match current {
        Some(Need::Casework) => Some(mandate_sim::asks::casework_odds(st, content)),
        _ => None,
    };
    AskView {
        id: a.id.0,
        template: a.template.clone(),
        promise: a.promise,
        title_key,
        detail_key,
        need_key,
        outcome_key,
        args: args_of(engine, a),
        asker: asker_view(engine, a),
        status: a.status.key().into(),
        unanswered,
        history: match a.history {
            History::None => "none",
            History::Kept => "kept",
            History::Broken => "broken",
        }
        .into(),
        public: a.public,
        big: a.big,
        opened: DateView::of(a.opened),
        respond_by: a.offered().then(|| DateView::of(a.respond_by)),
        due: DateView::of(a.due),
        days_left: a.next_date().map_or(0, |d| date.days_until(d)),
        resolved_on: resolved_on.map(DateView::of),
        due_this_turn: waits_within(a, date, engine.turn_days()),
        needs,
        progress: shown.and_then(|n| progress_of(a, n)),
        how: shown.and_then(|n| need_route(engine, a, n)),
        person,
        odds,
        keep_lines: lines(engine, &a.cast, &keep),
        break_lines: lines(engine, &a.cast, &broken),
        decline_lines: if a.offered() {
            lines(engine, &a.cast, &decline)
        } else {
            Vec::new()
        },
    }
}

/// Open asks and promises: offers first (soonest lapse first), then
/// accepted ones by due date.
pub fn open_asks(engine: &Engine) -> Vec<AskView> {
    let st = engine.state();
    if st.player.is_none() {
        return Vec::new();
    }
    let mut v: Vec<&Ask> = st.asks.open().collect();
    v.sort_by_key(|a| (!a.offered(), a.next_date(), a.id));
    v.into_iter().map(|a| ask_view(engine, a)).collect()
}

/// Asks and promises resolved within `days`, newest first (the Today
/// block's last outcomes), at most `n`.
pub fn recent_outcomes(engine: &Engine, days: u32, n: usize) -> Vec<AskView> {
    let st = engine.state();
    let date = st.calendar.date;
    st.asks
        .resolved()
        .filter(|a| {
            a.status
                .resolved_on()
                .is_some_and(|on| on.days_until(date) <= days)
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .take(n)
        .map(|a| ask_view(engine, a))
        .collect()
}

/// The pocket's badge: offers to answer, plus asks and promises with
/// something to do that fall due within the turn.
pub fn asks_due_soon(engine: &Engine) -> u32 {
    let st = engine.state();
    let (date, days) = (st.calendar.date, engine.turn_days());
    st.asks
        .open()
        .filter(|a| a.offered() || waits_within(a, date, days))
        .count() as u32
}

/// `QueryDto::Asks`: open asks, recent outcomes, the promises on record
/// and the career's counts.
pub fn asks_view(engine: &Engine) -> Option<AsksView> {
    let st = engine.state();
    st.player.as_ref()?;
    let a = &st.asks;
    let t = &a.tally;
    let outcomes = a.promise_outcomes();
    Some(AsksView {
        open: open_asks(engine),
        recent: a
            .resolved()
            .filter(|x| !x.promise)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .take(20)
            .map(|x| ask_view(engine, x))
            .collect(),
        promises: a
            .asks
            .iter()
            .filter(|x| x.promise)
            .rev()
            .map(|x| ask_view(engine, x))
            .collect(),
        tally: AskTallyView {
            offered: t.offered,
            accepted: t.accepted,
            declined: t.declined,
            lapsed: t.lapsed,
            kept: t.kept,
            broken: t.broken,
            void: t.void,
            promises_made: outcomes.made,
            promises_kept: outcomes.kept,
            promises_broken: outcomes.broken,
            promises_void: outcomes.void,
            promises_open: outcomes.open,
        },
    })
}

/// The post of the race the player is running in.
fn my_race(engine: &Engine) -> Option<Post> {
    let st = engine.state();
    let me = st.player.as_ref()?.politician;
    st.politics
        .elections
        .races
        .iter()
        .find(|r| r.candidates.iter().any(|c| c.politician == me))
        .map(|r| r.seat)
}

/// Whether a promise can be made now (the Today block's picker button).
pub fn can_promise(engine: &Engine) -> bool {
    !mandate_sim::asks::promise_menu(engine.state(), engine.content()).is_empty()
}

/// A promise as it would stand if made now (for its text): to the race's
/// voters, with the option's answer.
fn would_be(engine: &Engine, template: &str, answer: Option<String>, race: Option<Post>) -> Ask {
    let date = engine.state().calendar.date;
    Ask {
        id: AskId(0),
        template: template.to_string(),
        promise: true,
        asker: race.map(voters_of).unwrap_or(Asker::Electorate {
            at: Jurisdiction::Nation,
        }),
        cast: Vec::new(),
        needs: Vec::new(),
        step: 0,
        done: Vec::new(),
        public: true,
        big: false,
        opened: date,
        respond_by: date,
        due: date,
        status: AskStatus::Accepted { on: date },
        history: History::None,
        race,
        answer,
        tries: 0,
    }
}

/// `QueryDto::PromiseMenu`: the promises the player could make now, with
/// their text and what keeping or breaking them does. Empty when not
/// campaigning.
pub fn promise_menu_view(engine: &Engine) -> PromiseMenuView {
    let st = engine.state();
    let content = engine.content();
    let race = my_race(engine);
    let max = content.ask_params.promises.max_per_campaign as u32;
    let made = match race {
        Some(r) if st.asks.promises_race == Some(r) => st.asks.promises_this_campaign as u32,
        _ => 0,
    };
    let options = mandate_sim::asks::promise_menu(st, content)
        .into_iter()
        .filter_map(|o| {
            let t = content.promise(&o.template)?;
            let a = would_be(engine, &o.template, o.answer.clone(), race);
            let mut args = args_of(engine, &a);
            args.remove("days");
            let due_key = match &t.due {
                mandate_content::asks::PromiseDue::Term => "ui-promise-due-term",
                mandate_content::asks::PromiseDue::BudgetYear => "ui-promise-due-budget-year",
                mandate_content::asks::PromiseDue::Days { days } => {
                    args.insert("due_days".into(), days.to_string());
                    "ui-promise-due-days"
                }
            };
            Some(PromiseOptionView {
                template: o.template,
                answer: o.answer,
                concern: o.concern,
                title_key: t.name_key.clone(),
                detail_key: t.detail_key.clone(),
                need_key: t.need_key.clone(),
                args,
                due_key: due_key.into(),
                keep_lines: lines(engine, &[], &t.keep),
                break_lines: lines(engine, &[], &t.broken),
            })
        })
        .collect();
    let voters = race.map(|r| {
        let a = would_be(engine, "", None, Some(r));
        let mut args = args_of(engine, &a);
        args.remove("days");
        (asker_view(engine, &a), args)
    });
    PromiseMenuView {
        options,
        made,
        max,
        voters_args: voters.as_ref().map(|v| v.1.clone()).unwrap_or_default(),
        voters: voters.map(|v| v.0),
    }
}
