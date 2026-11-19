//! Media, scandals, and the legal path (DEC-034, DEC-006): the outlet
//! registry, the media parameter block (discovery, coverage, perception,
//! responses, rumours, shady actions, NPC temptation), and the justice
//! parameter block (investigations, charges, trials, sentences).
//!
//! Every number the engine tunes lives here (DEC-078 decision 1); the
//! mechanisms (who discovers, how a story is chosen and framed, how a case
//! moves) are code in `mandate_sim::media` and `mandate_sim::justice`.

use crate::ContentBundle;
use crate::types::ContentKey;
use crate::validate::Issue;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// How serious a hidden fact is (DEC-006 Legal Jeopardy): a lapse of
/// ethics, misconduct in office, or a crime.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Gravity {
    #[default]
    Ethics,
    Misconduct,
    Crime,
}

impl Gravity {
    pub const ALL: [Gravity; 3] = [Gravity::Ethics, Gravity::Misconduct, Gravity::Crime];

    pub fn key(self) -> &'static str {
        match self {
            Gravity::Ethics => "ethics",
            Gravity::Misconduct => "misconduct",
            Gravity::Crime => "crime",
        }
    }
}

/// A number per severity.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ByGravity {
    pub ethics: f64,
    pub misconduct: f64,
    pub crime: f64,
}

impl ByGravity {
    pub fn get(&self, s: Gravity) -> f64 {
        match s {
            Gravity::Ethics => self.ethics,
            Gravity::Misconduct => self.misconduct,
            Gravity::Crime => self.crime,
        }
    }
}

/// Who may uncover a hidden fact.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Discoverer {
    /// A reporter at an outlet in the subject's town (Named journalists and editors).
    Journalist,
    /// A political opponent: the subject's rival.
    Rival,
    /// The town's auditors (money trails).
    Auditor,
    /// Police and prosecutors' investigators.
    Investigator,
}

impl Discoverer {
    pub const ALL: [Discoverer; 4] = [
        Discoverer::Journalist,
        Discoverer::Rival,
        Discoverer::Auditor,
        Discoverer::Investigator,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Discoverer::Journalist => "journalist",
            Discoverer::Rival => "rival",
            Discoverer::Auditor => "auditor",
            Discoverer::Investigator => "investigator",
        }
    }
}

/// A borderline or illegal act: what it is, how serious, how much evidence
/// it leaves, and who might uncover it (DEC-034, DEC-073: the risk is shown
/// before the player commits).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HiddenSpec {
    /// Fact kind key (Fluent `fact-<kind>`), e.g. `consultancy_fee`.
    pub kind: String,
    pub severity: Gravity,
    /// How much evidence it leaves (0..1): a paper trail is high, a quiet word low.
    pub evidence: f64,
    /// Who could uncover it (empty = every discoverer).
    #[serde(default)]
    pub discoverers: Vec<Discoverer>,
}

/// Where an outlet publishes.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum OutletScope {
    /// One per town with its home place (`place`).
    Town,
    /// One per region.
    Region,
    /// One for the nation.
    Nation,
}

/// Who follows an outlet: relative weights by income class and age band.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AudienceDef {
    /// [low, middle, high].
    pub class: [f64; 3],
    /// [18–29, 30–44, 45–64, 65+].
    pub age: [f64; 4],
}

/// An outlet template (DEC-034): generated per town, region, or nation when
/// play begins.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutletDef {
    pub id: ContentKey,
    /// Fluent key of the outlet's name (`$outlet_place`: its home place's
    /// name for a town outlet, its region's name for a regional one).
    pub name_key: String,
    pub scope: OutletScope,
    /// A town outlet's home place kind (DEC-079): towns without one have none.
    #[serde(default)]
    pub place: Option<String>,
    /// A social platform: high reach, low credibility, spreads rumours.
    #[serde(default)]
    pub social: bool,
    /// Share of the scope's adults it reaches (an audience weight of 1).
    pub reach: f64,
    pub audience: AudienceDef,
    /// Credibility (0..1), drawn in [min, max].
    pub credibility: [f64; 2],
    /// Spread of its editorial lean around its audience's views (per issue).
    pub lean_sd: f64,
    /// Concerns it cares about (extra interest in stories that touch them).
    #[serde(default)]
    pub interests: Vec<ContentKey>,
    /// Yearly budget (currency), drawn in [min, max]: capacity and digging.
    pub finances: [f64; 2],
    /// Stories it runs a week at a typical budget.
    pub stories_per_week: u8,
    /// Occupations of its journalists (Named people at its home place).
    #[serde(default)]
    pub journalists: Vec<ContentKey>,
}

