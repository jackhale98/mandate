//! Domain event log with causal references (OVERVIEW §4.4): the nodes of
//! the causal spine (`crate::spine`, S1).

use crate::spine::{self, Cause, Causes, EffectAudit, Link, Role, Thread};
use mandate_core::{IdAllocator, SimDate, define_id};
use serde::{Deserialize, Serialize};

define_id!(
    /// Identifier of a logged domain event.
    EventId
);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    GameStarted {
        seed: u64,
    },
    HistoryBegan {
        year: u32,
    },
    YearBegan {
        year: u32,
    },
    TurnEnded {
        turn: u32,
    },
    GlobalRecessionBegan {
        depth_pct: u32,
    },
    GlobalRecoveryBegan,
    /// Commodity index: 0 agriculture, 1 minerals, 2 energy.
    CommodityShock {
        commodity: u8,
        change_pct: i32,
    },
    PlantClosure {
        municipality: u32,
        /// Content key of the sector (events are long-lived, so never an index).
        sector: mandate_content::ContentKey,
        jobs: u32,
        /// The industry, the plant and the weakest factor of its demand
        /// (DEC-110); absent in events from before named plants.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        industry: Option<mandate_content::ContentKey>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        plant: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        factor: Option<crate::economy::industries::Factor>,
        /// The upstream industry or sector, when the factor is supply.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        upstream: Option<mandate_content::ContentKey>,
    },
    /// A plant went on short time (DEC-110): the first step of its review.
    PlantShortTime {
        municipality: u32,
        plant: u32,
        industry: mandate_content::ContentKey,
        jobs: u32,
        factor: crate::economy::industries::Factor,
    },
    /// A closure notice was served: the plant closes on `closes_on` unless
    /// demand comes back. The date shown is the date used.
    PlantThreatened {
        municipality: u32,
        plant: u32,
        industry: mandate_content::ContentKey,
        jobs: u32,
        closes_on: SimDate,
        factor: crate::economy::industries::Factor,
    },
    /// The notice was lifted before its date.
    PlantReprieved {
        municipality: u32,
        plant: u32,
        industry: mandate_content::ContentKey,
        jobs: u32,
    },
    /// A plant opened (`reopened`: at a works that had closed).
    PlantOpened {
        municipality: u32,
        plant: u32,
        industry: mandate_content::ContentKey,
        jobs: u32,
        reopened: bool,
    },
    /// An industry's demand index in a market jumped (exogenous).
    IndustryShock {
        industry: mandate_content::ContentKey,
        /// Content key of the market.
        market: mandate_content::ContentKey,
        change_pct: i32,
    },
    InterestRateMove {
        change_bp: i32,
        rate_bp: u32,
    },
    Disaster {
        municipality: u32,
        hazard: crate::environment::HazardKind,
        severity_pct: u32,
    },
    NationalBudget {
        year: u32,
        balance: i64,
        debt_pct_gdp: u32,
    },
    PolicyIntroduced {
        policy: u32,
        municipality: u32,
        sponsor: u32,
    },
    PolicyEnacted {
        policy: u32,
        municipality: u32,
        yes: u32,
        no: u32,
        overrode_veto: bool,
    },
    PolicyFailed {
        policy: u32,
        municipality: u32,
        vetoed: bool,
        yes: u32,
        no: u32,
    },
    PolicyExpired {
        policy: u32,
        municipality: u32,
    },
    PolicyWithdrawn {
        policy: u32,
        municipality: u32,
    },
    /// Local campaign season begins (candidates declared).
    CampaignOpened {
        election_day: SimDate,
    },
    PollsPublished {
        count: u32,
    },
    RaceDecided {
        /// The jurisdiction's raw id (the town, for a local race).
        municipality: u32,
        /// The race was for an office (not a seat in a body).
        mayor: bool,
        winner: u32,
        incumbent_defeated: bool,
        player_won: bool,
        player_ran: bool,
        /// The post decided (DEC-098); absent in events from older saves.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        post: Option<crate::politics::Post>,
    },
    ElectionHeld {
        races: u32,
    },
    /// The player's party refused to renominate them (Standing too low).
    PlayerDeselected,
    /// The player filed for a post they do not hold and their party did not
    /// nominate them for it (play note 20). `reason`: `holder` (the party's
    /// sitting holder stands again), `standing` (below the post's gate,
    /// `min_standing`), `party` (the party fields no one there), `other`.
    /// `instead`: what the player stands for now (the seat they hold), or
    /// nothing.
    NominationRefused {
        post: crate::politics::Post,
        reason: String,
        /// The party's nominee for the post (the holder, for `holder`).
        nominee: Option<u32>,
        /// The player's Standing when the campaign opened.
        standing: u32,
        min_standing: Option<u32>,
        instead: Option<crate::politics::Post>,
    },
    /// The director spotlighted a decision (DEC-033, DEC-077 §4).
    DecisionOpened {
        decision: u32,
        /// Dilemma content id.
        template: String,
    },
    /// A decision was answered (or its default happened at the deadline).
    DecisionResolved {
        decision: u32,
        template: String,
        option: String,
        by_default: bool,
    },
    /// A decision's situation dissolved before an answer.
    DecisionLapsed {
        decision: u32,
        template: String,
    },
    /// A chapter began (DEC-005): `session` or `campaign`.
    ChapterBegan {
        index: u32,
        kind: String,
    },
    /// A storyline reached a stage (DEC-032).
    StorylineStage {
        run: u32,
        storyline: String,
        stage: String,
    },
    /// A development the director spotlighted (an entry in the facts log).
    StoryDevelopment {
        fact: u32,
        municipality: u32,
    },
    /// A pressure track reached danger (DEC-006 fairness warning).
    TrackWarning {
        track: String,
        value: i32,
    },
    /// The director's exogenous lever was applied by the owning system
    /// (`hazard_held`, `hazard_struck_late`, `hazard_averted`,
    /// `recession_held`, `recession_released`).
    DirectorIntervention {
        lever: String,
        municipality: u32,
    },
    /// A hidden fact may now be discovered (DEC-034's hook).
    FactDiscoverable {
        hidden: u32,
    },
    /// A hidden fact came out (DEC-034): the story broke.
    ScandalBroke {
        scandal: u32,
        politician: u32,
        municipality: u32,
        /// `ethics` | `misconduct` | `crime`.
        severity: String,
        /// The fact kind (Fluent `fact-<kind>`).
        kind: String,
    },
    /// Prosecutors opened an investigation (DEC-006).
    CaseOpened {
        case: u32,
        politician: u32,
    },
    /// A case moved on: `charged`, `dropped`, `acquitted`, `convicted`.
    CaseDeveloped {
        case: u32,
        politician: u32,
        stage: String,
    },
    /// The player went to prison (out of politics until `until`).
    PlayerImprisoned {
        years: u32,
        until: SimDate,
    },
    PlayerReleased,
    /// The career is over (DEC-006 terminal ending): `prison`, `disqualified`.
    CareerEnded {
        reason: String,
    },
    /// An unverified claim began to spread (DEC-034 rumours).
    RumourSpread {
        rumour: u32,
        politician: u32,
    },
    RumourCorrected {
        rumour: u32,
        politician: u32,
    },
    /// What the player did (DEC-097 §2): read by the asks system into the
    /// deed log; not news.
    PlayerDeed {
        deed: crate::asks::Deed,
    },
    /// Someone asked the player for something (an offer, DEC-097 §2).
    AskOffered {
        ask: u32,
        /// Ask template id.
        template: String,
        big: bool,
    },
    /// The player accepted an ask, or made a campaign promise.
    AskAccepted {
        ask: u32,
        template: String,
        big: bool,
    },
    /// An ask or promise ended: `kept`, `broken`, `declined` or `void`.
    AskResolved {
        ask: u32,
        template: String,
        outcome: String,
        public: bool,
    },
    /// The game's rules were updated under a running career (an app update).
    RulesUpdated {
        from_content: String,
        to_content: String,
        from_engine: String,
        to_engine: String,
    },
    /// A policy family's proposal came onto the table (DEC-092).
    ProposalOpened {
        proposal: u32,
        family: String,
        at: crate::world::Jurisdiction,
    },
    /// A proposal with no bill went away (expiry, eased, too long, removed).
    ProposalLapsed {
        proposal: u32,
        family: String,
        at: crate::world::Jurisdiction,
    },
    /// A calendar occasion of the turn just played (DEC-114): held on the
    /// day played (`part`), or reported as having happened on its own date
    /// (`part` none); whether the player came, and whether an expected
    /// absence cost them (`missed`).
    CalendarEvent {
        municipality: u32,
        /// Calendar event content id.
        event: String,
        /// Its name in the town (a Fluent key with `$town`).
        name_key: String,
        on: SimDate,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        part: Option<String>,
        attended: bool,
        missed: bool,
    },
    /// An arc was raised (DEC-109): its `raise` held at a jurisdiction and
    /// its cast was bound. Opens the run's thread.
    ArcRaised {
        arc: u32,
        /// Arc content id.
        template: String,
        at: crate::world::Jurisdiction,
    },
    /// A run entered a stage. `facts`: its dates and readings as they
    /// were (numbers and Fluent keys; names are resolved from its cast).
    ArcStage {
        arc: u32,
        template: String,
        stage: String,
        at: crate::world::Jurisdiction,
        facts: Vec<(String, String)>,
    },
    /// A beat of a run happened: `kind` is the beat's (`visit`, `talk`,
    /// `decision`, `ask`, `move`, `press`); `ok`: it was done (a reply
    /// agreed, a decision was answered, an ask was kept).
    ArcBeat {
        arc: u32,
        template: String,
        stage: String,
        beat: String,
        at: crate::world::Jurisdiction,
        kind: String,
        ok: bool,
        facts: Vec<(String, String)>,
    },
    /// A run reached an exit. Closes its thread.
    ArcEnded {
        arc: u32,
        template: String,
        exit: String,
        at: crate::world::Jurisdiction,
        facts: Vec<(String, String)>,
    },
    /// A round of an exchange scene was played (DEC-112): the prompt's
    /// `source`, the line the player said (`option`, empty when the round
    /// went unanswered) and whether it stood up.
    ArcExchange {
        arc: u32,
        template: String,
        stage: String,
        beat: String,
        at: crate::world::Jurisdiction,
        round: u8,
        source: String,
        option: String,
        stood: bool,
    },
    /// An exchange scene ended: `won`, `split`, `lost` or `absent`, with
    /// the weighted change in lean in thousandths.
    ArcExchangeEnded {
        arc: u32,
        template: String,
        stage: String,
        beat: String,
        at: crate::world::Jurisdiction,
        outcome: String,
        result_pm: i32,
        facts: Vec<(String, String)>,
    },
    /// A fault on a neighbourhood's streets (DEC-114, upkeep): opened,
    /// moved up the repair queue, paid for and being repaired (`other`: the
    /// fault it went ahead of), put back (`other`: the fault that took its
    /// money), or fixed.
    Fault {
        fault: u32,
        template: mandate_content::ContentKey,
        hood: u32,
        stage: crate::upkeep::FaultStage,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        other: Option<u32>,
    },
}

