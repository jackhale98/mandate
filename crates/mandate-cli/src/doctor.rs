//! `mandate-cli content doctor` (the content workbench, owner 2026-11-19): a
//! lab report on the content pack. It finds
//!
//! - content that can never fire: contradictory or impossible bounds,
//!   conditions that contradict each other, references to ids that do not
//!   exist, and roles or places no generated town has; and content that
//!   never fired across N bot-played careers (dilemmas, situational
//!   policies, storylines and their steps, scenes, overheard lines, topics,
//!   conversation lines and voices);
//! - content that fires too often (in most careers, or far above its share
//!   of the picks of its kind);
//! - Fluent problems: keys referenced by content or named by the engine but
//!   missing from English, keys nothing references, and arguments a message
//!   uses that its caller never supplies (where that is static);
//! - text too long for a phone, rendered with representative arguments
//!   (the longest variant of every selector);
//! - voice problems: lines tagged with a voice or a role that the speakers
//!   who can say them cannot have.
//!
//! Findings have a severity (error, warn, info); `--strict` exits non-zero
//! on errors. Read-only: careers are played on throwaway engines.

use crate::bots;
use mandate_content::conversation::{
    ConcernOf, LeadToDef, TalkLineDef, TalkPart, fluent_placeholders,
};
use mandate_content::validate::fluent_keys;
use mandate_content::{
    Cast, Condition, ContentBundle, HeardCondition, OfficeRole, ScenePlace, SituationCondition,
    Source,
};
use mandate_sim::explain;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(clap::Args, Clone, Debug)]
pub struct DoctorArgs {
    /// Bot-played careers to look for content that never (or too often) fires.
    #[arg(long, default_value_t = 16)]
    pub seeds: u64,
    /// Years per career.
    #[arg(long, default_value_t = 4)]
    pub years: u32,
    /// The bots' strategy.
    #[arg(long, default_value = "mixed")]
    pub strategy: String,
    /// Print the full report as JSON.
    #[arg(long)]
    pub json: bool,
    /// Exit non-zero when there are errors.
    #[arg(long)]
    pub strict: bool,
    /// Longest title (dilemma, storyline, topic), in characters.
    #[arg(long, default_value_t = 60)]
    pub max_title: usize,
    /// Longest line (overheard, conversation, scene beat, story step).
    #[arg(long, default_value_t = 280)]
    pub max_line: usize,
    /// Longest option label (a button), in characters.
    #[arg(long, default_value_t = 90)]
    pub max_label: usize,
    /// Longest body (a dilemma's situation or aftermath).
    #[arg(long, default_value_t = 600)]
    pub max_body: usize,
    /// Share of careers above which an item fires too often.
    #[arg(long, default_value_t = 0.6)]
    pub often_careers: f64,
    /// Share of all picks of its kind above which an item fires too often
    /// (and at least 2.5 times its fair share).
    #[arg(long, default_value_t = 0.25)]
    pub often_share: f64,
    /// Source tree scanned for the Fluent keys the engine and the UI name
    /// (`crates/*/src`, `web/src`); empty to skip.
    #[arg(long, default_value = ".")]
    pub source: PathBuf,
}

impl Default for DoctorArgs {
    fn default() -> Self {
        DoctorArgs {
            seeds: 16,
            years: 4,
            strategy: "mixed".into(),
            json: false,
            strict: false,
            max_title: 60,
            max_line: 280,
            max_body: 600,
            max_label: 90,
            often_careers: 0.6,
            often_share: 0.25,
            source: PathBuf::from("."),
        }
    }
}

// --- Findings ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Sev {
    Error,
    Warn,
    Info,
}

