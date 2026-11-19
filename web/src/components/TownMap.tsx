// The town map overview (DEC-086): the town screen's main view, an
// illustrated low-resolution map drawn from the same versioned layout as the
// walkable street (DEC-085). Tapping a building does what the list's place
// buttons do (`visit`: go there, free, and open its panel; DEC-079), and the
// player is shown walking there along the streets. Every hotspot is a real
// button (keyboard, screen readers), and the List view stays the text
// alternative with exactly the same places (DEC-051). Presentation only: the
// engine says what a place offers, where the player is and what is waiting.
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { LocationDto } from "../engine/protocol/LocationDto";
import type { PocketView } from "../engine/protocol/PocketView";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";
import { clockForDayPart } from "../hub/dayCycle";
import { bakeTownMap, type BakedTownMap } from "../hub/render/townMapRenderer";
import { townInputFromView, type TownInput } from "../hub/townInput";
import { LAYOUT_VERSION, layoutTown, townSeed } from "../hub/townLayout";
import { buildTownMapScene, hotspotOf, viewForInput, type MapHotspot, type TownMapScene } from "../hub/townMapScene";
import { TownMapView, WALK_MAX_MS } from "../hub/townMapView";
import type { Translator } from "../i18n/fluent";
import { placeIcon, placeName } from "../places/labels";
import { useGame } from "../state/gameStore";
import { say } from "./townMapText";

// ---------------------------------------------------------------------------
// The layout version a run's town is drawn with (DEC-085): recorded when the
// run first shows its town, so an app update never rearranges it.

const versionKey = (runId: string) => `mandate.townLayoutVersion.${runId}`;

export function layoutVersionFor(runId: string | null): number {
  if (!runId) return LAYOUT_VERSION;
  try {
    const v = Number(localStorage.getItem(versionKey(runId)));
    if (Number.isInteger(v) && v >= 1 && v <= LAYOUT_VERSION) return v;
    localStorage.setItem(versionKey(runId), String(LAYOUT_VERSION));
  } catch {
    // Storage blocked (private mode): the current version.
  }
  return LAYOUT_VERSION;
}

// ---------------------------------------------------------------------------
// Layout, scene and bake, made once per town and version (moving around does not change them)

export interface BuiltTownMap {
  key: string;
  scene: TownMapScene;
  baked: BakedTownMap;
  /** ms: layout (DEC-085), scene (hotspots, main routes), bake, and the bake's stages. */
  timings: { layoutMs: number; sceneMs: number; bakeMs: number } & BakedTownMap["stages"];
}

/** What the map depends on: the town's neighbourhoods and places, not where the player is. */
function townKey(town: TownPlacesView, seed: string, version: number): string {
  const parts = [seed, version, town.municipality, town.home_neighborhood ?? "-"];
  for (const h of town.neighborhoods) {
    parts.push(`${h.id}:${h.kind}:${h.name}`);
    for (const s of h.spots) parts.push(`${JSON.stringify(s.location.location)}${s.location.kind_key}${s.location.name}${s.actions.length + s.services.length > 0 ? "+" : "-"}`);
  }
  return parts.join("|");
}

export function buildTownMap(input: TownInput, view: TownPlacesView, seed: string, version: number): Omit<BuiltTownMap, "key"> {
  const t0 = performance.now();
  const map = layoutTown(input, townSeed(seed, view.municipality), version);
  const t1 = performance.now();
  const scene = buildTownMapScene(map, view);
  const t2 = performance.now();
  const baked = bakeTownMap(scene);
  const t3 = performance.now();
  return { scene, baked, timings: { layoutMs: t1 - t0, sceneMs: t2 - t1, bakeMs: t3 - t2, ...baked.stages } };
}

let cached: BuiltTownMap | null = null;
function townMapFor(town: TownPlacesView, seed: string, version: number): BuiltTownMap {
  const key = townKey(town, seed, version);
  if (cached?.key !== key) cached = { key, ...buildTownMap(townInputFromView(town), town, seed, version) };
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
  if (w.decisions > 0) parts.push(say(tr, "ui-town-map-waiting", { what: tr.t("ui-decision-count", { n: w.decisions }) }));
  if (w.vote) parts.push(say(tr, "ui-town-map-vote"));
  return parts.join(" · ") || null;
}

