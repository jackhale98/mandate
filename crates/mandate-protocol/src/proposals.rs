//! Policy-family proposals as the client sees them (DEC-092; task F3;
//! M6_ENGINE_CONTRACTS §2.12): the proposals on the player's desk and at a
//! place, and the preview of a draft with every option's cost, support and
//! impact (DEC-111), computed by the engine. The UI renders these and
//! computes nothing.
//!
//! **Knowledge (DEC-091, §1.1).** A proposal the player has not heard of is
//! left out. Heard of: a stub with its title, the reason it is on the table,
//! the sponsors' roles and the lead; no menu, no figures, and it cannot be
//! drafted. Known: the full menu (axes, options, their costs and impact), the
//! sponsors by name with their versions and prices (a price is what an action
//! costs, never gated). Support follows the `policy_support` subject as every
//! support does: a word below `known`, classes to 0.1 at `known`, exact in
//! detail. The impact is the same estimate as a draft bill's (no further
//! gate until the estimate's knowledge levels are built, its task 4).
//!
//! **Tier-generic (DEC-098).** A proposal is keyed by its `Jurisdiction`
//! (`at`); who may draft it is read from the constitution: the posts of that
//! jurisdiction whose powers include the family's `requires`. Nothing here
//! names an office or a body.

use crate::characters::{PersonRefView, person_ref_known};
use crate::impact::{ImpactView, impact_of};
use crate::knowledge::{Gate, KnowledgeLevelDto, KnowledgeView, gate_support};
use crate::office::{ParamValue, ParamView, SideEffectView, SupportView, side_effects_view, snake};
use crate::places::{LocationView, location_view};
use crate::post::{JurisdictionDto, PostDto, jurisdiction_dto, post_dto};
use crate::views::DateView;
use mandate_content::policy_family::{PolicyFamilyDef, PriceDef};
use mandate_content::{ContentBundle, ParamDef, PostKey};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Level, PolicyRef, Subject};
use mandate_sim::policy::PolicyInstance;
use mandate_sim::policy::drafting::{self, Prepared, Unavailable};
use mandate_sim::policy::families::{Bound, FamilyInstance, FamilyInstanceId, FamilyStatus};
use mandate_sim::politics::Post;
use mandate_sim::world::Jurisdiction;
use mandate_sim::{Engine, State};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// One design choice of a family draft: an axis and its option.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ChoiceValue {
    pub axis: String,
    pub option: String,
}

/// What a slot of a proposal is bound to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BindingView {
    pub slot: String,
    /// `place` | `neighborhood` | `person` | `sector` | `crisis` | `hazard`.
    pub kind: String,
    /// A place's or a neighbourhood's name, or a person's as the player
    /// knows it; empty for a sector, a crisis or a hazard (`name_key`).
    pub name: String,
    /// Fluent key of a sector, a crisis or a hazard.
    pub name_key: Option<String>,
    /// Where it is: the place, or the streets of the neighbourhood.
    pub location: Option<LocationView>,
    /// The person bound to a person slot.
    pub person: Option<PersonRefView>,
}

/// A sponsor of a proposal (DEC-092 §4), as the desk lists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SponsorRefView {
    pub id: String,
    /// Fluent key of their version (`sponsor-…`).
    pub name_key: String,
    /// Who brings it: by name once the proposal is known, else as the
    /// player knows the person (the role alone below `heard`).
    pub person: PersonRefView,
}

