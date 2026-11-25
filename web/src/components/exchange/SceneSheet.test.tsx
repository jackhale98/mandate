/// <reference types="node" />
// The scene's screen (X2) against the engine's own views (fixtures/exchange/*.json,
// written by crates/mandate-wasm/tests/scene_views.rs): on offer, a round and the
// result; staged and as a plain list. Nothing raw on screen; every line's
// accessible name is its label and its tags in words; the keyboard plays it.
import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { ArcSceneView } from "../../engine/protocol/ArcSceneView";
import { englishTranslator } from "../../i18n/englishLocale";
import { useGame } from "../../state/gameStore";
import { SceneSheet } from "./SceneSheet";
import { useScene } from "./sceneStore";
import { arrows } from "./sceneText";

const dir = path.join(path.dirname(fileURLToPath(import.meta.url)), "../fixtures/exchange");
const load = (name: string) => JSON.parse(readFileSync(path.join(dir, `${name}.json`), "utf8")) as ArcSceneView;

function setup(v: ArcSceneView, plain = false) {
  const calls = { begin: vi.fn(() => Promise.resolve()), say: vi.fn(() => Promise.resolve()), leave: vi.fn(() => Promise.resolve()), close: vi.fn() };
  useGame.setState({ tr: englishTranslator(), busy: false });
  useScene.setState({ open: { arc: v.arc, beat: v.beat }, view: v, plain, said: {}, ...calls });
  render(<SceneSheet />);
  return calls;
}

const clean = (el: HTMLElement) => {
  const text = el.textContent ?? "";
  expect(text).not.toContain("⟦");
  expect(text).not.toMatch(/\b(exchange|ui)-[a-z]/);
};

describe("the scene's screen", () => {
  afterEach(() => useScene.setState({ open: null, view: null }));

  it("offers the debate with its day and its place, and turns up from there", () => {
    const v = load("open");
    const c = setup(v);
    const scene = screen.getByTestId("scene");
    expect(scene.dataset.phase).toBe("open");
    clean(scene);
    expect(screen.getByTestId("scene-offer").textContent).toMatch(/debate/);
    fireEvent.click(screen.getByTestId("scene-begin"));
    expect(c.begin).toHaveBeenCalled();
  });

  it("shows a round: the strip, the prompt, every line with whom it lands with and what it risks", () => {
    const v = load("round");
    const c = setup(v);
    const scene = screen.getByTestId("scene");
    clean(scene);
    const chips = within(screen.getByRole("list", { name: "Who is listening" })).getAllByRole("listitem");
    expect(chips).toHaveLength(v.groups.length);
    for (const chip of chips) expect(chip.getAttribute("aria-label")).toMatch(/: /);
    const rows = screen.getAllByTestId("scene-option");
    expect(rows).toHaveLength(v.options.length);
    v.options.forEach((o, i) => {
      const b = within(rows[i] as HTMLElement).getAllByRole("button")[0] as HTMLElement;
      const name = b.getAttribute("aria-label") ?? "";
      // The label, then its tags in words, then its chance and its cost.
      expect(name).toMatch(/\. /);
      if (o.chance !== null) expect(name).toContain(`${o.chance}% chance`);
      for (const t of o.tags) expect((rows[i] as HTMLElement).textContent).toContain(arrows(t));
    });
    // The safe line is there; at most one locked line, folded under its control.
    expect(rows.filter((r) => r.dataset.safe === "true")).toHaveLength(1);
    expect(screen.queryAllByTestId("scene-locked").length).toBeLessThanOrEqual(1);
    // Digits choose; Right arrow opens a line's detail, Left arrow leaves it.
    const list = screen.getByTestId("scene-options");
    const first = within(rows[0] as HTMLElement).getAllByRole("button")[0] as HTMLElement;
    first.focus();
    fireEvent.keyDown(list, { key: "ArrowRight" });
    const detail = screen.getByTestId("scene-detail");
    clean(detail);
    expect(within(detail).getAllByRole("listitem").length).toBeGreaterThanOrEqual(v.groups.length);
    fireEvent.keyDown(detail, { key: "ArrowLeft" });
    expect(screen.queryByTestId("scene-detail")).toBeNull();
    fireEvent.keyDown(screen.getByTestId("scene-options"), { key: "2" });
    expect(c.say).toHaveBeenCalledWith(v.options[1]);
  });

  it("a group the player does not know: '?' on the strip, never a number, and a range on every line", () => {
    const v = load("round-unknown");
    expect(v.groups.every((g) => g.lean === null && g.moved_points === null)).toBe(true);
    setup(v);
    const scene = screen.getByTestId("scene");
    clean(scene);
    for (const chip of screen.getAllByTestId("scene-chip")) {
      expect(chip.dataset.known).toBe("false");
      expect(chip.textContent).toContain("?");
      expect(chip.getAttribute("aria-label")).toContain("Not known how they lean");
      // How it moved is an arrow, never points.
      expect(chip.textContent).not.toMatch(/\d/);
    }
    // Every line with a change for a group it cannot read shows that change as a range.
    const ranged = v.options.filter((o) => o.stands.some((c) => !c.exact));
    expect(ranged.length).toBeGreaterThan(0);
    const rows = screen.getAllByTestId("scene-option");
    const i = v.options.indexOf(ranged[0] as (typeof v.options)[number]);
    fireEvent.click(within(rows[i] as HTMLElement).getByTestId("scene-more"));
    expect(screen.getByTestId("scene-detail").textContent).toMatch(/ to /);
    expect(screen.getByTestId("scene-detail").textContent).toContain("not known exactly");
  });

  it("asks before walking out", () => {
    const v = load("round");
    const c = setup(v);
    act(() => {
      fireEvent.keyDown(window, { key: "Escape" });
    });
    expect(screen.getByTestId("scene-leave")).toBeTruthy();
    fireEvent.click(screen.getByTestId("scene-walk-out"));
    expect(c.leave).toHaveBeenCalled();
  });

  it("shows the result: the outcome, a table of the groups, the paper, Why?", () => {
    const v = load("result");
    const c = setup(v);
    const result = screen.getByTestId("scene-result");
    clean(result);
    expect(within(result).getAllByRole("columnheader")).toHaveLength(4);
    expect(within(result).getAllByRole("row")).toHaveLength(v.groups.length + 1);
    expect(screen.getByTestId("scene-paper").textContent).not.toBe("");
    expect(within(result).getByTestId("why-button").dataset.event).toBe(String(v.result?.event));
    fireEvent.click(screen.getByTestId("scene-close"));
    expect(c.close).toHaveBeenCalled();
  });

  it("in quick mode, is the same choices as a plain list", () => {
    const v = load("round");
    setup(v, true);
    const scene = screen.getByTestId("scene");
    expect(scene.dataset.mode).toBe("plain");
    clean(scene);
    expect(screen.getAllByTestId("scene-option")).toHaveLength(v.options.length);
    expect(screen.getByTestId("scene-prompt").textContent).toContain(v.other?.name ?? "");
    // Tags are written out.
    expect(scene.textContent).toMatch(/Lands|Costs you|Barely moves|Not known how/);
  });
});
