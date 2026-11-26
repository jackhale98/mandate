/// <reference types="node" />
// Native ⇄ WASM parity for the career card (M6 note 23): the engine's Node
// WASM build steps away on the first turn and makes the same card, ratings
// and world code included, as the native build pinned in the fixture
// (crates/mandate-wasm/tests/career_card.rs writes it with
// MANDATE_WRITE_FIXTURES=1). The opening's rating plays forks of the world to
// its first election, so a float or a draw apart would show here. Timed: the
// card (the world rebuilt and the opening rated) and a race rated as it opens.
// Needs target/wasm-node (scripts/build-wasm.sh); skipped without it.
import { createRequire } from "node:module";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { CareerCardView } from "./protocol/CareerCardView";
import type { RequestBody } from "./protocol/RequestBody";
import type { StorytellerListView } from "./protocol/StorytellerListView";

const here = path.dirname(fileURLToPath(import.meta.url));
const dir = process.env.MANDATE_WASM_NODE ?? path.resolve(here, "../../../target/wasm-node");
const built = existsSync(path.join(dir, "mandate_wasm.js"));

describe.skipIf(!built)("the career card through the engine (WASM build)", () => {
  it("is the card the native build made", () => {
    const require = createRequire(import.meta.url);
    const { WasmSession } = require(path.join(dir, "mandate_wasm.js")) as { WasmSession: new () => { handle(s: string): string } };
    const s = new WasmSession();
    let id = 0;
    const send = (body: RequestBody) => {
      const res = JSON.parse(s.handle(JSON.stringify({ protocol_version: 1, request_id: ++id, body })));
      if (res.result.status !== "ok") throw new Error(JSON.stringify(res.result.error));
      return res.result.body.payload;
    };
    const tellers = send({ kind: "query", payload: { type: "storytellers" } }) as StorytellerListView;
    const teller = tellers.storytellers.find((t) => t.default)?.id ?? null;
    send({ kind: "new_game", payload: { seed: "3", size: "small", scenario: null, storyteller: teller, local_knowledge: false, recent_openings: [] } });
    send({ kind: "command", payload: { type: "step_away" } });
    const t0 = performance.now();
    const card = send({ kind: "query", payload: { type: "career_card" } }) as CareerCardView;
    console.log(`career card, Node WASM, seed 3 small (world rebuilt, opening rated): ${Math.round(performance.now() - t0)} ms`);
    const native = JSON.parse(readFileSync(path.resolve(here, "../components/fixtures/card/first_turn.json"), "utf8")) as CareerCardView;
    expect(card).toEqual(native);

    // A race rated as it opens: the campaign's first days.
    send({ kind: "new_game", payload: { seed: "3", size: "small", scenario: null, storyteller: teller, local_knowledge: false, recent_openings: [] } });
    const t1 = performance.now();
    send({ kind: "command", payload: { type: "advance_days", days: 245 } });
    const t2 = performance.now();
    send({ kind: "command", payload: { type: "advance_days", days: 7 } });
    const t3 = performance.now();
    console.log(`245 days advanced: ${Math.round(t2 - t1)} ms; then a week into the campaign, its race rated: ${Math.round(t3 - t2)} ms`);
  }, 120_000);
});
