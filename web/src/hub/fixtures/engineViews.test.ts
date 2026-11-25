/// <reference types="node" />
// Native = WASM parity for the town geography view (TOWN_GENERATION.md §15
// E2): the WASM build's view of a town hashes to the value pinned for the
// native build in crates/mandate-wasm/tests/town.rs. Needs the Node build of
// the engine (target/wasm-node, from scripts/build-wasm.sh; CI's web job has
// it); skipped without it. MANDATE_WASM_NODE overrides its directory.
import { createRequire } from "node:module";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { MapView } from "../../engine/protocol/MapView";
import type { TownGeographyView } from "../../engine/protocol/TownGeographyView";
import type { TownPlacesView } from "../../engine/protocol/TownPlacesView";
import type { TownStateView } from "../../engine/protocol/TownStateView";
import { geoFromEngine, stateFromEngine } from "../townGeoAdapter";
import { townInputFromView } from "../townInput";

/** Seed, size and FNV-1a of the player's town's geography view: the same pin as the native test. Re-pinned (d23444f1 -> 158ea778) when NPC votes began to follow the platform party line (W1 note 9), then (158ea778 -> baff2383) when rents became bounded by incomes and movers were shared fairly (audit fix 2), then (baff2383 -> c720a820) when the record began to decide votes (audit fix 1). */
const PINNED = { seed: "777", size: "small", hash: "fc8b7717" };

const here = path.dirname(fileURLToPath(import.meta.url));
const dir = process.env.MANDATE_WASM_NODE ?? path.resolve(here, "../../../../target/wasm-node");
const built = existsSync(path.join(dir, "mandate_wasm.js"));

/** FNV-1a (32-bit) over the UTF-8 bytes of a string. */
function fnv1a(s: string): string {
  const bytes = unescape(encodeURIComponent(s));
  let h = 0x811c9dc5;
  for (let i = 0; i < bytes.length; i++) {
    h ^= bytes.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h.toString(16).padStart(8, "0");
}

describe.skipIf(!built)("engine town views (WASM build)", () => {
  it("geography matches the native pin; the adapter maps both views", () => {
    const require = createRequire(import.meta.url);
    const { WasmSession } = require(path.join(dir, "mandate_wasm.js")) as { WasmSession: new () => { handle(s: string): string } };
    const s = new WasmSession();
    let id = 0;
    const send = (body: unknown) => {
      const res = JSON.parse(s.handle(JSON.stringify({ protocol_version: 1, request_id: ++id, body })));
      if (res.result.status !== "ok") throw new Error(JSON.stringify(res.result.error));
      return res.result.body.payload;
    };
    send({ kind: "new_game", payload: { seed: PINNED.seed, size: PINNED.size, scenario: null } });
    const town = send({ kind: "query", payload: { type: "town" } }) as TownPlacesView;
    const map = send({ kind: "query", payload: { type: "map" } }) as MapView;
    const mine = map.municipalities.find((m) => m.name === town.municipality);
    expect(mine).toBeDefined();
    const view = send({ kind: "query", payload: { type: "town_geography", id: mine?.id } }) as TownGeographyView;
    expect(fnv1a(JSON.stringify(view))).toBe(PINNED.hash);

    const input = townInputFromView(town);
    const geo = geoFromEngine(view, input);
    expect(geo.name).toBe(town.municipality);
    expect(geo.neighborhoods.map((h) => h.id)).toEqual(input.neighborhoods.map((h) => h.id));
    expect(geo.baseline_population).toBe(view.baseline_population);
    const st = send({ kind: "query", payload: { type: "town_state", id: mine?.id } }) as TownStateView;
    expect(st.detailed).toBe(false);
    const dress = stateFromEngine(st, input.home, input.location);
    expect(dress.hoods?.length).toBe(view.neighborhoods.length);
    // Since industries (DEC-110) a town's history can shut a works before play begins: the street is told
    // of exactly the places the engine has closed.
    const closed = st.places.filter((p) => p.status === "closed").map((p) => p.id);
    expect(dress.closedPlaces).toEqual(closed);
    expect(closed.length).toBeLessThan(st.places.length);
    expect(dress.housingPm).toBeGreaterThan(900);
  }, 120_000);
});
