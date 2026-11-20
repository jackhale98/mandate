//! Domain event log with causal references (OVERVIEW §4.4).

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
}

/// How long an event is kept in full detail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Retention {
    Recent,
    Pinned,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainEvent {
    pub id: EventId,
    pub date: SimDate,
    pub kind: EventKind,
    pub causes: Vec<EventId>,
    pub retention: Retention,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventLog {
    ids: IdAllocator,
    events: Vec<DomainEvent>,
}

/// Recent events kept in full detail; older `Recent` events are dropped.
pub const RECENT_CAPACITY: usize = 2_000;

impl EventLog {
    pub fn record(&mut self, date: SimDate, kind: EventKind, causes: Vec<EventId>) -> EventId {
        let id: EventId = self.ids.alloc();
        self.events.push(DomainEvent {
            id,
            date,
            kind,
            causes,
            retention: Retention::Recent,
        });
        self.compact();
        id
    }

    pub fn pin(&mut self, id: EventId) {
        if let Some(e) = self.events.iter_mut().find(|e| e.id == id) {
            e.retention = Retention::Pinned;
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

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    fn compact(&mut self) {
        let recent = self
            .events
            .iter()
            .filter(|e| e.retention == Retention::Recent)
            .count();
        if recent > RECENT_CAPACITY {
            let mut to_drop = recent - RECENT_CAPACITY;
            self.events.retain(|e| {
                if to_drop > 0 && e.retention == Retention::Recent {
                    to_drop -= 1;
                    false
                } else {
                    true
                }
            });
        }
    }
}
