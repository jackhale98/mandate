//! The audition (DEC-101 §5; STAGED_ARCS §6): which openings are worth
//! offering. A pure function of the world as play begins, the content and
//! the parameters.
//!
//! 1. **Candidates** ([`scan`]). For every town where the constitution's
//!    first career act offers an entry post (a seat in the local
//!    legislature), every arc template whose `raise` holds there and whose
//!    cast can be bound ([`score::probe`]), or whose run is already live
//!    there. Each is a [`Candidate`]: the place, the template, the
//!    situation's facts (the `raise` conditions' arguments, read from the
//!    unforked state) and the opening it would be ([`score::Opening`]).
//!    The first [`Params::candidates`] (K) by the prior are kept: no two in
//!    one town, at most two of one template. If fewer than
//!    [`Params::draft`] are kept, the world is advanced
//!    [`SCAN_STEP_DAYS`] with nobody playing and scanned again, up to
//!    [`Params::scan_months`] times ([`Draft::months_ahead`] says how far:
//!    the game the player then starts is that world).
//! 2. **Forks** ([`play`]). Each candidate is played on a copy of the
//!    state under its own luck ([`luck`]: keyed on the world seed, the
//!    town and the template, through `standin::reluck`, so a fork draws
//!    from streams the real game never uses), with a [`Frame`] applied
//!    first (A8's seam; today [`Seated`]: the player holds a seat in the
//!    candidate's town from turn 1), for [`Params::horizon_weeks`].
//! 3. **Stand-ins** (`crate::standin`): passive, engaged, contrarian.
//! 4. **Two passes.** Passive for all K; engaged and contrarian for the
//!    best [`Params::second_pass`] by the first-pass product E × V × P.
//! 5. **The score**: the four terms and variety of `score`, each 0 to 1,
//!    and their weighted sum by §6.5's weights ([`WEIGHTS`]). The floors
//!    (A ≥ 0.3, V ≥ 0.4, a beat on turn 1) are measured on every
//!    candidate; whether one under them is dropped is a parameter
//!    ([`Params::enforce_floors`], off: report-only until the tuning pass,
//!    `TUNING_QUEUE.md`).
//! 6. **The draft** ([`Draft::openings`]): the best three by score among
//!    the candidates that escalate in their passive fork ([`escalates`]),
//!    no two of one template while enough templates qualify, one
//!    non-material (a standing arc) if any qualifies and none is among the
//!    three.
//!
//! **Nothing leaks.** The forks are copies, dropped when scored; the real
//! state is only read (its hash is unchanged). What the player may see of
//! an opening ([`Candidate`]) is read from the unforked state before any
//! fork is played; the forks give only the order. The scores stay in the
//! [`Draft`] for the lab and the run log (§6.6).
//!
//! **Incremental.** [`Audition::begin`] scans; each [`Audition::step`]
//! plays one fork (one worker message under the history animation, §6.8);
//! [`Audition::finish`] ranks. [`audition`] does all three.
//!
//! Tier-generic (DEC-098): the entry post is the constitution's (the
//! legislature of the first career act's tier), and a seat is a `Post`.

use super::score::{
    self, Agency, Escalation, Opening, Pacing, Trace, Vector, Visibility, Watch, evp,
};
use crate::condition::{Args, Ctx};
use crate::politics::{PoliticianId, Post, members_of};
use crate::standin::{self, Career, Kind, Seat, Stepped};
use crate::state::State;
use crate::world::{Jurisdiction, MunicipalityId};
use crate::{Command, Engine};
use mandate_content::arcs::ArcDef;
use mandate_content::{ContentBundle, TierKey};
use mandate_core::Stream;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Keyed-draw tag of a candidate's fork luck (`Stream::Misc`).
pub const KEY_AUDITION: u64 = 0xA0D1_7104;

/// The forward scan's step: the world is advanced this many days with
/// nobody playing before it is scanned again.
pub const SCAN_STEP_DAYS: u32 = 30;

/// §6.5's weights: agency, escalation, visibility, pacing, variety.
pub const WEIGHTS: Weights = Weights {
    agency: 0.30,
    escalation: 0.25,
    visibility: 0.20,
    pacing: 0.15,
    variety: 0.10,
};

/// The weights of the score's terms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights {
    pub agency: f64,
    pub escalation: f64,
    pub visibility: f64,
    pub pacing: f64,
    pub variety: f64,
}

