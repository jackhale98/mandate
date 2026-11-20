// The works kit (art round 8, DEC-099): the dress rules are pure and
// monotonic (a later decay stage never takes a mark away, a worn thing never
// mends), every kind renders in every phase and decay stage on its sample
// site, the kit keeps to the master palette and the round 7 colour budgets,
// and runs of fence and wall join on the street grid's own lines.
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { hash32 } from "../src/core/hash";
import { colourCount, offPalette } from "../src/core/lint";
import type { Indexed } from "../src/core/image";
import { buildPack, PACK_VERSION } from "../art/mandate/pack";
import { WORKS_ACTIVITY, WORKS_MONTHS, WORKS_PROGRESS } from "../art/mandate/samples";
import { inCut } from "../art/mandate/streets";
import { TERRAIN } from "../art/mandate/terrain";
import { BUILD_FROM, DECAY_FROM, decayStage, MARKS, WEAR, wearLevel, WORKS_KINDS, WORKS_PHASES, WORKS_RULES, worksLook, type WorksKind, type WorksLook, type WorksPhase } from "../art/mandate/works";
import { composeWorks, worksKit, type ComposedWorks } from "../art/mandate/worksCompose";
import { drawRun, pieceCentres, rectPieces, runCentres, LINE_STYLES, type LinePiece } from "../art/mandate/worksLines";
import { worksSamples } from "../art/mandate/worksScenes";

const pack = buildPack();
const pal = pack.palette;
const kit = worksKit(pack);
const samples = worksSamples();
const sampleOf = (k: WorksKind) => samples.find((s) => s.kind === k) as (typeof samples)[number];
const h = (img: Indexed) => hash32(img.w, img.h, ...Array.from({ length: img.h }, (_, y) => hash32(...img.px.subarray(y * img.w, (y + 1) * img.w))));
const hashWorks = (c: ComposedWorks) =>
  hash32(
    ...c.sprites.map((s) => hash32(s.x, s.y, s.base, ...s.frames.map(h), ...s.emissive.map((e) => (e ? h(e) : 0)))),
    ...c.decals.map((d) => hash32(d.x, d.y, h(d.img))),
    ...c.tints.map((t) => hash32(t.x, t.y, ...t.mask, ...t.table)),
    ...c.people.map((p) => hash32(p.x, p.y, JSON.stringify(p.look))),
  );

/** Every input worth checking: each phase at every build-stage threshold, each decay stage, each activity level. */
function inputs(_kind: WorksKind): [WorksPhase, number, number, { activity?: number; extraction?: number }][] {
  const out: [WorksPhase, number, number, { activity?: number; extraction?: number }][] = [];
  for (const p of [...BUILD_FROM, ...WORKS_PROGRESS]) out.push(["building", p, 0, {}]);
  for (const a of WORKS_ACTIVITY) for (const e of [0, 30, 60, 90]) out.push(["open", 100, 0, { activity: a, extraction: e }]);
  for (const p of [...BUILD_FROM, ...WORKS_PROGRESS]) for (const m of [0, ...DECAY_FROM, ...WORKS_MONTHS, 40]) for (const ph of ["stalled", "cleared"] as const) out.push([ph, p, m, {}]);
  return out;
}

