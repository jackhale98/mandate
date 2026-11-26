//! Concerns (DEC-078 decision 3) and overheard talk (DEC-079).

use super::*;

/// Concern registry (DEC-078 decision 3): at least one attitude per concern,
/// known attitude keys, weights in [-1, 1], and a sane level mapping. An empty
/// registry is allowed (every issue then weighs the same). Ids are unique
/// (claimed on load).
pub(super) fn validate_concerns(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let in_range = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
    for c in &b.concerns {
        if c.attitudes.is_empty() {
            issues.push(Issue::error(&c.id, "a concern needs at least one attitude"));
        }
        for (k, w) in &c.attitudes {
            if b.attitude_index(k).is_none() {
                issues.push(Issue::error(
                    &c.id,
                    &format!("attitudes references unknown attitude `{k}`"),
                ));
            }
            if !in_range(*w, -1.0, 1.0) || *w == 0.0 {
                issues.push(Issue::error(
                    &c.id,
                    &format!("attitude weight for `{k}` must be non-zero and in [-1, 1]"),
                ));
            }
        }
        let l = &c.level;
        for (name, v, lo, hi) in [
            ("level.baseline", l.baseline, -10.0, 10.0),
            ("level.slope", l.slope, -1000.0, 1000.0),
            ("level.min", l.min, 0.0, 10.0),
            ("level.max", l.max, 0.0, 10.0),
        ] {
            if !in_range(v, lo, hi) {
                issues.push(Issue::error(
                    &c.id,
                    &format!("{name} = {v} outside [{lo}, {hi}]"),
                ));
            }
        }
        for t in &c.also {
            if !in_range(t.slope, -1000.0, 1000.0) {
                issues.push(Issue::error(
                    &c.id,
                    &format!("also: slope {} outside [-1000, 1000]", t.slope),
                ));
            }
            if t.indicator == c.indicator
                || c.also.iter().filter(|x| x.indicator == t.indicator).count() > 1
            {
                issues.push(Issue::error(
                    &c.id,
                    &format!("also: {:?} is read twice", t.indicator),
                ));
            }
        }
        if l.min > l.max {
            issues.push(Issue::error(&c.id, "level.min must not exceed level.max"));
        }
    }
    // Neighbourhood concerns: workplaces are place kinds; a driver's kind
    // lean is given once.
    let lc = &b.opinion.local_concerns;
    for k in lc.workplaces.keys() {
        if b.place_kind(k).is_none() {
            issues.push(Issue::error(
                "opinion",
                &format!("local_concerns.workplaces names unknown place kind `{k}`"),
            ));
        }
    }
    for (i, s) in lc.sector_kinds.iter().enumerate() {
        if lc.sector_kinds[..i].iter().any(|x| x.driver == s.driver) {
            issues.push(Issue::error(
                "opinion",
                &format!("local_concerns.sector_kinds lists {:?} twice", s.driver),
            ));
        }
    }
}

/// Overheard talk: lines name real concerns and crises.
pub(super) fn validate_overheard(b: &ContentBundle, issues: &mut Vec<Issue>) {
    // Overheard talk: lines name real concerns and crises.
    for o in &b.overheard {
        if !o.id.starts_with("overheard.") {
            issues.push(Issue::error(
                &o.id,
                "overheard ids look like `overheard.name`",
            ));
        }
        if !(o.weight.is_finite() && o.weight > 0.0 && o.weight <= 10.0) {
            issues.push(Issue::error(&o.id, "overheard weight must be in (0, 10]"));
        }
        if let Some(c) = &o.concern
            && !b.concerns.iter().any(|x| &x.id == c)
        {
            issues.push(Issue::warning(
                &o.id,
                &format!("unknown concern `{c}` (the line is never heard)"),
            ));
        }
        crate::condition::check_all(
            b,
            &crate::condition::Site::new(&o.id, crate::condition::ConditionUse::Overheard, &[]),
            &o.when,
            issues,
        );
        // What a line teaches (DEC-091): lines have no cast, so no roles.
        for t in &o.teaches {
            if t.role().is_some() || matches!(t, crate::narrative::SubjectDef::Person { .. }) {
                issues.push(Issue::error(
                    &o.id,
                    &format!("overheard lines teach without roles (`{}`)", t.key()),
                ));
            }
            crate::narrative::check_subject(&o.id, b, t, issues);
        }
    }
}
