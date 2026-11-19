//! Characters and town character for the client (DEC-031, DEC-077 §1): facts
//! and Fluent keys only — names, roles, coarse personality labels, opinion
//! and trust bands, bonds, favours, and the memories a character would quote.

use crate::knowledge::{Gate, KnowledgeLevelDto, KnowledgeView, gate_concerns};
use crate::office::ConcernView;
use crate::places::{LocationView, location_view};
use crate::views::DateView;
use mandate_sim::characters::{self, CharacterId, MemoryKind, Tier, ambient};
use mandate_sim::places::Location;
use mandate_sim::player::knowledge::{Level, Subject};
use mandate_sim::politics::Role;
use mandate_sim::politics::elections::Seat;
use mandate_sim::world::MunicipalityId;
use mandate_sim::{Engine, State};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Someone to show in a list: stored (`id`), or a passer-by (`ambient_slot`)
/// who becomes a character when the player meets them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PersonRefView {
    pub id: Option<u32>,
    pub ambient_slot: Option<u8>,
    /// Empty where the player has not heard of them (DEC-091): show the
    /// role ("a union steward"). Ids still travel: commands need them.
    pub name: String,
    /// Fluent key: `role-councillor`, `role-mayor`, `role-private`,
    /// `role-candidate`, or an occupation's name key.
    pub role_key: String,
    /// `principal` | `named` | `ambient`.
    pub tier: String,
    /// What the player knows of them (passers-by: `unknown`).
    pub level: KnowledgeLevelDto,
    /// The name is only a rumour (the power broker, heard of).
    pub rumoured: bool,
}

/// What a memory or favour is about.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SubjectView {
    /// Fluent key: a policy's name key, `subject-mayoralty`, or `subject-ward`.
    pub key: String,
    /// The ward number, for `subject-ward`.
    pub n: Option<u32>,
}

/// A memory the character would bring up (facts and keys, no prose).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MemoryView {
    /// Fluent key, e.g. `memory-broke-promise` (args: `subject`, `year`).
    pub key: String,
    pub subject: Option<SubjectView>,
    pub year: u32,
    /// `good` | `bad` | `mixed`, from the holder's side.
    pub tone: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FavourView {
    /// The player owes them (otherwise they owe the player).
    pub you_owe: bool,
    pub count: u32,
    /// Fluent key of the context, e.g. `favour-bill`.
    pub context_key: String,
    pub subject: Option<SubjectView>,
    pub since_year: u32,
}

/// A favour between this character and someone other than the player
/// (DEC-091: known only in detail).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FavourWithView {
    pub counterpart: PersonRefView,
    /// They owe the counterpart (otherwise the counterpart owes them).
    pub they_owe: bool,
    pub count: u32,
}

