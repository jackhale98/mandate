//! Drafting a policy family's proposal (DEC-092; §2.6, §2.11): the
//! checks every drafter shares (choices, sponsor presets, parameters, the
//! area), the bill it makes, the sponsor's price and goodwill, amendments of
//! choices, and the moments that teach the player about a proposal
//! (knowledge hooks, §2.7, DEC-091).

use crate::characters::{CharacterId, MemoryKind};
use crate::player::knowledge::{self, Level, Source, Subject};
use crate::policy::families::{Bound, FamilyDraft, FamilyInstance, FamilyInstanceId, area_hoods};
use crate::policy::{Clause, PolicyInstance, PolicyInstanceId, PolicyStatus};
use crate::politics::PoliticianId;
use crate::state::State;
use crate::world::{Jurisdiction, NeighborhoodId};
use mandate_content::policy_family::{AreaDef, HookDef, PolicyFamilyDef, PriceDef};
use mandate_content::{ContentBundle, ContentKey, TierKey};
use mandate_core::math::clamp;
use std::collections::BTreeMap;

/// A checked draft, ready to become a bill.
#[derive(Clone, Debug, PartialEq)]
pub struct Prepared {
    pub family: ContentKey,
    pub at: Jurisdiction,
    pub params: BTreeMap<String, f64>,
    pub clauses: Vec<Clause>,
    pub draft: FamilyDraft,
}

/// Check a draft of an open proposal: every choice names a real option of
/// its axis, none is excluded by another or needs what the jurisdiction
/// lacks (M7), a sponsor's version keeps its preset, and parameters are in
/// range. Missing choices take the axis default (a sponsor's: its preset).
pub fn prepare(
    state: &State,
    content: &ContentBundle,
    proposal: FamilyInstanceId,
    choices: &BTreeMap<String, String>,
    params: &BTreeMap<String, f64>,
    sponsor: Option<&str>,
) -> Result<Prepared, String> {
    prepare_with(state, content, proposal, choices, params, sponsor, true)
}

