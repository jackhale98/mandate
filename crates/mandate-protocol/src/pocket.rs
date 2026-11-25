//! What matters now, and in what order (DEC-113; `POCKET_REWORK.md` §3.3,
//! §3.7): the reasons a person or a place matters, what needs the player,
//! and what comes next when nothing does.
//!
//! **The engine orders; the UI only lays out** (rule 4). Who matters is a
//! reading of asks, arcs, decisions, votes and where people stand, so the
//! order, the groups, the counts and every reason are decided here and
//! travel in the views (`PocketView.needs_you`, `PocketView.arcs`,
//! `PocketView.next_up`, `PeopleView`, `SpotView.now` and `SpotView.rank`).
//!
//! **Redaction (DEC-091).** A reason is given only about a matter the
//! player knows: an ask or a decision is aimed at them (never gated); an
//! arc counts from `known` (`crate::arcs::arc_level`); a member's lean is read
//! only where the place gives it (DEC-079). A reason's line names nobody:
//! it sits on the row of the person or place it is about, and what it
//! opens travels as an id and a title in the matter's own words.
//!
//! **Text (DEC-054).** Every line is a Fluent key ([`NowLine`]) with
//! arguments: `now-*` in `locale/en/ui.ftl`, 34 characters at most in
//! English.
//!
//! Views never write: everything here reads the state.

use crate::places::{RouteView, TownHoodView};
use crate::views::DateView;
use mandate_core::SimDate;
use mandate_sim::Engine;
use mandate_sim::arcs::beats::BeatAction;
use mandate_sim::asks::{Asker, Need};
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::narrative::Actor;
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::Level;
use mandate_sim::policy::PolicyStatus;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// A line of text as the engine gives it (DEC-054): a Fluent key and its
/// arguments (names and facts; an argument named `…_key` is a Fluent key).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TextView {
    pub key: String,
    pub args: BTreeMap<String, String>,
}

impl TextView {
    pub fn new(key: impl Into<String>) -> Self {
        TextView {
            key: key.into(),
            args: BTreeMap::new(),
        }
    }

    pub fn with(mut self, name: &str, value: impl Into<String>) -> Self {
        self.args.insert(name.into(), value.into());
        self
    }

    pub fn of(key: impl Into<String>, args: BTreeMap<String, String>) -> Self {
        TextView {
            key: key.into(),
            args,
        }
    }
}

/// Why a person or a place matters now (`POCKET_REWORK.md` §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum NowKind {
    /// A pending decision names them; a place: its scene is there.
    Decision,
    /// An ask or promise, offered or accepted and not settled; a place:
    /// its next need is done there.
    Ask,
    /// A beat of a running arc the player knows; a place: it is done there.
    Arc,
    /// A vote on the player's bill falls within the turn.
    Vote,
    /// The player's rival, during a campaign.
    Rival,
    /// Where the player is.
    Here,
}

/// Every `now-*` line the engine names (`locale/en/ui.ftl`): a reason, a
/// clock, or what comes next. The Fluent coverage test holds the two lists
/// equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NowLine {
    AskOffer,
    AskPromised,
    AskTalk,
    AskNamed,
    ArcTalk,
    Decision,
    VoteUndecided,
    VoteMember,
    Here,
    Rival,
    PlaceScene,
    PlaceAsk,
    PlaceBeat,
    PlaceHere,
    ClockToday,
    ClockTomorrow,
    ClockDays,
    NextVote,
    NextAsk,
    NextArc,
    NextElection,
}

impl NowLine {
    pub const ALL: [NowLine; 21] = [
        NowLine::AskOffer,
        NowLine::AskPromised,
        NowLine::AskTalk,
        NowLine::AskNamed,
        NowLine::ArcTalk,
        NowLine::Decision,
        NowLine::VoteUndecided,
        NowLine::VoteMember,
        NowLine::Here,
        NowLine::Rival,
        NowLine::PlaceScene,
        NowLine::PlaceAsk,
        NowLine::PlaceBeat,
        NowLine::PlaceHere,
        NowLine::ClockToday,
        NowLine::ClockTomorrow,
        NowLine::ClockDays,
        NowLine::NextVote,
        NowLine::NextAsk,
        NowLine::NextArc,
        NowLine::NextElection,
    ];

