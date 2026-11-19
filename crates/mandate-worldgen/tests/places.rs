#![allow(clippy::unwrap_used, clippy::expect_used)]
//! M5a integration tests: places as the interface (DEC-079). Located actions,
//! free movement, the auto-routing `Go` command, fallbacks, and determinism.

use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::places::{self, Location, Offer};
use mandate_sim::world::{PlaceId, PlaceKind};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

fn content() -> Arc<ContentBundle> {
    static C: OnceLock<Arc<ContentBundle>> = OnceLock::new();
    C.get_or_init(|| {
        let src =
            read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
        Arc::new(load(&src).unwrap().0)
    })
    .clone()
}

fn world(seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), content()).unwrap()
}

fn place_of(e: &Engine, kind: PlaceKind) -> Option<Location> {
    let st = e.state();
    let m = st.player.as_ref().unwrap().municipality;
    st.world.municipalities[m.index()]
        .places
        .iter()
        .find(|p| st.world.places[p.index()].kind == kind)
        .map(|p| Location::Place { place: *p })
}

fn location(e: &Engine) -> Location {
    e.state().player.as_ref().unwrap().location
}

fn draft_here(e: &mut Engine) -> u32 {
    let hall = place_of(e, PlaceKind::TownHall).unwrap();
    e.apply(Command::Player(PlayerCommand::MoveTo { location: hall }))
        .unwrap();
    e.apply(Command::Player(PlayerCommand::DraftPolicy {
        template: "policy.parks".into(),
        params: BTreeMap::new(),
    }))
    .unwrap()
    .result
    .unwrap()
    .created
    .unwrap()
}

#[test]
fn every_place_kind_has_behaviour() {
    assert!(places::registry_issues(&content()).is_empty());
}

#[test]
fn players_start_at_home_and_move_for_free() {
    let mut e = world(51);
    assert_eq!(location(&e), Location::Home);
    let slots = e.state().player.as_ref().unwrap().slots_left();
    let chamber = place_of(&e, PlaceKind::CouncilChamber).unwrap();
    e.apply(Command::Player(PlayerCommand::MoveTo { location: chamber }))
        .unwrap();
    assert_eq!(location(&e), chamber);
    assert_eq!(e.state().player.as_ref().unwrap().slots_left(), slots);
    // Another town's places are out of reach (travel is a later milestone).
    let me = e.state().player.as_ref().unwrap().municipality;
    let elsewhere = e
        .state()
        .world
        .places
        .iter()
        .find(|p| p.municipality != me)
        .map(|p| p.id)
        .unwrap();
    assert!(
        e.apply(Command::Player(PlayerCommand::MoveTo {
            location: Location::Place { place: elsewhere },
        }))
        .is_err()
    );
    assert_eq!(location(&e), chamber, "a refused move changes nothing");
}

#[test]
fn actions_are_done_where_they_are_offered() {
    let mut e = world(52);
    // Drafting happens at the town hall, not at home.
    let err = e
        .apply(Command::Player(PlayerCommand::DraftPolicy {
            template: "policy.parks".into(),
            params: BTreeMap::new(),
        }))
        .unwrap_err();
    assert!(
        err.to_string().contains("go to the town_hall first"),
        "{err}"
    );
    let id = draft_here(&mut e);
    // Introducing needs the council chamber.
    let introduce = PlayerCommand::IntroducePolicy { policy: id };
    let err = e.apply(Command::Player(introduce.clone())).unwrap_err();
    assert!(err.to_string().contains("council_chamber"), "{err}");
    let chamber = place_of(&e, PlaceKind::CouncilChamber).unwrap();
    e.apply(Command::Player(PlayerCommand::MoveTo { location: chamber }))
        .unwrap();
    e.apply(Command::Player(introduce)).unwrap();
    // Resting is done at home.
    assert!(
        e.apply(Command::Player(PlayerCommand::Act {
            action: Action::Rest,
            late_night: false,
        }))
        .is_err()
    );
    // Late-night lobbying happens at the diner, not in the chamber.
    let m = e.state().player.as_ref().unwrap().municipality;
    let target = e.state().politics.councils[m.index()].mayor.unwrap().0;
    let late = PlayerCommand::Act {
        action: Action::Lobby {
            politician: target,
            policy: id,
        },
        late_night: true,
    };
    assert!(e.apply(Command::Player(late.clone())).is_err());
    assert_eq!(
        places::best_location(e.state(), e.content(), &late),
        place_of(&e, PlaceKind::Diner)
    );
    e.apply(e.route(late)).unwrap();
    assert_eq!(location(&e), place_of(&e, PlaceKind::Diner).unwrap());
}