/// Discovery of hidden facts (DEC-034): whether a fact will come out is
/// settled when it happens (evidence × interest × access), never by the
/// director; when, within its evidence window, is the director's (DEC-033 B).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryParams {
    /// Discoverers' interest by severity.
    pub interest: ByGravity,
    /// Each discoverer's access (0..1) when present.
    pub access: BTreeMap<String, f64>,
    /// A town's journalists add access up to this many (a town without a paper has little).
    pub journalists_full: f64,
    /// Journalist access without any journalist in town (the regional press).
    pub journalist_floor: f64,
    /// Evidence window (days after the act) at evidence 1 and evidence 0.
    pub window_strong: [u32; 2],
    pub window_weak: [u32; 2],
    /// The risk line's horizon (DEC-073: "within a year").
    pub horizon_days: u32,
    /// Risk bands: low below [0], real below [1], high from [1].
    pub bands: [f64; 2],
}

/// Story selection, scandal heat, and perception (DEC-034).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CoverageParams {
    /// Newsworthiness by severity (a scandal at full heat).
    pub newsworthiness: ByGravity,
    /// Heat kept each week (a scandal fades).
    pub heat_keep: f64,
    /// A scandal ends below this heat.
    pub heat_end: f64,
    /// Stories below this score are not run.
    pub min_score: f64,
    /// Extra interest per unit of disagreement between an outlet's lean and the subject.
    pub hostility_interest: f64,
    /// Interest of a regional (nation) outlet in a town affair, × severity weight.
    pub regional_access: f64,
    pub national_access: f64,
    /// Budget at which an outlet runs its `stories_per_week`.
    pub typical_budget: f64,
    /// Neutral town news (closures, disasters, laws) the local paper runs.
    pub town_news: bool,
    /// Stories kept (for the newspaper and the audit).
    pub stories_kept: u32,
}

/// What coverage does to beliefs (DEC-034 perception, DEC-025: perceived
/// personal impact as shaped by media).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PerceptionParams {
    /// Weight of each severity.
    pub severity_weight: ByGravity,
    /// Approval of the subject lost per story × weight × heat × exposure × credibility.
    pub approval: f64,
    /// Name recognition per story × exposure (a scandal makes you known).
    pub awareness: f64,
    /// Hostile framing scales the hit by (1 + this × hostility).
    pub framing: f64,
    /// A politician's scandal valence (0..1) per story × weight × heat × exposure × credibility.
    pub valence: f64,
    /// The subject's Standing (party) per breaking scandal × weight.
    pub standing: f64,
    /// Approval per covered statement × framing × exposure × credibility.
    pub statement: f64,
    /// A legal development's story: × this (charges, a conviction), and an acquittal gives back this.
    pub legal_story: f64,
    pub acquittal: f64,
}

/// Responses to a story (the player's and NPCs').
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResponseParams {
    /// Heat kept after a denial when the evidence is weak (below `deny_strong`).
    pub deny_heat: f64,
    /// Evidence at which a denial unravels: heat × `deny_backfire_heat` and Standing.
    pub deny_strong: f64,
    pub deny_backfire_heat: f64,
    pub deny_backfire_standing: f64,
    pub explain_heat: f64,
    pub apologise_heat: f64,
    /// An apology is an admission: prosecutors' evidence rises by this.
    pub apologise_evidence: f64,
    pub counter_heat: f64,
    /// A counter-attack turns the outlet that broke it more hostile.
    pub counter_hostility: f64,
    /// A covered statement during a scandal keeps this much heat (× sympathy).
    pub statement_heat: f64,
    /// An ad buy during a scandal keeps this much heat (changing the subject).
    pub ad_heat: f64,
}

/// Unverified claims (DEC-034 rumours, kept small).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RumourParams {
    /// Weeks a rumour spreads on social platforms.
    pub spread_weeks: u8,
    /// Weekly chance a credible outlet corrects it (× its credibility).
    pub correction_chance: f64,
    /// Share of the harm a correction undoes.
    pub correction_share: f64,
    /// A rumour's weight beside a real scandal of the same severity.
    pub weight: f64,
}

/// A shady action at a place (DEC-079): its payoff and the hidden fact it leaves.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ShadyActionDef {
    pub hidden: HiddenSpec,
    /// Campaign funds gained (negative: spent).
    #[serde(default)]
    pub money: f64,
    /// Health gained.
    #[serde(default)]
    pub health: f64,
    /// Legal exposure added at once.
    #[serde(default)]
    pub legal: f64,
}

