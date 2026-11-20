import { act, fireEvent, render, screen, within } from "@testing-library/react";
import type { PocketView } from "../../engine/protocol/PocketView";
import { englishTranslator } from "../../i18n/englishLocale";
import { useGame } from "../../state/gameStore";
import type { TownPlacesView } from "../../engine/protocol/TownPlacesView";
import { notebookFixture, officeFixture, seenAtDiner, sheetFixture, townCharacterFixture } from "./fixture";
import { Notebook } from "./Notebook";
import type { SheetLoader } from "./People";

const pocket = (notebook_new: number) => ({ notebook_new }) as unknown as PocketView;

function setup(notebook_new = 0, fresh: string[] = []) {
  const seeNotebook = vi.fn(() => Promise.resolve());
  const loadNotebook = vi.fn(() => Promise.resolve());
  const leaveNotebook = vi.fn();
  const visit = vi.fn(() => Promise.resolve());
  const openTalk = vi.fn(() => Promise.resolve());
  const openCharacter = vi.fn(() => Promise.resolve());
  useGame.setState({ tr: englishTranslator(), busy: false, notebook: notebookFixture(), notebookFresh: fresh, pocket: pocket(notebook_new), office: null, town: null, seeNotebook, loadNotebook, leaveNotebook, visit, openTalk, openCharacter });
  return { seeNotebook, loadNotebook, leaveNotebook, visit, openTalk, openCharacter };
}

/** Character sheets as the engine would send them (Vera usually at the diner; the executive cool). */
const sheets: SheetLoader = (ids) => Promise.resolve(new Map(ids.map((id) => [id, sheetFixture(id, id === 12 ? { usually: seenAtDiner } : id === 21 ? { opinion_band: "cool", trust_band: "low" } : {})])));

