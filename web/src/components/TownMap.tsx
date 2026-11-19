// The town map overview (DEC-086): an illustrated low-resolution map drawn
// from the same versioned layout as the walkable street (DEC-085), shown full
// screen over the street (DEC-088 main screen). Picking a building hands it
// to `onPick` (the town screen closes the map and walks the player there in
// the street). Every hotspot is a real button (keyboard, screen readers), and
// the pocket's Places tab stays the text alternative with exactly the same
// places (DEC-051). Presentation only: the
// engine says what a place offers, where the player is and what is waiting.
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { LocationDto } from "../engine/protocol/LocationDto";
import type { PocketView } from "../engine/protocol/PocketView";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";
import { bakeTownMap, type BakedTownMap } from "../hub/render/townMapRenderer";
import type { TownInput } from "../hub/townInput";
import { dressTown } from "../hub/layoutV2/dress";
import { townSeed, type TownMap as TownMapModel } from "../hub/townLayout";
import { buildTownMapScene, hotspotOf, viewForInput, type MapHotspot, type TownMapScene } from "../hub/townMapScene";
import { TownMapView, WALK_MAX_MS } from "../hub/townMapView";
import { LIGHT_EASE_MS, PASSAGE_EASE_MS, shownMinutes } from "../hub/turnClock";
import type { TownGeo } from "../hub/layoutV2/geo";
import { planInWorker, townJobs } from "../hub/townWorkerClient";
import { liveRecipe } from "../hub/gameTown";
import { IdbPlanStore, TownPlanCache } from "../hub/townPlanCache";
import type { Translator } from "../i18n/fluent";
import { placeIcon, placeName } from "../places/labels";
import { useGame } from "../state/gameStore";

// ---------------------------------------------------------------------------
// Scene and bake, made once per dressed map and place list (moving around does not change them)

export interface BuiltTownMap {
  key: string;
  map: TownMapModel;
  scene: TownMapScene;
  baked: BakedTownMap;
  /** ms: scene (hotspots, main routes), bake, and the bake's stages. */
  timings: { sceneMs: number; bakeMs: number } & BakedTownMap["stages"];
}

/** What the scene depends on besides the map: the town's place list, not where the player is. */
function townKey(town: TownPlacesView): string {
  const parts = [town.municipality, town.home_neighborhood ?? "-"];
  for (const h of town.neighborhoods) {
    parts.push(`${h.id}:${h.kind}:${h.name}`);
    for (const s of h.spots) parts.push(`${JSON.stringify(s.location.location)}${s.location.kind_key}${s.location.name}${s.actions.length + s.services.length > 0 ? "+" : "-"}`);
  }
  return parts.join("|");
}

export function buildTownMap(map: TownMapModel, view: TownPlacesView): Omit<BuiltTownMap, "key"> {
  const t0 = performance.now();
  const scene = buildTownMapScene(map, view);
  const t1 = performance.now();
  const baked = bakeTownMap(scene);
  const t2 = performance.now();
  return { map, scene, baked, timings: { sceneMs: t1 - t0, bakeMs: t2 - t1, ...baked.stages } };
}

let cached: BuiltTownMap | null = null;
const mapIds = new WeakMap<TownMapModel, number>();
let nextMapId = 1;
function townMapFor(map: TownMapModel, town: TownPlacesView): BuiltTownMap {
  let id = mapIds.get(map);
  if (id === undefined) mapIds.set(map, (id = nextMapId++));
  const key = `${id}|${townKey(town)}`;
  if (cached?.key !== key) cached = { key, ...buildTownMap(map, town) };
  return cached;
}

// ---------------------------------------------------------------------------
// Overlays from what the UI already has (pocket and town views)

const keyOf = (l: LocationDto) => (l.type === "home" ? "home" : l.type === "street" ? `streets:${l.neighborhood}` : `place:${l.place}`);

interface Waiting {
  decisions: number;
  vote: boolean;
}

/** Places with something waiting: decisions whose scene is there, and the chamber when a vote is yours to cast. */
function waitingAt(pocket: PocketView | null): Map<string, Waiting> {
  const out = new Map<string, Waiting>();
  if (!pocket) return out;
  const get = (k: string) => {
    let w = out.get(k);
    if (!w) out.set(k, (w = { decisions: 0, vote: false }));
    return w;
  };
  for (const d of pocket.decisions) {
    const at = d.scene.place?.location;
    if (d.status === "pending" && at) get(keyOf(at)).decisions++;
  }
  if (pocket.bills.some((b) => b.can_vote)) {
    const to = pocket.routes.find((r) => r.action === "vote" && !r.late_night)?.to;
    if (to) get(keyOf(to.location)).vote = true;
  }
  return out;
}

