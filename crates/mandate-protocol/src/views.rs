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
    /// The act flow (DEC-012, DEC-006): over, and a skip in progress.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub act: Option<crate::ActStatusView>,
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
    /// A skip's progress (`CommandDto::Skip`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub skip: Option<crate::SkipProgressView>,
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
        .and_then(|p| mandate_sim::policy::resolve(engine.content(), &engine.state().policy, p))
        .map(|t| t.name_key.clone())
        .unwrap_or_default();
    put("policy_key", key);
    put("town", muni_name(engine, municipality));
}

/// An ask's or a promise's name key, by template id.
fn ask_name_key(engine: &Engine, template: &str) -> String {
    let c = engine.content();
    c.ask(template)
        .map(|t| t.name_key.clone())
        .or_else(|| c.promise(template).map(|t| t.name_key.clone()))
        .unwrap_or_default()
}

/// An arc event's line (DEC-109), until the arc view (task A3): a stage's
/// story so far (an exit's: how it ended), a story's headline, a cast
/// person's move, a visit's outcome; with the run's cast by role, the
/// facts recorded with the event, and its dates. Facts and keys only.
fn arc_event(e: &DomainEvent, engine: &Engine) -> Option<(String, BTreeMap<String, String>)> {
    use mandate_content::arcs::BeatKind;
    use mandate_sim::arcs::{ArcId, cast};
    let st = engine.state();
    let content = engine.content();
    let mut args: BTreeMap<String, String> = BTreeMap::new();
    let (arc, template, at, key) = match &e.kind {
        EventKind::ArcRaised { arc, template, at } => {
            let title = content.arc(template).map(|a| a.title_key.clone());
            args.insert("title_key".into(), title.unwrap_or_default());
            (*arc, template, *at, "event-arc-raised".to_string())
        }
        EventKind::ArcStage {
            arc,
            template,
            stage: id,
            at,
            facts,
        }
        | EventKind::ArcEnded {
            arc,
            template,
            exit: id,
            at,
            facts,
        } => {
            args.extend(facts.iter().cloned());
            let key = content.arc(template)?.stage(id)?.key.clone();
            (*arc, template, *at, key)
        }
        EventKind::ArcBeat {
            arc,
            template,
            stage,
            beat,
            at,
            kind,
            ok,
            facts,
        } => {
            args.extend(facts.iter().cloned());
            args.insert("ok".into(), if *ok { "yes" } else { "no" }.into());
            let b = content.arc(template)?.stage(stage)?.beat(beat)?;
            let key = match &b.kind {
                BeatKind::Press { headline_key } => headline_key.clone(),
                BeatKind::Move { line_key, .. } => line_key.clone(),
                BeatKind::Visit { outcome_key, .. } => outcome_key.clone(),
                BeatKind::Talk { with, .. } => {
                    args.insert("with_role".into(), with.clone());
                    "event-arc-talk".into()
                }
                _ => format!("event-arc-{kind}"),
            };
            (*arc, template, *at, key)
        }
        _ => return None,
    };
    let _ = template;
    args.insert("place".into(), cast::jurisdiction_name(&st.world, at));
    let id = ArcId(arc);
    if let Some(c) = st.arcs.cast_of(id) {
        for (role, name) in cast::names(&st.politics, &st.world, c) {
            args.entry(role).or_insert(name);
        }
        // People with their role words (and only as the player knows them).
        let people: Vec<mandate_sim::narrative::CastEntry> = c
            .iter()
            .filter_map(|x| match x.actor {
                mandate_sim::arcs::ArcActor::Person { id } => {
                    Some(mandate_sim::narrative::CastEntry {
                        role: x.role.clone(),
                        actor: mandate_sim::narrative::Actor::Person { id },
                    })
                }
                _ => None,
            })
            .collect();
        for (k, v) in crate::narrative::cast_args(engine, &people) {
            args.insert(k, v);
        }
        if let Some(role) = args.get("with_role").cloned() {
            let name = args.get(&role).cloned().unwrap_or_default();
            args.insert("with".into(), name);
        }
    }
    // A beat's event carries no facts of its own: its run's dates.
    let began = st
        .arcs
        .run(id)
        .map(|r| r.began)
        .or_else(|| st.arcs.past.iter().find(|p| p.id == id).map(|p| p.began));
    if let Some(b) = began {
        let c = b.calendar();
        args.entry("day".into()).or_insert(c.day.to_string());
        args.entry("month_key".into())
            .or_insert(format!("month-{}", c.month));
        args.entry("year".into()).or_insert(c.year.to_string());
        let days = if e.date >= b { b.days_until(e.date) } else { 0 };
        args.entry("weeks".into()).or_insert((days / 7).to_string());
    }
    Some((key, args))
}

pub fn event_view(e: &DomainEvent, engine: &Engine) -> EventView {
    if let Some((text_key, args)) = arc_event(e, engine) {
        return EventView {
            id: e.id.0,
            date: DateView::of(e.date),
            text_key,
            args,
        };
    }
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
            ..
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
        EventKind::NominationRefused {
            post,
            reason,
            nominee,
            standing,
            min_standing,
            instead,
        } => crate::campaign::refusal_args(
            engine,
            *post,
            reason,
            *nominee,
            *standing,
            *min_standing,
            *instead,
            &mut args,
        ),
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
        // Asks and promises (DEC-097 §2); the full view is E7's `AskView`.
        EventKind::PlayerDeed { .. } => "event-player-deed",
        // An ask's name takes the ask's own arguments (who asked, where,
        // about which bill), so they travel with the line.
        EventKind::AskOffered { ask, template, .. } => {
            put("ask_key", ask_name_key(engine, template));
            crate::asks::outcome_args(engine, *ask, &mut args);
            "event-ask-offered"
        }
        EventKind::AskAccepted { ask, template, .. } => {
            put("ask_key", ask_name_key(engine, template));
            crate::asks::outcome_args(engine, *ask, &mut args);
            "event-ask-accepted"
        }
        EventKind::AskResolved {
            ask,
            template,
            outcome,
            ..
        } => {
            put("ask_key", ask_name_key(engine, template));
            put("outcome", outcome.clone());
            crate::asks::outcome_args(engine, *ask, &mut args);
            "event-ask-resolved"
        }
        EventKind::RulesUpdated { to_engine, .. } => {
            put("version", to_engine.clone());
            "event-rules-updated"
        }
        // Policy families (DEC-092); their own views come with F3.
        EventKind::ProposalOpened { family, .. } => {
            put("family", family.clone());
            "event-proposal-opened"
        }
        EventKind::ProposalLapsed { family, .. } => {
            put("family", family.clone());
            "event-proposal-lapsed"
        }
        // An arc's events have their own lines (`arc_event`); one whose
        // template an update removed has only this.
        EventKind::ArcRaised { .. }
        | EventKind::ArcStage { .. }
        | EventKind::ArcBeat { .. }
        | EventKind::ArcEnded { .. } => "event-arc-raised",
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
                    // What the player did is in their action log.
                    | EventKind::PlayerDeed { .. }
                    // An arc's first stage tells of its raising.
                    | EventKind::ArcRaised { .. }
            ) && !matches!(&e.kind, EventKind::ArcBeat { kind, .. }
                if kind != "press" && kind != "move")
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
        act: s.player.as_ref().map(|_| crate::act_status(engine)),
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
