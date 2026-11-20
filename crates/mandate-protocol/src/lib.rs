//! Worker protocol DTOs (DEC-043). These types are the single source of truth
//! for the TypeScript client; `cargo test -p mandate-protocol` regenerates
//! `web/src/engine/protocol/*.ts`.

pub mod campaign;
pub mod characters;
#[cfg(feature = "dev-tools")]
pub mod dev;
pub mod knowledge;
pub mod layout;
pub mod map;
pub mod media;
pub mod narrative;
pub mod office;
pub mod places;
pub mod talk;
pub mod town;
pub mod views;

pub use campaign::*;
pub use characters::*;
pub use knowledge::*;
pub use layout::*;
pub use map::*;
pub use media::*;
pub use narrative::*;
pub use office::*;
pub use places::*;
use serde::{Deserialize, Serialize};
pub use talk::*;
pub use town::*;
use ts_rs::TS;
pub use views::*;

pub const PROTOCOL_VERSION: u32 = 1;

/// Serialize `u64` as a decimal string (JavaScript numbers lose precision).
pub mod u64_string {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Request {
    pub protocol_version: u32,
    pub request_id: u32,
    pub body: RequestBody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum WorldSizeDto {
    Small,
    Medium,
    Large,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct NewGameParams {
    #[serde(with = "u64_string")]
    #[ts(type = "string")]
    pub seed: u64,
    pub size: WorldSizeDto,
    /// Scenario content id (from `QueryDto::Scenarios`); `None` = a random country.
    pub scenario: Option<String>,
    /// Storyteller profile id (from `QueryDto::Storytellers`, DEC-033);
    /// `None` = the content's default.
    #[serde(default)]
    pub storyteller: Option<String>,
    /// "Start with local knowledge" (DEC-091; off by default): the town's
    /// worries, the broker, the council and who works where, as a local
    /// would know them. Engine state, chosen only at new game.
    #[serde(default)]
    pub local_knowledge: bool,
}

/// A starting world the title screen offers (DEC-078 decision 4).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScenarioView {
    pub id: String,
    pub name_key: String,
}

/// The scenario registry, in load order (available before a game starts).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScenarioListView {
    pub scenarios: Vec<ScenarioView>,
}

/// A storyteller profile the new-game screen offers (DEC-033).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StorytellerView {
    pub id: String,
    pub name_key: String,
    pub desc_key: String,
    pub default: bool,
}

/// The storyteller profiles, in load order (available before a game starts).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StorytellerListView {
    pub storytellers: Vec<StorytellerView>,
}

pub fn storyteller_list(content: &mandate_content::ContentBundle) -> StorytellerListView {
    StorytellerListView {
        storytellers: content
            .storytellers
            .iter()
            .map(|s| StorytellerView {
                id: s.id.clone(),
                name_key: s.name_key.clone(),
                desc_key: s.desc_key.clone(),
                default: s.id == content.director.default_profile,
            })
            .collect(),
    }
}

