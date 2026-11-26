#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Policy families in content (DEC-092; M6_ENGINE_CONTRACTS §2.9, §2.13,
//! task F1): the format parses, the sample families validate beside the base
//! pack, each validation item refuses what it should, family conditions are
//! refused outside families, non-local families use tier-generic conditions
//! only, the old hazard spelling still parses, and patches apply in pack
//! order.
use mandate_content::policy_family::{FamilyPatch, PolicyFamilyDef, Rarity};
use mandate_content::{
    Condition, ContentBundle, HazardKey, HazardRef, MechanismDef, Severity, Source,
    bundle::read_dir_sources, load,
};
use std::path::Path;

const FAMILIES: &str = include_str!("fixtures/families/test_families.jsonc");
const FTL: &str = include_str!("fixtures/families/test_families.ftl");

fn base_sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn with(extra: &[(&str, &str)]) -> Vec<Source> {
    let mut src = base_sources();
    for (path, text) in extra {
        src.push(Source {
            path: (*path).into(),
            text: (*text).into(),
        });
    }
    src
}

fn sample() -> Vec<Source> {
    with(&[
        ("zz_test/policy_families/test.jsonc", FAMILIES),
        ("zz_test/locale/en/test.ftl", FTL),
    ])
}

fn errors(src: &[Source]) -> Vec<String> {
    match load(src) {
        Ok((_, issues)) => issues
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .map(|i| i.to_string())
            .collect(),
        Err(issues) => issues
            .into_iter()
            .filter(|i| i.severity == Severity::Error)
            .map(|i| i.to_string())
            .collect(),
    }
}

/// The sample families with one family's JSON edited.
fn edited(f: impl Fn(&mut serde_json::Value)) -> Vec<String> {
    let json = mandate_content::jsonc::to_strict_json(FAMILIES);
    let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
    f(&mut v["items"][0]);
    let text = serde_json::to_string(&v).unwrap();
    errors(&with(&[
        ("zz_test/policy_families/test.jsonc", &text),
        ("zz_test/locale/en/test.ftl", FTL),
    ]))
}

fn has(errs: &[String], needle: &str) -> bool {
    errs.iter().any(|e| e.contains(needle))
}

fn loaded() -> ContentBundle {
    load(&sample()).unwrap_or_else(|e| panic!("{e:#?}")).0
}

#[test]
fn base_pack_has_family_params_and_the_first_pack() {
    let (b, _) = load(&base_sources()).unwrap();
    // The first pack (F4, M6_ENGINE_CONTRACTS §2.21): 15–20 families, each
    // of a tier whose offices come from the constitution (DEC-098).
    let n = b.policy_families.len();
    assert!((15..=20).contains(&n), "{n} families");
    assert!(
        b.policy_families
            .iter()
            .all(|f| f.id.starts_with("family."))
    );
    assert_eq!(b.policy_family_params.max_open_per_jurisdiction, 4);
    assert_eq!(b.policy_family_params.rarity.of(Rarity::Common), 0.85);
    // Place kinds carry their sector drivers.
    use mandate_content::SectorDriver as D;
    assert_eq!(
        b.place_kind("factory").unwrap().drivers,
        vec![D::Industrial]
    );
    assert_eq!(b.place_kind("mill").unwrap().drivers, vec![D::Industrial]);
    assert_eq!(b.place_kind("mine").unwrap().drivers, vec![D::Extraction]);
}

#[test]
fn sample_families_validate() {
    let b = loaded();
    assert_eq!(
        b.policy_families
            .iter()
            .filter(|f| f.id.starts_with("family.test_"))
            .count(),
        3
    );
    let f = b.family("family.test_flood_wall").unwrap();
    assert_eq!(f.max_instances, 2);
    assert_eq!(f.axes.len(), 2);
    assert_eq!(
        f.base.mechanisms[0],
        MechanismDef::HazardProtection {
            hazard: HazardRef::Slot("@hazard".into()),
            param: "height".into(),
            per_unit: 0.25,
        }
    );
    // Round trip through serde.
    let s = serde_json::to_string(f).unwrap();
    let back: PolicyFamilyDef = serde_json::from_str(&s).unwrap();
    assert_eq!(&back, f);
}

