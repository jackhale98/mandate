//! The asks' own commands: answering an offer (`RespondAsk`, free), making a
//! campaign promise (`Promise`, free, while campaigning, at most
//! `max_per_campaign` a race), and casework ("Sort it out" at the town
//! hall: `Action::Casework`, one slot). They run in the Commands phase on
//! the whole state; the effects they cause are the same requests the
//! system makes, applied at once ([`super::effects::apply_now`]).

use super::effects::{self, Outcome};
use super::offers;
use super::{Ask, AskId, AskStatus, Asker, Deed, History, KEY_CASEWORK};
use crate::actions::ActionResult;
use crate::commands::EngineError;
use crate::events::EventKind;
use crate::narrative::cast::View;
use crate::politics::Post;
use crate::scheduler::Reads;
use crate::scheduler::Request;
use crate::spine::{Cause, PlayerAct};
use crate::state::State;
use mandate_content::ContentBundle;
use mandate_content::asks::{PromiseAnswers, PromiseDue};
use mandate_core::{SimDate, Stream};
use serde::{Deserialize, Serialize};

fn bad(msg: &str) -> EngineError {
    EngineError::InvalidCommand(msg.into())
}

/// Answer an offer: accept it (a promise to the asker) or decline it.
pub fn respond(
    state: &mut State,
    content: &ContentBundle,
    ask: u32,
    accept: bool,
) -> Result<ActionResult, EngineError> {
    let id = AskId(ask);
    let date = state.calendar.date;
    let a = state.asks.get(id).ok_or_else(|| bad("no such ask"))?;
    if !a.offered() {
        return Err(bad("that ask has already been answered"));
    }
    let a = a.clone();
    let me = crate::characters::player_character(state);
    let fx = if accept {
        effects::promised(a.asker_person(), me)
    } else {
        let r = Reads::of_state(state, content);
        let defs = super::resolve::template_effects(&r, &a, Outcome::Declined);
        effects::requests(&r, &a, &defs)
    };
    // The player's answer is the cause of what follows from it.
    let cause = Cause::Player(PlayerAct::RespondAsk);
    effects::apply_now(
        state,
        content,
        fx.into_iter()
            .map(|effect| Request { effect, cause })
            .collect(),
    );
    let mut res = ActionResult::default();
    let name_key = content
        .ask(&a.template)
        .map(|t| t.name_key.clone())
        .unwrap_or_default();
    if let Some(x) = state.asks.get_mut(id) {
        if accept {
            x.status = AskStatus::Accepted { on: date };
        } else {
            x.status = AskStatus::Declined {
                on: date,
                unanswered: false,
            };
        }
    }
    if accept {
        state.asks.tally.accepted += 1;
        state.events.record(
            date,
            EventKind::AskAccepted {
                ask,
                template: a.template.clone(),
                big: a.big,
            },
            cause,
        );
        res.outcome_key = "outcome-ask-accepted".into();
    } else {
        state.asks.tally.declined += 1;
        state.events.record(
            date,
            EventKind::AskResolved {
                ask,
                template: a.template.clone(),
                outcome: "declined".into(),
                public: a.public,
            },
            cause,
        );
        res.outcome_key = "outcome-ask-declined".into();
    }
    res.created = Some(ask);
    res.args.insert("ask_key".into(), name_key);
    Ok(res)
}

/// The odds a piece of casework is sorted (shown before acting): in
/// office, `success_in_office`; out of it, the player can only chase it up.
pub fn casework_odds(state: &State, content: &ContentBundle) -> f64 {
    let c = &content.ask_params.casework;
    if crate::player::posts(state).is_empty() {
        c.success_out_of_office
    } else {
        c.success_in_office
    }
}

/// Whether "Sort it out" can be done for an ask now: an accepted ask whose
/// next need is casework, or a fault still waiting in the repair queue and
/// not moved up yet (DEC-114). Checked before the slot is spent.
pub fn can_casework(state: &State, ask: u32) -> bool {
    state.asks.get(AskId(ask)).is_some_and(|a| {
        a.accepted()
            && match a.current() {
                Some(super::Need::Casework) => true,
                Some(super::Need::Fixed { fault }) => state
                    .upkeep
                    .get(*fault)
                    .is_some_and(|f| f.open() && f.expedited.is_none()),
                _ => false,
            }
    })
}