fn prepare_with(
    state: &State,
    content: &ContentBundle,
    proposal: FamilyInstanceId,
    choices: &BTreeMap<String, String>,
    params: &BTreeMap<String, f64>,
    sponsor: Option<&str>,
    open_only: bool,
) -> Result<Prepared, String> {
    let inst = state
        .policy
        .families
        .get(proposal)
        .ok_or("no such proposal")?;
    if open_only && !inst.is_open() {
        return Err("that proposal is no longer on the table".into());
    }
    let f = content
        .family(&inst.family)
        .filter(|f| !f.disabled)
        .ok_or("that proposal's rules no longer exist")?;
    for axis in choices.keys() {
        if f.axis(axis).is_none() {
            return Err(format!("no choice `{axis}` in this proposal"));
        }
    }
    let preset = match sponsor {
        Some(s) => {
            let def = f.sponsor(s).ok_or("no such sponsor")?;
            if inst.sponsor(s).is_none() {
                return Err("that sponsor is not part of this proposal".into());
            }
            Some(def)
        }
        None => None,
    };
    // One option per axis: the choice, else the sponsor's preset, else the default.
    let mut picked: Vec<(String, String)> = Vec::new();
    for a in &f.axes {
        let id = choices
            .get(&a.id)
            .or_else(|| preset.and_then(|p| p.preset.get(&a.id)))
            .cloned()
            .or_else(|| a.default_option().map(str::to_string))
            .ok_or("an axis has no options")?;
        let o = a
            .option(&id)
            .ok_or_else(|| format!("`{}` has no option `{id}`", a.id))?;
        if o.needs.is_some() {
            return Err(format!(
                "`{}.{id}` needs powers this jurisdiction lacks",
                a.id
            ));
        }
        picked.push((a.id.clone(), id));
    }
    let keys: Vec<String> = picked.iter().map(|(a, o)| format!("{a}.{o}")).collect();
    for (a, o) in &picked {
        let opt = f.axis(a).and_then(|x| x.option(o)).ok_or("bad option")?;
        if let Some(x) = opt.excludes.iter().find(|x| keys.contains(x)) {
            return Err(format!("`{a}.{o}` cannot be combined with `{x}`"));
        }
    }
    if let Some(p) = preset {
        for a in &f.axes {
            let want = p
                .preset
                .get(&a.id)
                .map(String::as_str)
                .or_else(|| a.default_option());
            let got = picked
                .iter()
                .find(|(x, _)| x == &a.id)
                .map(|(_, o)| o.as_str());
            if want != got {
                return Err("a sponsor's version keeps its choices; amend it once drafted".into());
            }
        }
    }
    let chosen: BTreeMap<String, String> = picked.iter().cloned().collect();
    let t = crate::policy::resolve::compose(content, f, Some(inst), &chosen)
        .ok_or("the choices do not compose")?;
    // Sliders: given, else the sponsor's, else the default; hidden values fixed.
    let mut out = BTreeMap::new();
    for prm in &t.params {
        if prm.hidden {
            out.insert(prm.id.clone(), prm.default);
            continue;
        }
        let v = params
            .get(&prm.id)
            .or_else(|| preset.and_then(|p| p.params.get(&prm.id)))
            .copied()
            .unwrap_or(prm.default);
        if !v.is_finite() || v < prm.min || v > prm.max {
            return Err(format!("parameter {} out of range", prm.id));
        }
        let snapped = prm.min + ((v - prm.min) / prm.step).round() * prm.step;
        out.insert(prm.id.clone(), clamp(snapped, prm.min, prm.max));
    }
    let mut clauses = Vec::new();
    if let Some(hoods) = area_of(state, f, inst, &chosen) {
        clauses.push(Clause::Area {
            neighborhoods: hoods,
        });
    }
    Ok(Prepared {
        family: f.id.clone(),
        at: inst.at,
        params: out,
        clauses,
        draft: FamilyDraft {
            instance: inst.id,
            choices: picked,
            sponsor: sponsor.map(str::to_string),
            changed_axes: 0,
        },
    })
}

/// The neighbourhoods the chosen options' area covers (`None`: everywhere).
fn area_of(
    state: &State,
    f: &PolicyFamilyDef,
    inst: &FamilyInstance,
    chosen: &BTreeMap<String, String>,
) -> Option<Vec<NeighborhoodId>> {
    let area: &AreaDef = f
        .axes
        .iter()
        .filter_map(|a| a.option(chosen.get(&a.id)?))
        .find_map(|o| o.area.as_ref())?;
    area_hoods(&state.world, inst, area)
}

/// Make the bill (a draft, or for an NPC a bill under consideration) and
/// record it on the proposal. Returns its id.
pub fn make_bill(
    state: &mut State,
    content: &ContentBundle,
    p: Prepared,
    sponsor: PoliticianId,
    status: PolicyStatus,
    extra: Vec<Clause>,
) -> PolicyInstanceId {
    let id: PolicyInstanceId = state.policy.ids.alloc();
    let tier = content
        .family(&p.family)
        .map(|f| f.tier)
        .unwrap_or(TierKey::Local);
    let proposal = p.draft.instance;
    let mut clauses = p.clauses;
    clauses.extend(extra);
    state.policy.instances.push(PolicyInstance {
        id,
        template: p.family,
        tier,
        jurisdiction: p.at.raw(),
        sponsor: Some(sponsor),
        params: p.params,
        clauses,
        status,
        stage: 0,
        next_stage_on: None,
        votes: Vec::new(),
        introduced_on: None,
        enacted_on: None,
        progress: 0.0,
        awareness: 0.0,
        end_reason: None,
        cause: None,
        ledger: BTreeMap::new(),
        family: Some(p.draft),
    });
    if let Some(i) = state.policy.families.instances.get_mut(proposal.index()) {
        i.drafts.push(id);
    }
    id
}

