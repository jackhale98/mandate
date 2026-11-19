// THROWAWAY PROTOTYPE (DEC-050, DEC-079, DEC-080; M6 groundwork): the
// walkable pixel-art town on the real engine. It starts the engine worker as
// the app does (web/src/engine/client), generates a world for the seed in the
// URL (?seed=, default 4242), lays out the player's town from the engine's
// `town` view (web/src/hub/townLayout) and lets you walk it. Reaching a place
// moves the player there in the engine and shows what the place offers.
// URL: seed=<text>, t=<minutes>, speed=<game minutes per second>, n=<townsfolk>,
// res=css|device, hud=1. Exposes window.__hub for tests and measurements.
import { engine, expectKind } from "../../../src/engine/client";
import type { LocationDto } from "../../../src/engine/protocol/LocationDto";
import type { PlacePanelView } from "../../../src/engine/protocol/PlacePanelView";
import type { TownPlacesView } from "../../../src/engine/protocol/TownPlacesView";
import { DayClock, clockText, phaseAt, type DayPhase } from "../../../src/hub/dayCycle";
import { HubView } from "../../../src/hub/hubView";
import { townInputFromView } from "../../../src/hub/townInput";
import { buildingOfPlace, layoutTown, townSeed, type TownBuilding, type TownMap } from "../../../src/hub/townLayout";
import { Translator } from "../../../src/i18n/fluent";
import { actionKey, placeIcon } from "../../../src/places/labels";

const tStart = performance.now();
const q = new URLSearchParams(location.search);
const seed = (q.get("seed") ?? "4242").trim() || "4242";
const startMinutes = Number(q.get("t") ?? 17 * 60 + 20);
const speed = q.has("speed") ? Number(q.get("speed")) : 12;
const townsfolk = Math.max(0, Math.min(400, Number(q.get("n") ?? 16) || 0));
const resolution = q.get("res") === "device" ? "device" : "css";

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const stage = $<HTMLDivElement>("stage");
const status = $<HTMLParagraphElement>("status");
const hud = $<HTMLPreElement>("hud");
const loading = $<HTMLDivElement>("loading");
const loadingText = $<HTMLParagraphElement>("loading-text");
const placeList = $<HTMLDivElement>("place-list");
const panel = $<HTMLElement>("panel");
const scrub = $<HTMLInputElement>("scrub");
const clockTime = $<HTMLSpanElement>("clock-time");
const play = $<HTMLButtonElement>("play");
const speedSel = $<HTMLSelectElement>("speed");
const seedInput = $<HTMLInputElement>("seed");
seedInput.value = seed;
hud.hidden = q.get("hud") !== "1";

const PHASE: Record<DayPhase, string> = {
  night: "night",
  pre_dawn: "before dawn",
  sunrise: "sunrise",
  morning: "morning",
  midday: "midday",
  afternoon: "afternoon",
  golden_hour: "golden hour",
  dusk: "dusk",
};

const clock = new DayClock(startMinutes, speed, speed !== 0);
if (!Array.from(speedSel.options).some((o) => Number(o.value) === speed) && speed > 0) speedSel.add(new Option(`${speed}×`, String(speed), true, true));
else if (speed > 0) speedSel.value = String(speed);

let tr = new Translator("en", "");
let view: HubView | null = null;
let map: TownMap | null = null;
let town: TownPlacesView | null = null;
let busy = false;
const timings: Record<string, number> = {};

// ---------------------------------------------------------------- engine

async function request<K extends Parameters<typeof expectKind>[1]>(body: Parameters<ReturnType<typeof engine>["request"]>[0], kind: K) {
  return expectKind(await engine().request(body), kind);
}

async function start() {
  const e = engine();
  e.onProgress((stage, fraction) => {
    loadingText.textContent = `Generating the world (${stage.replace(/_/g, " ")}, ${Math.round(fraction * 100)}%)…`;
  });
  await e.ready;
  timings.engineReady = performance.now() - tStart;
  loadingText.textContent = `Generating the world for seed “${seed}”…`;
  await e.request({ kind: "new_game", payload: { seed, size: "medium", scenario: null } });
  timings.world = performance.now() - tStart;
  const locale = await request({ kind: "query", payload: { type: "locale", lang: "en" } }, "locale");
  tr = new Translator(locale.lang, locale.ftl);
  town = await request({ kind: "query", payload: { type: "town" } }, "town");
  const input = townInputFromView(town);
  const t0 = performance.now();
  map = layoutTown(input, townSeed(seed, town.municipality));
  timings.layoutMs = performance.now() - t0;
  mount(map);
  loading.hidden = true;
  timings.ready = performance.now() - tStart;
}

function placeLabel(b: TownBuilding): string | null {
  if (b.home) return tr.t("place-home");
  if (b.place === null) return null;
  return b.name || tr.t(`place-${b.kind}`);
}

