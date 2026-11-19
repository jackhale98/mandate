#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Per-neighbourhood hazard exposure, damage and displacement (DEC-090 D8,
//! TOWN_GENERATION §15 G1): exposure follows the terrain, a strike falls on
//! the exposed neighbourhoods, protections shrink it, the displaced come
//! home, and all of it is deterministic.

use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::Command;
use mandate_sim::environment::exposure::{HoodHit, split};
use mandate_sim::environment::{HazardKind, HoodExposure};
use mandate_sim::events::{EventKind, EventLog};
use mandate_sim::scheduler::{Effect, EffectQueue, Outs, Reads};
use mandate_sim::world::{MunicipalityId, NeighborhoodId, NeighborhoodKind};
use mandate_worldgen::exposure::cell;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn content() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

#[test]
fn exposure_is_higher_on_floodplains() {
    let c = content();
    let e = generate(&WorldConfig::new(81, WorldSize::Small), c).unwrap();
    let st = e.state();
    let t = e.terrain().unwrap();
    let env = &st.environment;
    assert_eq!(env.exposure.len(), st.world.neighborhoods.len());
    // On the land itself: the floodplain (on or beside a river, low) is more
    // exposed than high ground far from water.
    let land: Vec<usize> = (0..t.w * t.h)
        .filter(|&i| t.owner[i] != u32::MAX && t.cover[i] == mandate_sim::world::Cover::Country)
        .collect();
    let river_low: Vec<f64> = land
        .iter()
        .filter(|&&i| t.river[i] >= 2 && t.elevation[i] < 80.0)
        .map(|&i| cell(t, i).flood)
        .collect();
    let high_dry: Vec<f64> = land
        .iter()
        .filter(|&&i| t.river[i] == 0 && t.elevation[i] > 400.0)
        .map(|&i| cell(t, i).flood)
        .collect();
    assert!(!river_low.is_empty() && !high_dry.is_empty());
    assert!(
        mean(&river_low) > mean(&high_dry) + 0.3,
        "floodplain {} vs high ground {}",
        mean(&river_low),
        mean(&high_dry)
    );
    // By neighbourhood: the waterfront and industrial districts sit lower by
    // the water than the suburbs, and towns on rivers have a flood-prone
    // district (exposure varies within the town).
    let by_kind = |k: NeighborhoodKind| -> Vec<f64> {
        st.world
            .neighborhoods
            .iter()
            .filter(|n| n.kind == k)
            .map(|n| f64::from(env.exposure[n.id.index()].flood))
            .collect()
    };
    let (water, suburb) = (
        by_kind(NeighborhoodKind::Waterfront),
        by_kind(NeighborhoodKind::Suburb),
    );
    assert!(
        mean(&water) > mean(&suburb),
        "waterfront {} vs suburb {}",
        mean(&water),
        mean(&suburb)
    );
    let varied = st
        .world
        .municipalities
        .iter()
        .filter(|m| m.on_river && m.neighborhoods.len() > 2)
        .filter(|m| {
            let f: Vec<u16> = m
                .neighborhoods
                .iter()
                .map(|h| env.exposure[h.index()].flood)
                .collect();
            f.iter().max() > f.iter().min()
        })
        .count();
    assert!(varied > 0, "a river town has a flood-prone district");
    // Every exposure is a per mille.
    assert!(
        env.exposure
            .iter()
            .all(|x| x.flood <= 1000 && x.wildfire <= 1000 && x.storm <= 1000)
    );
}

