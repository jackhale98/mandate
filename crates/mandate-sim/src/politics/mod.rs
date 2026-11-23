//! Politicians, parties, and local bodies (DEC-027): council members and
//! mayors generated from their wards' opinions, each belonging to a party
//! generated from the society's cleavages. Elections live in `elections`.
//! Politicians are characters (DEC-031): their relationships, favours, and
//! memory live in `characters` (`PoliticsState::characters`).

pub mod elections;
pub mod parties;
pub mod party_line;

pub use parties::{Party, PartyId};

use crate::characters::{CharacterId, CharactersState};
use crate::population::PopulationState;
use crate::world::{ConstituencyId, Jurisdiction, MunicipalityId, PlaceKind, World};
use mandate_content::{
    BodyDef, ConstitutionDef, ContentBundle, OfficeDef, PostKey, PostRef, Power, TierKey,
};
use mandate_core::math::clamp;
use mandate_core::{IdAllocator, SimRng, define_id};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

define_id!(
    /// A politician (NPC or the player).
    PoliticianId
);

/// Something a politician can hold or run for (DEC-017, DEC-098): a seat in
/// a constitution body or a constitution office, for a jurisdiction of any
/// tier. Bodies and offices are named by their constitution ids only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Post {
    /// A seat in a body. `constituency` is the district for a `ward` body,
    /// `None` for an at-large body.
    Seat {
        body: PostKey,
        at: Jurisdiction,
        constituency: Option<ConstituencyId>,
    },
    /// A single-holder office.
    Office { office: PostKey, at: Jurisdiction },
}

impl Post {
    /// The jurisdiction the post serves.
    pub fn at(self) -> Jurisdiction {
        match self {
            Post::Seat { at, .. } | Post::Office { at, .. } => at,
        }
    }
    /// The body's or the office's constitution id.
    pub fn id(&self) -> &str {
        match self {
            Post::Seat { body, .. } => body.as_str(),
            Post::Office { office, .. } => office.as_str(),
        }
    }
    /// A single-holder office (not a seat in a body).
    pub fn is_office(self) -> bool {
        matches!(self, Post::Office { .. })
    }
    /// The seat's district, if it has one.
    pub fn constituency(self) -> Option<ConstituencyId> {
        match self {
            Post::Seat { constituency, .. } => constituency,
            Post::Office { .. } => None,
        }
    }
    /// The town it serves, for a local post.
    pub fn municipality(self) -> Option<u32> {
        self.at().municipality().map(|m| m.0)
    }
    /// The constitution's definition of the post.
    pub fn def(self, c: &ConstitutionDef) -> Option<PostRef<'_>> {
        let t = c.tier(self.at().tier())?;
        match self {
            Post::Seat { body, .. } => t.body(body.as_str()).map(PostRef::Body),
            Post::Office { office, .. } => t.office(office.as_str()).map(PostRef::Office),
        }
    }
    /// The post's role word (a body's `member_key`, an office's id).
    pub fn word(self, c: &ConstitutionDef) -> Option<&str> {
        self.def(c).map(|d| d.word())
    }
    /// Neighbourhood indices the post's voters live in: its district, or
    /// the whole jurisdiction.
    pub fn hoods(self, w: &World) -> Vec<usize> {
        match self.constituency() {
            Some(c) => w
                .constituencies
                .get(c.index())
                .map(|c| c.neighborhoods.iter().map(|h| h.index()).collect())
                .unwrap_or_default(),
            None => self.at().hoods(w),
        }
    }
}

/// A politician's place in the constitution (DEC-098): a seat, an office,
/// or out of office. The saved shape of a seat or an office is a `Post`'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Role {
    Seat {
        body: PostKey,
        at: Jurisdiction,
        constituency: Option<ConstituencyId>,
    },
    Office {
        office: PostKey,
        at: Jurisdiction,
    },
    /// Not in office (a defeated or retired politician, or a challenger);
    /// `home` is the town they live in.
    Private {
        home: MunicipalityId,
    },
}

