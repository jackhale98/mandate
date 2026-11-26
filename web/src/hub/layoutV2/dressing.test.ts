/// <reference types="node" />
// Street dressing (S7, DEC-114, task B4), over the real towns dumped from the
// engine (../fixtures/towns-v2.json) in eight run seeds: every spot of every
// fault kind (content/base/faults.jsonc, read here, never restated) has a
// cell on its neighbourhood's streets that never moves; a fault opened, being
// repaired and new shows three different cues on that cell; the cues move
// nothing else, block no one, and keep the town valid (the T2 checks).
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { FAULT_CUES } from "../../../../tools/pixelkit/art/mandate/faultCues";
import { townSeed, Terrain } from "../townLayout";
import { checkTownV2 } from "./check";
import { cueProp, DRAWN_CUES, dressFaults, faultCell, spotCells, SPOTS, type DressFault } from "./dressing";
import { GEO_FIXTURES, type GeoFixture } from "./fixtures";
import { dressHash, dressTown, layoutTownV2, type TownState } from "./index";
import { parseJsonc } from "./jsonc";

vi.setConfig({ testTimeout: 120_000 });

interface FaultKind {
  id: string;
  spot: { kind: string; slots: number };
  cue: { open: string; repairing: string; new: string };
}
const here = path.dirname(fileURLToPath(import.meta.url));
const KINDS = (parseJsonc(readFileSync(path.resolve(here, "../../../../content/base/faults.jsonc"), "utf8")) as { value: { faults: FaultKind[] } }).value.faults;
const STATUSES = [
  ["open", "open"],
  ["repairing", "repairing"],
  ["fixed", "new"],
] as const;

/** The first eight run seeds of the fixture towns, each with its towns. */
const SEEDS = [...new Set(GEO_FIXTURES.map((f) => f.seed))].slice(0, 8);
const towns = (seed: string) => GEO_FIXTURES.filter((f) => f.seed === seed);
const seedOf = (f: GeoFixture) => townSeed(f.seed, f.input.municipality);
const fault = (id: number, hood: number, k: FaultKind, slot: number, status: string, cue: string): DressFault => ({ id, hood, spot: { kind: k.spot.kind, slot, slots: k.spot.slots }, status, cue });

describe("street dressing: the registry", () => {
  it("knows every spot kind the content uses, and draws every cue it names", () => {
    expect(KINDS.length).toBeGreaterThan(0);
    for (const k of KINDS) {
      expect(SPOTS[k.spot.kind], `${k.id}: spot kind ${k.spot.kind}`).toBeDefined();
      for (const cue of Object.values(k.cue)) expect(DRAWN_CUES, `${k.id}: ${cue}`).toContain(cue);
    }
    // The registry's list is the art kit's.
    expect([...DRAWN_CUES].sort()).toEqual([...FAULT_CUES].sort());
  });

  it("maps a cue to its drawing, and a cue with none to its status's stand-in", () => {
    expect(cueProp("lamp_dark", "open")).toBe("cue_lamp_dark");
    expect(cueProp("not_drawn", "open")).toBe("cue_fault_open");
    expect(cueProp("not_drawn", "repairing")).toBe("cue_fault_repairing");
    expect(cueProp("not_drawn", "fixed")).toBe("cue_fault_new");
  });

  it("reads only the view's spot, status and cue", () => {
    const view = { id: 7, kind: "fault.x", asset: "a", name_key: "", what_key: "", args: {}, neighborhood: 3, spot: { kind: "road", slot: 1, slots: 5 }, status: "open", cue: "pothole", opened: { year: 1, month: 1, day: 1, iso: "" }, moved_up: false, next_in_queue: true, see: {} as never, here: false } as never;
    expect(dressFaults([view])).toEqual([{ id: 7, hood: 3, spot: { kind: "road", slot: 1, slots: 5 }, status: "open", cue: "pothole" }]);
    expect(dressFaults(undefined)).toEqual([]);
  });
});

