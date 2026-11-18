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
        municipality: u32,
        mayor: bool,
        winner: u32,
        incumbent_defeated: bool,
        player_won: bool,
        player_ran: bool,
    },
    ElectionHeld {
        races: u32,
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
