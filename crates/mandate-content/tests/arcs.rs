#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Staged arcs as content (DEC-109, task A1): the base pack's flood arc
//! loads, and validation refuses what the design forbids, each with its own
//! fixture (the flood arc with one thing broken): a stage that cannot be
//! reached, an exit without effects, a missing text key, a stage that does
//! not end on time, a mark read and never set, a role nobody is cast in,
//! an argument the arc does not provide, the arc conditions outside an arc.
use mandate_content::arcs::{BeatKind, inert, not_yet};
use mandate_content::condition::ConditionUse;
use mandate_content::{Condition, Source, bundle::read_dir_sources, load};
use std::path::Path;

fn sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

/// Errors of a load with the flood arc's file or its text edited.
fn errors(path: &str, from: &str, to: &str) -> Vec<String> {
    let mut src = sources();
    let f = src.iter_mut().find(|s| s.path.ends_with(path)).unwrap();
    assert!(f.text.contains(from), "{path} has no `{from}`");
    f.text = f.text.replacen(from, to, 1);
    match load(&src) {
        Ok(_) => Vec::new(),
        Err(i) => i.into_iter().map(|x| x.to_string()).collect(),
    }
}

fn refused(path: &str, from: &str, to: &str, why: &str) {
    let e = errors(path, from, to);
    assert!(e.iter().any(|x| x.contains(why)), "`{why}` not in {e:#?}");
}

const FLOOD: &str = "arcs/flood.jsonc";
const TEXT: &str = "locale/en/arcs.ftl";

#[test]
fn the_flood_arc_loads_and_uses_only_what_is_built() {
    let (b, warnings) = load(&sources()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let a = b.arc("arc.flood").unwrap();
    assert!(not_yet(a).is_empty(), "{:?}", not_yet(a));
    // From *struck*: three exits, a turn, and all eight beat kinds: the
    // turn is a scene (DEC-112), the meeting.
    assert_eq!(a.stages[0].id, "struck");
    assert_eq!(a.stages.iter().filter(|s| s.exit.is_some()).count(), 3);
    assert_eq!(a.stages.iter().filter(|s| s.climax).count(), 1);
    let kinds: std::collections::BTreeSet<&str> = a.beats().map(|(_, b)| b.kind.key()).collect();
    assert_eq!(
        kinds.into_iter().collect::<Vec<_>>(),
        [
            "ask", "decision", "exchange", "heard", "move", "press", "talk", "visit"
        ]
    );
    // What it carries joined the registries, after every other file's (the
    // arc files in name order: the plant's overheard line comes after).
    assert_eq!(b.dilemmas.last().unwrap().id, "dilemma.flood_money");
    assert_eq!(b.asks.last().unwrap().id, "ask.flood_shelter");
    assert!(b.overheard.iter().any(|o| o.id == "overheard.flood.skips"));
    assert!(b.scene("scene.flood_hall").is_some());
    // The meeting's decision is retired: the scene replaced it.
    assert!(b.dilemma("dilemma.flood_meeting").is_none());
    // Every decision the flood presents is its own (never spotlit alone).
    for (_, beat) in a.beats() {
        if let BeatKind::Decision { dilemma, .. } = &beat.kind {
            assert!(b.dilemma(dilemma).unwrap().arc_only);
        }
    }
    // What is parsed for later tasks is said to be; the turn (`climax`) is
    // the director's since task A2, and is no longer among it.
    let later = inert(a);
    assert!(!later.iter().any(|x| x.contains("`climax`")), "{later:?}");
    assert!(later.iter().any(|x| x.contains("`sides`")), "{later:?}");
    assert!(later.iter().any(|x| x.contains("`answers`")), "{later:?}");
}

#[test]
fn the_graph_is_checked() {
    // Stages no edge leads to (the first stage skips past them).
    refused(
        FLOOD,
        r#"{ "to": "counting", "clock_within_days": 0 }"#,
        r#"{ "to": "rebuilding", "clock_within_days": 0 }"#,
        "stage `counting` cannot be reached from the first (arc-unreachable-stage)",
    );
    // An exit without effects.
    refused(
        FLOOD,
        r#"{ "type": "approval", "area": "hood", "delta": 0.03 },
            { "when": [ { "type": "arc_mark", "mark": "wall" } ], "type": "record", "delta": 0.08 },
            { "when": [ { "type": "arc_mark", "mark": "at_hall" } ], "type": "opinion", "role": "organiser", "delta": 6 } ] } },"#,
        "] } },",
        "arc-exit-no-effects",
    );
    // A stage whose last edge waits on the world: it could last for ever.
    refused(
        FLOOD,
        r#"{ "to": "hollowed", "after_days": 120 } ] },"#,
        r#"{ "to": "hollowed", "when": [ { "type": "arc_heat", "min": 0.9 } ] } ] },"#,
        "arc-stuck",
    );
    // An edge on the clock where no clock is in force.
    refused(
        FLOOD,
        r#""key": "arc-flood-struck", "clock": { "days": [21, 28] },"#,
        r#""key": "arc-flood-struck","#,
        "no clock is in force",
    );
    // An edge to a stage that does not exist.
    refused(
        FLOOD,
        r#"{ "to": "counting", "clock_within_days": 0 }"#,
        r#"{ "to": "nowhere", "clock_within_days": 0 }"#,
        "unknown stage `nowhere`",
    );
}

