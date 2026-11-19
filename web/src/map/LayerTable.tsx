// The map layer as a list (DEC-089, DEC-051): every town with its figure for
// the active layer, sortable by figure or name, each town a button to its
// panel. The text alternative to the coloured map.
import { useState } from "react";
import type { MapLayerView } from "../engine/protocol/MapLayerView";
import type { Translator } from "../i18n/fluent";
import { formatValue, sortRows, type LayerRow, type SortKey } from "./layers";

interface Props {
  tr: Translator;
  layer: MapLayerView;
  rows: LayerRow[];
  colours: string[];
  home: number | null;
  /** The town being read, highlighted. */
  focus: number | null;
  onOpen: (id: number) => void;
}

export function LayerTable({ tr, layer, rows, colours, home, focus, onOpen }: Props) {
  const [sort, setSort] = useState<{ key: SortKey; desc: boolean }>({ key: "value", desc: true });
  const label = tr.t(layer.label_key);
  const sorted = sortRows(rows, sort.key, sort.desc, tr);
  const header = (key: SortKey, text: string) => {
    const on = sort.key === key;
    return (
      <th scope="col" aria-sort={on ? (sort.desc ? "descending" : "ascending") : "none"} className={key === "value" ? "num" : undefined}>
        <button
          type="button"
          className="map-table-sort"
          onClick={() => setSort(on ? { key, desc: !sort.desc } : { key, desc: key === "value" })}
          aria-label={tr.t("ui-map-table-sort", { col: text })}
          data-testid={`map-table-sort-${key}`}
        >
          {text}
          <span aria-hidden="true" className="map-table-arrow">
            {on ? (sort.desc ? "▼" : "▲") : "↕"}
          </span>
        </button>
      </th>
    );
  };
  return (
    <div className="map-table-wrap" data-testid="map-table">
      <table className="map-table">
        <caption className="visually-hidden">{tr.t("ui-map-table-caption", { layer: label })}</caption>
        <thead>
          <tr>
            {header("town", tr.t("ui-map-table-town"))}
            {header("value", label)}
          </tr>
        </thead>
        <tbody>
          {sorted.map((r) => {
            const v = formatValue(tr, layer, r.value);
            return (
              <tr key={r.id} className={r.id === focus ? "on" : undefined} data-town={r.id}>
                <th scope="row">
                  <button type="button" className="map-table-town" onClick={() => onOpen(r.id)} aria-label={tr.t("ui-map-table-open", { town: r.name })}>
                    {r.name}
                  </button>
                  {r.id === home && <span className="you-chip">{tr.t("ui-you")}</span>}
                </th>
                <td className={layer.scale === "categorical" ? undefined : "num"}>
                  <span className="map-swatch" style={{ background: r.cls === null ? "transparent" : colours[r.cls] }} aria-hidden="true" />
                  {v.text}
                  {v.detail && <span className="muted small"> · {v.detail}</span>}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
