//! Knowledge gating for views (DEC-091; M6_ENGINE_CONTRACTS §1.7, §1.8):
//! every view field the player has to learn goes through [`Gate`], which
//! reads the ledger through `knows` (so the lab switch and player-less runs
//! read as `detailed`). Below the level a value needs, the view carries its
//! fallback (a placeholder, a coarse figure, nothing) and the level travels
//! with it, so the client can say "not known yet" and show the lead. The UI
//! never decides what is known.
//!
//! Precision composes with DEC-079's information depth: a place can make a
//! figure exact, knowledge can make it exact, and the stricter of the two
//! wins. Views never write: nothing here touches the state.
//!
//! Never gated (DEC-091): the rules, the player's own state, what an action
//! does and its odds, deadlines, anything aimed at the player (decisions,
//! stories, cases), the standard policy menu, public statistics, elections,
//! and the rival's identity.
//!
//! The notebook ([`notebook_view`]) lists what the player knows, by subject,
//! with where and when it was learned, and the leads.

use crate::office::{ConcernView, SupportView, coarse};
use crate::places::{LocationView, RouteView, location_view};
use crate::views::DateView;
use mandate_sim::characters::routines::DayPart;
use mandate_sim::characters::{self, CharacterId};
use mandate_sim::places::{self, Location};
use mandate_sim::player::knowledge::{
    self as k, Lead, LeadTarget, Learned, Level, PolicyRef, Reading, Sighting, Source, Subject,
};
use mandate_sim::world::{MunicipalityId, NeighborhoodId, PlaceId};
use mandate_sim::{Engine, State};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

/// A knowledge level as views carry it (DEC-091).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeLevelDto {
    Unknown,
    Heard,
    Known,
    Detailed,
}

impl From<Level> for KnowledgeLevelDto {
    fn from(l: Level) -> Self {
        match l {
            Level::Unknown => KnowledgeLevelDto::Unknown,
            Level::Heard => KnowledgeLevelDto::Heard,
            Level::Known => KnowledgeLevelDto::Known,
            Level::Detailed => KnowledgeLevelDto::Detailed,
        }
    }
}

impl KnowledgeLevelDto {
    pub fn key(self) -> &'static str {
        match self {
            KnowledgeLevelDto::Unknown => "unknown",
            KnowledgeLevelDto::Heard => "heard",
            KnowledgeLevelDto::Known => "known",
            KnowledgeLevelDto::Detailed => "detailed",
        }
    }
}

/// A lead: where to go or whom to ask to learn more (guidance, never a
/// requirement). Arguments are names and Fluent keys (`…_key`); the
/// target's carry a `to_` prefix.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LeadView {
    /// What following it teaches about (K5: the notebook files a lead
    /// under its subject's tab).
    pub subject: SubjectDto,
    /// Fluent key (`lead-…`).
    pub key: String,
    pub args: BTreeMap<String, String>,
    /// What following it gives.
    pub level: KnowledgeLevelDto,
    /// Where it points, if that is a place in town.
    pub to: Option<LocationView>,
    /// Auto-routing there (DEC-079 rule 2): action `go`.
    pub route: Option<RouteView>,
    pub since: DateView,
}

/// Where the player saw someone (S1, DEC-031 routines): the last sighting,
/// or a habit (the same place and part of the day on two turns).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SeenView {
    /// Fluent key: `seen-last` ("Last seen at …") or `seen-usually`
    /// ("Usually at … in the afternoon").
    pub key: String,
    /// `place` (the place's name, or the neighbourhood's for its streets),
    /// `kind_key` (`place-<kind>`) and `part_key`.
    pub args: BTreeMap<String, String>,
    pub at: LocationView,
    /// `morning` | `afternoon` | `evening` | `late_night`.
    pub part: String,
    /// Fluent key of the part of the day (`day-part-<part>`).
    pub part_key: String,
    /// The turn it was (its first day).
    pub on: DateView,
}

/// What the player knows of a gated value: its level, the date of the
/// reading it shows (numbers), and the newest lead toward knowing more.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KnowledgeView {
    pub level: KnowledgeLevelDto,
    /// When the dated reading behind a figure was taken, if one was.
    pub as_of: Option<DateView>,
    /// That reading is older than `knowledge.fresh_days`.
    pub stale: bool,
    pub lead: Option<LeadView>,
}

/// The gate views read knowledge through (`Gate::of(engine)`).
pub struct Gate<'a> {
    engine: &'a Engine,
    me: Option<CharacterId>,
}

impl<'a> Gate<'a> {
    pub fn of(engine: &'a Engine) -> Self {
        Gate {
            engine,
            me: characters::player_character(engine.state()),
        }
    }