/// What the audition is asked to do.
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    /// K: candidates kept for the forks.
    pub candidates: usize,
    /// Candidates played by the engaged and contrarian stand-ins.
    pub second_pass: usize,
    /// Openings offered.
    pub draft: usize,
    /// Weeks each fork is played.
    pub horizon_weeks: u32,
    /// Times the world may be advanced [`SCAN_STEP_DAYS`] when fewer than
    /// [`Params::draft`] candidates are kept.
    pub scan_months: u32,
    /// The device's last openings (§6.7), oldest first: variety is
    /// measured against the last three.
    pub recent: Vec<Opening>,
    /// Drop a candidate under a floor (A, V, the turn-1 beat). Off: the
    /// floors are reported, not applied (rule 11, `TUNING_QUEUE.md`).
    pub enforce_floors: bool,
}

/// A world's size class, as K is chosen by (§6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeClass {
    Small,
    Medium,
    Large,
}

impl SizeClass {
    pub fn of(state: &State) -> Self {
        match state.meta.params.size {
            crate::state::WorldSize::Small => SizeClass::Small,
            crate::state::WorldSize::Medium => SizeClass::Medium,
            crate::state::WorldSize::Large => SizeClass::Large,
        }
    }
}

impl Params {
    /// The design's K of 8, 5 or 3 by size; 39 weeks; up to 36 months of
    /// forward scan; floors reported. The second pass is the draft's size,
    /// three, where the design says five: measured on 2026-11-26 (16 small
    /// worlds, 4 medium, 2 large), the fourth and fifth never changed a
    /// draft and cost a sixth of the audition (`lab audition
    /// --second-pass 5` plays the design's).
    pub fn for_size(size: SizeClass) -> Self {
        Params {
            candidates: match size {
                SizeClass::Small => 8,
                SizeClass::Medium => 5,
                SizeClass::Large => 3,
            },
            second_pass: 3,
            draft: 3,
            horizon_weeks: score::HORIZON_WEEKS,
            scan_months: 36,
            recent: Vec::new(),
            enforce_floors: false,
        }
    }

    pub fn for_state(state: &State) -> Self {
        Params::for_size(SizeClass::of(state))
    }
}

/// The situation a candidate opens on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Situation {
    /// The template's run is live in the town already.
    Live { run: u32, stage: String },
    /// Its `raise` holds there and a cast can be bound: it opens as soon
    /// as the town is followed (in the fork, the first day).
    Holds,
}

/// One opening that could be offered, as far as the unforked world shows
/// it: day-0 facts only.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub at: Jurisdiction,
    pub town: MunicipalityId,
    pub template: String,
    /// The template's index in the content (its order, and its key in
    /// [`luck`]).
    pub index: usize,
    pub title_key: String,
    pub situation: Situation,
    /// The `raise` conditions' arguments in the town (facts for a pitch).
    pub args: Args,
    /// The seat the player would hold there: the entry post's (none if no
    /// seat could be found).
    pub post: Option<Post>,
    pub opening: Opening,
    /// A matter of the world's, not a standing arc of the calendar's.
    pub material: bool,
    /// Variety against [`Params::recent`] (§6.5's N), read with the scan.
    pub variety: f64,
    /// The heat it opens at: a live run's, else the template's start or
    /// its floor's reading in the town, whichever is higher.
    pub heat: f32,
    /// The fork's luck ([`luck`]).
    pub luck: u64,
}

/// The luck a candidate's forks are played under: keyed on the world
/// seed, the town and the template (never 0, which is the game's own).
pub fn luck(state: &State, town: MunicipalityId, index: usize) -> u64 {
    mandate_core::rng::keyed(
        state.meta.seed,
        Stream::Misc,
        &[KEY_AUDITION, u64::from(town.0), index as u64],
    )
    .next_u64()
    .max(1)
}

/// The legislature of the constitution's first career act (the entry post
/// is a seat in it): its body id and tier.
fn entry_body(state: &State, content: &ContentBundle) -> Option<(String, TierKey)> {
    let c = content.constitutions.get(&state.meta.constitution)?;
    let tier = c
        .career
        .acts
        .first()
        .and_then(|a| a.tier)
        .unwrap_or(TierKey::Local);
    let body = crate::politics::legislature(c, tier)?;
    Some((body.id.clone(), tier))
}

