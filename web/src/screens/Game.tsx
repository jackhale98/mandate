import { useEffect, useState } from "react";
import { useGame } from "../state/gameStore";
import { formatCount, formatDate, formatMoney } from "../i18n/format";
import { WorldMap } from "../map/WorldMap";
import { MunicipalityPanel } from "../components/MunicipalityPanel";
import { NationalEconomy } from "../components/NationalEconomy";
import type { Overlay } from "../map/render";
import { ElectionNight } from "../components/CampaignCards";
import { Toast } from "../components/Toast";
import { Pocket } from "../components/Pocket";
import { Walking } from "../components/Walking";
import { Town } from "./Town";
import { PlacePanel } from "./PlacePanel";
import { CharacterSheet } from "../components/CharacterSheet";
import { DecisionSheet } from "../components/DecisionSheet";
import { StorySheet } from "../components/StorySheet";
import { placeIcon, placeName } from "../places/labels";
import { Loader } from "../components/Brand";

function MapScreen({ onSelect, selected }: { onSelect: (id: number | null) => void; selected: number | null }) {
  const tr = useGame((s) => s.tr);
  const map = useGame((s) => s.map);
  const homeId = useGame((s) => s.office?.player.municipality_id ?? null);
  const seed = useGame((s) => s.overview?.seed ?? "");
  const home = map?.municipalities.find((m) => m.id === homeId);
  const [overlay, setOverlay] = useState<Overlay>("terrain");
  return (
    <main className="map-body">
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
    </main>
  );
}

function OverviewScreen({ onSelect }: { onSelect: (id: number) => void }) {
  const tr = useGame((s) => s.tr);
  const overview = useGame((s) => s.overview);
  const map = useGame((s) => s.map);
  const homeId = useGame((s) => s.office?.player.municipality_id ?? null);
  const homeRegion = map?.municipalities.find((m) => m.id === homeId)?.region;
  const [openRegion, setOpenRegion] = useState<number | null>(null);
  if (!overview) return null;
  return (
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
                <span>
                  {r.name}
                  {r.id === homeRegion && <span className="you-chip">⚑ {tr.t("ui-you")}</span>}
                </span>
                <span className="num">{formatCount(r.population)}</span>
              </button>
              {openRegion === r.id && map && (
                <ul className="town-list">
                  {map.municipalities
                    .filter((m) => m.region === r.id)
                    .map((m) => (
                      <li key={m.id} className={m.id === homeId ? "town-home" : undefined}>
                        <button type="button" className="row-btn" onClick={() => onSelect(m.id)} aria-current={m.id === homeId ? "true" : undefined}>
                          <span>
                            {m.name} <span className="muted">· {tr.t(`class-${m.class}`)}</span>
                            {m.id === homeId && (
                              <span className="you-chip" data-testid="town-list-home">
                                ⚑ {tr.t("ui-you")}
                              </span>
                            )}
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
  );
}

export function Game() {
  const tr = useGame((s) => s.tr);
  const overview = useGame((s) => s.overview);
  const map = useGame((s) => s.map);
  const loadMap = useGame((s) => s.loadMap);
  const busy = useGame((s) => s.busy);
  const endTurn = useGame((s) => s.endTurn);
  const run = useGame((s) => s.run);
  const pocket = useGame((s) => s.pocket);
  const refreshDesk = useGame((s) => s.refreshDesk);
  const view = useGame((s) => s.view);
  const setView = useGame((s) => s.setView);
  const pocketOpen = useGame((s) => s.pocketOpen);
  const setPocketOpen = useGame((s) => s.setPocketOpen);
  const [selected, setSelected] = useState<number | null>(null);
  const panelOpen = useGame((s) => s.openPlace !== null);

  useEffect(() => {
    if (!map) void loadMap();
  }, [map, loadMap]);
  useEffect(() => {
    if (!pocket) void refreshDesk();
  }, [pocket, refreshDesk]);

  if (!overview) return null;

  return (
    <div className={`game${panelOpen && view === "town" ? " with-panel" : ""}${pocketOpen ? " with-pocket" : ""}`}>
      <header className="hud">
        <div className="hud-date">
          <strong>{formatDate(tr, overview.date)}</strong>
          <span className="muted">
            {tr.t("ui-turn", { turn: overview.turn })} · {tr.t("ui-turn-span", { days: overview.turn_days })}
          </span>
        </div>
        <div className="hud-right">
          {pocket && (
            <span className="chip" data-testid="hud-slots" aria-label={tr.t("ui-slots", { left: pocket.slots_left })}>
              {"●".repeat(pocket.slots_left)}
              {"○".repeat(Math.max(0, pocket.slots_total - pocket.slots_left))}
              {pocket.late_night_available ? " ☾" : ""}
            </span>
          )}
          {pocket && <span className="chip chip-muted hud-money">{formatMoney(pocket.money)}</span>}
          {pocket?.in_campaign && <span className="chip chip-warn">{tr.t("ui-campaign-season")}</span>}
          <span className="chip chip-muted hud-mode">{run ? tr.t("ui-mode-label", { mode: run.mode }) : ""}</span>
        </div>
      </header>

      {view !== "town" && (
        <div className="viewbar">
          <button type="button" className="btn btn-sm btn-ghost" onClick={() => setView("town")}>
            ← {tr.t("ui-back-to-town")}
          </button>
          <strong>{tr.t(view === "map" ? "ui-map" : "ui-overview")}</strong>
        </div>
      )}

      {view === "town" ? <Town /> : view === "map" ? <MapScreen selected={selected} onSelect={setSelected} /> : <OverviewScreen onSelect={setSelected} />}

      <Toast />
      <ElectionNight />
      <PlacePanel />
      <Pocket />
      <StorySheet />
      <DecisionSheet />
      <CharacterSheet />
      <Walking />
      {selected !== null && <MunicipalityPanel id={selected} onClose={() => setSelected(null)} />}

      <footer className="actionbar">
        <button type="button" className="btn pocket-btn" aria-expanded={pocketOpen} onClick={() => setPocketOpen(!pocketOpen)} data-testid="pocket-button">
          <span aria-hidden="true">📒 </span>
          {tr.t("ui-pocket")}
          {pocket && pocket.decisions.length > 0 && (
            <span className="badge badge-decisions" data-testid="decision-badge">
              {tr.t("ui-decision-count", { n: pocket.decisions.length })}
            </span>
          )}
          {pocket && pocket.alerts.length > 0 && <span className="badge">{tr.t("ui-alert-count", { n: pocket.alerts.length })}</span>}
        </button>
        {pocket && (
          <span className="where small" data-testid="where">
            <span aria-hidden="true">{placeIcon(pocket.location.kind_key)} </span>
            {placeName(tr, pocket.location)}
          </span>
        )}
        <button type="button" className="btn btn-primary" disabled={busy} onClick={() => void endTurn()}>
          {tr.t("ui-end-turn")}
        </button>
      </footer>
    </div>
  );
}
