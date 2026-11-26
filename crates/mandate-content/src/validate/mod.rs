//! Content validation rules (DEC-053). Run by `mandate-cli content validate`
//! in CI and on every load.

use crate::bundle::ContentBundle;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod characters;
mod concerns;
mod constitution;
mod economy;
mod fluent;
mod places;
mod policy;
mod scenarios;
mod tuning;

use characters::*;
use concerns::*;
use constitution::*;
use economy::*;
pub use fluent::*;
use places::*;
pub use policy::*;
use scenarios::*;
pub use tuning::*;

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

/// Whether the family mechanisms (`borrow`, `revenue`, `sector_capacity`)
/// are simulated: content may use them (F2b).
pub const FAMILY_MECHANISMS_SIMULATED: bool = true;

pub(crate) fn valid_key(id: &str) -> bool {
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

    validate_tuning(b, &mut issues);

    for id in b
        .sector_keys()
        .chain(b.industry_keys())
        .chain(b.markets.iter().map(|x| &x.id))
        .chain(b.goods.iter().map(|x| &x.id))
        .chain(
            b.industries
                .iter()
                .flat_map(|d| d.products.iter().map(|p| &p.id)),
        )
        .chain(b.scenarios.iter().map(|x| &x.id))
        .chain(b.concern_keys())
        .chain(b.attitudes.iter().map(|a| &a.id))
        .chain(b.community_keys())
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
    validate_sectors(b, &mut issues);
    validate_industries(b, &mut issues);
    validate_attitudes(b, &mut issues);
    validate_places(b, &mut issues);
    crate::layout::validate_layout(b, &mut issues);
    crate::map_layers::validate(b, &mut issues);
    crate::town_map_layers::validate(b, &mut issues);
    crate::interiors::validate(b, &mut issues);
    crate::routines::validate(b, &mut issues);
    crate::calendar::validate(b, &mut issues);
    crate::faults::validate(b, &mut issues);
    validate_communities(b, &mut issues);
    validate_scenarios(b, &mut issues);
    validate_concerns(b, &mut issues);
    validate_characters(b, &mut issues);
    crate::narrative::validate(b, &mut issues);
    crate::media::validate(b, &mut issues);
    crate::knowledge::validate(b, &mut issues);
    crate::visits::validate(b, &mut issues);
    crate::verb_talks::validate(b, &mut issues);
    crate::conversation::validate(b, &mut issues);
    crate::asks::validate(b, &mut issues);
    crate::arcs::validate(b, &mut issues);
    crate::act::validate(b, &mut issues);
    validate_param_bounds(b, &mut issues);
    validate_economy(b, &mut issues);

    validate_constitutions(b, &mut issues);
    validate_nomination(b, &mut issues);
    validate_policies(b, &mut issues);
    validate_treasury_shares(b, &mut issues);
    // Policy families (DEC-092): items 1–10 of the contract's §2.13.
    crate::policy_family::validate(b, &mut issues);
    // The impact table (DEC-111): no mechanism without a row.
    crate::types::validate_impact(b, &mut issues);
    validate_overheard(b, &mut issues);

    validate_fluent_refs(b, &mut issues);
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