/// The seat a new player would take in a town, as today's start chooses
/// one (worldgen's `seat_player`): the entry body's member of a major
/// party who won by the thinnest margin, else the middle member.
pub fn entry_seat(
    state: &State,
    content: &ContentBundle,
    town: MunicipalityId,
) -> Option<PoliticianId> {
    let (body, _) = entry_body(state, content)?;
    let pol = &state.politics;
    let members = members_of(pol, &body, Jurisdiction::town(town));
    let margin_of = |id: PoliticianId| -> f32 {
        pol.elections
            .results
            .iter()
            .rev()
            .find(|r| r.winner == id)
            .map(|r| {
                let mut s: Vec<f32> = r.candidates.iter().map(|c| c.share).collect();
                s.sort_by(|a, b| b.total_cmp(a));
                s[0] - s.get(1).copied().unwrap_or(0.0)
            })
            .unwrap_or(0.5)
    };
    let major = |id: &PoliticianId| {
        pol.get(*id)
            .is_some_and(|p| p.party.is_some_and(|x| x.0 < 2))
    };
    members
        .iter()
        .copied()
        .filter(major)
        .min_by(|a, b| margin_of(*a).total_cmp(&margin_of(*b)))
        .or_else(|| members.get(members.len() / 2).copied())
}

/// The towns an opening may be in: those whose entry body has members,
/// the capital aside (today's start never seats the player there).
fn towns(state: &State, content: &ContentBundle) -> Vec<MunicipalityId> {
    let Some((body, _)) = entry_body(state, content) else {
        return Vec::new();
    };
    state
        .world
        .municipalities
        .iter()
        .map(|m| m.id)
        .filter(|m| Some(*m) != state.world.capital)
        .filter(|m| !members_of(&state.politics, &body, Jurisdiction::town(*m)).is_empty())
        .collect()
}

/// Every candidate in the world as it stands, in town order then template
/// order. Reads only.
pub fn scan(state: &State, content: &ContentBundle, recent: &[Opening]) -> Vec<Candidate> {
    let mut out = Vec::new();
    for town in towns(state, content) {
        let at = Jurisdiction::town(town);
        let post = entry_seat(state, content, town)
            .and_then(|id| state.politics.get(id))
            .and_then(|p| p.role.post());
        for (index, def) in content.arcs.iter().enumerate() {
            if !def.tiers.contains(&at.tier()) || !super::not_yet(def).is_empty() {
                continue;
            }
            let live = state
                .arcs
                .at(at)
                .find(|r| r.template == def.id)
                .map(|r| Situation::Live {
                    run: r.id.0,
                    stage: r.stage.clone(),
                });
            let heat = match state.arcs.at(at).find(|r| r.template == def.id) {
                Some(r) => r.heat,
                None => opening_heat(state, content, def, town),
            };
            let situation = match live {
                Some(s) => s,
                None => {
                    let p = score::probe(state, content, def, at);
                    if !(p.raise && p.cast) {
                        continue;
                    }
                    Situation::Holds
                }
            };
            let opening = score::opening(state, content, def, at, post);
            out.push(Candidate {
                at,
                town,
                template: def.id.clone(),
                index,
                title_key: def.title_key.clone(),
                situation,
                args: facts(state, content, def, at),
                post,
                variety: score::variety(&opening, recent),
                heat,
                opening,
                material: def.standing.is_none(),
                luck: luck(state, town, index),
            });
        }
    }
    out
}

/// The heat a run of `def` would open at in a town: its start, or its
/// floor's reading there (as the step reads it), whichever is higher.
fn opening_heat(state: &State, content: &ContentBundle, def: &ArcDef, town: MunicipalityId) -> f32 {
    let floor = def.heat.floor.as_ref().map_or(0.0, |f| {
        let r = crate::scheduler::Reads::of_state(state, content);
        (crate::concerns::indicator(&r, town.index(), f.indicator) * f.scale).clamp(0.0, 1.0)
    });
    def.heat.start.clamp(0.0, 1.0).max(floor as f32)
}

/// The arguments a template's `raise` conditions give at a jurisdiction
/// (the facts a pitch is built from).
fn facts(state: &State, content: &ContentBundle, def: &ArcDef, at: Jurisdiction) -> Args {
    crate::condition::on_state(state, content, |v| {
        let cx = Ctx::new(v, None, Some(at));
        let mut args = Args::new();
        crate::condition::all_hold(&cx, &def.raise, &mut args);
        args
    })
}

/// The prior candidates are kept by (§6.1): the heat it opens at × its
/// variety (the design's "heat floor × cast quality × novelty"; no cast is
/// rated yet, so its quality is left out), then a matter of the world's
/// before a standing arc, one already live before one that would open,
/// then town and template order. No fitted number.
pub fn prior(c: &Candidate) -> f64 {
    f64::from(c.heat) * c.variety
}

