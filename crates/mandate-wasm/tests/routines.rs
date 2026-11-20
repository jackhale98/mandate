//! Daily routines and parity (DEC-031, DEC-051; task S1): "Go to them"
//! reaches every stored character at any time of any turn, by routing to
//! where the engine says they are now; a room lists everyone there (a
//! councillor at lunch in the diner is a visitor); the notebook and the
//! character sheet show where someone was last seen and, once seen there
//! twice, a lead to where they usually are.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_core::SimRng;
use mandate_protocol::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::characters::routines::{self, DayPart, Now};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Level, Source, Subject};
use mandate_sim::world::PlaceKind;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::Arc;

fn content() -> Arc<mandate_content::ContentBundle> {
    let (c, _) = mandate_content::load(&mandate_content::embedded::sources()).unwrap();
    Arc::new(c)
}

fn game(seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), content()).unwrap()
}

fn town(e: &Engine) -> mandate_sim::world::MunicipalityId {
    e.state().player.as_ref().unwrap().home()
}

fn go(e: &mut Engine, loc: Location) {
    e.apply(Command::Player(PlayerCommand::MoveTo { location: loc }))
        .unwrap();
}

#[test]
fn go_to_them_reaches_every_stored_character_at_random_times() {
    let mut r = SimRng::from_seed_bytes([7; 32]);
    for seed in [5u64, 4242] {
        let mut e = game(seed);
        for round in 0..4 {
            // A random turn and a random point in it (any part, late night too).
            for _ in 0..r.below(3) {
                e.apply(Command::EndTurn).unwrap();
            }
            let cap = e.state().player.as_ref().unwrap().slot_cap();
            let used = r.below(cap as u64 + 1) as u8;
            e.setup(|st| st.player.as_mut().unwrap().slots_used = used);
            let folk = routines::townsfolk(e.state(), town(&e));
            assert!(!folk.is_empty());
            for (i, id) in folk.iter().enumerate() {
                // The People tab's talk panel says where they are, and goes there.
                let v = talk_view(&e, TalkTargetDto::Person { person: id.0 }, false)
                    .unwrap_or_else(|| panic!("seed {seed}: no talk panel for {id:?}"));
                if !v.here {
                    let route = v
                        .route
                        .unwrap_or_else(|| panic!("seed {seed} round {round}: no way to {id:?}"));
                    let to = route.to.unwrap().location.to_sim();
                    assert_eq!(talk_location(&e, *id), Some(to));
                    go(&mut e, to);
                }
                assert!(
                    mandate_sim::talk::can_talk(e.state(), e.content(), *id),
                    "seed {seed} round {round}: {id:?} not talkable where the engine sent us"
                );
                // Every few, the whole panel as the client reads it (slow).
                if i % 7 == 0 {
                    let v = talk_view(&e, TalkTargetDto::Person { person: id.0 }, false).unwrap();
                    assert!(v.here && v.person.can_talk);
                    // In a room, they are listed (the room draws everyone there).
                    let here = e.state().player.as_ref().unwrap().location;
                    if matches!(here, Location::Place { .. }) {
                        let panel = place_panel_view(&e, LocationDto::of(here)).unwrap();
                        assert!(panel.people.iter().any(|p| p.id == Some(id.0)));
                    }
                }
            }
        }
    }
}

#[test]
fn a_councillor_at_the_diner_is_a_visitor_there() {
    let c = content();
    for seed in [3u64, 9, 21, 31] {
        let mut e = game(seed);
        let at = mandate_sim::world::Jurisdiction::town(town(&e));
        let members: Vec<CharacterId> =
            mandate_sim::politics::body_members_at(&e.state().politics, at)
                .into_iter()
                .filter_map(|p| e.state().politics.get(p))
                .filter(|p| !p.is_player)
                .map(|p| p.character)
                .collect();
        // Find a turn and part where a member is in the diner.
        for _ in 0..12 {
            for used in 0..3u8 {
                e.setup(|st| st.player.as_mut().unwrap().slots_used = used);
                let st = e.state();
                let hit = members.iter().find_map(|m| {
                    let l = routines::whereabouts(st, &c, *m)?;
                    let Location::Place { place } = l else {
                        return None;
                    };
                    (st.world.places[place.index()].kind == PlaceKind::Diner).then_some((*m, l))
                });
                if let Some((m, l)) = hit {
                    go(&mut e, l);
                    let panel = place_panel_view(&e, LocationDto::of(l)).unwrap();
                    let i = panel
                        .people
                        .iter()
                        .position(|p| p.id == Some(m.0))
                        .expect("listed in the diner");
                    assert_eq!(panel.here_as[i], "visitor");
                    let v = talk_view(&e, TalkTargetDto::Person { person: m.0 }, false).unwrap();
                    assert!(v.here && v.person.can_talk);
                    return;
                }
            }
            e.apply(Command::EndTurn).unwrap();
        }
    }
    panic!("no councillor ever lunched at the diner");
}

#[test]
fn the_notebook_shows_where_someone_was_seen_and_usually_is() {
    let c = content();
    let mut e = game(13);
    // The diner owner: at their diner most of the day.
    let (who, loc) = {
        let st = e.state();
        let all = routines::placements(st, &c, &Now::at(st, &c, DayPart::Morning), town(&e));
        all.into_iter()
            .find(|(id, l)| {
                st.politics
                    .characters
                    .get(*id)
                    .and_then(|ch| ch.named.as_deref())
                    .is_some_and(|n| {
                        n.place.is_some()
                            && Location::Place {
                                place: n.place.unwrap(),
                            } == *l
                    })
            })
            .unwrap()
    };
    e.setup(|st| {
        let on = st.calendar.date;
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.lab_omniscient = false;
        k.learn(
            Subject::Person { person: who },
            Level::Heard,
            on,
            Source::Lab,
        );
    });
    go(&mut e, loc);
    let sheet = character_view(&e, who.0).unwrap();
    let seen = sheet.last_seen.expect("last seen");
    assert_eq!(seen.key, "seen-last");
    assert_eq!(seen.at.location.to_sim(), loc);
    assert_eq!(seen.part, "morning");
    assert!(sheet.usually.is_none());
    // Seen there again on another turn, in the morning.
    let mut twice = false;
    for _ in 0..30 {
        e.apply(Command::EndTurn).unwrap();
        if routines::whereabouts(e.state(), &c, who) == Some(loc) {
            go(&mut e, Location::Home);
            go(&mut e, loc);
            twice = true;
            break;
        }
    }
    assert!(twice);
    let sheet = character_view(&e, who.0).unwrap();
    assert_eq!(sheet.usually.unwrap().key, "seen-usually");
    let nb = notebook_view(&e).unwrap();
    let entry = nb
        .sections
        .iter()
        .flat_map(|s| s.entries.iter())
        .find(|x| matches!(x.subject, SubjectDto::Person { person } if person == who.0))
        .expect("in the notebook");
    assert!(entry.seen.is_some());
    let lead = entry
        .lead
        .as_ref()
        .expect("a lead to where they usually are");
    assert_eq!(lead.key, "lead-usually-at");
    assert_eq!(
        lead.args.get("part_key").map(String::as_str),
        Some("day-part-morning")
    );
    assert_eq!(lead.to.as_ref().unwrap().location.to_sim(), loc);
    let _ = characters::player_character(e.state());
}