/// A proposal on the table (DEC-092): a policy family bound to real things
/// in one jurisdiction.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProposalView {
    /// The proposal's id (`QueryDto::PolicyPreview.proposal`,
    /// `CommandDto::DraftFamily.proposal`).
    pub id: u32,
    /// The family's content id (`QueryDto::PolicyPreview.template`).
    pub family: String,
    pub name_key: String,
    pub desc_key: String,
    /// Arguments of every Fluent key of the proposal and its options: the
    /// bound things' names by slot, `<slot>_key` for a sector, a crisis or
    /// a hazard; a place with no proper name as keys (`<slot>_key`: a named
    /// plant's `plant-name-<form>` with `stem` and `works_key`, else its
    /// kind). A person the player has not heard of is an empty name.
    pub args: BTreeMap<String, String>,
    /// Budget category in snake case.
    pub category: String,
    pub at: JurisdictionDto,
    /// What the player knows of it, the lead towards knowing more.
    pub level: KnowledgeLevelDto,
    pub knowledge: KnowledgeView,
    /// Fluent key of the circumstances that put it on the table.
    pub reason_key: Option<String>,
    /// What it is bound to, in slot order (from `known`).
    pub bindings: Vec<BindingView>,
    /// The yearly cost at the defaults (from `known`; never otherwise gated).
    pub est_annual_cost: Option<f64>,
    /// Months to put in place at the defaults (from `known`).
    pub months: Option<u32>,
    /// Who brings a ready version.
    pub sponsors: Vec<SponsorRefView>,
    /// `open` | `lapsed`.
    pub status: String,
    /// `expired` | `eased` | `too_long` | `removed`, once lapsed.
    pub lapse_reason: Option<String>,
    pub opened: DateView,
    /// Bills drafted from it.
    pub bills: Vec<u32>,
    /// The power drafting it needs (`propose_ordinance`, …).
    pub requires: String,
    /// The posts that hold that power in its jurisdiction, from the
    /// constitution (a seat in a body is listed once, without a district).
    pub proposers: Vec<PostDto>,
    /// Whether the player may draft it now (at the town hall, free).
    pub can_draft: bool,
    /// Why not: `proposal-blocked-<lapsed|heard|no-power>`.
    pub blocked_key: Option<String>,
}

/// What adopting a sponsor's version costs the player (DEC-092 §2.6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PriceView {
    /// `favour` | `endorsement`.
    pub kind: String,
    /// `sponsor-price-favour` (`count`, `context_key`) or
    /// `sponsor-price-endorsement` (`who`: `everyone` | `classes` |
    /// `area`, with `classes` the class words joined by commas and `area`
    /// the neighbourhoods' names).
    pub key: String,
    pub args: BTreeMap<String, String>,
    /// Favours the player would owe the sponsor.
    pub favours: u32,
    /// An endorsement: approval gained among the sponsor's people, and lost
    /// among everyone else in the jurisdiction (before room scaling).
    pub approval: f64,
    pub backlash: f64,
}

/// A sponsor's ready version and its price.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SponsorTermsView {
    pub id: String,
    pub name_key: String,
    pub person: PersonRefView,
    /// One option per axis (a missing axis in content: its default).
    pub preset: Vec<ChoiceValue>,
    /// Their slider values.
    pub params: Vec<ParamValue>,
    pub price: PriceView,
    /// The preview's choices and sliders are this version as it is (drafting
    /// now with `sponsor` set adopts it).
    pub matches_current: bool,
}

/// One option of an axis: the whole package with this option swapped in,
/// the other axes as chosen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OptionPreviewView {
    pub id: String,
    pub name_key: String,
    pub desc_key: String,
    /// The option chosen in the preview.
    pub chosen: bool,
    /// The axis default.
    pub is_default: bool,
    pub est_annual_cost: f64,
    /// That cost as a share of the jurisdiction's yearly budget.
    pub budget_share: f64,
    pub months: u32,
    /// Gated like every support (`policy_support`); `None` for an option
    /// that cannot be combined with the other choices (no such package).
    pub support: Option<SupportView>,
    pub side_effects: Vec<SideEffectView>,
    /// What the package would probably do (DEC-111); `None` where the
    /// impact table covers no such tier, or for an unavailable option.
    pub impact: Option<ImpactView>,
    /// The package can be drafted with this option (the proposal still
    /// open); whether the player may draft it is `ProposalView.can_draft`.
    pub available: bool,
    /// Why not: `option-unavailable-needs` or `option-unavailable-excludes`
    /// (`with_key`: the other option's name key) or `option-unavailable`.
    pub unavailable_key: Option<String>,
    pub unavailable_args: BTreeMap<String, String>,
}

/// A design choice of a family and its options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AxisPreviewView {
    pub id: String,
    pub name_key: String,
    /// The option chosen in the preview.
    pub chosen: String,
    pub options: Vec<OptionPreviewView>,
}

