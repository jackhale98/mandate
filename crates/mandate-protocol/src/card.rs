//! The career card (M6 note 23): when a run ends, the facts to copy or
//! share, so that a player can compare how they did with others against
//! how hard their conditions were. Facts and Fluent keys only (DEC-054):
//! the client words the text and draws the image, and computes nothing.
//! Every figure (the ratings, the wins expected, the years served) is the
//! engine's (`mandate_sim::career`).
//!
//! Nothing personal: the character and the world only (no device, no run
//! id, no time of day).

use crate::act::{ActLineView, EndingView, ending_view, entry_line};
use crate::{DateView, PostDto, TextView, WorldSizeDto, post_dto};
use mandate_sim::Engine;
use mandate_sim::arcs::ArcId;
use mandate_sim::arcs::opening::OpeningChoice;
use mandate_sim::career::code::{CodeError, WorldCode};
use mandate_sim::career::{self, RaceRating};
use mandate_sim::events::EventId;
use mandate_sim::legacy::LegacyKind;
use mandate_sim::state::WorldSize;
use mandate_sim::world::Jurisdiction;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A factor behind a rating: its mean share of the margin between the
/// player and the other of the first two, in thousandths (above zero it
/// worked for the player).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RatingFactView {
    pub factor: String,
    /// `factor-<factor>`.
    pub key: String,
    pub margin_pm: i32,
}

/// A race's rating (DEC-102 clause 6): the race played on forks by the
/// engaged stand-in, one play per luck, from the day it was rated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RatingView {
    pub plays: u32,
    pub wins: u32,
    /// The win rate, in thousandths.
    pub win_pm: u32,
    pub on_ballot: u32,
    pub rated_on: DateView,
    pub facts: Vec<RatingFactView>,
}

impl RatingView {
    pub fn of(r: &RaceRating) -> Self {
        RatingView {
            plays: r.plays,
            wins: r.wins,
            win_pm: r.win_pm(),
            on_ballot: r.on_ballot,
            rated_on: DateView::of(r.rated_on),
            facts: r
                .facts
                .iter()
                .map(|f| RatingFactView {
                    key: format!("factor-{}", f.factor),
                    factor: f.factor.clone(),
                    margin_pm: f.margin_pm,
                })
                .collect(),
        }
    }
}

/// A race the player entered.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CardRaceView {
    pub post: PostDto,
    pub election_day: DateView,
    /// Its rating as it opened (none: it was not rated, such as a race of
    /// a run loaded under other rules).
    pub rating: Option<RatingView>,
    /// `won`, `lost`, `walked_away`, `undecided`.
    pub outcome: String,
    /// The player's share of the vote, in thousandths, once decided.
    pub share_pm: Option<u32>,
}

/// The opening the run began on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CardOpeningView {
    pub town: String,
    /// The arc the draft's opening was framed on (none: play began as the
    /// world was generated), with its title's arguments.
    pub arc: Option<TextView>,
    /// Steps of the draft's scan the world was advanced first.
    pub months_ahead: u32,
    pub began: DateView,
    /// The seat the player began in.
    pub post: Option<PostDto>,
    /// Its rating: the player's first race, rated on the world as play
    /// began (none: not measured, the run cannot be rebuilt).
    pub rating: Option<RatingView>,
}

/// How an arc in the player's focus ended (or stood when the run ended).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CardArcView {
    pub title: TextView,
    /// The exit's Legacy line (`legacy-arc-*`), else its stage's text; for
    /// one still under way, its stage's.
    pub text: TextView,
    pub on: DateView,
    pub under_way: bool,
}

/// The kind of world.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CardWorldView {
    pub size: WorldSizeDto,
    /// The scenario's name key (none: a random country).
    pub scenario_key: Option<String>,
    pub constitution_key: String,
}

/// Wins against the wins the ratings expected (the one comparable figure).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CardFigureView {
    /// Races rated and decided.
    pub rated: u32,
    pub wins: u32,
    /// Thousandths of a win.
    pub expected_pm: u32,
    /// Wins less expected wins, in thousandths.
    pub over_pm: i32,
}

/// The career card (M6 note 23).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CareerCardView {
    /// The player stepped away from politics: the card is of an unfinished
    /// run (`ended_on` and `ended_post` say when and as what).
    pub unfinished: bool,
    pub name: String,
    pub party: Option<String>,
    pub town: String,
    pub world: CardWorldView,
    pub opening: CardOpeningView,
    /// By election day.
    pub races: Vec<CardRaceView>,
    pub highest_post: Option<PostDto>,
    pub days_served: u32,
    /// Years served, in tenths.
    pub years_tenths: u32,
    /// The notable decisions of the Legacy record, oldest first.
    pub decisions: Vec<ActLineView>,
    /// How each arc in the player's focus ended, oldest first.
    pub arcs: Vec<CardArcView>,
    pub ending: Option<EndingView>,
    pub ended_on: DateView,
    /// The posts held when the run ended.
    pub ended_posts: Vec<PostDto>,
    pub figure: CardFigureView,
    /// The world code (`WorldCode`): none when the run cannot be rebuilt
    /// from its start (it began before run files, or was loaded under
    /// other rules).
    pub code: Option<String>,
    /// The engine version and the content's short hash.
    pub build: String,
}

