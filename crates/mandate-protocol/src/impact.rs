//! What a bill does (DEC-111, `docs/design/IMPACT_ESTIMATE.md` §6, §9): the
//! engine's impact estimate as facts, Fluent keys and typed arguments. The
//! engine decides what is shown and how it is rounded; the UI formats
//! numbers and renders the keys, and computes nothing.
//!
//! This build shows the estimate at full knowledge (the lab range, to two
//! significant figures). Knowledge levels, sources and claims (the design's
//! §4) gate and widen these ranges here, in this crate, when they are built.

use mandate_sim::policy::PolicyInstance;
use mandate_sim::policy::impact::{DirectLine, Estimate, HORIZONS, OutcomeLine, Range, Unit};
use mandate_sim::{Engine, State};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// One line of an estimate: an outcome or a direct count at one horizon.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImpactLine {
    /// `homes` | `jobs` | `sector_jobs` | `rent` | `low_income` | `debt` |
    /// `services` | `approval` | `approval_low` | `approval_middle` |
    /// `approval_high`, or `target` for a direct count.
    pub outcome: String,
    /// `up` | `down` | `flat` (no clear change); `up` for a direct count.
    pub direction: String,
    pub horizon_months: u32,
    /// The horizon falls before the next election of the body voting on it.
    pub in_time: bool,
    /// `exact` (a direct count: `lo` alone), `about` (`lo` alone), `range`
    /// (`lo` to `hi`), or `words` (no number: no clear change, or a range
    /// whose far end is over three times its near one).
    pub form: String,
    /// Sizes, never signed: the near and the far end of the range in the
    /// direction stated. Rounded by the engine; the UI only formats them.
    pub lo: Option<f64>,
    pub hi: Option<f64>,
    /// `count` | `percent` | `points` | `money`
    pub unit: String,
    /// Fluent key of the whole sentence, with `form`, `lo` and `hi` (the
    /// formatted sizes) and `args`. An argument named `<x>_key` is itself
    /// a Fluent key, rendered first and passed as `<x>` (DEC-054).
    pub key: String,
    pub args: BTreeMap<String, String>,
    /// One of the few lines the bill's card shows (at the horizon in time
    /// for the election); the rest are for the full sheet.
    pub lead: bool,
    /// A cost of the bill beside its purpose.
    pub tradeoff: bool,
}

/// A bill's money: exact, never gated (DEC-091).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CostView {
    /// Yearly cost in full (nominal currency).
    pub yearly: f64,
    /// That cost as a percentage of the jurisdiction's yearly budget.
    pub budget_pct: f64,
    /// `rates` | `borrowing` | `levy` | `squeeze`
    pub paid_from: String,
    /// Funded until the next budget year, and from it on (per cent).
    pub funded_first_pct: u32,
    pub funded_later_pct: u32,
    /// Share of the cost borrowed (per cent).
    pub borrowed_pct: u32,
    /// `impact-cost` (with `amount`, `share`) or `impact-cost-none`.
    pub key: String,
    /// `impact-paid-<paid_from>` (with `funded`, `borrowed`); none for a
    /// bill that costs nothing.
    pub paid_key: Option<String>,
}

/// A group that gains or loses by the bill.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GroupImpact {
    /// `low` | `middle` | `high` | `older` | `area`
    pub group: String,
    pub gains: bool,
    /// `impact-gains-<group>` or `impact-loses-<group>`.
    pub key: String,
}

/// A sentence the engine states: a Fluent key and its arguments.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImpactText {
    pub key: String,
    pub args: BTreeMap<String, String>,
}

/// What a bill will probably do (the label on screen: "What it does").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ImpactView {
    pub at: crate::post::JurisdictionDto,
    pub cost: CostView,
    /// Every outcome and direct count at 12, 36 and 60 months; the card's
    /// lines (`lead`) first.
    pub lines: Vec<ImpactLine>,
    /// Who gains and who loses: at most two each way.
    pub groups: Vec<GroupImpact>,
    /// When it starts and when it is in full effect.
    pub when: Vec<ImpactText>,
    /// The horizon the card's lines are read at: `impact-horizon-<y1|y3|y5>`.
    pub horizon_key: String,
    pub horizon_months: u32,
    /// That horizon falls before the next election (`impact-in-time`).
    pub in_time: bool,
    /// How sure the sizes are: `rough` | `fair` | `firm`
    /// (`impact-sure-<sureness>`).
    pub sureness: String,
    /// Where the sizes come from (`impact-source-<source>`).
    pub source: ImpactText,
}

