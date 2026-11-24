#![allow(clippy::unwrap_used, clippy::expect_used)]
//! DEC-075: attitudes are a content registry. An expansion adds an issue (and
//! a policy that speaks to it) purely in content, and the society holds views
//! on it.

use mandate_content::{ContentBundle, Source, bundle::read_dir_sources, load};
use mandate_sim::Command;
use mandate_sim::world::NeighborhoodKind;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn content_with_land_rights() -> Arc<ContentBundle> {
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    src.push(Source {
        path: "land/attitudes.jsonc".into(),
        text: r#"{
          // A new issue: strongly held in the countryside, opposed downtown.
          "kind": "attitudes",
          "items": [
            { "id": "attitude.land_rights", "key": "land_rights",
              "seed": { "by_class": [0.1, 0.0, -0.1],
                        "by_neighborhood": { "rural": 0.5, "core": -0.4, "inner": -0.4 } } },
          ],
        }"#
        .into(),
    });
    src.push(Source {
        path: "land/policies.jsonc".into(),
        text: r#"{
          "kind": "policies",
          "items": [
            {
              "id": "policy.land_trust", "name_key": "policy-parks", "desc_key": "policy-parks-desc",
              "tier": "local", "category": "environment", "requires": "propose_ordinance",
              "params": [ { "id": "level", "name_key": "param-service-level", "min": 1, "max": 5, "default": 2, "step": 1, "unit_key": "unit-level" } ],
              "cost": { "per_capita_per_unit": 5, "param": "level" },
              "mechanisms": [ { "type": "service", "category": "environment", "param": "level", "per_unit": 0.01 } ],
              "implementation_months": 6,
              "signature": { "land_rights": 0.8, "localism": 0.2 },
              "secondary_keys": ["secondary-parks-upkeep"],
              "secondary_backing": { "secondary-parks-upkeep": "cost" },
              "side_effects": [ { "type": "disruption", "per_unit": 0.01, "param": "level" } ],
            },
          ],
        }"#
        .into(),
    });
    src.push(Source {
        path: "land/locale/en/land.ftl".into(),
        text:
            "conviction-pro-land_rights = Land back\nconviction-anti-land_rights = Settled title\nissue-land_rights = Land rights\n"
                .into(),
    });
    Arc::new(load(&src).unwrap().0)
}

fn run(content: &Arc<ContentBundle>, seed: u64) -> mandate_sim::Engine {
    let mut e = generate(&WorldConfig::new(seed, WorldSize::Small), content.clone()).unwrap();
    e.apply(Command::AdvanceDays { days: 365 * 2 }).unwrap();
    e
}

#[test]
fn an_expansion_attitude_generates_simulates_and_stays_deterministic() {
    let c = content_with_land_rights();
    assert_eq!(c.attitudes.len(), 9);
    let land = c.attitude_index("land_rights").unwrap();
    let t = &c.policies["policy.land_trust"];
    assert_eq!(t.signature_weights.len(), 9);
    assert_eq!(t.signature_weights[land], 0.8);
    assert_eq!(
        t.signature_weights[c.attitude_index("localism").unwrap()],
        0.2
    );

    let a = run(&c, 93);
    let b = run(&c, 93);
    assert_eq!(a.state_hash(), b.state_hash(), "same seed, same state");

    let s = a.state();
    assert_eq!(s.population.attitude_keys.len(), 9);
    assert_eq!(s.population.attitude_keys[land], "land_rights");
    for hp in &s.population.hoods {
        for co in &hp.cohorts {
            assert_eq!(co.attitudes.len(), 9);
            assert!(co.attitudes.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
        }
    }
    // Politicians and parties carry the new dimension, and take positions on it.
    assert!(!s.politics.politicians.is_empty());
    for p in &s.politics.politicians {
        assert_eq!(p.attitudes.len(), 9);
    }
    assert!(
        s.politics
            .politicians
            .iter()
            .any(|p| p.attitudes[land].abs() > 0.05)
    );
    assert!(!s.politics.parties.is_empty());
    for p in &s.politics.parties {
        assert_eq!(p.platform.len(), 9);
        assert!(p.factions.iter().all(|f| f.position.len() == 9));
    }
    assert!(s.politics.parties.iter().any(|p| p.platform[land] != 0.0));

    // The policy's public support follows the new issue: rural voters back
    // it, downtown voters do not.
    let hoods_of = |kinds: &[NeighborhoodKind]| -> Vec<usize> {
        s.world
            .neighborhoods
            .iter()
            .enumerate()
            .filter(|(_, n)| kinds.contains(&n.kind))
            .map(|(i, _)| i)
            .collect()
    };
    let rural = hoods_of(&[NeighborhoodKind::Rural]);
    let urban = hoods_of(&[NeighborhoodKind::Core, NeighborhoodKind::Inner]);
    assert!(!rural.is_empty() && !urban.is_empty());
    let support = |h: &[usize]| mandate_sim::policy::public_support(t, &s.population, h);
    let (r, u) = (support(&rural), support(&urban));
    assert!(r > 0.2, "rural support {r}");
    assert!(u < 0.0, "urban support {u}");

    // Base content is unchanged by the expansion's existence.
    let base_src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    let base = load(&base_src).unwrap().0;
    assert_eq!(base.attitudes.len(), 8);
}

#[test]
fn unknown_signature_attitudes_and_missing_conviction_keys_are_errors() {
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    src.push(Source {
        path: "bad/attitudes.jsonc".into(),
        text: r#"{"kind":"attitudes","items":[{"id":"attitude.language","key":"language"}]}"#
            .into(),
    });
    src.push(Source {
        path: "bad/policies.jsonc".into(),
        text: r#"{"kind":"policies","items":[{"id":"policy.x","name_key":"policy-parks","desc_key":"policy-parks-desc","tier":"local","category":"environment","requires":"propose_ordinance","params":[],"cost":{"per_capita_per_unit":1},"mechanisms":[],"implementation_months":1,"signature":{"whaling":0.5},"secondary_keys":["secondary-parks-upkeep"],"secondary_backing":{"secondary-parks-upkeep":"cost"}}]}"#
            .into(),
    });
    let errs = load(&src).unwrap_err();
    let msg = errs
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(msg.contains("unknown attitude `whaling`"), "{msg}");
    assert!(msg.contains("conviction-pro-language"), "{msg}");
    assert!(msg.contains("conviction-anti-language"), "{msg}");
}
