//! Platform-independent request handling used by the WASM adapter (and
//! testable natively). Owns the single authoritative engine (OVERVIEW §1).

use mandate_content::ContentBundle;
use mandate_protocol::*;
use mandate_save::{RunMode, SaveRequest};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate};
use std::sync::Arc;

pub struct Session {
    content: Arc<ContentBundle>,
    engine: Option<Engine>,
}

fn err(code: ErrorCode, message: impl Into<String>) -> ProtocolError {
    ProtocolError {
        code,
        message: message.into(),
    }
}

impl Session {
    /// Load the embedded content pack.
    pub fn new() -> Result<Self, String> {
        let sources = mandate_content::embedded::sources();
        let (content, _warnings) = mandate_content::load(&sources).map_err(|issues| {
            issues
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        })?;
        Ok(Session {
            content: Arc::new(content),
            engine: None,
        })
    }

    pub fn content(&self) -> &ContentBundle {
        &self.content
    }

    pub fn engine(&self) -> Option<&Engine> {
        self.engine.as_ref()
    }

    /// Handle a JSON request and return a JSON response. Never panics on bad input.
    pub fn handle_json(&mut self, request: &str) -> String {
        let response = match serde_json::from_str::<Request>(request) {
            Ok(req) => self.handle(req),
            Err(e) => Response {
                request_id: 0,
                result: ResponseResult::Err {
                    error: err(ErrorCode::BadRequest, e.to_string()),
                },
            },
        };
        serde_json::to_string(&response).unwrap_or_else(|e| {
            format!(r#"{{"request_id":0,"result":{{"status":"err","error":{{"code":"internal","message":"{e}"}}}}}}"#)
        })
    }

    pub fn handle(&mut self, req: Request) -> Response {
        let result = if req.protocol_version != PROTOCOL_VERSION {
            Err(err(
                ErrorCode::VersionMismatch,
                format!(
                    "client protocol v{} but engine speaks v{PROTOCOL_VERSION}",
                    req.protocol_version
                ),
            ))
        } else {
            self.dispatch(req.body)
        };
        Response {
            request_id: req.request_id,
            result: match result {
                Ok(body) => ResponseResult::Ok { body },
                Err(error) => ResponseResult::Err { error },
            },
        }
    }

    fn engine_mut(&mut self) -> Result<&mut Engine, ProtocolError> {
        self.engine
            .as_mut()
            .ok_or_else(|| err(ErrorCode::NoGame, "no game is running"))
    }

    fn dispatch(&mut self, body: RequestBody) -> Result<ResponseBody, ProtocolError> {
        match body {
            RequestBody::NewGame(p) => {
                let size = match p.size {
                    WorldSizeDto::Small => WorldSize::Small,
                    WorldSizeDto::Medium => WorldSize::Medium,
                    WorldSizeDto::Large => WorldSize::Large,
                };
                let engine = generate(&WorldConfig { seed: p.seed, size }, self.content.clone())
                    .map_err(|e| err(ErrorCode::Content, e.to_string()))?;
                let view = overview(&engine);
                self.engine = Some(engine);
                Ok(ResponseBody::GameStarted(view))
            }
            RequestBody::Command(c) => {
                let command = match c {
                    CommandDto::EndTurn => Command::EndTurn,
                    CommandDto::AdvanceDays { days } => Command::AdvanceDays { days },
                };
                let engine = self.engine_mut()?;
                let outcome = engine
                    .apply(command)
                    .map_err(|e| err(ErrorCode::InvalidCommand, e.to_string()))?;
                Ok(ResponseBody::CommandApplied(CommandAppliedView {
                    days_advanced: outcome.days_advanced,
                    overview: overview(engine),
                }))
            }
            RequestBody::Query(q) => match q {
                QueryDto::Overview => Ok(ResponseBody::Overview(overview(self.engine_mut()?))),
                QueryDto::Diagnostics => {
                    Ok(ResponseBody::Diagnostics(diagnostics(self.engine_mut()?)))
                }
                QueryDto::Locale { lang } => Ok(ResponseBody::Locale(LocaleView {
                    ftl: self.content.locale_text(&lang),
                    lang,
                })),
            },
        }
    }

    /// Encode the running game as a save file.
    pub fn export_save(&self, req: &SaveRequestDto) -> Result<Vec<u8>, String> {
        let engine = self.engine.as_ref().ok_or("no game is running")?;
        let r = SaveRequest {
            run_id: req.run_id.clone(),
            revision: req.revision.into(),
            parent_revision: req.parent_revision.map(u64::from),
            device_id: req.device_id.clone(),
            saved_at: req.saved_at.clone(),
            protocol_version: PROTOCOL_VERSION,
            run_mode: match req.run_mode {
                RunModeDto::Ironman => RunMode::Ironman,
                RunModeDto::Story => RunMode::Story,
            },
        };
        mandate_save::encode(engine.state(), &r)
            .map(|(b, _)| b)
            .map_err(|e| e.to_string())
    }

    /// Replace the running game with a decoded save; returns its overview.
    pub fn import_save(&mut self, bytes: &[u8]) -> Result<OverviewView, String> {
        let (state, _) = mandate_save::decode(bytes).map_err(|e| e.to_string())?;
        let engine = Engine::from_state(state, self.content.clone()).map_err(|e| e.to_string())?;
        let view = overview(&engine);
        self.engine = Some(engine);
        Ok(view)
    }
}

/// Read save metadata without loading the game.
pub fn read_save_meta(bytes: &[u8]) -> Result<SaveMetaDto, String> {
    let m = mandate_save::read_meta(bytes).map_err(|e| e.to_string())?;
    Ok(SaveMetaDto {
        format_version: m.format_version.into(),
        run_id: m.run_id,
        revision: m.revision as u32,
        parent_revision: m.parent_revision.map(|v| v as u32),
        device_id: m.device_id,
        saved_at: m.saved_at,
        engine_version: m.engine_version,
        content_hash: m.content_hash,
        protocol_version: m.protocol_version,
        run_mode: match m.run_mode {
            RunMode::Ironman => RunModeDto::Ironman,
            RunMode::Story => RunModeDto::Story,
        },
        seed: m.seed.to_string(),
        date: DateView::of(mandate_core::SimDate(m.sim_date)),
        turn: m.turn,
    })
}