/// How long an event is kept in full detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Retention {
    /// Kept by the spine's rule: age by salience, and while live state
    /// holds it (`spine::prune`).
    Recent,
    /// A landmark whatever its kind: kept for the run.
    Pinned,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainEvent {
    pub id: EventId,
    pub date: SimDate,
    pub kind: EventKind,
    /// What led to it (the causal spine, S1): links to earlier events and
    /// roots. Never empty.
    pub causes: Vec<Cause>,
    pub retention: Retention,
}

impl DomainEvent {
    /// The earlier events it names, with how each led here.
    pub fn links(&self) -> impl Iterator<Item = (EventId, Link)> + '_ {
        self.causes.iter().filter_map(Cause::event)
    }

    /// Its roots: every cause that is not a link.
    pub fn roots(&self) -> impl Iterator<Item = &Cause> + '_ {
        self.causes.iter().filter(|c| c.is_root())
    }
}

/// The event log: the causal graph's nodes, in id order (so in the order
/// recorded), bounded by `spine::prune`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventLog {
    ids: IdAllocator,
    pub(crate) events: Vec<DomainEvent>,
    /// The latest kept event of each thread, sorted by thread
    /// (`spine::Thread`): the next event on a thread links to it. A thread
    /// leaves when its latest event leaves the log. The flag: that event
    /// closed the thread.
    #[serde(default)]
    heads: Vec<(Thread, EventId, bool)>,
    /// When the log was last pruned.
    #[serde(default)]
    pruned_on: Option<SimDate>,
    /// Effect requests applied (the spine's audit; not state).
    #[serde(skip)]
    audit: EffectAudit,
    /// Lab switch (not state): keep everything. For tools that must read
    /// every event (the proof that the spine changes no outcome).
    #[serde(skip)]
    keep_all: bool,
}

