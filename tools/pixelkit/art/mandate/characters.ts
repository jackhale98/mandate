// Characters: a look (the appearance genome stand-in for P0) composed from
// the parts in ./characterParts into 16×24 frames, on the device (D10).
// Picks use rendezvous hashing (§3.5 rule 6) from the fixed hash, so the
// same seed always gives the same person, and adding a hair style later
// changes only the people it now wins for. Body, height, skin and hair are
// independent of clothing (§3.3).
//
// Round 3 (flat, graphic): one flat tone per material and at most one shade
// tone used as a hard shape; outlines in a darker colour of each material
// (`peopleLines`), not near-black everywhere; a contrast rule keeps hair
// apart from skin on every skin tone (`hairSteps`); hats, glasses, a lanyard,
// a backpack and three more cuts (hoodie, overalls, hi-vis vest, dress) so
// people differ by shape first; older people are grey and stoop.
import { stack, stretchRows, type LayerSpec } from "../../src/core/compose";
import { hexToRgb, rgbToLab, type Lab } from "../../src/core/color";
import { hash32, rendezvous } from "../../src/core/hash";
import { blank, clone, crop, grids, mirrorX, outline, remap, type Indexed } from "../../src/core/image";
import { ramp, type Palette } from "../../src/core/palette";
import { CLOTH_NAMES, HAIR_NAMES, LINES } from "./palette";
import {
  APRON,
  ARM_FRONT,
  ARM_SIDE,
  BACKPACK,
  BEARD,
  DRESS_SKIRT,
  GLASSES,
  HAIR_STYLES,
  HATS,
  HEAD,
  LANYARD,
  LEGEND,
  LEGS,
  LEGS_BROAD_DOWN,
  LEGS_SLIM_DOWN,
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
  type Hat,
  type Top,
} from "./characterParts";

export type Bottom = "trousers" | "shorts" | "skirt" | "dress";
export type Carry = "none" | "backpack" | "lanyard";

