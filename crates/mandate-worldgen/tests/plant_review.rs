//! The short-time review (DEC-110, amended 2026-11-26): short time ends in
//! a review after a set number of months. Demand still under the review
//! line: a dated notice (the notice path: a date, a reprieve possible);
//! else full time again, sized to its orders. The probe (`--ignored`)
//! measures it over every town of sixteen small worlds.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use mandate_content::ContentBundle;
use mandate_core::SimDate;
use mandate_sim::economy::industries;
use mandate_sim::economy::plants::{self, Plant, PlantId, PlantStatus, ReviewOutcome};
use mandate_sim::events::{DomainEvent, EventKind};
use mandate_sim::spine::{Cause, Link, Thread};
use mandate_sim::{Command, Engine};
use mandate_testkit as kit;
use std::sync::Arc;

// --- The review, both ways ------------------------------------------------------------------

/// A plant of an industry that closes, running, in a town with no notice on
/// record and nothing else under review: the largest of the first world
/// that has one (no arcs, so nothing pushes on its margin).
fn quiet_plant(c: &Arc<ContentBundle>) -> (Engine, Plant) {
    for seed in 1..=12u64 {
        let e = kit::world(c, seed);
        let st = e.state();
        let pick = st
            .economy
            .plants
            .iter()
            .filter(|p| p.status == PlantStatus::Running)
            .filter(|p| c.industry(&p.industry).is_some_and(|d| d.plants.closes))
            .filter(|p| {
                let m = p.municipality;
                st.economy.munis[m.index()].last_notice.is_none()
                    && plants::of_town(&st.economy, m).all(|x| x.status == PlantStatus::Running)
            })
            .max_by(|a, b| a.capacity.total_cmp(&b.capacity).then(b.id.cmp(&a.id)))
            .cloned();
        if let Some(p) = pick {
            return (e, p);
        }
    }
    panic!("no world of twelve has a quiet plant that can close");
}

/// The first of the month `n` months before `d`.
fn months_before(d: SimDate, n: u32) -> SimDate {
    let c = d.calendar();
    let months = c.year * 12 + (c.month - 1) - n;
    SimDate::from_ymd(months / 12, months % 12 + 1, 1)
}

/// Put the plant on short time `months` ago, its demand held near `level`
/// (a relief on its margin, as a request would put it, for two years).
fn on_short_time(e: &mut Engine, plant: PlantId, months: u32, level: f64) {
    let c = e.content_arc();
    let facts = {
        let st = e.state();
        plants::facts(&st.economy, &st.world, &c, plant).unwrap()
    };
    let date = e.state().calendar.date;
    e.setup(|st| {
        let pl = &mut st.economy.plants[plant.index()];
        let base = facts.demand / (1.0 + pl.margin as f64 + pl.relief as f64);
        pl.status = PlantStatus::ShortTime {
            since: months_before(date, months),
        };
        pl.strain = 0;
        pl.turn = 0;
        pl.relief = (level / base - 1.0 - pl.margin as f64) as f32;
        pl.relief_months = 24;
    });
}

/// The level to ask `on_short_time` for so that the review reads `want`:
/// a fork is played to its review once, and the level scaled by what the
/// review read (the margin-adjusted demand is proportional to the level).
fn calibrated(e: &Engine, plant: PlantId, months: u32, want: f64) -> f64 {
    let mut f = Engine::from_state(e.state().clone(), e.content_arc()).unwrap();
    on_short_time(&mut f, plant, months, want);
    for _ in 0..3 {
        month(&mut f);
        let read = plant_events(&f, plant).iter().find_map(|ev| match ev.kind {
            EventKind::PlantReviewed { demand_pct, .. } => Some(demand_pct),
            _ => None,
        });
        if let Some(pct) = read {
            return want * want * 100.0 / pct.max(1) as f64;
        }
    }
    panic!("the fork was not reviewed");
}

