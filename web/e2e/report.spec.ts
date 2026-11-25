import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test, type Page } from "@playwright/test";
import { officeFigures, openPeople, pocketMenu, showPlaces } from "./pocket";

// "Report a problem" (bug bundles for the M6 testers): play a few turns
// (canvass, talk, end turn), save a report from the pocket, read the file,
// and replay it through the engine (the node WASM build, as
// `mandate-cli replay --wasm` does): every checkpoint and the end match.
// A tampered command is found at its turn. With MANDATE_CLI=<path to
// mandate-cli>, the native replay checks the same file too.

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const WASM_NODE = process.env.MANDATE_WASM_NODE ? path.resolve(ROOT, process.env.MANDATE_WASM_NODE) : path.join(ROOT, "target/wasm-node");
const CLI = process.env.MANDATE_CLI ?? "";

// Always take the download path: some Chromium builds expose the Web Share API.
const noShare = () => {
  Object.defineProperty(Navigator.prototype, "canShare", { value: undefined, configurable: true });
};

interface Checkpoint {
  commands: number;
  turn: number;
  hash: string;
}
interface Outcome {
  commands_applied: number;
  commands_total: number;
  state_hash: string;
  checkpoints: { commands: number; turn: number; expected: string; actual: string }[];
  divergence: { at: { turn: number }; last_good_turn: number | null; commands_from: number; commands_to: number } | null;
}

/** Replay a report through the WASM build (scripts/wasm-replay.mjs). */
const replayWasm = (file: string): Outcome => JSON.parse(execFileSync("node", [path.join(ROOT, "scripts/wasm-replay.mjs"), file], { cwd: ROOT, env: { ...process.env, MANDATE_WASM_NODE: WASM_NODE }, encoding: "utf8" })) as Outcome;

/** `mandate-cli replay` (exit 0 when everything matches). */
const replayNative = (file: string): { ok: boolean; out: Record<string, unknown> } => {
  try {
    return { ok: true, out: JSON.parse(execFileSync(CLI, ["replay", file], { cwd: ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] })) as Record<string, unknown> };
  } catch (e) {
    const stdout = (e as { stdout?: string }).stdout ?? "{}";
    return { ok: false, out: JSON.parse(stdout) as Record<string, unknown> };
  }
};

const endTurn = async (page: Page, turn: number) => {
  await page.getByRole("button", { name: "End turn" }).click();
  await expect(page.getByText(`Turn ${turn}`)).toBeVisible();
};