    /// The Fluent key.
    pub fn key(self) -> &'static str {
        match self {
            NowLine::AskOffer => "now-ask-offer",
            NowLine::AskPromised => "now-ask-promised",
            NowLine::AskTalk => "now-ask-talk",
            NowLine::AskNamed => "now-ask-named",
            NowLine::ArcTalk => "now-arc-talk",
            NowLine::Decision => "now-decision",
            NowLine::VoteUndecided => "now-vote-undecided",
            NowLine::VoteMember => "now-vote-member",
            NowLine::Here => "now-here",
            NowLine::Rival => "now-rival",
            NowLine::PlaceScene => "now-place-scene",
            NowLine::PlaceAsk => "now-place-ask",
            NowLine::PlaceBeat => "now-place-beat",
            NowLine::PlaceHere => "now-place-here",
            NowLine::ClockToday => "now-clock-today",
            NowLine::ClockTomorrow => "now-clock-tomorrow",
            NowLine::ClockDays => "now-clock-days",
            NowLine::NextVote => "now-next-vote",
            NowLine::NextAsk => "now-next-ask",
            NowLine::NextArc => "now-next-arc",
            NowLine::NextElection => "now-next-election",
        }
    }
}

/// One reason a person or a place matters now: a short line, what it
/// opens, and its deadline if it has one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NowReasonView {
    pub kind: NowKind,
    /// Fluent key (`now-…`): 34 characters at most in English. It names
    /// nobody: it is read on the row of the person or place it is about.
    pub key: String,
    pub args: BTreeMap<String, String>,
    /// What it is about, in the matter's own words (an ask's title, an
    /// arc's, a decision's, a bill's name): for the opened row.
    pub title: Option<TextView>,
    /// What the reason opens: at most one of `ask`, `decision` and `bill`,
    /// and the arc run it belongs to (`QueryDto::Arc`), if any.
    pub ask: Option<u32>,
    pub arc: Option<u32>,
    pub decision: Option<u32>,
    pub bill: Option<u32>,
    pub due: Option<DateView>,
    pub days_left: Option<u32>,
}

impl NowReasonView {
    fn new(kind: NowKind, line: NowLine, today: SimDate, due: Option<SimDate>) -> Self {
        NowReasonView {
            kind,
            key: line.key().into(),
            args: BTreeMap::new(),
            title: None,
            ask: None,
            arc: None,
            decision: None,
            bill: None,
            due: due.map(DateView::of),
            days_left: due.map(|d| today.days_until(d)),
        }
    }

    /// Strongest first: the soonest deadline, then by kind.
    pub(crate) fn weight(&self) -> (u32, NowKind) {
        (
            self.due.as_ref().map_or(u32::MAX, |d| d.day_number),
            self.kind,
        )
    }
}

/// Strongest first, and one reason a matter: a person named twice by one
/// ask keeps the stronger line (the earlier in [`NowLine::ALL`]).
fn tidy(mut v: Vec<NowReasonView>) -> Vec<NowReasonView> {
    let line = |r: &NowReasonView| NowLine::ALL.iter().position(|l| l.key() == r.key);
    v.sort_by(|a, b| a.weight().cmp(&b.weight()).then(line(a).cmp(&line(b))));
    let mut out: Vec<NowReasonView> = Vec::new();
    for r in v {
        let same = |x: &NowReasonView| {
            x.kind == r.kind
                && x.ask == r.ask
                && x.arc == r.arc
                && x.decision == r.decision
                && x.bill == r.bill
        };
        if !out.iter().any(same) {
            out.push(r);
        }
    }
    out
}

/// A row's place among those that matter now (smallest first): its soonest
/// deadline's day, whether it is away from the player, its strongest reason.
pub(crate) type OrderKey = (u32, bool, NowKind);

