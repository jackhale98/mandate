// Pinned sample buildings: the kit's contact sheets, its golden images and the
// neighbour-similarity lint.
import type { EndKind, FacadeSpec } from "./buildings";

const base: FacadeSpec = { kind: "terrace", hood: "inner", w: 3, h: 3, doorX: 1, storeys: 2, visualSeed: 1, named: false, home: false, landmark: false, conditions: [], headroom: 32, ends: { left: "joined", right: "joined" } };

/** The town-centre family assembled several ways (terraces, shops, named places, civic landmarks, a boarded shop). */
export function sampleBuildings(): FacadeSpec[] {
  const specs: FacadeSpec[] = [];
  for (let i = 0; i < 6; i++) specs.push({ ...base, w: 3 + (i % 2), visualSeed: 100 + i, doorX: i % 3, ends: { left: i === 0 ? "street" : "joined", right: "joined" } });
  for (let i = 0; i < 6; i++) specs.push({ ...base, kind: "shop", hood: i < 3 ? "inner" : "core", w: 3 + (i % 3), h: 3 + (i % 2), visualSeed: 200 + i, doorX: 1, storeys: i % 2 ? 3 : 2, ends: { left: "joined", right: i === 5 ? "street" : "joined" } });
  specs.push({ ...base, kind: "diner", w: 4, visualSeed: 300, named: true });
  specs.push({ ...base, kind: "news_office", hood: "core", w: 4, h: 4, visualSeed: 301, named: true, storeys: 3 });
  specs.push({ ...base, kind: "party_office", hood: "core", w: 4, h: 3, visualSeed: 309, named: true });
  specs.push({ ...base, kind: "home", hood: "suburb", w: 3, visualSeed: 304, home: true, ends: { left: "gap", right: "gap" } });
  specs.push({ ...base, kind: "clinic", hood: "core", w: 5, h: 4, doorX: 2, visualSeed: 305, named: true, storeys: 3 });
  specs.push({ ...base, kind: "shop", w: 4, visualSeed: 307, conditions: ["boarded"] });
  specs.push({ ...base, kind: "town_hall", hood: "core", w: 9, h: 5, doorX: 4, visualSeed: 302, named: true, landmark: true, storeys: 3, ends: { left: "street", right: "street" } });
  specs.push({ ...base, kind: "council_chamber", hood: "core", w: 7, h: 5, doorX: 3, visualSeed: 303, named: true, landmark: true, storeys: 3, ends: { left: "street", right: "street" } });
  specs.push({ ...base, kind: "warehouse", hood: "industrial", w: 6, h: 4, doorX: 2, visualSeed: 306, ends: { left: "gap", right: "gap" } });
  return specs;
}

/** 24 buildings of different kinds and neighbourhoods (the variety sheet). */
export function varietyBuildings(): FacadeSpec[] {
  const g = (patch: Partial<FacadeSpec>, i: number): FacadeSpec => ({ ...base, visualSeed: 5000 + i * 37, ...patch });
  const open = { left: "gap" as EndKind, right: "gap" as EndKind };
  const list: Partial<FacadeSpec>[] = [
    { kind: "terrace", hood: "inner", w: 3 },
    { kind: "terrace", hood: "inner", w: 4, ends: { left: "street", right: "joined" } },
    { kind: "terrace", hood: "core", w: 3, h: 4 },
    { kind: "terrace", hood: "waterfront", w: 3 },
    { kind: "house", hood: "suburb", w: 4, ends: open },
    { kind: "house", hood: "suburb", w: 3, ends: open },
    { kind: "house", hood: "suburb", w: 5, doorX: 3, ends: open },
    { kind: "farmhouse", hood: "rural", w: 4, ends: open },
    { kind: "shop", hood: "inner", w: 3, ends: { left: "street", right: "joined" } },
    { kind: "shop", hood: "inner", w: 4 },
    { kind: "shop", hood: "core", w: 4, h: 4, ends: { left: "joined", right: "street" } },
    { kind: "shop", hood: "core", w: 5, h: 4 },
    { kind: "diner", hood: "inner", w: 4, named: true },
    { kind: "news_office", hood: "core", w: 4, h: 4, named: true },
    { kind: "party_office", hood: "core", w: 4, named: true },
    { kind: "union_hall", hood: "inner", w: 5, h: 4, doorX: 2, named: true },
    { kind: "market", hood: "core", w: 6, doorX: 3, named: true, ends: open },
    { kind: "clinic", hood: "core", w: 5, h: 4, doorX: 2, named: true },
    { kind: "school", hood: "suburb", w: 7, h: 4, doorX: 3, named: true, ends: open },
    { kind: "hotel", hood: "core", w: 5, h: 5, doorX: 2, named: true },
    { kind: "warehouse", hood: "industrial", w: 6, h: 4, doorX: 2, ends: open, visualSeed: 5000 + 20 * 37 + 1 },
    { kind: "shed", hood: "industrial", w: 3, ends: open },
    { kind: "boathouse", hood: "waterfront", w: 4, ends: open },
    { kind: "barn", hood: "rural", w: 4, h: 4, ends: open },
  ];
  return list.map((p, i) => g(p, i));
}

/**
 * Rows of neighbours as a town lays them out (terraces and shop rows, each
 * building knowing the one on its left): the neighbour-similarity lint's sample.
 */
export function sampleRows(): FacadeSpec[][] {
  const rows: FacadeSpec[][] = [];
  const kinds: [string, string][] = [
    ["terrace", "inner"],
    ["shop", "inner"],
    ["shop", "core"],
    ["terrace", "core"],
    ["house", "suburb"],
    ["terrace", "waterfront"],
    ["warehouse", "industrial"],
  ];
  for (let r = 0; r < 40; r++) {
    const [kind, hood] = kinds[r % kinds.length] as [string, string];
    const row: FacadeSpec[] = [];
    let left: FacadeSpec | null = null;
    for (let i = 0; i < 8; i++) {
      const w = 3 + ((r * 7 + i * 3) % 2);
      const spec: FacadeSpec = { ...base, kind, hood, w, h: hood === "core" ? 4 : 3, doorX: (r + i) % w, visualSeed: 90_000 + r * 101 + i * 13, ends: { left: i === 0 ? "street" : "joined", right: i === 7 ? "street" : "joined" }, left };
      row.push(spec);
      left = spec;
    }
    rows.push(row);
  }
  return rows;
}
