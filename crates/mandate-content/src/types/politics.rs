//! Politics parameters (DEC-027 to DEC-030, DEC-071): NPC utility AI, votes, turnout, polls, campaigns, party id.

use super::*;

// --- Politics parameters (DEC-027 to DEC-030, DEC-071) -----------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PoliticsParams {
    /// Parties generated per world under plurality rules (two majors plus minors).
    pub party_count: u8,
    pub party_names: PartyNameParts,
    pub vote: VoteParams,
    pub turnout: TurnoutParams,
    pub polls: PollParams,
    pub campaign: CampaignParams,
    pub party_id: PartyIdParams,
    pub ai: AiParams,
    /// How a party takes a position on a bill (the party line, DEC-027).
    pub party_line: PartyLineParams,
}

/// The party line: one rule for the player's whip and for NPC members
/// (DEC-030). `score` = `platform` × the bill's fit with the party's
/// platform (−1..1) + a term for who sponsors it; inside ±`free_band` the
/// vote is free.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PartyLineParams {
    /// Weight of the bill's fit with the party's platform.
    pub platform: f64,
    /// Sponsor terms: a bill from the party's own member, from the party that
    /// governs (holds the signing office), and from any other party.
    pub own_sponsor: f64,
    pub governing_sponsor: f64,
    pub other_sponsor: f64,
    /// A score inside ±this is a free vote.
    pub free_band: f64,
    /// How hard a whipped NPC member is pulled: lean = this × score × the
    /// member's loyalty factor.
    pub npc_pull: f64,
}

// --- NPC utility AI (DEC-030, DEC-071 §8) ---------------------------------------

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum AiAction {
    Canvass,
    TownHall,
    Advertise,
    Fundraise,
    Lobby,
    ProposeBill,
    Rest,
}

/// Normalised (0..1) inputs the engine computes for an NPC each decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiInput {
    /// 1 while the NPC is a candidate in an open race.
    Campaigning,
    /// 0 at the start of the campaign, 1 on election eve.
    ElectionNear,
    /// 1 for a dead heat (from polls when published, else party strength).
    RaceCloseness,
    /// 1 − the NPC's mean awareness among the race's voters.
    Unknown,
    /// Campaign funds relative to a few weeks of advertising.
    Funds,
    /// Strength of the NPC's view on the most important pending bill.
    BillStake,
    /// 1 when that bill's vote is on a knife edge.
    BillCloseness,
    /// Appeal of the NPC's best policy idea (own values and constituents).
    Agenda,
    /// 1 if the NPC has no bill of their own pending.
    NoOwnBill,
    /// 1 when the council has few bills pending.
    CouncilFree,
    /// Always 1.
    Constant,
}