    fn st(&self) -> &'a State {
        self.engine.state()
    }

    /// The level of a subject (`knows`: `detailed` without a player or with
    /// the lab switch on).
    pub fn level(&self, s: &Subject) -> Level {
        k::knows(self.st(), s)
    }

    pub fn dto(&self, s: &Subject) -> KnowledgeLevelDto {
        self.level(s).into()
    }

    /// The player's town (their zone of local knowledge).
    pub fn town(&self) -> Option<MunicipalityId> {
        self.st().player.as_ref().map(|p| p.municipality)
    }

    /// What the player knows of a person: themselves fully (the player is
    /// never a subject of their own ledger).
    pub fn person(&self, id: CharacterId) -> Level {
        if Some(id) == self.me {
            return Level::Detailed;
        }
        self.level(&Subject::Person { person: id })
    }

    /// A person's name as the player may see it: empty until heard of.
    pub fn name(&self, id: CharacterId, name: String) -> String {
        if self.person(id) >= Level::Heard {
            name
        } else {
            String::new()
        }
    }

    /// The player's town's worries (`detailed` without a player).
    pub fn town_concerns(&self, town: MunicipalityId) -> Level {
        self.level(&Subject::TownConcerns { town })
    }

    pub fn hood_concerns(&self, hood: NeighborhoodId) -> Level {
        self.level(&Subject::HoodConcerns { hood })
    }

    /// Who supports a standard or situational template in a town.
    pub fn support(&self, town: MunicipalityId, template: &str) -> Level {
        self.level(&Subject::PolicySupport {
            town,
            policy: PolicyRef::Template {
                template: template.into(),
            },
        })
    }

    /// The ledger's latest reading of a subject, if the player has one.
    pub fn reading(&self, s: &Subject) -> Option<&'a Reading> {
        self.st()
            .player
            .as_ref()?
            .knowledge
            .entry(s)?
            .reading
            .as_ref()
    }

    /// The newest open lead toward knowing more of a subject.
    pub fn lead(&self, s: &Subject) -> Option<LeadView> {
        let pl = self.st().player.as_ref()?;
        if pl.knowledge.lab_omniscient {
            return None;
        }
        let have = pl.knowledge.level(s);
        pl.knowledge
            .leads
            .iter()
            .rev()
            .find(|l| &l.subject == s && l.level > have)
            .map(|l| lead_view(self, l))
    }

    /// The level, reading date and lead of a subject, for a gated field.
    pub fn view(&self, s: &Subject) -> KnowledgeView {
        let level = self.level(s);
        let reading = self.reading(s);
        KnowledgeView {
            level: level.into(),
            as_of: reading.map(|r| DateView::of(r.on)),
            stale: reading.is_some_and(|r| self.stale(r)),
            lead: if level < Level::Detailed {
                self.lead(s)
            } else {
                None
            },
        }
    }

    /// [`Gate::view`] for a person: the player knows themselves in detail.
    pub fn person_view(&self, id: CharacterId) -> KnowledgeView {
        if Some(id) == self.me {
            return KnowledgeView {
                level: KnowledgeLevelDto::Detailed,
                as_of: None,
                stale: false,
                lead: None,
            };
        }
        self.view(&Subject::Person { person: id })
    }

    fn stale(&self, r: &Reading) -> bool {
        let fresh = self.engine.content().knowledge.fresh_days;
        self.st().calendar.date.0.saturating_sub(r.on.0) > fresh
    }
}

// --- Fallbacks by level (pure; §1.7) ------------------------------------------------

/// Concerns as known: unknown none; heard the top one, unweighted; known
/// the top three, weights to 0.1; detailed all of them, exact where the
/// place allows (`precise`, DEC-079) and to 0.1 elsewhere. `cap` limits
/// the count at every level (the pocket shows two).
pub fn gate_concerns(
    level: Level,
    mut list: Vec<ConcernView>,
    precise: bool,
    cap: usize,
) -> Vec<ConcernView> {
    let n = match level {
        Level::Unknown => 0,
        Level::Heard => 1,
        Level::Known => 3,
        Level::Detailed => usize::MAX,
    };
    list.truncate(n.min(cap));
    for c in &mut list {
        c.weight = match level {
            Level::Unknown | Level::Heard => 0.0,
            Level::Known => coarse(c.weight, 0.1),
            Level::Detailed if precise => c.weight,
            Level::Detailed => coarse(c.weight, 0.1),
        };
    }
    list
}

/// The overall support in a word: `popular`, `divisive` or `unpopular`.
pub fn support_band(overall: f64) -> &'static str {
    if overall > 0.15 {
        "popular"
    } else if overall < -0.15 {
        "unpopular"
    } else {
        "divisive"
    }
}

/// Support as known: below `known` only its band; at `known` by class and
/// overall to 0.1; at `detailed` exact.
pub fn gate_support(level: Level, by_class: [f64; 3], overall: f64) -> SupportView {
    let f = |v: f64| match level {
        Level::Unknown | Level::Heard => None,
        Level::Known => Some(coarse(v, 0.1)),
        Level::Detailed => Some(v),
    };
    SupportView {
        low: f(by_class[0]),
        middle: f(by_class[1]),
        high: f(by_class[2]),
        overall: f(overall),
        band: support_band(overall).into(),
        level: level.into(),
    }
}

/// A bill's support in the player's ward: to 0.25 below `known`, 0.1 at
/// `known`, exact at `detailed`.
pub fn gate_ward_support(level: Level, v: f64) -> f64 {
    match level {
        Level::Unknown | Level::Heard => coarse(v, 0.25),
        Level::Known => coarse(v, 0.1),
        Level::Detailed => v,
    }
}

// --- Hidden places (DEC-090 D7) ----------------------------------------------------------

/// Whether a place is hidden until found. Ordinary places are always known;
/// no generated place is hidden yet (G2 adds them, on `Subject::Place`).
pub fn place_is_hidden(_st: &State, _place: PlaceId) -> bool {
    false
}

/// Whether the town lists a place: always, unless it is hidden and not yet
/// found (`known`; a hint at `heard` lives in the notebook's leads).
pub fn place_listed(gate: &Gate<'_>, st: &State, place: PlaceId) -> bool {
    !place_is_hidden(st, place) || gate.level(&Subject::Place { place }) >= Level::Known
}

// --- Leads ----------------------------------------------------------------------------

/// Where a lead's target is in town, if it is somewhere.
fn target_location(engine: &Engine, to: &LeadTarget) -> Option<Location> {
    let st = engine.state();
    match to {
        LeadTarget::Place { place } => Some(Location::Place { place: *place }),
        LeadTarget::PlaceKind { town, kind } => st
            .world
            .places
            .iter()
            .find(|p| p.municipality == *town && p.kind.key() == kind && p.is_open())
            .map(|p| Location::Place { place: p.id }),
        // Where their daily routine has them now (DEC-031).
        LeadTarget::Person { person } => {
            characters::routines::whereabouts(st, engine.content(), *person)
        }
        LeadTarget::Streets { hood } => Some(Location::Street {
            neighborhood: *hood,
        }),
    }
}

fn place_name(st: &State, place: PlaceId) -> String {
    st.world
        .places
        .get(place.index())
        .map(|p| p.name.clone())
        .unwrap_or_default()
}

