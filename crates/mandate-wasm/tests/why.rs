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

/// What the player knows of someone (the ledger, as their sheet shows it).
fn level_of(e: &Engine, c: mandate_sim::characters::CharacterId) -> KnowledgeLevelDto {
    character_view(e, c.0)
        .map(|v| v.knowledge.level)
        .unwrap_or(KnowledgeLevelDto::Unknown)
}

/// A politician the player has not heard of, and their name.
fn a_stranger(e: &Engine) -> (mandate_sim::politics::Politician, String) {
    let st = e.state();
    let me = st.player.as_ref().unwrap().politician;
    let p = st
        .politics
        .politicians
        .iter()
        .filter(|p| p.id != me)
        .find(|p| level_of(e, p.character) == KnowledgeLevelDto::Unknown)
        .expect("someone the player has not heard of");
    (p.clone(), p.full_name())
}

/// Why names people only as the player knows them: a step about someone
/// the player has not heard of leaves the name empty (the unknown form),
/// while the overview's line is unchanged. (An investigation opening is
/// not a public result: see `PUBLIC_RESULTS` and the tests below.)
#[test]
fn a_step_names_only_people_the_player_knows() {
    use mandate_sim::player::knowledge::{self, Level, Source, Subject};
    let mut e = world(2026);
    let (stranger, name) = a_stranger(&e);
    let mut broke = EventId(0);
    e.setup(|s| {
        let date = s.calendar.date;
        broke = s.events.record(
            date,
            EventKind::CaseOpened {
                case: 901,
                politician: stranger.id.0,
            },
            [
                Cause::Process(Process::Justice),
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

// --- A public result is public (DEC-108 §8) ------------------------------

/// The public kinds, as a list: a kind joins it on purpose, with a test
/// below.
#[test]
fn the_public_results_are_these() {
    assert_eq!(
        PUBLIC_RESULTS,
        [
            "race_decided",
            "policy_enacted",
            "policy_failed",
            "scandal_broke",
            "case_developed"
        ]
    );
}

/// A step of a public kind about someone the player has not heard of: the
/// step reads as the public line does, name and all, and nothing is
/// written down (the ledger and the state are as they were). `as_town`:
/// the town carries the stranger's name, for a kind whose line names no
/// person today (a law's recorded vote), so the rule has a name to keep.
fn public_step(
    as_town: bool,
    kind: impl FnOnce(&mandate_sim::politics::Politician) -> EventKind,
    causes: impl FnOnce(&mandate_sim::politics::Politician) -> Causes,
) -> (WhyStepView, String) {
    let mut e = world(2026);
    let (stranger, name) = a_stranger(&e);
    let mut id = EventId(0);
    e.setup(|s| {
        if as_town {
            s.world.municipalities[0].name = name.clone();
        }
        let date = s.calendar.date;
        id = s.events.record(date, kind(&stranger), causes(&stranger));
    });
    let before = e.state_hash();
    let v = why_view(&e, id.0, None).unwrap();
    let step = v.chain[0].clone();
    assert!(
        PUBLIC_RESULTS.contains(&step.kind.as_str()),
        "{}",
        step.kind
    );
    assert!(public_result(&e.state().events.get(id).unwrap().kind));
    // The step reads as the public line does.
    let line = event_view(e.state().events.get(id).unwrap(), &e);
    assert_eq!((&step.text_key, &step.args), (&line.text_key, &line.args));
    assert!(step.args.values().any(|a| *a == name), "{:?}", step.args);
    // A query writes nothing: the player has still not heard of them.
    assert_eq!(e.state_hash(), before);
    assert_eq!(level_of(&e, stranger.character), KnowledgeLevelDto::Unknown);
    (step, name)
}

#[test]
fn race_decided_names_its_winner() {
    let (step, name) = public_step(
        false,
        |p| EventKind::RaceDecided {
            municipality: 0,
            mayor: false,
            winner: p.id.0,
            incumbent_defeated: false,
            player_won: false,
            player_ran: false,
            post: None,
        },
        |_| {
            [
                Cause::Process(Process::Elections),
                Cause::Exogenous(Shock::Race),
            ]
            .into()
        },
    );
    assert_eq!(step.kind, "race_decided");
    assert_eq!(step.args.get("name"), Some(&name));
}

#[test]
fn policy_enacted_reads_as_the_public_record() {
    let (step, name) = public_step(
        true,
        |_| EventKind::PolicyEnacted {
            policy: 0,
            municipality: 0,
            yes: 5,
            no: 2,
            overrode_veto: false,
        },
        |_| Cause::Process(Process::Lawmaking).into(),
    );
    assert_eq!(step.kind, "policy_enacted");
    assert_eq!(step.args.get("town"), Some(&name));
    assert_eq!(step.args.get("yes").map(String::as_str), Some("5"));
}

#[test]
fn policy_failed_reads_as_the_public_record() {
    let (step, name) = public_step(
        true,
        |_| EventKind::PolicyFailed {
            policy: 0,
            municipality: 0,
            vetoed: false,
            yes: 3,
            no: 4,
        },
        |_| Cause::Process(Process::Lawmaking).into(),
    );
    assert_eq!(step.kind, "policy_failed");
    assert_eq!(step.args.get("town"), Some(&name));
    assert_eq!(step.args.get("no").map(String::as_str), Some("4"));
}

#[test]
fn scandal_broke_names_its_subject() {
    let (step, name) = public_step(
        false,
        |p| EventKind::ScandalBroke {
            scandal: 901,
            politician: p.id.0,
            municipality: 0,
            severity: "ethics".into(),
            kind: "bribe".into(),
        },
        |p| [Cause::Process(Process::Press), Cause::Actor(p.character)].into(),
    );
    assert_eq!(step.kind, "scandal_broke");
    assert_eq!(step.args.get("name"), Some(&name));
    // Whom the story names is named as the one whose choice it was.
    assert!(step.causes.iter().any(|c| matches!(
        c,
        CauseView::Actor { person: Some(p) } if p.name == name
    )));
}

#[test]
fn case_developed_names_its_subject() {
    for stage in ["charged", "dropped", "acquitted", "convicted"] {
        let (step, name) = public_step(
            false,
            |p| EventKind::CaseDeveloped {
                case: 901,
                politician: p.id.0,
                stage: stage.into(),
            },
            |_| Cause::Process(Process::Justice).into(),
        );
        assert_eq!(step.kind, "case_developed");
        assert_eq!(step.args.get("name"), Some(&name), "{stage}");
    }
}

/// Everything else stays under the knowledge gate: the same stranger in a
/// step of another kind is not named, in its arguments or as its actor,
/// and neither is anyone a public step does not itself name.
#[test]
fn every_other_kind_stays_gated() {
    let mut e = world(2026);
    let (stranger, name) = a_stranger(&e);
    let other = {
        let st = e.state();
        st.politics
            .politicians
            .iter()
            .find(|p| {
                p.id != stranger.id
                    && level_of(&e, p.character) == KnowledgeLevelDto::Unknown
                    && p.full_name() != name
            })
            .expect("a second stranger")
            .clone()
    };
    let mut ids: Vec<EventId> = Vec::new();
    let mut law = EventId(0);
    e.setup(|s| {
        let date = s.calendar.date;
        let gated: Vec<EventKind> = vec![
            EventKind::CaseOpened {
                case: 902,
                politician: stranger.id.0,
            },
            EventKind::RumourSpread {
                rumour: 902,
                politician: stranger.id.0,
            },
            EventKind::RumourCorrected {
                rumour: 902,
                politician: stranger.id.0,
            },
            EventKind::PolicyIntroduced {
                policy: 0,
                municipality: 0,
                sponsor: stranger.id.0,
            },
        ];
        for kind in gated {
            ids.push(s.events.record(
                date,
                kind,
                [
                    Cause::Process(Process::Press),
                    Cause::Actor(stranger.character),
                ],
            ));
        }
        // A public step names its subject, not whoever else chose something.
        law = s.events.record(
            date,
            EventKind::ScandalBroke {
                scandal: 903,
                politician: stranger.id.0,
                municipality: 0,
                severity: "ethics".into(),
                kind: "bribe".into(),
            },
            [
                Cause::Process(Process::Press),
                Cause::Actor(other.character),
            ],
        );
    });
    for id in ids {
        let v = why_view(&e, id.0, None).unwrap();
        let step = &v.chain[0];
        assert!(
            !PUBLIC_RESULTS.contains(&step.kind.as_str()),
            "{}",
            step.kind
        );
        assert!(step.args.values().all(|a| *a != name), "{}", step.kind);
        assert!(
            step.causes.contains(&CauseView::Actor { person: None }),
            "{}",
            step.kind
        );
        assert!(!serde_json::to_string(&v).unwrap().contains(&name));
    }
    let v = why_view(&e, law.0, None).unwrap();
    assert_eq!(v.chain[0].args.get("name"), Some(&name));
    assert!(
        v.chain[0]
            .causes
            .contains(&CauseView::Actor { person: None })
    );
    assert!(
        !serde_json::to_string(&v)
            .unwrap()
            .contains(&other.full_name())
    );
}

// --- Every typed value has a line ----------------------------------------

/// Every variant of a type, as its serde key. The closure is an exhaustive
/// match: a variant missing from the list does not compile, so a new cause
/// cannot be added without being listed here, and then needs its line.
macro_rules! every {
    ($ty:ident: $($v:ident),+ $(,)?) => {{
        let _exhaustive = |x: $ty| match x {
            $($ty::$v => ()),+
        };
        vec![$(serde_key(&$ty::$v)),+]
    }};
}

fn serde_key<T: serde::Serialize>(v: &T) -> String {
    match serde_json::to_value(v).unwrap() {
        serde_json::Value::String(s) => s,
        other => panic!("not a plain key: {other}"),
    }
}

/// The Why panel words every typed value the view can carry (links, the
/// player's acts, processes, pressures, shocks and the bare roots) from
/// `locale/en/why.ftl`: a value with no line fails here.
#[test]
fn every_cause_and_link_has_a_line() {
    use mandate_sim::spine::Pressure;
    let keys = mandate_content::validate::fluent_keys(&content().locale_text("en"));
    let mut wanted: Vec<String> = Vec::new();
    let mut family = |prefix: &str, variants: Vec<String>| {
        assert!(!variants.is_empty());
        wanted.extend(variants.into_iter().map(|v| format!("{prefix}-{v}")));
    };
    family(
        "why-link",
        every!(Link: Caused, Continues, Prompted, Revealed, Ended),
    );
    // The protocol's own link type has the same keys.
    family(
        "why-link",
        every!(LinkDto: Caused, Continues, Prompted, Revealed, Ended),
    );
    family(
        "why-player",
        every!(PlayerAct: Start, Vote, File, Draft, Amend, Introduce, Withdraw, Move, Meet,
            Answer, LetPass, Talk, RespondAsk, Promise, Lobby, TradeFavor, TownHall, Press,
            Canvass, Fundraise, Rest, Advertise, TakeBribe, DipFunds, LeakFile, Visit, Casework),
    );
    family(
        "why-process",
        every!(Process: Calendar, Markets, CentralBank, Budget, Lawmaking, Agenda, Elections,
            Parties, Director, Press, Justice, Asks, Society, Knowledge, Arcs, Update),
    );
    family(
        "why-pressure",
        every!(Pressure: WorldDemand, CommodityPrices, Depletion, Competitiveness, LabourCosts,
            Track),
    );
    family(
        "why-exogenous",
        every!(Shock: WorldEconomy, Commodities, Hazard, Race),
    );
    // The roots with no value of their own: every kind of `CauseView` but a
    // link, the player's act and the families above.
    let bare = |c: &CauseView| match c {
        CauseView::History => Some("why-history"),
        CauseView::Forgotten => Some("why-forgotten"),
        CauseView::Unknown => Some("why-unknown"),
        CauseView::Actor { person: Some(_) } => Some("why-actor"),
        CauseView::Actor { person: None } => Some("why-actor-unnamed"),
        CauseView::Event { .. }
        | CauseView::Player { .. }
        | CauseView::Process { .. }
        | CauseView::Pressure { .. }
        | CauseView::Exogenous { .. } => None,
    };
    let e = world(2026);
    let someone = person_ref(&e, e.state().politics.politicians[0].character);
    assert!(someone.is_some());
    for c in [
        CauseView::History,
        CauseView::Forgotten,
        CauseView::Unknown,
        CauseView::Actor { person: someone },
        CauseView::Actor { person: None },
    ] {
        wanted.push(bare(&c).unwrap().to_string());
    }
    let missing: Vec<&String> = wanted.iter().filter(|k| !keys.contains(*k)).collect();
    assert!(missing.is_empty(), "no English line for {missing:?}");
    // And no line for a value the engine cannot send (a renamed variant
    // leaves its old line behind).
    let families = [
        "why-link-",
        "why-player-",
        "why-process-",
        "why-pressure-",
        "why-exogenous-",
    ];
    let stale: Vec<&String> = keys
        .iter()
        .filter(|k| families.iter().any(|f| k.starts_with(f)))
        .filter(|k| !wanted.contains(k))
        .collect();
    assert!(stale.is_empty(), "lines for no value: {stale:?}");
}

// --- The views that name an event to ask about ---------------------------

/// A bill's card names the event that decided it, and every line of the
/// overview's list can be asked about.
#[test]
fn a_bill_card_names_the_event_that_decided_it() {
    let mut e = world(2026);
    let from = last_id(&e);
    // Play on until the player's council has decided a bill tabled since.
    let mut decided = None;
    for _ in 0..60 {
        e.apply(Command::EndTurn).unwrap();
        decided = office_view(&e).unwrap().bills.iter().find_map(|b| {
            b.outcome_event
                .filter(|ev| *ev > from.0)
                .map(|ev| (b.id, ev))
        });
        if decided.is_some() {
            break;
        }
    }
    let (bill, ev) = decided.expect("a bill at home was decided");
    let event = e.state().events.get(EventId(ev)).unwrap();
    assert!(matches!(
        event.kind,
        EventKind::PolicyEnacted { policy, .. }
            | EventKind::PolicyFailed { policy, .. }
            | EventKind::PolicyExpired { policy, .. }
            | EventKind::PolicyWithdrawn { policy, .. } if policy == bill
    ));
    // Behind it: at least its introduction.
    let v = why_view(&e, ev, None).unwrap();
    assert!(v.chain.len() >= 2, "{:?}", v.chain);
    assert!(v.chain.iter().any(|s| s.kind == "policy_introduced"));
    // A draft or a bill still before the body names none; whatever a card
    // names can be asked about and is that bill's.
    for b in &office_view(&e).unwrap().bills {
        if matches!(b.status.as_str(), "draft" | "under_consideration") {
            assert_eq!(b.outcome_event, None, "{} {}", b.id, b.status);
        }
        if let Some(ev) = b.outcome_event {
            let v = why_view(&e, ev, None).unwrap();
            assert!(
                v.chain[0].kind.starts_with("policy_"),
                "{}",
                v.chain[0].kind
            );
        }
    }
    // The overview's list names events too: each line can be asked about.
    for line in &overview(&e).recent_events {
        assert!(why_view(&e, line.id, None).is_some(), "{}", line.text_key);
    }
}

/// A race's result card names the event of the result; the chain behind it
/// is the campaign, and the result's own factors travel with the view.
#[test]
fn a_result_names_its_race() {
    let mut e = world(2);
    let before = campaign_view(&e).unwrap().last_election;
    for _ in 0..80 {
        e.apply(Command::EndTurn).unwrap();
        if campaign_view(&e).unwrap().last_election != before {
            break;
        }
    }
    let cv = campaign_view(&e).unwrap();
    assert_ne!(cv.last_election, before, "an election was held");
    assert!(!cv.results.is_empty());
    for r in &cv.results {
        let ev = r.event.expect("the result's event");
        let v = why_view(&e, ev, None).unwrap();
        assert_eq!(v.chain[0].kind, "race_decided");
        assert_eq!(v.chain[0].date, r.date);
        // The winner is named, known or not (a public result).
        let winner = r.candidates.iter().find(|c| c.is_winner).unwrap();
        assert_eq!(v.chain[0].args.get("name"), Some(&winner.name));
        assert!(v.chain[0].causes.contains(&CauseView::Exogenous {
            shock: "race".into()
        }));
        // Behind it: the campaign that led to election day.
        assert!(v.chain.iter().any(|s| s.kind == "campaign_opened"));
    }
    // The Legacy record's lines name their events (`source_event`): the
    // player's race is the result's event, and each can be asked about.
    let mine = cv
        .results
        .iter()
        .find(|r| r.player_ran)
        .expect("the player stood");
    let legacy = legacy_view(&e).unwrap();
    assert!(
        legacy
            .entries
            .iter()
            .any(|l| l.text_key.starts_with("legacy-election") && l.event == mine.event),
        "{:?}",
        legacy.entries
    );
    for ev in legacy.entries.iter().filter_map(|l| l.event) {
        assert!(why_view(&e, ev, None).is_some());
    }
}

/// A line of the skip summary names its event where a thread of the log
/// keeps it (a law, a decision, an ask, a story, a case), and never an
/// event of another kind than the line reports.
#[test]
fn a_summary_line_names_its_event() {
    use mandate_content::act::SkipTargetKind;
    let mut named = 0;
    for seed in [15u64, 2026, 2] {
        let mut e = world(seed);
        e.apply(Command::EndTurn).unwrap();
        for _ in 0..3 {
            if e.apply(Command::Skip {
                target: SkipTargetKind::NextSession,
                limit: None,
            })
            .is_err()
            {
                break;
            }
            let Some(s) = skip_summary_view(&e) else {
                continue;
            };
            for line in &s.items {
                let Some(ev) = line.event else { continue };
                named += 1;
                let v = why_view(&e, ev, None).expect("the line's event can be asked about");
                let kind = v.chain[0].kind.as_str();
                let fits = match line.text_key.as_str() {
                    k if k.starts_with("skip-item-law-expired")
                        || k.starts_with("skip-item-law-repealed") =>
                    {
                        kind == "policy_expired"
                    }
                    k if k.starts_with("skip-item-law") => kind == "policy_enacted",
                    "skip-item-bill-failed" => kind == "policy_failed",
                    "skip-item-bill-withdrawn" => kind == "policy_withdrawn",
                    "skip-item-answered" | "skip-item-defaulted" => kind == "decision_resolved",
                    "skip-item-scandal" => kind == "scandal_broke",
                    "skip-item-case" => kind == "case_opened" || kind == "case_developed",
                    k if k.starts_with("skip-item-ask-") => kind == "ask_resolved",
                    _ => false,
                };
                assert!(fits, "{} names a {kind}", line.text_key);
            }
        }
    }
    assert!(named > 0, "no summary line named an event");
}

/// The answer says how far it walked and how far a query may ask.
#[test]
fn the_view_says_how_far_it_walked() {
    let e = world(2026);
    let id = last_id(&e);
    let v = why_view(&e, id.0, None).unwrap();
    assert_eq!((v.depth, v.max_depth), (WHY_DEPTH, WHY_MAX_DEPTH));
    let v = why_view(&e, id.0, Some(200)).unwrap();
    assert_eq!((v.depth, v.max_depth), (WHY_MAX_DEPTH, WHY_MAX_DEPTH));
    let v = why_view(&e, id.0, Some(0)).unwrap();
    assert_eq!(v.depth, 1);
}
