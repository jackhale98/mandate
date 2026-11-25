//! Industries (DEC-110, `docs/design/INDUSTRIES.md`): a registry level under
//! the sectors. A sector with industries is a *split sector*: its
//! `SectorState` is the sum of its industries', written here each month and
//! by nothing else, so every reader of sector state is unchanged.
//!
//! - **Demand** is one generic product of named factors ([`Factor`]) with
//!   exponents from content: demand itself (external and at home), the
//!   prices of goods sold and bought, a regional stock, water, import
//!   competition, local labour costs, and local supply (upstream jobs
//!   nearby against their ten-year average).
//! - **The seam for foreign trade.** External demand is a sum over the
//!   markets registry: an industry's share in each market × that market's
//!   index for the industry (its cycle × the industry's own keyed walk
//!   there) × the market's access. With one market that is exactly one
//!   world index per industry. Demand is `export_share × external + (1 −
//!   export_share) × home`. Prices are read through the goods registry. A
//!   cause that blames external demand names the market and the good.
//! - **Siting** ([`shares`]) shares a town's sector among its industries by
//!   geography, the region's tradition and keyed draws: a pure function of
//!   the world and the seed, used by worldgen and by a save that predates
//!   industries alike.
//!
//! No code here names an industry or a product: registry indices, `Factor`
//! and content keys only.

use super::plants::{self, Plant};
use super::{ActiveShock, COMMODITIES, EconomyState, Global, MuniEcon, National};
use crate::environment::RegionEnv;
use crate::events::{EventId, EventKind};
use crate::scheduler::Outs;
use crate::spine::{Cause, Pressure, Shock};
use crate::world::{LinkMode, NeighborhoodKind, SettlementClass, World};
use mandate_content::{
    ContentBundle, ContentKey, GatewayKind, GeoTerm, HomeSource, IndustryDef, PlantOpens,
    ResourceStock, SectorDriver, SiteFlag,
};
use mandate_core::math::{clamp, exp, ln, pow};
use mandate_core::{SimDate, Stream, define_id, rng::keyed};
use serde::{Deserialize, Serialize};

define_id!(
    /// A market, by its position in `EconomyState::market_keys`: appended
    /// to, never reordered or reused, so an old cause still names its market.
    MarketId
);

define_id!(
    /// A good, by its position in `EconomyState::good_keys` (as `MarketId`).
    GoodId
);

/// One market as the state holds it (DEC-110, the seam for foreign trade).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MarketState {
    /// Access to the market: 1 is open trade. Nothing changes it yet; a
    /// tariff, a treaty, a dispute or a closed border will.
    pub access: f64,
    /// Each industry's own index in this market (1.0 = trend), in
    /// `EconomyState::industry_keys` order.
    pub index: Vec<f64>,
}

/// The terms of an industry's demand formula.
pub const FACTORS: usize = 10;

/// A term of the demand formula (DEC-110): never a product, never an
/// industry. The weakest one, smoothed, is what a closure is blamed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Factor {
    /// The cycle of the markets it sells to, raised to its cyclicality.
    Cycle,
    /// Its own index in those markets, their access, and any demand shock
    /// on it here: external demand beyond the cycle.
    World,
    /// The prices of what it sells.
    Price,
    /// The prices of what it buys.
    Input,
    /// The regional stock it draws on.
    Stock,
    Water,
    /// The nation's manufacturing competitiveness.
    Competitiveness,
    /// The town's unit labour costs.
    LabourCost,
    /// Upstream jobs nearby against their long average.
    Supply,
    /// Demand at home (the domestic part of its demand).
    Home,
}

impl Factor {
    pub const ALL: [Factor; FACTORS] = [
        Factor::Cycle,
        Factor::World,
        Factor::Price,
        Factor::Input,
        Factor::Stock,
        Factor::Water,
        Factor::Competitiveness,
        Factor::LabourCost,
        Factor::Supply,
        Factor::Home,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// Snake-case key: the Fluent suffix (`plant-why-<key>`, `factor-<key>`).
    pub fn key(self) -> &'static str {
        match self {
            Factor::Cycle => "cycle",
            Factor::World => "world",
            Factor::Price => "price",
            Factor::Input => "input",
            Factor::Stock => "stock",
            Factor::Water => "water",
            Factor::Competitiveness => "competitiveness",
            Factor::LabourCost => "labour_cost",
            Factor::Supply => "supply",
            Factor::Home => "home",
        }
    }

    /// The standing pressure the spine records for this factor. External
    /// demand names the market that weighs most on it and the good.
    pub fn pressure(self, market: MarketId, good: GoodId) -> Pressure {
        match self {
            Factor::Cycle => Pressure::WorldDemand,
            Factor::World => Pressure::ExternalDemand { market, good },
            Factor::Home => Pressure::HomeDemand,
            Factor::Price => Pressure::CommodityPrices,
            Factor::Input => Pressure::InputCosts,
            Factor::Stock => Pressure::Depletion,
            Factor::Water => Pressure::Water,
            Factor::Competitiveness => Pressure::Competitiveness,
            Factor::LabourCost => Pressure::LabourCosts,
            Factor::Supply => Pressure::Supply,
        }
    }
}

/// One industry in one town. A town holds only the industries it has, in
/// registry order (`k` ascending).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IndustryState {
    /// Index into `EconomyState::industry_keys` (the content's registry).
    pub k: u16,
    /// Capacity of the unnamed firms.
    pub small: f64,
    /// `small` plus the town's open plants in this industry: written each
    /// month, and whenever a plant opens or closes.
    pub capacity: f64,
    pub jobs: f64,
    /// Annual wage per worker (nominal).
    pub wage: f64,
    pub productivity: f64,
    /// Ten-year moving average of jobs: the anchor of supply links.
    pub trend_jobs: f64,
    /// Smoothed log of each demand factor, in `Factor` order: who to blame.
    pub factors: [f32; FACTORS],
    /// Months the smoothed demand has been over the boom line.
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub boom: u16,
    /// Whole jobs as play began (the layout's input; never changed after).
    pub baseline_jobs: u32,
}