/// A town with differing flood exposure, and its shares of a flood.
fn flood_town(st: &mandate_sim::State, c: &ContentBundle, protection: f64) -> (u32, Vec<HoodHit>) {
    let m = st
        .world
        .municipalities
        .iter()
        .filter(|m| m.neighborhoods.len() > 2)
        .max_by_key(|m| {
            let f: Vec<u16> = m
                .neighborhoods
                .iter()
                .map(|h| st.environment.exposure[h.index()].flood)
                .collect();
            (
                f.iter().max().copied().unwrap_or(0) - f.iter().min().copied().unwrap_or(0),
                std::cmp::Reverse(m.id.0),
            )
        })
        .unwrap()
        .id
        .0;
    let hits = split(
        &st.world,
        &st.environment.exposure,
        &|h| st.population.hood_total(h),
        m,
        HazardKind::Flood,
        0.02,
        &|_| protection,
        &c.economy.hazards,
    );
    (m, hits)
}

#[test]
fn a_flood_damages_exposed_neighbourhoods_more() {
    let c = content();
    let e = generate(&WorldConfig::new(82, WorldSize::Small), c.clone()).unwrap();
    let st = e.state();
    let (_, hits) = flood_town(st, &c, 0.0);
    let flood = |h: &HoodHit| st.environment.exposure[h.neighborhood as usize].flood;
    let most = hits.iter().max_by_key(|h| flood(h)).unwrap();
    let least = hits.iter().min_by_key(|h| flood(h)).unwrap();
    assert!(flood(most) > flood(least));
    assert!(
        most.damage > least.damage,
        "{} vs {}",
        most.damage,
        least.damage
    );
    // The town's total is what an even split would give (the town roll and
    // its damage are unchanged).
    let town: f64 = hits.iter().map(|h| h.share * h.damage).sum();
    assert!((town - 0.02).abs() < 1e-6, "town damage {town}");
    // An even hazard (an earthquake) falls evenly.
    let quake = split(
        &st.world,
        &st.environment.exposure,
        &|h| st.population.hood_total(h),
        most.neighborhood,
        HazardKind::Earthquake,
        0.04,
        &|_| 0.0,
        &c.economy.hazards,
    );
    assert!(quake.iter().all(|h| (h.relative - 1.0).abs() < 1e-9));
}

#[test]
fn protections_reduce_the_damage() {
    let c = content();
    let e = generate(&WorldConfig::new(82, WorldSize::Small), c.clone()).unwrap();
    let st = e.state();
    let (_, open) = flood_town(st, &c, 0.0);
    let (_, walled) = flood_town(st, &c, 0.6);
    for (a, b) in open.iter().zip(&walled) {
        assert_eq!(a.neighborhood, b.neighborhood);
        assert!((b.damage - a.damage * 0.4).abs() < 1e-9);
        assert_eq!(
            a.relative, b.relative,
            "protection scales, never moves, the damage"
        );
    }
    // A neighbourhood without exposure gains nothing from protection.
    let st2 = {
        let mut s = st.clone();
        for x in s.environment.exposure.iter_mut() {
            *x = HoodExposure::default();
        }
        s.environment.exposure[0].flood = 800;
        s
    };
    let hits = split(
        &st2.world,
        &st2.environment.exposure,
        &|h| st2.population.hood_total(h),
        st2.world.neighborhoods[0].municipality.0,
        HazardKind::Flood,
        0.02,
        &|_| 0.5,
        &c.economy.hazards,
    );
    if hits.len() > 1 {
        assert!(hits.iter().skip(1).all(|h| h.damage == 0.0));
    }
}

/// Run one month of population migration on a state, with these effects.
fn migrate(st: &mut mandate_sim::State, c: &ContentBundle, effects: Vec<Effect>) {
    let snapshot = st.clone();
    let mut rng = snapshot.rng.clone();
    let mut events = EventLog::default();
    let mut q = EffectQueue::default();
    for e in effects {
        q.push(e);
    }
    let r = Reads::of_state(&snapshot, c);
    let mut out = Outs {
        rng: &mut rng,
        events: &mut events,
        effects: &mut q,
    };
    mandate_sim::population::migration(&mut st.population, &r, &mut out);
}

