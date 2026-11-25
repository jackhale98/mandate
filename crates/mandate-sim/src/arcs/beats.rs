//! The beats a player takes (STAGED_ARCS §1.5, §5): commands that already
//! exist reach every one, so quick mode and Places both do.
//!
//! - **visit**: the `Visit` action with the beat's verb id
//!   (`arc.<run>.<stage>.<beat>`), done at the cast place or on the streets
//!   of the cast area. `places::locations_for` knows where; the place lists
//!   it with its other verbs.
//! - **talk**: the arc's own choice in a conversation with a cast person,
//!   a talk hook (owner `arc`): the `Talk` command with the choice's id as
//!   its topic.
//! - **decision**, **ask**: the run asked the narrative and the asks for
//!   them (`step`); the player answers with `Answer` and `RespondAsk`.
//! - **exchange**: a played scene (DEC-112; [`super::exchange`]): the
//!   `Exchange` command, begun at the cast place.
//!
//! These are the arcs' own commands: they write `ArcsState`, the time the
//! act takes, and the event; what the beat changes elsewhere is applied
//! through its owner's function with its cause (`effects::apply_now`).
//! [`offers`] lists what can be done now, for the view (task A3), the
//! stand-ins (A4) and the tests.

use super::{ArcId, ArcRef, ArcRun, BeatRef, cast, effects, step};
use crate::actions::ActionResult;
use crate::characters::CharacterId;
use crate::commands::EngineError;
use crate::condition::Args;
use crate::narrative::CastEntry;
use crate::narrative::cast::View;
use crate::places::Location;
use crate::spine::{Cause, PlayerAct};
use crate::state::State;
use crate::talk::hooks::{self, Choice, ChoiceCost, Done, Hook, Reply};
use crate::world::Area;
use mandate_content::arcs::{ArcBeat, ArcDef, BeatKind, StageDef};
use mandate_content::visits::VisitTime;
use mandate_content::{Condition, ContentBundle};

/// The prefix of an arc's verb id (content verbs are `verb.*`).
pub const VERB_PREFIX: &str = "arc.";

fn bad(msg: &str) -> EngineError {
    EngineError::InvalidCommand(msg.into())
}

/// Whether a `Visit` verb is an arc's beat.
pub fn is_verb(verb: &str) -> bool {
    verb.starts_with(VERB_PREFIX)
}

/// `<run>.<stage>.<beat>`: a beat of a run, as verbs and choices name it.
fn local_id(run: ArcId, b: &BeatRef) -> String {
    format!("{}.{}.{}", run.0, b.stage, b.beat)
}

fn parse_local(s: &str) -> Option<(ArcId, BeatRef)> {
    let mut it = s.splitn(3, '.');
    let run = it.next()?.parse().ok().map(ArcId)?;
    let stage = it.next()?;
    let beat = it.next()?;
    Some((run, BeatRef::new(stage, beat)))
}

/// The verb id of a visit beat: `arc.<run>.<stage>.<beat>`.
pub fn verb_id(run: ArcId, b: &BeatRef) -> String {
    format!("{VERB_PREFIX}{}", local_id(run, b))
}

/// The topic of a talk beat's choice: `choice.arc.<run>.<stage>.<beat>`.
pub fn choice_id(run: ArcId, b: &BeatRef) -> String {
    hooks::choice_id(HOOK.owner, &local_id(run, b))
}

/// Where a visit beat is done: the cast place, or the streets of a cast
/// neighbourhood.
fn spot(run: &ArcRun, at: &str) -> Option<Location> {
    match run.actor(at)? {
        super::ArcActor::Place { id } => Some(Location::Place { place: id }),
        super::ArcActor::Area {
            area: Area::Hood { id },
        } => Some(Location::Street { neighborhood: id }),
        _ => None,
    }
}

/// A visit beat on offer.
#[derive(Clone, Debug, PartialEq)]
pub struct VisitOffer {
    /// The `Visit` action's verb.
    pub verb: String,
    pub run: ArcId,
    pub beat: BeatRef,
    pub location: Location,
    /// The button (no arguments).
    pub verb_key: String,
    pub time: VisitTime,
}

/// What the player can do about a run now, as the command that does it.
#[derive(Clone, Debug, PartialEq)]
pub enum BeatAction {
    /// `Act { Visit { verb } }` at `location`.
    Visit { verb: String, location: Location },
    /// `Talk { to: person, topic }`.
    Talk { person: CharacterId, topic: String },
    /// `Answer { decision, .. }`.
    Decision { decision: u32 },
    /// `RespondAsk { ask, .. }`, then whatever keeps it.
    Ask { ask: u32 },
    /// `Exchange { run, beat, act: Begin }` at `location`, then a `Say`
    /// for each round.
    Exchange {
        run: u32,
        beat: String,
        location: Location,
    },
}

