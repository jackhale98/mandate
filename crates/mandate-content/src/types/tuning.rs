//! Data-driven tuning (REQ-005): action costs and turn lengths.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActionCost {
    pub slots: u8,
    #[serde(default)]
    pub health: f64,
}

impl Tuning {
    pub fn action_cost(&self, action: &str) -> ActionCost {
        self.action_costs
            .get(action)
            .copied()
            .unwrap_or(ActionCost {
                slots: 1,
                health: 0.0,
            })
    }
}

/// Data-driven tuning parameters (REQ-005).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Tuning {
    /// Standard day-part slots per turn (Morning, Afternoon, Evening).
    pub standard_slots: u8,
    /// Whether the optional Late Night slot is offered.
    pub late_night_enabled: bool,
    /// Turn lengths in days by phase (DEC-012).
    pub turn_days: TurnDays,
    /// Years of NPC-only simulation before the player starts (DEC-022).
    pub burn_in_years: u8,
    /// Calendar year the history begins.
    pub start_year: u32,
    /// Time and health each action costs (DEC-073). Missing actions cost one
    /// slot and no health; negative health is recovery.
    #[serde(default)]
    pub action_costs: std::collections::BTreeMap<String, ActionCost>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TurnDays {
    pub campaign: u8,
    /// Days before an election that switch to daily turns.
    pub final_stretch: u8,
    pub governing_local: u8,
    pub governing_regional: u8,
    pub governing_national: u8,
}
