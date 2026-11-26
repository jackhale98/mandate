#![allow(clippy::unwrap_used, clippy::print_stdout)]
//! Throwaway probe: why plants on short time rarely move in 39 weeks.
use mandate_sim::economy::plants::PlantStatus;
use std::collections::BTreeMap;

#[test]
#[ignore]
fn probe() {
    let c = mandate_testkit::only(&["arc.plant_closure"]);
    let mut finals: BTreeMap<String, u32> = BTreeMap::new();
    let mut by_closes: BTreeMap<(bool, String), u32> = BTreeMap::new();
    let mut strain_hist: BTreeMap<u8, u32> = BTreeMap::new();
    let mut max_strain_hist: BTreeMap<u8, u32> = BTreeMap::new();
    let mut since_hist: BTreeMap<u32, u32> = BTreeMap::new();
    let mut total = 0;
    for seed in 1..=16u64 {
        let mut e = mandate_testkit::world(&c, seed);
        let st = e.state();
        let start = st.calendar.date;
        let watched: Vec<usize> = st
            .economy
            .plants
            .iter()
            .enumerate()
            .filter(|(_, p)| matches!(p.status, PlantStatus::ShortTime { .. }))
            .map(|(i, _)| i)
            .collect();
        for &i in &watched {
            if let PlantStatus::ShortTime { since } = st.economy.plants[i].status {
                let months = since.days_until(start) / 30;
                *since_hist.entry(months / 6 * 6).or_default() += 1;
            }
        }
        let mut max_strain = vec![0u8; st.economy.plants.len()];
        for _ in 0..20 {
            mandate_testkit::end_turn(&mut e);
            for &i in &watched {
                let p = &e.state().economy.plants[i];
                max_strain[i] = max_strain[i].max(p.strain);
            }
        }
        let st = e.state();
        for &i in &watched {
            let p = &st.economy.plants[i];
            let closes = c.industry(&p.industry).unwrap().plants.closes;
            let k = p.status.key().to_string();
            *finals.entry(k.clone()).or_default() += 1;
            *by_closes.entry((closes, k)).or_default() += 1;
            *strain_hist.entry(p.strain).or_default() += 1;
            *max_strain_hist.entry(max_strain[i]).or_default() += 1;
            total += 1;
        }
    }
    println!("plants on short time at the start: {total}");
    println!("after 20 turns: {finals:?}");
    println!("by (closes, status): {by_closes:?}");
    println!("strain at the end: {strain_hist:?}");
    println!("max strain in the 20 turns: {max_strain_hist:?}");
    println!("months on short time at the start (6-month bins): {since_hist:?}");
}
