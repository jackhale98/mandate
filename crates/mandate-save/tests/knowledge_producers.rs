//! Knowledge producers (K3; DEC-091, M6_ENGINE_CONTRACTS §1.3): arrival
//! introductions, overheard talk, actions, decisions and scenes, stories,
//! and the narrative `learn` / `reveal_place` effects and `knows` condition.
//! Learning is written but, until K2/K4, nothing reads it for outcomes.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{Condition, ContentBundle, SubjectDef, bundle::read_dir_sources, load};
use mandate_save::{RunMode, SaveRequest, decode, encode};
use mandate_sim::actions::{Action, PlayerCommand};
use mandate_sim::characters::CharacterId;
use mandate_sim::narrative::{
    self, Actor, CastEntry, Decision, DecisionId, DecisionStatus, Weight,
};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{self, KnowledgeLedger, Level, Source, Subject};
use mandate_sim::world::{NeighborhoodId, PlaceId, PlaceKind};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn content() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn game(seed: u64, c: &Arc<ContentBundle>) -> Engine {
    generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        c.clone(),
    )
    .unwrap()
}

fn ledger(e: &Engine) -> &KnowledgeLedger {
    &e.state().player.as_ref().unwrap().knowledge
}

fn level(e: &Engine, s: &Subject) -> Level {
    ledger(e).level(s)
}

fn town_places(e: &Engine) -> Vec<(PlaceId, PlaceKind)> {
    let st = e.state();
    let town = st.player.as_ref().unwrap().municipality;
    st.world
        .places
        .iter()
        .filter(|p| p.municipality == town)
        .map(|p| (p.id, p.kind))
        .collect()
}

fn move_to(e: &mut Engine, place: PlaceId) {
    e.apply(Command::Player(PlayerCommand::MoveTo {
        location: Location::Place { place },
    }))
    .unwrap();
}

fn canvass(e: &mut Engine, hood: NeighborhoodId) {
    let cmd = e.route(PlayerCommand::Act {
        action: Action::Canvass {
            neighborhood: hood.0,
        },
        late_night: false,
    });
    e.apply(cmd).unwrap();
}

#[test]
fn first_arrival_teaches_the_intro_once() {
    let c = content();
    let mut e = game(31, &c);
    // A place whose introduction has something new for a sitting councillor.
    let (place, intro) = town_places(&e)
        .into_iter()
        .find_map(|(p, _)| {
            let intro = knowledge::visit_intro(e.state(), &c, p);
            intro
                .iter()
                .any(|g| level(&e, &g.subject) < g.level)
                .then_some((p, intro))
        })
        .expect("some place introduces something new");
    assert!(!ledger(&e).visited.contains(&place));
    move_to(&mut e, place);
    let k = ledger(&e);
    assert!(k.visited.contains(&place));
    for g in &intro {
        assert!(k.level(&g.subject) >= g.level, "{:?}", g.subject);
    }
    let from_visit = |k: &KnowledgeLedger| {
        k.entries
            .values()
            .flat_map(|x| x.learned.iter())
            .filter(|l| l.source == Source::Visit { place })
            .count()
    };
    let n = from_visit(k);
    assert!(n > 0);
    // Leaving and coming back gives no second introduction.
    e.apply(Command::Player(PlayerCommand::MoveTo {
        location: Location::Home,
    }))
    .unwrap();
    move_to(&mut e, place);
    assert_eq!(from_visit(ledger(&e)), n);
    let mut again = usize::MAX;
    e.setup(|st| again = knowledge::arrive_at(st, &c, place));
    assert_eq!(again, 0, "the introduction is given once");
    // Arrival teaches lightly: introductions and names, never detail.
    assert!(
        ledger(&e)
            .entries
            .values()
            .flat_map(|x| x.learned.iter())
            .filter(|l| matches!(l.source, Source::Visit { .. } | Source::Overheard { .. }))
            .all(|l| l.level <= Level::Known)
    );
}