impl EventLog {
    /// Record an event with what led to it. The log adds the link to the
    /// event before it on its thread (a bill's introduction to its
    /// enactment), so a site names only what the thread cannot know: the
    /// root, and links across threads.
    pub fn record(&mut self, date: SimDate, kind: EventKind, causes: impl Into<Causes>) -> EventId {
        self.record_caused(date, kind, causes.into().into_vec())
    }

    /// `record`, once (not per cause type: the body is not small).
    fn record_caused(&mut self, date: SimDate, kind: EventKind, mut causes: Vec<Cause>) -> EventId {
        let id: EventId = self.ids.alloc();
        if let Some((thread, role)) = spine::thread(&kind, date) {
            let at = self.heads.binary_search_by_key(&thread, |h| h.0);
            // A thread that ended starts afresh when something moves it on
            // again (the next recession is not the last one's sequel);
            // what only notes or closes it again still links to its end.
            let fresh = role == Role::Opens
                || (role == Role::Advances && at.is_ok_and(|i| self.heads[i].2));
            if let (Ok(i), false) = (at, fresh) {
                let head = self.heads[i].1;
                if !causes
                    .iter()
                    .any(|c| c.event().is_some_and(|e| e.0 == head))
                {
                    let link = if role == Role::Closes {
                        Link::Ended
                    } else {
                        Link::Continues
                    };
                    causes.push(Cause::Event(head, link));
                }
            }
            let closed = role == Role::Closes;
            match (role, at) {
                (Role::Notes, _) => {}
                (_, Ok(i)) => self.heads[i] = (thread, id, closed),
                (_, Err(i)) => self.heads.insert(i, (thread, id, closed)),
            }
        }
        self.events.push(DomainEvent {
            id,
            date,
            kind,
            causes,
            retention: Retention::Recent,
        });
        id
    }