fn month(e: &mut Engine) {
    let next = industries::add_months(e.state().calendar.date, 1);
    let days = e.state().calendar.date.days_until(next).max(1);
    e.apply(Command::AdvanceDays { days }).unwrap();
}

fn plant_events(e: &Engine, plant: PlantId) -> Vec<DomainEvent> {
    e.state()
        .events
        .iter()
        .filter(|ev| match &ev.kind {
            EventKind::PlantShortTime { plant: p, .. }
            | EventKind::PlantReviewed { plant: p, .. }
            | EventKind::PlantThreatened { plant: p, .. }
            | EventKind::PlantReprieved { plant: p, .. } => *p == plant.0,
            EventKind::PlantClosure { plant: p, .. } => *p == Some(plant.0),
            _ => false,
        })
        .cloned()
        .collect()
}

fn has_pressure(ev: &DomainEvent) -> bool {
    ev.causes.iter().any(|c| matches!(c, Cause::Pressure(_)))
}

/// The town's jobs add up: the plant's industry is its unnamed firms and
/// its open plants, bit for bit.
fn reconciles(e: &Engine, plant: &Plant) {
    let st = e.state();
    let c = e.content();
    let m = plant.municipality.index();
    let k = c.industry_index(&plant.industry).unwrap();
    let ind = st.economy.munis[m]
        .industries
        .iter()
        .find(|x| x.k as usize == k)
        .unwrap();
    let open: f64 = plants::of_town(&st.economy, plant.municipality)
        .filter(|p| p.is_open() && p.industry == plant.industry)
        .map(|p| p.capacity)
        .sum();
    assert!(
        (ind.capacity - (ind.small + open)).abs() < 1e-6,
        "capacity {} = small {} + plants {open}",
        ind.capacity,
        ind.small
    );
}

/// Twelve months of short time with demand still under the review line:
/// the review serves the notice (a date, the notice path), and the notice
/// names the review as its cause.
#[test]
fn a_review_with_demand_still_short_serves_a_dated_notice() {
    let c = kit::no_arcs();
    let p = c.economy.industries.clone();
    let (mut e, plant) = quiet_plant(&c);
    let months = p.review_months as u32 - 1;
    let level = calibrated(&e, plant.id, months, (p.notice_line + p.review_line) / 2.0);
    on_short_time(&mut e, plant.id, months, level);
    for _ in 0..3 {
        month(&mut e);
        if matches!(
            e.state().economy.plants[plant.id.index()].status,
            PlantStatus::Threatened { .. }
        ) {
            break;
        }
    }
    let evs = plant_events(&e, plant.id);
    let kinds: Vec<&str> = evs
        .iter()
        .map(|ev| mandate_sim::spine::kind_key(&ev.kind))
        .collect();
    assert_eq!(kinds, ["plant_reviewed", "plant_threatened"], "{evs:?}");
    let EventKind::PlantReviewed {
        months,
        demand_pct,
        line_pct,
        outcome,
        jobs_cut,
        ..
    } = &evs[0].kind
    else {
        unreachable!()
    };
    assert_eq!(*outcome, ReviewOutcome::Notice);
    assert_eq!(*months, p.review_months);
    assert_eq!(*line_pct, 88);
    assert!(*demand_pct < *line_pct, "{demand_pct}");
    assert_eq!(*jobs_cut, 0);
    // Each step carries the weakest factor; the notice follows from the
    // review on the same day, and the review continues the thread.
    assert!(has_pressure(&evs[0]) && has_pressure(&evs[1]));
    assert_eq!(evs[0].date, evs[1].date);
    assert!(
        evs[1]
            .causes
            .contains(&Cause::Event(evs[0].id, Link::Caused))
    );
    assert_eq!(
        e.state().events.head(Thread::Plant(plant.id.0)),
        Some(evs[1].id)
    );
    let EventKind::PlantThreatened { closes_on, .. } = &evs[1].kind else {
        unreachable!()
    };
    let now = &e.state().economy.plants[plant.id.index()];
    assert_eq!(
        now.status,
        PlantStatus::Threatened {
            since: evs[1].date,
            closes_on: *closes_on
        }
    );
    assert!(*closes_on >= industries::add_months(evs[1].date, p.notice_months[0] as u32));
    assert!(*closes_on <= industries::add_months(evs[1].date, p.notice_months[1] as u32));
    assert_eq!(now.capacity, plant.capacity, "a notice cuts nothing yet");
    assert_eq!(
        e.state().economy.munis[plant.municipality.index()].last_notice,
        Some(evs[1].date)
    );
    reconciles(&e, &plant);
}