/// The family part of a preview (`PolicyPreviewView.family`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FamilyPreviewView {
    /// The proposal as the desk shows it (level, lead, who may draft it).
    pub proposal: ProposalView,
    /// The choices as the engine read them (a missing axis: its default).
    pub choices: Vec<ChoiceValue>,
    /// The sliders of the package as chosen (fixed values left out).
    pub param_defs: Vec<ParamView>,
    /// Months to put the package as chosen in place.
    pub months: u32,
    /// Empty below `known` (a stub: drafting needs it known).
    pub axes: Vec<AxisPreviewView>,
    /// Empty below `known`.
    pub sponsors: Vec<SponsorTermsView>,
    /// Why the package as chosen cannot be drafted (as an option's
    /// `unavailable_key`), if it cannot.
    pub unavailable_key: Option<String>,
    pub unavailable_args: BTreeMap<String, String>,
}

/// What a bill drafted from a proposal was drafted with (`BillView.family`):
/// amending it (`CommandDto::AmendPolicy.choices`) changes these.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BillFamilyView {
    pub proposal: u32,
    /// One option per axis, in axis order.
    pub choices: Vec<ChoiceValue>,
    /// The sponsor whose version was adopted, if any.
    pub sponsor: Option<String>,
    /// Arguments of the bill's keys (as `ProposalView.args`).
    pub args: BTreeMap<String, String>,
    /// The neighbourhoods it applies to; empty: the whole jurisdiction.
    pub area: Vec<String>,
}

/// A family bill's draft, for its bill view.
pub fn bill_family(engine: &Engine, p: &PolicyInstance) -> Option<BillFamilyView> {
    let st = engine.state();
    let d = p.family.as_ref()?;
    let gate = Gate::of(engine);
    let args = st
        .policy
        .families
        .get(d.instance)
        .map(|i| args_of(engine, &gate, i, level_of(&gate, i)))
        .unwrap_or_default();
    Some(BillFamilyView {
        proposal: d.instance.0,
        choices: d
            .choices
            .iter()
            .map(|(a, o)| ChoiceValue {
                axis: a.clone(),
                option: o.clone(),
            })
            .collect(),
        sponsor: d.sponsor.clone(),
        args,
        area: p
            .area()
            .unwrap_or_default()
            .iter()
            .filter_map(|h| st.world.neighborhoods.get(h.index()))
            .map(|n| n.name.clone())
            .collect(),
    })
}

// --- Reading a proposal ---------------------------------------------------------------

fn power_word(p: mandate_content::Power) -> String {
    snake(&p)
}

/// The proposal's knowledge level (`detailed` without a player).
fn level_of(gate: &Gate<'_>, inst: &FamilyInstance) -> Level {
    gate.level(&Subject::Proposal { proposal: inst.id })
}

/// What the player knows of who backs a proposal: what they learned of it,
/// or of a bill of its family in its town (a town hall or the chamber on
/// one teaches the family's support there).
fn support_level(gate: &Gate<'_>, st: &State, inst: &FamilyInstance) -> Level {
    let Some(town) = inst.at.municipality() else {
        return if st.player.is_none() {
            Level::Detailed
        } else {
            Level::Unknown
        };
    };
    let of = |policy| gate.level(&Subject::PolicySupport { town, policy });
    of(PolicyRef::Proposal { proposal: inst.id }).max(of(PolicyRef::Template {
        template: inst.family.clone(),
    }))
}