impl Role {
    /// The post held, if any.
    pub fn post(self) -> Option<Post> {
        match self {
            Role::Seat {
                body,
                at,
                constituency,
            } => Some(Post::Seat {
                body,
                at,
                constituency,
            }),
            Role::Office { office, at } => Some(Post::Office { office, at }),
            Role::Private { .. } => None,
        }
    }
    pub fn in_office(self) -> bool {
        !matches!(self, Role::Private { .. })
    }
    /// The district of a seat held, if any.
    pub fn constituency(self) -> Option<ConstituencyId> {
        self.post().and_then(Post::constituency)
    }
}

impl From<Post> for Role {
    fn from(p: Post) -> Self {
        match p {
            Post::Seat {
                body,
                at,
                constituency,
            } => Role::Seat {
                body,
                at,
                constituency,
            },
            Post::Office { office, at } => Role::Office { office, at },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Politician {
    pub id: PoliticianId,
    pub given: String,
    pub family: String,
    pub role: Role,
    pub is_player: bool,
    pub birth_year: u32,
    /// One value per attitude, in registry order (`PopulationState::attitude_keys`).
    pub attitudes: Vec<f32>,
    /// Willingness to deal and be persuaded (0..1).
    pub pragmatism: f32,
    pub integrity: f32,
    pub oratory: f32,
    pub negotiation: f32,
    /// Their character record (DEC-031): personality, relationships
    /// (including their opinion of the player), favours, and memory.
    pub character: CharacterId,
    pub party: Option<PartyId>,
    /// Drive to seek office (0..1).
    pub ambition: f32,
    /// Home neighbourhood (local ties in elections).
    pub home: u32,
    /// Valence damage from scandals (0..1), decays over time.
    pub scandal: f32,
    /// Campaign war chest (currency units).
    pub funds: f64,
    /// Years spent in elected office.
    pub years_in_office: u8,
    /// Terms served in the current office (term limits).
    pub terms_in_office: u8,
    /// Visible work in office (0..1): bills, votes, meetings, doorsteps. Fades
    /// monthly; a thin record is an opponent's best attack (DEC-076).
    #[serde(default)]
    pub record: f32,
}

impl Politician {
    pub fn in_office(&self) -> bool {
        !matches!(self.role, Role::Private { .. })
    }

    pub fn full_name(&self) -> String {
        format!("{} {}", self.given, self.family)
    }

    /// The town the politician lives and is active in: out of office, their
    /// home; in office, the town of their local post. (M6 instantiates local
    /// posts only; a higher-tier post's holder reads `u32::MAX` until M7
    /// derives it from `home`.)
    pub fn municipality(&self) -> u32 {
        match self.role {
            Role::Private { home } => home.0,
            Role::Seat { at, .. } | Role::Office { at, .. } => {
                at.municipality().map(|m| m.0).unwrap_or(u32::MAX)
            }
        }
    }
}

/// The seats of one body for one jurisdiction (DEC-098).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodySeats {
    pub body: PostKey,
    pub at: Jurisdiction,
    /// One member per seat: for a `ward` body, in constituency order.
    pub members: Vec<PoliticianId>,
    /// Where the members are found (the body's `works_at`, set when the
    /// body is instantiated).
    pub works_at: Option<PlaceKind>,
}

/// The holder of one office for one jurisdiction (DEC-098).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OfficeHolder {
    pub office: PostKey,
    pub at: Jurisdiction,
    pub holder: Option<PoliticianId>,
    /// Where the holder is found (the office's `works_at`).
    pub works_at: Option<PlaceKind>,
}

/// The officials found at a place kind of a jurisdiction (DEC-098: the
/// posts whose `works_at` is that kind): body members first, then office
/// holders.
pub fn working_at(st: &PoliticsState, at: Jurisdiction, kind: PlaceKind) -> Vec<PoliticianId> {
    st.bodies
        .iter()
        .filter(|b| b.at == at && b.works_at == Some(kind))
        .flat_map(|b| b.members.iter().copied())
        .chain(
            st.offices
                .iter()
                .filter(|o| o.at == at && o.works_at == Some(kind))
                .filter_map(|o| o.holder),
        )
        .collect()
}

/// Where a post's holder works, as instantiated.
pub fn workplace_of(st: &PoliticsState, post: Post) -> Option<PlaceKind> {
    match post {
        Post::Seat { body, at, .. } => st
            .bodies
            .iter()
            .find(|b| b.at == at && b.body == body)
            .and_then(|b| b.works_at),
        Post::Office { office, at } => st
            .offices
            .iter()
            .find(|o| o.at == at && o.office == office)
            .and_then(|o| o.works_at),
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PoliticsState {
    pub ids: IdAllocator,
    pub politicians: Vec<Politician>,
    /// Every instantiated body's seats, by jurisdiction then constitution
    /// order (DEC-098; M6 instantiates the local tier).
    pub bodies: Vec<BodySeats>,
    /// Every instantiated office's holder, in the same order.
    pub offices: Vec<OfficeHolder>,
    /// Per (politician, policy) persuasion built up by lobbying, trades, and earmarks.
    #[serde(with = "mandate_core::serde_util::map_as_pairs")]
    pub sway: BTreeMap<(u32, u32), f32>,
    pub parties: Vec<Party>,
    pub elections: elections::ElectionState,
    /// Publicly declared votes: (politician, policy) → +1 yes, −1 no, 0 abstain.
    #[serde(with = "mandate_core::serde_util::map_as_pairs")]
    pub declared_votes: BTreeMap<(u32, u32), i8>,
    /// Characters (DEC-031): every politician's character record, Named
    /// characters, relationships, favours, memory, and town characters.
    #[serde(default)]
    pub characters: CharactersState,
}

/// A declared vote on a bill (for a mayor: sign or veto).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vote {
    Yes,
    No,
    Abstain,
}

/// Add visible work to a politician's record (DEC-076: inactivity is punished).
pub fn add_record(st: &mut PoliticsState, id: PoliticianId, amount: f32) {
    if let Some(p) = st.get_mut(id) {
        p.record = (p.record + amount).min(1.0);
    }
}

/// Politics domain, Characters phase, daily: what coverage and the courts
/// did (DEC-034, DEC-006): scandal valence, memories, removals from office
/// (the party fills the seat), and NPCs' sentences.
pub fn media_effects(
    st: &mut PoliticsState,
    r: &crate::scheduler::Reads<'_>,
    out: &mut crate::scheduler::Outs<'_>,
) {
    use crate::scheduler::Effect;
    let player = r.player.map(|p| p.politician);
    for e in out.effects.take(|e| match e {
        Effect::Valence { .. } | Effect::Remember { .. } | Effect::Removal { .. } => true,
        Effect::Sentenced { politician, .. } => Some(*politician) != player,
        _ => false,
    }) {
        match e {
            Effect::Valence { politician, delta } => {
                if let Some(p) = st.get_mut(politician) {
                    p.scandal = (p.scandal + delta).clamp(0.0, 1.0);
                }
            }
            Effect::Remember {
                holder,
                actor,
                kind,
            } => {
                if st.characters.get(holder).and_then(|c| c.politician) != player {
                    crate::characters::remember(
                        st, r.content, r.date, holder, actor, kind, None, None,
                    );
                }
            }
            Effect::Removal { politician, barred } => {
                let Some(p) = st.get(politician) else {
                    continue;
                };
                if p.in_office() {
                    let m = p.municipality();
                    elections::appoint_replacement(st, r, politician);
                    if let Some(p) = st.get_mut(politician) {
                        p.role = Role::Private {
                            home: MunicipalityId(m),
                        };
                        p.terms_in_office = 0;
                    }
                }
                if barred
                    && let Some(p) = st.get_mut(politician)
                    && !p.is_player
                {
                    // In prison: no candidacy (the party will not recruit them).
                    p.ambition = 0.0;
                }
            }
            Effect::Sentenced {
                politician,
                sentence,
                ..
            } => {
                if let Some(p) = st.get_mut(politician) {
                    if let crate::justice::Sentence::Fine { amount }
                    | crate::justice::Sentence::Removed { amount } = sentence
                    {
                        p.funds = (p.funds - amount).max(0.0);
                    }
                    p.scandal = (p.scandal + 0.2).min(1.0);
                }
            }
            _ => {}
        }
    }
}

impl PoliticsState {
    pub fn get(&self, id: PoliticianId) -> Option<&Politician> {
        self.politicians.get(id.index())
    }
    pub fn get_mut(&mut self, id: PoliticianId) -> Option<&mut Politician> {
        self.politicians.get_mut(id.index())
    }
    pub fn party(&self, id: PartyId) -> Option<&Party> {
        self.parties.get(id.index())
    }
    pub fn sway(&self, p: PoliticianId, policy: u32) -> f32 {
        self.sway.get(&(p.0, policy)).copied().unwrap_or(0.0)
    }
}

// --- Posts: who holds what (DEC-098) -----------------------------------------------

/// The run's constitution.
pub fn constitution<'a>(content: &'a ContentBundle, id: &str) -> Option<&'a ConstitutionDef> {
    content.constitutions.get(id)
}

