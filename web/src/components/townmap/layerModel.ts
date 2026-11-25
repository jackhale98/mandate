// The town map's layers (DEC-086, DEC-089's pattern; M6 note 11): how what
// the engine serves (`TownMapLayersView`) is laid over the town's layout.
// Pure and deterministic (no DOM, no clock): where each district's outline
// runs, where a mark sits, which portraits would overlap and so share one
// mark, and which colour each class is painted. Presentation only: the engine
// decides the districts, who is where, every class and every legend entry;
// nothing here ranks, counts or filters by anything but the ids it is given.
import type { LocationDto } from "../../engine/protocol/LocationDto";
import type { TownAreaView } from "../../engine/protocol/TownAreaView";
import type { TownLayerHoodView } from "../../engine/protocol/TownLayerHoodView";
import type { TownLayerPartyView } from "../../engine/protocol/TownLayerPartyView";
import type { TownMapLayerView } from "../../engine/protocol/TownMapLayerView";
import type { TownMarkerView } from "../../engine/protocol/TownMarkerView";
import type { TownShadeOptionView } from "../../engine/protocol/TownShadeOptionView";
import type { Translator } from "../../i18n/fluent";
import type { Point } from "../../hub/pathfinding";
import type { TownMap } from "../../hub/townLayout";
import { hotspotOf, type MapHotspot } from "../../hub/townMapScene";
import { classColours } from "../../map/layerPalette";
import { hoodLabel } from "../../places/labels";
import { partyColour } from "../ElectionCharts";

/** The plain map: no layer. */
export const NO_LAYER = "none";
const LAYER_KEY = "mandate.townMapLayer";

/** A layer's key in the picker and in storage: its content id's name (`town_map_layer.people` → `people`). */
export function layerKey(l: Pick<TownMapLayerView, "id">): string {
  return l.id.replace(/^town_map_layer\./, "");
}

/** The layer chosen last on this device (best effort: storage may be blocked). */
export function readLayer(): string {
  try {
    return localStorage.getItem(LAYER_KEY) ?? NO_LAYER;
  } catch {
    return NO_LAYER;
  }
}
export function writeLayer(key: string) {
  try {
    localStorage.setItem(LAYER_KEY, key);
  } catch {
    // Storage blocked: the choice lasts for this visit.
  }
}

/** A neighbourhood's name as the Places tab gives it (the centre reads as the town centre). */
export function hoodName(tr: Translator, h: TownLayerHoodView): string {
  return hoodLabel(tr, h.name, h.name_key, h.name_args);
}

/** A district's name in the constitution's words, as the "where you are" chip gives it. */
export function areaName(tr: Translator, a: Pick<TownAreaView, "label_key" | "label_args">): string {
  return tr.t(a.label_key, a.label_args);
}

/** The app's colour for a party (the one rule: ../ElectionCharts), as a CSS value. */
export function partyCss(parties: readonly TownLayerPartyView[], party: number | null): string {
  return partyColour(
    parties.map((p) => ({ ...p, stances: [] })),
    party,
  );
}

/** A party's name, or null for none. */
export function partyName(parties: readonly TownLayerPartyView[], party: number | null): string | null {
  return party === null ? null : (parties.find((p) => p.id === party)?.name ?? null);
}

// ---------------------------------------------------------------------------
// Geometry: the layout's neighbourhoods (its `districts`) as cells

/** Layout district index per engine neighbourhood id. */
export function hoodIndex(map: TownMap): Map<number, number> {
  return new Map(map.districts.map((d) => [d.id, d.index]));
}

/**
 * The outline of a set of neighbourhoods as an SVG path in tiles: every cell
 * edge with the set on one side only, runs of edges merged into one segment.
 */
