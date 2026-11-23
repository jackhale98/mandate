//! The pacing cycle (ENGAGEMENT 4.3): build-up, peak, relax, and the climax fallback.

use super::*;

// --- The pacing cycle (ENGAGEMENT §4.3) ----------------------------------------------------

/// The climax fallback takes a template's second or third use, never a
/// fourth (ENGAGEMENT §4.1: at most three uses in five years).
pub(super) const FALLBACK_MAX_USES: u16 = 2;

/// The cooldown a fallback climax sets: the template's base cooldown, not a
/// grown one, so a borrowed use does not take it out of the rest of the
/// career.
pub(super) fn fallback_cooldown(
    _n: &NarrativeState,
    d: &DirectorParams,
    _key: &str,
    cooldown: u32,
) -> u32 {
    cooldown_days(d, cooldown, 1)
}

/// The fallback borrows the templates whose return the climax delays
/// least: the new cooldown's end against the one it was cooling to.
pub(super) fn least_delayed<'c>(
    n: &NarrativeState,
    d: &DirectorParams,
    date: SimDate,
    mut cands: Vec<Candidate<'c>>,
) -> Vec<Candidate<'c>> {
    let delay = |c: &Candidate<'_>| {
        let new = date.add_days(fallback_cooldown(n, d, &c.def.id, c.def.cooldown_days));
        let old = n.cooldowns.get(&c.def.id).copied().unwrap_or(date);
        old.days_until(new)
    };
    let Some(least) = cands.iter().map(delay).min() else {
        return cands;
    };
    cands.retain(|c| delay(c) == least);
    cands
}

/// Draw tags for phase lengths.
const TAG_BUILD_UP: u64 = 1;
const TAG_RELAX: u64 = 2;

/// What happened since the last weekly step that ends a relax early: the
/// player started something (introduced a bill, filed for office, accepted
/// a big ask), or a crisis began in a focus town.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Stir {
    pub started: bool,
    pub crisis: bool,
}

pub(super) fn stir(n: &mut NarrativeState, v: &View<'_>, out: &Outs<'_>, first: bool) -> Stir {
    let me = v.player().map(|p| p.politician.0);
    let last = n.seen.last_event;
    let mut started = false;
    for e in out.events.iter().rev() {
        if e.id.0 <= last {
            break;
        }
        if let EventKind::PolicyIntroduced { sponsor, .. } = &e.kind
            && Some(*sponsor) == me
        {
            started = true;
        }
        // Accepting a big ask starts something too (ENGAGEMENT §3.3).
        if let EventKind::AskAccepted { big: true, .. } = &e.kind {
            started = true;
        }
    }
    let filed = v.player().is_some_and(|p| p.filing.is_some());
    started |= filed && !n.seen.filed;
    n.seen.filed = filed;
    let crisis = !first
        && focus_towns(v).iter().any(|m| {
            v.pol.characters.towns.get(*m as usize).is_some_and(|t| {
                t.crises
                    .iter()
                    .any(|c| c.severity > 0.0 && !n.seen.crises.iter().any(|(k, _)| *k == c.kind))
            })
        });
    Stir { started, crisis }
}

/// The phase lengths of a kind of chapter.
pub fn lengths(d: &DirectorParams, kind: ChapterKind) -> &CycleLengths {
    match kind {
        ChapterKind::Session => &d.cycle.session,
        ChapterKind::Campaign => &d.cycle.campaign,
    }
}

/// A phase length: a keyed draw in `range` (days, inclusive), scaled by the
/// profile (frequency, not amplitude), at least a day.
pub fn span(seed: u64, chapter: u32, cycle: u32, tag: u64, range: [u32; 2], scale: f64) -> u32 {
    let lo = range[0].min(range[1]);
    let hi = range[1].max(lo);
    let u = mandate_core::rng::keyed(
        seed,
        Stream::Director,
        &[super::super::KEY_PHASE, chapter as u64, cycle as u64, tag],
    )
    .next_f64();
    let days = lo as f64 + (u * (hi - lo + 1) as f64).floor().min((hi - lo) as f64);
    ((days * scale).round() as u32).max(1)
}