fn is_zero_u16(x: &u16) -> bool {
    *x == 0
}

impl IndustryState {
    /// The weakest smoothed factor and its level (1 = neutral).
    pub fn weakest(&self) -> (Factor, f64) {
        let mut best = (Factor::Cycle, self.factors[0]);
        for f in Factor::ALL {
            if self.factors[f.index()] < best.1 {
                best = (f, self.factors[f.index()]);
            }
        }
        (best.0, exp(best.1 as f64))
    }

    /// Demand as the smoothed factors give it (their product).
    pub fn smoothed_demand(&self) -> f64 {
        exp(self.factors.iter().map(|x| *x as f64).sum::<f64>())
    }
}

// Keyed-draw tags (DEC-041): each draw is named by what it is for.
const KEY_WORLD: u64 = 0x1D01;
const KEY_SITE_REGION: u64 = 0x1D02;
const KEY_SITE_TOWN: u64 = 0x1D03;

/// A stable `u64` for a content id (keys draws by id, not registry position).
pub(crate) fn key_of(id: &str) -> u64 {
    crate::characters::key_of(id)
}

fn class_index(c: SettlementClass) -> usize {
    match c {
        SettlementClass::Rural => 0,
        SettlementClass::Town => 1,
        SettlementClass::City => 2,
        SettlementClass::Metro => 3,
    }
}

/// The first day of the month `n` months after `date`'s.
pub fn add_months(date: SimDate, n: u32) -> SimDate {
    let c = date.calendar();
    let months = c.year * 12 + (c.month - 1) + n;
    SimDate::from_ymd(months / 12, months % 12 + 1, 1)
}

// --- Siting -----------------------------------------------------------------------

fn has_flag(world: &World, m: usize, f: SiteFlag) -> bool {
    let mu = &world.municipalities[m];
    match f {
        SiteFlag::Coastal => mu.coastal,
        SiteFlag::River => mu.on_river,
        SiteFlag::Rail => world
            .links
            .iter()
            .any(|l| l.mode == LinkMode::Rail && (l.a == mu.id || l.b == mu.id)),
        SiteFlag::Industrial => world
            .hoods_of(mu.id)
            .any(|h| world.neighborhoods[h].kind == NeighborhoodKind::Industrial),
    }
}

/// A town's gateways (DEC-110): its open places of a kind the content
/// marks as one, where trade passes.
pub fn gateways(
    world: &World,
    content: &ContentBundle,
    m: usize,
) -> Vec<(GatewayKind, crate::world::PlaceId)> {
    let Some(mu) = world.municipalities.get(m) else {
        return Vec::new();
    };
    mu.places
        .iter()
        .filter_map(|id| {
            let p = world.places.get(id.index())?;
            let g = content.gateway_of(p.kind.key())?;
            p.is_open().then_some((g, *id))
        })
        .collect()
}

fn geo_term(world: &World, m: usize, t: GeoTerm) -> f64 {
    let g = &world.municipalities[m].geo;
    match t {
        GeoTerm::Fertility => g.fertility,
        GeoTerm::Timber => g.timber,
        GeoTerm::Minerals => g.minerals,
        GeoTerm::Fishing => g.fishing,
        GeoTerm::Energy => g.energy,
        GeoTerm::Scenic => g.scenic,
    }
}

/// Every industry's siting score in a town (registry order): geography,
/// what the town has, its class, the region's tradition and the town's own
/// draw; zero where a need fails. Pure: the same world and seed give the
/// same scores whenever it is asked.
pub fn scores(world: &World, content: &ContentBundle, seed: u64, m: usize) -> Vec<f64> {
    let p = &content.economy.industries;
    let mu = &world.municipalities[m];
    let class = class_index(mu.class);
    let gates = gateways(world, content, m);
    let has_gate = |k: GatewayKind| gates.iter().any(|g| g.0 == k);
    let raw: Vec<f64> = content
        .industries
        .iter()
        .map(|d| {
            let s = &d.siting;
            if s.needs.iter().any(|f| !has_flag(world, m, *f))
                || s.needs_gateway.iter().any(|g| !has_gate(*g))
            {
                return 0.0;
            }
            let mut x = s.base;
            for (t, w) in &s.geo {
                x += w * geo_term(world, m, *t);
            }
            for (f, w) in &s.likes {
                if has_flag(world, m, *f) {
                    x *= w;
                }
            }
            for (g, w) in &s.gateways {
                if has_gate(*g) {
                    x *= w;
                }
            }
            x *= s.by_class[class];
            let id = key_of(&d.id);
            if s.regional_sd > 0.0 {
                let mut r = keyed(
                    seed,
                    Stream::Worldgen,
                    &[KEY_SITE_REGION, mu.region.0 as u64, id],
                );
                x *= exp(r.normal(0.0, s.regional_sd));
            }
            if p.town_sd > 0.0 {
                let mut r = keyed(seed, Stream::Worldgen, &[KEY_SITE_TOWN, m as u64, id]);
                x *= exp(r.normal(0.0, p.town_sd));
            }
            x.max(0.0)
        })
        .collect();
    // An industry does better beside the ones it likes: how much, by how
    // strongly the other scores here (before this step, so order is moot).
    content
        .industries
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let mut x = raw[i];
            for (j, f) in &d.near_index {
                x *= pow(*f, clamp(raw[*j], 0.0, 1.0));
            }
            x
        })
        .collect()
}

