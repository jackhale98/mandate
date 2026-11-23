//! Place kinds (DEC-079).

use super::*;

/// Place-kind registry (DEC-079): known actions and services, pseudo-locations
/// present, fallbacks resolvable, and every action offered somewhere. That
/// every place kind worldgen can generate has an entry is checked by
/// `mandate_sim::places::registry_issues` (the kinds are a simulation enum).
pub(super) fn validate_places(b: &ContentBundle, issues: &mut Vec<Issue>) {
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