#[test]
fn canvassing_a_neighbourhood_raises_its_concerns() {
    let c = content();
    let mut e = game(32, &c);
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    let ward: Vec<NeighborhoodId> = mandate_sim::policy::ward_hoods(&st.world, pl.ward)
        .into_iter()
        .map(|h| NeighborhoodId(h as u32))
        .collect();
    let outside = st
        .world
        .hoods_of(pl.municipality)
        .map(|h| NeighborhoodId(h as u32))
        .find(|h| !ward.contains(h))
        .expect("a neighbourhood outside the ward");
    let s = Subject::HoodConcerns { hood: outside };
    assert_eq!(level(&e, &s), Level::Unknown);
    // The first canvass: its worries, first-hand.
    canvass(&mut e, outside);
    assert_eq!(level(&e, &s), Level::Known);
    let entry = ledger(&e).entry(&s).unwrap();
    assert!(entry.reading.is_none());
    assert!(matches!(
        &entry.learned[1].source,
        Source::Action { action, .. } if action == "canvass"
    ));
    // The second canvass reads them: detailed, dated, by concern.
    e.apply(Command::EndTurn).unwrap();
    canvass(&mut e, outside);
    let expected = knowledge::hood_reading(e.state(), &c, outside);
    assert!(!expected.is_empty());
    assert_eq!(level(&e, &s), Level::Detailed);
    let r = ledger(&e).entry(&s).unwrap().reading.clone().unwrap();
    assert_eq!(r.on, e.state().calendar.date);
    assert_eq!(r.values, expected);
    // A ward neighbourhood the councillor already knows is read at once.
    let w = Subject::HoodConcerns { hood: ward[0] };
    assert_eq!(level(&e, &w), Level::Known);
    e.apply(Command::EndTurn).unwrap();
    canvass(&mut e, ward[0]);
    assert_eq!(level(&e, &w), Level::Detailed);
}

#[test]
fn an_overheard_line_with_teaches_records_heard() {
    let c = content();
    let mut found = false;
    for seed in 33..41u64 {
        let mut e = game(seed, &c);
        let st = e.state();
        let Some((place, taught)) = town_places(&e).into_iter().find_map(|(p, _)| {
            let loc = Location::Place { place: p };
            let services = mandate_sim::places::services_at(st, &c, loc);
            let t = mandate_sim::overheard::teachings(st, &c, loc);
            let new: Vec<_> = t
                .into_iter()
                .filter(|(_, s)| ledger(&e).level(s) == Level::Unknown)
                .collect();
            (services.contains(&"overheard") && !new.is_empty()).then_some((p, new))
        }) else {
            continue;
        };
        // Every line that teaches is in the registry with that subject type.
        for (line, _) in &taught {
            assert!(
                !c.overheard
                    .iter()
                    .find(|d| &d.id == line)
                    .unwrap()
                    .teaches
                    .is_empty()
            );
        }
        let mut n = 0;
        e.setup(|st| n = mandate_sim::overheard::overhear(st, &c, Location::Place { place }));
        assert!(n > 0);
        for (_, s) in &taught {
            assert_eq!(level(&e, s), Level::Heard, "{s:?}");
            let first = &ledger(&e).entry(s).unwrap().learned[0];
            // The first line in the draw that teaches it is the source.
            let by = taught.iter().find(|(_, x)| x == s).unwrap();
            assert_eq!(
                first.source,
                Source::Overheard {
                    place,
                    line: by.0.clone()
                }
            );
        }
        // Arriving there does the same (through the arrival hook).
        let mut f = game(seed, &c);
        move_to(&mut f, place);
        for (_, s) in &taught {
            assert!(level(&f, s) >= Level::Heard);
        }
        found = true;
        break;
    }
    assert!(found, "some seed has a teaching line at a place");
}