fn hood_name(st: &State, hood: NeighborhoodId) -> String {
    st.world
        .neighborhoods
        .get(hood.index())
        .map(|h| h.name.clone())
        .unwrap_or_default()
}

fn town_name(st: &State, town: MunicipalityId) -> String {
    st.world
        .municipalities
        .get(town.index())
        .map(|m| m.name.clone())
        .unwrap_or_default()
}

fn person_name(gate: &Gate<'_>, id: CharacterId) -> String {
    let st = gate.st();
    let name = characters::profile(&st.politics, &st.world, id)
        .map(|p| p.full_name())
        .unwrap_or_default();
    gate.name(id, name)
}

pub(crate) fn lead_view(gate: &Gate<'_>, l: &Lead) -> LeadView {
    let st = gate.st();
    // What the lead is about, as far as the player knows it (at most a name
    // heard): a lead toward the broker must not name them.
    let mut args = subject_args(gate, &l.subject, gate.level(&l.subject).min(Level::Heard));
    match &l.to {
        LeadTarget::Place { place } => {
            args.insert("to_place".into(), place_name(st, *place));
            if let Some(p) = st.world.places.get(place.index()) {
                args.insert("to_kind_key".into(), format!("place-{}", p.kind.key()));
            }
        }
        LeadTarget::PlaceKind { kind, .. } => {
            args.insert("to_kind_key".into(), format!("place-{kind}"));
        }
        LeadTarget::Person { person } => {
            args.insert("to_person".into(), person_name(gate, *person));
            args.insert(
                "to_role_key".into(),
                crate::characters::person_ref(gate.engine, *person)
                    .map(|p| p.role_key)
                    .unwrap_or_default(),
            );
        }
        LeadTarget::Streets { hood } => {
            args.insert("to_hood".into(), hood_name(st, *hood));
        }
    }
    let loc =
        target_location(gate.engine, &l.to).filter(|loc| places::validate_move(st, *loc).is_ok());
    let here = st.player.as_ref().map(|p| p.location);
    LeadView {
        subject: SubjectDto::of(&l.subject),
        key: l.key.clone(),
        args,
        level: l.level.into(),
        to: loc.map(|loc| location_view(st, loc)),
        route: loc.map(|loc| RouteView {
            action: "go".into(),
            late_night: false,
            neighborhood: places::hood_of(st, loc).map(|h| h.0),
            to: Some(location_view(st, loc)),
            here: here == Some(loc),
        }),
        since: DateView::of(l.since),
    }
}

// --- Sightings (S1) ---------------------------------------------------------------------

fn part_key(p: DayPart) -> String {
    format!("day-part-{}", p.key().replace('_', "-"))
}

fn seen_view(st: &State, key: &str, s: Sighting) -> SeenView {
    let mut args = BTreeMap::new();
    let (place, kind) = match s.at {
        Location::Place { place } => (
            place_name(st, place),
            st.world
                .places
                .get(place.index())
                .map(|p| p.kind.key())
                .unwrap_or("home"),
        ),
        Location::Street { neighborhood } => (hood_name(st, neighborhood), "streets"),
        Location::Home => (String::new(), "home"),
    };
    args.insert("place".into(), place);
    args.insert("kind_key".into(), format!("place-{kind}"));
    args.insert("part_key".into(), part_key(s.part));
    SeenView {
        key: key.into(),
        args,
        at: location_view(st, s.at),
        part: s.part.key().into(),
        part_key: part_key(s.part),
        on: DateView::of(s.on),
    }
}

/// Where the player last saw someone.
pub fn last_seen(engine: &Engine, person: CharacterId) -> Option<SeenView> {
    let st = engine.state();
    let s = st.player.as_ref()?.knowledge.last_seen(person)?;
    Some(seen_view(st, "seen-last", s))
}

/// Where someone usually is, as far as the player has seen.
pub fn usually_seen(engine: &Engine, person: CharacterId) -> Option<SeenView> {
    let st = engine.state();
    let s = st.player.as_ref()?.knowledge.usually(person)?;
    Some(seen_view(st, "seen-usually", s))
}

/// A lead from a habit the player has seen: "Usually at the diner around
/// lunch" (`lead-usually-at`), routed there. Derived from the sightings, not
/// stored; following it gives no level by itself.
pub(crate) fn usual_lead(gate: &Gate<'_>, person: CharacterId) -> Option<LeadView> {
    let st = gate.st();
    let pl = st.player.as_ref()?;
    let s = pl.knowledge.usually(person)?;
    let subject = Subject::Person { person };
    let have = gate.level(&subject);
    let mut args = subject_args(gate, &subject, have.min(Level::Heard));
    match s.at {
        Location::Place { place } => {
            args.insert("to_place".into(), place_name(st, place));
            if let Some(p) = st.world.places.get(place.index()) {
                args.insert("to_kind_key".into(), format!("place-{}", p.kind.key()));
            }
        }
        Location::Street { neighborhood } => {
            args.insert("to_hood".into(), hood_name(st, neighborhood));
            args.insert("to_kind_key".into(), "place-streets".into());
        }
        Location::Home => return None,
    }
    args.insert("part_key".into(), part_key(s.part));
    let key = match s.at {
        Location::Street { .. } => "lead-usually-on-streets",
        _ => "lead-usually-at",
    };
    let ok = places::validate_move(st, s.at).is_ok();
    Some(LeadView {
        subject: SubjectDto::of(&subject),
        key: key.into(),
        args,
        level: have.into(),
        to: ok.then(|| location_view(st, s.at)),
        route: ok.then(|| RouteView {
            action: "go".into(),
            late_night: false,
            neighborhood: places::hood_of(st, s.at).map(|h| h.0),
            to: Some(location_view(st, s.at)),
            here: pl.location == s.at,
        }),
        since: DateView::of(s.on),
    })
}

// --- The notebook (§1.8) --------------------------------------------------------------

