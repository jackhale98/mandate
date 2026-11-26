/// <reference types="node" />
// Native ⇄ WASM parity for the audition (DEC-101 §5, tasks A7 and A8): the engine's
// Node WASM build drafts the same openings, in the same order, as the
// native build pinned in the fixture (crates/mandate-wasm/tests/draft.rs
// writes it with MANDATE_WRITE_FIXTURES=1). The audition plays dozens of
// forked turns, so one float or draw apart would show here. Needs
// target/wasm-node (scripts/build-wasm.sh); skipped without it.
// MANDATE_WASM_NODE overrides its directory.
import { createRequire } from "node:module";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { DraftView } from "./protocol/DraftView";
import type { PocketView } from "./protocol/PocketView";
import type { RequestBody } from "./protocol/RequestBody";

const here = path.dirname(fileURLToPath(import.meta.url));
const dir = process.env.MANDATE_WASM_NODE ?? path.resolve(here, "../../../target/wasm-node");
const built = existsSync(path.join(dir, "mandate_wasm.js"));
const SEED = "3";

describe.skipIf(!built)("the draft through the engine (WASM build)", () => {
  it("drafts what the native build drafted", () => {
    const require = createRequire(import.meta.url);
    const { WasmSession } = require(path.join(dir, "mandate_wasm.js")) as { WasmSession: new () => { handle(s: string): string; stateHash(): string } };
    const s = new WasmSession();
    let id = 0;
    const send = (body: RequestBody) => {
      const res = JSON.parse(s.handle(JSON.stringify({ protocol_version: 1, request_id: ++id, body })));
      if (res.result.status !== "ok") throw new Error(JSON.stringify(res.result.error));
      return res.result.body.payload;
    };
    send({ kind: "new_game", payload: { seed: SEED, size: "small", scenario: null, storyteller: null, local_knowledge: false, recent_openings: [] } });
    const hash = s.stateHash();
    // One fork a message, as the draft screen asks for it; timed (STAGED_ARCS §6.8).
    const t0 = performance.now();
    let steps = 0;
    let draft: DraftView | null = null;
    while (draft === null) {
      steps += 1;
      const res = JSON.parse(s.handle(JSON.stringify({ protocol_version: 1, request_id: ++id, body: { kind: "query", payload: { type: "draft_step" } } })));
      if (res.result.body.kind === "draft") draft = res.result.body.payload as DraftView;
    }
    console.log(`audition, Node WASM, seed ${SEED} small: ${Math.round(performance.now() - t0)} ms in ${steps} messages`);
    const native = JSON.parse(readFileSync(path.resolve(here, `../components/fixtures/draft/draft_${SEED}.json`), "utf8")) as DraftView;
    expect(draft).toEqual(native);
    expect(draft.openings.length).toBeGreaterThan(0);
    // The audition only reads the game; a second query reads the draft made.
    expect(s.stateHash()).toBe(hash);
    expect(send({ kind: "query", payload: { type: "draft" } })).toEqual(native);
    // The opening chosen: play begins there with its beat on offer (A8's first-beat guarantee).
    send({ kind: "choose_opening", payload: { index: 0 } });
    const pocket = send({ kind: "query", payload: { type: "pocket" } }) as PocketView;
    expect(pocket.arcs.some((a) => a.open_beats > 0)).toBe(true);
  }, 60_000);
});
