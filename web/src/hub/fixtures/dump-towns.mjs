// Dump real towns from the engine as TownInput fixtures for the layout tests.
// Uses the Node build of the engine (target/wasm-node, from scripts/build-wasm.sh).
//   cd web && node src/hub/fixtures/dump-towns.mjs
// Writes towns.json here: the player's town for each seed, plus the largest
// town on one map (a capital), each with the seed it came from.
import { createRequire } from "node:module";
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { townInputFromMunicipality, townInputFromView } from "../townInput.ts";

const here = path.dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const { WasmSession } = require(path.resolve(here, "../../../../target/wasm-node/mandate_wasm.js"));

const SEEDS = ["4242", "2026", "1", "777", "99", "31337", "5", "12345", "8080", "60606"];

const towns = [];
for (const seed of SEEDS) {
  const s = new WasmSession();
  let id = 0;
  const send = (body) => {
    const res = JSON.parse(s.handle(JSON.stringify({ protocol_version: 1, request_id: ++id, body })));
    if (res.result.status !== "ok") throw new Error(JSON.stringify(res.result.error));
    return res.result.body.payload;
  };
  send({ kind: "new_game", payload: { seed, size: "medium", scenario: null } });
  towns.push({ seed, source: "town", input: townInputFromView(send({ kind: "query", payload: { type: "town" } })) });
  if (seed === SEEDS[0]) {
    const map = send({ kind: "query", payload: { type: "map" } });
    let best = null;
    for (const m of map.municipalities) {
      const v = send({ kind: "query", payload: { type: "municipality", id: m.id } });
      if (!best || v.places.length > best.places.length) best = v;
    }
    towns.push({ seed, source: "largest", input: townInputFromMunicipality(best) });
  }
  console.log(seed, towns[towns.length - 1].input.municipality);
}
writeFileSync(path.join(here, "towns.json"), JSON.stringify(towns, null, 1) + "\n");
console.log(`wrote ${towns.length} towns`);