/// A policy a subject is about, with plain ids.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyRefDto {
    Template { template: String },
    Proposal { proposal: u32 },
}

/// Mirrors the engine's `Subject`, with plain `u32` ids.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SubjectDto {
    TownConcerns { town: u32 },
    HoodConcerns { hood: u32 },
    Broker { town: u32 },
    Person { person: u32 },
    RivalPlans { town: u32 },
    PolicySupport { town: u32, policy: PolicyRefDto },
    Proposal { proposal: u32 },
    Situational { town: u32, template: String },
    Place { place: u32 },
    Layer { layer: String, town: u32 },
    Secret { hidden: u32 },
    Sector { town: u32, sector: String },
    Site { place: u32 },
    Service { place: u32 },
}

impl SubjectDto {
    pub fn of(s: &Subject) -> Self {
        match s {
            Subject::TownConcerns { town } => SubjectDto::TownConcerns { town: town.0 },
            Subject::HoodConcerns { hood } => SubjectDto::HoodConcerns { hood: hood.0 },
            Subject::Broker { town } => SubjectDto::Broker { town: town.0 },
            Subject::Person { person } => SubjectDto::Person { person: person.0 },
            Subject::RivalPlans { town } => SubjectDto::RivalPlans { town: town.0 },
            Subject::PolicySupport { town, policy } => SubjectDto::PolicySupport {
                town: town.0,
                policy: match policy {
                    PolicyRef::Template { template } => PolicyRefDto::Template {
                        template: template.clone(),
                    },
                    PolicyRef::Proposal { proposal } => PolicyRefDto::Proposal {
                        proposal: proposal.0,
                    },
                },
            },
            Subject::Proposal { proposal } => SubjectDto::Proposal {
                proposal: proposal.0,
            },
            Subject::Situational { town, template } => SubjectDto::Situational {
                town: town.0,
                template: template.clone(),
            },
            Subject::Place { place } => SubjectDto::Place { place: place.0 },
            Subject::Layer { layer, town } => SubjectDto::Layer {
                layer: layer.clone(),
                town: town.0,
            },
            Subject::Secret { hidden } => SubjectDto::Secret { hidden: *hidden },
            Subject::Sector { town, sector } => SubjectDto::Sector {
                town: town.0,
                sector: sector.clone(),
            },
            Subject::Site { place } => SubjectDto::Site { place: place.0 },
            Subject::Service { place } => SubjectDto::Service { place: place.0 },
        }
    }
}

/// When and how a level was reached.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LearnedView {
    pub level: KnowledgeLevelDto,
    pub on: DateView,
    /// Fluent key `learned-<source type>`.
    pub key: String,
    pub args: BTreeMap<String, String>,
    /// Learned in a conversation: the exchange (`QueryDto::Exchange`), while
    /// it is in the log (DEC-093: the notebook links what was said).
    pub exchange: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NotebookEntryView {
    pub subject: SubjectDto,
    pub level: KnowledgeLevelDto,
    /// Fluent key `notebook-<subject type>`.
    pub title_key: String,
    /// Names and Fluent keys (`…_key`), as far as the level reveals.
    pub args: BTreeMap<String, String>,
    pub history: Vec<LearnedView>,
    pub as_of: Option<DateView>,
    pub stale: bool,
    /// Learned in play within the current turn and not yet seen in the
    /// notebook (`NotebookSeen`): what the badge counts. What a run starts
    /// with is never new.
    pub new_this_turn: bool,
    /// How to learn more, below `detailed`: an open lead, or where such
    /// things are usually learned (K5). For a person seen at the same place
    /// and time twice: where they usually are (`lead-usually-at`, S1).
    pub lead: Option<LeadView>,
    /// A person: where the player last saw them (S1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub seen: Option<SeenView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NotebookSectionView {
    /// `people` | `neighbourhoods` | `power` | `proposals` | `places` |
    /// `rival` | `support` | `secrets` (Fluent `notebook-section-<key>`).
    pub key: String,
    pub entries: Vec<NotebookEntryView>,
}

/// What the player knows (DEC-091 "the notebook"): only what they know,
/// by subject, never a total that would reveal how much exists.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NotebookView {
    /// The zone's name (the player's town).
    pub zone: Option<String>,
    /// `profile` | `local_option` | `migrated` | `unseeded`.
    pub origin: String,
    /// Non-empty sections, in a fixed order.
    pub sections: Vec<NotebookSectionView>,
    /// Open leads, newest first.
    pub leads: Vec<LeadView>,
    /// Public things not known yet (K5: the notebook's placeholders), each
    /// with a way to learn it. Only what anyone could know exists: the
    /// town's and its neighbourhoods' worries, who runs things, the rival's
    /// plans, support for the standard menu. Never people or hidden places.
    pub gaps: Vec<NotebookGapView>,
}

/// Section keys, in display order.
pub const NOTEBOOK_SECTIONS: [&str; 8] = [
    "people",
    "neighbourhoods",
    "power",
    "proposals",
    "places",
    "rival",
    "support",
    "secrets",
];

fn section_of(s: &Subject) -> &'static str {
    match s {
        Subject::Person { .. } => "people",
        Subject::TownConcerns { .. } | Subject::HoodConcerns { .. } => "neighbourhoods",
        Subject::Broker { .. } => "power",
        Subject::Proposal { .. } | Subject::Situational { .. } => "proposals",
        Subject::Place { .. }
        | Subject::Layer { .. }
        | Subject::Sector { .. }
        | Subject::Site { .. }
        | Subject::Service { .. } => "places",
        Subject::RivalPlans { .. } => "rival",
        Subject::PolicySupport { .. } => "support",
        Subject::Secret { .. } => "secrets",
    }
}

