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
    /// discretionary budget lines other than Education and Infrastructure,
    /// in real terms per head, against the level its budget rule aims for)
    /// − the service gains of laws in effect on lines other than Education
    /// and Health. 0 at the start; above 0 after cuts, below 0 with better
    /// services. Schools and health care (`CareShortfall`) and the streets
    /// (`UpkeepShortfall`) were split out of it (DEC-106).
    ServiceShortfall,
    /// 1 just after a disaster, fading to 0 over two years.
    RecentDisaster,
    /// Share of the town's jobs lost over the last year (0 when jobs grew):
    /// a plant closure or a failing industry.
    JobLoss,
    /// Prices against real wages (DEC-106): the share of its real value
    /// each income class's income (wages, benefits and transfers over the
    /// price level) lost over the last year (1 − now ÷ a year ago; below 0
    /// when incomes beat prices), weighted by the adults of each class and
    /// how strongly the class feels prices (`signals.cost_of_living_class`).
    /// A neighbourhood's reads its own residents' classes.
    CostOfLiving,
    /// Newcomers (DEC-106): adults who moved in from other towns over the
    /// last year, as a share of the town's adults. Arrivals settle by adult
    /// count, so every neighbourhood of a town has the same rate.
    Arrivals,
    /// Young people leaving (DEC-106): 18–29s who moved away over the last
    /// year less those who moved in, as a share of the town's 18–29s (below
    /// 0 when more came than went).
    YoungLeaving,
    /// An ageing town (DEC-106): the share of adults aged 65 and over, less
    /// the nation's share (a neighbourhood's: its own share less the
    /// nation's).
    OldAgeShare,
    /// Schools and health care against need (DEC-106), as a shortfall: 1 −
    /// the town's Education line and its region's Health line (its share,
    /// by population) against their need, − the gains of laws on those
    /// lines. Felt more where more residents are children or 65 and over.
    CareShortfall,
    /// The Infrastructure line against need, as a shortfall: 1 − min(line ÷
    /// need, `faults.params.maintain_cap`), the rate at which upkeep keeps
    /// the streets up.
    UpkeepShortfall,
    /// The streets' wear where upkeep keeps a condition (DEC-106 item 3,
    /// B2's `UpkeepState`): the midpoint of `faults.params.start` − the
    /// condition (a neighbourhood's own; the town's mean by residents);
    /// 0 where no condition is kept.
    StreetWear,
    /// Scandals and cases (DEC-106): the town's politicians' scandals that
    /// broke within `signals.scandal_days`, by gravity and fading with age,
    /// plus their open and recent cases (`signals`).
    Scandals,
    /// Construction pressure (DEC-106): homes under construction as a share
    /// of the town's stock. A neighbourhood feels it by how much faster it
    /// has grown than its town since play began.
    Construction,
    /// Disruption from works being built (laws and works projects): the
    /// town's share plus the neighbourhood's own (the town's: its own plus
    /// its neighbourhoods' mean by residents).
    WorksDisruption,
}

impl Indicator {
    pub const ALL: [Indicator; 19] = [
        Indicator::Unemployment,
        Indicator::UnemploymentRise,
        Indicator::RentBurden,
        Indicator::Pollution,
        Indicator::TaxBurden,
        Indicator::BudgetStress,
        Indicator::ServiceShortfall,
        Indicator::RecentDisaster,
        Indicator::JobLoss,
        Indicator::CostOfLiving,
        Indicator::Arrivals,
        Indicator::YoungLeaving,
        Indicator::OldAgeShare,
        Indicator::CareShortfall,
        Indicator::UpkeepShortfall,
        Indicator::StreetWear,
        Indicator::Scandals,
        Indicator::Construction,
        Indicator::WorksDisruption,
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
    /// More signals behind the same worry, each adding `slope` × its
    /// indicator to the level before the clamp (an ageing town and its
    /// young people leaving; the streets' line and their wear).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also: Vec<ConcernTerm>,
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

/// A further signal behind a worry: `slope` × `indicator`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConcernTerm {
    pub indicator: Indicator,
    pub slope: f64,
}

impl ConcernDef {
    /// Every indicator the worry reads: its own, then `also`'s.
    pub fn indicators(&self) -> impl Iterator<Item = Indicator> + '_ {
        std::iter::once(self.indicator).chain(self.also.iter().map(|t| t.indicator))
    }

    /// The worry's level from its indicators' values (`value` of each):
    /// `clamp(baseline + slope × indicator + Σ also's slope × indicator,
    /// min, max)`.
    pub fn level_from(&self, value: impl Fn(Indicator) -> f64) -> f64 {
        let l = &self.level;
        let raw = l.baseline
            + l.slope * value(self.indicator)
            + self
                .also
                .iter()
                .map(|t| t.slope * value(t.indicator))
                .sum::<f64>();
        raw.clamp(l.min, l.max)
    }
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