test("report a problem: the file holds the run, and it replays to the same hashes", async ({ page }, testInfo) => {
  test.setTimeout(180_000);
  await page.addInitScript(noShare);
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("7311");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();

  // Canvass the first neighbourhood's streets.
  const places = await showPlaces(page);
  await places.getByTestId("spot-streets").first().click();
  const panel = page.getByTestId("place-panel");
  await expect(panel.getByTestId("you-are-here-panel")).toBeVisible();
  const form = panel.getByTestId("form-canvass");
  const worry = form.getByTestId("canvass-worry");
  await worry.selectOption((await worry.locator("option").nth(1).getAttribute("value")) ?? "");
  await form.getByRole("button", { name: /^(Do it|Go)$/ }).click();
  await expect(form.getByTestId("canvass-landed")).toBeVisible();
  await endTurn(page, 1);

  // Talk to a council member (walked to first).
  const people = await openPeople(page);
  await officeFigures(people).getByTestId("talk-button").first().click();
  // The dialogue box (dialogue mode, DEC-093): walk there if they are elsewhere, ask, and wait for the reply's bubble.
  const talk = page.getByTestId("talk-panel");
  await expect(talk).toBeVisible();
  const go = talk.getByTestId("talk-go").locator("button.dlg-choice");
  if ((await go.count()) > 0) await go.click();
  await talk.getByTestId("talk-topic-topic.who_runs").locator("button.dlg-choice").click();
  await expect(page.getByTestId("talk-bubble").locator("p").first()).not.toBeEmpty({ timeout: 15_000 });
  await expect(page.getByTestId("talk-reply-live")).not.toBeEmpty();
  // Leave: Escape steps back out of the conversation (more than once if a reply page or step is open).
  for (let i = 0; i < 4 && (await talk.count()) > 0; i++) await page.keyboard.press("Escape");
  await expect(talk).toHaveCount(0);
  await endTurn(page, 2);
  await endTurn(page, 3);

  // The pocket's "Report a problem": what the file holds, and that nothing is sent.
  await (await pocketMenu(page)).getByTestId("report-problem").click();
  const dialog = page.getByRole("dialog", { name: "Report a problem" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByTestId("report-privacy")).toContainText("Nothing is sent anywhere");
  const note = dialog.getByRole("textbox", { name: "What happened? (optional)" });
  await expect(note).toBeFocused();
  await note.fill("The canvass result looked odd.");
  const download = page.waitForEvent("download");
  await dialog.getByTestId("report-save").click();
  const file = await download;
  expect(file.suggestedFilename()).toMatch(/^mandate-report-7311-\d{4}-\d{2}-\d{2}\.json$/);
  const saved = testInfo.outputPath(file.suggestedFilename());
  await file.saveAs(saved);
  await expect(dialog.getByTestId("report-status")).toContainText(`Saved ${file.suggestedFilename()}`);
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);

  // The file: the run from its start, the save, the device; nothing else.
  const report = JSON.parse(await readFile(saved, "utf8")) as {
    format: string;
    description: string;
    state_hash: string;
    run: { seed: string; turn: number };
    log: { new_game: { seed: string }; commands: { index: number; command: { type: string; player?: { type: string } } }[]; checkpoints: Checkpoint[] };
    save: { source: string; base64: string; length: number };
    errors: unknown[];
    device: { viewport: { width: number; height: number }; dpr: number; user_agent: string };
    app: { version: string; build_id: string };
    engine: { content_hash: string };
  };
  expect(Object.keys(report).sort()).toEqual(["app", "created_at", "description", "device", "engine", "errors", "format", "format_version", "log", "notes", "run", "save", "state_hash"]);
  expect(report.format).toBe("mandate-report");
  expect(report.description).toBe("The canvass result looked odd.");
  expect(report.run).toMatchObject({ seed: "7311", turn: 3 });
  expect(report.log.new_game.seed).toBe("7311");
  const types = report.log.commands.map((c) => (c.command.type === "player" ? c.command.player?.type : c.command.type));
  expect(types).toContain("act");
  expect(types).toContain("talk");
  expect(types.filter((t) => t === "end_turn")).toHaveLength(3);
  expect(report.log.checkpoints.map((c) => c.turn)).toEqual([0, 1, 2, 3]);
  expect(report.log.checkpoints.at(-1)?.hash).toBe(report.state_hash);
  expect(report.save.source).toBe("live");
  expect(report.save.length).toBeGreaterThan(1000);
  expect(report.device.user_agent.length).toBeGreaterThan(0);
  expect(report.device.dpr).toBeGreaterThan(0);
  expect(report.errors).toEqual([]);
  expect(report.engine.content_hash).toMatch(/^[0-9a-f]{64}$/);

  // Replayed through the engine: every checkpoint and the end match.
  if (!existsSync(path.join(WASM_NODE, "mandate_wasm.js"))) {
    test.skip(!process.env.CI, `no node WASM build at ${WASM_NODE} (scripts/build-wasm.sh)`);
    throw new Error(`no node WASM build at ${WASM_NODE}`);
  }
  const ok = replayWasm(saved);
  expect(ok.divergence).toBeNull();
  expect(ok.commands_applied).toBe(report.log.commands.length);
  expect(ok.checkpoints).toHaveLength(4);
  expect(ok.checkpoints.every((c) => c.expected === c.actual)).toBe(true);
  expect(ok.state_hash).toBe(report.state_hash);

  // A tampered command (the talk, in turn 2) is found at that turn.
  const at = report.log.commands.findIndex((c) => c.command.player?.type === "talk");
  const tampered = structuredClone(report);
  const entry = tampered.log.commands[at];
  if (entry) entry.command = { type: "advance_days", days: 2 } as never;
  const bad = testInfo.outputPath("tampered.json");
  await writeFile(bad, JSON.stringify(tampered));
  const out = replayWasm(bad);
  expect(out.divergence).not.toBeNull();
  const d = out.divergence;
  const turnOf = report.log.checkpoints.findIndex((c) => c.commands > at);
  expect(d?.at.turn).toBe(report.log.checkpoints[turnOf]?.turn);
  expect(d?.commands_from).toBeLessThanOrEqual(at);
  expect(d?.commands_to).toBeGreaterThan(at);

  // Natively too, when the CLI is at hand: the same file matches; the tampered one names the turn.
  if (CLI) {
    const n = replayNative(saved);
    expect(n.ok, JSON.stringify(n.out)).toBe(true);
    expect((n.out.save as { matches_report: boolean }).matches_report).toBe(true);
    const t = replayNative(bad);
    expect(t.ok).toBe(false);
    expect((t.out.native as { first_divergence: { turn: number } }).first_divergence.turn).toBe(d?.at.turn);
  }
});

test("report a problem from the title screen (after a crash and a reload)", async ({ page }) => {
  await page.goto("./");
  await expect(page.getByTestId("report-problem-title")).toBeVisible();
  await page.getByTestId("report-problem-title").click();
  const dialog = page.getByRole("dialog", { name: "Report a problem" });
  await expect(dialog).toContainText("No game is open");
  await dialog.getByRole("button", { name: "Close" }).first().click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByTestId("report-problem-title")).toBeFocused();
});
