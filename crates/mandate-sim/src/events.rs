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
    GameStarted { seed: u64 },
    YearBegan { year: u32 },
    TurnEnded { turn: u32 },
    RegionalShock { region: u32, population_delta: i64 },
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
