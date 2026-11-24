//! "Why?" through the protocol (S1, the causal spine): the chain behind an
//! event as facts and the engine's own text keys; the player's act nearest
//! behind it; the director's levers and hidden facts never shown; the
//! query's JSON shape.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_content::ContentBundle;
use mandate_protocol::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::events::{EventId, EventKind};
use mandate_sim::spine::{Cause, Causes, Link, PlayerAct, Process, Shock, Thread};
use mandate_sim::{Command, Engine};
use mandate_wasm::session::Session;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

fn content() -> Arc<ContentBundle> {
    static C: OnceLock<Arc<ContentBundle>> = OnceLock::new();
    C.get_or_init(|| {
        Arc::new(
            mandate_content::load(&mandate_content::embedded::sources())
                .unwrap()
                .0,
        )
    })
    .clone()
}

fn world(seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), content()).unwrap()
}

fn last_id(e: &Engine) -> EventId {
    e.state().events.iter().last().unwrap().id
}

#[test]
fn why_gives_the_chain_behind_a_law() {
    let mut e = world(2026);
    let from = last_id(&e);
    // Play on until the council has decided a bill tabled since.
    let mut decided = None;
    for _ in 0..40 {
        e.apply(Command::EndTurn).unwrap();
        decided = e.state().events.after(from).iter().find_map(|x| {
            let tabled_since = x.links().any(|(c, _)| c > from);
            matches!(
                x.kind,
                EventKind::PolicyEnacted { .. } | EventKind::PolicyFailed { .. }
            )
            .then_some(x.id)
            .filter(|_| tabled_since)
        });
        if decided.is_some() {
            break;
        }
    }
    let id = decided.expect("a bill was decided");
    let v = why_view(&e, id.0, None).unwrap();
    assert_eq!(v.event, id.0);
    // The event asked about comes first, with the engine's own text.
    let first = &v.chain[0];
    assert_eq!((first.id, first.depth), (id.0, 0));
    assert!(first.led_to.is_none());
    assert!(first.kind == "policy_enacted" || first.kind == "policy_failed");
    assert!(first.text_key.starts_with("event-policy-"));
    assert_eq!(
        first.text_key,
        event_view(e.state().events.get(id).unwrap(), &e).text_key
    );
    assert!(first.causes.contains(&CauseView::Process {
        process: "lawmaking".into()
    }));
    // Behind it: the introduction, one link back, and who tabled it.
    let intro = v
        .chain
        .iter()
        .find(|s| s.kind == "policy_introduced")
        .expect("the introduction is in the chain");
    assert_eq!(intro.depth, 1);
    let led = intro.led_to.as_ref().unwrap();
    assert_eq!(led.event, id.0);
    assert!(matches!(led.link, LinkDto::Continues | LinkDto::Ended));
    assert!(
        intro
            .causes
            .iter()
            .any(|c| matches!(c, CauseView::Actor { .. }))
    );
    assert!(first.causes.iter().any(|c| matches!(
        c,
        CauseView::Event { event, in_chain: true, .. } if *event == intro.id
    )));
    // Nobody played: no act of the player's lies behind it.
    assert_eq!(v.player, None);
    assert!(!v.truncated);
    // Steps are nearest first, dated, and every link's later end is a step.
    assert!(v.chain.windows(2).all(|w| w[0].depth <= w[1].depth));
    for s in &v.chain[1..] {
        let to = s.led_to.as_ref().unwrap().event;
        assert!(v.chain.iter().any(|x| x.id == to && x.depth + 1 == s.depth));
    }
    // No such event.
    assert!(why_view(&e, u32::MAX, None).is_none());
    // The depth is clamped to at least one link.
    let near = why_view(&e, id.0, Some(0)).unwrap();
    assert!(near.chain.iter().all(|s| s.depth <= 1));
}

