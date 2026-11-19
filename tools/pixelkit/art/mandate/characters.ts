// Characters: a look (the appearance genome stand-in for P0) composed from
// the parts in ./characterParts into 16×24 frames, on the device (D10).
// Picks use rendezvous hashing (§3.5 rule 6) from the fixed hash, so the
// same seed always gives the same person, and adding a hair style later
// changes only the people it now wins for. Body, height, skin and hair are
// independent of clothing (§3.3).
import { stack, stretchRows, type LayerSpec } from "../../src/core/compose";
import { hash32, rendezvous } from "../../src/core/hash";
import { blank, crop, grid, grids, mirrorX, outline, remap, type Indexed } from "../../src/core/image";
import { ramp, type Palette } from "../../src/core/palette";
import { CLOTH_NAMES, HAIR_NAMES, LINES } from "./palette";
import {
  APRON,
  ARM_FRONT,
  ARM_SIDE,
  BEARD,
  HAIR_STYLES,
  HEAD,
  LEGEND,
  LEGS,
  LEGS_BROAD_DOWN,
  SKIRT,
  SLOT,
  SLOT_COUNT,
  TOP_DETAIL,
  TORSO,
  WALK_ARMS,
  type ArmPose,
  type Build,
  type FacingKey,
  type HairStyle,
  type Top,
} from "./characterParts";

export type Bottom = "trousers" | "shorts" | "skirt";

/** One person's look. In P1 the engine stores the semantic choices (§3.5); here they come from a seed. */
export interface Look {
  build: Build;
  /** 0 short, 1 middle, 2 tall. */
  height: 0 | 1 | 2;
  /** Skin ramp, 0 (MST 1) .. 5 (MST 10). */
  skin: number;
  hair: HairStyle | "bald";
  hairColour: (typeof HAIR_NAMES)[number];
  beard: boolean;
  top: Top;
  topRamp: string;
  bottom: Bottom;
  bottomRamp: string;
  accentRamp: string;
  shoeRamp: string;
}

export const FACINGS: readonly FacingKey[] = ["down", "up", "side"];
/** Frames per facing: idle (2), then the walk (4). */
export const FRAMES = ["idle0", "idle1", "walk0", "walk1", "walk2", "walk3"] as const;
export const FRAME_W = 16;
export const FRAME_H = 24;
/** The feet: anchor x (the figure's centre column) and y (the row under the soles). */
export const ANCHOR = { x: 7, y: 23 } as const;

// ---------------------------------------------------------------------------
// Picks

type Choice<T> = { id: string; weight?: number; value: T };
const opts = <T extends string | number | boolean>(xs: readonly (readonly [T, number])[]): Choice<T>[] => xs.map(([value, weight]) => ({ id: String(value), weight, value }));
const pick = <T,>(seed: number, salt: string, xs: readonly Choice<T>[]): T => rendezvous(seed, salt, xs).value;

const OUTFITS: Record<Top, { top: readonly string[]; bottom: readonly (readonly [Bottom, number])[]; bottomRamp: readonly string[] | "same"; accent: readonly string[] }> = {
  tee: { top: ["red", "mustard", "green", "plum", "sky", "teal", "shirt", "navy"], bottom: [["trousers", 5], ["shorts", 2], ["skirt", 2]], bottomRamp: ["denim", "charcoal", "navy", "plum", "brown"], accent: ["red"] },
  suit: { top: ["navy", "charcoal", "brown", "denim"], bottom: [["trousers", 4], ["skirt", 1]], bottomRamp: "same", accent: ["red", "mustard", "sky", "green"] },
  scrubs: { top: ["teal", "sky"], bottom: [["trousers", 1]], bottomRamp: "same", accent: ["sky"] },
  apron: { top: ["shirt", "sky", "mustard", "red"], bottom: [["trousers", 3], ["skirt", 1]], bottomRamp: ["denim", "charcoal", "navy"], accent: ["green", "navy", "red", "brown"] },
};

/** Ramp names for an outfit colour (cloth, or a material ramp a garment shares). */
const clothRamp = (n: string) => (n === "brown" ? "wood" : (CLOTH_NAMES as readonly string[]).includes(n) ? `cloth_${n}` : n);

