//! Town layout v2 registries (DEC-090 D4, TOWN_GENERATION.md §6): the base
//! entries load, a pack can add a type with no code, and broken entries give
//! the expected validation errors.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{Issue, Severity, Source, bundle::read_dir_sources, load};
use std::path::Path;

fn base_sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn with(extra: &[(&str, &str)]) -> Vec<Source> {
    let mut s = base_sources();
    for (path, text) in extra {
        s.push(Source {
            path: (*path).into(),
            text: (*text).into(),
        });
    }
    s
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

const HARBOUR: &str = r#"{ "kind": "town_districts", "items": [{
  "id": "district.fishing_quay", "name_key": "district-fishing_quay",
  "appears": { "hood_kinds": ["waterfront"], "geo": { "coast": true, "fishing_pm": { "min": 450 } }, "weight": 3 },
  "site": { "prefers": ["shore", "low_slope"], "avoids": ["cliff"], "anchor": "harbour" },
  "streets": { "pattern": "quay", "block": [8, 12], "depth": [6, 8], "lanes": "stepped", "max_dead_ends": 2, "style": "asphalt" },
  "plots": { "frontage": [3, 4], "depth": [3, 3], "setback": 0, "joined": 0.8, "ground": "yard" },
  "buildings": { "mix": [["terrace", 5], ["boathouse", 2]], "storeys": [1, 3], "density": 0.85 },
  "hosts": ["port", "market"],
  "growth": { "reserve": 0.1, "fill": "infill_first", "decline": "boarded" },
  "tags": ["storm_surge"],
  "art": { "kit": "kit.harbour_cottage", "family": "residential" }
}] }"#;

#[test]
fn base_registries_load_in_order() {
    let (b, warnings) = load(&base_sources()).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(b.town_districts.len(), 17);
    assert_eq!(b.terrain_features.len(), 9);
    assert_eq!(b.connectors.len(), 6);
    assert_eq!(b.town_districts[0].id, "district.old_core");
    // Every place kind but the pseudo-locations has a layout block.
    for p in &b.places {
        let pseudo = p.key == "home" || p.key == "streets";
        assert_eq!(p.layout.is_some(), !pseudo, "{}", p.key);
    }
    let mine = b.place_kind("mine").unwrap().layout.as_ref().unwrap();
    assert_eq!(mine.signature.as_deref(), Some("headframe"));
    assert_eq!(mine.place[1], "toward:minerals");
}

#[test]
fn a_pack_adds_a_district_type_with_no_code() {
    let (b, warnings) = load(&with(&[("zz/layout.jsonc", HARBOUR)])).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(b.town_districts.last().unwrap().id, "district.fishing_quay");
}

