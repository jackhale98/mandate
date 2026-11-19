// A map layer's legend (DEC-089) with the readout of one town's figure.
// Numeric layers: a bar of classes with the engine's breaks between them;
// categorical layers: a swatch and name per category. Every class is named in
// text (a list for screen readers, the breaks on screen), and the town being
// read is marked in the legend by an outline as well as by colour (DEC-051).
import type { MapLayerView } from "../engine/protocol/MapLayerView";
import type { Translator } from "../i18n/fluent";
import { formatDate } from "../i18n/format";
import { binLabel, formatNumber, formatValue, layerNote, type LayerRow } from "./layers";

interface Props {
  tr: Translator;
  layer: MapLayerView;
  colours: string[];
  /** The town being read (hovered or tapped), if any. */
  town: (LayerRow & { home: boolean }) | null;
  /** Open the town's full panel. */
  onDetails?: (id: number) => void;
}

export function MapLegend({ tr, layer, colours, town, onDetails }: Props) {
  const label = tr.t(layer.label_key);
  const note = layerNote(tr, layer);
  const read = town ? formatValue(tr, layer, town.value) : null;
  const numeric = layer.scale !== "categorical";
  const edges = numeric ? layer.bins.slice(1).map((b) => (b.lo === null ? "" : formatNumber(tr, layer, b.lo, true))) : [];
  const total = layer.bins.length;
  return (
    <section className="map-legend legend" aria-label={`${tr.t("ui-map-legend")}: ${label}`} data-testid="map-legend" data-layer={layer.id}>
      <div className="map-readout" data-testid="map-readout" aria-live="polite">
        {town && read ? (
          <>
            <span
              className="map-swatch"
              style={{ background: town.cls === null ? "transparent" : colours[town.cls] }}
              aria-hidden="true"
            />
            <span className="map-readout-text">
              <strong className="map-readout-town">
                {town.name}
                {town.home && <span className="you-chip">{tr.t("ui-you")}</span>}
              </strong>{" "}
              <span className="map-readout-value" data-testid="map-readout-value">
                {read.text}
              </span>
              {read.detail && <span className="muted map-readout-detail"> {read.detail}</span>}
              {town.cls !== null && numeric && (
                <span className="visually-hidden"> · {tr.t("ui-map-class", { n: town.cls + 1, total })}</span>
              )}
            </span>
            {onDetails && (
              <button type="button" className="btn btn-sm btn-ghost map-readout-more" onClick={() => onDetails(town.id)} data-testid="map-readout-details">
                {tr.t("ui-map-details")} ›
              </button>
            )}
          </>
        ) : (
          <span className="muted">{tr.t("ui-map-readout-hint")}</span>
        )}
      </div>
      <div className="map-legend-head">
        <strong>{label}</strong> <span className="muted">· {tr.t(layer.unit_key)}</span>
        {layer.as_of && <span className="muted"> · {tr.t("ui-map-as-of", { date: formatDate(tr, layer.as_of) })}</span>}
      </div>
      {numeric ? (
        <div className="map-scale" aria-hidden="true">
          <div className="map-scale-bar">
            {layer.bins.map((b, i) => (
              <span
                key={i}
                className={town?.cls === i ? "map-scale-seg on" : "map-scale-seg"}
                style={{ background: colours[i] }}
                title={`${binLabel(tr, layer, b, false)} · ${tr.t("ui-map-legend-towns", { n: b.count })}`}
              />
            ))}
          </div>
          <div className="map-scale-edges">
            {edges.map((e, i) => (
              <span key={i} style={{ left: `${((i + 1) / total) * 100}%` }}>
                {e}
              </span>
            ))}
          </div>
        </div>
      ) : (
        <ul className="map-cats">
          {layer.bins.map((b, i) => (
            <li key={i} className={town?.cls === i ? "on" : undefined}>
              <span className="map-swatch" style={{ background: colours[i] }} aria-hidden="true" />
              {binLabel(tr, layer, b)} <span className="muted">({b.count})</span>
              <span className="visually-hidden"> {tr.t("ui-map-legend-towns", { n: b.count })}</span>
            </li>
          ))}
        </ul>
      )}
      {numeric && (
        <ul className="visually-hidden">
          {layer.bins.map((b, i) => (
            <li key={i}>
              {binLabel(tr, layer, b, false)}: {tr.t("ui-map-legend-towns", { n: b.count })}
            </li>
          ))}
        </ul>
      )}
      {note && <p className="map-legend-note muted">{note}</p>}
    </section>
  );
}
