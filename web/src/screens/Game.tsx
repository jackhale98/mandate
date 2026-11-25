import { useEffect, useState } from "react";
import { useGame } from "../state/gameStore";
import { formatCount, formatDate } from "../i18n/format";
import { MunicipalityPanel } from "../components/MunicipalityPanel";
import { NationalEconomy } from "../components/NationalEconomy";
import { ElectionNight } from "../components/CampaignCards";
import { Toast } from "../components/Toast";
import { Pocket } from "../components/Pocket";
import { Walking } from "../components/Walking";
import { TurnPassage } from "../components/TurnPassage";
import { Town } from "./Town";
import { PlacePanel } from "./PlacePanel";
import { CharacterSheet } from "../components/CharacterSheet";
import { DecisionSheet } from "../components/DecisionSheet";
import { StorySheet } from "../components/StorySheet";
import { ArcSheet } from "../components/ArcSheet";
import { RunBanner } from "../components/Newspaper";
import { TalkSheet } from "../components/talk/TalkSheet";
import { TermReview } from "../components/act/TermReview";
import { SkipSummary } from "../components/act/SkipSummary";
import { Epilogue } from "../components/act/Epilogue";
import { PromisePicker } from "../components/asks/PromisePicker";
import { WhyPanel } from "../components/why/WhyPanel";
import { WhyButton } from "../components/why/WhyButton";

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
              <WhyButton event={e.id} about={tr.event(e.text_key, e.args)} inline />
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

/** A count on the pocket button: an icon and the number on screen (not colour alone), the words for screen readers and on hover. */
function Badge({ icon, n, text, cls, testId }: { icon: string; n: number; text: string; cls: string; testId: string }) {
  return (
    <span className={`badge ${cls}`} data-testid={testId} title={text}>
      <span aria-hidden="true">
        {icon}
        {n}
      </span>
      <span className="visually-hidden">{text}</span>
    </span>
  );
}

/**
 * The game screen (DEC-088 main screen): the town's street full screen with a
 * slim bar (the map, the date and slots, the pocket, End turn). The map
 * overlay (../screens/Town) shows the town or the country; the overview is
 * reached from the pocket and comes back to the town. A "Skip to Places" link and landmarks reach the pocket and every
 * place without the canvas (DEC-051).
 */
