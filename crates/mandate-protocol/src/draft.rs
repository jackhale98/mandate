//! The opening draft (DEC-101 §3; STAGED_ARCS §6.6): the openings a new
//! game offers, as the player may see them before choosing.
//!
//! Each opening is a real place and a real situation, read from the world
//! before any fork was played: the town, the arc template's title key and
//! the facts its `raise` gave there (keys and values; the client builds
//! the line, DEC-054), and the seat the player would hold. The forks
//! decide only the order. No score is shown: "nothing from a fork is shown
//! or kept, except the scores in the run log for the lab" (§6.6). The
//! pitch line and the frame are A8's (`pitch_key` is empty until then).

use crate::{DateView, PostDto, post_dto};
use mandate_sim::Engine;
use mandate_sim::arcs::audition::{self, Candidate, Draft, Params, SCAN_STEP_DAYS};
use mandate_sim::arcs::score::Opening;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// What an opening is compared by for variety (§6.5, §6.7): kept by the
/// device's Legacy record and passed back as `recent`.
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
    /// The facts its `raise` gave in the town, for the line's arguments.
    pub args: BTreeMap<String, String>,
    /// The matter is under way there already (else it opens on the first
    /// day of play).
    pub live: bool,
    /// The seat the player would hold.
    pub post: Option<PostDto>,
    /// The frame's pitch line (task A8); `None` until frames exist.
    pub pitch_key: Option<String>,
    pub tag: OpeningTagDto,
}

/// The draft: best first. It may hold fewer than three (fewer situations
/// built in their forks), or none.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DraftView {
    pub openings: Vec<OpeningView>,
    /// Steps of `scan_step_days` the world is advanced, with nobody
    /// playing, before play begins (0: it begins as generated).
    pub months_ahead: u32,
    pub scan_step_days: u32,
    /// The day play would begin.
    pub opens: DateView,
}

fn opening_view(engine: &Engine, c: &Candidate) -> OpeningView {
    let world = &engine.state().world;
    let m = world.municipalities.get(c.town.index());
    OpeningView {
        town: c.town.0,
        town_name: m.map(|m| m.name.clone()).unwrap_or_default(),
        town_class: c.opening.town_class.clone(),
        template: c.template.clone(),
        title_key: c.title_key.clone(),
        args: c.args.clone(),
        live: matches!(c.situation, audition::Situation::Live { .. }),
        post: c.post.map(|p| post_dto(engine, p)),
        pitch_key: None,
        tag: OpeningTagDto::of(&c.opening),
    }
}

/// The view of a draft already auditioned.
pub fn draft_view_of(engine: &Engine, d: &Draft) -> DraftView {
    DraftView {
        openings: d
            .offered()
            .map(|a| opening_view(engine, &a.candidate))
            .collect(),
        months_ahead: d.months_ahead,
        scan_step_days: SCAN_STEP_DAYS,
        opens: DateView::of(d.opens),
    }
}

/// Audition the running game's world and return the draft (a pure
/// function of the state, the content and `recent`; the state is only
/// read). Seconds of work: one query, under the history animation.
pub fn draft_view(engine: &Engine, recent: &[OpeningTagDto]) -> Result<DraftView, String> {
    let mut p = Params::for_state(engine.state());
    p.recent = recent.iter().map(OpeningTagDto::opening).collect();
    let content = engine.content_arc();
    let d = audition::audition(engine.state(), &content, p)?;
    Ok(draft_view_of(engine, &d))
}
