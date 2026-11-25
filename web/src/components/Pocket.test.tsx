/// <reference types="node" />
// Today (DEC-113) against the engine's pocket view (fixtures/pocket/pocket.json):
// "Needs you" in the engine's order, a decision straight to its sheet, an
// offer opened in place, an arc's row to its sheet, the bar's two counts, and
// Save and the rest behind the ⋯ menu.
import { fireEvent, render, screen, within } from "@testing-library/react";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { PocketView } from "../engine/protocol/PocketView";
import { englishTranslator } from "../i18n/englishLocale";
import { useGame } from "../state/gameStore";
import { Pocket } from "./Pocket";

const pocket = JSON.parse(readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), "fixtures/pocket/pocket.json"), "utf8")) as PocketView;

describe("Today", () => {
  it("lists what needs you in the engine's order and opens each the shortest way", () => {
    const openDecision = vi.fn(() => Promise.resolve());
    const openArc = vi.fn(() => Promise.resolve());
    const saveAndQuit = vi.fn(() => Promise.resolve());
    useGame.setState({ tr: englishTranslator(), busy: false, pocket, pocketOpen: true, pocketTab: "today", folds: {}, office: null, overview: null, openDecision, openArc, saveAndQuit });
    render(<Pocket />);
    const needs = within(screen.getByTestId("pocket-needs-list"));
    const rows = needs.getAllByRole("listitem");
    expect(rows.map((r) => r.getAttribute("data-need"))).toEqual(pocket.needs_you.map((n) => n.kind));
    // A decision: Choose opens its sheet (no row to open first).
    fireEvent.click(needs.getByTestId("open-decision-0"));
    expect(openDecision).toHaveBeenCalledWith(0);
    // An offer opens in place, its price read before it is answered.
    const offer = needs.getByTestId("ask-2");
    expect(within(offer).queryByTestId("ask-accept-2")).toBeNull();
    fireEvent.click(within(offer).getAllByRole("button")[0] as HTMLElement);
    expect(within(screen.getByTestId("ask-2")).getByTestId("ask-accept-2")).toBeTruthy();
    // An arc under way opens its sheet.
    fireEvent.click(screen.getByTestId("open-arc-0"));
    expect(openArc).toHaveBeenCalledWith(0);
    // The switch carries the engine's count; Save is behind the ⋯ menu.
    expect(screen.getByTestId("pocket-tab-badge-today").textContent).toBe(String(pocket.needs_count));
    expect(screen.queryByRole("button", { name: "Save & quit" })).toBeNull();
    fireEvent.click(screen.getByTestId("pocket-more"));
    fireEvent.click(within(screen.getByTestId("pocket-menu")).getByRole("button", { name: "Save & quit" }));
    expect(saveAndQuit).toHaveBeenCalled();
    const text = screen.getByTestId("pocket").textContent ?? "";
    expect(text).not.toContain("⟦");
    expect(text).not.toMatch(/[●○]/);
  });
});
