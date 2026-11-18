import { useEffect, useState } from "react";
import { useGame } from "../state/gameStore";
import { formatCount, formatDate } from "../i18n/format";
import { WorldMap } from "../map/WorldMap";
import { MunicipalityPanel } from "../components/MunicipalityPanel";

type Tab = "map" | "overview";

export function Game() {
  const tr = useGame((s) => s.tr);
  const overview = useGame((s) => s.overview);
  const map = useGame((s) => s.map);
  const loadMap = useGame((s) => s.loadMap);
  const busy = useGame((s) => s.busy);
  const endTurn = useGame((s) => s.endTurn);
  const saveAndQuit = useGame((s) => s.saveAndQuit);
  const run = useGame((s) => s.run);
  const [tab, setTab] = useState<Tab>("map");
  const [selected, setSelected] = useState<number | null>(null);
  const [openRegion, setOpenRegion] = useState<number | null>(null);

  useEffect(() => {
    if (!map) void loadMap();
  }, [map, loadMap]);

  if (!overview) return null;

  return (
    <div className="game">
      <header className="hud">
        <div className="hud-date">
          <strong>{formatDate(tr, overview.date)}</strong>
          <span className="muted">
            {tr.t("ui-turn", { turn: overview.turn })} · {tr.t("ui-turn-span", { days: overview.turn_days })}
          </span>
        </div>
        <span className="chip">{run ? tr.t("ui-mode-label", { mode: run.mode }) : ""}</span>
      </header>

      <nav className="tabs" role="tablist">
        {(["map", "overview"] as const).map((t) => (
          <button key={t} role="tab" type="button" aria-selected={tab === t} className={tab === t ? "tab tab-on" : "tab"} onClick={() => setTab(t)}>
            {tr.t(`ui-${t}`)}
          </button>
        ))}
      </nav>

      {tab === "map" ? (
        <main className="map-body">
          {map ? (
            <WorldMap view={map} selected={selected} onSelect={setSelected} label={tr.t("ui-map-help")} />
          ) : (
            <div className="center">
              <div className="spinner" aria-hidden="true" />
            </div>
          )}
          <p className="map-help muted">{tr.t("ui-map-help")}</p>
        </main>
      ) : (
        <main className="game-body">
          <section className="card">
            <h2>
              {tr.t("ui-population")}: {formatCount(overview.total_population)}
            </h2>
            <h3 className="sub">{tr.t("ui-regions")}</h3>
            <ul className="region-list region-accordion">
              {overview.regions.map((r) => (
                <li key={r.id}>
                  <button type="button" className="row-btn" aria-expanded={openRegion === r.id} onClick={() => setOpenRegion(openRegion === r.id ? null : r.id)}>
                    <span>{r.name}</span>
                    <span className="num">{formatCount(r.population)}</span>
                  </button>
                  {openRegion === r.id && map && (
                    <ul className="town-list">
                      {map.municipalities
                        .filter((m) => m.region === r.id)
                        .map((m) => (
                          <li key={m.id}>
                            <button type="button" className="row-btn" onClick={() => setSelected(m.id)}>
                              <span>
                                {m.name} <span className="muted">· {tr.t(`class-${m.class}`)}</span>
                              </span>
                              <span className="num">{formatCount(m.population)}</span>
                            </button>
                          </li>
                        ))}
                    </ul>
                  )}
                </li>
              ))}
            </ul>
          </section>
          <section className="card" aria-live="polite">
            <h2>{tr.t("ui-recent")}</h2>
            <ol className="events">
              {overview.recent_events.map((e) => (
                <li key={e.id}>
                  <time className="muted">{formatDate(tr, e.date)}</time>
                  <span>{tr.t(e.text_key, e.args)}</span>
                </li>
              ))}
            </ol>
          </section>
          <p className="muted tiny">
            seed {overview.seed} · state {overview.state_hash}
          </p>
        </main>
      )}

      {selected !== null && <MunicipalityPanel id={selected} onClose={() => setSelected(null)} />}

      <footer className="actionbar">
        <button type="button" className="btn btn-ghost" disabled={busy} onClick={() => void saveAndQuit()}>
          {tr.t("ui-save-quit")}
        </button>
        <button type="button" className="btn btn-primary" disabled={busy} onClick={() => void endTurn()}>
          {tr.t("ui-end-turn")}
        </button>
      </footer>
    </div>
  );
}
