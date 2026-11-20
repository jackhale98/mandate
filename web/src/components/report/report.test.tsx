import { act, fireEvent, render, screen, within } from "@testing-library/react";
import type { EngineClient } from "../../engine/client";
import type { SaveRequestDto } from "../../engine/protocol/SaveRequestDto";
import { englishTranslator } from "../../i18n/englishLocale";
import { IndexedDbStore } from "../../persistence/indexedDbStore";
import { fromBase64 } from "../../persistence/runFile";
import { randomHex, type RunRecord } from "../../persistence/saveStore";
import { useGame } from "../../state/gameStore";
import { MAX_REPORT_ERRORS, noteCommand, recordError, useReport } from "../../state/reportStore";
import { assembleReport, buildId, encodeReport, gatherRun, reportFileName, type ReportCore } from "./buildReport";
import { ReportDialog } from "./ReportDialog";
import { ErrorBanner } from "../ErrorBanner";

const core = (hash: string): ReportCore => ({
  engine_version: "0.1.0",
  content_hash: "c0ffee",
  protocol_version: 1,
  save_format_version: 18,
  state_hash: hash,
  turn: 3,
  date: { day_number: 42, year: 2020, month: 2, day: 12 },
  log: {
    version: 1,
    new_game: { seed: "18446744073709551615", size: "Medium", scenario: null, storyteller: null, local_knowledge: false },
    engine_version: "0.1.0",
    content_hash: "c0ffee",
    commands: [{ index: 0, command: { type: "end_turn" } }],
    checkpoints: [{ commands: 0, turn: 0, hash: "h0" }],
    note: null,
  },
});

/** A fake engine: `exportSave` returns the given bytes, `reportCore` the given core. */
function fakeEngine(o: { bytes?: Uint8Array; core?: ReportCore; failed?: boolean; importFails?: boolean }) {
  const calls = { exported: [] as SaveRequestDto[], imported: [] as Uint8Array[], terminated: 0 };
  const e = {
    ready: Promise.resolve(),
    failed: o.failed ?? false,
    exportSave: async (r: SaveRequestDto) => {
      calls.exported.push(r);
      return o.bytes ?? new Uint8Array([1, 2, 3]);
    },
    importSave: async (b: Uint8Array) => {
      calls.imported.push(b);
      if (o.importFails) throw new Error("save format v99 is newer");
      return {};
    },
    reportCore: async () => JSON.stringify(o.core ?? core("live")),
    terminate: () => {
      calls.terminated++;
    },
  };
  return { engine: e as unknown as EngineClient, calls };
}

const record = (runId: string, revision: number, bytes: Uint8Array, updatedAt: string): RunRecord => ({
  runId,
  mode: "story",
  status: "active",
  revision,
  bytes,
  updatedAt,
  meta: {
    format_version: 18, run_id: runId, revision, parent_revision: null, device_id: "d", saved_at: updatedAt, engine_version: "0.1.0",
    content_hash: "c0ffee", protocol_version: 1, run_mode: "story", seed: "7", date: { day_number: 0, year: 2020, month: 1, day: 1 }, turn: 0,
  },
});

const device = { viewport: { width: 390, height: 844 }, dpr: 3, user_agent: "test-agent" };

describe("bug report file", () => {
  it("is named after the seed and the local date", () => {
    expect(reportFileName("4242", new Date(2026, 10, 19, 23, 50))).toBe("mandate-report-4242-2026-11-19.json");
    expect(reportFileName(null, new Date(2026, 0, 2))).toBe("mandate-report-unknown-2026-01-02.json");
    expect(reportFileName("../x", new Date(2026, 0, 2))).toBe("mandate-report-unknown-2026-01-02.json");
  });

  it("names the build by its version and hashed bundle", () => {
    expect(buildId("0.1.0", "https://x.test/mandate/play/assets/index-Ab12Cd.js?v=1")).toBe("0.1.0+index-Ab12Cd.js");
  });

  it("holds the engine's part as it is, the save in base64, the errors, the device and the note; nothing else", () => {
    const bytes = new Uint8Array([77, 78, 68, 84, 0, 255]);
    const errors = [{ at: "2026-11-19T10:00:00.000Z", message: "boom", command: { type: "end_turn" as const } }];
    const r = assembleReport(
      { core: core("abc"), save: { source: "live", revision: 5, bytes }, mode: "ironman", notes: [] },
      { description: "  it froze \n", errors, device, appVersion: "0.1.0", buildId: "0.1.0+index.js", now: new Date("2026-11-19T10:01:00Z") },
    );
    expect(Object.keys(r).sort()).toEqual(["app", "created_at", "description", "device", "engine", "errors", "format", "format_version", "log", "notes", "run", "save", "state_hash"]);
    expect(r.format).toBe("mandate-report");
    expect(r.description).toBe("it froze");
    expect(r.state_hash).toBe("abc");
    expect(r.run).toMatchObject({ seed: "18446744073709551615", mode: "ironman", turn: 3 });
    expect(r.engine).toEqual({ engine_version: "0.1.0", content_hash: "c0ffee", protocol_version: 1, save_format_version: 18 });
    expect(r.save?.length).toBe(6);
    expect(Array.from(fromBase64(r.save?.base64 ?? ""))).toEqual(Array.from(bytes));
    expect(r.errors).toEqual(errors);
    expect(r.device).toEqual(device);
    // The seed stays a string: exact beyond 2^53.
    const { name, text } = encodeReport(r, new Date(2026, 10, 19));
    expect(name).toBe("mandate-report-18446744073709551615-2026-11-19.json");
    expect(JSON.parse(text).log.new_game.seed).toBe("18446744073709551615");
  });
});

