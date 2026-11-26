//! Versioned run saves (DEC-044 rev. 2, DEC-052, TECH-009).
//!
//! File layout: `b"MNDT"` | format version (u16 LE) | meta length (u32 LE) |
//! meta JSON | DEFLATE-compressed payload JSON.
//!
//! The metadata can be read without decompressing the payload (save lists,
//! sync). The payload's BLAKE3 hash is stored in the metadata (integrity).
//! Compatibility across versions uses JSON-level migrations (`migrate`) and a
//! fixture corpus; any change to the serialized state shape must bump
//! `SAVE_FORMAT_VERSION` (enforced by the shape-fingerprint test).

pub mod runlog;

use mandate_core::hash::StableHash;
use mandate_sim::State;
use serde::{Deserialize, Serialize};

pub use runlog::{DraftEntry, DraftLog, NewGameSetup, RunLog};

pub const MAGIC: &[u8; 4] = b"MNDT";
/// v18: neighbourhood concerns (N1). v19: the run log beside the state (bug
/// reports). v20: offices from the constitution (R1, DEC-098). v21: the
/// ground game's memory (E1, DEC-097 §1). v22: the director's variety and
/// pacing cycle (E2, DEC-097 §3). v23: the act flow (time skips,
/// directives, the Legacy record and act endings; DEC-012, DEC-006). v24:
/// asks and promises and the player's deed log (E6, DEC-097 §2). v25:
/// policy families and works (F2b, DEC-092, DEC-099). v26: the causal
/// spine (S1): typed causes on every event, thread heads, the pruning rule;
/// and a politician's last post held (M6 note 16). v27: staged arcs (A1,
/// DEC-109): the arcs domain, the run a decision or an ask belongs to, arc
/// events on the spine, and repairs running faster on request. v28: the
/// director on arcs (A2, DEC-109): storylines folded into arcs, a run's
/// turn, a chapter's lead, changes voted into the next budget. v29:
/// industries and named plants (I1, DEC-110): industry state under the split
/// sectors, each industry's world index, plants and their review, a
/// sector's long average of jobs. v30: the impact estimate a law passed
/// with, kept with it (DEC-111). v31: exchange scenes (X1, DEC-112): a
/// scene under way and the scenes finished on an arc run, a story's source
/// that is a scene, the community filter of an approval request. v32: the
/// town calendar and familiarity (B1, DEC-114): the day played, the events
/// the player was at this turn, familiarity on a relationship. v33: upkeep
/// and faults (B2, DEC-114): the upkeep domain, a fault an ask is about,
/// fault events on the spine. (The arcs' infrastructure added a root a
/// cause can name, `Pressure::Worry`, with no change to the state's shape.)
pub const SAVE_FORMAT_VERSION: u16 = 33;
/// Refuse to inflate payloads larger than this (corrupt or hostile files).
pub const MAX_PAYLOAD_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Ironman,
    Story,
}

/// Sync-ready save metadata (DEC-052). Wall-clock fields are metadata only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SaveMeta {
    pub format_version: u16,
    /// Random 128-bit run identifier, hex (outside simulation state).
    pub run_id: String,
    pub revision: u64,
    pub parent_revision: Option<u64>,
    pub device_id: String,
    /// ISO-8601 wall-clock time supplied by the client.
    pub saved_at: String,
    pub engine_version: String,
    pub content_hash: String,
    pub protocol_version: u32,
    pub run_mode: RunMode,
    pub seed: u64,
    /// Simulation day number at save time.
    pub sim_date: u32,
    pub turn: u32,
    /// Hex BLAKE3 hash of the uncompressed payload.
    pub payload_hash: String,
}

/// Client-supplied fields for a new save.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SaveRequest {
    pub run_id: String,
    pub revision: u64,
    pub parent_revision: Option<u64>,
    pub device_id: String,
    pub saved_at: String,
    pub protocol_version: u32,
    pub run_mode: RunMode,
}

#[derive(Serialize, Deserialize)]
struct Payload {
    state: State,
    /// The run log (v18), outside the state: the state hash never covers it.
    #[serde(default)]
    log: Option<RunLog>,
}

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("not a Mandate save file")]
    BadMagic,
    #[error("save file is truncated or malformed")]
    Malformed,
    #[error("save format v{0} is newer than this build supports (v{SAVE_FORMAT_VERSION})")]
    TooNew(u16),
    #[error("save integrity check failed")]
    Integrity,
    #[error("save data error: {0}")]
    Data(String),
}

/// Encode a save file from the current state.
pub fn encode(state: &State, req: &SaveRequest) -> Result<(Vec<u8>, SaveMeta), SaveError> {
    encode_with_log(state, None, req)
}

/// Encode a save file from the current state and the run's log.
pub fn encode_with_log(
    state: &State,
    log: Option<&RunLog>,
    req: &SaveRequest,
) -> Result<(Vec<u8>, SaveMeta), SaveError> {
    let payload = serde_json::to_vec(&PayloadRef { state, log })
        .map_err(|e| SaveError::Data(e.to_string()))?;
    let meta = SaveMeta {
        format_version: SAVE_FORMAT_VERSION,
        run_id: req.run_id.clone(),
        revision: req.revision,
        parent_revision: req.parent_revision,
        device_id: req.device_id.clone(),
        saved_at: req.saved_at.clone(),
        engine_version: state.meta.engine_version.clone(),
        content_hash: state.meta.content_hash.clone(),
        protocol_version: req.protocol_version,
        run_mode: req.run_mode,
        seed: state.meta.seed,
        sim_date: state.calendar.date.0,
        turn: state.calendar.turn,
        payload_hash: StableHash::of_bytes(&payload).hex(),
    };
    let meta_json = serde_json::to_vec(&meta).map_err(|e| SaveError::Data(e.to_string()))?;
    let compressed = miniz_oxide::deflate::compress_to_vec(&payload, 6);
    let mut out = Vec::with_capacity(10 + meta_json.len() + compressed.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&SAVE_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&(meta_json.len() as u32).to_le_bytes());
    out.extend_from_slice(&meta_json);
    out.extend_from_slice(&compressed);
    Ok((out, meta))
}

#[derive(Serialize)]
struct PayloadRef<'a> {
    state: &'a State,
    log: Option<&'a RunLog>,
}