/// The slots' arguments as the player may see them: a person's name only
/// once heard of them, or once the proposal is known. A place with no
/// proper name is named by keys, as everywhere else: a named plant as
/// `<slot>_key` (`plant-name-<form>`) with `stem` and `works_key` (DEC-110),
/// another place by its in-sentence kind (`<slot>_key`), so a title never
/// prints an empty name.
fn args_of(
    engine: &Engine,
    gate: &Gate<'_>,
    inst: &FamilyInstance,
    level: Level,
) -> BTreeMap<String, String> {
    let st = engine.state();
    let mut args = inst.args.clone();
    for b in &inst.bindings {
        match b.bound {
            Bound::Person { person } if level < Level::Known => {
                if let Some(v) = args.get_mut(&b.slot) {
                    *v = gate.name(person, std::mem::take(v));
                }
            }
            Bound::Place { place } if args.get(&b.slot).is_none_or(|n| n.is_empty()) => {
                args.remove(&b.slot);
                let key = format!("{}_key", b.slot);
                match mandate_sim::economy::plants::at_place(&st.economy, place) {
                    Some(plant) => {
                        for (k, v) in mandate_sim::economy::plants::text_args(
                            &st.economy,
                            engine.content(),
                            plant.id,
                        ) {
                            let k = if k == "plant_key" {
                                key.clone()
                            } else {
                                k.into()
                            };
                            args.entry(k).or_insert(v);
                        }
                    }
                    None => {
                        if let Some(p) = st.world.places.get(place.index()) {
                            args.insert(key, mandate_sim::talk::context::place_talk_key(p.kind));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    args
}

fn person_view(
    engine: &Engine,
    gate: &Gate<'_>,
    person: mandate_sim::characters::CharacterId,
    level: Level,
) -> Option<PersonRefView> {
    if level >= Level::Known {
        crate::characters::person_ref(engine, person)
    } else {
        person_ref_known(engine, gate, person)
    }
}

fn binding_view(
    engine: &Engine,
    gate: &Gate<'_>,
    f: &PolicyFamilyDef,
    b: &mandate_sim::policy::families::SlotBinding,
    level: Level,
    args: &BTreeMap<String, String>,
) -> Option<BindingView> {
    let st = engine.state();
    let kind = mandate_sim::policy::families::slot_kind(f, &b.slot)
        .map(|k| k.label().to_string())
        .unwrap_or_default();
    let key = args.get(&format!("{}_key", b.slot)).cloned();
    let mut v = BindingView {
        slot: b.slot.clone(),
        kind,
        name: String::new(),
        name_key: None,
        location: None,
        person: None,
    };
    match &b.bound {
        Bound::Place { place } => {
            v.name = args.get(&b.slot).cloned().unwrap_or_default();
            v.name_key = key;
            v.location = Some(location_view(st, Location::Place { place: *place }));
        }
        Bound::Neighborhood { hood } => {
            v.name = args.get(&b.slot).cloned().unwrap_or_default();
            v.location = Some(location_view(
                st,
                Location::Street {
                    neighborhood: *hood,
                },
            ));
        }
        Bound::Person { person } => {
            let p = person_view(engine, gate, *person, level)?;
            v.name = p.name.clone();
            v.person = Some(p);
        }
        Bound::Sector { .. } | Bound::Crisis { .. } | Bound::Hazard { .. } => v.name_key = key,
        Bound::Unbound => return None,
    }
    Some(v)
}

/// The posts of a jurisdiction that hold `power`, from the constitution.
fn proposers(engine: &Engine, at: Jurisdiction, power: mandate_content::Power) -> Vec<PostDto> {
    let Some(tier) = crate::post::constitution(engine).and_then(|c| c.tier(at.tier())) else {
        return Vec::new();
    };
    let bodies = tier
        .bodies
        .iter()
        .filter(|b| b.powers.contains(&power))
        .filter_map(|b| {
            Some(Post::Seat {
                body: PostKey::new(&b.id)?,
                at,
                constituency: None,
            })
        });
    let offices = tier
        .offices
        .iter()
        .filter(|o| o.powers.contains(&power))
        .filter_map(|o| {
            Some(Post::Office {
                office: PostKey::new(&o.id)?,
                at,
            })
        });
    bodies.chain(offices).map(|p| post_dto(engine, p)).collect()
}

/// Whether the player may draft a proposal now, and why not (the checks
/// `DraftFamily` makes, in its order).
fn blocked(
    engine: &Engine,
    f: &PolicyFamilyDef,
    inst: &FamilyInstance,
    level: Level,
) -> Option<&'static str> {
    let st = engine.state();
    if !inst.is_open() {
        return Some("proposal-blocked-lapsed");
    }
    if level < Level::Known {
        return Some("proposal-blocked-heard");
    }
    if !mandate_sim::actions::player_holds_power_at(st, engine.content(), f.requires, inst.at) {
        return Some("proposal-blocked-no-power");
    }
    None
}

/// A checked draft of a proposal at these choices and sliders (open or
/// lapsed), as a bill that is not stored.
fn probe(
    st: &State,
    content: &ContentBundle,
    inst: &FamilyInstance,
    choices: &BTreeMap<String, String>,
    params: &BTreeMap<String, f64>,
) -> Result<(Prepared, PolicyInstance), String> {
    let p = drafting::prepare_any(st, content, inst.id, choices, params)?;
    let bill = drafting::probe_bill(content, &p);
    Ok((p, bill))
}

/// A proposal's title key and the arguments it takes, as the player may see
/// them (the notebook's entry for it).
pub fn title(engine: &Engine, inst: &FamilyInstance) -> Option<(String, BTreeMap<String, String>)> {
    let f = engine.content().family(&inst.family)?;
    let gate = Gate::of(engine);
    Some((
        f.name_key.clone(),
        args_of(engine, &gate, inst, level_of(&gate, inst)),
    ))
}

/// A proposal's view, or `None` while the player has not heard of it.
pub fn proposal_view(engine: &Engine, inst: &FamilyInstance) -> Option<ProposalView> {
    let st = engine.state();
    let content = engine.content();
    let gate = Gate::of(engine);
    let f = content.family(&inst.family)?;
    let level = level_of(&gate, inst);
    if level < Level::Heard {
        return None;
    }
    let known = level >= Level::Known;
    let default = known
        .then(|| probe(st, content, inst, &BTreeMap::new(), &BTreeMap::new()).ok())
        .flatten();
    let blocked_key = blocked(engine, f, inst, level);
    let args = args_of(engine, &gate, inst, level);
    let (status, lapse_reason) = match &inst.status {
        FamilyStatus::Open => ("open", None),
        FamilyStatus::Lapsed { reason, .. } => ("lapsed", Some(snake(reason))),
    };
    Some(ProposalView {
        id: inst.id.0,
        family: f.id.clone(),
        name_key: f.name_key.clone(),
        desc_key: f.desc_key.clone(),
        args: args.clone(),
        category: snake(&f.category),
        at: jurisdiction_dto(&st.world, inst.at),
        level: level.into(),
        knowledge: gate.view(&Subject::Proposal { proposal: inst.id }),
        reason_key: f.trigger.as_ref().map(|t| t.reason_key.clone()),
        bindings: if known {
            inst.bindings
                .iter()
                .filter_map(|b| binding_view(engine, &gate, f, b, level, &args))
                .collect()
        } else {
            Vec::new()
        },
        est_annual_cost: default
            .as_ref()
            .map(|(_, b)| mandate_sim::actions::estimate_cost(st, content, b)),
        months: default.as_ref().and_then(|(_, b)| {
            mandate_sim::policy::resolve(content, &st.policy, b)
                .map(|t| t.implementation_months as u32)
        }),
        sponsors: inst
            .sponsors
            .iter()
            .filter_map(|s| {
                let def = f.sponsor(&s.sponsor)?;
                Some(SponsorRefView {
                    id: def.id.clone(),
                    name_key: def.name_key.clone(),
                    person: person_view(engine, &gate, s.person, level)?,
                })
            })
            .collect(),
        status: status.into(),
        lapse_reason,
        opened: DateView::of(inst.opened),
        bills: inst.drafts.iter().map(|d| d.0).collect(),
        requires: power_word(f.requires),
        proposers: proposers(engine, inst.at, f.requires),
        can_draft: blocked_key.is_none(),
        blocked_key: blocked_key.map(String::from),
    })
}

/// The open proposals of the player's focus jurisdictions the player has
/// heard of, in the order they opened (`OfficeView.proposals`).
pub fn proposals(engine: &Engine) -> Vec<ProposalView> {
    let st = engine.state();
    let focus = st.focus();
    st.policy
        .families
        .instances
        .iter()
        .filter(|i| i.is_open() && focus.contains(&i.at))
        .filter_map(|i| proposal_view(engine, i))
        .collect()
}

/// The open proposals about a location the player has heard of: bound to
/// the place, or to the neighbourhood whose streets these are, or brought
/// by a sponsor who works here (`PlacePanelView.proposals_here`).
pub fn proposals_here(engine: &Engine, loc: Location) -> Vec<ProposalView> {
    let st = engine.state();
    let content = engine.content();
    let about = |i: &FamilyInstance| {
        i.bindings.iter().any(|b| match (&b.bound, loc) {
            (Bound::Place { place }, Location::Place { place: here }) => *place == here,
            (Bound::Neighborhood { hood }, Location::Street { neighborhood }) => {
                *hood == neighborhood
            }
            _ => false,
        }) || i.sponsors.iter().any(|s| {
            mandate_sim::characters::routines::workplace(st, content, s.person) == Some(loc)
        })
    };
    st.policy
        .families
        .instances
        .iter()
        .filter(|i| i.is_open() && about(i))
        .filter_map(|i| proposal_view(engine, i))
        .collect()
}

// --- The preview ----------------------------------------------------------------------

/// What the preview computes for one package.
struct Package {
    cost: f64,
    budget_share: f64,
    months: u32,
    support: SupportView,
    side_effects: Vec<SideEffectView>,
    impact: Option<ImpactView>,
}

fn package(
    engine: &Engine,
    support_level: Level,
    budget: f64,
    hoods: &[usize],
    bill: &PolicyInstance,
) -> Option<Package> {
    let st = engine.state();
    let content = engine.content();
    let t = mandate_sim::policy::resolve(content, &st.policy, bill)?;
    let cost = mandate_sim::actions::estimate_cost(st, content, bill);
    Some(Package {
        cost,
        budget_share: if budget > 0.0 { cost / budget } else { 0.0 },
        months: t.implementation_months as u32,
        support: gate_support(
            support_level,
            mandate_sim::policy::support_by_class(&t, &st.population, hoods),
            mandate_sim::policy::public_support(&t, &st.population, hoods),
        ),
        side_effects: side_effects_view(&t, bill),
        impact: impact_of(engine, bill),
    })
}

fn unavailable_of(
    f: &PolicyFamilyDef,
    picked: &[(String, String)],
) -> (String, BTreeMap<String, String>) {
    match drafting::unavailable(f, picked) {
        Some(Unavailable::Needs { .. }) => ("option-unavailable-needs".into(), BTreeMap::new()),
        Some(Unavailable::Excludes { with, .. }) => {
            let key = with
                .split_once('.')
                .and_then(|(a, o)| f.axis(a)?.option(o))
                .map(|o| o.name_key.clone())
                .unwrap_or_default();
            (
                "option-unavailable-excludes".into(),
                BTreeMap::from([("with_key".to_string(), key)]),
            )
        }
        None => ("option-unavailable".into(), BTreeMap::new()),
    }
}

fn param_view(p: &ParamDef) -> ParamView {
    ParamView {
        id: p.id.clone(),
        name_key: p.name_key.clone(),
        unit_key: p.unit_key.clone(),
        min: p.min,
        max: p.max,
        step: p.step,
        default: p.default,
    }
}

fn price_view(st: &State, inst: &FamilyInstance, price: &PriceDef) -> PriceView {
    match price {
        PriceDef::Favour { count, context_key } => PriceView {
            kind: "favour".into(),
            key: "sponsor-price-favour".into(),
            args: BTreeMap::from([
                ("count".to_string(), count.to_string()),
                ("context_key".to_string(), context_key.clone()),
            ]),
            favours: *count as u32,
            approval: 0.0,
            backlash: 0.0,
        },
        PriceDef::Endorsement {
            classes,
            area,
            approval,
            backlash,
        } => {
            let mut args = BTreeMap::new();
            let area = area
                .as_ref()
                .and_then(|a| mandate_sim::policy::families::area_hoods(&st.world, inst, a));
            let who = if let Some(hoods) = area {
                let names: Vec<String> = hoods
                    .iter()
                    .filter_map(|h| st.world.neighborhoods.get(h.index()))
                    .map(|n| n.name.clone())
                    .collect();
                args.insert("area".to_string(), names.join(", "));
                "area"
            } else if !classes.is_empty() {
                let words: Vec<String> = classes.iter().map(snake).collect();
                args.insert("classes".to_string(), words.join(","));
                "classes"
            } else {
                "everyone"
            };
            args.insert("who".to_string(), who.to_string());
            PriceView {
                kind: "endorsement".into(),
                key: "sponsor-price-endorsement".into(),
                args,
                favours: 0,
                approval: *approval,
                backlash: *backlash,
            }
        }
    }
}

/// The engine's preview of a draft of a proposal (`QueryDto::PolicyPreview`
/// with `proposal`): the package as chosen, every option of every axis with
/// that option swapped in (at most one composition per option, the package
/// as chosen shared: seven for three axes of three), and the sponsors'
/// versions. `None` for no such proposal, one the player has not heard of,
/// or a `template` that is not the proposal's family.
pub fn family_preview(
    engine: &Engine,
    template: &str,
    proposal: u32,
    params: &[ParamValue],
    choices: &[ChoiceValue],
) -> Option<crate::office::PolicyPreviewView> {
    let st = engine.state();
    let content = engine.content();
    let gate = Gate::of(engine);
    let inst = st.policy.families.get(FamilyInstanceId(proposal))?;
    if !template.is_empty() && template != inst.family {
        return None;
    }
    let f = content.family(&inst.family)?;
    let view = proposal_view(engine, inst)?;
    let level = level_of(&gate, inst);
    let support_level = support_level(&gate, st, inst);
    let hoods: Vec<usize> = inst.at.hoods(&st.world);
    let budget = mandate_sim::finance::treasury(&st.finance, inst.at)
        .map(|t| t.budget_total().units() as f64)
        .unwrap_or(0.0);
    let given: BTreeMap<String, f64> = params.iter().map(|p| (p.id.clone(), p.value)).collect();
    let mut chosen: BTreeMap<String, String> = BTreeMap::new();
    for a in &f.axes {
        let id = choices
            .iter()
            .find(|c| c.axis == a.id)
            .map(|c| c.option.clone())
            .or_else(|| a.default_option().map(str::to_string));
        if let Some(id) = id {
            chosen.insert(a.id.clone(), id);
        }
    }
    let picked: Vec<(String, String)> = f
        .axes
        .iter()
        .filter_map(|a| Some((a.id.clone(), chosen.get(&a.id)?.clone())))
        .collect();
    let choice_values: Vec<ChoiceValue> = picked
        .iter()
        .map(|(a, o)| ChoiceValue {
            axis: a.clone(),
            option: o.clone(),
        })
        .collect();
    let stub = level < Level::Known;
    let main = if stub {
        None
    } else {
        probe(st, content, inst, &chosen, &given).ok()
    };
    let main_pkg = main
        .as_ref()
        .and_then(|(_, b)| package(engine, support_level, budget, &hoods, b));
    let (unavailable_key, unavailable_args) = match (&main, stub) {
        (None, false) => {
            let (k, a) = unavailable_of(f, &picked);
            (Some(k), a)
        }
        _ => (None, BTreeMap::new()),
    };
    let composed = mandate_sim::policy::resolve::compose(content, f, Some(inst), &chosen);
    let param_defs: Vec<ParamView> = if stub {
        Vec::new()
    } else {
        composed
            .as_ref()
            .map(|t| {
                t.params
                    .iter()
                    .filter(|p| !p.hidden)
                    .map(param_view)
                    .collect()
            })
            .unwrap_or_default()
    };
    let shown_params: Vec<ParamValue> = match &main {
        Some((p, _)) => param_defs
            .iter()
            .map(|d| ParamValue {
                id: d.id.clone(),
                value: p.params.get(&d.id).copied().unwrap_or(d.default),
            })
            .collect(),
        None => Vec::new(),
    };
    let axes: Vec<AxisPreviewView> = if stub {
        Vec::new()
    } else {
        f.axes
            .iter()
            .map(|a| {
                let current = chosen.get(&a.id).cloned().unwrap_or_default();
                let options =
                    a.options
                        .iter()
                        .map(|o| {
                            let is_chosen = o.id == current;
                            let mut swapped = chosen.clone();
                            swapped.insert(a.id.clone(), o.id.clone());
                            let swapped_picked: Vec<(String, String)> = f
                                .axes
                                .iter()
                                .filter_map(|x| Some((x.id.clone(), swapped.get(&x.id)?.clone())))
                                .collect();
                            let pkg = if is_chosen {
                                main_pkg.as_ref().map(|p| Package {
                                    cost: p.cost,
                                    budget_share: p.budget_share,
                                    months: p.months,
                                    support: p.support.clone(),
                                    side_effects: p.side_effects.clone(),
                                    impact: p.impact.clone(),
                                })
                            } else {
                                probe(st, content, inst, &swapped, &given).ok().and_then(
                                    |(_, b)| package(engine, support_level, budget, &hoods, &b),
                                )
                            };
                            let (unavailable_key, unavailable_args) = if pkg.is_some() {
                                (None, BTreeMap::new())
                            } else {
                                let (k, a) = unavailable_of(f, &swapped_picked);
                                (Some(k), a)
                            };
                            let support = pkg.as_ref().map(|p| p.support.clone());
                            let pkg = pkg.unwrap_or(Package {
                                cost: 0.0,
                                budget_share: 0.0,
                                months: 0,
                                support: gate_support(Level::Unknown, [0.0; 3], 0.0),
                                side_effects: Vec::new(),
                                impact: None,
                            });
                            OptionPreviewView {
                                id: o.id.clone(),
                                name_key: o.name_key.clone(),
                                desc_key: o.desc_key.clone(),
                                chosen: is_chosen,
                                is_default: a.default_option() == Some(o.id.as_str()),
                                est_annual_cost: pkg.cost,
                                budget_share: pkg.budget_share,
                                months: pkg.months,
                                support,
                                side_effects: pkg.side_effects,
                                impact: pkg.impact,
                                available: unavailable_key.is_none() && inst.is_open(),
                                unavailable_key,
                                unavailable_args,
                            }
                        })
                        .collect();
                AxisPreviewView {
                    id: a.id.clone(),
                    name_key: a.name_key.clone(),
                    chosen: current,
                    options,
                }
            })
            .collect()
    };
    let sponsors: Vec<SponsorTermsView> = if stub {
        Vec::new()
    } else {
        inst.sponsors
            .iter()
            .filter_map(|s| {
                let def = f.sponsor(&s.sponsor)?;
                let preset: Vec<ChoiceValue> = f
                    .axes
                    .iter()
                    .filter_map(|a| {
                        let o = def
                            .preset
                            .get(&a.id)
                            .cloned()
                            .or_else(|| a.default_option().map(str::to_string))?;
                        Some(ChoiceValue {
                            axis: a.id.clone(),
                            option: o,
                        })
                    })
                    .collect();
                let same_choices = preset == choice_values;
                // Their sliders as the engine reads them for their version.
                let theirs = drafting::prepare(
                    st,
                    content,
                    inst.id,
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    Some(&def.id),
                )
                .ok()
                .map(|p| p.params);
                let params: Vec<ParamValue> = def
                    .params
                    .iter()
                    .map(|(k, v)| ParamValue {
                        id: k.clone(),
                        value: theirs
                            .as_ref()
                            .and_then(|t| t.get(k).copied())
                            .unwrap_or(*v),
                    })
                    .collect();
                let same_params = match (&main, &theirs) {
                    (Some((p, _)), Some(t)) => t
                        .iter()
                        .all(|(k, v)| p.params.get(k).is_some_and(|x| (x - v).abs() < 1e-9)),
                    _ => false,
                };
                Some(SponsorTermsView {
                    id: def.id.clone(),
                    name_key: def.name_key.clone(),
                    person: person_view(engine, &gate, s.person, level)?,
                    preset,
                    params,
                    price: price_view(st, inst, &def.price),
                    matches_current: same_choices && same_params,
                })
            })
            .collect()
    };
    let months = main_pkg.as_ref().map(|p| p.months).unwrap_or(0);
    let available = main.is_some() && inst.is_open();
    Some(crate::office::PolicyPreviewView {
        template: f.id.clone(),
        params: shown_params,
        est_annual_cost: main_pkg.as_ref().map(|p| p.cost).unwrap_or(0.0),
        budget_share: main_pkg.as_ref().map(|p| p.budget_share).unwrap_or(0.0),
        side_effects: main_pkg
            .as_ref()
            .map(|p| p.side_effects.clone())
            .unwrap_or_default(),
        // Below `known` (or a package that cannot be drafted), the support
        // of the default version, gated as ever.
        support: main_pkg
            .as_ref()
            .map(|p| p.support.clone())
            .or_else(|| {
                let (_, b) = probe(st, content, inst, &BTreeMap::new(), &BTreeMap::new()).ok()?;
                let t = mandate_sim::policy::resolve(content, &st.policy, &b)?;
                Some(gate_support(
                    support_level,
                    mandate_sim::policy::support_by_class(&t, &st.population, &hoods),
                    mandate_sim::policy::public_support(&t, &st.population, &hoods),
                ))
            })
            .unwrap_or_else(|| gate_support(Level::Unknown, [0.0; 3], 0.0)),
        available,
        impact: main_pkg.and_then(|p| p.impact),
        family: Some(FamilyPreviewView {
            proposal: view,
            choices: choice_values,
            param_defs,
            months,
            axes,
            sponsors,
            unavailable_key,
            unavailable_args,
        }),
    })
}