#[test]
fn cannot_vote_from_home_but_can_in_the_chamber() {
    let mut e = world(53);
    // A colleague's bill before the council.
    let me = e.state().player.as_ref().unwrap().politician;
    let m = e.state().player.as_ref().unwrap().municipality;
    let colleague = *e.state().politics.councils[m.index()]
        .members
        .iter()
        .find(|p| **p != me)
        .unwrap();
    let id = draft_here(&mut e);
    e.setup(|st| {
        let p = &mut st.policy.instances[id as usize];
        p.sponsor = Some(colleague);
        p.status = mandate_sim::policy::PolicyStatus::UnderConsideration;
    });
    e.apply(Command::Player(PlayerCommand::MoveTo {
        location: Location::Home,
    }))
    .unwrap();
    let vote = PlayerCommand::CastVote {
        policy: id,
        vote: mandate_sim::politics::Vote::Yes,
    };
    assert!(e.apply(Command::Player(vote.clone())).is_err());
    let chamber = place_of(&e, PlaceKind::CouncilChamber).unwrap();
    e.apply(Command::Player(PlayerCommand::MoveTo { location: chamber }))
        .unwrap();
    let r = e.apply(Command::Player(vote)).unwrap();
    assert_eq!(r.result.unwrap().outcome_key, "outcome-vote-yes");
}

#[test]
fn go_moves_and_acts_in_one_command_and_keeps_the_move_on_failure() {
    let mut e = world(54);
    let hood = {
        let st = e.state();
        mandate_sim::policy::ward_hoods(&st.world, st.player.as_ref().unwrap().ward)[0] as u32
    };
    let canvass = PlayerCommand::Act {
        action: Action::Canvass {
            neighborhood: hood,
            issue: None,
        },
        late_night: false,
    };
    let street = Location::Street {
        neighborhood: mandate_sim::world::NeighborhoodId(hood),
    };
    assert_eq!(
        places::best_location(e.state(), e.content(), &canvass),
        Some(street)
    );
    let logged = e.command_log().len();
    let r = e
        .apply(Command::Player(PlayerCommand::Go {
            location: street,
            then: Some(Box::new(canvass)),
        }))
        .unwrap();
    assert_eq!(r.result.unwrap().outcome_key, "outcome-canvass");
    assert_eq!(location(&e), street);
    assert_eq!(
        e.command_log().len(),
        logged + 1,
        "one command, one log entry"
    );
    // A failing command after the move: the move stands, and is logged so a
    // replay reproduces it.
    let chamber = place_of(&e, PlaceKind::CouncilChamber).unwrap();
    let r = e.apply(Command::Player(PlayerCommand::Go {
        location: chamber,
        then: Some(Box::new(PlayerCommand::IntroducePolicy { policy: 9_999 })),
    }));
    assert!(r.is_err());
    assert_eq!(location(&e), chamber);
    assert_eq!(e.command_log().len(), logged + 2);
    // A refused move logs nothing and changes nothing.
    let r = e.apply(Command::Player(PlayerCommand::Go {
        location: Location::Place {
            place: PlaceId(u32::MAX),
        },
        then: None,
    }));
    assert!(r.is_err());
    assert_eq!(location(&e), chamber);
    assert_eq!(e.command_log().len(), logged + 2);
}

#[test]
fn every_town_offers_every_action_somewhere() {
    // Fallbacks (DEC-079): a neighbourhood without a community centre meets on
    // its streets; a town without a newspaper uses the town hall.
    for seed in [61, 62, 63, 64] {
        let e = world(seed);
        let st = e.state();
        let c = e.content();
        let m = st.player.as_ref().unwrap().municipality;
        for key in mandate_content::PLACE_ACTIONS {
            if places::NEIGHBORHOOD_ACTIONS.contains(&key) {
                for h in st.world.hoods_of(m) {
                    assert!(
                        !places::locations_for(
                            st,
                            c,
                            Offer::Action {
                                key,
                                late_night: false
                            },
                            Some(h as u32)
                        )
                        .is_empty(),
                        "seed {seed}: {key} in hood {h}"
                    );
                }
            } else {
                assert!(
                    !places::locations_for(
                        st,
                        c,
                        Offer::Action {
                            key,
                            late_night: false
                        },
                        None
                    )
                    .is_empty(),
                    "seed {seed}: {key}"
                );
            }
        }
        for s in mandate_content::PLACE_SERVICES {
            assert!(
                !places::locations_for(st, c, Offer::Service(s), None).is_empty(),
                "seed {seed}: {s}"
            );
        }
    }
}

#[test]
fn routed_play_is_deterministic() {
    let run = || {
        let mut e = world(55);
        let hood = {
            let st = e.state();
            mandate_sim::policy::ward_hoods(&st.world, st.player.as_ref().unwrap().ward)[0] as u32
        };
        for _ in 0..6 {
            e.apply(e.route(PlayerCommand::Act {
                action: Action::Canvass {
                    neighborhood: hood,
                    issue: None,
                },
                late_night: false,
            }))
            .unwrap();
            let _ = e.apply(e.route(PlayerCommand::Act {
                action: Action::Fundraise,
                late_night: true,
            }));
            e.apply(Command::EndTurn).unwrap();
        }
        (e.state_hash(), location(&e))
    };
    assert_eq!(run(), run());
}
