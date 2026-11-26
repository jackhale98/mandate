//! Policy families in the lab (DEC-092; M6_ENGINE_CONTRACTS §2.13 item 11,
//! §2.14): what a world's families do over a run.
//!
//! - `policies --seed <s>` plays one world (NPC turns, the player idle) and
//!   lists the home town's deck, every proposal that opened (bindings by
//!   name, sponsors, status), the default package's cost, budget share and
//!   support, and each sponsor's version with its cost and price.
//! - `policies --coverage --seeds <n>` plays n worlds and reports, per
//!   family, how often it is drawn, binds and opens, its proposals per run,
//!   median months open, median default cost and support, what failed most,
//!   and in how many jurisdictions of its tier it can bind. **A CI gate:** a
//!   family that binds in no world fails it.
//! - `policies --check` is validation item 11: every family binds in at
//!   least one canonical test world (the five scenarios at two sizes, with
//!   a 36-month run). A family that never binds is an error.
//!
//! "Binds" means the family's slots bind and its trigger holds
//! (`policy::families::probe`) in a jurisdiction of its tier at some month
//! of the run, whatever the deck, the cap on open proposals or the focus
//! (DEC-098: the home town every month, every other jurisdiction of the tier
//! each quarter). "Opens" means a proposal opened in a focus jurisdiction.
//! Read-only probes; the run itself is the ordinary simulation. Numbers in
//! `--set` reach families (`policy_families[family.x].draw.weight=2`) and
//! their parameters (`policy_family_params.max_open_per_jurisdiction=3`).

use crate::lab::runner::{Driver, Game, Setup, par_map, threads};
use mandate_content::policy_family::PolicyFamilyDef;
use mandate_content::{ContentBundle, TierKey};
use mandate_core::SimDate;
use mandate_sim::Engine;
use mandate_sim::policy::families::{self, Bound, FamilyInstance, FamilyStatus, SlotBinding};
use mandate_sim::world::{Jurisdiction, MunicipalityId, RegionId};
use mandate_worldgen::WorldSize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Months of a coverage or listing run unless told otherwise.
pub const MONTHS: u32 = 24;
/// Months of a canonical world's run (validation item 11).
pub const CHECK_MONTHS: u32 = 36;
/// Every jurisdiction of a family's tier is probed every this many months
/// (the home town every month).
pub const WIDE_EVERY: u32 = 3;

/// One world played for the families.
pub struct World {
    pub seed: u64,
    pub size: WorldSize,
    pub scenario: Option<String>,
    pub e: Engine,
    /// Per family (registry order): binds anywhere, binds in the home town,
    /// why it failed in the home town (`slot <id>`, `trigger`, `tier`) by
    /// count, and the jurisdictions of its tier where it ever bound.
    pub probes: Vec<Probe>,
    /// The home town's indicators (the conditions families read), the
    /// highest each reached over the run: what a trigger could have met.
    pub home_max: BTreeMap<String, f64>,
}

#[derive(Default, Clone)]
pub struct Probe {
    pub home_months: u32,
    pub failed: BTreeMap<String, u32>,
    pub bound_in: Vec<Jurisdiction>,
    pub probed_in: Vec<Jurisdiction>,
}

impl Probe {
    pub fn binds(&self) -> bool {
        self.home_months > 0 || !self.bound_in.is_empty()
    }
}

fn size_name(s: WorldSize) -> &'static str {
    match s {
        WorldSize::Small => "small",
        WorldSize::Medium => "medium",
        WorldSize::Large => "large",
    }
}

/// The jurisdictions of a tier in a world (DEC-098).
fn of_tier(e: &Engine, tier: TierKey) -> Vec<Jurisdiction> {
    let w = &e.state().world;
    match tier {
        TierKey::Local => (0..w.municipalities.len() as u32)
            .map(|i| Jurisdiction::town(MunicipalityId(i)))
            .collect(),
        TierKey::Regional => (0..w.regions.len() as u32)
            .map(|i| Jurisdiction::Region { id: RegionId(i) })
            .collect(),
        TierKey::National => vec![Jurisdiction::Nation],
    }
}

fn home(e: &Engine) -> Option<Jurisdiction> {
    mandate_sim::player::home(e.state()).map(Jurisdiction::town)
}

