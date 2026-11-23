//! Topic 8, claims check: the mechanisms the docs describe, each switched
//! to neutral in turn, and whether anything the player sees changes (race
//! winners, unemployment, approval). The static half of the check (what is
//! described but not modelled at all) is in the report, from reading the
//! code; this measures the "modelled but no effect" half.

use super::sensitivity::{flipped, outcomes};
use super::*;

/// (mechanism, where it is claimed, the switch that turns it off).
/// A mechanism, where it is claimed, and the parameters that switch it off.
type Claim = (&'static str, &'static str, &'static [(&'static str, f64)]);

const CLAIMS: [Claim; 16] = [
    (
        "Directional voting rises with polarisation (β)",
        "SIMULATION_MODELS §1",
        &[
            ("politics.vote.beta_high", 0.15),
            ("politics.vote.beta_low", 0.15),
        ],
    ),
    (
        "Benchmarked retrospective economy term",
        "SIMULATION_MODELS §2",
        &[("politics.vote.retro_economy_weight", 0.0)],
    ),
    (
        "Executive weighted more for record",
        "SIMULATION_MODELS §2",
        &[("politics.vote.executive_multiplier", 1.0)],
    ),
    (
        "Media weight by the town's share of its region",
        "SIMULATION_MODELS §2",
        &[("politics.vote.media_town_share", 0.0)],
    ),
    (
        "Partisan perception filter",
        "SIMULATION_MODELS §3",
        &[("politics.vote.partisan_discount", 0.0)],
    ),
    (
        "Mover share of party ID drifts with the town hall's record",
        "SIMULATION_MODELS §3",
        &[("politics.party_id.governing_credit", 0.0)],
    ),
    (
        "Turnout habit",
        "SIMULATION_MODELS §4",
        &[("politics.turnout.habit", 0.0)],
    ),
    (
        "Strategic (wasted-vote) desertion of weak parties",
        "DEC-071",
        &[("politics.vote.strategic_weight", 0.0)],
    ),
    (
        "Home-ward bonus",
        "DEC-028",
        &[("politics.vote.home_ward_bonus", 0.0)],
    ),
    (
        "Experienced rent growth in approval",
        "opinion.jsonc",
        &[("opinion.experience.rent_growth", 0.0)],
    ),
    (
        "Experienced services in approval",
        "opinion.jsonc",
        &[("opinion.experience.services", 0.0)],
    ),
    (
        "Construction disruption in approval",
        "opinion.jsonc",
        &[("opinion.experience.disruption", 0.0)],
    ),
    (
        "Laws' signatures in approval",
        "opinion.jsonc",
        &[("opinion.law_reaction_weight", 0.0)],
    ),
    (
        "Neighbourhood job-loss tilt in worries",
        "opinion.local_concerns",
        &[("opinion.local_concerns.jobs_sector_tilt", 0.0)],
    ),
    (
        "Gravity migration between towns",
        "SIMULATION_MODELS §9",
        &[("economy.migration_rate", 0.0)],
    ),
    (
        "Poll shared error (polls only)",
        "SIMULATION_MODELS §6",
        &[("politics.polls.shared_sd", 0.0)],
    ),
];

pub fn run(cx: &Ctx) -> Result<Value, String> {
    let years = cx.years(10);
    let seeds = cx.args.seeds;
    let base = outcomes(&cx.content, &[], seeds, years, 0)?;
    let mut rows = Vec::new();
    for (what, src, sets) in CLAIMS {
        if !cx.args.only.is_empty() && !cx.args.only.iter().any(|o| what.contains(o.as_str())) {
            continue;
        }
        let sets: Vec<(String, Value)> = sets
            .iter()
            .map(|(p, v)| (p.to_string(), json!(v)))
            .collect();

        let o = outcomes(&cx.content, &sets, seeds, years, 0)?;
        let (f, n) = flipped(&base, &o);
        let identical = o.per_seed == base.per_seed && f == 0;
        let du = (o.u_median - base.u_median) * 100.0;
        let dr = (o.reelection - base.reelection) * 100.0;
        let dg = (o.gov_mean - base.gov_mean) * 100.0;
        let status = if identical {
            "no_effect_at_all"
        } else if f == 0 {
            "no_effect_on_outcomes"
        } else {
            "modelled_and_used"
        };
        let row = json!({
            "mechanism": what, "claimed_in": src, "switched_off_by": sets,
            "races_flipped": f, "races": n, "races_flipped_share": r3(f as f64 / n.max(1) as f64),
            "d_unemployment_pts": r3(du), "d_reelection_pts": r1(dr), "d_gov_approval_pts": r1(dg),
            "status": status,
        });
        eprintln!("{}", serde_json::to_string(&row).unwrap_or_default());
        rows.push(row);
    }
    Ok(json!({
        "params": { "seeds": seeds, "years": years, "driver": "idle", "world": "random country, small" },
        "base": super::sensitivity::outcome_json(&base),
        "rows": rows,
    }))
}
