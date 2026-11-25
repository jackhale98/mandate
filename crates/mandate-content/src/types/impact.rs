//! The impact table (DEC-111, `docs/design/IMPACT_ESTIMATE.md` §2): what
//! reaches a town of what a law does directly. The estimate before a vote is
//! the law's direct quantities (the simulation's own arithmetic) times these
//! pass-through coefficients, which `mandate-cli lab impact calibrate` fits
//! on twins and writes to `impact.jsonc`. Nothing here is written by hand
//! except `tiers` and `arithmetic`.

use super::*;

/// Months after enactment at which an estimate is stated (the lab's samples).
pub const IMPACT_HORIZONS: [u32; 3] = [12, 36, 60];

/// What a coefficient multiplies: a direct quantity of the bill, as a share
/// of the town (so one row serves any town size). One per mechanism kind
/// whose effect on the town the lab measures, `spend` for what all
/// spending does, and `law` for what any law does.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ImpactBasis {
    /// 1 for every bill: what passing any law does, whatever is in it. It
    /// takes a place on the statute book (a bill that would have passed
    /// does not), and no two futures are alike: its range is how far a
    /// town with one more law strays from the same town without, before
    /// the law's own content counts. Outcomes as shares of the town.
    Law,
    /// What the bill takes from the budget a year, over the budget.
    Spend,
    /// Homes a programme adds by the ledger's own count (`homes_added`),
    /// over the housing stock.
    HousingSupply,
    /// Homes private building adds under looser zoning (`homes_added`),
    /// over the stock.
    Zoning,
    /// The share by which a cap holds rents below the market (`rent_held_pct`).
    RentCap,
    /// Jobs a wage subsidy or a programme adds in its sectors by the
    /// ledger's count (`jobs_added_*`), over all jobs (over the sector's,
    /// for the outcome `sector_jobs`). Fed by `sector_subsidy` and
    /// `sector_capacity` alike.
    JobsAdded,
    /// The share by which a wage floor raises low incomes by the ledger's
    /// count (`low_income_added_pct`).
    WageFloor,
    /// A year's payment to an adult of the class, over that class's income.
    Transfer,
    /// The service index gained (`service_gain_pct`), as a share.
    Service,
    /// A year's revenue of the tax change, over the budget (negative for a cut).
    LocalTaxRate,
}

impl ImpactBasis {
    pub const ALL: [ImpactBasis; 10] = [
        ImpactBasis::Law,
        ImpactBasis::Spend,
        ImpactBasis::HousingSupply,
        ImpactBasis::Zoning,
        ImpactBasis::RentCap,
        ImpactBasis::JobsAdded,
        ImpactBasis::WageFloor,
        ImpactBasis::Transfer,
        ImpactBasis::Service,
        ImpactBasis::LocalTaxRate,
    ];

    /// Its key as content writes it.
    pub fn key(self) -> &'static str {
        match self {
            ImpactBasis::Law => "law",
            ImpactBasis::Spend => "spend",
            ImpactBasis::HousingSupply => "housing_supply",
            ImpactBasis::Zoning => "zoning",
            ImpactBasis::RentCap => "rent_cap",
            ImpactBasis::JobsAdded => "jobs_added",
            ImpactBasis::WageFloor => "wage_floor",
            ImpactBasis::Transfer => "transfer",
            ImpactBasis::Service => "service",
            ImpactBasis::LocalTaxRate => "local_tax_rate",
        }
    }

    /// The mechanism kinds that feed it (none for `law` and `spend`: every
    /// bill is a law, and every bill's cost is spending).
    pub fn kinds(self) -> &'static [&'static str] {
        match self {
            ImpactBasis::Law | ImpactBasis::Spend => &[],
            ImpactBasis::HousingSupply => &["housing_supply"],
            ImpactBasis::Zoning => &["zoning"],
            ImpactBasis::RentCap => &["rent_cap"],
            ImpactBasis::JobsAdded => &["sector_subsidy", "sector_capacity"],
            ImpactBasis::WageFloor => &["wage_floor"],
            ImpactBasis::Transfer => &["transfer"],
            ImpactBasis::Service => &["service"],
            ImpactBasis::LocalTaxRate => &["local_tax_rate"],
        }
    }

    /// The basis a mechanism feeds, if the lab measures one for it.
    pub fn of(m: &MechanismDef) -> Option<ImpactBasis> {
        ImpactBasis::ALL
            .into_iter()
            .find(|b| b.kinds().contains(&m.key()))
    }
}

