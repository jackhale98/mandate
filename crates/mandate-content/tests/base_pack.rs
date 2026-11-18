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
    // Attitudes are a registry (DEC-075); keyed signatures resolve densely in
    // registry order, with omitted attitudes at 0.
    assert_eq!(bundle.attitudes.len(), 8);
    assert_eq!(bundle.attitude_index("redistribution"), Some(0));
    let police = &bundle.policies["policy.police_staffing"];
    assert_eq!(police.signature_weights.len(), 8);
    let lo = bundle.attitude_index("law_and_order").unwrap();
    assert_eq!(
        police.signature_weights[lo],
        police.signature["law_and_order"]
    );
    let env = bundle.attitude_index("environment").unwrap();
    assert_eq!(police.signature_weights[env], 0.0);
    // Communities are a registry (DEC-075); the base pack has one, which
    // takes everyone (no share model) and shifts nothing.
    assert_eq!(bundle.communities.len(), 1);
    assert_eq!(bundle.community_index("community.general"), Some(0));
    assert_eq!(bundle.remainder_community(), 0);
    let general = &bundle.communities[0];
    assert!(general.share.is_none());
    assert_eq!(general.attitude_offset_weights, vec![0.0; 8]);
    assert_eq!(general.turnout_offset, 0.0);
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
        text: r#"{"kind":"sectors","items":[{"id":"sector.retail","name_key":"nope","class":"tertiary","driver":"local_services","base_weight":0.1,"base_wage":1,"cyclicality":0,"productivity_growth":0},{"id":"sector.new","name_key":"missing-key","class":"tertiary","driver":"local_services","base_weight":0.1,"base_wage":1,"cyclicality":0,"productivity_growth":0}]}"#.into(),
    });
    let errs = load(&s).unwrap_err();
    assert!(
        errs.iter()
            .any(|i| i.message.contains("duplicate content id `sector.retail`"))
    );
    assert!(errs.iter().any(|i| i.message.contains("missing-key")));
}

#[test]
fn sectors_are_a_registry_in_load_order() {
    let (bundle, _) = load(&base_sources()).unwrap();
    let keys: Vec<&str> = bundle.sector_keys().map(|k| k.as_str()).collect();
    assert_eq!(keys[0], "sector.agriculture");
    assert_eq!(keys[7], "sector.public");
    assert_eq!(bundle.sector_index("sector.retail"), Some(4));

    // Several sectors may share a driver; an expansion adds one in content.
    let mut s = base_sources();
    s.push(Source {
        path: "zz_fish/sectors.jsonc".into(),
        text: r#"{"kind":"sectors","items":[{"id":"sector.fishing","name_key":"sector-agriculture","class":"primary","driver":"land_based","base_weight":0.02,"base_wage":30000,"cyclicality":0.4,"productivity_growth":0.01}]}"#.into(),
    });
    let (b, _) = load(&s).unwrap();
    assert_eq!(b.sectors.len(), 9);
    assert_eq!(b.sector_index("sector.fishing"), Some(8));
    assert_eq!(
        b.sectors_with(mandate_content::SectorDriver::LandBased)
            .count(),
        2
    );
}

#[test]
fn requires_core_sector_drivers_and_known_sector_references() {
    let mut s = base_sources();
    let sec = s
        .iter_mut()
        .find(|x| x.path.ends_with("sectors.jsonc"))
        .unwrap();
    sec.text = sec
        .text
        .lines()
        .filter(|l| !l.contains("\"sector.public\""))
        .collect::<Vec<_>>()
        .join("\n");
    let pol = s
        .iter_mut()
        .find(|x| x.path.ends_with("policies/local.jsonc"))
        .unwrap();
    pol.text = pol.text.replace("\"sector.retail\"", "\"sector.nowhere\"");
    let errs = load(&s).unwrap_err();
    assert!(
        errs.iter()
            .any(|i| i.message.contains("at least one sector with driver Public")),
        "{errs:?}"
    );
    assert!(
        errs.iter()
            .any(|i| i.message.contains("unknown sector `sector.nowhere`"))
    );
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
