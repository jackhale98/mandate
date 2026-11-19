//! Characters (DEC-031, DEC-018): people worth remembering, with
//! personality, values, competencies, relationships, favours, and memory.
//!
//! **Tiers.** Politicians are Principal characters while they hold office or
//! stand for it, as are organisation leaders and a town's power broker and
//! rival; other stored characters are Named (shopkeepers, organisers,
//! journalists, …, from the occupation registry). Ambient people are
//! generated on demand from a place's cohorts and never stored; meeting one
//! promotes them to Named (`ambient`).
//!
//! **Politicians are characters.** A politician's identity, values,
//! ambition, integrity, oratory, and negotiation stay on `Politician`, where
//! elections, councils, and the NPC AI read them; their `Character` record
//! holds what DEC-031 adds (warmth, boldness, loyalty, traits, the other
//! competencies, memory). `profile` gives one resolved view of anyone.
//! Characters live inside the politics domain (`PoliticsState::characters`):
//! the politics systems create politicians (and so characters) and read
//! relationships on their hot paths (council leans, lobbying odds).
//!
//! **Relationships** are directed edges (holder → other) in a `BTreeMap`:
//! opinion (−100..100), trust (0..1), and typed bonds. A pair without an edge
//! has the holder's first impression as its opinion. The **favour ledger**
//! records who owes whom, how many, and over what.
//!
//! **Memory** is bounded per tier. A memory's salience is
//! `magnitude × e^(−decay × years)`; faded memories are forgotten (their
//! effect stays in the relationship's opinion), strong ones persist.
//!
//! **Determinism.** Generation uses keyed RNG (seed, entity, period), never a
//! sequential stream, so no existing draw shifts. Named characters update
//! lazily, one keyed step per year, so the result does not depend on when
//! they are evaluated (DEC-018).

pub mod ambient;
pub mod goals;
pub mod town;

pub use town::{ActiveCrisis, TownCharacter};

use crate::politics::elections::Seat;
use crate::politics::{Politician, PoliticianId, PoliticsState, Role};
use crate::population::PopulationState;
use crate::world::{PlaceId, World};
use mandate_content::{CharacterParams, ContentBundle, ContentKey, TraitDef};
use mandate_core::math::{clamp, exp};
use mandate_core::{IdAllocator, SimDate, SimRng, Stream, define_id};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

define_id!(
    /// A stored character (Principal or Named).
    CharacterId
);

/// Keyed-RNG tags (DEC-041): one per kind of draw, never reused.
pub(crate) const KEY_POLITICIAN: u64 = 0xC4A1;
pub(crate) const KEY_NAMED: u64 = 0xC4A2;
pub(crate) const KEY_DRIFT: u64 = 0xC4A3;
pub(crate) const KEY_AMBIENT: u64 = 0xC4A4;

pub(crate) fn rng(seed: u64, parts: &[u64]) -> SimRng {
    mandate_core::rng::keyed(seed, Stream::Npc, parts)
}

/// A stable `u64` for a content id (keys draws by id, not registry position).
pub(crate) fn key_of(id: &str) -> u64 {
    mandate_core::hash::derive_u64(id, &[])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Principal,
    Named,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bond {
    Family,
    Friend,
    Ally,
    Rival,
    Mentor,
    Employer,
    Patron,
}

impl Bond {
    pub fn key(self) -> &'static str {
        match self {
            Bond::Family => "family",
            Bond::Friend => "friend",
            Bond::Ally => "ally",
            Bond::Rival => "rival",
            Bond::Mentor => "mentor",
            Bond::Employer => "employer",
            Bond::Patron => "patron",
        }
    }
}

/// The kinds of memory the engine records (`mandate_content::MEMORY_KINDS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    BackedMyBill,
    OpposedMyBill,
    PromisedFavour,
    KeptPromise,
    BrokePromise,
    BrokeRanks,
    LobbiedMe,
    PressedMe,
    DefeatedMe,
    LostToMe,
    VetoedMyBill,
    SignedMyBill,
    Met,
    Scandal,
    /// Decisions (DEC-077 §4): the player took their side, the other side,
    /// or left them waiting.
    StoodWithMe,
    StoodAgainstMe,
    IgnoredMe,
    /// The player pinned their own wrongdoing on them (a betrayal).
    BlamedMe,
}

impl MemoryKind {
    /// Content key (`characters.memories`), e.g. `broke_promise`.
    pub fn key(self) -> &'static str {
        match self {
            MemoryKind::BackedMyBill => "backed_my_bill",
            MemoryKind::OpposedMyBill => "opposed_my_bill",
            MemoryKind::PromisedFavour => "promised_favour",
            MemoryKind::KeptPromise => "kept_promise",
            MemoryKind::BrokePromise => "broke_promise",
            MemoryKind::BrokeRanks => "broke_ranks",
            MemoryKind::LobbiedMe => "lobbied_me",
            MemoryKind::PressedMe => "pressed_me",
            MemoryKind::DefeatedMe => "defeated_me",
            MemoryKind::LostToMe => "lost_to_me",
            MemoryKind::VetoedMyBill => "vetoed_my_bill",
            MemoryKind::SignedMyBill => "signed_my_bill",
            MemoryKind::Met => "met",
            MemoryKind::Scandal => "scandal",
            MemoryKind::StoodWithMe => "stood_with_me",
            MemoryKind::StoodAgainstMe => "stood_against_me",
            MemoryKind::IgnoredMe => "ignored_me",
            MemoryKind::BlamedMe => "blamed_me",
        }
    }

    /// Fluent key, e.g. `memory-broke-promise`.
    pub fn fluent_key(self) -> String {
        format!("memory-{}", self.key().replace('_', "-"))
    }
}