/// The order of a row among those that matter now: the soonest deadline
/// first, then who (or what) is where the player is, then by reason.
pub(crate) fn order_key(now: &[NowReasonView], here: bool) -> OrderKey {
    let due = now.iter().map(|r| r.weight().0).min().unwrap_or(u32::MAX);
    let kind = now.iter().map(|r| r.kind).min().unwrap_or(NowKind::Here);
    (due, !here, kind)
}

/// The clock of something under way, in a sentence (`now-clock-*`).
pub(crate) fn clock_text(today: SimDate, due: SimDate) -> TextView {
    match today.days_until(due) {
        0 => TextView::new(NowLine::ClockToday.key()),
        1 => TextView::new(NowLine::ClockTomorrow.key()),
        n => TextView::new(NowLine::ClockDays.key()).with("days", n.to_string()),
    }
}

/// What matters now, read once for a view: the reasons by person and by
/// place. Being where the player is (`here`) is added by each view from
/// what it already knows of where people are.
#[derive(Default)]
pub(crate) struct Matters {
    people: BTreeMap<CharacterId, Vec<NowReasonView>>,
    places: Vec<(Location, NowReasonView)>,
}

impl Matters {
    pub(crate) fn of(engine: &Engine) -> Matters {
        let st = engine.state();
        let content = engine.content();
        let mut m = Matters::default();
        let Some(pl) = st.player.as_ref() else {
            return m;
        };
        let me = characters::player_character(st);
        let today = st.calendar.date;
        let person = |m: &mut Matters, who: CharacterId, r: NowReasonView| {
            if Some(who) != me {
                m.people.entry(who).or_default().push(r);
            }
        };

        // Asks and promises, offered or accepted and not settled.
        for a in st.asks.open() {
            let v = crate::asks::ask_view(engine, a);
            let due = a.next_date();
            let make = |kind: NowKind, line: NowLine, due: Option<SimDate>| {
                let mut r = NowReasonView::new(kind, line, today, due);
                r.title = Some(TextView::of(v.title_key.clone(), v.args.clone()));
                r.ask = Some(a.id.0);
                r.arc = a.arc.as_ref().map(|x| x.run.0);
                r
            };
            let asker = match a.asker {
                Asker::Person { id } => Some(id),
                Asker::Party { .. } => a.asker_person(),
                _ => None,
            };
            if let Some(p) = asker {
                let line = if a.offered() {
                    NowLine::AskOffer
                } else {
                    NowLine::AskPromised
                };
                person(&mut m, p, make(NowKind::Ask, line, due));
            }
            for n in a.needs.iter().skip(a.step as usize) {
                match n {
                    Need::Talk { person: p } if Some(*p) != asker => {
                        person(&mut m, *p, make(NowKind::Ask, NowLine::AskTalk, due));
                    }
                    Need::Quiet { person: p } if Some(*p) != asker => {
                        person(&mut m, *p, make(NowKind::Ask, NowLine::AskNamed, due));
                    }
                    _ => {}
                }
            }
            // Its own cast. (An arc's ask carries the whole cast of its
            // run: there, only its needs name anyone.)
            for c in a.cast.iter().filter(|_| a.arc.is_none()) {
                if let Actor::Person { id } = c.actor
                    && Some(id) != asker
                {
                    person(&mut m, id, make(NowKind::Ask, NowLine::AskNamed, due));
                }
            }
            // Where its next need is done, once accepted (an offer is
            // answered from anywhere).
            if a.accepted()
                && let Some(how) = v.how.as_ref().filter(|h| h.action != "wait")
                && let Some(to) = how.to.as_ref()
            {
                m.places.push((
                    to.location.to_sim(),
                    make(NowKind::Ask, NowLine::PlaceAsk, due),
                ));
            }
        }

        // Pending decisions: their people, and where the scene is.
        for d in st.narrative.pending() {
            let title = content.dilemma(&d.template).map(|t| {
                TextView::of(
                    t.title_key.clone(),
                    crate::narrative::cast_args(engine, &d.cast),
                )
            });
            let make = |line: NowLine| {
                let mut r = NowReasonView::new(NowKind::Decision, line, today, Some(d.deadline));
                r.title = title.clone();
                r.decision = Some(d.id.0);
                r.arc = d.arc.as_ref().map(|x| x.run.0);
                r
            };
            for c in &d.cast {
                if let Actor::Person { id } = c.actor {
                    person(&mut m, id, make(NowLine::Decision));
                }
            }
            if let Some(loc) = d.place {
                m.places.push((loc, make(NowLine::PlaceScene)));
            }
        }

        // Beats of running arcs the player knows: a word with someone, a
        // thing to do somewhere. (A run's decisions and asks are above.)
        for o in mandate_sim::arcs::beats::offers(st, content) {
            let Some(run) = st.arcs.run(o.run) else {
                continue;
            };
            let Some(def) = content.arc(&run.template) else {
                continue;
            };
            if crate::arcs::arc_level(engine, run, def) < Level::Known {
                continue;
            }
            let make = |line: NowLine| {
                let mut r = NowReasonView::new(NowKind::Arc, line, today, run.clock);
                r.title = Some(crate::arcs::title(engine, run, def));
                r.arc = Some(run.id.0);
                r
            };
            match o.action {
                BeatAction::Talk { person: p, .. } => {
                    person(&mut m, p, make(NowLine::ArcTalk));
                }
                // A scene (DEC-112) is begun at its place, as a visit is.
                BeatAction::Visit { location, .. } | BeatAction::Exchange { location, .. } => {
                    m.places.push((location, make(NowLine::PlaceBeat)));
                }
                BeatAction::Decision { .. } | BeatAction::Ask { .. } => {}
            }
        }

        // A vote on the player's own bill within the turn: where the place
        // gives the count member by member (DEC-079), those who are
        // undecided; elsewhere, whoever has a vote on it.
        let focus = pl.focus();
        let until = today.add_days(engine.turn_days());
        let constitution = content.constitutions.get(&st.meta.constitution);
        for p in st.policy.instances.iter().filter(|p| {
            p.sponsor == Some(pl.politician)
                && p.status == PolicyStatus::UnderConsideration
                && focus.contains(&p.at())
        }) {
            let Some(b) = crate::office::bill_view(engine, p) else {
                continue;
            };
            let Some(day) = b.next_vote.as_ref().map(|d| SimDate(d.day_number)) else {
                continue;
            };
            if day > until {
                continue;
            }
            let make = |line: NowLine| {
                let mut r = NowReasonView::new(NowKind::Vote, line, today, Some(day));
                r.title = Some(TextView::new(b.name_key.clone()));
                r.bill = Some(b.id);
                r
            };
            let voters: Vec<(mandate_sim::politics::PoliticianId, NowLine)> = if b.whip_exact {
                b.whip
                    .iter()
                    .filter(|w| !w.is_player && w.lean == "undecided")
                    .map(|w| {
                        (
                            mandate_sim::politics::PoliticianId(w.politician),
                            NowLine::VoteUndecided,
                        )
                    })
                    .collect()
            } else {
                constitution
                    .map(|c| mandate_sim::politics::legislature_of(&st.politics, c, p.at()))
                    .unwrap_or(&[])
                    .iter()
                    .map(|id| (*id, NowLine::VoteMember))
                    .collect()
            };
            for (id, line) in voters {
                if let Some(pol) = st.politics.get(id) {
                    person(&mut m, pol.character, make(line));
                }
            }
        }

        // The rival, while the player is in a race (the rival's identity
        // is never gated, DEC-091).
        let race = st
            .politics
            .elections
            .races
            .iter()
            .find(|r| r.candidates.iter().any(|c| c.politician == pl.politician));
        if let Some(race) = race
            && let Some(rival) = st
                .politics
                .characters
                .towns
                .get(pl.home().index())
                .and_then(|t| t.rival)
        {
            let r = NowReasonView::new(
                NowKind::Rival,
                NowLine::Rival,
                today,
                Some(race.election_day),
            );
            person(&mut m, rival, r);
        }

        m.places.push((
            pl.location,
            NowReasonView::new(NowKind::Here, NowLine::PlaceHere, today, None),
        ));
        m
    }