#[test]
fn old_hazard_spelling_parses() {
    let m: MechanismDef = serde_json::from_str(
        r#"{ "type": "hazard_protection", "hazard": "flood", "param": "h", "per_unit": 0.1 }"#,
    )
    .unwrap();
    assert!(matches!(
        m,
        MechanismDef::HazardProtection {
            hazard: HazardRef::Key(HazardKey::Flood),
            ..
        }
    ));
}

#[test]
fn item1_ids() {
    let e = edited(|f| f["id"] = "test_flood".into());
    assert!(has(&e, "family.<name>"), "{e:#?}");
    let e = edited(|f| f["slots"][1]["id"] = "hood".into());
    assert!(has(&e, "duplicate slot id `hood`"), "{e:#?}");
    let e = edited(|f| {
        f["axes"][1]["options"][0]["params"] = serde_json::json!([
            { "id": "height", "name_key": "param-test-wall-height", "min": 1, "max": 3, "default": 2, "step": 1, "unit_key": "unit-test-level" }
        ])
    });
    assert!(has(&e, "duplicate parameter id `height`"), "{e:#?}");
}

#[test]
fn item2_bindings_name_real_things() {
    let e =
        edited(|f| f["slots"][2]["bind"]["occupations"] = serde_json::json!(["occupation.wizard"]));
    assert!(has(&e, "unknown occupation"), "{e:#?}");
    let e = edited(|f| f["slots"][1]["bind"]["in_slot"] = "residents".into());
    assert!(has(&e, "later slot"), "{e:#?}");
    // A required slot never depends on an optional one.
    let e = edited(|f| {
        f["slots"][3]["bind"] = serde_json::json!({ "type": "person", "in_slot": "hood" });
        f["slots"][0]["optional"] = true.into();
    });
    assert!(has(&e, "depends on optional slot"), "{e:#?}");
}

#[test]
fn item3_slot_references() {
    let e = edited(|f| f["base"]["mechanisms"][0]["hazard"] = "@hood".into());
    assert!(has(&e, "hazard slot"), "{e:#?}");
}

#[test]
fn item4_triggers() {
    // A family condition on a template's unlock is refused.
    let unlock = r#"{ "kind": "policies", "items": [ {
        "id": "policy.zz_unlock", "name_key": "family-test-flood-wall", "desc_key": "family-test-flood-wall",
        "tier": "local", "category": "environment", "params": [], "cost": {},
        "mechanisms": [], "implementation_months": 1, "signature": {}, "secondary_keys": [],
        "requires": "propose_ordinance",
        "unlock": { "any": [ { "type": "place_open", "slot": "plant" } ], "reason_key": "x" } } ] }"#;
    let e = errors(&with(&[("zz_test/policies.jsonc", unlock)]));
    assert!(has(&e, "policy family's bound slot"), "{e:#?}");
    // Slot conditions name a slot of the right kind.
    let e = edited(|f| f["trigger"]["any"][1]["slot"] = "hazard".into());
    assert!(has(&e, "needs a neighborhood slot"), "{e:#?}");
    // A regional family may use tier-generic conditions only (DEC-098).
    let e = edited(|f| f["tier"] = "regional".into());
    assert!(has(&e, "tier-generic"), "{e:#?}");
    let e = edited(|f| {
        f["tier"] = "regional".into();
        f["trigger"] = serde_json::json!({ "any": [ { "type": "months_in_play", "min": 1 } ], "reason_key": "reason-test-flood-risk" });
    });
    assert!(!has(&e, "tier-generic"), "{e:#?}");
    // At least one condition.
    let e = edited(|f| f["trigger"]["any"] = serde_json::json!([]));
    assert!(has(&e, "at least one condition"), "{e:#?}");
    // Knowledge never decides (DEC-091).
    let e = edited(|f| {
        f["trigger"]["any"][0] =
            serde_json::json!({ "type": "knows", "subject": { "type": "broker" } })
    });
    assert!(has(&e, "DEC-091"), "{e:#?}");
}

