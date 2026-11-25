// A town map layer as a list (DEC-051; quick mode and screen readers):
// everything the layer draws, as text, in the engine's order. Districts with
// their neighbourhoods and member; people grouped by where the player knows
// to find them; neighbourhoods with their reading. Names open the same card
// the portraits do.
import type { TownMapLayerView } from "../../engine/protocol/TownMapLayerView";
import type { TownMapLayersView } from "../../engine/protocol/TownMapLayersView";
import type { TownMarkerView } from "../../engine/protocol/TownMarkerView";
import type { TownShadeOptionView } from "../../engine/protocol/TownShadeOptionView";
import type { Translator } from "../../i18n/fluent";
import { placeName } from "../../places/labels";
import { Unknown } from "../Knowing";
import { PersonRow, whereText } from "./TownLayerPanel";
import { areaName, hoodName, keptBy, shadeColours } from "./layerModel";

interface Props {
  tr: Translator;
  view: TownMapLayersView;
  layer: TownMapLayerView;
  option: TownShadeOptionView | null;
  filter: string;
  onPerson: (m: TownMarkerView) => void;
}

/** People grouped by where the player knows to find them, in the engine's order; those with no known place last. */
export function byPlace(tr: Translator, markers: readonly TownMarkerView[]): { title: string; people: TownMarkerView[] }[] {
  const groups = new Map<string, { title: string; people: TownMarkerView[] }>();
  const none: TownMarkerView[] = [];
  for (const m of markers) {
    if (!m.at) {
      none.push(m);
      continue;
    }
    const key = JSON.stringify(m.at.location);
    let g = groups.get(key);
    if (!g) groups.set(key, (g = { title: placeName(tr, m.at), people: [] }));
    g.people.push(m);
  }
  const out = [...groups.values()];
  if (none.length > 0) out.push({ title: tr.t("ui-town-map-unplaced"), people: none });
  return out;
}

export function TownLayerList({ tr, view, layer, option, filter, onPerson }: Props) {
  const data = layer.data;
  const label = tr.t(layer.label_key, layer.args);
  const colours = option ? shadeColours(option) : [];
  const empty = (data.kind === "boundaries" && data.areas.length === 0) || (data.kind === "markers" && keptBy(data.markers, filter).length === 0);
  return (
    <div className="tml-list" data-testid="town-map-list" data-layer={layer.id}>
      <h3 className="visually-hidden">{tr.t("ui-town-map-list-caption", { layer: label, town: view.at.name })}</h3>
      {empty && <p className="muted tml-list-empty">{tr.t("ui-town-map-layer-empty")}</p>}
      {data.kind === "boundaries" && (
        <ul className="plain-list tml-list-items">
          {data.areas.map((a) => (
            <li key={a.number} className="tml-list-item" data-testid="town-map-list-area" data-area={a.number}>
              <h4 className="tml-list-title">
                {areaName(tr, a)}
                {a.yours && <span className="you-chip">{tr.t("ui-town-map-area-yours")}</span>}
                {a.home && !a.yours && <span className="you-chip">{tr.t("ui-town-map-area-home")}</span>}
              </h4>
              <p className="muted small">
                {tr.t("ui-town-map-area-hoods", {
                  list: view.hoods
                    .filter((h) => a.hoods.includes(h.id))
                    .map((h) => hoodName(tr, h))
                    .join(", "),
                })}
              </p>
              {a.holder ? (
                <ul className="plain-list tml-rows">
                  <PersonRow tr={tr} m={a.holder} view={view} onPerson={onPerson} />
                </ul>
              ) : (
                <p className="small">{tr.t("ui-town-map-area-vacant")}</p>
              )}
            </li>
          ))}
        </ul>
      )}
      {data.kind === "markers" && (
        <ul className="plain-list tml-list-items">
          {byPlace(tr, keptBy(data.markers, filter)).map((g) => (
            <li key={g.title} className="tml-list-item" data-testid="town-map-list-place">
              <h4 className="tml-list-title">
                {g.title} <span className="muted">({g.people.length})</span>
              </h4>
              <ul className="plain-list tml-rows">
                {g.people.map((m, i) => (
                  <PersonRow key={m.person.id ?? `a${i}`} tr={tr} m={m} view={view} onPerson={onPerson} />
                ))}
              </ul>
              {g.people.some((m) => m.seen) && (
                <p className="muted small">
                  {g.people
                    .filter((m) => m.seen)
                    .map((m) => whereText(tr, m))
                    .filter((t, i, all) => all.indexOf(t) === i)
                    .join(" ")}
                </p>
              )}
            </li>
          ))}
        </ul>
      )}
      {data.kind === "shaded" && option && (
        <ul className="plain-list tml-list-items">
          {view.hoods.map((h, i) => {
            const cell = option.cells[i];
            const details = data.details[i] ?? [];
            const k = data.knowledge[i];
            return (
              <li key={h.id} className="tml-list-item" data-testid="town-map-list-hood" data-hood={h.id} data-known={cell?.label_key ? "true" : "false"}>
                <h4 className="tml-list-title">{hoodName(tr, h)}</h4>
                <p className="small">
                  <span className={cell?.class == null ? "map-swatch tml-hatch" : "map-swatch"} style={cell?.class == null ? undefined : { background: colours[cell.class] }} aria-hidden="true" />{" "}
                  {cell?.label_key ? `${tr.t(option.label_key)}: ${tr.t(cell.label_key)}` : tr.t("ui-town-map-unknown")}
                </p>
                {details.length > 0 ? (
                  <p className="muted small">{tr.t("ui-town-map-readout-worries", { list: details.map((d) => tr.t(d)).join(", ") })}</p>
                ) : (
                  k && (
                    <p className="small">
                      <Unknown k={k} text="knowledge-concerns-unknown" />
                    </p>
                  )
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
