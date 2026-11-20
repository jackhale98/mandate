//! Posts and jurisdictions as the client sees them (DEC-098 §1, R2): a seat
//! in a body or an office, named by its constitution id, with the
//! constitution's Fluent label. Nothing here names an office or a body.

use mandate_content::{ConstitutionDef, PostKey, TierKey};
use mandate_sim::politics::Post;
use mandate_sim::world::{ConstituencyId, Jurisdiction, World};
use mandate_sim::{Engine, State};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// What kind of jurisdiction: a town, a region, or the nation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum JurisdictionKindDto {
    Municipality,
    Region,
    Nation,
}

/// A jurisdiction of any tier (DEC-098): what a post serves, where a bill or
/// a proposal applies.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct JurisdictionDto {
    pub kind: JurisdictionKindDto,
    /// The constitution tier: `local` | `regional` | `national`.
    pub tier: String,
    /// The municipality's or region's id; 0 for the nation.
    pub id: u32,
    /// The town's or region's name; empty for the nation.
    pub name: String,
    /// Fluent key that names it: `jurisdiction-<kind>` (with `$name`).
    pub name_key: String,
}

/// A seat in a body or a single-holder office.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum PostKindDto {
    Seat,
    Office,
}

/// A constitution post (DEC-017, DEC-098): a seat in a body, or an office,
/// for a jurisdiction of any tier. Labels come from the constitution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PostDto {
    pub kind: PostKindDto,
    /// The body's or the office's constitution id.
    pub id: String,
    pub at: JurisdictionDto,
    /// The seat's district number, for a seat in a districted body
    /// (`None` for an office or an at-large seat).
    pub constituency_number: Option<u32>,
    /// The constitution's Fluent key for the post: the body's or the
    /// office's `name_key`.
    pub label_key: String,
    /// Fluent key of a holder's title: `role-<word>` (a body's
    /// `member_key`, an office's id).
    pub role_key: String,
}

fn tier_word(t: TierKey) -> &'static str {
    match t {
        TierKey::Local => "local",
        TierKey::Regional => "regional",
        TierKey::National => "national",
    }
}

/// A jurisdiction for the client.
pub fn jurisdiction_dto(world: &World, at: Jurisdiction) -> JurisdictionDto {
    let (kind, name) = match at {
        Jurisdiction::Municipality { id } => (
            JurisdictionKindDto::Municipality,
            world.municipalities.get(id.index()).map(|m| m.name.clone()),
        ),
        Jurisdiction::Region { id } => (
            JurisdictionKindDto::Region,
            world.regions.get(id.index()).map(|r| r.name.clone()),
        ),
        Jurisdiction::Nation => (JurisdictionKindDto::Nation, None),
    };
    let word = match kind {
        JurisdictionKindDto::Municipality => "municipality",
        JurisdictionKindDto::Region => "region",
        JurisdictionKindDto::Nation => "nation",
    };
    JurisdictionDto {
        kind,
        tier: tier_word(at.tier()).into(),
        id: at.raw(),
        name: name.unwrap_or_default(),
        name_key: format!("jurisdiction-{word}"),
    }
}

/// The jurisdiction a client names (by kind and id), if the world has it.
pub fn jurisdiction_of(world: &World, j: &JurisdictionDto) -> Option<Jurisdiction> {
    match j.kind {
        JurisdictionKindDto::Municipality => world
            .municipalities
            .get(j.id as usize)
            .map(|m| Jurisdiction::town(m.id)),
        JurisdictionKindDto::Region => world
            .regions
            .get(j.id as usize)
            .map(|r| Jurisdiction::Region { id: r.id }),
        JurisdictionKindDto::Nation => Some(Jurisdiction::Nation),
    }
    .filter(|at| at.raw() == j.id)
}

/// The run's constitution.
pub(crate) fn constitution(engine: &Engine) -> Option<&ConstitutionDef> {
    engine
        .content()
        .constitutions
        .get(&engine.state().meta.constitution)
}

fn constituency_number(st: &State, c: Option<ConstituencyId>) -> Option<u32> {
    c.and_then(|c| st.world.constituencies.get(c.index()))
        .map(|c| c.number)
}

/// A post for the client, labelled from the constitution (an id the
/// constitution lacks keeps its id and gets empty keys).
pub fn post_dto(engine: &Engine, post: Post) -> PostDto {
    let st = engine.state();
    let def = constitution(engine).and_then(|c| post.def(c));
    PostDto {
        kind: if post.is_office() {
            PostKindDto::Office
        } else {
            PostKindDto::Seat
        },
        id: post.id().to_string(),
        at: jurisdiction_dto(&st.world, post.at()),
        constituency_number: constituency_number(st, post.constituency()),
        label_key: def.map(|d| d.name_key().to_string()).unwrap_or_default(),
        role_key: def
            .map(|d| format!("role-{}", d.word()))
            .unwrap_or_default(),
    }
}

/// The post a client names, if the constitution and the world have it: the
/// body or office by id on the jurisdiction's tier, and a seat's district
/// by its number among the jurisdiction's constituencies.
pub fn post_of(engine: &Engine, dto: &PostDto) -> Option<Post> {
    let st = engine.state();
    let at = jurisdiction_of(&st.world, &dto.at)?;
    let tier = constitution(engine)?.tier(at.tier())?;
    let key = PostKey::new(&dto.id)?;
    match dto.kind {
        PostKindDto::Office => {
            tier.office(&dto.id)?;
            Some(Post::Office { office: key, at })
        }
        PostKindDto::Seat => {
            tier.body(&dto.id)?;
            let constituency = match dto.constituency_number {
                Some(n) => Some(at.constituencies(&st.world).find(|c| c.number == n)?.id),
                None => None,
            };
            Some(Post::Seat {
                body: key,
                at,
                constituency,
            })
        }
    }
}

/// The player's home town as a jurisdiction (DEC-098: `player::home`).
pub(crate) fn home_at(st: &State) -> Option<Jurisdiction> {
    mandate_sim::player::home(st).map(Jurisdiction::town)
}