/// What a memory or favour is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Subject {
    /// A policy instance (a bill or a law).
    Policy { id: u32 },
    /// An office: a ward seat or a mayoralty.
    Seat { seat: Seat },
    /// A decision the player took (`narrative::DecisionId`).
    Decision { id: u32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    pub kind: MemoryKind,
    /// Who did it (whom the memory is about).
    pub actor: CharacterId,
    pub subject: Option<Subject>,
    /// −1..1.
    pub sentiment: f32,
    /// Salience when fresh (0..1).
    pub magnitude: f32,
    pub date: SimDate,
    /// Fading per year, after the holder's traits.
    pub decay: f32,
    /// The opinion change it caused (relationship deltas over a period).
    pub opinion: f32,
}

impl Memory {
    /// Salience on a date: `magnitude × e^(−decay × years)`.
    pub fn salience(&self, on: SimDate) -> f64 {
        let years = self.date.days_until(on) as f64 / mandate_core::time::DAYS_PER_YEAR as f64;
        self.magnitude as f64 * exp(-(self.decay as f64) * years)
    }
}

/// A directed relationship: how the holder sees the other.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    /// −100..100.
    pub opinion: f32,
    /// 0..1.
    pub trust: f32,
    pub bonds: Vec<Bond>,
}

/// Favours owed: `debtor` owes `creditor` `count` favours over a subject.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Favour {
    pub debtor: CharacterId,
    pub creditor: CharacterId,
    pub count: u8,
    /// Fluent key of the context, e.g. `favour-bill`.
    pub context: String,
    pub subject: Option<Subject>,
    pub since: SimDate,
}

/// Identity, values, and the politicians' axes for a character who is not a
/// politician (politicians keep theirs on `Politician`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedCore {
    pub given: String,
    pub family: String,
    pub birth_year: u32,
    /// Home neighbourhood index.
    pub home: u32,
    /// Occupation content id.
    pub occupation: ContentKey,
    /// Where they work and can be met (an organisation they lead, for leaders).
    pub place: Option<PlaceId>,
    pub ambition: f32,
    pub integrity: f32,
    pub oratory: f32,
    pub negotiation: f32,
    /// One value per attitude, in registry order (`PopulationState::attitude_keys`).
    pub values: Vec<f32>,
    /// The last year the lazy yearly update was applied.
    pub updated_year: u32,
    /// The Ambient person this character was promoted from: (location key, period, slot).
    #[serde(default)]
    pub origin: Option<(u64, u32, u8)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub id: CharacterId,
    /// The politician this character is, if any.
    pub politician: Option<PoliticianId>,
    /// Identity and values of a non-politician.
    pub named: Option<Box<NamedCore>>,
    pub warmth: f32,
    pub boldness: f32,
    pub loyalty: f32,
    pub administration: f32,
    pub organising: f32,
    pub intrigue: f32,
    /// The issue they know best (an attitude key).
    pub expertise: String,
    /// Trait content ids (0–3).
    pub traits: Vec<ContentKey>,
    /// Opinion of someone they have no history with (−100..100).
    pub first_impression: f32,
    pub memories: Vec<Memory>,
}

/// Every stored character, their relationships and favours, and each town's
/// character (DEC-077 §1).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CharactersState {
    pub ids: IdAllocator,
    /// Indexed by `CharacterId`.
    pub people: Vec<Character>,
    /// (holder, other) → relationship. Pairs without history are absent.
    #[serde(with = "mandate_core::serde_util::map_as_pairs")]
    pub edges: BTreeMap<(u32, u32), Relationship>,
    pub favours: Vec<Favour>,
    /// Indexed by municipality.
    pub towns: Vec<TownCharacter>,
    /// Named characters have been generated for this world.
    pub named_seeded: bool,
    /// Registry ids the town characters were computed with (recomputed on
    /// load when they differ).
    pub crisis_keys: Vec<ContentKey>,
    /// Goals held in the player's town (DEC-032).
    #[serde(default)]
    pub goals: Vec<goals::Goal>,
    /// Goals given up recently (not taken up again for a while).
    #[serde(default)]
    pub ended_goals: Vec<goals::EndedGoal>,
}

impl CharactersState {
    pub fn get(&self, id: CharacterId) -> Option<&Character> {
        self.people.get(id.index())
    }
    pub fn get_mut(&mut self, id: CharacterId) -> Option<&mut Character> {
        self.people.get_mut(id.index())
    }
    pub fn edge(&self, holder: CharacterId, other: CharacterId) -> Option<&Relationship> {
        self.edges.get(&(holder.0, other.0))
    }
}

/// Personality axes of anyone (0..1).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Personality {
    pub ambition: f32,
    pub integrity: f32,
    pub warmth: f32,
    pub boldness: f32,
    pub loyalty: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Competencies {
    pub oratory: f32,
    pub negotiation: f32,
    pub administration: f32,
    pub organising: f32,
    pub intrigue: f32,
    pub expertise: String,
}

/// Where someone stands, resolved from the politician or the Named record.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub id: CharacterId,
    pub given: String,
    pub family: String,
    pub birth_year: u32,
    pub home: u32,
    pub municipality: Option<u32>,
    /// The politician, if they are one.
    pub politician: Option<PoliticianId>,
    pub occupation: Option<ContentKey>,
    pub place: Option<PlaceId>,
    pub personality: Personality,
    pub competencies: Competencies,
    pub values: Vec<f32>,
    pub traits: Vec<ContentKey>,
}