/// The player adopts a sponsor's version (§2.6): the price, and the
/// sponsor's goodwill (`adopt_opinion`, remembered as standing with them).
pub fn adopt(state: &mut State, content: &ContentBundle, bill: PolicyInstanceId) {
    let Some((sponsor_id, proposal)) = state
        .policy
        .get(bill)
        .and_then(|p| p.family.as_ref())
        .and_then(|d| Some((d.sponsor.clone()?, d.instance)))
    else {
        return;
    };
    let Some(inst) = state.policy.families.get(proposal).cloned() else {
        return;
    };
    let Some(f) = content.family(&inst.family) else {
        return;
    };
    let (Some(def), Some(person), Some(me)) = (
        f.sponsor(&sponsor_id),
        inst.sponsor(&sponsor_id),
        crate::characters::player_character(state),
    ) else {
        return;
    };
    let date = state.calendar.date;
    let subject = Some(crate::characters::Subject::Policy { id: bill.0 });
    crate::characters::remember(
        &mut state.politics,
        content,
        date,
        person,
        me,
        MemoryKind::StoodWithMe,
        subject.clone(),
        Some(content.policy_family_params.adopt_opinion),
    );
    match &def.price {
        PriceDef::Favour { count, context_key } => {
            for _ in 0..*count {
                crate::characters::owe(
                    &mut state.politics,
                    me,
                    person,
                    context_key,
                    subject.clone(),
                    date,
                );
            }
        }
        PriceDef::Endorsement {
            classes,
            area,
            approval,
            backlash,
        } => endorse(state, &inst, classes, area.as_ref(), *approval, *backlash),
    }
}

/// A public endorsement: approval of the player rises among the sponsor's
/// people (classes, or residents of the area) and falls a little among
/// everyone else in the jurisdiction (room-scaled, like every approval change).
fn endorse(
    state: &mut State,
    inst: &FamilyInstance,
    classes: &[mandate_content::ClassKey],
    area: Option<&AreaDef>,
    approval: f64,
    backlash: f64,
) {
    let hoods = inst.at.hoods(&state.world);
    let area: Option<Vec<NeighborhoodId>> = area.and_then(|a| area_hoods(&state.world, inst, a));
    for h in hoods {
        let in_area = area
            .as_ref()
            .is_none_or(|v| v.iter().any(|x| x.index() == h));
        let Some(hp) = state.population.hoods.get_mut(h) else {
            continue;
        };
        for (ci, c) in hp.cohorts.iter_mut().enumerate() {
            let class = crate::population::class_of(ci);
            let ours = in_area
                && (classes.is_empty()
                    || classes
                        .iter()
                        .any(|k| crate::policy::class_index(*k) == class));
            let delta = if ours { approval } else { -backlash };
            let room = 1.0 - (c.player_approval as f64).abs();
            c.player_approval = clamp(c.player_approval as f64 + delta * room, -1.0, 1.0) as f32;
        }
    }
}

/// The player amends a family bill's choices (and sliders): each axis newly
/// moved away from the adopted sponsor's preset, and each slider moved more
/// than `free_steps` from theirs, costs `amend_opinion` with the sponsor
/// (remembered as pressing them).
/// Whether an amendment of a family bill's choices and sliders is valid
/// (nothing changes).
pub fn check_amend(
    state: &State,
    content: &ContentBundle,
    bill: PolicyInstanceId,
    choices: &BTreeMap<String, String>,
    params: &BTreeMap<String, f64>,
) -> Result<(), String> {
    amended(state, content, bill, choices, params).map(|_| ())
}

fn amended(
    state: &State,
    content: &ContentBundle,
    bill: PolicyInstanceId,
    choices: &BTreeMap<String, String>,
    params: &BTreeMap<String, f64>,
) -> Result<(FamilyDraft, Prepared), String> {
    let p = state.policy.get(bill).ok_or("no such bill")?;
    let d = p
        .family
        .clone()
        .ok_or("only a proposal's bill has choices")?;
    let mut merged: BTreeMap<String, String> = d.choices.iter().cloned().collect();
    for (a, o) in choices {
        merged.insert(a.clone(), o.clone());
    }
    // A proposal that lapsed since keeps its bill: its rules still apply.
    let prepared = prepare_with(state, content, d.instance, &merged, params, None, false)?;
    Ok((d, prepared))
}