export function outlinePath(map: TownMap, hoods: readonly number[]): string {
  const idx = hoodIndex(map);
  const inSet = new Uint8Array(map.districts.length);
  for (const h of hoods) {
    const i = idx.get(h);
    if (i !== undefined) inSet[i] = 1;
  }
  const W = map.width;
  const H = map.height;
  const at = (x: number, y: number): number => {
    if (x < 0 || y < 0 || x >= W || y >= H) return 0;
    const d = map.district[y * W + x] as number;
    return d >= 0 ? (inSet[d] as number) : 0;
  };
  const parts: string[] = [];
  // Horizontal edges: between row y-1 and row y.
  for (let y = 0; y <= H; y++) {
    let start = -1;
    for (let x = 0; x <= W; x++) {
      const edge = x < W && at(x, y - 1) !== at(x, y);
      if (edge && start < 0) start = x;
      if (!edge && start >= 0) {
        parts.push(`M${start} ${y}H${x}`);
        start = -1;
      }
    }
  }
  // Vertical edges: between column x-1 and column x.
  for (let x = 0; x <= W; x++) {
    let start = -1;
    for (let y = 0; y <= H; y++) {
      const edge = y < H && at(x - 1, y) !== at(x, y);
      if (edge && start < 0) start = y;
      if (!edge && start >= 0) {
        parts.push(`M${x} ${start}V${y}`);
        start = -1;
      }
    }
  }
  return parts.join("");
}

/** The largest of a set of neighbourhoods in the layout (the first of equals), or null if it has none of them. */
function largest(map: TownMap, hoods: readonly number[]): TownMap["districts"][number] | null {
  const idx = hoodIndex(map);
  let best: TownMap["districts"][number] | null = null;
  for (const h of hoods) {
    const i = idx.get(h);
    const d = i === undefined ? undefined : map.districts[i];
    if (d && (!best || d.cells > best.cells)) best = d;
  }
  return best;
}

/** Where a set of neighbourhoods is marked: the centre of its largest one (always inside it), tiles. */
export function areaAnchor(map: TownMap, hoods: readonly number[]): Point | null {
  const d = largest(map, hoods);
  return d ? { ...d.centre } : null;
}

/** The neighbourhood a set's mark stands in (its largest), by engine id. */
export function areaAnchorHood(map: TownMap, hoods: readonly number[]): number | null {
  return largest(map, hoods)?.id ?? null;
}

/** A neighbourhood's centre, tiles. */
export function hoodAnchor(map: TownMap, hood: number): Point | null {
  const d = map.districts.find((x) => x.id === hood);
  return d ? { ...d.centre } : null;
}

// ---------------------------------------------------------------------------
// Fills: one RGBA per neighbourhood, painted a tile to a pixel

export interface HoodFill {
  /** Red, green, blue (0–255) and opacity (0–1). */
  rgba: readonly [number, number, number, number];
  /** Not known: drawn as diagonal stripes, never as a class. */
  hatch?: boolean;
}

/** Parse `#rrggbb` (or `rgb(r, g, b)`, as computed styles give it). */
export function parseColour(css: string): [number, number, number] {
  const s = css.trim();
  const hex = /^#([0-9a-f]{6})$/i.exec(s);
  if (hex) {
    const n = parseInt(hex[1] as string, 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }
  const rgb = /^rgba?\(\s*(\d+)[,\s]+(\d+)[,\s]+(\d+)/i.exec(s);
  if (rgb) return [Number(rgb[1]), Number(rgb[2]), Number(rgb[3])];
  return [138, 143, 158];
}

/** The neutral of a neighbourhood not known. */
export const UNKNOWN_RGB: readonly [number, number, number] = [72, 76, 88];

/**
 * The layer's fill as RGBA pixels, one per tile (`width × height × 4`): each
 * neighbourhood's colour over its cells, nothing outside the town. Unknown
 * neighbourhoods are striped (every third diagonal), so they never read as a
 * pale class.
 */
export function fillPixels(map: TownMap, fills: ReadonlyMap<number, HoodFill>): Uint8ClampedArray {
  const W = map.width;
  const H = map.height;
  const out = new Uint8ClampedArray(W * H * 4);
  const byIndex: (HoodFill | undefined)[] = map.districts.map((d) => fills.get(d.id));
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const d = map.district[y * W + x] as number;
      const f = d >= 0 ? byIndex[d] : undefined;
      if (!f) continue;
      const o = (y * W + x) * 4;
      const stripe = f.hatch ? ((x + y) % 4 < 2 ? 1 : 0.35) : 1;
      out[o] = f.rgba[0];
      out[o + 1] = f.rgba[1];
      out[o + 2] = f.rgba[2];
      out[o + 3] = Math.round(f.rgba[3] * stripe * 255);
    }
  }
  return out;
}