/** A townsperson's look from a seed (ambient people, P0). */
export function lookFromSeed(seed: number): Look {
  const s = hash32(seed, "look");
  const top = pick(s, "top", opts<Top>([["tee", 5], ["suit", 2], ["scrubs", 1], ["apron", 1]]));
  const o = OUTFITS[top];
  const topRamp = clothRamp(pick(s, "top-colour", opts(o.top.map((c) => [c, 1] as const))));
  const bottom = pick(s, "bottom", opts(o.bottom));
  const bottomRamp = o.bottomRamp === "same" ? topRamp : clothRamp(pick(s, "bottom-colour", opts(o.bottomRamp.map((c) => [c, 1] as const))));
  return {
    build: pick(s, "build", opts<Build>([["slim", 3], ["average", 4], ["broad", 3]])),
    height: pick(s, "height", opts<number>([[0, 1], [1, 1], [2, 1]])) as 0 | 1 | 2,
    skin: pick(s, "skin", opts<number>([0, 1, 2, 3, 4, 5].map((k) => [k, 1] as const))),
    hair: pick(s, "hair", opts<HairStyle | "bald">([["crop", 3], ["long", 3], ["bun", 2], ["coily", 3], ["bob", 2], ["buzz", 2], ["bald", 1]])),
    hairColour: pick(s, "hair-colour", opts<(typeof HAIR_NAMES)[number]>([["black", 3], ["dark_brown", 3], ["brown", 2], ["auburn", 1], ["blonde", 2], ["grey", 2]])),
    beard: hash32(s, "beard") % 100 < 16,
    top,
    topRamp,
    bottom,
    bottomRamp,
    accentRamp: clothRamp(pick(s, "accent", opts(o.accent.map((c) => [c, 1] as const)))),
    shoeRamp: pick(s, "shoes", opts<string>([["cloth_charcoal", 3], ["wood", 2], ["chalk", 1]])),
  };
}

/** The player: a navy suit with the logo flag's orange as the tie, so they read at a glance. */
export const PLAYER_LOOK: Look = {
  build: "average",
  height: 1,
  skin: 2,
  hair: "crop",
  hairColour: "dark_brown",
  beard: false,
  top: "suit",
  topRamp: "cloth_navy",
  bottom: "trousers",
  bottomRamp: "cloth_navy",
  accentRamp: "flag",
  shoeRamp: "wood",
};

/** The six features the clone-rate lint compares (§3.3). */
export function visibleFeatures(l: Look): string[] {
  return [l.build, `skin${Math.floor(l.skin / 2)}`, l.hair, l.hairColour, l.topRamp, `${l.top}/${l.bottom}`];
}

// ---------------------------------------------------------------------------
// Parsed parts (once)

const P = (t: string): Indexed => grid(t, LEGEND);
let parsed: ReturnType<typeof parseParts> | null = null;
function parseParts() {
  const head = { down: P(HEAD.down), up: P(HEAD.up), side: P(HEAD.side) };
  const hair = Object.fromEntries(Object.entries(HAIR_STYLES).map(([k, v]) => [k, { down: P(v.down), up: P(v.up), side: P(v.side) }])) as Record<HairStyle, Record<FacingKey, Indexed>>;
  const torso = Object.fromEntries(Object.entries(TORSO).map(([k, v]) => [k, { down: P(v.down), side: P(v.side), left: v.left, right: v.right }])) as Record<Build, { down: Indexed; side: Indexed; left: number; right: number }>;
  const armFront = Object.fromEntries(Object.entries(ARM_FRONT).map(([k, v]) => [k, P(v)])) as Record<ArmPose, Indexed>;
  const armSide = Object.fromEntries(Object.entries(ARM_SIDE).map(([k, v]) => [k, P(v)])) as Record<ArmPose, Indexed>;
  const legs = { down: LEGS.down.map(P), side: LEGS.side.map(P), broad: LEGS_BROAD_DOWN.map(P) };
  const detail = Object.fromEntries(Object.entries(TOP_DETAIL).map(([k, v]) => [k, { down: v.down ? P(v.down) : null, up: v.up ? P(v.up) : null, side: v.side ? P(v.side) : null }])) as Record<Top, Record<FacingKey, Indexed | null>>;
  const apronFull = { down: P(APRON.down), up: P(APRON.up), side: P(APRON.side) };
  const apron = Object.fromEntries(Object.entries(apronFull).map(([k, v]) => [k, { bib: crop(v, 0, 0, v.w, 6), skirt: crop(v, 0, 6, v.w, Math.max(0, v.h - 6)) }])) as Record<FacingKey, { bib: Indexed; skirt: Indexed }>;
  return { head, hair, torso, armFront, armSide, legs, detail, apron, beard: { down: P(BEARD.down), side: P(BEARD.side) }, skirt: { down: P(SKIRT.down), side: P(SKIRT.side) } };
}
export const parts = () => (parsed ??= parseParts());

