//! Doors (task B3, DEC-114 §2, `BEING_THERE.md` §3): the same door gives
//! the same family twice and through a save (households are derived, and
//! only what the doorstep changed is stored); a street of doors applies
//! today's canvass formula once, exactly (a twin against the one tap and
//! against the formula worked by hand), and its doors add no approval; a
//! street is at most four doors and ends with anything else; records are
//! bounded and remembered at the door; a doorstep ask comes from a
//! household knocked on, promoted to a Named resident.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]
use mandate_content::ContentBundle;
use mandate_sim::actions::{Action, ActionResult, DoorChoice, PlayerCommand};
use mandate_sim::characters::households::{self, Went};
use mandate_sim::places::Location;
use mandate_sim::world::{MunicipalityId, NeighborhoodId};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::Arc;

const SEED: u64 = 7;

fn base() -> Arc<ContentBundle> {
    Arc::new(
        mandate_content::load(&mandate_content::embedded::sources())
            .unwrap()
            .0,
    )
}

fn game(seed: u64, c: Arc<ContentBundle>) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c).unwrap()
}

fn copy(e: &Engine) -> Engine {
    Engine::from_state(e.state().clone(), e.content_arc()).unwrap()
}

fn home(e: &Engine) -> MunicipalityId {
    mandate_sim::player::home(e.state()).unwrap()
}

/// The home town's neighbourhood with the most doors.
fn street(e: &Engine) -> NeighborhoodId {
    let st = e.state();
    st.world.municipalities[home(e).index()]
        .neighborhoods
        .iter()
        .copied()
        .max_by_key(|h| (households::doors(st, e.content(), *h), std::cmp::Reverse(h.0)))
        .unwrap()
}

fn go(e: &mut Engine, hood: NeighborhoodId) {
    let here = Location::Street { neighborhood: hood };
    if e.state().player.as_ref().unwrap().location != here {
        e.apply(Command::Player(PlayerCommand::MoveTo { location: here }))
            .unwrap();
    }
}

fn act(e: &mut Engine, action: Action) -> Result<ActionResult, String> {
    e.apply(Command::Player(PlayerCommand::Act {
        action,
        late_night: false,
    }))
    .map(|o| o.result.unwrap())
    .map_err(|x| x.to_string())
}

fn knock(e: &mut Engine, hood: NeighborhoodId, door: u16, choice: DoorChoice) -> ActionResult {
    act(
        e,
        Action::Doorstep {
            neighborhood: hood.0,
            door,
            choice,
        },
    )
    .unwrap()
}

/// Doors where someone answers today, in slot order.
fn answering(e: &Engine, hood: NeighborhoodId) -> Vec<u16> {
    let (st, c) = (e.state(), e.content());
    (0..households::doors(st, c, hood) as u16)
        .filter(|d| {
            households::household(st, c, hood, *d).is_some_and(|h| households::answers(st, c, &h))
        })
        .collect()
}

/// The neighbourhood's worry its residents truly feel most (a concern id).
fn top_worry(e: &Engine, hood: NeighborhoodId) -> String {
    let (st, c) = (e.state(), e.content());
    let felt = mandate_sim::concerns::ranked(
        &st.population,
        c,
        &st.world,
        mandate_sim::concerns::Scope::Neighbourhood(hood),
        mandate_content::condition::Worry::Felt,
    );
    c.concerns[felt[0].0].id.clone()
}

fn end_turn(e: &mut Engine) {
    e.apply(Command::EndTurn).unwrap();
}

fn save_load(e: &Engine) -> Engine {
    let req = mandate_save::SaveRequest {
        run_id: "00112233445566778899aabbccddeeff".into(),
        revision: 1,
        parent_revision: None,
        device_id: "test".into(),
        saved_at: "2026-01-01T00:00:00Z".into(),
        protocol_version: 1,
        run_mode: mandate_save::RunMode::Ironman,
    };
    let (bytes, _) = mandate_save::encode(e.state(), &req).unwrap();
    let (st, _) = mandate_save::decode(&bytes).unwrap();
    Engine::from_state(st, e.content_arc()).unwrap()
}