/** The colours of a shaded choice's classes, lowest first (the map's own light ground: the light ramps). */
export function shadeColours(o: Pick<TownShadeOptionView, "scale" | "bins" | "id">): string[] {
  if (o.scale === "categorical") {
    const other = o.bins[o.bins.length - 1]?.label_key === "ui-map-legend-other";
    return classColours("categorical", o.bins.length, "light", other);
  }
  return classColours("sequential", o.bins.length, "light");
}

// ---------------------------------------------------------------------------
// Marks: people at places, clustered where their portraits would overlap

/** Where a location is marked on the map, tiles: over its building, or at its neighbourhood's centre. */
export function locationAnchor(hotspots: readonly MapHotspot[], loc: LocationDto): { at: Point; key: string } | null {
  const h = hotspotOf(hotspots, loc);
  if (!h) return null;
  if (h.kind === "streets") return { at: { ...h.anchor }, key: h.key };
  return { at: { x: h.rect.x + h.rect.w / 2, y: h.rect.y + h.rect.h / 2 }, key: h.key };
}

export interface PlacedMarker {
  marker: TownMarkerView;
  at: Point;
  /** The hotspot it stands at. */
  spot: string;
}

export interface MarkerCluster {
  /** Stable for a set of members: the first member's id. */
  key: string;
  at: Point;
  members: PlacedMarker[];
  /** The hotspot they share, when they all stand at one place. */
  spot: string | null;
}

/** The markers the map can place (those with a known location it has a spot for), and the rest. */
export function placeMarkers(hotspots: readonly MapHotspot[], markers: readonly TownMarkerView[]): { placed: PlacedMarker[]; unplaced: TownMarkerView[] } {
  const placed: PlacedMarker[] = [];
  const unplaced: TownMarkerView[] = [];
  for (const m of markers) {
    const a = m.at ? locationAnchor(hotspots, m.at.location) : null;
    if (a) placed.push({ marker: m, at: a.at, spot: a.key });
    else unplaced.push(m);
  }
  return { placed, unplaced };
}

/**
 * Group markers whose portraits would overlap at this zoom (`size` CSS px a
 * portrait, `zoom` CSS px a tile): people at one place always share a mark;
 * marks closer than a portrait merge, nearest first in the order given.
 * Deterministic: a pass in input order, each marker joining the first
 * cluster within reach.
 */
export function clusterMarkers(placed: readonly PlacedMarker[], zoom: number, size: number): MarkerCluster[] {
  const reach = size / Math.max(zoom, 0.0001);
  const out: MarkerCluster[] = [];
  for (const p of placed) {
    const c = out.find((q) => (q.spot !== null && q.spot === p.spot) || (Math.abs(q.at.x - p.at.x) < reach && Math.abs(q.at.y - p.at.y) < reach));
    if (c) {
      c.members.push(p);
      if (c.spot !== p.spot) c.spot = null;
    } else out.push({ key: `m${p.marker.person.id ?? p.marker.person.ambient_slot ?? out.length}`, at: { ...p.at }, members: [p], spot: p.spot });
  }
  return out;
}

/** The markers a filter keeps (the engine says which: each marker lists its filters). */
export function keptBy<T extends Pick<TownMarkerView, "filters">>(markers: readonly T[], filter: string): T[] {
  return markers.filter((m) => m.filters.includes(filter));
}

/** A shaded layer's choice by id, else its first. */
export function chosenOption(options: readonly TownShadeOptionView[], id: string | undefined): TownShadeOptionView | null {
  return options.find((o) => o.id === id) ?? options[0] ?? null;
}