/// The twin: the same plant, the same twelve months, demand back over the
/// review line (still under the full-time line). The review puts it back
/// on full time, shedding `review_cut` of its capacity.
#[test]
fn a_review_with_demand_back_over_the_line_returns_to_full_time_smaller() {
    let c = kit::no_arcs();
    let p = c.economy.industries.clone();
    let (mut e, plant) = quiet_plant(&c);
    let months = p.review_months as u32 - 1;
    let level = calibrated(&e, plant.id, months, (p.review_line + p.full_line) / 2.0);
    on_short_time(&mut e, plant.id, months, level);
    let jobs_before = {
        let st = e.state();
        plants::facts(&st.economy, &st.world, &c, plant.id)
            .unwrap()
            .jobs
    };
    for _ in 0..3 {
        month(&mut e);
        if e.state().economy.plants[plant.id.index()].status == PlantStatus::Running {
            break;
        }
    }
    let evs = plant_events(&e, plant.id);
    let kinds: Vec<&str> = evs
        .iter()
        .map(|ev| mandate_sim::spine::kind_key(&ev.kind))
        .collect();
    assert_eq!(kinds, ["plant_reviewed"], "{evs:?}");
    let EventKind::PlantReviewed {
        months,
        demand_pct,
        line_pct,
        outcome,
        jobs_cut,
        jobs,
        ..
    } = &evs[0].kind
    else {
        unreachable!()
    };
    assert_eq!(*outcome, ReviewOutcome::FullTime);
    assert_eq!(*months, p.review_months);
    assert!(*demand_pct >= *line_pct, "{demand_pct}");
    assert!(
        *jobs_cut > 0,
        "still under the full-time line: it sheds jobs"
    );
    assert!(*jobs + *jobs_cut <= jobs_before + 1);
    assert!(has_pressure(&evs[0]));
    // Full time ends the thread; the plant runs, smaller by review_cut.
    assert_eq!(
        e.state().events.head(Thread::Plant(plant.id.0)),
        Some(evs[0].id)
    );
    let now = &e.state().economy.plants[plant.id.index()];
    assert_eq!(now.status, PlantStatus::Running);
    assert!((now.capacity - plant.capacity * (1.0 - p.review_cut)).abs() < 1e-6);
    assert!(
        e.state().economy.munis[plant.municipality.index()]
            .last_notice
            .is_none()
    );
    reconciles(&e, &plant);
}

/// Short time that has not run its course is not reviewed: the same plant
/// a month short of the review, a month on, is still on short time.
#[test]
fn no_review_before_its_months() {
    let c = kit::no_arcs();
    let p = c.economy.industries.clone();
    let (mut e, plant) = quiet_plant(&c);
    let level = (p.notice_line + p.review_line) / 2.0;
    on_short_time(&mut e, plant.id, p.review_months as u32 - 3, level);
    month(&mut e);
    assert!(plant_events(&e, plant.id).is_empty());
    assert!(matches!(
        e.state().economy.plants[plant.id.index()].status,
        PlantStatus::ShortTime { .. }
    ));
}

// --- The probe -----------------------------------------------------------------------------

