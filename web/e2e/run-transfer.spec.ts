import { readFile } from "node:fs/promises";
import { expect, test } from "./csp";
import type { Page } from "@playwright/test";
import { pocketMenu } from "./pocket";

// DEC-082 (proposed): export a run to a .mandate file and import it on another
// device (a fresh browser context: empty storage), with the Ironman rule that
// an older revision of a run that exists here is refused.

const stateHash = async (page: Page) => {
  await (await pocketMenu(page)).getByRole("button", { name: "Overview", exact: true }).click();
  return page.getByText(/state [0-9a-f]{16}/).textContent();
};

const saveAndQuit = async (page: Page) => {
  await (await pocketMenu(page)).getByRole("button", { name: "Save & quit" }).click();
  await expect(page.getByRole("button", { name: "Continue" }).first()).toBeVisible();
};

const endTurn = async (page: Page, turn: number) => {
  await page.getByRole("button", { name: "End turn" }).click();
  await expect(page.getByText(`Turn ${turn}`)).toBeVisible();
};

/** Import through the visible button and the system file picker. */
const importFile = async (page: Page, path: string) => {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Import a run" }).click();
  await (await chooser).setFiles(path);
};

// Always take the download path: some Chromium builds expose the Web Share API.
const noShare = () => {
  Object.defineProperty(Navigator.prototype, "canShare", { value: undefined, configurable: true });
};

test("export, import on a cleared browser, continue with the identical state", async ({
  page,
  browser,
  baseURL,
  viewport,
  isMobile,
  hasTouch,
  userAgent,
  deviceScaleFactor,
}, testInfo) => {
  test.setTimeout(120_000);
  await page.addInitScript(noShare);
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("5151");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await endTurn(page, 1);
  const hash = await stateHash(page);
  expect(hash).toMatch(/state [0-9a-f]{16}/);
  await saveAndQuit(page);

  // Export: one file named after the town and today's date.
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: /^Export / }).click();
  const file = await download;
  expect(file.suggestedFilename()).toMatch(/^mandate-[a-z0-9-]+-\d{4}-\d{2}-\d{2}\.mandate$/);
  expect(file.suggestedFilename()).not.toMatch(/^mandate-run-/);
  const path = testInfo.outputPath(file.suggestedFilename());
  await file.saveAs(path);
  expect((await readFile(path, "utf8")).split("\n")[0]).toBe("MANDATE RUN EXPORT");
  await expect(page.getByTestId("transfer-status")).toContainText("Saved mandate-");

  // Another device (or a cleared browser): empty storage.
  const other = await browser.newContext({
    ...(baseURL ? { baseURL } : {}),
    viewport,
    isMobile,
    hasTouch,
    ...(userAgent ? { userAgent } : {}),
    deviceScaleFactor,
  });
  try {
    const fresh = await other.newPage();
    await fresh.goto("./");
    await expect(fresh.getByText("No runs yet.")).toBeVisible();
    await importFile(fresh, path);
    await expect(fresh.getByTestId("transfer-status")).toContainText("Run imported");
    await fresh.getByRole("button", { name: "Continue" }).first().click();
    await expect(fresh.getByText("Turn 1")).toBeVisible();
    expect(await stateHash(fresh)).toBe(hash);
  } finally {
    await other.close();
  }

  // Back on the first device the run moved on: the older file is refused.
  await page.getByRole("button", { name: "Continue" }).first().click();
  await endTurn(page, 2);
  const later = await stateHash(page);
  await saveAndQuit(page);
  await importFile(page, path);
  const status = page.getByTestId("transfer-status");
  await expect(status.getByRole("alert")).toContainText("Ironman runs can't go back in time");
  await expect(page.locator(".run")).toHaveCount(1);
  await page.getByRole("button", { name: "Continue" }).first().click();
  await expect(page.getByText("Turn 2")).toBeVisible();
  expect(await stateHash(page)).toBe(later);
});

test("drag and drop a file onto the title screen (desktop)", async ({ page, isMobile }) => {
  test.skip(isMobile, "drag and drop is a desktop path");
  await page.goto("./");
  await expect(page.getByRole("button", { name: "Import a run" })).toBeVisible();
  await expect(page.getByText(/Or drop a \.mandate file/)).toBeVisible();
  const drop = async (text: string, name: string) => {
    const dt = await page.evaluateHandle(
      ({ text, name }) => {
        const d = new DataTransfer();
        d.items.add(new File([text], name));
        return d;
      },
      { text, name },
    );
    await page.dispatchEvent("main.title", "dragenter", { dataTransfer: dt });
    await expect(page.getByText("Drop the file to import it.")).toBeVisible();
    await page.dispatchEvent("main.title", "drop", { dataTransfer: dt });
  };
  await drop("just some notes", "notes.mandate");
  await expect(page.getByTestId("transfer-status").getByRole("alert")).toHaveText("This is not a Mandate run file.");
  await drop("MANDATE RUN EXPORT\n{\"format\":1,\"broken", "damaged.mandate");
  await expect(page.getByTestId("transfer-status").getByRole("alert")).toContainText("damaged or incomplete");
  await expect(page.getByText("No runs yet.")).toBeVisible();
});