impl Profile {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.given, self.family)
    }
}

/// One resolved view of a character.
pub fn profile(pol: &PoliticsState, world: &World, id: CharacterId) -> Option<Profile> {
    let c = pol.characters.get(id)?;
    let personality = |ambition: f32, integrity: f32| Personality {
        ambition,
        integrity,
        warmth: c.warmth,
        boldness: c.boldness,
        loyalty: c.loyalty,
    };
    let comps = |oratory: f32, negotiation: f32| Competencies {
        oratory,
        negotiation,
        administration: c.administration,
        organising: c.organising,
        intrigue: c.intrigue,
        expertise: c.expertise.clone(),
    };
    if let Some(p) = c.politician.and_then(|p| pol.get(p)) {
        return Some(Profile {
            id,
            given: p.given.clone(),
            family: p.family.clone(),
            birth_year: p.birth_year,
            home: p.home,
            municipality: Some(p.municipality()),
            politician: Some(p.id),
            occupation: None,
            place: None,
            personality: personality(p.ambition, p.integrity),
            competencies: comps(p.oratory, p.negotiation),
            values: p.attitudes.clone(),
            traits: c.traits.clone(),
        });
    }
    let n = c.named.as_deref()?;
    Some(Profile {
        id,
        given: n.given.clone(),
        family: n.family.clone(),
        birth_year: n.birth_year,
        home: n.home,
        municipality: world
            .neighborhoods
            .get(n.home as usize)
            .map(|h| h.municipality.0),
        politician: None,
        occupation: Some(n.occupation.clone()),
        place: n.place,
        personality: personality(n.ambition, n.integrity),
        competencies: comps(n.oratory, n.negotiation),
        values: n.values.clone(),
        traits: c.traits.clone(),
    })
}

/// The municipality a character belongs to.
pub fn municipality_of(pol: &PoliticsState, world: &World, id: CharacterId) -> Option<u32> {
    let c = pol.characters.get(id)?;
    if let Some(p) = c.politician.and_then(|p| pol.get(p)) {
        return Some(p.municipality());
    }
    let n = c.named.as_deref()?;
    world
        .neighborhoods
        .get(n.home as usize)
        .map(|h| h.municipality.0)
}

/// Whether a politician is standing in the current campaign.
pub fn is_candidate(pol: &PoliticsState, p: PoliticianId) -> bool {
    pol.elections
        .races
        .iter()
        .any(|r| r.candidates.iter().any(|c| c.politician == p))
}

/// A character's tier (DEC-018): derived, not stored.
pub fn tier_of(pol: &PoliticsState, content: &ContentBundle, id: CharacterId) -> Tier {
    let Some(c) = pol.characters.get(id) else {
        return Tier::Named;
    };
    if let Some(p) = c.politician.and_then(|p| pol.get(p)) {
        return if p.is_player || p.in_office() || is_candidate(pol, p.id) {
            Tier::Principal
        } else {
            Tier::Named
        };
    }
    let leader = c
        .named
        .as_deref()
        .and_then(|n| content.occupation(&n.occupation))
        .is_some_and(|o| o.leader);
    let featured = pol
        .characters
        .towns
        .iter()
        .any(|t| t.broker == Some(id) || t.rival == Some(id));
    if leader || featured {
        Tier::Principal
    } else {
        Tier::Named
    }
}

fn memory_cap(params: &CharacterParams, tier: Tier) -> usize {
    match tier {
        Tier::Principal => params.memory.principal as usize,
        Tier::Named => params.memory.named as usize,
    }
}

// --- Generation --------------------------------------------------------------------

fn axis(map: &BTreeMap<String, f64>, k: &str) -> f64 {
    map.get(k).copied().unwrap_or(0.0)
}

/// 0–3 traits in registry order, each by its chance, respecting exclusions.
fn draw_traits(rng: &mut SimRng, traits: &[TraitDef]) -> Vec<ContentKey> {
    let mut out: Vec<ContentKey> = Vec::new();
    for t in traits {
        // One draw per trait, always, so a trait's outcome does not depend
        // on earlier ones.
        let hit = rng.chance(t.chance);
        if !hit || out.len() >= 3 {
            continue;
        }
        let clash = out.iter().any(|o| {
            t.excludes.contains(o)
                || traits
                    .iter()
                    .find(|x| &x.id == o)
                    .is_some_and(|x| x.excludes.contains(&t.id))
        });
        if !clash {
            out.push(t.id.clone());
        }
    }
    out
}

fn trait_offset(content: &ContentBundle, traits: &[ContentKey], k: &str) -> f64 {
    traits
        .iter()
        .filter_map(|t| content.trait_def(t))
        .map(|t| axis(&t.personality, k))
        .sum()
}

