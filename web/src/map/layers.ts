// The country map's layers (DEC-089) as the client reads them: the engine
// serves every figure, class break, legend bin and each town's class
// (`MapLayersView`); here they are only written out and coloured.
import type { MapLayerView } from "../engine/protocol/MapLayerView";
import type { MapLayersView } from "../engine/protocol/MapLayersView";
import type { MapTownFigures } from "../engine/protocol/MapTownFigures";
import type { MapCategoryFigure } from "../engine/protocol/MapCategoryFigure";
import type { MapLegendBin } from "../engine/protocol/MapLegendBin";
import type { Translator } from "../i18n/fluent";
import { classColours, hexToRgb, type Scheme } from "./layerPalette";

/** A town's figure for a layer: a number, a category, or unknown. */
export type LayerValue = { kind: "number"; value: number } | { kind: "category"; value: MapCategoryFigure } | { kind: "unknown" };

/** The figure a layer's `source` names in a town's figures. */
export function townValue(t: MapTownFigures | undefined, source: string): LayerValue {
  const v: unknown = t ? (t as unknown as Record<string, unknown>)[source] : undefined;
  if (typeof v === "number" && Number.isFinite(v)) return { kind: "number", value: v };
  if (v && typeof v === "object" && "label_key" in v) return { kind: "category", value: v as MapCategoryFigure };
  return { kind: "unknown" };
}

/** The picker's value for a layer: its id without the `map_layer.` prefix. */
export function layerKey(l: MapLayerView): string {
  return l.id.replace(/^map_layer\./, "");
}

/** Whether a layer's last class is a categorical "Other". */
export function hasOther(l: MapLayerView): boolean {
  return l.scale === "categorical" && l.bins.length > 0 && l.bins[l.bins.length - 1]?.category === null;
}

/** Each class's colour, lowest first. */
export function layerColours(l: MapLayerView, scheme: Scheme): string[] {
  return classColours(l.scale, l.bins.length, scheme, hasOther(l));
}

/** Each town's fill (by municipality id) for the map raster; towns of unknown figure are left out. */
export function layerPaint(view: MapLayersView, l: MapLayerView, scheme: Scheme): Map<number, readonly [number, number, number]> {
  const colours = layerColours(l, scheme).map(hexToRgb);
  const out = new Map<number, readonly [number, number, number]>();
  view.towns.forEach((t, i) => {
    const c = l.classes[i];
    const rgb = c === null || c === undefined ? undefined : colours[c];
    if (rgb) out.set(t.id, rgb);
  });
  return out;
}

/** A number in a layer's format; `compact` for the legend's edges. */
export function formatNumber(tr: Translator, l: Pick<MapLayerView, "format">, v: number, compact = false): string {
  switch (l.format) {
    case "count":
      return new Intl.NumberFormat("en", { notation: compact && v >= 1e4 ? "compact" : "standard", maximumFractionDigits: compact ? 1 : 0 }).format(v);
    case "per_km2": {
      const n = new Intl.NumberFormat("en", { maximumFractionDigits: v < 10 ? 1 : 0 }).format(v);
      return compact ? n : tr.t("ui-map-per-km2", { n });
    }
    case "percent":
      return new Intl.NumberFormat("en", { style: "percent", minimumFractionDigits: compact ? 0 : 1, maximumFractionDigits: 1 }).format(v);
    case "money":
      return new Intl.NumberFormat("en", { notation: compact && Math.abs(v) >= 1e4 ? "compact" : "standard", maximumFractionDigits: compact ? 1 : 0 }).format(v);
    case "category":
      return String(v);
  }
}

/** A town's figure as the readout and the list write it. */
export function formatValue(tr: Translator, l: MapLayerView, v: LayerValue): { text: string; detail: string | null } {
  if (v.kind === "unknown") return { text: tr.t("ui-map-legend-unknown"), detail: null };
  if (v.kind === "category") {
    return {
      text: tr.t(v.value.label_key),
      detail: l.detail_key ? tr.t(l.detail_key, { pct: Math.round(v.value.share * 100) }) : null,
    };
  }
  return { text: formatNumber(tr, l, v.value), detail: null };
}

/** A legend class's label: a category's name, "Other", or its range. */
export function binLabel(tr: Translator, l: MapLayerView, b: MapLegendBin, compact = true): string {
  if (l.scale === "categorical") return b.label_key ? tr.t(b.label_key) : tr.t("ui-map-legend-other");
  const f = (x: number) => formatNumber(tr, l, x, compact);
  if (b.lo === null && b.hi !== null) return tr.t("ui-map-legend-under", { hi: f(b.hi) });
  if (b.hi === null && b.lo !== null) return tr.t("ui-map-legend-over", { lo: f(b.lo) });
  if (b.lo !== null && b.hi !== null) return tr.t("ui-map-legend-range", { lo: f(b.lo), hi: f(b.hi) });
  return tr.t("ui-map-legend-unknown");
}

/** The legend's note, with the midpoint written in the layer's format. */
export function layerNote(tr: Translator, l: MapLayerView): string | null {
  if (!l.note_key) return null;
  return tr.t(l.note_key, l.midpoint === null ? {} : { mid: formatNumber(tr, l, l.midpoint) });
}

export interface LayerRow {
  id: number;
  name: string;
  value: LayerValue;
  cls: number | null;
}

/** The towns with their figures for a layer, in the map's order. */
export function layerRows(view: MapLayersView, l: MapLayerView, names: ReadonlyMap<number, string>): LayerRow[] {
  return view.towns.map((t, i) => ({ id: t.id, name: names.get(t.id) ?? String(t.id), value: townValue(t, l.source), cls: l.classes[i] ?? null }));
}

export type SortKey = "value" | "town";

/** Sort the list: by figure (highest first; categories by class, then name) or by name. */
export function sortRows(rows: LayerRow[], key: SortKey, descending: boolean, tr: Translator): LayerRow[] {
  const byName = (a: LayerRow, b: LayerRow) => a.name.localeCompare(b.name, "en");
  const num = (r: LayerRow) => (r.value.kind === "number" ? r.value.value : r.value.kind === "category" ? r.value.value.share : -Infinity);
  const cat = (r: LayerRow) => (r.value.kind === "category" ? tr.t(r.value.value.label_key) : "");
  const out = [...rows];
  out.sort((a, b) => {
    let d: number;
    if (key === "town") d = byName(a, b);
    else if (a.value.kind === "category" || b.value.kind === "category") d = cat(a).localeCompare(cat(b), "en") || num(a) - num(b);
    else d = num(a) - num(b);
    if (descending) d = -d;
    return d || byName(a, b);
  });
  return out;
}