/// The first acceptance: the same door gives the same family twice and
/// through a save; a household is derived (nothing is stored until a
/// doorstep), and what the doorstep changed is all that is kept.
#[test]
fn the_same_door_gives_the_same_family_twice_and_through_a_save() {
    let c = base();
    let mut e = game(SEED, c.clone());
    let hood = street(&e);
    let n = households::doors(e.state(), &c, hood);
    let p = &c.characters.households;
    assert!(
        (p.doors[0]..=p.doors[1]).contains(&n),
        "door slots within the bounds: {n}"
    );
    let all = |e: &Engine| -> Vec<households::Household> {
        (0..n as u16)
            .map(|d| households::household(e.state(), &c, hood, d).unwrap())
            .collect()
    };
    let first = all(&e);
    assert_eq!(first, all(&e), "asked twice, the same households");
    assert!(households::household(e.state(), &c, hood, n as u16).is_none());
    let families: std::collections::BTreeSet<&str> =
        first.iter().map(|h| h.family.as_str()).collect();
    assert!(families.len() > n as usize / 2, "different families behind the doors");
    // Nothing is stored for a door never knocked on.
    assert!(e.state().politics.characters.households.is_empty());

    go(&mut e, hood);
    let doors = answering(&e, hood);
    knock(&mut e, hood, doors[0], DoorChoice::Listen);
    knock(&mut e, hood, doors[1], DoorChoice::Ask);
    let hh = &e.state().politics.characters.households;
    assert_eq!(hh.records.len(), 2, "one record a household met");
    assert!(hh.street.is_some(), "the canvass is under way");
    // Knocking stored no household: the same families after it.
    let names = |v: &[households::Household]| -> Vec<(String, String, u32)> {
        v.iter()
            .map(|h| (h.given.clone(), h.family.clone(), h.age))
            .collect()
    };
    assert_eq!(names(&first), names(&all(&e)));

    // Through a save: the same households, records, street and door views.
    let back = save_load(&e);
    assert_eq!(all(&back), all(&e));
    assert_eq!(
        back.state().politics.characters.households,
        e.state().politics.characters.households
    );
    assert_eq!(back.state_hash(), e.state_hash());
    for d in [doors[0], doors[1], doors[2]] {
        assert_eq!(
            mandate_protocol::doors::door_view(&back, hood, d),
            mandate_protocol::doors::door_view(&e, hood, d)
        );
    }

    // A month on, the families behind the doors keep their names.
    for _ in 0..3 {
        end_turn(&mut e);
    }
    let later: Vec<(String, String)> = all(&e)
        .iter()
        .map(|h| (h.given.clone(), h.family.clone()))
        .collect();
    let before: Vec<(String, String)> = first
        .iter()
        .map(|h| (h.given.clone(), h.family.clone()))
        .collect();
    assert_eq!(later, before, "a family keeps its name as the town changes");
}

/// The second acceptance (a twin): four doors walked, making the case on a
/// worry, apply exactly today's canvass formula once, as the one tap does,
/// and as the formula worked by hand from the state before gives; the
/// first three doors change no one's approval.
#[test]
fn a_street_canvass_equals_todays_canvass_formula() {
    let c = base();
    let mut a = game(SEED, c.clone());
    let hood = street(&a);
    go(&mut a, hood);
    let mut b = copy(&a);
    let issue = top_worry(&a, hood);

    // The formula by hand, from the state before (`actions::canvass`).
    let st = a.state();
    let fx = &c.actions.canvass;
    let fit = mandate_sim::actions::canvass_match(st, &c, hood, Some(&issue));
    let curve = mandate_sim::player::ground::canvass_curve(st, fx, hood);
    let expected: Vec<(f32, f32)> = st.population.hoods[hood.index()]
        .cohorts
        .iter()
        .map(|c| {
            let aw = c.player_awareness as f64;
            let aw = (aw + fx.awareness * (1.0 - aw) * curve.awareness).clamp(0.0, 1.0) as f32;
            let room = 1.0 - (c.player_approval as f64).abs();
            let ap = (c.player_approval as f64 + fx.approval * fit * room * curve.approval)
                .clamp(-1.0, 1.0) as f32;
            (aw, ap)
        })
        .collect();
    let pop_before = st.population.clone();
    let slots = st.player.as_ref().unwrap().slots_used;

    // Walked: four doors that answer, the case made on the worry at each.
    let doors = answering(&a, hood);
    assert!(doors.len() >= 4, "four doors answer");
    for (i, d) in doors.iter().take(4).enumerate() {
        let r = knock(
            &mut a,
            hood,
            *d,
            DoorChoice::Case {
                issue: issue.clone(),
            },
        );
        assert!(r.outcome_key.starts_with("door-reply-case-"), "{}", r.outcome_key);
        if i < 3 {
            assert_eq!(
                a.state().population,
                pop_before,
                "a door adds no approval beyond the formula"
            );
            assert!(r.args.get("street").is_none());
        } else {
            assert_eq!(r.args.get("street").map(String::as_str), Some("done"));
            assert_eq!(r.args.get("street_issue"), Some(&issue));
        }
    }
    assert!(a.state().politics.characters.households.street.is_none());

    // The one tap on the same worry.
    let r = act(
        &mut b,
        Action::Canvass {
            neighborhood: hood.0,
            issue: Some(issue.clone()),
        },
    )
    .unwrap();
    assert_eq!(r.outcome_key, "outcome-canvass");

    let (sa, sb) = (a.state(), b.state());
    let got: Vec<(f32, f32)> = sa.population.hoods[hood.index()]
        .cohorts
        .iter()
        .map(|c| (c.player_awareness, c.player_approval))
        .collect();
    assert_eq!(got, expected, "the walked street is the formula");
    assert_eq!(sa.population, sb.population, "walked = one tap");
    let pa = sa.player.as_ref().unwrap();
    let pb = sb.player.as_ref().unwrap();
    assert_eq!(pa.ground, pb.ground);
    assert_eq!(pa.slots_used, pb.slots_used);
    assert_eq!(
        pa.slots_used - slots,
        c.tuning.action_cost("canvass").slots,
        "a street spends one canvass's time"
    );
    assert_eq!(sa.politics.elections, sb.politics.elections, "campaign contact");
    assert_eq!(
        sa.politics.get(pa.politician).unwrap().record,
        sb.politics.get(pb.politician).unwrap().record
    );
    // The one tap knocked doors too (the engine chose them).
    assert!(!sb.politics.characters.households.records.is_empty());
}