fn by_prior(a: &Candidate, b: &Candidate) -> std::cmp::Ordering {
    prior(b)
        .total_cmp(&prior(a))
        .then_with(|| b.material.cmp(&a.material))
        .then_with(|| {
            matches!(b.situation, Situation::Live { .. })
                .cmp(&matches!(a.situation, Situation::Live { .. }))
        })
        .then_with(|| a.town.cmp(&b.town))
        .then_with(|| a.index.cmp(&b.index))
}

/// The first `k` by the prior: no two in one town, at most two of one
/// template.
pub fn keep(mut all: Vec<Candidate>, k: usize) -> Vec<Candidate> {
    all.sort_by(by_prior);
    let mut kept: Vec<Candidate> = Vec::new();
    for c in all {
        if kept.len() >= k {
            break;
        }
        let same_town = kept.iter().any(|x| x.town == c.town);
        let same_template = kept.iter().filter(|x| x.template == c.template).count();
        if !same_town && same_template < 2 {
            kept.push(c);
        }
    }
    kept
}

/// What a candidate's opening does to a fork before play: A8's seam for
/// the opening frames (the post, the clock, the ally and opponent, the
/// hidden fact, the first beat). It writes only the fork.
pub trait Frame {
    fn apply(
        &self,
        state: &mut State,
        content: &ContentBundle,
        c: &Candidate,
    ) -> Result<(), String>;
}

/// Today's frame: the player holds the entry seat in the candidate's town
/// from turn 1, as today's start seats them in theirs, and nothing else.
pub struct Seated;

impl Frame for Seated {
    fn apply(
        &self,
        state: &mut State,
        content: &ContentBundle,
        c: &Candidate,
    ) -> Result<(), String> {
        seat_in(state, content, c.town)
    }
}

/// Seat the player in the entry seat of `town`, as a new game would have
/// (the start's stats, awareness in the district, the ledger seeded for
/// the town, the decks and the day): for a fork, or for a start the draft
/// chose. A player already at home there is left as they are.
pub fn seat_in(
    state: &mut State,
    content: &ContentBundle,
    town: MunicipalityId,
) -> Result<(), String> {
    if crate::player::home(state) == Some(town) {
        return Ok(());
    }
    let year = state.calendar.date.calendar().year;
    let seat = entry_seat(state, content, town).ok_or("no seat to take in the town")?;
    let old = state.player.as_ref().ok_or("no player")?.politician;
    if let Some(p) = state.politics.get_mut(old) {
        p.is_player = false;
    }
    let world = &state.world;
    let me = state.politics.get_mut(seat).ok_or("no such politician")?;
    me.is_player = true;
    me.oratory = 0.6;
    me.negotiation = 0.6;
    me.birth_year = year - 38;
    me.years_in_office = 3;
    me.terms_in_office = 1;
    me.scandal = 0.0;
    me.record = 0.3;
    let home_hood = crate::world::NeighborhoodId(me.home);
    let at = Jurisdiction::town(town);
    let ward = me
        .role
        .constituency()
        .or_else(|| {
            at.constituencies(world)
                .find(|c| c.neighborhoods.contains(&home_hood))
                .map(|c| c.id)
        })
        .ok_or("the seat has no district")?;
    let known: Vec<usize> = match me.role.post() {
        Some(p) if p.is_office() => world.hoods_of(town).collect(),
        _ => world
            .constituencies
            .get(ward.index())
            .map(|c| c.neighborhoods.iter().map(|h| h.index()).collect())
            .unwrap_or_default(),
    };
    for h in known {
        if let Some(hood) = state.population.hoods.get_mut(h) {
            for c in hood.cohorts.iter_mut() {
                c.player_awareness = 0.3;
                c.player_approval = 0.1;
            }
        }
    }
    let pl = state.player.as_mut().ok_or("no player")?;
    pl.politician = seat;
    pl.municipality = town;
    pl.ward = ward;
    pl.location = crate::places::Location::Home;
    pl.knowledge = Default::default();
    pl.talk = Default::default();
    pl.ground = Default::default();
    crate::player::knowledge::seed(state, content, None, false)?;
    crate::policy::families::on_load(state, content);
    state.calendar.day = crate::characters::calendar::choose_day(state, content);
    Ok(())
}

/// One stand-in's play of a candidate's fork.
#[derive(Clone, Debug, PartialEq)]
pub struct Fork {
    pub kind: Kind,
    pub trace: Trace,
    /// The world at the horizon.
    pub vector: Vector,
    pub horizon_turns: u32,
    /// Days simulated in the fork.
    pub days: u32,
    pub state_hash: String,
}