function waitingText(tr: Translator, w: Waiting | undefined): string | null {
  if (!w) return null;
  const parts: string[] = [];
  if (w.decisions > 0) parts.push(tr.t("ui-town-map-waiting", { what: tr.t("ui-decision-count", { n: w.decisions }) }));
  if (w.vote) parts.push(tr.t("ui-town-map-vote"));
  return parts.join(" · ") || null;
}

// ---------------------------------------------------------------------------

function hotspotName(tr: Translator, h: MapHotspot): string {
  if (h.kind === "streets") return tr.t("ui-streets-of", { hood: h.neighborhoodName });
  if (h.kind === "home") return tr.t("place-home");
  return placeName(tr, { location: h.location, kind_key: h.kindKey, name: h.name, neighborhood: h.neighborhood, neighborhood_name: h.neighborhoodName });
}

interface Probe {
  timings: BuiltTownMap["timings"];
  px: number;
  size: { width: number; height: number };
  view: TownMapView | null;
  /** Plan (in the town worker, uncached), dress, build and bake another town (measurements), ms. */
  bench(input: TownInput, geo: TownGeo, seed: string): Promise<TownBench>;
}

declare global {
  interface Window {
    __townMap?: Probe;
  }
}

/** Timings of one town through the game's path, ms (the `bench` probe). */
export interface TownBench {
  /** Cold: planned in the town worker (its own time, and the wait on the main thread for it). */
  planMs: number;
  coldMs: number;
  /** Warm: the same plan from memory, and from IndexedDB in a new cache (a reload). */
  memoryMs: number;
  storedMs: number;
  storedFrom: string;
  /** Main-thread work after the plan: dress, the map's scene and its bake. */
  dressMs: number;
  sceneMs: number;
  bakeMs: number;
  width: number;
  height: number;
}

/** Plan (cold, then warm from memory and IndexedDB), dress, build and bake a town as the game does, timed. */
async function bench(input: TownInput, geo: TownGeo, seed: string): Promise<TownBench> {
  const req = { runSeed: `bench-${seed}`, town: geo.municipality, contentHash: "bench", geo: { ...geo, places: input.places.map((p) => ({ ...p })) }, seed: townSeed(seed, input.municipality), recipe: await liveRecipe() };
  // A generator id of its own, so a bench never reads a plan an earlier run stored.
  const generator = `bench-${performance.now()}`;
  const planner = (g: TownGeo, s: number, r: typeof req.recipe) => planInWorker(townJobs(), g, s, r);
  const store = new IdbPlanStore();
  const cold = await new TownPlanCache(planner, store, generator).get(req);
  // Let the background write land before reading it back.
  await new Promise((r) => setTimeout(r, 50));
  const cache = new TownPlanCache(planner, store, generator);
  const stored = await cache.get(req);
  const memory = await cache.get(req);
  const t0 = performance.now();
  const map = dressTown(stored.plan, { home: input.home, location: { type: "home" } });
  const t1 = performance.now();
  const b = buildTownMap(map, viewForInput(input));
  return { planMs: cold.planMs ?? 0, coldMs: cold.ms, memoryMs: memory.ms, storedMs: stored.ms, storedFrom: stored.source, dressMs: t1 - t0, sceneMs: b.timings.sceneMs, bakeMs: b.timings.bakeMs, width: map.width, height: map.height };
}

export interface TownMapProps {
  town: TownPlacesView;
  /** The dressed layout v2 map (../hub/gameTown). */
  map: TownMapModel;
  /** A place with something to do was picked (tap, click, Enter). */
  onPick: (h: MapHotspot) => void;
}

