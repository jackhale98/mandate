//! Asks and promises (DEC-097 §2; ENGAGEMENT §3): the `asks` domain.
//!
//! People, the party, organisations and the press ask the player for
//! concrete things with deadlines; the player accepts (a promise), declines,
//! or lets an offer lapse. An accepted ask is **kept** when the engine sees
//! the player do what it needs before the deadline (a vote, a statement, a
//! town hall, a canvass, a visit, a talk, a bill, casework), **broken** when
//! the deadline passes, and **void** when it becomes impossible through no
//! fault of the player. Campaign promises are public asks from a
//! constituency's voters.
//!
//! The domain owns [`AskState`] and writes nothing else: kept, broken,
//! declined and void asks go out as effect requests (OVERVIEW §4.3), applied
//! by the characters, population, politics and player domains. Command
//! handlers never write it: they append `EventKind::PlayerDeed` to the event
//! log, and the daily system copies each deed into the deed log
//! ([`AskState::deeds`]) and resolves asks from it. Only the asks' own
//! commands (`RespondAsk`, `Promise`, `Casework`, in [`commands`]) touch it.
//!
//! Tier-generic (DEC-098): askers and scopes are people, parties,
//! constituencies and `Area`s; offers are made for `state.focus()`; a
//! promise belongs to the race for one `Post`.

pub mod commands;
pub mod effects;
pub mod offers;
pub mod resolve;

use crate::characters::CharacterId;
use crate::narrative::CastEntry;
use crate::politics::{PartyId, Post, Vote};
use crate::world::{ConstituencyId, Jurisdiction, NeighborhoodId, PlaceId};
use mandate_content::ContentKey;
use mandate_core::{IdAllocator, SimDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use commands::{PromiseOption, casework_odds, promise_menu};

mandate_core::define_id!(
    /// An ask (or promise); typed `u32`, never reused (DEC-041).
    AskId
);

/// Keyed-draw tags (`Stream::Asks`).
pub(crate) const KEY_OFFER: u64 = 0xA5_01;
pub(crate) const KEY_PICK: u64 = 0xA5_02;
pub(crate) const KEY_DEADLINE: u64 = 0xA5_03;
pub(crate) const KEY_DOORSTEP: u64 = 0xA5_04;
pub(crate) const KEY_CASEWORK: u64 = 0xA5_05;

/// The asks domain's state (`State.asks`; save v24).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AskState {
    /// Asks and promises in the order they were made: the open ones and the
    /// last `history` resolved ones.
    pub asks: Vec<Ask>,
    pub ids: IdAllocator,
    /// Template → the earliest day it may be offered again.
    pub cooldowns: BTreeMap<ContentKey, SimDate>,
    /// Asker → the earliest day they may ask again.
    pub asker_cooldowns: Vec<(Asker, SimDate)>,
    /// Promises made in the current campaign, and the race they belong to
    /// (a new race resets the count).
    pub promises_this_campaign: u8,
    pub promises_race: Option<Post>,
    /// The last offer (for `intent_by_quiet`).
    pub last_offer: Option<SimDate>,
    /// The player's deed log, oldest first (bounded at `deeds_kept`). Read
    /// by conversation lines and `ask.keep_it_quiet`; the town-wide event
    /// log is too short for a look-back of months.
    pub deeds: Vec<DeedRecord>,
    /// Event-log position read up to.
    pub last_event: u32,
    /// Whole-career counts (the history above is bounded).
    #[serde(default)]
    pub tally: AskTally,
}

/// Whole-career counts: offers, answers and outcomes, asks and promises.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AskTally {
    pub offered: u32,
    pub accepted: u32,
    pub declined: u32,
    /// Offers left unanswered (counted in `declined` too).
    pub lapsed: u32,
    pub kept: u32,
    pub broken: u32,
    pub void: u32,
    pub promises_made: u32,
    pub promises_kept: u32,
    pub promises_broken: u32,
    pub promises_void: u32,
}

