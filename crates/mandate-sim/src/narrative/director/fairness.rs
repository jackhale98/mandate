//! Daily fairness warnings (DEC-006): the career's dangers.

use super::*;

// --- Daily: fairness (DEC-006) ------------------------------------------------------------

/// The tracks the fairness rule watches: (key, value, in danger).
pub fn track_dangers(r: &Reads<'_>) -> Vec<(&'static str, f32, bool, f64)> {
    let Some(pl) = r.player else {
        return Vec::new();
    };
    let f = &r.content.director.fairness;
    let t = &pl.tracks;
    let low = |v: f32, warn: f64| ((warn - v as f64) / warn.max(1.0)).clamp(0.0, 1.0);
    let high = |v: f32, warn: f64| ((v as f64 - warn) / (100.0 - warn).max(1.0)).clamp(0.0, 1.0);
    vec![
        (
            "standing",
            t.standing,
            t.standing as f64 <= f.standing,
            low(t.standing, f.standing),
        ),
        (
            "viability",
            t.viability,
            t.viability as f64 <= f.viability,
            low(t.viability, f.viability),
        ),
        (
            "health",
            t.health,
            t.health as f64 <= f.health,
            low(t.health, f.health),
        ),
        (
            "legal_jeopardy",
            t.legal_jeopardy,
            t.legal_jeopardy as f64 >= f.legal_jeopardy,
            high(t.legal_jeopardy, f.legal_jeopardy),
        ),
        (
            "threat",
            t.threat,
            t.threat as f64 >= f.threat,
            high(t.threat, f.threat),
        ),
    ]
}

/// Narrative domain, Director phase, daily: a warning whenever a pressure
/// track reaches danger (repeated while it stays there). Warnings are never
/// budgeted: fairness comes before pacing.
pub fn daily(n: &mut NarrativeState, r: &Reads<'_>, out: &mut Outs<'_>) {
    if !active(r) {
        return;
    }
    let repeat = r.content.director.fairness.repeat_days;
    for (track, value, danger, _) in track_dangers(r) {
        if !danger {
            continue;
        }
        let recent = n
            .warnings
            .iter()
            .rev()
            .find(|w| w.track == track)
            .is_some_and(|w| w.date.add_days(repeat) > r.date);
        if recent {
            continue;
        }
        n.warnings.push(Warning {
            track: track.into(),
            date: r.date,
            value,
        });
        if n.warnings.len() > 100 {
            n.warnings.remove(0);
        }
        n.log(
            r.date,
            FactKind::TrackWarning {
                track: track.into(),
                value,
            },
            Vec::new(),
            true,
            0.8,
        );
        out.events.record(
            r.date,
            EventKind::TrackWarning {
                track: track.into(),
                value: value.round() as i32,
            },
            crate::spine::Cause::Pressure(crate::spine::Pressure::Track),
        );
    }
}
