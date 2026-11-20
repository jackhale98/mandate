import { Translator } from "../i18n/fluent";
import { memoryText, quoteKey, subjectKind } from "./CharacterSheet";

// A memory about nothing in particular must not leave a hole in the line
// ("You pressed them on  and got nowhere"): the line selects on $subject_kind.
const tr = new Translator(
  "en",
  [
    "policy-x = Local wage floor",
    "dilemma-x = the layoffs at the plant",
    "scene-quote-pressed-me = { $subject_kind ->",
    "    [bill] You leaned on me over the { $subject } bill.",
    "    [decision] You leaned on me over { $subject }.",
    "   *[other] You leaned on me once.",
    "}",
  ].join("\n"),
);

describe("memories in words", () => {
  it("says what kind of thing a memory is about", () => {
    expect(subjectKind(null)).toBe("none");
    expect(subjectKind({ key: "policy-x", n: null, post: null })).toBe("bill");
    expect(subjectKind({ key: "dilemma-x", n: null, post: null })).toBe("decision");
    expect(subjectKind({ key: "subject-ward", n: 3, post: null })).toBe("seat");
  });

  it("quotes a memory to the player's face, with or without a subject", () => {
    const key = quoteKey("memory-pressed-me");
    expect(key).toBe("scene-quote-pressed-me");
    expect(memoryText(tr, key, { subject: { key: "policy-x", n: null, post: null }, year: 2030 })).toBe("You leaned on me over the Local wage floor bill.");
    expect(memoryText(tr, key, { subject: { key: "dilemma-x", n: null, post: null }, year: 2030 })).toBe("You leaned on me over the layoffs at the plant.");
    expect(memoryText(tr, key, { subject: null, year: 2030 })).toBe("You leaned on me once.");
  });
});