/// Concern name keys, strongest first, as the level reveals them.
fn concern_args(args: &mut BTreeMap<String, String>, level: Level, keys: Vec<String>) {
    let n = match level {
        Level::Unknown => 0,
        Level::Heard => 1,
        _ => 3,
    };
    for (i, key) in keys.into_iter().take(n).enumerate() {
        let name = if i == 0 {
            "concern_key".to_string()
        } else {
            format!("concern{}_key", i + 1)
        };
        args.insert(name, key);
    }
}

/// The rival's plan: the seat they mean to contest (the player's: the
/// mayoralty if the player holds it or filed for it, else their ward).
pub(crate) fn rival_seat(st: &State) -> Option<(String, Option<u32>)> {
    use mandate_sim::politics::elections::Filing;
    let pl = st.player.as_ref()?;
    let me = st.politics.get(pl.politician)?;
    // An office: its constitution id (DTO unchanged until R2); a seat: `ward`.
    match (pl.filing, me.role.post()) {
        (Some(Filing::Retire), _) => None,
        (Some(Filing::Stand { post }), _) | (None, Some(post)) if post.is_office() => {
            Some((post.id().to_string(), None))
        }
        _ => Some((
            "ward".into(),
            st.world
                .constituencies
                .get(pl.ward.index())
                .map(|c| c.number),
        )),
    }
}

/// The issue a rival campaigns on: their strongest conviction (Fluent
/// `issue-<attitude key>`).
pub(crate) fn rival_focus(engine: &Engine, rival: CharacterId) -> Option<String> {
    let st = engine.state();
    let content = engine.content();
    let p = characters::profile(&st.politics, &st.world, rival)?;
    let n = p.values.len().min(content.attitudes.len());
    (0..n)
        .max_by(|a, b| {
            p.values[*a]
                .abs()
                .total_cmp(&p.values[*b].abs())
                .then(b.cmp(a))
        })
        .map(|i| format!("issue-{}", content.attitudes[i].key))
}

/// The broker's stake (§1.1 `detailed`): the town's defining industry.
pub(crate) fn broker_stake(engine: &Engine, town: MunicipalityId) -> Option<String> {
    let st = engine.state();
    let t = st.politics.characters.towns.get(town.index())?;
    t.industry
        .as_deref()
        .and_then(|s| engine.content().sector(s))
        .map(|s| s.name_key.clone())
}

/// What a subject's entry says, as far as `level` reveals.
pub(crate) fn subject_args(gate: &Gate<'_>, s: &Subject, level: Level) -> BTreeMap<String, String> {
    let engine = gate.engine;
    let st = gate.st();
    let content = engine.content();
    let mut args = BTreeMap::new();
    macro_rules! put {
        ($k:expr, $v:expr) => {{
            args.insert(String::from($k), $v);
        }};
    }
    match s {
        Subject::TownConcerns { town } => {
            put!("town", town_name(st, *town));
            // As the town's views rank them (felt across its neighbourhoods).
            let hoods: Vec<usize> = st.world.hoods_of(*town).collect();
            let keys = crate::office::concerns_for(st, content, *town, &hoods)
                .into_iter()
                .map(|c| c.name_key)
                .collect();
            concern_args(&mut args, level, keys);
        }
        Subject::HoodConcerns { hood } => {
            put!("hood", hood_name(st, *hood));
            if let Some(h) = st.world.neighborhoods.get(hood.index()) {
                let keys =
                    crate::office::concerns_for(st, content, h.municipality, &[hood.index()])
                        .into_iter()
                        .map(|c| c.name_key)
                        .collect();
                concern_args(&mut args, level, keys);
            }
        }
        Subject::Broker { town } => {
            put!("town", town_name(st, *town));
            let broker = st
                .politics
                .characters
                .towns
                .get(town.index())
                .and_then(|t| t.broker);
            if level >= Level::Heard
                && let Some(b) = broker
            {
                let name = characters::profile(&st.politics, &st.world, b)
                    .map(|p| p.full_name())
                    .unwrap_or_default();
                put!("name", name);
                put!("rumoured", (level == Level::Heard).to_string());
            }
            if level >= Level::Detailed
                && let Some(stake) = broker_stake(engine, *town)
            {
                put!("stake_key", stake);
            }
        }
        Subject::Person { person } => {
            put!("name", person_name(gate, *person));
            if let Some(p) = crate::characters::person_ref(engine, *person) {
                put!("role_key", p.role_key);
            }
            // K5: who cares about what (their strongest conviction), and
            // whom they are bound to (the one they owe most favours). Both
            // need the person known in detail: meeting someone (a chat
            // teaches them at `known`) tells you who they are, not what
            // they believe (text audit 1, owner report 2026-11-20).
            if level >= Level::Detailed
                && let Some(issue) = rival_focus(engine, *person)
            {
                put!("cares_key", issue);
            }
            if level >= Level::Detailed
                && let Some(other) = owes_most(st, *person)
                && let name = person_name(gate, other)
                && !name.is_empty()
            {
                put!("owes", name);
            }
        }
        Subject::RivalPlans { town } => {
            put!("town", town_name(st, *town));
            let rival = st
                .politics
                .characters
                .towns
                .get(town.index())
                .and_then(|t| t.rival);
            if let Some(r) = rival {
                // The rival's identity is never gated (DEC-091).
                if let Some(p) = characters::profile(&st.politics, &st.world, r) {
                    put!("rival", p.full_name());
                }
                if level >= Level::Known
                    && let Some((seat, ward)) = rival_seat(st)
                {
                    put!("seat_key", format!("rival-seat-{seat}"));
                    if let Some(w) = ward {
                        put!("ward", w.to_string());
                    }
                }
                if level >= Level::Detailed
                    && let Some(f) = rival_focus(engine, r)
                {
                    put!("focus_key", f);
                }
            }
        }
        Subject::PolicySupport { town, policy } => {
            if let PolicyRef::Template { template } = policy
                && let Some(t) = content.policies.get(template)
            {
                put!("policy_key", t.name_key.clone());
                let hoods: Vec<usize> = st.world.hoods_of(*town).collect();
                let overall = mandate_sim::policy::public_support(t, &st.population, &hoods);
                put!(
                    "band_key",
                    format!("support-band-{}", support_band(overall))
                );
                if level >= Level::Known {
                    let v = if level >= Level::Detailed {
                        overall
                    } else {
                        coarse(overall, 0.1)
                    };
                    put!("support", format!("{:.0}", v * 100.0));
                }
            }
            if let PolicyRef::Proposal { proposal } = policy {
                put!("proposal", proposal.0.to_string());
            }
        }
        Subject::Proposal { proposal } => put!("proposal", proposal.0.to_string()),
        Subject::Situational { template, .. } => {
            if let Some(t) = content.policies.get(template) {
                put!("policy_key", t.name_key.clone());
                if let Some(u) = &t.unlock {
                    put!("reason_key", u.reason_key.clone());
                }
            }
        }
        Subject::Place { place } => {
            put!("place", place_name(st, *place));
            if let Some(p) = st.world.places.get(place.index()) {
                put!("kind_key", format!("place-{}", p.kind.key()));
            }
        }
        Subject::Layer { layer, town } => {
            put!("layer", layer.clone());
            put!("town", town_name(st, *town));
        }
        Subject::Secret { hidden } => put!("hidden", hidden.to_string()),
        // Places with purpose (PL1): what a visit taught, in bands.
        Subject::Sector { town, sector } => {
            put!("town", town_name(st, *town));
            if let Some(d) = content.sector(sector) {
                put!("sector_key", d.name_key.clone());
            }
            let outlook = if level >= Level::Known {
                mandate_sim::visits::outlook_band(st, content, *town, sector)
            } else {
                "unknown"
            };
            put!("outlook", outlook.to_string());
        }
        Subject::Site { place } => {
            put!("place", place_name(st, *place));
            let r = mandate_sim::visits::site_reading(st, *place);
            let get = |k: &str| r.iter().find(|x| x.0 == k).map(|x| x.1).unwrap_or(0);
            let band = |v: i32, lo: i32, hi: i32| {
                if v >= hi {
                    "high"
                } else if v >= lo {
                    "some"
                } else {
                    "low"
                }
            };
            if level >= Level::Known {
                put!("pollution", band(get("pollution"), 400, 900).to_string());
                let hazard = get("flood").max(get("wildfire")).max(get("storm"));
                put!("hazard", band(hazard, 300, 650).to_string());
            }
        }
        Subject::Service { place } => {
            put!("place", place_name(st, *place));
            if level >= Level::Known {
                let r = mandate_sim::visits::service_reading(st, content, *place);
                let need = r.iter().map(|x| x.1).max().unwrap_or(0);
                let band = if need >= 450 {
                    "high"
                } else if need >= 200 {
                    "some"
                } else {
                    "low"
                };
                put!("need", band.to_string());
            }
        }
    }
    args
}

