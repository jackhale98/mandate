//! The knowledge ledger (DEC-091; M6_ENGINE_CONTRACTS §1): what the player
//! knows of each subject (a town's or a neighbourhood's worries, the power
//! broker, a person, a proposal, a hidden place), when and how each level was
//! reached, dated readings of numbers, and leads.
//!
//! It is player-domain state. The only writers are the methods of
//! [`KnowledgeLedger`]: the player's own commands call them directly, other
//! systems post `Effect::Learn`, `Effect::Read` and `Effect::Lead`, which
//! [`apply_effect`] applies (OVERVIEW §4.3). Views and bots read it through
//! [`knows`] and [`Knowing`], never the ledger itself, so the lab switch and
//! player-less runs read as omniscient.
//!
//! The ledger draws no randomness: every write is a function of commands and
//! state (DEC-041). In K1 nothing reads it for outcomes yet.

use crate::characters::CharacterId;
use crate::narrative::DecisionId;
use crate::places::Location;
use crate::policy::PolicyInstanceId;
use crate::politics::PartyId;
use crate::state::State;
use crate::world::{ConstituencyId, MunicipalityId, NeighborhoodId, PlaceId};
use mandate_content::knowledge::{GrantDef, GrantSubject};
use mandate_content::{ContentBundle, ContentKey, TierKey};
use mandate_core::{SimDate, define_id};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub use mandate_content::knowledge::Level;

define_id!(
    /// An open policy-family proposal (DEC-092). Declared here until the
    /// policy domain's families (F2b) own it; the serialized form (a plain
    /// `u32`) does not change when it moves.
    FamilyInstanceId
);

/// Something the player can know. Ordered (a `BTreeMap` key, DEC-041); typed ids.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Subject {
    TownConcerns {
        town: MunicipalityId,
    },
    HoodConcerns {
        hood: NeighborhoodId,
    },
    Broker {
        town: MunicipalityId,
    },
    Person {
        person: CharacterId,
    },
    RivalPlans {
        town: MunicipalityId,
    },
    PolicySupport {
        town: MunicipalityId,
        policy: PolicyRef,
    },
    Proposal {
        proposal: FamilyInstanceId,
    },
    /// Transitional (until the M5d situational templates become families).
    Situational {
        town: MunicipalityId,
        template: ContentKey,
    },
    Place {
        place: PlaceId,
    },
    Layer {
        layer: ContentKey,
        town: MunicipalityId,
    },
    /// A hidden fact (DEC-034). Raw `u32` until hidden facts get a typed id.
    Secret {
        hidden: u32,
    },
    /// How an industry (a sector id) is doing in a town's economy: taking
    /// people on, steady, or letting them go (places with purpose, PL1).
    Sector {
        town: MunicipalityId,
        sector: ContentKey,
    },
    /// Pollution and hazards at a place (a works, a farm).
    Site {
        place: PlaceId,
    },
    /// The real need a place's service meets (DEC-092 §2.22): its
    /// neighbourhood's felt level of the concerns it addresses.
    Service {
        place: PlaceId,
    },
}