/// The members of a body for a jurisdiction (empty when it has none).
pub fn members_of<'a>(st: &'a PoliticsState, body: &str, at: Jurisdiction) -> &'a [PoliticianId] {
    st.bodies
        .iter()
        .find(|b| b.at == at && b.body == *body)
        .map(|b| b.members.as_slice())
        .unwrap_or(&[])
}

/// The holder of an office for a jurisdiction.
pub fn holder_of(st: &PoliticsState, office: &str, at: Jurisdiction) -> Option<PoliticianId> {
    st.offices
        .iter()
        .find(|o| o.at == at && o.office == *office)
        .and_then(|o| o.holder)
}

/// Everyone holding a post for a jurisdiction: the members of its bodies
/// (in constitution order), then its office holders.
pub fn officials_at(st: &PoliticsState, at: Jurisdiction) -> Vec<PoliticianId> {
    st.bodies
        .iter()
        .filter(|b| b.at == at)
        .flat_map(|b| b.members.iter().copied())
        .chain(
            st.offices
                .iter()
                .filter(|o| o.at == at)
                .filter_map(|o| o.holder),
        )
        .collect()
}

/// The members of a jurisdiction's bodies, in constitution order.
pub fn body_members_at(st: &PoliticsState, at: Jurisdiction) -> Vec<PoliticianId> {
    st.bodies
        .iter()
        .filter(|b| b.at == at)
        .flat_map(|b| b.members.iter().copied())
        .collect()
}

