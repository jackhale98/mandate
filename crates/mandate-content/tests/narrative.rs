#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Narrative content validation (DEC-073, DEC-077 §4, DEC-032): every option
//! costs someone and buys something, exactly one default, roles resolve,
//! requests are raised by goals, storylines branch to real stages.
use mandate_content::{Severity, Source, bundle::read_dir_sources, load};
use std::path::Path;

fn with(extra: &str) -> Vec<String> {
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    src.push(Source {
        path: "aa_test/dilemmas.jsonc".into(),
        text: extra.into(),
    });
    match load(&src) {
        Ok(_) => Vec::new(),
        Err(issues) => issues
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .map(|i| i.to_string())
            .collect(),
    }
}

fn dilemma(options: &str, roles: &str) -> String {
    format!(
        r#"{{"kind":"dilemmas","items":[{{"id":"dilemma.test","category":"significant",
        "title_key":"dilemma-pollution","situation_key":"dilemma-pollution-situation",
        "roles":[{roles}],"cooldown_days":30,"options":[{options}]}}]}}"#
    )
}

const ROLE: &str = r#"{"id":"who","cast":{"type":"mayor"}}"#;

fn opt(id: &str, default: bool, effects: &str) -> String {
    format!(
        r#"{{"id":"{id}","default":{default},"label_key":"dilemma-pollution-wait","aftermath_key":"dilemma-pollution-wait-after","effects":[{effects}]}}"#
    )
}

#[test]
fn the_base_narrative_content_is_valid() {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    let (b, warnings) = load(&src).unwrap();
    assert!(warnings.is_empty());
    let standalone = b
        .dilemmas
        .iter()
        .filter(|d| d.category != mandate_content::DilemmaCategory::Request && !d.storyline_only)
        .count();
    assert!((8..=12).contains(&standalone), "{standalone} base dilemmas");
    assert!(b.storylines.len() >= 2);
    assert_eq!(b.storytellers.len(), 3);
}

#[test]
fn an_option_without_a_cost_is_an_error() {
    let good = r#"{"type":"opinion","role":"who","delta":-5}"#;
    let free = r#"{"type":"money","amount":500}"#;
    let e = with(&dilemma(
        &[opt("a", true, good), opt("b", false, free)].join(","),
        ROLE,
    ));
    assert!(e.iter().any(|x| x.contains("at least one cost")), "{e:?}");
}

#[test]
fn defaults_and_roles_are_checked() {
    let both = r#"{"type":"opinion","role":"who","delta":-5},{"type":"money","amount":500}"#;
    // Two defaults.
    let e = with(&dilemma(
        &[opt("a", true, both), opt("b", true, both)].join(","),
        ROLE,
    ));
    assert!(e.iter().any(|x| x.contains("exactly one option")), "{e:?}");
    // An unknown role.
    let ghost = r#"{"type":"opinion","role":"ghost","delta":-5},{"type":"money","amount":500}"#;
    let e = with(&dilemma(
        &[opt("a", true, ghost), opt("b", false, both)].join(","),
        ROLE,
    ));
    assert!(
        e.iter().any(|x| x.contains("unknown role `ghost`")),
        "{e:?}"
    );
    // A default that costs time.
    let slow = r#"{"type":"slots","n":1},{"type":"money","amount":500}"#;
    let e = with(&dilemma(
        &[opt("a", true, slow), opt("b", false, both)].join(","),
        ROLE,
    ));
    assert!(e.iter().any(|x| x.contains("cannot cost time")), "{e:?}");
    // A request no goal raises.
    let req = r#"{"kind":"dilemmas","items":[{"id":"request.orphan","category":"request",
        "title_key":"dilemma-pollution","situation_key":"dilemma-pollution-situation",
        "roles":[{"id":"holder","cast":{"type":"goal_holder"}}],"cooldown_days":30,"options":[
        {"id":"a","default":true,"label_key":"dilemma-pollution-wait","aftermath_key":"dilemma-pollution-wait-after","effects":[{"type":"opinion","role":"holder","delta":-5},{"type":"money","amount":5}]},
        {"id":"b","label_key":"dilemma-pollution-wait","aftermath_key":"dilemma-pollution-wait-after","effects":[{"type":"opinion","role":"holder","delta":5},{"type":"money","amount":-5}]}]}]}"#;
    let e = with(req);
    assert!(e.iter().any(|x| x.contains("raised by a goal")), "{e:?}");
}