/// A beat of a run on offer to the player.
#[derive(Clone, Debug, PartialEq)]
pub struct BeatOffer {
    pub run: ArcId,
    pub beat: BeatRef,
    pub action: BeatAction,
}

/// Every beat of a live run, with its run, template and stage, that is on
/// offer now.
fn on_offer<'c>(
    v: &View<'_>,
    state: &State,
    content: &'c ContentBundle,
) -> Vec<(ArcRun, &'c ArcDef, &'c StageDef, &'c ArcBeat)> {
    let mut out = Vec::new();
    for run in &state.arcs.runs {
        let Some(def) = content.arc(&run.template) else {
            continue;
        };
        let Some(stage) = def.stage(&run.stage) else {
            continue;
        };
        let people = run.narrative_cast();
        for beat in &stage.beats {
            if step::available(v, run, &people, stage, beat) {
                out.push((run.clone(), def, stage, beat));
            }
        }
    }
    out
}

/// The visit beats on offer now, in run and template order.
pub fn visit_offers(state: &State, content: &ContentBundle) -> Vec<VisitOffer> {
    if state.arcs.runs.is_empty() {
        return Vec::new();
    }
    crate::condition::on_state(state, content, |v| {
        on_offer(v, state, content)
            .into_iter()
            .filter_map(|(run, _, stage, beat)| {
                let BeatKind::Visit {
                    at, verb_key, time, ..
                } = &beat.kind
                else {
                    return None;
                };
                let b = BeatRef::new(&stage.id, &beat.id);
                Some(VisitOffer {
                    verb: verb_id(run.id, &b),
                    run: run.id,
                    location: spot(&run, at)?,
                    beat: b,
                    verb_key: verb_key.clone(),
                    time: *time,
                })
            })
            .collect()
    })
}

/// The visit beats done at a location.
pub fn visits_at(state: &State, content: &ContentBundle, loc: Location) -> Vec<VisitOffer> {
    visit_offers(state, content)
        .into_iter()
        .filter(|o| o.location == loc)
        .collect()
}

/// Where an arc's verb is done now (empty: it is not on offer).
pub fn visit_locations(state: &State, content: &ContentBundle, verb: &str) -> Vec<Location> {
    visit_offers(state, content)
        .into_iter()
        .filter(|o| o.verb == verb)
        .map(|o| o.location)
        .collect()
}

/// Whether the player has the time for a visit beat now.
pub fn has_time(state: &State, time: VisitTime) -> bool {
    let Some(pl) = state.player.as_ref() else {
        return false;
    };
    let half_open = pl.talk.half_slot == Some(state.calendar.turn);
    match time {
        VisitTime::Half if half_open => true,
        _ => pl.slots_left() >= 1,
    }
}

/// Spend the time a beat takes (half a slot shares the conversation's
/// bank).
pub(crate) fn spend(state: &mut State, time: VisitTime) {
    let turn = state.calendar.turn;
    if let Some(pl) = state.player.as_mut() {
        match time {
            VisitTime::Half if pl.talk.half_slot == Some(turn) => pl.talk.half_slot = None,
            VisitTime::Half => {
                pl.slots_used += 1;
                pl.talk.half_slot = Some(turn);
            }
            VisitTime::Slot => pl.slots_used += 1,
        }
    }
}

/// The names and dates a run's text is given from the simulation's side
/// (a visit's outcome, a choice's lines): `COMMON_ARGS`, the cast, and the
/// readings of its template's `facts` (a plant's name, its jobs).
fn text_args(state: &State, content: &ContentBundle, run: &ArcRun) -> Vec<(String, String)> {
    let mut out = match content.arc(&run.template) {
        Some(def) if !def.facts.is_empty() => {
            crate::condition::on_state(state, content, |v| cast::facts(v, def, run))
        }
        _ => Vec::new(),
    };
    out.extend(cast::names(&state.politics, &state.world, &run.cast));
    out.push((
        "place".into(),
        cast::jurisdiction_name(&state.world, run.at),
    ));
    if let Some(m) =
        crate::player::home(state).and_then(|m| state.world.municipalities.get(m.index()))
    {
        out.push(("town".into(), m.name.clone()));
    }
    out.extend(cast::dates(run, state.calendar.date));
    out
}