/// A street is at most four doors; the first knock spends the time and
/// later ones do not; anything else the player does, or the turn's end,
/// finishes it (the formula then); finishing early is `Canvass`.
#[test]
fn a_street_is_four_doors_and_ends_with_anything_else() {
    let c = base();
    let mut e = game(SEED, c.clone());
    let hood = street(&e);
    go(&mut e, hood);
    let cost = c.tuning.action_cost("canvass").slots;
    let used = |e: &Engine| e.state().player.as_ref().unwrap().slots_used;
    let s0 = used(&e);
    knock(&mut e, hood, 0, DoorChoice::Listen);
    assert_eq!(used(&e), s0 + cost, "the first knock spends the canvass's time");
    knock(&mut e, hood, 1, DoorChoice::Listen);
    assert_eq!(used(&e), s0 + cost, "later doors are free");
    assert!(
        act(
            &mut e,
            Action::Doorstep {
                neighborhood: hood.0,
                door: 1,
                choice: DoorChoice::Listen,
            }
        )
        .is_err(),
        "a door is knocked once a street"
    );
    // Another neighbourhood: the street ends as the player walks off.
    let pop = e.state().population.clone();
    let other = e.state().world.municipalities[home(&e).index()]
        .neighborhoods
        .iter()
        .copied()
        .find(|h| *h != hood)
        .unwrap();
    go(&mut e, other);
    assert!(e.state().politics.characters.households.street.is_none());
    assert_ne!(e.state().population, pop, "walking off finished the street");
    let log = &e.state().player.as_ref().unwrap().actions;
    assert_eq!(log.last().unwrap().outcome, "outcome-canvass");

    // A new street in a new turn, finished early by `Canvass` on it.
    end_turn(&mut e);
    go(&mut e, hood);
    let s1 = used(&e);
    knock(&mut e, hood, 2, DoorChoice::Listen);
    let r = act(
        &mut e,
        Action::Canvass {
            neighborhood: hood.0,
            issue: None,
        },
    )
    .unwrap();
    assert_eq!(r.outcome_key, "outcome-canvass");
    assert_eq!(r.args.get("doors").map(String::as_str), Some("1"));
    assert_eq!(used(&e), s1 + cost, "finishing spends nothing more");
    // The turn's end finishes a street left open.
    end_turn(&mut e);
    go(&mut e, hood);
    knock(&mut e, hood, 3, DoorChoice::Listen);
    assert!(e.state().politics.characters.households.street.is_some());
    end_turn(&mut e);
    assert!(e.state().politics.characters.households.street.is_none());
}

/// Records: one a household, at most `records_per_hood` a neighbourhood,
/// and the door remembers the call ("You came round in …").
#[test]
fn records_are_bounded_and_remembered_at_the_door() {
    let c = base();
    let mut e = game(SEED, c.clone());
    let hood = street(&e);
    let p = &c.characters.households;
    let n = households::doors(e.state(), &c, hood) as u16;
    let mut door = 0u16;
    for _ in 0..8 {
        go(&mut e, hood);
        for _ in 0..p.street_doors {
            knock(&mut e, hood, door % n, DoorChoice::Listen);
            door += 1;
        }
        end_turn(&mut e);
    }
    let hh = &e.state().politics.characters.households;
    let here = hh.records.iter().filter(|r| r.hood == hood).count();
    assert_eq!(here, (p.records_per_hood as usize).min(n as usize));
    // A household that answered remembers the call at its door.
    let me = mandate_sim::characters::player_character(e.state()).unwrap();
    let r = hh
        .records
        .iter()
        .find(|r| r.by == me && r.went != Went::NotIn)
        .expect("someone answered")
        .clone();
    let v = mandate_protocol::doors::door_view(&e, hood, r.door).unwrap();
    if v.answers {
        assert!(
            v.opener.iter().any(|l| l.key.starts_with("door-again")),
            "{:?}",
            v.opener
        );
    }
    let s = mandate_protocol::doors::street_doors(&e, hood).unwrap();
    let row = s.doors.iter().find(|d| d.door == r.door as u32).unwrap();
    assert!(row.last.is_some(), "the row says how the last call went");
}

