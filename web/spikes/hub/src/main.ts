// THROWAWAY SPIKE (DEC-050, DEC-079): walkable hub rendering prototype.
// URL params: r=canvas|pixi, n=<townsfolk>, res=device|css, light=day|dusk,
// seed=<int>. Exposes window.__spike for web/spikes/hub/measure.mjs.
import { HubCamera } from "../../../src/hub/camera";
import { resolveTap } from "../../../src/hub/pathfinding";
import { buildArt, type Art } from "./art";
import type { Drawable, FrameInput, HubRenderer, RendererFactory } from "./renderer";
import {
  H,
  TILE,
  W,
  generate,
  idx,
  makeWalker,
  mulberry32,
  stepWalker,
  tileOf,
  walkFrame,
  walkTo,
  type Building,
  type Walker,
} from "./scene";

const tStart = performance.now();
const q = new URLSearchParams(location.search);
const rendererName = q.get("r") === "pixi" ? "pixi" : "canvas";
const npcCount = Math.max(0, Math.min(1000, Number(q.get("n") ?? 60) || 0));
// device: backing store at devicePixelRatio; css: DPR 1; low: 1/3 of CSS px
// (about one backing pixel per art pixel at zoom 3), upscaled by the compositor.
const resMode = q.get("res") === "css" ? "css" : q.get("res") === "low" ? "low" : "device";
const dusk = q.get("light") === "dusk";
const seed = Number(q.get("seed") ?? 7) || 7;

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const stage = $<HTMLDivElement>("stage");
const labelLayer = $<HTMLDivElement>("labels");
const hud = $<HTMLPreElement>("hud");
const interior = $<HTMLDivElement>("interior");
const toast = $<HTMLDivElement>("toast");

// ---------------------------------------------------------------- scene

const town = generate(seed);
const art: Art = buildArt(town);
const tArt = performance.now();

interface StaticItem extends Drawable {
  l: number;
  t: number;
  r: number;
  b: number;
}
const bounds = (d: Drawable): StaticItem => {
  const fr = art.frames[d.frame] ?? { x: 0, y: 0, w: 0, h: 0, ax: 0, ay: 0 };
  const l = d.x - fr.ax;
  const t = d.y - fr.ay;
  return { ...d, l, t, r: l + fr.w, b: t + fr.h };
};
const statics: StaticItem[] = [];
for (const b of town.buildings) statics.push(bounds({ id: statics.length, frame: art.buildingFrame[b.id] ?? 0, x: b.x * TILE, y: (b.y + b.h) * TILE }));
for (const p of town.props) statics.push(bounds({ id: statics.length, frame: art.propFrame[p.kind], x: p.x * TILE + TILE / 2, y: p.y * TILE + TILE - 1 }));
statics.sort((a, b) => a.y - b.y);
const glows: Drawable[] = town.props
  .filter((p) => p.kind === "lamp")
  .map((p, i) => ({ id: 50000 + i, frame: art.glow, x: p.x * TILE + TILE / 2, y: p.y * TILE - 14 }));

const rng = mulberry32(seed * 31 + 1);
const randomTarget = () => town.targets[Math.floor(rng() * town.targets.length)] ?? 0;
const npcs: Walker[] = [];
for (let i = 0; i < npcCount; i++) {
  const w = makeWalker(i, randomTarget(), i % art.variants, 20 + rng() * 18);
  w.wait = rng() * 2;
  npcs.push(w);
}
const council = town.buildings.find((b) => b.enterable) as Building;
const player = makeWalker(9999, idx(council.doorX - 1, council.doorY), art.variants, 64);

// ---------------------------------------------------------------- camera

// The M6 hub camera (web/src/hub/camera.ts): pinch about the fingers'
// midpoint, re-baselined when a finger lifts, eased limits, wheel at the cursor.
const camera = new HubCamera({ x: player.x, y: player.y - 8, zoom: 3 }, { width: 1, height: 1 }, {
  minZoom: 1.25,
  maxZoom: 6,
  bounds: { minX: 0, minY: 0, maxX: W * TILE, maxY: H * TILE },
});
const cam = camera.state;
let follow = true;
let cssW = 1;
let cssH = 1;
let dpr = 1;