/// Probe every family where this month asks (the home town; every
/// jurisdiction of its tier on a quarter).
fn probe_month(e: &Engine, content: &ContentBundle, month: u32, probes: &mut [Probe]) {
    let home = home(e);
    let wide = month.is_multiple_of(WIDE_EVERY);
    for (f, p) in content.policy_families.iter().zip(probes.iter_mut()) {
        if f.disabled {
            continue;
        }
        if let Some(h) = home.filter(|h| h.tier() == f.tier) {
            match families::probe(e.state(), content, f, h) {
                Ok(_) => {
                    p.home_months += 1;
                    if !p.bound_in.contains(&h) {
                        p.bound_in.push(h);
                    }
                }
                Err(why) => *p.failed.entry(why).or_default() += 1,
            }
        }
        if wide {
            for at in of_tier(e, f.tier) {
                if Some(at) == home {
                    continue;
                }
                if !p.probed_in.contains(&at) {
                    p.probed_in.push(at);
                }
                if !p.bound_in.contains(&at) && families::probe(e.state(), content, f, at).is_ok()
                {
                    p.bound_in.push(at);
                }
            }
        }
    }
}

/// Play a world `months` months with NPC turns (the player idle), probing
/// the families each month.
pub fn play(
    content: &Arc<ContentBundle>,
    seed: u64,
    size: WorldSize,
    scenario: Option<&str>,
    months: u32,
) -> Result<World, String> {
    let setup = Setup {
        seed,
        size,
        scenario: scenario.map(str::to_string),
    };
    let mut g = Game::new(content, &setup, Driver::Idle)?;
    let mut probes = vec![Probe::default(); content.policy_families.len()];
    for p in probes.iter_mut() {
        if let Some(h) = home(&g.e) {
            p.probed_in.push(h);
        }
    }
    let mut home_max = BTreeMap::new();
    probe_month(&g.e, content, 0, &mut probes);
    indicators(&g.e, content, &mut home_max);
    for m in 1..=months {
        g.step_month();
        probe_month(&g.e, content, m, &mut probes);
        indicators(&g.e, content, &mut home_max);
    }
    Ok(World {
        seed,
        size,
        scenario: scenario.map(str::to_string),
        e: g.e,
        probes,
        home_max,
    })
}

/// Raise each of the home town's indicators to its value now, if higher.
fn indicators(e: &Engine, content: &ContentBundle, max: &mut BTreeMap<String, f64>) {
    let Some(m) = mandate_sim::player::home(e.state()) else {
        return;
    };
    let r = mandate_sim::scheduler::Reads::of_state(e.state(), content);
    for which in mandate_content::Indicator::ALL {
        let v = mandate_sim::concerns::indicator(&r, m.index(), which);
        let k = serde_json::to_value(which)
            .ok()
            .and_then(|x| x.as_str().map(str::to_string))
            .unwrap_or_default();
        let x = max.entry(k).or_insert(f64::MIN);
        if v > *x {
            *x = v;
        }
    }
}

fn jurisdiction_json(e: &Engine, at: Jurisdiction) -> Value {
    let w = &e.state().world;
    let name = match at {
        Jurisdiction::Municipality { id } => w.municipalities.get(id.index()).map(|m| m.name.clone()),
        Jurisdiction::Region { id } => w.regions.get(id.index()).map(|r| r.name.clone()),
        Jurisdiction::Nation => None,
    };
    let tier = match at.tier() {
        TierKey::Local => "local",
        TierKey::Regional => "regional",
        TierKey::National => "national",
    };
    json!({ "tier": tier, "id": at.raw(), "name": name })
}

fn binding_json(e: &Engine, b: &SlotBinding) -> Value {
    let st = e.state();
    let (kind, name) = match &b.bound {
        Bound::Place { place } => (
            "place",
            st.world.places.get(place.index()).map(|p| p.name.clone()),
        ),
        Bound::Neighborhood { hood } => (
            "neighborhood",
            st.world.neighborhoods.get(hood.index()).map(|n| n.name.clone()),
        ),
        Bound::Person { person } => (
            "person",
            mandate_sim::characters::profile(&st.politics, &st.world, *person)
                .map(|p| p.full_name()),
        ),
        Bound::Sector { sector } => ("sector", Some(sector.clone())),
        Bound::Crisis { kind } => ("crisis", Some(kind.clone())),
        Bound::Hazard { hazard } => (
            "hazard",
            Some(families::hazard_word(*hazard).to_string()),
        ),
        Bound::Unbound => ("unbound", None),
    };
    json!({ "slot": b.slot, "kind": kind, "name": name })
}

fn date(d: SimDate) -> String {
    let c = d.calendar();
    format!("{:04}-{:02}-{:02}", c.year, c.month, c.day)
}