/// Round away from zero to `figures` significant figures (`up`), or
/// towards it: a shown range never excludes what the exact one holds.
fn round_sig(v: f64, figures: i32, up: bool) -> f64 {
    if v == 0.0 || !v.is_finite() {
        return 0.0;
    }
    let mag = mandate_core::math::floor(mandate_core::math::log10(v.abs())) as i32;
    let step = mandate_core::math::pow(10.0, (mag - figures + 1) as f64);
    let q = v.abs() / step;
    let nearest = mandate_core::math::round_half_away(q);
    // A hair of tolerance, so 150.00000001 does not round out to 160.
    let n = if (q - nearest).abs() < 1e-9 {
        nearest
    } else if up {
        mandate_core::math::floor(q) + 1.0
    } else {
        mandate_core::math::floor(q)
    };
    n * step
}

/// A range as the view shows it: (direction, form, near end, far end).
fn shown(r: Range, unit: Unit) -> (&'static str, &'static str, Option<f64>, Option<f64>) {
    let dir = r.direction();
    if dir == "flat" {
        return (dir, "words", None, None);
    }
    let (near, far) = if dir == "up" {
        (r.lo, r.hi)
    } else {
        (-r.hi, -r.lo)
    };
    // Not vacuous: a range whose far end is over three times its near
    // one is said in words (the design's rule for counts, kept for every
    // unit: "0.0025% to 1.6%" says no more than "higher").
    if r.vague() {
        return (dir, "words", None, None);
    }
    let (lo, hi) = (round_sig(near, 2, false), round_sig(far, 2, true));
    // A count below one is nobody: say it in words.
    if lo <= 0.0 || (unit == Unit::Count && hi < 1.0) {
        return (dir, "words", None, None);
    }
    if lo == hi {
        (dir, "about", Some(lo), None)
    } else {
        (dir, "range", Some(lo), Some(hi))
    }
}

fn class_word(c: mandate_content::ClassKey) -> &'static str {
    match c {
        mandate_content::ClassKey::Low => "low",
        mandate_content::ClassKey::Middle => "middle",
        mandate_content::ClassKey::High => "high",
    }
}

fn sector_name_key(content: &mandate_content::ContentBundle, sector: &str) -> String {
    content
        .sector(sector)
        .map(|s| s.name_key.clone())
        .unwrap_or_default()
}

fn direct_line(
    content: &mandate_content::ContentBundle,
    d: &DirectLine,
    h: usize,
    in_time: bool,
    lead: bool,
) -> ImpactLine {
    let mut args = BTreeMap::new();
    if let Some(s) = &d.sector {
        args.insert("sector_key".to_string(), sector_name_key(content, s));
    }
    if let Some(c) = d.class {
        args.insert("class".to_string(), class_word(c).to_string());
    }
    if let Some(hz) = d.hazard {
        args.insert(
            "hazard_key".to_string(),
            format!("impact-hazard-{}", crate::office::snake(&hz)),
        );
    }
    if let Some(c) = d.category {
        args.insert(
            "category_key".to_string(),
            format!("impact-category-{}", crate::office::snake(&c)),
        );
    }
    let unit = match d.kind {
        "homes_started" | "jobs_supported" | "jobs_created" => Unit::Count,
        "paid" => Unit::Money,
        "tax_points" => Unit::Points,
        _ => Unit::Percent,
    };
    let v = d.at[h];
    // A sign in words (a tax cut, fewer people working), never in the number.
    let direction = if v < 0.0 { "down" } else { "up" };
    args.insert("direction".to_string(), direction.to_string());
    let n = match unit {
        Unit::Count | Unit::Money => {
            mandate_core::math::round_half_away(round_sig(v.abs(), 2, false))
        }
        _ => round_sig(v.abs(), 2, false),
    };
    ImpactLine {
        outcome: "target".into(),
        direction: direction.into(),
        horizon_months: HORIZONS[h],
        in_time,
        form: "exact".into(),
        lo: Some(n),
        hi: None,
        unit: unit.key().into(),
        key: format!("impact-target-{}", d.kind.replace('_', "-")),
        args,
        lead,
        tradeoff: d.kind == "labour_supply",
    }
}

