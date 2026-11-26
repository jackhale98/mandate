//! Upkeep with faults, and casework on faults (task B2, DEC-114, "being
//! there"): a budget cut shows as more open faults and a louder
//! roads-and-upkeep worry (a twin run; DEC-106 moved faults from the
//! services worry); "Sort it out" moves a fault up the repair queue, the
//! fault it goes ahead of is put back, and "Why?" names it; a resident's
//! ask is cast on a fault near their door and kept when it is fixed,
//! whoever fixes it; the views say what is wrong and where; and with no
//! fault content nothing changes.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]
use mandate_content::ContentBundle;
use mandate_protocol::*;
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::asks::{Ask, AskId, AskStatus, Asker, History, Need};
use mandate_sim::events::EventKind;
use mandate_sim::narrative::{Actor, CastEntry};
use mandate_sim::places::Location;
use mandate_sim::upkeep::{Fault, FaultId, FaultStage, FaultStatus};
use mandate_sim::world::{MunicipalityId, NeighborhoodId};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::Arc;

const INFRA: usize = 4; // `finance::Category::Infrastructure`

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

fn hoods(e: &Engine) -> Vec<NeighborhoodId> {
    e.state().world.municipalities[home(e).index()]
        .neighborhoods
        .clone()
}

/// Faults not yet repaired in the home town.
fn unrepaired(e: &Engine) -> usize {
    e.state()
        .upkeep
        .faults
        .iter()
        .filter(|f| !f.fixed())
        .count()
}

/// The roads-and-upkeep worry as the home town's residents feel it (mean
/// over its neighbourhoods).
fn upkeep_felt(e: &Engine) -> f64 {
    let st = e.state();
    let i = st
        .population
        .concern_keys
        .iter()
        .position(|k| k == "concern.upkeep")
        .unwrap();
    let hs = hoods(e);
    hs.iter()
        .map(|h| f64::from(st.population.hood_concerns[h.index()].levels[i]))
        .sum::<f64>()
        / hs.len() as f64
}

/// Play idle turns until `days` have passed, calling `each` after every turn.
fn idle(e: &mut Engine, days: u32, each: &mut dyn FnMut(&Engine)) {
    let until = e.state().calendar.date.add_days(days);
    while e.state().calendar.date < until {
        e.apply(Command::EndTurn).unwrap();
        each(e);
    }
}

/// The twin run (B2's first acceptance): the same world, once as it is and
/// once with the town's Infrastructure line halved. Over two years the cut
/// town has more faults waiting and its residents feel the roads-and-upkeep
/// worry more; the world as it is keeps its streets in repair.
#[test]
fn a_budget_cut_raises_open_faults_and_the_upkeep_worry() {
    let c = base();
    let (mut open, mut worry) = ([0usize; 2], [0.0f64; 2]);
    for seed in [2026u64, 7, 31] {
        let a = game(seed, c.clone());
        let mut b = copy(&a);
        let m = home(&a).index();
        b.setup(|st| {
            let line = &mut st.finance.local[m].budget[INFRA];
            *line = line.scale(0.5);
        });
        let mut row = [(0usize, 0.0f64, 0u32); 2];
        for (k, e) in [a, b].iter_mut().enumerate() {
            idle(e, 2 * 365, &mut |e| {
                row[k].0 += unrepaired(e);
                row[k].1 += upkeep_felt(e);
                row[k].2 += 1;
            });
        }
        println!(
            "seed {seed}: faults waiting per turn {:.2} -> {:.2}; upkeep worry {:.3} -> {:.3}",
            row[0].0 as f64 / f64::from(row[0].2),
            row[1].0 as f64 / f64::from(row[1].2),
            row[0].1 / f64::from(row[0].2),
            row[1].1 / f64::from(row[1].2),
        );
        for k in 0..2 {
            open[k] += row[k].0;
            worry[k] += row[k].1 / f64::from(row[k].2);
        }
    }
    assert!(
        open[1] > open[0],
        "a cut leaves more faults waiting: {open:?}"
    );
    assert!(
        worry[1] > worry[0],
        "a cut makes the upkeep worry louder: {worry:?}"
    );
}

