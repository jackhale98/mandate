//! The talk panel and exchanges (DEC-093; M6_ENGINE_CONTRACTS §3.9; task
//! C2): `TalkView` lists exactly what `Talk` accepts, with a reason for
//! what it refuses; the odds shown are the odds rolled (DEC-073); an
//! exchange reads back as spoken, with what was learned and the leads;
//! nothing hidden leaks through an exchange or the canvass worries
//! (DEC-091); the notebook links to the exchange; views never write.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_core::Stream;
use mandate_protocol::*;
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Level, Source, Subject};
use mandate_sim::talk::{self, AboutRef, TalkTarget};
use mandate_sim::world::{MunicipalityId, NeighborhoodId, PlaceKind};
use mandate_sim::{Command, Engine};
use mandate_wasm::session::Session;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::Arc;

fn content() -> Arc<mandate_content::ContentBundle> {
    let (c, _) = mandate_content::load(&mandate_content::embedded::sources()).unwrap();
    Arc::new(c)
}

fn game(seed: u64) -> Engine {
    generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        content(),
    )
    .unwrap()
}

/// A copy to try a command on (the original stays as it was).
fn copy(e: &Engine) -> Engine {
    Engine::from_state(e.state().clone(), e.content_arc()).unwrap()
}

fn town(e: &Engine) -> MunicipalityId {
    e.state().player.as_ref().unwrap().municipality
}

fn place(e: &Engine, kind: PlaceKind) -> Option<Location> {
    let t = town(e);
    e.state()
        .world
        .places
        .iter()
        .find(|p| p.municipality == t && p.kind == kind && p.is_open())
        .map(|p| Location::Place { place: p.id })
}

/// Where the council is now, on a day it sits (DEC-031 routines: members
/// are in their chamber on session days): turns pass until one, then the
/// engine says where a member is.
fn council_spot(e: &mut Engine) -> Location {
    let at = mandate_sim::world::Jurisdiction::town(town(e));
    for _ in 0..8 {
        let now = mandate_sim::characters::routines::Now::of(e.state(), e.content());
        if now.session(e.state().meta.seed, at) {
            break;
        }
        e.apply(Command::EndTurn).unwrap();
    }
    let m = mandate_sim::politics::body_members_at(&e.state().politics, at)
        .into_iter()
        .filter_map(|p| e.state().politics.get(p))
        .find(|p| !p.is_player)
        .map(|p| p.character)
        .unwrap();
    mandate_sim::characters::routines::whereabouts(e.state(), e.content(), m).unwrap()
}

fn go(e: &mut Engine, loc: Location) {
    e.apply(Command::Player(PlayerCommand::MoveTo { location: loc }))
        .unwrap();
}

/// Forget everything (test setup).
fn wipe(e: &mut Engine) {
    e.setup(|st| {
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        k.entries.clear();
        k.leads.clear();
    });
}

/// Put a subject at exactly `level` (test setup, not a command).
fn set(e: &mut Engine, s: &Subject, level: Level) {
    e.setup(|st| {
        let on = st.calendar.date;
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        k.entries.remove(s);
        if level > Level::Unknown {
            k.learn(s.clone(), level, on, Source::Lab);
        }
    });
}

fn talk_cmd(person: CharacterId, topic: &str, about: Option<AboutRef>, late: bool) -> Command {
    Command::Player(PlayerCommand::Talk {
        to: TalkTarget::Person { person },
        topic: topic.into(),
        about,
        late_night: late,
    })
}

/// A standard local template (for a bill to talk about).
fn standard_template(e: &Engine) -> String {
    e.content()
        .policies
        .values()
        .find(|t| t.tier == mandate_content::TierKey::Local && t.unlock.is_none())
        .unwrap()
        .id
        .clone()
}

/// Draft a bill and put it before the council (as a sponsor would).
fn bill(e: &mut Engine) -> u32 {
    let template = standard_template(e);
    let r = e
        .apply(e.route(PlayerCommand::DraftPolicy {
            template,
            params: Default::default(),
        }))
        .unwrap();
    let id = r.result.unwrap().created.unwrap();
    let intro = e.route(PlayerCommand::IntroducePolicy { policy: id });
    let _ = e.apply(intro);
    e.apply(Command::EndTurn).unwrap();
    id
}