fn months_open(e: &Engine, inst: &FamilyInstance) -> u32 {
    let end = match &inst.status {
        FamilyStatus::Open => e.state().calendar.date,
        FamilyStatus::Lapsed { on, .. } => *on,
    };
    inst.opened.days_until(end) / 30
}

/// The engine's preview of a proposal's package (as the best-informed
/// player would see it: the lab switch is on in `costed`).
fn package(
    e: &Engine,
    inst: &FamilyInstance,
    choices: &[mandate_protocol::proposals::ChoiceValue],
    params: &[mandate_protocol::ParamValue],
) -> Option<Value> {
    let v = mandate_protocol::proposals::family_preview(e, &inst.family, inst.id.0, params, choices)?;
    Some(json!({
        "est_annual_cost": v.est_annual_cost,
        "budget_share": v.budget_share,
        "support": v.support.overall,
        "support_band": v.support.band,
        "months": v.family.as_ref().map(|f| f.months),
        "available": v.available,
    }))
}

/// A proposal as the listing prints it: bindings, sponsors, status, the
/// default package and each sponsor's version (open proposals only).
fn instance_json(e: &Engine, content: &ContentBundle, inst: &FamilyInstance) -> Value {
    let (status, lapse) = match &inst.status {
        FamilyStatus::Open => ("open", Value::Null),
        FamilyStatus::Lapsed { on, reason } => (
            "lapsed",
            json!({ "on": date(*on), "reason": format!("{reason:?}").to_lowercase() }),
        ),
    };
    let f = content.family(&inst.family);
    let mut versions = Vec::new();
    let mut default = Value::Null;
    if inst.is_open() {
        default = package(e, inst, &[], &[]).unwrap_or(Value::Null);
        for s in f.map(|f| f.sponsors.as_slice()).unwrap_or_default() {
            let Some(person) = inst.sponsor(&s.id) else {
                continue;
            };
            let choices: Vec<_> = s
                .preset
                .iter()
                .map(|(a, o)| mandate_protocol::proposals::ChoiceValue {
                    axis: a.clone(),
                    option: o.clone(),
                })
                .collect();
            let params: Vec<_> = s
                .params
                .iter()
                .map(|(k, v)| mandate_protocol::ParamValue {
                    id: k.clone(),
                    value: *v,
                })
                .collect();
            let st = e.state();
            versions.push(json!({
                "sponsor": s.id,
                "person": mandate_sim::characters::profile(&st.politics, &st.world, person).map(|p| p.full_name()),
                "preset": s.preset,
                "params": s.params,
                "price": serde_json::to_value(&s.price).unwrap_or(Value::Null),
                "package": package(e, inst, &choices, &params),
            }));
        }
    }
    json!({
        "id": inst.id.0,
        "family": inst.family,
        "at": jurisdiction_json(e, inst.at),
        "opened": date(inst.opened),
        "status": status,
        "lapse": lapse,
        "months_open": months_open(e, inst),
        "bindings": inst.bindings.iter().map(|b| binding_json(e, b)).collect::<Vec<_>>(),
        "args": inst.args,
        "bills": inst.drafts.len(),
        "default": default,
        "sponsor_versions": versions,
    })
}

/// The lab switch on (every subject known in detail), so previews show what
/// the best-informed player would: the run is over, nothing else reads it.
fn costed(w: &mut World) {
    w.e.set_lab_omniscient(true);
}

/// `policies --seed`: one world's families.
pub fn listing(
    content: &Arc<ContentBundle>,
    seed: u64,
    size: WorldSize,
    scenario: Option<&str>,
    town: Option<u32>,
    months: u32,
) -> Result<Value, String> {
    let mut w = play(content, seed, size, scenario, months)?;
    costed(&mut w);
    let e = &w.e;
    let st = e.state();
    let home = home(e);
    let deck: Vec<String> = home
        .and_then(|h| st.policy.families.decks.get(&h))
        .cloned()
        .unwrap_or_default();
    let instances: Vec<Value> = st
        .policy
        .families
        .instances
        .iter()
        .map(|i| instance_json(e, content, i))
        .collect();
    let probe_at = town.map(|t| Jurisdiction::town(MunicipalityId(t)));
    let probes: Vec<Value> = content
        .policy_families
        .iter()
        .zip(&w.probes)
        .map(|(f, p)| {
            let now = probe_at.map(|at| match families::probe(st, content, f, at) {
                Ok(b) => json!({ "binds": true, "bindings": b.iter().map(|x| binding_json(e, x)).collect::<Vec<_>>() }),
                Err(why) => json!({ "binds": false, "why": why }),
            });
            json!({
                "family": f.id,
                "in_deck": deck.contains(&f.id),
                "home_months_bound": p.home_months,
                "home_failed": p.failed,
                "jurisdictions_bound": p.bound_in.len(),
                "jurisdictions_probed": p.probed_in.len(),
                "town": now,
            })
        })
        .collect();
    Ok(json!({
        "seed": seed,
        "size": size_name(size),
        "scenario": scenario,
        "months": months,
        "home": home.map(|h| jurisdiction_json(e, h)),
        "deck": deck,
        "max_open_per_jurisdiction": content.policy_family_params.max_open_per_jurisdiction,
        "instances": instances,
        "families": probes,
    }))
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    let n = v.len();
    Some(if n.is_multiple_of(2) {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    } else {
        v[n / 2]
    })
}

