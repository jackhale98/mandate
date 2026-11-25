//! Condition checks: impossible bounds and contradictory conjunctions.

use super::*;

/// A numeric quantity a condition bounds, its interval, and the quantity's
/// possible range.
pub(super) struct Bound {
    quantity: String,
    lo: f64,
    hi: f64,
    domain: (f64, f64),
}

pub(super) fn bound(
    quantity: String,
    min: Option<f64>,
    max: Option<f64>,
    domain: (f64, f64),
) -> Bound {
    Bound {
        quantity,
        lo: min.unwrap_or(f64::NEG_INFINITY),
        hi: max.unwrap_or(f64::INFINITY),
        domain,
    }
}

pub(super) const INF: f64 = f64::INFINITY;

pub(super) fn indicator_domain(i: mandate_content::Indicator) -> (f64, f64) {
    use mandate_content::Indicator as I;
    match i {
        I::Unemployment | I::RecentDisaster | I::JobLoss => (0.0, 1.0),
        I::UnemploymentRise => (-1.0, 1.0),
        I::ServiceShortfall => (-INF, 1.0),
        I::Pollution => (0.0, 3.0),
        I::RentBurden | I::TaxBurden | I::BudgetStress => (0.0, INF),
    }
}

/// The quantity a condition bounds (one table for the one condition language).
pub(super) fn cond_bound(c: &Condition) -> Option<Bound> {
    use Condition as C;
    Some(match c {
        C::Crisis {
            kind: Some(kind),
            min_severity,
        } => bound(
            format!("crisis {kind} severity"),
            Some(*min_severity),
            None,
            (0.0, 1.0),
        ),
        C::Crisis {
            kind: None,
            min_severity,
        } => bound(
            "live crisis severity".into(),
            Some(*min_severity),
            None,
            (0.0, 1.0),
        ),
        C::Indicator {
            indicator,
            min,
            max,
        } => bound(
            format!("indicator {indicator:?}"),
            *min,
            *max,
            indicator_domain(*indicator),
        ),
        // Every track, the record and the Mandate run 0..100.
        C::Track { track, min, max } => bound(format!("track {track:?}"), *min, *max, (0.0, 100.0)),
        C::Money { min, max } => bound("money".into(), *min, *max, (-INF, INF)),
        C::Opinion { slot, min, max } => {
            bound(format!("opinion of {slot}"), *min, *max, (-100.0, 100.0))
        }
        C::Personality {
            slot,
            axis,
            min,
            max,
        } => bound(format!("{slot} {axis}"), *min, *max, (0.0, 1.0)),
        C::Trust { slot, min, max } => bound(format!("trust of {slot}"), *min, *max, (0.0, 1.0)),
        C::PlayerAwareness { min, max } => bound("player awareness".into(), *min, *max, (0.0, 1.0)),
        C::PlayerApproval { min, max } => bound("player approval".into(), *min, *max, (-1.0, 1.0)),
        C::RivalRecord { min, max } => bound("rival record".into(), *min, *max, (0.0, 1.0)),
        _ => return None,
    })
}

/// A discrete quantity and the values a condition allows.
pub(super) fn cond_discrete(c: &Condition, words: &[String]) -> Option<(String, BTreeSet<String>)> {
    use Condition as C;
    let set = |xs: &[&str]| xs.iter().map(|x| x.to_string()).collect::<BTreeSet<_>>();
    Some(match c {
        C::Campaigning { is } => ("campaigning".into(), set(&[&is.to_string()])),
        // The player's post as a role word (DEC-098): `any` is every post's
        // word, `none` out of office.
        C::Office { posts, role } => (
            "the player's post".into(),
            posts
                .iter()
                .chain(role.iter())
                .flat_map(|w| match w.as_str() {
                    mandate_content::OFFICE_ANY => words.to_vec(),
                    mandate_content::OFFICE_NONE => vec!["none".to_string()],
                    w => vec![w.to_string()],
                })
                .collect(),
        ),
        // A negated one (`is: false`) allows every other role: no finite set
        // to compare here.
        C::Occupation {
            slot,
            occupations,
            is: true,
        } => (
            format!("role of {slot}"),
            occupations.iter().cloned().collect(),
        ),
        C::IsBroker { slot, is } => (format!("{slot} is the broker"), [is.to_string()].into()),
        C::IsRival { slot, is } => (format!("{slot} is the rival"), [is.to_string()].into()),
        C::Voice { voices } => ("voice".into(), voices.iter().cloned().collect()),
        // The one top worry (two top-one worries that share none contradict).
        C::Concern { .. } => {
            let w = c.worry()?;
            if w.top != 1 {
                return None;
            }
            let whose = match w.of {
                ConcernOf::Town => "the town's",
                ConcernOf::Speaker => "the speaker's",
                ConcernOf::Here => "the neighbourhood's",
            };
            let how = match w.by {
                Worry::Felt => "",
                Worry::Measured => " measured",
            };
            (
                format!("{whose} top{how} concern"),
                w.concerns.into_iter().cloned().collect(),
            )
        }
        _ => return None,
    })
}