/// "Sort it out": the roll of an `Action::Casework` (its slot is spent by
/// the caller). A success is a deed (the asks system keeps the ask); a
/// failure leaves it open for another try. On a fault (DEC-114, the owner's
/// answer 3) a success moves the fault up its repair queue instead: it does
/// not buy the repair, and the ask is kept when the fault is fixed.
pub fn casework(
    state: &mut State,
    content: &ContentBundle,
    ask: u32,
) -> Result<ActionResult, EngineError> {
    let id = AskId(ask);
    if !can_casework(state, ask) {
        return Err(bad("there is no casework to sort out for that ask"));
    }
    let a = state.asks.get(id).ok_or_else(|| bad("no such ask"))?;
    let fault = match a.current() {
        Some(super::Need::Fixed { fault }) => Some(*fault),
        _ => None,
    };
    let tries = a.tries;
    let odds = casework_odds(state, content);
    let ok = mandate_core::rng::keyed(
        state.meta.seed,
        Stream::Asks,
        &[KEY_CASEWORK, ask as u64, tries as u64],
    )
    .chance(odds);
    if let Some(x) = state.asks.get_mut(id) {
        x.tries = x.tries.saturating_add(1);
    }
    let mut res = ActionResult::default();
    if ok && let Some(f) = fault {
        super::record_deed(state, Deed::Casework { ask: id });
        effects::apply_now(
            state,
            content,
            vec![Request {
                effect: crate::scheduler::Effect::Expedite { fault: f.0 },
                cause: Cause::Player(PlayerAct::Casework),
            }],
        );
        res.outcome_key = "outcome-casework-queued".into();
    } else if ok {
        super::record_deed(state, Deed::Casework { ask: id });
        res.outcome_key = "outcome-casework-sorted".into();
    } else {
        res.outcome_key = "outcome-casework-chased".into();
    }
    if let Some(p) = state
        .asks
        .get(id)
        .and_then(|a| a.asker_person())
        .and_then(|p| crate::characters::profile(&state.politics, &state.world, p))
    {
        res.args.insert("name".into(), p.full_name());
    }
    res.args
        .insert("odds".into(), format!("{}", (odds * 100.0).round() as u32));
    res.created = Some(ask);
    Ok(res)
}

/// One promise the player could make now (the promise menu, E7).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromiseOption {
    pub template: String,
    /// The chosen answer (a policy template), for a promise on the crisis.
    pub answer: Option<String>,
    /// The worry it answers, if one.
    pub concern: Option<String>,
}

/// The race the player is running in: its post, election day and
/// neighbourhoods.
fn my_race(state: &State) -> Option<(Post, SimDate, Vec<u32>)> {
    let me = state.player.as_ref()?.politician;
    state
        .politics
        .elections
        .races
        .iter()
        .find(|r| r.candidates.iter().any(|c| c.politician == me))
        .map(|r| (r.seat, r.election_day, r.hoods.clone()))
}

/// The felt level of a worry across neighbourhoods (mean; 0..1).
fn felt(state: &State, hoods: &[u32], concern: &str) -> f64 {
    let Some(i) = state
        .population
        .concern_keys
        .iter()
        .position(|k| k == concern)
    else {
        return 0.0;
    };
    let levels: Vec<f64> = hoods
        .iter()
        .filter_map(|h| state.population.hood_concerns.get(*h as usize))
        .filter_map(|c| c.levels.get(i).map(|x| *x as f64))
        .collect();
    if levels.is_empty() {
        0.0
    } else {
        levels.iter().sum::<f64>() / levels.len() as f64
    }
}

/// The race's top worries (measured across its constituency:
/// `concerns::ranked`), strongest first.
fn top_worries(state: &State, content: &ContentBundle, hoods: &[u32], n: usize) -> Vec<String> {
    let hoods: Vec<usize> = hoods.iter().map(|h| *h as usize).collect();
    let ranking = crate::concerns::ranked(
        &state.population,
        content,
        &state.world,
        crate::concerns::Scope::District(&hoods),
        mandate_content::condition::Worry::Measured,
    );
    crate::concerns::top(&ranking, n)
        .filter_map(|(i, _)| content.concerns.get(i).map(|c| c.id.clone()))
        .collect()
}

