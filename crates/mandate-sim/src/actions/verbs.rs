//! Fundraising, the press, the bribe and the leak are conversations
//! (DEC-114, B5; BEING_THERE §2), as lobbying became one (M6 note 29). The
//! effect and the time stay the one action's (`publicity`, `misconduct`,
//! `tuning.action_costs`); the way in is a person, somewhere:
//!
//! - **The press:** an interview with a journalist or the editor of the
//!   town's paper, at the news office (or the town hall's press room), on
//!   nothing in particular or on a bill; the gaffe chance is shown
//!   (`press_gaffe_risk`), and the choice's odds are the chance it goes well.
//! - **Fundraising:** asking a donor (the party chair, a business leader, a
//!   plant manager, the hotel's manager, a tech founder) where money is
//!   raised; late, over dinner, with the late-night risk shown.
//! - **The bribe:** the envelope comes from whoever is offering (`briber`),
//!   in the diner's back booth late at night, with its risk line.
//! - **The leak:** a rival's file slipped to a reporter at the news office,
//!   with its risk line.
//!
//! Each is a talk hook (S4) with its own owner (`press`, `fundraise`,
//! `bribe`, `leak`); a choice costs [`ChoiceCost::Verb`]: the verb's own
//! slots and health, paid by the effect itself, where the verb is offered.
//! Whom it is had with comes from content (`actions.talks.<verb>.with`, the
//! Named people of that occupation in the player's home town); they see
//! the player where it is done, so the choice is `remote` (no walk to
//! them). [`one_action`] is the effect as the one action had it, which the
//! choice calls; the command and the routing ([`as_talk`]: an `Act` for
//! one of these is had as the conversation with [`partner_at`]) mean the
//! bots, the stand-ins and the time-skip proxy do the verbs as the player
//! does. Facts and Fluent keys only; no random number is drawn here.

use super::*;
use crate::characters::CharacterId;
use crate::places::{Location, Offer};
use crate::talk::hooks::{self, Choice, ChoiceCost, Done, Hook, Reply};

/// One of the four verbs had in a conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Press,
    Fundraise,
    Bribe,
    Leak,
}

impl Verb {
    pub const ALL: [Verb; 4] = [Verb::Press, Verb::Fundraise, Verb::Bribe, Verb::Leak];

    /// The action key (`tuning.action_costs`, places, `actions.talks`).
    pub fn key(self) -> &'static str {
        match self {
            Verb::Press => "press",
            Verb::Fundraise => "fundraise",
            Verb::Bribe => "take_bribe",
            Verb::Leak => "leak_file",
        }
    }

    /// The hook's owner: the `<owner>` in `choice.<owner>.<local>`.
    pub fn owner(self) -> &'static str {
        match self {
            Verb::Press => "press",
            Verb::Fundraise => "fundraise",
            Verb::Bribe => "bribe",
            Verb::Leak => "leak",
        }
    }

    /// The action it is with nothing in particular (the press: on no bill).
    pub fn action(self) -> Action {
        match self {
            Verb::Press => Action::Press { policy: None },
            Verb::Fundraise => Action::Fundraise,
            Verb::Bribe => Action::TakeBribe,
            Verb::Leak => Action::LeakFile,
        }
    }

    /// The verb an action key is, if it is one of these.
    pub fn of_key(key: &str) -> Option<Verb> {
        Verb::ALL.into_iter().find(|v| v.key() == key)
    }

    /// The verb an action is, if it is one of these.
    pub fn of_action(a: &Action) -> Option<Verb> {
        match a {
            Action::Press { .. } => Some(Verb::Press),
            Action::Fundraise => Some(Verb::Fundraise),
            Action::TakeBribe => Some(Verb::Bribe),
            Action::LeakFile => Some(Verb::Leak),
            _ => None,
        }
    }

    /// The verb a `Talk` topic is a choice of, if any.
    pub fn of_topic(topic: &str) -> Option<Verb> {
        let owner = topic.strip_prefix(hooks::PREFIX)?.split('.').next()?;
        Verb::ALL.into_iter().find(|v| v.owner() == owner)
    }
}

