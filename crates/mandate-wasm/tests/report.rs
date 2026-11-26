#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Bug reports (`reportCore`, `replayReport`): the session keeps the run log
//! from the new game on, carries it through a save and a load, and a replay
//! of it reproduces the run's hashes.
use mandate_protocol::*;
use mandate_wasm::session::{ReportCore, Session};

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
        ResponseResult::Err { error } => panic!("{error:?}"),
    }
}

fn save(s: &Session) -> Vec<u8> {
    s.export_save(&SaveRequestDto {
        run_id: "ab".repeat(16),
        revision: 3,
        parent_revision: Some(2),
        device_id: "d".into(),
        saved_at: "2026-11-19T00:00:00Z".into(),
        run_mode: RunModeDto::Ironman,
    })
    .unwrap()
}

#[test]
fn the_run_log_follows_the_game_and_replays_to_its_hashes() {
    let mut s = Session::new().unwrap();
    let teller = s.content().storytellers.last().map(|t| t.id.clone());
    send(
        &mut s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed: 31337,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: teller.clone(),
            local_knowledge: true,
            recent_openings: Vec::new(),
        }),
    );
    send(
        &mut s,
        2,
        RequestBody::Command(CommandDto::Act {
            action: ActionDto::Rest,
            late_night: false,
        }),
    );
    send(&mut s, 3, RequestBody::Command(CommandDto::EndTurn));
    send(&mut s, 4, RequestBody::Command(CommandDto::NotebookSeen));
    send(&mut s, 5, RequestBody::Command(CommandDto::EndTurn));

    let core: ReportCore = serde_json::from_str(&s.report_core().unwrap()).unwrap();
    let hash = s.engine().unwrap().state_hash();
    assert_eq!(core.state_hash, hash);
    assert_eq!(core.turn, 2);
    let setup = core.log.new_game.clone().expect("replayable");
    assert_eq!(setup.seed, 31337);
    assert_eq!(setup.storyteller, teller);
    assert!(setup.local_knowledge);
    assert_eq!(core.log.commands.len(), 4);
    // The new world, then each turn.
    let turns: Vec<u32> = core.log.checkpoints.iter().map(|c| c.turn).collect();
    assert_eq!(turns, vec![0, 1, 2]);
    assert_eq!(
        core.log.checkpoints.last().map(|c| c.hash.clone()),
        Some(hash.clone())
    );

    // The replay (as `mandate-cli replay --wasm` runs it) matches.
    let report = serde_json::json!({ "format": "mandate-report", "log": core.log });
    let out: mandate_save::runlog::ReplayOutcome =
        serde_json::from_str(&s.replay_report(&report.to_string(), None).unwrap()).unwrap();
    assert_eq!(out.divergence, None);
    assert_eq!(out.state_hash, hash);
    assert_eq!(out.checkpoints.len(), 3);
    // The running game is untouched.
    assert_eq!(s.engine().unwrap().state_hash(), hash);

    // Saved and loaded, the run goes on with its whole log.
    let bytes = save(&s);
    let mut s2 = Session::new().unwrap();
    s2.import_save(&bytes).unwrap();
    send(&mut s2, 6, RequestBody::Command(CommandDto::EndTurn));
    let core2: ReportCore = serde_json::from_str(&s2.report_core().unwrap()).unwrap();
    assert_eq!(core2.log.commands.len(), 5);
    assert_eq!(core2.log.commands[..4], core.log.commands[..]);
    assert_eq!(core2.log.checkpoints.len(), 4);
    let report = serde_json::json!({ "log": core2.log });
    let out: mandate_save::runlog::ReplayOutcome =
        serde_json::from_str(&s2.replay_report(&report.to_string(), None).unwrap()).unwrap();
    assert_eq!(out.divergence, None);
    assert_eq!(out.state_hash, core2.state_hash);
}

#[test]
fn a_save_without_a_log_reports_why_it_cannot_be_replayed() {
    let mut s = Session::new().unwrap();
    send(
        &mut s,
        1,
        RequestBody::NewGame(NewGameParams {
            seed: 5,
            size: WorldSizeDto::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: false,
            recent_openings: Vec::new(),
        }),
    );
    // A save as written before the run log (no log in the payload).
    let st = s.engine().unwrap().state().clone();
    let (bytes, _) = mandate_save::encode(
        &st,
        &mandate_save::SaveRequest {
            run_id: "cd".repeat(16),
            revision: 1,
            parent_revision: None,
            device_id: "d".into(),
            saved_at: "2026-11-19T00:00:00Z".into(),
            protocol_version: PROTOCOL_VERSION,
            run_mode: mandate_save::RunMode::Story,
        },
    )
    .unwrap();
    let mut s2 = Session::new().unwrap();
    s2.import_save(&bytes).unwrap();
    send(&mut s2, 2, RequestBody::Command(CommandDto::EndTurn));
    let core: ReportCore = serde_json::from_str(&s2.report_core().unwrap()).unwrap();
    assert!(core.log.new_game.is_none());
    assert!(core.log.note.is_some());
    // What happened since the load is still logged.
    assert_eq!(core.log.commands.len(), 1);
    assert!(
        s2.replay_report(&serde_json::json!({ "log": core.log }).to_string(), None)
            .is_err()
    );
}