#[derive(Default, Clone, Debug)]
struct Tally {
    spells: u32,
    world: u64,
    reviews_notice: u32,
    reviews_full: u32,
    notices: u32,
    reprieves: u32,
    closures: u32,
    closure_jobs: u64,
    cut_jobs: u64,
    /// Plants given a notice by a review, and of them those given two or
    /// more (a notice lifted, short time again, a review again).
    noticed: std::collections::BTreeMap<u64, u32>,
}

#[derive(Default, Clone, Debug)]
struct Snapshot {
    tally: Tally,
    /// Of the plants on short time at the start: still on short time, back
    /// on full time, under notice, closed.
    of_start: [u32; 4],
    /// Plants on short time now; of them a year or more; the longest (months).
    short_now: u32,
    short_year: u32,
    longest_months: u32,
}

fn months_between(a: mandate_core::SimDate, b: mandate_core::SimDate) -> u32 {
    let (x, y) = (a.calendar(), b.calendar());
    ((y.year as i64 - x.year as i64) * 12 + y.month as i64 - x.month as i64).max(0) as u32
}

fn snapshot(e: &Engine, start_short: &[u32], tally: &Tally) -> Snapshot {
    let st = e.state();
    let date = st.calendar.date;
    let mut s = Snapshot {
        tally: tally.clone(),
        ..Default::default()
    };
    for id in start_short {
        let i = match st.economy.plants[*id as usize].status {
            PlantStatus::ShortTime { .. } => 0,
            PlantStatus::Running => 1,
            PlantStatus::Threatened { .. } => 2,
            PlantStatus::Closed { .. } => 3,
        };
        s.of_start[i] += 1;
    }
    for p in &st.economy.plants {
        if let PlantStatus::ShortTime { since } = p.status {
            let m = months_between(since, date);
            s.short_now += 1;
            s.short_year += u32::from(m >= 12);
            s.longest_months = s.longest_months.max(m);
        }
    }
    s
}

/// Read new events by their serde tags, so the probe reads any version of
/// the plant events.
fn count(e: &Engine, after: &mut u32, t: &mut Tally) {
    let from = *after;
    for ev in e.state().events.iter().filter(|ev| ev.id.0 > from) {
        *after = ev.id.0;
        let v = serde_json::to_value(&ev.kind).unwrap();
        let num = |k: &str| v.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
        match mandate_sim::spine::kind_key(&ev.kind) {
            "plant_short_time" => t.spells += 1,
            "plant_reviewed" => {
                if v.get("outcome").and_then(|x| x.as_str()) == Some("notice") {
                    t.reviews_notice += 1;
                    *t.noticed
                        .entry(t.world * 100_000 + num("plant"))
                        .or_default() += 1;
                } else {
                    t.reviews_full += 1;
                }
                t.cut_jobs += num("jobs_cut");
            }
            "plant_threatened" => t.notices += 1,
            "plant_reprieved" => t.reprieves += 1,
            "plant_closure" if v.get("plant").is_some_and(|x| !x.is_null()) => {
                t.closures += 1;
                t.closure_jobs += num("jobs");
            }
            _ => {}
        }
    }
}

