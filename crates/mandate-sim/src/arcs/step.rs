//! The arcs' step (STAGED_ARCS §4.2): raise and cast new runs in the focus
//! jurisdictions; for each run in id order, apply the stage's rise and the
//! heat floor, settle what its decisions and asks came to, offer the
//! stage's beats (a decision, an ask, a story, a cast person's move), then
//! take the first edge that holds. It reads through `Reads`, writes
//! [`ArcsState`], and posts effect requests and events.
//!
//! A decision beat is asked of the director by listing it in the run's
//! `open`: the director presents it within its budgets
//! (`narrative::offered`), and the run reads the answer from the decision
//! that names it. An ask beat is a request to the asks (`OfferAsk`).
//!
//! It runs early in the day (the NPC actions phase), so everything a run
//! asks of another domain is applied by that domain's own system the same
//! day, and nothing waits outside the state between advances.

use super::effects::{self, post};
use super::{
    ArcCooldown, ArcId, ArcRef, ArcRun, ArcTurn, ArcsState, BeatRef, EXIT_REMOVED, beat_of, cast,
};
use crate::asks::AskStatus;
use crate::condition::{Args, Ctx};
use crate::events::{EventId, EventKind};
use crate::narrative::cast::View;
use crate::narrative::{CastEntry, DecisionStatus};
use crate::scheduler::{Effect, Outs, Reads, Request};
use crate::spine::{Cause, Link, PlayerAct, Process, Thread};
use crate::state::GamePhase;
use crate::world::Jurisdiction;
use mandate_content::Condition;
use mandate_content::arcs::{ArcBeat, ArcDef, BeatKind, GatedEffect, StageDef};
use mandate_core::{Cadence, SimDate, Stream};

/// The evaluator's context for a run: its jurisdiction, its people as
/// slots, its marks, heat and days. No whole state: the arcs' conditions
/// are decided inside the simulation.
pub(crate) fn context<'a>(v: &'a View<'a>, run: &'a ArcRun, people: &'a [CastEntry]) -> Ctx<'a> {
    let mut cx = Ctx::new(v, None, Some(run.at));
    cx.cast = people;
    cx.arc = Some(run);
    cx
}

/// Do all these conditions hold for the run now?
pub(crate) fn holds(v: &View<'_>, run: &ArcRun, people: &[CastEntry], cs: &[Condition]) -> bool {
    if cs.is_empty() {
        return true;
    }
    let cx = context(v, run, people);
    crate::condition::all_hold(&cx, cs, &mut Args::new())
}

/// The step runs while there is a player, play is on, and there is
/// something to do: arc content to raise, or a run in progress (one whose
/// template an update removed ends at its next step, even when the pack
/// has no arc left).
fn active(a: &ArcsState, r: &Reads<'_>) -> bool {
    (!r.content.arcs.is_empty() || !a.runs.is_empty())
        && r.player.is_some()
        && r.calendar.phase != GamePhase::BurnIn
}

/// Arcs domain, NPC actions phase, daily: requests held for a monthly
/// applier go out as the month turns, and a run whose clock runs out today
/// takes its edge today.
pub fn daily(a: &mut ArcsState, r: &Reads<'_>, out: &mut Outs<'_>) {
    if !a.deferred.is_empty() && Cadence::Monthly.hits(r.date) {
        for q in std::mem::take(&mut a.deferred) {
            out.effects.push_request(q);
        }
    }
    if !active(a, r) || Cadence::Weekly.hits(r.date) {
        return;
    }
    if a.runs.iter().any(|x| x.clock == Some(r.date)) {
        run_all(a, r, out, false);
    }
}

/// Arcs domain, NPC actions phase, weekly.
pub fn weekly(a: &mut ArcsState, r: &Reads<'_>, out: &mut Outs<'_>) {
    if !active(a, r) {
        return;
    }
    run_all(a, r, out, true);
    let pol = r.politics();
    let v = View::new(r, pol);
    raise(a, &v, out);
    a.cooldowns.retain(|c| c.until > r.date);
}