#[test]
fn the_players_act_is_named() {
    let mut e = world(2026);
    let me = e.state().player.as_ref().unwrap().politician.0;
    let template = e
        .content()
        .policies
        .values()
        .find(|t| {
            let cmd = e.route(PlayerCommand::DraftPolicy {
                template: t.id.clone(),
                params: BTreeMap::new(),
            });
            let mut probe = Engine::from_state(e.state().clone(), e.content_arc()).unwrap();
            probe.apply(cmd).is_ok()
        })
        .map(|t| t.id.clone())
        .expect("a template the player can draft");
    let drafted = e
        .apply(e.route(PlayerCommand::DraftPolicy {
            template,
            params: BTreeMap::new(),
        }))
        .unwrap();
    let policy = drafted.result.unwrap().created.unwrap();
    e.apply(e.route(PlayerCommand::IntroducePolicy { policy }))
        .unwrap();
    let intro = e
        .state()
        .events
        .iter()
        .rev()
        .find(|x| matches!(x.kind, EventKind::PolicyIntroduced { sponsor, .. } if sponsor == me))
        .unwrap()
        .id;
    let v = why_view(&e, intro.0, None).unwrap();
    assert!(v.chain[0].causes.contains(&CauseView::Player {
        act: "introduce".into()
    }));
    assert_eq!(
        v.player,
        Some(WhyPlayerView {
            act: "introduce".into(),
            event: intro.0,
            depth: 0,
        })
    );
    // The bill's fate has the player's act behind it.
    let mut fate = None;
    for _ in 0..12 {
        e.apply(Command::EndTurn).unwrap();
        fate = e
            .state()
            .events
            .after(intro)
            .iter()
            .find_map(|x| match x.kind {
                EventKind::PolicyEnacted { policy: p, .. }
                | EventKind::PolicyFailed { policy: p, .. }
                    if p == policy =>
                {
                    Some(x.id)
                }
                _ => None,
            });
        if fate.is_some() {
            break;
        }
    }
    let v = why_view(&e, fate.expect("the bill was decided").0, None).unwrap();
    let p = v.player.expect("the player tabled it");
    assert_eq!(
        (p.act.as_str(), p.event, p.depth),
        ("introduce", intro.0, 1)
    );
}

#[test]
fn diagnostics_are_never_shown() {
    let mut e = world(2026);
    let me = e.state().player.as_ref().unwrap().politician.0;
    let (mut fact, mut broke, mut late, mut disaster) =
        (EventId(0), EventId(0), EventId(0), EventId(0));
    e.setup(|s| {
        let date = s.calendar.date;
        // The player's bribe becomes discoverable, and the story breaks.
        fact = s.events.record(
            date,
            EventKind::FactDiscoverable { hidden: 900 },
            [
                Cause::Player(PlayerAct::TakeBribe),
                Cause::Process(Process::Director),
            ],
        );
        let head = s.events.head(Thread::Hidden(900));
        broke = s.events.record(
            date,
            EventKind::ScandalBroke {
                scandal: 900,
                politician: me,
                municipality: 0,
                severity: "crime".into(),
                kind: "bribe".into(),
            },
            Causes::from(Cause::Process(Process::Press)).and_event(head, Link::Revealed),
        );
        // A hazard the director held, striking late.
        s.events.record(
            date,
            EventKind::DirectorIntervention {
                lever: "hazard_held".into(),
                municipality: 0,
            },
            [
                Cause::Exogenous(Shock::Hazard),
                Cause::Process(Process::Director),
            ],
        );
        late = s.events.record(
            date,
            EventKind::DirectorIntervention {
                lever: "hazard_struck_late".into(),
                municipality: 0,
            },
            Cause::Process(Process::Director),
        );
        disaster = s.events.record(
            date,
            EventKind::Disaster {
                municipality: 0,
                hazard: mandate_sim::environment::HazardKind::Flood,
                severity_pct: 50,
            },
            Causes::from(Cause::Exogenous(Shock::Hazard)).and_event(Some(late), Link::Continues),
        );
    });
    // The scandal: one step. What the hidden step named is the scandal's:
    // the player's own act. The director's timing is not.
    let v = why_view(&e, broke.0, None).unwrap();
    assert_eq!(v.chain.len(), 1);
    let causes = &v.chain[0].causes;
    assert!(causes.contains(&CauseView::Process {
        process: "press".into()
    }));
    assert!(causes.contains(&CauseView::Player {
        act: "take_bribe".into()
    }));
    assert!(!causes.contains(&CauseView::Process {
        process: "director".into()
    }));
    assert!(causes.iter().all(|c| !matches!(c, CauseView::Event { .. })));
    assert_eq!(
        v.player,
        Some(WhyPlayerView {
            act: "take_bribe".into(),
            event: broke.0,
            depth: 0,
        })
    );
    // The disaster is exogenous, and only that: the levers leave no trace.
    let v = why_view(&e, disaster.0, None).unwrap();
    assert_eq!(v.chain.len(), 1);
    assert_eq!(
        v.chain[0].causes,
        vec![CauseView::Exogenous {
            shock: "hazard".into()
        }]
    );
    assert_eq!(v.player, None);
    // The hidden events cannot be asked about.
    assert!(why_view(&e, fact.0, None).is_none());
    assert!(why_view(&e, late.0, None).is_none());
}

