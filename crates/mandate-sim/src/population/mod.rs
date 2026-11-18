//! Population (DEC-018): 12 aggregate cohorts per neighbourhood, simulated
//! everywhere at all times. Owns resident counts and cohort political state.

use crate::scheduler::{Effect, Outs, Reads};
use crate::world::{MunicipalityId, NeighborhoodKind, RegionId, SettlementClass, World};
use mandate_core::math::{clamp, tanh};
use mandate_core::{SimRng, Stream};
use serde::{Deserialize, Serialize};

pub const AGE_BANDS: usize = 4;
pub const CLASSES: usize = 3;
pub const COHORTS: usize = AGE_BANDS * CLASSES;
pub const ATTITUDES: usize = 8;

/// Attitude dimensions, each in [-1, 1].
pub const ATTITUDE_KEYS: [&str; ATTITUDES] = [
    "redistribution",
    "tax_tolerance",
    "social_liberalism",
    "environment",
    "law_and_order",
    "localism",
    "tradition",
    "reform_appetite",
];

/// Age bands: 18–29, 30–44, 45–64, 65+.
pub const BAND_YEARS: [f64; AGE_BANDS] = [12.0, 15.0, 20.0, 15.0];
/// Labour-force participation by age band.
pub const PARTICIPATION: [f64; AGE_BANDS] = [0.72, 0.86, 0.78, 0.07];