// ---------------------------------------------------------------- the town

function mount(m: TownMap) {
  const t0 = performance.now();
  view = new HubView(stage, {
    map: m,
    label: placeLabel,
    townsfolk,
    resolution,
    clock,
    onArrive: (b) => void arrive(b),
    onFrame: hudFrame,
  });
  timings.firstFrameMs = view.timings.firstFrameMs;
  timings.mountMs = performance.now() - t0;
  $<HTMLHeadingElement>("town-name").textContent = m.municipality;
  document.title = `${m.municipality} · town prototype`;
  buildPlaceList();
  syncClock();
  setStatus();
}

function buildPlaceList() {
  if (!town || !map) return;
  placeList.replaceChildren();
  for (const h of town.neighborhoods) {
    const heading = document.createElement("h2");
    heading.textContent = `${h.name} · ${tr.t(`hood-${h.kind}`)}`;
    const ul = document.createElement("ul");
    for (const s of h.spots) {
      const loc = s.location.location;
      const li = document.createElement("li");
      const btn = document.createElement("button");
      btn.type = "button";
      const name = s.location.kind_key === "streets" ? tr.t("ui-streets-of", { hood: h.name }) : s.location.name || tr.t(`place-${s.location.kind_key}`);
      const icon = document.createElement("span");
      icon.className = "icon";
      icon.setAttribute("aria-hidden", "true");
      icon.textContent = placeIcon(s.location.kind_key);
      btn.append(icon, name);
      btn.dataset.location = JSON.stringify(loc);
      btn.dataset.testid = loc.type === "place" ? `go-place-${loc.place}` : loc.type === "home" ? "go-home" : `go-street-${loc.neighborhood}`;
      btn.setAttribute("aria-label", `Walk to ${name}`);
      btn.addEventListener("click", () => {
        closePanel();
        // The same as tapping the building (or the street) on the map.
        view?.goToLocation(loc);
        setStatus(`Walking to <strong>${escapeHtml(name)}</strong>…`);
      });
      li.appendChild(btn);
      ul.appendChild(li);
    }
    placeList.append(heading, ul);
  }
  markHere();
}

function markHere() {
  if (!town) return;
  for (const b of placeList.querySelectorAll<HTMLButtonElement>("button[data-location]")) {
    const here = b.dataset.location === JSON.stringify(town.location);
    if (here) b.setAttribute("aria-current", "location");
    else b.removeAttribute("aria-current");
  }
}

function locationName(loc: LocationDto): string {
  if (!town || !map) return "";
  if (loc.type === "home") return tr.t("place-home");
  if (loc.type === "street") {
    const h = town.neighborhoods.find((x) => x.id === loc.neighborhood);
    return tr.t("ui-streets-of", { hood: h?.name ?? "" });
  }
  const b = buildingOfPlace(map, loc.place);
  return b ? (placeLabel(b) ?? "") : "";
}

function setStatus(html?: string) {
  if (!town) return;
  status.innerHTML = html ?? `You are at <strong>${escapeHtml(locationName(town.location))}</strong>`;
}

const escapeHtml = (s: string) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c] as string);

/** Reached a door: move there in the engine, then show what the place offers. */
async function arrive(b: TownBuilding | null) {
  if (!map || !town) return;
  let loc: LocationDto | null = null;
  if (b?.home) loc = { type: "home" };
  else if (b && b.place !== null) loc = { type: "place", place: b.place };
  else {
    // Open ground: the streets of the neighbourhood the player is in.
    const cell = view ? view.scene.playerCell : -1;
    const d = cell >= 0 ? map.districts[map.district[cell] ?? -1] : undefined;
    if (d) loc = { type: "street", neighborhood: d.id };
  }
  if (!loc || busy) {
    setStatus();
    return;
  }
  busy = true;
  try {
    if (JSON.stringify(loc) !== JSON.stringify(town.location)) await engine().request({ kind: "command", payload: { type: "move_to", location: loc } });
    town = await request({ kind: "query", payload: { type: "town" } }, "town");
    markHere();
    setStatus();
    if (loc.type !== "street") {
      const p = await request({ kind: "query", payload: { type: "place", location: loc } }, "place");
      openPanel(p);
    }
  } catch (e) {
    setStatus(`Could not go there: ${escapeHtml(e instanceof Error ? e.message : String(e))}`);
  } finally {
    busy = false;
  }
}

