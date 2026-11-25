// A town map layer drawn over the map (DEC-086, M6 note 11): a fill a tile to
// a pixel (each neighbourhood's class, or its district's party), outlines as
// vector lines, and marks as HTML (district names, portraits, each
// neighbourhood's reading), all following the map's camera frame by frame
// (`TownMapView`'s `onFrame`). Every mark that does something is a real
// button (keyboard, screen readers); the fill and the lines are decoration
// for what the marks, the legend and the list say in words (DEC-051).
// Presentation only: what is shown comes from the engine (./layerModel).
import { useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type MutableRefObject } from "react";
import type { TownAreaView } from "../../engine/protocol/TownAreaView";
import type { TownMapLayerView } from "../../engine/protocol/TownMapLayerView";
import type { TownMapLayersView } from "../../engine/protocol/TownMapLayersView";
import type { TownMarkerView } from "../../engine/protocol/TownMarkerView";
import type { TownShadeOptionView } from "../../engine/protocol/TownShadeOptionView";
import type { TownMap } from "../../hub/townLayout";
import type { TownMapScene } from "../../hub/townMapScene";
import type { TownMapView } from "../../hub/townMapView";
import type { Translator } from "../../i18n/fluent";
import { personName } from "../Knowing";
import { roleText } from "../CharacterSheet";
import { Portrait } from "../portrait/Portrait";
import { lookVariant, personKey } from "../portrait/look";
import {
  UNKNOWN_RGB,
  areaAnchor,
  areaAnchorHood,
  areaName,
  clusterMarkers,
  fillPixels,
  hoodAnchor,
  hoodName,
  keptBy,
  outlinePath,
  parseColour,
  partyCss,
  placeMarkers,
  shadeColours,
  type HoodFill,
  type MarkerCluster,
} from "./layerModel";

/** A portrait mark's size, CSS px (the reach within which two share one mark). */
export const FACE = 38;

/** What the panel under the map reads: a person, a group sharing one mark (or with no known place), a neighbourhood, a district. */
export type LayerFocus = { type: "person"; id: number } | { type: "people"; key: string; title: string; ids: number[] } | { type: "hood"; id: number } | { type: "area"; number: number } | null;

interface Props {
  tr: Translator;
  map: TownMap;
  scene: TownMapScene;
  view: TownMapLayersView;
  layer: TownMapLayerView;
  /** The shaded layer's choice. */
  option: TownShadeOptionView | null;
  /** The markers layer's filter. */
  filter: string;
  focus: LayerFocus;
  /** A hotspot's name by its key (where a group of people stands). */
  spotName: (key: string | null) => string;
  getView: () => TownMapView | null;
  /** Called by the map after every frame it draws: set here to this overlay's placing. */
  frameRef: MutableRefObject<() => void>;
  onPerson: (m: TownMarkerView) => void;
  onCluster: (c: MarkerCluster) => void;
  onHood: (id: number) => void;
  onArea: (a: TownAreaView) => void;
}

/** A CSS colour (maybe `var(--x)`) as RGB, read from the page's styles. */
function resolveRgb(css: string): [number, number, number] {
  const m = /^var\((--[a-z0-9-]+)\)$/i.exec(css.trim());
  if (!m || typeof getComputedStyle !== "function") return parseColour(css);
  return parseColour(getComputedStyle(document.documentElement).getPropertyValue(m[1] as string));
}

export function markerName(tr: Translator, m: TownMarkerView): string {
  return m.is_player ? tr.t("ui-you") : personName(tr, m.person.name);
}

function Face({ m, parties, small = false }: { m: TownMarkerView; parties: TownMapLayersView["parties"]; small?: boolean }) {
  const key = personKey(m.person);
  return (
    <span className={small ? "tml-face tml-face-sm" : "tml-face"} style={{ "--ring": partyCss(parties, m.party) } as CSSProperties}>
      <Portrait variant={key ? lookVariant(key) : 0} player={m.is_player} className="tml-portrait" />
    </span>
  );
}
export { Face as MarkerFace };