describe("works rules", () => {
  it("name every kind and phase, and bump the pack", () => {
    assert.deepEqual([...WORKS_KINDS].sort(), ["bridge", "civic", "factory", "flood_wall", "housing", "levee", "mine", "road"]);
    assert.deepEqual([...WORKS_PHASES], ["building", "open", "stalled", "cleared"]);
    assert.equal(PACK_VERSION, "mandate-p0.9");
    for (const k of WORKS_KINDS) assert.equal(WORKS_RULES[k].build.length, 5, k);
  });
  it("are deterministic and plain data (the same inputs give the same parts; JSON round-trips)", () => {
    for (const k of WORKS_KINDS) {
      for (const [ph, p, m, o] of inputs(k)) {
        const a = worksLook(k, ph, p, m, 77, o);
        assert.deepEqual(worksLook(k, ph, p, m, 77, o), a);
        assert.deepEqual(JSON.parse(JSON.stringify(a)), a);
      }
    }
  });
  it("map progress and months to stages by fixed thresholds", () => {
    assert.equal(worksLook("civic", "building", 0, 0, 1).build, 0);
    assert.equal(worksLook("civic", "building", 50, 0, 1).build, 2);
    assert.equal(worksLook("civic", "building", 99, 0, 1).build, 3);
    assert.equal(worksLook("civic", "building", 100, 0, 1).build, 4);
    assert.deepEqual([0, 2, 3, 8, 9, 17, 18, 400].map(decayStage), [0, 0, 1, 1, 2, 2, 3, 3]);
    assert.equal(worksLook("civic", "stalled", 50, 18, 1).build, 2, "a stalled project keeps the stage it stopped at");
  });
  it("show the sponsor's plaque only on open works of kinds that carry one (and not when there is no sponsor)", () => {
    for (const k of WORKS_KINDS) {
      const open = worksLook(k, "open", 100, 0, 5);
      assert.equal(open.parts.some((p) => p.id === "plaque"), WORKS_RULES[k].plaque, k);
      assert.ok(!worksLook(k, "open", 100, 0, 5, { sponsor: false }).parts.some((p) => p.id === "plaque"), k);
      assert.ok(!worksLook(k, "stalled", 50, 9, 5).parts.some((p) => p.id === "plaque" || p.id === "ribbon"), k);
    }
    assert.ok(worksLook("civic", "building", 100, 0, 5).parts.some((p) => p.id === "ribbon"), "the ribbon waits at 100 %");
  });
  it("stop the site when it stalls: no workers, nothing lit, nothing turning", () => {
    for (const k of WORKS_KINDS) {
      for (const m of [0, 3, 9, 18]) {
        const l = worksLook(k, "stalled", 50, m, 9);
        assert.ok(!l.parts.some((p) => p.role === "people" || p.lit || p.id === "crane_load" || p.id === "smoke" || (p.frames && p.id !== "sheet")), `${k} ${m}`);
      }
      assert.ok(worksLook(k, "building", 50, 0, 9).parts.some((p) => p.role === "people"), `${k}: workers while building`);
    }
  });
});

describe("works decay is monotonic", () => {
  const marksOf = (l: WorksLook) => new Set(l.parts.filter((p) => p.mark).map((p) => `${p.id}#${p.n}`));
  const wornOf = (l: WorksLook) => new Map(l.parts.filter((p) => p.role !== "marks").map((p) => [`${p.role}#${p.n}#${(p.id.split("_")[0] as string).split(".")[0]}`, p.worn ?? 0]));
  it("a later stage never removes a decay mark, nor mends what has worn (every kind, progress and seed)", () => {
    for (const k of WORKS_KINDS) {
      for (const p of [0, 20, 45, 50, 80, 100]) {
        for (const seed of [1, 2, 3, 0xbeef, 0x5eed]) {
          for (const ph of ["stalled", "cleared"] as const) {
            let prev: WorksLook | null = null;
            for (let m = 0; m <= 30; m++) {
              const l = worksLook(k, ph, p, m, seed);
              if (prev) {
                for (const mk of marksOf(prev)) assert.ok(marksOf(l).has(mk), `${k} ${ph} ${p}% seed ${seed}: ${mk} gone at ${m} months`);
                const before = wornOf(prev);
                for (const [key, w] of wornOf(l)) if (before.has(key)) assert.ok(w >= (before.get(key) as number), `${k} ${p}% ${key} mended at ${m} months`);
                for (const part of l.parts) if (part.wear) assert.ok((part.decay ?? 0) >= (prev.decay ?? 0));
              }
              prev = l;
            }
          }
        }
      }
    }
  });
  it("marks grow with the stage, from none when just stopped", () => {
    for (const k of WORKS_KINDS) {
      for (const [, counts] of Object.entries(MARKS[k])) for (let s = 1; s < 4; s++) assert.ok((counts[s] as number) >= (counts[s - 1] as number), k);
      assert.equal(worksLook(k, "stalled", 50, 0, 3).parts.filter((p) => p.mark).length, 0, k);
      assert.ok(worksLook(k, "stalled", 50, 18, 3).parts.filter((p) => p.mark).length > worksLook(k, "stalled", 50, 3, 3).parts.filter((p) => p.mark).length, k);
    }
  });
  it("line pieces wear one way: each piece's level never falls as months pass", () => {
    for (const [name, rule] of Object.entries(WEAR)) {
      for (let n = 0; n < 40; n++) {
        let prev = 0;
        for (let d = 0; d <= 3; d++) {
          const w = wearLevel(rule, 0xabc, "perimeter", n, d);
          assert.ok(w >= prev && w < rule.chain.length, `${name} piece ${n}`);
          prev = w;
        }
      }
    }
  });
  it("composes monotonically too: every mark drawn at a stage is drawn, unchanged in place, at the next (ground marks)", () => {
    for (const s of samples) {
      for (let d = 1; d < 3; d++) {
        const a = composeWorks(worksLook(s.kind, "stalled", 50, DECAY_FROM[d] as number, s.seed), s.site, kit, s.seed);
        const b = composeWorks(worksLook(s.kind, "stalled", 50, DECAY_FROM[d + 1] as number, s.seed), s.site, kit, s.seed);
        const key = (x: { x: number; y: number; img: Indexed }) => `${x.x},${x.y},${h(x.img)}`;
        const later = new Set(b.decals.map(key));
        for (const dc of a.decals) assert.ok(later.has(key(dc)), `${s.kind} (${s.label}): a ground decal at stage ${d} is gone at ${d + 1}`);
      }
    }
  });
});

