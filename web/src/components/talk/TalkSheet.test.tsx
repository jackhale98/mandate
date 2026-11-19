import { fireEvent, render, screen, within } from "@testing-library/react";
import type { TalkView } from "../../engine/protocol/TalkView";
import type { ExchangeView } from "../../engine/protocol/ExchangeView";
import type { CanvassView } from "../../engine/protocol/CanvassView";
import type { PersonRefView } from "../../engine/protocol/PersonRefView";
import { englishTranslator } from "../../i18n/englishLocale";
import { useGame } from "../../state/gameStore";
import { TalkSheet } from "./TalkSheet";
import { WorryPicker } from "./WorryPicker";
import { streetPeople } from "./streetPeople";

const person = (over: Partial<PersonRefView> = {}): PersonRefView => ({
  id: 7,
  ambient_slot: null,
  name: "Dana Price",
  role_key: "role-councillor",
  tier: "principal",
  level: "known",
  rumoured: false,
  can_talk: true,
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
          { about: { type: "template", template: "policy.x" }, kind: "template", label: "Something else", label_key: null, odds: null, available: true, unavailable_key: null },
        ],
        odds: null,
        available: false,
        unavailable_key: "talk-why-needs-subject",
        asked_this_week: false,
      },
      { id: "topic.small_talk", name_key: "talk-topic-small-talk", mechanic: "small_talk", about: [], about_required: false, odds: null, available: false, unavailable_key: "talk-why-owe", asked_this_week: true },
    ],
    ...over,
  };
}

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
  leads: [{ key: "lead-ask-diner", args: {}, subject: { type: "broker", town: 0 }, level: "heard", to: null, route: null, since: date }],
  relationship: "warmer",
  outcome_key: "outcome-talk",
  outcome_args: { name: "Dana Price" },
  repeat: false,
};

describe("the talk panel", () => {
  beforeEach(() => {
    useGame.setState({ tr: englishTranslator(), talk: view(), exchange: null, talkLate: false, busy: false });
  });

  it("lists the topics as buttons with the engine's odds and cost, and says why one can't be asked", () => {
    render(<TalkSheet />);
    expect(screen.getByRole("heading", { name: "Talking with Dana Price" })).toBeTruthy();
    expect(screen.getByTestId("talk-opinion").textContent).toBe("Warm towards you");
    expect(screen.getByTestId("talk-cost").textContent).toMatch(/half a slot.*2 slots left/);
    expect(screen.getByRole("button", { name: /Who really runs things here\?/ })).toBeTruthy();
    // The pitch shows the chosen bill's odds, as the engine gave them.
    const pitch = screen.getByTestId("talk-topic-topic.pitch");
    expect(within(pitch).getByTestId("talk-odds").textContent).toBe("62% chance");
    fireEvent.change(within(pitch).getByTestId("talk-about"), { target: { value: JSON.stringify({ type: "template", template: "policy.x" }) } });
    expect(within(pitch).queryByTestId("talk-odds")).toBeNull();
    // Small talk: unavailable, with the engine's reason, and asked this week.
    const small = screen.getByTestId("talk-topic-topic.small_talk");
    expect((within(small).getByRole("button") as HTMLButtonElement).disabled).toBe(true);
    expect(within(small).getByTestId("talk-why").textContent).toBe("You already owe them too much.");
    expect(small.textContent).toContain("asked this week");
  });

  it("asks the engine (the topic, and the subject chosen)", () => {
    const talkAbout = vi.fn(async () => {});
    useGame.setState({ talkAbout });
    render(<TalkSheet />);
    fireEvent.click(screen.getByRole("button", { name: /Who really runs/ }));
    expect(talkAbout).toHaveBeenCalledWith("topic.who_runs", null);
    fireEvent.click(within(screen.getByTestId("talk-topic-topic.pitch")).getByTestId("talk-ask"));
    expect(talkAbout).toHaveBeenLastCalledWith("topic.pitch", { type: "bill", policy: 3 });
  });

  it("says plainly when the player is out of time", () => {
    useGame.setState({ talk: view({ out_of_time: true, slots_left: 0, topics: view().topics.map((t) => ({ ...t, available: false, unavailable_key: "talk-why-time", about: t.about.map((c) => ({ ...c, odds: null, available: false, unavailable_key: "talk-why-time" })) })) }) });
    render(<TalkSheet />);
    expect(screen.getByTestId("talk-out-of-time").textContent).toContain("out of time this turn");
    expect(screen.queryByTestId("talk-cost")).toBeNull();
    for (const b of screen.getByTestId("talk-topics").querySelectorAll("button")) expect((b as HTMLButtonElement).disabled).toBe(true);
  });

  it("someone elsewhere: where they are, and a way there", () => {
    const goToTalk = vi.fn(async () => {});
    const to = { location: { type: "place" as const, place: 2 }, kind_key: "diner", name: "", neighborhood: 1, neighborhood_name: "Old Town" };
    useGame.setState({ goToTalk, talk: view({ here: false, route: { action: "go", late_night: false, neighborhood: 1, to, here: false } }) });
    render(<TalkSheet />);
    expect(screen.getByTestId("talk-away").textContent).toMatch(/They're at/);
    fireEvent.click(screen.getByTestId("talk-go"));
    expect(goToTalk).toHaveBeenCalled();
  });

  it("shows the exchange as a dialogue in their voice, then what was learned and the leads", () => {
    useGame.setState({ exchange });
    render(<TalkSheet />);
    const x = screen.getByTestId("talk-exchange");
    expect(within(x).getByTestId("talk-you").textContent).toBe("You: Who really runs things here?");
    const lines = within(x).getByTestId("talk-dialogue").querySelectorAll("p");
    expect(lines.length).toBe(2);
    expect(lines[0]?.className).toBe("talk-stage");
    expect(lines[0]?.textContent).toContain("Dana Price");
    expect(within(x).getByTestId("talk-outcome").textContent).toContain("Dana Price seems warmer towards you.");
    const learned = within(x).getByTestId("talk-learned").textContent ?? "";
    expect(learned).toContain("What Ashford worries about: Housing");
    expect(learned).toContain("Heard of");
    expect(within(x).getByTestId("talk-leads").textContent).toContain("The diner hears everything");
    // Nothing on screen is an untranslated key.
    expect(x.textContent).not.toMatch(/\b(talk|ui|notebook|lead)-[a-z-]+\b/);
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
    const tr = englishTranslator();
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
});