/// The DEC-031 additions for a politician: keyed by the politician's id, so
/// creating one never shifts another stream. Their first impression of a
/// newcomer is drawn by the caller (it always was).
pub fn politician_character(
    content: &ContentBundle,
    seed: u64,
    id: CharacterId,
    p: &Politician,
    first_impression: f32,
) -> Character {
    let g = &content.characters.generation;
    let mut r = rng(seed, &[KEY_POLITICIAN, p.id.0 as u64]);
    let [plo, phi] = g.personality;
    let [clo, chi] = g.competency;
    let traits = draw_traits(&mut r, &content.traits);
    let mut ax = |k: &str| {
        clamp(
            r.range_f64(plo, phi) + trait_offset(content, &traits, k),
            0.0,
            1.0,
        ) as f32
    };
    let warmth = ax("warmth");
    let boldness = ax("boldness");
    let loyalty = ax("loyalty");
    let administration = r.range_f64(clo, chi) as f32;
    let organising = r.range_f64(clo, chi) as f32;
    let intrigue = r.range_f64(clo, chi) as f32;
    // Their pet issue: where their views are strongest.
    let expertise = p
        .attitudes
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()).then(b.0.cmp(&a.0)))
        .and_then(|(i, _)| content.attitudes.get(i))
        .map(|a| a.key.clone())
        .unwrap_or_default();
    Character {
        id,
        politician: Some(p.id),
        named: None,
        warmth,
        boldness,
        loyalty,
        administration,
        organising,
        intrigue,
        expertise,
        traits,
        first_impression,
        memories: Vec::new(),
    }
}

/// Register a politician's character (at the politician's creation).
pub fn add_politician(
    pol: &mut PoliticsState,
    content: &ContentBundle,
    seed: u64,
    pid: PoliticianId,
    first_impression: f32,
) -> CharacterId {
    let id: CharacterId = pol.characters.ids.alloc();
    if let Some(p) = pol.get(pid) {
        let c = politician_character(content, seed, id, p, first_impression);
        pol.characters.people.push(c);
    }
    if let Some(p) = pol.get_mut(pid) {
        p.character = id;
    }
    id
}

/// Characters for politicians that lack one (saves from before characters):
/// generated as at creation, without traits' ambition and integrity offsets
/// (those axes live on the politician).
pub fn ensure_politicians(pol: &mut PoliticsState, content: &ContentBundle, seed: u64) {
    for i in 0..pol.politicians.len() {
        let p = &pol.politicians[i];
        if p.character.index() < pol.characters.people.len() {
            continue;
        }
        let id = p.character;
        if id.index() != pol.characters.people.len() {
            continue;
        }
        let c = politician_character(content, seed, id, p, 0.0);
        pol.characters.people.push(c);
        if pol.characters.ids.peek_next() <= id.0 {
            pol.characters.ids = IdAllocator::starting_at(id.0 + 1);
        }
    }
}

// --- Relationships -------------------------------------------------------------------

/// How `holder` sees `other`: the edge, or their first impression.
pub fn opinion(pol: &PoliticsState, holder: CharacterId, other: CharacterId) -> f32 {
    match pol.characters.edge(holder, other) {
        Some(e) => e.opinion,
        None => pol
            .characters
            .get(holder)
            .map(|c| c.first_impression)
            .unwrap_or(0.0),
    }
}

pub fn trust(pol: &PoliticsState, content: &ContentBundle, h: CharacterId, o: CharacterId) -> f32 {
    pol.characters
        .edge(h, o)
        .map(|e| e.trust)
        .unwrap_or(content.characters.generation.trust_initial as f32)
}

/// A politician's opinion of another politician (the player's colleagues'
/// opinion of the player, for council leans and lobbying odds).
pub fn opinion_between(pol: &PoliticsState, holder: PoliticianId, other: PoliticianId) -> f32 {
    match (pol.get(holder), pol.get(other)) {
        (Some(h), Some(o)) => opinion(pol, h.character, o.character),
        _ => 0.0,
    }
}

fn edge_mut<'a>(
    pol: &'a mut PoliticsState,
    content: &ContentBundle,
    holder: CharacterId,
    other: CharacterId,
) -> &'a mut Relationship {
    let first = pol
        .characters
        .get(holder)
        .map(|c| c.first_impression)
        .unwrap_or(0.0);
    let trust = content.characters.generation.trust_initial as f32;
    pol.characters
        .edges
        .entry((holder.0, other.0))
        .or_insert_with(|| Relationship {
            opinion: first,
            trust,
            bonds: Vec::new(),
        })
}

/// The factor a holder's personality puts on an opinion change (DEC-031):
/// traits scale gains and losses; warmth scales gains.
pub fn opinion_scale(
    pol: &PoliticsState,
    content: &ContentBundle,
    holder: CharacterId,
    delta: f64,
) -> f64 {
    let Some(c) = pol.characters.get(holder) else {
        return 1.0;
    };
    let mut k = 1.0;
    for t in c.traits.iter().filter_map(|t| content.trait_def(t)) {
        k *= if delta >= 0.0 {
            t.opinion_change.positive
        } else {
            t.opinion_change.negative
        };
    }
    if delta > 0.0 {
        k *= 1.0 + content.characters.ai.warmth_opinion * (2.0 * c.warmth as f64 - 1.0);
    }
    k
}

/// Change how `holder` sees `other` by `delta` (scaled by the holder's
/// personality), clamped to −100..100. Returns the change applied. Allies and
/// rivals follow from the new opinion.
pub fn change_opinion(
    pol: &mut PoliticsState,
    content: &ContentBundle,
    holder: CharacterId,
    other: CharacterId,
    delta: f64,
) -> f32 {
    let d = delta * opinion_scale(pol, content, holder, delta);
    let b = &content.characters.bonds;
    let e = edge_mut(pol, content, holder, other);
    let before = e.opinion;
    e.opinion = clamp(e.opinion as f64 + d, -100.0, 100.0) as f32;
    let op = e.opinion as f64;
    set_bond(e, Bond::Ally, op >= b.ally_above);
    if op <= b.rival_below {
        set_bond(e, Bond::Rival, true);
    }
    e.opinion - before
}

