//! Playing a scene (EXCHANGE_SCENES §5, §11.2): the command, the rounds
//! nobody answered, and the end.
//!
//! `PlayerCommand::Exchange { run, beat, act }`:
//! - `Begin`: a located act (the cast place), spends the scene's time,
//!   binds the audience, builds the other side's deck and opens round 1;
//! - `Say { option }`: one of the lines the view lists, by its stable id.
//!   `l = landing(...)`, `h = keyed(...).chance(l.p)`, then `l.stands` or
//!   `l.fails` is applied and the line's own deeds are requested;
//! - `Leave`: the rounds left are forfeited, each biting in full.
//!
//! Ending the turn with a scene open forfeits the same way (the arcs'
//! step, the next day). A scene the player never began before its day
//! passed is `absent`: every prompt bites in full, the first one cold.
//!
//! These write `ArcsState` (the run), the player's time, and events. What
//! a line or a result changes elsewhere is an effect request applied
//! through its owner's function, with the round's or the closing event as
//! its cause (`effects::apply_now` here; `effects::post` from the step).

use super::attacks::{self, KEY_LINE_HOLDS, KEY_SAY, Opponent};
use super::landing::{self, Ended, RoundIn};
use super::moves::{self, Deed, Opt};
use super::state::{ArcExchangeState, ExchangePlayed, PromptNow, RoundPlayed, What};
use super::{Scene, audience};
use crate::actions::ActionResult;
use crate::arcs::{ArcId, ArcRun, cast, step};
use crate::characters::MemoryKind;
use crate::commands::EngineError;
use crate::events::{EventId, EventKind, EventLog};
use crate::narrative::cast::View;
use crate::scheduler::Effect;
use crate::spine::{Cause, Causes, Link, PlayerAct, Process};
use crate::state::State;
use mandate_content::ContentBundle;
use mandate_content::exchange::{MoveKind, Outcome, SourceKind};
use serde::{Deserialize, Serialize};

/// What the player does with a scene.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExchangeAct {
    /// Turn up: at the scene's place, for the scene's time.
    Begin,
    /// Say one of the lines on offer, by its id.
    Say { option: String },
    /// Walk out: the rounds left are forfeited.
    Leave,
}

fn bad(msg: &str) -> EngineError {
    EngineError::InvalidCommand(msg.into())
}

/// The command (`actions::apply` routes it here before the located check:
/// `Begin` checks its own place).
pub fn apply(
    state: &mut State,
    content: &ContentBundle,
    run: u32,
    beat: &str,
    act: &ExchangeAct,
) -> Result<ActionResult, EngineError> {
    let id = ArcId(run);
    let r = state.arcs.run(id).ok_or_else(|| bad("no such arc"))?;
    let bref = super::beat_ref(r, beat);
    let sc = super::scene(content, r, &bref).ok_or_else(|| bad("there is no scene like that"))?;
    match act {
        ExchangeAct::Begin => begin(state, content, id, &sc),
        ExchangeAct::Say { option } => say(state, content, id, &sc, option),
        ExchangeAct::Leave => leave(state, content, id, &sc),
    }
}

/// The text arguments every result of a scene carries: the cast and the
/// run's dates.
pub(crate) fn text_args(state: &State, run: &ArcRun) -> Vec<(String, String)> {
    let mut out = cast::names(&state.politics, &state.world, &run.cast);
    out.push((
        "place".into(),
        cast::jurisdiction_name(&state.world, run.at),
    ));
    out.extend(cast::dates(run, state.calendar.date));
    out
}

/// The other side airs a hidden fact as the round opens: saying it makes
/// it public (§4.1), by their own choice.
fn airs(prompt: &PromptNow, opp: &Opponent) -> Option<(Effect, Cause)> {
    let What::Hidden { id } = prompt.attack.subject.what else {
        return None;
    };
    if !prompt.stood {
        return None;
    }
    Some((
        Effect::Surface {
            hidden: id,
            by: opp.id,
        },
        opp.id.map_or(Cause::Process(Process::Arcs), Cause::Actor),
    ))
}

