//! The experiment runner (S2): a game from a seed, run month by month under
//! a driver, sampled at chosen months; twins that differ by one
//! intervention; and a deterministic parallel map over independent runs.

use super::metrics::{self, Snapshot, Tally};
use mandate_content::ContentBundle;
use mandate_content::overrides::Override;
use mandate_core::SimDate;
use mandate_sim::environment::{DeferredHazard, HazardKind};
use mandate_sim::events::EventKind;
use mandate_sim::policy::PolicyStatus;
use mandate_sim::spine::Cause;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;

/// The world a run starts from.
#[derive(Clone, Debug, Serialize)]
pub struct Setup {
    pub seed: u64,
    #[serde(serialize_with = "size_name")]
    pub size: WorldSize,
    /// A scenario id (`scenario.wealthy_coast`), or a random country.
    pub scenario: Option<String>,
}

fn size_name<S: serde::Serializer>(s: &WorldSize, ser: S) -> Result<S::Ok, S::Error> {
    ser.serialize_str(match s {
        WorldSize::Small => "small",
        WorldSize::Medium => "medium",
        WorldSize::Large => "large",
    })
}

impl Setup {
    pub fn small(seed: u64, scenario: Option<&str>) -> Self {
        Setup {
            seed,
            size: WorldSize::Small,
            scenario: scenario.map(str::to_string),
        }
    }
}

/// Who plays.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Driver {
    /// No turns: the calendar advances by days (NPC turns do not run; the
    /// sweep's world).
    None,
    /// Turns end without the player acting (NPC turns run).
    Idle,
    /// A bot strategy from `bots` plays every turn.
    Bot { strategy: String },
    /// A stand-in player (`mandate_sim::standin`: `passive`, `engaged`,
    /// `contrarian`, `attends`) plays, by commands alone; `attends` skips
    /// the quiet weeks.
    StandIn { kind: String },
}

/// The prefix of a stand-in driver's name (`standin:engaged`).
pub const STANDIN: &str = "standin:";

impl Driver {
    pub fn parse(s: &str) -> Driver {
        match s {
            "none" => Driver::None,
            "idle" => Driver::Idle,
            s if s.starts_with(STANDIN) => Driver::StandIn {
                kind: s[STANDIN.len()..].to_string(),
            },
            s => Driver::Bot {
                strategy: s.to_string(),
            },
        }
    }
    pub fn name(&self) -> String {
        match self {
            Driver::None => "none".into(),
            Driver::Idle => "idle".into(),
            Driver::Bot { strategy } => strategy.clone(),
            Driver::StandIn { kind } => format!("{STANDIN}{kind}"),
        }
    }
    pub fn standin(kind: mandate_sim::standin::Kind) -> Driver {
        Driver::StandIn {
            kind: kind.key().to_string(),
        }
    }
}

/// One change made to a game at a date.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Intervention {
    /// A law from a policy template, enacted at once in a town (no vote),
    /// with these parameters (others at their default). Funding and
    /// implementation then run as for any law.
    Policy {
        template: String,
        town: u32,
        params: BTreeMap<String, f64>,
    },
    /// Content parameters changed from this date (a balance patch: the
    /// state is reloaded with the new content).
    Set {
        sets: Vec<(String, serde_json::Value)>,
    },
    /// The lab's sector demand shock (`Command::LabSectorShock`).
    SectorShock {
        town: u32,
        sector: String,
        factor: f64,
        months: u16,
    },
    /// A plant closes (DEC-110): the town's largest plant that can close
    /// (in the given sector, if one is given), by the engine's own closure.
    /// `share` is the least of its sector's capacity that goes: where the
    /// plant is smaller, unnamed firms of its industry go with it; where
    /// the town has no such plant, unnamed firms of its largest closing
    /// sector go, under the sector's name.
    PlantClosure {
        town: u32,
        sector: Option<String>,
        share: f64,
    },
    /// A fall in demand for one industry (DEC-110), in one town or in
    /// every town that has it: what brings a plant's review on.
    IndustryShock {
        town: Option<u32>,
        industry: String,
        factor: f64,
        months: u16,
    },
    /// A fall in demand for the industry of the town's largest open plant
    /// that can close (DEC-110), in that town: what puts a plant on short
    /// time and, held long enough, under notice. An `IndustryShock` once the
    /// plant is found; nothing where the town has none.
    PlantShock { town: u32, factor: f64, months: u16 },
    /// The player owes one favour to a holder of a post at the town (the
    /// lab's: a debt a player's own act would leave, such as a vote traded
    /// for support): whoever sponsors the most bills, then the lowest
    /// id. Nothing without a player.
    Favour { town: u32 },
    /// A hazard strikes the town at the next monthly hazard step.
    Disaster {
        town: u32,
        hazard: String,
        severity: f64,
    },
    /// A global recession begins now.
    Recession { depth: f64, months: u16 },
}