describe("works render", () => {
  it("every kind × phase × build and decay stage composes on its sample site, on the palette, deterministically", () => {
    const n = pal.colors.length - 1;
    for (const s of samples) {
      for (const [ph, p, m, o] of inputs(s.kind)) {
        const look = worksLook(s.kind, ph, p, m, s.seed, o);
        const c = composeWorks(look, s.site, kit, s.seed);
        assert.equal(hashWorks(composeWorks(look, s.site, kit, s.seed)), hashWorks(c));
        const drawn = c.sprites.length + c.decals.length + c.tints.length;
        if (!(ph === "cleared" && (s.kind === "flood_wall" || s.kind === "levee") && look.parts.length === 0)) assert.ok(drawn > 0, `${s.kind} ${ph} ${p}% ${m} months draws nothing`);
        for (const sp of c.sprites) for (const f of [...sp.frames, ...sp.emissive.filter((e): e is Indexed => !!e)]) assert.equal(offPalette(f, n), 0, sp.id);
        for (const d of c.decals) assert.equal(offPalette(d.img, n), 0);
        for (const t of c.tints) for (const v of t.table) assert.ok(v <= n);
      }
    }
  });
  it("shows the works' state on the map: the build changes what is drawn, open differs from stalled, decay adds marks", () => {
    for (const s of samples) {
      const at = (ph: WorksPhase, p: number, m: number) => hashWorks(composeWorks(worksLook(s.kind, ph, p, m, s.seed), s.site, kit, s.seed));
      assert.notEqual(at("building", 0, 0), at("building", 50, 0), s.label);
      assert.notEqual(at("building", 50, 0), at("stalled", 50, 1), s.label);
      assert.notEqual(at("stalled", 50, 1), at("stalled", 50, 18), s.label);
      assert.notEqual(at("open", 100, 0), at("stalled", 50, 18), s.label);
    }
  });
});