/// The holders of a jurisdiction's offices, in constitution order.
pub fn office_holders_at(st: &PoliticsState, at: Jurisdiction) -> Vec<PoliticianId> {
    st.offices
        .iter()
        .filter(|o| o.at == at)
        .filter_map(|o| o.holder)
        .collect()
}

/// A seat's index among its body's members: its constituency's position
/// among the jurisdiction's constituencies (`None` for at-large seats).
pub fn seat_index(world: &World, post: Post) -> Option<usize> {
    let c = post.constituency()?;
    post.at().constituencies(world).position(|x| x.id == c)
}

/// Who holds a post now.
pub fn holder_of_post(st: &PoliticsState, world: &World, post: Post) -> Option<PoliticianId> {
    match post {
        Post::Office { office, at } => holder_of(st, office.as_str(), at),
        Post::Seat { body, at, .. } => members_of(st, body.as_str(), at)
            .get(seat_index(world, post)?)
            .copied(),
    }
}

/// Put `id` in a post's storage slot (the role is the caller's).
pub fn seat_holder(st: &mut PoliticsState, world: &World, post: Post, id: PoliticianId) {
    match post {
        Post::Office { office, at } => {
            if let Some(o) = st
                .offices
                .iter_mut()
                .find(|o| o.at == at && o.office == office)
            {
                o.holder = Some(id);
            }
        }
        Post::Seat { body, at, .. } => {
            let Some(i) = seat_index(world, post) else {
                return;
            };
            if let Some(slot) = st
                .bodies
                .iter_mut()
                .find(|b| b.at == at && b.body == body)
                .and_then(|b| b.members.get_mut(i))
            {
                *slot = id;
            }
        }
    }
}

/// The post a politician holds in storage (a seat or an office), if any.
pub fn post_held(st: &PoliticsState, world: &World, id: PoliticianId) -> Option<Post> {
    if let Some(o) = st.offices.iter().find(|o| o.holder == Some(id)) {
        return Some(Post::Office {
            office: o.office,
            at: o.at,
        });
    }
    let b = st.bodies.iter().find(|b| b.members.contains(&id))?;
    let i = b.members.iter().position(|x| *x == id)?;
    let constituency = b.at.constituencies(world).nth(i).map(|c| c.id);
    Some(Post::Seat {
        body: b.body,
        at: b.at,
        constituency,
    })
}

