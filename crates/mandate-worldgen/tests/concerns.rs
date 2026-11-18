#![allow(clippy::unwrap_used, clippy::expect_used)]
//! DEC-078 decision 3: voter concerns are a content registry. Each town reads
//! its indicators monthly, turns them into concern levels, and weighs the
//! attitudes behind its concerns more in the vote model (salience, mean 1).
//! An expansion adds a concern purely in content.

use mandate_content::{ContentBundle, Indicator, Source, bundle::read_dir_sources, load};
use mandate_sim::Command;
use mandate_sim::scheduler::Reads;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn base_sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn base() -> Arc<ContentBundle> {
    Arc::new(load(&base_sources()).unwrap().0)
}

/// Base content with the concern registry replaced.
fn with_concerns(items: &str) -> Arc<ContentBundle> {
    let mut src = base_sources();
    let f = src
        .iter_mut()
        .find(|s| s.path == "base/concerns.jsonc")
        .unwrap();
    f.text = format!(r#"{{"kind":"concerns","items":[{items}]}}"#);
    Arc::new(load(&src).unwrap().0)
}

/// Base content plus a concern from an expansion pack: industrial pollution
/// worries the traditional-minded (heritage landscapes).
fn with_heritage() -> Arc<ContentBundle> {
    let mut src = base_sources();
    src.push(Source {
        path: "heritage/concerns.jsonc".into(),
        text: r#"{
          "kind": "concerns",
          "items": [
            { "id": "concern.heritage", "name_key": "concern-heritage",
              "indicator": "pollution",
              "attitudes": { "tradition": 1.0, "localism": 0.5 },
              "level": { "baseline": 0.0, "slope": 1.5, "min": 0.0, "max": 2.0 } },
          ],
        }"#
        .into(),
    });
    src.push(Source {
        path: "heritage/locale/en/heritage.ftl".into(),
        text: "concern-heritage = Our heritage\n".into(),
    });
    Arc::new(load(&src).unwrap().0)
}

const SEED: u64 = 61;

fn engine(c: &Arc<ContentBundle>) -> mandate_sim::Engine {
    generate(&WorldConfig::new(SEED, WorldSize::Small), c.clone()).unwrap()
}

#[test]
fn every_town_has_concern_levels_and_mean_one_salience() {
    let c = base();
    assert_eq!(c.concerns.len(), 6);
    assert_eq!(c.concern_index("concern.jobs"), Some(0));
    let e = engine(&c);
    let pop = &e.state().population;
    assert_eq!(pop.concern_keys.len(), 6);
    assert_eq!(
        pop.town_concerns.len(),
        e.state().world.municipalities.len()
    );
    for t in &pop.town_concerns {
        assert_eq!(t.levels.len(), 6);
        assert_eq!(t.salience.len(), c.attitudes.len());
        let mean = t.salience.iter().map(|s| *s as f64).sum::<f64>() / t.salience.len() as f64;
        assert!((mean - 1.0).abs() < 1e-5, "mean salience {mean}");
        assert!(t.levels.iter().all(|l| (0.0..=1.0).contains(l)));
    }
    // Towns differ: concerns are local.
    let distinct: std::collections::BTreeSet<String> = pop
        .town_concerns
        .iter()
        .map(|t| format!("{:.3?}", t.salience))
        .collect();
    assert!(distinct.len() > 3);
    // The top concerns are queryable for any town, strongest first.
    let top = mandate_sim::concerns::top_concerns(pop, 0);
    assert_eq!(top.len(), 6);
    assert!(top.windows(2).all(|w| w[0].1 >= w[1].1));
}

#[test]
fn high_unemployment_towns_weigh_jobs_issues_more() {
    // Only the jobs concern: salience of its attitudes rises with unemployment.
    let c = with_concerns(
        r#"{ "id": "concern.jobs", "name_key": "concern-jobs", "indicator": "unemployment",
             "attitudes": { "redistribution": 0.6, "reform_appetite": 0.3 },
             "level": { "baseline": -0.15, "slope": 9.0, "min": 0.0, "max": 1.0 } }"#,
    );
    let e = engine(&c);
    let st = e.state();
    let r = Reads::of_state(st, e.content());
    let redis = c.attitude_index("redistribution").unwrap();
    let tradition = c.attitude_index("tradition").unwrap();
    let mut towns: Vec<(f64, f32, f32)> = (0..st.world.municipalities.len())
        .map(|m| {
            let s = &st.population.town_concerns[m].salience;
            (
                mandate_sim::concerns::indicator(&r, m, Indicator::Unemployment),
                s[redis],
                s[tradition],
            )
        })
        .collect();
    towns.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (low, high) = (towns[0], towns[towns.len() - 1]);
    assert!(
        high.0 > low.0 + 0.02,
        "unemployment spread {low:?} {high:?}"
    );
    assert!(
        high.1 > low.1,
        "jobs issue weighs more where unemployment is high"
    );
    assert!(high.2 < low.2, "and other issues correspondingly less");
    // Salience is monotonic in unemployment across all towns.
    for w in towns.windows(2) {
        assert!(w[1].1 >= w[0].1 - 1e-6, "{w:?}");
    }
}

