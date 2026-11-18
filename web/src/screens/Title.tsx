import { useState } from "react";
import { useGame } from "../state/gameStore";
import type { RunModeDto } from "../engine/protocol/RunModeDto";
import type { WorldSizeDto } from "../engine/protocol/WorldSizeDto";
import type { PresetDto } from "../engine/protocol/PresetDto";

const PRESETS: PresetDto[] = ["industrial_decline", "wealthy_coast", "tech_growth", "drought_farmland", "resource_frontier"];
import { formatDate } from "../i18n/format";

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
  const [preset, setPreset] = useState<PresetDto | "">("");
  const progress = useGame((s) => s.progress);

  const start = () => {
    const trimmed = seed.trim();
    void newGame({ size, mode, preset: preset === "" ? null : preset, ...(/^\d+$/.test(trimmed) ? { seed: trimmed } : {}) });
  };

  return (
    <main className="title">
      <header className="title-hero">
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
        <label className="field">
          <span>{tr.t("ui-scenario")}</span>
          <select className="input" value={preset} onChange={(e) => setPreset(e.target.value as PresetDto | "")}>
            <option value="">{tr.t("ui-scenario-random")}</option>
            {PRESETS.map((p) => (
              <option key={p} value={p}>
                {tr.t(`preset-${p}`)}
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
                  <strong>{tr.t("ui-mode-label", { mode: r.mode })}</strong>
                  <span className="muted">
                    {tr.t("ui-run-turn", { turn: r.meta.turn, date: formatDate(tr, r.meta.date) })}
                  </span>
                </div>
                <div className="run-actions">
                  <button type="button" className="btn btn-primary" disabled={busy} onClick={() => void continueRun(r.runId)}>
                    {tr.t("ui-continue")}
                  </button>
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
      </section>
      <footer className="foot muted">v{__APP_VERSION__}</footer>
    </main>
  );
}
