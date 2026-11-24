//! The last post held (M6 note 16): a politician who leaves office keeps
//! the post they held (by the constitution's ids, any tier) and the day it
//! ended; the character sheet shows it for someone out of office, from
//! `known` on.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_content::ContentBundle;
use mandate_protocol::*;
use mandate_sim::player::knowledge::{self, Level, Source, Subject};
use mandate_sim::politics::Role;
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::{Arc, OnceLock};

fn content() -> Arc<ContentBundle> {
    static C: OnceLock<Arc<ContentBundle>> = OnceLock::new();
    C.get_or_init(|| {
        Arc::new(
            mandate_content::load(&mandate_content::embedded::sources())
                .unwrap()
                .0,
        )
    })
    .clone()
}

/// A game played through its first election (idle): seats change hands.
fn after_an_election(seed: u64) -> Engine {
    let mut e = generate(&WorldConfig::new(seed, WorldSize::Small), content()).unwrap();
    let start = e.state().calendar.date;
    for _ in 0..120 {
        if e.state()
            .politics
            .elections
            .last_election
            .is_some_and(|d| d > start)
        {
            break;
        }
        if e.apply(Command::EndTurn).is_err() {
            break;
        }
    }
    e
}

#[test]
fn leaving_office_records_the_post_and_the_day() {
    let e = after_an_election(2026);
    let st = e.state();
    let c = e
        .content()
        .constitutions
        .get(&st.meta.constitution)
        .unwrap();
    let held = st.politics.elections.last_election.unwrap();
    let mut left_now = 0;
    for p in &st.politics.politicians {
        let Some(last) = p.last_post else { continue };
        // The post is one of the constitution's, by its id.
        assert!(last.post.def(c).is_some(), "{:?}", last.post);
        assert!(last.ended <= st.calendar.date);
        if last.ended == held {
            left_now += 1;
            // Whoever lost a post at this election is out of office now, or
            // holds another post.
            assert!(p.role.post() != Some(last.post));
        }
    }
    assert!(left_now > 0, "nobody left office at the election");
    // The election's losers among the incumbents are exactly those.
    for r in st
        .politics
        .elections
        .results
        .iter()
        .filter(|r| r.date == held)
    {
        let Some(inc) = r.incumbent.filter(|i| *i != r.winner) else {
            continue;
        };
        let p = st.politics.get(inc).unwrap();
        if matches!(p.role, Role::Private { .. }) {
            let last = p.last_post.expect("an incumbent who left has a last post");
            assert_eq!((last.post, last.ended), (r.seat, held));
        }
    }
    // Nobody who never left a post has one recorded.
    assert!(
        st.politics
            .politicians
            .iter()
            .any(|p| p.last_post.is_none())
    );
}

#[test]
fn the_sheet_shows_it_once_the_person_is_known() {
    let mut e = after_an_election(2026);
    let (former, sitting) = {
        let st = e.state();
        let home = mandate_sim::player::home(st).map(|m| m.0);
        let me = st.player.as_ref().unwrap().politician;
        let former = st
            .politics
            .politicians
            .iter()
            .filter(|p| p.id != me && !p.in_office() && p.last_post.is_some())
            .find(|p| Some(p.municipality()) == home)
            .or_else(|| {
                st.politics
                    .politicians
                    .iter()
                    .find(|p| p.id != me && !p.in_office() && p.last_post.is_some())
            })
            .expect("a former officeholder");
        let sitting = st
            .politics
            .politicians
            .iter()
            .find(|p| p.id != me && p.in_office())
            .unwrap();
        (former.clone(), sitting.clone())
    };
    let teach = |e: &mut Engine, person, level| {
        e.setup(|s| {
            knowledge::teach_one(s, Subject::Person { person }, level, Source::Lab);
        })
    };
    // Below `known` the sheet says nothing of it.
    let before = character_view(&e, former.character.0).unwrap();
    if matches!(
        before.knowledge.level,
        KnowledgeLevelDto::Unknown | KnowledgeLevelDto::Heard
    ) {
        assert_eq!(before.last_post, None);
    } else {
        assert!(before.last_post.is_some());
    }
    teach(&mut e, former.character, Level::Known);
    let v = character_view(&e, former.character.0).unwrap();
    let last = v
        .last_post
        .expect("known: the sheet shows the last post held");
    let was = former.last_post.unwrap();
    assert_eq!(last.post, post_dto(&e, was.post));
    assert_eq!(last.post.id, was.post.id());
    assert!(!last.post.label_key.is_empty() && last.post.role_key.starts_with("role-"));
    assert_eq!(last.ended, DateView::of(was.ended));
    assert_eq!(last.years_in_office, former.years_in_office as u32);
    // Someone in office shows their post, not a last one.
    teach(&mut e, sitting.character, Level::Detailed);
    assert_eq!(
        character_view(&e, sitting.character.0).unwrap().last_post,
        None
    );
    // The field is left out of the JSON when absent.
    let j = serde_json::to_value(character_view(&e, sitting.character.0).unwrap()).unwrap();
    assert!(j.get("last_post").is_none());
}
