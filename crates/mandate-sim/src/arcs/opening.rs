//! Arriving into an opening (DEC-101 §2 and §3; STAGED_ARCS §6.6; task
//! A8): the frame, and the start of play on the opening the draft chose.
//!
//! An opening is a town, an arc template and how far the world was
//! advanced before the draft found it ([`OpeningChoice`]). Play begins
//! there by one rule, for the real game, its replays and every fork of the
//! audition alike ([`open`]):
//!
//! 1. the world is advanced `months_ahead` steps of
//!    [`super::audition::SCAN_STEP_DAYS`] with nobody playing (the
//!    audition's forward scan: the same days, the same draws);
//! 2. the frame ([`arrive`]): the player is seated afresh in the town by
//!    the start rule (`player::start`), in the seat for the district the
//!    frame names (`frame.post.seat_for`), and the arcs are asked to raise
//!    the template's run there ([`Effect::OpenArc`]);
//! 3. one day is stepped, on which the arcs raise it as their weekly step
//!    would, its first stage entered and its beats offered: the beats are
//!    on offer on turn 1 (the first-beat guarantee, [`first_beat`]).
//!
//! None of it is a command: the command log is empty after it, and a run
//! file records the choice (`mandate-save`'s `NewGameSetup::opening`), not
//! the days.
//!
//! What a frame leaves behind of the world before it: runs of the arcs in
//! jurisdictions the player no longer follows end without a word (nobody
//! played them), and the decisions and asks they had opened with them.

use super::audition::SCAN_STEP_DAYS;
use super::{ArcActor, ArcId, ArcRun, beats, cast};
use crate::condition::Args;
use crate::politics::Post;
use crate::scheduler::{Effect, Request};
use crate::spine::{Cause, PlayerAct};
use crate::state::State;
use crate::world::{Area, Jurisdiction, MunicipalityId, NeighborhoodId};
use crate::{Command, Engine};
use mandate_content::ContentBundle;
use mandate_content::arcs::ArcDef;
use serde::{Deserialize, Serialize};

/// The opening a new game starts on (the draft's choice; part of the run
/// file, so the same seed and choice give the same game).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpeningChoice {
    /// Steps of `SCAN_STEP_DAYS` the world is advanced first.
    pub months_ahead: u32,
    /// The municipality's id.
    pub town: u32,
    pub template: String,
}

/// What the frame did, read as play begins: day-0 facts.
#[derive(Clone, Debug, PartialEq)]
pub struct Arrival {
    /// The opening's run (none: it could not be raised).
    pub run: Option<ArcId>,
    /// The post the player holds.
    pub post: Option<Post>,
    /// A beat of the run is on offer on turn 1, and the player knows of it.
    pub first_beat: bool,
    /// The pitch's arguments: the run's text arguments now.
    pub args: Args,
}

/// Advance the world `months` steps of [`SCAN_STEP_DAYS`], as the
/// audition's forward scan did. Not logged.
pub fn advance(engine: &mut Engine, months: u32) -> Result<(), String> {
    let log = engine.command_log().to_vec();
    for _ in 0..months {
        engine
            .apply(Command::AdvanceDays {
                days: SCAN_STEP_DAYS,
            })
            .map_err(|e| e.to_string())?;
    }
    engine.restore_log(log);
    Ok(())
}

/// The neighbourhood a frame's seat is for: the area its `seat_for` role
/// is bound to, in a run already live there, else as a run would be cast
/// now.
fn seat_for(
    state: &State,
    content: &ContentBundle,
    def: &ArcDef,
    at: Jurisdiction,
) -> Option<NeighborhoodId> {
    let role = &def.frame.as_ref()?.post.as_ref()?.seat_for;
    let hood_of = |c: &[super::ArcCastEntry]| {
        c.iter()
            .find(|x| x.role == *role)
            .and_then(|x| match x.actor {
                ArcActor::Area {
                    area: Area::Hood { id },
                } => Some(id),
                _ => None,
            })
    };
    if let Some(r) = state.arcs.at(at).find(|r| r.template == def.id) {
        return hood_of(&r.cast);
    }
    let bound = crate::condition::on_state(state, content, |v| cast::cast(v, at, def))?;
    hood_of(&bound)
}

