import { Translator } from "./fluent";

describe("Translator", () => {
  const tr = new Translator("en", "hello = Hi { $name }\nturns = { $n ->\n [one] one turn\n *[other] { $n } turns\n}\n");

  it("formats arguments and plurals", () => {
    expect(tr.t("hello", { name: "Ada" })).toBe("Hi Ada");
    expect(tr.t("turns", { n: 1 })).toBe("one turn");
    expect(tr.t("turns", { n: 3 })).toBe("3 turns");
  });

  it("selects plurals for engine counts, which arrive as strings", () => {
    expect(tr.event("turns", { n: "1" })).toBe("one turn");
    expect(tr.event("turns", { n: "4" })).toBe("4 turns");
  });

  it("makes missing keys visible", () => {
    expect(tr.t("nope")).toBe("⟦nope⟧");
    expect(tr.has("hello")).toBe(true);
  });
});
