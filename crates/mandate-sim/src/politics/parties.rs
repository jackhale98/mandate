//! Parties and factions (DEC-027): generated from the society's cleavages.
//!
//! Parties are found by weighted k-means over neighbourhood cohorts' attitudes,
//! so they reflect the coalitions that actually exist in this world. Each
//! party is a coalition of two factions (its wings), which set the platform.

use crate::population::PopulationState;
use mandate_content::PoliticsParams;
use mandate_core::{SimRng, define_id};
use serde::{Deserialize, Serialize};

define_id!(
    /// A political party.
    PartyId
);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Faction {
    /// Fluent key suffix for the wing, e.g. `left` → `faction-left`.
    pub wing: String,
    /// One value per attitude, in registry order.
    pub position: Vec<f32>,
    /// Share of the party's weight (0..1); shifts with members' success.
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Party {
    pub id: PartyId,
    pub name: String,
    pub short: String,
    /// The platform: factions blended by strength (one value per attitude,
    /// in registry order).
    pub platform: Vec<f32>,
    pub factions: Vec<Faction>,
    /// Stable colour slot for the UI (never re-assigned).
    pub colour: u8,
}

impl Party {
    pub fn recompute_platform(&mut self) {
        let total: f32 = self
            .factions
            .iter()
            .map(|f| f.strength)
            .sum::<f32>()
            .max(1e-6);
        let mut p = vec![0.0f32; self.platform.len()];
        for f in &self.factions {
            for (k, x) in p.iter_mut().enumerate() {
                *x += f.position.get(k).copied().unwrap_or(0.0) * f.strength / total;
            }
        }
        self.platform = p;
    }
}

/// A weighted attitude sample (one neighbourhood cohort).
struct Sample {
    x: Vec<f64>,
    w: f64,
}

fn to_f32(x: &[f64]) -> Vec<f32> {
    x.iter().map(|v| *v as f32).collect()
}

/// Generate parties from the electorate. Largest first.
pub fn generate(pop: &PopulationState, params: &PoliticsParams, rng: &mut SimRng) -> Vec<Party> {
    let na = pop.attitude_count();
    let mut samples = Vec::new();
    let mut mean = vec![0.0f64; na];
    let mut wsum = 0.0;
    for h in &pop.hoods {
        for c in &h.cohorts {
            if c.count == 0 {
                continue;
            }
            let mut x = vec![0.0f64; na];
            for k in 0..na {
                x[k] = c.attitudes[k] as f64;
                mean[k] += x[k] * c.count as f64;
            }
            wsum += c.count as f64;
            samples.push(Sample {
                x,
                w: c.count as f64 * c.turnout.max(0.05) as f64,
            });
        }
    }
    for m in mean.iter_mut() {
        *m /= wsum.max(1.0);
    }
    let k = (params.party_count as usize).clamp(2, 6);
    if samples.len() < k {
        return Vec::new();
    }
    let clusters = major_minor_split(&samples, &mean, k, rng);
    let mut used_names: Vec<String> = Vec::new();
    let mut parties = Vec::new();
    for (i, (centre, _share)) in clusters.iter().enumerate() {
        // Parties sharpen their members' views: platforms sit beyond the cluster mean.
        let mut platform = vec![0.0f64; na];
        for a in 0..na {
            platform[a] = (mean[a] + (centre[a] - mean[a]) * 1.6).clamp(-1.0, 1.0);
        }
        // Two wings: displaced along the party's most distinctive dimension pair.
        let (d1, _) = distinctive(&platform, &mean);
        let mut left = platform.clone();
        let mut right = platform.clone();
        for a in 0..na {
            let spread = 0.12 * if a == d1 { 2.0 } else { 0.5 };
            left[a] = (platform[a] + spread * (platform[a] - mean[a]).signum()).clamp(-1.0, 1.0);
            right[a] = (platform[a] - spread * (platform[a] - mean[a]).signum()).clamp(-1.0, 1.0);
        }
        let name = party_name(
            &platform,
            &mean,
            &pop.attitude_keys,
            params,
            rng,
            &used_names,
        );
        used_names.push(name.clone());
        let short = name
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .filter(|c| c.is_uppercase())
            .collect::<String>();
        let mut party = Party {
            id: PartyId(i as u32),
            name,
            short,
            platform: to_f32(&platform),
            factions: vec![
                Faction {
                    wing: "purist".into(),
                    position: to_f32(&left),
                    strength: 0.45,
                },
                Faction {
                    wing: "pragmatist".into(),
                    position: to_f32(&right),
                    strength: 0.55,
                },
            ],
            colour: i as u8,
        };
        party.recompute_platform();
        parties.push(party);
    }
    parties
}

/// Plurality politics: two major parties split the electorate along its main
/// cleavage (the first principal axis, cut at the weighted median), so they are
/// competitive by construction (Duverger). Minor parties form around the most
/// under-represented voters (largest distance from both majors).
fn major_minor_split(
    samples: &[Sample],
    mean: &[f64],
    k: usize,
    rng: &mut SimRng,
) -> Vec<(Vec<f64>, f64)> {
    let na = mean.len();
    // Weighted covariance and its leading eigenvector by power iteration.
    let mut cov = vec![vec![0.0f64; na]; na];
    let mut wsum = 0.0;
    for s in samples {
        for i in 0..na {
            for j in 0..na {
                cov[i][j] += s.w * (s.x[i] - mean[i]) * (s.x[j] - mean[j]);
            }
        }
        wsum += s.w;
    }
    let mut v = vec![0.0f64; na];
    for x in v.iter_mut() {
        *x = 0.5 + rng.next_f64();
    }
    for _ in 0..60 {
        let mut nv = vec![0.0f64; na];
        for i in 0..na {
            for j in 0..na {
                nv[i] += cov[i][j] * v[j];
            }
        }
        let n = mandate_core::math::sqrt(nv.iter().map(|x| x * x).sum::<f64>()).max(1e-12);
        for i in 0..na {
            v[i] = nv[i] / n;
        }
    }
    let mut proj: Vec<(f64, usize)> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| ((0..na).map(|a| (s.x[a] - mean[a]) * v[a]).sum(), i))
        .collect();
    proj.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut acc = 0.0;
    let mut side = vec![0usize; samples.len()];
    for (_, i) in &proj {
        acc += samples[*i].w;
        side[*i] = if acc <= wsum / 2.0 { 0 } else { 1 };
    }
    let centre = |pick: &dyn Fn(usize) -> bool| {
        let mut c = vec![0.0f64; na];
        let mut w = 0.0;
        for (i, s) in samples.iter().enumerate() {
            if pick(i) {
                for a in 0..na {
                    c[a] += s.x[a] * s.w;
                }
                w += s.w;
            }
        }
        for x in c.iter_mut() {
            *x /= w.max(1e-12);
        }
        (c, w / wsum.max(1e-12))
    };
    let mut out = vec![centre(&|i| side[i] == 0), centre(&|i| side[i] == 1)];
    // Minor parties: the tail of the next cross-cutting cleavage (principal
    // axes 2, 3, …), e.g. a green or localist party.
    let mut axes = vec![v];
    while out.len() < k {
        let mut u = vec![0.0f64; na];
        for x in u.iter_mut() {
            *x = 0.5 + rng.next_f64();
        }
        for _ in 0..60 {
            let mut nu = vec![0.0f64; na];
            for i in 0..na {
                for j in 0..na {
                    nu[i] += cov[i][j] * u[j];
                }
            }
            // Orthogonalise against the axes already used (deflation).
            for a in &axes {
                let d: f64 = (0..na).map(|i| nu[i] * a[i]).sum();
                for i in 0..na {
                    nu[i] -= d * a[i];
                }
            }
            let n = mandate_core::math::sqrt(nu.iter().map(|x| x * x).sum::<f64>()).max(1e-12);
            for i in 0..na {
                u[i] = nu[i] / n;
            }
        }
        let sign = if rng.chance(0.5) { 1.0 } else { -1.0 };
        let mut tail: Vec<(f64, usize)> = samples
            .iter()
            .enumerate()
            .map(|(i, s)| {
                (
                    sign * (0..na).map(|a| (s.x[a] - mean[a]) * u[a]).sum::<f64>(),
                    i,
                )
            })
            .collect();
        tail.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut chosen = vec![false; samples.len()];
        let mut got = 0.0;
        for (_, i) in &tail {
            if got >= wsum * 0.15 {
                break;
            }
            chosen[*i] = true;
            got += samples[*i].w;
        }
        out.push(centre(&|i| chosen[i]));
        axes.push(u);
    }
    // Largest first is already the case for the two majors (≈ 50/50).
    out
}

