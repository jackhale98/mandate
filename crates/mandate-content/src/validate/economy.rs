//! Economy registries: sectors, attitudes, economy numbers, treasury shares, communities.

use super::*;

/// Sector registry (DEC-075).
pub(super) fn validate_sectors(b: &ContentBundle, issues: &mut Vec<Issue>) {
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
}

/// Attitude registry (DEC-075) and the party-name words that use it.
pub(super) fn validate_attitudes(b: &ContentBundle, issues: &mut Vec<Issue>) {
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
}

/// Economy parameters and name pools.
pub(super) fn validate_economy(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let e = &b.economy;
    let unit = |issues: &mut Vec<Issue>, name: &str, v: f64| {
        if !(0.0..=1.0).contains(&v) {
            issues.push(Issue::error(
                "economy",
                &format!("{name} = {v} outside [0, 1]"),
            ));
        }
    };
    unit(issues, "natural_unemployment", e.natural_unemployment);
    unit(
        issues,
        "recession_monthly_chance",
        e.recession_monthly_chance,
    );
    unit(issues, "migration_rate", e.migration_rate);
    unit(issues, "target_vacancy", e.target_vacancy);
    let h = &e.housing;
    for (name, v) in [
        ("housing.reference_burden", h.reference_burden),
        ("housing.headship_elasticity", h.headship_elasticity),
        ("housing.rent_yield", h.rent_yield),
        ("housing.risk_premium", h.risk_premium),
        ("housing.rent_response", h.rent_response),
        ("housing.price_adjustment", h.price_adjustment),
        ("housing.demolition_rate", h.demolition_rate),
        ("housing.base_growth", h.base_growth),
    ] {
        unit(issues, name, v);
    }
    let [lo, hi] = h.adults_per_household_range;
    let positive = [
        h.adults_per_household,
        h.reference_burden,
        h.rent_yield,
        h.build_cost_share,
        h.starts_max,
        lo,
    ];
    if positive.iter().any(|v| !(v.is_finite() && *v > 0.0))
        || !(lo <= h.adults_per_household && h.adults_per_household <= hi)
        || !(h.starts_price_response.is_finite() && h.starts_price_response >= 0.0)
        || !(h.starts_vacancy_response.is_finite() && h.starts_vacancy_response >= 0.0)
    {
        issues.push(Issue::error(
            "economy",
            "housing: positive sizes, responses at least 0, adults_per_household inside its range",
        ));
    }
    for (i, r) in e.death_rates.iter().enumerate() {
        unit(issues, &format!("death_rates[{i}]"), *r);
    }
    for (i, r) in e.taxes.income.iter().enumerate() {
        unit(issues, &format!("taxes.income[{i}]"), *r);
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
}

/// Treasury shares (DEC-098): each tier's starting budget split.
pub(super) fn validate_treasury_shares(b: &ContentBundle, issues: &mut Vec<Issue>) {
    // Treasury shares (DEC-098): each tier's starting budget split.
    for (tier, shares) in [
        ("local", &b.economy.treasury_shares.local),
        ("regional", &b.economy.treasury_shares.regional),
        ("national", &b.economy.treasury_shares.national),
    ] {
        let sum: f64 = shares.iter().sum();
        if shares
            .iter()
            .any(|s| !(s.is_finite() && (0.0..=1.0).contains(s)))
            || sum > 1.0 + 1e-9
        {
            issues.push(Issue::error(
                "economy",
                &format!("treasury_shares.{tier}: each share in [0, 1], at most 1 in all"),
            ));
        }
    }
}

/// Community registry (DEC-075): at least one; exactly one without a share
/// model (it takes each neighbourhood's remainder); finite shares and
/// offsets; offsets name known attitudes. Ids are unique (claimed on load).
pub(super) fn validate_communities(b: &ContentBundle, issues: &mut Vec<Issue>) {
    if b.communities.is_empty() {
        issues.push(Issue::error(
            "communities",
            "at least one community is required",
        ));
        return;
    }
    let remainder = b.communities.iter().filter(|c| c.share.is_none()).count();
    if remainder != 1 {
        issues.push(Issue::error(
            "communities",
            &format!(
                "exactly one community must omit `share` (it takes the remainder); found {remainder}"
            ),
        ));
    }
    let in_range = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
    for c in &b.communities {
        if let Some(sh) = &c.share {
            let n = &sh.by_neighborhood;
            if !in_range(sh.base, 0.0, 1.0) {
                issues.push(Issue::error(&c.id, "share.base must be in [0, 1]"));
            }
            if [
                n.core,
                n.inner,
                n.waterfront,
                n.industrial,
                n.suburb,
                n.rural,
            ]
            .iter()
            .any(|v| !in_range(*v, -1.0, 1.0))
            {
                issues.push(Issue::error(
                    &c.id,
                    "share.by_neighborhood values must be in [-1, 1]",
                ));
            }
            if !in_range(sh.regional_sd, 0.0, 0.5) {
                issues.push(Issue::error(&c.id, "share.regional_sd must be in [0, 0.5]"));
            }
        }
        for (k, v) in &c.attitude_offsets {
            if b.attitude_index(k).is_none() {
                issues.push(Issue::error(
                    &c.id,
                    &format!("attitude_offsets references unknown attitude `{k}`"),
                ));
            }
            if !in_range(*v, -2.0, 2.0) {
                issues.push(Issue::error(
                    &c.id,
                    &format!("attitude offset for `{k}` must be in [-2, 2]"),
                ));
            }
        }
        if !in_range(c.turnout_offset, -0.5, 0.5) {
            issues.push(Issue::error(&c.id, "turnout_offset must be in [-0.5, 0.5]"));
        }
        if !in_range(c.immigration_share, 0.0, 1.0) {
            issues.push(Issue::error(&c.id, "immigration_share must be in [0, 1]"));
        }
    }
    // Shares that can add up to more than everyone are scaled down by the
    // engine; say so, since the content probably did not mean it.
    let kinds: [fn(&crate::types::NeighborhoodLean) -> f64; 6] = [
        |n| n.core,
        |n| n.inner,
        |n| n.waterfront,
        |n| n.industrial,
        |n| n.suburb,
        |n| n.rural,
    ];
    for kind in kinds {
        let sum: f64 = b
            .communities
            .iter()
            .filter_map(|c| c.share.as_ref())
            .map(|sh| (sh.base + kind(&sh.by_neighborhood)).clamp(0.0, 1.0))
            .sum();
        if sum > 1.0 + 1e-9 {
            issues.push(Issue::warning(
                "communities",
                "community shares add up to more than 1 in some neighbourhood kind; they will be scaled down",
            ));
            break;
        }
    }
    let immigration: f64 = b.communities.iter().map(|c| c.immigration_share).sum();
    if immigration > 1.0 + 1e-9 {
        issues.push(Issue::warning(
            "communities",
            "immigration shares add up to more than 1; they will be scaled down",
        ));
    }
}
