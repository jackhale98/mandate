import { render, screen, within } from "@testing-library/react";
import { englishTranslator } from "../i18n/englishLocale";
import type { SaveMetaDto } from "../engine/protocol/SaveMetaDto";
import type { RunSummary } from "../persistence/saveStore";
import { useGame } from "../state/gameStore";
import { runWho, Title } from "./Title";

const meta = (turn: number): SaveMetaDto => ({
  format_version: 10, run_id: "r", revision: turn, parent_revision: null, device_id: "d", saved_at: "2026-01-01T00:00:00Z", engine_version: "0.1.0", content_hash: "c", protocol_version: 1,
  run_mode: "ironman", seed: "1", date: { day_number: 7, year: 2020, month: 3, day: 4 }, turn,
});
const run = (id: string, turn: number, over: Partial<RunSummary> = {}): RunSummary => ({ runId: id, mode: "ironman", status: "active", revision: turn, meta: meta(turn), updatedAt: `2026-01-0${turn}T00:00:00Z`, name: "Larkowick", ...over });
const zeno = { name: "Zeno Dell", roleKey: "role-private", home: "Larkowick" };

function setup(runs: RunSummary[]) {
  const fillRunNames = vi.fn(() => Promise.resolve());
  useGame.setState({ tr: englishTranslator(), runs, busy: false, scenarios: [], storytellers: [], progress: null, fillRunNames });
  return { fillRunNames };
}

describe("the title screen", () => {
  it("with no saves, the new-career form is all there is, with the draft as the primary action and Begin as the quick start", () => {
    setup([]);
    render(<Title />);
    expect(screen.getByRole("heading", { name: "New career" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Choose an opening" }).className).toContain("btn-primary");
    expect(screen.getByRole("button", { name: "Begin" }).className).not.toContain("btn-primary");
    expect(screen.getByText("No runs yet.")).toBeTruthy();
  });

  it("with saves, the saved runs come first and the new career below; Continue on the latest run is the primary action", () => {
    setup([run("b", 2, { player: zeno }), run("a", 1)]);
    render(<Title />);
    const heads = screen.getAllByRole("heading", { level: 2 }).map((h) => h.textContent);
    expect(heads).toEqual(["Your runs", "New career"]);
    const rows = screen.getAllByTestId("run-row");
    const buttons = rows.map((r) => within(r).getByRole("button", { name: "Continue" }));
    expect(buttons[0]?.className).toContain("btn-primary");
    expect(buttons[1]?.className).not.toContain("btn-primary");
    expect(screen.getByRole("button", { name: "Begin" }).className).not.toContain("btn-primary");
    expect(screen.getByRole("button", { name: "Choose an opening" }).className).not.toContain("btn-primary");
  });

  it("a row says whose run it is, in the pocket header's form; a run without names shows as before and asks for them", () => {
    const { fillRunNames } = setup([run("b", 2, { player: zeno }), run("a", 1)]);
    render(<Title />);
    const [named, old] = screen.getAllByTestId("run-row") as [HTMLElement, HTMLElement];
    expect(within(named).getByTestId("run-who").textContent).toBe(`Zeno Dell · ${englishTranslator().t("role-private")} · Larkowick`);
    expect(named.textContent).toContain("Turn 2");
    expect(within(old).queryByTestId("run-who")).toBeNull();
    expect(old.textContent).toContain("Larkowick · ");
    expect(fillRunNames).toHaveBeenCalledTimes(1);
    expect(runWho(englishTranslator(), { ...zeno, roleKey: "role-private" })).toContain("Zeno Dell · ");
  });
});
