//! `mandate-cli arcs grade` (STAGED_ARCS §6.9): the arc scorer over
//! generated worlds.
//!
//! For each arc template and seed: find a world that raises it at the
//! player's home (the lab plants its cause where an `Intervention` can: a
//! disaster, a plant's closure; otherwise the world is given
//! `--wait-months` to raise it by its own calendar), fork the game the
//! turn it is raised, and let each stand-in (`mandate_sim::standin`:
//! passive, engaged, contrarian) play the fork on. A `Watch`
//! (`mandate_sim::arcs::score`) follows the run; the five terms and
//! coverage are computed from the traces.
//!
//! **Measuring, not judging.** Every figure is printed with the bar the
//! design intends for it (§6.9) as data beside it, and none fails the
//! command. The one enforced check is structural: coverage. From
//! [`COVERAGE_SEEDS`] seeds up, a stage or exit no stand-in visited fails
//! the command unless [`UNVISITED`] lists it with its reason.
//!
//! The same seeds give the same output: games are independent and
//! deterministic, and rows come back in seed order whatever the threads.

use crate::lab::runner::{self, Driver, Game, Intervention, Setup};
use mandate_content::arcs::ArcDef;
use mandate_content::{Condition, ContentBundle};
use mandate_sim::arcs::ArcCastEntry;
use mandate_sim::arcs::score::{
    self, Agency, Coverage, Escalation, Opening, Pacing, Trace, Vector, Visibility, Watch,
};
use mandate_sim::politics::Post;
use mandate_sim::standin::{Kind, Seat};
use mandate_sim::world::Jurisdiction;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(clap::Args, Clone, Debug)]
pub struct GradeArgs {
    /// Worlds per template (seeds 1..=N).
    #[arg(long, default_value_t = 16)]
    pub seeds: u64,
    /// One template (`arc.flood`); omitted: every template.
    #[arg(long)]
    pub arc: Option<String>,
    /// Months a world is given to raise the arc by itself.
    #[arg(long, default_value_t = 14)]
    pub wait_months: u32,
    /// Weeks a fork is played at most (the terms are measured at week 39;
    /// play goes on so that a run reaches its exit and coverage counts it:
    /// a stage may last two years).
    #[arg(long, default_value_t = 130)]
    pub max_weeks: u32,
    /// Also play the bots `idle`, `mixed` and `explorer` on every fork (a
    /// check on the stand-ins, §6.3; their rows are reported apart).
    #[arg(long)]
    pub bots: bool,
    /// Print a table instead of JSON.
    #[arg(long)]
    pub text: bool,
}

impl Default for GradeArgs {
    fn default() -> Self {
        GradeArgs {
            seeds: 16,
            arc: None,
            wait_months: 14,
            max_weeks: 130,
            bots: false,
            text: false,
        }
    }
}

/// The bars the design intends (§6.9), as data: reported beside the
/// figures, enforced by nothing. The tuning pass at the end of M6 decides
/// which become gates (`docs/development/TUNING_QUEUE.md`).
pub const EXITS_DIFFER_MIN: f64 = 0.6;
pub const MEDIAN_AGENCY_MIN: f64 = 0.4;
pub const ENGAGED_EXIT_SHARE_MAX: f64 = 0.7;
/// The floors a candidate opening is dropped under (§6.5).
pub const AGENCY_FLOOR: f64 = 0.3;
pub const VISIBILITY_FLOOR: f64 = 0.4;

/// Coverage is a claim about many worlds: it is enforced from this many
/// seeds up, and only printed under it.
pub const COVERAGE_SEEDS: u64 = 16;

/// Stages and exits no stand-in visits yet, each with its reason (what the
/// engine or the content needs before one can, as measured on 16 seeds on
/// 2026-11-25): listed, so that coverage can be enforced for everything
/// else. `(arc, stage, reason)`; a stage of `*` is every stage of the
/// template.
pub const UNVISITED: &[(&str, &str, &str)] = &[
    (
        "arc.budget_round",
        "turn_hole",
        "needs a treasury that closed its last year at least 1% of revenue short; every town graded closed over (the balanced-budget rule, STAGED_ARCS §13), so the hole comes only once a town's finances turn",
    ),
    (
        "arc.budget_round",
        "late",
        "needs the deal turn and heat of 0.6 on the day of the vote; the deal turn came in 5 of 16 worlds and the heat peaked at 0.58 (the contrarian's `whip` adds 0.15 and still falls short). The content's numbers: the tuning pass, or A6",
    ),
    (
        "arc.rival",
        "cooled",
        "needs a campaign that ends with the player's race undecided (a race called off, a rival who withdrew): no generated world had one, and no command of a player's makes one",
    ),
    (
        "arc.favour_debt",
        "*",
        "its cast needs someone the player owes a favour; a debt comes from an act of the player's that no stand-in takes (a traded favour), and the lab has no intervention that plants one",
    ),
];

