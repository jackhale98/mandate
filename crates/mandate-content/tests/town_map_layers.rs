//! The town map's layer registry (DEC-086, M6 note 11): the base layers
//! load in order; a pack adds a layer with no code; a bad source, a source
//! drawn as the wrong kind and other broken shapes are refused.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::town_map_layers::{TownLayerKind, TownLayerParam, TownLayerSource};
use mandate_content::{Issue, Severity, Source, bundle::read_dir_sources, load};
use std::path::Path;

fn with(extra: &str) -> Vec<Source> {
    let mut s =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    s.push(Source {
        path: "zz/town_map_layers.jsonc".into(),
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
        r#"{{ "kind": "town_map_layers", "items": [{{
  "id": "town_map_layer.extra", "label_key": "town-map-layer-people",
  "unit_key": "town-map-layer-people-unit",
  {body}
}}] }}"#
    )
}

#[test]
fn base_layers_load_in_order() {
    let (b, warnings) = load(&with(r#"{ "kind": "town_map_layers", "items": [] }"#)).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    let ids: Vec<&str> = b.town_map_layers.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "town_map_layer.districts",
            "town_map_layer.people",
            "town_map_layer.worries",
        ]
    );
    for l in &b.town_map_layers {
        assert_eq!(l.kind, l.source.kind(), "{}", l.id);
        assert_eq!(l.scale, l.source.scale(), "{}", l.id);
    }
    let w = &b.town_map_layers[2];
    assert_eq!(w.kind, TownLayerKind::Shaded);
    assert_eq!(w.source, TownLayerSource::Worries { top: true });
    assert_eq!(w.source.param(), Some(TownLayerParam::Worry));
    assert_eq!(w.categories(), 3);
}

#[test]
fn a_pack_adds_a_layer_with_no_code() {
    let (b, _) = load(&with(&layer(
        r#""kind": "shaded", "source": { "type": "worries", "top": false }, "scale": "sequential""#,
    )))
    .unwrap();
    let l = b.town_map_layers.last().unwrap();
    assert_eq!(l.id, "town_map_layer.extra");
    assert_eq!(l.source, TownLayerSource::Worries { top: false });
}

#[test]
fn a_bad_source_is_refused() {
    // A source the engine does not serve.
    let e = errors(&with(&layer(
        r#""kind": "shaded", "source": { "type": "voter_turnout" }, "scale": "sequential""#,
    )));
    assert!(e.contains("zz/town_map_layers.jsonc"), "{e}");
    assert!(e.contains("voter_turnout"), "{e}");
    // A bare name, not an object with a `type`.
    let e = errors(&with(&layer(
        r#""kind": "markers", "source": "people", "scale": "party""#,
    )));
    assert!(e.contains("zz/town_map_layers.jsonc"), "{e}");
    // A setting the source does not have.
    let e = errors(&with(&layer(
        r#""kind": "markers", "source": { "type": "people", "bill": 3 }, "scale": "party""#,
    )));
    assert!(e.contains("zz/town_map_layers.jsonc"), "{e}");
    // A source drawn as a kind it is not.
    let e = errors(&with(&layer(
        r#""kind": "shaded", "source": { "type": "people" }, "scale": "party""#,
    )));
    assert!(e.contains("source `people` is drawn as `markers`"), "{e}");
}

#[test]
fn broken_layers_are_refused() {
    for (body, want) in [
        (
            r#""kind": "boundaries", "source": { "type": "districts" }, "scale": "sequential""#,
            "needs the `party` scale",
        ),
        (
            r#""kind": "boundaries", "source": { "type": "districts" }, "scale": "party", "filters": ["met"]"#,
            "filters are for markers layers",
        ),
        (
            r#""kind": "markers", "source": { "type": "people" }, "scale": "party", "filters": ["met", "met"]"#,
            "listed twice",
        ),
        (
            r#""kind": "markers", "source": { "type": "people" }, "scale": "party", "legend": { "categories": 2 }"#,
            "legend.categories is for shaded layers",
        ),
        (
            r#""kind": "shaded", "source": { "type": "worries" }, "scale": "sequential", "legend": { "categories": 5 }"#,
            "1–3 categories",
        ),
    ] {
        let e = errors(&with(&layer(body)));
        assert!(e.contains(want), "{body}: {e}");
    }
    // An id outside the registry's namespace.
    let e = errors(&with(
        &layer(r#""kind": "markers", "source": { "type": "people" }, "scale": "party""#)
            .replace("town_map_layer.extra", "map_layer.extra"),
    ));
    assert!(e.contains("starting with `town_map_layer.`"), "{e}");
    // A missing Fluent key.
    let e = errors(&with(
        &layer(r#""kind": "markers", "source": { "type": "people" }, "scale": "party""#)
            .replace("town-map-layer-people-unit", "town-map-layer-nope"),
    ));
    assert!(e.contains("town-map-layer-nope"), "{e}");
    // A duplicate id.
    let e = errors(&with(
        &layer(r#""kind": "markers", "source": { "type": "people" }, "scale": "party""#)
            .replace("town_map_layer.extra", "town_map_layer.people"),
    ));
    assert!(
        e.contains("duplicate content id `town_map_layer.people`"),
        "{e}"
    );
}
