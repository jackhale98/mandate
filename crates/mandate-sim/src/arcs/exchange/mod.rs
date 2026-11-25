//! Exchange scenes (DEC-112; `docs/design/EXCHANGE_SCENES.md`): an arc's
//! turn is played. The eighth beat kind of a staged arc: a scene of a few
//! rounds in which the other side puts a true thing to the player, the
//! player answers with a line they have earned, and the people who matter
//! lean a little one way or the other.
//!
//! **It is not a minigame.** Every quantity it reads is one the game
//! already keeps (a law's ledger, a worry's rank in a place, promises,
//! declared votes, what the player knows, opinions, the run's heat), and
//! everything it changes outside the arcs' own state is an effect request
//! to the owner, with the round's event as its cause.
//!
//! **Shown = rolled.** [`landing::land`] is the one function that says what
//! a line does. The view calls it (at the ends of what the player does not
//! know) and the command calls it; there is no second formula.
//!
//! **Beats write; transitions read.** A scene ends won, split, lost or
//! absent, sets marks and posts requests. It never names a stage or an
//! exit.
//!
//! - [`state`]: what is saved on the run.
//! - [`audience`]: groups bound from live state, and how much each cares.
//! - [`attacks`]: the other side: sources, the deck, the round's draw.
//! - [`moves`]: the player's lines, the ones shown, the locked one.
//! - [`landing`]: the formula.
//! - [`play`]: the command (begin, say, leave), the empty chair, the end.
//! - [`view`]: what a player sees before choosing, as far as they know it.
//!
//! Tier-generic (DEC-098): a scene is on a run, a run is keyed by a
//! jurisdiction, the audience is bound by source there, and no code here
//! names an office, a body, a ward, a market or an industry.

pub mod attacks;
pub mod audience;
pub mod landing;
pub mod moves;
pub mod play;
pub mod state;
pub mod view;

pub use play::{ExchangeAct, apply};
pub use state::{
    AnswerHeld, ArcExchangeState, Attack, ExchangePlayed, Group, GroupLean, PromptNow, RoundPlayed,
    Subject, What,
};
pub use view::{OptionView, Phase, SceneView, view, view_known};

use crate::arcs::{ArcId, ArcRun, BeatRef};
use crate::narrative::Chapter;
use crate::places::Location;
use mandate_content::ContentBundle;
use mandate_content::arcs::{ArcBeat, ArcDef, BeatKind, StageDef};
use mandate_content::exchange::{ArcExchangeDef, ExchangeParams};
use mandate_core::SimDate;

/// A scene's content, resolved for a run.
pub struct Scene<'c> {
    pub arc: &'c ArcDef,
    pub stage: &'c StageDef,
    pub beat: &'c ArcBeat,
    pub def: &'c ArcExchangeDef,
    pub bref: BeatRef,
    /// The constants as this scene in this tier sees them.
    pub params: ExchangeParams,
}

/// A beat as a command names it: `<stage>.<beat>`, or the beat's id alone
/// (of the run's current stage).
pub fn beat_ref(run: &ArcRun, beat: &str) -> BeatRef {
    match beat.split_once('.') {
        Some((stage, id)) => BeatRef::new(stage, id),
        None => BeatRef::new(&run.stage, beat),
    }
}

/// The exchange a beat of a run is, if it is one.
pub fn scene<'c>(content: &'c ContentBundle, run: &ArcRun, bref: &BeatRef) -> Option<Scene<'c>> {
    let arc = content.arc(&run.template)?;
    let stage = arc.stage(&bref.stage)?;
    let beat = stage.beat(&bref.beat)?;
    let BeatKind::Exchange(def) = &beat.kind else {
        return None;
    };
    Some(Scene {
        arc,
        stage,
        beat,
        def,
        bref: bref.clone(),
        params: content.exchange.resolved(run.at.tier(), &def.params),
    })
}

/// Whether a template has a scene at all.
pub fn has_scene(arc: &ArcDef) -> bool {
    arc.beats()
        .any(|(_, b)| matches!(b.kind, BeatKind::Exchange(_)))
}

/// Who may hold a scene in this chapter (DEC-112, the owner's answer 4:
/// at most one a chapter, the lead arc's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Staging {
    /// One has been held since the chapter began.
    Nobody,
    /// The chapter's lead has a scene: only it.
    Only(ArcId),
    /// The lead has none (or there is no chapter): whichever comes first.
    Any,
}

impl Staging {
    pub fn allows(self, run: ArcId) -> bool {
        match self {
            Staging::Nobody => false,
            Staging::Only(id) => id == run,
            Staging::Any => true,
        }
    }
}

/// The chapter's staging, from the live runs and the chapter.
pub fn staging(runs: &[ArcRun], content: &ContentBundle, chapter: Option<&Chapter>) -> Staging {
    let Some(ch) = chapter else {
        return Staging::Any;
    };
    if runs
        .iter()
        .any(|r| r.played.iter().any(|p| p.on >= ch.start))
    {
        return Staging::Nobody;
    }
    let lead = ch
        .lead
        .and_then(|id| runs.iter().find(|r| r.id.0 == id))
        .filter(|r| content.arc(&r.template).is_some_and(has_scene));
    match lead {
        Some(r) => Staging::Only(r.id),
        None => Staging::Any,
    }
}

/// The last day the scene can be begun: the day before the stage's clock
/// runs out (the empty chair is on the clock's day).
pub fn last_day(run: &ArcRun) -> Option<SimDate> {
    run.clock.map(|c| SimDate(c.0.saturating_sub(1)))
}

/// The scene's day has passed.
pub fn day_passed(run: &ArcRun, today: SimDate) -> bool {
    run.clock.is_some_and(|c| today >= c)
}

/// Where a scene is held: the cast place.
pub fn place(run: &ArcRun, def: &ArcExchangeDef) -> Option<Location> {
    match run.actor(&def.at)? {
        crate::arcs::ArcActor::Place { id } => Some(Location::Place { place: id }),
        _ => None,
    }
}