/// The press hook (registered in `talk::hooks::HOOKS`).
pub const PRESS: Hook = Hook {
    owner: "press",
    offer: offer_press,
    chosen,
};
/// The fundraising hook.
pub const FUNDRAISE: Hook = Hook {
    owner: "fundraise",
    offer: offer_fundraise,
    chosen,
};
/// The bribe's hook.
pub const BRIBE: Hook = Hook {
    owner: "bribe",
    offer: offer_bribe,
    chosen,
};
/// The leak's hook.
pub const LEAK: Hook = Hook {
    owner: "leak",
    offer: offer_leak,
    chosen,
};

fn offer_press(state: &State, content: &ContentBundle, speaker: CharacterId) -> Vec<Choice> {
    offer(Verb::Press, state, content, speaker)
}
fn offer_fundraise(state: &State, content: &ContentBundle, speaker: CharacterId) -> Vec<Choice> {
    offer(Verb::Fundraise, state, content, speaker)
}
fn offer_bribe(state: &State, content: &ContentBundle, speaker: CharacterId) -> Vec<Choice> {
    offer(Verb::Bribe, state, content, speaker)
}
fn offer_leak(state: &State, content: &ContentBundle, speaker: CharacterId) -> Vec<Choice> {
    offer(Verb::Leak, state, content, speaker)
}

/// The `Talk` topic that does an action in conversation: the action and
/// whether it is done late (`None`: not one of these verbs, or a late
/// variant the verb has not).
pub fn choice_for(action: &Action, late_night: bool) -> Option<String> {
    let verb = Verb::of_action(action)?;
    let local = match (action, late_night) {
        (Action::Press { policy: None }, false) => "general".to_string(),
        (Action::Press { policy: Some(p) }, false) => p.to_string(),
        (Action::Fundraise, false) => "day".into(),
        (Action::Fundraise, true) => "late".into(),
        (Action::TakeBribe, true) => "take".into(),
        (Action::LeakFile, false) => "file".into(),
        _ => return None,
    };
    Some(hooks::choice_id(verb.owner(), &local))
}

/// The action a choice does, and whether late (the inverse of
/// [`choice_for`]).
pub fn action_of(id: &str) -> Option<(Action, bool)> {
    let verb = Verb::of_topic(id)?;
    let local = id
        .strip_prefix(hooks::PREFIX)?
        .strip_prefix(verb.owner())?
        .strip_prefix('.')?;
    Some(match (verb, local) {
        (Verb::Press, "general") => (Action::Press { policy: None }, false),
        (Verb::Press, n) => (
            Action::Press {
                policy: Some(n.parse().ok()?),
            },
            false,
        ),
        (Verb::Fundraise, "day") => (Action::Fundraise, false),
        (Verb::Fundraise, "late") => (Action::Fundraise, true),
        (Verb::Bribe, "take") => (Action::TakeBribe, true),
        (Verb::Leak, "file") => (Action::LeakFile, false),
        _ => return None,
    })
}

/// Everyone the verb can be had with now, by id: whoever is offering (the
/// bribe), else the Named people of `actions.talks.<verb>.with` whose own
/// place is in the player's home town.
pub fn partners(state: &State, content: &ContentBundle, verb: Verb) -> Vec<CharacterId> {
    let Some(pl) = state.player.as_ref() else {
        return Vec::new();
    };
    let me = characters::player_character(state);
    if verb == Verb::Bribe {
        return briber(state).into_iter().collect();
    }
    let Some(talk) = content.actions.talks.of(verb.key()) else {
        return Vec::new();
    };
    let home = pl.home();
    state
        .politics
        .characters
        .people
        .iter()
        .filter(|c| Some(c.id) != me && c.politician.is_none())
        .filter(|c| {
            c.named.as_deref().is_some_and(|n| {
                talk.with.contains(&n.occupation)
                    && n.place
                        .and_then(|p| state.world.places.get(p.index()))
                        .is_some_and(|p| p.municipality == home)
            })
        })
        .map(|c| c.id)
        .collect()
}

