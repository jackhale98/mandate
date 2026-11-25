//! Conversations with fewer options and better answers (M6 note 26):
//!
//! - the talk panel's order comes from public facts only: two people who
//!   differ only in what they secretly know, and in what they would say,
//!   get the same menu;
//! - a miss costs no time, and leaves a lead where the person knows who
//!   would know;
//! - how much is told follows the trust steps: a reply says the deepest the
//!   speaker would tell;
//! - "What's on your mind?" returns an ask, a worry or gossip from the
//!   person's own state;
//! - no nonsense pairing is offered, in the worlds enumerated;
//! - an officeholder asked who runs things answers from their office, never
//!   with a general line, and names nobody below the step that tells it.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_content::Condition;
use mandate_content::conversation::TalkPart;
use mandate_protocol::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::characters::{self, CharacterId, Relationship, knows};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{LeadTarget, Level, Subject};
use mandate_sim::talk::{self, AboutRef, Exchange, ExchangeId, TalkTarget};
use mandate_sim::world::{Jurisdiction, MunicipalityId};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::Arc;

const SEEDS: [u64; 3] = [11, 31, 2026];

fn content() -> Arc<mandate_content::ContentBundle> {
    let (c, _) = mandate_content::load(&mandate_content::embedded::sources()).unwrap();
    Arc::new(c)
}

