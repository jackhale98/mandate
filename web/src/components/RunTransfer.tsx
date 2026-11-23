// Run export and import on the title screen (DEC-082, proposed). The file
// format and the import rules live in persistence/runFile; this file only
// wires them to the browser (download, share, file picker, drag and drop)
// and to the engine, which validates a save by loading it.
import { useEffect, useRef, useState } from "react";
import { freshEngine } from "../engine/client";
import { saveStore } from "../persistence/indexedDbStore";
import { MAX_RUN_FILE_BYTES, RUN_FILE_EXTENSION, RunImportError, exportRun, importRun, validateInProbe, type SaveValidator } from "../persistence/runFile";
import type { RunSummary } from "../persistence/saveStore";
import { useGame } from "../state/gameStore";

/** A message for the saved-runs card: a Fluent key, its args, and whether it is an error. */
export interface TransferNotice {
  key: string;
  args?: Record<string, string>;
  error?: boolean;
  /** Technical detail from the engine (shown small, not translated). */
  detail?: string;
}

/** Load the save under the current rules in a throwaway engine; nothing is stored here and the app's own engine is not touched. */
const validateWithEngine: SaveValidator = (bytes) => validateInProbe(freshEngine, bytes);

const errText = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Save a file the way phones and desktops both support: Blob + object URL + <a download>. */
function download(name: string, file: Blob) {
  const url = URL.createObjectURL(file);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.rel = "noopener";
  a.style.display = "none";
  document.body.appendChild(a);
  a.click();
  a.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
}

/** On touch devices that can share files, offer the share sheet (save to Files, send to another device). */
async function shareOrDownload(name: string, text: string): Promise<"shared" | "cancelled" | "downloaded"> {
  const file = new File([text], name, { type: "application/octet-stream" });
  const coarse = window.matchMedia?.("(pointer: coarse)").matches === true;
  if (coarse && typeof navigator.canShare === "function" && navigator.canShare({ files: [file] })) {
    try {
      await navigator.share({ files: [file], title: name });
      return "shared";
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") return "cancelled";
      // Not allowed (for example the gesture expired): fall back to a download.
    }
  }
  download(name, file);
  return "downloaded";
}

export function ExportRunButton({ run, onNotice }: { run: RunSummary; onNotice: (n: TransferNotice) => void }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const [working, setWorking] = useState(false);
  const label = tr.t("ui-export-label", { name: run.name ?? tr.t("ui-mode-label", { mode: run.mode }) });
  const onClick = async () => {
    setWorking(true);
    try {
      const { name, text } = await exportRun(saveStore(), run.runId, __APP_VERSION__);
      const how = await shareOrDownload(name, text);
      if (how !== "cancelled") onNotice({ key: how === "shared" ? "ui-export-shared" : "ui-export-done", args: { file: name } });
    } catch (e) {
      onNotice({ key: "ui-export-failed", error: true, detail: errText(e) });
    } finally {
      setWorking(false);
    }
  };
  return (
    <button type="button" className="btn btn-ghost" disabled={busy || working} aria-label={label} onClick={() => void onClick()}>
      {tr.t("ui-export")}
    </button>
  );
}

const hasFiles = (e: DragEvent) => Array.from(e.dataTransfer?.types ?? []).includes("Files");

/** "Import a run": a file picker, plus drag and drop anywhere on the title screen (desktop). */
export function ImportRun({ onNotice }: { onNotice: (n: TransferNotice) => void }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const refreshRuns = useGame((s) => s.refreshRuns);
  const input = useRef<HTMLInputElement>(null);
  const [working, setWorking] = useState(false);
  const [dragging, setDragging] = useState(false);

  const importFile = async (file: File) => {
    setWorking(true);
    onNotice({ key: "ui-import-working" });
    try {
      if (file.size > MAX_RUN_FILE_BYTES) throw new RunImportError("corrupted", "file too large");
      const outcome = await importRun(saveStore(), new Uint8Array(await file.arrayBuffer()), validateWithEngine);
      await refreshRuns();
      onNotice({ key: `ui-import-${outcome.kind}` });
    } catch (e) {
      if (e instanceof RunImportError) {
        onNotice({ key: `ui-import-error-${e.code}`, error: true, ...(e.code === "incompatible" && e.detail ? { detail: e.detail } : {}) });
      } else {
        onNotice({ key: "ui-import-error-failed", error: true, detail: errText(e) });
      }
    } finally {
      setWorking(false);
    }
  };
  // The latest handler, for the window listeners below.
  const importRef = useRef(importFile);
  useEffect(() => {
    importRef.current = importFile;
  });

  useEffect(() => {
    let depth = 0;
    const over = (e: DragEvent) => {
      if (!hasFiles(e)) return;
      e.preventDefault();
      if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
    };
    const enter = (e: DragEvent) => {
      if (!hasFiles(e)) return;
      depth++;
      setDragging(true);
    };
    const leave = (e: DragEvent) => {
      if (!hasFiles(e)) return;
      depth = Math.max(0, depth - 1);
      if (depth === 0) setDragging(false);
    };
    const drop = (e: DragEvent) => {
      if (!hasFiles(e)) return;
      // Never let the browser navigate to a dropped file.
      e.preventDefault();
      depth = 0;
      setDragging(false);
      const file = e.dataTransfer?.files[0];
      if (file) void importRef.current(file);
    };
    window.addEventListener("dragover", over);
    window.addEventListener("dragenter", enter);
    window.addEventListener("dragleave", leave);
    window.addEventListener("drop", drop);
    return () => {
      window.removeEventListener("dragover", over);
      window.removeEventListener("dragenter", enter);
      window.removeEventListener("dragleave", leave);
      window.removeEventListener("drop", drop);
    };
  }, []);

  return (
    <div className={dragging ? "import-run import-run-drag" : "import-run"} data-testid="import-run">
      <button type="button" className="btn btn-block" disabled={busy || working} onClick={() => input.current?.click()}>
        {tr.t("ui-import")}
      </button>
      <input
        ref={input}
        type="file"
        className="visually-hidden"
        tabIndex={-1}
        aria-hidden="true"
        // No `accept` filter: phones grey out unknown extensions such as .mandate.
        onChange={(e) => {
          const file = e.target.files?.[0];
          e.target.value = "";
          if (file) void importFile(file);
        }}
      />
      <p className="hint">{tr.t("ui-import-help", { ext: RUN_FILE_EXTENSION })}</p>
      <p className="hint import-drop-hint">{tr.t(dragging ? "ui-import-drop-active" : "ui-import-drop", { ext: RUN_FILE_EXTENSION })}</p>
    </div>
  );
}

/** The latest export or import message (a polite live region; errors are alerts). */
export function TransferStatus({ notice }: { notice: TransferNotice | null }) {
  const tr = useGame((s) => s.tr);
  return (
    <div className="transfer-status-region" aria-live="polite" data-testid="transfer-status">
      {notice && (
        <div className={notice.error ? "transfer-status transfer-error" : "transfer-status"} role={notice.error ? "alert" : undefined}>
          <p>{tr.t(notice.key, notice.args)}</p>
          {notice.detail && <p className="hint transfer-detail">{notice.detail}</p>}
        </div>
      )}
    </div>
  );
}