/// The tier's legislature: the body of its `ordinance` procedure's first
/// `vote` stage (by procedure, never by name).
pub fn legislature(c: &ConstitutionDef, tier: TierKey) -> Option<&BodyDef> {
    c.tier(tier)?.legislature()
}

/// The tier's executive: the office of its `ordinance` procedure's `sign`
/// stage.
pub fn executive(c: &ConstitutionDef, tier: TierKey) -> Option<&OfficeDef> {
    c.tier(tier)?.executive()
}

/// The executive office of a jurisdiction, as a post.
pub fn executive_post(c: &ConstitutionDef, at: Jurisdiction) -> Option<Post> {
    let o = executive(c, at.tier())?;
    Some(Post::Office {
        office: PostKey::new(&o.id)?,
        at,
    })
}

/// The holder of a jurisdiction's executive office.
pub fn executive_of(
    st: &PoliticsState,
    c: &ConstitutionDef,
    at: Jurisdiction,
) -> Option<PoliticianId> {
    executive(c, at.tier()).and_then(|o| holder_of(st, &o.id, at))
}

/// The members of a jurisdiction's legislature.
pub fn legislature_of<'a>(
    st: &'a PoliticsState,
    c: &ConstitutionDef,
    at: Jurisdiction,
) -> &'a [PoliticianId] {
    legislature(c, at.tier())
        .map(|b| members_of(st, &b.id, at))
        .unwrap_or(&[])
}

/// The seat in a jurisdiction's legislature for a constituency.
pub fn legislature_seat(
    c: &ConstitutionDef,
    at: Jurisdiction,
    constituency: ConstituencyId,
) -> Option<Post> {
    let b = legislature(c, at.tier())?;
    Some(Post::Seat {
        body: PostKey::new(&b.id)?,
        at,
        constituency: Some(constituency),
    })
}

/// The powers a post holds, read from the constitution (DEC-017: no role
/// matches).
pub fn powers_of(c: &ConstitutionDef, post: Post) -> &[Power] {
    post.def(c).map(|d| d.powers()).unwrap_or(&[])
}

/// Whether `who` holds a post for `at` that lists `power`. (Powers that
/// reach down to a jurisdiction's towns come with M7.)
pub fn holds_power_at(
    st: &PoliticsState,
    c: &ConstitutionDef,
    who: PoliticianId,
    power: Power,
    at: Jurisdiction,
) -> bool {
    posts_of(st, who)
        .into_iter()
        .any(|p| p.at() == at && powers_of(c, p).contains(&power))
}

/// Whether `who` holds any post that lists `power`.
pub fn holds_power(
    st: &PoliticsState,
    c: &ConstitutionDef,
    who: PoliticianId,
    power: Power,
) -> bool {
    posts_of(st, who)
        .into_iter()
        .any(|p| powers_of(c, p).contains(&power))
}

/// The posts a politician holds (one in M6).
pub fn posts_of(st: &PoliticsState, who: PoliticianId) -> Vec<Post> {
    st.get(who)
        .and_then(|p| p.role.post())
        .into_iter()
        .collect()
}

/// A politician's role word for content and labels: their post's (a body's
/// `member_key`, an office's id), or `None` out of office.
pub fn role_word<'a>(c: &'a ConstitutionDef, p: &Politician) -> Option<&'a str> {
    p.role.post().and_then(|post| post.word(c))
}

/// Constitution ids the politics state refers to that the run's
/// constitution lacks (a save is refused rather than run without them).
pub fn missing_references(st: &PoliticsState, c: Option<&ConstitutionDef>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let Some(c) = c else {
        return out;
    };
    let mut check = |post: Post| {
        if post.def(c).is_none() && !out.iter().any(|x| x == post.id()) {
            out.push(post.id().to_string());
        }
    };
    for p in &st.politicians {
        if let Some(post) = p.role.post() {
            check(post);
        }
    }
    for b in &st.bodies {
        check(Post::Seat {
            body: b.body,
            at: b.at,
            constituency: None,
        });
    }
    for o in &st.offices {
        check(Post::Office {
            office: o.office,
            at: o.at,
        });
    }
    for r in &st.elections.races {
        check(r.seat);
    }
    out
}

