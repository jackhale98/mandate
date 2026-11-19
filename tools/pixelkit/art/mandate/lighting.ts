// The day's light for the pack (§3.4): grading keyframes, and the two night
// looks the owner chooses between (D12):
//   - "multiply": one grade (saturation, multiply, soft light) over the whole
//     scene, as the renderer applies it today;
//   - "moods": every palette colour of the static layers (ground, buildings,
//     props) is remapped to a designed colour for the moment (darker and bluer
//     at night, warmer at golden hour, violet at dusk), and only a gentler
//     multiply darkens the people, who stay readable.
// Everything is computed here, ahead of time, into a table every 15 minutes;
// the runtime only blends two neighbouring entries.
import { hexToRgb, labToRgb, mixLab, rgbToLab, type Rgb8 } from "../../src/core/color";
import type { Palette } from "../../src/core/palette";

export interface GradeKey {
  at: number;
  mul: string;
  glow: string;
  glowAlpha: number;
  sat: number;
  lights: number;
  /** Palette mood: lightness scale and lift, chroma scale, the hue colours lean to and how far (0..1). */
  mood: { L: number; lift: number; C: number; hue: number; amt: number };
  /** In the moods look, how much of the multiply the people keep (0 none .. 1 all). */
  sprite: number;
}

const NIGHT: Omit<GradeKey, "at"> = {
  mul: "#5a66aa",
  glow: "#1b2350",
  glowAlpha: 0.3,
  sat: 0.55,
  lights: 1,
  mood: { L: 0.4, lift: 0.035, C: 0.5, hue: 266, amt: 1 },
  sprite: 0.42,
};

/** Keyframes (minutes after midnight), wrapping at midnight. Night leans on the brand navy, golden hour on its gold. */
export const KEYS: readonly GradeKey[] = [
  { at: 0, ...NIGHT },
  { at: 270, ...NIGHT },
  { at: 330, mul: "#7c80b4", glow: "#38468a", glowAlpha: 0.22, sat: 0.6, lights: 0.9, mood: { L: 0.52, lift: 0.035, C: 0.6, hue: 258, amt: 0.9 }, sprite: 0.6 },
  { at: 390, mul: "#f2b496", glow: "#ff8a40", glowAlpha: 0.32, sat: 0.85, lights: 0.3, mood: { L: 0.88, lift: 0.0, C: 1.08, hue: 42, amt: 0.6 }, sprite: 0.5 },
  { at: 480, mul: "#fff4e2", glow: "#ffd7a0", glowAlpha: 0.1, sat: 0.95, lights: 0, mood: { L: 1, lift: 0, C: 1, hue: 80, amt: 0.06 }, sprite: 0.5 },
  { at: 720, mul: "#ffffff", glow: "#ffffff", glowAlpha: 0, sat: 1, lights: 0, mood: { L: 1, lift: 0, C: 1, hue: 0, amt: 0 }, sprite: 1 },
  { at: 900, mul: "#fff8ec", glow: "#ffe4b0", glowAlpha: 0.06, sat: 1, lights: 0, mood: { L: 1, lift: 0, C: 1, hue: 75, amt: 0.04 }, sprite: 0.5 },
  { at: 1080, mul: "#ffe0b4", glow: "#f0a848", glowAlpha: 0.28, sat: 1, lights: 0.1, mood: { L: 0.97, lift: 0, C: 1.15, hue: 62, amt: 0.65 }, sprite: 0.45 },
  { at: 1170, mul: "#a892d0", glow: "#6a4aa8", glowAlpha: 0.28, sat: 0.8, lights: 0.7, mood: { L: 0.66, lift: 0.02, C: 0.95, hue: 318, amt: 0.85 }, sprite: 0.55 },
  { at: 1230, ...NIGHT },
];

export const STEP_MINUTES = 15;
export const STEPS = 1440 / STEP_MINUTES;

export interface LightEntry {
  mul: Rgb8;
  glow: Rgb8;
  glowAlpha: number;
  sat: number;
  lights: number;
  /** The moods look: the people's gentler multiply, and the static layers' palette (stored ÷ that multiply, so the final colour is the mood's). */
  spriteMul: Rgb8;
  moodPalette: string[];
}

export interface Lighting {
  stepMinutes: number;
  entries: LightEntry[];
  shadowColor: string;
}

const smooth = (t: number) => t * t * (3 - 2 * t);
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

function around(m: number): { a: GradeKey; b: GradeKey; t: number } {
  for (let i = 0; i < KEYS.length - 1; i++) {
    const a = KEYS[i] as GradeKey;
    const b = KEYS[i + 1] as GradeKey;
    if (m >= a.at && m < b.at) return { a, b, t: smooth((m - a.at) / (b.at - a.at)) };
  }
  const last = KEYS[KEYS.length - 1] as GradeKey;
  const first = KEYS[0] as GradeKey;
  return { a: last, b: first, t: smooth((m - last.at) / (1440 - last.at + first.at)) };
}

/**
 * A palette colour in a mood, designed in Oklab: lightness scaled and lifted
 * (so night keeps its darkest steps apart), chroma scaled, and everything
 * pushed toward the mood's hue (blue at night, violet at dusk, gold at golden
 * hour), greys included.
 */
export function moodColour(c: Rgb8, mood: GradeKey["mood"]): Rgb8 {
  const lab = rgbToLab(c);
  const r = (mood.hue * Math.PI) / 180;
  const tint = 0.05 * mood.amt;
  return labToRgb({ L: lab.L * mood.L + mood.lift, a: lab.a * mood.C + tint * Math.cos(r), b: lab.b * mood.C + tint * Math.sin(r) });
}

export function buildLighting(p: Palette): Lighting {
  const rgb = p.colors.map(hexToRgb);
  const entries: LightEntry[] = [];
  for (let k = 0; k < STEPS; k++) {
    const { a, b, t } = around(k * STEP_MINUTES);
    const mul = mixLab(hexToRgb(a.mul), hexToRgb(b.mul), t);
    const glow = mixLab(hexToRgb(a.glow), hexToRgb(b.glow), t);
    const sprite = lerp(a.sprite, b.sprite, t);
    const spriteMul: Rgb8 = [Math.round(255 + (mul[0] - 255) * sprite), Math.round(255 + (mul[1] - 255) * sprite), Math.round(255 + (mul[2] - 255) * sprite)];
    const moodPalette = rgb.map((c, i) => {
      if (i === 0) return "#000000";
      const ca = moodColour(c, a.mood);
      const cb = moodColour(c, b.mood);
      const la = rgbToLab(ca);
      const lb = rgbToLab(cb);
      const m = labToRgb({ L: lerp(la.L, lb.L, t), a: lerp(la.a, lb.a, t), b: lerp(la.b, lb.b, t) });
      const stored = [0, 1, 2].map((ch) => Math.min(255, Math.round(((m[ch] as number) * 255) / Math.max(1, spriteMul[ch] as number))));
      return `#${stored.map((v) => v.toString(16).padStart(2, "0")).join("")}`;
    });
    entries.push({ mul, glow, glowAlpha: lerp(a.glowAlpha, b.glowAlpha, t), sat: lerp(a.sat, b.sat, t), lights: lerp(a.lights, b.lights, t), spriteMul, moodPalette });
  }
  return { stepMinutes: STEP_MINUTES, entries, shadowColor: "#1c1f3c" };
}
