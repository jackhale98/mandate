//! `mandate-cli replay <report.json>`: rebuild a tester's run from a bug
//! report (`web/src/components/report`) and check it against the report.
//!
//! The run is rebuilt natively from the report's seed, new-game setup and
//! command log (DEC-041: the same engine and content give the same run). It
//! checks the state hash at every checkpoint in the log (after the new game
//! and after every turn) and at the end, and that the bundled save decodes
//! and loads to the reported hash. A difference points at non-determinism,
//! a native/WASM difference (`--wasm` replays through the WASM build with
//! node as well), or a report made under other rules.

use mandate_content::ContentBundle;
use mandate_protocol::{DateView, LocationDto, PROTOCOL_VERSION};
use mandate_save::runlog::{CheckpointResult, ReplayOutcome, describe, replay};
use mandate_save::{NewGameSetup, RunLog};
use mandate_sim::Engine;
use mandate_worldgen::{WorldConfig, generate};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The report file's `format` field.
pub const REPORT_FORMAT: &str = "mandate-report";
/// The newest report layout this tool reads.
pub const REPORT_FORMAT_VERSION: u32 = 1;

/// The parts of a report file the replay reads (the rest is for people).
#[derive(Deserialize)]
pub struct Report {
    pub format: String,
    pub format_version: u32,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub engine: Option<ReportEngine>,
    #[serde(default)]
    pub state_hash: Option<String>,
    #[serde(default)]
    pub log: Option<RunLog>,
    #[serde(default)]
    pub save: Option<ReportSave>,
    #[serde(default)]
    pub errors: Vec<Value>,
}

#[derive(Deserialize)]
pub struct ReportEngine {
    pub engine_version: String,
    pub content_hash: String,
}

#[derive(Deserialize)]
pub struct ReportSave {
    /// `live` (the running game) or `autosave` (the latest stored save).
    #[serde(default)]
    pub source: Option<String>,
    pub base64: String,
}

pub struct Options {
    pub until: Option<u32>,
    pub out: Option<PathBuf>,
    pub dump: bool,
    pub wasm: bool,
}

pub fn read_report(path: &Path) -> Result<Report, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_report(&text)
}

pub fn parse_report(text: &str) -> Result<Report, String> {
    let r: Report = serde_json::from_str(text).map_err(|e| format!("not a report file: {e}"))?;
    if r.format != REPORT_FORMAT {
        return Err(format!("not a report file (format `{}`)", r.format));
    }
    if r.format_version > REPORT_FORMAT_VERSION {
        return Err(format!(
            "report format v{} is newer than this tool (v{REPORT_FORMAT_VERSION})",
            r.format_version
        ));
    }
    Ok(r)
}

/// A new game exactly as the browser session makes one (`session::start_game`).
pub fn start(content: Arc<ContentBundle>, setup: &NewGameSetup) -> Result<Engine, String> {
    let mut e = generate(
        &WorldConfig {
            seed: setup.seed,
            size: setup.size,
            scenario: setup.scenario.clone(),
        },
        content,
    )
    .map_err(|e| e.to_string())?;
    setup.prepare(&mut e).map_err(|e| e.to_string())?;
    Ok(e)
}

/// The result: a JSON summary, and whether everything matched.
pub struct Checked {
    pub summary: Value,
    pub ok: bool,
}