fn set_bond(e: &mut Relationship, bond: Bond, on: bool) {
    let has = e.bonds.contains(&bond);
    if on && !has {
        e.bonds.push(bond);
        e.bonds.sort();
    } else if !on && has {
        e.bonds.retain(|b| *b != bond);
    }
}

/// Add (or remove) a bond from `holder` to `other`.
pub fn bond(
    pol: &mut PoliticsState,
    content: &ContentBundle,
    holder: CharacterId,
    other: CharacterId,
    bond: Bond,
    on: bool,
) {
    if holder == other {
        return;
    }
    if !on && pol.characters.edge(holder, other).is_none() {
        return;
    }
    let e = edge_mut(pol, content, holder, other);
    set_bond(e, bond, on);
}

pub fn bonds(pol: &PoliticsState, holder: CharacterId, other: CharacterId) -> Vec<Bond> {
    pol.characters
        .edge(holder, other)
        .map(|e| e.bonds.clone())
        .unwrap_or_default()
}

fn change_trust(
    pol: &mut PoliticsState,
    content: &ContentBundle,
    h: CharacterId,
    o: CharacterId,
    d: f64,
) {
    if d == 0.0 {
        return;
    }
    let e = edge_mut(pol, content, h, o);
    e.trust = clamp(e.trust as f64 + d, 0.0, 1.0) as f32;
}

// --- Favours -------------------------------------------------------------------------

/// Favours `debtor` owes `creditor`.
pub fn favours_owed(pol: &PoliticsState, debtor: CharacterId, creditor: CharacterId) -> u32 {
    pol.characters
        .favours
        .iter()
        .filter(|f| f.debtor == debtor && f.creditor == creditor)
        .map(|f| f.count as u32)
        .sum()
}

/// Net favours between two characters: positive = `a` owes `b`.
pub fn favour_balance(pol: &PoliticsState, a: CharacterId, b: CharacterId) -> i32 {
    favours_owed(pol, a, b) as i32 - favours_owed(pol, b, a) as i32
}

/// Record a favour owed.
pub fn owe(
    pol: &mut PoliticsState,
    debtor: CharacterId,
    creditor: CharacterId,
    context: &str,
    subject: Option<Subject>,
    date: SimDate,
) {
    if let Some(f) = pol.characters.favours.iter_mut().find(|f| {
        f.debtor == debtor && f.creditor == creditor && f.context == context && f.subject == subject
    }) {
        f.count = f.count.saturating_add(1);
        return;
    }
    pol.characters.favours.push(Favour {
        debtor,
        creditor,
        count: 1,
        context: context.into(),
        subject,
        since: date,
    });
}

/// Repay one favour (the oldest). Returns what it was for.
pub fn repay(
    pol: &mut PoliticsState,
    debtor: CharacterId,
    creditor: CharacterId,
) -> Option<Favour> {
    let i = pol
        .characters
        .favours
        .iter()
        .position(|f| f.debtor == debtor && f.creditor == creditor)?;
    let f = &mut pol.characters.favours[i];
    let out = f.clone();
    f.count = f.count.saturating_sub(1);
    if f.count == 0 {
        pol.characters.favours.remove(i);
    }
    Some(out)
}

// --- Memory --------------------------------------------------------------------------

/// The holder's decay for a memory of this sentiment, after traits.
fn decay_for(
    pol: &PoliticsState,
    content: &ContentBundle,
    holder: CharacterId,
    base: f64,
    sentiment: f64,
) -> f64 {
    let Some(c) = pol.characters.get(holder) else {
        return base;
    };
    let mut d = base;
    for t in c.traits.iter().filter_map(|t| content.trait_def(t)) {
        d *= if sentiment >= 0.0 {
            t.memory_decay.positive
        } else {
            t.memory_decay.negative
        };
    }
    d
}

/// Record a memory held by `holder` about `actor`. `opinion` overrides the
/// kind's opinion change (the player's actions set their own); the trust
/// change comes from the kind. The memory list stays within the holder's
/// tier cap: the least salient memory goes first. Returns the opinion change.
#[allow(clippy::too_many_arguments)]
pub fn remember(
    pol: &mut PoliticsState,
    content: &ContentBundle,
    date: SimDate,
    holder: CharacterId,
    actor: CharacterId,
    kind: MemoryKind,
    subject: Option<Subject>,
    opinion: Option<f64>,
) -> f32 {
    if holder == actor || pol.characters.get(holder).is_none() {
        return 0.0;
    }
    let Some(k) = content.characters.memory(kind.key()).cloned() else {
        return 0.0;
    };
    let delta = opinion.unwrap_or(k.opinion);
    let applied = if delta != 0.0 {
        change_opinion(pol, content, holder, actor, delta)
    } else {
        0.0
    };
    change_trust(pol, content, holder, actor, k.trust);
    let decay = decay_for(pol, content, holder, k.decay, k.sentiment);
    let cap = memory_cap(&content.characters, tier_of(pol, content, holder));
    let m = Memory {
        kind,
        actor,
        subject,
        sentiment: k.sentiment as f32,
        magnitude: k.magnitude as f32,
        date,
        decay: decay as f32,
        opinion: applied,
    };
    if let Some(c) = pol.characters.get_mut(holder) {
        c.memories.push(m);
        trim(
            &mut c.memories,
            cap,
            date,
            content.characters.memory.forget_below,
        );
    }
    applied
}