/// A game in progress under a driver.
pub struct Game {
    pub e: Engine,
    pub content: Arc<ContentBundle>,
    pub driver: Driver,
    pub start: SimDate,
    pub start_turn: u32,
    pub month: u32,
    pub tally: Tally,
    /// Towns whose neighbourhoods are sampled.
    pub detail: Vec<u32>,
    /// A stand-in driver's memory of its career (whether a skip just
    /// stopped; what it skipped and played). Not part of the game: a fork
    /// carries a copy.
    pub career: Option<mandate_sim::standin::Career>,
}

/// The first day of the month `k` months after `start`'s month.
pub fn month_start(start: SimDate, k: u32) -> SimDate {
    let c = start.calendar();
    let m0 = c.year * 12 + (c.month - 1) + k;
    SimDate::from_ymd(m0 / 12, m0 % 12 + 1, 1)
}

impl Game {
    pub fn new(
        content: &Arc<ContentBundle>,
        setup: &Setup,
        driver: Driver,
    ) -> Result<Self, String> {
        let e = generate(
            &WorldConfig {
                seed: setup.seed,
                size: setup.size,
                scenario: setup.scenario.clone(),
            },
            content.clone(),
        )
        .map_err(|e| e.to_string())?;
        let start = e.state().calendar.date;
        let start_turn = e.state().calendar.turn;
        let mut g = Game {
            e,
            content: content.clone(),
            driver,
            start,
            start_turn,
            month: 0,
            tally: Tally::default(),
            detail: Vec::new(),
            career: None,
        };
        g.set_driver(g.driver.clone())?;
        metrics::observe(&g.e, 0, &mut g.tally);
        // Results from the history are not the run's.
        g.tally = Tally {
            last_event: g.tally.last_event,
            last_election: g.e.state().politics.elections.last_election,
            ..Tally::default()
        };
        Ok(g)
    }

    /// The player's home town (DEC-098), or town 0.
    pub fn home(&self) -> u32 {
        mandate_sim::player::home(self.e.state())
            .map(|m| m.0)
            .unwrap_or(0)
    }

    /// An exact copy to branch from (through the save path's constructor:
    /// the state is the whole game). Call between months, never with a lab
    /// command queued.
    pub fn fork(&self) -> Result<Game, String> {
        let e = Engine::from_state(self.e.state().clone(), self.content.clone())
            .map_err(|e| e.to_string())?;
        Ok(Game {
            e,
            content: self.content.clone(),
            driver: self.driver.clone(),
            start: self.start,
            start_turn: self.start_turn,
            month: self.month,
            tally: self.tally.clone(),
            detail: self.detail.clone(),
            career: self.career.clone(),
        })
    }

