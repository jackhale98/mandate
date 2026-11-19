//! Platform-independent request handling used by the WASM adapter (and
//! testable natively). Owns the single authoritative engine (OVERVIEW §1).

use mandate_content::ContentBundle;
use mandate_protocol::*;
use mandate_save::{RunMode, SaveRequest};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate_with_progress, rebuild_terrain};
use std::sync::Arc;

/// Progress callback: (Fluent stage key, fraction 0..1).
pub type ProgressFn = Box<dyn FnMut(&str, f64)>;

pub struct Session {
    content: Arc<ContentBundle>,
    engine: Option<Engine>,
    /// Called with (stage key, fraction) during long operations.
    progress: Option<ProgressFn>,
    /// `LayoutContentView`, built once per content hash.
    layout: Option<LayoutContentView>,
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
            progress: None,
            layout: None,
        })
    }

    pub fn set_progress(&mut self, f: ProgressFn) {
        self.progress = Some(f);
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
                if let Some(k) = &p.scenario
                    && self.content.scenario(k).is_none()
                {
                    return Err(err(
                        ErrorCode::BadRequest,
                        format!("unknown scenario `{k}`"),
                    ));
                }
                if let Some(k) = &p.storyteller
                    && self.content.storyteller(k).is_none()
                {
                    return Err(err(
                        ErrorCode::BadRequest,
                        format!("unknown storyteller `{k}`"),
                    ));
                }
                let mut noop = |_: &str, _: f64| {};
                let progress: &mut dyn FnMut(&str, f64) = match self.progress.as_mut() {
                    Some(f) => f.as_mut(),
                    None => &mut noop,
                };
                let engine = generate_with_progress(
                    &WorldConfig {
                        seed: p.seed,
                        size,
                        scenario: p.scenario.clone(),
                    },
                    self.content.clone(),
                    progress,
                )
                .map_err(|e| err(ErrorCode::Content, e.to_string()))?;
                let mut engine = engine;
                if let Some(k) = &p.storyteller {
                    // The run's storyteller (DEC-033): chosen before play, kept in the save.
                    engine.setup(|s| s.narrative.profile = k.clone());
                }
                let view = overview(&engine);
                self.engine = Some(engine);
                Ok(ResponseBody::GameStarted(view))
            }
            RequestBody::Command(c) => {
                let command = match c {
                    CommandDto::EndTurn => Command::EndTurn,
                    CommandDto::AdvanceDays { days } => Command::AdvanceDays { days },
                    other => Command::Player(player_command(other)?),
                };
                let engine = self.engine_mut()?;
                let outcome = engine
                    .apply(command)
                    .map_err(|e| err(ErrorCode::InvalidCommand, e.to_string()))?;
                Ok(ResponseBody::CommandApplied(CommandAppliedView {
                    days_advanced: outcome.days_advanced,
                    overview: overview(engine),
                    result: outcome.result.map(|r| ActionResultView {
                        created: r.created,
                        outcome_key: r.outcome_key,
                        args: r.args,
                    }),
                }))
            }
            RequestBody::Query(q) => match q {
                QueryDto::Overview => Ok(ResponseBody::Overview(overview(self.engine_mut()?))),
                QueryDto::Office => office_view(self.engine_mut()?)
                    .map(ResponseBody::Office)
                    .ok_or_else(|| err(ErrorCode::NoGame, "no player in this game")),
                QueryDto::Campaign => campaign_view(self.engine_mut()?)
                    .map(ResponseBody::Campaign)
                    .ok_or_else(|| err(ErrorCode::NoGame, "no player in this game")),
                QueryDto::Diagnostics => {
                    Ok(ResponseBody::Diagnostics(diagnostics(self.engine_mut()?)))
                }
                QueryDto::Map => map_view(self.engine_mut()?)
                    .map(ResponseBody::Map)
                    .ok_or_else(|| err(ErrorCode::Internal, "terrain unavailable")),
                QueryDto::Municipality { id } => municipality_view(self.engine_mut()?, id)
                    .map(ResponseBody::Municipality)
                    .ok_or_else(|| err(ErrorCode::BadRequest, format!("no municipality {id}"))),
                QueryDto::Pocket => pocket_view(self.engine_mut()?)
                    .map(ResponseBody::Pocket)
                    .ok_or_else(|| err(ErrorCode::NoGame, "no player in this game")),
                QueryDto::Town => town_places_view(self.engine_mut()?)
                    .map(ResponseBody::Town)
                    .ok_or_else(|| err(ErrorCode::NoGame, "no player in this game")),
                QueryDto::Place { location } => place_panel_view(self.engine_mut()?, location)
                    .map(ResponseBody::Place)
                    .ok_or_else(|| err(ErrorCode::BadRequest, "no such place in your town")),
                QueryDto::Character { id } => character_view(self.engine_mut()?, id)
                    .map(ResponseBody::Character)
                    .ok_or_else(|| err(ErrorCode::BadRequest, format!("no character {id}"))),
                QueryDto::Decision { id } => decision_by_id(self.engine_mut()?, id)
                    .map(ResponseBody::Decision)
                    .ok_or_else(|| err(ErrorCode::BadRequest, format!("no decision {id}"))),
                QueryDto::Story => Ok(ResponseBody::Story(story_view(self.engine_mut()?))),
                QueryDto::Newspaper => newspaper_view(self.engine_mut()?, 24)
                    .map(ResponseBody::Newspaper)
                    .ok_or_else(|| err(ErrorCode::NoGame, "no player in this game")),
                QueryDto::Scenarios => Ok(ResponseBody::Scenarios(scenario_list(&self.content))),
                QueryDto::Storytellers => {
                    Ok(ResponseBody::Storytellers(storyteller_list(&self.content)))
                }
                QueryDto::PolicyPreview { template, params } => {
                    policy_preview(self.engine_mut()?, &template, &params)
                        .map(ResponseBody::PolicyPreview)
                        .ok_or_else(|| {
                            err(
                                ErrorCode::BadRequest,
                                format!("no policy `{template}` to preview here"),
                            )
                        })
                }
                QueryDto::Locale { lang } => Ok(ResponseBody::Locale(LocaleView {
                    ftl: self.content.locale_text(&lang),
                    lang,
                })),
                QueryDto::LayoutContent => {
                    let hash = &self.content.content_hash;
                    if self.layout.as_ref().is_none_or(|v| &v.content_hash != hash) {
                        self.layout = Some(layout_content(&self.content));
                    }
                    Ok(ResponseBody::LayoutContent(
                        self.layout
                            .clone()
                            .unwrap_or_else(|| layout_content(&self.content)),
                    ))
                }
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
        // The terrain raster is derived from the seed (DEC-044 rev. 2 keeps saves small).
        let (terrain, _matches) =
            rebuild_terrain(&state, &self.content).map_err(|e| e.to_string())?;
        let mut engine =
            Engine::from_state(state, self.content.clone()).map_err(|e| e.to_string())?;
        engine.set_terrain(Arc::new(terrain));
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

fn params(v: Vec<ParamValue>) -> std::collections::BTreeMap<String, f64> {
    v.into_iter().map(|p| (p.id, p.value)).collect()
}

/// Protocol DTO → simulation command (the protocol never exposes lab commands).
fn player_command(c: CommandDto) -> Result<mandate_sim::actions::PlayerCommand, ProtocolError> {
    use mandate_sim::actions::{Action, PlayerCommand as P};
    use mandate_sim::policy::Clause;
    use mandate_sim::world::ConstituencyId;
    Ok(match c {
        CommandDto::MoveTo { location } => P::MoveTo {
            location: location.to_sim(),
        },
        CommandDto::Meet { slot } => P::Meet { slot },
        CommandDto::AnswerDecision { decision, option } => P::Answer { decision, option },
        CommandDto::Go { location, then } => P::Go {
            location: location.to_sim(),
            then: match then {
                Some(c) => Some(Box::new(player_command(*c)?)),
                None => None,
            },
        },
        CommandDto::DraftPolicy {
            template,
            params: p,
        } => P::DraftPolicy {
            template,
            params: params(p),
        },
        CommandDto::AmendPolicy {
            policy,
            params: p,
            clauses,
        } => P::AmendPolicy {
            policy,
            params: params(p),
            clauses: clauses
                .into_iter()
                .map(|c| match c {
                    ClauseDto::Earmark { ward } => Clause::Earmark {
                        ward: ConstituencyId(ward),
                    },
                    ClauseDto::Sunset { years } => Clause::Sunset {
                        years: years.min(255) as u8,
                    },
                    ClauseDto::Pilot { neighborhood } => Clause::Pilot { neighborhood },
                })
                .collect(),
        },
        CommandDto::IntroducePolicy { policy } => P::IntroducePolicy { policy },
        CommandDto::CastVote { policy, vote } => P::CastVote {
            policy,
            vote: match vote {
                VoteDto::Yes => mandate_sim::politics::Vote::Yes,
                VoteDto::No => mandate_sim::politics::Vote::No,
                VoteDto::Abstain => mandate_sim::politics::Vote::Abstain,
            },
        },
        CommandDto::File { filing } => P::File {
            filing: match filing {
                FilingDto::Council => mandate_sim::politics::elections::Filing::Council,
                FilingDto::Mayor => mandate_sim::politics::elections::Filing::Mayor,
                FilingDto::Retire => mandate_sim::politics::elections::Filing::Retire,
            },
        },
        CommandDto::WithdrawPolicy { policy } => P::WithdrawPolicy { policy },
        CommandDto::Act { action, late_night } => P::Act {
            late_night,
            action: match action {
                ActionDto::Lobby { politician, policy } => Action::Lobby { politician, policy },
                ActionDto::TradeFavor { politician, policy } => {
                    Action::TradeFavor { politician, policy }
                }
                ActionDto::TownHall {
                    neighborhood,
                    policy,
                } => Action::TownHall {
                    neighborhood,
                    policy,
                },
                ActionDto::Press { policy } => Action::Press { policy },
                ActionDto::Canvass { neighborhood } => Action::Canvass { neighborhood },
                ActionDto::Fundraise => Action::Fundraise,
                ActionDto::Rest => Action::Rest,
                ActionDto::Advertise => Action::Advertise,
                ActionDto::TakeBribe => Action::TakeBribe,
                ActionDto::DipFunds => Action::DipFunds,
                ActionDto::LeakFile => Action::LeakFile,
            },
        },
        // EndTurn and AdvanceDays are handled before this conversion; they
        // cannot follow a move.
        CommandDto::EndTurn | CommandDto::AdvanceDays { .. } => {
            return Err(err(
                ErrorCode::InvalidCommand,
                "time cannot pass as part of a move",
            ));
        }
    })
}
