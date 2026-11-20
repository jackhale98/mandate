//! The act flow (DEC-012, DEC-006, DEC-098): time skips with directives and
//! interrupts, the player-proxy, act endings and the Legacy record.
//!
//! - a skip equals the same turns played by the proxy command by command,
//!   and a skip in chunks equals one in a go;
//! - each interrupt kind stops a skip (synthetic states), and a real skip
//!   stops when the campaign the player stands in opens;
//! - each ending condition fires;
//! - endings and interrupts under a renamed constitution, and a regional act
//!   by data alone;
//! - the Legacy record survives save/load and migration from v22.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::act::SkipTargetKind;
use mandate_content::{Source, bundle::read_dir_sources, load};
use mandate_save::*;
use mandate_sim::legacy::{LegacyEntry, LegacyKind};
use mandate_sim::politics::Role;
use mandate_sim::skip::{SkipStop, check_interrupts};
use mandate_sim::world::Jurisdiction;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn content() -> Arc<mandate_content::ContentBundle> {
    Arc::new(load(&sources()).unwrap().0)
}

/// A new game, one turn in (the first chapter has begun).
fn game(c: &Arc<mandate_content::ContentBundle>, seed: u64) -> Engine {
    let mut e = generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap();
    e.apply(Command::EndTurn).unwrap();
    e
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

/// The state's hash without the skip's own bookkeeping.
fn hash_without_skip(e: &mut Engine) -> String {
    e.setup(|s| {
        s.act.skip = None;
        s.act.last_skip = None;
    });
    e.state_hash()
}

fn directives() -> Command {
    Command::SetDirectives {
        choices: vec![
            ("directive.ground".into(), "lots".into()),
            ("directive.press".into(), "some".into()),
            ("directive.vote".into(), "ward".into()),
        ],
    }
}

/// A skip and the same turns played by the proxy, command by command,
/// through `Command::Player` and `EndTurn`, leave the same state.
#[test]
fn a_skip_equals_the_proxy_played_command_by_command() {
    let c = content();
    let mut a = game(&c, 21);
    let mut b = game(&c, 21);
    a.apply(directives()).unwrap();
    b.apply(directives()).unwrap();
    let out = a
        .apply(Command::Skip {
            target: SkipTargetKind::NextSession,
            limit: None,
        })
        .unwrap();
    let summary = a.state().act.last_skip.clone().expect("a summary");
    let turns = summary.turns;
    assert!(turns > 3, "the skip ran {turns} turns");
    assert!(out.skip.is_some_and(|p| p.done));
    for _ in 0..turns {
        let mut t = mandate_sim::proxy::ProxyTurn::new(b.state(), b.content());
        while let Some(cmd) = t.next(b.state(), b.content()) {
            let r = b.apply(Command::Player(cmd));
            let created = r
                .as_ref()
                .ok()
                .and_then(|o| o.result.as_ref())
                .and_then(|x| x.created);
            t.result(r.is_ok(), created);
        }
        b.apply(Command::EndTurn).unwrap();
    }
    assert_eq!(a.state().calendar.turn, b.state().calendar.turn);
    assert_eq!(hash_without_skip(&mut a), hash_without_skip(&mut b));
}

/// Skipping in one-turn chunks (as the web client does, to show progress
/// and allow a cancel) gives the same state, summary included.
#[test]
fn a_skip_in_chunks_equals_one_in_a_go() {
    let c = content();
    let mut a = game(&c, 22);
    let mut b = game(&c, 22);
    let target = SkipTargetKind::NextSession;
    a.apply(Command::Skip {
        target,
        limit: None,
    })
    .unwrap();
    let mut chunks = 0;
    loop {
        let out = b
            .apply(Command::Skip {
                target,
                limit: Some(1),
            })
            .unwrap();
        chunks += 1;
        if out.skip.unwrap().done {
            break;
        }
        assert!(chunks < 200);
    }
    assert!(chunks > 1);
    assert_eq!(a.state_hash(), b.state_hash());
    // A cancel ends the skip in progress with its summary.
    let mut d = game(&c, 22);
    d.apply(Command::Skip {
        target,
        limit: Some(2),
    })
    .unwrap();
    assert!(d.state().act.skip.is_some());
    d.apply(Command::CancelSkip).unwrap();
    let s = d.state().act.last_skip.clone().unwrap();
    assert_eq!(s.stop, SkipStop::Cancelled);
    assert_eq!(s.turns, 2);
}

type TrackChange = fn(&mut mandate_sim::player::Tracks);
type StateChange = Box<dyn Fn(&mut mandate_sim::State)>;

/// The stop a synthetic change makes, for a skip started now.
fn stop_after(e: &mut Engine, change: impl FnOnce(&mut mandate_sim::State)) -> Option<String> {
    let st = e.state().clone();
    let mut run =
        mandate_sim::skip::start(&st, e.content(), SkipTargetKind::EndOfTerm).expect("a target");
    let mut after = st.clone();
    change(&mut after);
    match check_interrupts(&mut run, &after, e.content()) {
        Some(SkipStop::Interrupted { interrupt, .. }) => Some(interrupt),
        _ => None,
    }
}

/// Each interrupt kind stops a skip when it newly holds, and not when it
/// held already as the skip began.
#[test]
fn each_interrupt_kind_stops_a_skip() {
    let c = content();
    let mut e = game(&c, 23);
    let me = e.state().player.as_ref().unwrap().politician;
    let date = e.state().calendar.date;
    // Nothing changed: no stop.
    assert_eq!(stop_after(&mut e, |_| {}), None);
    // Tracks crossing their thresholds.
    let tracks: [(&str, TrackChange); 5] = [
        ("interrupt.standing", |t| t.standing = 10.0),
        ("interrupt.viability", |t| t.viability = 5.0),
        ("interrupt.legal", |t| t.legal_jeopardy = 70.0),
        ("interrupt.threat", |t| t.threat = 90.0),
        ("interrupt.health", |t| t.health = 10.0),
    ];
    for (id, f) in tracks {
        let got = stop_after(&mut e, |s| f(&mut s.player.as_mut().unwrap().tracks));
        assert_eq!(got.as_deref(), Some(id));
    }
    // The run's status.
    let got = stop_after(&mut e, |s| {
        s.player.as_mut().unwrap().status = mandate_sim::player::RunStatus::Imprisoned {
            until: date.add_days(400),
            case: 0,
        }
    });
    assert_eq!(got.as_deref(), Some("interrupt.status"));
    // A story about the player.
    let got = stop_after(&mut e, |s| {
        let v = serde_json::json!({
            "id": 900, "hidden": 0, "subject": me.0, "kind": "bribe", "severity": "crime",
            "evidence": 0.5, "town": 0, "broke": date.0, "by": "journalist", "who": null,
            "outlet": null, "heat": 1.0, "stories": 1, "response": null, "responded": null,
            "backfire_on": null, "ended": null
        });
        s.media.scandals.push(serde_json::from_value(v).unwrap());
    });
    assert_eq!(got.as_deref(), Some("interrupt.scandal"));
    // A critical decision; a request falling due.
    let decision = |weight: &str, goal: bool, days: u32| {
        let v = serde_json::json!({
            "id": 900, "template": "dilemma.x", "weight": weight, "cast": [], "reasons": [],
            "opened": date.0, "deadline": date.add_days(days).0, "chapter": 1, "relevance": 1.0,
            "storyline": null, "goal": if goal { serde_json::json!([0, "goal.x"]) } else { serde_json::Value::Null },
            "place": null, "status": "pending", "resolution": null
        });
        serde_json::from_value::<mandate_sim::narrative::Decision>(v).unwrap()
    };
    let d = decision("climax", false, 30);
    let got = stop_after(&mut e, |s| s.narrative.decisions.push(d));
    assert_eq!(got.as_deref(), Some("interrupt.decision"));
    let d = decision("minor", true, 3);
    let got = stop_after(&mut e, |s| s.narrative.decisions.push(d));
    assert_eq!(got.as_deref(), Some("interrupt.request_due"));
    // A minor request not yet due does not stop it.
    let d = decision("minor", true, 60);
    assert_eq!(stop_after(&mut e, |s| s.narrative.decisions.push(d)), None);
    // A bill of the player's reaching a vote.
    let got = stop_after(&mut e, |s| {
        let mut p = s.policy.instances.first().cloned().expect("a policy");
        p.id = mandate_sim::policy::PolicyInstanceId(s.policy.instances.len() as u32);
        p.sponsor = Some(me);
        p.status = mandate_sim::policy::PolicyStatus::UnderConsideration;
        p.next_stage_on = Some(date.add_days(5));
        s.policy.instances.push(p);
    });
    assert_eq!(got.as_deref(), Some("interrupt.sponsored_vote"));
    // An election the player stands in.
    let got = stop_after(&mut e, |s| {
        let pl = s.player.as_ref().unwrap();
        let post = mandate_sim::player::posts(s)[0];
        let v = serde_json::json!({
            "seat": post, "opened": date.0, "election_day": date.add_days(50).0, "hoods": [],
            "incumbent": me.0, "partisan": true, "shock": [], "poll_bias_z": [],
            "candidates": [{ "politician": pl.politician.0, "party": null, "awareness": [],
                "awareness_start": [], "persuasion": [], "mobilisation": [], "contacts": [], "spent": 0.0 }]
        });
        s.politics
            .elections
            .races
            .push(serde_json::from_value(v).unwrap());
    });
    assert_eq!(got.as_deref(), Some("interrupt.election"));
    // What already held when the skip began does not fire.
    e.setup(|s| s.player.as_mut().unwrap().tracks.standing = 10.0);
    assert_eq!(
        stop_after(&mut e, |s| s.player.as_mut().unwrap().tracks.standing = 5.0),
        None
    );
}

/// A real skip to the end of the term is stopped when the campaign the
/// player stands in opens (unless something else stops it first), with a
/// summary of what happened; the world went on meanwhile.
#[test]
fn a_skip_is_called_back_for_the_campaign() {
    let c = content();
    let mut e = game(&c, 24);
    let start = e.state().calendar.date;
    let npc_bills = e.state().policy.instances.len();
    e.apply(Command::Skip {
        target: SkipTargetKind::EndOfTerm,
        limit: None,
    })
    .unwrap();
    let s = e.state().act.last_skip.clone().unwrap();
    assert!(s.turns > 0);
    assert!(e.state().calendar.date > start);
    match &s.stop {
        SkipStop::Interrupted { interrupt, .. } => {
            assert!(interrupt.starts_with("interrupt."), "{interrupt}")
        }
        other => panic!("expected an interrupt, got {other:?}"),
    }
    // The world did not wait: chapters began, NPCs drafted bills.
    assert!(e.state().policy.instances.len() >= npc_bills);
    assert_eq!(s.end.date, e.state().calendar.date);
    // Skipping on to the campaign: interrupted along the way (the player's
    // own bills coming to a vote, decisions), it gets there in the end, or
    // stops for the campaign the player stands in.
    let mut f = game(&c, 24);
    let mut stops = Vec::new();
    for _ in 0..40 {
        if f.apply(Command::Skip {
            target: SkipTargetKind::Campaign,
            limit: None,
        })
        .is_err()
        {
            break;
        }
        let stop = f.state().act.last_skip.clone().unwrap().stop;
        let done = matches!(&stop, SkipStop::Reached)
            || matches!(&stop, SkipStop::Interrupted { interrupt, .. } if interrupt == "interrupt.election");
        stops.push(stop);
        if done {
            break;
        }
    }
    assert!(
        stops.iter().any(|s| matches!(s, SkipStop::Reached)
            || matches!(s, SkipStop::Interrupted { interrupt, .. } if interrupt == "interrupt.election")),
        "{stops:?}"
    );
    // Its campaign has opened (the player is a candidate) or its date passed.
    let me = f.state().player.as_ref().map(|p| p.politician);
    assert!(
        f.state()
            .politics
            .elections
            .races
            .iter()
            .any(|r| r.candidates.iter().any(|x| Some(x.politician) == me))
            || mandate_sim::skip::targets(f.state(), f.content())
                .iter()
                .all(|t| t.target != SkipTargetKind::Campaign)
    );
}

fn ending_now(e: &Engine) -> Option<String> {
    mandate_sim::endings::check(e.state(), e.content()).map(|d| d.id.clone())
}

/// Every ending condition fires; a fresh career has none.
#[test]
fn each_ending_condition_fires() {
    let c = content();
    let base = game(&c, 25);
    assert_eq!(ending_now(&base), None);
    let rec = base
        .state()
        .act
        .current()
        .expect("the record opened at play start");
    assert_eq!(rec.act, "act.local");
    let pl = base.state().player.clone().unwrap();
    let me = pl.politician;
    let town = Jurisdiction::town(pl.home());
    let my_seat = mandate_sim::player::posts(base.state())[0];
    let fork = || Engine::from_state(base.state().clone(), c.clone()).unwrap();
    let mut cases: Vec<(&str, StateChange)> = Vec::new();
    cases.push((
        "ending.jailed",
        Box::new(|s| {
            let d = s.calendar.date.add_days(900);
            s.player.as_mut().unwrap().status =
                mandate_sim::player::RunStatus::Imprisoned { until: d, case: 1 };
        }),
    ));
    cases.push((
        "ending.removed",
        Box::new(|s| {
            let date = s.calendar.date;
            s.act.records.last_mut().unwrap().entries.push(LegacyEntry {
                date,
                kind: LegacyKind::RemovedFromOffice { case: 1 },
            });
        }),
    ));
    cases.push((
        "ending.disgraced",
        Box::new(|s| s.player.as_mut().unwrap().tracks.standing = 0.0),
    ));
    let region = town.parent(&base.state().world).unwrap();
    cases.push((
        "ending.higher_office",
        Box::new(move |s| {
            s.politics.get_mut(me).unwrap().role = Role::Seat {
                body: mandate_content::PostKey::new("assembly").unwrap(),
                at: region,
                constituency: None,
            };
        }),
    ));
    cases.push((
        "ending.executive",
        Box::new(move |s| {
            s.politics.get_mut(me).unwrap().role = Role::Office {
                office: mandate_content::PostKey::new("mayor").unwrap(),
                at: town,
            };
        }),
    ));
    cases.push((
        "ending.deselected",
        Box::new(|s| {
            let date = s.calendar.date;
            s.act.records.last_mut().unwrap().entries.push(LegacyEntry {
                date,
                kind: LegacyKind::Deselected,
            });
        }),
    ));
    cases.push((
        "ending.lost_seat",
        Box::new(move |s| {
            let date = s.calendar.date;
            s.politics.get_mut(me).unwrap().role = Role::Private {
                home: town.municipality().unwrap(),
            };
            s.act.records.last_mut().unwrap().entries.push(LegacyEntry {
                date,
                kind: LegacyKind::Election {
                    post: my_seat,
                    won: false,
                    share: 0.4,
                },
            });
        }),
    ));
    cases.push((
        "ending.served",
        Box::new(move |s| {
            // Two wins, the last a full term (less its campaign) ago.
            let date = s.calendar.date;
            let r = s.act.records.last_mut().unwrap();
            for back in [8 * 365, 4 * 365 - 30] {
                r.entries.push(LegacyEntry {
                    date: mandate_core::SimDate(date.0 - back),
                    kind: LegacyKind::Election {
                        post: my_seat,
                        won: true,
                        share: 0.55,
                    },
                });
            }
        }),
    ));
    for (want, f) in &cases {
        let mut e = fork();
        e.setup(|s| f(s));
        assert_eq!(ending_now(&e).as_deref(), Some(*want));
    }
    // One win is not "served", nor two with the term still running.
    let mut e = fork();
    e.setup(|s| {
        let date = s.calendar.date;
        let r = s.act.records.last_mut().unwrap();
        r.entries.push(LegacyEntry {
            date: mandate_core::SimDate(date.0 - 365),
            kind: LegacyKind::Election {
                post: my_seat,
                won: true,
                share: 0.55,
            },
        });
    });
    assert_eq!(ending_now(&e), None);
    // Through a turn: the ending closes the record with its facts, and a
    // skip is refused once the act is over.
    let mut e = fork();
    e.setup(|s| s.player.as_mut().unwrap().tracks.standing = 0.0);
    e.apply(Command::EndTurn).unwrap();
    let end = e.state().act.ending.clone().expect("the act ended");
    assert_eq!(end.ending, "ending.disgraced");
    let rec = e.state().act.current().unwrap();
    assert!(rec.closed.is_some());
    let er = rec.ending.as_ref().unwrap();
    assert_eq!(er.facts.first().map(|f| f.0.as_str()), Some("years"));
    assert!(rec.final_standing.is_some());
    assert!(
        e.apply(Command::Skip {
            target: SkipTargetKind::NextSession,
            limit: None
        })
        .is_err()
    );
}

/// The base pack with its local posts renamed (as R1's test): the council
/// is a `moot` of `alderman`s, the mayor a `reeve`.
fn renamed() -> Arc<mandate_content::ContentBundle> {
    let mut src = sources();
    for s in src.iter_mut() {
        let p = s.path.replace('\\', "/");
        if p.contains("locale/") || p.ends_with("council.jsonc") || p.ends_with("interiors.jsonc") {
            continue;
        }
        let mut t = s
            .text
            .replace(r#"{ "type": "mayor" }"#, r#"{ "type": "office" }"#);
        t = t.replace("\"councillor\"", "\"alderman\"");
        t = t.replace("\"mayor\"", "\"reeve\"");
        if p.contains("constitutions/") {
            t = t.replace("\"council\"", "\"moot\"");
            // And the acts renamed too.
            t = t.replace("act.local", "act.borough");
        }
        if p.ends_with("endings.jsonc") {
            t = t.replace("act.local", "act.borough");
        }
        s.text = t;
    }
    Arc::new(load(&src).unwrap().0)
}

/// Under a constitution whose posts and acts have other names, the record
/// opens in its first act, the executive ending fires for its executive,
/// and skips are offered and interrupted as before.
#[test]
fn endings_and_interrupts_under_a_renamed_constitution() {
    let c = renamed();
    let mut e = game(&c, 26);
    let pl = e.state().player.clone().unwrap();
    let town = Jurisdiction::town(pl.home());
    assert_eq!(e.state().act.current().unwrap().act, "act.borough");
    let targets = mandate_sim::skip::targets(e.state(), e.content());
    assert!(targets.len() >= 3, "{targets:?}");
    // The executive (the reeve) by its powers.
    let mut f = Engine::from_state(e.state().clone(), c.clone()).unwrap();
    f.setup(|s| {
        s.politics.get_mut(pl.politician).unwrap().role = Role::Office {
            office: mandate_content::PostKey::new("reeve").unwrap(),
            at: town,
        };
    });
    assert_eq!(ending_now(&f).as_deref(), Some("ending.executive"));
    // Interrupts read the renamed seat's race.
    let got = stop_after(&mut e, |s| {
        let post = mandate_sim::player::posts(s)[0];
        assert_eq!(post.id(), "moot");
        let v = serde_json::json!({
            "seat": post, "opened": 0, "election_day": 99999, "hoods": [],
            "incumbent": null, "partisan": true, "shock": [], "poll_bias_z": [],
            "candidates": [{ "politician": pl.politician.0, "party": null, "awareness": [],
                "awareness_start": [], "persuasion": [], "mobilisation": [], "contacts": [], "spent": 0.0 }]
        });
        s.politics
            .elections
            .races
            .push(serde_json::from_value(v).unwrap());
    });
    assert_eq!(got.as_deref(), Some("interrupt.election"));
    // A skip runs under it.
    e.apply(Command::Skip {
        target: SkipTargetKind::NextSession,
        limit: None,
    })
    .unwrap();
    assert!(e.state().act.last_skip.is_some());
}

/// A regional act by data alone: a record opened for a player in the
/// regional assembly is in the regional act; the region's executive (by
/// its powers) ends it as `executive`, a national post as `higher_office`.
/// Then the career's acts reordered (the executive office an act of its
/// own): winning it is a later act.
#[test]
fn a_regional_act_and_reordered_acts_by_data() {
    let c = content();
    let mut e = game(&c, 27);
    let pl = e.state().player.clone().unwrap();
    let region = Jurisdiction::town(pl.home())
        .parent(&e.state().world)
        .unwrap();
    let seat = |b: &str, at| Role::Seat {
        body: mandate_content::PostKey::new(b).unwrap(),
        at,
        constituency: None,
    };
    let c1 = c.clone();
    e.setup(|s| {
        s.politics.get_mut(pl.politician).unwrap().role = seat("assembly", region);
        s.act.records.clear();
        mandate_sim::legacy::begin(s, &c1);
    });
    let rec = e.state().act.current().unwrap().clone();
    assert_eq!(rec.act, "act.regional");
    let mut f = Engine::from_state(e.state().clone(), c.clone()).unwrap();
    f.setup(|s| {
        s.politics.get_mut(pl.politician).unwrap().role = Role::Office {
            office: mandate_content::PostKey::new("governor").unwrap(),
            at: region,
        };
    });
    assert_eq!(ending_now(&f).as_deref(), Some("ending.executive"));
    let mut g = Engine::from_state(e.state().clone(), c.clone()).unwrap();
    g.setup(|s| {
        s.politics.get_mut(pl.politician).unwrap().role = seat("house", Jurisdiction::Nation);
    });
    assert_eq!(ending_now(&g).as_deref(), Some("ending.higher_office"));
    // Reordered acts: the mayoralty is an act of its own after the council.
    let mut src = sources();
    for s in src.iter_mut() {
        if s.path.contains("constitutions/") {
            s.text = s.text.replace(
                r#"{ "id": "act.local", "tier": "local" },"#,
                r#"{ "id": "act.local", "tier": "local", "except": ["mayor"] }, { "id": "act.town_leader", "posts": ["mayor"] },"#,
            );
        }
    }
    let c2 = Arc::new(load(&src).unwrap().0);
    let mut h = game(&c2, 28);
    let pl = h.state().player.clone().unwrap();
    h.setup(|s| {
        s.politics.get_mut(pl.politician).unwrap().role = Role::Office {
            office: mandate_content::PostKey::new("mayor").unwrap(),
            at: Jurisdiction::town(pl.home()),
        };
    });
    assert_eq!(ending_now(&h).as_deref(), Some("ending.higher_office"));
}

/// The Legacy record survives save/load, and a v22 save migrates to an
/// empty act flow whose record opens at the next turn.
#[test]
fn the_legacy_record_survives_save_load_and_migration() {
    let c = content();
    let mut e = game(&c, 29);
    e.apply(directives()).unwrap();
    for _ in 0..6 {
        e.apply(Command::EndTurn).unwrap();
    }
    e.setup(|s| {
        let date = s.calendar.date;
        s.act.records.last_mut().unwrap().entries.push(LegacyEntry {
            date,
            kind: LegacyKind::PromiseKept { person: 3 },
        });
    });
    e.apply(Command::Skip {
        target: SkipTargetKind::NextSession,
        limit: Some(2),
    })
    .unwrap();
    let (bytes, _) = encode(e.state(), &req()).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    assert_eq!(state.act, e.state().act);
    assert_eq!(state.hash(), e.state_hash());
    // Continue the skip after the load: the same as without it.
    let mut loaded = Engine::from_state(state, c.clone()).unwrap();
    let next = Command::Skip {
        target: SkipTargetKind::NextSession,
        limit: None,
    };
    loaded.apply(next.clone()).unwrap();
    e.apply(next).unwrap();
    assert_eq!(loaded.state_hash(), e.state_hash());
    // v22: no act flow in the payload.
    let mut v =
        serde_json::json!({ "state": serde_json::to_value(e.state()).unwrap(), "log": null });
    v["state"].as_object_mut().unwrap().remove("act");
    let migrated = migrate(22, v).unwrap();
    let st: mandate_sim::State = serde_json::from_value(migrated["state"].clone()).unwrap();
    assert!(st.act.records.is_empty());
    let mut m = Engine::from_state(st, c.clone()).unwrap();
    m.apply(Command::EndTurn).unwrap();
    let rec = m
        .state()
        .act
        .current()
        .expect("the record opens at the next turn");
    assert_eq!(rec.act, "act.local");
    assert!(!rec.start_posts.is_empty());
}