/// Per-family coverage over worlds (§2.14), and the gate.
fn coverage_of(content: &ContentBundle, worlds: &mut [World]) -> Value {
    let n = worlds.len().max(1) as f64;
    for w in worlds.iter_mut() {
        costed(w);
    }
    let mut rows = Vec::new();
    let mut never = Vec::new();
    let mut never_opened = Vec::new();
    for (i, f) in content.policy_families.iter().enumerate() {
        let mut drawn = 0u32;
        let mut binds = 0u32;
        let mut home_binds = 0u32;
        let mut opened_runs = 0u32;
        let mut instances = 0u32;
        let mut open_months = Vec::new();
        let mut costs = Vec::new();
        let mut supports = Vec::new();
        let mut failed: BTreeMap<String, u32> = BTreeMap::new();
        let mut bound_in = 0usize;
        let mut probed_in = 0usize;
        let mut worlds_binding: Vec<u64> = Vec::new();
        for w in worlds.iter() {
            let e = &w.e;
            let st = e.state();
            let p = &w.probes[i];
            let in_deck = home(e)
                .and_then(|h| st.policy.families.decks.get(&h))
                .is_some_and(|d| d.contains(&f.id));
            drawn += in_deck as u32;
            if !in_deck {
                *failed.entry("not drawn".into()).or_default() += 1;
            }
            for (k, v) in &p.failed {
                *failed.entry(k.clone()).or_default() += v;
            }
            home_binds += (p.home_months > 0) as u32;
            if p.binds() {
                binds += 1;
                worlds_binding.push(w.seed);
            }
            bound_in += p.bound_in.len();
            probed_in += p.probed_in.len();
            let mine: Vec<&FamilyInstance> = st
                .policy
                .families
                .instances
                .iter()
                .filter(|x| x.family == f.id)
                .collect();
            opened_runs += (!mine.is_empty()) as u32;
            instances += mine.len() as u32;
            for x in &mine {
                open_months.push(months_open(e, x) as f64);
                if x.is_open()
                    && let Some(v) = mandate_protocol::proposals::family_preview(
                        e, &x.family, x.id.0, &[], &[],
                    )
                {
                    costs.push(v.est_annual_cost);
                    if let Some(s) = v.support.overall {
                        supports.push(s);
                    }
                }
            }
        }
        if binds == 0 {
            never.push(f.id.clone());
        }
        if opened_runs == 0 {
            never_opened.push(f.id.clone());
        }
        let failed_most = failed
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
            .map(|(k, v)| json!({ "why": k, "count": v }));
        rows.push(json!({
            "family": f.id,
            "tier": tier_name(f.tier),
            "rarity": format!("{:?}", f.draw.rarity).to_lowercase(),
            "binds_on": binds_on(f),
            "drawn_share": drawn as f64 / n,
            "binds_share": binds as f64 / n,
            "home_binds_share": home_binds as f64 / n,
            "worlds_binding": worlds_binding,
            "opened_share": opened_runs as f64 / n,
            "instances_per_run": instances as f64 / n,
            "median_months_open": median(open_months),
            "median_default_cost": median(costs),
            "median_default_support": median(supports),
            "failed_most": failed_most,
            "jurisdictions_bound": bound_in,
            "jurisdictions_probed": probed_in,
        }));
    }
    // Proposals per jurisdiction: the focus towns' open and total counts.
    let mut per_jurisdiction = Vec::new();
    for w in worlds.iter() {
        let st = w.e.state();
        let mut by: BTreeMap<Jurisdiction, (u32, u32)> = BTreeMap::new();
        for x in &st.policy.families.instances {
            let c = by.entry(x.at).or_default();
            c.1 += 1;
            c.0 += x.is_open() as u32;
        }
        for (at, (open, total)) in by {
            per_jurisdiction.push(json!({
                "seed": w.seed,
                "at": jurisdiction_json(&w.e, at),
                "open_at_end": open,
                "opened": total,
            }));
        }
    }
    json!({
        "worlds": worlds.iter().map(|w| json!({ "seed": w.seed, "size": size_name(w.size), "scenario": w.scenario, "home_max": w.home_max })).collect::<Vec<_>>(),
        "max_open_per_jurisdiction": content.policy_family_params.max_open_per_jurisdiction,
        "families": rows,
        "per_jurisdiction": per_jurisdiction,
        "never_bind": never,
        "never_opened": never_opened,
        "pass": never.is_empty(),
    })
}