/// One ask, or one campaign promise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ask {
    pub id: AskId,
    /// Ask or promise template id.
    pub template: ContentKey,
    /// A campaign promise (a `promises.jsonc` template).
    #[serde(default)]
    pub promise: bool,
    pub asker: Asker,
    /// Narrative cast entries: `asker`, `asker_hood`, the template's roles.
    pub cast: Vec<CastEntry>,
    /// Resolved from the template's needs and the cast; met in order.
    pub needs: Vec<Need>,
    /// The first need not yet met.
    pub step: u8,
    /// Neighbourhoods done in the current step (a town hall in each).
    #[serde(default)]
    pub done: Vec<NeighborhoodId>,
    pub public: bool,
    pub big: bool,
    pub opened: SimDate,
    pub respond_by: SimDate,
    pub due: SimDate,
    pub status: AskStatus,
    /// The asker's last earlier ask, for the text (a callback).
    #[serde(default)]
    pub history: History,
    /// A promise's race (the post run for).
    #[serde(default)]
    pub race: Option<Post>,
    /// A promise's chosen answer (a policy template).
    #[serde(default)]
    pub answer: Option<ContentKey>,
    /// Casework attempts (each costs a slot).
    #[serde(default)]
    pub tries: u8,
}

impl Ask {
    pub fn open(&self) -> bool {
        matches!(self.status, AskStatus::Offered | AskStatus::Accepted { .. })
    }

    pub fn offered(&self) -> bool {
        self.status == AskStatus::Offered
    }

    pub fn accepted(&self) -> bool {
        matches!(self.status, AskStatus::Accepted { .. })
    }

    /// The need to meet next, if any.
    pub fn current(&self) -> Option<&Need> {
        self.needs.get(self.step as usize)
    }

    /// When it must be answered (an offer) or kept (accepted).
    pub fn next_date(&self) -> Option<SimDate> {
        match self.status {
            AskStatus::Offered => Some(self.respond_by),
            AskStatus::Accepted { .. } => Some(self.due),
            _ => None,
        }
    }

    /// The asker as a person, if it is one.
    pub fn asker_person(&self) -> Option<CharacterId> {
        crate::narrative::person_of(&self.cast, mandate_content::asks::ROLE_ASKER)
    }
}

/// Who asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Asker {
    Person {
        id: CharacterId,
    },
    Party {
        id: PartyId,
    },
    /// A typed `OutletId` when media ids are typed (M6 §6 item 12).
    Outlet {
        id: u32,
    },
    /// A constituency's voters: a ward, or (M7) a regional district.
    Constituency {
        id: ConstituencyId,
    },
    /// The voters of a whole jurisdiction (an office, or an at-large body).
    Electorate {
        at: Jurisdiction,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AskStatus {
    Offered,
    Accepted {
        on: SimDate,
    },
    Kept {
        on: SimDate,
    },
    Broken {
        on: SimDate,
    },
    /// Declined, or (`unanswered`) left to lapse.
    Declined {
        on: SimDate,
        #[serde(default)]
        unanswered: bool,
    },
    Void {
        on: SimDate,
        /// Fluent key of the reason.
        reason: ContentKey,
    },
}

impl AskStatus {
    /// `offered`, `accepted`, `kept`, `broken`, `declined`, `void`.
    pub fn key(&self) -> &'static str {
        match self {
            AskStatus::Offered => "offered",
            AskStatus::Accepted { .. } => "accepted",
            AskStatus::Kept { .. } => "kept",
            AskStatus::Broken { .. } => "broken",
            AskStatus::Declined { .. } => "declined",
            AskStatus::Void { .. } => "void",
        }
    }

    pub fn resolved_on(&self) -> Option<SimDate> {
        match self {
            AskStatus::Kept { on }
            | AskStatus::Broken { on }
            | AskStatus::Declined { on, .. }
            | AskStatus::Void { on, .. } => Some(*on),
            _ => None,
        }
    }
}

