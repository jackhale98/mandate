// The town map overview (DEC-086): an illustrated low-resolution map drawn
// from the same versioned layout as the walkable street (DEC-085), shown full
// screen over the street (DEC-088 main screen). Picking a building hands it
// to `onPick` (the town screen closes the map and walks the player there in
// the street). Every hotspot is a real button (keyboard, screen readers), and
// the pocket's Places tab stays the text alternative with exactly the same
// places (DEC-051). Presentation only: the
// engine says what a place offers, where the player is and what is waiting.
//
// Layers (M6 note 11; DEC-089's pattern): a picker lists the engine's town
// map layers (`town_map_layers`: districts and who holds each seat, people
// where the player knows to find them, what each neighbourhood worries
// about), each drawn over the map (./townmap/TownLayerOverlay), read in a
// legend under it and listed as text by "List" (DEC-051). The layer chosen is
// remembered per device, as the country map's is. Everything a layer shows
// is the engine's view; this only draws it.
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { engine, expectKind } from "../engine/client";
import type { LocationDto } from "../engine/protocol/LocationDto";
import type { PocketView } from "../engine/protocol/PocketView";
import type { TownAreaView } from "../engine/protocol/TownAreaView";
import type { TownMapLayersView } from "../engine/protocol/TownMapLayersView";
import type { TownMarkerView } from "../engine/protocol/TownMarkerView";
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
import { hoodLabel, placeIcon, placeName } from "../places/labels";
import { useGame } from "../state/gameStore";
import { TownLayerList } from "./townmap/TownLayerList";
import { TownLayerOverlay, type LayerFocus } from "./townmap/TownLayerOverlay";
import { LayerLegend, PeopleList, PersonCard } from "./townmap/TownLayerPanel";
import { NO_LAYER, chosenOption, keptBy, layerKey, partyName, placeMarkers, readLayer, writeLayer } from "./townmap/layerModel";
import "./townmap/townLayers.css";

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
    for (const s of h.spots) parts.push(`${JSON.stringify(s.location.location)}${s.location.kind_key}${s.location.name}${s.actions.length + s.services.length + s.verbs.length > 0 ? "+" : "-"}`);
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
  if (h.kind === "streets") return tr.t("ui-streets-of", { hood: hotspotHood(tr, h) });
  if (h.kind === "home") return tr.t("place-home");
  return placeName(tr, { location: h.location, kind_key: h.kindKey, name: h.name, neighborhood: h.neighborhood, neighborhood_name: h.neighborhoodName, ...(h.neighborhoodKey ? { neighborhood_key: h.neighborhoodKey, neighborhood_args: h.neighborhoodArgs ?? {} } : {}) });
}

