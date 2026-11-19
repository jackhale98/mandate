//! Environment dynamics (DEC-023): a seeded regional trajectory (warming,
//! drying, storminess), resource stocks, pollution, and hazard events.
//! Hazards strike towns; within a town they fall by neighbourhood exposure
//! (DEC-090 D8, `exposure`).

pub mod exposure;

pub use exposure::{HoodDamage, HoodExposure, HoodHazard, HoodHit, hood_hazard};

use crate::events::EventKind;
use crate::scheduler::{Effect, EffectQueue, Outs, Reads};
use crate::state::WorldMeta;
use crate::world::{NeighborhoodId, World};
use mandate_content::{ContentBundle, SectorDriver};
use mandate_core::math::clamp;
use mandate_core::{SimDate, SimRng, Stream};
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

/// A hazard the director held back (DEC-033 B: timing only). It strikes at
/// its drawn severity once the hold lifts, and at the latest
/// `director.chance.hazard_max_defer_days` after it was drawn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeferredHazard {
    pub municipality: u32,
    pub kind: HazardKind,
    pub severity: f64,
    pub drawn: mandate_core::SimDate,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentState {
    pub trajectory: Trajectory,
    pub regions: Vec<RegionEnv>,
    pub disasters_total: u32,
    /// Hazards held by the director, waiting to strike.
    #[serde(default)]
    pub deferred: Vec<DeferredHazard>,
    /// Each neighbourhood's exposure, indexed by neighbourhood (DEC-090 D8):
    /// fixed for a run, derived as the world is generated (or, for an older
    /// save, when it is migrated). Empty: every neighbourhood alike.
    pub exposure: Vec<HoodExposure>,
    /// Neighbourhoods with homes damaged and not yet repaired, by
    /// neighbourhood id (ascending).
    pub hood_damage: Vec<HoodDamage>,
}

impl EnvironmentState {
    /// A neighbourhood's exposure (`None` before exposure is derived).
    pub fn exposure(&self, id: NeighborhoodId) -> Option<HoodExposure> {
        self.exposure.get(id.index()).copied()
    }

    /// A neighbourhood's unrepaired damage, if any.
    pub fn damage(&self, id: NeighborhoodId) -> Option<&HoodDamage> {
        self.hood_damage
            .binary_search_by_key(&id.0, |d| d.neighborhood)
            .ok()
            .and_then(|i| self.hood_damage.get(i))
    }

    /// Record a strike's damage in a neighbourhood (added to what is still
    /// unrepaired).
    fn record_damage(&mut self, neighborhood: u32, hazard: HazardKind, date: SimDate, damage: f64) {
        let pm = exposure::per_mille(damage);
        if pm == 0 {
            return;
        }
        match self
            .hood_damage
            .binary_search_by_key(&neighborhood, |d| d.neighborhood)
        {
            Ok(i) => {
                let d = &mut self.hood_damage[i];
                d.damaged_pm = d.damaged_pm.saturating_add(pm).min(1000);
                d.hazard = hazard;
                d.struck = date;
            }
            Err(i) => self.hood_damage.insert(
                i,
                HoodDamage {
                    neighborhood,
                    hazard,
                    struck: date,
                    damaged_pm: pm,
                },
            ),
        }
    }
}