/// A character sheet (DEC-031), as far as the player knows them (DEC-091):
/// below `heard` no name; below `known` no personality, traits or
/// expertise, and no place below `heard`; goals and their favours with
/// others in detail only. How they see the player is never gated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CharacterView {
    pub id: u32,
    pub name: String,
    pub role_key: String,
    pub ward_number: Option<u32>,
    pub party: Option<u32>,
    /// `principal` | `named`.
    pub tier: String,
    pub age: u32,
    pub home_neighborhood: String,
    /// Where to find them, if they have a regular place.
    pub place: Option<LocationView>,
    /// Coarse personality labels (Fluent keys such as `personality-warmth-high`);
    /// middling axes are left out.
    pub personality: Vec<String>,
    /// Trait name keys.
    pub traits: Vec<String>,
    /// Fluent key of the issue they know best (`issue-<attitude key>`).
    pub expertise_key: Option<String>,
    /// Their opinion of the player: `hostile` | `cool` | `neutral` | `warm` | `friendly`.
    pub opinion_band: String,
    /// Their trust in the player: `low` | `some` | `high`.
    pub trust_band: String,
    /// Bonds they hold toward the player (`ally`, `rival`, `family`, …).
    pub bonds: Vec<String>,
    pub owes_you: u32,
    pub you_owe: u32,
    pub favours: Vec<FavourView>,
    /// Their three most salient memories of the player.
    pub memories: Vec<MemoryView>,
    pub is_player: bool,
    pub is_rival: bool,
    /// Once the player knows who really runs the town.
    pub is_broker: bool,
    /// Goal name keys (DEC-032), in detail only.
    pub goals: Option<Vec<String>>,
    /// Favours they hold with others, in detail only.
    pub favours_with_others: Option<Vec<FavourWithView>>,
    pub knowledge: KnowledgeView,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CrisisView {
    pub id: String,
    pub name_key: String,
    /// 0..1, to one decimal.
    pub severity: f64,
    /// `mild` | `serious` | `severe`.
    pub severity_band: String,
    /// `worsening` | `easing` | `steady`.
    pub trend: String,
    pub since: DateView,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SectorRefView {
    pub id: String,
    pub name_key: String,
}

/// A town's character (DEC-077 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TownCharacterView {
    pub industry: Option<SectorRefView>,
    /// The industry's share of jobs, to 5 points.
    pub industry_share: f64,
    pub crisis: Option<CrisisView>,
    /// Who really runs the town, once heard of (`rumoured` until known).
    pub broker: Option<PersonRefView>,
    pub broker_knowledge: KnowledgeView,
    /// The broker's stake (the industry behind them), in detail only.
    pub broker_stake_key: Option<String>,
    pub rival: Option<PersonRefView>,
    /// What the rival is planning, as far as the player knows.
    pub rival_plan: Option<RivalPlanView>,
    /// The town's top concerns, strongest first, as known: none, the top
    /// one (unweighted), three to 0.1, or exact.
    pub priorities: Vec<ConcernView>,
    pub priorities_knowledge: KnowledgeView,
}

/// What the rival is planning (DEC-091 `rival_plans`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RivalPlanView {
    /// Fluent key: `rival-plan-something` (heard), `rival-plan-seat`
    /// (known: `$seat_key`, `$ward`), `rival-plan-focus` (detailed: also
    /// `$focus_key`).
    pub key: String,
    /// `mayor` | `ward`, once known.
    pub seat: Option<String>,
    pub ward_number: Option<u32>,
    /// The issue they campaign on (`issue-<attitude key>`), in detail.
    pub focus_key: Option<String>,
    pub knowledge: KnowledgeView,
}