fn run_all(a: &mut ArcsState, r: &Reads<'_>, out: &mut Outs<'_>, weekly: bool) {
    let pol = r.politics();
    let v = View::new(r, pol);
    let mut ended: Vec<(usize, String)> = Vec::new();
    for i in 0..a.runs.len() {
        if !weekly && a.runs[i].clock != Some(r.date) {
            continue;
        }
        let mut run = a.runs[i].clone();
        let exit = tick(&mut a.deferred, &mut run, &v, out, weekly);
        a.runs[i] = run;
        if let Some(x) = exit {
            ended.push((i, x));
        }
    }
    for (i, exit) in ended.into_iter().rev() {
        let run = a.runs.remove(i);
        end(a, r, run, &exit);
    }
}

/// A run has ended: its cooldown, and its summary.
fn end(a: &mut ArcsState, r: &Reads<'_>, run: ArcRun, exit: &str) {
    let days = r.content.arc(&run.template).map_or(0, |d| d.cooldown_days);
    if days > 0 {
        let c = ArcCooldown {
            template: run.template.clone(),
            at: run.at,
            until: r.date.add_days(days),
        };
        match a
            .cooldowns
            .binary_search_by(|x| (&x.template, x.at).cmp(&(&c.template, c.at)))
        {
            Ok(i) => a.cooldowns[i] = c,
            Err(i) => a.cooldowns.insert(i, c),
        }
    }
    a.retire(run, exit, r.date);
}

/// One run's turn. Returns the exit it reached, if it ended.
fn tick(
    deferred: &mut Vec<Request>,
    run: &mut ArcRun,
    v: &View<'_>,
    out: &mut Outs<'_>,
    weekly: bool,
) -> Option<String> {
    let content = v.content();
    // An update removed the template or the stage: the run ends there.
    let Some(def) = content.arc(&run.template) else {
        return Some(EXIT_REMOVED.to_string());
    };
    let Some(stage) = def.stage(&run.stage) else {
        return Some(EXIT_REMOVED.to_string());
    };
    if weekly {
        run.add_heat(stage.rise);
        if let Some(f) = &def.heat.floor
            && let Some(m) = run.at.municipality()
        {
            let floor = crate::concerns::indicator(v.r, m.index(), f.indicator) * f.scale;
            run.heat = run.heat.max(floor.clamp(0.0, 1.0) as f32);
        }
        settle(deferred, run, def, v, out);
        offer(deferred, run, def, stage, v, out);
    } else {
        // The clock runs out today: what was answered since the week's
        // step counts before the edge is taken.
        settle(deferred, run, def, v, out);
    }
    edges(deferred, run, def, v, out)
}

/// Take edges until none holds (a stage's edges are checked on entry too).
fn edges(
    deferred: &mut Vec<Request>,
    run: &mut ArcRun,
    def: &ArcDef,
    v: &View<'_>,
    out: &mut Outs<'_>,
) -> Option<String> {
    let today = v.date();
    let mut moved = false;
    for _ in 0..=def.stages.len() {
        let Some(stage) = def.stage(&run.stage) else {
            break;
        };
        let people = run.narrative_cast();
        let days = run.days_in_stage(today);
        let next = stage.next.iter().find(|e| {
            days >= e.after_days
                && e.clock_within_days
                    .is_none_or(|d| run.clock.is_some_and(|c| today.add_days(d) >= c))
                && holds(v, run, &people, &e.when)
        });
        let Some(to) = next.and_then(|e| def.stage(&e.to)) else {
            break;
        };
        if let Some(exit) = enter(deferred, run, def, to, v, out) {
            return Some(exit);
        }
        moved = true;
    }
    // The stage the run came to rest in offers its beats the same day (a
    // stage passed through on the way offers none).
    if moved && let Some(stage) = def.stage(&run.stage) {
        offer(deferred, run, def, stage, v, out);
    }
    None
}

