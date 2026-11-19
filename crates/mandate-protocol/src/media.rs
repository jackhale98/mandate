//! The press, scandals, and the legal path for the client (DEC-034,
//! DEC-006): the newspaper (current coverage with outlet names and
//! framing), coarse headlines for the pocket, the player's scandals and
//! cases with their stage, and where the career stands. Facts and Fluent
//! keys only (DEC-054).

use crate::views::{DateView, event_view};
use mandate_content::ContentBundle;
use mandate_sim::Engine;
use mandate_sim::justice::{Case, Stage};
use mandate_sim::media::{Outlet, Scope, Story, StorySource};
use mandate_sim::player::RunStatus;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// An outlet as a reader sees it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OutletView {
    pub id: u32,
    /// Fluent key of its name, with `place` as `$place`.
    pub name_key: String,
    pub place: String,
    /// `town` | `region` | `nation` | `social`.
    pub kind: String,
    /// `low` | `medium` | `high` (coarse).
    pub credibility: String,
}

/// A story an outlet ran.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArticleView {
    pub id: u32,
    pub outlet: OutletView,
    pub date: DateView,
    /// Fluent key with `args` (argument names ending in `_key` are keys).
    pub headline_key: String,
    pub args: BTreeMap<String, String>,
    /// `hostile` | `straight` | `sympathetic` (the outlet's framing).
    pub framing: String,
    /// About the player.
    pub about_you: bool,
}

/// The newspaper (DEC-079: the newspaper office): what the outlets that
/// reach the player's town are running, newest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NewspaperView {
    pub outlets: Vec<OutletView>,
    pub articles: Vec<ArticleView>,
}

/// A scandal about the player.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScandalView {
    pub id: u32,
    pub fact_key: String,
    /// `ethics` | `misconduct` | `crime`.
    pub severity: String,
    pub broke: DateView,
    pub outlet: Option<OutletView>,
    /// `fading` | `running` | `burning` (coarse heat).
    pub heat: String,
    pub response: Option<String>,
    pub running: bool,
}

/// A case against the player.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CaseView {
    pub id: u32,
    pub fact_key: String,
    pub severity: String,
    /// `investigation` | `charged` | `adjourned` | `dropped` | `acquitted` | `convicted`.
    pub stage: String,
    pub opened: DateView,
    /// When the current stage ends (open cases).
    pub next: Option<DateView>,
    pub response: Option<String>,
    /// Coarse odds of the next step: `low` | `real` | `high` (charges at an
    /// investigation, a conviction at trial).
    pub odds: Option<String>,
    /// The sentence, if convicted: `fine` | `removed` | `prison` | `disqualified`.
    pub sentence: Option<String>,
    pub years: u32,
}

/// Where the career stands (DEC-006): `active`, `imprisoned`, or `ended`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RunView {
    pub status: String,
    /// Release date in prison.
    pub until: Option<DateView>,
    /// Why it ended (`prison`, `disqualified`), with the case's fact.
    pub reason: Option<String>,
    pub fact_key: Option<String>,
    pub years: u32,
    pub date: Option<DateView>,
}

/// Scandals and the legal path in the pocket (DEC-079: always available).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LegalView {
    pub run: RunView,
    pub scandals: Vec<ScandalView>,
    pub cases: Vec<CaseView>,
}

fn credibility(c: f32) -> &'static str {
    if c < 0.5 {
        "low"
    } else if c < 0.75 {
        "medium"
    } else {
        "high"
    }
}

pub fn outlet_view(content: &ContentBundle, o: &Outlet) -> OutletView {
    let (name_key, place) = mandate_sim::media::outlet_name(content, o);
    OutletView {
        id: o.id,
        name_key,
        place,
        kind: if o.social {
            "social"
        } else {
            match o.scope {
                Scope::Town { .. } => "town",
                Scope::Region { .. } => "region",
                Scope::Nation => "nation",
            }
        }
        .into(),
        credibility: credibility(o.credibility).into(),
    }
}

fn fact_key(kind: &str) -> String {
    format!("fact-{}", kind.replace('_', "-"))
}

fn name(engine: &Engine, p: mandate_sim::politics::PoliticianId) -> String {
    engine
        .state()
        .politics
        .get(p)
        .map(|x| x.full_name())
        .unwrap_or_default()
}