#[test]
fn item5_axes() {
    let e = edited(|f| f["axes"][0]["default"] = "moon".into());
    assert!(has(&e, "default `moon`"), "{e:#?}");
    let e = edited(|f| f["axes"][1]["options"][0]["area"] = serde_json::json!({ "type": "town" }));
    assert!(has(&e, "at most one axis"), "{e:#?}");
    let e = edited(|f| {
        f["axes"][1]["options"][0]["excludes"] =
            serde_json::json!(["where.hood", "where.riverfront"])
    });
    assert!(has(&e, "excludes every option"), "{e:#?}");
    let e =
        edited(|f| f["axes"][1]["options"][0]["excludes"] = serde_json::json!(["where.nowhere"]));
    assert!(has(&e, "excludes unknown"), "{e:#?}");
}

#[test]
fn item6_every_option_is_a_trade_off() {
    // The contractor without its grievance is only faster and dearer: still
    // a trade-off. Make it cheaper too and it dominates the works team.
    let e = edited(|f| {
        let c = &mut f["axes"][1]["options"][1];
        c["cost_factor"] = 0.9.into();
        c["side_effects"] = serde_json::json!([]);
    });
    assert!(has(&e, "better than `works_team` in every way"), "{e:#?}");
    let e = edited(|f| {
        f["axes"][1]["options"][1] = serde_json::json!({
            "id": "contractor", "name_key": "opt-test-contractor", "desc_key": "opt-test-contractor-desc" });
    });
    assert!(has(&e, "identical"), "{e:#?}");
}

#[test]
fn item7_sponsors() {
    let e = edited(|f| f["sponsors"][0]["slot"] = "hood".into());
    assert!(has(&e, "not a person"), "{e:#?}");
    let e = edited(|f| f["sponsors"][0]["preset"]["where"] = "moon".into());
    assert!(has(&e, "unknown option `where.moon`"), "{e:#?}");
    let e = edited(|f| f["sponsors"][1]["price"]["count"] = 5.into());
    assert!(has(&e, "favour count"), "{e:#?}");
    let e = edited(|f| f["sponsors"][0]["params"]["height"] = 9.into());
    assert!(has(&e, "out of range"), "{e:#?}");
}

#[test]
fn item8_knowledge_hooks() {
    let e = edited(|f| f["knowledge"]["heard"] = serde_json::json!([]));
    assert!(has(&e, "heard"), "{e:#?}");
    let e = edited(|f| f["knowledge"]["known"][0]["slot"] = "hood".into());
    assert!(has(&e, "person slot"), "{e:#?}");
    let e = edited(|f| f["knowledge"]["detailed"][0]["service"] = "oracle".into());
    assert!(has(&e, "unknown service"), "{e:#?}");
}

#[test]
fn item9_fluent_keys_and_arguments() {
    let e = edited(|f| f["name_key"] = "family-test-missing".into());
    assert!(
        has(&e, "missing Fluent key `family-test-missing`"),
        "{e:#?}"
    );
    // `$hazard_key` is the hazard slot's argument; `$hazard` is not one.
    let ftl = FTL.replace("against { $hazard_key }", "against { $hazard }");
    let e = errors(&with(&[
        ("zz_test/policy_families/test.jsonc", FAMILIES),
        ("zz_test/locale/en/test.ftl", &ftl),
    ]));
    assert!(has(&e, "`{ $hazard }`"), "{e:#?}");
}

#[test]
fn item10_draw_weight() {
    let e = edited(|f| f["draw"]["weight"] = 0.into());
    assert!(has(&e, "draw weight"), "{e:#?}");
    let e = edited(|f| f["draw"]["weight"] = 11.into());
    assert!(has(&e, "draw weight"), "{e:#?}");
}