/// Enter a stage: its clock, its event with the facts as they are, its
/// entry effects. An exit ends the run: its event closes the thread and
/// its effects go to their owners.
fn enter(
    deferred: &mut Vec<Request>,
    run: &mut ArcRun,
    def: &ArcDef,
    stage: &StageDef,
    v: &View<'_>,
    out: &mut Outs<'_>,
) -> Option<String> {
    let today = v.date();
    run.stage = stage.id.clone();
    run.since = today;
    if stage.climax && run.turn.is_none() {
        run.turn = Some(ArcTurn {
            stage: stage.id.clone(),
            on: today,
        });
    }
    if let Some(c) = &stage.clock {
        let mut draw = mandate_core::rng::keyed(
            v.r.meta.seed,
            Stream::Misc,
            &[
                super::KEY_CLOCK,
                run.id.0 as u64,
                mandate_core::hash::derive_u64(&stage.id, &[]),
            ],
        );
        // The dates the content allows: so many days from today, so many
        // before the jurisdiction's budget year turns, or where the two
        // overlap (else the first day `days` allows).
        let by_days = c
            .days
            .map(|[lo, hi]| (today.add_days(lo), today.add_days(hi)));
        let by_year = c.before_budget_year.map(|[least, most]| {
            let turns = crate::finance::budget_year_turns(run.at, today);
            let hi = SimDate(turns.0.saturating_sub(least)).max(today.add_days(1));
            let lo = SimDate(turns.0.saturating_sub(most)).max(today.add_days(1));
            (lo.min(hi), hi)
        });
        let range = match (by_days, by_year) {
            (Some((a, b)), Some((c, d))) => {
                let (lo, hi) = (a.max(c), b.min(d));
                Some(if lo <= hi { (lo, hi) } else { (a, a) })
            }
            (x, y) => x.or(y),
        };
        match range {
            // A director-owned clock (DEC-033 B): inside the content's
            // range, and where the chapter wants its turn if it can be.
            Some((lo, hi)) if c.placed => {
                let lead = def
                    .stages
                    .iter()
                    .flat_map(|s| s.next.iter())
                    .filter(|e| def.stage(&e.to).is_some_and(|t| t.climax))
                    .filter_map(|e| e.clock_within_days)
                    .max()
                    .unwrap_or(0);
                let (date, placed) =
                    crate::narrative::director::place_clock(v, lo, hi, lead, draw.next_f64());
                run.clock = Some(date);
                if placed {
                    out.events.record(
                        today,
                        EventKind::DirectorIntervention {
                            lever: "arc_clock".into(),
                            municipality: run
                                .at
                                .municipality()
                                .map(|m| m.0)
                                .or(v.town())
                                .unwrap_or(0),
                        },
                        Cause::Process(Process::Director),
                    );
                }
            }
            Some((lo, hi)) => {
                let d = draw.range_i64(
                    i64::from(today.days_until(lo)),
                    i64::from(today.days_until(hi)),
                );
                run.clock = Some(today.add_days(d as u32));
            }
            None => {}
        }
    }
    let facts = cast::facts(v, def, run);
    let (kind, list): (EventKind, &[GatedEffect]) = match &stage.exit {
        None => (
            EventKind::ArcStage {
                arc: run.id.0,
                template: def.id.clone(),
                stage: stage.id.clone(),
                at: run.at,
                facts,
            },
            &stage.enter,
        ),
        Some(x) => (
            EventKind::ArcEnded {
                arc: run.id.0,
                template: def.id.clone(),
                exit: stage.id.clone(),
                at: run.at,
                facts,
            },
            &x.effects,
        ),
    };
    // The edge's rule fired; the log links it to the run's last event.
    let ev = out
        .events
        .record(today, kind, Cause::Process(Process::Arcs));
    let asked = effects::resolve(v, run, list);
    run.add_heat(asked.heat);
    post(out, deferred, asked.requests, Cause::caused_by(ev));
    stage.exit.as_ref().map(|_| stage.id.clone())
}

