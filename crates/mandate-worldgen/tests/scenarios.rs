#![allow(clippy::unwrap_used, clippy::expect_used)]
//! DEC-078 decision 4: world scenarios are a content registry. The five
//! canonical worlds are entries in `content/base/scenarios.jsonc`, and a new
//! starting world (here a green, island-like country) is pure content.

use mandate_content::{ContentBundle, Source, bundle::read_dir_sources, load};
use mandate_sim::Command;
use mandate_worldgen::{WorldConfig, WorldSize, generate, generate_world};
use std::path::Path;
use std::sync::Arc;

fn base_sources() -> Vec<Source> {
    read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
}

fn base() -> Arc<ContentBundle> {
    Arc::new(load(&base_sources()).unwrap().0)
}

const ISLANDS: &str = r#"{
  // A wet, green archipelago-like country: three coasts, wealthy, tourism and
  // knowledge work, and voters who care about the environment.
  "kind": "scenarios",
  "items": [
    { "id": "scenario.green_isles", "name_key": "scenario-green_isles",
      "geography": { "sea_sides": 3, "wetness": 1.3, "population": 0.8 },
      "economy": { "endowment": { "visitor": 2.0, "extraction": 0.5 }, "wealth": 1.1 },
      "era": { "tech_growth": 0.03, "storminess": 0.008 },
      "politics": { "attitude_shift": { "environment": 0.3, "localism": 0.1 } } },
  ],
}"#;

fn with_islands(scenarios: &str) -> Vec<Source> {
    let mut src = base_sources();
    src.push(Source {
        path: "isles/scenarios.jsonc".into(),
        text: scenarios.into(),
    });
    src.push(Source {
        path: "isles/locale/en/isles.ftl".into(),
        text: "scenario-green_isles = The Green Isles\n".into(),
    });
    src
}

fn errors(src: &[Source]) -> String {
    match load(src) {
        Ok(_) => String::new(),
        Err(e) => e
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn mean_attitude(e: &mandate_sim::Engine, key: &str) -> f64 {
    let st = e.state();
    let i = e.content().attitude_index(key).unwrap();
    let (mut s, mut n) = (0.0, 0.0);
    for h in &st.population.hoods {
        for c in &h.cohorts {
            s += c.attitudes[i] as f64 * c.count as f64;
            n += c.count as f64;
        }
    }
    s / n
}

#[test]
fn base_scenarios_are_the_five_canonical_worlds() {
    let c = base();
    let ids: Vec<&str> = c.scenarios.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "scenario.industrial_decline",
            "scenario.wealthy_coast",
            "scenario.tech_growth",
            "scenario.drought_farmland",
            "scenario.resource_frontier",
        ]
    );
    // A world without a scenario is a random country: no biases.
    let random = c.scenario_or_random(None);
    assert!(random.economy.endowment.is_empty());
    assert_eq!(random.geography.population, 1.0);
    assert!(random.era.tech_growth.is_none());
    // Generation records the scenario in the state.
    let e = generate(
        &WorldConfig::with_scenario(5, WorldSize::Small, "scenario.tech_growth"),
        c.clone(),
    )
    .unwrap();
    assert_eq!(
        e.state().meta.params.scenario.as_deref(),
        Some("scenario.tech_growth")
    );
    assert_eq!(e.state().economy.national.tech_growth, 0.035);
    // An unknown scenario is refused.
    assert!(
        generate_world(
            &WorldConfig::with_scenario(5, WorldSize::Small, "scenario.atlantis"),
            &c,
            &mut |_, _| {}
        )
        .is_err()
    );
}

#[test]
fn a_content_only_scenario_loads_and_generates() {
    let c = Arc::new(load(&with_islands(ISLANDS)).unwrap().0);
    assert_eq!(c.scenarios.len(), 6);
    // Registry order is load order: `base/` sorts before `isles/`.
    assert_eq!(c.scenarios[5].id, "scenario.green_isles");
    let cfg = WorldConfig::with_scenario(9, WorldSize::Small, "scenario.green_isles");
    let mut a = generate(&cfg, c.clone()).unwrap();
    let mut b = generate(&cfg, c.clone()).unwrap();
    let plain = generate(&WorldConfig::new(9, WorldSize::Small), c.clone()).unwrap();
    assert_eq!(a.state_hash(), b.state_hash());
    // Its biases show: coastal towns, fewer people, a fixed era, greener voters.
    let coastal = |e: &mandate_sim::Engine| {
        e.state()
            .world
            .municipalities
            .iter()
            .filter(|m| m.coastal)
            .count()
    };
    assert!(coastal(&a) >= coastal(&plain));
    assert!(a.state().population.total() < plain.state().population.total());
    assert_eq!(a.state().economy.national.tech_growth, 0.03);
    assert_eq!(a.state().environment.trajectory.storminess, 0.008);
    assert!(mean_attitude(&a, "environment") > mean_attitude(&plain, "environment") + 0.2);
    // It runs, deterministically.
    for e in [&mut a, &mut b] {
        e.apply(Command::AdvanceDays { days: 365 }).unwrap();
    }
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn bad_scenarios_give_validation_errors() {
    let e = errors(&with_islands(
        &ISLANDS.replace("\"localism\": 0.1", "\"locals\": 0.1"),
    ));
    assert!(e.contains("unknown attitude `locals`"), "{e}");
    let e = errors(&with_islands(
        &ISLANDS.replace("\"sea_sides\": 3", "\"sea_sides\": 5"),
    ));
    assert!(e.contains("geography.sea_sides = 5 outside [1, 3]"), "{e}");
    let e = errors(&with_islands(
        &ISLANDS.replace("\"visitor\": 2.0", "\"tourism\": 2.0"),
    ));
    assert!(e.contains("tourism"), "{e}");
    let mut src = with_islands(ISLANDS);
    src.retain(|s| !s.path.ends_with("isles.ftl"));
    assert!(errors(&src).contains("missing Fluent key `scenario-green_isles`"));
    let e = errors(&with_islands(
        &ISLANDS.replace("scenario.green_isles", "scenario.wealthy_coast"),
    ));
    assert!(
        e.contains("duplicate content id `scenario.wealthy_coast`"),
        "{e}"
    );
}