/// A fault in a neighbourhood, waiting in the queue since `opened`.
fn plant(e: &mut Engine, id: u32, hood: NeighborhoodId, opened_days_ago: u32) -> FaultId {
    let f = FaultId(id);
    e.setup(|st| {
        let opened = mandate_core::SimDate(st.calendar.date.0 - opened_days_ago);
        st.upkeep.faults.push(Fault {
            id: f,
            kind: "fault.streetlight".into(),
            hood,
            spot: (id % 6) as u8,
            opened,
            status: FaultStatus::Open,
            expedited: None,
            ahead_of: None,
            put_back_for: None,
        });
        st.upkeep.faults.sort_by_key(|f| f.id);
    });
    f
}

/// A Named resident of the town living in `hood`, if one does.
fn resident_of(e: &Engine, hood: NeighborhoodId) -> Option<mandate_sim::characters::CharacterId> {
    let st = e.state();
    st.politics
        .characters
        .people
        .iter()
        .filter(|c| c.politician.is_none())
        .find(|c| c.named.as_deref().is_some_and(|n| n.home == hood.0))
        .map(|c| c.id)
}

/// An accepted ask about a fault, from a resident of its neighbourhood.
fn ask_about(e: &mut Engine, fault: FaultId, asker: mandate_sim::characters::CharacterId) -> AskId {
    let hood = e.state().upkeep.get(fault).unwrap().hood;
    let mut id = AskId(0);
    e.setup(|st| {
        let date = st.calendar.date;
        id = st.asks.ids.alloc();
        st.asks.asks.push(Ask {
            id,
            template: "ask.fault".into(),
            promise: false,
            asker: Asker::Person { id: asker },
            cast: vec![
                CastEntry {
                    role: "asker".into(),
                    actor: Actor::Person { id: asker },
                },
                CastEntry {
                    role: "asker_hood".into(),
                    actor: Actor::Neighborhood { id: hood.0 },
                },
            ],
            needs: vec![Need::Fixed { fault }],
            step: 0,
            done: Vec::new(),
            public: false,
            big: false,
            opened: date,
            respond_by: date,
            due: date.add_days(60),
            status: AskStatus::Accepted { on: date },
            history: History::None,
            race: None,
            answer: None,
            tries: 0,
            arc: None,
        });
    });
    id
}

/// Leave the home town's queue holding only the faults planted, and give
/// the month's repair money room for exactly one street light.
fn one_repair_a_month(e: &mut Engine) {
    let m = home(e).index();
    let c = e.content_arc();
    e.setup(|st| {
        st.upkeep.faults.clear();
        let price = st.economy.national.price_level;
        let cost = c.fault("fault.streetlight").unwrap().repair.cost * price;
        let fund = cost * 1.5;
        let yearly = fund * 12.0 / c.upkeep.params.repair_share;
        st.finance.local[m].budget[INFRA] = mandate_core::Money::from_units(yearly as i64);
    });
}

/// Days to the next month's first day (the queue's turn) from the turn's
/// first day.
fn to_next_month(e: &Engine) -> u32 {
    let d = e.state().calendar.date;
    let c = d.calendar();
    let next = if c.month == 12 {
        mandate_core::SimDate::from_ymd(c.year + 1, 1, 1)
    } else {
        mandate_core::SimDate::from_ymd(c.year, c.month + 1, 1)
    };
    d.days_until(next)
}

fn sort_it_out(e: &mut Engine, ask: AskId) {
    for _ in 0..8 {
        if !mandate_sim::asks::can_casework(e.state(), ask.0) {
            return;
        }
        let c = e.route(PlayerCommand::Act {
            action: Action::Casework { ask: ask.0 },
            late_night: false,
        });
        if e.apply(c).is_err() {
            e.apply(Command::EndTurn).unwrap();
        }
    }
    panic!("casework never went through");
}