/// A story as a reader sees it.
pub fn article(engine: &Engine, s: &Story) -> Option<ArticleView> {
    let st = engine.state();
    let content = engine.content();
    let o = st.media.outlet(s.outlet)?;
    let mut args = BTreeMap::new();
    if let Some(p) = s.subject {
        args.insert("name".into(), name(engine, p));
    }
    if let Some(m) = st.world.municipalities.get(s.town as usize) {
        args.insert("town".into(), m.name.clone());
    }
    let headline_key = match &s.source {
        StorySource::Scandal { id } => {
            let x = st.media.scandal(*id)?;
            args.insert("fact_key".into(), fact_key(&x.kind));
            args.insert(
                "severity_key".into(),
                format!("severity-{}", x.severity.key()),
            );
            if x.stories <= 1 && x.broke == s.date {
                "headline-scandal-breaks".to_string()
            } else {
                "headline-scandal".to_string()
            }
        }
        StorySource::Response { scandal, response } => {
            let x = st.media.scandal(*scandal)?;
            args.insert("fact_key".into(), fact_key(&x.kind));
            format!("headline-response-{}", response.key().replace('_', "-"))
        }
        StorySource::Backfire { scandal } => {
            let x = st.media.scandal(*scandal)?;
            args.insert("fact_key".into(), fact_key(&x.kind));
            "headline-backfire".into()
        }
        StorySource::Case { id, stage } => {
            if let Some(c) = st.justice.case(*id) {
                args.insert("fact_key".into(), fact_key(&c.kind));
                if let Some(sen) = c.sentence.filter(|_| stage == "convicted") {
                    args.insert("sentence_key".into(), format!("sentence-{}", sen.key()));
                    args.insert("years".into(), sen.prison_years().to_string());
                }
            }
            format!("headline-case-{stage}")
        }
        StorySource::Rumour { .. } => "headline-rumour".into(),
        StorySource::Correction { .. } => "headline-correction".into(),
        StorySource::Statement { policy, .. } => {
            if let Some(t) = policy
                .and_then(|p| st.policy.instances.get(p as usize))
                .and_then(|p| content.policies.get(&p.template))
            {
                args.insert("bill_key".into(), t.name_key.clone());
                "headline-statement-bill".into()
            } else {
                "headline-statement".into()
            }
        }
        StorySource::Town { event } => {
            let e = st.events.get(*event)?;
            let v = event_view(e, engine);
            args.extend(v.args);
            v.text_key
        }
    };
    Some(ArticleView {
        id: s.id,
        outlet: outlet_view(content, o),
        date: DateView::of(s.date),
        headline_key,
        args,
        framing: if s.framing < -0.25 {
            "hostile"
        } else if s.framing > 0.25 {
            "sympathetic"
        } else {
            "straight"
        }
        .into(),
        about_you: s.subject.is_some() && s.subject == st.player.as_ref().map(|p| p.politician),
    })
}

/// The outlets that reach the player's town, and what they ran lately.
pub fn newspaper_view(engine: &Engine, limit: usize) -> Option<NewspaperView> {
    let st = engine.state();
    let pl = st.player.as_ref()?;
    let m = pl.municipality.0;
    let outlets: Vec<&Outlet> = st
        .media
        .outlets
        .iter()
        .filter(|o| o.covers(&st.world, m))
        .collect();
    // The latest stories in town, with the latest about the player always
    // among them; newest first.
    let reach = |s: &&Story| outlets.iter().any(|o| o.id == s.outlet);
    let mut picked: Vec<&Story> = st
        .media
        .stories
        .iter()
        .rev()
        .filter(reach)
        .filter(|s| s.subject == Some(pl.politician))
        .take(limit.min(6))
        .collect();
    for s in st
        .media
        .stories
        .iter()
        .rev()
        .filter(reach)
        .filter(|s| s.town == m && s.subject != Some(pl.politician))
    {
        if picked.len() >= limit {
            break;
        }
        picked.push(s);
    }
    picked.sort_by(|a, b| b.id.cmp(&a.id));
    let articles = picked
        .into_iter()
        .filter_map(|s| article(engine, s))
        .collect();
    Some(NewspaperView {
        outlets: outlets
            .into_iter()
            .map(|o| outlet_view(engine.content(), o))
            .collect(),
        articles,
    })
}

fn heat(h: f32) -> &'static str {
    if h < 0.35 {
        "fading"
    } else if h < 0.75 {
        "running"
    } else {
        "burning"
    }
}

