import { render, screen } from "@testing-library/react";
import { ImpactLines } from "./BillCard";
import { useGame } from "../state/gameStore";
import { englishTranslator } from "../i18n/englishLocale";
import type { ImpactLine } from "../engine/protocol/ImpactLine";
import type { ImpactView } from "../engine/protocol/ImpactView";

// What a bill does (DEC-111): the engine's estimate, rendered with the real
// English locale. The engine chooses the lines, the horizon and the rounding;
// the card formats numbers and renders keys, and no raw key or placeholder
// may reach the screen.

function line(over: Partial<ImpactLine>): ImpactLine {
  return {
    outcome: "homes",
    direction: "up",
    horizon_months: 36,
    in_time: true,
    form: "range",
    lo: 110,
    hi: 190,
    unit: "count",
    key: "impact-homes-up",
    args: { group: "all" },
    lead: true,
    tradeoff: false,
    ...over,
  };
}

function impact(over: Partial<ImpactView>): ImpactView {
  return {
    at: { kind: "municipality", tier: "local", id: 3, name: "Iverland", name_key: "jurisdiction-municipality" },
    cost: { yearly: 12_600_000, budget_pct: 9.3, paid_from: "squeeze", funded_first_pct: 24, funded_later_pct: 88, borrowed_pct: 0, key: "impact-cost", paid_key: "impact-paid-squeeze" },
    lines: [
      line({ outcome: "target", form: "exact", lo: 580, hi: null, key: "impact-target-homes-started", args: { direction: "up" } }),
      line({}),
      line({ outcome: "jobs", key: "impact-jobs-down", direction: "down", form: "about", lo: 40, hi: null, tradeoff: true }),
      line({ outcome: "rent", key: "impact-rent-flat", direction: "flat", form: "words", lo: null, hi: null, unit: "percent" }),
      // Not on the card: another horizon, for the full sheet.
      line({ horizon_months: 60, lo: 200, hi: 320, lead: false }),
    ],
    groups: [
      { group: "low", gains: true, key: "impact-gains-low" },
      { group: "older", gains: false, key: "impact-loses-older" },
      { group: "area", gains: false, key: "impact-loses-area" },
    ],
    when: [
      { key: "impact-when-first-year", args: { funded: "24" } },
      { key: "impact-when-building", args: { n: "2" } },
      { key: "impact-when-full", args: { n: "18" } },
    ],
    horizon_key: "impact-horizon-y3",
    horizon_months: 36,
    in_time: true,
    sureness: "fair",
    source: { key: "impact-source-statistician", args: {} },
    ...over,
  };
}

describe("ImpactLines", () => {
  beforeEach(() => {
    useGame.setState({ tr: englishTranslator() });
  });

  it("says what the bill does, what it costs, who gains and loses, when and how sure", () => {
    render(<ImpactLines impact={impact({})} />);
    const card = screen.getByTestId("impact");
    expect(card).toHaveTextContent("What it does");
    expect(screen.getByTestId("impact-horizon")).toHaveTextContent("After three years · in time for the election");
    const lines = screen.getByTestId("impact-lines").querySelectorAll("li");
    expect([...lines].map((l) => l.textContent)).toEqual([
      "580 homes started.",
      "Between 110 and 190 more homes than there would otherwise be.",
      "About 40 fewer jobs overall.",
      "No clear change in rents.",
    ]);
    expect(screen.getByTestId("impact-cost")).toHaveTextContent(
      "Costs 12.6M a year, 9.3% of the budget. More than the budget can carry: it and every other programme would be funded at 88%.",
    );
    expect(screen.getByTestId("impact-groups")).toHaveTextContent("People on low incomes gain. Older residents lose. Streets are dug up while it builds.");
    expect(screen.getByTestId("impact-when")).toHaveTextContent(
      "Funded at 24% until the next budget. Work starts within 2 months. In full effect after 18 months. Fair estimate. The statistician's figures.",
    );
    // No raw key, no unfilled placeholder.
    expect(card.textContent).not.toMatch(/⟦|\{\s*\$|impact-/);
  });

  it("names sectors, classes, hazards and categories through their own keys", () => {
    render(
      <ImpactLines
        impact={impact({
          cost: { yearly: 0, budget_pct: 0, paid_from: "levy", funded_first_pct: 100, funded_later_pct: 100, borrowed_pct: 0, key: "impact-cost-none", paid_key: null },
          lines: [
            line({ outcome: "target", form: "exact", lo: 120, hi: null, key: "impact-target-jobs-supported", args: { direction: "up", sector_key: "sector-retail" } }),
            line({ outcome: "sector_jobs", key: "impact-sector-jobs-up", form: "words", lo: null, hi: null, args: { group: "all", sector_key: "sector-retail" } }),
            line({ outcome: "target", form: "exact", lo: 400, hi: null, unit: "money", key: "impact-target-paid", args: { direction: "up", class: "low" } }),
            line({ outcome: "target", form: "exact", lo: 25, hi: null, unit: "percent", key: "impact-target-protection", args: { direction: "up", hazard_key: "impact-hazard-flood" } }),
            line({ outcome: "target", form: "exact", lo: 8, hi: null, unit: "percent", key: "impact-target-service-level", args: { direction: "up", category_key: "impact-category-infrastructure" } }),
            line({ outcome: "target", form: "exact", lo: 0.1, hi: null, unit: "points", key: "impact-target-tax-points", args: { direction: "down" } }),
            line({ outcome: "low_income", key: "impact-low-income-up", form: "range", lo: 3.2, hi: 8, unit: "percent" }),
          ],
          groups: [],
          when: [{ key: "impact-when-at-once", args: {} }],
          in_time: false,
        })}
      />,
    );
    const text = [...screen.getByTestId("impact-lines").querySelectorAll("li")].map((l) => l.textContent);
    expect(text).toEqual([
      "120 jobs supported in Retail & services.",
      "More jobs in Retail & services.",
      "Pays 400 a year to each adult on a low income.",
      "Damage from a flood is cut by 25%.",
      "Service in roads and transit rises 8%.",
      "The property tax falls by 0.1 points of a home's value.",
      "Incomes on low pay end 3.2% to 8% higher.",
    ]);
    expect(screen.getByTestId("impact-horizon")).toHaveTextContent(/^After three years$/);
    expect(screen.getByTestId("impact-cost")).toHaveTextContent(/^Costs nothing to run\.$/);
    expect(screen.queryByTestId("impact-groups")).toBeNull();
    expect(screen.getByTestId("impact-when")).toHaveTextContent("In force from the day it passes. Fair estimate. The statistician's figures.");
    expect(screen.getByTestId("impact").textContent).not.toMatch(/⟦|\{\s*\$|impact-/);
  });

  it("shows nothing for a bill the engine gave no estimate (a law in effect, a tier the table does not cover)", () => {
    const { container } = render(<ImpactLines impact={undefined} />);
    expect(container).toBeEmptyDOMElement();
  });
});
