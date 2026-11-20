import { Translator } from "../../i18n/fluent";
import { countItems, markdown, renderAll, type RawItem } from "./corpusRender";

const tr = new Translator(
  "en",
  [
    "title-x = the bridge over { $hood }",
    "line-x = { $speaker } says { $concern } worries { $hood }.",
    "concern-rent = the rent",
    "scene-quote-backed-my-bill = you backed my bill in { $year }",
    "beat-x = { $memory }, { $player }.",
  ].join("\n"),
);

const sit = { seed: 1, date: "2031-02-03", town: "Fenoport", town_class: "town", strategy: "mixed", storyteller: "steady" };
const raw: RawItem[] = [
  { kind: "dilemma", part: "title", id: "dilemma.x", voice: null, render: "title", view: { key: "title-x", args: { hood: "Old Quay" } }, situation: sit },
  { kind: "talk", part: "substance", id: "line.x", voice: "voice.plain", render: "event", view: { key: "line-x", args: { speaker: "Ada Moss", concern_key: "concern-rent", hood: "Old Quay" } }, situation: sit },
  // The same rendering again (from another career): kept once.
  { kind: "talk", part: "substance", id: "line.x", voice: "voice.plain", render: "event", view: { key: "line-x", args: { speaker: "Ada Moss", concern_key: "concern-rent", hood: "Old Quay" } }, situation: sit },
  // A different fill: kept.
  { kind: "talk", part: "substance", id: "line.x", voice: "voice.plain", render: "event", view: { key: "line-x", args: { speaker: "Ben Ray", concern_key: "concern-rent" } }, situation: sit },
  { kind: "scene", part: "beat", id: "scene.x", voice: null, render: "beat", view: { key: "beat-x", args: { player: "Cleo" }, quote: { key: "memory-backed-my-bill", subject: null, year: 2030 } }, situation: sit },
];

describe("corpus rendering", () => {
  const items = renderAll(tr, raw);

  it("renders as the game does: titles capitalised, `_key` arguments translated, memories quoted to your face", () => {
    expect(items[0]?.text).toBe("The bridge over Old Quay");
    expect(items[1]?.text).toBe("Ada Moss says the rent worries Old Quay.");
    expect(items.find((i) => i.kind === "scene")?.text).toBe("you backed my bill in 2030, Cleo.");
  });

  it("keeps different fills, drops identical renderings, and flags what did not fill", () => {
    expect(items.filter((i) => i.id === "line.x").length).toBe(2);
    const unfilled = items.find((i) => i.args.speaker === "Ben Ray");
    expect(unfilled?.flags).toContain("unfilled-argument");
    const c = countItems(items);
    expect(c.by_kind.talk).toEqual({ items: 2, ids: 1, flagged: 1 });
    expect(markdown(items, c)).toContain("## talk");
  });
});
