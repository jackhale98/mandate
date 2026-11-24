//! Policy templates (REQ-006, DEC-073) and situation triggers (DEC-077).

use super::*;

/// Policies: referenced parameters exist; ranges are sane (REQ-006).
pub(super) fn validate_policies(b: &ContentBundle, issues: &mut Vec<Issue>) {
    // Policies: referenced parameters exist; ranges are sane (REQ-006).
    for p in b.policies.values() {
        for (k, v) in &p.signature {
            if b.attitude_index(k).is_none() {
                issues.push(Issue::error(
                    &p.id,
                    &format!("signature references unknown attitude `{k}`"),
                ));
            }
            if !(v.is_finite() && (-1.0..=1.0).contains(v)) {
                issues.push(Issue::error(
                    &p.id,
                    &format!("signature weight for `{k}` must be in [-1, 1]"),
                ));
            }
        }
        let has = |id: &str| p.params.iter().any(|x| x.id == id);
        for prm in &p.params {
            if !(prm.min <= prm.default && prm.default <= prm.max && prm.step > 0.0) {
                issues.push(Issue::error(
                    &p.id,
                    &format!("parameter `{}` has an invalid range", prm.id),
                ));
            }
        }
        use crate::types::MechanismDef as M;
        for m in &p.mechanisms {
            if let Some(param) = m.param()
                && !has(param)
            {
                issues.push(Issue::error(
                    &p.id,
                    &format!("mechanism references unknown parameter `{param}`"),
                ));
            }
            // Slot references (`@slot`) belong to policy families.
            if let M::HazardProtection {
                hazard: crate::types::HazardRef::Slot(h),
                ..
            } = m
            {
                issues.push(Issue::error(
                    &p.id,
                    &format!("hazard `{h}` is a slot reference; templates name a hazard"),
                ));
            }
            if crate::policy_family::is_family_mechanism(m) && !FAMILY_MECHANISMS_SIMULATED {
                issues.push(Issue::error(
                    &p.id,
                    &format!(
                        "mechanism `{}` is declared but not yet simulated (F2b)",
                        m.key()
                    ),
                ));
            }
            if let M::SectorSubsidy { sector, .. } = m
                && b.sector_index(sector).is_none()
            {
                issues.push(Issue::error(
                    &p.id,
                    &format!("mechanism references unknown sector `{sector}`"),
                ));
            }
        }
        if let Some(cp) = &p.cost.param
            && !has(cp)
        {
            issues.push(Issue::error(
                &p.id,
                &format!("cost references unknown parameter `{cp}`"),
            ));
        }
        if p.mechanisms.is_empty() {
            issues.push(Issue::error(&p.id, "policy needs at least one mechanism"));
        }
        for fx in &p.side_effects {
            if let Some(param) = fx.param()
                && !has(param)
            {
                issues.push(Issue::error(
                    &p.id,
                    &format!("side effect references unknown parameter `{param}`"),
                ));
            }
        }
        // DEC-073: every decision is a trade-off. Money alone is not enough: a
        // policy needs a simulated downside (a side effect, or a mechanism with
        // one built in, such as a wage floor's job losses).
        let inherent = p
            .mechanisms
            .iter()
            .any(|m| matches!(m, M::WageFloor { .. }));
        if p.side_effects.is_empty() && !inherent {
            issues.push(Issue::error(
                &p.id,
                "policy has no simulated trade-off: add a side effect (DEC-073)",
            ));
        }
        if p.secondary_keys.is_empty() {
            issues.push(Issue::error(
                &p.id,
                "policy needs at least one secondary effect (REQ-006)",
            ));
        }
        // No secondary line without a mechanism or a measured side effect
        // behind it (audit fix 8).
        for k in &p.secondary_keys {
            match p.secondary_backing.get(k) {
                None => issues.push(Issue::error(
                    &p.id,
                    &format!("secondary line `{k}` names nothing that backs it (secondary_backing)"),
                )),
                Some(by) if !backs(p, by) => issues.push(Issue::error(
                    &p.id,
                    &format!(
                        "secondary line `{k}` is backed by `{by}`: not a mechanism or side effect of this policy, `cost`, or `measured:<metric>`"
                    ),
                )),
                Some(_) => {}
            }
        }
        for k in p.secondary_backing.keys() {
            if !p.secondary_keys.contains(k) {
                issues.push(Issue::error(
                    &p.id,
                    &format!("secondary_backing names `{k}`, which is not a secondary line"),
                ));
            }
        }
        // Situational policies (DEC-077 §3): conditions name real things.
        if let Some(u) = &p.unlock {
            check_situation(b, &p.id, p.tier, u, issues);
        }
    }
}

/// Does `by` back a secondary line of `p`: one of its mechanisms or side
/// effects by type, its `cost`, or a metric the audit measures?
fn backs(p: &crate::PolicyTemplate, by: &str) -> bool {
    if by == "cost" {
        return true;
    }
    if let Some(metric) = by.strip_prefix("measured:") {
        return crate::types::SECONDARY_MEASURED.contains(&metric);
    }
    p.mechanisms.iter().any(|m| m.key() == by) || p.side_effects.iter().any(|f| f.key() == by)
}

/// A situation trigger (DEC-077 §3, reused by policy families): at least one
/// condition, each checked by the one condition validator as a policy's
/// unlock (no slots; the ledger refused, DEC-091; a non-local template uses
/// tier-generic conditions only, DEC-098).
pub fn check_situation(
    b: &ContentBundle,
    owner: &str,
    tier: crate::types::TierKey,
    u: &crate::types::SituationTrigger,
    issues: &mut Vec<Issue>,
) {
    if u.any.is_empty() {
        issues.push(Issue::error(
            owner,
            "a situation trigger needs at least one condition",
        ));
    }
    let site =
        crate::condition::Site::new(owner, crate::condition::ConditionUse::PolicyUnlock, &[])
            .tier(tier);
    crate::condition::check_all(b, &site, &u.any, issues);
}
