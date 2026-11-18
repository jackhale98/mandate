// Main thread ⇄ worker envelope. The protocol payloads themselves are the
// generated Rust DTOs in ./protocol (DEC-043).
import type { SaveRequestDto } from "./protocol/SaveRequestDto";

export type ToWorker =
  | { type: "request"; json: string }
  | { type: "export_save"; id: number; request: SaveRequestDto }
  | { type: "import_save"; id: number; bytes: Uint8Array }
  | { type: "read_save_meta"; id: number; bytes: Uint8Array };

export type FromWorker =
  | { type: "ready" }
  | { type: "fatal"; message: string }
  | { type: "response"; json: string }
  | { type: "save_bytes"; id: number; bytes: Uint8Array }
  | { type: "import_result"; id: number; json: string }
  | { type: "meta_result"; id: number; json: string }
  | { type: "op_error"; id: number; message: string };