/// Check one conjunction: each bound against its range and itself, then
/// bounds and discrete values against each other.
pub(super) fn check_conjunction(
    out: &mut Out,
    item: &str,
    what: &str,
    bounds: Vec<Bound>,
    discrete: Vec<(String, BTreeSet<String>)>,
) {
    let mut by: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for b in &bounds {
        if b.lo > b.hi {
            out.add(
                Sev::Error,
                "unsatisfiable",
                item,
                format!(
                    "{what}: {} bounds contradict (min {} > max {})",
                    b.quantity, b.lo, b.hi
                ),
            );
        } else if b.lo > b.domain.1 || b.hi < b.domain.0 {
            out.add(
                Sev::Error,
                "unsatisfiable",
                item,
                format!(
                    "{what}: {} in [{}, {}] is outside its possible range [{}, {}]",
                    b.quantity, b.lo, b.hi, b.domain.0, b.domain.1
                ),
            );
        }
        let e = by.entry(b.quantity.clone()).or_insert((-INF, INF));
        e.0 = e.0.max(b.lo);
        e.1 = e.1.min(b.hi);
    }
    for (q, (lo, hi)) in &by {
        let n = bounds.iter().filter(|b| &b.quantity == q).count();
        if n > 1 && lo > hi {
            out.add(
                Sev::Error,
                "contradiction",
                item,
                format!("{what}: the conditions on {q} cannot all hold (need ≥ {lo} and ≤ {hi})"),
            );
        }
    }
    let mut sets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for (q, s) in discrete {
        *counts.entry(q.clone()).or_default() += 1;
        match sets.get_mut(&q) {
            Some(cur) => *cur = cur.intersection(&s).cloned().collect(),
            None => {
                sets.insert(q, s);
            }
        }
    }
    for (q, s) in sets {
        if s.is_empty() && counts.get(&q).copied().unwrap_or(0) > 1 {
            out.add(
                Sev::Error,
                "contradiction",
                item,
                format!("{what}: the conditions on {q} allow no value together"),
            );
        }
    }
}

/// One list of conditions (a conjunction): bounds and discrete values, a law
/// both absent and in force, `dry` with `water`, and each `any`
/// alternative with the rest.
pub(super) fn check_conditions(out: &mut Out, item: &str, what: &str, cs: &[Condition]) {
    let bounds = cs.iter().filter_map(cond_bound).collect();
    let words = out.role_words.clone();
    let discrete = cs.iter().filter_map(|c| cond_discrete(c, &words)).collect();
    check_conjunction(out, item, what, bounds, discrete);
    // Two `law_in_force` on the same templates, one absent and one present.
    for a in cs {
        for b in cs {
            if let (
                Condition::LawInForce {
                    templates: ta,
                    absent: true,
                },
                Condition::LawInForce {
                    templates: tb,
                    absent: false,
                },
            ) = (a, b)
                && tb.iter().all(|t| ta.contains(t))
            {
                out.add(
                    Sev::Error,
                    "contradiction",
                    item,
                    format!("{what}: a law must be both absent and in force"),
                );
            }
        }
    }
    let dry = cs.iter().any(|c| matches!(c, Condition::Dry));
    let wet = cs.iter().any(|c| matches!(c, Condition::Water { .. }));
    if dry && wet {
        out.add(
            Sev::Error,
            "contradiction",
            item,
            format!("{what}: `dry` and `water` together"),
        );
    }
    for c in cs {
        if let Condition::Any { of } = c {
            for (i, x) in of.iter().enumerate() {
                check_conditions(
                    out,
                    item,
                    &format!("{what} (any, alternative {})", i + 1),
                    std::slice::from_ref(x),
                );
            }
        }
    }
}

/// The speaker roles a line's conditions name (`None`: any; a negated
/// condition, `is: false`, names none and so narrows nothing here).
pub(super) fn speaker_roles(cs: &[Condition]) -> Option<BTreeSet<String>> {
    let mut out: Option<BTreeSet<String>> = None;
    for c in cs {
        if let Condition::Occupation {
            slot,
            occupations,
            is: true,
        } = c
            && slot == "speaker"
        {
            let s: BTreeSet<String> = occupations.iter().cloned().collect();
            out = Some(match out {
                Some(o) => o.intersection(&s).cloned().collect(),
                None => s,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn contradictions_and_impossible_bounds_are_found() {
        let mut out = Out {
            findings: Vec::new(),
            files: BTreeMap::new(),
            role_words: vec!["councillor".into(), "mayor".into()],
        };
        check_conditions(
            &mut out,
            "x",
            "triggers",
            &[
                Condition::Opinion {
                    slot: "r".into(),
                    min: Some(120.0),
                    max: None,
                },
                Condition::Office {
                    posts: Vec::new(),
                    role: Some("mayor".into()),
                },
                Condition::Office {
                    posts: vec!["none".into()],
                    role: None,
                },
            ],
        );
        assert!(out.findings.iter().any(|f| f.check == "unsatisfiable"));
        assert!(out.findings.iter().any(|f| f.check == "contradiction"));
    }
}