/// Places where people are found, for sampling.
fn spots(e: &Engine) -> Vec<Location> {
    let mut v: Vec<Location> = [
        PlaceKind::CouncilChamber,
        PlaceKind::TownHall,
        PlaceKind::Diner,
        PlaceKind::UnionHall,
        PlaceKind::Market,
        PlaceKind::BusinessAssociation,
        PlaceKind::CommunityCentre,
    ]
    .into_iter()
    .filter_map(|k| place(e, k))
    .collect();
    let t = town(e);
    if let Some(h) = e.state().world.hoods_of(t).next() {
        v.push(Location::Street {
            neighborhood: NeighborhoodId(h as u32),
        });
    }
    v
}

/// A subject (or none), and what the panel says of it: available, why not, odds.
type Case = (Option<AboutRef>, bool, Option<String>, Option<f64>);

/// Every topic and subject the panel offers is exactly what the command
/// takes: available ⇔ `Talk` succeeds; every refusal has a reason; topics
/// left out are refused by the command.
#[test]
fn talk_view_matches_what_talk_accepts() {
    let mut e = game(41);
    let _ = bill(&mut e);
    let mut checked = 0;
    let mut refused = 0;
    for loc in spots(&e) {
        go(&mut e, loc);
        for person in talk::present(e.state(), e.content()).into_iter().take(4) {
            let v = talk_view(&e, TalkTargetDto::Person { person: person.0 }, false).unwrap();
            assert!(v.here && v.route.is_none());
            assert!(v.person.can_talk);
            assert_eq!(v.cost_halves, 1, "half a slot");
            let ids: Vec<&str> = v.topics.iter().map(|t| t.id.as_str()).collect();
            for t in &e.content().talk_topics {
                if ids.contains(&t.id.as_str()) || t.about_required {
                    continue;
                }
                let mut x = copy(&e);
                assert!(
                    x.apply(talk_cmd(person, &t.id, None, false)).is_err(),
                    "{} is left out, so the command refuses it",
                    t.id
                );
            }
            for t in &v.topics {
                let mut cases: Vec<Case> = Vec::new();
                if !t.about_required {
                    cases.push((None, t.available, t.unavailable_key.clone(), t.odds));
                }
                for c in &t.about {
                    cases.push((
                        Some(c.about.to_sim()),
                        c.available,
                        c.unavailable_key.clone(),
                        c.odds,
                    ));
                }
                for (about, available, why, odds) in cases {
                    let mut x = copy(&e);
                    let r = x.apply(talk_cmd(person, &t.id, about.clone(), false));
                    assert_eq!(
                        r.is_ok(),
                        available,
                        "{} {:?}: view says {available}, command says {:?}",
                        t.id,
                        about,
                        r.err()
                    );
                    if !available {
                        refused += 1;
                        let k = why.expect("an unavailable topic says why");
                        assert!(k.starts_with("talk-why-"), "{k}");
                        assert_ne!(k, "talk-why-other", "{} {:?}", t.id, about);
                        assert!(odds.is_none());
                    }
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 50, "{checked} topic choices checked");
    eprintln!("{checked} choices, {refused} refused");
}

/// Out of time, every topic says so (and the panel says out of time).
#[test]
fn out_of_time_is_said() {
    let mut e = game(42);
    let at = council_spot(&mut e);
    go(&mut e, at);
    let person = talk::present(e.state(), e.content())[0];
    e.setup(|st| {
        let pl = st.player.as_mut().unwrap();
        pl.slots_used = pl.slot_cap();
        pl.talk.half_slot = None;
    });
    // Late now: they have moved on; go to them (free).
    let there = talk_location(&e, person).unwrap();
    go(&mut e, there);
    let v = talk_view(&e, TalkTargetDto::Person { person: person.0 }, false).unwrap();
    assert!(v.out_of_time);
    assert_eq!(v.slots_left, 0);
    assert!(!v.topics.is_empty());
    for t in &v.topics {
        assert!(!t.available, "{}", t.id);
        assert_eq!(
            t.unavailable_key.as_deref(),
            Some("talk-why-time"),
            "{}",
            t.id
        );
    }
    // A half slot left over from a conversation is still time for one more.
    e.setup(|st| {
        let turn = st.calendar.turn;
        st.player.as_mut().unwrap().talk.half_slot = Some(turn);
    });
    // (That slot is still under way, so it is the evening again: go to them.)
    let there = talk_location(&e, person).unwrap();
    go(&mut e, there);
    let v = talk_view(&e, TalkTargetDto::Person { person: person.0 }, false).unwrap();
    assert!(v.half_slot_left && !v.out_of_time);
    assert!(v.topics.iter().any(|t| t.available));
}

/// The odds the panel shows are the odds the roll uses (DEC-073), even when
/// the conversation itself teaches who they are or warms them first.
#[test]
fn odds_shown_are_odds_rolled() {
    let mut base = game(43);
    let policy = bill(&mut base);
    let at = council_spot(&mut base);
    go(&mut base, at);
    let rival = base.state().politics.characters.towns[town(&base).index()].rival;
    let people: Vec<CharacterId> = talk::present(base.state(), base.content())
        .into_iter()
        .filter(|c| Some(*c) != rival)
        .collect();
    let mut trials = 0;
    let mut gap = 0;
    for (i, person) in people.iter().copied().enumerate().take(4) {
        for shift in 0..40 {
            let mut e = copy(&base);
            // Only heard of: talking teaches them `known`, which would raise
            // the odds if they were worked out after the reply.
            set(&mut e, &Subject::Person { person }, Level::Heard);
            e.setup(|st| {
                for _ in 0..(shift + i * 41) {
                    st.rng.get(Stream::Player).next_f64();
                }
            });
            let v = talk_view(&e, TalkTargetDto::Person { person: person.0 }, false).unwrap();
            let Some(t) = v.topics.iter().find(|t| t.id == "topic.pitch") else {
                continue;
            };
            let Some(c) = t
                .about
                .iter()
                .find(|c| c.about == (AboutRefDto::Bill { policy }))
            else {
                continue;
            };
            let p = c.odds.expect("a pitch on a bill to a colleague shows odds");
            assert!(c.available);
            // What the odds would be once they are known.
            let mut known = copy(&e);
            set(&mut known, &Subject::Person { person }, Level::Known);
            let later = talk::odds(
                known.state(),
                known.content(),
                person,
                "topic.pitch",
                Some(&AboutRef::Bill {
                    policy: mandate_sim::policy::PolicyInstanceId(policy),
                }),
                false,
            )
            .unwrap();
            let u = e.state().rng.clone().get(Stream::Player).next_f64();
            if (u >= p) != (u >= later) {
                gap += 1;
            }
            let r = e
                .apply(talk_cmd(
                    person,
                    "topic.pitch",
                    Some(AboutRef::Bill {
                        policy: mandate_sim::policy::PolicyInstanceId(policy),
                    }),
                    false,
                ))
                .unwrap()
                .result
                .unwrap();
            let x = exchange_view(&e, r.created.unwrap()).unwrap();
            let success = x.outcome_key == "outcome-talk-pitch-success";
            assert!(success || x.outcome_key == "outcome-talk-pitch-fail");
            assert_eq!(success, u < p, "shown {p}, drew {u}");
            trials += 1;
        }
    }
    assert!(trials >= 40, "{trials} pitches");
    assert!(gap > 0, "the test reaches draws between the two odds");
}

/// An exchange reads back as it was spoken: its lines, what was learned (in
/// the ledger), the leads, the outcome; the notebook links to it.
#[test]
fn exchange_view_reads_back_and_the_notebook_links_it() {
    let mut e = game(44);
    wipe(&mut e);
    let mut found = None;
    'outer: for loc in spots(&e) {
        go(&mut e, loc);
        for person in talk::present(e.state(), e.content()) {
            let mut x = copy(&e);
            let Ok(o) = x.apply(talk_cmd(person, "topic.worries", None, false)) else {
                continue;
            };
            let id = o.result.unwrap().created.unwrap();
            let v = exchange_view(&x, id).unwrap();
            if !v.learned.is_empty() {
                found = Some((x, id, v));
                break 'outer;
            }
        }
    }
    let (x, id, v) = found.expect("someone tells the player something");
    assert_eq!(v.id, id);
    assert_eq!(v.topic, "topic.worries");
    assert_eq!(v.topic_key, "talk-topic-worries");
    assert!(!v.parts.is_empty());
    assert!(v.parts.iter().any(|p| p.part == "substance"));
    assert!(v.parts.iter().all(|p| !p.key.is_empty()));
    assert!(!v.speaker.name.is_empty(), "talking is meeting them");
    assert!(v.outcome_args.contains_key("name"));
    let pl = x.state().player.as_ref().unwrap();
    for l in &v.learned {
        assert!(l.title_key.starts_with("notebook-"));
        let s = match &l.subject {
            SubjectDto::Person { person } => Subject::Person {
                person: CharacterId(*person),
            },
            SubjectDto::HoodConcerns { hood } => Subject::HoodConcerns {
                hood: NeighborhoodId(*hood),
            },
            SubjectDto::TownConcerns { town } => Subject::TownConcerns {
                town: MunicipalityId(*town),
            },
            _ => continue,
        };
        assert!(KnowledgeLevelDto::from(pl.knowledge.level(&s)) >= l.level);
    }
    // The notebook names the speaker and links the exchange.
    let n = notebook_view(&x).unwrap();
    let linked: Vec<u32> = n
        .sections
        .iter()
        .flat_map(|s| &s.entries)
        .flat_map(|e| &e.history)
        .filter(|h| h.key == "learned-talk")
        .filter_map(|h| h.exchange)
        .collect();
    assert!(linked.contains(&id), "the notebook links the exchange");
}

/// Someone elsewhere: the panel says where they are (auto-routing), topics
/// wait until the player is there; after going there they can be asked.
#[test]
fn someone_elsewhere_has_a_route() {
    let mut e = game(45);
    let chamber = council_spot(&mut e);
    go(&mut e, chamber);
    let person = talk::present(e.state(), e.content())[0];
    let home = Location::Home;
    go(&mut e, home);
    let v = talk_view(&e, TalkTargetDto::Person { person: person.0 }, false).unwrap();
    assert!(!v.here && !v.person.can_talk);
    let r = v.route.expect("where to find them");
    assert_eq!(r.action, "go");
    let to = r.to.unwrap();
    assert!(!v.topics.is_empty());
    assert!(v.topics.iter().all(|t| !t.available
        && t.unavailable_key.as_deref() == Some("talk-why-away")
        && t.odds.is_none()));
    go(&mut e, to.location.to_sim());
    let v = talk_view(&e, TalkTargetDto::Person { person: person.0 }, false).unwrap();
    assert!(v.here && v.person.can_talk);
    assert!(v.topics.iter().any(|t| t.available));
    // The place panel lists them as someone to talk to now.
    let panel = place_panel_view(&e, to.location).unwrap();
    assert!(
        panel
            .people
            .iter()
            .any(|p| p.id == Some(person.0) && p.can_talk)
    );
}

/// Nothing hidden leaks through an exchange: a person's name in what was
/// said is someone the player now knows of (it was said to them), and the
/// broker is named only once the player has heard who it is.
#[test]
fn exchanges_leak_nothing_hidden() {
    for seed in [46u64, 47] {
        let mut e = game(seed);
        let _ = bill(&mut e);
        wipe(&mut e);
        let t = town(&e);
        let broker = e.state().politics.characters.towns[t.index()].broker;
        let people: Vec<(CharacterId, String)> = e
            .state()
            .politics
            .characters
            .people
            .iter()
            .filter_map(|c| {
                characters::profile(&e.state().politics, &e.state().world, c.id)
                    .map(|p| (c.id, p.full_name()))
            })
            .collect();
        let mut exchanges = 0;
        for loc in spots(&e) {
            go(&mut e, loc);
            for person in talk::present(e.state(), e.content()).into_iter().take(3) {
                let v = talk_view(&e, TalkTargetDto::Person { person: person.0 }, false).unwrap();
                for topic in v.topics.iter() {
                    let about = if topic.about_required {
                        match topic.about.iter().find(|c| c.available) {
                            Some(c) => Some(c.about.to_sim()),
                            None => continue,
                        }
                    } else {
                        None
                    };
                    let mut x = copy(&e);
                    // Time is not the point here.
                    x.setup(|st| st.player.as_mut().unwrap().slots_used = 0);
                    let Ok(o) = x.apply(talk_cmd(person, &topic.id, about, false)) else {
                        continue;
                    };
                    let ev = exchange_view(&x, o.result.unwrap().created.unwrap()).unwrap();
                    exchanges += 1;
                    let knows = |c: CharacterId| {
                        mandate_sim::player::knowledge::knows(
                            x.state(),
                            &Subject::Person { person: c },
                        )
                    };
                    let json = serde_json::to_string(&ev).unwrap();
                    // A name inside the speaker's own (Rhea Petrov in Rhea
                    // Petrovfield) is the speaker, not someone else.
                    let speaker = people
                        .iter()
                        .find(|(c, _)| *c == person)
                        .map(|(_, n)| n.clone())
                        .unwrap_or_default();
                    for (c, name) in &people {
                        if *c == person
                            || speaker.contains(name.as_str())
                            || Some(*c) == characters::player_character(x.state())
                            || name.is_empty()
                            || !json.contains(name.as_str())
                        {
                            continue;
                        }
                        let broker_heard = mandate_sim::player::knowledge::knows(
                            x.state(),
                            &Subject::Broker { town: t },
                        ) >= Level::Heard;
                        assert!(
                            knows(*c) >= Level::Heard
                                || (Some(*c) == broker && broker_heard)
                                || e.state().politics.characters.towns[t.index()].rival == Some(*c),
                            "{name} named in an exchange the player has not heard of: {json}"
                        );
                    }
                    // What it says was learned is in the ledger now.
                    assert!(
                        ev.learned
                            .iter()
                            .all(|l| l.level > KnowledgeLevelDto::Unknown)
                    );
                }
            }
        }
        assert!(exchanges > 20, "{exchanges} exchanges sampled");
    }
}

/// The canvass worries are only those the player knows of, gated by level;
/// the match is the engine's (the command's own), shown as far as the
/// player knows the neighbourhood; the outcome's band agrees.
#[test]
fn canvass_worries_are_known_ones_with_the_engines_match() {
    let mut e = game(48);
    wipe(&mut e);
    let t = town(&e);
    let hood = NeighborhoodId(e.state().world.hoods_of(t).next().unwrap() as u32);
    let hs = Subject::HoodConcerns { hood };
    let ts = Subject::TownConcerns { town: t };
    // Knowing nothing: no worry to raise (only "no particular issue").
    let v = canvass_view(&e, hood.0).unwrap();
    assert!(v.worries.is_empty());
    assert_eq!(v.knowledge.level, KnowledgeLevelDto::Unknown);
    // Heard of the neighbourhood's worries: its top one, no match shown.
    set(&mut e, &hs, Level::Heard);
    let v = canvass_view(&e, hood.0).unwrap();
    assert_eq!(v.worries.len(), 1);
    assert!(v.worries[0].top && v.worries[0].source == "hood");
    assert!(v.worries[0].fit.is_none() && v.worries[0].fit_band.is_none());
    let hood_top = mandate_protocol::concerns_for(
        e.state(),
        e.content(),
        mandate_sim::concerns::Scope::Neighbourhood(hood),
    );
    assert_eq!(v.worries[0].id, hood_top[0].id);
    // The town's worries add theirs (known: three), still no match.
    set(&mut e, &ts, Level::Known);
    let v = canvass_view(&e, hood.0).unwrap();
    assert!(v.worries.len() <= 4);
    assert!(v.worries.iter().all(|w| w.fit_band.is_none()));
    // Known: bands; detailed: the engine's multiplier itself.
    set(&mut e, &hs, Level::Known);
    let v = canvass_view(&e, hood.0).unwrap();
    assert!(
        v.worries
            .iter()
            .all(|w| w.fit_band.is_some() && w.fit.is_none())
    );
    set(&mut e, &hs, Level::Detailed);
    let v = canvass_view(&e, hood.0).unwrap();
    for w in &v.worries {
        let m = mandate_sim::actions::canvass_match(e.state(), e.content(), hood, Some(&w.id));
        assert_eq!(w.fit, Some(m));
        assert_eq!(w.fit_band.as_deref(), Some(fit_band(e.content(), m)));
    }
    // A worry the player does not know of is never listed.
    let all = mandate_protocol::concerns_for(
        e.state(),
        e.content(),
        mandate_sim::concerns::Scope::Neighbourhood(hood),
    );
    wipe(&mut e);
    set(&mut e, &hs, Level::Heard);
    let v = canvass_view(&e, hood.0).unwrap();
    for c in all.iter().skip(1) {
        assert!(
            !v.worries.iter().any(|w| w.id == c.id),
            "{} is hidden",
            c.id
        );
    }
    // Canvass on the shown worry: the outcome's band is the preview's.
    set(&mut e, &hs, Level::Detailed);
    let v = canvass_view(&e, hood.0).unwrap();
    let w = v.worries[0].clone();
    let r = e
        .apply(e.route(PlayerCommand::Act {
            action: Action::Canvass {
                neighborhood: hood.0,
                issue: Some(w.id.clone()),
            },
            late_night: false,
        }))
        .unwrap()
        .result
        .unwrap();
    assert_eq!(r.args.get("fit").map(String::as_str), w.fit_band.as_deref());
}

/// Every key the talk views emit is in the English locale.
#[test]
fn talk_keys_are_in_the_locale() {
    let c = content();
    let ftl = c.locale_text("en");
    let has = |k: &str| {
        ftl.lines()
            .any(|l| l.split_once(" =").is_some_and(|(id, _)| id.trim() == k))
    };
    let mut keys: Vec<String> = [
        "time",
        "late-used",
        "late-here",
        "needs-subject",
        "role",
        "place",
        "not-now",
        "owe",
        "away",
        "about",
        "other",
    ]
    .iter()
    .map(|k| format!("talk-why-{k}"))
    .collect();
    keys.push("ui-your-rival".into());
    for t in &c.talk_topics {
        keys.push(t.name_key.clone());
    }
    for k in ["outcome-talk", "outcome-talk-again"] {
        keys.push(k.into());
    }
    for k in keys {
        assert!(has(&k), "{k} is missing from the English locale");
    }
}

/// Talk views never write: the state hash is unchanged by any query.
#[test]
fn talk_views_never_write() {
    let mut e = game(49);
    let at = council_spot(&mut e);
    go(&mut e, at);
    let person = talk::present(e.state(), e.content())[0];
    let r = e
        .apply(talk_cmd(person, "topic.small_talk", None, false))
        .unwrap()
        .result
        .unwrap();
    let before = e.state_hash();
    let t = town(&e);
    let _ = talk_view(&e, TalkTargetDto::Person { person: person.0 }, false);
    let _ = talk_view(&e, TalkTargetDto::Person { person: person.0 }, true);
    let _ = exchange_view(&e, r.created.unwrap());
    for h in e.state().world.hoods_of(t).collect::<Vec<_>>() {
        let _ = canvass_view(&e, h as u32);
    }
    let _ = notebook_view(&e);
    assert_eq!(e.state_hash(), before);
}

/// The worker takes the new requests: the talk panel, a `Talk` command,
/// the exchange it created, and the canvass preview.
#[test]
fn the_session_talks() {
    let mut s = Session::new().unwrap();
    let send = |s: &mut Session, id: u32, body: RequestBody| -> ResponseBody {
        let r: Response = serde_json::from_str(
            &s.handle_json(
                &serde_json::to_string(&Request {
                    protocol_version: PROTOCOL_VERSION,
                    request_id: id,
                    body,
                })
                .unwrap(),
            ),
        )
        .unwrap();
        match r.result {
            ResponseResult::Ok { body } => body,
            ResponseResult::Err { error } => panic!("request {id}: {}", error.message),
        }
    };
    send(
        &mut s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed: 50,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
        }),
    );
    let e = s.engine().unwrap();
    // Where the engine says a council member is now.
    let at = mandate_sim::world::Jurisdiction::town(town(e));
    let m = mandate_sim::politics::body_members_at(&e.state().politics, at)
        .into_iter()
        .filter_map(|p| e.state().politics.get(p))
        .find(|p| !p.is_player)
        .map(|p| p.character)
        .unwrap();
    let chamber =
        mandate_sim::characters::routines::whereabouts(e.state(), e.content(), m).unwrap();
    send(
        &mut s,
        2,
        RequestBody::Command(CommandDto::MoveTo {
            location: LocationDto::of(chamber),
        }),
    );
    let person = talk::present(s.engine().unwrap().state(), s.engine().unwrap().content())[0];
    let ResponseBody::Talk(v) = send(
        &mut s,
        3,
        RequestBody::Query(QueryDto::Talk {
            to: TalkTargetDto::Person { person: person.0 },
            late_night: false,
        }),
    ) else {
        panic!("a talk view")
    };
    let topic = v.topics.iter().find(|t| t.available).unwrap();
    let ResponseBody::CommandApplied(a) = send(
        &mut s,
        4,
        RequestBody::Command(CommandDto::Talk {
            to: TalkTargetDto::Person { person: person.0 },
            topic: topic.id.clone(),
            about: None,
            late_night: false,
        }),
    ) else {
        panic!("applied")
    };
    let id = a.result.unwrap().created.unwrap();
    let ResponseBody::Exchange(x) = send(&mut s, 5, RequestBody::Query(QueryDto::Exchange { id }))
    else {
        panic!("an exchange")
    };
    assert_eq!(x.topic, topic.id);
    let hood = s
        .engine()
        .unwrap()
        .state()
        .world
        .hoods_of(town(s.engine().unwrap()))
        .next()
        .unwrap() as u32;
    let ResponseBody::Canvass(c) = send(
        &mut s,
        6,
        RequestBody::Query(QueryDto::Canvass { neighborhood: hood }),
    ) else {
        panic!("a canvass view")
    };
    assert_eq!(c.neighborhood, hood);
}

