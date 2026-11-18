import { useEffect, useState } from "react";
import { useGame } from "../state/gameStore";
import { formatCount, formatDate } from "../i18n/format";
import { WorldMap } from "../map/WorldMap";
import { MunicipalityPanel } from "../components/MunicipalityPanel";
import { NationalEconomy } from "../components/NationalEconomy";
import type { Overlay } from "../map/render";
import { Office } from "./Office";
import { Council } from "./Council";
import { Campaign, ElectionNight } from "./Campaign";
import { Toast } from "../components/Toast";

type Tab = "office" | "council" | "campaign" | "map" | "overview";

export function Game() {
  const tr = useGame((s) => s.tr);
  const overview = useGame((s) => s.overview);
  const map = useGame((s) => s.map);
  const loadMap = useGame((s) => s.loadMap);
  const busy = useGame((s) => s.busy);
  const endTurn = useGame((s) => s.endTurn);
  const saveAndQuit = useGame((s) => s.saveAndQuit);
  const run = useGame((s) => s.run);
  const office = useGame((s) => s.office);
  const loadOffice = useGame((s) => s.loadOffice);
  const campaign = useGame((s) => s.campaign);
  const loadCampaign = useGame((s) => s.loadCampaign);
  const [tab, setTab] = useState<Tab>("office");
  const [selected, setSelected] = useState<number | null>(null);
  const [openRegion, setOpenRegion] = useState<number | null>(null);
  const [overlay, setOverlay] = useState<Overlay>("terrain");

  useEffect(() => {
    if (!map) void loadMap();
  }, [map, loadMap]);
  useEffect(() => {
    if (!office) void loadOffice();
  }, [office, loadOffice]);
  useEffect(() => {
    if (!campaign) void loadCampaign();
  }, [campaign, loadCampaign]);

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
        <div className="hud-right">
          {office && (
            <span className="chip" data-testid="hud-slots" aria-label={tr.t("ui-slots", { left: office.player.slots_left })}>
              {"●".repeat(office.player.slots_left)}
              {"○".repeat(Math.max(0, office.player.slots_total - office.player.slots_left))}
              {office.player.late_night_available ? " ☾" : ""}
            </span>
          )}
          {campaign?.in_campaign && <span className="chip chip-warn">{tr.t("ui-campaign-season")}</span>}
          <span className="chip chip-muted">{run ? tr.t("ui-mode-label", { mode: run.mode }) : ""}</span>
        </div>
      </header>

      <nav className="tabs" role="tablist">
        {(["office", "council", "campaign", "map", "overview"] as const).map((t) => (
          <button key={t} role="tab" type="button" aria-selected={tab === t} className={tab === t ? "tab tab-on" : "tab"} onClick={() => setTab(t)}>
            {tr.t(`ui-${t}`)}
          </button>
        ))}
      </nav>

      {tab === "office" ? (
        <Office />
      ) : tab === "council" ? (
        <Council />
      ) : tab === "campaign" ? (
        <Campaign />
      ) : tab === "map" ? (
        <main className="map-body">
          {map ? (
            <WorldMap view={map} selected={selected} onSelect={setSelected} label={tr.t("ui-map-help")} overlay={overlay} />
          ) : (
            <div className="center">
              <div className="spinner" aria-hidden="true" />
            </div>
          )}
          <div className="map-tools">
            <div className="segmented segmented-sm" role="radiogroup" aria-label={tr.t("ui-overlay")}>
              {(["terrain", "unemployment"] as const).map((o) => (
                <label key={o} className={overlay === o ? "seg seg-on" : "seg"}>
                  <input type="radio" name="overlay" checked={overlay === o} onChange={() => setOverlay(o)} />
                  {tr.t(`ui-overlay-${o}`)}
                </label>
              ))}
            </div>
            {overlay === "unemployment" && (
              <div className="legend" aria-hidden="true">
                <span>{tr.t("ui-legend-low")} 2%</span>
                <span className="legend-ramp" />
                <span>15% {tr.t("ui-legend-high")}</span>
              </div>
            )}
          </div>
          <p className="map-help muted">{tr.t("ui-map-help")}</p>
        </main>
      ) : (
        <main className="game-body">
          <NationalEconomy e={overview.economy} />
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
                  <span>{tr.event(e.text_key, e.args)}</span>
                </li>
              ))}
            </ol>
          </section>
          <p className="muted tiny">
            seed {overview.seed} · state {overview.state_hash}
          </p>
        </main>
      )}

      <Toast />
      <ElectionNight />
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
