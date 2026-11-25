//! What a line does (EXCHANGE_SCENES §5): **one function**, called by the
//! preview and by the command. Pure arithmetic over numbers already read
//! from state; nothing here reads the world, draws, or knows what the
//! player knows.
//!
//! ```text
//! bite(g)   = B · size(P) · fit(P, g) · cold(r)
//! cover     = C[k] · strength'(M)          if M answers P   (own, deny, pass: C[k] alone)
//! gain(g)   = G · voice · strength'(M) · fit(M, g) · reach(M, g) · temper
//! Δlean(g)  = − bite(g) · (1 − h · cover)  +  h · gain(g)  −  (1 − h) · F[k] · reach(M, g)
//! ```
//!
//! A range (what the player sees of a size that turns on something they
//! have not learned) is the same formula at the ends of what the unknown
//! input can be. Δ is linear in each of the prompt's fit, the line's fit
//! and the answer's size, so its extremes are at the corners.

use mandate_content::exchange::{CountNeeds, ExchangeParams, MoveKind, Outcome};

/// A number the formula reads, with the ends it could take as far as the
/// player knows (`lo == hi == value` when they know it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Known {
    pub value: f64,
    pub lo: f64,
    pub hi: f64,
}

impl Known {
    pub fn exact(value: f64) -> Self {
        Known {
            value,
            lo: value,
            hi: value,
        }
    }

    pub fn between(value: f64, lo: f64, hi: f64) -> Self {
        Known {
            value,
            lo: lo.min(value),
            hi: hi.max(value),
        }
    }

    pub fn is_exact(&self) -> bool {
        self.lo == self.hi
    }

    /// With everything known.
    pub fn truth(&self) -> Self {
        Known::exact(self.value)
    }
}

/// The round as the formula sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct RoundIn {
    /// The prompt's size, 0..1.
    pub size: f64,
    /// `cold` in a cold first round, else 1.
    pub cold: f64,
    /// `voice_base + voice_span × oratory`.
    pub voice: f64,
    /// `1 − drag × heat` where the scene has a temper, else 1.
    pub temper: f64,
    /// The prompt stood up. One that did not turns back on whoever made it.
    pub stood: bool,
    /// How much each group cares about the prompt's subject.
    pub fit: Vec<Known>,
    /// Each group's lean now (a lean is kept in [−1, 1]).
    pub lean: Vec<f64>,
}

/// A line as the formula sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct LineIn {
    pub kind: MoveKind,
    /// 0..1, after `repeat`, before the other side's answer.
    pub strength: f64,
    /// The chance it stands (1: no chance).
    pub p: f64,
    /// It answers the prompt.
    pub answers: bool,
    /// The size of the answer the other side gives (0: none), with what
    /// the player can know of it.
    pub answer: Known,
    /// How much each group cares about the line's subject.
    pub fit: Vec<Known>,
    /// 1 for the group a targeted line names (or for all, for the room),
    /// `spill` for the others.
    pub reach: Vec<f64>,
}

/// What a line does to each group: if it stands, and if it does not.
#[derive(Clone, Debug, PartialEq)]
pub struct Landing {
    pub p: f64,
    pub stands: Vec<f64>,
    pub fails: Vec<f64>,
}

/// A change as far as the player can know it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub lo: f64,
    pub hi: f64,
}

impl Span {
    pub fn exact(&self) -> bool {
        self.lo == self.hi
    }

    pub fn contains(&self, x: f64) -> bool {
        x >= self.lo - 1e-9 && x <= self.hi + 1e-9
    }

    pub fn mid(&self) -> f64 {
        (self.lo + self.hi) / 2.0
    }
}

/// A landing as the player sees it before choosing.
#[derive(Clone, Debug, PartialEq)]
pub struct Preview {
    pub p: f64,
    pub stands: Vec<Span>,
    pub fails: Vec<Span>,
}