impl Subject {
    /// Snake-case type key (Fluent suffixes: `notebook-<key>`).
    pub fn key(&self) -> &'static str {
        match self {
            Subject::TownConcerns { .. } => "town_concerns",
            Subject::HoodConcerns { .. } => "hood_concerns",
            Subject::Broker { .. } => "broker",
            Subject::Person { .. } => "person",
            Subject::RivalPlans { .. } => "rival_plans",
            Subject::PolicySupport { .. } => "policy_support",
            Subject::Proposal { .. } => "proposal",
            Subject::Situational { .. } => "situational",
            Subject::Place { .. } => "place",
            Subject::Layer { .. } => "layer",
            Subject::Secret { .. } => "secret",
            Subject::Sector { .. } => "sector",
            Subject::Site { .. } => "site",
            Subject::Service { .. } => "service",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyRef {
    Template { template: ContentKey },
    Proposal { proposal: FamilyInstanceId },
}

/// How something was learned. Facts only: the UI words it (`learned-<type>`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Source {
    Start {
        profile: ContentKey,
    },
    LocalOption,
    Primer {
        party: Option<PartyId>,
    },
    Migrated,
    Visit {
        place: PlaceId,
    },
    Overheard {
        place: PlaceId,
        line: ContentKey,
    },
    /// A player action (`canvass`, `lobby`, `town_hall`, …), where it was done.
    Action {
        action: String,
        place: Option<PlaceId>,
    },
    Met {
        person: CharacterId,
    },
    Service {
        place: PlaceId,
        service: String,
    },
    Story {
        story: u32,
    },
    Decision {
        decision: DecisionId,
    },
    Council {
        policy: PolicyInstanceId,
    },
    Sponsor {
        proposal: FamilyInstanceId,
        person: CharacterId,
    },
    /// A conversation (DEC-093).
    Talk {
        person: CharacterId,
        topic: ContentKey,
    },
    Lab,
}

impl Source {
    /// Snake-case type key (Fluent suffixes: `learned-<key>`).
    pub fn key(&self) -> &'static str {
        match self {
            Source::Start { .. } => "start",
            Source::LocalOption => "local_option",
            Source::Primer { .. } => "primer",
            Source::Migrated => "migrated",
            Source::Visit { .. } => "visit",
            Source::Overheard { .. } => "overheard",
            Source::Action { .. } => "action",
            Source::Met { .. } => "met",
            Source::Service { .. } => "service",
            Source::Story { .. } => "story",
            Source::Decision { .. } => "decision",
            Source::Council { .. } => "council",
            Source::Sponsor { .. } => "sponsor",
            Source::Talk { .. } => "talk",
            Source::Lab => "lab",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Learned {
    pub level: Level,
    pub on: SimDate,
    pub source: Source,
}

/// A dated reading of a numeric subject: values quantised to per mille, by
/// key (a concern id, `low`/`middle`/`high`, a neighbourhood id as a string).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reading {
    pub on: SimDate,
    pub source: Source,
    pub values: Vec<(String, i32)>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub level: Level,
    /// The first time each level was reached, lowest first (at most three).
    pub learned: Vec<Learned>,
    /// The latest reading, for numeric subjects at `detailed`.
    pub reading: Option<Reading>,
    /// The level the player has seen in the notebook (K5): levels reached
    /// above it are new. Left out of the save while `unknown`.
    #[serde(default, skip_serializing_if = "is_unknown")]
    pub seen: Level,
}

fn is_unknown(l: &Level) -> bool {
    *l == Level::Unknown
}

/// Where a lead points. A lead is guidance, never a requirement.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LeadTarget {
    Place { place: PlaceId },
    PlaceKind { town: MunicipalityId, kind: String },
    Person { person: CharacterId },
    Streets { hood: NeighborhoodId },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lead {
    pub subject: Subject,
    /// The level following the lead gives.
    pub level: Level,
    /// Fluent key (`lead-…`); its arguments come from the subject and target.
    pub key: String,
    pub to: LeadTarget,
    pub since: SimDate,
    pub source: Source,
}

/// A zone of local knowledge. Region and Nation arrive with M7–M8.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Zone {
    Town { town: MunicipalityId },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LedgerOrigin {
    /// Not seeded yet (the serde default: older saves are seeded on load).
    #[default]
    Unseeded,
    Profile {
        profile: ContentKey,
    },
    /// The "Start with local knowledge" option (DEC-091, off by default).
    LocalOption {
        profile: ContentKey,
    },
    Migrated,
}

impl LedgerOrigin {
    pub fn key(&self) -> &'static str {
        match self {
            LedgerOrigin::Unseeded => "unseeded",
            LedgerOrigin::Profile { .. } => "profile",
            LedgerOrigin::LocalOption { .. } => "local_option",
            LedgerOrigin::Migrated => "migrated",
        }
    }
}

/// A resolved grant: one subject raised to a level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    pub subject: Subject,
    pub level: Level,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeLedger {
    pub origin: LedgerOrigin,
    pub zone: Option<Zone>,
    #[serde(with = "mandate_core::serde_util::map_as_pairs")]
    pub entries: BTreeMap<Subject, Entry>,
    /// Newest last; capped (`knowledge.leads_cap`), oldest dropped first.
    pub leads: Vec<Lead>,
    /// Places whose first-visit introduction has been given.
    pub visited: BTreeSet<PlaceId>,
    /// Lab only: every subject reads as `detailed` (baselines). Never set by the UI.
    #[serde(default)]
    pub lab_omniscient: bool,
    /// Where the player has seen people (S1, DEC-031 routines): the last
    /// few sightings of each, oldest first. Left out of the save while empty.
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "mandate_core::serde_util::map_as_pairs"
    )]
    pub seen: BTreeMap<CharacterId, Vec<Sighting>>,
}

/// The player saw someone somewhere: the turn (its first day), the part of
/// the day, and the location.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sighting {
    pub on: SimDate,
    pub part: crate::characters::routines::DayPart,
    pub at: Location,
}

/// Sightings kept per person (the newest).
pub const SIGHTINGS_KEPT: usize = 4;

impl KnowledgeLedger {
    /// The level recorded for a subject (`Unknown` if none). Views and bots
    /// use [`knows`] instead, which honours the lab switch.
    pub fn level(&self, s: &Subject) -> Level {
        self.entries.get(s).map(|e| e.level).unwrap_or_default()
    }

    pub fn entry(&self, s: &Subject) -> Option<&Entry> {
        self.entries.get(s)
    }

    /// Raise to `level` if higher (never lowers). Records `Learned` for each
    /// newly reached level, drops leads the new level satisfies. Returns
    /// whether anything changed.
    pub fn learn(&mut self, s: Subject, level: Level, on: SimDate, src: Source) -> bool {
        let current = self.level(&s);
        if level <= current {
            return false;
        }
        self.leads.retain(|l| !(l.subject == s && l.level <= level));
        let e = self.entries.entry(s).or_default();
        let mut l = current;
        while l < level {
            l = l.next();
            e.learned.push(Learned {
                level: l,
                on,
                source: src.clone(),
            });
        }
        e.level = level;
        true
    }

    /// Store a dated reading; requires the subject at `known` or above and
    /// raises it to `detailed` (numbers need the identity first). Returns
    /// whether the reading was stored.
    pub fn read(&mut self, s: Subject, r: Reading) -> bool {
        if self.level(&s) < Level::Known {
            return false;
        }
        self.learn(s.clone(), Level::Detailed, r.on, r.source.clone());
        if let Some(e) = self.entries.get_mut(&s) {
            e.reading = Some(r);
        }
        true
    }

    /// Add a lead unless the subject is already at its level or the same
    /// (subject, level) lead exists; enforce the cap (oldest dropped first).
    /// Returns whether the lead was added.
    pub fn lead(&mut self, l: Lead, cap: usize) -> bool {
        if self.level(&l.subject) >= l.level
            || self
                .leads
                .iter()
                .any(|x| x.subject == l.subject && x.level == l.level)
        {
            return false;
        }
        self.leads.push(l);
        let over = self.leads.len().saturating_sub(cap);
        self.leads.drain(..over);
        true
    }

    /// Enter a new zone: set it and apply the primer's grants. Nothing is
    /// deleted (subjects are keyed by town, so a new town's start unknown).
    pub fn enter_zone(&mut self, z: Zone, grants: &[Grant], on: SimDate, src: Source) {
        self.zone = Some(z);
        for g in grants {
            self.learn(g.subject.clone(), g.level, on, src.clone());
        }
    }

