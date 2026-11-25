//! Industries and named plants (I0, I1; DEC-110): the registry under the
//! sectors, plants at the works a town has, the review a fall in demand
//! takes a plant through, exact reconciliation of jobs, and the seam for
//! foreign trade (one market gives exactly one world index per industry).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use mandate_content::bundle::{Source, read_dir_sources};
use mandate_content::{ContentBundle, PlantOpens, load};
use mandate_core::SimDate;
use mandate_sim::economy::ActiveShock;
use mandate_sim::economy::industries::{self, Factor};
use mandate_sim::economy::plants::{self, Plant, PlantId, PlantStatus};
use mandate_sim::events::{DomainEvent, EventKind};
use mandate_sim::spine::{Cause, Link, Pressure, Thread};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::{Arc, OnceLock};

fn sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn content() -> Arc<ContentBundle> {
    static C: OnceLock<Arc<ContentBundle>> = OnceLock::new();
    C.get_or_init(|| Arc::new(load(&sources()).unwrap().0))
        .clone()
}

fn world(seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), content()).unwrap()
}

/// A plant a fall in demand can take all the way: running, of an industry
/// whose plants close, in a town with nothing under review and no notice
/// on record. The largest such plant of the first world that has one.
fn quiet_plant() -> (u64, Plant) {
    let c = content();
    for seed in 1..=12u64 {
        let e = world(seed);
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
            .max_by(|a, b| a.capacity.total_cmp(&b.capacity).then(b.id.cmp(&a.id)));
        if let Some(p) = pick {
            return (seed, p.clone());
        }
    }
    panic!("no world of twelve has a quiet plant that can close");
}

/// The lab's industry shock: demand for one industry falls in one town.
fn shock(e: &mut Engine, plant: &Plant, factor: f64, months: u16) {
    let c = content();
    let def = c.industry(&plant.industry).unwrap().clone();
    let m = plant.municipality.index();
    e.setup(|st| {
        st.economy.munis[m].shocks.push(ActiveShock {
            sector: def.sector.clone(),
            factor,
            months_left: months,
            industry: Some(def.id.clone()),
            demand: true,
        });
    });
}

fn month(e: &mut Engine) {
    let c = e.state().calendar.date.calendar();
    let next = industries::add_months(e.state().calendar.date, 1);
    let days = e.state().calendar.date.days_until(next).max(1);
    let _ = c;
    e.apply(Command::AdvanceDays { days }).unwrap();
}

fn plant_events(e: &Engine, plant: PlantId) -> Vec<DomainEvent> {
    e.state()
        .events
        .iter()
        .filter(|ev| match &ev.kind {
            EventKind::PlantShortTime { plant: p, .. }
            | EventKind::PlantThreatened { plant: p, .. }
            | EventKind::PlantReprieved { plant: p, .. } => *p == plant.0,
            EventKind::PlantClosure { plant: p, .. } => *p == Some(plant.0),
            _ => false,
        })
        .cloned()
        .collect()
}

fn pressure(ev: &DomainEvent) -> Option<Pressure> {
    ev.causes.iter().find_map(|c| match c {
        Cause::Pressure(p) => Some(*p),
        _ => None,
    })
}