fn source_args(gate: &Gate<'_>, src: &Source) -> BTreeMap<String, String> {
    let engine = gate.engine;
    let st = gate.st();
    let mut args = BTreeMap::new();
    macro_rules! put {
        ($k:expr, $v:expr) => {{
            args.insert(String::from($k), $v);
        }};
    }
    macro_rules! place {
        ($p:expr) => {{
            let p: PlaceId = $p;
            put!("place", place_name(st, p));
            if let Some(x) = st.world.places.get(p.index()) {
                put!("kind_key", format!("place-{}", x.kind.key()));
            }
        }};
    }
    match src {
        Source::Start { profile } => put!("profile", profile.clone()),
        Source::LocalOption | Source::Migrated | Source::Lab => {}
        Source::Primer { party } => {
            if let Some(p) = party {
                put!("party", p.0.to_string());
            }
        }
        Source::Visit { place: p } => place!(*p),
        Source::Overheard { place: p, line } => {
            place!(*p);
            put!("line", line.clone());
        }
        Source::Action { action, place: p } => {
            put!("action_key", format!("action-{}", action.replace('_', "-")));
            if let Some(p) = p {
                place!(*p);
            }
        }
        Source::Met { person } => put!("person", person_name(gate, *person)),
        Source::Service { place: p, service } => {
            place!(*p);
            put!("service_key", format!("service-{service}"));
        }
        Source::Story { story } => put!("story", story.to_string()),
        Source::Decision { decision } => {
            put!("decision", decision.0.to_string());
            if let Some(key) = st
                .narrative
                .decision(*decision)
                .and_then(|d| engine.content().dilemma(&d.template))
                .map(|d| d.title_key.clone())
            {
                put!("title_key", key);
            }
        }
        Source::Council { policy } => {
            if let Some(key) = st
                .policy
                .instances
                .get(policy.index())
                .and_then(|p| {
                    mandate_sim::policy::resolve(engine.content(), &engine.state().policy, p)
                })
                .map(|t| t.name_key.clone())
            {
                put!("policy_key", key);
            }
        }
        Source::Sponsor { proposal, person } => {
            put!("proposal", proposal.0.to_string());
            put!("person", person_name(gate, *person));
        }
        Source::Talk { person, topic } => {
            put!("person", person_name(gate, *person));
            put!("topic", topic.clone());
        }
    }
    args
}

fn learned_view(gate: &Gate<'_>, s: &Subject, l: &Learned) -> LearnedView {
    LearnedView {
        level: l.level.into(),
        on: DateView::of(l.on),
        key: format!("learned-{}", l.source.key().replace('_', "-")),
        args: source_args(gate, &l.source),
        exchange: match &l.source {
            Source::Talk { person, topic } => {
                crate::talk::exchange_for(gate.st(), *person, topic, l.on, s)
            }
            _ => None,
        },
    }
}

/// Whether a date falls within the current turn (the days since the
/// previous turn's date, today included).
fn this_turn(engine: &Engine, on: mandate_core::SimDate) -> bool {
    let today = engine.state().calendar.date.0;
    on.0 + engine.turn_days().max(1) > today && on.0 <= today
}