/// The asker's last earlier ask (callbacks, ENGAGEMENT §3.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum History {
    #[default]
    None,
    Kept,
    Broken,
}

impl History {
    pub fn key(self) -> &'static str {
        match self {
            History::None => "none",
            History::Kept => "kept",
            History::Broken => "broken",
        }
    }
}

/// What a statement must be about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StatementNeed {
    /// The live crisis (of this kind).
    Crisis { kind: ContentKey },
    Concern { concern: ContentKey },
    Bill { policy: u32 },
    Any,
}

/// What keeps an ask (ENGAGEMENT §3.6): one of the player's deeds, or a
/// state test.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Need {
    Vote {
        policy: u32,
        vote: Vote,
    },
    /// "No tax rise": broken by a yes vote on, or sponsoring, a bill with
    /// the tag; kept at the deadline.
    VoteAgainstAll {
        tag: ContentKey,
    },
    Statement {
        about: StatementNeed,
    },
    /// A town hall in every one of them.
    TownHall {
        hoods: Vec<NeighborhoodId>,
    },
    Canvass {
        hood: NeighborhoodId,
        #[serde(default)]
        or_town_hall: bool,
    },
    /// Be there before the due date (the meeting).
    Visit {
        place: PlaceId,
    },
    Talk {
        person: CharacterId,
    },
    /// Introduce a bill of one of these templates (a family once F2b lands).
    Introduce {
        templates: Vec<ContentKey>,
    },
    Enacted {
        templates: Vec<ContentKey>,
        sponsor_player: bool,
    },
    Casework,
    /// "Clean politics": none of the player's hidden facts surfaces.
    NoSurfacedHiddenFact,
    /// No leak (or press tip, E9) about this person.
    Quiet {
        person: CharacterId,
    },
}

impl Need {
    /// Kept by not acting: only the deadline keeps it.
    pub fn watch(&self) -> bool {
        matches!(
            self,
            Need::VoteAgainstAll { .. } | Need::NoSurfacedHiddenFact | Need::Quiet { .. }
        )
    }

    /// `vote`, `statement`, … (bots, views).
    pub fn key(&self) -> &'static str {
        match self {
            Need::Vote { .. } => "vote",
            Need::VoteAgainstAll { .. } => "against_all",
            Need::Statement { .. } => "statement",
            Need::TownHall { .. } => "town_hall",
            Need::Canvass { .. } => "canvass",
            Need::Visit { .. } => "visit",
            Need::Talk { .. } => "talk",
            Need::Introduce { .. } => "introduce",
            Need::Enacted { .. } => "enacted",
            Need::Casework => "casework",
            Need::NoSurfacedHiddenFact => "no_surfaced_hidden_fact",
            Need::Quiet { .. } => "quiet",
        }
    }
}

/// What a press statement was about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StatementAbout {
    /// A bill or law.
    Bill { policy: u32 },
    /// No bill: the town's affairs (its crisis, its worries).
    General,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeedRecord {
    pub on: SimDate,
    pub deed: Deed,
}

/// What the player did (`EventKind::PlayerDeed`): appended by command
/// handlers to the event log, copied into the deed log by the asks system.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Deed {
    Vote {
        policy: u32,
        vote: Vote,
    },
    Statement {
        about: StatementAbout,
    },
    TownHall {
        hood: NeighborhoodId,
    },
    Canvass {
        hood: NeighborhoodId,
    },
    Arrive {
        place: PlaceId,
    },
    Talk {
        person: CharacterId,
        topic: ContentKey,
        /// The person the talk was about, if one.
        #[serde(default)]
        about: Option<CharacterId>,
    },
    Introduce {
        policy: u32,
    },
    Casework {
        ask: AskId,
    },
    /// A rival's file slipped to a reporter (the shady `leak_file`).
    Leak {
        politician: u32,
    },
}