pub fn init(
    world: &World,
    meta: &WorldMeta,
    content: &mandate_content::ContentBundle,
    rng: &mut SimRng,
) -> EnvironmentState {
    // A scenario's era fixes a trend instead of drawing it (no draw is made).
    let era = content
        .scenario_or_random(meta.params.scenario.as_deref())
        .era;
    let trajectory = Trajectory {
        warming: rng.range_f64(0.01, 0.045),
        drying: match era.drying {
            Some(v) => v,
            None => rng.normal(0.002, 0.004),
        },
        storminess: match era.storminess {
            Some(v) => v,
            None => rng.range_f64(0.0, 0.01),
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
        deferred: Vec::new(),
        // Derived from the terrain by worldgen once the world exists.
        exposure: Vec::new(),
        hood_damage: Vec::new(),
    }
}

/// Monthly drift of climate, stocks, and pollution; damaged homes are
/// repaired.
pub fn drift(env: &mut EnvironmentState, r: &Reads<'_>, _out: &mut Outs<'_>) {
    let rate = r.content.economy.hazards.repair_rate.clamp(0.0, 1.0);
    for d in env.hood_damage.iter_mut() {
        let fixed = (f64::from(d.damaged_pm) * rate).ceil().max(1.0) as u16;
        d.damaged_pm = d.damaged_pm.saturating_sub(fixed);
    }
    env.hood_damage.retain(|d| d.damaged_pm > 0);
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
                let extraction = me.jobs_with(r.content, SectorDriver::Extraction);
                resource_jobs += extraction;
                industry += me.jobs_with(r.content, SectorDriver::Industrial) + extraction;
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
            // The same sequential stream as ever (re-borrowed per check).
            let rng = out.rng.get(Stream::Environment);
            if !rng.chance(p) {
                continue;
            }
            let severity = rng.range_f64(0.2, 1.0);
            // The director's levers (DEC-033 B) apply to the player's town:
            // a held strike waits (timing only); under a thinner profile some
            // strikes miss. Neither draws from this stream.
            match director_lever(r, m as u32) {
                Lever::Hold => {
                    env.deferred.push(DeferredHazard {
                        municipality: m as u32,
                        kind,
                        severity,
                        drawn: r.date,
                    });
                    out.events.record(
                        r.date,
                        EventKind::DirectorIntervention {
                            lever: "hazard_held".into(),
                            municipality: m as u32,
                        },
                        vec![],
                    );
                    continue;
                }
                Lever::Miss => {
                    out.events.record(
                        r.date,
                        EventKind::DirectorIntervention {
                            lever: "hazard_averted".into(),
                            municipality: m as u32,
                        },
                        vec![],
                    );
                    continue;
                }
                Lever::None => {}
            }
            strike(env, r, out, m as u32, kind, severity, &mods);
        }
    }
    // Held hazards strike once the hold lifts, or at the latest after the
    // longest deferral the director may impose.
    let max = r.content.director.chance.hazard_max_defer_days;
    let due: Vec<DeferredHazard> = {
        let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut env.deferred)
            .into_iter()
            .partition(|h| h.drawn.add_days(max) <= r.date || !held(r, h.municipality));
        env.deferred = keep;
        due
    };
    for h in due {
        // A thinner profile thins late strikes too.
        if missed(r, h.municipality) {
            out.events.record(
                r.date,
                EventKind::DirectorIntervention {
                    lever: "hazard_averted".into(),
                    municipality: h.municipality,
                },
                vec![],
            );
            continue;
        }
        let fin = r.finance();
        let mods = crate::policy::mods_for(r.policy(), r.content, h.municipality, &|id| {
            fin.funded_share(id)
        });
        out.events.record(
            r.date,
            EventKind::DirectorIntervention {
                lever: "hazard_struck_late".into(),
                municipality: h.municipality,
            },
            vec![],
        );
        strike(env, r, out, h.municipality, h.kind, h.severity, &mods);
    }
}

enum Lever {
    None,
    Hold,
    Miss,
}

/// The director holds hazards on this town now.
fn held(r: &Reads<'_>, m: u32) -> bool {
    r.narrative()
        .chance
        .hazard_hold
        .is_some_and(|(t, until)| t == m && r.date < until)
}

fn director_lever(r: &Reads<'_>, m: u32) -> Lever {
    if held(r, m) {
        return Lever::Hold;
    }
    if missed(r, m) {
        return Lever::Miss;
    }
    Lever::None
}

/// Under a thinner profile, does this strike on the town miss?
fn missed(r: &Reads<'_>, m: u32) -> bool {
    if let Some((t, keep)) = r.narrative().chance.hazard_keep
        && t == m
    {
        // Keyed by the director's stream (never this system's sequence).
        let mut k = mandate_core::rng::keyed(
            r.meta.seed,
            Stream::Director,
            &[crate::narrative::KEY_HAZARD_KEEP, r.date.0 as u64, m as u64],
        );
        if !k.chance(keep as f64) {
            return true;
        }
    }
    false
}

