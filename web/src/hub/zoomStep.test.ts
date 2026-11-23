import { describe, expect, it } from "vitest";
import { zoomStepFor } from "./hubView";

describe("the street's zoom step", () => {
  it("is subdivided to at most a quarter below DPR 2, and the lowest zoom stays on it", () => {
    expect(zoomStepFor(1)).toBeCloseTo(0.25, 9);
    expect(zoomStepFor(1.25)).toBeCloseTo(0.2, 9);
    expect(zoomStepFor(1.5)).toBeCloseTo(2 / 9, 9);
    for (const dpr of [1, 1.1, 1.25, 1.5, 1.75]) {
      const step = zoomStepFor(dpr);
      expect(step).toBeLessThanOrEqual(0.25 + 1e-9);
      const n = 1 / dpr / step;
      expect(Math.abs(n - Math.round(n))).toBeLessThan(1e-9);
    }
  });
  it("is the whole device pixel from DPR 2 up", () => {
    expect(zoomStepFor(2)).toBe(0.5);
    expect(zoomStepFor(3)).toBeCloseTo(1 / 3, 9);
    expect(zoomStepFor(2.625)).toBeCloseTo(1 / 2.625, 9);
  });
});