/// A split sector's shares among its industries in a town: `weight ×
/// score²`, the strongest few by settlement class, small shares folded into
/// the rest. `(industry index, share)` in registry order; shares add to 1.
pub fn shares(
    content: &ContentBundle,
    scores: &[f64],
    class: SettlementClass,
    sector: usize,
) -> Vec<(usize, f64)> {
    let p = &content.economy.industries;
    let mut w: Vec<(usize, f64)> = content
        .industries_of(sector)
        .map(|(i, d)| (i, d.weight * scores[i] * scores[i]))
        .collect();
    if w.is_empty() {
        return w;
    }
    if w.iter().all(|x| x.1 <= 0.0) {
        // No industry of the sector scores here: a generic mix.
        for x in w.iter_mut() {
            x.1 = content.industries[x.0].weight;
        }
    }
    // The strongest few (ties to the lower index).
    let keep = p.max_industries[class_index(class)].max(1) as usize;
    let mut order: Vec<usize> = (0..w.len()).collect();
    order.sort_by(|a, b| w[*b].1.total_cmp(&w[*a].1).then(a.cmp(b)));
    for &o in order.iter().skip(keep) {
        w[o].1 = 0.0;
    }
    // Fold small shares into the others (the largest always stays).
    let total: f64 = w.iter().map(|x| x.1).sum();
    if total > 0.0 {
        let top = order[0];
        for (o, x) in w.iter_mut().enumerate() {
            if o != top && x.1 / total < p.min_share {
                x.1 = 0.0;
            }
        }
    }
    let total: f64 = w.iter().map(|x| x.1).sum::<f64>().max(1e-12);
    w.retain(|x| x.1 > 0.0);
    for x in w.iter_mut() {
        x.1 /= total;
    }
    w
}

/// Whole parts of a whole total, by the largest-remainder method (ties to
/// the lowest index): the one rounding every jobs figure goes through, top
/// down, so parts always add up to their total. Parts with no weight get
/// nothing; with no weight at all the first part takes the total.
pub fn apportion(total: u32, parts: &[f64]) -> Vec<u32> {
    let mut out = vec![0u32; parts.len()];
    if parts.is_empty() {
        return out;
    }
    let sum: f64 = parts.iter().filter(|w| w.is_finite() && **w > 0.0).sum();
    if sum.is_nan() || sum <= 0.0 {
        out[0] = total;
        return out;
    }
    let mut fracs: Vec<(f64, usize)> = Vec::with_capacity(parts.len());
    let mut given = 0u32;
    for (i, w) in parts.iter().enumerate() {
        if w.is_finite() && *w > 0.0 {
            let exact = total as f64 * w / sum;
            let whole = exact.floor();
            out[i] = whole as u32;
            given += out[i];
            fracs.push((exact - whole, i));
        }
    }
    fracs.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let left = total.saturating_sub(given) as usize;
    for (_, i) in fracs.iter().cycle().take(left) {
        out[*i] += 1;
    }
    out
}

// --- Splitting a sector, and its totals --------------------------------------------

/// How a sector's wage is shared when it is split.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SplitWage {
    /// A new world: each industry at its own base wage (the sector's wage
    /// becomes their mean).
    Own,
    /// A running game: the sector's wage stays what it was, the industries
    /// around it in proportion to their base wages.
    KeepMean,
}

/// Give a town the industries of every split sector it holds jobs in but
/// has no industry state for (a new world; a save from before industries;
/// content that splits another sector), by the siting shares. Capacity and
/// jobs are shared exactly: the last industry takes the remainder. Returns
/// whether anything was added.
pub(super) fn split_town(
    mu: &mut MuniEcon,
    world: &World,
    content: &ContentBundle,
    seed: u64,
    m: usize,
    wage: SplitWage,
) -> bool {
    let mut site: Option<Vec<f64>> = None;
    let mut added = false;
    for (k, sdef) in content.sectors.iter().enumerate() {
        if !content.is_split(k) || k >= mu.sectors.len() {
            continue;
        }
        let has = mu.industries.iter().any(|x| {
            content
                .industries
                .get(x.k as usize)
                .is_some_and(|d| d.sector_index == k)
        });
        let s = mu.sectors[k].clone();
        if has || !(s.capacity > 0.0 || s.jobs > 0.0) {
            continue;
        }
        let site = site.get_or_insert_with(|| scores(world, content, seed, m));
        let sh = shares(content, site, world.municipalities[m].class, k);
        if sh.is_empty() {
            continue;
        }
        let mean_base: f64 = match wage {
            SplitWage::Own => sdef.base_wage,
            SplitWage::KeepMean => sh
                .iter()
                .map(|(i, x)| x * content.industries[*i].base_wage)
                .sum(),
        };
        let baselines = apportion(s.baseline_jobs, &sh.iter().map(|x| x.1).collect::<Vec<_>>());
        let (mut cap_left, mut jobs_left, mut trend_left) = (s.capacity, s.jobs, s.trend_jobs);
        for (n, (i, x)) in sh.iter().enumerate() {
            let last = n + 1 == sh.len();
            let part = |total: f64, left: &mut f64| {
                let v = if last { left.max(0.0) } else { total * x };
                *left -= v;
                v
            };
            let capacity = part(s.capacity, &mut cap_left);
            let jobs = part(s.jobs, &mut jobs_left);
            let trend_jobs = part(s.trend_jobs, &mut trend_left);
            mu.industries.push(IndustryState {
                k: *i as u16,
                small: capacity,
                capacity,
                jobs,
                wage: s.wage * content.industries[*i].base_wage / mean_base.max(1.0),
                productivity: s.productivity,
                trend_jobs,
                factors: [0.0; FACTORS],
                boom: 0,
                baseline_jobs: baselines[n],
            });
            added = true;
        }
    }
    if added {
        mu.industries.sort_by_key(|x| x.k);
    }
    added
}

