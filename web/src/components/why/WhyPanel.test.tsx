import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { englishTranslator } from "../../i18n/englishLocale";
import { useGame } from "../../state/gameStore";
import type { WhyView } from "../../engine/protocol/WhyView";
import { everythingFixture, lawFixture } from "./fixture";
import { WhyButton } from "./WhyButton";
import { WhyPanel } from "./WhyPanel";
import { useWhy } from "./whyStore";

// The engine's answers: the panel shows them as given.
const request = vi.fn();
vi.mock("../../engine/client", async (original) => ({
  ...(await original<typeof import("../../engine/client")>()),
  engine: () => ({ request }),
}));

const answer = (view: WhyView) => Promise.resolve({ kind: "why", payload: view });

function setup() {
  useGame.setState({ tr: englishTranslator() });
  useWhy.setState({ open: null });
  request.mockReset();
}

describe("the Why control and its panel", () => {
  beforeEach(setup);

  it("asks the engine about the event and lays its answer out, newest first", async () => {
    request.mockImplementation(() => answer(lawFixture()));
    render(
      <>
        <WhyButton event={40} about="Transit frequency" />
        <WhyPanel />
      </>,
    );
    const control = screen.getByRole("button", { name: "Why? (Transit frequency)" });
    expect(control).toHaveTextContent("Why?");
    fireEvent.click(control);
    const panel = await screen.findByTestId("why-panel");
    expect(request).toHaveBeenCalledWith({ kind: "query", payload: { type: "why", event: 40, depth: null } });
    const steps = await within(panel).findAllByTestId("why-step");
    expect(steps.map((s) => within(s).getByText(/\.$/, { selector: ".why-text" }).textContent)).toEqual([
      "Iverland passed the Transit frequency bill.",
      "You got something done.",
      "A new bill in Iverland: Transit frequency.",
      "A new proposal came onto the table.",
    ]);
    expect(within(panel).getAllByTestId("why-join").map((j) => j.textContent)).toEqual(["after", "after", "prompted by"]);
    // The line at the top: the player's own act is behind it, named, with its day.
    expect(within(panel).getByTestId("why-player")).toHaveTextContent("Something you did is behind this. You declared your vote on a bill. · 5 March 2031");
    // What a step names as a start is under that step.
    expect(within(steps[2] as HTMLElement).getByTestId("why-root")).toHaveTextContent("You introduced a bill.");
    expect(within(panel).queryByTestId("why-more")).toBeNull();
    expect(panel).not.toHaveTextContent(/⟦|why-|\{\s*\$/);
  });

  it("takes the keyboard when it opens, closes on Escape and gives the keyboard back; nothing holds it there", async () => {
    request.mockImplementation(() => answer(lawFixture()));
    render(
      <>
        <WhyButton event={40} />
        <button type="button">Elsewhere</button>
        <WhyPanel />
      </>,
    );
    const control = screen.getByRole("button", { name: "Why?" });
    control.focus();
    fireEvent.click(control);
    const panel = await screen.findByTestId("why-panel");
    expect(panel).toHaveAccessibleName("Why this happened");
    expect(screen.getByRole("heading", { name: "Why this happened" })).toHaveFocus();
    // Focus can leave the panel: it is not a trap.
    screen.getByRole("button", { name: "Elsewhere" }).focus();
    expect(screen.getByRole("button", { name: "Elsewhere" })).toHaveFocus();
    fireEvent.keyDown(document, { key: "Escape" });
    await waitFor(() => expect(screen.queryByTestId("why-panel")).toBeNull());
    expect(control).toHaveFocus();
    // The close button does the same.
    fireEvent.click(control);
    fireEvent.click(within(await screen.findByTestId("why-panel")).getByRole("button", { name: "Close" }));
    await waitFor(() => expect(screen.queryByTestId("why-panel")).toBeNull());
  });

  it("says more lies further back and asks once for the longest walk", async () => {
    const short = everythingFixture();
    const long: WhyView = { ...short, depth: 12, truncated: true };
    request.mockImplementation((body: { payload: { depth: number | null } }) => answer(body.payload.depth === null ? short : long));
    render(
      <>
        <WhyButton event={90} />
        <WhyPanel />
      </>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Why?" }));
    const more = await screen.findByTestId("why-more");
    expect(more).toHaveTextContent("More lies further back.");
    await act(async () => {
      fireEvent.click(within(more).getByRole("button", { name: "Go further back" }));
      await Promise.resolve();
    });
    expect(request).toHaveBeenLastCalledWith({ kind: "query", payload: { type: "why", event: 90, depth: 12 } });
    // As far as the engine walks: the note stays, the offer goes.
    await waitFor(() => expect(screen.queryByTestId("why-further")).toBeNull());
    expect(screen.getByTestId("why-more")).toHaveTextContent("More lies further back.");
    // A second line of causes says which step it joins.
    expect(screen.getAllByTestId("why-join").map((j) => j.textContent)).toContain("And “You kept your word: Jobs first.” which ended");
  });

  it("shows a race's factors beside the chain, as the result gave them", async () => {
    request.mockImplementation(() => answer(lawFixture()));
    render(
      <>
        <WhyButton event={40} factors={{ factors: [{ factor: "campaign", margin: 0.04 }, { factor: "shock", margin: -0.01 }], sign: 1 }} />
        <WhyPanel />
      </>,
    );
    fireEvent.click(screen.getByRole("button", { name: "Why?" }));
    const factors = await screen.findByTestId("why-factors");
    expect(factors).toHaveTextContent("Why it went this way");
    expect(factors).toHaveTextContent("Campaigning");
    expect(factors).toHaveTextContent("+4");
    expect(factors).toHaveTextContent("Late swing and luck");
  });

  it("says so when the record no longer keeps the event, and shows no control where a view names none", async () => {
    request.mockImplementation(() => Promise.reject(new Error("no such event")));
    const { container } = render(
      <>
        <WhyButton event={3} />
        <span data-testid="none">
          <WhyButton event={null} />
          <WhyButton event={undefined} />
        </span>
        <WhyPanel />
      </>,
    );
    expect(within(screen.getByTestId("none")).queryByRole("button")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Why?" }));
    expect(await screen.findByTestId("why-gone")).toHaveTextContent("The record no longer keeps how this came about.");
    expect(container.querySelector("[data-testid=why-chain]")).toBeNull();
  });
});
