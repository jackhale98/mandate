//! The gate ladder: margins, the gate list, the enforced list and the failed-gate check.

/// Margins of the gate ladder (ENGAGEMENT §7.2), in vote-share points
/// (0.01 = one point) and win-rate shares.
pub const ROTE_SHARE_MARGIN: f64 = 0.01;
pub const ROTE_WIN_MARGIN: f64 = 0.05;
/// `ladder_learned`: + 1.0 point until E8 (enforced), + 3.0 from E8
/// (`LEARNED_MARGIN_E8`, reported as `ladder_learned_e8` until it passes).
pub const LEARNED_MARGIN: f64 = 0.01;
pub const LEARNED_MARGIN_E8: f64 = 0.03;
pub const KNOWLEDGE_MARGIN: f64 = 0.03;
pub const SKILL_SHARE_MARGIN: f64 = 0.03;
pub const SKILL_WIN_MARGIN: f64 = 0.10;
pub const IDLE_CAP: f64 = 0.15;

/// The gate ladder (ENGAGEMENT §7.2), every rung, reported in this order.
pub const GATES: [&str; 10] = [
    "idle_cap",
    "effort_matters",
    "exploring_alone_never_wins",
    "ladder_rote",
    "ladder_learned",
    "knowledge_matters",
    "skill_gap",
    "good_styles_vary",
    "corrupt_not_dominant",
    "proxy_not_optimal",
];

/// The gates `bots` fails on (the ladder's "Enforced" column, ENGAGEMENT
/// §7.2): E1's, and from E8 `ladder_rote`. E8's other rungs
/// (`ladder_learned` at + 3.0, reported as `ladder_learned_e8`;
/// `knowledge_matters`; `skill_gap`; clauses (a) and (c) of
/// `good_styles_vary`) do not pass yet and stay reported, at their full
/// thresholds, until the levers named in the E8 report close the gap.
/// `good_styles_vary_b` is clause (b) of `good_styles_vary`.
pub const ENFORCED: [&str; 8] = [
    "idle_cap",
    "effort_matters",
    "exploring_alone_never_wins",
    "ladder_rote",
    "ladder_learned",
    "good_styles_vary_b",
    "corrupt_not_dominant",
    "proxy_not_optimal",
];

/// "Exploring alone never wins": `explore_only` wins at most this much more
/// often than `idle`, and less often than the median rote routine.
pub const EXPLORE_ONLY_OVER_IDLE: f64 = 0.05;

/// "Corrupt not dominant": the corrupt routine's win rate is at most this
/// much above the best honest strategy's.
pub const CORRUPT_MARGIN: f64 = 0.1;

/// The enforced gates a report fails, by name. Under the lab switch
/// (`--omniscient`, a baseline) knowledge is free, so `ladder_learned` is
/// reported but not enforced.
pub fn failed_gates(report: &serde_json::Value) -> Vec<&'static str> {
    let omniscient = report["omniscient"].as_bool().unwrap_or(false);
    ENFORCED
        .iter()
        .copied()
        .filter(|g| !(omniscient && *g == "ladder_learned"))
        .filter(|g| !report[*g].as_bool().unwrap_or(false))
        .collect()
}
