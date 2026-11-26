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

/// Industry registry (DEC-110): a level under the sectors.
pub(super) fn validate_industries(b: &ContentBundle, issues: &mut Vec<Issue>) {
    use crate::types::SectorDriver as D;
    let unit = |x: f64| x.is_finite() && (0.0..=1.0).contains(&x);
    for d in &b.industries {
        let err = |issues: &mut Vec<Issue>, m: &str| issues.push(Issue::error(&d.id, m));
        // A split sector's closure rule, trend and wage floor are its
        // industries'; sectors that run on local demand keep their own.
        if let Some(s) = b.sectors.get(d.sector_index)
            && matches!(s.driver, D::Building | D::LocalServices | D::Public)
        {
            err(
                issues,
                &format!(
                    "sector `{}` runs on local demand ({:?}) and cannot be split into industries",
                    s.id, s.driver
                ),
            );
        }
        if !(d.weight.is_finite() && d.weight > 0.0) {
            err(issues, "weight must be finite and > 0");
        }
        if !(d.base_wage.is_finite() && d.base_wage > 0.0) {
            err(issues, "base_wage must be finite and > 0");
        }
        if !(d.cyclicality.is_finite() && (0.0..=3.0).contains(&d.cyclicality)) {
            err(issues, "cyclicality must be in [0, 3]");
        }
        if !(d.productivity_growth.is_finite() && (-0.1..=0.2).contains(&d.productivity_growth)) {
            err(issues, "productivity_growth must be in [-0.1, 0.2]");
        }
        if !(d.trend.is_finite() && (-0.1..=0.1).contains(&d.trend)) {
            err(issues, "trend must be in [-0.1, 0.1]");
        }
        let w = &d.world;
        if !(w.volatility.is_finite() && (0.0..=0.2).contains(&w.volatility)) {
            err(issues, "world.volatility must be in [0, 0.2]");
        }
        if !(w.reversion.is_finite() && w.reversion > 0.0 && w.reversion <= 1.0) {
            err(
                issues,
                "world.reversion must be in (0, 1]: the index must revert",
            );
        }
        if !unit(w.shock_chance) || !unit(w.shock_down) {
            err(
                issues,
                "world.shock_chance and shock_down must be in [0, 1]",
            );
        }
        if !(w.shock_size[0].is_finite()
            && w.shock_size[1].is_finite()
            && 0.0 <= w.shock_size[0]
            && w.shock_size[0] <= w.shock_size[1]
            && w.shock_size[1] <= 1.0)
        {
            err(
                issues,
                "world.shock_size must be [least, most] within [0, 1]",
            );
        }
        let e = &d.exposure;
        for (c, x) in &e.prices {
            if !(x.is_finite() && x.abs() <= 3.0) {
                err(issues, &format!("exposure.prices.{c} must be in [-3, 3]"));
            }
            match b.good(c) {
                None => err(issues, &format!("exposure.prices names unknown good `{c}`")),
                Some(g) if g.price.is_none() => err(
                    issues,
                    &format!("exposure.prices names `{c}`, a good with no price index"),
                ),
                _ => {}
            }
        }
        for (k, x) in &e.markets {
            if b.market(k).is_none() {
                err(
                    issues,
                    &format!("exposure.markets names unknown market `{k}`"),
                );
            }
            if !(x.is_finite() && *x >= 0.0) {
                err(issues, "exposure.markets weights must be finite and >= 0");
            }
        }
        if !b.markets.is_empty() && b.market_shares(d).iter().sum::<f64>() <= 0.0 {
            err(issues, "the industry has no share in any market");
        }
        if !unit(d.export_share) {
            err(issues, "export_share must be in [0, 1]");
        }
        if d.home.values().any(|x| !(x.is_finite() && *x >= 0.0)) {
            err(issues, "home weights must be finite and >= 0");
        }
        for (name, x) in [
            ("water", e.water),
            ("import_exposure", e.import_exposure),
            ("labour_cost", e.labour_cost),
            ("stock.power", e.stock.as_ref().map_or(0.0, |s| s.power)),
            ("link_power", d.link_power),
        ] {
            if !(x.is_finite() && (0.0..=3.0).contains(&x)) {
                err(issues, &format!("{name} must be in [0, 3]"));
            }
        }
        if !unit(e.low_pay) {
            err(issues, "exposure.low_pay must be in [0, 1]");
        }
        for l in &d.links {
            if l.from_index == usize::MAX {
                err(
                    issues,
                    &format!("link from unknown industry or sector `{}`", l.from),
                );
            }
            if l.from == d.id {
                err(issues, "an industry cannot link to itself");
            }
            if !(l.weight.is_finite() && l.weight >= 0.0) || !unit(l.local) {
                err(issues, "a link needs weight >= 0 and local in [0, 1]");
            }
        }
        let s = &d.siting;
        if !(s.base.is_finite() && s.base >= 0.0)
            || s.geo.values().any(|x| !(x.is_finite() && *x >= 0.0))
            || s.likes.values().any(|x| !(x.is_finite() && *x > 0.0))
            || s.gateways.values().any(|x| !(x.is_finite() && *x > 0.0))
            || s.by_class.iter().any(|x| !(x.is_finite() && *x >= 0.0))
            || s.near.values().any(|x| !(x.is_finite() && *x > 0.0))
            || !(s.regional_sd.is_finite() && (0.0..=2.0).contains(&s.regional_sd))
        {
            err(issues, "siting numbers must be finite and not negative");
        }
        if s.base <= 0.0 && s.geo.values().all(|x| *x <= 0.0) {
            err(
                issues,
                "siting gives no town a score (base and geo are zero)",
            );
        }
        for k in s.near.keys() {
            if b.industry_index(k).is_none() {
                err(issues, &format!("siting.near names unknown industry `{k}`"));
            }
        }
        let p = &d.plants;
        if let Some(kind) = &p.place_kind
            && !b.places.iter().any(|x| x.key == *kind)
        {
            err(
                issues,
                &format!("plants.place_kind `{kind}` is not a place kind"),
            );
        }
        if !(unit(p.named_share[0])
            && unit(p.named_share[1])
            && p.named_share[0] <= p.named_share[1])
        {
            err(
                issues,
                "plants.named_share must be [least, most] within [0, 1]",
            );
        }
        if p.founded[0] > p.founded[1] {
            err(issues, "plants.founded must be [least, most]");
        }
        if !(p.margin_sd.is_finite() && (0.0..=0.5).contains(&p.margin_sd)) {
            err(issues, "plants.margin_sd must be in [0, 0.5]");
        }
        if d.products.is_empty() {
            err(issues, "an industry needs at least one product");
        }
        for x in &d.products {
            if b.good(&x.good).is_none() {
                err(
                    issues,
                    &format!("product `{}` names unknown good `{}`", x.id, x.good),
                );
            }
            if let Some(w) = &x.with
                && b.industry_index(w).is_none()
            {
                err(
                    issues,
                    &format!("product `{}` names unknown industry `{w}`", x.id),
                );
            }
        }
    }
    // Markets and goods (the seam for foreign trade). Industries need a
    // market to sell to.
    if !b.industries.is_empty() && b.markets.is_empty() {
        issues.push(Issue::error(
            "markets",
            "industries need at least one market to sell to",
        ));
    }
    for m in &b.markets {
        if !(m.share.is_finite() && m.share >= 0.0) {
            issues.push(Issue::error(&m.id, "share must be finite and >= 0"));
        }
        if !(m.access.is_finite() && (0.0..=2.0).contains(&m.access)) {
            issues.push(Issue::error(&m.id, "access must be in [0, 2]"));
        }
    }
    // A split sector: weights that add up, and a wage near the sector's.
    for (k, s) in b.sectors.iter().enumerate() {
        let (mut w, mut wage) = (0.0, 0.0);
        for (_, d) in b.industries_of(k) {
            w += d.weight;
            wage += d.weight * d.base_wage;
        }
        if w <= 0.0 {
            continue;
        }
        if (w - 1.0).abs() > 0.02 {
            issues.push(Issue::warning(
                &s.id,
                &format!("its industries' weights add up to {w:.3}, not 1"),
            ));
        }
        let mean = wage / w;
        if (mean / s.base_wage - 1.0).abs() > 0.05 {
            issues.push(Issue::warning(
                &s.id,
                &format!(
                    "its industries' weighted wage ({mean:.0}) is more than 5% from the sector's ({:.0})",
                    s.base_wage
                ),
            ));
        }
    }
    let p = &b.economy.industries;
    let mut bad = |m: &str| issues.push(Issue::error("economy.industries", m));
    if !(p.notice_line < p.reprieve_line && p.reprieve_line <= p.short_line && p.notice_line > 0.0)
    {
        bad("lines must satisfy 0 < notice_line < reprieve_line <= short_line");
    }
    if !(p.full_line.is_finite() && p.full_line >= p.short_line) {
        bad("full_line must be at or above short_line");
    }
    if p.short_months == 0 || p.notice_after_months == 0 || p.reprieve_months == 0 {
        bad("short_months, notice_after_months and reprieve_months must be at least 1");
    }
    if p.notice_months[0] == 0 || p.notice_months[0] > p.notice_months[1] {
        bad("notice_months must be [least, most], at least one month");
    }
    if !(p.notice_line < p.review_line && p.review_line <= p.full_line) {
        bad("review_line must satisfy notice_line < review_line <= full_line");
    }
    if p.review_months <= p.short_months as u16 {
        bad("review_months must be more than short_months");
    }
    if p.reprieve_months > p.notice_months[0] {
        bad("reprieve_months must fit inside the shortest notice");
    }
    if !(unit(p.small_decay)
        && unit(p.max_town_share)
        && unit(p.min_share)
        && unit(p.boom_size)
        && unit(p.review_cut))
    {
        bad("small_decay, max_town_share, min_share, boom_size and review_cut must be in [0, 1]");
    }
    if !(p.factor_months >= 1.0 && p.trend_months >= 1.0) {
        bad("factor_months and trend_months must be at least 1");
    }
    if !(0.0 < p.link_range[0] && p.link_range[0] <= 1.0 && 1.0 <= p.link_range[1]) {
        bad("link_range must bracket 1");
    }
    if !(0.0 < p.world_range[0] && p.world_range[0] < 1.0 && 1.0 < p.world_range[1]) {
        bad("world_range must bracket 1");
    }
    if p.min_plant.iter().any(|x| !(x.is_finite() && *x > 0.0))
        || !(p.min_capacity.is_finite() && p.min_capacity >= 0.0)
    {
        bad("min_plant sizes must be above zero and min_capacity not negative");
    }
    if p.max_industries.contains(&0) {
        bad("max_industries must be at least 1 for every class");
    }
    if !(p.town_sd.is_finite() && (0.0..=2.0).contains(&p.town_sd)) {
        bad("town_sd must be in [0, 2]");
    }
    if p.name_forms.iter().any(|x| !(x.is_finite() && *x >= 0.0))
        || p.name_forms.iter().sum::<f64>() <= 0.0
    {
        bad("name_forms must be weights, not all zero");
    }
    if p.name_forms[2] > 0.0 && p.works_words.is_empty() {
        bad("works_words is empty but name_forms gives the word form a weight");
    }
    if !unit(p.pool_share) {
        bad("pool_share must be in [0, 1]");
    }
    if !(p.new_plant_margin.is_finite() && (0.0..=0.5).contains(&p.new_plant_margin)) {
        bad("new_plant_margin must be in [0, 0.5]");
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
    unit(
        issues,
        "housing.capped_vacancy_signal",
        h.capped_vacancy_signal,
    );
    let bw = &e.borrowing;
    for (i, v) in bw.spreads.iter().enumerate() {
        if !(0.0..=0.2).contains(v) {
            issues.push(Issue::error(
                "economy",
                &format!("borrowing.spreads[{i}] = {v} outside [0, 0.2]"),
            ));
        }
    }
    if bw.free_ratio.iter().any(|v| !(v.is_finite() && *v >= 0.0))
        || !(bw.per_ratio.is_finite() && bw.per_ratio >= 0.0)
        || !(0.0..=0.5).contains(&bw.max_extra)
    {
        issues.push(Issue::error(
            "economy",
            "borrowing: free_ratio and per_ratio at least 0, max_extra in [0, 0.5]",
        ));
    }
    let at = &e.attribution;
    for (name, v) in [
        ("attribution.homes_net", at.homes_net),
        ("attribution.jobs_net", at.jobs_net),
        ("attribution.wage_floor_net", at.wage_floor_net),
    ] {
        if !(0.0..=2.0).contains(&v) {
            issues.push(Issue::error(
                "economy",
                &format!("{name} = {v} outside [0, 2]"),
            ));
        }
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