    /// The player has looked at the notebook: every entry is seen at its
    /// current level (no longer new). Returns how many entries changed.
    pub fn mark_seen(&mut self) -> usize {
        let mut n = 0;
        for e in self.entries.values_mut() {
            if e.seen < e.level {
                e.seen = e.level;
                n += 1;
            }
        }
        n
    }

    /// Record a place's first visit. Returns whether it was the first.
    pub fn visit(&mut self, place: PlaceId) -> bool {
        self.visited.insert(place)
    }

    /// Record seeing someone (once per turn, part and location). Returns
    /// whether it was new.
    pub fn saw(&mut self, person: CharacterId, s: Sighting) -> bool {
        let v = self.seen.entry(person).or_default();
        if v.contains(&s) {
            return false;
        }
        v.push(s);
        let over = v.len().saturating_sub(SIGHTINGS_KEPT);
        v.drain(..over);
        true
    }

    /// Where the player last saw someone.
    pub fn last_seen(&self, person: CharacterId) -> Option<Sighting> {
        self.seen.get(&person).and_then(|v| v.last()).copied()
    }

    /// Where someone usually is, as far as the player has seen: a place and
    /// part of the day they were seen at on two different turns (the most
    /// recent such habit).
    pub fn usually(&self, person: CharacterId) -> Option<Sighting> {
        let v = self.seen.get(&person)?;
        v.iter()
            .rev()
            .find(|s| {
                v.iter()
                    .any(|o| o.at == s.at && o.part == s.part && o.on != s.on)
            })
            .copied()
    }

    fn apply(&mut self, grants: &[Grant], on: SimDate, src: &Source) {
        for g in grants {
            self.learn(g.subject.clone(), g.level, on, src.clone());
        }
    }
}

/// What the player knows of a subject; `Detailed` when there is no player
/// (lab runs) or the lab switch is on. Views and bots use this, never the
/// ledger directly.
pub fn knows(state: &State, s: &Subject) -> Level {
    level_of(state.player.as_ref(), s)
}

// --- Resolving grants ------------------------------------------------------------

/// What a grant's selectors refer to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope {
    pub town: MunicipalityId,
    /// The player's home neighbourhood (`home`).
    pub home: Option<NeighborhoodId>,
    /// The player's ward (`ward`).
    pub ward: Option<ConstituencyId>,
    /// The place being introduced (`here`).
    pub here: Option<PlaceId>,
}

impl Scope {
    /// The player's town, home and ward.
    pub fn of_player(state: &State) -> Option<Scope> {
        let pl = state.player.as_ref()?;
        Some(Scope {
            town: pl.municipality,
            home: crate::places::home_neighborhood(state),
            ward: Some(pl.ward),
            here: None,
        })
    }
}

/// Resolve content grants against the live state: every subject a grant
/// names, in content order, then subject order within a grant. The player
/// is never a subject of their own ledger.
pub fn resolve(
    state: &State,
    content: &ContentBundle,
    defs: &[GrantDef],
    scope: &Scope,
) -> Vec<Grant> {
    let me = crate::characters::player_character(state);
    let town = scope.town;
    let hoods_of_town = || -> Vec<NeighborhoodId> {
        state
            .world
            .hoods_of(town)
            .map(|h| NeighborhoodId(h as u32))
            .collect()
    };
    let here_place = scope
        .here
        .and_then(|p| state.world.places.get(p.index()))
        .filter(|p| p.municipality == town);
    // Who a place is known for (its regulars), whatever the time of day.
    let people_at = |place: PlaceId| -> Vec<CharacterId> {
        crate::characters::ambient::regulars_at(&state.politics, state, Location::Place { place })
    };
    let mut out = Vec::new();
    for g in defs {
        let mut subjects: Vec<Subject> = Vec::new();
        match g.subject {
            GrantSubject::TownConcerns => subjects.push(Subject::TownConcerns { town }),
            GrantSubject::RivalPlans => subjects.push(Subject::RivalPlans { town }),
            GrantSubject::HoodConcerns => {
                let hoods: Vec<NeighborhoodId> = match g.which.as_str() {
                    "home" => scope.home.into_iter().collect(),
                    "ward" => scope
                        .ward
                        .map(|w| {
                            crate::policy::ward_hoods(&state.world, w)
                                .into_iter()
                                .map(|h| NeighborhoodId(h as u32))
                                .collect()
                        })
                        .unwrap_or_default(),
                    "here" => here_place.map(|p| p.neighborhood).into_iter().collect(),
                    _ => hoods_of_town(),
                };
                subjects.extend(hoods.into_iter().map(|hood| Subject::HoodConcerns { hood }));
            }
            GrantSubject::Broker => {
                let broker = state
                    .politics
                    .characters
                    .towns
                    .get(town.index())
                    .and_then(|t| t.broker);
                let ok = match g.which.as_str() {
                    // Only if the broker is found at the place.
                    "here" => here_place
                        .zip(broker)
                        .is_some_and(|(p, b)| people_at(p.id).contains(&b)),
                    _ => true,
                };
                if ok {
                    subjects.push(Subject::Broker { town });
                }
            }
            GrantSubject::Council => {
                // Everyone holding the town's local-tier posts (DEC-098).
                {
                    let at = crate::world::Jurisdiction::town(town);
                    let people = crate::politics::officials_at(&state.politics, at)
                        .into_iter()
                        .filter_map(|id| state.politics.get(id))
                        .map(|p| p.character);
                    for person in people {
                        if Some(person) != me {
                            subjects.push(Subject::Person { person });
                        }
                    }
                }
            }
            GrantSubject::PeopleAt => {
                let places: Vec<PlaceId> = match g.which.as_str() {
                    "here" => here_place.map(|p| p.id).into_iter().collect(),
                    "town" => state
                        .world
                        .places
                        .iter()
                        .filter(|p| p.municipality == town)
                        .map(|p| p.id)
                        .collect(),
                    kind => state
                        .world
                        .places
                        .iter()
                        .filter(|p| p.municipality == town && p.kind.key() == kind)
                        .map(|p| p.id)
                        .collect(),
                };
                for p in places {
                    for person in people_at(p) {
                        if Some(person) != me {
                            subjects.push(Subject::Person { person });
                        }
                    }
                }
            }
            GrantSubject::PolicySupport => {
                // The standard menu: local templates that are always on the table.
                for t in content.policies.values() {
                    if t.tier == TierKey::Local && t.unlock.is_none() {
                        subjects.push(Subject::PolicySupport {
                            town,
                            policy: PolicyRef::Template {
                                template: t.id.clone(),
                            },
                        });
                    }
                }
            }
            GrantSubject::Sector => {
                let sector = here_place
                    .and_then(|p| content.place_kind(p.kind.key()))
                    .and_then(|k| k.sector.clone());
                if let (Some(p), Some(sector)) = (here_place, sector) {
                    subjects.push(Subject::Sector {
                        town: p.municipality,
                        sector,
                    });
                }
            }
            GrantSubject::Site => {
                subjects.extend(here_place.map(|p| Subject::Site { place: p.id }));
            }
            GrantSubject::Service => {
                let serves = here_place
                    .and_then(|p| content.place_kind(p.kind.key()))
                    .is_some_and(|k| !k.addresses.is_empty());
                if serves {
                    subjects.extend(here_place.map(|p| Subject::Service { place: p.id }));
                }
            }
            GrantSubject::Proposals => {
                // Situational templates on the table in the town (until F4
                // turns them into families; open family proposals join here).
                for t in content.policies.values() {
                    if t.tier == TierKey::Local
                        && t.unlock.is_some()
                        && crate::policy::unlocked(state, content, t, town.0)
                    {
                        subjects.push(Subject::Situational {
                            town,
                            template: t.id.clone(),
                        });
                    }
                }
            }
        }
        out.extend(subjects.into_iter().map(|subject| Grant {
            subject,
            level: g.level,
        }));
    }
    out
}