/// A pending decision injected for the test (the director's timing is its
/// own business; answering is what teaches).
fn inject(e: &mut Engine, template: &str, cast: Vec<CastEntry>) -> DecisionId {
    let mut id = DecisionId(0);
    e.setup(|st| {
        id = st.narrative.decision_ids.alloc();
        let date = st.calendar.date;
        st.narrative.decisions.push(Decision {
            id,
            template: template.into(),
            weight: Weight::Significant,
            cast,
            reasons: Vec::new(),
            opened: date,
            deadline: date.add_days(30),
            chapter: 0,
            relevance: 1.0,
            storyline: None,
            goal: None,
            place: None,
            status: DecisionStatus::Pending,
            resolution: None,
            about: None,
        });
    });
    id
}

/// Two Named people the player has never met, and a ward neighbourhood.
fn strangers(e: &Engine) -> (CharacterId, CharacterId, NeighborhoodId) {
    let st = e.state();
    let pl = st.player.as_ref().unwrap();
    let town = pl.municipality;
    let people: Vec<CharacterId> = st
        .politics
        .characters
        .people
        .iter()
        .filter(|c| c.named.is_some())
        .filter(|c| {
            mandate_sim::characters::municipality_of(&st.politics, &st.world, c.id) == Some(town.0)
        })
        .map(|c| c.id)
        .filter(|id| ledger(e).level(&Subject::Person { person: *id }) < Level::Known)
        .take(2)
        .collect();
    let hood = mandate_sim::policy::ward_hoods(&st.world, pl.ward)[0];
    (people[0], people[1], NeighborhoodId(hood as u32))
}

#[test]
fn a_dilemma_learn_effect_and_its_scene_teach() {
    let c = content();
    let mut e = game(41, &c);
    let (steward, owner, hood) = strangers(&e);
    let id = inject(
        &mut e,
        "dilemma.plant_layoffs",
        vec![
            CastEntry {
                role: "steward".into(),
                actor: Actor::Person { id: steward },
            },
            CastEntry {
                role: "owner".into(),
                actor: Actor::Person { id: owner },
            },
            CastEntry {
                role: "hood".into(),
                actor: Actor::Neighborhood { id: hood.0 },
            },
        ],
    );
    let town = e.state().player.as_ref().unwrap().municipality;
    assert_eq!(level(&e, &Subject::Broker { town }), Level::Unknown);
    e.apply(Command::Player(PlayerCommand::Answer {
        decision: id.0,
        option: "broker_talks".into(),
    }))
    .unwrap();
    // The option's `learn` effects: both sides in detail.
    for p in [steward, owner] {
        let s = Subject::Person { person: p };
        assert_eq!(level(&e, &s), Level::Detailed);
        assert!(
            ledger(&e)
                .entry(&s)
                .unwrap()
                .learned
                .iter()
                .all(|l| l.source == Source::Decision { decision: id })
        );
    }
    // The scene's hooks: the broker heard of, the neighbourhood heard out.
    assert_eq!(level(&e, &Subject::Broker { town }), Level::Heard);
    assert!(level(&e, &Subject::HoodConcerns { hood }) >= Level::Known);
    let d = e.state().narrative.decision(id).unwrap();
    let applied = &d.resolution.as_ref().unwrap().applied;
    assert_eq!(
        applied
            .iter()
            .filter(|a| matches!(a, narrative::Applied::Learned { subject, n: 1, .. } if subject == "person"))
            .count(),
        2
    );
}