    /// Why a person matters now, strongest first; with `here`, that they
    /// are where the player is and can be spoken to.
    pub(crate) fn person(
        &self,
        engine: &Engine,
        id: CharacterId,
        here: bool,
    ) -> Vec<NowReasonView> {
        let mut v = self.people.get(&id).cloned().unwrap_or_default();
        if here {
            v.push(NowReasonView::new(
                NowKind::Here,
                NowLine::Here,
                engine.state().calendar.date,
                None,
            ));
        }
        tidy(v)
    }

    /// Everyone a reason names (the player aside): people aimed at the
    /// player are named by what is aimed at them, whatever the ledger says.
    pub(crate) fn people(&self) -> impl Iterator<Item = CharacterId> + '_ {
        self.people.keys().copied()
    }

    /// Whether any reason of a kind names this person.
    pub(crate) fn has(&self, id: CharacterId, kind: NowKind) -> bool {
        self.people
            .get(&id)
            .is_some_and(|v| v.iter().any(|r| r.kind == kind))
    }

    /// Why a place matters now, strongest first.
    pub(crate) fn place(&self, loc: Location) -> Vec<NowReasonView> {
        tidy(
            self.places
                .iter()
                .filter(|(l, _)| *l == loc)
                .map(|(_, r)| r.clone())
                .collect(),
        )
    }
}