/// The one the verb is had with at a location: a partner whose day has
/// them there now, else the first partner (they come to see the player).
pub fn partner_at(
    state: &State,
    content: &ContentBundle,
    verb: Verb,
    at: Location,
) -> Option<CharacterId> {
    let all = partners(state, content, verb);
    let here = characters::ambient::stored_at(state, content, at);
    all.iter()
        .find(|c| here.contains(c))
        .or_else(|| all.first())
        .copied()
}

/// Where the verb is done now, nearest first (the player's location if it
/// offers it).
pub fn places_for(state: &State, content: &ContentBundle, key: &str, late: bool) -> Vec<Location> {
    let mut locs = crate::places::locations_for(
        state,
        content,
        Offer::Action {
            key,
            late_night: late,
        },
        None,
    );
    if let Some(here) = state.player.as_ref().map(|p| p.location)
        && let Some(i) = locs.iter().position(|l| *l == here)
    {
        let h = locs.remove(i);
        locs.insert(0, h);
    }
    locs
}

/// The checks of a verb's choice that are not the conversation's: the
/// player is where it is done, and has its time (its slots, or the late
/// slot).
pub fn check_time(
    state: &State,
    content: &ContentBundle,
    key: &str,
    late: bool,
) -> Result<(), EngineError> {
    let pl = state.player.as_ref().ok_or_else(|| bad("no player"))?;
    if !places_for(state, content, key, late).contains(&pl.location) {
        return Err(bad("go to where it is done first"));
    }
    if late {
        if pl.late_night_used {
            return Err(bad("late night already used this turn"));
        }
    } else if pl.slots_left() < content.tuning.action_cost(key).slots {
        return Err(bad("not enough time left this turn"));
    }
    Ok(())
}

/// The verb's effect as the one action had it (what `Act` did before
/// B5): its time and health, the effect, and what doing it teaches of open
/// proposals (`policy::drafting`). The caller has checked where it is done.
/// The conversation's choice calls exactly this (the twin tests compare the
/// two).
pub fn one_action(
    state: &mut State,
    content: &ContentBundle,
    action: &Action,
    late_night: bool,
) -> Result<ActionResult, EngineError> {
    let pl = state
        .player
        .as_ref()
        .ok_or_else(|| bad("no player"))?
        .clone();
    let m = pl.municipality.0;
    if late_night && !matches!(action, Action::Fundraise | Action::TakeBribe) {
        return Err(bad(
            "only private meetings and fundraising happen late at night",
        ));
    }
    if !late_night && matches!(action, Action::TakeBribe) {
        return Err(bad("envelopes change hands late at night"));
    }
    let res = match action {
        Action::Press { policy } => publicity::press(state, content, &pl, m, policy)?,
        Action::Fundraise => publicity::fundraise(state, content, &pl, m, &late_night)?,
        Action::TakeBribe | Action::LeakFile => {
            let target = matches!(action, Action::LeakFile)
                .then(|| leak_target(state))
                .flatten();
            let res = misconduct::shady(state, content, action, late_night)?;
            if let Some(t) = target {
                crate::asks::record_deed(state, crate::asks::Deed::Leak { politician: t.0 });
            }
            res
        }
        _ => return Err(bad("that is not a verb of a conversation")),
    };
    crate::policy::drafting::learn(
        state,
        content,
        crate::policy::drafting::Moment::Did {
            action: action.key(),
            hood: None,
        },
    );
    Ok(res)
}

