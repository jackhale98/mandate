//! Fundraising, the press, the bribe and the leak are conversations
//! (DEC-114, B5): each verb's effect equals the one action's in a twin
//! (same world, the one-action path against the conversation, the same
//! state change but for the conversation's own record); the odds shown are
//! the odds rolled; the risk lines are shown before choosing; the old `Act`
//! is refused and routing makes it the conversation (what bots, stand-ins
//! and the time-skip proxy send); every key named exists; and a game that
//! does all four is deterministic and survives a save.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_core::Stream;
use mandate_save::{RunMode, SaveRequest, decode, encode};
use mandate_sim::actions::verbs::{self, Verb};
use mandate_sim::actions::{self, Action, PlayerCommand};
use mandate_sim::asks::Deed;
use mandate_sim::characters::CharacterId;
use mandate_sim::events::EventKind;
use mandate_sim::player::knowledge::Subject;
use mandate_sim::talk::{TalkTarget, hooks};
use mandate_sim::{Command, Engine, State};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

fn content() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn game(seed: u64, c: &Arc<ContentBundle>) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

/// A game a few turns in, with the player's own bill before the council
/// (something to talk to the press about).
fn with_bill(seed: u64, c: &Arc<ContentBundle>) -> (Engine, u32) {
    let mut e = game(seed, c);
    let template = c
        .policies
        .values()
        .find(|t| t.tier == mandate_content::TierKey::Local && t.unlock.is_none() && !t.retired)
        .unwrap()
        .id
        .clone();
    let r = e
        .apply(e.route(PlayerCommand::DraftPolicy {
            template,
            params: Default::default(),
        }))
        .unwrap();
    let bill = r.result.unwrap().created.unwrap();
    e.apply(e.route(PlayerCommand::IntroducePolicy { policy: bill }))
        .unwrap();
    e.apply(Command::EndTurn).unwrap();
    (e, bill)
}

/// What each verb is, as the one action and late or not.
fn plans(bill: u32) -> Vec<(Action, bool)> {
    vec![
        (Action::Press { policy: None }, false),
        (Action::Press { policy: Some(bill) }, false),
        (Action::Fundraise, false),
        (Action::Fundraise, true),
        (Action::TakeBribe, true),
        (Action::LeakFile, false),
    ]
}

/// The state where the verb is done, and the conversation that does it.
fn at_place(
    st: &State,
    c: &ContentBundle,
    action: &Action,
    late: bool,
) -> (State, PlayerCommand, CharacterId) {
    let verb = Verb::of_action(action).unwrap();
    let at = verbs::places_for(st, c, verb.key(), late)[0];
    let mut s = st.clone();
    actions::apply(&mut s, c, &PlayerCommand::MoveTo { location: at }).unwrap();
    let cmd = verbs::as_talk(
        &s,
        c,
        &PlayerCommand::Act {
            action: action.clone(),
            late_night: late,
        },
    )
    .expect("a conversation for the verb");
    let PlayerCommand::Talk {
        to: TalkTarget::Person { person },
        ..
    } = &cmd
    else {
        panic!("already there: the verb is a talk, not a walk: {cmd:?}");
    };
    let person = *person;
    (s, cmd, person)
}

/// The conversation's own record put aside: the talk log, the action log,
/// what the player knows of the person and who they saw where.
fn without_conversation(a: &State, b: &mut State, speaker: CharacterId) {
    let (pa, pb) = (a.player.as_ref().unwrap(), b.player.as_mut().unwrap());
    pb.talk = pa.talk.clone();
    pb.actions = pa.actions.clone();
    pb.knowledge.seen = pa.knowledge.seen.clone();
    let person = Subject::Person { person: speaker };
    match pa.knowledge.entries.get(&person) {
        Some(e) => {
            pb.knowledge.entries.insert(person, e.clone());
        }
        None => {
            pb.knowledge.entries.remove(&person);
        }
    }
    b.politics.characters.edges = a.politics.characters.edges.clone();
    // Talking to someone is a deed of its own (DEC-097 §2): the one more
    // event, and its line in the asks' deed log.
    let talk = |k: &EventKind| matches!(k, EventKind::PlayerDeed { deed: Deed::Talk { person, .. } } if *person == speaker);
    assert_eq!(b.events.len(), a.events.len() + 1);
    assert!(b.events.iter().last().is_some_and(|e| talk(&e.kind)));
    b.events = a.events.clone();
    b.asks
        .deeds
        .retain(|d| !matches!(&d.deed, Deed::Talk { person, .. } if *person == speaker));
    b.asks.last_event = a.asks.last_event;
}

