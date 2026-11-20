import { render, screen, within } from "@testing-library/react";
import { ExplainTree } from "./ExplainTree";
import type { DevNodeView } from "./protocol/DevNodeView";

const leaf = (label: string, ok: boolean, kind = "condition", values: [string, string][] = []): DevNodeView => ({
  kind,
  label,
  detail: kind === "condition" ? `${label}=x` : "",
  ok,
  values: values.map(([name, value]) => ({ name, value })),
  children: [],
});

const tree: DevNodeView = {
  kind: "all",
  label: "dilemma.plant_layoffs",
  detail: "",
  ok: false,
  values: [],
  children: [
    { ...leaf("gates", true, "all"), children: [leaf("cooldown", true, "gate")] },
    {
      ...leaf("triggers", false, "all"),
      children: [leaf("crisis", false, "condition", [["severity", "no such crisis"]]), leaf("track", true)],
    },
    leaf("pacing", true, "info"),
  ],
};

describe("ExplainTree", () => {
  it("marks every node with the engine's verdict in words as well as glyphs", () => {
    render(<ExplainTree tree={tree} />);
    const root = screen.getByRole("list", { name: "explanation" });
    expect(within(root).getAllByLabelText("holds").length).toBe(3);
    expect(within(root).getAllByLabelText("fails").length).toBe(3);
    expect(within(root).getAllByLabelText("note").length).toBe(1);
  });

  it("shows the values the evaluator read and the condition's parameters", () => {
    render(<ExplainTree tree={tree} />);
    expect(screen.getByText("no such crisis")).toBeInTheDocument();
    expect(screen.getByText("crisis=x")).toBeInTheDocument();
  });

  it("opens failing branches so the reason is on screen", () => {
    const { container } = render(<ExplainTree tree={tree} />);
    const failing = container.querySelector('li[data-ok="false"] > details');
    expect(failing).not.toBeNull();
    expect((failing as HTMLDetailsElement).open).toBe(true);
  });
});
