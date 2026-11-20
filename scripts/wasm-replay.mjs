// Replay a bug report through the WASM build (`mandate-cli replay --wasm`).
// Usage: node scripts/wasm-replay.mjs <report.json> [--until <turn>]
// Prints the engine's `ReplayOutcome` as JSON. MANDATE_WASM_NODE points at
// another node build (side-by-side checkouts), as for wasm-parity.mjs.
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
const require = createRequire(import.meta.url);
const path = require("node:path");
const here = path.dirname(new URL(import.meta.url).pathname);
const nodeDir = process.env.MANDATE_WASM_NODE
  ? path.resolve(process.env.MANDATE_WASM_NODE)
  : path.join(here, "../target/wasm-node");
const { WasmSession } = require(path.join(nodeDir, "mandate_wasm.js"));

const args = process.argv.slice(2);
const file = args.find((a, i) => !a.startsWith("--") && args[i - 1] !== "--until");
const u = args.indexOf("--until");
const until = u >= 0 ? Number(args[u + 1]) : undefined;
if (!file || (until !== undefined && !Number.isInteger(until))) {
  console.error("usage: node scripts/wasm-replay.mjs <report.json> [--until <turn>]");
  process.exit(2);
}
const s = new WasmSession();
process.stdout.write(s.replayReport(readFileSync(file, "utf8"), until) + "\n");
