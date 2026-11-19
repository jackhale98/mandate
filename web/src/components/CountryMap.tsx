// The national map (DEC-021), shown in the map overlay's Country view: the
// country's towns on a canvas with a layer picker, a "You: <town>" button
// that selects the player's town, and the help line. Tapping a town selects
// it (the overlay opens its municipality panel; travel between towns is M7).
// The town list in Overview is its text alternative (DEC-051).
import { useState } from "react";
import { useGame } from "../state/gameStore";
import { WorldMap } from "../map/WorldMap";
import type { Overlay } from "../map/render";
import { Loader } from "./Brand";

export function CountryMap({ onSelect, selected }: { onSelect: (id: number | null) => void; selected: number | null }) {
  const tr = useGame((s) => s.tr);
  const map = useGame((s) => s.map);
  const homeId = useGame((s) => s.office?.player.municipality_id ?? null);
  const seed = useGame((s) => s.overview?.seed ?? "");
  const home = map?.municipalities.find((m) => m.id === homeId);
  const [overlay, setOverlay] = useState<Overlay>("terrain");
  return (
    <div className="map-body" data-testid="country-map">
      {map ? (
        <WorldMap
          view={map}
          selected={selected}
          onSelect={onSelect}
          label={home ? `${tr.t("ui-map-help")} ${tr.t("ui-you")}: ${home.name}.` : tr.t("ui-map-help")}
          overlay={overlay}
          home={home?.id ?? null}
          homeLabel={tr.t("ui-you")}
          seed={seed}
        />
      ) : (
        <div className="center">
          <Loader compact />
        </div>
      )}
      <div className="map-tools">
        <label className="map-layer">
          <span className="muted small map-layer-label">{tr.t("ui-overlay")}</span>
          <select value={overlay} onChange={(e) => setOverlay(e.target.value as Overlay)} data-testid="map-layer">
            {(["terrain", "unemployment"] as const).map((o) => (
              <option key={o} value={o}>
                {tr.t(`ui-overlay-${o}`)}
              </option>
            ))}
          </select>
        </label>
        {home && (
          <button type="button" className="btn btn-sm btn-ghost map-home" onClick={() => onSelect(home.id)} data-testid="map-home">
            <span aria-hidden="true">⚑ </span>
            {tr.t("ui-you")}: {home.name}
          </button>
        )}
        {overlay === "unemployment" && (
          <div className="legend" aria-hidden="true">
            <span>{tr.t("ui-legend-low")} 2%</span>
            <span className="legend-ramp" />
            <span>15% {tr.t("ui-legend-high")}</span>
          </div>
        )}
      </div>
      <p className="map-help muted">{tr.t("ui-map-help")}</p>
    </div>
  );
}
