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

  it("renders key arguments first, with the plain arguments (an outlet's name)", () => {
    const t2 = new Translator("en", "outlet = { $outlet_place } Regional News\nstory = { $outlet } has it: { $fact }\nfact = a bribe\n");
    expect(t2.event("story", { outlet_key: "outlet", outlet_place: "Iverland", fact_key: "fact" })).toBe("Iverland Regional News has it: a bribe");
  });

  it("renders a key argument that names keys of its own (a retold headline)", () => {
    const t3 = new Translator(
      "en",
      "heard = Did you see it? '{ $headline }.'\nhl = { $town } passes the { $policy } bill\npol = Local wage floor\n",
    );
    expect(t3.event("heard", { headline_key: "hl", policy_key: "pol", town: "Iverland" })).toBe(
      "Did you see it? 'Iverland passes the Local wage floor bill.'",
    );
  });

  it("makes missing keys visible", () => {
    expect(tr.t("nope")).toBe("⟦nope⟧");
    expect(tr.has("hello")).toBe(true);
  });
});
