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

    // Every referenced Fluent key must exist in English (the reference locale).
    let en = fluent_keys(&b.locale_text("en"));
    let mut referenced: Vec<(&str, &str)> = Vec::new();
    for p in &b.packs {
        referenced.push((&p.id, &p.name_key));
    }
    for s in b.sectors.values() {
        referenced.push((&s.id, &s.name_key));
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
