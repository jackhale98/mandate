//! Replies answer what was asked, and what an exchange teaches is what was
//! said (owner reports, 2026-11-20; text audit 1):
//!
//! - asked about their own workplace, people answer from inside it (the
//!   reporter asked about the paper talked about the cost of living);
//! - asked what worries a neighbourhood, people answer about that
//!   neighbourhood, never the town at large;
//! - the worries a speaker names are the worries the notebook records for
//!   the subject the exchange teaches;
//! - meeting someone teaches who they are, not what they believe.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_protocol::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::characters::{self, CharacterId, Relationship};
use mandate_sim::places::Location;
use mandate_sim::talk::{self, AboutRef, TalkTarget};
use mandate_sim::world::NeighborhoodId;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeMap;
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

fn copy(e: &Engine) -> Engine {
    Engine::from_state(e.state().clone(), e.content_arc()).unwrap()
}

fn me(e: &Engine) -> CharacterId {
    characters::player_character(e.state()).unwrap()
}

/// The speaker thinks well enough of the player to answer (test setup).
fn friendly(e: &mut Engine, who: CharacterId) {
    let me = me(e);
    e.setup(|st| {
        st.politics.characters.edges.insert(
            (who.0, me.0),
            Relationship {
                opinion: 30.0,
                trust: 0.6,
                bonds: Vec::new(),
                familiar: 0,
            },
        );
    });
}

fn talk(
    e: &mut Engine,
    person: CharacterId,
    topic: &str,
    about: Option<AboutRef>,
) -> Option<ExchangeView> {
    let o = e
        .apply(Command::Player(PlayerCommand::Talk {
            to: TalkTarget::Person { person },
            topic: topic.into(),
            about,
            late_night: false,
        }))
        .ok()?;
    let id = o.result?.created?;
    exchange_view(e, id)
}

fn substance(v: &ExchangeView) -> &SpokenLineView {
    v.parts.iter().find(|p| p.part == "substance").unwrap()
}

/// The fallback keys of a topic (the safe answers, deflections).
fn fallbacks(e: &Engine, topic: &str) -> Vec<String> {
    e.content()
        .talk_lines
        .iter()
        .filter(|l| l.fallback && l.topic.as_deref() == Some(topic))
        .map(|l| l.key.clone())
        .collect()
}

/// Every place in the player's town, and its streets.
fn spots(e: &Engine) -> Vec<Location> {
    let st = e.state();
    let town = st.player.as_ref().unwrap().municipality;
    let mut v: Vec<Location> = st
        .world
        .places
        .iter()
        .filter(|p| p.municipality == town && p.is_open())
        .map(|p| Location::Place { place: p.id })
        .collect();
    v.extend(st.world.hoods_of(town).map(|h| Location::Street {
        neighborhood: NeighborhoodId(h as u32),
    }));
    v
}

/// Everyone the player can find in town: a copy of the game standing where
/// they are, and who they are.
fn people(e: &Engine) -> Vec<(Engine, CharacterId)> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for loc in spots(e) {
        let mut x = copy(e);
        if x.apply(Command::Player(PlayerCommand::MoveTo { location: loc }))
            .is_err()
        {
            continue;
        }
        for who in talk::present(x.state(), x.content()) {
            if seen.insert(who) {
                out.push((copy(&x), who));
            }
        }
    }
    out
}

/// A person's role and the place they work at (a named character's own).
fn work(e: &Engine, who: CharacterId) -> Option<(String, mandate_sim::world::PlaceId)> {
    let n = e.state().politics.characters.get(who)?.named.as_deref()?;
    Some((n.occupation.clone(), n.place?))
}

/// The owner's case: a reporter asked about the newspaper office answered
/// about the cost of living (a neighbourhood-worries line reading the place
/// as its neighbourhood). Asked about their own workplace, people now
/// answer from inside it, naming it.
#[test]
fn asked_about_their_own_workplace_people_answer_from_inside_it() {
    let mut reporters = 0;
    for seed in [7, 44, 2026, 4242] {
        let base = game(seed);
        for (mut e, who) in people(&base) {
            let Some((role, place)) = work(&e, who) else {
                continue;
            };
            if ["occupation.resident", "occupation.retiree"].contains(&role.as_str()) {
                continue;
            }
            friendly(&mut e, who);
            let Some(v) = talk(&mut e, who, "topic.about", Some(AboutRef::Place { place })) else {
                continue;
            };
            let s = substance(&v);
            assert!(
                s.key.starts_with("talk-about-place-own-"),
                "seed {seed}: asked about their workplace, {} answered with `{}`",
                v.speaker.name,
                s.key
            );
            assert!(
                s.args.contains_key("about") || s.args.contains_key("about_key"),
                "the reply names the place: {:?}",
                s.args
            );
            if s.key.contains("-press") || s.key.contains("-editor") {
                reporters += 1;
            }
        }
    }
    assert!(reporters > 0, "a reporter was asked about the paper");
}

