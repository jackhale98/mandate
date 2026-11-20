//! Browser adapter (the only crate that touches wasm-bindgen; OVERVIEW §1).

pub mod session;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmSession {
    inner: session::Session,
}

#[wasm_bindgen]
impl WasmSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WasmSession, JsError> {
        console_error_panic_hook::set_once();
        session::Session::new()
            .map(|inner| WasmSession { inner })
            .map_err(|e| JsError::new(&e))
    }

    /// Register a progress callback `(stageKey: string, fraction: number)`.
    #[wasm_bindgen(js_name = setProgress)]
    pub fn set_progress(&mut self, f: js_sys::Function) {
        self.inner.set_progress(Box::new(move |stage, fraction| {
            let _ = f.call2(
                &JsValue::NULL,
                &JsValue::from_str(stage),
                &JsValue::from_f64(fraction),
            );
        }));
    }

    /// Handle one JSON protocol request; returns a JSON response.
    pub fn handle(&mut self, request_json: &str) -> String {
        self.inner.handle_json(request_json)
    }

    /// Encode the running game. `request_json` is a `SaveRequestDto`.
    #[wasm_bindgen(js_name = exportSave)]
    pub fn export_save(&self, request_json: &str) -> Result<Vec<u8>, JsError> {
        let req = serde_json::from_str(request_json).map_err(|e| JsError::new(&e.to_string()))?;
        self.inner.export_save(&req).map_err(|e| JsError::new(&e))
    }

    /// Load a save; returns the `OverviewView` JSON.
    #[wasm_bindgen(js_name = importSave)]
    pub fn import_save(&mut self, bytes: &[u8]) -> Result<String, JsError> {
        let view = self
            .inner
            .import_save(bytes)
            .map_err(|e| JsError::new(&e))?;
        serde_json::to_string(&view).map_err(|e| JsError::new(&e.to_string()))
    }

    /// The engine's part of a bug report (`ReportCore` JSON): versions, the
    /// state hash, and the run log since the start.
    #[wasm_bindgen(js_name = reportCore)]
    pub fn report_core(&self) -> Result<String, JsError> {
        self.inner.report_core().map_err(|e| JsError::new(&e))
    }

    /// Replay a bug report's run log on an engine of its own (developer
    /// tool: `mandate-cli replay --wasm`); returns the `ReplayOutcome` JSON.
    #[wasm_bindgen(js_name = replayReport)]
    pub fn replay_report(&self, report_json: &str, until: Option<u32>) -> Result<String, JsError> {
        self.inner
            .replay_report(report_json, until)
            .map_err(|e| JsError::new(&e))
    }

    /// Current state hash (determinism checks).
    #[wasm_bindgen(js_name = stateHash)]
    pub fn state_hash(&self) -> String {
        self.inner
            .engine()
            .map(|e| e.state_hash())
            .unwrap_or_default()
    }
}

/// Read save metadata (`SaveMetaDto` JSON) without loading the game.
#[wasm_bindgen(js_name = readSaveMeta)]
pub fn read_save_meta(bytes: &[u8]) -> Result<String, JsError> {
    let m = session::read_save_meta(bytes).map_err(|e| JsError::new(&e))?;
    serde_json::to_string(&m).map_err(|e| JsError::new(&e.to_string()))
}
