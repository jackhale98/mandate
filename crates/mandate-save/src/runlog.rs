//! The run log (bug reports): the new-game parameters, every command applied
//! since, and state-hash checkpoints, so that a tester's run can be rebuilt
//! exactly (DEC-041) by `mandate-cli replay`.
//!
//! The log is kept in the save beside the state (format v18), never inside
//! it: the state hash does not cover it, and the game itself never replays
//! it (DEC-074: a save is loaded as it is). Only the developer tools do.

use mandate_sim::arcs::audition::Draft;
use mandate_sim::arcs::opening::OpeningChoice;
use mandate_sim::arcs::score::Opening;
use mandate_sim::engine::LoggedCommand;
use mandate_sim::state::WorldSize;
use mandate_sim::{Command, Engine, EngineError};
use serde::{Deserialize, Serialize};

/// Version of the `RunLog` layout (inside the save and in report files).
pub const RUN_LOG_VERSION: u16 = 1;

/// The choices a new game was made with (the protocol's `NewGameParams`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NewGameSetup {
    /// As a decimal string: JSON readers in the browser lose precision on
    /// integers above 2^53.
    #[serde(with = "seed_string")]
    pub seed: u64,
    pub size: WorldSize,
    pub scenario: Option<String>,
    pub storyteller: Option<String>,
    pub local_knowledge: bool,
    /// The device's last openings (DEC-101 §3, STAGED_ARCS §6.7), oldest
    /// first: the draft's variety is measured against them, so the same
    /// seed and parameters give the same draft. A shared seed passes none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_openings: Vec<Opening>,
    /// The opening the draft's choice started play on (none: play began
    /// as the world was generated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<OpeningChoice>,
}

impl NewGameSetup {
    /// What a new game does after world generation and before any command:
    /// the storyteller (DEC-033) and the starting knowledge (DEC-091). The
    /// browser session and the replay tools both go through here.
    pub fn prepare(&self, engine: &mut Engine) -> Result<(), EngineError> {
        if let Some(k) = &self.storyteller {
            engine.setup(|s| s.narrative.profile = k.clone());
        }
        if self.local_knowledge {
            engine.set_knowledge_start(None, true)?;
        }
        // The opening chosen in the draft: the world advanced as the
        // audition advanced it, then the frame (`arcs::opening`).
        if let Some(o) = &self.opening {
            mandate_sim::arcs::opening::open(engine, o).map_err(EngineError::InvalidCommand)?;
        }
        Ok(())
    }
}

/// What the audition measured, for the lab (STAGED_ARCS §6.6: "nothing
/// from a fork is shown or kept, except the scores in the run log"). The
/// game never reads it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DraftLog {
    pub months_ahead: u32,
    /// Candidates the scan found, and the forks played.
    pub found: u32,
    pub forks: u32,
    /// `audition::digest`: two auditions that agree to the last score.
    pub digest: String,
    /// The candidate chosen (an index into `candidates`).
    pub chosen: Option<u32>,
    pub candidates: Vec<DraftEntry>,
}

/// One candidate auditioned, and its terms (each 0 to 1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DraftEntry {
    pub town: u32,
    pub template: String,
    /// Its place in the draft, if offered.
    pub offered: Option<u32>,
    pub escalates: bool,
    pub agency: Option<f64>,
    pub escalation: f64,
    pub visibility: f64,
    pub pacing: f64,
    pub variety: f64,
    pub total: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl DraftLog {
    /// The scores of a draft, and which of its openings (by its place in
    /// the draft) was chosen.
    pub fn of(d: &Draft, chosen: Option<usize>) -> Self {
        let r = |x: f64| (x * 1000.0).round() / 1000.0;
        DraftLog {
            months_ahead: d.months_ahead,
            found: d.found as u32,
            forks: d.forks,
            digest: mandate_sim::arcs::audition::digest(d),
            chosen: chosen.and_then(|i| d.openings.get(i)).map(|i| *i as u32),
            candidates: d
                .auditioned
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let s = a.score.as_ref();
                    DraftEntry {
                        town: a.candidate.town.0,
                        template: a.candidate.template.clone(),
                        offered: d.openings.iter().position(|x| *x == i).map(|p| p as u32),
                        escalates: s.is_some_and(|s| s.escalates),
                        agency: s.and_then(|s| s.agency.as_ref()).map(|a| r(a.value())),
                        escalation: s.map_or(0.0, |s| r(s.escalation.value())),
                        visibility: s.map_or(0.0, |s| r(s.visibility.value())),
                        pacing: s.map_or(0.0, |s| r(s.pacing.value())),
                        variety: s.map_or(0.0, |s| r(s.variety)),
                        total: s.map_or(0.0, |s| r(s.total)),
                        error: a.error.clone(),
                    }
                })
                .collect(),
        }
    }
}