/// Forget faded memories, then keep the `cap` most salient (ties: newest).
fn trim(ms: &mut Vec<Memory>, cap: usize, on: SimDate, forget_below: f64) {
    ms.retain(|m| m.salience(on) >= forget_below);
    while ms.len() > cap {
        let Some((i, _)) = ms.iter().enumerate().min_by(|a, b| {
            a.1.salience(on)
                .total_cmp(&b.1.salience(on))
                .then(a.1.date.cmp(&b.1.date))
        }) else {
            break;
        };
        ms.remove(i);
    }
}

/// A holder's memories about someone, most salient first (ties: newest), as
/// of a date. `about = None` lists everything they remember.
pub fn salient_memories(
    pol: &PoliticsState,
    holder: CharacterId,
    about: Option<CharacterId>,
    on: SimDate,
    n: usize,
) -> Vec<Memory> {
    let Some(c) = pol.characters.get(holder) else {
        return Vec::new();
    };
    let mut v: Vec<&Memory> = c
        .memories
        .iter()
        .filter(|m| about.is_none_or(|a| m.actor == a))
        .collect();
    v.sort_by(|a, b| {
        b.salience(on)
            .total_cmp(&a.salience(on))
            .then(b.date.cmp(&a.date))
            .then(a.kind.cmp(&b.kind))
    });
    v.into_iter().take(n).cloned().collect()
}

/// How much `holder`'s opinion of `other` moved through remembered events
/// since a date (memories already forgotten are not counted).
pub fn relationship_delta(
    pol: &PoliticsState,
    holder: CharacterId,
    other: CharacterId,
    since: SimDate,
) -> f32 {
    pol.characters
        .get(holder)
        .map(|c| {
            c.memories
                .iter()
                .filter(|m| m.actor == other && m.date >= since)
                .map(|m| m.opinion)
                .sum()
        })
        .unwrap_or(0.0)
}

// --- Systems ------------------------------------------------------------------------

/// Politics domain, Characters phase, daily: memories other domains asked
/// for (OVERVIEW §4.3).
pub fn apply_effects(
    pol: &mut PoliticsState,
    r: &crate::scheduler::Reads<'_>,
    out: &mut crate::scheduler::Outs<'_>,
) {
    for e in out
        .effects
        .take(|e| matches!(e, crate::scheduler::Effect::Memory { .. }))
    {
        if let crate::scheduler::Effect::Memory {
            holder,
            actor,
            kind,
            subject,
        } = e
        {
            let (Some(h), Some(a)) = (pol.get(holder), pol.get(actor)) else {
                continue;
            };
            let (h, a) = (h.character, a.character);
            // The player's own memories are the player's.
            if pol.get(holder).is_some_and(|p| p.is_player) {
                continue;
            }
            remember(pol, r.content, r.date, h, a, kind, subject, None);
        }
    }
}

/// Politics domain, Characters phase, monthly: forget faded memories, keep
/// each character within their tier's cap, and recompute every town's
/// character.
pub fn monthly(
    pol: &mut PoliticsState,
    r: &crate::scheduler::Reads<'_>,
    _out: &mut crate::scheduler::Outs<'_>,
) {
    upkeep(pol, r.content, r.date);
    let towns = town::compute_all(pol, r);
    pol.characters.towns = towns;
    pol.characters.crisis_keys = r.content.crisis_keys().cloned().collect();
    goals::refresh(pol, r);
}

/// Memory upkeep for every stored character.
pub fn upkeep(pol: &mut PoliticsState, content: &ContentBundle, on: SimDate) {
    let forget = content.characters.memory.forget_below;
    for i in 0..pol.characters.people.len() {
        if pol.characters.people[i].memories.is_empty() {
            continue;
        }
        let cap = memory_cap(
            &content.characters,
            tier_of(pol, content, CharacterId(i as u32)),
        );
        trim(&mut pol.characters.people[i].memories, cap, on, forget);
    }
}

// --- Named characters -----------------------------------------------------------------

fn age_band(age: u32) -> usize {
    match age {
        0..=29 => 0,
        30..=44 => 1,
        45..=64 => 2,
        _ => 3,
    }
}

/// A cohort-conditioned view (DEC-031): the age × class cohort of a
/// neighbourhood (any community, by size), plus noise.
pub(crate) fn conditioned_values(
    pop: &PopulationState,
    hood: usize,
    age: u32,
    class: usize,
    sd: f64,
    r: &mut SimRng,
) -> Vec<f32> {
    let n = pop.attitude_count();
    let band = age_band(age);
    let Some(h) = pop.hoods.get(hood) else {
        return vec![0.0; n];
    };
    let mut acc = vec![0.0f64; n];
    let mut w = 0.0;
    for (ci, c) in h.cohorts.iter().enumerate() {
        if crate::population::age_of(ci) == band && crate::population::class_of(ci) == class {
            for (a, x) in acc.iter_mut().zip(&c.attitudes) {
                *a += *x as f64 * c.count as f64;
            }
            w += c.count as f64;
        }
    }
    acc.iter()
        .map(|a| clamp(a / w.max(1.0) + r.normal(0.0, sd), -1.0, 1.0) as f32)
        .collect()
}