#[test]
fn a_shock_takes_a_plant_through_short_time_a_dated_notice_and_the_closure() {
    let c = content();
    let (seed, plant) = quiet_plant();
    let mut e = world(seed);
    let before = e.state().calendar.date;
    let place = plant.place;
    assert!(e.state().world.places[place.index()].is_open());
    shock(&mut e, &plant, 0.55, 30);
    let mut seen = Vec::new();
    for _ in 0..30 {
        month(&mut e);
        let now = e.state().economy.plants[plant.id.index()].status;
        if now != PlantStatus::Running && seen.last() != Some(&now.key()) {
            seen.push(now.key());
        }
        if matches!(now, PlantStatus::Closed { .. }) {
            break;
        }
    }
    assert_eq!(
        seen,
        ["short_time", "threatened", "closed"],
        "the review's steps, in order"
    );
    let evs = plant_events(&e, plant.id);
    let kinds: Vec<&str> = evs
        .iter()
        .map(|ev| mandate_sim::spine::kind_key(&ev.kind))
        .collect();
    assert_eq!(
        kinds,
        ["plant_short_time", "plant_threatened", "plant_closure"]
    );
    // Each step carries its cause: a pressure the engine's formula names.
    for ev in &evs {
        assert!(pressure(ev).is_some(), "{:?} has no pressure", ev.kind);
    }
    // The notice names a date, and the plant closed on it: shown = used.
    let EventKind::PlantThreatened {
        closes_on, jobs, ..
    } = &evs[1].kind
    else {
        unreachable!()
    };
    assert!(*jobs > 0);
    let p = &c.economy.industries;
    let notice = evs[1].date;
    assert!(*closes_on >= industries::add_months(notice, p.notice_months[0] as u32));
    assert!(*closes_on <= industries::add_months(notice, p.notice_months[1] as u32));
    assert_eq!(
        evs[2].date, *closes_on,
        "it closed on the date the notice gave"
    );
    assert!(notice > before);
    // The closure names the plant, its industry and the weakest factor: the
    // fall in demand for what it makes, in the market it sells to.
    let EventKind::PlantClosure {
        industry,
        plant: named,
        factor,
        jobs,
        ..
    } = &evs[2].kind
    else {
        unreachable!()
    };
    assert_eq!(industry.as_deref(), Some(plant.industry.as_str()));
    assert_eq!(*named, Some(plant.id.0));
    assert_eq!(*factor, Some(Factor::World));
    assert!(*jobs > 0);
    let market = industries::market_id(&e.state().economy, "market.world").unwrap();
    assert_eq!(
        pressure(&evs[2]),
        Some(Pressure::ExternalDemand {
            market,
            good: plant.good
        }),
        "the cause says which market and which good"
    );
    // The review is one thread: the closure ends what the notice continued.
    let links = |ev: &DomainEvent| -> Vec<(u32, Link)> {
        ev.causes
            .iter()
            .filter_map(|c| c.event())
            .map(|(id, l)| (id.0, l))
            .collect()
    };
    assert!(links(&evs[1]).contains(&(evs[0].id.0, Link::Continues)));
    assert!(links(&evs[2]).contains(&(evs[1].id.0, Link::Ended)));
    assert_eq!(
        e.state().events.head(Thread::Plant(plant.id.0)),
        Some(evs[2].id)
    );
    // The plant keeps its record; its place is shut; its capacity is gone
    // from the industry, and the sums still hold.
    let st = e.state();
    let closed = &st.economy.plants[plant.id.index()];
    assert!(matches!(
        closed.status,
        PlantStatus::Closed {
            factor: Factor::World,
            ..
        }
    ));
    assert_eq!(closed.name, plant.name);
    assert!(!st.world.places[place.index()].is_open());
    reconciles(&e, &c);
    let facts = plants::facts(&st.economy, &st.world, &c, plant.id).unwrap();
    assert_eq!(facts.jobs, 0);
    assert_eq!(facts.factor, Factor::World);
}

