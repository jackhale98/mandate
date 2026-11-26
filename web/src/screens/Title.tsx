import { useEffect, useState } from "react";
import { useGame } from "../state/gameStore";
import { useReport } from "../state/reportStore";
import type { RunModeDto } from "../engine/protocol/RunModeDto";
import type { WorldSizeDto } from "../engine/protocol/WorldSizeDto";
import { formatDate } from "../i18n/format";
import { buildId } from "../components/report/buildReport";
import { Loader, LogoMark } from "../components/Brand";
import type { Translator } from "../i18n/fluent";
import type { RunPlayer } from "../persistence/saveStore";
import { ExportRunButton, ImportRun, TransferStatus, type TransferNotice } from "../components/RunTransfer";

/** Whose run it is, as the pocket's header says it: name, post, home (the post's word and the home come from the engine's view of the save). */
export function runWho(tr: Translator, p: RunPlayer): string {
  return [p.name, tr.t(p.roleKey), p.home].join(" · ");
}

export function Title() {
  const tr = useGame((s) => s.tr);
  const runs = useGame((s) => s.runs);
  const busy = useGame((s) => s.busy);
  const newGame = useGame((s) => s.newGame);
  const draftGame = useGame((s) => s.draftGame);
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
  const openReport = useReport((s) => s.openReport);

  const opts = () => {
    const trimmed = seed.trim();
    return { size, mode, scenario: scenario === "" ? null : scenario, storyteller: chosenTeller, localKnowledge: local, ...(/^\d+$/.test(trimmed) ? { seed: trimmed } : {}) };
  };
  // Begin: straight in, in the largest town (the quick start). Choose an opening: the draft (DEC-101 §3).
  const start = () => void newGame(opts());
  const startDraft = () => void draftGame(opts());

  const hasRuns = runs.length > 0;
  // Runs saved before names were kept gain them once, read from each save (a failed one keeps its old row).
  const fillRunNames = useGame((s) => s.fillRunNames);
  const missing = runs.filter((r) => !r.player).map((r) => r.runId).join(",");
  useEffect(() => {
    if (missing) void fillRunNames();
  }, [missing, fillRunNames]);
  const newCareer = (
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
      <button type="button" className={hasRuns ? "btn btn-block" : "btn btn-primary btn-block"} disabled={busy} onClick={startDraft} data-testid="draft-start">
        {tr.t("ui-draft-start")}
      </button>
      <button type="button" className="btn btn-block" disabled={busy} onClick={start} aria-describedby="start-help">
        {tr.t("ui-start")}
      </button>
      <p className="hint" id="start-help">
        {tr.t("ui-start-help")}
      </p>
      {progress && (
        <div className="progress" role="status" aria-live="polite">
          <Loader compact decorative />
          <span>{tr.t(progress.stage)}</span>
          <progress max={1} value={progress.fraction} />
        </div>
      )}
    </section>
  );
  const runsSection = (
    <section className="card" aria-labelledby="runs">
      <h2 id="runs">{tr.t("ui-saved-runs")}</h2>
      {runs.length === 0 ? (
        <p className="muted">{tr.t("ui-no-runs")}</p>
      ) : (
        <ul className="run-list">
          {runs.map((r, i) => (
            <li key={r.runId} className="run" data-testid="run-row">
              <div className="run-info">
                {r.player ? (
                  <>
                    <strong data-testid="run-who">{runWho(tr, r.player)}</strong>
                    <span className="muted small">{tr.t("ui-mode-label", { mode: r.mode })}</span>
                  </>
                ) : (
                  <strong>
                    {r.name ? `${r.name} · ` : ""}
                    {tr.t("ui-mode-label", { mode: r.mode })}
                  </strong>
                )}
                {r.storyteller && <span className="muted small">{tr.t("ui-storyteller-label", { name: tr.t(r.storyteller) })}</span>}
                <span className="muted">{tr.t("ui-run-turn", { turn: r.meta.turn, date: formatDate(tr, r.meta.date) })}</span>
              </div>
              <div className="run-actions">
                <button type="button" className={i === 0 ? "btn btn-primary" : "btn"} disabled={busy} onClick={() => void continueRun(r.runId)}>
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
  );

  return (
    <main className="title">
      <header className="title-hero">
        <LogoMark size={72} />
        <h1 className="wordmark">{tr.t("app-title")}</h1>
        <p className="tagline">{tr.t("app-tagline")}</p>
      </header>

      {hasRuns ? (
        <>
          {runsSection}
          {newCareer}
        </>
      ) : (
        <>
          {newCareer}
          {runsSection}
        </>
      )}
      <footer className="foot muted">
        <p className="fiction">{tr.t("app-fiction")}</p>
        <span className="small build-id" data-testid="build-id" title={tr.t("ui-build-id-hint")}>
          {tr.t("ui-build-id", { id: buildId(__APP_VERSION__) })}
        </span>{" "}
        <a className="btn btn-ghost btn-sm" href={`${import.meta.env.BASE_URL}../privacy.html`} target="_blank" rel="noopener" title={tr.t("ui-privacy-note")} data-testid="privacy-link-title">
          {tr.t("ui-privacy")}
        </a>{" "}
        {/* After a crash and a reload, testers land here: the report takes the latest saved game. */}
        <button type="button" className="btn btn-ghost btn-sm" onClick={() => openReport("title")} data-testid="report-problem-title">
          {tr.t("ui-report")}
        </button>
      </footer>
    </main>
  );
}