export function TownMap({ town, map, onPick }: TownMapProps) {
  const tr = useGame((s) => s.tr);
  const pocket = useGame((s) => s.pocket);
  const busy = useGame((s) => s.busy);
  const walking = useGame((s) => s.walking);
  const passage = useGame((s) => s.passage?.phase ?? null);
  const built = townMapFor(map, town);
  const { scene, baked } = built;
  const wrapRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const layerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<TownMapView | null>(null);
  const [quiet, setQuiet] = useState<string | null>(null);

  const here = useMemo(() => hotspotOf(scene.hotspots, pocket?.location.location ?? town.location)?.key ?? null, [scene, pocket, town.location]);
  const waiting = useMemo(() => waitingAt(pocket), [pocket]);
  // The turn's time of day, eased forward when a slot is spent; night and the new morning when a turn ends.
  const minutes = shownMinutes(pocket, passage);
  const easeMs = passage ? PASSAGE_EASE_MS : LIGHT_EASE_MS;

  // The latest activation handler, read by the view (created once per town).
  const activate = useRef<(h: MapHotspot) => void>(() => {});
  useEffect(() => {
    activate.current = (h) => {
      if (busy) return;
      if (!h.active) {
        setQuiet(tr.t("ui-town-map-quiet", { place: hotspotName(tr, h) }));
        return;
      }
      setQuiet(null);
      onPick(h);
    };
  });

  useEffect(() => {
    const wrap = wrapRef.current;
    const canvas = canvasRef.current;
    const layer = layerRef.current;
    if (!wrap || !canvas || !layer) return;
    const reducedMotion = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
    const view = new TownMapView(wrap, canvas, layer, { scene, baked, reducedMotion, onActivate: (h) => activate.current(h) });
    viewRef.current = view;
    window.__townMap = {
      timings: built.timings,
      px: baked.px,
      size: { width: baked.width, height: baked.height },
      view,
      bench,
    };
    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, [scene, baked, built.timings]);

  useLayoutEffect(() => {
    viewRef.current?.syncElements();
  });

  useEffect(() => {
    viewRef.current?.setOverlay({ here, waiting: new Set(waiting.keys()), minutes }, easeMs);
  }, [here, waiting, minutes, easeMs]);

  // An auto-routed action (the agenda) walks the player on the map too.
  useEffect(() => {
    if (!walking) return;
    const h = hotspotOf(scene.hotspots, walking.location);
    if (h) viewRef.current?.walkTo(h, WALK_MAX_MS);
  }, [walking, scene]);

  const label = tr.t("ui-town-map-label", { town: town.municipality, n: scene.hotspots.filter((h) => h.active).length });
  return (
    <div className="town-map" data-testid="town-map">
      <div
        ref={wrapRef}
        className="town-map-wrap"
        role="group"
        aria-roledescription={tr.t("ui-town-map")}
        aria-label={town.municipality}
        aria-describedby="town-map-help"
      >
        <canvas ref={canvasRef} className="town-map-canvas" role="img" aria-label={label} data-testid="town-map-canvas" />
        <div ref={layerRef} className="town-map-layer">
          {scene.hotspots.map((h) => {
            const name = hotspotName(tr, h);
            const isHere = h.key === here;
            const w = waitingText(tr, waiting.get(h.key));
            const kind = h.kind === "place" && h.name ? tr.t(`place-${h.kindKey}`) : null;
            const aria = [name, kind, h.kind !== "streets" ? h.neighborhoodName : null, isHere ? tr.t("ui-you-are-here") : null, w, h.active ? null : tr.t("ui-nothing-here")].filter(Boolean).join(", ");
            return (
              <button
                key={h.key}
                type="button"
                className={`town-map-hit${h.active ? "" : " town-map-hit-quiet"}`}
                data-hotspot={h.key}
                data-name={name}
                data-testid={`map-${h.kind === "place" ? h.kindKey : h.kind === "home" ? "home" : `streets-${h.neighborhood}`}`}
                aria-label={aria}
                aria-current={isHere ? "location" : undefined}
                aria-disabled={busy || undefined}
              />
            );
          })}
          {scene.hotspots.map((h) => {
            const isHere = h.key === here;
            const w = waitingText(tr, waiting.get(h.key));
            const cls = ["town-map-chip", `town-map-chip-${h.kind}`, h.active ? "" : "town-map-chip-quiet", isHere ? "town-map-chip-here" : "", w ? "town-map-chip-waiting" : "", h.landmark ? "town-map-chip-landmark" : ""].filter(Boolean).join(" ");
            return (
              <span key={h.key} className={cls} data-chip={h.key} data-shown="0" aria-hidden="true">
                {h.kind !== "streets" && <span className="town-map-chip-icon">{placeIcon(h.kindKey)}</span>}
                <span className="town-map-chip-text">
                  <span className="town-map-chip-name">{h.kind === "streets" ? h.neighborhoodName : hotspotName(tr, h)}</span>
                  {isHere && <span className="town-map-chip-flag">📍 {tr.t("ui-you-are-here")}</span>}
                  {w && <span className="town-map-chip-flag town-map-chip-flag-waiting">❗ {w}</span>}
                </span>
              </span>
            );
          })}
        </div>
        <div className="town-map-tools" data-map-control="">
          <button type="button" className="btn btn-sm town-map-tool" onClick={() => viewRef.current?.zoomBy(1.5)} aria-label={tr.t("ui-town-map-zoom-in")} title={tr.t("ui-town-map-zoom-in")}>
            +
          </button>
          <button type="button" className="btn btn-sm town-map-tool" onClick={() => viewRef.current?.zoomBy(1 / 1.5)} aria-label={tr.t("ui-town-map-zoom-out")} title={tr.t("ui-town-map-zoom-out")}>
            −
          </button>
          <button type="button" className="btn btn-sm town-map-tool" onClick={() => viewRef.current?.fit()} aria-label={tr.t("ui-town-map-fit")} title={tr.t("ui-town-map-fit")} data-testid="town-map-fit">
            ⤢
          </button>
        </div>
      </div>
      <p className="town-map-status small" role="status" data-testid="town-map-status">
        {quiet}
      </p>
      <p id="town-map-help" className="muted small town-map-help">
        {tr.t("ui-town-map-help")}
      </p>
    </div>
  );
}
