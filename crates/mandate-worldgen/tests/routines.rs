#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Daily routines (DEC-031, task S1): placement is derived from the state
//! and the turn's day, deterministic and keyed, varies by part of the day
//! and by turn, follows posts by their constitution role (no office named
//! here), and never puts anyone where their town has nothing. The player's
//! ledger records where they saw people, and a habit seen twice.

use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::actions::PlayerCommand;
use mandate_sim::characters::routines::{self, DayPart, Now};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::places::{self, Location};
use mandate_sim::politics;
use mandate_sim::world::{Jurisdiction, PlaceKind};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn base() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn engine(c: &Arc<ContentBundle>, seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

fn town(e: &Engine) -> mandate_sim::world::MunicipalityId {
    e.state().player.as_ref().unwrap().home()
}

fn all_at(e: &Engine, part: DayPart) -> Vec<(CharacterId, Location)> {
    let st = e.state();
    let now = Now::at(st, e.content(), part);
    routines::placements(st, e.content(), &now, town(e))
}

/// Set how much of the turn the player has used (test setup).
fn use_slots(e: &mut Engine, n: u8) {
    e.setup(|st| st.player.as_mut().unwrap().slots_used = n);
}

#[test]
fn placement_is_deterministic_and_varies_by_part_and_day() {
    let c = base();
    let mut e = engine(&c, 21);
    let first: Vec<_> = DayPart::ALL.iter().map(|p| all_at(&e, *p)).collect();
    // The same answer however often it is asked, and after a save round trip.
    let again: Vec<_> = DayPart::ALL.iter().map(|p| all_at(&e, *p)).collect();
    assert_eq!(first, again);
    let reloaded = Engine::from_state(e.state().clone(), e.content_arc()).unwrap();
    let third: Vec<_> = DayPart::ALL.iter().map(|p| all_at(&reloaded, *p)).collect();
    assert_eq!(first, third);
    // Everyone stored in town is somewhere.
    let folk = routines::townsfolk(e.state(), town(&e));
    assert!(folk.len() > 10, "a town has people: {}", folk.len());
    assert_eq!(first[0].len(), folk.len());
    // People move through the day.
    let moved = |a: &[(CharacterId, Location)], b: &[(CharacterId, Location)]| {
        a.iter().zip(b).filter(|(x, y)| x.1 != y.1).count()
    };
    for w in first.windows(2) {
        assert!(moved(&w[0], &w[1]) > 0, "someone moves between parts");
    }
    // And from day to day: over a few turns, the same part differs.
    let mut changed = 0;
    for _ in 0..4 {
        e.apply(Command::EndTurn).unwrap();
        let next = all_at(&e, DayPart::Morning);
        changed += moved(&first[0], &next);
    }
    assert!(changed > 0, "routines vary by day");
    // The part of the day follows the player's standard slots, then late night.
    let cap = e.state().player.as_ref().unwrap().slot_cap();
    use_slots(&mut e, 0);
    assert_eq!(routines::day_part(e.state()), DayPart::Morning);
    use_slots(&mut e, cap);
    assert_eq!(routines::day_part(e.state()), DayPart::LateNight);
    if cap == 3 {
        use_slots(&mut e, 1);
        assert_eq!(routines::day_part(e.state()), DayPart::Afternoon);
        use_slots(&mut e, 2);
        assert_eq!(routines::day_part(e.state()), DayPart::Evening);
        // A slot half spent on a conversation is still under way.
        e.setup(|st| {
            let turn = st.calendar.turn;
            let pl = st.player.as_mut().unwrap();
            pl.slots_used = 1;
            pl.talk.half_slot = Some(turn);
        });
        assert_eq!(routines::day_part(e.state()), DayPart::Morning);
    }
}

#[test]
fn the_weekday_moves_on_each_turn() {
    let c = base();
    let mut e = engine(&c, 22);
    let mut days = Vec::new();
    for _ in 0..7 {
        days.push(Now::of(e.state(), &c).weekday);
        e.apply(Command::EndTurn).unwrap();
    }
    days.sort();
    days.dedup();
    assert_eq!(days.len(), 7, "every weekday comes round: {days:?}");
}

#[test]
fn a_session_day_puts_members_in_their_chamber() {
    let c = base();
    for seed in [3, 31, 77] {
        let e = engine(&c, seed);
        let st = e.state();
        let at = Jurisdiction::town(town(&e));
        for part in [DayPart::Morning, DayPart::Afternoon] {
            let mut now = Now::at(st, &c, part);
            now.sessions.push(at);
            for pid in politics::body_members_at(&st.politics, at) {
                let p = st.politics.get(pid).unwrap();
                if p.is_player {
                    continue;
                }
                let kind = politics::workplace_of(&st.politics, p.role.post().unwrap()).unwrap();
                let l = routines::locate(st, &c, &now, p.character).unwrap();
                let Location::Place { place } = l else {
                    panic!("seed {seed}: a member on the streets on a session day");
                };
                assert_eq!(st.world.places[place.index()].kind, kind, "seed {seed}");
            }
        }
    }
}

#[test]
fn the_broker_is_at_the_late_night_spot_late_at_night() {
    let c = base();
    let mut checked = 0;
    for seed in [3, 5, 31, 77, 4242] {
        let e = engine(&c, seed);
        let st = e.state();
        let m = town(&e);
        let Some(b) = st.politics.characters.towns[m.index()].broker else {
            continue;
        };
        if characters::player_character(st) == Some(b) {
            continue;
        }
        let diner = st
            .world
            .places
            .iter()
            .any(|p| p.municipality == m && p.kind == PlaceKind::Diner && p.is_open());
        let now = Now::at(st, &c, DayPart::LateNight);
        let l = routines::locate(st, &c, &now, b).unwrap();
        if diner {
            let Location::Place { place } = l else {
                panic!("seed {seed}: the broker is not in the diner late at night");
            };
            assert_eq!(st.world.places[place.index()].kind, PlaceKind::Diner);
            checked += 1;
        }
    }
    assert!(checked > 0);
}

#[test]
fn nobody_is_placed_where_their_town_has_nothing() {
    let c = base();
    for seed in [1, 8, 4242] {
        let mut e = engine(&c, seed);
        for _ in 0..6 {
            for part in DayPart::ALL {
                for (id, l) in all_at(&e, part) {
                    let st = e.state();
                    places::validate_move(st, l)
                        .unwrap_or_else(|err| panic!("seed {seed} {id:?} at {l:?}: {err:?}"));
                    assert_ne!(l, Location::Home);
                    if let Location::Place { place } = l {
                        assert!(st.world.places[place.index()].is_open());
                    }
                    assert_eq!(routines::town_of(st, l), Some(town(&e)));
                }
            }
            e.apply(Command::EndTurn).unwrap();
        }
    }
}

#[test]
fn present_is_who_the_routines_put_here() {
    let c = base();
    let e = engine(&c, 9);
    let st = e.state();
    let now = Now::of(st, &c);
    let all = routines::placements(st, &c, &now, town(&e));
    let mut locs: Vec<Location> = all.iter().map(|x| x.1).collect();
    locs.sort();
    locs.dedup();
    let mut n = 0;
    for l in locs {
        let here = characters::ambient::stored_at(st, &c, l);
        let expect: Vec<CharacterId> = all.iter().filter(|x| x.1 == l).map(|x| x.0).collect();
        let mut a = here.clone();
        a.sort();
        assert_eq!(a, expect);
        n += here.len();
    }
    assert_eq!(n, all.len());
}

#[test]
fn last_seen_is_recorded_and_a_habit_seen_twice_is_noted() {
    let c = base();
    let mut e = engine(&c, 13);
    // Somewhere with someone in it now.
    let (loc, who) = {
        let st = e.state();
        let all = routines::placements(st, &c, &Now::of(st, &c), town(&e));
        all.iter()
            .find(|(_, l)| matches!(l, Location::Place { .. }))
            .map(|(id, l)| (*l, *id))
            .unwrap()
    };
    e.apply(Command::Player(PlayerCommand::MoveTo { location: loc }))
        .unwrap();
    let k = &e.state().player.as_ref().unwrap().knowledge;
    let s = k.last_seen(who).expect("seen on arrival");
    assert_eq!(s.at, loc);
    assert_eq!(s.part, DayPart::Morning);
    assert_eq!(s.on, e.state().calendar.date);
    assert!(k.usually(who).is_none(), "once is not a habit");
    // Seen there again in the same part of another turn: a habit.
    let mut found = false;
    for _ in 0..30 {
        e.apply(Command::EndTurn).unwrap();
        let here = routines::whereabouts(e.state(), &c, who);
        if here == Some(loc) {
            e.apply(Command::Player(PlayerCommand::MoveTo {
                location: Location::Home,
            }))
            .unwrap();
            e.apply(Command::Player(PlayerCommand::MoveTo { location: loc }))
                .unwrap();
            found = true;
            break;
        }
    }
    assert!(found, "they come back to their place");
    let k = &e.state().player.as_ref().unwrap().knowledge;
    let u = k.usually(who).expect("a habit");
    assert_eq!((u.at, u.part), (loc, DayPart::Morning));
    assert!(k.seen[&who].len() >= 2);
    // A save keeps it.
    let back: mandate_sim::State =
        serde_json::from_value(serde_json::to_value(e.state()).unwrap()).unwrap();
    assert_eq!(
        back.player.as_ref().unwrap().knowledge.seen,
        e.state().player.as_ref().unwrap().knowledge.seen
    );
}
