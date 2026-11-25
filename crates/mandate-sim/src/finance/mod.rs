//! Public finance per jurisdiction (local, regional, national): revenues,
//! budgets by category, entitlements, deficits, and debt (brief §7.3).
//! Ledger amounts use fixed-point `Money` (DEC-041).

use crate::economy::EconomyState;
use crate::events::EventKind;
use crate::population::{CLASSES, PopulationState, group_index};
use crate::scheduler::{Outs, Reads};
use crate::spine::{Cause, Process};
use crate::world::{MunicipalityId, World};
use mandate_content::EconomyParams;
use mandate_core::Money;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Category {
    Administration,
    Education,
    Health,
    Safety,
    Infrastructure,
    Social,
    Housing,
    Environment,
    EconomicDevelopment,
}

impl Category {
    pub const ALL: [Category; 9] = [
        Category::Administration,
        Category::Education,
        Category::Health,
        Category::Safety,
        Category::Infrastructure,
        Category::Social,
        Category::Housing,
        Category::Environment,
        Category::EconomicDevelopment,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Revenue {
    PropertyTax,
    SalesTax,
    IncomeTax,
    BusinessTax,
    Grants,
    Fees,
}

impl Revenue {
    pub const ALL: [Revenue; 6] = [
        Revenue::PropertyTax,
        Revenue::SalesTax,
        Revenue::IncomeTax,
        Revenue::BusinessTax,
        Revenue::Grants,
        Revenue::Fees,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TierKind {
    Local,
    Regional,
    National,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct YearSummary {
    pub year: u32,
    pub revenue: Money,
    pub spending: Money,
    pub interest: Money,
    pub balance: Money,
    pub debt: Money,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Treasury {
    pub tier: TierKind,
    pub cash: Money,
    pub debt: Money,
    /// Discretionary annual budget by category (indexed like `Category::ALL`).
    pub budget: Vec<Money>,
    /// Budget at initialisation (anchor for the public-sector demand index).
    pub base_budget_total: Money,
    pub revenue_ytd: Vec<Money>,
    pub spending_ytd: Vec<Money>,
    pub interest_ytd: Money,
    pub last_year: YearSummary,
    /// Changes voted into the next budget ([`vote_budget`]); applied and
    /// cleared when the budget year turns. Not written while empty, so a
    /// treasury nobody voted on hashes as it did.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub voted: Vec<BudgetVote>,
}

impl Treasury {
    /// A treasury whose discretionary budget splits `annual` by its tier's
    /// shares (`economy.treasury_shares`, DEC-098: content, not code).
    fn new(tier: TierKind, annual: Money, e: &EconomyParams) -> Self {
        let t = &e.treasury_shares;
        let shares = match tier {
            TierKind::Local => &t.local,
            TierKind::Regional => &t.regional,
            TierKind::National => &t.national,
        };
        let budget: Vec<Money> = shares.iter().map(|s| annual.scale(*s)).collect();
        Treasury {
            tier,
            cash: annual.scale(0.05),
            debt: if tier == TierKind::National {
                annual.scale(1.2)
            } else {
                annual.scale(0.2)
            },
            base_budget_total: budget.iter().copied().sum(),
            budget,
            revenue_ytd: vec![Money::ZERO; Revenue::ALL.len()],
            spending_ytd: vec![Money::ZERO; Category::ALL.len()],
            interest_ytd: Money::ZERO,
            last_year: YearSummary::default(),
            voted: Vec::new(),
        }
    }

    pub fn budget_total(&self) -> Money {
        self.budget.iter().copied().sum()
    }

    fn earn(&mut self, src: Revenue, amount: Money) {
        self.revenue_ytd[src as usize] += amount;
        self.cash += amount;
    }

    fn spend(&mut self, cat: Category, amount: Money) {
        self.spending_ytd[cat as usize] += amount;
        self.cash -= amount;
    }

    /// Borrow to cover negative cash; repay debt from surplus cash.
    fn settle(&mut self, reserve: Money) {
        if self.cash.is_negative() {
            self.debt += -self.cash;
            self.cash = Money::ZERO;
        } else if self.cash > reserve && self.debt > Money::ZERO {
            let repay = (self.cash - reserve).min(self.debt);
            self.debt -= repay;
            self.cash -= repay;
        }
    }
}

/// Standing tax settings (DEC-024: changed through the budget procedure).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TaxRates {
    /// Property tax per municipality (share of dwelling value per year).
    pub property: Vec<f64>,
    /// Sales tax per region.
    pub sales: Vec<f64>,
    /// National income tax by class: low, middle, high.
    pub income: [f64; CLASSES],
    pub business: f64,
}

/// Funding for an enacted policy (authorization and appropriation are distinct, DEC-024).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Appropriation {
    pub municipality: u32,
    pub category: usize,
    /// Annual cost (nominal).
    pub annual_cost: f64,
    /// Share of the cost actually funded (0..1).
    pub funded_share: f64,
    /// Share of the cost borrowed rather than paid from the budget (a
    /// family bill's `borrow`, DEC-092): added to debt as it is spent.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub borrow_share: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

impl Appropriation {
    /// The part of its funded cost paid from this year's budget.
    pub fn from_budget(&self) -> f64 {
        self.annual_cost * self.funded_share * (1.0 - self.borrow_share)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FinanceState {
    pub taxes: TaxRates,
    /// Keyed by policy instance id.
    pub appropriations: std::collections::BTreeMap<u32, Appropriation>,
    pub local: Vec<Treasury>,
    pub regional: Vec<Treasury>,
    pub national: Option<Treasury>,
    /// Per-capita grants (annual) from higher tiers.
    pub grant_national_to_regional: f64,
    pub grant_national_to_local: f64,
    pub grant_regional_to_local: f64,
    /// Structural national deficit target as a share of output.
    pub structural_deficit: f64,
    /// National mean wage and population at initialisation (budget indexation).
    pub base_mean_wage: f64,
    pub base_population: f64,
}

impl FinanceState {
    pub fn funded_share(&self, policy: u32) -> f64 {
        self.appropriations
            .get(&policy)
            .map(|a| a.funded_share)
            .unwrap_or(0.0)
    }

    /// Annual cost of funded programmes in a municipality.
    pub fn programmes_cost(&self, m: u32) -> f64 {
        self.appropriations
            .values()
            .filter(|a| a.municipality == m)
            .map(|a| a.annual_cost * a.funded_share)
            .sum()
    }

    /// Public spending relative to baseline, averaged over the three tiers
    /// serving a municipality (drives public-sector jobs).
    pub fn public_spending_index(&self, m: usize) -> f64 {
        let ratio = |t: &Treasury| t.budget_total().0 as f64 / t.base_budget_total.0.max(1) as f64;
        let l = self.local.get(m).map(ratio).unwrap_or(1.0);
        let n = self.national.as_ref().map(ratio).unwrap_or(1.0);
        // Regional treasury index is looked up by the caller's region elsewhere;
        // use the mean regional ratio as an approximation.
        let r = if self.regional.is_empty() {
            1.0
        } else {
            self.regional.iter().map(ratio).sum::<f64>() / self.regional.len() as f64
        };
        (l + r + n) / 3.0
    }
}

struct Flows {
    property: Vec<f64>,
    sales_by_region: Vec<f64>,
    income_tax: f64,
    business_tax: f64,
    pensions: f64,
    benefits: f64,
    population_by_muni: Vec<f64>,
    population_by_region: Vec<f64>,
}

/// Annual flows implied by the current economy and population.
fn annual_flows(
    world: &World,
    pop: &PopulationState,
    econ: &EconomyState,
    e: &EconomyParams,
    tax: &TaxRates,
    tax_delta: &dyn Fn(usize) -> f64,
) -> Flows {
    let nm = world.municipalities.len().min(econ.munis.len());
    let mut f = Flows {
        property: vec![0.0; nm],
        sales_by_region: vec![0.0; world.regions.len()],
        income_tax: 0.0,
        business_tax: 0.0,
        pensions: 0.0,
        benefits: 0.0,
        population_by_muni: vec![0.0; nm],
        population_by_region: vec![0.0; world.regions.len()],
    };
    for m in 0..nm {
        let mu = &econ.munis[m];
        let region = world.municipalities[m].region.index();
        let groups = pop.municipality_groups(world, MunicipalityId(m as u32));
        let people = pop.municipality_total(world, MunicipalityId(m as u32)) as f64;
        f.population_by_muni[m] = people;
        f.population_by_region[region] += people;
        f.property[m] = mu.housing.stock
            * mu.housing.price
            * tax.property.get(m).copied().unwrap_or(e.taxes.property)
            + mu.housing.stock * mu.housing.price * tax_delta(m);
        f.sales_by_region[region] +=
            mu.consumption * tax.sales.get(region).copied().unwrap_or(e.taxes.sales);
        f.business_tax += mu.output * tax.business;
        for c in 0..CLASSES {
            let working: f64 = (0..3).map(|a| groups[group_index(a, c)]).sum();
            let seniors = groups[group_index(3, c)];
            f.income_tax += working * mu.class_income[c] * tax.income[c];
            f.pensions += seniors * e.pension_replacement * mu.mean_wage;
            f.benefits += working
                * (1.0 - mu.class_employment[c])
                * e.unemployment_replacement
                * mu.mean_wage;
        }
    }
    f
}

pub fn default_taxes(world: &World, e: &EconomyParams) -> TaxRates {
    TaxRates {
        property: vec![e.taxes.property; world.municipalities.len()],
        sales: vec![e.taxes.sales; world.regions.len()],
        income: e.taxes.income,
        business: e.taxes.business,
    }
}

pub fn init(
    world: &World,
    pop: &PopulationState,
    econ: &EconomyState,
    e: &EconomyParams,
) -> FinanceState {
    let taxes = default_taxes(world, e);
    let f = annual_flows(world, pop, econ, e, &taxes, &|_| 0.0);
    let (g_nr, g_nl, g_rl) = (900.0, 120.0, 200.0);
    let local = (0..f.property.len())
        .map(|m| {
            let rev = f.property[m] * 1.05 + f.population_by_muni[m] * (g_nl + g_rl);
            Treasury::new(TierKind::Local, Money::from_units(rev as i64), e)
        })
        .collect();
    let regional = (0..world.regions.len())
        .map(|r| {
            let rev = f.sales_by_region[r] + f.population_by_region[r] * (g_nr - g_rl);
            Treasury::new(
                TierKind::Regional,
                Money::from_units(rev.max(0.0) as i64),
                e,
            )
        })
        .collect();
    let total_pop: f64 = f.population_by_muni.iter().sum();
    let nat_rev = f.income_tax + f.business_tax;
    let nat_discretionary = (nat_rev - f.pensions - f.benefits - total_pop * (g_nr + g_nl)
        + 0.015 * econ.national.gdp.max(0.0))
    .max(0.0);
    FinanceState {
        taxes,
        appropriations: Default::default(),
        local,
        regional,
        national: Some(Treasury::new(
            TierKind::National,
            Money::from_units(nat_discretionary as i64),
            e,
        )),
        grant_national_to_regional: g_nr,
        grant_national_to_local: g_nl,
        grant_regional_to_local: g_rl,
        structural_deficit: 0.015,
        base_mean_wage: econ
            .munis
            .iter()
            .map(|m| m.mean_wage * m.total_jobs())
            .sum::<f64>()
            / econ
                .munis
                .iter()
                .map(|m| m.total_jobs())
                .sum::<f64>()
                .max(1.0),
        base_population: total_pop,
    }
}

/// Monthly accrual of revenues, spending, entitlements, interest, and debt.
pub fn accrue(fin: &mut FinanceState, r: &Reads<'_>, _out: &mut Outs<'_>) {
    let world = r.world;
    let econ = r.economy();
    let e = &r.content.economy;
    let policy_state = r.policy();
    let content = r.content;
    let funded = |id: u32| fin.funded_share(id);
    let deltas: Vec<f64> = (0..world.municipalities.len())
        .map(|m| {
            crate::policy::mods_for(policy_state, content, m as u32, &funded).property_tax_delta
        })
        .collect();
    let f = annual_flows(world, r.population(), econ, e, &fin.taxes, &|m| {
        deltas.get(m).copied().unwrap_or(0.0)
    });
    let rate = econ.national.policy_rate;
    let m12 = |v: f64| Money::from_units((v / 12.0) as i64);
    // Grants are indexed to prices × productivity so they keep their real value.
    let index = (econ.national.price_level * econ.national.productivity_index).max(0.1);
    let (g_nr, g_nl, g_rl) = (
        fin.grant_national_to_regional * index,
        fin.grant_national_to_local * index,
        fin.grant_regional_to_local * index,
    );

    for (m, t) in fin.local.iter_mut().enumerate() {
        let people = f.population_by_muni.get(m).copied().unwrap_or(0.0);
        t.earn(
            Revenue::PropertyTax,
            m12(f.property.get(m).copied().unwrap_or(0.0)),
        );
        t.earn(
            Revenue::Fees,
            m12(f.property.get(m).copied().unwrap_or(0.0) * 0.05),
        );
        t.earn(Revenue::Grants, m12(people * (g_nl + g_rl)));
    }
    for (rg, t) in fin.regional.iter_mut().enumerate() {
        let people = f.population_by_region.get(rg).copied().unwrap_or(0.0);
        t.earn(
            Revenue::SalesTax,
            m12(f.sales_by_region.get(rg).copied().unwrap_or(0.0)),
        );
        t.earn(Revenue::Grants, m12(people * g_nr));
        t.spend(Category::Administration, m12(people * g_rl)); // grants passed down
    }
    let total_pop: f64 = f.population_by_muni.iter().sum();
    let reserve_n;
    if let Some(n) = fin.national.as_mut() {
        n.earn(Revenue::IncomeTax, m12(f.income_tax));
        n.earn(Revenue::BusinessTax, m12(f.business_tax));
        n.spend(Category::Social, m12(f.pensions + f.benefits));
        n.spend(Category::Administration, m12(total_pop * (g_nr + g_nl)));
        reserve_n = n.budget_total().scale(0.02);
    } else {
        reserve_n = Money::ZERO;
    }
    // Programme appropriations for enacted local policies (DEC-024: funding is
    // a status distinct from enactment).
    let policy = r.policy();
    let price = econ.national.price_level;
    let active: std::collections::BTreeSet<u32> = policy
        .instances
        .iter()
        .filter(|p| p.active())
        .map(|p| p.id.0)
        .collect();
    fin.appropriations.retain(|id, _| active.contains(id));
    // Income from family bills' `revenue` mechanisms, by town (DEC-092).
    let mut income: Vec<(usize, f64)> = Vec::new();
    for p in policy.instances.iter().filter(|p| p.active()) {
        let Some(t) = crate::policy::resolve(r.content, policy, p) else {
            continue;
        };
        let m = p.jurisdiction as usize;
        let town = f.population_by_muni.get(m).copied().unwrap_or(0.0);
        // Per-capita costs count the area's residents (an `area` clause).
        let residents = crate::policy::cost_residents(world, r.population(), p, town);
        let cost = crate::policy::annual_cost(&t, p, residents, price);
        let borrow = crate::policy::borrow_share(&t);
        let revenue = crate::policy::annual_revenue(&t, p, residents, price);
        if revenue > 0.0 {
            income.push((m, revenue * p.progress as f64 * fin.funded_share(p.id.0)));
        }
        let category = t.category as usize;
        if let Some(a) = fin.appropriations.get_mut(&p.id.0) {
            a.annual_cost = cost;
            a.borrow_share = borrow;
        } else if let Some(treasury) = fin.local.get(m) {
            // New law: fund what fits in last year's room plus a 3% contingency.
            let existing: f64 = fin
                .appropriations
                .values()
                .filter(|a| a.municipality == p.jurisdiction)
                .map(|a| a.from_budget())
                .sum();
            // A borrowed share needs no room in the budget.
            let needs = cost * (1.0 - borrow);
            let share = first_share(room(treasury), existing, needs);
            fin.appropriations.insert(
                p.id.0,
                Appropriation {
                    municipality: p.jurisdiction,
                    category,
                    annual_cost: cost,
                    funded_share: share,
                    borrow_share: borrow,
                },
            );
        }
    }
    for a in fin.appropriations.values() {
        if let Some(t) = fin.local.get_mut(a.municipality as usize) {
            if a.borrow_share > 0.0 {
                // The borrowed part is spent too, and added to the debt.
                let monthly = a.annual_cost * a.funded_share / 12.0;
                let borrowed = Money::from_units((monthly * a.borrow_share) as i64);
                let paid = Money::from_units((monthly * (1.0 - a.borrow_share)) as i64);
                t.spend(Category::ALL[a.category.min(Category::ALL.len() - 1)], paid);
                t.debt += borrowed;
            } else {
                let spend = Money::from_units((a.annual_cost * a.funded_share / 12.0) as i64);
                t.spend(
                    Category::ALL[a.category.min(Category::ALL.len() - 1)],
                    spend,
                );
            }
        }
    }
    for (m, v) in income {
        if let Some(t) = fin.local.get_mut(m) {
            t.earn(Revenue::Fees, Money::from_units((v / 12.0) as i64));
        }
    }

    let borrowing = &r.content.economy.borrowing;
    let all = fin
        .local
        .iter_mut()
        .chain(fin.regional.iter_mut())
        .chain(fin.national.iter_mut());
    for t in all {
        for (i, cat) in Category::ALL.iter().enumerate() {
            let monthly = Money(t.budget[i].0 / 12);
            t.spend(*cat, monthly);
        }
        let interest = t.debt.scale((rate + spread(borrowing, t)) / 12.0);
        t.interest_ytd += interest;
        t.cash -= interest;
        let reserve = if t.tier == TierKind::National {
            reserve_n
        } else {
            t.budget_total().scale(0.03)
        };
        t.settle(reserve);
    }
}

/// The room a treasury has for a new law until its next budget: what last
/// year's revenue left after interest and the budget, plus a 3% contingency.
pub fn room(t: &Treasury) -> f64 {
    let budget = t.budget_total().units() as f64;
    (t.last_year.revenue.units() as f64 - t.last_year.interest.units() as f64 - budget).max(0.0)
        + budget * 0.03
}

/// The share of a new law's cost funded until the next budget year (the
/// funding rule as a pure function, shared with the impact estimate,
/// DEC-111): what fits in the treasury's `room` after the programmes
/// already paid from the budget (`existing`). `needs` is its yearly cost
/// less any borrowed share, which needs no room.
pub fn first_share(room: f64, existing: f64, needs: f64) -> f64 {
    if needs <= 0.0 {
        1.0
    } else {
        clamp((room - existing) / needs, 0.0, 1.0)
    }
}

/// The share every programme of a jurisdiction is funded at from a budget
/// year on: programmes may take a quarter of the `budget`; beyond that all
/// are pro-rated. `full` is their yearly cost from the budget in full.
pub fn capped_share(budget: f64, full: f64) -> f64 {
    if full <= 0.0 {
        1.0
    } else {
        clamp(0.25 * budget / full, 0.0, 1.0)
    }
}

/// The treasury of a jurisdiction (DEC-098): a town's, a region's, the
/// nation's.
pub fn treasury(fin: &FinanceState, at: crate::world::Jurisdiction) -> Option<&Treasury> {
    use crate::world::Jurisdiction as J;
    match at {
        J::Municipality { id } => fin.local.get(id.index()),
        J::Region { id } => fin.regional.get(id.index()),
        J::Nation => fin.national.as_ref(),
    }
}

/// The day a jurisdiction's budget year turns next after `date`. Every
/// treasury, whatever its tier, closes its books on 1 January
/// ([`new_budget_year`]); a tier with its own fiscal year would be read
/// here.
pub fn budget_year_turns(
    at: crate::world::Jurisdiction,
    date: mandate_core::SimDate,
) -> mandate_core::SimDate {
    let _ = at;
    mandate_core::SimDate::from_ymd(date.calendar().year + 1, 1, 1)
}

/// Bounds of one budget change asked for by another domain.
pub const BUDGET_FACTOR: [f64; 2] = [0.8, 1.25];
/// Changes voted for one budget year, at most.
pub const VOTES_CAP: usize = 8;

/// A change voted into a jurisdiction's next budget (DEC-109: the budget
/// round): one line of the discretionary budget (an index into
/// `Category::ALL`), or every line, × `factor` against what the baseline
/// rule would set.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BudgetVote {
    pub category: Option<u8>,
    pub factor: f64,
}

/// Vote a change into a jurisdiction's next budget: when its budget year
/// turns ([`new_budget_year`]) the baseline budget is set as ever, then
/// each voted change is applied to it, and the votes are spent. Finance's
/// own change: the applier of `Effect::Budget`, and the commands that ask
/// for it outside the scheduler.
pub fn vote_budget(
    fin: &mut FinanceState,
    at: crate::world::Jurisdiction,
    category: Option<usize>,
    factor: f64,
) {
    use crate::world::Jurisdiction as J;
    if !factor.is_finite() || category.is_some_and(|c| c >= Category::ALL.len()) {
        return;
    }
    let t = match at {
        J::Municipality { id } => fin.local.get_mut(id.index()),
        J::Region { id } => fin.regional.get_mut(id.index()),
        J::Nation => fin.national.as_mut(),
    };
    let Some(t) = t else { return };
    if t.voted.len() >= VOTES_CAP {
        return;
    }
    t.voted.push(BudgetVote {
        category: category.map(|c| c as u8),
        factor: clamp(factor, BUDGET_FACTOR[0], BUDGET_FACTOR[1]),
    });
}

/// Finance domain, daily: changes to the next budget other domains asked for.
pub fn effects(fin: &mut FinanceState, _r: &Reads<'_>, out: &mut Outs<'_>) {
    for q in out.take(|e| matches!(e, crate::scheduler::Effect::Budget { .. })) {
        if let crate::scheduler::Effect::Budget {
            at,
            category,
            factor,
        } = q.effect
        {
            vote_budget(fin, at, category, factor);
        }
    }
}

/// What a treasury pays to borrow over the policy rate: its tier's spread,
/// rising with its debt against a year's revenue beyond what lenders
/// ignore (`economy.borrowing`). Never below the tier's: a prudent
/// jurisdiction pays what it always has.
pub fn spread(b: &mandate_content::BorrowingParams, t: &Treasury) -> f64 {
    let i = match t.tier {
        TierKind::Local => 0,
        TierKind::Regional => 1,
        TierKind::National => 2,
    };
    let base = b.spreads[i];
    // A year's revenue: last year's, else (in the first year) the budget.
    let revenue = if t.last_year.revenue.0 > 0 {
        t.last_year.revenue
    } else {
        t.budget_total()
    };
    let ratio = t.debt.units() as f64 / (revenue.units() as f64).max(1.0);
    base + clamp(
        (ratio - b.free_ratio[i]) * b.per_ratio,
        0.0,
        b.max_extra.max(0.0),
    )
}

/// Yearly: close the books and set next year's budgets.
///
/// Baseline fiscal policy (until governments are run by players and NPCs,
/// M3–M4): discretionary budgets are indexed to wages and population so public
/// services stay constant in real terms. Local budgets also respond to their
/// own revenue; the national budget has a mild debt brake.
pub fn new_budget_year(fin: &mut FinanceState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let year = r.date.calendar().year.saturating_sub(1);
    let econ = r.economy();
    let gdp = econ.national.gdp;
    // Real public services held constant: index to prices × productivity.
    let wage_index = (econ.national.price_level * econ.national.productivity_index).max(0.1);
    let pop_index = (r.population().total() as f64 / fin.base_population.max(1.0)).max(0.1);
    // The new budget adopts every enacted programme (DEC-024: appropriation is
    // yearly). Programmes may take up to a quarter of a town's budget; beyond
    // that all are pro-rated. Baseline services make room below.
    for m in 0..fin.local.len() {
        let budget = fin.local[m].budget_total().units() as f64;
        let full: f64 = fin
            .appropriations
            .values()
            .filter(|a| a.municipality as usize == m)
            .map(|a| a.annual_cost * (1.0 - a.borrow_share))
            .sum();
        let share = capped_share(budget, full);
        for a in fin
            .appropriations
            .values_mut()
            .filter(|a| a.municipality as usize == m)
        {
            a.funded_share = share;
        }
    }
    let programme_cost_by_idx: std::collections::BTreeMap<usize, f64> = (0..fin.local.len())
        .map(|m| (m, fin.programmes_cost(m as u32)))
        .collect();
    let mut national_gap: Option<f64> = None;
    let mut local_short: Vec<(TierKind, usize, f64)> = Vec::new();
    let n_local = fin.local.len();
    let n_regional = fin.regional.len();
    let all = fin
        .local
        .iter_mut()
        .chain(fin.regional.iter_mut())
        .chain(fin.national.iter_mut());
    for (k, t) in all.enumerate() {
        let idx = if k < n_local {
            k
        } else if k < n_local + n_regional {
            k - n_local
        } else {
            0
        };
        let revenue: Money = t.revenue_ytd.iter().copied().sum();
        let spending: Money = t.spending_ytd.iter().copied().sum();
        t.last_year = YearSummary {
            year,
            revenue,
            spending,
            interest: t.interest_ytd,
            balance: revenue - spending - t.interest_ytd,
            debt: t.debt,
        };
        let mut target = wage_index * pop_index;
        match t.tier {
            TierKind::Local | TierKind::Regional => {
                // Sub-national governments feel their own revenue position.
                let cover = if spending.0 > 0 {
                    revenue.0 as f64 / spending.0 as f64
                } else {
                    1.0
                };
                target *= clamp(cover, 0.85, 1.1).sqrt();
            }
            TierKind::National => {
                // Debt brake: consolidation grows with debt above 60% of output.
                let debt_ratio = if gdp > 0.0 {
                    t.debt.units() as f64 / gdp
                } else {
                    0.0
                };
                if debt_ratio > 0.6 {
                    target *= 1.0 - (debt_ratio - 0.6).min(0.8) * 0.25;
                }
            }
        }
        let current = t.budget_total();
        let mut goal = t.base_budget_total.scale(target);
        if t.tier == TierKind::National {
            // Revenue-aware: the gap between discretionary needs and what
            // revenue supports (after entitlements, grants, interest, and a
            // 2% deficit allowance) is closed half by taxes, half by trims.
            let entitlements = t.spending_ytd[Category::Social as usize];
            let grants = (t.spending_ytd[Category::Administration as usize]
                - t.budget[Category::Administration as usize])
                .max(Money::ZERO);
            let allowance = Money::from_units((gdp * 0.02) as i64);
            let cap = revenue - entitlements - grants - t.interest_ytd + allowance;
            let gap = (goal - cap).units() as f64 / gdp.max(1.0);
            national_gap = Some(gap);
            if gap > 0.0 {
                let trimmed = goal - (goal - cap).scale(0.5);
                goal = trimmed.max(current.scale(0.95));
            }
        }
        if matches!(t.tier, TierKind::Local | TierKind::Regional) {
            // Balanced-budget baseline: spend no more than revenue after
            // interest and a 5% debt paydown; shortfalls raise local taxes.
            let programmes = if t.tier == TierKind::Local {
                Money::from_units(programme_cost_by_idx.get(&idx).copied().unwrap_or(0.0) as i64)
            } else {
                Money::ZERO
            };
            // Funded programmes crowd out baseline services (a real tradeoff).
            let cap = revenue
                - t.interest_ytd
                - t.debt.scale(0.05)
                - pass_through_estimate(t)
                - programmes;
            if cap < goal {
                local_short.push((
                    t.tier,
                    idx,
                    (goal - cap).units() as f64 / goal.units().max(1) as f64,
                ));
                goal = cap.max(current.scale(0.95));
            }
        }
        if current.0 > 0 {
            for b in t.budget.iter_mut() {
                *b = b.scale_ratio(goal.0, current.0);
            }
        }
        // What was voted for this year, on top of the baseline.
        for vote in std::mem::take(&mut t.voted) {
            for (i, b) in t.budget.iter_mut().enumerate() {
                if vote.category.is_none_or(|c| usize::from(c) == i) {
                    *b = b.scale(vote.factor);
                }
            }
        }
        t.revenue_ytd.iter_mut().for_each(|v| *v = Money::ZERO);
        t.spending_ytd.iter_mut().for_each(|v| *v = Money::ZERO);
        t.interest_ytd = Money::ZERO;
    }
    // Tax adjustments (baseline fiscal rule until governments set taxes, M3).
    if let Some(gap) = national_gap {
        let step = clamp(gap * 0.5, -0.003, 0.006);
        let caps = [0.18, 0.32, 0.48];
        for (c, rate) in fin.taxes.income.iter_mut().enumerate() {
            *rate = clamp(*rate + step * [0.6, 1.0, 1.3][c], 0.02, caps[c]);
        }
    }
    for (tier, i, short) in local_short {
        let bump = 1.0 + clamp(short * 0.5, 0.0, 0.08);
        match tier {
            TierKind::Local => {
                if let Some(rate) = fin.taxes.property.get_mut(i) {
                    *rate = clamp(*rate * bump, 0.002, 0.025);
                }
            }
            TierKind::Regional => {
                if let Some(rate) = fin.taxes.sales.get_mut(i) {
                    *rate = clamp(*rate * bump, 0.01, 0.15);
                }
            }
            TierKind::National => {}
        }
    }
    if let Some(n) = &fin.national {
        let debt_ratio = if gdp > 0.0 {
            n.debt.units() as f64 / gdp
        } else {
            0.0
        };
        out.events.record(
            r.date,
            EventKind::NationalBudget {
                year,
                balance: n.last_year.balance.units(),
                debt_pct_gdp: (debt_ratio * 100.0).round() as u32,
            },
            Cause::Process(Process::Budget),
        );
    }
}

/// Grants a regional treasury passes down to municipalities (booked as
/// Administration spending outside the discretionary budget).
fn pass_through_estimate(t: &Treasury) -> Money {
    if t.tier == TierKind::Regional {
        let admin_budget = t.budget[Category::Administration as usize];
        (t.spending_ytd[Category::Administration as usize] - admin_budget).max(Money::ZERO)
    } else {
        Money::ZERO
    }
}

fn clamp(x: f64, lo: f64, hi: f64) -> f64 {
    mandate_core::math::clamp(x, lo, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> EconomyParams {
        let src = mandate_content::bundle::read_dir_sources(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        )
        .unwrap();
        mandate_content::load(&src).unwrap().0.economy
    }

    fn treasury(debt: i64, revenue: i64) -> Treasury {
        let mut t = Treasury::new(TierKind::Local, Money::from_units(revenue), &params());
        t.debt = Money::from_units(debt);
        t.last_year.revenue = Money::from_units(revenue);
        t
    }

    /// A prudent town pays its tier's spread; debt beyond what lenders
    /// ignore raises it, up to the cap.
    #[test]
    fn borrowing_costs_rise_with_debt() {
        let b = params().borrowing;
        let base = b.spreads[0];
        assert_eq!(spread(&b, &treasury(0, 1_000_000)), base);
        assert_eq!(spread(&b, &treasury(100_000, 1_000_000)), base);
        let one = spread(&b, &treasury(1_000_000, 1_000_000));
        assert!((one - (base + (1.0 - b.free_ratio[0]) * b.per_ratio)).abs() < 1e-12);
        let huge = spread(&b, &treasury(1_000_000_000, 1_000_000));
        assert!((huge - (base + b.max_extra)).abs() < 1e-12);
    }
}
