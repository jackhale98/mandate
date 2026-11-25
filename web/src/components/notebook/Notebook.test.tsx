/// <reference types="node" />
// The Notebook (DEC-113) against the engine's views for the pocket
// (../fixtures/pocket/*.json, written by crates/mandate-wasm/tests/pocket_views.rs):
// four tabs and a search; People from the one People query, in the engine's
// sections and order; Places by the engine's rank with its reasons; Town with
// its loose ends; and nothing that counts knowledge (DEC-091).
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { NotebookView } from "../../engine/protocol/NotebookView";
import type { PeopleView } from "../../engine/protocol/PeopleView";
import type { PocketView } from "../../engine/protocol/PocketView";
import type { TownPlacesView } from "../../engine/protocol/TownPlacesView";
import { englishTranslator } from "../../i18n/englishLocale";
import { useGame } from "../../state/gameStore";
import { Notebook, NotebookTabs } from "./Notebook";

const dir = path.join(path.dirname(fileURLToPath(import.meta.url)), "../fixtures/pocket");
const load = <T,>(name: string) => JSON.parse(readFileSync(path.join(dir, `${name}.json`), "utf8")) as T;

function setup(over: { notebook?: NotebookView; notebook_new?: number } = {}) {
  const seeNotebook = vi.fn(() => Promise.resolve());
  const loadNotebook = vi.fn(() => Promise.resolve());
  const leaveNotebook = vi.fn();
  const visit = vi.fn(() => Promise.resolve());
  const openTalk = vi.fn(() => Promise.resolve());
  const openCharacter = vi.fn(() => Promise.resolve());
  const openDecision = vi.fn(() => Promise.resolve());
  const pocket = { ...load<PocketView>("pocket"), notebook_new: over.notebook_new ?? 0 };
  useGame.setState({
    tr: englishTranslator(),
    busy: false,
    notebook: over.notebook ?? load<NotebookView>("notebook"),
    notebookFresh: [],
    people: load<PeopleView>("people"),
    pocket,
    town: load<TownPlacesView>("town"),
    notebookTab: "people",
    notebookQuery: null,
    folds: {},
    seeNotebook,
    loadNotebook,
    leaveNotebook,
    visit,
    openTalk,
    openCharacter,
    openDecision,
  });
  render(
    <>
      <NotebookTabs />
      <Notebook />
    </>,
  );
  return { seeNotebook, loadNotebook, leaveNotebook, visit, openTalk, openCharacter, openDecision };
}