fn begin(
    state: &mut State,
    content: &ContentBundle,
    id: ArcId,
    sc: &Scene<'_>,
) -> Result<ActionResult, EngineError> {
    let run = state.arcs.run(id).ok_or_else(|| bad("no such arc"))?.clone();
    if run.exchange.is_some() {
        return Err(bad("the scene is already under way"));
    }
    if !super::view::open_now(state, content, &run, sc) {
        return Err(bad("that is not on offer now"));
    }
    let here = state
        .player
        .as_ref()
        .map(|p| p.location)
        .ok_or_else(|| bad("no player"))?;
    let at = super::place(&run, sc.def).ok_or_else(|| bad("that is not on offer now"))?;
    if here != at {
        return Err(bad("that is held somewhere else: go there first"));
    }
    if !crate::arcs::beats::has_time(state, sc.def.time) {
        return Err(bad("not enough time left this turn"));
    }
    let p = &sc.params;
    let built = crate::condition::on_state(state, content, |v| {
        let groups = audience::bind(v, &run, sc.def, p);
        if groups.is_empty() {
            return None;
        }
        let opp = attacks::opponent(v, p, &run, sc.def);
        let deck = attacks::deck(v, &state.events, p, &run, sc.def, &groups, &opp);
        let (answers, answers_left) =
            attacks::answers(v, &state.events, p, &run, sc.def, &groups, &opp);
        let prompt = attacks::draw(
            v, p, &run, &sc.bref, sc.def, &groups, &deck, &[], &opp, 1,
        );
        let aired = airs(&prompt, &opp);
        Some((
            ArcExchangeState {
                beat: sc.bref.clone(),
                on: v.date(),
                groups,
                round: 1,
                prompt: Some(prompt),
                deck,
                answers,
                answers_left,
                retired: Vec::new(),
                used: Vec::new(),
                table: Vec::new(),
                rounds: Vec::new(),
            },
            aired,
        ))
    });
    let (xs, aired) = built.ok_or_else(|| bad("nobody is there to hear it"))?;
    crate::arcs::beats::spend(state, sc.def.time);
    if let Some(r) = state.arcs.run_mut(id) {
        r.exchange = Some(xs);
    }
    if let Some((effect, cause)) = aired {
        crate::arcs::effects::apply_now(state, content, vec![effect], cause);
    }
    let mut res = ActionResult {
        outcome_key: format!("{}-open", sc.def.text),
        ..Default::default()
    };
    res.args.extend(text_args(state, &run));
    res.args.insert("run".into(), id.0.to_string());
    res.args.insert("beat".into(), sc.bref.key());
    Ok(res)
}

/// The causes of a round: the player's act (or their letting it pass),
/// the round before it, and the event its prompt came from.
fn round_causes(
    log: &EventLog,
    xs: &ArcExchangeState,
    prompt: &PromptNow,
    said: bool,
) -> Causes {
    let first: Causes = if said {
        Cause::Player(PlayerAct::Exchange).into()
    } else {
        [
            Cause::Process(Process::Arcs),
            Cause::Player(PlayerAct::LetPass),
        ]
        .into()
    };
    let anchor = prompt.attack.anchor.filter(|e| log.get(*e).is_some());
    first
        .and_event(xs.rounds.last().map(|r| r.event), Link::Continues)
        .and_event(anchor, Link::Prompted)
}

fn round_event(
    run: &ArcRun,
    sc: &Scene<'_>,
    round: u8,
    prompt: &PromptNow,
    o: Option<(&Opt, bool)>,
) -> EventKind {
    EventKind::ArcExchange {
        arc: run.id.0,
        template: sc.arc.id.clone(),
        stage: sc.bref.stage.clone(),
        beat: sc.bref.beat.clone(),
        at: run.at,
        round,
        source: prompt.attack.source.key().to_string(),
        option: o.map(|x| x.0.id.clone()).unwrap_or_default(),
        stood: o.is_some_and(|x| x.1),
    }
}