#[test]
fn marks_roles_and_text_are_checked() {
    // A mark an edge reads and nothing sets.
    refused(
        FLOOD,
        r#""date": { "sets": "dated","#,
        r#""date": { "sets": "promised","#,
        "mark `dated` is read and never set",
    );
    // A beat that names a role the arc lacks.
    refused(
        FLOOD,
        r#""type": "visit", "at": "hall""#,
        r#""type": "visit", "at": "chapel""#,
        "arc-role-uncast",
    );
    // A role of the wrong kind: nobody talks to a neighbourhood.
    refused(
        FLOOD,
        r#""type": "talk", "with": "official""#,
        r#""type": "talk", "with": "hood""#,
        "must be a person",
    );
    // A missing text key.
    refused(
        TEXT,
        "arc-flood-counting = ",
        "arc-flood-counting-gone = ",
        "missing Fluent key `arc-flood-counting`",
    );
    // An argument the arc does not provide.
    refused(
        TEXT,
        "Work has started in { $hood }.",
        "Work has started in { $suburb }.",
        "uses `$suburb`, which this arc does not provide",
    );
    // A visit's button takes no arguments (the place lists it bare).
    refused(
        TEXT,
        "visit-arc-flood-works = Walk the repairs",
        "visit-arc-flood-works = Walk the repairs in { $hood }",
        "is a button",
    );
    // A person's role word only where the view renders the text.
    refused(
        TEXT,
        "You sorted donations at { $hall } for an afternoon. { $organiser } gave",
        "You sorted donations at { $hall } for an afternoon. { $organiser } ({ $organiser_role }) gave",
        "uses `$organiser_role`",
    );
    // A decision whose option the dilemma lacks; an ask that is not the arc's.
    refused(
        FLOOD,
        r#""honest": { "heat": -0.15 },"#,
        r#""frank": { "heat": -0.15 },"#,
        "has no option `frank`",
    );
    refused(
        FLOOD,
        r#""name_key": "ask-flood-shelter", "from": ["arc"],"#,
        r#""name_key": "ask-flood-shelter","#,
        r#"must come `from: ["arc"]` only"#,
    );
}

#[test]
fn the_arc_conditions_belong_to_arcs() {
    let mark: Condition = serde_json::from_str(r#"{ "type": "arc_mark", "mark": "x" }"#).unwrap();
    let away: Condition =
        serde_json::from_str(r#"{ "type": "displaced", "slot": "hood", "min": 0.1 }"#).unwrap();
    for u in ConditionUse::ALL {
        let inside = matches!(
            u,
            ConditionUse::ArcEdge
                | ConditionUse::ArcGate
                | ConditionUse::ArcFacts
                | ConditionUse::ArcReply
        );
        assert_eq!(u.refuses(&mark).is_none(), inside, "{u:?}");
        assert_eq!(u.refuses(&away).is_none(), inside, "{u:?}");
    }
    // The ledger decides nothing in an arc (STAGED_ARCS §2).
    let knows: Condition =
        serde_json::from_str(r#"{ "type": "knows", "subject": { "type": "broker" } }"#).unwrap();
    for u in [
        ConditionUse::ArcRaise,
        ConditionUse::ArcEdge,
        ConditionUse::ArcGate,
        ConditionUse::ArcFacts,
        ConditionUse::ArcReply,
    ] {
        assert!(u.refuses(&knows).is_some(), "{u:?}");
    }
    // Through the pipeline: a run's mark in a dilemma's triggers; another
    // run read from inside an arc; a local-only condition in an arc that
    // lists the regional tier.
    refused(
        "dilemmas.jsonc",
        r#""triggers": ["#,
        r#""triggers": [ { "type": "arc_mark", "mark": "x" }, "#,
        "reads an arc run",
    );
    refused(
        FLOOD,
        r#""raise": [ { "type": "disaster", "hazard": "flood", "within_months": 1 } ],"#,
        r#""raise": [ { "type": "disaster", "hazard": "flood", "within_months": 1 }, { "type": "arc", "arc": "arc.flood" } ],"#,
        "an `arc` condition inside an arc",
    );
    refused(
        FLOOD,
        r#""tiers": ["local"]"#,
        r#""tiers": ["local", "regional"]"#,
        "DEC-098",
    );
}
