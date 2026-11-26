//! The engine owns one authoritative `State` and applies commands serially.

use crate::commands::{Command, CommandOutcome, EngineError};
use crate::events::EventKind;
use crate::scheduler::Schedule;
use crate::spine::{Cause, PlayerAct, Process};
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
    /// The day's effect appliers, run once more after an advance's last day
    /// so that nothing queued late in it is lost (`settle`).
    appliers: Schedule,
    command_log: Vec<LoggedCommand>,
    next_command: u64,
    /// Effects delivered on the next advance: those queued by lab commands,
    /// and those an advance left for an applier that runs monthly.
    pending_effects: Vec<crate::scheduler::Request>,
}

/// The daily systems that only apply effect requests other systems made
/// (OVERVIEW §4.3), in schedule order: run once more at the end of an
/// advance (`Engine::settle`). Each must do nothing but drain its effects
/// (re-running it on the same day with an empty queue changes nothing).
const SETTLE_APPLIERS: [&str; 5] = [
    "population.coverage",
    "population.effects",
    "characters.effects",
    "politics.media_effects",
    "player.effects",
];

/// Effects whose applier runs monthly (the economy's and population's):
/// still queued after an advance, they wait for the next one.
fn waits_for_monthly(e: &crate::scheduler::Effect) -> bool {
    use crate::scheduler::Effect;
    matches!(
        e,
        Effect::HousingDamage { .. }
            | Effect::SectorShock { .. }
            | Effect::PlantRelief { .. }
            | Effect::Displacement { .. }
            | Effect::HoodDisaster { .. }
    )
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
        // A family bill (DEC-092) needs its family and the options it chose.
        if let Some(d) = &p.family {
            match content.family(&p.template) {
                None => {
                    if !out.contains(&p.template) {
                        out.push(p.template.clone());
                    }
                }
                Some(f) => {
                    for (axis, option) in &d.choices {
                        if f.axis(axis).and_then(|a| a.option(option)).is_none() {
                            let k = format!("{}: {axis}.{option}", f.id);
                            if !out.contains(&k) {
                                out.push(k);
                            }
                        }
                    }
                }
            }
            continue;
        }
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
    // Nor industry data, nor a plant of an industry or product that is gone.
    for k in state
        .economy
        .industry_keys
        .iter()
        .chain(state.economy.plants.iter().map(|p| &p.industry))
    {
        if content.industry_index(k).is_none() && !out.contains(k) {
            out.push(k.clone());
        }
    }
    for p in &state.economy.plants {
        if content.product(&p.product).is_none() && !out.contains(&p.product) {
            out.push(p.product.clone());
        }
    }
    // Nor the index of a market that is gone.
    for k in &state.economy.market_keys {
        if content.market(k).is_none() && !out.contains(k) {
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
    // Nor can politicians hold posts the constitution lacks (DEC-098).
    let c = content.constitutions.get(&state.meta.constitution);
    for k in crate::politics::missing_references(&state.politics, c) {
        if !out.contains(&k) {
            out.push(k);
        }
    }
    // Nor can characters keep traits or occupations that were removed.
    for k in crate::characters::missing_references(&state.politics, content) {
        if !out.contains(&k) {
            out.push(k);
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
                Cause::Process(Process::Update),
            );
            state.events.pin(id);
            state.meta.content_hash = content.content_hash.clone();
            state.meta.engine_version = mandate_core::ENGINE_VERSION.to_string();
        }
        // New content may add or reorder sectors: re-key sector data. (Content
        // that removed one of the save's sectors was refused above.)
        crate::economy::remap_sectors(&mut state.economy, &state.world, &state.meta, &content);
        // Likewise industries (DEC-110); a save from before them gets each
        // split sector shared among its industries, and named plants at the
        // works its towns already have.
        crate::economy::industries::remap(
            &mut state.economy,
            &state.world,
            &state.meta,
            state.calendar.date,
            &content,
        );
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
        // Concerns are derived: recompute them for content whose concern
        // registry differs from the one they were computed with (a new
        // game, or content that added or removed a concern).
        let keys: Vec<String> = content.concern_keys().cloned().collect();
        if state.population.concern_keys != keys
            || state.population.town_concerns.len() != state.world.municipalities.len()
            || state.population.town_concerns.iter().any(|t| {
                t.salience.len() != state.population.attitude_count()
                    || t.measured.len() != t.levels.len()
            })
        {
            let (towns, hoods) = crate::concerns::compute_all(
                &crate::scheduler::Reads::of_state(&state, &content),
                &state.population,
            );
            state.population.town_concerns = towns;
            state.population.hood_concerns = hoods;
            state.population.concern_keys = keys;
        } else if !crate::concerns::hoods_current(&state.population, &state.world) {
            // A run from before neighbourhood concerns (save format v17):
            // derive them, and the felt town levels that aggregate them. The
            // measured levels and salience (the town's politics) are kept.
            let (towns, hoods) = crate::concerns::compute_all(
                &crate::scheduler::Reads::of_state(&state, &content),
                &state.population,
            );
            for (t, n) in state.population.town_concerns.iter_mut().zip(towns) {
                t.levels = n.levels;
            }
            state.population.hood_concerns = hoods;
        }
        // Characters (DEC-031): records for politicians from older saves,
        // Named characters for a game in play, and town characters for a
        // changed crisis registry (derived state, recomputed).
        crate::characters::on_load(&mut state, &content);
        // Narrative (DEC-074): pending decisions and storylines whose
        // template was removed lapse; nothing is refused.
        crate::narrative::on_load(&mut state, &content);
        // Media (DEC-034): a save from before the press gets its outlets.
        crate::media::on_load(&mut state, &content);
        // Knowledge (DEC-091): a run from before the ledger is seeded
        // generously (`Migrated`); a seeded ledger is kept as it is.
        crate::player::knowledge::on_load(&mut state, &content);
        // Policy families (DEC-092): decks for the focus jurisdictions
        // (keyed: the deck a fresh start would draw), and new families
        // drawn in; open proposals of a removed family lapse.
        crate::policy::families::on_load(&mut state, &content);
        // The day played (DEC-114): a new game, or a save from before it,
        // has none inside the turn; choose it now (a save keeps its own).
        if !crate::characters::calendar::Span::now(&state, &content).contains(state.calendar.day) {
            state.calendar.day = crate::characters::calendar::choose_day(&state, &content);
        }
        Ok(Engine {
            state,
            terrain: None,
            content,
            schedule: Schedule::new(default_systems()),
            appliers: Schedule::new(
                default_systems()
                    .into_iter()
                    .filter(|s| SETTLE_APPLIERS.contains(&s.name))
                    .collect(),
            ),
            command_log: Vec::new(),
            next_command: 0,
            pending_effects: Vec::new(),
        })
    }

    /// Record the start-of-game event and seed the player's knowledge from
    /// the content's default start profile (DEC-091; the draft of three will
    /// name a profile per background). Called once by worldgen for new games.
    pub fn mark_started(&mut self) {
        // Validated content always has its default profile.
        let _ = crate::player::knowledge::seed(&mut self.state, &self.content, None, false);
        let seed = self.state.meta.seed;
        // The run's policy-family decks (DEC-092).
        crate::policy::families::on_load(&mut self.state, &self.content);
        // The first turn's day played (DEC-114), now the player has a home.
        self.state.calendar.day =
            crate::characters::calendar::choose_day(&self.state, &self.content);
        let id = self.state.events.record(
            self.state.calendar.date,
            EventKind::GameStarted { seed },
            Cause::Player(PlayerAct::Start),
        );
        self.state.events.pin(id);
        // The act's Legacy record opens as play begins (DEC-006).
        if self.state.act.records.is_empty() {
            crate::legacy::begin(&mut self.state, &self.content);
        }
    }

    /// Choose the run's starting knowledge (DEC-091): a start profile
    /// (`None`: the content's default) and the "Start with local knowledge"
    /// option. New-game setup only: refused once a command has been applied.
    pub fn set_knowledge_start(
        &mut self,
        profile: Option<&str>,
        local_option: bool,
    ) -> Result<(), EngineError> {
        if self.next_command > 0 || self.state.calendar.turn > 0 {
            return Err(EngineError::InvalidCommand(
                "starting knowledge is chosen before play begins".into(),
            ));
        }
        crate::player::knowledge::seed(&mut self.state, &self.content, profile, local_option)
            .map_err(EngineError::InvalidCommand)
    }

    /// Lab only (`mandate-cli … --omniscient`): every subject reads as
    /// `detailed`, for baselines. Never offered to players.
    pub fn set_lab_omniscient(&mut self, on: bool) {
        if let Some(p) = self.state.player.as_mut() {
            p.knowledge.lab_omniscient = on;
        }
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

    /// Request an effect for the next day stepped (new-game setup: the
    /// opening's run, `arcs::opening`). Never logged.
    pub(crate) fn seed_request(&mut self, r: crate::scheduler::Request) {
        self.pending_effects.push(r);
    }

    pub fn command_log(&self) -> &[LoggedCommand] {
        &self.command_log
    }

    /// Clear the replay log after a snapshot has been written.
    pub fn truncate_log(&mut self) {
        self.command_log.clear();
    }

    /// Carry a loaded run's command log on (bug reports, `mandate-save`'s
    /// run log): the next command is logged after it. Touches nothing else;
    /// the log is never replayed by the game (DEC-074).
    pub fn restore_log(&mut self, log: Vec<LoggedCommand>) {
        self.next_command = log.last().map_or(0, |c| c.index + 1);
        self.command_log = log;
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
        if !matches!(
            command,
            Command::Skip { .. }
                | Command::CancelSkip
                | Command::NotebookSeen
                | Command::SetDirectives { .. }
        ) {
            self.finish_skip(crate::skip::SkipStop::Cancelled);
        }
        let outcome = match &command {
            Command::EndTurn => {
                if self
                    .state
                    .player
                    .as_ref()
                    .is_some_and(|p| matches!(p.status, crate::player::RunStatus::Ended { .. }))
                {
                    return Err(EngineError::InvalidCommand("the career is over".into()));
                }
                // In prison (a short sentence, DEC-006): time passes, turn by
                // turn, until release; the world goes on (a time skip).
                let mut days = 0;
                while let Some(crate::player::RunStatus::Imprisoned { .. }) =
                    self.state.player.as_ref().map(|p| p.status)
                {
                    days += self.end_one_turn();
                }
                if days > 0 {
                    let turn = self.state.calendar.turn;
                    self.command_log.push(LoggedCommand {
                        index: self.next_command,
                        command,
                    });
                    self.next_command += 1;
                    return Ok(CommandOutcome {
                        days_advanced: days,
                        turn,
                        result: None,
                        skip: None,
                    });
                }
                let days = self.end_one_turn();
                CommandOutcome {
                    days_advanced: days,
                    turn: self.state.calendar.turn,
                    result: None,
                    skip: None,
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
                    skip: None,
                }
            }
            Command::Skip { target, limit } => {
                return self.skip_with(*target, *limit, &mut |_| true);
            }
            Command::CancelSkip => {
                self.finish_skip(crate::skip::SkipStop::Cancelled);
                CommandOutcome {
                    days_advanced: 0,
                    turn: self.state.calendar.turn,
                    result: None,
                    skip: None,
                }
            }
            Command::SetDirectives { choices } => {
                let reg = &self.content.act_flow;
                for (d, o) in choices {
                    let Some(def) = reg.directive(d) else {
                        return Err(EngineError::InvalidCommand(format!("no directive `{d}`")));
                    };
                    if !crate::skip::offered(def, o, &self.content) {
                        return Err(EngineError::InvalidCommand(format!(
                            "`{o}` is not an option of `{d}`"
                        )));
                    }
                }
                let dirs = &mut self.state.act.directives.choices;
                for (d, o) in choices {
                    match dirs.iter_mut().find(|(k, _)| k == d) {
                        Some(x) => x.1 = o.clone(),
                        None => dirs.push((d.clone(), o.clone())),
                    }
                }
                dirs.sort();
                CommandOutcome {
                    days_advanced: 0,
                    turn: self.state.calendar.turn,
                    result: None,
                    skip: None,
                }
            }
            Command::NotebookSeen => {
                if let Some(p) = self.state.player.as_mut() {
                    p.knowledge.mark_seen();
                }
                CommandOutcome {
                    days_advanced: 0,
                    turn: self.state.calendar.turn,
                    result: None,
                    skip: None,
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
                };
                // Delivered on the next advance; applied at the next monthly economy step.
                self.pending_effects.push(crate::scheduler::Request {
                    effect,
                    cause: Cause::Lab,
                });
                CommandOutcome {
                    days_advanced: 0,
                    turn: self.state.calendar.turn,
                    result: None,
                    skip: None,
                }
            }
            Command::AdvanceDays { days } => {
                if *days > 366 * 100 {
                    return Err(EngineError::InvalidCommand(
                        "advance limited to 100 years".into(),
                    ));
                }
                // In weeks, so that decisions falling due take their default
                // on time (the same result as one advance otherwise).
                let mut seeded = std::mem::take(&mut self.pending_effects);
                let mut left = *days;
                while left > 0 {
                    let step = left.min(7);
                    let until = self.state.calendar.date.add_days(step);
                    crate::narrative::resolve_due(&mut self.state, &self.content, until);
                    seeded = self.schedule.advance_with(
                        &mut self.state,
                        &self.content,
                        step,
                        std::mem::take(&mut seeded),
                        &mut |_| {},
                    );
                    left -= step;
                }
                self.settle(seeded);
                crate::spine::prune::due(&mut self.state);
                self.refresh_day();
                CommandOutcome {
                    days_advanced: *days,
                    turn: self.state.calendar.turn,
                    result: None,
                    skip: None,
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

    /// One turn: decisions falling due take their default, NPCs act, the
    /// calendar advances. Returns the days advanced.
    fn end_one_turn(&mut self) -> u32 {
        // A canvass under way ends with the turn (DEC-114 §3.3).
        crate::actions::close_street(&mut self.state, &self.content);
        let days = self.turn_days();
        // Decisions whose deadline falls in this turn take their
        // default: the player let them pass (DEC-077 §4).
        let until = self.state.calendar.date.add_days(days);
        crate::narrative::resolve_due(&mut self.state, &self.content, until);
        // The calendar's day ends (DEC-114): the turn's occasions as facts,
        // an expected absence's cost, familiarity's monthly fade.
        crate::characters::calendar::end_turn(&mut self.state, &self.content);
        // NPCs act in the same turn as the player, before time moves (DEC-030).
        self.npc_turns();
        let seeded = std::mem::take(&mut self.pending_effects);
        let left =
            self.schedule
                .advance_with(&mut self.state, &self.content, days, seeded, &mut |_| {});
        self.settle(left);
        self.state.calendar.turn += 1;
        let turn = self.state.calendar.turn;
        self.state.events.record(
            self.state.calendar.date,
            EventKind::TurnEnded { turn },
            Cause::Process(Process::Calendar),
        );
        if let Some(p) = self.state.player.as_mut() {
            p.slots_used = 0;
            p.late_night_used = false;
            p.ads_this_turn = 0;
        }
        self.update_phase();
        // The act's record and its ending (played and skipped turns alike).
        crate::act::after_turn(&mut self.state, &self.content);
        // The event log's bound (the causal spine), about once a month, at
        // a turn's end: a skip prunes exactly as its turns played one by
        // one would.
        crate::spine::prune::due(&mut self.state);
        // The next turn's day played (DEC-114), once its phase is known.
        self.state.calendar.day =
            crate::characters::calendar::choose_day(&self.state, &self.content);
        days
    }

    /// The day played (DEC-114), chosen again when the stored one is not in
    /// the current turn (after a lab advance or the history's burn-in).
    fn refresh_day(&mut self) {
        let span = crate::characters::calendar::Span::now(&self.state, &self.content);
        if !span.contains(self.state.calendar.day) {
            self.state.calendar.day =
                crate::characters::calendar::choose_day(&self.state, &self.content);
        }
    }

    /// Close the skip in progress, if any, into its summary.
    fn finish_skip(&mut self, stop: crate::skip::SkipStop) {
        if let Some(run) = self.state.act.skip.take() {
            let s = crate::skip::finish(run, &self.state, &self.content, stop);
            self.state.act.last_skip = Some(s);
        }
    }

    /// Play one of the proxy's commands as the player's (the Commands
    /// phase, exactly as `Command::Player`; not logged: the skip is).
    fn proxy_command(
        &mut self,
        cmd: &crate::actions::PlayerCommand,
    ) -> Result<crate::actions::ActionResult, EngineError> {
        crate::actions::apply(&mut self.state, &self.content, cmd)
    }

    /// A time skip (DEC-012), reporting progress before each turn: the
    /// callback returns `false` to stop at that safe point (the skip stays
    /// in progress, as after a chunk; `Command::CancelSkip` ends it). The
    /// command is logged with the turns it ran, so a replay matches.
    pub fn skip_with(
        &mut self,
        target: mandate_content::act::SkipTargetKind,
        limit: Option<u32>,
        progress: &mut dyn FnMut(&crate::skip::SkipProgress) -> bool,
    ) -> Result<CommandOutcome, EngineError> {
        use crate::skip::{self, SkipStop, SummaryItem};
        let Some(pl) = self.state.player.as_ref() else {
            return Err(EngineError::InvalidCommand("no player".into()));
        };
        if !pl.status.active() {
            return Err(EngineError::InvalidCommand(
                "there is no skipping out of prison".into(),
            ));
        }
        if self.state.act.over() {
            return Err(EngineError::InvalidCommand("the act is over".into()));
        }
        // Go on with the skip in progress, or start one.
        let mut run = match self.state.act.skip.take() {
            Some(r) if r.target == target => r,
            other => {
                if let Some(r) = other {
                    self.state.act.skip = Some(r);
                    self.finish_skip(SkipStop::Cancelled);
                }
                let r = skip::start(&self.state, &self.content, target).ok_or_else(|| {
                    EngineError::InvalidCommand(format!(
                        "there is no `{}` to skip to now",
                        target.key()
                    ))
                })?;
                self.state.act.last_skip = None;
                r
            }
        };
        let max = self.content.act_flow.max_turns as u32;
        let mut days = 0;
        let mut ran = 0u32;
        let mut stopped_early = false;
        let stop: Option<SkipStop> = loop {
            if let Some(e) = &self.state.act.ending {
                break Some(SkipStop::ActEnded {
                    ending: e.ending.clone(),
                });
            }
            if self.state.calendar.date >= run.until {
                break Some(SkipStop::Reached);
            }
            if run.turns >= max {
                break Some(SkipStop::Limit);
            }
            if limit.is_some_and(|l| ran >= l) {
                break None;
            }
            if !progress(&skip::SkipProgress::of(
                &run,
                self.state.calendar.date,
                false,
            )) {
                stopped_early = true;
                break None;
            }
            // The proxy plays the turn, command by command. It learns
            // nothing for the player (DEC-102 clause 7; `proxy.learns`):
            // what its acts would teach (a canvassed street's worries, a
            // first visit, who was seen where) is put back as it was, since
            // the player was not there. What the turn's end makes public
            // still reaches them.
            let ledger = (!self.content.act_flow.proxy.learns)
                .then(|| self.state.player.as_ref().map(|p| p.knowledge.clone()))
                .flatten();
            let mut t = crate::proxy::ProxyTurn::new(&self.state, &self.content);
            while let Some(cmd) = t.next(&self.state, &self.content) {
                let r = self.proxy_command(&cmd);
                if r.is_ok() {
                    if let Some((decision, option)) = t.answered() {
                        let template = self
                            .state
                            .narrative
                            .decision(crate::narrative::DecisionId(decision))
                            .map(|d| d.template.clone())
                            .unwrap_or_default();
                        skip::push(
                            &mut run,
                            SummaryItem::Answered {
                                decision,
                                template,
                                option: option.to_string(),
                            },
                        );
                    }
                    let inner = match &cmd {
                        crate::actions::PlayerCommand::Go { then: Some(c), .. } => c.as_ref(),
                        c => c,
                    };
                    if let Some((policy, vote)) = t.voted(inner) {
                        skip::push(
                            &mut run,
                            SummaryItem::Voted {
                                policy,
                                vote: vote.into(),
                            },
                        );
                    }
                }
                let created = r.as_ref().ok().and_then(|x| x.created);
                t.result(r.is_ok(), created);
            }
            if let (Some(p), Some(k)) = (self.state.player.as_mut(), ledger) {
                p.knowledge = k;
            }
            days += self.end_one_turn();
            ran += 1;
            run.turns += 1;
            skip::observe(&mut run, &self.state);
            if let Some(e) = &self.state.act.ending {
                break Some(SkipStop::ActEnded {
                    ending: e.ending.clone(),
                });
            }
            if let Some(s) = skip::check_interrupts(&mut run, &self.state, &self.content) {
                break Some(s);
            }
        };
        let date = self.state.calendar.date;
        let progress = skip::SkipProgress::of(&run, date, stop.is_some());
        match stop {
            Some(s) => {
                let summary = skip::finish(run, &self.state, &self.content, s);
                self.state.act.last_skip = Some(summary);
            }
            None => self.state.act.skip = Some(run),
        }
        let logged = Command::Skip {
            target,
            limit: if stopped_early { Some(ran) } else { limit },
        };
        self.command_log.push(LoggedCommand {
            index: self.next_command,
            command: logged,
        });
        self.next_command += 1;
        Ok(CommandOutcome {
            days_advanced: days,
            turn: self.state.calendar.turn,
            result: None,
            skip: Some(progress),
        })
    }

    /// Deliver what an advance left queued (OVERVIEW §4.3): requests made
    /// on its last day after their applier ran (the director opens a
    /// decision after `player.effects`, so its cast's `heard` would be lost).
    /// The day's effect appliers run once more, on the same date and in
    /// schedule order; requests for a monthly applier wait for the next
    /// advance. So a turn ends with nothing queued, and a save between
    /// turns misses nothing. Nothing runs when nothing is left (a run without
    /// late requests is unchanged), and no RNG is drawn.
    fn settle(&mut self, left: Vec<crate::scheduler::Request>) {
        if left.is_empty() {
            return;
        }
        // `advance_with` steps one day forward: start it from the day
        // before, so the appliers run on today's date.
        let today = self.state.calendar.date;
        self.state.calendar.date = mandate_core::SimDate(today.0.saturating_sub(1));
        let rest = self
            .appliers
            .advance_with(&mut self.state, &self.content, 1, left, &mut |_| {});
        debug_assert_eq!(self.state.calendar.date, today);
        self.state.calendar.date = today;
        // What no applier took is either for a monthly applier, or for the
        // player domain when there is no player (dropped).
        self.pending_effects
            .extend(rest.into_iter().filter(|r| waits_for_monthly(&r.effect)));
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
                crate::spine::prune::due(&mut self.state);
                left -= d;
            }
            progress(y + 1);
        }
        self.state.calendar.phase = prev;
        self.refresh_day();
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
