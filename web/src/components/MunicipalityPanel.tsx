import { useEffect, useState } from "react";
import { engine, expectKind } from "../engine/client";
import type { MunicipalityView } from "../engine/protocol/MunicipalityView";
import type { GeoIndexView } from "../engine/protocol/GeoIndexView";
import { useGame } from "../state/gameStore";
import { formatCount, formatMoney, formatPct } from "../i18n/format";
import { StatTile } from "./StatTile";
import type { MuniEconomyView } from "../engine/protocol/MuniEconomyView";

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

function LocalEconomy({ e }: { e: MuniEconomyView }) {
  const tr = useGame((s) => s.tr);
  const maxJobs = Math.max(1, ...e.sectors.map((x) => x.jobs));
  const hist = e.unemployment_history.slice(-12);
  const yearAgo = e.unemployment_history[e.unemployment_history.length - 13];
  const d = yearAgo === undefined ? undefined : e.unemployment - yearAgo;
  return (
    <>
      <h3 className="sub">{tr.t("ui-local-economy")}</h3>
      <div className="stats stats-2">
        <StatTile
          label={tr.t("ui-unemployment")}
          value={formatPct(e.unemployment, 1)}
          delta={d === undefined ? undefined : { text: `${d >= 0 ? "+" : "−"}${formatPct(Math.abs(d), 1)}`, good: Math.abs(d) < 1e-4 ? null : d < 0 }}
          trend={hist}
          formatPoint={(v, ago) => `${formatPct(v, 1)} · ${tr.t("ui-months-ago", { n: ago })}`}
          trendLabel={tr.t("ui-trend-12m", { what: tr.t("ui-unemployment") })}
        />
        <StatTile label={tr.t("ui-typical-income")} value={formatMoney(e.typical_income)} />
        <StatTile label={tr.t("ui-rent")} value={formatMoney(e.monthly_rent)} />
        <StatTile label={tr.t("ui-home-price")} value={formatMoney(e.home_price)} />
      </div>
      {!e.detailed && <p className="note">{tr.t("ui-detail-at-town-hall")}</p>}
      {e.detailed && (
        <>
      <h3 className="sub">{tr.t("ui-jobs-by-sector")}</h3>
      <ul className="meters">
        {e.sectors.map((x) => (
          <li key={x.id}>
            <span>{tr.t(x.name_key)}</span>
            <span className="meter" aria-hidden="true">
              <span className="meter-fill" style={{ width: `${(x.jobs / maxJobs) * 100}%` }} />
            </span>
            <span className="num">{formatCount(x.jobs)}</span>
          </li>
        ))}
      </ul>
      <h3 className="sub">{tr.t("ui-local-budget")}</h3>
      <div className="stats stats-3">
        <StatTile label={tr.t("ui-budget")} value={formatMoney(e.budget)} />
        <StatTile label={tr.t("ui-local-debt")} value={formatMoney(e.debt)} />
        <StatTile label={tr.t("ui-last-balance")} value={formatMoney(e.last_balance)} />
      </div>
        </>
      )}
    </>
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
          {view.top_concerns.length > 0 && (
            <p className="muted" data-testid="town-top-concerns">
              {tr.t("ui-town-concerns", { list: view.top_concerns.map((c) => tr.t(c.name_key)).join(", ") })}
            </p>
          )}
          {view.communities.length > 1 && (
            <p className="muted">
              {tr.t("ui-communities")}: {view.communities.map((c) => `${tr.t(c.name_key)} ${formatPct(c.share, 0)}`).join(" · ")}
            </p>
          )}
          {view.economy && <LocalEconomy e={view.economy} />}
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