/** Every part as (name, image of slot codes), for the parts contact sheet and the lints. */
export function partList(): { name: string; img: Indexed }[] {
  const p = parts();
  const out: { name: string; img: Indexed }[] = [];
  for (const f of ["down", "up", "side"] as const) out.push({ name: `head ${f}`, img: p.head[f] });
  for (const [k, v] of Object.entries(p.hair)) for (const f of ["down", "up", "side"] as const) out.push({ name: `hair ${k} ${f}`, img: v[f] });
  out.push({ name: "beard down", img: p.beard.down }, { name: "beard side", img: p.beard.side });
  for (const [k, v] of Object.entries(p.torso)) out.push({ name: `torso ${k} down`, img: v.down }, { name: `torso ${k} side`, img: v.side });
  for (const [k, v] of Object.entries(p.armFront)) out.push({ name: `arm ${k} front`, img: v });
  for (const [k, v] of Object.entries(p.armSide)) out.push({ name: `arm ${k} side`, img: v });
  p.legs.down.forEach((img, i) => out.push({ name: `legs down ${i}`, img }));
  p.legs.side.forEach((img, i) => out.push({ name: `legs side ${i}`, img }));
  p.legs.broad.forEach((img, i) => out.push({ name: `legs broad ${i}`, img }));
  for (const [k, v] of Object.entries(p.detail)) for (const f of ["down", "up", "side"] as const) if (v[f]) out.push({ name: `top ${k} ${f}`, img: v[f] as Indexed });
  for (const f of ["down", "up", "side"] as const) out.push({ name: `apron ${f}`, img: grid(APRON[f], LEGEND) });
  out.push({ name: "skirt down", img: p.skirt.down }, { name: "skirt side", img: p.skirt.side });
  return out;
}

// ---------------------------------------------------------------------------
// Slots to palette

function step(p: Palette, rampName: string, k: -1 | 0 | 1): number {
  const r = ramp(p, rampName);
  const i = Math.min(r.steps.length - 1, Math.max(0, r.anchorStep + k));
  return r.steps[i] as number;
}

/** The slot → palette table for a look. */
export function slotTable(l: Look, p: Palette): Uint8Array {
  const t = new Uint8Array(SLOT_COUNT);
  const skin = `skin${l.skin + 1}`;
  const hair = `hair_${l.hairColour}`;
  t[SLOT.skinS] = step(p, skin, -1);
  t[SLOT.skin] = step(p, skin, 0);
  t[SLOT.skinL] = step(p, skin, 1);
  t[SLOT.hairS] = step(p, hair, -1);
  t[SLOT.hair] = step(p, hair, 0);
  t[SLOT.hairL] = step(p, hair, 1);
  t[SLOT.topS] = step(p, l.topRamp, -1);
  t[SLOT.top] = step(p, l.topRamp, 0);
  t[SLOT.topL] = step(p, l.topRamp, 1);
  t[SLOT.botS] = step(p, l.bottomRamp, -1);
  t[SLOT.bot] = step(p, l.bottomRamp, 0);
  t[SLOT.shoeS] = step(p, l.shoeRamp, -1);
  t[SLOT.shoe] = step(p, l.shoeRamp, 0);
  t[SLOT.accS] = step(p, l.accentRamp, -1);
  t[SLOT.acc] = step(p, l.accentRamp, 0);
  t[SLOT.accL] = step(p, l.accentRamp, 1);
  // Eyes: dark, except on the darkest skin, where a dark eye would vanish (a light iris reads instead).
  t[SLOT.ink] = l.skin >= 5 ? step(p, "chalk", -1) : p.colors.indexOf(LINES.warm);
  t[SLOT.white] = step(p, "chalk", 0);
  const long = l.top === "suit" || l.top === "apron";
  t[SLOT.armR] = long ? (t[SLOT.top] as number) : (t[SLOT.skin] as number);
  t[SLOT.armRS] = long ? (t[SLOT.topS] as number) : (t[SLOT.skinS] as number);
  const legsCovered = l.bottom === "trousers";
  t[SLOT.legU] = l.bottom === "skirt" ? (t[SLOT.skin] as number) : (t[SLOT.bot] as number);
  t[SLOT.legUS] = l.bottom === "skirt" ? (t[SLOT.skinS] as number) : (t[SLOT.botS] as number);
  t[SLOT.legL] = legsCovered ? (t[SLOT.bot] as number) : (t[SLOT.skin] as number);
  t[SLOT.legLS] = legsCovered ? (t[SLOT.botS] as number) : (t[SLOT.skinS] as number);
  return t;
}

