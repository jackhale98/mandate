/// <reference types="node" />
// Asks as rows (Today, the arc sheet), casework and the promise picker render the engine's views
// (asks.fixture.json: a real game's views, round-tripped through the Rust
// types by crates/mandate-wasm/tests/asks.rs) with every string in Fluent,
// keyboard-reachable buttons with screen-reader labels, and commands in the
// protocol's shape.
import { fireEvent, render, screen, within } from "@testing-library/react";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { AskView } from "../../engine/protocol/AskView";
import type { AsksView } from "../../engine/protocol/AsksView";
import type { CommandDto } from "../../engine/protocol/CommandDto";
import type { PocketView } from "../../engine/protocol/PocketView";
import type { PromiseMenuView } from "../../engine/protocol/PromiseMenuView";
import { englishTranslator } from "../../i18n/englishLocale";
import { useGame } from "../../state/gameStore";
import { barBadges } from "../Pocket";
import { AskRow, CaseworkHere, StoryPromises } from "./Asks";
import { PromisePicker } from "./PromisePicker";

interface Fixture {
  pocket_asks: AskView[];
  asks_recent: AskView[];
  asks_due_soon: number;
  menu: PromiseMenuView;
  asks: AsksView;
  commands: CommandDto[];
}

const fixture = JSON.parse(readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), "asks.fixture.json"), "utf8")) as Fixture;

function pocket(over: Partial<PocketView> = {}): PocketView {
  return { asks: fixture.pocket_asks, asks_recent: fixture.asks_recent, asks_due_soon: fixture.asks_due_soon, can_promise: true, decisions: [], alerts: [], notebook_new: 0, ...over } as PocketView;
}

describe("asks and promises", () => {
  const respondAsk = vi.fn(async () => {});
  const casework = vi.fn(async () => {});
  const openPromises = vi.fn(async () => {});
  const makePromise = vi.fn(async () => {});
  beforeEach(() => {
    respondAsk.mockClear();
    casework.mockClear();
    makePromise.mockClear();
    useGame.setState({ tr: englishTranslator(), busy: false, pocket: pocket(), respondAsk, casework, openPromises, makePromise, promiseMenu: null, asks: null });
  });

  it("an ask as a row: two lines closed, opened in place to read its price, then Accept and Decline, or the next step with the engine's odds", () => {
    const offer = fixture.pocket_asks.find((a) => a.status === "offered");
    const work = fixture.pocket_asks.find((a) => a.status === "accepted" && a.odds !== null);
    expect(offer).toBeDefined();
    expect(work).toBeDefined();
    if (!offer || !work) return;
    render(
      <ul>
        <AskRow a={offer} />
        <AskRow a={work} />
      </ul>,
    );
    const row = screen.getByTestId(`ask-${offer.id}`);
    expect(row.getAttribute("data-status")).toBe("offered");
    expect(row.textContent).not.toContain("⟦");
    expect(row.textContent).not.toMatch(/\{\s*\$/);
    // Closed: no answer yet; the row's button says it opens.
    expect(within(row).queryByTestId(`ask-accept-${offer.id}`)).toBeNull();
    const toggle = within(row).getAllByRole("button")[0] as HTMLElement;
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(toggle);
    const accept = within(screen.getByTestId(`ask-${offer.id}`)).getByTestId(`ask-accept-${offer.id}`);
    expect(accept.getAttribute("aria-label")).toMatch(/^Accept: /);
    fireEvent.click(accept);
    expect(respondAsk).toHaveBeenCalledWith(offer.id, true);
    fireEvent.click(screen.getByTestId(`ask-decline-${offer.id}`));
    expect(respondAsk).toHaveBeenCalledWith(offer.id, false);
    // An accepted casework ask: its next step on the closed row; opened, the engine's odds and "Sort it out".
    const item = screen.getByTestId(`ask-${work.id}`);
    expect(item.textContent).toMatch(/Next: /);
    fireEvent.click(within(item).getAllByRole("button")[0] as HTMLElement);
    expect(within(item).getByTestId("ask-odds").textContent).toContain(`${Math.round((work.odds ?? 0) * 100)}%`);
    expect(within(item).getByTestId("ask-need").textContent).toMatch(/^Next: /);
    expect(item.textContent).toMatch(/Due .* left|Due .* today/);
    const sort = within(item).getByTestId(`ask-casework-${work.id}`);
    expect(sort.getAttribute("aria-label")).toMatch(/^Sort it out at the town hall: /);
    fireEvent.click(sort);
    expect(casework).toHaveBeenCalledWith(work.id);
  });

  it("a settled ask reads as how it ended", () => {
    const kept = fixture.asks_recent.find((a) => a.status === "kept");
    expect(kept).toBeDefined();
    if (!kept) return;
    render(
      <ul>
        <AskRow a={kept} />
      </ul>,
    );
    expect(screen.getByTestId(`ask-${kept.id}`).textContent).toContain("Kept");
  });

  it("the bar's count is the engine's, and says what it counts", () => {
    const tr = englishTranslator();
    const b = barBadges(tr, pocket({ needs_count: 2, notebook_new: 1 }));
    expect(b.needs.n).toBe(2);
    expect(b.needs.text).toBe("2 things need you");
    expect(b.notebook.text).toBe("1 new in your notebook");
  });

  it("casework at the services desk lists what needs sorting, with the odds", () => {
    render(<CaseworkHere />);
    const c = screen.getByTestId("place-casework");
    expect(c.textContent).toContain("Casework");
    expect(within(c).getAllByTestId("ask-odds").length).toBe(fixture.pocket_asks.filter((a) => a.status === "accepted" && a.odds !== null).length);
    expect(c.textContent).not.toContain("⟦");
  });

  it("the promise picker: the engine's menu, what each commits to, and Promise this", () => {
    useGame.setState({ promiseMenu: fixture.menu });
    render(<PromisePicker />);
    const sheet = screen.getByTestId("promise-picker");
    expect(sheet.textContent).not.toContain("⟦");
    expect(sheet.textContent).not.toMatch(/\{\s*\$/);
    expect(within(sheet).getByTestId("promise-made").textContent).toContain(`${fixture.menu.made} of ${fixture.menu.max} promises`);
    const o = fixture.menu.options[0];
    expect(o).toBeDefined();
    if (!o) return;
    const make = within(sheet).getByTestId(`promise-make-${o.template}`);
    expect(make.getAttribute("aria-label")).toMatch(/^Promise: /);
    fireEvent.click(make);
    expect(makePromise).toHaveBeenCalledWith(o.template, o.answer);
  });

  it("the story sheet lists the promises made, kept and broken", () => {
    useGame.setState({ asks: fixture.asks });
    render(<StoryPromises />);
    const s = screen.getByTestId("story-promises");
    expect(s.textContent).toContain(`Made ${fixture.asks.tally.promises_made}`);
    expect(s.textContent).not.toContain("⟦");
  });

  it("the commands have the protocol's shape", () => {
    const types = fixture.commands.map((c) => (c.type === "act" ? `act:${c.action.type}` : c.type));
    expect(types).toEqual(["respond_ask", "respond_ask", "promise", "act:casework"]);
  });
});