/// The attitude dimension where the platform departs most from the mean, and its sign.
fn distinctive(platform: &[f64], mean: &[f64]) -> (usize, f64) {
    let mut best = 0;
    let mut bv = 0.0f64;
    for a in 0..platform.len().min(mean.len()) {
        let d = platform[a] - mean[a];
        if d.abs() > bv.abs() {
            bv = d;
            best = a;
        }
    }
    (best, bv)
}

fn party_name(
    platform: &[f64],
    mean: &[f64],
    keys: &[String],
    params: &PoliticsParams,
    rng: &mut SimRng,
    used: &[String],
) -> String {
    // Try dimensions from most to least distinctive until a fresh name appears.
    let mut dims: Vec<usize> = (0..platform.len().min(keys.len())).collect();
    dims.sort_by(|a, b| {
        (platform[*b] - mean[*b])
            .abs()
            .total_cmp(&(platform[*a] - mean[*a]).abs())
    });
    for a in dims {
        let Some(words) = params
            .party_names
            .by_attitude
            .iter()
            .find(|w| w.attitude == keys[a])
        else {
            continue;
        };
        let pool = if platform[a] >= mean[a] {
            &words.pro
        } else {
            &words.anti
        };
        let Some(adj) = rng.pick(pool).cloned() else {
            continue;
        };
        let noun = rng
            .pick(&params.party_names.nouns)
            .cloned()
            .unwrap_or_else(|| "Party".into());
        let name = format!("{adj} {noun}");
        if !used.contains(&name) && !used.iter().any(|u| u.starts_with(&adj)) {
            return name;
        }
    }
    format!("Party {}", used.len() + 1)
}

