//! Owner note 1 (2026-11-21): "Out of office" is the player's own standing.
//! No one else in any view (street people, pocket, notebook, chamber, talk
//! speaker, character sheet) is labelled `role-private`: a politician with
//! no post shows their trade.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_protocol::characters::{character_view, people_at, person_ref};
use mandate_protocol::talk::{TalkTargetDto, talk_view};
use mandate_protocol::{knowledge, office, places};
use mandate_sim::politics::Role;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::Arc;

fn game(seed: u64) -> Engine {
    let (c, _) = mandate_content::load(&mandate_content::embedded::sources()).unwrap();
    generate(
        &WorldConfig {
            seed,
            size: WorldSize::Small,
            scenario: None,
        },
        Arc::new(c),
    )
    .unwrap()
}

/// Every `role_key` in a serialised view that reads `role-private`.
fn private_labels(v: &serde_json::Value, path: &str, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(m) => {
            for (k, x) in m {
                if k == "role_key" && x == "role-private" {
                    out.push(format!("{path}: {m:?}"));
                }
                private_labels(x, &format!("{path}.{k}"), out);
            }
        }
        serde_json::Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                private_labels(x, &format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}

fn check(e: &Engine, what: &str) {
    let st = e.state();
    let me = mandate_sim::characters::player_character(st);
    let mut bad = Vec::new();
    for c in &st.politics.characters.people {
        let id = c.id;
        if Some(id) == me {
            continue;
        }
        let p = person_ref(e, id).expect("person");
        assert_ne!(p.role_key, "role-private", "{what}: person_ref {id:?}");
        let v = character_view(e, id.0).expect("sheet");
        assert_ne!(v.role_key, "role-private", "{what}: character_view {id:?}");
        if let Some(t) = talk_view(e, TalkTargetDto::Person { person: id.0 }, false) {
            private_labels(&serde_json::to_value(&t).unwrap(), "talk", &mut bad);
        }
    }
    // The chamber: the player's own seat is theirs; strip it, check the rest.
    let mut o = serde_json::to_value(office::office_view(e).unwrap()).unwrap();
    o.as_object_mut().unwrap().remove("player");
    private_labels(&o, "office", &mut bad);
    private_labels(
        &serde_json::to_value(places::pocket_view(e).unwrap()).unwrap(),
        "pocket",
        &mut bad,
    );
    private_labels(
        &serde_json::to_value(knowledge::notebook_view(e).unwrap()).unwrap(),
        "notebook",
        &mut bad,
    );
    let loc = st.player.as_ref().unwrap().location;
    private_labels(
        &serde_json::to_value(people_at(e, loc)).unwrap(),
        "street",
        &mut bad,
    );
    assert!(bad.is_empty(), "{what}: {bad:#?}");
}

#[test]
fn nobody_but_the_player_is_out_of_office_over_their_head() {
    for seed in [3u64, 11, 29, 41] {
        let mut e = game(seed);
        e.set_lab_omniscient(true);
        // Retire a share of the field so out-of-office politicians exist in
        // every seed, whatever the elections do.
        e.setup(|st| {
            let me = st.player.as_ref().unwrap().politician;
            let ids: Vec<_> = st.politics.politicians.iter().map(|p| p.id).collect();
            for (i, id) in ids.into_iter().enumerate() {
                if id != me
                    && i % 3 == 0
                    && let Some(p) = st.politics.get_mut(id)
                {
                    p.role = Role::Private {
                        home: mandate_sim::world::MunicipalityId(p.home),
                    };
                }
            }
        });
        for turn in 0..12 {
            check(&e, &format!("seed {seed} turn {turn}"));
            e.apply(Command::EndTurn).unwrap();
        }
    }
}
