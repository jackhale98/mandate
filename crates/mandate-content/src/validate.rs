//! Content validation rules (DEC-053). Run by `mandate-cli content validate`
//! in CI and on every load.

use crate::bundle::ContentBundle;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Issue {
    pub severity: Severity,
    pub file: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub message: String,
}

impl Issue {
    pub fn error(file: &str, message: &str) -> Self {
        Issue {
            severity: Severity::Error,
            file: file.into(),
            line: None,
            column: None,
            message: message.into(),
        }
    }
    pub fn warning(file: &str, message: &str) -> Self {
        Issue {
            severity: Severity::Warning,
            file: file.into(),
            line: None,
            column: None,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        match (self.line, self.column) {
            (Some(l), Some(c)) => write!(f, "{sev}: {}:{l}:{c}: {}", self.file, self.message),
            _ => write!(f, "{sev}: {}: {}", self.file, self.message),
        }
    }
}

/// Fluent message identifiers defined in a language's `.ftl` sources.
pub fn fluent_keys(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let first = line.chars().next()?;
            if !first.is_ascii_alphabetic() {
                return None;
            }
            let (id, _) = line.split_once('=')?;
            let id = id.trim();
            id.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                .then(|| id.to_string())
        })
        .collect()
}

fn valid_key(id: &str) -> bool {
    !id.is_empty()
        && id.split('.').all(|seg| {
            !seg.is_empty()
                && seg
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
}

/// Semantic validation over a loaded bundle.
pub fn validate_bundle(b: &ContentBundle) -> Vec<Issue> {
    let mut issues = Vec::new();
    if b.packs.is_empty() {
        issues.push(Issue::error("<content>", "no pack manifest found"));
    }

    let t = &b.tuning;
    let range = |issues: &mut Vec<Issue>, name: &str, v: u32, lo: u32, hi: u32| {
        if v < lo || v > hi {
            issues.push(Issue::error(
                "tuning",
                &format!("{name} = {v} outside [{lo}, {hi}]"),
            ));
        }
    };
    range(&mut issues, "standard_slots", t.standard_slots.into(), 1, 6);
    range(
        &mut issues,
        "turn_days.campaign",
        t.turn_days.campaign.into(),
        1,
        31,
    );
    range(
        &mut issues,
        "turn_days.final_stretch",
        t.turn_days.final_stretch.into(),
        0,
        14,
    );
    range(
        &mut issues,
        "turn_days.governing_local",
        t.turn_days.governing_local.into(),
        1,
        62,
    );
    range(
        &mut issues,
        "turn_days.governing_regional",
        t.turn_days.governing_regional.into(),
        1,
        62,
    );
    range(
        &mut issues,
        "turn_days.governing_national",
        t.turn_days.governing_national.into(),
        1,
        62,
    );
    range(&mut issues, "burn_in_years", t.burn_in_years.into(), 0, 60);

    for id in b
        .sectors
        .keys()
        .chain(b.name_pools.keys())
        .chain(b.packs.iter().map(|p| &p.id))
    {
        if !valid_key(id) {
            issues.push(Issue::error(
                id,
                "content id must be lowercase dot-separated segments of [a-z0-9_]",
            ));
        }
    }
    for s in b.sectors.values() {
        if !(s.base_weight.is_finite() && s.base_weight >= 0.0) {
            issues.push(Issue::error(&s.id, "base_weight must be finite and >= 0"));
        }
    }
    for kind in crate::types::SectorKind::ALL {
        let n = b.sectors.values().filter(|s| s.kind == kind).count();
        if n != 1 {
            issues.push(Issue::error(
                "sectors",
                &format!("expected exactly one sector of kind {kind:?}, found {n}"),
            ));
        }
    }
    let e = &b.economy;
    let unit = |issues: &mut Vec<Issue>, name: &str, v: f64| {
        if !(0.0..=1.0).contains(&v) {
            issues.push(Issue::error(
                "economy",
                &format!("{name} = {v} outside [0, 1]"),
            ));
        }
    };
    unit(&mut issues, "natural_unemployment", e.natural_unemployment);
    unit(
        &mut issues,
        "recession_monthly_chance",
        e.recession_monthly_chance,
    );
    unit(&mut issues, "migration_rate", e.migration_rate);
    unit(&mut issues, "target_vacancy", e.target_vacancy);
    for (i, r) in e.death_rates.iter().enumerate() {
        unit(&mut issues, &format!("death_rates[{i}]"), *r);
    }
    for (i, r) in e.taxes.income.iter().enumerate() {
        unit(&mut issues, &format!("taxes.income[{i}]"), *r);
    }
    if e.recession_depth[0] > e.recession_depth[1] || e.recession_months[0] > e.recession_months[1]
    {
        issues.push(Issue::error(
            "economy",
            "recession ranges must be [min, max]",
        ));
    }
    for n in b.name_pools.values() {
        if n.prefixes.is_empty() || n.suffixes.is_empty() {
            issues.push(Issue::error(
                &n.id,
                "name pools need at least one prefix and one suffix",
            ));
        }
    }

    // Constitutions: every procedure stage references a real body or office,
    // and the local tier can propose and approve ordinances (playability).
    for c in b.constitutions.values() {
        let Some(local) = c.tier(crate::types::TierKey::Local) else {
            issues.push(Issue::error(&c.id, "constitution has no local tier"));
            continue;
        };
        for t in &c.tiers {
            for p in &t.procedures {
                if p.stages.is_empty() {
                    issues.push(Issue::error(
                        &c.id,
                        &format!("procedure `{}` has no stages", p.id),
                    ));
                }
                for st in &p.stages {
                    match st {
                        crate::types::StageDef::Vote { body, .. } => {
                            if t.body(body).is_none() {
                                issues.push(Issue::error(
                                    &c.id,
                                    &format!("procedure `{}` votes in unknown body `{body}`", p.id),
                                ));
                            }
                        }
                        crate::types::StageDef::Sign {
                            office,
                            override_body,
                            ..
                        } => {
                            if t.office(office).is_none() {
                                issues.push(Issue::error(
                                    &c.id,
                                    &format!(
                                        "procedure `{}` needs unknown office `{office}`",
                                        p.id
                                    ),
                                ));
                            }
                            if let Some(ob) = override_body
                                && t.body(ob).is_none()
                            {
                                issues.push(Issue::error(
                                    &c.id,
                                    &format!("override body `{ob}` unknown"),
                                ));
                            }
                        }
                    }
                }
            }
        }
        for needed in ["ordinance", "budget"] {
            if local.procedure(needed).is_none() {
                issues.push(Issue::error(
                    &c.id,
                    &format!("local tier lacks the `{needed}` procedure"),
                ));
            }
        }
        if local
            .holder_of(crate::types::Power::ProposeOrdinance)
            .is_none()
        {
            issues.push(Issue::error(
                &c.id,
                "no local holder of propose_ordinance (unplayable)",
            ));
        }
    }
    if b.constitutions.is_empty() {
        issues.push(Issue::error("<content>", "no constitution defined"));
    }
    // Policies: referenced parameters exist; ranges are sane (REQ-006).
    for p in b.policies.values() {
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
            let param = match m {
                M::Service { param, .. }
                | M::Transfer { param, .. }
                | M::HousingSupply { param }
                | M::Zoning { param, .. }
                | M::SectorSubsidy { param, .. }
                | M::WageFloor { param }
                | M::HazardProtection { param, .. }
                | M::LocalTaxRate { param } => param,
            };
            if !has(param) {
                issues.push(Issue::error(
                    &p.id,
                    &format!("mechanism references unknown parameter `{param}`"),
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
    }

    // Every referenced Fluent key must exist in English (the reference locale).
    let en = fluent_keys(&b.locale_text("en"));
    let mut referenced: Vec<(&str, &str)> = Vec::new();
    for p in &b.packs {
        referenced.push((&p.id, &p.name_key));
    }
    for s in b.sectors.values() {
        referenced.push((&s.id, &s.name_key));
    }
    for c in b.constitutions.values() {
        referenced.push((&c.id, &c.name_key));
        for t in &c.tiers {
            for x in &t.bodies {
                referenced.push((&c.id, &x.name_key));
            }
            for x in &t.offices {
                referenced.push((&c.id, &x.name_key));
            }
        }
    }
    for p in b.policies.values() {
        referenced.push((&p.id, &p.name_key));
        referenced.push((&p.id, &p.desc_key));
        for k in &p.secondary_keys {
            referenced.push((&p.id, k));
        }
        for prm in &p.params {
            referenced.push((&p.id, &prm.name_key));
            referenced.push((&p.id, &prm.unit_key));
        }
    }
    for (owner, key) in referenced {
        if !en.contains(key) {
            issues.push(Issue::error(
                owner,
                &format!("missing Fluent key `{key}` in locale en"),
            ));
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fluent_ids() {
        let keys =
            fluent_keys("# c\nhello = Hi\n  .attr = x\nsector-manufacturing = Manufacturing\n");
        assert!(keys.contains("hello") && keys.contains("sector-manufacturing"));
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn content_key_rules() {
        assert!(valid_key("sector.manufacturing"));
        assert!(!valid_key("Sector.x"));
        assert!(!valid_key("a..b"));
    }
}
