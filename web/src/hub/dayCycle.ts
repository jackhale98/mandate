// The hub's continuous day cycle (DEC-013, DEC-049, DEC-080). Time of day
// is a clock in minutes (0..1440) that the view sets with `setClock`; in the
// game it comes from the engine's day-part plus the player's actions in it
// (`clockForDayPart`), in the prototype it runs on its own (`DayClock`).
// Everything here is pure: one keyframe table for the colour grade, the sun
// for building shadows, and how far the windows and street lamps are lit.

/** Minutes in a day. */
export const DAY_MINUTES = 1440;

export type Rgb = readonly [number, number, number];

/** What the renderer applies over the scene for one moment of the day. */
export interface Grade {
  /** Multiply tint (brightness folded in), 0..255 per channel. White leaves the scene as it is. */
  mul: Rgb;
  /** Soft-light colour over the scene (warmth at sunrise, blue at night), 0..255. */
  glow: Rgb;
  /** Strength of `glow`, 0..1. */
  glowAlpha: number;
  /** Saturation, 0 (grey) .. 1 (unchanged). */
  sat: number;
  /** Windows and street lamps, 0 (all off) .. 1 (all on). */
  lights: number;
}

export interface Keyframe extends Grade {
  /** Minutes after midnight. */
  at: number;
  /** A name for the moment (the prototype shows it; the game names day-parts itself). */
  name: DayPhase;
}

export type DayPhase = "night" | "pre_dawn" | "sunrise" | "morning" | "midday" | "afternoon" | "golden_hour" | "dusk";

const hex = (s: string): Rgb => [parseInt(s.slice(1, 3), 16), parseInt(s.slice(3, 5), 16), parseInt(s.slice(5, 7), 16)];

const NIGHT = { mul: hex("#5c6aae"), glow: hex("#1b2350"), glowAlpha: 0.3, sat: 0.55, lights: 1 } as const;

/**
 * The day's grading, in one table (sorted by `at`). Between two keyframes
 * every value blends smoothly (smoothstep), and the table wraps at midnight,
 * so the grade is continuous at every minute. The night colour leans on the
 * brand's dark navy (#11131a family, DEC-081), golden hour on its gold.
 */
export const DAY_KEYFRAMES: readonly Keyframe[] = [
  { at: 0, name: "night", ...NIGHT },
  { at: 270, name: "night", ...NIGHT },
  { at: 330, name: "pre_dawn", mul: hex("#7c80b4"), glow: hex("#38468a"), glowAlpha: 0.22, sat: 0.6, lights: 0.9 },
  { at: 390, name: "sunrise", mul: hex("#f2b496"), glow: hex("#ff8a40"), glowAlpha: 0.35, sat: 0.85, lights: 0.3 },
  { at: 480, name: "morning", mul: hex("#fff2de"), glow: hex("#ffd7a0"), glowAlpha: 0.12, sat: 0.95, lights: 0 },
  { at: 720, name: "midday", mul: hex("#ffffff"), glow: hex("#ffffff"), glowAlpha: 0, sat: 1, lights: 0 },
  { at: 900, name: "afternoon", mul: hex("#fff8ec"), glow: hex("#ffe4b0"), glowAlpha: 0.06, sat: 1, lights: 0 },
  { at: 1080, name: "golden_hour", mul: hex("#ffe0b4"), glow: hex("#f0a848"), glowAlpha: 0.3, sat: 1, lights: 0.1 },
  { at: 1170, name: "dusk", mul: hex("#a690d0"), glow: hex("#6a4aa8"), glowAlpha: 0.3, sat: 0.75, lights: 0.7 },
  { at: 1230, name: "night", ...NIGHT },
];

/** Wrap any number of minutes into [0, 1440). */
export function wrapMinutes(m: number): number {
  if (!Number.isFinite(m)) return 0;
  const r = m % DAY_MINUTES;
  return r < 0 ? r + DAY_MINUTES : r;
}

const smooth = (t: number) => t * t * (3 - 2 * t);
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const lerpRgb = (a: Rgb, b: Rgb, t: number): Rgb => [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)];

/** The two keyframes around `m` and how far between them (0..1, eased). */
function around(m: number): { a: Keyframe; b: Keyframe; t: number } {
  const k = DAY_KEYFRAMES;
  const last = k[k.length - 1] as Keyframe;
  const first = k[0] as Keyframe;
  for (let i = 0; i < k.length - 1; i++) {
    const a = k[i] as Keyframe;
    const b = k[i + 1] as Keyframe;
    if (m >= a.at && m < b.at) return { a, b, t: smooth((m - a.at) / (b.at - a.at)) };
  }
  // From the last keyframe round midnight to the first.
  const span = DAY_MINUTES - last.at + first.at;
  return { a: last, b: first, t: smooth((m - last.at) / span) };
}