/// Attachment of a cohort to each party (sums to ≤ `attached`): closer
/// platforms attract more, sharpened by `sharpness`.
pub fn attachment(attitudes: &[f32], parties: &[Party], attached: f64, sharpness: f64) -> Vec<f32> {
    if parties.is_empty() {
        return Vec::new();
    }
    let scores: Vec<f64> = parties
        .iter()
        .map(|p| {
            let d: f64 = attitudes
                .iter()
                .zip(&p.platform)
                .map(|(a, q)| {
                    let x = *a as f64 - *q as f64;
                    x * x
                })
                .sum();
            -sharpness * d
        })
        .collect();
    let max = scores.iter().cloned().fold(f64::MIN, f64::max);
    let exps: Vec<f64> = scores
        .iter()
        .map(|s| mandate_core::math::exp(s - max))
        .collect();
    let sum: f64 = exps.iter().sum::<f64>().max(1e-12);
    exps.iter().map(|e| (attached * e / sum) as f32).collect()
}

/// The party whose platform is nearest an attitude vector.
pub fn nearest_party(attitudes: &[f32], parties: &[Party]) -> Option<PartyId> {
    parties
        .iter()
        .map(|p| {
            let d: f64 = attitudes
                .iter()
                .zip(&p.platform)
                .map(|(a, q)| {
                    let x = *a as f64 - *q as f64;
                    x * x
                })
                .sum();
            (p.id, d)
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}