describe("works palette (the round 7 precedent)", () => {
  it("adds no colour to the master palette", () => {
    assert.equal(pal.colors.length - 1, 114);
  });
  it("keeps every part within 14 colours (furniture's budget) and its frames on the palette", () => {
    for (const p of Object.values(kit.parts)) for (const f of p.frames) {
      assert.equal(offPalette(f, pal.colors.length - 1), 0, p.id);
      assert.ok(colourCount(f) <= 14, `${p.id}: ${colourCount(f)} colours`);
    }
  });
  it("keeps each line piece within 10 colours and a whole composed site within 56 (a room's)", () => {
    for (const st of LINE_STYLES) for (const w of [0, 1, 2]) for (const sp of drawRun(pal, pack.lightness, st, runCentres(rectPieces(0, 0, 3, 2)), 3, () => w)) assert.ok(colourCount(sp.img) <= 10, `${st}: ${colourCount(sp.img)}`);
    for (const s of samples) {
      for (const [ph, p, m, o] of inputs(s.kind).filter((_, i) => i % 7 === 0)) {
        const c = composeWorks(worksLook(s.kind, ph, p, m, s.seed, o), s.site, kit, s.seed);
        const used = new Set<number>();
        for (const sp of c.sprites) if (!sp.id.startsWith("facade")) for (const f of sp.frames) for (const v of f.px) if (v) used.add(v);
        for (const d of c.decals) for (const v of d.img.px) if (v) used.add(v);
        assert.ok(used.size <= 56, `${s.kind} ${ph} ${p}% ${m}: ${used.size} colours`);
      }
    }
  });
});