/// NPCs' temptations (DEC-030: the same acts and the same exposure).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TemptationParams {
    /// Chance per turn for an officeholder of integrity 0 (× ambition factor).
    pub chance: f64,
    /// Integrity's weight: chance × (1 − integrity)^this.
    pub integrity_power: f64,
    /// Chance × (base + ambition).
    pub ambition_base: f64,
    /// What they take: the player's diner bribe, or a donor deal.
    pub acts: Vec<ContentKey>,
}

/// The media parameter block (one file across all packs).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MediaParams {
    pub discovery: DiscoveryParams,
    pub coverage: CoverageParams,
    pub perception: PerceptionParams,
    pub responses: ResponseParams,
    pub rumours: RumourParams,
    /// Shady actions by action key (`take_bribe`, `dip_funds`, `leak_file`).
    pub shady_actions: BTreeMap<String, ShadyActionDef>,
    /// NPC acts by id (a `shady_actions` key, or `donor_deal`), with their payoff in funds.
    pub npc_acts: BTreeMap<String, ShadyActionDef>,
    pub temptation: TemptationParams,
    /// Late-night deals and fundraising (M5c's hidden facts).
    pub late_night_deal: HiddenSpec,
    pub late_night_fundraising: HiddenSpec,
}

/// Responses to an investigation (the player's decision, NPCs by personality).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LegalResponseParams {
    /// Cooperating: the charge chance × this, evidence + this, sentence × this.
    pub cooperate_charge: f64,
    pub cooperate_evidence: f64,
    pub cooperate_sentence: f64,
    /// A lawyer: the charge chance − this, the conviction logit − this.
    pub lawyer_charge: f64,
    pub lawyer_convict: f64,
    /// Stonewalling: at evidence ≥ `stonewall_strong` the charge chance + this
    /// (obstruction), below it − `stonewall_relief`.
    pub stonewall_strong: f64,
    pub stonewall_charge: f64,
    pub stonewall_relief: f64,
    /// Blaming an ally: the charge chance × this; the ally's scandal valence + this.
    pub blame_charge: f64,
    pub blame_valence: f64,
}

/// The legal path (DEC-006 Legal Jeopardy): investigation, charges, trial, sentence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JusticeParams {
    /// Prosecutors' interest: chance an investigation opens, × evidence.
    pub open: ByGravity,
    /// Extra when auditors or investigators found it.
    pub official_bonus: f64,
    /// Days from discovery to the decision to investigate, [min, max].
    pub open_days: [u32; 2],
    /// Days an investigation lasts, [min, max].
    pub investigation_days: [u32; 2],
    /// Chance of charges at the end, × (evidence + `charge_evidence_base`).
    pub charge: ByGravity,
    pub charge_evidence_base: f64,
    /// Days from charges to trial, [min, max] (at least the fairness response time).
    pub trial_days: [u32; 2],
    /// Conviction: logistic(base + evidence × evidence − lawyer + mood × (0.5 − approval)).
    pub convict_base: f64,
    pub convict_evidence: f64,
    pub convict_mood: f64,
    /// Sentences: a fine (currency, by severity), removal from office
    /// (chance for misconduct; always for prison), prison years for crimes.
    pub fine: ByGravity,
    pub removal_misconduct: f64,
    pub prison_years: [u8; 2],
    /// A sentence this long (years) or a disqualification ends a career.
    pub long_years: u8,
    /// Chance a crime conviction disqualifies from office.
    pub disqualify: f64,
    pub responses: LegalResponseParams,
    /// Legal Jeopardy floors by stage (the track shows the case).
    pub track_investigation: f64,
    pub track_charged: f64,
    /// Legal Jeopardy falls this much a month with no open case.
    pub track_decay: f64,
    /// Out of prison: Standing and viability set to at most these.
    pub release_standing: f64,
    pub release_viability: f64,
    /// Standing and viability lost to a fine or a removal.
    pub fine_standing: f64,
    pub removal_viability: f64,
}

fn in_range(v: f64, lo: f64, hi: f64) -> bool {
    v.is_finite() && (lo..=hi).contains(&v)
}

