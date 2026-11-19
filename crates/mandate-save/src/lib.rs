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

use mandate_core::hash::StableHash;
use mandate_sim::State;
use serde::{Deserialize, Serialize};

pub const MAGIC: &[u8; 4] = b"MNDT";
pub const SAVE_FORMAT_VERSION: u16 = 20;
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
    let payload =
        serde_json::to_vec(&PayloadRef { state }).map_err(|e| SaveError::Data(e.to_string()))?;
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
    Ok((p.state, meta))
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
        // Neighbourhood concern levels (N1): the previous format to the
        // next. Renumber only this key if another bump merges first.
        (17, hood_concerns),
        // Reserved for R1 (offices from the constitution, v18 → v19), which
        // runs beside E1: a no-op here; R1's own step replaces it at merge.
        (18, reserved_for_r1),
        // The ground game's memory (E1, DEC-097 §1): the previous format to
        // the next. Renumber only this key if another bump merges first.
        (19, ground_game),
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

/// R1's format step (v18 → v19), reserved while R1 and E1 run side by side:
/// it changes nothing here, and R1's step takes its place at merge.
fn reserved_for_r1(value: serde_json::Value) -> Result<serde_json::Value, SaveError> {
    Ok(value)
}

/// Previous format → next (v19 → v20 when written; E1, DEC-097 §1): the
/// player's per-neighbourhood canvass memory. The step drops any a newer
/// build might have left, so the serde default applies: no doorsteps
/// remembered, every neighbourhood fresh.
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