fn band(v: f64, cuts: &[(f64, &'static str)], top: &'static str) -> String {
    cuts.iter()
        .find(|(c, _)| v < *c)
        .map(|(_, s)| *s)
        .unwrap_or(top)
        .to_string()
}

pub fn opinion_band(v: f64) -> String {
    band(
        v,
        &[
            (-50.0, "hostile"),
            (-15.0, "cool"),
            (15.0, "neutral"),
            (50.0, "warm"),
        ],
        "friendly",
    )
}

pub fn trust_band(v: f64) -> String {
    band(v, &[(0.35, "low"), (0.65, "some")], "high")
}

fn tier_key(t: Tier) -> &'static str {
    match t {
        Tier::Principal => "principal",
        Tier::Named => "named",
    }
}

fn role_key(st: &State, id: CharacterId) -> (String, Option<u32>, Option<u32>) {
    let Some(c) = st.politics.characters.get(id) else {
        return (String::new(), None, None);
    };
    if let Some(p) = c.politician.and_then(|p| st.politics.get(p)) {
        let (key, ward) = match p.role {
            Role::Councillor { ward, .. } => (
                "role-councillor",
                st.world.constituencies.get(ward.index()).map(|w| w.number),
            ),
            Role::Mayor { .. } => ("role-mayor", None),
            Role::Private { .. } if characters::is_candidate(&st.politics, p.id) => {
                ("role-candidate", None)
            }
            Role::Private { .. } => ("role-private", None),
        };
        return (key.into(), ward, p.party.map(|x| x.0));
    }
    (String::new(), None, None)
}

fn occupation_key(engine: &Engine, occ: Option<&str>) -> String {
    occ.and_then(|o| engine.content().occupation(o))
        .map(|o| o.name_key.clone())
        .unwrap_or_default()
}

/// Someone named in full, whatever the player knows of them: for what is
/// aimed at the player (decisions, stories), which is never gated. The
/// level still travels.
pub fn person_ref(engine: &Engine, id: CharacterId) -> Option<PersonRefView> {
    let st = engine.state();
    let p = characters::profile(&st.politics, &st.world, id)?;
    let (mut role, _, _) = role_key(st, id);
    if role.is_empty() {
        role = occupation_key(engine, p.occupation.as_deref());
    }
    Some(PersonRefView {
        id: Some(id.0),
        ambient_slot: None,
        name: p.full_name(),
        role_key: role,
        tier: tier_key(characters::tier_of(&st.politics, engine.content(), id)).into(),
        level: Gate::of(engine).person(id).into(),
        rumoured: false,
    })
}

/// Someone as the player knows them (DEC-091): no name below `heard`.
pub fn person_ref_known(
    engine: &Engine,
    gate: &Gate<'_>,
    id: CharacterId,
) -> Option<PersonRefView> {
    let mut p = person_ref(engine, id)?;
    p.name = gate.name(id, p.name);
    Some(p)
}

pub(crate) fn subject_view(engine: &Engine, s: Option<characters::Subject>) -> Option<SubjectView> {
    use characters::Subject;
    let st = engine.state();
    match s? {
        Subject::Policy { id } => {
            let inst = st.policy.instances.get(id as usize)?;
            let t = engine.content().policies.get(&inst.template)?;
            Some(SubjectView {
                key: t.name_key.clone(),
                n: None,
            })
        }
        Subject::Decision { id } => {
            let d = st
                .narrative
                .decision(mandate_sim::narrative::DecisionId(id))?;
            Some(SubjectView {
                key: engine.content().dilemma(&d.template)?.title_key.clone(),
                n: None,
            })
        }
        Subject::Seat { seat } => Some(match seat {
            Seat::Mayor { .. } => SubjectView {
                key: "subject-mayoralty".into(),
                n: None,
            },
            Seat::Council { ward, .. } => SubjectView {
                key: "subject-ward".into(),
                n: st.world.constituencies.get(ward.index()).map(|w| w.number),
            },
        }),
    }
}

pub(crate) fn tone(sentiment: f32) -> String {
    if sentiment > 0.15 {
        "good"
    } else if sentiment < -0.15 {
        "bad"
    } else {
        "mixed"
    }
    .into()
}

/// A character's sheet as the player sees it.
pub fn character_view(engine: &Engine, id: u32) -> Option<CharacterView> {
    let st = engine.state();
    let content = engine.content();
    let id = CharacterId(id);
    let pol = &st.politics;
    let p = characters::profile(pol, &st.world, id)?;
    let me = characters::player_character(st);
    let is_player = me == Some(id);
    let gate = Gate::of(engine);
    let level = gate.person(id);
    let known = level >= Level::Known;
    let detailed = level >= Level::Detailed;
    let (mut role, ward, party) = role_key(st, id);
    if role.is_empty() {
        role = occupation_key(engine, p.occupation.as_deref());
    }
    let year = st.calendar.date.calendar().year;
    let px = &p.personality;
    let mut personality = Vec::new();
    for (k, v) in [
        ("ambition", px.ambition),
        ("integrity", px.integrity),
        ("warmth", px.warmth),
        ("boldness", px.boldness),
        ("loyalty", px.loyalty),
    ]
    .into_iter()
    .filter(|_| known)
    {
        if v >= 0.67 {
            personality.push(format!("personality-{k}-high"));
        } else if v <= 0.33 {
            personality.push(format!("personality-{k}-low"));
        }
    }
    let traits = p
        .traits
        .iter()
        .filter(|_| known)
        .filter_map(|t| content.trait_def(t))
        .map(|t| t.name_key.clone())
        .collect();
    let expertise_key = Some(&p.competencies.expertise)
        .filter(|e| known && content.attitude_index(e).is_some())
        .map(|e| format!("issue-{e}"));
    let goals = detailed.then(|| {
        pol.characters
            .goals
            .iter()
            .filter(|g| g.holder == id)
            .map(|g| {
                content
                    .goal(&g.goal)
                    .map(|d| d.name_key.clone())
                    .unwrap_or_else(|| g.goal.clone())
            })
            .collect()
    });
    // Favours with anyone but the player (those are the player's own, below).
    let favours_with_others = detailed.then(|| {
        let mut by: std::collections::BTreeMap<(u32, bool), u32> = Default::default();
        for f in &pol.characters.favours {
            let (other, they_owe) = if f.debtor == id {
                (f.creditor, true)
            } else if f.creditor == id {
                (f.debtor, false)
            } else {
                continue;
            };
            if Some(other) == me {
                continue;
            }
            *by.entry((other.0, they_owe)).or_default() += f.count as u32;
        }
        by.into_iter()
            .filter_map(|((other, they_owe), count)| {
                Some(FavourWithView {
                    counterpart: person_ref_known(engine, &gate, CharacterId(other))?,
                    they_owe,
                    count,
                })
            })
            .collect()
    });
    let (opinion, trust, bonds, owes, owe, favours, memories) = match me {
        Some(me) if !is_player => {
            let favours = pol
                .characters
                .favours
                .iter()
                .filter(|f| {
                    (f.debtor == me && f.creditor == id) || (f.debtor == id && f.creditor == me)
                })
                .map(|f| FavourView {
                    you_owe: f.debtor == me,
                    count: f.count as u32,
                    context_key: f.context.clone(),
                    subject: subject_view(engine, f.subject),
                    since_year: f.since.calendar().year,
                })
                .collect();
            let memories = characters::salient_memories(pol, id, Some(me), st.calendar.date, 3)
                .into_iter()
                .map(|m| MemoryView {
                    key: MemoryKind::fluent_key(m.kind),
                    subject: subject_view(engine, m.subject),
                    year: m.date.calendar().year,
                    tone: tone(m.sentiment),
                })
                .collect();
            (
                characters::opinion(pol, id, me) as f64,
                characters::trust(pol, content, id, me) as f64,
                characters::bonds(pol, id, me)
                    .into_iter()
                    .map(|b| b.key().to_string())
                    .collect(),
                characters::favours_owed(pol, id, me),
                characters::favours_owed(pol, me, id),
                favours,
                memories,
            )
        }
        _ => (0.0, 1.0, Vec::new(), 0, 0, Vec::new(), Vec::new()),
    };
    let m = characters::municipality_of(pol, &st.world, id);
    let town = m.and_then(|m| pol.characters.towns.get(m as usize));
    Some(CharacterView {
        id: id.0,
        name: gate.name(id, p.full_name()),
        role_key: role,
        ward_number: ward,
        party,
        tier: tier_key(characters::tier_of(pol, content, id)).into(),
        age: year.saturating_sub(p.birth_year),
        home_neighborhood: st
            .world
            .neighborhoods
            .get(p.home as usize)
            .map(|h| h.name.clone())
            .unwrap_or_default(),
        place: p
            .place
            .filter(|_| level >= Level::Heard)
            .map(|place| location_view(st, Location::Place { place })),
        personality,
        traits,
        expertise_key,
        opinion_band: opinion_band(opinion),
        trust_band: trust_band(trust),
        bonds,
        owes_you: owes,
        you_owe: owe,
        favours,
        memories,
        is_player,
        is_rival: town.is_some_and(|t| t.rival == Some(id)),
        is_broker: m.is_some_and(|m| {
            gate.level(&Subject::Broker {
                town: MunicipalityId(m),
            }) >= Level::Known
        }) && town.is_some_and(|t| t.broker == Some(id)),
        goals,
        favours_with_others,
        knowledge: gate.view(&Subject::Person { person: id }),
    })
}

/// People at a location: stored characters there, then passers-by.
/// Names are as the player knows them (DEC-091): a stranger shows only
/// their role, and a passer-by's name comes with meeting them.
pub fn people_at(engine: &Engine, loc: Location) -> Vec<PersonRefView> {
    let st = engine.state();
    let content = engine.content();
    let gate = Gate::of(engine);
    let max = content.characters.ambient.max_listed as usize;
    let mut out: Vec<PersonRefView> = ambient::stored_at(&st.politics, st, loc)
        .into_iter()
        .filter_map(|id| person_ref_known(engine, &gate, id))
        .collect();
    // Council members are all in the chamber; elsewhere the list is short.
    let chamber = matches!(loc, Location::Place { place } if st.world.places.get(place.index()).is_some_and(|p| p.kind == mandate_sim::world::PlaceKind::CouncilChamber));
    if !chamber {
        out.truncate(max);
    }
    for (slot, c) in ambient::ambient_at(st, content, loc) {
        if out.len() >= max.max(if chamber { out.len() + 1 } else { 0 }) {
            break;
        }
        let Some(n) = c.named.as_deref() else {
            continue;
        };
        // A passer-by is a stranger until met (lab runs see the name).
        let level = if st
            .player
            .as_ref()
            .is_some_and(|p| !p.knowledge.lab_omniscient)
        {
            Level::Unknown
        } else {
            Level::Detailed
        };
        out.push(PersonRefView {
            id: None,
            ambient_slot: Some(slot),
            name: if level >= Level::Heard {
                format!("{} {}", n.given, n.family)
            } else {
                String::new()
            },
            role_key: occupation_key(engine, Some(&n.occupation)),
            tier: "ambient".into(),
            level: level.into(),
            rumoured: false,
        });
    }
    out
}

/// The player's town's character.
pub fn town_character_view(engine: &Engine, m: usize) -> Option<TownCharacterView> {
    let st = engine.state();
    let content = engine.content();
    let t = st.politics.characters.towns.get(m)?;
    let crisis = t.crisis().and_then(|c| {
        let def = content.crises.iter().find(|d| d.id == c.kind)?;
        let trend = c.trend();
        Some(CrisisView {
            id: def.id.clone(),
            name_key: def.name_key.clone(),
            severity: crate::office::coarse(c.severity as f64, 0.1),
            severity_band: band(
                c.severity as f64,
                &[(0.34, "mild"), (0.67, "serious")],
                "severe",
            ),
            trend: if trend > 0.05 {
                "worsening"
            } else if trend < -0.05 {
                "easing"
            } else {
                "steady"
            }
            .into(),
            since: DateView::of(c.since),
        })
    });
    let hoods: Vec<usize> = st
        .world
        .hoods_of(mandate_sim::world::MunicipalityId(m as u32))
        .collect();
    let town = MunicipalityId(m as u32);
    let gate = Gate::of(engine);
    let concerns_subject = Subject::TownConcerns { town };
    let priorities = gate_concerns(
        gate.level(&concerns_subject),
        crate::office::concerns_for(st, content, town, &hoods),
        true,
        3,
    );
    let broker_subject = Subject::Broker { town };
    let broker_level = gate.level(&broker_subject);
    let broker = t
        .broker
        .filter(|_| broker_level >= Level::Heard)
        .and_then(|id| person_ref(engine, id))
        .map(|p| PersonRefView {
            rumoured: broker_level == Level::Heard,
            ..p
        });
    let plans_subject = Subject::RivalPlans { town };
    let plans = gate.level(&plans_subject);
    let rival_plan = t.rival.filter(|_| plans >= Level::Heard).map(|r| {
        let seat = (plans >= Level::Known)
            .then(|| crate::knowledge::rival_seat(st))
            .flatten();
        let focus_key = (plans >= Level::Detailed)
            .then(|| crate::knowledge::rival_focus(engine, r))
            .flatten();
        RivalPlanView {
            key: match plans {
                Level::Detailed if focus_key.is_some() => "rival-plan-focus",
                Level::Known | Level::Detailed if seat.is_some() => "rival-plan-seat",
                _ => "rival-plan-something",
            }
            .into(),
            ward_number: seat.as_ref().and_then(|s| s.1),
            seat: seat.map(|s| s.0),
            focus_key,
            knowledge: gate.view(&plans_subject),
        }
    });
    Some(TownCharacterView {
        industry: t
            .industry
            .as_deref()
            .and_then(|s| content.sector(s))
            .map(|s| SectorRefView {
                id: s.id.clone(),
                name_key: s.name_key.clone(),
            }),
        industry_share: crate::office::coarse(t.industry_share as f64, 0.05),
        crisis,
        broker,
        broker_knowledge: gate.view(&broker_subject),
        broker_stake_key: (broker_level >= Level::Detailed)
            .then(|| crate::knowledge::broker_stake(engine, town))
            .flatten(),
        // The rival's identity is never gated (DEC-091).
        rival: t.rival.and_then(|id| person_ref(engine, id)),
        rival_plan,
        priorities,
        priorities_knowledge: gate.view(&concerns_subject),
    })
}
