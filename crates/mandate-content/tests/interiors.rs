//! Interior recipes (DEC-088, DEC-098 §4): the base recipes load and pass the
//! tile rules ported from the art pack's validator; broken recipes give the
//! expected errors.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{Issue, Severity, Source, bundle::read_dir_sources, load};
use std::path::Path;

const FILE: &str = "base/interiors.jsonc";

fn base_sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

/// Replace one base file's text.
fn patched(path: &str, f: impl Fn(&str) -> String) -> Vec<Source> {
    let mut s = base_sources();
    let src = s.iter_mut().find(|x| x.path == path).unwrap();
    src.text = f(&src.text);
    s
}

fn errors(sources: &[Source]) -> String {
    match load(sources) {
        Ok(_) => String::new(),
        Err(issues) => issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .map(Issue::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Replace text in the interiors file (once; the text must be there).
fn with_change(from: &str, to: &str) -> Vec<Source> {
    patched(FILE, |t| {
        assert!(t.contains(from), "fixture text not found: {from}");
        t.replacen(from, to, 1)
    })
}

#[test]
fn base_recipes_load_and_places_name_them() {
    let (b, warnings) = load(&base_sources()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let ids: Vec<String> = b
        .interiors
        .iter()
        .map(|r| format!("{}@{}", r.id, r.version))
        .collect();
    assert_eq!(
        ids,
        [
            "interior.council_chamber@1",
            "interior.town_hall@2",
            "interior.diner@1"
        ]
    );
    for key in ["council_chamber", "town_hall", "diner"] {
        let kind = b.place_kind(key).unwrap();
        assert_eq!(
            kind.interior.as_deref(),
            Some(format!("interior.{key}").as_str())
        );
    }
    // Places without a recipe keep the panel.
    assert_eq!(b.place_kind("school").unwrap().interior, None);
}

#[test]
fn a_spot_out_of_reach_of_the_door_is_refused() {
    // Wall the town hall's waiting bench in: a filing cabinet on each open side of (0, 7)–(1, 7).
    let e = errors(&with_change(
        r#"{ "part": "waiting_bench", "x": 0, "y": 7, "len": 2 },"#,
        r#"{ "part": "waiting_bench", "x": 0, "y": 7, "len": 2 },
        { "part": "filing_cabinet", "x": 0, "y": 8 },
        { "part": "filing_cabinet", "x": 1, "y": 8 },
        { "part": "filing_cabinet", "x": 2, "y": 7 },
        { "part": "filing_cabinet", "x": 0, "y": 6 },
        { "part": "filing_cabinet", "x": 1, "y": 6 },"#,
    ));
    assert!(
        e.contains("spot waiting3 at (0, 7) is not reachable from the door"),
        "{e}"
    );
}

#[test]
fn a_spot_on_furniture_is_refused() {
    let e = errors(&with_change(
        r#"{ "id": "forms1", "role": "public", "x": 9, "y": 6, "facing": "down" },"#,
        r#"{ "id": "forms1", "role": "public", "x": 9, "y": 7, "facing": "down" },"#,
    ));
    assert!(
        e.contains("spot forms1 at (9, 7) is not on walkable floor"),
        "{e}"
    );
}

#[test]
fn a_hotspot_action_the_place_does_not_offer_is_refused() {
    // The town hall does not take bribes; the diner offers `take_bribe` only late.
    let e = errors(&with_change(
        r#"{ "id": "window2", "actions": ["draft"],"#,
        r#"{ "id": "window2", "actions": ["draft", "take_bribe"],"#,
    ));
    assert!(
        e.contains("hotspot window2: `take_bribe` is not an action of town_hall"),
        "{e}"
    );
    let e = errors(&with_change(
        r#"{ "id": "counter", "services": ["overheard"],"#,
        r#"{ "id": "counter", "services": ["overheard", "budget_papers"],"#,
    ));
    assert!(
        e.contains("hotspot counter: `budget_papers` is not a service of diner"),
        "{e}"
    );
}

#[test]
fn unknown_parts_overlaps_and_seatless_sitters_are_refused() {
    let e = errors(&with_change(
        r#"{ "part": "leaflet_stand", "x": 11, "y": 4 },"#,
        r#"{ "part": "leaflet_stand", "x": 11, "y": 4 },
        { "part": "no_such_part", "x": 4, "y": 6 },
        { "part": "plant_tall", "x": 6, "y": 3 },"#,
    ));
    assert!(e.contains(r#"unknown part "no_such_part""#), "{e}");
    assert!(e.contains("plant_tall overlaps counter at (6, 3)"), "{e}");
    let e = errors(&with_change(
        r#"{ "id": "official", "role": "official", "x": 4, "y": 1, "facing": "down" },"#,
        r#"{ "id": "official", "role": "official", "x": 4, "y": 1, "facing": "down", "pose": "sit" },"#,
    ));
    assert!(
        e.contains("spot official sits but (4, 1) has no seat"),
        "{e}"
    );
}

#[test]
fn cast_roles_and_place_links_are_checked() {
    let e = errors(&with_change(
        r#""cast": { "presiding": ["official", "public"],"#,
        r#""cast": { "mayor": ["official"], "presiding": ["bouncer", "public"],"#,
    ));
    assert!(e.contains("cast: `mayor` is not one of"), "{e}");
    assert!(e.contains("cast `presiding`: no `bouncer` spot"), "{e}");
    let e = errors(&patched("base/places.jsonc", |t| {
        t.replacen(
            r#""interior": "interior.diner","#,
            r#""interior": "interior.bistro","#,
            1,
        )
    }));
    assert!(
        e.contains("interior `interior.bistro` is not a recipe for diner"),
        "{e}"
    );
}

/// Numbers as f64 (a file's `2` and a serialised `2.0` are the same weight).
fn norm(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Number(n) => serde_json::json!(n.as_f64()),
        serde_json::Value::Array(a) => serde_json::Value::Array(a.iter().map(norm).collect()),
        serde_json::Value::Object(o) => {
            serde_json::Value::Object(o.iter().map(|(k, x)| (k.clone(), norm(x))).collect())
        }
        other => other.clone(),
    }
}

#[test]
fn a_recipe_travels_as_the_content_says_it() {
    // The place view carries the recipe as JSON (the client's composer reads it): nothing lost or changed.
    let (b, _) = load(&base_sources()).unwrap();
    let text = base_sources()
        .into_iter()
        .find(|s| s.path == FILE)
        .unwrap()
        .text;
    let file: serde_json::Value =
        serde_json::from_str(&mandate_content::jsonc::to_strict_json(&text)).unwrap();
    let items = file["items"].as_array().unwrap();
    assert_eq!(items.len(), b.interiors.len());
    for (r, raw) in b.interiors.iter().zip(items) {
        let v = serde_json::to_value(r).unwrap();
        let back: mandate_content::interiors::InteriorRecipe =
            serde_json::from_value(v.clone()).unwrap();
        assert_eq!(&back, r);
        // Every field the file writes comes out the same (defaults may be added, never changed).
        for (k, val) in raw.as_object().unwrap() {
            assert_eq!(norm(&v[k]), norm(val), "{} {k}", r.id);
        }
    }
}