/// An outcome the simulation computes each month and the lab samples. Every
/// one is a change against the same town without the law, as a share
/// (approval: of the 0..1 scale, so 0.01 is a point).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ImpactOutcome {
    /// Homes in the jurisdiction, net of private building displaced.
    Homes,
    /// Jobs in the jurisdiction, net.
    Jobs,
    /// Jobs in a sector a mechanism names.
    SectorJobs,
    /// Typical rent.
    Rent,
    /// Income per adult on low incomes.
    LowIncome,
    /// The treasury's debt, over its yearly budget.
    Debt,
    /// The service level (budget against need plus laws' gains).
    Services,
    /// Approval of the government.
    Approval,
    ApprovalLow,
    ApprovalMiddle,
    ApprovalHigh,
}

impl ImpactOutcome {
    pub const ALL: [ImpactOutcome; 11] = [
        ImpactOutcome::Homes,
        ImpactOutcome::Jobs,
        ImpactOutcome::SectorJobs,
        ImpactOutcome::Rent,
        ImpactOutcome::LowIncome,
        ImpactOutcome::Debt,
        ImpactOutcome::Services,
        ImpactOutcome::Approval,
        ImpactOutcome::ApprovalLow,
        ImpactOutcome::ApprovalMiddle,
        ImpactOutcome::ApprovalHigh,
    ];

    pub fn key(self) -> &'static str {
        match self {
            ImpactOutcome::Homes => "homes",
            ImpactOutcome::Jobs => "jobs",
            ImpactOutcome::SectorJobs => "sector_jobs",
            ImpactOutcome::Rent => "rent",
            ImpactOutcome::LowIncome => "low_income",
            ImpactOutcome::Debt => "debt",
            ImpactOutcome::Services => "services",
            ImpactOutcome::Approval => "approval",
            ImpactOutcome::ApprovalLow => "approval_low",
            ImpactOutcome::ApprovalMiddle => "approval_middle",
            ImpactOutcome::ApprovalHigh => "approval_high",
        }
    }
}

/// One fitted coefficient: across the lab's twins, the 10th, 50th and 90th
/// percentile of (the outcome's change, less what the bill's other bases
/// explain) ÷ this basis, `months` after enactment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImpactRow {
    pub basis: ImpactBasis,
    pub outcome: ImpactOutcome,
    pub months: u32,
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
    /// Twins the row was fitted on.
    pub n: u32,
}

/// How the table was fitted (what `lab impact calibrate` ran), so a reader
/// knows how much it rests on.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImpactFit {
    /// Seeds per world, counted from `first_seed`.
    pub seeds: u32,
    pub first_seed: u64,
    /// Worlds: `none` (a random country) and scenario ids.
    pub worlds: Vec<String>,
    /// Parameter levels each template was enacted at: `default`, `max`.
    pub levels: Vec<String>,
    /// Months at which laws were enacted.
    pub enacted_at: Vec<u32>,
    /// `home`, or `classes` (the home town and one town of each other class).
    pub towns: String,
    /// Twins run (template × level × town × month × world).
    pub twins: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImpactTable {
    /// Tiers the rows were calibrated for. A bill of another tier gets no
    /// estimate, not a guess (DEC-098).
    pub tiers: Vec<TierKey>,
    /// Mechanism and side-effect kinds the estimate states by arithmetic
    /// alone, with no coefficient: money (`borrow`, `revenue`), protection
    /// against a reference event (`hazard_protection`), and the side
    /// effects, whose size scales with their mechanism's in every template
    /// and so cannot be told apart from it on twins (their consequences are
    /// inside the mechanism's rows). Every other kind in use needs a row.
    pub arithmetic: Vec<String>,
    #[serde(default)]
    pub fitted: ImpactFit,
    pub rows: Vec<ImpactRow>,
}

impl ImpactTable {
    /// The coefficient of `basis` on `outcome` at `months`, if fitted.
    pub fn row(
        &self,
        basis: ImpactBasis,
        outcome: ImpactOutcome,
        months: u32,
    ) -> Option<&ImpactRow> {
        self.rows
            .iter()
            .find(|r| r.basis == basis && r.outcome == outcome && r.months == months)
    }

    /// Whether bills of this tier are estimated.
    pub fn covers(&self, tier: TierKey) -> bool {
        self.tiers.contains(&tier)
    }

    /// Whether a mechanism or side-effect kind is accounted for: a fitted
    /// basis, or stated by arithmetic.
    pub fn backs(&self, kind: &str) -> bool {
        self.arithmetic.iter().any(|k| k == kind)
            || self.rows.iter().any(|r| r.basis.kinds().contains(&kind))
    }
}