/// The state hash once `commands` log entries had been applied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub commands: u64,
    pub turn: u32,
    pub hash: String,
}

/// A run's replay log.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunLog {
    pub version: u16,
    /// `None`: the run cannot be rebuilt from its start (it began before
    /// run logs, or a save was loaded under other rules); `note` says why.
    pub new_game: Option<NewGameSetup>,
    /// The engine and content the logged commands ran under.
    pub engine_version: String,
    pub content_hash: String,
    /// Every command applied since the start, in order (the engine's log).
    #[serde(with = "entries")]
    pub commands: Vec<LoggedCommand>,
    /// State hashes: after the new game (0 commands), then after each turn.
    pub checkpoints: Vec<Checkpoint>,
    #[serde(default)]
    pub note: Option<String>,
    /// The audition's scores, when the run began on a drafted opening.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<DraftLog>,
}

impl RunLog {
    /// A log for a game just started: its first checkpoint is the new world.
    pub fn start(setup: NewGameSetup, engine: &Engine) -> Self {
        let st = engine.state();
        RunLog {
            version: RUN_LOG_VERSION,
            new_game: Some(setup),
            engine_version: st.meta.engine_version.clone(),
            content_hash: st.meta.content_hash.clone(),
            commands: Vec::new(),
            checkpoints: vec![Checkpoint {
                commands: engine.command_log().len() as u64,
                turn: st.calendar.turn,
                hash: engine.state_hash(),
            }],
            note: None,
            draft: None,
        }
    }

    /// A log that starts where it is (no way back to the run's start).
    pub fn unreplayable(engine: &Engine, note: impl Into<String>) -> Self {
        let st = engine.state();
        RunLog {
            version: RUN_LOG_VERSION,
            new_game: None,
            engine_version: st.meta.engine_version.clone(),
            content_hash: st.meta.content_hash.clone(),
            commands: Vec::new(),
            checkpoints: Vec::new(),
            note: Some(note.into()),
            draft: None,
        }
    }

    /// Record the engine's state as a checkpoint (after a turn).
    pub fn checkpoint(&mut self, engine: &Engine) {
        let commands = engine.command_log().len() as u64;
        if self
            .checkpoints
            .last()
            .is_some_and(|c| c.commands == commands)
        {
            return;
        }
        self.checkpoints.push(Checkpoint {
            commands,
            turn: engine.state().calendar.turn,
            hash: engine.state_hash(),
        });
    }

    /// The log as it stands, with the engine's commands.
    pub fn with_commands(&self, engine: &Engine) -> RunLog {
        RunLog {
            commands: engine.command_log().to_vec(),
            ..self.clone()
        }
    }
}

/// One checkpoint compared during a replay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CheckpointResult {
    pub commands: u64,
    pub turn: u32,
    pub expected: String,
    pub actual: String,
}

impl CheckpointResult {
    pub fn matches(&self) -> bool {
        self.expected == self.actual
    }
}

/// Where a replay first parted from the recorded run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Divergence {
    /// The checkpoint whose hash differs.
    pub at: CheckpointResult,
    /// The turn the last matching checkpoint was taken at (`None`: the
    /// new world itself differs: world generation or setup).
    pub last_good_turn: Option<u32>,
    /// The log entries applied between the last matching checkpoint and this
    /// one, `[from, to)`: one of them (or the turn they end) diverged.
    pub commands_from: u64,
    pub commands_to: u64,
}

/// A log entry the replay could not apply. Expected only for a failed `Go`
/// that still moved the player (DEC-079), which the engine logs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayRefusal {
    pub index: u64,
    pub error: String,
}

/// What a replay found.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayOutcome {
    pub commands_applied: u64,
    pub commands_total: u64,
    /// Stopped early (`until`) or at the first divergence.
    pub stopped_early: bool,
    pub turn: u32,
    pub state_hash: String,
    pub checkpoints: Vec<CheckpointResult>,
    pub divergence: Option<Divergence>,
    pub refusals: Vec<ReplayRefusal>,
    /// The log's indices are not 0, 1, 2…: entries are missing.
    pub gaps: bool,
}

