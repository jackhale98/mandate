#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_content::{Source, bundle::read_dir_sources, load};
use std::path::Path;

fn base_sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

#[test]
fn base_pack_is_valid() {
    let (bundle, warnings) = load(&base_sources()).unwrap_or_else(|e| {
        panic!(
            "{}",
            e.iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )
    });
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(bundle.sectors.len(), 8);
    assert_eq!(bundle.tuning.standard_slots, 3);
}

#[test]
fn content_hash_is_stable_and_order_independent() {
    let a = base_sources();
    let mut b = a.clone();
    b.reverse();
    assert_eq!(
        load(&a).unwrap().0.content_hash,
        load(&b).unwrap().0.content_hash
    );
}

#[test]
fn reports_parse_errors_with_line_numbers() {
    let mut s = base_sources();
    s.push(Source {
        path: "bad/x.jsonc".into(),
        text: "{\n  // c\n  \"kind\": \"sectors\",\n  \"items\": [ { \"id\": 3 } ]\n}".into(),
    });
    let errs = load(&s).unwrap_err();
    let e = errs.iter().find(|i| i.file == "bad/x.jsonc").unwrap();
    assert_eq!(e.line, Some(4));
}

#[test]
fn detects_duplicates_and_missing_locale_keys() {
    let mut s = base_sources();
    s.push(Source {
        path: "dup/sectors.jsonc".into(),
        text: r#"{"kind":"sectors","items":[{"id":"sector.retail","name_key":"nope","class":"tertiary","base_weight":0.1},{"id":"sector.new","name_key":"missing-key","class":"tertiary","base_weight":0.1}]}"#.into(),
    });
    let errs = load(&s).unwrap_err();
    assert!(
        errs.iter()
            .any(|i| i.message.contains("duplicate content id `sector.retail`"))
    );
    assert!(errs.iter().any(|i| i.message.contains("missing-key")));
}

#[test]
fn embedded_matches_disk() {
    // Compiled with --features embedded in CI; skip silently otherwise.
    #[cfg(feature = "embedded")]
    {
        let disk = load(&base_sources()).unwrap().0.content_hash;
        let emb = load(&mandate_content::embedded::sources())
            .unwrap()
            .0
            .content_hash;
        assert_eq!(disk, emb);
    }
}
