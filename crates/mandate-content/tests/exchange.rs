#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Exchange scenes as content (DEC-112, task X1; EXCHANGE_SCENES §9.4):
//! the base pack's debate loads, and each static lint has a fixture that
//! fails it (the rival arc with one thing broken).
use mandate_content::arcs::{BeatKind, not_yet};
use mandate_content::exchange::{MoveKind, Outcome, SourceKind};
use mandate_content::{Source, bundle::read_dir_sources, load};
use std::path::Path;

fn sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn edited(path: &str, from: &str, to: &str) -> Vec<Source> {
    let mut src = sources();
    let f = src.iter_mut().find(|s| s.path.ends_with(path)).unwrap();
    assert!(f.text.contains(from), "{path} has no `{from}`");
    f.text = f.text.replacen(from, to, 1);
    src
}

fn refused(path: &str, from: &str, to: &str, why: &str) {
    let e: Vec<String> = match load(&edited(path, from, to)) {
        Ok(_) => Vec::new(),
        Err(i) => i.into_iter().map(|x| x.to_string()).collect(),
    };
    assert!(e.iter().any(|x| x.contains(why)), "`{why}` not in {e:#?}");
}

const RIVAL: &str = "arcs/rival.jsonc";
const TEXT: &str = "locale/en/exchange.ftl";
const DEBATE: &str = r#"{ "id": "debate", "type": "exchange", "once": true,"#;

#[test]
fn the_debate_loads_and_uses_only_what_is_built() {
    let (b, _) = load(&sources()).unwrap();
    let a = b.arc("arc.rival").unwrap();
    assert!(not_yet(a).is_empty(), "{:?}", not_yet(a));
    let stage = a.stage("showdown").unwrap();
    assert!(stage.climax && stage.clock.is_some());
    let BeatKind::Exchange(x) = &stage.beat("debate").unwrap().kind else {
        panic!("the showdown's turn is an exchange");
    };
    assert_eq!(x.rounds(&b.exchange), 3);
    assert!(x.moves.contains(&MoveKind::Pass));
    assert_eq!(x.outcome.len(), 4);
    assert_eq!(
        x.outcome[&Outcome::Absent].sets.as_deref(),
        Some("debate_ducked")
    );
    // Every round ends in a source that always gives a prompt.
    for r in &x.prompts {
        assert!(r.from.last().unwrap().always(), "round {}", r.round);
        assert!(r.from.iter().all(|s| s.built()));
    }
    // The showdown is no longer a decision: nothing asks yes or no.
    assert!(
        !stage
            .beats
            .iter()
            .any(|b| matches!(b.kind, BeatKind::Decision { .. })),
        "the debate replaces the accept or decline decision"
    );
    // The constants are the design's first values, stated in the pack.
    let p = &b.exchange;
    assert_eq!((p.bite, p.gain, p.cold, p.spill), (0.30, 0.25, 1.25, 0.3));
    assert_eq!((p.win, p.lose, p.group_margin), (0.08, 0.08, 0.05));
    assert_eq!((p.world.approve, p.world.aware), (0.08, 0.04));
    assert_eq!(p, &{
        // The pack states the code's defaults again, and adds its tables.
        let mut d = mandate_content::exchange::ExchangeParams {
            worries: p.worries.clone(),
            ..Default::default()
        };
        d.deck.grudge_traits = p.deck.grudge_traits.clone();
        d.member_fit.integrity = p.member_fit.integrity.clone();
        d.member_fit.loyalty = p.member_fit.loyalty.clone();
        d
    });
}