/// Amend a family bill's choices and sliders (`check_amend` first).
pub fn amend_choices(
    state: &mut State,
    content: &ContentBundle,
    bill: PolicyInstanceId,
    choices: &BTreeMap<String, String>,
    params: &BTreeMap<String, f64>,
) -> Result<(), String> {
    let (d, prepared) = amended(state, content, bill, choices, params)?;
    let inst = state
        .policy
        .families
        .get(d.instance)
        .cloned()
        .ok_or("no such proposal")?;
    let f = content.family(&inst.family).ok_or("no such family")?;
    // What departs from the adopted sponsor's version.
    let mut changed = 0u8;
    if let Some(sid) = &d.sponsor
        && let Some(def) = f.sponsor(sid)
    {
        for (a, o) in &prepared.draft.choices {
            let want = def
                .preset
                .get(a)
                .map(String::as_str)
                .or_else(|| f.axis(a).and_then(|x| x.default_option()));
            if want != Some(o.as_str()) {
                changed = changed.saturating_add(1);
            }
        }
        let steps = content.policy_family_params.free_steps;
        for prm in f.all_params() {
            if let (Some(w), Some(g)) = (def.params.get(&prm.id), prepared.params.get(&prm.id))
                && (g - w).abs() > steps * prm.step + 1e-9
            {
                changed = changed.saturating_add(1);
            }
        }
        let new = changed.saturating_sub(d.changed_axes);
        if new > 0
            && let (Some(person), Some(me)) = (
                inst.sponsor(sid),
                crate::characters::player_character(state),
            )
        {
            let date = state.calendar.date;
            crate::characters::remember(
                &mut state.politics,
                content,
                date,
                person,
                me,
                MemoryKind::PressedMe,
                Some(crate::characters::Subject::Policy { id: bill.0 }),
                Some(content.policy_family_params.amend_opinion * new as f64),
            );
        }
    }
    let Some(p) = state.policy.get_mut(bill) else {
        return Err("no such bill".into());
    };
    p.params = prepared.params;
    // Keep the bill's other clauses; the area follows the choices.
    p.clauses.retain(|c| !matches!(c, Clause::Area { .. }));
    p.clauses.extend(prepared.clauses);
    if let Some(fd) = p.family.as_mut() {
        fd.choices = prepared.draft.choices;
        fd.changed_axes = fd.changed_axes.max(changed);
    }
    Ok(())
}

// --- Knowledge hooks (§2.7, DEC-091) ------------------------------------------------------

/// A moment that may teach the player about open proposals.
#[derive(Clone, Copy, Debug)]
pub enum Moment<'a> {
    /// The player is at a location (an arrival).
    At(crate::places::Location),
    /// The player met, talked with, lobbied or traded a favour with someone.
    Met(CharacterId),
    /// The player did an action in a neighbourhood or at a place.
    Did {
        action: &'a str,
        hood: Option<NeighborhoodId>,
    },
}

/// After a player command: the moments it made (arrivals, meetings,
/// actions), each checked against every open proposal's hooks.
pub fn learn_from(state: &mut State, content: &ContentBundle, cmd: &crate::actions::PlayerCommand) {
    use crate::actions::{Action, PlayerCommand as C};
    if state.policy.families.instances.is_empty() {
        return;
    }
    let mut moments: Vec<Moment<'_>> = Vec::new();
    let here = state.player.as_ref().map(|p| p.location);
    match cmd {
        C::MoveTo { .. } | C::Go { .. } => moments.extend(here.map(Moment::At)),
        C::Talk { to, .. } => {
            if let crate::talk::TalkTarget::Person { person } = to {
                moments.push(Moment::Met(*person));
            }
        }
        C::Act { action, .. } => match action {
            Action::Lobby { politician, .. } | Action::TradeFavor { politician, .. } => {
                if let Some(p) = state.politics.get(PoliticianId(*politician)) {
                    moments.push(Moment::Met(p.character));
                }
            }
            Action::Canvass { neighborhood, .. } => moments.push(Moment::Did {
                action: "canvass",
                hood: Some(NeighborhoodId(*neighborhood)),
            }),
            Action::TownHall { neighborhood, .. } => moments.push(Moment::Did {
                action: "town_hall",
                hood: Some(NeighborhoodId(*neighborhood)),
            }),
            other => moments.push(Moment::Did {
                action: other.key(),
                hood: None,
            }),
        },
        _ => {}
    }
    for m in moments {
        learn(state, content, m);
    }
}