/// Write every split sector's state as the sum of its industries: capacity,
/// jobs and the long average as sums in registry order; wage and
/// productivity as job-weighted means. A split sector with no industry here
/// is empty. Nothing else writes a split sector.
pub(super) fn write_totals(mu: &mut MuniEcon, content: &ContentBundle, split: &[bool]) {
    for (k, s) in mu.sectors.iter_mut().enumerate() {
        if !split.get(k).copied().unwrap_or(false) {
            continue;
        }
        let (mut cap, mut jobs, mut trend, mut bill, mut prod) = (0.0, 0.0, 0.0, 0.0, 0.0);
        let (mut cap_wage, mut cap_prod) = (0.0, 0.0);
        for x in &mu.industries {
            if content.industries[x.k as usize].sector_index != k {
                continue;
            }
            cap += x.capacity;
            jobs += x.jobs;
            trend += x.trend_jobs;
            bill += x.jobs * x.wage;
            prod += x.jobs * x.productivity;
            cap_wage += x.capacity * x.wage;
            cap_prod += x.capacity * x.productivity;
        }
        s.capacity = cap;
        s.jobs = jobs;
        s.trend_jobs = trend;
        if jobs > 0.0 {
            s.wage = bill / jobs;
            s.productivity = prod / jobs;
        } else if cap > 0.0 {
            s.wage = cap_wage / cap;
            s.productivity = cap_prod / cap;
        }
    }
}

/// Recompute a town's industry capacities (unnamed firms plus open plants)
/// and its split sectors' totals, after a change made outside the monthly
/// step (the lab; a test).
pub fn refresh(econ: &mut EconomyState, content: &ContentBundle, m: usize) {
    let split = split_flags(content);
    let Some(mu) = econ.munis.get_mut(m) else {
        return;
    };
    for st in mu.industries.iter_mut() {
        let key = econ.industry_keys.get(st.k as usize);
        let open: f64 = econ
            .plants
            .iter()
            .filter(|p| p.municipality.index() == m && p.is_open() && Some(&p.industry) == key)
            .map(|p| p.capacity)
            .sum();
        st.capacity = st.small + open;
    }
    write_totals(mu, content, &split);
}

/// Which sectors are split, in registry order.
pub fn split_flags(content: &ContentBundle) -> Vec<bool> {
    (0..content.sectors.len())
        .map(|k| content.is_split(k))
        .collect()
}

/// Re-key industry state to the content's registry and give every town the
/// industries of split sectors it has none for (a save from before
/// industries; content that adds industries to a sector). A town keeps what
/// it has; an industry the content adds elsewhere starts absent (it can
/// arrive by an opening). Plants are placed, at the works a town already
/// has, only when the state had no industries at all. Returns whether
/// anything changed.
pub fn remap(
    econ: &mut EconomyState,
    world: &World,
    meta: &crate::state::WorldMeta,
    date: SimDate,
    content: &ContentBundle,
) -> bool {
    let keys: Vec<mandate_content::ContentKey> = content.industry_keys().cloned().collect();
    let mut changed = false;
    let fresh = econ.industry_keys.is_empty() && econ.plants.is_empty();
    if econ.industry_keys != keys {
        // Old index -> new index (content that removed one was refused).
        let map: Vec<Option<u16>> = econ
            .industry_keys
            .iter()
            .map(|k| keys.iter().position(|x| x == k).map(|i| i as u16))
            .collect();
        for mu in &mut econ.munis {
            mu.industries
                .retain_mut(|x| match map.get(x.k as usize).copied().flatten() {
                    Some(k) => {
                        x.k = k;
                        true
                    }
                    None => false,
                });
            mu.industries.sort_by_key(|x| x.k);
        }
        for mk in &mut econ.markets {
            mk.index = keys
                .iter()
                .map(|k| {
                    econ.industry_keys
                        .iter()
                        .position(|x| x == k)
                        .and_then(|i| mk.index.get(i).copied())
                        .unwrap_or(1.0)
                })
                .collect();
        }
        econ.industry_keys = keys;
        changed = true;
    }
    changed |= sync_markets(econ, content);
    let split = split_flags(content);
    let n = world.municipalities.len().min(econ.munis.len());
    for m in 0..n {
        let mu = &mut econ.munis[m];
        // A sector's long average starts at its jobs (a save from before).
        for s in &mut mu.sectors {
            if s.trend_jobs == 0.0 && s.jobs > 0.0 {
                s.trend_jobs = s.jobs;
            }
        }
        if split_town(mu, world, content, meta.seed, m, SplitWage::KeepMean) {
            write_totals(mu, content, &split);
            changed = true;
        }
    }
    if fresh && changed {
        plants::place_all(econ, world, content, meta.seed, date, meta.start_date);
    }
    changed
}

// --- The markets ----------------------------------------------------------------------

/// Give the state every market and good the content has that it lacks:
/// appended, so a `MarketId` or a `GoodId` an old cause holds stays what it
/// was. A new market starts at its content access with every index at 1.
/// (Content that removed a market was refused before this.)
pub(super) fn sync_markets(econ: &mut EconomyState, content: &ContentBundle) -> bool {
    let mut changed = false;
    for m in &content.markets {
        if !econ.market_keys.contains(&m.id) {
            econ.market_keys.push(m.id.clone());
            econ.markets.push(MarketState {
                access: m.access,
                index: vec![1.0; econ.industry_keys.len()],
            });
            changed = true;
        }
    }
    for g in &content.goods {
        if !econ.good_keys.contains(&g.id) {
            econ.good_keys.push(g.id.clone());
            changed = true;
        }
    }
    changed
}

/// A market's id in the state, by content key.
pub fn market_id(econ: &EconomyState, key: &str) -> Option<MarketId> {
    econ.market_keys
        .iter()
        .position(|k| k == key)
        .map(|i| MarketId(i as u32))
}

/// A good's id in the state, by content key.
pub fn good_id(econ: &EconomyState, key: &str) -> Option<GoodId> {
    econ.good_keys
        .iter()
        .position(|k| k == key)
        .map(|i| GoodId(i as u32))
}

