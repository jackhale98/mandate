#![allow(clippy::unwrap_used, clippy::expect_used)]
use mandate_protocol::*;
use mandate_wasm::session::{Session, read_save_meta};

fn req(id: u32, body: RequestBody) -> String {
    serde_json::to_string(&Request {
        protocol_version: PROTOCOL_VERSION,
        request_id: id,
        body,
    })
    .unwrap()
}

#[test]
fn new_game_end_turn_save_load() {
    let mut s = Session::new().unwrap();
    let r: Response = serde_json::from_str(&s.handle_json(&req(
        1,
        RequestBody::NewGame(NewGameParams {
            seed: 42,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
        }),
    )))
    .unwrap();
    assert!(matches!(
        r.result,
        ResponseResult::Ok {
            body: ResponseBody::GameStarted(_)
        }
    ));
    for i in 0..3 {
        let r: Response = serde_json::from_str(
            &s.handle_json(&req(10 + i, RequestBody::Command(CommandDto::EndTurn))),
        )
        .unwrap();
        assert_eq!(r.request_id, 10 + i);
    }
    let hash = s.engine().unwrap().state_hash();
    let save = s
        .export_save(&SaveRequestDto {
            run_id: "ab".repeat(16),
            revision: 3,
            parent_revision: Some(2),
            device_id: "d".into(),
            saved_at: "2026-11-18T00:00:00Z".into(),
            run_mode: RunModeDto::Ironman,
        })
        .unwrap();
    assert_eq!(read_save_meta(&save).unwrap().revision, 3);
    let mut s2 = Session::new().unwrap();
    s2.import_save(&save).unwrap();
    assert_eq!(s2.engine().unwrap().state_hash(), hash);
    // Whose the save is, read by a session with no game of its own (the title screen's list): the
    // pocket header's words, and the running game untouched.
    let s3 = Session::new().unwrap();
    let who = s3.read_save_summary(&save).unwrap();
    let o = office_view(s.engine().unwrap()).unwrap();
    assert_eq!(who.player_name, o.player.name);
    assert_eq!(who.role_key, o.player.role_key);
    assert_eq!(who.home, o.player.municipality);
    assert!(!who.player_name.is_empty() && !who.home.is_empty());
    assert!(s3.engine().is_none());
    assert!(s3.read_save_summary(b"not a save").is_err());
}

#[test]
fn bad_requests_return_errors_not_panics() {
    let mut s = Session::new().unwrap();
    let r: Response = serde_json::from_str(&s.handle_json("{not json")).unwrap();
    assert!(matches!(r.result, ResponseResult::Err { .. }));
    let r: Response =
        serde_json::from_str(&s.handle_json(&req(2, RequestBody::Command(CommandDto::EndTurn))))
            .unwrap();
    assert!(matches!(
        r.result,
        ResponseResult::Err {
            error: ProtocolError {
                code: ErrorCode::NoGame,
                ..
            }
        }
    ));
    let mut bad = Request {
        protocol_version: 99,
        request_id: 3,
        body: RequestBody::Query(QueryDto::Overview),
    };
    bad.protocol_version = 99;
    let r: Response =
        serde_json::from_str(&s.handle_json(&serde_json::to_string(&bad).unwrap())).unwrap();
    assert!(matches!(
        r.result,
        ResponseResult::Err {
            error: ProtocolError {
                code: ErrorCode::VersionMismatch,
                ..
            }
        }
    ));
}

#[test]
fn every_event_key_has_english_text() {
    let s = Session::new().unwrap();
    let keys = mandate_content::validate::fluent_keys(&s.content().locale_text("en"));
    for k in [
        "event-game-started",
        "event-year-began",
        "event-turn-ended",
        "event-regional-shock",
    ] {
        assert!(keys.contains(k), "missing {k}");
    }
}

#[test]
fn map_and_municipality_queries_survive_save_load() {
    let mut s = Session::new().unwrap();
    s.handle_json(&req(
        1,
        RequestBody::NewGame(NewGameParams {
            seed: 5,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
        }),
    ));
    let map = |s: &mut Session| -> MapView {
        let r: Response =
            serde_json::from_str(&s.handle_json(&req(2, RequestBody::Query(QueryDto::Map))))
                .unwrap();
        match r.result {
            ResponseResult::Ok {
                body: ResponseBody::Map(m),
            } => m,
            other => panic!("{other:?}"),
        }
    };
    let before = map(&mut s);
    assert_eq!(
        before.biome_b64.len(),
        (before.width * before.height).div_ceil(3) as usize * 4
    );
    let save = s
        .export_save(&SaveRequestDto {
            run_id: "cd".repeat(16),
            revision: 1,
            parent_revision: None,
            device_id: "d".into(),
            saved_at: "2026-11-18T00:00:00Z".into(),
            run_mode: RunModeDto::Story,
        })
        .unwrap();
    let mut s2 = Session::new().unwrap();
    s2.import_save(&save).unwrap();
    assert_eq!(map(&mut s2), before);
    let r: Response = serde_json::from_str(&s2.handle_json(&req(
        3,
        RequestBody::Query(QueryDto::Municipality { id: 0 }),
    )))
    .unwrap();
    assert!(matches!(
        r.result,
        ResponseResult::Ok {
            body: ResponseBody::Municipality(_)
        }
    ));
}

fn send(s: &mut Session, id: u32, body: RequestBody) -> ResponseBody {
    let r: Response = serde_json::from_str(&s.handle_json(&req(id, body))).unwrap();
    match r.result {
        ResponseResult::Ok { body } => body,
        ResponseResult::Err { error } => panic!("request {id} failed: {}", error.message),
    }
}