#[test]
fn family_mechanisms_are_declared_and_refused_until_simulated() {
    let m: MechanismDef = serde_json::from_str(r#"{ "type": "borrow", "share": 1.0 }"#).unwrap();
    assert_eq!(m, MechanismDef::Borrow { share: 1.0 });
    let e = edited(|f| {
        f["axes"][1]["options"][0]["mechanisms"] =
            serde_json::json!([{ "type": "borrow", "share": 1.0 }])
    });
    let refused = has(&e, "not yet simulated");
    assert_eq!(
        refused,
        !mandate_content::validate::FAMILY_MECHANISMS_SIMULATED,
        "{e:#?}"
    );
}

#[test]
fn family_conditions_parse_and_are_local_only() {
    let c: Condition = serde_json::from_str(
        r#"{ "type": "hood_concern", "slot": "hood", "concern": "concern.jobs" }"#,
    )
    .unwrap();
    assert_eq!(c.tier(), mandate_content::condition::CondTier::Local);
    assert_eq!(
        c.family_slot().map(|(s, k)| (s, k.label())),
        Some(("hood", "neighborhood"))
    );
}

#[test]
fn patches_apply_in_pack_order() {
    let patches = r#"{ "kind": "policy_family_patches", "items": [
        { "op": "set_draw", "family": "family.test_flood_wall", "draw": { "rarity": "rare", "weight": 5 } },
        { "op": "remove_sponsor", "family": "family.test_flood_wall", "sponsor": "builders" },
        { "op": "remove_option", "family": "family.test_plant_rescue", "axis": "pay", "option": "levy" },
        { "op": "add_option", "family": "family.test_plant_rescue", "axis": "pay",
          "option": { "id": "loan", "name_key": "opt-test-pay-bond", "desc_key": "opt-test-pay-bond-desc",
                      "cost_factor": 1.1, "effect_factor": 1.1 } },
        { "op": "disable", "family": "family.test_plant_rescue" }
    ] }"#;
    let src = with(&[
        ("zz_test/policy_families/test.jsonc", FAMILIES),
        ("zz_test/locale/en/test.ftl", FTL),
        ("zz_patch/policy_families/patch.jsonc", patches),
    ]);
    let (b, _) = load(&src).unwrap_or_else(|e| panic!("{e:#?}"));
    let f = b.family("family.test_flood_wall").unwrap();
    assert_eq!(f.draw.rarity, Rarity::Rare);
    assert_eq!(f.draw.weight, 5.0);
    assert!(f.sponsor("builders").is_none());
    let p = b.family("family.test_plant_rescue").unwrap();
    assert!(p.disabled);
    assert!(p.axis("pay").unwrap().option("loan").is_some());
    assert!(p.axis("pay").unwrap().option("levy").is_none());
    // A patch for a family no pack defines is an error.
    let bad = r#"{ "kind": "policy_family_patches", "items": [ { "op": "disable", "family": "family.nope" } ] }"#;
    let e = errors(&with(&[("zz_patch/policy_family_patches.jsonc", bad)]));
    assert!(has(&e, "unknown family `family.nope`"), "{e:#?}");
    // The patch type is the format's.
    let p: FamilyPatch =
        serde_json::from_str(r#"{ "op": "disable", "family": "family.x" }"#).unwrap();
    assert_eq!(p.family(), "family.x");
}

#[test]
fn retired_and_hidden_default_off() {
    let (b, _) = load(&base_sources()).unwrap();
    // The trim and the conversions (§2.19, §2.20) retire seven templates;
    // the standard menu keeps eight general levers.
    let retired: Vec<&str> = b
        .policies
        .values()
        .filter(|t| t.retired)
        .map(|t| t.id.as_str())
        .collect();
    assert_eq!(
        retired,
        vec![
            "policy.closure_response",
            "policy.emergency_rent",
            "policy.fiscal_bridge",
            "policy.flood_defenses",
            "policy.local_wage_floor",
            "policy.rebuild_fund",
            "policy.tech_incubator",
        ]
    );
    assert_eq!(b.policies.values().filter(|t| !t.retired).count(), 8);
    assert!(
        b.policies
            .values()
            .filter(|t| !t.retired)
            .all(|t| t.unlock.is_none())
    );
    assert!(
        b.policies
            .values()
            .flat_map(|t| &t.params)
            .all(|p| !p.hidden)
    );
}
