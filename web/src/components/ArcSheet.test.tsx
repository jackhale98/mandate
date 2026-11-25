/// <reference types="node" />
// The arc sheet (A3) against the engine's arc views (fixtures/pocket/arc*.json,
// written by crates/mandate-wasm/tests/pocket_views.rs): where it stands and its
// clock, every beat as the command that takes it, the cast, what happened;
// below "known" a lead and nothing else; an ended arc says how it ended.
import { fireEvent, render, screen, within } from "@testing-library/react";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { ArcView } from "../engine/protocol/ArcView";
import type { PocketView } from "../engine/protocol/PocketView";
import { englishTranslator } from "../i18n/englishLocale";
import { useGame } from "../state/gameStore";
import { ArcSheet } from "./ArcSheet";

const dir = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "fixtures/pocket",
);
const load = <T,>(name: string) =>
  JSON.parse(readFileSync(path.join(dir, `${name}.json`), "utf8")) as T;

function setup(arc: ArcView) {
  const calls = {
    openDecision: vi.fn(() => Promise.resolve()),
    openTalk: vi.fn(() => Promise.resolve()),
    visit: vi.fn(() => Promise.resolve()),
    openPanel: vi.fn(() => Promise.resolve()),
    openArc: vi.fn(() => Promise.resolve()),
    respondAsk: vi.fn(() => Promise.resolve()),
  };
  useGame.setState({
    tr: englishTranslator(),
    busy: false,
    arc,
    pocket: load<PocketView>("pocket"),
    folds: {},
    ...calls,
  });
  render(<ArcSheet />);
  return calls;
}

describe("the arc sheet", () => {
  it("says where it stands and its clock, and lists what you can do, each going straight to its command or route", () => {
    const c = setup(load<ArcView>("arc"));
    const sheet = screen.getByTestId("arc-sheet");
    expect(sheet.textContent).not.toContain("⟦");
    expect(sheet.textContent).not.toMatch(/\{\s*\$/);
    expect(screen.getByRole("heading", { level: 2 }).textContent).toBe(
      "After the water",
    );
    expect(screen.getByTestId("arc-clock").textContent).toMatch(
      /^Moves on in \d+ days/,
    );
    expect(screen.getByTestId("arc-stage").textContent).not.toBe("");
    const beats = within(screen.getByTestId("arc-beats"));
    // The decision: its sheet.
    fireEvent.click(beats.getByTestId("arc-beat-act-money"));
    expect(c.openDecision).toHaveBeenCalledWith(0);
    // The visit: done where the player is, its place's panel opens on the verb.
    const arcView = load<ArcView>("arc");
    const box = arcView.beats.find((x) => x.beat === "box");
    fireEvent.click(beats.getByTestId("arc-beat-act-box"));
    expect(c.openPanel).toHaveBeenCalledWith(box?.route?.to?.location);
    // The ask, when the arc has one on offer: the ask itself, opened in place to be answered.
    const askId = arcView.beats.find((x) => x.ask !== null)?.ask;
    if (askId != null) {
      const ask = beats.getByTestId(`ask-${askId}`);
      fireEvent.click(within(ask).getAllByRole("button")[0] as HTMLElement);
      fireEvent.click(
        within(beats.getByTestId(`ask-${askId}`)).getByTestId(
          `ask-accept-${askId}`,
        ),
      );
      expect(c.respondAsk).toHaveBeenCalledWith(askId, true);
    }
    // Who is in it: people with Talk.
    const who = arcView.cast.find((x) => x.person?.can_talk)?.person;
    const cast = within(screen.getByTestId("arc-cast-list"));
    fireEvent.click(cast.getByRole("button", { name: `Talk to ${who?.name}` }));
    expect(c.openTalk).toHaveBeenCalledWith({
      type: "person",
      person: who?.id,
    });
    // Heat is never drawn.
    expect(sheet.querySelector(".bar")).toBeNull();
  });

  it("below known shows a lead only: where to ask, never who or what", () => {
    setup(load<ArcView>("arc-heard"));
    const sheet = screen.getByTestId("arc-sheet");
    expect(screen.getByTestId("arc-lead-only").textContent).toContain(
      "Something is going on in Orchardfield.",
    );
    expect(screen.queryByTestId("arc-beats")).toBeNull();
    expect(screen.queryByTestId("arc-cast-list")).toBeNull();
    expect(screen.queryByTestId("arc-clock")).toBeNull();
    expect(sheet.textContent).not.toContain("water");
  });

  it("an ended arc says how it ended, and offers nothing to do", () => {
    setup(load<ArcView>("arc-ended"));
    expect(screen.getByTestId("arc-ended").textContent).toMatch(/Ended /);
    expect(screen.queryByTestId("arc-beats")).toBeNull();
  });
});