/// One month of an industry's index in a market: `ln x ← (1 − reversion) ×
/// ln x + normal(0, volatility) + jump`, kept within the range; the jump
/// (in logs) is returned when there was one. The draw is keyed by the
/// market's id, the industry's id and the day.
pub fn walk(
    x: f64,
    w: &mandate_content::WorldIndexDef,
    range: [f64; 2],
    seed: u64,
    market: &str,
    industry: &str,
    date: SimDate,
) -> (f64, Option<f64>) {
    let mut rng = keyed(
        seed,
        Stream::Economy,
        &[KEY_WORLD, key_of(market), key_of(industry), date.0 as u64],
    );
    let mut lx = ln(x.max(0.05));
    lx = (1.0 - w.reversion) * lx + rng.normal(0.0, w.volatility);
    let mut jumped = None;
    if rng.chance(w.shock_chance) {
        let size = rng.range_f64(w.shock_size[0], w.shock_size[1]);
        let jump = if rng.chance(w.shock_down) {
            -size
        } else {
            size
        };
        lx += jump;
        jumped = Some(jump);
    }
    (clamp(exp(lx), range[0], range[1]), jumped)
}

/// Monthly: each industry's own demand index in each market (DEC-110): a
/// mean-reverting walk in logs with rare jumps, on a draw keyed by the
/// market's id, the industry's id and the month, so nothing in a town, and
/// no industry or market added later, reshuffles another's history.
pub(super) fn step_world(
    econ: &mut EconomyState,
    content: &ContentBundle,
    seed: u64,
    date: SimDate,
    out: &mut Outs<'_>,
) {
    let p = &content.economy.industries;
    let ni = content.industries.len();
    for (mk, key) in econ.markets.iter_mut().zip(&econ.market_keys) {
        if mk.index.len() != ni {
            mk.index.resize(ni, 1.0);
        }
        for (i, d) in content.industries.iter().enumerate() {
            let (x, jumped) = walk(mk.index[i], &d.world, p.world_range, seed, key, &d.id, date);
            mk.index[i] = x;
            if let Some(jump) = jumped {
                out.events.record(
                    date,
                    EventKind::IndustryShock {
                        industry: d.id.clone(),
                        market: key.clone(),
                        change_pct: (jump * 100.0).round() as i32,
                    },
                    Cause::Exogenous(Shock::WorldEconomy),
                );
            }
        }
    }
}

/// An industry's external demand (DEC-110, the seam for foreign trade): the
/// sum over markets of its share in the market × the market's index for it
/// (the market's cycle to the industry's cyclicality × its own walk there)
/// × the market's access. Returns the demand, its cycle part (the
/// share-weighted cycles) and the market that weighs most on it (the
/// lowest index × access among those it sells to). With one market at open
/// access this is exactly `cycle ^ cyclicality × index`.
pub fn external_demand(
    econ: &EconomyState,
    content: &ContentBundle,
    g: &Global,
    i: usize,
) -> (f64, f64, MarketId) {
    let Some(def) = content.industries.get(i) else {
        return (1.0, 1.0, MarketId(0));
    };
    let shares = content.market_shares(def);
    // Every market reads the world's cycle today. A neighbour's market
    // will read its own economy's.
    let cycle = pow(g.demand, def.cyclicality);
    let (mut ext, mut cyc) = (0.0, 0.0);
    let mut worst: Option<(usize, f64)> = None;
    for (m, share) in content.markets.iter().zip(&shares) {
        if *share <= 0.0 {
            continue;
        }
        let Some(s) = econ.market_keys.iter().position(|k| *k == m.id) else {
            continue;
        };
        let st = &econ.markets[s];
        let x = st.index.get(i).copied().unwrap_or(1.0);
        ext += share * ((cycle * x) * st.access);
        cyc += share * cycle;
        if worst.is_none_or(|w| x * st.access < w.1) {
            worst = Some((s, x * st.access));
        }
    }
    if worst.is_none() {
        // No market to sell to: neutral, so content without markets runs.
        return (cycle, cycle, MarketId(0));
    }
    (ext, cyc, MarketId(worst.map_or(0, |w| w.0) as u32))
}

// --- The monthly step -----------------------------------------------------------------

/// What every town's step shares this month: each industry's national
/// factors, and last month's jobs and long averages by region (so supply
/// links read the same values whichever town is stepped first).
pub(super) struct Month {
    pub split: Vec<bool>,
    /// Per industry: external demand, its cycle part, price, input, import
    /// competition.
    nat: Vec<[f64; 5]>,
    /// Per industry: the market that weighs most on its external demand.
    pub market: Vec<MarketId>,
    /// National household consumption in real terms against the start,
    /// over productivity (the home index every town shares).
    national_consumption: f64,
    /// Per industry: this month's capacity drift.
    trend: Vec<f64>,
    regions: usize,
    /// `[region × industry]`: last month's jobs, and the long average.
    ind_jobs: Vec<f64>,
    ind_trend: Vec<f64>,
    /// `[region × sector]`.
    sec_jobs: Vec<f64>,
    sec_trend: Vec<f64>,
    /// Each plant's town and industry index (`u16::MAX`: not in the registry).
    pub plant_at: Vec<(u32, u16)>,
    /// Each town's plants (indices into `EconomyState::plants`), as the
    /// month began: one opened during the month joins the next.
    pub town_plants: Vec<Vec<u32>>,
    pub recession: Option<EventId>,
    pub in_recession: bool,
    pub date: SimDate,
    pub seed: u64,
    /// The state's goods, by id.
    pub good_keys: Vec<ContentKey>,
}