// ---------------------------------------------------------------- frame data

const dyn: Drawable[] = [];
const order: number[] = [];
const sprites: Drawable[] = [];
const decals: Drawable[] = [];
const doorArrow: Drawable = { id: 99999, frame: art.doorArrow, x: 0, y: 0 };
let marker: Drawable | null = null;
const input: FrameInput = {
  camX: 0,
  camY: 0,
  zoom: 3,
  decals,
  decalCount: 0,
  sprites,
  spriteCount: 0,
  tint: dusk ? 0x8a80b8 : null,
  glows,
  glowCount: glows.length,
};

function collect(now: number) {
  const hw = cssW / 2 / cam.zoom + 8;
  const hh = cssH / 2 / cam.zoom + 8;
  const vl = cam.x - hw;
  const vr = cam.x + hw;
  const vt = cam.y - hh;
  const vb = cam.y + hh + 48; // tall sprites below the view can reach up
  // Dynamic drawables, kept nearly sorted between frames (insertion sort).
  const walkers = npcs.length + 1;
  while (dyn.length < walkers) {
    order.push(dyn.length);
    dyn.push({ id: 0, frame: 0, x: 0, y: 0 });
  }
  const fill = (d: Drawable, w: Walker, moving: boolean) => {
    d.id = 10000 + w.id;
    d.frame = art.charBase + (w.variant * 4 + w.dir) * 3 + walkFrame(w, moving);
    d.x = Math.round(w.x);
    d.y = Math.round(w.y);
  };
  for (let i = 0; i < npcs.length; i++) fill(dyn[i] as Drawable, npcs[i] as Walker, moving[i] === 1);
  fill(dyn[npcs.length] as Drawable, player, playerMoving);
  for (let i = 1; i < order.length; i++) {
    const v = order[i] as number;
    const y = (dyn[v] as Drawable).y;
    let j = i - 1;
    while (j >= 0 && (dyn[order[j] as number] as Drawable).y > y) {
      order[j + 1] = order[j] as number;
      j--;
    }
    order[j + 1] = v;
  }
  // Merge with the pre-sorted statics, culling against the view.
  let n = 0;
  let si = 0;
  let di = 0;
  while (si < statics.length || di < order.length) {
    const s = statics[si];
    const d = di < order.length ? (dyn[order[di] as number] as Drawable) : undefined;
    if (s && (!d || s.y <= d.y)) {
      si++;
      if (s.r < vl || s.l > vr || s.b < vt || s.t > vb) continue;
      sprites[n++] = s;
    } else if (d) {
      di++;
      if (d.x < vl || d.x > vr || d.y < vt || d.y - 24 > vb) continue;
      sprites[n++] = d;
    }
  }
  // Door marker for the one enterable place, bobbing above everything.
  doorArrow.x = council.doorX * TILE + TILE / 2;
  doorArrow.y = council.doorY * TILE - 22 + Math.round(Math.sin(now / 250) * 2);
  sprites[n++] = doorArrow;
  input.spriteCount = n;
  input.decalCount = 0;
  if (marker) decals[input.decalCount++] = marker;
  input.camX = cam.x;
  input.camY = cam.y;
  input.zoom = cam.zoom;
}

// ---------------------------------------------------------------- simulation step (UI-side)

