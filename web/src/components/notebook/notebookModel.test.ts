/// <reference types="node" />
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { NotebookView } from "../../engine/protocol/NotebookView";
import { englishTranslator } from "../../i18n/englishLocale";
import { notebookFixture } from "./fixture";
import { freshKeys, groupNotebook, itemFacts, itemText, itemTitle, matches, newCounts, personItems, subjectKey, tabOf } from "./notebookModel";

const tr = englishTranslator();
const here = path.dirname(fileURLToPath(import.meta.url));
/** The engine's notebook for the pocket's fixtures (crates/mandate-wasm/tests/pocket_views.rs). */
const engineNotebook = () => JSON.parse(readFileSync(path.join(here, "../fixtures/pocket/notebook.json"), "utf8")) as NotebookView;

describe("the notebook's arrangement (DEC-113)", () => {
  it("files every subject under one of the four tabs", () => {
    expect(tabOf({ type: "person", person: 1 })).toBe("people");
    expect(tabOf({ type: "broker", town: 0 })).toBe("town");
    expect(tabOf({ type: "rival_plans", town: 0 })).toBe("town");
    expect(tabOf({ type: "secret", hidden: 2 })).toBe("town");
    expect(tabOf({ type: "town_concerns", town: 0 })).toBe("town");
    expect(tabOf({ type: "hood_concerns", hood: 0 })).toBe("places");
    expect(tabOf({ type: "place", place: 0 })).toBe("places");
    expect(tabOf({ type: "policy_support", town: 0, policy: { type: "template", template: "x" } })).toBe("policies");
    expect(tabOf({ type: "situational", town: 0, template: "x" })).toBe("policies");
  });

  it("lists only what is known: the engine's gaps are kept for search, never as entries (knowledge is not a checklist)", () => {
    const v = engineNotebook();
    const g = groupNotebook(v);
    const listed = [...g.people, ...g.places, ...g.town, ...g.policies];
    expect(listed.every((i) => i.entry !== null && i.level !== "unknown")).toBe(true);
    expect(listed).toHaveLength(v.sections.reduce((n, s) => n + s.entries.length, 0));
    expect(g.gaps).toEqual(v.gaps);
    expect(g.gaps.length).toBeGreaterThan(0);
    // The town's worries under Town; a neighbourhood's under Places, by name.
    expect(g.town[0]?.subject.type).toBe("town_concerns");
    const hoods = v.sections.flatMap((s) => s.entries).filter((e) => e.subject.type === "hood_concerns").length;
    expect(hoods).toBeGreaterThan(0);
    expect(g.places.filter((i) => i.subject.type === "hood_concerns").length).toBe(hoods);
  });

  it("marks what the engine says is new, and keeps the marks it is given while the player looks", () => {
    const v = notebookFixture();
    const westfield = subjectKey({ type: "hood_concerns", hood: 2 });
    expect(freshKeys(v)).toEqual([westfield]);
    for (const x of v.sections) for (const e of x.entries) e.new_this_turn = false;
    const g = groupNotebook(v, new Set([westfield]));
    expect(g.places.filter((i) => i.isNew).map((i) => i.args.hood)).toEqual(["Westfield"]);
    expect(newCounts(g, 3)).toEqual({ people: 3, places: 1, town: 0, policies: 0 });
  });

  it("words what is known, and a search finds it by any of its words", () => {
    const g = groupNotebook(notebookFixture());
    const vera = personItems(g.people).get(12);
    expect(vera).toBeDefined();
    if (!vera) return;
    expect(itemTitle(tr, vera)).toBe("Vera Holt");
    expect(itemFacts(tr, vera).join(" ")).not.toMatch(/●|○|of \d+ known|%/);
    expect(matches(itemText(tr, vera), "vera")).toBe(true);
    expect(matches(itemText(tr, vera), "nobody-here")).toBe(false);
    expect(matches("anything", "  ")).toBe(true);
  });
});