/** Nothing counts knowledge (L8): no meter, no "Not known yet", no "N of M known", no per cent beside a knowledge word. */
function expectNoKnowledgeCount(el: HTMLElement) {
  const text = el.textContent ?? "";
  expect(text).not.toMatch(/[●○]/);
  expect(text).not.toContain("Not known yet");
  expect(text).not.toMatch(/\d+ of \d+ known|known \d+%|\d+% known/i);
  expect(text).not.toContain("⟦");
  expect(text).not.toMatch(/\{\s*\$/);
}

describe("the Notebook", () => {
  it("has one row of four tabs and a search at its end; arrows move along it", () => {
    setup();
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((t) => t.id)).toEqual(["notebook-tab-people", "notebook-tab-places", "notebook-tab-town", "notebook-tab-policies"]);
    expect(tabs.map((t) => t.getAttribute("tabindex"))).toEqual(["0", "-1", "-1", "-1"]);
    fireEvent.keyDown(tabs[0] as HTMLElement, { key: "ArrowRight" });
    expect(useGame.getState().notebookTab).toBe("places");
    expect(screen.getByRole("tabpanel").getAttribute("aria-labelledby")).toBe("notebook-tab-places");
    expect(screen.getByTestId("notebook-search-open").getAttribute("aria-label")).toBe("Search the notebook");
  });

  it("People: the engine's sections and order; who matters now open with why, the rest folded with what they hold counted", () => {
    const { visit, openTalk, openCharacter } = setup();
    const people = load<PeopleView>("people");
    const now = within(screen.getByTestId("notebook-people-now")).getAllByTestId("notebook-person");
    expect(now.map((r) => Number(r.getAttribute("data-person")))).toEqual(people.rows.filter((r) => r.group === "now").map((r) => r.id));
    // Three lines: the name, the stance in a word with its shape and the role, the reason (34 characters at most).
    const first = now[0] as HTMLElement;
    expect(first.textContent).toContain("Bettina Zeller");
    expect(first.textContent).toContain("◐ Neutral");
    expect(first.textContent).toContain("An ask needs a word with them");
    // Folded sections: a count, never a share.
    for (const [id, n] of [
      ["notebook-key-figures", 5],
      ["notebook-people-rest", 23],
      ["notebook-people-gone", 10],
    ] as const) {
      const fold = screen.getByTestId(id);
      const head = within(fold).getByRole("button", { expanded: false });
      expect(head.textContent).toMatch(new RegExp(`${n}, ${n} people`));
      expect(within(fold).queryAllByTestId("notebook-person")).toHaveLength(0);
    }
    // Opened: why now, how they stand, Talk and Go to them, and everything about them.
    fireEvent.click(within(first).getAllByRole("button", { expanded: false })[0] as HTMLElement);
    const opened = screen.getAllByTestId("notebook-person")[0] as HTMLElement;
    expect(within(opened).getAllByTestId("person-why").length).toBe(2);
    expect(within(opened).getByTestId("notebook-standing").textContent).toBe("Neutral about you · Trusts you a little");
    fireEvent.click(within(opened).getByRole("button", { name: "Go to Bettina Zeller" }));
    expect(visit).toHaveBeenCalledWith({ type: "place", place: 132 });
    fireEvent.click(within(opened).getByRole("button", { name: "Talk to Bettina Zeller" }));
    expect(openTalk).toHaveBeenCalledWith({ type: "person", person: 1027 });
    fireEvent.click(within(opened).getByTestId("notebook-person-name"));
    expect(openCharacter).toHaveBeenCalledWith(1027);
    // A person only heard of: the engine's one sentence, no placeholder.
    const sana = screen.getAllByTestId("notebook-person")[1] as HTMLElement;
    fireEvent.click(within(sana).getAllByRole("button")[0] as HTMLElement);
    expect(within(screen.getAllByTestId("notebook-person")[1] as HTMLElement).getByTestId("person-note").textContent).toBe("You only know the name.");
    // Someone here: Talk is the row's action.
    const here = screen.getAllByTestId("notebook-person")[2] as HTMLElement;
    expect(within(here).getByTestId("talk-button")).toBeTruthy();
    // A fold opens on a tap and shows the engine's first twelve, then the rest on asking.
    fireEvent.click(within(screen.getByTestId("notebook-people-rest")).getByRole("button", { expanded: false }));
    const rest = screen.getByTestId("notebook-people-rest");
    expect(within(rest).getAllByTestId("notebook-person")).toHaveLength(12);
    fireEvent.click(within(rest).getByTestId("show-more"));
    expect(within(rest).getAllByTestId("notebook-person")).toHaveLength(23);
    expectNoKnowledgeCount(document.body);
  });

  it("Places: what matters now first in the engine's order with its reasons; each neighbourhood a row with how many places, the one you are in open", () => {
    const { visit } = setup();
    fireEvent.click(screen.getByRole("tab", { name: /Places/ }));
    const now = screen.getByTestId("places-now");
    expect(within(now).getAllByRole("button").map((b) => b.getAttribute("data-testid"))).toEqual(["spot-streets", "spot-community_centre"]);
    expect(now.textContent).toContain("There is something to do here");
    expect(within(now).getByTestId("you-are-here")).toBeTruthy();
    const hoods = screen.getAllByTestId("town-hood");
    expect(hoods).toHaveLength(6);
    const centre = hoods[0] as HTMLElement;
    expect(within(centre).getByRole("button", { expanded: false }).textContent).toContain("10");
    fireEvent.click(within(centre).getByRole("button", { expanded: false }));
    fireEvent.click(within(screen.getAllByTestId("town-hood")[0] as HTMLElement).getByTestId("spot-town_hall"));
    expect(visit).toHaveBeenCalledWith({ type: "place", place: 132 });
    expectNoKnowledgeCount(document.body);
  });

  it("Town: the card without placeholders, and the loose ends (an arc only heard of is a lead with where)", () => {
    const withLead = load<NotebookView>("notebook-arc-lead");
    setup({ notebook: withLead });
    fireEvent.click(screen.getByRole("tab", { name: /Town/ }));
    expect(screen.getByTestId("town-card")).toBeTruthy();
    fireEvent.click(within(screen.getByTestId("notebook-loose-ends")).getByRole("button", { expanded: false }));
    expect(screen.getByTestId("arc-lead").textContent).toContain("Something is going on in Orchardfield.");
    expectNoKnowledgeCount(document.body);
  });

  it("search: one flat list grouped by kind; something not known answers with the way to find out", () => {
    setup();
    fireEvent.click(screen.getByTestId("notebook-search-open"));
    fireEvent.change(screen.getByTestId("notebook-search"), { target: { value: "vargawell" } });
    expect(screen.queryAllByRole("tab")).toHaveLength(0);
    expect(within(screen.getByTestId("notebook-results-people")).getAllByTestId("notebook-person")).toHaveLength(1);
    fireEvent.change(screen.getByTestId("notebook-search"), { target: { value: "abbey green" } });
    const gap = screen.getByTestId("notebook-results-gaps");
    expect(gap.textContent).toContain("Canvass Abbey Green");
    fireEvent.change(screen.getByTestId("notebook-search"), { target: { value: "zzzz" } });
    expect(screen.getByTestId("notebook-results").textContent).toContain("Nothing you know matches “zzzz”.");
    fireEvent.keyDown(screen.getByTestId("notebook-search"), { key: "Escape" });
    expect(screen.getAllByRole("tab")).toHaveLength(4);
  });

  it("looking is seeing: the engine is told once, and again only for something newer", () => {
    const { seeNotebook, loadNotebook } = setup({ notebook_new: 2 });
    expect(loadNotebook).toHaveBeenCalledTimes(1);
    expect(seeNotebook).toHaveBeenCalledTimes(1);
    act(() => useGame.setState({ pocket: { ...(useGame.getState().pocket as PocketView), notebook_new: 0 } }));
    expect(seeNotebook).toHaveBeenCalledTimes(1);
  });
});
