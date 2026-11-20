//! The lookup seam (M6_ENGINE_CONTRACTS §2.10, task F2a): every reader of a
//! bill's or law's rules asks [`resolve`], never the template registry
//! directly. A standard bill resolves to its template as it is; a policy
//! family's bill (DEC-092) composes its family's base and chosen options on
//! the fly (pure; no cache: it runs only where templates are read).

use crate::policy::families::{Bound, FamilyInstance};
use crate::policy::{PolicyInstance, PolicyState};
use mandate_content::policy_family::{FamilyOptionDef, PolicyFamilyDef, slot_ref};
use mandate_content::{ContentBundle, CostDef, HazardRef, MechanismDef, ParamDef, PolicyTemplate};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::ops::Deref;

/// A bill's rules: a template (borrowed for a standard bill, composed for a
/// family bill) plus what composition adds. It derefs to the template, so
/// readers use it as one.
#[derive(Clone, Debug)]
pub struct Resolved<'a> {
    template: Cow<'a, PolicyTemplate>,
    /// Multiplies every mechanism's effect (a family option's
    /// `effect_factor`; 1 for a standard bill).
    pub effect_factor: f64,
    /// Fluent arguments for its keys (a family's slots; empty for a
    /// standard bill).
    pub args: BTreeMap<String, String>,
}

impl<'a> Resolved<'a> {
    /// A standard template as it is.
    pub fn plain(t: &'a PolicyTemplate) -> Self {
        Resolved {
            template: Cow::Borrowed(t),
            effect_factor: 1.0,
            args: BTreeMap::new(),
        }
    }

    /// The template (composed for a family bill).
    pub fn template(&self) -> &PolicyTemplate {
        &self.template
    }
}

impl Deref for Resolved<'_> {
    type Target = PolicyTemplate;
    fn deref(&self) -> &PolicyTemplate {
        &self.template
    }
}

/// A bill's or law's rules: its template, or its family composed with the
/// choices it was drafted with. `None` when the content no longer has them.
pub fn resolve<'a>(
    content: &'a ContentBundle,
    st: &PolicyState,
    p: &PolicyInstance,
) -> Option<Resolved<'a>> {
    match &p.family {
        None => content.policies.get(&p.template).map(Resolved::plain),
        Some(d) => {
            let f = content.family(&p.template)?;
            let inst = st.families.get(d.instance);
            let choices: BTreeMap<String, String> = d.choices.iter().cloned().collect();
            compose(content, f, inst, &choices)
        }
    }
}

/// The chosen option of every axis: `choices`, else the axis default.
/// `None` if a choice names an option the axis lacks.
pub fn chosen<'f>(
    f: &'f PolicyFamilyDef,
    choices: &BTreeMap<String, String>,
) -> Option<Vec<(&'f str, &'f FamilyOptionDef)>> {
    f.axes
        .iter()
        .map(|a| {
            let id = choices
                .get(&a.id)
                .map(|s| s.as_str())
                .or_else(|| a.default_option())?;
            a.option(id).map(|o| (a.id.as_str(), o))
        })
        .collect()
}

/// Compose a family's base with one option per axis (`choices`, missing
/// axes at their defaults; §2.5): the template a family bill runs on.
/// Slot references (`@slot`) are replaced by what `inst` bound; without an
/// instance (a preview of the format) they stay and do nothing.
pub fn compose<'a>(
    content: &'a ContentBundle,
    f: &PolicyFamilyDef,
    inst: Option<&FamilyInstance>,
    choices: &BTreeMap<String, String>,
) -> Option<Resolved<'a>> {
    let picked = chosen(f, choices)?;
    let b = &f.base;
    let mut params: Vec<ParamDef> = b.params.clone();
    let mut cost = b.cost.clone();
    let mut cost_factor = 1.0;
    let mut effect_factor = 1.0;
    let mut months = b.implementation_months as i32;
    let mut mechanisms: Vec<MechanismDef> = b.mechanisms.clone();
    let mut side_effects = b.side_effects.clone();
    let mut signature = b.signature.clone();
    let mut secondary_keys = b.secondary_keys.clone();
    for (_, o) in &picked {
        params.extend(o.params.iter().cloned());
        for (k, v) in &o.fixed {
            params.push(ParamDef {
                id: k.clone(),
                name_key: String::new(),
                min: *v,
                max: *v,
                default: *v,
                step: 1.0,
                unit_key: String::new(),
                hidden: true,
            });
        }
        if let Some(c) = &o.cost {
            add_cost(&mut cost, c);
        }
        cost_factor *= o.cost_factor;
        effect_factor *= o.effect_factor;
        months += o.months as i32;
        mechanisms.extend(o.mechanisms.iter().cloned());
        side_effects.extend(o.side_effects.iter().cloned());
        for (k, v) in &o.signature {
            let e = signature.entry(k.clone()).or_insert(0.0);
            *e = (*e + v).clamp(-1.0, 1.0);
        }
        secondary_keys.extend(o.secondary_keys.iter().cloned());
    }
    cost.per_capita *= cost_factor;
    cost.per_capita_per_unit *= cost_factor;
    cost.per_unit *= cost_factor;
    if let Some(inst) = inst {
        for m in mechanisms.iter_mut() {
            bind_mechanism(m, inst);
        }
    }
    let signature_weights = content
        .attitudes
        .iter()
        .map(|a| signature.get(&a.key).copied().unwrap_or(0.0))
        .collect();
    let t = PolicyTemplate {
        id: f.id.clone(),
        name_key: f.name_key.clone(),
        desc_key: f.desc_key.clone(),
        tier: f.tier,
        category: f.category,
        params,
        cost,
        mechanisms,
        implementation_months: months.max(1) as u16,
        signature,
        signature_weights,
        secondary_keys,
        side_effects,
        requires: f.requires,
        unlock: None,
        retired: false,
    };
    Some(Resolved {
        template: Cow::Owned(t),
        effect_factor,
        args: inst.map(|i| i.args.clone()).unwrap_or_default(),
    })
}

/// An option's cost added to the base's (validation keeps their parameter
/// the same, or the option's without one).
fn add_cost(c: &mut CostDef, o: &CostDef) {
    c.per_capita += o.per_capita;
    c.per_capita_per_unit += o.per_capita_per_unit;
    c.per_unit += o.per_unit;
    if c.param.is_none() {
        c.param = o.param.clone();
    }
}

/// Replace `@slot` references with what the instance bound.
fn bind_mechanism(m: &mut MechanismDef, inst: &FamilyInstance) {
    let sector = |s: &mut String| {
        if let Some(slot) = slot_ref(s)
            && let Some(Bound::Sector { sector }) = inst.bound(slot)
        {
            *s = sector.clone();
        }
    };
    match m {
        MechanismDef::SectorSubsidy { sector: s, .. }
        | MechanismDef::SectorCapacity { sector: s, .. } => sector(s),
        MechanismDef::HazardProtection { hazard, .. } => {
            if let HazardRef::Slot(s) = hazard
                && let Some(slot) = slot_ref(s)
                && let Some(Bound::Hazard { hazard: h }) = inst.bound(slot)
            {
                *hazard = HazardRef::Key(*h);
            }
        }
        _ => {}
    }
}