fn say(
    state: &mut State,
    content: &ContentBundle,
    id: ArcId,
    sc: &Scene<'_>,
    option: &str,
) -> Result<ActionResult, EngineError> {
    let mut run = state.arcs.run(id).ok_or_else(|| bad("no such arc"))?.clone();
    let mut xs = run
        .exchange
        .take()
        .filter(|x| x.beat == sc.bref)
        .ok_or_else(|| bad("the scene has not begun"))?;
    let p = &sc.params;
    let date = state.calendar.date;
    let seed = state.meta.seed;
    let prompt = xs.prompt.clone().ok_or_else(|| bad("the scene is over"))?;
    // The same build the view makes: an option must be one it lists.
    let opts = moves::options(state, content, p, &run, sc.arc, sc.def, &xs);
    let r: RoundIn = crate::condition::on_state(state, content, |v| {
        moves::round_in(v, p, &run, sc.def, &xs)
    })
    .ok_or_else(|| bad("the scene is over"))?;
    let weights: Vec<f64> = xs.groups.iter().map(|g| f64::from(g.weight)).collect();
    let shown = moves::shown(p, &r, &weights, &opts);
    let o = shown
        .iter()
        .map(|i| &opts[*i])
        .find(|o| o.id == option)
        .ok_or_else(|| bad("that line is not on offer"))?
        .clone();
    // Shown = rolled: the one function, on the true numbers, then the
    // keyed draw at the chance it shows.
    let (kr, kl) = landing::known(&r, &o.line);
    let l = landing::land(p, &kr, &kl);
    let key = mandate_core::hash::derive_u64(&o.id, &[]);
    let stood = l.p >= 1.0
        || attacks::keyed(seed, KEY_LINE_HOLDS, &run, &sc.bref, xs.round, key).chance(l.p);
    let deltas: Vec<f32> = if stood { &l.stands } else { &l.fails }
        .iter()
        .map(|d| *d as f32)
        .collect();
    for (g, d) in xs.groups.iter_mut().zip(&deltas) {
        g.lean = (g.lean + d).clamp(-1.0, 1.0);
    }
    let answered = stood && kl.answer.value > 0.0;
    if answered {
        xs.answers_left = xs.answers_left.saturating_sub(1);
    }
    let variants = i64::from(p.say_variants.max(1));
    let variant =
        attacks::keyed(seed, KEY_SAY, &run, &sc.bref, xs.round, key).range_i64(1, variants) as u8;
    let causes = round_causes(&state.events, &xs, &prompt, true);
    let ev = state.events.record(
        date,
        round_event(&run, sc, xs.round, &prompt, Some((&o, stood))),
        causes,
    );
    // The line's own deeds: the run's, then requests to the owners.
    let mut requests: Vec<Effect> = Vec::new();
    let me = crate::characters::player_character(state);
    let mut promise: Option<(String, Option<String>)> = None;
    let mut draft: Option<String> = None;
    let mut slot = false;
    for d in &o.deeds {
        match d {
            Deed::Promise { template, answer } => promise = Some((template.clone(), answer.clone())),
            Deed::Draft { template } => draft = Some(template.clone()),
            Deed::Standing { delta } => requests.push(Effect::PlayerTrack {
                standing: *delta as f32,
                viability: 0.0,
                money: 0.0,
                legal_floor: None,
            }),
            Deed::Pressed { person } => {
                if let Some(me) = me {
                    requests.push(Effect::Opinion {
                        holder: *person,
                        toward: me,
                        delta: p.world.pressed,
                        memory: Some(MemoryKind::PressedMe),
                    });
                }
            }
            Deed::Surface { hidden } => {
                // A claim that did not stand up is not believed: the fact
                // stays where it was.
                if stood {
                    requests.push(Effect::Surface {
                        hidden: *hidden,
                        by: me,
                    });
                }
                run.set_mark("aired");
            }
            Deed::Slot => slot = true,
            Deed::Retire => {}
            Deed::Cool { by } => {
                if stood {
                    run.add_heat(-(*by as f32));
                }
            }
        }
    }
    if let Some(m) = o.kind.mark().filter(|_| o.kind != MoveKind::Turn) {
        run.set_mark(m);
    }
    // Owning a broken promise disarms "another promise" (§6.1).
    if o.kind == MoveKind::Own && prompt.attack.source == SourceKind::BrokenPromise {
        xs.answers.retain(|a| a.kind != MoveKind::Promise);
    }
    // No attack is put twice, and a subject is said once.
    let akey = prompt.attack.key();
    if !xs.retired.contains(&akey) {
        xs.retired.push(akey);
    }
    if !o.kind.always_answers() {
        xs.used.push(o.id.clone());
    }
    xs.rounds.push(RoundPlayed {
        prompt,
        option: o.id.clone(),
        kind: Some(o.kind),
        variant,
        stood,
        answered,
        deltas,
        event: ev,
    });
    let last = xs.round >= sc.def.rounds(p);
    let round = xs.round;
    let mut next_aired: Option<(Effect, Cause)> = None;
    if !last {
        xs.round += 1;
        let next = crate::condition::on_state(state, content, |v| {
            let opp = attacks::opponent(v, p, &run, sc.def);
            let prompt = attacks::draw(
                v, p, &run, &sc.bref, sc.def, &xs.groups, &xs.deck, &xs.retired, &opp, xs.round,
            );
            (airs(&prompt, &opp), prompt)
        });
        next_aired = next.0;
        xs.prompt = Some(next.1);
        run.exchange = Some(xs);
        if let Some(r) = state.arcs.run_mut(id) {
            *r = run.clone();
        }
    } else {
        xs.prompt = None;
        xs.round += 1;
        finish(state, content, id, sc, run.clone(), xs, false);
    }
    // Requests go to their owners, with the round's event as their cause.
    let cause = Cause::caused_by(ev);
    crate::arcs::effects::apply_now(state, content, requests, cause);
    if let Some((template, answer)) = promise {
        crate::arcs::effects::apply_now(
            state,
            content,
            vec![Effect::Promise { template, answer }],
            cause,
        );
    }
    if let Some(template) = draft {
        // The path a dilemma's `draft` takes: the policy system's own
        // function, with its defaults.
        state.events.note_effect(&cause);
        let _ = crate::actions::draft(state, content, &template, &Default::default());
    }
    if slot {
        crate::arcs::beats::spend(state, mandate_content::visits::VisitTime::Slot);
    }
    if let Some((effect, c)) = next_aired {
        crate::arcs::effects::apply_now(state, content, vec![effect], c);
    }
    let run_now = state.arcs.run(id).cloned().unwrap_or(run);
    let mut res = ActionResult {
        outcome_key: format!("{}-say-{}-{variant}", sc.def.text, o.kind.key()),
        ..Default::default()
    };
    res.args.extend(text_args(state, &run_now));
    res.args.extend(o.args.iter().cloned());
    res.args.insert("option".into(), o.id.clone());
    res.args.insert("round".into(), round.to_string());
    res.args
        .insert("stood".into(), if stood { "yes" } else { "no" }.into());
    if last && let Some(done) = run_now.played.last() {
        res.args.insert("outcome".into(), done.outcome.key().into());
    }
    Ok(res)
}

