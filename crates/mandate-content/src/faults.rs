//! Upkeep made visible (DEC-114 §3, `BEING_THERE.md` §4): a neighbourhood's
//! condition shows as **faults at spots** (a light out, a blocked drain, a
//! pothole), repaired in a queue paid from one line of the town's budget.
//!
//! The file `faults.jsonc` holds the upkeep rules (`params`) and the fault
//! kinds (`faults`). With no fault kinds the upkeep domain stays empty and
//! nothing else changes: no condition is kept, no fault opens, no ask is cast
//! on one and no worry reads one.
//!
//! A spot is a slot kind on a neighbourhood's streets with a number of slots
//! (like doors: the client maps it to a cell). Nothing here names an office
//! or a body; the line that pays is a budget category of the town's own
//! treasury (DEC-098).

use crate::bundle::ContentBundle;
use crate::types::{BudgetCategory, ContentKey};
use crate::validate::{Issue, valid_key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Where on a neighbourhood's streets a fault is (the client maps a kind
/// and a slot to a cell of the street).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SpotKind {
    /// The carriageway: potholes, a sunken manhole.
    Road,
    /// The pavement and what stands on it: lamps, signs, bins.
    Pavement,
    /// The kerb and the gutter: drains.
    Kerb,
    /// A bus stop or a shelter.
    Stop,
    /// A green, a play area, a bench.
    Green,
}

impl SpotKind {
    pub fn key(self) -> &'static str {
        match self {
            SpotKind::Road => "road",
            SpotKind::Pavement => "pavement",
            SpotKind::Kerb => "kerb",
            SpotKind::Stop => "stop",
            SpotKind::Green => "green",
        }
    }
}

/// A fault's spot kind and how many such spots a neighbourhood has.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpotDef {
    pub kind: SpotKind,
    /// Spots of this kind on a neighbourhood's streets (1–16).
    pub slots: u8,
}

/// When a fault opens: each month, in each neighbourhood whose condition is
/// below `below`, with the chance `chance × (below − condition) / below`,
/// × `winter` in December, January and February.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RaiseDef {
    /// Condition (0–1) below which it can open.
    pub below: f64,
    /// The monthly chance at a condition of 0 (0–1).
    pub chance: f64,
    /// × in winter (1: no season).
    #[serde(default = "one")]
    pub winter: f64,
}

fn one() -> f64 {
    1.0
}

/// What a repair costs (money at the starting price level; the engine
/// scales it by prices) and how long it takes once paid for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RepairDef {
    pub cost: f64,
    pub days: u32,
}

/// The street cues of a fault (S7; drawn by the client in B4): while it is
/// open, while it is being repaired, and once repaired (for `new_days`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CueDef {
    pub open: String,
    pub repairing: String,
    pub new: String,
}

/// One kind of fault (`fault.<name>`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FaultDef {
    pub id: ContentKey,
    /// What it is on (`street_light`, `drain`): the asset, for cues.
    pub asset: String,
    pub spot: SpotDef,
    pub raise: RaiseDef,
    /// The budget line of the town's treasury that repairs it.
    pub pays: BudgetCategory,
    pub repair: RepairDef,
    /// Condition (0–1) the neighbourhood gains when it is repaired.
    pub restore: f64,
    /// The worry it adds to while open (a concern id).
    pub feeds: ContentKey,
    pub cue: CueDef,
    /// Its Fluent prefix: `<text>name` ("the street light"), `<text>what`
    /// ("the light on { $hood }'s streets is out").
    pub text: String,
}

/// Upkeep's rules.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpkeepParams {
    /// A neighbourhood's condition when the engine first keeps it: a keyed
    /// value in this range.
    pub start: [f64; 2],
    /// Condition lost each month.
    pub decay: f64,
    /// Condition kept up each month by routine maintenance, × the
    /// Infrastructure line against need (1 at need; at most `maintain_cap`).
    pub maintain: f64,
    pub maintain_cap: f64,
    /// Open faults a neighbourhood can have at once.
    pub max_open: u8,
    /// The share of a budget line's month that pays its repair queue.
    pub repair_share: f64,
    /// Added to a neighbourhood's level of the worry a fault feeds, for
    /// each such fault open there.
    pub worry_per_fault: f64,
    /// Days a repaired spot looks new (then the fault is forgotten).
    pub new_days: u32,
}

impl Default for UpkeepParams {
    fn default() -> Self {
        UpkeepParams {
            start: [0.6, 0.8],
            decay: 0.0,
            maintain: 0.0,
            maintain_cap: 1.0,
            max_open: 0,
            repair_share: 0.0,
            worry_per_fault: 0.0,
            new_days: 0,
        }
    }
}

/// The `faults` file.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpkeepDef {
    pub params: UpkeepParams,
    #[serde(default)]
    pub faults: Vec<FaultDef>,
}

impl ContentBundle {
    pub fn fault(&self, id: &str) -> Option<&FaultDef> {
        self.upkeep.faults.iter().find(|f| f.id == id)
    }
}

/// Shape and reference rules the schema cannot express.
pub fn validate(b: &ContentBundle, issues: &mut Vec<Issue>) {
    let p = &b.upkeep.params;
    let unit = |x: f64| (0.0..=1.0).contains(&x);
    if !b.upkeep.faults.is_empty() {
        let mut err = |m: &str| issues.push(Issue::error("faults", m));
        if !(unit(p.start[0]) && unit(p.start[1]) && p.start[0] <= p.start[1]) {
            err("`start` is a range within 0–1");
        }
        if !(unit(p.decay) && unit(p.maintain) && p.maintain_cap >= 0.0) {
            err("`decay` and `maintain` are 0–1 a month; `maintain_cap` is not negative");
        }
        if p.max_open == 0 {
            err("`max_open` must allow at least one open fault");
        }
        if !unit(p.repair_share) {
            err("`repair_share` is a share (0–1)");
        }
        if !(0.0..=0.5).contains(&p.worry_per_fault) {
            err("`worry_per_fault` is 0–0.5");
        }
    }
    for (i, f) in b.upkeep.faults.iter().enumerate() {
        let id = f.id.as_str();
        let mut err = |m: &str| issues.push(Issue::error(id, m));
        if !valid_key(id) || !id.starts_with("fault.") {
            err("a fault id must be a key starting with `fault.`");
        }
        if b.upkeep.faults[..i].iter().any(|x| x.id == f.id) {
            err("duplicate fault id");
        }
        if f.asset.is_empty() || f.text.is_empty() || !f.text.ends_with('-') {
            err("a fault needs an asset and a Fluent prefix ending in `-`");
        }
        if !(1..=16).contains(&f.spot.slots) {
            err("a spot kind has 1 to 16 slots");
        }
        if !(unit(f.raise.below) && unit(f.raise.chance) && f.raise.winter >= 0.0) {
            err("`raise`: `below` and `chance` are 0–1, `winter` is not negative");
        }
        if !(f.repair.cost > 0.0 && f.repair.cost.is_finite()) || f.repair.days == 0 {
            err("a repair costs something and takes at least a day");
        }
        if !unit(f.restore) {
            err("`restore` is 0–1");
        }
        if !b.concerns.iter().any(|c| c.id == f.feeds) {
            err("`feeds` must name a concern");
        }
        if f.cue.open.is_empty() || f.cue.repairing.is_empty() || f.cue.new.is_empty() {
            err("a fault names its three cues");
        }
    }
}