/// Replay a log's commands on an engine freshly made from its `new_game`
/// (world generated, `NewGameSetup::prepare` applied). Compares the state
/// hash at every checkpoint and stops at the first that differs, or before
/// the first command applied on turn `until`.
pub fn replay(engine: &mut Engine, log: &RunLog, until: Option<u32>) -> ReplayOutcome {
    let mut cmp = Comparer {
        log,
        next: 0,
        results: Vec::new(),
    };
    let mut refusals = Vec::new();
    let mut applied: u64 = 0;
    let mut stopped_early = false;
    let gaps = log
        .commands
        .iter()
        .enumerate()
        .any(|(i, c)| c.index != i as u64);
    let mut divergence = cmp.check(engine, 0);
    if divergence.is_none() {
        for entry in &log.commands {
            if until.is_some_and(|t| engine.state().calendar.turn >= t) {
                stopped_early = true;
                break;
            }
            if let Err(e) = engine.apply(entry.command.clone()) {
                refusals.push(ReplayRefusal {
                    index: entry.index,
                    error: e.to_string(),
                });
            }
            applied += 1;
            divergence = cmp.check(engine, applied);
            if divergence.is_some() {
                stopped_early = applied < log.commands.len() as u64;
                break;
            }
        }
    }
    ReplayOutcome {
        commands_applied: applied,
        commands_total: log.commands.len() as u64,
        stopped_early,
        turn: engine.state().calendar.turn,
        state_hash: engine.state_hash(),
        checkpoints: cmp.results,
        divergence,
        refusals,
        gaps,
    }
}

/// Walks the log's checkpoints alongside a replay.
struct Comparer<'a> {
    log: &'a RunLog,
    next: usize,
    results: Vec<CheckpointResult>,
}

impl Comparer<'_> {
    /// Compare the checkpoint taken after `applied` commands, if there is one.
    fn check(&mut self, engine: &Engine, applied: u64) -> Option<Divergence> {
        while let Some(cp) = self.log.checkpoints.get(self.next) {
            if cp.commands > applied {
                break;
            }
            self.next += 1;
            if cp.commands < applied {
                continue;
            }
            let r = CheckpointResult {
                commands: cp.commands,
                turn: cp.turn,
                expected: cp.hash.clone(),
                actual: engine.state_hash(),
            };
            let last_good = self.results.last().cloned();
            self.results.push(r.clone());
            if !r.matches() {
                return Some(Divergence {
                    at: r,
                    last_good_turn: last_good.as_ref().map(|c| c.turn),
                    commands_from: last_good.map_or(0, |c| c.commands),
                    commands_to: cp.commands,
                });
            }
        }
        None
    }
}

/// A command as the log writes it: `Command`'s own form, except that a
/// player command goes under `player`
/// (`{"type": "player", "player": {"type": "go", …}}`). `Command` and
/// `PlayerCommand` are both tagged `type`, so `Command`'s derived form of a
/// player command has the key twice and cannot be read back.
pub fn command_json(c: &Command) -> serde_json::Value {
    match c {
        Command::Player(p) => serde_json::json!({ "type": "player", "player": p }),
        other => serde_json::to_value(other).unwrap_or(serde_json::Value::Null),
    }
}

/// Read a command written by `command_json`.
pub fn command_from_json(v: serde_json::Value) -> Result<Command, String> {
    if v.get("type").and_then(serde_json::Value::as_str) == Some("player") {
        let p = v
            .get("player")
            .cloned()
            .ok_or("a player command without `player`")?;
        return serde_json::from_value(p)
            .map(Command::Player)
            .map_err(|e| e.to_string());
    }
    serde_json::from_value(v).map_err(|e| e.to_string())
}

/// A command for a log entry, for display.
pub fn describe(c: &Command) -> serde_json::Value {
    command_json(c)
}

/// `RunLog::commands` as `[{"index": 0, "command": …}, …]` (`command_json`).
mod entries {
    use super::{command_from_json, command_json};
    use mandate_sim::engine::LoggedCommand;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

    #[derive(Serialize, Deserialize)]
    struct Entry {
        index: u64,
        command: serde_json::Value,
    }

    pub fn serialize<S: Serializer>(v: &[LoggedCommand], s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(v.iter().map(|c| Entry {
            index: c.index,
            command: command_json(&c.command),
        }))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<LoggedCommand>, D::Error> {
        Vec::<Entry>::deserialize(d)?
            .into_iter()
            .map(|e| {
                command_from_json(e.command)
                    .map(|command| LoggedCommand {
                        index: e.index,
                        command,
                    })
                    .map_err(|m| D::Error::custom(format!("log entry {}: {m}", e.index)))
            })
            .collect()
    }
}

mod seed_string {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(v)
    }

    /// A decimal string (or, leniently, a JSON number).
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum S {
            Str(String),
            Num(u64),
        }
        match S::deserialize(d)? {
            S::Str(s) => s.parse().map_err(D::Error::custom),
            S::Num(n) => Ok(n),
        }
    }
}
