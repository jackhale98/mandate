import type { CauseView } from "../../engine/protocol/CauseView";
import { englishTranslator } from "../../i18n/englishLocale";
import { LINKS, PLAYER_ACTS, PRESSURES, PROCESSES, SHOCKS, closureFixture, everythingFixture, lawFixture, person } from "./fixture";
import { layoutWhy, readingOrder, rootOf, stepText, type RootKind } from "./whyModel";

const tr = englishTranslator();

/** Text a player must never see: a missing key, a raw key or id, an unfilled argument. */
const RAW = /⟦|why-|ui-|event-|concern\.|\{\s*\$|[a-z]+\.[a-z_]+|_/;

function texts(l: ReturnType<typeof layoutWhy>): string[] {
  return [...l.rows.flatMap((r) => [r.text, r.date, r.join ?? "", r.from ?? "", ...r.roots.map((x) => x.text)]), l.player?.text ?? "", l.player?.date ?? ""].filter((s) => s !== "");
}

describe("the Why panel's layout", () => {
  it("reads a law newest first, each step joined to the one above by its link", () => {
    const l = layoutWhy(tr, lawFixture());
    expect(l.rows.map((r) => [r.date, r.text, r.join])).toEqual([
      ["12 March 2031", "Iverland passed the Transit frequency bill.", null],
      ["5 March 2031", "You got something done.", "after"],
      ["20 January 2031", "A new bill in Iverland: Transit frequency.", "after"],
      ["2 November 2030", "A new proposal came onto the table.", "prompted by"],
    ]);
    // One line of causes: every step joins the row above it.
    expect(l.rows.every((r) => r.from === null)).toBe(true);
    expect(l.rows.map((r) => r.roots.map((x) => [x.kind, x.text]))).toEqual([
      [["process", "The rules for making law: debate, votes, signing and expiry."]],
      [["player", "You declared your vote on a bill."]],
      [["player", "You introduced a bill."]],
      [["process", "The agenda: proposals come up, and drop off, as conditions call for them."]],
    ]);
    expect(l.more).toBe(false);
    expect(l.further).toBe(false);
  });

  it("says the player's act is behind it, the nearest one, with the day of the step that names it", () => {
    expect(layoutWhy(tr, lawFixture()).player).toEqual({ text: "You declared your vote on a bill.", date: "5 March 2031" });
    expect(layoutWhy(tr, closureFixture()).player).toBeNull();
  });

  it("names a pressure and an outside shock under their steps", () => {
    const l = layoutWhy(tr, closureFixture());
    expect(l.rows.map((r) => [r.text, r.join, r.roots.map((x) => x.text)])).toEqual([
      ["Manufacturing layoffs in Iverland: 120 jobs lost.", null, ["What weighed most on the sector: weak world demand."]],
      ["A global recession begins (demand down 4%).", "because of", ["The world economy: nothing here caused it."]],
    ]);
  });

  it("follows one line of causes back before the next, and says which step a second line joins", () => {
    const v = everythingFixture();
    // The engine gives the steps level by level.
    expect(v.chain.map((s) => s.id)).toEqual([90, 88, 60, 50, 45, 41]);
    expect(readingOrder(v.chain).map((s) => s.id)).toEqual([90, 88, 60, 50, 45, 41]);
    const l = layoutWhy(tr, v);
    const kept = "You kept your word: Jobs first.";
    expect(l.rows[0]?.text).toBe(kept);
    // The deed joins the row above it; the promise joins the first row, two rows up, so it says so.
    expect(l.rows.map((r) => [r.id, r.join, r.from])).toEqual([
      [90, null, null],
      [88, "because of", null],
      [60, "which ended", kept],
      [50, "after", null],
      [45, "prompted by", null],
      [41, "which brought to light", null],
    ]);
    // A deeper branch is read to its end before the next one starts.
    const base = closureFixture().chain;
    const top = base.find((s) => s.depth === 0);
    const behind = base.find((s) => s.depth === 1);
    if (!top || !behind) throw new Error("the fixture has two steps");
    const order = readingOrder([
      { ...top, id: 1, led_to: null },
      { ...behind, id: 2, led_to: { event: 1, link: "caused" } },
      { ...behind, id: 3, led_to: { event: 1, link: "ended" } },
      { ...behind, id: 4, led_to: { event: 2, link: "continues" } },
      { ...behind, id: 5, led_to: { event: 99, link: "continues" } },
    ]);
    expect(order.map((s) => s.id)).toEqual([1, 2, 4, 3, 5]);
  });

  it("covers every link type and every kind of start across the fixtures, in plain words", () => {
    const views = [lawFixture(), closureFixture(), everythingFixture()];
    const links = new Set(views.flatMap((v) => v.chain.flatMap((s) => (s.led_to ? [s.led_to.link] : []))));
    expect([...links].sort()).toEqual([...LINKS].sort());
    const kinds = new Set(views.flatMap((v) => layoutWhy(tr, v).rows.flatMap((r) => r.roots.map((x) => x.kind))));
    const every: RootKind[] = ["player", "actor", "process", "pressure", "exogenous", "history", "forgotten", "unknown"];
    expect([...kinds].sort()).toEqual([...every].sort());
    for (const v of views) for (const s of texts(layoutWhy(tr, v))) expect(s, s).not.toMatch(RAW);
  });

  it("says more lies further back, and offers a longer walk only while the engine has one", () => {
    const v = everythingFixture();
    expect(layoutWhy(tr, v)).toMatchObject({ more: true, further: true });
    expect(layoutWhy(tr, { ...v, depth: 12 })).toMatchObject({ more: true, further: false });
    expect(layoutWhy(tr, { ...v, truncated: false })).toMatchObject({ more: false, further: false });
  });

  it("names someone as the player knows them: by name, or as someone they have not heard of", () => {
    const l = layoutWhy(tr, everythingFixture());
    const offered = l.rows.find((r) => r.id === 50);
    expect(offered?.roots.map((x) => x.text)).toEqual(["Vera Holt's own choice.", "Someone's own choice. You have not heard who."]);
    // A name the engine left empty in a step's sentence.
    expect(l.rows.find((r) => r.id === 45)?.text).toBe("Scandal: someone you have not heard of and a cash bribe (Iverland).");
    expect(stepText(tr, { text_key: "event-development-rival", args: { name: "" } })).toBe("Someone you have not heard of is your rival now.");
    expect(rootOf(tr, { type: "actor", person: person("") })?.text).toBe("Someone's own choice. You have not heard who.");
  });

  it("has a line for every value the engine can send", () => {
    const causes: CauseView[] = [
      ...PLAYER_ACTS.map((act): CauseView => ({ type: "player", act })),
      ...PROCESSES.map((process): CauseView => ({ type: "process", process })),
      ...PRESSURES.map((pressure): CauseView => ({ type: "pressure", pressure })),
      ...SHOCKS.map((shock): CauseView => ({ type: "exogenous", shock })),
      { type: "actor", person: person("Vera Holt") },
      { type: "actor", person: null },
      { type: "history" },
      { type: "forgotten" },
      { type: "unknown" },
    ];
    const lines = causes.map((c) => rootOf(tr, c)?.text ?? "");
    for (const s of lines) {
      expect(s, s).not.toMatch(RAW);
      expect(s.length, s).toBeGreaterThan(8);
      // A line is a short sentence: it fits a phone's width in two or three lines.
      expect(s.length, s).toBeLessThanOrEqual(96);
      expect(s, s).toMatch(/[.]$/);
    }
    expect(new Set(lines).size).toBe(lines.length);
    for (const link of LINKS) expect(tr.t(`why-link-${link}`), link).not.toMatch(RAW);
    expect(rootOf(tr, { type: "event", event: 1, link: "caused", in_chain: true })).toBeNull();
  });
});
