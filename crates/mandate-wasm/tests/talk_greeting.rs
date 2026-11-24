//! M6 note 19 through the worker: the talk view carries the greeting (the
//! opener the first reply would have started with, by the same rule and
//! keyed draw), the first reply leaves it out, and the worries question
//! offers the whole town and names the centre as the town centre.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use mandate_content::conversation::TalkPart;
use mandate_protocol::*;
use mandate_sim::characters::{self, knows};
use mandate_sim::talk::{self, context::Ctx, reply};
use mandate_wasm::session::Session;

fn send(s: &mut Session, id: u32, body: RequestBody) -> ResponseBody {
    let r: Response = serde_json::from_str(
        &s.handle_json(
            &serde_json::to_string(&Request {
                protocol_version: PROTOCOL_VERSION,
                request_id: id,
                body,
            })
            .unwrap(),
        ),
    )
    .unwrap();
    match r.result {
        ResponseResult::Ok { body } => body,
        ResponseResult::Err { error } => panic!("request {id}: {}", error.message),
    }
}

/// A session with the player beside a council member; their id.
fn beside_a_member(s: &mut Session, seed: u64) -> u32 {
    send(
        s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
        }),
    );
    let e = s.engine().unwrap();
    let town = mandate_sim::player::home(e.state()).unwrap();
    let at = mandate_sim::world::Jurisdiction::town(town);
    let m = mandate_sim::politics::body_members_at(&e.state().politics, at)
        .into_iter()
        .filter_map(|p| e.state().politics.get(p))
        .find(|p| !p.is_player)
        .map(|p| p.character)
        .unwrap();
    let spot = characters::routines::whereabouts(e.state(), e.content(), m).unwrap();
    send(
        s,
        2,
        RequestBody::Command(CommandDto::MoveTo {
            location: LocationDto::of(spot),
        }),
    );
    m.0
}

fn view(s: &mut Session, id: u32, person: u32) -> TalkView {
    let ResponseBody::Talk(v) = send(
        s,
        id,
        RequestBody::Query(QueryDto::Talk {
            to: TalkTargetDto::Person { person },
            late_night: false,
        }),
    ) else {
        panic!("a talk view")
    };
    v
}

#[test]
fn the_greeting_comes_with_the_view_and_the_first_reply_has_none() {
    for seed in [50, 77] {
        let mut s = Session::new().unwrap();
        let person = beside_a_member(&mut s, seed);
        let v = view(&mut s, 3, person);
        let g = v.greeting.clone().expect("the person speaks as it opens");
        assert_eq!(g.part, "opener");
        // The opener a reply would have started with, now, by the engine's
        // own rule (`reply::compose` with an opener): the same line.
        let e = s.engine().unwrap();
        let st = e.state();
        let c = e.content();
        let who = characters::CharacterId(person);
        let me = characters::player_character(st).unwrap();
        let pl = st.player.as_ref().unwrap();
        let ctx = Ctx {
            st,
            content: c,
            town: pl.home(),
            me,
            speaker: who,
            about: None,
            here: pl.location,
            cap: knows::discretion_cap(st, c, who, me, knows::Openness::default()),
        };
        let week = talk::week_of(st.calendar.date);
        let topic = c.talk_topic("topic.who_runs").unwrap();
        let composed = reply::compose(&ctx, topic, pl.talk.used_by(who), week, 0);
        let opener = composed
            .parts
            .iter()
            .find(|p| p.part == TalkPart::Opener)
            .unwrap();
        assert_eq!(opener.line.key, g.key, "seed {seed}");
        assert_eq!(v.voice, reply::voice(&ctx).map(|x| x.id.clone()));
        // The first reply: no opener of its own.
        let ResponseBody::CommandApplied(a) = send(
            &mut s,
            4,
            RequestBody::Command(CommandDto::Talk {
                to: TalkTargetDto::Person { person },
                topic: "topic.who_runs".into(),
                about: None,
                late_night: false,
            }),
        ) else {
            panic!("applied")
        };
        let id = a.result.unwrap().created.unwrap();
        let ResponseBody::Exchange(x) =
            send(&mut s, 5, RequestBody::Query(QueryDto::Exchange { id }))
        else {
            panic!("an exchange")
        };
        assert!(x.parts.iter().all(|p| p.part != "opener"), "{:?}", x.parts);
        assert!(!x.parts.is_empty());
        // Greeted today: the view carries no greeting now.
        assert!(view(&mut s, 6, person).greeting.is_none());
    }
}

#[test]
fn the_worries_question_offers_the_whole_town_and_the_town_centre() {
    let mut s = Session::new().unwrap();
    let person = beside_a_member(&mut s, 50);
    let v = view(&mut s, 3, person);
    let worries = v.topics.iter().find(|t| t.id == "topic.worries").unwrap();
    let whole = worries
        .about
        .iter()
        .find(|c| c.kind == "jurisdiction")
        .expect("the whole town");
    assert_eq!(whole.label_key.as_deref(), Some("talk-whole-local"));
    assert!(whole.available, "{:?}", whole.unavailable_key);
    // A neighbourhood that has its town's name is named as the town centre.
    let e = s.engine().unwrap();
    let st = e.state();
    let town = mandate_sim::player::home(st).unwrap();
    let name = &st.world.municipalities[town.index()].name;
    for c in worries.about.iter().filter(|c| c.kind == "hood") {
        if &c.label == name {
            assert_eq!(c.label_key.as_deref(), Some("ui-hood-centre"));
            assert_eq!(
                c.label_args.as_ref().and_then(|a| a.get("centre_town")),
                Some(name)
            );
        } else {
            assert!(c.label_args.is_none());
        }
    }
}