/// Play one fork of `base` for a candidate: a copy under the candidate's
/// luck, the frame applied, the stand-in playing for the seat to the
/// horizon. The copy is dropped; `base` is only read.
pub fn play(
    base: &State,
    content: &Arc<ContentBundle>,
    c: &Candidate,
    kind: Kind,
    frame: &dyn Frame,
    horizon_weeks: u32,
) -> Result<Fork, String> {
    let mut st = base.clone();
    standin::reluck(&mut st, c.luck);
    frame.apply(&mut st, content, c)?;
    let mut e = Engine::from_state(st, content.clone()).map_err(|x| x.to_string())?;
    let seat = Seat::of_player(e.state(), e.content()).ok_or("no seat")?;
    let mut career = Career::new(kind);
    career.seat = Some(Seat { at: c.at, ..seat });
    let start = e.state().calendar.date;
    let turn0 = e.state().calendar.turn;
    let horizon = horizon_weeks * 7;
    let mut watch = Watch::new(e.state(), &c.template, c.at);
    watch.observe(e.state(), content);
    loop {
        if start.days_until(e.state().calendar.date) >= horizon {
            break;
        }
        if standin::step(&mut e, &mut career) == Stepped::Over {
            break;
        }
        watch.observe(e.state(), content);
    }
    let st = e.state();
    let cast = st
        .arcs
        .runs
        .iter()
        .find(|r| Some(r.id.0) == watch.trace.run)
        .map(|r| r.cast.clone())
        .or_else(|| {
            st.arcs
                .past
                .iter()
                .rev()
                .find(|p| Some(p.id.0) == watch.trace.run)
                .map(|p| p.cast.clone())
        })
        .unwrap_or_default();
    Ok(Fork {
        kind,
        vector: score::vector(st, content, c.at, &cast, seat.post),
        horizon_turns: st.calendar.turn - turn0,
        days: start.days_until(st.calendar.date),
        state_hash: e.state_hash(),
        trace: watch.trace,
    })
}

/// Whether a candidate's matter built in its passive fork: its run was
/// raised and moved past the stage it opened in, or its heat rose above
/// where the template starts it.
pub fn escalates(def: Option<&ArcDef>, passive: &Trace) -> bool {
    let start = def.map_or(0.0, |d| d.heat.start);
    passive.run.is_some() && (passive.stages_entered() >= 2 || passive.peak_heat > start)
}

/// The floors a candidate is measured against (§6.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Floors {
    pub agency: bool,
    pub visibility: bool,
    pub first_beat: bool,
}

impl Floors {
    pub fn met(&self) -> bool {
        self.agency && self.visibility && self.first_beat
    }
}

/// A candidate's score: the terms and their weighted sum.
#[derive(Clone, Debug, PartialEq)]
pub struct Score {
    pub escalation: Escalation,
    pub visibility: Visibility,
    /// From the engaged run (the passive one's in the first pass).
    pub pacing: Pacing,
    /// Only for the second pass.
    pub agency: Option<Agency>,
    pub variety: f64,
    /// E × V × P on the passive run (the first pass's order).
    pub first_pass: f64,
    /// The weighted sum ([`WEIGHTS`]); agency counts 0 without a second
    /// pass.
    pub total: f64,
    pub floors: Floors,
    pub escalates: bool,
}

impl Score {
    fn of(c: &Candidate, def: Option<&ArcDef>, forks: &[Fork]) -> Option<Score> {
        let by = |k: Kind| forks.iter().find(|f| f.kind == k);
        let pa = by(Kind::Passive)?;
        let escalation = Escalation::of(&pa.trace);
        let visibility = Visibility::of(&pa.trace);
        let first = Pacing::of(&pa.trace, pa.horizon_turns);
        let first_pass = evp(&escalation, &visibility, &first);
        let (pacing, agency) = match (by(Kind::Engaged), by(Kind::Contrarian)) {
            (Some(en), Some(co)) => (
                Pacing::of(&en.trace, en.horizon_turns),
                Some(Agency::of(
                    &pa.trace,
                    &en.trace,
                    &co.trace,
                    score::distance(&pa.vector, &en.vector),
                )),
            ),
            _ => (first, None),
        };
        let a = agency.as_ref().map_or(0.0, Agency::value);
        let w = WEIGHTS;
        let total = w.agency * a
            + w.escalation * escalation.value()
            + w.visibility * visibility.value()
            + w.pacing * pacing.value()
            + w.variety * c.variety;
        let floors = Floors {
            agency: a >= AGENCY_FLOOR,
            visibility: visibility.value() >= VISIBILITY_FLOOR,
            first_beat: pacing.beat_on_turn_1,
        };
        Some(Score {
            escalates: escalates(def, &pa.trace),
            escalation,
            visibility,
            pacing,
            agency,
            variety: c.variety,
            first_pass,
            total,
            floors,
        })
    }
}