describe.each(SEEDS)("street dressing: seed %s", (seed) => {
  for (const f of towns(seed)) {
    it(`${f.input.municipality} (${f.source}): every spot has a fixed cell; open, repairing and new show three cues there; nothing else moves`, () => {
      const { plan } = layoutTownV2(f.input, f.geo, seedOf(f));
      const base: TownState = { home: f.input.home, location: f.input.location };
      const plain = dressTown(plan, base);
      const W = plan.width;
      let id = 1;
      for (const d of plan.districts) {
        const claimed = new Map<number, string>();
        for (const k of KINDS) {
          const cells = spotCells(plan, d.index, k.spot.kind, k.spot.slots);
          // Every slot has room in a neighbourhood with streets; never two spots on one cell.
          expect(cells.length, `${d.name}: ${k.id}`).toBe(k.spot.slots);
          for (const c of cells) {
            expect(plan.district[c], `${d.name}: ${k.id} cell outside its neighbourhood`).toBe(d.index);
            expect(plain.solid[c], `${d.name}: ${k.id} on a solid cell`).toBe(0);
            expect(plain.terrain[c]).not.toBe(Terrain.Building);
            expect(claimed.get(c), `${d.name}: ${k.id} shares a cell`).toBeUndefined();
            claimed.set(c, k.id);
          }
          // A pure function of the plan: asked again (and with more slots), the same cells.
          expect(spotCells(plan, d.index, k.spot.kind, k.spot.slots)).toEqual(cells);
        }
      }
      // Each kind's first and last spot in the home neighbourhood, in each status.
      const home = plan.districts.find((d) => d.id === f.input.home) ?? plan.districts[0];
      if (!home) throw new Error("no district");
      for (const k of KINDS) {
        for (const slot of [0, k.spot.slots - 1]) {
          const cell = faultCell(plan, fault(0, home.id, k, slot, "open", k.cue.open));
          expect(cell).not.toBeNull();
          const seen = new Set<string>();
          for (const [status, key] of STATUSES) {
            const fl = fault(id++, home.id, k, slot, status, k.cue[key]);
            const map = dressTown(plan, { ...base, faults: [fl] });
            const cues = map.props.filter((p) => p.fault !== undefined);
            expect(cues).toHaveLength(1);
            const cue = cues[0];
            expect(cue?.y === undefined ? -1 : cue.y * W + cue.x, `${k.id} ${status}: on its spot`).toBe(cell);
            expect(cue?.kind).toBe(`cue_${k.cue[key]}`);
            expect(cue?.blocks).toBe(false);
            seen.add(cue?.kind ?? "");
            // Nothing else moves: the same buildings, ground and walking; the other props as planned (a lamp gives way to its cue).
            expect(map.terrain).toEqual(plain.terrain);
            expect(map.solid).toEqual(plain.solid);
            expect(map.buildings).toEqual(plain.buildings);
            const others = map.props.filter((p) => p.fault === undefined);
            const replaced = plain.props.filter((p) => !others.includes(p));
            expect(replaced.length).toBeLessThanOrEqual(1);
            if (replaced.length) expect(replaced[0]?.kind).toBe("lamp");
            if (replaced.length) expect((replaced[0]?.y ?? -1) * W + (replaced[0]?.x ?? -1)).toBe(cell);
            // Deterministic, and a different map hash only through the cue.
            expect(dressHash(dressTown(plan, { ...base, faults: [fl] }))).toBe(dressHash(map));
            expect(dressHash(map)).not.toBe(dressHash(plain));
          }
          expect(seen.size, `${k.id}: three different cues`).toBe(3);
        }
      }
      // Every fault at once (each kind's every slot in every neighbourhood): still a valid town (T2), each cue on its own cell.
      const all: DressFault[] = plan.districts.flatMap((d) => KINDS.flatMap((k) => Array.from({ length: k.spot.slots }, (_, slot) => fault(id++, d.id, k, slot, "repairing", k.cue.repairing))));
      const busy = dressTown(plan, { ...base, faults: all });
      const cueCells = busy.props.filter((p) => p.fault !== undefined).map((p) => p.y * W + p.x);
      expect(cueCells.length).toBe(all.length);
      expect(new Set(cueCells).size).toBe(cueCells.length);
      expect(checkTownV2(busy, plan, f.input)).toEqual([]);
    });
  }
});