#[test]
fn the_displaced_return_over_time() {
    let c = content();
    let e = generate(&WorldConfig::new(83, WorldSize::Small), c.clone()).unwrap();
    let mut st = e.state().clone();
    let (m, hits) = flood_town(&st, &c, 0.0);
    let hit = hits
        .iter()
        .max_by(|a, b| a.relative.total_cmp(&b.relative))
        .unwrap()
        .clone();
    let home = NeighborhoodId(hit.neighborhood);
    let town =
        |s: &mandate_sim::State| s.population.municipality_total(&s.world, MunicipalityId(m));
    let before_home = st.population.hood_total(home.index());
    let before_town = town(&st);
    let effects: Vec<Effect> = hits
        .iter()
        .map(|h| Effect::HoodDisaster {
            neighborhood: h.neighborhood,
            relative: h.relative,
            leave: 0.0,
            temporary: 0.2 * h.relative,
            cause: None,
        })
        .collect();
    migrate(&mut st, &c, effects);
    let away = st.population.displaced_from(home);
    assert!(away > 0, "residents of the flooded district stay elsewhere");
    assert!(st.population.hood_total(home.index()) < before_home);
    assert!((st.population.impact(home.index()) - hit.relative).abs() < 1e-6);
    // Staying elsewhere in the town, they still count in it (no one left
    // for good, give or take the month's ordinary moves).
    let moved = town(&st).abs_diff(before_town);
    assert!(moved < before_town / 50, "town moved {moved}");
    let mut last = away;
    for _ in 0..60 {
        migrate(&mut st, &c, Vec::new());
        let now = st.population.displaced_from(home);
        assert!(now <= last, "the displaced only come home");
        last = now;
    }
    assert_eq!(last, 0, "everyone is home or gone within five years");
    assert!(st.population.displaced.is_empty());
}

#[test]
fn play_is_deterministic_and_damage_falls_where_exposed() {
    let c = content();
    let cfg = WorldConfig::new(84, WorldSize::Small);
    let mut a = generate(&cfg, c.clone()).unwrap();
    let mut b = generate(&cfg, c.clone()).unwrap();
    assert_eq!(
        a.state().environment.exposure,
        b.state().environment.exposure
    );
    assert_eq!(a.state_hash(), b.state_hash());
    // Month by month: whenever a flood strikes a town with differing
    // exposure, its most exposed neighbourhood is damaged at least as much
    // as its least exposed one.
    let mut floods = 0;
    let mut seen = a.state().events.iter().count();
    for _ in 0..72 {
        a.apply(Command::AdvanceDays { days: 30 }).unwrap();
        b.apply(Command::AdvanceDays { days: 30 }).unwrap();
        let st = a.state();
        let new: Vec<(u32, HazardKind)> = st
            .events
            .iter()
            .skip(seen)
            .filter_map(|ev| match ev.kind {
                EventKind::Disaster {
                    municipality,
                    hazard: HazardKind::Flood,
                    ..
                } => Some((municipality, HazardKind::Flood)),
                _ => None,
            })
            .collect();
        seen = st.events.iter().count();
        for (m, _) in new {
            let hoods = &st.world.municipalities[m as usize].neighborhoods;
            let ex = |h: &NeighborhoodId| st.environment.exposure[h.index()].flood;
            let dmg = |h: &NeighborhoodId| st.environment.damage(*h).map_or(0, |d| d.damaged_pm);
            let most = hoods.iter().max_by_key(|h| (ex(h), std::cmp::Reverse(h.0)));
            let least = hoods.iter().min_by_key(|h| (ex(h), h.0));
            if let (Some(hi), Some(lo)) = (most, least)
                && ex(hi) > ex(lo)
            {
                floods += 1;
                assert!(dmg(hi) >= dmg(lo), "flood in town {m}");
            }
        }
    }
    assert_eq!(a.state_hash(), b.state_hash());
    assert!(floods > 0, "six years see a flood somewhere");
}