const moving = new Uint8Array(npcCount);
let playerMoving = false;
function update(dt: number) {
  for (let i = 0; i < npcs.length; i++) {
    const w = npcs[i] as Walker;
    const m = stepWalker(w, dt);
    moving[i] = m ? 1 : 0;
    if (!m && !w.walk) {
      w.wait -= dt;
      if (w.wait <= 0) {
        if (!walkTo(town, w, randomTarget())) w.wait = 1;
        else w.wait = 0.5 + rng() * 2.5;
      }
    }
  }
  playerMoving = stepWalker(player, dt);
  if (!playerMoving && !player.walk) marker = null;
  if (follow && !camera.gesturing) {
    // Follow the player, but only as far as the view stays over the block.
    const k = Math.min(1, dt * 6);
    const t = camera.clampCentre(player.x, player.y - 8);
    cam.x += (t.x - cam.x) * k;
    cam.y += (t.y - cam.y) * k;
  }
  // Ease back inside the zoom limits and the block's edges (never snapped).
  camera.update(dt);
}

// ---------------------------------------------------------------- places (DEC-079)

function showToast(text: string) {
  toast.textContent = text;
  toast.hidden = false;
  window.setTimeout(() => (toast.hidden = true), 1800);
}

function goToPlace(b: Building) {
  follow = true;
  const door = idx(b.doorX, b.doorY);
  marker = { id: 88888, frame: art.marker, x: b.doorX * TILE + TILE / 2, y: b.doorY * TILE + TILE - 3 };
  walkTo(town, player, door, () => {
    marker = null;
    if (b.enterable) openInterior(b);
    else showToast(`${b.name ?? "A house"}: not enterable in this spike`);
  });
}

let paused = false;
function openInterior(b: Building) {
  $<HTMLHeadingElement>("interior-title").textContent = b.name ?? "Place";
  interior.hidden = false;
  paused = true; // nothing animates behind the panel: save the battery
  $<HTMLButtonElement>("interior-close").focus();
}
$<HTMLButtonElement>("interior-close").addEventListener("click", () => {
  interior.hidden = true;
  paused = false;
  last = performance.now();
  schedule();
});

const labels: { b: Building; el: HTMLButtonElement }[] = [];
const placeList = $<HTMLUListElement>("place-list");
for (const b of town.buildings) {
  if (!b.name) continue;
  const el = document.createElement("button");
  el.className = b.enterable ? "label enterable" : "label";
  el.textContent = b.name;
  el.setAttribute("aria-label", `${b.enterable ? "Enter" : "Go to"} ${b.name}`);
  el.tabIndex = -1; // the list below is the keyboard / screen-reader route
  el.addEventListener("click", (e) => {
    e.stopPropagation();
    goToPlace(b);
  });
  labelLayer.appendChild(el);
  labels.push({ b, el });
  const li = document.createElement("li");
  const btn = document.createElement("button");
  btn.textContent = b.enterable ? `Enter ${b.name}` : `Go to ${b.name}`;
  btn.addEventListener("click", () => goToPlace(b));
  li.appendChild(btn);
  placeList.appendChild(li);
}
function placeLabels() {
  const show = cam.zoom >= 1.8;
  for (const { b, el } of labels) {
    const sx = ((b.x + b.w / 2) * TILE - cam.x) * cam.zoom + cssW / 2;
    const sy = ((b.y - 1) * TILE - cam.y) * cam.zoom + cssH / 2;
    const vis = show && sx > -80 && sx < cssW + 80 && sy > -20 && sy < cssH + 20;
    el.style.display = vis ? "" : "none";
    if (vis) el.style.transform = `translate(${Math.round(sx)}px, ${Math.round(sy)}px) translate(-50%, -100%)`;
  }
}

// ---------------------------------------------------------------- input

function tap(sx: number, sy: number) {
  const p = camera.screenToWorld({ x: sx, y: sy });
  // A building (or its roof) goes to its door; open ground to that spot; an
  // obstacle to the nearest reachable spot on the player's side of it.
  const target = resolveTap(town.grid, town.footprints, tileOf(player), { x: p.x / TILE, y: p.y / TILE });
  if (!target) return;
  if (target.kind === "door") {
    goToPlace(town.buildings[target.building] as Building);
    return;
  }
  follow = true;
  const goal = target.cell;
  if (walkTo(town, player, goal)) marker = { id: 88888, frame: art.marker, x: (goal % W) * TILE + TILE / 2, y: Math.floor(goal / W) * TILE + TILE - 3 };
}

