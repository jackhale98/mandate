/// <reference types="node" />
// Dumps the layout v2 fixtures (towns-v2.json): real towns from the engine
// with the geography layout v2 plans them from (the engine's
// `TownGeographyView`, through ../townGeoAdapter) and their state as play
// begins (`TownStateView`).
// It runs under vitest (so it can import the TypeScript adapter) and only
// when asked:
//   cd web && DUMP_GEO=1 npx vitest run src/hub/fixtures/dumpGeo.test.ts
// It needs the Node build of the engine (target/wasm-node, from
// scripts/build-wasm.sh); MANDATE_WASM_NODE overrides its directory.
// Towns are picked by geography (TOWN_GENERATION.md §13): the player's town
// for each seed (river, coast, hill and mining towns), the largest capital,
// a coastal capital, and the smallest town of a world.
import { createRequire } from "node:module";
import { writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { MapView } from "../../engine/protocol/MapView";
import type { MunicipalityView } from "../../engine/protocol/MunicipalityView";
import type { TownGeographyView } from "../../engine/protocol/TownGeographyView";
import type { TownPlacesView } from "../../engine/protocol/TownPlacesView";
import type { TownStateView } from "../../engine/protocol/TownStateView";
import { geoFromEngine } from "../townGeoAdapter";
import { townInputFromMunicipality, townInputFromView } from "../townInput";
import type { GeoFixture } from "../layoutV2/fixtures";

const run = process.env.DUMP_GEO === "1";

describe.skipIf(!run)("dump layout v2 fixtures", () => {
  it("writes towns-v2.json", () => {
    const here = path.dirname(fileURLToPath(import.meta.url));
    const dir = process.env.MANDATE_WASM_NODE ?? path.resolve(here, "../../../../target/wasm-node");
    const require = createRequire(import.meta.url);
    const { WasmSession } = require(path.join(dir, "mandate_wasm.js")) as { WasmSession: new () => { handle(s: string): string } };
    const SEEDS = ["4242", "2026", "1", "777", "99", "31337", "5", "12345", "8080", "60606"];
    const out: GeoFixture[] = [];
    for (const seed of SEEDS) {
      const s = new WasmSession();
      let id = 0;
      const send = (body: unknown) => {
        const res = JSON.parse(s.handle(JSON.stringify({ protocol_version: 1, request_id: ++id, body })));
        if (res.result.status !== "ok") throw new Error(JSON.stringify(res.result.error));
        return res.result.body.payload;
      };
      send({ kind: "new_game", payload: { seed, size: "medium", scenario: null } });
      const town = send({ kind: "query", payload: { type: "town" } }) as TownPlacesView;
      const map = send({ kind: "query", payload: { type: "map" } }) as MapView;
      const muni = (mid: number) => send({ kind: "query", payload: { type: "municipality", id: mid } }) as MunicipalityView;
      const add = (source: GeoFixture["source"], mv: MunicipalityView, input: GeoFixture["input"]) => {
        const view = send({ kind: "query", payload: { type: "town_geography", id: mv.id } }) as TownGeographyView;
        const st = send({ kind: "query", payload: { type: "town_state", id: mv.id } }) as TownStateView;
        out.push({
          seed,
          source,
          input,
          geo: geoFromEngine(view, input),
          state: {
            vacancy: st.housing.vacancy_pm / 1000,
            unemployment: st.unemployment_pm / 1000,
            hoods: st.neighborhoods.map((h) => ({ id: h.id, population: h.population })),
          },
        });
      };
      const mine = map.municipalities.find((m) => m.name === town.municipality);
      if (mine) add("town", muni(mine.id), townInputFromView(town));
      const bySize = [...map.municipalities].sort((a, b) => b.population - a.population || a.id - b.id);
      if (seed === "4242" || seed === "99") {
        const big = bySize[0];
        if (big) {
          const mv = muni(big.id);
          add("largest", mv, townInputFromMunicipality(mv));
        }
      }
      if (seed === "4242") {
        const small = bySize[bySize.length - 1];
        if (small) {
          const mv = muni(small.id);
          add("smallest", mv, townInputFromMunicipality(mv));
        }
      }
    }
    writeFileSync(path.join(here, "towns-v2.json"), JSON.stringify(out, null, 1) + "\n");
    expect(out.length).toBeGreaterThanOrEqual(12);
  }, 300_000);
});
