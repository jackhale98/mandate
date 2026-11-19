//! The map layer registry (DEC-089): the base layers load; a pack adds a
//! layer with no code; an unknown source and broken shapes are rejected.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::map_layers::{MapLayerScale, MapLayerSource};
use mandate_content::{Issue, Severity, Source, bundle::read_dir_sources, load};
use std::path::Path;

fn with(extra: &str) -> Vec<Source> {
    let mut s =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    s.push(Source {
        path: "zz/map_layers.jsonc".into(),
        text: extra.into(),
    });
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

fn layer(body: &str) -> String {
    format!(
        r#"{{ "kind": "map_layers", "items": [{{
  "id": "map_layer.extra", "label_key": "map-layer-rent", "unit_key": "map-layer-rent-unit",
  {body}
}}] }}"#
    )
}

#[test]
fn base_layers_load_in_order() {
    let (b, warnings) = load(&with(r#"{ "kind": "map_layers", "items": [] }"#)).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let ids: Vec<&str> = b.map_layers.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "map_layer.population",
            "map_layer.density",
            "map_layer.unemployment",
            "map_layer.median_income",
            "map_layer.rent",
            "map_layer.industry",
            "map_layer.resource",
        ]
    );
    let u = &b.map_layers[2];
    assert_eq!(u.source, MapLayerSource::Unemployment);
    assert_eq!(u.scale, MapLayerScale::Diverging);
}

#[test]
fn a_pack_adds_a_layer_with_no_code() {
    let (b, _) = load(&with(&layer(
        r#""source": "rent", "scale": "sequential", "format": "money", "legend": { "classes": 4, "breaks": "equal" }"#,
    )))
    .unwrap();
    assert_eq!(b.map_layers.last().unwrap().id, "map_layer.extra");
}

#[test]
fn an_unknown_source_field_is_rejected() {
    let e = errors(&with(&layer(
        r#""source": "voter_turnout", "scale": "sequential", "format": "percent", "legend": { "classes": 5 }"#,
    )));
    assert!(e.contains("zz/map_layers.jsonc"), "{e}");
    assert!(e.contains("voter_turnout"), "{e}");
}

#[test]
fn broken_layers_are_rejected() {
    for (body, want) in [
        (
            r#""source": "main_industry", "scale": "sequential", "format": "count", "legend": { "classes": 5 }"#,
            "categorical scale",
        ),
        (
            r#""source": "rent", "scale": "diverging", "format": "money", "legend": { "classes": 5 }"#,
            "needs a midpoint",
        ),
        (
            r#""source": "rent", "scale": "diverging", "midpoint": { "value": 900 }, "format": "money", "legend": { "classes": 4 }"#,
            "odd 3–7",
        ),
        (
            r#""source": "main_resource", "scale": "categorical", "format": "category", "legend": { "classes": 5 }"#,
            "1–3 categories",
        ),
        (
            r#""source": "rent", "scale": "sequential", "midpoint": "national", "format": "money", "legend": { "classes": 5 }"#,
            "only a diverging layer",
        ),
    ] {
        let e = errors(&with(&layer(body)));
        assert!(e.contains(want), "{body}: {e}");
    }
    // A missing Fluent key.
    let e = errors(&with(
        r#"{ "kind": "map_layers", "items": [{ "id": "map_layer.extra", "label_key": "map-layer-nope",
          "unit_key": "map-layer-rent-unit", "source": "rent", "scale": "sequential", "format": "money",
          "legend": { "classes": 5 } }] }"#,
    ));
    assert!(e.contains("map-layer-nope"), "{e}");
    // A duplicate id.
    let e = errors(&with(&layer(
        r#""source": "rent", "scale": "sequential", "format": "money", "legend": { "classes": 5 }"#,
    )
    .replace("map_layer.extra", "map_layer.rent")));
    assert!(e.contains("duplicate content id `map_layer.rent`"), "{e}");
}
