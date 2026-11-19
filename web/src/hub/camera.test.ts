import { HubCamera, type Vec } from "./camera";

const VIEW = { width: 400, height: 800 };
const make = (opts: Partial<ConstructorParameters<typeof HubCamera>[2]> = {}) =>
  new HubCamera({ x: 300, y: 200, zoom: 3 }, VIEW, { minZoom: 1.25, maxZoom: 6, ...opts });

const near = (a: Vec, b: Vec, eps = 1e-9) => {
  expect(a.x).toBeCloseTo(b.x, -Math.log10(eps));
  expect(a.y).toBeCloseTo(b.y, -Math.log10(eps));
};
const mid = (a: Vec, b: Vec) => ({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 });

describe("pinch zoom", () => {
  it("keeps the world point under the fingers' midpoint across zoom steps", () => {
    const cam = make();
    const a = { x: 120, y: 300 };
    const b = { x: 220, y: 420 };
    cam.pointerDown({ id: 1, ...a });
    cam.pointerDown({ id: 2, ...b });
    const anchor = cam.screenToWorld(mid(a, b));
    // Spread, pinch, and drift: the same town point stays under the midpoint.
    const steps: [Vec, Vec][] = [
      [{ x: 110, y: 290 }, { x: 240, y: 440 }],
      [{ x: 90, y: 270 }, { x: 260, y: 470 }],
      [{ x: 140, y: 330 }, { x: 180, y: 380 }],
      [{ x: 160, y: 360 }, { x: 300, y: 500 }],
    ];
    let zooms = 0;
    for (const [pa, pb] of steps) {
      const before = cam.state.zoom;
      cam.pointerMove({ id: 1, ...pa });
      cam.pointerMove({ id: 2, ...pb });
      if (cam.state.zoom !== before) zooms++;
      near(cam.screenToWorld(mid(pa, pb)), anchor, 1e-6);
    }
    expect(zooms).toBeGreaterThanOrEqual(3);
  });

  it("zoom follows the ratio of finger distances", () => {
    const cam = make();
    cam.pointerDown({ id: 1, x: 100, y: 400 });
    cam.pointerDown({ id: 2, x: 200, y: 400 });
    cam.pointerMove({ id: 2, x: 250, y: 400 });
    expect(cam.state.zoom).toBeCloseTo(4.5, 9);
  });

  it("lifting one finger causes no jump; the next 6 px move pans 6 px", () => {
    const cam = make();
    cam.pointerDown({ id: 1, x: 100, y: 400 });
    cam.pointerDown({ id: 2, x: 200, y: 400 });
    cam.pointerMove({ id: 2, x: 240, y: 430 });
    const before = { ...cam.state };
    expect(cam.pointerUp({ id: 2, x: 240, y: 430 })).toBeNull();
    expect(cam.state).toEqual(before);
    const under = cam.screenToWorld({ x: 100, y: 400 });
    cam.pointerMove({ id: 1, x: 106, y: 400 });
    expect((before.x - cam.state.x) * cam.state.zoom).toBeCloseTo(6, 9);
    expect(cam.state.y).toBeCloseTo(before.y, 9);
    expect(cam.state.zoom).toBe(before.zoom);
    near(cam.screenToWorld({ x: 106, y: 400 }), under, 1e-9);
  });

  it("the first finger lifting (not the second) also re-baselines", () => {
    const cam = make();
    cam.pointerDown({ id: 1, x: 100, y: 400 });
    cam.pointerDown({ id: 2, x: 200, y: 400 });
    cam.pointerMove({ id: 1, x: 80, y: 380 });
    const before = { ...cam.state };
    cam.pointerUp({ id: 1, x: 80, y: 380 });
    expect(cam.state).toEqual(before);
    cam.pointerMove({ id: 2, x: 200, y: 406 });
    expect((before.y - cam.state.y) * cam.state.zoom).toBeCloseTo(6, 9);
  });

  it("a cancelled pointer re-baselines the same way, and is never a tap", () => {
    const cam = make();
    cam.pointerDown({ id: 1, x: 100, y: 400 });
    cam.pointerDown({ id: 2, x: 200, y: 400 });
    cam.pointerMove({ id: 2, x: 260, y: 400 });
    const before = { ...cam.state };
    cam.pointerCancel(2);
    expect(cam.state).toEqual(before);
    cam.pointerMove({ id: 1, x: 94, y: 400 });
    expect((cam.state.x - before.x) * cam.state.zoom).toBeCloseTo(6, 9);
    expect(cam.pointerUp({ id: 1, x: 94, y: 400 })).toBeNull();
  });

  it("a third finger landing mid-pinch does not jump either", () => {
    const cam = make();
    cam.pointerDown({ id: 1, x: 100, y: 400 });
    cam.pointerDown({ id: 2, x: 200, y: 400 });
    cam.pointerMove({ id: 2, x: 230, y: 400 });
    const before = { ...cam.state };
    cam.pointerDown({ id: 3, x: 300, y: 600 });
    expect(cam.state).toEqual(before);
  });
});