fn spot_of(town: &TownPlacesView, kind: &str) -> LocationDto {
    town.neighborhoods
        .iter()
        .flat_map(|h| &h.spots)
        .find(|s| s.location.kind_key == kind)
        .map(|s| s.location.location)
        .unwrap()
}

fn office(s: &mut Session, id: u32) -> OfficeView {
    match send(s, id, RequestBody::Query(QueryDto::Office)) {
        ResponseBody::Office(o) => o,
        _ => panic!("not an office view"),
    }
}

/// DEC-079 rule 4: places give depth, not access. The exact whip count is
/// only in the council chamber; detailed town data only at the town hall.
#[test]
fn information_depth_depends_on_where_the_player_is() {
    let mut s = Session::new().unwrap();
    send(
        &mut s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed: 31337,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
        }),
    );
    let ResponseBody::Town(town) = send(&mut s, 2, RequestBody::Query(QueryDto::Town)) else {
        panic!("not a town view");
    };
    assert_eq!(town.location, LocationDto::Home);
    let hall = spot_of(&town, "town_hall");
    let chamber = spot_of(&town, "council_chamber");
    // The pocket routes drafting to the town hall.
    let ResponseBody::Pocket(pocket) = send(&mut s, 3, RequestBody::Query(QueryDto::Pocket)) else {
        panic!("not a pocket view");
    };
    let draft = pocket.routes.iter().find(|r| r.action == "draft").unwrap();
    assert_eq!(draft.to.as_ref().unwrap().location, hall);
    assert!(!draft.here);
    // Draft and introduce through auto-routing (one request each).
    let template = office(&mut s, 4).catalog[0].id.clone();
    let ResponseBody::CommandApplied(r) = send(
        &mut s,
        5,
        RequestBody::Command(CommandDto::Go {
            location: hall,
            then: Some(Box::new(CommandDto::DraftPolicy {
                template,
                params: vec![],
            })),
        }),
    ) else {
        panic!("not applied");
    };
    let bill = r.result.unwrap().created.unwrap();
    // At the town hall: detailed data; the chamber's whip stays coarse.
    let ResponseBody::Place(panel) = send(
        &mut s,
        6,
        RequestBody::Query(QueryDto::Place { location: hall }),
    ) else {
        panic!("not a place view");
    };
    assert!(panel.here && panel.town_data.is_some() && panel.budget.is_some());
    assert!(office(&mut s, 7).town.detailed);
    send(
        &mut s,
        8,
        RequestBody::Command(CommandDto::Go {
            location: chamber,
            then: Some(Box::new(CommandDto::IntroducePolicy { policy: bill })),
        }),
    );
    let o = office(&mut s, 9);
    let b = o.bills.iter().find(|b| b.id == bill).unwrap();
    assert!(b.whip_exact && !b.whip.is_empty(), "exact in the chamber");
    assert!(!o.town.detailed);
    // Back home: "about N of M", no per-member odds.
    send(
        &mut s,
        10,
        RequestBody::Command(CommandDto::MoveTo {
            location: LocationDto::Home,
        }),
    );
    let o = office(&mut s, 11);
    let b = o.bills.iter().find(|b| b.id == bill).unwrap();
    assert!(!b.whip_exact && b.whip.is_empty());
    assert_eq!(b.expected_yes, b.expected_yes.round());
    // Away from the town hall its panel lists services but holds no data.
    let ResponseBody::Place(panel) = send(
        &mut s,
        12,
        RequestBody::Query(QueryDto::Place { location: hall }),
    ) else {
        panic!("not a place view");
    };
    assert!(!panel.here && panel.town_data.is_none());
    assert!(panel.services.iter().any(|x| x == "town_data_detailed"));
    // Voting from home is refused with a pointer to the chamber.
    let r: Response = serde_json::from_str(&s.handle_json(&req(
        13,
        RequestBody::Command(CommandDto::IntroducePolicy { policy: bill }),
    )))
    .unwrap();
    assert!(matches!(r.result, ResponseResult::Err { .. }));
}

#[test]
fn scenarios_are_listed_before_a_game_and_start_one() {
    // DEC-078 decision 4: the title screen lists the scenario registry.
    let mut s = Session::new().unwrap();
    let r: Response =
        serde_json::from_str(&s.handle_json(&req(1, RequestBody::Query(QueryDto::Scenarios))))
            .unwrap();
    let ResponseResult::Ok {
        body: ResponseBody::Scenarios(list),
    } = r.result
    else {
        panic!("expected the scenario list");
    };
    assert_eq!(list.scenarios.len(), 5);
    let coast = list
        .scenarios
        .iter()
        .find(|x| x.id == "scenario.wealthy_coast")
        .unwrap();
    assert_eq!(coast.name_key, "scenario-wealthy_coast");
    let start = |s: &mut Session, scenario: &str| -> Response {
        serde_json::from_str(&s.handle_json(&req(
            2,
            RequestBody::NewGame(NewGameParams {
                seed: 7,
                size: WorldSizeDto::Small,
                scenario: Some(scenario.into()),
                storyteller: None,
                local_knowledge: false,
            }),
        )))
        .unwrap()
    };
    assert!(matches!(
        start(&mut s, "scenario.atlantis").result,
        ResponseResult::Err { .. }
    ));
    assert!(matches!(
        start(&mut s, "scenario.wealthy_coast").result,
        ResponseResult::Ok {
            body: ResponseBody::GameStarted(_)
        }
    ));
    assert_eq!(
        s.engine().unwrap().state().meta.params.scenario.as_deref(),
        Some("scenario.wealthy_coast")
    );
}
