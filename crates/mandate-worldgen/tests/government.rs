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
        .apply(e.route(PlayerCommand::DraftPolicy {
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
    let members: Vec<u32> = council_members(e.state(), m.index())
        .iter()
        .filter(|p| **p != me)
        .map(|p| p.0)
        .collect();
    let mayor = mayor_of(e.state(), m.index()).unwrap().0;
    // Spend this turn's slots trading favours, nearest members first.
    for id in members.into_iter().chain([mayor]) {
        if e.state().player.as_ref().unwrap().slots_left() == 0 {
            break;
        }
        let _ = e.apply(e.route(PlayerCommand::Act {
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
    e.apply(e.route(PlayerCommand::IntroducePolicy { policy: id }))
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
        // Funded from the next budget year at the latest (funding is a
        // status, DEC-024: a law can pass and wait for money).
        for _ in 0..30 {
            e.apply(Command::EndTurn).unwrap();
            if e.state().policy.instances[id as usize].status == PolicyStatus::InEffect {
                break;
            }
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
    b.apply(b.route(PlayerCommand::IntroducePolicy { policy: id }))
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
    // Canvassing takes two slots (a long day), press one: three in all.
    e.apply(e.route(PlayerCommand::Act {
        action: Action::Canvass {
            neighborhood: hood,
            issue: None,
        },
        late_night: false,
    }))
    .unwrap();
    assert_eq!(
        e.state().player.as_ref().unwrap().slots_left(),
        pl.standard_slots - 2
    );
    assert!(
        e.apply(e.route(PlayerCommand::Act {
            action: Action::Canvass {
                neighborhood: hood,
                issue: None
            },
            late_night: false,
        }))
        .is_err(),
        "a second canvass needs two free slots"
    );
    e.apply(e.route(PlayerCommand::Act {
        action: Action::Press { policy: None },
        late_night: false,
    }))
    .unwrap();
    assert_eq!(e.state().player.as_ref().unwrap().slots_left(), 0);
    let err = e.apply(e.route(PlayerCommand::Act {
        action: Action::Rest,
        late_night: false,
    }));
    assert!(err.is_err(), "no slots left");
    // Late night is only for private meetings and fundraising.
    assert!(
        e.apply(e.route(PlayerCommand::Act {
            action: Action::Canvass {
                neighborhood: hood,
                issue: None
            },
            late_night: true
        }))
        .is_err()
    );
    e.apply(e.route(PlayerCommand::Act {
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
        e.apply(e.route(PlayerCommand::Act {
            action: Action::Canvass {
                neighborhood: other,
                issue: None
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
        e.apply(e.route(PlayerCommand::IntroducePolicy { policy: id }))
            .unwrap();
        let m = e.state().player.as_ref().unwrap().municipality;
        let target = mayor_of(e.state(), m.index()).unwrap().0;
        let _ = e.apply(e.route(PlayerCommand::Act {
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
    e.apply(e.route(PlayerCommand::IntroducePolicy { policy: id }))
        .unwrap();
    let me = e.state().player.as_ref().unwrap().politician;
    let m = e.state().player.as_ref().unwrap().municipality;
    let target = *council_members(e.state(), m.index())
        .iter()
        .find(|p| **p != me)
        .unwrap();
    e.apply(e.route(PlayerCommand::Act {
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
    e.apply(e.route(PlayerCommand::IntroducePolicy { policy: id }))
        .unwrap();
    let me = e.state().player.as_ref().unwrap().politician;
    let m = e.state().player.as_ref().unwrap().municipality;
    let target = *council_members(e.state(), m.index())
        .iter()
        .find(|p| **p != me)
        .unwrap();
    let trade = || PlayerCommand::Act {
        action: Action::TradeFavor {
            politician: target.0,
            policy: id,
        },
        late_night: false,
    };
    e.apply(e.route(trade())).unwrap();
    e.apply(e.route(trade())).unwrap();
    // Two favours owed: the third trade is refused (slots remain from the late-night allowance aside).
    e.apply(Command::EndTurn).unwrap();
    assert!(e.apply(e.route(trade())).is_err());
}

#[test]
fn saves_survive_a_balance_patch_but_not_missing_rules() {
    use mandate_sim::events::EventKind;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let mut e = world(36);
    let id = draft(&mut e, "policy.parks", &[]);
    e.apply(e.route(PlayerCommand::IntroducePolicy { policy: id }))
        .unwrap();
    let state = e.state().clone();

    // A balance patch: one tuning number changes, every id still exists.
    let mut sources = read_dir_sources(&root).unwrap();
    let pol = sources
        .iter_mut()
        .find(|s| s.path.ends_with("politics.jsonc"))
        .unwrap();
    pol.text = pol.text.replace("\"ad_cost\": 1500", "\"ad_cost\": 1600");
    let patched = Arc::new(load(&sources).unwrap().0);
    assert_ne!(patched.content_hash, content().content_hash);
    let mut e2 = Engine::from_state(state.clone(), patched.clone()).expect("a balance patch loads");
    assert!(
        e2.state()
            .events
            .iter()
            .any(|ev| matches!(ev.kind, EventKind::RulesUpdated { .. }))
    );
    assert_eq!(e2.state().meta.content_hash, patched.content_hash);
    e2.apply(Command::EndTurn).unwrap();

    // A rule the save depends on is gone: refuse with a clear message.
    let mut missing = (*content()).clone();
    missing.policies.remove("policy.parks");
    missing.content_hash = "different".into();
    let err = Engine::from_state(state, Arc::new(missing))
        .err()
        .expect("missing rules refuse");
    assert!(err.to_string().contains("policy.parks"));
}

#[test]
fn the_players_vote_has_consequences() {
    use mandate_sim::policy::{PolicyInstance, PolicyInstanceId};
    use mandate_sim::politics::Vote;
    let mut e = world(37);
    let (me, m) = {
        let p = e.state().player.as_ref().unwrap();
        (p.politician, p.municipality.0)
    };
    let my_party = e.state().politics.get(me).unwrap().party;
    // A colleague from the player's own party sponsors a bill; the player owes them.
    let members = council_members(e.state(), m as usize);
    let sponsor = *members
        .iter()
        .find(|id| **id != me && e.state().politics.get(**id).unwrap().party == my_party)
        .or_else(|| members.iter().find(|id| **id != me))
        .unwrap();
    let same_party = e.state().politics.get(sponsor).unwrap().party == my_party;
    let bill = e.state().policy.instances.len() as u32;
    let (sc, mc) = (
        e.state().politics.get(sponsor).unwrap().character,
        e.state().politics.get(me).unwrap().character,
    );
    e.setup(|st| {
        let date = st.calendar.date;
        mandate_sim::characters::owe(&mut st.politics, mc, sc, "favour-bill", None, date);
        st.policy.ids.alloc_raw();
        st.policy.instances.push(PolicyInstance {
            family: None,
            id: PolicyInstanceId(bill),
            template: "policy.parks".into(),
            tier: mandate_content::TierKey::Local,
            jurisdiction: m,
            sponsor: Some(sponsor),
            params: Default::default(),
            clauses: Vec::new(),
            status: PolicyStatus::UnderConsideration,
            stage: 0,
            next_stage_on: Some(st.calendar.date.add_days(21)),
            votes: Vec::new(),
            introduced_on: Some(st.calendar.date),
            enacted_on: None,
            progress: 0.0,
            awareness: 0.3,
            end_reason: None,
            cause: None,
            ledger: Default::default(),
        });
    });
    let standing = e.state().player.as_ref().unwrap().tracks.standing;
    let r = e
        .apply(e.route(PlayerCommand::CastVote {
            policy: bill,
            vote: Vote::Yes,
        }))
        .unwrap();
    assert_eq!(r.result.unwrap().outcome_key, "outcome-vote-yes");
    assert_eq!(
        mandate_sim::characters::favour_balance(&e.state().politics, sc, mc),
        0,
        "the owed favour is repaid"
    );
    let kept = mandate_sim::characters::salient_memories(
        &e.state().politics,
        sc,
        Some(mc),
        e.state().calendar.date,
        5,
    );
    assert!(
        kept.iter()
            .any(|m| m.kind == mandate_sim::characters::MemoryKind::KeptPromise),
        "the sponsor remembers the promise kept"
    );
    if same_party {
        assert!(
            e.state().player.as_ref().unwrap().tracks.standing > standing,
            "loyalty is noticed"
        );
    }
    // Final: no second declaration, and the declared vote drives the count.
    assert!(
        e.apply(e.route(PlayerCommand::CastVote {
            policy: bill,
            vote: Vote::No
        }))
        .is_err()
    );
    assert_eq!(
        e.state().politics.declared_votes.get(&(me.0, bill)),
        Some(&1)
    );
}

/// The members of a town's bodies (the base constitution's council; DEC-098).
fn council_members(st: &mandate_sim::State, m: usize) -> Vec<mandate_sim::politics::PoliticianId> {
    let at = mandate_sim::world::Jurisdiction::town(mandate_sim::world::MunicipalityId(m as u32));
    mandate_sim::politics::body_members_at(&st.politics, at)
}

/// The holder of a town's office (the base constitution's mayor; DEC-098).
fn mayor_of(st: &mandate_sim::State, m: usize) -> Option<mandate_sim::politics::PoliticianId> {
    let at = mandate_sim::world::Jurisdiction::town(mandate_sim::world::MunicipalityId(m as u32));
    mandate_sim::politics::office_holders_at(&st.politics, at)
        .first()
        .copied()
}