describe("limits", () => {
  it("past the maximum the pinch meets resistance, then eases back (no snap), keeping the focus still", () => {
    const cam = make();
    cam.pointerDown({ id: 1, x: 150, y: 400 });
    cam.pointerDown({ id: 2, x: 250, y: 400 });
    cam.pointerMove({ id: 1, x: 0, y: 400 });
    cam.pointerMove({ id: 2, x: 400, y: 400 }); // raw zoom 12, twice the maximum
    const over = cam.state.zoom;
    expect(over).toBeGreaterThan(6);
    expect(over).toBeLessThan(12);
    cam.pointerUp({ id: 1, x: 0, y: 400 });
    cam.pointerUp({ id: 2, x: 400, y: 400 });
    expect(cam.state.zoom).toBe(over); // nothing snaps on release
    const focus = { x: 400, y: 400 }; // the last finger's position
    const anchor = cam.screenToWorld(focus);
    const zooms: number[] = [];
    for (let i = 0; i < 90; i++) {
      cam.update(1 / 60);
      zooms.push(cam.state.zoom);
      near(cam.screenToWorld(focus), anchor, 1e-6);
    }
    expect(zooms[0]).toBeLessThan(over);
    expect(zooms[0]).toBeGreaterThan(6.01);
    for (let i = 1; i < zooms.length; i++) expect(zooms[i] as number).toBeLessThanOrEqual(zooms[i - 1] as number);
    expect(cam.state.zoom).toBe(6);
    expect(cam.update(1 / 60)).toBe(false);
  });

  it("past the minimum the same, from below", () => {
    const cam = make();
    cam.pointerDown({ id: 1, x: 0, y: 400 });
    cam.pointerDown({ id: 2, x: 400, y: 400 });
    cam.pointerMove({ id: 2, x: 40, y: 400 }); // raw zoom 0.3
    const under = cam.state.zoom;
    expect(under).toBeLessThan(1.25);
    expect(under).toBeGreaterThan(0.3);
    cam.pointerUp({ id: 1, x: 0, y: 400 });
    cam.pointerUp({ id: 2, x: 40, y: 400 });
    cam.update(1 / 60);
    expect(cam.state.zoom).toBeGreaterThan(under);
    expect(cam.state.zoom).toBeLessThan(1.25);
    for (let i = 0; i < 120; i++) cam.update(1 / 60);
    expect(cam.state.zoom).toBe(1.25);
  });

  it("nothing eases while fingers are down", () => {
    const cam = make();
    cam.pointerDown({ id: 1, x: 150, y: 400 });
    cam.pointerDown({ id: 2, x: 250, y: 400 });
    cam.pointerMove({ id: 2, x: 450, y: 400 });
    const z = cam.state.zoom;
    expect(cam.update(1 / 60)).toBe(false);
    expect(cam.state.zoom).toBe(z);
  });

  it("panned past the town's edge, the view eases back inside", () => {
    const cam = make({ bounds: { minX: 0, minY: 0, maxX: 768, maxY: 512 } });
    cam.pointerDown({ id: 1, x: 200, y: 400 });
    cam.pointerMove({ id: 1, x: 390, y: 400 });
    cam.pointerMove({ id: 1, x: 400, y: 790 });
    cam.pointerUp({ id: 1, x: 400, y: 790 });
    const start = { ...cam.state };
    const target = cam.clampCentre(start.x, start.y);
    expect(target).not.toEqual({ x: start.x, y: start.y });
    cam.update(1 / 60);
    expect(cam.state.y).not.toBe(target.y); // eased, not snapped
    expect(Math.abs(cam.state.y - target.y)).toBeLessThan(Math.abs(start.y - target.y));
    for (let i = 0; i < 120; i++) cam.update(1 / 60);
    expect(cam.state.x).toBeCloseTo(target.x, 6);
    expect(cam.state.y).toBeCloseTo(target.y, 6);
  });

  it("can settle on whole-number zoom steps (DEC-080)", () => {
    const cam = make({ integerZoom: true });
    cam.wheel({ x: 200, y: 400 }, -150); // zoom × e^0.225 ≈ 3.76
    for (let i = 0; i < 120; i++) cam.update(1 / 60);
    expect(cam.state.zoom).toBe(4);
  });

  it("can settle on whole device-pixel steps (zoomStep = 1/dpr)", () => {
    const cam = make({ integerZoom: true, zoomStep: 1 / 3, minZoom: 1 / 3 });
    cam.wheel({ x: 200, y: 400 }, -150); // 9 → 11.27 device px; zooming in snaps up to 12
    for (let i = 0; i < 120; i++) cam.update(1 / 60);
    expect(cam.state.zoom * 3).toBeCloseTo(12, 9);
    expect(cam.settledZoom(0.1)).toBeCloseTo(1 / 3, 9); // floor: one device pixel per art pixel
  });
});