fn outcome_line(
    content: &mandate_content::ContentBundle,
    o: &OutcomeLine,
    h: usize,
    in_time: bool,
    lead: bool,
) -> ImpactLine {
    let (direction, form, lo, hi) = shown(o.at[h], o.unit);
    let mut args = BTreeMap::new();
    if let Some(s) = &o.sector {
        args.insert("sector_key".to_string(), sector_name_key(content, s));
    }
    // The three class approvals share the approval sentences, with the class.
    let word = o.outcome.key();
    let (stem, class) = match word.strip_prefix("approval_") {
        Some(c) => ("approval", Some(c)),
        None => (word, None),
    };
    args.insert("group".to_string(), class.unwrap_or("all").to_string());
    ImpactLine {
        outcome: word.into(),
        direction: direction.into(),
        horizon_months: HORIZONS[h],
        in_time,
        form: form.into(),
        lo,
        hi,
        unit: o.unit.key().into(),
        key: format!("impact-{}-{direction}", stem.replace('_', "-")),
        args,
        lead,
        tradeoff: !o.lead && o.adverse(direction),
    }
}

/// The view of an estimate, at full knowledge.
pub fn impact_view(
    st: &State,
    content: &mandate_content::ContentBundle,
    e: &Estimate,
) -> ImpactView {
    let f = e.focus;
    // The card: the bill's own count, its purpose, and up to two other
    // clear effects (costs first). Approval is the ward's and the party's
    // line on the card; here it feeds who gains and who loses.
    let approval = |o: &OutcomeLine| o.outcome.key().starts_with("approval");
    let lead_outcome = e.outcomes.iter().position(|o| o.lead);
    let mut others: Vec<usize> = e
        .outcomes
        .iter()
        .enumerate()
        .filter(|(i, o)| Some(*i) != lead_outcome && !approval(o))
        .filter(|(_, o)| o.at[f].direction() != "flat")
        .map(|(i, _)| i)
        .collect();
    others.sort_by_key(|i| !e.outcomes[*i].adverse(e.outcomes[*i].at[f].direction()));
    others.truncate(2);
    // The direct count the card leads with: the first, and a programme's
    // protection (which has no outcome of its own without its hazard).
    let lead_direct = |i: usize, d: &DirectLine| {
        i == 0 || (d.kind == "protection" && lead_outcome.is_none() && i < 2)
    };
    let mut lines: Vec<ImpactLine> = Vec::new();
    for h in 0..HORIZONS.len() {
        let in_time = e.in_time && h <= f;
        for (i, d) in e.direct.iter().enumerate() {
            lines.push(direct_line(
                content,
                d,
                h,
                in_time,
                h == f && lead_direct(i, d),
            ));
        }
        for (i, o) in e.outcomes.iter().enumerate() {
            let lead = h == f && (Some(i) == lead_outcome || others.contains(&i));
            lines.push(outcome_line(content, o, h, in_time, lead));
        }
    }
    // Lead lines first: counts, then the purpose, then the rest.
    lines.sort_by_key(|l| !l.lead);

    let c = &e.cost;
    let pct = |v: f64| mandate_core::math::round_half_away(v * 100.0).max(0.0) as u32;
    let cost = CostView {
        yearly: c.yearly,
        budget_pct: round_sig(c.budget_share * 100.0, 2, false),
        paid_from: c.paid_from.key().into(),
        funded_first_pct: pct(c.funded_first),
        funded_later_pct: pct(c.funded_later),
        borrowed_pct: pct(c.borrow_share),
        key: if c.yearly > 0.0 {
            "impact-cost"
        } else {
            "impact-cost-none"
        }
        .into(),
        paid_key: (c.yearly > 0.0).then(|| format!("impact-paid-{}", c.paid_from.key())),
    };

    let mut groups: Vec<GroupImpact> = Vec::new();
    for gains in [true, false] {
        groups.extend(
            e.groups
                .iter()
                .filter(|g| g.gains == gains)
                .take(2)
                .map(|g| GroupImpact {
                    group: g.group.into(),
                    gains,
                    key: format!(
                        "impact-{}-{}",
                        if gains { "gains" } else { "loses" },
                        g.group
                    ),
                }),
        );
    }

    let months = |m: u32| BTreeMap::from([("n".to_string(), m.to_string())]);
    let mut when: Vec<ImpactText> = Vec::new();
    if e.timing.at_once {
        when.push(ImpactText {
            key: "impact-when-at-once".into(),
            args: BTreeMap::new(),
        });
    } else {
        // The first year's room is small: say so when it holds the bill back.
        if c.yearly > 0.0 && c.funded_first < 0.995 && c.funded_first < c.funded_later {
            when.push(ImpactText {
                key: "impact-when-first-year".into(),
                args: BTreeMap::from([("funded".to_string(), pct(c.funded_first).to_string())]),
            });
        }
        if let Some(m) = e.timing.start {
            when.push(ImpactText {
                key: "impact-when-building".into(),
                args: months(m),
            });
        }
        if let Some(m) = e.timing.full {
            when.push(ImpactText {
                key: "impact-when-full".into(),
                args: months(m),
            });
        }
    }

    ImpactView {
        at: crate::post::jurisdiction_dto(&st.world, e.at),
        cost,
        lines,
        groups,
        when,
        horizon_key: format!("impact-horizon-{}", ["y1", "y3", "y5"][f.min(2)]),
        horizon_months: HORIZONS[f.min(2)],
        in_time: e.in_time,
        // Arithmetic is firm; the lab's ranges at full knowledge are fair.
        sureness: "fair".into(),
        source: ImpactText {
            key: "impact-source-statistician".into(),
            args: BTreeMap::new(),
        },
    }
}

