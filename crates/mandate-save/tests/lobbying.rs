//! Lobbying is a conversation (M6 note 29): the arguments a member offers
//! depend on what the player knows of them, and the plain pitch is always
//! there (DEC-091's principle); the odds shown are the odds rolled for every
//! argument (DEC-073); meeting after hours carries its legal risk; a member
//! who has committed is settled; an ally brings several; two exchanges a
//! member a week; NPCs lobby through the same function (DEC-030); and a
//! game with lobbying is deterministic and survives a save.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_core::Stream;
use mandate_save::{RunMode, SaveRequest, decode, encode};
use mandate_sim::actions::PlayerCommand;
use mandate_sim::actions::lobbying::{self, Argument};
use mandate_sim::characters::{self, Bond, CharacterId, Relationship};
use mandate_sim::player::knowledge::{self, Level, Subject};
use mandate_sim::politics::PoliticianId;
use mandate_sim::talk::TalkTarget;
use mandate_sim::world::{Jurisdiction, NeighborhoodId};
use mandate_sim::{Command, CommandOutcome, Engine, EngineError};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
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

/// A game with the player's own bill before the council.
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

fn me(e: &Engine) -> PoliticianId {
    e.state().player.as_ref().unwrap().politician
}

fn my_char(e: &Engine) -> CharacterId {
    characters::player_character(e.state()).unwrap()
}

/// The council's members other than the player, as (politician, character).
fn members(e: &Engine) -> Vec<(PoliticianId, CharacterId)> {
    let st = e.state();
    let at = Jurisdiction::town(st.player.as_ref().unwrap().home());
    mandate_sim::politics::body_members_at(&st.politics, at)
        .into_iter()
        .filter(|p| *p != me(e))
        .map(|p| (p, st.politics.get(p).unwrap().character))
        .collect()
}

/// The arguments offered to the player on `bill` with this person now.
fn offered(e: &Engine, person: CharacterId, bill: u32) -> Vec<Argument> {
    lobbying::arguments(e.state(), e.content(), person)
        .into_iter()
        .filter(|p| p.policy == bill)
        .map(|p| p.argument)
        .collect()
}

fn argue(
    e: &mut Engine,
    person: CharacterId,
    bill: u32,
    a: Argument,
) -> Result<CommandOutcome, EngineError> {
    e.apply(Command::Player(PlayerCommand::Talk {
        to: TalkTarget::Person { person },
        topic: lobbying::choice_id(bill, a),
        about: None,
        late_night: false,
    }))
}

fn outcome(o: &CommandOutcome) -> String {
    o.result.as_ref().unwrap().outcome_key.clone()
}

fn copy(e: &Engine) -> Engine {
    Engine::from_state(e.state().clone(), e.content_arc()).unwrap()
}

/// Forget everything the player knows (a newcomer who has met no one).
fn forget(e: &mut Engine) {
    e.setup(|st| st.player.as_mut().unwrap().knowledge.entries.clear());
}

fn teach(e: &mut Engine, s: Subject, level: Level) {
    e.setup(|st| {
        let src = knowledge::action_source(st, "lobby");
        knowledge::teach_one(st, s.clone(), level, src);
    });
}

