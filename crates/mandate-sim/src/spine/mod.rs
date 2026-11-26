//! The causal spine (S1, `docs/design/CAUSAL_SPINE.md`): every event and
//! every effect request names what led to it, so "why did this happen?" is
//! answered by walking a graph, never by guessing.
//!
//! This is bookkeeping, not inference. A cause is written down where the
//! engine already knows it: the event a system was reacting to, the command
//! the player gave, the scheduled process whose rule fired, or an explicit
//! mark that something is exogenous by design. Continuous effects (a
//! 0.3-point change in unemployment) are attributed offline by the lab's
//! twins, never here.
//!
//! - [`Cause`]: a typed link to an earlier event ([`Link`]), or a root.
//! - [`Thread`]: the matter an event belongs to (a bill, an ask, a case).
//!   The log links each event to the one before it on its thread, so
//!   lifecycles chain without every site threading ids through its state.
//! - [`graph`]: the queries (back to the roots, forward to the
//!   consequences, the nearest player act, patterns along a chain).
//! - [`prune`]: the bound (age and salience), and what live state holds.

pub mod graph;
pub mod prune;

#[cfg(test)]
mod tests;

use crate::actions::{Action, PlayerCommand};
use crate::characters::CharacterId;
use crate::events::{EventId, EventKind};
use mandate_core::SimDate;
use serde::{Deserialize, Serialize};

/// How an earlier event led to a later one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Link {
    /// It brought this about directly (a recession closed a plant).
    Caused,
    /// The same matter moving on (a bill introduced, then enacted).
    Continues,
    /// Someone acted because of it (an ask after a closure, a case after a
    /// scandal, a decision about a story).
    Prompted,
    /// This made the earlier event known (a story, a scandal breaking).
    Revealed,
    /// This closes what the earlier event opened (a law expires, an ask is
    /// settled, a recovery ends a recession).
    Ended,
}

/// What the player did, as a root (one per command kind).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerAct {
    /// Began the career.
    Start,
    Vote,
    File,
    Draft,
    Amend,
    Introduce,
    Withdraw,
    Move,
    Meet,
    /// Answered a decision.
    Answer,
    /// Let a decision pass: its default happened.
    LetPass,
    Talk,
    /// Answered an ask.
    RespondAsk,
    Promise,
    Lobby,
    TradeFavor,
    TownHall,
    Press,
    Canvass,
    Fundraise,
    Rest,
    Advertise,
    TakeBribe,
    DipFunds,
    LeakFile,
    Visit,
    Casework,
    /// Took part in an exchange scene (began it, said a line, left).
    Exchange,
}

impl PlayerAct {
    pub fn of_action(a: &Action) -> PlayerAct {
        match a {
            Action::Lobby { .. } => PlayerAct::Lobby,
            Action::TradeFavor { .. } => PlayerAct::TradeFavor,
            Action::TownHall { .. } => PlayerAct::TownHall,
            Action::Press { .. } => PlayerAct::Press,
            Action::Canvass { .. } => PlayerAct::Canvass,
            Action::Fundraise => PlayerAct::Fundraise,
            Action::Rest => PlayerAct::Rest,
            Action::Advertise => PlayerAct::Advertise,
            Action::TakeBribe => PlayerAct::TakeBribe,
            Action::DipFunds => PlayerAct::DipFunds,
            Action::LeakFile => PlayerAct::LeakFile,
            Action::Visit { .. } => PlayerAct::Visit,
            Action::Casework { .. } => PlayerAct::Casework,
        }
    }

