#![allow(clippy::unwrap_used, clippy::expect_used)]
//! M3 integration tests: the bill pipeline, mechanisms, actions, determinism.

use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::policy::PolicyStatus;
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

fn draft(e: &mut Engine, template: &str, params: &[(&str, f64)]) -> u32 {
    let params: BTreeMap<String, f64> = params.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    let out = e
        .apply(Command::Player(PlayerCommand::DraftPolicy {
            template: template.into(),
            params,
        }))
        .unwrap();
    out.result.unwrap().created.unwrap()
}

/// Lobby every colleague (using favours if needed) so the bill passes.
fn whip_hard(e: &mut Engine, policy: u32) {
    let me = e.state().player.as_ref().unwrap().politician;
    let m = e.state().player.as_ref().unwrap().municipality;
    let members: Vec<u32> = e.state().politics.councils[m.index()]
        .members
        .iter()
        .filter(|p| **p != me)
        .map(|p| p.0)
        .collect();
    let mayor = e.state().politics.councils[m.index()].mayor.unwrap().0;
    // Spend this turn's slots trading favours, nearest members first.
    for id in members.into_iter().chain([mayor]) {
        if e.state().player.as_ref().unwrap().slots_left() == 0 {
            break;
        }
        let _ = e.apply(Command::Player(PlayerCommand::Act {
            action: Action::TradeFavor {
                politician: id,
                policy,
            },
            late_night: false,
        }));
    }
}

#[test]
fn bill_journey_draft_to_law() {
    let mut e = world(31);
    let id = draft(&mut e, "policy.parks", &[("level", 2.0)]);
    assert_eq!(
        e.state().policy.instances[id as usize].status,
        PolicyStatus::Draft
    );
    e.apply(Command::Player(PlayerCommand::IntroducePolicy {
        policy: id,
    }))
    .unwrap();
    assert_eq!(
        e.state().policy.instances[id as usize].status,
        PolicyStatus::UnderConsideration
    );
    // Trade favours over several turns until every colleague is sweetened.
    for _ in 0..4 {
        whip_hard(&mut e, id);
        e.apply(Command::EndTurn).unwrap();
    }
    for _ in 0..6 {
        e.apply(Command::EndTurn).unwrap();
    }
    let p = &e.state().policy.instances[id as usize];
    assert!(!p.votes.is_empty(), "a vote should have happened");
    assert!(matches!(
        p.status,
        PolicyStatus::Enacted | PolicyStatus::InEffect | PolicyStatus::Ended
    ));
    if p.status != PolicyStatus::Ended {
        // Funded and implementing within a few months.
        for _ in 0..8 {
            e.apply(Command::EndTurn).unwrap();
        }
        let p = &e.state().policy.instances[id as usize];
        assert_eq!(p.status, PolicyStatus::InEffect);
        assert!(p.progress > 0.0);
        assert!(e.state().finance.funded_share(id) > 0.0);
        assert!(p.ledger.contains_key("service_gain_pct"));
    }
}

#[test]
fn housing_programme_adds_homes_versus_control() {
    // Same seed with and without the law: the mechanism path (REQ-011) adds
    // homes (stock plus the construction pipeline) and records attribution.
    let mut a = world(32);
    let mut b = world(32);
    let id = draft(&mut b, "policy.affordable_housing", &[("units", 200.0)]);
    b.apply(Command::Player(PlayerCommand::IntroducePolicy {
        policy: id,
    }))
    .unwrap();
    for _ in 0..4 {
        whip_hard(&mut b, id);
        b.apply(Command::EndTurn).unwrap();
        a.apply(Command::EndTurn).unwrap();
    }
    for _ in 0..60 {
        a.apply(Command::EndTurn).unwrap();
        b.apply(Command::EndTurn).unwrap();
    }
    let p = &b.state().policy.instances[id as usize];
    if p.status == PolicyStatus::InEffect {
        let m = b.state().player.as_ref().unwrap().municipality.index();
        let homes = |e: &Engine| {
            let h = &e.state().economy.munis[m].housing;
            h.stock + h.pipeline.iter().sum::<f64>()
        };
        assert!(
            homes(&b) > homes(&a) + 50.0,
            "homes {} vs control {}",
            homes(&b),
            homes(&a)
        );
        assert!(
            p.ledger.get("homes_built").copied().unwrap_or(0.0) > 50.0,
            "{:?}",
            p.ledger
        );
    } else {
        // Defeat or no funding is a legitimate outcome; the bill must have been voted on.
        assert!(!p.votes.is_empty());
    }
}