#[inline]
pub fn cohort_index(age: usize, class: usize) -> usize {
    age * CLASSES + class
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cohort {
    pub count: u32,
    pub attitudes: [f32; ATTITUDES],
    pub trust: f32,
    pub turnout: f32,
    /// Approval of the local government (0..1).
    pub gov_approval: f32,
    /// How well this cohort knows the player (0..1).
    pub player_awareness: f32,
    /// Opinion of the player among those aware (−1..1).
    pub player_approval: f32,
    /// Attachment to each party (indexed by party id; sums to ≤ 1).
    pub party_id: Vec<f32>,
    /// Turnout at the last election (habit, DEC-071 §4).
    pub last_turnout: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HoodPop {
    pub children: u32,
    /// Indexed by `cohort_index(age, class)`.
    pub cohorts: Vec<Cohort>,
}

impl HoodPop {
    pub fn adults(&self) -> u64 {
        self.cohorts.iter().map(|c| c.count as u64).sum()
    }
    pub fn total(&self) -> u64 {
        self.adults() + self.children as u64
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DemographyYear {
    pub year: u32,
    pub births: u64,
    pub deaths: u64,
    pub immigration: u64,
    pub internal_moves: u64,
    pub population: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PopulationState {
    pub hoods: Vec<HoodPop>,
    /// Internal moves accumulated this year.
    pub moves_ytd: u64,
    pub history: Vec<DemographyYear>,
    /// The last election whose turnout has been folded into habits.
    #[serde(default)]
    pub last_election_seen: Option<mandate_core::SimDate>,
}

impl PopulationState {
    pub fn total(&self) -> u64 {
        self.hoods.iter().map(|h| h.total()).sum()
    }
    pub fn hood_total(&self, i: usize) -> u64 {
        self.hoods.get(i).map(|h| h.total()).unwrap_or(0)
    }
    pub fn municipality_total(&self, world: &World, m: MunicipalityId) -> u64 {
        world.hoods_of(m).map(|i| self.hood_total(i)).sum()
    }
    pub fn region_total(&self, world: &World, r: RegionId) -> u64 {
        world
            .hoods_of_region(r)
            .iter()
            .map(|&i| self.hood_total(i))
            .sum()
    }
    /// Adults per (age band, class) in a municipality.
    pub fn municipality_cohorts(&self, world: &World, m: MunicipalityId) -> [f64; COHORTS] {
        let mut out = [0.0; COHORTS];
        for i in world.hoods_of(m) {
            for (k, c) in self.hoods[i].cohorts.iter().enumerate() {
                out[k] += c.count as f64;
            }
        }
        out
    }
    /// Labour force (participation-weighted adults) by class.
    pub fn labour_force_by_class(&self, world: &World, m: MunicipalityId) -> [f64; CLASSES] {
        let c = self.municipality_cohorts(world, m);
        let mut lf = [0.0; CLASSES];
        for a in 0..AGE_BANDS {
            for (k, v) in lf.iter_mut().enumerate() {
                *v += c[cohort_index(a, k)] * PARTICIPATION[a];
            }
        }
        lf
    }
}

// --- Initialisation ---------------------------------------------------------

fn class_mix(kind: NeighborhoodKind, wealthy: bool) -> [f64; CLASSES] {
    let base = match kind {
        NeighborhoodKind::Core => [0.34, 0.45, 0.21],
        NeighborhoodKind::Inner => [0.30, 0.47, 0.23],
        NeighborhoodKind::Waterfront => [0.30, 0.44, 0.26],
        NeighborhoodKind::Industrial => [0.55, 0.40, 0.05],
        NeighborhoodKind::Suburb => [0.22, 0.55, 0.23],
        NeighborhoodKind::Rural => [0.42, 0.48, 0.10],
    };
    if wealthy {
        [base[0] * 0.75, base[1], base[2] * 1.4]
    } else {
        base
    }
}

fn age_mix(kind: NeighborhoodKind) -> ([f64; AGE_BANDS], f64) {
    // Adult age shares and children per adult.
    match kind {
        NeighborhoodKind::Core | NeighborhoodKind::Inner => ([0.27, 0.29, 0.29, 0.15], 0.18),
        NeighborhoodKind::Waterfront => ([0.22, 0.27, 0.31, 0.20], 0.2),
        NeighborhoodKind::Industrial => ([0.22, 0.28, 0.33, 0.17], 0.26),
        NeighborhoodKind::Suburb => ([0.17, 0.30, 0.33, 0.20], 0.3),
        NeighborhoodKind::Rural => ([0.15, 0.24, 0.35, 0.26], 0.27),
    }
}

/// Starting attitudes for a cohort (conditioned on context, not determined).
fn seed_attitudes(
    rng: &mut SimRng,
    age: usize,
    class: usize,
    kind: NeighborhoodKind,
    regional: &[f64; ATTITUDES],
) -> [f32; ATTITUDES] {
    let urban = matches!(
        kind,
        NeighborhoodKind::Core | NeighborhoodKind::Inner | NeighborhoodKind::Waterfront
    );
    let rural = kind == NeighborhoodKind::Rural;
    let young = age == 0;
    let senior = age == 3;
    let class_lean = [0.35, 0.0, -0.35][class];
    let b = |c: bool, v: f64| if c { v } else { 0.0 };
    let base = [
        class_lean + b(young, 0.1),
        [0.05, -0.08, -0.28][class] + b(senior, -0.05),
        b(young, 0.35) + b(senior, -0.3) + b(urban, 0.15) + b(rural, -0.2),
        b(young, 0.25) + b(rural, -0.1) + b(urban, 0.08),
        b(senior, 0.3) + b(rural, 0.15) + b(young, -0.1),
        b(rural, 0.3) + b(urban, -0.1),
        b(senior, 0.35) + b(rural, 0.25) + b(young, -0.2),
        b(young, 0.2) + [0.1, 0.0, -0.1][class],
    ];
    let mut out = [0.0f32; ATTITUDES];
    for k in 0..ATTITUDES {
        out[k] = clamp(base[k] + regional[k] + rng.normal(0.0, 0.08), -1.0, 1.0) as f32;
    }
    out
}

/// Build the starting population from worldgen's per-neighbourhood totals.
pub fn init(world: &World, totals: &[u64], world_seed: u64, wealthy: bool) -> PopulationState {
    let region_culture: Vec<[f64; ATTITUDES]> = (0..world.regions.len())
        .map(|r| {
            let mut rng =
                mandate_core::rng::keyed(world_seed, Stream::Population, &[0xC0170, r as u64]);
            let mut v = [0.0; ATTITUDES];
            for x in v.iter_mut() {
                *x = rng.normal(0.0, 0.14);
            }
            v
        })
        .collect();
    let mut hoods = Vec::with_capacity(world.neighborhoods.len());
    for (i, n) in world.neighborhoods.iter().enumerate() {
        let total = totals.get(i).copied().unwrap_or(0) as f64;
        let (ages, kids_per_adult) = age_mix(n.kind);
        let adults = total / (1.0 + kids_per_adult);
        let classes = class_mix(n.kind, wealthy);
        let csum: f64 = classes.iter().sum();
        let region = world.municipalities[n.municipality.index()].region.index();
        let mut rng =
            mandate_core::rng::keyed(world_seed, Stream::Population, &[0xC0407, i as u64]);
        let mut cohorts = Vec::with_capacity(COHORTS);
        for (a, share_a) in ages.iter().enumerate() {
            for (c, share_c) in classes.iter().enumerate() {
                let count = (adults * share_a * share_c / csum).round().max(0.0) as u32;
                let turnout = [0.42, 0.55, 0.66, 0.74][a] + [-0.08, 0.0, 0.1][c];
                cohorts.push(Cohort {
                    count,
                    attitudes: seed_attitudes(&mut rng, a, c, n.kind, &region_culture[region]),
                    trust: clamp(
                        0.55 + [-0.08, 0.0, 0.06][c] + rng.normal(0.0, 0.05),
                        0.0,
                        1.0,
                    ) as f32,
                    turnout: clamp(turnout + rng.normal(0.0, 0.03), 0.05, 0.98) as f32,
                    gov_approval: clamp(0.5 + rng.normal(0.0, 0.08), 0.0, 1.0) as f32,
                    player_awareness: 0.0,
                    player_approval: 0.0,
                    party_id: Vec::new(),
                    last_turnout: turnout as f32,
                });
            }
        }
        let adults_total: u64 = cohorts.iter().map(|c| c.count as u64).sum();
        let children = (total as u64).saturating_sub(adults_total) as u32;
        hoods.push(HoodPop { children, cohorts });
    }
    PopulationState {
        hoods,
        moves_ytd: 0,
        history: Vec::new(),
        last_election_seen: None,
    }
}

// --- Systems ------------------------------------------------------------------

/// Move `n` people from cohort `from` to cohort `to` within a neighbourhood,
/// blending attitudes by headcount.
fn shift(h: &mut HoodPop, from: usize, to: usize, n: u32) {
    let n = n.min(h.cohorts[from].count);
    if n == 0 || from == to {
        return;
    }
    let src = h.cohorts[from].clone();
    let dst = &mut h.cohorts[to];
    let total = dst.count as f64 + n as f64;
    let w_new = n as f64 / total;
    for k in 0..ATTITUDES {
        dst.attitudes[k] =
            (dst.attitudes[k] as f64 * (1.0 - w_new) + src.attitudes[k] as f64 * w_new) as f32;
    }
    let mix = |a: f32, b: f32| (a as f64 * (1.0 - w_new) + b as f64 * w_new) as f32;
    dst.trust = mix(dst.trust, src.trust);
    dst.turnout = mix(dst.turnout, src.turnout);
    dst.gov_approval = mix(dst.gov_approval, src.gov_approval);
    dst.player_awareness = mix(dst.player_awareness, src.player_awareness);
    dst.player_approval = mix(dst.player_approval, src.player_approval);
    dst.count += n;
    h.cohorts[from].count -= n;
}

/// Stochastic rounding of an expected count (deterministic via the RNG stream).
fn draw(rng: &mut SimRng, expected: f64) -> u32 {
    if expected <= 0.0 {
        return 0;
    }
    let base = expected.floor();
    (base
        + if rng.chance(expected - base) {
            1.0
        } else {
            0.0
        }) as u32
}

/// Yearly demography: births, ageing, deaths, class mobility, immigration.
pub fn demography(pop: &mut PopulationState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let p = &r.content.economy;
    let econ = r.economy();
    let rng = out.rng.get(Stream::Population);
    let mut year = DemographyYear {
        year: r.date.calendar().year.saturating_sub(1),
        ..Default::default()
    };
    for (i, n) in r.world.neighborhoods.iter().enumerate() {
        let m = n.municipality.index();
        let unemployment = econ.munis.get(m).map(|e| e.unemployment).unwrap_or(0.05);
        let knowledge = econ
            .munis
            .get(m)
            .map(|e| e.knowledge_share())
            .unwrap_or(0.1);
        let class = r.world.municipalities[m].class;
        let h = &mut pop.hoods[i];

        // Births to adults aged 18–44.
        let parents: u64 = (0..CLASSES)
            .map(|c| {
                (h.cohorts[cohort_index(0, c)].count + h.cohorts[cohort_index(1, c)].count) as u64
            })
            .sum();
        let births = draw(rng, parents as f64 * p.birth_rate);
        // Children come of age into the youngest band, by parents' class mix.
        let coming_of_age = draw(rng, h.children as f64 / 18.0);
        let coming_of_age = coming_of_age.min(h.children);
        let child_deaths =
            draw(rng, h.children as f64 * p.death_rates[0]).min(h.children - coming_of_age);
        h.children = h.children - coming_of_age - child_deaths + births;
        // Split exactly (largest share takes the remainder) so people are conserved.
        let parent_mix: Vec<u64> = (0..CLASSES)
            .map(|c| h.cohorts[cohort_index(1, c)].count as u64)
            .collect();
        let pm_sum: u64 = parent_mix.iter().sum::<u64>().max(1);
        let mut given = 0u32;
        for c in 0..CLASSES {
            let n = (coming_of_age as u64 * parent_mix[c] / pm_sum) as u32;
            h.cohorts[cohort_index(0, c)].count += n;
            given += n;
        }
        let biggest = (0..CLASSES)
            .max_by_key(|&c| (parent_mix[c], c))
            .unwrap_or(1);
        h.cohorts[cohort_index(0, biggest)].count += coming_of_age - given;
        year.births += births as u64;
        year.deaths += child_deaths as u64;

        // Ageing (oldest band first so people move one band per year).
        for a in (0..AGE_BANDS - 1).rev() {
            for c in 0..CLASSES {
                let from = cohort_index(a, c);
                let n_age = draw(rng, h.cohorts[from].count as f64 / BAND_YEARS[a]);
                shift(h, from, cohort_index(a + 1, c), n_age);
            }
        }
        // Deaths.
        for a in 0..AGE_BANDS {
            for c in 0..CLASSES {
                let k = cohort_index(a, c);
                let d = draw(
                    rng,
                    h.cohorts[k].count as f64 * p.death_rates[a + 1] * [1.15, 1.0, 0.85][c],
                )
                .min(h.cohorts[k].count);
                h.cohorts[k].count -= d;
                year.deaths += d as u64;
            }
        }
        // Class mobility among working ages.
        for a in 0..3 {
            let up_low = 0.02 * (1.0 - unemployment * 4.0).max(0.0) + 0.01 * knowledge;
            let down_mid = 0.008 + unemployment * 0.08;
            let up_mid = 0.006 + 0.03 * knowledge;
            let down_high = 0.006 + unemployment * 0.03;
            let (lo, mi, hi) = (cohort_index(a, 0), cohort_index(a, 1), cohort_index(a, 2));
            let n1 = draw(rng, h.cohorts[lo].count as f64 * up_low);
            let n2 = draw(rng, h.cohorts[mi].count as f64 * down_mid);
            let n3 = draw(rng, h.cohorts[mi].count as f64 * up_mid);
            let n4 = draw(rng, h.cohorts[hi].count as f64 * down_high);
            shift(h, lo, mi, n1);
            shift(h, mi, lo, n2);
            shift(h, mi, hi, n3);
            shift(h, hi, mi, n4);
        }
        // International immigration to larger places.
        let imm_rate = match class {
            SettlementClass::Metro => 0.006,
            SettlementClass::City => 0.0035,
            SettlementClass::Town => 0.0012,
            SettlementClass::Rural => 0.0004,
        };
        let imm = draw(rng, h.adults() as f64 * imm_rate);
        h.cohorts[cohort_index(0, 0)].count += imm / 2;
        h.cohorts[cohort_index(0, 1)].count += imm - imm / 2;
        year.immigration += imm as u64;
    }
    year.internal_moves = pop.moves_ytd;
    pop.moves_ytd = 0;
    year.population = pop.total();
    pop.history.push(year);
    if pop.history.len() > 60 {
        pop.history.remove(0);
    }
}

/// Monthly internal migration toward attractive municipalities, plus
/// displacement after disasters.
pub fn migration(pop: &mut PopulationState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let econ = r.economy();
    let world = r.world;
    let nm = world.municipalities.len();
    if econ.munis.len() != nm || nm == 0 {
        return;
    }
    let mut extra_out = vec![0.0f64; nm];
    for e in out
        .effects
        .take(|e| matches!(e, Effect::Displacement { .. }))
    {
        if let Effect::Displacement {
            municipality,
            fraction,
            ..
        } = e
            && let Some(x) = extra_out.get_mut(municipality as usize)
        {
            *x += fraction;
        }
    }
    let attract: Vec<f64> = econ.munis.iter().map(|m| m.attractiveness).collect();
    let mean = attract.iter().sum::<f64>() / nm as f64;
    let rate = r.content.economy.migration_rate;
    let rng = out.rng.get(Stream::Population);

    // Movers leave per origin (only the two mobile bands) and carry their
    // origin's attitudes. Destinations follow a gravity model (DEC-071):
    // attractiveness × housing room × size^½ × (1 + d/d₀)^−γ.
    let mobile: Vec<usize> = (0..2)
        .flat_map(|a| (0..CLASSES).map(move |c| cohort_index(a, c)))
        .collect();
    let mut movers = vec![[0u64; COHORTS]; nm];
    let mut mover_att = vec![[[0.0f64; ATTITUDES]; COHORTS]; nm];
    let np = pop
        .hoods
        .first()
        .and_then(|h| h.cohorts.first())
        .map(|c| c.party_id.len())
        .unwrap_or(0);
    let mut mover_pid = vec![vec![vec![0.0f64; np]; COHORTS]; nm];
    for (m, mu) in world.municipalities.iter().enumerate() {
        let push = rate * (1.0 - tanh((attract[m] - mean) * 1.5));
        let out_share = (push + extra_out[m]).min(0.5);
        for h in &mu.neighborhoods {
            let hp = &mut pop.hoods[h.index()];
            for &k in &mobile {
                let n = draw(rng, hp.cohorts[k].count as f64 * out_share);
                if n == 0 {
                    continue;
                }
                hp.cohorts[k].count -= n;
                movers[m][k] += n as u64;
                for (j, a) in hp.cohorts[k].attitudes.iter().enumerate() {
                    mover_att[m][k][j] += *a as f64 * n as f64;
                }
                for (j, a) in hp.cohorts[k].party_id.iter().enumerate().take(np) {
                    mover_pid[m][k][j] += *a as f64 * n as f64;
                }
            }
        }
    }
    let pull: Vec<f64> = (0..nm)
        .map(|m| {
            let e = &econ.munis[m];
            let room = (e.housing.vacancy + 0.01).max(0.002);
            mandate_core::math::exp(2.0 * (attract[m] - mean))
                * room
                * mandate_core::math::sqrt(e.labour_force.max(1.0))
        })
        .collect();
    let d0 = gravity_scale(world);
    // Arrivals are pooled per destination and cohort, then settled once.
    let mut arr_n = vec![[0u64; COHORTS]; nm];
    let mut arr_att = vec![[[0.0f64; ATTITUDES]; COHORTS]; nm];
    let mut arr_pid = vec![vec![vec![0.0f64; np]; COHORTS]; nm];
    let mut weights = vec![0.0f64; nm];
    for origin in 0..nm {
        if movers[origin].iter().all(|n| *n == 0) {
            continue;
        }
        let (ox, oy) = world.municipalities[origin].center;
        for d in 0..nm {
            weights[d] = if d == origin && nm > 1 {
                0.0
            } else {
                let (dx, dy) = world.municipalities[d].center;
                let ddx = dx as f64 - ox as f64;
                let ddy = dy as f64 - oy as f64;
                let dist = mandate_core::math::sqrt(ddx * ddx + ddy * ddy);
                // (1 + d/d₀)^−γ with γ = 1 (DEC-071; literature 0.5–2).
                pull[d] / (1.0 + dist / d0)
            };
        }
        let wsum: f64 = weights.iter().sum::<f64>().max(1e-12);
        let last = (0..nm).rev().find(|d| weights[*d] > 0.0).unwrap_or(origin);
        for &k in &mobile {
            let total = movers[origin][k];
            if total == 0 {
                continue;
            }
            pop.moves_ytd += total;
            let mut remaining = total;
            for m in 0..nm {
                if weights[m] <= 0.0 {
                    continue;
                }
                let share = if m == last {
                    remaining
                } else {
                    ((total as f64) * weights[m] / wsum).floor() as u64
                };
                let share = share.min(remaining);
                remaining -= share;
                if share == 0 {
                    continue;
                }
                let f = share as f64 / total as f64;
                arr_n[m][k] += share;
                for j in 0..ATTITUDES {
                    arr_att[m][k][j] += mover_att[origin][k][j] * f;
                }
                for j in 0..np {
                    arr_pid[m][k][j] += mover_pid[origin][k][j] * f;
                }
            }
        }
    }
    for m in 0..nm {
        for &k in &mobile {
            let n = arr_n[m][k];
            if n == 0 {
                continue;
            }
            let att: Vec<f32> = arr_att[m][k]
                .iter()
                .map(|s| (s / n as f64) as f32)
                .collect();
            let pid: Vec<f32> = arr_pid[m][k]
                .iter()
                .map(|s| (s / n as f64) as f32)
                .collect();
            settle(pop, world, m, k, n, &att, &pid);
        }
    }
}

/// d₀: a quarter of the settled extent, so decay is scale-free across world sizes.
fn gravity_scale(world: &World) -> f64 {
    let (mut minx, mut miny, mut maxx, mut maxy) = (u32::MAX, u32::MAX, 0u32, 0u32);
    for m in &world.municipalities {
        minx = minx.min(m.center.0);
        miny = miny.min(m.center.1);
        maxx = maxx.max(m.center.0);
        maxy = maxy.max(m.center.1);
    }
    (maxx.saturating_sub(minx).max(maxy.saturating_sub(miny)) as f64 / 4.0).max(1.0)
}

/// Spread arrivals across a municipality's neighbourhoods by adult count.
fn settle(
    pop: &mut PopulationState,
    world: &World,
    m: usize,
    k: usize,
    share: u64,
    att: &[f32],
    pid: &[f32],
) {
    let hoods = &world.municipalities[m].neighborhoods;
    let adults: Vec<u64> = hoods
        .iter()
        .map(|h| pop.hoods[h.index()].adults().max(1))
        .collect();
    let asum: u64 = adults.iter().sum();
    let mut left = share;
    for (j, h) in hoods.iter().enumerate() {
        let n = if j + 1 == hoods.len() {
            left
        } else {
            (share * adults[j] / asum).min(left)
        };
        left -= n;
        if n == 0 {
            continue;
        }
        let c = &mut pop.hoods[h.index()].cohorts[k];
        let w = n as f64 / (c.count as f64 + n as f64);
        for (a, v) in c.attitudes.iter_mut().zip(att) {
            *a = (*a as f64 * (1.0 - w) + *v as f64 * w) as f32;
        }
        for (a, v) in c.party_id.iter_mut().zip(pid) {
            *a = (*a as f64 * (1.0 - w) + *v as f64 * w) as f32;
        }
        // Newcomers do not know local politicians yet.
        c.player_awareness = (c.player_awareness as f64 * (1.0 - w)) as f32;
        c.count += n as u32;
    }
}

/// Monthly opinion (DEC-025): experienced conditions and anticipated reactions
/// to policies shape approval of the local government and of the player.
pub fn opinion(pop: &mut PopulationState, r: &Reads<'_>, _out: &mut Outs<'_>) {
    use crate::policy::PolicyStatus;
    let econ = r.economy();
    let policy = r.policy();
    let fin = r.finance();
    let pol = r.politics();
    // Retrospection is benchmarked against the nation (DEC-071 §2): a national
    // recession is not the town hall's fault.
    let dn = {
        let h = &econ.national.history;
        match (h.last(), h.get(h.len().saturating_sub(13))) {
            (Some(a), Some(b)) => (a.unemployment - b.unemployment) as f64,
            _ => 0.0,
        }
    };
    let params = &r.content.politics;
    let parties = &pol.parties;
    // Party attachment moves slowly: once a year is enough (and cheap).
    let january = r.date.calendar().month == 1;
    // Fold the last election's turnout into habits once.
    let new_election = pol.elections.last_election.is_some()
        && pol.elections.last_election != pop.last_election_seen;
    if new_election {
        for (h, t) in &pol.elections.last_hood_turnout {
            if let Some(hp) = pop.hoods.get_mut(*h as usize) {
                for c in hp.cohorts.iter_mut() {
                    // Cohorts differ around the neighbourhood mean by their own propensity.
                    c.last_turnout =
                        clamp(*t as f64 * (0.6 + 0.8 * c.turnout as f64), 0.02, 0.95) as f32;
                }
            }
        }
        pop.last_election_seen = pol.elections.last_election;
    }
    for (m, mu) in r.world.municipalities.iter().enumerate() {
        let Some(e) = econ.munis.get(m) else { continue };
        let mayor_party = pol
            .councils
            .get(m)
            .and_then(|c| c.mayor)
            .and_then(|id| pol.get(id))
            .and_then(|p| p.party);
        let du = match (
            e.history.last(),
            e.history.get(e.history.len().saturating_sub(13)),
        ) {
            (Some(now), Some(then)) => (now.unemployment - then.unemployment) as f64,
            _ => 0.0,
        };
        let rent_growth = match (
            e.history.last(),
            e.history.get(e.history.len().saturating_sub(13)),
        ) {
            (Some(now), Some(then)) if then.rent > 0.0 => {
                (now.rent / then.rent) as f64 - 1.0 - econ.national.inflation
            }
            _ => 0.0,
        };
        let mods = crate::policy::mods_for(policy, r.content, m as u32, &|id| fin.funded_share(id));
        let services: f64 = mods.services.iter().sum();
        let disaster = if e.months_since_disaster < 6 {
            0.3
        } else {
            0.0
        };
        let experienced = -8.0 * (du - dn) - 2.0 * rent_growth + 2.0 * services
            - disaster
            - 1.5 * mods.disruption;
        // Reactions to laws and bills, pre-summed into one weighted signature
        // per town (linear in attitudes, so each cohort needs one dot product).
        let mut gov_vec = [0.0f64; ATTITUDES];
        let mut player_vec = [0.0f64; ATTITUDES];
        let mut n_gov = 0usize;
        let mut grievance = [[0.0f64; CLASSES]; AGE_BANDS];
        let mut player_grievance = [[0.0f64; CLASSES]; AGE_BANDS];
        for p in policy.in_municipality(m as u32).filter(|p| {
            matches!(
                p.status,
                PolicyStatus::UnderConsideration | PolicyStatus::Enacted | PolicyStatus::InEffect
            )
        }) {
            let Some(t) = r.content.policies.get(&p.template) else {
                continue;
            };
            let norm: f64 = t.signature.iter().map(|x| x.abs()).sum();
            if norm <= 0.0 {
                continue;
            }
            let w = 2.0 * p.awareness as f64 / norm;
            let mine = p
                .sponsor
                .and_then(|s| pol.get(s))
                .is_some_and(|s| s.is_player);
            for k in 0..ATTITUDES {
                if p.status != PolicyStatus::UnderConsideration {
                    gov_vec[k] += w * t.signature[k];
                }
                if mine {
                    player_vec[k] += w * t.signature[k];
                }
            }
            if p.status != PolicyStatus::UnderConsideration {
                n_gov += 1;
            }
            // Grievances (DEC-073): groups a law hurts materially, while it runs.
            if p.status == PolicyStatus::InEffect {
                let k = p.progress as f64 * p.pilot_scale() * fin.funded_share(p.id.0);
                for fx in &t.side_effects {
                    if let mandate_content::SideEffectDef::Grievance {
                        class,
                        min_age_band,
                        ..
                    } = fx
                    {
                        let a = crate::policy::side_effect_amount(fx, p) * k;
                        for age in (*min_age_band as usize).min(AGE_BANDS - 1)..AGE_BANDS {
                            for c in 0..CLASSES {
                                if class.is_none_or(|x| crate::policy::class_index(x) == c) {
                                    grievance[age][c] += a;
                                    if mine {
                                        player_grievance[age][c] += a;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Many laws should not saturate opinion: scale by √n.
        let gov_scale = 1.0 / mandate_core::math::sqrt(n_gov.max(1) as f64);
        for h in &mu.neighborhoods {
            for (ci, c) in pop.hoods[h.index()].cohorts.iter_mut().enumerate() {
                let a = ci / CLASSES;
                let mut gov_react = 0.0;
                let mut player_react = 0.0;
                for k in 0..ATTITUDES {
                    gov_react += gov_vec[k] * c.attitudes[k] as f64;
                    player_react += player_vec[k] * c.attitudes[k] as f64;
                }
                let class = ci % CLASSES;
                let gov_react = clamp(gov_react * gov_scale - grievance[a][class], -2.0, 2.0);
                let player_react = clamp(player_react - player_grievance[a][class], -2.0, 2.0);
                let target = 0.5 + 0.5 * tanh(experienced + 0.6 * gov_react);
                c.gov_approval = (c.gov_approval as f64 * 0.92 + target * 0.08) as f32;
                c.player_approval = clamp(
                    c.player_approval as f64 * 0.9 + 0.1 * tanh(player_react * 1.5),
                    -1.0,
                    1.0,
                ) as f32;
                c.player_awareness = (c.player_awareness as f64 * 0.985) as f32;
                if january {
                    update_party_id(c, a, parties, params, mayor_party);
                }
            }
        }
    }
}

/// Mover–stayer party attachment (DEC-071 §3): the mover share drifts toward
/// ideological fit plus credit or blame for the party running the town hall.
fn update_party_id(
    c: &mut Cohort,
    age_band: usize,
    parties: &[crate::politics::Party],
    params: &mandate_content::PoliticsParams,
    governing: Option<crate::politics::PartyId>,
) {
    if parties.is_empty() {
        return;
    }
    let p = &params.party_id;
    let attached = p.attached_by_age[age_band.min(3)];
    let mut target =
        crate::politics::parties::attachment(&c.attitudes, parties, attached, p.sharpness);
    if let Some(g) = governing
        && let Some(t) = target.get_mut(g.index())
    {
        *t = clamp(
            *t as f64 * (1.0 + (c.gov_approval as f64 - 0.5) * 0.6),
            0.0,
            1.0,
        ) as f32;
    }
    if c.party_id.len() != parties.len() {
        c.party_id = target;
        return;
    }
    let rate = p.mover_rate * (1.0 - p.stayer_share);
    for (v, t) in c.party_id.iter_mut().zip(&target) {
        *v = (*v as f64 + rate * (*t as f64 - *v as f64)) as f32;
    }
}

/// Initial party attachment for every cohort (after parties are generated).
pub fn assign_party_ids(
    pop: &mut PopulationState,
    parties: &[crate::politics::Party],
    params: &mandate_content::PoliticsParams,
) {
    for h in pop.hoods.iter_mut() {
        for (k, c) in h.cohorts.iter_mut().enumerate() {
            let band = k / CLASSES;
            c.party_id = crate::politics::parties::attachment(
                &c.attitudes,
                parties,
                params.party_id.attached_by_age[band.min(3)],
                params.party_id.sharpness,
            );
        }
    }
}