    /// The act a command is (`Go` is what it goes on to do, else the move).
    pub fn of(cmd: &PlayerCommand) -> PlayerAct {
        match cmd {
            PlayerCommand::CastVote { .. } => PlayerAct::Vote,
            PlayerCommand::File { .. } => PlayerAct::File,
            PlayerCommand::DraftPolicy { .. } | PlayerCommand::DraftFamily { .. } => {
                PlayerAct::Draft
            }
            PlayerCommand::AmendPolicy { .. } => PlayerAct::Amend,
            PlayerCommand::IntroducePolicy { .. } => PlayerAct::Introduce,
            PlayerCommand::WithdrawPolicy { .. } => PlayerAct::Withdraw,
            PlayerCommand::Act { action, .. } => PlayerAct::of_action(action),
            PlayerCommand::MoveTo { .. } => PlayerAct::Move,
            PlayerCommand::Go { then, .. } => {
                then.as_deref().map_or(PlayerAct::Move, PlayerAct::of)
            }
            PlayerCommand::Meet { .. } => PlayerAct::Meet,
            PlayerCommand::Answer { .. } => PlayerAct::Answer,
            PlayerCommand::Talk { .. } => PlayerAct::Talk,
            PlayerCommand::RespondAsk { .. } => PlayerAct::RespondAsk,
            PlayerCommand::Promise { .. } => PlayerAct::Promise,
            PlayerCommand::Exchange { .. } => PlayerAct::Exchange,
        }
    }
}

/// A scheduled process whose standing rule produced something: the
/// calendar, a content trigger that held, a procedure's next stage. A root
/// names the process; what it produced is in the event itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Process {
    /// Time itself: years and turns.
    Calendar,
    /// The economy's monthly step (demand, jobs, wages).
    Markets,
    /// The interest-rate rule.
    CentralBank,
    /// The budget year.
    Budget,
    /// Deliberation, votes, signatures and sunsets, by the constitution's
    /// procedures.
    Lawmaking,
    /// Policy families: proposals open and lapse by their triggers.
    Agenda,
    /// The electoral calendar: campaigns, polls, election day.
    Elections,
    /// Parties' nominations.
    Parties,
    /// The narrative director: decisions, chapters, storylines, warnings.
    Director,
    /// Outlets: stories, scandals, rumours and corrections.
    Press,
    /// Prosecutors and courts.
    Justice,
    /// Asks: offers by their triggers, deadlines.
    Asks,
    /// Residents: opinion, coverage, moving.
    Society,
    /// What being somewhere or holding a post teaches the player.
    Knowledge,
    /// Staged arcs: a run raised by its conditions, an edge that fired, a
    /// beat a stage offers.
    Arcs,
    /// The game's rules were updated under a running career.
    Update,
}

/// A standing condition that crossed a line: not an event, a state of the
/// world the engine's own formula names (the weakest factor of an
/// industry's demand when a plant goes on short time, gets its notice or
/// closes; a track in danger).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pressure {
    /// Weak world demand for what the sector sells.
    WorldDemand,
    /// Low commodity prices.
    CommodityPrices,
    /// The resource is running out.
    Depletion,
    /// The nation's industry is losing ground (the structural trend).
    Competitiveness,
    /// Local wages have outrun prices and productivity.
    LabourCosts,
    /// One of the player's tracks reached danger.
    Track,
    /// Demand in the markets one industry sells to, beyond the world cycle
    /// (DEC-110): the market that weighs most on it and the good, so a
    /// tariff or a closed border is a cause the spine can show.
    ExternalDemand {
        market: crate::economy::industries::MarketId,
        good: crate::economy::industries::GoodId,
    },
    /// Demand at home for what an industry sells here.
    HomeDemand,
    /// The prices of what an industry buys.
    InputCosts,
    /// Less is coming in from the industries nearby that it runs on.
    Supply,
    /// The region is short of water.
    Water,
    /// Idle labour in the town: what draws a new works.
    SlackLabour,
    /// A long boom in an industry's demand: what reopens a works.
    Boom,
}