#[test]
fn the_same_shock_lifted_in_time_ends_in_a_reprieve() {
    let c = content();
    let p = &c.economy.industries;
    let (seed, plant) = quiet_plant();
    let mut e = world(seed);
    // The shock lasts until the notice is served, and a month more: demand
    // is back before the shortest notice runs out.
    let months = p.notice_after_months as u16 + 1;
    shock(&mut e, &plant, 0.55, months);
    for _ in 0..(months as u32 + p.notice_months[1] as u32 + 2) {
        month(&mut e);
    }
    let evs = plant_events(&e, plant.id);
    let kinds: Vec<&str> = evs
        .iter()
        .map(|ev| mandate_sim::spine::kind_key(&ev.kind))
        .collect();
    assert_eq!(
        kinds,
        ["plant_short_time", "plant_threatened", "plant_reprieved"]
    );
    let EventKind::PlantThreatened { closes_on, .. } = &evs[1].kind else {
        unreachable!()
    };
    assert!(evs[2].date <= *closes_on, "lifted before the date");
    assert!(
        evs[2]
            .causes
            .iter()
            .any(|c| c.event() == Some((evs[1].id, Link::Ended))),
        "the reprieve ends the notice's thread"
    );
    let st = e.state();
    let now = &st.economy.plants[plant.id.index()];
    assert!(now.is_open());
    assert!(st.world.places[plant.place.index()].is_open());
    assert!(!matches!(now.status, PlantStatus::Threatened { .. }));
    // The twin differs from the closure only in how long the shock lasted.
    assert!(
        plants::facts(&st.economy, &st.world, &c, plant.id)
            .unwrap()
            .jobs
            > 0
    );
}

/// Every sum holds in one state: a split sector is its industries; an
/// industry's capacity is its unnamed firms and its open plants; and the
/// whole numbers on a sheet add up, plants to industries to sectors.
fn reconciles(e: &Engine, c: &ContentBundle) {
    let st = e.state();
    for (m, mu) in st.economy.munis.iter().enumerate() {
        for (k, s) in mu.sectors.iter().enumerate() {
            if !c.is_split(k) {
                continue;
            }
            let of = |f: &dyn Fn(&industries::IndustryState) -> f64| -> f64 {
                mu.industries
                    .iter()
                    .filter(|x| c.industries[x.k as usize].sector_index == k)
                    .map(f)
                    .sum()
            };
            assert_eq!(
                s.jobs.to_bits(),
                of(&|x| x.jobs).to_bits(),
                "jobs, town {m}"
            );
            assert_eq!(
                s.capacity.to_bits(),
                of(&|x| x.capacity).to_bits(),
                "capacity, town {m}"
            );
        }
        for x in &mu.industries {
            let key = &st.economy.industry_keys[x.k as usize];
            let open: f64 = st
                .economy
                .plants
                .iter()
                .filter(|p| p.municipality.index() == m && p.is_open() && p.industry == *key)
                .map(|p| p.capacity)
                .sum();
            assert_eq!(
                x.capacity.to_bits(),
                (x.small + open).to_bits(),
                "capacity of {key} in town {m}"
            );
            assert!(x.small >= 0.0 && x.jobs >= 0.0 && x.jobs.is_finite());
        }
        let (sectors, inds) = plants::town_jobs(&st.economy, c, m);
        for (k, total) in sectors.iter().enumerate() {
            if !c.is_split(k) {
                continue;
            }
            let parts: u32 = inds
                .iter()
                .filter(|i| c.industries[i.k].sector_index == k)
                .map(|i| i.jobs)
                .sum();
            assert_eq!(*total, parts, "industries to the sector, town {m}");
        }
        for i in &inds {
            let named: u32 = i.plants.iter().map(|p| p.jobs).sum();
            assert_eq!(i.jobs, named + i.small, "plants to the industry, town {m}");
        }
    }
}

#[test]
fn jobs_reconcile_in_every_town_and_month_of_thirty_years() {
    let c = content();
    let mut e = world(5);
    reconciles(&e, &c);
    for _ in 0..360 {
        month(&mut e);
        reconciles(&e, &c);
    }
    // And the run had something to reconcile through: plants closed and
    // opened on the way.
    let closed = e
        .state()
        .economy
        .plants
        .iter()
        .filter(|p| !p.is_open())
        .count();
    assert!(
        closed > 0,
        "thirty years and fifty of history close a plant"
    );
}

