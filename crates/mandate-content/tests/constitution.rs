#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Constitution validation (DEC-017, DEC-098): posts by id, role words,
//! only implemented voting rules, seats for at-large bodies, the engine's
//! procedures, and a career ladder that reaches its summit.
use mandate_content::{Severity, bundle::read_dir_sources, load};
use std::path::Path;

const CONSTITUTION: &str = "base/constitutions/federal_presidential.jsonc";

/// Every error after replacing `from` with `to` (once) in a base file.
fn errors_with(file: &str, from: &str, to: &str) -> String {
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    let f = src
        .iter_mut()
        .find(|s| s.path.replace('\\', "/") == file)
        .unwrap();
    assert!(f.text.contains(from), "{from}");
    f.text = f.text.replacen(from, to, 1);
    match load(&src) {
        Ok(_) => String::new(),
        Err(issues) => issues
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

#[test]
fn the_base_constitution_is_valid() {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    let (b, warnings) = load(&src).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let c = b.constitutions.values().next().unwrap();
    assert_eq!(c.career.summit, "president");
    // The derived ladder: local, regional, national; seats before offices.
    let ladder: Vec<&str> = c.ladder().iter().map(|(_, p)| p.id()).collect();
    assert_eq!(
        ladder,
        [
            "council",
            "mayor",
            "assembly",
            "governor",
            "house",
            "senate",
            "president"
        ]
    );
    let local = c.tier(mandate_content::TierKey::Local).unwrap();
    assert_eq!(local.legislature().unwrap().id, "council");
    assert_eq!(local.executive().unwrap().id, "mayor");
    assert_eq!(
        c.act_of(mandate_content::TierKey::National, "president")
            .unwrap()
            .id,
        "act.summit"
    );
    assert_eq!(
        c.act_of(mandate_content::TierKey::National, "senate")
            .unwrap()
            .id,
        "act.national"
    );
    assert_eq!(b.role_words(), ["councillor", "mayor"]);
    assert!(b.is_role_word("senator") && !b.is_role_word("pope"));
    assert_eq!(
        b.career
            .nomination
            .get("mayor")
            .and_then(|g| g.min_standing),
        Some(40.0)
    );
}

#[test]
fn only_plurality_has_a_tabulator() {
    let e = errors_with(
        CONSTITUTION,
        r#""rule": "plurality", "constituency": "ward" },
            "term_years": 4"#,
        r#""rule": "two_round", "constituency": "ward" },
            "term_years": 4"#,
    );
    assert!(e.contains("has no tabulator yet (only plurality)"), "{e}");
}

#[test]
fn an_at_large_body_needs_seats() {
    let e = errors_with(CONSTITUTION, r#""seats": 30,"#, "");
    assert!(e.contains("at-large body `senate` needs `seats`"), "{e}");
}

#[test]
fn ids_are_unique_across_tiers() {
    let e = errors_with(
        CONSTITUTION,
        r#""id": "governor", "name_key": "office-governor""#,
        r#""id": "mayor", "name_key": "office-governor""#,
    );
    assert!(e.contains("body or office id `mayor` is used twice"), "{e}");
}

#[test]
fn every_tier_has_the_engine_procedures() {
    let e = errors_with(
        CONSTITUTION,
        r#""id": "budget", "deliberation_days": 28"#,
        r#""id": "budgets", "deliberation_days": 28"#,
    );
    assert!(e.contains("lacks the `budget` procedure"), "{e}");
}

#[test]
fn the_career_reaches_a_real_summit() {
    let e = errors_with(
        CONSTITUTION,
        r#""summit": "president""#,
        r#""summit": "emperor""#,
    );
    assert!(e.contains("career summit `emperor` is not a post"), "{e}");
    let e = errors_with(
        CONSTITUTION,
        r#"{ "id": "act.summit", "posts": ["president"] },"#,
        r#"{ "id": "act.summit", "posts": ["pontiff"] },"#,
    );
    assert!(
        e.contains("act `act.summit` names unknown post `pontiff`"),
        "{e}"
    );
}

#[test]
fn a_workplace_is_a_place_kind() {
    let e = errors_with(
        CONSTITUTION,
        r#""works_at": "town_hall""#,
        r#""works_at": "palace""#,
    );
    assert!(
        e.contains("`mayor` works at `palace`, which is not a place kind"),
        "{e}"
    );
}

#[test]
fn content_names_posts_by_role_word() {
    // An office condition with a word no constitution defines.
    let e = errors_with(
        "base/dilemmas.jsonc",
        r#"{ "type": "office", "role": "councillor" }"#,
        r#"{ "type": "office", "role": "alderman" }"#,
    );
    assert!(e.contains("unknown office word `alderman`"), "{e}");
    // A conversation line for a speaker with an unknown role.
    let e = errors_with(
        "base/conversations/lines.jsonc",
        r#""occupations": ["councillor", "mayor"]"#,
        r#""occupations": ["councillor", "doge"]"#,
    );
    assert!(e.contains("unknown role `doge`"), "{e}");
    // A nomination gate for an unknown post.
    let e = errors_with(
        "base/career.jsonc",
        r#""mayor": { "min_standing": 40 }"#,
        r#""doge": { "min_standing": 40 }"#,
    );
    assert!(
        e.contains("nomination gate for unknown role word `doge`"),
        "{e}"
    );
}
