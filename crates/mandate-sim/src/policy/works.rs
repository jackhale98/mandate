//! Works you can see (DEC-099; F2b): a physical work a policy family's bill
//! builds at a bound place or neighbourhood. It is **approved** when the bill
//! is enacted, **building** while funded, **open** when done, and **ageing**
//! after `works.ageing_after_months`. Building **stalls** when funding stops
//! for `works.stall_unfunded_months`, when the law is repealed or ends, or
//! when the contractor fails (a keyed draw per work and month); a stalled site
//! loses condition every month and is **abandoned** after
//! `works.abandon_after_months`. A funding stall resumes if the money comes
//! back before the site is abandoned; any other stall waits for a decision
//! (resume at extra cost, or clear; a later task).
//!
//! The site is keyed to what the family bound (a place, else a
//! neighbourhood), so nothing ever moves. The art and street layers read
//! [`by_hood`]: each work's kind, stage and condition.

use crate::policy::{PolicyInstanceId, PolicyState, PolicyStatus};
use crate::politics::PoliticianId;
use crate::scheduler::{Outs, Reads};
use crate::world::{Jurisdiction, NeighborhoodId, PlaceId};
use mandate_core::{IdAllocator, SimDate, Stream, define_id};
use serde::{Deserialize, Serialize};

define_id!(
    /// A works project (DEC-099).
    WorksId
);

/// Keyed-draw tag for contractor failure (`Stream::Policy`).
pub const TAG_CONTRACTOR: u64 = 0xC0_7AC7;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorksState {
    pub ids: IdAllocator,
    /// Indexed by `WorksId`.
    pub projects: Vec<WorksProject>,
}

/// One physical work.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorksProject {
    pub id: WorksId,
    /// The kind the art and street layers draw (`flood_wall`, …).
    pub kind: String,
    /// The bill or law that builds it.
    pub policy: PolicyInstanceId,
    /// Its family (content id).
    pub family: String,
    /// The jurisdiction it belongs to (DEC-098).
    pub at: Jurisdiction,
    pub site: WorksSite,
    /// The neighbourhood it stands in.
    pub hood: NeighborhoodId,
    /// Who sponsored the bill (credit and blame, V1).
    pub sponsor: Option<PoliticianId>,
    pub approved: SimDate,
    /// Months of building at full funding.
    pub months: u16,
    /// Monthly chance of a contractor failure while building.
    pub contractor_risk: f32,
    /// 0..1 built.
    pub progress: f32,
    /// 0..1: 1 is sound; stalled and ageing works lose it.
    pub condition: f32,
    pub stage: WorksStage,
    /// Consecutive months without funding while building.
    pub unfunded_months: u16,
}

