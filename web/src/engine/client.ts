// Typed client for the engine worker. The only path from UI to simulation.
import type { FromWorker, ToWorker } from "./messages";
import type { Request } from "./protocol/Request";
import type { RequestBody } from "./protocol/RequestBody";
import type { Response } from "./protocol/Response";
import type { ResponseBody } from "./protocol/ResponseBody";
import type { ProtocolError } from "./protocol/ProtocolError";
import type { SaveRequestDto } from "./protocol/SaveRequestDto";
import type { SaveMetaDto } from "./protocol/SaveMetaDto";
import type { OverviewView } from "./protocol/OverviewView";

export const PROTOCOL_VERSION = 1;

export class EngineError extends Error {
  readonly code: ProtocolError["code"];
  constructor(e: ProtocolError) {
    super(e.message);
    this.code = e.code;
  }
}

/** Minimal worker surface, so tests can supply a fake. */
export interface WorkerLike {
  postMessage(message: ToWorker, transfer?: Transferable[]): void;
  onmessage: ((ev: MessageEvent<FromWorker>) => void) | null;
  terminate(): void;
}

type Pending = { resolve: (v: never) => void; reject: (e: Error) => void };

export class EngineClient {
  private nextId = 1;
  private pending = new Map<number, Pending>();
  readonly ready: Promise<void>;
  private fatal: Error | null = null;
  private progressListeners = new Set<(stage: string, fraction: number) => void>();

  /** Subscribe to progress events; returns an unsubscribe function. */
  onProgress(f: (stage: string, fraction: number) => void): () => void {
    this.progressListeners.add(f);
    return () => this.progressListeners.delete(f);
  }

  constructor(private worker: WorkerLike) {
    let markReady: () => void = () => {};
    let markFailed: (e: Error) => void = () => {};
    this.ready = new Promise((res, rej) => {
      markReady = res;
      markFailed = rej;
    });
    worker.onmessage = (ev) => {
      const m = ev.data;
      switch (m.type) {
        case "ready":
          markReady();
          break;
        case "progress":
          for (const f of this.progressListeners) f(m.stage, m.fraction);
          break;
        case "fatal":
          this.fatal = new Error(m.message);
          markFailed(this.fatal);
          for (const p of this.pending.values()) p.reject(this.fatal);
          this.pending.clear();
          break;
        case "response": {
          const res = JSON.parse(m.json) as Response;
          this.settle(res.request_id, () =>
            res.result.status === "ok" ? { ok: res.result.body } : { err: new EngineError(res.result.error) },
          );
          break;
        }
        case "save_bytes":
          this.settle(m.id, () => ({ ok: m.bytes }));
          break;
        case "import_result":
          this.settle(m.id, () => ({ ok: JSON.parse(m.json) as OverviewView }));
          break;
        case "meta_result":
          this.settle(m.id, () => ({ ok: JSON.parse(m.json) as SaveMetaDto }));
          break;
        case "report_core_result":
          this.settle(m.id, () => ({ ok: m.json }));
          break;
        case "op_error":
          this.settle(m.id, () => ({ err: new Error(m.message) }));
          break;
      }
    };
  }

  private settle(id: number, f: () => { ok: unknown } | { err: Error }) {
    const p = this.pending.get(id);
    if (!p) return;
    this.pending.delete(id);
    const r = f();
    if ("ok" in r) p.resolve(r.ok as never);
    else p.reject(r.err);
  }

  private track<T>(id: number): Promise<T> {
    if (this.fatal) return Promise.reject(this.fatal);
    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, { resolve: resolve as (v: never) => void, reject });
    });
  }

  request(body: RequestBody): Promise<ResponseBody> {
    const id = this.nextId++;
    const req: Request = { protocol_version: PROTOCOL_VERSION, request_id: id, body };
    const p = this.track<ResponseBody>(id);
    this.worker.postMessage({ type: "request", json: JSON.stringify(req) });
    return p;
  }

  exportSave(request: SaveRequestDto): Promise<Uint8Array> {
    const id = this.nextId++;
    const p = this.track<Uint8Array>(id);
    this.worker.postMessage({ type: "export_save", id, request });
    return p;
  }

  importSave(bytes: Uint8Array): Promise<OverviewView> {
    const id = this.nextId++;
    const p = this.track<OverviewView>(id);
    this.worker.postMessage({ type: "import_save", id, bytes });
    return p;
  }

  readSaveMeta(bytes: Uint8Array): Promise<SaveMetaDto> {
    const id = this.nextId++;
    const p = this.track<SaveMetaDto>(id);
    this.worker.postMessage({ type: "read_save_meta", id, bytes });
    return p;
  }

  /** The engine's part of a bug report: `ReportCore` as JSON text (the report embeds it as it is). */
  reportCore(): Promise<string> {
    const id = this.nextId++;
    const p = this.track<string>(id);
    this.worker.postMessage({ type: "report_core", id });
    return p;
  }

  /** The engine has failed (a panic, or it never started): nothing more can be asked of it. */
  get failed(): boolean {
    return this.fatal !== null;
  }

  terminate() {
    this.worker.terminate();
  }
}

/** A second engine in a worker of its own (a bug report after the app's engine failed); terminate it when done. */
export function freshEngine(): EngineClient {
  const w = new Worker(new URL("./engine.worker.ts", import.meta.url), { type: "module" });
  return new EngineClient(w as unknown as WorkerLike);
}

let shared: EngineClient | null = null;

/** The app-wide engine client (one worker, one simulation). */
export function engine(): EngineClient {
  if (!shared) {
    const w = new Worker(new URL("./engine.worker.ts", import.meta.url), { type: "module" });
    shared = new EngineClient(w as unknown as WorkerLike);
  }
  return shared;
}

/** Payload type of the response variant with the given kind. */
export type PayloadOf<K extends ResponseBody["kind"]> =
  ResponseBody extends infer R ? (R extends { kind: K; payload: infer P } ? P : never) : never;

/** Narrow a response body to the expected kind or throw. */
export function expectKind<K extends ResponseBody["kind"]>(body: ResponseBody, kind: K): PayloadOf<K> {
  if (body.kind !== kind) throw new Error(`expected ${kind} response, got ${body.kind}`);
  return body.payload as unknown as PayloadOf<K>;
}
