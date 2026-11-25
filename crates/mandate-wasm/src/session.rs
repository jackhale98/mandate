//! Platform-independent request handling used by the WASM adapter (and
//! testable natively). Owns the single authoritative engine (OVERVIEW §1).

use mandate_content::ContentBundle;
use mandate_protocol::*;
use mandate_save::runlog::{ReplayOutcome, replay};
use mandate_save::{NewGameSetup, RunLog, RunMode, SaveRequest};
use mandate_sim::{Command, Engine};
use mandate_worldgen::{WorldConfig, WorldSize, generate_with_progress, rebuild_terrain};
use serde::{Deserialize, Serialize};
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
    /// The running game's run log (bug reports), without its commands:
    /// those are the engine's own log.
    log: Option<RunLog>,
}

/// What the engine contributes to a bug report (`reportCore`): the versions,
/// the state hash now, and the run log with every command since the start.
/// The client adds the save, recent errors, device facts and the tester's
/// description (`web/src/components/report`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportCore {
    pub engine_version: String,
    pub content_hash: String,
    pub protocol_version: u32,
    pub save_format_version: u16,
    pub state_hash: String,
    pub turn: u32,
    pub date: DateView,
    pub log: RunLog,
}

/// The part of a report file a replay reads.
#[derive(Deserialize)]
struct ReportLog {
    log: RunLog,
}

