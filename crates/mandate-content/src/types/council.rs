//! Balance blocks for councils, lobbying, careers and NPCs (DEC-026, DEC-071).

use super::*;

/// How council members lean on bills, how mayors sign, and lobbying odds
/// (DEC-026, DEC-071 §7). Shared by the player and NPCs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CouncilParams {
    /// A member's lean: own values (weighted by 1 − pragmatism), the ward's
    /// support (by pragmatism), and the mayor's lean.
    pub own_values: f64,
    pub ward_support: f64,
    pub mayor: f64,
    /// Goodwill toward the player as sponsor, per 100 points of opinion.
    pub player_opinion: f64,
    /// Riders: a sunset, a pilot, each other ward's earmark, and an earmark
    /// for the member's own ward.
    pub sunset: f64,
    pub pilot: f64,
    pub foreign_earmark: f64,
    pub own_earmark: f64,
    /// Standard deviation of each member's idiosyncrasy per bill.
    pub noise_sd: f64,
    /// The mayor's lean on a bill: own values and the town's support.
    pub mayor_own: f64,
    pub mayor_public: f64,
    /// Logistic slope from a lean to the chance of a yes vote.
    pub vote_sharpness: f64,
    /// Mayors sign unless their lean is below −`sign_bias`.
    pub sign_bias: f64,
    /// Public awareness a bill gains at enactment, and the share it loses each month.
    pub enactment_awareness: f64,
    pub awareness_fade: f64,
    pub lobby: LobbyParams,
    /// Personality (DEC-031): party discipline × (1 + this × (2 × loyalty − 1)).
    pub loyalty_discipline: f64,
}

/// Lobbying odds and sway (player and NPCs alike).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LobbyParams {
    /// Skill = `skill_negotiation` × negotiation + `skill_oratory` × oratory.
    pub skill_negotiation: f64,
    pub skill_oratory: f64,
    /// Logit of success: `skill_scale` × (skill − 0.5) + `pragmatism` × the
    /// target's pragmatism + `bias` + ties (+ `late_night`).
    pub skill_scale: f64,
    pub pragmatism: f64,
    pub bias: f64,
    /// Ties with the player: the target's opinion of them divided by this.
    pub opinion_scale: f64,
    /// Ties between NPCs: the same party, or not.
    pub same_party: f64,
    pub other_party: f64,
    pub late_night: f64,
    /// Personality (DEC-031): − this × (target's integrity − 0.5) in the logit.
    pub integrity: f64,
    pub odds_min: f64,
    pub odds_max: f64,
    /// Sway gained on success: uniform in [min, max] × (0.5 + negotiation).
    pub gain: [f64; 2],
    /// Sway gained on failure.
    pub fail_gain: f64,
}

/// Careers (DEC-011, DEC-076): the record of visible work, the party's
/// patience, Mandate and viability, name recognition, and election results.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CareerParams {
    /// Record gained per piece of visible work (the player and NPCs).
    pub record_gain: RecordGains,
    /// Share of every record lost each month.
    pub record_fade: f64,
    /// NPC officeholders' monthly record gain per unit of diligence, and the
    /// weights of integrity and ambition in diligence.
    pub npc_diligence_rate: f64,
    pub diligence_integrity: f64,
    pub diligence_ambition: f64,
    /// Party patience: monthly Standing change below a thin record and above
    /// a strong one.
    pub thin_record: f64,
    pub thin_record_standing: f64,
    pub strong_record: f64,
    pub strong_record_standing: f64,
    /// Name recognition kept each month, and while the record is below
    /// `invisible_record`.
    pub invisible_record: f64,
    pub awareness_keep: f64,
    pub awareness_keep_invisible: f64,
    /// Mandate: monthly pull toward 50 + `mandate_per_net_approval` × net
    /// approval among the ward's voters.
    pub mandate_rate: f64,
    pub mandate_per_net_approval: f64,
    /// Viability: monthly pull toward a mix of Mandate and Standing (this
    /// share Standing).
    pub viability_rate: f64,
    pub viability_standing_share: f64,
    /// Health recovered each month.
    pub health_recovery: f64,
    /// After a win: Mandate = `win_mandate_base` + `win_mandate_per_margin` ×
    /// margin (at least `win_mandate_min`), and track gains.
    pub win_mandate_base: f64,
    pub win_mandate_per_margin: f64,
    pub win_mandate_min: f64,
    pub win_viability: f64,
    pub win_standing: f64,
    /// After a loss: viability changes by `loss_viability` +
    /// `loss_viability_per_margin` × margin (margin ≥ −0.5), and Standing.
    pub loss_viability: f64,
    pub loss_viability_per_margin: f64,
    pub loss_standing: f64,
    /// Share of scandal damage kept each year.
    pub scandal_keep_yearly: f64,
    /// Officeholders' yearly war-chest income: base + per unit of oratory.
    pub officeholder_funds: f64,
    pub officeholder_funds_oratory: f64,
    /// What a party asks before it nominates the player for a post, keyed
    /// by role word (an office's id or a body's `member_key`; DEC-098).
    /// A post with no entry has no gate.
    #[serde(default)]
    pub nomination: BTreeMap<String, NominationGate>,
}

