//! Knowledge in words (DEC-113 §4; DEC-091's principle of 2026-11-25;
//! `POCKET_REWORK.md` §3.4): the `note` on a person row, a neighbourhood, a
//! place, a bill and a notebook entry.
//!
//! **Knowledge is not a checklist.** The level is never drawn and nothing
//! counts what is still to learn. A note is one sentence, and it is there
//! only where a gap or a fact changes something the player could do now:
//!
//! | Line | Said of | When |
//! |---|---|---|
//! | `know-person-lobby` | a person | a vote on the player's bill names them, and the player knows them too little for full lobbying odds (`knowledge.effects.lobby_logit`) |
//! | `know-person-name-only` | a person | they matter now, and the player knows no more than the name |
//! | `know-hood-worries` | a neighbourhood | a canvass there is in hand (an ask needs it, the player is there, their race is fought there) and its worries are unknown, so it cannot be aimed (`actions::canvass_match`) |
//! | `know-bill-support` | a bill | it is the player's to move or vote on, and support for it is known only as a band, so their voters' side is a rough estimate |
//! | `know-figures-stale` | a neighbourhood, a bill, a notebook entry | the reading shown is older than `knowledge.fresh_days` |
//! | `know-place-shut` | a place | it has shut: nothing can be done there |
//! | `know-arc-lead` | an arc | the player has only heard of it (`crate::arcs`) |
//!
//! Whether a gap changes something is the engine's to say: it knows which
//! actions are open and which of them turn on the subject. Elsewhere the
//! note is absent, and the facts the level reveals are the display.
//!
//! **Text (DEC-054).** Each line is a Fluent key ([`KnowLine`]) with
//! arguments: `know-*` in `locale/en/ui.ftl`, each a whole sentence. No
//! note names a person or a matter the player does not know.
//!
//! Views never write: everything here reads the state.

use crate::knowledge::Gate;
use crate::pocket::{Matters, NowKind, NowReasonView, TextView};
use mandate_sim::Engine;
use mandate_sim::asks::Need;
use mandate_sim::characters::CharacterId;
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Entry, Level, PolicyRef, Reading, Subject};
use mandate_sim::policy::PolicyStatus;
use mandate_sim::world::{MunicipalityId, NeighborhoodId};
use std::cell::OnceCell;

/// Every `know-*` line the engine names (`locale/en/ui.ftl`). The Fluent
/// coverage test holds the two lists equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowLine {
    PersonNameOnly,
    PersonLobby,
    HoodWorries,
    BillSupport,
    FiguresStale,
    PlaceShut,
    ArcLead,
}

impl KnowLine {
    pub const ALL: [KnowLine; 7] = [
        KnowLine::PersonNameOnly,
        KnowLine::PersonLobby,
        KnowLine::HoodWorries,
        KnowLine::BillSupport,
        KnowLine::FiguresStale,
        KnowLine::PlaceShut,
        KnowLine::ArcLead,
    ];

    /// The Fluent key.
    pub fn key(self) -> &'static str {
        match self {
            KnowLine::PersonNameOnly => "know-person-name-only",
            KnowLine::PersonLobby => "know-person-lobby",
            KnowLine::HoodWorries => "know-hood-worries",
            KnowLine::BillSupport => "know-bill-support",
            KnowLine::FiguresStale => "know-figures-stale",
            KnowLine::PlaceShut => "know-place-shut",
            KnowLine::ArcLead => "know-arc-lead",
        }
    }
}

/// "These figures are from March 2038. They may have moved."
fn stale_note(r: &Reading) -> TextView {
    let c = r.on.calendar();
    TextView::new(KnowLine::FiguresStale.key())
        .with("month_key", format!("month-{}", c.month))
        .with("year", c.year.to_string())
}

/// A bill's note: for one that is the player's to move or vote on
/// (`in_hand`), that support for it is known only as a band, or that the
/// reading is old.
pub(crate) fn bill_note(
    gate: &Gate<'_>,
    town: MunicipalityId,
    template: &str,
    in_hand: bool,
) -> Option<TextView> {
    if !in_hand {
        return None;
    }
    let subject = Subject::PolicySupport {
        town,
        policy: PolicyRef::Template {
            template: template.into(),
        },
    };
    if gate.level(&subject) < Level::Known {
        return Some(TextView::new(KnowLine::BillSupport.key()));
    }
    gate.reading(&subject)
        .filter(|r| gate.stale(r))
        .map(stale_note)
}

/// The notes of a view, with what matters now read once.
pub(crate) struct Notes<'a> {
    engine: &'a Engine,
    gate: Gate<'a>,
    pub(crate) matters: Matters,
    /// Neighbourhoods where a canvass is in hand.
    canvass: Vec<NeighborhoodId>,
    /// Templates of the bills that are the player's to move or vote on.
    bills: OnceCell<Vec<String>>,
}

