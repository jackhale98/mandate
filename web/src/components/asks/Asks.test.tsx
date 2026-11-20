/// <reference types="node" />
// The asks block, casework and the promise picker render the engine's views
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
import { pocketBadges } from "../Pocket";
import { CaseworkHere, StoryPromises, TodayAsks } from "./Asks";
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

  it("the Today block: offers with Accept and Decline, accepted asks with their next step and deadline, recent outcomes", () => {
    render(<TodayAsks p={pocket()} />);
    const block = screen.getByTestId("pocket-asks");
    expect(block.textContent).not.toContain("⟦");
    expect(block.textContent).not.toMatch(/\{\s*\$/);
    const offer = fixture.pocket_asks.find((a) => a.status === "offered");
    expect(offer).toBeDefined();
    if (!offer) return;
    const accept = screen.getByTestId(`ask-accept-${offer.id}`);
    expect(accept.getAttribute("aria-label")).toMatch(/^Accept: /);
    fireEvent.click(accept);
    expect(respondAsk).toHaveBeenCalledWith(offer.id, true);
    fireEvent.click(screen.getByTestId(`ask-decline-${offer.id}`));
    expect(respondAsk).toHaveBeenCalledWith(offer.id, false);
    // An accepted casework ask: the engine's odds, and "Sort it out" with its label.
    const work = fixture.pocket_asks.find((a) => a.status === "accepted" && a.odds !== null);
    expect(work).toBeDefined();
    if (!work) return;
    const item = screen.getByTestId(`ask-${work.id}`);
    expect(within(item).getByTestId("ask-odds").textContent).toContain(`${Math.round((work.odds ?? 0) * 100)}%`);
    expect(within(item).getByTestId("ask-need").textContent).toMatch(/^Next: /);
    expect(item.textContent).toMatch(/Due .* left|Due .* today/);
    const sort = within(item).getByTestId(`ask-casework-${work.id}`);
    expect(sort.getAttribute("aria-label")).toMatch(/^Sort it out at the town hall: /);
    fireEvent.click(sort);
    expect(casework).toHaveBeenCalledWith(work.id);
    // Kept asks are marked kept among the last outcomes.
    const kept = within(screen.getByTestId("asks-recent")).getAllByTestId("ask-status");
    expect(kept.map((k) => k.textContent)).toContain("Kept");
    // The picker opens while campaigning.
    fireEvent.click(screen.getByTestId("open-promises"));
    expect(openPromises).toHaveBeenCalledWith(true);
  });

  it("the badge counts offers and asks due within the turn, and says so", () => {
    const tr = englishTranslator();
    const b = pocketBadges(tr, pocket({ asks_due_soon: 2 }));
    expect(b.today?.n).toBe(2);
    expect(b.today?.text).toBe("2 asks or promises waiting");
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
