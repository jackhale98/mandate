import { EngineClient, EngineError, expectKind, type WorkerLike } from "./client";
import type { FromWorker, ToWorker } from "./messages";
import type { Request } from "./protocol/Request";

class FakeWorker implements WorkerLike {
  onmessage: ((ev: MessageEvent<FromWorker>) => void) | null = null;
  sent: ToWorker[] = [];
  postMessage(m: ToWorker) {
    this.sent.push(m);
  }
  terminate() {}
  emit(m: FromWorker) {
    this.onmessage?.({ data: m } as MessageEvent<FromWorker>);
  }
}

describe("EngineClient", () => {
  it("correlates responses by request id", async () => {
    const w = new FakeWorker();
    const c = new EngineClient(w);
    w.emit({ type: "ready" });
    await c.ready;
    const p1 = c.request({ kind: "query", payload: { type: "overview" } });
    const p2 = c.request({ kind: "query", payload: { type: "diagnostics" } });
    const [r1, r2] = w.sent.map((m) => (m.type === "request" ? (JSON.parse(m.json) as Request) : null));
    w.emit({ type: "response", json: JSON.stringify({ request_id: r2?.request_id, result: { status: "err", error: { code: "no_game", message: "none" } } }) });
    w.emit({ type: "response", json: JSON.stringify({ request_id: r1?.request_id, result: { status: "ok", body: { kind: "locale", payload: { lang: "en", ftl: "" } } } }) });
    expect(expectKind(await p1, "locale").lang).toBe("en");
    await expect(p2).rejects.toBeInstanceOf(EngineError);
  });

  it("rejects pending requests on a fatal worker error", async () => {
    const w = new FakeWorker();
    const c = new EngineClient(w);
    const p = c.request({ kind: "query", payload: { type: "overview" } });
    w.emit({ type: "fatal", message: "boom" });
    await expect(p).rejects.toThrow("boom");
    await expect(c.ready).rejects.toThrow("boom");
  });
});