/// Record a deed in the event log (command handlers call this; they never
/// write the asks domain).
pub fn record_deed(state: &mut crate::State, deed: Deed) {
    if state.player.is_none() {
        return;
    }
    let date = state.calendar.date;
    state
        .events
        .record(date, crate::events::EventKind::PlayerDeed { deed }, vec![]);
}

/// Outcomes of the campaign promises, for the Legacy record and views.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PromiseOutcomes {
    pub made: u32,
    pub kept: u32,
    pub broken: u32,
    pub void: u32,
    /// Promises still open.
    pub open: u32,
}

impl AskState {
    pub fn get(&self, id: AskId) -> Option<&Ask> {
        self.asks.iter().find(|a| a.id == id)
    }

    pub(crate) fn get_mut(&mut self, id: AskId) -> Option<&mut Ask> {
        self.asks.iter_mut().find(|a| a.id == id)
    }

    /// Offered and accepted asks and open promises.
    pub fn open(&self) -> impl Iterator<Item = &Ask> {
        self.asks.iter().filter(|a| a.open())
    }

    /// Open asks that count against `max_open` (promises do not).
    pub fn open_asks(&self) -> usize {
        self.asks.iter().filter(|a| a.open() && !a.promise).count()
    }

    /// Resolved asks and promises, oldest first.
    pub fn resolved(&self) -> impl Iterator<Item = &Ask> {
        self.asks.iter().filter(|a| !a.open())
    }

    /// Open asks (and promises) that must be answered or kept within `days`
    /// of `date`, soonest first (the time skip's `request_due` interrupt).
    pub fn due_within(&self, date: SimDate, days: u32) -> Vec<&Ask> {
        let until = date.add_days(days);
        let mut v: Vec<&Ask> = self
            .open()
            .filter(|a| a.next_date().is_some_and(|d| d <= until))
            .collect();
        v.sort_by_key(|a| (a.next_date(), a.id));
        v
    }

    /// The career's promise outcomes (the Legacy record: kept and broken).
    pub fn promise_outcomes(&self) -> PromiseOutcomes {
        PromiseOutcomes {
            made: self.tally.promises_made,
            kept: self.tally.promises_kept,
            broken: self.tally.promises_broken,
            void: self.tally.promises_void,
            open: self.asks.iter().filter(|a| a.promise && a.open()).count() as u32,
        }
    }

    /// The player's deeds on or after `from`, oldest first.
    pub fn deeds_since(&self, from: SimDate) -> impl Iterator<Item = &DeedRecord> {
        self.deeds.iter().filter(move |d| d.on >= from)
    }

    /// The asker's last resolved ask (kept or broken), for callbacks.
    pub fn history_of(&self, asker: Asker) -> History {
        self.asks
            .iter()
            .rev()
            .filter(|a| a.asker == asker)
            .find_map(|a| match a.status {
                AskStatus::Kept { .. } => Some(History::Kept),
                AskStatus::Broken { .. } => Some(History::Broken),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// Public promises the player has broken (they discount new ones).
    pub fn broken_public_promises(&self) -> u32 {
        self.tally.promises_broken
    }
}

/// Asks domain, Director phase, daily: the player's new deeds into the deed
/// log; asks kept, broken, declined and void; doorstep and conversation
/// offers. (The weekly draw is [`offers::weekly`].)
pub fn daily(a: &mut AskState, r: &crate::scheduler::Reads<'_>, out: &mut crate::scheduler::Outs<'_>) {
    if r.player.is_none() {
        return;
    }
    let news = resolve::read_events(a, r, out);
    resolve::resolve_all(a, r, out, &news);
    offers::from_deeds(a, r, out, &news.deeds);
    resolve::trim(a, r.content.ask_params.history as usize);
}

/// The days since play began (`min_career_days`; ENGAGEMENT §4.7).
pub fn career_days(r: &crate::scheduler::Reads<'_>) -> u32 {
    let began = r.meta.start_date.add_days(
        r.content.tuning.burn_in_years as u32 * mandate_core::time::DAYS_PER_YEAR,
    );
    began.days_until(r.date)
}
