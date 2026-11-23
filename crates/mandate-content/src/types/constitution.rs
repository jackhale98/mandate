//! Constitution primitives (DEC-017): tiers, bodies, offices, procedures, calendars, posts.

use super::*;

// --- Constitution primitives (DEC-017) -----------------------------------------

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TierKey {
    Local,
    Regional,
    National,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Power {
    ProposeOrdinance,
    ApproveOrdinance,
    Veto,
    Override,
    ProposeBudget,
    ApproveBudget,
    Appoint,
    SetLocalTaxes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum DecisionRule {
    Majority,
    Supermajority { num: u8, den: u8 },
    Unanimity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VotingRule {
    Plurality,
    TwoRound,
    RankedChoice,
    ListPr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Constituency {
    /// One seat per generated ward / district.
    Ward,
    /// The whole jurisdiction.
    AtLarge,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Selection {
    Election {
        rule: VotingRule,
        constituency: Constituency,
    },
    AppointedBy {
        office: String,
    },
    ElectedByBody {
        body: String,
    },
}

/// A collective that holds seats (DEC-017): a council, an assembly, a
/// chamber. Its id is how procedures, content and saves refer to it (DEC-098:
/// no code names a body).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BodyDef {
    /// Unique across the constitution's tiers; at most [`PostKey::MAX`] bytes.
    pub id: String,
    pub name_key: String,
    /// The word for one member (`councillor`): Fluent `role-<member_key>`,
    /// and the role word content uses (topics, lines, conditions, nomination
    /// gates). Unique across the constitution, and never an office's id.
    pub member_key: String,
    pub selection: Selection,
    /// Seats of an at-large body (required when its constituency is
    /// `at_large`; a `ward` body has one seat per constituency).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seats: Option<u16>,
    pub term_years: u8,
    /// Its own election calendar (a chamber elected off the tier's cycle);
    /// default: the tier's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar: Option<ElectionCalendarDef>,
    pub decision: DecisionRule,
    pub powers: Vec<Power>,
    /// Where members work: a place kind (`places.jsonc` `key`).
    pub works_at: String,
}

/// A single-holder office (DEC-017). Its id is also its role word (`mayor`):
/// Fluent `role-<id>`, and the word content uses.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OfficeDef {
    /// Unique across the constitution's tiers; at most [`PostKey::MAX`] bytes.
    pub id: String,
    pub name_key: String,
    pub selection: Selection,
    pub term_years: u8,
    #[serde(default)]
    pub term_limit: Option<u8>,
    /// Its own election calendar (an office elected off the tier's cycle);
    /// default: the tier's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar: Option<ElectionCalendarDef>,
    pub powers: Vec<Power>,
    /// Where the holder works: a place kind (`places.jsonc` `key`).
    pub works_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StageDef {
    /// A vote of a body under a decision rule.
    Vote { body: String, rule: DecisionRule },
    /// An office signs or vetoes; a veto may be overridden by a body.
    Sign {
        office: String,
        override_body: Option<String>,
        override_rule: Option<DecisionRule>,
    },
}

/// The procedure ids the engine runs: every tier defines both (a contract
/// with the engine, DEC-098). Law-making follows `ordinance`; its first
/// `vote` stage names the tier's legislature and its `sign` stage the
/// tier's executive.
pub const ENGINE_PROCEDURES: [&str; 2] = ["ordinance", "budget"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProcedureDef {
    /// `ordinance` or `budget` ([`ENGINE_PROCEDURES`]).
    pub id: String,
    /// Days between introduction and the first stage (deliberation).
    pub deliberation_days: u16,
    pub stages: Vec<StageDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TierDef {
    pub tier: TierKey,
    pub bodies: Vec<BodyDef>,
    pub offices: Vec<OfficeDef>,
    pub procedures: Vec<ProcedureDef>,
    /// Sub-national balanced-budget rule.
    #[serde(default)]
    pub balanced_budget: bool,
    /// When this tier's elections happen (its elected bodies and offices,
    /// unless one has its own `calendar`).
    #[serde(default)]
    pub elections: Option<ElectionCalendarDef>,
}

/// Whether a tier votes on the same day as a higher tier (DEC-071: turnout).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ElectionTiming {
    Concurrent,
    Standalone,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ElectionCalendarDef {
    pub month: u8,
    pub day: u8,
    /// Elections fall in years where `year % term_years == cycle_offset`.
    pub cycle_offset: u8,
    pub campaign_weeks: u8,
    pub timing: ElectionTiming,
    /// Party labels appear on the ballot (otherwise a nonpartisan race).
    pub partisan_ballot: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstitutionDef {
    pub id: ContentKey,
    pub name_key: String,
    pub tiers: Vec<TierDef>,
    /// The career ladder: the summit and the acts (DEC-098; DEC-004).
    pub career: CareerLadderDef,
}

/// The career ladder (DEC-098; audit §7.5). Acts are not tiers: the summit
/// is one office, an act of its own. Validation checks that the player has
/// a path to the summit (DEC-017).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CareerLadderDef {
    /// The office (or body) at the top of a career.
    pub summit: String,
    /// The acts of a career, in order.
    pub acts: Vec<ActDef>,
}

/// One act of a career: every post of a tier (less `except`), or the named
/// posts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActDef {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<TierKey>,
    /// Posts of `tier` that belong to a later act (body or office ids).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub except: Vec<String>,
    /// The act's posts (body or office ids), when it is not a whole tier.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub posts: Vec<String>,
}

impl ActDef {
    /// Whether a post (body or office id) of tier `tier` belongs to this act.
    pub fn covers(&self, tier: TierKey, id: &str) -> bool {
        self.posts.iter().any(|p| p == id)
            || (self.tier == Some(tier) && !self.except.iter().any(|p| p == id))
    }
}

impl ConstitutionDef {
    pub fn tier(&self, t: TierKey) -> Option<&TierDef> {
        self.tiers.iter().find(|x| x.tier == t)
    }

    /// A body or an office by id, on whichever tier (ids are unique across
    /// tiers).
    pub fn post(&self, id: &str) -> Option<(TierKey, PostRef<'_>)> {
        self.tiers.iter().find_map(|t| {
            t.body(id)
                .map(PostRef::Body)
                .or_else(|| t.office(id).map(PostRef::Office))
                .map(|p| (t.tier, p))
        })
    }

    /// The role words of every post: each body's `member_key`, then each
    /// office's id, tier by tier.
    pub fn role_words(&self) -> Vec<&str> {
        self.tiers
            .iter()
            .flat_map(|t| {
                t.bodies
                    .iter()
                    .map(|b| b.member_key.as_str())
                    .chain(t.offices.iter().map(|o| o.id.as_str()))
            })
            .collect()
    }

    /// The post a role word names: a body (by its `member_key`) or an office.
    pub fn post_of_word(&self, word: &str) -> Option<(TierKey, PostRef<'_>)> {
        self.tiers.iter().find_map(|t| {
            t.bodies
                .iter()
                .find(|b| b.member_key == word)
                .map(PostRef::Body)
                .or_else(|| t.office(word).map(PostRef::Office))
                .map(|p| (t.tier, p))
        })
    }

    /// The act a post (body or office id) of `tier` belongs to.
    pub fn act_of(&self, tier: TierKey, id: &str) -> Option<&ActDef> {
        self.career.acts.iter().find(|a| a.covers(tier, id))
    }

    /// The derived career ladder (audit §7.5): tier by tier (local,
    /// regional, national), seats before offices within a tier.
    pub fn ladder(&self) -> Vec<(TierKey, PostRef<'_>)> {
        let mut tiers: Vec<&TierDef> = self.tiers.iter().collect();
        tiers.sort_by_key(|t| t.tier);
        tiers
            .into_iter()
            .flat_map(|t| t.posts().map(move |p| (t.tier, p)))
            .collect()
    }
}

/// A body or an office of a constitution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PostRef<'a> {
    Body(&'a BodyDef),
    Office(&'a OfficeDef),
}

impl<'a> PostRef<'a> {
    pub fn id(self) -> &'a str {
        match self {
            PostRef::Body(b) => &b.id,
            PostRef::Office(o) => &o.id,
        }
    }
    /// The role word: a body's `member_key`, an office's id.
    pub fn word(self) -> &'a str {
        match self {
            PostRef::Body(b) => &b.member_key,
            PostRef::Office(o) => &o.id,
        }
    }
    pub fn name_key(self) -> &'a str {
        match self {
            PostRef::Body(b) => &b.name_key,
            PostRef::Office(o) => &o.name_key,
        }
    }
    pub fn selection(self) -> &'a Selection {
        match self {
            PostRef::Body(b) => &b.selection,
            PostRef::Office(o) => &o.selection,
        }
    }
    pub fn powers(self) -> &'a [Power] {
        match self {
            PostRef::Body(b) => &b.powers,
            PostRef::Office(o) => &o.powers,
        }
    }
    pub fn term_years(self) -> u8 {
        match self {
            PostRef::Body(b) => b.term_years,
            PostRef::Office(o) => o.term_years,
        }
    }
    /// A body has no term limit; an office may.
    pub fn term_limit(self) -> Option<u8> {
        match self {
            PostRef::Body(_) => None,
            PostRef::Office(o) => o.term_limit,
        }
    }
    /// The post's own election calendar, if it has one.
    pub fn calendar(self) -> Option<&'a ElectionCalendarDef> {
        match self {
            PostRef::Body(b) => b.calendar.as_ref(),
            PostRef::Office(o) => o.calendar.as_ref(),
        }
    }
    pub fn works_at(self) -> &'a str {
        match self {
            PostRef::Body(b) => &b.works_at,
            PostRef::Office(o) => &o.works_at,
        }
    }
    /// Chosen by the voters (an election, not an appointment).
    pub fn elected(self) -> bool {
        matches!(self.selection(), Selection::Election { .. })
    }
    /// Its election's constituency, if the voters choose it.
    pub fn constituency(self) -> Option<Constituency> {
        match self.selection() {
            Selection::Election { constituency, .. } => Some(*constituency),
            _ => None,
        }
    }
}