#[test]
fn broken_district_types_are_refused() {
    let bad = HARBOUR
        .replace(
            r#""hood_kinds": ["waterfront"]"#,
            r#""hood_kinds": ["moon"]"#,
        )
        .replace(r#""weight": 3"#, r#""weight": 0"#)
        .replace(r#""block": [8, 12]"#, r#""block": [3, 4]"#)
        .replace(r#""joined": 0.8"#, r#""joined": 1.5"#)
        .replace(r#"["boathouse", 2]"#, r#"["castle", 2]"#)
        .replace(
            r#""hosts": ["port", "market"]"#,
            r#""hosts": ["spaceport"]"#,
        )
        .replace(r#""prefers": ["shore", "#, r#""prefers": ["moonlit", "#)
        .replace(r#""min": 450"#, r#""min": 1450"#)
        .replace("kit.harbour_cottage", "Kit Harbour");
    let e = errors(&with(&[("zz/layout.jsonc", &bad)]));
    for want in [
        "unknown neighbourhood kind `moon`",
        "appears.weight must be a positive integer",
        "minimum block 6 × 5",
        "plots.joined must be 0..1",
        "unknown filler building kind `castle`",
        "unknown place kind `spaceport`",
        "unknown site predicate `moonlit`",
        "fishing_pm must be a range within 0..1000",
        "art key `Kit Harbour`",
    ] {
        assert!(e.contains(want), "missing `{want}` in:\n{e}");
    }
    // A street pattern or a field the generator does not know fails to parse.
    let e = errors(&with(&[(
        "zz/layout.jsonc",
        &HARBOUR.replace(r#""pattern": "quay""#, r#""pattern": "spiral""#),
    )]));
    assert!(e.contains("spiral"), "{e}");
    let e = errors(&with(&[(
        "zz/layout.jsonc",
        &HARBOUR.replace(r#""coast": true"#, r#""volcano": true"#),
    )]));
    assert!(e.contains("volcano"), "{e}");
    // The art family is required (§7.2).
    let e = errors(&with(&[(
        "zz/layout.jsonc",
        &HARBOUR.replace(r#", "family": "residential""#, ""),
    )]));
    assert!(e.contains("family"), "{e}");
    // Ids are unique across packs.
    let e = errors(&with(&[
        ("zz/layout.jsonc", HARBOUR),
        ("zz/again.jsonc", HARBOUR),
    ]));
    assert!(
        e.contains("duplicate content id `district.fishing_quay`"),
        "{e}"
    );
}

#[test]
fn every_neighbourhood_kind_keeps_a_type_that_always_applies() {
    let s = patched("base/layout/town_districts.jsonc", |t| {
        t.replace(
            r#""hood_kinds": ["rural"], "weight": 3"#,
            r#""hood_kinds": ["rural"], "population": { "min": 1 }, "weight": 3"#,
        )
    });
    let e = errors(&s);
    assert!(
        e.contains("no unconditional district type for neighbourhood kind `rural`"),
        "{e}"
    );
}

#[test]
fn terrain_features_and_connectors_are_checked() {
    let e = errors(&with(&[(
        "zz/layout.jsonc",
        r#"{ "kind": "connectors", "items": [
          { "id": "connector.rope", "kind": "footbridge", "max_span": 40, "roads": ["path", "zipline"], "crosses": ["lava"], "art": {} }
        ] }"#,
    )]));
    for want in [
        "max_span must be 1..32",
        "unknown road `zipline`",
        "unknown crossing `lava`",
        "art.kit is required",
    ] {
        assert!(e.contains(want), "missing `{want}` in:\n{e}");
    }
    // The generator is integer-only.
    let e = errors(&with(&[(
        "zz/layout.jsonc",
        r#"{ "kind": "terrain_features", "items": [
          { "id": "feature.dunes", "generator": "beach", "params": { "width": 2.5 }, "art": { "ground": "ground.sand" } }
        ] }"#,
    )]));
    assert!(!e.is_empty());
    // The base game needs a river, a creek, a coast and hills.
    let s = patched("base/layout/terrain_features.jsonc", |t| {
        t.replace(r#""generator": "creek""#, r#""generator": "marsh""#)
    });
    assert!(errors(&s).contains("towns need a `creek` feature"));
    let s = patched("base/layout/connectors.jsonc", |t| {
        t.replace(r#""kind": "ford""#, r#""kind": "footbridge""#)
    });
    assert!(errors(&s).contains("towns need a `ford` connector"));
}

#[test]
fn place_layout_blocks_are_checked() {
    let s = patched("base/places.jsonc", |t| {
        t.replace(r#""slot": "works", "footprint": [[5, 4], [8, 5]], "place": ["edge", "toward:minerals""#,
            r#""slot": "pit", "footprint": [[9, 4], [8, 5]], "place": ["edge", "toward:sea""#)
            .replace("near_place:factory", "near_place:spaceport")
            .replace(r#""place": ["central", "arterial"]"#, r#""place": ["central", "upside_down"]"#)
    });
    let e = errors(&s);
    for want in [
        "unknown layout slot `pit`",
        "layout.footprint must be",
        "`toward:` takes minerals or woods, not `sea`",
        "unknown place kind `spaceport`",
        "unknown placement predicate `upside_down`",
    ] {
        assert!(e.contains(want), "missing `{want}` in:\n{e}");
    }
    // A drawn kind without a block is drawn generically, with a warning.
    let s = patched("base/places.jsonc", |t| {
        let start = t
            .find(r#""layout": { "slot": "community", "footprint": [[6, 4]"#)
            .unwrap();
        let end = start + t[start..].find("\n").unwrap();
        format!("{}{}", &t[..start], &t[end..])
    });
    let (_, warnings) = load(&s).unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.file == "place.school" && w.message.contains("generic footprint")),
        "{warnings:?}"
    );
    // A pseudo-location is not a building.
    let s = patched("base/places.jsonc", |t| {
        t.replace(
            r#""name_key": "place-home", "actions": ["rest"]"#,
            r#""name_key": "place-home", "actions": ["rest"], "layout": { "slot": "civic", "footprint": [[3, 3], [3, 3]], "place": ["quiet"], "landmark": false, "art": { "family": "residential" } }"#,
        )
    });
    assert!(errors(&s).contains("a pseudo-location is not a building"));
}