/// Arrive in `town` on the opening of `template` (the frame): the player
/// seated, the run raised on the one day stepped. Not logged.
pub fn arrive(
    engine: &mut Engine,
    town: MunicipalityId,
    template: &str,
) -> Result<Arrival, String> {
    let content = engine.content_arc();
    let def = content
        .arc(template)
        .ok_or_else(|| format!("no arc `{template}`"))?;
    let at = Jurisdiction::town(town);
    let near = seat_for(engine.state(), &content, def, at);
    let mut seated = Ok(());
    engine.setup(|s| {
        seated = crate::player::start::seat_in(s, &content, town, near);
        if seated.is_ok() {
            forget_elsewhere(s);
        }
    });
    seated?;
    if !engine.state().arcs.at(at).any(|r| r.template == def.id) {
        engine.seed_request(Request {
            effect: Effect::OpenArc {
                template: def.id.clone(),
                at,
            },
            cause: Cause::Player(PlayerAct::Start),
        });
    }
    let log = engine.command_log().to_vec();
    engine
        .apply(Command::AdvanceDays { days: 1 })
        .map_err(|e| e.to_string())?;
    engine.restore_log(log);
    engine.setup(|s| {
        // The act's record begins where the player does (DEC-006), and the
        // first turn's day played follows the day stepped.
        s.act.records.clear();
        crate::legacy::begin(s, &content);
        s.calendar.day = crate::characters::calendar::choose_day(s, &content);
    });
    let st = engine.state();
    let run = st.arcs.at(at).find(|r| r.template == def.id);
    Ok(Arrival {
        run: run.map(|r| r.id),
        post: crate::player::posts(st).first().cloned(),
        first_beat: run.is_some_and(|r| first_beat(st, &content, r)),
        args: run
            .map(|r| pitch_args(st, &content, def, r))
            .unwrap_or_default(),
    })
}

/// Advance and arrive: the start of play on a chosen opening.
pub fn open(engine: &mut Engine, choice: &OpeningChoice) -> Result<Arrival, String> {
    advance(engine, choice.months_ahead)?;
    arrive(engine, MunicipalityId(choice.town), &choice.template)
}

/// What was put to whoever the player was before lapses (they are someone
/// else now, and nobody answered it), except what a run the new player
/// follows opened; runs in jurisdictions the player no longer follows end
/// without a word: nobody played them.
fn forget_elsewhere(s: &mut State) {
    let Some(focus) = s.player.as_ref().map(|p| p.focus()) else {
        return;
    };
    s.arcs.runs.retain(|r| focus.contains(&r.at));
    let today = s.calendar.date;
    let runs: Vec<ArcId> = s.arcs.runs.iter().map(|r| r.id).collect();
    let kept = |a: &Option<super::ArcRef>| a.as_ref().is_some_and(|x| runs.contains(&x.run));
    for d in s.narrative.decisions.iter_mut() {
        if d.status == crate::narrative::DecisionStatus::Pending && !kept(&d.arc) {
            d.status = crate::narrative::DecisionStatus::Lapsed;
        }
    }
    for a in s.asks.asks.iter_mut() {
        if a.open() && !kept(&a.arc) {
            a.status = crate::asks::AskStatus::Declined {
                on: today,
                unanswered: true,
            };
        }
    }
}

/// Whether a beat of the run is on offer now, and the player knows of it:
/// its stage is in the open (the break never is hidden), or a decision or
/// an ask of it is put to the player (the pocket's rule for a Today item).
pub fn first_beat(state: &State, content: &ContentBundle, run: &ArcRun) -> bool {
    let known = content
        .arc(&run.template)
        .and_then(|d| d.stage(&run.stage))
        .is_some_and(|s| !s.hidden)
        || !run.open.is_empty();
    known
        && beats::offers(state, content)
            .iter()
            .any(|o| o.run == run.id)
}

/// The run's text arguments as they stand: its facts and dates, where it
/// runs (`place`), the player's home (`town`) and its cast's names.
pub fn pitch_args(state: &State, content: &ContentBundle, def: &ArcDef, run: &ArcRun) -> Args {
    let mut args: Args = crate::condition::on_state(state, content, |v| cast::facts(v, def, run))
        .into_iter()
        .collect();
    args.insert(
        "place".into(),
        cast::jurisdiction_name(&state.world, run.at),
    );
    if let Some(home) = crate::player::home(state) {
        args.insert(
            "town".into(),
            cast::jurisdiction_name(&state.world, Jurisdiction::town(home)),
        );
    }
    for (role, name) in cast::names(&state.politics, &state.world, &run.cast) {
        args.insert(role, name);
    }
    args
}