pub fn run(
    content: Arc<ContentBundle>,
    report_path: &Path,
    opts: &Options,
) -> Result<Checked, String> {
    let report = read_report(report_path)?;
    let mut ok = true;
    let mut out = serde_json::Map::new();
    let log = report.log.as_ref();
    let seed = log.and_then(|l| l.new_game.as_ref()).map(|s| s.seed);
    out.insert(
        "report".into(),
        json!({
            "file": report_path.display().to_string(),
            "created_at": report.created_at,
            "description": report.description,
            "seed": seed.map(|s| s.to_string()),
            "commands": log.map(|l| l.commands.len()),
            "checkpoints": log.map(|l| l.checkpoints.len()),
            "state_hash": report.state_hash,
            "errors": report.errors.len(),
        }),
    );

    // Rules: the replay is exact only under the engine and content the run
    // was played with.
    let (rep_engine, rep_content) = match (&report.engine, log) {
        (Some(e), _) => (Some(e.engine_version.clone()), Some(e.content_hash.clone())),
        (None, Some(l)) => (Some(l.engine_version.clone()), Some(l.content_hash.clone())),
        _ => (None, None),
    };
    let log_rules_match = log.is_none_or(|l| {
        l.engine_version == mandate_core::ENGINE_VERSION && l.content_hash == content.content_hash
    });
    out.insert(
        "rules".into(),
        json!({
            "report_engine": rep_engine,
            "report_content": rep_content,
            "local_engine": mandate_core::ENGINE_VERSION,
            "local_content": content.content_hash,
            "match": log_rules_match,
        }),
    );

    // The bundled save: it decodes, its state hashes to the report's, and it
    // loads (as the browser would) to the same hash.
    out.insert("save".into(), check_save(&report, &content, &mut ok));

    // The replay itself.
    let Some(log) = log else {
        out.insert(
            "native".into(),
            json!({ "skipped": "the report has no run log" }),
        );
        return Ok(Checked {
            summary: Value::Object(out),
            ok: false,
        });
    };
    let Some(setup) = &log.new_game else {
        out.insert(
            "native".into(),
            json!({ "skipped": log.note.clone().unwrap_or_else(|| "the run cannot be replayed from its start".into()) }),
        );
        return Ok(Checked {
            summary: Value::Object(out),
            ok: false,
        });
    };
    let mut engine = start(content.clone(), setup)?;
    let native = replay(&mut engine, log, opts.until);
    let native_summary = summarize(&native, log, &report, opts.until);
    ok &= native_summary["ok"].as_bool().unwrap_or(false);
    out.insert("native".into(), native_summary);

    if opts.wasm {
        let wasm = replay_wasm(report_path, opts.until)?;
        let wasm_summary = summarize(&wasm, log, &report, opts.until);
        ok &= wasm_summary["ok"].as_bool().unwrap_or(false);
        let parity = parity(&native, &wasm);
        ok &= parity["match"].as_bool().unwrap_or(false);
        out.insert("wasm".into(), wasm_summary);
        out.insert("native_vs_wasm".into(), parity);
    }

    if let Some(n) = opts.until {
        let path = opts.out.clone().unwrap_or_else(|| {
            PathBuf::from(format!("mandate-replay-{}-turn{n}.mandate", setup.seed))
        });
        // The log so far goes in the save, so the replay can go on from it.
        let mut part = log.clone();
        part.commands.truncate(native.commands_applied as usize);
        part.checkpoints
            .retain(|c| c.commands <= native.commands_applied);
        write_save(&engine, &part, &path)?;
        out.insert("wrote".into(), json!(path.display().to_string()));
    }
    if opts.dump {
        out.insert("dump".into(), dump(&engine));
    }
    out.insert("ok".into(), json!(ok));
    Ok(Checked {
        summary: Value::Object(out),
        ok,
    })
}

fn check_save(report: &Report, content: &Arc<ContentBundle>, ok: &mut bool) -> Value {
    let Some(save) = &report.save else {
        return json!({ "present": false });
    };
    let bytes = match base64_decode(&save.base64) {
        Ok(b) => b,
        Err(e) => {
            *ok = false;
            return json!({ "present": true, "error": e });
        }
    };
    let meta = mandate_save::read_meta(&bytes).ok();
    let (state, log, _) = match mandate_save::decode_with_log(&bytes) {
        Ok(x) => x,
        Err(e) => {
            *ok = false;
            return json!({ "present": true, "bytes": bytes.len(), "error": e.to_string() });
        }
    };
    let decoded = state.hash();
    let turn = state.calendar.turn;
    let loaded = Engine::from_state(state, content.clone()).map(|e| e.state_hash());
    let matches = report.state_hash.as_deref() == Some(decoded.as_str());
    let loads_same = loaded.as_deref().ok() == Some(decoded.as_str());
    let log_matches = match (&log, &report.log) {
        (Some(a), Some(b)) => Some(a == b),
        _ => None,
    };
    *ok &= matches && loads_same && log_matches != Some(false);
    json!({
        "present": true,
        "source": save.source,
        "bytes": bytes.len(),
        "format_version": meta.as_ref().map(|m| m.format_version),
        "turn": turn,
        "state_hash": decoded,
        "matches_report": matches,
        "loads": loaded.is_ok(),
        "load_error": loaded.as_ref().err().map(|e| e.to_string()),
        "loaded_hash_matches": loads_same,
        "log_matches_report": log_matches,
    })
}