#[test]
fn answering_meets_the_cast_and_letting_it_pass_does_not() {
    let c = content();
    let mut e = game(42, &c);
    let (a, b, hood) = strangers(&e);
    let cast = |x: CharacterId, y: CharacterId| {
        vec![
            CastEntry {
                role: "steward".into(),
                actor: Actor::Person { id: x },
            },
            CastEntry {
                role: "owner".into(),
                actor: Actor::Person { id: y },
            },
            CastEntry {
                role: "hood".into(),
                actor: Actor::Neighborhood { id: hood.0 },
            },
        ]
    };
    let id = inject(&mut e, "dilemma.plant_layoffs", cast(a, b));
    e.apply(Command::Player(PlayerCommand::Answer {
        decision: id.0,
        option: "back_owner".into(),
    }))
    .unwrap();
    assert_eq!(level(&e, &Subject::Person { person: a }), Level::Known);
    assert_eq!(level(&e, &Subject::Person { person: b }), Level::Known);
    // The default (the player let it pass) meets nobody.
    let mut f = game(42, &c);
    let id = inject(&mut f, "dilemma.plant_layoffs", cast(a, b));
    let before = ledger(&f).clone();
    let mut ok = false;
    f.setup(|st| {
        ok = mandate_sim::narrative::answer(st, &c, id, "stay_out", true).is_ok();
    });
    assert!(ok);
    assert_eq!(ledger(&f), &before);
}

#[test]
fn the_knows_condition_gates() {
    let c = content();
    let mut e = game(43, &c);
    let (p, _, hood) = strangers(&e);
    let cast = vec![
        CastEntry {
            role: "who".into(),
            actor: Actor::Person { id: p },
        },
        CastEntry {
            role: "hood".into(),
            actor: Actor::Neighborhood { id: hood.0 },
        },
    ];
    let holds = |e: &Engine, cond: &Condition| {
        let st = e.state();
        let r = mandate_sim::scheduler::Reads::of_state(st, &c);
        let v = mandate_sim::narrative::cast::View::new(&r, &st.politics);
        mandate_sim::narrative::cast::holds(&v, cond, &cast)
    };
    let knows = |subject: SubjectDef, min: Level| Condition::Knows { subject, min };
    let person = knows(SubjectDef::Person { role: "who".into() }, Level::Known);
    let broker = knows(SubjectDef::Broker, Level::Heard);
    let ward = knows(
        SubjectDef::HoodConcerns {
            role: Some("hood".into()),
        },
        Level::Known,
    );
    let missing = knows(
        SubjectDef::Person {
            role: "nobody".into(),
        },
        Level::Heard,
    );
    assert!(!holds(&e, &person));
    assert!(!holds(&e, &broker));
    assert!(holds(&e, &ward), "the sitting councillor knows the ward");
    assert!(!holds(&e, &missing));
    let town = e.state().player.as_ref().unwrap().municipality;
    e.setup(|st| {
        knowledge::teach_one(st, Subject::Person { person: p }, Level::Heard, Source::Lab);
        knowledge::teach_one(st, Subject::Broker { town }, Level::Heard, Source::Lab);
    });
    assert!(!holds(&e, &person), "heard is not known");
    assert!(holds(&e, &broker));
    e.setup(|st| {
        knowledge::teach_one(st, Subject::Person { person: p }, Level::Known, Source::Lab);
    });
    assert!(holds(&e, &person));
    // The lab switch reads everything as known; a role not cast never holds.
    let mut f = game(43, &c);
    f.set_lab_omniscient(true);
    assert!(holds(&f, &person));
    assert!(!holds(&f, &missing));
    // The condition parses from content with `heard` as the default level.
    let parsed: Condition =
        serde_json::from_str(r#"{ "type": "knows", "subject": { "type": "broker" } }"#).unwrap();
    assert_eq!(parsed, knows(SubjectDef::Broker, Level::Heard));
}

/// Base content plus a test pack: a dilemma whose trigger is a `knows`
/// condition and whose options `reveal_place` and `learn`.
fn content_with_tip() -> Arc<ContentBundle> {
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    src.push(mandate_content::Source {
        path: "zz_test/dilemmas.jsonc".into(),
        text: r#"{"kind":"dilemmas","items":[{"id":"dilemma.test_tip","category":"significant",
          "title_key":"test-tip","situation_key":"test-tip",
          "roles":[{"id":"hood","cast":{"type":"ward_neighborhood"}}],
          "triggers":[{"type":"knows","subject":{"type":"broker"},"min":"known"}],
          "cooldown_days":30,
          "options":[
            {"id":"follow","label_key":"test-tip","aftermath_key":"test-tip","effects":[
              {"type":"health","delta":-1},
              {"type":"reveal_place","role":"hood"},
              {"type":"learn","subject":{"type":"broker"},"level":"detailed"}]},
            {"id":"skip","default":true,"label_key":"test-tip","aftermath_key":"test-tip","effects":[
              {"type":"health","delta":-1},
              {"type":"learn","subject":{"type":"town_concerns"},"level":"heard"}]}]}]}"#
            .into(),
    });
    src.push(mandate_content::Source {
        path: "zz_test/locale/en/test.ftl".into(),
        text: "test-tip = A tip\n".into(),
    });
    let (b, issues) = load(&src).unwrap();
    assert!(
        issues
            .iter()
            .all(|i| i.severity != mandate_content::Severity::Error),
        "{issues:?}"
    );
    Arc::new(b)
}

