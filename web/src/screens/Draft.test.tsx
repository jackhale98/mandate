// The draft screen (DEC-101 §3; STAGED_ARCS A8) against the engine's own view
// (fixtures/draft/draft_3.json, written by crates/mandate-wasm/tests/draft.rs):
// each opening shows its town, its line, the seat and its tag, and a button to
// begin there; the screen names no score; while the audition runs it shows
// progress, and choosing sends the opening's place in the draft.
import { fireEvent, render, screen, within } from "@testing-library/react";
import { englishTranslator } from "../i18n/englishLocale";
import { useGame } from "../state/gameStore";
import type { DraftView } from "../engine/protocol/DraftView";
import fixture from "../components/fixtures/draft/draft_3.json";
import { Draft, tagWords } from "./Draft";

const draft = fixture as DraftView;

function setup(over: Partial<ReturnType<typeof useGame.getState>> = {}) {
  const chooseOpening = vi.fn(() => Promise.resolve());
  const rerollDraft = vi.fn(() => Promise.resolve());
  const leaveDraft = vi.fn();
  useGame.setState({ tr: englishTranslator(), busy: false, draft, draftProgress: null, chooseOpening, rerollDraft, leaveDraft, ...over });
  return { chooseOpening, rerollDraft, leaveDraft };
}

describe("the draft screen", () => {
  it("shows each opening with its town, line, seat and tag, best first", () => {
    setup();
    render(<Draft />);
    const cards = screen.getAllByTestId("draft-opening");
    expect(cards).toHaveLength(draft.openings.length);
    expect(draft.openings.length).toBeGreaterThan(0);
    const tr = englishTranslator();
    draft.openings.forEach((o, i) => {
      const card = cards[i] as HTMLElement;
      expect(within(card).getByRole("heading", { name: o.town_name })).toBeTruthy();
      const pitch = within(card).getByTestId("draft-pitch").textContent ?? "";
      expect(pitch.length).toBeGreaterThan(20);
      // Every argument the line names is filled (Fluent leaves `{$x}` otherwise).
      expect(pitch).not.toMatch(/\{|\}|\$/);
      expect(within(card).getByTestId("draft-seat").textContent).toContain(tr.t(o.post?.role_key ?? ""));
      for (const w of tagWords(tr, o)) expect(card.textContent).toContain(w);
      expect(within(card).getByRole("button", { name: `Begin in ${o.town_name}` })).toBeTruthy();
    });
  });

  it("names no score", () => {
    setup();
    const { container } = render(<Draft />);
    expect(container.textContent?.toLowerCase()).not.toMatch(/score|agency|escalation|visibility|pacing/);
    expect(JSON.stringify(draft)).not.toMatch(/score|agency|escalation|visibility|pacing|total/);
  });

  it("choosing sends the opening's place; surprise me takes the first; other openings rerolls; back leaves", () => {
    const { chooseOpening, rerollDraft, leaveDraft } = setup();
    render(<Draft />);
    const last = draft.openings.length - 1;
    fireEvent.click(within(screen.getAllByTestId("draft-opening")[last] as HTMLElement).getByTestId("draft-choose"));
    expect(chooseOpening).toHaveBeenLastCalledWith(last);
    fireEvent.click(screen.getByRole("button", { name: "Surprise me" }));
    expect(chooseOpening).toHaveBeenLastCalledWith(0);
    fireEvent.click(screen.getByRole("button", { name: "Other openings" }));
    expect(rerollDraft).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(leaveDraft).toHaveBeenCalledTimes(1);
  });

  it("while the engine auditions, shows its progress and no opening", () => {
    setup({ draft: null, draftProgress: { done: 3, total: 14 } });
    render(<Draft />);
    const status = screen.getByTestId("draft-progress");
    expect(status.textContent).toContain("Looking for a place to begin");
    const bar = within(status).getByRole("progressbar") as HTMLProgressElement;
    expect(bar.value).toBe(3);
    expect(bar.max).toBe(14);
    expect(screen.queryByTestId("draft-opening")).toBeNull();
    expect(screen.queryByRole("button", { name: "Surprise me" })).toBeNull();
  });
});
