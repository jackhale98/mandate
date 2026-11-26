//! Asks in conversation (M6 note 12; S4 talk hooks): an ask whose need is
//! "talk to X" puts a choice of its own in the conversation with X, with
//! its own lines and a reply by X's opinion of the player; a need met by a
//! command advances the ask in that command; a later day never breaks an
//! ask before its deadline; no exchange is silent; every need kind has a
//! command that meets it and a route the player is shown.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_protocol::*;
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::asks::{Ask, AskId, AskStatus, Asker, History, Need, StatementNeed};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::narrative::{Actor, CastEntry};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Level, Source, Subject};
use mandate_sim::talk::{AboutRef, TalkTarget, hooks};
use mandate_sim::world::{Jurisdiction, NeighborhoodId};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::Arc;

fn content() -> Arc<mandate_content::ContentBundle> {
    let (c, _) = mandate_content::load(&mandate_content::embedded::sources()).unwrap();
    Arc::new(c)
}

fn game(c: &Arc<mandate_content::ContentBundle>, seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

fn copy(e: &Engine) -> Engine {
    Engine::from_state(e.state().clone(), e.content_arc()).unwrap()
}

fn home(e: &Engine) -> u32 {
    mandate_sim::player::home(e.state()).unwrap().0
}

/// The town's executive (not the player), cast as `ask.introduce_me` casts it.
fn official(e: &Engine) -> CharacterId {
    let st = e.state();
    let c = e
        .content()
        .constitutions
        .get(&st.meta.constitution)
        .unwrap();
    let at = Jurisdiction::town(mandate_sim::world::MunicipalityId(home(e)));
    let p = mandate_sim::politics::executive_of(&st.politics, c, at).unwrap();
    let p = st.politics.get(p).unwrap();
    assert!(!p.is_player);
    p.character
}

/// A Named resident of the player's town who is no politician.
fn resident(e: &Engine, not: CharacterId) -> CharacterId {
    let st = e.state();
    st.politics
        .characters
        .people
        .iter()
        .find(|c| {
            c.named.is_some()
                && c.politician.is_none()
                && c.id != not
                && Some(c.id) != characters::player_character(st)
                && characters::municipality_of(&st.politics, &st.world, c.id) == Some(home(e))
                && characters::routines::whereabouts(st, e.content(), c.id).is_some()
        })
        .map(|c| c.id)
        .unwrap()
}

/// Put an accepted ask of a template with these needs and cast before the
/// player; the player knows the people in it.
fn put(
    e: &mut Engine,
    template: &str,
    cast: Vec<(&str, Actor)>,
    needs: Vec<Need>,
    due_in: u32,
) -> AskId {
    let date = e.state().calendar.date;
    let asker = cast
        .iter()
        .find(|(r, _)| *r == "asker")
        .and_then(|(_, a)| match a {
            Actor::Person { id } => Some(*id),
            _ => None,
        })
        .unwrap();
    let cast: Vec<CastEntry> = cast
        .into_iter()
        .map(|(role, actor)| CastEntry {
            role: role.into(),
            actor,
        })
        .collect();
    let people: Vec<CharacterId> = cast
        .iter()
        .filter_map(|c| match c.actor {
            Actor::Person { id } => Some(id),
            _ => None,
        })
        .collect();
    let mut id = AskId(0);
    e.setup(|st| {
        id = st.asks.ids.alloc();
        st.asks.asks.push(Ask {
            arc: None,
            id,
            template: template.into(),
            promise: false,
            asker: Asker::Person { id: asker },
            cast,
            needs,
            step: 0,
            done: Vec::new(),
            public: false,
            big: false,
            opened: date,
            respond_by: date,
            due: date.add_days(due_in),
            status: AskStatus::Accepted { on: date },
            history: History::None,
            race: None,
            answer: None,
            tries: 0,
        });
        let on = st.calendar.date;
        let k = &mut st.player.as_mut().unwrap().knowledge;
        for p in people {
            k.learn(Subject::Person { person: p }, Level::Known, on, Source::Lab);
        }
    });
    id
}

/// Cosmo asks for a word with the official (`ask.introduce_me`).
fn introduction(e: &mut Engine, asker: CharacterId, official: CharacterId, due_in: u32) -> AskId {
    put(
        e,
        "ask.introduce_me",
        vec![
            ("asker", Actor::Person { id: asker }),
            ("official", Actor::Person { id: official }),
        ],
        vec![
            Need::Talk { person: official },
            Need::Talk { person: asker },
        ],
        due_in,
    )
}

/// Set how someone feels about the player (test setup).
fn set_opinion(e: &mut Engine, holder: CharacterId, to: f32) {
    let c = e.content_arc();
    let me = characters::player_character(e.state()).unwrap();
    e.setup(|st| {
        for _ in 0..40 {
            let op = characters::opinion(&st.politics, holder, me);
            if (op - to).abs() <= 3.0 {
                break;
            }
            let d = ((to - op) as f64).clamp(-30.0, 30.0);
            characters::change_opinion(&mut st.politics, &c, holder, me, d);
        }
    });
}

/// Go to where someone is now (turns pass until they can be reached).
fn find(e: &mut Engine, who: CharacterId) {
    for _ in 0..12 {
        if mandate_sim::talk::can_talk(e.state(), e.content(), who) {
            return;
        }
        if let Some(loc) = talk_location(e, who) {
            let r = e.apply(Command::Player(PlayerCommand::MoveTo { location: loc }));
            if r.is_ok() && mandate_sim::talk::can_talk(e.state(), e.content(), who) {
                return;
            }
        }
        e.apply(Command::AdvanceDays { days: 1 }).unwrap();
    }
    panic!("could not reach {who:?}");
}

fn ask(e: &Engine, id: AskId) -> Ask {
    e.state().asks.get(id).unwrap().clone()
}

fn talk(e: &mut Engine, to: CharacterId, topic: &str, about: Option<AboutRef>) -> u32 {
    let r = e
        .apply(Command::Player(PlayerCommand::Talk {
            to: TalkTarget::Person { person: to },
            topic: topic.into(),
            about,
            late_night: false,
        }))
        .unwrap_or_else(|err| panic!("talk `{topic}`: {err}"));
    r.result.unwrap().created.unwrap()
}

/// The ask's choice as the talk panel lists it.
fn listed(e: &Engine, person: CharacterId, id: AskId) -> Option<TopicView> {
    let v = talk_view(e, TalkTargetDto::Person { person: person.0 }, false)?;
    let want = mandate_sim::asks::hook::choice_id(id);
    v.topics.into_iter().find(|t| t.id == want)
}

fn view(e: &Engine, id: AskId) -> AskView {
    ask_view(e, e.state().asks.get(id).unwrap())
}

/// The owner's report, step one: talking to the official about the asker.
/// The ask's own choice is in the official's panel; choosing it gets a real
/// reply and meets the first need in that command (not at the next tick).
/// The old way (`topic.about` the asker) is never silent either.
#[test]
fn bug_1_the_official_answers_and_the_step_is_met_at_once() {
    let c = content();
    for seed in [2026u64, 7, 31] {
        let mut e = game(&c, seed);
        let off = official(&e);
        let asker = resident(&e, off);
        let id = introduction(&mut e, asker, off, 30);
        set_opinion(&mut e, off, 40.0);
        find(&mut e, off);
        // Asked about the asker: something is said.
        let x = talk(
            &mut e,
            off,
            "topic.about",
            Some(AboutRef::Person { person: asker }),
        );
        assert!(
            !exchange_view(&e, x).unwrap().parts.is_empty(),
            "seed {seed}"
        );
        assert_eq!(
            ask(&e, id).step,
            0,
            "an ordinary talk is not the ask's choice"
        );
        // The choice, listed and available, labelled with the asker's name.
        let t = listed(&e, off, id).expect("the ask's choice is in the panel");
        assert!(t.available, "seed {seed}: {:?}", t.unavailable_key);
        assert_eq!(t.name_key, "ask-choice-introduce-official");
        assert_eq!(t.mechanic, "choice");
        let name = characters::profile(&e.state().politics, &e.state().world, asker)
            .unwrap()
            .full_name();
        assert_eq!(t.name_args.as_ref().unwrap().get("asker"), Some(&name));
        assert!(listed(&e, asker, id).is_none(), "not with the asker yet");
        let x = talk(&mut e, off, &t.id, None);
        let v = exchange_view(&e, x).unwrap();
        assert_eq!(v.parts.len(), 1);
        assert_eq!(v.parts[0].key, "talk-ask-introduce-warm");
        assert_eq!(v.parts[0].args.get("asker"), Some(&name));
        assert_eq!(v.outcome_key, "outcome-ask-choice-agreed");
        // Met in the command: the Today block shows the next step now.
        assert_eq!(ask(&e, id).step, 1, "seed {seed}");
        let av = view(&e, id);
        assert_eq!(av.needs[0].state, "done");
        assert_eq!(av.needs[1].state, "current");
        assert_eq!(av.person.as_ref().and_then(|p| p.id), Some(asker.0));
        assert!(
            listed(&e, off, id).is_none(),
            "asked once: the choice is gone"
        );
    }
}

/// The owner's report, step two: the official on one day, the asker the
/// next. The ask is kept in that command, and stays kept.
#[test]
fn bug_2_a_later_day_does_not_break_the_introduction() {
    let c = content();
    for seed in [2026u64, 7, 31] {
        let mut e = game(&c, seed);
        let off = official(&e);
        let asker = resident(&e, off);
        let id = introduction(&mut e, asker, off, 30);
        set_opinion(&mut e, off, 0.0);
        // Out of order first: telling the asker before the official is
        // nothing, and breaks nothing.
        find(&mut e, asker);
        talk(&mut e, asker, "topic.small_talk", None);
        e.apply(Command::AdvanceDays { days: 1 }).unwrap();
        assert!(ask(&e, id).accepted(), "seed {seed}");
        assert_eq!(ask(&e, id).step, 0);
        find(&mut e, off);
        let choice = mandate_sim::asks::hook::choice_id(id);
        let x = talk(&mut e, off, &choice, None);
        // A curt yes still counts.
        assert_eq!(
            exchange_view(&e, x).unwrap().parts[0].key,
            "talk-ask-introduce-curt"
        );
        assert_eq!(ask(&e, id).step, 1);
        e.apply(Command::AdvanceDays { days: 1 }).unwrap();
        assert!(
            ask(&e, id).accepted(),
            "seed {seed}: {:?}",
            ask(&e, id).status
        );
        find(&mut e, asker);
        let x = talk(&mut e, asker, &choice, None);
        assert_eq!(
            exchange_view(&e, x).unwrap().parts[0].key,
            "talk-ask-introduce-thanks"
        );
        assert!(
            matches!(ask(&e, id).status, AskStatus::Kept { .. }),
            "seed {seed}: kept in the command, {:?}",
            ask(&e, id).status
        );
        e.apply(Command::AdvanceDays { days: 2 }).unwrap();
        assert!(matches!(ask(&e, id).status, AskStatus::Kept { .. }));
        let resolved = e
            .state()
            .events
            .iter()
            .filter(|ev| matches!(&ev.kind, mandate_sim::events::EventKind::AskResolved { ask, .. } if *ask == id.0))
            .count();
        assert_eq!(resolved, 1, "resolved exactly once");
    }
}

/// Only the deadline breaks an ask nobody acted on: every day before it
/// leaves it open.
#[test]
fn only_the_deadline_breaks_an_untouched_ask() {
    let c = content();
    let mut e = game(&c, 2026);
    let off = official(&e);
    let asker = resident(&e, off);
    let id = introduction(&mut e, asker, off, 10);
    for _ in 0..10 {
        e.apply(Command::AdvanceDays { days: 1 }).unwrap();
        assert!(ask(&e, id).accepted(), "{:?}", e.state().calendar.date);
    }
    e.apply(Command::AdvanceDays { days: 1 }).unwrap();
    assert!(matches!(ask(&e, id).status, AskStatus::Broken { .. }));
}

/// An official who thinks little of the player declines: the need stays
/// open, the ask says so, and asking again once they think better of the
/// player works.
#[test]
fn a_cold_official_declines_and_the_ask_says_so() {
    let c = content();
    let mut e = game(&c, 7);
    let off = official(&e);
    let asker = resident(&e, off);
    let id = introduction(&mut e, asker, off, 30);
    set_opinion(&mut e, off, -60.0);
    find(&mut e, off);
    let choice = mandate_sim::asks::hook::choice_id(id);
    let x = talk(&mut e, off, &choice, None);
    let v = exchange_view(&e, x).unwrap();
    assert_eq!(v.parts[0].key, "talk-ask-introduce-decline");
    assert_eq!(v.outcome_key, "outcome-ask-choice-refused");
    assert_eq!(ask(&e, id).step, 0);
    assert!(ask(&e, id).accepted());
    let av = view(&e, id);
    assert_eq!(av.note_key.as_deref(), Some("ui-ask-talk-refused"));
    assert!(av.args.contains_key("name"));
    // Still offered: they can be asked again.
    assert!(listed(&e, off, id).is_some());
    set_opinion(&mut e, off, 30.0);
    e.apply(Command::EndTurn).unwrap();
    find(&mut e, off);
    talk(&mut e, off, &choice, None);
    assert_eq!(ask(&e, id).step, 1);
    assert_eq!(view(&e, id).note_key, None);
}

/// The registry lists a hook's choices for the right person only, and the
/// talk panel carries them with their arguments.
#[test]
fn hooks_list_choices_for_the_person_the_need_names() {
    let c = content();
    let mut e = game(&c, 31);
    let off = official(&e);
    let asker = resident(&e, off);
    let other = {
        let st = e.state();
        st.politics
            .characters
            .people
            .iter()
            .find(|p| p.named.is_some() && p.id != off && p.id != asker)
            .unwrap()
            .id
    };
    assert!(hooks::offered(e.state(), e.content(), off).is_empty());
    let id = introduction(&mut e, asker, off, 30);
    let at_official = hooks::offered(e.state(), e.content(), off);
    assert_eq!(at_official.len(), 1);
    assert_eq!(at_official[0].id, format!("choice.ask.{}", id.0));
    assert_eq!(at_official[0].replies.len(), 3);
    assert!(hooks::offered(e.state(), e.content(), asker).is_empty());
    assert!(hooks::offered(e.state(), e.content(), other).is_empty());
    let (h, ch) = hooks::find(e.state(), e.content(), off, &at_official[0].id).unwrap();
    assert_eq!(h.owner, "ask");
    assert_eq!(ch, at_official[0]);
    // An offer not yet accepted offers nothing.
    e.setup(|st| {
        if let Some(a) = st.asks.asks.iter_mut().find(|a| a.id == id) {
            a.status = AskStatus::Offered;
        }
    });
    assert!(hooks::offered(e.state(), e.content(), off).is_empty());
    // An unknown choice is refused, and changes nothing.
    find(&mut e, off);
    let before = e.state().player.as_ref().unwrap().slots_used;
    let r = e.apply(Command::Player(PlayerCommand::Talk {
        to: TalkTarget::Person { person: off },
        topic: "choice.ask.999".into(),
        about: None,
        late_night: false,
    }));
    assert!(r.is_err());
    assert_eq!(e.state().player.as_ref().unwrap().slots_used, before);
}

/// A deed that meets a need resolves the ask in its own command.
#[test]
fn steps_resolve_in_the_command_that_meets_them() {
    let c = content();
    let mut e = game(&c, 3);
    let off = official(&e);
    let asker = resident(&e, off);
    let id = put(
        &mut e,
        "ask.casework",
        vec![("asker", Actor::Person { id: asker })],
        vec![Need::Statement {
            about: StatementNeed::Any,
        }],
        20,
    );
    let cmd = e.route(PlayerCommand::Act {
        action: Action::Press { policy: None },
        late_night: false,
    });
    e.apply(cmd).unwrap();
    assert!(matches!(ask(&e, id).status, AskStatus::Kept { .. }));
    // The Today block shows it among the recent outcomes at once.
    assert!(recent_outcomes(&e, 1, 5).iter().any(|a| a.id == id.0));
}

/// Every topic and choice the panel offers says at least one line.
#[test]
fn no_exchange_is_silent() {
    let c = content();
    for seed in [2026u64, 5] {
        let mut e = game(&c, seed);
        let off = official(&e);
        let asker = resident(&e, off);
        introduction(&mut e, asker, off, 30);
        find(&mut e, off);
        let here: Vec<CharacterId> = mandate_sim::talk::present(e.state(), e.content());
        let mut said = 0;
        for p in here.into_iter().take(4) {
            let Some(v) = talk_view(&e, TalkTargetDto::Person { person: p.0 }, false) else {
                continue;
            };
            for t in &v.topics {
                let mut tries: Vec<Option<AboutRefDto>> = Vec::new();
                if t.available {
                    tries.push(None);
                }
                tries.extend(
                    t.about
                        .iter()
                        .filter(|a| a.available)
                        .take(3)
                        .map(|a| Some(a.about.clone())),
                );
                for about in tries {
                    let mut x = copy(&e);
                    let r = x
                        .apply(Command::Player(PlayerCommand::Talk {
                            to: TalkTarget::Person { person: p },
                            topic: t.id.clone(),
                            about: about.as_ref().map(|a| a.to_sim()),
                            late_night: false,
                        }))
                        .unwrap_or_else(|err| panic!("{} offered but refused: {err}", t.id));
                    let id = r.result.unwrap().created.unwrap();
                    let ex = exchange_view(&x, id).unwrap();
                    assert!(
                        !ex.parts.is_empty(),
                        "seed {seed}: {} about {about:?} said nothing",
                        t.id
                    );
                    said += 1;
                }
            }
        }
        assert!(said > 5, "seed {seed}: {said}");
    }
}

/// The need-path audit: every need kind has a route the player is shown
/// (`how`) and a command that meets it.
#[test]
fn every_need_kind_has_a_command_and_a_route() {
    let c = content();
    let e = game(&c, 4);
    let off = official(&e);
    let asker = resident(&e, off);
    let hood = {
        let st = e.state();
        let pl = st.player.as_ref().unwrap();
        NeighborhoodId(mandate_sim::policy::ward_hoods(&st.world, pl.ward)[0] as u32)
    };
    let place = e
        .state()
        .world
        .places
        .iter()
        .find(|p| Some(p.municipality) == mandate_sim::player::home(e.state()) && p.is_open())
        .unwrap()
        .id;
    let template = e
        .content()
        .policies
        .values()
        .find(|t| t.tier == mandate_content::TierKey::Local && t.unlock.is_none())
        .unwrap()
        .id
        .clone();
    /// Every kind (a new `Need` fails to compile here until it is audited).
    fn kind(n: &Need) -> &'static str {
        match n {
            Need::Vote { .. } => "vote",
            Need::VoteAgainstAll { .. } => "against_all",
            Need::Statement { .. } => "statement",
            Need::TownHall { .. } => "town_hall",
            Need::Canvass { .. } => "canvass",
            Need::Visit { .. } => "visit",
            Need::Talk { .. } => "talk",
            Need::Introduce { .. } => "introduce",
            Need::Enacted { .. } => "enacted",
            Need::Casework => "casework",
            Need::Fixed { .. } => "fixed",
            Need::NoSurfacedHiddenFact => "no_surfaced_hidden_fact",
            Need::Quiet { .. } => "quiet",
        }
    }
    let all = vec![
        Need::Vote {
            policy: 0,
            vote: mandate_sim::politics::Vote::Yes,
        },
        Need::VoteAgainstAll {
            tag: "tax_rise".into(),
        },
        Need::Statement {
            about: StatementNeed::Any,
        },
        Need::TownHall { hoods: vec![hood] },
        Need::Canvass {
            hood,
            or_town_hall: false,
        },
        Need::Visit { place },
        Need::Talk { person: asker },
        Need::Introduce {
            templates: vec![template.clone()],
        },
        Need::Enacted {
            templates: vec![template.clone()],
            sponsor_player: true,
        },
        Need::Casework,
        Need::Fixed { fault: FAULT },
        Need::NoSurfacedHiddenFact,
        Need::Quiet { person: off },
    ];
    let mut kinds: Vec<&str> = all.iter().map(kind).collect();
    kinds.dedup();
    assert_eq!(kinds.len(), 13);
    // A fault on the ward's streets for the fault need (DEC-114).
    const FAULT: mandate_sim::upkeep::FaultId = mandate_sim::upkeep::FaultId(9_000);
    let with_fault = |x: &mut Engine| {
        x.setup(|st| {
            let opened = st.calendar.date;
            st.upkeep.faults.push(mandate_sim::upkeep::Fault {
                id: FAULT,
                kind: "fault.streetlight".into(),
                hood,
                spot: 0,
                opened,
                status: mandate_sim::upkeep::FaultStatus::Open,
                expedited: None,
                ahead_of: None,
                put_back_for: None,
            });
        });
    };
    for n in &all {
        let mut x = copy(&e);
        if matches!(n, Need::Fixed { .. }) {
            with_fault(&mut x);
        }
        let id = put(
            &mut x,
            if matches!(n, Need::Talk { .. }) {
                "ask.meet_the_newcomer"
            } else if matches!(n, Need::Fixed { .. }) {
                "ask.fault"
            } else {
                "ask.casework"
            },
            vec![
                ("asker", Actor::Person { id: asker }),
                ("newcomer", Actor::Person { id: asker }),
            ],
            vec![n.clone()],
            40,
        );
        let v = view(&x, id);
        let how = v
            .how
            .clone()
            .unwrap_or_else(|| panic!("{}: no route", kind(n)));
        assert!(!how.action.is_empty(), "{}", kind(n));
        // The command that meets it.
        let route = |x: &Engine, c: PlayerCommand| x.route(c);
        match n {
            Need::Statement { .. } => {
                let c = route(
                    &x,
                    PlayerCommand::Act {
                        action: Action::Press { policy: None },
                        late_night: false,
                    },
                );
                x.apply(c).unwrap();
            }
            Need::TownHall { .. } => {
                let c = route(
                    &x,
                    PlayerCommand::Act {
                        action: Action::TownHall {
                            neighborhood: hood.0,
                            policy: None,
                        },
                        late_night: false,
                    },
                );
                x.apply(c).unwrap();
            }
            Need::Canvass { .. } => {
                let c = route(
                    &x,
                    PlayerCommand::Act {
                        action: Action::Canvass {
                            neighborhood: hood.0,
                            issue: None,
                        },
                        late_night: false,
                    },
                );
                x.apply(c).unwrap();
            }
            Need::Visit { place } => {
                x.apply(Command::Player(PlayerCommand::MoveTo {
                    location: Location::Place { place: *place },
                }))
                .unwrap();
            }
            Need::Talk { person } => {
                find(&mut x, *person);
                let t = listed(&x, *person, id).expect("the choice");
                talk(&mut x, *person, &t.id, None);
            }
            Need::Casework => {
                for _ in 0..6 {
                    if !ask(&x, id).accepted() {
                        break;
                    }
                    let c = route(
                        &x,
                        PlayerCommand::Act {
                            action: Action::Casework { ask: id.0 },
                            late_night: false,
                        },
                    );
                    if x.apply(c).is_err() {
                        x.apply(Command::EndTurn).unwrap();
                    }
                }
            }
            // Moved up the queue at the services desk, then repaired when
            // the month's money comes round.
            Need::Fixed { fault } => {
                assert_eq!(how.action, "casework");
                for _ in 0..6 {
                    if !mandate_sim::asks::can_casework(x.state(), id.0) {
                        break;
                    }
                    let c = route(
                        &x,
                        PlayerCommand::Act {
                            action: Action::Casework { ask: id.0 },
                            late_night: false,
                        },
                    );
                    if x.apply(c).is_err() {
                        x.apply(Command::EndTurn).unwrap();
                    }
                }
                assert!(x.state().upkeep.get(*fault).unwrap().expedited.is_some());
                x.apply(Command::AdvanceDays { days: 39 }).unwrap();
            }
            Need::Introduce { templates } => {
                let c = route(
                    &x,
                    PlayerCommand::DraftPolicy {
                        template: templates[0].clone(),
                        params: Default::default(),
                    },
                );
                let p = x.apply(c).unwrap().result.unwrap().created.unwrap();
                let c = route(&x, PlayerCommand::IntroducePolicy { policy: p });
                x.apply(c).unwrap();
            }
            // Kept by not acting: time passing keeps it.
            Need::VoteAgainstAll { .. } | Need::NoSurfacedHiddenFact | Need::Quiet { .. } => {
                assert_eq!(how.action, "wait");
                x.apply(Command::AdvanceDays { days: 41 }).unwrap();
            }
            // A vote on a bill before the council (`CastVote`) and a bill
            // becoming law (`DraftPolicy`, then the council): routes only.
            Need::Vote { .. } => assert_eq!(how.action, "vote"),
            Need::Enacted { .. } => assert_eq!(how.action, "draft"),
        }
        if !matches!(n, Need::Vote { .. } | Need::Enacted { .. }) {
            assert!(
                matches!(ask(&x, id).status, AskStatus::Kept { .. }),
                "{}: {:?}",
                kind(n),
                ask(&x, id).status
            );
        }
    }
}