/// The listed reason a stage or exit of a template goes unvisited.
pub fn unvisited_reason(arc: &str, stage: &str) -> Option<&'static str> {
    UNVISITED
        .iter()
        .find(|x| x.0 == arc && (x.1 == stage || x.1 == "*"))
        .map(|x| x.2)
}

/// How hard the lab's planted causes are (the lab's numbers, not the
/// game's): a disaster's severity, and the share of the town's largest
/// industrial sector a planted closure takes, by seed, so that the worlds
/// graded hold a small closure, a middling one and a grave one.
pub const PLANT_SEVERITY: f64 = 0.9;
pub const PLANT_SHARES: [f64; 3] = [0.15, 0.3, 0.5];
/// A planted fall in demand for a plant's industry (DEC-110), by seed: a
/// mild one (short time, and hours back when it passes), a middling one (a
/// notice a rescue can lift) and a grave one (a notice nothing short of the
/// market lifts); and how long it lasts, so a notice falls inside it.
pub const PLANT_FACTORS: [f64; 3] = [0.85, 0.72, 0.6];
pub const PLANT_SHOCK_MONTHS: u16 = 30;

/// The lab's intervention that plants a template's cause in a town, if
/// its `raise` reads one the lab can plant, or its cast needs a debt of
/// the player's (a favour owed: the act of a player's that leaves one).
pub fn plant(def: &ArcDef, town: u32, seed: u64) -> Option<Intervention> {
    if def
        .roles
        .iter()
        .any(|r| !r.optional && matches!(r.cast, mandate_content::arcs::ArcCast::Creditor))
    {
        return Some(Intervention::Favour { town });
    }
    def.raise.iter().find_map(|c| match c {
        Condition::Plant { slot: None, .. } => Some(Intervention::PlantShock {
            town,
            factor: PLANT_FACTORS[(seed % PLANT_FACTORS.len() as u64) as usize],
            months: PLANT_SHOCK_MONTHS,
        }),
        Condition::Disaster { hazard, .. } => Some(Intervention::Disaster {
            town,
            hazard: hazard
                .map(|h| format!("{h:?}").to_lowercase())
                .unwrap_or_else(|| "flood".into()),
            severity: PLANT_SEVERITY,
        }),
        Condition::Crisis { kind: Some(k), .. } if k == "crisis.plant_closure" => {
            Some(Intervention::PlantClosure {
                town,
                sector: None,
                share: PLANT_SHARES[(seed % PLANT_SHARES.len() as u64) as usize],
            })
        }
        _ => None,
    })
}

/// A world with the template's run just raised: the game to fork.
pub struct Raised {
    /// `raised` (the world's own doing) or `planted` (the lab's).
    pub cause: &'static str,
    pub game: Game,
    pub at: Jurisdiction,
    pub post: Option<Post>,
    pub cast: Vec<ArcCastEntry>,
    pub opening: Opening,
    /// Turns from the start of play to the raise.
    pub waited: u32,
}

/// A world that did not raise the template in time.
#[derive(Clone, Debug)]
pub struct Missed {
    pub seed: u64,
    pub planted: bool,
    /// `never_raised` (its `raise` never held), `cast_fails` (it held and
    /// no cast could be bound), `held_back` (it held with a cast: the cap
    /// on runs, a cooldown, `given`, or an engine feature it waits for),
    /// or an error.
    pub reason: String,
}