const local = (e: PointerEvent | WheelEvent) => {
  const r = stage.getBoundingClientRect();
  return { x: e.clientX - r.left, y: e.clientY - r.top };
};
const pointer = (e: PointerEvent) => ({ id: e.pointerId, ...local(e) });
stage.addEventListener("pointerdown", (e) => {
  if (e.target !== stage && !(e.target instanceof HTMLCanvasElement)) return;
  stage.setPointerCapture(e.pointerId);
  camera.pointerDown(pointer(e));
});
stage.addEventListener("pointermove", (e) => {
  const { x, y, zoom } = cam;
  camera.pointerMove(pointer(e));
  // The player took the camera: stop following until the next walk.
  if (cam.x !== x || cam.y !== y || cam.zoom !== zoom) follow = false;
});
stage.addEventListener("pointerup", (e) => {
  const t = camera.pointerUp(pointer(e));
  if (t) tap(t.x, t.y);
});
stage.addEventListener("pointercancel", (e) => camera.pointerCancel(e.pointerId));
stage.addEventListener(
  "wheel",
  (e) => {
    e.preventDefault();
    follow = false;
    camera.wheel(local(e), e.deltaY);
  },
  { passive: false },
);

// ---------------------------------------------------------------- metrics

const RING = 240;
const ringDt = new Float64Array(RING);
const ringCpu = new Float64Array(RING);
let ringN = 0;
const rec = { on: false, deltas: [] as number[], cpu: [] as number[], render: [] as number[], sprites: [] as number[] };
const longTasks: { start: number; duration: number }[] = [];
let loadLongTasks: { start: number; duration: number }[] = [];
try {
  new PerformanceObserver((list) => {
    for (const e of list.getEntries()) longTasks.push({ start: e.startTime, duration: e.duration });
  }).observe({ type: "longtask", buffered: true });
} catch {
  // longtask is Chromium-only
}
const pct = (arr: ArrayLike<number>, n: number, p: number) => {
  const a = Array.from({ length: n }, (_, i) => arr[i] as number).sort((x, y) => x - y);
  return a.length ? (a[Math.min(a.length - 1, Math.floor(p * a.length))] as number) : 0;
};

// ---------------------------------------------------------------- autopilot (measure.mjs)

let auto: { t0: number; route: number[]; leg: number } | null = null;
function autopilotStep(now: number) {
  if (!auto) return;
  const t = (now - auto.t0) / 1000;
  if (!playerMoving && !player.walk) {
    const goal = auto.route[auto.leg % auto.route.length] as number;
    auto.leg++;
    walkTo(town, player, goal);
  }
  if (t < 2.5) {
    follow = true;
    cam.zoom = 3;
  } else if (t < 5) {
    cam.zoom = 3 + 1.6 * Math.sin(((t - 2.5) / 2.5) * Math.PI * 2); // pinch in and out
  } else if (t < 7.5) {
    follow = false; // drag pan in a circle around the player
    const a = ((t - 5) / 2.5) * Math.PI * 2;
    cam.zoom = 3;
    cam.x = player.x + Math.cos(a) * 90;
    cam.y = player.y + Math.sin(a) * 90;
  } else {
    follow = true;
    cam.zoom = 3 - Math.min(1, (t - 7.5) / 1.5); // wide view: the most sprites on screen
  }
}

// ---------------------------------------------------------------- loop

let renderer: HubRenderer;
let last = performance.now();
let raf = 0;
let firstFrameAt = 0;
let hudAt = 0;
const schedule = () => {
  if (!raf && !paused) raf = requestAnimationFrame(frame);
};