describe("the notebook page", () => {
  it("is a tab list of sections; placeholders say what is not known, with the lead that would reveal it", () => {
    const { visit } = setup();
    render(<Notebook />);
    expect(screen.getByRole("heading", { name: "What you know about Ashford" })).toBeTruthy();
    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((t) => t.getAttribute("id"))).toEqual(["notebook-tab-people", "notebook-tab-places", "notebook-tab-issues", "notebook-tab-policies", "notebook-tab-leads"]);
    fireEvent.click(screen.getByRole("tab", { name: /Worries/ }));
    const panel = screen.getByRole("tabpanel");
    expect(panel.getAttribute("aria-labelledby")).toBe("notebook-tab-issues");
    expect(within(panel).getByRole("heading", { name: "By neighbourhood" })).toBeTruthy();
    const gap = within(panel).getByTestId("notebook-gap");
    expect(within(gap).getByRole("heading").textContent).toBe("What Eastgate worries about");
    // The level in words beside its dots (not colour alone).
    expect(gap.textContent).toContain("○○○");
    expect(gap.textContent).toContain("Not known yet");
    expect(within(gap).getByTestId("notebook-lead").textContent).toContain("Canvass Eastgate to hear what worries people there.");
    fireEvent.click(within(gap).getByTestId("notebook-lead-go"));
    expect(visit).toHaveBeenCalledWith({ type: "street", neighborhood: 3 });
    // A known entry: what, how known, from where and when.
    const known = within(panel).getAllByTestId("notebook-entry");
    expect(known.map((k) => k.getAttribute("data-level"))).toEqual(["known", "known"]);
    expect(within(known[1] as HTMLElement).getByTestId("notebook-from").textContent).toMatch(/ · Canvass a neighbourhood, /);
  });

  it("marks new entries, lands on their tab and tells the engine they have been seen", () => {
    const westfield = JSON.stringify({ type: "hood_concerns", hood: 2 });
    const { seeNotebook, loadNotebook } = setup(1, [westfield]);
    const { unmount } = render(<Notebook />);
    expect(loadNotebook).toHaveBeenCalledTimes(1);
    expect(seeNotebook).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("tab", { selected: true }).id).toBe("notebook-tab-issues");
    expect(screen.getByTestId("notebook-tab-badge-issues").textContent).toBe("1");
    expect(screen.getByTestId("notebook-fresh").textContent).toBe("One new entry since you last looked.");
    const fresh = screen.getAllByTestId("notebook-new");
    expect(fresh).toHaveLength(1);
    expect((fresh[0] as HTMLElement).closest("li")?.textContent).toContain("Westfield");
    // Seen: the engine's count is zero and no second call is made.
    act(() => useGame.setState({ pocket: pocket(0) }));
    expect(seeNotebook).toHaveBeenCalledTimes(1);
    unmount();
    expect(useGame.getState().leaveNotebook).toHaveBeenCalled();
  });

  it("searches across sections and says where else something matches", () => {
    setup();
    render(<Notebook />);
    fireEvent.click(screen.getByRole("tab", { name: /People/ }));
    fireEvent.change(screen.getByTestId("notebook-search"), { target: { value: "vera" } });
    expect(screen.getAllByTestId("notebook-person")).toHaveLength(1);
    fireEvent.click(screen.getByRole("tab", { name: /Worries/ }));
    expect(screen.getByRole("tabpanel").textContent).toContain("Nothing here matches “vera”.");
    expect(within(screen.getByTestId("notebook-elsewhere")).getByRole("button").textContent).toBe("People (1)");
  });

  it("People is the single place for people: the key figures pinned, then everyone else, each with Talk and Go to them", async () => {
    const { openTalk, openCharacter } = setup();
    useGame.setState({ office: officeFixture(), town: { character: townCharacterFixture() } as unknown as TownPlacesView });
    render(<Notebook loadSheets={sheets} />);
    fireEvent.click(screen.getByRole("tab", { name: /People/ }));
    const pinned = within(screen.getByTestId("notebook-key-figures"));
    expect(pinned.getByRole("heading", { name: "Key figures" })).toBeTruthy();
    const cards = pinned.getAllByTestId("notebook-person");
    // Who runs things, the rival, the executive, the members (the player left out).
    expect(cards.map((c) => c.getAttribute("data-person"))).toEqual(["30", "31", "21", "12", "22"]);
    expect(cards[0]?.textContent).toContain("Power broker");
    expect(cards[0]?.textContent).toContain("(rumoured)");
    expect(cards[1]?.textContent).toContain(`Your rival · ${englishTranslator().t("role-councillor")} · Ward 1`);
    expect(cards[2]?.textContent).toContain(englishTranslator().t("role-mayor"));
    // How they stand with you, what you know and where you saw them (S1).
    const vera = cards[3] as HTMLElement;
    expect(await within(vera).findByText("Usually at Rosa's in the afternoon.", { exact: false })).toBeTruthy();
    expect(within(vera).getByTestId("notebook-standing").textContent).toBe("Warm toward you · Trusts you a little");
    expect(vera.textContent).toContain("●●○ Known");
    expect(within(cards[2] as HTMLElement).getByTestId("notebook-standing").textContent).toBe("Cool toward you · Doesn't trust you");
    // Talk opens the dialogue; the name opens the sheet; Go to them is there for each.
    fireEvent.click(within(vera).getByRole("button", { name: "Talk to Vera Holt" }));
    expect(openTalk).toHaveBeenCalledWith({ type: "person", person: 12 });
    fireEvent.click(within(vera).getByTestId("notebook-person-name"));
    expect(openCharacter).toHaveBeenCalledWith(12);
    expect(within(vera).getByRole("button", { name: "Go to Vera Holt" })).toBeTruthy();
    // Everyone else known, with the same card.
    const others = within(screen.getByTestId("notebook-people-others")).getAllByTestId("notebook-person");
    expect(others.map((c) => c.getAttribute("data-person"))).toEqual(["4"]);
    expect(within(others[0] as HTMLElement).getByRole("button", { name: "Talk to Albert Crane" })).toBeTruthy();
    // Placeholders stay (who runs things, the rival's plans).
    expect(within(screen.getByTestId("notebook-people-power")).getAllByTestId("notebook-gap")).toHaveLength(2);
  });

  it("lists the leads picked up and the ways to find out the rest", () => {
    setup();
    render(<Notebook />);
    fireEvent.click(screen.getByRole("tab", { name: /Leads/ }));
    expect(within(screen.getByTestId("notebook-leads-open")).getAllByTestId("notebook-lead")).toHaveLength(1);
    expect(within(screen.getByTestId("notebook-leads-standing")).getByTestId("notebook-lead").textContent).toContain("Canvass Eastgate");
  });
});