/// What doing a beat does to the run: an outcome of its own first (an
/// option's, a reply's: a mark, heat, effects), then, if the beat itself
/// was done (`full`), its mark, relief and effects. Returns the requests
/// for other domains.
pub(crate) fn complete(
    v: &View<'_>,
    run: &mut ArcRun,
    beat: &ArcBeat,
    own: Option<(Option<&str>, f32, &[GatedEffect])>,
    full: bool,
) -> Vec<Effect> {
    let mut requests = Vec::new();
    if let Some((sets, heat, list)) = own {
        if let Some(m) = sets {
            run.set_mark(m);
        }
        run.add_heat(heat);
        let asked = effects::resolve(v, run, list);
        run.add_heat(asked.heat);
        requests.extend(asked.requests);
    }
    if full {
        if let Some(m) = &beat.sets {
            run.set_mark(m);
        }
        run.add_heat(-beat.relief);
        let asked = effects::resolve(v, run, &beat.effects);
        run.add_heat(asked.heat);
        requests.extend(asked.requests);
    }
    requests
}

/// The event of a beat (it notes the run's thread), with the facts as
/// they are.
pub(crate) fn beat_event(
    v: &View<'_>,
    run: &ArcRun,
    def: &ArcDef,
    b: &BeatRef,
    kind: &str,
    ok: bool,
) -> EventKind {
    EventKind::ArcBeat {
        facts: cast::facts(v, def, run),
        arc: run.id.0,
        template: def.id.clone(),
        stage: b.stage.clone(),
        beat: b.beat.clone(),
        at: run.at,
        kind: kind.to_string(),
        ok,
    }
}

/// What the run's open decisions and asks came to.
fn settle(
    deferred: &mut Vec<Request>,
    run: &mut ArcRun,
    def: &ArcDef,
    v: &View<'_>,
    out: &mut Outs<'_>,
) {
    let today = v.date();
    for b in run.open.clone() {
        let Some((_, beat)) = beat_of(def, &b) else {
            run.open.retain(|x| x != &b);
            continue;
        };
        let aref = ArcRef {
            run: run.id,
            beat: b.clone(),
        };
        match &beat.kind {
            BeatKind::Decision { on, .. } => {
                let found =
                    v.r.narrative()
                        .decisions
                        .iter()
                        .find(|d| d.arc.as_ref() == Some(&aref));
                let Some(d) = found else {
                    // Not presented yet (the director's timing), or gone
                    // from the history: it stays asked while its stage
                    // lasts.
                    if run.stage != b.stage || !player_active(v) {
                        run.open.retain(|x| x != &b);
                    }
                    continue;
                };
                match d.status {
                    DecisionStatus::Pending => {}
                    DecisionStatus::Lapsed => run.mark_done(b, today),
                    DecisionStatus::Answered | DecisionStatus::Defaulted => {
                        let option = d
                            .resolution
                            .as_ref()
                            .map(|x| x.option.as_str())
                            .unwrap_or_default();
                        let answered = d.status == DecisionStatus::Answered;
                        let own = on
                            .get(option)
                            .map(|o| (o.sets.as_deref(), o.heat, o.effects.as_slice()));
                        // The answer (or the default: the player let it
                        // pass) led here.
                        let cause = Cause::event_or(
                            out.events.head(Thread::Decision(d.id.0)),
                            Link::Caused,
                            Cause::Player(if answered {
                                PlayerAct::Answer
                            } else {
                                PlayerAct::LetPass
                            }),
                        );
                        let ev = out.events.record(
                            today,
                            beat_event(v, run, def, &b, "decision", answered),
                            cause,
                        );
                        let reqs = complete(v, run, beat, own, answered);
                        run.mark_done(b, today);
                        post(out, deferred, reqs, Cause::caused_by(ev));
                    }
                }
            }
            BeatKind::Ask { .. } => {
                let found =
                    v.r.asks()
                        .asks
                        .iter()
                        .find(|x| x.arc.as_ref() == Some(&aref));
                let Some(ask) = found else {
                    reoffer(deferred, run, &b, beat, v, out);
                    continue;
                };
                let kept = match ask.status {
                    AskStatus::Offered | AskStatus::Accepted { .. } => continue,
                    AskStatus::Kept { .. } => true,
                    AskStatus::Broken { .. }
                    | AskStatus::Declined { .. }
                    | AskStatus::Void { .. } => false,
                };
                let cause = Cause::event_or(
                    out.events.head(Thread::Ask(ask.id.0)),
                    Link::Caused,
                    Cause::Process(Process::Asks),
                );
                let ev = out
                    .events
                    .record(today, beat_event(v, run, def, &b, "ask", kept), cause);
                let reqs = complete(v, run, beat, None, kept);
                run.mark_done(b, today);
                post(out, deferred, reqs, Cause::caused_by(ev));
            }
            _ => run.open.retain(|x| x != &b),
        }
    }
}

