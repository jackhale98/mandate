//! Population (DEC-018): aggregate cohorts per neighbourhood (age band ×
//! income class × community, DEC-075), simulated everywhere at all times.
//! Owns resident counts and cohort political state.

use crate::scheduler::{Effect, Outs, Reads};
use crate::world::{MunicipalityId, NeighborhoodKind, RegionId, SettlementClass, World};
use mandate_core::math::{clamp, tanh};
use mandate_core::{SimRng, Stream};
use serde::{Deserialize, Serialize};

pub const AGE_BANDS: usize = 4;
pub const CLASSES: usize = 3;
/// Age × class groups per community. A neighbourhood holds `GROUPS` cohorts
/// for each community in the registry (`PopulationState::community_keys`).
pub const GROUPS: usize = AGE_BANDS * CLASSES;

// Attitude dimensions are a content registry (DEC-075): each cohort holds
// one value in [-1, 1] per attitude, in registry order
// (`ContentBundle::attitudes`). `PopulationState::attitude_keys` records
// which attitudes the state was built with.

/// Age bands: 18–29, 30–44, 45–64, 65+.
pub const BAND_YEARS: [f64; AGE_BANDS] = [12.0, 15.0, 20.0, 15.0];
/// Labour-force participation by age band.
pub const PARTICIPATION: [f64; AGE_BANDS] = [0.72, 0.86, 0.78, 0.07];

// Cohort indexing (DEC-075). `HoodPop::cohorts` is community-major, then age
// band, then class: with one community (the base pack) the layout is the
// plain age × class grid. Code that needs a cohort's age, class, or community
// uses the helpers below, never arithmetic on the index, so adding a
// dimension cannot silently break it.

/// Index of an age × class group, for per-group aggregates that sum over
/// communities (`PopulationState::municipality_groups`).
#[inline]
pub fn group_index(age: usize, class: usize) -> usize {
    age * CLASSES + class
}

/// Index of a cohort in `HoodPop::cohorts`.
#[inline]
pub fn cohort_index(age: usize, class: usize, community: usize) -> usize {
    community * GROUPS + group_index(age, class)
}

/// The age × class group of a cohort index.
#[inline]
pub fn group_of(ci: usize) -> usize {
    ci % GROUPS
}

/// The age band of a cohort index.
#[inline]
pub fn age_of(ci: usize) -> usize {
    group_of(ci) / CLASSES
}

/// The income class of a cohort index.
#[inline]
pub fn class_of(ci: usize) -> usize {
    group_of(ci) % CLASSES
}