/** The hotspot's neighbourhood as the Places tab names it (the centre reads as the town centre). */
function hotspotHood(tr: Translator, h: MapHotspot): string {
  return hoodLabel(tr, h.neighborhoodName, h.neighborhoodKey, h.neighborhoodArgs);
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
  const setOpen = useGame((s) => s.setTownMapOpen);
  const openCharacter = useGame((s) => s.openCharacter);
  const openTalk = useGame((s) => s.openTalk);
  const visit = useGame((s) => s.visit);
  // The layers, as the engine serves them: fetched when the map opens and after every command (the pocket is new then).
  const [layers, setLayers] = useState<TownMapLayersView | null>(null);
  const [chosen, setChosen] = useState<string>(readLayer);
  const [params, setParams] = useState<Record<string, string>>({});
  const [filterId, setFilterId] = useState("all");
  const [list, setList] = useState(false);
  const [focus, setFocus] = useState<LayerFocus>(null);
  // What the view calls after each frame: the overlay's placing while a layer is drawn.
  const frame = useRef<() => void>(() => {});
  useEffect(() => {
    let live = true;
    engine()
      .request({ kind: "query", payload: { type: "town_map_layers" } })
      .then((b) => live && setLayers(expectKind(b, "town_map_layers")))
      .catch(() => {
        // The map still works without layers; the picker offers only the plain map.
      });
    return () => {
      live = false;
    };
  }, [pocket]);
  const layer = layers?.layers.find((l) => layerKey(l) === chosen) ?? null;
  const data = layer?.data ?? null;
  const option = data?.kind === "shaded" ? chosenOption(data.options, layer ? params[layer.id] : undefined) : null;
  const filter = data?.kind === "markers" && data.filters.some((f) => f.id === filterId) ? filterId : "all";
  // The map's own name chips stay where they help (the plain map, and people at places).
  const chips = !data || data.kind === "markers";

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
    const view = new TownMapView(wrap, canvas, layer, { scene, baked, reducedMotion, onActivate: (h) => activate.current(h), onFrame: () => frame.current() });
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

  // The list covers the map: the keyboard and screen readers skip what is under it.
  useEffect(() => {
    if (wrapRef.current) wrapRef.current.inert = list && layer !== null;
  }, [list, layer]);

  // --- Layers: what is being read, and what a tap on a mark does.
  const choose = (key: string) => {
    setChosen(key);
    writeLayer(key);
    setFocus(null);
    if (key === NO_LAYER) setList(false);
  };
  const spotName = (key: string | null) => {
    const h = key === null ? undefined : scene.hotspots.find((x) => x.key === key);
    return h ? hotspotName(tr, h) : "";
  };
  const marker = (id: number): TownMarkerView | null => {
    if (data?.kind === "boundaries") return data.areas.find((a) => a.holder?.person.id === id)?.holder ?? null;
    if (data?.kind === "markers") return data.markers.find((m) => m.person.id === id) ?? null;
    return null;
  };
  const person = focus?.type === "person" ? marker(focus.id) : null;
  const group = focus?.type === "people" && data?.kind === "markers" ? data.markers.filter((m) => m.person.id !== null && focus.ids.includes(m.person.id)) : null;
  const area: TownAreaView | null = focus?.type === "area" && data?.kind === "boundaries" ? (data.areas.find((a) => a.number === focus.number) ?? null) : null;
  const unplaced = data?.kind === "markers" ? placeMarkers(scene.hotspots, keptBy(data.markers, filter)).unplaced : [];
  const onPerson = (m: TownMarkerView) => {
    if (m.person.id !== null) setFocus({ type: "person", id: m.person.id });
  };
  // Their full card is a sheet of the main screen: the map closes for it.
  const openCard = (m: TownMarkerView) => {
    if (m.person.id === null) return;
    setOpen(false);
    void openCharacter(m.person.id);
  };
  // Go to them: where the engine's talk view says they can be found now (auto-routed, DEC-079), walked in the street as a tapped place is.
  const goTo = async (m: TownMarkerView) => {
    if (m.person.id === null) return;
    const to = { type: "person" as const, person: m.person.id };
    try {
      const v = expectKind(await engine().request({ kind: "query", payload: { type: "talk", to, late_night: false } }), "talk");
      const at = v.route?.to?.location;
      if (at) {
        const h = hotspotOf(scene.hotspots, at);
        if (h) return onPick(h);
        setOpen(false);
        return void visit(at);
      }
    } catch {
      // The talk panel says where they are, or why not.
    }
    setOpen(false);
    await openTalk(to);
  };

  const label = tr.t("ui-town-map-label", { town: town.municipality, n: scene.hotspots.filter((h) => h.active).length });
  return (
    <div className="town-map" data-testid="town-map" data-layer={layer ? layerKey(layer) : NO_LAYER} data-layers={layers ? layers.layers.length : undefined}>
      <div className="tml-bar" data-testid="town-map-bar">
        <label className="tml-pick">
          <span className="muted small map-layer-label">{tr.t("ui-overlay")}</span>
          <select value={layer ? chosen : NO_LAYER} onChange={(e) => choose(e.target.value)} data-testid="town-map-layer" aria-label={tr.t("ui-overlay")}>
            <option value={NO_LAYER}>{tr.t("ui-town-map-layer-none")}</option>
            {layers?.layers.map((l) => (
              <option key={l.id} value={layerKey(l)}>
                {tr.t(l.label_key, l.args)}
              </option>
            ))}
          </select>
        </label>
        {layer && data?.kind === "shaded" && data.options.length > 1 && (
          <label className="tml-pick">
            <select
              value={option?.id ?? ""}
              onChange={(e) => {
                setParams({ ...params, [layer.id]: e.target.value });
                setFocus(null);
              }}
              data-testid="town-map-param"
              aria-label={data.param_label_key ? tr.t(data.param_label_key) : tr.t("ui-overlay")}
            >
              {data.options.map((o) => (
                <option key={o.id} value={o.id}>
                  {tr.t(o.label_key)}
                </option>
              ))}
            </select>
          </label>
        )}
        {layer && data?.kind === "markers" && data.filters.length > 1 && (
          <label className="tml-pick">
            <select
              value={filter}
              onChange={(e) => {
                setFilterId(e.target.value);
                setFocus(null);
              }}
              data-testid="town-map-filter"
              aria-label={tr.t("ui-town-map-filter")}
            >
              {data.filters.map((f) => (
                <option key={f.id} value={f.id}>
                  {f.label_key ? tr.t(f.label_key) : (partyName(layers?.parties ?? [], f.party) ?? "")} ({f.count})
                </option>
              ))}
            </select>
          </label>
        )}
        {layer && (
          <button
            type="button"
            className={list ? "btn btn-sm map-list-toggle on" : "btn btn-sm btn-ghost map-list-toggle"}
            aria-pressed={list}
            onClick={() => setList(!list)}
            title={tr.t(list ? "ui-map-list-hide" : "ui-town-map-list-show")}
            data-testid="town-map-list-toggle"
          >
            <span aria-hidden="true">☰ </span>
            {tr.t("ui-map-list")}
          </button>
        )}
      </div>
      <div className="tml-stage">
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
            const aria = [name, kind, h.kind !== "streets" ? hotspotHood(tr, h) : null, isHere ? tr.t("ui-you-are-here") : null, w, h.active ? null : tr.t("ui-nothing-here")].filter(Boolean).join(", ");
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
          {(chips ? scene.hotspots : []).map((h) => {
            const isHere = h.key === here;
            const w = waitingText(tr, waiting.get(h.key));
            const cls = ["town-map-chip", `town-map-chip-${h.kind}`, h.active ? "" : "town-map-chip-quiet", isHere ? "town-map-chip-here" : "", w ? "town-map-chip-waiting" : "", h.landmark ? "town-map-chip-landmark" : ""].filter(Boolean).join(" ");
            return (
              <span key={h.key} className={cls} data-chip={h.key} data-shown="0" aria-hidden="true">
                {h.kind !== "streets" && <span className="town-map-chip-icon">{placeIcon(h.kindKey)}</span>}
                <span className="town-map-chip-text">
                  <span className="town-map-chip-name">{h.kind === "streets" ? hotspotHood(tr, h) : hotspotName(tr, h)}</span>
                  {isHere && <span className="town-map-chip-flag">📍 {tr.t("ui-you-are-here")}</span>}
                  {w && <span className="town-map-chip-flag town-map-chip-flag-waiting">❗ {w}</span>}
                </span>
              </span>
            );
          })}
        </div>
        {layer && layers && (
          <TownLayerOverlay
            tr={tr}
            map={map}
            scene={scene}
            view={layers}
            layer={layer}
            option={option}
            filter={filter}
            focus={focus}
            spotName={spotName}
            getView={() => viewRef.current}
            frameRef={frame}
            onPerson={onPerson}
            onCluster={(c) => setFocus({ type: "people", key: c.key, title: tr.t("ui-town-map-cluster-title", { place: spotName(c.spot ?? c.members[0]?.spot ?? null) }), ids: c.members.flatMap((p) => (p.marker.person.id === null ? [] : [p.marker.person.id])) })}
            onHood={(id) => setFocus({ type: "hood", id })}
            onArea={(a) => setFocus({ type: "area", number: a.number })}
          />
        )}
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
        {list && layer && layers && <TownLayerList tr={tr} view={layers} layer={layer} option={option} filter={filter} onPerson={onPerson} />}
        <p className="town-map-status small" role="status" data-testid="town-map-status">
          {quiet}
        </p>
      </div>
      {layer && layers && (
        <div className="tml-panel" data-testid="town-map-panel">
          {person && <PersonCard tr={tr} m={person} view={layers} busy={busy} onClose={() => setFocus(null)} onOpenCard={openCard} onGo={(m) => void goTo(m)} />}
          {group && focus?.type === "people" && <PeopleList tr={tr} title={focus.title} people={group} view={layers} onPerson={onPerson} onClose={() => setFocus(null)} />}
          <LayerLegend
            tr={tr}
            view={layers}
            layer={layer}
            option={option}
            hood={focus?.type === "hood" ? focus.id : null}
            area={area}
            unplaced={unplaced.length}
            onUnplaced={() => setFocus({ type: "people", key: "unplaced", title: tr.t("ui-town-map-unplaced"), ids: unplaced.flatMap((m) => (m.person.id === null ? [] : [m.person.id])) })}
            onPerson={onPerson}
          />
        </div>
      )}
      <p id="town-map-help" className="muted small town-map-help">
        {tr.t("ui-town-map-help")}
      </p>
    </div>
  );
}