function frame(now: number) {
  raf = 0;
  const dt = Math.min(0.1, (now - last) / 1000);
  const frameDt = now - last;
  last = now;
  const c0 = performance.now();
  autopilotStep(now);
  update(dt);
  collect(now);
  const c1 = performance.now();
  renderer.render(input);
  const renderMs = performance.now() - c1;
  placeLabels();
  const cpu = performance.now() - c0;
  if (!firstFrameAt) firstFrameAt = performance.now();
  else {
    ringDt[ringN % RING] = frameDt;
    ringCpu[ringN % RING] = cpu;
    ringN++;
    if (rec.on) {
      rec.deltas.push(frameDt);
      rec.cpu.push(cpu);
      rec.render.push(renderMs);
      rec.sprites.push(input.spriteCount + input.decalCount + 1);
    }
  }
  if (now - hudAt > 500) {
    hudAt = now;
    const n = Math.min(ringN, RING);
    const p50 = pct(ringDt, n, 0.5);
    const p95 = pct(ringDt, n, 0.95);
    const sum = Array.from({ length: n }, (_, i) => ringDt[i] as number).reduce((a, b) => a + b, 0);
    hud.textContent =
      `${renderer.info}\n` +
      `N=${npcCount}  res=${resMode}  ${dusk ? "dusk" : "day"}  zoom ${cam.zoom.toFixed(2)}\n` +
      `${n ? ((1000 * n) / sum).toFixed(1) : "-"} fps  frame p50 ${p50.toFixed(1)} / p95 ${p95.toFixed(1)} ms\n` +
      `cpu p95 ${pct(ringCpu, n, 0.95).toFixed(2)} ms  sprites ${input.spriteCount + input.decalCount + 1}` +
      `${input.tint !== null ? ` + ${glows.length} glows` : ""}\n` +
      `backing ${Math.round(cssW * dpr)}x${Math.round(cssH * dpr)} @${dpr.toFixed(2)}`;
  }
  schedule();
}

function resize() {
  cssW = Math.max(1, stage.clientWidth);
  cssH = Math.max(1, stage.clientHeight);
  dpr = resMode === "css" ? 1 : resMode === "low" ? 1 / 3 : window.devicePixelRatio || 1;
  camera.resize(cssW, cssH);
  renderer.resize(cssW, cssH, dpr);
}

declare global {
  interface Window {
    __spike: unknown;
  }
}

async function main() {
  const canvas = document.createElement("canvas");
  canvas.setAttribute("role", "img");
  canvas.setAttribute("aria-label", "Town street block. Use the Places list to travel.");
  stage.prepend(canvas);
  const tImport0 = performance.now();
  const mod: { createRenderer: RendererFactory } = rendererName === "pixi" ? await import("./pixiRenderer") : await import("./canvasRenderer");
  const tImport = performance.now();
  renderer = await mod.createRenderer(canvas, art);
  const tInit = performance.now();
  resize();
  new ResizeObserver(() => resize()).observe(stage);
  // Draw the first frame synchronously so time-to-first-frame is honest.
  last = performance.now();
  collect(last);
  renderer.render(input);
  placeLabels();
  firstFrameAt = performance.now();
  schedule();

  const route = [idx(34, 14), idx(34, 29), idx(14, 29), idx(14, 14), idx(council.doorX - 1, council.doorY)];
  window.__spike = {
    ready: true,
    info: renderer.info,
    timings: { artMs: tArt - tStart, importMs: tImport - tImport0, initMs: tInit - tImport, firstFrameAt, scriptStart: tStart },
    autopilot() {
      auto = { t0: performance.now(), route, leg: 0 };
      player.walk = null;
    },
    start() {
      rec.on = true;
      rec.deltas = [];
      rec.cpu = [];
      rec.render = [];
      rec.sprites = [];
      loadLongTasks = [...longTasks];
      longTasks.length = 0;
    },
    stop() {
      rec.on = false;
      return { deltas: rec.deltas, cpu: rec.cpu, render: rec.render, sprites: rec.sprites, longTasks: [...longTasks], loadLongTasks, backing: [Math.round(cssW * dpr), Math.round(cssH * dpr)] };
    },
    enter() {
      goToPlace(council);
    },
  };
}

main().catch((e: unknown) => {
  hud.textContent = `failed: ${String(e)}`;
  console.error(e);
});