/// Find a world's raise of a template at the player's home, under idle
/// play.
pub fn find(
    content: &Arc<ContentBundle>,
    def: &ArcDef,
    seed: u64,
    wait_months: u32,
) -> Result<Raised, Missed> {
    let miss = |planted: bool, reason: String| Missed {
        seed,
        planted,
        reason,
    };
    let mut g =
        Game::new(content, &Setup::small(seed, None), Driver::Idle).map_err(|e| miss(false, e))?;
    let at = Jurisdiction::town(mandate_sim::world::MunicipalityId(g.home()));
    let planted = match plant(def, g.home(), seed) {
        Some(iv) => {
            g.intervene(&iv).map_err(|e| miss(true, e))?;
            true
        }
        None => false,
    };
    let until = runner::month_start(g.start, wait_months);
    let (mut held, mut castable) = (false, false);
    let mut waited = 0;
    loop {
        let st = g.e.state();
        let cast = st
            .arcs
            .at(at)
            .find(|r| r.template == def.id)
            .map(|r| r.cast.clone());
        if let Some(cast) = cast {
            let post = Seat::of_player(st, content).and_then(|s| s.post);
            let opening = score::opening(st, content, def, at, post);
            return Ok(Raised {
                cause: if planted { "planted" } else { "raised" },
                at,
                post,
                cast,
                opening,
                waited,
                game: g,
            });
        }
        let p = score::probe(st, content, def, at);
        held |= p.raise;
        castable |= p.cast;
        if st.calendar.date >= until || !g.step_turn() {
            break;
        }
        waited += 1;
    }
    Err(miss(
        planted,
        match (held, castable) {
            (false, _) => "never_raised",
            (true, false) => "cast_fails",
            (true, true) => "held_back",
        }
        .into(),
    ))
}

/// One player's play of a fork.
#[derive(Clone, Debug)]
pub struct Played {
    pub who: String,
    pub trace: Trace,
    /// The world at the horizon (week 39), and the turns played to it.
    pub vector: Vector,
    pub horizon_turns: u32,
    pub state_hash: String,
}

/// Play a fork of a raised world with a driver, watching the run.
pub fn play(
    w: &Raised,
    content: &Arc<ContentBundle>,
    template: &str,
    driver: Driver,
    max_weeks: u32,
) -> Result<Played, String> {
    let mut g = w.game.fork()?;
    let who = match &driver {
        Driver::StandIn { kind } => kind.clone(),
        d => format!("bot:{}", d.name()),
    };
    g.set_driver(driver)?;
    if let Some(c) = g.career.as_mut() {
        // The stand-in plays for the seat as the run was raised.
        c.seat = Some(Seat {
            at: w.at,
            post: w.post,
        });
    }
    let start = g.e.state().calendar.date;
    let turn0 = g.e.state().calendar.turn;
    let horizon = score::HORIZON_WEEKS * 7;
    let limit = max_weeks.max(score::HORIZON_WEEKS) * 7;
    let mut watch = Watch::new(g.e.state(), template, w.at);
    watch.observe(g.e.state(), content);
    let mut at_horizon: Option<(Vector, u32)> = None;
    loop {
        let st = g.e.state();
        let days = start.days_until(st.calendar.date);
        if at_horizon.is_none() && days >= horizon {
            at_horizon = Some((
                score::vector(st, content, w.at, &w.cast, w.post),
                st.calendar.turn - turn0,
            ));
        }
        if (watch.ended() && at_horizon.is_some()) || days >= limit || !g.step_turn() {
            break;
        }
        watch.observe(g.e.state(), content);
    }
    let st = g.e.state();
    let (vector, horizon_turns) = at_horizon.unwrap_or_else(|| {
        (
            score::vector(st, content, w.at, &w.cast, w.post),
            st.calendar.turn - turn0,
        )
    });
    Ok(Played {
        who,
        trace: watch.trace,
        vector,
        horizon_turns,
        state_hash: g.e.state_hash(),
    })
}

/// One player's runs of a template: E, V and P of each, the traces, and
/// what it alone covered.
type PerPlayer = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<Trace>, Coverage);

/// One world graded.
pub struct WorldGrade {
    pub seed: u64,
    pub cause: &'static str,
    pub waited: u32,
    pub opening: Opening,
    /// Passive, engaged, contrarian, in that order; then the bots, if
    /// asked for.
    pub plays: Vec<Played>,
}

impl WorldGrade {
    pub fn of(&self, kind: Kind) -> Option<&Played> {
        self.plays.iter().find(|p| p.who == kind.key())
    }
}

