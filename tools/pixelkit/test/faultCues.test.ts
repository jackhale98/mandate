// Fault and repair cues (DEC-114 §3, task B4): every cue the content names
// (`content/base/faults.jsonc`) has a drawing; each fault kind's three cues
// (open, being repaired, new) differ in shape as well as colour, so they read
// apart at phone size and in the dark; the cues keep to the master palette,
// are outlined like every prop, stand on their cell, and only the new lamp
// lights up.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { colourCount, offPalette } from "../src/core/lint";
import type { Indexed } from "../src/core/image";
import { FALLBACK_CUES, FAULT_CUES } from "../art/mandate/faultCues";
import { REPO_ROOT } from "../art/mandate/interiorsContent";
import { jsoncToJson } from "../art/mandate/interiorsRecipes";
import { buildPack } from "../art/mandate/pack";

const pack = buildPack();
const content = JSON.parse(jsoncToJson(readFileSync(path.join(REPO_ROOT, "content/base/faults.jsonc"), "utf8"))) as {
  value: { faults: { id: string; cue: { open: string; repairing: string; new: string } }[] };
};
const kinds = content.value.faults;

/** The cue's silhouette: opaque pixels, in the cell's frame (the anchor at 0, 0). */
function silhouette(img: Indexed, ax: number, ay: number): Set<string> {
  const s = new Set<string>();
  for (let y = 0; y < img.h; y++) for (let x = 0; x < img.w; x++) if (img.px[y * img.w + x]) s.add(`${x - ax},${y - ay}`);
  return s;
}
const iou = (a: Set<string>, b: Set<string>) => {
  let n = 0;
  for (const k of a) if (b.has(k)) n++;
  return n / (a.size + b.size - n);
};

describe("fault cues", () => {
  it("draws every cue the content names, and the three stand-ins", () => {
    assert.ok(kinds.length > 0, "faults.jsonc has fault kinds");
    for (const k of kinds) for (const cue of [k.cue.open, k.cue.repairing, k.cue.new]) assert.ok(pack.props[`cue_${cue}`]?.length, `${k.id}: no drawing for ${cue}`);
    for (const cue of Object.values(FALLBACK_CUES)) assert.ok(FAULT_CUES.includes(cue));
    for (const cue of FAULT_CUES) assert.ok(pack.props[`cue_${cue}`]?.length, cue);
  });

  it("tells open, being repaired and new apart by shape, not only colour", () => {
    for (const k of kinds) {
      const [o, r, n] = [k.cue.open, k.cue.repairing, k.cue.new].map((c) => {
        const p = pack.props[`cue_${c}`]?.[0];
        assert.ok(p);
        return silhouette(p.albedo, p.ax, p.ay);
      });
      for (const [a, b, what] of [
        [o, r, "open / repairing"],
        [r, n, "repairing / new"],
        [o, n, "open / new"],
      ] as const) assert.ok(iou(a as Set<string>, b as Set<string>) < 0.85, `${k.id}: ${what} silhouettes too alike (IoU ${iou(a as Set<string>, b as Set<string>).toFixed(2)})`);
    }
  });

  it("keeps to the palette, is outlined, stands on its cell and fits the street", () => {
    const n = pack.palette.colors.length - 1;
    for (const cue of FAULT_CUES) {
      for (const [i, p] of (pack.props[`cue_${cue}`] ?? []).entries()) {
        assert.equal(offPalette(p.albedo, n), 0, `${cue} ${i}: off-palette pixels`);
        // A cue is a small composed scene (a lamp, a ladder, a van, a cone): the works kit's per-part budget (14) does not fit it; 20 with the outlines.
        assert.ok(colourCount(p.albedo) <= 20, `${cue} ${i}: ${colourCount(p.albedo)} colours`);
        // The cell (16 × 16 from the anchor) holds the foot: something is drawn on its bottom rows.
        const s = silhouette(p.albedo, p.ax, p.ay);
        assert.ok([...s].some((k) => Number(k.split(",")[1]) >= 10 && Number(k.split(",")[1]) < 17), `${cue} ${i}: nothing on its cell's ground`);
        // Readable at phone size without swamping the street: at most three cells wide, under two and a half tall.
        assert.ok(p.albedo.w <= 48 && p.albedo.h <= 40, `${cue} ${i}: ${p.albedo.w}×${p.albedo.h}`);
      }
    }
  });

  it("lights only the new lamp, and gives the vans a mirrored look", () => {
    for (const cue of FAULT_CUES) {
      const lit = (pack.props[`cue_${cue}`] ?? []).some((p) => p.emissive !== null);
      assert.equal(lit, cue === "lamp_new", cue);
    }
    for (const cue of ["lamp_van", "shelter_van", "swing_van"]) {
      const [a, b] = pack.props[`cue_${cue}`] ?? [];
      assert.ok(a && b, cue);
      assert.notDeepEqual(silhouette(a.albedo, a.ax, a.ay), silhouette(b.albedo, b.ax, b.ay));
    }
  });
});