fn leave(
    state: &mut State,
    content: &ContentBundle,
    id: ArcId,
    sc: &Scene<'_>,
) -> Result<ActionResult, EngineError> {
    let mut run = state.arcs.run(id).ok_or_else(|| bad("no such arc"))?.clone();
    let xs = run
        .exchange
        .take()
        .filter(|x| x.beat == sc.bref)
        .ok_or_else(|| bad("the scene has not begun"))?;
    finish(state, content, id, sc, run.clone(), xs, false);
    let run_now = state.arcs.run(id).cloned().unwrap_or(run);
    let mut res = ActionResult {
        outcome_key: format!("{}-left", sc.def.text),
        ..Default::default()
    };
    res.args.extend(text_args(state, &run_now));
    if let Some(done) = run_now.played.last() {
        res.args.insert("outcome".into(), done.outcome.key().into());
    }
    Ok(res)
}

/// A round nobody answered, worked out and not yet recorded.
pub(crate) struct Unanswered {
    pub prompt: PromptNow,
    pub round: u8,
    pub deltas: Vec<f32>,
}

/// The rounds left with no line (the player left, the turn ended, or they
/// never came): each prompt bites in full. With `absent` the first is
/// cold whatever was rehearsed, and the scene starts from nothing. The
/// other side spends no secret on an empty chair.
pub(crate) fn unanswered(
    v: &View<'_>,
    sc: &Scene<'_>,
    run: &ArcRun,
    xs: &mut ArcExchangeState,
    absent: bool,
) -> Vec<Unanswered> {
    let p = &sc.params;
    let opp = attacks::opponent(v, p, run, sc.def);
    let rounds = sc.def.rounds(p);
    let mut out = Vec::new();
    while xs.round <= rounds {
        let prompt = match xs.prompt.take() {
            Some(x) => x,
            None => {
                let deck: Vec<_> = xs
                    .deck
                    .iter()
                    .filter(|a| !matches!(a.subject.what, What::Hidden { .. }))
                    .cloned()
                    .collect();
                attacks::draw(
                    v, p, run, &sc.bref, sc.def, &xs.groups, &deck, &xs.retired, &opp, xs.round,
                )
            }
        };
        xs.prompt = Some(prompt.clone());
        let Some(mut r) = moves::round_in(v, p, run, sc.def, xs) else {
            break;
        };
        if absent && xs.round == 1 {
            r.cold = p.cold;
        }
        let deltas: Vec<f32> = landing::unanswered(p, &r).iter().map(|d| *d as f32).collect();
        for (g, d) in xs.groups.iter_mut().zip(&deltas) {
            g.lean = (g.lean + d).clamp(-1.0, 1.0);
        }
        let key = prompt.attack.key();
        if !xs.retired.contains(&key) {
            xs.retired.push(key);
        }
        out.push(Unanswered {
            prompt,
            round: xs.round,
            deltas,
        });
        xs.prompt = None;
        xs.round += 1;
    }
    out
}