/// The bots that check the stand-ins (§6.3).
pub const CHECK_BOTS: [&str; 3] = ["idle", "mixed", "explorer"];

/// Grade one template in one world.
pub fn grade_world(
    content: &Arc<ContentBundle>,
    def: &ArcDef,
    seed: u64,
    args: &GradeArgs,
) -> Result<WorldGrade, Missed> {
    let w = find(content, def, seed, args.wait_months)?;
    let mut drivers: Vec<Driver> = Kind::ARC.iter().map(|k| Driver::standin(*k)).collect();
    if args.bots {
        drivers.extend(CHECK_BOTS.iter().map(|b| Driver::parse(b)));
    }
    let mut plays = Vec::new();
    for d in drivers {
        plays.push(
            play(&w, content, &def.id, d, args.max_weeks).map_err(|e| Missed {
                seed,
                planted: w.cause == "planted",
                reason: e,
            })?,
        );
    }
    Ok(WorldGrade {
        seed,
        cause: w.cause,
        waited: w.waited,
        opening: w.opening,
        plays,
    })
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    Some(if n.is_multiple_of(2) {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    } else {
        v[n / 2]
    })
}

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

fn spread(v: Vec<f64>) -> Value {
    let lo = v.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    match median(v) {
        Some(m) => json!({ "median": r3(m), "min": r3(lo), "max": r3(hi) }),
        None => Value::Null,
    }
}

/// A report-only bar beside its figure.
fn bar(name: &str, target: &str, measured: Option<f64>, pass: Option<bool>) -> Value {
    json!({ "gate": name, "target": target, "measured": measured.map(r3), "pass": pass, "enforced": false })
}

/// A template's grade: what the worlds measured, as JSON.
pub struct ArcGrade {
    pub arc: String,
    pub report: Value,
    pub coverage: Coverage,
    /// Unvisited stages and exits with no listed reason.
    pub unexplained: Vec<String>,
    pub missed: Vec<Missed>,
    pub found: usize,
    /// The engaged run's median turns to the turn, and the turns in a
    /// chapter (the doctor's `arc-too-long`).
    pub engaged_turns_to_turn: Option<f64>,
    pub chapter_turns: f64,
}

