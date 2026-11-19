// Pinned sample buildings: the kit's contact sheet and its golden images.
import type { FacadeSpec } from "./buildings";

/** The town-centre family assembled several ways (terraces, shops, named places, civic landmarks, a boarded shop). */
export function sampleBuildings(): FacadeSpec[] {
  const base: FacadeSpec = { kind: "terrace", hood: "inner", w: 3, h: 3, doorX: 1, storeys: 2, visualSeed: 1, named: false, home: false, landmark: false, conditions: [] };
  const specs: FacadeSpec[] = [];
  for (let i = 0; i < 6; i++) specs.push({ ...base, w: 3 + (i % 2), visualSeed: 100 + i, doorX: i % 3 });
  for (let i = 0; i < 6; i++) specs.push({ ...base, kind: "shop", hood: i < 3 ? "inner" : "core", w: 3 + (i % 3), h: 3 + (i % 2), visualSeed: 200 + i, doorX: 1, storeys: i % 2 ? 3 : 2 });
  specs.push({ ...base, kind: "diner", w: 4, visualSeed: 300, named: true });
  specs.push({ ...base, kind: "news_office", hood: "core", w: 4, h: 4, visualSeed: 301, named: true, storeys: 3 });
  specs.push({ ...base, kind: "party_office", hood: "core", w: 4, h: 3, visualSeed: 309, named: true });
  specs.push({ ...base, kind: "home", hood: "suburb", w: 3, visualSeed: 304, home: true });
  specs.push({ ...base, kind: "clinic", hood: "core", w: 5, h: 4, doorX: 2, visualSeed: 305, named: true, storeys: 3 });
  specs.push({ ...base, kind: "shop", w: 4, visualSeed: 307, conditions: ["boarded"] });
  specs.push({ ...base, kind: "town_hall", hood: "core", w: 9, h: 5, doorX: 4, visualSeed: 302, named: true, landmark: true, storeys: 3 });
  specs.push({ ...base, kind: "council_chamber", hood: "core", w: 7, h: 5, doorX: 3, visualSeed: 303, named: true, landmark: true, storeys: 3 });
  specs.push({ ...base, kind: "warehouse", hood: "industrial", w: 6, h: 4, doorX: 2, visualSeed: 306 });
  return specs;
}