fn split(bytes: &[u8]) -> Result<(u16, &[u8], &[u8]), SaveError> {
    if bytes.len() < 10 {
        return Err(SaveError::Malformed);
    }
    if &bytes[..4] != MAGIC {
        return Err(SaveError::BadMagic);
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    let meta_len = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
    let meta_end = 10usize.checked_add(meta_len).ok_or(SaveError::Malformed)?;
    if meta_end > bytes.len() {
        return Err(SaveError::Malformed);
    }
    Ok((version, &bytes[10..meta_end], &bytes[meta_end..]))
}

/// Read only the metadata (cheap; used for save lists and sync).
pub fn read_meta(bytes: &[u8]) -> Result<SaveMeta, SaveError> {
    let (version, meta, _) = split(bytes)?;
    if version > SAVE_FORMAT_VERSION {
        return Err(SaveError::TooNew(version));
    }
    serde_json::from_slice(meta).map_err(|e| SaveError::Data(e.to_string()))
}

/// Decode, verify, and migrate a save file to the current format.
pub fn decode(bytes: &[u8]) -> Result<(State, SaveMeta), SaveError> {
    decode_with_log(bytes).map(|(state, _, meta)| (state, meta))
}

/// `decode`, with the run's log (`None` before v18, or when not kept).
pub fn decode_with_log(bytes: &[u8]) -> Result<(State, Option<RunLog>, SaveMeta), SaveError> {
    let meta = read_meta(bytes)?;
    let (version, _, compressed) = split(bytes)?;
    let payload = miniz_oxide::inflate::decompress_to_vec_with_limit(compressed, MAX_PAYLOAD_BYTES)
        .map_err(|_| SaveError::Malformed)?;
    if StableHash::of_bytes(&payload).hex() != meta.payload_hash {
        return Err(SaveError::Integrity);
    }
    let value: serde_json::Value =
        serde_json::from_slice(&payload).map_err(|e| SaveError::Data(e.to_string()))?;
    let value = migrate(version, value)?;
    let p: Payload = serde_json::from_value(value).map_err(|e| SaveError::Data(e.to_string()))?;
    Ok((p.state, p.log, meta))
}

/// One migration step from format `vN` to `vN+1`.
pub type MigrationStep = fn(serde_json::Value) -> Result<serde_json::Value, SaveError>;

/// JSON-level migration chain `vN → vN+1`. Add one arm per format bump.
pub fn migrate(from: u16, value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    // One step per format bump, applied in order. Pre-release policy:
    // development builds may bump the version without a migration; such
    // saves are rejected with a clear error instead.
    let steps: &[(u16, MigrationStep)] = &[
        (9, v9_scenario_ids),
        (10, v10_characters),
        (11, v11_narrative),
        (12, v12_media),
        // Town baselines (TOWN_GENERATION §15 E1): the previous format to
        // the next. Renumber only this key if another bump merges first.
        (13, town_baselines),
        // The knowledge ledger (DEC-091): the previous format to the next.
        (14, knowledge_ledger),
        // Per-neighbourhood hazard exposure (DEC-090 D8, G1): the previous
        // format to the next. Renumbered to 15 at merge (K1 took 14 → 15).
        (15, hazard_exposure),
        // Conversations (DEC-093): the previous format to the next.
        // Renumber only this key if another bump merges first.
        (16, talk_state),
        // Neighbourhood concern levels (N1): v17 → v18.
        (17, hood_concerns),
        // The run log (bug reports): v18 → v19.
        (18, run_log),
        // Offices from the constitution (DEC-098, R1): v19 → v20.
        (19, constitution_posts),
        // The ground game's memory (E1, DEC-097 §1): v20 → v21. Renumber
        // only this key if another bump merges first.
        (20, ground_game),
        // The director's variety and pacing cycle (E2, DEC-097 §3): v21 →
        // v22.
        (21, director_pacing),
        // The act flow (DEC-012, DEC-006): v22 → v23. Renumber only this
        // key if another bump merges first.
        (22, act_flow),
        // Asks and promises, the deed log (E6, DEC-097 §2): v23 → v24.
        // Renumber only this key if another bump merges first.
        (23, asks_domain),
        // Policy families and works (F2b, DEC-092, DEC-099): v24 → v25.
        // Renumber only this key if another bump merges first.
        (24, policy_families),
        // The causal spine (S1) and the last post held: v25 → v26. Renumber
        // only this key if another bump merges first.
        (25, causal_spine),
        // Staged arcs (A1, DEC-109): v26 → v27. Renumber only this key if
        // another bump merges first.
        (26, staged_arcs),
        // Storylines folded into arcs (A2, DEC-109): v27 → v28. Renumber
        // only this key if another bump merges first.
        (27, storylines_to_arcs),
        // Industries and named plants (I1, DEC-110): v28 → v29. Renumber
        // only this key if another bump merges first.
        (28, industries),
        // The impact estimate kept with a law (DEC-111): v29 → v30. Renumber
        // only this key if another bump merges first (the step names no
        // version).
        (29, frozen_estimates),
        // Exchange scenes (X1, DEC-112): v30 → v31. Renumber only this key
        // if another bump merges first.
        (30, exchange_scenes),
        // The calendar and familiarity (B1, DEC-114): the previous format to
        // the next. Renumber only this key if another bump merges first (the
        // step names no version).
        (31, calendar_familiarity),
        // Upkeep and faults (B2, DEC-114): the previous format to the next.
        // Renumber only this key if another bump merges first (the step
        // names no version).
        (32, upkeep_faults),
    ];
    let mut value = value;
    let mut v = from;
    while v < SAVE_FORMAT_VERSION {
        let Some((_, step)) = steps.iter().find(|(at, _)| *at == v) else {
            return Err(SaveError::Data(format!(
                "this save is from an older development build (format v{v}) and cannot be migrated"
            )));
        };
        value = step(value)?;
        v += 1;
    }
    Ok(value)
}

/// v9 → v10 (DEC-078 decision 4): the world's preset became a scenario
/// content id. The five presets map to the base pack's scenarios.
fn v9_scenario_ids(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    let params = value
        .pointer_mut("/state/meta/params")
        .and_then(|p| p.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    let scenario = match params.remove("preset") {
        None | Some(serde_json::Value::Null) => serde_json::Value::Null,
        Some(serde_json::Value::String(p)) => {
            let id = match p.as_str() {
                "IndustrialDecline" => "scenario.industrial_decline",
                "WealthyCoast" => "scenario.wealthy_coast",
                "TechGrowth" => "scenario.tech_growth",
                "DroughtFarmland" => "scenario.drought_farmland",
                "ResourceFrontier" => "scenario.resource_frontier",
                other => {
                    return Err(SaveError::Data(format!("unknown world preset `{other}`")));
                }
            };
            serde_json::Value::String(id.into())
        }
        Some(_) => return Err(SaveError::Malformed),
    };
    params.insert("scenario".into(), scenario);
    Ok(value)
}

/// v10 → v11 (DEC-031): politicians became characters. Each politician's
/// opinion of the player and favours move onto relationship edges and the
/// favour ledger; their character id is their politician id. The character
/// records themselves (personality, traits, competencies) are generated on
/// load from the world seed (`characters::on_load`), as are the towns' Named
/// characters and town characters.
fn v10_characters(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    use serde_json::{Value, json};
    let date = value
        .pointer("/state/calendar/date")
        .cloned()
        .unwrap_or(Value::from(0));
    let pol = value
        .pointer_mut("/state/politics")
        .and_then(|p| p.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    let list = pol
        .get_mut("politicians")
        .and_then(|p| p.as_array_mut())
        .ok_or(SaveError::Malformed)?;
    let player = list
        .iter()
        .position(|p| p.get("is_player").and_then(Value::as_bool) == Some(true));
    let mut edges = Vec::new();
    let mut favours = Vec::new();
    let n = list.len();
    for (i, p) in list.iter_mut().enumerate() {
        let obj = p.as_object_mut().ok_or(SaveError::Malformed)?;
        let opinion = obj
            .remove("opinion_of_player")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let favors = obj.remove("favors").and_then(|v| v.as_i64()).unwrap_or(0);
        obj.insert("character".into(), Value::from(i as u64));
        let Some(pc) = player.filter(|pc| *pc != i) else {
            continue;
        };
        edges.push(json!([[i, pc], { "opinion": opinion, "trust": 0.5, "bonds": [] }]));
        if favors != 0 {
            let (debtor, creditor) = if favors < 0 { (pc, i) } else { (i, pc) };
            favours.push(json!({
                "debtor": debtor, "creditor": creditor, "count": favors.unsigned_abs().min(255),
                "context": "favour-bill", "subject": null, "since": date,
            }));
        }
    }
    pol.insert(
        "characters".into(),
        json!({
            "ids": { "next": n },
            "people": [],
            "edges": edges,
            "favours": favours,
            "towns": [],
            "named_seeded": false,
            "crisis_keys": [],
        }),
    );
    Ok(value)
}

/// v11 → v12 (DEC-033, DEC-032, DEC-077 §4): the narrative domain (the
/// director, decisions, storylines, facts), NPC goals, held hazards, and a
/// held recession. The director starts fresh: its state is the default (no
/// decisions, no storylines; the first chapter opens on the next director
/// day), and so are goals (recomputed monthly) and the levers (none held).
fn v11_narrative(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    use serde_json::{Value, json};
    let st = value
        .pointer_mut("/state")
        .and_then(|p| p.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    st.remove("narrative");
    if let Some(ch) = st
        .get_mut("politics")
        .and_then(|p| p.get_mut("characters"))
        .and_then(Value::as_object_mut)
    {
        ch.insert("goals".into(), json!([]));
        ch.insert("ended_goals".into(), json!([]));
    }
    if let Some(env) = st.get_mut("environment").and_then(Value::as_object_mut) {
        env.insert("deferred".into(), json!([]));
    }
    if let Some(g) = st
        .get_mut("economy")
        .and_then(|e| e.get_mut("global"))
        .and_then(Value::as_object_mut)
    {
        g.insert("held_recession".into(), Value::Null);
    }
    Ok(value)
}

/// v12 → v13 (DEC-034, DEC-006): the media and justice domains, a career's
/// status, hidden facts' fates, and raised decisions. Media and justice start
/// empty (outlets are generated on load for a game in play); the player is
/// in play; hidden facts from before have no fate (they never come out);
/// decisions answer nothing. Every new field has a serde default, so the
/// step only drops anything a newer build might have left.
fn v12_media(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    let st = value
        .pointer_mut("/state")
        .and_then(|p| p.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    st.remove("media");
    st.remove("justice");
    if let Some(p) = st.get_mut("player").and_then(|p| p.as_object_mut()) {
        p.remove("status");
    }
    Ok(value)
}

/// Previous format → next (v13 → v14 when written; TOWN_GENERATION §15 E1):
/// town baselines. A run from before them takes its baselines from the
/// values it holds now, deterministically: each neighbourhood's residents
/// (adults and children), each sector's jobs and each town's housing stock,
/// rounded to whole units (the same rounding a new game records with). The
/// town's layout is then planned from the town as it is when first loaded,
/// and stays put from there.
fn town_baselines(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    use serde_json::Value;
    let whole = |v: Option<&Value>| -> Value {
        let x = v.and_then(Value::as_f64).unwrap_or(0.0);
        Value::from(mandate_sim::economy::whole(x))
    };
    let hoods = value
        .pointer_mut("/state/population/hoods")
        .and_then(Value::as_array_mut)
        .ok_or(SaveError::Malformed)?;
    for h in hoods {
        let h = h.as_object_mut().ok_or(SaveError::Malformed)?;
        let children = h.get("children").and_then(Value::as_u64).unwrap_or(0);
        let adults: u64 = h
            .get("cohorts")
            .and_then(Value::as_array)
            .ok_or(SaveError::Malformed)?
            .iter()
            .map(|c| c.get("count").and_then(Value::as_u64).unwrap_or(0))
            .sum();
        let total = u32::try_from(children + adults).unwrap_or(u32::MAX);
        h.insert("baseline_population".into(), Value::from(total));
    }
    let munis = value
        .pointer_mut("/state/economy/munis")
        .and_then(Value::as_array_mut)
        .ok_or(SaveError::Malformed)?;
    for mu in munis {
        let mu = mu.as_object_mut().ok_or(SaveError::Malformed)?;
        let sectors = mu
            .get_mut("sectors")
            .and_then(Value::as_array_mut)
            .ok_or(SaveError::Malformed)?;
        for s in sectors {
            let s = s.as_object_mut().ok_or(SaveError::Malformed)?;
            let jobs = whole(s.get("jobs"));
            s.insert("baseline_jobs".into(), jobs);
        }
        let housing = mu
            .get_mut("housing")
            .and_then(Value::as_object_mut)
            .ok_or(SaveError::Malformed)?;
        let stock = whole(housing.get("stock"));
        housing.insert("baseline_stock".into(), stock);
    }
    Ok(value)
}

/// Previous format → next (v14 → v15 when written; DEC-091): the player's
/// knowledge ledger. The step drops any
/// ledger a newer build might have left, so the serde default (`Unseeded`)
/// applies, and `player::knowledge::on_load` seeds it generously on load
/// (`Migrated`: the default profile, the local option, and everyone with a
/// relationship to the player known). A run in progress loses nothing its
/// player plainly knew.
fn knowledge_ledger(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    let st = value
        .pointer_mut("/state")
        .and_then(|p| p.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    if let Some(p) = st.get_mut("player").and_then(|p| p.as_object_mut()) {
        p.remove("knowledge");
    }
    Ok(value)
}

/// Previous format → next (v15 → v16 after the K1 merge; written as v14 → v15; DEC-090 D8, G1):
/// per-neighbourhood hazard exposure. A save holds no terrain (it is
/// rebuilt from the seed outside the state), so a run from before exposure
/// takes it from what the save does hold: each town's geographic profile
/// and each neighbourhood's kind (`environment::exposure::from_profile`),
/// deterministically. It starts with no damage on record, nobody displaced,
/// and every neighbourhood hit as hard as its town's average.
fn hazard_exposure(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    use serde_json::Value;
    let world: mandate_sim::world::World = serde_json::from_value(
        value
            .pointer("/state/world")
            .cloned()
            .ok_or(SaveError::Malformed)?,
    )
    .map_err(|e| SaveError::Data(e.to_string()))?;
    let exposure = mandate_sim::environment::exposure::from_profile(&world);
    let env = value
        .pointer_mut("/state/environment")
        .and_then(Value::as_object_mut)
        .ok_or(SaveError::Malformed)?;
    env.insert(
        "exposure".into(),
        serde_json::to_value(exposure).map_err(|e| SaveError::Data(e.to_string()))?,
    );
    env.insert("hood_damage".into(), Value::Array(Vec::new()));
    let pop = value
        .pointer_mut("/state/population")
        .and_then(Value::as_object_mut)
        .ok_or(SaveError::Malformed)?;
    pop.insert("hood_impact".into(), Value::Array(Vec::new()));
    pop.insert("displaced".into(), Value::Array(Vec::new()));
    Ok(value)
}

/// Previous format → next (v16 → v17 when written; DEC-093): the player's
/// conversation state (the repetition guard's used lines, the exchange log,
/// the half slot). The step drops any a newer build might have left, so the
/// serde default applies: nobody has said anything to the player yet, and no
/// half slot is pending.
fn talk_state(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    let st = value
        .pointer_mut("/state")
        .and_then(|p| p.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    if let Some(p) = st.get_mut("player").and_then(|p| p.as_object_mut()) {
        p.remove("talk");
    }
    Ok(value)
}

/// Previous format → next (v17 → v18 when written; N1): each
/// neighbourhood's own concern levels, and a town's levels split into felt
/// and measured. A town's stored levels were measured from its town-wide
/// indicators: they become `measured`, beside the salience they produced
/// (the town's politics are unchanged). Neighbourhood levels are derived
/// state: the step leaves them empty and the engine derives them on load
/// from the run's state and content, the same every time
/// (`concerns::compute_all`), with the felt town levels that aggregate them.
fn hood_concerns(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    use serde_json::Value;
    let pop = value
        .pointer_mut("/state/population")
        .and_then(Value::as_object_mut)
        .ok_or(SaveError::Malformed)?;
    if let Some(towns) = pop.get_mut("town_concerns").and_then(Value::as_array_mut) {
        for t in towns {
            let t = t.as_object_mut().ok_or(SaveError::Malformed)?;
            let levels = t.get("levels").cloned().unwrap_or(Value::Array(Vec::new()));
            t.insert("measured".into(), levels);
        }
    }
    pop.insert("hood_concerns".into(), Value::Array(Vec::new()));
    Ok(value)
}

/// Previous format → next (v18 → v19): the run log beside the
/// state (bug reports, `runlog`). A run from before it has none: its log
/// starts empty when loaded, marked as not replayable from the start. The
/// state is untouched (its hash is unchanged).
fn run_log(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    let payload = value.as_object_mut().ok_or(SaveError::Malformed)?;
    payload.insert("log".into(), serde_json::Value::Null);
    Ok(value)
}

/// Previous format → next (v19 → v20; DEC-098, R1): offices
/// from the constitution. The old format knew two posts, which existed only
/// for the base constitution's local tier: a ward councillor and a mayor.
/// They become constitution posts by id (the `council` body's seat for a
/// ward, the `mayor` office), keyed by the town's jurisdiction; this step
/// is the one place those ids are named. `Engine::from_state` then checks
/// every post against the run's constitution and refuses a save that needs
/// one it lacks (`missing_references`).
///
/// | Path | Old | New |
/// |---|---|---|
/// | `politicians/*/role` | `{"Councillor":{municipality,ward}}`, `{"Mayor":{municipality}}`, `{"Private":{municipality}}` | `{"type":"seat",…}`, `{"type":"office",…}`, `{"type":"private","home":m}` |
/// | `politics/councils[m]` | `{municipality, members, mayor}` | `bodies[]` and `offices[]` |
/// | races' and results' `seat`, memories' and favours' `{"type":"seat","seat":…}` | `{"Council":{…}}`, `{"Mayor":{…}}` | the post |
/// | `player/filing` | `"Council"`, `"Mayor"`, `"Retire"` | `{"type":"stand","post":…}`, `{"type":"retire"}` |
///
/// Events keep their shape (`RaceDecided.post` is new and optional).
fn constitution_posts(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    use serde_json::{Value, json};
    // The base constitution's local posts, as the old format knew them.
    const BODY: &str = "council";
    const OFFICE: &str = "mayor";
    let town = |m: &Value| json!({ "type": "municipality", "id": m.clone() });
    let seat = |m: &Value, ward: &Value| json!({ "type": "seat", "body": BODY, "at": town(m), "constituency": ward.clone() });
    let office = |m: &Value| json!({ "type": "office", "office": OFFICE, "at": town(m) });
    // An old role or seat (the same shapes, `Councillor` and `Council` alike).
    // A post already in the new shape (tagged by `type`) is left as it is.
    let post = |old: &Value| -> Result<Value, SaveError> {
        if old.get("type").is_some() {
            return Ok(old.clone());
        }
        let obj = old.as_object().ok_or(SaveError::Malformed)?;
        let (k, inner) = obj.iter().next().ok_or(SaveError::Malformed)?;
        let m = inner.get("municipality").ok_or(SaveError::Malformed)?;
        Ok(match k.as_str() {
            "Councillor" | "Council" => seat(m, inner.get("ward").ok_or(SaveError::Malformed)?),
            "Mayor" => office(m),
            "Private" => json!({ "type": "private", "home": m.clone() }),
            other => return Err(SaveError::Data(format!("unknown role `{other}`"))),
        })
    };
    // Memories and favours about a seat, wherever a subject is kept.
    fn subjects(
        v: &mut Value,
        post: &dyn Fn(&Value) -> Result<Value, SaveError>,
    ) -> Result<(), SaveError> {
        match v {
            Value::Object(o) => {
                if o.get("type").and_then(Value::as_str) == Some("seat")
                    && let Some(old) = o.get("seat")
                    && old.get("type").is_none()
                {
                    let new = post(old)?;
                    o.insert("seat".into(), new);
                }
                for x in o.values_mut() {
                    subjects(x, post)?;
                }
            }
            Value::Array(a) => {
                for x in a {
                    subjects(x, post)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    let pol = value
        .pointer_mut("/state/politics")
        .and_then(Value::as_object_mut)
        .ok_or(SaveError::Malformed)?;
    for p in pol
        .get_mut("politicians")
        .and_then(Value::as_array_mut)
        .ok_or(SaveError::Malformed)?
    {
        let p = p.as_object_mut().ok_or(SaveError::Malformed)?;
        let role = post(p.get("role").ok_or(SaveError::Malformed)?)?;
        p.insert("role".into(), role);
    }
    let councils = pol.remove("councils");
    let mut bodies = Vec::new();
    let mut offices = Vec::new();
    for c in councils
        .iter()
        .map(|c| c.as_array().ok_or(SaveError::Malformed))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
    {
        let m = c.get("municipality").ok_or(SaveError::Malformed)?;
        bodies.push(json!({
            "body": BODY, "at": town(m),
            "members": c.get("members").cloned().unwrap_or(json!([])),
            "works_at": "CouncilChamber",
        }));
        offices.push(json!({
            "office": OFFICE, "at": town(m),
            "holder": c.get("mayor").cloned().unwrap_or(Value::Null),
            "works_at": "TownHall",
        }));
    }
    if councils.is_some() {
        pol.insert("bodies".into(), Value::Array(bodies));
        pol.insert("offices".into(), Value::Array(offices));
    }
    if let Some(el) = pol.get_mut("elections").and_then(Value::as_object_mut) {
        for list in ["races", "results"] {
            for r in el
                .get_mut(list)
                .and_then(Value::as_array_mut)
                .into_iter()
                .flatten()
            {
                let r = r.as_object_mut().ok_or(SaveError::Malformed)?;
                let s = post(r.get("seat").ok_or(SaveError::Malformed)?)?;
                r.insert("seat".into(), s);
            }
        }
    }
    if let Some(ch) = pol.get_mut("characters") {
        subjects(ch, &post)?;
    }
    // The player's filing: their own ward's seat, or their town's office.
    if let Some(pl) = value
        .pointer_mut("/state/player")
        .and_then(Value::as_object_mut)
    {
        let m = pl.get("municipality").cloned().unwrap_or(Value::Null);
        let ward = pl.get("ward").cloned().unwrap_or(Value::Null);
        let filing = match pl.get("filing").and_then(Value::as_str) {
            None => pl.get("filing").cloned().unwrap_or(Value::Null),
            Some("Council") => json!({ "type": "stand", "post": seat(&m, &ward) }),
            Some("Mayor") => json!({ "type": "stand", "post": office(&m) }),
            Some("Retire") => json!({ "type": "retire" }),
            Some(other) => return Err(SaveError::Data(format!("unknown filing `{other}`"))),
        };
        pl.insert("filing".into(), filing);
    }
    Ok(value)
}

/// Previous format → next (v20 → v21; E1, DEC-097 §1): the player's
/// per-neighbourhood canvass memory. The step drops any a newer build might
/// have left, so the serde default applies: no doorsteps remembered, every
/// neighbourhood fresh.
fn ground_game(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    let st = value
        .pointer_mut("/state")
        .and_then(|p| p.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    if let Some(p) = st.get_mut("player").and_then(|p| p.as_object_mut()) {
        p.remove("ground");
    }
    Ok(value)
}

/// Previous format → next (v21 → v22; E2, DEC-097 §3): the director's
/// template uses, recent climaxes, the player's answers, the pacing cycle's
/// phase and log, a setback, and per chapter its theme and peaks. Every new
/// field has a serde default (none remembered, no phase: the cycle begins
/// with a build-up at the next weekly step), so a v21 file needs no change;
/// the step checks its shape.
fn director_pacing(value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    if !value.pointer("/state").is_some_and(|s| s.is_object()) {
        return Err(SaveError::Malformed);
    }
    Ok(value)
}

/// v22 → v23 (DEC-012, DEC-006): the act flow (directives, a skip in
/// progress, the Legacy record, the act's ending) is new state. A run from
/// before starts it empty: the record opens at its next turn, from there on.
fn act_flow(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    let st = value
        .pointer_mut("/state")
        .and_then(|p| p.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    st.remove("act");
    Ok(value)
}

/// v23 → v24 (E6, DEC-097 §2): `State.asks` (asks, promises, the deed log)
/// is new. A run from before has none: the step drops any a newer build
/// might have left, so the serde default applies (nothing asked, nothing
/// promised, an empty deed log; the first offer comes at the sliding rate).
fn asks_domain(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    let st = value
        .pointer_mut("/state")
        .and_then(|p| p.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    st.remove("asks");
    Ok(value)
}

/// v24 → v25 (DEC-092, DEC-099): policy families (decks, proposals,
/// cooldowns) and works projects are new state in the policy domain; a bill
/// may carry its family draft and an `area` clause, an appropriation a
/// borrowed share. All have serde defaults (no families, no works), so the
/// step checks the shape and leaves any partial family state out: the decks
/// are drawn on load (keyed, as a fresh start would draw them), and no
/// proposal exists before the next monthly pass.
fn policy_families(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    let policy = value
        .pointer_mut("/state/policy")
        .and_then(|p| p.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    policy.remove("families");
    policy.remove("works");
    Ok(value)
}

/// v25 → v26 (S1, `docs/design/CAUSAL_SPINE.md`): the causal spine.
///
/// - An event's `causes` were ids of earlier events (laws and closures
///   only). Each becomes a typed link: `ended` for a bill that failed or a
///   law that expired, `continues` for an enactment, `caused` otherwise. An
///   id the log no longer holds becomes `forgotten`.
/// - An event with no recorded cause gets `unknown`: recorded before the
///   spine. Nothing is invented (not even "exogenous" for a disaster).
/// - Pins the new rule no longer needs are dropped (laws, closures and
///   track warnings are kept by salience and by live state instead), so an
///   old log shrinks to the rule's bound at its first prune.
/// - The threads' latest events (`heads`) are found by replaying the
///   log, so that what happens next to a bill, an ask or a case from before
///   the spine links to it. The thread table below is the engine's at v26
///   (`spine::thread`), frozen here.
///
/// The other new fields default: a hidden fact's `cause`, a Legacy entry's
/// `source_event`, the log's `pruned_on`. A politician's `last_post` is
/// filled where the kept election results show it (`last_post_held`).
fn causal_spine(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    use serde_json::{Value, json};
    let log = value
        .pointer_mut("/state/events")
        .and_then(|e| e.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    // A log that already has its threads is already the spine's (the
    // tests of older steps hand this chain a current state with one part
    // taken back): it is left as it is.
    if log.contains_key("heads") {
        last_post_held(&mut value)?;
        return Ok(value);
    }
    let events = log
        .get_mut("events")
        .and_then(|e| e.as_array_mut())
        .ok_or(SaveError::Malformed)?;
    let ids: Vec<u64> = events
        .iter()
        .filter_map(|e| e.get("id").and_then(Value::as_u64))
        .collect();
    // (rank, payload) orders threads as the engine's `Thread` does.
    let mut heads: std::collections::BTreeMap<(u8, u64), (Value, u64, bool)> = Default::default();
    for e in events.iter_mut() {
        let id = e
            .get("id")
            .and_then(Value::as_u64)
            .ok_or(SaveError::Malformed)?;
        let date = e.get("date").and_then(Value::as_u64).unwrap_or(0);
        let kind = e.get("kind").cloned().unwrap_or(Value::Null);
        let tag = kind.get("type").and_then(Value::as_str).unwrap_or("");
        let num = |k: &str| kind.get(k).and_then(Value::as_u64);
        // Causes.
        let link = match tag {
            "policy_failed" | "policy_expired" => "ended",
            "policy_enacted" => "continues",
            _ => "caused",
        };
        let old = e
            .get("causes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut causes: Vec<Value> = Vec::new();
        for c in old.iter().filter_map(Value::as_u64) {
            let new = if ids.binary_search(&c).is_ok() {
                json!({ "event": [c, link] })
            } else {
                json!("forgotten")
            };
            if !causes.contains(&new) {
                causes.push(new);
            }
        }
        if causes.is_empty() {
            causes.push(json!("unknown"));
        }
        let obj = e.as_object_mut().ok_or(SaveError::Malformed)?;
        obj.insert("causes".into(), Value::Array(causes));
        // Pins the rule replaces.
        if matches!(
            tag,
            "policy_enacted" | "policy_failed" | "track_warning" | "plant_closure"
        ) {
            obj.insert("retention".into(), json!("Recent"));
        }
        // Threads: (rank, key, thread as saved, role).
        let deed = kind.get("deed");
        let deed_tag = deed
            .and_then(|d| d.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let lever = kind.get("lever").and_then(Value::as_str).unwrap_or("");
        let stage = kind.get("stage").and_then(Value::as_str).unwrap_or("");
        let one = |rank: u8, name: &str, key: Option<u64>, role: char| {
            key.map(|k| (rank, k, json!({ name: k }), role))
        };
        let unit = |rank: u8, name: &str, role: char| Some((rank, 0, json!(name), role));
        // Roles: o opens, a advances, n notes, c closes.
        let thread = match tag {
            "policy_introduced" => one(0, "bill", num("policy"), 'o'),
            "policy_enacted" => one(0, "bill", num("policy"), 'a'),
            "policy_failed" | "policy_expired" | "policy_withdrawn" => {
                one(0, "bill", num("policy"), 'c')
            }
            "player_deed" if deed_tag == "vote" => one(
                0,
                "bill",
                deed.and_then(|d| d.get("policy")).and_then(Value::as_u64),
                'a',
            ),
            "proposal_opened" => one(1, "proposal", num("proposal"), 'o'),
            "proposal_lapsed" => one(1, "proposal", num("proposal"), 'c'),
            "ask_offered" => one(2, "ask", num("ask"), 'o'),
            "ask_accepted" => one(2, "ask", num("ask"), 'a'),
            "ask_resolved" => one(2, "ask", num("ask"), 'c'),
            "decision_opened" => one(3, "decision", num("decision"), 'o'),
            "decision_resolved" | "decision_lapsed" => one(3, "decision", num("decision"), 'c'),
            "fact_discoverable" => one(4, "hidden", num("hidden"), 'o'),
            "scandal_broke" => one(5, "scandal", num("scandal"), 'o'),
            "case_opened" => one(6, "case", num("case"), 'o'),
            "case_developed" => one(
                6,
                "case",
                num("case"),
                if matches!(stage, "dropped" | "acquitted" | "convicted") {
                    'c'
                } else {
                    'a'
                },
            ),
            "rumour_spread" => one(7, "rumour", num("rumour"), 'o'),
            "rumour_corrected" => one(7, "rumour", num("rumour"), 'c'),
            "storyline_stage" => one(8, "storyline", num("run"), 'a'),
            "global_recession_began" => unit(9, "recession", 'a'),
            "global_recovery_began" => unit(9, "recession", 'c'),
            "campaign_opened" => one(10, "election", num("election_day"), 'o'),
            "election_held" => one(10, "election", Some(date), 'c'),
            "player_imprisoned" => unit(11, "sentence", 'o'),
            "player_released" => unit(11, "sentence", 'c'),
            "director_intervention" => match lever {
                "recession_held" => unit(9, "recession", 'o'),
                "recession_released" => unit(9, "recession", 'a'),
                _ => None,
            },
            _ => None,
        };
        if let Some((rank, key, saved, role)) = thread
            && role != 'n'
        {
            // The thread's latest event, and whether it closed the thread.
            heads.insert((rank, key), (saved, id, role == 'c'));
        }
    }
    let heads: Vec<Value> = heads
        .into_values()
        .map(|(thread, id, closed)| json!([thread, id, closed]))
        .collect();
    log.insert("heads".into(), Value::Array(heads));
    last_post_held(&mut value)?;
    Ok(value)
}

/// v25 → v26, the last post held (M6 note 16): known for someone out of
/// office whom the kept election results show as a post's incumbent who
/// did not win it again (and who won nothing later): that post, ended on
/// that election's day. Anyone else's is left empty: a post lost longer
/// ago, or to a sentence, was never recorded.
fn last_post_held(value: &mut serde_json::Value) -> Result<(), SaveError> {
    use serde_json::{Value, json};
    let results = value
        .pointer("/state/politics/elections/results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let Some(list) = value
        .pointer_mut("/state/politics/politicians")
        .and_then(Value::as_array_mut)
    else {
        return Err(SaveError::Malformed);
    };
    for (i, p) in list.iter_mut().enumerate() {
        let private = p.pointer("/role/type").and_then(Value::as_str) == Some("private");
        let Some(obj) = p
            .as_object_mut()
            .filter(|o| private && !o.contains_key("last_post"))
        else {
            continue;
        };
        let id = obj.get("id").and_then(Value::as_u64).unwrap_or(i as u64);
        // Results are kept in date order: the latest that concerns them.
        let mut last: Option<(Value, Value)> = None;
        for r in &results {
            let won = r.get("winner").and_then(Value::as_u64) == Some(id);
            let sat = r.get("incumbent").and_then(Value::as_u64) == Some(id);
            if won {
                last = None;
            } else if sat && let (Some(seat), Some(date)) = (r.get("seat"), r.get("date")) {
                last = Some((seat.clone(), date.clone()));
            }
        }
        if let Some((post, ended)) = last {
            obj.insert("last_post".into(), json!({ "post": post, "ended": ended }));
        }
    }
    Ok(())
}

/// v26 → v27 (A1, DEC-109, `docs/design/STAGED_ARCS.md`): staged arcs.
///
/// An older save holds no arc, so nothing in it is rewritten: the arcs
/// domain is absent (an empty domain is never written, so that a game no
/// arc has touched hashes as it did), no decision or ask belongs to a run,
/// no repairs run faster, and the log holds no arc event. The step checks
/// that this is so and refuses a payload that claims the older format and
/// carries arc state anyway (it was not written by a v26 build).
///
/// What the first turn after loading does is the simulation's, not the
/// migration's: a matter already under way (a flood within the month)
/// raises its arc at the next weekly step, at the stage the world is at.
/// Running storylines stay storylines until the next step folds them into
/// arcs ([`storylines_to_arcs`]).
fn staged_arcs(value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    let state = value.get("state").ok_or(SaveError::Malformed)?;
    if state.get("arcs").is_some() {
        return Err(SaveError::Data(
            "a save older than v27 cannot hold arc runs".into(),
        ));
    }
    Ok(value)
}

/// The impact estimate kept with a law (DEC-111): the previous format to
/// the next. From this format a law carries `estimate`, what its bill said
/// it would do when it passed. An older law has none and none is invented:
/// what a bill would have said then cannot be known from the state now, so
/// such a law's account simply has no "estimated then" column.
///
/// The one thing the step rewrites is a ledger entry's name: homes a
/// programme started were booked as `homes_built` before the ledger told
/// started from added (audit fix 5). They are `homes_started` here, as the
/// engine now writes them, so an old law's account reads in the same words
/// as a new one's from the moment it loads.
fn frozen_estimates(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    let laws = value
        .get_mut("state")
        .and_then(|s| s.get_mut("policy"))
        .and_then(|p| p.get_mut("instances"))
        .and_then(|i| i.as_array_mut())
        .ok_or(SaveError::Malformed)?;
    for law in laws {
        let Some(ledger) = law.get_mut("ledger").and_then(|l| l.as_object_mut()) else {
            continue;
        };
        if let Some(built) = ledger.remove("homes_built") {
            // The entry under its new name, as it was; added to one already
            // there (a save from the months between the two names).
            let total = match ledger.get("homes_started").and_then(|v| v.as_f64()) {
                Some(started) => serde_json::json!(started + built.as_f64().unwrap_or(0.0)),
                None => built,
            };
            ledger.insert("homes_started".into(), total);
        }
    }
    Ok(value)
}

/// Exchange scenes (X1, DEC-112, `docs/design/EXCHANGE_SCENES.md` §11.5),
/// the previous format to the next (the step names no version). Everything
/// new has a serde default and is not written when empty (a run's scene
/// under way and its finished scenes, the community filter of an approval
/// request), so the state needs no rewriting.
///
/// One thing in flight is dropped (rule 9 of `AGENT_CONTEXT.md`: the cheap
/// rule). The rival's debate was a decision, and is a scene now: a
/// challenge waiting on the player in the save is withdrawn with no effect
/// (it lapses, as a decision does whose situation dissolved), and the
/// debate is not held for that campaign (an older run has no hall cast).
/// The run itself carries on by its edges.
/// The town calendar and familiarity (B1, DEC-114). The cheapest step:
/// everyone starts a stranger (no familiarity: the field's default), no
/// event attended this turn, and no day played kept, so the engine chooses
/// this turn's day as it loads the state (`Engine::from_state`).
fn calendar_familiarity(value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    Ok(value)
}

/// Upkeep and faults (B2, DEC-114). Nothing to rewrite: the upkeep
/// domain is new and empty in an older save (its default), and the engine
/// keeps the focus's condition from the next month on; no ask in it is
/// about a fault.
fn upkeep_faults(value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    Ok(value)
}

fn exchange_scenes(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    /// Decisions that were an arc's beat and are an exchange now.
    const RETIRED: [&str; 1] = ["dilemma.debate_challenge"];
    let decisions = value
        .get_mut("state")
        .and_then(|s| s.get_mut("narrative"))
        .and_then(|n| n.get_mut("decisions"))
        .and_then(|d| d.as_array_mut());
    for d in decisions.into_iter().flatten() {
        let retired = d
            .get("template")
            .and_then(|t| t.as_str())
            .is_some_and(|t| RETIRED.contains(&t));
        let waiting = d.get("status").and_then(|s| s.as_str()) == Some("Pending");
        if retired
            && waiting
            && let Some(o) = d.as_object_mut()
        {
            o.insert("status".into(), serde_json::json!("Lapsed"));
        }
    }
    Ok(value)
}

/// v28 → v29 (I1, DEC-110): industries and named plants. The step itself
/// only gives the economy its new, empty fields and starts each sector's
/// long average of jobs at its jobs. Sharing a split sector among its
/// industries needs the content's registry and the world's geography, so
/// the engine does it as it loads the state (`economy::industries::remap`,
/// as it re-keys sectors): each split sector's capacity, jobs, wage and
/// productivity are shared by the siting function worldgen uses (keyed
/// draws; no stream advances), the last industry taking the remainder, so
/// every sector total is what it was; and the works a town already has
/// become named plants (one each, the town's largest industry of that kind
/// of works), taking their capacity out of the unnamed firms. No place
/// appears in a town the player knows. Indices start at 1 and nothing is
/// under review. Old `plant_closure` events keep no plant; their text is
/// the sector's line.
fn industries(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    use serde_json::json;
    let econ = value
        .get_mut("state")
        .and_then(|s| s.get_mut("economy"))
        .and_then(|e| e.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    // A payload that already holds industry state keeps it (the tests that
    // strip one newer field from today's state and migrate from an old
    // version rely on every later step leaving what is there alone).
    for (key, empty) in [
        ("industry_keys", json!([])),
        ("market_keys", json!([])),
        ("markets", json!([])),
        ("good_keys", json!([])),
        ("plants", json!([])),
        ("plant_ids", json!({ "next": 0 })),
    ] {
        econ.entry(key).or_insert(empty);
    }
    for mu in econ
        .get_mut("munis")
        .and_then(|m| m.as_array_mut())
        .ok_or(SaveError::Malformed)?
    {
        for s in mu
            .get_mut("sectors")
            .and_then(|s| s.as_array_mut())
            .ok_or(SaveError::Malformed)?
        {
            let jobs = s.get("jobs").cloned().unwrap_or(json!(0.0));
            if let Some(o) = s.as_object_mut() {
                o.entry("trend_jobs").or_insert(jobs);
            }
        }
    }
    Ok(value)
}

/// What a storyline of the v27 pack became (the pack as it shipped then:
/// `storylines.jsonc` is gone, so the table lives here).
struct FoldedStoryline {
    storyline: &'static str,
    arc: &'static str,
    /// The arc's heat as a run is raised.
    heat: f64,
    /// The storyline's stages a run can rest at; the arc has stages of
    /// the same ids. (Its endings are reached and over in one step.)
    stages: &'static [&'static str],
    /// The arc's turn, among them.
    turn: &'static str,
    /// A stage's decision: (dilemma, stage, the arc's beat).
    decisions: &'static [(&'static str, &'static str, &'static str)],
    /// What an answer set: (dilemma, option, mark).
    marks: &'static [(&'static str, &'static str, &'static str)],
}

const FOLDED: [FoldedStoryline; 3] = [
    FoldedStoryline {
        storyline: "storyline.rival",
        arc: "arc.rival",
        heat: 0.3,
        stages: &["shadow", "offer", "pact", "showdown"],
        turn: "showdown",
        decisions: &[
            ("dilemma.rival_offer", "offer", "terms"),
            ("dilemma.debate_challenge", "showdown", "debate"),
        ],
        marks: &[
            ("dilemma.rival_offer", "truce", "pact"),
            ("dilemma.rival_offer", "refuse", "refused"),
            ("dilemma.debate_challenge", "accept", "debated"),
        ],
    },
    FoldedStoryline {
        storyline: "storyline.favour_debt",
        arc: "arc.favour_debt",
        heat: 0.1,
        stages: &["debt", "reminder", "called_in"],
        turn: "called_in",
        decisions: &[("dilemma.favour_called_in", "called_in", "call")],
        marks: &[
            ("dilemma.favour_called_in", "pay_back", "paid"),
            ("dilemma.favour_called_in", "refuse", "refused"),
            ("dilemma.favour_called_in", "dodge", "dodged"),
        ],
    },
    FoldedStoryline {
        storyline: "storyline.plant_closure",
        arc: "arc.plant_closure",
        heat: 0.3,
        stages: &["layoffs", "walkout", "organising"],
        turn: "organising",
        decisions: &[
            ("dilemma.plant_layoffs", "layoffs", "sides"),
            ("dilemma.workers_campaign", "organising", "campaign"),
        ],
        marks: &[
            ("dilemma.plant_layoffs", "back_owner", "with_owner"),
            ("dilemma.plant_layoffs", "back_workers", "with_workers"),
            ("dilemma.plant_layoffs", "broker_talks", "brokered"),
            ("dilemma.workers_campaign", "march", "marched"),
            ("dilemma.workers_campaign", "statement", "spoke"),
        ],
    },
];

/// v27 → v28 (A2, DEC-109, `docs/design/STAGED_ARCS.md` §1): storylines
/// fold into arcs, and the director times arcs and nothing else.
///
/// - **A running storyline becomes a run of its arc at the stage of the
///   same id:** the last stage it reached (its first, if it had reached
///   none), in the player's town, with its cast, the marks its answered
///   decisions set, those decisions as beats done, and a decision still
///   waiting on the player as the beat it now belongs to (`Decision::arc`).
///   A storyline whose stage the arc lacks, or whose template is unknown,
///   ends there (it faded). Ended storylines are not kept.
/// - **The narrative loses its storyline state:** `storylines`, `run_ids`,
///   `follow_ups`; a decision's `storyline`; the storyline facts of the
///   facts log; the fallback flag of a phase. A storyline's cooldown
///   becomes its arc's in the town; a `storyline` reason becomes an `arc`
///   reason; its use count carries to the arc.
/// - The log's `storyline_stage` events stay as they are (the engine still
///   reads the kind, and words them plainly).
///
/// What a v28 build adds beyond that needs no rewriting: a run's turn and
/// a chapter's lead are absent until set, and no treasury holds a vote.
fn storylines_to_arcs(mut value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    use serde_json::{Value, json};
    let state = value
        .get_mut("state")
        .and_then(|s| s.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    // The player's town: where a storyline ran (the narrative read no
    // other place), and so where its arc runs.
    let home = state
        .get("player")
        .and_then(|p| p.get("municipality"))
        .and_then(|m| m.as_u64());
    let at = home.map(|m| json!({ "type": "municipality", "id": m }));
    if state.get("narrative").is_none_or(|n| !n.is_object()) {
        // No narrative domain: nothing to fold.
        return Ok(value);
    }
    let mut arcs = match state.remove("arcs") {
        Some(Value::Object(a)) => a,
        Some(Value::Null) | None => serde_json::Map::new(),
        Some(_) => return Err(SaveError::Malformed),
    };
    let mut next_id = arcs
        .get("ids")
        .and_then(|i| i.get("next"))
        .and_then(|n| n.as_u64())
        .unwrap_or(0);
    let mut runs: Vec<Value> = match arcs.remove("runs") {
        Some(Value::Array(r)) => r,
        _ => Vec::new(),
    };
    let mut cooldowns: Vec<Value> = match arcs.remove("cooldowns") {
        Some(Value::Array(c)) => c,
        _ => Vec::new(),
    };
    let n = state
        .get_mut("narrative")
        .and_then(|n| n.as_object_mut())
        .ok_or(SaveError::Malformed)?;
    let storylines = match n.remove("storylines") {
        Some(Value::Array(s)) => s,
        _ => Vec::new(),
    };
    n.remove("run_ids");
    n.remove("follow_ups");
    // The decisions of each storyline run, before the field goes.
    let mut decisions = match n.remove("decisions") {
        Some(Value::Array(d)) => d,
        _ => Vec::new(),
    };
    let mut folded = 0u32;
    for run in &storylines {
        if !run.get("ended").is_none_or(|e| e.is_null()) {
            continue;
        }
        let (Some(id), Some(at)) = (run.get("id").and_then(|x| x.as_u64()), at.as_ref()) else {
            continue;
        };
        let template = run.get("template").and_then(|t| t.as_str()).unwrap_or("");
        let Some(f) = FOLDED.iter().find(|f| f.storyline == template) else {
            continue;
        };
        let reached = run.get("stages").and_then(|s| s.as_array());
        let last = reached.and_then(|s| s.last());
        let stage = match last.and_then(|x| x.get(0)).and_then(|x| x.as_str()) {
            None => f.stages[0],
            Some(sid) => match f.stages.iter().find(|x| **x == sid) {
                Some(x) => x,
                // An ending, or a stage the arc lacks: it faded.
                None => continue,
            },
        };
        let since = last
            .and_then(|x| x.get(1))
            .cloned()
            .or_else(|| run.get("since").cloned())
            .unwrap_or(Value::Null);
        let started = run.get("started").cloned().unwrap_or(since.clone());
        let cast: Vec<Value> = run
            .get("cast")
            .and_then(|c| c.as_array())
            .map(|c| {
                c.iter()
                    .filter_map(|e| {
                        let role = e.get("role")?.clone();
                        let a = e.get("actor")?;
                        let actor = match a.get("type")?.as_str()? {
                            "person" => json!({ "type": "person", "id": a.get("id")? }),
                            "neighborhood" => json!({
                                "type": "area",
                                "area": { "type": "hood", "id": a.get("id")? },
                            }),
                            _ => return None,
                        };
                        Some(json!({ "role": role, "actor": actor }))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let arc_id = next_id;
        next_id += 1;
        let mut marks: Vec<&str> = Vec::new();
        let mut done: Vec<Value> = Vec::new();
        let mut open: Vec<Value> = Vec::new();
        for d in decisions.iter_mut() {
            if d.get("storyline").and_then(|x| x.as_u64()) != Some(id) {
                continue;
            }
            let dilemma = d.get("template").and_then(|t| t.as_str()).unwrap_or("");
            let Some((_, at_stage, beat)) = f.decisions.iter().find(|x| x.0 == dilemma) else {
                continue;
            };
            let beat = json!({ "stage": at_stage, "beat": beat });
            let status = d.get("status").and_then(|x| x.as_str()).unwrap_or("");
            if status == "pending" {
                // Still waiting on the player: the run's open beat.
                if *at_stage == stage {
                    open.push(beat.clone());
                    if let Some(o) = d.as_object_mut() {
                        o.insert("arc".into(), json!({ "run": arc_id, "beat": beat }));
                    }
                }
                continue;
            }
            let res = d.get("resolution");
            let on = res
                .and_then(|r| r.get("date"))
                .cloned()
                .or_else(|| d.get("opened").cloned())
                .unwrap_or(Value::Null);
            done.push(json!({ "beat": beat, "on": on }));
            let option = res
                .and_then(|r| r.get("option"))
                .and_then(|x| x.as_str())
                .unwrap_or("");
            if matches!(status, "answered" | "defaulted")
                && let Some((_, _, mark)) = f.marks.iter().find(|x| x.0 == dilemma && x.1 == option)
                && !marks.contains(mark)
            {
                marks.push(*mark);
            }
        }
        marks.sort_unstable();
        let mut arc = json!({
            "id": arc_id,
            "template": f.arc,
            "at": at,
            "cast": cast,
            "stage": stage,
            "since": since,
            "started": started,
            "began": started,
            "clock": null,
            "heat": f.heat,
            "marks": marks,
            "done": done,
            "open": open,
            "base": [],
            "root": null,
        });
        if stage == f.turn
            && let Some(o) = arc.as_object_mut()
        {
            o.insert("turn".into(), json!({ "stage": stage, "on": since }));
        }
        runs.push(arc);
        folded += 1;
    }
    // A decision no longer names a storyline; its reason names the arc.
    for d in decisions.iter_mut() {
        let Some(o) = d.as_object_mut() else { continue };
        o.remove("storyline");
        if let Some(Value::Array(rs)) = o.get_mut("reasons") {
            for r in rs.iter_mut() {
                if r.get("type").and_then(|t| t.as_str()) != Some("storyline") {
                    continue;
                }
                let arc = r
                    .get("storyline")
                    .and_then(|s| s.as_str())
                    .and_then(|s| FOLDED.iter().find(|f| f.storyline == s))
                    .map(|f| f.arc);
                *r = match arc {
                    Some(a) => json!({ "type": "arc", "arc": a }),
                    None => json!({ "type": "opportunity" }),
                };
            }
        }
    }
    n.insert("decisions".into(), Value::Array(decisions));
    if let Some(Value::Array(facts)) = n.get_mut("facts") {
        facts.retain(|f| {
            !matches!(
                f.pointer("/kind/type").and_then(|t| t.as_str()),
                Some("storyline_stage" | "storyline_ended")
            )
        });
    }
    // A storyline's cooldown is its arc's in the town; its uses carry over.
    if let Some(Value::Array(cds)) = n.get_mut("cooldowns") {
        let mut keep = Vec::new();
        for c in cds.drain(..) {
            let key = c.get(0).and_then(|k| k.as_str()).unwrap_or("");
            match (FOLDED.iter().find(|f| f.storyline == key), at.as_ref()) {
                (Some(f), Some(at)) => cooldowns.push(json!({
                    "template": f.arc,
                    "at": at,
                    "until": c.get(1).cloned().unwrap_or(Value::Null),
                })),
                (Some(_), None) => {}
                (None, _) => keep.push(c),
            }
        }
        *cds = keep;
    }
    if let Some(Value::Array(uses)) = n.get_mut("uses") {
        for u in uses.iter_mut() {
            let arc = u
                .get(0)
                .and_then(|k| k.as_str())
                .and_then(|k| FOLDED.iter().find(|f| f.storyline == k))
                .map(|f| f.arc);
            if let (Some(a), Some(pair)) = (arc, u.as_array_mut()) {
                pair[0] = json!(a);
            }
        }
        uses.sort_by(|a, b| {
            let k = |x: &Value| x.get(0).and_then(|k| k.as_str()).unwrap_or("").to_string();
            k(a).cmp(&k(b))
        });
    }
    let mut strip = |p: &mut Value| {
        if let Some(o) = p.as_object_mut() {
            o.remove("climax_fallback");
        }
    };
    if let Some(p) = n.get_mut("phase") {
        strip(p);
    }
    if let Some(Value::Array(log)) = n.get_mut("phase_log") {
        log.iter_mut().for_each(&mut strip);
    }
    // The arcs domain is written only when it holds something.
    cooldowns.sort_by(|a, b| {
        let k = |x: &Value| {
            (
                x.get("template")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string(),
                x.pointer("/at/id").and_then(|i| i.as_u64()).unwrap_or(0),
            )
        };
        k(a).cmp(&k(b))
    });
    if folded > 0 || !arcs.is_empty() || !runs.is_empty() || !cooldowns.is_empty() {
        arcs.insert("runs".into(), Value::Array(runs));
        arcs.insert("ids".into(), json!({ "next": next_id }));
        arcs.insert("cooldowns".into(), Value::Array(cooldowns));
        arcs.entry("past").or_insert_with(|| json!([]));
        state.insert("arcs".into(), Value::Object(arcs));
    }
    Ok(value)
}

/// Structural fingerprint of a JSON value: object keys and value kinds, with
/// array contents collapsed to their first element's shape.
pub fn shape_fingerprint(value: &serde_json::Value) -> String {
    fn walk(v: &serde_json::Value, out: &mut String) {
        match v {
            serde_json::Value::Null => out.push('n'),
            serde_json::Value::Bool(_) => out.push('b'),
            serde_json::Value::Number(_) => out.push('#'),
            serde_json::Value::String(_) => out.push('s'),
            serde_json::Value::Array(a) => {
                out.push('[');
                if let Some(first) = a.first() {
                    walk(first, out);
                }
                out.push(']');
            }
            serde_json::Value::Object(o) => {
                out.push('{');
                for (k, val) in o {
                    out.push_str(k);
                    out.push(':');
                    walk(val, out);
                    out.push(',');
                }
                out.push('}');
            }
        }
    }
    let mut s = String::new();
    walk(value, &mut s);
    StableHash::of_bytes(s.as_bytes()).short()
}