/// A constitution id (a body's or an office's) carried by a post, copied
/// freely: at most [`PostKey::MAX`] bytes, saved as a plain string.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostKey {
    len: u8,
    bytes: [u8; PostKey::MAX],
}

impl PostKey {
    /// The longest body or office id (validation refuses longer ones).
    pub const MAX: usize = 31;

    /// `None` when `s` is empty or longer than [`PostKey::MAX`] bytes.
    pub fn new(s: &str) -> Option<Self> {
        let b = s.as_bytes();
        if b.is_empty() || b.len() > Self::MAX {
            return None;
        }
        let mut bytes = [0u8; Self::MAX];
        bytes[..b.len()].copy_from_slice(b);
        Some(Self {
            len: b.len() as u8,
            bytes,
        })
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or_default()
    }
}

impl PartialOrd for PostKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PostKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialEq<str> for PostKey {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<String> for PostKey {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl std::fmt::Debug for PostKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

impl std::fmt::Display for PostKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for PostKey {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PostKey {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        PostKey::new(&s).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "`{s}` is not a body or office id (1 to {} bytes)",
                PostKey::MAX
            ))
        })
    }
}

impl TierDef {
    pub fn procedure(&self, id: &str) -> Option<&ProcedureDef> {
        self.procedures.iter().find(|p| p.id == id)
    }
    pub fn body(&self, id: &str) -> Option<&BodyDef> {
        self.bodies.iter().find(|b| b.id == id)
    }
    pub fn office(&self, id: &str) -> Option<&OfficeDef> {
        self.offices.iter().find(|o| o.id == id)
    }
    /// Every post of the tier: its bodies, then its offices.
    pub fn posts(&self) -> impl Iterator<Item = PostRef<'_>> {
        self.bodies
            .iter()
            .map(PostRef::Body)
            .chain(self.offices.iter().map(PostRef::Office))
    }
    /// The tier's legislature: the body of the `ordinance` procedure's first
    /// `vote` stage (by procedure, never by name; DEC-098).
    pub fn legislature(&self) -> Option<&BodyDef> {
        self.procedure("ordinance")?
            .stages
            .iter()
            .find_map(|s| match s {
                StageDef::Vote { body, .. } => self.body(body),
                StageDef::Sign { .. } => None,
            })
    }
    /// The tier's executive: the office of the `ordinance` procedure's
    /// `sign` stage.
    pub fn executive(&self) -> Option<&OfficeDef> {
        self.procedure("ordinance")?
            .stages
            .iter()
            .find_map(|s| match s {
                StageDef::Sign { office, .. } => self.office(office),
                StageDef::Vote { .. } => None,
            })
    }
    /// Which body or office holds a power (first match; REQ-009: no archetype names).
    pub fn holder_of(&self, p: Power) -> Option<&str> {
        self.offices
            .iter()
            .find(|o| o.powers.contains(&p))
            .map(|o| o.id.as_str())
            .or_else(|| {
                self.bodies
                    .iter()
                    .find(|b| b.powers.contains(&p))
                    .map(|b| b.id.as_str())
            })
    }
}