/// Auto-routing (DEC-079) of one of these verbs: an `Act` is had as the
/// conversation with whoever the verb is had with where it is done, walked
/// to first. `None`: not one of these, or nowhere or nobody to do it with
/// (the `Act` then stays, and is refused).
pub fn as_talk(
    state: &State,
    content: &ContentBundle,
    cmd: &PlayerCommand,
) -> Option<PlayerCommand> {
    let PlayerCommand::Act { action, late_night } = cmd else {
        return None;
    };
    let verb = Verb::of_action(action)?;
    let topic = choice_for(action, *late_night)?;
    let here = state.player.as_ref()?.location;
    let at = *places_for(state, content, verb.key(), *late_night).first()?;
    let person = partner_at(state, content, verb, at)?;
    let talk = PlayerCommand::Talk {
        to: crate::talk::TalkTarget::Person { person },
        topic,
        about: None,
        late_night: *late_night,
    };
    Some(if at == here {
        talk
    } else {
        PlayerCommand::Go {
            location: at,
            then: Some(Box::new(talk)),
        }
    })
}

/// The Why under a verb's result (DEC-114): a Fluent key that takes the
/// outcome's arguments, for a conversation's choice and its outcome.
pub fn why_key(topic: &str, outcome: &str) -> Option<String> {
    let (action, late) = action_of(topic)?;
    let key = match (&action, outcome) {
        (Action::Press { .. }, "outcome-press") => "why-press",
        (Action::Press { .. }, "outcome-press-gaffe") => "why-press-gaffe",
        (Action::Fundraise, "outcome-fundraise") if late => "why-fundraise-late",
        (Action::Fundraise, "outcome-fundraise") => "why-fundraise",
        (Action::TakeBribe, "outcome-take-bribe") => "why-take-bribe",
        (Action::LeakFile, "outcome-leak-talk-exposed") => "why-leak-file-exposed",
        (Action::LeakFile, "outcome-leak-talk-rumour") => "why-leak-file-rumour",
        _ => return None,
    };
    Some(key.into())
}

fn name(state: &State, c: CharacterId) -> String {
    characters::profile(&state.politics, &state.world, c)
        .map(|p| p.full_name())
        .unwrap_or_default()
}

fn pct(x: f64) -> String {
    format!("{}", (x * 100.0).round() as i64)
}

/// The money a fundraiser raises at the player's oratory: (low, high).
pub fn fundraise_range(state: &State, content: &ContentBundle) -> (i64, i64) {
    let oratory = state
        .player
        .as_ref()
        .and_then(|p| state.politics.get(p.politician))
        .map(|p| p.oratory as f64)
        .unwrap_or(0.5);
    let [lo, hi] = content.actions.fundraise.raised;
    let k = 0.5 + oratory;
    ((lo * k) as i64, (hi * k) as i64)
}

/// A hidden fact's risk line as shown before acting (DEC-073): its
/// severity and the coarse band of it coming out within a year.
fn exposure_args(
    state: &State,
    content: &ContentBundle,
    spec: &mandate_content::HiddenSpec,
) -> Vec<(String, String)> {
    let Some(me) = state.player.as_ref().map(|p| p.politician) else {
        return Vec::new();
    };
    let e = crate::media::exposure(state, content, spec, me);
    let p = crate::media::risk_within_horizon(content, &e);
    vec![
        (
            "risk_key".into(),
            format!("risk-{}", crate::media::band(content, p)),
        ),
        (
            "severity_key".into(),
            format!("severity-{}", spec.severity.key()),
        ),
    ]
}