/// Grade one template over seeds 1..=N.
pub fn grade(content: &Arc<ContentBundle>, def: &ArcDef, args: &GradeArgs) -> ArcGrade {
    let seeds: Vec<u64> = (1..=args.seeds).collect();
    let results = runner::par_map(&seeds, runner::threads(), |s| {
        grade_world(content, def, *s, args)
    });
    let mut worlds: Vec<WorldGrade> = Vec::new();
    let mut missed: Vec<Missed> = Vec::new();
    for r in results {
        match r {
            Ok(w) => worlds.push(w),
            Err(m) => missed.push(m),
        }
    }
    let mut coverage = Coverage::new(def);
    let mut bot_coverage = Coverage::new(def);
    let mut rows = Vec::new();
    let mut recent: Vec<Opening> = Vec::new();
    let (mut agency, mut variety, mut evp) = (Vec::new(), Vec::new(), Vec::new());
    let (mut differ, mut differ_contra, mut compared) = (0u32, 0u32, 0u32);
    let (mut under_agency, mut under_visibility, mut no_first_beat) = (0u32, 0u32, 0u32);
    // Per player: E, V, P on its own trace, its exits, its own coverage.
    let mut per: BTreeMap<String, PerPlayer> = BTreeMap::new();
    for w in &worlds {
        for p in &w.plays {
            let is_bot = p.who.starts_with("bot:");
            if is_bot {
                bot_coverage.add(&p.who, &p.trace);
            } else {
                coverage.add(&p.who, &p.trace);
            }
            let e = per.entry(p.who.clone()).or_insert_with(|| {
                (
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Coverage::new(def),
                )
            });
            e.0.push(Escalation::of(&p.trace).value());
            e.1.push(Visibility::of(&p.trace).value());
            e.2.push(Pacing::of(&p.trace, p.horizon_turns).value());
            e.3.push(p.trace.clone());
            e.4.add(&p.who, &p.trace);
        }
        let n = score::variety(&w.opening, &recent);
        recent.push(w.opening.clone());
        variety.push(n);
        let (Some(pa), Some(en), Some(co)) = (
            w.of(Kind::Passive),
            w.of(Kind::Engaged),
            w.of(Kind::Contrarian),
        ) else {
            continue;
        };
        let a = Agency::of(
            &pa.trace,
            &en.trace,
            &co.trace,
            score::distance(&pa.vector, &en.vector),
        );
        let e = Escalation::of(&pa.trace);
        let v = Visibility::of(&pa.trace);
        let p = Pacing::of(&en.trace, en.horizon_turns);
        // The first pass has only the passive run to order by (§6.4).
        let first_pass = score::evp(&e, &v, &Pacing::of(&pa.trace, pa.horizon_turns));
        compared += 1;
        differ += u32::from(a.passive_engaged_differ);
        differ_contra += u32::from(a.contrarian_engaged_differ);
        under_agency += u32::from(a.value() < AGENCY_FLOOR);
        under_visibility += u32::from(v.value() < VISIBILITY_FLOOR);
        no_first_beat += u32::from(!p.beat_on_turn_1);
        agency.push(a.value());
        evp.push(first_pass);
        let path =
            |t: &Trace| -> Vec<String> { t.entries.iter().map(|e| e.stage.clone()).collect() };
        rows.push(json!({
            "seed": w.seed, "cause": w.cause, "turns_to_raise": w.waited,
            "opening": { "matter": w.opening.matter, "post": w.opening.post, "town_class": w.opening.town_class },
            "agency": { "value": r3(a.value()), "passive_engaged_exits_differ": a.passive_engaged_differ,
                "contrarian_engaged_exits_differ": a.contrarian_engaged_differ, "distance": r3(a.distance) },
            "escalation": { "value": r3(e.value()), "stages": e.stages, "peak_heat": r3(f64::from(e.peak_heat)), "turned": e.turned },
            "visibility": { "value": r3(v.value()), "entries": v.entries, "seen": v.seen },
            "pacing": { "value": r3(p.value()), "beat_on_turn_1": p.beat_on_turn_1, "turns_to_turn": p.turned,
                "longest_gap": p.longest_gap, "turns_to_exit": p.exit, "horizon_turns": p.horizon },
            "variety": r3(n),
            "evp_first_pass": r3(first_pass),
            "paths": w.plays.iter().map(|p| (p.who.clone(), json!(path(&p.trace)))).collect::<serde_json::Map<_, _>>(),
            "beats_taken": w.plays.iter().map(|p| (p.who.clone(), json!(p.trace.taken.len()))).collect::<serde_json::Map<_, _>>(),
            "state_hashes": w.plays.iter().map(|p| (p.who.clone(), json!(p.state_hash))).collect::<serde_json::Map<_, _>>(),
        }));
    }
    let by_player: serde_json::Map<String, Value> = per
        .iter()
        .map(|(who, (e, v, p, traces, cov))| {
            let refs: Vec<&Trace> = traces.iter().collect();
            let reached = refs.iter().filter(|t| t.exit.is_some()).count();
            (
                who.clone(),
                json!({
                    "runs": traces.len(),
                    "escalation": spread(e.clone()),
                    "visibility": spread(v.clone()),
                    "pacing": spread(p.clone()),
                    "coverage_share": r3(cov.share()),
                    "reached_an_exit": reached,
                    "exit_shares": score::exit_shares(&refs).into_iter().map(|(k, x)| (k, json!(r3(x)))).collect::<serde_json::Map<_, _>>(),
                    "median_beats_taken": median(traces.iter().map(|t| t.taken.len() as f64).collect()),
                }),
            )
        })
        .collect();
    let engaged: Vec<&Trace> = per
        .get(Kind::Engaged.key())
        .map(|x| x.3.iter().collect())
        .unwrap_or_default();
    let engaged_exit_max = score::exit_shares(&engaged)
        .values()
        .copied()
        .fold(None::<f64>, |m, x| Some(m.map_or(x, |m| m.max(x))));
    let engaged_turns_to_turn = median(
        engaged
            .iter()
            .filter_map(|t| t.turned.map(f64::from))
            .collect(),
    );
    // A chapter is a budget year of governing turns.
    let chapter_turns = 365.0 / f64::from(content.tuning.turn_days.governing_local.max(1));
    let differ_share = (compared > 0).then(|| f64::from(differ) / f64::from(compared));
    let median_agency = median(agency.clone());
    let who_map =
        |list: &[(String, std::collections::BTreeSet<String>)]| -> serde_json::Map<String, Value> {
            list.iter().map(|(k, v)| (k.clone(), json!(v))).collect()
        };
    let reason = |stage: &str| unvisited_reason(&def.id, stage);
    let unvisited: Vec<Value> = coverage
        .unvisited_stages()
        .into_iter()
        .map(|s| (s, "stage"))
        .chain(coverage.unvisited_exits().into_iter().map(|s| (s, "exit")))
        .map(|(s, kind)| json!({ "id": s, "kind": kind, "reason": reason(s), "by_a_check_bot": !bot_coverage.stages.iter().chain(&bot_coverage.exits).find(|x| x.0 == s).is_none_or(|x| x.1.is_empty()) }))
        .collect();
    let unexplained: Vec<String> = coverage
        .unvisited_stages()
        .into_iter()
        .chain(coverage.unvisited_exits())
        .filter(|s| reason(s).is_none())
        .map(str::to_string)
        .collect();
    let mut causes: BTreeMap<&str, u32> = BTreeMap::new();
    for w in &worlds {
        *causes.entry(w.cause).or_default() += 1;
    }
    let mut misses: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    for m in &missed {
        misses.entry(m.reason.clone()).or_default().push(m.seed);
    }
    let enforced = args.seeds >= COVERAGE_SEEDS && !worlds.is_empty();
    let report = json!({
        "arc": def.id,
        "seeds": args.seeds,
        "worlds_found": worlds.len(),
        "causes": causes,
        "not_raised": misses,
        "terms": {
            "agency": spread(agency),
            "escalation": per.get(Kind::Passive.key()).map(|x| spread(x.0.clone())),
            "visibility": per.get(Kind::Passive.key()).map(|x| spread(x.1.clone())),
            "pacing": per.get(Kind::Engaged.key()).map(|x| spread(x.2.clone())),
            "variety": spread(variety),
            "evp_first_pass": spread(evp),
        },
        "exits_differ": {
            "worlds": compared,
            "passive_engaged": differ,
            "contrarian_engaged": differ_contra,
            "passive_engaged_share": differ_share.map(r3),
        },
        "floors": {
            "agency_under": { "floor": AGENCY_FLOOR, "worlds": under_agency },
            "visibility_under": { "floor": VISIBILITY_FLOOR, "worlds": under_visibility },
            "no_beat_on_turn_1": no_first_beat,
        },
        "by_player": by_player,
        "coverage": {
            "share": r3(coverage.share()),
            "complete": coverage.complete(),
            "stages": who_map(&coverage.stages),
            "exits": who_map(&coverage.exits),
            "unvisited": unvisited,
            "enforced": enforced,
            "enforced_from_seeds": COVERAGE_SEEDS,
        },
        "bars": [
            bar("exits_differ", &format!("passive and engaged exits differ in >= {EXITS_DIFFER_MIN} of worlds"), differ_share, differ_share.map(|x| x >= EXITS_DIFFER_MIN)),
            bar("median_agency", &format!("median A >= {MEDIAN_AGENCY_MIN}"), median_agency, median_agency.map(|x| x >= MEDIAN_AGENCY_MIN)),
            bar("engaged_exit_share", &format!("no exit takes over {ENGAGED_EXIT_SHARE_MAX} of engaged runs"), engaged_exit_max, engaged_exit_max.map(|x| x <= ENGAGED_EXIT_SHARE_MAX)),
            bar("turn_within_a_chapter", &format!("the engaged median reaches the turn within one chapter ({} turns)", r3(chapter_turns)), engaged_turns_to_turn, engaged_turns_to_turn.map(|x| x <= chapter_turns)),
        ],
        "rows": rows,
    });
    ArcGrade {
        arc: def.id.clone(),
        report,
        coverage,
        unexplained,
        found: worlds.len(),
        missed,
        engaged_turns_to_turn,
        chapter_turns,
    }
}