/// Turnout-weighted mean attitudes of the adults in a set of neighbourhoods.
pub fn mean_attitudes(pop: &PopulationState, hoods: &[usize]) -> Vec<f32> {
    let mut acc = vec![0.0f64; pop.attitude_count()];
    let mut w = 0.0f64;
    for &h in hoods {
        for c in &pop.hoods[h].cohorts {
            let weight = c.count as f64 * c.turnout as f64;
            for (k, a) in acc.iter_mut().enumerate() {
                *a += c.attitudes[k] as f64 * weight;
            }
            w += weight;
        }
    }
    acc.iter().map(|a| (a / w.max(1.0)) as f32).collect()
}

/// A generated person's (given, family) name from the name pools.
pub fn person_name(rng: &mut SimRng, given: &[String], family: &[String]) -> (String, String) {
    let g = rng.pick(given).cloned().unwrap_or_else(|| "Alex".into());
    let f1 = rng.pick(family).cloned().unwrap_or_else(|| "Smith".into());
    let f2 = if rng.chance(0.5) {
        String::new()
    } else {
        ["wood", "field", "ley", "son", "er", "well", "by"][rng.below(7) as usize].to_string()
    };
    (g, f1 + &f2)
}

/// The tiers whose posts are instantiated and elected (M6: the local tier;
/// M7 adds the regional one with no change of shape).
pub const INSTANTIATED_TIERS: [TierKey; 1] = [TierKey::Local];

/// The posts of every jurisdiction of the instantiated tiers, reflecting
/// their voters (with noise): each body's seats (one per constituency for a
/// `ward` body), then each office's holder, as the constitution defines them.
#[allow(clippy::too_many_arguments)]
pub fn init(
    world: &World,
    pop: &PopulationState,
    rng: &mut SimRng,
    given: &[String],
    family: &[String],
    year: u32,
    parties: Vec<Party>,
    content: &mandate_content::ContentBundle,
    constitution: &str,
    seed: u64,
) -> PoliticsState {
    let mut st = PoliticsState {
        parties,
        ..Default::default()
    };
    let make = |st: &mut PoliticsState,
                rng: &mut SimRng,
                role: Role,
                base: Vec<f32>,
                home: u32|
     -> PoliticianId {
        let mut attitudes = base;
        for a in attitudes.iter_mut() {
            *a = clamp(*a as f64 + rng.normal(0.0, 0.18), -1.0, 1.0) as f32;
        }
        let party = parties::nearest_party(&attitudes, &st.parties);
        new_politician(
            st, rng, given, family, role, attitudes, party, home, year, content, seed,
        )
    };
    let _ = &make;
    let Some(c) = content.constitutions.get(constitution) else {
        return st;
    };
    for tier in INSTANTIATED_TIERS {
        let Some(t) = c.tier(tier) else { continue };
        for at in Jurisdiction::all_of(tier, world) {
            let all = at.hoods(world);
            for body in &t.bodies {
                let Some(key) = PostKey::new(&body.id) else {
                    continue;
                };
                let mut members = Vec::new();
                if PostRef::Body(body).constituency() == Some(mandate_content::Constituency::Ward) {
                    let wards: Vec<_> = at.constituencies(world).collect();
                    for w in &wards {
                        let hoods: Vec<usize> = w.neighborhoods.iter().map(|h| h.index()).collect();
                        let base = mean_attitudes(pop, &hoods);
                        let home = w.neighborhoods.first().map(|h| h.0).unwrap_or(0);
                        let role = Role::Seat {
                            body: key,
                            at,
                            constituency: Some(w.id),
                        };
                        members.push(make(&mut st, rng, role, base, home));
                    }
                } else {
                    // At-large seats (validation keeps them off the local
                    // tier until their races exist).
                    for _ in 0..body.seats.unwrap_or(0) {
                        let home = all.first().map(|h| *h as u32).unwrap_or(0);
                        let role = Role::Seat {
                            body: key,
                            at,
                            constituency: None,
                        };
                        let base = mean_attitudes(pop, &all);
                        members.push(make(&mut st, rng, role, base, home));
                    }
                }
                st.bodies.push(BodySeats {
                    body: key,
                    at,
                    members,
                    works_at: PlaceKind::from_key(&body.works_at),
                });
            }
            for office in &t.offices {
                let Some(key) = PostKey::new(&office.id) else {
                    continue;
                };
                let home = all.first().map(|h| *h as u32).unwrap_or(0);
                let role = Role::Office { office: key, at };
                let holder = make(&mut st, rng, role, mean_attitudes(pop, &all), home);
                st.offices.push(OfficeHolder {
                    office: key,
                    at,
                    holder: Some(holder),
                    works_at: PlaceKind::from_key(&office.works_at),
                });
            }
        }
    }
    // Sitting officials have served some years already.
    for p in st.politicians.iter_mut() {
        p.record = 0.4;
        p.years_in_office = rng.range_i64(0, 12) as u8;
        p.terms_in_office = 1 + p.years_in_office / 4;
    }
    st
}

