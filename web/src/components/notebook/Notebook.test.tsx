import { act, fireEvent, render, screen, within } from "@testing-library/react";
import type { PocketView } from "../../engine/protocol/PocketView";
import { englishTranslator } from "../../i18n/englishLocale";
import { useGame } from "../../state/gameStore";
import { notebookFixture } from "./fixture";
import { Notebook } from "./Notebook";

const pocket = (notebook_new: number) => ({ notebook_new }) as unknown as PocketView;

function setup(notebook_new = 0, fresh: string[] = []) {
  const seeNotebook = vi.fn(() => Promise.resolve());
  const loadNotebook = vi.fn(() => Promise.resolve());
  const leaveNotebook = vi.fn();
  const visit = vi.fn(() => Promise.resolve());
  useGame.setState({ tr: englishTranslator(), busy: false, notebook: notebookFixture(), notebookFresh: fresh, pocket: pocket(notebook_new), seeNotebook, loadNotebook, leaveNotebook, visit });
  return { seeNotebook, loadNotebook, leaveNotebook, visit };
}

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
    expect(screen.getAllByTestId("notebook-entry")).toHaveLength(1);
    fireEvent.click(screen.getByRole("tab", { name: /Worries/ }));
    expect(screen.getByRole("tabpanel").textContent).toContain("Nothing here matches “vera”.");
    expect(within(screen.getByTestId("notebook-elsewhere")).getByRole("button").textContent).toBe("People (1)");
  });

  it("lists the leads picked up and the ways to find out the rest", () => {
    setup();
    render(<Notebook />);
    fireEvent.click(screen.getByRole("tab", { name: /Leads/ }));
    expect(within(screen.getByTestId("notebook-leads-open")).getAllByTestId("notebook-lead")).toHaveLength(1);
    expect(within(screen.getByTestId("notebook-leads-standing")).getByTestId("notebook-lead").textContent).toContain("Canvass Eastgate");
  });
});