export function Game() {
  const tr = useGame((s) => s.tr);
  const overview = useGame((s) => s.overview);
  const map = useGame((s) => s.map);
  const loadMap = useGame((s) => s.loadMap);
  const busy = useGame((s) => s.busy);
  const endTurn = useGame((s) => s.endTurn);
  const pocket = useGame((s) => s.pocket);
  const refreshDesk = useGame((s) => s.refreshDesk);
  const view = useGame((s) => s.view);
  const setView = useGame((s) => s.setView);
  const pocketOpen = useGame((s) => s.pocketOpen);
  const setPocketOpen = useGame((s) => s.setPocketOpen);
  const openPocket = useGame((s) => s.openPocket);
  const docked = useGame((s) => s.pocketDocked);
  const setTownMapOpen = useGame((s) => s.setTownMapOpen);
  const [selected, setSelected] = useState<number | null>(null);
  const panelOpen = useGame((s) => s.openPlace !== null);
  const openLegacy = useGame((s) => s.openLegacy);

  useEffect(() => {
    if (!map) void loadMap();
  }, [map, loadMap]);
  useEffect(() => {
    if (!pocket) void refreshDesk();
  }, [pocket, refreshDesk]);

  if (!overview) return null;

  const cls = ["game", panelOpen && view === "town" ? "with-panel" : "", pocketOpen ? "with-pocket" : "", pocketOpen && docked ? "with-pocket-docked" : ""].filter(Boolean).join(" ");
  const notebookNew = pocket?.notebook_new ?? 0;
  // The bar's two counts (DEC-113, owner's answer 5): what needs you (the engine's count) and what is new in the notebook.
  const needs = pocket?.needs_count ?? 0;
  return (
    <div className={cls}>
      <a
        href="#notebook-panel-places"
        className="skip-link"
        data-testid="skip-to-places"
        onClick={(e) => {
          e.preventDefault();
          openPocket("places", true);
        }}
      >
        {tr.t("ui-skip-places")}
      </a>

      {view !== "town" && (
        <div className="viewbar">
          <button type="button" className="btn btn-sm btn-ghost" onClick={() => setView("town")}>
            ← {tr.t("ui-back-to-town")}
          </button>
          <strong>{tr.t("ui-overview")}</strong>
        </div>
      )}

      {view === "town" ? <Town /> : <OverviewScreen onSelect={setSelected} />}

      <Toast />
      <ElectionNight />
      <PlacePanel />
      <Pocket />
      <StorySheet />
      <ArcSheet />
      <DecisionSheet />
      <CharacterSheet />
      <TalkSheet />
      <TermReview />
      <SkipSummary />
      <PromisePicker />
      <Epilogue />
      <WhyPanel />
      <Walking />
      <TurnPassage />
      {selected !== null && <MunicipalityPanel id={selected} onClose={() => setSelected(null)} />}

      {pocket?.legal && <RunBanner l={pocket.legal} />}

      <nav className="gamebar" aria-label={tr.t("ui-game-bar")} data-testid="gamebar">
        <button
          type="button"
          className="btn gamebar-map"
          onClick={() => {
            if (view !== "town") setView("town");
            setTownMapOpen(true);
          }}
          aria-label={tr.t("ui-town-map")}
          data-testid="town-map-button"
        >
          <span aria-hidden="true">🗺️</span>
          <span className="gamebar-label" aria-hidden="true">
            {tr.t("ui-town-map-open")}
          </span>
        </button>
        <div className="gamebar-date">
          <strong>
            <span className="gamebar-date-full">{formatDate(tr, overview.date)}</span>
            <span className="gamebar-date-short" aria-hidden="true">
              {tr.t("ui-date-short", { day: overview.date.day, month: tr.t(`month-short-${overview.date.month}`) })}
            </span>
          </strong>
          <span className="gamebar-sub muted">
            <span>{tr.t("ui-turn", { turn: overview.turn })}</span>
            <span className="gamebar-span"> · {tr.t("ui-turn-span", { days: overview.turn_days })}</span>
            {pocket && (
              <>
                {" "}
                {/* The separator belongs to the dots: when the line wraps they go down together, never "Turn 0 ·" on its own. */}
                <span className="gamebar-more">
                  {"· "}
                  <span className="gamebar-slots" data-testid="hud-slots" role="img" aria-label={tr.t("ui-slots", { left: pocket.slots_left })}>
                    {"●".repeat(pocket.slots_left)}
                    {"○".repeat(Math.max(0, pocket.slots_total - pocket.slots_left))}
                    {pocket.late_night_available ? " ☾" : ""}
                  </span>
                </span>
              </>
            )}
          </span>
        </div>
        <button type="button" className="btn pocket-btn" aria-expanded={pocketOpen} onClick={() => setPocketOpen(!pocketOpen)} data-testid="pocket-button">
          <span aria-hidden="true">📒</span>
          <span className="gamebar-label">{tr.t("ui-pocket")}</span>
          {needs > 0 && <Badge icon="❗" n={needs} text={tr.t("ui-needs-count", { n: needs })} cls="badge-decisions" testId="needs-badge" />}
          {notebookNew > 0 && <Badge icon="✎" n={notebookNew} text={tr.t("ui-notebook-new-count", { n: notebookNew })} cls="badge-notebook" testId="notebook-badge" />}
        </button>
        {overview.act?.ended_key ? (
          // The act is over (DEC-006): its epilogue, not another turn.
          <button type="button" className="btn btn-primary gamebar-end" onClick={() => void openLegacy(true)} data-testid="open-epilogue">
            {tr.t("ui-epilogue")}
          </button>
        ) : (
          <button type="button" className="btn btn-primary gamebar-end" disabled={busy || pocket?.legal?.run.status === "ended"} onClick={() => void endTurn()} data-testid="end-turn">
            {pocket?.legal?.run.status === "imprisoned" ? tr.t("ui-serve-sentence") : tr.t("ui-end-turn")}
          </button>
        )}
      </nav>
    </div>
  );
}
