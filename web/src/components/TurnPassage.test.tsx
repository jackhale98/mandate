import { act, fireEvent, render, screen } from "@testing-library/react";
import { englishTranslator } from "../i18n/englishLocale";
import { useGame } from "../state/gameStore";
import { PASSAGE_MS, TurnPassage, passageSpan } from "./TurnPassage";

const from = { day_number: 0, year: 2020, month: 1, day: 1 };
const to = { day_number: 14, year: 2020, month: 1, day: 15 };

describe("the night between turns", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useGame.setState({ tr: englishTranslator(), passage: null, afterPassage: { electionNight: false }, electionNight: false });
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("goes dusk → night → (the engine answers) → dawn → done, and only then opens what waited for it", () => {
    useGame.setState({ passage: { phase: "dusk", from, to: null, days: 14, id: 1 } });
    render(<TurnPassage />);
    expect(screen.getByTestId("turn-passage").dataset.phase).toBe("dusk");
    act(() => vi.advanceTimersByTime(PASSAGE_MS.dusk));
    expect(useGame.getState().passage?.phase).toBe("night");
    // The engine is slow: the night holds, with the loader.
    act(() => vi.advanceTimersByTime(5000));
    expect(useGame.getState().passage?.phase).toBe("night");
    expect(screen.queryByTestId("turn-passage-date")).toBeNull();
    // It answers, and an election night is due: it waits for the passage.
    act(() => useGame.setState({ passage: { phase: "night", from, to, days: 14, id: 1 }, afterPassage: { electionNight: true } }));
    expect(screen.getByTestId("turn-passage-date").textContent).toBe("15 January 2020");
    expect(screen.getByRole("status").textContent).toContain("2 weeks later");
    act(() => vi.advanceTimersByTime(0));
    expect(useGame.getState().passage?.phase).toBe("dawn");
    expect(useGame.getState().electionNight).toBe(false);
    act(() => vi.advanceTimersByTime(PASSAGE_MS.dawn));
    expect(useGame.getState().passage).toBeNull();
    expect(useGame.getState().electionNight).toBe(true);
    expect(screen.queryByTestId("turn-passage")).toBeNull();
  });

  it("lasts at least the night's length even when the engine answers at once", () => {
    useGame.setState({ passage: { phase: "dusk", from, to, days: 7, id: 2 } });
    render(<TurnPassage />);
    act(() => vi.advanceTimersByTime(PASSAGE_MS.dusk));
    expect(useGame.getState().passage?.phase).toBe("night");
    act(() => vi.advanceTimersByTime(PASSAGE_MS.night - 50));
    expect(useGame.getState().passage?.phase).toBe("night");
    act(() => vi.advanceTimersByTime(60));
    expect(useGame.getState().passage?.phase).toBe("dawn");
  });

  it("skips to the end on a tap or a key once the engine has answered, not before", () => {
    useGame.setState({ passage: { phase: "night", from, to: null, days: 7, id: 3 } });
    render(<TurnPassage />);
    fireEvent.click(screen.getByTestId("turn-passage"));
    fireEvent.keyDown(window, { key: "Enter" });
    expect(useGame.getState().passage).not.toBeNull();
    act(() => useGame.setState({ passage: { phase: "night", from, to, days: 7, id: 3 } }));
    fireEvent.keyDown(window, { key: "Tab" });
    expect(useGame.getState().passage).not.toBeNull();
    fireEvent.keyDown(window, { key: " " });
    expect(useGame.getState().passage).toBeNull();
  });

  it("says how long the turn was from the engine's day count", () => {
    const tr = englishTranslator();
    expect(passageSpan(tr, 1)).toBe("A day later");
    expect(passageSpan(tr, 7)).toBe("A week later");
    expect(passageSpan(tr, 14)).toBe("2 weeks later");
    expect(passageSpan(tr, 10)).toBe("10 days later");
  });
});
