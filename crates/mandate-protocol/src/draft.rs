//! The opening draft (DEC-101 §3; STAGED_ARCS §6.6): the openings a new
//! game offers, as the player may see them before choosing.
//!
//! Each opening is a real place and a real situation, read from the world
//! before any fork was played: the town, the arc template's title key, its
//! frame's line (`pitch_key`, with the run's text arguments on the day
//! play would begin) and the seat the player would hold, as the frame
//! seats them (`arcs::opening`). The forks decide only the order. No score
//! is shown: "nothing from a fork is shown or kept, except the scores in
//! the run log for the lab" (§6.6).

use crate::{DateView, PostDto, post_dto};
use mandate_sim::Engine;
use mandate_sim::arcs::audition::{self, Auditioned, Draft, Params, SCAN_STEP_DAYS};
use mandate_sim::arcs::score::Opening;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// What an opening is compared by for variety (§6.5, §6.7): kept by the
/// device and passed back as `NewGameParams.recent_openings`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OpeningTagDto {
    pub template: String,
    /// The hazard, else the town's defining industry (a content id).
    pub matter: String,
    /// The post's role word (the constitution's).
    pub post: String,
    pub town_class: String,
}

impl OpeningTagDto {
    pub fn of(o: &Opening) -> Self {
        OpeningTagDto {
            template: o.template.clone(),
            matter: o.matter.clone(),
            post: o.post.clone(),
            town_class: o.town_class.clone(),
        }
    }

    pub fn opening(&self) -> Opening {
        Opening {
            template: self.template.clone(),
            matter: self.matter.clone(),
            post: self.post.clone(),
            town_class: self.town_class.clone(),
        }
    }
}

/// One opening on offer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OpeningView {
    pub town: u32,
    pub town_name: String,
    /// `village`, `town`, `city` (the settlement class).
    pub town_class: String,
    /// The arc template's id and its title key (`arc-plant`).
    pub template: String,
    pub title_key: String,
    /// The line's arguments: the facts its `raise` gave in the town, and
    /// the run's text arguments as play would begin (its cast by role,
    /// `place`, `town`, the dates, its facts).
    pub args: BTreeMap<String, String>,
    /// The matter is under way there already (else it opens on the first
    /// day of play).
    pub live: bool,
    /// The seat the player would hold.
    pub post: Option<PostDto>,
    /// The frame's line (`pitch-*`).
    pub pitch_key: Option<String>,
    pub tag: OpeningTagDto,
}

/// The draft: best first. It may hold fewer than three (fewer situations
/// built in their forks, or gave a beat on turn 1), or none.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DraftView {
    pub openings: Vec<OpeningView>,
    /// Steps of `scan_step_days` the world is advanced, with nobody
    /// playing, before play begins (0: it begins as generated).
    pub months_ahead: u32,
    pub scan_step_days: u32,
    /// The day play begins (the day after the world the draft was found
    /// in: the frame steps one, on which the opening's run is raised).
    pub opens: DateView,
}

/// The audition under way (`QueryDto::DraftStep`): forks played of those
/// planned. The total grows once the second pass is planned.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DraftProgressView {
    pub done: u32,
    pub total: u32,
}

fn opening_view(engine: &Engine, a: &Auditioned) -> OpeningView {
    let c = &a.candidate;
    let world = &engine.state().world;
    let m = world.municipalities.get(c.town.index());
    let mut args = c.args.clone();
    args.extend(a.arrival.args.clone());
    OpeningView {
        town: c.town.0,
        town_name: m.map(|m| m.name.clone()).unwrap_or_default(),
        town_class: c.opening.town_class.clone(),
        template: c.template.clone(),
        title_key: c.title_key.clone(),
        args,
        live: matches!(c.situation, audition::Situation::Live { .. }),
        post: a.arrival.post.or(c.post).map(|p| post_dto(engine, p)),
        pitch_key: engine
            .content()
            .arc(&c.template)
            .and_then(|d| d.frame.as_ref())
            .map(|f| f.pitch_key.clone()),
        tag: OpeningTagDto::of(&c.opening),
    }
}

/// The view of a draft already auditioned.
pub fn draft_view_of(engine: &Engine, d: &Draft) -> DraftView {
    DraftView {
        openings: d.offered().map(|a| opening_view(engine, a)).collect(),
        months_ahead: d.months_ahead,
        scan_step_days: SCAN_STEP_DAYS,
        opens: DateView::of(d.opens.add_days(1)),
    }
}

/// The audition's parameters for the running game: by its size, against
/// the device's last openings.
pub fn draft_params(engine: &Engine, recent: &[Opening]) -> Params {
    let mut p = Params::for_state(engine.state());
    p.recent = recent.to_vec();
    p
}