function openPanel(p: PlacePanelView) {
  const name = p.location.name || tr.t(`place-${p.location.kind_key}`);
  $<HTMLHeadingElement>("panel-title").textContent = name;
  $<HTMLParagraphElement>("panel-sub").textContent = p.location.name ? `${tr.t(`place-${p.location.kind_key}`)} · ${p.location.neighborhood_name}` : p.location.neighborhood_name;
  const body = $<HTMLDivElement>("panel-body");
  body.replaceChildren();
  const list = (title: string, items: string[]) => {
    if (items.length === 0) return;
    const h = document.createElement("p");
    h.textContent = title;
    const ul = document.createElement("ul");
    for (const i of items) {
      const li = document.createElement("li");
      li.textContent = i;
      ul.appendChild(li);
    }
    body.append(h, ul);
  };
  list("What you can do here", [...new Set(p.actions.map((a) => tr.t(actionKey(a.action, a.late_night))))]);
  list("What you can learn here", p.services.map((s) => tr.t(`short-${s}`)));
  list("Who is here", p.people.slice(0, 6).map((x) => x.name));
  if (body.childElementCount === 0) {
    const none = document.createElement("p");
    none.textContent = "Nothing to do here yet (M6).";
    body.appendChild(none);
  }
  panel.hidden = false;
  if (view) view.paused = true;
  $<HTMLButtonElement>("panel-close").focus();
}

function closePanel() {
  if (panel.hidden) return;
  panel.hidden = true;
  if (view) view.paused = false;
}
$<HTMLButtonElement>("panel-close").addEventListener("click", closePanel);
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") closePanel();
});

// ---------------------------------------------------------------- the clock

function syncClock() {
  const m = clock.minutes;
  if (document.activeElement !== scrub) scrub.value = String(Math.floor(m));
  clockTime.innerHTML = `<strong>${clockText(m)}</strong> · ${PHASE[phaseAt(m)]}`;
  play.textContent = clock.running ? "Pause" : "Play";
  play.setAttribute("aria-pressed", String(clock.running));
}
scrub.addEventListener("input", () => {
  view?.setClock(Number(scrub.value));
  syncClock();
});
play.addEventListener("click", () => {
  clock.running = !clock.running;
  if (clock.running && clock.speed === 0) clock.speed = Number(speedSel.value);
  view?.wake();
  syncClock();
});
speedSel.addEventListener("change", () => {
  clock.speed = Number(speedSel.value);
  view?.wake();
});
setInterval(() => {
  if (clock.running) syncClock();
}, 250);

const toggle = $<HTMLButtonElement>("places-toggle");
toggle.addEventListener("click", () => {
  const nav = $<HTMLElement>("places");
  const open = nav.dataset.open !== "true";
  nav.dataset.open = String(open);
  toggle.setAttribute("aria-expanded", String(open));
});
// Phones start with the list folded so the town fills the screen (DEC-048).
if (matchMedia("(max-width: 899px)").matches) {
  $<HTMLElement>("places").dataset.open = "false";
  toggle.setAttribute("aria-expanded", "false");
}

$<HTMLFormElement>("seed-form").addEventListener("submit", (e) => {
  e.preventDefault();
  const s = seedInput.value.trim();
  if (!s) return;
  const u = new URL(location.href);
  u.searchParams.set("seed", s);
  location.assign(u.toString());
});

// ---------------------------------------------------------------- HUD and test hooks

function hudFrame(v: HubView) {
  if (hud.hidden) return;
  const iv = v.timings.intervals.slice(-120);
  const avg = iv.length ? iv.reduce((a, b) => a + b, 0) / iv.length : 0;
  const st = v.renderer.stats;
  hud.textContent =
    `${avg ? (1000 / avg).toFixed(0) : "-"} fps  draw ${st.lastFrameMs.toFixed(2)} ms\n` +
    `frames ${st.frames}  skipped ${st.skipped}\n` +
    `bakes ${st.baseBakes}  shadow ${st.shadowBakes}  chunks ${st.chunks}\n` +
    `zoom ${v.zoom.toFixed(2)}  ${v.map.width}×${v.map.height} tiles`;
}

declare global {
  interface Window {
    __hub: unknown;
  }
}

window.__hub = {
  get ready() {
    return view !== null;
  },
  get view() {
    return view;
  },
  get map() {
    return map;
  },
  get town() {
    return town;
  },
  timings,
  /** Time to lay out any town here (measure-town.mjs times the largest capital under a CPU throttle). */
  layoutMs(input: Parameters<typeof layoutTown>[0], runs = 5) {
    let best = Infinity;
    for (let i = 0; i < runs; i++) {
      const t0 = performance.now();
      layoutTown(input, i + 1);
      best = Math.min(best, performance.now() - t0);
    }
    return best;
  },
  setClock(m: number) {
    clock.running = false;
    view?.setClock(m);
    syncClock();
  },
};

start().catch((e: unknown) => {
  loadingText.textContent = `The engine failed to start: ${e instanceof Error ? e.message : String(e)}`;
  console.error(e);
});