/// An open beat nobody took up: ask its owner again while its stage
/// lasts, else let it go.
fn reoffer(
    deferred: &mut Vec<Request>,
    run: &mut ArcRun,
    b: &BeatRef,
    beat: &ArcBeat,
    v: &View<'_>,
    out: &mut Outs<'_>,
) {
    if run.stage != b.stage || !player_active(v) {
        run.open.retain(|x| x != b);
        return;
    }
    if let Some(e) = request_of(run, b, beat) {
        let cause = prompted(out, run.id);
        post(out, deferred, vec![e], cause);
    }
}

fn player_active(v: &View<'_>) -> bool {
    v.player().is_some_and(|p| p.status.active())
}

/// The run's latest event prompted this (a decision, an ask).
fn prompted(out: &Outs<'_>, run: ArcId) -> Cause {
    Cause::event_or(
        out.events.head(Thread::Arc(run.0)),
        Link::Prompted,
        Cause::Process(Process::Arcs),
    )
}

/// The request that offers an ask beat with the run's cast.
fn request_of(run: &ArcRun, b: &BeatRef, beat: &ArcBeat) -> Option<Effect> {
    let arc = ArcRef {
        run: run.id,
        beat: b.clone(),
    };
    match &beat.kind {
        BeatKind::Ask { template, asker } => Some(Effect::OfferAsk {
            template: template.clone(),
            asker: run.person(asker)?,
            cast: run.narrative_cast(),
            arc,
        }),
        _ => None,
    }
}

/// The roles a beat needs are cast (an optional role may be empty).
pub(crate) fn roles_cast(run: &ArcRun, beat: &ArcBeat) -> bool {
    let role = match &beat.kind {
        BeatKind::Ask { asker, .. } => asker,
        BeatKind::Talk { with, .. } => with,
        BeatKind::Visit { at, .. } => at,
        BeatKind::Move { actor, .. } => actor,
        BeatKind::Decision { .. } | BeatKind::Press { .. } | BeatKind::Heard { .. } => return true,
    };
    run.actor(role).is_some()
}

/// A beat is on offer: of the current stage, not done (unless it repeats),
/// its roles cast and its conditions holding.
pub(crate) fn available(
    v: &View<'_>,
    run: &ArcRun,
    people: &[CastEntry],
    stage: &StageDef,
    beat: &ArcBeat,
) -> bool {
    let b = BeatRef::new(&stage.id, &beat.id);
    run.stage == stage.id
        && !(beat.once && run.is_done(&b))
        && roles_cast(run, beat)
        && holds(v, run, people, &beat.when)
}

/// The chance a cast person makes their move this week: the beat's weight,
/// more for the bold, less for someone who thinks well of the player (a
/// cultivated ally moves later; an enemy moves first).
pub fn move_chance(v: &View<'_>, run: &ArcRun, actor: &str, weight: f32) -> f64 {
    let Some(id) = run.person(actor) else {
        return 0.0;
    };
    let bold = crate::characters::profile(v.pol, v.r.world, id)
        .map_or(0.5, |p| f64::from(p.personality.boldness));
    let opinion = v.me().map_or(0.0, |me| {
        f64::from(crate::characters::opinion(v.pol, id, me))
    });
    (f64::from(weight) * (0.5 + bold) * (1.0 - opinion / 200.0)).clamp(0.0, 1.0)
}