/// Asked about a place that is not theirs, people talk about the place
/// (what it is, who goes there), not their own or the district's worries.
#[test]
fn asked_about_a_place_people_talk_about_the_place() {
    let e0 = game(44);
    let st = e0.state();
    let town = st.player.as_ref().unwrap().municipality;
    let place = st
        .world
        .places
        .iter()
        .find(|p| p.municipality == town && p.kind == mandate_sim::world::PlaceKind::TownHall)
        .unwrap()
        .id;
    let safe = fallbacks(&e0, "topic.about");
    let mut asked = 0;
    for (mut e, who) in people(&e0) {
        if work(&e, who).is_some_and(|(_, w)| w == place) {
            continue;
        }
        friendly(&mut e, who);
        let Some(v) = talk(&mut e, who, "topic.about", Some(AboutRef::Place { place })) else {
            continue;
        };
        let k = &substance(&v).key;
        assert!(
            k.starts_with("talk-about-place-") || safe.contains(k),
            "asked about the town hall: `{k}`"
        );
        assert!(
            !k.starts_with("talk-about-hood"),
            "never the district's worries: `{k}`"
        );
        asked += 1;
    }
    assert!(asked > 0);
}

/// "What are people worried about?" asked of one neighbourhood: the reply
/// is about that neighbourhood (or a safe answer), and the worry named is
/// the one the notebook records for what the exchange taught.
#[test]
fn the_worries_spoken_are_the_worries_noted() {
    let mut checked = 0;
    for seed in [7, 44, 2026] {
        let base = game(seed);
        let st = base.state();
        let town = st.player.as_ref().unwrap().municipality;
        let hoods: Vec<u32> = st.world.hoods_of(town).map(|h| h as u32).collect();
        let safe = fallbacks(&base, "topic.worries");
        for (here, who) in people(&base) {
            for (n, about) in hoods
                .iter()
                .map(|h| {
                    Some(AboutRef::Hood {
                        hood: NeighborhoodId(*h),
                    })
                })
                .chain([None])
                .enumerate()
            {
                if n > 2 && about.is_some() {
                    continue;
                }
                let mut e = copy(&here);
                friendly(&mut e, who);
                let Some(v) = talk(&mut e, who, "topic.worries", about.clone()) else {
                    continue;
                };
                let s = substance(&v);
                if let Some(AboutRef::Hood { hood }) = &about {
                    let line = e
                        .content()
                        .talk_lines
                        .iter()
                        .find(|l| l.key == s.key)
                        .unwrap();
                    assert!(
                        safe.contains(&s.key)
                            || mandate_content::conversation::reads_about(&line.when),
                        "asked about neighbourhood {}: `{}` is about something else",
                        hood.0,
                        s.key
                    );
                }
                // What was said against what was noted.
                let Some(said) = s.args.get("concern_key") else {
                    continue;
                };
                let notebook = notebook_view(&e).unwrap();
                for l in &v.learned {
                    if !matches!(
                        l.subject,
                        SubjectDto::HoodConcerns { .. } | SubjectDto::TownConcerns { .. }
                    ) {
                        continue;
                    }
                    let entry = notebook
                        .sections
                        .iter()
                        .flat_map(|s| &s.entries)
                        .find(|x| x.subject == l.subject)
                        .expect("the notebook has what was learned");
                    assert_eq!(
                        entry.args.get("concern_key"),
                        Some(said),
                        "seed {seed}: `{}` named {said}; the notebook says {:?}",
                        s.key,
                        entry.args.get("concern_key")
                    );
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 0, "some exchanges taught worries");
}

/// Talking to someone is meeting them: the player learns who they are,
/// never their convictions (the notebook's "Cares about").
#[test]
fn meeting_someone_does_not_reveal_what_they_care_about() {
    let base = game(44);
    let mut met = 0;
    for (mut e, who) in people(&base).into_iter().take(12) {
        e.setup(|st| {
            let k = &mut st.player.as_mut().unwrap().knowledge;
            k.lab_omniscient = false;
            k.entries.clear();
        });
        friendly(&mut e, who);
        let Some(v) = talk(&mut e, who, "topic.small_talk", None) else {
            continue;
        };
        let person = v
            .learned
            .iter()
            .find(|l| l.subject == SubjectDto::Person { person: who.0 })
            .expect("talking is meeting them");
        assert!(!person.args.contains_key("cares_key"), "{:?}", person.args);
        let n = notebook_view(&e).unwrap();
        let entry: BTreeMap<String, String> = n
            .sections
            .iter()
            .flat_map(|s| &s.entries)
            .find(|x| x.subject == SubjectDto::Person { person: who.0 })
            .map(|x| x.args.clone())
            .unwrap();
        assert!(
            entry.contains_key("name") && entry.contains_key("role_key"),
            "{entry:?}"
        );
        assert!(!entry.contains_key("cares_key"), "{entry:?}");
        met += 1;
    }
    assert!(met > 0);
}