/// Fluent keys the engine names for an estimate (`impact.ftl`): every one
/// needs an English entry.
pub fn impact_fluent_keys(sectors: &[SectorDef]) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for o in [
        "homes",
        "jobs",
        "sector-jobs",
        "rent",
        "low-income",
        "debt",
        "services",
        "approval",
    ] {
        for d in ["up", "down", "flat"] {
            v.push(format!("impact-{o}-{d}"));
        }
    }
    for t in [
        "homes-started",
        "extra-building",
        "jobs-supported",
        "jobs-created",
        "paid",
        "wage-uplift",
        "service-level",
        "tax-points",
        "rent-cap",
        "protection",
        "labour-supply",
    ] {
        v.push(format!("impact-target-{t}"));
    }
    v.push("impact-cost".into());
    v.push("impact-cost-none".into());
    for p in ["rates", "borrowing", "levy", "squeeze"] {
        v.push(format!("impact-paid-{p}"));
    }
    for g in ["low", "middle", "high", "older", "area"] {
        v.push(format!("impact-gains-{g}"));
        v.push(format!("impact-loses-{g}"));
    }
    for w in ["building", "full", "at-once", "first-year"] {
        v.push(format!("impact-when-{w}"));
    }
    for h in ["y1", "y3", "y5"] {
        v.push(format!("impact-horizon-{h}"));
    }
    v.push("impact-in-time".into());
    for s in ["rough", "fair", "firm"] {
        v.push(format!("impact-sure-{s}"));
    }
    v.push("impact-source-statistician".into());
    for h in ["flood", "storm", "wildfire"] {
        v.push(format!("impact-hazard-{h}"));
    }
    for c in [
        "administration",
        "education",
        "health",
        "safety",
        "infrastructure",
        "social",
        "housing",
        "environment",
        "economic_development",
    ] {
        v.push(format!("impact-category-{c}"));
    }
    v.push("impact-heading".into());
    // Sectors are named by their own keys.
    v.extend(sectors.iter().map(|s| s.name_key.clone()));
    v
}

/// Every mechanism and side-effect kind the pack's policies and families
/// use is accounted for by the table (a fitted basis, or `arithmetic`), and
/// the rows are well formed.
pub fn validate_impact(b: &crate::ContentBundle, issues: &mut Vec<crate::Issue>) {
    use crate::Issue;
    let t = &b.impact;
    let file = "impact_table";
    for r in &t.rows {
        if !IMPACT_HORIZONS.contains(&r.months) {
            issues.push(Issue::error(
                file,
                &format!(
                    "row {} → {} is at {} months: not a horizon ({IMPACT_HORIZONS:?})",
                    r.basis.key(),
                    r.outcome.key(),
                    r.months
                ),
            ));
        }
        let finite = r.p10.is_finite() && r.p50.is_finite() && r.p90.is_finite();
        if !finite || r.p10 > r.p50 || r.p50 > r.p90 {
            issues.push(Issue::error(
                file,
                &format!(
                    "row {} → {} at {} months needs finite p10 ≤ p50 ≤ p90",
                    r.basis.key(),
                    r.outcome.key(),
                    r.months
                ),
            ));
        }
    }
    let mut seen = BTreeMap::new();
    for r in &t.rows {
        if seen.insert((r.basis, r.outcome, r.months), ()).is_some() {
            issues.push(Issue::error(
                file,
                &format!(
                    "row {} → {} at {} months is given twice",
                    r.basis.key(),
                    r.outcome.key(),
                    r.months
                ),
            ));
        }
    }
    // A table with no rows is a pack without estimates: nothing to back.
    if t.rows.is_empty() && t.tiers.is_empty() {
        return;
    }
    let mut check = |owner: &str, kind: &str| {
        if !t.backs(kind) {
            issues.push(Issue::error(
                owner,
                &format!(
                    "`{kind}` has no row in the impact table and is not in its `arithmetic` list: a bill with it could not say what it does"
                ),
            ));
        }
    };
    for p in b.policies.values().filter(|p| t.covers(p.tier)) {
        for m in &p.mechanisms {
            check(&p.id, m.key());
        }
        for fx in &p.side_effects {
            check(&p.id, fx.key());
        }
    }
    for f in &b.policy_families {
        let options = f.axes.iter().flat_map(|a| a.options.iter());
        for m in f
            .base
            .mechanisms
            .iter()
            .chain(options.clone().flat_map(|o| o.mechanisms.iter()))
        {
            check(&f.id, m.key());
        }
        for fx in f
            .base
            .side_effects
            .iter()
            .chain(options.flat_map(|o| o.side_effects.iter()))
        {
            check(&f.id, fx.key());
        }
    }
}