    /// Change who plays from here on (a fork played by another stand-in).
    pub fn set_driver(&mut self, driver: Driver) -> Result<(), String> {
        self.career = match &driver {
            Driver::StandIn { kind } => Some(mandate_sim::standin::Career::new(
                mandate_sim::standin::Kind::parse(kind)
                    .ok_or_else(|| format!("no stand-in `{kind}`"))?,
            )),
            _ => None,
        };
        self.driver = driver;
        Ok(())
    }

    /// One turn under the driver (a stand-in that attends the stops may
    /// skip a stretch instead). `false`: no turn can end (the career is
    /// over), or the driver takes no turns.
    pub fn step_turn(&mut self) -> bool {
        match self.driver.clone() {
            Driver::None => false,
            Driver::StandIn { .. } => match self.career.as_mut() {
                Some(c) => {
                    mandate_sim::standin::step(&mut self.e, c)
                        != mandate_sim::standin::Stepped::Over
                }
                None => false,
            },
            // A bot's turn and its end (`directed`: a turn of a real skip).
            Driver::Bot { strategy } if self.player_active() => {
                crate::bots::turn(&mut self.e, &self.content, &strategy, self.start_turn).is_ok()
            }
            _ => self.e.apply(Command::EndTurn).is_ok(),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        metrics::snapshot(&self.e, &self.content, self.month, &self.detail)
    }

    fn player_active(&self) -> bool {
        self.e
            .state()
            .player
            .as_ref()
            .is_some_and(|p| p.status.active())
    }

    /// Advance to the first day of the next month under the driver.
    pub fn step_month(&mut self) {
        let target = month_start(self.start, self.month + 1);
        if self.driver == Driver::None {
            let days = self.e.state().calendar.date.days_until(target);
            if days > 0 {
                let _ = self.e.apply(Command::AdvanceDays { days });
            }
        } else {
            while self.e.state().calendar.date < target {
                if !self.step_turn() {
                    // The career ended: the world goes on without turns.
                    let days = self.e.state().calendar.date.days_until(target);
                    if days > 0 {
                        let _ = self.e.apply(Command::AdvanceDays { days });
                    }
                    break;
                }
            }
        }
        self.month += 1;
        metrics::observe(&self.e, self.month, &mut self.tally);
    }

    /// Run until `month`, sampling at each listed month on the way (and now,
    /// if listed and not yet passed).
    pub fn run_to(&mut self, month: u32, samples: &[u32], out: &mut Vec<Snapshot>) {
        if samples.contains(&self.month) && out.iter().all(|s| s.month != self.month) {
            out.push(self.snapshot());
        }
        while self.month < month {
            self.step_month();
            if samples.contains(&self.month) {
                out.push(self.snapshot());
            }
        }
    }

    /// Apply an intervention now.
    pub fn intervene(&mut self, iv: &Intervention) -> Result<(), String> {
        let date = self.e.state().calendar.date;
        let content = self.content.clone();
        let nm = self.e.state().world.municipalities.len() as u32;
        let town_ok = |t: u32| {
            if t < nm {
                Ok(())
            } else {
                Err(format!("no town {t}"))
            }
        };
        match iv {
            Intervention::Policy {
                template,
                town,
                params,
            } => {
                town_ok(*town)?;
                let t = content
                    .policies
                    .get(template.as_str())
                    .ok_or_else(|| format!("no policy template `{template}`"))?;
                let awareness = content.council.enactment_awareness as f32;
                let mut err = None;
                self.e.setup(|st| {
                    let mut p = mandate_sim::policy::probe(t, *town, params);
                    if st.policy.ids.peek_next() as usize != st.policy.instances.len() {
                        err = Some("policy ids out of step".to_string());
                        return;
                    }
                    p.id = st.policy.ids.alloc();
                    p.status = PolicyStatus::Enacted;
                    p.introduced_on = Some(date);
                    p.enacted_on = Some(date);
                    p.awareness = awareness;
                    let ev = st.events.record(
                        date,
                        EventKind::PolicyEnacted {
                            policy: p.id.0,
                            municipality: *town,
                            yes: 0,
                            no: 0,
                            overrode_veto: false,
                        },
                        Cause::Lab,
                    );
                    st.events.pin(ev);
                    p.cause = Some(ev);
                    st.policy.instances.push(p);
                });
                if let Some(e) = err {
                    return Err(e);
                }
            }
            Intervention::Set { sets } => {
                let ovs: Vec<Override> = sets
                    .iter()
                    .map(|(path, value)| Override {
                        path: path.clone(),
                        value: value.clone(),
                    })
                    .collect();
                let c = super::load(&content_root(), &ovs)?;
                self.e = Engine::from_state(self.e.state().clone(), c.clone())
                    .map_err(|e| e.to_string())?;
                self.content = c;
            }
            Intervention::SectorShock {
                town,
                sector,
                factor,
                months,
            } => {
                self.e
                    .apply(Command::LabSectorShock {
                        municipality: *town,
                        sector: sector.as_str().into(),
                        factor: *factor,
                        months: *months,
                    })
                    .map_err(|e| e.to_string())?;
            }
            Intervention::PlantClosure {
                town,
                sector,
                share,
            } => {
                town_ok(*town)?;
                use mandate_sim::economy::{industries, plants};
                let st = self.e.state();
                let mu = &st.economy.munis[*town as usize];
                let want = match sector {
                    Some(s) => Some(
                        content
                            .sector_index(s)
                            .ok_or_else(|| format!("no sector `{s}`"))?,
                    ),
                    None => None,
                };
                let closes = |k: usize| {
                    content
                        .industries_of(k)
                        .any(|(_, d)| d.plants.closes && d.plants.place_kind.is_some())
                };
                // The largest plant that can close.
                let plant = plants::of_town(&st.economy, mandate_sim::world::MunicipalityId(*town))
                    .filter(|p| p.is_open())
                    .filter(|p| {
                        content.industry(&p.industry).is_some_and(|d| {
                            d.plants.closes && want.is_none_or(|k| d.sector_index == k)
                        })
                    })
                    .max_by(|a, b| a.capacity.total_cmp(&b.capacity).then(b.id.cmp(&a.id)))
                    .map(|p| (p.id, p.industry.clone(), p.capacity));
                let share = share.clamp(0.0, 1.0);
                match plant {
                    Some((id, industry, size)) => {
                        let k = content
                            .industry_index(&industry)
                            .ok_or("the plant's industry is gone")?;
                        let sector = content.industries[k].sector_index;
                        let floor = mu.sectors[sector].capacity * share;
                        let content2 = content.clone();
                        self.e.setup(|st| {
                            // Unnamed firms of the plant's industry go with
                            // it, up to the share asked for.
                            let extra = (floor - size).max(0.0);
                            if let Some(x) = st.economy.munis[*town as usize]
                                .industries
                                .iter_mut()
                                .find(|x| x.k as usize == k)
                            {
                                let cut = extra.min(x.small);
                                if x.capacity > 0.0 {
                                    x.jobs -= x.jobs * (cut / x.capacity).min(1.0);
                                }
                                x.small -= cut;
                            }
                            industries::refresh(&mut st.economy, &content2, *town as usize);
                            if let Some(ev) = plants::close_now(st, &content2, id, Cause::Lab) {
                                st.events.pin(ev);
                            }
                        });
                    }
                    None => {
                        // No plant: unnamed firms of the largest closing
                        // sector, each industry in proportion.
                        let k = match want {
                            Some(k) => k,
                            None => (0..content.sectors.len())
                                .filter(|k| closes(*k))
                                .max_by(|a, b| {
                                    mu.sectors[*a]
                                        .capacity
                                        .total_cmp(&mu.sectors[*b].capacity)
                                        .then(b.cmp(a))
                                })
                                .ok_or("no sector whose plants close")?,
                        };
                        let key = content.sectors[k].id.clone();
                        let content2 = content.clone();
                        self.e.setup(|st| {
                            let mu = &mut st.economy.munis[*town as usize];
                            let mut jobs = 0.0;
                            if content2.is_split(k) {
                                for x in mu
                                    .industries
                                    .iter_mut()
                                    .filter(|x| content2.industries[x.k as usize].sector_index == k)
                                {
                                    let cut = (x.capacity * share).min(x.small);
                                    if x.capacity > 0.0 {
                                        x.jobs -= x.jobs * (cut / x.capacity).min(1.0);
                                    }
                                    x.small -= cut;
                                    jobs += cut;
                                }
                            } else {
                                let s = &mut mu.sectors[k];
                                jobs = (s.capacity * share).round();
                                s.capacity -= jobs;
                            }
                            industries::refresh(&mut st.economy, &content2, *town as usize);
                            let ev = st.events.record(
                                date,
                                EventKind::PlantClosure {
                                    municipality: *town,
                                    sector: key,
                                    jobs: jobs.round() as u32,
                                    industry: None,
                                    plant: None,
                                    factor: None,
                                    upstream: None,
                                },
                                Cause::Lab,
                            );
                            st.events.pin(ev);
                        });
                    }
                }
            }
            Intervention::IndustryShock {
                town,
                industry,
                factor,
                months,
            } => {
                let def = content
                    .industry(industry)
                    .ok_or_else(|| format!("no industry `{industry}`"))?;
                if let Some(t) = town {
                    town_ok(*t)?;
                }
                let k = content.industry_index(industry).unwrap_or(usize::MAX);
                let shock = mandate_sim::economy::ActiveShock {
                    sector: def.sector.clone(),
                    factor: *factor,
                    months_left: *months,
                    industry: Some(def.id.clone()),
                    demand: true,
                };
                self.e.setup(|st| {
                    for (m, mu) in st.economy.munis.iter_mut().enumerate() {
                        let here = town.is_none_or(|t| t as usize == m);
                        if here && mu.industries.iter().any(|x| x.k as usize == k) {
                            mu.shocks.push(shock.clone());
                        }
                    }
                });
            }
            Intervention::PlantShock {
                town,
                factor,
                months,
            } => {
                town_ok(*town)?;
                use mandate_sim::economy::plants;
                let st = self.e.state();
                let industry = plants::of_town(&st.economy, mandate_sim::world::MunicipalityId(*town))
                    .filter(|p| p.is_open())
                    .filter(|p| content.industry(&p.industry).is_some_and(|d| d.plants.closes))
                    .max_by(|a, b| a.capacity.total_cmp(&b.capacity).then(b.id.cmp(&a.id)))
                    .map(|p| p.industry.to_string());
                if let Some(industry) = industry {
                    self.intervene(&Intervention::IndustryShock {
                        town: Some(*town),
                        industry,
                        factor: *factor,
                        months: *months,
                    })?;
                }
            }
            Intervention::Favour { town } => {
                town_ok(*town)?;
                let st = self.e.state();
                let Some(me) = st.player.as_ref().map(|p| p.politician) else {
                    return Ok(());
                };
                let pol = &st.politics;
                let mine = pol.get(me).map(|p| p.character);
                let sponsored = |id: mandate_sim::politics::PoliticianId| {
                    st.policy
                        .instances
                        .iter()
                        .filter(|i| i.sponsor == Some(id))
                        .count()
                };
                let creditor = pol
                    .politicians
                    .iter()
                    .filter(|p| !p.is_player && p.id != me && p.municipality() == *town)
                    .filter(|p| p.role.post().is_some())
                    .max_by(|a, b| {
                        sponsored(a.id)
                            .cmp(&sponsored(b.id))
                            .then(b.id.cmp(&a.id))
                    })
                    .map(|p| p.character);
                if let (Some(me), Some(c)) = (mine, creditor) {
                    self.e.setup(|st| {
                        mandate_sim::characters::owe(&mut st.politics, me, c, "favour-lab", None, date)
                    });
                }
            }
            Intervention::Disaster {
                town,
                hazard,
                severity,
            } => {
                town_ok(*town)?;
                let kind = match hazard.as_str() {
                    "flood" => HazardKind::Flood,
                    "storm" => HazardKind::Storm,
                    "wildfire" => HazardKind::Wildfire,
                    "earthquake" => HazardKind::Earthquake,
                    h => return Err(format!("unknown hazard `{h}`")),
                };
                let max = content.director.chance.hazard_max_defer_days;
                let drawn = SimDate(date.0.saturating_sub(max + 1));
                self.e.setup(|st| {
                    st.environment.deferred.push(DeferredHazard {
                        municipality: *town,
                        kind,
                        severity: severity.clamp(0.0, 1.0),
                        drawn,
                        hold_event: None,
                    })
                });
            }
            Intervention::Recession { depth, months } => {
                self.e.setup(|st| {
                    let ev = st.events.record(
                        date,
                        EventKind::GlobalRecessionBegan {
                            depth_pct: (depth * 100.0).round() as u32,
                        },
                        Cause::Lab,
                    );
                    st.events.pin(ev);
                    st.economy.global.recession = Some(mandate_sim::economy::Recession {
                        depth: *depth,
                        months_total: *months,
                        months_elapsed: 0,
                        event: Some(ev),
                    });
                });
            }
        }
        Ok(())
    }
}

static ROOT: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// Where `Intervention::Set` reads content from (the CLI's `--content`).
pub fn set_content_root(p: &std::path::Path) {
    let _ = ROOT.set(p.to_path_buf());
}

pub fn content_root() -> std::path::PathBuf {
    ROOT.get()
        .cloned()
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
}

/// What to run.
#[derive(Clone, Debug, Serialize)]
pub struct Plan {
    pub setup: Setup,
    pub driver: Driver,
    /// Months to run.
    pub months: u32,
    /// Months at which to sample.
    pub samples: Vec<u32>,
    /// Sample the home town's neighbourhoods too.
    pub home_hoods: bool,
}

/// A run's result.
#[derive(Clone, Debug, Serialize)]
pub struct RunOut {
    pub seed: u64,
    pub scenario: Option<String>,
    pub driver: String,
    pub home: u32,
    pub intervention: Option<(u32, Intervention)>,
    pub samples: Vec<Snapshot>,
    pub tally: Tally,
    pub state_hash: String,
}

fn finish(
    g: &Game,
    plan: &Plan,
    iv: Option<(u32, Intervention)>,
    samples: Vec<Snapshot>,
) -> RunOut {
    RunOut {
        seed: plan.setup.seed,
        scenario: plan.setup.scenario.clone(),
        driver: plan.driver.name(),
        home: g.home(),
        intervention: iv,
        samples,
        tally: g.tally.clone(),
        state_hash: g.e.state_hash(),
    }
}

/// Start a plan's game (with the home town's neighbourhoods if asked).
pub fn start(content: &Arc<ContentBundle>, plan: &Plan) -> Result<Game, String> {
    let mut g = Game::new(content, &plan.setup, plan.driver.clone())?;
    if plan.home_hoods {
        g.detail = vec![g.home()];
    }
    Ok(g)
}

/// Run a plan, with an optional intervention at a month.
pub fn run(
    content: &Arc<ContentBundle>,
    plan: &Plan,
    iv: Option<(u32, Intervention)>,
) -> Result<RunOut, String> {
    let mut g = start(content, plan)?;
    let mut samples = Vec::new();
    if let Some((at, x)) = &iv {
        g.run_to(*at, &plan.samples, &mut samples);
        g.intervene(x)?;
    }
    g.run_to(plan.months, &plan.samples, &mut samples);
    Ok(finish(&g, plan, iv, samples))
}

/// Per sample month: metric path → treated − base, for paths that differ.
pub type Diff = Vec<(u32, BTreeMap<String, f64>)>;

pub fn diff(base: &[Snapshot], treated: &[Snapshot]) -> Diff {
    base.iter()
        .zip(treated)
        .map(|(a, b)| {
            let (mut fa, mut fb) = (BTreeMap::new(), BTreeMap::new());
            metrics::flatten(&serde_json::to_value(a).unwrap_or_default(), "", &mut fa);
            metrics::flatten(&serde_json::to_value(b).unwrap_or_default(), "", &mut fb);
            let keys: std::collections::BTreeSet<&String> = fa.keys().chain(fb.keys()).collect();
            let d = keys
                .into_iter()
                .filter_map(|k| {
                    let x = fb.get(k).copied().unwrap_or(0.0) - fa.get(k).copied().unwrap_or(0.0);
                    (x != 0.0).then(|| (k.clone(), metrics::r6(x)))
                })
                .collect();
            (a.month, d)
        })
        .collect()
}

#[derive(Clone, Debug, Serialize)]
pub struct TwinOut {
    pub base: RunOut,
    pub treated: Vec<RunOut>,
    /// One per treatment, in order.
    pub diff: Vec<Diff>,
}

/// Twins: one base run and one branch per intervention, all from the same
/// game up to `at` (one prefix, forked), each run to the plan's end. With
/// an empty list of interventions, `treated` holds one branch with none.
pub fn twins(
    content: &Arc<ContentBundle>,
    plan: &Plan,
    at: u32,
    ivs: &[Intervention],
) -> Result<TwinOut, String> {
    let mut g = start(content, plan)?;
    let mut prefix = Vec::new();
    g.run_to(at, &plan.samples, &mut prefix);
    let branch = |x: Option<&Intervention>| -> Result<RunOut, String> {
        let mut b = g.fork()?;
        let mut samples = prefix.clone();
        if let Some(x) = x {
            b.intervene(x)?;
        }
        b.run_to(plan.months, &plan.samples, &mut samples);
        Ok(finish(&b, plan, x.map(|x| (at, x.clone())), samples))
    };
    let base = branch(None)?;
    let treated: Vec<RunOut> = if ivs.is_empty() {
        vec![branch(None)?]
    } else {
        ivs.iter()
            .map(|x| branch(Some(x)))
            .collect::<Result<_, _>>()?
    };
    let diff = treated
        .iter()
        .map(|t| diff(&base.samples, &t.samples))
        .collect();
    Ok(TwinOut {
        base,
        treated,
        diff,
    })
}

/// Map `f` over `items` on up to `threads` threads (0: all cores); the
/// output is in input order whatever the scheduling (DEC-041 rule 1 allows
/// threads between independent runs).
pub fn par_map<T: Sync, R: Send>(
    items: &[T],
    threads: usize,
    f: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
    if items.is_empty() {
        return Vec::new();
    }
    let n = if threads == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(2)
    } else {
        threads
    }
    .clamp(1, items.len());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let mut out: Vec<(usize, R)> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..n)
            .map(|_| {
                sc.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if i >= items.len() {
                            break;
                        }
                        mine.push((i, f(&items[i])));
                    }
                    mine
                })
            })
            .collect();
        hs.into_iter()
            .flat_map(|h| h.join().unwrap_or_default())
            .collect()
    });
    out.sort_by_key(|x| x.0);
    out.into_iter().map(|x| x.1).collect()
}

