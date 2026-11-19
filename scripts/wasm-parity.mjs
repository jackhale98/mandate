// Native vs WASM determinism parity (DEC-041 rule 7, TEST-001).
// Usage: node scripts/wasm-parity.mjs <seed> <turns> <expected-native-hash>
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const nodeDir = process.env.MANDATE_WASM_NODE
  ? require("node:path").resolve(process.env.MANDATE_WASM_NODE)
  : require("node:path").join(__dirname, "../target/wasm-node");
const { WasmSession } = require(require("node:path").join(nodeDir, "mandate_wasm.js"));

const [seed = "1", turns = "50", expected] = process.argv.slice(2);
const s = new WasmSession();
let id = 0;
const send = (body) => {
  const res = JSON.parse(s.handle(JSON.stringify({ protocol_version: 1, request_id: ++id, body })));
  if (res.result.status !== "ok") throw new Error(JSON.stringify(res.result.error));
  return res.result.body;
};
send({ kind: "new_game", payload: { seed, size: "medium" } });
for (let i = 0; i < Number(turns); i++) send({ kind: "command", payload: { type: "end_turn" } });
const hash = s.stateHash();
console.log(hash);
if (expected && expected !== hash) {
  console.error(`PARITY FAILURE: native ${expected} != wasm ${hash}`);
  process.exit(1);
}
