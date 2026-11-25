#![allow(clippy::unwrap_used, clippy::expect_used)]
//! The content workbench's explanations (`mandate_sim::explain`) agree with
//! the engine's own evaluators: for every dilemma at several seeds and
//! dates, the explained verdict on casting and triggers is the director's
//! (a cast search over the same roles and triggers), and every condition
//! node's verdict is the evaluator's; situational policies, overheard lines
//! and conversation lines likewise. Explaining never changes the state.

use mandate_content::{ContentBundle, DilemmaCategory, bundle::read_dir_sources, load};
use mandate_sim::explain::{self, Focus, ItemKind, Node};
use mandate_sim::narrative::cast::{self, View};
use mandate_sim::scheduler::Reads;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn base() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn child<'a>(n: &'a Node, label: &str) -> &'a Node {
    n.children
        .iter()
        .find(|c| c.label == label)
        .unwrap_or_else(|| panic!("no `{label}` under `{}`", n.label))
}

/// The director's verdict on casting and triggers, through the public
/// casting API (`director::evaluate` is a cast search with this test).
fn castable(e: &Engine, id: &str) -> bool {
    let st = e.state();
    let content = e.content();
    let def = content.dilemma(id).unwrap();
    let r = Reads::of_state(st, content);
    let v = View::new(&r, &st.politics);
    let mut preset = Vec::new();
    // An arc's decision beat is cast from its run.
    if let Some(run) = st.arcs.runs.iter().find(|run| {
        content
            .arc(&run.template)
            .and_then(|a| a.stage(&run.stage))
            .is_some_and(|s| {
                s.beats.iter().any(|b| {
                    matches!(&b.kind, mandate_content::arcs::BeatKind::Decision { dilemma, .. }
                        if dilemma == id)
                })
            })
    }) {
        preset = run.narrative_cast();
    }
    let mut holders: Vec<Option<mandate_sim::characters::CharacterId>> = vec![None];
    if def.category == DilemmaCategory::Request {
        let hs: Vec<_> = st
            .politics
            .characters
            .goals
            .iter()
            .filter(|g| {
                content
                    .goal(&g.goal)
                    .is_some_and(|x| x.request.as_deref() == Some(id))
            })
            .map(|g| Some(g.holder))
            .collect();
        if !hs.is_empty() {
            holders = hs;
        }
    }
    holders.into_iter().any(|h| {
        let ok = |c: &[mandate_sim::narrative::CastEntry]| {
            def.triggers.iter().all(|x| cast::holds(&v, x, c))
        };
        cast::cast_search(&v, &def.roles, &preset, h, 6, &ok).is_some()
    })
}

#[test]
fn explanations_match_the_evaluators_for_every_dilemma() {
    let c = base();
    let (mut checked, mut castable_seen, mut blocked_seen) = (0, 0, 0);
    for seed in [3u64, 11, 29] {
        let mut e = generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap();
        for step in 0..6 {
            if step > 0 {
                e.apply(Command::AdvanceDays { days: 91 }).unwrap();
            }
            let before = e.state_hash();
            for d in &c.dilemmas {
                let x = explain::explain(e.state(), &c, ItemKind::Dilemma, &d.id, Focus::default())
                    .unwrap();
                let ct = child(&x.tree, "cast and triggers");
                let expected = castable(&e, &d.id);
                assert_eq!(
                    ct.ok, expected,
                    "seed {seed} step {step}: {} explained castable={} but the director's search says {expected}",
                    d.id, ct.ok
                );
                // The verdict follows from the nodes: castable means every
                // role filled and every trigger holding for that cast.
                let roles = child(ct, "cast");
                let triggers = child(ct, "triggers");
                if expected {
                    assert!(
                        roles.ok && triggers.ok,
                        "{}: castable but a node fails",
                        d.id
                    );
                    assert!(!x.forced);
                    castable_seen += 1;
                } else {
                    assert!(
                        !(roles.ok && triggers.ok),
                        "{}: blocked but no node fails",
                        d.id
                    );
                    blocked_seen += 1;
                }
                // Each trigger node is the evaluator's verdict for the cast shown.
                let r = Reads::of_state(e.state(), &c);
                let v = View::new(&r, &e.state().politics);
                for (t, n) in d.triggers.iter().zip(&triggers.children) {
                    assert_eq!(n.ok, cast::holds(&v, t, &x.cast), "{}: {}", d.id, n.label);
                }
                // Could fire = gates and casting.
                let gates = child(&x.tree, "gates");
                assert_eq!(x.could_fire, gates.ok && ct.ok, "{}", d.id);
                checked += 1;
            }
            assert_eq!(e.state_hash(), before, "explaining changed the state");
        }
    }
    assert!(checked > 100);
    // Both verdicts occur, so the comparison means something.
    assert!(
        castable_seen > 0 && blocked_seen > 0,
        "{castable_seen} {blocked_seen}"
    );
}

#[test]
fn explanations_match_the_evaluators_for_policies_lines_and_talk() {
    let c = base();
    let mut e = generate(&WorldConfig::new(7, WorldSize::Small), c.clone()).unwrap();
    for _ in 0..20 {
        e.apply(Command::EndTurn).unwrap();
    }
    let st = e.state();
    let r = Reads::of_state(st, &c);
    let v = View::new(&r, &st.politics);
    let town = v.town().unwrap();
    for t in c.policies.values().filter(|p| p.unlock.is_some()) {
        let x = explain::explain(st, &c, ItemKind::Situational, &t.id, Focus::default()).unwrap();
        assert_eq!(
            x.could_fire,
            mandate_sim::policy::unlocked(st, &c, t, town),
            "{}",
            t.id
        );
    }
    let places = explain::talk_places(st, &c);
    assert!(!places.is_empty());
    for o in &c.overheard {
        let x = explain::explain(st, &c, ItemKind::Overheard, &o.id, Focus::default()).unwrap();
        let holds = places.iter().any(|p| {
            mandate_sim::overheard::available(
                st,
                &c,
                mandate_sim::places::Location::Place { place: *p },
            )
            .iter()
            .any(|l| l.id == o.id)
        });
        assert_eq!(x.could_fire, holds, "{}", o.id);
    }
    // A line for a speaker: its `when` node is the conversation's own reading.
    let speakers = explain::speakers(st, &c);
    assert!(!speakers.is_empty());
    let mut lines_ok = 0;
    for l in &c.talk_lines {
        let x = explain::explain(
            st,
            &c,
            ItemKind::Line,
            &l.id,
            Focus {
                speaker: Some(speakers[0]),
                place: None,
            },
        )
        .unwrap();
        assert_eq!(x.speaker, Some(speakers[0]));
        // The subject it was explained with is one the topic takes.
        if let Some(a) = &x.about {
            let t = l.topic.as_deref().and_then(|k| c.talk_topic(k)).unwrap();
            assert!(t.about.contains(&a.kind()), "{}", l.id);
        }
        let ctx = explain::talk_ctx(st, &c, speakers[0], x.about.as_ref()).unwrap();
        let speaker = x.tree.children[0]
            .children
            .iter()
            .find(|n| n.kind == "speaker")
            .unwrap();
        let when = child(speaker, "when");
        assert_eq!(
            when.ok,
            ctx.all_hold(&l.when, &mut Default::default()),
            "{}",
            l.id
        );
        if x.could_fire {
            lines_ok += 1;
        }
    }
    assert!(lines_ok > 0);
}
