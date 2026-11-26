/// <reference types="node" />
// Faults on a neighbourhood's streets (DEC-114, task B2) render the engine's
// view (fixtures/pocket/street.json: a real game's street, written by
// crates/mandate-wasm/tests/pocket_views.rs) in Fluent, with "Why?" on each
// and "See it" moving the player to the streets in the protocol's shape.
import { fireEvent, render, screen, within } from "@testing-library/react";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { PlacePanelView } from "../../engine/protocol/PlacePanelView";
import { englishTranslator } from "../../i18n/englishLocale";
import { useGame } from "../../state/gameStore";
import { FaultLine, FaultsHere } from "./Faults";

const street = JSON.parse(readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), "../fixtures/pocket/street.json"), "utf8")) as PlacePanelView;
const faults = street.faults ?? [];

describe("faults", () => {
  const visit = vi.fn(async () => {});
  beforeEach(() => {
    visit.mockClear();
    useGame.setState({ tr: englishTranslator(), busy: false, visit });
  });

  it("the street lists each fault in words, with its standing in the queue and Why", () => {
    expect(faults.length).toBeGreaterThan(1);
    render(<FaultsHere faults={faults} />);
    const section = screen.getByTestId("faults-here");
    expect(section.textContent).not.toContain("⟦");
    const tr = englishTranslator();
    for (const f of faults) {
      const row = within(section).getByTestId(`fault-${f.id}`);
      expect(row.textContent).toContain(tr.event(f.what_key, f.args));
      expect(row.getAttribute("data-cue")).toBe(f.cue);
      expect(row.getAttribute("data-spot")).toBe(`${f.spot.kind}:${f.spot.slot}`);
      expect(within(row).getByTestId("fault-status").textContent).toBe(tr.t(`ui-fault-status-${f.status}`));
      if (f.event !== undefined) expect(within(row).getByTestId("why-button")).toBeTruthy();
    }
  });

  it("See it walks to the streets the fault is on", () => {
    const f = faults[0];
    if (!f) throw new Error("no fault in the fixture");
    render(<FaultLine f={{ ...f, here: false }} />);
    fireEvent.click(screen.getByTestId(`fault-see-${f.id}`));
    expect(visit).toHaveBeenCalledWith(f.see.location);
  });
});
