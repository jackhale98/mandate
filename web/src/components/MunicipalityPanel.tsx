import { useEffect, useState } from "react";
import { engine, expectKind } from "../engine/client";
import type { MunicipalityView } from "../engine/protocol/MunicipalityView";
import type { GeoIndexView } from "../engine/protocol/GeoIndexView";
import { useGame } from "../state/gameStore";
import { formatCount } from "../i18n/format";

function IndexBars({ items, tone }: { items: GeoIndexView[]; tone: "good" | "risk" }) {
  const tr = useGame((s) => s.tr);
  return (
    <ul className="bars">
      {items.map((i) => {
        const pct = Math.round(i.value * 100);
        return (
          <li key={i.key}>
            <span>{tr.t(`geo-${i.key}`)}</span>
            <span className="bar" aria-hidden="true">
              <span className={`bar-fill bar-${tone}`} style={{ width: `${pct}%` }} />
            </span>
            <span className="num">{tr.t("ui-index", { pct })}</span>
          </li>
        );
      })}
    </ul>
  );
}

export function MunicipalityPanel({ id, onClose }: { id: number; onClose: () => void }) {
  const tr = useGame((s) => s.tr);
  const [view, setView] = useState<MunicipalityView | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    engine()
      .request({ kind: "query", payload: { type: "municipality", id } })
      .then((b) => live && setView(expectKind(b, "municipality")))
      .catch((e: unknown) => live && setError(String(e)));
    return () => {
      live = false;
    };
  }, [id]);

  return (
    <aside className="sheet" aria-label={view?.name ?? ""}>
      <div className="sheet-head">
        <div>
          <h2>{view?.name ?? "…"}</h2>
          {view && (
            <p className="muted">
              {tr.t(`class-${view.class}`)} · {view.region} · {formatCount(view.population)}
            </p>
          )}
        </div>
        <button type="button" className="btn btn-ghost" onClick={onClose} aria-label={tr.t("ui-close")}>
          ✕
        </button>
      </div>
      {error && <p role="alert">{error}</p>}
      {view && (
        <div className="sheet-body">
          <div className="tags">
            {view.biome && <span className="chip">{tr.t(`biome-${view.biome}`)}</span>}
            {view.coastal && <span className="chip">{tr.t("ui-coastal")}</span>}
            {view.port && <span className="chip">{tr.t("ui-port")}</span>}
            {view.on_river && <span className="chip">{tr.t("ui-river")}</span>}
          </div>
          <h3 className="sub">{tr.t("ui-climate")}</h3>
          <p className="muted">
            {tr.t("ui-elevation", { m: Math.round(view.elevation_m) })} · {tr.t("ui-temperature", { c: view.temperature_c })} ·{" "}
            {tr.t("ui-precipitation", { mm: Math.round(view.precipitation_mm) })}
          </p>
          <h3 className="sub">{tr.t("ui-resources")}</h3>
          <IndexBars items={view.resources} tone="good" />
          <h3 className="sub">{tr.t("ui-hazards")}</h3>
          <IndexBars items={view.hazards} tone="risk" />
          <h3 className="sub">{tr.t("ui-neighborhoods")}</h3>
          <ul className="region-list">
            {view.neighborhoods.map((n) => (
              <li key={n.id}>
                <span>
                  {n.name} <span className="muted">· {tr.t(`hood-${n.kind}`)}</span>
                </span>
                <span className="num">{formatCount(n.population)}</span>
              </li>
            ))}
          </ul>
          <h3 className="sub">{tr.t("ui-places")}</h3>
          <ul className="place-list">
            {view.places.map((p) => (
              <li key={p.id}>{p.name ? `${p.name} · ${tr.t(`place-${p.kind}`)}` : tr.t(`place-${p.kind}`)}</li>
            ))}
          </ul>
        </div>
      )}
    </aside>
  );
}