/// A hidden event is never a step, however many lie in a row: the walk
/// passes through six of them to what is behind, and through a run longer
/// than the walk's limit it stops and says so.
#[test]
fn a_run_of_hidden_events_is_never_shown() {
    let mut e = world(2026);
    let (mut root, mut top, mut deep_top) = (EventId(0), EventId(0), EventId(0));
    let mut hidden_ids = Vec::new();
    let lever = |municipality: u32| EventKind::DirectorIntervention {
        lever: "hazard_held".into(),
        municipality,
    };
    e.setup(|s| {
        let date = s.calendar.date;
        let disaster = |cause: Causes| -> (EventKind, Causes) {
            (
                EventKind::Disaster {
                    municipality: 0,
                    hazard: mandate_sim::environment::HazardKind::Flood,
                    severity_pct: 40,
                },
                cause,
            )
        };
        // A visible event, six hidden ones chained on it, a visible one on top.
        let (k, c) = disaster(Cause::Exogenous(Shock::Hazard).into());
        root = s.events.record(date, k, c);
        let mut prev = root;
        for i in 0..6 {
            prev = s.events.record(
                date,
                lever(1000 + i),
                Causes::from(Cause::Process(Process::Director))
                    .and_event(Some(prev), Link::Continues),
            );
            hidden_ids.push(prev);
        }
        let (k, c) = disaster(
            Causes::from(Cause::Exogenous(Shock::Hazard)).and_event(Some(prev), Link::Caused),
        );
        top = s.events.record(date, k, c);
        // And a run of twenty hidden events under another visible one.
        let mut prev = root;
        for i in 0..20 {
            prev = s.events.record(
                date,
                lever(2000 + i),
                Causes::from(Cause::Process(Process::Director))
                    .and_event(Some(prev), Link::Continues),
            );
            hidden_ids.push(prev);
        }
        let (k, c) = disaster(
            Causes::from(Cause::Exogenous(Shock::Hazard)).and_event(Some(prev), Link::Caused),
        );
        deep_top = s.events.record(date, k, c);
    });
    let shows_hidden = |v: &WhyView| {
        v.chain.iter().any(|s| {
            s.kind == "director_intervention"
                || hidden_ids.iter().any(|h| h.0 == s.id)
                || s.causes.iter().any(|c| {
                    matches!(c, CauseView::Event { event, .. } if hidden_ids.iter().any(|h| h.0 == *event))
                        || *c == CauseView::Process { process: "director".into() }
                })
        })
    };
    // Six in a row: the chain is the top event and, one link back, the root.
    let v = why_view(&e, top.0, None).unwrap();
    assert!(!shows_hidden(&v));
    assert_eq!(
        v.chain.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![top.0, root.0]
    );
    assert_eq!(v.chain[1].depth, 1);
    assert_eq!(v.chain[1].led_to.as_ref().unwrap().link, LinkDto::Caused);
    assert!(!v.truncated);
    // At any depth asked for.
    for depth in [1, 2, 3, 12] {
        assert!(!shows_hidden(&why_view(&e, top.0, Some(depth)).unwrap()));
        assert!(!shows_hidden(
            &why_view(&e, deep_top.0, Some(depth)).unwrap()
        ));
    }
    // Twenty in a row: past the limit the walk stops, shows none of them,
    // and says more lies behind.
    let v = why_view(&e, deep_top.0, None).unwrap();
    assert_eq!(v.chain.len(), 1);
    assert!(v.truncated);
    // None of them can be asked about.
    assert!(hidden_ids.iter().all(|h| why_view(&e, h.0, None).is_none()));
}