/// Promises made in this race already (same template and answer).
fn promised_here(state: &State, race: Post, template: &str, answer: Option<&str>) -> bool {
    state.asks.asks.iter().any(|a| {
        a.promise && a.race == Some(race) && a.template == template && a.answer.as_deref() == answer
    })
}

/// The promise menu (ENGAGEMENT §3.5): while campaigning, built from the
/// live state of the constituency the player is running in: its top
/// worries, the town's crisis (each answer to it), and promises of presence
/// and honesty. Empty when not campaigning or after `max_per_campaign`.
pub fn promise_menu(state: &State, content: &ContentBundle) -> Vec<PromiseOption> {
    let Some((race, _, hoods)) = my_race(state) else {
        return Vec::new();
    };
    if state.player.as_ref().is_none_or(|p| !p.status.active()) {
        return Vec::new();
    }
    let made = if state.asks.promises_race == Some(race) {
        state.asks.promises_this_campaign
    } else {
        0
    };
    if made >= content.ask_params.promises.max_per_campaign {
        return Vec::new();
    }
    let r = Reads::of_state(state, content);
    let v = View::new(&r, &state.politics);
    let top = top_worries(state, content, &hoods, 3);
    let mut out = Vec::new();
    for t in &content.promises {
        let options: Vec<PromiseOption> = match &t.answers {
            PromiseAnswers::Crisis => match offers::live_crisis(&r) {
                Some(kind) => super::resolve::crisis_answers(content, &kind)
                    .into_iter()
                    .filter(|p| content.policies.contains_key(p))
                    .map(|p| PromiseOption {
                        template: t.id.clone(),
                        answer: Some(p),
                        concern: content
                            .crises
                            .iter()
                            .find(|c| c.id == kind)
                            .and_then(|c| c.concern.clone()),
                    })
                    .collect(),
                None => Vec::new(),
            },
            PromiseAnswers::Concern { concern } if top.contains(concern) => vec![PromiseOption {
                template: t.id.clone(),
                answer: None,
                concern: Some(concern.clone()),
            }],
            PromiseAnswers::Concern { .. } => Vec::new(),
            PromiseAnswers::Any => vec![PromiseOption {
                template: t.id.clone(),
                answer: None,
                concern: None,
            }],
        };
        for o in options {
            if promised_here(state, race, &o.template, o.answer.as_deref()) {
                continue;
            }
            if offers::needs(&r, &v, &t.needs, &[], Some((&hoods, o.answer.as_deref()))).is_none() {
                continue;
            }
            out.push(o);
        }
    }
    out
}

/// When a promise falls due.
fn promise_due(
    state: &State,
    content: &ContentBundle,
    due: &PromiseDue,
    race: Post,
    election: SimDate,
) -> SimDate {
    match due {
        PromiseDue::Term => {
            let years = content
                .constitutions
                .get(&state.meta.constitution)
                .and_then(|c| crate::politics::elections::calendar_for(c, race))
                .map(|(_, t)| t)
                .unwrap_or(4);
            election.add_days(years * mandate_core::time::DAYS_PER_YEAR)
        }
        PromiseDue::BudgetYear => {
            let y = election.calendar().year;
            SimDate::from_ymd(y + 2, 1, 1)
        }
        PromiseDue::Days { days } => election.add_days(*days),
    }
}

/// Make a campaign promise: a public ask from the voters of the
/// constituency the player is running in, kept or broken by the deadline
/// of the post run for. It persuades where its worry is felt (less for each
/// public promise broken, nothing for one already kept last term).
pub fn promise(
    state: &mut State,
    content: &ContentBundle,
    template: &str,
    args: &[(String, String)],
) -> Result<ActionResult, EngineError> {
    let answer = args
        .iter()
        .find(|(k, _)| k == mandate_content::asks::ROLE_ANSWER)
        .map(|(_, v)| v.clone());
    make_promise(
        state,
        content,
        template,
        answer,
        true,
        Cause::Player(PlayerAct::Promise),
    )
}

