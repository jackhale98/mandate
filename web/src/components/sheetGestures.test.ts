import { registerSheet, topSheet } from "./sheetGestures";

const sheet = (z?: string) => {
  const el = document.createElement("aside");
  if (z) el.style.zIndex = z;
  document.body.append(el);
  return el;
};
const escape = () => document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", cancelable: true }));

describe("sheet Escape", () => {
  afterEach(() => document.body.replaceChildren());

  it("closes only the sheet on top: the highest layer, then the latest opened", () => {
    const closed: string[] = [];
    const offA = registerSheet(sheet("6"), () => closed.push("pocket"));
    const offB = registerSheet(sheet("4"), () => closed.push("town"));
    const offC = registerSheet(sheet("6"), () => closed.push("decision"));
    expect(topSheet()?.order).toBeGreaterThan(0);
    escape();
    expect(closed).toEqual(["decision"]);
    offC();
    escape();
    expect(closed).toEqual(["decision", "pocket"]);
    offA();
    escape();
    expect(closed).toEqual(["decision", "pocket", "town"]);
    offB();
    escape();
    expect(closed).toHaveLength(3);
    expect(topSheet()).toBeUndefined();
  });

  it("leaves Escape to an open modal dialog and to handlers that took it", () => {
    const closed: string[] = [];
    const off = registerSheet(sheet(), () => closed.push("sheet"));
    const modal = document.createElement("div");
    modal.setAttribute("aria-modal", "true");
    document.body.append(modal);
    escape();
    expect(closed).toEqual([]);
    modal.remove();
    const taken = new KeyboardEvent("keydown", { key: "Escape", cancelable: true });
    taken.preventDefault();
    document.dispatchEvent(taken);
    expect(closed).toEqual([]);
    escape();
    expect(closed).toEqual(["sheet"]);
    off();
  });
});