/// End the phase in force (into the log) and begin another.
#[allow(clippy::too_many_arguments)]
fn begin(
    n: &mut NarrativeState,
    kind: PhaseKind,
    date: SimDate,
    until: Option<SimDate>,
    cycle: u32,
    climax: bool,
    eased: bool,
) {
    let chapter = n.chapter.as_ref().map(|c| c.index).unwrap_or(0);
    if let Some(mut old) = n.phase.take() {
        old.until = Some(date);
        n.phase_log.push(old);
        if n.phase_log.len() > super::super::PHASE_LOG_CAP {
            n.phase_log.remove(0);
        }
    }
    if kind == PhaseKind::Peak
        && let Some(ch) = n.chapter.as_mut()
    {
        ch.peaks = ch.peaks.saturating_add(1);
    }
    n.phase = Some(DirectorPhase {
        kind,
        since: date,
        until,
        cycle,
        chapter,
        climax,
        eased,
        decision: None,
        beats: 0,
        climax_fallback: false,
    });
}

/// What a step of the pacing cycle reads, beyond the narrative state: the
/// date, the draw seed, phase lengths and the climax window. (A plain value,
/// so the cycle can be tested without a world.)
#[derive(Clone, Debug)]
pub(crate) struct CycleCtx {
    pub date: SimDate,
    pub seed: u64,
    pub lens: CycleLengths,
    pub build_up_scale: f64,
    pub relax_scale: f64,
    pub setback_relax_mult: f64,
    pub climax_force_days: u32,
    /// The session's climax window, [first day, last day]; `None` in a
    /// campaign.
    pub window: Option<[SimDate; 2]>,
    /// The chapter still wants its climax.
    pub climax_open: bool,
    pub stir: Stir,
    pub kind_changed: bool,
}

/// Step the pacing cycle to today (several transitions may fall due at
/// once). The cycle runs across chapters; a change of kind of chapter clips
/// a long phase to the new kind's lengths.
pub(super) fn step_cycle(
    n: &mut NarrativeState,
    v: &View<'_>,
    p: &StorytellerDef,
    b: Budget,
    s: Stir,
    kind_changed: bool,
) {
    let d = &v.content().director;
    let Some(ch) = n.chapter.as_ref() else { return };
    let window = (ch.kind == ChapterKind::Session).then(|| {
        let total = ch.start.days_until(climax_horizon(v, ch)) as f64;
        let w = &d.tension.climax_window;
        [
            ch.start.add_days((total * w[0]).ceil() as u32),
            ch.start.add_days((total * w[1]).floor() as u32),
        ]
    });
    let ctx = CycleCtx {
        date: v.date(),
        seed: v.r.meta.seed,
        lens: lengths(d, ch.kind).clone(),
        build_up_scale: p.build_up_scale,
        relax_scale: p.relax_scale,
        setback_relax_mult: d.cycle.setback_relax_mult,
        climax_force_days: d.cycle.climax_force_days,
        window,
        climax_open: ch.kind == ChapterKind::Session
            && ch.climaxes < b.climax
            && ch.climax.is_none(),
        stir: s,
        kind_changed,
    };
    cycle_core(n, &ctx);
}