    /// Keep an event for the run, whatever its kind.
    pub fn pin(&mut self, id: EventId) {
        if let Ok(i) = self.events.binary_search_by_key(&id, |e| e.id) {
            self.events[i].retention = Retention::Pinned;
        }
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &DomainEvent> {
        self.events.iter()
    }

    pub fn get(&self, id: EventId) -> Option<&DomainEvent> {
        self.events
            .binary_search_by_key(&id, |e| e.id)
            .ok()
            .map(|i| &self.events[i])
    }

    /// The events recorded after `id`, in order.
    pub fn after(&self, id: EventId) -> &[DomainEvent] {
        let from = self.events.partition_point(|e| e.id <= id);
        &self.events[from..]
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// The latest event of a thread (the closing one, once it is closed),
    /// while the log keeps it.
    pub fn head(&self, thread: Thread) -> Option<EventId> {
        self.heads
            .binary_search_by_key(&thread, |h| h.0)
            .ok()
            .map(|i| self.heads[i].1)
    }

    /// The latest event of a thread that is still open: none once it has
    /// closed (a lapsed proposal prompts no bill).
    pub fn open_head(&self, thread: Thread) -> Option<EventId> {
        self.heads
            .binary_search_by_key(&thread, |h| h.0)
            .ok()
            .filter(|i| !self.heads[*i].2)
            .map(|i| self.heads[i].1)
    }

    /// The threads, sorted, each with its latest event.
    pub fn heads(&self) -> &[(Thread, EventId, bool)] {
        &self.heads
    }

    /// The threads as replaying the kept events gives them (what a save
    /// from before the spine is given).
    pub fn replayed_heads(&self) -> Vec<(Thread, EventId, bool)> {
        let mut heads: Vec<(Thread, EventId, bool)> = Vec::new();
        for e in &self.events {
            let Some((thread, role)) = spine::thread(&e.kind, e.date) else {
                continue;
            };
            let at = heads.binary_search_by_key(&thread, |h| h.0);
            let closed = role == Role::Closes;
            match (role, at) {
                (Role::Notes, _) => {}
                (_, Ok(i)) => heads[i] = (thread, e.id, closed),
                (_, Err(i)) => heads.insert(i, (thread, e.id, closed)),
            }
        }
        heads
    }

    /// Threads whose latest event is no longer kept are forgotten.
    pub(crate) fn drop_dead_heads(&mut self) {
        let events = &self.events;
        self.heads
            .retain(|(_, id, _)| events.binary_search_by_key(id, |e| e.id).is_ok());
    }

    /// Prune by the spine's rule if a month has passed since the last
    /// time (`spine::prune`). `holds` is read only when it is time.
    pub fn prune_if_due(
        &mut self,
        today: SimDate,
        holds: impl FnOnce() -> spine::prune::Holds,
    ) -> Option<spine::prune::Pruned> {
        if self.keep_all
            || self
                .pruned_on
                .is_some_and(|on| on.days_until(today) < spine::prune::PRUNE_EVERY_DAYS)
        {
            return None;
        }
        self.pruned_on = Some(today);
        Some(spine::prune::prune(self, today, &holds()))
    }

    /// The id the next event will get.
    pub fn next_id(&self) -> EventId {
        EventId(self.ids.peek_next())
    }

    /// Lab switch: never prune (not saved; a loaded game prunes again).
    pub fn set_keep_all(&mut self, on: bool) {
        self.keep_all = on;
    }

    /// An effect request was applied (the spine's audit).
    pub fn note_effect(&mut self, cause: &Cause) {
        self.audit.note(cause);
    }

    pub fn effect_audit(&self) -> EffectAudit {
        self.audit
    }
}