/// Response curve mapping an input (0..1) to a score (0..1).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Curve {
    /// m·x + b, clamped to 0..1.
    Linear { m: f64, b: f64 },
    /// 1 / (1 + e^(−k (x − x0))).
    Logistic { k: f64, x0: f64 },
    /// `lo` below `x`, `hi` at or above.
    Step { x: f64, lo: f64, hi: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Consideration {
    pub input: AiInput,
    pub curve: Curve,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiActionDef {
    pub action: AiAction,
    pub weight: f64,
    pub considerations: Vec<Consideration>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiProfileMult {
    pub action: AiAction,
    pub mult: f64,
}

/// Strategic profile (brief §9.4): multiplies action scores.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiProfile {
    pub id: String,
    pub mults: Vec<AiProfileMult>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AiParams {
    /// Choose by weighted chance among the best `top_n` scores.
    pub top_n: u8,
    pub actions: Vec<AiActionDef>,
    pub profiles: Vec<AiProfile>,
}

/// Words for generated party names: an attitude-led adjective plus a noun.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PartyNameParts {
    pub nouns: Vec<String>,
    pub by_attitude: Vec<AttitudeWords>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AttitudeWords {
    /// An attitude key, e.g. `redistribution`.
    pub attitude: String,
    pub pro: Vec<String>,
    pub anti: Vec<String>,
}

/// Per-cohort vote utility (DEC-028, DEC-071 §1–2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VoteParams {
    pub issue_weight: f64,
    /// Directional share β at low and high polarisation.
    pub beta_low: f64,
    pub beta_high: f64,
    /// Mean attitude standard deviation mapped to β_low and β_high.
    pub polarization_low: f64,
    pub polarization_high: f64,
    pub party_weight: f64,
    pub valence_weight: f64,
    pub retrospective_weight: f64,
    /// Retrospective weight multiplier for executive offices.
    pub executive_multiplier: f64,
    pub home_ward_bonus: f64,
    /// Share of retrospective blame partisans forgive their own side.
    pub partisan_discount: f64,
    /// Awareness floor for an incumbent within their constituency.
    pub incumbent_awareness: f64,
    /// Awareness of a fresh challenger before campaigning.
    pub challenger_awareness: f64,
    /// Per-candidate race shock (utility units).
    pub race_shock_sd: f64,
    /// National swing per party shared by every race in an election.
    pub national_swing_sd: f64,
    /// Wasted-vote penalty for candidates whose party is weak locally (Duverger).
    pub strategic_weight: f64,
    /// Minor parties contest only where their attachment share reaches this.
    pub minor_party_threshold: f64,
    /// An incumbent's record of visible work (DEC-076). Below the baseline,
    /// opponents campaign on it and every voter hears (`record_penalty` per
    /// point, not discounted by awareness); above it, voters who know the
    /// incumbent reward it a little (`record_bonus`).
    pub record_penalty: f64,
    pub record_bonus: f64,
    pub record_baseline: f64,
    /// Below this Standing the party will not renominate its own officeholder.
    pub renomination_standing: f64,
    /// Issue salience (DEC-078 decision 3): every attitude's raw salience in
    /// a town is this base plus each concern's level × |its weight on the
    /// attitude|; the vector is then scaled to a mean of 1, so the issue
    /// weight keeps its meaning. A larger base mutes local priorities.
    pub salience_base: f64,
    /// Perceived position on a partisan ballot: the candidate's own views and
    /// their party's platform, blended with these weights.
    pub perceived_own_weight: f64,
    pub perceived_party_weight: f64,
    /// Valence: weight of oratory and of integrity (each around its midpoint).
    pub valence_oratory: f64,
    pub valence_integrity: f64,
    /// Share of the incumbent's record carried by the governing party's new nominee.
    pub governing_party_record: f64,
    /// Weight of the town's benchmarked economy in retrospection, beside
    /// approval of the local government.
    pub retro_economy_weight: f64,
    /// Retrospective signal per point of local unemployment change beyond the
    /// national change over a year (clamped to ±1).
    pub retro_unemployment_scale: f64,
    /// Media attention on a town's record: a base, plus its share of the region's people.
    pub media_base: f64,
    pub media_town_share: f64,
    /// Weight of voters' approval of the player in the player's own races.
    pub player_approval_weight: f64,
    /// Local viability of a candidate without a party.
    pub independent_viability: f64,
    /// Starting awareness: the player as incumbent and other sitting
    /// officeholders in town (shares of `incumbent_awareness`), the bonus in a
    /// candidate's home neighbourhood, and the cap.
    pub player_incumbent_awareness: f64,
    pub officeholder_awareness: f64,
    pub home_awareness_bonus: f64,
    pub awareness_cap: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TurnoutParams {
    /// Turnout multiplier for a standalone local election.
    pub standalone: f64,
    pub concurrent: f64,
    /// Weight on last election's turnout (habit).
    pub habit: f64,
    /// Extra turnout from a fully mobilised neighbourhood.
    pub mobilisation_max: f64,
    /// After an election, each cohort's habit is its neighbourhood's turnout ×
    /// (`habit_base` + `habit_propensity` × its own turnout propensity).
    pub habit_base: f64,
    pub habit_propensity: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PollParams {
    pub pollsters: Vec<String>,
    pub sample_size: u32,
    /// House-effect standard deviation (share points, 0..1 scale).
    pub house_sd: f64,
    /// Election-wide shared error on election eve (share, 0..1 scale).
    pub shared_sd: f64,
    /// Shared error multiplier at the start of the campaign.
    pub early_multiplier: f64,
    pub every_days: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CampaignParams {
    pub persuasion_half_life_days: f64,
    pub awareness_per_canvass: f64,
    pub persuasion_per_canvass: f64,
    pub mobilisation_per_canvass: f64,
    /// Each repeated contact in a neighbourhood is worth this fraction of the last.
    pub diminishing: f64,
    pub ad_cost: i64,
    pub awareness_per_ad: f64,
    /// NPC campaign effort in abstracted races (contacts per week per candidate).
    pub npc_contacts_per_week: f64,
    /// Abstracted NPC effort scales with `npc_effort_base` + oratory.
    pub npc_effort_base: f64,
    /// Share of recent contacts remembered each week (diminishing returns recover).
    pub contact_memory: f64,
    /// Each further ad in a turn is worth this fraction of the last.
    pub ad_saturation: f64,
    /// Persuasion of one ad buy, as a share of one canvass contact's.
    pub ad_persuasion_share: f64,
}

/// Mover–stayer party attachment (DEC-071 §3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PartyIdParams {
    /// Share of adults attached to any party, by age band (young → old).
    pub attached_by_age: [f64; 4],
    /// Durable share of attachment.
    pub stayer_share: f64,
    /// Yearly pull of the mover share toward current evaluations.
    pub mover_rate: f64,
    /// Sharpness of attachment by ideological distance.
    pub sharpness: f64,
    /// Credit or blame for the party running the town hall: attachment to it
    /// scales by 1 + (approval − 0.5) × this.
    pub governing_credit: f64,
}
