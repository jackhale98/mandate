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
    // arc files in name order): whatever comes after the flood's is an arc
    // file's too (an arc's own decision, an ask only a run offers), so a new
    // arc file changes nothing here.
    let d = b
        .dilemmas
        .iter()
        .position(|x| x.id == "dilemma.flood_money")
        .unwrap();
    assert!(
        b.dilemmas[d..].iter().all(|x| x.arc_only),
        "{:?}",
        &b.dilemmas[d..]
    );
    let k = b
        .asks
        .iter()
        .position(|x| x.id == "ask.flood_shelter")
        .unwrap();
    assert!(
        b.asks[k..]
            .iter()
            .all(|x| x.from == [mandate_content::asks::AskFrom::Arc]),
        "{:?}",
        b.asks[k..].iter().map(|x| &x.id).collect::<Vec<_>>()
    );
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
            { "when": [ { "type": "any", "of": [ { "type": "arc_mark", "mark": "meeting_won" }, { "type": "arc_mark", "mark": "meeting_split" } ] } ],
              "type": "approval", "area": "hood", "delta": 0.01 },
            { "when": [ { "type": "arc_mark", "mark": "tabled" } ], "type": "record", "delta": 0.08 },
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
        r#""id": "date", "type": "talk", "with": "organiser", "sets": "dated","#,
        r#""id": "date", "type": "talk", "with": "organiser", "sets": "a_date","#,
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
        r#""now": { "sets": "paid_first","#,
        r#""soon": { "sets": "paid_first","#,
        "has no option `soon`",
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

/// The arcs' infrastructure (2026-11-26): a scene's clock shorter than a
/// turn is a warning (the engine holds it to a turn); an `arc_scene` names
/// a scene of its own arc; a standing arc takes no rank; a crisis is ranked
/// in content, never by its id.
#[test]
fn scene_windows_reads_of_scenes_and_ranks_are_checked() {
    // The flood's meeting on a week's clock: it loads, with the warning.
    let mut src = sources();
    let f = src.iter_mut().find(|s| s.path.ends_with(FLOOD)).unwrap();
    let from = r#""climax": true, "clock": { "days": [14, 21] }"#;
    assert!(f.text.contains(from));
    f.text = f
        .text
        .replacen(from, r#""climax": true, "clock": { "days": [7, 14] }"#, 1);
    let (_, warnings) = load(&src).unwrap();
    let w: Vec<String> = warnings.iter().map(|x| x.to_string()).collect();
    assert!(
        w.iter()
            .any(|x| x.contains("exchange-window-short") && x.contains("`meeting` (7 days)")),
        "{w:#?}"
    );
    // A read of a scene the arc does not have.
    refused(
        FLOOD,
        r#""beat": "meeting.room", "outcomes": ["won", "split"]"#,
        r#""beat": "meeting.nowhere", "outcomes": ["won", "split"]"#,
        "`meeting.nowhere` is not a scene of this arc",
    );
    // A standing arc holds no place.
    refused(
        "arcs/rival.jsonc",
        r#""standing": "campaign","#,
        r#""standing": "campaign", "rank": "crisis","#,
        "it takes no `rank`",
    );
    let (b, _) = load(&sources()).unwrap();
    use mandate_content::arcs::ArcRank;
    for (id, rank) in [
        ("arc.flood", ArcRank::Crisis),
        ("arc.plant_closure", ArcRank::Crisis),
        ("arc.zoning", ArcRank::Matter),
        ("arc.favour_debt", ArcRank::Matter),
    ] {
        assert_eq!(b.arc(id).unwrap().rank, rank, "{id}");
    }
    // Every scene of the pack but the zoning meeting reaches its ending
    // (`arc-scene-unread`: the zoning meeting's exits read no outcome).
    let unread = mandate_content::arcs::unread_scenes(&b);
    assert_eq!(unread, vec![("arc.zoning", "meeting.room".to_string())]);
}
