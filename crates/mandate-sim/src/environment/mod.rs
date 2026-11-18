//! Environment dynamics (DEC-023): a seeded regional trajectory (warming,
//! drying, storminess), resource stocks, pollution, and hazard events.

use crate::events::EventKind;
use crate::scheduler::{Effect, Outs, Reads};
use crate::state::{Preset, WorldMeta};
use crate::world::World;
use mandate_content::SectorKind;
use mandate_core::math::clamp;
use mandate_core::{SimRng, Stream};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Trajectory {
    /// °C per year.
    pub warming: f64,
    /// Change in water availability per year.
    pub drying: f64,
    /// Change in storm intensity multiplier per year.
    pub storminess: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RegionEnv {
    pub temp_anomaly: f64,
    /// Water availability (1.0 = baseline).
    pub water: f64,
    /// Remaining share of the initial mineral stock.
    pub mineral_stock: f64,
    pub timber_stock: f64,
    pub fish_stock: f64,
    pub pollution: f64,
    pub storm_multiplier: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HazardKind {
    Flood,
    Drought,
    Wildfire,
    Storm,
    Earthquake,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentState {
    pub trajectory: Trajectory,
    pub regions: Vec<RegionEnv>,
    pub disasters_total: u32,
}

pub fn init(world: &World, meta: &WorldMeta, rng: &mut SimRng) -> EnvironmentState {
    let preset = meta.params.preset;
    let trajectory = Trajectory {
        warming: rng.range_f64(0.01, 0.045),
        drying: if preset == Some(Preset::DroughtFarmland) {
            0.012
        } else {
            rng.normal(0.002, 0.004)
        },
        storminess: if preset == Some(Preset::WealthyCoast) {
            0.012
        } else {
            rng.range_f64(0.0, 0.01)
        },
    };
    let regions = world
        .regions
        .iter()
        .map(|_| RegionEnv {
            temp_anomaly: 0.0,
            water: 1.0,
            mineral_stock: 1.0,
            timber_stock: 1.0,
            fish_stock: 1.0,
            pollution: 0.2,
            storm_multiplier: 1.0,
        })
        .collect();
    EnvironmentState {
        trajectory,
        regions,
        disasters_total: 0,
    }
}

/// Monthly drift of climate, stocks, and pollution.
pub fn drift(env: &mut EnvironmentState, r: &Reads<'_>, _out: &mut Outs<'_>) {
    let econ = r.economy();
    let t = env.trajectory.clone();
    let month = r.date.calendar().month;
    for (ri, reg) in r.world.regions.iter().enumerate() {
        let Some(e) = env.regions.get_mut(ri) else {
            continue;
        };
        e.temp_anomaly += t.warming / 12.0;
        let seasonal = if (6..=9).contains(&month) {
            -0.05
        } else {
            0.02
        };
        e.water = clamp(
            e.water - t.drying / 12.0 - e.temp_anomaly * 0.0004 + seasonal * 0.02,
            0.2,
            1.4,
        );
        e.storm_multiplier = clamp(e.storm_multiplier + t.storminess / 12.0, 0.5, 3.0);
        let (mut resource_jobs, mut industry) = (0.0, 0.0);
        for m in &reg.municipalities {
            if let Some(me) = econ.munis.get(m.index()) {
                resource_jobs += me.sectors[SectorKind::Resources.index()].jobs;
                industry += me.sectors[SectorKind::Manufacturing.index()].jobs
                    + me.sectors[SectorKind::Resources.index()].jobs;
            }
        }
        let scale = 1.0 / 60_000.0;
        e.mineral_stock = clamp(e.mineral_stock - resource_jobs * scale * 0.002, 0.0, 1.0);
        e.timber_stock = clamp(
            e.timber_stock + 0.002 - resource_jobs * scale * 0.0015,
            0.0,
            1.0,
        );
        e.fish_stock = clamp(
            e.fish_stock + 0.004 * e.fish_stock * (1.0 - e.fish_stock)
                - resource_jobs * scale * 0.0005,
            0.05,
            1.0,
        );
        e.pollution = clamp(
            e.pollution * (1.0 - 0.004) + industry * scale * 0.002,
            0.0,
            3.0,
        );
    }
}

/// Monthly hazard events per municipality; consequences flow as effects.
pub fn hazards(env: &mut EnvironmentState, r: &Reads<'_>, out: &mut Outs<'_>) {
    let month = r.date.calendar().month;
    if month == 7 {
        regional_droughts(env, r, out);
    }
    let summer = (6..=9).contains(&month);
    let stormy_season = matches!(month, 9..=12 | 1..=2);
    for (m, mu) in r.world.municipalities.iter().enumerate() {
        let g = &mu.geo;
        let Some(e) = env.regions.get(mu.region.index()) else {
            continue;
        };
        let fin = r.finance();
        let mods =
            crate::policy::mods_for(r.policy(), r.content, m as u32, &|id| fin.funded_share(id));
        let rng = out.rng.get(Stream::Environment);
        let checks = [
            (HazardKind::Flood, 0.0035 * g.flood * e.storm_multiplier),
            // Droughts are regional; see `regional_droughts`.
            (
                HazardKind::Wildfire,
                if summer {
                    0.004 * g.wildfire * (1.0 + e.temp_anomaly) / e.water.max(0.3)
                } else {
                    0.0
                },
            ),
            (
                HazardKind::Storm,
                if stormy_season {
                    0.004 * g.storm * e.storm_multiplier
                } else {
                    0.0
                },
            ),
            (HazardKind::Earthquake, 0.0003 * g.seismic),
        ];
        for (kind, p) in checks {
            if !rng.chance(p) {
                continue;
            }
            let severity = rng.range_f64(0.2, 1.0);
            env.disasters_total += 1;
            let ev = out.events.record(
                r.date,
                EventKind::Disaster {
                    municipality: m as u32,
                    hazard: kind,
                    severity_pct: (severity * 100.0).round() as u32,
                },
                vec![],
            );
            if severity > 0.7 {
                out.events.pin(ev);
            }
            let cause = Some(ev);
            let municipality = m as u32;
            match kind {
                HazardKind::Flood | HazardKind::Storm | HazardKind::Earthquake => {
                    // Protective policies (flood defences) reduce the damage.
                    let protection = match kind {
                        HazardKind::Flood => mods.flood_protection,
                        HazardKind::Storm => mods.storm_protection,
                        _ => 0.0,
                    };
                    let damage = (1.0 - protection)
                        * severity
                        * if kind == HazardKind::Earthquake {
                            0.04
                        } else {
                            0.02
                        };
                    out.effects.push(Effect::HousingDamage {
                        municipality,
                        fraction: damage,
                        cause,
                    });
                    out.effects.push(Effect::Displacement {
                        municipality,
                        fraction: damage * 0.25,
                        cause,
                    });
                    out.effects.push(Effect::SectorShock {
                        municipality,
                        sector: SectorKind::Tourism,
                        factor: 1.0 - 0.4 * severity,
                        months: 6,
                        cause,
                    });
                }
                HazardKind::Drought => {
                    out.effects.push(Effect::SectorShock {
                        municipality,
                        sector: SectorKind::Agriculture,
                        factor: 1.0 - 0.45 * severity,
                        months: 12,
                        cause,
                    });
                }
                HazardKind::Wildfire => {
                    out.effects.push(Effect::HousingDamage {
                        municipality,
                        fraction: severity * 0.008 * (1.0 - mods.wildfire_protection),
                        cause,
                    });
                    out.effects.push(Effect::SectorShock {
                        municipality,
                        sector: SectorKind::Tourism,
                        factor: 1.0 - 0.5 * severity,
                        months: 9,
                        cause,
                    });
                    out.effects.push(Effect::SectorShock {
                        municipality,
                        sector: SectorKind::Resources,
                        factor: 1.0 - 0.3 * severity,
                        months: 12,
                        cause,
                    });
                }
            }
        }
    }
}

/// Each July, a drought may strike a whole region; drought-prone towns in it
/// lose farm output for a year. One event per region, at its capital.
fn regional_droughts(env: &mut EnvironmentState, r: &Reads<'_>, out: &mut Outs<'_>) {
    for (ri, reg) in r.world.regions.iter().enumerate() {
        let Some(e) = env.regions.get(ri) else {
            continue;
        };
        let n = reg.municipalities.len().max(1) as f64;
        let mean = reg
            .municipalities
            .iter()
            .map(|m| r.world.municipalities[m.index()].geo.drought)
            .sum::<f64>()
            / n;
        let rng = out.rng.get(Stream::Environment);
        if !rng.chance(0.06 * mean / e.water.max(0.3)) {
            continue;
        }
        let severity = rng.range_f64(0.3, 1.0);
        env.disasters_total += 1;
        let ev = out.events.record(
            r.date,
            EventKind::Disaster {
                municipality: reg.capital.0,
                hazard: HazardKind::Drought,
                severity_pct: (severity * 100.0).round() as u32,
            },
            vec![],
        );
        if severity > 0.7 {
            out.events.pin(ev);
        }
        for m in &reg.municipalities {
            let g = &r.world.municipalities[m.index()].geo;
            if g.drought >= 0.25 {
                out.effects.push(Effect::SectorShock {
                    municipality: m.0,
                    sector: SectorKind::Agriculture,
                    factor: 1.0 - 0.45 * severity * g.drought.min(1.0),
                    months: 12,
                    cause: Some(ev),
                });
            }
        }
    }
}