/// `arc-turn-single-decision` (a warning until the arcs are rewritten):
/// the base pack's turns that are still one decision, by name.
#[test]
fn a_turn_that_is_one_decision_is_flagged() {
    let (b, warnings) = load(&sources()).unwrap();
    // The pack's own warnings stay at none; the lint is reported beside
    // them (`mandate-cli content validate`).
    assert!(warnings.is_empty(), "{warnings:?}");
    let mut flagged: Vec<String> = mandate_content::arcs::lints(&b)
        .iter()
        .map(|w| w.to_string())
        .filter(|w| w.starts_with("warning") && w.contains("arc-turn-single-decision"))
        .collect();
    flagged.sort();
    let stages: Vec<(&str, &str)> = vec![
        ("arc.budget_round", "turn_deal"),
        ("arc.budget_round", "turn_hole"),
        ("arc.budget_round", "turn_line"),
        ("arc.budget_round", "turn_surplus"),
        ("arc.favour_debt", "called_in"),
        ("arc.flood", "meeting"),
        ("arc.plant_closure", "organising"),
    ];
    assert_eq!(flagged.len(), stages.len(), "{flagged:#?}");
    for (arc, stage) in stages {
        assert!(
            flagged
                .iter()
                .any(|w| w.contains(arc) && w.contains(&format!("stage `{stage}`"))),
            "{arc} {stage}: {flagged:#?}"
        );
    }
    // The rival's turn is played: not flagged.
    assert!(!flagged.iter().any(|w| w.contains("arc.rival")));
}

#[test]
fn a_scene_needs_a_safe_move_a_fallback_a_clock_and_a_turn() {
    refused(
        RIVAL,
        r#""own", "second", "pass"]"#,
        r#""own", "second"]"#,
        "exchange-no-safe-move",
    );
    refused(
        RIVAL,
        r#"{ "round": 3, "by": "chair", "from": ["top_worry"] }"#,
        r#"{ "round": 3, "by": "chair", "from": ["thin_record"] }"#,
        "exchange-no-fallback",
    );
    refused(
        RIVAL,
        r#""clock": { "days": [14, 21] },"#,
        "",
        "exchange-no-clock",
    );
    refused(
        RIVAL,
        r#""key": "arc-rival-showdown", "climax": true,"#,
        r#""key": "arc-rival-showdown","#,
        "exchange-not-at-turn",
    );
    // A round nobody wrote.
    refused(
        RIVAL,
        r#"{ "round": 3, "by": "chair", "from": ["top_worry"] }"#,
        r#"{ "round": 4, "by": "chair", "from": ["top_worry"] }"#,
        "round 3 has no prompts",
    );
}

#[test]
fn a_kind_the_arc_can_never_give_is_refused() {
    // `second` with no mark that adds it.
    refused(
        RIVAL,
        r#"{ "mark": "endorsed", "adds": "second" },"#,
        "",
        "exchange-move-no-requirement",
    );
    // `proof` with nothing found.
    refused(
        RIVAL,
        r#""own", "second", "pass"]"#,
        r#""own", "second", "proof", "pass"]"#,
        "move `proof` can never be offered (exchange-move-no-requirement)",
    );
    // `deny` with no fact to deny.
    let src = edited(
        RIVAL,
        r#""from": ["scandal", "hidden_fact", "absent","#,
        r#""from": ["absent","#,
    );
    let mut src2 = src.clone();
    let f = src2.iter_mut().find(|s| s.path.ends_with(RIVAL)).unwrap();
    f.text = f
        .text
        .replacen(
            r#""own", "second", "pass"]"#,
            r#""own", "deny", "second", "pass"]"#,
            1,
        )
        .replacen(
            r#"{ "mark": "pact", "bars": ["hidden_fact", "scandal"] } ],"#,
            "],",
            1,
        );
    let e: Vec<String> = load(&src2)
        .err()
        .unwrap_or_default()
        .into_iter()
        .map(|x| x.to_string())
        .collect();
    assert!(
        e.iter()
            .any(|x| x.contains("move `deny` can never be offered")),
        "{e:#?}"
    );
    // A `prep` mark nothing sets.
    refused(
        RIVAL,
        r#""with": "ally", "sets": "rehearsed","#,
        r#""with": "ally","#,
        "mark `rehearsed` is read and never set",
    );
}