/// Fill a town's places with why each matters now, its order in the town
/// (`rank`: 0 first; the UI sorts by nothing else) and its note, and each
/// neighbourhood with its note (`crate::notebook`).
pub(crate) fn rank_spots(engine: &Engine, hoods: &mut [TownHoodView]) {
    let notes = crate::notebook::Notes::of(engine);
    // Places that matter first; the rest keep the town's own order.
    let mut order: Vec<(bool, OrderKey, usize, usize)> = Vec::new();
    for (hi, h) in hoods.iter_mut().enumerate() {
        h.note = notes.hood(mandate_sim::world::NeighborhoodId(h.id));
        for (si, s) in h.spots.iter_mut().enumerate() {
            let loc = s.location.location.to_sim();
            s.now = notes.matters.place(loc);
            s.note = notes.place(loc);
            order.push((s.now.is_empty(), order_key(&s.now, s.here), hi, si));
        }
    }
    order.sort();
    for (rank, (_, _, hi, si)) in order.into_iter().enumerate() {
        hoods[hi].spots[si].rank = rank as u32;
    }
}

// --- Needs you ---------------------------------------------------------------------------

/// What kind of thing waits on the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum NeedKind {
    /// A pending decision (`CommandDto::AnswerDecision`).
    Decision,
    /// An ask offered and not answered (`CommandDto::RespondAsk`).
    AskOffer,
    /// A vote to cast (`CommandDto::CastVote`).
    Vote,
    /// An accepted ask or promise with something left to do, due within
    /// the turn.
    AskDue,
    /// A beat of a running arc that is the player's to take, in its last
    /// turn before the clock runs out (`ArcView.beats`).
    Beat,
}

/// What a need opens.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NeedTargetDto {
    Decision {
        id: u32,
    },
    Ask {
        id: u32,
    },
    Bill {
        id: u32,
    },
    /// A beat of an arc run (`QueryDto::Arc`; `ArcBeatView.stage`, `.beat`).
    Beat {
        arc: u32,
        stage: String,
        beat: String,
    },
}

/// Something that waits on the player (Today's "Needs you").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NeedView {
    pub kind: NeedKind,
    /// Its title in the matter's own words (a dilemma's, an ask's, a
    /// bill's name, a beat's button), with `args`.
    pub title_key: String,
    pub args: BTreeMap<String, String>,
    pub due: Option<DateView>,
    pub days_left: Option<u32>,
    pub target: NeedTargetDto,
    /// The arc run it belongs to (`QueryDto::Arc`), if any.
    pub arc: Option<u32>,
}

