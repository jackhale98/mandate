import { englishTranslator } from "../../i18n/englishLocale";
import { canvassLead, notebookFixture, officeFixture, talkLead, townCharacterFixture } from "./fixture";
import { firstNewTab, freshKeys, groupNotebook, itemFacts, itemText, itemTitle, keyFigures, levelDots, matches, newCounts, personItems, subjectKey, tabOf } from "./notebookModel";

const tr = englishTranslator();

/** The i-th element, which the test expects to be there. */
function at<T>(a: readonly T[], i: number): T {
  const x = a[i];
  if (x === undefined) throw new Error(`nothing at ${i}`);
  return x;
}

describe("the notebook's arrangement", () => {
  it("files every subject under one tab", () => {
    expect(tabOf({ type: "person", person: 1 })).toBe("people");
    expect(tabOf({ type: "broker", town: 0 })).toBe("people");
    expect(tabOf({ type: "rival_plans", town: 0 })).toBe("people");
    expect(tabOf({ type: "secret", hidden: 2 })).toBe("people");
    expect(tabOf({ type: "town_concerns", town: 0 })).toBe("issues");
    expect(tabOf({ type: "hood_concerns", hood: 0 })).toBe("issues");
    expect(tabOf({ type: "place", place: 0 })).toBe("places");
    expect(tabOf({ type: "policy_support", town: 0, policy: { type: "template", template: "x" } })).toBe("policies");
    expect(tabOf({ type: "situational", town: 0, template: "x" })).toBe("policies");
  });

  it("puts placeholders beside what is known: worries by neighbourhood, who runs things first among people", () => {
    const g = groupNotebook(notebookFixture());
    // People: the broker (unknown), the rival's plans (unknown), then people by name.
    expect(g.people.map((i) => [i.subject.type, i.entry ? "known" : "gap", i.args.name ?? ""])).toEqual([
      ["broker", "gap", ""],
      ["rival_plans", "gap", ""],
      ["person", "known", "Albert Crane"],
      ["person", "known", "Vera Holt"],
    ]);
    // Worries: what is new first (Westfield, just canvassed), then the whole
    // town, then neighbourhoods by name, known or not.
    expect(g.issues.map((i) => [i.subject.type, i.args.hood ?? "", i.level, i.isNew])).toEqual([
      ["hood_concerns", "Westfield", "known", true],
      ["town_concerns", "", "known", false],
      ["hood_concerns", "Eastgate", "unknown", false],
    ]);
    const seen = notebookFixture();
    for (const x of seen.sections) for (const e of x.entries) e.new_this_turn = false;
    expect(groupNotebook(seen).issues.map((i) => i.args.hood ?? "")).toEqual(["", "Eastgate", "Westfield"]);
    expect(g.policies).toHaveLength(1);
    expect(g.places).toHaveLength(0);
  });

  it("lists leads picked up first, then each way to learn more once", () => {
    const g = groupNotebook(notebookFixture());
    expect(g.leads.open).toEqual([talkLead]);
    // The broker's placeholder points at the open lead: not repeated.
    expect(g.leads.standing).toEqual([canvassLead]);
  });

  it("marks what the engine says is new, and keeps the marks it is given while the player looks", () => {
    const v = notebookFixture();
    const westfield = subjectKey({ type: "hood_concerns", hood: 2 });
    expect(freshKeys(v)).toEqual([westfield]);
    let g = groupNotebook(v);
    expect(newCounts(g)).toEqual({ people: 0, places: 0, issues: 1, policies: 0 });
    expect(firstNewTab(g)).toBe("issues");
    // After `notebook_seen` the engine no longer says so; the seen marks keep it for this look.
    for (const s of v.sections) for (const e of s.entries) e.new_this_turn = false;
    expect(firstNewTab(groupNotebook(v))).toBeNull();
    g = groupNotebook(v, new Set([westfield]));
    expect(g.issues.find((i) => i.key === westfield)?.isNew).toBe(true);
    // Placeholders are never new.
    expect(g.issues.filter((i) => i.entry === null).every((i) => !i.isNew)).toBe(true);
  });

  it("words entries as far as they are known, and placeholders with nothing they do not know", () => {
    const g = groupNotebook(notebookFixture());
    const [westfield, town, eastgate] = [at(g.issues, 0), at(g.issues, 1), at(g.issues, 2)];
    expect(itemTitle(tr, town)).toBe("What Ashford worries about");
    expect(itemFacts(tr, town)).toEqual([`Worries: ${tr.t("concern-jobs")}, ${tr.t("concern-housing")}, ${tr.t("concern-services")}`]);
    expect(itemFacts(tr, westfield)[0]).toMatch(/^Top worry: /);
    expect(itemTitle(tr, eastgate)).toBe("What Eastgate worries about");
    expect(itemFacts(tr, eastgate)).toEqual([]);
    const [broker, rival, vera] = [at(g.people, 0), at(g.people, 1), at(g.people, 3)];
    expect(itemTitle(tr, broker)).toBe("Who really runs Ashford");
    expect(itemFacts(tr, broker)).toEqual([]);
    // The rival's identity is public, their plans are not: a placeholder says nothing of them.
    expect(itemFacts(tr, rival)).toEqual(["The rival: Mara Quill"]);
    expect(itemFacts(tr, vera)).toEqual(["Councillor", expect.stringMatching(/^Cares about /)]);
    expect(itemFacts(tr, at(g.policies, 0))).toEqual(["Popular: net support +20 points"]);
    expect(itemText(tr, eastgate)).toContain("canvass eastgate");
  });

  it("shows the level without relying on colour", () => {
    expect(["unknown", "heard", "known", "detailed"].map((l) => levelDots(l as "heard"))).toEqual(["○○○", "●○○", "●●○", "●●●"]);
  });

  it("searches by every word", () => {
    expect(matches("what eastgate worries about canvass eastgate", "east canvass")).toBe(true);
    expect(matches("what eastgate worries about", "westfield")).toBe(false);
    expect(matches("anything", "  ")).toBe(true);
  });
});

describe("the key figures (pinned at the top of People)", () => {
  it("pins the broker, the rival, the executive and the body's members, in that order, once each and without the player", () => {
    const f = keyFigures(officeFixture(), townCharacterFixture());
    expect(f.map((x) => x.id)).toEqual([30, 31, 21, 12, 22]);
    expect(f.map((x) => x.pin)).toEqual(["broker", "rival", null, null, null]);
    // The rival sits in the body: pinned once, as the rival, with the office view's line on them.
    expect(at(f, 1).member?.id).toBe(4);
    expect(at(f, 1).ward).toBe(1);
    // The words are the posts' role keys the views give (from the constitution), never named here.
    expect(f.map((x) => x.roleKey)).toEqual(["role-private", "role-councillor", "role-mayor", "role-councillor", "role-councillor"]);
    expect(at(f, 0).person?.rumoured).toBe(true);
  });

  it("is empty without the views, and leaves out a broker not heard of", () => {
    expect(keyFigures(null, null)).toEqual([]);
    expect(keyFigures(null, { ...townCharacterFixture(), broker: null }).map((x) => x.pin)).toEqual(["rival"]);
  });

  it("finds the notebook's people by id", () => {
    const g = groupNotebook(notebookFixture());
    expect([...personItems(g.people).keys()].sort((a, b) => a - b)).toEqual([4, 12]);
  });
});
