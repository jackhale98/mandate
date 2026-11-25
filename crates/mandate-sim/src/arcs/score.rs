//! The arc scorer (STAGED_ARCS §6.5): five terms and coverage, each a
//! plain measured quantity.
//!
//! A [`Watch`] follows one template at one jurisdiction through a played
//! game and leaves a [`Trace`]: the stages its run entered, what the
//! player was shown and offered, its heat, its exit. The terms are
//! computed from the traces of the stand-in players (`crate::standin`):
//!
//! | Term | Type | From |
//! |---|---|---|
//! | Agency A | [`Agency`] | the passive, engaged and contrarian runs |
//! | Escalation E | [`Escalation`] | the passive run |
//! | Visibility V | [`Visibility`] | one run's stage entries and exit |
//! | Pacing P | [`Pacing`] | the engaged run |
//! | Variety N | [`variety`] | the opening against the last three |
//!
//! Each term keeps the quantities it was measured from beside its value.
//! The numbers inside a term (0.6, 0.2, 0.2; three stages; turns 8 to 14;
//! a gap of three turns; week 39) are the design's own, written once here
//! as constants. Nothing is fitted, and there is no weighted total: the
//! weights of §6.5's table are the audition's (task A7) and the tuning
//! pass's. [`evp`] is the design's product for ordering candidates.
//!
//! Everything here reads; nothing writes state or draws.

use super::{ArcActor, ArcCastEntry, ArcId, beats};
use crate::events::EventKind;
use crate::politics::Post;
use crate::state::State;
use crate::world::{Area, Jurisdiction};
use mandate_content::ContentBundle;
use mandate_content::arcs::ArcDef;
use std::collections::{BTreeMap, BTreeSet};

/// Agency: the share of it that an exit the engaged player changed
/// carries, that the contrarian's carries, and that the worlds' distance
/// carries (§6.5).
pub const AGENCY_PASSIVE_EXIT: f64 = 0.6;
pub const AGENCY_CONTRARIAN_EXIT: f64 = 0.2;
pub const AGENCY_DISTANCE: f64 = 0.2;
/// Escalation: stages entered are counted up to this many (§6.5).
pub const ESCALATION_STAGES: u32 = 3;
/// Pacing: the turn is reached within these turns; no stretch without a
/// beat on offer is longer than this many turns (§6.5).
pub const PACING_TURN_WINDOW: (u32, u32) = (8, 14);
pub const PACING_MAX_GAP: u32 = 3;
/// The audition's horizon: an exit by this week; the worlds are compared
/// at it (§6.2, §6.5).
pub const HORIZON_WEEKS: u32 = 39;
/// Variety: an opening is compared with this many before it (§6.5).
pub const VARIETY_RECENT: usize = 3;

/// A stage a run entered (its exit is an entry too).
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub stage: String,
    /// Turns from the start of the watch.
    pub turn: u32,
    pub exit: bool,
    /// Something of it reached the player without their seeking it: a
    /// decision or an ask of it lay open (a Today item), a cast person
    /// moved (Today, the skip summary), a story ran (a `press` beat, or an
    /// exit with a headline).
    pub seen: bool,
}

/// What a watched run left to measure.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Trace {
    pub run: Option<u32>,
    /// Turns from the start of the watch to the raise (0 when it was
    /// running already).
    pub raised: Option<u32>,
    /// In order; the first is the stage the run was raised at (or stood at
    /// when the watch began).
    pub entries: Vec<Entry>,
    /// The exit reached, and after how many turns.
    pub exit: Option<(String, u32)>,
    /// After how many turns the run's first `climax` stage was entered.
    pub turned: Option<u32>,
    pub peak_heat: f32,
    /// Turns (from the start of the watch) at whose start a beat of the
    /// run was on offer to the player.
    pub offered: Vec<u32>,
    /// Turns in which the player took a beat (a visit, a reply, an answer
    /// of their own, an ask kept).
    pub taken: Vec<u32>,
    /// Turns watched while the run lived.
    pub turns: u32,
}

impl Trace {
    /// Non-exit stages entered.
    pub fn stages_entered(&self) -> u32 {
        self.entries.iter().filter(|e| !e.exit).count() as u32
    }