/// The number of threads lab commands use (`MANDATE_LAB_THREADS`, default
/// all cores).
pub fn threads() -> usize {
    std::env::var("MANDATE_LAB_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content() -> Arc<ContentBundle> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        super::super::load(&root, &[]).unwrap_or_else(|e| panic!("{e}"))
    }

    fn plan(driver: Driver) -> Plan {
        Plan {
            setup: Setup::small(3, None),
            driver,
            months: 14,
            samples: vec![0, 6, 14],
            home_hoods: true,
        }
    }

    #[test]
    fn a_fork_continues_exactly_as_the_original() {
        let c = content();
        let p = plan(Driver::Idle);
        let mut a = start(&c, &p).unwrap_or_else(|e| panic!("{e}"));
        a.run_to(3, &[], &mut Vec::new());
        let mut b = a.fork().unwrap_or_else(|e| panic!("{e}"));
        a.run_to(9, &[], &mut Vec::new());
        b.run_to(9, &[], &mut Vec::new());
        assert_eq!(a.e.state_hash(), b.e.state_hash());
        assert_eq!(a.tally, b.tally);
    }

    #[test]
    fn a_twin_without_an_intervention_differs_in_nothing() {
        let c = content();
        let t = twins(&c, &plan(Driver::Idle), 2, &[]).unwrap_or_else(|e| panic!("{e}"));
        assert!(t.diff[0].iter().all(|(_, d)| d.is_empty()), "{:?}", t.diff);
        assert_eq!(t.base.state_hash, t.treated[0].state_hash);
        // And a straight run (no fork) matches the forked base.
        let r = run(&c, &plan(Driver::Idle), None).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(r.state_hash, t.base.state_hash);
        assert_eq!(r.samples, t.base.samples);
    }

    #[test]
    fn the_same_experiment_twice_gives_identical_json() {
        let c = content();
        let p = plan(Driver::Bot {
            strategy: "mixed".into(),
        });
        let iv = Intervention::Policy {
            template: "policy.hardship_fund".into(),
            town: 0,
            params: BTreeMap::new(),
        };
        let go = || {
            let t = twins(&c, &p, 1, std::slice::from_ref(&iv)).unwrap_or_else(|e| panic!("{e}"));
            serde_json::to_string(&t).unwrap_or_default()
        };
        let a = go();
        assert_eq!(a, go());
        // Parallel runs come back in input order, identical to serial ones.
        let seeds = [1u64, 2, 3];
        let one = |s: &u64| {
            let mut p = plan(Driver::None);
            p.setup.seed = *s;
            p.months = 6;
            run(&c, &p, None).map(|r| r.state_hash).unwrap_or_default()
        };
        assert_eq!(par_map(&seeds, 3, one), par_map(&seeds, 1, one));
    }

    #[test]
    fn interventions_move_what_they_touch() {
        let c = content();
        let p = Plan {
            months: 8,
            samples: vec![8],
            ..plan(Driver::None)
        };
        let home = start(&c, &p).map(|g| g.home()).unwrap_or(0);
        let ivs = [
            Intervention::Policy {
                template: "policy.affordable_housing".into(),
                town: home,
                params: BTreeMap::new(),
            },
            Intervention::PlantClosure {
                town: home,
                sector: None,
                share: 0.5,
            },
            Intervention::Recession {
                depth: 0.08,
                months: 12,
            },
            Intervention::Disaster {
                town: home,
                hazard: "flood".into(),
                severity: 0.9,
            },
            Intervention::SectorShock {
                town: home,
                sector: "sector.retail".into(),
                factor: 0.5,
                months: 6,
            },
            Intervention::Set {
                sets: vec![("economy.migration_rate".into(), serde_json::json!(0.0))],
            },
        ];
        let t = twins(&c, &p, 1, &ivs).unwrap_or_else(|e| panic!("{e}"));
        for (k, d) in t.diff.iter().enumerate() {
            assert!(!d[0].1.is_empty(), "intervention {k} changed nothing");
        }
        let kinds: Vec<&str> = t.treated[1]
            .tally
            .events
            .iter()
            .map(|e| e.kind.as_str())
            .collect();
        assert!(kinds.contains(&"plant_closure"), "{kinds:?}");
        assert!(
            t.treated[3]
                .tally
                .events
                .iter()
                .any(|e| e.kind == "disaster" && e.town == Some(home)),
            "the released hazard struck"
        );
    }
}