/// Each verb's effect equals the one action's: the same world, the one
/// action against the conversation, the same state but for the
/// conversation's record (and the relationships it touches, compared
/// apart: nothing but familiarity moves).
#[test]
fn each_verb_equals_the_one_action_in_a_twin() {
    let c = content();
    for seed in [1u64, 7, 12] {
        let (e, bill) = with_bill(seed, &c);
        for (action, late) in plans(bill) {
            let (st, cmd, speaker) = at_place(e.state(), &c, &action, late);
            let mut a = st.clone();
            let ra = verbs::one_action(&mut a, &c, &action, late).unwrap();
            mandate_sim::asks::settle_now(&mut a, &c);
            let mut b = st.clone();
            let rb = actions::apply(&mut b, &c, &cmd).unwrap();
            // The conversation names the same result (a leak's said as the
            // reporter's), with the act's arguments.
            let key = rb.outcome_key.replace("leak-talk", "leak-file");
            assert_eq!(key, ra.outcome_key, "seed {seed} {action:?}");
            if let Some(amount) = ra.args.get("amount") {
                assert_eq!(rb.args.get("amount"), Some(amount));
            }
            // The relationships: only familiarity may move, and only the
            // conversation's sightings move it.
            let (ea, eb) = (&a.politics.characters.edges, &b.politics.characters.edges);
            for (k, x) in eb {
                match ea.get(k) {
                    Some(y) => {
                        let mut x = x.clone();
                        x.familiar = y.familiar;
                        assert_eq!(*y, x, "seed {seed} {action:?}: edge {k:?}");
                    }
                    None => assert!(x.familiar > 0, "seed {seed} {action:?}: new edge {k:?}"),
                }
            }
            assert!(ea.keys().all(|k| eb.contains_key(k)));
            without_conversation(&a, &mut b, speaker);
            if a.hash() != b.hash() {
                let (va, vb) = (
                    serde_json::to_value(&a).unwrap(),
                    serde_json::to_value(&b).unwrap(),
                );
                let diff: Vec<String> = va
                    .as_object()
                    .unwrap()
                    .iter()
                    .filter(|(k, v)| vb.get(k.as_str()) != Some(*v))
                    .map(|(k, _)| k.clone())
                    .collect();
                for k in &diff {
                    let (x, y) = (va[k.as_str()].to_string(), vb[k.as_str()].to_string());
                    let i = x.chars().zip(y.chars()).take_while(|(p, q)| p == q).count();
                    let at = |s: &str| {
                        s.chars()
                            .skip(i.saturating_sub(300))
                            .take(900)
                            .collect::<String>()
                    };
                    eprintln!("{k}:\n A {}\n B {}", at(&x), at(&y));
                }
                panic!("seed {seed} {action:?}: the states differ in {diff:?}");
            }
        }
    }
}

/// The odds shown are the odds rolled (DEC-073): the press's chance it
/// goes well is one less the gaffe risk, and the conversation gaffes
/// exactly when the player's stream, drawn at that risk, says so.
#[test]
fn the_press_rolls_the_odds_it_shows() {
    let c = content();
    let mut gaffes = 0;
    let mut n = 0;
    for seed in 1..=12u64 {
        let e = game(seed, &c);
        let (st, cmd, speaker) = at_place(e.state(), &c, &Action::Press { policy: None }, false);
        let choice = hooks::offered(&st, &c, speaker)
            .into_iter()
            .find(|x| x.id == verbs::choice_for(&Action::Press { policy: None }, false).unwrap())
            .unwrap();
        let risk = actions::press_gaffe_risk(&st, &c);
        assert_eq!(choice.odds, Some(1.0 - risk));
        // The risk line carries over.
        assert!(choice.costs.iter().any(|(k, a)| k == "press-cost-gaffe"
            && a.contains(&("risk".into(), format!("{}", (risk * 100.0).round() as i64)))));
        let mut probe = st.clone();
        let gaffe = probe
            .rng
            .get(Stream::Player)
            .chance(1.0 - choice.odds.unwrap());
        let mut s = st.clone();
        let r = actions::apply(&mut s, &c, &cmd).unwrap();
        assert_eq!(r.outcome_key == "outcome-press-gaffe", gaffe, "seed {seed}");
        gaffes += gaffe as u32;
        n += 1;
    }
    assert!(n == 12 && gaffes < n);
}

/// What each choice shows before it is chosen: its time, the press's gaffe
/// chance, fundraising's range and donor ties (late: legal risk and the
/// exposure line), and the shady acts' payoff, legal risk and exposure.
#[test]
fn the_risk_lines_are_shown() {
    let c = content();
    let (e, bill) = with_bill(1, &c);
    let want: [(&Action, bool, &[&str]); 6] = [
        (
            &Action::Press { policy: None },
            false,
            &["verb-cost-slot", "press-cost-gaffe"],
        ),
        (
            &Action::Press { policy: Some(bill) },
            false,
            &["verb-cost-slot", "press-cost-gaffe"],
        ),
        (
            &Action::Fundraise,
            false,
            &[
                "verb-cost-slot",
                "verb-cost-health",
                "fundraise-cost-raise",
                "fundraise-cost-ties",
            ],
        ),
        (
            &Action::Fundraise,
            true,
            &[
                "verb-cost-late",
                "fundraise-cost-raise",
                "verb-cost-legal",
                "verb-cost-exposure",
            ],
        ),
        (
            &Action::TakeBribe,
            true,
            &[
                "verb-cost-late",
                "verb-cost-pays",
                "verb-cost-legal",
                "verb-cost-exposure",
            ],
        ),
        (
            &Action::LeakFile,
            false,
            &["verb-cost-slot", "verb-cost-legal", "verb-cost-exposure"],
        ),
    ];
    for (action, late, keys) in want {
        let (st, _, speaker) = at_place(e.state(), &c, action, late);
        let id = verbs::choice_for(action, late).unwrap();
        let choice = hooks::offered(&st, &c, speaker)
            .into_iter()
            .find(|x| x.id == id)
            .unwrap_or_else(|| panic!("{id} offered"));
        let shown: Vec<&str> = choice.costs.iter().map(|(k, _)| k.as_str()).collect();
        for k in keys {
            assert!(shown.contains(k), "{id}: {k} in {shown:?}");
        }
        assert!(
            mandate_sim::talk::check_choice(&st, &c, speaker, &id, late).is_ok(),
            "{id}"
        );
    }
}

