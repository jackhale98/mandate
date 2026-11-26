// The career card (M6 note 23) against the engine's own views
// (fixtures/card/*.json, written by crates/mandate-wasm/tests/career_card.rs):
// a run stepped away from on the first turn, and one stepped away from in a
// campaign in the middle of a scene. The card shows every section with no
// unfilled argument, says unfinished, names the race walked away from; Copy
// puts the text block on the clipboard; Share falls back to saving the image
// and copying the text where the device cannot share; the step-away dialog
// asks once.
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { englishTranslator } from "../../i18n/englishLocale";
import { useGame } from "../../state/gameStore";
import type { CareerCardView } from "../../engine/protocol/CareerCardView";
import first from "../fixtures/card/first_turn.json";
import midRace from "../fixtures/card/mid_race.json";
import { CareerCard } from "./CareerCard";
import { StepAway } from "./StepAway";
import { cardText, pct, points, signed } from "./cardText";
import * as share from "./share";

const firstTurn = first as CareerCardView;
const mid = midRace as CareerCardView;
const tr = englishTranslator();

function setup(card: CareerCardView | null, over: Partial<ReturnType<typeof useGame.getState>> = {}) {
  useGame.setState({ tr, careerCard: card, cardLoading: false, progress: null, ...over });
}

/** Fluent leaves `{$x}` (or `⟦key⟧` for a missing key) where something is unfilled. */
const unfilled = /\{|\}|\$|⟦/;

describe("the career card", () => {
  afterEach(() => vi.restoreAllMocks());

  it("shows every section of a run stepped away from on the first turn, marked unfinished", () => {
    setup(firstTurn);
    render(<CareerCard />);
    const c = screen.getByTestId("career-card");
    expect(c.dataset.unfinished).toBe("true");
    expect(screen.getByTestId("card-unfinished").textContent).toBe("Unfinished");
    expect(within(c).getByRole("heading", { name: firstTurn.name })).toBeTruthy();
    expect(c.textContent).toContain(firstTurn.town);
    expect(c.textContent).toContain(tr.t(`ui-card-size-${firstTurn.world.size}`));
    // The opening, with its rating and the facts behind it.
    const r = firstTurn.opening.rating;
    expect(r).not.toBeNull();
    const rating = screen.getByTestId("card-opening-rating").textContent ?? "";
    expect(rating).toContain(pct(r?.win_pm ?? 0));
    for (const f of r?.facts ?? []) expect(rating).toContain(tr.t(f.key));
    expect(screen.getByTestId("card-ended").textContent).toContain("Stepped away");
    expect(screen.getByTestId("card-code").textContent).toBe(firstTurn.code);
    expect(screen.getByTestId("card-build").textContent).toContain(firstTurn.build);
    expect(c.textContent).not.toMatch(unfilled);
  });

  it("names the race walked away from, with its rating at the time, and the matter under way", () => {
    setup(mid);
    render(<CareerCard />);
    const walked = screen.getByTestId("card-races").querySelector('[data-outcome="walked_away"]');
    expect(walked?.textContent).toContain("walked away from");
    expect(walked?.textContent).toMatch(/rated \d+%/);
    expect(screen.getByTestId("card-arcs").textContent).toContain("still under way");
    expect(screen.getByTestId("career-card").textContent).not.toMatch(unfilled);
  });

  it("the text block holds the same facts, short, with the code and the build", () => {
    for (const c of [firstTurn, mid]) {
      const text = cardText(tr, c);
      expect(text.split("\n")[0]).toBe("Mandate career card (unfinished)");
      expect(text).toContain(c.name);
      expect(text).toContain(c.code ?? "");
      expect(text).toContain(c.build);
      expect(text).not.toMatch(unfilled);
      expect(text.split("\n").length).toBeLessThan(24);
    }
    expect(cardText(tr, mid)).toContain("walked away from");
  });

  it("formats the engine's thousandths without computing anything", () => {
    expect(pct(750)).toBe("75%");
    expect(signed(250)).toBe("+0.25");
    expect(signed(-1500)).toBe("−1.50");
    expect(signed(0)).toBe("0.00");
    expect(points(122)).toBe("+12");
    expect(points(-131)).toBe("−13");
  });

  it("Copy puts the text block on the clipboard", async () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    setup(firstTurn);
    render(<CareerCard />);
    fireEvent.click(screen.getByTestId("card-copy"));
    await waitFor(() => expect(screen.getByTestId("card-status").textContent).toBe("Copied."));
    expect(writeText).toHaveBeenCalledWith(cardText(tr, firstTurn));
  });

  it("Share without a share sheet saves the image and copies the text", async () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    Object.defineProperty(navigator, "share", { value: undefined, configurable: true });
    Object.defineProperty(URL, "createObjectURL", { value: vi.fn(() => "blob:card"), configurable: true });
    Object.defineProperty(URL, "revokeObjectURL", { value: vi.fn(), configurable: true });
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
    const r = await share.shareCard("the text", "title", () => Promise.resolve(new Blob(["png"], { type: "image/png" })));
    expect(r).toBe("saved");
    expect(click).toHaveBeenCalledTimes(1);
    expect(writeText).toHaveBeenCalledWith("the text");
  });

  it("Share with a share sheet sends the image and the text", async () => {
    const shareFn = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "share", { value: shareFn, configurable: true });
    Object.defineProperty(navigator, "canShare", { value: () => true, configurable: true });
    const r = await share.shareCard("the text", "title", () => Promise.resolve(new Blob(["png"], { type: "image/png" })));
    expect(r).toBe("shared");
    const arg = (shareFn.mock.calls[0] as unknown as [ShareData])[0];
    expect(arg.text).toBe("the text");
    expect(arg.files?.[0]?.name).toBe(share.CARD_FILE);
    Object.defineProperty(navigator, "share", { value: undefined, configurable: true });
    Object.defineProperty(navigator, "canShare", { value: undefined, configurable: true });
  });

  it("while the engine makes the card, shows its progress", () => {
    setup(null, { cardLoading: true, progress: { stage: "progress-card", fraction: 0.4 } });
    render(<CareerCard />);
    expect(screen.getByTestId("card-loading").textContent).toContain("Measuring how your career began");
  });
});

describe("stepping away", () => {
  it("asks once, and stepping away sends the command", async () => {
    const stepAway = vi.fn(() => Promise.resolve());
    const openStepAway = vi.fn((open: boolean) => useGame.setState({ stepAwayOpen: open }));
    useGame.setState({ tr, stepAwayOpen: true, busy: false, campaign: null, stepAway, openStepAway });
    render(<StepAway />);
    const d = screen.getByTestId("step-away-dialog");
    expect(d.textContent).toContain("Step away from politics?");
    expect(d.textContent).toContain("unfinished");
    fireEvent.click(screen.getByTestId("step-away-yes"));
    expect(stepAway).toHaveBeenCalledTimes(1);
    act(() => {
      fireEvent.click(screen.getByTestId("step-away-no"));
    });
    expect(openStepAway).toHaveBeenLastCalledWith(false);
  });
});