/// A passer-by is talked to directly (met as the talk starts): their panel
/// says what the command does, and meeting them is learned.
#[test]
fn passers_by_are_met_as_the_talk_starts() {
    let mut e = game(51);
    wipe(&mut e);
    let t = town(&e);
    let hood = NeighborhoodId(e.state().world.hoods_of(t).next().unwrap() as u32);
    go(&mut e, Location::Street { neighborhood: hood });
    let loc = e.state().player.as_ref().unwrap().location;
    let slots: Vec<u8> = characters::ambient::ambient_at(e.state(), e.content(), loc)
        .into_iter()
        .map(|(s, _)| s)
        .collect();
    assert!(!slots.is_empty(), "passers-by on the streets");
    let mut talked = 0;
    for slot in slots {
        let before = e.state_hash();
        let v = talk_view(&e, TalkTargetDto::Ambient { slot }, false).unwrap();
        assert_eq!(e.state_hash(), before, "the view met nobody");
        assert!(v.here && v.person.id.is_none() && v.person.ambient_slot == Some(slot));
        assert_eq!(v.person.name, "", "a stranger until met");
        for topic in &v.topics {
            if topic.about_required {
                continue;
            }
            let mut x = copy(&e);
            let r = x.apply(Command::Player(PlayerCommand::Talk {
                to: TalkTarget::Ambient { slot },
                topic: topic.id.clone(),
                about: None,
                late_night: false,
            }));
            assert_eq!(r.is_ok(), topic.available, "{}", topic.id);
            if let Ok(o) = r {
                let ev = exchange_view(&x, o.result.unwrap().created.unwrap()).unwrap();
                assert!(!ev.speaker.name.is_empty(), "met now");
                let id = ev.speaker.id.unwrap();
                assert!(
                    ev.learned
                        .iter()
                        .any(|l| l.subject == SubjectDto::Person { person: id }),
                    "meeting them is learned"
                );
                talked += 1;
            }
        }
    }
    assert!(talked > 0);
}