impl Sev {
    fn key(self) -> &'static str {
        match self {
            Sev::Error => "error",
            Sev::Warn => "warn",
            Sev::Info => "info",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Finding {
    pub severity: Sev,
    /// `validate` | `unsatisfiable` | `contradiction` | `unknown-ref` |
    /// `no-town-has` | `never-fired` | `too-often` | `fluent-missing` |
    /// `fluent-unused` | `fluent-args` | `too-long` | `voice`.
    pub check: String,
    /// The content file (or `.ftl`) that defines the item.
    pub file: String,
    pub item: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct KindStats {
    pub kind: String,
    pub items: usize,
    /// Items seen firing in at least one career.
    pub fired: usize,
    pub never: usize,
    pub too_often: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Report {
    pub content_hash: String,
    pub seeds: u64,
    pub years: u32,
    pub strategy: String,
    pub careers: usize,
    pub coverage: Vec<KindStats>,
    /// (check, errors, warns, infos).
    pub summary: Vec<(String, usize, usize, usize)>,
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == Sev::Error)
            .count()
    }

    #[cfg(test)]
    pub fn has(&self, check: &str, item: &str) -> bool {
        self.findings
            .iter()
            .any(|f| f.check == check && f.item == item)
    }
}

struct Out {
    findings: Vec<Finding>,
    files: BTreeMap<String, String>,
}

impl Out {
    fn file_of(&self, item: &str) -> String {
        self.files.get(item).cloned().unwrap_or_else(|| "?".into())
    }

    fn add(&mut self, sev: Sev, check: &str, item: &str, message: impl Into<String>) {
        let file = self.file_of(item);
        self.findings.push(Finding {
            severity: sev,
            check: check.into(),
            file,
            item: item.into(),
            message: message.into(),
        });
    }
}

/// Where each id (`"id": "…"`) and each Fluent key is defined.
fn origins(sources: &[Source]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for s in sources {
        if s.path.ends_with(".ftl") {
            for k in fluent_keys(&s.text) {
                out.entry(k).or_insert_with(|| s.path.clone());
            }
            continue;
        }
        let mut rest = s.text.as_str();
        while let Some(i) = rest.find("\"id\"") {
            rest = &rest[i + 4..];
            let after = rest.trim_start();
            let Some(after) = after.strip_prefix(':') else {
                continue;
            };
            let after = after.trim_start();
            if let Some(v) = after.strip_prefix('"')
                && let Some(end) = v.find('"')
            {
                out.entry(v[..end].to_string())
                    .or_insert_with(|| s.path.clone());
            }
        }
    }
    out
}

// --- Loading ---------------------------------------------------------------------------

const STUBS: &str = "locale/en/zz-doctor-stubs.ftl";

/// Load the pack. Missing English keys stop a normal load; the doctor stubs
/// them (and reports each) so the rest can be checked. Any other error is
/// reported and ends the run.
fn load(sources: &[Source], out: &mut Out) -> Option<Arc<ContentBundle>> {
    match mandate_content::load(sources) {
        Ok((b, warnings)) => {
            for w in warnings {
                out.add(Sev::Warn, "validate", &w.file, w.message);
            }
            Some(Arc::new(b))
        }
        Err(issues) => {
            let mut missing = BTreeSet::new();
            let mut fatal = false;
            for i in &issues {
                let key = i
                    .message
                    .strip_prefix("missing Fluent key `")
                    .and_then(|r| r.split('`').next());
                match (i.severity, key) {
                    (_, Some(k)) => {
                        missing.insert(k.to_string());
                        out.add(
                            Sev::Error,
                            "fluent-missing",
                            &i.file,
                            format!("`{k}` is referenced but missing in English"),
                        );
                    }
                    (mandate_content::Severity::Error, None) => {
                        fatal = true;
                        out.add(Sev::Error, "validate", &i.file, i.message.clone());
                    }
                    (mandate_content::Severity::Warning, None) => {
                        out.add(Sev::Warn, "validate", &i.file, i.message.clone())
                    }
                }
            }
            if fatal || missing.is_empty() {
                return None;
            }
            let mut stub = String::new();
            for k in &missing {
                stub.push_str(&format!("{k} = ⟦{k}⟧\n"));
            }
            let mut with = sources.to_vec();
            with.push(Source {
                path: STUBS.into(),
                text: stub,
            });
            match mandate_content::load(&with) {
                Ok((b, _)) => Some(Arc::new(b)),
                Err(issues) => {
                    for i in issues {
                        out.add(Sev::Error, "validate", &i.file, i.message);
                    }
                    None
                }
            }
        }
    }
}

// --- Static: bounds and contradictions -----------------------------------------------------

/// A numeric quantity a condition bounds, its interval, and the quantity's
/// possible range.
struct Bound {
    quantity: String,
    lo: f64,
    hi: f64,
    domain: (f64, f64),
}

fn bound(quantity: String, min: Option<f64>, max: Option<f64>, domain: (f64, f64)) -> Bound {
    Bound {
        quantity,
        lo: min.unwrap_or(f64::NEG_INFINITY),
        hi: max.unwrap_or(f64::INFINITY),
        domain,
    }
}

const INF: f64 = f64::INFINITY;

fn indicator_domain(i: mandate_content::Indicator) -> (f64, f64) {
    use mandate_content::Indicator as I;
    match i {
        I::Unemployment | I::RecentDisaster | I::JobLoss => (0.0, 1.0),
        I::UnemploymentRise => (-1.0, 1.0),
        I::Pollution => (0.0, 3.0),
        I::RentBurden | I::TaxBurden | I::BudgetStress => (0.0, INF),
    }
}

fn narrative_bound(c: &Condition) -> Option<Bound> {
    Some(match c {
        Condition::Crisis { kind, min_severity } => bound(
            format!("crisis {kind} severity"),
            Some(*min_severity),
            None,
            (0.0, 1.0),
        ),
        Condition::LiveCrisis { min_severity } => bound(
            "live crisis severity".into(),
            Some(*min_severity),
            None,
            (0.0, 1.0),
        ),
        Condition::Indicator {
            indicator,
            min,
            max,
        } => bound(
            format!("indicator {indicator:?}"),
            *min,
            *max,
            indicator_domain(*indicator),
        ),
        // Every track, the record and the Mandate run 0..100.
        Condition::Track { track, min, max } => {
            bound(format!("track {track:?}"), *min, *max, (0.0, 100.0))
        }
        Condition::Money { min, max } => bound("money".into(), *min, *max, (-INF, INF)),
        Condition::Opinion { role, min, max } => {
            bound(format!("opinion of {role}"), *min, *max, (-100.0, 100.0))
        }
        Condition::Personality {
            role,
            axis,
            min,
            max,
        } => bound(format!("{role} {axis}"), *min, *max, (0.0, 1.0)),
        _ => return None,
    })
}

fn situation_bound(c: &SituationCondition) -> Option<Bound> {
    use SituationCondition as U;
    Some(match c {
        U::Crisis { kind, min_severity } => bound(
            format!("crisis {kind} severity"),
            Some(*min_severity),
            None,
            (0.0, 1.0),
        ),
        U::Indicator {
            indicator,
            min,
            max,
        } => bound(
            format!("indicator {indicator:?}"),
            *min,
            *max,
            indicator_domain(*indicator),
        ),
        U::Personality {
            slot,
            axis,
            min,
            max,
        } => bound(format!("{slot} {axis}"), *min, *max, (0.0, 1.0)),
        U::Opinion { slot, min, max } => {
            bound(format!("opinion of {slot}"), *min, *max, (-100.0, 100.0))
        }
        U::Trust { slot, min, max } => bound(format!("trust of {slot}"), *min, *max, (0.0, 1.0)),
        _ => return None,
    })
}

fn heard_bound(c: &HeardCondition) -> Option<Bound> {
    Some(match c {
        HeardCondition::PlayerAwareness { min, max } => {
            bound("player awareness".into(), *min, *max, (0.0, 1.0))
        }
        HeardCondition::PlayerApproval { min, max } => {
            bound("player approval".into(), *min, *max, (-1.0, 1.0))
        }
        HeardCondition::RivalRecord { min, max } => {
            bound("rival record".into(), *min, *max, (0.0, 1.0))
        }
        _ => return None,
    })
}

/// A discrete quantity and the values a condition allows.
fn narrative_discrete(c: &Condition) -> Option<(String, BTreeSet<String>)> {
    let set = |xs: &[&str]| xs.iter().map(|x| x.to_string()).collect::<BTreeSet<_>>();
    Some(match c {
        Condition::Campaigning { is } => ("campaigning".into(), set(&[&is.to_string()])),
        Condition::Office { role } => (
            "office".into(),
            match role {
                OfficeRole::Councillor => set(&["councillor"]),
                OfficeRole::Mayor => set(&["mayor"]),
                OfficeRole::Any => set(&["councillor", "mayor"]),
                OfficeRole::None => set(&["none"]),
            },
        ),
        _ => return None,
    })
}

fn situation_discrete(c: &SituationCondition) -> Option<(String, BTreeSet<String>)> {
    use SituationCondition as U;
    Some(match c {
        U::Occupation { slot, occupations } => (
            format!("role of {slot}"),
            occupations.iter().cloned().collect(),
        ),
        U::IsBroker { slot, is } => (format!("{slot} is the broker"), [is.to_string()].into()),
        U::IsRival { slot, is } => (format!("{slot} is the rival"), [is.to_string()].into()),
        U::Voice { voices } => ("voice".into(), voices.iter().cloned().collect()),
        U::Concern {
            of: ConcernOf::Town,
            concerns,
        } => (
            "the town's top concern".into(),
            concerns.iter().cloned().collect(),
        ),
        U::Concern {
            of: ConcernOf::Speaker,
            concerns,
        } => (
            "the speaker's top concern".into(),
            concerns.iter().cloned().collect(),
        ),
        _ => return None,
    })
}

/// Check one conjunction: each bound against its range and itself, then
/// bounds and discrete values against each other.
fn check_conjunction(
    out: &mut Out,
    item: &str,
    what: &str,
    bounds: Vec<Bound>,
    discrete: Vec<(String, BTreeSet<String>)>,
) {
    let mut by: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for b in &bounds {
        if b.lo > b.hi {
            out.add(
                Sev::Error,
                "unsatisfiable",
                item,
                format!(
                    "{what}: {} bounds contradict (min {} > max {})",
                    b.quantity, b.lo, b.hi
                ),
            );
        } else if b.lo > b.domain.1 || b.hi < b.domain.0 {
            out.add(
                Sev::Error,
                "unsatisfiable",
                item,
                format!(
                    "{what}: {} in [{}, {}] is outside its possible range [{}, {}]",
                    b.quantity, b.lo, b.hi, b.domain.0, b.domain.1
                ),
            );
        }
        let e = by.entry(b.quantity.clone()).or_insert((-INF, INF));
        e.0 = e.0.max(b.lo);
        e.1 = e.1.min(b.hi);
    }
    for (q, (lo, hi)) in &by {
        let n = bounds.iter().filter(|b| &b.quantity == q).count();
        if n > 1 && lo > hi {
            out.add(
                Sev::Error,
                "contradiction",
                item,
                format!("{what}: the conditions on {q} cannot all hold (need ≥ {lo} and ≤ {hi})"),
            );
        }
    }
    let mut sets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for (q, s) in discrete {
        *counts.entry(q.clone()).or_default() += 1;
        match sets.get_mut(&q) {
            Some(cur) => *cur = cur.intersection(&s).cloned().collect(),
            None => {
                sets.insert(q, s);
            }
        }
    }
    for (q, s) in sets {
        if s.is_empty() && counts.get(&q).copied().unwrap_or(0) > 1 {
            out.add(
                Sev::Error,
                "contradiction",
                item,
                format!("{what}: the conditions on {q} allow no value together"),
            );
        }
    }
}

fn check_narrative(out: &mut Out, item: &str, what: &str, cs: &[Condition]) {
    let bounds = cs.iter().filter_map(narrative_bound).collect();
    let discrete = cs.iter().filter_map(narrative_discrete).collect();
    check_conjunction(out, item, what, bounds, discrete);
    // Two `law_in_force` on the same templates, one absent and one present.
    for a in cs {
        for b in cs {
            if let (
                Condition::LawInForce {
                    templates: ta,
                    absent: true,
                },
                Condition::LawInForce {
                    templates: tb,
                    absent: false,
                },
            ) = (a, b)
                && tb.iter().all(|t| ta.contains(t))
            {
                out.add(
                    Sev::Error,
                    "contradiction",
                    item,
                    format!("{what}: a law must be both absent and in force"),
                );
            }
        }
    }
}

fn check_situation(out: &mut Out, item: &str, what: &str, cs: &[SituationCondition]) {
    let bounds = cs.iter().filter_map(situation_bound).collect();
    let discrete = cs.iter().filter_map(situation_discrete).collect();
    check_conjunction(out, item, what, bounds, discrete);
    for c in cs {
        if let SituationCondition::Any { of } = c {
            for (i, x) in of.iter().enumerate() {
                check_situation(
                    out,
                    item,
                    &format!("{what} (any, alternative {})", i + 1),
                    std::slice::from_ref(x),
                );
            }
        }
    }
}

fn check_heard(out: &mut Out, item: &str, what: &str, cs: &[HeardCondition]) {
    let bounds = cs.iter().filter_map(heard_bound).collect();
    check_conjunction(out, item, what, bounds, Vec::new());
    let dry = cs.iter().any(|c| matches!(c, HeardCondition::Dry));
    let wet = cs.iter().any(|c| matches!(c, HeardCondition::Water { .. }));
    if dry && wet {
        out.add(
            Sev::Error,
            "contradiction",
            item,
            format!("{what}: `dry` and `water` together"),
        );
    }
}

/// The speaker roles a line's conditions allow (`None`: any).
fn speaker_roles(cs: &[SituationCondition]) -> Option<BTreeSet<String>> {
    let mut out: Option<BTreeSet<String>> = None;
    for c in cs {
        if let SituationCondition::Occupation { slot, occupations } = c
            && slot == "speaker"
        {
            let s: BTreeSet<String> = occupations.iter().cloned().collect();
            out = Some(match out {
                Some(o) => o.intersection(&s).cloned().collect(),
                None => s,
            });
        }
    }
    out
}

fn static_checks(b: &ContentBundle, out: &mut Out) {
    let crisis = |k: &str| b.crises.iter().any(|c| c.id == k);
    let concern = |k: &str| b.concerns.iter().any(|c| c.id == k);
    let roles_all = b.talk_roles();
    let unknown = |out: &mut Out, item: &str, what: &str, id: &str| {
        out.add(
            Sev::Error,
            "unknown-ref",
            item,
            format!("{what} `{id}` does not exist"),
        );
    };
    let narrative_refs = |out: &mut Out, item: &str, roles: &[&str], cs: &[Condition]| {
        for c in cs {
            for r in c.roles() {
                if !roles.contains(&r) {
                    out.add(
                        Sev::Error,
                        "unknown-ref",
                        item,
                        format!("a condition names role `{r}`, which the item does not cast"),
                    );
                }
            }
            match c {
                Condition::Crisis { kind, .. } if !crisis(kind) => {
                    unknown(out, item, "crisis", kind)
                }
                Condition::Concern { concern: k, top } => {
                    if !concern(k) {
                        unknown(out, item, "concern", k);
                    }
                    if *top == 0 {
                        out.add(
                            Sev::Error,
                            "unsatisfiable",
                            item,
                            "a concern among the top 0 never holds",
                        );
                    }
                }
                Condition::Trait { traits, .. } => {
                    for t in traits {
                        if b.trait_def(t).is_none() {
                            unknown(out, item, "trait", t);
                        }
                    }
                }
                Condition::LawInForce { templates, .. } => {
                    for t in templates {
                        if !b.policies.contains_key(t) {
                            unknown(out, item, "policy template", t);
                        }
                    }
                }
                Condition::Goal { goal, .. } if b.goal(goal).is_none() => {
                    unknown(out, item, "goal", goal)
                }
                Condition::Remembers { kinds, .. } => {
                    for k in kinds {
                        if !mandate_content::MEMORY_KINDS.contains(&k.as_str()) {
                            unknown(out, item, "memory kind", k);
                        }
                    }
                }
                _ => {}
            }
        }
    };
    // Dilemmas.
    for d in &b.dilemmas {
        let mut roles: Vec<&str> = d.roles.iter().map(|r| r.id.as_str()).collect();
        // A storyline stage's dilemma may name the storyline's roles.
        for s in &b.storylines {
            if s.stages
                .iter()
                .any(|x| x.decision.as_deref() == Some(d.id.as_str()))
            {
                roles.extend(s.roles.iter().map(|r| r.id.as_str()));
            }
        }
        check_narrative(out, &d.id, "triggers", &d.triggers);
        narrative_refs(out, &d.id, &roles, &d.triggers);
        if let Some(s) = &d.scene
            && b.scene(s).is_none()
        {
            unknown(out, &d.id, "scene", s);
        }
        if d.category == mandate_content::DilemmaCategory::Request
            && !b
                .goals
                .iter()
                .any(|g| g.request.as_deref() == Some(d.id.as_str()))
        {
            out.add(
                Sev::Error,
                "unsatisfiable",
                &d.id,
                "a request that no goal raises can never come up",
            );
        }
        if d.storyline_only
            && !b.storylines.iter().any(|s| {
                s.stages
                    .iter()
                    .any(|x| x.decision.as_deref() == Some(d.id.as_str()))
            })
        {
            out.add(
                Sev::Error,
                "unsatisfiable",
                &d.id,
                "storyline-only, but no storyline stage presents it",
            );
        }
        for r in &d.roles {
            check_cast(b, out, &d.id, &r.cast);
        }
    }
    // Goals.
    for g in &b.goals {
        check_narrative(out, &g.id, "conditions", &g.conditions);
        narrative_refs(out, &g.id, &["holder"], &g.conditions);
        for (axis, [lo, hi]) in &g.personality {
            if lo > hi || *lo > 1.0 || *hi < 0.0 {
                out.add(
                    Sev::Error,
                    "unsatisfiable",
                    &g.id,
                    format!("holder's {axis} in [{lo}, {hi}] never holds"),
                );
            }
        }
        if let Some(r) = &g.request
            && b.dilemma(r).is_none()
        {
            unknown(out, &g.id, "request dilemma", r);
        }
    }
    // Storylines.
    for s in &b.storylines {
        let roles: Vec<&str> = s.roles.iter().map(|r| r.id.as_str()).collect();
        check_narrative(out, &s.id, "start", &s.start);
        narrative_refs(out, &s.id, &roles, &s.start);
        for st in &s.stages {
            check_narrative(out, &s.id, &format!("stage {} advance", st.id), &st.advance);
            narrative_refs(out, &s.id, &roles, &st.advance);
            if let Some(d) = &st.decision
                && b.dilemma(d).is_none()
            {
                unknown(out, &s.id, "stage decision", d);
            }
        }
        if let Some(f) = &s.follow_up
            && b.storyline(f).is_none()
        {
            unknown(out, &s.id, "follow-up storyline", f);
        }
        for r in &s.roles {
            check_cast(b, out, &s.id, &r.cast);
        }
    }
    // Scenes nobody presents.
    for sc in &b.scenes {
        if !b
            .dilemmas
            .iter()
            .any(|d| d.scene.as_deref() == Some(sc.id.as_str()))
        {
            out.add(
                Sev::Error,
                "unsatisfiable",
                &sc.id,
                "no dilemma presents this scene",
            );
        }
    }
    // Situational policies: each alternative of the unlock.
    for p in b.policies.values() {
        if let Some(u) = &p.unlock {
            for (i, c) in u.any.iter().enumerate() {
                check_situation(
                    out,
                    &p.id,
                    &format!("unlock alternative {}", i + 1),
                    std::slice::from_ref(c),
                );
                situation_refs(b, out, &p.id, std::slice::from_ref(c));
            }
        }
    }
    // Overheard lines.
    for o in &b.overheard {
        check_heard(out, &o.id, "when", &o.when);
        for c in &o.when {
            if let HeardCondition::Crisis { kind: Some(k) } = c
                && !crisis(k)
            {
                unknown(out, &o.id, "crisis", k);
            }
        }
        if let Some(c) = &o.concern
            && !concern(c)
        {
            unknown(out, &o.id, "concern", c);
        }
    }
    // Topics, voices, lines; with the voice and topic they need.
    for t in &b.talk_topics {
        check_situation(out, &t.id, "offered when", &t.offered.when);
        situation_refs(b, out, &t.id, &t.offered.when);
        for r in &t.offered.roles {
            if !roles_all.contains(r) {
                unknown(out, &t.id, "role", r);
            }
        }
    }
    // A voice never wins below an unconditional voice of higher priority
    // (or of equal priority, loaded earlier: ties go to load order).
    for (vi, v) in b.talk_voices.iter().enumerate() {
        check_situation(out, &v.id, "when", &v.when);
        situation_refs(b, out, &v.id, &v.when);
        if let Some(w) = b.talk_voices.iter().enumerate().find_map(|(wi, w)| {
            (wi != vi
                && w.when.is_empty()
                && (w.priority > v.priority || (w.priority == v.priority && wi < vi)))
                .then_some(w)
        }) {
            out.add(
                Sev::Error,
                "voice",
                &v.id,
                format!(
                    "never a speaker's voice: `{}` has no conditions and priority {} ≥ {}",
                    w.id, w.priority, v.priority
                ),
            );
        }
    }
    for l in &b.talk_lines {
        check_situation(out, &l.id, "when", &l.when);
        situation_refs(b, out, &l.id, &l.when);
        line_voice_checks(b, out, l);
    }
}

fn check_cast(b: &ContentBundle, out: &mut Out, item: &str, c: &Cast) {
    match c {
        Cast::Occupation { occupations } => {
            for o in occupations {
                if b.occupation(o).is_none() {
                    out.add(
                        Sev::Error,
                        "unknown-ref",
                        item,
                        format!("cast names unknown occupation `{o}`"),
                    );
                }
            }
        }
        Cast::Law { templates, .. } => {
            for t in templates {
                if !b.policies.contains_key(t) {
                    out.add(
                        Sev::Error,
                        "unknown-ref",
                        item,
                        format!("cast names unknown policy template `{t}`"),
                    );
                }
            }
        }
        _ => {}
    }
}

fn situation_refs(b: &ContentBundle, out: &mut Out, item: &str, cs: &[SituationCondition]) {
    use SituationCondition as U;
    let roles = b.talk_roles();
    for c in cs {
        match c {
            U::Crisis { kind, .. } if !b.crises.iter().any(|x| &x.id == kind) => out.add(
                Sev::Error,
                "unknown-ref",
                item,
                format!("crisis `{kind}` does not exist"),
            ),
            U::Occupation { occupations, .. } => {
                for o in occupations {
                    if !roles.contains(o) {
                        out.add(
                            Sev::Error,
                            "unknown-ref",
                            item,
                            format!("role `{o}` does not exist"),
                        );
                    }
                }
            }
            U::Voice { voices } => {
                for v in voices {
                    if b.talk_voice(v).is_none() {
                        out.add(
                            Sev::Error,
                            "unknown-ref",
                            item,
                            format!("voice `{v}` does not exist"),
                        );
                    }
                }
            }
            U::Concern { concerns, .. } => {
                for k in concerns {
                    if !b.concerns.iter().any(|x| &x.id == k) {
                        out.add(
                            Sev::Error,
                            "unknown-ref",
                            item,
                            format!("concern `{k}` does not exist"),
                        );
                    }
                }
            }
            U::Any { of } => situation_refs(b, out, item, of),
            _ => {}
        }
    }
}

/// Voice and role problems of a line: a closer's voice (or an opener's
/// `voice` condition) whose speakers can never have the line's roles, a
/// substance line for roles its topic is never offered to, and the line's
/// conditions together with the voice's or the topic's.
fn line_voice_checks(b: &ContentBundle, out: &mut Out, l: &TalkLineDef) {
    let mine = speaker_roles(&l.when);
    let voices: Vec<&str> = match (&l.voice, l.part) {
        (Some(v), TalkPart::Closer) => vec![v.as_str()],
        _ => l
            .when
            .iter()
            .filter_map(|c| match c {
                SituationCondition::Voice { voices } => {
                    Some(voices.iter().map(|x| x.as_str()).collect::<Vec<_>>())
                }
                _ => None,
            })
            .next()
            .unwrap_or_default(),
    };
    if !voices.is_empty() {
        let possible: Vec<&str> = voices
            .iter()
            .copied()
            .filter(|v| {
                let Some(vd) = b.talk_voice(v) else {
                    return false;
                };
                let vr = speaker_roles(&vd.when);
                match (&mine, vr) {
                    (Some(a), Some(bb)) => a.intersection(&bb).next().is_some(),
                    _ => true,
                }
            })
            .collect();
        if possible.is_empty() {
            out.add(
                Sev::Error,
                "voice",
                &l.id,
                format!(
                    "speakers with voice {} never have this line's roles ({})",
                    voices.join(" / "),
                    mine.as_ref()
                        .map(|m| m.iter().cloned().collect::<Vec<_>>().join(", "))
                        .unwrap_or_default()
                ),
            );
        }
        for v in &voices {
            if let Some(vd) = b.talk_voice(v) {
                let mut all = vd.when.clone();
                all.extend(
                    l.when
                        .iter()
                        .filter(|c| !matches!(c, SituationCondition::Voice { .. }))
                        .cloned(),
                );
                check_situation(out, &l.id, &format!("with voice {v}"), &all);
            }
        }
    }
    if let Some(t) = l.topic.as_deref().and_then(|k| b.talk_topic(k)) {
        if l.part == TalkPart::Substance
            && !t.offered.roles.is_empty()
            && let Some(m) = &mine
            && !m.iter().any(|r| t.offered.roles.contains(r))
        {
            out.add(
                Sev::Error,
                "voice",
                &l.id,
                format!(
                    "its roles ({}) are never offered topic `{}` ({})",
                    m.iter().cloned().collect::<Vec<_>>().join(", "),
                    t.id,
                    t.offered.roles.join(", ")
                ),
            );
        }
        let mut all = t.offered.when.clone();
        all.extend(l.when.iter().cloned());
        check_situation(out, &l.id, &format!("with topic {}", t.id), &all);
    }
    if let Some(lead) = &l.lead
        && let LeadToDef::PlaceKind { kind } = &lead.to
        && b.place_kind(kind).is_none()
    {
        out.add(
            Sev::Error,
            "unknown-ref",
            &l.id,
            format!("lead to unknown place kind `{kind}`"),
        );
    }
}

// --- Simulation: what fired in N careers -------------------------------------------------

/// What one career saw.
#[derive(Clone, Debug, Default)]
struct Seen {
    /// Item key (`kind:id`) → picks.
    picks: BTreeMap<String, u64>,
    /// Lines whose conditions held for some speaker (not necessarily picked).
    held: BTreeSet<String>,
    place_kinds: BTreeSet<String>,
    roles: BTreeSet<String>,
    role_voice: BTreeSet<(String, String)>,
}

impl Seen {
    fn pick(&mut self, kind: &str, id: &str) {
        *self.picks.entry(format!("{kind}:{id}")).or_default() += 1;
    }
}

/// How often the talk sample runs (days).
const TALK_SAMPLE_DAYS: u32 = 91;

/// A copy of the state in which everyone passing by in town has been met:
/// passers-by (residents, retirees, clergy, farmers…) are speakers too, and
/// only exist once met. The career's own engine is untouched.
fn with_passers_by(e: &Engine, content: &ContentBundle) -> mandate_sim::State {
    use mandate_sim::places::Location;
    let mut st = e.state().clone();
    let r = mandate_sim::scheduler::Reads::of_state(&st, content);
    let v = mandate_sim::narrative::cast::View::new(&r, &st.politics);
    let Some(m) = v.town() else { return st };
    let mut locs: Vec<Location> = st
        .world
        .neighborhoods
        .iter()
        .enumerate()
        .filter(|(_, h)| h.municipality.0 == m)
        .map(|(i, _)| Location::Street {
            neighborhood: mandate_sim::world::NeighborhoodId(i as u32),
        })
        .collect();
    locs.extend(
        st.world
            .places
            .iter()
            .filter(|p| p.municipality.0 == m)
            .map(|p| Location::Place { place: p.id }),
    );
    for loc in locs {
        let slots: Vec<u8> = mandate_sim::characters::ambient::ambient_at(&st, content, loc)
            .into_iter()
            .map(|(s, _)| s)
            .collect();
        for s in slots {
            mandate_sim::characters::ambient::promote(&mut st, content, loc, s);
        }
    }
    st
}

fn sample_talk(e: &Engine, content: &ContentBundle, seen: &mut Seen) {
    let met = with_passers_by(e, content);
    let st = &met;
    let week = mandate_sim::talk::week_of(st.calendar.date);
    for s in explain::speakers(st, content) {
        let role = mandate_sim::characters::knows::role_key(st, s);
        if let Some(r) = &role {
            seen.roles.insert(r.clone());
        }
        let Some(ctx) = explain::talk_ctx(st, content, s, None) else {
            continue;
        };
        let voice = mandate_sim::talk::reply::voice(&ctx);
        if let Some(v) = voice {
            seen.pick("voice", &v.id);
            if let Some(r) = &role {
                seen.role_voice.insert((r.clone(), v.id.clone()));
            }
        }
        for t in &content.talk_topics {
            let o = &t.offered;
            if !o.roles.is_empty() && !role.as_ref().is_some_and(|r| o.roles.contains(r)) {
                continue;
            }
            // Without a subject (when it needs none), then with one of each
            // kind it takes.
            let mut asked = false;
            for about in explain::sample_abouts(st, content, &t.about, t.about_required) {
                let Some(ctx) = explain::talk_ctx(st, content, s, about.as_ref()) else {
                    continue;
                };
                if !ctx.all_hold(&o.when, &mut Default::default()) {
                    continue;
                }
                asked = true;
                for part in TalkPart::ALL {
                    let (cands, _) = mandate_sim::talk::reply::candidates(&ctx, t, part, voice);
                    for (l, _) in cands {
                        seen.held.insert(l.id.clone());
                    }
                }
                let reply = mandate_sim::talk::reply::compose(&ctx, t, &[], week, 0);
                for c in reply.parts {
                    seen.pick("line", &c.line.id);
                }
            }
            if asked {
                seen.pick("topic", &t.id);
            }
        }
    }
}

fn career(content: &Arc<ContentBundle>, seed: u64, years: u32, strategy: &str) -> Option<Seen> {
    let mut e = generate(&WorldConfig::new(seed, WorldSize::Small), content.clone()).ok()?;
    let mut seen = Seen::default();
    {
        let st = e.state();
        // The town's place kinds (static for a run).
        let r = mandate_sim::scheduler::Reads::of_state(st, content);
        let v = mandate_sim::narrative::cast::View::new(&r, &st.politics);
        if let Some(m) = v.town() {
            for p in st.world.places.iter().filter(|p| p.municipality.0 == m) {
                seen.place_kinds.insert(p.kind.key().to_string());
            }
        }
    }
    let start = e.state().calendar.date;
    let end = start.add_days(years * 365);
    let start_turn = e.state().calendar.turn;
    let mut last_decision = 0u32;
    let mut next_talk = start;
    let mut runs_seen: BTreeSet<(u32, usize)> = BTreeSet::new();
    while e.state().calendar.date < end {
        bots::play_turn_from(&mut e, content, strategy, start_turn);
        if e.apply(Command::EndTurn).is_err() {
            break;
        }
        let st = e.state();
        // Decisions opened (and the scenes that present them).
        for d in st
            .narrative
            .decisions
            .iter()
            .filter(|d| d.id.0 >= last_decision)
        {
            if d.id.0 < last_decision {
                continue;
            }
            seen.pick("dilemma", &d.template);
            if d.place.is_some()
                && let Some(s) = content.dilemma(&d.template).and_then(|x| x.scene.as_ref())
            {
                seen.pick("scene", s);
            }
        }
        last_decision = st
            .narrative
            .decisions
            .iter()
            .map(|d| d.id.0 + 1)
            .max()
            .unwrap_or(last_decision);
        // Storylines started and the steps they reached.
        for run in &st.narrative.storylines {
            if runs_seen.insert((run.id.0, usize::MAX)) {
                seen.pick("storyline", &run.template);
            }
            for (i, (stage, _)) in run.stages.iter().enumerate() {
                if runs_seen.insert((run.id.0, i)) {
                    seen.pick("step", &format!("{}/{stage}", run.template));
                }
            }
        }
        // Situational policies on the table in the player's town.
        let r = mandate_sim::scheduler::Reads::of_state(st, content);
        let v = mandate_sim::narrative::cast::View::new(&r, &st.politics);
        if let Some(m) = v.town() {
            for t in content.policies.values().filter(|p| p.unlock.is_some()) {
                if mandate_sim::policy::unlocked(st, content, t, m) {
                    seen.pick("situational", &t.id);
                }
            }
        }
        // This week's talk at every place where people talk.
        for p in explain::talk_places(st, content) {
            let loc = mandate_sim::places::Location::Place { place: p };
            for l in mandate_sim::overheard::lines(st, content, loc) {
                seen.pick("overheard", &l.id);
            }
            for l in mandate_sim::overheard::available(st, content, loc) {
                seen.held.insert(l.id);
            }
        }
        if st.calendar.date >= next_talk {
            sample_talk(&e, content, &mut seen);
            next_talk = e.state().calendar.date.add_days(TALK_SAMPLE_DAYS);
        }
    }
    Some(seen)
}

fn careers(content: &Arc<ContentBundle>, args: &DoctorArgs) -> Vec<Seen> {
    let seeds: Vec<u64> = (1..=args.seeds).collect();
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, 4);
    let mut results: Vec<(u64, Seen)> = Vec::new();
    for chunk in seeds.chunks(workers) {
        std::thread::scope(|s| {
            let hs: Vec<_> = chunk
                .iter()
                .map(|seed| {
                    let c = content.clone();
                    let seed = *seed;
                    let strategy = args.strategy.clone();
                    let years = args.years;
                    s.spawn(move || (seed, career(&c, seed, years, &strategy)))
                })
                .collect();
            for h in hs {
                if let Ok((seed, Some(x))) = h.join() {
                    results.push((seed, x));
                }
            }
        });
    }
    results.sort_by_key(|r| r.0);
    results.into_iter().map(|r| r.1).collect()
}

/// Items by kind, as the doctor counts them: (kind, id, file item, group for shares).
fn all_items(b: &ContentBundle) -> Vec<(&'static str, String, String, Option<String>)> {
    let mut out = Vec::new();
    for d in &b.dilemmas {
        // Requests, raised and storyline-only dilemmas come up by their own
        // routes; the spotlight's share is among the spotlight's.
        let g = (d.category != mandate_content::DilemmaCategory::Request
            && !d.storyline_only
            && d.raised_by.is_none())
        .then(|| "spotlight".to_string());
        out.push(("dilemma", d.id.clone(), d.id.clone(), g));
    }
    for p in b.policies.values().filter(|p| p.unlock.is_some()) {
        out.push(("situational", p.id.clone(), p.id.clone(), None));
    }
    for s in &b.storylines {
        out.push(("storyline", s.id.clone(), s.id.clone(), None));
        for st in &s.stages {
            out.push(("step", format!("{}/{}", s.id, st.id), s.id.clone(), None));
        }
    }
    for s in &b.scenes {
        out.push(("scene", s.id.clone(), s.id.clone(), None));
    }
    for o in &b.overheard {
        out.push((
            "overheard",
            o.id.clone(),
            o.id.clone(),
            Some("overheard".into()),
        ));
    }
    for t in &b.talk_topics {
        out.push(("topic", t.id.clone(), t.id.clone(), None));
    }
    for l in &b.talk_lines {
        let g = match l.part {
            TalkPart::Substance => format!("substance:{}", l.topic.clone().unwrap_or_default()),
            TalkPart::Closer => format!("closer:{}", l.voice.clone().unwrap_or_default()),
            p => p.key().to_string(),
        };
        out.push(("line", l.id.clone(), l.id.clone(), Some(g)));
    }
    for v in &b.talk_voices {
        out.push(("voice", v.id.clone(), v.id.clone(), None));
    }
    out
}

fn dynamic_checks(
    b: &ContentBundle,
    seen: &[Seen],
    args: &DoctorArgs,
    out: &mut Out,
) -> Vec<KindStats> {
    let n = seen.len();
    let items = all_items(b);
    let mut stats: BTreeMap<&str, KindStats> = BTreeMap::new();
    // Picks per share group.
    let mut group_total: BTreeMap<String, u64> = BTreeMap::new();
    let mut group_size: BTreeMap<String, usize> = BTreeMap::new();
    for (kind, id, _, g) in &items {
        if let Some(g) = g {
            *group_size.entry(g.clone()).or_default() += 1;
            let k = format!("{kind}:{id}");
            *group_total.entry(g.clone()).or_default() += seen
                .iter()
                .map(|s| s.picks.get(&k).copied().unwrap_or(0))
                .sum::<u64>();
        }
    }
    for (kind, id, item, g) in &items {
        let st = stats.entry(kind).or_insert_with(|| KindStats {
            kind: kind.to_string(),
            ..Default::default()
        });
        st.items += 1;
        let k = format!("{kind}:{id}");
        let in_careers = seen.iter().filter(|s| s.picks.contains_key(&k)).count();
        let picks: u64 = seen
            .iter()
            .map(|s| s.picks.get(&k).copied().unwrap_or(0))
            .sum();
        if in_careers > 0 {
            st.fired += 1;
        } else {
            st.never += 1;
            let held = *kind == "line" && seen.iter().any(|s| s.held.contains(id));
            let (sev, msg) = if held {
                (
                    Sev::Info,
                    format!("held for some speaker but was never picked in {n} careers"),
                )
            } else {
                (Sev::Warn, format!("never fired in {n} careers"))
            };
            let label = if *kind == "step" {
                format!("step {id}: {msg}")
            } else {
                msg
            };
            out.add(sev, "never-fired", item, label);
            continue;
        }
        let share_careers = in_careers as f64 / n.max(1) as f64;
        let mut often = false;
        if matches!(*kind, "dilemma" | "storyline" | "scene" | "situational")
            && share_careers > args.often_careers
            && !(*kind == "dilemma" && g.is_none())
        {
            often = true;
            out.add(
                Sev::Warn,
                "too-often",
                item,
                format!(
                    "{kind} fired in {in_careers} of {n} careers ({:.0}%)",
                    share_careers * 100.0
                ),
            );
        }
        if let Some(g) = g {
            let total = group_total.get(g).copied().unwrap_or(0);
            let size = group_size.get(g).copied().unwrap_or(1).max(1);
            let share = picks as f64 / total.max(1) as f64;
            if total >= 20 && size >= 3 && share > args.often_share && share > 2.5 / size as f64 {
                often = true;
                out.add(
                    Sev::Warn,
                    "too-often",
                    item,
                    format!(
                        "{:.0}% of the {total} picks among {size} {kind}s in `{g}` (fair share {:.0}%)",
                        share * 100.0,
                        100.0 / size as f64
                    ),
                );
            }
        }
        if often {
            st.too_often += 1;
        }
    }
    // Roles and places no generated town had.
    let kinds: BTreeSet<&String> = seen.iter().flat_map(|s| s.place_kinds.iter()).collect();
    let roles: BTreeSet<&String> = seen.iter().flat_map(|s| s.roles.iter()).collect();
    let rv: BTreeSet<&(String, String)> = seen.iter().flat_map(|s| s.role_voice.iter()).collect();
    let no_town = |out: &mut Out, item: &str, what: String| {
        out.add(
            Sev::Error,
            "no-town-has",
            item,
            format!("{what} in none of the {n} generated towns"),
        );
    };
    let cast_roles = |out: &mut Out, item: &str, c: &Cast| {
        if let Cast::Occupation { occupations } = c
            && !occupations.iter().any(|o| roles.contains(o))
        {
            no_town(out, item, format!("a {} to cast", occupations.join(" or ")));
        }
    };
    for d in &b.dilemmas {
        for r in &d.roles {
            cast_roles(out, &d.id, &r.cast);
        }
    }
    for s in &b.storylines {
        for r in &s.roles {
            cast_roles(out, &s.id, &r.cast);
        }
    }
    for sc in &b.scenes {
        let k = match &sc.place {
            ScenePlace::Kind { kind } => kind,
            ScenePlace::Workplace { fallback, .. } => fallback,
        };
        if !mandate_content::PSEUDO_PLACES.contains(&k.as_str()) && !kinds.contains(k) {
            no_town(out, &sc.id, format!("a `{k}` for the scene"));
        }
    }
    for t in &b.talk_topics {
        let o = &t.offered;
        let places_ok = o.places.is_empty()
            || o.places
                .iter()
                .any(|k| mandate_content::PSEUDO_PLACES.contains(&k.as_str()) || kinds.contains(k));
        if !places_ok {
            no_town(
                out,
                &t.id,
                format!("a place where it is raised ({})", o.places.join(", ")),
            );
        }
        if !o.roles.is_empty() && !o.roles.iter().any(|r| roles.contains(r)) {
            no_town(
                out,
                &t.id,
                format!("someone it is offered to ({})", o.roles.join(", ")),
            );
        }
    }
    for l in &b.talk_lines {
        if let Some(m) = speaker_roles(&l.when)
            && !m.iter().any(|r| roles.contains(r))
        {
            no_town(
                out,
                &l.id,
                format!(
                    "a speaker with its role ({})",
                    m.into_iter().collect::<Vec<_>>().join(", ")
                ),
            );
        }
        if let Some(lead) = &l.lead
            && let LeadToDef::PlaceKind { kind } = &lead.to
            && !kinds.contains(kind)
        {
            no_town(out, &l.id, format!("a `{kind}` for its lead"));
        }
        // A voice's line for roles never heard in that voice.
        if l.part == TalkPart::Closer
            && let Some(v) = &l.voice
        {
            let allowed = speaker_roles(&l.when);
            let any = rv
                .iter()
                .any(|(r, vv)| vv == v && allowed.as_ref().is_none_or(|a| a.contains(r)));
            if !any && n > 0 {
                out.add(
                    Sev::Warn,
                    "voice",
                    &l.id,
                    format!(
                        "no speaker with voice `{v}`{} was met in {n} careers",
                        allowed
                            .map(|a| format!(
                                " and role {}",
                                a.into_iter().collect::<Vec<_>>().join("/")
                            ))
                            .unwrap_or_default()
                    ),
                );
            }
        }
    }
    let order = [
        "dilemma",
        "situational",
        "storyline",
        "step",
        "scene",
        "overheard",
        "topic",
        "line",
        "voice",
    ];
    order.iter().filter_map(|k| stats.get(k).cloned()).collect()
}

// --- Fluent: missing, unused, arguments ---------------------------------------------------

/// Fluent keys content references, with the item that references each.
fn content_keys(b: &ContentBundle) -> BTreeMap<String, String> {
    fn looks(s: &str) -> bool {
        s.contains('-')
            && s.starts_with(|c: char| c.is_ascii_lowercase())
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    }
    fn walk(v: &serde_json::Value, owner: &str, field: &str, out: &mut BTreeMap<String, String>) {
        match v {
            serde_json::Value::Object(m) => {
                let id = m
                    .get("id")
                    .and_then(|x| x.as_str())
                    .filter(|s| s.contains('.'))
                    .unwrap_or(owner)
                    .to_string();
                for (k, x) in m {
                    walk(x, &id, k, out);
                }
            }
            serde_json::Value::Array(xs) => {
                for x in xs {
                    walk(x, owner, field, out);
                }
            }
            serde_json::Value::String(s) => {
                let keyish = field.ends_with("_key")
                    || field.ends_with("_keys")
                    || matches!(field, "key" | "line" | "quote_fallback");
                if keyish && looks(s) {
                    out.entry(s.clone()).or_insert_with(|| owner.to_string());
                }
            }
            _ => {}
        }
    }
    let mut v = serde_json::to_value(b).unwrap_or_default();
    if let Some(m) = v.as_object_mut() {
        m.remove("locales");
        m.remove("content_hash");
        // Presentation registries name pixel-art keys, not Fluent keys.
        for k in ["town_districts", "terrain_features", "connectors"] {
            m.remove(k);
        }
    }
    let mut out = BTreeMap::new();
    walk(&v, "content", "", &mut out);
    out
}

/// A Fluent message: its value text (continuation lines included,
/// attributes left out).
fn messages(ftl: &str) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let mut cur: Option<String> = None;
    for line in ftl.lines() {
        let first = line.chars().next();
        if first.is_some_and(|c| c.is_ascii_alphabetic() || c == '-')
            && let Some((id, rest)) = line.split_once('=')
            && id
                .trim()
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            let id = id.trim().to_string();
            out.insert(id.clone(), rest.trim().to_string());
            cur = Some(id);
            continue;
        }
        if line.starts_with(' ') || line.starts_with('\t') {
            if line.trim_start().starts_with('.') {
                // An attribute: not part of the value.
                cur = None;
                continue;
            }
            if let Some(id) = &cur
                && let Some(v) = out.get_mut(id)
            {
                v.push('\n');
                v.push_str(line.trim());
            }
            continue;
        }
        cur = None;
    }
    out
}