/// Teach what a moment reveals: each open proposal in the player's focus
/// whose `heard`, `known` or `detailed` hook it matches.
pub fn learn(state: &mut State, content: &ContentBundle, m: Moment<'_>) {
    let focus = state.focus();
    let mut grants: Vec<(FamilyInstanceId, Level, Source)> = Vec::new();
    for inst in state
        .policy
        .families
        .instances
        .iter()
        .filter(|i| i.is_open() && focus.contains(&i.at))
    {
        let Some(f) = content.family(&inst.family) else {
            continue;
        };
        let k = &f.knowledge;
        for (level, hooks) in [
            (Level::Heard, &k.heard),
            (Level::Known, &k.known),
            (Level::Detailed, &k.detailed),
        ] {
            if let Some(src) = hooks
                .iter()
                .find_map(|h| matches(state, content, inst, h, m))
            {
                grants.push((inst.id, level, src));
            }
        }
    }
    for (proposal, level, src) in grants {
        knowledge::teach_one(state, Subject::Proposal { proposal }, level, src);
    }
}

/// Whether a hook fires at a moment, and what the player learned it from.
fn matches(
    state: &State,
    content: &ContentBundle,
    inst: &FamilyInstance,
    h: &HookDef,
    m: Moment<'_>,
) -> Option<Source> {
    use crate::places::Location;
    let place_at = |loc: Location| match loc {
        Location::Place { place } => state.world.places.get(place.index()),
        _ => None,
    };
    match (h, m) {
        (HookDef::Overheard { at }, Moment::At(loc)) => {
            let p = place_at(loc)?;
            (at.iter().any(|k| k == p.kind.key())
                && inst.at.contains_hood(&state.world, p.neighborhood))
            .then_some(Source::Visit { place: p.id })
        }
        (HookDef::Visit { slot }, Moment::At(loc)) => match (inst.bound(slot)?, loc) {
            (Bound::Place { place }, Location::Place { place: here }) if *place == here => {
                Some(Source::Visit { place: here })
            }
            (Bound::Neighborhood { hood }, Location::Street { neighborhood })
                if *hood == neighborhood =>
            {
                Some(Source::Action {
                    action: "visit".into(),
                    place: None,
                })
            }
            _ => None,
        },
        (HookDef::Service { service }, Moment::At(loc)) => {
            let p = place_at(loc)?;
            crate::places::services_at(state, content, loc)
                .contains(&service.as_str())
                .then_some(Source::Service {
                    place: p.id,
                    service: service.clone(),
                })
        }
        (HookDef::Meet { slot } | HookDef::Talk { slot, .. }, Moment::Met(person)) => {
            (inst.person(slot) == Some(person)).then_some(Source::Sponsor {
                proposal: inst.id,
                person,
            })
        }
        (HookDef::Action { action, slot }, Moment::Did { action: did, hood }) => {
            if action != did {
                return None;
            }
            let ok = match slot {
                None => true,
                Some(s) => match inst.bound(s) {
                    Some(Bound::Neighborhood { hood: h }) => hood == Some(*h),
                    _ => false,
                },
            };
            ok.then(|| Source::Action {
                action: action.clone(),
                place: None,
            })
        }
        _ => None,
    }
}