#[test]
fn an_audience_that_cannot_bind_is_refused() {
    refused(
        RIVAL,
        r#"{ "from": "areas", "of": "race", "max": 5, "weight": "voters", "start": "approval" }"#,
        r#"{ "from": "members" }"#,
        "`members` needs a `power` (exchange-audience-uncast)",
    );
    refused(
        RIVAL,
        r#"{ "from": "areas", "of": "race", "max": 5, "weight": "voters", "start": "approval" }"#,
        r#"{ "from": "person", "role": "chair" }"#,
        "its only groups are optional people",
    );
    // `of: race` in an arc a campaign does not raise.
    refused(
        RIVAL,
        r#""raise": [ { "type": "campaigning" } ],"#,
        r#""raise": [ { "type": "months_in_play", "min": 1 } ],"#,
        "`of: race` in an arc a campaign does not raise (exchange-audience-uncast)",
    );
    // A role the arc lacks.
    refused(
        RIVAL,
        r#""sides": { "other": "rival", "chair": "chair", "second": "backer" },"#,
        r#""sides": { "other": "nobody", "chair": "chair", "second": "backer" },"#,
        "arc-role-uncast",
    );
    // Its place must be a place.
    refused(
        RIVAL,
        r#""at": "hall","#,
        r#""at": "rival","#,
        "must be a place",
    );
}

#[test]
fn a_private_table_airs_no_secret_and_is_not_built_yet() {
    refused(
        RIVAL,
        DEBATE,
        r#"{ "id": "debate", "type": "exchange", "once": true, "table": {},"#,
        "exchange-private-secret",
    );
    // Without the secret it loads, and the engine never raises it: the
    // negotiation's offers arrive with the favour's arc.
    let mut src = edited(
        RIVAL,
        DEBATE,
        r#"{ "id": "debate", "type": "exchange", "once": true, "table": {},"#,
    );
    let f = src.iter_mut().find(|s| s.path.ends_with(RIVAL)).unwrap();
    f.text = f
        .text
        .replacen(
            r#""from": ["scandal", "hidden_fact", "absent","#,
            r#""from": ["scandal", "absent","#,
            1,
        )
        .replacen(
            r#"{ "mark": "pact", "bars": ["hidden_fact", "scandal"] } ],"#,
            r#"{ "mark": "pact", "bars": ["scandal"] } ],"#,
            1,
        );
    let (b, _) = load(&src).unwrap();
    let n = not_yet(b.arc("arc.rival").unwrap());
    assert!(n.iter().any(|x| x.contains("a `table`")), "{n:?}");
    // A source the engine does not build yet does the same.
    let src = edited(
        RIVAL,
        r#""from": ["scandal", "hidden_fact", "absent","#,
        r#""from": ["the_fact", "scandal", "hidden_fact", "absent","#,
    );
    match load(&src) {
        // Its sentences are not written either.
        Err(e) => assert!(
            e.iter()
                .any(|x| x.to_string().contains("exchange-debate-prompt-the_fact-1")),
            "{e:#?}"
        ),
        Ok((b, _)) => assert!(!not_yet(b.arc("arc.rival").unwrap()).is_empty()),
    }
    assert!(!SourceKind::TheFact.built());
}

#[test]
fn text_and_constants_are_checked() {
    // A sentence the engine can name and nobody wrote.
    refused(
        TEXT,
        "exchange-debate-prompt-thin_record-2 =",
        "exchange-debate-prompt-thin_record-9 =",
        "missing Fluent key `exchange-debate-prompt-thin_record-2`",
    );
    refused(
        TEXT,
        "exchange-locked-heard =",
        "exchange-locked-heard-x =",
        "missing Fluent key `exchange-locked-heard`",
    );
    // An argument no scene gives.
    refused(
        TEXT,
        r#"exchange-debate-say-pass-1 = "I have said"#,
        r#"exchange-debate-say-pass-1 = "{ $nobody } I have said"#,
        "uses `$nobody`",
    );
    // A scene's own constant must name one.
    refused(
        RIVAL,
        r#""text": "exchange-debate" },"#,
        r#""text": "exchange-debate", "params": { "no_such": 1.0 } },"#,
        "`no_such` names no constant",
    );
    // The shared block is range-checked.
    refused(
        "exchange.jsonc",
        r#""bite": 0.30,"#,
        r#""bite": 3.0,"#,
        "`bite` must be in [0, 1]",
    );
    refused(
        "exchange.jsonc",
        r#""pass": 0.2,"#,
        "",
        "`cover.pass` is missing",
    );
}