/// The floors of §6.5.
pub const AGENCY_FLOOR: f64 = 0.3;
pub const VISIBILITY_FLOOR: f64 = 0.4;

/// A candidate auditioned.
#[derive(Clone, Debug, PartialEq)]
pub struct Auditioned {
    pub candidate: Candidate,
    /// Passive, then engaged and contrarian if it reached the second pass.
    pub forks: Vec<Fork>,
    pub score: Option<Score>,
    /// Why it was not played, or a fork failed.
    pub error: Option<String>,
}

impl Auditioned {
    pub fn second_pass(&self) -> bool {
        self.forks.iter().any(|f| f.kind == Kind::Engaged)
    }
}

/// What the audition decided, and what it measured (for the lab and the
/// run log; never a view).
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    /// Times the world was advanced [`SCAN_STEP_DAYS`] before the scan
    /// found enough candidates (0: as play begins).
    pub months_ahead: u32,
    /// The day play would begin (the world the forks were taken from).
    pub opens: mandate_core::SimDate,
    /// Candidates the scan found (before K).
    pub found: usize,
    /// The K auditioned, in the prior's order.
    pub auditioned: Vec<Auditioned>,
    /// The openings offered: indices into `auditioned`, best first.
    pub openings: Vec<usize>,
    pub forks: u32,
    /// Days simulated across every fork.
    pub fork_days: u64,
}

impl Draft {
    pub fn offered(&self) -> impl Iterator<Item = &Auditioned> {
        self.openings.iter().filter_map(|i| self.auditioned.get(*i))
    }
}

/// An audition under way: the scan done, the forks played one a step.
pub struct Audition {
    content: Arc<ContentBundle>,
    /// The world the forks are taken from (the real state, or its future
    /// after a forward scan).
    base: State,
    params: Params,
    months_ahead: u32,
    found: usize,
    auditioned: Vec<Auditioned>,
    /// Forks still to play: (candidate, stand-in).
    queue: std::collections::VecDeque<(usize, Kind)>,
    second_planned: bool,
}

impl Audition {
    /// Scan the world (advancing a copy of it when too few candidates are
    /// found) and plan the first pass. `state` is only read.
    pub fn begin(
        state: &State,
        content: &Arc<ContentBundle>,
        params: Params,
    ) -> Result<Self, String> {
        let mut base: Option<Engine> = None;
        let mut months_ahead = 0;
        let (found, kept) = loop {
            let st = base.as_ref().map_or(state, |e| e.state());
            let found = scan(st, content, &params.recent);
            let n = found.len();
            let kept = keep(found, params.candidates);
            if kept.len() >= params.draft || months_ahead >= params.scan_months {
                break (n, kept);
            }
            let e = match base.as_mut() {
                Some(e) => e,
                None => base.insert(
                    Engine::from_state(state.clone(), content.clone())
                        .map_err(|x| x.to_string())?,
                ),
            };
            e.apply(Command::AdvanceDays {
                days: SCAN_STEP_DAYS,
            })
            .map_err(|x| x.to_string())?;
            months_ahead += 1;
        };
        let queue = (0..kept.len()).map(|i| (i, Kind::Passive)).collect();
        Ok(Audition {
            content: content.clone(),
            base: base.map_or_else(|| state.clone(), |e| e.state().clone()),
            params,
            months_ahead,
            found,
            auditioned: kept
                .into_iter()
                .map(|candidate| Auditioned {
                    candidate,
                    forks: Vec::new(),
                    score: None,
                    error: None,
                })
                .collect(),
            queue,
            second_planned: false,
        })
    }

    /// The candidates kept, before any fork is played.
    pub fn candidates(&self) -> impl Iterator<Item = &Candidate> {
        self.auditioned.iter().map(|a| &a.candidate)
    }

    /// Forks left to play (the second pass's are counted once planned).
    pub fn remaining(&self) -> usize {
        self.queue.len()
    }

    pub fn months_ahead(&self) -> u32 {
        self.months_ahead
    }

