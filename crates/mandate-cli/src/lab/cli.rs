//! `mandate-cli lab run|twin`: the experiment runner from the command line.
//!
//! ```text
//! lab run  --seed 4 --scenario industrial_decline --driver idle --months 60 --sample 0,12,60
//! lab twin --seed 4 --at 1 --policy policy.affordable_housing@home --months 60 --sample 12,60
//! lab twin --seed 4 --at 6 --closure home:0.3 --months 36 --sample 12,36 --hoods
//! ```
//! Interventions (`twin`, or `run` with `--at`): `--policy <template>@<town|home>[,param=v…]`,
//! `--closure <town|home>:<share>`, `--disaster <town|home>:<hazard>:<severity>`,
//! `--recession <depth>:<months>`, `--shock <town|home>:<sector>:<factor>:<months>`,
//! `--set-at <path>=<value>` (repeatable; one intervention). With several,
//! `twin` branches once per intervention.

use super::runner::{self, Driver, Intervention, Plan, Setup};
use mandate_content::ContentBundle;
use mandate_worldgen::WorldSize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(clap::Args, Clone)]
pub struct LabCmd {
    /// `run` or `twin`.
    pub mode: String,
    #[arg(long, default_value_t = 1)]
    pub seed: u64,
    /// Scenario id (`industrial_decline`); omitted = a random country.
    #[arg(long)]
    pub scenario: Option<String>,
    /// `none` (days, no turns), `idle`, or a bot strategy.
    #[arg(long, default_value = "idle")]
    pub driver: String,
    #[arg(long, default_value_t = 60)]
    pub months: u32,
    /// Months to sample, comma-separated (default: every 12 and the end).
    #[arg(long)]
    pub sample: Option<String>,
    /// The month the intervention happens.
    #[arg(long, default_value_t = 1)]
    pub at: u32,
    #[arg(long)]
    pub policy: Vec<String>,
    #[arg(long)]
    pub closure: Vec<String>,
    #[arg(long)]
    pub disaster: Vec<String>,
    #[arg(long)]
    pub recession: Vec<String>,
    #[arg(long)]
    pub shock: Vec<String>,
    #[arg(long = "set-at")]
    pub set_at: Vec<String>,
    /// Include the home town's neighbourhoods.
    #[arg(long)]
    pub hoods: bool,
    /// `twin`: print both runs in full, not only the difference.
    #[arg(long)]
    pub full: bool,
}

fn town(s: &str, home: u32) -> Result<u32, String> {
    if s == "home" {
        Ok(home)
    } else {
        s.parse().map_err(|_| format!("bad town `{s}`"))
    }
}

fn num(s: &str) -> Result<f64, String> {
    s.parse().map_err(|_| format!("bad number `{s}`"))
}

fn interventions(c: &LabCmd, home: u32) -> Result<Vec<Intervention>, String> {
    let mut v = Vec::new();
    for p in &c.policy {
        let (head, rest) = p.split_once(',').unwrap_or((p, ""));
        let (tpl, at) = head.split_once('@').unwrap_or((head, "home"));
        let mut params = BTreeMap::new();
        for kv in rest.split(',').filter(|x| !x.is_empty()) {
            let (k, val) = kv.split_once('=').ok_or(format!("bad param `{kv}`"))?;
            params.insert(k.to_string(), num(val)?);
        }
        v.push(Intervention::Policy {
            template: tpl.to_string(),
            town: town(at, home)?,
            params,
        });
    }
    for x in &c.closure {
        let (t, share) = x.split_once(':').ok_or("--closure <town>:<share>")?;
        v.push(Intervention::PlantClosure {
            town: town(t, home)?,
            sector: None,
            share: num(share)?,
        });
    }
    for x in &c.disaster {
        let p: Vec<&str> = x.split(':').collect();
        if p.len() != 3 {
            return Err("--disaster <town>:<hazard>:<severity>".into());
        }
        v.push(Intervention::Disaster {
            town: town(p[0], home)?,
            hazard: p[1].to_string(),
            severity: num(p[2])?,
        });
    }
    for x in &c.recession {
        let (d, m) = x.split_once(':').ok_or("--recession <depth>:<months>")?;
        v.push(Intervention::Recession {
            depth: num(d)?,
            months: num(m)? as u16,
        });
    }
    for x in &c.shock {
        let p: Vec<&str> = x.split(':').collect();
        if p.len() != 4 {
            return Err("--shock <town>:<sector>:<factor>:<months>".into());
        }
        v.push(Intervention::SectorShock {
            town: town(p[0], home)?,
            sector: p[1].to_string(),
            factor: num(p[2])?,
            months: num(p[3])? as u16,
        });
    }
    if !c.set_at.is_empty() {
        let sets = c
            .set_at
            .iter()
            .map(|s| {
                let (k, val) = s.split_once('=').ok_or(format!("bad --set-at `{s}`"))?;
                let val: Value =
                    serde_json::from_str(val).unwrap_or_else(|_| Value::String(val.to_string()));
                Ok((k.to_string(), val))
            })
            .collect::<Result<Vec<_>, String>>()?;
        v.push(Intervention::Set { sets });
    }
    Ok(v)
}

pub fn run(content: Arc<ContentBundle>, c: LabCmd) -> Result<Value, String> {
    let scenario = c.scenario.as_ref().map(|s| {
        if s.contains('.') {
            s.clone()
        } else {
            format!("scenario.{s}")
        }
    });
    let samples: Vec<u32> = match &c.sample {
        Some(s) => s
            .split(',')
            .map(|x| x.trim().parse().map_err(|_| format!("bad month `{x}`")))
            .collect::<Result<_, String>>()?,
        None => (0..=c.months)
            .filter(|m| m % 12 == 0 || *m == c.months)
            .collect(),
    };
    let plan = Plan {
        setup: Setup {
            seed: c.seed,
            size: WorldSize::Small,
            scenario,
        },
        driver: Driver::parse(&c.driver),
        months: c.months,
        samples,
        home_hoods: c.hoods,
    };
    let home = runner::start(&content, &plan)?.home();
    let ivs = interventions(&c, home)?;
    match c.mode.as_str() {
        "run" => {
            let iv = ivs.into_iter().next().map(|x| (c.at, x));
            Ok(serde_json::to_value(runner::run(&content, &plan, iv)?).unwrap_or_default())
        }
        "twin" => {
            let t = runner::twins(&content, &plan, c.at, &ivs)?;
            if c.full {
                return Ok(serde_json::to_value(&t).unwrap_or_default());
            }
            Ok(json!({
                "plan": plan,
                "home": home,
                "base_hash": t.base.state_hash,
                "branches": t.treated.iter().zip(&t.diff).map(|(r, d)| json!({
                    "intervention": r.intervention,
                    "state_hash": r.state_hash,
                    "diff": d.iter().map(|(m, x)| json!({ "month": m, "changed": x })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            }))
        }
        m => Err(format!("lab mode `{m}`: use run or twin")),
    }
}