/** A gentle time of day from the pocket: morning to evening as the turn's slots are used, then late night. */
function minutesOf(p: PocketView | null): number | null {
  if (!p) return null;
  if (p.slots_left <= 0) return clockForDayPart(p.late_night_available ? "evening" : "late_night", p.late_night_available ? 0.75 : 0.2);
  const used = Math.max(0, p.slots_total - p.slots_left) / Math.max(1, p.slots_total);
  return used < 0.5 ? clockForDayPart("morning", 0.35 + used) : clockForDayPart("afternoon", (used - 0.5) * 1.6);
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
  /** Lay out, build and bake another town (measurements), ms. */
  bench(input: TownInput, seed: string): BuiltTownMap["timings"] & { px: number; width: number; height: number };
}

declare global {
  interface Window {
    __townMap?: Probe;
  }
}

export function TownMap({ town }: { town: TownPlacesView }) {
  const tr = useGame((s) => s.tr);
  const pocket = useGame((s) => s.pocket);
  const seed = useGame((s) => s.overview?.seed ?? "");
  const runId = useGame((s) => s.run?.runId ?? null);
  const busy = useGame((s) => s.busy);
  const visit = useGame((s) => s.visit);
  const walking = useGame((s) => s.walking);
  const version = useMemo(() => layoutVersionFor(runId), [runId]);
  const built = townMapFor(town, seed, version);
  const { scene, baked } = built;
  const wrapRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const layerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<TownMapView | null>(null);
  const [quiet, setQuiet] = useState<string | null>(null);

  const here = useMemo(() => hotspotOf(scene.hotspots, pocket?.location.location ?? town.location)?.key ?? null, [scene, pocket, town.location]);
  const waiting = useMemo(() => waitingAt(pocket), [pocket]);
  const minutes = minutesOf(pocket);

  // The latest activation handler, read by the view (created once per town).
  const activate = useRef<(h: MapHotspot) => void>(() => {});
  useEffect(() => {
    activate.current = (h) => {
      if (busy) return;
      if (!h.active) {
        setQuiet(say(tr, "ui-town-map-quiet", { place: hotspotName(tr, h) }));
        return;
      }
      setQuiet(null);
      viewRef.current?.walkTo(h);
      void visit(h.location);
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
      bench(input, s) {
        const b = buildTownMap(input, viewForInput(input), s, LAYOUT_VERSION);
        return { ...b.timings, px: b.baked.px, width: b.baked.width, height: b.baked.height };
      },
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
    viewRef.current?.setOverlay({ here, waiting: new Set(waiting.keys()), minutes });
  }, [here, waiting, minutes]);

  // An auto-routed action (the agenda) walks the player on the map too.
  useEffect(() => {
    if (!walking) return;
    const h = hotspotOf(scene.hotspots, walking.location);
    if (h) viewRef.current?.walkTo(h, WALK_MAX_MS);
  }, [walking, scene]);

  const label = say(tr, "ui-town-map-label", { town: town.municipality, n: scene.hotspots.filter((h) => h.active).length });
  return (
    <div className="town-map" data-testid="town-map">
      <div
        ref={wrapRef}
        className="town-map-wrap"
        style={{ aspectRatio: `${baked.width} / ${Math.round(baked.height * 1.08)}` }}
        role="group"
        aria-roledescription={say(tr, "ui-town-view-map")}
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
          <button type="button" className="btn btn-sm town-map-tool" onClick={() => viewRef.current?.zoomBy(1.5)} aria-label={say(tr, "ui-town-map-zoom-in")} title={say(tr, "ui-town-map-zoom-in")}>
            +
          </button>
          <button type="button" className="btn btn-sm town-map-tool" onClick={() => viewRef.current?.zoomBy(1 / 1.5)} aria-label={say(tr, "ui-town-map-zoom-out")} title={say(tr, "ui-town-map-zoom-out")}>
            −
          </button>
          <button type="button" className="btn btn-sm town-map-tool" onClick={() => viewRef.current?.fit()} aria-label={say(tr, "ui-town-map-fit")} title={say(tr, "ui-town-map-fit")} data-testid="town-map-fit">
            ⤢
          </button>
        </div>
      </div>
      <p className="town-map-status small" role="status" data-testid="town-map-status">
        {quiet}
      </p>
      <p id="town-map-help" className="muted small town-map-help">
        {say(tr, "ui-town-map-help")}
      </p>
    </div>
  );
}