    pub fn exit_id(&self) -> Option<&str> {
        self.exit.as_ref().map(|x| x.0.as_str())
    }
}

/// Follows one template at one jurisdiction through a game. Call
/// [`Watch::observe`] once before the first turn and after every turn.
#[derive(Clone, Debug)]
pub struct Watch {
    template: String,
    at: Jurisdiction,
    cursor: u32,
    turn0: u32,
    /// A run that was live when the watch began, taken as raised then.
    began: bool,
    done: bool,
    pub trace: Trace,
}

impl Watch {
    pub fn new(state: &State, template: &str, at: Jurisdiction) -> Self {
        Watch {
            template: template.to_string(),
            at,
            cursor: state.events.iter().last().map_or(0, |e| e.id.0 + 1),
            turn0: state.calendar.turn,
            began: false,
            done: false,
            trace: Trace::default(),
        }
    }

    fn enter(&mut self, def: Option<&ArcDef>, stage: &str, turn: u32, exit: bool) {
        let sdef = def.and_then(|d| d.stage(stage));
        if sdef.is_some_and(|s| s.climax) && self.trace.turned.is_none() {
            self.trace.turned = Some(turn);
        }
        let headline = sdef
            .and_then(|s| s.exit.as_ref())
            .is_some_and(|x| x.headline_key.is_some());
        self.trace.entries.push(Entry {
            stage: stage.to_string(),
            turn,
            exit,
            seen: exit && headline,
        });
        if exit {
            self.trace.exit = Some((stage.to_string(), turn));
            self.done = true;
        }
    }

    /// Whether the watched run has reached its exit.
    pub fn ended(&self) -> bool {
        self.done
    }