#[test]
fn towns_get_a_few_industries_by_geography_and_plants_at_their_works() {
    let c = content();
    let p = &c.economy.industries;
    let mut defining = std::collections::BTreeSet::new();
    for seed in [1u64, 2, 3] {
        // Before history, so the placement itself is what is read.
        let mut bare = (*c).clone();
        bare.tuning.burn_in_years = 0;
        let bare = Arc::new(bare);
        let e = generate(&WorldConfig::new(seed, WorldSize::Small), bare.clone()).unwrap();
        let st = e.state();
        for (m, mu) in st.economy.munis.iter().enumerate() {
            let w = &st.world.municipalities[m];
            let class = match w.class {
                mandate_sim::world::SettlementClass::Rural => 0,
                mandate_sim::world::SettlementClass::Town => 1,
                mandate_sim::world::SettlementClass::City => 2,
                mandate_sim::world::SettlementClass::Metro => 3,
            };
            for k in 0..c.sectors.len() {
                let here = mu
                    .industries
                    .iter()
                    .filter(|x| c.industries[x.k as usize].sector_index == k)
                    .count();
                assert!(here <= p.max_industries[class] as usize);
                if c.is_split(k) && mu.sectors[k].jobs > 0.0 {
                    assert!(here >= 1, "a split sector with jobs has an industry");
                }
            }
            let gates = industries::gateways(&st.world, &bare, m);
            for x in &mu.industries {
                let d = &c.industries[x.k as usize];
                // What an industry needs, the town has.
                for g in &d.siting.needs_gateway {
                    assert!(gates.iter().any(|y| y.0 == *g), "{} without a {g:?}", d.id);
                }
                if d.siting.needs.contains(&mandate_content::SiteFlag::Coastal) {
                    assert!(w.coastal, "{} inland", d.id);
                }
            }
            if let Some(top) = mu
                .industries
                .iter()
                .max_by(|a, b| a.jobs.total_cmp(&b.jobs))
            {
                defining.insert(top.k);
            }
        }
        // Plants: at a works the town has, one each, of the industry whose
        // works that kind of place is; and never most of a town.
        let mut places = std::collections::BTreeSet::new();
        for pl in &st.economy.plants {
            let d = c.industry(&pl.industry).unwrap();
            let place = &st.world.places[pl.place.index()];
            assert_eq!(place.municipality, pl.municipality);
            assert_eq!(d.plants.place_kind.as_deref(), Some(place.kind.key()));
            assert!(places.insert(pl.place), "one plant a place");
            assert!(d.products.iter().any(|x| x.id == pl.product));
            assert_eq!(
                st.economy.good_keys[pl.good.index()],
                c.product(&pl.product).unwrap().1.good
            );
            assert!(!pl.name.stem.is_empty());
            let jobs = st.economy.munis[pl.municipality.index()].total_jobs();
            assert!(pl.capacity <= p.max_town_share * jobs * 1.0001);
        }
        assert!(!st.economy.plants.is_empty());
    }
    assert!(
        defining.len() >= 6,
        "three small countries are led by {} industries",
        defining.len()
    );
}

#[test]
fn the_same_seed_gives_the_same_plants_and_they_survive_a_save() {
    let (a, b) = (world(9), world(9));
    assert_eq!(a.state().economy.plants, b.state().economy.plants);
    assert_eq!(a.state_hash(), b.state_hash());
    // A plant that can reopen exists in content (the openings are data).
    assert!(
        content()
            .industries
            .iter()
            .any(|d| d.plants.opens == PlantOpens::Investment)
    );
}

// --- The seam for foreign trade ---------------------------------------------------