/// Generate a Named (or leader) character for an occupation. Pure in its
/// inputs: the caller supplies the keyed generator.
#[allow(clippy::too_many_arguments)]
pub(crate) fn named_character(
    content: &ContentBundle,
    pop: &PopulationState,
    r: &mut SimRng,
    id: CharacterId,
    occupation: &mandate_content::OccupationDef,
    home: u32,
    place: Option<PlaceId>,
    year: u32,
) -> Character {
    let g = &content.characters.generation;
    let given: Vec<String> = content
        .name_pools
        .get("names.given")
        .map(|p| p.prefixes.clone())
        .unwrap_or_default();
    let family: Vec<String> = content
        .name_pools
        .get("names.family")
        .map(|p| p.prefixes.clone())
        .unwrap_or_default();
    let (gn, fname) = crate::politics::person_name(r, &given, &family);
    let age = r.range_i64(occupation.age[0] as i64, occupation.age[1] as i64) as u32;
    let class = r.weighted_index(&occupation.class).unwrap_or(1);
    let values = conditioned_values(pop, home as usize, age, class, g.values_sd, r);
    let traits = draw_traits(r, &content.traits);
    let [plo, phi] = g.personality;
    let [clo, chi] = g.competency;
    let mut ax = |k: &str| {
        clamp(
            r.range_f64(plo, phi)
                + axis(&occupation.personality, k)
                + trait_offset(content, &traits, k),
            0.0,
            1.0,
        ) as f32
    };
    let ambition = ax("ambition");
    let integrity = ax("integrity");
    let warmth = ax("warmth");
    let boldness = ax("boldness");
    let loyalty = ax("loyalty");
    let mut comp = |k: &str| {
        clamp(
            r.range_f64(clo, chi) + axis(&occupation.competencies, k),
            0.0,
            1.0,
        ) as f32
    };
    let oratory = comp("oratory");
    let negotiation = comp("negotiation");
    let administration = comp("administration");
    let organising = comp("organising");
    let intrigue = comp("intrigue");
    let expertise = occupation.expertise.clone().unwrap_or_else(|| {
        let i = r.below(content.attitudes.len().max(1) as u64) as usize;
        content
            .attitudes
            .get(i)
            .map(|a| a.key.clone())
            .unwrap_or_default()
    });
    let first = clamp(r.normal(0.0, g.first_impression_sd), -40.0, 40.0) as f32;
    Character {
        id,
        politician: None,
        named: Some(Box::new(NamedCore {
            given: gn,
            family: fname,
            birth_year: year.saturating_sub(age),
            home,
            occupation: occupation.id.clone(),
            place,
            ambition,
            integrity,
            oratory,
            negotiation,
            values,
            updated_year: year,
            origin: None,
        })),
        warmth,
        boldness,
        loyalty,
        administration,
        organising,
        intrigue,
        expertise,
        traits,
        first_impression: first,
        memories: Vec::new(),
    }
}

/// Generate the Named characters of every town (DEC-018): for each place,
/// each occupation found there by its chance; for each neighbourhood, each
/// occupation found on the streets. Keyed by place (or neighbourhood) and
/// occupation id, so the result does not depend on registry order or on any
/// sequential stream. Run once, when play begins.
pub fn seed_named(
    pol: &mut PoliticsState,
    world: &World,
    pop: &PopulationState,
    content: &ContentBundle,
    seed: u64,
    date: SimDate,
) {
    if pol.characters.named_seeded {
        return;
    }
    pol.characters.named_seeded = true;
    let year = date.calendar().year;
    let g = content.characters.generation.clone();
    // Places first (in world order), then each neighbourhood's streets.
    let mut slots: Vec<(u64, u32, Option<PlaceId>, &str)> = Vec::new();
    for p in &world.places {
        slots.push((p.id.0 as u64, p.neighborhood.0, Some(p.id), p.kind.key()));
    }
    for h in &world.neighborhoods {
        slots.push(((1u64 << 32) | h.id.0 as u64, h.id.0, None, "streets"));
    }
    for (key, hood, place, kind) in slots {
        for o in content
            .occupations
            .iter()
            .filter(|o| o.chance > 0.0 && o.places.iter().any(|k| k == kind))
        {
            let mut r = rng(seed, &[KEY_NAMED, key, key_of(&o.id)]);
            if !r.chance(o.chance) {
                continue;
            }
            // People who work at a place live near it: its neighbourhood, or
            // a neighbouring one.
            let home = world
                .neighborhoods
                .get(hood as usize)
                .map(|h| {
                    let mut opts = vec![h.id.0];
                    opts.extend(
                        h.neighbors
                            .iter()
                            .filter(|n| {
                                world
                                    .neighborhoods
                                    .get(n.index())
                                    .is_some_and(|x| x.municipality == h.municipality)
                            })
                            .map(|n| n.0),
                    );
                    *r.pick(&opts).unwrap_or(&h.id.0)
                })
                .unwrap_or(hood);
            let id: CharacterId = pol.characters.ids.alloc();
            let c = named_character(content, pop, &mut r, id, o, home, place, year);
            // Family of a local politician, sometimes.
            let kin = if r.chance(g.family_chance) {
                let m = world
                    .neighborhoods
                    .get(home as usize)
                    .map(|h| h.municipality.0);
                let local: Vec<CharacterId> = pol
                    .politicians
                    .iter()
                    .filter(|p| !p.is_player && Some(p.municipality()) == m && p.in_office())
                    .map(|p| p.character)
                    .collect();
                r.pick(&local).copied()
            } else {
                None
            };
            let mut c = c;
            if let (Some(k), Some(n)) = (kin, c.named.as_deref_mut())
                && let Some(p) = pol
                    .characters
                    .get(k)
                    .and_then(|x| x.politician)
                    .and_then(|p| pol.get(p))
            {
                n.family = p.family.clone();
            }
            pol.characters.people.push(c);
            if let Some(k) = kin {
                bond(pol, content, id, k, Bond::Family, true);
                bond(pol, content, k, id, Bond::Family, true);
            }
        }
    }
    // Staff of an organisation answer to its leader (employer bonds).
    let leaders: Vec<(PlaceId, CharacterId)> = pol
        .characters
        .people
        .iter()
        .filter_map(|c| {
            let n = c.named.as_deref()?;
            let o = content.occupation(&n.occupation)?;
            o.leader.then_some((n.place?, c.id))
        })
        .collect();
    let staff: Vec<(CharacterId, CharacterId)> = pol
        .characters
        .people
        .iter()
        .filter_map(|c| {
            let n = c.named.as_deref()?;
            let o = content.occupation(&n.occupation)?;
            if o.leader {
                return None;
            }
            let place = n.place?;
            leaders
                .iter()
                .find(|(p, l)| *p == place && *l != c.id)
                .map(|(_, l)| (c.id, *l))
        })
        .collect();
    for (s, l) in staff {
        bond(pol, content, s, l, Bond::Employer, true);
    }
}