/// Create a politician (NPC) and register them, with their character
/// (DEC-031). Used at generation and for new challengers at each election.
/// The draws from `rng` are as before characters existed; the character's
/// own draws come from a stream keyed by the politician's id.
#[allow(clippy::too_many_arguments)]
pub fn new_politician(
    st: &mut PoliticsState,
    rng: &mut SimRng,
    given: &[String],
    family: &[String],
    role: Role,
    attitudes: Vec<f32>,
    party: Option<PartyId>,
    home: u32,
    year: u32,
    content: &mandate_content::ContentBundle,
    seed: u64,
) -> PoliticianId {
    let id: PoliticianId = st.ids.alloc();
    let (g, f) = person_name(rng, given, family);
    let birth_year = year.saturating_sub(rng.range_i64(30, 68) as u32);
    let pragmatism = rng.range_f64(0.2, 0.9) as f32;
    let integrity = rng.range_f64(0.2, 0.95) as f32;
    let oratory = rng.range_f64(0.2, 0.9) as f32;
    let negotiation = rng.range_f64(0.2, 0.9) as f32;
    // How they take to a newcomer (the player): their first impression.
    let first_impression = rng.normal(0.0, 12.0).clamp(-40.0, 40.0) as f32;
    let ambition = rng.range_f64(0.2, 0.95) as f32;
    let funds = rng.range_f64(2_000.0, 12_000.0);
    st.politicians.push(Politician {
        id,
        given: g,
        family: f,
        role,
        is_player: false,
        birth_year,
        attitudes,
        pragmatism,
        integrity,
        oratory,
        negotiation,
        character: CharacterId(u32::MAX),
        party,
        ambition,
        home,
        scandal: 0.0,
        funds,
        years_in_office: 0,
        terms_in_office: 0,
        record: 0.0,
    });
    crate::characters::add_politician(st, content, seed, id, first_impression);
    id
}

