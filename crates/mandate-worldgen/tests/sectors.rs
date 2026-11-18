#![allow(clippy::unwrap_used, clippy::expect_used)]
//! DEC-075: economic sectors are a content registry. An expansion adds an
//! industry purely in content and the economy simulates it.

use mandate_content::{ContentBundle, SectorDriver, Source, bundle::read_dir_sources, load};
use mandate_sim::Command;
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn content_with_fishing() -> Arc<ContentBundle> {
    let mut src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    src.push(Source {
        path: "fishing/sectors.jsonc".into(),
        text: r#"{
          // A second land-based industry sharing agriculture's behaviour.
          "kind": "sectors",
          "items": [
            { "id": "sector.fishing", "name_key": "sector-fishing", "class": "primary",
              "driver": "land_based", "base_weight": 0.03, "base_wage": 34000,
              "cyclicality": 0.5, "productivity_growth": 0.01 },
          ],
        }"#
        .into(),
    });
    src.push(Source {
        path: "fishing/locale/en/fishing.ftl".into(),
        text: "sector-fishing = Fishing\n".into(),
    });
    Arc::new(load(&src).unwrap().0)
}

fn run(content: &Arc<ContentBundle>, seed: u64) -> mandate_sim::Engine {
    let mut e = generate(&WorldConfig::new(seed, WorldSize::Small), content.clone()).unwrap();
    e.apply(Command::AdvanceDays { days: 365 * 2 }).unwrap();
    e
}

#[test]
fn an_expansion_sector_generates_simulates_and_stays_deterministic() {
    let c = content_with_fishing();
    assert_eq!(c.sectors.len(), 9);
    assert_eq!(c.sectors_with(SectorDriver::LandBased).count(), 2);
    let fish = c.sector_index("sector.fishing").unwrap();

    let a = run(&c, 91);
    let b = run(&c, 91);
    assert_eq!(a.state_hash(), b.state_hash(), "same seed, same state");

    let s = a.state();
    assert_eq!(s.economy.sector_keys.len(), 9);
    let jobs: f64 = s.economy.munis.iter().map(|m| m.sectors[fish].jobs).sum();
    assert!(jobs > 0.0, "fishing employs people somewhere");
    for m in &s.economy.munis {
        assert_eq!(m.sectors.len(), 9);
        assert!(m.unemployment.is_finite() && (0.0..=0.6).contains(&m.unemployment));
        assert!(
            m.sectors
                .iter()
                .all(|x| x.jobs.is_finite() && x.jobs >= 0.0)
        );
    }

    // The lab shock addresses sectors by content key.
    let mut e = generate(&WorldConfig::new(91, WorldSize::Small), c.clone()).unwrap();
    e.apply(Command::LabSectorShock {
        municipality: 0,
        sector: "sector.fishing".into(),
        factor: 0.5,
        months: 6,
    })
    .unwrap();
    assert!(
        e.apply(Command::LabSectorShock {
            municipality: 0,
            sector: "sector.whaling".into(),
            factor: 0.5,
            months: 6,
        })
        .is_err()
    );
}