/// A replay's result as the report reads: what matched and where it parted.
fn summarize(r: &ReplayOutcome, log: &RunLog, report: &Report, until: Option<u32>) -> Value {
    let matched = r.checkpoints.iter().filter(|c| c.matches()).count();
    let complete = r.commands_applied == r.commands_total && r.divergence.is_none();
    // The end of the log is the state the report was made at.
    let final_match = if complete && until.is_none() {
        report.state_hash.as_deref().map(|h| h == r.state_hash)
    } else {
        None
    };
    let ok = r.divergence.is_none() && final_match != Some(false) && !r.gaps;
    let divergence = r.divergence.as_ref().map(|d| {
        let commands: Vec<Value> = log
            .commands
            .iter()
            .filter(|c| c.index >= d.commands_from && c.index < d.commands_to)
            .map(|c| json!({ "index": c.index, "command": describe(&c.command) }))
            .collect();
        json!({
            "turn": d.at.turn,
            "after_commands": d.at.commands,
            "last_good_turn": d.last_good_turn,
            "expected": d.at.expected,
            "actual": d.at.actual,
            "suspects": { "from": d.commands_from, "to": d.commands_to, "commands": commands },
            "explain": match d.last_good_turn {
                None => "the new world itself differs (world generation or new-game setup)".to_string(),
                Some(t) => format!(
                    "the state matched after turn {t} and differs after turn {}: one of log entries {}..{} (or the turn they end) diverged",
                    d.at.turn, d.commands_from, d.commands_to
                ),
            },
        })
    });
    json!({
        "ok": ok,
        "commands_applied": r.commands_applied,
        "commands_total": r.commands_total,
        "stopped_early": r.stopped_early,
        "turn": r.turn,
        "state_hash": r.state_hash,
        "checkpoints_checked": r.checkpoints.len(),
        "checkpoints_matched": matched,
        "final_hash_matches_report": final_match,
        "first_divergence": divergence,
        "refused_commands": r.refusals,
        "log_has_gaps": r.gaps,
    })
}

/// The first checkpoint where the native and WASM replays differ.
fn parity(native: &ReplayOutcome, wasm: &ReplayOutcome) -> Value {
    let first = native
        .checkpoints
        .iter()
        .zip(&wasm.checkpoints)
        .find(|(a, b)| a.actual != b.actual)
        .map(|(a, b): (&CheckpointResult, &CheckpointResult)| {
            json!({ "turn": a.turn, "after_commands": a.commands, "native": a.actual, "wasm": b.actual })
        });
    let same_end =
        native.state_hash == wasm.state_hash && native.commands_applied == wasm.commands_applied;
    json!({
        "match": first.is_none() && same_end,
        "first_difference": first,
        "native_hash": native.state_hash,
        "wasm_hash": wasm.state_hash,
    })
}

/// Replay through the WASM build with node (`scripts/wasm-replay.mjs`;
/// `MANDATE_WASM_NODE` picks the build, as for `wasm-parity.mjs`).
fn replay_wasm(report: &Path, until: Option<u32>) -> Result<ReplayOutcome, String> {
    let local = Path::new("scripts/wasm-replay.mjs");
    let script = if local.exists() {
        local.to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/wasm-replay.mjs")
    };
    let mut cmd = std::process::Command::new("node");
    cmd.arg(&script).arg(report);
    if let Some(n) = until {
        cmd.arg("--until").arg(n.to_string());
    }
    let res = cmd
        .output()
        .map_err(|e| format!("running node {}: {e}", script.display()))?;
    if !res.status.success() {
        return Err(format!(
            "the WASM replay failed: {}",
            String::from_utf8_lossy(&res.stderr).trim()
        ));
    }
    serde_json::from_slice(&res.stdout).map_err(|e| format!("the WASM replay's output: {e}"))
}

