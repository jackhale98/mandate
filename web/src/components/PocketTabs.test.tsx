import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import type { PocketView } from "../engine/protocol/PocketView";
import { englishTranslator } from "../i18n/englishLocale";
import { POCKET_TABS, NOTEBOOK_TABS, placeTab, readNotebookTab, readPocketDocked, readPocketTab, writeNotebookTab, writePocketDocked, writePocketTab, type PocketSide } from "../state/devicePrefs";
import { useGame } from "../state/gameStore";
import { PocketTabs } from "./PocketTabs";

const pocket = (needs_count: number, notebook_new: number) => ({ needs_count, notebook_new }) as unknown as PocketView;

function Harness({ p = pocket(0, 0), wide = false }: { p?: PocketView; wide?: boolean }) {
  const [side, setSide] = useState<PocketSide>("today");
  const [book, setBook] = useState(false);
  return (
    <PocketTabs
      side={side}
      onChange={(t) => {
        setSide(t);
        if (t === "notebook") setBook(true);
      }}
      p={p}
      wide={wide}
      bookOpen={book}
    />
  );
}

describe("the pocket's switch (DEC-113)", () => {
  beforeEach(() => {
    useGame.setState({ tr: englishTranslator() });
  });

  it("is a tab list of two on a phone: one in the Tab order, arrows, Home and End move and select", () => {
    render(<Harness />);
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((t) => t.textContent)).toEqual(["Today", "Notebook"]);
    expect(tabs.map((t) => t.getAttribute("tabindex"))).toEqual(["0", "-1"]);
    expect(tabs[0]?.getAttribute("aria-controls")).toBe("pocket-panel-today");
    expect(tabs.map((t) => t.getAttribute("data-on"))).toEqual(["true", "false"]);
    fireEvent.keyDown(tabs[0] as HTMLElement, { key: "ArrowLeft" });
    expect(screen.getByRole("tab", { selected: true }).id).toBe("pocket-tab-notebook");
    expect(document.activeElement?.id).toBe("pocket-tab-notebook");
    fireEvent.keyDown(document.activeElement as HTMLElement, { key: "ArrowRight" });
    expect(screen.getByRole("tab", { selected: true }).id).toBe("pocket-tab-today");
    fireEvent.keyDown(document.activeElement as HTMLElement, { key: "End" });
    expect(screen.getByRole("tab", { selected: true }).id).toBe("pocket-tab-notebook");
    fireEvent.keyDown(document.activeElement as HTMLElement, { key: "Home" });
    expect(screen.getByRole("tab", { selected: true }).id).toBe("pocket-tab-today");
  });

  it("carries two counts, the engine's: what needs you and what is new in the notebook, said in words", () => {
    render(<Harness p={pocket(3, 0)} />);
    expect(screen.getByTestId("pocket-tab-badge-today").textContent).toBe("3");
    expect(screen.queryByTestId("pocket-tab-badge-notebook")).toBeNull();
    expect(screen.getByRole("tab", { name: /Today.*3 things need you/ })).toBeTruthy();
  });

  it("on a wide screen is two buttons, each showing its own panel (both can be open)", () => {
    render(<Harness wide p={pocket(0, 2)} />);
    expect(screen.queryAllByRole("tab")).toHaveLength(0);
    const book = screen.getByTestId("pocket-tab-notebook");
    expect(book.getAttribute("aria-expanded")).toBe("false");
    expect(screen.getByTestId("pocket-tab-today").getAttribute("data-on")).toBe("true");
    expect(screen.getByTestId("pocket-tab-badge-notebook").textContent).toBe("✎ 2");
    fireEvent.click(book);
    expect(screen.getByTestId("pocket-tab-notebook").getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByTestId("pocket-tab-today").getAttribute("data-on")).toBe("true");
  });
});

describe("pocket choices remembered on the device", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    localStorage.clear();
  });

  it("keeps the side, the Notebook's tab and the docking, and falls back when storage is blocked", () => {
    expect(readPocketTab()).toBe("today");
    expect(readNotebookTab()).toBe("people");
    writePocketTab("notebook");
    writeNotebookTab("policies");
    writePocketDocked(true);
    expect(readPocketTab()).toBe("notebook");
    expect(readNotebookTab()).toBe("policies");
    expect(readPocketDocked()).toBe(true);
    localStorage.setItem("mandate.pocketTab", "nonsense");
    expect(readPocketTab()).toBe("today");
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    expect(readPocketTab()).toBe("today");
    expect(readNotebookTab()).toBe("people");
    expect(readPocketDocked()).toBe(false);
    expect(() => writePocketTab("notebook")).not.toThrow();
  });

  it("opens the Notebook where an old Town, Places or People tab was remembered (they moved there)", () => {
    for (const [old, tab] of [
      ["town", "town"],
      ["places", "places"],
      ["people", "people"],
    ] as const) {
      localStorage.clear();
      localStorage.setItem("mandate.pocketTab", old);
      expect(readPocketTab()).toBe("notebook");
      expect(readNotebookTab()).toBe(tab);
    }
    expect(POCKET_TABS).toEqual(["today", "notebook"]);
    expect(NOTEBOOK_TABS).toEqual(["people", "places", "town", "policies"]);
    expect(placeTab("places")).toEqual({ side: "notebook", book: "places" });
    expect(placeTab("today")).toEqual({ side: "today", book: null });
  });
});