fn game(seed: u64, c: &Arc<mandate_content::ContentBundle>) -> Engine {
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

fn copy(e: &Engine) -> Engine {
    Engine::from_state(e.state().clone(), e.content_arc()).unwrap()
}

fn town(e: &Engine) -> MunicipalityId {
    e.state().player.as_ref().unwrap().home()
}

fn me(e: &Engine) -> CharacterId {
    characters::player_character(e.state()).unwrap()
}

/// Everyone stored in the player's town, but the player.
fn townsfolk(e: &Engine) -> Vec<CharacterId> {
    let st = e.state();
    let t = town(e).0;
    (0..st.politics.characters.people.len() as u32)
        .map(CharacterId)
        .filter(|c| *c != me(e))
        .filter(|c| characters::municipality_of(&st.politics, &st.world, *c) == Some(t))
        .collect()
}

/// Put the player where someone is now (test setup). `false`: they are
/// nowhere to be talked to.
fn meet(e: &mut Engine, who: CharacterId) -> bool {
    let Some(loc) = characters::routines::whereabouts(e.state(), e.content(), who) else {
        return false;
    };
    e.setup(|st| st.player.as_mut().unwrap().location = loc);
    talk::can_talk(e.state(), e.content(), who)
}

/// How someone feels about the player (test setup).
fn relation(e: &mut Engine, who: CharacterId, opinion: f32, trust: f32) {
    let me = me(e);
    e.setup(|st| {
        st.politics.characters.edges.insert(
            (who.0, me.0),
            Relationship {
                opinion,
                trust,
                bonds: Vec::new(),
                familiar: 0,
            },
        );
    });
}

/// No trait shifts what they would tell (test setup).
fn plain(e: &mut Engine, who: CharacterId) {
    e.setup(|st| st.politics.characters.get_mut(who).unwrap().traits.clear());
}

fn ask(e: &mut Engine, who: CharacterId, topic: &str, about: Option<AboutRef>) -> Option<Exchange> {
    let o = e
        .apply(Command::Player(PlayerCommand::Talk {
            to: TalkTarget::Person { person: who },
            topic: topic.into(),
            about,
            late_night: false,
        }))
        .ok()?;
    let id = o.result?.created?;
    e.state()
        .player
        .as_ref()?
        .talk
        .exchange(ExchangeId(id))
        .cloned()
}

fn substance(x: &Exchange) -> &talk::SpokenLine {
    x.parts
        .iter()
        .find(|p| p.part == TalkPart::Substance)
        .expect("a reply has a substance")
}

/// The time the player has used this turn: whole slots, and the half left.
fn clock(e: &Engine) -> (u8, Option<u32>, bool) {
    let pl = e.state().player.as_ref().unwrap();
    (pl.slots_used, pl.talk.half_slot, pl.late_night_used)
}

fn learned(x: &Exchange, s: &Subject) -> Level {
    x.learned
        .iter()
        .filter(|(k, _)| k == s)
        .map(|(_, l)| *l)
        .max()
        .unwrap_or_default()
}

/// The menu as the panel orders it: each topic (not a system's own choice)
/// with its subjects, and which are likely.
fn menu(e: &Engine, who: CharacterId) -> Vec<(String, Vec<(AboutRefDto, bool)>)> {
    let v = talk_view(e, TalkTargetDto::Person { person: who.0 }, false).unwrap();
    v.topics
        .into_iter()
        .filter(|t| t.mechanic != "choice")
        .map(|t| {
            (
                t.id,
                t.about.into_iter().map(|c| (c.about, c.likely)).collect(),
            )
        })
        .collect()
}

#[test]
fn the_menu_does_not_depend_on_hidden_knowledge() {
    let c = content();
    let mut compared = 0;
    for seed in SEEDS {
        let e = game(seed, &c);
        let t = town(&e);
        let broker = e.state().politics.characters.towns[t.index()]
            .broker
            .expect("the town has a broker");
        let member =
            mandate_sim::politics::body_members_at(&e.state().politics, Jurisdiction::town(t))
                .into_iter()
                .filter_map(|p| e.state().politics.get(p))
                .find(|p| !p.is_player)
                .map(|p| p.character)
                .unwrap();
        for who in [broker, member] {
            // One world where they are the broker, cool towards the player
            // and close-mouthed; one where they are not, and are an open
            // friend. Everything anyone can see of them is the same (their
            // traits too: a routine may follow them).
            let mut a = copy(&e);
            let mut b = copy(&e);
            let other = if who == broker { member } else { broker };
            a.setup(|st| {
                st.politics.characters.towns[t.index()].broker = Some(who);
            });
            b.setup(|st| {
                st.politics.characters.towns[t.index()].broker = Some(other);
            });
            relation(&mut a, who, -60.0, 0.1);
            relation(&mut b, who, 80.0, 0.9);
            // The player meets them wherever their day has them (where
            // someone is can be seen; a subject that is the very place
            // they stand in is set aside when the two places differ).
            if !meet(&mut a, who) || !meet(&mut b, who) {
                continue;
            }
            let spots = [&a, &b].map(|x| x.state().player.as_ref().unwrap().location);
            let elsewhere = |m: Vec<(String, Vec<(AboutRefDto, bool)>)>| {
                m.into_iter()
                    .map(|(id, about)| {
                        let about: Vec<_> = about
                            .into_iter()
                            .filter(|(r, _)| match r {
                                AboutRefDto::Place { place } => !spots.contains(&Location::Place {
                                    place: mandate_sim::world::PlaceId(*place),
                                }),
                                _ => true,
                            })
                            .collect();
                        (id, about)
                    })
                    .collect::<Vec<_>>()
            };
            compared += 1;
            // What they would tell does differ...
            let s = Subject::Broker { town: t };
            let tells = |x: &Engine| {
                let cap =
                    knows::discretion_cap(x.state(), &c, who, me(x), knows::Openness::default());
                knows::tells(x.state(), &c, who, &s, cap)
            };
            assert_ne!(
                tells(&a),
                tells(&b),
                "seed {seed}: the two differ in what they would tell"
            );
            // ...what public facts say of them does not, and nor does the menu.
            assert_eq!(
                knows::expected_knows(a.state(), &c, who, &s),
                knows::expected_knows(b.state(), &c, who, &s)
            );
            let (ma, mb) = (elsewhere(menu(&a, who)), elsewhere(menu(&b, who)));
            assert!(ma.iter().any(|(id, _)| id == "topic.who_runs"));
            assert!(
                ma.iter()
                    .any(|(_, about)| about.iter().any(|(_, l)| *l)
                        && about.iter().any(|(_, l)| !*l)),
                "seed {seed}: some subjects are likely and some are not"
            );
            assert_eq!(ma, mb, "seed {seed}: the same menu");
            // Likely subjects come first.
            for (id, about) in &ma {
                let first_other = about.iter().position(|(_, l)| !*l).unwrap_or(about.len());
                assert!(
                    about[first_other..].iter().all(|(_, l)| !*l),
                    "{id}: the likely subjects are listed first"
                );
            }
        }
    }
    assert!(compared >= 3, "{compared} pairs of worlds compared");
}

#[test]
fn a_miss_costs_no_time_and_leaves_a_lead_where_they_know_who_would_know() {
    let c = content();
    let (mut misses, mut person_leads, mut hits) = (0, 0, 0);
    for seed in SEEDS {
        let e = game(seed, &c);
        let t = town(&e);
        let broker = Subject::Broker { town: t };
        for who in townsfolk(&e) {
            let mut x = copy(&e);
            if !meet(&mut x, who) {
                continue;
            }
            relation(&mut x, who, 30.0, 0.6);
            plain(&mut x, who);
            let before = clock(&x);
            let leads_before = x.state().player.as_ref().unwrap().knowledge.leads.len();
            let Some(said) = ask(&mut x, who, "topic.who_runs", None) else {
                continue;
            };
            let free = clock(&x) == before;
            let miss = said.outcome.as_deref() == Some(talk::MISS);
            assert_eq!(
                free, miss,
                "seed {seed}: a miss, and only a miss, costs no time"
            );
            if miss {
                misses += 1;
                assert_eq!(
                    learned(&said, &broker),
                    Level::Unknown,
                    "a miss teaches nothing"
                );
                // The lead is in the notebook, pointing at someone who is
                // expected to know, by public facts.
                for l in &said.leads {
                    if let LeadTarget::Person { person } = l.to {
                        person_leads += 1;
                        assert_ne!(person, who);
                        assert!(
                            knows::expected_knows(x.state(), &c, person, &l.subject) >= l.level,
                            "seed {seed}: the lead points at someone expected to know"
                        );
                        let k = &x.state().player.as_ref().unwrap().knowledge;
                        assert!(k.leads.len() > leads_before, "the notebook gained the lead");
                        assert!(
                            k.level(&Subject::Person { person }) >= Level::Heard,
                            "the player has heard of whoever the lead names"
                        );
                    }
                }
            } else {
                hits += 1;
            }
        }
    }
    assert!(misses > 10, "{misses} misses");
    assert!(person_leads > 3, "{person_leads} leads to a person");
    assert!(hits > 3, "{hits} answers");
}

#[test]
fn a_miss_needs_the_time_to_ask_and_a_neighbourhood_miss_points_somewhere() {
    let c = content();
    let mut pointed = 0;
    for seed in SEEDS {
        let e = game(seed, &c);
        let st = e.state();
        let t = town(&e);
        let hoods: Vec<_> = st
            .world
            .neighborhoods
            .iter()
            .filter(|h| h.municipality == t)
            .map(|h| h.id)
            .collect();
        for who in townsfolk(&e).into_iter().take(60) {
            let mut x = copy(&e);
            if !meet(&mut x, who) {
                continue;
            }
            relation(&mut x, who, 30.0, 0.6);
            let ctx_hood = hoods.iter().copied().find(|h| {
                knows::expected_knows(x.state(), &c, who, &Subject::HoodConcerns { hood: *h })
                    == Level::Unknown
                    && mandate_sim::places::hood_of(
                        x.state(),
                        x.state().player.as_ref().unwrap().location,
                    ) != Some(*h)
            });
            let Some(hood) = ctx_hood else {
                continue;
            };
            let before = clock(&x);
            let Some(said) = ask(&mut x, who, "topic.worries", Some(AboutRef::Hood { hood }))
            else {
                continue;
            };
            assert_eq!(
                said.outcome.as_deref(),
                Some(talk::MISS),
                "seed {seed}: {:?}",
                substance(&said).key
            );
            assert_eq!(clock(&x), before, "it cost no time");
            assert_eq!(
                learned(&said, &Subject::HoodConcerns { hood }),
                Level::Unknown
            );
            if !said.leads.is_empty() {
                pointed += 1;
            }
            // With no time left, nothing can be asked: what is offered
            // never shows who would have known.
            let mut y = copy(&e);
            assert!(meet(&mut y, who));
            y.setup(|st| {
                let pl = st.player.as_mut().unwrap();
                pl.slots_used = pl.slots_used.saturating_add(pl.slots_left());
                pl.talk.half_slot = None;
            });
            assert!(
                ask(&mut y, who, "topic.worries", Some(AboutRef::Hood { hood })).is_none(),
                "no time, no question"
            );
        }
    }
    assert!(
        pointed > 5,
        "{pointed} misses about a neighbourhood left a lead"
    );
}

#[test]
fn depth_follows_the_trust_steps() {
    let c = content();
    let d = &c.talk.discretion;
    // One opinion inside each step the content defines, and the top.
    let mut steps: Vec<f32> = Vec::new();
    let mut low = -100.0;
    for b in &d.bands {
        steps.push(((low + b.below) / 2.0) as f32);
        low = b.below;
    }
    steps.push(((low + 100.0) / 2.0) as f32);
    let mut told_levels = std::collections::BTreeSet::new();
    for seed in SEEDS {
        let e = game(seed, &c);
        let t = town(&e);
        let s = Subject::Broker { town: t };
        let tc = &e.state().politics.characters.towns[t.index()];
        let broker = tc.broker;
        for who in townsfolk(&e) {
            // Someone who knows who runs things, is not the broker, and is
            // not in a post (officeholders: the next test).
            if Some(who) == broker
                || knows::speaker_knows(e.state(), &c, who, &s) < Level::Known
                || e.state()
                    .politics
                    .characters
                    .get(who)
                    .unwrap()
                    .politician
                    .is_some()
            {
                continue;
            }
            let mut last = Level::Unknown;
            for op in &steps {
                let mut x = copy(&e);
                if !meet(&mut x, who) {
                    continue;
                }
                plain(&mut x, who);
                relation(&mut x, who, *op, 0.9);
                let cap =
                    knows::discretion_cap(x.state(), &c, who, me(&x), knows::Openness::default());
                let tells = knows::tells(x.state(), &c, who, &s, cap);
                let said = ask(&mut x, who, "topic.who_runs", None).unwrap();
                assert_eq!(
                    learned(&said, &s),
                    tells,
                    "seed {seed}: at opinion {op} they tell {tells:?}, and say it ({})",
                    substance(&said).key
                );
                assert!(tells >= last, "a warmer step never tells less");
                last = tells;
                told_levels.insert(tells);
                let named = substance(&said).args.iter().any(|(k, _)| k == "broker");
                assert_eq!(named, tells >= Level::Heard, "named only when told");
            }
        }
    }
    assert!(
        told_levels.contains(&Level::Unknown)
            && told_levels.contains(&Level::Heard)
            && told_levels.contains(&Level::Known),
        "the steps were exercised: {told_levels:?}"
    );
}

/// The general "who runs things" lines: informative, and written for no
/// role in particular.
fn general_lines(c: &mandate_content::ContentBundle) -> Vec<String> {
    c.talk_lines
        .iter()
        .filter(|l| l.topic.as_deref() == Some("topic.who_runs") && !l.fallback)
        .filter(|l| {
            !l.when.iter().any(|w| {
                matches!(w, Condition::Occupation { slot, is: true, .. } if slot == "speaker")
                    || matches!(w, Condition::IsBroker { is: true, .. })
            })
        })
        .map(|l| l.id.clone())
        .collect()
}

#[test]
fn an_officeholder_asked_who_runs_things_answers_from_their_office() {
    let c = content();
    let general = general_lines(&c);
    assert!(general.len() >= 10, "the general lines exist: {general:?}");
    let mut named = 0;
    let mut coy = 0;
    for seed in SEEDS {
        let e = game(seed, &c);
        let t = town(&e);
        let s = Subject::Broker { town: t };
        let at = Jurisdiction::town(t);
        let holders: Vec<CharacterId> = e
            .state()
            .politics
            .politicians
            .iter()
            .filter(|p| !p.is_player && p.role.post().is_some_and(|post| post.at() == at))
            // The broker stays coy, in lines of their own.
            .filter(|p| e.state().politics.characters.towns[t.index()].broker != Some(p.character))
            .map(|p| p.character)
            .collect();
        assert!(holders.len() >= 3, "seed {seed}: officeholders");
        for who in holders {
            for op in [-80.0, -30.0, 0.0, 30.0, 80.0] {
                let mut x = copy(&e);
                if !meet(&mut x, who) {
                    continue;
                }
                plain(&mut x, who);
                relation(&mut x, who, op, 0.9);
                // Asked three times in the week (the repetition guard moves
                // through every line that holds).
                for _ in 0..3 {
                    x.setup(|st| {
                        let pl = st.player.as_mut().unwrap();
                        pl.slots_used = 0;
                    });
                    let said = ask(&mut x, who, "topic.who_runs", None).unwrap();
                    let line = substance(&said);
                    assert!(
                        !general.contains(&line.line),
                        "seed {seed}: an officeholder said the general line {}",
                        line.line
                    );
                    let has = |k: &str| line.args.iter().any(|(a, _)| a == k);
                    let fallback = c.talk_lines.iter().any(|l| l.id == line.line && l.fallback);
                    if !fallback {
                        // From their own office and the body, in the
                        // constitution's words.
                        assert!(has("post_key") && has("chamber_key") && has("office_key"));
                        let level = learned(&said, &s);
                        if has("broker") {
                            named += 1;
                            assert!(op >= 15.0, "named only from the step that tells it plainly");
                        } else {
                            coy += 1;
                            assert_eq!(level, Level::Unknown, "nobody named, nothing learned");
                        }
                    }
                }
            }
        }
    }
    assert!(named > 5 && coy > 5, "named {named}, coy {coy}");
}

#[test]
fn whats_on_your_mind_is_an_ask_a_worry_or_gossip_from_their_own_state() {
    let c = content();
    let (mut worries, mut gossip, mut asks, mut nothing) = (0, 0, 0, 0);
    for seed in SEEDS {
        let mut e = game(seed, &c);
        // An ask of the player, if one is offered in the first weeks.
        let mut asker = None;
        for _ in 0..40 {
            asker = e
                .state()
                .asks
                .asks
                .iter()
                .filter(|a| a.open() && !a.promise)
                .find_map(|a| a.asker_person());
            if asker.is_some() {
                break;
            }
            if e.apply(Command::EndTurn).is_err() {
                break;
            }
        }
        if let Some(who) = asker {
            let mut x = copy(&e);
            if meet(&mut x, who) {
                relation(&mut x, who, 30.0, 0.6);
                assert!(talk::open::has_ask(x.state(), who));
                let said = ask(&mut x, who, "topic.open", None).unwrap();
                assert!(
                    substance(&said).key.starts_with("talk-mind-ask"),
                    "{}",
                    substance(&said).key
                );
                assert_eq!(said.about, None);
                assert_ne!(said.outcome.as_deref(), Some(talk::MISS));
                asks += 1;
            }
        }
        let t = town(&e);
        for who in townsfolk(&e).into_iter().take(40) {
            if talk::open::has_ask(e.state(), who) {
                continue;
            }
            let mut x = copy(&e);
            if !meet(&mut x, who) {
                continue;
            }
            plain(&mut x, who);
            relation(&mut x, who, 60.0, 0.9);
            let home = characters::profile(&x.state().politics, &x.state().world, who)
                .map(|p| mandate_sim::world::NeighborhoodId(p.home))
                .unwrap();
            let before = clock(&x);
            let first = ask(&mut x, who, "topic.open", None).unwrap();
            // Their main worry: their own neighbourhood's, as they feel it.
            assert_eq!(
                first.about,
                Some(AboutRef::Hood { hood: home }),
                "seed {seed}"
            );
            assert!(
                substance(&first).key.starts_with("talk-mind-worry"),
                "{}",
                substance(&first).key
            );
            assert!(learned(&first, &Subject::HoodConcerns { hood: home }) >= Level::Heard);
            assert_ne!(clock(&x), before, "an answer costs the time");
            worries += 1;
            // Asked again the same week: something else they hold (gossip
            // about someone they know), or the same again.
            x.setup(|st| st.player.as_mut().unwrap().slots_used = 0);
            let second = ask(&mut x, who, "topic.open", None).unwrap();
            if let Some(AboutRef::Person { person }) = second.about {
                assert_ne!(person, who);
                assert_ne!(person, me(&x));
                assert!(
                    knows::speaker_knows(x.state(), &c, who, &Subject::Person { person })
                        >= Level::Known,
                    "gossip is about someone they know"
                );
                assert!(substance(&second).key.starts_with("talk-mind-gossip"));
                assert!(
                    learned(&second, &Subject::Person { person }) >= Level::Heard
                        || x.state()
                            .player
                            .as_ref()
                            .unwrap()
                            .knowledge
                            .level(&Subject::Person { person })
                            >= Level::Known
                );
                assert_eq!(
                    talk::open::gossip_about(e.state(), &c, who, me(&e), t),
                    Some(person),
                    "the same person all week"
                );
                gossip += 1;
            }
            // Cool towards the player: nothing they would share (a miss).
            let mut y = copy(&e);
            assert!(meet(&mut y, who));
            plain(&mut y, who);
            relation(&mut y, who, -40.0, 0.2);
            let before = clock(&y);
            let cold = ask(&mut y, who, "topic.open", None).unwrap();
            assert_eq!(cold.outcome.as_deref(), Some(talk::MISS));
            assert_eq!(clock(&y), before);
            assert!(substance(&cold).key.starts_with("talk-mind-"));
            nothing += 1;
        }
    }
    assert!(
        worries > 20 && gossip > 10 && nothing > 20,
        "{worries} {gossip} {nothing}"
    );
    assert!(asks >= 1, "an ask was brought up in at least one world");
}

#[test]
fn no_nonsense_pairing_is_offered() {
    let c = content();
    let mut seen = 0;
    let mut asked = 0;
    for seed in SEEDS {
        let e = game(seed, &c);
        let t = town(&e);
        let tc = &e.state().politics.characters.towns[t.index()];
        let rival = tc.rival;
        let mut people = townsfolk(&e);
        // The rival and the broker first: the pairings that went wrong.
        people.sort_by_key(|p| (Some(*p) != rival, Some(*p) != tc.broker));
        for who in people.into_iter().take(45) {
            let mut x = copy(&e);
            if !meet(&mut x, who) {
                continue;
            }
            relation(&mut x, who, 30.0, 0.6);
            let st = x.state();
            let here = st.player.as_ref().unwrap().location;
            let name = characters::profile(&st.politics, &st.world, who)
                .unwrap()
                .full_name();
            let works = knows::workplace(st, who, t).map(|p| p.id);
            let politician = st.politics.characters.get(who).unwrap().politician;
            let v = talk_view(&x, TalkTargetDto::Person { person: who.0 }, false).unwrap();
            for topic in v.topics.iter().filter(|tp| tp.mechanic != "choice") {
                if topic.mechanic == "open" {
                    assert!(topic.about.is_empty(), "the open question takes no subject");
                }
                for ch in &topic.about {
                    seen += 1;
                    let about = ch.about.to_sim();
                    // By what the pairing is, not by a list of cases.
                    match &about {
                        AboutRef::Person { person } => {
                            assert_ne!(*person, who, "seed {seed}: {name} asked about themselves")
                        }
                        AboutRef::Rival => {
                            assert_ne!(
                                rival,
                                Some(who),
                                "seed {seed}: the rival asked about the rival"
                            )
                        }
                        AboutRef::Place { place } => assert!(
                            here != Location::Place { place: *place } || works == Some(*place),
                            "seed {seed}: {name} asked, at a place, about that place from outside"
                        ),
                        AboutRef::Bill { policy } if topic.mechanic != "inform" => assert!(
                            politician.is_none()
                                || st.policy.instances[policy.index()].sponsor != politician,
                            "seed {seed}: {name} lobbied on their own bill"
                        ),
                        _ => {}
                    }
                    assert_ne!(
                        ch.label, name,
                        "seed {seed}: a subject with the speaker's own name"
                    );
                }
                // And the replies: asked on the first subjects of each
                // kind, nobody is told about themselves.
                if topic.mechanic != "inform" {
                    continue;
                }
                let mut kinds = std::collections::BTreeSet::new();
                for ch in topic.about.iter().filter(|ch| ch.available) {
                    if !kinds.insert((ch.kind.clone(), ch.likely)) {
                        continue;
                    }
                    let mut y = copy(&x);
                    let said = ask(&mut y, who, &topic.id, Some(ch.about.to_sim()))
                        .unwrap_or_else(|| panic!("{}: an offered subject is refused", topic.id));
                    asked += 1;
                    let line = substance(&said);
                    assert_ne!(
                        line.key,
                        talk::NOTHING_TO_SAY,
                        "{}: nothing to say",
                        topic.id
                    );
                    for (k, val) in &line.args {
                        if matches!(k.as_str(), "about" | "person" | "rival") {
                            assert_ne!(val, &name, "seed {seed}: {} said of themselves", line.key);
                        }
                    }
                }
            }
        }
    }
    assert!(
        seen > 1500 && asked > 200,
        "{seen} pairings seen, {asked} asked"
    );
}

#[test]
fn the_occupation_condition_can_exclude_a_role() {
    let c = content();
    let e = game(31, &c);
    let pl = e.state().player.as_ref().unwrap();
    let t = town(&e);
    let member = mandate_sim::politics::body_members_at(&e.state().politics, Jurisdiction::town(t))
        .into_iter()
        .filter_map(|p| e.state().politics.get(p))
        .find(|p| !p.is_player)
        .map(|p| p.character)
        .unwrap();
    let role = knows::role_key(e.state(), &c, member).unwrap();
    let other = townsfolk(&e)
        .into_iter()
        .find(|p| knows::role_key(e.state(), &c, *p).is_some_and(|r| r != role))
        .unwrap();
    let holds = |who: CharacterId, is: bool| {
        let ctx = talk::context::Ctx {
            st: e.state(),
            content: &c,
            town: t,
            me: me(&e),
            speaker: who,
            about: None,
            here: pl.location,
            cap: Level::Unknown,
        };
        ctx.holds(
            &Condition::Occupation {
                slot: "speaker".into(),
                occupations: vec![role.clone()],
                is,
            },
            &mut Default::default(),
        )
    };
    assert!(holds(member, true) && !holds(member, false));
    assert!(!holds(other, true) && holds(other, false));
    // In content: `is` is optional, and `false` excludes.
    let parsed: Condition = serde_json::from_str(
        r#"{ "type": "occupation", "slot": "speaker", "is": false, "occupations": ["mayor"] }"#,
    )
    .unwrap();
    assert!(matches!(parsed, Condition::Occupation { is: false, .. }));
    let plain: Condition = serde_json::from_str(
        r#"{ "type": "occupation", "slot": "speaker", "occupations": ["mayor"] }"#,
    )
    .unwrap();
    assert!(matches!(plain, Condition::Occupation { is: true, .. }));
}