    /// Play one fork. Returns whether any remain.
    pub fn step(&mut self) -> bool {
        if self.queue.is_empty() && !self.second_planned {
            self.plan_second();
        }
        let Some((i, kind)) = self.queue.pop_front() else {
            return false;
        };
        let a = &mut self.auditioned[i];
        if a.error.is_none() {
            match play(
                &self.base,
                &self.content,
                &a.candidate,
                kind,
                &Seated,
                self.params.horizon_weeks,
            ) {
                Ok(f) => a.forks.push(f),
                Err(e) => a.error = Some(e),
            }
        }
        if self.queue.is_empty() && !self.second_planned {
            self.plan_second();
        }
        !self.queue.is_empty()
    }

    /// The second pass: engaged and contrarian for the best by E × V × P
    /// on the passive run (ties to the prior's order).
    fn plan_second(&mut self) {
        self.second_planned = true;
        let mut order: Vec<(usize, f64)> = self
            .auditioned
            .iter()
            .enumerate()
            .filter(|(_, a)| a.error.is_none())
            .filter_map(|(i, a)| {
                let def = self.content.arc(&a.candidate.template);
                Score::of(&a.candidate, def, &a.forks).map(|s| (i, s.first_pass))
            })
            .collect();
        order.sort_by(|x, y| y.1.total_cmp(&x.1).then(x.0.cmp(&y.0)));
        for (i, _) in order.into_iter().take(self.params.second_pass) {
            self.queue.push_back((i, Kind::Engaged));
            self.queue.push_back((i, Kind::Contrarian));
        }
    }

    /// Score every candidate and choose the draft.
    pub fn finish(mut self) -> Draft {
        while self.step() {}
        for a in &mut self.auditioned {
            let def = self.content.arc(&a.candidate.template);
            a.score = Score::of(&a.candidate, def, &a.forks);
        }
        let openings = choose(&self.auditioned, &self.params);
        let forks = self.auditioned.iter().map(|a| a.forks.len() as u32).sum();
        let fork_days = self
            .auditioned
            .iter()
            .flat_map(|a| &a.forks)
            .map(|f| u64::from(f.days))
            .sum();
        Draft {
            months_ahead: self.months_ahead,
            opens: self.base.calendar.date,
            found: self.found,
            auditioned: self.auditioned,
            openings,
            forks,
            fork_days,
        }
    }
}

/// The draft (§6.6): the best by score among the candidates played by all
/// three stand-ins that escalate (and meet the floors, if they are
/// enforced), no two of one template while other templates qualify (then
/// the best of the rest); then, if none of them is non-material and one
/// qualifies, the best such takes the last place.
pub fn choose(all: &[Auditioned], params: &Params) -> Vec<usize> {
    let mut eligible: Vec<(usize, &Score)> = all
        .iter()
        .enumerate()
        .filter(|(_, a)| a.second_pass())
        .filter_map(|(i, a)| a.score.as_ref().map(|s| (i, s)))
        .filter(|(_, s)| s.escalates && (!params.enforce_floors || s.floors.met()))
        .collect();
    eligible.sort_by(|x, y| y.1.total.total_cmp(&x.1.total).then(x.0.cmp(&y.0)));
    let mut out: Vec<usize> = Vec::new();
    for (i, _) in &eligible {
        if out.len() >= params.draft {
            break;
        }
        let t = &all[*i].candidate.template;
        if !out.iter().any(|j| all[*j].candidate.template == *t) {
            out.push(*i);
        }
    }
    // Too few templates qualify: the best of the rest fill the draft (each
    // in its own town, as every candidate kept is).
    for (i, _) in &eligible {
        if out.len() >= params.draft {
            break;
        }
        if !out.contains(i) {
            out.push(*i);
        }
    }
    let has_standing = out.iter().any(|i| !all[*i].candidate.material);
    if !has_standing
        && out.len() == params.draft
        && let Some((i, _)) = eligible.iter().find(|(i, _)| {
            !all[*i].candidate.material
                && !out[..out.len() - 1]
                    .iter()
                    .any(|j| all[*j].candidate.template == all[*i].candidate.template)
        })
    {
        let last = out.len() - 1;
        out[last] = *i;
    }
    out
}

/// The whole audition at once.
pub fn audition(
    state: &State,
    content: &Arc<ContentBundle>,
    params: Params,
) -> Result<Draft, String> {
    Ok(Audition::begin(state, content, params)?.finish())
}