/** Slots for the far or right arm: everything one step into shade. */
const SHADE = (() => {
  const m = new Uint8Array(SLOT_COUNT);
  for (let i = 0; i < SLOT_COUNT; i++) m[i] = i;
  m[SLOT.top] = SLOT.topS;
  m[SLOT.topL] = SLOT.topS;
  m[SLOT.armR] = SLOT.armRS;
  m[SLOT.skin] = SLOT.skinS;
  return m;
})();

// ---------------------------------------------------------------------------
// Composition

/** The slot image of one frame (before recolouring and outline), 15×24, side views facing right. */
export function composeSlots(l: Look, facing: FacingKey, frame: number): Indexed {
  const p = parts();
  const legIdx = frame <= 1 ? 0 : frame - 1;
  const side = facing === "side";
  const legs = stretchRows((side ? p.legs.side : l.build === "broad" ? p.legs.broad : p.legs.down)[legIdx] as Indexed, 2, l.height - 1);
  const legsTop = 22 - (legs.h - 1);
  const torsoTop = legsTop - 6;
  // The chin sits on the shoulders (one row of overlap), which leaves room above the tallest hair.
  const headY = torsoTop - 10 + (frame === 1 ? 1 : 0);
  const torso = p.torso[l.build];
  const [nearPose, farPose] = WALK_ARMS[legIdx] as readonly [ArmPose, ArmPose];
  const layers: LayerSpec[] = [];
  if (side) layers.push({ img: p.armSide[farPose], x: 0, y: torsoTop, map: SHADE, under: true });
  layers.push({ img: legs, x: 0, y: legsTop });
  if (l.bottom === "skirt") layers.push({ img: side ? p.skirt.side : p.skirt.down, x: 0, y: legsTop });
  layers.push({ img: side ? torso.side : torso.down, x: 0, y: torsoTop });
  const detail = p.detail[l.top][facing];
  if (detail) layers.push({ img: detail, x: 0, y: torsoTop, clip: true });
  if (l.top === "apron") {
    layers.push({ img: p.apron[facing].bib, x: 0, y: torsoTop, clip: true });
    layers.push({ img: p.apron[facing].skirt, x: 0, y: torsoTop + 6 });
  }
  if (side) layers.push({ img: p.armSide[nearPose], x: 0, y: torsoTop });
  else {
    // Seen from behind, the person's left arm is on screen left too, and swings the same way.
    const left = p.armFront[nearPose];
    const right = p.armFront[farPose];
    layers.push({ img: left, x: torso.left - 2, y: torsoTop });
    layers.push({ img: right, x: torso.right + 1, y: torsoTop, mirror: true, map: SHADE });
  }
  layers.push({ img: p.head[facing], x: 0, y: headY });
  if (l.beard && facing !== "up") layers.push({ img: side ? p.beard.side : p.beard.down, x: 0, y: headY });
  if (l.hair !== "bald") layers.push({ img: p.hair[l.hair][facing], x: 0, y: headY });
  return stack(15, FRAME_H, layers);
}

export interface CharacterSheet {
  /** Frames by facing (down, up, side facing right) and frame (`FRAMES`), 16×24, palette indices. */
  frames: Indexed[][];
}

/** Compose every frame of a look: recoloured from the palette, outlined (D2). */
export function composeCharacter(l: Look, pal: Palette, lightness: readonly number[]): CharacterSheet {
  const table = slotTable(l, pal);
  const frames = FACINGS.map((f) =>
    FRAMES.map((_, k) => {
      const img = outline(remap(composeSlots(l, f, k), table), pal.lineOf, lightness);
      const out = blank(FRAME_W, FRAME_H);
      for (let y = 0; y < FRAME_H; y++) out.px.set(img.px.subarray(y * 15, y * 15 + 15), y * FRAME_W);
      return out;
    }),
  );
  return { frames };
}

/** A side frame facing left (the mirror of the drawn right-facing one, about the figure's centre column). */
export function faceLeft(img: Indexed): Indexed {
  const m = mirrorX(crop(img, 0, 0, 15, img.h));
  const out = blank(img.w, img.h);
  for (let y = 0; y < img.h; y++) out.px.set(m.px.subarray(y * 15, y * 15 + 15), y * img.w);
  return out;
}

// Re-exported for sheets.
export { grids };
