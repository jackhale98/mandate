import type { PersonRefView } from "../../engine/protocol/PersonRefView";
import { englishTranslator } from "../../i18n/englishLocale";
import { bobTags, fillPersonTag } from "../../hub/hubView";
import { streetPeople } from "./streetPeople";

const tr = englishTranslator();
const person = (over: Partial<PersonRefView> = {}): PersonRefView => ({ id: 7, ambient_slot: null, name: "Dana Price", role_key: "role-councillor", tier: "principal", level: "known", rumoured: false, can_talk: true, has_news: false, ...over });

describe("name tags' markers", () => {
  it("marks everyone to talk to, and those the engine says have something new, in words", () => {
    const list = streetPeople(tr, [person(), person({ id: 8, name: "Ola Berg", has_news: true }), person({ id: 9, can_talk: false, has_news: true }), person({ id: null, ambient_slot: 2, name: "", role_key: "" })]);
    expect(list.map((p) => [p.key, p.marker, p.tagLabel])).toEqual([
      ["c7", "talk", "Dana Price, can talk"],
      ["c8", "news", "Ola Berg, has something new"],
      ["a2", "talk", `${list[2]?.label ?? ""}, can talk`],
    ]);
  });

  it("fills a tag with a marker that is not colour alone, named for screen readers; only the '!' bobs", () => {
    const el = document.createElement("button");
    expect(fillPersonTag(el, "Ola Berg", "news", "Ola Berg, has something new")).toBe(true);
    expect(el.getAttribute("aria-label")).toBe("Ola Berg, has something new");
    expect(el.getAttribute("aria-hidden")).toBeNull();
    expect(el.querySelector(".hub-tag-mark")?.getAttribute("aria-hidden")).toBe("true");
    expect(el.querySelector(".hub-tag-name")?.textContent).toBe("Ola Berg");
    expect(el.dataset.marker).toBe("news");
    // The same again changes nothing (a room measures a tag once).
    expect(fillPersonTag(el, "Ola Berg", "news", "Ola Berg, has something new")).toBe(false);
    const talk = document.createElement("button");
    fillPersonTag(talk, "Dana Price", "talk", "Dana Price, can talk");
    // The two markers differ in shape (the glyph), not only in colour.
    expect(talk.querySelector(".hub-mark-glyph")?.getAttribute("d")).not.toBe(el.querySelector(".hub-mark-glyph")?.getAttribute("d"));
    bobTags([el, talk], 0.75, false);
    expect(el.classList.contains("hub-tag-bob")).toBe(true);
    expect(talk.classList.contains("hub-tag-bob")).toBe(false);
    bobTags([el, talk], 1.25, false);
    expect(el.classList.contains("hub-tag-bob")).toBe(false);
    // Reduced motion: still.
    bobTags([el], 0.75, true);
    expect(el.classList.contains("hub-tag-bob")).toBe(false);
  });
});
