//! The lab's metric set: a [`Snapshot`] of the nation and every town (and,
//! on request, their neighbourhoods) at one date, and a [`Tally`] of what
//! happened between samples (elections, events, crises). Everything is read
//! from the state; nothing here writes it or draws from a stream.

use mandate_content::ContentBundle;
use mandate_sim::Engine;
use mandate_sim::events::{EventId, EventKind};
use mandate_sim::politics::Post;
use mandate_sim::world::{Jurisdiction, MunicipalityId};
use serde::Serialize;
use std::collections::BTreeMap;

/// Round to 6 significant decimals so JSON stays short (the values are
/// deterministic either way).
pub fn r6(x: f64) -> f64 {
    if !x.is_finite() {
        return 0.0;
    }
    (x * 1e6).round() / 1e6
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Nation {
    pub unemployment: f64,
    pub inflation: f64,
    pub price_level: f64,
    pub gdp: f64,
    pub mean_wage: f64,
    pub policy_rate: f64,
    pub population: u64,
    pub debt_pct_gdp: f64,
    pub in_recession: bool,
    pub global_demand: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Hood {
    pub id: u32,
    pub population: u64,
    /// Worry levels as residents feel them (concern registry order, by key).
    pub concerns: BTreeMap<String, f64>,
    /// Residents' approval of the town government (0..1), count-weighted.
    pub gov_approval: f64,
    /// Residents' approval of the player (−1..1) and awareness (0..1).
    pub player_approval: f64,
    pub player_awareness: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Town {
    pub id: u32,
    pub class: String,
    pub population: u64,
    pub labour_force: f64,
    pub unemployment: f64,
    pub mean_wage: f64,
    /// Mean wage over the national price level.
    pub real_wage: f64,
    pub rent: f64,
    /// Yearly rent over the middle class's income.
    pub rent_burden: f64,
    /// Income per adult by class (low, middle, high), nominal.
    pub income: [f64; 3],
    pub vacancy: f64,
    /// Housing starts over the town's baseline starts (1 = normal building).
    pub starts_index: f64,
    pub housing_stock: f64,
    pub output: f64,
    pub jobs: BTreeMap<String, f64>,
    pub attractiveness: f64,
    /// Residents' approval of the town government (0..1), count-weighted.
    pub gov_approval: f64,
    /// Worry levels as felt (by concern key) and as measured town-wide.
    pub concerns: BTreeMap<String, f64>,
    pub measured: BTreeMap<String, f64>,
    /// The town's treasury, in currency units.
    pub cash: f64,
    pub debt: f64,
    pub crises: Vec<String>,
    /// The party of the town's executive (constitution id), if any.
    pub executive_party: Option<u32>,
    /// Neighbourhoods: only for the towns a run asks for (`detail`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hoods: Vec<Hood>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct PlayerSnap {
    pub home: u32,
    pub ward: u32,
    pub active: bool,
    pub in_office: bool,
    pub record: f64,
    /// Count-weighted approval and awareness of the player in their ward.
    pub ward_approval: f64,
    pub ward_awareness: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Snapshot {
    /// Months since the run's start.
    pub month: u32,
    pub date: String,
    pub nation: Nation,
    pub towns: Vec<Town>,
    pub player: Option<PlayerSnap>,
}

/// One race decided during a run.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Election {
    pub month: u32,
    pub date: String,
    /// The town (a local race), if any.
    pub town: Option<u32>,
    /// The constitution id of the body or office.
    pub post: String,
    pub office: bool,
    pub constituency: Option<u32>,
    pub incumbent: Option<u32>,
    pub incumbent_ran: bool,
    pub incumbent_won: bool,
    pub incumbent_party: Option<u32>,
    pub winner: u32,
    pub winner_party: Option<u32>,
    /// The candidate with the second-most votes.
    pub runner_up: Option<u32>,
    pub turnout: f64,
    /// Winner's share minus the runner-up's.
    pub margin: f64,
    pub player_ran: bool,
    pub player_won: bool,
    /// The engine's factor decomposition of the winner's margin
    /// (election-night "Why it went this way").
    pub decomposition: BTreeMap<String, f64>,
}

/// An event of interest: closures, disasters, recessions, laws.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct EventRec {
    pub month: u32,
    pub kind: String,
    pub town: Option<u32>,
    pub detail: String,
    /// Events this one names as its causes, by kind (the causal spine).
    pub causes: Vec<String>,
}

/// A town crisis seen at monthly resolution.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct CrisisSpan {
    pub town: u32,
    pub kind: String,
    pub begun: u32,
    pub ended: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Tally {
    pub elections: Vec<Election>,
    pub events: Vec<EventRec>,
    pub crises: Vec<CrisisSpan>,
    /// The last event seen (not serialized: bookkeeping).
    #[serde(skip)]
    pub last_event: Option<EventId>,
    #[serde(skip)]
    pub last_election: Option<mandate_core::SimDate>,
    #[serde(skip)]
    pub seen_results: usize,
}

pub fn date_str(d: mandate_core::SimDate) -> String {
    let c = d.calendar();
    format!("{:04}-{:02}-{:02}", c.year, c.month, c.day)
}

fn weighted<'a>(
    cohorts: impl Iterator<Item = &'a mandate_sim::population::Cohort>,
    f: impl Fn(&mandate_sim::population::Cohort) -> f64,
) -> f64 {
    let (mut acc, mut n) = (0.0, 0.0);
    for c in cohorts {
        acc += f(c) * c.count as f64;
        n += c.count as f64;
    }
    if n > 0.0 { acc / n } else { 0.0 }
}

fn concern_map(keys: &[String], levels: &[f32]) -> BTreeMap<String, f64> {
    keys.iter()
        .zip(levels)
        .map(|(k, v)| (k.trim_start_matches("concern.").to_string(), r6(*v as f64)))
        .collect()
}

fn sector_short(k: &str) -> String {
    k.trim_start_matches("sector.").to_string()
}

/// A snapshot of the state now; `detail` lists towns whose neighbourhoods
/// are included.
pub fn snapshot(e: &Engine, content: &ContentBundle, month: u32, detail: &[u32]) -> Snapshot {
    let st = e.state();
    let ec = &st.economy;
    let nat = &ec.national;
    let gdp = nat.gdp.max(1.0);
    let debt = st
        .finance
        .national
        .as_ref()
        .map(|t| t.debt.units() as f64 / gdp)
        .unwrap_or(0.0);
    let nation = Nation {
        unemployment: r6(nat.unemployment),
        inflation: r6(nat.inflation),
        price_level: r6(nat.price_level),
        gdp: r6(nat.gdp),
        mean_wage: r6(nat.mean_wage),
        policy_rate: r6(nat.policy_rate),
        population: st.population.total(),
        debt_pct_gdp: r6(debt),
        in_recession: ec.global.recession.is_some(),
        global_demand: r6(ec.global.demand),
    };
    let keys = &st.population.concern_keys;
    let constitution = content.constitutions.get(&st.meta.constitution);
    let towns = st
        .world
        .municipalities
        .iter()
        .enumerate()
        .map(|(m, mu)| {
            let Some(me) = ec.munis.get(m) else {
                return Town::default();
            };
            let mid = MunicipalityId(m as u32);
            let hoods_c = mu
                .neighborhoods
                .iter()
                .flat_map(|h| st.population.hoods[h.index()].cohorts.iter());
            let gov = weighted(hoods_c, |c| c.gov_approval as f64);
            let tc = st.population.town_concerns.get(m);
            let tr = st.finance.local.get(m);
            let exec = constitution
                .and_then(|c| {
                    mandate_sim::politics::executive_of(&st.politics, c, Jurisdiction::town(mid))
                })
                .and_then(|id| st.politics.get(id))
                .and_then(|p| p.party)
                .map(|p| p.0);
            let hoods = if detail.contains(&(m as u32)) {
                mu.neighborhoods
                    .iter()
                    .map(|h| {
                        let hp = &st.population.hoods[h.index()];
                        Hood {
                            id: h.0,
                            population: hp.total(),
                            concerns: st
                                .population
                                .hood_concerns
                                .get(h.index())
                                .map(|c| concern_map(keys, &c.levels))
                                .unwrap_or_default(),
                            gov_approval: r6(weighted(hp.cohorts.iter(), |c| {
                                c.gov_approval as f64
                            })),
                            player_approval: r6(weighted(hp.cohorts.iter(), |c| {
                                c.player_approval as f64
                            })),
                            player_awareness: r6(weighted(hp.cohorts.iter(), |c| {
                                c.player_awareness as f64
                            })),
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };
            Town {
                id: m as u32,
                class: format!("{:?}", mu.class).to_lowercase(),
                population: st.population.municipality_total(&st.world, mid),
                labour_force: r6(me.labour_force),
                unemployment: r6(me.unemployment),
                mean_wage: r6(me.mean_wage),
                real_wage: r6(me.mean_wage / nat.price_level.max(1e-6)),
                rent: r6(me.housing.rent),
                rent_burden: r6(me.housing.rent * 12.0 / me.class_income[1].max(1.0)),
                income: me.class_income.map(r6),
                vacancy: r6(me.housing.vacancy),
                starts_index: r6(mandate_sim::economy::housing::starts_index(&me.housing)),
                housing_stock: r6(me.housing.stock),
                output: r6(me.output),
                jobs: ec
                    .sector_keys
                    .iter()
                    .zip(&me.sectors)
                    .map(|(k, s)| (sector_short(k), r6(s.jobs)))
                    .collect(),
                attractiveness: r6(me.attractiveness),
                gov_approval: r6(gov),
                concerns: tc.map(|t| concern_map(keys, &t.levels)).unwrap_or_default(),
                measured: tc
                    .map(|t| concern_map(keys, &t.measured))
                    .unwrap_or_default(),
                cash: tr.map(|t| t.cash.units() as f64).unwrap_or(0.0),
                debt: tr.map(|t| t.debt.units() as f64).unwrap_or(0.0),
                crises: st
                    .politics
                    .characters
                    .towns
                    .get(m)
                    .map(|t| t.crises.iter().map(|c| c.kind.to_string()).collect())
                    .unwrap_or_default(),
                executive_party: exec,
                hoods,
            }
        })
        .collect();
    let player = st.player.as_ref().map(|p| {
        let ward_hoods = mandate_sim::policy::ward_hoods(&st.world, p.ward);
        let cohorts = || {
            ward_hoods
                .iter()
                .flat_map(|h| st.population.hoods[*h].cohorts.iter())
        };
        let pol = st.politics.get(p.politician);
        PlayerSnap {
            home: p.home().0,
            ward: p.ward.0,
            active: p.status.active(),
            in_office: pol.is_some_and(|x| x.role.in_office()),
            record: r6(pol.map(|x| x.record as f64).unwrap_or(0.0)),
            ward_approval: r6(weighted(cohorts(), |c| c.player_approval as f64)),
            ward_awareness: r6(weighted(cohorts(), |c| c.player_awareness as f64)),
        }
    });
    Snapshot {
        month,
        date: date_str(st.calendar.date),
        nation,
        towns,
        player,
    }
}

fn town_of_event(k: &EventKind) -> Option<(String, Option<u32>, String)> {
    Some(match k {
        EventKind::PlantClosure {
            municipality,
            sector,
            jobs,
        } => (
            "plant_closure".into(),
            Some(*municipality),
            format!("{sector}:{jobs}"),
        ),
        EventKind::Disaster {
            municipality,
            hazard,
            severity_pct,
        } => (
            "disaster".into(),
            Some(*municipality),
            format!("{hazard:?}:{severity_pct}"),
        ),
        EventKind::GlobalRecessionBegan { depth_pct } => {
            ("recession".into(), None, depth_pct.to_string())
        }
        EventKind::GlobalRecoveryBegan => ("recovery".into(), None, String::new()),
        EventKind::PolicyEnacted {
            policy,
            municipality,
            ..
        } => (
            "policy_enacted".into(),
            Some(*municipality),
            policy.to_string(),
        ),
        EventKind::ScandalBroke { .. } => ("scandal".into(), None, String::new()),
        _ => return None,
    })
}

fn kind_name(k: &EventKind) -> String {
    let s = format!("{k:?}");
    s.split([' ', '{', '(']).next().unwrap_or("").to_string()
}

/// Fold what happened since the last call into the tally (call monthly).
pub fn observe(e: &Engine, month: u32, t: &mut Tally) {
    let st = e.state();
    // Events.
    let last = t.last_event;
    for ev in st.events.iter().filter(|x| last.is_none_or(|l| x.id > l)) {
        if let Some((kind, town, detail)) = town_of_event(&ev.kind) {
            let causes = ev
                .causes
                .iter()
                .filter_map(|c| st.events.get(*c))
                .map(|c| kind_name(&c.kind))
                .collect();
            t.events.push(EventRec {
                month,
                kind,
                town,
                detail,
                causes,
            });
        }
    }
    t.last_event = st.events.iter().last().map(|x| x.id).or(t.last_event);
    // Crises: open spans for new ones, close spans that ended.
    for (m, town) in st.politics.characters.towns.iter().enumerate() {
        let m = m as u32;
        for c in &town.crises {
            let k = c.kind.to_string();
            if !t
                .crises
                .iter()
                .any(|s| s.town == m && s.kind == k && s.ended.is_none())
            {
                t.crises.push(CrisisSpan {
                    town: m,
                    kind: k,
                    begun: month,
                    ended: None,
                });
            }
        }
        for s in t
            .crises
            .iter_mut()
            .filter(|s| s.town == m && s.ended.is_none())
        {
            if !town.crises.iter().any(|c| c.kind.as_str() == s.kind) {
                s.ended = Some(month);
            }
        }
    }
    // Elections: results dated after the last one seen.
    let player = st.player.as_ref().map(|p| p.politician);
    let since = t.last_election;
    let mut newest = since;
    for r in st
        .politics
        .elections
        .results
        .iter()
        .filter(|r| since.is_none_or(|d| r.date > d))
    {
        newest = Some(newest.map_or(r.date, |d: mandate_core::SimDate| d.max(r.date)));
        let mut shares: Vec<f64> = r.candidates.iter().map(|c| c.share as f64).collect();
        shares.sort_by(|a, b| b.total_cmp(a));
        let runner_up = r
            .candidates
            .iter()
            .filter(|c| c.politician != r.winner)
            .max_by(|a, b| a.share.total_cmp(&b.share))
            .map(|c| c.politician.0);
        let party_of = |id: mandate_sim::politics::PoliticianId| {
            r.candidates
                .iter()
                .find(|c| c.politician == id)
                .and_then(|c| c.party)
                .map(|p| p.0)
        };
        let post: Post = r.seat;
        t.elections.push(Election {
            month,
            date: date_str(r.date),
            town: post.municipality(),
            post: post.id().to_string(),
            office: post.is_office(),
            constituency: post.constituency().map(|c| c.0),
            incumbent: r.incumbent.map(|p| p.0),
            incumbent_ran: r
                .incumbent
                .is_some_and(|i| r.candidates.iter().any(|c| c.politician == i)),
            incumbent_won: r.incumbent == Some(r.winner),
            incumbent_party: r.incumbent.and_then(|i| {
                party_of(i).or_else(|| st.politics.get(i).and_then(|p| p.party).map(|p| p.0))
            }),
            winner: r.winner.0,
            winner_party: party_of(r.winner),
            runner_up,
            turnout: r6(r.turnout as f64),
            margin: r6(
                shares.first().copied().unwrap_or(0.0) - shares.get(1).copied().unwrap_or(0.0)
            ),
            player_ran: player.is_some_and(|p| r.candidates.iter().any(|c| c.politician == p)),
            player_won: player == Some(r.winner),
            decomposition: r
                .decomposition
                .iter()
                .map(|f| (f.factor.clone(), r6(f.margin as f64)))
                .collect(),
        });
    }
    t.last_election = newest;
}

/// Every numeric leaf of a JSON value, by path (`towns.3.unemployment`).
pub fn flatten(v: &serde_json::Value, prefix: &str, out: &mut BTreeMap<String, f64>) {
    use serde_json::Value as V;
    let key = |k: &str| {
        if prefix.is_empty() {
            k.to_string()
        } else {
            format!("{prefix}.{k}")
        }
    };
    match v {
        V::Number(n) => {
            out.insert(prefix.to_string(), n.as_f64().unwrap_or(0.0));
        }
        V::Bool(b) => {
            out.insert(prefix.to_string(), if *b { 1.0 } else { 0.0 });
        }
        V::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                flatten(x, &key(&i.to_string()), out);
            }
            if a.iter().all(|x| x.is_string()) {
                out.insert(key("len"), a.len() as f64);
            }
        }
        V::Object(o) => {
            for (k, x) in o {
                flatten(x, &key(k), out);
            }
        }
        V::String(_) | V::Null => {}
    }
}