#[test]
fn what_a_member_offers_depends_on_what_the_player_knows() {
    let c = content();
    let (mut e, bill) = with_bill(31, &c);
    forget(&mut e);
    let all = members(&e);
    for (p, person) in &all {
        let args = offered(&e, *person, bill);
        assert!(
            args.contains(&Argument::Plain),
            "the plain pitch is always there: {args:?}"
        );
        // Knowing nothing opens nothing that needs knowing.
        assert!(!args.contains(&Argument::Ward), "{args:?}");
        assert!(!args.contains(&Argument::Broker), "{args:?}");
        assert!(!args.contains(&Argument::CallIn), "no favour owed yet");
        // The odds are odds.
        for x in lobbying::arguments(e.state(), &c, *person) {
            assert!((0.0..=1.0).contains(&x.odds), "{x:?}");
        }
        let _ = p;
    }
    // Knowing a neighbourhood's worries in a member's ward opens arguing
    // from it; a favour they owe opens calling it in.
    let (p, person) = *all
        .iter()
        .find(|(p, _)| {
            e.state()
                .politics
                .get(*p)
                .unwrap()
                .role
                .constituency()
                .is_some()
        })
        .expect("a ward member");
    let ward = e
        .state()
        .politics
        .get(p)
        .unwrap()
        .role
        .constituency()
        .unwrap();
    let hood = mandate_sim::policy::ward_hoods(&e.state().world, ward)[0];
    teach(
        &mut e,
        Subject::HoodConcerns {
            hood: NeighborhoodId(hood as u32),
        },
        Level::Known,
    );
    let me = my_char(&e);
    e.setup(|st| {
        let d = st.calendar.date;
        characters::owe(&mut st.politics, person, me, "favour-bill", None, d);
    });
    let args = offered(&e, person, bill);
    assert!(args.contains(&Argument::Ward), "{args:?}");
    assert!(args.contains(&Argument::CallIn), "{args:?}");
    assert!(args.contains(&Argument::Plain), "{args:?}");
    // Each is its own argument: the ward's odds do not read their opinion
    // of the player, the plain pitch's do.
    let odds = |e: &Engine, a: Argument| {
        lobbying::pitch(e.state(), &c, self::me(e), p, bill, a, 1.0)
            .unwrap()
            .odds
    };
    let (plain, ward) = (odds(&e, Argument::Plain), odds(&e, Argument::Ward));
    e.setup(|st| {
        st.politics.characters.edges.insert(
            (person.0, me.0),
            Relationship {
                opinion: -80.0,
                trust: 0.2,
                bonds: Vec::new(),
                familiar: 0,
            },
        );
    });
    assert!(odds(&e, Argument::Plain) < plain);
    assert_eq!(odds(&e, Argument::Ward), ward);
}

#[test]
fn shown_equals_rolled_for_every_argument() {
    let c = content();
    let (base, bill) = with_bill(31, &c);
    let mut tried = std::collections::BTreeSet::new();
    let mut n = 0;
    for (i, (_, person)) in members(&base).into_iter().enumerate() {
        // An ally (bring others), and a favour owed (call it in).
        for shift in 0..12 {
            let mut e = copy(&base);
            let me = my_char(&e);
            e.setup(|st| {
                st.politics.characters.edges.insert(
                    (person.0, me.0),
                    Relationship {
                        opinion: 80.0,
                        trust: 0.8,
                        bonds: vec![Bond::Ally],
                        familiar: 0,
                    },
                );
                let d = st.calendar.date;
                characters::owe(&mut st.politics, person, me, "favour-bill", None, d);
                for _ in 0..(shift + i * 13) {
                    st.rng.get(Stream::Player).next_f64();
                }
            });
            let choices = mandate_sim::talk::hooks::offered(e.state(), &c, person);
            let lobby: Vec<_> = choices.iter().filter(|x| x.bill == Some(bill)).collect();
            let Some(choice) = lobby.get(shift % lobby.len().max(1)) else {
                continue;
            };
            let (_, a) = lobbying::parse_id(&choice.id).unwrap();
            let u = e.state().rng.clone().get(Stream::Player).next_f64();
            let o = argue(&mut e, person, bill, a).unwrap();
            let key = outcome(&o);
            match choice.odds {
                Some(p) => {
                    let won = key != "outcome-lobby-lost";
                    assert_eq!(won, u < p, "{}: shown {p}, drew {u}, {key}", choice.id);
                }
                // Cannot fail: certain, and nothing drawn.
                None => {
                    assert_ne!(key, "outcome-lobby-lost");
                    assert_eq!(e.state().rng.clone().get(Stream::Player).next_f64(), u);
                }
            }
            tried.insert(a);
            n += 1;
        }
    }
    assert!(n >= 30, "{n} arguments made");
    for a in [
        Argument::Plain,
        Argument::Rally,
        Argument::CallIn,
        Argument::Owe,
        Argument::AfterHours,
    ] {
        assert!(tried.contains(&a), "{a:?} tried: {tried:?}");
    }
}