/// What a choice costs, shown before it is chosen (Fluent keys and their
/// arguments): its time, and the risk lines the one action's form showed.
fn costs(
    state: &State,
    content: &ContentBundle,
    action: &Action,
    late: bool,
) -> Vec<(String, Vec<(String, String)>)> {
    let key = action.key();
    let cost = content.tuning.action_cost(key);
    let mut out: Vec<(String, Vec<(String, String)>)> = Vec::new();
    if late {
        let health = content.actions.late_night_health + cost.health.max(0.0);
        out.push((
            "verb-cost-late".into(),
            vec![("health".into(), format!("{}", health.round() as i64))],
        ));
    } else {
        match cost.slots {
            0 => {}
            1 => out.push(("verb-cost-slot".into(), Vec::new())),
            n => out.push((
                "verb-cost-slots".into(),
                vec![("slots".into(), n.to_string())],
            )),
        }
        if cost.health > 0.0 {
            out.push((
                "verb-cost-health".into(),
                vec![("health".into(), format!("{}", cost.health.round() as i64))],
            ));
        }
    }
    let legal = |x: f64| {
        (
            "verb-cost-legal".to_string(),
            vec![("legal".to_string(), format!("{x:.1}"))],
        )
    };
    match action {
        Action::Press { .. } => out.push((
            "press-cost-gaffe".into(),
            vec![("risk".into(), pct(press_gaffe_risk(state, content)))],
        )),
        Action::Fundraise => {
            let (low, high) = fundraise_range(state, content);
            out.push((
                "fundraise-cost-raise".into(),
                vec![
                    ("low".into(), low.to_string()),
                    ("high".into(), high.to_string()),
                ],
            ));
            out.push(("fundraise-cost-ties".into(), Vec::new()));
            if late {
                out.push(legal(content.actions.fundraise.late_night_legal));
                out.push((
                    "verb-cost-exposure".into(),
                    exposure_args(state, content, &content.media.late_night_fundraising),
                ));
            }
        }
        Action::TakeBribe | Action::LeakFile => {
            if let Some(def) = content.media.shady_actions.get(key) {
                if def.money > 0.0 {
                    out.push((
                        "verb-cost-pays".into(),
                        vec![("amount".into(), (def.money as i64).to_string())],
                    ));
                }
                if def.legal > 0.0 {
                    out.push(legal(def.legal));
                }
                out.push((
                    "verb-cost-exposure".into(),
                    exposure_args(state, content, &def.hidden),
                ));
            }
        }
        _ => {}
    }
    out
}

