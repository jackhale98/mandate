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

/// Errors from loading `sources`, as one string.
fn errors(sources: &[Source]) -> String {
    match load(sources) {
        Ok(_) => String::new(),
        Err(issues) => issues
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Replace `from` with `to` in the base file at `path` (exactly once).
fn edited(path: &str, from: &str, to: &str) -> Vec<Source> {
    let mut s = base_sources();
    let f = s.iter_mut().find(|x| x.path == path).unwrap();
    assert_eq!(f.text.matches(from).count(), 1, "{from}");
    f.text = f.text.replace(from, to);
    s
}

#[test]
fn balance_parameter_blocks_are_validated() {
    // DEC-078 decision 1: every block is required, and out-of-range values
    // are reported with their path.
    let b = load(&base_sources()).unwrap().0;
    assert_eq!(b.opinion.gov_approval_rate, 0.08);
    assert_eq!(b.council.lobby.gain, [0.2, 0.35]);
    assert_eq!(b.career.record_gain.introduce, 0.12);
    assert_eq!(b.npc.bill_sunset_years, [4, 8]);

    let e = errors(&edited(
        "base/opinion.jsonc",
        "\"gov_approval_rate\": 0.08",
        "\"gov_approval_rate\": 1.5",
    ));
    assert!(
        e.contains("opinion.gov_approval_rate = 1.5 outside [0, 1]"),
        "{e}"
    );
    let e = errors(&edited(
        "base/council.jsonc",
        "\"gain\": [0.2, 0.35]",
        "\"gain\": [0.35, 0.2]",
    ));
    assert!(e.contains("council.lobby.gain must be [min, max]"), "{e}");
    let e = errors(&edited(
        "base/actions.jsonc",
        "\"contact\": 1.6,",
        "\"contact\": 1.6, \"contakt\": 2,",
    ));
    assert!(e.contains("unknown field `contakt`"), "{e}");
    let mut s = base_sources();
    s.retain(|x| x.path != "base/career.jsonc");
    assert!(errors(&s).contains("parameter files are all required"));
    let mut s = base_sources();
    let npc = s
        .iter()
        .find(|x| x.path == "base/npc.jsonc")
        .unwrap()
        .clone();
    s.push(Source {
        path: "zz/npc.jsonc".into(),
        text: npc.text,
    });
    assert!(errors(&s).contains("npc defined twice"));
}

/// DEC-077 §3: the base pack answers each kind of crisis with a situational
/// policy, every unlock names a real crisis, and each has a trade-off.
#[test]
fn situational_policies_answer_the_base_crises() {
    let (b, _) = load(&base_sources()).unwrap();
    let situational: Vec<_> = b.policies.values().filter(|p| p.unlock.is_some()).collect();
    assert!(situational.len() >= 4, "{} situational", situational.len());
    for c in &b.crises {
        let answered = situational.iter().any(|p| {
            p.unlock.as_ref().unwrap().any.iter().any(|x| {
                matches!(x, mandate_content::SituationCondition::Crisis { kind, .. } if kind == &c.id)
            })
        });
        assert!(answered, "no situational policy for {}", c.id);
    }
    for p in &situational {
        assert!(!p.side_effects.is_empty(), "{} has no trade-off", p.id);
    }
}

/// A situation trigger naming an unknown crisis is reported (it never
/// holds; a pack may remove a crisis kind, as for dilemma triggers).
#[test]
fn an_unlock_with_an_unknown_crisis_is_reported() {
    let mut src = base_sources();
    let f = src
        .iter_mut()
        .find(|s| s.path.ends_with("policies/situational.jsonc"))
        .unwrap();
    f.text = f.text.replacen(
        "\"kind\": \"crisis.plant_closure\"",
        "\"kind\": \"crisis.nope\"",
        1,
    );
    let (_, warnings) = load(&src).unwrap();
    assert!(
        warnings
            .iter()
            .any(|i| i.to_string().contains("crisis.nope")),
        "{warnings:?}"
    );
}

/// Overheard talk (DEC-079): at least five lines per concern, and the river
/// lines need a river.
#[test]
fn overheard_lines_cover_every_concern() {
    use mandate_content::{HeardCondition, WaterFeature};
    let (b, _) = load(&base_sources()).unwrap();
    for c in &b.concerns {
        let n = b
            .overheard
            .iter()
            .filter(|o| o.concern.as_ref() == Some(&c.id))
            .count();
        assert!(n >= 5, "{} has {n} overheard lines", c.id);
    }
    for o in b
        .overheard
        .iter()
        .filter(|o| o.key.contains("river") || o.key.contains("weir"))
    {
        assert!(
            o.when.iter().any(|w| matches!(
                w,
                HeardCondition::Water {
                    feature: WaterFeature::River
                }
            )),
            "{} mentions a river without needing one",
            o.id
        );
    }
}