describe("works runs join the street grid", () => {
  const SIDES = ["n", "e", "s", "w"] as const;
  const all: LinePiece[] = [];
  for (const a of SIDES) for (const b of [...SIDES, "c"] as const) if (a !== b) for (const sharp of [false, true]) all.push({ x: 3, y: 3, a, b, sharp });
  it("every piece starts and ends on a mid-side (or the centre), inside its own cell", () => {
    for (const p of all) {
      const cs = pieceCentres(p);
      assert.ok(cs.length >= 7, `${p.a}-${p.b}`);
      for (const c of cs) assert.ok(c.X >= 48 && c.X < 64 && c.Y >= 48 && c.Y < 64);
      const local = new Set(cs.map((c) => `${c.X - 48},${c.Y - 48}`));
      const end = (s: string) => (s === "n" ? "8,0" : s === "w" ? "0,8" : s === "e" ? "15,8" : s === "s" ? "8,15" : "8,8");
      // East and south mid-sides belong to the neighbour, so a piece reaches the pixel next to them.
      assert.ok(local.has(end(p.a)) || (p.a === "e" && local.has("15,9")) || (p.a === "s" && local.has("9,15")) || (p.a === "e" && local.has("15,7")) || (p.a === "s" && local.has("7,15")), `${p.a}-${p.b} start`);
    }
  });
  it("45° pieces lie along the 45° streets' own edge lines (round 5's corner cuts)", () => {
    const cut: Record<string, number> = { "n-w": 0, "w-n": 0, "n-e": 1, "e-n": 1, "e-s": 2, "s-e": 2, "s-w": 3, "w-s": 3 };
    for (const p of all) {
      if (p.sharp || p.b === "c" || cut[`${p.a}-${p.b}`] === undefined) continue;
      const corner = cut[`${p.a}-${p.b}`] as number;
      for (const c of pieceCentres(p)) {
        const x = c.X - 48;
        const y = c.Y - 48;
        const here = inCut(corner, x, y);
        const touches = [[1, 0], [-1, 0], [0, 1], [0, -1]].some(([dx, dy]) => inCut(corner, x + (dx as number), y + (dy as number)) !== here);
        assert.ok(touches, `${p.a}-${p.b} pixel ${x},${y} is off the street edge line`);
      }
    }
  });
  it("consecutive pieces meet without a gap (8-connected centres), and every style draws a run as one piece of art", () => {
    const run: LinePiece[] = [
      { x: 0, y: 2, a: "w", b: "e" },
      { x: 1, y: 2, a: "w", b: "n" },
      { x: 1, y: 1, a: "s", b: "e" },
      { x: 2, y: 1, a: "w", b: "e" },
      { x: 3, y: 1, a: "w", b: "s", sharp: true },
      { x: 3, y: 2, a: "n", b: "s" },
      { x: 3, y: 3, a: "n", b: "e" },
      { x: 4, y: 3, a: "w", b: "s" },
      { x: 4, y: 4, a: "n", b: "e" },
      { x: 5, y: 4, a: "w", b: "e" },
    ];
    for (let i = 1; i < run.length; i++) {
      const a = pieceCentres(run[i - 1] as LinePiece);
      const b = pieceCentres(run[i] as LinePiece);
      assert.ok(a.some((p) => b.some((q) => Math.abs(p.X - q.X) <= 1 && Math.abs(p.Y - q.Y) <= 1)), `pieces ${i - 1} and ${i} do not meet`);
    }
    for (const st of LINE_STYLES) {
      if (st === "heras" || st === "barrier" || st === "railing" || st === "piles") continue; // open patterns: holes by design
      const sprites = drawRun(pal, pack.lightness, st, runCentres(run), 1);
      // Composite the per-cell sprites and check the opaque pixels form one 4-connected shape (no seam at a cell edge).
      const x0 = Math.min(...sprites.map((s) => s.x));
      const y0 = Math.min(...sprites.map((s) => s.y));
      const W = Math.max(...sprites.map((s) => s.x + s.img.w)) - x0;
      const H = Math.max(...sprites.map((s) => s.y + s.img.h)) - y0;
      const m = new Uint8Array(W * H);
      for (const s of sprites) for (let y = 0; y < s.img.h; y++) for (let x = 0; x < s.img.w; x++) if (s.img.px[y * s.img.w + x]) m[(s.y - y0 + y) * W + s.x - x0 + x] = 1;
      const start = m.indexOf(1);
      const seen = new Uint8Array(W * H);
      const stack = [start];
      seen[start] = 1;
      let count = 0;
      while (stack.length) {
        const i = stack.pop() as number;
        count++;
        const x = i % W;
        for (const j of [i - 1, i + 1, i - W, i + W]) {
          if (j < 0 || j >= W * H || seen[j] || !m[j] || (j === i - 1 && x === 0) || (j === i + 1 && x === W - 1)) continue;
          seen[j] = 1;
          stack.push(j);
        }
      }
      assert.equal(count, m.reduce((a, v) => a + v, 0), `${st}: the run breaks at a cell edge`);
    }
  });
  it("the 45° flood wall and the road works' barrier on the round 5 town stay off the carriageway", () => {
    for (const s of samples.filter((x) => x.scene.streetAxis)) {
      const cs = s.site.pieces ? runCentres(s.site.pieces) : (s.site.closure ?? []);
      assert.ok(cs.length > 0);
      if (s.kind === "flood_wall") for (const c of cs) assert.notEqual(s.scene.terrain[(c.Y >> 4) * s.scene.width + (c.X >> 4)], TERRAIN.Road, `wall on the road at ${c.X},${c.Y}`);
      // The road works' barrier runs along the street's centre line: the lane boundary itself.
      if (s.kind === "road" && s.site.lane) for (const c of cs) assert.ok(Math.abs(s.site.lane.nx * c.X + s.site.lane.ny * c.Y - s.site.lane.d) <= 2, `barrier off the centre line at ${c.X},${c.Y}`);
    }
  });
});

describe("works goldens", () => {
  it("pin the kit's parts (a change bumps PACK_VERSION or WORKS_RULES_VERSION and repins)", () => {
    const now = hash32(...Object.entries(kit.parts).map(([id, p]) => hash32(id, ...p.frames.map(h), ...p.emissive.map((e) => (e ? h(e) : 0)))));
    const sites = hash32(...samples.map((s) => hashWorks(composeWorks(worksLook(s.kind, "stalled", 50, 18, s.seed), s.site, kit, s.seed))));
    if (process.env.UPDATE_GOLDENS) {
      console.log(JSON.stringify({ parts: now.toString(16), sites: sites.toString(16) }));
      return;
    }
    assert.deepEqual({ parts: now.toString(16), sites: sites.toString(16) }, PINNED);
  });
});

const PINNED = { parts: "0", sites: "0" };

void sampleOf;