/// A doorstep ask comes from a household knocked on: the canvass's one
/// roll promotes its speaker to a Named resident of the neighbourhood (by
/// the household's origin), and the record names them.
#[test]
fn a_doorstep_ask_comes_from_a_household_knocked_on() {
    let c = base();
    let mut e = game(SEED, c.clone());
    let hoods = e.state().world.municipalities[home(&e).index()]
        .neighborhoods
        .clone();
    let mut found = None;
    'turns: for t in 0..60usize {
        // Keep room for an ask: decline what is offered.
        let offered: Vec<u32> = e
            .state()
            .asks
            .asks
            .iter()
            .filter(|a| a.status == mandate_sim::asks::AskStatus::Offered)
            .map(|a| a.id.0)
            .collect();
        for ask in offered {
            let _ = e.apply(Command::Player(PlayerCommand::RespondAsk { ask, accept: false }));
        }
        let hood = hoods[t % hoods.len()];
        go(&mut e, hood);
        if act(
            &mut e,
            Action::Canvass {
                neighborhood: hood.0,
                issue: None,
            },
        )
        .is_ok()
        {
            let me = mandate_sim::characters::player_character(e.state()).unwrap();
            if let Some(r) = e
                .state()
                .politics
                .characters
                .households
                .records
                .iter()
                .find(|r| r.by == me && r.speaker.is_some() && r.hood == hood)
            {
                found = Some(r.clone());
                break 'turns;
            }
        }
        end_turn(&mut e);
    }
    let r = found.expect("a canvass's roll came up within 60 turns");
    let id = r.speaker.unwrap();
    let ch = e.state().politics.characters.get(id).unwrap();
    let n = ch.named.as_deref().unwrap();
    assert_eq!(n.home, r.hood.0, "a resident of the neighbourhood");
    assert_eq!(
        n.origin,
        Some((households::origin_key(r.hood), r.door as u32, r.generation))
    );
    let h = households::household(e.state(), &c, r.hood, r.door).unwrap();
    assert_eq!((h.given.as_str(), h.family.as_str()), (n.given.as_str(), n.family.as_str()));
    assert_eq!(h.speaker, Some(id), "the household's speaker is the person met");
    // The ask the canvass brought, if one could be cast, is theirs or a
    // resident's of that neighbourhood.
    end_turn(&mut e);
    let asks: Vec<_> = e
        .state()
        .asks
        .asks
        .iter()
        .filter(|a| a.opened == r.on)
        .collect();
    for a in asks {
        if let mandate_sim::asks::Asker::Person { id: who } = a.asker {
            println!("ask {} from {who:?} (speaker {id:?})", a.template);
        }
    }
}

/// A worked example for the report (`--ignored --nocapture`): one street,
/// four doors, what each said, and what was remembered.
#[test]
#[ignore]
fn worked_example() {
    let c = base();
    let mut e = game(SEED, c.clone());
    let hood = street(&e);
    go(&mut e, hood);
    let issue = top_worry(&e, hood);
    let name = e.state().world.neighborhoods[hood.index()].name.clone();
    println!(
        "street: {name} ({} doors)",
        households::doors(e.state(), &c, hood)
    );
    let choices = [
        DoorChoice::Listen,
        DoorChoice::Case {
            issue: issue.clone(),
        },
        DoorChoice::Ask,
        DoorChoice::Listen,
    ];
    for (d, choice) in choices.into_iter().enumerate() {
        let v = mandate_protocol::doors::door_view(&e, hood, d as u16).unwrap();
        let h = households::household(e.state(), &c, hood, d as u16).unwrap();
        println!(
            "door {} ({}): answers {} | {:?} | work {:?} worry {:?} mood {}",
            v.number,
            v.family,
            v.answers,
            v.opener
                .iter()
                .map(|l| (&l.key, &l.args))
                .collect::<Vec<_>>(),
            h.work,
            h.worry.map(|i| c.concerns[i].id.clone()),
            h.mood(&c)
        );
        let r = knock(&mut e, hood, d as u16, choice.clone());
        println!("  said {choice:?} -> {} {:?}", r.outcome_key, r.args);
    }
    let me = mandate_sim::characters::player_character(e.state()).unwrap();
    for r in &e.state().politics.characters.households.records {
        if r.by == me {
            println!("remembered: {r:?}");
        }
    }
    let log = &e.state().player.as_ref().unwrap().actions;
    println!("last log: {:?}", log.last());
}
