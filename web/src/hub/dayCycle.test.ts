import {
  DAY_KEYFRAMES,
  DAY_MINUTES,
  DayClock,
  SHADOW_BUCKETS,
  applyGrade,
  clockForDayPart,
  clockText,
  gradeAt,
  lightLevel,
  phaseAt,
  shadowBucket,
  shadowVector,
  sunAt,
  wrapMinutes,
  type Grade,
  type Rgb,
} from "./dayCycle";

// Colours a town is made of: grass, pavement, a red roof, a white wall, the gold accent.
const SAMPLES: Rgb[] = [
  [92, 148, 76],
  [160, 156, 148],
  [176, 74, 56],
  [236, 230, 214],
  [232, 180, 76],
];

const maxDelta = (a: Grade, b: Grade) => {
  let d = 0;
  for (const c of SAMPLES) {
    const x = applyGrade(c, a);
    const y = applyGrade(c, b);
    for (let i = 0; i < 3; i++) d = Math.max(d, Math.abs((x[i] as number) - (y[i] as number)));
  }
  return Math.max(d / 255, Math.abs(a.lights - b.lights), Math.abs(a.sat - b.sat), Math.abs(a.glowAlpha - b.glowAlpha));
};

describe("day cycle grading", () => {
  it("keyframes are sorted, inside the day, and go through every phase the owner asked for", () => {
    for (let i = 1; i < DAY_KEYFRAMES.length; i++) expect((DAY_KEYFRAMES[i] as { at: number }).at).toBeGreaterThan((DAY_KEYFRAMES[i - 1] as { at: number }).at);
    expect(DAY_KEYFRAMES.every((k) => k.at >= 0 && k.at < DAY_MINUTES)).toBe(true);
    const order = [...new Set(DAY_KEYFRAMES.map((k) => k.name))];
    expect(order).toEqual(["night", "pre_dawn", "sunrise", "morning", "midday", "afternoon", "golden_hour", "dusk"]);
  });

  it("is continuous: no visible jump between consecutive minutes, or across midnight", () => {
    let worst = 0;
    for (let m = 0; m < DAY_MINUTES; m += 0.25) worst = Math.max(worst, maxDelta(gradeAt(m), gradeAt(m + 0.25)) * 4);
    // At most 1.5% of the colour range per game minute: under one 8-bit level per
    // frame even with the clock at a game hour per real second.
    expect(worst).toBeLessThan(0.015);
    expect(maxDelta(gradeAt(DAY_MINUTES - 1e-6), gradeAt(0))).toBeLessThan(1e-4);
    // Both sides of every keyframe agree.
    for (const k of DAY_KEYFRAMES) expect(maxDelta(gradeAt(k.at - 1e-6), gradeAt(k.at + 1e-6))).toBeLessThan(1e-4);
  });

  it("never just switches between light and dark: every hour looks different from the next", () => {
    const seen = new Set<string>();
    for (let h = 5; h < 22; h++) {
      const g = gradeAt(h * 60);
      seen.add(applyGrade(SAMPLES[3] as Rgb, g).map((v) => Math.round(v / 4)).join(","));
    }
    expect(seen.size).toBeGreaterThanOrEqual(14);
  });

  it("noon is neutral, golden hour warm, dusk blue-purple, night dark and blue", () => {
    const white: Rgb = [255, 255, 255];
    const noon = applyGrade(white, gradeAt(12 * 60));
    expect(noon.every((v) => v > 250)).toBe(true);
    const golden = applyGrade(white, gradeAt(18 * 60));
    expect(golden[0]).toBeGreaterThan(golden[2] + 40);
    const dusk = applyGrade(white, gradeAt(19 * 60 + 30));
    expect(dusk[2]).toBeGreaterThan(dusk[1]);
    expect(dusk[0]).toBeGreaterThan(dusk[1]);
    const night = applyGrade(white, gradeAt(2 * 60));
    expect(night[2]).toBeGreaterThan(night[0]);
    expect(Math.max(...night)).toBeLessThan(170);
    const sunrise = applyGrade(white, gradeAt(6 * 60 + 30));
    expect(sunrise[0]).toBeGreaterThan(sunrise[2] + 30);
  });

  it("phases and clock text", () => {
    expect(phaseAt(3 * 60)).toBe("night");
    expect(phaseAt(6 * 60 + 40)).toBe("sunrise");
    expect(phaseAt(12 * 60)).toBe("midday");
    expect(phaseAt(18 * 60 + 10)).toBe("golden_hour");
    expect(phaseAt(19 * 60 + 40)).toBe("dusk");
    expect(phaseAt(23 * 60)).toBe("night");
    expect(clockText(7 * 60 + 5)).toBe("07:05");
    expect(clockText(-1)).toBe("23:59");
    expect(wrapMinutes(1440 * 3 + 10)).toBe(10);
  });
});

