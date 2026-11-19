import { act, fireEvent, render, screen } from "@testing-library/react";
import { TemplateCard, PREVIEW_DEBOUNCE_MS } from "./Desk";
import { useGame } from "../state/gameStore";
import { Translator } from "../i18n/fluent";
import { formatMoney } from "../i18n/format";
import type { TemplateView } from "../engine/protocol/TemplateView";
import type { ParamValue } from "../engine/protocol/ParamValue";
import type { PolicyPreviewView } from "../engine/protocol/PolicyPreviewView";

const FTL = `
policy-transit = Transit frequency
policy-transit-desc = Run buses more often.
param-service-level = Service level
unit-level = level
ui-est-cost = Estimated cost { $amount } / year
ui-impl-months = { $n } months to build
ui-budget-share = { $pct }% of the town budget
ui-preview-updating = Updating estimate…
ui-support = Support
ui-tradeoffs = Trade-offs
ui-draft = Create draft
sidefx-disruption = Disruption while it is built ({ $size }).
sidefx-group-all = residents
sidefx-size-small = small
sidefx-size-moderate = moderate
sidefx-size-large = large
`;

const template: TemplateView = {
  id: "policy.transit_frequency",
  name_key: "policy-transit",
  desc_key: "policy-transit-desc",
  category: "infrastructure",
  params: [{ id: "level", name_key: "param-service-level", unit_key: "unit-level", min: 1, max: 5, step: 1, default: 2 }],
  implementation_months: 9,
  secondary_keys: [],
  support: { low: 0.1, middle: 0, high: -0.1, overall: 0 },
  est_cost_default: 50_000,
  side_effects: [{ kind: "disruption", group: "all", older: false, amount: 0.06, size: "moderate" }],
  situational: false,
  available: true,
  unlock_reason_key: null,
};

// The engine's answer (an arbitrary function of the level: the UI must show
// it as given, never compute its own).
function engineAnswer(_template: string, params: ParamValue[]): Promise<PolicyPreviewView> {
  const level = params.find((p) => p.id === "level")?.value ?? 0;
  return Promise.resolve({
    template: template.id,
    params,
    est_annual_cost: 31_337 * level,
    budget_share: 0.01 * level,
    side_effects: [{ kind: "disruption", group: "all", older: false, amount: 0.03 * level, size: level > 3 ? "large" : "small" }],
    support: template.support,
    available: true,
  });
}

describe("TemplateCard policy preview", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("asks the engine for the cost when a slider moves, and shows its answer", async () => {
    const previewPolicy = vi.fn(engineAnswer);
    useGame.setState({ tr: new Translator("en", FTL), busy: false, previewPolicy });
    render(<TemplateCard t={template} />);
    fireEvent.click(screen.getByRole("button", { name: /Transit frequency/ }));
    // At the defaults: the catalogue's estimate, no query.
    expect(screen.getByTestId("policy-cost")).toHaveTextContent(formatMoney(50_000));
    expect(previewPolicy).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText(/Service level/), { target: { value: "4" } });
    expect(screen.getByTestId("policy-estimate")).toHaveTextContent("Updating estimate…");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(PREVIEW_DEBOUNCE_MS + 10);
    });
    expect(previewPolicy).toHaveBeenCalledTimes(1);
    expect(previewPolicy).toHaveBeenLastCalledWith(template.id, [{ id: "level", value: 4 }]);
    expect(screen.getByTestId("policy-cost")).toHaveTextContent(formatMoney(31_337 * 4));
    expect(screen.getByTestId("policy-estimate")).toHaveTextContent("4.0% of the town budget");
    expect(screen.getByTestId("policy-tradeoffs")).toHaveTextContent("large");

    // A second move changes the cost again (debounced: one query per pause).
    fireEvent.change(screen.getByLabelText(/Service level/), { target: { value: "5" } });
    fireEvent.change(screen.getByLabelText(/Service level/), { target: { value: "3" } });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(PREVIEW_DEBOUNCE_MS + 10);
    });
    expect(previewPolicy).toHaveBeenCalledTimes(2);
    expect(screen.getByTestId("policy-cost")).toHaveTextContent(formatMoney(31_337 * 3));
    expect(screen.getByTestId("policy-tradeoffs")).toHaveTextContent("small");
  });
});