/// The player's views where the replay stopped (pocket, office, the place
/// they are at), as the browser would show them.
fn dump(engine: &Engine) -> Value {
    let st = engine.state();
    let place = st
        .player
        .as_ref()
        .and_then(|p| mandate_protocol::place_panel_view(engine, LocationDto::of(p.location)));
    json!({
        "turn": st.calendar.turn,
        "date": DateView::of(st.calendar.date),
        "state_hash": engine.state_hash(),
        "pocket": mandate_protocol::pocket_view(engine),
        "office": mandate_protocol::office_view(engine),
        "place": place,
    })
}

/// Write the replayed state: a `.mandate` run file (importable on the
/// title screen, DEC-082) or, for any other extension, the raw engine save.
fn write_save(engine: &Engine, log: &RunLog, path: &Path) -> Result<(), String> {
    let st = engine.state();
    let run_id = mandate_core::hash::StableHash::of_bytes(
        format!("replay:{}:{}", st.meta.seed, engine.state_hash()).as_bytes(),
    )
    .hex()
    .chars()
    .take(32)
    .collect::<String>();
    let saved_at = "1970-01-01T00:00:00Z";
    let req = mandate_save::SaveRequest {
        run_id: run_id.clone(),
        revision: 1,
        parent_revision: None,
        device_id: "cli-replay".into(),
        saved_at: saved_at.into(),
        protocol_version: PROTOCOL_VERSION,
        run_mode: mandate_save::RunMode::Story,
    };
    let (bytes, meta) =
        mandate_save::encode_with_log(st, Some(log), &req).map_err(|e| e.to_string())?;
    let data = if path.extension().is_some_and(|e| e == "mandate") {
        let header = json!({
            "format": 1,
            "appVersion": "mandate-cli replay",
            "exportedAt": saved_at,
            "run": {
                "runId": run_id,
                "mode": "story",
                "status": "active",
                "revision": 1,
                "name": Value::Null,
                "savedAt": saved_at,
                "saveFormatVersion": meta.format_version,
                "engineVersion": meta.engine_version,
                "contentHash": meta.content_hash,
                "turn": meta.turn,
                "date": DateView::of(st.calendar.date),
            },
            "save": { "length": bytes.len(), "crc32": format!("{:08x}", crc32(&bytes)) },
        });
        format!(
            "MANDATE RUN EXPORT\n{}\n{}\n",
            serde_json::to_string(&header).map_err(|e| e.to_string())?,
            base64_encode(&bytes)
        )
        .into_bytes()
    } else {
        bytes
    };
    std::fs::write(path, data).map_err(|e| format!("{}: {e}", path.display()))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for &b in bytes {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn base64_decode(text: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut acc, mut bits) = (0u32, 0u32);
    for ch in text.bytes() {
        let v = match ch {
            b'A'..=b'Z' => ch - b'A',
            b'a'..=b'z' => ch - b'a' + 26,
            b'0'..=b'9' => ch - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' | b' ' => continue,
            _ => return Err("the save is not base64".into()),
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits & 0xff) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use mandate_sim::Command;
    use mandate_sim::state::WorldSize;

    fn content() -> Arc<ContentBundle> {
        crate::load_content(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")).unwrap()
    }

    #[test]
    fn base64_round_trips() {
        for n in 0..10usize {
            let b: Vec<u8> = (0..n as u8).map(|i| i.wrapping_mul(37)).collect();
            assert_eq!(base64_decode(&base64_encode(&b)).unwrap(), b);
        }
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    /// A report as the browser writes one (`buildReport.ts`), made natively:
    /// a bot plays a few turns (canvassing, talking, …).
    fn report(content: &Arc<ContentBundle>, turns: u32) -> (Value, RunLog) {
        let setup = NewGameSetup {
            seed: 4242,
            size: WorldSize::Small,
            scenario: None,
            storyteller: None,
            local_knowledge: true,
        };
        let mut e = start(content.clone(), &setup).unwrap();
        let mut log = RunLog::start(setup.clone(), &e);
        for _ in 0..turns {
            crate::bots::play_turn_from(&mut e, content, "mixed", 0);
            e.apply(Command::EndTurn).unwrap();
            log.checkpoint(&e);
        }
        let log = log.with_commands(&e);
        let req = mandate_save::SaveRequest {
            run_id: "ab".repeat(16),
            revision: 3,
            parent_revision: Some(2),
            device_id: "d".into(),
            saved_at: "2026-11-19T00:00:00Z".into(),
            protocol_version: PROTOCOL_VERSION,
            run_mode: mandate_save::RunMode::Ironman,
        };
        let (bytes, _) = mandate_save::encode_with_log(e.state(), Some(&log), &req).unwrap();
        let v = json!({
            "format": REPORT_FORMAT,
            "format_version": 1,
            "created_at": "2026-11-19T00:00:00Z",
            "description": "test",
            "engine": { "engine_version": mandate_core::ENGINE_VERSION, "content_hash": content.content_hash },
            "run": { "seed": setup.seed.to_string(), "new_game": setup },
            "state_hash": e.state_hash(),
            "log": log,
            "save": { "source": "live", "base64": base64_encode(&bytes) },
            "errors": [],
            "device": { "viewport": { "width": 1, "height": 1 }, "dpr": 1, "user_agent": "test" },
        });
        (v, log)
    }

    fn check(content: &Arc<ContentBundle>, v: &Value, until: Option<u32>) -> Checked {
        let dir = std::env::temp_dir().join(format!(
            "mandate-replay-test-{}-{}",
            std::process::id(),
            until.unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("report.json");
        std::fs::write(&path, serde_json::to_string(v).unwrap()).unwrap();
        let out = dir.join("part.mandate");
        let r = run(
            content.clone(),
            &path,
            &Options {
                until,
                out: Some(out.clone()),
                dump: until.is_some(),
                wasm: false,
            },
        )
        .unwrap();
        if until.is_some() {
            let text = std::fs::read_to_string(&out).unwrap();
            assert!(text.starts_with("MANDATE RUN EXPORT\n"));
        }
        let _ = std::fs::remove_dir_all(&dir);
        r
    }

    #[test]
    fn a_report_replays_to_its_hashes_and_a_tampered_command_is_found() {
        let c = content();
        let (v, log) = report(&c, 4);
        assert!(log.commands.len() > 4, "the bot acted");
        assert_eq!(log.checkpoints.len(), 5);

        let r = check(&c, &v, None);
        assert!(r.ok, "{:#}", r.summary);
        assert_eq!(r.summary["native"]["checkpoints_matched"], 5);
        assert_eq!(r.summary["native"]["final_hash_matches_report"], true);
        assert_eq!(r.summary["save"]["matches_report"], true);
        assert_eq!(r.summary["save"]["loaded_hash_matches"], true);
        assert_eq!(r.summary["save"]["log_matches_report"], true);

        // Stop at turn 2 and write a save there; the views are dumped.
        let r = check(&c, &v, Some(2));
        assert!(r.ok, "{:#}", r.summary);
        assert_eq!(r.summary["native"]["turn"], 2);
        assert_eq!(r.summary["native"]["stopped_early"], true);
        assert!(r.summary["dump"]["pocket"].is_object());

        // Plant a tampered command in turn 3 (after the second checkpoint
        // past the start): the replay names that turn and the entries in it.
        let from = log.checkpoints[2].commands;
        let to = log.checkpoints[3].commands;
        let planted = from + (to - from) / 2;
        let mut bad = v.clone();
        bad["log"]["commands"][planted as usize]["command"] =
            json!({ "type": "advance_days", "days": 1 });
        let r = check(&c, &bad, None);
        assert!(!r.ok);
        let d = &r.summary["native"]["first_divergence"];
        assert_eq!(d["turn"], 3, "{d:#}");
        assert_eq!(d["last_good_turn"], 2);
        assert_eq!(d["suspects"]["from"], from);
        assert_eq!(d["suspects"]["to"], to);
        assert!((from..to).contains(&planted));
        // The bundled save is still the run as played.
        assert_eq!(r.summary["save"]["matches_report"], true);
        assert_eq!(r.summary["save"]["log_matches_report"], false);

        // A different seed diverges at the new world itself.
        let mut other = v.clone();
        other["log"]["new_game"]["seed"] = json!("4243");
        let r = check(&c, &other, None);
        assert!(!r.ok);
        assert_eq!(r.summary["native"]["first_divergence"]["turn"], 0);
        assert_eq!(
            r.summary["native"]["first_divergence"]["last_good_turn"],
            Value::Null
        );
    }
}