impl Pressure {
    /// Snake-case name: the stable key views and the audit use.
    pub fn key(self) -> &'static str {
        match self {
            Pressure::WorldDemand => "world_demand",
            Pressure::CommodityPrices => "commodity_prices",
            Pressure::Depletion => "depletion",
            Pressure::Competitiveness => "competitiveness",
            Pressure::LabourCosts => "labour_costs",
            Pressure::Track => "track",
            Pressure::ExternalDemand { .. } => "external_demand",
            Pressure::HomeDemand => "home_demand",
            Pressure::InputCosts => "input_costs",
            Pressure::Supply => "supply",
            Pressure::Water => "water",
            Pressure::SlackLabour => "slack_labour",
            Pressure::Boom => "boom",
        }
    }
}

/// Exogenous by design: uncaused inside the simulation, and saying so.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shock {
    /// The world economy: recessions.
    WorldEconomy,
    /// World commodity markets.
    Commodities,
    /// Weather and the ground: floods, storms, fires, droughts, quakes.
    Hazard,
    /// The part of a race nobody controls (the race shock).
    Race,
}

/// What led to an event or an effect request: a typed link to an earlier
/// event, or a root (where the chain starts).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    /// An earlier event, and how it led here.
    Event(EventId, Link),
    /// The player's command (their proxy's during a time skip: a skip is
    /// the player's turns, played for them under their directives).
    Player(PlayerAct),
    /// Someone else chose to (their own agency; who is in the event too).
    Actor(CharacterId),
    /// A scheduled process applying its standing rule.
    Process(Process),
    /// A standing condition the engine's formula names.
    Pressure(Pressure),
    /// Worldgen: the world as it was made.
    History,
    /// Exogenous by design.
    Exogenous(Shock),
    /// A lab intervention (never in a player's game).
    Lab,
    /// An earlier event that is no longer kept (the log's hard cap).
    Forgotten,
    /// Recorded before the spine (an older save): not known, not invented.
    Unknown,
}

impl Cause {
    /// A link to an earlier event that directly caused this.
    pub fn caused_by(id: EventId) -> Cause {
        Cause::Event(id, Link::Caused)
    }

    /// A link, or the fallback when there is no event to link to.
    pub fn event_or(id: Option<EventId>, link: Link, fallback: Cause) -> Cause {
        match id {
            Some(id) => Cause::Event(id, link),
            None => fallback,
        }
    }

    pub fn event(&self) -> Option<(EventId, Link)> {
        match self {
            Cause::Event(id, link) => Some((*id, *link)),
            _ => None,
        }
    }

    /// A root: where a chain starts (anything but a link).
    pub fn is_root(&self) -> bool {
        !matches!(self, Cause::Event(..))
    }

    /// The player's act, if this is one.
    pub fn player_act(&self) -> Option<PlayerAct> {
        match self {
            Cause::Player(a) => Some(*a),
            _ => None,
        }
    }

    /// Named: anything but `Unknown` and `Forgotten`.
    pub fn is_known(&self) -> bool {
        !matches!(self, Cause::Unknown | Cause::Forgotten)
    }

    pub fn class(&self) -> CauseClass {
        match self {
            Cause::Event(..) => CauseClass::Linked,
            Cause::Player(_) => CauseClass::Player,
            Cause::Actor(_) => CauseClass::Actor,
            Cause::Process(_) | Cause::History | Cause::Lab => CauseClass::Process,
            Cause::Pressure(_) => CauseClass::Pressure,
            Cause::Exogenous(_) => CauseClass::Exogenous,
            Cause::Forgotten | Cause::Unknown => CauseClass::Unknown,
        }
    }
}

/// A cause's class, for coverage tables (the audit).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CauseClass {
    /// Linked to an earlier event.
    Linked,
    Player,
    Actor,
    Pressure,
    Process,
    Exogenous,
    Unknown,
}

impl CauseClass {
    pub fn key(self) -> &'static str {
        match self {
            CauseClass::Linked => "linked",
            CauseClass::Player => "player",
            CauseClass::Actor => "actor",
            CauseClass::Pressure => "pressure",
            CauseClass::Process => "process",
            CauseClass::Exogenous => "exogenous",
            CauseClass::Unknown => "unknown",
        }
    }
}

