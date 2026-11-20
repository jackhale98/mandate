//! Content validation rules (DEC-053). Run by `mandate-cli content validate`
//! in CI and on every load.

use crate::bundle::ContentBundle;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

/// The variables (`{ $x }`, also inside selectors) each Fluent message of
/// a language's `.ftl` sources uses, by message id. A message runs from its
/// `id = …` line through the indented lines after it.
pub fn fluent_placeholders(text: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    let scan = |line: &str, vars: &mut BTreeSet<String>| {
        let mut rest = line;
        while let Some(i) = rest.find('$') {
            let name: String = rest[i + 1..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if !name.is_empty() {
                vars.insert(name.clone());
            }
            rest = &rest[i + 1 + name.len()..];
        }
    };
    for line in text.lines() {
        let first = line.chars().next();
        match first {
            Some(c) if c.is_ascii_alphabetic() => {
                current = line.split_once('=').map(|(id, _)| id.trim().to_string());
                if let (Some(id), Some((_, value))) = (&current, line.split_once('=')) {
                    scan(value, out.entry(id.clone()).or_default());
                }
            }
            Some(c) if c.is_whitespace() => {
                if let Some(id) = &current {
                    scan(line, out.entry(id.clone()).or_default());
                }
            }
            _ => current = None,
        }
    }
    out
}

/// A constitution's posts and career (DEC-017, DEC-098): ids and role words
/// are unique, every tier has the engine's procedures, only implemented
/// voting rules, at-large bodies have seats, workplaces are place kinds,
/// and the career ladder reaches its summit.
fn validate_posts(b: &ContentBundle, c: &crate::types::ConstitutionDef, issues: &mut Vec<Issue>) {
    use crate::types::{Constituency, PostKey, PostRef, Selection, TierKey, VotingRule};
    let err = |issues: &mut Vec<Issue>, msg: String| issues.push(Issue::error(&c.id, &msg));
    let mut ids: Vec<&str> = Vec::new();
    let mut words: Vec<&str> = Vec::new();
    for t in &c.tiers {
        // The engine's procedures (a contract: law-making runs `ordinance`).
        for needed in crate::types::ENGINE_PROCEDURES {
            if t.procedure(needed).is_none() {
                err(
                    issues,
                    format!("{:?} tier lacks the `{needed}` procedure", t.tier),
                );
            }
        }
        for p in t.posts() {
            let id = p.id();
            if ids.contains(&id) {
                err(
                    issues,
                    format!("body or office id `{id}` is used twice (ids are unique across tiers)"),
                );
            }
            ids.push(id);
            if !valid_key(id) || id.contains('.') || PostKey::new(id).is_none() {
                err(
                    issues,
                    format!(
                        "body or office id `{id}` must be [a-z0-9_], at most {} bytes",
                        PostKey::MAX
                    ),
                );
            }
            let word = p.word();
            if words.contains(&word) {
                err(issues, format!("role word `{word}` names two posts"));
            }
            words.push(word);
            if !valid_key(word) || word.contains('.') {
                err(issues, format!("role word `{word}` must be [a-z0-9_]"));
            }
            if b.occupation(word).is_some()
                || word == crate::narrative::OFFICE_ANY
                || word == crate::narrative::OFFICE_NONE
            {
                err(
                    issues,
                    format!("role word `{word}` is already an occupation or a keyword"),
                );
            }
            if p.term_years() == 0 {
                err(issues, format!("`{id}` needs a term of at least a year"));
            }
            if !b.places.iter().any(|x| x.key == p.works_at()) {
                err(
                    issues,
                    format!(
                        "`{id}` works at `{}`, which is not a place kind",
                        p.works_at()
                    ),
                );
            }
            match p.selection() {
                Selection::Election { rule, constituency } => {
                    // Only the plurality tabulator exists (DEC-098 note).
                    if *rule != VotingRule::Plurality {
                        err(
                            issues,
                            format!(
                                "`{id}` uses the voting rule {rule:?}, which has no tabulator yet (only plurality)"
                            ),
                        );
                    }
                    match (p, constituency) {
                        (PostRef::Body(body), Constituency::AtLarge) => {
                            if body.seats.unwrap_or(0) == 0 {
                                err(issues, format!("at-large body `{id}` needs `seats`"));
                            }
                            if t.tier == TierKey::Local {
                                // M6 instantiates local posts only, and has
                                // no at-large races for bodies yet.
                                err(
                                    issues,
                                    format!(
                                        "at-large body `{id}` is not supported on the local tier yet"
                                    ),
                                );
                            }
                        }
                        (PostRef::Body(body), Constituency::Ward) => {
                            if body.seats.is_some() {
                                err(
                                    issues,
                                    format!(
                                        "`{id}` has a seat per ward: `seats` is for at-large bodies"
                                    ),
                                );
                            }
                        }
                        (PostRef::Office(_), Constituency::Ward) => {
                            err(issues, format!("office `{id}` cannot be elected by ward"));
                        }
                        (PostRef::Office(_), Constituency::AtLarge) => {}
                    }
                }
                Selection::AppointedBy { office } => {
                    if t.office(office).is_none() {
                        err(
                            issues,
                            format!("`{id}` is appointed by unknown office `{office}`"),
                        );
                    }
                }
                Selection::ElectedByBody { body } => {
                    if t.body(body).is_none() {
                        err(
                            issues,
                            format!("`{id}` is elected by unknown body `{body}`"),
                        );
                    }
                }
            }
            if let Some(cal) = p.calendar()
                && !(1..=12).contains(&cal.month)
            {
                err(issues, format!("`{id}` has an election month outside 1–12"));
            }
        }
    }
    // Role words are distinct from ids of other posts (an office's id is its
    // own word).
    for t in &c.tiers {
        for body in &t.bodies {
            if c.post(&body.member_key)
                .is_some_and(|(_, p)| p.id() != body.id)
            {
                err(
                    issues,
                    format!("`{}`'s member word is another post's id", body.id),
                );
            }
        }
    }
    // The career ladder (DEC-017: a viable path to the summit).
    let ladder = &c.career;
    let Some((summit_tier, summit)) = c.post(&ladder.summit) else {
        err(
            issues,
            format!("career summit `{}` is not a post", ladder.summit),
        );
        return;
    };
    let reachable = |p: PostRef<'_>| {
        matches!(
            p.selection(),
            Selection::Election { .. } | Selection::ElectedByBody { .. }
        )
    };
    if !reachable(summit) {
        err(issues, "the career summit must be elected".into());
    }
    let mut tiers: Vec<TierKey> = c.tiers.iter().map(|t| t.tier).collect();
    tiers.sort();
    for tk in tiers.iter().filter(|t| **t < summit_tier) {
        if !c.tier(*tk).is_some_and(|t| t.posts().any(reachable)) {
            err(
                issues,
                format!("no elected post on the {tk:?} tier: no path to the summit"),
            );
        }
    }
    if ladder.acts.is_empty() {
        err(issues, "the career needs acts".into());
    }
    let mut act_ids: Vec<&str> = Vec::new();
    for a in &ladder.acts {
        if act_ids.contains(&a.id.as_str()) {
            err(issues, format!("act `{}` defined twice", a.id));
        }
        act_ids.push(&a.id);
        if a.tier.is_none() && a.posts.is_empty() {
            err(issues, format!("act `{}` needs a tier or posts", a.id));
        }
        if a.tier.is_some_and(|tk| c.tier(tk).is_none()) {
            err(
                issues,
                format!("act `{}` names a tier the constitution lacks", a.id),
            );
        }
        for p in a.posts.iter().chain(&a.except) {
            if c.post(p).is_none() {
                err(issues, format!("act `{}` names unknown post `{p}`", a.id));
            }
        }
    }
    for t in &c.tiers {
        for p in t.posts() {
            if c.act_of(t.tier, p.id()).is_none() {
                err(issues, format!("post `{}` belongs to no act", p.id()));
            }
        }
    }
}