// --- Seeding --------------------------------------------------------------------

/// Seed the player's ledger as play begins (`Engine::mark_started`): the
/// zone is the player's town, then the start profile's grants (`None`: the
/// content's `default_start`), then the local option's if it is on. Replaces
/// whatever the ledger held, keeping the lab switch. Does nothing without a
/// player; an unknown profile is an error.
pub fn seed(
    state: &mut State,
    content: &ContentBundle,
    profile: Option<&str>,
    local_option: bool,
) -> Result<(), String> {
    let k = &content.knowledge;
    let id = profile.unwrap_or(&k.default_start);
    let Some(start) = k.start(id) else {
        return Err(format!("unknown start profile `{id}`"));
    };
    let Some(scope) = Scope::of_player(state) else {
        return Ok(());
    };
    let profile_grants = resolve(state, content, &start.grants, &scope);
    let option_grants = if local_option {
        resolve(state, content, &k.local_option, &scope)
    } else {
        Vec::new()
    };
    let on = state.calendar.date;
    let Some(pl) = state.player.as_mut() else {
        return Ok(());
    };
    let lab = pl.knowledge.lab_omniscient;
    let mut ledger = KnowledgeLedger {
        origin: if local_option {
            LedgerOrigin::LocalOption {
                profile: start.id.clone(),
            }
        } else {
            LedgerOrigin::Profile {
                profile: start.id.clone(),
            }
        },
        zone: Some(Zone::Town { town: scope.town }),
        lab_omniscient: lab,
        ..Default::default()
    };
    ledger.apply(
        &profile_grants,
        on,
        &Source::Start {
            profile: start.id.clone(),
        },
    );
    ledger.apply(&option_grants, on, &Source::LocalOption);
    pl.knowledge = ledger;
    Ok(())
}

/// On load (`Engine::from_state`): a run from before the ledger (its origin
/// `Unseeded`) is seeded generously as `Migrated`: the default profile plus
/// the local option, plus `person known` for everyone with a relationship
/// edge to the player or a memory of them. A run in progress never loses
/// what its player plainly knew. A seeded ledger is left as it is.
pub fn on_load(state: &mut State, content: &ContentBundle) {
    let Some(pl) = state.player.as_ref() else {
        return;
    };
    if pl.knowledge.origin != LedgerOrigin::Unseeded {
        return;
    }
    let Some(scope) = Scope::of_player(state) else {
        return;
    };
    let k = &content.knowledge;
    let mut grants = k
        .start(&k.default_start)
        .map(|s| resolve(state, content, &s.grants, &scope))
        .unwrap_or_default();
    grants.extend(resolve(state, content, &k.local_option, &scope));
    if let Some(me) = crate::characters::player_character(state) {
        let ch = &state.politics.characters;
        let mut people: BTreeSet<CharacterId> = BTreeSet::new();
        for (holder, other) in ch.edges.keys() {
            if *other == me.0 {
                people.insert(CharacterId(*holder));
            } else if *holder == me.0 {
                people.insert(CharacterId(*other));
            }
        }
        for c in &ch.people {
            if c.id != me && c.memories.iter().any(|m| m.actor == me) {
                people.insert(c.id);
            }
        }
        people.remove(&me);
        grants.extend(people.into_iter().map(|person| Grant {
            subject: Subject::Person { person },
            level: Level::Known,
        }));
    }
    let on = state.calendar.date;
    let Some(pl) = state.player.as_mut() else {
        return;
    };
    let lab = pl.knowledge.lab_omniscient;
    let mut ledger = KnowledgeLedger {
        origin: LedgerOrigin::Migrated,
        zone: Some(Zone::Town { town: scope.town }),
        lab_omniscient: lab,
        ..Default::default()
    };
    ledger.apply(&grants, on, &Source::Migrated);
    pl.knowledge = ledger;
}