/// B2's second acceptance (the owner's answer 3): "Sort it out" moves a
/// fault up the queue and does not buy the repair. With money for one
/// repair this month, the newer fault moved up is repaired and the older
/// one waits; both say so, and "Why?" on the one put back walks to the
/// fault that went ahead and the player's casework.
#[test]
fn an_expedited_fault_delays_the_next_and_why_names_it() {
    let c = base();
    let mut e = game(2026, c);
    let hood = hoods(&e)[0];
    let asker = resident_of(&e, hood).expect("a resident");
    one_repair_a_month(&mut e);
    let old = plant(&mut e, 9_001, hood, 40);
    let new = plant(&mut e, 9_002, hood, 5);
    let ask = ask_about(&mut e, new, asker);
    // The twin with no casework: the older fault is repaired first.
    let mut twin = copy(&e);
    sort_it_out(&mut e, ask);
    assert!(e.state().upkeep.get(new).unwrap().expedited.is_some());
    let days = to_next_month(&e) + 1;
    e.apply(Command::AdvanceDays { days }).unwrap();
    twin.apply(Command::AdvanceDays {
        days: to_next_month(&twin) + 1,
    })
    .unwrap();
    let st = e.state();
    let (o, n) = (st.upkeep.get(old).unwrap(), st.upkeep.get(new).unwrap());
    assert!(
        matches!(n.status, FaultStatus::Repairing { .. }),
        "{:?}",
        n.status
    );
    assert_eq!(o.status, FaultStatus::Open);
    assert_eq!(n.ahead_of, Some(old));
    assert_eq!(o.put_back_for, Some(new));
    let t = twin.state();
    assert!(matches!(
        t.upkeep.get(old).unwrap().status,
        FaultStatus::Repairing { .. }
    ));
    assert_eq!(t.upkeep.get(new).unwrap().status, FaultStatus::Open);
    // The views say so.
    let ov = fault_view(&e, o);
    assert_eq!(ov.note_key.as_deref(), Some("ui-fault-put-back"));
    assert_eq!(
        ov.args.get("other_key").map(String::as_str),
        Some("fault-streetlight-name")
    );
    let nv = fault_view(&e, n);
    assert_eq!(nv.note_key.as_deref(), Some("ui-fault-ahead"));
    assert!(nv.moved_up);
    // "Why?" on the fault put back.
    let ev = ov.event.expect("its latest event");
    let why = why_view(&e, ev, None).unwrap();
    let keys: Vec<&str> = why.chain.iter().map(|s| s.text_key.as_str()).collect();
    assert_eq!(keys[0], "event-fault-delayed");
    assert!(keys.contains(&"event-fault-repairing-ahead"), "{keys:?}");
    assert!(keys.contains(&"event-fault-expedited"), "{keys:?}");
    assert_eq!(
        why.player.as_ref().map(|p| p.act.as_str()),
        Some("casework")
    );
    // The new fault is repaired, and the ask kept (by the repair).
    e.apply(Command::AdvanceDays { days: 8 }).unwrap();
    assert!(e.state().upkeep.get(new).unwrap().fixed());
    let a = e.state().asks.get(ask).unwrap();
    assert!(matches!(a.status, AskStatus::Kept { .. }), "{:?}", a.status);
    // Next month the older one's turn comes.
    e.apply(Command::AdvanceDays {
        days: to_next_month(&e) + 1,
    })
    .unwrap();
    assert!(!e.state().upkeep.get(old).unwrap().open());
}

/// A resident's ask is cast on the oldest fault waiting near their door,
/// and it is kept when the fault is fixed, whoever fixes it (the world
/// does not wait, DEC-091). Two asks are never about one fault.
#[test]
fn an_ask_is_cast_on_a_fault_near_the_door_and_kept_when_it_is_fixed() {
    let c = base();
    let mut e = game(2026, c);
    let hood = hoods(&e)[0];
    let asker = resident_of(&e, hood).expect("a resident");
    one_repair_a_month(&mut e);
    let f = plant(&mut e, 9_010, hood, 30);
    // Canvass the neighbourhood until someone brings it (a doorstep ask).
    let mut asked = None;
    for _ in 0..40 {
        let c = e.route(PlayerCommand::Act {
            action: Action::Canvass {
                neighborhood: hood.0,
                issue: None,
            },
            late_night: false,
        });
        if e.apply(c).is_err() {
            e.apply(Command::EndTurn).unwrap();
        }
        asked = e
            .state()
            .asks
            .open()
            .find(|a| a.template == "ask.fault")
            .map(|a| a.id);
        if asked.is_some() || !e.state().upkeep.get(f).is_some_and(|x| x.open()) {
            break;
        }
    }
    let _ = asker;
    let Some(id) = asked else {
        // Repaired by the queue before anyone asked: that is allowed.
        assert!(!e.state().upkeep.get(f).unwrap().open());
        return;
    };
    let a = e.state().asks.get(id).unwrap().clone();
    assert_eq!(a.needs, vec![Need::Fixed { fault: f }]);
    let v = ask_view(&e, &a);
    let fv = v.fault.expect("the ask shows its fault");
    assert_eq!(fv.id, f.0);
    assert_eq!(fv.see.neighborhood, Some(hood.0));
    assert_eq!(
        v.args.get("fault_key").map(String::as_str),
        Some("fault-streetlight-name")
    );
    e.apply(Command::Player(PlayerCommand::RespondAsk {
        ask: id.0,
        accept: true,
    }))
    .unwrap();
    // Nobody sorts it out: the queue reaches it all the same.
    for _ in 0..6 {
        if !e.state().asks.get(id).unwrap().open() {
            break;
        }
        e.apply(Command::EndTurn).unwrap();
    }
    let st = e.state();
    let a = st.asks.get(id).unwrap();
    assert!(matches!(a.status, AskStatus::Kept { .. }), "{:?}", a.status);
    assert!(st.upkeep.get(f).is_none_or(|x| x.fixed()));
}

