//! Worker protocol DTOs (DEC-043). These types are the single source of truth
//! for the TypeScript client; `cargo test -p mandate-protocol` regenerates
//! `web/src/engine/protocol/*.ts`.

pub mod campaign;
pub mod map;
pub mod office;
pub mod views;

pub use campaign::*;
pub use map::*;
pub use office::*;
use serde::{Deserialize, Serialize};
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
    pub preset: Option<PresetDto>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum PresetDto {
    IndustrialDecline,
    WealthyCoast,
    TechGrowth,
    DroughtFarmland,
    ResourceFrontier,
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
    },
    Fundraise,
    Rest,
    Advertise,
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
    Municipality {
        id: u32,
    },
    /// Fluent sources for a language (DEC-054).
    Locale {
        lang: String,
    },
    Diagnostics,
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
    Municipality(MunicipalityView),
    Locale(LocaleView),
    Diagnostics(DiagnosticsView),
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
                preset: None,
            }),
        };
        let j = serde_json::to_string(&r).unwrap_or_default();
        assert_eq!(
            j,
            r#"{"protocol_version":1,"request_id":7,"body":{"kind":"new_game","payload":{"seed":"18446744073709551615","size":"small","preset":null}}}"#
        );
        let back: Request = serde_json::from_str(&j).unwrap_or(r.clone());
        assert_eq!(back, r);
    }
}