/// Message references (`{ other-key }`, `{ -term }`) in a value.
fn references(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find('{') {
        rest = &rest[i + 1..];
        let t = rest.trim_start();
        let id: String = t
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !id.is_empty() && !t.starts_with('$') {
            out.push(id);
        }
    }
    out
}

/// Quoted identifier-like strings in a source file (`"talk-runs-self"`,
/// `'ui-yes'`), wherever they sit, even inside a template literal; code
/// only (Rust test modules and line comments are skipped). As (string, line).
fn literals(text: &str, rust: bool) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let word = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
    for (n, line) in text.lines().enumerate() {
        if rust && line.trim_start().starts_with("#[cfg(test)]") {
            break;
        }
        if line.trim_start().starts_with("//") {
            continue;
        }
        let cs: Vec<char> = line.chars().collect();
        for i in 0..cs.len() {
            let q = cs[i];
            if !(q == '"' || (!rust && (q == '\'' || q == '`'))) {
                continue;
            }
            let s: String = cs[i + 1..].iter().take_while(|c| word(**c)).collect();
            if !s.is_empty() && cs.get(i + 1 + s.chars().count()) == Some(&q) {
                out.push((s, n + 1));
            }
        }
    }
    out
}

fn source_files(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().and_then(|x| x.to_str()).unwrap_or("");
        if p.is_dir() {
            // Generated types, builds, and the dev tools themselves.
            if !matches!(
                name,
                "node_modules" | "pkg" | "protocol" | "dev" | "fixtures" | "target"
            ) {
                source_files(&p, exts, out);
            }
        } else if exts.iter().any(|x| name.ends_with(x)) && !name.contains(".test.") {
            out.push(p);
        }
    }
}