/// Generate a new game's world and apply its setup: the one path to a new
/// game, for players and for replays (`replay_report`, `mandate-cli replay`
/// does the same natively).
pub fn start_game(
    content: &Arc<ContentBundle>,
    setup: &NewGameSetup,
    progress: &mut dyn FnMut(&str, f64),
) -> Result<Engine, ProtocolError> {
    if let Some(k) = &setup.scenario
        && content.scenario(k).is_none()
    {
        return Err(err(
            ErrorCode::BadRequest,
            format!("unknown scenario `{k}`"),
        ));
    }
    if let Some(k) = &setup.storyteller
        && content.storyteller(k).is_none()
    {
        return Err(err(
            ErrorCode::BadRequest,
            format!("unknown storyteller `{k}`"),
        ));
    }
    let mut engine = generate_with_progress(
        &WorldConfig {
            seed: setup.seed,
            size: setup.size,
            scenario: setup.scenario.clone(),
        },
        content.clone(),
        progress,
    )
    .map_err(|e| err(ErrorCode::Content, e.to_string()))?;
    // The storyteller (DEC-033) and "Start with local knowledge" (DEC-091),
    // chosen before play and kept in the save.
    setup
        .prepare(&mut engine)
        .map_err(|e| err(ErrorCode::Content, e.to_string()))?;
    Ok(engine)
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
            log: None,
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
                let setup = NewGameSetup {
                    seed: p.seed,
                    size: match p.size {
                        WorldSizeDto::Small => WorldSize::Small,
                        WorldSizeDto::Medium => WorldSize::Medium,
                        WorldSizeDto::Large => WorldSize::Large,
                    },
                    scenario: p.scenario,
                    storyteller: p.storyteller,
                    local_knowledge: p.local_knowledge,
                };
                let mut noop = |_: &str, _: f64| {};
                let progress: &mut dyn FnMut(&str, f64) = match self.progress.as_mut() {
                    Some(f) => f.as_mut(),
                    None => &mut noop,
                };
                let engine = start_game(&self.content, &setup, progress)?;
                let view = overview(&engine);
                self.log = Some(RunLog::start(setup, &engine));
                self.engine = Some(engine);
                Ok(ResponseBody::GameStarted(view))
            }
            RequestBody::Command(c) => {
                let command = match c {
                    CommandDto::EndTurn => Command::EndTurn,
                    CommandDto::AdvanceDays { days } => Command::AdvanceDays { days },
                    CommandDto::NotebookSeen => Command::NotebookSeen,
                    CommandDto::SetDirectives { choices } => Command::SetDirectives {
                        choices: choices
                            .into_iter()
                            .map(|c| (c.directive, c.option))
                            .collect(),
                    },
                    CommandDto::Skip { target, limit } => Command::Skip {
                        target: target.to_content(),
                        // At most a year a request: the client asks again.
                        limit: Some(limit.unwrap_or(26).clamp(1, 26)),
                    },
                    CommandDto::CancelSkip => Command::CancelSkip,
                    other => Command::Player(player_command(self.engine.as_ref(), other)?),
                };
                let engine = self
                    .engine
                    .as_mut()
                    .ok_or_else(|| err(ErrorCode::NoGame, "no game is running"))?;
                let outcome = engine
                    .apply(command)
                    .map_err(|e| err(ErrorCode::InvalidCommand, e.to_string()))?;
                // A checkpoint for the bug-report replay after time passes.
                if outcome.days_advanced > 0
                    && let Some(log) = self.log.as_mut()
                {
                    log.checkpoint(engine);
                }
                Ok(ResponseBody::CommandApplied(CommandAppliedView {
                    days_advanced: outcome.days_advanced,
                    overview: overview(engine),
                    result: outcome.result.map(|mut r| {
                        // An ask's own commands name the ask: its text needs its arguments.
                        let about_ask = ["outcome-ask-", "outcome-promised", "outcome-casework-"]
                            .iter()
                            .any(|p| r.outcome_key.starts_with(p));
                        if about_ask && let Some(id) = r.created {
                            outcome_args(engine, id, &mut r.args);
                        }
                        ActionResultView {
                            created: r.created,
                            outcome_key: r.outcome_key,
                            args: r.args,
                        }
                    }),
                    skip: outcome.skip.as_ref().map(SkipProgressView::of),
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
                QueryDto::MapLayers => {
                    Ok(ResponseBody::MapLayers(map_layers_view(self.engine_mut()?)))
                }
                QueryDto::Municipality { id } => municipality_view(self.engine_mut()?, id)
                    .map(ResponseBody::Municipality)
                    .ok_or_else(|| err(ErrorCode::BadRequest, format!("no municipality {id}"))),
                QueryDto::Pocket => pocket_view(self.engine_mut()?)
                    .map(ResponseBody::Pocket)
                    .ok_or_else(|| err(ErrorCode::NoGame, "no player in this game")),
                QueryDto::Town => town_places_view(self.engine_mut()?)
                    .map(ResponseBody::Town)
                    .ok_or_else(|| err(ErrorCode::NoGame, "no player in this game")),
                QueryDto::TownMapLayers => town_map_layers_view(self.engine_mut()?)
                    .map(ResponseBody::TownMapLayers)
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
                QueryDto::TownGeography { id } => town_geography_view(self.engine_mut()?, id)
                    .map(ResponseBody::TownGeography)
                    .ok_or_else(|| err(ErrorCode::BadRequest, format!("no municipality {id}"))),
                QueryDto::TownState { id } => town_state_view(self.engine_mut()?, id)
                    .map(ResponseBody::TownState)
                    .ok_or_else(|| err(ErrorCode::BadRequest, format!("no municipality {id}"))),
                QueryDto::Notebook => notebook_view(self.engine_mut()?)
                    .map(ResponseBody::Notebook)
                    .ok_or_else(|| err(ErrorCode::NoGame, "no player in this game")),
                QueryDto::Talk { to, late_night } => talk_view(self.engine_mut()?, to, late_night)
                    .map(ResponseBody::Talk)
                    .ok_or_else(|| err(ErrorCode::BadRequest, "nobody like that to talk to")),
                QueryDto::Exchange { id } => exchange_view(self.engine_mut()?, id)
                    .map(ResponseBody::Exchange)
                    .ok_or_else(|| err(ErrorCode::BadRequest, format!("no exchange {id}"))),
                QueryDto::TermReview => Ok(ResponseBody::TermReview(term_review_view(
                    self.engine_mut()?,
                ))),
                QueryDto::SkipSummary => Ok(ResponseBody::SkipSummary(skip_summary_view(
                    self.engine_mut()?,
                ))),
                QueryDto::Legacy => Ok(ResponseBody::Legacy(legacy_view(self.engine_mut()?))),
                QueryDto::Asks => Ok(ResponseBody::Asks(asks_view(self.engine_mut()?))),
                QueryDto::Why { event, depth } => why_view(self.engine_mut()?, event, depth)
                    .map(ResponseBody::Why)
                    .ok_or_else(|| err(ErrorCode::BadRequest, format!("no event {event}"))),
                QueryDto::PromiseMenu => Ok(ResponseBody::PromiseMenu(promise_menu_view(
                    self.engine_mut()?,
                ))),
                QueryDto::People => people_view(self.engine_mut()?)
                    .map(ResponseBody::People)
                    .ok_or_else(|| err(ErrorCode::NoGame, "no player in this game")),
                QueryDto::Arc { id } => arc_view(self.engine_mut()?, id)
                    .map(ResponseBody::Arc)
                    .ok_or_else(|| err(ErrorCode::BadRequest, format!("no arc {id}"))),
                QueryDto::Canvass { neighborhood } => {
                    canvass_view(self.engine_mut()?, neighborhood)
                        .map(ResponseBody::Canvass)
                        .ok_or_else(|| {
                            err(
                                ErrorCode::BadRequest,
                                format!("no neighbourhood {neighborhood} in your town"),
                            )
                        })
                }
                // The content workbench (dev builds only; read-only).
                #[cfg(feature = "dev-tools")]
                QueryDto::DevCatalog => Ok(ResponseBody::DevCatalog(dev::dev_catalog(
                    self.engine_mut()?,
                ))),
                #[cfg(feature = "dev-tools")]
                QueryDto::DevExplain {
                    kind,
                    id,
                    speaker,
                    place,
                } => dev::dev_explain(self.engine_mut()?, &kind, &id, speaker, place)
                    .map(ResponseBody::DevExplain)
                    .map_err(|e| err(ErrorCode::BadRequest, e)),
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
        let log = self.log.as_ref().map(|l| l.with_commands(engine));
        mandate_save::encode_with_log(engine.state(), log.as_ref(), &r)
            .map(|(b, _)| b)
            .map_err(|e| e.to_string())
    }

    /// The engine's part of a bug report, as JSON (`ReportCore`).
    pub fn report_core(&self) -> Result<String, String> {
        let engine = self.engine.as_ref().ok_or("no game is running")?;
        let st = engine.state();
        let log = match &self.log {
            Some(l) => l.with_commands(engine),
            None => RunLog::unreplayable(engine, "no run log was kept"),
        };
        let core = ReportCore {
            engine_version: mandate_core::ENGINE_VERSION.to_string(),
            content_hash: self.content.content_hash.clone(),
            protocol_version: PROTOCOL_VERSION,
            save_format_version: mandate_save::SAVE_FORMAT_VERSION,
            state_hash: engine.state_hash(),
            turn: st.calendar.turn,
            date: DateView::of(st.calendar.date),
            log,
        };
        serde_json::to_string(&core).map_err(|e| e.to_string())
    }

    /// Replay a bug report's run log here (in WASM, for `mandate-cli replay
    /// --wasm`), on an engine of its own: the running game is untouched.
    /// Returns the `ReplayOutcome` as JSON.
    pub fn replay_report(&self, report_json: &str, until: Option<u32>) -> Result<String, String> {
        let r: ReportLog = serde_json::from_str(report_json).map_err(|e| e.to_string())?;
        let setup = r
            .log
            .new_game
            .as_ref()
            .ok_or("this run cannot be replayed from its start")?;
        let mut engine = start_game(&self.content, setup, &mut |_, _| {}).map_err(|e| e.message)?;
        let outcome: ReplayOutcome = replay(&mut engine, &r.log, until);
        serde_json::to_string(&outcome).map_err(|e| e.to_string())
    }

    /// Who a save belongs to (a read-only look: the running game is untouched).
    pub fn read_save_summary(&self, bytes: &[u8]) -> Result<SaveSummaryDto, String> {
        let (state, _log, _) = mandate_save::decode_with_log(bytes).map_err(|e| e.to_string())?;
        let engine = Engine::from_state(state, self.content.clone()).map_err(|e| e.to_string())?;
        let office = office_view(&engine).ok_or("the save has no player")?;
        Ok(SaveSummaryDto {
            player_name: office.player.name,
            role_key: office.player.role_key,
            home: office.player.municipality,
        })
    }

    /// Replace the running game with a decoded save; returns its overview.
    pub fn import_save(&mut self, bytes: &[u8]) -> Result<OverviewView, String> {
        let (state, log, _) = mandate_save::decode_with_log(bytes).map_err(|e| e.to_string())?;
        // Loaded under other rules (DEC-074), the run can no longer be
        // rebuilt from its start: its log starts again here.
        let same_rules = state.meta.content_hash == self.content.content_hash
            && state.meta.engine_version == mandate_core::ENGINE_VERSION;
        let rules = format!(
            "the rules changed at turn {} (engine {} to {}, content {} to {})",
            state.calendar.turn,
            state.meta.engine_version,
            mandate_core::ENGINE_VERSION,
            state.meta.content_hash.get(..12).unwrap_or_default(),
            self.content.content_hash.get(..12).unwrap_or_default(),
        );
        // The terrain raster is derived from the seed (DEC-044 rev. 2 keeps saves small).
        let (terrain, _matches) =
            rebuild_terrain(&state, &self.content).map_err(|e| e.to_string())?;
        let mut engine =
            Engine::from_state(state, self.content.clone()).map_err(|e| e.to_string())?;
        engine.set_terrain(Arc::new(terrain));
        self.log = Some(match log {
            Some(mut log) if same_rules => {
                engine.restore_log(std::mem::take(&mut log.commands));
                log
            }
            Some(_) => RunLog::unreplayable(&engine, rules),
            None => RunLog::unreplayable(&engine, "the run began before run logs were kept"),
        });
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
fn player_command(
    engine: Option<&mandate_sim::Engine>,
    c: CommandDto,
) -> Result<mandate_sim::actions::PlayerCommand, ProtocolError> {
    use mandate_sim::actions::{Action, PlayerCommand as P};
    use mandate_sim::policy::Clause;
    use mandate_sim::world::ConstituencyId;
    Ok(match c {
        CommandDto::MoveTo { location } => P::MoveTo {
            location: location.to_sim(),
        },
        CommandDto::Meet { slot } => P::Meet { slot },
        CommandDto::AnswerDecision { decision, option } => P::Answer { decision, option },
        CommandDto::Talk {
            to,
            topic,
            about,
            late_night,
        } => P::Talk {
            to: to.to_sim(),
            topic,
            about: about.map(|a| a.to_sim()),
            late_night,
        },
        CommandDto::Go { location, then } => P::Go {
            location: location.to_sim(),
            then: match then {
                Some(c) => Some(Box::new(player_command(engine, *c)?)),
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
            // A family bill's choices come with the proposal sheet (F3, F6).
            choices: None,
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
        CommandDto::File { post, retire } => {
            // A post the player may file for (`CampaignView.nominations`),
            // or retire; with neither, stand again for the post held.
            use mandate_sim::politics::elections::Filing;
            let engine = engine.ok_or_else(|| err(ErrorCode::InvalidCommand, "no game"))?;
            let filing = if retire {
                Some(Filing::Retire)
            } else {
                match post {
                    Some(dto) => mandate_protocol::campaign::fileable(engine, &dto),
                    None => {
                        let held = mandate_sim::player::posts(engine.state());
                        mandate_protocol::campaign::fileable_posts(engine)
                            .into_iter()
                            .find(|p| held.contains(p))
                    }
                }
                .map(|post| Filing::Stand { post })
            }
            .ok_or_else(|| err(ErrorCode::InvalidCommand, "no such post to file for"))?;
            P::File { filing }
        }
        CommandDto::WithdrawPolicy { policy } => P::WithdrawPolicy { policy },
        CommandDto::RespondAsk { ask, accept } => P::RespondAsk { ask, accept },
        CommandDto::Promise { template, answer } => P::Promise {
            template,
            args: answer
                .map(|a| (mandate_content::asks::ROLE_ANSWER.to_string(), a))
                .into_iter()
                .collect(),
        },
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
                ActionDto::Canvass {
                    neighborhood,
                    issue,
                } => Action::Canvass {
                    neighborhood,
                    issue,
                },
                ActionDto::Fundraise => Action::Fundraise,
                ActionDto::Rest => Action::Rest,
                ActionDto::Advertise => Action::Advertise,
                ActionDto::TakeBribe => Action::TakeBribe,
                ActionDto::DipFunds => Action::DipFunds,
                ActionDto::LeakFile => Action::LeakFile,
                ActionDto::Visit { verb } => Action::Visit { verb },
                ActionDto::Casework { ask } => Action::Casework { ask },
            },
        },
        // EndTurn and AdvanceDays are handled before this conversion; they
        // cannot follow a move.
        CommandDto::EndTurn
        | CommandDto::AdvanceDays { .. }
        | CommandDto::NotebookSeen
        | CommandDto::SetDirectives { .. }
        | CommandDto::Skip { .. }
        | CommandDto::CancelSkip => {
            return Err(err(
                ErrorCode::InvalidCommand,
                "time cannot pass as part of a move",
            ));
        }
    })
}