#[test]
fn an_expansion_concern_shifts_salience_toward_its_attitudes_where_its_indicator_is_high() {
    let base = base();
    let ext = with_heritage();
    assert_eq!(ext.concerns.len(), 7);
    assert_eq!(ext.concern_index("concern.heritage"), Some(6));
    let (a, b) = (engine(&base), engine(&ext));
    let tradition = ext.attitude_index("tradition").unwrap();
    let r = Reads::of_state(b.state(), b.content());
    let mut shifts: Vec<(f64, f64)> = (0..b.state().world.municipalities.len())
        .map(|m| {
            let before = a.state().population.town_concerns[m].salience[tradition] as f64;
            let after = b.state().population.town_concerns[m].salience[tradition] as f64;
            (
                mandate_sim::concerns::indicator(&r, m, Indicator::Pollution),
                after - before,
            )
        })
        .collect();
    shifts.sort_by(|x, y| x.0.total_cmp(&y.0));
    let n = shifts.len();
    let low: f64 = shifts[..n / 3].iter().map(|s| s.1).sum::<f64>() / (n / 3) as f64;
    let high: f64 = shifts[n - n / 3..].iter().map(|s| s.1).sum::<f64>() / (n / 3) as f64;
    assert!(high > 0.0, "tradition weighs more with the concern: {high}");
    assert!(
        high > low,
        "most where pollution is high: low {low} high {high}"
    );
    // The new concern is listed with its Fluent name, and the world runs on.
    let mut b = b;
    b.apply(Command::AdvanceDays { days: 365 }).unwrap();
    let levels = &b.state().population.town_concerns[0].levels;
    assert_eq!(levels.len(), 7);
}

#[test]
fn without_concerns_every_issue_weighs_the_same() {
    let c = with_concerns("");
    let e = engine(&c);
    for t in &e.state().population.town_concerns {
        assert!(t.levels.is_empty());
        assert!(t.salience.iter().all(|s| (*s - 1.0).abs() < 1e-6));
    }
}

#[test]
fn bad_concerns_give_validation_errors() {
    let err = |item: &str| {
        let mut src = base_sources();
        src.push(Source {
            path: "x/concerns.jsonc".into(),
            text: format!(r#"{{"kind":"concerns","items":[{item}]}}"#),
        });
        match load(&src) {
            Ok(_) => String::new(),
            Err(e) => e
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    };
    let ok = r#"{ "id": "concern.x", "name_key": "concern-jobs", "indicator": "pollution",
                  "attitudes": { "environment": 0.5 },
                  "level": { "baseline": 0.0, "slope": 1.0, "min": 0.0, "max": 1.0 } }"#;
    assert_eq!(err(ok), "");
    assert!(err(&ok.replace("environment", "ecology")).contains("unknown attitude `ecology`"));
    assert!(err(&ok.replace("pollution", "crime")).contains("unknown variant `crime`"));
    assert!(err(&ok.replace("\"min\": 0.0", "\"min\": 2.0")).contains("level.min must not exceed"));
    assert!(err(&ok.replace("0.5", "0.0")).contains("non-zero"));
    assert!(
        err(&ok.replace("concern-jobs", "concern-x")).contains("missing Fluent key `concern-x`")
    );
    assert!(err(&ok.replace("concern.x", "concern.jobs")).contains("duplicate content id"));
}

#[test]
fn salience_reaches_the_vote_model() {
    // Same world, same draws: only issue salience differs, and the burn-in's
    // elections come out differently.
    let shares = |c: &Arc<ContentBundle>| -> Vec<u32> {
        let e = engine(c);
        e.state()
            .politics
            .elections
            .results
            .iter()
            .flat_map(|r| r.candidates.iter().map(|x| (x.share * 10_000.0) as u32))
            .collect()
    };
    assert_ne!(shares(&base()), shares(&with_concerns("")));
}