/// Range checks for the media and justice blocks (DEC-078 decision 1).
pub fn param_ranges(b: &ContentBundle, r: &mut dyn FnMut(&str, f64, f64, f64)) {
    let m = &b.media;
    let d = &m.discovery;
    for s in Gravity::ALL {
        r(
            &format!("media.discovery.interest.{}", s.key()),
            d.interest.get(s),
            0.0,
            1.0,
        );
    }
    for (k, v) in &d.access {
        r(&format!("media.discovery.access.{k}"), *v, 0.0, 1.0);
    }
    r(
        "media.discovery.journalists_full",
        d.journalists_full,
        0.1,
        20.0,
    );
    r(
        "media.discovery.journalist_floor",
        d.journalist_floor,
        0.0,
        1.0,
    );
    for (k, w) in [
        ("window_strong", d.window_strong),
        ("window_weak", d.window_weak),
    ] {
        r(&format!("media.discovery.{k}[0]"), w[0].into(), 1.0, 3650.0);
        r(
            &format!("media.discovery.{k}[1]"),
            w[1].into(),
            w[0].into(),
            3650.0,
        );
    }
    r(
        "media.discovery.horizon_days",
        d.horizon_days.into(),
        30.0,
        3650.0,
    );
    r("media.discovery.bands[0]", d.bands[0], 0.0, 1.0);
    r("media.discovery.bands[1]", d.bands[1], d.bands[0], 1.0);
    let c = &m.coverage;
    for s in Gravity::ALL {
        r(
            &format!("media.coverage.newsworthiness.{}", s.key()),
            c.newsworthiness.get(s),
            0.0,
            10.0,
        );
    }
    r("media.coverage.heat_keep", c.heat_keep, 0.0, 0.99);
    r("media.coverage.heat_end", c.heat_end, 0.0, 1.0);
    r("media.coverage.min_score", c.min_score, 0.0, 10.0);
    r(
        "media.coverage.hostility_interest",
        c.hostility_interest,
        0.0,
        10.0,
    );
    r(
        "media.coverage.regional_access",
        c.regional_access,
        0.0,
        1.0,
    );
    r(
        "media.coverage.national_access",
        c.national_access,
        0.0,
        1.0,
    );
    r("media.coverage.typical_budget", c.typical_budget, 1.0, 1e9);
    r(
        "media.coverage.stories_kept",
        c.stories_kept.into(),
        10.0,
        5000.0,
    );
    let p = &m.perception;
    for s in Gravity::ALL {
        r(
            &format!("media.perception.severity_weight.{}", s.key()),
            p.severity_weight.get(s),
            0.0,
            10.0,
        );
    }
    for (k, v, hi) in [
        ("approval", p.approval, 1.0),
        ("awareness", p.awareness, 1.0),
        ("framing", p.framing, 5.0),
        ("valence", p.valence, 1.0),
        ("standing", p.standing, 50.0),
        ("statement", p.statement, 1.0),
        ("legal_story", p.legal_story, 10.0),
        ("acquittal", p.acquittal, 10.0),
    ] {
        r(&format!("media.perception.{k}"), v, 0.0, hi);
    }
    let x = &m.responses;
    for (k, v) in [
        ("deny_heat", x.deny_heat),
        ("deny_strong", x.deny_strong),
        ("deny_backfire_heat", x.deny_backfire_heat / 3.0),
        ("explain_heat", x.explain_heat),
        ("apologise_heat", x.apologise_heat),
        ("apologise_evidence", x.apologise_evidence),
        ("counter_heat", x.counter_heat),
        ("counter_hostility", x.counter_hostility),
        ("statement_heat", x.statement_heat),
        ("ad_heat", x.ad_heat),
    ] {
        r(&format!("media.responses.{k}"), v, 0.0, 1.0);
    }
    r(
        "media.responses.deny_backfire_standing",
        x.deny_backfire_standing,
        0.0,
        50.0,
    );
    let u = &m.rumours;
    r(
        "media.rumours.spread_weeks",
        u.spread_weeks.into(),
        1.0,
        52.0,
    );
    r(
        "media.rumours.correction_chance",
        u.correction_chance,
        0.0,
        1.0,
    );
    r(
        "media.rumours.correction_share",
        u.correction_share,
        0.0,
        1.0,
    );
    r("media.rumours.weight", u.weight, 0.0, 2.0);
    let t = &m.temptation;
    r("media.temptation.chance", t.chance, 0.0, 1.0);
    r(
        "media.temptation.integrity_power",
        t.integrity_power,
        0.0,
        10.0,
    );
    r("media.temptation.ambition_base", t.ambition_base, 0.0, 2.0);
    let j = &b.justice;
    for s in Gravity::ALL {
        r(
            &format!("justice.open.{}", s.key()),
            j.open.get(s),
            0.0,
            1.0,
        );
        r(
            &format!("justice.charge.{}", s.key()),
            j.charge.get(s),
            0.0,
            1.0,
        );
        r(
            &format!("justice.fine.{}", s.key()),
            j.fine.get(s),
            0.0,
            1e7,
        );
    }
    r("justice.official_bonus", j.official_bonus, 0.0, 1.0);
    for (k, w) in [
        ("open_days", j.open_days),
        ("investigation_days", j.investigation_days),
        ("trial_days", j.trial_days),
    ] {
        r(&format!("justice.{k}[0]"), w[0].into(), 1.0, 3650.0);
        r(&format!("justice.{k}[1]"), w[1].into(), w[0].into(), 3650.0);
    }
    r(
        "justice.trial_days[0]",
        j.trial_days[0].into(),
        b.director.fairness.response_days.into(),
        3650.0,
    );
    r(
        "justice.charge_evidence_base",
        j.charge_evidence_base,
        0.0,
        1.0,
    );
    r("justice.convict_base", j.convict_base, -10.0, 10.0);
    r("justice.convict_evidence", j.convict_evidence, 0.0, 20.0);
    r("justice.convict_mood", j.convict_mood, 0.0, 20.0);
    r("justice.removal_misconduct", j.removal_misconduct, 0.0, 1.0);
    r(
        "justice.prison_years[0]",
        j.prison_years[0].into(),
        1.0,
        40.0,
    );
    r(
        "justice.prison_years[1]",
        j.prison_years[1].into(),
        j.prison_years[0].into(),
        40.0,
    );
    r("justice.long_years", j.long_years.into(), 1.0, 40.0);
    r("justice.disqualify", j.disqualify, 0.0, 1.0);
    let lr = &j.responses;
    for (k, v, lo, hi) in [
        ("cooperate_charge", lr.cooperate_charge, 0.0, 2.0),
        ("cooperate_evidence", lr.cooperate_evidence, 0.0, 1.0),
        ("cooperate_sentence", lr.cooperate_sentence, 0.0, 2.0),
        ("lawyer_charge", lr.lawyer_charge, 0.0, 1.0),
        ("lawyer_convict", lr.lawyer_convict, 0.0, 10.0),
        ("stonewall_strong", lr.stonewall_strong, 0.0, 1.0),
        ("stonewall_charge", lr.stonewall_charge, 0.0, 1.0),
        ("stonewall_relief", lr.stonewall_relief, 0.0, 1.0),
        ("blame_charge", lr.blame_charge, 0.0, 2.0),
        ("blame_valence", lr.blame_valence, 0.0, 1.0),
    ] {
        r(&format!("justice.responses.{k}"), v, lo, hi);
    }
    for (k, v) in [
        ("track_investigation", j.track_investigation),
        ("track_charged", j.track_charged),
        ("track_decay", j.track_decay),
        ("release_standing", j.release_standing),
        ("release_viability", j.release_viability),
        ("fine_standing", j.fine_standing),
        ("removal_viability", j.removal_viability),
    ] {
        r(&format!("justice.{k}"), v, 0.0, 100.0);
    }
}

