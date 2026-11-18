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