/// The best class among an event's causes: a link to an earlier event or a
/// named act explains more than a process root (the order of [`CauseClass`]).
pub fn best_class(causes: &[Cause]) -> CauseClass {
    causes
        .iter()
        .map(Cause::class)
        .min()
        .unwrap_or(CauseClass::Unknown)
}

/// One or more causes: never empty, so nothing is recorded uncaused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Causes(Vec<Cause>);

impl Causes {
    pub fn and(mut self, c: Cause) -> Causes {
        if !self.0.contains(&c) {
            self.0.push(c);
        }
        self
    }

    /// Add a link when there is an event to link to.
    pub fn and_event(self, id: Option<EventId>, link: Link) -> Causes {
        match id {
            Some(id) => self.and(Cause::Event(id, link)),
            None => self,
        }
    }

    pub fn into_vec(self) -> Vec<Cause> {
        self.0
    }

    pub fn as_slice(&self) -> &[Cause] {
        &self.0
    }
}

impl From<Cause> for Causes {
    fn from(c: Cause) -> Self {
        Causes(vec![c])
    }
}

impl<const N: usize> From<[Cause; N]> for Causes {
    fn from(cs: [Cause; N]) -> Self {
        let mut out = Causes(Vec::with_capacity(N.max(1)));
        for c in cs {
            if !out.0.contains(&c) {
                out.0.push(c);
            }
        }
        if out.0.is_empty() {
            out.0.push(Cause::Unknown);
        }
        out
    }
}

/// The matter an event belongs to. The log keeps the latest event of each
/// thread and links the next one to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Thread {
    /// A bill, then a law: introduced, voted, in force, ended.
    Bill(u32),
    /// A policy family's proposal on the table.
    Proposal(u32),
    Ask(u32),
    Decision(u32),
    /// A hidden fact, from the day it can be discovered.
    Hidden(u32),
    /// A scandal, from the day it broke.
    Scandal(u32),
    /// A case with prosecutors.
    Case(u32),
    Rumour(u32),
    /// A storyline run.
    Storyline(u32),
    /// The world recession, from its (possibly held) onset to recovery.
    Recession,
    /// An election, by its day: campaign, races, the count.
    Election(u32),
    /// The player's prison term.
    Sentence,
    /// An arc run (DEC-109): raised, its stages, its beats, its exit.
    Arc(u32),
    /// A plant's review (DEC-110): short time, the notice, then the
    /// reprieve or the closure.
    Plant(u32),
}

/// What an event does to its thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Starts it (no link back).
    Opens,
    /// Moves it on: linked `Continues`, and becomes the thread's latest.
    Advances,
    /// Belongs to it without moving it on: linked `Continues` only.
    Notes,
    /// Ends it: linked `Ended` (it stays the thread's latest event, so
    /// what is said of it afterwards links to how it ended).
    Closes,
}