/// The bite a prompt leaves on one group before any answer: how hard it
/// pulls them from the player. Negative when the prompt did not stand.
pub fn bite(p: &ExchangeParams, r: &RoundIn, fit: f64) -> f64 {
    if r.stood {
        p.bite * r.size * fit * r.cold
    } else {
        -p.backfire(MoveKind::Turn) * fit
    }
}

/// The change in one group's lean, before the lean's own bounds.
#[allow(clippy::too_many_arguments)]
fn raw(
    p: &ExchangeParams,
    r: &RoundIn,
    line: &LineIn,
    fit_p: f64,
    fit_m: f64,
    answer: f64,
    reach: f64,
    stands: bool,
) -> f64 {
    let b = bite(p, r, fit_p);
    if !stands {
        return -b - p.backfire(line.kind) * reach;
    }
    let strength = line.strength * (1.0 - answer.clamp(0.0, 1.0));
    let cover = if !line.answers || b <= 0.0 {
        0.0
    } else if line.kind.always_answers() {
        p.cover(line.kind)
    } else {
        p.cover(line.kind) * strength
    }
    .clamp(0.0, 1.0);
    let gain = p.gain * r.voice * strength * fit_m * reach * r.temper;
    -b * (1.0 - cover) + gain
}

/// A lean moved by `delta`, kept in [−1, 1]: the change that lands.
pub fn bounded(lean: f64, delta: f64) -> f64 {
    (lean + delta).clamp(-1.0, 1.0) - lean
}

/// The one function (§5.7): what the line does to each group, standing
/// and not. The command applies it; the preview is this at the ends of
/// what the player does not know.
pub fn land(p: &ExchangeParams, r: &RoundIn, line: &LineIn) -> Landing {
    let n = r.lean.len();
    let at = |g: usize, stands: bool| {
        bounded(
            r.lean[g],
            raw(
                p,
                r,
                line,
                r.fit[g].value,
                line.fit[g].value,
                line.answer.value,
                line.reach[g],
                stands,
            ),
        )
    };
    Landing {
        p: line.p,
        stands: (0..n).map(|g| at(g, true)).collect(),
        fails: (0..n).map(|g| at(g, false)).collect(),
    }
}

/// The landing as far as the player knows its inputs: for each group, the
/// least and the most the formula gives over the ends of the prompt's
/// fit, the line's fit and the other side's answer. With everything known
/// it is the landing itself, to the bit.
pub fn preview(p: &ExchangeParams, r: &RoundIn, line: &LineIn) -> Preview {
    let n = r.lean.len();
    let span = |g: usize, stands: bool| {
        let (fp, fm, an) = (r.fit[g], line.fit[g], line.answer);
        if fp.is_exact() && fm.is_exact() && an.is_exact() {
            let x = bounded(
                r.lean[g],
                raw(p, r, line, fp.value, fm.value, an.value, line.reach[g], stands),
            );
            return Span { lo: x, hi: x };
        }
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for a in [fp.lo, fp.hi] {
            for b in [fm.lo, fm.hi] {
                for c in [an.lo, an.hi] {
                    let x = bounded(
                        r.lean[g],
                        raw(p, r, line, a, b, c, line.reach[g], stands),
                    );
                    lo = lo.min(x);
                    hi = hi.max(x);
                }
            }
        }
        Span { lo, hi }
    };
    Preview {
        p: line.p,
        stands: (0..n).map(|g| span(g, true)).collect(),
        fails: (0..n).map(|g| span(g, false)).collect(),
    }
}

/// The line with everything known: its preview is its landing.
pub fn known(r: &RoundIn, line: &LineIn) -> (RoundIn, LineIn) {
    let mut r = r.clone();
    let mut line = line.clone();
    r.fit.iter_mut().for_each(|f| *f = f.truth());
    line.fit.iter_mut().for_each(|f| *f = f.truth());
    line.answer = line.answer.truth();
    (r, line)
}

