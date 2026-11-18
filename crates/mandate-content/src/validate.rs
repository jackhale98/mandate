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
        .sector_keys()
        .chain(b.attitudes.iter().map(|a| &a.id))
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
    // Sector registry (DEC-075): sane traits, and every driver the base
    // economy needs has at least one sector. Ids are unique (claimed on load).
    for s in &b.sectors {
        if !(s.base_weight.is_finite() && s.base_weight >= 0.0) {
            issues.push(Issue::error(&s.id, "base_weight must be finite and >= 0"));
        }
        if !(s.base_wage.is_finite() && s.base_wage > 0.0) {
            issues.push(Issue::error(&s.id, "base_wage must be finite and > 0"));
        }
        if !(s.cyclicality.is_finite() && (0.0..=3.0).contains(&s.cyclicality)) {
            issues.push(Issue::error(&s.id, "cyclicality must be in [0, 3]"));
        }
        if !(s.productivity_growth.is_finite() && (-0.1..=0.2).contains(&s.productivity_growth)) {
            issues.push(Issue::error(
                &s.id,
                "productivity_growth must be in [-0.1, 0.2]",
            ));
        }
    }
    for driver in crate::types::SectorDriver::REQUIRED {
        if b.sectors_with(driver).next().is_none() {
            issues.push(Issue::error(
                "sectors",
                &format!("the economy needs at least one sector with driver {driver:?}"),
            ));
        }
    }
    // Attitude registry (DEC-075): at least one, unique short keys usable in
    // Fluent ids, and finite seeding parameters. Ids are unique (claimed on load).
    if b.attitudes.is_empty() {
        issues.push(Issue::error(
            "attitudes",
            "at least one attitude is required",
        ));
    }
    for (i, a) in b.attitudes.iter().enumerate() {
        let short_ok = a.key.starts_with(|c: char| c.is_ascii_lowercase())
            && a.key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !short_ok {
            issues.push(Issue::error(
                &a.id,
                "attitude key must be [a-z][a-z0-9_]* (it is used in Fluent ids)",
            ));
        }
        if b.attitudes[..i].iter().any(|x| x.key == a.key) {
            issues.push(Issue::error(
                &a.id,
                &format!("duplicate attitude key `{}`", a.key),
            ));
        }
        let sd = &a.seed;
        let n = &sd.by_neighborhood;
        let leans = sd.by_class.iter().chain(&sd.by_age).chain([
            &n.core,
            &n.inner,
            &n.waterfront,
            &n.industrial,
            &n.suburb,
            &n.rural,
        ]);
        if leans
            .into_iter()
            .any(|v| !(v.is_finite() && (-2.0..=2.0).contains(v)))
        {
            issues.push(Issue::error(
                &a.id,
                "seed leans must be finite and in [-2, 2]",
            ));
        }
        for (name, v) in [("regional_sd", sd.regional_sd), ("noise_sd", sd.noise_sd)] {
            if !(v.is_finite() && (0.0..=1.0).contains(&v)) {
                issues.push(Issue::error(&a.id, &format!("{name} must be in [0, 1]")));
            }
        }
    }
    for w in &b.politics.party_names.by_attitude {
        if b.attitude_index(&w.attitude).is_none() {
            issues.push(Issue::error(
                "politics.party_names",
                &format!(
                    "party-name words reference unknown attitude `{}`",
                    w.attitude
                ),
            ));
        }
    }
    validate_places(b, &mut issues);
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
    }

    // Every referenced Fluent key must exist in English (the reference locale).
    let en = fluent_keys(&b.locale_text("en"));
    let mut referenced: Vec<(&str, &str)> = Vec::new();
    for p in &b.packs {
        referenced.push((&p.id, &p.name_key));
    }
    for s in &b.sectors {
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
    for p in &b.places {
        referenced.push((&p.id, &p.name_key));
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
    // Attitudes are named in the UI through their conviction keys.
    let convictions: Vec<(&str, String)> = b
        .attitudes
        .iter()
        .flat_map(|a| {
            ["pro", "anti"].map(|side| (a.id.as_str(), format!("conviction-{side}-{}", a.key)))
        })
        .collect();
    for (owner, key) in &convictions {
        referenced.push((owner, key));
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

/// Place-kind registry (DEC-079): known actions and services, pseudo-locations
/// present, fallbacks resolvable, and every action offered somewhere. That
/// every place kind worldgen can generate has an entry is checked by
/// `mandate_sim::places::registry_issues` (the kinds are a simulation enum).
fn validate_places(b: &ContentBundle, issues: &mut Vec<Issue>) {
    use crate::types::{LATE_NIGHT_ACTIONS, PLACE_ACTIONS, PLACE_SERVICES, PSEUDO_PLACES};
    for (i, p) in b.places.iter().enumerate() {
        if !valid_key(&p.id) {
            issues.push(Issue::error(
                &p.id,
                "content id must be lowercase dot-separated segments of [a-z0-9_]",
            ));
        }
        let key_ok = p.key.starts_with(|c: char| c.is_ascii_lowercase())
            && p.key
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !key_ok {
            issues.push(Issue::error(&p.id, "place key must be [a-z][a-z0-9_]*"));
        }
        if b.places[..i].iter().any(|x| x.key == p.key) {
            issues.push(Issue::error(
                &p.id,
                &format!("duplicate place key `{}`", p.key),
            ));
        }
        for a in &p.actions {
            if !PLACE_ACTIONS.contains(&a.as_str()) {
                issues.push(Issue::error(&p.id, &format!("unknown action `{a}`")));
            }
        }
        for a in &p.late_night {
            if !LATE_NIGHT_ACTIONS.contains(&a.as_str()) {
                issues.push(Issue::error(
                    &p.id,
                    &format!("action `{a}` has no late-night variant"),
                ));
            }
            if !p.actions.contains(a) {
                issues.push(Issue::error(
                    &p.id,
                    &format!("late-night action `{a}` must also be offered in `actions`"),
                ));
            }
        }
        for s in &p.services {
            if !PLACE_SERVICES.contains(&s.as_str()) {
                issues.push(Issue::error(&p.id, &format!("unknown service `{s}`")));
            }
        }
        if let Some(f) = &p.fallback {
            if f == &p.key {
                issues.push(Issue::error(&p.id, "a place cannot fall back to itself"));
            } else if b.place_kind(f).is_none() {
                issues.push(Issue::error(
                    &p.id,
                    &format!("fallback `{f}` is not a place kind"),
                ));
            }
        }
    }
    if b.places.is_empty() {
        issues.push(Issue::error("places", "no place kinds defined"));
        return;
    }
    for k in PSEUDO_PLACES {
        if b.place_kind(k).is_none() {
            issues.push(Issue::error(
                "places",
                &format!("the pseudo-location `{k}` needs an entry"),
            ));
        }
    }
    for a in PLACE_ACTIONS {
        if !b.places.iter().any(|p| p.actions.iter().any(|x| x == a)) {
            issues.push(Issue::error(
                "places",
                &format!("no place offers the action `{a}` (unplayable)"),
            ));
        }
    }
    for a in LATE_NIGHT_ACTIONS {
        if !b.places.iter().any(|p| p.late_night.iter().any(|x| x == a)) {
            issues.push(Issue::warning(
                "places",
                &format!("no place offers `{a}` late at night"),
            ));
        }
    }
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
