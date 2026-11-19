//! View models: read-only, per-screen projections of simulation state
//! (OVERVIEW §4.5). Text is never prose: events carry Fluent keys and args.

use mandate_sim::Engine;
use mandate_sim::events::{DomainEvent, EventKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DateView {
    pub day_number: u32,
    pub year: u32,
    pub month: u32,
    pub day: u32,
}

impl DateView {
    pub fn of(d: mandate_core::SimDate) -> Self {
        let c = d.calendar();
        DateView {
            day_number: d.0,
            year: c.year,
            month: c.month,
            day: c.day,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RegionView {
    pub id: u32,
    pub name: String,
    pub population: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EventView {
    pub id: u32,
    pub date: DateView,
    /// Fluent message key, e.g. `event-year-began`.
    pub text_key: String,
    pub args: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OverviewView {
    pub date: DateView,
    pub turn: u32,
    pub turn_days: u32,
    pub seed: String,
    pub content_hash: String,
    pub state_hash: String,
    pub total_population: f64,
    pub regions: Vec<RegionView>,
    pub recent_events: Vec<EventView>,
    pub economy: NationalView,
    /// The run's storyteller profile (DEC-033): id and Fluent name key.
    pub storyteller: String,
    pub storyteller_name_key: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NationalView {
    pub unemployment: f64,
    pub inflation: f64,
    pub policy_rate: f64,
    pub gdp: f64,
    pub gdp_growth_annual: f64,
    pub debt_pct_gdp: f64,
    pub in_recession: bool,
    /// Monthly, oldest first (up to 10 years).
    pub unemployment_history: Vec<f64>,
    pub inflation_history: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CommandAppliedView {
    pub days_advanced: u32,
    pub overview: OverviewView,
    pub result: Option<crate::ActionResultView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LocaleView {
    pub lang: String,
    pub ftl: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DiagnosticsView {
    pub engine_version: String,
    pub protocol_version: u32,
    pub systems: Vec<String>,
    pub event_count: u32,
    pub state_hash: String,
}

fn muni_name(engine: &Engine, m: u32) -> String {
    engine
        .state()
        .world
        .municipalities
        .get(m as usize)
        .map(|x| x.name.clone())
        .unwrap_or_default()
}

fn pol_name(engine: &Engine, p: u32) -> String {
    engine
        .state()
        .politics
        .get(mandate_sim::politics::PoliticianId(p))
        .map(|x| x.full_name())
        .unwrap_or_default()
}

fn snake_debug<T: std::fmt::Debug>(v: &T) -> String {
    let s = format!("{v:?}");
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn put_policy(put: &mut dyn FnMut(&str, String), engine: &Engine, policy: u32, municipality: u32) {
    let st = engine.state();
    let key = st
        .policy
        .instances
        .get(policy as usize)
        .and_then(|p| engine.content().policies.get(&p.template))
        .map(|t| t.name_key.clone())
        .unwrap_or_default();
    put("policy_key", key);
    put("town", muni_name(engine, municipality));
}

pub fn event_view(e: &DomainEvent, engine: &Engine) -> EventView {
    let mut args = BTreeMap::new();
    let mut put = |k: &str, v: String| {
        args.insert(k.to_string(), v);
    };
    let key = match &e.kind {
        EventKind::GameStarted { seed } => {
            put("seed", seed.to_string());
            "event-game-started"
        }
        EventKind::HistoryBegan { year } => {
            put("year", year.to_string());
            "event-history-began"
        }
        EventKind::YearBegan { year } => {
            put("year", year.to_string());
            "event-year-began"
        }
        EventKind::TurnEnded { turn } => {
            put("turn", turn.to_string());
            "event-turn-ended"
        }
        EventKind::GlobalRecessionBegan { depth_pct } => {
            put("depth", depth_pct.to_string());
            "event-global-recession"
        }
        EventKind::GlobalRecoveryBegan => "event-global-recovery",
        EventKind::CommodityShock {
            commodity,
            change_pct,
        } => {
            put(
                "commodity_key",
                format!(
                    "commodity-{}",
                    ["agriculture", "minerals", "energy"]
                        .get(*commodity as usize)
                        .copied()
                        .unwrap_or("energy")
                ),
            );
            put("change", change_pct.to_string());
            if *change_pct >= 0 {
                "event-commodity-spike"
            } else {
                "event-commodity-slump"
            }
        }
        EventKind::PlantClosure {
            municipality,
            sector,
            jobs,
        } => {
            put("town", muni_name(engine, *municipality));
            put(
                "sector_key",
                engine
                    .content()
                    .sector(sector)
                    .map(|d| d.name_key.clone())
                    .unwrap_or_default(),
            );
            put("jobs", jobs.to_string());
            "event-plant-closure"
        }
        EventKind::InterestRateMove { change_bp, rate_bp } => {
            put("rate", format!("{:.2}", *rate_bp as f64 / 100.0));
            if *change_bp >= 0 {
                "event-rate-rise"
            } else {
                "event-rate-cut"
            }
        }
        EventKind::Disaster {
            municipality,
            hazard,
            severity_pct,
        } => {
            put("town", muni_name(engine, *municipality));
            put("hazard_key", format!("hazard-{}", snake_debug(hazard)));
            put("severity", severity_pct.to_string());
            "event-disaster"
        }
        EventKind::PolicyIntroduced {
            policy,
            municipality,
            ..
        } => {
            put_policy(&mut put, engine, *policy, *municipality);
            "event-policy-introduced"
        }
        EventKind::PolicyEnacted {
            policy,
            municipality,
            yes,
            no,
            overrode_veto,
        } => {
            put_policy(&mut put, engine, *policy, *municipality);
            put("yes", yes.to_string());
            put("no", no.to_string());
            if *overrode_veto {
                "event-policy-override"
            } else {
                "event-policy-enacted"
            }
        }
        EventKind::PolicyFailed {
            policy,
            municipality,
            vetoed,
            yes,
            no,
        } => {
            put_policy(&mut put, engine, *policy, *municipality);
            put("yes", yes.to_string());
            put("no", no.to_string());
            if *vetoed {
                "event-policy-vetoed"
            } else {
                "event-policy-defeated"
            }
        }
        EventKind::PolicyExpired {
            policy,
            municipality,
        } => {
            put_policy(&mut put, engine, *policy, *municipality);
            "event-policy-expired"
        }
        EventKind::PolicyWithdrawn {
            policy,
            municipality,
        } => {
            put_policy(&mut put, engine, *policy, *municipality);
            "event-policy-withdrawn"
        }
        EventKind::CampaignOpened { election_day } => {
            let d = election_day.calendar();
            put("day", d.day.to_string());
            put("month_key", format!("month-{}", d.month));
            put("year", d.year.to_string());
            "event-campaign-opened"
        }
        EventKind::PollsPublished { count } => {
            put("count", count.to_string());
            "event-polls-published"
        }
        EventKind::RaceDecided {
            municipality,
            mayor,
            winner,
            incumbent_defeated,
            player_won,
            player_ran,
        } => {
            put("town", muni_name(engine, *municipality));
            let name = engine
                .state()
                .politics
                .get(mandate_sim::politics::PoliticianId(*winner))
                .map(|p| p.full_name())
                .unwrap_or_default();
            put("name", name);
            match (*player_ran, *player_won, *mayor, *incumbent_defeated) {
                (true, true, true, _) => "event-player-elected-mayor",
                (true, true, false, _) => "event-player-elected",
                (true, false, _, _) => "event-player-defeated",
                (false, _, true, true) => "event-mayor-ousted",
                (false, _, true, false) => "event-mayor-elected",
                (false, _, false, true) => "event-councillor-ousted",
                (false, _, false, false) => "event-councillor-elected",
            }
        }
        EventKind::ElectionHeld { races } => {
            put("races", races.to_string());
            "event-election-held"
        }
        EventKind::PlayerDeselected => "event-player-deselected",
        EventKind::DecisionOpened { template, .. } => {
            let t = engine.content().dilemma(template);
            put(
                "title_key",
                t.map(|t| t.title_key.clone()).unwrap_or_default(),
            );
            "event-decision-opened"
        }
        EventKind::DecisionResolved {
            template,
            by_default,
            ..
        } => {
            let t = engine.content().dilemma(template);
            put(
                "title_key",
                t.map(|t| t.title_key.clone()).unwrap_or_default(),
            );
            if *by_default {
                "event-decision-defaulted"
            } else {
                "event-decision-answered"
            }
        }
        EventKind::DecisionLapsed { template, .. } => {
            let t = engine.content().dilemma(template);
            put(
                "title_key",
                t.map(|t| t.title_key.clone()).unwrap_or_default(),
            );
            "event-decision-lapsed"
        }
        EventKind::ChapterBegan { kind, .. } => {
            put("kind_key", format!("chapter-kind-{kind}"));
            "event-chapter-began"
        }
        EventKind::StorylineStage { run, stage, .. } => {
            // The stage's own story line, with its cast.
            let st = engine.state();
            if let Some(r) = st.narrative.storylines.iter().find(|r| r.id.0 == *run)
                && let Some(def) = engine.content().storyline(&r.template)
                && let Some(s) = def.stages.iter().find(|s| &s.id == stage)
            {
                for (k, v) in crate::narrative::cast_args(engine, &r.cast) {
                    put(&k, v);
                }
                return EventView {
                    id: e.id.0,
                    date: DateView::of(e.date),
                    text_key: s.key.clone(),
                    args,
                };
            }
            "event-storyline-advanced"
        }
        EventKind::StoryDevelopment { fact, .. } => {
            let st = engine.state();
            if let Some(f) = st.narrative.facts.iter().find(|f| f.id == *fact)
                && let Some(v) = crate::narrative::fact_event(engine, f)
            {
                return EventView {
                    id: e.id.0,
                    date: DateView::of(e.date),
                    text_key: v.text_key,
                    args: v.args,
                };
            }
            "event-storyline-advanced"
        }
        EventKind::TrackWarning { track, .. } => {
            put("track_key", format!("track-{track}"));
            "event-track-warning"
        }
        EventKind::DirectorIntervention {
            lever,
            municipality,
        } => {
            put("lever", lever.clone());
            put("town", muni_name(engine, *municipality));
            "event-director-intervention"
        }
        EventKind::FactDiscoverable { .. } => "event-fact-discoverable",
        EventKind::ScandalBroke {
            politician,
            municipality,
            kind,
            ..
        } => {
            put("name", pol_name(engine, *politician));
            put("town", muni_name(engine, *municipality));
            put("fact_key", format!("fact-{}", kind.replace('_', "-")));
            "event-scandal-broke"
        }
        EventKind::CaseOpened { politician, .. } => {
            put("name", pol_name(engine, *politician));
            "event-case-opened"
        }
        EventKind::CaseDeveloped {
            politician, stage, ..
        } => {
            put("name", pol_name(engine, *politician));
            put("stage_key", format!("case-stage-{stage}"));
            "event-case-developed"
        }
        EventKind::PlayerImprisoned { years, until } => {
            put("years", years.to_string());
            put("year", until.calendar().year.to_string());
            "event-player-imprisoned"
        }
        EventKind::PlayerReleased => "event-player-released",
        EventKind::CareerEnded { reason } => {
            put("reason_key", format!("ending-{reason}"));
            "event-career-ended"
        }
        EventKind::RumourSpread { politician, .. } => {
            put("name", pol_name(engine, *politician));
            "event-rumour-spread"
        }
        EventKind::RumourCorrected { politician, .. } => {
            put("name", pol_name(engine, *politician));
            "event-rumour-corrected"
        }
        EventKind::RulesUpdated { to_engine, .. } => {
            put("version", to_engine.clone());
            "event-rules-updated"
        }
        EventKind::NationalBudget {
            year,
            balance,
            debt_pct_gdp,
        } => {
            put("year", year.to_string());
            put("balance", balance.to_string());
            put("debt", debt_pct_gdp.to_string());
            if *balance >= 0 {
                "event-budget-surplus"
            } else {
                "event-budget-deficit"
            }
        }
    };
    EventView {
        id: e.id.0,
        date: DateView::of(e.date),
        text_key: key.into(),
        args,
    }
}

pub fn overview(engine: &Engine) -> OverviewView {
    let s = engine.state();
    let recent_events = s
        .events
        .iter()
        .rev()
        .filter(|e| {
            !matches!(
                e.kind,
                EventKind::TurnEnded { .. }
                    | EventKind::YearBegan { .. }
                    // The director's levers are logged, not news (diagnostics).
                    | EventKind::DirectorIntervention { .. }
                    | EventKind::FactDiscoverable { .. }
            )
        })
        .take(15)
        .map(|e| event_view(e, engine))
        .collect();
    let hash = engine.state_hash();
    let n = &s.economy.national;
    let debt = s
        .finance
        .national
        .as_ref()
        .map(|t| t.debt.units() as f64)
        .unwrap_or(0.0);
    let profile = s.narrative.profile(engine.content());
    OverviewView {
        storyteller: profile.map(|p| p.id.clone()).unwrap_or_default(),
        storyteller_name_key: profile.map(|p| p.name_key.clone()).unwrap_or_default(),
        date: DateView::of(s.calendar.date),
        turn: s.calendar.turn,
        turn_days: engine.turn_days(),
        seed: s.meta.seed.to_string(),
        content_hash: s.meta.content_hash.chars().take(12).collect(),
        state_hash: hash.chars().take(16).collect(),
        total_population: s.population.total() as f64,
        regions: s
            .world
            .regions
            .iter()
            .map(|r| RegionView {
                id: r.id.0,
                name: r.name.clone(),
                population: s.population.region_total(&s.world, r.id) as f64,
            })
            .collect(),
        recent_events,
        economy: NationalView {
            unemployment: n.unemployment,
            inflation: n.inflation,
            policy_rate: n.policy_rate,
            gdp: n.gdp,
            gdp_growth_annual: n.gdp_growth * 12.0,
            debt_pct_gdp: if n.gdp > 0.0 { debt / n.gdp } else { 0.0 },
            in_recession: s.economy.global.recession.is_some(),
            unemployment_history: n
                .history
                .iter()
                .rev()
                .take(120)
                .rev()
                .map(|h| h.unemployment as f64)
                .collect(),
            inflation_history: n
                .history
                .iter()
                .rev()
                .take(120)
                .rev()
                .map(|h| h.inflation as f64)
                .collect(),
        },
    }
}

pub fn diagnostics(engine: &Engine) -> DiagnosticsView {
    DiagnosticsView {
        engine_version: mandate_core::ENGINE_VERSION.to_string(),
        protocol_version: crate::PROTOCOL_VERSION,
        systems: engine
            .system_names()
            .iter()
            .map(|s| s.to_string())
            .collect(),
        event_count: engine.state().events.len() as u32,
        state_hash: engine.state_hash(),
    }
}