impl Month {
    pub fn new(
        econ: &EconomyState,
        world: &World,
        content: &ContentBundle,
        g: &Global,
        nat: &National,
        date: SimDate,
        seed: u64,
    ) -> Month {
        let ni = content.industries.len();
        let ns = content.sectors.len();
        let offset = nat.manufacturing_trend - content.economy.manufacturing_trend;
        let mut natf = Vec::with_capacity(ni);
        let mut trend = Vec::with_capacity(ni);
        let mut market = Vec::with_capacity(ni);
        for (i, d) in content.industries.iter().enumerate() {
            let e = &d.exposure;
            // Prices, read through the goods registry.
            let (mut price, mut input) = (1.0, 1.0);
            for (c, x) in &e.price_index {
                let v = g.commodities.get(c.index()).copied().unwrap_or(1.0);
                debug_assert!(c.index() < COMMODITIES);
                if *x > 0.0 {
                    price *= pow(v, *x);
                } else if *x < 0.0 {
                    input *= pow(v, *x);
                }
            }
            // Import competition: how hard foreign makers bite.
            let compete = if e.import_exposure != 0.0 {
                pow(nat.manufacturing_competitiveness, e.import_exposure)
            } else {
                1.0
            };
            let (ext, cyc, weak) = external_demand(econ, content, g, i);
            market.push(weak);
            natf.push([ext, cyc, price, input, compete]);
            // The era moves every manufacturing industry's trend together.
            let era = match content.sectors.get(d.sector_index).map(|s| s.driver) {
                Some(SectorDriver::Industrial) => offset,
                _ => 0.0,
            };
            trend.push((d.trend + era) / 12.0);
        }
        let regions = world.regions.len().max(1);
        let mut ind_jobs = vec![0.0; regions * ni];
        let mut ind_trend = vec![0.0; regions * ni];
        let mut sec_jobs = vec![0.0; regions * ns];
        let mut sec_trend = vec![0.0; regions * ns];
        for (m, mu) in econ.munis.iter().enumerate() {
            let Some(w) = world.municipalities.get(m) else {
                break;
            };
            let r = w.region.index().min(regions - 1);
            for x in &mu.industries {
                if (x.k as usize) < ni {
                    ind_jobs[r * ni + x.k as usize] += x.jobs;
                    ind_trend[r * ni + x.k as usize] += x.trend_jobs;
                }
            }
            for (k, s) in mu.sectors.iter().enumerate().take(ns) {
                sec_jobs[r * ns + k] += s.jobs;
                sec_trend[r * ns + k] += s.trend_jobs;
            }
        }
        let plant_at = econ
            .plants
            .iter()
            .map(|p| {
                (
                    p.municipality.0,
                    content
                        .industry_index(&p.industry)
                        .map_or(u16::MAX, |i| i as u16),
                )
            })
            .collect();
        let price = nat.price_level.max(0.01);
        let (cons, base): (f64, f64) = econ.munis.iter().fold((0.0, 0.0), |a, m| {
            (a.0 + m.consumption, a.1 + m.base_consumption)
        });
        let national_consumption =
            (cons / price / base.max(1.0) / nat.productivity_index.max(0.01)).max(0.1);
        let mut town_plants = vec![Vec::new(); econ.munis.len()];
        for (i, p) in econ.plants.iter().enumerate() {
            if let Some(v) = town_plants.get_mut(p.municipality.index()) {
                v.push(i as u32);
            }
        }
        Month {
            split: split_flags(content),
            nat: natf,
            market,
            national_consumption,
            trend,
            regions,
            ind_jobs,
            ind_trend,
            sec_jobs,
            sec_trend,
            plant_at,
            town_plants,
            recession: g.recession.as_ref().and_then(|r| r.event),
            in_recession: g.recession.is_some(),
            date,
            seed,
            good_keys: econ.good_keys.clone(),
        }
    }

    /// The national product of an industry's demand (no town term): its
    /// external demand, prices and import competition.
    pub(super) fn national_demand(&self, i: usize) -> f64 {
        self.nat.get(i).map_or(1.0, |f| f[0] * f[2] * f[3] * f[4])
    }
}

/// A town's own jobs and long averages as the month begins.
struct Own {
    ind: Vec<(u16, f64, f64)>,
    sec: Vec<(f64, f64)>,
}

/// The supply term of an industry in a town, and its weakest link: for each
/// link, upstream jobs on a blend of the town and the rest of its region,
/// over the same blend of their long averages.
#[allow(clippy::too_many_arguments)]
fn supply(
    def: &IndustryDef,
    mo: &Month,
    own: &Own,
    own_trend: f64,
    region: usize,
    range: [f64; 2],
    ni: usize,
    ns: usize,
) -> (f64, Option<usize>) {
    if def.links.is_empty() {
        return (1.0, None);
    }
    let r = region.min(mo.regions - 1);
    let (mut num, mut den) = (0.0, 0.0);
    let mut worst: Option<(usize, f64)> = None;
    for (n, l) in def.links.iter().enumerate() {
        if l.from_index == usize::MAX {
            continue;
        }
        // (the town's jobs, its average, the region's jobs, its average)
        let (tj, tt, rj, rt) = if l.from_sector {
            if l.from_index >= ns {
                continue;
            }
            let (j, t) = own.sec.get(l.from_index).copied().unwrap_or((0.0, 0.0));
            (
                j,
                t,
                mo.sec_jobs[r * ns + l.from_index],
                mo.sec_trend[r * ns + l.from_index],
            )
        } else {
            if l.from_index >= ni {
                continue;
            }
            let (j, t) = own
                .ind
                .iter()
                .find(|x| x.0 as usize == l.from_index)
                .map_or((0.0, 0.0), |x| (x.1, x.2));
            (
                j,
                t,
                mo.ind_jobs[r * ni + l.from_index],
                mo.ind_trend[r * ni + l.from_index],
            )
        };
        let a = l.local * tt + (1.0 - l.local) * (rt - tt).max(0.0);
        if a < 1.0 {
            continue;
        }
        let now = l.local * tj + (1.0 - l.local) * (rj - tj).max(0.0);
        let u = clamp(now / a, range[0], range[1]);
        num += l.weight * a * u;
        den += l.weight * a;
        if worst.is_none_or(|w| u < w.1) {
            worst = Some((n, u));
        }
    }
    if den < 1.0 {
        return (1.0, None);
    }
    // An industry runs on its upstream only as far as there is upstream to
    // run on: where what is nearby is small beside the industry itself, the
    // link counts for that much less (a works of a thousand does not stop
    // for a foundry of twenty).
    let reliance = clamp(den / own_trend.max(1.0), 0.0, 1.0);
    (
        pow(num / den, def.link_power * reliance),
        worst.map(|w| w.0),
    )
}