/// The place view of a neighbourhood's streets lists its faults with the
/// spot and what is wrong ("See it"), and faults open, are repaired and
/// look new through a save.
#[test]
fn the_street_shows_its_faults_and_they_survive_a_save() {
    let c = base();
    let mut e = game(7, c.clone());
    idle(&mut e, 365, &mut |_| {});
    let st = e.state();
    assert!(!st.upkeep.hoods.is_empty(), "the focus's condition is kept");
    let opened = st
        .events
        .iter()
        .filter(|x| {
            matches!(
                x.kind,
                EventKind::Fault {
                    stage: FaultStage::Opened,
                    ..
                }
            )
        })
        .count();
    assert!(opened > 0, "faults open in a year");
    let f = st
        .upkeep
        .faults
        .first()
        .expect("a fault on the streets")
        .clone();
    let panel = place_panel_view(
        &e,
        LocationDto::of(Location::Street {
            neighborhood: f.hood,
        }),
    )
    .unwrap();
    let fv = panel
        .faults
        .iter()
        .find(|x| x.id == f.id.0)
        .expect("the street lists it");
    assert!(!fv.spot.kind.is_empty() && fv.spot.slot < fv.spot.slots);
    assert!(fv.what_key.starts_with("fault-") && fv.what_key.ends_with("-what"));
    assert!(!fv.cue.is_empty());
    // Through a save, the same.
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
    let mut back = Engine::from_state(st, e.content_arc()).unwrap();
    assert_eq!(back.state().upkeep, e.state().upkeep);
    assert_eq!(back.state_hash(), e.state_hash());
    // And they go on the same.
    idle(&mut e, 90, &mut |_| {});
    idle(&mut back, 90, &mut |_| {});
    assert_eq!(back.state_hash(), e.state_hash());
}

/// With no fault kinds in the content, the upkeep domain stays empty, is
/// not written, and no ask is cast on a fault (B2's third acceptance; the
/// lab's spine proof games with the faults removed give the digests of
/// before upkeep).
#[test]
fn with_no_fault_content_nothing_changes() {
    let mut bare = (*base()).clone();
    bare.upkeep.faults.clear();
    let mut e = game(2026, Arc::new(bare));
    idle(&mut e, 2 * 365, &mut |_| {});
    let st = e.state();
    assert!(st.upkeep.is_empty());
    let v = serde_json::to_value(st).unwrap();
    assert!(v.get("upkeep").is_none());
    assert!(
        !st.events
            .iter()
            .any(|x| matches!(x.kind, EventKind::Fault { .. }))
    );
    assert!(!st.asks.asks.iter().any(|a| a.template == "ask.fault"));
}

