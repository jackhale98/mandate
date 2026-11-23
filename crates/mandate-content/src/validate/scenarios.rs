//! Scenarios (DEC-078 decision 4).

use super::*;

/// Scenario registry (DEC-078 decision 4): valid ids; biases finite and in
/// range; attitude shifts name known attitudes. Ids are unique (claimed on load).
pub(super) fn validate_scenarios(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let in_range = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
    for x in &b.scenarios {
        let g = &x.geography;
        let mut check = |name: &str, v: f64, lo: f64, hi: f64| {
            if !in_range(v, lo, hi) {
                issues.push(Issue::error(
                    &x.id,
                    &format!("{name} = {v} outside [{lo}, {hi}]"),
                ));
            }
        };
        if let Some(n) = g.sea_sides {
            check("geography.sea_sides", n.into(), 1.0, 3.0);
        }
        if let Some(v) = g.mountain_strength {
            check("geography.mountain_strength", v, 0.0, 1.0);
        }
        check("geography.country_extent", g.country_extent, 0.5, 1.5);
        if let Some(v) = g.wetness {
            check("geography.wetness", v, 0.2, 2.0);
        }
        if let Some(v) = g.temp_south {
            check("geography.temp_south", v, -10.0, 35.0);
        }
        check("geography.mineral_richness", g.mineral_richness, 0.0, 5.0);
        check("geography.population", g.population, 0.1, 3.0);
        let e = &x.economy;
        for (d, v) in &e.endowment {
            check(&format!("economy.endowment.{d:?}"), *v, 0.0, 10.0);
        }
        check("economy.wealth", e.wealth, 0.2, 5.0);
        for (i, v) in e.class_mix.iter().enumerate() {
            check(&format!("economy.class_mix[{i}]"), *v, 0.0, 10.0);
        }
        let era = &x.era;
        for (name, v) in [
            ("era.tech_growth", era.tech_growth),
            ("era.manufacturing_trend", era.manufacturing_trend),
            ("era.drying", era.drying),
            ("era.storminess", era.storminess),
        ] {
            if let Some(v) = v {
                check(name, v, -0.1, 0.1);
            }
        }
        for (k, v) in &x.politics.attitude_shift {
            check(&format!("politics.attitude_shift.{k}"), *v, -1.0, 1.0);
        }
        if e.class_mix.iter().all(|v| *v == 0.0) {
            issues.push(Issue::error(&x.id, "economy.class_mix cannot be all zero"));
        }
        for k in x.politics.attitude_shift.keys() {
            if b.attitude_index(k).is_none() {
                issues.push(Issue::error(
                    &x.id,
                    &format!("politics.attitude_shift references unknown attitude `{k}`"),
                ));
            }
        }
    }
}