impl<'a> Notes<'a> {
    pub(crate) fn of(engine: &'a Engine) -> Self {
        let st = engine.state();
        let mut canvass: Vec<NeighborhoodId> = Vec::new();
        if let Some(pl) = st.player.as_ref() {
            // Where the player stands.
            canvass.extend(mandate_sim::places::hood_of(st, pl.location));
            // What an open ask still needs.
            for a in st.asks.open() {
                for n in a.needs.iter().skip(a.step as usize) {
                    match n {
                        Need::Canvass { hood, .. } => canvass.push(*hood),
                        Need::TownHall { hoods } => {
                            canvass.extend(hoods.iter().filter(|h| !a.done.contains(h)));
                        }
                        _ => {}
                    }
                }
            }
            // Where the player's race is fought.
            for r in st
                .politics
                .elections
                .races
                .iter()
                .filter(|r| r.candidates.iter().any(|c| c.politician == pl.politician))
            {
                canvass.extend(r.hoods.iter().map(|h| NeighborhoodId(*h)));
            }
        }
        canvass.sort();
        canvass.dedup();
        Notes {
            engine,
            gate: Gate::of(engine),
            matters: Matters::of(engine),
            canvass,
            bills: OnceCell::new(),
        }
    }

    /// A person's note, given why they matter now.
    pub(crate) fn person(&self, id: CharacterId, now: &[NowReasonView]) -> Option<TextView> {
        if now.is_empty() {
            return None;
        }
        let level = self.gate.person(id);
        let k = &self.engine.content().knowledge.effects.lobby_logit;
        if now.iter().any(|r| r.kind == NowKind::Vote)
            && level < Level::Known
            && k.get(level) < k.get(Level::Known)
        {
            return Some(TextView::new(KnowLine::PersonLobby.key()));
        }
        (level <= Level::Heard).then(|| TextView::new(KnowLine::PersonNameOnly.key()))
    }

    /// A neighbourhood's note.
    pub(crate) fn hood(&self, hood: NeighborhoodId) -> Option<TextView> {
        if !self.canvass.contains(&hood) {
            return None;
        }
        let st = self.engine.state();
        let subject = Subject::HoodConcerns { hood };
        if self.gate.level(&subject) == Level::Unknown {
            let mut t = TextView::new(KnowLine::HoodWorries.key()).with(
                "hood",
                st.world
                    .neighborhoods
                    .get(hood.index())
                    .map(|h| h.name.clone())
                    .unwrap_or_default(),
            );
            if let Some((key, extra)) = crate::talk::hood_centre_label(st, hood) {
                t.args.insert("hood_key".into(), key);
                t.args.extend(extra);
            }
            return Some(t);
        }
        self.gate
            .reading(&subject)
            .filter(|r| self.gate.stale(r))
            .map(stale_note)
    }

    /// A place's note.
    pub(crate) fn place(&self, loc: Location) -> Option<TextView> {
        let Location::Place { place } = loc else {
            return None;
        };
        let p = self.engine.state().world.places.get(place.index())?;
        (!p.is_open()).then(|| TextView::new(KnowLine::PlaceShut.key()))
    }

    /// The templates of the bills that are the player's to move or vote on.
    fn bills(&self) -> &[String] {
        self.bills.get_or_init(|| {
            let st = self.engine.state();
            let Some(pl) = st.player.as_ref() else {
                return Vec::new();
            };
            let focus = pl.focus();
            st.policy
                .instances
                .iter()
                .filter(|p| {
                    matches!(
                        p.status,
                        PolicyStatus::Draft | PolicyStatus::UnderConsideration
                    ) && focus.contains(&p.at())
                })
                .filter(|p| {
                    p.sponsor == Some(pl.politician)
                        || crate::office::bill_view(self.engine, p).is_some_and(|b| b.can_vote)
                })
                .map(|p| p.template.clone())
                .collect()
        })
    }

    /// A notebook entry's note: an old reading wherever its figure is
    /// shown; else its subject's own, where the gap bears on something.
    pub(crate) fn entry(&self, s: &Subject, e: &Entry) -> Option<TextView> {
        if let Some(r) = e.reading.as_ref().filter(|r| self.gate.stale(r)) {
            return Some(stale_note(r));
        }
        match s {
            Subject::Person { person } => {
                let here = mandate_sim::talk::can_talk(
                    self.engine.state(),
                    self.engine.content(),
                    *person,
                );
                self.person(*person, &self.matters.person(self.engine, *person, here))
            }
            Subject::HoodConcerns { hood } => self.hood(*hood),
            Subject::PolicySupport {
                town,
                policy: PolicyRef::Template { template },
            } => bill_note(
                &self.gate,
                *town,
                template,
                self.bills().iter().any(|t| t == template),
            ),
            _ => None,
        }
    }
}
