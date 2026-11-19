//! Characters and town character for the client (DEC-031, DEC-077 §1): facts
//! and Fluent keys only — names, roles, coarse personality labels, opinion
//! and trust bands, bonds, favours, and the memories a character would quote.

use crate::office::ConcernView;
use crate::places::{LocationView, location_view};
use crate::views::DateView;
use mandate_sim::characters::{self, CharacterId, MemoryKind, Subject, Tier, ambient};
use mandate_sim::places::Location;
use mandate_sim::politics::Role;
use mandate_sim::politics::elections::Seat;
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
    pub name: String,
    /// Fluent key: `role-councillor`, `role-mayor`, `role-private`,
    /// `role-candidate`, or an occupation's name key.
    pub role_key: String,
    /// `principal` | `named` | `ambient`.
    pub tier: String,
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

/// A character sheet (DEC-031).
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
    pub is_broker: bool,
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
    pub broker: Option<PersonRefView>,
    pub rival: Option<PersonRefView>,
    /// The town's top concerns, strongest first.
    pub priorities: Vec<ConcernView>,
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
    })
}

pub(crate) fn subject_view(engine: &Engine, s: Option<Subject>) -> Option<SubjectView> {
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
    ] {
        if v >= 0.67 {
            personality.push(format!("personality-{k}-high"));
        } else if v <= 0.33 {
            personality.push(format!("personality-{k}-low"));
        }
    }
    let traits = p
        .traits
        .iter()
        .filter_map(|t| content.trait_def(t))
        .map(|t| t.name_key.clone())
        .collect();
    let expertise_key = Some(&p.competencies.expertise)
        .filter(|e| content.attitude_index(e).is_some())
        .map(|e| format!("issue-{e}"));
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
        name: p.full_name(),
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
        is_broker: town.is_some_and(|t| t.broker == Some(id)),
    })
}

/// People at a location: stored characters there, then passers-by.
pub fn people_at(engine: &Engine, loc: Location) -> Vec<PersonRefView> {
    let st = engine.state();
    let content = engine.content();
    let max = content.characters.ambient.max_listed as usize;
    let mut out: Vec<PersonRefView> = ambient::stored_at(&st.politics, st, loc)
        .into_iter()
        .filter_map(|id| person_ref(engine, id))
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
        out.push(PersonRefView {
            id: None,
            ambient_slot: Some(slot),
            name: format!("{} {}", n.given, n.family),
            role_key: occupation_key(engine, Some(&n.occupation)),
            tier: "ambient".into(),
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
    let mut priorities = crate::office::concerns_for(
        st,
        content,
        mandate_sim::world::MunicipalityId(m as u32),
        &hoods,
    );
    priorities.truncate(3);
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
        broker: t.broker.and_then(|id| person_ref(engine, id)),
        rival: t.rival.and_then(|id| person_ref(engine, id)),
        priorities,
    })
}
