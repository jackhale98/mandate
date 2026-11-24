//! Indicators and concerns (DEC-078 decision 3) and overheard talk (DEC-079).

use super::*;

// --- Indicators and concerns (DEC-078 decision 3) --------------------------------

/// Engine-provided measures of a municipality, recomputed monthly. Concerns
/// read them; situational policies (DEC-077) will use them as unlock
/// conditions. Adding one is a code extension point (one variant, one
/// computation in `mandate_sim::concerns::indicator`, and a test).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Indicator {
    /// Unemployment rate (0..1).
    Unemployment,
    /// Change in the unemployment rate over the last year (points, 0..1 scale).
    UnemploymentRise,
    /// Annual rent of an average home as a share of a typical (middle-class) income.
    RentBurden,
    /// The region's pollution index (0 clean, about 0.2 at start, up to 3).
    Pollution,
    /// Annual property tax on an average home as a share of a typical income.
    TaxBurden,
    /// The town's debt as a share of its annual budget.
    BudgetStress,
    /// Services delivered against need, as a shortfall: 1 − (the town's
    /// discretionary budget in real terms per head, against the level its
    /// budget rule aims for) − the service gains of laws in effect. 0 at
    /// the start; above 0 after cuts, below 0 with better services.
    ServiceShortfall,
    /// 1 just after a disaster, fading to 0 over two years.
    RecentDisaster,
    /// Share of the town's jobs lost over the last year (0 when jobs grew):
    /// a plant closure or a failing industry.
    JobLoss,
}

impl Indicator {
    pub const ALL: [Indicator; 9] = [
        Indicator::Unemployment,
        Indicator::UnemploymentRise,
        Indicator::RentBurden,
        Indicator::Pollution,
        Indicator::TaxBurden,
        Indicator::BudgetStress,
        Indicator::ServiceShortfall,
        Indicator::RecentDisaster,
        Indicator::JobLoss,
    ];
}

/// A voter concern (DEC-078 decision 3): something that worries a town,
/// read from an indicator, and the issues (attitudes) it makes voters weigh.
/// Registry order is load order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConcernDef {
    /// Content id, e.g. `concern.jobs`.
    pub id: ContentKey,
    /// Fluent key of the concern's name, e.g. `concern-jobs`.
    pub name_key: String,
    /// Fluent key of the concern as people name it in talk: a lower-case
    /// noun phrase for mid-sentence use ("the rent"), as sectors have; the
    /// name is used without one.
    #[serde(default)]
    pub talk_key: Option<String>,
    pub indicator: Indicator,
    /// The attitudes behind the concern, by key. The magnitude is how much a
    /// worried town weighs that issue; the sign is the side worried voters
    /// lean toward (used to rank concerns by neighbourhood).
    pub attitudes: BTreeMap<String, f64>,
    /// `attitudes` resolved to a dense vector in attitude-registry order at
    /// load. Not part of the content format.
    #[serde(skip)]
    #[schemars(skip)]
    pub attitude_weights: Vec<f64>,
    /// How the indicator's value becomes the concern's level.
    pub level: ConcernLevel,
}

/// `level = clamp(baseline + slope × indicator, min, max)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConcernLevel {
    pub baseline: f64,
    pub slope: f64,
    pub min: f64,
    pub max: f64,
}

impl ConcernLevel {
    pub fn of(&self, indicator: f64) -> f64 {
        (self.baseline + self.slope * indicator).clamp(self.min, self.max)
    }
}

// --- Overheard talk (DEC-079 information services) -------------------------------

/// One thing residents may be heard saying at a diner, market, or union hall
/// (the "overheard" service). Each week every such place draws a few lines
/// that hold for its neighbourhood: lines about the neighbourhood's top
/// concerns, and opinions and news grounded in facts. Registry order is
/// load order. Fluent arguments always include `$town` and `$hood`; each
/// condition adds its own (see `condition::Condition::provides`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OverheardDef {
    /// Content id, e.g. `overheard.jobs.hiring`.
    pub id: ContentKey,
    /// Fluent key of the line.
    pub key: String,
    /// Heard only where this concern is among the neighbourhood's top three
    /// (a concern id); the stronger the concern, the likelier the line.
    #[serde(default)]
    pub concern: Option<ContentKey>,
    /// Every condition must hold.
    #[serde(default)]
    pub when: Vec<crate::condition::Condition>,
    /// Relative chance among the lines that hold (default 1).
    #[serde(default = "one_f64")]
    pub weight: f64,
    /// What hearing the line lets the player hear of (DEC-091): each subject
    /// is raised to `heard`. Lines have no roles; a neighbourhood or a place
    /// is where the line is heard.
    #[serde(default)]
    pub teaches: Vec<crate::narrative::SubjectDef>,
}

fn one_f64() -> f64 {
    1.0
}
