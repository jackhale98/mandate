//! Balance blocks for opinion dynamics and local concerns (DEC-025, DEC-078 decision 1).

use super::*;

// --- Balance parameter blocks (DEC-078 decision 1) ---------------------------
// Numbers we expect to tune live here, one block per system; structural
// constants (midpoints, unit conversions, array sizes) stay in code. Ranges
// are validated in `validate::param_ranges`. The lab overrides any of them
// with `mandate-cli sweep|bots --set <kind>.<path>=<value>`.

/// Monthly opinion dynamics (DEC-025): experienced conditions and reactions
/// to laws and bills move approval of the local government and the player.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpinionParams {
    /// What residents experience, in approval units (tanh scale).
    pub experience: ExperienceWeights,
    /// Weight of a bill or law's anticipated reaction per unit of awareness.
    pub policy_reaction: f64,
    /// Weight of reactions to laws in government approval, beside experience.
    pub law_reaction_weight: f64,
    /// Monthly pull of government approval toward its target.
    pub gov_approval_rate: f64,
    /// Monthly pull of approval of the player toward reactions to their bills.
    pub player_approval_rate: f64,
    /// Sharpness of reactions to the player's bills.
    pub player_reaction_scale: f64,
    /// How a neighbourhood's own conditions turn its town's indicators into
    /// its own concern levels.
    pub local_concerns: LocalConcernParams,
}

/// Neighbourhood concern levels: the town's indicators are measured per
/// town; each neighbourhood feels them through its own residents, homes and
/// surroundings. A neighbourhood's indicator is the town's × a relative
/// factor, scaled so the residents' weighted mean over the town is 1 (the
/// town's own figure is unchanged). Tax burden and budget stress are felt at
/// the town's level everywhere; a recent disaster by its impact (DEC-090 D8).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalConcernParams {
    /// Rent by neighbourhood kind, relative (every kind > 0). A district's
    /// rent burden is its rent over its residents' incomes (the mean of
    /// rent ÷ income across its income classes).
    pub rent: NeighborhoodLean,
    /// Elasticity of a district's rent to how much faster it has grown since
    /// play began than its town (residents against their baseline).
    pub rent_growth: f64,
    /// Exposure to the region's pollution by neighbourhood kind, before
    /// workplaces and water.
    pub pollution: NeighborhoodLean,
    /// Added exposure per polluting workplace (an industrial or extraction
    /// workplace in `workplaces`) in the district.
    pub pollution_workplace: f64,
    /// Added exposure per polluting workplace in a neighbouring district.
    pub pollution_next_door: f64,
    /// Added exposure × the district's flood exposure (0..1): the low land
    /// by the river or shore takes the runoff.
    pub pollution_floodplain: f64,
    /// Place kinds (snake-case keys) that employ a sector driver's workers.
    pub workplaces: BTreeMap<String, SectorDriver>,
    /// A district's extra share of a sector's workers per workplace of that
    /// sector in it (half for one next door).
    pub workplace_affinity: f64,
    /// A district's extra share of a sector's workers by its kind.
    pub sector_kinds: Vec<SectorKindLean>,
    /// Weight of a sector's decline since play began (1 − jobs ÷ its
    /// starting jobs) beside its cyclicality in who feels the town's job
    /// losses.
    pub job_loss_decline: f64,
    /// Share of the jobs a district's sectors lost since play began, beyond
    /// the town's average, that adds to its unemployment rate (0: by income
    /// class alone).
    pub jobs_sector_tilt: f64,
}

/// Extra share of one sector driver's workers by neighbourhood kind.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SectorKindLean {
    pub driver: SectorDriver,
    pub kinds: NeighborhoodLean,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExperienceWeights {
    /// Per point of local unemployment rise beyond the national rise (a year).
    pub unemployment: f64,
    /// Per point of real rent growth (a year).
    pub rent_growth: f64,
    /// Per unit of service improvement from laws in effect.
    pub services: f64,
    /// While a disaster is recent.
    pub disaster: f64,
    /// Months a disaster stays recent.
    pub disaster_months: u16,
    /// Per unit of construction disruption.
    pub disruption: f64,
}