/// A promise made in front of a room (DEC-112, `Effect::Promise`): the
/// same promise, from the same menu, with its deadline, its keep and its
/// break, accepted on the spot. The scene's audience heard it, and their
/// lean is its persuasion: the menu's own door-to-door persuasion is not
/// added on top. `cause`: the round it was made in.
pub fn promise_on_stage(
    state: &mut State,
    content: &ContentBundle,
    template: &str,
    answer: Option<&str>,
    cause: Cause,
) -> Result<ActionResult, EngineError> {
    make_promise(
        state,
        content,
        template,
        answer.map(String::from),
        false,
        cause,
    )
}

fn make_promise(
    state: &mut State,
    content: &ContentBundle,
    template: &str,
    answer: Option<String>,
    persuade: bool,
    cause: Cause,
) -> Result<ActionResult, EngineError> {
    let (race, election, hoods) =
        my_race(state).ok_or_else(|| bad("promises are made while campaigning"))?;
    let menu = promise_menu(state, content);
    let option = menu
        .iter()
        .find(|o| o.template == template && o.answer == answer)
        .cloned()
        .ok_or_else(|| bad("that promise is not on offer"))?;
    let t = content
        .promise(template)
        .ok_or_else(|| bad("no such promise"))?;
    let date = state.calendar.date;
    let needs = {
        let r = Reads::of_state(state, content);
        let v = View::new(&r, &state.politics);
        offers::needs(&r, &v, &t.needs, &[], Some((&hoods, answer.as_deref())))
            .ok_or_else(|| bad("that promise cannot be kept now"))?
    };
    let due = promise_due(state, content, &t.due, race, election);
    let asker = match race {
        Post::Seat {
            constituency: Some(id),
            ..
        } => Asker::Constituency { id },
        other => Asker::Electorate { at: other.at() },
    };
    // Persuasion where its worry is felt (ENGAGEMENT §3.5, §4.7).
    let p = &content.ask_params.promises;
    let kept_before = state.asks.asks.iter().any(|a| {
        a.promise
            && a.template == template
            && a.answer == answer
            && a.race != Some(race)
            && matches!(a.status, AskStatus::Kept { .. })
    });
    let discount = p
        .broken_discount
        .powi(state.asks.broken_public_promises() as i32);
    if !kept_before && persuade {
        for h in &hoods {
            let salience = match &option.concern {
                Some(c) => felt(state, &[*h], c),
                None => p.any_salience,
            };
            let delta = p.persuasion * salience * discount;
            if delta > 0.0 {
                crate::population::apply_approval(
                    &mut state.population,
                    &state.world,
                    crate::world::Area::Hood {
                        id: crate::world::NeighborhoodId(*h),
                    },
                    delta,
                    &[],
                );
            }
        }
    }
    let a = &mut state.asks;
    if a.promises_race != Some(race) {
        a.promises_race = Some(race);
        a.promises_this_campaign = 0;
    }
    a.promises_this_campaign += 1;
    let id: AskId = a.ids.alloc();
    a.asks.push(Ask {
        id,
        template: template.to_string(),
        promise: true,
        asker,
        cast: Vec::new(),
        needs,
        step: 0,
        done: Vec::new(),
        public: true,
        big: false,
        opened: date,
        respond_by: date,
        due,
        status: AskStatus::Accepted { on: date },
        history: History::None,
        race: Some(race),
        answer: answer.clone(),
        tries: 0,
        arc: None,
    });
    a.tally.promises_made += 1;
    state.events.record(
        date,
        EventKind::AskAccepted {
            ask: id.0,
            template: template.to_string(),
            big: false,
        },
        cause,
    );
    let mut res = ActionResult {
        created: Some(id.0),
        outcome_key: "outcome-promised".into(),
        ..Default::default()
    };
    res.args.insert("promise_key".into(), t.name_key.clone());
    if let Some(x) = answer.as_deref().and_then(|x| content.policies.get(x)) {
        res.args.insert("answer_key".into(), x.name_key.clone());
    }
    Ok(res)
}

/// Promise outcomes for the run's record are `AskState::promise_outcomes`.
pub fn kept_or_broken(a: &Ask) -> Option<bool> {
    match a.status {
        AskStatus::Kept { .. } => Some(true),
        AskStatus::Broken { .. } => Some(false),
        _ => None,
    }
}