/** One person's look. In P1 the engine stores the semantic choices (§3.5); here they come from a seed. */
export interface Look {
  build: Build;
  /** 0 short, 1 middle, 2 tall. */
  height: 0 | 1 | 2;
  /** Skin ramp, 0 (MST 1) .. 5 (MST 10). */
  skin: number;
  hair: HairStyle;
  hairColour: (typeof HAIR_NAMES)[number];
  beard: boolean;
  /** Older: grey hair (picked with it) and a stoop (the head 1 px lower, and forward in the side view). */
  old: boolean;
  hat: Hat | "none";
  hatRamp: string;
  glasses: boolean;
  carry: Carry;
  bagRamp: string;
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
const one = (seed: number, salt: string, xs: readonly string[]): string => pick(seed, salt, opts(xs.map((c) => [c, 1] as const)));

interface Outfit {
  top: readonly string[];
  bottom: readonly (readonly [Bottom, number])[];
  bottomRamp: readonly string[] | "same";
  accent: readonly string[];
  /** Long sleeves (the forearm in the top's colour). */
  long: boolean;
  carry: readonly (readonly [Carry, number])[];
}

const OUTFITS: Record<Top, Outfit> = {
  tee: { top: ["red", "mustard", "green", "plum", "sky", "teal", "shirt", "navy"], bottom: [["trousers", 5], ["shorts", 2], ["skirt", 2]], bottomRamp: ["denim", "charcoal", "navy", "plum", "brown"], accent: ["red"], long: false, carry: [["none", 4], ["backpack", 1]] },
  suit: { top: ["navy", "charcoal", "brown", "denim"], bottom: [["trousers", 4], ["skirt", 1]], bottomRamp: "same", accent: ["red", "mustard", "sky", "green"], long: true, carry: [["none", 3], ["lanyard", 1]] },
  scrubs: { top: ["teal", "sky", "plum"], bottom: [["trousers", 1]], bottomRamp: "same", accent: ["navy", "red", "mustard"], long: false, carry: [["none", 1], ["lanyard", 2]] },
  apron: { top: ["shirt", "sky", "mustard", "red"], bottom: [["trousers", 3], ["skirt", 1]], bottomRamp: ["denim", "charcoal", "navy"], accent: ["green", "navy", "red", "brown"], long: true, carry: [["none", 1]] },
  hoodie: { top: ["charcoal", "red", "green", "plum", "sky", "navy", "mustard", "teal"], bottom: [["trousers", 4], ["shorts", 1]], bottomRamp: ["denim", "charcoal", "navy"], accent: ["red"], long: true, carry: [["none", 3], ["backpack", 2]] },
  overalls: { top: ["shirt", "red", "mustard", "sky", "green"], bottom: [["trousers", 1]], bottomRamp: ["denim", "navy", "green", "brown"], accent: ["red"], long: false, carry: [["none", 1]] },
  hivis: { top: ["charcoal", "navy", "shirt", "denim", "sky"], bottom: [["trousers", 1]], bottomRamp: ["denim", "charcoal", "navy"], accent: ["mustard", "flag"], long: false, carry: [["none", 1]] },
  dress: { top: ["red", "plum", "green", "teal", "sky", "mustard", "navy", "denim"], bottom: [["dress", 1]], bottomRamp: "same", accent: ["red"], long: false, carry: [["none", 3], ["backpack", 1]] },
};

/** Ramp names for an outfit colour (cloth, or a material ramp a garment shares). */
const clothRamp = (n: string) => (n === "brown" ? "wood" : (CLOTH_NAMES as readonly string[]).includes(n) ? `cloth_${n}` : n);

/** Hair styles a cap fits over (the others are too big for it). */
const UNDER_CAP: readonly HairStyle[] = ["crop", "buzz", "long", "bob", "ponytail", "bald"];

/** A townsperson's look from a seed (ambient people, P0). */
export function lookFromSeed(seed: number): Look {
  const s = hash32(seed, "look");
  const top = pick(s, "top", opts<Top>([["tee", 5], ["suit", 2], ["scrubs", 1], ["apron", 1], ["hoodie", 2], ["overalls", 1], ["hivis", 1], ["dress", 2]]));
  const o = OUTFITS[top];
  const topRamp = clothRamp(one(s, "top-colour", o.top));
  const bottom = pick(s, "bottom", opts(o.bottom));
  const bottomRamp = o.bottomRamp === "same" ? topRamp : clothRamp(one(s, "bottom-colour", o.bottomRamp));
  const old = hash32(s, "old") % 100 < 12;
  const hair = pick(s, "hair", opts<HairStyle>([["crop", 3], ["long", 3], ["bun", 2], ["coily", 3], ["bob", 2], ["buzz", 2], ["ponytail", 2], ["bald", old ? 4 : 1]]));
  let hat = pick(s, "hat", opts<Hat | "none">([["none", 10], ["cap", 2], ["scarf", 1]]));
  if (hat === "cap" && !UNDER_CAP.includes(hair)) hat = "none";
  if (top === "suit" && hat === "cap") hat = "none";
  const hairColour = old ? "grey" : pick(s, "hair-colour", opts<(typeof HAIR_NAMES)[number]>([["black", 3], ["dark_brown", 3], ["brown", 2], ["auburn", 1], ["blonde", 2]]));
  return {
    build: pick(s, "build", opts<Build>([["slim", 3], ["average", 4], ["broad", 3]])),
    height: pick(s, "height", opts<number>([[0, 1], [1, 1], [2, 1]])) as 0 | 1 | 2,
    skin: pick(s, "skin", opts<number>([0, 1, 2, 3, 4, 5].map((k) => [k, 1] as const))),
    hair,
    hairColour,
    beard: hat !== "scarf" && hash32(s, "beard") % 100 < (old ? 30 : 16),
    old,
    hat,
    hatRamp: clothRamp(one(s, "hat-colour", hat === "scarf" ? ["plum", "teal", "mustard", "navy", "green", "red"] : ["red", "navy", "mustard", "green", "charcoal", "sky"])),
    glasses: hash32(s, "glasses") % 100 < (old ? 40 : 14),
    carry: pick(s, "carry", opts(o.carry)),
    bagRamp: clothRamp(one(s, "bag-colour", ["charcoal", "brown", "navy", "red", "green", "mustard"])),
    top,
    topRamp,
    bottom,
    bottomRamp,
    accentRamp: clothRamp(one(s, "accent", o.accent)),
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
  old: false,
  hat: "none",
  hatRamp: "cloth_navy",
  glasses: false,
  carry: "none",
  bagRamp: "cloth_charcoal",
  top: "suit",
  topRamp: "cloth_navy",
  bottom: "trousers",
  bottomRamp: "cloth_navy",
  accentRamp: "flag",
  shoeRamp: "wood",
};

/** The six features the clone-rate lint compares (§3.3); what is on the head and what is carried count with hair and clothes. */
export function visibleFeatures(l: Look): string[] {
  const head = l.hat === "scarf" ? `scarf/${l.hatRamp}` : `${l.hair}${l.hat === "cap" ? `+cap/${l.hatRamp}` : ""}${l.glasses ? "+glasses" : ""}`;
  return [l.build, `skin${Math.floor(l.skin / 2)}`, head, l.hairColour, l.topRamp, `${l.top}/${l.bottom}/${l.carry}`];
}

// ---------------------------------------------------------------------------
// Parsed parts (once)

type Views = Record<FacingKey, Indexed>;
/** A block of three views side by side (down, up, side). */
const V = (t: string): Views => {
  const [down, up, side] = grids(t, LEGEND) as [Indexed, Indexed, Indexed];
  return { down, up, side };
};
/** A block of two views (down, side); up is empty. */
const V2 = (t: string): Record<FacingKey, Indexed | null> => {
  const [down, side] = grids(t, LEGEND) as [Indexed, Indexed];
  return { down, up: null, side };
};
const P = (t: string): Indexed => grids(t, LEGEND)[0] as Indexed;

let parsed: ReturnType<typeof parseParts> | null = null;
function parseParts() {
  const hair = Object.fromEntries(Object.entries(HAIR_STYLES).map(([k, v]) => [k, V(v)])) as Record<HairStyle, Views>;
  const hats = Object.fromEntries(Object.entries(HATS).map(([k, v]) => [k, V(v)])) as Record<Hat, Views>;
  const torso = Object.fromEntries(Object.entries(TORSO).map(([k, v]) => [k, { down: P(v.down), side: P(v.side), left: v.left, right: v.right }])) as Record<Build, { down: Indexed; side: Indexed; left: number; right: number }>;
  const armFront = Object.fromEntries(Object.entries(ARM_FRONT).map(([k, v]) => [k, P(v)])) as Record<ArmPose, Indexed>;
  const armSide = Object.fromEntries(Object.entries(ARM_SIDE).map(([k, v]) => [k, P(v)])) as Record<ArmPose, Indexed>;
  const legs = { down: LEGS.down.map(P), side: LEGS.side.map(P), broad: LEGS_BROAD_DOWN.map(P), slim: LEGS_SLIM_DOWN.map(P) };
  const detail = Object.fromEntries(Object.entries(TOP_DETAIL).map(([k, v]) => [k, v ? V(v) : null])) as Record<Top, Views | null>;
  const apronFull = V(APRON);
  const apron = Object.fromEntries(Object.entries(apronFull).map(([k, v]) => [k, { bib: crop(v, 0, 0, v.w, 6), skirt: crop(v, 0, 6, v.w, Math.max(0, v.h - 6)) }])) as Record<FacingKey, { bib: Indexed; skirt: Indexed }>;
  const [skirtDown, skirtSide] = grids(SKIRT, LEGEND) as [Indexed, Indexed];
  const [dressDown, dressSide] = grids(DRESS_SKIRT, LEGEND) as [Indexed, Indexed];
  return {
    head: V(HEAD),
    hair,
    hats,
    torso,
    armFront,
    armSide,
    legs,
    detail,
    apron,
    apronFull,
    beard: V2(BEARD),
    glasses: V2(GLASSES),
    lanyard: V(LANYARD),
    backpack: V(BACKPACK),
    skirt: { down: skirtDown, side: skirtSide },
    dress: { down: dressDown, side: dressSide },
  };
}
export const parts = () => (parsed ??= parseParts());

/** Every part as (name, image of slot codes), for the parts contact sheet and the lints. */
export function partList(): { name: string; img: Indexed }[] {
  const p = parts();
  const out: { name: string; img: Indexed }[] = [];
  const views = (name: string, v: Record<FacingKey, Indexed | null>) => {
    for (const f of FACINGS) if (v[f]) out.push({ name: `${name} ${f}`, img: v[f] as Indexed });
  };
  views("head", p.head);
  for (const [k, v] of Object.entries(p.hair)) views(`hair ${k}`, v);
  for (const [k, v] of Object.entries(p.hats)) views(`hat ${k}`, v);
  views("beard", p.beard);
  views("glasses", p.glasses);
  for (const [k, v] of Object.entries(p.torso)) out.push({ name: `torso ${k} down`, img: v.down }, { name: `torso ${k} side`, img: v.side });
  for (const [k, v] of Object.entries(p.armFront)) out.push({ name: `arm ${k} front`, img: v });
  for (const [k, v] of Object.entries(p.armSide)) out.push({ name: `arm ${k} side`, img: v });
  p.legs.down.forEach((img, i) => out.push({ name: `legs down ${i}`, img }));
  p.legs.side.forEach((img, i) => out.push({ name: `legs side ${i}`, img }));
  p.legs.broad.forEach((img, i) => out.push({ name: `legs broad ${i}`, img }));
  p.legs.slim.forEach((img, i) => out.push({ name: `legs slim ${i}`, img }));
  for (const [k, v] of Object.entries(p.detail)) if (v) views(`top ${k}`, v);
  views("apron", p.apronFull);
  out.push({ name: "skirt down", img: p.skirt.down }, { name: "skirt side", img: p.skirt.side });
  out.push({ name: "dress down", img: p.dress.down }, { name: "dress side", img: p.dress.side });
  views("lanyard", p.lanyard);
  views("backpack", p.backpack);
  return out;
}

// ---------------------------------------------------------------------------
// Slots to palette

function step(p: Palette, rampName: string, k: number): number {
  const r = ramp(p, rampName);
  const i = Math.min(r.steps.length - 1, Math.max(0, r.anchorStep + k));
  return r.steps[i] as number;
}

const labCache = new WeakMap<Palette, Lab[]>();
function labs(p: Palette): Lab[] {
  let l = labCache.get(p);
  if (!l) labCache.set(p, (l = p.colors.map((c) => rgbToLab(hexToRgb(c)))));
  return l;
}
const dE = (a: Lab, b: Lab) => Math.hypot(a.L - b.L, a.a - b.a, a.b - b.b);

/** Hair needs this Oklab distance from the skin, or the head reads as one blob. */
export const HAIR_SKIN_MIN = 0.1;

/**
 * The hair's base and shade for a skin (the contrast rule, round 3): the
 * ramp's middle step, unless it sits too close to the skin; then the step
 * further from the skin. Black hair on the darkest skin (the same colour as
 * MST 10) is drawn a soft charcoal black, the darkest colour that still
 * parts from it.
 */
export function hairSteps(p: Palette, skin: number, hairColour: string): [base: number, shade: number] {
  const lab = labs(p);
  const skinC = step(p, `skin${skin + 1}`, 0);
  const name = `hair_${hairColour}`;
  const at = (k: number) => step(p, name, k);
  const d = (k: number) => dE(lab[at(k)] as Lab, lab[skinC] as Lab);
  if (d(0) >= HAIR_SKIN_MIN) return [at(0), at(-1)];
  if (hairColour === "black") return [step(p, "cloth_charcoal", -1), step(p, "ink", 0)];
  // Lighter: the middle step becomes the shade; darker: the ramp has no step below, so the hair is one flat tone.
  return d(1) > d(-1) ? [at(1), at(0)] : [at(-1), at(-1)];
}

/** The colour of the eyes and glasses for a skin: a dark warm brown, or the darkest ink on dark skins. */
export function eyeColour(p: Palette, skin: number): number {
  return skin >= 4 ? step(p, "ink", -2) : p.colors.indexOf(LINES.warm);
}

/** The slot → palette table for a look. */
export function slotTable(l: Look, p: Palette): Uint8Array {
  const t = new Uint8Array(SLOT_COUNT);
  const skin = `skin${l.skin + 1}`;
  const [hair, hairS] = hairSteps(p, l.skin, l.hairColour);
  t[SLOT.skinS] = step(p, skin, -1);
  t[SLOT.skin] = step(p, skin, 0);
  // Flat: the old light slots take the base colour.
  t[SLOT.skinL] = t[SLOT.skin] as number;
  t[SLOT.hairS] = hairS;
  t[SLOT.hair] = hair;
  t[SLOT.hairL] = hair;
  t[SLOT.topS] = step(p, l.topRamp, -1);
  t[SLOT.top] = step(p, l.topRamp, 0);
  t[SLOT.topL] = t[SLOT.top] as number;
  t[SLOT.botS] = step(p, l.bottomRamp, -1);
  t[SLOT.bot] = step(p, l.bottomRamp, 0);
  t[SLOT.shoeS] = step(p, l.shoeRamp, -1);
  t[SLOT.shoe] = step(p, l.shoeRamp, 0);
  t[SLOT.accS] = step(p, l.accentRamp, -1);
  t[SLOT.acc] = step(p, l.accentRamp, 0);
  t[SLOT.accL] = t[SLOT.acc] as number;
  t[SLOT.ink] = eyeColour(p, l.skin);
  t[SLOT.white] = step(p, "chalk", 0);
  // The top of each eye: dark, except on the darkest skin, where a dark eye vanishes and a light catch shows where it is.
  // On light skins the top of the eye is the skin's shade (a 1 px eye under a soft brow: two dark pixels on a pale face read as sockets).
  t[SLOT.eye] = l.skin >= 5 ? step(p, "chalk", -1) : l.skin >= 3 ? (t[SLOT.ink] as number) : (t[SLOT.skinS] as number);
  t[SLOT.hatS] = step(p, l.hatRamp, -1);
  t[SLOT.hat] = step(p, l.hatRamp, 0);
  t[SLOT.bagS] = step(p, l.bagRamp, -1);
  t[SLOT.bag] = step(p, l.bagRamp, 0);
  const long = OUTFITS[l.top].long;
  t[SLOT.armR] = long ? (t[SLOT.top] as number) : (t[SLOT.skin] as number);
  t[SLOT.armRS] = long ? (t[SLOT.topS] as number) : (t[SLOT.skinS] as number);
  const legsCovered = l.bottom === "trousers";
  const bare = l.bottom === "skirt" || l.bottom === "dress";
  t[SLOT.legU] = bare ? (t[SLOT.skin] as number) : (t[SLOT.bot] as number);
  t[SLOT.legUS] = bare ? (t[SLOT.skinS] as number) : (t[SLOT.botS] as number);
  t[SLOT.legL] = legsCovered ? (t[SLOT.bot] as number) : (t[SLOT.skin] as number);
  t[SLOT.legLS] = legsCovered ? (t[SLOT.botS] as number) : (t[SLOT.skinS] as number);
  return t;
}

/** The material each slot paints for a look (for the flatness lint): its tones are counted together. */
export function slotMaterials(l: Look): string[] {
  const m: string[] = new Array(SLOT_COUNT).fill("");
  const set = (name: string, ...slots: number[]) => slots.forEach((s) => (m[s] = name));
  set("skin", SLOT.skinS, SLOT.skin, SLOT.skinL);
  set("hair", SLOT.hairS, SLOT.hair, SLOT.hairL);
  set("top", SLOT.topS, SLOT.top, SLOT.topL);
  set("bottoms", SLOT.botS, SLOT.bot);
  set("shoes", SLOT.shoeS, SLOT.shoe);
  set("accent", SLOT.accS, SLOT.acc, SLOT.accL);
  set("hat", SLOT.hatS, SLOT.hat);
  set("bag", SLOT.bagS, SLOT.bag);
  const long = OUTFITS[l.top].long;
  set(long ? "top" : "skin", SLOT.armR, SLOT.armRS);
  const bare = l.bottom === "skirt" || l.bottom === "dress";
  set(bare ? "skin" : "bottoms", SLOT.legU, SLOT.legUS);
  set(l.bottom === "trousers" ? "bottoms" : "skin", SLOT.legL, SLOT.legLS);
  return m;
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
  const legs = stretchRows((side ? p.legs.side : l.build === "broad" ? p.legs.broad : l.build === "slim" ? p.legs.slim : p.legs.down)[legIdx] as Indexed, 2, l.height - 1);
  const legsTop = 22 - (legs.h - 1);
  const torsoTop = legsTop - 6;
  // The chin sits on the shoulders (one row of overlap), which leaves room
  // above the tallest hair; an older person stoops: the head 1 px lower, and
  // forward in the side view.
  const headY = torsoTop - 10 + (frame === 1 ? 1 : 0) + (l.old ? 1 : 0);
  const headX = l.old && side ? 1 : 0;
  const torso = p.torso[l.build];
  const [nearPose, farPose] = WALK_ARMS[legIdx] as readonly [ArmPose, ArmPose];
  const layers: LayerSpec[] = [];
  if (side) layers.push({ img: p.armSide[farPose], x: 0, y: torsoTop, map: SHADE, under: true });
  layers.push({ img: legs, x: 0, y: legsTop });
  if (l.bottom === "skirt") layers.push({ img: side ? p.skirt.side : p.skirt.down, x: 0, y: legsTop });
  if (l.bottom === "dress") layers.push({ img: side ? p.dress.side : p.dress.down, x: 0, y: legsTop });
  layers.push({ img: side ? torso.side : torso.down, x: 0, y: torsoTop });
  const detail = p.detail[l.top]?.[facing];
  if (detail) layers.push({ img: detail, x: 0, y: torsoTop, clip: true });
  if (l.top === "apron") {
    layers.push({ img: p.apron[facing].bib, x: 0, y: torsoTop, clip: true });
    layers.push({ img: p.apron[facing].skirt, x: 0, y: torsoTop + 6 });
  }
  if (l.carry === "lanyard") layers.push({ img: p.lanyard[facing], x: 0, y: torsoTop, clip: true });
  if (l.carry === "backpack") {
    if (facing === "down") layers.push({ img: p.backpack.down, x: 0, y: torsoTop, clip: true });
    else if (facing === "side") layers.push({ img: p.backpack.side, x: 0, y: torsoTop, under: true });
  }
  if (side) layers.push({ img: p.armSide[nearPose], x: 0, y: torsoTop });
  else {
    // Seen from behind, the person's left arm is on screen left too, and swings the same way.
    const left = p.armFront[nearPose];
    const right = p.armFront[farPose];
    layers.push({ img: left, x: torso.left - 2, y: torsoTop });
    layers.push({ img: right, x: torso.right + 1, y: torsoTop, mirror: true });
  }
  // From behind, the backpack covers the back and the arms' tops.
  if (l.carry === "backpack" && facing === "up") layers.push({ img: p.backpack.up, x: 0, y: torsoTop });
  layers.push({ img: p.head[facing], x: headX, y: headY });
  const face = facing === "up" ? null : facing;
  if (face && l.beard) layers.push({ img: p.beard[face] as Indexed, x: headX, y: headY });
  if (l.hat !== "scarf") layers.push({ img: p.hair[l.hair][facing], x: headX, y: headY });
  if (face && l.glasses) layers.push({ img: p.glasses[face] as Indexed, x: headX, y: headY });
  if (l.hat !== "none") layers.push({ img: p.hats[l.hat][facing], x: headX, y: headY });
  return stack(15, FRAME_H, layers);
}

export interface CharacterSheet {
  /** Frames by facing (down, up, side facing right) and frame (`FRAMES`), 16×24, palette indices. */
  frames: Indexed[][];
}

/** How much darker than a material's darkest step its outline is aimed (Oklab L), and the lightest an outline may be. */
export const LINE_DROP = 0.2;
export const LINE_MAX_L = 0.36;

const lineCache = new WeakMap<Palette, number[]>();
/**
 * The people's outline colour per palette colour (D2; round 3: per material):
 * a palette colour of the same hue family as the material's darkest step,
 * aimed `LINE_DROP` darker than it and never lighter than `LINE_MAX_L`, so a
 * light shirt or pale skin gets a soft, coloured edge and a navy suit a
 * near-black one. The darkest colour of the palette when nothing is dark
 * enough. Chosen from colours the palette already has (no new colours).
 */
export function peopleLines(pal: Palette): number[] {
  const hit = lineCache.get(pal);
  if (hit) return hit;
  const lab = labs(pal);
  let darkest = 1;
  for (let i = 1; i < lab.length; i++) if ((lab[i] as Lab).L < (lab[darkest] as Lab).L) darkest = i;
  const out = pal.lineOf.map((hard, i) => {
    const ri = pal.rampOf[i] as number;
    const r = ri >= 0 ? pal.ramps[ri] : undefined;
    // Only ramp steps are materials; line colours keep their own line.
    if (!r || i === 0 || (pal.stepOf[i] as number) < 0) return hard;
    const d = lab[r.steps[0] as number] as Lab;
    const target = Math.min(d.L - LINE_DROP, LINE_MAX_L);
    let best = darkest;
    let bestScore = Infinity;
    for (let j = 1; j < lab.length; j++) {
      const c = lab[j] as Lab;
      if (c.L > target) continue;
      const score = target - c.L + 2 * Math.hypot(c.a - d.a, c.b - d.b);
      if (score < bestScore) {
        bestScore = score;
        best = j;
      }
    }
    return best;
  });
  lineCache.set(pal, out);
  return out;
}

/** The outline colours a look can produce (for the outline lint). */
export function lookLines(l: Look, pal: Palette): Set<number> {
  const lines = peopleLines(pal);
  return new Set([...slotTable(l, pal)].filter((v) => v > 0).map((v) => lines[v] as number));
}

/** Compose every frame of a look: recoloured from the palette, outlined (D2). */
export function composeCharacter(l: Look, pal: Palette, lightness: readonly number[]): CharacterSheet {
  const table = slotTable(l, pal);
  const lines = peopleLines(pal);
  const frames = FACINGS.map((f) =>
    FRAMES.map((_, k) => {
      const img = outline(innerGaps(remap(composeSlots(l, f, k), table), pal, lightness), lines, lightness);
      const out = blank(FRAME_W, FRAME_H);
      for (let y = 0; y < FRAME_H; y++) out.px.set(img.px.subarray(y * 15, y * 15 + 15), y * FRAME_W);
      return out;
    }),
  );
  return { frames };
}

/** One step darker in a colour's own ramp (its line when it is the darkest step). */
function shadeOf(pal: Palette, c: number): number {
  const r = pal.ramps[pal.rampOf[c] as number];
  const k = pal.stepOf[c] as number;
  return r && k > 0 ? (r.steps[k - 1] as number) : (peopleLines(pal)[c] as number);
}

/**
 * Inner edges (round 3): a one-pixel gap between two parts of the figure (an
 * arm and the body, the two legs) is drawn in the darker neighbour's shade
 * tone instead of the outline, so lines inside the figure stay soft and only
 * the silhouette is outlined [IKF].
 */
function innerGaps(flat: Indexed, pal: Palette, lightness: readonly number[]): Indexed {
  const img = clone(flat);
  const { w, h } = flat;
  for (let y = 0; y < h; y++) {
    for (let x = 1; x < w - 1; x++) {
      const i = y * w + x;
      if (flat.px[i] !== 0) continue;
      const a = flat.px[i - 1] as number;
      const b = flat.px[i + 1] as number;
      if (!a || !b) continue;
      const sa = shadeOf(pal, a);
      const sb = shadeOf(pal, b);
      img.px[i] = (lightness[sa] as number) <= (lightness[sb] as number) ? sa : sb;
    }
  }
  return img;
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