/// What a bill or law would do if it passed now, in its own jurisdiction
/// (`inst.at()`, DEC-098). `None` where the impact table covers no such
/// tier: no estimate, not a guess.
pub fn impact_of(engine: &Engine, p: &PolicyInstance) -> Option<ImpactView> {
    let st = engine.state();
    let content = engine.content();
    let r = mandate_sim::scheduler::Reads::of_state(st, content);
    let e = mandate_sim::policy::impact::estimate_for(&r, &st.policy, p)?;
    Some(impact_view(st, content, &e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shown_range_never_excludes_the_exact_one() {
        for (lo, hi) in [
            (113.4, 187.9),
            (0.0223, 0.0456),
            (1502.0, 1549.0),
            (9.96, 10.04),
        ] {
            let r = Range {
                lo,
                mid: (lo + hi) / 2.0,
                hi,
            };
            let (dir, form, a, b) = shown(r, Unit::Percent);
            assert_eq!(dir, "up");
            let a = a.unwrap_or(0.0);
            let b = b.unwrap_or(a);
            assert!(a <= lo + 1e-9, "{form}: {a} > {lo}");
            assert!(b >= hi - 1e-9 || form == "about", "{form}: {b} < {hi}");
        }
        // The same, mirrored, for a fall; sizes are never signed.
        let (dir, form, a, b) = shown(
            Range {
                lo: -187.9,
                mid: -150.0,
                hi: -113.4,
            },
            Unit::Count,
        );
        assert_eq!((dir, form), ("down", "range"));
        assert_eq!((a, b), (Some(110.0), Some(190.0)));
    }

    #[test]
    fn no_clear_change_and_vague_counts_are_said_in_words() {
        let flat = Range {
            lo: -3.0,
            mid: 1.0,
            hi: 5.0,
        };
        assert_eq!(shown(flat, Unit::Percent), ("flat", "words", None, None));
        // The far end more than three times the near one: no number.
        let wide = Range {
            lo: 20.0,
            mid: 60.0,
            hi: 100.0,
        };
        assert_eq!(shown(wide, Unit::Count), ("up", "words", None, None));
        assert_eq!(shown(wide, Unit::Percent), ("up", "words", None, None));
        // A range barely clear of zero is a direction, not a figure.
        let hair = Range {
            lo: 0.0025,
            mid: 0.8,
            hi: 1.6,
        };
        assert_eq!(shown(hair, Unit::Percent), ("up", "words", None, None));
        assert_eq!(round_sig(150.0, 2, true), 150.0);
        assert_eq!(round_sig(151.0, 2, true), 160.0);
        assert_eq!(round_sig(159.0, 2, false), 150.0);
    }
}