/// The party contact's primer for a zone, resolved against its town.
pub fn primer(state: &State, content: &ContentBundle, z: Zone) -> Vec<Grant> {
    let Zone::Town { town } = z;
    let scope = Scope {
        town,
        home: None,
        ward: None,
        here: None,
    };
    resolve(state, content, &content.knowledge.primer, &scope)
}

/// Move the player's knowledge into a new zone with the party primer
/// (DEC-091). M6 has no zone change in play: this is the API M7 uses, the
/// lab drives it, and nothing is deleted.
pub fn enter_zone(state: &mut State, content: &ContentBundle, z: Zone, party: Option<PartyId>) {
    let grants = primer(state, content, z);
    let on = state.calendar.date;
    if let Some(pl) = state.player.as_mut() {
        pl.knowledge
            .enter_zone(z, &grants, on, Source::Primer { party });
    }
}

/// A place's first-visit introduction (for the arrival producer, K3): the
/// grants of `visit_all` and of the place kind's `visit_intro`, resolved for
/// that place. Empty for a place outside the player's zone town.
pub fn visit_intro(state: &State, content: &ContentBundle, place: PlaceId) -> Vec<Grant> {
    let Some(p) = state.world.places.get(place.index()) else {
        return Vec::new();
    };
    let Some(mut scope) = Scope::of_player(state) else {
        return Vec::new();
    };
    scope.town = p.municipality;
    scope.here = Some(place);
    let k = &content.knowledge;
    let mut out = resolve(state, content, &k.visit_all, &scope);
    if let Some(defs) = k.visit_intro.get(p.kind.key()) {
        out.extend(resolve(state, content, defs, &scope));
    }
    out
}

// --- Producers (K3; M6_ENGINE_CONTRACTS §1.3) ---------------------------------------
//
// The player's own commands write the ledger directly through these (the
// player domain writing its own state, Commands phase). Other systems post
// `Effect::Learn` / `Read` / `Lead`, and only when a player exists. Nothing
// here draws randomness, and nothing reads the ledger for outcomes.

/// What a player knows of a subject (`knows` for a player that may be
/// absent): `Detailed` without a player or with the lab switch on.
pub fn level_of(player: Option<&crate::player::PlayerState>, s: &Subject) -> Level {
    match player {
        None => Level::Detailed,
        Some(p) if p.knowledge.lab_omniscient => Level::Detailed,
        Some(p) => p.knowledge.level(s),
    }
}

/// Raise each grant in the player's ledger (dated today, from `src`).
/// Returns how many subjects changed; nothing without a player.
pub fn teach(state: &mut State, grants: impl IntoIterator<Item = Grant>, src: &Source) -> usize {
    let on = state.calendar.date;
    let Some(pl) = state.player.as_mut() else {
        return 0;
    };
    grants
        .into_iter()
        .filter(|g| {
            pl.knowledge
                .learn(g.subject.clone(), g.level, on, src.clone())
        })
        .count()
}

/// Raise one subject in the player's ledger. Returns whether it changed.
pub fn teach_one(state: &mut State, subject: Subject, level: Level, src: Source) -> bool {
    teach(state, [Grant { subject, level }], &src) > 0
}

/// Store a dated reading for a subject the player already knows (raising
/// it to `detailed`). Returns whether it was stored.
pub fn read_now(
    state: &mut State,
    subject: Subject,
    values: Vec<(String, i32)>,
    src: Source,
) -> bool {
    let on = state.calendar.date;
    let Some(pl) = state.player.as_mut() else {
        return false;
    };
    pl.knowledge.read(
        subject,
        Reading {
            on,
            source: src,
            values,
        },
    )
}

/// A neighbourhood's worries as felt there now, per mille by concern id,
/// strongest first (a `hood_concerns` reading).
pub fn hood_reading(
    state: &State,
    content: &ContentBundle,
    hood: NeighborhoodId,
) -> Vec<(String, i32)> {
    let Some(h) = state.world.neighborhoods.get(hood.index()) else {
        return Vec::new();
    };
    crate::concerns::felt_concerns(
        &state.population,
        content,
        h.municipality.index(),
        &[hood.index()],
    )
    .into_iter()
    .filter_map(|(i, w)| {
        let c = content.concerns.get(i)?;
        Some((c.id.clone(), (w * 1000.0).round() as i32))
    })
    .collect()
}

/// A neighbourhood's worries learned first-hand (canvassing, a town hall):
/// `known` the first time; once known, a dated reading (`detailed`).
pub fn learn_hood(state: &mut State, content: &ContentBundle, hood: NeighborhoodId, src: Source) {
    let s = Subject::HoodConcerns { hood };
    let current = state.player.as_ref().map(|p| p.knowledge.level(&s));
    match current {
        None => {}
        Some(l) if l >= Level::Known => {
            let values = hood_reading(state, content, hood);
            read_now(state, s, values, src);
        }
        Some(_) => {
            teach_one(state, s, Level::Known, src);
        }
    }
}

/// The player's place, if they stand at one (the `place` of a source).
pub fn here(state: &State) -> Option<PlaceId> {
    match state.player.as_ref()?.location {
        Location::Place { place } => Some(place),
        _ => None,
    }
}

/// Where a player action was done (`Source::Action`).
pub fn action_source(state: &State, action: &str) -> Source {
    Source::Action {
        action: action.into(),
        place: here(state),
    }
}