/// What the card is made from besides the state: the run file's ratings
/// and its opening.
pub struct CardInputs<'a> {
    pub ratings: &'a [RaceRating],
    pub opening_rating: Option<&'a RaceRating>,
    pub opening: Option<&'a OpeningChoice>,
    pub code: Option<String>,
}

pub fn size_dto(s: WorldSize) -> WorldSizeDto {
    match s {
        WorldSize::Small => WorldSizeDto::Small,
        WorldSize::Medium => WorldSizeDto::Medium,
        WorldSize::Large => WorldSizeDto::Large,
    }
}

/// The build a card names.
pub fn build_label(content_hash: &str) -> String {
    format!(
        "{} · {}",
        mandate_core::ENGINE_VERSION,
        content_hash.get(..8).unwrap_or(content_hash)
    )
}

/// An arc's title and how it ended, from the run's cast (live or past)
/// and its closing event's facts where the log keeps it.
pub(crate) fn arc_ended_text(
    engine: &Engine,
    arc: u32,
    template: &str,
    exit: &str,
    source: Option<EventId>,
) -> (TextView, TextView) {
    let st = engine.state();
    let content = engine.content();
    let def = content.arc(template);
    let event = source.and_then(|id| st.events.iter().find(|e| e.id == id));
    let mut args = match event {
        Some(e) => crate::views::event_view(e, engine).args,
        None => {
            let at = st
                .arcs
                .past
                .iter()
                .find(|p| p.id == ArcId(arc))
                .map(|p| p.at);
            match (st.arcs.cast_of(ArcId(arc)), at) {
                (Some(c), Some(at)) => crate::arcs::text_args(engine, c, at, Vec::new()),
                _ => Default::default(),
            }
        }
    };
    args.remove("title_key");
    let title = TextView::of(
        def.map(|d| d.title_key.clone()).unwrap_or_default(),
        args.clone(),
    );
    let stage = def.and_then(|d| d.stage(exit));
    let key = stage
        .and_then(|s| s.exit.as_ref())
        .and_then(|x| x.legacy_key.clone())
        .or_else(|| stage.map(|s| s.key.clone()))
        .unwrap_or_default();
    (title, TextView::of(key, args))
}

