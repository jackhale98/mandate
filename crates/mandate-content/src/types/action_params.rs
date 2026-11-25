//! Balance blocks for player action effects and risks (DEC-073).

use super::*;

/// Player action effects and risks (DEC-073). Time and health costs are in
/// `tuning.action_costs`; record gains in `career.record_gain`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionParams {
    /// Health a late-night action costs on top of its own cost.
    pub late_night_health: f64,
    pub vote: CouncilVoteEffects,
    pub amend: AmendEffects,
    pub introduce: IntroduceEffects,
    pub lobby: LobbyEffects,
    pub town_hall: TownHallEffects,
    pub press: PressEffects,
    pub canvass: CanvassEffects,
    pub fundraise: FundraiseEffects,
    /// The `visit` action: its verbs and numbers (places with purpose).
    #[serde(default)]
    pub visit: crate::visits::VisitParams,
}

/// The player's declared council vote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CouncilVoteEffects {
    /// Standing for voting with the party line, against it, and abstaining.
    pub standing_loyal: f64,
    pub standing_defy: f64,
    pub standing_abstain: f64,
    /// Ward approval per unit of alignment × (0.5 + bill awareness) × room.
    pub ward_reaction: f64,
    /// Ward approval change for abstaining.
    pub ward_abstain: f64,
    /// The sponsor's opinion of the player: a yes, a no, and a no while they
    /// are owed a favour.
    pub sponsor_yes: f64,
    pub sponsor_no: f64,
    pub sponsor_no_owed: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AmendEffects {
    /// Share of the goodwill already built on a bill that survives an amendment.
    pub sway_keep: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IntroduceEffects {
    /// Public awareness of a bill when it is introduced (player and NPCs).
    pub awareness: f64,
}

/// The player's lobbying (the odds and the sway gained are in `council.lobby`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LobbyEffects {
    /// Sway bought by trading a favour.
    pub trade_gain: f64,
    /// The target's opinion of the player after a success and a failure.
    pub opinion_success: f64,
    pub opinion_fail: f64,
    /// Legal jeopardy of a late-night deal.
    pub late_night_legal: f64,
    /// Lobbying as a conversation (M6 note 29): the arguments the player
    /// can make, beside the plain pitch.
    pub arguments: LobbyArguments,
}

/// The arguments of a lobbying conversation (M6 note 29). Each replaces
/// the plain pitch's ties term (the member's opinion of the player /
/// `council.lobby.opinion_scale`) in the lobbying logit with its own.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LobbyArguments {
    /// Lobbying exchanges with one member on one bill a week.
    pub exchanges: u32,
    /// Their ward: × the ward's support for the bill (−1..1, on the
    /// player's side).
    pub ward: f64,
    /// Their party's line (on the player's side): × the member's loyalty.
    pub party: f64,
    /// The player's word (an ask of theirs kept): × (2 × trust − 1).
    pub word: f64,
    /// The broker (who wants what the player wants): × the member's
    /// compliance with the broker (0..1).
    pub broker: f64,
    /// An ally or the party whip brings others: sway per member ×
    /// (their opinion of the one asked / 100, plus their loyalty if in the
    /// same party), clamped to 0..1.
    pub rally_sway: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TownHallEffects {
    /// Awareness gained: `awareness` × (`awareness_oratory_base` + oratory).
    pub awareness: f64,
    pub awareness_oratory_base: f64,
    /// Approval nudge per unit of alignment with the bill discussed, and
    /// without a bill.
    pub bill_nudge: f64,
    pub nudge: f64,
    /// Approval nudge from a hostile crowd.
    pub hostile_nudge: f64,
    /// Bill awareness gained: `bill_awareness` × (0.5 + oratory).
    pub bill_awareness: f64,
    /// Campaign contact: (`contact_base` + `contact_oratory` × oratory), ×
    /// `hostile_contact` when hostile. NPC town halls use the same scale.
    pub contact_base: f64,
    pub contact_oratory: f64,
    pub hostile_contact: f64,
    /// Chance of a hostile crowd: `risk_base` − `risk_approval` × the
    /// player's net approval − `risk_government` × (government approval −
    /// 0.5), within [`risk_min`, `risk_max`].
    pub risk_base: f64,
    pub risk_approval: f64,
    pub risk_government: f64,
    pub risk_min: f64,
    pub risk_max: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PressEffects {
    /// Town-wide awareness gained.
    pub awareness: f64,
    /// Approval lost to a gaffe, per unit of awareness.
    pub gaffe_approval: f64,
    /// Bill awareness gained.
    pub bill_awareness: f64,
    /// Campaign contact scale (everywhere in the race), unless there was a gaffe.
    pub contact: f64,
    /// Gaffe risk: `gaffe_base` + `gaffe_oratory` × (1 − oratory), within
    /// [`gaffe_min`, `gaffe_max`].
    pub gaffe_base: f64,
    pub gaffe_oratory: f64,
    pub gaffe_min: f64,
    pub gaffe_max: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CanvassEffects {
    /// Share of the remaining awareness gap closed (× `max(fresh,
    /// awareness_floor)`).
    pub awareness: f64,
    /// Approval gained per unit of room (× fit × fresh × season × same turn).
    pub approval: f64,
    /// Campaign contact scale (a full day on the doorsteps; × the same-turn
    /// cut).
    pub contact: f64,
    /// Freshness per remembered visit to a neighbourhood (DEC-097 §1,
    /// ENGAGEMENT §2.1): `fresh = hood_decay ^ visits`.
    pub hood_decay: f64,
    /// The same in campaign season, when doorsteps are expected.
    pub hood_decay_campaign: f64,
    /// Share of a neighbourhood's remembered visits kept each month (30 days).
    pub keep: f64,
    /// Approval multiplier while the player is a candidate in an open race.
    pub campaign_mult: f64,
    /// Approval and contact multiplier for a second canvass in one turn.
    pub same_turn_mult: f64,
    /// Awareness is built at least at this share of the full rate, however
    /// stale the doorsteps (canvassing stays the way to be known).
    pub awareness_floor: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FundraiseEffects {
    /// Money raised: uniform in [min, max] × (0.5 + oratory). NPCs too.
    pub raised: [f64; 2],
    /// Standing gained (the party likes a fundraiser).
    pub standing: f64,
    /// Legal jeopardy of late-night fundraising.
    pub late_night_legal: f64,
    /// Approval lost among low-income voters, per unit of awareness.
    pub donor_ties: f64,
}