#[test]
fn one_market_gives_exactly_the_single_world_index() {
    let c = content();
    assert_eq!(c.markets.len(), 1, "the base pack sells to one market");
    let mut e = world(4);
    for step in 0..18 {
        let before = e.state().economy.markets[0].index.clone();
        let from = e.state().calendar.date;
        month(&mut e);
        let st = e.state();
        let econ = &st.economy;
        assert_eq!(econ.market_keys, ["market.world"]);
        assert_eq!(econ.markets[0].access, 1.0);
        // The one market's index for an industry is the design's single
        // world index: the same walk, on the draw keyed by market, industry
        // and month.
        let stepped = monthly_dates(from, st.calendar.date);
        for (i, d) in c.industries.iter().enumerate() {
            let mut x = before[i];
            for day in &stepped {
                x = industries::walk(
                    x,
                    &d.world,
                    c.economy.industries.world_range,
                    st.meta.seed,
                    "market.world",
                    &d.id,
                    *day,
                )
                .0;
            }
            assert_eq!(x.to_bits(), econ.markets[0].index[i].to_bits(), "{}", d.id);
            // And external demand is exactly cycle ^ cyclicality × index:
            // the sum over one market at share 1 and open access adds
            // nothing and loses nothing.
            let (ext, cyc, market) = industries::external_demand(econ, &c, &econ.global, i);
            let cycle = mandate_core::math::pow(econ.global.demand, d.cyclicality);
            let design = cycle * econ.markets[0].index[i];
            assert_eq!(ext.to_bits(), design.to_bits(), "{} at step {step}", d.id);
            assert_eq!(cyc.to_bits(), cycle.to_bits());
            assert_eq!(market.0, 0);
            // All of its demand is external today, so home demand has no
            // weight in the number.
            assert_eq!(d.export_share, 1.0);
        }
    }
}

/// The first of each month after `from` up to `to`: the days the monthly
/// systems ran.
fn monthly_dates(from: SimDate, to: SimDate) -> Vec<SimDate> {
    let mut out = Vec::new();
    let mut d = industries::add_months(from, 1);
    while d <= to {
        out.push(d);
        d = industries::add_months(d, 1);
    }
    out
}

/// The base pack plus a second market that takes a quarter of every
/// industry's external sales.
fn with_a_neighbour() -> Arc<ContentBundle> {
    let mut src = sources();
    src.push(Source {
        path: "neighbour/markets.jsonc".into(),
        text: r#"{ "kind": "markets", "items": [
            { "id": "market.neighbour", "name_key": "market-neighbour", "share": 0.3333333333333333, "access": 1.0 } ] }"#
            .into(),
    });
    src.push(Source {
        path: "neighbour/locale/en/neighbour.ftl".into(),
        text: "market-neighbour = The neighbour\n".into(),
    });
    Arc::new(load(&src).unwrap().0)
}

#[test]
fn a_second_market_is_data_and_a_cause_names_the_market_that_closed() {
    let c = with_a_neighbour();
    assert_eq!(c.markets.len(), 2);
    let mut e = generate(&WorldConfig::new(4, WorldSize::Small), c.clone()).unwrap();
    {
        let econ = &e.state().economy;
        assert_eq!(econ.market_keys, ["market.world", "market.neighbour"]);
        let shares = c.market_shares(&c.industries[0]);
        assert!((shares[0] - 0.75).abs() < 1e-12 && (shares[1] - 0.25).abs() < 1e-12);
        // Each market has its own index for each industry, on its own draws.
        assert_ne!(econ.markets[0].index, econ.markets[1].index);
        for i in 0..c.industries.len() {
            let (ext, cyc, _) = industries::external_demand(econ, &c, &econ.global, i);
            let sum: f64 = (0..2)
                .map(|m| shares[m] * ((cyc * econ.markets[m].index[i]) * econ.markets[m].access))
                .sum();
            assert!((ext - sum).abs() < 1e-12);
        }
    }
    // The border closes: access to the neighbour falls to nothing. External
    // demand falls by about its share, and what is blamed is that market.
    let neighbour = industries::market_id(&e.state().economy, "market.neighbour").unwrap();
    let before: Vec<f64> = (0..c.industries.len())
        .map(|i| {
            industries::external_demand(&e.state().economy, &c, &e.state().economy.global, i).0
        })
        .collect();
    e.setup(|st| st.economy.markets[neighbour.index()].access = 0.0);
    for (i, was) in before.iter().enumerate() {
        let econ = &e.state().economy;
        let (ext, _, weakest) = industries::external_demand(econ, &c, &econ.global, i);
        assert!(ext < *was);
        assert_eq!(weakest, neighbour);
    }
    // A save from before the neighbour existed gains the market as it loads,
    // after the ones it had (ids never move).
    let base = world(4);
    let loaded = Engine::from_state(base.state().clone(), c.clone()).unwrap();
    assert_eq!(
        loaded.state().economy.market_keys,
        ["market.world", "market.neighbour"]
    );
    assert_eq!(
        loaded.state().economy.markets[0],
        base.state().economy.markets[0]
    );
}