/// What waits on the player, soonest first: decisions, asks to answer,
/// votes to cast, accepted asks due within the turn, and the beats of a
/// known arc in their last turn. Its length is the bar's count.
pub fn needs_you(engine: &Engine) -> Vec<NeedView> {
    let st = engine.state();
    let content = engine.content();
    let Some(pl) = st.player.as_ref() else {
        return Vec::new();
    };
    let today = st.calendar.date;
    let until = today.add_days(engine.turn_days());
    let mut out: Vec<(u32, NeedKind, u32, NeedView)> = Vec::new();
    let mut push = |kind: NeedKind,
                    id: u32,
                    title: TextView,
                    due: Option<SimDate>,
                    target: NeedTargetDto,
                    arc: Option<u32>| {
        out.push((
            due.map_or(u32::MAX, |d| d.0),
            kind,
            id,
            NeedView {
                kind,
                title_key: title.key,
                args: title.args,
                due: due.map(DateView::of),
                days_left: due.map(|d| today.days_until(d)),
                target,
                arc,
            },
        ));
    };
    for d in crate::narrative::pending_decisions(engine) {
        let arc = st
            .narrative
            .decision(mandate_sim::narrative::DecisionId(d.id))
            .and_then(|x| x.arc.as_ref())
            .map(|a| a.run.0);
        push(
            NeedKind::Decision,
            d.id,
            TextView::of(d.title_key, d.args),
            Some(SimDate(d.deadline.day_number)),
            NeedTargetDto::Decision { id: d.id },
            arc,
        );
    }
    for a in st.asks.open() {
        let kind = if a.offered() {
            NeedKind::AskOffer
        } else if a.due <= until && a.current().is_some_and(|n| !n.watch()) {
            NeedKind::AskDue
        } else {
            continue;
        };
        let v = crate::asks::ask_view(engine, a);
        push(
            kind,
            a.id.0,
            TextView::of(v.title_key, v.args),
            a.next_date(),
            NeedTargetDto::Ask { id: a.id.0 },
            a.arc.as_ref().map(|x| x.run.0),
        );
    }
    let focus = pl.focus();
    for p in st.policy.instances.iter().filter(|p| {
        p.status == PolicyStatus::UnderConsideration
            && p.sponsor != Some(pl.politician)
            && focus.contains(&p.at())
    }) {
        let Some(b) = crate::office::bill_view(engine, p).filter(|b| b.can_vote) else {
            continue;
        };
        push(
            NeedKind::Vote,
            b.id,
            TextView::new(b.name_key),
            b.next_vote.map(|d| SimDate(d.day_number)),
            NeedTargetDto::Bill { id: b.id },
            None,
        );
    }
    for o in mandate_sim::arcs::beats::offers(st, content) {
        if !matches!(o.action, BeatAction::Visit { .. } | BeatAction::Talk { .. }) {
            continue;
        }
        let Some(run) = st.arcs.run(o.run) else {
            continue;
        };
        let Some(def) = content.arc(&run.template) else {
            continue;
        };
        let Some(clock) = run.clock.filter(|c| *c <= until) else {
            continue;
        };
        if crate::arcs::arc_level(engine, run, def) < Level::Known {
            continue;
        }
        let Some(label) = crate::arcs::beat_label(engine, run, def, &o.beat) else {
            continue;
        };
        push(
            NeedKind::Beat,
            run.id.0,
            label,
            Some(clock),
            NeedTargetDto::Beat {
                arc: run.id.0,
                stage: o.beat.stage.clone(),
                beat: o.beat.beat.clone(),
            },
            Some(run.id.0),
        );
    }
    out.sort_by_key(|a| (a.0, a.1, a.2));
    out.into_iter().map(|x| x.3).collect()
}

// --- What comes next (the empty state) -----------------------------------------------------

/// What kind of dated thing comes next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum NextKind {
    /// A vote on a bill the player sponsors or has voted on.
    Vote,
    /// An accepted ask or promise falls due.
    Ask,
    /// A running arc the player knows moves on.
    Arc,
    /// Election day.
    Election,
}