/// A choice of the verb with `speaker`, doing `action`.
fn choice(
    state: &State,
    content: &ContentBundle,
    verb: Verb,
    action: &Action,
    late: bool,
) -> Option<Choice> {
    let id = choice_for(action, late)?;
    let mut args: Vec<(String, String)> = Vec::new();
    let (label, say) = match action {
        Action::Press { policy: None } => ("press-choice-general", "press-say-general"),
        Action::Press { policy: Some(p) } => {
            let inst = state.policy.instances.get(*p as usize)?;
            let t = crate::policy::resolve(content, &state.policy, inst)?;
            args.push(("bill_key".into(), t.template().name_key.clone()));
            ("press-choice-bill", "press-say-bill")
        }
        Action::Fundraise if late => ("fundraise-choice-late", "fundraise-say-late"),
        Action::Fundraise => ("fundraise-choice-day", "fundraise-say-day"),
        Action::TakeBribe => ("bribe-choice", "bribe-say"),
        Action::LeakFile => {
            let r = leak_target(state)?;
            args.push((
                "rival".into(),
                state
                    .politics
                    .get(r)
                    .map(|p| p.full_name())
                    .unwrap_or_default(),
            ));
            ("leak-choice", "leak-say")
        }
        _ => return None,
    };
    let replies = content
        .actions
        .talks
        .of(verb.key())
        .map(|t| {
            t.replies
                .iter()
                .map(|r| Reply {
                    key: r.key.clone(),
                    when: r.when.clone(),
                    agrees: true,
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Choice {
        id,
        label_key: label.into(),
        say_key: say.into(),
        args,
        replies,
        cost: ChoiceCost::Verb {
            key: action.key(),
            late,
        },
        // The press: the chance it goes well (a gaffe is its complement,
        // shown as a cost and rolled as `press_gaffe_risk`).
        odds: matches!(action, Action::Press { .. })
            .then(|| 1.0 - press_gaffe_risk(state, content)),
        costs: costs(state, content, action, late),
        // They see the player where the verb is done (no walk to them).
        remote: true,
        bill: None,
    })
}

/// The bills a statement can be about (as the one action's form listed
/// them): the home town's bills before its body or in force.
fn press_bills(state: &State) -> Vec<u32> {
    let Some(home) = state.player.as_ref().map(|p| p.home()) else {
        return Vec::new();
    };
    state
        .policy
        .in_municipality(home.0)
        .filter(|p| {
            matches!(
                p.status,
                PolicyStatus::UnderConsideration | PolicyStatus::Enacted | PolicyStatus::InEffect
            )
        })
        .map(|p| p.id.0)
        .collect()
}

/// The verb's choices for a conversation with `speaker` now (read-only).
fn offer(verb: Verb, state: &State, content: &ContentBundle, speaker: CharacterId) -> Vec<Choice> {
    if !partners(state, content, verb).contains(&speaker) {
        return Vec::new();
    }
    let mut plans: Vec<(Action, bool)> = Vec::new();
    match verb {
        Verb::Press => {
            plans.push((Action::Press { policy: None }, false));
            for b in press_bills(state) {
                plans.push((Action::Press { policy: Some(b) }, false));
            }
        }
        Verb::Fundraise => {
            plans.push((Action::Fundraise, false));
            // Late, where money is raised late (the party office, the hotel).
            if !places_for(state, content, verb.key(), true).is_empty() {
                plans.push((Action::Fundraise, true));
            }
        }
        Verb::Bribe => plans.push((Action::TakeBribe, true)),
        Verb::Leak => {
            if leak_target(state).is_some() {
                plans.push((Action::LeakFile, false));
            }
        }
    }
    plans
        .iter()
        .filter_map(|(a, late)| choice(state, content, verb, a, *late))
        .collect()
}

/// The choice was made: the one action's effect, at the odds shown, with
/// the arguments its Why reads.
fn chosen(
    state: &mut State,
    content: &ContentBundle,
    choice: &Choice,
    speaker: CharacterId,
    _reply: &Reply,
) -> Done {
    let fail = || Done {
        outcome_key: "outcome-talk".into(),
        args: Vec::new(),
    };
    let Some((action, late)) = action_of(&choice.id) else {
        return fail();
    };
    // What the Why says, read before the act changes anything (the risk
    // lines and the range the choice showed).
    let mut why: Vec<(String, String)> = Vec::new();
    match &action {
        Action::Press { .. } => {
            let risk = press_gaffe_risk(state, content);
            // Shown = rolled (DEC-073): the odds shown are 1 − this risk.
            debug_assert!(choice.odds.is_none_or(|o| o == 1.0 - risk));
            why.push(("risk".into(), pct(risk)));
            why.extend(choice.args.iter().filter(|(k, _)| k == "bill_key").cloned());
        }
        Action::Fundraise => {
            let (low, high) = fundraise_range(state, content);
            why.push(("low".into(), low.to_string()));
            why.push(("high".into(), high.to_string()));
        }
        Action::TakeBribe | Action::LeakFile => {
            if let Some(def) = content.media.shady_actions.get(action.key()) {
                why.extend(exposure_args(state, content, &def.hidden));
            }
        }
        _ => {}
    }
    let Ok(res) = one_action(state, content, &action, late) else {
        return fail();
    };
    // `name` is whom it was had with; the act's own arguments follow (the
    // amount; a leak's rival is `rival`, its outcome said as the reporter's).
    let mut args: Vec<(String, String)> = vec![("name".into(), name(state, speaker))];
    for (k, v) in res.args {
        let k = match (&action, k.as_str()) {
            (Action::LeakFile, "name") => "rival".to_string(),
            _ => k,
        };
        args.retain(|(x, _)| *x != k);
        args.push((k, v));
    }
    let outcome_key = match res.outcome_key.as_str() {
        "outcome-leak-file-exposed" => "outcome-leak-talk-exposed".to_string(),
        "outcome-leak-file-rumour" => "outcome-leak-talk-rumour".to_string(),
        _ => res.outcome_key,
    };
    for (k, v) in why {
        if !args.iter().any(|(x, _)| *x == k) {
            args.push((k, v));
        }
    }
    Done { outcome_key, args }
}