/// The weakest supply link of an industry in a town as things stand: the
/// upstream industry or sector a "why" line names.
pub fn weakest_link(
    econ: &EconomyState,
    world: &World,
    content: &ContentBundle,
    m: usize,
    k: usize,
) -> Option<mandate_content::ContentKey> {
    let def = content.industries.get(k)?;
    let mu = econ.munis.get(m)?;
    let mo = Month::new(
        econ,
        world,
        content,
        &econ.global,
        &econ.national,
        SimDate(0),
        0,
    );
    let own = own_of(mu);
    let own_trend = mu
        .industries
        .iter()
        .find(|x| x.k as usize == k)
        .map_or(0.0, |x| x.trend_jobs);
    let region = world.municipalities.get(m)?.region.index();
    let (_, worst) = supply(
        def,
        &mo,
        &own,
        own_trend,
        region,
        content.economy.industries.link_range,
        content.industries.len(),
        content.sectors.len(),
    );
    worst.map(|n| def.links[n].from.clone())
}

fn own_of(mu: &MuniEcon) -> Own {
    Own {
        ind: mu
            .industries
            .iter()
            .map(|x| (x.k, x.jobs, x.trend_jobs))
            .collect(),
        sec: mu.sectors.iter().map(|s| (s.jobs, s.trend_jobs)).collect(),
    }
}

fn applies(s: &ActiveShock, def: &IndustryDef) -> bool {
    match &s.industry {
        Some(i) => *i == def.id,
        None => s.sector == def.sector,
    }
}

/// What a town's step needs from the rest of the month's work.
pub(super) struct TownIn<'a> {
    pub m: usize,
    pub world: &'a World,
    pub content: &'a ContentBundle,
    pub region_env: Option<&'a RegionEnv>,
    pub mods: &'a crate::policy::PolicyMods,
    /// The town's unit labour costs.
    pub ulc: f64,
    /// The nation's price level.
    pub price: f64,
    /// Local labour-market tightness (as wages read it).
    pub tight: f64,
    pub local_wage: f64,
    /// The town's own home-demand indices: local consumption, building,
    /// public spending (each 1 at the start).
    pub local_consumption: f64,
    pub construction: f64,
    pub public: f64,
}

/// Demand at home for an industry in a town: its weights over the home
/// sources (omitted: national consumption).
fn home_demand(def: &IndustryDef, mo: &Month, t: &TownIn<'_>) -> f64 {
    if def.home.is_empty() {
        return mo.national_consumption;
    }
    let (mut num, mut den) = (0.0, 0.0);
    for (s, w) in &def.home {
        let v = match s {
            HomeSource::NationalConsumption => mo.national_consumption,
            HomeSource::LocalConsumption => t.local_consumption,
            HomeSource::Construction => t.construction,
            HomeSource::Public => t.public,
        };
        num += w * v;
        den += w;
    }
    if den > 0.0 {
        num / den
    } else {
        mo.national_consumption
    }
}