/// Everyone holding a post in the player's focus (the members of its
/// bodies, then its office holders), as people (not the player; DEC-098).
pub fn council_people(state: &State) -> Vec<CharacterId> {
    let me = crate::characters::player_character(state);
    let pol = &state.politics;
    state
        .focus()
        .into_iter()
        .flat_map(|at| crate::politics::officials_at(pol, at))
        .filter_map(|id| pol.get(id))
        .map(|p| p.character)
        .filter(|c| Some(*c) != me)
        .collect()
}

/// Arriving at a place (`MoveTo`, or a `Go` landing there): the first visit
/// in the zone gives the place's introduction (`visit_all` and the kind's
/// `visit_intro`), once. Arrival teaches lightly (the owner, 2026-11-19):
/// introductions and names; anything deeper needs an action or a person.
/// Returns how many subjects changed.
/// The player notes who is where they are now (on arriving, after a
/// conversation): a sighting of each stored character present. Returns how
/// many were new.
pub fn note_sightings(state: &mut State, content: &ContentBundle) -> usize {
    let Some(at) = state.player.as_ref().map(|p| p.location) else {
        return 0;
    };
    let now = crate::characters::routines::Now::of(state, content);
    let me = crate::characters::player_character(state);
    let here: Vec<CharacterId> = crate::characters::routines::present(state, content, &now, at)
        .into_iter()
        .filter(|c| Some(*c) != me)
        .collect();
    let s = Sighting {
        on: now.date,
        part: now.part,
        at,
    };
    let Some(pl) = state.player.as_mut() else {
        return 0;
    };
    here.into_iter().filter(|c| pl.knowledge.saw(*c, s)).count()
}

pub fn arrive_at(state: &mut State, content: &ContentBundle, place: PlaceId) -> usize {
    let first = match state.player.as_mut() {
        Some(pl) => pl.knowledge.visit(place),
        None => return 0,
    };
    if !first {
        return 0;
    }
    let intro = visit_intro(state, content, place);
    teach(state, intro, &Source::Visit { place })
}

// --- Effects (OVERVIEW §4.3) ------------------------------------------------------

/// Apply one knowledge effect request to the player's ledger (called by
/// `player::effects`, the player domain). Returns whether the ledger changed.
pub fn apply_effect(
    ledger: &mut KnowledgeLedger,
    e: crate::scheduler::Effect,
    on: SimDate,
    content: &ContentBundle,
) -> bool {
    use crate::scheduler::Effect;
    match e {
        Effect::Learn {
            subject,
            level,
            source,
        } => ledger.learn(subject, level, on, source),
        Effect::Read { subject, reading } => ledger.read(subject, reading),
        Effect::Lead { lead } => ledger.lead(lead, content.knowledge.leads_cap as usize),
        _ => false,
    }
}

// --- The read interface for bots --------------------------------------------------

/// Gated information for strategy code (lab bots play as the player): each
/// method returns `None` below the level the matching view would need, so a
/// bot never acts on what the player could not see. Truth decides outcomes;
/// this only decides what may be aimed at.
pub struct Knowing<'a> {
    state: &'a State,
    content: &'a ContentBundle,
}

impl<'a> Knowing<'a> {
    pub fn new(state: &'a State, content: &'a ContentBundle) -> Self {
        Knowing { state, content }
    }

    pub fn of(engine: &'a crate::Engine) -> Self {
        Knowing::new(engine.state(), engine.content())
    }

    /// The level of a subject (as `knows`).
    pub fn level(&self, s: &Subject) -> Level {
        knows(self.state, s)
    }

    /// The player's town (the zone of local knowledge).
    pub fn town(&self) -> Option<MunicipalityId> {
        self.state.player.as_ref().map(|p| p.municipality)
    }

    fn ranked(&self, level: Level, mut keys: Vec<ContentKey>) -> Option<Vec<ContentKey>> {
        match level {
            Level::Unknown => None,
            Level::Heard => {
                keys.truncate(1);
                Some(keys)
            }
            Level::Known => {
                keys.truncate(3);
                Some(keys)
            }
            Level::Detailed => Some(keys),
        }
    }

    /// The town's worries, strongest first: the top one at `heard`, three
    /// at `known`, all at `detailed`.
    pub fn town_concerns(&self) -> Option<Vec<ContentKey>> {
        let town = self.town()?;
        let level = self.level(&Subject::TownConcerns { town });
        let keys = crate::concerns::top_concerns(&self.state.population, town.index())
            .into_iter()
            .filter_map(|(i, _)| self.content.concerns.get(i).map(|c| c.id.clone()))
            .collect();
        self.ranked(level, keys)
    }

    /// A neighbourhood's worries, strongest first, as felt there: the top
    /// one at `heard`, three at `known`, all at `detailed`.
    pub fn concerns_of(&self, hood: NeighborhoodId) -> Option<Vec<ContentKey>> {
        let h = self.state.world.neighborhoods.get(hood.index())?;
        let level = self.level(&Subject::HoodConcerns { hood });
        let keys = crate::concerns::felt_concerns(
            &self.state.population,
            self.content,
            h.municipality.index(),
            &[hood.index()],
        )
        .into_iter()
        .filter_map(|(i, _)| self.content.concerns.get(i).map(|c| c.id.clone()))
        .collect();
        self.ranked(level, keys)
    }

    /// Who really runs the player's town, once confirmed (`known`).
    pub fn broker(&self) -> Option<CharacterId> {
        let town = self.town()?;
        if self.level(&Subject::Broker { town }) < Level::Known {
            return None;
        }
        self.state
            .politics
            .characters
            .towns
            .get(town.index())
            .and_then(|t| t.broker)
    }

    /// A person's goals (goal content ids, DEC-032), at `detailed`.
    pub fn goals_of(&self, person: CharacterId) -> Option<Vec<ContentKey>> {
        if self.level(&Subject::Person { person }) < Level::Detailed {
            return None;
        }
        Some(
            self.state
                .politics
                .characters
                .goals
                .iter()
                .filter(|g| g.holder == person)
                .map(|g| g.goal.clone())
                .collect(),
        )
    }