describe("whole steps follow the direction of travel", () => {
  it("one small wheel notch always takes a step, in and out", () => {
    const cam = make({ integerZoom: true, zoomStep: 1 / 3, minZoom: 1 / 3 });
    const settle = () => {
      for (let i = 0; i < 120; i++) cam.update(1 / 60);
    };
    settle();
    const start = Math.round(cam.state.zoom * 3);
    cam.wheel({ x: 200, y: 400 }, -40); // a small notch in: well under half a step
    settle();
    expect(Math.round(cam.state.zoom * 3)).toBe(start + 1);
    cam.wheel({ x: 200, y: 400 }, 40); // and back out
    settle();
    expect(Math.round(cam.state.zoom * 3)).toBe(start);
  });
});

describe("pan, tap, wheel", () => {
  it("a short press is a tap, a drag is not", () => {
    const cam = make();
    cam.pointerDown({ id: 1, x: 100, y: 100 });
    cam.pointerMove({ id: 1, x: 104, y: 103 });
    expect(cam.pointerUp({ id: 1, x: 104, y: 103 })).toEqual({ x: 104, y: 103 });
    expect(cam.state).toMatchObject({ x: 300, y: 200, zoom: 3 });
    cam.pointerDown({ id: 1, x: 100, y: 100 });
    cam.pointerMove({ id: 1, x: 130, y: 100 });
    expect(cam.pointerUp({ id: 1, x: 130, y: 100 })).toBeNull();
    expect(cam.state.x).toBeCloseTo(290, 9);
  });

  it("a drag keeps the pressed town point under the finger", () => {
    const cam = make();
    const anchor = cam.screenToWorld({ x: 100, y: 100 });
    cam.pointerDown({ id: 1, x: 100, y: 100 });
    for (const p of [
      { x: 120, y: 110 },
      { x: 200, y: 300 },
      { x: 50, y: 20 },
    ]) {
      cam.pointerMove({ id: 1, ...p });
      near(cam.screenToWorld(p), anchor, 1e-9);
    }
  });

  it("two-finger taps are not taps", () => {
    const cam = make();
    cam.pointerDown({ id: 1, x: 100, y: 100 });
    cam.pointerDown({ id: 2, x: 140, y: 100 });
    expect(cam.pointerUp({ id: 2, x: 140, y: 100 })).toBeNull();
    expect(cam.pointerUp({ id: 1, x: 100, y: 100 })).toBeNull();
  });

  it("the wheel zooms about the cursor", () => {
    const cam = make();
    const at = { x: 50, y: 700 };
    const anchor = cam.screenToWorld(at);
    for (const d of [-100, -100, 60, 300]) {
      cam.wheel(at, d);
      near(cam.screenToWorld(at), anchor, 1e-9);
    }
  });

  it("world and screen transforms are inverse", () => {
    const cam = make();
    const p = { x: 123.4, y: 567.8 };
    near(cam.worldToScreen(cam.screenToWorld(p)), p, 1e-9);
  });
});
