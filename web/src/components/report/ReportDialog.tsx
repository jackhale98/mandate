// "Report a problem" (bug bundles for the M6 testers): says plainly what the
// file holds and that nothing is sent anywhere, takes an optional note, and
// saves the file (or offers the share sheet on phones). An accessible modal
// dialog (DEC-051): labelled, focus moves in and stays in, Escape closes, and
// focus goes back to where it was.
import { useEffect, useId, useRef, useState, type KeyboardEvent } from "react";
import { engine, freshEngine } from "../../engine/client";
import { saveStore } from "../../persistence/indexedDbStore";
import { useGame } from "../../state/gameStore";
import { useReport } from "../../state/reportStore";
import { assembleReport, buildId, deviceInfo, encodeReport, formatSize, gatherRun } from "./buildReport";
import { deliverFile } from "./deliver";
import "./report.css";

type Status = { kind: "idle" } | { kind: "working" } | { kind: "done"; key: string; args: Record<string, string> } | { kind: "failed"; detail: string };

const FOCUSABLE = 'button:not([disabled]), textarea:not([disabled]), [href], input:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** The app's engine has failed (a panic): the report will take the latest stored save. */
function engineHasFailed(): boolean {
  try {
    return engine().failed;
  } catch {
    // No worker here (tests): nothing has failed.
    return false;
  }
}

export function ReportDialog() {
  const open = useReport((s) => s.open);
  return open ? <ReportDialogBody /> : null;
}

function ReportDialogBody() {
  const tr = useGame((s) => s.tr);
  const run = useGame((s) => s.run);
  const close = useReport((s) => s.closeReport);
  const [note, setNote] = useState("");
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const box = useRef<HTMLDivElement>(null);
  const field = useRef<HTMLTextAreaElement>(null);
  const id = useId();
  const engineFailed = useState(engineHasFailed)[0];

  // Focus moves into the dialog, and back to where it was when it closes.
  useEffect(() => {
    const before = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    field.current?.focus();
    return () => before?.focus();
  }, []);

  // Escape closes the dialog, wherever focus is, and nothing under it.
  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.stopPropagation();
      e.preventDefault();
      close();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [close]);

  const save = async () => {
    if (status.kind === "working") return;
    setStatus({ kind: "working" });
    try {
      const now = new Date();
      const parts = await gatherRun({ live: engine(), run, store: saveStore(), fresh: freshEngine, now });
      const report = assembleReport(parts, {
        description: note,
        errors: useReport.getState().errors,
        device: deviceInfo(),
        appVersion: __APP_VERSION__,
        buildId: buildId(__APP_VERSION__),
        now,
      });
      const file = encodeReport(report, now);
      const how = await deliverFile(file.name, file.text);
      const args = { file: file.name, size: formatSize(new Blob([file.text]).size) };
      setStatus(how === "cancelled" ? { kind: "idle" } : { kind: "done", key: how === "shared" ? "ui-report-shared" : "ui-report-done", args });
    } catch (e) {
      setStatus({ kind: "failed", detail: e instanceof Error ? e.message : String(e) });
    }
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== "Tab" || !box.current) return;
    // Keep the keyboard inside the dialog.
    const items = Array.from(box.current.querySelectorAll<HTMLElement>(FOCUSABLE));
    const first = items[0];
    const last = items[items.length - 1];
    if (!first || !last) return;
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };

  const working = status.kind === "working";
  return (
    <div className="modal-backdrop report-backdrop">
      <div ref={box} className="modal report-dialog" role="dialog" aria-modal="true" aria-labelledby={`${id}-title`} aria-describedby={`${id}-intro ${id}-privacy`} onKeyDown={onKeyDown} data-testid="report-dialog">
        <div className="sheet-head">
          <h2 id={`${id}-title`}>{tr.t("ui-report")}</h2>
          <button type="button" className="btn btn-ghost" onClick={close} aria-label={tr.t("ui-close")}>
            ✕
          </button>
        </div>
        <p id={`${id}-intro`}>{tr.t("ui-report-intro")}</p>
        {(engineFailed || !run) && <p className="hint">{tr.t(engineFailed ? "ui-report-engine-failed" : "ui-report-no-game")}</p>}
        <div>
          <p className="report-contains">{tr.t("ui-report-contains")}</p>
          <ul className="report-items" data-testid="report-items">
            <li>{tr.t("ui-report-item-run")}</li>
            <li>{tr.t("ui-report-item-save")}</li>
            <li>{tr.t("ui-report-item-errors")}</li>
            <li>{tr.t("ui-report-item-device")}</li>
            <li>{tr.t("ui-report-item-note")}</li>
          </ul>
          <p id={`${id}-privacy`} className="report-privacy" data-testid="report-privacy">
            {tr.t("ui-report-privacy")}
          </p>
        </div>
        <div className="field">
          <label htmlFor={`${id}-note`}>{tr.t("ui-report-describe")}</label>
          <textarea ref={field} id={`${id}-note`} rows={4} value={note} maxLength={4000} onChange={(e) => setNote(e.target.value)} aria-describedby={`${id}-note-hint`} data-testid="report-description" />
          <span id={`${id}-note-hint`} className="hint">
            {tr.t("ui-report-describe-hint")}
          </span>
        </div>
        <div aria-live="polite" data-testid="report-status">
          {status.kind === "working" && <p>{tr.t("ui-report-working")}</p>}
          {status.kind === "done" && <p>{tr.t(status.key, status.args)}</p>}
          {status.kind === "failed" && (
            <div role="alert">
              <p>{tr.t("ui-report-failed")}</p>
              <p className="hint">{status.detail}</p>
            </div>
          )}
        </div>
        <div className="report-actions">
          <button type="button" className="btn btn-primary" aria-disabled={working} onClick={() => void save()} data-testid="report-save">
            {tr.t("ui-report-save")}
          </button>
          <button type="button" className="btn btn-ghost" onClick={close}>
            {tr.t("ui-close")}
          </button>
        </div>
      </div>
    </div>
  );
}