/// The pacing cycle's transitions (ENGAGEMENT §4.3):
/// - build-up, at its drawn length → peak (the climax peak when its
///   build-up ends inside the climax window, or easing off after a setback);
/// - peak, once its decision is answered or at `peak_max` → peak fade;
/// - peak fade, once nothing significant is pending → relax (the longer one
///   after the climax; × `setback_relax_mult` after a setback);
/// - relax, at its drawn length or early when the player starts something
///   or a crisis begins → the next cycle's build-up;
/// - a session with no climax within `climax_force_days` of its window's
///   end: the phase ends and the climax peak opens.
pub(crate) fn cycle_core(n: &mut NarrativeState, c: &CycleCtx) {
    let date = c.date;
    let Some(chapter) = n.chapter.as_ref().map(|ch| ch.index) else {
        return;
    };
    let lens = &c.lens;
    if n.phase.is_none() {
        let cycle = n.phase_log.last().map(|p| p.cycle + 1).unwrap_or(1);
        let len = span(
            c.seed,
            chapter,
            cycle,
            TAG_BUILD_UP,
            lens.build_up,
            c.build_up_scale,
        );
        begin(
            n,
            PhaseKind::BuildUp,
            date,
            Some(date.add_days(len)),
            cycle,
            false,
            false,
        );
    }
    if c.kind_changed
        && let Some(ph) = n.phase.as_mut()
    {
        let longest = |r: [u32; 2], k: f64| ((r[1] as f64 * k).round() as u32).max(1);
        let cap = match ph.kind {
            PhaseKind::BuildUp => Some(longest(lens.build_up, c.build_up_scale)),
            PhaseKind::Relax => Some(longest(
                lens.relax_after_climax.unwrap_or(lens.relax),
                c.relax_scale * c.setback_relax_mult,
            )),
            _ => None,
        };
        if let (Some(cap), Some(u)) = (cap, ph.until) {
            ph.until = Some(u.min(date.add_days(cap)));
        }
    }
    let in_window = |on: SimDate| c.window.is_some_and(|[a, b]| on >= a && on <= b);
    let force = c.climax_open
        && c.window
            .is_some_and(|[_, end]| date <= end && date.add_days(c.climax_force_days) >= end);
    for _ in 0..6 {
        let Some(ph) = n.phase.clone() else { return };
        if force && !(ph.kind == PhaseKind::Peak && ph.climax) {
            let cycle = ph.cycle + (ph.kind != PhaseKind::BuildUp) as u32;
            let eased = n.setback.is_some() || (ph.kind == PhaseKind::BuildUp && ph.eased);
            let until = Some(date.add_days(lens.peak_max));
            begin(n, PhaseKind::Peak, date, until, cycle, true, eased);
            continue;
        }
        let due = |u: Option<SimDate>| u.is_some_and(|u| date >= u);
        match ph.kind {
            PhaseKind::BuildUp => {
                if !due(ph.until) {
                    break;
                }
                let climax = c.climax_open && in_window(date);
                let eased = ph.eased || n.setback.is_some();
                let until = Some(date.add_days(lens.peak_max));
                begin(n, PhaseKind::Peak, date, until, ph.cycle, climax, eased);
            }
            PhaseKind::Peak => {
                let answered = ph.decision.is_some_and(|id| {
                    n.decision(id)
                        .is_none_or(|x| x.status != DecisionStatus::Pending)
                });
                if !answered && !due(ph.until) {
                    break;
                }
                let climax = ph.climax && ph.decision.is_some();
                begin(
                    n,
                    PhaseKind::PeakFade,
                    date,
                    None,
                    ph.cycle,
                    climax,
                    ph.eased,
                );
            }
            PhaseKind::PeakFade => {
                let waiting = n
                    .pending()
                    .any(|x| x.weight != Weight::Minor && x.about.is_none());
                if waiting {
                    break;
                }
                let range = if ph.climax {
                    lens.relax_after_climax.unwrap_or(lens.relax)
                } else {
                    lens.relax
                };
                let mut len = span(c.seed, chapter, ph.cycle, TAG_RELAX, range, c.relax_scale);
                // Easing off after a setback (RimWorld): a longer relax.
                let eased = n.setback.take().is_some();
                if eased {
                    len = ((len as f64 * c.setback_relax_mult).round() as u32).max(1);
                }
                let until = Some(date.add_days(len));
                begin(n, PhaseKind::Relax, date, until, ph.cycle, ph.climax, eased);
            }
            PhaseKind::Relax => {
                // It ends early when the player starts something (Booth's
                // relax ends when the survivors move on), or a crisis begins.
                let early = ph.since < date && (c.stir.started || c.stir.crisis);
                if !early && !due(ph.until) {
                    break;
                }
                let cycle = ph.cycle + 1;
                let len = span(
                    c.seed,
                    chapter,
                    cycle,
                    TAG_BUILD_UP,
                    lens.build_up,
                    c.build_up_scale,
                );
                let until = Some(date.add_days(len));
                begin(n, PhaseKind::BuildUp, date, until, cycle, false, ph.eased);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lens() -> CycleLengths {
        CycleLengths {
            build_up: [35, 60],
            peak_max: 14,
            relax: [21, 35],
            relax_after_climax: Some([30, 45]),
        }
    }

    /// A narrative in a session chapter from day 1000 to 1365, its climax
    /// window [1237, 1310], and a context on `date`.
    fn session(date: u32) -> (NarrativeState, CycleCtx) {
        let n = NarrativeState {
            chapter: Some(Chapter::new(
                1,
                ChapterKind::Session,
                SimDate(1000),
                SimDate(1365),
            )),
            ..Default::default()
        };
        let ctx = CycleCtx {
            date: SimDate(date),
            seed: 7,
            lens: lens(),
            build_up_scale: 1.0,
            relax_scale: 1.0,
            setback_relax_mult: 1.5,
            climax_force_days: 14,
            window: Some([SimDate(1237), SimDate(1310)]),
            climax_open: true,
            stir: Stir::default(),
            kind_changed: false,
        };
        (n, ctx)
    }

    fn phase(kind: PhaseKind, since: u32, until: Option<u32>) -> DirectorPhase {
        DirectorPhase {
            kind,
            since: SimDate(since),
            until: until.map(SimDate),
            cycle: 3,
            chapter: 1,
            climax: false,
            eased: false,
            decision: None,
            beats: 0,
            climax_fallback: false,
        }
    }

    #[test]
    fn the_climax_peak_is_forced_two_weeks_before_the_window_closes() {
        // A long build-up that would end after the window: 15 days before
        // its end nothing happens; 14 days before, the climax peak opens.
        let (mut n, mut c) = session(1295);
        n.phase = Some(phase(PhaseKind::BuildUp, 1260, Some(1330)));
        cycle_core(&mut n, &c);
        assert_eq!(n.phase.as_ref().unwrap().kind, PhaseKind::BuildUp);
        c.date = SimDate(1296);
        cycle_core(&mut n, &c);
        let p = n.phase.as_ref().unwrap();
        assert_eq!((p.kind, p.climax), (PhaseKind::Peak, true));
        assert_eq!(p.until, Some(SimDate(1310)));
        assert_eq!(n.chapter.as_ref().unwrap().peaks, 1);
        // A relax is cut short the same way, into a new cycle.
        let (mut n, c) = session(1300);
        n.phase = Some(phase(PhaseKind::Relax, 1290, Some(1320)));
        cycle_core(&mut n, &c);
        let p = n.phase.as_ref().unwrap();
        assert_eq!((p.kind, p.climax, p.cycle), (PhaseKind::Peak, true, 4));
    }

    #[test]
    fn a_build_up_that_ends_in_the_window_makes_the_climax_peak() {
        let (mut n, c) = session(1240);
        n.phase = Some(phase(PhaseKind::BuildUp, 1190, Some(1240)));
        cycle_core(&mut n, &c);
        let p = n.phase.as_ref().unwrap();
        assert_eq!((p.kind, p.climax), (PhaseKind::Peak, true));
        // Before the window: an ordinary peak.
        let (mut n, c) = session(1200);
        n.phase = Some(phase(PhaseKind::BuildUp, 1150, Some(1200)));
        cycle_core(&mut n, &c);
        let p = n.phase.as_ref().unwrap();
        assert_eq!((p.kind, p.climax), (PhaseKind::Peak, false));
    }

    #[test]
    fn a_setback_lengthens_the_next_relax_by_half() {
        let relax_len = |setback: bool| {
            let (mut n, mut c) = session(1100);
            c.climax_open = false;
            n.phase = Some(phase(PhaseKind::PeakFade, 1095, None));
            if setback {
                n.setback = Some(SimDate(1090));
            }
            cycle_core(&mut n, &c);
            let p = n.phase.clone().unwrap();
            assert_eq!(p.kind, PhaseKind::Relax);
            assert_eq!(p.eased, setback);
            assert!(n.setback.is_none(), "eased once");
            p.since.days_until(p.until.unwrap())
        };
        let plain = relax_len(false);
        assert!((21..=35).contains(&plain), "{plain}");
        assert_eq!(relax_len(true), (plain as f64 * 1.5).round() as u32);
    }

    #[test]
    fn the_peak_after_a_setback_draws_opportunities_first() {
        let (mut n, mut c) = session(1100);
        c.climax_open = false;
        n.phase = Some(phase(PhaseKind::BuildUp, 1050, Some(1100)));
        n.setback = Some(SimDate(1080));
        cycle_core(&mut n, &c);
        let p = n.phase.as_ref().unwrap();
        assert_eq!((p.kind, p.eased), (PhaseKind::Peak, true));
    }

    #[test]
    fn relax_ends_early_when_the_player_starts_something() {
        let (mut n, mut c) = session(1100);
        c.climax_open = false;
        n.phase = Some(phase(PhaseKind::Relax, 1093, Some(1120)));
        cycle_core(&mut n, &c);
        assert_eq!(n.phase.as_ref().unwrap().kind, PhaseKind::Relax);
        c.stir.started = true;
        cycle_core(&mut n, &c);
        let p = n.phase.as_ref().unwrap();
        assert_eq!((p.kind, p.cycle), (PhaseKind::BuildUp, 4));
        let log = n.phase_log.last().unwrap();
        assert_eq!(
            (log.kind, log.until),
            (PhaseKind::Relax, Some(SimDate(1100)))
        );
        // A crisis beginning ends it too; a relax that begins this very step
        // does not end on what came before it.
        let (mut n, mut c) = session(1100);
        c.climax_open = false;
        c.stir.crisis = true;
        n.phase = Some(phase(PhaseKind::PeakFade, 1090, None));
        cycle_core(&mut n, &c);
        assert_eq!(n.phase.as_ref().unwrap().kind, PhaseKind::Relax);
    }

    #[test]
    fn a_peak_lasts_until_its_decision_is_answered_or_two_weeks() {
        let (mut n, mut c) = session(1100);
        c.climax_open = false;
        n.phase = Some(phase(PhaseKind::Peak, 1093, Some(1107)));
        cycle_core(&mut n, &c);
        assert_eq!(n.phase.as_ref().unwrap().kind, PhaseKind::Peak);
        c.date = SimDate(1107);
        cycle_core(&mut n, &c);
        // Nothing pending: the fade passes straight into the relax.
        assert_eq!(n.phase.as_ref().unwrap().kind, PhaseKind::Relax);
        let kinds: Vec<_> = n.phase_log.iter().map(|p| p.kind).collect();
        assert_eq!(kinds, vec![PhaseKind::Peak, PhaseKind::PeakFade]);
    }

    #[test]
    fn phase_lengths_are_keyed_draws_scaled_by_the_profile() {
        let a = span(7, 1, 3, TAG_RELAX, [21, 35], 1.0);
        assert_eq!(a, span(7, 1, 3, TAG_RELAX, [21, 35], 1.0));
        assert!((21..=35).contains(&a));
        assert_eq!(
            span(7, 1, 3, TAG_RELAX, [21, 35], 1.3),
            (a as f64 * 1.3).round() as u32
        );
        let draws: std::collections::BTreeSet<u32> = (0..40)
            .map(|c| span(7, 1, c, TAG_BUILD_UP, [35, 60], 1.0))
            .collect();
        assert!(draws.len() > 10 && draws.iter().all(|x| (35..=60).contains(x)));
    }
}