/// Why names people only as the player knows them: a step about someone
/// the player has not heard of leaves the name empty (the unknown form),
/// while the overview's line is unchanged.
#[test]
fn a_step_names_only_people_the_player_knows() {
    use mandate_sim::player::knowledge::{self, Level, Source, Subject};
    let mut e = world(2026);
    let (stranger, name) = {
        let st = e.state();
        let me = st.player.as_ref().unwrap().politician;
        let gate_level = |c| {
            character_view(&e, c)
                .map(|v| v.knowledge.level)
                .unwrap_or(KnowledgeLevelDto::Unknown)
        };
        let p = st
            .politics
            .politicians
            .iter()
            .filter(|p| p.id != me)
            .find(|p| gate_level(p.character.0) == KnowledgeLevelDto::Unknown)
            .expect("someone the player has not heard of");
        (p.clone(), p.full_name())
    };
    let mut broke = EventId(0);
    e.setup(|s| {
        let date = s.calendar.date;
        broke = s.events.record(
            date,
            EventKind::ScandalBroke {
                scandal: 901,
                politician: stranger.id.0,
                municipality: 0,
                severity: "ethics".into(),
                kind: "bribe".into(),
            },
            [
                Cause::Process(Process::Press),
                Cause::Actor(stranger.character),
            ],
        );
    });
    // The overview's line names them, as before.
    let line = event_view(e.state().events.get(broke).unwrap(), &e);
    assert_eq!(line.args.get("name"), Some(&name));
    // Why does not: not in the step's arguments, not as the actor.
    let v = why_view(&e, broke.0, None).unwrap();
    let step = &v.chain[0];
    assert_eq!(step.text_key, line.text_key);
    assert_eq!(step.args.get("name").map(String::as_str), Some(""));
    assert!(step.args.values().all(|a| !a.contains(&stranger.family)));
    assert!(step.causes.contains(&CauseView::Actor { person: None }));
    let json = serde_json::to_string(&v).unwrap();
    assert!(!json.contains(&name), "the name is nowhere in the answer");
    // Once the player has heard of them, the step names them.
    e.setup(|s| {
        knowledge::teach_one(
            s,
            Subject::Person {
                person: stranger.character,
            },
            Level::Heard,
            Source::Lab,
        );
    });
    let v = why_view(&e, broke.0, None).unwrap();
    assert_eq!(v.chain[0].args.get("name"), Some(&name));
    assert!(v.chain[0].causes.iter().any(|c| matches!(
        c,
        CauseView::Actor { person: Some(p) } if p.name == name
    )));
}

#[test]
fn why_through_the_protocol() {
    let mut s = Session::new().unwrap();
    let send = |s: &mut Session, body: RequestBody| -> Result<ResponseBody, ProtocolError> {
        let j = serde_json::to_string(&Request {
            protocol_version: PROTOCOL_VERSION,
            request_id: 1,
            body,
        })
        .unwrap();
        let r: Response = serde_json::from_str(&s.handle_json(&j)).unwrap();
        match r.result {
            ResponseResult::Ok { body } => Ok(body),
            ResponseResult::Err { error } => Err(error),
        }
    };
    let Ok(ResponseBody::GameStarted(o)) = send(
        &mut s,
        RequestBody::NewGame(NewGameParams {
            seed: 6,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
        }),
    ) else {
        panic!("no game");
    };
    // Every event the overview lists can be asked about; the query changes
    // nothing.
    let before = s.engine().unwrap().state_hash();
    for ev in &o.recent_events {
        let Ok(ResponseBody::Why(v)) = send(
            &mut s,
            RequestBody::Query(QueryDto::Why {
                event: ev.id,
                depth: None,
            }),
        ) else {
            panic!("no why for event {}", ev.id);
        };
        assert_eq!(v.event, ev.id);
        assert_eq!(v.chain[0].text_key, ev.text_key);
        assert!(!v.chain[0].causes.is_empty(), "{}", ev.text_key);
        assert!(v.chain[0].causes.iter().all(|c| *c != CauseView::Unknown));
    }
    assert!(!o.recent_events.is_empty());
    assert_eq!(s.engine().unwrap().state_hash(), before);
    assert!(
        send(
            &mut s,
            RequestBody::Query(QueryDto::Why {
                event: u32::MAX,
                depth: Some(3),
            }),
        )
        .is_err()
    );
    // The JSON a client sends and reads.
    let q: QueryDto = serde_json::from_str(r#"{"type":"why","event":7}"#).unwrap();
    assert_eq!(
        q,
        QueryDto::Why {
            event: 7,
            depth: None
        }
    );
    let causes = vec![
        CauseView::Event {
            event: 3,
            link: LinkDto::Prompted,
            in_chain: true,
        },
        CauseView::Player { act: "vote".into() },
        CauseView::Exogenous {
            shock: "hazard".into(),
        },
        CauseView::Unknown,
    ];
    assert_eq!(
        serde_json::to_string(&causes).unwrap(),
        r#"[{"type":"event","event":3,"link":"prompted","in_chain":true},{"type":"player","act":"vote"},{"type":"exogenous","shock":"hazard"},{"type":"unknown"}]"#
    );
}