/// The draft as a digest of what it measured (the lab's and the parity
/// tests' check that two runs agree to the last score).
pub fn digest(d: &Draft) -> String {
    let mut parts: BTreeMap<String, String> = BTreeMap::new();
    parts.insert("months_ahead".into(), d.months_ahead.to_string());
    parts.insert("openings".into(), format!("{:?}", d.openings));
    for (i, a) in d.auditioned.iter().enumerate() {
        let forks: Vec<&str> = a.forks.iter().map(|f| f.state_hash.as_str()).collect();
        parts.insert(
            format!("{i:02}"),
            format!(
                "{}@{}:{:?}:{}",
                a.candidate.template,
                a.candidate.town.0,
                forks,
                a.score.as_ref().map_or(0.0, |s| s.total)
            ),
        );
    }
    let s = format!("{parts:?}");
    mandate_core::hash::StableHash::of_bytes(s.as_bytes()).hex()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(town: u32, template: &str, index: usize, material: bool, variety: f64) -> Candidate {
        Candidate {
            at: Jurisdiction::town(MunicipalityId(town)),
            town: MunicipalityId(town),
            template: template.into(),
            index,
            title_key: format!("{template}-title"),
            situation: Situation::Holds,
            args: Args::new(),
            post: None,
            opening: Opening {
                template: template.into(),
                matter: "none".into(),
                post: "none".into(),
                town_class: "town".into(),
            },
            material,
            variety,
            heat: if material { 0.3 } else { 0.2 },
            luck: 1,
        }
    }

    #[test]
    fn keep_takes_one_a_town_and_two_a_template_by_the_prior() {
        let all = vec![
            cand(1, "arc.budget", 0, false, 1.0),
            cand(1, "arc.flood", 1, true, 1.0),
            cand(2, "arc.flood", 1, true, 1.0),
            cand(3, "arc.flood", 1, true, 1.0),
            cand(4, "arc.budget", 0, false, 1.0),
        ];
        let k = keep(all, 8);
        let got: Vec<(u32, &str)> = k.iter().map(|c| (c.town.0, c.template.as_str())).collect();
        // Hotter first; no third flood; town 1 once.
        assert_eq!(
            got,
            vec![(1, "arc.flood"), (2, "arc.flood"), (4, "arc.budget")]
        );
        assert_eq!(keep(k, 1).len(), 1);
    }

    fn scored(c: Candidate, total: f64, escalates: bool, second: bool) -> Auditioned {
        let t = Trace::default();
        let fork = |kind| Fork {
            kind,
            trace: t.clone(),
            vector: Vector {
                jobs: 0.0,
                homes: 0.0,
                away: 0.0,
                gov_approval: 0.0,
                player_approval: 0.0,
                opinions: Vec::new(),
                treasury: 0.0,
            },
            horizon_turns: 0,
            days: 0,
            state_hash: String::new(),
        };
        let mut forks = vec![fork(Kind::Passive)];
        if second {
            forks.push(fork(Kind::Engaged));
            forks.push(fork(Kind::Contrarian));
        }
        let mut s = Score::of(&c, None, &forks).unwrap();
        s.total = total;
        s.escalates = escalates;
        Auditioned {
            candidate: c,
            forks,
            score: Some(s),
            error: None,
        }
    }

    #[test]
    fn the_draft_is_the_best_that_escalate_one_a_template_with_a_standing_arc() {
        let p = Params::for_size(SizeClass::Small);
        let all = vec![
            scored(cand(1, "arc.flood", 1, true, 1.0), 0.9, true, true),
            scored(cand(2, "arc.flood", 1, true, 1.0), 0.8, true, true),
            scored(cand(3, "arc.plant", 2, true, 1.0), 0.7, true, true),
            scored(cand(4, "arc.rival", 3, true, 1.0), 0.95, false, true),
            scored(cand(5, "arc.favour", 4, true, 1.0), 0.6, true, true),
            scored(cand(6, "arc.budget", 0, false, 1.0), 0.5, true, true),
            scored(cand(7, "arc.other", 5, true, 1.0), 0.99, true, false),
        ];
        // The rival never escalated; the other had no second pass; one
        // flood; the budget round takes the last place from the favour.
        assert_eq!(choose(&all, &p), vec![0, 2, 5]);
        // Without a standing arc that qualifies, the best three.
        assert_eq!(choose(&all[..5], &p), vec![0, 2, 4]);
    }

    #[test]
    fn escalation_needs_a_run_that_moved_or_heated() {
        let mut t = Trace::default();
        assert!(!escalates(None, &t));
        t.run = Some(1);
        t.peak_heat = 0.0;
        assert!(!escalates(None, &t));
        t.peak_heat = 0.2;
        assert!(escalates(None, &t));
    }
}