fn tier_name(t: TierKey) -> &'static str {
    match t {
        TierKey::Local => "local",
        TierKey::Regional => "regional",
        TierKey::National => "national",
    }
}

/// What a family binds on, in short (its required slots' kinds).
fn binds_on(f: &PolicyFamilyDef) -> Vec<String> {
    f.slots
        .iter()
        .filter(|s| !s.optional)
        .map(|s| format!("{}:{}", s.id, s.bind.kind().label()))
        .collect()
}

/// `policies --coverage`: `seeds` worlds from `first`, `months` each.
pub fn coverage(
    content: &Arc<ContentBundle>,
    first: u64,
    seeds: u64,
    size: WorldSize,
    scenario: Option<&str>,
    months: u32,
) -> Result<Value, String> {
    let list: Vec<u64> = (first..first + seeds).collect();
    let mut worlds: Vec<World> = par_map(&list, threads(), |s| {
        play(content, *s, size, scenario, months)
    })
    .into_iter()
    .collect::<Result<_, _>>()?;
    let mut v = coverage_of(content, &mut worlds);
    v["seeds"] = json!(seeds);
    v["first_seed"] = json!(first);
    v["months"] = json!(months);
    Ok(v)
}

/// `policies --check`: validation item 11 on the canonical worlds (the five
/// scenarios at two sizes, 36 months, seed 1).
pub fn check(content: &Arc<ContentBundle>, seed: u64, months: u32) -> Result<Value, String> {
    let mut cases: Vec<(WorldSize, String)> = Vec::new();
    for size in [WorldSize::Small, WorldSize::Medium] {
        for s in &content.scenarios {
            cases.push((size, s.id.clone()));
        }
    }
    let mut worlds: Vec<World> = par_map(&cases, threads(), |(size, sc)| {
        play(content, seed, *size, Some(sc), months)
    })
    .into_iter()
    .collect::<Result<_, _>>()?;
    let mut v = coverage_of(content, &mut worlds);
    v["check"] = json!("every family binds in at least one canonical test world (validation item 11)");
    v["months"] = json!(months);
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content() -> Arc<ContentBundle> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        crate::lab::load(&root, &[]).unwrap_or_else(|e| panic!("{e}"))
    }

    /// The listing is deterministic and names every proposal's bindings.
    #[test]
    fn a_listing_names_its_proposals_and_repeats() {
        let c = content();
        let a = listing(&c, 3, WorldSize::Small, None, None, 6).unwrap_or_else(|e| panic!("{e}"));
        let b = listing(&c, 3, WorldSize::Small, None, None, 6).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(a, b);
        assert_eq!(
            a["families"].as_array().map(|v| v.len()),
            Some(c.policy_families.len())
        );
        for i in a["instances"].as_array().into_iter().flatten() {
            assert!(i["bindings"].as_array().is_some_and(|b| !b.is_empty()));
        }
    }

    /// `--set` reaches the family parameters: with one proposal allowed
    /// open at a time, no jurisdiction ever holds two.
    #[test]
    fn the_cap_on_open_proposals_is_a_set_path() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
        let sets = crate::lab::parse_sets(&["policy_family_params.max_open_per_jurisdiction=1".into()])
            .unwrap_or_else(|e| panic!("{e}"));
        let c = crate::lab::load(&root, &sets).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(c.policy_family_params.max_open_per_jurisdiction, 1);
        let w = play(&c, 2, WorldSize::Small, None, 6).unwrap_or_else(|e| panic!("{e}"));
        let st = w.e.state();
        let home = home(&w.e).unwrap_or_else(|| panic!("no home"));
        assert!(st.policy.families.open_at(home).count() <= 1);
    }
}
