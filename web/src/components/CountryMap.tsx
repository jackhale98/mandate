// The national map (DEC-021), shown in the map overlay's Country view: the
// country's towns on a canvas with a layer picker (DEC-089), a "You: <town>"
// button that selects the player's town, and the help line.
//
// Layers come from the engine (`map_layers`): the registry's layers in its
// order, each with every town's class and a legend, refreshed as the days
// pass. The client only colours the classes it is given. With a layer on, a
// tap (or a hover with a mouse) reads the town's figure into the legend and
// "Details" opens its panel; with terrain only, a tap opens the panel. "List"
// shows the layer as a sortable table of towns, its text alternative (DEC-051).
import { useEffect, useMemo, useState } from "react";
import { engine, expectKind } from "../engine/client";
import type { MapLayersView } from "../engine/protocol/MapLayersView";
import { useGame } from "../state/gameStore";
import { WorldMap } from "../map/WorldMap";
import { MapLegend } from "../map/MapLegend";
import { LayerTable } from "../map/LayerTable";
import { layerColours, layerKey, layerPaint, layerRows } from "../map/layers";
import { useScheme } from "../map/useScheme";
import { Loader } from "./Brand";
import "../map/mapLayers.css";

const TERRAIN = "terrain";
const LAYER_KEY = "mandate.mapLayer";

/** The layer chosen last on this device (best effort: storage may be blocked). */
function readLayer(): string {
  try {
    return localStorage.getItem(LAYER_KEY) ?? TERRAIN;
  } catch {
    return TERRAIN;
  }
}
function writeLayer(key: string) {
  try {
    localStorage.setItem(LAYER_KEY, key);
  } catch {
    // Storage blocked: the choice lasts for this visit.
  }
}

export function CountryMap({ onSelect, selected }: { onSelect: (id: number | null) => void; selected: number | null }) {
  const tr = useGame((s) => s.tr);
  const map = useGame((s) => s.map);
  const homeId = useGame((s) => s.office?.player.municipality_id ?? null);
  const seed = useGame((s) => s.overview?.seed ?? "");
  const day = useGame((s) => s.overview?.date.day_number ?? 0);
  const scheme = useScheme();
  const home = map?.municipalities.find((m) => m.id === homeId);
  const [layers, setLayers] = useState<MapLayersView | null>(null);
  const [chosen, setChosen] = useState<string>(readLayer);
  const [list, setList] = useState(false);
  const [focus, setFocus] = useState<number | null>(null);
  const [hover, setHover] = useState<number | null>(null);

  // This month's figures; the previous ones stay on screen while the next load.
  useEffect(() => {
    let live = true;
    engine()
      .request({ kind: "query", payload: { type: "map_layers" } })
      .then((b) => live && setLayers(expectKind(b, "map_layers")))
      .catch(() => {
        // The map still works on terrain; the picker offers only that.
      });
    return () => {
      live = false;
    };
  }, [day]);

  const layer = layers?.layers.find((l) => layerKey(l) === chosen) ?? null;
  const colours = useMemo(() => (layer ? layerColours(layer, scheme) : []), [layer, scheme]);
  const paint = useMemo(() => (layers && layer ? layerPaint(layers, layer, scheme) : null), [layers, layer, scheme]);
  const names = useMemo(() => new Map((map?.municipalities ?? []).map((m) => [m.id, m.name])), [map]);
  const rows = useMemo(() => (layers && layer ? layerRows(layers, layer, names) : []), [layers, layer, names]);
  const readId = hover ?? focus ?? (selected !== null && layer ? selected : null);
  const read = readId === null ? null : rows.find((r) => r.id === readId);

  const choose = (key: string) => {
    setChosen(key);
    writeLayer(key);
    if (key === TERRAIN) setList(false);
  };
  const pick = (id: number | null) => {
    if (layer) {
      setFocus(id);
      return;
    }
    onSelect(id);
  };
  const options = [TERRAIN, ...(layers?.layers.map(layerKey) ?? [])];
  const layerName = layer ? tr.t(layer.label_key) : tr.t("ui-overlay-terrain");
  const help = home ? `${tr.t("ui-map-help")} ${tr.t("ui-you")}: ${home.name}.` : tr.t("ui-map-help");

  return (
    <div className="map-body" data-testid="country-map" data-layer={layer ? layerKey(layer) : TERRAIN} data-figures-day={layers?.date.day_number}>
      <div className="map-tools">
        <label className="map-layer">
          <span className="muted small map-layer-label">{tr.t("ui-overlay")}</span>
          <select value={layer ? chosen : TERRAIN} onChange={(e) => choose(e.target.value)} data-testid="map-layer">
            {options.map((key) => {
              const l = layers?.layers.find((x) => layerKey(x) === key);
              return (
                <option key={key} value={key}>
                  {l ? tr.t(l.label_key) : tr.t("ui-overlay-terrain")}
                </option>
              );
            })}
          </select>
        </label>
        {layer && (
          <button
            type="button"
            className={list ? "btn btn-sm map-list-toggle on" : "btn btn-sm btn-ghost map-list-toggle"}
            aria-pressed={list}
            onClick={() => setList(!list)}
            title={tr.t(list ? "ui-map-list-hide" : "ui-map-list-show")}
            data-testid="map-list-toggle"
          >
            <span aria-hidden="true">☰ </span>
            {tr.t("ui-map-list")}
          </button>
        )}
        {home && (
          <button
            type="button"
            className="btn btn-sm btn-ghost map-home"
            onClick={() => {
              setFocus(home.id);
              onSelect(home.id);
            }}
            data-testid="map-home"
          >
            <span aria-hidden="true">⚑ </span>
            {tr.t("ui-you")}: {home.name}
          </button>
        )}
      </div>
      {!map ? (
        <div className="center">
          <Loader compact />
        </div>
      ) : list && layer ? (
        <LayerTable tr={tr} layer={layer} rows={rows} colours={colours} home={homeId} focus={readId} onOpen={(id) => onSelect(id)} />
      ) : (
        <WorldMap
          view={map}
          selected={layer ? (focus ?? selected) : selected}
          onSelect={pick}
          onHover={layer ? (id) => setHover((h) => (h === id ? h : id)) : undefined}
          label={layer ? `${help} ${tr.t("ui-overlay")}: ${layerName}.` : help}
          paint={paint}
          home={home?.id ?? null}
          homeLabel={tr.t("ui-you")}
          seed={seed}
        />
      )}
      {layer && (
        <MapLegend
          tr={tr}
          layer={layer}
          colours={colours}
          town={read ? { ...read, home: read.id === homeId } : null}
          onDetails={(id) => onSelect(id)}
        />
      )}
      {!layer && <p className="map-help muted">{tr.t("ui-map-help")}</p>}
    </div>
  );
}
