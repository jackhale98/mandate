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

fn short(h: &str) -> String {
    h.chars().take(12).collect()
}

/// Content ids a state refers to that the loaded content lacks.
pub fn missing_references(state: &State, content: &ContentBundle) -> Vec<String> {
    let mut out = Vec::new();
    if !content.constitutions.contains_key(&state.meta.constitution) {
        out.push(state.meta.constitution.clone());
    }
    for p in &state.policy.instances {
        if !content.policies.contains_key(&p.template) && !out.contains(&p.template) {
            out.push(p.template.clone());
        }
    }
    // The world's scenario rebuilds its terrain and seeds new sectors.
    if let Some(k) = &state.meta.params.scenario
        && content.scenario(k).is_none()
    {
        out.push(k.clone());
    }
    // Sector data cannot be carried into content that removed the sector.
    for k in &state.economy.sector_keys {
        if content.sector_index(k).is_none() && !out.contains(k) {
            out.push(k.clone());
        }
    }
    // Nor can opinions on an attitude the content no longer has (DEC-075).
    for k in &state.population.attitude_keys {
        if content.attitude_index(k).is_none() && !out.contains(k) {
            out.push(k.clone());
        }
    }
    // Nor can the people of a community the content no longer has.
    for k in &state.population.community_keys {
        if content.community_index(k).is_none() && !out.contains(k) {
            out.push(k.clone());
        }
    }
    out
}

impl Engine {
    /// Wrap an existing state (from worldgen or a save).
    pub fn from_state(mut state: State, content: Arc<ContentBundle>) -> Result<Self, EngineError> {
        // A save from another content or engine version (a balance patch or an
        // app update) loads if every rule it refers to still exists; the new
        // rules apply from here on and the run's history records the change.
        // Determinism holds within one (engine, content) version.
        if state.meta.content_hash != content.content_hash
            || state.meta.engine_version != mandate_core::ENGINE_VERSION
        {
            let missing = missing_references(&state, &content);
            if !missing.is_empty() {
                return Err(EngineError::Content(format!(
                    "this save needs rules that no longer exist: {}",
                    missing.join(", ")
                )));
            }
            let from = short(&state.meta.content_hash);
            let to = short(&content.content_hash);
            let date = state.calendar.date;
            let id = state.events.record(
                date,
                EventKind::RulesUpdated {
                    from_content: from,
                    to_content: to,
                    from_engine: state.meta.engine_version.clone(),
                    to_engine: mandate_core::ENGINE_VERSION.to_string(),
                },
                vec![],
            );
            state.events.pin(id);
            state.meta.content_hash = content.content_hash.clone();
            state.meta.engine_version = mandate_core::ENGINE_VERSION.to_string();
        }
        // New content may add or reorder sectors: re-key sector data. (Content
        // that removed one of the save's sectors was refused above.)
        crate::economy::remap_sectors(&mut state.economy, &state.world, &state.meta, &content);
        // Likewise attitudes: re-key every attitude vector by attitude key.
        crate::population::remap_attitudes(
            &mut state.population,
            &mut state.politics,
            &state.world,
            &content,
        );
        // Then communities: cohorts are re-laid out by community id (a new
        // community starts empty).
        crate::population::remap_communities(
            &mut state.population,
            &state.politics,
            &state.world,
            &content,
        );
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

    pub fn content_arc(&self) -> Arc<ContentBundle> {
        self.content.clone()
    }

    /// World-generation setup on the live state (before play starts). Not a
    /// command: it is never logged or available to players.
    pub fn setup(&mut self, f: impl FnOnce(&mut State)) {
        f(&mut self.state);
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
                // NPCs act in the same turn as the player, before time moves (DEC-030).
                self.npc_turns();
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
                    p.ads_this_turn = 0;
                }
                self.update_phase();
                CommandOutcome {
                    days_advanced: days,
                    turn,
                    result: None,
                }
            }
            Command::Player(cmd) => {
                let before = self.state.player.as_ref().map(|p| p.location);
                let result = match crate::actions::apply(&mut self.state, &self.content, cmd) {
                    Ok(r) => r,
                    Err(e) => {
                        // A failed `Go` keeps its move (DEC-079): log it so a
                        // replay reproduces the new location.
                        if self.state.player.as_ref().map(|p| p.location) != before {
                            self.command_log.push(LoggedCommand {
                                index: self.next_command,
                                command,
                            });
                            self.next_command += 1;
                        }
                        return Err(e);
                    }
                };
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
                if self.content.sector_index(sector).is_none() {
                    return Err(EngineError::InvalidCommand(format!(
                        "no such sector `{sector}`"
                    )));
                }
                let effect = crate::scheduler::Effect::SectorShock {
                    municipality: *municipality,
                    sector: sector.clone(),
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
        // Councils legislate during history too: NPC legislative turns every
        // six months; campaigns are abstracted (DEC-022, DEC-030).
        const STEP: u32 = 182;
        for y in 0..years {
            let mut left = mandate_core::time::DAYS_PER_YEAR;
            while left > 0 {
                crate::npc::take_turn_all(&mut self.state, &self.content, false);
                let d = STEP.min(left);
                self.schedule
                    .advance(&mut self.state, &self.content, d, &mut |_| {});
                left -= d;
            }
            progress(y + 1);
        }
        self.state.calendar.phase = prev;
    }

    /// NPC turns before the calendar advances: the player's town in full
    /// every turn, other towns' legislative work every fourth turn.
    fn npc_turns(&mut self) {
        let home = self.state.player.as_ref().map(|p| p.municipality.0);
        if let Some(m) = home {
            crate::npc::take_turn(&mut self.state, &self.content, m, true);
        }
        if self.state.calendar.turn.is_multiple_of(4) {
            let n = self.state.world.municipalities.len() as u32;
            for m in (0..n).filter(|m| Some(*m) != home) {
                crate::npc::take_turn(&mut self.state, &self.content, m, false);
            }
        }
    }

    /// Campaign season shortens turns (DEC-012).
    fn update_phase(&mut self) {
        let me = self.state.player.as_ref().map(|p| p.politician);
        let campaigning = self
            .state
            .politics
            .elections
            .races
            .iter()
            .any(|r| r.candidates.iter().any(|c| Some(c.politician) == me));
        self.state.calendar.phase = if campaigning {
            GamePhase::Campaign
        } else {
            GamePhase::Governing
        };
    }

    /// A player command routed to where it is done (DEC-079 auto-routing):
    /// a `Go` to the best place for it when the player is elsewhere.
    pub fn route(&self, cmd: crate::actions::PlayerCommand) -> Command {
        Command::Player(crate::places::routed(&self.state, &self.content, cmd))
    }

    pub fn system_names(&self) -> Vec<&'static str> {
        self.schedule.names()
    }
}
