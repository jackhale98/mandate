// Owns the single authoritative simulation instance (OVERVIEW §1). Requests
// are handled strictly in arrival order (DEC-043).
import init, { WasmSession, readSaveMeta } from "./pkg/mandate_wasm.js";
import wasmUrl from "./pkg/mandate_wasm_bg.wasm?url";
import type { FromWorker, ToWorker } from "./messages";

const post = (m: FromWorker, transfer: Transferable[] = []) => self.postMessage(m, { transfer });

let session: WasmSession | null = null;
const queue: ToWorker[] = [];

function handle(msg: ToWorker) {
  if (!session) return;
  try {
    switch (msg.type) {
      case "request":
        post({ type: "response", json: session.handle(msg.json) });
        break;
      case "export_save": {
        const bytes = session.exportSave(JSON.stringify(msg.request));
        post({ type: "save_bytes", id: msg.id, bytes }, [bytes.buffer]);
        break;
      }
      case "import_save":
        post({ type: "import_result", id: msg.id, json: session.importSave(msg.bytes) });
        break;
      case "read_save_meta":
        post({ type: "meta_result", id: msg.id, json: readSaveMeta(msg.bytes) });
        break;
    }
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    if (msg.type === "request") {
      post({ type: "fatal", message });
    } else {
      post({ type: "op_error", id: msg.id, message });
    }
  }
}

self.onmessage = (ev: MessageEvent<ToWorker>) => {
  if (session) handle(ev.data);
  else queue.push(ev.data);
};

init({ module_or_path: wasmUrl })
  .then(() => {
    session = new WasmSession();
    post({ type: "ready" });
    for (const m of queue.splice(0)) handle(m);
  })
  .catch((e: unknown) => post({ type: "fatal", message: e instanceof Error ? e.message : String(e) }));