/// Cosine-style alignment between a policy signature and an attitude vector, in [-1, 1].
/// Both are in attitude-registry order; a missing tail counts as 0.
pub fn alignment(signature: &[f64], attitudes: &[f32]) -> f64 {
    let mut dot = 0.0;
    let mut norm = 0.0;
    for (s, a) in signature.iter().zip(attitudes) {
        dot += s * *a as f64;
        norm += s.abs();
    }
    if norm <= 0.0 {
        0.0
    } else {
        clamp(dot / norm * 2.0, -1.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use crate::politics::elections::{calendar_for, next_election_day};
    use mandate_core::SimDate;

    /// The base pack's constitution.
    fn base() -> ConstitutionDef {
        let src = include_str!("../../../../content/base/constitutions/federal_presidential.jsonc");
        let v: serde_json::Value =
            serde_json::from_str(&mandate_content::jsonc::to_strict_json(src)).unwrap();
        serde_json::from_value(v["value"].clone()).unwrap()
    }

    fn key(s: &str) -> PostKey {
        PostKey::new(s).unwrap()
    }

    fn town(m: u32) -> Jurisdiction {
        Jurisdiction::town(MunicipalityId(m))
    }

    /// A post's saved shape is the migration table's (audit §7.7), and a
    /// role holding it saves the same.
    #[test]
    fn post_serde_shape() {
        let seat = Post::Seat {
            body: key("council"),
            at: town(3),
            constituency: Some(ConstituencyId(7)),
        };
        let json = serde_json::json!({
            "type": "seat", "body": "council",
            "at": { "type": "municipality", "id": 3 }, "constituency": 7,
        });
        assert_eq!(serde_json::to_value(seat).unwrap(), json);
        assert_eq!(serde_json::to_value(Role::from(seat)).unwrap(), json);
        assert_eq!(serde_json::from_value::<Post>(json).unwrap(), seat);
        let office = Post::Office {
            office: key("mayor"),
            at: town(3),
        };
        assert_eq!(
            serde_json::to_value(office).unwrap(),
            serde_json::json!({ "type": "office", "office": "mayor", "at": { "type": "municipality", "id": 3 } })
        );
        assert_eq!(
            serde_json::to_value(Role::Private {
                home: MunicipalityId(3)
            })
            .unwrap(),
            serde_json::json!({ "type": "private", "home": 3 })
        );
        assert_eq!(
            serde_json::to_value(Jurisdiction::Nation).unwrap(),
            serde_json::json!({ "type": "nation" })
        );
        // Keys longer than a post key are refused, not cut.
        assert!(PostKey::new(&"x".repeat(PostKey::MAX + 1)).is_none());
        assert!(serde_json::from_value::<PostKey>(serde_json::json!("")).is_err());
        assert_eq!(Role::from(office).post(), Some(office));
        assert_eq!(seat.constituency(), Some(ConstituencyId(7)));
        assert!(office.is_office() && !seat.is_office());
    }

    /// Powers are read from the constitution: a seat has its body's, an
    /// office its own.
    #[test]
    fn powers_of_reads_the_constitution() {
        let c = base();
        let seat = Post::Seat {
            body: key("council"),
            at: town(0),
            constituency: Some(ConstituencyId(0)),
        };
        let mayor = Post::Office {
            office: key("mayor"),
            at: town(0),
        };
        assert!(powers_of(&c, seat).contains(&Power::ApproveOrdinance));
        assert!(!powers_of(&c, seat).contains(&Power::Veto));
        assert!(powers_of(&c, mayor).contains(&Power::Veto));
        // A post the constitution lacks has no powers.
        let other = Post::Office {
            office: key("doge"),
            at: town(0),
        };
        assert!(powers_of(&c, other).is_empty());
        // A town post read on the wrong tier is not found either.
        let misplaced = Post::Office {
            office: key("mayor"),
            at: Jurisdiction::Nation,
        };
        assert!(powers_of(&c, misplaced).is_empty());
        assert_eq!(
            legislature(&c, TierKey::Local).map(|b| b.id.as_str()),
            Some("council")
        );
        assert_eq!(
            executive(&c, TierKey::Local).map(|o| o.id.as_str()),
            Some("mayor")
        );
        assert_eq!(
            executive(&c, TierKey::National).map(|o| o.id.as_str()),
            Some("president")
        );
        assert_eq!(seat.word(&c), Some("councillor"));
        assert_eq!(mayor.word(&c), Some("mayor"));
    }

    /// Each post has its own term and calendar: the president is elected
    /// every four years, the house every two (the latent bug was the
    /// president on the house's term).
    #[test]
    fn calendar_for_uses_each_posts_term() {
        let c = base();
        let president = Post::Office {
            office: key("president"),
            at: Jurisdiction::Nation,
        };
        let house = Post::Seat {
            body: key("house"),
            at: Jurisdiction::Nation,
            constituency: Some(ConstituencyId(0)),
        };
        let (pc, pt) = calendar_for(&c, president).unwrap();
        let (hc, ht) = calendar_for(&c, house).unwrap();
        assert_eq!((pt, ht), (4, 2));
        let after = SimDate::from_ymd(2021, 1, 1);
        assert_eq!(
            next_election_day(pc, pt, after).calendar().year,
            2024,
            "the president on a four-year cycle"
        );
        assert_eq!(next_election_day(hc, ht, after).calendar().year, 2022);
        let mayor = Post::Office {
            office: key("mayor"),
            at: town(0),
        };
        assert_eq!(calendar_for(&c, mayor).map(|(_, t)| t), Some(4));
        // The local cycle: the first election day after the start.
        let (day, weeks) = elections::next_cycle(&c, SimDate::from_ymd(2020, 1, 15)).unwrap();
        assert_eq!(day, SimDate::from_ymd(2020, 11, 3));
        assert_eq!(weeks, 8);
    }
}