describe("sun and shadows", () => {
  it("shadows are short at noon, long morning and evening, absent at night", () => {
    const len = (m: number) => {
      const v = shadowVector(shadowBucket(m));
      return Math.hypot(v.dx, v.dy);
    };
    expect(shadowBucket(2 * 60)).toBe(-1);
    expect(shadowBucket(22 * 60)).toBe(-1);
    expect(sunAt(2 * 60).shadowAlpha).toBe(0);
    expect(len(13 * 60)).toBeLessThan(0.9);
    expect(len(7 * 60 + 15)).toBeGreaterThan(1.5 * len(13 * 60));
    expect(len(18 * 60 + 45)).toBeGreaterThan(1.5 * len(13 * 60));
    // West in the morning, east in the evening.
    expect(shadowVector(shadowBucket(8 * 60)).dx).toBeLessThan(0);
    expect(shadowVector(shadowBucket(17 * 60)).dx).toBeGreaterThan(0);
  });

  it("shadow strength fades in and out with the sun (no pop at sunrise or sunset)", () => {
    let worst = 0;
    for (let m = 0; m < DAY_MINUTES; m += 0.5) worst = Math.max(worst, Math.abs(sunAt(m).shadowAlpha - sunAt(m + 0.5).shadowAlpha) * 2);
    expect(worst).toBeLessThan(0.02);
  });

  it("buckets change a few dozen times a day, in order", () => {
    let changes = 0;
    let prev = shadowBucket(0);
    for (let m = 1; m < DAY_MINUTES; m++) {
      const b = shadowBucket(m);
      if (b !== prev) changes++;
      if (b >= 0 && prev >= 0) expect(b).toBeGreaterThanOrEqual(prev);
      prev = b;
    }
    expect(changes).toBe(SHADOW_BUCKETS + 1);
  });
});

describe("lights", () => {
  it("come on gradually through dusk and go off through dawn", () => {
    expect(gradeAt(12 * 60).lights).toBe(0);
    expect(gradeAt(2 * 60).lights).toBe(1);
    // Over dusk, the share of windows lit rises steadily (thresholds spread over 0..1).
    const thresholds = Array.from({ length: 50 }, (_, i) => 0.08 + (i / 50) * 0.8);
    const lit = (m: number) => thresholds.reduce((a, t) => a + lightLevel(gradeAt(m).lights, t), 0) / thresholds.length;
    let prev = lit(17 * 60);
    for (let m = 17 * 60 + 5; m <= 21 * 60; m += 5) {
      const v = lit(m);
      expect(v).toBeGreaterThanOrEqual(prev - 1e-9);
      expect(v - prev).toBeLessThan(0.12);
      prev = v;
    }
    expect(lit(17 * 60)).toBeLessThan(0.1);
    expect(lit(21 * 60)).toBeGreaterThan(0.95);
    expect(lit(9 * 60)).toBe(0);
  });
});

describe("clocks", () => {
  it("DayClock advances, wraps and pauses", () => {
    const c = new DayClock(1430, 20);
    expect(c.advance(1)).toBe(true);
    expect(c.minutes).toBeCloseTo(10);
    c.running = false;
    expect(c.advance(1)).toBe(false);
    expect(c.minutes).toBeCloseTo(10);
  });

  it("day-parts map onto the clock", () => {
    expect(clockForDayPart("morning", 0)).toBe(7 * 60);
    expect(clockForDayPart("evening", 1)).toBe(22 * 60);
    expect(clockForDayPart("late_night", 1)).toBe(2 * 60);
    expect(phaseAt(clockForDayPart("evening", 0.25))).toBe("golden_hour");
  });
});