/// The next dated thing ahead (Today's empty state, §3.7): "Nothing is
/// waiting on you. Next: …". The engine names it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NextUpView {
    pub kind: NextKind,
    /// Fluent key (`now-next-…`).
    pub key: String,
    pub args: BTreeMap<String, String>,
    /// What it is, in its own words (a bill's name, an ask's title, an
    /// arc's), where it has one.
    pub title: Option<TextView>,
    pub date: DateView,
    pub days_left: u32,
    pub ask: Option<u32>,
    pub arc: Option<u32>,
    pub bill: Option<u32>,
}

/// The soonest dated thing ahead of the player that is not already among
/// `needs_you`: a vote on their bill, a promise falling due, a known arc's
/// clock, election day.
pub fn next_up(engine: &Engine) -> Option<NextUpView> {
    let st = engine.state();
    let content = engine.content();
    let pl = st.player.as_ref()?;
    let today = st.calendar.date;
    let mut out: Vec<(u32, NextKind, u32, NextUpView)> = Vec::new();
    let mut push =
        |kind: NextKind, line: NowLine, id: u32, on: SimDate, title: Option<TextView>| {
            if on < today {
                return;
            }
            out.push((
                on.0,
                kind,
                id,
                NextUpView {
                    kind,
                    key: line.key().into(),
                    args: BTreeMap::new(),
                    title,
                    date: DateView::of(on),
                    days_left: today.days_until(on),
                    ask: (kind == NextKind::Ask).then_some(id),
                    arc: (kind == NextKind::Arc).then_some(id),
                    bill: (kind == NextKind::Vote).then_some(id),
                },
            ));
        };
    let focus = pl.focus();
    for p in st
        .policy
        .instances
        .iter()
        .filter(|p| p.status == PolicyStatus::UnderConsideration && focus.contains(&p.at()))
    {
        let Some(b) = crate::office::bill_view(engine, p) else {
            continue;
        };
        // The player's business: their own bill, or one they have voted on.
        // (One still awaiting their vote is a need.)
        if !(b.sponsor_is_player || b.my_vote.is_some()) {
            continue;
        }
        if let Some(d) = b.next_vote {
            push(
                NextKind::Vote,
                NowLine::NextVote,
                b.id,
                SimDate(d.day_number),
                Some(TextView::new(b.name_key)),
            );
        }
    }
    // An accepted ask's due date (one with something to do that falls due
    // within the turn is a need already).
    let until = today.add_days(engine.turn_days());
    for a in st
        .asks
        .open()
        .filter(|a| a.accepted() && !(a.due <= until && a.current().is_some_and(|n| !n.watch())))
    {
        let v = crate::asks::ask_view(engine, a);
        push(
            NextKind::Ask,
            NowLine::NextAsk,
            a.id.0,
            a.due,
            Some(TextView::of(v.title_key, v.args)),
        );
    }
    for run in &st.arcs.runs {
        let (Some(def), Some(clock)) = (content.arc(&run.template), run.clock) else {
            continue;
        };
        if crate::arcs::arc_level(engine, run, def) >= Level::Known {
            push(
                NextKind::Arc,
                NowLine::NextArc,
                run.id.0,
                clock,
                Some(crate::arcs::title(engine, run, def)),
            );
        }
    }
    if let Some(day) = st
        .politics
        .elections
        .races
        .iter()
        .map(|r| r.election_day)
        .min()
    {
        push(NextKind::Election, NowLine::NextElection, 0, day, None);
    }
    out.sort_by_key(|a| (a.0, a.1, a.2));
    out.into_iter().next().map(|x| x.3)
}