/// The community (registry index) of a cohort index.
#[inline]
pub fn community_of(ci: usize) -> usize {
    ci / GROUPS
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cohort {
    pub count: u32,
    /// One value per attitude, in registry order (`PopulationState::attitude_keys`).
    pub attitudes: Vec<f32>,
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
    /// All children in the neighbourhood.
    pub children: u32,
    /// Children by community, in `PopulationState::community_keys` order,
    /// summing to `children`. Empty while the state has one community (all
    /// children are in it), so the base game stores nothing extra.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children_by_community: Vec<u32>,
    /// Indexed by `cohort_index(age, class, community)`.
    pub cohorts: Vec<Cohort>,
}

impl HoodPop {
    pub fn adults(&self) -> u64 {
        self.cohorts.iter().map(|c| c.count as u64).sum()
    }
    pub fn total(&self) -> u64 {
        self.adults() + self.children as u64
    }
    /// Number of communities this neighbourhood's cohorts cover.
    pub fn communities(&self) -> usize {
        self.cohorts.len() / GROUPS
    }
    /// Adults of one community.
    pub fn community_adults(&self, k: usize) -> u64 {
        self.cohorts
            .iter()
            .skip(k * GROUPS)
            .take(GROUPS)
            .map(|c| c.count as u64)
            .sum()
    }
    /// Children of one community.
    pub fn children_of(&self, k: usize) -> u32 {
        if self.children_by_community.is_empty() {
            if k == 0 { self.children } else { 0 }
        } else {
            self.children_by_community.get(k).copied().unwrap_or(0)
        }
    }
    /// Set the children of one community (keeping `children` the total).
    pub fn set_children_of(&mut self, k: usize, n: u32) {
        if self.children_by_community.is_empty() {
            if k == 0 {
                self.children = n;
            }
        } else if let Some(v) = self.children_by_community.get_mut(k) {
            *v = n;
            self.children = self.children_by_community.iter().sum();
        }
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
    /// Attitude keys, in the order every attitude vector in the state uses
    /// (cohorts, politicians, parties, factions). Saves record it so content
    /// that adds or reorders attitudes can be remapped on load (DEC-075).
    pub attitude_keys: Vec<String>,
    /// Community ids, in the order cohorts are laid out per neighbourhood
    /// (`cohort_index`). Saves record it so content that adds or reorders
    /// communities can be remapped on load (DEC-075).
    pub community_keys: Vec<String>,
    pub hoods: Vec<HoodPop>,
    /// Internal moves accumulated this year.
    pub moves_ytd: u64,
    pub history: Vec<DemographyYear>,
    /// The last election whose turnout has been folded into habits.
    #[serde(default)]
    pub last_election_seen: Option<mandate_core::SimDate>,
}

impl PopulationState {
    /// Number of attitude dimensions in this state.
    pub fn attitude_count(&self) -> usize {
        self.attitude_keys.len()
    }
    /// Number of communities in this state.
    pub fn community_count(&self) -> usize {
        self.community_keys.len()
    }
    /// Adults per community in a municipality, in `community_keys` order.
    pub fn municipality_communities(&self, world: &World, m: MunicipalityId) -> Vec<u64> {
        let mut out = vec![0u64; self.community_count()];
        for i in world.hoods_of(m) {
            for (k, c) in self.hoods[i].cohorts.iter().enumerate() {
                if let Some(x) = out.get_mut(community_of(k)) {
                    *x += c.count as u64;
                }
            }
        }
        out
    }
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
    /// Adults per age × class group in a municipality, summed over
    /// communities; index with `group_index(age, class)`.
    pub fn municipality_groups(&self, world: &World, m: MunicipalityId) -> [f64; GROUPS] {
        let mut out = [0.0; GROUPS];
        for i in world.hoods_of(m) {
            for (k, c) in self.hoods[i].cohorts.iter().enumerate() {
                out[group_of(k)] += c.count as f64;
            }
        }
        out
    }
    /// Labour force (participation-weighted adults) by class.
    pub fn labour_force_by_class(&self, world: &World, m: MunicipalityId) -> [f64; CLASSES] {
        let c = self.municipality_groups(world, m);
        let mut lf = [0.0; CLASSES];
        for a in 0..AGE_BANDS {
            for (k, v) in lf.iter_mut().enumerate() {
                *v += c[group_index(a, k)] * PARTICIPATION[a];
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

/// Lean of a neighbourhood kind in an attitude's seeding model.
fn kind_lean(l: &mandate_content::NeighborhoodLean, kind: NeighborhoodKind) -> f64 {
    match kind {
        NeighborhoodKind::Core => l.core,
        NeighborhoodKind::Inner => l.inner,
        NeighborhoodKind::Waterfront => l.waterfront,
        NeighborhoodKind::Industrial => l.industrial,
        NeighborhoodKind::Suburb => l.suburb,
        NeighborhoodKind::Rural => l.rural,
    }
}

/// Starting attitudes for a cohort (conditioned on context, not determined).
/// The leans come from content (`AttitudeDef::seed`), one draw per attitude
/// in registry order, plus the cohort's community offsets (dense in attitude
/// order; empty = none).
fn seed_attitudes(
    rng: &mut SimRng,
    defs: &[mandate_content::AttitudeDef],
    age: usize,
    class: usize,
    kind: NeighborhoodKind,
    regional: &[f64],
    offsets: &[f64],
) -> Vec<f32> {
    defs.iter()
        .zip(regional)
        .enumerate()
        .map(|(i, (d, reg))| {
            let base = seed_mean(d, age, class, kind) + offsets.get(i).copied().unwrap_or(0.0);
            clamp(base + reg + rng.normal(0.0, d.seed.noise_sd), -1.0, 1.0) as f32
        })
        .collect()
}

/// An attitude's seeding-model mean for a class, age, and neighbourhood kind.
fn seed_mean(
    d: &mandate_content::AttitudeDef,
    age: usize,
    class: usize,
    kind: NeighborhoodKind,
) -> f64 {
    let s = &d.seed;
    s.by_class[class] + s.by_age[age] + kind_lean(&s.by_neighborhood, kind)
}

/// Seeded turnout propensity for an age band and class (before noise).
fn seed_turnout(age: usize, class: usize, offset: f64) -> f64 {
    [0.42, 0.55, 0.66, 0.74][age] + [-0.08, 0.0, 0.1][class] + offset
}

/// Each community's share of a neighbourhood (DEC-075), in registry order.
/// Communities with a share model take `base + by_neighborhood[kind] +
/// regional offset`, clamped to [0, 1] and scaled down together if they add
/// up to more than 1; the remainder community's entry is unused (it takes
/// whatever is left).
fn community_shares(
    communities: &[mandate_content::CommunityDef],
    kind: NeighborhoodKind,
    regional: &[f64],
) -> Vec<f64> {
    let mut shares: Vec<f64> = communities
        .iter()
        .zip(regional)
        .map(|(c, reg)| match &c.share {
            Some(sh) => clamp(
                sh.base + kind_lean(&sh.by_neighborhood, kind) + reg,
                0.0,
                1.0,
            ),
            None => 0.0,
        })
        .collect();
    let sum: f64 = shares.iter().sum();
    if sum > 1.0 {
        for x in shares.iter_mut() {
            *x /= sum;
        }
    }
    shares
}

/// Split `n` people across communities by share, exactly: each community with
/// a share model takes its rounded share of what is left in registry order,
/// and the remainder community takes the rest.
fn split_by_share(n: u32, shares: &[f64], remainder: usize) -> Vec<u32> {
    let mut out = vec![0u32; shares.len().max(1)];
    let mut left = n;
    for (k, s) in shares.iter().enumerate() {
        if k == remainder || *s <= 0.0 {
            continue;
        }
        let x = ((n as f64 * s).round() as u32).min(left);
        out[k] = x;
        left -= x;
    }
    if let Some(r) = out.get_mut(remainder) {
        *r += left;
    }
    out
}

/// Build the starting population from worldgen's per-neighbourhood totals.
/// With one community (the base pack) this makes exactly the draws and
/// cohorts of the plain age × class model.
pub fn init(
    world: &World,
    totals: &[u64],
    world_seed: u64,
    wealthy: bool,
    attitudes: &[mandate_content::AttitudeDef],
    communities: &[mandate_content::CommunityDef],
) -> PopulationState {
    let region_culture: Vec<Vec<f64>> = (0..world.regions.len())
        .map(|r| {
            let mut rng =
                mandate_core::rng::keyed(world_seed, Stream::Population, &[0xC0170, r as u64]);
            attitudes
                .iter()
                .map(|a| rng.normal(0.0, a.seed.regional_sd))
                .collect()
        })
        .collect();
    let ncom = communities.len().max(1);
    let remainder = communities
        .iter()
        .position(|c| c.share.is_none())
        .unwrap_or(0);
    // Regional share offsets: one draw per region and community with a share
    // model, in a stream keyed by the community id (so registry order does not
    // matter). The remainder community draws nothing, so the base pack adds
    // no draws.
    let region_share: Vec<Vec<f64>> = (0..world.regions.len())
        .map(|r| {
            communities
                .iter()
                .map(|c| match &c.share {
                    Some(sh) if sh.regional_sd > 0.0 => {
                        let key = mandate_core::hash::derive_u64(&c.id, &[]);
                        let mut rng = mandate_core::rng::keyed(
                            world_seed,
                            Stream::Population,
                            &[0xC0C0, key, r as u64],
                        );
                        rng.normal(0.0, sh.regional_sd)
                    }
                    _ => 0.0,
                })
                .collect()
        })
        .collect();
    let no_offsets: Vec<f64> = Vec::new();
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
        // Adults per age × class group, then split across communities.
        let mut groups = [0u32; GROUPS];
        for (a, share_a) in ages.iter().enumerate() {
            for (c, share_c) in classes.iter().enumerate() {
                groups[group_index(a, c)] =
                    (adults * share_a * share_c / csum).round().max(0.0) as u32;
            }
        }
        let split: Vec<Vec<u32>> = if ncom > 1 {
            let shares = community_shares(communities, n.kind, &region_share[region]);
            groups
                .iter()
                .map(|g| split_by_share(*g, &shares, remainder))
                .collect()
        } else {
            groups.iter().map(|g| vec![*g]).collect()
        };
        let mut cohorts = Vec::with_capacity(GROUPS * ncom);
        for k in 0..ncom {
            let cm = communities.get(k);
            let offsets = cm
                .map(|c| &c.attitude_offset_weights)
                .unwrap_or(&no_offsets);
            let turnout_offset = cm.map(|c| c.turnout_offset).unwrap_or(0.0);
            for a in 0..AGE_BANDS {
                for c in 0..CLASSES {
                    let count = split[group_index(a, c)][k];
                    let turnout = seed_turnout(a, c, turnout_offset);
                    cohorts.push(Cohort {
                        count,
                        attitudes: seed_attitudes(
                            &mut rng,
                            attitudes,
                            a,
                            c,
                            n.kind,
                            &region_culture[region],
                            offsets,
                        ),
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
        }
        let adults_total: u64 = cohorts.iter().map(|c| c.count as u64).sum();
        let children = (total as u64).saturating_sub(adults_total) as u32;
        let mut hood = HoodPop {
            children,
            children_by_community: Vec::new(),
            cohorts,
        };
        if ncom > 1 {
            // Children follow their community's share of the adults.
            let shares: Vec<f64> = (0..ncom)
                .map(|k| hood.community_adults(k) as f64 / adults_total.max(1) as f64)
                .collect();
            hood.children_by_community = split_by_share(children, &shares, remainder);
        }
        hoods.push(hood);
    }
    PopulationState {
        attitude_keys: attitudes.iter().map(|a| a.key.clone()).collect(),
        community_keys: communities.iter().map(|c| c.id.clone()).collect(),
        hoods,
        moves_ytd: 0,
        history: Vec::new(),
        last_election_seen: None,
    }
}

/// Re-key every attitude vector in the state to the content's attitude
/// registry (DEC-075), for a save loaded into content that added or reordered
/// attitudes. Values are kept under their key. A new attitude starts without
/// randomness: cohorts at their seeding model's mean for their class, age, and
/// neighbourhood kind (no regional offset or noise); politicians at the
/// turnout-weighted view of their home neighbourhood; party platforms and
/// factions neutral (0). Content that removed an attitude the state uses is
/// refused before this runs (`engine::missing_references`). Returns whether
/// anything changed.
pub fn remap_attitudes(
    pop: &mut PopulationState,
    politics: &mut crate::politics::PoliticsState,
    world: &World,
    content: &mandate_content::ContentBundle,
) -> bool {
    let keys: Vec<String> = content.attitude_keys().cloned().collect();
    if pop.attitude_keys == keys {
        return false;
    }
    let old: Vec<Option<usize>> = keys
        .iter()
        .map(|k| pop.attitude_keys.iter().position(|o| o == k))
        .collect();
    let remap = |v: &[f32], fresh: &dyn Fn(usize) -> f32| -> Vec<f32> {
        old.iter()
            .enumerate()
            .map(|(i, o)| match o.and_then(|j| v.get(j)) {
                Some(x) => *x,
                None => fresh(i),
            })
            .collect()
    };
    // A cohort's community offsets, from the content's community of the same
    // id (the community registry is remapped after this).
    let offsets: Vec<Option<&Vec<f64>>> = pop
        .community_keys
        .iter()
        .map(|k| {
            content
                .community_index(k)
                .map(|i| &content.communities[i].attitude_offset_weights)
        })
        .collect();
    for (h, hp) in pop.hoods.iter_mut().enumerate() {
        let Some(kind) = world.neighborhoods.get(h).map(|n| n.kind) else {
            continue;
        };
        for (ci, co) in hp.cohorts.iter_mut().enumerate() {
            let (a, c) = (age_of(ci), class_of(ci));
            let off = offsets.get(community_of(ci)).copied().flatten();
            co.attitudes = remap(&co.attitudes, &|i| {
                let o = off.and_then(|v| v.get(i)).copied().unwrap_or(0.0);
                clamp(seed_mean(&content.attitudes[i], a, c, kind) + o, -1.0, 1.0) as f32
            });
        }
    }
    pop.attitude_keys = keys;
    for p in politics.politicians.iter_mut() {
        let home = p.home as usize;
        let local = if home < pop.hoods.len() {
            crate::politics::mean_attitudes(pop, &[home])
        } else {
            vec![0.0; pop.attitude_count()]
        };
        p.attitudes = remap(&p.attitudes, &|i| local[i]);
    }
    for party in politics.parties.iter_mut() {
        party.platform = remap(&party.platform, &|_| 0.0);
        for f in party.factions.iter_mut() {
            f.position = remap(&f.position, &|_| 0.0);
        }
    }
    true
}

/// Re-key cohorts to the content's community registry (DEC-075), for a save
/// loaded into content that added or reordered communities. Each community
/// keeps its cohorts and children under its id. A new community starts with
/// no people: its cohorts are empty, seeded without randomness (attitudes at
/// the seeding model's mean plus the community's offsets, turnout at the
/// seeded propensity, party attachment from those attitudes), and it grows
/// only through immigration (`immigration_share`) and the people born into
/// it from then on. Content that removed a community the state uses is
/// refused before this runs (`engine::missing_references`). Run after
/// `remap_attitudes`. Returns whether anything changed.
pub fn remap_communities(
    pop: &mut PopulationState,
    politics: &crate::politics::PoliticsState,
    world: &World,
    content: &mandate_content::ContentBundle,
) -> bool {
    let keys: Vec<String> = content.community_keys().cloned().collect();
    if pop.community_keys == keys || keys.is_empty() {
        return false;
    }
    let old: Vec<Option<usize>> = keys
        .iter()
        .map(|k| pop.community_keys.iter().position(|o| o == k))
        .collect();
    let params = &content.politics.party_id;
    for (h, hp) in pop.hoods.iter_mut().enumerate() {
        let Some(kind) = world.neighborhoods.get(h).map(|n| n.kind) else {
            continue;
        };
        let mut cohorts = Vec::with_capacity(GROUPS * keys.len());
        let mut children = Vec::with_capacity(keys.len());
        for (k, o) in old.iter().enumerate() {
            match o {
                Some(j) if hp.cohorts.len() >= (j + 1) * GROUPS => {
                    cohorts.extend_from_slice(&hp.cohorts[j * GROUPS..(j + 1) * GROUPS]);
                    children.push(hp.children_of(*j));
                }
                _ => {
                    let cm = &content.communities[k];
                    for a in 0..AGE_BANDS {
                        for c in 0..CLASSES {
                            let attitudes: Vec<f32> = content
                                .attitudes
                                .iter()
                                .enumerate()
                                .map(|(i, d)| {
                                    let o =
                                        cm.attitude_offset_weights.get(i).copied().unwrap_or(0.0);
                                    clamp(seed_mean(d, a, c, kind) + o, -1.0, 1.0) as f32
                                })
                                .collect();
                            let turnout = clamp(seed_turnout(a, c, cm.turnout_offset), 0.05, 0.98);
                            let party_id = crate::politics::parties::attachment(
                                &attitudes,
                                &politics.parties,
                                params.attached_by_age[a.min(3)],
                                params.sharpness,
                            );
                            cohorts.push(Cohort {
                                count: 0,
                                attitudes,
                                trust: clamp(0.55 + [-0.08, 0.0, 0.06][c], 0.0, 1.0) as f32,
                                turnout: turnout as f32,
                                gov_approval: 0.5,
                                player_awareness: 0.0,
                                player_approval: 0.0,
                                party_id,
                                last_turnout: turnout as f32,
                            });
                        }
                    }
                    children.push(0);
                }
            }
        }
        hp.cohorts = cohorts;
        hp.children = children.iter().sum();
        hp.children_by_community = if keys.len() > 1 { children } else { Vec::new() };
    }
    pop.community_keys = keys;
    true
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
    for (d, s) in dst.attitudes.iter_mut().zip(&src.attitudes) {
        *d = (*d as f64 * (1.0 - w_new) + *s as f64 * w_new) as f32;
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
    // Immigration shares by state community (the state's community order is
    // the content's: `Engine::from_state` remaps it).
    let remainder = r.content.remainder_community();
    let imm_shares: Vec<f64> = {
        let v: Vec<f64> = r
            .content
            .communities
            .iter()
            .map(|c| c.immigration_share)
            .collect();
        let sum: f64 = v.iter().sum();
        if sum > 1.0 {
            v.iter().map(|x| x / sum).collect()
        } else {
            v
        }
    };
    for (i, n) in r.world.neighborhoods.iter().enumerate() {
        let m = n.municipality.index();
        let unemployment = econ.munis.get(m).map(|e| e.unemployment).unwrap_or(0.05);
        let knowledge = econ
            .munis
            .get(m)
            .map(|e| e.knowledge_share(r.content))
            .unwrap_or(0.1);
        let class = r.world.municipalities[m].class;
        let h = &mut pop.hoods[i];
        let ncom = h.communities();

        // Births to adults aged 18–44, into their parents' community.
        for k in 0..ncom {
            let parents: u64 = (0..CLASSES)
                .map(|c| {
                    (h.cohorts[cohort_index(0, c, k)].count
                        + h.cohorts[cohort_index(1, c, k)].count) as u64
                })
                .sum();
            let births = draw(rng, parents as f64 * p.birth_rate);
            // Children come of age into the youngest band, by parents' class mix.
            let kids = h.children_of(k);
            let coming_of_age = draw(rng, kids as f64 / 18.0);
            let coming_of_age = coming_of_age.min(kids);
            let child_deaths = draw(rng, kids as f64 * p.death_rates[0]).min(kids - coming_of_age);
            h.set_children_of(k, kids - coming_of_age - child_deaths + births);
            // Split exactly (largest share takes the remainder) so people are conserved.
            let parent_mix: Vec<u64> = (0..CLASSES)
                .map(|c| h.cohorts[cohort_index(1, c, k)].count as u64)
                .collect();
            let pm_sum: u64 = parent_mix.iter().sum::<u64>().max(1);
            let mut given = 0u32;
            for c in 0..CLASSES {
                let n = (coming_of_age as u64 * parent_mix[c] / pm_sum) as u32;
                h.cohorts[cohort_index(0, c, k)].count += n;
                given += n;
            }
            let biggest = (0..CLASSES)
                .max_by_key(|&c| (parent_mix[c], c))
                .unwrap_or(1);
            h.cohorts[cohort_index(0, biggest, k)].count += coming_of_age - given;
            year.births += births as u64;
            year.deaths += child_deaths as u64;
        }

        for k in 0..ncom {
            // Ageing (oldest band first so people move one band per year).
            for a in (0..AGE_BANDS - 1).rev() {
                for c in 0..CLASSES {
                    let from = cohort_index(a, c, k);
                    let n_age = draw(rng, h.cohorts[from].count as f64 / BAND_YEARS[a]);
                    shift(h, from, cohort_index(a + 1, c, k), n_age);
                }
            }
            // Deaths.
            for a in 0..AGE_BANDS {
                for c in 0..CLASSES {
                    let ci = cohort_index(a, c, k);
                    let d = draw(
                        rng,
                        h.cohorts[ci].count as f64 * p.death_rates[a + 1] * [1.15, 1.0, 0.85][c],
                    )
                    .min(h.cohorts[ci].count);
                    h.cohorts[ci].count -= d;
                    year.deaths += d as u64;
                }
            }
            // Class mobility among working ages.
            for a in 0..3 {
                let up_low = 0.02 * (1.0 - unemployment * 4.0).max(0.0) + 0.01 * knowledge;
                let down_mid = 0.008 + unemployment * 0.08;
                let up_mid = 0.006 + 0.03 * knowledge;
                let down_high = 0.006 + unemployment * 0.03;
                let (lo, mi, hi) = (
                    cohort_index(a, 0, k),
                    cohort_index(a, 1, k),
                    cohort_index(a, 2, k),
                );
                let n1 = draw(rng, h.cohorts[lo].count as f64 * up_low);
                let n2 = draw(rng, h.cohorts[mi].count as f64 * down_mid);
                let n3 = draw(rng, h.cohorts[mi].count as f64 * up_mid);
                let n4 = draw(rng, h.cohorts[hi].count as f64 * down_high);
                shift(h, lo, mi, n1);
                shift(h, mi, lo, n2);
                shift(h, mi, hi, n3);
                shift(h, hi, mi, n4);
            }
        }
        // International immigration to larger places.
        let imm_rate = match class {
            SettlementClass::Metro => 0.006,
            SettlementClass::City => 0.0035,
            SettlementClass::Town => 0.0012,
            SettlementClass::Rural => 0.0004,
        };
        let imm = draw(rng, h.adults() as f64 * imm_rate);
        // Communities with an immigration share take theirs; the remainder
        // community takes the rest (everyone, in the base pack).
        let per_community = if ncom > 1 && imm_shares.len() == ncom {
            split_by_share(imm, &imm_shares, remainder)
        } else {
            vec![imm]
        };
        for (k, n) in per_community.iter().enumerate().take(ncom) {
            h.cohorts[cohort_index(0, 0, k)].count += n / 2;
            h.cohorts[cohort_index(0, 1, k)].count += n - n / 2;
        }
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
    // Movers keep their community: cohort indices carry it.
    let ncoh = pop.hoods.first().map(|h| h.cohorts.len()).unwrap_or(GROUPS);
    let mobile: Vec<usize> = (0..ncoh / GROUPS)
        .flat_map(|k| (0..2).flat_map(move |a| (0..CLASSES).map(move |c| cohort_index(a, c, k))))
        .collect();
    let mut movers = vec![vec![0u64; ncoh]; nm];
    let na = pop.attitude_count();
    let mut mover_att = vec![vec![vec![0.0f64; na]; ncoh]; nm];
    let np = pop
        .hoods
        .first()
        .and_then(|h| h.cohorts.first())
        .map(|c| c.party_id.len())
        .unwrap_or(0);
    let mut mover_pid = vec![vec![vec![0.0f64; np]; ncoh]; nm];
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
    let mut arr_n = vec![vec![0u64; ncoh]; nm];
    let mut arr_att = vec![vec![vec![0.0f64; na]; ncoh]; nm];
    let mut arr_pid = vec![vec![vec![0.0f64; np]; ncoh]; nm];
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
                for j in 0..na {
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
    // Name recognition fades faster for an invisible officeholder (DEC-076).
    let player_record = r
        .player
        .and_then(|p| pol.get(p.politician))
        .map(|p| p.record as f64)
        .unwrap_or(0.5);
    let career = &r.content.career;
    let awareness_keep = if player_record < career.invisible_record {
        career.awareness_keep_invisible
    } else {
        career.awareness_keep
    };
    let op = &r.content.opinion;
    let habit = &params.turnout;
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
                    c.last_turnout = clamp(
                        *t as f64 * (habit.habit_base + habit.habit_propensity * c.turnout as f64),
                        0.02,
                        0.95,
                    ) as f32;
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
        let x = &op.experience;
        let disaster = if e.months_since_disaster < x.disaster_months {
            x.disaster
        } else {
            0.0
        };
        let experienced = -x.unemployment * (du - dn) - x.rent_growth * rent_growth
            + x.services * services
            - disaster
            - x.disruption * mods.disruption;
        // Reactions to laws and bills, pre-summed into one weighted signature
        // per town (linear in attitudes, so each cohort needs one dot product).
        let na = pop.attitude_count();
        let mut gov_vec = vec![0.0f64; na];
        let mut player_vec = vec![0.0f64; na];
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
            let sig = &t.signature_weights;
            let norm: f64 = sig.iter().map(|x| x.abs()).sum();
            if norm <= 0.0 {
                continue;
            }
            let w = op.policy_reaction * p.awareness as f64 / norm;
            let mine = p
                .sponsor
                .and_then(|s| pol.get(s))
                .is_some_and(|s| s.is_player);
            for k in 0..na.min(sig.len()) {
                if p.status != PolicyStatus::UnderConsideration {
                    gov_vec[k] += w * sig[k];
                }
                if mine {
                    player_vec[k] += w * sig[k];
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
                let a = age_of(ci);
                let mut gov_react = 0.0;
                let mut player_react = 0.0;
                for (k, a) in c.attitudes.iter().enumerate().take(na) {
                    gov_react += gov_vec[k] * *a as f64;
                    player_react += player_vec[k] * *a as f64;
                }
                let class = class_of(ci);
                let gov_react = clamp(gov_react * gov_scale - grievance[a][class], -2.0, 2.0);
                let player_react = clamp(player_react - player_grievance[a][class], -2.0, 2.0);
                let target = 0.5 + 0.5 * tanh(experienced + op.law_reaction_weight * gov_react);
                c.gov_approval = (c.gov_approval as f64 * (1.0 - op.gov_approval_rate)
                    + target * op.gov_approval_rate) as f32;
                c.player_approval = clamp(
                    c.player_approval as f64 * (1.0 - op.player_approval_rate)
                        + op.player_approval_rate * tanh(player_react * op.player_reaction_scale),
                    -1.0,
                    1.0,
                ) as f32;
                c.player_awareness = (c.player_awareness as f64 * awareness_keep) as f32;
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
            *t as f64 * (1.0 + (c.gov_approval as f64 - 0.5) * p.governing_credit),
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
            let band = age_of(k);
            c.party_id = crate::politics::parties::attachment(
                &c.attitudes,
                parties,
                params.party_id.attached_by_age[band.min(3)],
                params.party_id.sharpness,
            );
        }
    }
}
