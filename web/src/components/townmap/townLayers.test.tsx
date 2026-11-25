import { fireEvent, render, screen, within } from "@testing-library/react";
import type { TownMapLayerView } from "../../engine/protocol/TownMapLayerView";
import { englishTranslator } from "../../i18n/englishLocale";
import { useGame } from "../../state/gameStore";
import { TownLayerList, byPlace } from "./TownLayerList";
import { LayerLegend, PersonCard, whereText } from "./TownLayerPanel";
import { layersView as view } from "./fixture";
import { chosenOption } from "./layerModel";

const tr = englishTranslator();
const layer = (source: string): TownMapLayerView => {
  const l = view.layers.find((x) => x.source === source);
  if (!l) throw new Error(source);
  return l;
};
const worries = layer("worries");
const options = worries.data.kind === "shaded" ? worries.data.options : [];
const noop = () => {};
// The constitution's words, from its Fluent keys (DEC-098: no office or body is named here).
const member = tr.t("role-councillor");
const chamber = tr.t("place-council_chamber");

/** Nothing the engine named is shown raw: no missing-key marker, no content id, no Fluent key, no placeholder. */
function expectWords(el: HTMLElement) {
  const text = el.textContent ?? "";
  expect(text).not.toMatch(/⟦|town-map-|map-layer-|concern[.-]|ui-|\{ ?\$|town_map_layer/);
}

beforeEach(() => {
  useGame.setState({ tr });
});

describe("town map layers as lists (DEC-051)", () => {
  it("lists every district with its neighbourhoods, its member and the player's own", () => {
    const { container } = render(<TownLayerList tr={tr} view={view} layer={layer("districts")} option={null} filter="all" onPerson={noop} />);
    const areas = screen.getAllByTestId("town-map-list-area");
    expect(areas).toHaveLength(2);
    const first = areas[0] as HTMLElement;
    expect(within(first).getByRole("heading")).toHaveTextContent("Ward 1");
    expect(first).toHaveTextContent("Your seat");
    expect(first).toHaveTextContent("Covers Ashford town centre, Eastfield");
    expect(within(first).getByTestId("town-map-row-person")).toHaveTextContent("You");
    // A member the player has not heard of: the role, the district and the party are public; the name is not.
    const second = areas[1] as HTMLElement;
    expect(second).toHaveTextContent("Ward 2");
    expect(second).not.toHaveTextContent("Your seat");
    expect(within(second).getByTestId("town-map-row-person")).toHaveTextContent("Someone you haven't met");
    expect(second).toHaveTextContent(`${member} · Ward 2 · Labour Alliance`);
    expectWords(container);
  });

  it("lists people by where the player knows to find them, as the filter keeps them", () => {
    const people = layer("people");
    const onPerson = vi.fn();
    const { container, rerender } = render(<TownLayerList tr={tr} view={view} layer={people} option={null} filter="all" onPerson={onPerson} />);
    const groups = screen.getAllByTestId("town-map-list-place");
    expect(groups.map((g) => within(g).getByRole("heading").textContent)).toEqual([`${chamber} (2)`, "Union hall (1)", "Streets of Riverside (1)", "Whereabouts not known (1)"]);
    expect(groups[2]).toHaveTextContent("Last seen at Riverside, in the morning.");
    fireEvent.click(within(groups[0] as HTMLElement).getByRole("button", { name: "Ines Okafor" }));
    expect(onPerson).toHaveBeenCalledWith(expect.objectContaining({ person: expect.objectContaining({ id: 3 }) }));
    expectWords(container);
    // The body's members only.
    rerender(<TownLayerList tr={tr} view={view} layer={people} option={null} filter="body" onPerson={onPerson} />);
    expect(screen.getAllByTestId("town-map-row-person").map((b) => b.textContent)).toEqual(["Ines Okafor", "Tomas Brandt"]);
    rerender(<TownLayerList tr={tr} view={view} layer={people} option={null} filter="party:9" onPerson={onPerson} />);
    expect(screen.getByText("Nothing to show here yet.")).toBeInTheDocument();
    if (people.data.kind === "markers") expect(byPlace(tr, people.data.markers).map((g) => g.people.length)).toEqual([2, 1, 1, 1]);
  });

  it("lists each neighbourhood's worry, and says so where it is not known", () => {
    const { container, rerender } = render(<TownLayerList tr={tr} view={view} layer={worries} option={chosenOption(options, "top")} filter="all" onPerson={noop} />);
    const hoods = screen.getAllByTestId("town-map-list-hood");
    expect(hoods).toHaveLength(3);
    expect(hoods[0]).toHaveTextContent("Ashford town centre");
    expect(hoods[0]).toHaveTextContent("Top worry: Jobs");
    expect(hoods[0]).toHaveTextContent("Worries you know of: Jobs, Housing costs, Taxes");
    // Not known: said in words, with the engine's placeholder, and never as a class.
    expect(hoods[2]).toHaveAttribute("data-known", "false");
    expect(hoods[2]).toHaveTextContent("Not known yet");
    expect(within(hoods[2] as HTMLElement).getByTestId("knowledge-placeholder")).toBeInTheDocument();
    expectWords(container);
    rerender(<TownLayerList tr={tr} view={view} layer={worries} option={chosenOption(options, "concern.jobs")} filter="all" onPerson={noop} />);
    const again = screen.getAllByTestId("town-map-list-hood");
    expect(again[0]).toHaveTextContent("Jobs: Top worry");
    expect(again[1]).toHaveTextContent("Jobs: In the top three");
    expect(again[2]).toHaveTextContent("Not known yet");
    expect(again[2]).not.toHaveTextContent("lesser");
    expectWords(container);
  });
});

describe("town map layer legend and cards", () => {
  it("names every class of a worry, the most first, and unknown apart from them", () => {
    const { container } = render(<LayerLegend tr={tr} view={view} layer={worries} option={chosenOption(options, "concern.jobs")} hood={12} area={null} unplaced={0} onUnplaced={noop} onPerson={noop} />);
    const legend = screen.getByTestId("town-map-legend");
    expect(legend).toHaveAccessibleName("Legend: Worries");
    const items = within(legend).getAllByRole("listitem");
    expect(items.map((li) => li.textContent?.trim())).toEqual(["Top worry (1)", "In the top three (1)", "A lesser worry (0)", "Not known yet"]);
    // The neighbourhood being read is unknown: the legend marks the unknown entry, not a class.
    expect(screen.getByTestId("town-map-legend-unknown")).toHaveClass("on");
    expect(screen.getByTestId("town-map-readout")).toHaveTextContent("Riverside");
    expect(screen.getByTestId("town-map-readout-value")).toHaveTextContent("Not known yet");
    expectWords(container);
  });

  it("reads a district: its neighbourhoods and member; parties are named, never colour alone", () => {
    const districts = layer("districts");
    const area = districts.data.kind === "boundaries" ? (districts.data.areas[1] ?? null) : null;
    const onPerson = vi.fn();
    const { container } = render(<LayerLegend tr={tr} view={view} layer={districts} option={null} hood={null} area={area} unplaced={0} onUnplaced={noop} onPerson={onPerson} />);
    expect(screen.getByTestId("town-map-legend")).toHaveAccessibleName("Legend: Wards");
    const read = screen.getByTestId("town-map-readout");
    expect(read).toHaveTextContent("Ward 2");
    expect(read).toHaveTextContent("Covers Riverside");
    fireEvent.click(screen.getByTestId("town-map-readout-member"));
    expect(onPerson).toHaveBeenCalledTimes(1);
    expect(within(screen.getByTestId("town-map-legend")).getAllByRole("listitem").map((li) => li.textContent?.trim())).toEqual(["Civic Union (1)", "Labour Alliance (1)"]);
    expectWords(container);
  });

  it("offers the people with no known place from the legend", () => {
    const onUnplaced = vi.fn();
    render(<LayerLegend tr={tr} view={view} layer={layer("people")} option={null} hood={null} area={null} unplaced={1} onUnplaced={onUnplaced} onPerson={noop} />);
    expect(screen.getByTestId("town-map-legend")).toHaveTextContent("No party (3)");
    fireEvent.click(screen.getByRole("button", { name: "Whereabouts not known (1)" }));
    expect(onUnplaced).toHaveBeenCalledTimes(1);
  });

  it("shows a person's card with where the player knows to find them, their card and the way to them", () => {
    const people = layer("people");
    const ms = people.data.kind === "markers" ? people.data.markers : [];
    const [ines, , , sam, lena] = ms;
    if (!ines || !sam || !lena) throw new Error("fixture");
    const onGo = vi.fn();
    const onOpenCard = vi.fn();
    const { container, rerender } = render(<PersonCard tr={tr} m={ines} view={view} busy={false} onClose={noop} onOpenCard={onOpenCard} onGo={onGo} />);
    const card = screen.getByTestId("town-map-person-card");
    expect(within(card).getByRole("heading")).toHaveTextContent("Ines Okafor");
    expect(card).toHaveTextContent(`${member} · Ward 3 · Civic Union`);
    expect(screen.getByTestId("town-map-person-where")).toHaveTextContent(`Found at ${chamber}`);
    fireEvent.click(screen.getByRole("button", { name: "Go to Ines Okafor" }));
    expect(onGo).toHaveBeenCalledWith(ines);
    fireEvent.click(screen.getByRole("button", { name: "Open their card" }));
    expect(onOpenCard).toHaveBeenCalledWith(ines);
    expectWords(container);
    // Seen, not known to work anywhere: the sighting. No known place: said so.
    expect(whereText(tr, sam)).toBe("Last seen at Riverside, in the morning.");
    rerender(<PersonCard tr={tr} m={lena} view={view} busy={false} onClose={noop} onOpenCard={onOpenCard} onGo={onGo} />);
    expect(screen.getByTestId("town-map-person-where")).toHaveTextContent("You do not know where to find them yet.");
    expect(screen.getByTestId("town-map-person-card")).toHaveTextContent("Heard of");
    // The player's own card has no way to themselves.
    const districts = layer("districts");
    const you = districts.data.kind === "boundaries" ? districts.data.areas[0]?.holder : null;
    if (!you) throw new Error("fixture");
    rerender(<PersonCard tr={tr} m={you} view={view} busy={false} onClose={noop} onOpenCard={onOpenCard} onGo={onGo} />);
    expect(screen.getByTestId("town-map-person-card")).toHaveTextContent("You");
    expect(screen.queryByTestId("town-map-person-go")).toBeNull();
    expect(screen.queryByTestId("town-map-person-where")).toBeNull();
  });
});