/// The thread an event belongs to, and its part in it.
pub fn thread(kind: &EventKind, date: SimDate) -> Option<(Thread, Role)> {
    use EventKind as K;
    Some(match kind {
        K::PolicyIntroduced { policy, .. } => (Thread::Bill(*policy), Role::Opens),
        // The player's vote is a step in the bill's life: what follows on
        // the bill links to it (credit and blame read which way it went).
        K::PlayerDeed {
            deed: crate::asks::Deed::Vote { policy, .. },
        } => (Thread::Bill(*policy), Role::Advances),
        K::PlayerDeed {
            deed: crate::asks::Deed::Introduce { policy },
        } => (Thread::Bill(*policy), Role::Notes),
        K::PolicyEnacted { policy, .. } => (Thread::Bill(*policy), Role::Advances),
        K::PolicyFailed { policy, .. }
        | K::PolicyExpired { policy, .. }
        | K::PolicyWithdrawn { policy, .. } => (Thread::Bill(*policy), Role::Closes),
        K::ProposalOpened { proposal, .. } => (Thread::Proposal(*proposal), Role::Opens),
        K::ProposalLapsed { proposal, .. } => (Thread::Proposal(*proposal), Role::Closes),
        K::AskOffered { ask, .. } => (Thread::Ask(*ask), Role::Opens),
        // A promise is accepted as it is made: the thread starts here.
        K::AskAccepted { ask, .. } => (Thread::Ask(*ask), Role::Advances),
        K::AskResolved { ask, .. } => (Thread::Ask(*ask), Role::Closes),
        K::DecisionOpened { decision, .. } => (Thread::Decision(*decision), Role::Opens),
        K::DecisionResolved { decision, .. } | K::DecisionLapsed { decision, .. } => {
            (Thread::Decision(*decision), Role::Closes)
        }
        K::FactDiscoverable { hidden } => (Thread::Hidden(*hidden), Role::Opens),
        K::ScandalBroke { scandal, .. } => (Thread::Scandal(*scandal), Role::Opens),
        K::CaseOpened { case, .. } => (Thread::Case(*case), Role::Opens),
        K::CaseDeveloped { case, stage, .. } => (
            Thread::Case(*case),
            if matches!(stage.as_str(), "dropped" | "acquitted" | "convicted") {
                Role::Closes
            } else {
                Role::Advances
            },
        ),
        K::RumourSpread { rumour, .. } => (Thread::Rumour(*rumour), Role::Opens),
        K::RumourCorrected { rumour, .. } => (Thread::Rumour(*rumour), Role::Closes),
        K::StorylineStage { run, .. } => (Thread::Storyline(*run), Role::Advances),
        K::GlobalRecessionBegan { .. } => (Thread::Recession, Role::Advances),
        K::GlobalRecoveryBegan => (Thread::Recession, Role::Closes),
        K::CampaignOpened { election_day } => (Thread::Election(election_day.0), Role::Opens),
        K::RaceDecided { .. } => (Thread::Election(date.0), Role::Notes),
        K::ElectionHeld { .. } => (Thread::Election(date.0), Role::Closes),
        K::PlantOpened { plant, .. } | K::PlantShortTime { plant, .. } => {
            (Thread::Plant(*plant), Role::Opens)
        }
        K::PlantThreatened { plant, .. } => (Thread::Plant(*plant), Role::Advances),
        K::PlantReprieved { plant, .. }
        | K::PlantClosure {
            plant: Some(plant), ..
        } => (Thread::Plant(*plant), Role::Closes),
        K::ArcRaised { arc, .. } => (Thread::Arc(*arc), Role::Opens),
        K::ArcStage { arc, .. } => (Thread::Arc(*arc), Role::Advances),
        K::ArcBeat { arc, .. } | K::ArcExchange { arc, .. } | K::ArcExchangeEnded { arc, .. } => {
            (Thread::Arc(*arc), Role::Notes)
        }
        K::ArcEnded { arc, .. } => (Thread::Arc(*arc), Role::Closes),
        K::PlayerImprisoned { .. } => (Thread::Sentence, Role::Opens),
        K::PlayerReleased => (Thread::Sentence, Role::Closes),
        K::DirectorIntervention { lever, .. } => match lever.as_str() {
            "recession_held" => (Thread::Recession, Role::Opens),
            "recession_released" => (Thread::Recession, Role::Advances),
            // A held hazard is not a thread: a town can have several, so
            // each hold's end links to its own hold (`DeferredHazard`).
            _ => return None,
        },
        _ => return None,
    })
}

/// How long an event matters: the pruning rule's salience classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Salience {
    /// Bookkeeping: turns, years, polls, the player's deed log, levers.
    Trace,
    /// Ordinary political and economic life.
    Routine,
    /// What a town remembers for a generation.
    Notable,
    /// What a career is told by: kept for the whole run.
    Landmark,
}