/// The worked example and the queue's numbers (`cargo test -p mandate-wasm
/// --test faults report -- --ignored --nocapture`): a player who canvasses
/// where faults are, takes the ask a resident brings, and sorts it out at
/// the services desk, until a fault moved up puts another back.
#[test]
#[ignore]
fn report() {
    let c = base();
    for seed in 1..=40u64 {
        let mut e = game(seed, c.clone());
        let start = e.state().calendar.date;
        let mut mine: Vec<AskId> = Vec::new();
        let until = start.add_days(3 * 365);
        let mut found = None;
        while e.state().calendar.date < until && found.is_none() {
            // Answer a fault ask, sort out the ones accepted.
            let offered: Vec<AskId> = e
                .state()
                .asks
                .open()
                .filter(|a| a.template == "ask.fault" && a.offered())
                .map(|a| a.id)
                .collect();
            for id in offered {
                let _ = e.apply(Command::Player(PlayerCommand::RespondAsk {
                    ask: id.0,
                    accept: true,
                }));
                mine.push(id);
            }
            for id in mine.clone() {
                if mandate_sim::asks::can_casework(e.state(), id.0) {
                    let c = e.route(PlayerCommand::Act {
                        action: Action::Casework { ask: id.0 },
                        late_night: false,
                    });
                    let _ = e.apply(c);
                }
            }
            // Canvass a neighbourhood with a fault waiting.
            let h = e
                .state()
                .upkeep
                .faults
                .iter()
                .find(|f| f.open())
                .map(|f| f.hood);
            if let Some(h) = h {
                for _ in 0..2 {
                    let c = e.route(PlayerCommand::Act {
                        action: Action::Canvass {
                            neighborhood: h.0,
                            issue: None,
                        },
                        late_night: false,
                    });
                    let _ = e.apply(c);
                }
            }
            e.apply(Command::EndTurn).unwrap();
            found = e.state().events.iter().find_map(|x| match &x.kind {
                EventKind::Fault {
                    stage: FaultStage::Delayed,
                    ..
                } => Some(x.id),
                _ => None,
            });
        }
        let Some(ev) = found else { continue };
        println!(
            "seed {seed}: delayed event {} on {:?}",
            ev.0,
            e.state().events.get(ev).unwrap().date
        );
        let why = why_view(&e, ev.0, None).unwrap();
        for s in &why.chain {
            println!(
                "  {} {:?} {} {:?} causes {:?}",
                s.depth, s.date, s.text_key, s.args, s.causes
            );
        }
        for a in e
            .state()
            .asks
            .asks
            .iter()
            .filter(|a| a.template == "ask.fault")
        {
            let v = ask_view(&e, a);
            println!(
                "  ask {} {} asker {:?} args {:?} opened {:?}",
                a.id.0, v.status, v.asker, v.args, a.opened
            );
        }
        for f in &e.state().upkeep.faults {
            println!("  fault {:?}", f);
        }
        // The month's money against costs.
        let r = mandate_sim::scheduler::Reads::of_state(e.state(), e.content());
        let fund = mandate_sim::upkeep::monthly_fund(
            &r,
            home(&e),
            mandate_content::BudgetCategory::Infrastructure,
        );
        println!(
            "  fund this month {fund:.0}; a street light costs {:.0}",
            mandate_sim::upkeep::repair_cost(&r, e.content().fault("fault.streetlight").unwrap())
        );
        break;
    }
    // A year of the queue in an idle game.
    for seed in [2026u64, 7, 31] {
        let mut e = game(seed, c.clone());
        let mut stages = std::collections::BTreeMap::<&str, u32>::new();
        idle(&mut e, 365, &mut |_| {});
        for x in e.state().events.iter() {
            if let EventKind::Fault { stage, .. } = &x.kind {
                *stages.entry(stage.key()).or_default() += 1;
            }
        }
        let st = e.state();
        let conds: Vec<f32> = st.upkeep.hoods.iter().map(|h| h.condition).collect();
        let r = mandate_sim::scheduler::Reads::of_state(st, e.content());
        let fund = mandate_sim::upkeep::monthly_fund(
            &r,
            home(&e),
            mandate_content::BudgetCategory::Infrastructure,
        );
        println!("seed {seed}: a year: {stages:?}; condition {conds:?}; fund {fund:.0}/month");
    }
}

/// The baseline queue over five years, and the upkeep worry with and
/// without faults (`--ignored --nocapture`).
#[test]
#[ignore]
fn report_trajectory() {
    let c = base();
    let mut bare = (*c).clone();
    bare.upkeep.faults.clear();
    let bare = Arc::new(bare);
    for seed in [2026u64, 7, 31] {
        let mut e = game(seed, c.clone());
        let mut n = game(seed, bare.clone());
        let mut line = Vec::new();
        for half in 1..=10 {
            idle(&mut e, 182, &mut |_| {});
            idle(&mut n, 182, &mut |_| {});
            let conds: f32 = e
                .state()
                .upkeep
                .hoods
                .iter()
                .map(|h| h.condition)
                .sum::<f32>()
                / e.state().upkeep.hoods.len().max(1) as f32;
            line.push(format!(
                "{}m: {} waiting, cond {:.2}, worry {:.3} (no faults {:.3})",
                half * 6,
                unrepaired(&e),
                conds,
                upkeep_felt(&e),
                upkeep_felt(&n)
            ));
        }
        println!("seed {seed}:\n  {}", line.join("\n  "));
    }
}