/// What a round with no line does (an empty chair, a round forfeited):
/// the prompt bites in full.
pub fn unanswered(p: &ExchangeParams, r: &RoundIn) -> Vec<f64> {
    (0..r.lean.len())
        .map(|g| bounded(r.lean[g], -bite(p, r, r.fit[g].value)))
        .collect()
}

/// The expected change by weight: what the choice of lines ranks by
/// (§3.3, step 2), on the engine's true numbers.
pub fn score(l: &Landing, weights: &[f64]) -> f64 {
    weights
        .iter()
        .enumerate()
        .map(|(g, w)| w * (l.p * l.stands[g] + (1.0 - l.p) * l.fails[g]))
        .sum()
}

/// A group as the result reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ended {
    pub weight: f64,
    pub start: f64,
    pub lean: f64,
    /// A member or a person (a head, where the scene counts heads).
    pub head: bool,
}

/// The result (§5.2): `D = Σ weight × change`, won at `win`, lost at
/// `−lose`. With a count, by heads against `member_line`. A scene the
/// player never began is `absent` whatever the sum.
pub fn outcome(
    p: &ExchangeParams,
    groups: &[Ended],
    count: Option<CountNeeds>,
) -> (Outcome, f64) {
    let d: f64 = groups.iter().map(|g| g.weight * (g.lean - g.start)).sum();
    if let Some(needs) = count {
        let heads: Vec<&Ended> = groups.iter().filter(|g| g.head).collect();
        if !heads.is_empty() {
            let with = heads.iter().filter(|g| g.lean >= p.member_line).count();
            let against = heads.iter().filter(|g| g.lean <= -p.member_line).count();
            let out = if needs.met(with, heads.len()) {
                Outcome::Won
            } else if needs.met(against, heads.len()) {
                Outcome::Lost
            } else {
                Outcome::Split
            };
            return (out, d);
        }
    }
    let out = if d >= p.win {
        Outcome::Won
    } else if d <= -p.lose {
        Outcome::Lost
    } else {
        Outcome::Split
    };
    (out, d)
}