#[test]
fn reveal_place_and_learn_effects_from_content() {
    let c = content_with_tip();
    let mut e = game(44, &c);
    let town = e.state().player.as_ref().unwrap().municipality;
    // A neighbourhood with places in it.
    let hood = {
        let st = e.state();
        st.world
            .places
            .iter()
            .find(|p| p.municipality == town)
            .unwrap()
            .neighborhood
    };
    let in_hood: Vec<PlaceId> = e
        .state()
        .world
        .places
        .iter()
        .filter(|p| p.neighborhood == hood)
        .map(|p| p.id)
        .collect();
    let id = inject(
        &mut e,
        "dilemma.test_tip",
        vec![CastEntry {
            role: "hood".into(),
            actor: Actor::Neighborhood { id: hood.0 },
        }],
    );
    e.apply(Command::Player(PlayerCommand::Answer {
        decision: id.0,
        option: "follow".into(),
    }))
    .unwrap();
    for place in &in_hood {
        assert_eq!(
            level(&e, &Subject::Place { place: *place }),
            Level::Known,
            "revealed"
        );
    }
    // Places elsewhere are not.
    assert!(
        e.state()
            .world
            .places
            .iter()
            .filter(|p| p.municipality == town && p.neighborhood != hood)
            .all(|p| level(&e, &Subject::Place { place: p.id }) == Level::Unknown)
    );
    let b = ledger(&e).entry(&Subject::Broker { town }).unwrap();
    assert_eq!(b.level, Level::Detailed);
    assert_eq!(b.learned.len(), 3, "each level is recorded on the way up");
    let d = e.state().narrative.decision(id).unwrap();
    let applied = &d.resolution.as_ref().unwrap().applied;
    assert!(applied.iter().any(|a| matches!(
        a,
        narrative::Applied::Learned { subject, n, .. } if subject == "place" && *n as usize == in_hood.len()
    )));
    // Meeting someone is knowing them (the `Meet` command).
    let mut f = game(44, &c);
    let (place, slot) = town_places(&f)
        .into_iter()
        .find_map(|(p, _)| {
            let loc = Location::Place { place: p };
            mandate_sim::characters::ambient::ambient_at(f.state(), &c, loc)
                .first()
                .map(|(s, _)| (p, *s))
        })
        .expect("someone to meet somewhere");
    move_to(&mut f, place);
    let out = f
        .apply(Command::Player(PlayerCommand::Meet { slot }))
        .unwrap();
    let met = CharacterId(out.result.unwrap().created.unwrap());
    let s = Subject::Person { person: met };
    assert_eq!(level(&f, &s), Level::Known);
    assert_eq!(
        ledger(&f).entry(&s).unwrap().learned[1].source,
        Source::Met { person: met }
    );
}