/// The templates a run of the grader covers.
pub fn templates<'c>(
    content: &'c ContentBundle,
    arc: Option<&str>,
) -> Result<Vec<&'c ArcDef>, String> {
    match arc {
        Some(id) => content
            .arc(id)
            .map(|d| vec![d])
            .ok_or_else(|| format!("no arc `{id}`")),
        None => Ok(content.arcs.iter().collect()),
    }
}

/// Grade the templates asked for.
pub fn grade_all(content: &Arc<ContentBundle>, args: &GradeArgs) -> Result<Vec<ArcGrade>, String> {
    Ok(templates(content, args.arc.as_deref())?
        .into_iter()
        .map(|d| grade(content, d, args))
        .collect())
}

fn cell(v: &Value) -> String {
    match v {
        Value::Null => "-".into(),
        Value::Number(n) => n.as_f64().map(|x| format!("{x:.3}")).unwrap_or_default(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        v => v.to_string(),
    }
}

/// The report as a table (the JSON holds everything; this is the part a
/// person reads).
pub fn text(grades: &[ArcGrade]) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    for g in grades {
        let r = &g.report;
        let _ = writeln!(
            s,
            "{}: {} of {} worlds (causes {}; not raised {})",
            g.arc, r["worlds_found"], r["seeds"], r["causes"], r["not_raised"]
        );
        let _ = writeln!(s, "  term        median   min     max");
        for t in [
            "agency",
            "escalation",
            "visibility",
            "pacing",
            "variety",
            "evp_first_pass",
        ] {
            let x = &r["terms"][t];
            let _ = writeln!(
                s,
                "  {t:<11} {:<8} {:<7} {}",
                cell(&x["median"]),
                cell(&x["min"]),
                cell(&x["max"])
            );
        }
        let _ = writeln!(s, "  player      E      V      P      coverage  exits");
        if let Some(m) = r["by_player"].as_object() {
            for (who, x) in m {
                let _ = writeln!(
                    s,
                    "  {who:<11} {:<6} {:<6} {:<6} {:<9} {}",
                    cell(&x["escalation"]["median"]),
                    cell(&x["visibility"]["median"]),
                    cell(&x["pacing"]["median"]),
                    cell(&x["coverage_share"]),
                    x["exit_shares"]
                );
            }
        }
        let _ = writeln!(
            s,
            "  exits differ: passive/engaged {} of {}, contrarian/engaged {}",
            r["exits_differ"]["passive_engaged"],
            r["exits_differ"]["worlds"],
            r["exits_differ"]["contrarian_engaged"]
        );
        let _ = writeln!(
            s,
            "  coverage {} (enforced: {})",
            cell(&r["coverage"]["share"]),
            r["coverage"]["enforced"]
        );
        for u in r["coverage"]["unvisited"].as_array().into_iter().flatten() {
            let _ = writeln!(
                s,
                "    unvisited {} {}: {}",
                cell(&u["kind"]),
                cell(&u["id"]),
                u["reason"].as_str().unwrap_or("NO REASON LISTED")
            );
        }
        for b in r["bars"].as_array().into_iter().flatten() {
            let _ = writeln!(
                s,
                "  bar (report-only) {}: measured {}, pass {}",
                cell(&b["target"]),
                cell(&b["measured"]),
                cell(&b["pass"])
            );
        }
    }
    s
}

/// `mandate-cli arcs grade`: the report, and the coverage failures (empty
/// under [`COVERAGE_SEEDS`] seeds, where coverage is only printed).
pub fn run(
    content: &Arc<ContentBundle>,
    args: &GradeArgs,
) -> Result<(Value, Vec<String>, String), String> {
    let grades = grade_all(content, args)?;
    let failures: Vec<String> = if args.seeds >= COVERAGE_SEEDS {
        grades
            .iter()
            .filter(|g| g.found > 0)
            .flat_map(|g| g.unexplained.iter().map(move |s| format!("{}:{s}", g.arc)))
            .collect()
    } else {
        Vec::new()
    };
    let table = text(&grades);
    let report = json!({
        "seeds": args.seeds,
        "horizon_weeks": score::HORIZON_WEEKS,
        "max_weeks": args.max_weeks,
        "wait_months": args.wait_months,
        "stand_ins": Kind::ARC.iter().map(|k| k.key()).collect::<Vec<_>>(),
        "coverage_failures": failures,
        "arcs": grades.into_iter().map(|g| g.report).collect::<Vec<_>>(),
    });
    Ok((report, failures, table))
}
