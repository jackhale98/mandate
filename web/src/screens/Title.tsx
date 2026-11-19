import { useState } from "react";
import { useGame } from "../state/gameStore";
import type { RunModeDto } from "../engine/protocol/RunModeDto";
import type { WorldSizeDto } from "../engine/protocol/WorldSizeDto";
import { formatDate } from "../i18n/format";
import { Loader, LogoMark } from "../components/Brand";
import { ExportRunButton, ImportRun, TransferStatus, type TransferNotice } from "../components/RunTransfer";

export function Title() {
  const tr = useGame((s) => s.tr);
  const runs = useGame((s) => s.runs);
  const busy = useGame((s) => s.busy);
  const newGame = useGame((s) => s.newGame);
  const continueRun = useGame((s) => s.continueRun);
  const deleteRun = useGame((s) => s.deleteRun);
  const [size, setSize] = useState<WorldSizeDto>("medium");
  const [mode, setMode] = useState<RunModeDto>("ironman");
  const [seed, setSeed] = useState("");
  // "Start with local knowledge" (DEC-091): off by default (owner).
  const [local, setLocal] = useState(false);
  const scenarios = useGame((s) => s.scenarios);
  const [scenario, setScenario] = useState("");
  const storytellers = useGame((s) => s.storytellers);
  const [teller, setTeller] = useState<string | null>(null);
  const chosenTeller = teller ?? storytellers.find((s) => s.default)?.id ?? storytellers[0]?.id ?? null;
  const tellerDesc = storytellers.find((s) => s.id === chosenTeller)?.desc_key;
  const progress = useGame((s) => s.progress);
  const [notice, setNotice] = useState<TransferNotice | null>(null);

  const start = () => {
    const trimmed = seed.trim();
    void newGame({ size, mode, scenario: scenario === "" ? null : scenario, storyteller: chosenTeller, localKnowledge: local, ...(/^\d+$/.test(trimmed) ? { seed: trimmed } : {}) });
  };

  return (
    <main className="title">
      <header className="title-hero">
        <LogoMark size={72} />
        <h1 className="wordmark">{tr.t("app-title")}</h1>
        <p className="tagline">{tr.t("app-tagline")}</p>
      </header>

      <section className="card" aria-labelledby="new-run">
        <h2 id="new-run">{tr.t("ui-new-game")}</h2>
        <fieldset className="field">
          <legend>{tr.t("ui-world-size")}</legend>
          <div className="segmented" role="radiogroup">
            {(["small", "medium", "large"] as const).map((s) => (
              <label key={s} className={size === s ? "seg seg-on" : "seg"}>
                <input type="radio" name="size" value={s} checked={size === s} onChange={() => setSize(s)} />
                {tr.t(`ui-size-${s}`)}
              </label>
            ))}
          </div>
        </fieldset>
        <fieldset className="field">
          <legend>{tr.t("ui-mode")}</legend>
          <div className="segmented" role="radiogroup">
            {(["ironman", "story"] as const).map((m) => (
              <label key={m} className={mode === m ? "seg seg-on" : "seg"}>
                <input type="radio" name="mode" value={m} checked={mode === m} onChange={() => setMode(m)} />
                {tr.t(`ui-mode-${m}`)}
              </label>
            ))}
          </div>
          <p className="hint">{tr.t(`ui-mode-${mode}-help`)}</p>
        </fieldset>
        {storytellers.length > 0 && (
          <fieldset className="field" data-testid="storyteller-picker">
            <legend>{tr.t("ui-storyteller")}</legend>
            <div className="segmented" role="radiogroup">
              {storytellers.map((s) => (
                <label key={s.id} className={chosenTeller === s.id ? "seg seg-on" : "seg"}>
                  <input type="radio" name="storyteller" value={s.id} checked={chosenTeller === s.id} onChange={() => setTeller(s.id)} />
                  {tr.t(s.name_key)}
                </label>
              ))}
            </div>
            {tellerDesc && <p className="hint">{tr.t(tellerDesc)}</p>}
          </fieldset>
        )}
        <div className="field local-knowledge" data-testid="local-knowledge">
          <label className="check">
            <input type="checkbox" checked={local} onChange={(e) => setLocal(e.target.checked)} aria-describedby="local-knowledge-help" data-testid="local-knowledge-toggle" />
            <span>{tr.t("ui-local-knowledge")}</span>
          </label>
          <p className="hint" id="local-knowledge-help">
            {tr.t("ui-local-knowledge-help")}
          </p>
        </div>
        <label className="field">
          <span>{tr.t("ui-scenario")}</span>
          <select className="input" value={scenario} onChange={(e) => setScenario(e.target.value)}>
            <option value="">{tr.t("ui-scenario-random")}</option>
            {scenarios.map((s) => (
              <option key={s.id} value={s.id}>
                {tr.t(s.name_key)}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span>{tr.t("ui-seed")}</span>
          <input
            className="input"
            inputMode="numeric"
            value={seed}
            onChange={(e) => setSeed(e.target.value)}
            placeholder="e.g. 20260"
          />
        </label>
        <button type="button" className="btn btn-primary btn-block" disabled={busy} onClick={start}>
          {tr.t("ui-start")}
        </button>
        {progress && (
          <div className="progress" role="status" aria-live="polite">
            <Loader compact decorative />
            <span>{tr.t(progress.stage)}</span>
            <progress max={1} value={progress.fraction} />
          </div>
        )}
      </section>

      <section className="card" aria-labelledby="runs">
        <h2 id="runs">{tr.t("ui-saved-runs")}</h2>
        {runs.length === 0 ? (
          <p className="muted">{tr.t("ui-no-runs")}</p>
        ) : (
          <ul className="run-list">
            {runs.map((r) => (
              <li key={r.runId} className="run">
                <div className="run-info">
                  <strong>
                    {r.name ? `${r.name} · ` : ""}
                    {tr.t("ui-mode-label", { mode: r.mode })}
                  </strong>
                  {r.storyteller && <span className="muted small">{tr.t("ui-storyteller-label", { name: tr.t(r.storyteller) })}</span>}
                  <span className="muted">
                    {tr.t("ui-run-turn", { turn: r.meta.turn, date: formatDate(tr, r.meta.date) })}
                  </span>
                </div>
                <div className="run-actions">
                  <button type="button" className="btn btn-primary" disabled={busy} onClick={() => void continueRun(r.runId)}>
                    {tr.t("ui-continue")}
                  </button>
                  <ExportRunButton run={r} onNotice={setNotice} />
                  <button
                    type="button"
                    className="btn btn-ghost"
                    disabled={busy}
                    onClick={() => {
                      if (window.confirm(tr.t("ui-delete-confirm"))) void deleteRun(r.runId);
                    }}
                  >
                    {tr.t("ui-delete")}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
        <ImportRun onNotice={setNotice} />
        <TransferStatus notice={notice} />
      </section>
      <footer className="foot muted">v{__APP_VERSION__}</footer>
    </main>
  );
}