/// Do a visit beat where the player stands (`visits::visit` routes an
/// arc's verb here; `places::check_at` has checked the place).
pub fn visit(
    state: &mut State,
    content: &ContentBundle,
    verb: &str,
    late_night: bool,
) -> Result<ActionResult, EngineError> {
    if late_night {
        return Err(bad("that is not done late at night"));
    }
    let offer = visit_offers(state, content)
        .into_iter()
        .find(|o| o.verb == verb)
        .ok_or_else(|| bad("there is nothing like that to do now"))?;
    let here = state
        .player
        .as_ref()
        .map(|p| p.location)
        .ok_or_else(|| bad("no player"))?;
    if here != offer.location {
        return Err(bad("that is done somewhere else"));
    }
    if !has_time(state, offer.time) {
        return Err(bad("not enough time left this turn"));
    }
    let (def, beat) = content
        .arc(
            &state
                .arcs
                .run(offer.run)
                .map(|r| r.template.clone())
                .unwrap_or_default(),
        )
        .and_then(|d| Some((d, super::beat_of(d, &offer.beat)?.1)))
        .ok_or_else(|| bad("there is nothing like that to do now"))?;
    let BeatKind::Visit { outcome_key, .. } = &beat.kind else {
        return Err(bad("there is nothing like that to do now"));
    };
    spend(state, offer.time);
    let run = done(
        state,
        content,
        def,
        offer.run,
        &offer.beat,
        beat,
        None,
        true,
        "visit",
        PlayerAct::Visit,
    )
    .ok_or_else(|| bad("there is nothing like that to do now"))?;
    let mut res = ActionResult {
        outcome_key: outcome_key.clone(),
        ..Default::default()
    };
    res.args.extend(text_args(state, content, &run));
    res.args.insert("verb".into(), verb.into());
    Ok(res)
}

/// A beat the player took with a command: the run's marks, heat and done
/// list, the event, and its requests applied through their owners with
/// that event as their cause. Returns the run as it is now.
#[allow(clippy::too_many_arguments)]
fn done(
    state: &mut State,
    content: &ContentBundle,
    def: &ArcDef,
    id: ArcId,
    b: &BeatRef,
    beat: &ArcBeat,
    own: Option<(Option<&str>, f32, &[mandate_content::arcs::GatedEffect])>,
    full: bool,
    kind: &str,
    act: PlayerAct,
) -> Option<ArcRun> {
    let mut run = state.arcs.run(id)?.clone();
    let date = state.calendar.date;
    let (requests, event) = crate::condition::on_state(state, content, |v| {
        let requests = step::complete(v, &mut run, beat, own, full);
        if full && beat.once {
            run.mark_done(b.clone(), date);
        }
        (requests, step::beat_event(v, &run, def, b, kind, full))
    });
    let ev = state.events.record(date, event, Cause::Player(act));
    *state.arcs.run_mut(id)? = run.clone();
    effects::apply_now(state, content, requests, Cause::caused_by(ev));
    Some(run)
}

// --- The talk hook ----------------------------------------------------------------------

/// The arcs' hook (registered in `talk::hooks::HOOKS`).
pub const HOOK: Hook = Hook {
    owner: "arc",
    offer: talk_offer,
    chosen: talk_chosen,
};

/// A reply's conditions with the run's own decided now: the talk system
/// decides the rest with the conversation's slots, and knows no run.
/// `None`: one of the run's conditions fails, so the reply cannot be said.
fn narrow(
    v: &View<'_>,
    run: &ArcRun,
    people: &[CastEntry],
    when: &[Condition],
) -> Option<Vec<Condition>> {
    let cx = step::context(v, run, people);
    let now = |c: &Condition| crate::condition::holds(&cx, c, &mut Args::new());
    let mut out = Vec::new();
    for c in when {
        match c {
            Condition::Any { of } if c.reads_arc() => {
                if of.iter().any(|x| x.reads_arc() && now(x)) {
                    continue;
                }
                let rest: Vec<Condition> = of.iter().filter(|x| !x.reads_arc()).cloned().collect();
                if rest.is_empty() {
                    return None;
                }
                out.push(Condition::Any { of: rest });
            }
            c if c.reads_arc() => {
                if !now(c) {
                    return None;
                }
            }
            c => out.push(c.clone()),
        }
    }
    Some(out)
}