struct SourceScan {
    /// Literals that are Fluent-key shaped, with where (Rust only).
    engine: Vec<(String, String)>,
    /// Every literal (Rust and TypeScript).
    all: BTreeSet<String>,
    /// Static prefixes of keys built at run time (`fact-{…}`, `` `place-${…}` ``).
    prefixes: BTreeSet<String>,
}

fn scan_sources(root: &Path) -> Option<SourceScan> {
    if root.as_os_str().is_empty() || !root.join("crates").is_dir() {
        return None;
    }
    let mut rs = Vec::new();
    for c in [
        "mandate-sim",
        "mandate-protocol",
        "mandate-content",
        "mandate-worldgen",
        "mandate-wasm",
    ] {
        source_files(&root.join("crates").join(c).join("src"), &[".rs"], &mut rs);
    }
    let mut ts = Vec::new();
    source_files(&root.join("web/src"), &[".ts", ".tsx"], &mut ts);
    let mut scan = SourceScan {
        engine: Vec::new(),
        all: BTreeSet::new(),
        prefixes: BTreeSet::new(),
    };
    let keyish = |s: &str| {
        s.contains('-')
            && s.starts_with(|c: char| c.is_ascii_lowercase())
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
            && !s.ends_with('-')
    };
    for (files, rust) in [(&rs, true), (&ts, false)] {
        for f in files {
            let Ok(text) = std::fs::read_to_string(f) else {
                continue;
            };
            let rel = f
                .strip_prefix(root)
                .unwrap_or(f)
                .to_string_lossy()
                .replace('\\', "/");
            // Keys built at run time, wherever the placeholder sits (nested
            // template literals defeat the literal scan): `fact-{…}` in Rust
            // format strings, `` `lean-${…}` `` in TypeScript.
            let open = if rust { "-{" } else { "-${" };
            let mut from = 0;
            while let Some(i) = text[from..].find(open) {
                let end = from + i + 1;
                let head: String = text[..end]
                    .chars()
                    .rev()
                    .take_while(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '_'
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                if head.len() > 2 && head.starts_with(|c: char| c.is_ascii_lowercase()) {
                    scan.prefixes.insert(head);
                }
                from = end;
            }
            for (lit, line) in literals(&text, rust) {
                // A prefix keys are concatenated onto (`"place-" + kind`).
                if lit.ends_with('-') && lit.len() > 2 {
                    scan.prefixes.insert(lit.clone());
                }
                if keyish(&lit) {
                    if rust {
                        scan.engine.push((lit.clone(), format!("{rel}:{line}")));
                    }
                    scan.all.insert(lit);
                }
            }
        }
    }
    Some(scan)
}

/// The arguments a heard condition gives (an `any`: those every
/// alternative gives).
fn heard_args(c: &HeardCondition) -> Option<BTreeSet<String>> {
    let set = |xs: &[&str]| Some(xs.iter().map(|x| x.to_string()).collect());
    match c {
        HeardCondition::Crisis { .. } => set(&["crisis_key"]),
        HeardCondition::PlantClosure { .. } => set(&["industry_key", "jobs"]),
        HeardCondition::Industry => set(&["industry_key"]),
        HeardCondition::Broker => set(&["broker"]),
        HeardCondition::Rival | HeardCondition::RivalRecord { .. } => set(&["rival"]),
        HeardCondition::PlayerAwareness { .. } | HeardCondition::PlayerApproval { .. } => {
            set(&["player"])
        }
        HeardCondition::Scandal { .. } => set(&["subject", "fact_key"]),
        // A retold story brings its headline's own arguments: not static.
        HeardCondition::Headline => None,
        HeardCondition::Any { of } => {
            let mut it = of.iter().map(heard_args);
            let first = it.next().flatten()?;
            let mut acc = first;
            for x in it {
                acc = acc.intersection(&x?).cloned().collect();
            }
            Some(acc)
        }
        _ => set(&[]),
    }
}

/// `x_key` also provides `x` (the client renders the key as `$x`).
fn with_stems(mut s: BTreeSet<String>) -> BTreeSet<String> {
    let stems: Vec<String> = s
        .iter()
        .filter_map(|a| a.strip_suffix("_key").map(str::to_string))
        .collect();
    s.extend(stems);
    s
}

fn fluent_checks(b: &ContentBundle, sources: &[Source], scan: Option<&SourceScan>, out: &mut Out) {
    let en_text = b.locale_text("en");
    let msgs = messages(&en_text);
    let defined: BTreeSet<String> = fluent_keys(&en_text);
    let refs = content_keys(b);
    // Missing: content references (a normal load refuses them; the doctor
    // stubs them, so look in the real files) and keys the engine names.
    let real: BTreeSet<String> = sources
        .iter()
        .filter(|s| s.path.ends_with(".ftl") && s.path.contains("/en/"))
        .flat_map(|s| fluent_keys(&s.text))
        .collect();
    for (k, owner) in &refs {
        if !real.contains(k)
            && !out
                .findings
                .iter()
                .any(|f| f.check == "fluent-missing" && f.message.contains(&format!("`{k}`")))
        {
            out.add(
                Sev::Error,
                "fluent-missing",
                owner,
                format!("`{k}` is referenced but missing in English"),
            );
        }
    }
    let families: BTreeSet<&str> = defined.iter().filter_map(|k| k.split('-').next()).collect();
    if let Some(scan) = scan {
        let mut reported = BTreeSet::new();
        for (lit, at) in &scan.engine {
            let fam = lit.split('-').next().unwrap_or("");
            if families.contains(fam)
                && !defined.contains(lit)
                && !scan.prefixes.iter().any(|p| p == &format!("{lit}-"))
                // A suffix the engine puts after a prefix (`outcome-talk-` + `favour-offered`).
                && !defined.iter().any(|k| k.ends_with(&format!("-{lit}")))
                && reported.insert(lit.clone())
            {
                out.findings.push(Finding {
                    severity: Sev::Warn,
                    check: "fluent-missing".into(),
                    file: at.clone(),
                    item: lit.clone(),
                    message: format!("the engine names `{lit}`, which is not in English"),
                });
            }
        }
    }
    // Unused: nothing references the key (content, other messages, a
    // literal in the engine or the UI, or a prefix keys are built from).
    let mut referenced: BTreeSet<String> = refs.keys().cloned().collect();
    for v in msgs.values() {
        referenced.extend(references(v));
    }
    if let Some(scan) = scan {
        for k in &defined {
            if referenced.contains(k) {
                continue;
            }
            let used =
                scan.all.contains(k) || scan.prefixes.iter().any(|p| k.starts_with(p.as_str()));
            if !used {
                out.add(
                    Sev::Info,
                    "fluent-unused",
                    k,
                    format!("`{k}` is never referenced (content, messages, engine or UI)"),
                );
            }
        }
    }
    // Arguments a message uses that its caller never supplies.
    let vars = fluent_placeholders(&en_text);
    let check = |out: &mut Out, item: &str, key: &str, provided: &BTreeSet<String>, what: &str| {
        let Some(used) = vars.get(key) else { return };
        for v in used {
            if !provided.contains(v) {
                out.add(
                    Sev::Error,
                    "fluent-args",
                    item,
                    format!("`{key}` uses `${v}`, which {what} never supplies"),
                );
            }
        }
    };
    let base = |roles: &[&str], policy_roles: &[&str]| -> BTreeSet<String> {
        let mut s: BTreeSet<String> = ["town", "player", "crisis_key"]
            .iter()
            .map(|x| x.to_string())
            .collect();
        for r in roles {
            s.insert(r.to_string());
            s.insert(format!("{r}_role_key"));
        }
        for r in policy_roles {
            s.insert(format!("{r}_key"));
        }
        with_stems(s)
    };
    let split = |roles: &[mandate_content::RoleDef]| -> (Vec<String>, Vec<String>) {
        let mut people = Vec::new();
        let mut policies = Vec::new();
        for r in roles {
            match r.cast {
                Cast::Bill { .. } | Cast::Law { .. } => policies.push(r.id.clone()),
                _ => people.push(r.id.clone()),
            }
        }
        (people, policies)
    };
    for d in &b.dilemmas {
        let mut all_roles = d.roles.clone();
        for s in &b.storylines {
            if s.stages
                .iter()
                .any(|x| x.decision.as_deref() == Some(d.id.as_str()))
            {
                all_roles.extend(s.roles.iter().cloned());
            }
        }
        let (p, q) = split(&all_roles);
        let pr: Vec<&str> = p.iter().map(|x| x.as_str()).collect();
        let qr: Vec<&str> = q.iter().map(|x| x.as_str()).collect();
        let mut provided = base(&pr, &qr);
        if d.raised_by.is_some() {
            provided.extend(with_stems(
                ["fact_key", "severity_key", "outlet_key", "outlet_place"]
                    .iter()
                    .map(|x| x.to_string())
                    .collect(),
            ));
        }
        let mut keys = vec![d.title_key.as_str(), d.situation_key.as_str()];
        for o in &d.options {
            keys.push(&o.label_key);
            keys.push(&o.aftermath_key);
        }
        for k in keys {
            check(out, &d.id, k, &provided, "the decision's cast");
        }
        // `$crisis` is there only while the town has a live crisis.
        let crisis_trigger = d
            .triggers
            .iter()
            .any(|c| matches!(c, Condition::Crisis { .. } | Condition::LiveCrisis { .. }));
        if !crisis_trigger
            && vars
                .get(&d.situation_key)
                .is_some_and(|v| v.contains("crisis"))
        {
            out.add(
                Sev::Warn,
                "fluent-args",
                &d.id,
                format!(
                    "`{}` uses `$crisis`, which is there only while the town has a live crisis (no crisis trigger)",
                    d.situation_key
                ),
            );
        }
    }
    for s in &b.storylines {
        let (p, q) = split(&s.roles);
        let pr: Vec<&str> = p.iter().map(|x| x.as_str()).collect();
        let qr: Vec<&str> = q.iter().map(|x| x.as_str()).collect();
        let provided = base(&pr, &qr);
        check(out, &s.id, &s.title_key, &provided, "the storyline's cast");
        check(out, &s.id, &s.faded_key, &provided, "the storyline's cast");
        for st in &s.stages {
            check(out, &s.id, &st.key, &provided, "the storyline's cast");
        }
    }
    for sc in &b.scenes {
        let mut roles = Vec::new();
        for d in b
            .dilemmas
            .iter()
            .filter(|d| d.scene.as_deref() == Some(sc.id.as_str()))
        {
            roles.extend(d.roles.iter().cloned());
        }
        let (p, q) = split(&roles);
        let pr: Vec<&str> = p.iter().map(|x| x.as_str()).collect();
        let qr: Vec<&str> = q.iter().map(|x| x.as_str()).collect();
        let mut provided = base(&pr, &qr);
        provided.extend(["memory", "subject", "year"].iter().map(|x| x.to_string()));
        for beat in &sc.beats {
            check(out, &sc.id, &beat.line, &provided, "the scene's cast");
        }
    }
    for o in &b.overheard {
        let mut provided: BTreeSet<String> =
            ["town", "hood"].iter().map(|x| x.to_string()).collect();
        let mut static_ok = true;
        for c in &o.when {
            match heard_args(c) {
                Some(a) => provided.extend(a),
                None => static_ok = false,
            }
        }
        if static_ok {
            check(out, &o.id, &o.key, &with_stems(provided), "its conditions");
        }
    }
}

// --- Length -------------------------------------------------------------------------

/// A representative value for an argument: long names, a long town.
fn sample_arg(name: &str) -> String {
    let n = name.trim_end_matches("_key");
    match n {
        "n" | "jobs" | "days" | "amount" | "count" | "votes" | "seats" | "weeks" | "months" => {
            "12,345".into()
        }
        "year" => "2031".into(),
        "town" | "hood" | "here" | "outlet_place" | "place" | "broker_place" => {
            "Saint-Aubin-des-Marais".into()
        }
        _ => "Bartholomew Okonkwo-Hale".into(),
    }
}

/// The longest rendering of a message (every selector's longest variant),
/// with representative arguments; `x_key` arguments render a message of
/// typical length.
fn render(msgs: &BTreeMap<String, String>, text: &str, depth: u32) -> String {
    let mut out = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '{' {
            if chars[i] != '\n' {
                out.push(chars[i]);
            } else {
                out.push(' ');
            }
            i += 1;
            continue;
        }
        // The placeable up to its matching brace.
        let mut depth_b = 0;
        let mut j = i;
        while j < chars.len() {
            match chars[j] {
                '{' => depth_b += 1,
                '}' => {
                    depth_b -= 1;
                    if depth_b == 0 {
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        let inner: String = chars[i + 1..j.min(chars.len())].iter().collect();
        out.push_str(&placeable(msgs, inner.trim(), depth));
        i = j + 1;
    }
    out
}

fn placeable(msgs: &BTreeMap<String, String>, inner: &str, depth: u32) -> String {
    if depth > 6 {
        return String::new();
    }
    if let Some(arrow) = inner.find("->") {
        // A selector: the longest variant.
        let variants = &inner[arrow + 2..];
        let mut best = String::new();
        // Split on variant heads `[key]` at nesting level 0.
        let mut parts: Vec<String> = Vec::new();
        let chars: Vec<char> = variants.chars().collect();
        let mut k = 0;
        let mut lvl = 0;
        let mut start: Option<usize> = None;
        while k < chars.len() {
            match chars[k] {
                '{' => lvl += 1,
                '}' => lvl -= 1,
                '[' if lvl == 0 => {
                    if let Some(s) = start {
                        parts.push(chars[s..k].iter().collect());
                    }
                    // Skip the head.
                    while k < chars.len() && chars[k] != ']' {
                        k += 1;
                    }
                    start = Some(k + 1);
                }
                _ => {}
            }
            k += 1;
        }
        if let Some(s) = start {
            parts.push(chars[s.min(chars.len())..].iter().collect());
        }
        for p in parts {
            let p = p.trim().trim_end_matches('*').trim();
            let r = render(msgs, p, depth + 1);
            if r.chars().count() > best.chars().count() {
                best = r;
            }
        }
        return best;
    }
    if let Some(v) = inner.strip_prefix('$') {
        let name: String = v
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        return sample_arg(&name);
    }
    if let Some(s) = inner.strip_prefix('"') {
        return s.trim_end_matches('"').to_string();
    }
    let id: String = inner
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if let Some(m) = msgs.get(&id) {
        return render(msgs, m, depth + 1);
    }
    // A function call (`NUMBER($n)`) or something else: a short number.
    "12,345".into()
}

fn length_checks(b: &ContentBundle, args: &DoctorArgs, out: &mut Out) {
    let msgs = messages(&b.locale_text("en"));
    // `x_key` arguments are other messages: render $x as a typical one.
    let mut checks: Vec<(String, String, usize, &str)> = Vec::new();
    let title = |checks: &mut Vec<_>, item: &str, k: &str| {
        checks.push((item.to_string(), k.to_string(), args.max_title, "title"))
    };
    let label = |checks: &mut Vec<_>, item: &str, k: &str| {
        checks.push((
            item.to_string(),
            k.to_string(),
            args.max_label,
            "option label",
        ))
    };
    let line = |checks: &mut Vec<_>, item: &str, k: &str| {
        checks.push((item.to_string(), k.to_string(), args.max_line, "line"))
    };
    let body = |checks: &mut Vec<_>, item: &str, k: &str| {
        checks.push((item.to_string(), k.to_string(), args.max_body, "body"))
    };
    for d in &b.dilemmas {
        title(&mut checks, &d.id, &d.title_key);
        body(&mut checks, &d.id, &d.situation_key);
        for o in &d.options {
            label(&mut checks, &d.id, &o.label_key);
            body(&mut checks, &d.id, &o.aftermath_key);
        }
    }
    for s in &b.storylines {
        title(&mut checks, &s.id, &s.title_key);
        line(&mut checks, &s.id, &s.faded_key);
        for st in &s.stages {
            line(&mut checks, &s.id, &st.key);
        }
    }
    for sc in &b.scenes {
        for beat in &sc.beats {
            line(&mut checks, &sc.id, &beat.line);
        }
    }
    for o in &b.overheard {
        line(&mut checks, &o.id, &o.key);
    }
    for t in &b.talk_topics {
        title(&mut checks, &t.id, &t.name_key);
    }
    for l in &b.talk_lines {
        line(&mut checks, &l.id, &l.key);
        if let Some(lead) = &l.lead {
            line(&mut checks, &l.id, &lead.key);
        }
    }
    let mut seen = BTreeSet::new();
    for (item, key, max, what) in checks {
        if !seen.insert(key.clone()) {
            continue;
        }
        let Some(m) = msgs.get(&key) else { continue };
        let text = render(&msgs, m, 0);
        let n = text.chars().count();
        if n > max {
            out.add(
                Sev::Warn,
                "too-long",
                &item,
                format!(
                    "{what} `{key}` renders to {n} characters (limit {max}): \"{}…\"",
                    text.chars().take(60).collect::<String>()
                ),
            );
        }
    }
}

// --- Report ----------------------------------------------------------------------------

/// Run every check on a set of sources.
pub fn diagnose(sources: &[Source], args: &DoctorArgs) -> Report {
    let mut out = Out {
        findings: Vec::new(),
        files: origins(sources),
    };
    let mut report = Report {
        seeds: args.seeds,
        years: args.years,
        strategy: args.strategy.clone(),
        ..Default::default()
    };
    if let Some(b) = load(sources, &mut out) {
        report.content_hash = b.content_hash.clone();
        static_checks(&b, &mut out);
        let scan = scan_sources(&args.source);
        fluent_checks(&b, sources, scan.as_ref(), &mut out);
        length_checks(&b, args, &mut out);
        if args.seeds > 0 {
            let seen = careers(&b, args);
            report.careers = seen.len();
            report.coverage = dynamic_checks(&b, &seen, args, &mut out);
        }
    }
    // The stubs file is the doctor's own.
    out.findings.retain(|f| f.file != STUBS);
    out.findings.sort_by(|a, b| {
        (
            a.file.as_str(),
            a.severity,
            a.item.as_str(),
            a.check.as_str(),
        )
            .cmp(&(
                b.file.as_str(),
                b.severity,
                b.item.as_str(),
                b.check.as_str(),
            ))
    });
    out.findings.dedup();
    let mut summary: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    for f in &out.findings {
        let e = summary.entry(f.check.clone()).or_default();
        match f.severity {
            Sev::Error => e.0 += 1,
            Sev::Warn => e.1 += 1,
            Sev::Info => e.2 += 1,
        }
    }
    report.summary = summary
        .into_iter()
        .map(|(k, (e, w, i))| (k, e, w, i))
        .collect();
    report.findings = out.findings;
    report
}

/// The report as text for a terminal.
pub fn human(r: &Report) -> String {
    let mut s = String::new();
    let hash: String = r.content_hash.chars().take(12).collect();
    s.push_str(&format!(
        "Content doctor: {} careers × {} years (strategy {}), content {hash}\n\n",
        r.careers, r.years, r.strategy
    ));
    s.push_str("Summary\n");
    s.push_str(&format!(
        "  {:<16} {:>6} {:>6} {:>6}\n",
        "check", "errors", "warns", "infos"
    ));
    let (mut te, mut tw, mut ti) = (0, 0, 0);
    for (k, e, w, i) in &r.summary {
        s.push_str(&format!("  {k:<16} {e:>6} {w:>6} {i:>6}\n"));
        te += e;
        tw += w;
        ti += i;
    }
    s.push_str(&format!("  {:<16} {te:>6} {tw:>6} {ti:>6}\n", "total"));
    if !r.coverage.is_empty() {
        s.push_str(&format!(
            "\nCoverage in {} careers\n  {:<12} {:>6} {:>6} {:>6} {:>9}\n",
            r.careers, "kind", "items", "fired", "never", "too often"
        ));
        for k in &r.coverage {
            s.push_str(&format!(
                "  {:<12} {:>6} {:>6} {:>6} {:>9}\n",
                k.kind, k.items, k.fired, k.never, k.too_often
            ));
        }
    }
    let mut file = "";
    let mut infos = 0usize;
    const INFOS_PER_FILE: usize = 12;
    for f in &r.findings {
        if f.file != file {
            if infos > INFOS_PER_FILE {
                s.push_str(&format!(
                    "  … {} more infos (--json for all)\n",
                    infos - INFOS_PER_FILE
                ));
            }
            file = &f.file;
            infos = 0;
            s.push_str(&format!("\n{file}\n"));
        }
        if f.severity == Sev::Info {
            infos += 1;
            if infos > INFOS_PER_FILE {
                continue;
            }
        }
        s.push_str(&format!(
            "  {:<5} {:<14} {}: {}\n",
            f.severity.key(),
            f.check,
            f.item,
            f.message
        ));
    }
    if infos > INFOS_PER_FILE {
        s.push_str(&format!(
            "  … {} more infos (--json for all)\n",
            infos - INFOS_PER_FILE
        ));
    }
    s
}

/// `content doctor`: print the report; `Err` under `--strict` with errors.
pub fn run(root: &Path, args: &DoctorArgs) -> Result<(), String> {
    let sources = mandate_content::bundle::read_dir_sources(root)
        .map_err(|e| format!("reading {}: {e}", root.display()))?;
    let report = diagnose(&sources, args);
    {
        use std::io::Write;
        let mut o = std::io::stdout().lock();
        let text = if args.json {
            serde_json::to_string_pretty(&report).unwrap_or_default()
        } else {
            human(&report)
        };
        let _ = writeln!(o, "{text}");
    }
    if args.strict && report.errors() > 0 {
        return Err(format!("content doctor: {} error(s)", report.errors()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn base_sources() -> Vec<Source> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        mandate_content::bundle::read_dir_sources(&root).unwrap()
    }

    /// The base pack plus a planted dilemma that can never fire (its
    /// triggers contradict each other), an option whose aftermath key is
    /// missing from English, and an overheard line far too long for a phone.
    fn planted() -> Vec<Source> {
        let mut s = base_sources();
        s.push(Source {
            path: "base/planted_dilemmas.jsonc".into(),
            text: r#"{
  "kind": "dilemmas",
  "items": [
    {
      "id": "dilemma.planted_never", "category": "significant",
      "title_key": "dilemma-planted-never", "situation_key": "dilemma-planted-never-situation",
      "roles": [ { "id": "rival", "cast": { "type": "rival" } } ],
      "triggers": [
        { "type": "track", "track": "standing", "min": 80 },
        { "type": "track", "track": "standing", "max": 20 }
      ],
      "cooldown_days": 30,
      "options": [
        { "id": "a", "label_key": "dilemma-planted-never-a", "aftermath_key": "dilemma-planted-missing-after", "default": true,
          "effects": [ { "type": "standing", "delta": 2 }, { "type": "health", "delta": -2 } ] },
        { "id": "b", "label_key": "dilemma-planted-never-b", "aftermath_key": "dilemma-planted-never-b-after",
          "effects": [ { "type": "standing", "delta": -2 }, { "type": "health", "delta": 2 } ] }
      ]
    }
  ]
}"#
            .into(),
        });
        s.push(Source {
            path: "base/planted_overheard.jsonc".into(),
            text: r#"{ "kind": "overheard", "items": [ { "id": "overheard.planted_long", "key": "overheard-planted-long" } ] }"#.into(),
        });
        let long = "On and on it goes about the { $town } bypass, ".repeat(9);
        s.push(Source {
            path: "base/locale/en/planted.ftl".into(),
            text: format!(
                "dilemma-planted-never = A planted dilemma\n\
                 dilemma-planted-never-situation = {{ $rival }} waits.\n\
                 dilemma-planted-never-a = Wait\n\
                 dilemma-planted-never-b = Act\n\
                 dilemma-planted-never-b-after = You acted.\n\
                 overheard-planted-long = {long}\n"
            ),
        });
        s
    }

    fn quick() -> DoctorArgs {
        DoctorArgs {
            seeds: 0,
            source: PathBuf::new(),
            ..Default::default()
        }
    }

    #[test]
    fn the_doctor_finds_planted_problems() {
        let r = diagnose(&planted(), &quick());
        let show = || human(&r);
        assert!(
            r.has("contradiction", "dilemma.planted_never"),
            "{}",
            show()
        );
        let missing = r.findings.iter().find(|f| {
            f.check == "fluent-missing" && f.message.contains("dilemma-planted-missing-after")
        });
        assert!(missing.is_some(), "{}", show());
        assert_eq!(missing.unwrap().severity, Sev::Error);
        let long = r
            .findings
            .iter()
            .find(|f| f.check == "too-long" && f.item == "overheard.planted_long");
        assert!(long.is_some(), "{}", show());
        // Findings are filed where the item is defined.
        assert_eq!(long.unwrap().file, "base/planted_overheard.jsonc");
        assert!(r.errors() >= 2);
    }

    #[test]
    fn the_base_pack_loads_and_its_static_checks_run() {
        let r = diagnose(&base_sources(), &quick());
        assert!(!r.content_hash.is_empty(), "{}", human(&r));
        assert!(
            !r.findings
                .iter()
                .any(|f| f.check == "validate" && f.severity == Sev::Error)
        );
    }

    #[test]
    fn a_short_career_run_reports_coverage() {
        let args = DoctorArgs {
            seeds: 1,
            years: 1,
            source: PathBuf::new(),
            ..Default::default()
        };
        let r = diagnose(&base_sources(), &args);
        assert_eq!(r.careers, 1);
        let d = r.coverage.iter().find(|k| k.kind == "dilemma").unwrap();
        assert_eq!(d.items, d.fired + d.never);
        assert!(
            r.coverage
                .iter()
                .any(|k| k.kind == "overheard" && k.fired > 0)
        );
    }

    #[test]
    fn rendering_takes_the_longest_variant_and_references() {
        let msgs = messages(
            "a = { $n ->\n    [one] one\n   *[other] { $n } many many things\n}\nb = See { a } in { $town }\n",
        );
        let r = render(&msgs, msgs.get("b").unwrap(), 0);
        assert!(r.contains("many many things"), "{r}");
        assert!(r.contains("Saint-Aubin-des-Marais"), "{r}");
    }

    #[test]
    fn contradictions_and_impossible_bounds_are_found() {
        let mut out = Out {
            findings: Vec::new(),
            files: BTreeMap::new(),
        };
        check_narrative(
            &mut out,
            "x",
            "triggers",
            &[
                Condition::Opinion {
                    role: "r".into(),
                    min: Some(120.0),
                    max: None,
                },
                Condition::Office {
                    role: OfficeRole::Mayor,
                },
                Condition::Office {
                    role: OfficeRole::None,
                },
            ],
        );
        assert!(out.findings.iter().any(|f| f.check == "unsatisfiable"));
        assert!(out.findings.iter().any(|f| f.check == "contradiction"));
    }
}