/// Whether the family mechanisms (`borrow`, `revenue`, `sector_capacity`)
/// are simulated: content may use them (F2b).
pub const FAMILY_MECHANISMS_SIMULATED: bool = false;

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
    crate::layout::validate_layout(b, &mut issues);
    crate::map_layers::validate(b, &mut issues);
    crate::interiors::validate(b, &mut issues);
    crate::routines::validate(b, &mut issues);
    validate_communities(b, &mut issues);
    validate_scenarios(b, &mut issues);
    validate_concerns(b, &mut issues);
    validate_characters(b, &mut issues);
    crate::narrative::validate(b, &mut issues);
    crate::media::validate(b, &mut issues);
    crate::knowledge::validate(b, &mut issues);
    crate::visits::validate(b, &mut issues);
    crate::conversation::validate(b, &mut issues);
    crate::act::validate(b, &mut issues);
    for (name, v, lo, hi) in param_ranges(b) {
        if !(v.is_finite() && (lo..=hi).contains(&v)) {
            let kind = name.split('.').next().unwrap_or("parameters").to_string();
            issues.push(Issue::error(
                &kind,
                &format!("{name} = {v} outside [{lo}, {hi}]"),
            ));
        }
    }
    for (name, [lo, hi]) in [
        ("actions.fundraise.raised", b.actions.fundraise.raised),
        ("council.lobby.gain", b.council.lobby.gain),
    ] {
        if lo > hi {
            issues.push(Issue::error(name, &format!("{name} must be [min, max]")));
        }
    }
    let [lo, hi] = b.npc.bill_sunset_years;
    if lo > hi || lo == 0 {
        issues.push(Issue::error(
            "npc",
            "npc.bill_sunset_years must be [min, max] with min >= 1",
        ));
    }
    if b.actions.town_hall.risk_min > b.actions.town_hall.risk_max
        || b.actions.press.gaffe_min > b.actions.press.gaffe_max
        || b.council.lobby.odds_min > b.council.lobby.odds_max
    {
        issues.push(Issue::error(
            "parameters",
            "risk and odds bounds must be [min, max]",
        ));
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
        validate_posts(b, c, &mut issues);
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
    // Nomination gates name posts by role word (DEC-098).
    for (word, gate) in &b.career.nomination {
        if !b.is_role_word(word) {
            issues.push(Issue::error(
                "career",
                &format!("nomination gate for unknown role word `{word}`"),
            ));
        }
        if gate
            .min_standing
            .is_some_and(|m| !(m.is_finite() && (0.0..=100.0).contains(&m)))
        {
            issues.push(Issue::error(
                "career",
                &format!("nomination gate for `{word}`: min_standing must be in 0–100"),
            ));
        }
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
            if let Some(param) = m.param()
                && !has(param)
            {
                issues.push(Issue::error(
                    &p.id,
                    &format!("mechanism references unknown parameter `{param}`"),
                ));
            }
            // Slot references (`@slot`) belong to policy families.
            if let M::HazardProtection {
                hazard: crate::types::HazardRef::Slot(h),
                ..
            } = m
            {
                issues.push(Issue::error(
                    &p.id,
                    &format!("hazard `{h}` is a slot reference; templates name a hazard"),
                ));
            }
            if crate::policy_family::is_family_mechanism(m) && !FAMILY_MECHANISMS_SIMULATED {
                issues.push(Issue::error(
                    &p.id,
                    &format!(
                        "mechanism `{}` is declared but not yet simulated (F2b)",
                        m.key()
                    ),
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
        // Situational policies (DEC-077 §3): conditions name real things.
        if let Some(u) = &p.unlock {
            check_situation(b, &p.id, p.tier, u, &mut issues);
        }
    }
    // Policy families (DEC-092): items 1–10 of the contract's §2.13.
    crate::policy_family::validate(b, &mut issues);
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
            &mut issues,
        );
        // What a line teaches (DEC-091): lines have no cast, so no roles.
        for t in &o.teaches {
            if t.role().is_some() || matches!(t, crate::narrative::SubjectDef::Person { .. }) {
                issues.push(Issue::error(
                    &o.id,
                    &format!("overheard lines teach without roles (`{}`)", t.key()),
                ));
            }
            crate::narrative::check_subject(&o.id, b, t, &mut issues);
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
    for c in &b.communities {
        referenced.push((&c.id, &c.name_key));
    }
    for x in &b.scenarios {
        referenced.push((&x.id, &x.name_key));
    }
    for c in &b.concerns {
        referenced.push((&c.id, &c.name_key));
    }
    for o in &b.overheard {
        referenced.push((&o.id, &o.key));
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
    for t in &b.traits {
        referenced.push((&t.id, &t.name_key));
    }
    for o in &b.occupations {
        referenced.push((&o.id, &o.name_key));
    }
    for c in &b.crises {
        referenced.push((&c.id, &c.name_key));
    }
    // Memories are quoted through `memory-<kind>`.
    let memory_keys: Vec<String> = crate::types::MEMORY_KINDS
        .iter()
        .map(|k| format!("memory-{}", k.replace('_', "-")))
        .collect();
    for key in &memory_keys {
        referenced.push(("characters", key));
    }
    for p in b.policies.values() {
        referenced.push((&p.id, &p.name_key));
        referenced.push((&p.id, &p.desc_key));
        if let Some(u) = &p.unlock {
            referenced.push((&p.id, &u.reason_key));
        }
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
        // A character's expertise names the issue (DEC-031).
        .chain(
            b.attitudes
                .iter()
                .map(|a| (a.id.as_str(), format!("issue-{}", a.key))),
        )
        .collect();
    for (owner, key) in &convictions {
        referenced.push((owner, key));
    }
    // Residents name a town's industry in overheard talk (`talk-<name key>`).
    for s in &b.sectors {
        if let Some(k) = &s.talk_key {
            referenced.push((&s.id, k));
        }
    }
    // And its worries in conversation.
    for c in &b.concerns {
        if let Some(k) = &c.talk_key {
            referenced.push((&c.id, k));
        }
    }
    referenced.extend(crate::map_layers::fluent_refs(b));
    referenced.extend(crate::act::fluent_refs(b));
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

/// Ranges for the balance numbers (DEC-078 decision 1): (path, value, min,
/// max). Ranges are wide guardrails against typos and broken overrides, not
/// balance targets.
pub fn param_ranges(b: &ContentBundle) -> Vec<(String, f64, f64, f64)> {
    let mut out: Vec<(String, f64, f64, f64)> = Vec::new();
    let mut r = |name: &str, v: f64, lo: f64, hi: f64| out.push((name.to_string(), v, lo, hi));
    let v = &b.politics.vote;
    for (name, x, lo, hi) in [
        ("issue_weight", v.issue_weight, 0.0, 20.0),
        ("beta_low", v.beta_low, 0.0, 1.0),
        ("beta_high", v.beta_high, 0.0, 1.0),
        ("polarization_low", v.polarization_low, 0.0, 2.0),
        ("polarization_high", v.polarization_high, 0.0, 2.0),
        ("party_weight", v.party_weight, 0.0, 20.0),
        ("valence_weight", v.valence_weight, 0.0, 20.0),
        ("retrospective_weight", v.retrospective_weight, 0.0, 50.0),
        ("executive_multiplier", v.executive_multiplier, 0.0, 10.0),
        ("home_ward_bonus", v.home_ward_bonus, 0.0, 5.0),
        ("partisan_discount", v.partisan_discount, 0.0, 1.0),
        ("incumbent_awareness", v.incumbent_awareness, 0.0, 1.0),
        ("challenger_awareness", v.challenger_awareness, 0.0, 1.0),
        ("race_shock_sd", v.race_shock_sd, 0.0, 5.0),
        ("national_swing_sd", v.national_swing_sd, 0.0, 5.0),
        ("strategic_weight", v.strategic_weight, 0.0, 20.0),
        ("minor_party_threshold", v.minor_party_threshold, 0.0, 1.0),
        ("record_penalty", v.record_penalty, 0.0, 50.0),
        ("record_bonus", v.record_bonus, 0.0, 10.0),
        ("record_baseline", v.record_baseline, 0.0, 1.0),
        ("renomination_standing", v.renomination_standing, 0.0, 100.0),
        ("salience_base", v.salience_base, 0.01, 100.0),
        ("perceived_own_weight", v.perceived_own_weight, 0.0, 1.0),
        ("perceived_party_weight", v.perceived_party_weight, 0.0, 1.0),
        ("valence_oratory", v.valence_oratory, 0.0, 5.0),
        ("valence_integrity", v.valence_integrity, 0.0, 5.0),
        ("governing_party_record", v.governing_party_record, 0.0, 1.0),
        ("retro_economy_weight", v.retro_economy_weight, 0.0, 5.0),
        (
            "retro_unemployment_scale",
            v.retro_unemployment_scale,
            0.0,
            100.0,
        ),
        ("media_base", v.media_base, 0.0, 5.0),
        ("media_town_share", v.media_town_share, 0.0, 5.0),
        (
            "player_approval_weight",
            v.player_approval_weight,
            0.0,
            20.0,
        ),
        ("independent_viability", v.independent_viability, 0.0, 1.0),
        (
            "player_incumbent_awareness",
            v.player_incumbent_awareness,
            0.0,
            1.0,
        ),
        ("officeholder_awareness", v.officeholder_awareness, 0.0, 1.0),
        ("home_awareness_bonus", v.home_awareness_bonus, 0.0, 1.0),
        ("awareness_cap", v.awareness_cap, 0.0, 1.0),
    ] {
        r(&format!("politics.vote.{name}"), x, lo, hi);
    }
    let t = &b.politics.turnout;
    r("politics.turnout.habit", t.habit, 0.0, 1.0);
    r("politics.turnout.habit_base", t.habit_base, 0.0, 2.0);
    r(
        "politics.turnout.habit_propensity",
        t.habit_propensity,
        0.0,
        2.0,
    );
    r(
        "politics.turnout.mobilisation_max",
        t.mobilisation_max,
        0.0,
        1.0,
    );
    let c = &b.politics.campaign;
    r("politics.campaign.diminishing", c.diminishing, 0.0, 1.0);
    r(
        "politics.campaign.npc_effort_base",
        c.npc_effort_base,
        0.0,
        5.0,
    );
    r(
        "politics.campaign.contact_memory",
        c.contact_memory,
        0.0,
        1.0,
    );
    r("politics.campaign.ad_saturation", c.ad_saturation, 0.0, 1.0);
    r(
        "politics.campaign.ad_persuasion_share",
        c.ad_persuasion_share,
        0.0,
        5.0,
    );
    r(
        "politics.party_id.governing_credit",
        b.politics.party_id.governing_credit,
        0.0,
        2.0,
    );

    let o = &b.opinion;
    let e = &o.experience;
    r(
        "opinion.experience.unemployment",
        e.unemployment,
        0.0,
        100.0,
    );
    r("opinion.experience.rent_growth", e.rent_growth, 0.0, 100.0);
    r("opinion.experience.services", e.services, 0.0, 100.0);
    r("opinion.experience.disaster", e.disaster, 0.0, 5.0);
    r(
        "opinion.experience.disaster_months",
        e.disaster_months.into(),
        0.0,
        120.0,
    );
    r("opinion.experience.disruption", e.disruption, 0.0, 100.0);
    r("opinion.policy_reaction", o.policy_reaction, 0.0, 20.0);
    r(
        "opinion.law_reaction_weight",
        o.law_reaction_weight,
        0.0,
        10.0,
    );
    r("opinion.gov_approval_rate", o.gov_approval_rate, 0.0, 1.0);
    r(
        "opinion.player_approval_rate",
        o.player_approval_rate,
        0.0,
        1.0,
    );
    r(
        "opinion.player_reaction_scale",
        o.player_reaction_scale,
        0.0,
        20.0,
    );
    let lc = &o.local_concerns;
    let kinds = |l: &crate::NeighborhoodLean| {
        [
            ("core", l.core),
            ("inner", l.inner),
            ("waterfront", l.waterfront),
            ("industrial", l.industrial),
            ("suburb", l.suburb),
            ("rural", l.rural),
        ]
    };
    for (k, v) in kinds(&lc.rent) {
        r(&format!("opinion.local_concerns.rent.{k}"), v, 0.05, 10.0);
    }
    for (k, v) in kinds(&lc.pollution) {
        r(
            &format!("opinion.local_concerns.pollution.{k}"),
            v,
            0.0,
            10.0,
        );
    }
    for (name, v, hi) in [
        ("rent_growth", lc.rent_growth, 10.0),
        ("pollution_workplace", lc.pollution_workplace, 10.0),
        ("pollution_next_door", lc.pollution_next_door, 10.0),
        ("pollution_floodplain", lc.pollution_floodplain, 10.0),
        ("workplace_affinity", lc.workplace_affinity, 20.0),
        ("job_loss_decline", lc.job_loss_decline, 50.0),
        ("jobs_sector_tilt", lc.jobs_sector_tilt, 5.0),
    ] {
        r(&format!("opinion.local_concerns.{name}"), v, 0.0, hi);
    }
    for s in &lc.sector_kinds {
        for (k, v) in kinds(&s.kinds) {
            r(
                &format!("opinion.local_concerns.sector_kinds.{:?}.{k}", s.driver),
                v,
                0.0,
                20.0,
            );
        }
    }

    let a = &b.actions;
    r("actions.late_night_health", a.late_night_health, 0.0, 100.0);
    let vo = &a.vote;
    for (name, x) in [
        ("standing_loyal", vo.standing_loyal),
        ("standing_defy", vo.standing_defy),
        ("standing_abstain", vo.standing_abstain),
    ] {
        r(&format!("actions.vote.{name}"), x, -100.0, 100.0);
    }
    r("actions.vote.ward_reaction", vo.ward_reaction, 0.0, 1.0);
    r("actions.vote.ward_abstain", vo.ward_abstain, -1.0, 1.0);
    for (name, x) in [
        ("sponsor_yes", vo.sponsor_yes),
        ("sponsor_no", vo.sponsor_no),
        ("sponsor_no_owed", vo.sponsor_no_owed),
    ] {
        r(&format!("actions.vote.{name}"), x, 0.0, 100.0);
    }
    r("actions.amend.sway_keep", a.amend.sway_keep, 0.0, 1.0);
    r(
        "actions.introduce.awareness",
        a.introduce.awareness,
        0.0,
        1.0,
    );
    r("actions.lobby.trade_gain", a.lobby.trade_gain, 0.0, 5.0);
    r(
        "actions.lobby.opinion_success",
        a.lobby.opinion_success,
        -100.0,
        100.0,
    );
    r(
        "actions.lobby.opinion_fail",
        a.lobby.opinion_fail,
        -100.0,
        100.0,
    );
    r(
        "actions.lobby.late_night_legal",
        a.lobby.late_night_legal,
        0.0,
        100.0,
    );
    let th = &a.town_hall;
    for (name, x, lo, hi) in [
        ("awareness", th.awareness, 0.0, 1.0),
        (
            "awareness_oratory_base",
            th.awareness_oratory_base,
            0.0,
            5.0,
        ),
        ("bill_nudge", th.bill_nudge, 0.0, 1.0),
        ("nudge", th.nudge, -1.0, 1.0),
        ("hostile_nudge", th.hostile_nudge, -1.0, 1.0),
        ("bill_awareness", th.bill_awareness, 0.0, 1.0),
        ("contact_base", th.contact_base, 0.0, 5.0),
        ("contact_oratory", th.contact_oratory, 0.0, 5.0),
        ("hostile_contact", th.hostile_contact, 0.0, 1.0),
        ("risk_base", th.risk_base, -1.0, 1.0),
        ("risk_approval", th.risk_approval, 0.0, 5.0),
        ("risk_government", th.risk_government, 0.0, 5.0),
        ("risk_min", th.risk_min, 0.0, 1.0),
        ("risk_max", th.risk_max, 0.0, 1.0),
    ] {
        r(&format!("actions.town_hall.{name}"), x, lo, hi);
    }
    let pr = &a.press;
    for (name, x, lo, hi) in [
        ("awareness", pr.awareness, 0.0, 1.0),
        ("gaffe_approval", pr.gaffe_approval, 0.0, 1.0),
        ("bill_awareness", pr.bill_awareness, 0.0, 1.0),
        ("contact", pr.contact, 0.0, 5.0),
        ("gaffe_base", pr.gaffe_base, 0.0, 1.0),
        ("gaffe_oratory", pr.gaffe_oratory, 0.0, 1.0),
        ("gaffe_min", pr.gaffe_min, 0.0, 1.0),
        ("gaffe_max", pr.gaffe_max, 0.0, 1.0),
    ] {
        r(&format!("actions.press.{name}"), x, lo, hi);
    }
    r("actions.canvass.awareness", a.canvass.awareness, 0.0, 1.0);
    r("actions.canvass.approval", a.canvass.approval, 0.0, 1.0);
    r("actions.canvass.contact", a.canvass.contact, 0.0, 10.0);
    let f = &a.fundraise;
    r("actions.fundraise.raised[0]", f.raised[0], 0.0, 1e7);
    r("actions.fundraise.raised[1]", f.raised[1], 0.0, 1e7);
    r("actions.fundraise.standing", f.standing, -100.0, 100.0);
    r(
        "actions.fundraise.late_night_legal",
        f.late_night_legal,
        0.0,
        100.0,
    );
    r("actions.fundraise.donor_ties", f.donor_ties, 0.0, 1.0);

    let co = &b.council;
    for (name, x, lo, hi) in [
        ("own_values", co.own_values, 0.0, 5.0),
        ("ward_support", co.ward_support, 0.0, 5.0),
        ("mayor", co.mayor, 0.0, 5.0),
        ("player_opinion", co.player_opinion, 0.0, 5.0),
        ("same_party", co.same_party, -2.0, 2.0),
        ("rival_party", co.rival_party, -2.0, 2.0),
        ("sunset", co.sunset, -2.0, 2.0),
        ("pilot", co.pilot, -2.0, 2.0),
        ("foreign_earmark", co.foreign_earmark, -2.0, 2.0),
        ("own_earmark", co.own_earmark, -2.0, 2.0),
        ("noise_sd", co.noise_sd, 0.0, 2.0),
        ("mayor_own", co.mayor_own, 0.0, 5.0),
        ("mayor_public", co.mayor_public, 0.0, 5.0),
        ("vote_sharpness", co.vote_sharpness, 0.0, 100.0),
        ("sign_bias", co.sign_bias, -2.0, 2.0),
        ("enactment_awareness", co.enactment_awareness, 0.0, 1.0),
        ("awareness_fade", co.awareness_fade, 0.0, 1.0),
    ] {
        r(&format!("council.{name}"), x, lo, hi);
    }
    let l = &co.lobby;
    for (name, x, lo, hi) in [
        ("skill_negotiation", l.skill_negotiation, 0.0, 5.0),
        ("skill_oratory", l.skill_oratory, 0.0, 5.0),
        ("skill_scale", l.skill_scale, 0.0, 50.0),
        ("pragmatism", l.pragmatism, 0.0, 50.0),
        ("bias", l.bias, -50.0, 50.0),
        ("opinion_scale", l.opinion_scale, 1.0, 1000.0),
        ("same_party", l.same_party, -10.0, 10.0),
        ("other_party", l.other_party, -10.0, 10.0),
        ("late_night", l.late_night, -10.0, 10.0),
        ("odds_min", l.odds_min, 0.0, 1.0),
        ("odds_max", l.odds_max, 0.0, 1.0),
        ("gain[0]", l.gain[0], 0.0, 5.0),
        ("gain[1]", l.gain[1], 0.0, 5.0),
        ("fail_gain", l.fail_gain, -5.0, 5.0),
    ] {
        r(&format!("council.lobby.{name}"), x, lo, hi);
    }

    let ca = &b.career;
    let g = &ca.record_gain;
    for (name, x) in [
        ("introduce", g.introduce),
        ("amend", g.amend),
        ("vote", g.vote),
        ("lobby", g.lobby),
        ("town_hall", g.town_hall),
        ("canvass", g.canvass),
        ("press", g.press),
    ] {
        r(&format!("career.record_gain.{name}"), x, 0.0, 1.0);
    }
    for (name, x, lo, hi) in [
        ("record_fade", ca.record_fade, 0.0, 1.0),
        ("npc_diligence_rate", ca.npc_diligence_rate, 0.0, 1.0),
        ("diligence_integrity", ca.diligence_integrity, 0.0, 1.0),
        ("diligence_ambition", ca.diligence_ambition, 0.0, 1.0),
        ("thin_record", ca.thin_record, 0.0, 1.0),
        (
            "thin_record_standing",
            ca.thin_record_standing,
            -100.0,
            100.0,
        ),
        ("strong_record", ca.strong_record, 0.0, 1.0),
        (
            "strong_record_standing",
            ca.strong_record_standing,
            -100.0,
            100.0,
        ),
        ("invisible_record", ca.invisible_record, 0.0, 1.0),
        ("awareness_keep", ca.awareness_keep, 0.0, 1.0),
        (
            "awareness_keep_invisible",
            ca.awareness_keep_invisible,
            0.0,
            1.0,
        ),
        ("mandate_rate", ca.mandate_rate, 0.0, 1.0),
        (
            "mandate_per_net_approval",
            ca.mandate_per_net_approval,
            0.0,
            1000.0,
        ),
        ("viability_rate", ca.viability_rate, 0.0, 1.0),
        (
            "viability_standing_share",
            ca.viability_standing_share,
            0.0,
            1.0,
        ),
        ("health_recovery", ca.health_recovery, 0.0, 100.0),
        ("win_mandate_base", ca.win_mandate_base, 0.0, 100.0),
        (
            "win_mandate_per_margin",
            ca.win_mandate_per_margin,
            0.0,
            1000.0,
        ),
        ("win_mandate_min", ca.win_mandate_min, 0.0, 100.0),
        ("win_viability", ca.win_viability, -100.0, 100.0),
        ("win_standing", ca.win_standing, -100.0, 100.0),
        ("loss_viability", ca.loss_viability, -100.0, 100.0),
        (
            "loss_viability_per_margin",
            ca.loss_viability_per_margin,
            0.0,
            1000.0,
        ),
        ("loss_standing", ca.loss_standing, -100.0, 100.0),
        ("scandal_keep_yearly", ca.scandal_keep_yearly, 0.0, 1.0),
        ("officeholder_funds", ca.officeholder_funds, 0.0, 1e7),
        (
            "officeholder_funds_oratory",
            ca.officeholder_funds_oratory,
            0.0,
            1e7,
        ),
    ] {
        r(&format!("career.{name}"), x, lo, hi);
    }

    let n = &b.npc;
    for (name, x, lo, hi) in [
        ("ideologue_below", n.ideologue_below, 0.0, 1.0),
        ("populist_above", n.populist_above, 0.0, 1.0),
        ("close_margin", n.close_margin, 0.01, 1.0),
        ("funds_in_ads", n.funds_in_ads, 0.0, 100.0),
        ("busy_pending", n.busy_pending, 0.1, 100.0),
        ("busy_laws", n.busy_laws, 0.1, 100.0),
        (
            "bill_interval_days",
            n.bill_interval_days.into(),
            0.0,
            3650.0,
        ),
        ("insurance_votes", n.insurance_votes, 0.0, 10.0),
        ("safe_gap", n.safe_gap, 0.01, 100.0),
        ("comfortable_weight", n.comfortable_weight, 0.0, 1.0),
        ("lobby_closeness_keep", n.lobby_closeness_keep, 0.0, 1.0),
        ("contact_unknown_keep", n.contact_unknown_keep, 0.0, 1.0),
        ("two_ads_above", n.two_ads_above, 0.0, 10.0),
        ("one_ad_above", n.one_ad_above, 0.0, 10.0),
        ("ad_reserve", n.ad_reserve, 0.0, 100.0),
    ] {
        r(&format!("npc.{name}"), x, lo, hi);
    }
    r("npc.crisis_agenda", n.crisis_agenda, 0.0, 5.0);
    r("npc.concern_agenda", n.concern_agenda, 0.0, 5.0);

    let ch = &b.characters;
    r(
        "characters.memory.principal",
        ch.memory.principal.into(),
        1.0,
        100.0,
    );
    r(
        "characters.memory.named",
        ch.memory.named.into(),
        1.0,
        100.0,
    );
    r(
        "characters.memory.forget_below",
        ch.memory.forget_below,
        0.0,
        1.0,
    );
    for (k, m) in &ch.memories {
        let p = format!("characters.memories.{k}");
        r(&format!("{p}.sentiment"), m.sentiment, -1.0, 1.0);
        r(&format!("{p}.magnitude"), m.magnitude, 0.0, 1.0);
        r(&format!("{p}.decay"), m.decay, 0.0, 20.0);
        r(&format!("{p}.opinion"), m.opinion, -100.0, 100.0);
        r(&format!("{p}.trust"), m.trust, -1.0, 1.0);
    }
    let g = &ch.generation;
    for (name, [lo, hi]) in [("personality", g.personality), ("competency", g.competency)] {
        r(&format!("characters.generation.{name}[0]"), lo, 0.0, 1.0);
        r(&format!("characters.generation.{name}[1]"), hi, lo, 1.0);
    }
    r("characters.generation.values_sd", g.values_sd, 0.0, 1.0);
    r(
        "characters.generation.first_impression_sd",
        g.first_impression_sd,
        0.0,
        100.0,
    );
    r(
        "characters.generation.family_chance",
        g.family_chance,
        0.0,
        1.0,
    );
    r("characters.generation.drift_sd", g.drift_sd, 0.0, 1.0);
    r(
        "characters.generation.trust_initial",
        g.trust_initial,
        0.0,
        1.0,
    );
    r(
        "characters.bonds.ally_above",
        ch.bonds.ally_above,
        -100.0,
        100.0,
    );
    r(
        "characters.bonds.rival_below",
        ch.bonds.rival_below,
        -100.0,
        ch.bonds.ally_above,
    );
    r(
        "council.loyalty_discipline",
        b.council.loyalty_discipline,
        0.0,
        5.0,
    );
    r(
        "council.lobby.integrity",
        b.council.lobby.integrity,
        0.0,
        10.0,
    );
    r(
        "characters.ai.warmth_opinion",
        ch.ai.warmth_opinion,
        0.0,
        1.0,
    );
    let a = &ch.ambient;
    r(
        "characters.ambient.present[0]",
        a.present[0].into(),
        0.0,
        10.0,
    );
    r(
        "characters.ambient.present[1]",
        a.present[1].into(),
        a.present[0].into(),
        10.0,
    );
    r(
        "characters.ambient.period_days",
        a.period_days.into(),
        1.0,
        366.0,
    );
    r(
        "characters.ambient.max_listed",
        a.max_listed.into(),
        1.0,
        20.0,
    );
    r(
        "characters.town.trend_months",
        ch.town.trend_months.into(),
        1.0,
        36.0,
    );
    r(
        "characters.town.broker_seniority",
        ch.town.broker_seniority,
        0.0,
        10.0,
    );
    r(
        "characters.town.broker_stake",
        ch.town.broker_stake,
        0.0,
        10.0,
    );
    crate::narrative::director_ranges(&b.director, &mut out);
    crate::media::param_ranges(b, &mut |n, v, lo, hi| out.push((n.to_string(), v, lo, hi)));
    crate::knowledge::param_ranges(b, &mut |n, v, lo, hi| out.push((n.to_string(), v, lo, hi)));
    crate::conversation::param_ranges(b, &mut |n, v, lo, hi| out.push((n.to_string(), v, lo, hi)));
    out
}

/// Characters (DEC-031): known memory kinds, each with a parameter row;
/// traits, occupations, and crisis kinds with sane values and known
/// references. Ids are unique (claimed on load).
fn validate_characters(b: &ContentBundle, issues: &mut Vec<Issue>) {
    use crate::types::{COMPETENCIES, MEMORY_KINDS, PERSONALITY_AXES};
    let in_range = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
    let ch = &b.characters;
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

/// Concern registry (DEC-078 decision 3): at least one attitude per concern,
/// known attitude keys, weights in [-1, 1], and a sane level mapping. An empty
/// registry is allowed (every issue then weighs the same). Ids are unique
/// (claimed on load).
fn validate_concerns(b: &ContentBundle, issues: &mut Vec<Issue>) {
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

/// Scenario registry (DEC-078 decision 4): valid ids; biases finite and in
/// range; attitude shifts name known attitudes. Ids are unique (claimed on load).
fn validate_scenarios(b: &ContentBundle, issues: &mut Vec<Issue>) {
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

/// Community registry (DEC-075): at least one; exactly one without a share
/// model (it takes each neighbourhood's remainder); finite shares and
/// offsets; offsets name known attitudes. Ids are unique (claimed on load).
fn validate_communities(b: &ContentBundle, issues: &mut Vec<Issue>) {
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

/// A situation trigger (DEC-077 §3, reused by policy families): at least one
/// condition, each checked by the one condition validator as a policy's
/// unlock (no slots; the ledger refused, DEC-091; a non-local template uses
/// tier-generic conditions only, DEC-098).
pub fn check_situation(
    b: &ContentBundle,
    owner: &str,
    tier: crate::types::TierKey,
    u: &crate::types::SituationTrigger,
    issues: &mut Vec<Issue>,
) {
    if u.any.is_empty() {
        issues.push(Issue::error(
            owner,
            "a situation trigger needs at least one condition",
        ));
    }
    let site =
        crate::condition::Site::new(owner, crate::condition::ConditionUse::PolicyUnlock, &[])
            .tier(tier);
    crate::condition::check_all(b, &site, &u.any, issues);
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
