import { act, fireEvent, render, screen, within } from "@testing-library/react";
import type { TalkView } from "../../engine/protocol/TalkView";
import type { ExchangeView } from "../../engine/protocol/ExchangeView";
import type { CanvassView } from "../../engine/protocol/CanvassView";
import type { PersonRefView } from "../../engine/protocol/PersonRefView";
import { englishTranslator } from "../../i18n/englishLocale";
import { useGame } from "../../state/gameStore";
import { TalkSheet } from "./TalkSheet";
import { WorryPicker } from "./WorryPicker";
import { streetPeople } from "./streetPeople";
import { costText, PAGE_CHARS, replyPages, splitLong, topicRow } from "./dialogueModel";
import { setTalkStage, useTalkStage, type TalkStage } from "./stage";
import { keepLook, lookVariant, resetLooks } from "../portrait/look";

const person = (over: Partial<PersonRefView> = {}): PersonRefView => ({
  id: 7,
  ambient_slot: null,
  name: "Dana Price",
  role_key: "role-councillor",
  tier: "principal",
  level: "known",
  rumoured: false,
  can_talk: true,
  has_news: false,
  ...over,
});

const date = { day_number: 737314, year: 2020, month: 1, day: 15 };

function view(over: Partial<TalkView> = {}): TalkView {
  return {
    person: person(),
    here: true,
    route: null,
    opinion_band: "warm",
    trust_band: "some",
    cost_halves: 1,
    half_slot_left: false,
    slots_left: 2,
    late_night: false,
    late_night_possible: false,
    out_of_time: false,
    topics: [
      { id: "topic.who_runs", name_key: "talk-topic-who-runs", mechanic: "inform", about_required: false, about: [], odds: null, available: true, unavailable_key: null, asked_this_week: false },
      {
        id: "topic.pitch",
        name_key: "talk-topic-pitch",
        mechanic: "pitch",
        about_required: true,
        about: [
          { about: { type: "bill", policy: 3 }, kind: "bill", label: "", label_key: "talk-topic-pitch", odds: 0.62, available: true, unavailable_key: null },
          { about: { type: "template", template: "policy.x" }, kind: "template", label: "Something else", label_key: null, odds: null, available: false, unavailable_key: "talk-why-role" },
        ],
        odds: null,
        available: false,
        unavailable_key: "talk-why-needs-subject",
        asked_this_week: false,
      },
      { id: "topic.small_talk", name_key: "talk-topic-small-talk", mechanic: "small_talk", about: [], about_required: false, odds: 0.4, available: false, unavailable_key: "talk-why-owe", asked_this_week: true },
    ],
    ...over,
  };
}

const to = { location: { type: "place" as const, place: 2 }, kind_key: "diner", name: "", neighborhood: 1, neighborhood_name: "Old Town" };

const exchange: ExchangeView = {
  id: 4,
  date,
  speaker: person(),
  place: null,
  topic: "topic.who_runs",
  topic_key: "talk-topic-who-runs",
  about_label: null,
  about_label_key: null,
  parts: [
    { part: "opener", key: "talk-open-neutral-3", args: { speaker: "Dana Price", player: "You", town: "Ashford", here_key: "talk-place-diner" } },
    { part: "substance", key: "talk-runs-fb-politician", args: { speaker: "Dana Price", player: "You", town: "Ashford", here_key: "talk-place-diner" } },
  ],
  learned: [
    { subject: { type: "town_concerns", town: 1 }, title_key: "notebook-town-concerns", args: { town: "Ashford", concern_key: "concern-housing" }, level: "heard" },
    { subject: { type: "person", person: 7 }, title_key: "notebook-person", args: { name: "Dana Price", role_key: "role-councillor" }, level: "known" },
  ],
  leads: [
    { key: "lead-ask-diner", args: {}, subject: { type: "broker", town: 0 }, level: "heard", to: null, route: null, since: date },
    { key: "lead-ask-market", args: {}, subject: { type: "broker", town: 0 }, level: "heard", to: null, route: { action: "go", late_night: false, neighborhood: 1, to, here: false }, since: date },
  ],
  relationship: "warmer",
  outcome_key: "outcome-talk",
  outcome_args: { name: "Dana Price" },
  repeat: false,
};

