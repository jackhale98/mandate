//! The engine owns one authoritative `State` and applies commands serially.

use crate::commands::{Command, CommandOutcome, EngineError};
use crate::events::EventKind;
use crate::scheduler::Schedule;
use crate::state::{GamePhase, State};
use crate::systems::default_systems;
use mandate_content::ContentBundle;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A command as recorded in the replay log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoggedCommand {
    pub index: u64,
    pub command: Command,
}

pub struct Engine {
    state: State,
    /// Derived terrain raster (not serialized; rebuilt from the seed).
    terrain: Option<Arc<crate::world::TerrainData>>,
    content: Arc<ContentBundle>,
    schedule: Schedule,
    command_log: Vec<LoggedCommand>,
    next_command: u64,
    /// Effects queued by lab commands, delivered on the next advance.
    pending_effects: Vec<crate::scheduler::Effect>,
}

impl Engine {
    /// Wrap an existing state (from worldgen or a save).
    pub fn from_state(state: State, content: Arc<ContentBundle>) -> Result<Self, EngineError> {
        if state.meta.content_hash != content.content_hash {
            return Err(EngineError::Content(format!(
                "state was created with content {} but {} is loaded",
                &state.meta.content_hash[..12.min(state.meta.content_hash.len())],
                &content.content_hash[..12]
            )));
        }
        Ok(Engine {
            state,
            terrain: None,
            content,
            schedule: Schedule::new(default_systems()),
            command_log: Vec::new(),
            next_command: 0,
            pending_effects: Vec::new(),
        })
    }

    /// Record the start-of-game event. Called once by worldgen for new games.
    pub fn mark_started(&mut self) {
        let seed = self.state.meta.seed;
        let id = self.state.events.record(
            self.state.calendar.date,
            EventKind::GameStarted { seed },
            vec![],
        );
        self.state.events.pin(id);
    }

    pub fn set_terrain(&mut self, t: Arc<crate::world::TerrainData>) {
        self.terrain = Some(t);
    }

    pub fn terrain(&self) -> Option<&crate::world::TerrainData> {
        self.terrain.as_deref()
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn content(&self) -> &ContentBundle {
        &self.content
    }

    pub fn command_log(&self) -> &[LoggedCommand] {
        &self.command_log
    }

    /// Clear the replay log after a snapshot has been written.
    pub fn truncate_log(&mut self) {
        self.command_log.clear();
    }

    pub fn state_hash(&self) -> String {
        self.state.hash()
    }

    pub fn turn_days(&self) -> u32 {
        let t = &self.content.tuning.turn_days;
        match self.state.calendar.phase {
            GamePhase::Campaign => t.campaign.into(),
            GamePhase::Governing | GamePhase::BurnIn => t.governing_local.into(),
        }
    }

    pub fn apply(&mut self, command: Command) -> Result<CommandOutcome, EngineError> {
        let outcome = match &command {
            Command::EndTurn => {
                let days = self.turn_days();
                let seeded = std::mem::take(&mut self.pending_effects);
                self.schedule.advance_with(
                    &mut self.state,
                    &self.content,
                    days,
                    seeded,
                    &mut |_| {},
                );
                self.state.calendar.turn += 1;
                let turn = self.state.calendar.turn;
                self.state.events.record(
                    self.state.calendar.date,
                    EventKind::TurnEnded { turn },
                    vec![],
                );
                if let Some(p) = self.state.player.as_mut() {
                    p.slots_used = 0;
                    p.late_night_used = false;
                }
                CommandOutcome {
                    days_advanced: days,
                    turn,
                    result: None,
                }
            }
            Command::Player(cmd) => {
                let result = crate::actions::apply(&mut self.state, &self.content, cmd)?;
                CommandOutcome {
                    days_advanced: 0,
                    turn: self.state.calendar.turn,
                    result: Some(result),
                }
            }
            Command::LabSectorShock {
                municipality,
                sector,
                factor,
                months,
            } => {
                if *municipality as usize >= self.state.world.municipalities.len() {
                    return Err(EngineError::InvalidCommand("no such municipality".into()));
                }
                let effect = crate::scheduler::Effect::SectorShock {
                    municipality: *municipality,
                    sector: *sector,
                    factor: *factor,
                    months: *months,
                    cause: None,
                };
                // Delivered on the next advance; applied at the next monthly economy step.
                self.pending_effects.push(effect);
                CommandOutcome {
                    days_advanced: 0,
                    turn: self.state.calendar.turn,
                    result: None,
                }
            }
            Command::AdvanceDays { days } => {
                if *days > 366 * 100 {
                    return Err(EngineError::InvalidCommand(
                        "advance limited to 100 years".into(),
                    ));
                }
                let seeded = std::mem::take(&mut self.pending_effects);
                self.schedule.advance_with(
                    &mut self.state,
                    &self.content,
                    *days,
                    seeded,
                    &mut |_| {},
                );
                CommandOutcome {
                    days_advanced: *days,
                    turn: self.state.calendar.turn,
                    result: None,
                }
            }
        };
        self.command_log.push(LoggedCommand {
            index: self.next_command,
            command,
        });
        self.next_command += 1;
        Ok(outcome)
    }

    /// NPC-only history before play begins (DEC-022). Not logged as a command:
    /// it is part of world generation. Reports progress per simulated year.
    pub fn burn_in(&mut self, years: u32, progress: &mut dyn FnMut(u32)) {
        let prev = self.state.calendar.phase;
        self.state.calendar.phase = GamePhase::BurnIn;
        for y in 0..years {
            self.schedule.advance(
                &mut self.state,
                &self.content,
                mandate_core::time::DAYS_PER_YEAR,
                &mut |_| {},
            );
            progress(y + 1);
        }
        self.state.calendar.phase = prev;
    }

    pub fn system_names(&self) -> Vec<&'static str> {
        self.schedule.names()
    }
}