pub fn scenario_list(content: &mandate_content::ContentBundle) -> ScenarioListView {
    ScenarioListView {
        scenarios: content
            .scenarios
            .iter()
            .map(|x| ScenarioView {
                id: x.id.clone(),
                name_key: x.name_key.clone(),
            })
            .collect(),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandDto {
    EndTurn,
    AdvanceDays {
        days: u32,
    },
    DraftPolicy {
        template: String,
        params: Vec<ParamValue>,
    },
    AmendPolicy {
        policy: u32,
        params: Vec<ParamValue>,
        clauses: Vec<ClauseDto>,
    },
    IntroducePolicy {
        policy: u32,
    },
    WithdrawPolicy {
        policy: u32,
    },
    Act {
        action: ActionDto,
        late_night: bool,
    },
    /// Declare intentions for the next local election.
    File {
        filing: FilingDto,
    },
    /// Declare the player's vote on a bill (public and final).
    CastVote {
        policy: u32,
        vote: VoteDto,
    },
    /// Move within the player's town: free (DEC-079).
    MoveTo {
        location: LocationDto,
    },
    /// Move, then do a command there, in one request (auto-routing). If the
    /// command fails, the move still stands.
    Go {
        location: LocationDto,
        then: Option<Box<CommandDto>>,
    },
    /// Meet a passer-by where the player is (free): they become a Named
    /// character, whose id is the result's `created`.
    Meet {
        slot: u8,
    },
    /// Answer a pending decision (DEC-077 §4), from anywhere (DEC-079).
    AnswerDecision {
        decision: u32,
        option: String,
    },
    /// The player has looked at their notebook (DEC-091, K5): what they
    /// learned is no longer new, and the badge clears. No slot, no time.
    NotebookSeen,
    /// Talk to someone here on a topic (DEC-093): half a slot (late at
    /// night, the late slot). The result's `created` is the exchange's id
    /// (`QueryDto::Exchange`). Someone elsewhere: `Go` there first.
    Talk {
        to: TalkTargetDto,
        /// Topic content id (`TopicView.id`).
        topic: String,
        about: Option<AboutRefDto>,
        late_night: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum VoteDto {
    Yes,
    No,
    Abstain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum FilingDto {
    Council,
    Mayor,
    Retire,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionDto {
    Lobby {
        politician: u32,
        policy: u32,
    },
    TradeFavor {
        politician: u32,
        policy: u32,
    },
    TownHall {
        neighborhood: u32,
        policy: Option<u32>,
    },
    Press {
        policy: Option<u32>,
    },
    Canvass {
        neighborhood: u32,
        /// The worry raised on the doorsteps (a concern id, DEC-091): the
        /// neighbourhood's real top worry works better, one it barely feels
        /// worse. Omitted: a general canvass.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        issue: Option<String>,
    },
    Fundraise,
    Rest,
    Advertise,
    /// Shady (DEC-034): late at night, at the diner.
    TakeBribe,
    /// Shady: at the party office.
    DipFunds,
    /// Shady: at the newspaper office (about the rival).
    LeakFile,
    /// A verb the place offers (places with purpose): `PlacePanelView.verbs`.
    Visit { verb: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ActionResultView {
    pub created: Option<u32>,
    pub outcome_key: String,
    pub args: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueryDto {
    Overview,
    Office,
    Campaign,
    Map,
    /// The country map's layers (DEC-089): every town's public figures and
    /// each registry layer's legend and classes. Small; refresh each turn.
    MapLayers,
    Municipality {
        id: u32,
    },
    /// The pocket: always-available, coarse information and the agenda's routes.
    Pocket,
    /// The player's town as places, grouped by neighbourhood.
    Town,
    /// A place panel: what can be done and learned at a location.
    Place {
        location: LocationDto,
    },
    /// A character sheet (DEC-031).
    Character {
        id: u32,
    },
    /// A decision, pending or answered (with its aftermath).
    Decision {
        id: u32,
    },
    /// The story so far: chapter, tension, storylines, past decisions.
    Story,
    /// The newspaper: what the outlets reaching the player's town run (DEC-034).
    Newspaper,
    /// Fluent sources for a language (DEC-054).
    Locale {
        lang: String,
    },
    /// The scenario registry (no game needed).
    Scenarios,
    /// The storyteller profiles a new game can choose (no game needed).
    Storytellers,
    /// What a draft of a policy with these parameters would cost and do
    /// (the catalogue's sliders; the engine computes it).
    PolicyPreview {
        template: String,
        params: Vec<ParamValue>,
    },
    Diagnostics,
    /// The town layout v2 registries (no game needed; cache by `content_hash`).
    LayoutContent,
    /// A town's geography for layout v2 (TOWN_GENERATION §4.1): static for
    /// a run, quantised.
    TownGeography {
        id: u32,
    },
    /// A town this month for layout v2's dress pass (§4.2), rounded unless
    /// the player is at their own town hall (DEC-079).
    TownState {
        id: u32,
    },
    /// What the player knows, by subject, with leads (DEC-091 notebook).
    Notebook,
    /// The talk panel for someone (DEC-093): topics, odds, cost. A
    /// passer-by where the player is (`ambient`) is met as the talk starts.
    Talk {
        to: TalkTargetDto,
        late_night: bool,
    },
    /// A conversation the player had (the `Talk` command's `created`).
    Exchange {
        id: u32,
    },
    /// The worries the player can canvass on in a neighbourhood, with the
    /// engine's match (DEC-091 §1.6).
    Canvass {
        neighborhood: u32,
    },
    /// Dev builds only (`dev-tools`): every content item and whether it
    /// could fire now (the content workbench). Not in the generated types.
    #[cfg(feature = "dev-tools")]
    #[ts(skip)]
    DevCatalog,
    /// Dev builds only (`dev-tools`): why an item could or could not fire
    /// now, as the engine's evaluators decide, with a preview.
    #[cfg(feature = "dev-tools")]
    #[ts(skip)]
    DevExplain {
        kind: String,
        id: String,
        #[serde(default)]
        speaker: Option<u32>,
        #[serde(default)]
        place: Option<u32>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum RequestBody {
    NewGame(NewGameParams),
    Command(CommandDto),
    Query(QueryDto),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Response {
    pub request_id: u32,
    pub result: ResponseResult,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "status", rename_all = "snake_case")]
// Responses are built once and serialized immediately; boxing buys nothing.
#[allow(clippy::large_enum_variant)]
pub enum ResponseResult {
    Ok { body: ResponseBody },
    Err { error: ProtocolError },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
// One response per request; boxing views would only add indirection.
#[allow(clippy::large_enum_variant)]
pub enum ResponseBody {
    GameStarted(OverviewView),
    CommandApplied(CommandAppliedView),
    Overview(OverviewView),
    Office(OfficeView),
    Campaign(CampaignView),
    Map(MapView),
    MapLayers(MapLayersView),
    Municipality(MunicipalityView),
    Pocket(PocketView),
    Town(TownPlacesView),
    Place(PlacePanelView),
    Character(CharacterView),
    Decision(DecisionView),
    Story(StoryView),
    Newspaper(NewspaperView),
    Locale(LocaleView),
    Scenarios(ScenarioListView),
    Storytellers(StorytellerListView),
    PolicyPreview(PolicyPreviewView),
    Diagnostics(DiagnosticsView),
    LayoutContent(LayoutContentView),
    TownGeography(TownGeographyView),
    TownState(TownStateView),
    Notebook(NotebookView),
    Talk(TalkView),
    Exchange(ExchangeView),
    Canvass(CanvassView),
    #[cfg(feature = "dev-tools")]
    #[ts(skip)]
    DevCatalog(dev::DevCatalogView),
    #[cfg(feature = "dev-tools")]
    #[ts(skip)]
    DevExplain(dev::DevExplainView),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadRequest,
    VersionMismatch,
    NoGame,
    InvalidCommand,
    Content,
    Save,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProtocolError {
    pub code: ErrorCode,
    pub message: String,
}

/// Progress for long operations (generation, burn-in, time skips).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProgressEvent {
    pub request_id: u32,
    /// Fluent key describing the stage.
    pub stage_key: String,
    pub fraction: f64,
}

/// Client-supplied save fields (mirrors `mandate_save::SaveRequest`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SaveRequestDto {
    pub run_id: String,
    pub revision: u32,
    pub parent_revision: Option<u32>,
    pub device_id: String,
    pub saved_at: String,
    pub run_mode: RunModeDto,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum RunModeDto {
    Ironman,
    Story,
}

/// Save metadata visible to the client (mirrors `mandate_save::SaveMeta`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SaveMetaDto {
    pub format_version: u32,
    pub run_id: String,
    pub revision: u32,
    pub parent_revision: Option<u32>,
    pub device_id: String,
    pub saved_at: String,
    pub engine_version: String,
    pub content_hash: String,
    pub protocol_version: u32,
    pub run_mode: RunModeDto,
    pub seed: String,
    pub date: DateView,
    pub turn: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_json_shape_is_stable() {
        let r = Request {
            protocol_version: PROTOCOL_VERSION,
            request_id: 7,
            body: RequestBody::NewGame(NewGameParams {
                seed: u64::MAX,
                size: WorldSizeDto::Small,
                scenario: None,
                storyteller: None,
                local_knowledge: false,
            }),
        };
        let j = serde_json::to_string(&r).unwrap_or_default();
        assert_eq!(
            j,
            r#"{"protocol_version":1,"request_id":7,"body":{"kind":"new_game","payload":{"seed":"18446744073709551615","size":"small","scenario":null,"storyteller":null,"local_knowledge":false}}}"#
        );
        let back: Request = serde_json::from_str(&j).unwrap_or(r.clone());
        assert_eq!(back, r);
    }
}