/// Same seed and commands: the same ledger; save, load and continue
/// matches; and learning changes nothing but the ledger (the lab switch,
/// which reads everything as known, gives the same world).
#[test]
fn learning_is_deterministic_and_changes_nothing_else() {
    let c = content();
    let script = |e: &mut Engine, turns: u32| {
        for t in 0..turns {
            let places = town_places(e);
            let (p, _) = places[(t as usize * 7) % places.len()];
            let _ = e.apply(Command::Player(PlayerCommand::MoveTo {
                location: Location::Place { place: p },
            }));
            let st = e.state();
            let pl = st.player.as_ref().unwrap();
            let hoods: Vec<u32> = st
                .world
                .hoods_of(pl.municipality)
                .map(|h| h as u32)
                .collect();
            let h = hoods[t as usize % hoods.len()];
            let cmd = e.route(PlayerCommand::Act {
                action: Action::Canvass { neighborhood: h },
                late_night: false,
            });
            let _ = e.apply(cmd);
            let pending: Vec<_> = e.state().narrative.pending().cloned().collect();
            for d in pending {
                let def = c.dilemma(&d.template).unwrap();
                if let Some(o) = def.options.iter().find(|o| !o.default) {
                    let _ = e.apply(Command::Player(PlayerCommand::Answer {
                        decision: d.id.0,
                        option: o.id.clone(),
                    }));
                }
            }
            e.apply(Command::EndTurn).unwrap();
        }
    };
    let mut a = game(45, &c);
    let mut b = game(45, &c);
    let mut lab = game(45, &c);
    lab.set_lab_omniscient(true);
    script(&mut a, 12);
    script(&mut b, 12);
    script(&mut lab, 12);
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(ledger(&a), ledger(&b));
    // Learning happened, from more than one kind of source.
    let kinds: std::collections::BTreeSet<&str> = ledger(&a)
        .entries
        .values()
        .flat_map(|x| x.learned.iter())
        .map(|l| l.source.key())
        .collect();
    assert!(kinds.contains("visit"), "{kinds:?}");
    assert!(kinds.contains("action"), "{kinds:?}");
    let strip = |e: &Engine| {
        let mut v = serde_json::to_value(e.state()).unwrap();
        v.pointer_mut("/player")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("knowledge");
        v
    };
    assert_eq!(
        strip(&a),
        strip(&lab),
        "the ledger is not read for outcomes"
    );
    let mut lab_ledger = ledger(&lab).clone();
    lab_ledger.lab_omniscient = false;
    assert_eq!(
        &lab_ledger,
        ledger(&a),
        "the lab switch writes the same ledger"
    );
    // Save, load, continue.
    let req = SaveRequest {
        run_id: "00112233445566778899aabbccddeeff".into(),
        revision: 1,
        parent_revision: None,
        device_id: "test-device".into(),
        saved_at: "2026-01-01T00:00:00Z".into(),
        protocol_version: 1,
        run_mode: RunMode::Ironman,
    };
    let (bytes, _) = encode(b.state(), &req).unwrap();
    let (state, _) = decode(&bytes).unwrap();
    let mut b2 = Engine::from_state(state, c.clone()).unwrap();
    script(&mut a, 6);
    script(&mut b2, 6);
    assert_eq!(a.state_hash(), b2.state_hash());
}

/// Stories and decisions teach through effect requests: over a few seeds,
/// people named in the paper or in a decision are heard of.
#[test]
fn stories_and_decisions_teach_through_effects() {
    let c = content();
    let (mut story, mut decision) = (false, false);
    for seed in 46..52u64 {
        let mut e = game(seed, &c);
        for _ in 0..40 {
            e.apply(Command::EndTurn).unwrap();
        }
        for x in ledger(&e).entries.values() {
            for l in &x.learned {
                match l.source {
                    Source::Story { story: id } => {
                        assert!(e.state().media.stories.iter().any(|s| s.id == id));
                        story = true;
                    }
                    Source::Decision { decision: id } => {
                        assert!(e.state().narrative.decision(id).is_some());
                        decision = true;
                    }
                    _ => {}
                }
            }
        }
        if story && decision {
            break;
        }
    }
    assert!(story, "a story named someone");
    assert!(decision, "a decision named someone");
}