/** The colour grade at a moment (any minutes; wrapped). Continuous in `minutes`. */
export function gradeAt(minutes: number): Grade {
  const { a, b, t } = around(wrapMinutes(minutes));
  return {
    mul: lerpRgb(a.mul, b.mul, t),
    glow: lerpRgb(a.glow, b.glow, t),
    glowAlpha: lerp(a.glowAlpha, b.glowAlpha, t),
    sat: lerp(a.sat, b.sat, t),
    lights: lerp(a.lights, b.lights, t),
  };
}

/** The named moment the clock is in (the last keyframe at or before it). */
export function phaseAt(minutes: number): DayPhase {
  const m = wrapMinutes(minutes);
  let name: DayPhase = (DAY_KEYFRAMES[0] as Keyframe).name;
  for (const k of DAY_KEYFRAMES) if (k.at <= m) name = k.name;
  return name;
}

/** "07:05" for a clock. */
export function clockText(minutes: number): string {
  const m = Math.floor(wrapMinutes(minutes));
  return `${String(Math.floor(m / 60)).padStart(2, "0")}:${String(m % 60).padStart(2, "0")}`;
}

// ---------------------------------------------------------------------------
// Sun and shadows

export const SUNRISE = 6 * 60 + 15;
export const SUNSET = 19 * 60 + 45;
/** Longest shadow, in tiles per tile of building height (a low sun, clamped). */
export const MAX_SHADOW = 2.6;
/** Darkest shadow (alpha of the shadow layer), at full sun. */
export const SHADOW_ALPHA = 0.34;
/** Sun positions pre-rendered per day: shadows are re-baked only when the bucket changes. */
export const SHADOW_BUCKETS = 28;

export interface Sun {
  /** Above the horizon. */
  up: boolean;
  /** 0 at sunrise .. 1 at sunset (clamped outside). */
  phase: number;
  /** Height in the sky, 0 (horizon) .. 1 (noon). */
  height: number;
  /** Shadow strength now (alpha of the shadow layer), 0 at night, fading in and out at the horizon. */
  shadowAlpha: number;
}

export function sunAt(minutes: number): Sun {
  const m = wrapMinutes(minutes);
  const up = m > SUNRISE && m < SUNSET;
  const phase = Math.min(1, Math.max(0, (m - SUNRISE) / (SUNSET - SUNRISE)));
  const height = up ? Math.sin(Math.PI * phase) : 0;
  // Fade over the first and last ~40 minutes of daylight.
  const fade = Math.min(1, height / 0.18);
  return { up, phase, height, shadowAlpha: up ? SHADOW_ALPHA * smooth(fade) : 0 };
}

/** The shadow bucket for a moment, or -1 at night (no shadows to draw). */
export function shadowBucket(minutes: number): number {
  const s = sunAt(minutes);
  if (!s.up) return -1;
  return Math.min(SHADOW_BUCKETS - 1, Math.floor(s.phase * SHADOW_BUCKETS));
}

/**
 * The shadow cast by one tile of height for a bucket, in tiles: west in the
 * morning, east in the evening, long when the sun is low and short at noon,
 * always a little to the south so a high sun still grounds a building (the
 * three-quarter view shows its south face, DEC-049).
 */
export function shadowVector(bucket: number): { dx: number; dy: number } {
  if (bucket < 0) return { dx: 0, dy: 0 };
  const phase = (bucket + 0.5) / SHADOW_BUCKETS;
  // Sun elevation, 8°..62°, and how long a shadow that gives.
  const elev = ((8 + 54 * Math.sin(Math.PI * phase)) * Math.PI) / 180;
  const length = Math.min(MAX_SHADOW, Math.cos(elev) / Math.sin(elev));
  return { dx: -Math.cos(Math.PI * phase) * length, dy: 0.22 + 0.12 * length };
}

// ---------------------------------------------------------------------------
// Lights

/**
 * How lit one light is (0..1) for the day's `lights` level: each light has
 * its own switch-on point in [0, 1) (`threshold`), and ramps over `width`,
 * so windows come on one by one through dusk and go off through dawn.
 */
export function lightLevel(lights: number, threshold: number, width = 0.14): number {
  return Math.min(1, Math.max(0, (lights - threshold) / width + 0.5));
}

// ---------------------------------------------------------------------------
// Clocks

/** Day-parts (DEC-013) as stretches of the clock: where a part's representative day starts and ends. */
export const DAY_PARTS = {
  morning: { from: 7 * 60, to: 12 * 60 },
  afternoon: { from: 12 * 60, to: 17 * 60 + 30 },
  evening: { from: 17 * 60 + 30, to: 22 * 60 },
  late_night: { from: 22 * 60, to: 26 * 60 },
} as const;
export type DayPart = keyof typeof DAY_PARTS;

