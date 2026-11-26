// Start from a world code (M6 note 23), on the title screen: paste a code from
// someone's career card, read it (the engine says what it would start, and
// whether it was made under another build), then begin that world. The
// engine reads the code; this screen shows what it says.
import { useState } from "react";
import type { RunModeDto } from "../../engine/protocol/RunModeDto";
import type { WorldCodeCheckView } from "../../engine/protocol/WorldCodeCheckView";
import { useGame } from "../../state/gameStore";
import "./card.css";

export function CodeStart({ mode }: { mode: RunModeDto }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const readCode = useGame((s) => s.readCode);
  const startFromCode = useGame((s) => s.startFromCode);
  const [code, setCode] = useState("");
  const [read, setRead] = useState<WorldCodeCheckView | null>(null);
  const w = read?.world ?? null;
  const check = async () => {
    if (!code.trim()) return;
    setRead(await readCode(code));
  };
  return (
    <section className="card code-start" aria-labelledby="code-start" data-testid="code-start">
      <h2 id="code-start">{tr.t("ui-code-start")}</h2>
      <label className="field">
        <span>{tr.t("ui-code-label")}</span>
        <input
          className="input"
          value={code}
          autoCapitalize="characters"
          autoCorrect="off"
          spellCheck={false}
          onChange={(e) => {
            setCode(e.target.value);
            setRead(null);
          }}
          placeholder="M1.…"
          data-testid="code-input"
        />
      </label>
      <div aria-live="polite" data-testid="code-result">
        {read?.error && <p className="hint">{tr.t(`ui-code-error-${read.error}`)}</p>}
        {w && (
          <>
            <p>{tr.t("ui-code-world", { size: tr.t(`ui-card-size-${w.size}`), scenario: w.scenario_key ? tr.t(w.scenario_key) : tr.t("ui-card-world-random") })}</p>
            <p>{w.opening ? tr.t("ui-code-opening", { arc: tr.t(w.opening.title_key).replace(/\{.*?\}/g, "").trim(), ahead: tr.t("ui-card-opening-ahead", { n: w.opening.months_ahead }) }) : tr.t("ui-code-opening-none")}</p>
            {w.local_knowledge && <p className="muted small">{tr.t("ui-code-knowledge")}</p>}
            {!w.same_build && (
              <p className="hint" data-testid="code-other-build">
                {tr.t("ui-code-other-build")}
              </p>
            )}
          </>
        )}
      </div>
      <div className="code-start-row">
        <button type="button" className="btn" disabled={busy || !code.trim()} onClick={() => void check()} data-testid="code-read">
          {tr.t("ui-code-read")}
        </button>
        <button type="button" className="btn btn-primary" disabled={busy || !w} onClick={() => w && void startFromCode(w.code, mode)} data-testid="code-begin">
          {tr.t("ui-code-begin")}
        </button>
      </div>
    </section>
  );
}