/// A scene as it starts for an empty chair: the audience and the deck as
/// they would have been (no hidden fact: it is kept for a room).
pub(crate) fn empty_chair(
    v: &View<'_>,
    log: &EventLog,
    sc: &Scene<'_>,
    run: &ArcRun,
) -> Option<ArcExchangeState> {
    let p = &sc.params;
    let groups = audience::bind(v, run, sc.def, p);
    if groups.is_empty() {
        return None;
    }
    let opp = attacks::opponent(v, p, run, sc.def);
    let deck = attacks::deck(v, log, p, run, sc.def, &groups, &opp);
    Some(ArcExchangeState {
        beat: sc.bref.clone(),
        on: v.date(),
        groups,
        round: 1,
        prompt: None,
        deck,
        answers: Vec::new(),
        answers_left: 0,
        retired: Vec::new(),
        used: Vec::new(),
        table: Vec::new(),
        rounds: Vec::new(),
    })
}

/// What ending a scene asks for (§5.5), before its closing event exists.
pub(crate) struct Closed {
    pub outcome: Outcome,
    pub result: f64,
    pub requests: Vec<Effect>,
    pub event: EventKind,
}

/// The end (§5.4, §5.5): the result, the requests to the owners, and what
/// the outcome does to the run (its marks, its heat, its gated effects:
/// the run is changed here; nothing names a stage or an exit).
pub(crate) fn close(
    v: &View<'_>,
    sc: &Scene<'_>,
    run: &mut ArcRun,
    xs: &ArcExchangeState,
    absent: bool,
) -> Closed {
    let p = &sc.params;
    let ended: Vec<Ended> = xs
        .groups
        .iter()
        .map(|g| Ended {
            weight: f64::from(g.weight),
            start: f64::from(g.start),
            lean: f64::from(g.lean),
            head: !g.group.crowd(),
        })
        .collect();
    let (scored, result) = landing::outcome(p, &ended, sc.def.count.as_ref().map(|c| c.needs));
    let outcome = if absent { Outcome::Absent } else { scored };
    let me = v.me();
    let mut requests: Vec<Effect> = Vec::new();
    for g in &xs.groups {
        let change = f64::from(g.lean - g.start);
        match g.group.person() {
            // A crowd: approval by its change in lean, and recognition for
            // having been seen (an empty chair is not seen).
            None => {
                for (scope, classes, communities) in audience::scopes(&g.group) {
                    if change != 0.0 {
                        requests.push(Effect::Approval {
                            scope,
                            delta: p.world.approve * change,
                            classes: classes.clone(),
                            communities: communities.clone(),
                        });
                    }
                    if !absent && p.world.aware > 0.0 {
                        requests.push(Effect::Awareness {
                            scope,
                            delta: p.world.aware,
                            classes,
                            communities,
                        });
                    }
                }
            }
            // A member or a person: opinion by their change in lean.
            Some(holder) => {
                if let Some(toward) = me
                    && change != 0.0
                {
                    requests.push(Effect::Opinion {
                        holder,
                        toward,
                        delta: p.world.opinion * change,
                        memory: None,
                    });
                }
            }
        }
    }
    // A backer who spoke has spent their name if the scene was lost.
    let seconded = xs.rounds.iter().any(|r| r.kind == Some(MoveKind::Second));
    if seconded
        && outcome == Outcome::Lost
        && let (Some(b), Some(toward)) = (
            sc.def.sides.second.as_deref().and_then(|r| run.person(r)),
            me,
        )
    {
        requests.push(Effect::Opinion {
            holder: b,
            toward,
            delta: p.world.second_lost,
            memory: None,
        });
    }
    // The outcome's own marks, heat and gated effects, then the beat's.
    let own = sc
        .def
        .outcome
        .get(&outcome)
        .map(|o| (o.sets.as_deref(), o.heat, o.effects.as_slice()));
    requests.extend(step::complete(v, run, sc.beat, own, true));
    run.mark_done(sc.bref.clone(), v.date());
    let event = EventKind::ArcExchangeEnded {
        arc: run.id.0,
        template: sc.arc.id.clone(),
        stage: sc.bref.stage.clone(),
        beat: sc.bref.beat.clone(),
        at: run.at,
        outcome: outcome.key().to_string(),
        result_pm: (result * 1000.0).round() as i32,
        facts: cast::facts(v, sc.arc, run),
    };
    Closed {
        outcome,
        result,
        requests,
        event,
    }
}

