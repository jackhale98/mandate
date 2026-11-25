//! Conversations in whole games (DEC-093, M6_ENGINE_CONTRACTS §3): every
//! topic, discretion by relationship and traits, voices, the repetition
//! guard, asking again, the half-slot cost, learning with the person as the
//! source, passers-by, determinism, and saves.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::conversation::TalkPart;
use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_save::{RunMode, SaveRequest, decode, encode};
use mandate_sim::actions::{ActionResult, PlayerCommand};
use mandate_sim::characters::knows::{self, Openness};
use mandate_sim::characters::{self, CharacterId, Relationship};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Level, Source, Subject};
use mandate_sim::talk::context::Ctx;
use mandate_sim::talk::{self, AboutRef, Exchange, TalkTarget, reply};
use mandate_sim::world::PlaceKind;
use mandate_sim::{Command, Engine, EngineError};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn content() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn game(seed: u64, c: &Arc<ContentBundle>) -> Engine {
    generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap()
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

fn me(e: &Engine) -> CharacterId {
    characters::player_character(e.state()).unwrap()
}

fn town(e: &Engine) -> mandate_sim::world::MunicipalityId {
    e.state().player.as_ref().unwrap().municipality
}

fn go(e: &mut Engine, loc: Location) {
    e.apply(Command::Player(PlayerCommand::MoveTo { location: loc }))
        .unwrap();
}

/// The council's members who are not the rival (nor the player), and the
/// player where the first of them is now (their daily routine, DEC-031).
fn at_chamber(e: &mut Engine) -> Vec<CharacterId> {
    let rival = e.state().politics.characters.towns[town(e).index()].rival;
    let at = mandate_sim::world::Jurisdiction::town(town(e));
    let v: Vec<CharacterId> = mandate_sim::politics::body_members_at(&e.state().politics, at)
        .into_iter()
        .filter_map(|p| e.state().politics.get(p))
        .filter(|p| !p.is_player)
        .map(|p| p.character)
        .filter(|c| Some(*c) != rival)
        .collect();
    assert!(v.len() >= 2, "a council to talk to");
    find(e, v[0]);
    v
}

/// Go to where the engine says someone is now, unless they are here.
fn find(e: &mut Engine, person: CharacterId) {
    if !talk::can_talk(e.state(), e.content(), person)
        && let Some(l) = characters::routines::whereabouts(e.state(), e.content(), person)
    {
        go(e, l);
    }
}

fn say(
    e: &mut Engine,
    person: CharacterId,
    topic: &str,
    about: Option<AboutRef>,
) -> Result<ActionResult, EngineError> {
    find(e, person);
    e.apply(Command::Player(PlayerCommand::Talk {
        to: TalkTarget::Person { person },
        topic: topic.into(),
        about,
        late_night: false,
    }))
    .map(|o| o.result.unwrap())
}

fn exchange(e: &Engine, r: &ActionResult) -> Exchange {
    let id = talk::ExchangeId(r.created.unwrap());
    e.state()
        .player
        .as_ref()
        .unwrap()
        .talk
        .exchange(id)
        .unwrap()
        .clone()
}

/// Set how `holder` sees the player.
fn relate(e: &mut Engine, holder: CharacterId, opinion: f32, trust: f32) {
    let me = me(e);
    e.setup(|st| {
        st.politics.characters.edges.insert(
            (holder.0, me.0),
            Relationship {
                opinion,
                trust,
                bonds: Vec::new(),
            },
        );
    });
}

fn set_traits(e: &mut Engine, who: CharacterId, traits: &[&str], boldness: f32, warmth: f32) {
    e.setup(|st| {
        let c = st.politics.characters.get_mut(who).unwrap();
        c.traits = traits.iter().map(|t| t.to_string()).collect();
        c.boldness = boldness;
        c.warmth = warmth;
    });
}

fn plain(e: &mut Engine, who: CharacterId) {
    set_traits(e, who, &[], 0.3, 0.6);
}

fn level(e: &Engine, s: &Subject) -> Level {
    e.state().player.as_ref().unwrap().knowledge.level(s)
}

fn substance(x: &Exchange) -> &str {
    x.parts
        .iter()
        .find(|p| p.part == TalkPart::Substance)
        .map(|p| p.line.as_str())
        .unwrap_or("")
}

fn standard_template(c: &ContentBundle) -> String {
    c.policies
        .values()
        .find(|t| t.tier == mandate_content::TierKey::Local && t.unlock.is_none())
        .unwrap()
        .id
        .clone()
}

#[test]
fn every_topic_works() {
    let c = content();
    let mut e = game(31, &c);
    // A bill to talk about.
    let template = standard_template(&c);
    let r = e
        .apply(e.route(PlayerCommand::DraftPolicy {
            template: template.clone(),
            params: Default::default(),
        }))
        .unwrap();
    let bill = mandate_sim::policy::PolicyInstanceId(r.result.unwrap().created.unwrap());
    let people = at_chamber(&mut e);
    let (a, b) = (people[0], people[1]);
    let topics: Vec<(&str, Option<AboutRef>)> = vec![
        ("topic.worries", None),
        ("topic.who_runs", None),
        ("topic.about", Some(AboutRef::Person { person: b })),
        (
            "topic.pitch",
            Some(AboutRef::Template {
                template: template.clone(),
            }),
        ),
        ("topic.pitch", Some(AboutRef::Bill { policy: bill })),
        ("topic.support", None),
        ("topic.support", Some(AboutRef::Bill { policy: bill })),
        ("topic.favour", Some(AboutRef::Bill { policy: bill })),
        ("topic.small_talk", None),
        ("topic.open", None),
    ];
    assert_eq!(c.talk_topics.len(), 8, "the eight base topics");
    for (i, (topic, about)) in topics.into_iter().enumerate() {
        if i % 2 == 0 {
            e.apply(Command::EndTurn).unwrap();
        }
        let r = say(&mut e, a, topic, about.clone()).unwrap_or_else(|x| panic!("{topic}: {x}"));
        let x = exchange(&e, &r);
        assert_eq!(x.topic, topic);
        assert_eq!(x.speaker, a);
        // The open question is about what the engine chose (M6 note 26).
        if topic != "topic.open" {
            assert_eq!(x.about, about);
        }
        assert!(!substance(&x).is_empty(), "{topic}: a substance line");
        assert!(
            x.parts
                .iter()
                .all(|p| p.args.iter().any(|(k, _)| k == "speaker")),
            "{topic}: every line names the speaker"
        );
        assert!(
            r.outcome_key.starts_with("outcome-talk"),
            "{}",
            r.outcome_key
        );
        assert_eq!(r.args.get("name").map(|s| !s.is_empty()), Some(true));
    }
    // Mechanics left their marks through the owning domains.
    let st = e.state();
    let pol = st
        .politics
        .get(st.politics.characters.get(a).unwrap().politician.unwrap())
        .unwrap();
    assert!(
        st.politics.sway.contains_key(&(pol.id.0, bill.0)),
        "the pitch and the favour sway the bill"
    );
    assert!(
        characters::favours_owed(&st.politics, me(&e), a) >= 1,
        "a favour offered"
    );
    assert!(
        st.politics
            .characters
            .get(a)
            .unwrap()
            .memories
            .iter()
            .any(|m| m.kind == characters::MemoryKind::Met),
        "small talk is remembered"
    );
    // Topics are offered by role, and subjects must be real.
    // At the council chamber itself: `say` follows `a` wherever their
    // routine takes them (a diner, where talking late is allowed, in some
    // worlds), so the late-night rule is checked in the chamber, not where
    // `a` happens to be.
    let chamber = e
        .state()
        .world
        .places
        .iter()
        .find(|p| p.municipality == town(&e) && p.kind == PlaceKind::CouncilChamber)
        .map(|p| Location::Place { place: p.id })
        .expect("the town has a council chamber");
    go(&mut e, chamber);
    assert!(
        !talk::late_ok(e.state(), e.content()),
        "the chamber offers no late-night talk"
    );
    let late = e.apply(Command::Player(PlayerCommand::Talk {
        to: TalkTarget::Person { person: a },
        topic: "topic.small_talk".into(),
        about: None,
        late_night: true,
    }));
    assert!(late.is_err(), "no late conversations at the chamber");
    assert!(say(&mut e, a, "topic.nonsense", None).is_err());
    assert!(
        say(&mut e, a, "topic.pitch", None).is_err(),
        "a pitch needs a subject"
    );
    let myself = me(&e);
    assert!(
        say(
            &mut e,
            a,
            "topic.about",
            Some(AboutRef::Person { person: myself })
        )
        .is_err()
    );
    assert!(say(&mut e, myself, "topic.small_talk", None).is_err());
}

#[test]
fn favours_are_traded_only_with_politicians() {
    let c = content();
    let mut e = game(32, &c);
    // A Named person at some place in town.
    let t = town(&e);
    let spot = e
        .state()
        .world
        .places
        .iter()
        .filter(|p| p.municipality == t)
        .map(|p| Location::Place { place: p.id })
        .find(|l| {
            characters::ambient::stored_at(e.state(), e.content(), *l)
                .iter()
                .any(|c| {
                    e.state()
                        .politics
                        .characters
                        .get(*c)
                        .unwrap()
                        .named
                        .is_some()
                })
        })
        .unwrap();
    go(&mut e, spot);
    let named = talk::present(e.state(), e.content())
        .into_iter()
        .find(|c| {
            e.state()
                .politics
                .characters
                .get(*c)
                .unwrap()
                .named
                .is_some()
        })
        .unwrap();
    let r = say(
        &mut e,
        named,
        "topic.favour",
        Some(AboutRef::Template {
            template: standard_template(&c),
        }),
    );
    assert!(r.is_err());
    // Talking teaches who they are, with them as the source.
    let before = level(&e, &Subject::Person { person: named });
    assert!(before < Level::Known);
    say(&mut e, named, "topic.small_talk", None).unwrap();
    let k = &e.state().player.as_ref().unwrap().knowledge;
    let entry = k.entry(&Subject::Person { person: named }).unwrap();
    assert_eq!(entry.level, Level::Known);
    // Arriving may have taught their name (`heard`); meeting them is the talk.
    let known = entry
        .learned
        .iter()
        .find(|l| l.level == Level::Known)
        .unwrap();
    assert_eq!(
        known.source,
        Source::Talk {
            person: named,
            topic: "topic.small_talk".into()
        }
    );
}

#[test]
fn discretion_follows_the_relationship_and_traits() {
    let c = content();
    let mut e = game(33, &c);
    let people = at_chamber(&mut e);
    let s = people[0];
    let me = me(&e);
    let cap = |e: &Engine, open: Openness| knows::discretion_cap(e.state(), &c, s, me, open);
    let shut = Openness::default();
    plain(&mut e, s);
    for (op, trust, want) in [
        (-70.0, 0.9, Level::Unknown), // hostile
        (-30.0, 0.9, Level::Unknown), // cool
        (0.0, 0.5, Level::Heard),     // neutral
        (30.0, 0.5, Level::Known),    // warm
        (70.0, 0.7, Level::Detailed), // friendly and trusting
        (70.0, 0.3, Level::Known),    // friendly, not trusting
    ] {
        relate(&mut e, s, op, trust);
        assert_eq!(cap(&e, shut), want, "opinion {op}, trust {trust}");
    }
    // Schemers and the cautious are one step more guarded.
    relate(&mut e, s, 30.0, 0.5);
    set_traits(&mut e, s, &["trait.schemer"], 0.3, 0.6);
    assert_eq!(cap(&e, shut), Level::Heard);
    set_traits(&mut e, s, &["trait.cautious"], 0.3, 0.6);
    assert_eq!(cap(&e, shut), Level::Heard);
    // The vain open up when flattered; a called-in favour opens anyone.
    set_traits(&mut e, s, &["trait.vain"], 0.3, 0.6);
    assert_eq!(cap(&e, shut), Level::Known);
    let flattered = Openness {
        flattered: true,
        favour_called: false,
    };
    assert_eq!(cap(&e, flattered), Level::Detailed);
    plain(&mut e, s);
    let called = Openness {
        flattered: false,
        favour_called: true,
    };
    assert_eq!(cap(&e, called), Level::Detailed);
    // They tell min(what they know, the cap): a councillor knows the
    // rival's plans (`known`), never more.
    let rival = Subject::RivalPlans { town: town(&e) };
    relate(&mut e, s, 70.0, 0.9);
    assert_eq!(knows::speaker_knows(e.state(), &c, s, &rival), Level::Known);
    assert_eq!(
        knows::tells(e.state(), &c, s, &rival, Level::Detailed),
        Level::Known
    );
    assert_eq!(
        knows::tells(e.state(), &c, s, &rival, Level::Heard),
        Level::Heard
    );
}

#[test]
fn a_cool_colleague_deflects_and_a_warm_one_tells() {
    let c = content();
    let rival_plans = |e: &Engine| Subject::RivalPlans { town: town(e) };
    for (op, learns) in [(-30.0, false), (0.0, false), (30.0, true)] {
        let mut e = game(34, &c);
        let people = at_chamber(&mut e);
        let s = people[0];
        plain(&mut e, s);
        relate(&mut e, s, op, 0.5);
        let before = level(&e, &rival_plans(&e));
        assert!(before < Level::Known);
        let r = say(&mut e, s, "topic.about", Some(AboutRef::Rival)).unwrap();
        let x = exchange(&e, &r);
        let after = level(&e, &rival_plans(&e));
        if learns {
            assert_eq!(substance(&x), "line.about.rival");
            assert_eq!(after, Level::Known);
            let entry = e
                .state()
                .player
                .as_ref()
                .unwrap()
                .knowledge
                .entry(&rival_plans(&e))
                .unwrap()
                .clone();
            assert_eq!(
                entry.learned.last().unwrap().source,
                Source::Talk {
                    person: s,
                    topic: "topic.about".into()
                },
                "learned with the person as the source"
            );
            assert!(x.learned.contains(&(rival_plans(&e), Level::Known)));
        } else {
            assert!(
                substance(&x).starts_with("line.about."),
                "{}",
                substance(&x)
            );
            assert_ne!(
                substance(&x),
                "line.about.rival",
                "opinion {op}: they deflect"
            );
            assert_eq!(after, before);
        }
    }
}

#[test]
fn voices_choose_the_closers() {
    let c = content();
    // A chamber where the two voices close unequally (tests/seeds.json, found
    // again when the world changes).
    let mut e = game(mandate_seeds::seed("voices_close_unequally"), &c);
    let people = at_chamber(&mut e);
    let (smooth, flat) = (people[0], people[1]);
    set_traits(&mut e, smooth, &["trait.charmer"], 0.3, 0.6);
    plain(&mut e, flat);
    let voice_of = |e: &Engine, s: CharacterId| {
        let st = e.state();
        let pl = st.player.as_ref().unwrap();
        let ctx = Ctx {
            st,
            content: &c,
            town: pl.municipality,
            me: me(e),
            speaker: s,
            about: None,
            here: pl.location,
            cap: Level::Unknown,
        };
        reply::voice(&ctx).map(|v| v.id.clone())
    };
    assert_eq!(voice_of(&e, smooth).as_deref(), Some("voice.smooth"));
    assert_eq!(voice_of(&e, flat).as_deref(), Some("voice.plain"));
    set_traits(&mut e, flat, &[], 0.8, 0.2);
    assert_eq!(voice_of(&e, flat).as_deref(), Some("voice.blunt"));
    plain(&mut e, flat);
    let mut closers: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    for turn in 0..12 {
        for (i, s) in [smooth, flat].into_iter().enumerate() {
            let topic = ["topic.worries", "topic.who_runs", "topic.small_talk"][turn % 3];
            let r = say(&mut e, s, topic, None).unwrap();
            let x = exchange(&e, &r);
            closers[i].extend(
                x.parts
                    .iter()
                    .filter(|p| p.part == TalkPart::Closer)
                    .map(|p| p.line.clone()),
            );
        }
        e.apply(Command::EndTurn).unwrap();
    }
    assert!(
        !closers[0].is_empty() && !closers[1].is_empty(),
        "{closers:?}"
    );
    assert!(
        closers[0]
            .iter()
            .all(|l| l.starts_with("line.close.smooth.")),
        "{closers:?}"
    );
    assert!(
        closers[1]
            .iter()
            .all(|l| l.starts_with("line.close.plain.")),
        "{closers:?}"
    );
    // The smooth voice closes more often (0.7 against 0.3).
    assert!(closers[0].len() > closers[1].len(), "{closers:?}");
}

#[test]
fn people_never_repeat_a_line_while_another_holds() {
    let c = content();
    let mut e = game(36, &c);
    let people = at_chamber(&mut e);
    let s = people[0];
    let me = me(&e);
    let topics = [
        "topic.worries",
        "topic.who_runs",
        "topic.about",
        "topic.pitch",
    ];
    let mut checked = 0;
    for i in 0..30 {
        if i % 6 == 0 {
            e.apply(Command::EndTurn).unwrap();
            // Relationships drift; vary them so different lines hold.
            relate(&mut e, s, [-30.0, 0.0, 30.0, 70.0][(i / 6) % 4], 0.6);
        }
        let topic = topics[i % topics.len()];
        let about = match topic {
            "topic.about" => Some(AboutRef::Person { person: people[1] }),
            "topic.pitch" => Some(AboutRef::Template {
                template: standard_template(&c),
            }),
            _ => None,
        };
        // The candidates as the speaker sees them now.
        let (cands, used) = {
            let st = e.state();
            let pl = st.player.as_ref().unwrap();
            let cap = knows::discretion_cap(st, &c, s, me, Openness::default());
            let ctx = Ctx {
                st,
                content: &c,
                town: pl.municipality,
                me,
                speaker: s,
                about: about.as_ref(),
                here: pl.location,
                cap,
            };
            let t = c.talk_topic(topic).unwrap();
            let v = reply::voice(&ctx);
            let cands: Vec<(TalkPart, Vec<String>)> = TalkPart::ALL
                .into_iter()
                .map(|p| {
                    let (x, _) = reply::candidates(&ctx, t, p, v);
                    (p, x.into_iter().map(|(l, _)| l.id.clone()).collect())
                })
                .collect();
            let used: Vec<String> = pl.talk.used_by(s).iter().map(|u| u.line.clone()).collect();
            (cands, used)
        };
        let r = say(&mut e, s, topic, about).unwrap();
        let x = exchange(&e, &r);
        for p in &x.parts {
            let all = &cands.iter().find(|(k, _)| *k == p.part).unwrap().1;
            assert!(all.contains(&p.line), "{} is a candidate", p.line);
            let fresh_exists = all.iter().any(|l| !used.contains(l));
            if fresh_exists {
                assert!(
                    !used.contains(&p.line),
                    "exchange {i}: {} repeated while another held ({all:?}, used {used:?})",
                    p.line
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 10, "the guard was exercised ({checked})");
    // The memory is short: at most `memory_lines` per person.
    let used = e.state().player.as_ref().unwrap().talk.used_by(s).len();
    assert!(used <= c.talk.memory_lines as usize);
}

#[test]
fn asking_again_gives_another_line_and_nothing_new() {
    let c = content();
    let mut e = game(37, &c);
    let people = at_chamber(&mut e);
    let s = people[0];
    plain(&mut e, s);
    relate(&mut e, s, 30.0, 0.5);
    let first = say(&mut e, s, "topic.worries", None).unwrap();
    let a = exchange(&e, &first);
    assert!(!a.repeat);
    // (They may have moved on as the day went on: go to them first.)
    find(&mut e, s);
    let ledger = e.state().player.as_ref().unwrap().knowledge.clone();
    let second = say(&mut e, s, "topic.worries", None).unwrap();
    let b = exchange(&e, &second);
    assert!(b.repeat);
    assert_eq!(second.outcome_key, "outcome-talk-again");
    assert!(b.learned.is_empty() && b.leads.is_empty());
    // Nothing new learned (where people were seen is noted, not learned).
    let mut now = e.state().player.as_ref().unwrap().knowledge.clone();
    now.seen = ledger.seen.clone();
    assert_eq!(now, ledger, "nothing new");
    let lines = |x: &Exchange| x.parts.iter().map(|p| p.line.clone()).collect::<Vec<_>>();
    assert_ne!(lines(&a), lines(&b), "a different line");
    assert_ne!(
        substance(&a),
        substance(&b),
        "a different substance while another holds"
    );
    assert!(talk::asked_this_week(e.state(), s, "topic.worries"));
    // Unless the relationship changed: a warmer colleague tells more.
    let rival = Subject::RivalPlans { town: town(&e) };
    relate(&mut e, s, -30.0, 0.5);
    say(&mut e, s, "topic.about", Some(AboutRef::Rival)).unwrap();
    assert!(level(&e, &rival) < Level::Known);
    relate(&mut e, s, 30.0, 0.5);
    let r = say(&mut e, s, "topic.about", Some(AboutRef::Rival)).unwrap();
    assert!(exchange(&e, &r).repeat);
    assert_eq!(level(&e, &rival), Level::Known, "the relationship changed");
    // Another week: a fresh ask.
    for _ in 0..2 {
        e.apply(Command::EndTurn).unwrap();
    }
    let r = say(&mut e, s, "topic.worries", None).unwrap();
    assert!(!exchange(&e, &r).repeat);
}

#[test]
fn a_conversation_costs_half_a_slot() {
    let c = content();
    let mut e = game(38, &c);
    let people = at_chamber(&mut e);
    let s = people[0];
    let used = |e: &Engine| e.state().player.as_ref().unwrap().slots_used;
    let start = used(&e);
    say(&mut e, s, "topic.small_talk", None).unwrap();
    assert_eq!(
        used(&e),
        start + 1,
        "the first takes a slot, half of it left"
    );
    say(&mut e, s, "topic.worries", None).unwrap();
    assert_eq!(used(&e), start + 1, "the second uses the half left");
    say(&mut e, s, "topic.who_runs", None).unwrap();
    assert_eq!(used(&e), start + 2);
    // Out of time: only the half left can be used.
    let cap = e.state().player.as_ref().unwrap().slot_cap();
    e.setup(|st| st.player.as_mut().unwrap().slots_used = cap);
    say(
        &mut e,
        s,
        "topic.about",
        Some(AboutRef::Person { person: people[1] }),
    )
    .unwrap();
    assert!(
        say(&mut e, s, "topic.worries", None).is_err(),
        "no time left"
    );
    // A half slot does not carry over to the next turn.
    e.apply(Command::EndTurn).unwrap();
    say(&mut e, s, "topic.worries", None).unwrap();
    assert_eq!(used(&e), 1);
    e.apply(Command::EndTurn).unwrap();
    assert_eq!(used(&e), 0);
    say(&mut e, s, "topic.small_talk", None).unwrap();
    assert_eq!(used(&e), 1, "the stale half slot is not used");
}

#[test]
fn passers_by_are_met_by_talking() {
    let c = content();
    let mut e = game(39, &c);
    let t = town(&e);
    let streets = e
        .state()
        .world
        .hoods_of(t)
        .map(|h| Location::Street {
            neighborhood: mandate_sim::world::NeighborhoodId(h as u32),
        })
        .find(|l| !characters::ambient::ambient_at(e.state(), &c, *l).is_empty())
        .unwrap();
    go(&mut e, streets);
    let slot = characters::ambient::ambient_at(e.state(), &c, streets)[0].0;
    let n = e.state().politics.characters.people.len();
    let bad = e.apply(Command::Player(PlayerCommand::Talk {
        to: TalkTarget::Ambient { slot },
        topic: "topic.nonsense".into(),
        about: None,
        late_night: false,
    }));
    assert!(bad.is_err());
    assert_eq!(
        e.state().politics.characters.people.len(),
        n,
        "a refused talk changes nothing"
    );
    let r = e
        .apply(Command::Player(PlayerCommand::Talk {
            to: TalkTarget::Ambient { slot },
            topic: "topic.worries".into(),
            about: None,
            late_night: false,
        }))
        .unwrap()
        .result
        .unwrap();
    assert_eq!(
        e.state().politics.characters.people.len(),
        n + 1,
        "promoted"
    );
    let x = exchange(&e, &r);
    assert_eq!(x.speaker.0 as usize, n);
    assert!(talk::can_talk(e.state(), e.content(), x.speaker));
}

/// A scripted run with conversations: the same seed and commands give the
/// same state; saving midway and continuing matches.
fn script(e: &mut Engine, c: &ContentBundle, from: usize, to: usize) {
    for step in from..to {
        let people = at_chamber(e);
        let topic = [
            "topic.worries",
            "topic.who_runs",
            "topic.small_talk",
            "topic.about",
        ][step % 4];
        let about = (topic == "topic.about").then_some(AboutRef::Rival);
        let _ = say(e, people[step % people.len()], topic, about);
        let _ = say(
            e,
            people[(step + 1) % people.len()],
            "topic.pitch",
            Some(AboutRef::Template {
                template: standard_template(c),
            }),
        );
        e.apply(Command::EndTurn).unwrap();
    }
}

#[test]
fn conversations_are_deterministic_and_survive_saves() {
    let c = content();
    let mut a = game(40, &c);
    let mut b = game(40, &c);
    script(&mut a, &c, 0, 6);
    script(&mut b, &c, 0, 6);
    assert_eq!(a.state_hash(), b.state_hash());
    let log = &a.state().player.as_ref().unwrap().talk.log;
    assert!(log.len() >= 10, "talked: {}", log.len());
    // Save, load, continue.
    let (bytes, meta) = encode(b.state(), &req()).unwrap();
    assert_eq!(meta.format_version, mandate_save::SAVE_FORMAT_VERSION);
    let (state, _) = decode(&bytes).unwrap();
    assert_eq!(state.hash(), b.state_hash());
    assert_eq!(
        state.player.as_ref().unwrap().talk,
        b.state().player.as_ref().unwrap().talk
    );
    let mut b2 = Engine::from_state(state, c.clone()).unwrap();
    script(&mut a, &c, 6, 10);
    script(&mut b2, &c, 6, 10);
    assert_eq!(a.state_hash(), b2.state_hash());
    // A different seed talks differently.
    let mut d = game(41, &c);
    script(&mut d, &c, 0, 6);
    let lines = |e: &Engine| {
        e.state()
            .player
            .as_ref()
            .unwrap()
            .talk
            .log
            .iter()
            .map(|x| x.parts.clone())
            .collect::<Vec<_>>()
    };
    assert_ne!(lines(&d), lines(&a));
}

#[test]
fn older_saves_migrate_to_empty_talk() {
    let c = content();
    let mut e = game(42, &c);
    script(&mut e, &c, 0, 2);
    assert!(!e.state().player.as_ref().unwrap().talk.log.is_empty());
    let mut v16 = serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap() });
    v16.pointer_mut("/state/player")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("talk");
    let migrated = mandate_save::migrate(16, v16).unwrap();
    let state: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    assert_eq!(
        state.player.as_ref().unwrap().talk,
        talk::TalkState::default()
    );
    let mut e2 = Engine::from_state(state, c.clone()).unwrap();
    script(&mut e2, &c, 2, 3);
    assert!(!e2.state().player.as_ref().unwrap().talk.log.is_empty());
}