/// The choices for a conversation with `speaker`: one for each talk beat
/// on offer whose person they are.
fn talk_offer(state: &State, content: &ContentBundle, speaker: CharacterId) -> Vec<Choice> {
    if state.arcs.runs.is_empty() || state.player.is_none() {
        return Vec::new();
    }
    crate::condition::on_state(state, content, |v| {
        let mut out = Vec::new();
        for (run, _, stage, beat) in on_offer(v, state, content) {
            let BeatKind::Talk { with, choice } = &beat.kind else {
                continue;
            };
            if run.person(with) != Some(speaker) {
                continue;
            }
            let people = run.narrative_cast();
            let replies: Vec<Reply> = choice
                .replies
                .iter()
                .filter_map(|r| {
                    Some(Reply {
                        key: r.key.clone(),
                        when: narrow(v, &run, &people, &r.when)?,
                        agrees: r.agrees,
                    })
                })
                .collect();
            if replies.is_empty() {
                continue;
            }
            let b = BeatRef::new(&stage.id, &beat.id);
            out.push(Choice {
                id: choice_id(run.id, &b),
                label_key: choice.label_key.clone(),
                say_key: choice.say_key.clone(),
                args: text_args(state, content, &run),
                replies,
                cost: ChoiceCost::Conversation,
            });
        }
        out
    })
}

/// The reply was said: its mark and effects, and, if they agreed, the
/// beat's own. A refusal leaves the beat on offer.
fn talk_chosen(
    state: &mut State,
    content: &ContentBundle,
    choice: &Choice,
    _speaker: CharacterId,
    reply: &Reply,
) -> Done {
    let fallback = || Done {
        outcome_key: "outcome-talk".into(),
        args: Vec::new(),
    };
    let Some((id, b)) = choice
        .id
        .strip_prefix(hooks::PREFIX)
        .and_then(|s| s.strip_prefix("arc."))
        .and_then(parse_local)
    else {
        return fallback();
    };
    let Some(def) = state.arcs.run(id).and_then(|r| content.arc(&r.template)) else {
        return fallback();
    };
    let Some((_, beat)) = super::beat_of(def, &b) else {
        return fallback();
    };
    let BeatKind::Talk { choice: cdef, .. } = &beat.kind else {
        return fallback();
    };
    let Some(rdef) = cdef.replies.iter().find(|r| r.key == reply.key) else {
        return fallback();
    };
    let own = Some((rdef.sets.as_deref(), 0.0, rdef.effects.as_slice()));
    let Some(run) = done(
        state,
        content,
        def,
        id,
        &b,
        beat,
        own,
        rdef.agrees,
        "talk",
        PlayerAct::Talk,
    ) else {
        return fallback();
    };
    Done {
        outcome_key: if rdef.agrees {
            "outcome-arc-choice-agreed"
        } else {
            "outcome-arc-choice-refused"
        }
        .into(),
        args: text_args(state, content, &run),
    }
}

// --- What can be done now ---------------------------------------------------------------

/// Every beat of every live run the player can take now, as the command
/// that takes it: the arc's "what you can do" (STAGED_ARCS §5).
pub fn offers(state: &State, content: &ContentBundle) -> Vec<BeatOffer> {
    if state.arcs.runs.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<BeatOffer> = visit_offers(state, content)
        .into_iter()
        .map(|o| BeatOffer {
            run: o.run,
            beat: o.beat,
            action: BeatAction::Visit {
                verb: o.verb,
                location: o.location,
            },
        })
        .collect();
    let talks: Vec<(ArcId, BeatRef, CharacterId)> =
        crate::condition::on_state(state, content, |v| {
            on_offer(v, state, content)
                .into_iter()
                .filter_map(|(run, _, stage, beat)| match &beat.kind {
                    BeatKind::Talk { with, .. } => {
                        Some((run.id, BeatRef::new(&stage.id, &beat.id), run.person(with)?))
                    }
                    _ => None,
                })
                .collect()
        });
    for (run, beat, person) in talks {
        out.push(BeatOffer {
            run,
            action: BeatAction::Talk {
                person,
                topic: choice_id(run, &beat),
            },
            beat,
        });
    }
    // What the run asked the narrative and the asks for, still open.
    for d in state.narrative.pending() {
        if let Some(ArcRef { run, beat }) = &d.arc {
            out.push(BeatOffer {
                run: *run,
                beat: beat.clone(),
                action: BeatAction::Decision { decision: d.id.0 },
            });
        }
    }
    for a in state.asks.open() {
        if let Some(ArcRef { run, beat }) = &a.arc {
            out.push(BeatOffer {
                run: *run,
                beat: beat.clone(),
                action: BeatAction::Ask { ask: a.id.0 },
            });
        }
    }
    // A scene that can be begun now.
    for (run, beat, location) in super::exchange::view::open_scenes(state, content) {
        out.push(BeatOffer {
            run,
            action: BeatAction::Exchange {
                run: run.0,
                beat: beat.key(),
                location,
            },
            beat,
        });
    }
    out
}
