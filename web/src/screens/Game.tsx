import { useGame } from "../state/gameStore";
import { formatCount, formatDate } from "../i18n/format";

export function Game() {
  const tr = useGame((s) => s.tr);
  const overview = useGame((s) => s.overview);
  const busy = useGame((s) => s.busy);
  const endTurn = useGame((s) => s.endTurn);
  const saveAndQuit = useGame((s) => s.saveAndQuit);
  const run = useGame((s) => s.run);
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

      <main className="game-body">
        <p className="note">{tr.t("ui-foundation-note")}</p>
        <section className="card">
          <h2>
            {tr.t("ui-population")}: {formatCount(overview.total_population)}
          </h2>
          <h3 className="sub">{tr.t("ui-regions")}</h3>
          <ul className="region-list">
            {overview.regions.map((r) => (
              <li key={r.id}>
                <span>{r.name}</span>
                <span className="num">{formatCount(r.population)}</span>
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