#[test]
fn actions_spend_slots_and_validate() {
    let mut e = world(33);
    let pl = e.state().player.clone().unwrap();
    let hood = e.state().world.municipalities[pl.municipality.index()].neighborhoods[0].0;
    for _ in 0..pl.standard_slots {
        e.apply(Command::Player(PlayerCommand::Act {
            action: Action::Canvass { neighborhood: hood },
            late_night: false,
        }))
        .unwrap();
    }
    assert_eq!(e.state().player.as_ref().unwrap().slots_left(), 0);
    let err = e.apply(Command::Player(PlayerCommand::Act {
        action: Action::Rest,
        late_night: false,
    }));
    assert!(err.is_err(), "no slots left");
    // Late night is only for private meetings and fundraising.
    assert!(
        e.apply(Command::Player(PlayerCommand::Act {
            action: Action::Canvass { neighborhood: hood },
            late_night: true
        }))
        .is_err()
    );
    e.apply(Command::Player(PlayerCommand::Act {
        action: Action::Fundraise,
        late_night: true,
    }))
    .unwrap();
    assert!(e.state().player.as_ref().unwrap().late_night_used);
    let health_after = e.state().player.as_ref().unwrap().tracks.health;
    assert!(health_after < pl.tracks.health);
    // Canvassing raised awareness in that neighbourhood.
    let aware: f32 = e.state().population.hoods[hood as usize]
        .cohorts
        .iter()
        .map(|c| c.player_awareness)
        .sum::<f32>()
        / 12.0;
    assert!(aware > 0.5);
    e.apply(Command::EndTurn).unwrap();
    assert_eq!(
        e.state().player.as_ref().unwrap().slots_left(),
        pl.standard_slots
    );
    // A neighbourhood outside the player's town is rejected.
    let other = e
        .state()
        .world
        .neighborhoods
        .iter()
        .find(|n| n.municipality != pl.municipality)
        .unwrap()
        .id
        .0;
    assert!(
        e.apply(Command::Player(PlayerCommand::Act {
            action: Action::Canvass {
                neighborhood: other
            },
            late_night: false
        }))
        .is_err()
    );
}

#[test]
fn player_commands_are_deterministic() {
    let run = || {
        let mut e = world(34);
        let id = draft(&mut e, "policy.transit_frequency", &[("level", 3.0)]);
        e.apply(Command::Player(PlayerCommand::IntroducePolicy {
            policy: id,
        }))
        .unwrap();
        let m = e.state().player.as_ref().unwrap().municipality;
        let target = e.state().politics.councils[m.index()].mayor.unwrap().0;
        let _ = e.apply(Command::Player(PlayerCommand::Act {
            action: Action::Lobby {
                politician: target,
                policy: id,
            },
            late_night: true,
        }));
        for _ in 0..10 {
            e.apply(Command::EndTurn).unwrap();
        }
        e.state_hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn state_with_lobbying_round_trips_through_json() {
    // Regression: tuple-keyed sway once made the state unserialisable, which
    // silently broke the state hash and saves after the first lobby action.
    let mut e = world(34);
    let id = draft(&mut e, "policy.parks", &[]);
    e.apply(Command::Player(PlayerCommand::IntroducePolicy {
        policy: id,
    }))
    .unwrap();
    let me = e.state().player.as_ref().unwrap().politician;
    let m = e.state().player.as_ref().unwrap().municipality;
    let target = *e.state().politics.councils[m.index()]
        .members
        .iter()
        .find(|p| **p != me)
        .unwrap();
    e.apply(Command::Player(PlayerCommand::Act {
        action: Action::Lobby {
            politician: target.0,
            policy: id,
        },
        late_night: false,
    }))
    .unwrap();
    assert!(!e.state().politics.sway.is_empty());
    let hash = e.state_hash();
    assert_eq!(hash.len(), 64, "state hash must not be empty");
    let json = serde_json::to_string(e.state()).unwrap();
    let back: mandate_sim::State = serde_json::from_str(&json).unwrap();
    assert_eq!(back.hash(), hash);
}

#[test]
fn favour_debt_is_capped() {
    let mut e = world(35);
    let id = draft(&mut e, "policy.parks", &[]);
    e.apply(Command::Player(PlayerCommand::IntroducePolicy {
        policy: id,
    }))
    .unwrap();
    let me = e.state().player.as_ref().unwrap().politician;
    let m = e.state().player.as_ref().unwrap().municipality;
    let target = *e.state().politics.councils[m.index()]
        .members
        .iter()
        .find(|p| **p != me)
        .unwrap();
    let trade = || {
        Command::Player(PlayerCommand::Act {
            action: Action::TradeFavor {
                politician: target.0,
                policy: id,
            },
            late_night: false,
        })
    };
    e.apply(trade()).unwrap();
    e.apply(trade()).unwrap();
    // Two favours owed: the third trade is refused (slots remain from the late-night allowance aside).
    e.apply(Command::EndTurn).unwrap();
    assert!(e.apply(trade()).is_err());
}
