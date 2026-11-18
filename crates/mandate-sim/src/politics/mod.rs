//! Politicians, parties, and local bodies (DEC-027, DEC-031 subset): council
//! members and mayors generated from their wards' opinions, each belonging to
//! a party generated from the society's cleavages. Elections live in
//! `elections`; full character simulation arrives in M5.

pub mod elections;
pub mod parties;

pub use parties::{Party, PartyId};

use crate::population::{ATTITUDES, PopulationState};
use crate::world::{ConstituencyId, MunicipalityId, Tier, World};
use mandate_core::math::clamp;
use mandate_core::{IdAllocator, SimRng, define_id};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

define_id!(
    /// A politician (NPC or the player).
    PoliticianId
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Councillor {
        municipality: u32,
        ward: ConstituencyId,
    },
    Mayor {
        municipality: u32,
    },
    /// Not in office (a defeated or retired politician, or a challenger).
    Private {
        municipality: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Politician {
    pub id: PoliticianId,
    pub given: String,
    pub family: String,
    pub role: Role,
    pub is_player: bool,
    pub birth_year: u32,
    pub attitudes: [f32; ATTITUDES],
    /// Willingness to deal and be persuaded (0..1).
    pub pragmatism: f32,
    pub integrity: f32,
    pub oratory: f32,
    pub negotiation: f32,
    /// Opinion of the player, −100..100.
    pub opinion_of_player: f32,
    /// Favours: positive = they owe the player; negative = the player owes them.
    pub favors: i32,
    pub party: Option<PartyId>,
    /// Drive to seek office (0..1).
    pub ambition: f32,
    /// Home neighbourhood (local ties in elections).
    pub home: u32,
    /// Valence damage from scandals (0..1), decays over time.
    pub scandal: f32,
    /// Campaign war chest (currency units).
    pub funds: f64,
    /// Years spent in elected office.
    pub years_in_office: u8,
    /// Terms served in the current office (term limits).
    pub terms_in_office: u8,
}

impl Politician {
    pub fn in_office(&self) -> bool {
        !matches!(self.role, Role::Private { .. })
    }

    pub fn full_name(&self) -> String {
        format!("{} {}", self.given, self.family)
    }

    pub fn municipality(&self) -> u32 {
        match self.role {
            Role::Councillor { municipality, .. }
            | Role::Mayor { municipality }
            | Role::Private { municipality } => municipality,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Council {
    pub municipality: u32,
    /// One member per ward, in ward order.
    pub members: Vec<PoliticianId>,
    pub mayor: Option<PoliticianId>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoliticsState {
    pub ids: IdAllocator,
    pub politicians: Vec<Politician>,
    /// Indexed by municipality.
    pub councils: Vec<Council>,
    /// Per (politician, policy) persuasion built up by lobbying, trades, and earmarks.
    #[serde(with = "mandate_core::serde_util::map_as_pairs")]
    pub sway: BTreeMap<(u32, u32), f32>,
    pub parties: Vec<Party>,
    pub elections: elections::ElectionState,
}

impl PoliticsState {
    pub fn get(&self, id: PoliticianId) -> Option<&Politician> {
        self.politicians.get(id.index())
    }
    pub fn get_mut(&mut self, id: PoliticianId) -> Option<&mut Politician> {
        self.politicians.get_mut(id.index())
    }
    pub fn party(&self, id: PartyId) -> Option<&Party> {
        self.parties.get(id.index())
    }
    pub fn sway(&self, p: PoliticianId, policy: u32) -> f32 {
        self.sway.get(&(p.0, policy)).copied().unwrap_or(0.0)
    }
}

/// Turnout-weighted mean attitudes of the adults in a set of neighbourhoods.
pub fn mean_attitudes(pop: &PopulationState, hoods: &[usize]) -> [f32; ATTITUDES] {
    let mut acc = [0.0f64; ATTITUDES];
    let mut w = 0.0f64;
    for &h in hoods {
        for c in &pop.hoods[h].cohorts {
            let weight = c.count as f64 * c.turnout as f64;
            for k in 0..ATTITUDES {
                acc[k] += c.attitudes[k] as f64 * weight;
            }
            w += weight;
        }
    }
    let mut out = [0.0f32; ATTITUDES];
    for k in 0..ATTITUDES {
        out[k] = (acc[k] / w.max(1.0)) as f32;
    }
    out
}

fn name(rng: &mut SimRng, given: &[String], family: &[String]) -> (String, String) {
    let g = rng.pick(given).cloned().unwrap_or_else(|| "Alex".into());
    let f1 = rng.pick(family).cloned().unwrap_or_else(|| "Smith".into());
    let f2 = if rng.chance(0.5) {
        String::new()
    } else {
        ["wood", "field", "ley", "son", "er", "well", "by"][rng.below(7) as usize].to_string()
    };
    (g, f1 + &f2)
}

/// Councils and mayors for every municipality, reflecting their voters (with noise).
pub fn init(
    world: &World,
    pop: &PopulationState,
    rng: &mut SimRng,
    given: &[String],
    family: &[String],
    year: u32,
    parties: Vec<Party>,
) -> PoliticsState {
    let mut st = PoliticsState {
        parties,
        ..Default::default()
    };
    let make = |st: &mut PoliticsState,
                rng: &mut SimRng,
                role: Role,
                base: [f32; ATTITUDES],
                home: u32|
     -> PoliticianId {
        let mut attitudes = base;
        for a in attitudes.iter_mut() {
            *a = clamp(*a as f64 + rng.normal(0.0, 0.18), -1.0, 1.0) as f32;
        }
        let party = parties::nearest_party(&attitudes, &st.parties);
        new_politician(st, rng, given, family, role, attitudes, party, home, year)
    };
    let _ = &make;
    for m in &world.municipalities {
        let wards: Vec<_> = world
            .constituencies
            .iter()
            .filter(|c| c.tier == Tier::Local && c.jurisdiction == m.id.0)
            .collect();
        let mut members = Vec::new();
        for w in &wards {
            let hoods: Vec<usize> = w.neighborhoods.iter().map(|h| h.index()).collect();
            let base = mean_attitudes(pop, &hoods);
            let home = w.neighborhoods.first().map(|h| h.0).unwrap_or(0);
            let id = make(
                &mut st,
                rng,
                Role::Councillor {
                    municipality: m.id.0,
                    ward: w.id,
                },
                base,
                home,
            );
            members.push(id);
        }
        let all: Vec<usize> = world.hoods_of(MunicipalityId(m.id.0)).collect();
        let home = m.neighborhoods.first().map(|h| h.0).unwrap_or(0);
        let mayor = make(
            &mut st,
            rng,
            Role::Mayor {
                municipality: m.id.0,
            },
            mean_attitudes(pop, &all),
            home,
        );
        st.councils.push(Council {
            municipality: m.id.0,
            members,
            mayor: Some(mayor),
        });
    }
    // Sitting officials have served some years already.
    for p in st.politicians.iter_mut() {
        p.years_in_office = rng.range_i64(0, 12) as u8;
        p.terms_in_office = 1 + p.years_in_office / 4;
    }
    st
}

/// Create a politician (NPC) and register them. Used at generation and for
/// new challengers at each election.
#[allow(clippy::too_many_arguments)]
pub fn new_politician(
    st: &mut PoliticsState,
    rng: &mut SimRng,
    given: &[String],
    family: &[String],
    role: Role,
    attitudes: [f32; ATTITUDES],
    party: Option<PartyId>,
    home: u32,
    year: u32,
) -> PoliticianId {
    {
        let id: PoliticianId = st.ids.alloc();
        let (g, f) = name(rng, given, family);
        st.politicians.push(Politician {
            id,
            given: g,
            family: f,
            role,
            is_player: false,
            birth_year: year.saturating_sub(rng.range_i64(30, 68) as u32),
            attitudes,
            pragmatism: rng.range_f64(0.2, 0.9) as f32,
            integrity: rng.range_f64(0.2, 0.95) as f32,
            oratory: rng.range_f64(0.2, 0.9) as f32,
            negotiation: rng.range_f64(0.2, 0.9) as f32,
            opinion_of_player: rng.normal(0.0, 12.0).clamp(-40.0, 40.0) as f32,
            favors: 0,
            party,
            ambition: rng.range_f64(0.2, 0.95) as f32,
            home,
            scandal: 0.0,
            funds: rng.range_f64(2_000.0, 12_000.0),
            years_in_office: 0,
            terms_in_office: 0,
        });
        id
    }
}

/// Cosine-style alignment between a policy signature and an attitude vector, in [-1, 1].
pub fn alignment(signature: &[f64; ATTITUDES], attitudes: &[f32; ATTITUDES]) -> f64 {
    let mut dot = 0.0;
    let mut norm = 0.0;
    for k in 0..ATTITUDES {
        dot += signature[k] * attitudes[k] as f64;
        norm += signature[k].abs();
    }
    if norm <= 0.0 {
        0.0
    } else {
        clamp(dot / norm * 2.0, -1.0, 1.0)
    }
}