/// A party's conditions for nominating the player for a post (DEC-098).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NominationGate {
    /// The player's Standing with the party must be at least this.
    #[serde(default)]
    pub min_standing: Option<f64>,
}

impl NominationGate {
    /// Whether a player with this Standing passes the gate.
    pub fn open(&self, standing: f64) -> bool {
        self.min_standing.is_none_or(|m| standing >= m)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordGains {
    pub introduce: f64,
    pub amend: f64,
    pub vote: f64,
    pub lobby: f64,
    pub town_hall: f64,
    /// Presence, not delivery (DEC-097 §1).
    pub canvass: f64,
    pub press: f64,
    /// Delivery (ENGAGEMENT §2.2): an ask kept, a campaign promise kept and
    /// an ask broken (negative), read by the asks domain (E6); the player's
    /// own bill becoming law, credited with V1.
    pub ask_kept: f64,
    pub promise_kept: f64,
    pub ask_broken: f64,
    pub law_enacted: f64,
}

/// NPC AI balance numbers (DEC-030). Scores and curves are in `politics.ai`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NpcParams {
    /// Strategic profile from traits: an ideologue below this pragmatism,
    /// else a populist above this ambition, else a pragmatist.
    pub ideologue_below: f64,
    pub populist_above: f64,
    /// A race is a dead heat at 0 and not close at this poll margin.
    pub close_margin: f64,
    /// Funds count as full at this many ad buys.
    pub funds_in_ads: f64,
    /// The council is busy at this many pending bills, or this many laws in force.
    pub busy_pending: f64,
    pub busy_laws: f64,
    /// At most one bill of one's own in this many days.
    pub bill_interval_days: u32,
    /// Whip counts: yes votes of insurance wanted beyond the threshold, the
    /// gap at which a count stops being close, and the weight of a count
    /// already on the right side.
    pub insurance_votes: f64,
    pub safe_gap: f64,
    pub comfortable_weight: f64,
    /// Planning within a turn: closeness kept after lobbying, and the unknown
    /// share kept after a contact.
    pub lobby_closeness_keep: f64,
    pub contact_unknown_keep: f64,
    /// Ads: two buys above this ad score, one above this, and the reserve
    /// (in ad costs) a candidate keeps.
    pub two_ads_above: f64,
    pub one_ad_above: f64,
    pub ad_reserve: f64,
    /// NPC bills sunset after this many years, [min, max].
    pub bill_sunset_years: [u8; 2],
    /// Agendas follow the town's character (DEC-077 §1): ranking its ideas, an
    /// NPC adds this × severity to a template that answers the live crisis,
    /// and `concern_agenda` × fit to one that addresses the town's concerns
    /// (alignment with the concerns' attitudes, weighted by their levels).
    /// The bonus changes which idea comes first, not how keen it is.
    pub crisis_agenda: f64,
    pub concern_agenda: f64,
}