/// How many subjects the player learned something of this turn (the
/// pocket's notebook badge). What the player starts with (the start
/// profile, the local-knowledge option, the party primer) and what a save
/// migration seeded is not news: only what was learned in play counts.
/// Seeing the notebook (`NotebookSeen`) clears it (K5).
pub fn notebook_new(engine: &Engine) -> u32 {
    let Some(pl) = engine.state().player.as_ref() else {
        return 0;
    };
    pl.knowledge
        .entries
        .values()
        .filter(|e| is_new(engine, e))
        .count() as u32
}

/// Learned in play this turn, above what the player has seen.
fn is_new(engine: &Engine, e: &k::Entry) -> bool {
    let starting = |s: &Source| {
        matches!(
            s,
            Source::Start { .. } | Source::LocalOption | Source::Primer { .. } | Source::Migrated
        )
    };
    e.learned
        .iter()
        .any(|l| l.level > e.seen && !starting(&l.source) && this_turn(engine, l.on))
}

/// The notebook: what the player knows, by subject (§1.8).
pub fn notebook_view(engine: &Engine) -> Option<NotebookView> {
    let st = engine.state();
    let pl = st.player.as_ref()?;
    let gate = Gate::of(engine);
    let ledger = &pl.knowledge;
    let mut sections: BTreeMap<&'static str, Vec<NotebookEntryView>> = BTreeMap::new();
    for (s, e) in &ledger.entries {
        if e.level == Level::Unknown {
            continue;
        }
        let stale = e.reading.as_ref().is_some_and(|r| gate.stale(r));
        sections
            .entry(section_of(s))
            .or_default()
            .push(NotebookEntryView {
                subject: SubjectDto::of(s),
                level: e.level.into(),
                title_key: format!("notebook-{}", s.key().replace('_', "-")),
                args: subject_args(&gate, s, e.level),
                history: e
                    .learned
                    .iter()
                    .map(|l| learned_view(&gate, s, l))
                    .collect(),
                as_of: e.reading.as_ref().map(|r| DateView::of(r.on)),
                stale,
                new_this_turn: is_new(engine, e),
                lead: match s {
                    Subject::Person { person } => usual_lead(&gate, *person),
                    _ => None,
                }
                .or_else(|| next_lead(&gate, s, e.level)),
                seen: match s {
                    Subject::Person { person } => last_seen(engine, *person),
                    _ => None,
                },
            });
    }
    Some(NotebookView {
        zone: ledger.zone.map(|z| match z {
            k::Zone::Town { town } => town_name(st, town),
        }),
        origin: ledger.origin.key().into(),
        sections: NOTEBOOK_SECTIONS
            .iter()
            .filter_map(|key| {
                sections.remove(key).map(|entries| NotebookSectionView {
                    key: (*key).into(),
                    entries,
                })
            })
            .collect(),
        leads: ledger
            .leads
            .iter()
            .rev()
            .map(|l| lead_view(&gate, l))
            .collect(),
        gaps: gaps(&gate),
    })
}

// --- Gaps and standing leads (K5) -------------------------------------------------------

/// Something public the player does not know yet, with a way to learn it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NotebookGapView {
    pub subject: SubjectDto,
    /// The section it would join (`neighbourhoods`, `power`, `rival`, `support`).
    pub section: String,
    /// Fluent key `notebook-<subject type>`, with the subject's public args.
    pub title_key: String,
    pub args: BTreeMap<String, String>,
    pub lead: Option<LeadView>,
}

/// The one a person owes the most favours to, other than the player.
fn owes_most(st: &State, person: CharacterId) -> Option<CharacterId> {
    let me = characters::player_character(st);
    let mut by: BTreeMap<CharacterId, u32> = BTreeMap::new();
    for f in &st.politics.characters.favours {
        if f.debtor == person && Some(f.creditor) != me {
            *by.entry(f.creditor).or_default() += f.count as u32;
        }
    }
    by.into_iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|(c, _)| c)
}

/// The first of these place kinds the town has open.
fn kind_in_town(st: &State, town: MunicipalityId, kinds: &[&str]) -> Option<String> {
    kinds
        .iter()
        .find(|k| {
            st.world
                .places
                .iter()
                .any(|p| p.municipality == town && p.kind.key() == **k && p.is_open())
        })
        .map(|k| (*k).to_string())
}

/// An open place in town offering a service.
fn service_in_town(engine: &Engine, town: MunicipalityId, service: &str) -> Option<PlaceId> {
    engine
        .state()
        .world
        .places
        .iter()
        .find(|p| {
            p.municipality == town
                && p.is_open()
                && engine
                    .content()
                    .place_kind(p.kind.key())
                    .is_some_and(|d| places::kind_offers(d, places::Offer::Service(service)))
        })
        .map(|p| p.id)
}

/// Where a subject is usually learned (§1.3), when no lead is open: the
/// streets of a neighbourhood for its worries (canvassing), the locals at
/// the diner or market for the town's worries and who runs things, the
/// party office's polls for the rival's plans and for support. A standing
/// lead is guidance read from the rules, never stored, and only below
/// `known`.
fn standing_lead(gate: &Gate<'_>, s: &Subject, have: Level) -> Option<Lead> {
    const LOCALS: [&str; 3] = ["diner", "market", "community_centre"];
    let st = gate.st();
    let engine = gate.engine;
    if have >= Level::Known {
        return None;
    }
    let (key, level, to) = match s {
        Subject::HoodConcerns { hood } => (
            "lead-notebook-canvass".to_string(),
            Level::Known,
            LeadTarget::Streets { hood: *hood },
        ),
        Subject::TownConcerns { town } => (
            "lead-notebook-worries".to_string(),
            have.next(),
            LeadTarget::PlaceKind {
                town: *town,
                kind: kind_in_town(st, *town, &LOCALS)?,
            },
        ),
        Subject::Broker { town } => {
            let kind = kind_in_town(st, *town, &LOCALS)?;
            (
                format!("lead-ask-{}", kind.replace('_', "-")),
                have.next(),
                LeadTarget::PlaceKind { town: *town, kind },
            )
        }
        Subject::RivalPlans { town } => (
            "lead-notebook-rival".to_string(),
            have.next(),
            LeadTarget::Place {
                place: service_in_town(engine, *town, "internal_polls")?,
            },
        ),
        Subject::PolicySupport { town, .. } => (
            "lead-notebook-polls".to_string(),
            Level::Known,
            LeadTarget::Place {
                place: service_in_town(engine, *town, "internal_polls")?,
            },
        ),
        _ => return None,
    };
    Some(Lead {
        subject: s.clone(),
        level,
        key,
        to,
        since: st.calendar.date,
        source: Source::Lab,
    })
}