/// Offer the stage's beats: open its decisions and asks, run its stories,
/// draw its moves. A visit, a choice or an overheard line is there for the
/// player while the stage lasts; nothing is done about it here.
fn offer(
    deferred: &mut Vec<Request>,
    run: &mut ArcRun,
    def: &ArcDef,
    stage: &StageDef,
    v: &View<'_>,
    out: &mut Outs<'_>,
) {
    let today = v.date();
    let people = run.narrative_cast();
    for beat in &stage.beats {
        let b = BeatRef::new(&stage.id, &beat.id);
        if run.is_done(&b) || run.open.contains(&b) || !available(v, run, &people, stage, beat) {
            continue;
        }
        match &beat.kind {
            // Asked of the director, who presents it (`narrative::offered`).
            BeatKind::Decision { .. } => {
                if player_active(v) {
                    run.open.push(b);
                }
            }
            BeatKind::Ask { .. } => {
                // Nothing is put to a player who cannot act (prison).
                if !player_active(v) {
                    continue;
                }
                if let Some(e) = request_of(run, &b, beat) {
                    let cause = prompted(out, run.id);
                    post(out, deferred, vec![e], cause);
                    run.open.push(b);
                }
            }
            BeatKind::Press { .. } => {
                let ev = out.events.record(
                    today,
                    beat_event(v, run, def, &b, "press", true),
                    Cause::Process(Process::Arcs),
                );
                let reqs = complete(v, run, beat, None, true);
                run.mark_done(b, today);
                post(out, deferred, reqs, Cause::caused_by(ev));
            }
            BeatKind::Move { actor, weight, .. } => {
                let p = move_chance(v, run, actor, *weight);
                let fires = mandate_core::rng::keyed(
                    v.r.meta.seed,
                    Stream::Misc,
                    &[
                        super::KEY_MOVE,
                        run.id.0 as u64,
                        mandate_core::hash::derive_u64(&b.key(), &[]),
                        today.0 as u64,
                    ],
                )
                .chance(p);
                if !fires {
                    continue;
                }
                // Someone chose to (their own agency).
                let cause = run
                    .person(actor)
                    .map_or(Cause::Process(Process::Arcs), Cause::Actor);
                let ev = out
                    .events
                    .record(today, beat_event(v, run, def, &b, "move", true), cause);
                let reqs = complete(v, run, beat, None, true);
                run.mark_done(b, today);
                post(out, deferred, reqs, Cause::caused_by(ev));
            }
            BeatKind::Talk { .. } | BeatKind::Visit { .. } | BeatKind::Heard { .. } => {}
        }
    }
}

/// The event that made a template's `raise` true at a jurisdiction: the
/// latest disaster (of the hazard) or plant closure there that one of its
/// conditions reads. `None` when its conditions read no event.
fn root_event(
    log: &crate::events::EventLog,
    at: Jurisdiction,
    raise: &[Condition],
    today: SimDate,
) -> Option<(EventId, SimDate)> {
    let town = at.municipality()?.0;
    let mut all = Vec::new();
    mandate_content::condition::walk(raise, &mut all);
    let mut best: Option<(EventId, SimDate)> = None;
    for c in all {
        let (months, hazard, closure) = match c {
            Condition::Disaster {
                within_months,
                hazard,
            } => (*within_months, hazard.map(cast::hazard_kind), false),
            Condition::PlantClosure { within_months } => (*within_months, None, true),
            _ => continue,
        };
        let days = u32::from(months) * 31;
        let found = log
            .iter()
            .rev()
            .take_while(|e| e.date.days_until(today) <= days || e.date > today)
            .find(|e| match &e.kind {
                EventKind::Disaster {
                    municipality,
                    hazard: h,
                    ..
                } => !closure && *municipality == town && hazard.is_none_or(|k| k == *h),
                EventKind::PlantClosure { municipality, .. } => closure && *municipality == town,
                _ => false,
            })
            .map(|e| (e.id, e.date));
        if let Some(f) = found
            && best.is_none_or(|b| f.0 > b.0)
        {
            best = Some(f);
        }
    }
    best
}