#[test]
fn meeting_after_hours_carries_its_legal_risk() {
    let c = content();
    let (mut e, bill) = with_bill(31, &c);
    let (_, person) = members(&e)[0];
    let before = e.state().player.as_ref().unwrap().clone();
    let hidden = e.state().narrative.next_hidden;
    let p = lobbying::arguments(e.state(), &c, person)
        .into_iter()
        .find(|p| p.argument == Argument::AfterHours)
        .expect("after hours offered");
    assert!(p.late && p.legal > 0.0);
    assert_eq!(p.legal, c.actions.lobby.late_night_legal);
    // The late bonus: better odds than the plain pitch.
    let plain = lobbying::arguments(e.state(), &c, person)
        .into_iter()
        .find(|p| p.argument == Argument::Plain)
        .unwrap();
    assert!(p.odds >= plain.odds);
    argue(&mut e, person, bill, Argument::AfterHours).unwrap();
    let after = e.state().player.as_ref().unwrap();
    assert!(after.late_night_used);
    assert_eq!(
        after.slots_used, before.slots_used,
        "the late slot, not a day's"
    );
    assert!(after.tracks.legal_jeopardy > before.tracks.legal_jeopardy);
    assert!(after.tracks.health < before.tracks.health);
    assert_eq!(
        e.state().narrative.next_hidden,
        hidden + 1,
        "a back-room deal"
    );
    // Once a turn.
    assert!(
        !offered(&e, person, bill).contains(&Argument::AfterHours),
        "the late slot is spent"
    );
}

#[test]
fn a_committed_member_is_settled() {
    let c = content();
    let (mut e, bill) = with_bill(31, &c);
    let (p, person) = members(&e)[0];
    assert!(!offered(&e, person, bill).is_empty());
    e.setup(|st| {
        st.politics.declared_votes.insert((p.0, bill), 1);
    });
    assert!(lobbying::settled(e.state(), p, bill));
    assert!(offered(&e, person, bill).is_empty());
    assert!(argue(&mut e, person, bill, Argument::Plain).is_err());
    // The player's own bill: they are its sponsor, and settled on it.
    assert!(lobbying::settled(e.state(), me(&e), bill));
}

#[test]
fn an_ally_brings_several_votes_at_once() {
    let c = content();
    let (base, bill) = with_bill(31, &c);
    let mut seen = false;
    'outer: for (p, person) in members(&base) {
        for week in 0..8 {
            let mut e = copy(&base);
            for _ in 0..week * 2 {
                e.apply(Command::EndTurn).unwrap();
            }
            let me = my_char(&e);
            e.setup(|st| {
                st.politics.characters.edges.insert(
                    (person.0, me.0),
                    Relationship {
                        opinion: 90.0,
                        trust: 0.9,
                        bonds: vec![Bond::Ally],
                        familiar: 0,
                    },
                );
            });
            if !lobbying::brings_others(e.state(), &c, self::me(&e), p, bill) {
                continue 'outer;
            }
            let Some(r) = lobbying::arguments(e.state(), &c, person)
                .into_iter()
                .find(|x| x.argument == Argument::Rally && x.others.len() >= 2)
            else {
                continue 'outer;
            };
            let sway = |e: &Engine, id: PoliticianId| e.state().politics.sway(id, bill);
            let before: Vec<f32> = r.others.iter().map(|(id, _)| sway(&e, *id)).collect();
            let owed = characters::favours_owed(&e.state().politics, me, person);
            let o = argue(&mut e, person, bill, Argument::Rally).unwrap();
            if outcome(&o) != "outcome-lobby-rallied" {
                // Not persuaded: nobody else moves.
                for ((id, _), b) in r.others.iter().zip(&before) {
                    assert_eq!(sway(&e, *id), *b);
                }
                continue;
            }
            for ((id, amount), b) in r.others.iter().zip(&before) {
                assert!((sway(&e, *id) - b - *amount as f32).abs() < 1e-5);
            }
            assert_eq!(
                characters::favours_owed(&e.state().politics, me, person),
                owed + 1,
                "the player owes them for it"
            );
            seen = true;
            break 'outer;
        }
    }
    assert!(seen, "an ally brought others round");
}

