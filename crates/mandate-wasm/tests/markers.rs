//! The name tags' "!" (`PersonRefView::has_news`): the engine decides who
//! has something new for the player, from its own talk log (a topic that
//! tells things, not asked this week) and knowledge ledger (a lead points to
//! them). Views never write.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_content::conversation::TalkMechanic;
use mandate_protocol::*;
use mandate_sim::actions::PlayerCommand;
use mandate_sim::characters::CharacterId;
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Lead, LeadTarget, Level, Source, Subject};
use mandate_sim::talk::{self, TalkTarget};
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

/// Where a member of the town's body is on a day it sits (DEC-031 routines).
fn body_spot(e: &mut Engine) -> Location {
    let at = mandate_sim::world::Jurisdiction::town(mandate_sim::player::home(e.state()).unwrap());
    for _ in 0..8 {
        let now = mandate_sim::characters::routines::Now::of(e.state(), e.content());
        if now.session(e.state().meta.seed, at) {
            break;
        }
        e.apply(Command::EndTurn).unwrap();
    }
    let m = mandate_sim::politics::body_members_at(&e.state().politics, at)
        .into_iter()
        .filter_map(|p| e.state().politics.get(p))
        .find(|p| !p.is_player)
        .map(|p| p.character)
        .unwrap();
    mandate_sim::characters::routines::whereabouts(e.state(), e.content(), m).unwrap()
}

fn tells_things(e: &Engine) -> Vec<String> {
    e.content()
        .talk_topics
        .iter()
        .filter(|t| t.mechanic == TalkMechanic::Inform && !t.about_required)
        .map(|t| t.id.clone())
        .collect()
}

fn ask(e: &mut Engine, person: CharacterId, topic: &str) {
    e.apply(Command::Player(PlayerCommand::Talk {
        to: TalkTarget::Person { person },
        topic: topic.into(),
        about: None,
        late_night: false,
    }))
    .unwrap();
}

fn leads_to(e: &mut Engine, person: CharacterId, keep: bool) {
    e.setup(|st| {
        let on = st.calendar.date;
        let k = &mut st.player.as_mut().unwrap().knowledge;
        k.leads.retain(|l| l.to != LeadTarget::Person { person });
        if keep {
            k.leads.push(Lead {
                subject: Subject::Person { person },
                level: Level::Detailed,
                key: "lead-person-knows".into(),
                to: LeadTarget::Person { person },
                since: on,
                source: Source::Lab,
            });
        }
    });
}

/// Someone not asked this week has news; once asked everything that tells
/// things, they have none until next week; a lead to them is news again.
#[test]
fn news_is_an_unasked_topic_or_a_lead() {
    let mut e = game(41);
    let at = body_spot(&mut e);
    e.apply(Command::Player(PlayerCommand::MoveTo { location: at }))
        .unwrap();
    let person = talk::present(e.state(), e.content())[0];
    leads_to(&mut e, person, false);
    let p = person_ref(&e, person).unwrap();
    assert!(p.can_talk && p.has_news, "not asked yet: {p:?}");
    assert_eq!(news(&e, person), vec![News::Topic]);

    let before = e.state_hash();
    let _ = people_at(&e, at);
    assert_eq!(e.state_hash(), before, "views never write");

    for t in tells_things(&e) {
        if talk::check(e.state(), e.content(), person, &t, None, false).is_ok() {
            ask(&mut e, person, &t);
        }
    }
    // A reply may open a lead to them: that is the other reason, tested below.
    leads_to(&mut e, person, false);
    assert!(news(&e, person).is_empty(), "asked this week");
    assert!(!person_ref(&e, person).unwrap().has_news);
    let listed = people_at(&e, at);
    let here = listed.iter().find(|p| p.id == Some(person.0)).unwrap();
    assert!(here.can_talk && !here.has_news);

    leads_to(&mut e, person, true);
    assert_eq!(news(&e, person), vec![News::Lead]);
    assert!(
        person_ref(&e, person).unwrap().has_news,
        "a lead points to them"
    );
    leads_to(&mut e, person, false);

    // A new week: what tells things can be asked again.
    let week = talk::week_of(e.state().calendar.date);
    while talk::week_of(e.state().calendar.date) == week {
        e.apply(Command::EndTurn).unwrap();
    }
    leads_to(&mut e, person, false);
    assert_eq!(news(&e, person), vec![News::Topic], "a new week");
}

/// Passers-by and the player have no news; nothing breaks without a player.
#[test]
fn strangers_and_the_player_have_none() {
    let mut e = game(43);
    let me = mandate_sim::characters::player_character(e.state()).unwrap();
    assert!(!has_news(&e, me));
    let hood = e
        .state()
        .world
        .hoods_of(mandate_sim::player::home(e.state()).unwrap())
        .next()
        .unwrap();
    let street = Location::Street {
        neighborhood: mandate_sim::world::NeighborhoodId(hood as u32),
    };
    e.apply(Command::Player(PlayerCommand::MoveTo { location: street }))
        .unwrap();
    let people = people_at(&e, street);
    assert!(
        people
            .iter()
            .filter(|p| p.ambient_slot.is_some())
            .all(|p| !p.has_news)
    );
}
