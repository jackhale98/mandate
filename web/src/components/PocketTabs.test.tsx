import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { englishTranslator } from "../i18n/englishLocale";
import { POCKET_TABS, readPocketDocked, readPocketTab, writePocketDocked, writePocketTab, type PocketTab } from "../state/devicePrefs";
import { useGame } from "../state/gameStore";
import { PocketTabs } from "./PocketTabs";

function Harness({ badges = {} }: { badges?: Parameters<typeof PocketTabs>[0]["badges"] }) {
  const [tab, setTab] = useState<PocketTab>("today");
  return <PocketTabs tab={tab} onChange={setTab} badges={badges} />;
}

describe("the pocket's tabs", () => {
  beforeEach(() => {
    useGame.setState({ tr: englishTranslator() });
  });

  it("is a tab list: one tab in the Tab order, arrows, Home and End move and select", () => {
    render(<Harness />);
    const tabs = screen.getAllByRole("tab");
    // Four tabs: people are in the Notebook (its People section).
    expect(tabs.map((t) => t.textContent)).toEqual(["📅Today", "🏛️Town", "📍Places", "📓Notebook"]);
    expect(tabs.map((t) => t.getAttribute("tabindex"))).toEqual(["0", "-1", "-1", "-1"]);
    expect(tabs[0]?.getAttribute("aria-controls")).toBe("pocket-panel-today");
    fireEvent.keyDown(tabs[0] as HTMLElement, { key: "ArrowLeft" });
    expect(screen.getByRole("tab", { selected: true }).id).toBe("pocket-tab-notebook");
    expect(document.activeElement?.id).toBe("pocket-tab-notebook");
    fireEvent.keyDown(document.activeElement as HTMLElement, { key: "ArrowRight" });
    expect(screen.getByRole("tab", { selected: true }).id).toBe("pocket-tab-today");
    fireEvent.keyDown(document.activeElement as HTMLElement, { key: "End" });
    expect(screen.getByRole("tab", { selected: true }).id).toBe("pocket-tab-notebook");
    fireEvent.keyDown(document.activeElement as HTMLElement, { key: "Home" });
    expect(screen.getByRole("tab", { selected: true }).id).toBe("pocket-tab-today");
    fireEvent.click(screen.getByRole("tab", { name: /Places/ }));
    expect(screen.getByRole("tab", { selected: true }).id).toBe("pocket-tab-places");
  });

  it("shows a count only where something waits, and says what it counts", () => {
    render(<Harness badges={{ today: { n: 3, text: "2 decisions, 1 alert" }, notebook: { n: 0, text: "none" } }} />);
    expect(screen.getByTestId("pocket-tab-badge-today").textContent).toBe("3");
    expect(screen.queryByTestId("pocket-tab-badge-notebook")).toBeNull();
    expect(screen.getByRole("tab", { name: /Today.*2 decisions, 1 alert/ })).toBeTruthy();
  });
});

describe("pocket choices remembered on the device", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    localStorage.clear();
  });

  it("keeps the last tab and the docking, and falls back when storage is blocked", () => {
    expect(readPocketTab()).toBe("today");
    writePocketTab("places");
    writePocketDocked(true);
    expect(readPocketTab()).toBe("places");
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
    expect(readPocketDocked()).toBe(false);
    expect(() => writePocketTab("town")).not.toThrow();
  });

  it("opens the Notebook where a People tab was remembered (people moved there)", () => {
    localStorage.setItem("mandate.pocketTab", "people");
    expect(readPocketTab()).toBe("notebook");
    expect(POCKET_TABS).toEqual(["today", "town", "places", "notebook"]);
  });
});
