import { fireEvent, render, screen, within } from "@testing-library/react";
import type { MapLayerView } from "../engine/protocol/MapLayerView";
import type { MapLayersView } from "../engine/protocol/MapLayersView";
import { englishTranslator } from "../i18n/englishLocale";
import { classColours } from "./layerPalette";
import { LayerTable } from "./LayerTable";
import { binLabel, formatValue, layerColours, layerPaint, layerRows, townValue } from "./layers";
import { MapLegend } from "./MapLegend";

const tr = englishTranslator();

const towns: MapLayersView["towns"] = [
  { id: 0, population: 1_501_475, density: 889.8, unemployment: 0.037, median_income: 123_371, rent: 2661, main_industry: { id: "sector.knowledge", label_key: "sector-knowledge", share: 0.35 }, main_resource: null },
  { id: 1, population: 48_200, density: 12.4, unemployment: 0.094, median_income: 74_000, rent: 880, main_industry: { id: "sector.tourism", label_key: "sector-tourism", share: 0.2 }, main_resource: null },
  { id: 2, population: 61_000, density: 30.1, unemployment: 0.052, median_income: 77_500, rent: 1050, main_industry: { id: "sector.manufacturing", label_key: "sector-manufacturing", share: 0.3 }, main_resource: null },
];

const unemployment: MapLayerView = {
  id: "map_layer.unemployment",
  label_key: "map-layer-unemployment",
  unit_key: "map-layer-unemployment-unit",
  note_key: "map-layer-unemployment-note",
  detail_key: null,
  source: "unemployment",
  scale: "diverging",
  format: "percent",
  midpoint: 0.0536,
  bins: [
    { lo: null, hi: 0.015, category: null, label_key: null, count: 0 },
    { lo: 0.015, hi: 0.04, category: null, label_key: null, count: 1 },
    { lo: 0.04, hi: 0.065, category: null, label_key: null, count: 1 },
    { lo: 0.065, hi: 0.09, category: null, label_key: null, count: 0 },
    { lo: 0.09, hi: null, category: null, label_key: null, count: 1 },
  ],
  classes: [1, 4, 2],
  as_of: null,
};

const industry: MapLayerView = {
  ...unemployment,
  id: "map_layer.industry",
  label_key: "map-layer-industry",
  unit_key: "map-layer-industry-unit",
  note_key: "map-layer-industry-note",
  detail_key: "map-layer-industry-detail",
  source: "main_industry",
  scale: "categorical",
  format: "category",
  midpoint: null,
  bins: [
    { lo: null, hi: null, category: "sector.manufacturing", label_key: "sector-manufacturing", count: 1 },
    { lo: null, hi: null, category: "sector.knowledge", label_key: "sector-knowledge", count: 1 },
    { lo: null, hi: null, category: null, label_key: null, count: 1 },
  ],
  classes: [1, 2, 0],
};

const view: MapLayersView = { date: { day_number: 1, year: 2026, month: 1, day: 1 }, towns, layers: [unemployment, industry] };
const names = new Map([
  [0, "Ashford"],
  [1, "Brill"],
  [2, "Caldy"],
]);