/// The way to learn more of a subject: its newest open lead, else the
/// standing one. None at `detailed` or with the lab switch on.
fn next_lead(gate: &Gate<'_>, s: &Subject, have: Level) -> Option<LeadView> {
    if have >= Level::Detailed || gate.st().player.as_ref()?.knowledge.lab_omniscient {
        return None;
    }
    gate.lead(s)
        .or_else(|| standing_lead(gate, s, have).map(|l| lead_view(gate, &l)))
}

/// The notebook's placeholders: public subjects of the player's town not
/// known at all yet, in section order.
fn gaps(gate: &Gate<'_>) -> Vec<NotebookGapView> {
    let st = gate.st();
    let content = gate.engine.content();
    let Some(town) = gate.town() else {
        return Vec::new();
    };
    let mut subjects = vec![Subject::TownConcerns { town }];
    subjects.extend(st.world.hoods_of(town).map(|h| Subject::HoodConcerns {
        hood: NeighborhoodId(h as u32),
    }));
    subjects.push(Subject::Broker { town });
    // The rival's identity is public (DEC-091); their plans are not.
    if st
        .politics
        .characters
        .towns
        .get(town.index())
        .is_some_and(|t| t.rival.is_some())
    {
        subjects.push(Subject::RivalPlans { town });
    }
    // The standard menu (as `GrantSubject::PolicySupport` resolves it).
    for t in content.policies.values() {
        if t.tier == mandate_content::TierKey::Local && t.unlock.is_none() {
            subjects.push(Subject::PolicySupport {
                town,
                policy: PolicyRef::Template {
                    template: t.id.clone(),
                },
            });
        }
    }
    subjects
        .into_iter()
        .filter(|s| gate.level(s) == Level::Unknown)
        .map(|s| NotebookGapView {
            subject: SubjectDto::of(&s),
            section: section_of(&s).into(),
            title_key: format!("notebook-{}", s.key().replace('_', "-")),
            args: subject_args(gate, &s, Level::Unknown),
            lead: next_lead(gate, &s, Level::Unknown),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(id: &str, w: f64) -> ConcernView {
        ConcernView {
            id: id.into(),
            name_key: format!("{id}-key"),
            weight: w,
        }
    }

    fn list() -> Vec<ConcernView> {
        vec![c("a", 0.93), c("b", 0.71), c("c", 0.44), c("d", 0.12)]
    }

    #[test]
    fn concerns_fall_back_by_level() {
        assert!(gate_concerns(Level::Unknown, list(), true, 9).is_empty());
        let h = gate_concerns(Level::Heard, list(), true, 9);
        assert_eq!(h.len(), 1);
        assert_eq!((h[0].id.as_str(), h[0].weight), ("a", 0.0));
        let k = gate_concerns(Level::Known, list(), true, 9);
        assert_eq!(k.len(), 3);
        assert!((k[0].weight - 0.9).abs() < 1e-9);
        let d = gate_concerns(Level::Detailed, list(), true, 9);
        assert_eq!(d.len(), 4);
        assert_eq!(d[0].weight, 0.93);
        // The stricter wins: detailed knowledge away from the desk is coarse.
        let away = gate_concerns(Level::Detailed, list(), false, 9);
        assert!((away[0].weight - 0.9).abs() < 1e-9);
        // The pocket's cap holds at every level.
        assert_eq!(gate_concerns(Level::Detailed, list(), true, 2).len(), 2);
    }

    #[test]
    fn support_falls_back_by_level() {
        for l in [Level::Unknown, Level::Heard] {
            let s = gate_support(l, [0.33, -0.21, 0.05], 0.27);
            assert_eq!(
                (s.low, s.middle, s.high, s.overall),
                (None, None, None, None)
            );
            assert_eq!(s.band, "popular");
        }
        let s = gate_support(Level::Known, [0.33, -0.21, 0.05], 0.27);
        assert!((s.low.unwrap() - 0.3).abs() < 1e-9);
        assert!((s.middle.unwrap() + 0.2).abs() < 1e-9);
        assert!((s.overall.unwrap() - 0.3).abs() < 1e-9);
        let s = gate_support(Level::Detailed, [0.33, -0.21, 0.05], -0.27);
        assert_eq!(s.low, Some(0.33));
        assert_eq!(s.overall, Some(-0.27));
        assert_eq!(s.band, "unpopular");
        assert_eq!(
            gate_support(Level::Detailed, [0.0; 3], 0.1).band,
            "divisive"
        );
        assert_eq!(gate_ward_support(Level::Unknown, 0.4), 0.5);
        assert_eq!(gate_ward_support(Level::Heard, 0.1), 0.0);
        assert!((gate_ward_support(Level::Known, 0.43) - 0.4).abs() < 1e-9);
        assert_eq!(gate_ward_support(Level::Detailed, 0.43), 0.43);
    }

    #[test]
    fn level_dto_is_ordered_and_snake_case() {
        assert!(KnowledgeLevelDto::Unknown < KnowledgeLevelDto::Detailed);
        assert_eq!(
            serde_json::to_string(&KnowledgeLevelDto::from(Level::Heard)).unwrap(),
            r#""heard""#
        );
    }
}