#[test]
fn two_exchanges_a_member_a_week() {
    let c = content();
    let (mut e, bill) = with_bill(31, &c);
    // The vote waits (so the bill is still before the council next week).
    e.setup(|st| {
        let p = &mut st.policy.instances[bill as usize];
        p.next_stage_on = p.next_stage_on.map(|d| d.add_days(90));
    });
    let (_, person) = members(&e)[0];
    let limit = c.actions.lobby.arguments.exchanges;
    for _ in 0..limit {
        argue(&mut e, person, bill, Argument::Plain).unwrap();
    }
    assert!(offered(&e, person, bill).is_empty());
    assert!(argue(&mut e, person, bill, Argument::Plain).is_err());
    // Another member still can be.
    let (_, other) = members(&e)[1];
    assert!(offered(&e, other, bill).contains(&Argument::Plain));
    // Next week, again.
    e.apply(Command::EndTurn).unwrap();
    assert!(offered(&e, person, bill).contains(&Argument::Plain));
}

#[test]
fn npcs_lobby_through_the_same_function() {
    let c = content();
    let (e, bill) = with_bill(31, &c);
    let all = members(&e);
    let (actor, _) = all[0];
    let (target, _) = all[1];
    let st = e.state();
    let p = lobbying::pitch(st, &c, actor, target, bill, Argument::Plain, 1.0).unwrap();
    assert_eq!(
        p.odds,
        mandate_sim::npc::lobby_odds_between(st, &c, actor, target, false)
    );
    // An NPC's lobbying is a whole action: the full sway range.
    assert_eq!(p.gain, c.council.lobby.gain);
    assert_eq!(p.fail_gain, c.council.lobby.fail_gain);
    // The player's arguments that read the ledger are the player's.
    for a in [
        Argument::Ward,
        Argument::Broker,
        Argument::Owe,
        Argument::AfterHours,
    ] {
        assert!(lobbying::pitch(st, &c, actor, target, bill, a, 1.0).is_none());
    }
    // The player's plain pitch is the same formula.
    let mine = lobbying::pitch(st, &c, me(&e), target, bill, Argument::Plain, 1.0).unwrap();
    assert_eq!(
        mine.odds,
        mandate_sim::actions::lobby_odds(st, &c, target, false)
    );
}

fn req() -> SaveRequest {
    SaveRequest {
        run_id: "00112233445566778899aabbccddeeff".into(),
        revision: 1,
        parent_revision: None,
        device_id: "test-device".into(),
        saved_at: "2026-01-01T00:00:00Z".into(),
        protocol_version: 1,
        run_mode: RunMode::Ironman,
    }
}

/// Lobby every member with the best day's argument offered, a turn at a time.
fn script(e: &mut Engine, bill: u32, turns: usize) {
    for _ in 0..turns {
        for (_, person) in members(e) {
            let best = lobbying::arguments(e.state(), e.content(), person)
                .into_iter()
                .filter(|p| p.policy == bill)
                .max_by(|a, b| a.odds.total_cmp(&b.odds).then(b.argument.cmp(&a.argument)));
            if let Some(p) = best {
                let _ = argue(e, person, bill, p.argument);
            }
        }
        e.apply(Command::EndTurn).unwrap();
    }
}

#[test]
fn lobbying_is_deterministic_and_survives_a_save() {
    let c = content();
    let (mut a, bill) = with_bill(40, &c);
    let (mut b, _) = with_bill(40, &c);
    script(&mut a, bill, 3);
    script(&mut b, bill, 3);
    assert_eq!(a.state_hash(), b.state_hash());
    let lobbied = a
        .state()
        .player
        .as_ref()
        .unwrap()
        .talk
        .log
        .iter()
        .filter(|x| x.topic.starts_with("choice.lobby."))
        .count();
    assert!(lobbied >= 4, "lobbied {lobbied} times");
    let (bytes, _) = encode(b.state(), &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    assert_eq!(state.hash(), b.state_hash());
    let mut b2 = Engine::from_state(state, c.clone()).unwrap();
    script(&mut a, bill, 3);
    script(&mut b2, bill, 3);
    assert_eq!(a.state_hash(), b2.state_hash());
}