/// How one group came out of it: won, lost or neither (`group_margin`).
pub fn group_result(p: &ExchangeParams, start: f64, lean: f64) -> i8 {
    let d = lean - start;
    if d >= p.group_margin {
        1
    } else if d <= -p.group_margin {
        -1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> ExchangeParams {
        ExchangeParams::default()
    }

    fn round(size: f64, fit: &[f64], lean: &[f64]) -> RoundIn {
        RoundIn {
            size,
            cold: 1.0,
            voice: 1.0,
            temper: 1.0,
            stood: true,
            fit: fit.iter().map(|f| Known::exact(*f)).collect(),
            lean: lean.to_vec(),
        }
    }

    fn line(kind: MoveKind, strength: f64, answers: bool, fit: &[f64], reach: &[f64]) -> LineIn {
        LineIn {
            kind,
            strength,
            p: 1.0,
            answers,
            answer: Known::exact(0.0),
            fit: fit.iter().map(|f| Known::exact(*f)).collect(),
            reach: reach.to_vec(),
        }
    }

    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 0.0006, "{a} is not {b}");
    }

    /// The design's worked debate, round 1 (§6.1): a broken promise, size
    /// 0.6, no worry as its subject.
    #[test]
    fn round_one_of_the_worked_debate() {
        let p = p();
        let r = round(0.6, &[0.6, 0.6, 0.6], &[0.10, -0.05, 0.0]);
        // Own it: the bite of 0.108 covered by six tenths.
        let own = land(&p, &r, &line(MoveKind::Own, 0.0, true, &[0.6; 3], &[1.0; 3]));
        near(own.stands[0], -0.043);
        // Move on.
        let pass = land(&p, &r, &line(MoveKind::Pass, 0.0, true, &[0.6; 3], &[1.0; 3]));
        near(pass.stands[1], -0.086);
        // The hardship fund, 0.7 on jobs: top in the first, low in the
        // second, next in the third. It changes the subject.
        let record = land(
            &p,
            &r,
            &line(MoveKind::Record, 0.7, false, &[1.0, 0.25, 0.6], &[1.0; 3]),
        );
        near(record.stands[0], 0.067);
        near(record.stands[1], -0.064);
        near(record.stands[2], -0.003);
        // What was heard in the first: strength 0.8, a little elsewhere.
        let heard = land(
            &p,
            &r,
            &line(MoveKind::Heard, 0.8, false, &[1.0, 0.25, 0.6], &[1.0, 0.3, 0.3]),
        );
        near(heard.stands[0], 0.092);
        near(heard.stands[1], -0.093);
        near(heard.stands[2], -0.072);
    }

    /// Round 2: a vote on taxes, size 0.5; a promise at half strength that
    /// answers it.
    #[test]
    fn round_two_a_promise_that_answers() {
        let p = p();
        let r = round(0.5, &[0.25, 1.0, 0.25], &[0.057, -0.093, -0.043]);
        let promise = land(
            &p,
            &r,
            &line(MoveKind::Promise, 0.5, true, &[0.25, 1.0, 0.25], &[1.0; 3]),
        );
        near(promise.stands[1], 0.0125);
        near(promise.stands[0], 0.003);
        // The same promise met by "another promise": half as strong.
        let mut answered = line(MoveKind::Promise, 0.5, true, &[0.25, 1.0, 0.25], &[1.0; 3]);
        answered.answer = Known::exact(0.5);
        let l = land(&p, &r, &answered);
        near(l.stands[1], -0.150 * (1.0 - 0.125) + 0.25 * 0.25);
        assert!(l.stands[1] < promise.stands[1]);
    }

    /// Round 3: the room's question on jobs, answered with the record.
    #[test]
    fn round_three_the_record_answers_the_question() {
        let p = p();
        let r = round(0.3, &[1.0, 0.25, 0.6], &[0.06, -0.081, -0.04]);
        let record = land(
            &p,
            &r,
            &line(MoveKind::Record, 0.7, true, &[1.0, 0.25, 0.6], &[1.0; 3]),
        );
        near(record.stands[0], 0.135);
        near(record.stands[1], 0.034);
        near(record.stands[2], 0.081);
    }

    #[test]
    fn a_cold_first_round_bites_harder() {
        let p = p();
        let mut r = round(0.6, &[0.6], &[0.0]);
        r.cold = p.cold;
        near(unanswered(&p, &r)[0], -0.135);
        r.cold = 1.0;
        near(unanswered(&p, &r)[0], -0.108);
    }

    #[test]
    fn a_line_that_fails_gains_nothing_and_backfires() {
        let p = p();
        let r = round(0.5, &[0.6], &[0.0]);
        let mut turn = line(MoveKind::Turn, 0.8, false, &[1.0], &[1.0]);
        turn.p = 0.4;
        let l = land(&p, &r, &turn);
        near(l.stands[0], -0.09 + 0.2);
        near(l.fails[0], -0.09 - 0.25);
        near(score(&l, &[1.0]), 0.4 * 0.11 + 0.6 * -0.34);
    }

    #[test]
    fn a_hot_room_hears_less() {
        let p = p();
        let mut r = round(0.3, &[1.0], &[0.0]);
        let l = line(MoveKind::Promise, 1.0, false, &[1.0], &[1.0]);
        let cool = land(&p, &r, &l).stands[0];
        r.temper = 1.0 - 0.5 * 0.8;
        let hot = land(&p, &r, &l).stands[0];
        near(cool - hot, 0.25 * 0.4);
    }

    #[test]
    fn a_prompt_that_did_not_stand_turns_back() {
        let p = p();
        let mut r = round(0.8, &[1.0], &[0.0]);
        r.stood = false;
        // No bite, and the room moves to the player whatever they say.
        let pass = land(&p, &r, &line(MoveKind::Pass, 0.0, true, &[1.0], &[1.0]));
        near(pass.stands[0], 0.25);
    }

    #[test]
    fn a_lean_stays_in_bounds_and_the_landing_says_so() {
        let p = p();
        let r = round(1.0, &[1.0], &[-0.95]);
        let l = land(&p, &r, &line(MoveKind::Pass, 0.0, true, &[1.0], &[1.0]));
        near(l.stands[0], -0.05);
    }

    #[test]
    fn with_everything_known_the_preview_is_the_landing() {
        let p = p();
        let r = round(0.5, &[0.25, 1.0], &[0.2, -0.3]);
        let l = line(MoveKind::Record, 0.7, true, &[1.0, 0.6], &[1.0, 1.0]);
        let landing = land(&p, &r, &l);
        let pre = preview(&p, &r, &l);
        for g in 0..2 {
            assert_eq!(pre.stands[g].lo.to_bits(), landing.stands[g].to_bits());
            assert_eq!(pre.stands[g].hi.to_bits(), landing.stands[g].to_bits());
            assert_eq!(pre.fails[g].lo.to_bits(), landing.fails[g].to_bits());
        }
    }

    /// The truth is inside the range, for every corner of what the player
    /// might not know (a grid, not a draw: the formula is linear in each).
    #[test]
    fn the_truth_is_always_inside_the_range() {
        let p = p();
        let fits = [0.25, 0.6, 1.0];
        let answers = [0.0, 0.3, 0.5, 1.0];
        for kind in MoveKind::ALL {
            for &fp in &fits {
                for &fm in &fits {
                    for &an in &answers {
                        for answers_prompt in [false, true] {
                            for lean in [-0.99, -0.2, 0.0, 0.5, 0.99] {
                                let mut r = round(0.6, &[fp], &[lean]);
                                r.fit[0] = Known::between(fp, 0.25, 1.0);
                                let mut l = line(kind, 0.7, answers_prompt, &[fm], &[1.0]);
                                l.fit[0] = Known::between(fm, 0.25, 1.0);
                                l.answer = Known::between(an, 0.0, 1.0);
                                l.p = 0.5;
                                let (kr, kl) = known(&r, &l);
                                let truth = land(&p, &kr, &kl);
                                let seen = preview(&p, &r, &l);
                                assert!(
                                    seen.stands[0].contains(truth.stands[0]),
                                    "{kind:?} {fp} {fm} {an} {lean}"
                                );
                                assert!(seen.fails[0].contains(truth.fails[0]));
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_result_is_a_weighted_sum_or_a_count_of_heads() {
        let p = p();
        let g = |weight: f64, start: f64, lean: f64, head: bool| Ended {
            weight,
            start,
            lean,
            head,
        };
        // The worked debate's end: 0.052, under `win`.
        let crowd = [
            g(0.45, 0.10, 0.195, false),
            g(0.35, -0.05, -0.047, false),
            g(0.20, 0.0, 0.041, false),
        ];
        let (out, d) = outcome(&p, &crowd, None);
        near(d, 0.052);
        assert_eq!(out, Outcome::Split);
        assert_eq!(group_result(&p, 0.10, 0.195), 1);
        assert_eq!(group_result(&p, -0.05, -0.047), 0);
        // Six heads: four at or over the line clear a majority.
        let heads: Vec<Ended> = [0.5, 0.3, 0.2, 0.25, -0.4, -0.1]
            .iter()
            .map(|l| g(1.0 / 6.0, 0.0, *l, true))
            .collect();
        assert_eq!(outcome(&p, &heads, Some(CountNeeds::Majority)).0, Outcome::Won);
        let heads: Vec<Ended> = [0.5, 0.3, 0.2, -0.25, -0.4, -0.3]
            .iter()
            .map(|l| g(1.0 / 6.0, 0.0, *l, true))
            .collect();
        assert_eq!(
            outcome(&p, &heads, Some(CountNeeds::Majority)).0,
            Outcome::Split
        );
    }
}