/// The career card of a run whose act is over; `None` while it runs.
pub fn career_card_view(engine: &Engine, inp: &CardInputs<'_>) -> Option<CareerCardView> {
    let st = engine.state();
    if !st.act.over() {
        return None;
    }
    let content = engine.content();
    let pl = st.player.as_ref()?;
    let me = st.politics.get(pl.politician)?;
    let first = st.act.records.first()?;
    let last = st.act.records.last()?;
    let town = st
        .world
        .municipalities
        .get(pl.home().index())
        .map(|m| m.name.clone())
        .unwrap_or_default();
    let party = me
        .party
        .and_then(|id| st.politics.parties.iter().find(|p| p.id == id))
        .map(|p| p.name.clone());
    let c = content.constitutions.get(&st.meta.constitution);
    // The opening: where and on what play began.
    let opening_town = inp
        .opening
        .and_then(|o| st.world.municipalities.get(o.town as usize))
        .map_or_else(|| town.clone(), |m| m.name.clone());
    let opening_arc = inp.opening.and_then(|o| {
        let def = content.arc(&o.template)?;
        let at = Jurisdiction::town(mandate_sim::world::MunicipalityId(o.town));
        // The run framed on: the first of the template there.
        let past = st
            .arcs
            .past
            .iter()
            .find(|p| p.template == o.template && p.at == at)
            .map(|p| (p.id.0, p.cast.as_slice()));
        let live = st
            .arcs
            .runs
            .iter()
            .find(|r| r.template == o.template && r.at == at)
            .map(|r| (r.id.0, r.cast.as_slice()));
        let cast = match (past, live) {
            (Some(p), Some(l)) => Some(if p.0 <= l.0 { p.1 } else { l.1 }),
            (p, l) => p.or(l).map(|x| x.1),
        };
        let mut args = cast
            .map(|c| crate::arcs::text_args(engine, c, at, Vec::new()))
            .unwrap_or_default();
        args.remove("title_key");
        Some(TextView::of(def.title_key.clone(), args))
    });
    let opening = CardOpeningView {
        town: opening_town,
        arc: opening_arc,
        months_ahead: inp.opening.map_or(0, |o| o.months_ahead),
        began: DateView::of(first.began),
        post: first.start_posts.first().map(|p| post_dto(engine, *p)),
        rating: inp.opening_rating.map(RatingView::of),
    };
    let races = career::races(st, inp.ratings);
    let tally = career::tally(&races);
    let days = career::days_served(st);
    let mut decisions = Vec::new();
    let mut arcs = Vec::new();
    for rec in &st.act.records {
        for e in &rec.entries {
            match &e.kind {
                LegacyKind::Decision { .. } => {
                    decisions.push(entry_line(engine, e.date, &e.kind, e.source_event))
                }
                LegacyKind::ArcEnded {
                    arc,
                    template,
                    exit,
                } => {
                    let (title, text) =
                        arc_ended_text(engine, *arc, template, exit, e.source_event);
                    arcs.push(CardArcView {
                        title,
                        text,
                        on: DateView::of(e.date),
                        under_way: false,
                    });
                }
                _ => {}
            }
        }
    }
    // Arcs still under way in the player's focus when the run ended.
    let focus = pl.focus();
    for run in st.arcs.runs.iter().filter(|r| focus.contains(&r.at)) {
        let Some(def) = content.arc(&run.template) else {
            continue;
        };
        let Some(stage) = def.stage(&run.stage) else {
            continue;
        };
        if stage.hidden {
            continue;
        }
        let mut args = crate::arcs::live_args(engine, run);
        args.remove("title_key");
        arcs.push(CardArcView {
            title: TextView::of(def.title_key.clone(), args.clone()),
            text: TextView::of(stage.key.clone(), args),
            on: DateView::of(run.started),
            under_way: true,
        });
    }
    Some(CareerCardView {
        unfinished: st.act.stepped_away.is_some(),
        name: me.full_name(),
        party,
        town,
        world: CardWorldView {
            size: size_dto(st.meta.params.size),
            scenario_key: st
                .meta
                .params
                .scenario
                .as_deref()
                .and_then(|k| content.scenario(k))
                .map(|s| s.name_key.clone()),
            constitution_key: c.map(|c| c.name_key.clone()).unwrap_or_default(),
        },
        opening,
        races: races
            .iter()
            .map(|r| CardRaceView {
                post: post_dto(engine, r.post),
                election_day: DateView::of(r.election_day),
                rating: r.rating.as_ref().map(RatingView::of),
                outcome: r.outcome.key().into(),
                share_pm: r.share_pm,
            })
            .collect(),
        highest_post: career::highest_post(st, content).map(|p| post_dto(engine, p)),
        days_served: days,
        years_tenths: ((u64::from(days) * 10 + u64::from(mandate_core::time::DAYS_PER_YEAR) / 2)
            / u64::from(mandate_core::time::DAYS_PER_YEAR)) as u32,
        decisions,
        arcs,
        ending: ending_view(engine, last),
        ended_on: DateView::of(last.closed.unwrap_or(st.calendar.date)),
        ended_posts: last
            .final_standing
            .as_ref()
            .map(|f| f.posts.iter().map(|p| post_dto(engine, *p)).collect())
            .unwrap_or_default(),
        figure: CardFigureView {
            rated: tally.rated,
            wins: tally.wins,
            expected_pm: tally.expected_pm,
            over_pm: tally.over_pm,
        },
        code: inp.code.clone(),
        build: build_label(&content.content_hash),
    })
}

/// The opening a world code names.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CodeOpeningView {
    pub months_ahead: u32,
    pub town: u32,
    /// The arc's title key.
    pub title_key: String,
}

/// A world code read (`QueryDto::WorldCode`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorldCodeView {
    /// The code as this build writes it.
    pub code: String,
    /// Made under this build. A code from another build may give another
    /// world, or none.
    pub same_build: bool,
    #[ts(type = "string")]
    #[serde(with = "crate::u64_string")]
    pub seed: u64,
    pub size: WorldSizeDto,
    pub scenario_key: Option<String>,
    pub storyteller_key: Option<String>,
    pub local_knowledge: bool,
    pub opening: Option<CodeOpeningView>,
}

/// A world code read, or why not (`ui-code-error-<key>`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorldCodeCheckView {
    pub world: Option<WorldCodeView>,
    /// `malformed`, `version`, `check`, `unknown`, `other_build`.
    pub error: Option<String>,
}

pub fn world_code_view(content: &mandate_content::ContentBundle, code: &str) -> WorldCodeCheckView {
    match WorldCode::decode(code, content) {
        Ok(w) => WorldCodeCheckView {
            world: Some(WorldCodeView {
                code: w.encode(),
                same_build: w.same_build(content),
                seed: w.seed,
                size: size_dto(w.size),
                scenario_key: w
                    .scenario
                    .as_deref()
                    .and_then(|k| content.scenario(k))
                    .map(|s| s.name_key.clone()),
                storyteller_key: w
                    .storyteller
                    .as_deref()
                    .and_then(|k| content.storyteller(k))
                    .map(|s| s.name_key.clone()),
                local_knowledge: w.local_knowledge,
                opening: w.opening.as_ref().map(|o| CodeOpeningView {
                    months_ahead: o.months_ahead,
                    town: o.town,
                    title_key: content
                        .arc(&o.template)
                        .map(|a| a.title_key.clone())
                        .unwrap_or_default(),
                }),
            }),
            error: None,
        },
        Err(e) => WorldCodeCheckView {
            world: None,
            error: Some(code_error_key(e).into()),
        },
    }
}

pub fn code_error_key(e: CodeError) -> &'static str {
    e.key()
}
