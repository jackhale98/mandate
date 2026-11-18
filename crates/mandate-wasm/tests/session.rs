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
            preset: None,
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
            preset: None,
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
