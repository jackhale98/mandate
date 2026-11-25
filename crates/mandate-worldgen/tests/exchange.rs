#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stdout)]
//! Exploration (temporary).

use mandate_content::{ContentBundle, bundle::read_dir_sources, load};
use mandate_sim::arcs::exchange;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::path::Path;
use std::sync::Arc;

fn base() -> Arc<ContentBundle> {
    let src =
        read_dir_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap();
    Arc::new(load(&src).unwrap().0)
}

fn engine(c: &Arc<ContentBundle>, seed: u64) -> Engine {
    generate(&WorldConfig::new(seed, WorldSize::Small), c.clone()).unwrap()
}

#[test]
#[ignore]
fn explore() {
    let c = base();
    for seed in [1u64, 2, 3, 7, 31, 99, 777, 2026] {
        let mut e = engine(&c, seed);
        println!("== seed {seed}");
        for turn in 0..40 {
            let st = e.state();
            let open = exchange::view::open_scenes(st, &c);
            let runs: Vec<String> = st
                .arcs
                .runs
                .iter()
                .map(|r| {
                    format!(
                        "{}#{}@{} clock={:?} played={:?} marks={:?}",
                        r.template,
                        r.id.0,
                        r.stage,
                        r.clock.map(|d| d.0),
                        r.played.iter().map(|p| (p.outcome, p.result)).collect::<Vec<_>>(),
                        r.marks
                    )
                })
                .collect();
            println!(
                "t{turn} d{} phase={:?} lead={:?} open={} runs={runs:?}",
                st.calendar.date.0,
                st.calendar.phase,
                st.narrative.chapter.as_ref().map(|ch| (ch.kind, ch.lead)),
                open.len()
            );
            if let Some((run, beat, _)) = open.first() {
                let v = exchange::view(st, &c, run.0, &beat.key()).unwrap();
                println!(
                    "   scene: phase={:?} place={:?} groups={:?} last={:?}",
                    v.phase,
                    v.place,
                    v.groups
                        .iter()
                        .map(|g| (g.key.clone(), g.weight, g.lean))
                        .collect::<Vec<_>>(),
                    v.last_day.map(|d| d.0)
                );
            }
            if e.apply(Command::EndTurn).is_err() {
                break;
            }
            if e.state().arcs.past.iter().any(|p| p.template == "arc.rival") {
                let p = e
                    .state()
                    .arcs
                    .past
                    .iter()
                    .find(|p| p.template == "arc.rival")
                    .unwrap();
                println!("   rival ended: {} marks={:?}", p.exit, p.marks);
                break;
            }
        }
    }
}