describe("gathering the run for a report", () => {
  const now = new Date("2026-11-19T12:00:00Z");

  it("takes the running game from the app's engine", async () => {
    const store = new IndexedDbStore(`t-${randomHex(4)}`);
    const live = fakeEngine({ bytes: new Uint8Array([9]), core: core("live") });
    const fresh = fakeEngine({});
    const parts = await gatherRun({ live: live.engine, run: { runId: "r1", mode: "ironman", revision: 4 }, store, fresh: () => fresh.engine, now });
    expect(parts.core?.state_hash).toBe("live");
    expect(parts.save).toMatchObject({ source: "live", revision: 4 });
    expect(live.calls.exported[0]).toMatchObject({ run_id: "r1", revision: 4, run_mode: "ironman" });
    expect(fresh.calls.imported).toHaveLength(0);
  });

  it("after the engine failed, reads the latest stored save with a fresh engine", async () => {
    const store = new IndexedDbStore(`t-${randomHex(4)}`);
    await store.putRun(record("old", 2, new Uint8Array([1]), "2026-11-18T00:00:00Z"));
    await store.putRun(record("new", 7, new Uint8Array([7, 7]), "2026-11-19T00:00:00Z"));
    const live = fakeEngine({ failed: true });
    const fresh = fakeEngine({ core: core("stored") });
    const parts = await gatherRun({ live: live.engine, run: null, store, fresh: () => fresh.engine, now });
    expect(live.calls.exported).toHaveLength(0);
    expect(Array.from(fresh.calls.imported[0] ?? [])).toEqual([7, 7]);
    expect(fresh.calls.terminated).toBe(1);
    expect(parts.core?.state_hash).toBe("stored");
    expect(parts.save).toMatchObject({ source: "autosave", revision: 7 });
  });

  it("keeps the save and notes why when it cannot be read; with no save at all, says so", async () => {
    const store = new IndexedDbStore(`t-${randomHex(4)}`);
    const none = await gatherRun({ live: null, run: null, store, fresh: () => fakeEngine({}).engine, now });
    expect(none).toMatchObject({ core: null, save: null, notes: ["no saved game"] });
    await store.putRun(record("r", 3, new Uint8Array([3]), "2026-11-19T00:00:00Z"));
    const fresh = fakeEngine({ importFails: true });
    const parts = await gatherRun({ live: null, run: null, store, fresh: () => fresh.engine, now });
    expect(parts.core).toBeNull();
    expect(parts.save?.revision).toBe(3);
    expect(parts.notes[0]).toContain("newer");
    expect(fresh.calls.terminated).toBe(1);
  });
});

describe("caught errors", () => {
  it("keeps the last few, each with the command that was being run", () => {
    useReport.setState({ errors: [] });
    noteCommand({ type: "end_turn" });
    for (let i = 0; i < MAX_REPORT_ERRORS + 3; i++) recordError(`e${i}`);
    const errors = useReport.getState().errors;
    expect(errors).toHaveLength(MAX_REPORT_ERRORS);
    expect(errors[0]?.message).toBe("e3");
    expect(errors.at(-1)).toMatchObject({ message: `e${MAX_REPORT_ERRORS + 2}`, command: { type: "end_turn" } });
  });

  it("records the errors the app shows", () => {
    useReport.setState({ errors: [] });
    act(() => useGame.setState({ error: "engine exploded" }));
    act(() => useGame.setState({ error: null }));
    expect(useReport.getState().errors.map((e) => e.message)).toEqual(["engine exploded"]);
  });
});

describe("the Report a problem dialog", () => {
  beforeEach(() => {
    useGame.setState({ tr: englishTranslator(), run: null });
    useReport.setState({ open: false });
  });

  it("is a labelled modal dialog that says what the file holds and that nothing is sent; focus goes in and back; Escape closes", () => {
    const opener = document.createElement("button");
    document.body.appendChild(opener);
    opener.focus();
    render(<ReportDialog />);
    expect(screen.queryByRole("dialog")).toBeNull();
    act(() => useReport.getState().openReport("menu"));
    const dialog = screen.getByRole("dialog", { name: "Report a problem" });
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-describedby")).toBeTruthy();
    expect(screen.getByTestId("report-privacy").textContent).toContain("Nothing is sent anywhere");
    expect(screen.getByTestId("report-items").querySelectorAll("li")).toHaveLength(5);
    // No game open: it says the report takes the latest save.
    expect(dialog.textContent).toContain("No game is open");
    const note = screen.getByRole("textbox", { name: "What happened? (optional)" });
    expect(document.activeElement).toBe(note);
    // Tab from the last control comes back to the first.
    const buttons = within(dialog).getAllByRole("button");
    buttons.at(-1)?.focus();
    fireEvent.keyDown(dialog, { key: "Tab" });
    expect(document.activeElement).toBe(buttons[0]);
    fireEvent.keyDown(dialog, { key: "Escape" });
    expect(useReport.getState().open).toBe(false);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(opener);
    opener.remove();
  });
});

describe("the error banner", () => {
  it("offers Report a problem, which opens the dialog", () => {
    useGame.setState({ tr: englishTranslator(), error: "invalid command: boom" });
    useReport.setState({ open: false });
    render(<ErrorBanner />);
    fireEvent.click(screen.getByRole("button", { name: "Report a problem" }));
    expect(useReport.getState()).toMatchObject({ open: true, origin: "error" });
    act(() => useGame.setState({ error: null }));
  });
});