fn check_spec(owner: &str, s: &HiddenSpec, issues: &mut Vec<Issue>) {
    if !in_range(s.evidence, 0.0, 1.0) {
        issues.push(Issue::error(owner, "hidden evidence must be in [0, 1]"));
    }
    if s.kind.is_empty() || !s.kind.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
        issues.push(Issue::error(
            owner,
            "a hidden fact kind is lower-case letters and `_`",
        ));
    }
}

/// The hidden-fact kinds content names (each needs `fact-<kind>` in English).
pub fn hidden_kinds(b: &ContentBundle) -> Vec<String> {
    let m = &b.media;
    let mut v: Vec<String> = m
        .shady_actions
        .values()
        .chain(m.npc_acts.values())
        .map(|a| a.hidden.kind.clone())
        .chain([
            m.late_night_deal.kind.clone(),
            m.late_night_fundraising.kind.clone(),
        ])
        .collect();
    for d in &b.dilemmas {
        for o in &d.options {
            for e in &o.effects {
                if let crate::narrative::EffectDef::Hidden { kind, .. } = e {
                    v.push(kind.clone());
                }
            }
        }
    }
    v.sort();
    v.dedup();
    v
}

/// The outlet registry and the media and justice blocks.
pub(crate) fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    for o in &b.outlets {
        let owner = o.id.as_str();
        if !crate::validate::valid_key(owner) {
            issues.push(Issue::error(owner, "outlet ids look like `outlet.name`"));
        }
        match (&o.scope, &o.place) {
            (OutletScope::Town, None) => issues.push(Issue::error(
                owner,
                "a town outlet needs its home `place` kind (DEC-079)",
            )),
            (OutletScope::Town, Some(p)) => {
                if b.place_kind(p).is_none() || crate::types::PSEUDO_PLACES.contains(&p.as_str()) {
                    issues.push(Issue::error(owner, &format!("unknown place kind `{p}`")));
                }
            }
            (_, Some(_)) => issues.push(Issue::error(owner, "only town outlets have a home place")),
            _ => {}
        }
        if !in_range(o.reach, 0.0, 1.0) || !in_range(o.lean_sd, 0.0, 2.0) {
            issues.push(Issue::error(
                owner,
                "reach must be in [0, 1] and lean_sd in [0, 2]",
            ));
        }
        if !in_range(o.credibility[0], 0.0, 1.0)
            || !in_range(o.credibility[1], o.credibility[0], 1.0)
        {
            issues.push(Issue::error(owner, "credibility must be 0 ≤ min ≤ max ≤ 1"));
        }
        if !in_range(o.finances[0], 0.0, 1e9) || !in_range(o.finances[1], o.finances[0], 1e9) {
            issues.push(Issue::error(owner, "finances must be 0 ≤ min ≤ max"));
        }
        if o.audience
            .class
            .iter()
            .chain(o.audience.age.iter())
            .any(|w| !in_range(*w, 0.0, 10.0))
        {
            issues.push(Issue::error(owner, "audience weights must be in [0, 10]"));
        }
        if o.stories_per_week == 0 || o.stories_per_week > 20 {
            issues.push(Issue::error(owner, "stories_per_week must be 1–20"));
        }
        for c in &o.interests {
            if b.concern_index(c).is_none() {
                issues.push(Issue::warning(owner, &format!("unknown concern `{c}`")));
            }
        }
        for j in &o.journalists {
            if b.occupation(j).is_none() {
                issues.push(Issue::error(owner, &format!("unknown occupation `{j}`")));
            }
        }
    }
    let m = &b.media;
    for k in m.discovery.access.keys() {
        if !Discoverer::ALL.iter().any(|d| d.key() == k) {
            issues.push(Issue::error("media", &format!("unknown discoverer `{k}`")));
        }
    }
    for d in Discoverer::ALL {
        if !m.discovery.access.contains_key(d.key()) {
            issues.push(Issue::error(
                "media",
                &format!("discovery.access needs `{}`", d.key()),
            ));
        }
    }
    for (k, a) in &m.shady_actions {
        if !crate::types::SHADY_ACTIONS.contains(&k.as_str()) {
            issues.push(Issue::error(
                "media",
                &format!("unknown shady action `{k}`"),
            ));
        }
        check_spec(&format!("media.shady_actions.{k}"), &a.hidden, issues);
    }
    for k in crate::types::SHADY_ACTIONS {
        if !m.shady_actions.contains_key(k) {
            issues.push(Issue::error("media", &format!("shady_actions needs `{k}`")));
        }
    }
    for (k, a) in &m.npc_acts {
        check_spec(&format!("media.npc_acts.{k}"), &a.hidden, issues);
    }
    for a in &m.temptation.acts {
        if !m.npc_acts.contains_key(a) {
            issues.push(Issue::error(
                "media",
                &format!("temptation act `{a}` is not in npc_acts"),
            ));
        }
    }
    check_spec("media.late_night_deal", &m.late_night_deal, issues);
    check_spec(
        "media.late_night_fundraising",
        &m.late_night_fundraising,
        issues,
    );
    // Fluent keys (DEC-054).
    let en = crate::validate::fluent_keys(&b.locale_text("en"));
    let mut keys: Vec<(String, String)> = Vec::new();
    for o in &b.outlets {
        keys.push((o.id.clone(), o.name_key.clone()));
    }
    for k in hidden_kinds(b) {
        keys.push(("media".into(), format!("fact-{}", k.replace('_', "-"))));
    }
    for (owner, key) in keys {
        if !en.contains(key.as_str()) {
            issues.push(Issue::error(
                &owner,
                &format!("missing Fluent key `{key}` in locale en"),
            ));
        }
    }
}