/// A scene's end, worked out and not yet recorded: the rounds nobody
/// answered and what closing it asks for.
pub(crate) struct Plan {
    pub left: Vec<Unanswered>,
    pub closed: Closed,
}

/// Work out a scene's end: forfeit the rounds left, then close it. The
/// run's marks, heat and done list change here; nothing is recorded.
pub(crate) fn plan(
    v: &View<'_>,
    log: &EventLog,
    sc: &Scene<'_>,
    run: &mut ArcRun,
    xs: &mut ArcExchangeState,
    absent: bool,
) -> Plan {
    let mut left = unanswered(v, sc, run, xs, absent);
    // An attack names its event only while the log keeps it.
    for u in &mut left {
        u.prompt.attack.anchor = u.prompt.attack.anchor.filter(|e| log.get(*e).is_some());
    }
    let closed = close(v, sc, run, xs, absent);
    Plan { left, closed }
}

/// Record a planned end (each forfeited round, then the closing event)
/// and put the scene on the run. Returns the requests for other domains
/// and the closing event, their cause.
pub(crate) fn commit(
    plan: Plan,
    sc: &Scene<'_>,
    run: &mut ArcRun,
    mut xs: ArcExchangeState,
    on: mandate_core::SimDate,
    record: &mut dyn FnMut(EventKind, Causes) -> EventId,
) -> (Vec<Effect>, EventId) {
    for u in plan.left {
        let causes: Causes = [
            Cause::Process(Process::Arcs),
            Cause::Player(PlayerAct::LetPass),
        ]
        .into();
        let causes = causes
            .and_event(xs.rounds.last().map(|r| r.event), Link::Continues)
            .and_event(u.prompt.attack.anchor, Link::Prompted);
        let ev = record(round_event(run, sc, u.round, &u.prompt, None), causes);
        xs.rounds.push(RoundPlayed {
            prompt: u.prompt,
            option: String::new(),
            kind: None,
            variant: 0,
            stood: false,
            answered: false,
            deltas: u.deltas,
            event: ev,
        });
    }
    let said = xs.rounds.iter().any(|r| r.kind.is_some());
    let causes: Causes = if said {
        Cause::Player(PlayerAct::Exchange).into()
    } else {
        [
            Cause::Process(Process::Arcs),
            Cause::Player(PlayerAct::LetPass),
        ]
        .into()
    };
    let causes = causes.and_event(xs.rounds.last().map(|r| r.event), Link::Caused);
    let ev = record(plan.closed.event, causes);
    run.exchange = None;
    run.played.push(ExchangePlayed {
        beat: sc.bref.clone(),
        on,
        outcome: plan.closed.outcome,
        result: plan.closed.result as f32,
        groups: xs.groups,
        rounds: xs.rounds,
        event: ev,
    });
    (plan.closed.requests, ev)
}

/// End a scene from a command: forfeit what is left, close it, record it
/// and apply what it asks for.
fn finish(
    state: &mut State,
    content: &ContentBundle,
    id: ArcId,
    sc: &Scene<'_>,
    mut run: ArcRun,
    mut xs: ArcExchangeState,
    absent: bool,
) {
    let date = state.calendar.date;
    let planned = crate::condition::on_state(state, content, |v| {
        plan(v, &state.events, sc, &mut run, &mut xs, absent)
    });
    let (requests, ev) = commit(planned, sc, &mut run, xs, date, &mut |kind, causes| {
        state.events.record(date, kind, causes)
    });
    if let Some(r) = state.arcs.run_mut(id) {
        *r = run;
    }
    crate::arcs::effects::apply_now(state, content, requests, Cause::caused_by(ev));
}