/// An event kind's salience.
pub fn salience(kind: &EventKind) -> Salience {
    use EventKind as K;
    match kind {
        K::TurnEnded { .. }
        | K::YearBegan { .. }
        | K::CalendarEvent { .. }
        | K::PollsPublished { .. }
        | K::PlayerDeed { .. }
        | K::DirectorIntervention { .. }
        | K::FactDiscoverable { .. }
        | K::InterestRateMove { .. }
        | K::StoryDevelopment { .. } => Salience::Trace,
        K::CommodityShock { .. }
        | K::IndustryShock { .. }
        | K::PlantShortTime { .. }
        | K::NationalBudget { .. }
        | K::PolicyIntroduced { .. }
        | K::PolicyWithdrawn { .. }
        | K::PolicyExpired { .. }
        | K::CampaignOpened { .. }
        | K::ElectionHeld { .. }
        | K::DecisionOpened { .. }
        | K::DecisionLapsed { .. }
        | K::StorylineStage { .. }
        | K::RumourSpread { .. }
        | K::RumourCorrected { .. }
        | K::AskOffered { .. }
        | K::AskAccepted { .. }
        | K::ProposalOpened { .. }
        | K::ProposalLapsed { .. }
        | K::TrackWarning { .. }
        | K::ArcBeat { .. }
        | K::ArcExchange { .. }
        | K::CaseDeveloped { .. } => Salience::Routine,
        K::RaceDecided {
            player_ran: false, ..
        } => Salience::Routine,
        K::GlobalRecessionBegan { .. }
        | K::GlobalRecoveryBegan
        | K::PlantClosure { .. }
        | K::PlantThreatened { .. }
        | K::PlantReprieved { .. }
        | K::PlantOpened { .. }
        | K::Disaster { .. }
        | K::PolicyEnacted { .. }
        | K::PolicyFailed { .. }
        | K::DecisionResolved { .. }
        | K::ChapterBegan { .. }
        | K::ScandalBroke { .. }
        | K::CaseOpened { .. }
        | K::ArcRaised { .. }
        | K::ArcStage { .. }
        | K::ArcEnded { .. }
        | K::ArcExchangeEnded { .. }
        | K::AskResolved { .. } => Salience::Notable,
        K::GameStarted { .. }
        | K::HistoryBegan { .. }
        | K::RaceDecided {
            player_ran: true, ..
        }
        | K::PlayerDeselected
        | K::NominationRefused { .. }
        | K::PlayerImprisoned { .. }
        | K::PlayerReleased
        | K::CareerEnded { .. }
        | K::RulesUpdated { .. } => Salience::Landmark,
    }
}

/// The kinds the "nothing significant is uncaused" test and the audit
/// read: everything above bookkeeping.
pub fn significant(kind: &EventKind) -> bool {
    salience(kind) > Salience::Trace
}