/// 16 small worlds, every town, idle from the first day of play: the
/// plants' review at 39 weeks and at five years.
/// `cargo test --release -p mandate-worldgen --test plant_review -- --ignored --nocapture`
/// `MANDATE_PROBE_REVIEW_MONTHS=9999` puts the review out of reach (the
/// rule before the amendment, the same run otherwise: the review draws
/// nothing), for the before column.
#[test]
#[ignore = "a probe: prints a table"]
fn probe_short_time_over_sixteen_worlds() {
    let mut c = kit::pack();
    if let Some(m) = std::env::var("MANDATE_PROBE_REVIEW_MONTHS")
        .ok()
        .and_then(|x| x.parse().ok())
    {
        c.economy.industries.review_months = m;
    }
    let c = Arc::new(c);
    let horizons = [273u32, 5 * 365 + 1];
    let mut at = [Snapshot::default(), Snapshot::default()];
    let (mut start_n, mut start_year, mut start_longest, mut towns) = (0u32, 0u32, 0u32, 0u32);
    for seed in 1..=16u64 {
        let mut e = kit::world(&c, seed);
        let start = e.state().calendar.date;
        towns += e.state().world.municipalities.len() as u32;
        let start_short: Vec<u32> = e
            .state()
            .economy
            .plants
            .iter()
            .filter(|p| matches!(p.status, PlantStatus::ShortTime { .. }))
            .map(|p| p.id.0)
            .collect();
        start_n += start_short.len() as u32;
        for p in &e.state().economy.plants {
            if let PlantStatus::ShortTime { since } = p.status {
                let m = months_between(since, start);
                start_year += u32::from(m >= 12);
                start_longest = start_longest.max(m);
            }
        }
        let mut after = e.state().events.iter().last().map_or(0, |ev| ev.id.0);
        let mut tally = Tally {
            world: seed,
            ..Default::default()
        };
        for (h, days) in horizons.iter().enumerate() {
            while start.days_until(e.state().calendar.date) < *days {
                kit::end_turn(&mut e);
                count(&e, &mut after, &mut tally);
            }
            let s = snapshot(&e, &start_short, &tally);
            let a = &mut at[h];
            a.tally.spells += s.tally.spells;
            a.tally.reviews_notice += s.tally.reviews_notice;
            a.tally.reviews_full += s.tally.reviews_full;
            a.tally.notices += s.tally.notices;
            a.tally.reprieves += s.tally.reprieves;
            a.tally.closures += s.tally.closures;
            a.tally.closure_jobs += s.tally.closure_jobs;
            a.tally.cut_jobs += s.tally.cut_jobs;
            a.tally.noticed.extend(s.tally.noticed.clone());
            for i in 0..4 {
                a.of_start[i] += s.of_start[i];
            }
            a.short_now += s.short_now;
            a.short_year += s.short_year;
            a.longest_months = a.longest_months.max(s.longest_months);
        }
        // A worked example: the first plant of world 1 a review gave a
        // notice, and its whole thread.
        if seed == 1
            && let Some(first) = tally.noticed.keys().next()
        {
            let plant = PlantId((first % 100_000) as u32);
            let pl = &e.state().economy.plants[plant.index()];
            println!(
                "example: world 1, plant {} ({}, {:?}) in town {}",
                plant.0, pl.industry, pl.name, pl.municipality.0
            );
            for ev in plant_events(&e, plant) {
                let d = ev.date.calendar();
                println!(
                    "  {}-{:02} {:?} <- {:?}",
                    d.year, d.month, ev.kind, ev.causes
                );
            }
        }
    }
    println!(
        "start: {start_n} plants on short time; {start_year} a year or more; longest {start_longest} months; {towns} towns"
    );
    for (h, a) in at.iter().enumerate() {
        let years = horizons[h] as f64 / 365.25;
        let t = &a.tally;
        println!(
            "after {} days: short-time spells begun {}; reviews {} (to notice {}, to full time {}); notices {}; reprieves {}; closures {}",
            horizons[h],
            t.spells,
            t.reviews_notice + t.reviews_full,
            t.reviews_notice,
            t.reviews_full,
            t.notices,
            t.reprieves,
            t.closures
        );
        println!(
            "  of the start's short time: short {} / full {} / notice {} / closed {}; on short time now {} (a year or more {}, longest {} months)",
            a.of_start[0],
            a.of_start[1],
            a.of_start[2],
            a.of_start[3],
            a.short_now,
            a.short_year,
            a.longest_months
        );
        println!(
            "  plants given a notice by a review {}; of them twice or more {}",
            t.noticed.len(),
            t.noticed.values().filter(|n| **n >= 2).count()
        );
        println!(
            "  jobs lost: closures {} + review cuts {}; per town per year {:.2}",
            t.closure_jobs,
            t.cut_jobs,
            (t.closure_jobs + t.cut_jobs) as f64 / towns as f64 / years
        );
    }
}