#[test]
fn demand_at_home_counts_by_the_export_share() {
    // An industry that sells half at home: its plants read home demand too.
    // (No industry of the base pack does yet; this is the seam, exercised.)
    let mut src = sources();
    for s in src
        .iter_mut()
        .filter(|s| s.path.ends_with("industries.jsonc"))
    {
        s.text = s
            .text
            .replace("\"export_share\": 1.0", "\"export_share\": 0.5");
    }
    let half = Arc::new(load(&src).unwrap().0);
    assert!(half.industries.iter().all(|d| d.export_share == 0.5));
    let mut a = generate(&WorldConfig::new(6, WorldSize::Small), half).unwrap();
    let mut b = world(6);
    for _ in 0..24 {
        month(&mut a);
        month(&mut b);
    }
    let jobs = |e: &Engine| -> f64 {
        e.state()
            .economy
            .munis
            .iter()
            .flat_map(|m| m.industries.iter().map(|x| x.jobs))
            .sum()
    };
    assert!(jobs(&a).is_finite() && jobs(&a) > 0.0);
    assert_ne!(
        jobs(&a).to_bits(),
        jobs(&b).to_bits(),
        "home demand moved jobs"
    );
    reconciles(&a, &content());
}

// --- The registry's validation ------------------------------------------------------

#[test]
fn a_bad_parent_link_key_good_or_no_market_is_refused() {
    // The base pack itself loads clean (no errors, or `content()` would panic).
    assert_eq!(content().industries.len(), 16);
    let broken = |file: &str, from: &str, to: &str| {
        let mut src = sources();
        let mut hit = false;
        for s in src.iter_mut().filter(|s| s.path.ends_with(file)) {
            assert!(s.text.contains(from), "{from} not in {file}");
            s.text = s.text.replacen(from, to, 1);
            hit = true;
        }
        assert!(hit, "no {file}");
        load(&src).is_err()
    };
    let f = "industries.jsonc";
    assert!(
        broken(
            f,
            "\"sector\": \"sector.manufacturing\"",
            "\"sector\": \"sector.nowhere\""
        ),
        "an unknown parent"
    );
    assert!(
        broken(
            f,
            "\"sector\": \"sector.manufacturing\"",
            "\"sector\": \"sector.retail\""
        ),
        "a sector that runs on local demand"
    );
    assert!(
        broken(
            f,
            "\"from\": \"industry.mining\"",
            "\"from\": \"industry.nowhere\""
        ),
        "an unknown link"
    );
    assert!(
        broken(
            f,
            "\"name_key\": \"industry-metals\"",
            "\"name_key\": \"industry-nope\""
        ),
        "a name with no English"
    );
    assert!(
        broken(f, "\"good\": \"good.metals\"", "\"good\": \"good.nope\""),
        "a product of no good"
    );
    assert!(
        broken(
            f,
            "\"prices\": { \"good.minerals\": 0.6 }",
            "\"prices\": { \"good.ships\": 0.6 }"
        ),
        "a price exposure to a good with no price index"
    );
    assert!(
        broken(
            "markets.jsonc",
            "\"name_key\": \"market-world\"",
            "\"name_key\": \"market-nope\""
        ),
        "a market with no English name"
    );
    let mut src = sources();
    src.retain(|s| !s.path.ends_with("markets.jsonc"));
    assert!(load(&src).is_err(), "industries with no market to sell to");
}