/// A hazard strikes a municipality: the event and its consequences as effects.
fn strike(
    env: &mut EnvironmentState,
    r: &Reads<'_>,
    out: &mut Outs<'_>,
    municipality: u32,
    kind: HazardKind,
    severity: f64,
    mods: &crate::policy::PolicyMods,
) {
    env.disasters_total += 1;
    let ev = out.events.record(
        r.date,
        EventKind::Disaster {
            municipality,
            hazard: kind,
            severity_pct: (severity * 100.0).round() as u32,
        },
        vec![],
    );
    if severity > 0.7 {
        out.events.pin(ev);
    }
    let cause = Some(ev);
    match kind {
        HazardKind::Flood | HazardKind::Storm | HazardKind::Earthquake => {
            // Protective policies (flood defences) reduce the damage.
            let protection = match kind {
                HazardKind::Flood => mods.flood_protection,
                HazardKind::Storm => mods.storm_protection,
                _ => 0.0,
            };
            let damage = severity
                * if kind == HazardKind::Earthquake {
                    0.04
                } else {
                    0.02
                };
            // Shared by exposure: the flood-prone district takes most of a
            // flood; the town's total is the same as an even split.
            let town = fall(
                env,
                r,
                out,
                municipality,
                kind,
                damage,
                protection,
                true,
                cause,
            );
            out.effects.push(Effect::HousingDamage {
                municipality,
                fraction: town,
                cause,
            });
            sector_shock(
                out.effects,
                r.content,
                SectorDriver::Visitor,
                municipality,
                1.0 - 0.4 * severity,
                6,
                cause,
            );
        }
        HazardKind::Drought => {
            // Felt evenly across the town.
            fall(env, r, out, municipality, kind, 0.0, 0.0, false, cause);
            sector_shock(
                out.effects,
                r.content,
                SectorDriver::LandBased,
                municipality,
                1.0 - 0.45 * severity,
                12,
                cause,
            );
        }
        HazardKind::Wildfire => {
            // The forest edge burns; those who lose homes stay elsewhere for
            // a while, none leave the town for good (as before).
            let town = fall(
                env,
                r,
                out,
                municipality,
                kind,
                severity * 0.008,
                mods.wildfire_protection,
                false,
                cause,
            );
            out.effects.push(Effect::HousingDamage {
                municipality,
                fraction: town,
                cause,
            });
            sector_shock(
                out.effects,
                r.content,
                SectorDriver::Visitor,
                municipality,
                1.0 - 0.5 * severity,
                9,
                cause,
            );
            sector_shock(
                out.effects,
                r.content,
                SectorDriver::Extraction,
                municipality,
                1.0 - 0.3 * severity,
                12,
                cause,
            );
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
                fall(
                    env,
                    r,
                    out,
                    m.0,
                    HazardKind::Drought,
                    0.0,
                    0.0,
                    false,
                    Some(ev),
                );
                sector_shock(
                    out.effects,
                    r.content,
                    SectorDriver::LandBased,
                    m.0,
                    1.0 - 0.45 * severity * g.drought.min(1.0),
                    12,
                    Some(ev),
                );
            }
        }
    }
}

/// Where a strike falls in a town (DEC-090 D8): its damage (before
/// protection) shared by neighbourhood exposure, the footprint recorded, and
/// each neighbourhood's share sent to the population (`HoodDisaster`: who
/// leaves for good if `leaves`, who stays elsewhere for a while, how hard it
/// was hit). Returns the town's damage, a share of its homes.
#[allow(clippy::too_many_arguments)]
fn fall(
    env: &mut EnvironmentState,
    r: &Reads<'_>,
    out: &mut Outs<'_>,
    municipality: u32,
    kind: HazardKind,
    damage: f64,
    protection: f64,
    leaves: bool,
    cause: Option<crate::events::EventId>,
) -> f64 {
    let params = &r.content.economy.hazards;
    let pop = r.population();
    let residents = |h: usize| pop.hood_total(h);
    // Protection is a town's policies today; a neighbourhood-level clause
    // (DEC-092 families) would answer per neighbourhood here.
    let hits = exposure::split(
        r.world,
        &env.exposure,
        &residents,
        municipality,
        kind,
        damage,
        &|_| protection,
        params,
    );
    let mut town = 0.0;
    for hit in &hits {
        town += hit.share * hit.damage;
        env.record_damage(hit.neighborhood, kind, r.date, hit.damage);
        out.effects.push(Effect::HoodDisaster {
            neighborhood: hit.neighborhood,
            relative: hit.relative,
            leave: if leaves {
                hit.damage * params.leave_share.clamp(0.0, 1.0)
            } else {
                0.0
            },
            temporary: hit.damage * params.temporary_share.clamp(0.0, 1.0),
            cause,
        });
    }
    town
}

/// Shock every sector with a driver in a municipality (economy applies).
fn sector_shock(
    effects: &mut EffectQueue,
    content: &ContentBundle,
    driver: SectorDriver,
    municipality: u32,
    factor: f64,
    months: u16,
    cause: Option<crate::events::EventId>,
) {
    for (_, def) in content.sectors_with(driver) {
        effects.push(Effect::SectorShock {
            municipality,
            sector: def.id.clone(),
            factor,
            months,
            cause,
        });
    }
}
