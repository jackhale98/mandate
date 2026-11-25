//! Characters (DEC-031): memories, traits, occupations, crises.

use super::*;

/// Characters (DEC-031): known memory kinds, each with a parameter row;
/// traits, occupations, and crisis kinds with sane values and known
/// references. Ids are unique (claimed on load).
pub(super) fn validate_characters(b: &ContentBundle, issues: &mut Vec<Issue>) {
    use crate::types::{COMPETENCIES, MEMORY_KINDS, PERSONALITY_AXES};
    let in_range = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
    let ch = &b.characters;
    for o in &ch.broker.press_occupations {
        if b.occupation(o).is_none() {
            issues.push(Issue::error(
                "characters.broker.press_occupations",
                &format!("unknown occupation `{o}`"),
            ));
        }
    }
    for k in MEMORY_KINDS {
        if !ch.memories.contains_key(k) {
            issues.push(Issue::error(
                "characters",
                &format!("memories needs a row for `{k}`"),
            ));
        }
    }
    for k in ch.memories.keys() {
        if !MEMORY_KINDS.contains(&k.as_str()) {
            issues.push(Issue::error(
                "characters",
                &format!(
                    "unknown memory kind `{k}` (known: {})",
                    MEMORY_KINDS.join(", ")
                ),
            ));
        }
    }
    let axes =
        |owner: &str, map: &BTreeMap<String, f64>, known: &[&str], issues: &mut Vec<Issue>| {
            for (k, v) in map {
                if !known.contains(&k.as_str()) {
                    issues.push(Issue::error(
                        owner,
                        &format!("unknown key `{k}` (known: {})", known.join(", ")),
                    ));
                }
                if !in_range(*v, -1.0, 1.0) {
                    issues.push(Issue::error(owner, &format!("`{k}` = {v} outside [-1, 1]")));
                }
            }
        };
    for t in &b.traits {
        if !valid_key(&t.id) {
            issues.push(Issue::error(&t.id, "trait ids look like `trait.name`"));
        }
        if !in_range(t.chance, 0.0, 1.0) {
            issues.push(Issue::error(&t.id, "chance must be in [0, 1]"));
        }
        axes(&t.id, &t.personality, &PERSONALITY_AXES, issues);
        for (name, v) in [
            ("memory_decay.positive", t.memory_decay.positive),
            ("memory_decay.negative", t.memory_decay.negative),
            ("opinion_change.positive", t.opinion_change.positive),
            ("opinion_change.negative", t.opinion_change.negative),
        ] {
            if !in_range(v, 0.0, 10.0) {
                issues.push(Issue::error(
                    &t.id,
                    &format!("{name} = {v} outside [0, 10]"),
                ));
            }
        }
        for x in &t.excludes {
            if b.trait_def(x).is_none() {
                issues.push(Issue::error(
                    &t.id,
                    &format!("excludes unknown trait `{x}`"),
                ));
            }
        }
    }
    let place_keys: Vec<&str> = b.places.iter().map(|p| p.key.as_str()).collect();
    for o in &b.occupations {
        if !valid_key(&o.id) {
            issues.push(Issue::error(
                &o.id,
                "occupation ids look like `occupation.name`",
            ));
        }
        if o.places.is_empty() {
            issues.push(Issue::error(
                &o.id,
                "an occupation needs at least one place",
            ));
        }
        for k in &o.places {
            if k == "home" || !place_keys.contains(&k.as_str()) {
                issues.push(Issue::error(
                    &o.id,
                    &format!("places references unknown place kind `{k}`"),
                ));
            }
        }
        if !in_range(o.chance, 0.0, 1.0) || !in_range(o.influence, 0.0, 1.0) {
            issues.push(Issue::error(
                &o.id,
                "chance and influence must be in [0, 1]",
            ));
        }
        if o.class.iter().any(|w| !in_range(*w, 0.0, 100.0)) || o.class.iter().sum::<f64>() <= 0.0 {
            issues.push(Issue::error(
                &o.id,
                "class weights must be non-negative, not all 0",
            ));
        }
        if o.age[0] < 18 || o.age[0] > o.age[1] || o.age[1] > 100 {
            issues.push(Issue::error(&o.id, "age must be [min, max] within 18..100"));
        }
        axes(&o.id, &o.personality, &PERSONALITY_AXES, issues);
        axes(&o.id, &o.competencies, &COMPETENCIES, issues);
        if let Some(e) = &o.expertise
            && b.attitude_index(e).is_none()
        {
            issues.push(Issue::error(
                &o.id,
                &format!("expertise names unknown attitude `{e}`"),
            ));
        }
        for c in &o.stake.crises {
            if b.crisis_index(c).is_none() {
                issues.push(Issue::error(
                    &o.id,
                    &format!("stake names unknown crisis `{c}`"),
                ));
            }
        }
    }
    for c in &b.crises {
        if !valid_key(&c.id) {
            issues.push(Issue::error(&c.id, "crisis ids look like `crisis.name`"));
        }
        let ok = [c.begin, c.end, c.severe]
            .iter()
            .all(|v| in_range(*v, -1000.0, 1000.0));
        if !ok || c.end > c.begin || c.severe < c.begin {
            issues.push(Issue::error(
                &c.id,
                "thresholds must be finite with end ≤ begin ≤ severe",
            ));
        }
        // The link is informational: a pack that removes the concern keeps
        // a working crisis.
        if let Some(k) = &c.concern
            && b.concern_index(k).is_none()
        {
            issues.push(Issue::warning(
                &c.id,
                &format!("concern names unknown concern `{k}`"),
            ));
        }
        for p in &c.policies {
            if !b.policies.contains_key(p) && b.family(p).is_none() {
                issues.push(Issue::error(
                    &c.id,
                    &format!("policies names unknown policy `{p}`"),
                ));
            }
        }
    }
}