/// Days a template whose `raise` reads no event waits before it may be
/// raised again, whatever its cooldown.
const MIN_COOLDOWN_DAYS: u32 = 28;

/// Raise and cast new runs in the focus jurisdictions, as soon as a
/// template's `raise` holds: one run a template in a jurisdiction, and at
/// most `director.arcs_active` there, standing arcs aside (the world does
/// not wait, but nobody follows a third matter at once: its owning system
/// resolves it by its own rule).
fn raise(a: &mut ArcsState, v: &View<'_>, out: &mut Outs<'_>) {
    let Some(pl) = v.player() else { return };
    let content = v.content();
    let today = v.date();
    let began = v.play_began();
    let career = if today >= began {
        began.days_until(today)
    } else {
        0
    };
    let cap = usize::from(content.director.arcs_active);
    for at in pl.focus() {
        for def in &content.arcs {
            if !def.tiers.contains(&at.tier()) || career < def.min_career_days {
                continue;
            }
            if a.at(at).any(|x| x.template == def.id) {
                continue;
            }
            let others = a
                .at(at)
                .filter(|x| {
                    content
                        .arc(&x.template)
                        .is_none_or(|d| d.standing.is_none())
                })
                .count();
            if def.standing.is_none() && others >= cap {
                continue;
            }
            if a.cooldowns
                .iter()
                .any(|c| c.template == def.id && c.at == at && c.until > today)
            {
                continue;
            }
            let cx = Ctx::new(v, None, Some(at));
            if !crate::condition::all_hold(&cx, &def.raise, &mut Args::new()) {
                continue;
            }
            // Parsed and validated, never raised (STAGED_ARCS §9).
            if !super::not_yet(def).is_empty() {
                continue;
            }
            let root = root_event(out.events, at, &def.raise, today);
            let last = a
                .past
                .iter()
                .rev()
                .find(|p| p.template == def.id && p.at == at);
            // One run a matter: the same event raises nothing twice, and
            // a template that reads no event waits a while.
            if last.is_some_and(|p| match root {
                Some((id, _)) => p.root == Some(id),
                None => p.ended.days_until(today) < MIN_COOLDOWN_DAYS,
            }) {
                continue;
            }
            let Some(cast) = cast::cast(v, at, def) else {
                continue;
            };
            let Some(first) = def.stages.first() else {
                continue;
            };
            let id = ArcId(a.ids.peek_next());
            let mut run = ArcRun {
                id,
                template: def.id.clone(),
                at,
                base: cast::base_readings(v.r, &cast),
                cast,
                stage: first.id.clone(),
                since: today,
                started: today,
                began: root.map_or(today, |r| r.1),
                clock: None,
                heat: def.heat.start.clamp(0.0, 1.0),
                marks: Vec::new(),
                done: Vec::new(),
                open: Vec::new(),
                root: root.map(|r| r.0),
                turn: None,
                before: a.last_turn(&def.id, at).map(String::from),
            };
            // A matter too small to tell raises nothing (`given`).
            if !holds(v, &run, &run.narrative_cast(), &def.given) {
                continue;
            }
            a.ids.alloc_raw();
            // The event that made it true is the arc's root on the spine.
            out.events.record(
                today,
                EventKind::ArcRaised {
                    arc: id.0,
                    template: def.id.clone(),
                    at,
                },
                match root {
                    Some((ev, _)) => Cause::caused_by(ev),
                    None => Cause::Process(Process::Arcs),
                },
            );
            // An arc raised mid-matter starts where the world is: the
            // first stage's edges are checked on entry, and the stage it
            // rests in offers its beats the same day.
            let mut exit = enter(&mut a.deferred, &mut run, def, first, v, out);
            if exit.is_none() {
                exit = edges(&mut a.deferred, &mut run, def, v, out);
                if exit.is_none() && run.stage == first.id {
                    offer(&mut a.deferred, &mut run, def, first, v, out);
                }
            }
            match exit {
                Some(x) => end(a, v.r, run, &x),
                None => a.runs.push(run),
            }
        }
    }
}