/// A route to a location: `Go` there (`action`: what is done on arrival).
pub(crate) fn route_to(engine: &Engine, action: &str, loc: Location) -> Option<RouteView> {
    let st = engine.state();
    let pl = st.player.as_ref()?;
    mandate_sim::places::validate_move(st, loc).ok()?;
    Some(RouteView {
        action: action.into(),
        late_night: false,
        neighborhood: mandate_sim::places::hood_of(st, loc).map(|h| h.0),
        to: Some(crate::places::location_view(st, loc)),
        here: pl.location == loc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notebook::KnowLine;
    use std::collections::BTreeSet;

    const UI: &str = include_str!("../../../content/base/locale/en/ui.ftl");

    /// Every message of `ui.ftl` whose key has the prefix, with its text.
    fn lines(prefix: &str) -> Vec<(&'static str, &'static str)> {
        UI.lines()
            .filter_map(|l| l.split_once(" = "))
            .filter(|(k, _)| k.starts_with(prefix) && !k.contains(' '))
            .collect()
    }

    fn keys(prefix: &str) -> BTreeSet<&'static str> {
        lines(prefix).into_iter().map(|(k, _)| k).collect()
    }

    #[test]
    fn every_reason_has_a_line_and_every_line_a_reason() {
        let named: BTreeSet<&str> = NowLine::ALL.iter().map(|l| l.key()).collect();
        assert_eq!(named.len(), NowLine::ALL.len(), "a key is named twice");
        assert_eq!(
            named,
            keys("now-"),
            "`now-*`: the engine's lines and ui.ftl's"
        );
    }

    #[test]
    fn every_note_has_a_line_and_every_line_a_note() {
        let named: BTreeSet<&str> = KnowLine::ALL.iter().map(|l| l.key()).collect();
        assert_eq!(named.len(), KnowLine::ALL.len(), "a key is named twice");
        assert_eq!(
            named,
            keys("know-"),
            "`know-*`: the engine's lines and ui.ftl's"
        );
    }

    /// A reason sits on the third line of a row at 393 px: 34 characters at
    /// most in English, with its number at its longest.
    #[test]
    fn a_reason_is_34_characters_or_fewer() {
        for (key, text) in lines("now-") {
            let shown = text.replace("{ $days }", "365");
            assert!(
                !shown.contains('{'),
                "{key}: a reason takes no argument but `$days` (it names nobody): {text}"
            );
            assert!(
                shown.chars().count() <= 34,
                "{key} is {} characters: {shown}",
                shown.chars().count()
            );
        }
    }

    /// A note is a whole sentence (or two): a capital, a full stop.
    #[test]
    fn a_note_is_a_whole_sentence() {
        for (key, text) in lines("know-") {
            assert!(
                text.chars().next().is_some_and(char::is_uppercase) && text.ends_with('.'),
                "{key} is not a sentence: {text}"
            );
            assert!(
                text.split_whitespace().count() >= 4,
                "{key} is a fragment: {text}"
            );
        }
    }

    #[test]
    fn the_clock_reads_by_days() {
        let today = SimDate(100);
        assert_eq!(clock_text(today, SimDate(100)).key, "now-clock-today");
        assert_eq!(clock_text(today, SimDate(101)).key, "now-clock-tomorrow");
        let t = clock_text(today, SimDate(109));
        assert_eq!(t.key, "now-clock-days");
        assert_eq!(t.args.get("days").map(String::as_str), Some("9"));
    }

    /// The soonest deadline first, then who is here, then by reason; one
    /// reason a matter.
    #[test]
    fn reasons_are_ordered_and_one_a_matter() {
        let today = SimDate(10);
        let due = Some(SimDate(30));
        let mut late = NowReasonView::new(NowKind::Ask, NowLine::AskOffer, today, due);
        late.ask = Some(1);
        let mut again = NowReasonView::new(NowKind::Ask, NowLine::AskNamed, today, due);
        again.ask = Some(1);
        let mut soon = NowReasonView::new(
            NowKind::Decision,
            NowLine::Decision,
            today,
            Some(SimDate(12)),
        );
        soon.decision = Some(4);
        let here = NowReasonView::new(NowKind::Here, NowLine::Here, today, None);
        let v = tidy(vec![here.clone(), again, late.clone(), soon.clone()]);
        assert_eq!(v, vec![soon.clone(), late.clone(), here.clone()]);
        assert_eq!(soon.days_left, Some(2));
        // A deadline beats being here; being here beats a reason without one.
        assert!(order_key(&[late], false) < order_key(std::slice::from_ref(&here), true));
        let rival = NowReasonView::new(NowKind::Rival, NowLine::Rival, today, None);
        assert!(order_key(&[here], true) < order_key(&[rival], false));
    }
}