/// One lazy yearly step for a Named character: their values drift a little
/// (keyed by character and year, so steps are order-independent).
fn named_step(c: &mut Character, seed: u64, drift_sd: f64, year: u32) {
    let id = c.id.0 as u64;
    let Some(n) = c.named.as_deref_mut() else {
        return;
    };
    let mut r = rng(seed, &[KEY_DRIFT, id, year as u64]);
    for v in n.values.iter_mut() {
        *v = clamp(*v as f64 + r.normal(0.0, drift_sd), -1.0, 1.0) as f32;
    }
}

/// A Named character brought up to a year (DEC-018 catch-up). Pure: the same
/// result whether it is applied once or in several steps, in any order with
/// other characters.
pub fn caught_up(c: &Character, seed: u64, content: &ContentBundle, year: u32) -> Character {
    let mut c = c.clone();
    let from = c.named.as_deref().map(|n| n.updated_year).unwrap_or(year);
    for y in (from + 1)..=year {
        named_step(&mut c, seed, content.characters.generation.drift_sd, y);
    }
    if let Some(n) = c.named.as_deref_mut()
        && n.updated_year < year
    {
        n.updated_year = year;
    }
    c
}

/// Bring a stored Named character up to date (before they act or are
/// remembered by). Principals and politicians are already current.
pub fn touch(
    pol: &mut PoliticsState,
    content: &ContentBundle,
    seed: u64,
    id: CharacterId,
    on: SimDate,
) {
    let year = on.calendar().year;
    let Some(c) = pol.characters.get(id) else {
        return;
    };
    if c.named.as_deref().is_none_or(|n| n.updated_year >= year) {
        return;
    }
    let fresh = caught_up(c, seed, content, year);
    if let Some(slot) = pol.characters.get_mut(id) {
        *slot = fresh;
    }
}

// --- Loading -------------------------------------------------------------------------

/// Bring characters in line with a loaded state and its content: characters
/// for politicians that lack them, Named characters for a game in play that
/// has none, and town characters for a changed crisis registry.
pub fn on_load(state: &mut crate::State, content: &ContentBundle) {
    let seed = state.meta.seed;
    ensure_politicians(&mut state.politics, content, seed);
    if state.player.is_some() && !state.politics.characters.named_seeded {
        seed_named(
            &mut state.politics,
            &state.world,
            &state.population,
            content,
            seed,
            state.calendar.date,
        );
    }
    let keys: Vec<ContentKey> = content.crisis_keys().cloned().collect();
    if state.politics.characters.crisis_keys != keys
        || state.politics.characters.towns.len() != state.world.municipalities.len()
    {
        refresh_towns(state, content);
    }
}

/// Recompute every town's character now (game start, load, or a change of
/// the player's plans).
pub fn refresh_towns(state: &mut crate::State, content: &ContentBundle) {
    let towns = {
        let r = crate::scheduler::Reads::of_state(state, content);
        town::compute_all_from(&state.politics, &r, &state.politics.characters.towns)
    };
    state.politics.characters.towns = towns;
    state.politics.characters.crisis_keys = content.crisis_keys().cloned().collect();
}

/// The rival of the player's town, recomputed now (the player's plans changed).
pub fn refresh_rival(state: &mut crate::State) {
    let Some(pl) = state.player.as_ref() else {
        return;
    };
    let m = pl.municipality.index();
    let rival = town::rival(&state.politics, &state.world, state.player.as_ref(), m);
    if let Some(t) = state.politics.characters.towns.get_mut(m) {
        t.rival = rival;
    }
}

/// Content ids the characters refer to that the content lacks (traits and
/// occupations are rules a stored character uses).
pub fn missing_references(pol: &PoliticsState, content: &ContentBundle) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in &pol.characters.people {
        for t in &c.traits {
            if content.trait_def(t).is_none() && !out.contains(t) {
                out.push(t.clone());
            }
        }
        if let Some(n) = c.named.as_deref()
            && content.occupation(&n.occupation).is_none()
            && !out.contains(&n.occupation)
        {
            out.push(n.occupation.clone());
        }
    }
    out
}

/// The player's character, if there is a player.
pub fn player_character(state: &crate::State) -> Option<CharacterId> {
    let pl = state.player.as_ref()?;
    state.politics.get(pl.politician).map(|p| p.character)
}

/// The politician's role for display: in office, standing, or a former member.
pub fn role_of(p: &Politician) -> &'static str {
    match p.role {
        Role::Councillor { .. } => "councillor",
        Role::Mayor { .. } => "mayor",
        Role::Private { .. } => "private",
    }
}