fn case_view(engine: &Engine, c: &Case) -> CaseView {
    let content = engine.content();
    let st = engine.state();
    let reads = mandate_sim::scheduler::Reads::of_state(st, content);
    let odds = match c.stage {
        Stage::Investigation => Some(mandate_sim::justice::charge_chance(content, c)),
        Stage::Charged | Stage::Adjourned => {
            Some(mandate_sim::justice::conviction_chance(&reads, c))
        }
        _ => None,
    }
    .map(|p| {
        if p < 0.3 {
            "low"
        } else if p < 0.6 {
            "real"
        } else {
            "high"
        }
        .to_string()
    });
    CaseView {
        id: c.id,
        fact_key: fact_key(&c.kind),
        severity: c.severity.key().into(),
        stage: c.stage.key().into(),
        opened: DateView::of(c.opened),
        next: c.stage.open().then(|| DateView::of(c.next_on)),
        response: c.response.map(|r| r.key().into()),
        odds,
        sentence: c.sentence.map(|s| s.key().into()),
        years: c.sentence.map(|s| s.prison_years() as u32).unwrap_or(0),
    }
}

/// The player's scandals (running, then the last few) and cases, and where
/// the career stands.
pub fn legal_view(engine: &Engine) -> Option<LegalView> {
    let st = engine.state();
    let content = engine.content();
    let pl = st.player.as_ref()?;
    let me = pl.politician;
    let mut scandals: Vec<ScandalView> = st
        .media
        .scandals
        .iter()
        .rev()
        .filter(|s| s.subject == me)
        .take(5)
        .map(|s| ScandalView {
            id: s.id,
            fact_key: fact_key(&s.kind),
            severity: s.severity.key().into(),
            broke: DateView::of(s.broke),
            outlet: s
                .outlet
                .and_then(|o| st.media.outlet(o))
                .map(|o| outlet_view(content, o)),
            heat: heat(s.heat).into(),
            response: s.response.map(|r| r.key().into()),
            running: s.running(),
        })
        .collect();
    scandals.sort_by_key(|s| !s.running);
    let cases = st
        .justice
        .cases
        .iter()
        .rev()
        .filter(|c| c.subject == me)
        .take(5)
        .map(|c| case_view(engine, c))
        .collect();
    let case_fact = |id: u32| st.justice.case(id).map(|c| fact_key(&c.kind));
    let run = match pl.status {
        RunStatus::Active => RunView {
            status: "active".into(),
            until: None,
            reason: None,
            fact_key: None,
            years: 0,
            date: None,
        },
        RunStatus::Imprisoned { until, case } => RunView {
            status: "imprisoned".into(),
            until: Some(DateView::of(until)),
            reason: Some("prison".into()),
            fact_key: case_fact(case),
            years: st
                .justice
                .case(case)
                .and_then(|c| c.sentence)
                .map(|s| s.prison_years() as u32)
                .unwrap_or(0),
            date: None,
        },
        RunStatus::Ended { date, case, reason } => RunView {
            status: "ended".into(),
            until: None,
            reason: Some(reason.key().into()),
            fact_key: case_fact(case),
            years: st
                .justice
                .case(case)
                .and_then(|c| c.sentence)
                .map(|s| s.prison_years() as u32)
                .unwrap_or(0),
            date: Some(DateView::of(date)),
        },
    };
    Some(LegalView {
        run,
        scandals,
        cases,
    })
}

/// A shady action's risk line (DEC-073): what it pays, and a coarse chance it
/// comes out within a year.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ShadyOfferView {
    pub action: String,
    pub money: f64,
    pub health: f64,
    /// `ethics` | `misconduct` | `crime`.
    pub severity: String,
    /// `low` | `real` | `high`.
    pub risk: String,
    /// Not possible now (no one offering, no rival), as a Fluent key.
    pub unavailable_key: Option<String>,
}

pub fn shady_offers(engine: &Engine) -> Vec<ShadyOfferView> {
    let st = engine.state();
    let content = engine.content();
    let Some(pl) = st.player.as_ref() else {
        return Vec::new();
    };
    content
        .media
        .shady_actions
        .iter()
        .map(|(k, d)| {
            let e = mandate_sim::media::exposure(st, content, &d.hidden, pl.politician);
            let p = mandate_sim::media::risk_within_horizon(content, &e);
            let unavailable_key = match k.as_str() {
                "take_bribe" if mandate_sim::actions::briber(st).is_none() => {
                    Some("ui-shady-no-offer".to_string())
                }
                "leak_file" if mandate_sim::actions::leak_target(st).is_none() => {
                    Some("ui-shady-no-rival".to_string())
                }
                "dip_funds" if (pl.money.units() as f64) < -d.money => {
                    Some("ui-not-enough-money".to_string())
                }
                _ => None,
            };
            ShadyOfferView {
                action: k.clone(),
                money: d.money,
                health: d.health,
                severity: d.hidden.severity.key().into(),
                risk: mandate_sim::media::band(content, p).into(),
                unavailable_key,
            }
        })
        .collect()
}