    /// Town-wide support for a policy (−1..1): to 0.1 at `known`, exact at
    /// `detailed`. Family proposals read `None` until F2b.
    pub fn support_of(&self, policy: &PolicyRef) -> Option<f64> {
        let town = self.town()?;
        let level = self.level(&Subject::PolicySupport {
            town,
            policy: policy.clone(),
        });
        if level < Level::Known {
            return None;
        }
        let PolicyRef::Template { template } = policy else {
            return None;
        };
        let t = self.content.policies.get(template)?;
        let hoods: Vec<usize> = self.state.world.hoods_of(town).collect();
        let v = crate::policy::public_support(t, &self.state.population, &hoods);
        Some(if level >= Level::Detailed {
            v
        } else {
            (v * 10.0).round() / 10.0
        })
    }

    /// A policy's support among the player's ward voters (−1..1), as the bill
    /// view shows it: to 0.25 below `known`, 0.1 at `known`, exact at
    /// `detailed`. Family proposals read `None` until F2b.
    pub fn ward_support_of(&self, policy: &PolicyRef) -> Option<f64> {
        let pl = self.state.player.as_ref()?;
        let PolicyRef::Template { template } = policy else {
            return None;
        };
        let t = self.content.policies.get(template)?;
        let level = self.level(&Subject::PolicySupport {
            town: pl.municipality,
            policy: policy.clone(),
        });
        let hoods = crate::policy::ward_hoods(&self.state.world, pl.ward);
        let v = crate::policy::public_support(t, &self.state.population, &hoods);
        let coarse = |step: f64| (v / step).round() * step;
        Some(match level {
            Level::Unknown | Level::Heard => coarse(0.25),
            Level::Known => coarse(0.1),
            Level::Detailed => v,
        })
    }

    /// Whether a local template is on the player's menu: the standard menu
    /// is never gated; a situational template must be on the table and seen
    /// properly (`known`, as the catalogue shows it).
    pub fn on_menu(&self, template: &mandate_content::PolicyTemplate) -> bool {
        if template.tier != TierKey::Local {
            return false;
        }
        if template.unlock.is_none() {
            return true;
        }
        let Some(town) = self.town() else {
            return false;
        };
        self.proposals().contains(&Subject::Situational {
            town,
            template: template.id.clone(),
        })
    }

    /// Whether the player has had a place's first-visit introduction (their
    /// own memory; the lab switch does not pretend they have been).
    pub fn visited(&self, place: PlaceId) -> bool {
        self.state
            .player
            .as_ref()
            .is_some_and(|p| p.knowledge.visited.contains(&place))
    }

    /// How well the player knows a person.
    pub fn person(&self, person: CharacterId) -> Level {
        self.level(&Subject::Person { person })
    }

    /// The worry a neighbourhood feels most, once heard of.
    pub fn top_worry(&self, hood: NeighborhoodId) -> Option<ContentKey> {
        self.concerns_of(hood).and_then(|v| v.into_iter().next())
    }

