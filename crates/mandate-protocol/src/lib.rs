//! Worker protocol DTOs (DEC-043). These types are the single source of truth
//! for the TypeScript client; `cargo test -p mandate-protocol` regenerates
//! `web/src/engine/protocol/*.ts`.

pub mod act;
pub mod arcs;
pub mod asks;
pub mod calendar;
pub mod campaign;
pub mod characters;
#[cfg(feature = "dev-tools")]
pub mod dev;
pub mod exchange;
pub mod impact;
pub mod knowledge;
pub mod layout;
pub mod map;
pub mod media;
pub mod narrative;
pub mod notebook;
pub mod office;
pub mod people;
pub mod places;
pub mod pocket;
pub mod post;
pub mod proposals;
pub mod talk;
pub mod town;
pub mod town_layers;
pub mod views;
pub mod why;

pub use act::*;
pub use arcs::*;
pub use asks::*;
pub use calendar::*;
pub use campaign::*;
pub use characters::*;
pub use exchange::*;
pub use impact::*;
pub use knowledge::*;
pub use layout::*;
pub use map::*;
pub use media::*;
pub use narrative::*;
pub use notebook::*;
pub use office::*;
pub use people::*;
pub use places::*;
pub use pocket::*;
pub use post::*;
pub use proposals::*;
use serde::{Deserialize, Serialize};
pub use talk::*;
pub use town::*;
pub use town_layers::*;
use ts_rs::TS;
pub use views::*;
pub use why::*;

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
        /// A family bill's choices (`BillView.family`): missing axes keep
        /// theirs. Refused on a standard bill. Moving away from an adopted
        /// sponsor's version costs goodwill with them.
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        choices: Option<Vec<ChoiceValue>>,
    },
    /// Draft a policy-family proposal (DEC-092 §2.11; `ProposalView`): one
    /// option per axis (missing: the axis default, or the sponsor's), the
    /// sliders, and optionally a sponsor whose version is adopted as it is
    /// (`SponsorTermsView`; its price is paid now). At the town hall, free.
    /// Refused unless `ProposalView.can_draft`. The result's `created` is
    /// the bill.
    DraftFamily {
        proposal: u32,
        choices: Vec<ChoiceValue>,
        params: Vec<ParamValue>,
        sponsor: Option<String>,
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
    /// Declare intentions for the next election (DEC-098): stand for a
    /// post (one of `CampaignView.nominations`), or retire. With neither,
    /// stand again for the post the player holds.
    File {
        post: Option<PostDto>,
        retire: bool,
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
    /// Set standing directives for time skips (DEC-012; `TermReviewView`).
    SetDirectives {
        choices: Vec<DirectiveChoiceDto>,
    },
    /// Skip ahead to a point `TermReviewView.targets` offers, at most
    /// `limit` turns now (the next `Skip` to the same target goes on): the
    /// result's `skip` is the progress; once `done`, `QueryDto::SkipSummary`.
    Skip {
        target: SkipTargetDto,
        limit: Option<u32>,
    },
    /// Stop the skip in progress (between turns).
    CancelSkip,
    /// Answer an ask (DEC-097 §2): accept it (a promise to the asker) or
    /// decline it. Free, from anywhere.
    RespondAsk {
        ask: u32,
        accept: bool,
    },
    /// Make a campaign promise from `QueryDto::PromiseMenu` (campaigning
    /// only; free). `answer`: the option's chosen bill, if it has one.
    Promise {
        template: String,
        answer: Option<String>,
    },
    /// Play a scene of an arc (DEC-112): `Begin` at its place (inside
    /// `Go`), then `Say` a line each round (`ArcSceneView.options`), or
    /// `Leave`. `beat` is `<stage>.<beat>`.
    ArcExchange {
        arc: u32,
        beat: String,
        act: SceneActDto,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionDto {
    TownHall {
        neighborhood: u32,
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
    // Fundraising, the press, the bribe and the leak are conversations
    // (DEC-114, B5): `CommandDto::Talk` with a verb's choice
    // (`TopicView`, `choice.press.*`, `choice.fundraise.*`, …).
    Rest,
    Advertise,
    /// Shady: at the party office.
    DipFunds,
    /// A verb the place offers (places with purpose): `PlacePanelView.verbs`.
    Visit {
        verb: String,
    },
    /// "Sort it out" (DEC-097 §2): casework on an accepted ask, at the town
    /// hall, one slot; the odds are `AskView.odds`.
    Casework {
        ask: u32,
    },
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
    /// The town map's layers (DEC-086, M6 note 11): for the player's home
    /// town, each registry layer with what it shows, as far as the player
    /// knows it. Small; refresh after each command.
    TownMapLayers,
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
    /// (the catalogue's sliders; the engine computes it). With `proposal`,
    /// a family proposal's sheet (`template`: its family id, or empty):
    /// the package at these `choices` (missing axes: their defaults), each
    /// option with its cost, support and impact, and the sponsors' terms.
    PolicyPreview {
        template: String,
        params: Vec<ParamValue>,
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        proposal: Option<u32>,
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        choices: Option<Vec<ChoiceValue>>,
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
    /// Term in review (DEC-012): directives and the points to skip to.
    TermReview,
    /// The last skip's summary (the interrupt or homecoming summary).
    SkipSummary,
    /// The act's Legacy record, with its epilogue once the act is over.
    Legacy,
    /// Asks and promises (DEC-097 §2): open, recent, promises, counts.
    Asks,
    /// Everyone the player has heard of, as rows in sections and in order,
    /// with why each matters now (DEC-113): one query for the whole list.
    People,
    /// One arc run as the player sees it (DEC-109, task A3): a running one
    /// (`PocketView.arcs`, `NotebookView.arc_leads`) or one that ended.
    /// Below `known`, a lead only; an arc the player knows nothing of
    /// reads as no such arc.
    Arc {
        id: u32,
    },
    /// A scene of an arc (DEC-112, `EXCHANGE_SCENES.md` §7): its audience,
    /// the prompt, the lines on offer with what each would do as far as the
    /// player can know it, the rounds played and the result. `beat` is
    /// `<stage>.<beat>` (`NeedTargetDto::Scene`, `ArcBeatView`).
    ArcExchange {
        arc: u32,
        beat: String,
    },
    /// The promises the player could make now (campaigning only).
    PromiseMenu,
    /// Why an event happened (S1, the causal spine): the chain of what led
    /// to it, at most `depth` links back (default 6, at most 12), as facts
    /// and the engine's own text keys. Read-only.
    Why {
        event: u32,
        #[serde(default)]
        depth: Option<u8>,
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
    TownMapLayers(TownMapLayersView),
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
    TermReview(TermReviewView),
    /// `None` before any skip.
    SkipSummary(Option<SkipSummaryView>),
    /// `None` before play began.
    Legacy(Option<LegacyView>),
    /// `None` before play began.
    Asks(Option<AsksView>),
    PromiseMenu(PromiseMenuView),
    Why(WhyView),
    People(PeopleView),
    Arc(ArcView),
    ArcExchange(ArcSceneView),
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

/// Who a save belongs to, as the title screen lists it (a read-only look at a
/// save: the player's name, the title of their post as a Fluent key, and
/// their home place), in the same words as the pocket's header.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SaveSummaryDto {
    pub player_name: String,
    /// Fluent key of the title of the player's first post (`role-<word>`, or `role-private`).
    pub role_key: String,
    /// The player's home place.
    pub home: String,
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