export function TownLayerOverlay({ tr, map, scene, view, layer, option, filter, focus, spotName, getView, frameRef, onPerson, onCluster, onHood, onArea }: Props) {
  const root = useRef<HTMLDivElement>(null);
  const fill = useRef<HTMLCanvasElement>(null);
  const lines = useRef<SVGGElement>(null);
  // The zoom the clusters were made for (re-made when it changes by about a tenth).
  const [zoom, setZoom] = useState(() => getView()?.camera.state.zoom ?? 4);
  const zoomRef = useRef(zoom);
  const data = layer.data;

  // --- What is drawn: fills per neighbourhood, outlines, and the marks' anchors.
  const fills = useMemo(() => {
    const out = new Map<number, HoodFill>();
    if (data.kind === "boundaries") {
      for (const a of data.areas) {
        if (!a.holder) continue;
        const [r, g, b] = resolveRgb(partyCss(view.parties, a.holder.party));
        for (const h of a.hoods) out.set(h, { rgba: [r, g, b, 0.36] });
      }
    } else if (data.kind === "shaded" && option) {
      const colours = shadeColours(option);
      view.hoods.forEach((h, i) => {
        const cls = option.cells[i]?.class ?? null;
        if (cls === null) out.set(h.id, { rgba: [UNKNOWN_RGB[0], UNKNOWN_RGB[1], UNKNOWN_RGB[2], 0.5], hatch: true });
        else {
          const [r, g, b] = parseColour(colours[cls] ?? "#888888");
          out.set(h.id, { rgba: [r, g, b, 0.66] });
        }
      });
    }
    return out;
  }, [data, option, view.hoods, view.parties]);

  const outlines = useMemo(() => {
    if (data.kind === "boundaries") return data.areas.map((a) => ({ key: `a${a.number}`, d: outlinePath(map, a.hoods), strong: a.yours || (a.home && !data.areas.some((x) => x.yours)) }));
    if (data.kind === "shaded") return view.hoods.map((h) => ({ key: `h${h.id}`, d: outlinePath(map, [h.id]), strong: false }));
    return [];
  }, [data, map, view.hoods]);

  const people = useMemo(() => (data.kind === "markers" ? placeMarkers(scene.hotspots, keptBy(data.markers, filter)) : null), [data, scene, filter]);
  const clusters = useMemo(() => (people ? clusterMarkers(people.placed, zoom, FACE) : []), [people, zoom]);

  // --- Paint the fill (a tile to a pixel; the browser scales it, pixelated).
  useEffect(() => {
    const c = fill.current;
    const ctx = c?.getContext("2d");
    if (!c || !ctx) return;
    const img = ctx.createImageData(map.width, map.height);
    img.data.set(fillPixels(map, fills));
    ctx.putImageData(img, 0, 0);
  }, [map, fills]);

  // --- Follow the camera: the fill and the lines by one transform; the marks one by one, hiding those that would overlap.
  const place = () => {
    const v = getView();
    const el = root.current;
    if (!v || !el) return;
    const s = v.camera.state;
    const vw = v.camera.viewport.width;
    const vh = v.camera.viewport.height;
    const tx = vw / 2 - s.x * s.zoom;
    const ty = vh / 2 - s.y * s.zoom;
    if (fill.current) fill.current.style.transform = `translate(${tx}px, ${ty}px) scale(${s.zoom})`;
    lines.current?.setAttribute("transform", `translate(${tx} ${ty}) scale(${s.zoom})`);
    // The zoom buttons' corner is taken.
    const taken: { x: number; y: number; w: number; h: number }[] = [{ x: vw - 56, y: vh - 152, w: 56, h: 152 }];
    const marks = [...el.querySelectorAll<HTMLElement>("[data-wx]")].sort((a, b) => Number(a.dataset.rank) - Number(b.dataset.rank));
    for (const m of marks) {
      const w = m.offsetWidth;
      const h = m.offsetHeight;
      const cx = (Number(m.dataset.wx) - s.x) * s.zoom + vw / 2;
      const cy = (Number(m.dataset.wy) - s.y) * s.zoom + vh / 2 + Number(m.dataset.dy ?? 0);
      // A label of a portrait goes beside it: below, right, left, above, then the corners; the first that is free
      // (one that must stay goes where it covers least).
      const r = Number(m.dataset.around ?? 0);
      const ax = r / 2 + w / 2 + 3;
      const ay = r / 2 + h / 2 + 2;
      const tries: [number, number][] = r > 0 ? [[0, ay], [ax, 0], [-ax, 0], [0, -ay], [ax, ay], [-ax, ay], [ax, -ay], [-ax, -ay]] : [[0, 0]];
      const keep = m.dataset.keep === "1";
      let box: { x: number; y: number; w: number; h: number } | null = null;
      let least = Infinity;
      let free = false;
      for (const [ox, oy] of tries) {
        const b = { x: Math.round(cx + ox - w / 2) - 1, y: Math.round(cy + oy - h / 2) - 1, w: w + 2, h: h + 2 };
        // What runs off the view counts as covered while the thing it names is in view.
        const inView = cx >= 0 && cy >= 0 && cx <= vw && cy <= vh;
        let covered = inView ? b.w * b.h - Math.max(0, Math.min(b.x + b.w, vw) - Math.max(b.x, 0)) * Math.max(0, Math.min(b.y + b.h, vh) - Math.max(b.y, 0)) : 0;
        for (const q of taken) covered += Math.max(0, Math.min(b.x + b.w, q.x + q.w) - Math.max(b.x, q.x)) * Math.max(0, Math.min(b.y + b.h, q.y + q.h) - Math.max(b.y, q.y));
        if (covered < least) {
          least = covered;
          box = b;
        }
        if (covered === 0) {
          free = true;
          break;
        }
      }
      if (!box) continue;
      const show = keep || free;
      m.style.transform = `translate(${box.x + 1}px, ${box.y + 1}px)`;
      if ((m.dataset.shown === "1") !== show) m.dataset.shown = show ? "1" : "0";
      if (show) taken.push(box);
    }
    if (Math.abs(Math.log(s.zoom / zoomRef.current)) > 0.09) {
      zoomRef.current = s.zoom;
      setZoom(s.zoom);
    }
  };
  useLayoutEffect(() => {
    frameRef.current = place;
    place();
    return () => {
      frameRef.current = () => {};
    };
  });

  const focusPerson = focus?.type === "person" ? focus.id : null;
  const face = (m: TownMarkerView, at: { x: number; y: number }, dy: number, key: string) => (
    <button
      key={key}
      type="button"
      className={m.person.id === focusPerson ? "tml-mark tml-person on" : "tml-mark tml-person"}
      data-map-control=""
      data-wx={at.x}
      data-wy={at.y}
      data-dy={dy}
      data-rank="0"
      data-keep="1"
      data-testid="town-map-person"
      data-person={m.person.id ?? undefined}
      data-member={m.seat ? "true" : undefined}
      data-you={m.is_player ? "true" : undefined}
      aria-label={tr.t("ui-town-map-person", { name: markerName(tr, m), role: roleText(tr, m.person) })}
      onClick={() => onPerson(m)}
    >
      <Face m={m} parties={view.parties} />
    </button>
  );

  return (
    <div ref={root} className="tml" data-testid="town-map-layer-overlay" data-kind={data.kind}>
      <canvas ref={fill} className="tml-fill" width={map.width} height={map.height} aria-hidden="true" />
      <svg className="tml-lines" aria-hidden="true">
        <g ref={lines}>
          {[...outlines]
            .sort((a, b) => Number(a.strong) - Number(b.strong))
            .map((o) => (
              <g key={o.key} className={o.strong ? "tml-outline tml-outline-own" : "tml-outline"}>
                <path className="tml-outline-case" d={o.d} vectorEffect="non-scaling-stroke" />
                <path className="tml-outline-line" d={o.d} vectorEffect="non-scaling-stroke" />
              </g>
            ))}
        </g>
      </svg>
      <div className="tml-marks">
        {data.kind === "boundaries" &&
          data.areas.map((a) => {
            const at = areaAnchor(map, a.hoods);
            if (!at) return null;
            const on = focus?.type === "area" && focus.number === a.number;
            return [
              a.holder ? face(a.holder, at, 0, `p${a.number}`) : null,
              <button
                key={`l${a.number}`}
                type="button"
                className={["tml-mark", "tml-area", a.yours ? "tml-area-yours" : "", on ? "on" : ""].filter(Boolean).join(" ")}
                data-map-control=""
                data-wx={at.x}
                data-wy={at.y}
                data-dy={0}
                data-around={a.holder ? FACE : undefined}
                data-rank="1"
                data-keep="1"
                data-testid="town-map-area"
                data-area={a.number}
                data-yours={a.yours ? "true" : undefined}
                data-home={a.home ? "true" : undefined}
                onClick={() => onArea(a)}
              >
                <span className="tml-area-name">{areaName(tr, a)}</span>
                {a.yours && <span className="tml-flag">★ {tr.t("ui-town-map-area-yours")}</span>}
                {a.home && !a.yours && <span className="tml-flag">⌂ {tr.t("ui-town-map-area-home")}</span>}
              </button>,
            ];
          })}
        {data.kind === "boundaries" &&
          view.hoods.map((h) => {
            // Named where there is room (the reading of its district and the list always name it): beside the
            // district's mark where that stands in it, else at its centre.
            const marked = data.areas.some((a) => areaAnchorHood(map, a.hoods) === h.id);
            const at = hoodAnchor(map, h.id);
            return at ? (
              <span key={`n${h.id}`} className="tml-mark tml-hood-name" data-wx={at.x} data-wy={at.y} data-dy={0} data-around={marked ? FACE + 48 : undefined} data-rank="5" aria-hidden="true">
                {hoodName(tr, h)}
              </span>
            ) : null;
          })}
        {data.kind === "markers" &&
          clusters.map((c) => {
            const first = c.members[0];
            if (!first) return null;
            const dy = first.spot.startsWith("streets:") ? 26 : 0;
            if (c.members.length === 1) {
              const name = markerName(tr, first.marker);
              return [
                face(first.marker, c.at, dy, c.key),
                <span key={`${c.key}n`} className="tml-mark tml-person-name" data-wx={c.at.x} data-wy={c.at.y} data-dy={dy} data-around={FACE} data-rank="6" aria-hidden="true">
                  {name}
                </span>,
              ];
            }
            const on = focus?.type === "people" && focus.key === c.key;
            return (
              <button
                key={c.key}
                type="button"
                className={on ? "tml-mark tml-cluster on" : "tml-mark tml-cluster"}
                data-map-control=""
                data-wx={c.at.x}
                data-wy={c.at.y}
                data-dy={dy}
                data-rank="0"
                data-keep="1"
                data-testid="town-map-cluster"
                data-count={c.members.length}
                aria-label={tr.t("ui-town-map-cluster", { n: c.members.length, place: spotName(c.spot ?? first.spot) })}
                onClick={() => onCluster(c)}
              >
                <Face m={first.marker} parties={view.parties} />
                <span className="tml-count" aria-hidden="true">
                  {c.members.length}
                </span>
              </button>
            );
          })}
        {data.kind === "shaded" &&
          option &&
          view.hoods.map((h, i) => {
            const at = hoodAnchor(map, h.id);
            if (!at) return null;
            const cell = option.cells[i];
            const known = cell?.label_key ?? null;
            const on = focus?.type === "hood" && focus.id === h.id;
            const cells = map.districts.find((d) => d.id === h.id)?.cells ?? 0;
            return (
              <button
                key={h.id}
                type="button"
                className={["tml-mark", "tml-hood", known ? "" : "tml-hood-unknown", on ? "on" : ""].filter(Boolean).join(" ")}
                data-map-control=""
                data-wx={at.x}
                data-wy={at.y}
                data-dy={0}
                data-rank={2 + 1 / (1 + cells)}
                data-testid="town-map-hood"
                data-hood={h.id}
                data-known={known ? "true" : "false"}
                onClick={() => onHood(h.id)}
              >
                <span className="tml-hood-title">{hoodName(tr, h)}</span>
                {/* Unknown is the striped ground and the legend's entry; a label on every such neighbourhood would read as a list of places still to visit (DEC-091, 2026-11-25). Still said to a screen reader. */}
                <span className={known ? "tml-hood-value" : "visually-hidden"}>{known ? tr.t(known) : tr.t("ui-town-map-unknown")}</span>
              </button>
            );
          })}
      </div>
    </div>
  );
}