/// One town's industries for one month: demand, the plants' closure
/// process, the unnamed firms, jobs, wages, openings, and the split
/// sectors' totals.
pub(super) fn step_town(
    mo: &Month,
    t: &TownIn<'_>,
    mu: &mut MuniEcon,
    all_plants: &mut Vec<Plant>,
    plant_ids: &mut mandate_core::IdAllocator,
    out: &mut Outs<'_>,
) {
    let content = t.content;
    let p = &content.economy.industries;
    let ni = content.industries.len();
    let ns = content.sectors.len();
    let class = t.world.municipalities[t.m].class;
    let region = t.world.municipalities[t.m].region.index();
    let own = own_of(mu);
    let sector_cap: Vec<f64> = mu.sectors.iter().map(|s| s.capacity).collect();
    let town_jobs = mu.total_jobs();
    let natural = content.economy.natural_unemployment;
    let slack = mu.unemployment - natural;
    let adjust = clamp(slack * 0.25, -0.01, 0.04) / 12.0;
    let mut invest = 0.0;
    for n in 0..mu.industries.len() {
        let k = mu.industries[n].k as usize;
        let Some(def) = content.industries.get(k) else {
            continue;
        };
        let e = &def.exposure;
        // Demand: the product of the named factors.
        let nat = mo.nat[k];
        let demand_shock: f64 = mu
            .shocks
            .iter()
            .filter(|s| s.demand && applies(s, def))
            .map(|s| s.factor)
            .product();
        let stock = match (&e.stock, t.region_env) {
            (Some(s), Some(env)) => {
                let v = match s.of {
                    ResourceStock::Mineral => env.mineral_stock,
                    ResourceStock::Timber => env.timber_stock,
                    ResourceStock::Fish => env.fish_stock,
                };
                (v, pow(v.max(0.05), s.power))
            }
            _ => (1.0, 1.0),
        };
        let water = match (e.water != 0.0, t.region_env) {
            (true, Some(env)) => pow(env.water.max(0.2), e.water),
            _ => 1.0,
        };
        let labour = if e.labour_cost != 0.0 {
            pow(t.ulc, -e.labour_cost)
        } else {
            1.0
        };
        let own_trend = mu.industries[n].trend_jobs;
        let (sup, worst_link) = supply(def, mo, &own, own_trend, region, p.link_range, ni, ns);
        // Demand: what it sells abroad and what it sells at home, by its
        // export share (all of it abroad: exactly the external demand).
        let e_share = def.export_share;
        let home = if e_share < 1.0 {
            home_demand(def, mo, t)
        } else {
            1.0
        };
        let demand = (e_share * nat[0] + (1.0 - e_share) * home) * demand_shock;
        // The same terms as levels, for blame (never for the number).
        let f: [f64; FACTORS] = [
            nat[1],
            nat[0] / nat[1].max(1e-9) * demand_shock,
            nat[2],
            nat[3],
            stock.1,
            water,
            nat[4],
            labour,
            sup,
            home,
        ];
        let driver = demand * nat[2] * nat[3] * stock.1 * water * nat[4] * labour * sup;
        let support = 1.0
            + t.mods
                .subsidy_elasticity
                .get(def.sector_index)
                .copied()
                .unwrap_or(0.0)
                * t.mods.subsidy.get(def.sector_index).copied().unwrap_or(0.0);
        let floor = 1.0 - 0.4 * e.low_pay * t.mods.wage_floor;
        // What the closure rule reads: demand as policy leaves it. A
        // disruption (a hazard's shock) stays out, so a flooded works is
        // not closed by the rule.
        let effective = driver * support * floor;
        let disruption: f64 = mu
            .shocks
            .iter()
            .filter(|s| !s.demand && applies(s, def))
            .map(|s| s.factor)
            .product();
        let st = &mut mu.industries[n];
        for (j, x) in f.iter().enumerate() {
            let l = ln(x.max(1e-6)) as f32;
            st.factors[j] += (l - st.factors[j]) / p.factor_months as f32;
        }
        // Capacity drifts with the industry's trend: firms and plants alike.
        let drift = 1.0 + mo.trend[k];
        st.small *= drift;
        let mine = mo.town_plants.get(t.m).map_or(&[][..], |v| v.as_slice());
        let mut open_cap = 0.0;
        for &i in mine {
            let i = i as usize;
            if mo.plant_at[i].1 == k as u16 && all_plants[i].is_open() {
                all_plants[i].capacity *= drift;
                open_cap += all_plants[i].capacity;
            }
        }
        st.capacity = st.small + open_cap;
        // The plants' closure process.
        let upstream = worst_link.map(|l| def.links[l].from.clone());
        for &i in mine {
            let i = i as usize;
            if mo.plant_at[i].1 != k as u16 || !all_plants[i].is_open() {
                continue;
            }
            let step = plants::Step {
                market: mo.market[k],
                effective,
                def,
                date: mo.date,
                seed: mo.seed,
                recession: mo.recession,
                in_recession: mo.in_recession,
                upstream: upstream.clone(),
            };
            plants::process(&step, p, &mut all_plants[i], st, &mut mu.last_notice, out);
        }
        // The unnamed firms have no event: they thin out while demand is
        // under the notice line.
        if def.plants.closes
            && effective < p.notice_line
            && sector_cap.get(def.sector_index).copied().unwrap_or(0.0) > p.min_capacity
        {
            st.small *= 1.0 - p.small_decay;
            st.capacity = st.small + plants::open_capacity(all_plants, mo, t.m, k);
        }
        // A long boom reopens the works.
        let demand_smooth = st.smoothed_demand();
        if def.plants.opens == PlantOpens::Boom {
            st.boom = if demand_smooth > p.boom_line && stock.0 > p.boom_stock {
                st.boom.saturating_add(1)
            } else {
                0
            };
            if st.boom >= p.boom_months
                && plants::reopen_after_boom(mo, p, def, k, t, st, all_plants, plant_ids, out)
            {
                st.boom = 0;
            }
        }
        // Idle labour draws new capacity (and a tight market sheds it).
        if def.plants.opens == PlantOpens::Investment {
            let amount = st.capacity * adjust;
            if amount >= 0.0 {
                invest += amount;
            } else {
                st.small = (st.small + amount).max(0.0);
                st.capacity = st.small + plants::open_capacity(all_plants, mo, t.m, k);
            }
        }
        // Jobs follow demand.
        let extra = match (
            t.mods.capacity.get(def.sector_index),
            sector_cap.get(def.sector_index),
        ) {
            (Some(x), Some(c)) if *c > 0.0 => x * st.capacity / c,
            _ => 0.0,
        };
        let target = (st.capacity + extra).max(0.0) * effective * disruption;
        let speed = if target < st.jobs { 0.16 } else { 0.1 };
        st.jobs = (st.jobs + (target - st.jobs) * speed).max(0.0);
        st.productivity *= 1.0 + def.productivity_growth / 12.0;
        st.trend_jobs += (st.jobs - st.trend_jobs) / p.trend_months;
        let base = def.base_wage * t.local_wage;
        let target_wage = base * t.price * st.productivity * (1.0 + 2.0 * t.tight);
        st.wage += (target_wage - st.wage) * 0.08;
    }
    // What idle labour drew this month: a closed works reopens when the
    // pool is a plant's size; with none to reopen, the unnamed firms grow.
    if invest > 0.0 || mu.prospect > 0.0 {
        plants::invest(
            mo, p, t, mu, all_plants, plant_ids, invest, class, town_jobs, out,
        );
    }
    // Capacity, exactly: the unnamed firms plus the open plants (one opened
    // this month is past the month's index).
    let opened = all_plants.get(mo.plant_at.len()..).unwrap_or(&[]);
    for st in mu.industries.iter_mut() {
        let k = st.k as usize;
        let mut open = plants::open_capacity(all_plants, mo, t.m, k);
        for pl in opened {
            if pl.municipality.index() == t.m
                && pl.is_open()
                && content.industry_index(&pl.industry) == Some(k)
            {
                open += pl.capacity;
            }
        }
        st.capacity = st.small + open;
    }
    write_totals(mu, content, &mo.split);
}