describe("map layer legend (DEC-089)", () => {
  it("names a diverging layer's classes by the engine's breaks, with units and the national midpoint", () => {
    const colours = layerColours(unemployment, "light");
    render(<MapLegend tr={tr} layer={unemployment} colours={colours} town={null} />);
    const legend = screen.getByTestId("map-legend");
    expect(legend).toHaveTextContent("Unemployment");
    expect(legend).toHaveTextContent("Share of workers without a job");
    // The breaks between the classes, compact.
    expect([...legend.querySelectorAll(".map-scale-edges span")].map((e) => e.textContent)).toEqual(["1.5%", "4%", "6.5%", "9%"]);
    // Every class named in text with its count, for screen readers.
    const items = within(legend).getAllByRole("listitem").map((li) => li.textContent);
    expect(items).toEqual(["under 1.5%: 0 towns", "1.5%–4.0%: 1 town", "4.0%–6.5%: 1 town", "6.5%–9.0%: 0 towns", "9.0% and over: 1 town"]);
    expect(legend).toHaveTextContent("national rate of 5.4%");
    // Two hues with a grey middle class.
    const segs = [...legend.querySelectorAll<HTMLElement>(".map-scale-seg")];
    expect(segs).toHaveLength(5);
    expect(segs[2]?.style.background).toBe("rgb(240, 239, 236)");
    expect(screen.getByTestId("map-readout")).toHaveTextContent("Tap a town");
  });

  it("reads a town's figure and marks its class by outline, and opens its details", () => {
    const rows = layerRows(view, unemployment, names);
    const brill = rows[1];
    if (!brill) throw new Error("no row");
    const onDetails = vi.fn();
    const colours = layerColours(unemployment, "dark");
    render(<MapLegend tr={tr} layer={unemployment} colours={colours} town={{ ...brill, home: true }} onDetails={onDetails} />);
    const readout = screen.getByTestId("map-readout");
    expect(readout).toHaveTextContent("Brill");
    expect(readout).toHaveTextContent("You");
    expect(screen.getByTestId("map-readout-value")).toHaveTextContent("9.4%");
    expect(readout).toHaveTextContent("Class 5 of 5");
    const on = screen.getByTestId("map-legend").querySelector(".map-scale-seg.on");
    expect(on).not.toBeNull();
    expect([...(on?.parentElement?.children ?? [])].indexOf(on as Element)).toBe(4);
    fireEvent.click(screen.getByTestId("map-readout-details"));
    expect(onDetails).toHaveBeenCalledWith(1);
  });

  it("lists a categorical layer's categories in a fixed colour order, with Other last", () => {
    const colours = layerColours(industry, "light");
    expect(colours).toEqual(["#2a78d6", "#eb6834", "#c3c2b7"]);
    const caldy = layerRows(view, industry, names)[2];
    render(<MapLegend tr={tr} layer={industry} colours={colours} town={caldy ? { ...caldy, home: false } : null} />);
    const legend = screen.getByTestId("map-legend");
    const items = within(legend).getAllByRole("listitem");
    expect(items.map((li) => li.textContent?.replace(/\s+/g, " ").trim())).toEqual([
      "Manufacturing (1) 1 town",
      "Knowledge & tech (1) 1 town",
      "Other (1) 1 town",
    ]);
    expect(items[0]).toHaveClass("on");
    expect(screen.getByTestId("map-readout")).toHaveTextContent("Manufacturing 30% of local jobs");
  });

  it("never cycles colours: a categorical scale stops at three named classes", () => {
    expect(classColours("categorical", 4, "dark", true)).toEqual(["#3987e5", "#d95926", "#199e70", "#52514e"]);
    expect(classColours("sequential", 5, "light")).toHaveLength(5);
    expect(new Set(classColours("sequential", 7, "dark")).size).toBe(7);
  });

  it("paints each town by the engine's class and leaves unknown towns out", () => {
    const paint = layerPaint({ ...view, layers: [unemployment] }, { ...unemployment, classes: [1, null, 2] }, "light");
    expect(paint.get(0)).toEqual([134, 182, 239]);
    expect(paint.has(1)).toBe(false);
    expect(townValue(towns[0], "nope")).toEqual({ kind: "unknown" });
    expect(formatValue(tr, unemployment, { kind: "unknown" }).text).toBe("Unknown");
    expect(binLabel(tr, industry, industry.bins[2] as MapLayerView["bins"][number])).toBe("Other");
  });

  it("the list sorts by figure or by town, and opens a town", () => {
    const onOpen = vi.fn();
    const colours = layerColours(unemployment, "light");
    render(<LayerTable tr={tr} layer={unemployment} rows={layerRows(view, unemployment, names)} colours={colours} home={0} focus={null} onOpen={onOpen} />);
    const towns = () => screen.getAllByRole("row").slice(1).map((r) => within(r).getByRole("rowheader").textContent);
    expect(towns()).toEqual(["Brill", "Caldy", "AshfordYou"]);
    fireEvent.click(screen.getByTestId("map-table-sort-value"));
    expect(towns()).toEqual(["AshfordYou", "Caldy", "Brill"]);
    fireEvent.click(screen.getByTestId("map-table-sort-town"));
    expect(towns()).toEqual(["AshfordYou", "Brill", "Caldy"]);
    expect(screen.getByRole("columnheader", { name: /Town/ })).toHaveAttribute("aria-sort", "ascending");
    fireEvent.click(screen.getByRole("button", { name: "Open Caldy" }));
    expect(onOpen).toHaveBeenCalledWith(2);
  });
});