    /// The proposals the player has seen properly (`known`), in subject order.
    pub fn proposals(&self) -> Vec<Subject> {
        let Some(town) = self.town() else {
            return Vec::new();
        };
        // Every situational template on the table (open families join in F2b).
        let on_table = self
            .content
            .policies
            .values()
            .filter(|t| {
                t.tier == TierKey::Local
                    && t.unlock.is_some()
                    && crate::policy::unlocked(self.state, self.content, t, town.0)
            })
            .map(|t| Subject::Situational {
                town,
                template: t.id.clone(),
            });
        match self.state.player.as_ref() {
            Some(p) if !p.knowledge.lab_omniscient => {
                let mut v: Vec<Subject> = on_table
                    .filter(|s| p.knowledge.level(s) >= Level::Known)
                    .collect();
                v.extend(
                    p.knowledge
                        .entries
                        .iter()
                        .filter(|(s, e)| {
                            matches!(s, Subject::Proposal { .. }) && e.level >= Level::Known
                        })
                        .map(|(s, _)| s.clone()),
                );
                v.sort();
                v
            }
            _ => on_table.collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(n: u32) -> SimDate {
        SimDate(n)
    }

    fn person(n: u32) -> Subject {
        Subject::Person {
            person: CharacterId(n),
        }
    }

    fn lead(s: Subject, level: Level, since: u32) -> Lead {
        Lead {
            subject: s,
            level,
            key: "lead-test".into(),
            to: LeadTarget::Person {
                person: CharacterId(9),
            },
            since: d(since),
            source: Source::Lab,
        }
    }

    #[test]
    fn levels_only_rise_and_record_each_step() {
        let mut k = KnowledgeLedger::default();
        assert_eq!(k.level(&person(1)), Level::Unknown);
        assert!(k.learn(person(1), Level::Known, d(5), Source::Lab));
        assert!(!k.learn(person(1), Level::Heard, d(6), Source::Migrated));
        assert!(!k.learn(person(1), Level::Known, d(6), Source::Migrated));
        assert_eq!(k.level(&person(1)), Level::Known);
        let e = k.entry(&person(1)).unwrap();
        assert_eq!(
            e.learned.iter().map(|l| l.level).collect::<Vec<_>>(),
            vec![Level::Heard, Level::Known]
        );
        assert!(k.learn(person(1), Level::Detailed, d(9), Source::Migrated));
        let e = k.entry(&person(1)).unwrap();
        assert_eq!(e.learned.len(), 3);
        assert_eq!(e.learned[2].on, d(9));
        assert_eq!(e.learned[2].source, Source::Migrated);
        assert!(!k.learn(person(2), Level::Unknown, d(9), Source::Lab));
        assert!(k.entry(&person(2)).is_none());
    }

    #[test]
    fn readings_need_the_identity_first() {
        let mut k = KnowledgeLedger::default();
        let s = Subject::HoodConcerns {
            hood: NeighborhoodId(3),
        };
        let r = Reading {
            on: d(10),
            source: Source::Lab,
            values: vec![("concern.jobs".into(), 640)],
        };
        assert!(!k.read(s.clone(), r.clone()));
        k.learn(s.clone(), Level::Heard, d(1), Source::Lab);
        assert!(!k.read(s.clone(), r.clone()));
        k.learn(s.clone(), Level::Known, d(2), Source::Lab);
        assert!(k.read(s.clone(), r.clone()));
        assert_eq!(k.level(&s), Level::Detailed);
        assert_eq!(k.entry(&s).unwrap().reading.as_ref(), Some(&r));
        // A later reading replaces the number; the level history stays.
        let r2 = Reading { on: d(40), ..r };
        assert!(k.read(s.clone(), r2.clone()));
        assert_eq!(k.entry(&s).unwrap().reading.as_ref(), Some(&r2));
        assert_eq!(k.entry(&s).unwrap().learned.len(), 3);
    }

    #[test]
    fn leads_are_unique_capped_and_dropped_when_satisfied() {
        let mut k = KnowledgeLedger::default();
        assert!(k.lead(lead(person(1), Level::Known, 1), 3));
        assert!(!k.lead(lead(person(1), Level::Known, 2), 3), "duplicate");
        assert!(k.lead(lead(person(1), Level::Detailed, 2), 3));
        k.learn(person(2), Level::Known, d(1), Source::Lab);
        assert!(
            !k.lead(lead(person(2), Level::Heard, 3), 3),
            "already known"
        );
        assert!(k.lead(lead(person(3), Level::Heard, 4), 3));
        assert!(k.lead(lead(person(4), Level::Heard, 5), 3));
        assert_eq!(k.leads.len(), 3, "the oldest is dropped");
        assert_eq!(k.leads[0].subject, person(1));
        assert_eq!(k.leads[0].level, Level::Detailed);
        // Learning satisfies the leads at or below the new level.
        k.learn(person(1), Level::Detailed, d(6), Source::Lab);
        assert!(k.leads.iter().all(|l| l.subject != person(1)));
        assert_eq!(k.leads.len(), 2);
    }

    #[test]
    fn entering_a_zone_keeps_what_was_known() {
        let mut k = KnowledgeLedger::default();
        let a = MunicipalityId(0);
        let b = MunicipalityId(1);
        k.zone = Some(Zone::Town { town: a });
        k.learn(Subject::Broker { town: a }, Level::Known, d(1), Source::Lab);
        let primer = [Grant {
            subject: Subject::Broker { town: b },
            level: Level::Heard,
        }];
        k.enter_zone(
            Zone::Town { town: b },
            &primer,
            d(2),
            Source::Primer { party: None },
        );
        assert_eq!(k.zone, Some(Zone::Town { town: b }));
        assert_eq!(k.level(&Subject::Broker { town: a }), Level::Known);
        assert_eq!(k.level(&Subject::Broker { town: b }), Level::Heard);
    }

    #[test]
    fn seeing_the_notebook_marks_each_entry_at_its_level() {
        let mut k = KnowledgeLedger::default();
        k.learn(person(1), Level::Heard, d(1), Source::Lab);
        k.learn(person(2), Level::Known, d(1), Source::Lab);
        assert_eq!(k.entry(&person(1)).unwrap().seen, Level::Unknown);
        // Unseen entries serialize as before (no `seen` field).
        assert!(!serde_json::to_string(&k).unwrap().contains("seen"));
        assert_eq!(k.mark_seen(), 2);
        assert_eq!(k.mark_seen(), 0);
        assert_eq!(k.entry(&person(2)).unwrap().seen, Level::Known);
        // Learning more makes it new again, above what was seen.
        k.learn(person(1), Level::Known, d(2), Source::Lab);
        let e = k.entry(&person(1)).unwrap();
        assert_eq!((e.seen, e.level), (Level::Heard, Level::Known));
        let back: KnowledgeLedger =
            serde_json::from_str(&serde_json::to_string(&k).unwrap()).unwrap();
        assert_eq!(back, k);
    }

    #[test]
    fn the_ledger_round_trips_through_json_in_order() {
        let mut k = KnowledgeLedger {
            origin: LedgerOrigin::Profile {
                profile: "start.x".into(),
            },
            zone: Some(Zone::Town {
                town: MunicipalityId(2),
            }),
            ..Default::default()
        };
        k.learn(
            person(5),
            Level::Known,
            d(1),
            Source::Met {
                person: CharacterId(5),
            },
        );
        k.learn(
            Subject::PolicySupport {
                town: MunicipalityId(2),
                policy: PolicyRef::Template {
                    template: "policy.transit".into(),
                },
            },
            Level::Heard,
            d(1),
            Source::Start {
                profile: "start.x".into(),
            },
        );
        k.learn(
            Subject::Secret { hidden: 7 },
            Level::Heard,
            d(3),
            Source::Story { story: 4 },
        );
        k.lead(lead(person(6), Level::Known, 3), 24);
        k.visit(PlaceId(11));
        let j = serde_json::to_string(&k).unwrap();
        let back: KnowledgeLedger = serde_json::from_str(&j).unwrap();
        assert_eq!(back, k);
        // The serde default is an unseeded, empty ledger (older saves).
        let empty: KnowledgeLedger = serde_json::from_str(
            r#"{"origin":{"type":"unseeded"},"zone":null,"entries":[],"leads":[],"visited":[]}"#,
        )
        .unwrap();
        assert_eq!(empty, KnowledgeLedger::default());
    }
}