/// Every key the four conversations name exists in English: labels, lines,
/// replies, costs, outcomes and the Why.
#[test]
fn every_key_named_exists() {
    let c = content();
    let en: BTreeSet<String> = mandate_content::validate::fluent_keys(&c.locale_text("en"));
    let (e, bill) = with_bill(3, &c);
    let mut seen = 0;
    for (action, late) in plans(bill) {
        let (st, cmd, speaker) = at_place(e.state(), &c, &action, late);
        let id = verbs::choice_for(&action, late).unwrap();
        let choice = hooks::offered(&st, &c, speaker)
            .into_iter()
            .find(|x| x.id == id)
            .unwrap();
        let mut keys = vec![choice.label_key.clone(), choice.say_key.clone()];
        keys.extend(choice.replies.iter().map(|r| r.key.clone()));
        keys.extend(choice.costs.iter().map(|(k, _)| k.clone()));
        keys.extend(choice.costs.iter().flat_map(|(_, a)| {
            a.iter()
                .filter(|(k, _)| k.ends_with("_key"))
                .map(|(_, v)| v.clone())
        }));
        let mut s = st.clone();
        let r = actions::apply(&mut s, &c, &cmd).unwrap();
        keys.push(r.outcome_key.clone());
        let why = verbs::why_key(&id, &r.outcome_key).expect("a Why for every result");
        keys.push(why);
        for k in keys {
            assert!(en.contains(&k), "{id}: missing `{k}`");
        }
        seen += 1;
    }
    assert_eq!(seen, 6);
}

/// The one-tap `Act` is refused for the player; routing it makes it the
/// conversation with whoever the verb is had with, which is what the bots,
/// the stand-ins and the time-skip proxy send, and it does the verb.
#[test]
fn routing_makes_the_act_a_conversation() {
    let c = content();
    let (mut e, bill) = with_bill(2, &c);
    for (action, late) in plans(bill) {
        let act = PlayerCommand::Act {
            action: action.clone(),
            late_night: late,
        };
        let mut s = e.state().clone();
        assert!(
            actions::apply(&mut s, &c, &act).is_err(),
            "{action:?} as one tap"
        );
        let routed = e.route(act);
        let Command::Player(p) = &routed else {
            panic!()
        };
        let inner = match p {
            PlayerCommand::Go { then: Some(t), .. } => t.as_ref(),
            x => x,
        };
        assert!(
            matches!(inner, PlayerCommand::Talk { topic, .. } if Verb::of_topic(topic).is_some())
        );
        if late && e.state().player.as_ref().unwrap().late_night_used {
            continue;
        }
        if e.state().player.as_ref().unwrap().slots_left() == 0 {
            e.apply(Command::EndTurn).unwrap();
        }
        let routed = e.route(PlayerCommand::Act {
            action: action.clone(),
            late_night: late,
        });
        e.apply(routed)
            .unwrap_or_else(|x| panic!("{action:?}: {x:?}"));
    }
}

/// A game that does all four verbs in conversation is the same twice, and
/// the same after a save and a load.
#[test]
fn deterministic_and_saved() {
    let c = content();
    let play = || {
        let (mut e, bill) = with_bill(5, &c);
        for (action, late) in plans(bill) {
            if e.state().player.as_ref().unwrap().slots_left() == 0 {
                e.apply(Command::EndTurn).unwrap();
            }
            let cmd = e.route(PlayerCommand::Act {
                action,
                late_night: late,
            });
            let _ = e.apply(cmd);
        }
        e
    };
    let (a, b) = (play(), play());
    assert_eq!(a.state_hash(), b.state_hash());
    let req = SaveRequest {
        run_id: "00112233445566778899aabbccddeeff".into(),
        revision: 1,
        parent_revision: None,
        device_id: "test-device".into(),
        saved_at: "2026-01-01T00:00:00Z".into(),
        protocol_version: 1,
        run_mode: RunMode::Ironman,
    };
    let (bytes, _) = encode(a.state(), &req).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    assert_eq!(state.hash(), a.state_hash());
}