    /// Read what happened since the last look.
    pub fn observe(&mut self, state: &State, content: &ContentBundle) {
        if self.done {
            return;
        }
        let turn = state.calendar.turn.saturating_sub(self.turn0);
        let def = content.arc(&self.template);
        // A run already live as the watch begins: its stage is entry 0.
        if !self.began {
            self.began = true;
            if let Some(r) = state.arcs.at(self.at).find(|r| r.template == self.template) {
                self.trace.run = Some(r.id.0);
                self.trace.raised = Some(0);
                let stage = r.stage.clone();
                self.enter(def, &stage, 0, false);
            }
        }
        let cursor = self.cursor;
        let mut last = cursor;
        for ev in state.events.iter().filter(|e| e.id.0 >= cursor) {
            last = ev.id.0 + 1;
            match &ev.kind {
                EventKind::ArcRaised { arc, template, at }
                    if self.trace.run.is_none() && *template == self.template && *at == self.at =>
                {
                    self.trace.run = Some(*arc);
                    self.trace.raised = Some(turn);
                }
                EventKind::ArcStage { arc, stage, .. } if Some(*arc) == self.trace.run => {
                    self.enter(def, stage, turn, false);
                }
                EventKind::ArcEnded { arc, exit, .. } if Some(*arc) == self.trace.run => {
                    self.enter(def, exit, turn, true);
                }
                EventKind::ArcBeat {
                    arc,
                    stage,
                    kind,
                    ok,
                    ..
                } if Some(*arc) == self.trace.run => {
                    match kind.as_str() {
                        "press" | "move" => {
                            if let Some(e) = self
                                .trace
                                .entries
                                .iter_mut()
                                .rev()
                                .find(|e| e.stage == *stage)
                            {
                                e.seen = true;
                            }
                        }
                        // An answer of the player's own, a reply that
                        // agreed, a visit, an ask kept: a beat taken in
                        // the turn that just ended.
                        _ if *ok => {
                            let t = turn.saturating_sub(1);
                            if self.trace.taken.last() != Some(&t) {
                                self.trace.taken.push(t);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        self.cursor = last;
        if self.done {
            return;
        }
        let Some(run) = self.trace.run.and_then(|id| state.arcs.run(ArcId(id))) else {
            return;
        };
        self.trace.turns = turn + 1;
        self.trace.peak_heat = self.trace.peak_heat.max(run.heat);
        // A decision or an ask of the stage lies open: a Today item.
        if !run.open.is_empty()
            && let Some(e) = self
                .trace
                .entries
                .iter_mut()
                .rev()
                .find(|e| e.stage == run.stage)
        {
            e.seen = true;
        }
        if beats::offers(state, content)
            .iter()
            .any(|o| Some(o.run.0) == self.trace.run)
        {
            self.trace.offered.push(turn);
        }
    }
}

/// **Agency**: did playing change where the matter went?
#[derive(Clone, Debug, PartialEq)]
pub struct Agency {
    /// The passive and the engaged runs reached different exits (one of
    /// them reaching none within the horizon counts as different).
    pub passive_engaged_differ: bool,
    /// The contrarian and the engaged runs reached different exits.
    pub contrarian_engaged_differ: bool,
    /// [`distance`] between the passive and the engaged worlds at the
    /// horizon, 0 to 1.
    pub distance: f64,
}

impl Agency {
    pub fn of(passive: &Trace, engaged: &Trace, contrarian: &Trace, distance: f64) -> Self {
        Agency {
            passive_engaged_differ: passive.exit_id() != engaged.exit_id(),
            contrarian_engaged_differ: contrarian.exit_id() != engaged.exit_id(),
            distance,
        }
    }

    /// 0.6 if the passive and engaged exits differ, 0.2 if the contrarian
    /// and engaged exits differ, 0.2 × the distance (§6.5).
    pub fn value(&self) -> f64 {
        AGENCY_PASSIVE_EXIT * f64::from(u8::from(self.passive_engaged_differ))
            + AGENCY_CONTRARIAN_EXIT * f64::from(u8::from(self.contrarian_engaged_differ))
            + AGENCY_DISTANCE * self.distance.clamp(0.0, 1.0)
    }
}

/// **Escalation**: does the matter build when nobody acts? Measured on
/// the passive run.
#[derive(Clone, Debug, PartialEq)]
pub struct Escalation {
    /// Stages entered (exits not counted).
    pub stages: u32,
    pub peak_heat: f32,
    /// The turn (a `climax` stage) was reached.
    pub turned: bool,
}

impl Escalation {
    pub fn of(passive: &Trace) -> Self {
        Escalation {
            stages: passive.stages_entered(),
            peak_heat: passive.peak_heat,
            turned: passive.turned.is_some(),
        }
    }

    /// Stages entered ÷ 3 (at most 1) × peak heat, halved if the turn was
    /// never reached (§6.5).
    pub fn value(&self) -> f64 {
        let stages = f64::from(self.stages.min(ESCALATION_STAGES)) / f64::from(ESCALATION_STAGES);
        let v = stages * f64::from(self.peak_heat);
        if self.turned { v } else { v / 2.0 }
    }
}

/// **Visibility**: the share of a run's stage entries and its exit that
/// produced something seen without seeking ([`Entry::seen`]). Two of the
/// design's four signs are measured: a Today item and a story. A place's
/// state waits for `place_state` (task A5a), and a worry in the seat's top
/// three is not tied to an arc yet.
#[derive(Clone, Debug, PartialEq)]
pub struct Visibility {
    pub entries: u32,
    pub seen: u32,
}

impl Visibility {
    pub fn of(t: &Trace) -> Self {
        Visibility {
            entries: t.entries.len() as u32,
            seen: t.entries.iter().filter(|e| e.seen).count() as u32,
        }
    }

    pub fn value(&self) -> f64 {
        if self.entries == 0 {
            0.0
        } else {
            f64::from(self.seen) / f64::from(self.entries)
        }
    }
}

/// **Pacing**: four checks on a run (the engaged one, in the design's
/// table).
#[derive(Clone, Debug, PartialEq)]
pub struct Pacing {
    /// A beat was on offer at the start of the first turn (the design's
    /// floor: a candidate without it is dropped).
    pub beat_on_turn_1: bool,
    /// After how many turns the turn was reached.
    pub turned: Option<u32>,
    /// The longest stretch of turns with no beat on offer, while it ran.
    pub longest_gap: u32,
    /// After how many turns it reached an exit.
    pub exit: Option<u32>,
    /// The horizon the exit is held to, in turns.
    pub horizon: u32,
}

impl Pacing {
    /// `horizon`: the turns in [`HORIZON_WEEKS`] at the game's turn length.
    pub fn of(t: &Trace, horizon: u32) -> Self {
        let end = t.exit.as_ref().map_or(t.turns, |x| x.1);
        let mut gap = 0;
        let mut last: Option<u32> = None;
        for &turn in t.offered.iter().chain(std::iter::once(&end)) {
            let from = last.map_or(0, |l| l + 1);
            gap = gap.max(turn.saturating_sub(from));
            last = Some(turn);
        }
        Pacing {
            beat_on_turn_1: t.offered.first() == Some(&0),
            turned: t.turned,
            longest_gap: gap,
            exit: t.exit.as_ref().map(|x| x.1),
            horizon,
        }
    }

    pub fn turn_in_window(&self) -> bool {
        self.turned
            .is_some_and(|t| (PACING_TURN_WINDOW.0..=PACING_TURN_WINDOW.1).contains(&t))
    }

    pub fn exit_by_horizon(&self) -> bool {
        self.exit.is_some_and(|t| t <= self.horizon)
    }

    /// The share of the four checks met: a beat on turn 1, the turn within
    /// turns 8 to 14, no gap over three turns, an exit by week 39. The
    /// design lists the four and not how they combine; an equal share
    /// each is the plainest reading.
    pub fn value(&self) -> f64 {
        let met = [
            self.beat_on_turn_1,
            self.turn_in_window(),
            self.longest_gap <= PACING_MAX_GAP,
            self.exit_by_horizon(),
        ];
        met.iter().filter(|x| **x).count() as f64 / met.len() as f64
    }
}

/// What an opening is compared by (§6.5: template, industry or hazard,
/// post, town class). Keys and ids, never names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opening {
    pub template: String,
    /// The hazard the arc is about, else the town's defining industry.
    pub matter: String,
    /// The post's role word (the constitution's).
    pub post: String,
    pub town_class: String,
}

/// The opening a run would be: its template, the town's matter, the seat's
/// post and the town's class.
pub fn opening(
    state: &State,
    content: &ContentBundle,
    def: &ArcDef,
    at: Jurisdiction,
    post: Option<Post>,
) -> Opening {
    use mandate_content::Condition;
    let hazard = def.raise.iter().find_map(|c| match c {
        Condition::Disaster {
            hazard: Some(h), ..
        } => Some(format!("{h:?}").to_lowercase()),
        _ => None,
    });
    let town = at.municipality();
    let industry = town
        .and_then(|m| state.politics.characters.towns.get(m.index()))
        .and_then(|t| t.industry.clone());
    let c = content.constitutions.get(&state.meta.constitution);
    Opening {
        template: def.id.clone(),
        matter: hazard.or(industry).unwrap_or_else(|| "none".into()),
        post: post
            .and_then(|p| c.and_then(|c| p.word(c)).map(str::to_string))
            .unwrap_or_else(|| "none".into()),
        town_class: town
            .and_then(|m| state.world.municipalities.get(m.index()))
            .map(|m| format!("{:?}", m.class).to_lowercase())
            .unwrap_or_else(|| format!("{:?}", at.tier()).to_lowercase()),
    }
}

/// **Variety**: how far an opening is from the last three: the share of
/// its four attributes that differ, averaged over them (1 with none
/// before it).
pub fn variety(o: &Opening, recent: &[Opening]) -> f64 {
    let last: Vec<&Opening> = recent.iter().rev().take(VARIETY_RECENT).collect();
    if last.is_empty() {
        return 1.0;
    }
    last.iter()
        .map(|r| {
            let differ = [
                o.template != r.template,
                o.matter != r.matter,
                o.post != r.post,
                o.town_class != r.town_class,
            ];
            differ.iter().filter(|x| **x).count() as f64 / differ.len() as f64
        })
        .sum::<f64>()
        / last.len() as f64
}

/// The design's product for ordering candidates after the first pass
/// (§6.4): E × V × P, no constants.
pub fn evp(e: &Escalation, v: &Visibility, p: &Pacing) -> f64 {
    e.value() * v.value() * p.value()
}

/// The fixed vector two worlds are compared over (§6.5). Elections stay
/// out of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Vector {
    /// People in work in the jurisdiction's towns. (The design's "jobs in
    /// the subject's sector" waits for the `subject` cast, task A5a.)
    pub jobs: f64,
    /// Sound homes: the towns' dwellings less those damaged and unrepaired.
    pub homes: f64,
    /// Its people staying elsewhere after a disaster.
    pub away: f64,
    /// Approval of the government in the arc's area (its first area role,
    /// else the jurisdiction), 0 to 1.
    pub gov_approval: f64,
    /// The player's approval among the seat's voters, −1 to 1.
    pub player_approval: f64,
    /// What each cast person thinks of the player, in cast order.
    pub opinions: Vec<f64>,
    /// The jurisdiction's treasury: cash less debt, in minor units.
    pub treasury: f64,
}

/// Read the vector for an arc's jurisdiction, cast and seat.
pub fn vector(
    state: &State,
    content: &ContentBundle,
    at: Jurisdiction,
    cast: &[ArcCastEntry],
    post: Option<Post>,
) -> Vector {
    let r = crate::scheduler::Reads::of_state(state, content);
    let world = &state.world;
    let hoods = at.hoods(world);
    let towns: BTreeSet<usize> = hoods
        .iter()
        .filter_map(|h| world.neighborhoods.get(*h))
        .map(|n| n.municipality.index())
        .collect();
    let econ = |f: &dyn Fn(&crate::economy::MuniEcon) -> f64| -> f64 {
        towns
            .iter()
            .filter_map(|m| state.economy.munis.get(*m))
            .map(f)
            .sum()
    };
    let whole = Area::In { at };
    let area = cast
        .iter()
        .find_map(|c| match c.actor {
            ArcActor::Area { area } => Some(area),
            _ => None,
        })
        .unwrap_or(whole);
    let mean = |hs: &[usize], f: &dyn Fn(&crate::population::Cohort) -> f32| -> f64 {
        let (mut acc, mut n) = (0.0, 0.0);
        for h in hs {
            for c in state
                .population
                .hoods
                .get(*h)
                .map(|x| x.cohorts.as_slice())
                .unwrap_or(&[])
            {
                acc += f64::from(f(c)) * f64::from(c.count);
                n += f64::from(c.count);
            }
        }
        if n > 0.0 { acc / n } else { 0.0 }
    };
    let seat = post
        .map(|p| p.hoods(world))
        .unwrap_or_else(|| hoods.clone());
    let me = state
        .player
        .as_ref()
        .and_then(|p| state.politics.get(p.politician))
        .map(|p| p.character);
    let opinions = cast
        .iter()
        .filter_map(|c| match c.actor {
            ArcActor::Person { id } => Some(id),
            _ => None,
        })
        .map(|id| {
            me.map_or(0.0, |m| {
                f64::from(crate::characters::opinion(&state.politics, id, m))
            })
        })
        .collect();
    Vector {
        jobs: econ(&|m| m.employed),
        homes: econ(&|m| m.housing.stock) - super::cast::damaged_homes(&r, whole) as f64,
        away: super::cast::away(&r, whole).0 as f64,
        gov_approval: mean(&area.hoods(world), &|c| c.gov_approval),
        player_approval: mean(&seat, &|c| c.player_approval),
        opinions,
        treasury: crate::finance::treasury(&state.finance, at)
            .map_or(0.0, |t| (t.cash.units() - t.debt.units()) as f64),
    }
}

/// The opinion scale's width (−100 to 100).
const OPINION_SPAN: f64 = 200.0;

/// How far apart two worlds are over the vector, 0 to 1: the mean of
/// seven parts, each 0 to 1 and free of any fitted scale. Counts and
/// money (jobs, homes, people away, the treasury) by their relative
/// difference `|a − b| ÷ max(|a|, |b|)`; approvals by their difference
/// over their range; the cast's opinions by the mean difference over the
/// opinion scale.
pub fn distance(a: &Vector, b: &Vector) -> f64 {
    let rel = |x: f64, y: f64| {
        let m = x.abs().max(y.abs());
        if m > 0.0 { (x - y).abs() / m } else { 0.0 }
    };
    let n = a.opinions.len().min(b.opinions.len());
    let opinions = if n == 0 {
        0.0
    } else {
        a.opinions
            .iter()
            .zip(&b.opinions)
            .map(|(x, y)| ((x - y).abs() / OPINION_SPAN).min(1.0))
            .sum::<f64>()
            / n as f64
    };
    let parts = [
        rel(a.jobs, b.jobs),
        rel(a.homes, b.homes),
        rel(a.away, b.away),
        (a.gov_approval - b.gov_approval).abs().min(1.0),
        ((a.player_approval - b.player_approval).abs() / 2.0).min(1.0),
        opinions,
        rel(a.treasury, b.treasury).min(1.0),
    ];
    parts.iter().sum::<f64>() / parts.len() as f64
}

/// **Coverage**: which players visited each stage and exit of a template.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Coverage {
    /// Stage id → who entered it (a stand-in's key), for every stage of
    /// the template that is not an exit, in template order.
    pub stages: Vec<(String, BTreeSet<String>)>,
    pub exits: Vec<(String, BTreeSet<String>)>,
}

impl Coverage {
    pub fn new(def: &ArcDef) -> Self {
        let mut c = Coverage::default();
        for s in &def.stages {
            let slot = (s.id.clone(), BTreeSet::new());
            if s.exit.is_some() {
                c.exits.push(slot);
            } else {
                c.stages.push(slot);
            }
        }
        c
    }

    /// Count a player's trace.
    pub fn add(&mut self, who: &str, t: &Trace) {
        for e in &t.entries {
            let list = if e.exit {
                &mut self.exits
            } else {
                &mut self.stages
            };
            if let Some(x) = list.iter_mut().find(|x| x.0 == e.stage) {
                x.1.insert(who.to_string());
            }
        }
    }

    pub fn unvisited_stages(&self) -> Vec<&str> {
        self.stages
            .iter()
            .filter(|x| x.1.is_empty())
            .map(|x| x.0.as_str())
            .collect()
    }

    pub fn unvisited_exits(&self) -> Vec<&str> {
        self.exits
            .iter()
            .filter(|x| x.1.is_empty())
            .map(|x| x.0.as_str())
            .collect()
    }

    pub fn complete(&self) -> bool {
        self.unvisited_stages().is_empty() && self.unvisited_exits().is_empty()
    }

    /// Visited ÷ all, over stages and exits together.
    pub fn share(&self) -> f64 {
        let all = self.stages.len() + self.exits.len();
        let seen = self
            .stages
            .iter()
            .chain(&self.exits)
            .filter(|x| !x.1.is_empty())
            .count();
        if all == 0 {
            1.0
        } else {
            seen as f64 / all as f64
        }
    }
}

/// Whether a template could be raised at a jurisdiction now: its `raise`
/// holds there, and its cast can be bound (the doctor's `arc-cast-fails`:
/// a cause the world shows that finds nobody to tell it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Probe {
    pub raise: bool,
    /// Meaningful only with `raise`.
    pub cast: bool,
}

/// Read [`Probe`] for a template at a jurisdiction. It evaluates what the
/// step evaluates and changes nothing.
pub fn probe(state: &State, content: &ContentBundle, def: &ArcDef, at: Jurisdiction) -> Probe {
    crate::condition::on_state(state, content, |v| {
        let cx = crate::condition::Ctx::new(v, None, Some(at));
        let raise = crate::condition::all_hold(&cx, &def.raise, &mut crate::condition::Args::new());
        Probe {
            raise,
            cast: raise && super::cast::cast(v, at, def).is_some(),
        }
    })
}

/// How often each exit was reached, as shares of the runs that reached
/// one (the grader's "no exit takes over 70% of engaged runs").
pub fn exit_shares(traces: &[&Trace]) -> BTreeMap<String, f64> {
    let mut n: BTreeMap<String, f64> = BTreeMap::new();
    let mut total = 0.0;
    for t in traces {
        if let Some(x) = t.exit_id() {
            *n.entry(x.to_string()).or_default() += 1.0;
            total += 1.0;
        }
    }
    if total > 0.0 {
        for v in n.values_mut() {
            *v /= total;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(stage: &str, turn: u32, exit: bool, seen: bool) -> Entry {
        Entry {
            stage: stage.into(),
            turn,
            exit,
            seen,
        }
    }

    fn trace(exit: Option<&str>) -> Trace {
        let mut t = Trace {
            entries: vec![entry("a", 0, false, true), entry("b", 9, false, false)],
            turned: Some(9),
            peak_heat: 0.5,
            offered: vec![0, 1, 2, 5, 9],
            turns: 12,
            ..Trace::default()
        };
        if let Some(x) = exit {
            t.entries.push(entry(x, 12, true, true));
            t.exit = Some((x.into(), 12));
        }
        t
    }

    #[test]
    fn the_terms_are_the_designs_formulas() {
        let (p, e, c) = (trace(Some("x")), trace(Some("y")), trace(Some("y")));
        let a = Agency::of(&p, &e, &c, 0.5);
        assert!(a.passive_engaged_differ && !a.contrarian_engaged_differ);
        assert!((a.value() - (0.6 + 0.2 * 0.5)).abs() < 1e-12);
        let same = Agency::of(&p, &p, &p, 0.0);
        assert_eq!(same.value(), 0.0);
        // Two stages of three, peak heat 0.5, turned.
        let esc = Escalation::of(&p);
        assert!((esc.value() - (2.0 / 3.0) * 0.5).abs() < 1e-6);
        let mut never = p.clone();
        never.turned = None;
        assert!((Escalation::of(&never).value() - (2.0 / 3.0) * 0.25).abs() < 1e-6);
        // Three entries, two seen.
        let vis = Visibility::of(&p);
        assert_eq!((vis.entries, vis.seen), (3, 2));
        // A beat on turn 1, the turn at 9 (in 8..=14), an exit by the
        // horizon, and a gap of three turns (6, 7, 8): all four.
        let pace = Pacing::of(&p, 39);
        assert_eq!(pace.longest_gap, 3);
        assert_eq!(pace.value(), 1.0);
        let mut late = p.clone();
        late.offered = vec![1, 9];
        late.turned = Some(20);
        let pace = Pacing::of(&late, 10);
        assert!(!pace.beat_on_turn_1 && !pace.turn_in_window() && !pace.exit_by_horizon());
        assert_eq!(pace.longest_gap, 7);
        assert_eq!(pace.value(), 0.0);
        assert!((evp(&esc, &vis, &Pacing::of(&p, 39)) - esc.value() * vis.value()).abs() < 1e-12);
    }

    #[test]
    fn variety_is_the_share_of_attributes_that_differ() {
        let o = |t: &str, m: &str| Opening {
            template: t.into(),
            matter: m.into(),
            post: "p".into(),
            town_class: "town".into(),
        };
        assert_eq!(variety(&o("a", "x"), &[]), 1.0);
        assert_eq!(variety(&o("a", "x"), &[o("a", "x")]), 0.0);
        assert_eq!(variety(&o("a", "x"), &[o("b", "y")]), 0.5);
        // Only the last three count.
        let recent = [o("b", "y"), o("a", "x"), o("a", "x"), o("a", "x")];
        assert_eq!(variety(&o("a", "x"), &recent), 0.0);
    }

    #[test]
    fn distance_is_zero_for_one_world_and_scale_free() {
        let v = |jobs: f64, away: f64, op: f64| Vector {
            jobs,
            homes: 1000.0,
            away,
            gov_approval: 0.5,
            player_approval: 0.0,
            opinions: vec![op],
            treasury: -50.0,
        };
        assert_eq!(distance(&v(10.0, 5.0, 3.0), &v(10.0, 5.0, 3.0)), 0.0);
        // People away halve, an opinion moves 20 of 200: (0.5 + 0.1) / 7.
        let d = distance(&v(10.0, 10.0, 0.0), &v(10.0, 5.0, 20.0));
        assert!((d - 0.6 / 7.0).abs() < 1e-12);
        // The same worlds in other units are as far apart.
        let d2 = distance(&v(1e6, 1e4, 0.0), &v(1e6, 5e3, 20.0));
        assert!((d - d2).abs() < 1e-12);
    }
}