/** Reduced motion: no typewriter, so a reply is all there at once. */
function reduceMotion(on: boolean) {
  window.matchMedia = ((q: string) => ({ matches: on && q.includes("reduce"), media: q, addEventListener() {}, removeEventListener() {} })) as unknown as typeof window.matchMedia;
}

const tr = englishTranslator();
const choice = (testId: string) => within(screen.getByTestId(testId)).getByRole("button");

describe("dialogue mode", () => {
  beforeEach(() => {
    reduceMotion(true);
    resetLooks();
    setTalkStage(null);
    useGame.setState({ tr, talk: view(), talkTo: { type: "person", person: 7 }, exchange: null, talkLate: false, busy: false });
  });

  it("is a box of choices: the topics with the engine's odds, the cost, and why one can't be raised", () => {
    render(<TalkSheet />);
    expect(screen.getByRole("dialog", { name: "Talking with Dana Price" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Dana Price" })).toBeTruthy();
    expect(screen.getByTestId("talk-opinion").textContent).toBe("Warm towards you");
    expect(screen.getByTestId("talk-cost").textContent).toMatch(/^½ slot · 2 left.*half a slot.*2 slots left/);
    // Away from the street (no stage): over a portrait.
    expect(screen.getByTestId("talk-layer").dataset.mode).toBe("portrait");
    expect(screen.getByTestId("talk-portrait")).toBeTruthy();
    expect(choice("talk-topic-topic.who_runs").textContent).toBe("Who really runs things here?");
    // A topic about something opens a subject step (›) and shows no odds of its own.
    expect(within(screen.getByTestId("talk-topic-topic.pitch")).getByText("›")).toBeTruthy();
    expect(choice("talk-topic-topic.pitch").getAttribute("aria-disabled")).toBeNull();
    // Small talk: unavailable, with the engine's reason, its odds, and asked this week.
    const small = screen.getByTestId("talk-topic-topic.small_talk");
    expect(choice("talk-topic-topic.small_talk").getAttribute("aria-disabled")).toBe("true");
    expect(within(small).getByTestId("talk-why").textContent).toBe("You already owe them too much.");
    expect(within(small).getByTestId("talk-odds").textContent).toBe("40%40% chance");
    expect(small.textContent).toContain("asked this week");
    expect(choice("talk-goodbye").textContent).toBe("Goodbye");
    // Nothing on screen is an untranslated key.
    expect(screen.getByTestId("talk-layer").textContent).not.toMatch(/\b(talk|ui|notebook|lead)-[a-z-]+\b/);
  });

  it("raises a topic in one tap; a topic about something asks for the subject in the same box", () => {
    const talkAbout = vi.fn(async () => {});
    useGame.setState({ talkAbout });
    render(<TalkSheet />);
    fireEvent.click(choice("talk-topic-topic.who_runs"));
    expect(talkAbout).toHaveBeenCalledWith("topic.who_runs", null);
    // While the engine answers, nobody has spoken yet: the person's opener comes first, then the question.
    expect(screen.queryByTestId("talk-you")).toBeNull();
    act(() => useGame.setState({ exchange: { ...exchange, id: 9 } }));
    expect(screen.getByTestId("talk-bubble").querySelector("[data-stage]")?.textContent).toContain("Dana Price checks the time");
    expect(screen.queryByTestId("talk-you")).toBeNull();
    fireEvent.click(choice("talk-continue"));
    expect(screen.getByTestId("talk-you").textContent).toBe("Who really runs things here?");
    expect(screen.queryByTestId("talk-bubble")).toBeNull();
    fireEvent.click(choice("talk-topic-topic.pitch"));
    const subjects = screen.getByTestId("talk-subjects");
    // Grouped by kind, each with its own odds and availability.
    expect(subjects.textContent).toContain("Bills");
    expect(subjects.textContent).toContain("Policies");
    const rows = within(subjects).getAllByTestId("talk-subject");
    expect(within(rows[0] as HTMLElement).getByTestId("talk-odds").textContent).toMatch(/^62%/);
    expect(within(rows[1] as HTMLElement).getByTestId("talk-why").textContent).toBe("They're not the person to ask about that.");
    fireEvent.click(within(rows[1] as HTMLElement).getByRole("button"));
    expect(talkAbout).toHaveBeenCalledTimes(1);
    fireEvent.click(within(rows[0] as HTMLElement).getByRole("button"));
    expect(talkAbout).toHaveBeenLastCalledWith("topic.pitch", { type: "bill", policy: 3 });
    // Back at the topics; the player's line names the subject.
    expect(screen.getByTestId("talk-topics")).toBeTruthy();
    act(() => useGame.setState({ exchange: { ...exchange, id: 10, parts: exchange.parts.filter((p) => p.part !== "opener") } }));
    // No opener: the question goes up as the reply arrives.
    expect(screen.getByTestId("talk-you").textContent).toBe("Pitch a policy: Pitch a policy");
  });

  it("the keyboard: arrows move, Escape steps back from the subjects, then leaves", () => {
    const openTalk = vi.fn(async () => {});
    useGame.setState({ openTalk });
    render(<TalkSheet />);
    const list = screen.getByTestId("talk-topics");
    const buttons = within(list).getAllByRole("button");
    // The first choice has the keyboard; one Tab stop for the list.
    expect(document.activeElement).toBe(buttons[0]);
    expect(list.querySelectorAll("button[tabindex='0']").length).toBe(1);
    fireEvent.keyDown(document.activeElement as Element, { key: "ArrowDown" });
    expect(document.activeElement).toBe(buttons[1]);
    fireEvent.keyDown(document.activeElement as Element, { key: "End" });
    expect(document.activeElement).toBe(choice("talk-goodbye"));
    fireEvent.keyDown(document.activeElement as Element, { key: "ArrowDown" });
    expect(document.activeElement).toBe(buttons[0]);
    fireEvent.keyDown(document.activeElement as Element, { key: "ArrowUp" });
    expect(document.activeElement).toBe(choice("talk-goodbye"));
    fireEvent.click(choice("talk-topic-topic.pitch"));
    expect(screen.getByTestId("talk-subjects")).toBeTruthy();
    fireEvent.keyDown(document.activeElement as Element, { key: "Escape" });
    expect(screen.queryByTestId("talk-subjects")).toBeNull();
    expect(document.activeElement).toBe(choice("talk-topic-topic.pitch"));
    expect(openTalk).not.toHaveBeenCalled();
    fireEvent.keyDown(document.activeElement as Element, { key: "Escape" });
    expect(openTalk).toHaveBeenCalledWith(null);
  });

  it("says plainly when the player is out of time", () => {
    useGame.setState({ talk: view({ out_of_time: true, slots_left: 0, topics: view().topics.map((t) => ({ ...t, available: false, unavailable_key: "talk-why-time", about: t.about.map((c) => ({ ...c, odds: null, available: false, unavailable_key: "talk-why-time" })) })) }) });
    render(<TalkSheet />);
    expect(screen.getByTestId("talk-out-of-time").textContent).toContain("out of time this turn");
    expect(screen.queryByTestId("talk-cost")).toBeNull();
    expect(screen.getByText("Out of time")).toBeTruthy();
    for (const id of ["topic.who_runs", "topic.pitch", "topic.small_talk"]) expect(choice(`talk-topic-${id}`).getAttribute("aria-disabled")).toBe("true");
    // Said once, not on every line.
    expect(screen.queryAllByTestId("talk-why")).toHaveLength(0);
    expect(choice("talk-goodbye").getAttribute("aria-disabled")).toBeNull();
  });

  it("someone elsewhere: where they are, and going there first", () => {
    const goToTalk = vi.fn(async () => {});
    useGame.setState({ goToTalk, talk: view({ here: false, route: { action: "go", late_night: false, neighborhood: 1, to, here: false } }) });
    render(<TalkSheet />);
    expect(screen.getByTestId("talk-away").textContent).toMatch(/They're at/);
    const go = choice("talk-go");
    expect(go.textContent).toMatch(/Go to/);
    expect(document.activeElement).toBe(go);
    fireEvent.click(go);
    expect(goToTalk).toHaveBeenCalled();
  });

  it("a reply: page by page in their voice (stage directions in italics), read out, then the outcome, a notebook toast and a lead to follow", () => {
    const openTalk = vi.fn(async () => {});
    const openPocket = vi.fn();
    const visit = vi.fn(async () => {});
    useGame.setState({ exchange, openTalk, openPocket, visit });
    render(<TalkSheet />);
    const pages = replyPages(tr, exchange);
    expect(pages.length).toBeGreaterThan(1);
    const bubble = screen.getByTestId("talk-bubble");
    const live = screen.getByTestId("talk-reply-live");
    // The first page: the opener as a stage direction; the live region reads the same words.
    expect(bubble.querySelector("[data-stage]")?.textContent).toContain("Dana Price checks the time");
    expect(live.textContent).toBe(`Dana Price: ${pages[0]?.map((s) => s.text).join(" ")}`);
    expect(bubble.textContent).toContain(pages[0]?.[0]?.text ?? "?");
    // Not at the end: no outcome, no toast yet.
    expect(screen.queryByTestId("talk-outcome")).toBeNull();
    expect(screen.queryByTestId("talk-noted")).toBeNull();
    for (let i = 1; i < pages.length; i++) fireEvent.click(choice("talk-continue"));
    expect(screen.queryByTestId("talk-continue")).toBeNull();
    // The last page: the mood's change, as a stage direction.
    expect(screen.getByTestId("talk-bubble").textContent).toContain("Dana Price seems warmer towards you.");
    expect(screen.getByTestId("talk-outcome").textContent).toBe("You talked with Dana Price.");
    const noted = screen.getByTestId("talk-noted");
    expect(noted.textContent).toContain("Noted: What Ashford worries about: Housing");
    expect(noted.textContent).toContain("and 1 more");
    // A lead with somewhere to go is a choice.
    const lead = choice("talk-lead-go");
    expect(lead.textContent).toMatch(/^Go to /);
    fireEvent.click(lead);
    expect(openTalk).toHaveBeenCalledWith(null);
    expect(visit).toHaveBeenCalledWith(to.location);
    fireEvent.click(within(noted).getByTestId("talk-noted-open"));
    expect(openPocket).toHaveBeenCalledWith("notebook");
    expect(screen.getByTestId("talk-layer").textContent).not.toMatch(/\b(talk|ui|notebook|lead)-[a-z-]+\b/);
  });

  it("the typewriter reveals a page; a tap shows it all, the next tap turns it", () => {
    reduceMotion(false);
    vi.useFakeTimers();
    try {
      render(<TalkSheet />);
      // The reply arrives.
      act(() => useGame.setState({ exchange }));
      const bubble = () => screen.getByTestId("talk-bubble");
      // All the words are in the DOM from the start; what is not typed yet is hidden.
      expect(bubble().querySelector(".dlg-untyped")).toBeTruthy();
      const first = bubble().textContent;
      act(() => {
        vi.advanceTimersByTime(200);
      });
      expect(bubble().textContent).toBe(first);
      fireEvent.click(bubble());
      expect(bubble().querySelector(".dlg-untyped")).toBeNull();
      fireEvent.click(bubble());
      expect(bubble().textContent).not.toBe(first);
    } finally {
      vi.useRealTimers();
    }
  });

  it("in the street: the stage frames the two speakers while the conversation lasts", () => {
    const calls: string[] = [];
    const stage: TalkStage = {
      canFrame: (k) => k === "c7",
      begin: (k) => void calls.push(`begin ${k}`),
      cover: () => {},
      speaking: (w) => void calls.push(`speaking ${w}`),
      end: () => void calls.push("end"),
      anchors: () => ({ player: { x: 100, y: 300 }, person: { x: 160, y: 300 }, scale: 48, bounds: { left: 0, top: 0, right: 400, bottom: 800 } }),
      onFrame: () => () => {},
      lookOf: () => 321,
      focus: () => void calls.push("focus"),
    };
    setTalkStage(stage);
    const { unmount } = render(<TalkSheet />);
    expect(screen.getByTestId("talk-layer").dataset.mode).toBe("street");
    expect(screen.getByTestId("talk-veil")).toBeTruthy();
    expect(useTalkStage.getState().mode).toBe("street");
    expect(calls).toContain("begin c7");
    unmount();
    expect(calls).toContain("end");
    // The keyboard goes back to the street.
    expect(calls).toContain("focus");
  });
});

describe("the dialogue's words", () => {
  it("pages a long reply for a phone's bubble, between sentences", () => {
    const long = Array.from({ length: 12 }, (_, i) => `Sentence number ${i} is here.`).join(" ");
    const parts = splitLong(long);
    expect(parts.length).toBeGreaterThan(1);
    for (const p of parts) expect(p.length).toBeLessThanOrEqual(PAGE_CHARS);
    expect(parts.join(" ")).toBe(long);
    expect(parts.every((p) => p.endsWith("."))).toBe(true);
  });

  it("states the cost in a few words", () => {
    expect(costText(tr, view())?.short).toBe("½ slot · 2 left");
    expect(costText(tr, view({ half_slot_left: true, slots_left: 2 }))?.short).toBe("½ slot · 2½ left");
    expect(costText(tr, view({ half_slot_left: true, slots_left: 0 }))?.short).toBe("½ slot · ½ left");
    expect(costText(tr, view({ late_night: true }))?.short).toBe("The late slot");
    expect(costText(tr, view({ out_of_time: true }))).toBeNull();
  });

  it("a topic about something is open while one of its subjects is", () => {
    const pitch = view().topics[1];
    if (!pitch) throw new Error("fixture");
    expect(topicRow(pitch)).toMatchObject({ opens: true, available: true, why: null });
    const none = { ...pitch, about: pitch.about.map((c) => ({ ...c, available: false, unavailable_key: "talk-why-away" })) };
    expect(topicRow(none)).toMatchObject({ available: false, why: "talk-why-away" });
    expect(topicRow({ ...pitch, about: [] })).toMatchObject({ opens: false, available: false, why: "talk-why-needs-subject" });
  });
});

describe("the worry picker", () => {
  beforeEach(() => {
    useGame.setState({ tr: englishTranslator() });
  });
  const canvass = (over: Partial<CanvassView> = {}): CanvassView => ({
    neighborhood: 3,
    name: "Old Town",
    worries: [
      { id: "concern.housing", name_key: "concern-housing", source: "hood", top: true, fit_band: "strong", fit: 1.45 },
      { id: "concern.jobs", name_key: "concern-jobs", source: "town", top: false, fit_band: null, fit: null },
    ],
    knowledge: { level: "detailed", as_of: null, stale: false, lead: null },
    ...over,
  });

  it("defaults to no particular issue, and shows the engine's match for a chosen worry", () => {
    const onChange = vi.fn();
    const { rerender } = render(<WorryPicker view={canvass()} issue={null} onChange={onChange} />);
    const select = screen.getByTestId("canvass-worry") as HTMLSelectElement;
    expect(select.value).toBe("");
    expect([...select.options].map((o) => o.textContent)).toEqual(["No particular issue", "Housing costs · their top worry · a strong fit", "Jobs · a town worry"]);
    expect(screen.getByTestId("canvass-fit").textContent).toBe("A general canvass, on no worry in particular.");
    fireEvent.change(select, { target: { value: "concern.housing" } });
    expect(onChange).toHaveBeenCalledWith("concern.housing");
    rerender(<WorryPicker view={canvass()} issue="concern.housing" onChange={onChange} />);
    expect(screen.getByTestId("canvass-fit").textContent).toBe("Housing costs here: a strong fit, ×1.45 the effect.");
    rerender(<WorryPicker view={canvass()} issue="concern.jobs" onChange={onChange} />);
    expect(screen.getByTestId("canvass-fit").textContent).toContain("isn't known yet");
  });

  it("offers nothing the player doesn't know of", () => {
    render(<WorryPicker view={canvass({ worries: [], knowledge: { level: "unknown", as_of: null, stale: false, lead: null } })} issue={null} onChange={() => {}} />);
    expect((screen.getByTestId("canvass-worry") as HTMLSelectElement).options.length).toBe(1);
    expect(screen.getByTestId("canvass-fit").textContent).toContain("don't know what worries people here");
  });
});

describe("the street's people", () => {
  it("are those the engine lists as here, named as the player knows them, with a stable look", () => {
    const list = streetPeople(tr, [
      person(),
      person({ id: null, ambient_slot: 1, name: "", role_key: "occupation-retiree", tier: "ambient", level: "unknown" }),
      person({ id: 9, can_talk: false }),
    ]);
    expect(list.map((p) => p.key)).toEqual(["c7", "a1"]);
    expect(list[0]?.label).toBe("Dana Price");
    expect(list[1]?.label).toBe("Retiree");
    expect(streetPeople(tr, [person()])[0]?.variant).toBe(list[0]?.variant);
  });

  it("a passer-by met keeps their look under their new key (the street and the portrait agree)", () => {
    resetLooks();
    const was = lookVariant("a1");
    keepLook("c40", was);
    expect(lookVariant("c40")).toBe(was);
    expect(streetPeople(tr, [person({ id: 40 })])[0]?.variant).toBe(was);
  });
});