/// Where a work stands: a bound place, or a neighbourhood.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorksSite {
    Place { place: PlaceId },
    Neighborhood { hood: NeighborhoodId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorksStage {
    /// Enacted, not yet funded.
    Approved,
    Building,
    Open {
        since: SimDate,
    },
    Ageing {
        since: SimDate,
    },
    /// Half-built and idle.
    Stalled {
        since: SimDate,
        cause: StallCause,
    },
    /// Stalled so long it is given up: fencing down, weeds up.
    Abandoned {
        since: SimDate,
        cause: StallCause,
    },
}

impl WorksStage {
    /// Snake-case key for views and Fluent (`works-stage-<key>`).
    pub fn key(&self) -> &'static str {
        match self {
            WorksStage::Approved => "approved",
            WorksStage::Building => "building",
            WorksStage::Open { .. } => "open",
            WorksStage::Ageing { .. } => "ageing",
            WorksStage::Stalled { .. } => "stalled",
            WorksStage::Abandoned { .. } => "abandoned",
        }
    }
    pub fn is_stalled(&self) -> bool {
        matches!(
            self,
            WorksStage::Stalled { .. } | WorksStage::Abandoned { .. }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StallCause {
    /// Funding was cut, or the budget ran dry.
    FundingCut,
    /// The law was repealed, or ended.
    Repealed,
    /// A successor cancelled it.
    Cancelled,
    /// The contractor failed.
    ContractorFailed,
}

/// The read API for the art and street layers: one work as they draw it.
#[derive(Clone, Debug, PartialEq)]
pub struct WorksSummary {
    pub id: WorksId,
    pub kind: String,
    pub site: WorksSite,
    /// `approved`, `building`, `open`, `ageing`, `stalled`, `abandoned`.
    pub stage: &'static str,
    pub progress: f32,
    pub condition: f32,
}

/// Every work in a neighbourhood, in id order.
pub fn in_hood(st: &PolicyState, hood: NeighborhoodId) -> Vec<WorksSummary> {
    st.works
        .projects
        .iter()
        .filter(|w| w.hood == hood)
        .map(summary)
        .collect()
}

/// Every work, by neighbourhood (ascending), each in id order.
pub fn by_hood(st: &PolicyState) -> std::collections::BTreeMap<NeighborhoodId, Vec<WorksSummary>> {
    let mut out: std::collections::BTreeMap<NeighborhoodId, Vec<WorksSummary>> = Default::default();
    for w in &st.works.projects {
        out.entry(w.hood).or_default().push(summary(w));
    }
    out
}

fn summary(w: &WorksProject) -> WorksSummary {
    WorksSummary {
        id: w.id,
        kind: w.kind.clone(),
        site: w.site,
        stage: w.stage.key(),
        progress: w.progress,
        condition: w.condition,
    }
}

/// Monthly (Policy implementation phase, after `policy.implementation`):
/// approve works for newly enacted family bills, then build, stall, open,
/// age and decay.
pub fn monthly(st: &mut PolicyState, r: &Reads<'_>, _out: &mut Outs<'_>) {
    approve(st, r);
    let fin = r.finance();
    let wp = &r.content.policy_family_params.works;
    let today = r.date;
    for i in 0..st.works.projects.len() {
        let (policy, id) = (st.works.projects[i].policy, st.works.projects[i].id);
        let law = st.get(policy).map(|p| (p.status, p.end_reason));
        let funded = fin.funded_share(policy.0);
        let w = &mut st.works.projects[i];
        let ended = !matches!(
            law.map(|(s, _)| s),
            Some(PolicyStatus::Enacted | PolicyStatus::InEffect)
        );
        match w.stage {
            WorksStage::Approved | WorksStage::Building => {
                if ended {
                    w.stage = WorksStage::Stalled {
                        since: today,
                        cause: StallCause::Repealed,
                    };
                    continue;
                }
                if funded <= 0.0 {
                    w.unfunded_months = w.unfunded_months.saturating_add(1);
                    if w.stage == WorksStage::Building
                        && w.unfunded_months >= wp.stall_unfunded_months
                    {
                        w.stage = WorksStage::Stalled {
                            since: today,
                            cause: StallCause::FundingCut,
                        };
                    }
                    continue;
                }
                w.unfunded_months = 0;
                w.stage = WorksStage::Building;
                // The contractor may fail (keyed per work and month).
                if w.contractor_risk > 0.0
                    && mandate_core::rng::keyed(
                        r.meta.seed,
                        Stream::Policy,
                        &[TAG_CONTRACTOR, id.0 as u64, today.0 as u64],
                    )
                    .chance(w.contractor_risk as f64)
                {
                    w.stage = WorksStage::Stalled {
                        since: today,
                        cause: StallCause::ContractorFailed,
                    };
                    continue;
                }
                w.progress = (w.progress + (funded / w.months.max(1) as f64) as f32).min(1.0);
                if w.progress >= 1.0 {
                    w.stage = WorksStage::Open { since: today };
                }
            }
            WorksStage::Open { since } => {
                if since.days_until(today) / 30 >= wp.ageing_after_months as u32 {
                    w.stage = WorksStage::Ageing { since: today };
                }
            }
            WorksStage::Ageing { .. } => {
                w.condition = (w.condition - wp.wear_per_month as f32).max(0.0);
            }
            WorksStage::Stalled { since, cause } => {
                w.condition = (w.condition - wp.decay_per_month as f32).max(0.0);
                if cause == StallCause::FundingCut && !ended && funded > 0.0 {
                    // The money came back in time: building resumes.
                    w.stage = WorksStage::Building;
                    w.unfunded_months = 0;
                } else if since.days_until(today) / 30 >= wp.abandon_after_months as u32 {
                    w.stage = WorksStage::Abandoned {
                        since: today,
                        cause,
                    };
                }
            }
            WorksStage::Abandoned { .. } => {
                w.condition = (w.condition - wp.decay_per_month as f32).max(0.0);
            }
        }
    }
}

/// Approve a work for each enacted family bill that declares one and has none.
fn approve(st: &mut PolicyState, r: &Reads<'_>) {
    let content = r.content;
    let mut new = Vec::new();
    for p in &st.instances {
        if !matches!(p.status, PolicyStatus::Enacted | PolicyStatus::InEffect) {
            continue;
        }
        let Some(d) = &p.family else {
            continue;
        };
        if st.works.projects.iter().any(|w| w.policy == p.id) {
            continue;
        }
        let Some(f) = content.family(&p.template) else {
            continue;
        };
        let Some(inst) = st.families.get(d.instance) else {
            continue;
        };
        // The chosen option's works, else the base's.
        let def = f
            .axes
            .iter()
            .filter_map(|a| d.choice(&a.id).and_then(|o| a.option(o)))
            .find_map(|o| o.works.as_ref())
            .or(f.base.works.as_ref());
        let Some(def) = def else {
            continue;
        };
        let (site, hood) = match inst.bound(&def.at) {
            Some(super::families::Bound::Place { place }) => {
                let Some(h) = r.world.places.get(place.index()).map(|p| p.neighborhood) else {
                    continue;
                };
                (WorksSite::Place { place: *place }, h)
            }
            Some(super::families::Bound::Neighborhood { hood }) => {
                (WorksSite::Neighborhood { hood: *hood }, *hood)
            }
            _ => continue,
        };
        let months = def.months.unwrap_or_else(|| {
            crate::policy::resolve(content, st, p)
                .map(|t| t.implementation_months)
                .unwrap_or(12)
        });
        new.push((
            p.id,
            f.id.clone(),
            def.kind.clone(),
            inst.at,
            site,
            hood,
            p.sponsor,
            months,
            def.contractor_risk,
        ));
    }
    for (policy, family, kind, at, site, hood, sponsor, months, risk) in new {
        let id: WorksId = st.works.ids.alloc();
        st.works.projects.push(WorksProject {
            id,
            kind,
            policy,
            family,
            at,
            site,
            hood,
            sponsor,
            approved: r.date,
            months: months.max(1),
            contractor_risk: risk as f32,
            progress: 0.0,
            condition: 1.0,
            stage: WorksStage::Approved,
            unfunded_months: 0,
        });
    }
}