/// An event kind's serde tag (`plant_closure`): the stable name the
/// protocol and the audit use.
pub fn kind_key(kind: &EventKind) -> &'static str {
    use EventKind as K;
    match kind {
        K::GameStarted { .. } => "game_started",
        K::HistoryBegan { .. } => "history_began",
        K::YearBegan { .. } => "year_began",
        K::TurnEnded { .. } => "turn_ended",
        K::CalendarEvent { .. } => "calendar_event",
        K::GlobalRecessionBegan { .. } => "global_recession_began",
        K::GlobalRecoveryBegan => "global_recovery_began",
        K::CommodityShock { .. } => "commodity_shock",
        K::PlantClosure { .. } => "plant_closure",
        K::PlantShortTime { .. } => "plant_short_time",
        K::PlantThreatened { .. } => "plant_threatened",
        K::PlantReprieved { .. } => "plant_reprieved",
        K::PlantOpened { .. } => "plant_opened",
        K::IndustryShock { .. } => "industry_shock",
        K::InterestRateMove { .. } => "interest_rate_move",
        K::Disaster { .. } => "disaster",
        K::NationalBudget { .. } => "national_budget",
        K::PolicyIntroduced { .. } => "policy_introduced",
        K::PolicyEnacted { .. } => "policy_enacted",
        K::PolicyFailed { .. } => "policy_failed",
        K::PolicyExpired { .. } => "policy_expired",
        K::PolicyWithdrawn { .. } => "policy_withdrawn",
        K::CampaignOpened { .. } => "campaign_opened",
        K::PollsPublished { .. } => "polls_published",
        K::RaceDecided { .. } => "race_decided",
        K::ElectionHeld { .. } => "election_held",
        K::PlayerDeselected => "player_deselected",
        K::NominationRefused { .. } => "nomination_refused",
        K::DecisionOpened { .. } => "decision_opened",
        K::DecisionResolved { .. } => "decision_resolved",
        K::DecisionLapsed { .. } => "decision_lapsed",
        K::ChapterBegan { .. } => "chapter_began",
        K::StorylineStage { .. } => "storyline_stage",
        K::StoryDevelopment { .. } => "story_development",
        K::TrackWarning { .. } => "track_warning",
        K::DirectorIntervention { .. } => "director_intervention",
        K::FactDiscoverable { .. } => "fact_discoverable",
        K::ScandalBroke { .. } => "scandal_broke",
        K::CaseOpened { .. } => "case_opened",
        K::CaseDeveloped { .. } => "case_developed",
        K::PlayerImprisoned { .. } => "player_imprisoned",
        K::PlayerReleased => "player_released",
        K::CareerEnded { .. } => "career_ended",
        K::RumourSpread { .. } => "rumour_spread",
        K::RumourCorrected { .. } => "rumour_corrected",
        K::PlayerDeed { .. } => "player_deed",
        K::AskOffered { .. } => "ask_offered",
        K::AskAccepted { .. } => "ask_accepted",
        K::AskResolved { .. } => "ask_resolved",
        K::RulesUpdated { .. } => "rules_updated",
        K::ProposalOpened { .. } => "proposal_opened",
        K::ProposalLapsed { .. } => "proposal_lapsed",
        K::ArcRaised { .. } => "arc_raised",
        K::ArcStage { .. } => "arc_stage",
        K::ArcBeat { .. } => "arc_beat",
        K::ArcEnded { .. } => "arc_ended",
        K::ArcExchange { .. } => "arc_exchange",
        K::ArcExchangeEnded { .. } => "arc_exchange_ended",
    }
}

/// Effect requests applied, for the "nothing is uncaused" test. Not part
/// of the state: never saved, never hashed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EffectAudit {
    /// Requests an applier took.
    pub applied: u64,
    /// Of those, with no named cause (`Unknown` or `Forgotten`).
    pub unnamed: u64,
}

impl EffectAudit {
    pub fn note(&mut self, cause: &Cause) {
        self.applied += 1;
        if !cause.is_known() {
            self.unnamed += 1;
        }
    }
}

/// Someone's own choice, as a root (`Unknown` if they do not exist).
pub fn actor(
    politics: &crate::politics::PoliticsState,
    who: crate::politics::PoliticianId,
) -> Cause {
    politics
        .get(who)
        .map_or(Cause::Unknown, |p| Cause::Actor(p.character))
}

/// The opening of the proposal a bill answers: none for a bill from the
/// catalogue, and none once the proposal has lapsed (a closed thread's
/// last event is how it ended, which prompted nothing).
pub fn proposal_of(state: &crate::State, bill: crate::policy::PolicyInstanceId) -> Option<EventId> {
    let f = state
        .policy
        .families
        .instances
        .iter()
        .find(|i| i.drafts.contains(&bill))?;
    state.events.open_head(Thread::Proposal(f.id.0))
}