/**
 * The clock for a day-part and how far through it the player is (0..1, e.g.
 * actions taken in it over actions available): the game's way to drive the
 * cycle from the engine without a wall clock.
 */
export function clockForDayPart(part: DayPart, progress: number): number {
  const p = DAY_PARTS[part];
  return wrapMinutes(p.from + (p.to - p.from) * Math.min(1, Math.max(0, progress)));
}

/**
 * A clock that runs on its own (the prototype: `speed` game minutes per real
 * second), or that holds still and eases forward to a new time when told
 * (the game: the turn's slots drive it, `easeTo`).
 */
export class DayClock {
  minutes: number;
  speed: number;
  running: boolean;
  private ease: { from: number; by: number; t: number; secs: number } | null = null;
  constructor(minutes = 12 * 60, speed = 20, running = true) {
    this.minutes = wrapMinutes(minutes);
    this.speed = speed;
    this.running = running;
  }
  set(minutes: number) {
    this.ease = null;
    this.minutes = wrapMinutes(minutes);
  }
  /** Move forward to `minutes` (always forward: evening to the next morning passes through the night) over `ms`. */
  easeTo(minutes: number, ms: number) {
    const by = wrapMinutes(minutes - this.minutes);
    if (ms <= 0 || by < 0.5) {
      this.set(minutes);
      return;
    }
    this.ease = { from: this.minutes, by, t: 0, secs: ms / 1000 };
  }
  /** Whether an `easeTo` is under way. */
  get easing(): boolean {
    return this.ease !== null;
  }
  /** Where an ease is going (or the time now). */
  get target(): number {
    return this.ease ? wrapMinutes(this.ease.from + this.ease.by) : this.minutes;
  }
  /** Advance by `dt` real seconds; returns whether the clock moved. */
  advance(dt: number): boolean {
    if (!(dt > 0)) return false;
    const e = this.ease;
    if (e) {
      e.t = Math.min(1, e.t + dt / e.secs);
      const k = e.t < 0.5 ? 2 * e.t * e.t : 1 - (-2 * e.t + 2) ** 2 / 2;
      this.minutes = wrapMinutes(e.from + e.by * k);
      if (e.t >= 1) this.ease = null;
      return true;
    }
    if (!this.running || this.speed === 0) return false;
    this.minutes = wrapMinutes(this.minutes + dt * this.speed);
    return true;
  }
}

/** How many of a town's townsfolk are out at a time of day, 0..1: everyone by day, a few late at night. */
export function townsfolkShare(minutes: number): number {
  const m = wrapMinutes(minutes);
  // Out from about 6:00, all out 8:00–19:00, fewer through the evening, a quarter after 23:00.
  const ramp = (a: number, b: number, x: number) => Math.min(1, Math.max(0, (x - a) / (b - a)));
  if (m < 6 * 60) return 0.25;
  if (m < 8 * 60) return 0.25 + 0.75 * ramp(6 * 60, 8 * 60, m);
  if (m < 19 * 60) return 1;
  if (m < 23 * 60) return 1 - 0.75 * ramp(19 * 60, 23 * 60, m);
  return 0.25;
}

// ---------------------------------------------------------------------------
// The grade as the canvas applies it (tests, and colours for HTML over the canvas)

const softLight = (b: number, s: number) => {
  if (s <= 0.5) return b - (1 - 2 * s) * b * (1 - b);
  const d = b <= 0.25 ? ((16 * b - 12) * b + 4) * b : Math.sqrt(b);
  return b + (2 * s - 1) * (d - b);
};
const lum = (c: Rgb) => 0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2];

/**
 * A colour (0..255) after the grade, in the order the renderer composites:
 * saturation (a grey "saturation" fill at 1 - sat), multiply, then soft light.
 */
export function applyGrade(c: Rgb, g: Grade): Rgb {
  const l = lum(c);
  const k = 1 - g.sat;
  const desat: Rgb = [c[0] + (l - c[0]) * k, c[1] + (l - c[1]) * k, c[2] + (l - c[2]) * k];
  const m: Rgb = [(desat[0] * g.mul[0]) / 255, (desat[1] * g.mul[1]) / 255, (desat[2] * g.mul[2]) / 255];
  const out = [0, 0, 0];
  for (let i = 0; i < 3; i++) {
    const b = (m[i] as number) / 255;
    const s = (g.glow[i] as number) / 255;
    out[i] = 255 * (b + (softLight(b, s) - b) * g.glowAlpha);
  }
  return [out[0] as number, out[1] as number, out[2] as number];
}
